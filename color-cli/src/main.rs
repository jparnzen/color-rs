// Copyright © 2026 John P. ARNZEN
// SPDX-License-Identifier: MIT

use anyhow::Result;
use clap::{Arg, command};
use color_cli::{CliColor, OutputFormat, parse_color_function, parse_rgbhex};
use color_lib::{BoundedColorSpace, SRGB};

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
        parse_color_function(color_string)?
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
