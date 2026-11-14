//! Tests for the IRSA SIA2 + cutout client (`spherex_backend::irsa`).
//!
//! All fixtures are synthetic: VOTable documents are built as strings
//! and a local axum stub serves them plus synthetic MEF cutouts. No
//! test reaches the real IRSA except `live_irsa_smoke`, which is
//! opt-in via `SPHEREX_LIVE=1`.

mod fixtures;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{routing::get, Router};
use fixtures::*;
use spherex_backend::error::AppError;
use spherex_backend::irsa::{
    cutout_url, dedup_by_url, em_to_um, fov_to_cutout_size, observations_from_votable,
    parse_votable, select_frames, IrsaClient, IrsaConfig, IrsaError, Observation, WarmTarget,
};

// ---------------------------------------------------------------
// Synthetic VOTable fixtures
// ---------------------------------------------------------------

/// A minimal SIA2 VOTable: one data table with the columns the client
/// reads, `urls.len()` rows (em in metres, t in MJD days as IRSA emits).
fn sia_votable(urls: &[String]) -> String {
    let mut rows = String::new();
    for (i, u) in urls.iter().enumerate() {
        let t0 = 61000.0 + i as f64;
        rows.push_str(&format!(
            "<TR><TD>{t0}</TD><TD>{}</TD><TD>1.10e-6</TD><TD>1.117e-6</TD>\
             <TD>SPHEREx-D{}</TD><TD>{u}</TD></TR>",
            t0 + 0.0014,
            i % 6 + 1
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\
         <VOTABLE version=\"1.3\" xmlns=\"http://www.ivoa.net/xml/VOTable/v1.3\">\
         <RESOURCE type=\"meta\"><DESCRIPTION>service metadata</DESCRIPTION></RESOURCE>\
         <RESOURCE type=\"results\"><TABLE>\
         <FIELD name=\"t_min\" datatype=\"double\" unit=\"d\"/>\
         <FIELD name=\"t_max\" datatype=\"double\" unit=\"d\"/>\
         <FIELD name=\"em_min\" datatype=\"double\" unit=\"m\"/>\
         <FIELD name=\"em_max\" datatype=\"double\" unit=\"m\"/>\
         <FIELD name=\"energy_bandpassname\" datatype=\"char\" arraysize=\"*\"/>\
         <FIELD name=\"access_url\" datatype=\"char\" arraysize=\"*\"/>\
         <DATA><TABLEDATA>{rows}</TABLEDATA></DATA></TABLE></RESOURCE></VOTABLE>"
    )
}

/// A valid-but-empty data table (position with no coverage).
const EMPTY_VOTABLE: &str = "<?xml version=\"1.0\"?><VOTABLE><RESOURCE><TABLE>\
    <FIELD name=\"t_min\" datatype=\"double\" unit=\"d\"/>\
    <FIELD name=\"access_url\" datatype=\"char\" arraysize=\"*\"/>\
    <DATA><TABLEDATA></TABLEDATA></DATA></TABLE></RESOURCE></VOTABLE>";

/// Minimal SPHEREx MEF cutout (4x4) parseable by `fits::parse_frame`.
fn fake_cutout() -> Vec<u8> {
    let (nx, ny) = (4usize, 4usize);
    let wcs = vec![
        card_str("CTYPE1", "RA---TAN"),
        card_str("CTYPE2", "DEC--TAN"),
        card_flt("CRPIX1", 2.5),
        card_flt("CRPIX2", 2.5),
        card_flt("CRVAL1", 337.878),
        card_flt("CRVAL2", -5.189),
        card_flt("CD1_1", -0.001),
        card_flt("CD1_2", 0.0),
        card_flt("CD2_1", 0.0),
        card_flt("CD2_2", 0.001),
        card_int("DETECTOR", 3),
        card_str("BUNIT", "MJy / sr"),
    ];
    let mut f = primary_hdu();
    f.extend(image_hdu(
        "IMAGE",
        -32,
        nx,
        ny,
        &f32_data(&vec![1.0; nx * ny]),
        &wcs,
    ));
    f.extend(image_hdu(
        "FLAGS",
        32,
        nx,
        ny,
        &i32_data(&vec![0; nx * ny]),
        &[],
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
        &[1, 4],
        &[1, 4],
        &[1.10, 0.05, 1.20, 0.05, 1.15, 0.05, 1.25, 0.05],
        &[],
    ));
    f
}

// ---------------------------------------------------------------
// Local axum stub (never the real IRSA)
// ---------------------------------------------------------------

/// Shared stub state: counts hits, fails the first `sia_fail_first`
/// SIA requests with 503, measures in-flight cutout requests.
struct Stub {
    sia_hits: AtomicUsize,
    sia_fail_first: usize,
    sia_body: String,
    ibe_hits: AtomicUsize,
    in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
    ibe_delay: Duration,
    fits: Vec<u8>,
}

async fn sia_handler(State(s): State<Arc<Stub>>) -> Response {
    let n = s.sia_hits.fetch_add(1, Ordering::SeqCst) + 1;
    if n <= s.sia_fail_first {
        return (StatusCode::SERVICE_UNAVAILABLE, "busy").into_response();
    }
    (StatusCode::OK, s.sia_body.clone()).into_response()
}

async fn ibe_handler(State(s): State<Arc<Stub>>) -> Response {
    s.ibe_hits.fetch_add(1, Ordering::SeqCst);
    let cur = s.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
    s.max_in_flight.fetch_max(cur, Ordering::SeqCst);
    tokio::time::sleep(s.ibe_delay).await;
    s.in_flight.fetch_sub(1, Ordering::SeqCst);
    (StatusCode::OK, s.fits.clone()).into_response()
}

/// Bind a stub on an ephemeral port; returns (base_url, stub state).
async fn start_stub(mut stub: Stub) -> (String, Arc<Stub>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stub");
    let addr: SocketAddr = listener.local_addr().expect("local addr");
    let base = format!("http://{addr}");
    if stub.sia_body.is_empty() {
        let urls: Vec<String> = (0..3).map(|i| format!("{{base}}/ibe/f{i}.fits")).collect();
        stub.sia_body = sia_votable(&urls);
    }
    // Fixtures template access_url with the literal "{base}" so they can
    // be built before the ephemeral port is known.
    stub.sia_body = stub.sia_body.replace("{base}", &base);
    let stub = Arc::new(stub);
    let app = Router::new()
        .route("/SIA", get(sia_handler))
        .route("/ibe/*rest", get(ibe_handler))
        .with_state(stub.clone());
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (base, stub)
}

fn stub_with_urls(urls: Vec<String>, fits: Vec<u8>) -> Stub {
    Stub {
        sia_hits: AtomicUsize::new(0),
        sia_fail_first: 0,
        sia_body: sia_votable(&urls),
        ibe_hits: AtomicUsize::new(0),
        in_flight: AtomicUsize::new(0),
        max_in_flight: AtomicUsize::new(0),
        ibe_delay: Duration::from_millis(0),
        fits,
    }
}

/// Test config pointed at the stub, with fast retries.
fn test_cfg(base: &str) -> IrsaConfig {
    IrsaConfig {
        base_url: base.to_string(),
        collections: vec!["spherex_qr3".to_string()],
        retry_base_delay: Duration::from_millis(1),
        request_timeout: Duration::from_secs(5),
        ..IrsaConfig::default()
    }
}

fn obs_with_url(url: &str, t_mjd: f64) -> Observation {
    Observation {
        t_min_mjd: t_mjd,
        t_max_mjd: t_mjd + 0.001,
        em_min_um: 1.0,
        em_max_um: 1.2,
        band_name: "SPHEREx-D1".to_string(),
        access_url: url.to_string(),
        collection: "spherex_qr3".to_string(),
    }
}

// ---------------------------------------------------------------
// Pure parsing / selection tests
// ---------------------------------------------------------------

#[test]
fn votable_parses_table_with_entities() {
    let xml = "<?xml version=\"1.0\"?><!DOCTYPE VOTABLE [ <!ENTITY x \"y\"> ]>\
        <VOTABLE><RESOURCE><TABLE>\
        <FIELD name=\"a\" datatype=\"double\" unit=\"m\"/>\
        <FIELD name=\"b\" datatype=\"char\" arraysize=\"*\"/>\
        <DATA><TABLEDATA>\
        <TR><TD>1.5</TD><TD>x&amp;y&quot;z</TD></TR>\
        <TR><TD/><TD><![CDATA[c<d>]]></TD></TR>\
        </TABLEDATA></DATA></TABLE></RESOURCE></VOTABLE>";
    let tables = parse_votable(xml).expect("parse");
    assert_eq!(tables.len(), 1);
    let t = &tables[0];
    assert_eq!(t.fields.len(), 2);
    assert_eq!(t.fields[0].name, "a");
    assert_eq!(t.fields[0].unit.as_deref(), Some("m"));
    assert_eq!(t.rows.len(), 2);
    assert_eq!(t.rows[0][1], "x&y\"z");
    assert_eq!(t.rows[1][0], "");
    assert_eq!(t.rows[1][1], "c<d>");
}

#[test]
fn sia_rows_become_observations_with_units() {
    let urls = vec!["https://irsa.ipac.caltech.edu/ibe/x.fits".to_string()];
    let obs = observations_from_votable(&sia_votable(&urls), "spherex_qr3").expect("obs");
    assert_eq!(obs.len(), 1);
    let o = &obs[0];
    assert!((o.t_min_mjd - 61000.0).abs() < 1e-9);
    // em was declared in metres -> micrometres.
    assert!((o.em_min_um - 1.10).abs() < 1e-9);
    assert!((o.em_max_um - 1.117).abs() < 1e-9);
    assert_eq!(o.band_name, "SPHEREx-D1");
    assert_eq!(o.collection, "spherex_qr3");
    assert_eq!(o.id().len(), 16);
}

#[test]
fn sia_skips_rows_without_access_url() {
    let xml = "<VOTABLE><RESOURCE><TABLE>\
        <FIELD name=\"t_min\" datatype=\"double\" unit=\"d\"/>\
        <FIELD name=\"access_url\" datatype=\"char\" arraysize=\"*\"/>\
        <DATA><TABLEDATA>\
        <TR><TD>1.0</TD><TD/></TR>\
        <TR><TD>2.0</TD><TD>http://x/f.fits</TD></TR>\
        </TABLEDATA></DATA></TABLE></RESOURCE></VOTABLE>";
    let obs = observations_from_votable(xml, "c").expect("obs");
    assert_eq!(obs.len(), 1);
    assert_eq!(obs[0].access_url, "http://x/f.fits");
}

#[test]
fn em_unit_conversion() {
    assert!((em_to_um(1.117e-6, Some("m")) - 1.117).abs() < 1e-9);
    assert!((em_to_um(1.117, Some("um")) - 1.117).abs() < 1e-9);
    assert!((em_to_um(1117.0, Some("nm")) - 1.117).abs() < 1e-6);
    // Unknown/absent unit: sub-millimetre values read as metres.
    assert!((em_to_um(1.117e-6, None) - 1.117).abs() < 1e-9);
    assert!((em_to_um(1.117, None) - 1.117).abs() < 1e-9);
}

#[test]
fn dedup_keeps_first_collection_hit() {
    let a = obs_with_url("http://x/same.fits", 1.0);
    let mut b = obs_with_url("http://x/same.fits", 1.0);
    b.collection = "spherex_dr1".to_string();
    let c = obs_with_url("http://x/other.fits", 2.0);
    let out = dedup_by_url(vec![a, b, c]);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].collection, "spherex_qr3");
}

#[test]
fn fov_maps_to_square_degrees() {
    // IBE cutouts: size defaults to degrees, one value = square
    // (https://irsa.ipac.caltech.edu/ibe/cutouts.html).
    assert_eq!(fov_to_cutout_size(0.05), "0.05deg");
    let url = cutout_url("https://h/x.fits", 337.8783221, -5.1891588, 0.05);
    assert_eq!(
        url,
        "https://h/x.fits?center=337.8783221,-5.1891588&size=0.05deg"
    );
    let url2 = cutout_url("https://h/x.fits?raw=1", 1.0, 2.0, 0.01);
    assert!(url2.contains("&center=1,2&size=0.01deg"));
}

#[test]
fn select_frames_spreads_over_time_newest_first() {
    let obs: Vec<Observation> = (0..100)
        .map(|i| obs_with_url(&format!("{{base}}/ibe/f{i}.fits"), 61000.0 + i as f64))
        .collect();
    let picked = select_frames(&obs, 10);
    assert_eq!(picked.len(), 10);
    // Evenly spaced indices 0,11,22,...,99 -> epochs 99,88,...,0 desc.
    assert!((picked[0].t_min_mjd - 61099.0).abs() < 1e-9);
    assert!((picked[9].t_min_mjd - 61000.0).abs() < 1e-9);
    for w in picked.windows(2) {
        assert!(w[0].t_min_mjd > w[1].t_min_mjd);
    }
    // Below the cap: all kept, still newest first.
    let all = select_frames(&obs[..5], 10);
    assert_eq!(all.len(), 5);
    assert!((all[0].t_min_mjd - 61004.0).abs() < 1e-9);
}

// ---------------------------------------------------------------
// HTTP client tests against the local stub
// ---------------------------------------------------------------

#[tokio::test]
async fn search_retries_on_503_then_succeeds() {
    let mut stub = stub_with_urls(vec!["{base}/ibe/f0.fits".to_string()], fake_cutout());
    stub.sia_fail_first = 2; // 503, 503, then 200
    let (base, stub) = start_stub(stub).await;
    let client = IrsaClient::new(test_cfg(&base)).expect("client");
    let obs = client.search(337.8783221, -5.1891588, 0.05).await;
    assert!(obs.is_ok(), "expected success after retries: {obs:?}");
    assert_eq!(stub.sia_hits.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn unavailable_after_retries_maps_to_502() {
    let mut stub = stub_with_urls(vec![], fake_cutout());
    stub.sia_fail_first = usize::MAX;
    let (base, _stub) = start_stub(stub).await;
    let client = IrsaClient::new(test_cfg(&base)).expect("client");
    let err = client
        .search(337.8783221, -5.1891588, 0.05)
        .await
        .expect_err("must fail");
    assert!(matches!(err, IrsaError::Unavailable { attempts: 3, .. }));
    let app: AppError = err.into();
    assert_eq!(app.status_code(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn empty_result_maps_to_404() {
    let mut stub = stub_with_urls(vec![], fake_cutout());
    stub.sia_body = EMPTY_VOTABLE.to_string();
    let (base, _stub) = start_stub(stub).await;
    let client = IrsaClient::new(test_cfg(&base)).expect("client");
    let err = client
        .search(337.8783221, -5.1891588, 0.05)
        .await
        .expect_err("must fail");
    assert!(matches!(err, IrsaError::NoObservations));
    let app: AppError = err.into();
    assert_eq!(app.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn dedup_across_collections_over_http() {
    // Two configured collections returning the same access_url merge
    // into a single observation.
    let (base, _s) = {
        let stub = stub_with_urls(vec!["{base}/ibe/shared.fits".to_string()], fake_cutout());
        start_stub(stub).await
    };
    let mut cfg = test_cfg(&base);
    cfg.collections = vec!["spherex_dr1".to_string(), "spherex_qr3".to_string()];
    let client = IrsaClient::new(cfg).expect("client");
    let obs = client.search(10.0, -5.0, 0.05).await.expect("search");
    assert_eq!(obs.len(), 1);
    assert_eq!(obs[0].collection, "spherex_dr1"); // first wins
}

#[tokio::test]
async fn fetch_frames_decodes_cutouts() {
    let (base, _stub) = start_stub(stub_with_urls(
        (0..3).map(|i| format!("{{base}}/ibe/f{i}.fits")).collect(),
        fake_cutout(),
    ))
    .await;
    let client = IrsaClient::new(test_cfg(&base)).expect("client");
    let set = client
        .fetch_frames(337.8783221, -5.1891588, 0.05)
        .await
        .expect("frames");
    assert_eq!(set.frames.len(), 3);
    assert_eq!(set.total_observations, 3);
    assert!(!set.truncated);
    let f = &set.frames[0].frame;
    assert_eq!(f.header.detector, Some(3));
    assert_eq!(f.image.dim(), (4, 4));
    // Newest first.
    assert!(set.frames[0].observation.t_min_mjd > set.frames[2].observation.t_min_mjd);
}

#[tokio::test]
async fn concurrency_is_bounded_by_semaphore() {
    let urls: Vec<String> = (0..8).map(|i| format!("{{base}}/ibe/f{i}.fits")).collect();
    let mut stub = stub_with_urls(urls, fake_cutout());
    stub.ibe_delay = Duration::from_millis(60);
    let (base, stub) = start_stub(stub).await;
    let mut cfg = test_cfg(&base);
    cfg.max_concurrency = 2;
    cfg.max_frames = 8;
    let client = IrsaClient::new(cfg).expect("client");
    let set = client
        .fetch_frames(337.8783221, -5.1891588, 0.05)
        .await
        .expect("frames");
    assert_eq!(set.frames.len(), 8);
    assert_eq!(stub.ibe_hits.load(Ordering::SeqCst), 8);
    let peak = stub.max_in_flight.load(Ordering::SeqCst);
    assert!(peak <= 2, "semaphore bound violated: peak {peak}");
    assert!(peak >= 2, "expected parallelism to be used: peak {peak}");
}

#[tokio::test]
async fn body_size_cap_rejects_large_responses() {
    let (base, _stub) = start_stub(stub_with_urls(
        vec!["{base}/ibe/f0.fits".to_string()],
        fake_cutout(),
    ))
    .await;
    let mut cfg = test_cfg(&base);
    cfg.max_body_bytes = 4096; // smaller than the synthetic cutout
    let client = IrsaClient::new(cfg).expect("client");
    let obs = obs_with_url(&format!("{base}/ibe/big.fits"), 61000.0);
    let err = client
        .fetch_cutout(&obs, 337.8783221, -5.1891588, 0.05)
        .await
        .expect_err("must fail");
    assert!(matches!(err, IrsaError::BodyTooLarge { .. }), "got {err:?}");
}

#[tokio::test]
async fn cache_avoids_refetching() {
    let (base, stub) = start_stub(stub_with_urls(
        (0..3).map(|i| format!("{{base}}/ibe/f{i}.fits")).collect(),
        fake_cutout(),
    ))
    .await;
    let client = IrsaClient::new(test_cfg(&base)).expect("client");
    for _ in 0..2 {
        let set = client
            .fetch_frames(337.8783221, -5.1891588, 0.05)
            .await
            .expect("frames");
        assert_eq!(set.frames.len(), 3);
    }
    assert_eq!(stub.sia_hits.load(Ordering::SeqCst), 1, "search not cached");
    assert_eq!(
        stub.ibe_hits.load(Ordering::SeqCst),
        3,
        "cutouts not cached"
    );
}

#[tokio::test]
async fn max_frames_caps_and_reports_truncation() {
    let urls: Vec<String> = (0..6).map(|i| format!("{{base}}/ibe/f{i}.fits")).collect();
    let (base, _stub) = start_stub(stub_with_urls(urls, fake_cutout())).await;
    let mut cfg = test_cfg(&base);
    cfg.max_frames = 3;
    let client = IrsaClient::new(cfg).expect("client");
    let set = client
        .fetch_frames(337.8783221, -5.1891588, 0.05)
        .await
        .expect("frames");
    assert_eq!(set.frames.len(), 3);
    assert_eq!(set.total_observations, 6);
    assert!(set.truncated);
}

#[tokio::test]
async fn warm_prefetches_targets() {
    let (base, _stub) = start_stub(stub_with_urls(
        (0..2).map(|i| format!("{{base}}/ibe/f{i}.fits")).collect(),
        fake_cutout(),
    ))
    .await;
    let client = IrsaClient::new(test_cfg(&base)).expect("client");
    let report = client
        .warm(&[WarmTarget {
            ra_deg: 337.8783221,
            dec_deg: -5.1891588,
            fov_deg: 0.05,
        }])
        .await;
    assert_eq!(report.succeeded, 1);
    assert_eq!(report.failed, 0);
    assert_eq!(report.frames, 2);
}

#[tokio::test]
async fn invalid_coordinates_rejected_before_http() {
    let (base, stub) = start_stub(stub_with_urls(vec![], fake_cutout())).await;
    let client = IrsaClient::new(test_cfg(&base)).expect("client");
    let err = client
        .fetch_frames(400.0, -5.0, 0.05)
        .await
        .expect_err("must fail");
    assert!(matches!(err, IrsaError::InvalidQuery(_)));
    let app: AppError = err.into();
    assert_eq!(app.status_code(), StatusCode::BAD_REQUEST);
    assert_eq!(stub.sia_hits.load(Ordering::SeqCst), 0);
}

// ---------------------------------------------------------------
// Opt-in live smoke test (never runs by default)
// ---------------------------------------------------------------

/// Real end-to-end check against IRSA. Enable with `SPHEREX_LIVE=1`.
#[tokio::test]
async fn live_irsa_smoke() {
    if std::env::var("SPHEREX_LIVE").ok().as_deref() != Some("1") {
        return;
    }
    let client = IrsaClient::from_env().expect("client from env");
    let obs = client
        .search(337.8783221, -5.1891588, 0.05)
        .await
        .expect("live search");
    assert!(!obs.is_empty());
    // Some SIA-listed files legitimately fail per-cutout (422 "Arrays
    // do not overlap" when the square misses the array, transient
    // 503s under load). Probe a few; at least one must decode.
    let mut decoded = 0usize;
    for o in obs.iter().take(5) {
        match client.fetch_cutout(o, 337.8783221, -5.1891588, 0.05).await {
            Ok(f) => {
                let det = f.header.detector.expect("DETECTOR header");
                assert!((1..=6).contains(&det));
                decoded += 1;
            }
            Err(e) => eprintln!("live cutout failed (tolerated): {e}"),
        }
    }
    assert!(decoded >= 1, "no live cutout decoded");
}
