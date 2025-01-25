use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub mod show;
pub mod modify;

/// Editor for ELF files.
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[arg(long, value_name = "INPUT")]
    /// Input ELF file to process.
    pub input: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

/// A specific action to perform.
#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(subcommand)]
    /// Show the input file
    Show(show::ShowArgs),

    /// Modify the input file
    Modify(modify::ModifyArgs),
}
