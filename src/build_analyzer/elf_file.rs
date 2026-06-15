use elf::ElfBytes;
use elf::endian::LittleEndian;
use elf::relocation::Rel;
use elf::relocation::Rela;
use elf::section::SectionHeader;
use elf::segment::ProgramHeader;
use elf::symbol::Symbol;
use std::error::Error;
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
    pub filename: String,
    pub sections: Vec<SectionHeaderWithData>,
    pub segments: Vec<ProgramHeader>,
    pub symbols: Vec<NamedSymbol>,
    pub rels: Vec<Rel>,
    pub relas: Vec<Rela>,
}

impl ElfFile {
    pub fn from_path(elf_name: &String) -> Result<ElfFile, Box<dyn Error>> {
        let elf_path = PathBuf::from(elf_name);
        let elf_data = std::fs::read(elf_path)?;
        let elf_bytes = ElfBytes::<LittleEndian>::minimal_parse(elf_data.as_slice())?;
        if elf_bytes.ehdr.class != elf::file::Class::ELF32 {
            return Err("not a 32-bit ELF".into());
        }

        if elf_bytes.ehdr.e_machine != elf::abi::EM_ARM {
            return Err("not an ARM32 ELF".into());
        }

        // read tables
        // Get the program headers for the load offsets and sizes
        let program_headers = elf_bytes
            .segments()
            .into_iter()
            .flat_map(|t| t.into_iter())
            .collect::<Vec<_>>();

        // Get the section headers and strtab to find the code/data in the final ROM
        let section_headers = elf_bytes
            .section_headers()
            .into_iter()
            .flat_map(|s| s.into_iter())
            .map(|shdr| -> Result<SectionHeaderWithData, elf::ParseError> {
                Ok(SectionHeaderWithData {
                    shdr: shdr,
                    data: elf_bytes.section_data(&shdr)?.0.to_vec(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let rels = section_headers
            .iter()
            .filter(|shdr| shdr.shdr.sh_type == elf::abi::SHT_REL)
            .map(
                |shdr| -> Result<elf::relocation::RelIterator<LittleEndian>, elf::ParseError> {
                    Ok(elf_bytes.section_data_as_rels(&shdr.shdr)?)
                },
            )
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        let relas = section_headers
            .iter()
            .filter(|shdr| shdr.shdr.sh_type == elf::abi::SHT_RELA)
            .map(
                |shdr| -> Result<elf::relocation::RelaIterator<LittleEndian>, elf::ParseError> {
                    Ok(elf_bytes.section_data_as_relas(&shdr.shdr)?)
                },
            )
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        let Some((symtab, strtab)) = elf_bytes.symbol_table()? else {
            return Err("no symtab or strtab".into());
        };
        let syms = symtab
            .into_iter()
            .map(|sym| -> Result<NamedSymbol, Box<dyn Error>> {
                Ok(NamedSymbol {
                    name: strtab.get(sym.st_name.try_into()?)?.to_string(),
                    sym: sym,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ElfFile {
            filename: elf_name.to_owned(),
            sections: section_headers,
            segments: program_headers,
            symbols: syms,
            rels: rels,
            relas: relas,
        })
    }

    pub fn symbol_data(&self, sym: &NamedSymbol) -> Result<Vec<u8>, Box<dyn Error>> {
        let Some(shdr) = self.sections.get(usize::from(sym.sym.st_shndx)) else {
            return Err("unrecognized section".into());
        };
        let start = sym.sym.st_value - shdr.shdr.sh_addr;
        let end = start + sym.sym.st_size;
        let result = shdr.data[usize::try_from(start)?..usize::try_from(end)?].to_vec();
        Ok(result)
    }
}
