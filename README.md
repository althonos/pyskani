# 🐍⛓️🧬 Pyskani [![Stars](https://img.shields.io/github/stars/althonos/pyskani.svg?style=social&maxAge=3600&label=Star)](https://github.com/althonos/pyskani/stargazers)

*[PyO3](https://pyo3.rs/) bindings and Python interface to [skani](https://github.com/bluenote-1577/skani), a method for fast fast genomic identity calculation using sparse chaining.*

[![Actions](https://img.shields.io/github/actions/workflow/status/althonos/pyskani/test.yml?branch=main&logo=github&style=flat-square&maxAge=300)](https://github.com/althonos/pyskani/actions)
[![Coverage](https://img.shields.io/codecov/c/gh/althonos/pyskani/branch/main.svg?style=flat-square&maxAge=3600)](https://codecov.io/gh/althonos/pyskani/)
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
[![Downloads](https://img.shields.io/badge/dynamic/json?style=flat-square&color=303f9f&maxAge=86400&label=downloads&query=%24.total_downloads&url=https%3A%2F%2Fapi.pepy.tech%2Fapi%2Fprojects%2Fpyskani)](https://pepy.tech/project/pyskani)


## 🗺️ Overview

`skani` is a method developed by [Jim Shaw](https://jim-shaw-bluenote.github.io/)
and [Yun William Yu](https://github.com/yunwilliamyu) for fast and robust
metagenomic sequence comparison through sparse chaining. It improves on
FastANI by being more accurate and much faster, while requiring less memory.

`pyskani` is a Python module, implemented using the [PyO3](https://pyo3.rs/)
framework, that provides bindings to `skani`. It directly links to the
``skani`` code, which has the following advantages over CLI wrappers:

- **pre-built wheels**: `pyskani` is distributed on PyPI and features
  pre-built wheels for common platforms, including x86-64 and Arm64 UNIX.
- **single dependency**: If your software or your analysis pipeline is
  distributed as a Python package, you can add `pyskani` as a dependency to
  your project, and stop worrying about the `skani` binary being present on
  the end-user machine.
- **sans I/O**: Everything happens in memory, in Python objects you control,
  making it easier to pass your sequences to `skani` without having to write
  them to a temporary file.

*This library is still a work-in-progress, and in an experimental stage,
but it should already pack enough features to be used in a standard pipeline.*


## 🔧 Installing

Pyskani can be installed directly from [PyPI](https://pypi.org/project/pyskani/),
which hosts some pre-built CPython wheels for x86-64 Unix platforms, as well
as the code required to compile from source with Rust:
```console
$ pip install pyskani
```
<!-- Otherwise, pyskani is also available as a [Bioconda](https://anaconda.org/bioconda/pyskani)
package:
```console
$ conda install -c bioconda pyskani
``` -->

In the event you have to compile the package from source, all the required
Rust libraries are vendored in the source distribution, and a Rust compiler
will be setup automatically if there is none on the host machine.


## 💡 Examples

### 📝 Creating a database

A database can be created either in memory or using a folder on the machine
filesystem to store the sketches. Independently of the storage, a database
can be used immediately for querying, or saved to a different location.

Here is how to create a database into memory,
using [Biopython](https://github.com/biopython/biopython)
to load the record:
```python
database = pyskani.Database()
record = Bio.SeqIO.read("vendor/skani/test_files/e.coli-EC590.fasta", "fasta")
database.sketch("E. coli EC590", bytes(record.seq))
```

For draft genomes, simply pass more arguments to the `sketch` method, for
which you can use the splat operator:
```python
database = pyskani.Database()
records = Bio.SeqIO.parse("vendor/skani/test_files/e.coli-o157.fasta", "fasta")
sequences = (bytes(record.seq) for record in records)
database.sketch("E. coli O157", *sequences)
```

### 🗒️ Loading a database

To load a database, either created from `skani` or `pyskani`, you can either
load all sketches into memory, for fast querying:
```python
database = pyskani.Database.load("path/to/sketches")
```

Or load the files lazily to save memory, at the cost of slower querying:
```python
database = pyskani.Database.open("path/to/sketches")
```

### 🔎 Querying a database

Once a database has been created or loaded, use the `Database.query` method
to compute ANI for some query genomes:
```python
record = Bio.SeqIO.read("vendor/skani/test_files/e.coli-K12.fasta", "fasta")
hits = database.query("E. coli K12", bytes(record.seq))
```

## 🔎 See Also

Computing ANI for closed genomes? You may also be interested in
[`pyfastani`, a Python package for computing ANI](https://github.com/althonos/pyfastani)
using the [FastANI method](https://www.nature.com/articles/s41467-018-07641-9)
developed by [Chirag Jain](https://github.com/cjain7) *et al.*

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
