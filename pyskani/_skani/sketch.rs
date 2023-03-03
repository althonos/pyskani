use pyo3::prelude::*;

/// A sketched genome.
#[pyclass(module = "pyskani._skani")]
#[derive(Clone)]
pub struct Sketch {
    sketch: skani::types::Sketch,
}

impl AsRef<skani::types::Sketch> for Sketch {
    fn as_ref(&self) -> &skani::types::Sketch {
        &self.sketch
    }
}

#[pymethods]
impl Sketch {
    #[getter]
    fn get_name(&self) -> &str {
        &self.sketch.file_name
    }

    #[getter]
    fn get_c(&self) -> usize {
        self.sketch.c
    }

    #[getter]
    fn get_amino_acid(&self) -> bool {
        self.sketch.amino_acid
    }
}

impl From<skani::types::Sketch> for Sketch {
    fn from(sketch: skani::types::Sketch) -> Self {
        Self { sketch }
    }
}
