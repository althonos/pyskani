import os
import tempfile
import unittest

import pyskani
from . import fasta


DATA_FOLDER = os.path.realpath(os.path.join(__file__, os.pardir, "data"))


@unittest.skipUnless(os.path.exists(DATA_FOLDER), "missing data folder")
class TestAniEC590(unittest.TestCase):

    @classmethod
    def setUpClass(cls):
        cls.ref = next(fasta.parse(os.path.join(DATA_FOLDER, "e.coli-EC590.fasta")))
        cls.db = pyskani.Database()
        cls.db.sketch("EC590", cls.ref.seq.encode("ascii"))
        cls.query = next(fasta.parse(os.path.join(DATA_FOLDER, "e.coli-K12.fasta")))

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
        self.assertAlmostEqual(hits[0].identity, 0.9978, places=4)

    def test_median(self):
        hits = self.db.query("K12", self.query.seq.encode("ascii"), median=True)
        self.assertEqual(len(hits), 1)
        self.assertAlmostEqual(hits[0].reference_fraction, 0.9246, places=4)
        self.assertAlmostEqual(hits[0].query_fraction, 0.9189, places=4)
        self.assertAlmostEqual(hits[0].identity, 0.9995, places=4)