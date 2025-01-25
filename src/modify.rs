use std::{fs::File, io};

use goblin::{
    container::Ctx,
    elf::{Elf, SectionHeader},
};

use crate::args::modify::{ModifyArgs, ModifyCommand};

mod dyn_sym;

pub fn run(
    input_bytes: &[u8],
    elf: &Elf,
    ctx: Ctx,
    ModifyArgs {
        output: output_path,
        command,
    }: ModifyArgs,
) {
    let output = match File::create(&output_path) {
        Ok(output) => output,
        Err(err) => {
            println!(
                "Failed to open the output file: {}\n\
                 Error: {}",
                output_path.to_string_lossy(),
                err,
            );
            return;
        }
    };

    match command {
        ModifyCommand::DynSym(args) => dyn_sym::run(input_bytes, elf, ctx, output, args),
    }
}

#[allow(unused)]
fn keep_all_sections_as_is() -> Box<
    impl for<'bytes, 'header, 'output> Fn(
        /* input_bytes: */ &'bytes [u8],
        /* section_header: */ &'header SectionHeader,
        /* ctx: */ Ctx,
        /* output: */ &'output mut dyn io::Write,
    ) -> Option<u64>,
> {
    Box::new(
        move |_input_bytes: &[u8],
              _section_headers: &SectionHeader,
              _ctx: Ctx,
              _output: &mut dyn io::Write|
              -> Option<u64> { None },
    )
}
