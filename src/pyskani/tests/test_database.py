import os
import pathlib
import tempfile
import unittest

import pyskani


class TestDatabase(unittest.TestCase):
    
    def test_memory(self):
        database = pyskani.Database()
        database.sketch("test genome", b"ATGC"*100)
        self.assertIs(database.path, None)

    def test_folder(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            database = pyskani.Database(tmpdir)
            database.sketch("test", b"ATGC"*100)
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "test.sketch")))
            self.assertFalse(os.path.exists(os.path.join(tmpdir, "markers.bin")))
            database.flush()
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "test.sketch")))
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "markers.bin")))
            self.assertEqual(database.path, pathlib.Path(tmpdir))
