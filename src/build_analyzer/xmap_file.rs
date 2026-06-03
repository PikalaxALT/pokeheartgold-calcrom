use regex::Regex;
use std::collections::HashMap;

pub struct SingleFileStats {
    pub code_bytes: u32,
    pub data_bytes: u32,
    pub is_asm: bool,
}

pub fn parse_xmap(path: &String) -> HashMap<String, SingleFileStats> {
    let mut result = HashMap::<String, SingleFileStats>::new();
    let pat = Regex::new(r"^*(?<addr>[0-9A-F]{8})\s+(?<size>[0-9A-F]{8})\s+(?<section>\S+)\s+(?<name>\S+)\t\((?<ofile>\S+\.o)\)$").unwrap();
    let text_sections = [".text", ".init", ".itcm"];
    let data_sections = [".data", ".rodata", ".sdata", ".dtcm"];

    std::fs::read_to_string(path)
        .expect(&std::format!("no such file or directory: {}", path))
        .lines()
        .map(String::from)
        .for_each(|line| {
            let Some(caps) = pat.captures(&line) else {
                return;
            };
            let size = u32::from_str_radix(&caps["size"], 16).unwrap();
            if size == 0 {
                return;
            }
            let name = &caps["ofile"];
            if result.get(name).is_none() {
                result.insert(
                    name.to_string(),
                    SingleFileStats {
                        code_bytes: 0,
                        data_bytes: 0,
                        is_asm: false,
                    },
                );
            }
            let cur_result = result.get_mut(name).unwrap();
            let is_text = text_sections.contains(&&caps["section"]);
            let is_data = data_sections.contains(&&caps["section"]);
            if !cur_result.is_asm && caps["name"] == caps["section"] {
                cur_result.code_bytes = 0;
                cur_result.data_bytes = 0;
                cur_result.is_asm = true;
            }
            let ref_cur_bytes: &mut u32;
            if is_text {
                ref_cur_bytes = &mut cur_result.code_bytes;
            } else if is_data {
                ref_cur_bytes = &mut cur_result.data_bytes;
            } else {
                return;
            }
            if cur_result.is_asm {
                if caps["name"] == caps["section"] {
                    *ref_cur_bytes = size;
                }
            } else {
                *ref_cur_bytes += size;
            }
        });

    result
}
