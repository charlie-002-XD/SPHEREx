//! Coordinate and query-parameter validation (task b1).
//!
//! Accepted coordinate grammar (case-insensitive units):
//! - decimal degrees, e.g. `133.795129`, `-7.245147`, `+30.4`, `1.5e1`;
//! - sexagesimal `HHhMMmSS[.sss]s` for RA (hours, scaled x15) and
//!   `[+|-]DDdMMmSS[.sss]s` for Dec. Trailing components may be omitted
//!   (`10h30m`, `-7d14m`); minutes and seconds must be < 60. For RA the
//!   first unit may also be `d` (already degrees, no x15 scaling).
//!
//! Anything outside this grammar (NaN/inf spellings, hex, embedded
//! whitespace, punctuation such as `;`) is rejected with HTTP 400 and the
//! raw input is never echoed in the error body.

use crate::error::AppError;

/// Inclusive/exclusive bounds from the API contract and project rules.
pub const RA_MIN_DEG: f64 = 0.0;
/// RA upper bound, exclusive.
pub const RA_MAX_DEG: f64 = 360.0;
pub const DEC_MIN_DEG: f64 = -90.0;
pub const DEC_MAX_DEG: f64 = 90.0;
pub const FOV_MIN_DEG: f64 = 0.01;
pub const FOV_MAX_DEG: f64 = 0.5;

/// A validated sky query, in decimal degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Query {
    pub ra_deg: f64,
    pub dec_deg: f64,
    pub fov_deg: f64,
}

/// Parse RA and Dec strings into decimal degrees.
///
/// Accepts sexagesimal (`08h55m10.831s`, `-7d14m42.53s`) and decimal
/// degrees. The result is NOT range-checked: pass it through
/// [`validate_query_params`] before use.
pub fn parse_coordinates(ra: &str, dec: &str) -> Result<(f64, f64), AppError> {
    let ra_deg = parse_angle(ra, Coord::Ra).map_err(|_| {
        AppError::invalid_parameter(
            "ra: expected decimal degrees or sexagesimal (e.g. 08h55m10.8s)",
        )
    })?;
    let dec_deg = parse_angle(dec, Coord::Dec).map_err(|_| {
        AppError::invalid_parameter(
            "dec: expected decimal degrees or sexagesimal (e.g. -7d14m42.5s)",
        )
    })?;
    Ok((ra_deg, dec_deg))
}

/// Validate parsed angles and the field of view.
///
/// Rejects NaN/infinite values and enforces `0 <= ra < 360`,
/// `-90 <= dec <= 90`, `0.01 <= fov <= 0.5` (all degrees). Any violation
/// maps to HTTP 400 via [`AppError`].
pub fn validate_query_params(ra_deg: f64, dec_deg: f64, fov_deg: f64) -> Result<Query, AppError> {
    for (name, value) in [("ra", ra_deg), ("dec", dec_deg), ("fov", fov_deg)] {
        if !value.is_finite() {
            return Err(AppError::invalid_parameter(format!(
                "{name}: must be a finite number"
            )));
        }
    }
    if !(RA_MIN_DEG..RA_MAX_DEG).contains(&ra_deg) {
        return Err(AppError::invalid_parameter("ra: out of range [0, 360) deg"));
    }
    if !(DEC_MIN_DEG..=DEC_MAX_DEG).contains(&dec_deg) {
        return Err(AppError::invalid_parameter(
            "dec: out of range [-90, 90] deg",
        ));
    }
    if !(FOV_MIN_DEG..=FOV_MAX_DEG).contains(&fov_deg) {
        return Err(AppError::invalid_parameter(
            "fov: out of range [0.01, 0.5] deg",
        ));
    }
    Ok(Query {
        ra_deg,
        dec_deg,
        fov_deg,
    })
}

/// Which celestial coordinate a string represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Coord {
    /// Right ascension: sexagesimal `h` components are scaled x15.
    Ra,
    /// Declination: sexagesimal components are already in degrees.
    Dec,
}

/// Parse one coordinate string into decimal degrees (RA hours scaled x15).
fn parse_angle(raw: &str, coord: Coord) -> Result<f64, ()> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(());
    }
    let has_unit = s
        .bytes()
        .any(|b| matches!(b.to_ascii_lowercase(), b'h' | b'd' | b'm' | b's'));
    if has_unit {
        parse_sexagesimal(s, coord)
    } else {
        parse_decimal(s)
    }
}

/// Strict decimal grammar: `[+-]? <digits>[.<digits>][(e|E)[+-]?<digits>]`.
/// The character whitelist alone rejects `NaN`, `inf` and punctuation.
fn parse_decimal(s: &str) -> Result<f64, ()> {
    if !s
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.' | b'e' | b'E'))
    {
        return Err(());
    }
    let value: f64 = s.parse().map_err(|_| ())?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(())
    }
}

/// Sexagesimal grammar: `[+-]? <num><u0> [<num>m [<num>s]]` where `u0` is
/// `h` (RA only, x15) or `d`. Components are strictly ordered; minutes and
/// seconds must be < 60. Returns decimal degrees.
fn parse_sexagesimal(s: &str, coord: Coord) -> Result<f64, ()> {
    let (sign, rest) = match s.as_bytes().first() {
        Some(b'-') => (-1.0, &s[1..]),
        Some(b'+') => (1.0, &s[1..]),
        _ => (1.0, s),
    };
    let bytes = rest.as_bytes();
    if bytes.is_empty() {
        return Err(());
    }

    /// Divisor applied to each component (units, minutes, seconds).
    const DIV: [f64; 3] = [1.0, 60.0, 3600.0];
    let mut value = 0.0;
    let mut component = 0usize;
    let mut first_unit_hours = false;
    let mut i = 0usize;

    while i < bytes.len() {
        // Numeric part: digits with at most one '.', at least one digit.
        let start = i;
        let mut digits = 0usize;
        let mut seen_dot = false;
        while i < bytes.len() {
            match bytes[i] {
                b'0'..=b'9' => {
                    digits += 1;
                    i += 1;
                }
                b'.' if !seen_dot => {
                    seen_dot = true;
                    i += 1;
                }
                _ => break,
            }
        }
        if digits == 0 || i >= bytes.len() {
            return Err(());
        }
        let part: f64 = rest[start..i].parse().map_err(|_| ())?;
        if !part.is_finite() {
            return Err(());
        }

        let unit = bytes[i].to_ascii_lowercase();
        i += 1;
        match component {
            0 => match (coord, unit) {
                (Coord::Ra, b'h') => first_unit_hours = true,
                (_, b'd') => {}
                _ => return Err(()),
            },
            1 | 2 => {
                let expected = if component == 1 { b'm' } else { b's' };
                if unit != expected || part >= 60.0 {
                    return Err(());
                }
            }
            _ => return Err(()),
        }
        value += part / DIV[component];
        component += 1;
    }
    if component == 0 {
        return Err(());
    }

    let factor = if first_unit_hours { 15.0 } else { 1.0 };
    let degrees = sign * value * factor;
    if degrees.is_finite() {
        Ok(degrees)
    } else {
        Err(())
    }
}
