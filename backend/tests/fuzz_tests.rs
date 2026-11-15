//! Deterministic input fuzzing of the API query strings plus
//! property-style tests of the coordinate grammar (task b6).
//!
//! The generator is a seeded xorshift64* (`support::Rng`) — no new
//! dependencies, fully reproducible runs. Every case goes through
//! `tower::oneshot` against the stub `FrameSource`; invariants:
//!
//! - never a 5xx, never a panic;
//! - every 4xx body is valid JSON matching `ErrorBody` with a contract
//!   `code` and a non-empty `message`;
//! - raw input is never echoed in the body;
//! - each case completes in < 200 ms.

mod fixtures;
mod support;

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use axum::http::{Method, StatusCode};
use axum::Router;
use serde_json::Value;
use spherex_backend::api::validation::{parse_coordinates, validate_query_params};
use support::*;

const SEED_FRAMES: u64 = 0xb6f4_11ce_0000_0001;
const SEED_SPECTRUM: u64 = 0xb6f4_11ce_0000_0002;
const MAX_CASE_MS: f64 = 200.0;
const MAX_ERROR_BODY: usize = 4096;

// ---------------------------------------------------------------------
// Value pools
// ---------------------------------------------------------------------

const NUMERIC: &[&str] = &[
    "0",
    "1",
    "-1",
    "+1",
    "359.999999",
    "359.9999999999",
    "360",
    "360.0000001",
    "-0",
    "-0.0",
    "0.5",
    "90",
    "-90",
    "90.0000001",
    "-90.0000001",
    "0.01",
    "0.0099",
    "0.5000001",
    "1e-2",
    "1e1",
    "1E2",
    "1e400",
    "-1e400",
    "1e-400",
    "NaN",
    "nan",
    "NAN",
    "inf",
    "-inf",
    "INF",
    "Infinity",
    "-Infinity",
    "infinity",
    ".5",
    "5.",
    "+.5",
    "-.5",
    "0x10",
    "0b101",
    "0o17",
    "1_000",
    "1,5",
    "1;5",
    "1.2.3",
    "e5",
    ".e5",
    "1e",
    "1e+",
    "1e-",
    "-",
    "+",
    ".",
    "..",
    " ",
    "  ",
    " 12",
    "12 ",
    "1 2",
    "00012",
    "012.",
    "4294967296",
    "18446744073709551615",
    "99999999999",
    "-2147483648",
    "3.141592653589793",
    "2.2250738585072014e-308",
    "4.9e-324",
    "1.7976931348623157e308",
    "133.795129",
    "-7.245147",
    "337.8783221",
    "-5.1891588",
];

const SEXA: &[&str] = &[
    "08h55m10.831s",
    "23h59m59.999s",
    "0h0m0s",
    "24h0m0s",
    "10h30m",
    "10h30m15.5s",
    "10h60m",
    "10h59m60s",
    "-1h30m",
    "+5h15m",
    "h",
    "10h",
    "10h30",
    "10h30x",
    "10h30m5x",
    "10.5h",
    "0d0m0s",
    "-07d14m42.53s",
    "+30d24m05s",
    "90d0m0s",
    "90d0m0.1s",
    "-90d0m0s",
    "-91d0m0s",
    "45d30m",
    "45d60m",
    "12d30m60s",
    "d30m",
    "10d-30m",
    "10d30m15.999s",
    "08H55M10.831S",
    "10H30M",
    "-0d0m0s",
    "360d0m0s",
    "359d59m59.99s",
    "10d30m15s5",
    "5s",
    "10h-30m",
    "25h0m0s",
];

const ATTACK: &[&str] = &[
    "1; DROP TABLE users--",
    "' OR '1'='1",
    "1' OR '1'='1' --",
    "0 UNION SELECT * FROM data",
    "$(rm -rf /)",
    "`id`",
    "| cat /etc/passwd",
    "; cat /etc/passwd",
    "../../etc/passwd",
    "../../../../etc/shadow",
    "..\\..\\win.ini",
    "<script>alert(1)</script>",
    "<img src=x onerror=alert(1)>",
    "javascript:alert(1)",
    "{{7*7}}",
    "${7*7}",
    "<%= 7*7 %>",
    "|ls -la",
    "&& whoami",
    "`sleep 10`",
    "; sleep 10",
    "1\n2",
    "1\t2",
    "\u{0}",
    "../../../../proc/self/environ",
    "file:///etc/passwd",
    "http://evil.example/x",
    "RA] OR 1=1--",
    "'--",
    "\" OR \"\"=\"",
    "eval()",
    "printenv",
];

const UNICODE_SNIPPETS: &[&str] = &[
    "αξϛ",
    "١٢٣٤٥",
    "１２３",
    "🚀🛰️🪐",
    "Iñtërnâtiônàlizætiøn",
    "\u{202e}txet\u{202c}",
    "\u{200b}12",
    "12\u{00a0}",
    "",
    "한국어",
    "ﬀﬁﬂ",
    "𝕏𝕐𝕑",
    "München",
    "Ångström",
];

const FOV: &[&str] = &[
    "0.01",
    "0.05",
    "0.5",
    "0.0099",
    "0.009999",
    "0.5000001",
    "0",
    "1",
    "-0.01",
    "1e-2",
    "5e-1",
    "NaN",
    "inf",
    "0.05 ",
    " 0.1",
    "abc",
    "",
];

const MODE: &[&str] = &[
    "raw", "diff", "coadd", "stack", "RAW", "Raw", " raw", "raw ", "0", "true", "rawx", "di ff",
    "co-add", "", "r\u{0}aw",
];

const DETECTOR: &[&str] = &[
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "0",
    "7",
    "-1",
    "abc",
    "1.0",
    "+3",
    " 2",
    "6 ",
    "99999999999999999999",
    "0x3",
    "03",
    "",
];

const MAX_FRAMES: &[&str] = &[
    "1",
    "24",
    "60",
    "0",
    "61",
    "-1",
    "99999999999",
    "abc",
    "1.5",
    "+60",
    " 30",
    "060",
    "18446744073709551615",
    "18446744073709551616",
    "",
];

const BOOL: &[&str] = &[
    "true", "false", "1", "0", "TRUE", "False", "yes", "on", "t", "", "  true", "2",
];

const APERTURE: &[&str] = &[
    "1",
    "3",
    "5",
    "2",
    "0",
    "7",
    "-1",
    "abc",
    "1.0",
    " 3",
    "3 ",
    "03",
    "+3",
    "99999999999999999999",
    "",
];

/// Raw percent-sequences injected unencoded (decoder-level edge cases).
const RAW_PCT: &[&str] = &[
    "%00",
    "%0A",
    "%0D%0A",
    "%FF",
    "%C0%AF",
    "%E0%80%80",
    "%",
    "%zz",
    "%2",
    "%u0041",
    "%25",
    "%G0",
];

// ---------------------------------------------------------------------
// Fuzz statistics
// ---------------------------------------------------------------------

#[derive(Default)]
struct Stats {
    cases: u64,
    unbuildable: u64,
    slowest_ms: f64,
    slowest_desc: String,
    max_body: usize,
    statuses: BTreeMap<u16, u64>,
    violations: Vec<String>,
}

impl Stats {
    fn violation(&mut self, msg: String) {
        if self.violations.len() < 25 {
            self.violations.push(msg);
        }
    }

    fn report(&self, name: &str) {
        eprintln!(
            "{name}: {} cases ({} unbuildable), slowest {:.1} ms [{}], max body {} B, statuses {:?}",
            self.cases, self.unbuildable, self.slowest_ms, self.slowest_desc, self.max_body,
            self.statuses
        );
    }
}

// ---------------------------------------------------------------------
// Case construction
// ---------------------------------------------------------------------

struct Case {
    uri: String,
    /// Raw (decoded) parameter values — for the no-echo invariant.
    raws: Vec<String>,
    desc: String,
}

fn unicode_garbage(rng: &mut Rng) -> String {
    let mut s = rng.pick(UNICODE_SNIPPETS).to_string();
    if rng.chance(0.6) {
        s.push_str(rng.pick(UNICODE_SNIPPETS));
    }
    // A few random scalar values (skip surrogates via char::from_u32).
    for _ in 0..rng.below(6) {
        if let Some(c) = char::from_u32(rng.below(0x2FFF) as u32) {
            s.push(c);
        }
    }
    s
}

fn ascii_garbage(rng: &mut Rng) -> String {
    const CH: &[u8] = b"!@#$%^&*()={}[]|\\:;\"'<>?,/~`abcXYZ019 .-_";
    (0..=rng.below(20))
        .map(|_| CH[rng.below(CH.len())] as char)
        .collect()
}

fn numeric_mutant(rng: &mut Rng) -> String {
    let base = rng.pick(NUMERIC);
    match rng.below(6) {
        0 => format!("{base}junk"),
        1 => format!("{base}{}", rng.below(10)),
        2 => format!("{}{}", &base[..base.len().min(2)], base),
        3 => format!("  {base}  "),
        4 => {
            let mut s = base.to_string();
            s.insert(rng.below(s.len().max(1)), '.');
            s
        }
        _ => format!("{base}e{}", rng.below(400)),
    }
}

fn value(rng: &mut Rng) -> String {
    match rng.below(100) {
        0..=34 => rng.pick(NUMERIC).to_string(),
        35..=49 => rng.pick(SEXA).to_string(),
        50..=64 => rng.pick(ATTACK).to_string(),
        65..=79 => unicode_garbage(rng),
        80..=89 => numeric_mutant(rng),
        90..=97 => ascii_garbage(rng),
        _ => {
            // ~2%: very long values. `http::Uri` caps request targets at
            // u16::MAX bytes — stay under it so the case actually reaches
            // the router (the >64 KiB layer is covered over raw TCP).
            let len = *rng.pick(&[1024usize, 8192, 16_384, 32_768, 60_000]);
            let c = *rng.pick(b"Aa19;/%<. ".as_slice()) as char;
            std::iter::repeat_n(c, len).collect()
        }
    }
}

fn frames_case(rng: &mut Rng) -> Case {
    let mut parts: Vec<String> = Vec::new();
    let mut raws: Vec<String> = Vec::new();
    let mut tags: Vec<&str> = Vec::new();
    let mut add = |name: &str, v: &str| {
        parts.push(format!("{name}={}", pct_encode(v)));
        raws.push(v.to_string());
    };

    // Core coordinates (~8% of cases drop or corrupt one).
    if rng.chance(0.96) {
        add("ra", &value(rng));
    } else {
        tags.push("no-ra");
    }
    if rng.chance(0.96) {
        add("dec", &value(rng));
    } else {
        tags.push("no-dec");
    }
    if rng.chance(0.5) {
        add("fov", rng.pick(FOV));
    }
    if rng.chance(0.3) {
        add("mode", rng.pick(MODE));
    }
    if rng.chance(0.3) {
        add("detector", rng.pick(DETECTOR));
    }
    if rng.chance(0.3) {
        add("max_frames", rng.pick(MAX_FRAMES));
    }
    if rng.chance(0.2) {
        add("subtract_zodi", rng.pick(BOOL));
    }

    // Exotic mutations on top (~15%).
    match rng.below(100) {
        0..=4 => {
            // Repeated parameter: last-wins per the decoder.
            add("ra", &value(rng));
            tags.push("dup-ra");
        }
        5..=8 => {
            // Raw percent junk appended to the query.
            parts.push(rng.pick(RAW_PCT).to_string());
            tags.push("raw-pct");
        }
        9..=12 => {
            // Weird key names (missing/case/extra params).
            let k = *rng.pick(&["RA", "ra[0]", "r", "decdec", "ra.x", ""]);
            parts.push(format!("{k}={}", pct_encode(&value(rng))));
            tags.push("weird-key");
        }
        _ => {}
    }

    let desc = format!("frames[{}] {}", tags.join(","), parts.join("&"));
    Case {
        uri: format!("/api/v1/frames?{}", parts.join("&")),
        raws,
        desc,
    }
}

fn spectrum_case(rng: &mut Rng) -> Case {
    let mut parts: Vec<String> = Vec::new();
    let mut raws: Vec<String> = Vec::new();
    let mut add = |name: &str, v: &str| {
        parts.push(format!("{name}={}", pct_encode(v)));
        raws.push(v.to_string());
    };
    if rng.chance(0.96) {
        add("ra", &value(rng));
    }
    if rng.chance(0.96) {
        add("dec", &value(rng));
    }
    if rng.chance(0.7) {
        add("aperture_px", rng.pick(APERTURE));
    }
    if rng.chance(0.05) {
        add("aperture_px", &value(rng));
    }
    let desc = format!("spectrum {}", parts.join("&"));
    Case {
        uri: format!("/api/v1/spectrum?{}", parts.join("&")),
        raws,
        desc,
    }
}

// ---------------------------------------------------------------------
// Case runner enforcing the invariants
// ---------------------------------------------------------------------

async fn run_case(app: &Router, stats: &mut Stats, case: &Case, check_ok_keys: &[&str]) {
    let t0 = Instant::now();
    let Some(res) = call_guarded(app, &Method::GET, &case.uri, &[]).await else {
        stats.unbuildable += 1;
        return;
    };
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    stats.cases += 1;
    if ms > stats.slowest_ms {
        stats.slowest_ms = ms;
        stats.slowest_desc = case.desc.chars().take(160).collect();
    }
    if ms > MAX_CASE_MS {
        stats.violation(format!("{ms:.1} ms > {MAX_CASE_MS}: {}", short(&case.desc)));
    }

    let (status, _headers, body) = match res {
        Ok(t) => t,
        Err(e) => {
            stats.violation(format!("{e} :: {}", short(&case.desc)));
            return;
        }
    };
    *stats.statuses.entry(status.as_u16()).or_insert(0) += 1;
    stats.max_body = stats.max_body.max(body.len());

    if status.is_server_error() {
        stats.violation(format!("{status} :: {}", short(&case.desc)));
        return;
    }
    if status.is_client_error() {
        if body.len() > MAX_ERROR_BODY {
            stats.violation(format!(
                "4xx body {} B > {MAX_ERROR_BODY} :: {}",
                body.len(),
                short(&case.desc)
            ));
        }
        match check_error_body(&body) {
            Err(e) => stats.violation(format!(
                "{status} bad ErrorBody: {e} :: {}",
                short(&case.desc)
            )),
            Ok(code) => {
                // With a non-empty stub the reachable contract codes are
                // invalid_parameter (400) and not_found (404: uncovered
                // point / empty detector group). `internal` must never
                // surface; a 5xx was already rejected above.
                if code == "internal" {
                    stats.violation(format!(
                        "{status} code \"internal\" leaked :: {}",
                        short(&case.desc)
                    ));
                }
            }
        }
        // Raw-input echo check on long, distinctive payloads only.
        let text = String::from_utf8_lossy(&body);
        for raw in &case.raws {
            if raw.len() >= 8 && text.contains(raw.as_str()) {
                stats.violation(format!(
                    "{status} echoes raw input {:?} :: {}",
                    short(raw),
                    short(&case.desc)
                ));
            }
        }
        return;
    }
    // 2xx/3xx: body must be JSON; spot-check the endpoint's top-level keys.
    let Ok(v) = serde_json::from_slice::<Value>(&body) else {
        stats.violation(format!("{status} non-JSON body :: {}", short(&case.desc)));
        return;
    };
    for k in check_ok_keys {
        if v.get(*k).is_none() {
            stats.violation(format!(
                "{status} missing key {k:?} :: {}",
                short(&case.desc)
            ));
        }
    }
}

fn short(s: &str) -> String {
    s.chars().take(140).collect()
}

// ---------------------------------------------------------------------
// The batteries (>= 5000 cases in total)
// ---------------------------------------------------------------------

#[tokio::test]
async fn fuzz_frames_query_strings() {
    let app = app(state(StubSource::happy(4)));
    let mut rng = Rng::new(SEED_FRAMES);
    let mut stats = Stats::default();
    let ok_keys = ["query", "width", "height", "unit", "frames", "flag_legend"];

    for _ in 0..3600 {
        let case = frames_case(&mut rng);
        run_case(&app, &mut stats, &case, &ok_keys).await;
    }
    stats.report("fuzz frames");
    assert!(stats.cases >= 3000, "too few cases ran: {}", stats.cases);
    assert!(
        stats.violations.is_empty(),
        "invariant violations:\n{}",
        stats.violations.join("\n")
    );
}

#[tokio::test]
async fn fuzz_spectrum_query_strings() {
    let app = app(state(StubSource::happy(4)));
    let mut rng = Rng::new(SEED_SPECTRUM);
    let mut stats = Stats::default();
    let ok_keys = ["ra_deg", "dec_deg", "unit", "points"];

    for _ in 0..2400 {
        let case = spectrum_case(&mut rng);
        run_case(&app, &mut stats, &case, &ok_keys).await;
    }
    stats.report("fuzz spectrum");
    assert!(stats.cases >= 2000, "too few cases ran: {}", stats.cases);
    assert!(
        stats.violations.is_empty(),
        "invariant violations:\n{}",
        stats.violations.join("\n")
    );
}

/// Curated boundary/hostile cases pinned to concrete statuses.
#[tokio::test]
async fn curated_boundary_cases() {
    let app = app(state(StubSource::happy(4)));
    // (uri, expected status class)
    let cases: &[(&str, u16)] = &[
        ("/api/v1/frames?ra=360&dec=0", 400),
        ("/api/v1/frames?ra=360.0000001&dec=0", 400),
        ("/api/v1/frames?ra=-0&dec=0", 200), // -0.0 == 0.0, in range
        ("/api/v1/frames?ra=-0.000001&dec=0", 400),
        ("/api/v1/frames?ra=0&dec=90", 200),
        ("/api/v1/frames?ra=0&dec=-90", 200),
        ("/api/v1/frames?ra=0&dec=90.0000001", 400),
        ("/api/v1/frames?ra=0&dec=0&fov=0.0099", 400),
        ("/api/v1/frames?ra=0&dec=0&fov=0.01", 200),
        ("/api/v1/frames?ra=0&dec=0&fov=0.5", 200),
        ("/api/v1/frames?ra=0&dec=0&fov=0.5000001", 400),
        ("/api/v1/frames?ra=0&dec=0&max_frames=0", 400),
        ("/api/v1/frames?ra=0&dec=0&max_frames=61", 400),
        ("/api/v1/frames?ra=0&dec=0&max_frames=-1", 400),
        ("/api/v1/frames?ra=0&dec=0&max_frames=99999999999", 400),
        ("/api/v1/frames?ra=0&dec=0&max_frames=60", 200),
        ("/api/v1/frames?ra=0&dec=0&detector=6", 200),
        ("/api/v1/frames?ra=0&dec=0&detector=7", 400),
        // Duplicate keys: HashMap keeps the last occurrence.
        ("/api/v1/frames?ra=10&ra=400&dec=0", 400),
        ("/api/v1/frames?ra=400&ra=337.878&dec=-5.189", 200),
        // Percent-encoded forms of valid input.
        ("/api/v1/frames?ra=%33%33%37%2e%38%37%38&dec=-5.189", 200),
        ("/api/v1/frames?ra=%2b10&dec=0", 200),   // "+10"
        ("/api/v1/frames?ra=+10&dec=0", 200),     // '+' -> space -> trimmed
        ("/api/v1/frames?ra=1%2e5e1&dec=0", 200), // "1.5e1" -> 15
        // Encoded control bytes / NULs / invalid UTF-8.
        ("/api/v1/frames?ra=10%00&dec=0", 400),
        ("/api/v1/frames?ra=%0010&dec=0", 400),
        ("/api/v1/frames?ra=%ff%fe&dec=0", 400), // invalid UTF-8
        ("/api/v1/frames?ra=%c0%af&dec=0", 400), // overlong '/'
        ("/api/v1/frames?ra=%&dec=0", 400),      // bare '%'
        ("/api/v1/frames?ra=%zz10&dec=0", 400),  // bad escape
        ("/api/v1/frames?ra=NaN&dec=0", 400),
        ("/api/v1/frames?ra=inf&dec=0", 400),
        ("/api/v1/frames?ra=0&dec=-Infinity", 400),
        // Sexagesimal edge cases.
        ("/api/v1/frames?ra=24h0m0s&dec=0", 400), // 360 deg
        ("/api/v1/frames?ra=23h59m60s&dec=0", 400), // seconds >= 60
        ("/api/v1/frames?ra=10h60m&dec=0", 400),  // minutes >= 60
        ("/api/v1/frames?ra=10h30m&dec=0", 200),  // 157.5 deg
        ("/api/v1/frames?ra=0&dec=-0d0m0s", 200), // -0.0 in range
        ("/api/v1/frames?ra=0&dec=90d0m0.1s", 400), // > 90 deg
        ("/api/v1/frames?ra=0&dec=08h55m10s", 400), // 'h' invalid for dec
        ("/api/v1/frames?ra=10h30s&dec=0", 400),  // skipped component
        ("/api/v1/frames?ra=5s&dec=0", 400),      // first unit != h/d
        // Spectrum aperture.
        ("/api/v1/spectrum?ra=337.878&dec=-5.189&aperture_px=1", 200),
        ("/api/v1/spectrum?ra=337.878&dec=-5.189&aperture_px=3", 200),
        ("/api/v1/spectrum?ra=337.878&dec=-5.189&aperture_px=5", 200),
        ("/api/v1/spectrum?ra=337.878&dec=-5.189&aperture_px=0", 400),
        ("/api/v1/spectrum?ra=337.878&dec=-5.189&aperture_px=2", 400),
        ("/api/v1/spectrum?ra=337.878&dec=-5.189&aperture_px=7", 400),
    ];
    let mut failures = Vec::new();
    for (uri, want) in cases {
        let Some(Ok((status, _h, body))) = call_guarded(&app, &Method::GET, uri, &[]).await else {
            failures.push(format!("{uri}: could not be served"));
            continue;
        };
        if status.as_u16() != *want {
            failures.push(format!("{uri}: got {status}, want {want}; body {body:?}"));
            continue;
        }
        if status.is_client_error() {
            if let Err(e) = check_error_body(&body) {
                failures.push(format!("{uri}: {e}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "curated failures:\n{}",
        failures.join("\n")
    );
}

/// Long-string battery: large values inside and outside the grammar,
/// checking timing and response-size bounds. `http::Uri` rejects
/// request targets longer than u16::MAX, so oneshot coverage tops out
/// at 60 KiB; multi-hundred-KiB request lines are exercised over raw
/// TCP in `security_tests`.
#[tokio::test]
async fn very_long_query_values_are_handled() {
    let app = app(state(StubSource::happy(4)));
    let mut slowest = Duration::ZERO;
    for (len, ch) in [
        (60_000usize, '1'),
        (60_000, 'A'),
        (40_000, '%'),
        (32_768, ';'),
        (8192, '\u{1f680}'), // rocket, multi-byte
    ] {
        let v: String = std::iter::repeat_n(ch, len).collect();
        let encoded = pct_encode(&v);
        if encoded.len() > u16::MAX as usize - 512 {
            // Encoded form exceeds the u16::MAX request-target bound of
            // `http::Uri`; this size is only reachable over raw TCP.
            continue;
        }
        let uri = format!("/api/v1/frames?ra={}&dec=0", encoded);
        let t0 = Instant::now();
        let Some(res) = call_guarded(&app, &Method::GET, &uri, &[]).await else {
            panic!("unbuildable URI of len {len}");
        };
        slowest = slowest.max(t0.elapsed());
        let (status, _h, body) = res.expect("no panic");
        assert!(status.is_client_error(), "len {len} '{ch}': {status}");
        check_error_body(&body).expect("error body");
        assert!(body.len() < MAX_ERROR_BODY, "echoed long input");
    }
    // Document the parser-level bound: >64 KiB targets cannot be
    // represented as `http::Uri` at all (hyper rejects them upstream).
    let huge = format!("/api/v1/frames?ra={}&dec=0", "1".repeat(1 << 20));
    assert!(
        axum::http::Request::builder()
            .uri(&huge)
            .body(axum::body::Body::empty())
            .is_err(),
        "expected 1 MiB request target to be unrepresentable"
    );
    eprintln!(
        "long-value battery slowest: {:.1} ms",
        slowest.as_secs_f64() * 1000.0
    );
}

// ---------------------------------------------------------------------
// Property-style tests of the coordinate grammar
// ---------------------------------------------------------------------

/// RA degrees -> `HHhMMmSS.sssss` sexagesimal (hours = deg/15).
fn ra_to_sexa(deg: f64) -> String {
    let total_h = deg / 15.0;
    let h = total_h.floor();
    let rem_m = (total_h - h) * 60.0;
    let m = rem_m.floor();
    let s = ((rem_m - m) * 60.0 * 1e5).round() / 1e5;
    let (s, m) = if s >= 60.0 { (0.0, m + 1.0) } else { (s, m) };
    let (m, h) = if m >= 60.0 { (0.0, h + 1.0) } else { (m, h) };
    format!("{h:.0}h{m:.0}m{s:.5}s")
}

/// Dec degrees -> `[+-]DDdMMmSS.sssss`.
fn dec_to_sexa(deg: f64) -> String {
    let (sign, a) = if deg < 0.0 || deg.is_sign_negative() {
        ("-", -deg)
    } else {
        ("", deg)
    };
    let d = a.floor();
    let rem_m = (a - d) * 60.0;
    let m = rem_m.floor();
    let s = ((rem_m - m) * 60.0 * 1e5).round() / 1e5;
    let (s, m) = if s >= 60.0 { (0.0, m + 1.0) } else { (s, m) };
    let (m, d) = if m >= 60.0 { (0.0, d + 1.0) } else { (m, d) };
    format!("{sign}{d:.0}d{m:.0}m{s:.5}s")
}

#[test]
fn roundtrip_decimal_sexagesimal() {
    let mut rng = Rng::new(0xb6b6_0000_0000_0003);
    let mut worst_ra = 0.0f64;
    let mut worst_dec = 0.0f64;
    for _ in 0..4000 {
        let ra = rng.between(0.0, 359.9999);
        let dec = rng.between(-90.0, 90.0);
        let (pra, pdec) =
            parse_coordinates(&ra_to_sexa(ra), &dec_to_sexa(dec)).expect("sexagesimal parses");
        worst_ra = worst_ra.max((pra - ra).abs());
        worst_dec = worst_dec.max((pdec - dec).abs());
        assert!(
            (pra - ra).abs() < 1e-6,
            "ra {ra} -> {} -> {pra}",
            ra_to_sexa(ra)
        );
        assert!(
            (pdec - dec).abs() < 1e-6,
            "dec {dec} -> {} -> {pdec}",
            dec_to_sexa(dec)
        );
        // And the same angles through decimal spelling.
        let (dra, ddec) =
            parse_coordinates(&format!("{ra:.9}"), &format!("{dec:.9}")).expect("decimal parses");
        assert!((dra - ra).abs() < 1e-6, "decimal ra drift");
        assert!((ddec - dec).abs() < 1e-6, "decimal dec drift");
        validate_query_params(pra, pdec, 0.05).expect("round-tripped values validate");
    }
    eprintln!("roundtrip worst: ra {worst_ra:.3e} deg, dec {worst_dec:.3e} deg");
}

#[test]
fn out_of_range_values_always_rejected() {
    let mut rng = Rng::new(0xb6b6_0000_0000_0004);
    // Pinned boundaries.
    for (ra, dec, fov) in [
        (-1e-9, 0.0, 0.05),
        (360.0, 0.0, 0.05),
        (360.0000000001, 0.0, 0.05),
        (0.0, -90.0000000001, 0.05),
        (0.0, 90.0000000001, 0.05),
        (0.0, 0.0, 0.009999999),
        (0.0, 0.0, 0.5000001),
        (f64::NAN, 0.0, 0.05),
        (0.0, f64::INFINITY, 0.05),
        (0.0, 0.0, f64::NEG_INFINITY),
        (f64::MAX, 0.0, 0.05),
        (0.0, f64::MIN, 0.05),
        (0.0, 0.0, 0.0),
        (0.0, 0.0, -0.0),
    ] {
        assert!(
            validate_query_params(ra, dec, fov).is_err(),
            "accepted ra={ra} dec={dec} fov={fov}"
        );
    }
    // Random out-of-range values.
    for _ in 0..2000 {
        let ra = rng.between(-1e6, 1e6);
        if !(0.0..360.0).contains(&ra) {
            assert!(validate_query_params(ra, 0.0, 0.05).is_err(), "ra {ra}");
        }
        let dec = rng.between(-1e6, 1e6);
        if !(-90.0..=90.0).contains(&dec) {
            assert!(validate_query_params(0.0, dec, 0.05).is_err(), "dec {dec}");
        }
        let fov = rng.between(-1e3, 1e3);
        if !(0.01..=0.5).contains(&fov) {
            assert!(validate_query_params(0.0, 0.0, fov).is_err(), "fov {fov}");
        }
    }
    // Boundary-exact acceptance.
    for (ra, dec, fov) in [
        (0.0, -90.0, 0.01),
        (359.999999999, 90.0, 0.5),
        (f64::from_bits(360.0f64.to_bits() - 1), 0.0, 0.05),
    ] {
        assert!(
            validate_query_params(ra, dec, fov).is_ok(),
            "rejected ra={ra} dec={dec} fov={fov}"
        );
    }
}

#[test]
fn parser_never_panics_on_arbitrary_bytes() {
    let mut rng = Rng::new(0xb6b6_0000_0000_0005);
    for i in 0..6000u32 {
        let len = rng.below(65);
        let bytes: Vec<u8> = (0..len).map(|_| rng.next_u64() as u8).collect();
        let a = String::from_utf8_lossy(&bytes).into_owned();
        let b = {
            let bytes: Vec<u8> = (0..rng.below(33)).map(|_| rng.next_u64() as u8).collect();
            String::from_utf8_lossy(&bytes).into_owned()
        };
        let res =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse_coordinates(&a, &b)));
        assert!(res.is_ok(), "panicked on case {i}: {a:?} / {b:?}");
        // Whether Ok or Err is irrelevant: the contract is "no panic".
    }
}

/// Byte-level grammar mutant: take a valid sexagesimal string and
/// splice random bytes into it; parser must return, never panic.
#[test]
fn sexagesimal_mutants_never_panic() {
    let mut rng = Rng::new(0xb6b6_0000_0000_0006);
    for i in 0..2000u32 {
        let base = *rng.pick(SEXA);
        let mut s = base.to_string();
        match rng.below(4) {
            0 => {
                let pos = rng.below(s.len() + 1);
                let c = char::from_u32(rng.below(0x7F) as u32).unwrap_or('x');
                s.insert(pos.min(s.len()), c);
            }
            1 => {
                if !s.is_empty() {
                    s.truncate(rng.below(s.len()) + 1);
                }
            }
            2 => s.push_str(base),
            _ => {
                s = format!("{s}{}", ascii_garbage(&mut rng));
            }
        }
        let res =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse_coordinates(&s, "0")));
        assert!(res.is_ok(), "panicked on mutant {i}: {s:?}");
    }
}

/// Idempotence/stability: parsing is deterministic — same input, same
/// output, no state leakage between calls.
#[test]
fn parser_is_deterministic() {
    let mut rng = Rng::new(0xb6b6_0000_0000_0007);
    for _ in 0..500 {
        let a = value(&mut rng);
        let b = value(&mut rng);
        let r1 = parse_coordinates(&a, &b);
        let r2 = parse_coordinates(&a, &b);
        match (r1, r2) {
            (Ok((ra1, de1)), Ok((ra2, de2))) => {
                assert_eq!(ra1.to_bits(), ra2.to_bits());
                assert_eq!(de1.to_bits(), de2.to_bits());
            }
            (Err(_), Err(_)) => {}
            _ => panic!("non-deterministic parse of {a:?} / {b:?}"),
        }
    }
}

/// `StatusCode` sanity for `AppError` used via the router (kept here so
/// the fuzz file also pins the 4xx surface end-to-end).
#[tokio::test]
async fn health_unaffected_by_adjacent_bad_queries() {
    let app = app(state(StubSource::happy(4)));
    let mut rng = Rng::new(SEED_FRAMES);
    for _ in 0..200 {
        let case = frames_case(&mut rng);
        let _ = call_guarded(&app, &Method::GET, &case.uri, &[]).await;
    }
    let (status, _h, body) = call(&app, &Method::GET, "/api/v1/health", &[])
        .await
        .expect("request builds");
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(v["status"], "ok");
}

// ---------------------------------------------------------------------
// Bugs found by this battery (kept green via #[ignore]; see handoff)
// ---------------------------------------------------------------------

/// Contract: `dec`/`ra` must resolve to -90..=90 / [0,360) — violating
/// values deserve a 400 `invalid_parameter`. `spectrum_params` never
/// calls `validate_query_params`, so out-of-range coordinates sail
/// through and surface as `not_found` (404) instead.
#[tokio::test]
#[ignore = "BUG: /api/v1/spectrum does not range-check ra/dec; out-of-range coordinates return 404 not_found instead of 400 invalid_parameter"]
async fn spectrum_range_checks_coordinates() {
    let app = app(state(StubSource::happy(4)));
    for uri in [
        "/api/v1/spectrum?ra=400&dec=0",
        "/api/v1/spectrum?ra=-1&dec=0",
        "/api/v1/spectrum?ra=0&dec=91",
        "/api/v1/spectrum?ra=0&dec=-91",
        "/api/v1/spectrum?ra=0&dec=inf",
    ] {
        let (status, _h, body) = call(&app, &Method::GET, uri, &[]).await.expect("req");
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri} -> {status}");
        let code = check_error_body(&body).unwrap_or_else(|e| panic!("{uri}: {e}"));
        assert_eq!(code, "invalid_parameter", "{uri}");
    }
}

/// Filtering every cutout out via `detector` yields 200 + empty
/// `frames` in mode raw, but `group_frames`/`stack_detectors` map an
/// empty input to `ScienceError::EmptyInput` -> `NotFound` in
/// diff/coadd/stack. Inconsistent with raw, and the contract's 404
/// means "no observations at this position", which is false here.
#[tokio::test]
#[ignore = "BUG: /frames?detector=N with no matching cutouts returns 404 in diff/coadd/stack modes (raw returns 200 + empty frames)"]
async fn detector_filtered_empty_group_is_200_not_404() {
    let app = app(state(StubSource::happy(4))); // all cutouts are detector 3
    for mode in ["diff", "coadd", "stack"] {
        let uri = format!("/api/v1/frames?ra=337.878&dec=-5.189&detector=1&mode={mode}");
        let (status, _h, body) = call(&app, &Method::GET, &uri, &[]).await.expect("req");
        assert_eq!(status, StatusCode::OK, "{uri} -> {status}: {body:?}");
        let v: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(v["frames"].as_array().map(Vec::len), Some(0), "{uri}");
    }
}
