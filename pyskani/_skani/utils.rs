use pyo3::prelude::*;
use pyo3::types::PyString;

/// Try to obtain a path from a Python object using `os.fsdecode`.
pub fn fsdecode<'py>(object: &'py PyAny) -> PyResult<&PyString> {
    let py = object.py();
    py.import(pyo3::intern!(py, "os"))?
        .call_method1(pyo3::intern!(py, "fsdecode"), (object,))?
        .downcast::<PyString>()
        .map_err(PyErr::from)
}