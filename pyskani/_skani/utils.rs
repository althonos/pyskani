use std::borrow::Cow;

use pyo3::buffer::PyBuffer;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::types::PyString;

/// Try to obtain a path from a Python object using `os.fsdecode`.
pub fn fsdecode<'py>(object: &'py PyAny) -> PyResult<&PyString> {
    let py = object.py();
    py.import(pyo3::intern!(py, "os"))?
        .call_method1(pyo3::intern!(py, "fsdecode"), (object,))?
        .downcast::<PyString>()
        .map_err(PyErr::from)
}

/// Get the bytes from a `str` or any buffer implementor.
pub fn as_bytes<'py>(object: &'py PyAny) -> PyResult<Cow<'py, [u8]>> {
    let py = object.py();
    if let Ok(string) = object.downcast::<PyString>() {
        let contents = string.to_str()?.as_bytes();
        Ok(Cow::Borrowed(contents))
    } else if let Ok(bytes) = object.downcast::<PyBytes>() {
        let contents = bytes.as_bytes();
        Ok(Cow::Borrowed(contents))
    } else {
        let buffer = PyBuffer::get(object)?;
        let contents = buffer.to_vec(py)?;
        Ok(Cow::Owned(contents))
    }
}

/// Create a new error
pub fn poisoned_lock_error() -> PyErr {
    PyRuntimeError::new_err("Poisoned lock")
}
