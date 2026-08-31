// Copyright © 2026 John P. ARNZEN
// SPDX-License-Identifier: MIT

mod lexparse;

use anyhow::Result;
use clap::{Arg, command};
use color_lib::{BoundedColorSpace, Color, OKLAB, OKLCH, OKLrAB, OKLrCH, SRGB, XYZD65};
use lexparse::{parse_color_string, parse_rgbhex};
use std::fmt::Display;

// Wrapper type for the type-safe Color<> structs
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone)]
enum CliColor {
    SRGB(Color<SRGB>),
    OKLCH(Color<OKLCH>),
    OKLAB(Color<OKLAB>),
    OKLrCH(Color<OKLrCH>),
    OKLrAB(Color<OKLrAB>),
}

impl CliColor {
    fn to_output_format(&self) -> OutputFormat {
        match self {
            Self::SRGB(_) => OutputFormat::Rgb,
            Self::OKLAB(_) => OutputFormat::Oklab,
            Self::OKLCH(_) => OutputFormat::Oklch,
            Self::OKLrAB(_) => OutputFormat::Oklrab,
            Self::OKLrCH(_) => OutputFormat::Oklrch,
        }
    }

    fn convert_to(&self, dest_format: &OutputFormat) -> CliColor {
        // Prevent doing double-conversions to the same output/dest format.
        // Double-conversion (e.g. sRGB -> XYZ -> sRGB) is a lossy operation,
        // imparting noise especially for expected zero values.
        // This has caused in-gamut colors to report as out of gamut.
        if self.to_output_format() == *dest_format {
            return self.clone();
        }

        let xyz = self.to_XYZ();
        match dest_format {
            OutputFormat::Rgb => Self::SRGB(xyz.convert_to()),
            OutputFormat::Oklab => Self::OKLAB(xyz.convert_to()),
            OutputFormat::Oklch => Self::OKLCH(xyz.convert_to()),
            OutputFormat::Oklrab => Self::OKLrAB(xyz.convert_to()),
            OutputFormat::Oklrch => Self::OKLrCH(xyz.convert_to()),
        }
    }

    #[allow(non_snake_case)]
    fn to_XYZ(&self) -> Color<XYZD65> {
        match self {
            Self::SRGB(c) => c.convert_to(),
            Self::OKLAB(c) => c.convert_to(),
            Self::OKLCH(c) => c.convert_to(),
            Self::OKLrAB(c) => c.convert_to(),
            Self::OKLrCH(c) => c.convert_to(),
        }
    }

    fn gamutmap_minde(&self) -> CliColor {
        match self {
            Self::SRGB(c) => Self::SRGB(c.gamutmap_local_minde()),
            Self::OKLAB(c) => Self::SRGB(c.gamutmap_local_minde()),
            Self::OKLCH(c) => Self::SRGB(c.gamutmap_local_minde()),
            Self::OKLrAB(c) => Self::SRGB(c.gamutmap_local_minde()),
            Self::OKLrCH(c) => Self::SRGB(c.gamutmap_local_minde()),
        }
    }

    fn gamutmap_raytrace(&self) -> CliColor {
        match self {
            Self::SRGB(c) => Self::SRGB(c.gamutmap_raytrace()),
            Self::OKLAB(c) => Self::SRGB(c.gamutmap_raytrace()),
            Self::OKLCH(c) => Self::SRGB(c.gamutmap_raytrace()),
            Self::OKLrAB(c) => Self::SRGB(c.gamutmap_raytrace()),
            Self::OKLrCH(c) => Self::SRGB(c.gamutmap_raytrace()),
        }
    }

    fn clip(&self) -> CliColor {
        match self {
            Self::SRGB(c) => Self::SRGB(c.clip()),
            Self::OKLAB(c) => Self::SRGB(c.clip()),
            Self::OKLCH(c) => Self::SRGB(c.clip()),
            Self::OKLrAB(c) => Self::SRGB(c.clip()),
            Self::OKLrCH(c) => Self::SRGB(c.clip()),
        }
    }
}

impl Display for CliColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SIG_DIG: i32 = 5;
        match self {
            Self::SRGB(color) => {
                let [r, g, b] = color.coords.map(|c| round_to_significant(c, SIG_DIG));
                // NOTE that casting to i32 will fix any -0 situations from round()
                let [r255, g255, b255] = color.coords.map(|c| (c * 255.).round() as i32);

                write!(
                    f,
                    "color(srgb {r} {g} {b})  \u{2502}  rgb({r255} {g255} {b255})  \u{2502}  #{:02x}{:02x}{:02x}",
                    r255.clamp(0, 255) as u8,
                    g255.clamp(0, 255) as u8,
                    b255.clamp(0, 255) as u8
                )
            }
            Self::OKLAB(color) => {
                let [l, a, b] = color.coords.map(|v| round_to_significant(v, SIG_DIG));
                write!(f, "oklab({l} {a} {b})")
            }
            Self::OKLCH(color) => {
                let [l, c, h] = color.coords.map(|v| round_to_significant(v, SIG_DIG));
                write!(f, "oklch({l} {c} {h})")
            }
            Self::OKLrAB(color) => {
                let [lr, a, b] = color.coords.map(|v| round_to_significant(v, SIG_DIG));
                write!(f, "color(--oklrab {lr} {a} {b})")
            }
            Self::OKLrCH(color) => {
                let [lr, c, h] = color.coords.map(|v| round_to_significant(v, SIG_DIG));
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

    let gma_color = match matches.get_one::<GamutMapAlgo>("gma") {
        Some(GamutMapAlgo::Clip) => dest_color.clip(),
        Some(GamutMapAlgo::Minde) => dest_color.gamutmap_minde(),
        Some(GamutMapAlgo::Raytrace) => dest_color.gamutmap_raytrace(),
        None => dest_color,
    };

    let output_color = gma_color.convert_to(dest_format);

    if let CliColor::SRGB(color) = &output_color
        && !SRGB::in_gamut(color)
    {
        eprintln!(
            ">>> WARNING <<< The color is outside of sRGB gamut limits => hex values will be clipped."
        );
    }

    println!("{output_color}");

    Ok(())
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
    fn test_parse_rgb_values_success() {
        let src = "rgb(100% 50% 20%)";
        let result = parse_color_string(src).expect("color string should be parseable");
        let CliColor::SRGB(result) = result else {
            panic!("Expected SRGB color from parse_color_string()");
        };
        let expected = [1.0, 0.5, 0.2];

        assert_eq!(result.coords, expected)
    }

    #[test]
    fn test_srgb_to_oklch_conversion() {
        let src = "#ffff00";
        let rgb_color = parse_rgbhex(src).expect("color string should be parseable");
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
        let oklch_color = parse_color_string(src).expect("color string should be parseable");
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
        let oklch_color = parse_color_string(src).expect("color string should be parseable");
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
        let color = parse_color_string(src).expect("color string should be parseable");
        let output_string = format!("{color}");
        assert_eq!(output_string, src);
    }
}
