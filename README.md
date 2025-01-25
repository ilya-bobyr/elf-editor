# ELF editor

An ELF editor for a limited set of operations.

Uses the [goblin](https://github.com/m4b/goblin) library for reading and,
partially, for writing.

## Usage

```
❯ cargo run -- help
Editor for ELF files.

Usage: elf-editor --input <INPUT> <COMMAND>

Commands:
  show    Show the input file
  modify  Modify the input file
  help    Print this message or the help of the given subcommand(s)

Options:
      --input <INPUT>  Input ELF file to process
  -h, --help           Print help
  -V, --version        Print version
```

```
❯ cargo run -- help show
Show the input file

Usage: elf-editor --input <INPUT> show <COMMAND>

Commands:
  header            Show the ELF header
  layout            Overview of the file layout
  program-sections  Show the program sections
  file-segments     Show the file segments
  dyn-sym           Show the .dynsym table and the .dynstr string table content
  sh-str-tab        Show the .shstrtab string table content
  relocations       Show the relocation information. TODO Incomplete for now
  entrypoint        Find a dynamic symbol "entrypoint" and show info on it
  help              Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

```
❯ cargo run -- help modify
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
     Running `target/debug/elf-editor help modify`
Modify the input file

Usage: elf-editor --input <INPUT> modify --output <OUTPUT> <COMMAND>

Commands:
  dyn-sym  Modify the .dynsym section, holding the loader dynamic symbols
  help     Print this message or the help of the given subcommand(s)

Options:
      --output <OUTPUT>  Output ELF file to generate
  -h, --help             Print help
```

```
❯ cargo run -- help modify dyn-sym
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
     Running `target/debug/elf-editor help modify dyn-sym`
Modify the .dynsym section, holding the loader dynamic symbols

Usage: elf-editor modify --output <OUTPUT> dyn-sym <COMMAND>

Commands:
  add     Add an entry to the .dynsym table
  remove  Remove an entry from the .dynsym table
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```
