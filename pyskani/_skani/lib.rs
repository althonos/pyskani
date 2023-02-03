extern crate bincode;
extern crate pyo3;
extern crate rayon;
extern crate skani;

mod hit;
mod sketch;

use std::collections::HashSet;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::io::BufReader;

use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyValueError;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyOSError;
use pyo3::exceptions::PyUnicodeError;
use pyo3::prelude::*;
use pyo3::types::PyType;
use pyo3::types::PyTuple;
use pyo3::buffer::PyBuffer;
use skani::params::CommandParams;
use skani::params::SketchParams;

use self::hit::Hit;
use self::sketch::Sketch;

enum DatabaseStorage {
    Memory(HashMap<String, Sketch>),
    Folder(PathBuf),
}

impl DatabaseStorage {
    pub fn store(&mut self, sketch: Sketch, params: &SketchParams) -> PyResult<()> {
        match self {
            DatabaseStorage::Memory(memory) => {
                let name = sketch.as_ref().file_name.clone();
                memory.insert(name, sketch);
                Ok(())
            }
            DatabaseStorage::Folder(folder) => {
                let sketch_path = folder.join(format!("{}.sketch", &sketch.as_ref().file_name));
                let writer = match File::create(&sketch_path) {
                    Ok(writer) => writer,
                    Err(err) => return if let Some(code) = err.raw_os_error() {
                        let msg = format!("Failed to create {}", sketch_path.display());
                        Err(PyOSError::new_err((code, msg)))
                    } else {
                        Err(PyRuntimeError::new_err(err.to_string()))
                    }
                };
                match bincode::serialize_into(writer, &(params, sketch.as_ref())) {
                    Ok(()) => Ok(()),
                    Err(err) => return Err(PyValueError::new_err(err.to_string())),
                }
            }
        }
    }

    pub fn load(&self, name: &str) -> PyResult<&Sketch> {
        match self {
            DatabaseStorage::Memory(memory) => {
                memory.get(name).ok_or_else(|| PyKeyError::new_err(name.to_string()))
            }
            DatabaseStorage::Folder(path) => {
                unimplemented!()
            }
        }
    }
}

#[pyclass]
pub struct Database {
    params: SketchParams,
    markers: Vec<Sketch>,
    sketches: DatabaseStorage,
}

impl Database {
    fn _sketch<'c, C>(
        &self,
        name: String,
        contigs: C,
        seed: bool,
    ) -> PyResult<Sketch>
    where
        C: IntoIterator<Item = &'c [u8]>,
    {
        // Adapted for a single genome from `fastx_to_sketches`
        let mut is_valid = false;
        let mut contig_count = 0;
        let mut sketch = skani::types::Sketch::new(
            self.params.marker_c,
            self.params.c,
            self.params.k,
            name.clone(), // file name
            self.params.use_aa,
        );

        for (i, contig) in contigs.into_iter().enumerate() {
            if contig.len() >= skani::params::MIN_LENGTH_CONTIG {
                sketch.contigs.push(format!("{}_{}", &name, i));
                sketch
                    .contig_lengths
                    .push(contig.len() as skani::types::GnPosition);
                sketch.total_sequence_length += contig.len();
                if self.params.use_aa {
                    unimplemented!()
                } else {
                    skani::seeding::fmh_seeds(&contig, &self.params, contig_count, &mut sketch, seed);
                }
                contig_count += 1;
                is_valid = true;
            }
        }

        if is_valid && sketch.total_sequence_length > 20_000_000 {
            sketch.repetitive_kmers = skani::seeding::get_repetitive_kmers(&sketch.kmer_seeds_k);
        }

        Ok(Sketch::from(sketch))
    }
}

#[pymethods]
impl Database {
    /// Load a database from a folder containing sketches.
    ///
    /// The sketches will be loaded in memory to speed-up querying. To
    /// reduce memory consumption and load sketches lazily from the folder, 
    /// use `Database.open`.
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
    #[allow(unused)]
    pub fn load(cls: &PyType, path: &str) -> PyResult<Self> {
        // load marker genes like in `Database.open`.
        let mut db = Self::open(cls, path)?;

        // load reference sketches
        let mut sketches = HashMap::new();
        for entry in std::fs::read_dir(path)
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
                    let msg = format!("Failed to open {}", entry_path.display());
                    Err(PyOSError::new_err((code, msg)))
                } else {
                    Err(PyRuntimeError::new_err(err.to_string()))
                }
            };
            match bincode::deserialize_from::<_, (SketchParams, skani::types::Sketch)>(reader) {
                Err(err) => return Err(PyValueError::new_err(err.to_string())),
                Ok((params, raw_sketch)) => {
                    let name = raw_sketch.file_name.clone();
                    sketches.insert(name, Sketch::from(raw_sketch));
                },
            };
        }
        db.sketches = DatabaseStorage::Memory(sketches);
        Ok(db)
    }

    // TODO: Change `path` to support `os.PathLike` objects as well.
    
    /// Open a database from a folder containing sketches.
    ///
    /// The marker sketches will be loaded in memory, but the sketches will
    /// be loaded only when needed when querying. To speed-up querying by
    /// pre-fetching sketches, use `Database.load`.
    #[classmethod]
    #[allow(unused)]
    pub fn open(cls: &PyType, path: &str) -> PyResult<Self> {
        // load marker sketches
        let markers_path = std::path::Path::new(path).join("markers.bin");
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

        // use the folder for storage
        Ok(Self {
            params,
            markers,
            sketches: DatabaseStorage::Folder(PathBuf::from(path)),
        })
    }

    #[new]
    #[pyo3(signature = (path=None, *, marker_c=1000, c=125, k=15, amino_acid=false))]
    pub fn __init__(
        path: Option<&str>,
        marker_c: usize,
        c: usize,
        k: usize,
        amino_acid: bool,
    ) -> PyResult<PyClassInitializer<Self>> {
        let storage = match path {
            None => DatabaseStorage::Memory(HashMap::new()),
            Some(folder) => DatabaseStorage::Folder(PathBuf::from(folder)),
        };
        let sketcher = Self {
            sketches: storage,
            markers: Vec::new(),
            params: SketchParams::new(marker_c, c, k, false, amino_acid),
        };
        Ok(sketcher.into())
    }

    /// `pathlib.Path` or `None`: The path where sketches are stored.
    #[getter]
    pub fn get_path(&self, py: Python) -> PyResult<PyObject> {
        match &self.sketches {
            DatabaseStorage::Memory(_) => Ok(py.None()),
            DatabaseStorage::Folder(folder) => {
                let pathlib = py.import(pyo3::intern!(py, "pathlib"))?;
                let path = pathlib.call_method1(pyo3::intern!(py, "Path"), (folder,))?;
                Ok(path.to_object(py))
            }
        }
    }

    /// Add a reference genome to the database.
    ///
    /// This method is a shortcut for `Database.add_draft` when a genome is
    /// complete (i.e. only contains a single contig).
    ///
    /// Arguments:
    ///     name (`object`): The name of the reference genome to add.
    ///     contigs (`bytes`, `bytearray` or `memoryview`): The contigs of the 
    ///         reference genome. 
    /// 
    #[pyo3(signature = (name, *contigs, seed=true))]
    pub fn sketch(&mut self, name: String, contigs: &PyTuple, seed: bool) -> PyResult<()> {
        let py = contigs.py();
        let buffers = contigs
            .into_iter()
            .map(|item| PyBuffer::get(item))
            .collect::<PyResult<Vec<PyBuffer<u8>>>>()?;
        let bytes = buffers
            .into_iter()
            .map(|buffer| buffer.to_vec(py))
            .collect::<PyResult< Vec< Vec<u8> >>>()?;
        let views = bytes
            .iter()
            .map(|x| x.as_ref());

        let sketch = self._sketch(name, views, seed)?;
        self.markers.push(skani::types::Sketch::get_markers_only(sketch.as_ref()).into());
        self.sketches.store(sketch, &self.params);
        Ok(())
    }

    /// Query the database with a genome.
    ///      
    /// Arguments:
    ///     name (`str`): The name of the query genome.
    ///     contigs (`bytes`, `bytearray` or `memoryview`): The contigs of the 
    ///         query genome.  
    ///   
    ///   Returns:
    ///       `list` of `~pyskani.Hit`: The hits found for the query.
    ///   
    #[pyo3(signature = (name, *contigs, seed=true))]
    pub fn query(&self, name: String, contigs: &PyTuple, seed: bool) -> PyResult<Vec<Hit>> {
        let py = contigs.py();
        let buffers = contigs
            .into_iter()
            .map(|item| PyBuffer::get(item))
            .collect::<PyResult<Vec<PyBuffer<u8>>>>()?;
        let bytes = buffers
            .into_iter()
            .map(|buffer| buffer.to_vec(py))
            .collect::<PyResult< Vec< Vec<u8> >>>()?;
        let views = bytes
            .iter()
            .map(|x| x.as_ref());

        let query = self._sketch(name, views, seed)?;
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

        let mut shortlist = HashSet::new();
        for marker in self.markers.iter() {
            if skani::chain::check_markers_quickly(
                query.as_ref(),
                marker.as_ref(),
                command_params.screen_val,
            ) {
                shortlist.insert(&marker.as_ref().file_name);
            }
        }

        let mut hits = Vec::new();
        for name in shortlist.iter() {
            let reference = &self.sketches.load(name)?;
            let map_params = skani::chain::map_params_from_sketch(
                reference.as_ref(),
                self.params.use_aa,
                &command_params,
            );
            let ani_res = skani::chain::chain_seeds(reference.as_ref(), query.as_ref(), map_params);
            if ani_res.ani > 0.5 {
                hits.push(Hit::from(ani_res));
            }
        }

        Ok(hits)
    }
}

#[pymodule]
#[pyo3(name = "_skani")]
pub fn init(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__package__", "pyskani")?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", env!("CARGO_PKG_AUTHORS").replace(':', "\n"))?;
    // m.add("__build__", pyo3_built!(py, built))?;

    m.add_class::<Database>()?;
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
