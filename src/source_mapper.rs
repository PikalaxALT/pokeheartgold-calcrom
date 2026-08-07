use anyhow::{Context, Result, ensure};
use cached::cached;
use regex::Regex;
use std::{collections::HashMap, path::PathBuf};

#[allow(clippy::needless_pass_by_value)]
#[cached()]
pub fn get_source_files(dir: PathBuf, linkname: String) -> Result<HashMap<String, (String, bool)>> {
    let re = Regex::new(r"^\s*Object\s+(\S+)\.o")?;
    let mut name_map = HashMap::<String, (String, bool)>::new();
    let dir_as_str = format!("{}/", dir.to_str().context("conversion failed")?);
    let lsf_file = dir.join(format!("{linkname}.lsf"));
    std::fs::read_to_string(&lsf_file)?
        .lines()
        .filter_map(|line| re.captures(line))
        .try_for_each(|m| {
            let cap = dir.join(&m[1]);
            let source_o_path = cap.to_str().context("conversion failed")?;
            let stem = source_o_path
                .rsplit_once('/')
                .context("no slash in path")?
                .1
                .to_string();
            let is_c_file = std::path::PathBuf::from(format!("{source_o_path}.c")).exists();
            let is_asm_file = std::path::PathBuf::from(format!("{source_o_path}.s")).exists();
            ensure!(
                !is_c_file || !is_asm_file,
                format!("{stem}.o has both C and ASM files in the same directory")
            );

            ensure!(
                name_map.get(&stem).is_none_or(|(_, s)| *s == is_c_file),
                format!("{stem}.o has conflicting source file types")
            );

            let source_rel = source_o_path.replace(&dir_as_str, "");
            name_map.insert(stem, (source_rel, is_c_file));

            Ok(())
        })?;

    Ok(name_map)
}
