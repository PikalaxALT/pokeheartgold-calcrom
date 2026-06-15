use std::{collections::HashMap, error::Error};

use cached::cached;
use regex::Regex;

#[cached()]
pub fn get_source_files(
    dir: String,
    linkname: String,
) -> Result<HashMap<String, (String, bool)>, Box<dyn Error>> {
    let re = Regex::new(r"^\s*Object\s+(\S+)\.o")?;
    let mut name_map = HashMap::<String, (String, bool)>::new();
    let lsf_file = format!("{}/{}.lsf", dir, linkname);
    std::fs::read_to_string(&lsf_file)?
        .lines()
        .map(String::from)
        .try_for_each(|line| {
            let Some(m) = re.captures(&line) else {
                return Ok(());
            };
            let source_o_path = format!("{}/{}", dir, &m[1]);
            let Some((_, stem)) = source_o_path.rsplit_once("/") else {
                return Err("no slash in path".into());
            };
            let is_cfile = std::path::PathBuf::from(format!("{}.c", source_o_path)).exists();
            let is_sfile = std::path::PathBuf::from(format!("{}.s", source_o_path)).exists();
            if is_cfile && is_sfile {
                return Err(format!(
                    ".o file with stem {} has both C and ASM files in the same directory",
                    stem.to_string()
                ));
            }
            if name_map
                .get(&stem.to_string())
                .is_some_and(|(_, s)| *s != is_cfile)
            {
                return Err(format!(
                    ".o file with stem {} has conflicting source file types",
                    stem.to_string()
                ));
            }
            let source_rel = source_o_path.replace(&dir, "");
            name_map.insert(stem.to_string(), (source_rel, is_cfile));
            Ok(())
        })?;

    Ok(name_map)
}
