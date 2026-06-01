use regex::Regex;
use std::collections::HashMap;

pub struct SingleFileStats {
    pub code_bytes: u32,
    pub data_bytes: u32,
}

pub fn parse_xmap(path: &String) -> HashMap<String, SingleFileStats> {
    let mut result = HashMap::<String, SingleFileStats>::new();
    let pat = Regex::new(r"^*(?<addr>[0-9A-F]{8})\s+(?<size>[0-9A-F]{8})\s+(?<section>\S+)\s+(?<name>\S+)\t\((?<ofile>\w+\.o)\)$").unwrap();

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
                result.insert(name.to_string(), SingleFileStats{
                    code_bytes: 0,
                    data_bytes: 0,
                });
            }
            let cur_result = result.get_mut(name).unwrap();
            if &caps["section"] == ".text" || &caps["section"] == ".itcm" {
                cur_result.code_bytes += size;
            } else {
                cur_result.data_bytes += size;
            }
        });

    result
}
