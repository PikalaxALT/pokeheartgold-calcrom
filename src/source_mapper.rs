use anyhow::{Context, Result, ensure};
use cached::cached;
use log::debug;
use regex::Regex;
use std::{collections::HashMap, path::PathBuf};

pub type SourceMap = HashMap<String, (PathBuf, bool)>;

fn file_with_extension_exists(mut path: PathBuf, extension: &str) -> bool {
    path.add_extension(extension) && path.exists()
}

#[allow(clippy::needless_pass_by_value)]
#[cached()]
pub fn get_source_files(dir: PathBuf, linkname: String) -> Result<SourceMap> {
    let re = Regex::new(r"^\s*Object\s+(\S+)\.o")?;
    let mut name_map = SourceMap::new();
    let lsf_file = dir.join(format!("{linkname}.lsf"));
    std::fs::read_to_string(&lsf_file)?
        .lines()
        .filter_map(|line| re.captures(line))
        .try_for_each(|m| {
            let source_o_path = dir.clone().join(&m[1]);
            let stem = source_o_path
                .file_stem()
                .context("no filename stem")?
                .to_str()
                .context("conversion failed")?
                .to_string();
            let is_c_file = file_with_extension_exists(source_o_path.clone(), "c");
            let is_asm_file = file_with_extension_exists(source_o_path.clone(), "s");
            ensure!(
                !is_c_file || !is_asm_file,
                format!("{stem}.o has both C and ASM files in the same directory")
            );

            ensure!(
                name_map.get(&stem).is_none_or(|(_, s)| *s == is_c_file),
                format!("{stem}.o has conflicting source file types")
            );

            let source_rel = source_o_path.strip_prefix(dir.clone())?;
            debug!("source_rel = {}", source_rel.display());
            name_map.insert(stem, (source_rel.to_path_buf(), is_c_file));

            Ok(())
        })?;

    Ok(name_map)
}
