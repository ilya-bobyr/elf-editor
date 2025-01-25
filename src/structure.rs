//! ELF files could have different structures, and this tool only supports ELF files that are bayed
//! out in certain order.  It is just a limitation that reduced the implementation effort.
//!
//! This module provides functionality for checking that the structure is as expected by the rest of
//! the code.

use goblin::{
    container::Ctx,
    elf::{self, Elf, Header, SectionHeader},
};
use scroll::ctx::SizeWith;

/// Verifies that the ELF structure matches all the assumptions the rest of the functions expect.
/// Should be called for the input file ELF.
///
/// Returns true if it does.  Prints an explanation and returns false if it does not.
pub fn verify_elf_structure(bytes: &[u8], elf: &Elf, ctx: Ctx) -> Result<(), String> {
    macro_rules! check_that {
        ($cond:expr, on_fail: $($on_fail:tt)*) => {
            if !$cond {
                return Err(format!($($on_fail)*));
            }
        };
    }

    macro_rules! must_be_zero_bytes_gap {
        ($bytes:expr, on_fail: $($on_fail:tt)*) => {
            if $bytes.iter().any(|&v| v != 0) {
                return Err(format!($($on_fail)*));
            }
        };
    }

    // We expect all the bytes in the ELF to be covered by known structures, except that there could
    // be zero byte gaps.
    //
    // I've seen them used for alignment purposes.  One option would be to be more strict and only
    // allow for zero byte gaps in cases when we size is does not fall on an word boundary (for some
    // word size). But as I am not sure if there is a certain alignment requirements or conventions
    // for ELF sections.  And I've seen at least two different cases.  So allowing for arbitrary
    // gaps, as long as they are all zero bytes.
    let mut covered_up_to = Header::size_with(&ctx) as u64;

    {
        let elf::Header {
            e_phoff,
            e_phentsize,
            e_phnum,
            ..
        } = elf.header;
        let size = u64::from(e_phentsize) * u64::from(e_phnum);

        if e_phoff < covered_up_to {
            return Err(format!(
                "Program section headers table overlaps with the ELF header.\n\
                 Program section headers table offset: 0x{e_phoff:x}, size: 0x{size:x}\n\
                 ELF header ends at: 0x{covered_up_to:x}",
            ));
        } else if e_phoff > covered_up_to {
            must_be_zero_bytes_gap! {
                bytes[covered_up_to as usize .. e_phoff as usize],
                on_fail:
                "There is a non-zero byte gap between the ELF header and the program section \
                 headers table.\n\
                 Program section headers table offset: 0x{e_phoff:x}, size: 0x{size:x}\n\
                 Last section ends at: 0x{covered_up_to:x}",
            }
        }

        covered_up_to = e_phoff + size;
    }

    let file_sections = elf.section_headers.as_slice();

    check_that! {
        file_sections.len() > 1,
        on_fail:
        "ELF must have at least 2 sections.  Got: {}",
        file_sections.len()
    };

    {
        let SectionHeader {
            sh_offset, sh_size, ..
        } = &file_sections[0];
        check_that! {
            *sh_offset == 0 && *sh_size == 0,
            on_fail:
            "First section is not 0/0.\n\
             Got offset: 0x{sh_offset:x}, size: 0x{sh_size:x}",
        };
    }

    for SectionHeader {
        sh_name,
        sh_offset,
        sh_size,
        ..
    } in &file_sections[1..]
    {
        if *sh_offset < covered_up_to {
            return Err(format!(
                "Section offset points to a range already covered by a previous section.\n\
                 Section name: {}, offset: 0x{:x}, size: 0x{:x}\n\
                 Previous section ends at: 0x{:x}",
                elf.shdr_strtab.get_at(*sh_name).unwrap_or("---"),
                sh_offset,
                sh_size,
                covered_up_to,
            ));
        } else if *sh_offset > covered_up_to {
            must_be_zero_bytes_gap! {
                bytes[covered_up_to as usize .. *sh_offset as usize],
                on_fail:
                "There is a non-zero byte gab after the previous section end.\n\
                 Section name: {}, offset: 0x{:x}, size: 0x{:x}\n\
                 Previous section ends at: 0x{:x}",
                elf.shdr_strtab.get_at(*sh_name).unwrap_or("---"),
                sh_offset,
                sh_size,
                covered_up_to,
            };
        }

        covered_up_to = sh_offset + sh_size;
    }

    {
        let elf::Header {
            e_shoff,
            e_shentsize,
            e_shnum,
            ..
        } = elf.header;
        let size = u64::from(e_shentsize) * u64::from(e_shnum);

        if e_shoff < covered_up_to {
            return Err(format!(
                "Section headers table starts at a point that is already covered by the previous \
                 section.\n\
                 Section headers table offset: 0x{e_shoff:x}, size: 0x{size:x}\n\
                 Last section ends at: 0x{covered_up_to:x}",
            ));
        } else if e_shoff > covered_up_to {
            must_be_zero_bytes_gap! {
                bytes[covered_up_to as usize .. e_shoff as usize],
                on_fail:
                "There is a non-zero byte gap between the last section and the section headers \
                 table.\n\
                 Section headers table offset: 0x{e_shoff:x}, size: 0x{size:x}\n\
                 Last section ends at: 0x{covered_up_to:x}",
            };
        }

        covered_up_to = e_shoff + size;
    }

    must_be_zero_bytes_gap! {
        bytes[covered_up_to as usize .. bytes.len()],
        on_fail:
        "There are non-zero bytes after the section headers table that is expected to be the last \
         element of the file.\n\
         Section headers table end: 0x{covered_up_to:x}\n\
         File size: 0x{:x}",
        bytes.len(),
    };

    Ok(())
}
