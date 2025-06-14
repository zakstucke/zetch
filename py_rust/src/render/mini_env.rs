use std::{collections::HashMap, fs, io, path::Path};

use minijinja::syntax::SyntaxConfig;
use pyo3::prelude::*;
use pythonize::depythonize;

use crate::{custom_exts::py_interface, prelude::*, state::State};

pub fn new_mini_env<'a>(
    root: &Path,
    state: &'a State,
) -> Result<minijinja::Environment<'a>, Report<Zerr>> {
    let mut env: minijinja::Environment<'a> = minijinja::Environment::new();
    // Adding in extra builtins like urlencode, tojson and pluralize:
    minijinja_contrib::add_to_environment(&mut env);

    // User configurable options added below:

    env.set_syntax(
        SyntaxConfig::builder()
            .block_delimiters(
                state.conf.engine.block_start.clone(),
                state.conf.engine.block_end.clone(),
            )
            .variable_delimiters(
                state.conf.engine.variable_start.clone(),
                state.conf.engine.variable_end.clone(),
            )
            .comment_delimiters(
                state.conf.engine.comment_start.clone(),
                state.conf.engine.comment_end.clone(),
            )
            .build()
            .change_context(Zerr::InternalError)?,
    );

    // Used to be user configurable, but want to modify code as little as possible, so forcibly disable modification of newlines:
    env.set_keep_trailing_newline(true);

    // Forcibly prevent undefined vars:
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);

    // Disable all default auto escaping, this caused problems with e.g. adding strings around values in json files:
    env.set_auto_escape_callback(|_: &str| -> minijinja::AutoEscape {
        minijinja::AutoEscape::None
    });

    // This will allow loading files from templates using the relative root e.g. ./template where . is the root dir:
    env.set_loader(custom_loader(root));

    // Load in the context:
    for (name, value) in state.ctx.iter() {
        env.add_global(name, minijinja::Value::from_serialize(value));
    }

    // Load in custom rust functions:
    env.add_function("env_default", gen_env_default_fn(state)?);

    // Load in any custom extensions to the PY_USER_FUNCS global:
    let custom_funcs = py_interface::load_custom_exts(&state.conf.engine.custom_extensions, state)?;
    for (name, py_fn) in custom_funcs.into_iter() {
        debug!("Registering custom function: '{}'", name);

        // Confirm doesn't clash with config var:
        if state.ctx.contains_key(&name) {
            return Err(zerr!(
                Zerr::ConfigInvalid,
                "Failed to register custom function: '{}.{}' as it clashes with a context key.",
                Python::with_gil(|py| { py_fn.getattr(py, "__module__")?.extract::<String>(py) })
                    .change_context(Zerr::InternalError)?,
                name
            ));
        }

        // If superlight, add a pseudo fn that returns an empty string
        if state.superlight {
            let empty_str = minijinja::Value::from_safe_string("".to_string());
            env.add_function(
                name.clone(),
                move |_values: minijinja::value::Rest<minijinja::Value>| empty_str.clone(),
            );
        } else {
            // Add the rust-wrapped python fn to the minijinja environment:
            env.add_function(
                name.clone(),
                move |
                        values: minijinja::value::Rest<minijinja::Value>|
                        -> core::result::Result<minijinja::Value, minijinja::Error> {
                    let result =
                        Python::with_gil(|py| -> Result<serde_json::Value, Report<Zerr>> {
                            let (py_args, py_kwargs) = py_interface::mini_values_to_py_params(py, values)?;
                            let py_result = py_fn
                                .call(py, py_args, py_kwargs.as_ref())
                                .map_err(|e: PyErr| zerr!(Zerr::CustomPyFunctionError, "{}", e))?;
                            let rustified: serde_json::Value =
                                depythonize(py_result.bind(py)).change_context(Zerr::CustomPyFunctionError).attach_printable_lazy(|| {
                                    "Failed to convert python result to a rust-like value."
                                })?;
                            Ok(rustified)
                        });
                    match result {
                        Err(e) => Err(minijinja::Error::new(
                            minijinja::ErrorKind::InvalidOperation,
                            format!("Failed to call custom filter '{name}'. Err: \n{e:?}"),
                        )),
                        Ok(result) => Ok(minijinja::Value::from_serialize(&result)),
                    }
                },
            )
        }
    }

    Ok(env)
}

fn custom_loader<'x, P: AsRef<Path> + 'x>(
    dir: P,
) -> impl for<'a> Fn(&'a str) -> core::result::Result<Option<String>, minijinja::Error>
+ Send
+ Sync
+ 'static {
    let dir = dir.as_ref().to_path_buf();
    move |name| match fs::read_to_string(dir.join(name)) {
        Ok(result) => Ok(Some(result)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            "could not read template",
        )
        .with_source(err)),
    }
}

fn gen_env_default_fn(
    state: &State,
) -> Result<
    impl Fn(String) -> core::result::Result<minijinja::Value, minijinja::Error> + use<>,
    Report<Zerr>,
> {
    // Get a simple dict of available env vars to their defaults as minijinja::Value(s):
    let mut env_defaults = HashMap::new();
    for (key, value) in state.conf.context.env.iter() {
        if let Some(var) = value.default.as_ref() {
            env_defaults.insert(key.clone(), minijinja::Value::from_serialize(&var.read()?));
        }
    }

    Ok(move |name: String| match env_defaults.get(&name) {
        Some(default) => Ok(default.clone()),
        None => {
            let mut env_keys = env_defaults
                .keys()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>();
            env_keys.sort_by_key(|name| name.to_lowercase());
            Err(minijinja::Error::new(
                minijinja::ErrorKind::InvalidOperation,
                format!(
                    "context.env var '{}' doesn't exist or doesn't have a default. All ctx env vars with defaults: '{}'.",
                    name,
                    env_keys.join(", ")
                ),
            ))
        }
    })
}
