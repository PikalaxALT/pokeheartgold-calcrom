use regex::Regex;
use std::boxed::Box;
use std::collections::HashMap;
use std::error::Error;

/// Returns Some(true) if the section is code, Some(false) if data, None if neither
fn is_section_code(name: &str) -> Option<bool> {
    match name {
        ".text" => Some(true),
        ".init" => Some(true),
        ".itcm" => Some(true),
        ".sinit" => Some(true),
        ".wram" => Some(true),
        ".data" => Some(false),
        ".rodata" => Some(false),
        ".sdata" => Some(false),
        ".dtcm" => Some(false),
        ".exception" => Some(false),
        ".version" => Some(false),
        _ => None,
    }
}

pub struct XmapSymbol {
    pub section_name: String,
    pub is_code: bool,
    pub size: usize,
}

/// Maps (stem, is_cfile) to Vec<XmapSymbol>
pub fn parse_xmap(
    path: &String,
    source_map: &HashMap<String, (String, bool)>,
) -> Result<HashMap<(String, bool), HashMap<String, XmapSymbol>>, Box<dyn Error>> {
    let mut result = HashMap::<(String, bool), HashMap<String, XmapSymbol>>::new();
    let pat = Regex::new(
        r"^\s*(?<addr>[0-9A-F]{8})\s+(?<size>[0-9A-F]{8})\s+(?<section>\S+)\s+(?<name>\S+)\t\((?<ofile>\S+)\.o\)$",
    )?;

    std::fs::read_to_string(path)?
        .lines()
        .filter_map(|line| pat.captures(&line))
        .map(|caps| -> Result<(), Box<dyn Error>> {
            // Get the object size
            let size = usize::from_str_radix(&caps["size"], 16)?;
            if size == 0 {
                return Ok(());
            }

            // Get the source file name (stem)
            let name = String::from(&caps["ofile"]);
            let Some((ofile_name, is_cfile)) = source_map.get(&name) else {
                return Ok(());
            };

            // Get the section type
            let Some(is_text) = is_section_code(&caps["section"]) else {
                return Ok(());
            };

            // Get file data
            let key = (ofile_name.to_owned(), is_cfile.to_owned());
            let cur_result = result
                .entry(key)
                .or_insert_with(HashMap::<String, XmapSymbol>::new);

            if let Some(_r) = cur_result.insert(
                caps["name"].to_string(),
                XmapSymbol {
                    section_name: caps["section"].to_string(),
                    is_code: is_text,
                    size: size,
                },
            ) {
                Err(format!("Collision on {}", ofile_name).into())
            } else {
                Ok(())
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(result)
}
