use std::path::{Path, PathBuf};

use ignore::{overrides::OverrideBuilder, WalkBuilder};
use once_cell::sync::Lazy;
use regex::Regex;
use tracing::debug;

use super::lockfile::LOCKFILE_NAME;
use crate::{
    args::RenderCommand,
    config::{final_config_path, Config},
    prelude::*,
};

pub fn create(
    args: &crate::args::Args,
    render_args: &RenderCommand,
    conf: &Config,
) -> Result<WalkBuilder, Zerr> {
    let mut builder = WalkBuilder::new(&render_args.root);
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
    if let Some(rel_config) = config_path_relative_to_root(
        &render_args.root,
        &final_config_path(&args.config, Some(&render_args.root))?,
    )? {
        all_excludes.push(rel_config.display().to_string());
    }

    // Add in config supplied excludes:
    all_excludes.extend(conf.exclude.iter().map(|s| s.to_string()));

    let mut overrider: OverrideBuilder = OverrideBuilder::new(&render_args.root);
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

static MIDDLE_MATCHER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(.*)(\.zetch\.)(.*)").expect("Regex failed to compile"));

static END_MATCHER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(.*)(\.zetch)$").expect("Regex failed to compile"));

fn try_regexes_get_match(filename: &str) -> Option<String> {
    if let Some(caps) = MIDDLE_MATCHER.captures(filename) {
        return Some(format!(
            "{}.{}",
            caps.get(1).map_or("", |m| m.as_str()),
            caps.get(3).map_or("", |m| m.as_str())
        ));
    }

    if let Some(caps) = END_MATCHER.captures(filename) {
        return Some(caps.get(1).map_or("", |m| m.as_str()).to_string());
    }

    None
}

pub fn find_templates(
    render_args: &RenderCommand,
    walker: WalkBuilder,
) -> Result<Vec<super::template::Template>, Zerr> {
    let mut templates = vec![];
    let mut files_checked = 0;
    for entry in walker.build() {
        let entry = entry.change_context(Zerr::InternalError)?;
        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            let filename = entry.file_name().to_string_lossy();
            if let Some(compiled_name) = try_regexes_get_match(&filename) {
                templates.push(super::template::Template::new(
                    render_args.root.clone(),
                    entry.path().to_path_buf(),
                    // Replacing the name with the compiled name:
                    entry.path().parent().unwrap().join(compiled_name),
                ));
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
