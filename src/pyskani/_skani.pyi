import os
from array import array
from pathlib import Path
from types import TracebackType
from typing import Dict, Union, Optional, Type, List, Literal

_FORMAT = Literal["consolidated", "separated"]

_Path = Union[str, bytes, os.PathLike[str]]
_Sequence = Union[str, bytes, bytearray, memoryview, array]

__version__: str
__author__: str
__build__: Dict[str, object]

class Hit:
    def __init__(
        self,
        identity: float,
        query_name: str,
        query_fraction: float,
        reference_name: str,
        reference_fraction: float,
    ) -> None: ...
    def __repr__(self) -> str: ...
    @property
    def identity(self) -> float: ...
    @property
    def query_name(self) -> str: ...
    @property
    def query_fraction(self) -> float: ...
    @property
    def reference_name(self) -> str: ...
    @property
    def reference_fraction(self) -> float: ...

class Sketch:
    @property
    def name(self) -> str: ...
    @property
    def amino_acid(self) -> bool: ...

class Database:
    @classmethod
    def load(cls, path: _Path) -> Database: ...
    @classmethod
    def open(cls, path: _Path) -> Database: ...
    def __init__(
        self,
        path: Union[str, bytes, os.PathLike[str], None] = None,
        *,
        compression: int = ...,
        marker_compression: int = ...,
        k: int = ...,
        format: Optional[_FORMAT] = None,
    ) -> None: ...
    def __enter__(self) -> Database: ...
    def __exit__(
        self,
        exc_type: Optional[Type[BaseException]],
        exc: Optional[BaseException],
        traceback: Optional[TracebackType],
    ) -> Optional[bool]: ...
    @property
    def path(self) -> Optional[Path]: ...
    @property
    def compression(self) -> int: ...
    @property
    def marker_compression(self) -> int: ...
    def sketch(self, name: str, *contigs: _Sequence, seed: bool = True) -> None: ...
    def query(
        self,
        name: str,
        *contigs: _Sequence,
        seed: bool = True,
        learned_ani: Optional[bool] = None,
        median: bool = False,
        robust: bool = False,
        cutoff: Optional[float] = None,
        faster_small: bool = False,
    ) -> List[Hit]: ...
    def save(self, path: _Path, format: Optional[_FORMAT] = None) -> None: ...
    def flush(self) -> None: ...
