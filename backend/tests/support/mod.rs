//! Shared plumbing for the b6 security/robustness test battery
//! (`security_tests.rs` and `fuzz_tests.rs` both declare `mod support`).
//!
//! Everything here runs against a stubbed [`FrameSource`] with synthetic
//! MEF cutouts (see `tests/fixtures`) or a loopback-only stub server;
//! no test reaches the real IRSA.
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Write as IoWrite;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use axum::body::{to_bytes, Body, Bytes};
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode};
use axum::Router;
use serde_json::Value;
use spherex_backend::api::{build_router, AppState, FrameSource};
use spherex_backend::fits::{parse_frame, Frame};
use spherex_backend::irsa::{IrsaError, Observation};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tower::ServiceExt;
use tracing_subscriber::fmt::writer::MakeWriter;

use crate::fixtures::*;

/// Peer address injected as `ConnectInfo` on `oneshot` requests
/// (the governor's `SmartIpKeyExtractor` falls back to it).
pub const TEST_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

/// `error.code` values allowed by `shared/openapi-contract.json`.
pub const CONTRACT_CODES: [&str; 5] = [
    "invalid_parameter",
    "not_found",
    "rate_limited",
    "upstream_unavailable",
    "internal",
];

// ---------------------------------------------------------------------
// Deterministic PRNG (xorshift64*); fixed seeds -> reproducible runs.
// ---------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Uniform integer in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// Uniform double in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / 9_007_199_254_740_992.0
    }

    /// Uniform double in `[lo, hi)`.
    pub fn between(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }
}

// ---------------------------------------------------------------------
// Percent codec for building query strings by hand.
// ---------------------------------------------------------------------

/// Percent-encode every byte outside RFC 3986 `unreserved`.
pub fn pct_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

/// Lossy percent-decode (for the "no raw echo" invariant check only).
pub fn pct_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = u8::from_str_radix(&s[i + 1..i + 3], 16);
            match h {
                Ok(b) => {
                    out.push(b);
                    i += 3;
                }
                Err(_) => {
                    out.push(b'%');
                    i += 1;
                }
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ---------------------------------------------------------------------
// Synthetic upstream: observations + MEF cutouts via tests/fixtures.
// ---------------------------------------------------------------------

/// `n` synthetic observations covering the fixture position.
pub fn stub_observations(n: usize) -> Vec<Observation> {
    (0..n)
        .map(|i| Observation {
            t_min_mjd: 61000.0 + i as f64,
            t_max_mjd: 61000.0 + i as f64 + 0.0014,
            em_min_um: 1.10,
            em_max_um: 1.117,
            band_name: format!("SPHEREx-D{}", i % 6 + 1),
            access_url: format!("https://stub.invalid/ibe/data/cutout{i}.fits"),
            collection: "spherex_qr3".to_string(),
        })
        .collect()
}

/// FNV-1a 64-bit of `access_url`, matching `Observation::id_hash`.
pub fn obs_id_hash(o: &Observation) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in o.access_url.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Minimal MEF cutout (8x8) at the fixture position, constant `flux`
/// image, detector `detector`, constant wavelength map around `wave_um`.
pub fn cutout(flux: f32, detector: i64, wave_um: f32) -> Vec<u8> {
    let (nx, ny) = (8usize, 8usize);
    let wcs = vec![
        card_str("CTYPE1", "RA---TAN"),
        card_str("CTYPE2", "DEC--TAN"),
        card_flt("CRPIX1", 4.5),
        card_flt("CRPIX2", 4.5),
        card_flt("CRVAL1", 337.878),
        card_flt("CRVAL2", -5.189),
        card_flt("CD1_1", -0.001),
        card_flt("CD1_2", 0.0),
        card_flt("CD2_1", 0.0),
        card_flt("CD2_2", 0.001),
        card_int("DETECTOR", detector),
        card_str("BUNIT", "MJy / sr"),
    ];
    let flags_legend = vec![
        card_str("MSKN0000", "TRANSIENT"),
        card_str("MSKD0000", "Transient source in the pixel"),
        card_str("MSKN0001", "SATURATED"),
        card_str("MSKD0001", "Pixel saturated"),
    ];
    let mut f = primary_hdu();
    f.extend(image_hdu(
        "IMAGE",
        -32,
        nx,
        ny,
        &f32_data(&vec![flux; nx * ny]),
        &wcs,
    ));
    f.extend(image_hdu(
        "FLAGS",
        32,
        nx,
        ny,
        &i32_data(&vec![0; nx * ny]),
        &flags_legend,
    ));
    f.extend(image_hdu(
        "VARIANCE",
        -32,
        nx,
        ny,
        &f32_data(&vec![0.5; nx * ny]),
        &[],
    ));
    f.extend(image_hdu(
        "ZODI",
        -32,
        nx,
        ny,
        &f32_data(&vec![0.1; nx * ny]),
        &[],
    ));
    f.extend(wcs_wave_hdu(
        &[1, 8],
        &[1, 8],
        &[
            wave_um,
            0.05,
            wave_um + 0.01,
            0.05,
            wave_um,
            0.05,
            wave_um + 0.01,
            0.05,
        ],
        &[],
    ));
    f
}

/// Decrement-on-drop marker for in-flight stub calls.
struct FlightGuard {
    ctr: Arc<AtomicUsize>,
}

impl Drop for FlightGuard {
    fn drop(&mut self) {
        self.ctr.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Configurable stub `FrameSource`: fixed observation list, per-frame
/// bytes or failure, per-observation fetch latency, and live in-flight
/// counters for deadline/cancellation checks.
pub struct StubSource {
    pub observations: Vec<Observation>,
    pub frames: HashMap<u64, Arc<Frame>>,
    /// Error returned by every `fetch_cutout` call.
    pub fetch_fail: Option<IrsaError>,
    /// Error returned by `search`.
    pub search_error: Option<IrsaError>,
    /// Per-observation fetch latency keyed by [`obs_id_hash`].
    pub delays: HashMap<u64, Duration>,
    /// `fetch_cutout` calls currently inside the stub.
    pub in_flight: Arc<AtomicUsize>,
    /// Total `fetch_cutout` calls observed.
    pub fetch_calls: Arc<AtomicUsize>,
}

impl StubSource {
    /// Every observation decodes to `cutout(1.0, 3, 1.10)`.
    pub fn happy(n: usize) -> Self {
        let observations = stub_observations(n);
        let frames = observations
            .iter()
            .map(|o| {
                (
                    obs_id_hash(o),
                    Arc::new(parse_frame(&cutout(1.0, 3, 1.10)).expect("fixture parses")),
                )
            })
            .collect();
        Self {
            observations,
            frames,
            fetch_fail: None,
            search_error: None,
            delays: HashMap::new(),
            in_flight: Arc::new(AtomicUsize::new(0)),
            fetch_calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

/// `IrsaError` is not `Clone`; rebuild the variants tests use.
pub fn clone_irsa_error(e: &IrsaError) -> IrsaError {
    match e {
        IrsaError::NoObservations => IrsaError::NoObservations,
        IrsaError::Unavailable { attempts, detail } => IrsaError::Unavailable {
            attempts: *attempts,
            detail: detail.clone(),
        },
        IrsaError::Http { status } => IrsaError::Http { status: *status },
        IrsaError::BodyTooLarge { limit } => IrsaError::BodyTooLarge { limit: *limit },
        other => IrsaError::Internal(format!("{other}")),
    }
}

impl FrameSource for StubSource {
    async fn search(
        &self,
        _ra_deg: f64,
        _dec_deg: f64,
        _fov_deg: f64,
    ) -> Result<Arc<Vec<Observation>>, IrsaError> {
        if let Some(e) = &self.search_error {
            return Err(clone_irsa_error(e));
        }
        if self.observations.is_empty() {
            return Err(IrsaError::NoObservations);
        }
        Ok(Arc::new(self.observations.clone()))
    }

    async fn fetch_cutout(
        &self,
        observation: Observation,
        _ra_deg: f64,
        _dec_deg: f64,
        _fov_deg: f64,
    ) -> Result<Arc<Frame>, IrsaError> {
        self.fetch_calls.fetch_add(1, Ordering::SeqCst);
        self.in_flight.fetch_add(1, Ordering::SeqCst);
        let _guard = FlightGuard {
            ctr: Arc::clone(&self.in_flight),
        };
        if let Some(d) = self.delays.get(&obs_id_hash(&observation)) {
            tokio::time::sleep(*d).await;
        }
        if let Some(e) = &self.fetch_fail {
            return Err(clone_irsa_error(e));
        }
        self.frames
            .get(&obs_id_hash(&observation))
            .cloned()
            .ok_or(IrsaError::Http { status: 422 })
    }
}

// ---------------------------------------------------------------------
// App state / router / request helpers
// ---------------------------------------------------------------------

pub fn state(source: StubSource) -> AppState<StubSource> {
    AppState {
        source: Arc::new(source),
        collections: Arc::new(vec!["spherex_qr3".to_string()]),
        static_dir: PathBuf::from("frontend/dist"),
        allowed_origins: Arc::new(Vec::new()),
        output_npix: 16,
        max_frames_cap: 60,
        request_deadline: Duration::from_secs(5),
        // High enough that the fuzz battery never trips the governor.
        rate_limit_rps: 1_000_000,
    }
}

pub fn app<S: FrameSource>(state: AppState<S>) -> Router {
    build_router(state)
}

pub fn hdr(name: &'static str, value: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static(name),
        HeaderValue::from_str(value).expect("header value"),
    )
}

/// One request through the router; `None` when the URI string cannot be
/// represented in an `http::Request` at all (e.g. invalid characters).
pub async fn call(
    app: &Router,
    method: &Method,
    uri: &str,
    headers: &[(HeaderName, HeaderValue)],
) -> Option<(StatusCode, HeaderMap, Bytes)> {
    let mut b = Request::builder().method(method.clone()).uri(uri);
    for (k, v) in headers {
        b = b.header(k.clone(), v.clone());
    }
    let mut req = b.body(Body::empty()).ok()?;
    req.extensions_mut().insert(ConnectInfo(TEST_ADDR));
    let res = app.clone().oneshot(req).await.expect("router answers");
    let status = res.status();
    let headers = res.headers().clone();
    let body = to_bytes(res.into_body(), 64 * 1024 * 1024)
        .await
        .expect("body reads");
    Some((status, headers, body))
}

/// Same as [`call`] but a panic inside the handler is reported as
/// `Err` instead of aborting the test (fuzz-harness wrapper).
pub async fn call_guarded(
    app: &Router,
    method: &Method,
    uri: &str,
    headers: &[(HeaderName, HeaderValue)],
) -> Option<Result<(StatusCode, HeaderMap, Bytes), String>> {
    let mut b = Request::builder().method(method.clone()).uri(uri);
    for (k, v) in headers {
        b = b.header(k.clone(), v.clone());
    }
    let mut req = b.body(Body::empty()).ok()?;
    req.extensions_mut().insert(ConnectInfo(TEST_ADDR));
    let app = app.clone();
    let joined = tokio::spawn(async move {
        let res = app.oneshot(req).await.expect("router answers");
        let status = res.status();
        let headers = res.headers().clone();
        let body = to_bytes(res.into_body(), 64 * 1024 * 1024)
            .await
            .expect("body reads");
        (status, headers, body)
    })
    .await;
    Some(joined.map_err(|e| {
        if e.is_panic() {
            format!("handler panicked: {e}")
        } else {
            format!("request task failed: {e}")
        }
    }))
}

/// Validate that `body` is a contract `ErrorBody`; returns the code.
pub fn check_error_body(body: &[u8]) -> Result<String, String> {
    let v: Value = serde_json::from_slice(body)
        .map_err(|e| format!("error body is not JSON: {e}; body head: {body:?}"))?;
    let err = v
        .get("error")
        .and_then(|e| e.as_object())
        .ok_or_else(|| format!("missing error object: {v}"))?;
    let code = err
        .get("code")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing error.code: {v}"))?;
    if !CONTRACT_CODES.contains(&code) {
        return Err(format!("non-contract error code {code:?}"));
    }
    match err.get("message") {
        Some(Value::String(m)) if !m.is_empty() => Ok(code.to_string()),
        _ => Err(format!("missing/empty error.message: {v}")),
    }
}

// ---------------------------------------------------------------------
// Real loopback server + raw TCP exchange (HTTP-layer robustness).
// ---------------------------------------------------------------------

/// Serve `state` on an ephemeral loopback port (ConnectInfo wired like
/// `main`); returns the bound address.
pub async fn spawn_server<S: FrameSource>(state: AppState<S>) -> SocketAddr {
    let app = build_router(state);
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0u16))
        .await
        .expect("bind test server");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let _ = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await;
    });
    addr
}

/// Result of a raw TCP exchange.
#[derive(Debug)]
pub struct RawAnswer {
    /// Bytes received before EOF or the read budget expiring.
    pub bytes: Vec<u8>,
    /// True when the read side timed out before EOF.
    pub timed_out: bool,
}

/// Write `request` verbatim, half-close the write side and collect the
/// response until EOF or `budget` elapses.
pub async fn raw_exchange(addr: SocketAddr, request: &[u8], budget: Duration) -> RawAnswer {
    let mut s = match TcpStream::connect(addr).await {
        Ok(s) => s,
        Err(_) => {
            return RawAnswer {
                bytes: Vec::new(),
                timed_out: true,
            }
        }
    };
    if s.write_all(request).await.is_err() {
        return RawAnswer {
            bytes: Vec::new(),
            timed_out: false,
        };
    }
    let _ = s.shutdown().await;
    let mut out = Vec::new();
    match tokio::time::timeout(budget, s.read_to_end(&mut out)).await {
        Ok(_) => RawAnswer {
            bytes: out,
            timed_out: false,
        },
        Err(_) => RawAnswer {
            bytes: out,
            timed_out: true,
        },
    }
}

/// Parse the status code of a raw HTTP response status line.
pub fn status_line_code(bytes: &[u8]) -> Option<u16> {
    let end = bytes.iter().position(|&b| b == b'\n')?;
    let line = std::str::from_utf8(&bytes[..end]).ok()?;
    let mut parts = line.split_whitespace();
    if !parts.next()?.starts_with("HTTP/") {
        return None;
    }
    parts.next()?.parse().ok()
}

// ---------------------------------------------------------------------
// Temp static dir (SPA fallback / traversal checks).
// ---------------------------------------------------------------------

pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!("spherex-b6-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("mkdir temp static dir");
        Self(p)
    }

    /// Write `rel` (relative path) inside the dir; parents created.
    pub fn write(&self, rel: &str, content: &str) {
        let f = self.0.join(rel);
        if let Some(parent) = f.parent() {
            std::fs::create_dir_all(parent).expect("mkdir parent");
        }
        std::fs::write(f, content).expect("write temp file");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ---------------------------------------------------------------------
// /proc/self RSS sampler (memory-growth bound).
// ---------------------------------------------------------------------

/// Current resident set in bytes (Linux `/proc/self/status`), 0 on error.
pub fn rss_bytes() -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines()
        .find(|l| l.starts_with("VmRSS"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse::<u64>().ok())
        .map(|kb| kb * 1024)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------
// In-process `tracing` capture (leak checks).
// ---------------------------------------------------------------------

/// Global log sink installed once per test binary.
#[derive(Clone, Default)]
pub struct LogSink {
    buf: Arc<Mutex<Vec<u8>>>,
}

/// `MakeWriter` handle writing into the sink.
pub struct SinkWriter(Arc<Mutex<Vec<u8>>>);

impl IoWrite for SinkWriter {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("sink lock").extend_from_slice(b);
        Ok(b.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogSink {
    type Writer = SinkWriter;

    fn make_writer(&'a self) -> SinkWriter {
        SinkWriter(Arc::clone(&self.buf))
    }
}

impl LogSink {
    /// Install the global tracing subscriber once; every caller gets a
    /// handle to the same buffer.
    pub fn install() -> Self {
        static SINK: OnceLock<LogSink> = OnceLock::new();
        SINK.get_or_init(|| {
            let sink = LogSink::default();
            tracing_subscriber::fmt()
                .with_writer(sink.clone())
                .with_max_level(tracing::Level::TRACE)
                .with_ansi(false)
                .init();
            sink
        })
        .clone()
    }

    /// Everything captured so far.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.buf.lock().expect("sink lock")).into_owned()
    }

    /// Drop captured output (call at test start).
    pub fn clear(&self) {
        self.buf.lock().expect("sink lock").clear();
    }
}
