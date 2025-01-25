use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[command(name = "show")]
pub enum ShowArgs {
    /// Show the ELF header.
    Header,

    /// Overview of the file layout.
    Layout,

    /// Show the program sections.
    ProgramSections,

    /// Show the file segments.
    FileSegments,

    /// Show the .dynsym table and the .dynstr string table content.
    DynSym,

    /// Show the .shstrtab string table content.
    ShStrTab,

    /// Show the relocation information.
    /// TODO Incomplete for now.
    Relocations,

    /// Find a dynamic symbol "entrypoint" and show info on it.
    ///
    /// Used by the Solana VM loader.
    Entrypoint,
}
