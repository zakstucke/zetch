use strum::IntoEnumIterator;

use super::source::Source;
use crate::{args::FileSharedArgs, prelude::*};

/// Supported filetypes to process.
#[derive(Debug, strum::EnumIter, Copy, Clone)]
pub enum FileType {
    Json,
    Yaml,
    Toml,
}

pub static VALID_FILE_EXTS_AND_OPTS: &[&str] = &["json", "yaml", "yml", "toml"];

impl FileType {
    fn validate_file(&self, contents: &str) -> Result<(), Zerr> {
        match self {
            FileType::Json => {
                // Using fjson rather than serde to allow c-style comments in json files:
                fjson::ast::parse(contents)
                    .change_context(Zerr::FileSyntaxError)
                    .attach_printable_lazy(|| "Invalid Json.")?;
            }
            FileType::Yaml => {
                serde_yaml::from_str::<serde_json::Value>(contents)
                    .change_context(Zerr::FileSyntaxError)
                    .attach_printable_lazy(|| "Invalid Yaml.")?;
            }
            FileType::Toml => {
                toml::from_str::<toml::Value>(contents)
                    .change_context(Zerr::FileSyntaxError)
                    .attach_printable_lazy(|| "Invalid Toml.")?;
            }
        };

        Ok(())
    }
}

pub fn valid_ft_opts_str() -> String {
    let mut s = "".to_string();
    for (index, ft) in VALID_FILE_EXTS_AND_OPTS.iter().enumerate() {
        if index == VALID_FILE_EXTS_AND_OPTS.len() - 1 {
            s.push_str(&format!("or '--{}'", ft));
        } else {
            s.push_str(&format!("'--{}', ", ft));
        }
    }
    s
}

/// Infers the filetype and validates the file is readable as that type.
pub fn get_filetype(
    _args: &crate::args::Args,
    sargs: &FileSharedArgs,
    file_contents: &str,
    source: &Source,
) -> Result<FileType, Zerr> {
    let ft = if [sargs.json, sargs.yaml, sargs.toml]
        .iter()
        .filter(|x| **x)
        .count()
        > 1
    {
        return Err(zerr!(
            Zerr::FileCmdUsageError,
            "Only one of {} can be specified.",
            valid_ft_opts_str()
        ));
    } else if sargs.json {
        FileType::Json
    } else if sargs.yaml {
        FileType::Yaml
    } else if sargs.toml {
        FileType::Toml
    } else {
        // No specific given, need to infer:

        // First try to infer from the file extension:
        let maybe_ft = if let Some(ext) = source.file_ext() {
            let ext = ext.to_str().ok_or_else(|| {
                zerr_int!(
                    "Could not read file extension for file: '{}'.",
                    if let Some(fp) = source.filepath() {
                        fp.display().to_string()
                    } else {
                        "(contents passed in from command line)".to_string()
                    }
                )
            })?;

            match ext {
                "json" => Some(FileType::Json),
                "yaml" | "yml" => Some(FileType::Yaml),
                "toml" => Some(FileType::Toml),
                _ => None,
            }
        } else {
            None
        };

        if let Some(ft) = maybe_ft {
            ft
        } else {
            // Try and decode with each of the filetypes, storing any syntax errors:
            // If more than one matches or none match, raise error:
            let mut results: Vec<(FileType, Option<error_stack::Report<Zerr>>)> = vec![];
            for ft in FileType::iter() {
                let res = ft.validate_file(file_contents);
                results.push((ft, res.err()));
            }

            let num_succ = results.iter().filter(|(_, err)| err.is_none()).count();
            if num_succ == 1 {
                // Only one succeeded, we can be pretty sure its the correct filetype
                // Return the ft that succeeded:
                results
                    .into_iter()
                    .find(|(_, err)| err.is_none())
                    .unwrap()
                    .0
            } else {
                // Either none or more than one matched, raise error:
                let mut report = zerr!(
                    Zerr::FileCmdUsageError,
                    "{}",
                    if num_succ == 0 {
                        "Filetype could not be inferred automatically when file extension is unknown, parsing failed using all filetype parsers.".to_string()
                    } else {
                        format!("Filetype could not be inferred automatically when file extension is unknown, multiple filetype parsers can decode the contents.\nSpecify e.g. {} to disambiguate.", valid_ft_opts_str())
                    }
                );

                for (ft, maybe_err) in results {
                    report = report.attach_printable(format!(
                        "{:?}: {}",
                        ft,
                        if let Some(err) = maybe_err {
                            format!("parsing failed: {:?}", err)
                        } else {
                            "valid".to_string()
                        }
                    ));
                }

                return Err(report);
            }
        }
    };

    // Make sure contents valid for filetype (some of the above branches don't check the file):
    ft.validate_file(file_contents)?;

    Ok(ft)
}
