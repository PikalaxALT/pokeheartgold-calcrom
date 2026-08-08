use anyhow::{Context, Result, ensure};
use elf::{
    ElfBytes,
    endian::LittleEndian,
    relocation::{Rel, Rela},
    section::SectionHeader,
    segment::ProgramHeader,
    symbol::Symbol,
};
use itertools::Itertools;
use log::debug;
use std::{path::PathBuf, vec::Vec};

/// A wrapper struct that associates an Elf section with its data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionHeaderWithData {
    pub shdr: SectionHeader,
    pub data: Vec<u8>,
}

/// A wrapper struct that associates an Elf symbol with its name
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedSymbol {
    pub sym: Symbol,
    pub name: String,
}

/// A wrapper struct representing a parsed ELF.
/// Contains sections, segments, symbols, and relocations with and without addend.
#[derive(Debug, Clone)]
pub struct ElfFile {
    pub sections: Vec<SectionHeaderWithData>,
    pub segments: Vec<ProgramHeader>,
    pub symbols: Vec<NamedSymbol>,
    pub rels: Vec<Rel>,
    pub relas: Vec<Rela>,
}

impl ElfFile {
    pub fn from_path(elf_path: &PathBuf) -> Result<Self> {
        // Load the ELF file into memory and assert that it is a 32-bit ARM elf.
        // Anything else is unsupported
        debug!("elf_path = {}", elf_path.display());
        ensure!(
            elf_path.exists(),
            format!("no such file or directory: {}", elf_path.display())
        );
        let elf_data = std::fs::read(elf_path)?;
        let elf_bytes = ElfBytes::<LittleEndian>::minimal_parse(elf_data.as_slice())?;
        ensure!(
            elf_bytes.ehdr.class == elf::file::Class::ELF32,
            "not a 32-bit ELF"
        );

        ensure!(
            elf_bytes.ehdr.e_machine == elf::abi::EM_ARM,
            "not an ARM32 ELF"
        );

        // read tables
        // Get the program headers for the load offsets and sizes
        let program_headers = elf_bytes.segments().into_iter().flatten().collect_vec();

        // Get the section headers and strtab to find the code/data in the final ROM
        let section_headers_raw = elf_bytes
            .section_headers()
            .into_iter()
            .flatten()
            .collect_vec();
        let section_headers_by_type = section_headers_raw
            .iter()
            .into_group_map_by(|shdr| shdr.sh_type);

        let section_headers = section_headers_raw
            .iter()
            .map(|shdr| -> Result<SectionHeaderWithData, elf::ParseError> {
                Ok(SectionHeaderWithData {
                    shdr: shdr.to_owned(),
                    data: elf_bytes.section_data(shdr)?.0.to_vec(),
                })
            })
            .process_results(|iter| iter.collect_vec())?;

        let rels = section_headers_by_type
            .get(&elf::abi::SHT_REL)
            .unwrap_or(&Vec::<&SectionHeader>::new())
            .iter()
            .map(|shdr| elf_bytes.section_data_as_rels(shdr))
            .process_results(|iter| iter.collect_vec())?
            .into_iter()
            .flatten()
            .collect_vec();

        #[allow(clippy::similar_names)]
        let addend_rels = section_headers_by_type
            .get(&elf::abi::SHT_RELA)
            .unwrap_or(&Vec::<&SectionHeader>::new())
            .iter()
            .map(|shdr| elf_bytes.section_data_as_relas(shdr))
            .process_results(|iter| iter.collect_vec())?
            .into_iter()
            .flatten()
            .collect_vec();

        let (symtab, strtab) = elf_bytes.symbol_table()?.context("no symtab or strtab")?;
        let syms = symtab
            .into_iter()
            .map(|sym| -> Result<NamedSymbol> {
                Ok(NamedSymbol {
                    name: strtab.get(sym.st_name.try_into()?)?.to_string(),
                    sym,
                })
            })
            .process_results(|iter| iter.collect_vec())?;

        Ok(Self {
            sections: section_headers,
            segments: program_headers,
            symbols: syms,
            rels,
            relas: addend_rels,
        })
    }

    pub fn symbol_data(&self, sym: &NamedSymbol) -> Result<Vec<u8>> {
        let shdr = self
            .sections
            .get(usize::from(sym.sym.st_shndx))
            .context("unrecognized section")?;
        let start = sym.sym.st_value.saturating_sub(shdr.shdr.sh_addr);
        let end = start.saturating_add(sym.sym.st_size);
        let result = shdr
            .data
            .get(usize::try_from(start)?..usize::try_from(end)?)
            .context("bad slice")?
            .to_vec();
        Ok(result)
    }
}
