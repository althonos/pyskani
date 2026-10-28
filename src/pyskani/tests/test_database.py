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

    def test_folder_separated(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            database = pyskani.Database(tmpdir, format="separated")
            database.sketch("test1", b"ATGC"*100)
            database.sketch("test2", b"TTGC"*100)
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "test1.sketch")))
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "test2.sketch")))
            self.assertFalse(os.path.exists(os.path.join(tmpdir, "markers.bin")))
            database.flush()
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "test1.sketch")))
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "test2.sketch")))
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "markers.bin")))
            self.assertEqual(database.path, pathlib.Path(tmpdir))

    def test_folder_legacy(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            database = pyskani.Database(tmpdir, format="consolidated", format_version=0)
            database.sketch("test1", b"ATGC"*100)
            database.sketch("test2", b"TTGC"*100)
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "sketches.db")))
            self.assertFalse(os.path.exists(os.path.join(tmpdir, "index.db")))
            self.assertFalse(os.path.exists(os.path.join(tmpdir, "markers.bin")))
            database.flush()
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "sketches.db")))
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "index.db")))
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "markers.bin")))
            self.assertEqual(database.path, pathlib.Path(tmpdir))

    def test_folder_consolidated(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            database = pyskani.Database(tmpdir, format="consolidated") # default
            database.sketch("test1", b"ATGC"*100)
            database.sketch("test2", b"TTGC"*100)
            self.assertFalse(os.path.exists(os.path.join(tmpdir, "sketches.db")))
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "sketches.db2")))
            self.assertFalse(os.path.exists(os.path.join(tmpdir, "index.db")))
            self.assertFalse(os.path.exists(os.path.join(tmpdir, "index.db2")))
            self.assertFalse(os.path.exists(os.path.join(tmpdir, "markers.bin")))
            self.assertFalse(os.path.exists(os.path.join(tmpdir, "markers.bin2")))
            database.flush()
            self.assertFalse(os.path.exists(os.path.join(tmpdir, "sketches.db")))
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "sketches.db2")))
            self.assertFalse(os.path.exists(os.path.join(tmpdir, "index.db")))
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "index.db2")))
            # self.assertTrue(os.path.exists(os.path.join(tmpdir, "markers.bin")))
            self.assertTrue(os.path.exists(os.path.join(tmpdir, "markers.bin2")))
            self.assertEqual(database.path, pathlib.Path(tmpdir))

    def test_format_version(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            db = pyskani.Database(tmpdir, format="consolidated")
            self.assertEqual(db.format, "consolidated")
            self.assertEqual(db.format_version, 1)
        with tempfile.TemporaryDirectory() as tmpdir:
            db = pyskani.Database(tmpdir, format="consolidated", format_version=0)
            self.assertEqual(db.format, "consolidated")
            self.assertEqual(db.format_version, 0)
        with tempfile.TemporaryDirectory() as tmpdir:
            db = pyskani.Database(tmpdir, format="separated")
            self.assertEqual(db.format, "separated")
            self.assertEqual(db.format_version, 0)
        with tempfile.TemporaryDirectory() as tmpdir:
            db = pyskani.Database()
            self.assertEqual(db.format, "memory")
            self.assertEqual(db.format_version, 1)