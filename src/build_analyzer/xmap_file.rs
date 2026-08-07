use anyhow::Result;
use itertools::Itertools;
use log::debug;
use regex::Regex;
use std::{collections::HashMap, path::PathBuf};

type ParseXmapRegexType = Option<((String, bool), (String, XmapSymbol))>;
type ParseXmapReturnType = HashMap<(String, bool), HashMap<String, XmapSymbol>>;

/// Returns Some(true) if the section is code, Some(false) if data, None if neither
fn is_section_code(name: &str) -> Option<bool> {
    match name {
        ".text" | ".init" | ".itcm" | ".sinit" | ".wram" => Some(true),
        ".data" | ".rodata" | ".sdata" | ".dtcm" | ".exception" | ".version" => Some(false),
        _ => None,
    }
}

pub struct XmapSymbol {
    /// Name of the ELF section this symbol was placed in
    pub section_name: String,
    /// true if the symbol is from a code section, false otherwise
    pub is_code: bool,
    /// Size of the symbol, in bytes
    pub size: usize,
    #[cfg(debug_assertions)]
    /// Runtime address of the symbol
    pub addr: u64,
}

/// Parse an mwldarm .xMAP file.
/// Returns a `HashMap` from (`stem`, `is_cfile`) to Vec<XmapSymbol>
/// `stem`: The basename of the source file without its final extension
/// `is_cfile`: true if the source file is decompiled C, false otherwise (extracted ASM)
pub fn parse_xmap(
    path: &PathBuf,
    source_map: &HashMap<String, (String, bool)>,
) -> Result<ParseXmapReturnType> {
    debug!("path = {path:#?}");
    let pat = Regex::new(
        r"^\s*(?<addr>[0-9A-F]{8})\s+(?<size>[0-9A-F]{8})\s+(?<section>\S+)\s+(?<name>\S+)\t\((?<ofile>\S+)\.o\)$",
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
            let Some((ofile_name, is_cfile)) = source_map.get(&name) else {
                return Ok(None);
            };

            // Get the section type
            let Some(is_text) = is_section_code(&caps["section"]) else {
                return Ok(None);
            };

            // Get file data
            let key = (ofile_name.to_owned(), is_cfile.to_owned());
            Ok(Some((
                key,
                (
                    caps["name"].to_string(),
                    XmapSymbol {
                        section_name: caps["section"].to_string(),
                        is_code: is_text,
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
