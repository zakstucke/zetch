use strum::IntoEnumIterator;

use crate::{args::FileCommand, prelude::*};

/// Supported filetypes to process.
#[derive(Debug, strum::EnumIter)]
pub enum FileType {
    Json,
    Yaml,
    Toml,
}

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

/// Infers the filetype and validates the file is readable as that type.
pub fn get_filetype(
    _args: &crate::args::Args,
    fargs: &FileCommand,
    file_contents: &str,
) -> Result<FileType, Zerr> {
    let ft = if [fargs.json, fargs.yaml, fargs.toml]
        .iter()
        .filter(|x| **x)
        .count()
        > 1
    {
        return Err(zerr!(
            Zerr::FileCmdUsageError,
            "Only one of '--json', '--yaml' or '--toml' can be specified."
        ));
    } else if fargs.json {
        FileType::Json
    } else if fargs.yaml {
        FileType::Yaml
    } else if fargs.toml {
        FileType::Toml
    } else {
        // No specific given, need to infer:

        // First try to infer from the file extension:
        let maybe_ft = if let Some(ext) = fargs.filepath.extension() {
            let ext = ext.to_str().ok_or_else(|| {
                zerr!(
                    Zerr::InternalError,
                    "Could not read file extension for file: '{}'.",
                    fargs.filepath.display()
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
                        "Filetype could not be inferred automatically when file extension is unknown, parsing failed using all filetype parsers."
                    } else {
                        "Filetype could not be inferred automatically when file extension is unknown, multiple filetype parsers can decode the contents.\nSpecify e.g. --json, --yaml or --toml to disambiguate."
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
