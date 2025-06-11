use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

use sha2::{digest::generic_array::GenericArray, Digest, Sha256};
use tracing::{debug, warn};

use super::template;
use crate::prelude::*;
pub static LOCKFILE_NAME: &str = ".zetch.lock";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Contents {
    version: String,
    // Keep ordering in lockfile static to reduce git conflicts and diff noise:
    #[serde(serialize_with = "crate::utils::ordered_map_serializer")]
    files: HashMap<String, String>,
}

impl Contents {
    pub fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            files: HashMap::new(),
        }
    }
}

pub fn hash_contents(contents: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(contents.as_bytes());
    let mut out = GenericArray::default();
    hasher.finalize_into_reset(&mut out);
    format!("{out:x}")
}

pub struct Lockfile {
    filepath: PathBuf,
    seen_template_paths: HashSet<String>,
    contents: Contents,
    // Modified at the moment is the same as newly_created,
    // but during template additions modified may become different:
    pub _newly_created: bool,
    pub modified: bool,
}

impl Lockfile {
    pub fn load(root: PathBuf, force: bool) -> Self {
        let filepath = root.join(LOCKFILE_NAME);
        let mut modified = false;
        let mut newly_created = false;

        let contents = if force {
            newly_created = true;
            modified = true;
            warn!("Cli forced lockfile override.");
            Contents::default()
        } else {
            let str_contents = match fs::read_to_string(&filepath) {
                Ok(contents) => Some(contents),
                Err(err) => {
                    warn!(
                        "Starting lockfile afresh, failed to read existing at '{}': {}",
                        filepath.display(),
                        err
                    );
                    None
                }
            };

            match str_contents {
                Some(str_contents) => match serde_json::from_str::<Contents>(&str_contents) {
                    Ok(contents) => {
                        if contents.version != env!("CARGO_PKG_VERSION") {
                            warn!(
                                "Starting lockfile afresh, version mismatch: {} != {}",
                                contents.version,
                                env!("CARGO_PKG_VERSION")
                            );
                            modified = true;
                            newly_created = true;
                            Contents::default()
                        } else {
                            debug!(
                                "Loaded lockfile from '{}' successfully.",
                                filepath.display()
                            );
                            contents
                        }
                    }
                    Err(err) => {
                        warn!(
                            "Starting lockfile afresh, failed to parse existing at '{}': {}",
                            filepath.display(),
                            err
                        );
                        modified = true;
                        newly_created = true;
                        Contents::default()
                    }
                },
                None => {
                    debug!(
                        "Couldn't find existing lockfile, creating new at '{}'",
                        filepath.display()
                    );
                    modified = true;
                    newly_created = true;
                    Contents::default()
                }
            }
        };

        Self {
            filepath,
            contents,
            seen_template_paths: HashSet::new(),
            _newly_created: newly_created,
            modified,
        }
    }

    /// After compiling a template run this, it will update the lockfile and write the compiled template to disk.
    ///
    /// Returns true when added, false when identical already present in lockfile.
    pub fn add_template(
        &mut self,
        template: &template::Template,
        compiled: String,
    ) -> Result<bool, Report<Zerr>> {
        // To prevent bloating the filesize and readability of the lockfile, only include a hash of the compiled template rather than the full contents. (sha-256)
        let hashed = timeit!("Hashing compiled files for lockfile", {
            hash_contents(&compiled)
        });

        let identical = if let Some(old_hashed) = self.contents.files.get(&template.rel_path) {
            if old_hashed != &hashed {
                debug!(
                    "Template '{}' has changed, updating lockfile and rewriting.",
                    template.rel_path
                );
                self.modified = true;
                false
            } else {
                debug!(
                    "Template '{}' has identical hash in lockfile, skipping.",
                    template.rel_path
                );
                true
            }
        } else {
            debug!(
                "Template '{}' didn't exist in lockfile prior, updating lockfile and rewriting.",
                template.rel_path
            );
            self.modified = true;
            false
        };

        // Only update if not already identical:
        if !identical {
            self.modified = true;
            self.contents
                .files
                .insert(template.rel_path.clone(), hashed);

            // Write the compiled file:
            fs::write(template.out_path.clone(), compiled).change_context(Zerr::InternalError)?;
        }

        self.seen_template_paths.insert(template.rel_path.clone());

        Ok(!identical)
    }

    /// After all compiled templates have been added, run this to close out and save the lockfile.
    pub fn sync(&mut self) -> Result<(), Report<Zerr>> {
        let before_len = self.contents.files.len();
        // Anything which isn't in the new compiled set should be removed from the lockfile:
        self.contents
            .files
            .retain(|template_path, _| self.seen_template_paths.contains(template_path));

        if self.contents.files.len() != before_len {
            debug!(
                "Removed {} templates from lockfile which no longer exist.",
                before_len - self.contents.files.len()
            );
            self.modified = true;
        }

        if self.modified {
            // Write the updated lockfile
            debug!("Writing updated lockfile to '{}'", self.filepath.display());
            fs::write(
                &self.filepath,
                serde_json::to_string_pretty(&self.contents).change_context(Zerr::InternalError)?,
            )
            .change_context(Zerr::InternalError)?;
        }

        Ok(())
    }
}
