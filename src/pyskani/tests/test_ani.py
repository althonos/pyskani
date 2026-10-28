import gzip
import os
import tempfile
import unittest

import pyskani
from . import fasta

# TODO: Update to use `pkg-resources`
DATA_FOLDER = os.path.realpath(os.path.join(__file__, os.pardir))


@unittest.skipUnless(os.path.exists(DATA_FOLDER), "missing data folder")
class TestAniEC590(unittest.TestCase):

    @staticmethod
    def load(filename):
        with gzip.open(os.path.join(DATA_FOLDER, f"{filename}.gz"), mode="rt") as f:
            return next(fasta.parse(f))

    @classmethod
    def setUpClass(cls):
        cls.ref = cls.load("e.coli-EC590.fasta")
        cls.db = pyskani.Database()
        cls.db.sketch("EC590", cls.ref.seq.encode("ascii"))
        cls.query = cls.load("e.coli-K12.fasta")

    def test_basic(self):
        hits = self.db.query("K12", self.query.seq.encode("ascii"))
        self.assertEqual(len(hits), 1)
        self.assertAlmostEqual(hits[0].reference_fraction, 0.9246, places=4)
        self.assertAlmostEqual(hits[0].query_fraction, 0.9189, places=4)
        self.assertAlmostEqual(hits[0].identity, 0.9939, places=4)

    def test_no_learned_ani(self):
        hits = self.db.query("K12", self.query.seq.encode("ascii"), learned_ani=False)
        self.assertEqual(len(hits), 1)
        self.assertAlmostEqual(hits[0].reference_fraction, 0.9246, places=4)
        self.assertAlmostEqual(hits[0].query_fraction, 0.9189, places=4)
        self.assertAlmostEqual(hits[0].identity, 0.9946, places=4)

    def test_learned_ani(self):
        hits = self.db.query("K12", self.query.seq.encode("ascii"), learned_ani=True)
        self.assertEqual(len(hits), 1)
        self.assertAlmostEqual(hits[0].reference_fraction, 0.9246, places=4)
        self.assertAlmostEqual(hits[0].query_fraction, 0.9189, places=4)
        self.assertAlmostEqual(hits[0].identity, 0.9939, places=4)

    def test_robust(self):
        hits = self.db.query("K12", self.query.seq.encode("ascii"), robust=True)
        self.assertEqual(len(hits), 1)
        self.assertAlmostEqual(hits[0].reference_fraction, 0.9246, places=4)
        self.assertAlmostEqual(hits[0].query_fraction, 0.9189, places=4)
        self.assertAlmostEqual(hits[0].identity, 0.9977, places=4)

    def test_median(self):
        hits = self.db.query("K12", self.query.seq.encode("ascii"), median=True)
        self.assertEqual(len(hits), 1)
        self.assertAlmostEqual(hits[0].reference_fraction, 0.9246, places=4)
        self.assertAlmostEqual(hits[0].query_fraction, 0.9189, places=4)
        self.assertAlmostEqual(hits[0].identity, 0.9995, places=4)

    def test_database_load_legacy(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            self.db.save(tmpdir, format="consolidated", format_version=0)
            db2 = pyskani.Database.load(tmpdir)
            hits1 = self.db.query("K12", self.query.seq.encode("ascii"))
            hits2 = db2.query("K12", self.query.seq.encode("ascii"))
            self.assertEqual(len(hits1), len(hits2))
            for h1, h2 in zip(hits1, hits2):
                self.assertEqual(h1.reference_fraction, h2.reference_fraction)
                self.assertEqual(h1.query_fraction, h2.query_fraction)
                self.assertEqual(h1.identity, h2.identity)

    def test_database_load_separated(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            self.db.save(tmpdir, format="separated")
            db2 = pyskani.Database.load(tmpdir)
            hits1 = self.db.query("K12", self.query.seq.encode("ascii"))
            hits2 = db2.query("K12", self.query.seq.encode("ascii"))
            self.assertEqual(len(hits1), len(hits2))
            for h1, h2 in zip(hits1, hits2):
                self.assertEqual(h1.reference_fraction, h2.reference_fraction)
                self.assertEqual(h1.query_fraction, h2.query_fraction)
                self.assertEqual(h1.identity, h2.identity)

    def test_database_load(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            self.db.save(tmpdir, format="consolidated", format_version=1)
            db2 = pyskani.Database.load(tmpdir)
            hits1 = self.db.query("K12", self.query.seq.encode("ascii"))
            hits2 = db2.query("K12", self.query.seq.encode("ascii"))
            self.assertEqual(len(hits1), len(hits2))
            for h1, h2 in zip(hits1, hits2):
                self.assertEqual(h1.reference_fraction, h2.reference_fraction)
                self.assertEqual(h1.query_fraction, h2.query_fraction)
                self.assertEqual(h1.identity, h2.identity)
            