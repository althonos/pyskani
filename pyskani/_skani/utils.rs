use pyo3::buffer::PyBuffer;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedBytes;
use pyo3::pybacked::PyBackedStr;
use pyo3::types::PyByteArray;
use pyo3::types::PyBytes;
use pyo3::types::PyString;

/// Try to obtain a path from a Python object using `os.fsdecode`.
pub fn fsdecode<'py>(object: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyString>> {
    let py = object.py();
    py.import_bound(pyo3::intern!(py, "os"))?
        .call_method1(pyo3::intern!(py, "fsdecode"), (object,))
        .and_then(|x| x.extract())
}

pub enum Text {
    Bytes(PyBackedBytes),
    Str(PyBackedStr),
    Vec(Vec<u8>),
}

impl Text {
    pub fn new<'py>(object: Borrowed<'_, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(str) = object.downcast::<PyString>() {
            Ok(Text::Str(PyBackedStr::try_from(str.clone())?))
        } else if let Ok(bytes) = object.downcast::<PyBytes>() {
            Ok(Text::Bytes(PyBackedBytes::from(bytes.clone())))
        } else if let Ok(bytes) = object.downcast::<PyByteArray>() {
            Ok(Text::Bytes(PyBackedBytes::from(bytes.clone())))
        } else {
            let buffer = PyBuffer::get_bound(&object)?;
            let contents = buffer.to_vec(object.py())?;
            Ok(Text::Vec(contents))
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Text::Bytes(b) => b.as_ref(),
            Text::Str(b) => b.as_ref(),
            Text::Vec(b) => b.as_slice(),
        }
    }
}

/// Create a new error
pub fn poisoned_lock_error() -> PyErr {
    PyRuntimeError::new_err("Poisoned lock")
}
