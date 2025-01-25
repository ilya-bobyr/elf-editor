use std::path::PathBuf;

use clap::{Args, Subcommand};

pub mod dyn_sym;

#[derive(Args, Debug)]
#[command(name = "modify")]
pub struct ModifyArgs {
    #[arg(long, value_name = "OUTPUT")]
    /// Output ELF file to generate.
    pub output: PathBuf,

    #[command(subcommand)]
    pub command: ModifyCommand,
}

#[derive(Subcommand, Debug)]
pub enum ModifyCommand {
    #[command(subcommand)]
    /// Modify the .dynsym section, holding the loader dynamic symbols.
    DynSym(dyn_sym::DynSymArgs),
}
