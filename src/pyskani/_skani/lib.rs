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
use std::io::Read;
use std::io::BufRead;
use std::io::Seek;
use std::io::Write;
use std::ops::Index;
use std::path::Path;
use std::path::PathBuf;
use std::sync::RwLock;
use std::unimplemented;

use pyo3::exceptions::PyFileNotFoundError;
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
use skani::types::SketchFormat;

use self::hit::Hit;
use self::sketch::Sketch;

#[derive(Debug, Clone)]
enum FormatVersion {
    /// Legacy format, as implemented in `skani <0.4`; sketch entries are 
    /// written immediately after each other.
    Legacy,
    /// Format introduced in `skani >=0.4`; sketch entries each contain a
    /// small header with a magic number and a format version number prior
    /// to each entry.
    V2(u32),
}

impl FormatVersion {
    pub fn sketches_filename(&self) -> &'static str {
        match self {
            Self::Legacy => "sketches.db",
            Self::V2(_) => "sketches.db2",
        }
    }

    pub fn markers_filename(&self) -> &'static str {
        match self {
            Self::Legacy => "markers.bin",
            Self::V2(_) => "markers.bin2",
        }
    }

    pub fn index_filename(&self) -> &'static str {
        match self {
            Self::Legacy => "index.db",
            Self::V2(_) => "index.db2",
        }
    }
}


enum DatabaseStorage {
    Memory(HashMap<String, Sketch>),
    Folder(PathBuf),
    Consolidated(PathBuf, HashMap<String, IndexEntry>, FormatVersion),
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
                let writer = utils::buffered_create(&sketch_path)?;
                let legacy_sketch: self::sketch::LegacySketch = sketch.into();
                match bincode::serialize_into(writer, &(params, legacy_sketch)) {
                    Ok(()) => Ok(()),
                    Err(err) => return Err(PyValueError::new_err(err.to_string())),
                }
            }
            DatabaseStorage::Consolidated(folder, index, format_version) => {
                // ensure no duplicate names otherwise the hashmap is invalid
                let name = sketch.as_ref().file_name.clone();
                if index.contains_key(&name) {
                    return Err(PyValueError::new_err(format!(
                        "duplicate name in sketches: {:?}",
                        name
                    )));
                }
                // serialize the sketches to a temporary buffer using either
                // a legacy sketch (for legacy format) or a native sketch 
                // prefixed with the sketch header
                let buffer = match format_version {
                    FormatVersion::Legacy => {
                        let legacy_sketch: self::sketch::LegacySketch = sketch.into();
                        match bincode::serialize(&(params, legacy_sketch)) {
                            Ok(buffer) => buffer,
                            Err(err) => return Err(PyValueError::new_err(err.to_string())),
                        }
                    }
                    FormatVersion::V2(_) => {
                        let mut v = Vec::new();
                        skani::types::write_sketch_header(&mut v)?;
                        match bincode::serialize_into(&mut v, &(params, sketch.as_ref())) {
                            Ok(_) => v,
                            Err(err) => return Err(PyValueError::new_err(err.to_string())),
                        }
                    }
                };
                // open the sketches file and create an index entry
                let mut writer = utils::buffered_append(&folder.join(format_version.sketches_filename()))?;
                let index_entry = IndexEntry {
                    file_name: name.clone(),
                    offset: writer.stream_position()?,
                    length: buffer.len() as u64,
                };
                // write the serialized sketches and the record the index entry
                writer.write_all(&buffer)?;
                index.insert(name, index_entry);
                Ok(())
            }
        }
    }

    fn load<'db>(&'db self, name: &str) -> PyResult<Cow<'db, Sketch>> {
        match self {
            DatabaseStorage::Memory(memory) => match memory.get(name) {
                Some(sketch) => Ok(Cow::Borrowed(sketch)),
                None => Err(PyKeyError::new_err(name.to_string())),
            }
            DatabaseStorage::Folder(path) => {
                let entry_path = path.join(format!("{}.sketch", name));
                let reader = utils::buffered_open(&entry_path)?;
                match bincode::deserialize_from::<_, (SketchParams, skani::types::LegacySketch)>(reader) {
                    Err(err) => Err(PyValueError::new_err(err.to_string())),
                    Ok((_, raw_sketch)) => Ok(Cow::Owned(Sketch::from(raw_sketch))),
                }
            }
            DatabaseStorage::Consolidated(path, index, format_version) => {
                // Load entry location from index
                let entry = match index.get(name) {
                    Some(entry) => entry,
                    None => return Err(PyKeyError::new_err(name.to_string())),
                };
                // Filename depends on the database version
                let filename = format_version.sketches_filename();
                // Extract data for entry
                let mut reader = utils::buffered_open(&path.join(filename))?;
                let mut buffer = vec![0; entry.length as usize];
                reader.seek(std::io::SeekFrom::Start(entry.offset))?;
                reader.read_exact(&mut buffer)?;
                // Method for loading data depends on the detected sketch format
                match skani::types::detect_sketch_format(&buffer) {
                    SketchFormat::Native => {
                        match bincode::deserialize::<(SketchParams, skani::types::Sketch)>(&buffer[skani::types::SKETCH_HEADER_LEN..]) {
                            Err(err) => Err(PyValueError::new_err(err.to_string())),
                            Ok((_, raw_sketch)) => Ok(Cow::Owned(Sketch::from(raw_sketch))),
                        }
                    }
                    SketchFormat::Legacy => {
                        match bincode::deserialize::<(SketchParams, skani::types::LegacySketch)>(&buffer) {
                            Err(err) => Err(PyValueError::new_err(err.to_string())),
                            Ok((_, raw_sketch)) => Ok(Cow::Owned(Sketch::from(raw_sketch))),
                        }
                    }
                    SketchFormat::Unsupported(version) => {
                        Err(PyValueError::new_err(
                            format!(
                                "database was written with a newer skani version: sketch format version {}", 
                                version.map(|v| v.to_string()).unwrap_or(String::from("unknown")),
                            )
                        ))
                    }
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

        // NOTE(@althonos): In skani v0.4.0 the seeds must be finalized once the
        //                  genome (or individual contigs) have been sketched
        sketch.finalize_seed_index();

        Ok(Sketch::from(sketch))
    }

    fn _save_markers<P>(&self, folder: P, format_version: &FormatVersion) -> PyResult<()>
    where
        P: AsRef<Path>,
    {
        let path = folder.as_ref().join(format_version.markers_filename());
        let mut writer = utils::buffered_create(path.as_ref())?;
        if let Ok(vec) = self.markers.read() {
            match format_version {
                FormatVersion::Legacy => {
                    let refs = vec
                        .iter()
                        .cloned()
                        .map(self::sketch::LegacySketch::from)
                        .collect::<Vec<_>>();
                    match bincode::serialize_into(writer, &(&self.params, &refs)) {
                        Ok(()) => Ok(()),
                        Err(err) => Err(PyValueError::new_err(err.to_string())),
                    }
                }
                FormatVersion::V2(_) => {
                    let refs = vec.iter().map(|s| s.as_ref()).collect::<Vec<_>>();
                    skani::types::write_sketch_header(&mut writer)?;
                    match bincode::serialize_into(&mut writer, &(&self.params, &refs)) {
                        Ok(()) => Ok(()),
                        Err(err) => Err(PyValueError::new_err(err.to_string())),
                    }
                }
            }
        } else {
            Err(self::utils::poisoned_lock_error())
        }
    }

    fn _save_index<P>(&self, folder: P, format_version: &FormatVersion, index: &HashMap<String, IndexEntry>) -> PyResult<()>
    where
        P: AsRef<Path>,
    {
        let mut index_vec: Vec<&IndexEntry> = index.values().collect();
        index_vec.sort_unstable_by_key(|entry| entry.offset);

        let path = folder.as_ref().join(format_version.index_filename());
        let writer = utils::buffered_create(path.as_ref())?;
        match bincode::serialize_into(writer, &index_vec) {
            Ok(()) => Ok(()),
            Err(err) => return Err(PyValueError::new_err(err.to_string())),
        }
    }

    fn _flush(&self, storage: &DatabaseStorage) -> PyResult<()> {
        match storage {
            DatabaseStorage::Memory(_) => Ok(()),
            DatabaseStorage::Folder(folder) => {
                self._save_markers(folder, &FormatVersion::Legacy)
            }
            DatabaseStorage::Consolidated(folder, index, format_version) => {
                self._save_markers(folder, &format_version)?;
                self._save_index(folder, &format_version, index)?;
                Ok(())
            }
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

        // load reference sketches record in marker file
        let mut sketches = HashMap::new();
        let mut handle = db.sketches.read().unwrap();
        for marker in db.markers.read().unwrap().iter() {
            let sketch = handle.load(&marker.as_ref().file_name)?;
            let name = Path::new(&(*sketch).as_ref().file_name)
                .file_name()
                .unwrap()
                .to_os_string()
                .into_string()
                .unwrap(); // FIXME
            sketches.insert(name, sketch.into_owned());
        }

        // Ensure sketches are finalized before making them immutable
        for (_, sketch) in sketches.iter_mut() {
            sketch.as_mut().finalize_seed_index();
        }

        drop(handle);
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
        // obtain Unicode representation of path
        let decoded = self::utils::fsdecode(path)?;
        let fspath = Path::new(decoded.to_str()?);

        // detect format version based off filenames
        let format_version = [FormatVersion::V2(1), FormatVersion::Legacy]
            .iter()
            .find(|fv| fspath.join(fv.markers_filename()).exists())
            .cloned()
            .ok_or_else(|| PyFileNotFoundError::new_err(
                fspath.join(FormatVersion::V2(1).markers_filename()).display().to_string(),
            ))?;

        // load markers from detected format
        let markers_path = fspath.join(format_version.markers_filename());
        let mut reader = utils::buffered_open(&markers_path)?;
        let header = reader.fill_buf()?;

        let (params, markers) = match skani::types::detect_sketch_format(header) {
            SketchFormat::Legacy => {
                match bincode::deserialize_from::<_, (SketchParams, Vec<skani::types::LegacySketch>)>(reader)
                {
                    Ok((params, raw_markers)) => (params, raw_markers.into_iter().map(Sketch::from).collect()),
                    Err(err) => return Err(PyValueError::new_err(err.to_string())),
                }
            }
            SketchFormat::Native => {
                reader.consume(skani::types::SKETCH_HEADER_LEN);
                match bincode::deserialize_from::<_, (SketchParams, Vec<skani::types::Sketch>)>(reader)
                {
                    Ok((params, raw_markers)) => (params, raw_markers.into_iter().map(Sketch::from).collect()),
                    Err(err) => return Err(PyValueError::new_err(err.to_string())),
                }
            }
            SketchFormat::Unsupported(version) => {
                return Err(PyValueError::new_err(
                    format!(
                        "database was written with a newer skani version: sketch format version {}", 
                        version.map(|v| v.to_string()).unwrap_or(String::from("unknown")),
                    )
                ))
            }
        };

        // identify whether this is a consolidated database or not
        let index_path = fspath.join(format_version.index_filename());
        let sketches_path = fspath.join(format_version.sketches_filename());
        if index_path.exists() {
            if !sketches_path.exists() {
                return Err(PyFileNotFoundError::new_err(sketches_path.display().to_string()))
            };
            let reader = utils::buffered_open(&index_path)?;
            let index = match bincode::deserialize_from::<_, Vec<IndexEntry>>(reader) {
                Ok(v) => v
                    .into_iter()
                    .map(|entry| (entry.file_name.clone(), entry))
                    .collect::<HashMap<_, _>>(),
                Err(err) => return Err(PyValueError::new_err(err.to_string())),
            };
            Ok(Self {
                params,
                markers: RwLock::new(markers),
                sketches: RwLock::new(DatabaseStorage::Consolidated(fspath.into(), index, format_version)),
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
    ///     format (`str`): The database format to use. Use either
    ///         ``separated`` to write one sketch file per sketch genome
    ///         (as in ``skani<0.3.0``), or ``consolidated`` (the default)
    ///         to write a single sketch file (as in ``skani>=0.3.0``)
    ///
    /// Raises:
    ///     `OSError`: When a new folder could not be created.
    ///     `FileExistsError`: When the folder already contains sketches.
    ///
    /// .. versionadded:: 0.2.0
    ///     The ``format`` keyword argument.
    ///
    #[new]
    #[pyo3(signature = (path=None, *, compression=125, marker_compression=1000, k=15, format=None, format_version=None))]
    pub fn __init__<'py>(
        path: Option<&Bound<'py, PyAny>>,
        compression: usize,
        marker_compression: usize,
        k: usize,
        format: Option<String>,
        format_version: Option<u32>,
    ) -> PyResult<PyClassInitializer<Self>> {
        let storage = match path {
            None => DatabaseStorage::Memory(HashMap::new()),
            Some(folder) => {

                // check that the parameters are correct
                let fv = match format.as_ref().map(String::as_str) {
                    Some("consolidated") | None => match format_version {
                        Some(0) => FormatVersion::Legacy,
                        None => FormatVersion::V2(skani::types::SKETCH_FORMAT_VERSION),
                        Some(n) if n <= skani::types::SKETCH_FORMAT_VERSION => FormatVersion::V2(n),
                        Some(n) => return Err(PyValueError::new_err(format!("invalid format version: {}", n))),
                    },
                    Some("separated") => match format_version {
                        None | Some(0) => FormatVersion::Legacy,
                        Some(1) => return Err(PyValueError::new_err(format!("\"separated\" format only supports format version 0 (legacy)"))),
                        Some(n) => return Err(PyValueError::new_err(format!("invalid format version: {}", n))),
                    }
                    Some(other) => {
                        return Err(PyValueError::new_err(format!("invalid format: {}", other)))
                    }
                };

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
                // check the folder is not already in use independently of format version
                for fv in &[FormatVersion::Legacy, FormatVersion::V2(1)] {
                    let marker_path = buf.join(fv.markers_filename());
                    if marker_path.exists() {
                        return Err(PyFileExistsError::new_err(
                            marker_path.display().to_string(),
                        ));
                    }
                }
                match format
                    .as_ref()
                    .map(String::as_str)
                    .unwrap_or("consolidated")
                {
                    "separated" => DatabaseStorage::Folder(buf),
                    "consolidated" => DatabaseStorage::Consolidated(buf, HashMap::new(), fv),
                    _ => unreachable!(),
                }
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
                DatabaseStorage::Folder(ref folder)
                | DatabaseStorage::Consolidated(ref folder, _, _) => {
                    let pathlib = py.import(pyo3::intern!(py, "pathlib"))?;
                    let path = pathlib.call_method1(pyo3::intern!(py, "Path"), (folder,))?;
                    Ok(path.into())
                }
            }
        } else {
            Err(self::utils::poisoned_lock_error())
        }
    }

    /// `int`: The compression factor for sketches.
    #[getter]
    pub fn get_compression<'py>(slf: PyRef<'py, Self>, _py: Python) -> usize {
        slf.params.c
    }

    /// `int`: The compression factor for marker k-mers.
    #[getter]
    pub fn get_marker_compression<'py>(slf: PyRef<'py, Self>, _py: Python) -> usize {
        slf.params.marker_c
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
    /// ..versionadded:: 0.2.0
    ///     The ``cutoff`` and ``faster_small`` keyword arguments.
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
    #[pyo3(signature = (path, overwrite=false, format=None, format_version=None))]
    pub fn save<'py>(
        &self,
        path: &Bound<'py, PyAny>,
        overwrite: bool,
        format: Option<String>,
        format_version: Option<u32>,
    ) -> PyResult<()> {
        // detect format version
        let fv = match format.as_ref().map(String::as_str) {
            Some("consolidated") | None => match format_version {
                Some(0) => FormatVersion::Legacy,
                None => FormatVersion::V2(skani::types::SKETCH_FORMAT_VERSION),
                Some(n) if n <= skani::types::SKETCH_FORMAT_VERSION => FormatVersion::V2(n),
                Some(n) => return Err(PyValueError::new_err(format!("invalid format version: {}", n))),
            },
            Some("separated") => match format_version {
                None | Some(0) => FormatVersion::Legacy,
                Some(1) => return Err(PyValueError::new_err(format!("\"separated\" format only supports format version 0 (legacy)"))),
                Some(n) => return Err(PyValueError::new_err(format!("invalid format version: {}", n))),
            }
            Some(other) => {
                return Err(PyValueError::new_err(format!("invalid format: {}", other)))
            }
        };

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
        let markers_path = folder.join(fv.markers_filename());
        if !overwrite && markers_path.exists() {
            return Err(PyFileExistsError::new_err(
                markers_path.display().to_string(),
            ));
        }
        self._save_markers(folder, &fv)?;

        // decode which format is required
        let mut storage = match format.as_ref().map(String::as_str) {
            Some("consolidated") | None => DatabaseStorage::Consolidated(folder.to_owned(), HashMap::new(), fv),
            Some("separated") => DatabaseStorage::Folder(folder.to_owned()),
            _ => unreachable!(),
        };

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
                .unwrap(); // FIXME?
            let sketch = self
                .sketches
                .read()
                .map_err(|_| self::utils::poisoned_lock_error())?
                .load(&name)?
                .into_owned();
            storage.store(sketch, &self.params)?;
        }

        self._flush(&storage)
    }

    /// Flush the database buffers to disk.
    ///
    /// This does nothing for a database loaded in memory. For a database
    /// stored in a folder, this will save the markers into a file named
    /// ``markers.bin``.
    ///
    pub fn flush(&self) -> PyResult<()> {
        if let Ok(sketches) = self.sketches.read() {
            self._flush(&*sketches)
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
