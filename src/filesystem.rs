use std::fs::{self, Permissions};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, de::Error};


pub fn expand_path(path: &str) -> PathBuf {
    let path = shellexpand::tilde(path);
    PathBuf::from(&*path)
}


pub fn create_valid_parent(path: &Path) {
    let parent = path.parent().unwrap();
    assert!(!parent.is_file());
    if !parent.exists() {
        fs::create_dir_all(parent).unwrap();
    }
}


pub fn parse_permissions<'de, D>(deserializer: D)
    -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let mode: Option<u32> = Option::deserialize(deserializer)?;
    if let Some(s) = mode {
        let mode_str = s.to_string();
        match u32::from_str_radix(&mode_str, 8) {
            Ok(i) => Ok(Some(i)),
            _ => Err(D::Error::custom("invalid permissions")),
        }
    } else {
        Ok(None)
    }
}


pub fn set_permissions<P>(path: P, mode: Option<u32>) -> Result<(), io::Error>
where
    P: AsRef<Path>
{
    match mode {
        None => Ok(()),
        Some(mode) => fs::set_permissions(path, Permissions::from_mode(mode)),
    }
}
