# 🐍⛓️🧬 Pyskani [![Stars](https://img.shields.io/github/stars/althonos/pyskani.svg?style=social&maxAge=3600&label=Star)](https://github.com/althonos/pyskani/stargazers)

*[PyO3](https://pyo3.rs/) bindings and Python interface to [skani](https://github.com/bluenote-1577/skani), a method for fast fast genomic identity calculation using sparse chaining.

[![Actions](https://img.shields.io/github/actions/workflow/status/althonos/pyskani/test.yml?branch=main&logo=github&style=flat-square&maxAge=300)](https://github.com/althonos/pyskani/actions)
<!-- [![Coverage](https://img.shields.io/codecov/c/gh/althonos/pyskani/branch/main.svg?style=flat-square&maxAge=3600)](https://codecov.io/gh/althonos/pyskani/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square&maxAge=2678400)](https://choosealicense.com/licenses/mit/)
[![PyPI](https://img.shields.io/pypi/v/pyskani.svg?style=flat-square&maxAge=3600)](https://pypi.org/project/pyskani)
[![Bioconda](https://img.shields.io/conda/vn/bioconda/pyskani?style=flat-square&maxAge=3600&logo=anaconda)](https://anaconda.org/bioconda/pyskani)
[![AUR](https://img.shields.io/aur/version/python-pyskani?logo=archlinux&style=flat-square&maxAge=3600)](https://aur.archlinux.org/packages/python-pyskani)
[![Wheel](https://img.shields.io/pypi/wheel/pyskani.svg?style=flat-square&maxAge=3600)](https://pypi.org/project/pyskani/#files)
[![Python Versions](https://img.shields.io/pypi/pyversions/pyskani.svg?style=flat-square&maxAge=600)](https://pypi.org/project/pyskani/#files)
[![Python Implementations](https://img.shields.io/pypi/implementation/pyskani.svg?style=flat-square&maxAge=600&label=impl)](https://pypi.org/project/pyskani/#files)
[![Source](https://img.shields.io/badge/source-GitHub-303030.svg?maxAge=2678400&style=flat-square)](https://github.com/althonos/pyskani/)
[![Mirror](https://img.shields.io/badge/mirror-EMBL-009f4d?style=flat-square&maxAge=2678400)](https://git.embl.de/larralde/pyskani/)
[![Issues](https://img.shields.io/github/issues/althonos/pyskani.svg?style=flat-square&maxAge=600)](https://github.com/althonos/pyskani/issues)
[![Docs](https://img.shields.io/readthedocs/pyskani/latest?style=flat-square&maxAge=600)](https://pyskani.readthedocs.io)
[![Changelog](https://img.shields.io/badge/keep%20a-changelog-8A0707.svg?maxAge=2678400&style=flat-square)](https://github.com/althonos/pyskani/blob/master/CHANGELOG.md)
[![Downloads](https://img.shields.io/badge/dynamic/json?style=flat-square&color=303f9f&maxAge=86400&label=downloads&query=%24.total_downloads&url=https%3A%2F%2Fapi.pepy.tech%2Fapi%2Fprojects%2Fpyskani)](https://pepy.tech/project/pyskani) -->


## 🗺️ Overview

`skani` is a method developed by [Jim Shaw](https://jim-shaw-bluenote.github.io/)
and [Yun William Yu](https://github.com/yunwilliamyu) for fast and robust 
metagenomic sequence comparison through sparse chaining. It improves on 
FastANI by being more accurate and much faster, while requiring less memory.

`pyskani` is a Python module, implemented using the [PyO3](https://pyo3.rs/)
framework, that provides bindings to `skani`. 

*This library is still a work-in-progress, and in an experimental stage,
but it should already pack enough features to be used in a standard pipeline.*


## 🔧 Installing

<!-- pyskani can be installed directly from [PyPI](https://pypi.org/project/pyskani/),
which hosts some pre-built CPython wheels for x86-64 Unix platforms, as well
as the code required to compile from source with Cython:
```console
$ pip install pyskani
``` -->

In the event you have to compile the package from source, all the required
Rust libraries are vendored in the source distribution, and a Rust compiler
will be setup automatically if there is none on the host machine.

<!-- Otherwise, pyskani is also available as a [Bioconda](https://anaconda.org/bioconda/pyskani)
package:
```console
$ conda install -c bioconda pyskani
``` -->

## 💡 Example

The following snippets show how to compute the ANI between two genomes,
with the reference being a draft genome.  For one-to-many or many-to-many 
searches, simply add additional references with `database.add_draft` before 
indexing.


### 🔬 [Biopython](https://github.com/biopython/biopython)

Biopython does not let us access to the sequence directly, so we need to
convert it to bytes first with the `bytes` builtin function. For older
versions of Biopython (earlier than 1.79), use `record.seq.encode()`
instead of `bytes(record.seq)`.

```python
import pyskani
import Bio.SeqIO

database = pyskani.Database()

# add a single draft genome to the database
ref = list(Bio.SeqIO.parse("vendor/skani/test_files/e.coli-o157.fasta", "fasta"))
database.add_draft("E. coli O157", [bytes(record.seq) for record in ref])

# read the query and query the database
query = Bio.SeqIO.read("vendor/skani/test_files/e.coli-K12.fasta", "fasta")
hits = database.query_genome("E.coli K12", bytes(query.seq))
for hit in hits:
    print(hit.query_name, hit.reference_name, hit.identity, hit.query_fraction, hit.reference_fraction)
```

### 🧪 [Scikit-bio](https://github.com/biocore/scikit-bio)

Scikit-bio lets us access to the sequence directly as a `numpy` array, but
shows the values as byte strings by default. To make them readable as
`char` (for compatibility with the C code), they must be cast with
`seq.values.view('B')`.

```python
import pyskani
import skbio.io

database = pyskani.Database()

# add a single draft genome to the database
ref = list(skbio.io.read("vendor/skani/test_files/e.coli-o157.fasta", "fasta"))
database.add_draft("E. coli O157", [seq.values.view('B')for seq in ref])

# read the query and query the mapper
query = next(skbio.io.read("vendor/skani/test_files/e.coli-K12.fasta", "fasta"))
hits = database.query_genome("E.coli K12", query.values.view('B'))
for hit in hits:
    print(hit.query_name, hit.reference_name, hit.identity, hit.query_fraction, hit.reference_fraction)
```


## 💭 Feedback

### ⚠️ Issue Tracker

Found a bug ? Have an enhancement request ? Head over to the 
[GitHub issue tracker](https://github.com/althonos/pyskani/issues) if you need 
to report or ask something. If you are filing in on a bug, please include as 
much information as you can about the issue, and try to recreate the same bug
in a simple, easily reproducible situation.

### 🏗️ Contributing

Contributions are more than welcome! See
[`CONTRIBUTING.md`](https://github.com/althonos/pyskani/blob/master/CONTRIBUTING.md)
for more details.


## ⚖️ License

This library is provided under the [MIT License](https://choosealicense.com/licenses/mit/).

The `skani` code was written by [Jim Shaw](https://jim-shaw-bluenote.github.io/)
and is distributed under the terms of the [MIT License](https://choosealicense.com/licenses/mit/) 
as well. See `vendor/skani/LICENSE` for more information. Source distributions 
of `pyskani` vendors additional sources under their own terms using 
the [`cargo vendor`](https://doc.rust-lang.org/cargo/commands/cargo-vendor.html) 
command.

*This project is in no way not affiliated, sponsored, or otherwise endorsed
by the [original `skani` authors](https://jim-shaw-bluenote.github.io/). 
It was developed by [Martin Larralde](https://github.com/althonos/) during his 
PhD project at the [European Molecular Biology Laboratory](https://www.embl.de/) 
in the [Zeller team](https://github.com/zellerlab).*
