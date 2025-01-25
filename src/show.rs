///! Helpers for inspection of the input ELF.

use goblin::{
    container::Ctx,
    elf::{self, Elf, Header, ProgramHeader, SectionHeader},
};
use scroll::ctx::SizeWith as _;

use crate::{args::show::ShowArgs, inspect::{find_current_entrypoint, SymbolInfo}};

pub fn run(input_bytes: &[u8], elf: &Elf, ctx: Ctx, args: ShowArgs) {
    match args {
        ShowArgs::Header => print_header(elf, ctx),
        ShowArgs::Layout => print_layout(input_bytes, elf, ctx),
        ShowArgs::ProgramSections => print_program_sections(elf),
        ShowArgs::FileSegments => print_file_segments(&elf),
        ShowArgs::DynSym => print_dynsyms(elf),
        ShowArgs::ShStrTab => print_shstrtab(elf),
        ShowArgs::Relocations => print_relocations(elf),
        ShowArgs::Entrypoint => print_entrypoint(elf),
    }
}

fn print_header(elf: &Elf, ctx: Ctx) {
    println!("ELF header offsets: 0x{:0>16x} - 0x{:0>16x}",
        0,
        Header::size_with(&ctx)
    );

    println!("{:#?}", elf.header);
}

pub fn print_layout(input_bytes: &[u8], elf: &Elf, ctx: Ctx) {
    println!("Input file data size: 0x{:0>16x}", input_bytes.len());

    println!("File type: {:?}", elf.header.e_type);

    println!("ELF header:");
    println!(
        "  {:16}: 0x{:0>16x} - 0x{:0>16x}",
        "",
        0,
        Header::size_with(&ctx)
    );

    println!("Program section header table:");
    {
        let elf::Header {
            e_phoff,
            e_phentsize,
            e_phnum,
            ..
        } = elf.header;
        println!(
            "  0x{:0>16x} - 0x{:0>16x}",
            e_phoff,
            e_phoff + u64::from(e_phentsize) * u64::from(e_phnum)
        );
    }

    print_program_sections(&elf);
    print_file_segments(&elf);

    println!("File segment header table:");
    {
        let elf::Header {
            e_shoff,
            e_shentsize,
            e_shnum,
            ..
        } = elf.header;
        println!(
            "  0x{:0>16x} - 0x{:0>16x}",
            e_shoff,
            e_shoff + u64::from(e_shentsize) * u64::from(e_shnum)
        );
    }

    println!("Input file data size: 0x{:0>16x}", input_bytes.len());
}

fn print_program_sections(elf: &Elf) {
    println!("All programs sections byte offsets:");
    for ProgramHeader {
        p_type,
        p_offset,
        p_filesz,
        p_vaddr,
        p_paddr,
        p_memsz,
        p_align,
        ..
    } in &elf.program_headers
    {
        println!(
            "  {:16}: 0x{:0>16x} - 0x{:0>16x}, \
             paddr: 0x{:0>16x}, vaddr: 0x{:0>16x}, memsz: 0x{:0>16x}, align: {}",
            elf::program_header::pt_to_str(*p_type),
            p_offset,
            p_offset + p_filesz,
            p_paddr,
            p_vaddr,
            p_memsz,
            p_align,
        );
    }
}

fn print_file_segments(elf: &Elf) {
    println!("All file segments byte offsets:");
    for SectionHeader {
        sh_name,
        sh_offset,
        sh_size,
        sh_addralign,
        ..
    } in &elf.section_headers
    {
        println!(
            "  {:16}: 0x{:0>16x} - 0x{:0>16x}, align: {}",
            elf.shdr_strtab.get_at(*sh_name).unwrap_or("---"),
            sh_offset,
            sh_offset + sh_size,
            sh_addralign,
        );
    }
}

fn print_dynsyms(elf: &Elf) {
    println!("Dynamic symbols ({}):", elf.dynsyms.len());
    for symbol in elf.dynsyms.iter() {
        println!(
            "  {}:",
            elf.dynstrtab.get_at(symbol.st_name).unwrap_or("---")
        );
        println!("    {:?}", symbol);
    }

    println!(".dynstr content:");
    for string in elf.dynstrtab.to_vec().expect("Input .dynstr is parsable") {
        println!("  \"{string}\"");
    }
}

fn print_shstrtab(elf: &Elf) {
    println!(".shstrtab content:");
    for string in elf
        .shdr_strtab
        .to_vec()
        .expect("Input .shstrtab is parsable")
    {
        println!("  \"{string}\"");
    }
}

fn print_relocations(elf: &Elf) {
    println!("TODO: Just the counts for now");

    println!("elf.dynrelas: {:#?}", elf.dynrelas.len());
    println!("elf.dynrels: {:#?}", elf.dynrels.len());
    println!("elf.pltrelocs: {:#?}", elf.pltrelocs.len());
    println!("elf.shdr_relocs: {:#?}", elf.shdr_relocs.len());
}

fn print_entrypoint(elf: &Elf) {
    let Some(SymbolInfo { offset, size }) = find_current_entrypoint(elf) else {
        println!("Input does not have an \"entrypoint\" dynamic symbol");
        return;
    };

    println!(
        "\"entrypoint\" address: 0x{:0>16x} - 0x{:0>16x}, size: 0x{:0>8x}",
        offset,
        offset + size,
        size,
    );
}
