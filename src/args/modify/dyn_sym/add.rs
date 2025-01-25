use clap::Args;

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Name of the symbol being added.
    pub name: String,

    /// `st_info` field.  TODO Provide a better parser.
    pub info: u8,

    /// `st_other` field.  TODO Provide a better parser.
    pub other: u8,

    // `st_shndx` field.  TODO Provide a better parser.
    pub shndx: usize,

    // Offset of the symbol in the file.
    pub value: u64,

    // Size of the symbol.
    pub size: u64,
}
