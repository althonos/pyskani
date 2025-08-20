use std::io::BufReader;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use pyo3::buffer::PyBuffer;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedBytes;
use pyo3::pybacked::PyBackedStr;
use pyo3::types::PyByteArray;
use pyo3::types::PyBytes;
use pyo3::types::PyString;
use pyo3::exceptions::PyOSError;

/// Try to obtain a path from a Python object using `os.fsdecode`.
pub fn fsdecode<'py>(object: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyString>> {
    let py = object.py();
    py.import(pyo3::intern!(py, "os"))?
        .call_method1(pyo3::intern!(py, "fsdecode"), (object,))
        .and_then(|x| x.extract())
}

/// Try to open a path or fail with Python error handling.
pub fn buffered_open(path: &Path) -> PyResult<BufReader<File>> {
    match File::open(&path).map(BufReader::new) {
        Ok(reader) => Ok(reader),
        Err(err) => {
            return if let Some(code) = err.raw_os_error() {
                let msg = format!("Failed to open {}", path.display());
                Err(PyOSError::new_err((code, msg)))
            } else {
                Err(PyRuntimeError::new_err(err.to_string()))
            }
        }
    }
}

/// Try to create a file or fail with Python error handling.
pub fn buffered_create(path: &Path) -> PyResult<BufWriter<File>> {
    match File::create(&path).map(BufWriter::new) {
        Ok(writer) => Ok(writer),
        Err(err) => {
            return if let Some(code) = err.raw_os_error() {
                let msg = format!("Failed to create {}", path.display());
                Err(PyOSError::new_err((code, msg)))
            } else {
                Err(PyRuntimeError::new_err(err.to_string()))
            }
        }
    }
}

/// Try to append to a file or fail with Python error handling.
pub fn buffered_append(path: &Path) -> PyResult<BufWriter<File>> {
    match std::fs::OpenOptions::new().create(true).append(true).open(&path).map(BufWriter::new) {
        Ok(writer) => Ok(writer),
        Err(err) => {
            return if let Some(code) = err.raw_os_error() {
                let msg = format!("Failed to open {}", path.display());
                Err(PyOSError::new_err((code, msg)))
            } else {
                Err(PyRuntimeError::new_err(err.to_string()))
            }
        }
    }
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
            let buffer = PyBuffer::get(&object)?;
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

