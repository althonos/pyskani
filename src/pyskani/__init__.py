from . import _skani
from ._skani import Sketch, Database, Hit

__version__ = _skani.__version__
__author__ = _skani.__author__
__doc__ = _skani.__doc__
__build__ = _skani.__build__
__all__ = [
    "Sketch",
    "Database",
    "Hit",
    "SKANI_VERSION",
]

# Expose the version of embedded skani
SKANI_VERSION = _skani.__build__["dependencies"]["skani"]  # type: ignore

# Small addition to the docstring: show a link redirecting to the
# online version of the documentation, but this can only work when
# Python is running with docstrings enabled
if __doc__ is not None:
    __doc__ += """\nSee Also:
    An online rendered version of the documentation for this version
    of the library on
    `Read The Docs <https://{}.readthedocs.io/en/v{}/>`_.

    """.format(
        __name__,
        __version__
    )

