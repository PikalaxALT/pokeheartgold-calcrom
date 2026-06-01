use elf::ElfStream;
use elf::endian::LittleEndian;
use elf::ParseError;

pub fn load_elf(elf_name: String) -> Result<ElfStream<LittleEndian, std::fs::File>, ParseError> {
    let elf_path = std::fs::File::open(elf_name.clone()).unwrap();
    ElfStream::<LittleEndian, std::fs::File>::open_stream(elf_path)
}
