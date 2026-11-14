//! Integration tests for task b1: coordinate/query validation and AppError.
//! All inputs are synthetic; no IRSA access.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use http_body_util::BodyExt;
use spherex_backend::api::validation::{parse_coordinates, validate_query_params};
use spherex_backend::error::AppError;

const TOL: f64 = 1e-6;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOL,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn wise_0855_sexagesimal() {
    let (ra, dec) = parse_coordinates("08h55m10.831s", "-7d14m42.53s").unwrap();
    assert_close(ra, 133.795129);
    assert_close(dec, -7.245147);

    // Zero-padded declination is equivalent.
    let (ra2, dec2) = parse_coordinates("08h55m10.831s", "-07d14m42.53s").unwrap();
    assert_close(ra2, ra);
    assert_close(dec2, dec);

    let q = validate_query_params(ra, dec, 0.05).unwrap();
    assert_close(q.fov_deg, 0.05);
}

#[test]
fn wise_0855_decimal_degrees() {
    let (ra, dec) = parse_coordinates("133.795129", "-7.245147").unwrap();
    assert_close(ra, 133.795129);
    assert_close(dec, -7.245147);
}

#[test]
fn benchmark_targets_parse_and_validate() {
    // The five official test targets (references/rules.md).
    let targets = [
        ("17h57m48.5s", "+04d41m36s"),      // Barnard's Star
        ("22h04m10.5s", "-56d46m58s"),      // Epsilon Indi B
        ("08h55m10.831s", "-07d14m42.53s"), // WISE 0855
        ("05h07m49.5s", "+30d24m05s"),      // RW Aurigae
        ("16h32m40.2s", "+66d45m17s"),      // R Draconis
    ];
    for (ra, dec) in targets {
        let (ra_deg, dec_deg) =
            parse_coordinates(ra, dec).unwrap_or_else(|e| panic!("{ra} {dec}: {e}"));
        let q = validate_query_params(ra_deg, dec_deg, 0.05)
            .unwrap_or_else(|e| panic!("{ra} {dec}: {e}"));
        assert!((0.0..360.0).contains(&q.ra_deg));
        assert!((-90.0..=90.0).contains(&q.dec_deg));
    }
}

#[test]
fn decimal_exponent_and_partial_sexagesimal() {
    let (ra, _) = parse_coordinates("1.5e1", "0").unwrap();
    assert_close(ra, 15.0);

    let (ra, _) = parse_coordinates("10h30m", "0").unwrap();
    assert_close(ra, 157.5);

    let (_, dec) = parse_coordinates("0", "+30d30m").unwrap();
    assert_close(dec, 30.5);
}

#[test]
fn rejects_out_of_range_values() {
    // Range violations are 400s, and parse itself succeeds on plain numbers.
    let (ra, _) = parse_coordinates("400", "0").unwrap();
    assert!(matches!(
        validate_query_params(ra, 0.0, 0.05),
        Err(AppError::InvalidParameter(_))
    ));
    assert!(validate_query_params(-1.0, 0.0, 0.05).is_err());
    assert!(validate_query_params(360.0, 0.0, 0.05).is_err());
    assert!(validate_query_params(0.0, -95.0, 0.05).is_err());
    assert!(validate_query_params(0.0, 91.0, 0.05).is_err());
    assert!(validate_query_params(0.0, 0.0, 10.0).is_err());
    assert!(validate_query_params(0.0, 0.0, 0.001).is_err());
}

#[test]
fn rejects_non_finite_values() {
    assert!(validate_query_params(f64::NAN, 0.0, 0.05).is_err());
    assert!(validate_query_params(0.0, f64::INFINITY, 0.05).is_err());
    assert!(validate_query_params(0.0, 0.0, f64::NEG_INFINITY).is_err());
    // NaN/inf spellings are outside the coordinate grammar too.
    assert!(parse_coordinates("NaN", "0").is_err());
    assert!(parse_coordinates("0", "inf").is_err());
    assert!(parse_coordinates("-Infinity", "0").is_err());
}

#[test]
fn rejects_injection_and_malformed_strings() {
    for bad in [
        "1; DROP TABLE",
        "0 OR 1=1",
        "<script>alert(1)</script>",
        "0x10",
        "10h20x",
        "10h70m", // minutes >= 60
        "-7d14m42.53s5",
        "10 30",
        "",
        "h",
        "10h30s", // components cannot be skipped
        "5s",     // first unit must be h or d
        "10h-30m",
    ] {
        assert!(parse_coordinates(bad, "0").is_err(), "ra accepted {bad:?}");
        assert!(parse_coordinates("0", bad).is_err(), "dec accepted {bad:?}");
    }
    // Dec in hours is malformed.
    assert!(parse_coordinates("0", "08h55m10.8s").is_err());
}

#[tokio::test]
async fn error_response_is_400_and_does_not_echo_input() {
    let err = parse_coordinates("1; DROP TABLE", "0").unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
    assert_eq!(err.code(), "invalid_parameter");

    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8_lossy(&bytes);
    assert!(body.contains("\"invalid_parameter\""), "body: {body}");
    assert!(!body.contains("DROP"), "body echoed input: {body}");
    assert!(!body.contains("TABLE"), "body echoed input: {body}");
}
