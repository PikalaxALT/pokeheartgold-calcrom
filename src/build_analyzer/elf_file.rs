use elf::ElfBytes;
use elf::endian::LittleEndian;
use elf::relocation::Rel;
use elf::relocation::Rela;
use elf::section::SectionHeader;
use elf::segment::ProgramHeader;
use elf::symbol::Symbol;
use std::path::PathBuf;
use std::vec::Vec;

pub struct SectionHeaderWithData {
    pub shdr: SectionHeader,
    pub data: Vec<u8>,
}

pub struct NamedSymbol {
    pub sym: Symbol,
    pub name: String,
}

pub struct ElfFile {
    pub sections: Vec<SectionHeaderWithData>,
    pub segments: Vec<ProgramHeader>,
    pub symbols: Vec<NamedSymbol>,
    pub rels: Vec<Rel>,
    pub relas: Vec<Rela>,
}

impl ElfFile {
    pub fn from_path(elf_name: &String) -> ElfFile {
        let elf_path = PathBuf::from(elf_name);
        let elf_data =
            std::fs::read(elf_path).expect(&format!("unable to read ELF file: {}", elf_name));
        let elf_bytes = ElfBytes::<LittleEndian>::minimal_parse(elf_data.as_slice())
            .expect("could not parse ELF");
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

        // read tables
        // Get the program headers for the load offsets and sizes
        let program_headers = match elf_bytes.segments() {
            Some(phdr_tab) => phdr_tab.into_iter().collect::<Vec<_>>(),
            None => Vec::<_>::new(),
        };

        // Get the section headers and strtab to find the code/data in the final ROM
        let section_headers = elf_bytes
            .section_headers()
            .expect("could not get shdr")
            .into_iter()
            .map(|shdr| SectionHeaderWithData {
                shdr: shdr,
                data: elf_bytes
                    .section_data(&shdr)
                    .expect("section data grab failed")
                    .0
                    .to_vec(),
            })
            .collect::<Vec<_>>();

        let rels = section_headers
            .iter()
            .filter(|shdr| shdr.shdr.sh_type == elf::abi::SHT_REL)
            .map(|shdr| {
                elf_bytes
                    .section_data_as_rels(&shdr.shdr)
                    .expect("reltab parsing failed")
            })
            .flatten()
            .collect::<Vec<_>>();

        let relas = section_headers
            .iter()
            .filter(|shdr| shdr.shdr.sh_type == elf::abi::SHT_RELA)
            .map(|shdr| {
                elf_bytes
                    .section_data_as_relas(&shdr.shdr)
                    .expect("relatab parsing failed")
            })
            .flatten()
            .collect::<Vec<_>>();

        let (symtab, strtab) = elf_bytes
            .symbol_table()
            .expect("symtab or strtab parsing failed")
            .expect("no symtab or strtab");
        let syms = symtab
            .into_iter()
            .map(|sym| NamedSymbol {
                name: strtab
                    .get(sym.st_name.try_into().unwrap())
                    .expect("strtab lookup failed")
                    .to_string(),
                sym: sym,
            })
            .collect::<Vec<_>>();

        ElfFile {
            sections: section_headers,
            segments: program_headers,
            symbols: syms,
            rels: rels,
            relas: relas,
        }
    }

    pub fn symbol_data(&self, sym: &NamedSymbol) -> Result<Vec<u8>, String> {
        let Some(shdr) = self.sections.get(usize::from(sym.sym.st_shndx)) else {
            return Err("unrecognized section".to_string());
        };
        let start = sym.sym.st_value - shdr.shdr.sh_addr;
        let end = start + sym.sym.st_size;
        let result =
            shdr.data[usize::try_from(start).unwrap()..usize::try_from(end).unwrap()].to_vec();
        Ok(result)
    }
}
