use pyo3::{
    prelude::*,
    types::{PyDict, PyTuple},
};

use crate::prelude::*;

/// An individual accessor for mapping or array in a yaml file.
#[derive(Clone)]
pub enum YamlLoc {
    Key(String),
    Index(usize),
}

impl<'py> IntoPyObject<'py> for YamlLoc {
    type Target = PyAny; // the Python type
    type Output = Bound<'py, Self::Target>; // in most cases this will be `Bound`
    type Error = std::convert::Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(match self {
            YamlLoc::Index(index) => index.into_pyobject(py)?.into_any(),
            YamlLoc::Key(key) => key.into_pyobject(py)?.into_any(),
        })
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

impl<'py> IntoPyObject<'py> for YamlUpdate {
    type Target = PyDict;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let mut items: Vec<Bound<PyTuple>> = vec![];
        let path = match self {
            YamlUpdate::Delete(path) => path,
            YamlUpdate::Put(path, data) => {
                items.push(("put", data).into_pyobject(py)?);
                path
            }
        };
        items.push(("path", path).into_pyobject(py)?);
        if let Ok(dic) = PyDict::from_sequence(&items.into_pyobject(py)?) {
            Ok(dic.into_pyobject(py)?)
        } else {
            panic!("Error creating py_yaml update, could not build dictionary.")
        }
    }
}

/// Delegate batched yaml updates to python ruamel.yaml library.
///
/// The only library I've found that preserves comments!
pub fn py_modify_yaml(src: String, updates: Vec<YamlUpdate>) -> Result<String, Report<Zerr>> {
    let bstr = Python::with_gil(|py| {
        let py_fn = py.import("zetch._yaml")?.getattr("modify_yaml")?;

        // Returns as a memoryview:
        py_fn
            .call1((src, updates.into_pyobject(py)?))?
            .extract::<Vec<u8>>()
    })
    .change_context(Zerr::InternalError)?;

    String::from_utf8(bstr).change_context(Zerr::InternalError)
}
