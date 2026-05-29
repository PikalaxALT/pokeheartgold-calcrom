use std::option::Option;
mod elf_file;

pub struct BuildAnalyzer {
    pub basedir: String,
    pub name: Option<String>,
}

impl BuildAnalyzer {
    pub fn process(self) {
        let elf_name: String;
        match self.name {
            Some(ref my_name) => {
                elf_name = std::format!("{}/build/{}/main.elf", self.basedir, my_name);
            }
            None => {
                elf_name = std::format!("{}/build/ichneumon_sub.elf", self.basedir);
            }
        }
        let xmap_name = std::format!("{}.xMAP", elf_name);
        println!("using elf {} and xmap {}", elf_name, xmap_name);

        let mut elf_file = elf_file::load_elf(elf_name);
        elf_file.parse();
    }
}
