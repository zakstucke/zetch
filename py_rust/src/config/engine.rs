use std::{
    collections::{hash_map::Entry, HashMap},
    fs, io,
    ops::Deref,
    path::Path,
};

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use pyo3::{
    prelude::*,
    types::{PyDict, PyList, PyTuple},
};
use pythonize::{depythonize, pythonize};
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::Config;
use crate::prelude::*;

pub static PY_CONTEXT: Lazy<Mutex<Option<PyObject>>> = Lazy::new(Mutex::default);
static PY_USER_FUNCS: Lazy<Mutex<HashMap<String, PyObject>>> = Lazy::new(Mutex::default);

pub fn register_py_func(py: Python, py_fn: &PyAny) -> Result<(), Zerr> {
    let (module_name, fn_name) = (|| -> core::result::Result<_, PyErr> {
        let module_name = py_fn.getattr("__module__")?.extract::<String>()?;
        let fn_name = py_fn.getattr("__name__")?.extract::<String>()?;
        Ok((module_name, fn_name))
    })()
    .change_context(Zerr::InternalError)?;

    debug!("Registering custom function: '{}.{}'", module_name, fn_name);

    // Confirm it's a function:
    if !py_fn.is_callable() {
        return Err(zerr!(
            Zerr::CustomPyFunctionError,
            "Failed to register custom function: '{}.{}' as it's not a function",
            module_name,
            fn_name
        ));
    }

    let mut func_store = PY_USER_FUNCS.lock();

    // Raise error if something with the same name already registered:
    if let Entry::Vacant(e) = func_store.entry(fn_name.clone()) {
        e.insert(py_fn.to_object(py));
    } else {
        return Err(zerr!(
            Zerr::CustomPyFunctionError,
            "Failed to register custom function: '{}.{}' as '{}' is already registered.",
            module_name,
            fn_name,
            fn_name
        ));
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Engine {
    #[serde(default = "default_block_start")]
    block_start: String,
    #[serde(default = "default_block_end")]
    block_end: String,
    #[serde(default = "default_variable_start")]
    variable_start: String,
    #[serde(default = "default_variable_end")]
    variable_end: String,
    #[serde(default = "default_comment_start")]
    comment_start: String,
    #[serde(default = "default_comment_end")]
    comment_end: String,
    #[serde(default = "default_keep_trailing_newline")]
    keep_trailing_newline: bool,
    #[serde(default = "default_allow_undefined")]
    allow_undefined: bool,
    #[serde(default = "default_custom_extensions")]
    pub custom_extensions: Vec<String>,
}

impl Engine {
    pub fn default() -> Self {
        Self {
            // NOTE: when adding new, make sure to update schema.json and tests/helpers/types.py plus update tests.
            block_start: default_block_start(),
            block_end: default_block_end(),
            variable_start: default_variable_start(),
            variable_end: default_variable_end(),
            comment_start: default_comment_start(),
            comment_end: default_comment_end(),
            keep_trailing_newline: default_keep_trailing_newline(),
            allow_undefined: default_allow_undefined(),
            custom_extensions: default_custom_extensions(),
        }
    }

    pub fn create_minijinja_env<'a>(
        &self,
        root: &Path,
        conf: &'a Config,
    ) -> Result<minijinja::Environment<'a>, Zerr> {
        let mut env: minijinja::Environment<'a> = minijinja::Environment::new();
        // Adding in extra builtins like urlencode, tojson and pluralize:
        minijinja_contrib::add_to_environment(&mut env);

        // User configurable options added below:

        env.set_syntax(minijinja::Syntax {
            block_start: self.block_start.clone().into(),
            block_end: self.block_end.clone().into(),
            variable_start: self.variable_start.clone().into(),
            variable_end: self.variable_end.clone().into(),
            comment_start: self.comment_start.clone().into(),
            comment_end: self.comment_end.clone().into(),
        })
        .change_context(Zerr::InternalError)?;
        env.set_keep_trailing_newline(self.keep_trailing_newline);
        env.set_undefined_behavior(if self.allow_undefined {
            minijinja::UndefinedBehavior::Lenient
        } else {
            minijinja::UndefinedBehavior::Strict
        });

        // Disable all default auto escaping, this caused problems with e.g. adding strings around values in json files:
        env.set_auto_escape_callback(|_: &str| -> minijinja::AutoEscape {
            minijinja::AutoEscape::None
        });

        // This will allow loading files from templates using the relative root e.g. ./template where . is the root dir:
        env.set_loader(custom_loader(root));

        // Load in the context:
        for (name, value) in conf.context.iter() {
            env.add_global(name, minijinja::Value::from_serializable(value));
        }

        // Load in custom rust functions:
        env.add_function("env_default", gen_env_default_fn(conf));

        // Load in any custom extensions to the PY_USER_FUNCS global:
        if !self.custom_extensions.is_empty() {
            Python::with_gil(|py| {
                // Pythonize a copy of the context and add to the global PY_CONTEXT so its usable from zetch.context():
                let mut py_ctx = PY_CONTEXT.lock();
                *py_ctx = Some(pythonize(py, &conf.context).change_context(Zerr::InternalError)?);

                let syspath: &PyList = py
                    .import("sys")
                    .change_context(Zerr::InternalError)?
                    .getattr("path")
                    .change_context(Zerr::InternalError)?
                    .downcast()
                    .map_err(|e| {
                        zerr!(
                            Zerr::InternalError,
                            "Failed to get sys.path whilst importing custom extension: '{}'",
                            e
                        )
                    })?;
                for extension_path in self.custom_extensions.iter() {
                    let result: Result<(), Zerr> = (|| {
                        // Get the parent dir of the file/module:
                        let path = Path::new(extension_path);
                        let parent = path.parent().ok_or_else(|| {
                            zerr!(
                                Zerr::InternalError,
                                "Failed to get parent of path '{}'",
                                extension_path
                            )
                        })?;
                        let name = path
                            .file_stem()
                            .ok_or_else(|| {
                                zerr!(
                                    Zerr::InternalError,
                                    "Failed to get file stem of path '{}'",
                                    extension_path
                                )
                            })?
                            .to_str()
                            .ok_or_else(|| {
                                zerr!(
                                    Zerr::InternalError,
                                    "Failed to convert file stem to string of path '{}'",
                                    extension_path
                                )
                            })?;
                        syspath
                            .insert(0, parent)
                            .change_context(Zerr::InternalError)?;
                        py.import(name).change_context(Zerr::InternalError)?;
                        Ok(())
                    })();

                    if result.is_err() {
                        return result.attach_printable_lazy(|| {
                            format!("Failed to import custom extension '{}'.", extension_path)
                        });
                    }
                }

                Ok::<_, error_stack::Report<Zerr>>(())
            })?;

            // Consume current contents of PY_USER_FUNCS and add to minijinja env:
            let mut custom_funcs_global = PY_USER_FUNCS.lock();

            // Extract all the loaded funcs from the global mutex to be passed to individual closures:
            let custom_funcs = std::mem::take(&mut *custom_funcs_global);
            *custom_funcs_global = HashMap::new();

            for (name, py_fn) in custom_funcs.into_iter() {
                // Confirm doesn't clash with config var:
                if conf.context.contains_key(&name) {
                    return Err(zerr!(
                        Zerr::ConfigInvalid,
                        "Failed to register custom function: '{}.{}' as it clashes with a context key.",
                        Python::with_gil(|py| {py_fn.getattr(py, "__module__")?.extract::<String>(py)}).change_context(Zerr::InternalError)?,
                        name
                    ));
                }

                env.add_function(
                    name.clone(),
                    move |
                          values: minijinja::value::Rest<minijinja::Value>|
                          -> core::result::Result<minijinja::Value, minijinja::Error> {
                        // Loop over the values and extract the args and kwargs given to the func:
                        let mut args = vec![];
                        let mut kwargs: HashMap<String, minijinja::Value> = HashMap::new();
                        for value in values.deref().iter() {
                            if value.is_kwargs() {
                                for key in value.try_iter()? {
                                    let kwarg_val = value.get_item(&key)?;
                                    kwargs.insert(key.into(), kwarg_val);
                                }
                            } else {
                                args.push(value);
                            }
                        }

                        let result =
                            Python::with_gil(|py| -> Result<serde_json::Value, Zerr> {
                                let py_args = PyTuple::new(
                                    py,
                                    args.into_iter()
                                        .map(|v| {
                                            let py_val = pythonize(py, v).change_context(Zerr::InternalError)?;
                                            Ok(py_val)
                                        })
                                        .collect::<Result<Vec<_>, Zerr>>()?,
                                );

                                let py_kwargs = match kwargs.is_empty() {
                                    true => Ok::<_, Zerr>(None),
                                    false => {
                                        let dic = PyDict::new(py);
                                        for (key, value) in kwargs {
                                            let py_val = pythonize(py, &value).change_context(Zerr::InternalError)?;
                                            dic.set_item(key, py_val).change_context(Zerr::InternalError)?;
                                        }
                                        Ok(Some(dic))
                                    }
                                }?;

                                let py_result = py_fn
                                    .call(py, py_args, py_kwargs)
                                    .map_err(|e: PyErr| zerr!(Zerr::CustomPyFunctionError, "{}", e))?;

                                let rustified: serde_json::Value =
                                    depythonize(py_result.as_ref(py)).change_context(Zerr::CustomPyFunctionError).attach_printable_lazy(|| {
                                        "Failed to convert python result to a rust-like value."
                                    })?;

                                Ok(rustified)
                            });

                        match result {
                            Err(e) => Err(minijinja::Error::new(
                                minijinja::ErrorKind::InvalidOperation,
                                format!("Failed to call custom filter '{}'. Err: \n{:?}", name, e),
                            )),
                            Ok(result) => Ok(minijinja::Value::from_serializable(&result)),
                        }
                    },
                )
            }
        }

        Ok(env)
    }
}

fn default_block_start() -> String {
    // NOTE: when changing make sure to update schema.json default for config hinting
    "{%".to_string()
}

fn default_block_end() -> String {
    // NOTE: when changing make sure to update schema.json default for config hinting
    "%}".to_string()
}

fn default_variable_start() -> String {
    // NOTE: when changing make sure to update schema.json default for config hinting
    "{{".to_string()
}

fn default_variable_end() -> String {
    // NOTE: when changing make sure to update schema.json default for config hinting
    "}}".to_string()
}

fn default_comment_start() -> String {
    // NOTE: when changing make sure to update schema.json default for config hinting
    "{#".to_string()
}

fn default_comment_end() -> String {
    // NOTE: when changing make sure to update schema.json default for config hinting
    "#}".to_string()
}

fn default_keep_trailing_newline() -> bool {
    // NOTE: when changing make sure to update schema.json default for config hinting
    // Don't modify a user's source code if we can help it:
    true
}

fn default_allow_undefined() -> bool {
    // NOTE: when changing make sure to update schema.json default for config hinting
    false
}

fn default_custom_extensions() -> Vec<String> {
    // NOTE: when changing make sure to update schema.json default for config hinting
    vec![]
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
    conf: &Config,
) -> impl Fn(String) -> core::result::Result<minijinja::Value, minijinja::Error> {
    // Get a simple dict of available env vars to their defaults as minijinja::Value(s):
    let mut env_defaults = HashMap::new();
    for (key, value) in conf.raw.context.env.iter() {
        if let Some(default) = value.default.as_ref() {
            env_defaults.insert(key.clone(), minijinja::Value::from_serializable(default));
        }
    }

    move |name: String| match env_defaults.get(&name) {
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
    }
}
