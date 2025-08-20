Pyskani |Stars|
================

.. |Stars| image:: https://img.shields.io/github/stars/althonos/pyskani.svg?style=social&maxAge=3600&label=Star
   :target: https://github.com/althonos/pyskani/stargazers

`PyO3 <https://pyo3.rs/>`_ *bindings and Python interface to* `skani <https://github.com/bluenote-1577/skani>`_,
*a method for fast fast genomic identity calculation using sparse chaining.*

|Actions| |Coverage| |PyPI| |Bioconda| |AUR| |Wheel| |Versions| |Implementations| |License| |Source| |Mirror| |Issues| |Docs| |Changelog| |Downloads|

.. |Actions| image:: https://img.shields.io/github/actions/workflow/status/althonos/pyskani/test.yml?branch=main&logo=github&style=flat-square&maxAge=300
   :target: https://github.com/althonos/pyskani/actions

.. |Coverage| image:: https://img.shields.io/codecov/c/gh/althonos/pyskani/branch/main.svg?style=flat-square&maxAge=600
   :target: https://codecov.io/gh/althonos/pyskani/

.. |PyPI| image:: https://img.shields.io/pypi/v/pyskani.svg?style=flat-square&maxAge=3600
   :target: https://pypi.python.org/pypi/pyskani

.. |Bioconda| image:: https://img.shields.io/conda/vn/bioconda/pyskani?style=flat-square&maxAge=3600
   :target: https://anaconda.org/bioconda/pyskani

.. |AUR| image:: https://img.shields.io/aur/version/python-pyskani?logo=archlinux&style=flat-square&maxAge=3600
   :target: https://aur.archlinux.org/packages/python-pyskani

.. |Wheel| image:: https://img.shields.io/pypi/wheel/pyskani?style=flat-square&maxAge=3600
   :target: https://pypi.org/project/pyskani/#files

.. |Versions| image:: https://img.shields.io/pypi/pyversions/pyskani.svg?style=flat-square&maxAge=3600
   :target: https://pypi.org/project/pyskani/#files

.. |Implementations| image:: https://img.shields.io/pypi/implementation/pyskani.svg?style=flat-square&maxAge=3600&label=impl
   :target: https://pypi.org/project/pyskani/#files

.. |License| image:: https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square&maxAge=3600
   :target: https://choosealicense.com/licenses/mit/

.. |Source| image:: https://img.shields.io/badge/source-GitHub-303030.svg?maxAge=2678400&style=flat-square
   :target: https://github.com/althonos/pyskani/

.. |Mirror| image:: https://img.shields.io/badge/mirror-EMBL-009f4d?style=flat-square&maxAge=2678400
   :target: https://git.embl.de/larralde/pyskani/

.. |Issues| image:: https://img.shields.io/github/issues/althonos/pyskani.svg?style=flat-square&maxAge=600
   :target: https://github.com/althonos/pyskani/issues

.. |Docs| image:: https://img.shields.io/readthedocs/pyskani?style=flat-square&maxAge=3600
   :target: http://pyskani.readthedocs.io/en/stable/?badge=stable

.. |Changelog| image:: https://img.shields.io/badge/keep%20a-changelog-8A0707.svg?maxAge=2678400&style=flat-square
   :target: https://github.com/althonos/pyskani/blob/main/CHANGELOG.md

.. |Downloads| image:: https://img.shields.io/badge/dynamic/regex?url=https%3A%2F%2Fpepy.tech%2Fprojects%2Fpyskani&search=%5B0-9%5D%2B.%5B0-9%5D%2B(k%7CM)&style=flat-square&label=downloads&color=303f9f&cacheSeconds=86400
   :target: https://pepy.tech/project/pyskani


Overview
--------

``skani`` is a method developed by `Jim Shaw <https://jim-shaw-bluenote.github.io/>`_
and `Yun William Yu <https://github.com/yunwilliamyu>`_ for fast and robust
metagenomic sequence comparison through sparse chaining. It improves on
FastANI by being more accurate and much faster, while requiring less memory.

``pyskani`` is a Python module, implemented using the `PyO3 <https://pyo3.rs/>`_
framework, that provides bindings to ``skani``. It directly links to the
``skani`` code, which has the following advantages over CLI wrappers:

- **pre-built wheels**: ``pyskani`` is distributed on PyPI and features
  pre-built wheels for common platforms, including x86-64 and Arm64 UNIX.
- **single dependency**: If your software or your analysis pipeline is
  distributed as a Python package, you can add ``pyskani`` as a dependency to
  your project, and stop worrying about the ``skani`` binary being present on
  the end-user machine.
- **sans I/O**: Everything happens in memory, in Python objects you control,
  making it easier to pass your sequences to ``skani`` without having to write
  them to a temporary file.


Setup
-----

Pyskani is available for all modern Python versions supported by PyO3 (3.8+).

Run ``pip install pyskani`` in a shell to download the latest release and all
its dependencies from PyPi, or have a look at the
:doc:`Installation page <guide/install>` to find other ways to install ``pyskani``.


Library
-------

.. toctree::
   :maxdepth: 2

   User Guide <guide/index>
   API Reference <api/index>


Related Projects
----------------

The following Python libraries may be of interest for bioinformaticians.

.. include:: related.rst
   

License
-------

This library is provided under the `MIT License <https://choosealicense.com/licenses/mit/>`_.

The ``skani`` code was written by `Jim Shaw <https://jim-shaw-bluenote.github.io/>`_
and is distributed under the terms of the
`MIT License <https://choosealicense.com/licenses/mit/>`_ as well. Source
distributions of ``pyskani`` may vendors additional sources under their
own terms using the ``cargo vendor`` command.

*This project is in no way not affiliated, sponsored, or otherwise endorsed by
the original* ``skani`` *authors. It was developed by*
`Martin Larralde <https://github.com/althonos>`_ *during his
PhD project at the* `European Molecular Biology Laboratory <https://www.embl.de/>`_
*in the* `Zeller team <https://github.com/zellerlab>`_.
