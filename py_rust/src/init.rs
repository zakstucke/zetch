use std::path::PathBuf;

use crate::{
    args::{InitCommand, DEFAULT_CONFIG_PATH},
    prelude::*,
};

/// Initialize the config file in the current directory.
pub fn init(_args: &InitCommand) -> Result<(), Zerr> {
    // Raise if config file already exists:
    if PathBuf::from(&DEFAULT_CONFIG_PATH).exists() {
        return Err(zerr!(
            Zerr::ConfigExistsError,
            "Config file already exists at the default location: '{}'.",
            DEFAULT_CONFIG_PATH
        ));
    }

    // Check if a .gitignore file exists in the current directory:
    let gitignore_path = PathBuf::from(".gitignore");
    let gitignore_exists = gitignore_path.exists();

    std::fs::write(DEFAULT_CONFIG_PATH, get_default_conf(gitignore_exists)).map_err(|e| {
        zerr!(
            Zerr::InternalError,
            "Failed to write config file to '{}': '{}'.",
            DEFAULT_CONFIG_PATH,
            e
        )
    })?;

    info!(
        "Successfully wrote config file to '{}'.",
        DEFAULT_CONFIG_PATH
    );

    Ok(())
}

static SCHEMA_DIRECTIVE_PREFIX: &str = "#:schema ";

/// The directive that should be placed at the top of the config file to enable linting and descriptions.
///
/// Note this should be prepended with `"#:schema"`
fn get_schema_directive() -> String {
    format!(
        "https://github.com/zakstucke/zetch/blob/v{}/py_rust/src/config/schema.json",
        env!("CARGO_PKG_VERSION")
    )
}

/// Should be called by the config loader that reads the contents & has access to the config path.
/// Will return the config contents with the updated schema directive if changed.
/// Will not modify the formatting or any other part of the config file.
///
/// The caller must rewrite the updated config file if returned.
pub fn update_schema_directive_if_needed(contents: &str) -> Option<String> {
    for (index, line) in contents.lines().enumerate() {
        let line = line.trim(); // Ignore any whitespace
        if let Some(old_directive) = line.strip_prefix(SCHEMA_DIRECTIVE_PREFIX) {
            let cur_directive = get_schema_directive();
            if old_directive != cur_directive {
                let mut new_contents: Vec<&str> = contents.lines().collect();
                let replacement = format!("{}{}", SCHEMA_DIRECTIVE_PREFIX, cur_directive);
                new_contents[index] = &replacement;
                warn!(
                    "Found old config schema directive, updating from '{line}' to '{replacement}'."
                );
                return Some(new_contents.join("\n"));
            }
        } else if !line.is_empty() {
            // If there's a line with contents before finding a schema, no schema so break:
            break;
        }
    }
    None
}

fn get_default_conf(gitignore_exists: bool) -> String {
    format!(
        r#"{}{}

ignore_files = [{}] {}

exclude = []

# Matchers zetch will use to identify templates.
# E.g. by default files like foo.zetch.txt & foo.txt.zetch are matched.
matchers = ["zetch"]

[engine]
keep_trailing_newline = true
allow_undefined = false
custom_extensions = []

[context.static]
FOO = {{ value = "foo" }}

[context.env]
BAR = {{ default = "bar" }}

[context.cli]
BAZ = {{ commands = ["echo 1"], coerce = "int" }}
"#,
        SCHEMA_DIRECTIVE_PREFIX,
        get_schema_directive(),
        if gitignore_exists {
            "\".gitignore\""
        } else {
            ""
        },
        if gitignore_exists {
            ""
        } else {
            "# Couldn't find a .gitignore, not adding by default. Recommended if available."
        }
    )
}
