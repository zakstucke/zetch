#![warn(clippy::disallowed_types)]

use std::ops::Deref;

use colored::Colorize;
use config::PY_CONTEXT;
use pyo3::{exceptions::PyValueError, prelude::*};
use pythonize::depythonize;
use render::hash_contents;

mod arg_matcher;
mod args;
mod config;
mod error;
mod init;
mod prelude;
mod read;
mod render;
mod replace_matcher;
mod run;
mod utils;

use prelude::*;

#[pyfunction]
pub fn cli() {
    match run::run() {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            // Only include the file location of the errors if its an internal error, if its a user error its just bloat as an expected issue.
            match e.current_context() {
                Zerr::InternalError => {}
                _ => {
                    error_stack::Report::install_debug_hook::<std::panic::Location>(|_, _| {});
                }
            };

            #[allow(clippy::print_stderr)]
            {
                eprintln!("{}", "zetch failed".red().bold());
                eprintln!("{:?}", e);
            }
            // Wait 5ms to make sure logs have all flushed, std::process::exit() will cause logs to be dropped if they haven't finished flushing.
            std::thread::sleep(std::time::Duration::from_millis(5));
            std::process::exit(1);
        }
    }
}

#[pyfunction]
#[pyo3(name = "register_function")]
pub fn py_register_function(py: Python, py_fn: &PyAny) -> PyResult<()> {
    match config::register_py_func(py, py_fn) {
        Ok(_) => Ok(()),
        Err(e) => Err(PyValueError::new_err(format!("{:?}", e))),
    }
}

/// Get the current context as a Python dictionary to be used in custom user functions.
#[pyfunction]
#[pyo3(name = "context")]
pub fn py_context(py: Python) -> PyResult<PyObject> {
    let py_ctx = PY_CONTEXT.lock();
    if let Some(py_ctx) = py_ctx.deref() {
        Ok(py_ctx.clone_ref(py))
    } else {
        Err(PyValueError::new_err(
            "Context not registered. This should only be called by custom user extensions.",
        ))
    }
}

#[pyfunction]
#[pyo3(name = "_toml_update")]
pub fn py_toml_update(
    initial: &str,
    update: Option<&PyAny>,
    remove: Option<&PyAny>,
) -> PyResult<String> {
    let update: Option<serde_json::Value> = if let Some(update) = update {
        depythonize(update)?
    } else {
        None
    };
    let remove: Option<Vec<Vec<String>>> = if let Some(remove) = remove {
        depythonize(remove)?
    } else {
        None
    };

    match utils::toml::update(initial, update, remove) {
        Ok(s) => Ok(s),
        Err(e) => Err(PyValueError::new_err(format!("{:?}", e))),
    }
}

#[pyfunction]
#[pyo3(name = "_hash_contents")]
pub fn py_hash_contents(contents: &str) -> PyResult<String> {
    Ok(hash_contents(contents))
}

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
#[pyo3(name = "_rs")]
fn root_module(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(cli, m)?)?;

    m.add_function(wrap_pyfunction!(py_register_function, m)?)?;

    m.add_function(wrap_pyfunction!(py_context, m)?)?;

    m.add_function(wrap_pyfunction!(py_toml_update, m)?)?;

    m.add_function(wrap_pyfunction!(py_hash_contents, m)?)?;

    Ok(())
}
