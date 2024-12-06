Installation
============

.. note::

    Wheels are provided for Linux and OSX x86-64, but other machines will
    have to build the wheel from the source distribution. Building ``pyskani``
    involves compiling ``skani``, which requires a Rust compiler to be 
    available on the local machine.


PyPi
^^^^

``pyskani`` is hosted on GitHub, but the easiest way to install it is to download
the latest release from its `PyPi repository <https://pypi.python.org/pypi/pyskani>`_.
It will install all dependencies then install ``pyskani`` either from a wheel if
one is available, or from source after compiling the Cython code :

.. code:: console

   $ pip install --user pyskani


Conda
^^^^^

`pyskani` is also available as a `recipe <https://anaconda.org/bioconda/pyskani>`_
in the `bioconda <https://bioconda.github.io/>`_ channel. To install, simply
use the ``conda`` installer:

.. code:: console

   $ conda install -c bioconda pyskani


Arch User Repository
^^^^^^^^^^^^^^^^^^^^

A package recipe for Arch Linux can be found in the Arch User Repository
under the name `python-pyskani <https://aur.archlinux.org/packages/python-pyskani>`_.
It will always match the latest release from PyPI.

Steps to install on ArchLinux depend on your `AUR helper <https://wiki.archlinux.org/title/AUR_helpers>`_
(``yaourt``, ``aura``, ``yay``, etc.). For ``aura``, you'll need to run:

.. code:: console

    $ aura -A python-pyskani


GitHub + ``pip``
^^^^^^^^^^^^^^^^

If, for any reason, you prefer to download the library from GitHub, you can clone
the repository and install the repository by running (with the admin rights):

.. code:: console

   $ git clone https://github.com/althonos/pyskani
   $ pip install --user ./pyskani

.. caution::

    Keep in mind this will install always try to install the latest commit,
    which may not even build, so consider using a versioned release instead.


GitHub + ``build``
^^^^^^^^^^^^^^^^^^

If you do not want to use ``pip``, you can still clone the repository and
use ``build`` and ``installer`` manually:

.. code:: console

    $ git clone --recursive https://github.com/althonos/pyskani
    $ cd pyskani
    $ python -m build .
    # python -m installer dist/*.whl

.. Danger::

    Installing packages without ``pip`` is strongly discouraged, as they can
    only be uninstalled manually, and may damage your system.
