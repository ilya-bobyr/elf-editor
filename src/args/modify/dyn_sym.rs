use clap::Subcommand;

pub mod add;
pub mod remove;

#[derive(Subcommand, Debug)]
#[command(name = "dyn-sym")]
pub enum DynSymArgs {
    /// Add an entry to the .dynsym table.
    Add(add::AddArgs),

    /// Remove an entry from the .dynsym table.
    Remove(remove::RemoveArgs),
}
