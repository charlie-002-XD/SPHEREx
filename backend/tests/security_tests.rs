//! Security and robustness battery for the HTTP layer (task b6).
//!
//! Covers the parts `api_tests.rs`/`validation_tests.rs` do not: method
//! handling, static-server traversal, SPA fallback, CORS, rate limiting,
//! request-deadline/cancellation behaviour, resource bounds and
//! secret/internal-info leakage in error bodies and logs. A handful of
//! checks go through a real loopback TCP server to exercise hyper's
//! HTTP/1 parser (request-line/header size limits); everything else
//! uses `tower::oneshot` with the stub [`FrameSource`]. No network.

mod fixtures;
mod support;

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::http::{Method, StatusCode};
use serde_json::{json, Value};
use spherex_backend::api::AppState;
use spherex_backend::irsa::{IrsaClient, IrsaConfig};
use support::*;

// ---------------------------------------------------------------------
// 1. Security headers
// ---------------------------------------------------------------------

#[tokio::test]
async fn csp_frame_ancestors_and_no_xfo() {
    let dir = TempDir::new("csp");
    dir.write("index.html", "<html>SPHEREX-INDEX-MARKER</html>");
    let mut st = state(StubSource::happy(0));
    st.static_dir = dir.0.clone();
    let app = app(st);

    for uri in ["/api/v1/health", "/missing-page", "/"] {
        let (status, headers, _b) = call(&app, &Method::GET, uri, &[])
            .await
            .expect("request builds");
        assert!(
            status.is_success() || status.is_client_error(),
            "{uri}: {status}"
        );
        let csp = headers
            .get("content-security-policy")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(
            csp.contains("frame-ancestors"),
            "{uri}: CSP header missing frame-ancestors: {csp:?}"
        );
        assert!(
            headers.get("x-frame-options").is_none(),
            "{uri}: X-Frame-Options must not be set (framing is a feature)"
        );
    }
}

// ---------------------------------------------------------------------
// 2. Method handling: only GET exists on the API
// ---------------------------------------------------------------------

#[tokio::test]
async fn non_get_methods_on_api_get_405() {
    let app = app(state(StubSource::happy(1)));
    for method in [Method::POST, Method::PUT, Method::DELETE, Method::PATCH] {
        for uri in [
            "/api/v1/health",
            "/api/v1/targets",
            "/api/v1/frames?ra=1&dec=0",
            "/api/v1/spectrum?ra=1&dec=0",
        ] {
            let (status, headers, _b) =
                call(&app, &method, uri, &[]).await.expect("request builds");
            assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {uri}");
            let allow = headers
                .get("allow")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default();
            assert!(allow.contains("GET"), "{method} {uri}: allow={allow}");
            assert!(!allow.contains("POST"), "{method} {uri}: allow={allow}");
        }
    }
}

#[tokio::test]
async fn options_gets_informative_200_with_get_only_allow() {
    // axum's MethodRouter answers bare OPTIONS itself (200 + `allow`);
    // the CORS layer adds `access-control-allow-methods` limited to GET.
    let app = app(state(StubSource::happy(1)));
    for uri in ["/api/v1/health", "/api/v1/frames?ra=1&dec=0"] {
        let (status, headers, _b) = call(&app, &Method::OPTIONS, uri, &[])
            .await
            .expect("request builds");
        assert_eq!(status, StatusCode::OK, "OPTIONS {uri}");
        let allow = headers
            .get("allow")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_ascii_uppercase();
        assert!(allow.contains("GET"), "OPTIONS {uri}: allow={allow}");
        for m in ["POST", "PUT", "DELETE", "PATCH"] {
            assert!(!allow.contains(m), "OPTIONS {uri}: allow={allow}");
        }
        if let Some(acam) = headers
            .get("access-control-allow-methods")
            .and_then(|v| v.to_str().ok())
        {
            let acam = acam.to_ascii_uppercase();
            assert!(acam.contains("GET"), "OPTIONS {uri}: ACAM={acam}");
            for m in ["POST", "PUT", "DELETE", "PATCH"] {
                assert!(!acam.contains(m), "OPTIONS {uri}: ACAM={acam}");
            }
        }
    }
}

#[tokio::test]
async fn head_is_served_like_get() {
    // axum's `get()` routes also answer HEAD with an empty body.
    let app = app(state(StubSource::happy(0)));
    let (status, _h, body) = call(&app, &Method::HEAD, "/api/v1/health", &[])
        .await
        .expect("request builds");
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_empty(), "HEAD must have an empty body");
}

// ---------------------------------------------------------------------
// 3. Routing: API 404s, SPA fallback, path traversal
// ---------------------------------------------------------------------

#[tokio::test]
async fn unknown_api_paths_return_json_404() {
    let app = app(state(StubSource::happy(0)));
    for uri in [
        "/api/v1/nonexistent",
        "/api/v1/frames/extra",
        "/api/v1",
        "/api/v1/FRAMES?ra=1&dec=0",
        "/api/v1//", // inner path "//": still hits the API fallback
    ] {
        let (status, _h, body) = call(&app, &Method::GET, uri, &[])
            .await
            .expect("request builds");
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        let code = check_error_body(&body).unwrap_or_else(|e| panic!("{uri}: {e}"));
        assert_eq!(code, "not_found", "{uri}");
    }
}

/// `/api/...` outside the mounted `/api/v1` tree currently falls through
/// to the SPA fallback (`index.html`) instead of a JSON `ErrorBody` 404.
/// That includes `/api/v1/` (trailing slash): `nest` only matches
/// `/api/v1` exactly or `/api/v1/<non-empty>`. Keeping this pinned down
/// as a known deviation; see the handoff.
#[tokio::test]
#[ignore = "BUG: /api/ paths outside /api/v1 (and /api/v1/ itself) are served the SPA index.html, not a JSON 404"]
async fn other_api_prefixes_also_return_json_404() {
    let dir = TempDir::new("apiver");
    dir.write("index.html", "<html>SPHEREX-INDEX-MARKER</html>");
    let mut st = state(StubSource::happy(0));
    st.static_dir = dir.0.clone();
    let app = app(st);
    for uri in ["/api/v2/frames?ra=1&dec=0", "/api/", "/api/v9", "/api/v1/"] {
        let (status, _h, body) = call(&app, &Method::GET, uri, &[])
            .await
            .expect("request builds");
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        check_error_body(&body).unwrap_or_else(|e| panic!("{uri}: {e}"));
    }
}

#[tokio::test]
async fn spa_fallback_serves_index_html() {
    let dir = TempDir::new("spa");
    dir.write("index.html", "<html>SPHEREX-INDEX-MARKER</html>");
    let mut st = state(StubSource::happy(0));
    st.static_dir = dir.0.clone();
    let app = app(st);

    for uri in ["/", "/some/deep/link", "/frames?x=1", "/assets/../missing"] {
        let (status, _h, body) = call(&app, &Method::GET, uri, &[])
            .await
            .expect("request builds");
        assert_eq!(status, StatusCode::OK, "{uri}");
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("SPHEREX-INDEX-MARKER"), "{uri}: {text:?}");
    }
}

#[tokio::test]
async fn static_path_traversal_never_leaks_files() {
    // A readable file OUTSIDE the served tree, next to it.
    let base = TempDir::new("trav-base");
    base.write(
        "sibling-secret.txt",
        "B6-SIBLING-SECRET-d41d8cd98f00b204e980",
    );
    let static_dir = base.0.join("public");
    std::fs::create_dir_all(&static_dir).expect("mkdir static");
    std::fs::write(static_dir.join("index.html"), "<html>IDX-MARKER</html>").expect("write index");

    let mut st = state(StubSource::happy(0));
    st.static_dir = static_dir;
    let app = app(st);

    let uris = [
        "/..%2f..%2fetc%2fpasswd",
        "/%2e%2e%2f%2e%2e%2f%2e%2e%2fetc%2fpasswd",
        "/..%2fsibling-secret.txt",
        "/%2e%2e/sibling-secret.txt",
        "/../../etc/passwd", // raw, unencoded
        "/../../../etc/passwd",
        "/..%252f..%252fetc%252fpasswd", // double-encoded
        "/%252e%252e%252fetc%252fpasswd",
        "/%2e%2e%5c%2e%2e%5cetc%5cpasswd", // backslash variant
        "/%c0%af..%c0%af..%c0%afetc%c0%afpasswd", // overlong UTF-8
        "/%00",                            // percent-encoded NUL
        "/index.html%00",
        "/%00%2f..%2fetc%2fpasswd",
        "/....//....//etc/passwd",
        "/%2e%2e%2f", // bare traversal of the root
    ];
    for uri in uris {
        let (status, _h, body) = call(&app, &Method::GET, uri, &[])
            .await
            .expect("request builds");
        let text = String::from_utf8_lossy(&body);
        assert!(
            !status.is_server_error(),
            "{uri}: unexpected status {status}"
        );
        assert!(
            !text.contains("root:") && !text.contains("B6-SIBLING-SECRET"),
            "{uri}: leaked a file outside STATIC_DIR: {status}"
        );
    }
}

// ---------------------------------------------------------------------
// 4. Raw HTTP layer (hyper): oversized and malformed request lines.
// ---------------------------------------------------------------------

#[tokio::test]
async fn oversized_request_line_is_handled() {
    let addr = spawn_server(state(StubSource::happy(1))).await;
    // ~1 MiB request target: well over hyper's head-size cap.
    let long_target = format!("/api/v1/frames?ra={}&dec=0", "1".repeat(1 << 20));
    let req = format!("GET {long_target} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n");
    let ans = raw_exchange(addr, req.as_bytes(), Duration::from_secs(5)).await;
    if let Some(code) = status_line_code(&ans.bytes) {
        assert!(
            (400..500).contains(&code),
            "expected a 4xx (e.g. 400/414/431), got {code}"
        );
    }
    // Server must still serve normal requests (no hang/crash).
    let ok = raw_exchange(
        addr,
        b"GET /api/v1/health HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(status_line_code(&ok.bytes), Some(200), "{ok:?}");
    assert!(!ok.timed_out);
}

#[tokio::test]
async fn oversized_headers_are_handled() {
    let addr = spawn_server(state(StubSource::happy(1))).await;
    // One ~1 MiB header value plus a thousand small headers.
    let mut req = String::from("GET /api/v1/health HTTP/1.1\r\nHost: x\r\n");
    req.push_str(&format!("X-Pad: {}\r\n", "A".repeat(1 << 20)));
    for i in 0..1000 {
        req.push_str(&format!("X-H{i}: {i}\r\n"));
    }
    req.push_str("Connection: close\r\n\r\n");
    let ans = raw_exchange(addr, req.as_bytes(), Duration::from_secs(5)).await;
    if let Some(code) = status_line_code(&ans.bytes) {
        assert!((400..500).contains(&code), "expected 4xx, got {code}");
    }
    let ok = raw_exchange(
        addr,
        b"GET /api/v1/health HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(status_line_code(&ok.bytes), Some(200));
}

#[tokio::test]
async fn malformed_request_bytes_are_handled() {
    let addr = spawn_server(state(StubSource::happy(1))).await;
    for garbage in [
        b"GARBAGE\r\n\r\n".as_slice(),
        b"\x00\x01\x02\x03\xff\xfe\r\n\r\n".as_slice(),
        b"GET\r\n\r\n".as_slice(),
        b"GET /api/v1/health HTTP/9.9\r\n\r\n".as_slice(),
        b"GET /api/v1/health HTTP/1.1\r\nBrokenHeader\r\n\r\n".as_slice(),
    ] {
        let ans = raw_exchange(addr, garbage, Duration::from_secs(5)).await;
        if let Some(code) = status_line_code(&ans.bytes) {
            assert!((400..500).contains(&code), "expected 4xx, got {code}");
        }
        // No panic, no hang: server still answers afterwards.
    }
    let ok = raw_exchange(
        addr,
        b"GET /api/v1/health HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(status_line_code(&ok.bytes), Some(200));
}

// ---------------------------------------------------------------------
// 5. CORS
// ---------------------------------------------------------------------

#[tokio::test]
async fn cors_allowed_origin_is_echoed() {
    let mut st = state(StubSource::happy(0));
    st.allowed_origins = Arc::new(vec!["https://embed.example".to_string()]);
    let app = app(st);

    let (status, headers, _b) = call(
        &app,
        &Method::GET,
        "/api/v1/health",
        &[hdr("origin", "https://embed.example")],
    )
    .await
    .expect("request builds");
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("https://embed.example")
    );
}

#[tokio::test]
async fn cors_disallowed_origin_gets_no_acao() {
    let mut st = state(StubSource::happy(0));
    st.allowed_origins = Arc::new(vec!["https://embed.example".to_string()]);
    let app = app(st);

    let (status, headers, _b) = call(
        &app,
        &Method::GET,
        "/api/v1/health",
        &[hdr("origin", "https://evil.example")],
    )
    .await
    .expect("request builds");
    assert_eq!(status, StatusCode::OK);
    assert!(
        headers.get("access-control-allow-origin").is_none(),
        "disallowed origin must not be echoed"
    );
}

#[tokio::test]
async fn cors_preflight_allows_get_only() {
    let mut st = state(StubSource::happy(0));
    st.allowed_origins = Arc::new(vec!["https://embed.example".to_string()]);
    let app = app(st);

    let (status, headers, _b) = call(
        &app,
        &Method::OPTIONS,
        "/api/v1/frames",
        &[
            hdr("origin", "https://embed.example"),
            hdr("access-control-request-method", "GET"),
        ],
    )
    .await
    .expect("request builds");
    assert!(status.is_success(), "preflight status {status}");
    assert_eq!(
        headers
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("https://embed.example")
    );
    let methods = headers
        .get("access-control-allow-methods")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_uppercase();
    assert!(methods.contains("GET"), "ACAM {methods:?}");
    for m in ["POST", "PUT", "DELETE", "PATCH"] {
        assert!(!methods.contains(m), "ACAM must not list {m}: {methods:?}");
    }
}

#[tokio::test]
async fn cors_preflight_from_disallowed_origin_not_echoed() {
    let mut st = state(StubSource::happy(0));
    st.allowed_origins = Arc::new(vec!["https://embed.example".to_string()]);
    let app = app(st);

    let (_status, headers, _b) = call(
        &app,
        &Method::OPTIONS,
        "/api/v1/frames",
        &[
            hdr("origin", "https://evil.example"),
            hdr("access-control-request-method", "GET"),
        ],
    )
    .await
    .expect("request builds");
    assert_ne!(
        headers
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("https://evil.example")
    );
}

#[tokio::test]
async fn cors_never_wildcard_and_no_credentials() {
    // Even with a permissive-looking entry, no `*` ACAO and no
    // `access-control-allow-credentials` may appear.
    let mut st = state(StubSource::happy(0));
    st.allowed_origins = Arc::new(vec!["https://embed.example".to_string()]);
    let app = app(st);
    for uri in [
        "/api/v1/health",
        "/api/v1/frames?ra=999&dec=0", // 400 path also CORS-consistent
    ] {
        let (_s, headers, _b) = call(
            &app,
            &Method::GET,
            uri,
            &[hdr("origin", "https://embed.example")],
        )
        .await
        .expect("request builds");
        let acao = headers
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_ne!(acao, "*", "{uri}: wildcard ACAO is forbidden");
        assert!(
            headers.get("access-control-allow-credentials").is_none(),
            "{uri}: credentials must not be allowed"
        );
    }
}

// ---------------------------------------------------------------------
// 6. Rate limiting (tower-governor, SmartIpKeyExtractor)
// ---------------------------------------------------------------------

#[tokio::test]
async fn rate_limit_burst_returns_429_contract_body() {
    let mut st = state(StubSource::happy(0));
    st.rate_limit_rps = 1;
    let app = app(st);
    let mut got_429 = false;
    for i in 0..5 {
        let (status, _h, body) = call(&app, &Method::GET, "/api/v1/health", &[])
            .await
            .expect("request builds");
        if i == 0 {
            assert_eq!(status, StatusCode::OK);
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            got_429 = true;
            let code = check_error_body(&body).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(code, "rate_limited");
        } else {
            assert!(status.is_success(), "unexpected status {status}");
        }
    }
    assert!(got_429, "burst never produced a 429");
}

#[tokio::test]
async fn rate_limit_other_client_key_not_penalised() {
    let mut st = state(StubSource::happy(0));
    st.rate_limit_rps = 1;
    let app = app(st);
    // Client A exhausts its budget.
    let (_s, _h, _b) = call(
        &app,
        &Method::GET,
        "/api/v1/health",
        &[hdr("x-forwarded-for", "203.0.113.7")],
    )
    .await
    .expect("request builds");
    let (status_a, _h, _b) = call(
        &app,
        &Method::GET,
        "/api/v1/health",
        &[hdr("x-forwarded-for", "203.0.113.7")],
    )
    .await
    .expect("request builds");
    assert_eq!(
        status_a,
        StatusCode::TOO_MANY_REQUESTS,
        "client A second hit"
    );
    // Client B (different key) must not inherit A's penalty.
    let (status_b, _h, _b) = call(
        &app,
        &Method::GET,
        "/api/v1/health",
        &[hdr("x-forwarded-for", "198.51.100.23")],
    )
    .await
    .expect("request builds");
    assert_eq!(status_b, StatusCode::OK, "client B was penalised");
}

#[tokio::test]
async fn rate_limit_garbage_xff_falls_back_to_peer_ip() {
    // A malformed X-Forwarded-For must not produce a 5xx: the extractor
    // falls back to the ConnectInfo peer address.
    let mut st = state(StubSource::happy(0));
    st.rate_limit_rps = 1000;
    let app = app(st);
    for xff in ["garbage", "", "999.999.999.999", "a,b,c", "10.0.0.1, bad"] {
        let (status, _h, _b) = call(
            &app,
            &Method::GET,
            "/api/v1/health",
            &[hdr("x-forwarded-for", xff)],
        )
        .await
        .expect("request builds");
        assert!(!status.is_server_error(), "XFF {xff:?} produced {status}");
    }
}

// ---------------------------------------------------------------------
// 7. Deadline behaviour and in-flight cancellation
// ---------------------------------------------------------------------

#[tokio::test]
async fn deadline_partial_results_are_truncated() {
    // One fast observation, three sleeping well past the deadline.
    let mut s = StubSource::happy(4);
    for o in &s.observations[1..] {
        s.delays.insert(obs_id_hash(o), Duration::from_secs(60));
    }
    let in_flight = Arc::clone(&s.in_flight);
    let mut st = state(s);
    st.request_deadline = Duration::from_millis(100);
    let app = app(st);

    let t0 = Instant::now();
    let (status, _h, body) = call(
        &app,
        &Method::GET,
        "/api/v1/frames?ra=337.878&dec=-5.189&fov=0.05",
        &[],
    )
    .await
    .expect("request builds");
    let elapsed = t0.elapsed();
    assert_eq!(status, StatusCode::OK, "body: {body:?}");
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(v["truncated"], json!(true), "{v}");
    assert_eq!(v["total_observations"], json!(4));
    let n = v["frames"].as_array().map(Vec::len).unwrap_or(0);
    assert!(
        (1..4).contains(&n),
        "expected a partial result set, got {n}"
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "deadline did not bound the request: {elapsed:?}"
    );
    // Aborted stub calls must drain (no leaked in-flight work).
    for _ in 0..200 {
        if in_flight.load(Ordering::SeqCst) == 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("in-flight stub calls never drained");
}

#[tokio::test]
async fn deadline_nothing_decoded_is_502_and_cancels() {
    let mut s = StubSource::happy(3);
    for o in &s.observations {
        s.delays.insert(obs_id_hash(o), Duration::from_secs(60));
    }
    let in_flight = Arc::clone(&s.in_flight);
    let fetch_calls = Arc::clone(&s.fetch_calls);
    let mut st = state(s);
    st.request_deadline = Duration::from_millis(50);
    let app = app(st);

    let t0 = Instant::now();
    let (status, _h, body) = call(
        &app,
        &Method::GET,
        "/api/v1/frames?ra=337.878&dec=-5.189",
        &[],
    )
    .await
    .expect("request builds");
    let elapsed = t0.elapsed();
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    let code = check_error_body(&body).expect("error body");
    assert_eq!(code, "upstream_unavailable");
    assert!(elapsed < Duration::from_secs(3), "hang: {elapsed:?}");
    assert_eq!(fetch_calls.load(Ordering::SeqCst), 3, "fetches spawned");

    for _ in 0..200 {
        if in_flight.load(Ordering::SeqCst) == 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("aborted stub calls still in flight");
}

// ---------------------------------------------------------------------
// 8. Resource bounds
// ---------------------------------------------------------------------

#[tokio::test]
async fn max_frames_cap_and_truncation_flag() {
    let app = app(state(StubSource::happy(200)));
    let (status, _h, body) = call(
        &app,
        &Method::GET,
        "/api/v1/frames?ra=337.878&dec=-5.189&max_frames=60",
        &[],
    )
    .await
    .expect("request builds");
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_slice(&body).expect("json");
    let n = v["frames"].as_array().map(Vec::len).unwrap_or(usize::MAX);
    assert!(n <= 60, "cap violated: {n} frames");
    assert_eq!(v["total_observations"], json!(200));
    assert_eq!(v["truncated"], json!(true));
}

/// Largest *default* request: `max_frames=60` at `OUTPUT_NPIX=192`
/// (the production default). Assert the decoded payload stays under the
/// documented 20 MB bound and report the measured number.
#[tokio::test]
async fn largest_default_payload_under_20mb() {
    let mut st = state(StubSource::happy(70));
    st.output_npix = 192; // production default (config.rs)
    let app = app(st);
    let (status, _h, body) = call(
        &app,
        &Method::GET,
        "/api/v1/frames?ra=337.878&dec=-5.189&fov=0.5&mode=raw&max_frames=60",
        &[],
    )
    .await
    .expect("request builds");
    assert_eq!(status, StatusCode::OK);
    let mib = body.len() as f64 / (1024.0 * 1024.0);
    eprintln!(
        "largest default payload: {:.2} MiB ({} bytes)",
        mib,
        body.len()
    );
    assert!(
        body.len() < 20 * 1024 * 1024,
        "payload exceeds 20 MiB: {} bytes",
        body.len()
    );
}

/// Same request at the hard ceiling `OUTPUT_NPIX=256` (still a legal
/// config value) breaches 20 MiB: 60 rasters x 256x256 f32 ≈ 21 MiB
/// base64. Documented bound is defined at the 192 default.
#[tokio::test]
#[ignore = "BUG: OUTPUT_NPIX=256 x 60 frames produces ~21 MiB, above the 20 MiB documented bound"]
async fn largest_payload_at_max_npix_also_under_20mb() {
    let mut st = state(StubSource::happy(70));
    st.output_npix = 256; // legal ceiling per config.rs
    let app = app(st);
    let (status, _h, body) = call(
        &app,
        &Method::GET,
        "/api/v1/frames?ra=337.878&dec=-5.189&fov=0.5&mode=raw&max_frames=60",
        &[],
    )
    .await
    .expect("request builds");
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.len() < 20 * 1024 * 1024,
        "payload exceeds 20 MiB: {} bytes",
        body.len()
    );
}

#[tokio::test]
async fn rss_growth_bounded_over_200_requests() {
    let mut st = state(StubSource::happy(8));
    st.output_npix = 64;
    let app = app(st);
    let uri = "/api/v1/frames?ra=337.878&dec=-5.189&fov=0.05";
    // Warm up allocators/caches/layers before sampling.
    for _ in 0..30 {
        let (s, _h, _b) = call(&app, &Method::GET, uri, &[]).await.expect("req");
        assert_eq!(s, StatusCode::OK);
    }
    let before = rss_bytes();
    for _ in 0..200 {
        let (s, _h, _b) = call(&app, &Method::GET, uri, &[]).await.expect("req");
        assert_eq!(s, StatusCode::OK);
    }
    let after = rss_bytes();
    let delta = after.saturating_sub(before);
    eprintln!(
        "RSS over 200 requests: {} -> {} bytes (delta {:.2} MiB)",
        before,
        after,
        delta as f64 / (1024.0 * 1024.0)
    );
    // Generous bound to stay flake-free across allocators while still
    // catching unbounded retention.
    assert!(delta < 128 * 1024 * 1024, "RSS grew {delta} bytes");
}

// ---------------------------------------------------------------------
// 9. Secrets / internal-info leakage
// ---------------------------------------------------------------------

#[tokio::test]
async fn error_bodies_never_echo_input_or_internals() {
    // A marker env var and a marker upstream path: neither may appear in
    // any error body. `fetch_fail` simulates the exact `detail` string a
    // reqwest transport error produces (it embeds the full URL).
    let secret = "B6-BODY-LEAK-MARKER-7d2e";
    std::env::set_var("SPHEREX_B6_TEST_SECRET", secret);

    let mut s = StubSource::happy(2);
    s.fetch_fail = Some(spherex_backend::irsa::IrsaError::Unavailable {
        attempts: 3,
        detail: format!("connect error: error sending request for url (https://stub.invalid/ibe/data/{secret}/x.fits)"),
    });
    let app = app(state(s));

    for uri in [
        "/api/v1/frames?ra=bad&dec=0",
        "/api/v1/frames?ra=999&dec=0",
        "/api/v1/frames?ra=337.878&dec=-5.189", // all fetches fail -> 502
        "/api/v1/spectrum?ra=%3Cscript%3E&dec=0",
        "/api/v1/nope",
    ] {
        let (status, _h, body) = call(&app, &Method::GET, uri, &[])
            .await
            .expect("request builds");
        assert!(
            status.is_client_error() || status.is_server_error(),
            "{uri}: {status}"
        );
        // Every non-2xx API answer must be a contract ErrorBody.
        let code = check_error_body(&body).unwrap_or_else(|e| panic!("{uri}: {e}"));
        assert_ne!(code, "internal", "{uri}: internal leaked to client");
        let text = String::from_utf8_lossy(&body);
        for needle in [
            secret,
            "stub.invalid",
            ".rs",
            "panicked",
            "RUST_BACKTRACE",
            "error sending request for url",
            "/ibe/data/",
        ] {
            assert!(
                !text.contains(needle),
                "{uri}: body leaks {needle:?}: {text}"
            );
        }
    }
}

/// `fetch_cutouts` logs `warn!("cutout skipped: {e}")`. `IrsaError`'s
/// Display carries `detail`, which for transport failures embeds the
/// full upstream URL (reqwest renders it) — including its path and the
/// `center`/`size` query. This check verifies nothing beyond the host
/// reaches the logs; it is expected to fail while the leak exists.
#[tokio::test]
#[ignore = "BUG: warn! in api/fetch.rs logs IrsaError detail containing the full upstream URL (path + query)"]
async fn logs_do_not_leak_upstream_url_paths() {
    let sink = LogSink::install();
    sink.clear();

    // Real IrsaClient against a local SIA stub whose access_url points
    // at a closed port with a marker path: connect errors embed the URL
    // in the reqwest error string exactly as in production.
    use axum::extract::State;
    use axum::routing::get;
    let urls = vec!["{base}/ibe/data/B6_SECRET_PATH_MARKER/x.fits".to_string()];
    let mut rows = String::new();
    for u in &urls {
        rows.push_str(&format!(
            "<TR><TD>61000</TD><TD>61000.001</TD><TD>1.1e-6</TD><TD>1.2e-6</TD>\
             <TD>SPHEREx-D1</TD><TD>{u}</TD></TR>"
        ));
    }
    let votable = format!(
        "<VOTABLE><RESOURCE><TABLE>\
         <FIELD name=\"t_min\" datatype=\"double\" unit=\"d\"/>\
         <FIELD name=\"t_max\" datatype=\"double\" unit=\"d\"/>\
         <FIELD name=\"em_min\" datatype=\"double\" unit=\"m\"/>\
         <FIELD name=\"em_max\" datatype=\"double\" unit=\"m\"/>\
         <FIELD name=\"energy_bandpassname\" datatype=\"char\" arraysize=\"*\"/>\
         <FIELD name=\"access_url\" datatype=\"char\" arraysize=\"*\"/>\
         <DATA><TABLEDATA>{rows}</TABLEDATA></DATA></TABLE></RESOURCE></VOTABLE>"
    );

    // Reserve a port, then drop the listener so connects are refused.
    let closed_port = {
        let l = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind");
        l.local_addr().expect("addr").port()
    };
    let sia_listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind sia stub");
    let sia_addr = sia_listener.local_addr().expect("addr");
    let body = votable.replace("{base}", &format!("http://127.0.0.1:{closed_port}"));
    tokio::spawn(async move {
        let app =
            axum::Router::new().route("/SIA", get(move |State(b): State<String>| async move { b }));
        let _ = axum::serve(sia_listener, app.with_state(body)).await;
    });

    let mut cfg = IrsaConfig {
        base_url: format!("http://{sia_addr}"),
        collections: vec!["spherex_qr3".to_string()],
        max_attempts: 1,
        retry_base_delay: Duration::from_millis(1),
        request_timeout: Duration::from_secs(2),
        ..IrsaConfig::default()
    };
    cfg.max_body_bytes = 1 << 20;
    let client = IrsaClient::new(cfg).expect("client");
    let st: AppState<IrsaClient> = AppState {
        source: Arc::new(client),
        collections: Arc::new(vec!["spherex_qr3".to_string()]),
        static_dir: PathBuf::from("frontend/dist"),
        allowed_origins: Arc::new(Vec::new()),
        output_npix: 16,
        max_frames_cap: 60,
        request_deadline: Duration::from_secs(5),
        rate_limit_rps: 1_000_000,
    };
    let app = app(st);

    let (status, _h, body) = call(
        &app,
        &Method::GET,
        "/api/v1/frames?ra=337.878&dec=-5.189",
        &[],
    )
    .await
    .expect("request builds");
    assert_eq!(status, StatusCode::BAD_GATEWAY, "body {body:?}");

    // Give tracing a moment to flush the warn! into the sink.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let logs = sink.text();
    let body_text = String::from_utf8_lossy(&body);
    assert!(
        !body_text.contains("B6_SECRET_PATH_MARKER"),
        "error body leaked the upstream path: {body_text}"
    );
    assert!(
        !logs.contains("B6_SECRET_PATH_MARKER"),
        "logs leaked the upstream URL path:\n{logs}"
    );
}

#[tokio::test]
async fn logs_do_not_leak_secrets_or_internals() {
    let sink = LogSink::install();
    sink.clear();
    let secret = "B6-SECRET-ENV-MARKER-a3f9c1";
    std::env::set_var("SPHEREX_B6_TEST_SECRET", secret);

    // Exercise warn! paths: skipped cutouts + bad queries.
    let mut s = StubSource::happy(3);
    s.fetch_fail = Some(spherex_backend::irsa::IrsaError::Http { status: 422 });
    let app = app(state(s));
    std::env::set_var("SPHEREX_B6_TEST_SECRET", secret);
    for uri in [
        "/api/v1/frames?ra=337.878&dec=-5.189",
        "/api/v1/frames?ra=bad&dec=0",
        "/api/v1/spectrum?ra=999&dec=0",
    ] {
        let _ = call(&app, &Method::GET, uri, &[]).await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    let logs = sink.text();
    for needle in [secret, "panicked at", "RUST_BACKTRACE"] {
        assert!(!logs.contains(needle), "logs leak {needle:?}:\n{logs}");
    }
}
