extern crate bincode;
extern crate pyo3;
extern crate rayon;
extern crate skani;

use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Mutex;

use crate::rayon::iter::ParallelIterator;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyValueError;
use pyo3::exceptions::PyOSError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyString;
use pyo3::types::PyType;
use rayon::iter::IntoParallelIterator;
use skani::params::CommandParams;
use skani::params::MapParams;
use skani::params::SketchParams;

fn sketch_drafts<'c, C>(
    params: &SketchParams,
    name: String,
    contigs: C,
    seed: bool,
) -> PyResult<Sketch>
where
    C: IntoIterator<Item = &'c &'c [u8]>,
{
    // Adapted for a single genome from `fastx_to_sketches`
    let mut is_valid = false;
    let mut contig_count = 0;
    let mut sketch = skani::types::Sketch::new(
        params.marker_c,
        params.c,
        params.k,
        name.clone(), // file name
        params.use_aa,
    );

    for (i, contig) in contigs.into_iter().enumerate() {
        if contig.len() >= skani::params::MIN_LENGTH_CONTIG {
            sketch.contigs.push(format!("{}_{}", &name, i));
            sketch
                .contig_lengths
                .push(contig.len() as skani::types::GnPosition);
            sketch.total_sequence_length += contig.len();
            if params.use_aa {
                unimplemented!()
            } else {
                skani::seeding::fmh_seeds(&contig, params, contig_count, &mut sketch, seed);
            }
            contig_count += 1;
            is_valid = true;
        }
    }

    if is_valid && sketch.total_sequence_length > 20_000_000 {
        sketch.repetitive_kmers = skani::seeding::get_repetitive_kmers(&sketch.kmer_seeds_k);
    }

    Ok(Sketch { sketch })
}

/// A single hit found when querying a `~pyskani.Database` with a genome.
///
/// Attributes:
///     identity (`float`): 
///     query_name (`str`): The name of the query genome.
///     reference_name (`str`): The name of the reference genome.
///     query_fraction (`float`): The fraction of the query sequence
///         covered by the alignment.
///     reference_fraction (`float`): The fraction of the reference 
///         sequence covered by the alignment.
///
#[pyclass]
pub struct Hit {
    result: skani::types::AniEstResult,
}

#[pymethods]
impl Hit {
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

    pub fn __repr__(&self, py: Python) -> PyResult<PyObject> {
        Python::with_gil(|py| {
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
            fmt.map(|r| r.to_object(py))
        })
    }
}

impl From<skani::types::AniEstResult> for Hit {
    fn from(result: skani::types::AniEstResult) -> Self {
        Self { result }
    }
}

#[pyclass]
pub struct Sketch {
    sketch: skani::types::Sketch,
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

#[pyclass]
pub struct Database {
    params: SketchParams,
    sketches: Vec<Sketch>,
    markers: Vec<Sketch>,
}

#[pymethods]
impl Database {
    // TODO: Change `path` to support `os.PathLike` objects as well.
    /// Load a database from a folder containing sketches.
    ///
    /// Arguments:
    ///     path (`str`): The path to the folder containing the sketched 
    ///         sequences. 
    ///
    /// Returns:
    ///     `~pyskani.Database`: A database with all sketches loaded in memory.
    ///
    /// Raises:
    ///     `OSError`: When the files from the folder could not be opened.
    ///     `ValueError`: When the sketches could not be deserialized.
    ///
    #[classmethod]
    fn load(cls: &PyType, path: &str) -> PyResult<Self> {
        let folder = std::path::Path::new(path);

        // load marker sketches
        let markers_path = folder.join("markers.bin");
        let reader = match File::open(&markers_path).map(BufReader::new) {
            Ok(reader) => reader,
            Err(err) => return if let Some(code) = err.raw_os_error() {
                let msg = format!("Failed to open {}", markers_path.display());
                Err(PyOSError::new_err((code, msg)))
            } else {
                Err(PyRuntimeError::new_err(err.to_string()))
            }
        };
        let (params, raw_markers) =
            match bincode::deserialize_from::<_, (SketchParams, Vec<skani::types::Sketch>)>(reader)
            {
                Ok((params, raw_markers)) => (params, raw_markers),
                Err(err) => return Err(PyValueError::new_err(err.to_string())),
            };
        let markers = raw_markers.into_iter().map(Sketch::from).collect();

        // load reference sketches
        let mut sketches = Vec::new();
        for entry in std::fs::read_dir(folder)
            .unwrap() // safe to unwrap, the `File::open` would have crashed otherwise
            .filter_map(Result::ok)
            .filter(|entry| {
                let filename = entry.file_name();
                let filepath: &std::path::Path = filename.as_ref();
                match filepath.extension() {
                    None => false,
                    Some(ext) => ext == "sketch",
                }
            })
        {
            let entry_path = entry.path();
            let reader = match File::open(&entry_path).map(BufReader::new) {
                Ok(reader) => reader,
                Err(err) => return if let Some(code) = err.raw_os_error() {
                    let msg = format!("Failed to open {}", markers_path.display());
                    Err(PyOSError::new_err((code, msg)))
                } else {
                    Err(PyRuntimeError::new_err(err.to_string()))
                }
            };
            match bincode::deserialize_from::<_, (SketchParams, skani::types::Sketch)>(reader) {
                Ok((params, raw_sketch)) => sketches.push(Sketch::from(raw_sketch)),
                Err(err) => return Err(PyValueError::new_err(err.to_string())),
            };
        }

        Ok(Self {
            params,
            markers,
            sketches,
        })
    }


    #[new]
    #[pyo3(signature = (marker_c=1000, c=125, k=15, amino_acid=false))]
    pub fn __init__(
        marker_c: usize,
        c: usize,
        k: usize,
        amino_acid: bool,
    ) -> PyResult<PyClassInitializer<Self>> {
        let sketcher = Self {
            sketches: Vec::new(),
            markers: Vec::new(),
            params: SketchParams::new(marker_c, c, k, false, amino_acid),
        };
        Ok(sketcher.into())
    }

    /// Add a reference genome to the database.
    ///
    /// This method is a shortcut for `Database.add_draft` when a genome is
    /// complete (i.e. only contains a single contig).
    ///
    /// Arguments:
    ///     name (`object`): The name of the reference genome to add.
    ///     sequence (`bytes`): The sequence of the reference genome. 
    /// 
    #[pyo3(signature = (name, sequence, seed=true))]
    pub fn add_genome(&mut self, name: String, sequence: &[u8], seed: bool) -> PyResult<()> {
        self.add_draft(name, vec![sequence], seed)
    }

    /// Add a reference draft genome to the database.
    ///
    /// Using this method is fine even when the genome has a single contig,
    /// although `Database.add_genome` is easier to use in that case.
    ///
    /// Arguments:
    ///     name (`object`): The name of the reference genome to add.
    ///     contigs (`list` of `bytes`): The contigs of the reference genome. 
    /// 
    #[pyo3(signature = (name, contigs, seed=true))]
    pub fn add_draft(&mut self, name: String, contigs: Vec<&[u8]>, seed: bool) -> PyResult<()> {
        let sketch = sketch_drafts(&self.params, name, &contigs, seed)?;
        self.markers.push(skani::types::Sketch::get_markers_only(&sketch.sketch).into());
        self.sketches.push(sketch);
        Ok(())
    }

    /// Query the database with a complete genome.
    ///      
    /// Arguments:
    ///     name (`str`): The name of the query genome.
    ///     sequence (`bytes`): The sequence of the query genome.
    ///   
    ///   Returns:
    ///       `list` of `~pyskani.Hit`: The hits found for the query.
    ///   
    #[pyo3(signature = (name, sequence, seed=true))]
    pub fn query_genome<'s>(
        &self,
        name: String,
        sequence: &[u8],
        seed: bool,
    ) -> PyResult<Vec<Hit>> {
        self.query_draft(name, vec![sequence], seed)
    }

    /// Query the database with a draft genome.
    ///      
    /// Arguments:
    ///     name (`str`): The name of the query genome.
    ///     contigs (`list` of `bytes`): The sequence of the contigs from
    ///         the query genome.
    ///   
    ///   Returns:
    ///       `list` of `~pyskani.Hit`: The hits found for the query.
    ///   
    #[pyo3(signature = (name, contigs, seed=true))]
    pub fn query_draft<'s>(
        &self,
        name: String,
        contigs: Vec<&[u8]>,
        seed: bool,
    ) -> PyResult<Vec<Hit>> {
        let query = sketch_drafts(&self.params, name, &contigs, seed)?;
        let command_params = CommandParams {
            screen: false,
            mode: skani::params::Mode::Search,
            screen_val: skani::params::SEARCH_ANI_CUTOFF_DEFAULT,
            out_file_name: Default::default(),
            ref_files: Default::default(),
            query_files: Default::default(),
            refs_are_sketch: true,
            queries_are_sketch: true,
            robust: false,
            median: false,
            sparse: false,
            full_matrix: false,
            max_results: 1_000_000_000,
            individual_contig_q: false,
            individual_contig_r: false,
            min_aligned_frac: skani::params::D_FRAC_COVER_CUTOFF.parse::<f64>().unwrap() / 100.0,
            keep_refs: true,
            est_ci: Default::default(),
        };

        let mut ref_to_try = HashSet::new();
        for (index, marker) in self.sketches.iter().enumerate() {
            if skani::chain::check_markers_quickly(
                &query.sketch,
                &marker.sketch,
                command_params.screen_val,
            ) {
                ref_to_try.insert(index);
            }
        }

        let mut hits = Vec::new();
        for &index in ref_to_try.iter() {
            let reference = &self.sketches[index];
            let map_params = skani::chain::map_params_from_sketch(
                &reference.sketch,
                self.params.use_aa,
                &command_params,
            );
            let ani_res = skani::chain::chain_seeds(&reference.sketch, &query.sketch, map_params);
            if ani_res.ani > 0.5 {
                hits.push(Hit::from(ani_res));
            }
        }

        Ok(hits)
    }
}


#[pymodule]
#[pyo3(name = "_skani")]
pub fn init(py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__package__", "pyskani")?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", env!("CARGO_PKG_AUTHORS").replace(':', "\n"))?;
    // m.add("__build__", pyo3_built!(py, built))?;

    m.add_class::<Database>()?;
    m.add_class::<Sketch>()?;

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn it_works() {
//         let result = 2 + 2;
//         assert_eq!(result, 4);
//     }
// }
