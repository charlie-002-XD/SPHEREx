# References (every source used must be listed; also paste into the submission form)

## NASA data and documentation
- NASA/IPAC Infrared Science Archive, SPHEREx: https://irsa.ipac.caltech.edu/Missions/spherex.html
- SPHEREx Data Access (SIA2 collections, cutouts): https://caltech-ipac.github.io/spherex-archive-documentation/spherex-data-access/
- SPHEREx Cutout Tool: https://irsa.ipac.caltech.edu/data/SPHEREx/docs/cutout_tool.html
- IRSA SPHEREx tutorial notebooks (FITS layout): https://caltech-ipac.github.io/irsa-tutorials/spherex/
- Gaia DR3 (ESA/Gaia, via STScI MAST): https://archive.stsci.edu/missions-and-data/gaia
- Gaia DR3 via CDS VizieR (catalog overlay, I/355/gaiadr3): https://vizier.cds.unistra.fr/viz-bin/VizieR?-source=I/355
- AllWISE via CDS VizieR (catalog overlay, II/328/allwise): https://vizier.cds.unistra.fr/viz-bin/VizieR?-source=II/328
- WiseView (inspiration only): http://byw.tools/wiseview

## Challenge
- Planet X and SPHEREx, NASA Space Apps 2026: https://www.spaceappschallenge.org/2026/challenges/planet-x-and-spherex/

## Software (add name, version, license when added)
- uPlot 1.6.32 (MIT, Leon Sorokin), bundled as an npm dependency and loaded lazily: https://github.com/leeoniya/uPlot
- Aladin Lite 3.8.2 (LGPL-3.0, CDS, Université de Strasbourg), loaded from the CDS CDN: https://aladin.cds.unistra.fr/AladinLite/
- Rust crates and npm packages: filled in from `Cargo.lock` / `package-lock.json` before submission.

## Scientific references
- Molecular ice band positions (H2O 3.05 µm, CO2 4.27 µm, CO 4.67 µm): Boogert, Gerakines & Whittet 2015, ARA&A 53, 541.
- CH4 nu3 band ~3.32 µm: Ehrenfreund & Charnley 2000, ARA&A 38, 427.
