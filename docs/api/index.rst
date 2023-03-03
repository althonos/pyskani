API Reference
=============


The version of the wrapped ``skani`` library is stored in the
:py:const:`SKANI_VERSION` constant, accessible from the top-level namespace.
The version of ``skani`` wrapped in this version of ``pyskani`` is
|SKANI_VERSION|_.

.. note::

    Versioning of wrapper libraries and bindings is not necessarily obvious. 
    Since ``pyskani`` follows `semantic versioning <https://semver.org/>`_, it 
    does not keep its version synchronized with ``skani`` in any way, treating 
    it as any other dependency. The changelog will document updates, so it can 
    be used to find a particular ``pyskani`` version to pin if a given ``skani`` 
    release is wanted.


.. currentmodule:: pyskani

.. toctree::
   :hidden:

   database <database>
   sketch <sketch>
   hit <hit>


.. only:: html

    .. autosummary::
        :nosignatures:

        pyskani.Database
        pyskani.Sketch
        pyskani.Hit
