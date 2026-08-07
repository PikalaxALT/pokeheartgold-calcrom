use crate::build_analyzer::elf_file::{ElfFile, NamedSymbol};
use crate::build_analyzer::xmap_file::XmapSymbol;
use elf::segment::ProgramHeader;
use itertools::Itertools;
use log::debug;
use std::path::Path;
use std::{collections::HashMap, option::Option};
mod elf_file;
mod xmap_file;
use anyhow::Result;

/// Collects statistics about the decompilation effort based on the build outputs
#[derive(Default)]
pub struct Stats {
    pub c_code_bytes: usize,
    pub c_data_bytes: usize,
    pub asm_code_bytes: usize,
    pub asm_data_bytes: usize,
    pub resolved_pointers: usize,
    pub hardcoded_pointers: usize,
}

fn segments_to_ranges(program_headers: &[ProgramHeader]) -> Vec<(u64, u64)> {
    // Build a Vec of (start, end) pairs
    let mut phdr_sorted = program_headers
        .iter()
        .map(|phdr| (phdr.p_vaddr, phdr.p_vaddr.saturating_add(phdr.p_memsz)))
        .collect_vec();

    // Merge overlapping ranges
    // Sort by start address
    phdr_sorted.sort_unstable();
    let mut phdr_ranges = Vec::<(u64, u64)>::new();
    for (start, end) in phdr_sorted {
        if let Some((_, lmend)) = phdr_ranges.last_mut()
            && *lmend >= start
        {
            *lmend = end;
        } else {
            phdr_ranges.push((start, end));
        }
    }

    debug!(
        target: "phdr collapse",
        "Collapsed {} programs into {} contiguous address ranges",
        program_headers.len(),
        phdr_ranges.len()
    );
    phdr_ranges
}

/// Count 32-bit words that are possibly hard-coded pointers
/// This is a liberal upper bound that counts all words whose
/// values are possible addresses in the final ROM
#[cfg(debug_assertions)]
fn count_hardcoded_pointers(
    sym: &NamedSymbol,
    elf: &ElfFile,
    phdr_ranges: &[(u64, u64)],
    rxsym: &XmapSymbol,
) -> Result<usize> {
    let raw_data = elf.symbol_data(sym)?;
    // Loop over 32-bit words
    let num = raw_data
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .map(|(idx, word_raw)| -> Result<(u64, u64)> {
            let my_word = u64::from(u32::from_le_bytes(word_raw.to_owned()));
            let (my_addr, _) = u64::try_from(idx)?.carrying_mul_add(4, 0, rxsym.addr);
            Ok((my_addr, my_word))
        })
        .process_results(|iter| iter.collect_vec())?
        .into_iter()
        .filter(|(_, my_word)| {
            *my_word >= 0x0100_0000_u64
                && phdr_ranges
                    .iter()
                    .any(|region| region.0 <= *my_word && *my_word < region.1)
        })
        .inspect(|(my_addr, my_word)| {
            debug!("hardcoded pointer at 0x{my_addr:08X} --> 0x{my_word:08X}");
        })
        .count();
    Ok(num)
}

#[cfg(not(debug_assertions))]
fn count_hardcoded_pointers(
    sym: &NamedSymbol,
    elf: &ElfFile,
    phdr_ranges: &Vec<(u64, u64)>,
    _rxsym: &XmapSymbol,
) -> Result<usize> {
    let raw_data = elf.symbol_data(sym)?;
    // Loop over 32-bit words
    let num = raw_data
        .as_chunks::<4>()
        .0
        .to_owned()
        .into_iter()
        .filter(|word_raw| -> bool {
            let my_word = u64::from(u32::from_le_bytes(word_raw.to_owned()));
            my_word >= 0x01000000u64
                && phdr_ranges
                    .iter()
                    .find(|region| region.0 <= my_word && my_word < region.1)
                    .is_some()
        })
        .count();
    Ok(num)
}

pub fn analyze_build(
    basedir: &Path,
    buildname: Option<&String>,
    name: &String,
    source_map: &HashMap<String, (String, bool)>,
) -> Result<Stats> {
    debug!("Analyzing build of {}", buildname.unwrap_or(name));
    let mut stats = Stats::default();

    let mut build_path = basedir.join("build");
    if let Some(buildname_s) = buildname {
        build_path = build_path.join(buildname_s);
    }

    // Load the xMAP file
    let xmap_name = build_path.join(format!("{name}.elf.xMAP"));
    let xmap = xmap_file::parse_xmap(&xmap_name, source_map)?;

    // Read the ELF file into memory and make sure it does in fact represent an NDS binary
    let elf_name = build_path.join(format!("{name}.elf"));
    let elf_file = ElfFile::from_path(&elf_name)?;
    let elf_segment_bounds = segments_to_ranges(&elf_file.segments);

    // Count pointers and bytes coming from each source, stratified by C vs ASM and code vs data
    source_map
        .iter()
        .map(|(_stem, (subpath, is_cfile))| -> Result<()> {
            // Get the ELF representing the .o file resulting from this C or ASM object
            // It should exist. Panic if it doesn't.
            debug!("subpath = {subpath}");
            let ofile_path = build_path.join(format!("{subpath}.o"));
            let ofile_elf = ElfFile::from_path(&ofile_path)?;
            // Properly-linked pointers are encoded in REL and RELA sections
            stats.resolved_pointers = stats
                .resolved_pointers
                .saturating_add(ofile_elf.rels.len())
                .saturating_add(ofile_elf.relas.len());

            // Select syms that appear in the xmap file
            let Some(xmapped_syms) = xmap.get(&(subpath.to_owned(), *is_cfile)) else {
                return Ok(());
            };
            ofile_elf
                .symbols
                .iter()
                .map(|nsym| -> Result<()> {
                    if nsym.sym.st_size != 0
                        && let Some(rxsym) = xmapped_syms.get(&nsym.name)
                        && (*is_cfile || rxsym.section_name == nsym.name)
                    {
                        let counter = match (is_cfile, rxsym.is_code) {
                            (true, true) => &mut stats.c_code_bytes,
                            (true, false) => &mut stats.c_data_bytes,
                            (false, true) => &mut stats.asm_code_bytes,
                            (false, false) => &mut stats.asm_data_bytes,
                        };
                        *counter = counter.saturating_add(rxsym.size);
                        stats.hardcoded_pointers =
                            stats
                                .hardcoded_pointers
                                .saturating_add(count_hardcoded_pointers(
                                    nsym,
                                    &ofile_elf,
                                    &elf_segment_bounds,
                                    rxsym,
                                )?);
                    }
                    Ok(())
                })
                .process_results(|iter| iter.collect_vec())?;
            Ok(())
        })
        .process_results(|iter| iter.collect_vec())?;

    Ok(stats)
}

#[cfg(test)]
mod testing {
    use super::count_hardcoded_pointers;
    use crate::build_analyzer::{
        elf_file::{ElfFile, NamedSymbol, SectionHeaderWithData},
        xmap_file::XmapSymbol,
    };
    use elf::{section::SectionHeader, symbol::Symbol};

    #[test]
    fn test_count_hardcoded_pointers() {
        let sym = NamedSymbol {
            name: "foo".into(),
            sym: Symbol {
                st_name: 0,
                st_shndx: 0,
                st_info: 0,
                st_other: 0,
                st_value: 0,
                st_size: 8,
            },
        };
        let rxsym = XmapSymbol {
            section_name: ".rodata".into(),
            is_code: false,
            size: 8,
            #[cfg(debug_assertions)]
            addr: 0x02000000,
        };
        let phdr_ranges = vec![(0x02000000u64, 0x02000800u64)];
        let elffile = ElfFile {
            sections: vec![SectionHeaderWithData {
                data: vec![
                    0x09u8, 0x00u8, 0x00u8, 0x02u8, 0x00u8, 0x00u8, 0x00u8, 0x00u8,
                ],
                shdr: SectionHeader {
                    sh_name: 0,
                    sh_type: 0,
                    sh_flags: 0,
                    sh_addr: 0,
                    sh_offset: 0,
                    sh_size: 0,
                    sh_link: 0,
                    sh_info: 0,
                    sh_addralign: 0,
                    sh_entsize: 0,
                },
            }],
            segments: vec![],
            symbols: vec![],
            rels: vec![],
            relas: vec![],
        };
        let count = count_hardcoded_pointers(&sym, &elffile, &phdr_ranges, &rxsym).unwrap();
        assert_eq!(count, 1);
    }
}
