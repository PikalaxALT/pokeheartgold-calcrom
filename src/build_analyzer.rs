use elf::ElfBytes;
use elf::endian::LittleEndian;
use std::collections::HashMap;
use std::option::Option;
use std::path::PathBuf;

use crate::build_analyzer::xmap_file::SingleFileStats;

mod xmap_file;

pub struct Stats {
    pub c_code_bytes: u32,
    pub c_data_bytes: u32,
    pub asm_code_bytes: u32,
    pub asm_data_bytes: u32,
    pub resolved_pointers: u32,
    pub hardcoded_pointers: u32,
}

fn analyze_source_files(stats: &mut Stats, xmap: &HashMap<String, SingleFileStats>) -> () {
    let mut seen_stems = std::collections::HashSet::<String>::new();
    for (stem, xmap_stats) in xmap.iter() {
        if !seen_stems.insert(String::from(stem)) {
            continue;
        }

        let num_bytes_data: &mut u32;
        let num_bytes_code: &mut u32;
        if xmap_stats.is_asm {
            num_bytes_code = &mut stats.asm_code_bytes;
            num_bytes_data = &mut stats.asm_data_bytes;
        } else {
            num_bytes_code = &mut stats.c_code_bytes;
            num_bytes_data = &mut stats.c_data_bytes;
        }

        *num_bytes_code += xmap_stats.code_bytes;
        *num_bytes_data += xmap_stats.data_bytes;
    }
}

fn analyze_elf_relocations(stats: &mut Stats, build_path: &String, name: &str) -> () {
    // Create a stream wrapping the elf file
    let elf_name = std::format!("{}/{}.elf", build_path, name);
    let elf_path = PathBuf::from(elf_name);
    let elf_data = std::fs::read(elf_path).expect("unable to read ELF file");
    let elf_bytes =
        ElfBytes::<LittleEndian>::minimal_parse(elf_data.as_slice()).expect("could not parse ELF");
    assert_eq!(
        elf_bytes.ehdr.class,
        elf::file::Class::ELF32,
        "not a 32-bit ELF"
    );
    assert_eq!(
        elf_bytes.ehdr.e_machine,
        elf::abi::EM_ARM,
        "not an ARM32 ELF"
    );

    // Pointers analysis
    // Get the program headers for the load offsets and sizes
    // Get the section headers and strtab to find the code/data in the final ROM
    let mut program_headers = elf_bytes
        .segments()
        .expect("could not get phdr")
        .into_iter()
        .filter(|phdr| phdr.p_memsz != 0 && (phdr.p_memsz & 3) == 0 && phdr.p_vaddr != 0)
        .map(|phdr| {
            let vaddr = u32::try_from(phdr.p_vaddr).unwrap();
            let e_vaddr = vaddr + u32::try_from(phdr.p_memsz).unwrap();
            [vaddr, e_vaddr]
        });
    let (section_headers_o, strtab_o) = elf_bytes
        .section_headers_with_strtab()
        .expect("could not get shdr and/or strtab");
    let section_headers = section_headers_o.expect("could not get shdr");
    let strtab = strtab_o.expect("could not get strtab");

    // Only process each SBIN once per section
    let mut seen_names = std::collections::HashSet::<&str>::new();
    for section in section_headers {
        let section_name = strtab
            .get(usize::try_from(section.sh_name).unwrap())
            .expect("could not get section name");
        if !seen_names.insert(section_name) {
            continue;
        }
        let sbin_name = format!("{}/{}.sbin", build_path, section_name);
        let sbin_path = std::path::PathBuf::from(&sbin_name);
        if sbin_path.exists() {
            let raw = std::fs::read(sbin_path).expect(&format!("unable to read {}", sbin_name));
            let nbytes = raw.len();
            assert_ne!(nbytes, 0, "file size is 0");
            if (nbytes & 3) != 0 {
                // eprintln!(
                //     "Skipping section {} because its size is not word-aligned",
                //     section_name
                // );
                continue;
            }

            // Loop over 32-bit words
            let (chunks, _remainder) = raw.as_chunks::<4>();
            stats.hardcoded_pointers = chunks
                .iter()
                .filter(|word_raw: &&[u8; 4]| {
                    let my_word = u32::from_le_bytes(**word_raw);
                    my_word >= 0x01000000
                        && program_headers
                            .find(|region| region[0] <= my_word && my_word < region[1])
                            .is_some()
                })
                .count()
                .try_into()
                .unwrap();
        } else if section.sh_type == elf::abi::SHT_REL {
            let rels = elf_bytes
                .section_data_as_rels(&section)
                .expect("reltab parse failure");
            let nrels: u32 = rels.count().try_into().unwrap();
            stats.resolved_pointers += nrels;
        } else if section.sh_type == elf::abi::SHT_RELA {
            let rels = elf_bytes
                .section_data_as_relas(&section)
                .expect("reltab parse failure");
            let nrels: u32 = rels.count().try_into().unwrap();
            stats.resolved_pointers += nrels;
        }
    }
}

pub fn analyze_build(
    basedir: &String,
    buildname: Option<&String>,
    name: &str,
    source_map: &HashMap<String, bool>,
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
        Some(ref my_name) => {
            build_subdir = std::format!("build/{}", my_name);
        }
        None => {
            build_subdir = "build".to_string();
        }
    }
    let build_path = std::format!("{}/{}", basedir, build_subdir);

    // Load the xMAP file
    let xmap_name = std::format!("{}/{}.elf.xMAP", build_path, name);
    let xmap = xmap_file::parse_xmap(&xmap_name, source_map);

    analyze_source_files(&mut stats, &xmap);
    analyze_elf_relocations(&mut stats, &build_path, name);

    stats
}
