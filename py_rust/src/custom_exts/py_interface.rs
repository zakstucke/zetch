use std::{
    collections::{HashMap, hash_map::Entry},
    ops::Deref,
    path::Path,
};

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use pyo3::{
    exceptions::PyValueError,
    prelude::*,
    types::{PyDict, PyTuple},
};
use pythonize::pythonize;

use crate::{prelude::*, state::State};

static PY_CONTEXT: Lazy<Mutex<Option<PyObject>>> = Lazy::new(Mutex::default);
static PY_USER_FUNCS: Lazy<Mutex<HashMap<String, PyObject>>> = Lazy::new(Mutex::default);

#[pyfunction]
#[pyo3(name = "register_function")]
pub fn py_register_function(py: Python, py_fn: Bound<PyAny>) -> PyResult<()> {
    match register_py_func(py, py_fn) {
        Ok(_) => Ok(()),
        Err(e) => Err(PyValueError::new_err(format!("{e:?}"))),
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

pub fn load_custom_exts(
    exts: &[String],
    state: &State,
) -> Result<HashMap<String, PyObject>, Report<Zerr>> {
    // Don't touch python if unneeded:
    if exts.is_empty() {
        return Ok(HashMap::new());
    }

    Python::with_gil(|py| {
        // Pythonize a copy of the context and add to the global PY_CONTEXT so its usable from zetch.context():
        let mut py_ctx = PY_CONTEXT.lock();

        if py_ctx.is_some() {
            return Err(zerr!(
                Zerr::InternalError,
                "Custom extensions loaded more than once."
            ));
        }

        *py_ctx = Some(
            pythonize(py, &state.ctx)
                .change_context(Zerr::InternalError)?
                .into_pyobject(py)
                .change_context(Zerr::InternalError)?
                .into(),
        );

        let syspath_src = py
            .import("sys")
            .change_context(Zerr::InternalError)?
            .getattr("path")
            .change_context(Zerr::InternalError)?;
        let syspath = syspath_src.downcast().map_err(|e| {
            zerr!(
                Zerr::InternalError,
                "Failed to get sys.path whilst importing custom extension: '{}'",
                e
            )
        })?;
        for extension_path in state.conf.engine.custom_extensions.iter() {
            let result: Result<(), Report<Zerr>> = (|| {
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
                // confirm the file exists:
                if !path.exists() {
                    return Err(zerr!(
                        Zerr::InternalError,
                        "Custom extension '{}' does not exist.",
                        extension_path
                    ));
                }
                // PyModule::import(py, extension_path).change_context(Zerr::InternalError)?;
                py.import(name).change_context(Zerr::InternalError)?;
                Ok(())
            })();

            if result.is_err() {
                return result.attach_printable_lazy(|| {
                    format!("Failed to import custom extension '{extension_path}'.")
                });
            }
        }

        Ok::<_, error_stack::Report<Zerr>>(())
    })?;

    // Extra the loaded user funcs, this fn is checked to only run once. So no need to clone and maintain global var.
    Ok(std::mem::take(&mut *PY_USER_FUNCS.lock()))
}

pub fn mini_values_to_py_params(
    py: Python,
    values: minijinja::value::Rest<minijinja::Value>,
) -> Result<(Bound<PyTuple>, Option<Bound<PyDict>>), Report<Zerr>> {
    // Loop over the values and extract the args and kwargs given to the func:
    let mut args = vec![];
    let mut kwargs: HashMap<String, minijinja::Value> = HashMap::new();
    for value in values.iter() {
        if value.is_kwargs() {
            for key in value.try_iter().change_context(Zerr::InternalError)? {
                let kwarg_val = value.get_item(&key).change_context(Zerr::InternalError)?;
                kwargs.insert(key.into(), kwarg_val);
            }
        } else {
            args.push(value);
        }
    }

    let py_args = PyTuple::new(
        py,
        args.into_iter()
            .map(|v| {
                let py_val = pythonize(py, v).change_context(Zerr::InternalError)?;
                Ok(py_val)
            })
            .collect::<Result<Vec<_>, Report<Zerr>>>()?,
    )
    .change_context(Zerr::InternalError)?;

    let py_kwargs = match kwargs.is_empty() {
        true => Ok::<_, Report<Zerr>>(None),
        false => {
            let dic = PyDict::new(py);
            for (key, value) in kwargs {
                let py_val = pythonize(py, &value).change_context(Zerr::InternalError)?;
                dic.set_item(key, py_val)
                    .change_context(Zerr::InternalError)?;
            }
            Ok(Some(dic))
        }
    }?;

    Ok((py_args, py_kwargs))
}

fn register_py_func(py: Python, py_fn: Bound<PyAny>) -> Result<(), Report<Zerr>> {
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
        e.insert(
            py_fn
                .into_pyobject(py)
                .change_context(Zerr::InternalError)?
                .into(),
        );
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
