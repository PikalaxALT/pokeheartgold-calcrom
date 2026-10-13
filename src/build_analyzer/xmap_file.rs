use anyhow::{Result, ensure};
use itertools::Itertools;
use log::{debug, warn};
use regex::Regex;
use std::{collections::HashMap, path::PathBuf};

pub enum SectionType {
    Code,
    Data,
    NoLoad,
    Unknown,
}

impl SectionType {
    /// Returns Some(true) if the section is code, Some(false) if data, None if neither
    fn from_name(name: &str) -> Self {
        match name {
            ".text" | ".init" | ".itcm" | ".sinit" | ".wram" => Self::Code,
            ".data" | ".rodata" | ".sdata" | ".dtcm" | ".exception" | ".version" => Self::Data,
            ".bss" | ".dtcm.bss" => Self::NoLoad,
            _ => {
                warn!("ignoring unmapped section: {name}");
                Self::Unknown
            }
        }
    }

    fn to_str(&self) -> &str {
        match self {
            Self::Code => "code",
            Self::Data => "data",
            Self::NoLoad => "noload",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for SectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

pub struct XmapSymbol {
    /// Name of the ELF section this symbol was placed in
    pub section_name: String,
    /// true if the symbol is from a code section, false otherwise
    pub section_type: SectionType,
    /// Size of the symbol, in bytes
    pub size: usize,
    #[cfg(debug_assertions)]
    /// Runtime address of the symbol
    pub addr: u64,
}

type ParseXmapRegexType = Option<((PathBuf, bool), (String, XmapSymbol))>;
type ParseXmapReturnType = HashMap<(PathBuf, bool), HashMap<String, XmapSymbol>>;

/// Parse an mwldarm .xMAP file.
/// Returns a `HashMap` from (`stem`, `is_cfile`) to Vec<XmapSymbol>
/// `stem`: The basename of the source file without its final extension
/// `is_cfile`: true if the source file is decompiled C, false otherwise (extracted ASM)
pub fn parse_xmap(path: &PathBuf) -> Result<ParseXmapReturnType> {
    debug!("path = {}", path.display());
    let pat = Regex::new(
        r"^\s*(?<addr>[0-9A-F]{8})\s+(?<size>[0-9A-F]{8})\s+(?<section>\S+)\s+(?<name>\S+)\t\((?<ofile>\S+\.o)\)$",
    )?;

    let result = std::fs::read_to_string(path)?
        .lines()
        .filter_map(|line| pat.captures(line))
        .map(|caps| -> Result<ParseXmapRegexType> {
            // Get the object size
            let size = usize::from_str_radix(&caps["size"], 16)?;
            if size == 0 {
                return Ok(None);
            }

            // Get the source file name (stem)
            let name = String::from(&caps["ofile"]);
            let is_cfile = name.ends_with(".c.o");
            let is_asmfile = name.ends_with(".s.o");
            ensure!(
                is_cfile ^ is_asmfile,
                format!("{name} should be either C or ASM, not both nor neither")
            );

            // Get the section type
            let section_type = SectionType::from_name(&caps["section"]);

            // Get file data
            let key = (PathBuf::from(name), is_cfile.clone());
            let section_name = caps["section"].to_string();
            let symbol_name = caps["name"].to_string();
            Ok(Some((
                key,
                (
                    symbol_name,
                    XmapSymbol {
                        section_name,
                        section_type,
                        size,
                        #[cfg(debug_assertions)]
                        addr: u64::from_str_radix(&caps["addr"], 16)?,
                    },
                ),
            )))
        })
        .process_results(|it| it.flatten().into_group_map())?
        .into_iter()
        .map(|(key, value)| (key, value.into_iter().collect::<HashMap<_, _>>()))
        .collect::<HashMap<_, _>>();

    Ok(result)
}
