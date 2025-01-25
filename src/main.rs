use std::{fs, io};

use clap::Parser as _;
use goblin::{container::Ctx, elf::Elf};

use structure::verify_elf_structure;

mod args;
mod inspect;
mod modify;
mod show;
mod structure;
mod transformer;

fn main() -> io::Result<()> {
    let args::Args {
        input: input_path,
        command,
    } = args::Args::parse();

    let input_bytes = fs::read(input_path)?;

    let elf = Elf::parse(&input_bytes).expect("Input is an ELF");

    let ctx = Ctx {
        container: elf
            .header
            .container()
            .expect("Input ELF header has a valid size"),
        le: elf
            .header
            .endianness()
            .expect("Input ELF header has a valid endianness"),
    };

    match command {
        args::Command::Show(args) => show::run(&input_bytes, &elf, ctx, args),
        args::Command::Modify(args) => {
            if let Err(err) = verify_elf_structure(&input_bytes, &elf, ctx) {
                println!("Unsupported ELF structure:\n{err}");
                return Ok(());
            };

            modify::run(&input_bytes, &elf, ctx, args);
        }
    }

    Ok(())
}
