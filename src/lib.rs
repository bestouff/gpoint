#![deny(missing_docs)]

//! Wrapper for using C's `printf("%g")` format for your floating point output.
//!
//! By default, formatting is done in pure Rust. Enable the `libc` feature to
//! delegate to your system's `snprintf()` instead (only available on Unix).

use std::fmt;

/// A wrapper around floats providing an implementation of `Display` which uses
/// C's `printf()` `"%g"` format, for when you need to match exactly what a C
/// program would output.
///
/// `Float` should be a floating point type, i.e. `f32` or `f64`.
///
/// Available formatting options:
/// ```
/// use gpoint::GPoint;
///
/// assert!(format!("{}",    GPoint(42f32))  == "42");
/// assert!(format!("{}",    GPoint(42f64))  == "42");
/// assert!(format!("{:.3}", GPoint(1.2345)) == "1.23");
/// assert!(format!("{:4}",  GPoint(42.))    == "  42");
/// assert!(format!("{:-4}", GPoint(42.))    == "42  ");
/// assert!(format!("{:04}", GPoint(42.))    == "0042");
/// assert!(format!("{:+}",  GPoint(42.))    == "+42");
/// assert!(format!("{:#4}", GPoint(42.))    == "42.0000");
/// ```
#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct GPoint<Float>(
    /// Your floating point number you want to `Display`
    pub Float,
);

impl std::fmt::Display for GPoint<f64> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_g(f, self.0)
    }
}

impl std::fmt::Display for GPoint<f32> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_g(f, self.0 as f64)
    }
}

// ---------------------------------------------------------------------------
// libc-based implementation (Unix only, behind "libc" feature)
// ---------------------------------------------------------------------------

#[cfg(all(feature = "libc", unix))]
fn fmt_g(formatter: &mut fmt::Formatter<'_>, value: f64) -> fmt::Result {
    use libc::c_char;
    use std::io::Write;

    const FORMAT_SIZE: usize = 20;
    const NUMSTR_SIZE: usize = 200;

    let mut format = [0u8; FORMAT_SIZE];
    let numstr = [0u8; NUMSTR_SIZE];
    let mut fmtbuf = std::io::Cursor::new(&mut format[..FORMAT_SIZE - 1]); // keep final 0

    let zero_pad = if formatter.sign_aware_zero_pad() {
        "0"
    } else {
        ""
    };
    let sign_pad = if formatter.sign_minus() {
        "-"
    } else if formatter.sign_plus() {
        "+"
    } else {
        ""
    };
    let alternate = if formatter.alternate() { "#" } else { "" };
    match (formatter.width(), formatter.precision()) {
        (None, None) => write!(fmtbuf, "%{}{}g", alternate, sign_pad),
        (Some(w), None) => write!(fmtbuf, "%{}{}{}{}g", alternate, sign_pad, zero_pad, w),
        (None, Some(p)) => write!(fmtbuf, "%{}.{}g", alternate, p),
        (Some(w), Some(p)) => {
            write!(fmtbuf, "%{}{}{}{}.{}g", alternate, sign_pad, zero_pad, w, p)
        }
    }
    .map_err(|_| fmt::Error)?;
    let nbchars = unsafe {
        libc::snprintf(
            numstr.as_ptr() as *mut c_char,
            NUMSTR_SIZE,
            format.as_ptr() as *const c_char,
            value,
        )
    };
    // check if we (virtually) overflowed our buffer
    if nbchars < 0 || nbchars >= NUMSTR_SIZE as i32 {
        return Err(fmt::Error);
    }
    let numstr = &numstr[..nbchars as usize];

    formatter.write_str(unsafe { std::str::from_utf8_unchecked(numstr) })
}

// ---------------------------------------------------------------------------
// Pure Rust implementation (default, works on all platforms)
// ---------------------------------------------------------------------------

#[cfg(not(all(feature = "libc", unix)))]
fn fmt_g(formatter: &mut fmt::Formatter<'_>, value: f64) -> fmt::Result {
    // Precision: %g default is 6, precision 0 is treated as 1
    let prec = match formatter.precision() {
        None => 6,
        Some(0) => 1,
        Some(p) => p,
    };

    // Format the core number (no width/padding yet)
    let mut buf = [0u8; 200];
    let core_str = fmt_g_core(
        value,
        prec,
        formatter.alternate(),
        formatter.sign_plus(),
        &mut buf,
    );

    // Apply width and padding via Formatter
    // We need to handle zero-padding, left-align, and right-align ourselves
    // because formatter.pad_integral doesn't know about our sign handling.
    let width = formatter.width().unwrap_or(0);
    let len = core_str.len();

    if len >= width {
        formatter.write_str(core_str)
    } else {
        let pad_count = width - len;
        let zero_pad = formatter.sign_aware_zero_pad();
        let left_align = formatter.sign_minus();

        if left_align {
            // Left-aligned: content then spaces
            formatter.write_str(core_str)?;
            for _ in 0..pad_count {
                formatter.write_str(" ")?;
            }
            Ok(())
        } else if zero_pad {
            // Zero-padded: sign, then zeros, then digits
            // For NaN/inf, zero-padding acts like space-padding (C behavior)
            if value.is_nan() || value.is_infinite() {
                for _ in 0..pad_count {
                    formatter.write_str(" ")?;
                }
                formatter.write_str(core_str)
            } else {
                // Split off the sign if present
                let (sign, digits) = if core_str.starts_with('-') || core_str.starts_with('+') {
                    core_str.split_at(1)
                } else {
                    ("", core_str)
                };
                formatter.write_str(sign)?;
                for _ in 0..pad_count {
                    formatter.write_str("0")?;
                }
                formatter.write_str(digits)
            }
        } else {
            // Right-aligned with spaces (default)
            for _ in 0..pad_count {
                formatter.write_str(" ")?;
            }
            formatter.write_str(core_str)
        }
    }
}

/// Format the core number string (sign + digits, no width padding).
/// Returns a string slice into `buf`.
#[cfg(not(all(feature = "libc", unix)))]
fn fmt_g_core(
    value: f64,
    prec: usize,
    alternate: bool,
    show_plus: bool,
    buf: &mut [u8; 200],
) -> &str {
    use std::io::Write;

    let mut cursor = std::io::Cursor::new(&mut buf[..]);

    if value.is_nan() {
        let sign = if show_plus {
            "+"
        } else if value.is_sign_negative() {
            "-"
        } else {
            ""
        };
        let _ = write!(cursor, "{}nan", sign);
    } else if value.is_infinite() {
        let sign = if value.is_sign_negative() {
            "-"
        } else if show_plus {
            "+"
        } else {
            ""
        };
        let _ = write!(cursor, "{}inf", sign);
    } else {
        // Determine sign prefix
        let (sign, abs_val) = if value.is_sign_negative() && value != 0.0 {
            ("-", -value)
        } else if show_plus {
            ("+", value)
        } else {
            ("", value)
        };

        // %g algorithm:
        // 1. Format as scientific to find exponent X
        // 2. If -4 <= X < prec: use fixed with (prec - 1 - X) decimal places
        // 3. Otherwise: use scientific with (prec - 1) decimal places
        // 4. Unless # flag: trim trailing zeros and trailing decimal point

        if abs_val == 0.0 {
            // Special case: zero
            let _ = write!(cursor, "{}", sign);
            if alternate {
                // With #, show trailing zeros up to precision
                if prec <= 1 {
                    let _ = write!(cursor, "0.");
                } else {
                    let _ = write!(cursor, "0.");
                    for _ in 0..prec - 1 {
                        let _ = write!(cursor, "0");
                    }
                }
            } else {
                let _ = write!(cursor, "0");
            }
        } else {
            let exponent = abs_val.log10().floor() as i32;

            if exponent >= -4 && exponent < prec as i32 {
                // Fixed notation
                let decimal_digits = (prec as i32 - 1 - exponent).max(0) as usize;
                let _ = write!(cursor, "{}{:.prec$}", sign, abs_val, prec = decimal_digits);
                if !alternate {
                    // Trim trailing zeros and decimal point
                    trim_trailing_zeros(&mut cursor);
                } else if decimal_digits == 0 {
                    // Alternate form: ensure decimal point is present
                    let _ = write!(cursor, ".");
                }
            } else {
                // Scientific notation
                let sci_prec = prec - 1;
                // Use Rust's {:e} and then normalize the exponent format to match C
                let _ = write!(cursor, "{}{:.prec$e}", sign, abs_val, prec = sci_prec);
                if !alternate {
                    trim_trailing_zeros_sci(&mut cursor);
                }
                normalize_exponent(&mut cursor);
            }
        }
    }

    let pos = cursor.position() as usize;
    // SAFETY: we only wrote ASCII characters
    unsafe { std::str::from_utf8_unchecked(&cursor.into_inner()[..pos]) }
}

/// Trim trailing zeros (and trailing decimal point) from a fixed-point number in the cursor.
#[cfg(not(all(feature = "libc", unix)))]
fn trim_trailing_zeros(cursor: &mut std::io::Cursor<&mut [u8]>) {
    let pos = cursor.position() as usize;
    let buf = cursor.get_ref();
    // Only trim if there's a decimal point
    let has_dot = buf[..pos].contains(&b'.');
    if !has_dot {
        return;
    }
    let mut new_pos = pos;
    while new_pos > 0 && buf[new_pos - 1] == b'0' {
        new_pos -= 1;
    }
    if new_pos > 0 && buf[new_pos - 1] == b'.' {
        new_pos -= 1;
    }
    cursor.set_position(new_pos as u64);
}

/// Trim trailing zeros from the mantissa part of a scientific-notation number,
/// i.e. trim zeros before the 'e'.
#[cfg(not(all(feature = "libc", unix)))]
fn trim_trailing_zeros_sci(cursor: &mut std::io::Cursor<&mut [u8]>) {
    let pos = cursor.position() as usize;
    let buf = cursor.get_ref();
    // Find 'e'
    let e_pos = match buf[..pos].iter().position(|&b| b == b'e') {
        Some(p) => p,
        None => return,
    };
    // Trim zeros before 'e'
    let mut trim_end = e_pos;
    while trim_end > 0 && buf[trim_end - 1] == b'0' {
        trim_end -= 1;
    }
    if trim_end > 0 && buf[trim_end - 1] == b'.' {
        trim_end -= 1;
    }
    // Shift the exponent part
    if trim_end < e_pos {
        let exp_part_len = pos - e_pos;
        let dest = cursor.get_mut();
        dest.copy_within(e_pos..pos, trim_end);
        cursor.set_position((trim_end + exp_part_len) as u64);
    }
}

/// Normalize Rust's scientific exponent format to match C's %g:
/// - Rust uses `e1`, `e-1`, etc.
/// - C uses `e+01`, `e-01`, etc. (always sign, at least 2 digits)
#[cfg(not(all(feature = "libc", unix)))]
fn normalize_exponent(cursor: &mut std::io::Cursor<&mut [u8]>) {
    let pos = cursor.position() as usize;
    let buf = cursor.get_ref();
    let e_pos = match buf[..pos].iter().position(|&b| b == b'e') {
        Some(p) => p,
        None => return,
    };

    // Parse the exponent substring (after 'e')
    let exp_str = unsafe { std::str::from_utf8_unchecked(&buf[e_pos + 1..pos]) };
    let exp_val: i32 = exp_str.parse().unwrap_or(0);

    // Rewrite from 'e' onward
    let dest = cursor.get_mut();
    let mut temp = [0u8; 20];
    let mut tc = std::io::Cursor::new(&mut temp[..]);
    use std::io::Write;
    if exp_val >= 0 {
        let _ = write!(tc, "e+{:02}", exp_val);
    } else {
        let _ = write!(tc, "e-{:02}", -exp_val);
    }
    let tc_len = tc.position() as usize;
    dest[e_pos..e_pos + tc_len].copy_from_slice(&temp[..tc_len]);
    cursor.set_position((e_pos + tc_len) as u64);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn simple() {
        for (num, res) in [
            (42., "42"),
            (f64::NAN, "nan"),
            (-f64::INFINITY, "-inf"),
            (f64::INFINITY, "inf"),
        ] {
            let num = GPoint(num);
            assert_eq!(&format!("{}", num), res);
        }
    }
    #[test]
    fn pad() {
        for (num, res) in [
            (42., "      42"),
            (f64::NAN, "     nan"),
            (-f64::INFINITY, "    -inf"),
            (f64::INFINITY, "     inf"),
        ] {
            let num = GPoint(num);
            assert_eq!(&format!("{:8}", num), res);
        }
    }
    #[test]
    fn zero_pad() {
        for (num, res) in [
            (42., "00000042"),
            (-1.01, "-0001.01"),
            (f64::NAN, "     nan"),
            (-f64::INFINITY, "    -inf"),
            (f64::INFINITY, "     inf"),
        ] {
            let num = GPoint(num);
            assert_eq!(&format!("{:08}", num), res);
        }
    }
    #[test]
    fn minus_pad() {
        for (num, res) in [
            (42., "42      "),
            (-1.01, "-1.01   "),
            (f64::NAN, "nan     "),
            (-f64::INFINITY, "-inf    "),
            (f64::INFINITY, "inf     "),
        ] {
            let num = GPoint(num);
            assert_eq!(&format!("{:-8}", num), res);
        }
    }
    #[test]
    fn plus() {
        for (num, res) in [
            (42., "+42"),
            (-1.01, "-1.01"),
            (f64::NAN, "+nan"),
            (-f64::INFINITY, "-inf"),
            (f64::INFINITY, "+inf"),
        ] {
            let num = GPoint(num);
            assert_eq!(&format!("{:+}", num), res);
        }
    }
    #[test]
    fn plus_pad() {
        for (num, res) in [
            (42., "     +42"),
            (-1.01, "   -1.01"),
            (f64::NAN, "    +nan"),
            (-f64::INFINITY, "    -inf"),
            (f64::INFINITY, "    +inf"),
        ] {
            let num = GPoint(num);
            assert_eq!(&format!("{:+8}", num), res);
        }
    }
    #[test]
    fn prec() {
        for (num, res) in [
            (42., "42"),
            (-1.012345678901, "-1.01"),
            (-42.8952, "-42.9"),
            (4321., "4.32e+03"),
        ] {
            let num = GPoint(num);
            assert_eq!(&format!("{:.3}", num), res);
        }
    }
    #[test]
    fn alt() {
        for (num, res) in [
            (42., "42.0000"),
            (-1.012345678901, "-1.01235"),
            (432100., "432100."),
        ] {
            let num = GPoint(num);
            assert_eq!(&format!("{:#}", num), res);
        }
    }
    #[test]
    fn in_context() {
        assert_eq!(&format!("answer={}!", GPoint(42.)), "answer=42!");
    }
}
