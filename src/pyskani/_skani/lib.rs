extern crate bincode;
extern crate pyo3;
extern crate pyo3_built;
extern crate skani;

mod hit;
mod sketch;
mod utils;

#[allow(dead_code)]
mod build {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::path::Path;
use std::path::PathBuf;
use std::sync::RwLock;

use pyo3::exceptions::PyFileExistsError;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyOSError;
use pyo3::exceptions::PyRuntimeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use pyo3::types::PyType;
use pyo3_built::pyo3_built;
use skani::params::CommandParams;
use skani::params::SketchParams;
use skani::sketch_db::IndexEntry;

use self::hit::Hit;
use self::sketch::Sketch;

enum DatabaseStorage {
    Memory(HashMap<String, Sketch>),
    Folder(PathBuf),
    Consolidated(PathBuf, HashMap<String, skani::sketch_db::IndexEntry>),
}

impl DatabaseStorage {
    fn store(&mut self, sketch: Sketch, params: &SketchParams) -> PyResult<()> {
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
                    Err(err) => {
                        return if let Some(code) = err.raw_os_error() {
                            let msg = format!("Failed to create {}", sketch_path.display());
                            Err(PyOSError::new_err((code, msg)))
                        } else {
                            Err(PyRuntimeError::new_err(err.to_string()))
                        }
                    }
                };
                match bincode::serialize_into(writer, &(params, sketch.as_ref())) {
                    Ok(()) => Ok(()),
                    Err(err) => return Err(PyValueError::new_err(err.to_string())),
                }
            }
            DatabaseStorage::Consolidated(_, _) => unimplemented!(),
        }
    }

    fn load<'db>(&'db self, name: &str) -> PyResult<Cow<'db, Sketch>> {
        match self {
            DatabaseStorage::Memory(memory) => match memory.get(name) {
                Some(sketch) => Ok(Cow::Borrowed(sketch)),
                None => Err(PyKeyError::new_err(name.to_string())),
            },
            DatabaseStorage::Folder(path) => {
                let entry_path = path.join(format!("{}.sketch", name));
                let reader = match File::open(&entry_path).map(BufReader::new) {
                    Ok(reader) => reader,
                    Err(err) => {
                        return if let Some(code) = err.raw_os_error() {
                            let msg = format!("Failed to open {}", entry_path.display());
                            Err(PyOSError::new_err((code, msg)))
                        } else {
                            Err(PyRuntimeError::new_err(err.to_string()))
                        }
                    }
                };
                match bincode::deserialize_from::<_, (SketchParams, skani::types::Sketch)>(reader) {
                    Err(err) => Err(PyValueError::new_err(err.to_string())),
                    Ok((_, raw_sketch)) => Ok(Cow::Owned(Sketch::from(raw_sketch))),
                }
            }
            DatabaseStorage::Consolidated(path, index) => {
                let entry = match index.get(name) {
                    Some(entry) => entry,
                    None => return Err(PyKeyError::new_err(name.to_string())),
                };
                let mut reader = match File::open(&path.join("sketches.db")).map(BufReader::new) {
                    Ok(reader) => reader,
                    Err(err) => {
                        return if let Some(code) = err.raw_os_error() {
                            let msg = format!("Failed to open {}", path.display());
                            Err(PyOSError::new_err((code, msg)))
                        } else {
                            Err(PyRuntimeError::new_err(err.to_string()))
                        }
                    }
                };
                let mut buffer = vec![0; entry.length as usize];
                reader.seek_relative(entry.offset as i64)?;
                reader.read_exact(&mut buffer)?;
                match bincode::deserialize::<(SketchParams, skani::types::Sketch)>(&buffer) {
                    Err(err) => Err(PyValueError::new_err(err.to_string())),
                    Ok((_, raw_sketch)) => Ok(Cow::Owned(Sketch::from(raw_sketch))),
                }
            }
        }
    }
}

/// A database storing sketched genomes.
///
/// The database contains two different sketch collections with different
/// compression levels: marker sketches, which are heavily compressed, and
/// always kept in memory; and genome sketches, which take more memory, but
/// may be stored inside an external file.
///
#[pyclass(module = "pyskani._skani")]
pub struct Database {
    params: SketchParams,
    markers: RwLock<Vec<Sketch>>,
    sketches: RwLock<DatabaseStorage>,
}

impl Database {
    fn _sketch<'c, C>(&self, name: String, contigs: C, seed: bool) -> PyResult<Sketch>
    where
        C: IntoIterator<Item = &'c [u8]>,
    {
        // Adapted for a single genome from `fastx_to_sketches`
        // let mut is_valid = false;
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
                    skani::seeding::fmh_seeds(
                        &contig,
                        &self.params,
                        contig_count,
                        &mut sketch,
                        seed,
                    );
                }
                contig_count += 1;
                // is_valid = true;
            }
        }

        // NOTE(@althonos): This was removed in skani v0.3.0.
        // if is_valid && sketch.total_sequence_length > skani::params::REPET_KMER_THRESHOLD {
        //     sketch.repetitive_kmers =
        //         skani::seeding::get_repetitive_kmers(&sketch.kmer_seeds_k, sketch.c);
        // }

        Ok(Sketch::from(sketch))
    }

    fn _save_markers<P>(&self, path: P) -> PyResult<()>
    where
        P: AsRef<Path>,
    {
        let writer = match File::create(&path) {
            Ok(writer) => writer,
            Err(err) => {
                return if let Some(code) = err.raw_os_error() {
                    let msg = format!("Failed to create {}", path.as_ref().display());
                    Err(PyOSError::new_err((code, msg)))
                } else {
                    Err(PyRuntimeError::new_err(err.to_string()))
                }
            }
        };

        if let Ok(vec) = self.markers.read() {
            let refs = vec.iter().map(|s| s.as_ref()).collect::<Vec<_>>();
            match bincode::serialize_into(writer, &(&self.params, &refs)) {
                Ok(()) => Ok(()),
                Err(err) => Err(PyValueError::new_err(err.to_string())),
            }
        } else {
            Err(self::utils::poisoned_lock_error())
        }
    }

    fn _save_sketch<P>(&self, path: P, sketch: &Sketch) -> PyResult<()>
    where
        P: AsRef<Path>,
    {
        let writer = match File::create(path.as_ref()) {
            Ok(writer) => writer,
            Err(err) => {
                return if let Some(code) = err.raw_os_error() {
                    let msg = format!("Failed to create {}", path.as_ref().display());
                    Err(PyOSError::new_err((code, msg)))
                } else {
                    Err(PyRuntimeError::new_err(err.to_string()))
                }
            }
        };
        match bincode::serialize_into(writer, &(&self.params, sketch.as_ref())) {
            Ok(()) => Ok(()),
            Err(err) => return Err(PyValueError::new_err(err.to_string())),
        }
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
    ///     path (`str`, `bytes`, or `os.PathLike`): The path to the
    ///         folder containing the sketched references.
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
    pub fn load<'py>(cls: &Bound<'py, PyType>, path: &Bound<'py, PyAny>) -> PyResult<Self> {
        // load marker genes like in `Database.open`.
        let mut db = Self::open(cls, path)?;

        // obtain Unicode representation of path
        let path = self::utils::fsdecode(path)?;

        // load reference sketches
        let mut sketches = HashMap::new();
        for entry in std::fs::read_dir(path.to_str()?)
            .unwrap() // safe to unwrap, the `File::open` would have crashed otherwise
            .filter_map(Result::ok)
            .filter(|entry| {
                let filename = entry.file_name();
                let filepath: &Path = filename.as_ref();
                match filepath.extension() {
                    None => false,
                    Some(ext) => ext == "sketch",
                }
            })
        {
            let entry_path = entry.path();
            let reader = match File::open(&entry_path).map(BufReader::new) {
                Ok(reader) => reader,
                Err(err) => {
                    return if let Some(code) = err.raw_os_error() {
                        let msg = format!("Failed to open {}", entry_path.display());
                        Err(PyOSError::new_err((code, msg)))
                    } else {
                        Err(PyRuntimeError::new_err(err.to_string()))
                    }
                }
            };
            match bincode::deserialize_from::<_, (SketchParams, skani::types::Sketch)>(reader) {
                Err(err) => return Err(PyValueError::new_err(err.to_string())),
                Ok((params, raw_sketch)) => {
                    let name = Path::new(&raw_sketch.file_name)
                        .file_name()
                        .unwrap()
                        .to_os_string()
                        .into_string()
                        .unwrap(); // FIXME
                    sketches.insert(name, Sketch::from(raw_sketch));
                }
            };
        }
        db.sketches = DatabaseStorage::Memory(sketches).into();
        Ok(db)
    }

    /// Open a database from a folder containing sketches.
    ///
    /// The marker sketches will be loaded in memory, but the sketches will
    /// be loaded only when needed when querying. To speed-up querying by
    /// pre-fetching sketches, use `Database.load`.
    ///
    /// Arguments:
    ///     path (`str`, `bytes`, or `os.PathLike`): The path to the
    ///         folder containing the sketched references.
    ///
    /// Returns:
    ///     `~pyskani.Database`: A database with only markers loaded in memory.
    ///
    /// Raises:
    ///     `OSError`: When the files from the folder could not be opened.
    ///     `ValueError`: When the markers could not be deserialized.
    ///
    #[classmethod]
    #[allow(unused)]
    pub fn open<'py>(cls: &Bound<'py, PyType>, path: &Bound<'py, PyAny>) -> PyResult<Self> {
        macro_rules! open_or_fail {
            ($path:ident) => {
                match File::open(&$path).map(BufReader::new) {
                    Ok(reader) => reader,
                    Err(err) => {
                        return if let Some(code) = err.raw_os_error() {
                            let msg = format!("Failed to open {}", $path.display());
                            Err(PyOSError::new_err((code, msg)))
                        } else {
                            Err(PyRuntimeError::new_err(err.to_string()))
                        }
                    }
                }
            }
        }

        // obtain Unicode representation of path
        let decoded = self::utils::fsdecode(path)?;
        let fspath = Path::new(decoded.to_str()?);

        // load marker sketches
        let markers_path = fspath.join("markers.bin");
        let reader = open_or_fail!(markers_path);
        let (params, raw_markers) =
            match bincode::deserialize_from::<_, (SketchParams, Vec<skani::types::Sketch>)>(reader)
            {
                Ok((params, raw_markers)) => (params, raw_markers),
                Err(err) => return Err(PyValueError::new_err(err.to_string())),
            };
        let markers = raw_markers.into_iter().map(Sketch::from).collect();

        // identify whether this is a consolidated database or not
        let index_path = fspath.join("index.db");
        let sketches_path = fspath.join("sketches.db");
        if index_path.exists() && sketches_path.exists() {
            let reader = open_or_fail!(index_path);
            let index = match bincode::deserialize_from::<_, Vec<skani::sketch_db::IndexEntry>>(reader) {
                Ok(v) => v.into_iter().map(|entry| (entry.file_name.clone(), entry)).collect::<HashMap<_, _>>(),
                Err(err) => return Err(PyValueError::new_err(err.to_string())),
            };
            Ok(Self {
                params,
                markers: RwLock::new(markers),
                sketches: RwLock::new(DatabaseStorage::Consolidated(PathBuf::from(fspath), index)),
            })
        } else {
            // use the folder for storage
            Ok(Self {
                params,
                markers: RwLock::new(markers),
                sketches: RwLock::new(DatabaseStorage::Folder(PathBuf::from(fspath))),
            })
        }
    }

    /// Create a new database.
    ///
    /// Arguments:
    ///     path (`str`, `bytes`, `os.PathLike`, or `None`): The path of the
    ///         folder to use for storing the sketches. If `None` given, the
    ///         sketches are kept in memory. A new folder will be created if
    ///         it does not exist.
    ///
    /// Keyword Arguments:
    ///     compression (`int`): The compression factor for sketches. Memory
    ///         usage and runtime is inversely proportional to the given
    ///         value; lower values allows for ANI comparison of more distant
    ///         genomes
    ///     marker_compression (`int`): The compression factor for marker
    ///         k-mers. Markers are used for filtering. You want at least ~100
    ///         markers, so ``genome_size/marker_compression > 100`` is highly
    ///         recommended. Higher value is more time/memory efficient.
    ///
    /// Raises:
    ///     `OSError`: When a new folder could not be created.
    ///     `FileExistsError`: When the folder already contains sketches.
    ///
    #[new]
    #[pyo3(signature = (path=None, *, compression=125, marker_compression=1000, k=15))]
    pub fn __init__<'py>(
        path: Option<&Bound<'py, PyAny>>,
        compression: usize,
        marker_compression: usize,
        k: usize,
    ) -> PyResult<PyClassInitializer<Self>> {
        let storage = match path {
            None => DatabaseStorage::Memory(HashMap::new()),
            Some(folder) => {
                // obtain Unicode representation of path
                let folder = self::utils::fsdecode(folder)?;
                // create the folder if it does not exist
                let buf = PathBuf::from(folder.to_str()?);
                if !buf.exists() {
                    if let Err(err) = std::fs::create_dir_all(&buf) {
                        return if let Some(code) = err.raw_os_error() {
                            let msg = format!("Failed to create {}", buf.display());
                            Err(PyOSError::new_err((code, msg)))
                        } else {
                            Err(PyRuntimeError::new_err(err.to_string()))
                        };
                    }
                }
                // check the folder is not already in use
                if buf.join("markers.bin").exists() {
                    return Err(PyFileExistsError::new_err(
                        buf.join("markers.bin").display().to_string(),
                    ));
                }
                // use folder for storage
                DatabaseStorage::Folder(buf)
            }
        };
        let sketcher = Self {
            sketches: RwLock::new(storage),
            markers: Default::default(),
            params: SketchParams::new(marker_compression, compression, k, false, false),
        };
        Ok(sketcher.into())
    }

    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    #[allow(unused_variables)]
    pub fn __exit__<'py>(
        &self,
        exc_type: &Bound<'py, PyAny>,
        exc_value: &Bound<'py, PyAny>,
        traceback: &Bound<'py, PyAny>,
    ) -> PyResult<bool> {
        self.flush()?;
        Ok(false)
    }

    /// `pathlib.Path` or `None`: The path where sketches are stored.
    #[getter]
    pub fn get_path(&self, py: Python) -> PyResult<PyObject> {
        if let Ok(sketches) = self.sketches.read() {
            match *sketches {
                DatabaseStorage::Memory(_) => Ok(py.None()),
                DatabaseStorage::Folder(ref folder) | DatabaseStorage::Consolidated(ref folder, _) => {
                    let pathlib = py.import_bound(pyo3::intern!(py, "pathlib"))?;
                    let path = pathlib.call_method1(pyo3::intern!(py, "Path"), (folder,))?;
                    Ok(path.to_object(py))
                }
            }
        } else {
            Err(self::utils::poisoned_lock_error())
        }
    }

    /// Add a reference genome to the database.
    ///
    /// Arguments:
    ///     name (`str`): The name of the reference genome to add.
    ///     contigs (`str`, `bytes`, `bytearray` or `memoryview`): The contigs
    ///         of the reference genome.
    ///
    /// Keyword Arguments:
    ///     seed (`bool`): Compute seed positions while sketching
    ///         the query.
    ///
    #[pyo3(signature = (name, *contigs, seed=true))]
    pub fn sketch<'py>(
        &mut self,
        name: String,
        contigs: &Bound<'py, PyTuple>,
        seed: bool,
    ) -> PyResult<()> {
        // Get a view on the contigs
        let contents = contigs
            .into_iter()
            .map(|item| self::utils::Text::new(item.as_borrowed()))
            .collect::<PyResult<Vec<_>>>()?;
        let views = contents.iter().map(|text| text.as_bytes());

        // Release the GIL while sketching
        let py = contigs.py();
        let (sketch, marker) = py.allow_threads(|| {
            self._sketch(name, views, seed).map(|sketch| {
                let marker = skani::types::Sketch::get_markers_only(sketch.as_ref()).into();
                (sketch, marker)
            })
        })?;

        // Record sketches
        self.markers
            .write()
            .map_err(|_| self::utils::poisoned_lock_error())?
            .push(marker);
        self.sketches
            .write()
            .map_err(|_| self::utils::poisoned_lock_error())?
            .store(sketch, &self.params)?;
        Ok(())
    }

    /// Query the database with a genome.
    ///      
    /// Arguments:
    ///     name (`str`): The name of the query genome.
    ///     contigs (`str`, `bytes`, `bytearray` or `memoryview`): The contigs
    ///         of the query genome.
    ///
    /// Keyword Arguments:
    ///     seed (`bool`): Compute seed positions while sketching
    ///         the query.
    ///     learned_ani (`bool` or `None`): Use a regression model to
    ///         compute ANI, using a model trained on MAGs. Pass `True`
    ///         or `False` to force enabling or disabling the model,
    ///         respectively. By default, the regression model is enabled
    ///         when the sketch compression factor is >=70 and not running
    ///         in median mode.
    ///     median (`bool`): Estimate median identity instead of average
    ///         identity. Disabled by default. Equivalent to the ``--median``
    ///         flag of the CLI.
    ///     robust (`bool`): Estimate mean after trim off 10%/90% quantiles.
    ///         Disabled by default. Equivalent to the ``--robust`` flag
    ///         of the CLI.
    ///     cutoff (`float` or `None`): The cutoff to use to screen out pairs
    ///         with *approximately* lower identity, as computed with k-mer
    ///         sketching. Defaults to 0.8 for ANI and 0.6 for AAI.
    ///         Equivalent to the ``-s`` flag from the CLI.
    ///     faster_small (`bool`): Set to ``True`` to filter genomes with
    ///         less than 20 marker k-mers more aggressively. Disabled by
    ///         default. Equivalent to the ``--faster-small`` flag of the
    ///         CLI.
    ///
    /// Returns:
    ///     `list` of `~pyskani.Hit`: The hits found for the query.
    ///   
    #[pyo3(signature = (name, *contigs, seed=true, learned_ani=None, median=false, robust=false, cutoff=None, faster_small=false))]
    pub fn query<'py>(
        &self,
        name: String,
        contigs: Bound<'py, PyTuple>,
        seed: bool,
        learned_ani: Option<bool>,
        median: bool,
        robust: bool,
        cutoff: Option<f64>,
        faster_small: bool,
    ) -> PyResult<Vec<Hit>> {
        // Get a view on the contigs
        let contents = contigs
            .iter()
            .map(|item| self::utils::Text::new(item.as_borrowed()))
            .collect::<PyResult<Vec<_>>>()?;
        let views = contents.iter().map(|text| text.as_bytes());
        // Release the GIL while querying
        let py = contigs.py();
        py.allow_threads(move || {
            // Sketch query
            let query = self._sketch(name, views, seed)?;
            // Build command parameters
            let command_params = CommandParams {
                screen: false,
                screen_val: cutoff.unwrap_or(0.0),
                mode: skani::params::Mode::Search,
                out_file_name: Default::default(),
                ref_files: Default::default(),
                query_files: Default::default(),
                refs_are_sketch: true,
                queries_are_sketch: true,
                robust,
                median,
                sparse: false,
                full_matrix: false,
                max_results: 1_000_000_000,
                individual_contig_q: false,
                individual_contig_r: false,
                min_aligned_frac: skani::params::D_FRAC_COVER_CUTOFF.parse::<f64>().unwrap()
                    / 100.0,
                keep_refs: true,
                est_ci: Default::default(),
                learned_ani: learned_ani.unwrap_or(false),
                detailed_out: false,
                diagonal: false,
                distance: false,
                rescue_small: !faster_small,
                separate_sketches: false,
                both_min_aligned_frac: -0.01,
                short_header: false,
            };
            // Get cutoff for screening
            let screen_val = if command_params.screen_val != 0.0 {
                command_params.screen_val
            } else if self.params.use_aa {
                skani::params::SEARCH_AAI_CUTOFF_DEFAULT
            } else {
                skani::params::SEARCH_ANI_CUTOFF_DEFAULT
            };
            // Load regression model if requested / necessary
            let learned = learned_ani.unwrap_or_else(|| {
                skani::regression::use_learned_ani(self.params.c, false, false, median)
            });
            let model_opt = skani::regression::get_model(self.params.c, learned);
            // Search marker sketches first
            let mut shortlist = HashSet::new();
            for marker in self
                .markers
                .read()
                .map_err(|_| self::utils::poisoned_lock_error())?
                .iter()
            {
                if skani::screen::check_markers_quickly(
                    query.as_ref(),
                    marker.as_ref(),
                    screen_val,
                    command_params.rescue_small,
                ) {
                    let name = Path::new(&marker.as_ref().file_name)
                        .file_name()
                        .unwrap()
                        .to_os_string()
                        .into_string()
                        .unwrap();
                    shortlist.insert(name);
                }
            }
            // Search full sketches
            let mut hits = Vec::new();
            for name in shortlist.iter() {
                let guard = self
                    .sketches
                    .read()
                    .map_err(|_| self::utils::poisoned_lock_error())?;
                let reference = &*guard.load(&name)?;
                let map_params = skani::chain::map_params_from_sketch(
                    reference.as_ref(),
                    self.params.use_aa,
                    &command_params,
                    &model_opt,
                );
                let ani_res =
                    skani::chain::chain_seeds(reference.as_ref(), query.as_ref(), map_params);
                if ani_res.ani > 0.1 {
                    hits.push(Hit::from(ani_res));
                }
            }
            Ok(hits)
        })
    }

    /// Save the database to the given path.
    #[pyo3(signature = (path, overwrite=false))]
    pub fn save<'py>(&self, path: &Bound<'py, PyAny>, overwrite: bool) -> PyResult<()> {
        // obtain Unicode representation of path
        let path = self::utils::fsdecode(path)?;

        // Create folder if it doesn't exist
        let folder = Path::new(path.to_str()?);
        if !folder.exists() {
            if let Err(err) = std::fs::create_dir_all(folder) {
                return if let Some(code) = err.raw_os_error() {
                    let msg = format!("Failed to create {}", folder.display());
                    Err(PyOSError::new_err((code, msg)))
                } else {
                    Err(PyRuntimeError::new_err(err.to_string()))
                };
            }
        }

        // Serialize the markers
        let markers_path = folder.join("markers.bin");
        if !overwrite && markers_path.exists() {
            return Err(PyFileExistsError::new_err(
                markers_path.display().to_string(),
            ));
        }
        self._save_markers(markers_path)?;

        // FIXME: Handle new consolidated format.

        // Serialize the sketches
        for filename in self
            .markers
            .read()
            .map_err(|_| self::utils::poisoned_lock_error())?
            .iter()
            .map(|marker| Path::new(&marker.as_ref().file_name))
        {
            let name = filename
                .file_name()
                .unwrap()
                .to_os_string()
                .into_string()
                .unwrap(); // FIXME
            let sketch_path = folder.join(format!("{}.sketch", name));
            if !overwrite && sketch_path.exists() {
                return Err(PyFileExistsError::new_err(
                    sketch_path.display().to_string(),
                ));
            }
            self._save_sketch(
                sketch_path,
                &*self
                    .sketches
                    .read()
                    .map_err(|_| self::utils::poisoned_lock_error())?
                    .load(&name)?,
            )?;
        }
        Ok(())
    }

    /// Flush the database.
    ///
    /// This does nothing for a database loaded in memory. For a database
    /// stored in a folder, this will save the markers into a file named
    /// ``markers.bin``.
    ///
    pub fn flush(&self) -> PyResult<()> {
        if let Ok(sketches) = self.sketches.read() {
            match &*sketches {
                DatabaseStorage::Memory(_) => Ok(()),
                DatabaseStorage::Folder(folder) => {
                    let path = folder.join("markers.bin");
                    self._save_markers(&path)
                }
                DatabaseStorage::Consolidated(folder, _) => {
                    let path = folder.join("markers.bin");
                    self._save_markers(&path)
                    // FIXME: Is that it?
                }
            }
        } else {
            Err(self::utils::poisoned_lock_error())
        }
    }
}

/// A Python module for metagenomic sequence comparison with ``skani``.
///
#[pymodule]
#[pyo3(name = "_skani")]
pub fn init<'py>(py: Python<'py>, m: &Bound<'py, PyModule>) -> PyResult<()> {
    m.add("__package__", "pyskani")?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", env!("CARGO_PKG_AUTHORS").replace(':', "\n"))?;
    m.add("__build__", pyo3_built!(py, build))?;

    m.add_class::<Database>()?;
    m.add_class::<Hit>()?;
    m.add_class::<Sketch>()?;

    Ok(())
}
