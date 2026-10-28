use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use skani::types::TaggedIndex;
use smallvec::SmallVec;

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

impl AsMut<skani::types::Sketch> for Sketch {
    fn as_mut(&mut self) -> &mut skani::types::Sketch {
        &mut self.sketch
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

impl From<skani::types::LegacySketch> for Sketch {
    fn from(sketch: skani::types::LegacySketch) -> Self {
        Self { sketch: sketch.0 }
    }
}

impl From<LegacySketch> for Sketch {
    fn from(value: LegacySketch) -> Self {
        skani::types::Sketch {
            file_name: value.file_name,
            seed_index: None,
            contigs: value.contigs,
            total_sequence_length: value.total_sequence_length,
            contig_lengths: value.contig_lengths,
            repetitive_kmers: value.repetitive_kmers,
            marker_seeds: value.marker_seeds,
            marker_c: value.marker_c,
            c: value.c,
            k: value.k,
            contig_order: value.contig_order,
            individual_contig: value.individual_contig,
            amino_acid: value.amino_acid,
        }
        .into()
    }
}

/// A sketch as defined in `skani <=0.3`.
#[derive(Serialize, Deserialize)]
pub(crate) struct LegacySketch {
    file_name: String,
    kmer_seeds_k: Option<skani::types::KmerSeeds>,
    multi_position_storage: skani::types::MultiPositionStorage,
    contigs: Vec<String>,
    total_sequence_length: usize,
    contig_lengths: Vec<skani::types::GnPosition>,
    repetitive_kmers: usize,
    marker_seeds: skani::types::MMHashSet<skani::types::MarkerBits>,
    marker_c: usize,
    c: usize,
    k: usize,
    contig_order: usize,
    individual_contig: bool,
    amino_acid: bool,
}

impl From<Sketch> for LegacySketch {
    fn from(value: Sketch) -> Self {
        // Ensure the seed index is finalized, otherwise we won't be able
        // to access the arrays
        let mut index = value.sketch.seed_index.clone().unwrap_or_default();
        index.finalize();
        // Rebuild the original kmer_seeds_k
        let mut multi_position_storage = Vec::new();
        let mut kmer_seeds_k = skani::types::KmerSeeds::default(); // <SeedBits, u64> where u64 is a TaggedIndex
        for run in index.runs() {
            if run.positions.len() == 1 {
                kmer_seeds_k.insert(
                    run.seed,
                    skani::types::TaggedIndex::single(&run.positions[0]),
                );
            } else {
                let storage_index = multi_position_storage.len();
                let storage_vec = multi_position_storage.push_mut(SmallVec::default());
                kmer_seeds_k.insert(run.seed, TaggedIndex::multiple(storage_index));
                for &seed_position in run.positions {
                    storage_vec.push(seed_position);
                }
            }
        }
        // Return the legacy sketch
        LegacySketch {
            file_name: value.sketch.file_name,
            kmer_seeds_k: Some(kmer_seeds_k),
            multi_position_storage: multi_position_storage,
            contigs: value.sketch.contigs,
            total_sequence_length: value.sketch.total_sequence_length,
            contig_lengths: value.sketch.contig_lengths,
            repetitive_kmers: value.sketch.repetitive_kmers,
            marker_seeds: value.sketch.marker_seeds,
            marker_c: value.sketch.marker_c,
            c: value.sketch.c,
            k: value.sketch.k,
            contig_order: value.sketch.contig_order,
            individual_contig: value.sketch.individual_contig,
            amino_acid: value.sketch.amino_acid,
        }
    }
}
