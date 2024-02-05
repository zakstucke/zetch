use pyo3::{prelude::*, types::PyDict};

use crate::prelude::*;

/// An individual accessor for mapping or array in a yaml file.
#[derive(Clone)]
pub enum YamlLoc {
    Key(String),
    Index(usize),
}

impl IntoPy<PyObject> for YamlLoc {
    fn into_py(self, py: Python<'_>) -> PyObject {
        match self {
            YamlLoc::Index(index) => index.into_py(py),
            YamlLoc::Key(key) => key.into_py(py),
        }
    }
}

/// An update that should be applied to a yaml document.
///
/// NOTES:
/// - Intermediary paths on a deletion should exist, shouldn't call into python if not needed!
/// - Intermediary paths on puts should be made in individual batch updates, as little logic in python as poss.
pub enum YamlUpdate {
    Delete(Vec<YamlLoc>),
    // Where the string is a json_str that will be decoded python side.
    Put(Vec<YamlLoc>, String),
}

impl IntoPy<PyObject> for YamlUpdate {
    fn into_py(self, py: Python<'_>) -> PyObject {
        let mut items: Vec<PyObject> = vec![];
        let path = match self {
            YamlUpdate::Delete(path) => path,
            YamlUpdate::Put(path, data) => {
                items.push(("put", data).into_py(py));
                path
            }
        };
        items.push(("path", path).into_py(py));
        if let Ok(dic) = PyDict::from_sequence(py, items.into_py(py)) {
            dic.into_py(py)
        } else {
            panic!("Error creating py_yaml update, could not build dictionary.")
        }
    }
}

/// Delegate batched yaml updates to python ruamel.yaml library.
///
/// The only library I've found that preserves comments!
pub fn py_modify_yaml(src: String, updates: Vec<YamlUpdate>) -> Result<String, Zerr> {
    let bstr = Python::with_gil(|py| {
        let py_fn = py.import("zetch._yaml")?.getattr("modify_yaml")?;

        // Returns as a memoryview:
        py_fn
            .call1((src, updates.into_py(py)))?
            .extract::<Vec<u8>>()
    })
    .change_context(Zerr::InternalError)?;

    String::from_utf8(bstr).change_context(Zerr::InternalError)
}
