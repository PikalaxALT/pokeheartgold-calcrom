use anyhow::{Context, Result, ensure};
use cached::cached;
use regex::Regex;
use std::collections::HashMap;

#[cached()]
pub fn get_source_files(dir: String, linkname: String) -> Result<HashMap<String, (String, bool)>> {
    let re = Regex::new(r"^\s*Object\s+(\S+)\.o")?;
    let mut name_map = HashMap::<String, (String, bool)>::new();
    let lsf_file = format!("{}/{}.lsf", dir, linkname);
    std::fs::read_to_string(&lsf_file)?
        .lines()
        .filter_map(|line| re.captures(line))
        .try_for_each(|m| {
            let source_o_path = format!("{}/{}", dir, &m[1]);
            let stem = source_o_path
                .rsplit_once("/")
                .context("no slash in path")?
                .1
                .to_string();
            let is_cfile = std::path::PathBuf::from(format!("{}.c", source_o_path)).exists();
            let is_sfile = std::path::PathBuf::from(format!("{}.s", source_o_path)).exists();
            ensure!(
                !is_cfile || !is_sfile,
                format!("{}.o has both C and ASM files in the same directory", stem)
            );

            ensure!(
                name_map.get(&stem).is_none_or(|(_, s)| *s == is_cfile),
                format!("{}.o has conflicting source file types", stem)
            );

            let source_rel = source_o_path.replace(&dir, "");
            name_map.insert(stem, (source_rel, is_cfile));

            Ok(())
        })?;

    Ok(name_map)
}
