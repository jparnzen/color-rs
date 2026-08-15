// Copyright © 2026 John P. ARNZEN
// SPDX-License-Identifier: MIT

mod lexparse;

use anyhow::Result;
use clap::{Arg, command};
use color_lib::{Color, OKLAB, OKLCH, OKLrAB, OKLrCH, SRGB, Vector3, XYZD65};
use lexparse::*;
use std::collections::VecDeque;
use std::fmt::Display;

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
enum CliError {
    ParseError,
    ValueError,
    ColorError,
    ParseIntError(std::num::ParseIntError),
    ParseFloatError(std::num::ParseFloatError),
}

impl std::error::Error for CliError {}
impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::ParseError => write!(f, "parsing error"),
            CliError::ValueError => write!(f, "invalid value"),
            CliError::ColorError => write!(f, "invalid color specifier"),
            CliError::ParseIntError(e) => write!(f, "invalid integer: {e}"),
            CliError::ParseFloatError(e) => write!(f, "invalid floating point number: {e}"),
        }
    }
}

// Wrapper type for the type-safe Color<> structs
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug)]
enum CliColor {
    SRGB(Color<SRGB>),
    OKLCH(Color<OKLCH>),
    OKLAB(Color<OKLAB>),
    OKLrCH(Color<OKLrCH>),
    OKLrAB(Color<OKLrAB>),
}

impl CliColor {
    fn convert_to(&self, dest_format: &OutputFormat) -> CliColor {
        let xyz = self.to_XYZ();
        match dest_format {
            OutputFormat::Rgb => CliColor::SRGB(xyz.convert_to()),
            OutputFormat::Oklab => CliColor::OKLAB(xyz.convert_to()),
            OutputFormat::Oklch => CliColor::OKLCH(xyz.convert_to()),
            OutputFormat::Oklrab => CliColor::OKLrAB(xyz.convert_to()),
            OutputFormat::Oklrch => CliColor::OKLrCH(xyz.convert_to()),
        }
    }

    #[allow(non_snake_case)]
    fn to_XYZ(&self) -> Color<XYZD65> {
        match self {
            CliColor::SRGB(c) => c.convert_to(),
            CliColor::OKLAB(c) => c.convert_to(),
            CliColor::OKLCH(c) => c.convert_to(),
            CliColor::OKLrAB(c) => c.convert_to(),
            CliColor::OKLrCH(c) => c.convert_to(),
        }
    }

    fn gamutmap_minde(&self) -> CliColor {
        match self {
            CliColor::SRGB(c) => CliColor::SRGB(c.gamutmap_local_minde()),
            CliColor::OKLAB(c) => CliColor::SRGB(c.gamutmap_local_minde()),
            CliColor::OKLCH(c) => CliColor::SRGB(c.gamutmap_local_minde()),
            CliColor::OKLrAB(c) => CliColor::SRGB(c.gamutmap_local_minde()),
            CliColor::OKLrCH(c) => CliColor::SRGB(c.gamutmap_local_minde()),
        }
    }

    fn gamutmap_raytrace(&self) -> CliColor {
        match self {
            CliColor::SRGB(c) => CliColor::SRGB(c.gamutmap_raytrace()),
            CliColor::OKLAB(c) => CliColor::SRGB(c.gamutmap_raytrace()),
            CliColor::OKLCH(c) => CliColor::SRGB(c.gamutmap_raytrace()),
            CliColor::OKLrAB(c) => CliColor::SRGB(c.gamutmap_raytrace()),
            CliColor::OKLrCH(c) => CliColor::SRGB(c.gamutmap_raytrace()),
        }
    }

    fn clip(&self) -> CliColor {
        match self {
            CliColor::SRGB(c) => CliColor::SRGB(c.clip()),
            CliColor::OKLAB(c) => CliColor::SRGB(c.clip()),
            CliColor::OKLCH(c) => CliColor::SRGB(c.clip()),
            CliColor::OKLrAB(c) => CliColor::SRGB(c.clip()),
            CliColor::OKLrCH(c) => CliColor::SRGB(c.clip()),
        }
    }
}

impl Display for CliColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SIGDIG: i32 = 5;
        match self {
            CliColor::SRGB(color) => {
                let [r, g, b] = color.coords.map(|v| round_to_significant(v, SIGDIG));
                write!(
                    f,
                    "rgb({r} {g} {b}) || #{:02x}{:02x}{:02x}",
                    (r.clamp(0., 1.) * 255.).round() as u8,
                    (g.clamp(0., 1.) * 255.).round() as u8,
                    (b.clamp(0., 1.) * 255.).round() as u8
                )
            }
            CliColor::OKLAB(color) => {
                let [l, a, b] = color.coords.map(|v| round_to_significant(v, SIGDIG));
                write!(f, "oklab({l} {a} {b})")
            }
            CliColor::OKLCH(color) => {
                let [l, c, h] = color.coords.map(|v| round_to_significant(v, SIGDIG));
                write!(f, "oklch({l} {c} {h})")
            }
            CliColor::OKLrAB(color) => {
                let [lr, a, b] = color.coords.map(|v| round_to_significant(v, SIGDIG));
                write!(f, "color(--oklrab {lr} {a} {b})")
            }
            CliColor::OKLrCH(color) => {
                let [lr, c, h] = color.coords.map(|v| round_to_significant(v, SIGDIG));
                write!(f, "color(--oklrch {lr} {c} {h})")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, clap::ValueEnum)]
enum OutputFormat {
    Rgb,
    Oklch,
    Oklab,
    Oklrch,
    Oklrab,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum GamutMapAlgo {
    Clip,
    Minde,
    Raytrace,
}

fn main() -> Result<()> {
    let matches = command!()
        .arg(
            Arg::new("color_string")
                .required(true)
                .help("CSS-style input color string or RGB hexcode (e.g. '#aabbcc', 'oklch(0.5 0.21 180)', 'color(srgb 1.0 0.5 0.2)')")
        )
        .arg(
            Arg::new("format")
                .long("format")
                .required(true)
                .help("CSS-style output string in format color space")
                .value_parser(clap::value_parser!(OutputFormat)),
        )
        .arg(
            Arg::new("gma")
                .long("gma")
                .help("Gamut mapping algorithm to use for output string")
                .value_parser(clap::value_parser!(GamutMapAlgo)),
        )
        .get_matches();

    let color_string = matches.get_one::<String>("color_string").unwrap();
    let color_string = color_string.trim();

    let dest_format = matches.get_one::<OutputFormat>("format").unwrap();

    let input_color = if color_string.starts_with('#') {
        parse_rgbhex(color_string)?
    } else {
        parse_color_string(color_string)?
    };

    let dest_color = input_color.convert_to(dest_format);

    let (gma_color, gma_applied) = match matches.get_one::<GamutMapAlgo>("gma") {
        Some(GamutMapAlgo::Clip) => (dest_color.clip(), true),
        Some(GamutMapAlgo::Minde) => (dest_color.gamutmap_minde(), true),
        Some(GamutMapAlgo::Raytrace) => (dest_color.gamutmap_raytrace(), true),
        None => (dest_color, false),
    };

    // Prevent doing double-conversions to the same output/dest format
    let output_color = if gma_applied && dest_format != &OutputFormat::Rgb {
        gma_color.convert_to(dest_format)
    } else {
        gma_color
    };

    if let CliColor::SRGB(color) = &output_color
        && !color.in_gamut::<SRGB>()
    {
        eprintln!("WARNING: color is outside of sRGB gamut limits => hex values will be clipped.");
    }

    println!("{output_color}");

    Ok(())
}

/// Parse an RGB hex string (#rrggbb) to an sRGB color
fn parse_rgbhex(hex_str: &str) -> Result<CliColor, CliError> {
    if hex_str.len() != 7 {
        return Err(CliError::ParseError);
    }

    let r = u8::from_str_radix(&hex_str[1..=2], 16).map_err(CliError::ParseIntError)?;
    let g = u8::from_str_radix(&hex_str[3..=4], 16).map_err(CliError::ParseIntError)?;
    let b = u8::from_str_radix(&hex_str[5..=6], 16).map_err(CliError::ParseIntError)?;

    Ok(CliColor::SRGB(Color::<SRGB>::new([
        f64::from(r) / 255.,
        f64::from(g) / 255.,
        f64::from(b) / 255.,
    ])))
}

/// Parse the various CSS color spec functions (without alpha values):
/// rgb(r g b)
/// oklch(l c h)
/// oklab(l a b)
/// color(srgb r g b)
/// color(--oklch l c h)
/// color(--oklab l a b)
/// color(--oklrch lr c h)
/// color(--oklrab lr a b)
fn parse_color_string(color_string: &str) -> Result<CliColor, CliError> {
    let make: fn(Vector3) -> CliColor =
        if color_string.starts_with("rgb(") || color_string.starts_with("color(srgb ") {
            |v| CliColor::SRGB(Color::<SRGB>::new(v))
        } else if color_string.starts_with("oklch(") || color_string.starts_with("color(--oklch ") {
            |v| CliColor::OKLCH(Color::<OKLCH>::new(v))
        } else if color_string.starts_with("oklab(") || color_string.starts_with("color(--oklab ") {
            |v| CliColor::OKLAB(Color::<OKLAB>::new(v))
        } else if color_string.starts_with("color(--oklrch ") {
            |v| CliColor::OKLrCH(Color::<OKLrCH>::new(v))
        } else if color_string.starts_with("color(--oklrab ") {
            |v| CliColor::OKLrAB(Color::<OKLrAB>::new(v))
        } else {
            return Err(CliError::ColorError);
        };

    Ok(make(parse_values(color_string)?))
}

/// Parse the f64 values between parentheses, returning the first 3 f64s found.
/// If there are 4 values between the parens, and the first value isn't an f64,
/// it's assumed to be a color space ID and is discarded.
/// If there are more than 3 f64 values between the parens, then only the first
/// 3 f64 values are returned; the rest are discarded.
fn parse_values(s: &str) -> Result<Vector3, CliError> {
    // check for starting and ending parens
    let Some(lp_pos) = s.find('(') else {
        return Err(CliError::ParseError);
    };
    let Some(rp_pos) = s.find(')') else {
        return Err(CliError::ParseError);
    };

    // grab the values between the parens and trim off any whitespace
    let str_vals = s[lp_pos + 1..rp_pos].trim();
    let mut vals = str_vals.split_whitespace().collect::<VecDeque<_>>();
    if vals.len() < 3 {
        return Err(CliError::ValueError);
    }
    // if there are at least 4 items to parse, and the first item isn't parseable
    // as an f64, treat the first item as a color space identifier and discard it.
    if vals.len() >= 4 {
        vals.pop_front_if(|v| v.parse::<f64>().is_err());
        vals.truncate(3);
    }

    let parse = |s: &str| s.parse::<f64>().map_err(CliError::ParseFloatError);
    Ok([parse(vals[0])?, parse(vals[1])?, parse(vals[2])?])
}

// Tweaked from Google Gemini code re: rust get significant digits
fn round_to_significant(value: f64, digits: i32) -> f64 {
    if value == 0. {
        return 0.;
    }
    let multiplier = 10.0_f64.powi(digits - ((value.abs().log10().floor() as i32) + 1));
    (value * multiplier).round() / multiplier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rgbhex_success() {
        let src = "#aabbcc";
        let result = parse_rgbhex(src).unwrap();
        let CliColor::SRGB(result) = result else {
            panic!("Expected SRGB color from parse_rgbhex()");
        };
        let expected = [
            f64::from(0xaa) / 255.,
            f64::from(0xbb) / 255.,
            f64::from(0xcc) / 255.,
        ];

        assert_eq!(result.coords, expected)
    }

    #[test]
    fn test_parse_rgb_values_success() {
        let src = "rgb(1.0 0.5 0.2)";
        let result = parse_color_string(src).unwrap();
        let CliColor::SRGB(result) = result else {
            panic!("Expected SRGB color from parse_color_string()");
        };
        let expected = [1.0, 0.5, 0.2];

        assert_eq!(result.coords, expected)
    }

    #[test]
    fn test_parse_rgb_values_too_many_values() {
        let src = "color(srgb 1.0 0.5 0.2 / 0.8)";
        let result = parse_color_string(src).unwrap();
        let CliColor::SRGB(result) = result else {
            panic!("Expected SRGB color from parse_color_string()");
        };
        let expected = [1.0, 0.5, 0.2];

        assert_eq!(result.coords, expected);
    }

    #[test]
    fn test_srgb_to_oklch_conversion() {
        let src = "#ffff00";
        let rgb_color = parse_rgbhex(src).unwrap();
        let oklch_color = rgb_color.convert_to(&OutputFormat::Oklch);
        let CliColor::OKLCH(result) = oklch_color else {
            panic!("Expected OKLCH color from conversion");
        };
        let expected = [0.968, 0.211, 109.769];
        for (a, b) in result.coords.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-3);
        }
    }

    #[test]
    fn test_gamutmap_minde() {
        let src = "oklch(0.99 0.8 110)";
        let oklch_color = parse_color_string(src).unwrap();
        let gma_color = oklch_color.gamutmap_minde();
        let CliColor::SRGB(result) = gma_color else {
            panic!("Expected sRGB color from gamutmapping");
        };
        let expected = [1., 1., 0.339];
        for (a, b) in result.coords.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-3);
        }
    }

    #[test]
    fn test_gamutmap_clip() {
        let src = "oklch(0.99 0.8 110)";
        let oklch_color = parse_color_string(src).unwrap();
        let clipped_color = oklch_color.clip();
        let CliColor::SRGB(result) = clipped_color else {
            panic!("Expected sRGB color from clip");
        };
        let expected = [1., 0.976, 0.];
        for (a, b) in result.coords.iter().zip(expected.iter()) {
            assert!((a - b).abs() < 1e-3);
        }
    }

    #[test]
    fn test_output_format() {
        let src = "oklch(0.5 0.5 110)";
        let color = parse_color_string(src).unwrap();
        let output_string = format!("{color}");
        assert_eq!(output_string, src);
    }
}
