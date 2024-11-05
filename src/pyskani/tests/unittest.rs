extern crate pyo3;
extern crate pyskani;

use std::path::Path;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyModule;
use pyo3::Python;

pub fn main() -> PyResult<()> {
    // get the relative path to the project folder
    let folder = Path::new(file!())
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
        let sys = py.import_bound("sys").unwrap();
        sys.getattr("path")
            .unwrap()
            .downcast::<PyList>()
            .unwrap()
            .insert(0, folder)
            .unwrap();

        // create a Python module from our rust code with debug symbols
        let module = PyModule::new_bound(py, "pyskani._skani").unwrap();
        pyskani::init(py, &module).unwrap();
        sys.getattr("modules")
            .unwrap()
            .downcast::<PyDict>()
            .unwrap()
            .set_item("pyskani._skani", module)
            .unwrap();

        // run unittest on the tests
        let kwargs = PyDict::new_bound(py);
        kwargs.set_item("exit", false).unwrap();
        kwargs.set_item("verbosity", 2u8).unwrap();
        py.import_bound("unittest")
            .unwrap()
            .call_method("TestProgram", ("pyskani.tests",), Some(&kwargs))
            .map(|_| ())
    })
}
