use elf::ElfBytes;
use elf::endian::LittleEndian;

pub struct ElfFile<'elffile> {
    name: String,
    raw: Vec<u8>,
    bytes: Option<ElfBytes<'elffile, LittleEndian>>,
}

impl<'elffile> ElfFile<'elffile> {
    pub fn parse(&'elffile mut self) {
        let elf_parse_error = std::format!("Unable to parse ELF file {}", self.name);
        let elf_slice = self.raw.as_slice();
        self.bytes.replace(
            ElfBytes::<'elffile, LittleEndian>::minimal_parse(elf_slice).expect(&elf_parse_error),
        );
    }
}

pub fn load_elf<'elffile>(elf_name: String) -> ElfFile<'elffile> {
    let elf_read_error = std::format!("Could not open {} for reading", elf_name);

    let elf_path = std::path::PathBuf::from(elf_name.clone());
    let elf_raw = std::fs::read(elf_path).expect(&elf_read_error);
    ElfFile {
        name: elf_name,
        raw: elf_raw.clone(),
        bytes: None,
    }
}
