// Copyright © 2026 John P. ARNZEN
// SPDX-License-Identifier: MIT

use wasm_bindgen::prelude::*;

use color_lib::{Color, OKLCH, SRGB, Vector3};

#[wasm_bindgen]
pub fn srgb_to_oklch(r: f32, g: f32, b: f32) -> Vec<f32> {
    let rgb: Vector3 = [f64::from(r), f64::from(g), f64::from(b)];

    let [l, c, h] = Color::<SRGB>::new(rgb).convert_to::<OKLCH>().coords;

    vec![l as f32, c as f32, h as f32]
}

#[wasm_bindgen]
pub fn oklch_to_srgb(l: f32, c: f32, h: f32) -> Vec<f32> {
    let lch: Vector3 = [f64::from(l), f64::from(c), f64::from(h)];

    let [r, g, b] = Color::<OKLCH>::new(lch).convert_to::<SRGB>().coords;

    vec![r as f32, g as f32, b as f32]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        // f32 precision is around 7 decimal places;
        // f64 epsilon in rust docs is 1e-10, so we'll use 1e-5 assuming "half" precision
        const EPSILON: f32 = 1e-5;

        let rgb = [1., 1., 0.];

        let lch = srgb_to_oklch(rgb[0], rgb[1], rgb[2]);
        let back = oklch_to_srgb(lch[0], lch[1], lch[2]);

        for (a, b) in rgb.iter().zip(back.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }
}
