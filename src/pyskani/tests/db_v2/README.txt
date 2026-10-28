skani database
==============

Built by   : skani 0.4.0
Built on   : 2026-10-28 19:09:22 UTC
Command    : skani sketch -o db_v2 e.coli-EC590.fasta e.coli-K12.fasta

Sketch parameters (baked in -- see the last note)
-------------------------------------------------
  c (compression)       : 125
  k (k-mer length)      : 15
  marker c              : 1000
  sequence type         : DNA
  individual contigs    : no
  inputs sketched       : 2

Files
-----
  sketches.db2   the sketches, one record per genome
  index.db2      byte offsets into sketches.db2
  markers.bin2   marker k-mers, used to screen candidates before full comparison
  markers.bin    compatibility stub, see below
  README.txt     this file

Notes
-----
* This database is in the skani 0.4 format and CANNOT be read by skani 0.3.x
  or earlier. Point an older skani at this folder and it exits with
  'Problem reading .../markers.bin'. That file is a deliberate stub whose
  only job is to make old versions fail immediately rather than slowly; the
  real marker data is in markers.bin2.

* skani 0.4 still reads databases built by 0.3.x. It warns once and converts
  them while loading, which is slower than using a 0.4 database.

* Search against this database with:
      skani search -d db_v2 -q query.fa

* The sketch parameters above are fixed at build time. Comparing against
  genomes sketched with a different -c or -k does not work -- if you see a
  warning about 'Input parameter c = ... is not equal to the sketch
  parameter c = ...', that is the cause, and the sketch parameters win.
