extern crate pyo3;
extern crate pyskani;

use std::path::Path;
use std::str::FromStr;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyModule;
use pyo3::Python;

pub fn main() -> PyResult<()> {
    // get the relative path to the project folder
    let folder = Path::new(file!())
        .canonicalize()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("src");

    // spawn a Python interpreter
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        // insert the project folder in `sys.modules` so that
        // the main module can be imported by Python
        let sys = py.import("sys")?;
        sys.getattr("path")?
            .downcast::<PyList>()?
            .insert(0, folder.to_str().unwrap())?;

        // create a Python module from our rust code with debug symbols
        let module = PyModule::new(py, "pyskani._skani")?;
        pyskani::init(py, &module)?;
        sys.getattr("modules")?
            .downcast::<PyDict>()?
            .set_item("pyskani._skani", module)
            .unwrap();

        // run unittest on the tests
        let kwargs = PyDict::new(py);
        kwargs.set_item("exit", false).unwrap();
        kwargs.set_item("verbosity", 2u8).unwrap();
        py.import("unittest")
            .unwrap()
            .call_method("TestProgram", ("pyskani.tests",), Some(&kwargs))
            .map(|_| ())
    })
}
