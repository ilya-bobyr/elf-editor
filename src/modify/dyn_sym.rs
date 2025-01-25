use std::{fs::File, io};

use goblin::{
    container::Ctx,
    elf::{self, Elf, SectionHeader},
};
use scroll::{ctx::SizeWith, IOwrite};

use crate::{
    args::modify::dyn_sym::{add::AddArgs, remove::RemoveArgs, DynSymArgs},
    inspect::find_in_strtab,
    transformer::transform_elf_sections,
};

pub fn run(input_bytes: &[u8], elf: &Elf, ctx: Ctx, output: File, args: DynSymArgs) {
    match args {
        DynSymArgs::Add(args) => add(input_bytes, elf, ctx, output, args),
        DynSymArgs::Remove(args) => remove(input_bytes, elf, ctx, output, args),
    }
}

fn add(input_bytes: &[u8], elf: &Elf, ctx: Ctx, mut output: File, args: AddArgs) {
    let symbol = elf::Sym {
        // This will be populated by `append_to_dynsyms`.
        st_name: 0,
        st_info: args.info,
        st_other: args.other,
        st_shndx: args.shndx,
        st_value: args.value,
        st_size: args.size,
    };

    transform_elf_sections(
        input_bytes,
        elf,
        ctx,
        &mut output,
        append_to_dynsyms(elf, &args.name, symbol),
    );
}

fn remove(_input_bytes: &[u8], _elf: &Elf, _ctx: Ctx, _output: File, _args: RemoveArgs) {
    todo!("TODO Not implemented yet");
}

/// `symbol.st_name` should be `0`.  It will be replaced by a reference to a new `.dynstr` entry
/// that will hold the `symbol_name` value.
#[allow(unused)]
fn append_to_dynsyms<'symbol_name>(
    elf: &Elf<'_>,
    symbol_name: &'symbol_name str,
    mut symbol: elf::Sym,
) -> Box<
    impl for<'bytes, 'header, 'output> Fn(
            /* input_bytes: */ &'bytes [u8],
            /* section_header: */ &'header SectionHeader,
            /* ctx: */ Ctx,
            /* output: */ &'output mut dyn io::Write,
        ) -> Option<u64>
        + 'symbol_name,
> {
    // We are going to append to the `.dynstr` string table, so the new string will start where the
    // table currently ends.
    let st_name = elf.dynstrtab.len();
    symbol.st_name = st_name;

    let dynstr_sh_name =
        find_in_strtab(&elf.shdr_strtab, ".dynstr").expect("Input ELF has a .dynstr section");
    let dynsym_sh_name =
        find_in_strtab(&elf.shdr_strtab, ".dynsym").expect("Input ELF has a .dynsym section");

    fn copy_existing_section_bytes(
        output: &mut dyn io::Write,
        input_bytes: &[u8],
        sh_offset: u64,
        sh_size: u64,
    ) {
        let input_start = sh_offset as usize;
        let input_end = (sh_offset + sh_size) as usize;
        output
            .write_all(&input_bytes[input_start..input_end])
            .expect("Output can consume all the produced data");
    }

    let process = move |input_bytes: &[u8],
                        SectionHeader {
                            sh_name,
                            sh_offset,
                            sh_size,
                            ..
                        }: &SectionHeader,
                        ctx: Ctx,
                        output: &mut dyn io::Write|
          -> Option<u64> {
        if *sh_name == dynstr_sh_name {
            copy_existing_section_bytes(output, input_bytes, *sh_offset, *sh_size);

            output
                .write_all(symbol_name.as_bytes())
                .expect("Output can consume all the produced data");
            output
                .write_all(&[0])
                .expect("Output can consume all the produced data");

            Some(*sh_size + symbol_name.len() as u64 + 1)
        } else if *sh_name == dynsym_sh_name {
            copy_existing_section_bytes(output, input_bytes, *sh_offset, *sh_size);

            output
                .iowrite_with(symbol, ctx)
                .expect("Output can consume all the produced data");

            Some(*sh_size + elf::Sym::size_with(&ctx) as u64)
        } else {
            None
        }
    };

    Box::new(process)
}
