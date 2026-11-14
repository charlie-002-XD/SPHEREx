//! Integration tests for the HTTP API (task b5): `tower::oneshot`
//! requests against `build_router` with a stubbed [`FrameSource`]
//! (synthetic MEF cutouts via `tests/fixtures`). No network.

mod fixtures;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use fixtures::*;
use serde_json::{json, Value};
use spherex_backend::api::{build_router, AppState, FrameSource};
use spherex_backend::fits::{parse_frame, Frame};
use spherex_backend::irsa::{IrsaError, Observation};
use tower::ServiceExt;

// ---------------------------------------------------------------------
// Stub FrameSource (synthetic data, no IRSA)
// ---------------------------------------------------------------------

/// Build `n` synthetic observations covering the fixture position.
fn stub_observations(n: usize) -> Vec<Observation> {
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

/// Minimal MEF cutout (8x8) at the fixture position, detector 3,
/// constant image `flux`, constant wavelength map.
fn cutout(flux: f32, detector: i64, wave_um: f32) -> Vec<u8> {
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

/// Configurable stub: fixed observation list, per-index frame bytes or
/// failure, plus an artificial fetch latency for deadline tests.
struct StubSource {
    observations: Vec<Observation>,
    frames: HashMap<u64, Arc<Frame>>,
    fail: Option<IrsaError>,
    search_error: Option<IrsaError>,
    fetch_delay: Duration,
}

impl StubSource {
    /// Stub where every observation decodes to `cutout(1.0, 3, 1.10)`.
    fn happy(n: usize) -> Self {
        let observations = stub_observations(n);
        let frames = observations
            .iter()
            .map(|o| {
                (
                    o.id_hash_for_test(),
                    Arc::new(parse_frame(&cutout(1.0, 3, 1.10)).expect("fixture parses")),
                )
            })
            .collect();
        Self {
            observations,
            frames,
            fail: None,
            search_error: None,
            fetch_delay: Duration::ZERO,
        }
    }
}

trait ObsId {
    fn id_hash_for_test(&self) -> u64;
}

impl ObsId for Observation {
    /// FNV-1a of `access_url`, matching `Observation::id_hash` (private).
    fn id_hash_for_test(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in self.access_url.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
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
            return Err(clone_error(e));
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
        if !self.fetch_delay.is_zero() {
            tokio::time::sleep(self.fetch_delay).await;
        }
        if let Some(e) = &self.fail {
            return Err(clone_error(e));
        }
        self.frames
            .get(&observation.id_hash_for_test())
            .cloned()
            .ok_or(IrsaError::Http { status: 422 })
    }
}

/// `IrsaError` is not `Clone`; rebuild the few variants tests use.
fn clone_error(e: &IrsaError) -> IrsaError {
    match e {
        IrsaError::NoObservations => IrsaError::NoObservations,
        IrsaError::Unavailable { attempts, detail } => IrsaError::Unavailable {
            attempts: *attempts,
            detail: detail.clone(),
        },
        IrsaError::Http { status } => IrsaError::Http { status: *status },
        other => IrsaError::Internal(format!("{other}")),
    }
}

// ---------------------------------------------------------------------
// Test plumbing
// ---------------------------------------------------------------------

const TEST_ADDR: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 0);

fn state(source: StubSource) -> AppState<StubSource> {
    AppState {
        source: Arc::new(source),
        collections: Arc::new(vec!["spherex_qr3".to_string()]),
        static_dir: "frontend/dist".into(),
        allowed_origins: Arc::new(Vec::new()),
        output_npix: 16,
        max_frames_cap: 60,
        request_deadline: Duration::from_secs(5),
        rate_limit_rps: 1000,
    }
}

/// GET `uri` against a router built on `state`.
async fn get(state: AppState<StubSource>, uri: &str) -> (StatusCode, Value) {
    let app = build_router(state);
    let mut req = Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("request builds");
    req.extensions_mut().insert(ConnectInfo(TEST_ADDR));
    let res = app.oneshot(req).await.expect("router answers");
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), 16 * 1024 * 1024)
        .await
        .expect("body reads");
    let v: Value = serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({"_raw": "not json"}));
    (status, v)
}

fn error_code(v: &Value) -> &str {
    v["error"]["code"].as_str().unwrap_or_default()
}

// ---------------------------------------------------------------------
// 400s: validation never reaches the stub, never echoes input
// ---------------------------------------------------------------------

#[tokio::test]
async fn frames_rejects_ra_out_of_range() {
    let (status, v) = get(state(StubSource::happy(1)), "/api/v1/frames?ra=400&dec=0").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_code(&v), "invalid_parameter");
}

#[tokio::test]
async fn frames_rejects_dec_out_of_range() {
    let (status, v) = get(state(StubSource::happy(1)), "/api/v1/frames?ra=10&dec=-95").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_code(&v), "invalid_parameter");
}

#[tokio::test]
async fn frames_rejects_fov_out_of_range() {
    let (status, v) = get(
        state(StubSource::happy(1)),
        "/api/v1/frames?ra=10&dec=0&fov=10",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_code(&v), "invalid_parameter");
}

#[tokio::test]
async fn frames_rejects_injection_and_never_echoes_it() {
    let payload = "1%3B%20DROP%20TABLE%20users--";
    let (status, v) = get(
        state(StubSource::happy(1)),
        &format!("/api/v1/frames?ra={payload}&dec=0"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_code(&v), "invalid_parameter");
    let body = serde_json::to_string(&v).expect("serializes");
    assert!(
        !body.contains("DROP TABLE"),
        "error body must not echo raw input: {body}"
    );
}

#[tokio::test]
async fn frames_rejects_bad_mode_detector_max_frames() {
    for (q, case) in [
        ("ra=10&dec=0&mode=evil", "mode"),
        ("ra=10&dec=0&detector=7", "detector"),
        ("ra=10&dec=0&detector=abc", "detector"),
        ("ra=10&dec=0&max_frames=0", "max_frames"),
        ("ra=10&dec=0&max_frames=61", "max_frames"),
        ("ra=10&dec=0&subtract_zodi=maybe", "subtract_zodi"),
    ] {
        let (status, v) = get(state(StubSource::happy(1)), &format!("/api/v1/frames?{q}")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{case}");
        assert_eq!(error_code(&v), "invalid_parameter", "{case}");
    }
}

#[tokio::test]
async fn frames_requires_ra_and_dec() {
    for q in ["dec=0", "ra=10", ""] {
        let (status, v) = get(state(StubSource::happy(1)), &format!("/api/v1/frames?{q}")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{q}");
        assert_eq!(error_code(&v), "invalid_parameter", "{q}");
    }
}

#[tokio::test]
async fn spectrum_rejects_bad_aperture() {
    for q in ["aperture_px=2", "aperture_px=9", "aperture_px=abc"] {
        let (status, v) = get(
            state(StubSource::happy(1)),
            &format!("/api/v1/spectrum?ra=337.878&dec=-5.189&{q}"),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{q}");
        assert_eq!(error_code(&v), "invalid_parameter", "{q}");
    }
}

// ---------------------------------------------------------------------
// 200 shapes match the contract
// ---------------------------------------------------------------------

#[tokio::test]
async fn health_shape() {
    let (status, v) = get(state(StubSource::happy(0)), "/api/v1/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["status"], "ok");
    assert!(v["version"].is_string());
    assert_eq!(v["collections"], json!(["spherex_qr3"]));
}

#[tokio::test]
async fn targets_returns_five_presets() {
    let (status, v) = get(state(StubSource::happy(0)), "/api/v1/targets").await;
    assert_eq!(status, StatusCode::OK);
    let arr = v.as_array().expect("array of targets");
    assert_eq!(arr.len(), 5);
    for (t, (id, ra, dec, fov)) in arr.iter().zip([
        ("barnards-star", 269.452083, 4.693333, 0.1),
        ("epsilon-indi-b", 331.04375, -56.782778, 0.05),
        ("wise-0855", 133.795129, -7.245147, 0.05),
        ("rw-aurigae", 76.95625, 30.401389, 0.05),
        ("r-draconis", 248.1675, 66.754722, 0.05),
    ]) {
        assert_eq!(t["id"], id);
        assert!(t["name"].is_string());
        assert!(
            (t["ra_deg"].as_f64().unwrap_or_default() - ra).abs() < 1e-5,
            "{id}"
        );
        assert!(
            (t["dec_deg"].as_f64().unwrap_or_default() - dec).abs() < 1e-5,
            "{id}"
        );
        assert!(
            (t["fov_deg"].as_f64().unwrap_or_default() - fov).abs() < 1e-9,
            "{id}"
        );
    }
}

#[tokio::test]
async fn frames_raw_shape_matches_contract() {
    let (status, v) = get(
        state(StubSource::happy(3)),
        "/api/v1/frames?ra=337.878&dec=-5.189&fov=0.05&mode=raw",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{v}");
    assert_eq!(v["query"]["ra_deg"], 337.878);
    assert_eq!(v["query"]["dec_deg"], -5.189);
    assert_eq!(v["query"]["fov_deg"], 0.05);
    assert_eq!(v["query"]["mode"], "raw");
    assert_eq!(v["width"], 16);
    assert_eq!(v["height"], 16);
    assert!(v["unit"].is_string());
    assert_eq!(v["total_observations"], 3);
    assert_eq!(v["truncated"], false);
    let frames = v["frames"].as_array().expect("frames array");
    assert_eq!(frames.len(), 3);
    // Sorted by observation time (ascending).
    let mjds: Vec<f64> = frames
        .iter()
        .map(|f| f["obs_mjd_start"].as_f64().unwrap_or_default())
        .collect();
    assert!(mjds.windows(2).all(|w| w[0] <= w[1]), "{mjds:?}");
    for f in frames {
        for key in [
            "id",
            "obs_mjd_start",
            "obs_mjd_end",
            "detector",
            "wavelength_um",
            "collection",
            "wcs",
            "data_b64",
        ] {
            assert!(f.get(key).is_some(), "missing {key}");
        }
        assert_eq!(f["detector"], 3);
        assert_eq!(f["collection"], "spherex_qr3");
        assert_eq!(f["wcs"]["crpix"].as_array().map(Vec::len), Some(2));
        // data_b64 decodes to width*height*4 bytes of float32.
        let raw = STANDARD
            .decode(f["data_b64"].as_str().expect("b64 string"))
            .expect("decodes");
        assert_eq!(raw.len(), 16 * 16 * 4);
    }
}

#[tokio::test]
async fn frames_diff_coadd_stack_shapes() {
    // Three epochs on the same detector, same wavelength cluster.
    let (status, v) = get(
        state(StubSource::happy(3)),
        "/api/v1/frames?ra=337.878&dec=-5.189&mode=diff",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{v}");
    assert_eq!(v["query"]["mode"], "diff");
    // 3 members, first is the reference -> 2 differences.
    assert_eq!(v["frames"].as_array().map(Vec::len), Some(2));

    let (status, v) = get(
        state(StubSource::happy(3)),
        "/api/v1/frames?ra=337.878&dec=-5.189&mode=coadd",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{v}");
    assert_eq!(v["frames"].as_array().map(Vec::len), Some(1));

    let (status, v) = get(
        state(StubSource::happy(3)),
        "/api/v1/frames?ra=337.878&dec=-5.189&mode=stack",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{v}");
    assert_eq!(v["frames"].as_array().map(Vec::len), Some(1));
    assert_eq!(v["frames"][0]["detector"], 3);
}

#[tokio::test]
async fn frames_truncated_when_more_observations_exist() {
    let (status, v) = get(
        state(StubSource::happy(5)),
        "/api/v1/frames?ra=337.878&dec=-5.189&max_frames=2",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["total_observations"], 5);
    assert_eq!(v["frames"].as_array().map(Vec::len), Some(2));
    assert_eq!(v["truncated"], true);
}

#[tokio::test]
async fn frames_detector_filter() {
    let (status, v) = get(
        state(StubSource::happy(3)),
        "/api/v1/frames?ra=337.878&dec=-5.189&detector=3",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["frames"].as_array().map(Vec::len), Some(3));

    let (status, v) = get(
        state(StubSource::happy(3)),
        "/api/v1/frames?ra=337.878&dec=-5.189&detector=6",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["frames"].as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn frames_sexagesimal_coordinates() {
    // WISE 0855 coordinates from the challenge page.
    let (status, v) = get(
        state(StubSource::happy(1)),
        "/api/v1/frames?ra=08h55m10.831s&dec=-07d14m42.53s&fov=0.05",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{v}");
    assert!((v["query"]["ra_deg"].as_f64().unwrap_or_default() - 133.795129).abs() < 1e-5);
    assert!((v["query"]["dec_deg"].as_f64().unwrap_or_default() - (-7.245147)).abs() < 1e-5);
}

// ---------------------------------------------------------------------
// Error paths: 404, 502, deadline
// ---------------------------------------------------------------------

#[tokio::test]
async fn frames_404_when_no_observations() {
    let mut s = StubSource::happy(0);
    s.observations.clear();
    let (status, v) = get(state(s), "/api/v1/frames?ra=10&dec=0").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error_code(&v), "not_found");
}

#[tokio::test]
async fn frames_502_when_upstream_unavailable() {
    let mut s = StubSource::happy(0);
    s.search_error = Some(IrsaError::Unavailable {
        attempts: 3,
        detail: "HTTP 503".into(),
    });
    let (status, v) = get(state(s), "/api/v1/frames?ra=10&dec=0").await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(error_code(&v), "upstream_unavailable");
}

#[tokio::test]
async fn frames_deadline_returns_partial_or_502() {
    // All fetches hang past the deadline -> nothing decoded -> 502.
    let mut s = StubSource::happy(2);
    s.fetch_delay = Duration::from_secs(60);
    let mut st = state(s);
    st.request_deadline = Duration::from_millis(50);
    let (status, v) = get(st, "/api/v1/frames?ra=337.878&dec=-5.189").await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(error_code(&v), "upstream_unavailable");
}

#[tokio::test]
async fn spectrum_shape() {
    let (status, v) = get(
        state(StubSource::happy(2)),
        "/api/v1/spectrum?ra=337.878&dec=-5.189&aperture_px=3",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{v}");
    assert_eq!(v["ra_deg"], 337.878);
    assert_eq!(v["unit"], "MJy/sr");
    let points = v["points"].as_array().expect("points array");
    assert_eq!(points.len(), 2);
    for p in points {
        for key in ["wavelength_um", "value", "sigma", "obs_mjd", "detector"] {
            assert!(p.get(key).is_some(), "missing {key}");
        }
    }
}

#[tokio::test]
async fn spectrum_404_when_nothing_covers_point() {
    let mut s = StubSource::happy(0);
    s.observations.clear();
    let (status, v) = get(state(s), "/api/v1/spectrum?ra=337.878&dec=-5.189").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error_code(&v), "not_found");
}

// ---------------------------------------------------------------------
// Rate limiting: second request inside the window -> 429 JSON body
// ---------------------------------------------------------------------

#[tokio::test]
async fn rate_limit_returns_429_json() {
    let mut st = state(StubSource::happy(0));
    st.rate_limit_rps = 1;
    let app = build_router(st);
    for expected in [StatusCode::OK, StatusCode::TOO_MANY_REQUESTS] {
        let mut req = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .expect("request builds");
        req.extensions_mut().insert(ConnectInfo(TEST_ADDR));
        let res = app.clone().oneshot(req).await.expect("router answers");
        assert_eq!(res.status(), expected);
        if expected == StatusCode::TOO_MANY_REQUESTS {
            let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
                .await
                .expect("body reads");
            let v: Value = serde_json::from_slice(&bytes).expect("429 body is JSON");
            assert_eq!(error_code(&v), "rate_limited");
        }
    }
}
