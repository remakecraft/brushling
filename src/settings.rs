//! Per-user settings store: a flat `key=value` text file in
//! `<config_dir>/brushling/settings.ini`. Features read and write their own
//! keys through [`Settings::get`] / [`Settings::set`]; the file is written by
//! [`Settings::save`] (on exit and whenever a persisted value changes).
//! A missing, unreadable or garbled file silently yields defaults.

use crate::doc::MAX_DIM;
use crate::types::{HResult, HarnessError};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const DIR_NAME: &str = "brushling";
const FILE_NAME: &str = "settings.ini";
const KEY_NEW_SIZE: &str = "new_size";

#[derive(Debug, Clone)]
pub struct Settings {
    file: PathBuf,
    values: BTreeMap<String, String>,
}

impl Settings {
    /// Load the settings stored under `config_dir`; never fails.
    pub fn load(config_dir: &Path) -> Settings {
        let file = config_dir.join(DIR_NAME).join(FILE_NAME);
        let values = crate::sysfs::read(&file)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .map(|t| parse(&t))
            .unwrap_or_default();
        Settings { file, values }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Store a value in memory; call [`Settings::save`] to persist it.
    /// Keys may not contain `=` or line breaks, values no line breaks.
    pub fn set(&mut self, key: &str, value: &str) -> HResult<()> {
        if key.is_empty() || key.contains(['=', '\n', '\r']) || value.contains(['\n', '\r']) {
            return Err(HarnessError::Invalid(format!("unstorable setting {key:?}")));
        }
        self.values.insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    pub fn save(&self) -> HResult<()> {
        if let Some(dir) = self.file.parent() {
            crate::sysfs::create_dir_all(dir)?;
        }
        let mut text = String::new();
        for (k, v) in &self.values {
            text.push_str(k);
            text.push('=');
            text.push_str(v);
            text.push('\n');
        }
        crate::sysfs::write(&self.file, text.as_bytes())?;
        Ok(())
    }

    /// The size used by File > New, if one was confirmed in Attributes.
    pub fn new_size(&self) -> Option<(u32, u32)> {
        let (w, h) = self.get(KEY_NEW_SIZE)?.split_once('x')?;
        let (w, h) = (w.parse::<u32>().ok()?, h.parse::<u32>().ok()?);
        ((1..=MAX_DIM).contains(&w) && (1..=MAX_DIM).contains(&h)).then_some((w, h))
    }

    pub fn set_new_size(&mut self, w: u32, h: u32) -> HResult<()> {
        crate::doc::check_size(w, h)?;
        self.set(KEY_NEW_SIZE, &format!("{w}x{h}"))
    }
}

fn parse(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|l| l.split_once('='))
        .filter(|(k, _)| !k.is_empty() && !k.contains(char::is_control))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_garbage() {
        let dir = std::env::temp_dir().join(format!("brushling-settings-{}", std::process::id()));
        let mut s = Settings::load(&dir);
        assert_eq!(s.new_size(), None);
        s.set_new_size(100, 50).unwrap();
        s.save().unwrap();
        assert_eq!(Settings::load(&dir).new_size(), Some((100, 50)));
        std::fs::write(dir.join("brushling").join("settings.ini"), b"\x00\xffgarbage{{{[[[").unwrap();
        assert_eq!(Settings::load(&dir).new_size(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
