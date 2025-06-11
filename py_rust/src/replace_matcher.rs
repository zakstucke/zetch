use std::path::PathBuf;

use crate::{args::ReplaceMatcherCommand, prelude::*, state::State};

/// Search the current directory for all template files (using the old matcher), replace the filename with the new matcher.
///
/// Will show all files that will be renamed and prompt for confirmation before renaming.
pub fn replace(
    args: &crate::args::Args,
    replace_args: &ReplaceMatcherCommand,
) -> Result<(), Report<Zerr>> {
    let root = PathBuf::from(".");
    let state = State::new(args)?;

    let mapping = crate::render::get_template_matcher_rewrite_mapping(
        &root,
        &state,
        &replace_args.old_matcher,
        &replace_args.new_matcher,
    )?;

    if mapping.is_empty() {
        info!(
            "No templates found with matcher '{}'.",
            replace_args.old_matcher
        );
        return Ok(());
    }

    info!(
        "\nFound {} templates with matcher '{}'. The following files will be renamed:\n",
        mapping.len(),
        replace_args.old_matcher
    );
    for (old_path, new_path) in mapping.iter() {
        info!("  {} -> {}", old_path.display(), new_path.display());
    }

    let confirmed = crate::utils::user_input::sync_confirm("Update filenames?")?;
    if !confirmed {
        info!("Aborting.");
    } else {
        for (old_path, new_path) in mapping.iter() {
            std::fs::rename(old_path, new_path).change_context(Zerr::InternalError)?;
        }
    }

    Ok(())
}
