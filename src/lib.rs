#![deny(missing_docs)]

//! Wrapper for using libc's `printf("%g")` format for your floating point output

use libc::c_char;
use std::fmt;
use std::io::Write;

/// A wrapper around floats providing an implementation of `Display` which uses
/// the underlying `libc`'s `printf()` with format `"%g"`, for when you need to
/// match exactly what C a program would output.
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

const FORMAT_SIZE: usize = 20;
const NUMSTR_SIZE: usize = 200;

fn fmt_g(formatter: &mut fmt::Formatter<'_>, value: f64) -> fmt::Result {
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
        (Some(w), Some(p)) => write!(fmtbuf, "%{}{}{}{}.{}g", alternate, sign_pad, zero_pad, w, p),
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
