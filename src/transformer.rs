//! ELF is edited by producing a new version with edits applied to individual sections.
//!
//! This module describes this transformation process.

use std::{io, mem::size_of_val};

use goblin::{
    container::Ctx,
    elf::{self, Elf, ProgramHeader, SectionHeader},
};
use scroll::{
    ctx::{SizeWith, TryIntoCtx},
    IOwrite,
};

#[allow(unused)]
pub fn transform_elf_sections<Output, SectionTransformer>(
    input_bytes: &[u8],
    elf: &Elf,
    ctx: Ctx,
    mut output: Output,
    transformer: SectionTransformer,
) where
    Output: io::Write,
    SectionTransformer: for<'bytes, 'header, 'output> Fn(
        /* input_bytes: */ &'bytes [u8],
        /* section_header: */ &'header SectionHeader,
        /* ctx: */ Ctx,
        /* output: */ &'output mut dyn io::Write,
    ) -> Option<u64>,
{
    // Serialization buffer.
    let mut buf = [0u8; 256];
    assert!(
        size_of_val(&buf) >= ProgramHeader::size_with(&ctx),
        "Single serialized `ProgramHeader` value should fit into the serialization buffer.\n\
         Current buffer size: {}\n\
         ProgramHeader size: {}",
        size_of_val(&buf),
        ProgramHeader::size_with(&ctx),
    );
    assert!(
        size_of_val(&buf) >= SectionHeader::size_with(&ctx),
        "Single serialized `SectionHeader` value should fit into the serialization buffer.\n\
         Current buffer size: {}\n\
         SectionHeader size: {}",
        size_of_val(&buf),
        SectionHeader::size_with(&ctx),
    );

    let ComputeShiftsResult {
        program_headers: output_program_headers,
        section_headers: output_section_headers,
        section_headers_start,
    } = compute_shifts(
        input_bytes,
        &elf.program_headers,
        &elf.section_headers,
        ctx,
        &transformer,
    );

    let mut written_up_to = 0;

    let new_header = {
        let mut res = elf.header.clone();
        res.e_shoff = section_headers_start;
        res
    };

    output
        .iowrite_with(new_header, ctx)
        .expect("ELF header serializes correctly and fits into the output");
    written_up_to += elf::Header::size_with(&ctx) as u64;

    // We do not allow adding or removing sections for now, so the position or the side of the
    // program headers is not expected to be any different.
    assert_eq!(written_up_to, elf.header.e_phoff);
    for header in output_program_headers {
        iowrite_from_scroll(&mut buf, &mut output, header, ctx)
            .expect("`ProgramHeader` values serialize correctly");
        written_up_to += ProgramHeader::size_with(&ctx) as u64;
    }

    {
        let mut input_section_headers = elf.section_headers.iter();
        let mut output_section_headers = output_section_headers.iter();

        while let (Some(input_section_header), Some(output_section_header)) =
            (input_section_headers.next(), output_section_headers.next())
        {
            add_padding(
                &mut output,
                &mut buf,
                output_section_header.sh_offset,
                &mut written_up_to,
            );

            match transformer(input_bytes, &input_section_header, ctx, &mut output) {
                Some(_) => {
                    // `transformer` is expected to write the updated bytes into `output`.
                }
                None => {
                    let section_start = input_section_header.sh_offset as usize;
                    let section_end = section_start + input_section_header.sh_size as usize;

                    output
                        .write_all(&input_bytes[section_start..section_end])
                        .expect("Output can consume all the section data");
                }
            };
        }

        assert_eq!(input_section_headers.len(), 0);
        assert_eq!(output_section_headers.len(), 0);
    }

    add_padding(
        &mut output,
        &mut buf,
        section_headers_start,
        &mut written_up_to,
    );

    for header in output_section_headers {
        iowrite_from_scroll(&mut buf, &mut output, header, ctx)
            .expect("`SectionHeader` values serialize correctly");
        written_up_to += SectionHeader::size_with(&ctx) as u64;
    }
}

fn iowrite_from_scroll<Output, T, Ctx>(
    buf: &mut [u8],
    output: &mut Output,
    value: T,
    ctx: Ctx,
) -> Result<(), <T as TryIntoCtx<Ctx>>::Error>
where
    Output: io::Write,
    T: SizeWith<Ctx> + TryIntoCtx<Ctx>,
    Ctx: Copy,
{
    let size = T::size_with(&ctx);
    let buf = &mut buf[0..size];
    value.try_into_ctx(buf, ctx)?;
    output
        .write_all(buf)
        .expect("Output can fit all the serialized values");
    Ok(())
}

fn add_padding<Output>(
    output: &mut Output,
    buf: &mut [u8],
    target_offset: u64,
    written_up_to: &mut u64,
) where
    Output: io::Write,
{
    while *written_up_to < target_offset {
        let size = target_offset
            .saturating_sub(*written_up_to)
            .min(size_of_val(&buf) as u64);
        let buf = &mut buf[0..size as usize];
        buf.fill(0);
        output
            .write_all(&buf)
            .expect("Output can fit all the section paddings");

        *written_up_to += size;
    }
}

fn strict_signed_diff(a: u64, b: u64) -> i64 {
    let res = a.wrapping_sub(b) as i64;
    let overflow = (a >= b) == (res < 0);

    assert!(!overflow, "{a}: u64 - {b}: u64 overflows i64");
    res
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputeShiftsResult {
    program_headers: Vec<ProgramHeader>,
    section_headers: Vec<SectionHeader>,
    section_headers_start: u64,
}

/// Records when we update a program header, to make sure we only update each program header once
/// and each program headers is updated.  This help identify bugs, and unexpected input, it does not
/// affect the output produced.
struct ProgramHeaderUpdate {
    /// We have seen a section that starts at this program header start.
    start: bool,
    /// We have seen a section that ends at this program header end.
    end: bool,
}

impl ProgramHeaderUpdate {
    fn no_updates() -> Self {
        Self {
            start: false,
            end: false,
        }
    }
}

struct SectionDimensions {
    offset: u64,
    size: u64,
}

/// Helper used to update program headers.
struct OutputProgramHeadersUpdater {
    /// Holds exiting section offset and size, and flags that indicate if this section was updated or
    /// not.  Same size as `output` and matches based on the index.
    meta: Vec<(SectionDimensions, ProgramHeaderUpdate)>,
    /// Holds a value for the new program header after the edit.  Same size as `meta` and matches
    /// based on the index.
    output: Vec<ProgramHeader>,
}

impl OutputProgramHeadersUpdater {
    /// Initially `OutputProgramHeaders` contains a copy of the `program_headers`, and none are
    /// marked as updated.
    fn new(program_headers: &[ProgramHeader]) -> Self {
        Self {
            meta: program_headers
                .iter()
                .map(|section| {
                    (
                        SectionDimensions {
                            offset: section.p_offset,
                            size: section.p_filesz,
                        },
                        ProgramHeaderUpdate::no_updates(),
                    )
                })
                .collect(),
            output: program_headers.to_vec(),
        }
    }

    /// Every time a file section is updated we might need to update a program section that holds
    /// it.  This method does it, under an assumption that a file section start or end with match a
    /// program section start or end, respectively.  And that there should be only one such match.
    ///
    /// It does a linear search through program sections, but there should not be that many of them.
    fn observe_file_section(&mut self, old: SectionDimensions, new: SectionDimensions) {
        let Self { meta, output } = self;

        let SectionDimensions {
            offset: old_offset,
            size: old_size,
        } = old;
        let SectionDimensions {
            offset: new_offset,
            size: new_size,
        } = new;

        if let Some(i) = meta
            .iter()
            .position(|(SectionDimensions { offset, .. }, _)| *offset == old_offset)
        {
            let updates = &mut meta[i].1;

            assert!(
                !updates.start,
                "Program section at offset 0x{old_offset:0>16x}: Two file sections coincide with \
                 the start of this program section.\n\
                 This tool code does not support ELF files with such structure, as it makes it \
                 harder to know when such a program section offset needs to be updated.",
            );

            updates.start = true;
            output[i].p_offset = new_offset;
        };

        if let Some(i) = meta
            .iter()
            .position(|(SectionDimensions { offset, size }, _)| {
                offset + size == old_offset + old_size
            })
        {
            let updates = &mut meta[i].1;
            let output = &mut output[i];

            assert!(
                !updates.end,
                "Program section at offset 0x{old_offset:0>16x}: Two file sections coincide with \
                 the start of this program section.\n\
                 This tool code does not support ELF files with such structure, as it makes it \
                 harder to know when such a program section offset needs to be updated.",
            );

            updates.end = true;
            // This is a bit tricky, as we need to compute the program section size, but we only
            // know the file section size.  And the file section may not cover the whole program
            // section.  So we need to go to absolute values and then back to relative.
            let new_filesz = new_offset
                .checked_add(new_size)
                .expect("File section size end fits into u64")
                .checked_sub(output.p_offset)
                .expect("Program section size is positive");
            let size_adjustment = strict_signed_diff(new_filesz, output.p_filesz);
            output.p_filesz = new_filesz;
            output.p_memsz = output.p_memsz.checked_add_signed(size_adjustment).expect(
                "Program section p_memsz is positive and fits into u64 after an adjustment",
            );
        };
    }

    fn into_result(self) -> Vec<ProgramHeader> {
        let Self { meta, output } = self;

        for (i, (_, ProgramHeaderUpdate { start, end })) in meta.into_iter().enumerate() {
            let target = &output[i];
            assert!(
                start,
                "Program section at offset 0x{:0>16x}: No file sections coincide with the start of \
                 this program section.\n\
                 This tool code does not support ELF files with such structure, as it makes it \
                 harder to know when such a program section offset needs to be updated.",
                target.p_offset,
            );
            assert!(
                end,
                "Program section at offset 0x{:0>16x}: No file sections coincide with the end of \
                 this program section.\n\
                 This tool code does not support ELF files with such structure, as it makes it \
                 harder to know when such a program section size needs to be updated.",
                target.p_offset,
            );
        }

        output
    }
}

/// Runs section transformation for the whole file, without producing any outputs.  But records size
/// changes for each section, which allows us to compute correct updates for the ELF header, program
/// section headers and section headers table in one go.
///
/// Returns a mapping from the existing file section offset to that section size adjustment.
pub fn compute_shifts<SectionTransformer>(
    input_bytes: &[u8],
    input_program_headers: &[ProgramHeader],
    input_section_headers: &[SectionHeader],
    ctx: Ctx,
    transformer: SectionTransformer,
) -> ComputeShiftsResult
where
    SectionTransformer: for<'bytes, 'header, 'output> Fn(
        /* input_bytes: */ &'bytes [u8],
        /* section_header: */ &'header SectionHeader,
        /* ctx: */ Ctx,
        /* output: */ &'output mut dyn io::Write,
    ) -> Option<u64>,
{
    let mut vacant_at = match input_section_headers.first() {
        Some(first_section_header) => first_section_header.sh_offset,
        None => {
            return ComputeShiftsResult {
                program_headers: vec![],
                section_headers: vec![],
                section_headers_start: 0,
            }
        }
    };

    let mut input_section_headers = input_section_headers.iter();

    let mut output_program_headers_updater =
        OutputProgramHeadersUpdater::new(input_program_headers);
    let mut output_section_headers = Vec::with_capacity(input_section_headers.len());

    while let Some(input_section_header) = input_section_headers.next() {
        let new_section_size =
            match transformer(input_bytes, &input_section_header, ctx, &mut io::empty()) {
                Some(new_size) => new_size,
                None => input_section_header.sh_size,
            };

        let old_section_offset = input_section_header.sh_offset;
        let input_section_alignment = input_section_header.sh_addralign;
        let new_section_offset = if input_section_alignment <= 1 {
            vacant_at
        } else {
            vacant_at.next_multiple_of(input_section_alignment)
        };

        let old_section_size = input_section_header.sh_size;

        output_section_headers.push(SectionHeader {
            sh_offset: new_section_offset,
            sh_size: new_section_size,
            ..input_section_header.clone()
        });

        output_program_headers_updater.observe_file_section(
            SectionDimensions {
                offset: old_section_offset,
                size: old_section_size,
            },
            SectionDimensions {
                offset: new_section_offset,
                size: new_section_size,
            },
        );

        vacant_at = new_section_offset + new_section_size;
    }

    ComputeShiftsResult {
        program_headers: output_program_headers_updater.into_result(),
        section_headers: output_section_headers,
        section_headers_start: vacant_at,
    }
}

#[cfg(test)]
mod tests {
    use crate::transformer::ComputeShiftsResult;

    use super::compute_shifts;

    use std::io;

    use goblin::{
        container::Ctx,
        elf::{self, ProgramHeader, SectionHeader},
    };
    use pretty_assertions::assert_eq;

    // We only care about program section offsets and sizes, so is nice to have a helper that
    // populates the rest with arbitrary values.
    fn test_program_header(p_offset: u64, p_filesz: u64, p_align: u64) -> ProgramHeader {
        ProgramHeader {
            p_type: elf::program_header::PT_HIOS,
            p_flags: 262_999,
            p_offset,
            p_vaddr: p_offset + 89_991,
            p_paddr: p_offset + 60_877,
            p_filesz,
            p_memsz: p_filesz,
            p_align,
        }
    }

    // We only care about section offsets, sizes, and alignment, and a little about the names, so it
    // is nice to have a helper that populates the rest with arbitrary values.
    fn test_section_header(
        sh_name: usize,
        sh_offset: u64,
        sh_size: u64,
        sh_addralign: u64,
    ) -> SectionHeader {
        assert!(
            sh_addralign == 0 || (sh_addralign.count_ones() == 1 && sh_addralign <= 64),
            "sh_addralign must be a zero or a power of two, up to 64.\n\
             Got: {sh_addralign}",
        );

        SectionHeader {
            sh_name,
            sh_type: elf::section_header::SHT_HIUSER,
            sh_flags: 20_724_251,
            sh_addr: 148_258_883,
            sh_offset,
            sh_size,
            sh_link: 90_103,
            sh_info: 295_353,
            sh_addralign,
            sh_entsize: 512_515_680,
        }
    }

    fn noop_transformer(
    ) -> Box<impl Fn(&[u8], &SectionHeader, Ctx, &mut dyn io::Write) -> Option<u64>> {
        Box::new(
            move |_input_bytes: &[u8],
                  _section_header: &SectionHeader,
                  _ctx: Ctx,
                  _output: &mut dyn io::Write|
                  -> Option<u64> { None },
        )
    }

    fn adjust_single_section(
        target_section_name: usize,
        adjustment: i64,
    ) -> Box<impl Fn(&[u8], &SectionHeader, Ctx, &mut dyn io::Write) -> Option<u64>> {
        Box::new(
            move |_input_bytes: &[u8],
                  section_header: &SectionHeader,
                  _ctx: Ctx,
                  _output: &mut dyn io::Write|
                  -> Option<u64> {
                (section_header.sh_name == target_section_name).then(|| {
                    section_header
                        .sh_size
                        .checked_add_signed(adjustment)
                        .expect("Adjusted section size fits into u64")
                })
            },
        )
    }

    #[test]
    fn compute_shifts_noop_no_change() {
        let input_program_headers = vec![test_program_header(140, 24, 4)];
        let input_section_headers = vec![
            test_section_header(1, 140, 15, 0),
            test_section_header(2, 160, 4, 16),
            test_section_header(3, 164, 4, 4),
        ];

        let res = compute_shifts(
            &[],
            &input_program_headers,
            &input_section_headers,
            Ctx::default(),
            noop_transformer(),
        );

        assert_eq!(
            res,
            ComputeShiftsResult {
                program_headers: input_program_headers.clone(),
                section_headers: input_section_headers.clone(),
                section_headers_start: 168,
            }
        );
    }

    #[test]
    fn compute_shifts_noop_remove_empty_space_outside_program_section() {
        let input_program_headers = vec![test_program_header(140, 24, 4)];
        let input_section_headers = vec![
            test_section_header(1, 140, 15, 0),
            test_section_header(2, 160, 4, 16),
            test_section_header(3, 168, 4, 4),
        ];

        let res = compute_shifts(
            &[],
            &input_program_headers,
            &input_section_headers,
            Ctx::default(),
            noop_transformer(),
        );

        let expected_section_headers = vec![
            test_section_header(1, 140, 15, 0),
            test_section_header(2, 160, 4, 16),
            test_section_header(3, 164, 4, 4),
        ];

        assert_eq!(
            res,
            ComputeShiftsResult {
                program_headers: input_program_headers.clone(),
                section_headers: expected_section_headers,
                section_headers_start: 168,
            }
        );
    }

    #[test]
    fn compute_shifts_noop_remove_empty_space_inside_program_section() {
        let input_program_headers = vec![test_program_header(140, 40, 4)];
        let input_section_headers = vec![
            test_section_header(1, 140, 15, 0),
            test_section_header(2, 176, 4, 16),
            test_section_header(3, 180, 4, 4),
        ];

        let res = compute_shifts(
            &[],
            &input_program_headers,
            &input_section_headers,
            Ctx::default(),
            noop_transformer(),
        );

        let expected_program_headers = vec![test_program_header(140, 24, 4)];
        let expected_section_headers = vec![
            test_section_header(1, 140, 15, 0),
            test_section_header(2, 160, 4, 16),
            test_section_header(3, 164, 4, 4),
        ];

        assert_eq!(
            res,
            ComputeShiftsResult {
                program_headers: expected_program_headers,
                section_headers: expected_section_headers,
                section_headers_start: 168,
            }
        );
    }

    #[test]
    fn compute_shifts_one_size_increase_within_padding() {
        let input_program_headers = vec![test_program_header(140, 24, 4)];
        let input_section_headers = vec![
            test_section_header(1, 140, 15, 0),
            test_section_header(2, 160, 4, 16),
            test_section_header(3, 168, 4, 4),
        ];

        let res = compute_shifts(
            &[],
            &input_program_headers,
            &input_section_headers,
            Ctx::default(),
            adjust_single_section(2, 3),
        );

        let expected_program_headers = vec![test_program_header(140, 27, 4)];
        let expected_section_headers = vec![
            test_section_header(1, 140, 15, 0),
            test_section_header(2, 160, 7, 16),
            test_section_header(3, 168, 4, 4),
        ];

        assert_eq!(
            res,
            ComputeShiftsResult {
                program_headers: expected_program_headers,
                section_headers: expected_section_headers,
                section_headers_start: 172,
            }
        );
    }

    #[test]
    fn compute_shifts_one_size_increase_with_alignment_update() {
        let input_program_headers = vec![test_program_header(140, 24, 4)];
        let input_section_headers = vec![
            test_section_header(1, 140, 15, 0),
            test_section_header(2, 160, 4, 16),
            test_section_header(3, 164, 4, 4),
        ];

        let res = compute_shifts(
            &[],
            &input_program_headers,
            &input_section_headers,
            Ctx::default(),
            adjust_single_section(2, 1),
        );

        let expected_program_headers = vec![test_program_header(140, 25, 4)];
        let expected_section_headers = vec![
            test_section_header(1, 140, 15, 0),
            test_section_header(2, 160, 5, 16),
            test_section_header(3, 168, 4, 4),
        ];

        assert_eq!(
            res,
            ComputeShiftsResult {
                program_headers: expected_program_headers,
                section_headers: expected_section_headers,
                section_headers_start: 172,
            }
        );
    }
}
