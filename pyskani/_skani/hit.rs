use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyString;
use skani::types::AniEstResult;

/// A single hit found when querying a `~pyskani.Database` with a genome.
///
/// Attributes:
///     identity (`float`): The estimated Average Nucleotide Identity
///         between the query and reference genomes.
///     query_name (`str`): The name of the query genome.
///     reference_name (`str`): The name of the reference genome.
///     query_fraction (`float`): The fraction of the query sequence
///         covered by the alignment.
///     reference_fraction (`float`): The fraction of the reference
///         sequence covered by the alignment.
///
#[pyclass(module = "pyskani._skani")]
pub struct Hit {
    result: AniEstResult,
}

#[pymethods]
impl Hit {
    /// Create a new `Hit` object.
    #[new]
    pub fn __init__(
        identity: f32,
        query_name: &str,
        query_fraction: f32,
        reference_name: &str,
        reference_fraction: f32,
    ) -> PyResult<PyClassInitializer<Self>> {
        if identity < 0.0 || identity > 1.0 {
            let msg = format!("Invalid value for `identity`: {}", identity);
            return Err(PyValueError::new_err(msg));
        }
        if query_fraction < 0.0 || query_fraction > 1.0 {
            let msg = format!("Invalid value for `query_fraction`: {}", query_fraction);
            return Err(PyValueError::new_err(msg));
        }
        if reference_fraction < 0.0 || reference_fraction > 1.0 {
            let msg = format!(
                "Invalid value for `reference_fraction`: {}",
                reference_fraction
            );
            return Err(PyValueError::new_err(msg));
        }

        let mut result = AniEstResult::default();
        result.ani = identity;
        result.align_fraction_query = query_fraction;
        result.align_fraction_ref = reference_fraction;
        result.query_file = query_name.to_string();
        result.ref_file = reference_name.to_string();

        Ok(Hit::from(result).into())
    }

    /// Return ``repr(self)``.
    pub fn __repr__<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        let template = PyString::new(py, "Hit(identity={!r}, query_name={!r}, query_fraction={!r}, reference_name={!r}, reference_fraction={!r})");
        let fmt = template.call_method1(
            pyo3::intern!(py, "format"),
            (
                self.get_identity(),
                self.get_query_name(),
                self.get_query_fraction(),
                self.get_reference_name(),
                self.get_reference_fraction(),
            ),
        );
        fmt
    }

    /// `float`: The average nucleotide identity between the two genomes.
    #[getter]
    pub fn get_identity(&self) -> f32 {
        self.result.ani
    }

    /// `str`: The name of the query genome.
    #[getter]
    pub fn get_query_name(&self) -> &str {
        self.result.query_file.as_str()
    }

    /// `float`: The fraction of the query genome covered by the alignment.
    #[getter]
    pub fn get_query_fraction(&self) -> f32 {
        self.result.align_fraction_query
    }

    /// `str`: The name of the reference genome.
    #[getter]
    pub fn get_reference_name(&self) -> &str {
        self.result.ref_file.as_str()
    }

    /// `float`: The fraction of the reference genome covered by the alignment.
    #[getter]
    pub fn get_reference_fraction(&self) -> f32 {
        self.result.align_fraction_ref
    }
}

impl AsRef<AniEstResult> for Hit {
    fn as_ref(&self) -> &AniEstResult {
        &self.result
    }
}

impl AsMut<AniEstResult> for Hit {
    fn as_mut(&mut self) -> &mut AniEstResult {
        &mut self.result
    }
}

impl From<AniEstResult> for Hit {
    fn from(result: AniEstResult) -> Self {
        Self { result }
    }
}
