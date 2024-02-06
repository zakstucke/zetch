#![warn(clippy::disallowed_types)]

use std::ops::Deref;

use colored::Colorize;
use config::PY_CONTEXT;
use pyo3::{exceptions::PyValueError, prelude::*};
use pythonize::depythonize;
use render::hash_contents;

mod arg_matcher;
mod args;
mod coerce;
mod config;
mod error;
mod init;
mod prelude;
mod read_write;
mod render;
mod replace_matcher;
mod run;
mod utils;
mod var;

use prelude::*;

#[pyfunction]
pub fn cli() -> i32 {
    match run::run() {
        Ok(_) => 0,
        Err(e) => {
            // if ZETCH_LOCATION env var is set, always show location:
            if std::env::var("ZETCH_LOCATION").is_err() {
                // Only include the file location of the errors if its an internal error, if its a user error its just bloat as an expected issue.
                match e.current_context() {
                    Zerr::InternalError => {}
                    _ => {
                        error_stack::Report::install_debug_hook::<std::panic::Location>(|_, _| {});
                    }
                };
            }

            #[allow(clippy::print_stderr)]
            {
                eprintln!("{}", "zetch failed".red().bold());
                eprintln!("{:?}", e);
            }
            1
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

/// Create a TOML string from a Python object, used by tests.
#[pyfunction]
#[pyo3(name = "_toml_create")]
pub fn py_toml_create(data: &PyAny) -> PyResult<String> {
    let decoded: serde_json::Value = depythonize(data)?;
    match toml::to_string(&decoded) {
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

    m.add_function(wrap_pyfunction!(py_toml_create, m)?)?;

    m.add_function(wrap_pyfunction!(py_hash_contents, m)?)?;

    Ok(())
}
