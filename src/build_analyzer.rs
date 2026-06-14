use elf::segment::ProgramHeader;
use log::*;
use std::option::Option;
use std::path::PathBuf;
use std::{collections::HashMap, error::Error};

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
    // Build a Vec of (start, end) pairs
    let mut phdr_sorted = program_headers
        .iter()
        .map(|phdr| (phdr.p_vaddr, phdr.p_vaddr + phdr.p_memsz))
        .collect::<Vec<_>>();

    // Merge overlapping ranges
    // Sort by start address
    phdr_sorted.sort();
    let mut phdr_ranges = Vec::<(u64, u64)>::new();
    phdr_sorted.into_iter().for_each(|(start, end)| {
        if let Some(last_mut) = phdr_ranges.last_mut()
            && last_mut.1 >= start
        {
            last_mut.1 = end;
        } else {
            phdr_ranges.push((start, end));
        }
    });

    debug!(
        target: "phdr collapse",
        "Collapsed {} programs into {} contiguous address ranges",
        program_headers.len(),
        phdr_ranges.len()
    );
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
        .map(|(addr, my_word)| {
            debug!(
                target: "hardcoded pointers",
                "Hardcoded pointer: {0} | {1} | 0x{addr:08X} | 0x{my_word:08X}", elf.filename, sym.name
            )
        })
        .count()
}

pub fn analyze_build(
    basedir: &String,
    buildname: &Option<String>,
    name: &String,
    source_map: &HashMap<String, (String, bool)>,
) -> Result<Stats, Box<dyn Error>> {
    let mut stats = Stats {
        c_code_bytes: 0,
        c_data_bytes: 0,
        asm_code_bytes: 0,
        asm_data_bytes: 0,
        resolved_pointers: 0,
        hardcoded_pointers: 0,
    };

    let build_path = [
        Some(basedir),
        Some(&String::from("build")),
        buildname.as_ref(),
    ]
    .iter()
    .filter_map(|x| x.to_owned())
    .map(|x| x.to_owned())
    .collect::<Vec<_>>()
    .join("/");

    // Load the xMAP file
    let xmap_name = std::format!("{}/{}.elf.xMAP", build_path, name);
    let xmap = xmap_file::parse_xmap(&xmap_name, source_map)?;

    // Read the ELF file into memory and make sure it does in fact represent an NDS binary
    let elf_name = std::format!("{}/{}.elf", build_path, name);
    let elf_file = ElfFile::from_path(&elf_name);
    let elf_segment_bounds = segments_to_ranges(&elf_file.segments);
    source_map.iter().for_each(|(_stem, (subpath, is_cfile))| {
        let ofile_path = format!("{}/{}.o", build_path, subpath);
        let ofile_pathbuf = PathBuf::from(&ofile_path);
        if !ofile_pathbuf.exists() {
            warn!("no such file or directory: {}", ofile_path);
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
            .filter_map(|nsym| {
                if nsym.sym.st_size == 0 {
                    None
                } else if let Some(rxsym) = xmapped_syms.get(&nsym.name)
                    && (*is_cfile || rxsym.section_name == nsym.name)
                {
                    Some((nsym, rxsym))
                } else {
                    None
                }
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

    Ok(stats)
}
