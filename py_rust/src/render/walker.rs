use std::path::{Path, PathBuf};

use ignore::{overrides::OverrideBuilder, WalkBuilder};
use regex::Regex;
use tracing::debug;

use super::lockfile::LOCKFILE_NAME;
use crate::{config::Config, prelude::*};

pub fn create(root: &Path, conf: &Config) -> Result<WalkBuilder, Zerr> {
    let mut builder = WalkBuilder::new(root);
    builder.git_exclude(false); // Don't auto read .git/info/exclude
    builder.git_global(false); // Don't auto use a global .gitignore file
    builder.git_ignore(false); // Don't auto use .gitignore file
    builder.ignore(false); // Don't auto use .ignore file
    builder.require_git(false); // Works better when not in a git repo
    builder.hidden(false); // Doesn't auto ignore hidden files

    for ignore_file in conf.ignore_files.iter() {
        builder.add_ignore(ignore_file);
    }

    let mut all_excludes = vec![
        // Don't ever match the lockfile:
        LOCKFILE_NAME.to_string(),
    ];

    // If the config is inside the root, add it to the excludes:
    if let Some(rel_config) = config_path_relative_to_root(root, &conf.final_config_path)? {
        all_excludes.push(rel_config.display().to_string());
    }

    // Add in config supplied excludes:
    all_excludes.extend(conf.exclude.iter().map(|s| s.to_string()));

    let mut overrider: OverrideBuilder = OverrideBuilder::new(root);
    for exclude in all_excludes.iter() {
        // The override adder is the opposite, i.e. a match is a whitelist, so need to invert the exclude pattern provided:
        let trimmed = exclude.trim();
        let inverted = if trimmed.starts_with('!') {
            // Remove the leading "!" to invert:
            trimmed
                .strip_prefix('!')
                .ok_or_else(|| {
                    zerr!(
                        Zerr::InternalError,
                        "Failed to strip leading '!' from exclude: {}",
                        trimmed
                    )
                })?
                .to_string()
        } else {
            // Add a leading "!" to invert:
            format!("!{}", trimmed)
        };
        overrider
            .add(&inverted)
            .change_context(Zerr::InternalError)?;
    }

    builder.overrides(overrider.build().change_context(Zerr::InternalError)?);

    Ok(builder)
}

/// If config is inside root, return the relative path to it, otherwise return None.
fn config_path_relative_to_root(root: &Path, config: &Path) -> Result<Option<PathBuf>, Zerr> {
    // Make both absolute to start:
    let root = if root.is_relative() {
        root.canonicalize().change_context(Zerr::InternalError)?
    } else {
        root.to_path_buf()
    };

    let config = if config.is_relative() {
        config.canonicalize().change_context(Zerr::InternalError)?
    } else {
        config.to_path_buf()
    };

    // If config is inside root, return the relative path to it, otherwise return None.
    if config.starts_with(&root) {
        Ok(Some(
            config
                .strip_prefix(&root)
                .change_context(Zerr::InternalError)?
                .to_path_buf(),
        ))
    } else {
        Ok(None)
    }
}

fn get_middle_regex(matcher: &str) -> Regex {
    Regex::new(&format!(r"(.*)(\.{}\.)(.*)", matcher)).expect("Regex failed to compile")
}

fn get_end_regex(matcher: &str) -> Regex {
    Regex::new(&format!(r"(.*)(\.{})$", matcher)).expect("Regex failed to compile")
}

fn try_regexes_and_rewrite(
    filename: &str,
    middle_regex: &Regex,
    end_regex: &Regex,
) -> Option<String> {
    if let Some(caps) = middle_regex.captures(filename) {
        return Some(format!(
            "{}.{}",
            caps.get(1).map_or("", |m| m.as_str()),
            caps.get(3).map_or("", |m| m.as_str())
        ));
    }

    if let Some(caps) = end_regex.captures(filename) {
        return Some(caps.get(1).map_or("", |m| m.as_str()).to_string());
    }

    None
}

pub fn find_templates(
    root: &Path,
    walker: WalkBuilder,
    matchers: &[String],
) -> Result<Vec<super::template::Template>, Zerr> {
    let regex_pairs = matchers
        .iter()
        .map(|matcher| {
            let middle_regex = get_middle_regex(matcher);
            let end_regex = get_end_regex(matcher);
            (middle_regex, end_regex)
        })
        .collect::<Vec<_>>();

    let mut templates = vec![];
    let mut files_checked = 0;
    for entry in walker.build() {
        let entry = entry.change_context(Zerr::InternalError)?;
        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            let filename = entry.file_name().to_string_lossy();
            for (middle_regex, end_regex) in regex_pairs.iter() {
                if let Some(compiled_name) =
                    try_regexes_and_rewrite(&filename, middle_regex, end_regex)
                {
                    templates.push(super::template::Template::new(
                        root.into(),
                        entry.path().to_path_buf(),
                        // Replacing the name with the compiled name:
                        entry.path().parent().unwrap().join(compiled_name),
                    ));
                    // Don't match twice with different matchers:
                    break;
                }
            }
        }
        files_checked += 1;
    }

    debug!(
        "Checked {} unignored files to find {} templates.",
        files_checked,
        templates.len()
    );

    Ok(templates)
}

/// Replace the matcher in the filename with the new matcher.
/// Used by the replace-matcher command.
fn rewrite_template_matcher(
    filename: &str,
    middle_regex: &Regex,
    end_regex: &Regex,
    new_matcher: &str,
) -> Result<String, Zerr> {
    let filename = if let Some(caps) = middle_regex.captures(filename) {
        format!(
            "{}.{}.{}",
            caps.get(1).map_or("", |m| m.as_str()),
            new_matcher,
            caps.get(3).map_or("", |m| m.as_str())
        )
    } else {
        filename.to_string()
    };

    let filename = if let Some(caps) = end_regex.captures(&filename) {
        format!("{}.{}", caps.get(1).map_or("", |m| m.as_str()), new_matcher)
    } else {
        filename
    };

    Ok(filename)
}

/// Returns a mapping of current template paths to new template paths with an old and new match string.
/// Used by the replace-matcher command, otherwise used internally in render().
pub fn get_template_matcher_rewrite_mapping(
    root: &Path,
    conf: &Config,
    old_matcher: &str,
    new_matcher: &str,
) -> Result<Vec<(PathBuf, PathBuf)>, Zerr> {
    let templates = find_templates(root, create(root, conf)?, &[old_matcher.to_string()])?;

    let middle_regex = get_middle_regex(old_matcher);
    let end_regex = get_end_regex(old_matcher);

    templates
        .into_iter()
        .map(|t| {
            let old_filename = t
                .path
                .file_name()
                .ok_or_else(|| {
                    zerr!(
                        Zerr::InternalError,
                        "Failed to get filename from path: {}",
                        t.path.display()
                    )
                })?
                .to_string_lossy()
                .to_string();

            let new_path = t
                .path
                .to_path_buf()
                .with_file_name(rewrite_template_matcher(
                    &old_filename,
                    &middle_regex,
                    &end_regex,
                    new_matcher,
                )?);
            Ok((t.path, new_path))
        })
        .collect::<Result<Vec<_>, Zerr>>()
}
