use elf::segment::ProgramHeader;
use std::collections::HashMap;
use std::option::Option;
use std::path::PathBuf;

use crate::build_analyzer::elf_file::{ElfFile, NamedSymbol};

mod elf_file;
mod xmap_file;

/// Collects statistics about the decompilation effort based on the build outputs
pub struct Stats {
    pub c_code_bytes: usize,
    pub c_data_bytes: usize,
    pub asm_code_bytes: usize,
    pub asm_data_bytes: usize,
    pub resolved_pointers: usize,
    pub hardcoded_pointers: usize,
}

fn segments_to_ranges(program_headers: &Vec<ProgramHeader>) -> Vec<(u64, u64)> {
    let mut phdr_sorted = program_headers
        .iter()
        .map(|phdr| (phdr.p_vaddr, phdr.p_vaddr + phdr.p_memsz))
        .collect::<Vec<_>>();
    phdr_sorted.sort();
    let mut phdr_ranges = Vec::<(u64, u64)>::new();
    phdr_sorted.into_iter().for_each(|(start, end)| {
        if phdr_ranges.is_empty() || phdr_ranges.last_mut().unwrap().1 <= start {
            phdr_ranges.push((start, end));
        } else {
            phdr_ranges.last_mut().unwrap().1 = end;
        }
    });
    phdr_ranges
}

fn count_hardcoded_pointers(
    sym: &NamedSymbol,
    elf: &ElfFile,
    phdr_ranges: &Vec<(u64, u64)>,
) -> usize {
    let raw_data = elf.symbol_data(sym).expect("failed to parse sym data");
    let sh_addr = sym.sym.st_value;
    // Loop over 32-bit words
    raw_data
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .map(|(idx, word_raw)| {
            (
                sh_addr + u64::try_from(4 * idx).unwrap(),
                u64::from(u32::from_le_bytes(*word_raw)),
            )
        })
        .filter(|(addr, my_word)| {
            *my_word >= 0x01000000
                && phdr_ranges
                    .iter()
                    .find(|region| region.0 <= *my_word && *my_word < region.1)
                    .is_some()
                && elf.rels.iter().find(|rel| rel.r_offset == *addr).is_none()
                && elf.relas.iter().find(|rel| rel.r_offset == *addr).is_none()
        })
        .count()
}

pub fn analyze_build(
    basedir: &String,
    buildname: Option<&String>,
    name: &str,
    source_map: &HashMap<String, (String, bool)>,
) -> Stats {
    let mut stats = Stats {
        c_code_bytes: 0,
        c_data_bytes: 0,
        asm_code_bytes: 0,
        asm_data_bytes: 0,
        resolved_pointers: 0,
        hardcoded_pointers: 0,
    };

    let build_subdir: String;
    match buildname {
        Some(my_name) => {
            build_subdir = std::format!("build/{}", my_name);
        }
        None => {
            build_subdir = String::from("build");
        }
    }
    let build_path = std::format!("{}/{}", basedir, build_subdir);

    // Load the xMAP file
    let xmap_name = std::format!("{}/{}.elf.xMAP", build_path, name);
    let xmap = xmap_file::parse_xmap(&xmap_name, source_map);

    // Read the ELF file into memory and make sure it does in fact represent an NDS binary
    let elf_name = std::format!("{}/{}.elf", build_path, name);
    let elf_file = ElfFile::from_path(&elf_name);
    let elf_segment_bounds = segments_to_ranges(&elf_file.segments);
    source_map.iter().for_each(|(_stem, (subpath, is_cfile))| {
        let ofile_path = format!("{}/{}.o", build_path, subpath);
        let ofile_pathbuf = PathBuf::from(&ofile_path);
        if !ofile_pathbuf.exists() {
            eprintln!("no such file or directory: {}", ofile_path);
            return;
        }
        let ofile_elf = ElfFile::from_path(&ofile_path);
        stats.resolved_pointers += ofile_elf.rels.len() + ofile_elf.relas.len();
        let Some(xmapped_syms) = xmap.get(&(subpath.clone(), *is_cfile)) else {
            return;
        };

        // Select syms that appear in the xmap file
        ofile_elf
            .symbols
            .iter()
            .filter(|nsym| nsym.sym.st_size != 0)
            .map(|nsym| (nsym, xmapped_syms.get(&nsym.name)))
            .filter_map(|(nsym, xsym)| match xsym {
                Some(rxsym) => {
                    if *is_cfile || rxsym.section_name == nsym.name {
                        Some((nsym, rxsym))
                    } else {
                        None
                    }
                }
                None => None,
            })
            .for_each(|(nsym, sym)| {
                let counter = match (*is_cfile, sym.is_code) {
                    (true, true) => &mut stats.c_code_bytes,
                    (true, false) => &mut stats.c_data_bytes,
                    (false, true) => &mut stats.asm_code_bytes,
                    (false, false) => &mut stats.asm_data_bytes,
                };
                *counter += sym.size;
                stats.hardcoded_pointers +=
                    count_hardcoded_pointers(&nsym, &ofile_elf, &elf_segment_bounds);
            });
    });

    stats
}
