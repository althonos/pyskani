# Contributing to Pyskani

For bug fixes or new features, please file an issue before submitting a
pull request. If the change isn't trivial, it may be best to wait for
feedback.

## Setting up a local repository

You can just clone the repository without needing any extra setup to get a
local copy of the code:

```console
$ git clone https://github.com/althonos/pyskani
```

## Running tests

Tests are written as usual Python unit tests with the `unittest` module of
the standard library. Running them requires the extension to be built
locally:

```console
$ python setup.py build_ext --inplace
$ python -m unittest discover -vv
```

## Coding guidelines

This project targets Python 3.6 or later.

### Type hints

Python objects should be typed where applicable. For the Rust code,
an external type stub must be maintained; make sure to update `pyskani/_skani.pyi`
file as well when making changes to the Python interface.

### Interfacing with Rust

This project uses Rust and [`pyo3`](https://pyo3.rs) to interface with the
`skani` code. You'll need a Rust compiler on your system to compile the
extension if you are going to tinker with it, even if you only intend
to edit the Python part.

If you're unfamiliar with Rust, check the [*Get Started*](https://www.rust-lang.org/learn/get-started)
page of the Rust documentation on how to get a local toolchain.
