use elf;
use std::option::Option;

mod elf_file;
mod xmap_file;

pub struct Stats {
    pub c_code_bytes: u32,
    pub c_data_bytes: u32,
    pub asm_code_bytes: u32,
    pub asm_data_bytes: u32,
    pub resolved_pointers: u32,
    pub hardcoded_pointers: u32,
}

pub struct BuildAnalyzer {
    pub basedir: String,
    pub buildname: Option<String>,
    pub name: String,
}

impl BuildAnalyzer {
    fn analyze_source_files(&self, stats: &mut Stats, build_path: &String) -> () {
        // Load the xMAP file
        let xmap_name = std::format!("{}/{}.elf.xMAP", build_path, self.name);
        let xmap = xmap_file::parse_xmap(&xmap_name);

        // ELF files produced by mwldarm don't capture the full path to the origin file
        let valid_extensions = ["c", "s"];
        for objfile in walkdir::WalkDir::new(build_path.clone())
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let objpath = objfile.path();
            if objpath.is_file() && objpath.extension().is_some_and(|x| x == "o") {
                // Get source file path
                let relative_path = objpath.strip_prefix(build_path.clone()).unwrap();
                let source_path = std::path::Path::new(&self.basedir).join(relative_path);
                let Some(source_file) = valid_extensions
                    .iter()
                    .map(|e| source_path.with_extension(e))
                    .find(|p| p.exists())
                else {
                    continue;
                };
                let stem = objpath.file_name().unwrap().to_str().unwrap();
                let Some(xmap_stats) = xmap.get(stem) else {
                    // eprintln!("WARN: no build info for {}", stem);
                    continue;
                };

                let num_bytes_data: &mut u32;
                let num_bytes_code: &mut u32;
                if source_file.extension().unwrap() == "c" {
                    num_bytes_code = &mut stats.c_code_bytes;
                    num_bytes_data = &mut stats.c_data_bytes;
                } else {
                    num_bytes_code = &mut stats.asm_code_bytes;
                    num_bytes_data = &mut stats.asm_data_bytes;
                }

                // find .text section corresponding to
                *num_bytes_code += xmap_stats.code_bytes;
                *num_bytes_data += xmap_stats.data_bytes;
            }
        }
    }

    fn analyze_elf_relocations(&self, stats: &mut Stats, build_path: &String) -> () {
        // Create a stream wrapping the elf file
        let elf_name = std::format!("{}/{}.elf", build_path, self.name);
        let elf_stream = elf_file::load_elf(elf_name).unwrap();
        assert_eq!(elf_stream.ehdr.class, elf::file::Class::ELF32);
        assert_eq!(elf_stream.ehdr.e_machine, elf::abi::EM_ARM);

        // Pointers analysis
        let section_headers = elf_stream.section_headers();
    }

    pub fn process(&self) -> Stats {
        let mut stats = Stats {
            c_code_bytes: 0,
            c_data_bytes: 0,
            asm_code_bytes: 0,
            asm_data_bytes: 0,
            resolved_pointers: 0,
            hardcoded_pointers: 0,
        };

        let build_subdir: String;
        match self.buildname {
            Some(ref my_name) => {
                build_subdir = std::format!("build/{}", my_name);
            }
            None => {
                build_subdir = "build".to_string();
            }
        }
        let build_path = std::format!("{}/{}", self.basedir, build_subdir);

        self.analyze_source_files(&mut stats, &build_path);
        self.analyze_elf_relocations(&mut stats, &build_path);

        stats
    }
}
