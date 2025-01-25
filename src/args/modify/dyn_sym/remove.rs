use clap::Args;

#[derive(Args, Debug)]
pub struct RemoveArgs {
    /// Name of the symbol to remove.
    pub name: String,
}
