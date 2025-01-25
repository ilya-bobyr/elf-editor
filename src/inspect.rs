///! Helpers for inspection of the input ELF.

use goblin::{elf::Elf, strtab::Strtab};

pub fn find_in_strtab(strtab: &Strtab, target: &str) -> Option<usize> {
    for i in 0..strtab.len() {
        let Some(name) = strtab.get_at(i) else {
            continue;
        };

        if name == target {
            return Some(i);
        }
    }

    None
}

pub struct SymbolInfo {
    pub offset: u64,
    pub size: u64,
}

pub fn find_current_entrypoint(elf: &Elf) -> Option<SymbolInfo> {
    let Some(entrypoint_st_name) = find_in_strtab(&elf.dynstrtab, "entrypoint") else {
        return None;
    };

    elf.dynsyms
        .iter()
        .find(|symbol| symbol.st_name == entrypoint_st_name)
        .map(|symbol| SymbolInfo {
            offset: symbol.st_value,
            size: symbol.st_size,
        })
}
