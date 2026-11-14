# Project Submission — field-by-field draft

One section per field of the NASA Space Apps 2026 Project Submission page, in
the form's order. Text between the `--- copy start ---` / `--- copy end ---`
markers is meant to be pasted into the form verbatim.

`TODO(team):` = the human team must fill this. `TODO(measured):` = fill after
`integration-live` produces real numbers/screenshots.

---

## Project Name

--- copy start ---
SPHEREx Blink — a time-lapse window into the SPHEREx all-sky survey
--- copy end ---

(Short alternative if the field has a tight length limit: "SPHEREx Blink".)

---

## High-Level Project Summary

<= 120 words. Current word count: **101** (limit 120) — recount after any
edit; count the copy block below only.

--- copy start ---
SPHEREx Blink is a public web tool that turns NASA's SPHEREx spectral sky
survey into a blink comparator. Enter any RA/Dec and the app builds a
time-lapse of every SPHEREx exposure covering that patch, fetched live from
the IRSA archive, so moving objects like brown dwarfs and high-proper-motion
stars jump out visually. Blink, differencing and stacking modes compare
epochs like-for-like; clicking any pixel assembles a spectrum from all
exposures that cover it. Every view is a shareable, embeddable URL, and the
five official challenge targets are built-in presets. Our goal is to make
Planet-X-style discovery accessible to anyone with a browser.
--- copy end ---

---

## Link to Project Demo

Public link, no login. Video ≤ 30 s OR slide deck ≤ 7 slides including
title/intro.

--- copy start ---
TODO(team): <public demo URL — recorded video link or hosted slide deck>
--- copy end ---

- Video route: record with the script in [`demo-video-script.md`](demo-video-script.md).
- Slides route: build from the outline in [`demo-slides.md`](demo-slides.md).

---

## Link to Final Project

Public link, no login/permission/registration.

--- copy start ---
TODO(team): <deployed app URL>

Optional embed URL for judges (iframe-ready):
TODO(team): <deployed app URL>?embed=1&ra=08h55m10.831s&dec=-07d14m42.53s
--- copy end ---

---

## Detailed Project Description

Sections below follow the guide's prompts: what it does, how it works,
benefits, what we hope to achieve, tools/languages used. Paste as one block.

--- copy start ---
**What it does.** SPHEREx Blink is a public web tool that answers "what does
SPHEREx show at this position, and does anything change there?". A visitor
types sky coordinates (decimal degrees or sexagesimal) and a field of view;
the app returns a time-lapse of every SPHEREx spectral image covering that
patch, played like a short video. Moving objects — brown dwarfs, asteroids,
high-proper-motion stars — and variable sources become visible at a glance,
the same technique used to discover Pluto and, with WISE, the Backyard
Worlds planets. The five targets named by the challenge (Barnard's Star,
Epsilon Indi B, WISE 0855, RW Aurigae, R Draconis) are one-click presets.

**How it works.** For each query the Rust backend runs a SIA2 cone search
against the IRSA SPHEREx collections, downloads the archive cutouts for each
epoch, and decodes the multi-extension FITS files with a purpose-built,
bounds-checked reader (RICE-compressed image tiles, TAN+SIP sky WCS, the
WCS-WAVE wavelength map and the FLAGS bitmask legend are all parsed from the
files themselves). A science layer reprojects every cutout onto a common
north-up grid, then offers four comparison modes — raw, epoch difference,
inverse-variance co-add and per-detector stack — so each comparison is
like-for-like (same detector, same wavelength). Clicking any pixel assembles
a spectrum from every covering exposure: aperture-averaged surface
brightness per wavelength with uncertainties propagated from the VARIANCE
extension, plus flag badges read straight from each file's header legend.
The frontend renders frames on a WebGL2 canvas, with stretch and colormap
applied in the shader. IRSA is a shared public service, so the backend caps
concurrency at 3, retries with backoff, caches cutouts in memory, enforces a
per-request deadline that returns partial results rather than an error, and
rate-limits clients. Every setting — position, field of view, mode, stretch,
colormap, playback speed — lives in the URL, so any view can be shared or
embedded in an iframe.

**Benefits.** It removes the two barriers between SPHEREx data and the
public: no FITS handling or archive API knowledge is needed, and results are
immediate. The same view helps a classroom see a star move, an amateur
astronomer survey a patch, and a researcher preview whether a position is
worth a deeper look. Honest metadata (per-frame flags, wavelengths,
collections) keeps it scientifically trustworthy rather than a toy.

**What we hope to achieve.** Turn "spot the moving object" into something
anyone can do, and widen the pool of eyes on SPHEREx data while the survey
is still young. The tool is built so future data releases are adopted by
configuration alone — no code changes — so it can grow with the archive.

**Tools and languages.** Rust (axum, tokio, reqwest, ndarray, moka) backend;
TypeScript + React frontend bundled with Vite, WebGL2 rendering, zod
validation, uPlot charts, Aladin Lite catalog overlays; OpenAPI contract
shared by both halves; Docker for one-image deployment.
--- copy end ---

---

## Use of Super Intelligence (SI)

--- copy start ---
The team chose the challenge, defined the product idea and requirements,
verified the results, and made all design and scope decisions. SI tools
accelerated implementation under human direction:

- **Claude Code (Anthropic)** acted as project coordinator: it drafted the
  OpenAPI contract and task breakdown from the team's problem statement,
  reviewed every change, and kept project notes. It did not write
  application code.
- **Devin (Cognition)** implemented most backend and frontend tasks — FITS
  parsing, IRSA client, science routines, API routes, viewer, spectrum
  panel, layout, Docker/CI — each in an isolated git branch reviewed before
  merging. Devin commits carry the trailer `Generated-by: Devin
  (SI-assisted)`.
- **A Claude Opus 5.5 agent (Anthropic)** was delegated one complex
  cross-cutting task (live-data integration and verification) under the same
  review gates.

SI-generated: essentially all application code, tests, Dockerfile/CI, and
documentation text (including this submission draft). Not SI-generated: the
idea of how to solve the challenge, the product definition, all measurements
reported, and any screenshots/recordings in the demo, which are real output
of the running application. We used no SI-generated images, audio or video,
and generated no NASA logos or branding. SI use is also documented in
`SI_USAGE.md` in the repository.
--- copy end ---

---

## NASA Data

--- copy start ---
- **NASA/IPAC Infrared Science Archive (IRSA) — SPHEREx spectral images.**
  Queried live via the IRSA SIA2 service (collections `spherex_dr1` and
  `spherex_qr3`) and the IRSA cutout service; multi-extension FITS products
  (IMAGE, VARIANCE, FLAGS, ZODI, WCS-WAVE) are decoded directly.
  https://irsa.ipac.caltech.edu/Missions/spherex.html
- **AllWISE source catalog** (NASA/WISE) — optional overlay markers, queried
  through CDS VizieR (`II/328/allwise`) from the app.
- **Gaia DR3** (ESA mission, hosted by CDS) — optional overlay markers via
  VizieR (`I/355/gaiadr3`); also listed under partner data.
- **IRSA SPHEREx documentation and tutorial notebooks** (data layout, flags,
  access patterns): https://caltech-ipac.github.io/irsa-tutorials/spherex/
  and https://caltech-ipac.github.io/spherex-archive-documentation/spherex-data-access/
--- copy end ---

---

## Space Agency Partner Data and Other References

--- copy start ---
**Partner data and services**
- ESA **Gaia DR3** via CDS VizieR (`I/355/gaiadr3`):
  https://vizier.cds.unistra.fr/viz-bin/VizieR?-source=I/355
- **AllWISE** via CDS VizieR (`II/328/allwise`):
  https://vizier.cds.unistra.fr/viz-bin/VizieR?-source=II/328
- **Aladin Lite 3.8.2** (CDS, Université de Strasbourg, LGPL-3.0) — sky
  overlay and HiPS imagery, loaded from the CDS CDN:
  https://aladin.cds.unistra.fr/AladinLite/

**Challenge**
- Planet X and SPHEREx, NASA Space Apps Challenge 2026:
  https://www.spaceappschallenge.org/2026/challenges/planet-x-and-spherex/

**Scientific references**
- Molecular ice band positions (H₂O 3.05 µm, CO₂ 4.27 µm, CO 4.67 µm):
  Boogert, Gerakines & Whittet 2015, ARA&A 53, 541.
- CH₄ ν₃ band ~3.32 µm: Ehrenfreund & Charnley 2000, ARA&A 38, 427.

**Inspiration (not a dependency)**
- WiseView — http://byw.tools/wiseview

**Software dependencies** — every third-party library actually shipped or
used to build, with version and license. Generated by
`docs/submission/list-deps.sh` from `backend/Cargo.lock` and
`frontend/package-lock.json`.

### Rust crates (from backend/Cargo.lock)
- adler2 2.0.1 (0BSD OR MIT OR Apache-2.0)
- aho-corasick 1.1.5 (Unlicense OR MIT)
- assert-json-diff 2.0.2 (MIT)
- async-compression 0.4.52 (MIT OR Apache-2.0)
- async-lock 3.5.0 (Apache-2.0 OR MIT)
- async-trait 0.1.92 (MIT OR Apache-2.0)
- atomic-waker 1.1.2 (Apache-2.0 OR MIT)
- autocfg 1.5.1 (Apache-2.0 OR MIT)
- axum 0.7.9 (MIT)
- axum-core 0.4.5 (MIT)
- base64 0.22.1 (MIT OR Apache-2.0)
- base64 0.23.1 (MIT OR Apache-2.0)
- bitflags 2.13.2 (MIT OR Apache-2.0)
- bumpalo 3.20.4 (MIT OR Apache-2.0)
- bytes 1.12.1 (MIT)
- cc 1.11.2 (MIT OR Apache-2.0)
- cfg-if 1.0.5 (MIT OR Apache-2.0)
- cfg_aliases 0.2.2 (MIT)
- chacha20 0.10.2 (MIT OR Apache-2.0)
- compression-codecs 0.4.47 (MIT OR Apache-2.0)
- compression-core 0.4.34 (MIT OR Apache-2.0)
- cpufeatures 0.3.1 (MIT OR Apache-2.0)
- crc32fast 1.5.5 (MIT OR Apache-2.0)
- crossbeam-channel 0.5.17 (MIT OR Apache-2.0)
- crossbeam-epoch 0.9.21 (MIT OR Apache-2.0)
- crossbeam-utils 0.8.23 (MIT OR Apache-2.0)
- dashmap 5.5.3 (MIT)
- deadpool 0.12.3 (MIT OR Apache-2.0)
- deadpool-runtime 0.1.4 (MIT OR Apache-2.0)
- displaydoc 0.2.7 (MIT OR Apache-2.0)
- equivalent 1.0.3 (Apache-2.0 OR MIT)
- errno 0.3.15 (MIT OR Apache-2.0)
- event-listener 5.4.2 (Apache-2.0 OR MIT)
- event-listener-strategy 0.5.4 (Apache-2.0 OR MIT)
- find-msvc-tools 0.1.15 (MIT OR Apache-2.0)
- flate2 1.1.11 (MIT OR Apache-2.0)
- fnv 1.0.7 (Apache-2.0 / MIT)
- form_urlencoded 1.2.2 (MIT OR Apache-2.0)
- forwarded-header-value 0.1.1 (ISC)
- futures 0.3.34 (MIT OR Apache-2.0)
- futures-channel 0.3.34 (MIT OR Apache-2.0)
- futures-core 0.3.34 (MIT OR Apache-2.0)
- futures-executor 0.3.34 (MIT OR Apache-2.0)
- futures-io 0.3.34 (MIT OR Apache-2.0)
- futures-macro 0.3.34 (MIT OR Apache-2.0)
- futures-sink 0.3.34 (MIT OR Apache-2.0)
- futures-task 0.3.34 (MIT OR Apache-2.0)
- futures-timer 3.0.4 (MIT/Apache-2.0)
- futures-util 0.3.34 (MIT OR Apache-2.0)
- getrandom 0.2.17 (MIT OR Apache-2.0)
- getrandom 0.4.3 (MIT OR Apache-2.0)
- governor 0.6.3 (MIT)
- h2 0.4.20 (MIT)
- hashbrown 0.14.5 (MIT OR Apache-2.0)
- hashbrown 0.17.1 (MIT OR Apache-2.0)
- hermit-abi 0.5.3 (MIT OR Apache-2.0)
- http 1.5.0 (MIT OR Apache-2.0)
- http-body 1.1.0 (MIT)
- http-body-util 0.1.5 (MIT)
- http-range-header 0.4.2 (MIT)
- httparse 1.10.1 (MIT OR Apache-2.0)
- httpdate 1.0.3 (MIT OR Apache-2.0)
- hyper 1.12.0 (MIT)
- hyper-rustls 0.27.10 (Apache-2.0 OR ISC OR MIT)
- hyper-util 0.1.21 (MIT)
- icu_collections 2.4.0 (Unicode-3.0)
- icu_locale_core 2.4.0 (Unicode-3.0)
- icu_normalizer 2.4.0 (Unicode-3.0)
- icu_normalizer_data 2.4.0 (Unicode-3.0)
- icu_properties 2.4.0 (Unicode-3.0)
- icu_properties_data 2.4.0 (Unicode-3.0)
- icu_provider 2.4.0 (Unicode-3.0)
- idna 1.1.0 (MIT OR Apache-2.0)
- idna_adapter 1.2.2 (Apache-2.0 OR MIT)
- indexmap 2.14.2 (Apache-2.0 OR MIT)
- ipnet 2.12.2 (MIT OR Apache-2.0)
- itoa 1.0.19 (MIT OR Apache-2.0)
- js-sys 0.3.107 (MIT OR Apache-2.0)
- lazy_static 1.5.1 (MIT OR Apache-2.0)
- libc 0.2.191 (MIT OR Apache-2.0)
- litemap 0.8.3 (Unicode-3.0)
- lock_api 0.4.14 (MIT OR Apache-2.0)
- log 0.4.35 (MIT OR Apache-2.0)
- lru-slab 0.1.3 (MIT OR Apache-2.0 OR Zlib)
- matchers 0.2.0 (MIT)
- matchit 0.7.3 (MIT AND BSD-3-Clause)
- matrixmultiply 0.3.11 (MIT/Apache-2.0)
- memchr 2.8.3 (Unlicense OR MIT)
- mime 0.3.18 (MIT OR Apache-2.0)
- mime_guess 2.0.5 (MIT)
- miniz_oxide 0.9.1 (MIT OR Zlib OR Apache-2.0)
- mio 1.2.5 (MIT)
- moka 0.12.16 ((MIT OR Apache-2.0) AND Apache-2.0)
- ndarray 0.16.1 (MIT OR Apache-2.0)
- no-std-compat 0.4.1 (MIT)
- nonempty 0.7.0 (MIT)
- nonzero_ext 0.3.0 (Apache-2.0)
- nu-ansi-term 0.50.3 (MIT)
- num-complex 0.4.6 (MIT OR Apache-2.0)
- num-integer 0.1.47 (MIT OR Apache-2.0)
- num-traits 0.2.19 (MIT OR Apache-2.0)
- num_cpus 1.17.0 (MIT OR Apache-2.0)
- once_cell 1.21.4 (MIT OR Apache-2.0)
- parking 2.2.1 (Apache-2.0 OR MIT)
- parking_lot 0.12.5 (MIT OR Apache-2.0)
- parking_lot_core 0.9.12 (MIT OR Apache-2.0)
- percent-encoding 2.3.2 (MIT OR Apache-2.0)
- pin-project 1.1.13 (Apache-2.0 OR MIT)
- pin-project-internal 1.1.13 (Apache-2.0 OR MIT)
- pin-project-lite 0.2.17 (Apache-2.0 OR MIT)
- portable-atomic 1.15.0 (Apache-2.0 OR MIT)
- portable-atomic-util 0.2.8 (Apache-2.0 OR MIT)
- potential_utf 0.1.6 (Unicode-3.0)
- ppv-lite86 0.2.21 (MIT OR Apache-2.0)
- proc-macro2 1.0.107 (MIT OR Apache-2.0)
- quanta 0.12.6 (MIT)
- quinn 0.11.12 (MIT OR Apache-2.0)
- quinn-proto 0.11.19 (MIT OR Apache-2.0)
- quinn-udp 0.5.16 (MIT OR Apache-2.0)
- quote 1.0.47 (MIT OR Apache-2.0)
- r-efi 6.0.0 (MIT OR Apache-2.0 OR LGPL-2.1-or-later)
- rand 0.10.3 (MIT OR Apache-2.0)
- rand 0.8.8 (MIT OR Apache-2.0)
- rand_chacha 0.3.1 (MIT OR Apache-2.0)
- rand_core 0.10.1 (MIT OR Apache-2.0)
- rand_core 0.6.4 (MIT OR Apache-2.0)
- rand_pcg 0.10.2 (MIT OR Apache-2.0)
- raw-cpuid 11.6.0 (MIT)
- rawpointer 0.2.1 (MIT/Apache-2.0)
- redox_syscall 0.5.18 (MIT)
- regex 1.13.1 (MIT OR Apache-2.0)
- regex-automata 0.4.18 (MIT OR Apache-2.0)
- regex-syntax 0.8.11 (MIT OR Apache-2.0)
- reqwest 0.12.28 (MIT OR Apache-2.0)
- ring 0.17.14 (Apache-2.0 AND ISC)
- rustc-hash 2.1.3 (Apache-2.0 OR MIT)
- rustls 0.23.46 (Apache-2.0 OR ISC OR MIT)
- rustls-pki-types 1.16.0 (MIT OR Apache-2.0)
- rustls-webpki 0.103.17 (ISC)
- rustversion 1.0.23 (MIT OR Apache-2.0)
- ryu 1.0.23 (Apache-2.0 OR BSL-1.0)
- scopeguard 1.2.0 (MIT OR Apache-2.0)
- serde 1.0.229 (MIT OR Apache-2.0)
- serde_core 1.0.229 (MIT OR Apache-2.0)
- serde_derive 1.0.229 (MIT OR Apache-2.0)
- serde_json 1.0.154 (MIT OR Apache-2.0)
- serde_path_to_error 0.1.20 (MIT OR Apache-2.0)
- serde_urlencoded 0.7.1 (MIT/Apache-2.0)
- sharded-slab 0.1.7 (MIT)
- shlex 2.0.1 (MIT OR Apache-2.0)
- signal-hook-registry 1.4.8 (MIT OR Apache-2.0)
- simd-adler32 0.3.10 (MIT)
- slab 0.4.12 (MIT)
- smallvec 1.16.5 (MIT OR Apache-2.0)
- socket2 0.6.5 (MIT OR Apache-2.0)
- spinning_top 0.3.1 (MIT OR Apache-2.0)
- stable_deref_trait 1.2.1 (MIT OR Apache-2.0)
- subtle 2.6.1 (BSD-3-Clause)
- syn 2.0.119 (MIT OR Apache-2.0)
- syn 3.0.10 (MIT OR Apache-2.0)
- sync_wrapper 1.0.2 (Apache-2.0)
- synstructure 0.14.0 (MIT)
- tagptr 0.2.0 (MIT/Apache-2.0)
- thiserror 1.0.69 (MIT OR Apache-2.0)
- thiserror 2.0.21 (MIT OR Apache-2.0)
- thiserror-impl 1.0.69 (MIT OR Apache-2.0)
- thiserror-impl 2.0.21 (MIT OR Apache-2.0)
- thread_local 1.1.10 (MIT OR Apache-2.0)
- tinystr 0.8.5 (Unicode-3.0)
- tinyvec 1.13.3 (Zlib OR Apache-2.0 OR MIT)
- tokio 1.54.1 (MIT)
- tokio-macros 2.7.2 (MIT)
- tokio-rustls 0.26.6 (MIT OR Apache-2.0)
- tokio-util 0.7.20 (MIT)
- tower 0.5.3 (MIT)
- tower-http 0.6.11 (MIT)
- tower-layer 0.3.3 (MIT)
- tower-service 0.3.3 (MIT)
- tower_governor 0.4.3 (MIT OR Apache-2.0)
- tracing 0.1.44 (MIT)
- tracing-attributes 0.1.31 (MIT)
- tracing-core 0.1.36 (MIT)
- tracing-log 0.2.0 (MIT)
- tracing-subscriber 0.3.23 (MIT)
- try-lock 0.2.5 (MIT)
- unicase 2.10.0 (MIT OR Apache-2.0)
- unicode-ident 1.0.27 ((MIT OR Apache-2.0) AND Unicode-3.0)
- untrusted 0.9.0 (ISC)
- url 2.5.8 (MIT OR Apache-2.0)
- utf8_iter 1.1.0 (Apache-2.0 OR MIT)
- utf_types 1.0.0 (Apache-2.0 OR MIT)
- uuid 1.29.2 (Apache-2.0 OR MIT)
- valuable 0.1.1 (MIT)
- want 0.3.2 (MIT)
- wasi 0.11.1+wasi-snapshot-preview1 (Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT)
- wasm-bindgen 0.2.130 (MIT OR Apache-2.0)
- wasm-bindgen-futures 0.4.80 (MIT OR Apache-2.0)
- wasm-bindgen-macro 0.2.130 (MIT OR Apache-2.0)
- wasm-bindgen-macro-support 0.2.130 (MIT OR Apache-2.0)
- wasm-bindgen-shared 0.2.130 (MIT OR Apache-2.0)
- wasm-streams 0.4.2 (MIT OR Apache-2.0)
- web-sys 0.3.107 (MIT OR Apache-2.0)
- web-time 1.1.0 (MIT OR Apache-2.0)
- webpki-roots 1.0.12 (CDLA-Permissive-2.0)
- winapi 0.3.9 (MIT/Apache-2.0)
- winapi-i686-pc-windows-gnu 0.4.0 (MIT/Apache-2.0)
- winapi-x86_64-pc-windows-gnu 0.4.0 (MIT/Apache-2.0)
- windows-link 0.2.1 (MIT OR Apache-2.0)
- windows-sys 0.52.0 (MIT OR Apache-2.0)
- windows-sys 0.61.2 (MIT OR Apache-2.0)
- windows-targets 0.52.6 (MIT OR Apache-2.0)
- windows_aarch64_gnullvm 0.52.6 (MIT OR Apache-2.0)
- windows_aarch64_msvc 0.52.6 (MIT OR Apache-2.0)
- windows_i686_gnu 0.52.6 (MIT OR Apache-2.0)
- windows_i686_gnullvm 0.52.6 (MIT OR Apache-2.0)
- windows_i686_msvc 0.52.6 (MIT OR Apache-2.0)
- windows_x86_64_gnu 0.52.6 (MIT OR Apache-2.0)
- windows_x86_64_gnullvm 0.52.6 (MIT OR Apache-2.0)
- windows_x86_64_msvc 0.52.6 (MIT OR Apache-2.0)
- wiremock 0.6.5 (MIT/Apache-2.0)
- writeable 0.6.4 (Unicode-3.0)
- yoke 0.8.4 (Unicode-3.0)
- yoke-derive 0.8.4 (Unicode-3.0)
- zerocopy 0.8.63 (BSD-2-Clause OR Apache-2.0 OR MIT)
- zerocopy-derive 0.8.63 (BSD-2-Clause OR Apache-2.0 OR MIT)
- zerofrom 0.1.8 (Unicode-3.0)
- zerofrom-derive 0.1.8 (Unicode-3.0)
- zeroize 1.9.1 (Apache-2.0 OR MIT)
- zerotrie 0.2.5 (Unicode-3.0)
- zerovec 0.11.9 (Unicode-3.0)
- zerovec-derive 0.11.7 (Unicode-3.0)
- zlib-rs 0.6.8 (Zlib)
- zmij 1.0.23 (MIT)

### npm packages (from frontend/package-lock.json)
- @asamuzakjp/css-color 3.2.0 (MIT)
- @babel/code-frame 7.29.7 (MIT)
- @babel/compat-data 7.29.7 (MIT)
- @babel/core 7.29.13 (MIT)
- @babel/generator 7.29.14 (MIT)
- @babel/helper-compilation-targets 7.29.13 (MIT)
- @babel/helper-globals 7.29.7 (MIT)
- @babel/helper-module-imports 7.29.7 (MIT)
- @babel/helper-module-transforms 7.29.14 (MIT)
- @babel/helper-plugin-utils 7.29.7 (MIT)
- @babel/helper-string-parser 7.29.14 (MIT)
- @babel/helper-validator-identifier 7.29.7 (MIT)
- @babel/helper-validator-option 7.29.7 (MIT)
- @babel/helpers 7.29.14 (MIT)
- @babel/parser 7.29.14 (MIT)
- @babel/plugin-transform-react-jsx-self 7.29.7 (MIT)
- @babel/plugin-transform-react-jsx-source 7.29.7 (MIT)
- @babel/runtime 7.29.14 (MIT)
- @babel/template 7.29.7 (MIT)
- @babel/traverse 7.29.14 (MIT)
- @babel/types 7.29.14 (MIT)
- @csstools/color-helpers 5.1.0 (MIT-0)
- @csstools/css-calc 2.1.4 (MIT)
- @csstools/css-color-parser 3.1.0 (MIT)
- @csstools/css-parser-algorithms 3.0.5 (MIT)
- @csstools/css-tokenizer 3.0.4 (MIT)
- @esbuild/aix-ppc64 0.21.5 (MIT)
- @esbuild/android-arm 0.21.5 (MIT)
- @esbuild/android-arm64 0.21.5 (MIT)
- @esbuild/android-x64 0.21.5 (MIT)
- @esbuild/darwin-arm64 0.21.5 (MIT)
- @esbuild/darwin-x64 0.21.5 (MIT)
- @esbuild/freebsd-arm64 0.21.5 (MIT)
- @esbuild/freebsd-x64 0.21.5 (MIT)
- @esbuild/linux-arm 0.21.5 (MIT)
- @esbuild/linux-arm64 0.21.5 (MIT)
- @esbuild/linux-ia32 0.21.5 (MIT)
- @esbuild/linux-loong64 0.21.5 (MIT)
- @esbuild/linux-mips64el 0.21.5 (MIT)
- @esbuild/linux-ppc64 0.21.5 (MIT)
- @esbuild/linux-riscv64 0.21.5 (MIT)
- @esbuild/linux-s390x 0.21.5 (MIT)
- @esbuild/linux-x64 0.21.5 (MIT)
- @esbuild/netbsd-x64 0.21.5 (MIT)
- @esbuild/openbsd-x64 0.21.5 (MIT)
- @esbuild/sunos-x64 0.21.5 (MIT)
- @esbuild/win32-arm64 0.21.5 (MIT)
- @esbuild/win32-ia32 0.21.5 (MIT)
- @esbuild/win32-x64 0.21.5 (MIT)
- @eslint-community/eslint-utils 4.10.1 (MIT)
- @eslint-community/regexpp 4.12.3 (MIT)
- @eslint/config-array 0.19.2 (Apache-2.0)
- @eslint/core 0.13.0 (Apache-2.0)
- @eslint/core 0.9.1 (Apache-2.0)
- @eslint/eslintrc 3.3.7 (MIT)
- @eslint/js 9.15.0 (MIT)
- @eslint/object-schema 2.1.7 (Apache-2.0)
- @eslint/plugin-kit 0.2.8 (Apache-2.0)
- @humanfs/core 0.19.2 (Apache-2.0)
- @humanfs/node 0.16.8 (Apache-2.0)
- @humanfs/types 0.15.0 (Apache-2.0)
- @humanwhocodes/module-importer 1.0.1 (Apache-2.0)
- @humanwhocodes/retry 0.4.3 (Apache-2.0)
- @jridgewell/gen-mapping 0.3.13 (MIT)
- @jridgewell/remapping 2.3.5 (MIT)
- @jridgewell/resolve-uri 3.1.3 (MIT)
- @jridgewell/sourcemap-codec 1.6.0 (MIT)
- @jridgewell/trace-mapping 0.3.31 (MIT)
- @nodelib/fs.scandir 2.1.5 (MIT)
- @nodelib/fs.stat 2.0.5 (MIT)
- @nodelib/fs.walk 1.2.8 (MIT)
- @rollup/rollup-android-arm-eabi 4.68.3 (MIT)
- @rollup/rollup-android-arm64 4.68.3 (MIT)
- @rollup/rollup-android-x64 4.68.3 (MIT)
- @rollup/rollup-darwin-arm64 4.68.3 (MIT)
- @rollup/rollup-darwin-x64 4.68.3 (MIT)
- @rollup/rollup-freebsd-arm64 4.68.3 (MIT)
- @rollup/rollup-freebsd-x64 4.68.3 (MIT)
- @rollup/rollup-linux-arm-gnueabihf 4.68.3 (MIT)
- @rollup/rollup-linux-arm-musleabihf 4.68.3 (MIT)
- @rollup/rollup-linux-arm64-gnu 4.68.3 (MIT)
- @rollup/rollup-linux-arm64-musl 4.68.3 (MIT)
- @rollup/rollup-linux-loong64-gnu 4.68.3 (MIT)
- @rollup/rollup-linux-loong64-musl 4.68.3 (MIT)
- @rollup/rollup-linux-ppc64-gnu 4.68.3 (MIT)
- @rollup/rollup-linux-ppc64-musl 4.68.3 (MIT)
- @rollup/rollup-linux-riscv64-gnu 4.68.3 (MIT)
- @rollup/rollup-linux-riscv64-musl 4.68.3 (MIT)
- @rollup/rollup-linux-s390x-gnu 4.68.3 (MIT)
- @rollup/rollup-linux-x64-gnu 4.68.3 (MIT)
- @rollup/rollup-linux-x64-musl 4.68.3 (MIT)
- @rollup/rollup-openbsd-x64 4.68.3 (MIT)
- @rollup/rollup-openharmony-arm64 4.68.3 (MIT)
- @rollup/rollup-win32-arm64-msvc 4.68.3 (MIT)
- @rollup/rollup-win32-ia32-msvc 4.68.3 (MIT)
- @rollup/rollup-win32-x64-gnu 4.68.3 (MIT)
- @rollup/rollup-win32-x64-msvc 4.68.3 (MIT)
- @testing-library/dom 10.4.0 (MIT)
- @testing-library/react 16.1.0 (MIT)
- @types/aria-query 5.0.4 (MIT)
- @types/babel__core 7.20.5 (MIT)
- @types/babel__generator 7.27.0 (MIT)
- @types/babel__template 7.4.4 (MIT)
- @types/babel__traverse 7.28.0 (MIT)
- @types/estree 1.0.10 (MIT)
- @types/json-schema 7.0.15 (MIT)
- @types/prop-types 15.7.15 (MIT)
- @types/react 18.3.12 (MIT)
- @types/react-dom 18.3.1 (MIT)
- @typescript-eslint/eslint-plugin 8.15.0 (MIT)
- @typescript-eslint/parser 8.15.0 (BSD-2-Clause)
- @typescript-eslint/scope-manager 8.15.0 (MIT)
- @typescript-eslint/type-utils 8.15.0 (MIT)
- @typescript-eslint/types 8.15.0 (MIT)
- @typescript-eslint/typescript-estree 8.15.0 (BSD-2-Clause)
- @typescript-eslint/utils 8.15.0 (MIT)
- @typescript-eslint/visitor-keys 8.15.0 (MIT)
- @vitejs/plugin-react 4.3.4 (MIT)
- @vitest/expect 2.1.5 (MIT)
- @vitest/mocker 2.1.5 (MIT)
- @vitest/pretty-format 2.1.5 (MIT)
- @vitest/pretty-format 2.1.9 (MIT)
- @vitest/runner 2.1.5 (MIT)
- @vitest/snapshot 2.1.5 (MIT)
- @vitest/spy 2.1.5 (MIT)
- @vitest/utils 2.1.5 (MIT)
- acorn 8.19.0 (MIT)
- acorn-jsx 5.3.2 (MIT)
- agent-base 7.1.4 (MIT)
- ajv 6.15.0 (MIT)
- ansi-regex 5.0.1 (MIT)
- ansi-styles 4.3.0 (MIT)
- ansi-styles 5.2.0 (MIT)
- argparse 2.0.1 (Python-2.0)
- aria-query 5.3.0 (Apache-2.0)
- assertion-error 2.0.1 (MIT)
- asynckit 0.4.0 (MIT)
- balanced-match 1.0.2 (MIT)
- baseline-browser-mapping 2.11.37 (Apache-2.0)
- brace-expansion 1.1.22 (MIT)
- brace-expansion 2.1.8 (MIT)
- braces 3.0.3 (MIT)
- browserslist 4.29.5 (MIT)
- cac 6.7.14 (MIT)
- call-bind-apply-helpers 1.0.2 (MIT)
- callsites 3.1.0 (MIT)
- caniuse-lite 1.0.30001826 (CC-BY-4.0)
- chai 5.3.3 (MIT)
- chalk 4.1.2 (MIT)
- check-error 2.1.3 (MIT)
- color-convert 2.0.1 (MIT)
- color-name 1.1.4 (MIT)
- combined-stream 1.0.8 (MIT)
- concat-map 0.0.1 (MIT)
- convert-source-map 2.0.0 (MIT)
- cross-spawn 7.0.6 (MIT)
- cssstyle 4.6.0 (MIT)
- csstype 3.2.3 (MIT)
- data-urls 5.0.0 (MIT)
- debug 4.4.3 (MIT)
- decimal.js 10.6.0 (MIT)
- deep-eql 5.0.2 (MIT)
- deep-is 0.1.4 (MIT)
- delayed-stream 1.0.0 (MIT)
- dequal 2.0.3 (MIT)
- dom-accessibility-api 0.5.16 (MIT)
- dunder-proto 1.0.1 (MIT)
- electron-to-chromium 1.5.485 (ISC)
- entities 6.0.1 (BSD-2-Clause)
- es-define-property 1.0.1 (MIT)
- es-errors 1.3.0 (MIT)
- es-module-lexer 1.7.0 (MIT)
- es-object-atoms 1.1.2 (MIT)
- es-set-tostringtag 2.1.0 (MIT)
- esbuild 0.21.5 (MIT)
- escalade 3.2.0 (MIT)
- escape-string-regexp 4.0.0 (MIT)
- eslint 9.15.0 (MIT)
- eslint-scope 8.4.0 (BSD-2-Clause)
- eslint-visitor-keys 3.4.3 (Apache-2.0)
- eslint-visitor-keys 4.2.1 (Apache-2.0)
- espree 10.4.0 (BSD-2-Clause)
- esquery 1.7.0 (BSD-3-Clause)
- esrecurse 4.3.0 (BSD-2-Clause)
- estraverse 5.3.0 (BSD-2-Clause)
- estree-walker 3.0.3 (MIT)
- esutils 2.0.3 (BSD-2-Clause)
- expect-type 1.4.0 (Apache-2.0)
- fast-deep-equal 3.1.3 (MIT)
- fast-glob 3.3.3 (MIT)
- fast-json-stable-stringify 2.1.0 (MIT)
- fast-levenshtein 2.0.6 (MIT)
- fastq 1.20.3 (ISC)
- file-entry-cache 8.0.0 (MIT)
- fill-range 7.1.1 (MIT)
- find-up 5.0.0 (MIT)
- flat-cache 4.0.1 (MIT)
- flatted 3.4.4 (ISC)
- form-data 4.0.6 (MIT)
- fsevents 2.3.3 (MIT)
- function-bind 1.1.2 (MIT)
- gensync 1.0.0-beta.2 (MIT)
- get-intrinsic 1.3.0 (MIT)
- get-proto 1.0.1 (MIT)
- glob-parent 5.1.2 (ISC)
- glob-parent 6.0.2 (ISC)
- globals 14.0.0 (MIT)
- gopd 1.2.0 (MIT)
- graphemer 1.4.0 (MIT)
- has-flag 4.0.0 (MIT)
- has-symbols 1.1.0 (MIT)
- has-tostringtag 1.0.2 (MIT)
- hasown 2.0.4 (MIT)
- html-encoding-sniffer 4.0.0 (MIT)
- http-proxy-agent 7.0.2 (MIT)
- https-proxy-agent 7.0.6 (MIT)
- iconv-lite 0.6.3 (MIT)
- ignore 5.3.2 (MIT)
- import-fresh 3.3.1 (MIT)
- imurmurhash 0.1.4 (MIT)
- is-extglob 2.1.1 (MIT)
- is-glob 4.0.3 (MIT)
- is-number 7.0.0 (MIT)
- is-potential-custom-element-name 1.0.1 (MIT)
- isexe 2.0.0 (ISC)
- js-tokens 4.0.0 (MIT)
- js-yaml 4.3.2 (MIT)
- jsdom 25.0.1 (MIT)
- jsesc 3.1.0 (MIT)
- json-buffer 3.0.1 (MIT)
- json-schema-traverse 0.4.1 (MIT)
- json-stable-stringify-without-jsonify 1.0.1 (MIT)
- json5 2.2.3 (MIT)
- keyv 4.5.4 (MIT)
- levn 0.4.1 (MIT)
- locate-path 6.0.0 (MIT)
- lodash.merge 4.6.2 (MIT)
- loose-envify 1.4.0 (MIT)
- loupe 3.2.1 (MIT)
- lru-cache 10.4.3 (ISC)
- lru-cache 5.1.1 (ISC)
- lz-string 1.5.0 (MIT)
- magic-string 0.30.21 (MIT)
- math-intrinsics 1.1.0 (MIT)
- merge2 1.4.1 (MIT)
- micromatch 4.0.8 (MIT)
- mime-db 1.52.0 (MIT)
- mime-types 2.1.35 (MIT)
- minimatch 3.1.5 (ISC)
- minimatch 9.0.9 (ISC)
- ms 2.1.3 (MIT)
- nanoid 3.3.20 (MIT)
- natural-compare 1.4.0 (MIT)
- node-releases 2.0.64 (MIT)
- nwsapi 2.2.28 (MIT)
- optionator 0.9.4 (MIT)
- p-limit 3.1.0 (MIT)
- p-locate 5.0.0 (MIT)
- parent-module 1.0.1 (MIT)
- parse5 7.3.0 (MIT)
- path-exists 4.0.0 (MIT)
- path-key 3.1.1 (MIT)
- pathe 1.1.2 (MIT)
- pathval 2.0.1 (MIT)
- picocolors 1.1.1 (ISC)
- picomatch 2.3.2 (MIT)
- postcss 8.5.33 (MIT)
- prelude-ls 1.2.1 (MIT)
- pretty-format 27.5.1 (MIT)
- punycode 2.3.1 (MIT)
- queue-microtask 1.2.3 (MIT)
- react 18.3.1 (MIT)
- react-dom 18.3.1 (MIT)
- react-is 17.0.2 (MIT)
- react-refresh 0.14.2 (MIT)
- resolve-from 4.0.0 (MIT)
- reusify 1.1.0 (MIT)
- rollup 4.68.3 (MIT)
- rrweb-cssom 0.7.1 (MIT)
- rrweb-cssom 0.8.0 (MIT)
- run-parallel 1.2.0 (MIT)
- safer-buffer 2.1.2 (MIT)
- saxes 6.0.0 (ISC)
- scheduler 0.23.2 (MIT)
- semver 6.3.2 (ISC)
- semver 7.8.6 (ISC)
- shebang-command 2.0.0 (MIT)
- shebang-regex 3.0.0 (MIT)
- siginfo 2.0.0 (ISC)
- source-map-js 1.2.2 (BSD-3-Clause)
- stackback 0.0.2 (MIT)
- std-env 3.10.0 (MIT)
- strip-json-comments 3.1.1 (MIT)
- supports-color 7.2.0 (MIT)
- symbol-tree 3.2.4 (MIT)
- tinybench 2.9.0 (MIT)
- tinyexec 0.3.2 (MIT)
- tinypool 1.1.1 (MIT)
- tinyrainbow 1.2.0 (MIT)
- tinyspy 3.0.2 (MIT)
- tldts 6.1.86 (MIT)
- tldts-core 6.1.86 (MIT)
- to-regex-range 5.0.1 (MIT)
- tough-cookie 5.1.2 (BSD-3-Clause)
- tr46 5.1.1 (MIT)
- ts-api-utils 1.4.3 (MIT)
- type-check 0.4.0 (MIT)
- typescript 5.6.3 (Apache-2.0)
- typescript-eslint 8.15.0 (MIT)
- update-browserslist-db 1.3.4 (MIT)
- uplot 1.6.32 (MIT)
- uri-js 4.4.1 (BSD-2-Clause)
- vite 5.4.11 (MIT)
- vite-node 2.1.5 (MIT)
- vitest 2.1.5 (MIT)
- w3c-xmlserializer 5.0.0 (MIT)
- webidl-conversions 7.0.0 (BSD-2-Clause)
- whatwg-encoding 3.1.1 (MIT)
- whatwg-mimetype 4.0.0 (MIT)
- whatwg-url 14.2.0 (MIT)
- which 2.0.2 (ISC)
- why-is-node-running 2.3.0 (MIT)
- word-wrap 1.2.5 (MIT)
- ws 8.23.0 (MIT)
- xml-name-validator 5.0.0 (Apache-2.0)
- xmlchars 2.2.0 (MIT)
- yallist 3.1.1 (ISC)
- yocto-queue 0.1.0 (MIT)
- zod 3.23.8 (MIT)
--- copy end ---

---

*Consistency note:* `docs/REFERENCES.md` in the repository carries the same
list; regenerate with `docs/submission/list-deps.sh` if the lockfiles change
before submission.
