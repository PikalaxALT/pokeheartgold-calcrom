use std::collections::HashMap;

use regex::Regex;

#[derive(Debug)]
pub enum SourceMapperError {
    /// same filename has two different extensions
    CollisionError(String),
}

impl std::error::Error for SourceMapperError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            SourceMapperError::CollisionError(_) => None,
        }
    }
}

impl core::fmt::Display for SourceMapperError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            SourceMapperError::CollisionError(ref name) => {
                write!(f, "Detected multiple source files with name {name}")
            }
        }
    }
}

pub fn get_source_files(
    dir: &String,
    linkname: &'static str,
) -> Result<HashMap<String, (String, bool)>, SourceMapperError> {
    let re = Regex::new(r"^\s*Object\s+(\S+)\.o").unwrap();
    let mut name_map = HashMap::<String, (String, bool)>::new();
    let lsf_file = format!("{}/{}.lsf", dir, linkname);
    std::fs::read_to_string(&lsf_file)
        .expect(&format!("no such file or directory: {}", lsf_file))
        .lines()
        .map(String::from)
        .try_for_each(|line| {
            let Some(m) = re.captures(&line) else {
                return Ok(());
            };
            let source_o_path = format!("{}/{}", dir, &m[1]);
            let stem = String::from(source_o_path.rsplit_once("/").unwrap().1);
            let is_cfile = std::path::PathBuf::from(format!("{}.c", source_o_path)).exists();
            let is_sfile = std::path::PathBuf::from(format!("{}.s", source_o_path)).exists();
            if is_cfile && is_sfile {
                return Err(SourceMapperError::CollisionError(stem));
            }
            if name_map.get(&stem).is_some_and(|(_, s)| *s != is_cfile) {
                return Err(SourceMapperError::CollisionError(stem));
            }
            let source_rel = source_o_path.replace(dir, "");
            name_map.insert(stem, (source_rel, is_cfile));
            Ok(())
        })?;

    Ok(name_map)
}
