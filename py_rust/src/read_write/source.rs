use std::{ffi::OsStr, path::PathBuf};

use super::filetype::VALID_FILE_EXTS_AND_OPTS;
use crate::prelude::*;

/// The provided source for file commands, either a filepath to the contents, or the contents as a string.
pub enum Source {
    Value(Option<String>),
    File(PathBuf),
}

impl Source {
    /// Create the source from the string, which is either a filepath or the contents directly.
    pub fn new(source: &str) -> Result<Self, Zerr> {
        // Try and workout whether a path or not:
        if let Some(pb) = as_file(source) {
            if !pb.exists() {
                return Err(zerr!(
                    Zerr::FileNotFound,
                    "File not found: '{}'.",
                    pb.display()
                ));
            }

            Ok(Self::File(pb))
        } else {
            Ok(Self::Value(Some(source.to_string())))
        }
    }

    /// Returns the file extension if it's a file (and if it has one)
    pub fn file_ext(&self) -> Option<&OsStr> {
        match self {
            Self::Value(_) => None,
            Self::File(p) => p.extension(),
        }
    }

    /// Returns the file path if it's a file.
    pub fn filepath(&self) -> Option<&PathBuf> {
        match self {
            Self::Value(_) => None,
            Self::File(p) => Some(p),
        }
    }

    /// Consume the contents of the source.
    pub fn read(&mut self) -> Result<String, Zerr> {
        match self {
            Self::Value(s) => Ok(s
                .take()
                .ok_or_else(|| zerr!(Zerr::InternalError, "Source should only be read once!"))?),
            Self::File(p) => {
                // Read from file:
                std::fs::read_to_string(p).change_context(Zerr::InternalError)
            }
        }
    }

    /// "Write" the new value,
    /// if was passed in via console should be written to stdout,
    /// otherwise replace the contents of the file:
    pub fn write(&self, contents: &str) -> Result<(), Zerr> {
        match self {
            Self::Value(_) => {
                println!("{}", contents);
            }
            Self::File(p) => {
                // Write to file:
                std::fs::write(p, contents).change_context(Zerr::InternalError)?;
            }
        }
        Ok(())
    }
}

fn as_file(s: &str) -> Option<PathBuf> {
    let path = PathBuf::from(s);

    // If it exists, it's definitely a path!
    if path.exists() {
        return Some(path);
    }

    // If its multiline, definitely not:
    if s.chars().filter(|&c| c == '\n').count() > 1 {
        return None;
    }

    // Look for common path indicators:
    if (path.is_absolute()) || path.starts_with("~") || path.starts_with(".") {
        return Some(path);
    }

    // If path ends with a supported extension, definitely:
    if let Some(ext) = path.extension() {
        if VALID_FILE_EXTS_AND_OPTS.contains(&ext.to_str().unwrap()) {
            return Some(path);
        }
    }

    None
}
