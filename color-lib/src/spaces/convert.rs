// Copyright © 2026 John P. ARNZEN
// SPDX-License-Identifier: MIT

use crate::matrix::{Matrix3, Vector3, matrix3_mult_vector3};

use std::marker::PhantomData;

/// Unit/marker structs for each supported color space.
pub struct SRGB;
pub struct SRGBLinear;
pub struct XYZD65;
pub struct OKLAB;
pub struct OKLCH;
pub struct OKLrAB;
pub struct OKLrCH;

/// A more type-safe Color representation, using phantom types.
#[derive(PartialEq)]
pub struct Color<S> {
    pub coords: Vector3,
    _space: PhantomData<S>,
}

impl<S> Color<S> {
    pub fn new(coords: Vector3) -> Self {
        Self {
            coords,
            _space: PhantomData,
        }
    }
}

// Manual implementation to avoid over-restrictive #[derive] additions.
impl<S> Clone for Color<S> {
    fn clone(&self) -> Self {
        Self {
            coords: self.coords,
            _space: PhantomData,
        }
    }
}

// Manual implementation to avoid over-restrictive #[derive] additions.
impl<S> std::fmt::Debug for Color<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Color")
            .field("coords", &self.coords)
            .finish()
    }
}

/// Trait for base transformations to and from XYZ color space.
#[allow(non_snake_case)]
pub trait ColorConversion: Sized {
    fn to_XYZ(color: &Color<Self>) -> Color<XYZD65>;
    fn from_XYZ(color: &Color<XYZD65>) -> Color<Self>;
}

/// Blanket implementation for converting from S to Dest color spaces.
impl<S: ColorConversion> Color<S> {
    pub fn convert_to<Dest: ColorConversion>(&self) -> Color<Dest> {
        Dest::from_XYZ(&S::to_XYZ(self))
    }
}

/// Defines gamut bounds for bounded color spaces (i.e. RGB spaces), with default in-gamut checking,
/// and clipping to gamut bounds.
pub trait BoundedColorSpace: ColorConversion {
    const BOUNDS: [(f64, f64); 3];

    fn in_gamut(color: &Color<Self>) -> bool {
        color
            .coords
            .iter()
            .zip(Self::BOUNDS.iter())
            .all(|(c, &(lo, hi))| *c >= lo && *c <= hi)
    }

    fn clip(color: &Color<Self>) -> Color<Self> {
        let coords =
            std::array::from_fn(|i| color.coords[i].clamp(Self::BOUNDS[i].0, Self::BOUNDS[i].1));
        Color::new(coords)
    }
}

impl BoundedColorSpace for SRGB {
    const BOUNDS: [(f64, f64); 3] = [(0., 1.); 3];
}

impl BoundedColorSpace for SRGBLinear {
    const BOUNDS: [(f64, f64); 3] = [(0., 1.); 3];
}

/// Trait for gamma-encoded RGB spaces, such as sRGB, Display P3, and Rec2020.
/// Linear RGB spaces DO NOT implement this trait.
pub trait RGBSpace: Sized {
    type Linear: BoundedColorSpace;

    const LINEAR_TO_XYZ: Matrix3;
    const XYZ_TO_LINEAR: Matrix3;

    fn linearize_channel(c: f64) -> f64;
    fn gamma_encode_channel(c: f64) -> f64;

    // default implementations of transfer functions
    fn linearize(color: &Color<Self>) -> Color<Self::Linear> {
        Color::new(color.coords.map(Self::linearize_channel))
    }
    fn gamma_encode(color: &Color<Self::Linear>) -> Color<Self> {
        Color::new(color.coords.map(Self::gamma_encode_channel))
    }
}

// Blanket implementation for all current and future RGB spaces
impl<S: RGBSpace> ColorConversion for S {
    fn to_XYZ(color: &Color<Self>) -> Color<XYZD65> {
        let linear = S::linearize(color);
        Color::new(matrix3_mult_vector3(&S::LINEAR_TO_XYZ, &linear.coords))
    }

    fn from_XYZ(color: &Color<XYZD65>) -> Color<Self> {
        let linear = Color::new(matrix3_mult_vector3(&S::XYZ_TO_LINEAR, &color.coords));
        S::gamma_encode(&linear)
    }
}

impl RGBSpace for SRGB {
    type Linear = SRGBLinear;

    /// Translated from CSS Color 4 sample javascript code.
    /// <https://drafts.csswg.org/css-color-4/#color-conversion-code>
    #[rustfmt::skip]
    const LINEAR_TO_XYZ: Matrix3 = [
        [506752. / 1228815.,  87881. / 245763.,   12673. /   70218.],
        [ 87098. /  409605., 175762. / 245763.,   12673. /  175545.],
        [  7918. /  409605.,  87881. / 737289., 1001167. / 1053270.],
    ];
    #[rustfmt::skip]
    const XYZ_TO_LINEAR: Matrix3 = [
        [  12831. /   3959.,    -329. /    214., -1974. /   3959.],
        [-851781. / 878810., 1648619. / 878810., 36519. / 878810.],
        [    705. /  12673.,   -2585. /  12673.,   705. /    667.],
    ];

    /// Remove sRGB gamma-encoding from a color component (r, g, or b channel of sRGB).
    /// Translated from CSS Color 4 sample javascript code.
    /// <https://drafts.csswg.org/css-color-4/#color-conversion-code>
    fn linearize_channel(c: f64) -> f64 {
        let abs_c = c.abs();

        if abs_c <= 0.04045 {
            c / 12.92
        } else {
            ((abs_c + 0.055) / 1.055).powf(2.4).copysign(c)
        }
    }

    /// Gamma-encode a linear sRGB color component (r, g, or b channel of sRGB).
    /// Translated from CSS Color 4 sample javascript code.
    /// <https://drafts.csswg.org/css-color-4/#color-conversion-code>
    fn gamma_encode_channel(c: f64) -> f64 {
        let abs_c = c.abs();

        if abs_c <= 0.0031308 {
            c * 12.92
        } else {
            (1.055 * abs_c.powf(1.0 / 2.4) - 0.055).copysign(c)
        }
    }
}

impl ColorConversion for SRGBLinear {
    fn to_XYZ(color: &Color<Self>) -> Color<XYZD65> {
        Color::new(matrix3_mult_vector3(&SRGB::LINEAR_TO_XYZ, &color.coords))
    }

    fn from_XYZ(color: &Color<XYZD65>) -> Color<Self> {
        Color::new(matrix3_mult_vector3(&SRGB::XYZ_TO_LINEAR, &color.coords))
    }
}

impl ColorConversion for XYZD65 {
    fn to_XYZ(color: &Color<Self>) -> Color<XYZD65> {
        color.clone()
    }

    fn from_XYZ(color: &Color<XYZD65>) -> Color<Self> {
        color.clone()
    }
}

/// Translated from CSS Color 4 sample javascript code.
/// <https://drafts.csswg.org/css-color-4/#color-conversion-code>
#[allow(clippy::excessive_precision)]
impl OKLAB {
    #[rustfmt::skip]
    const OKLAB_TO_LMS: Matrix3 = [
        [1.0000000000000000,  0.3963377773761749,  0.2158037573099136],
        [1.0000000000000000, -0.1055613458156586, -0.0638541728258133],
        [1.0000000000000000, -0.0894841775298119, -1.2914855480194092],
    ];
    #[rustfmt::skip]
    const LMS_TO_XYZD65: Matrix3 = [
        [ 1.2268798758459243, -0.5578149944602171,  0.2813910456659647],
        [-0.0405757452148008,  1.1122868032803170, -0.0717110580655164],
        [-0.0763729366746601, -0.4214933324022432,  1.5869240198367816],
    ];
    #[rustfmt::skip]
    const XYZD65_TO_LMS: Matrix3 = [
        [0.8190224379967030, 0.3619062600528904, -0.1288737815209879],
        [0.0329836539323885, 0.9292868615863434,  0.0361446663506424],
        [0.0481771893596242, 0.2642395317527308,  0.6335478284694309],
    ];
    #[rustfmt::skip]
    const LMS_TO_OKLAB: Matrix3 = [
        [0.2104542683093140,  0.7936177747023054, -0.0040720430116193],
        [1.9779985324311684, -2.4285922420485799,  0.4505937096174110],
        [0.0259040424655478,  0.7827717124575296, -0.8086757549230774],
    ];
}

impl ColorConversion for OKLAB {
    fn to_XYZ(color: &Color<Self>) -> Color<XYZD65> {
        let lms = matrix3_mult_vector3(&OKLAB::OKLAB_TO_LMS, &color.coords);
        Color::new(matrix3_mult_vector3(
            &OKLAB::LMS_TO_XYZD65,
            &lms.map(|c| c.powi(3)),
        ))
    }

    fn from_XYZ(color: &Color<XYZD65>) -> Color<Self> {
        let lms = matrix3_mult_vector3(&OKLAB::XYZD65_TO_LMS, &color.coords);
        Color::new(matrix3_mult_vector3(
            &OKLAB::LMS_TO_OKLAB,
            &lms.map(|c| c.cbrt()),
        ))
    }
}

/// Transform OKLCH to and from OKLAB for further transformations.
/// Translated from CSS Color 4 sample javascript code.
/// <https://drafts.csswg.org/css-color-4/#color-conversion-code>
#[allow(non_snake_case)]
impl OKLCH {
    fn to_OKLAB(oklch: &Color<Self>) -> Color<OKLAB> {
        const EPSILON: f64 = 0.000_004;

        let [l, c, h] = oklch.coords;
        if c <= EPSILON || h.is_nan() {
            return Color::new([l, 0., 0.]);
        }

        let h_prime = h.to_radians();
        Color::new([l, c * h_prime.cos(), c * h_prime.sin()])
    }

    fn from_OKLAB(oklab: &Color<OKLAB>) -> Color<Self> {
        const EPSILON: f64 = 0.000_004;

        let [l, a, b] = oklab.coords;
        let chroma = (a.powi(2) + b.powi(2)).sqrt();
        let hue = if chroma <= EPSILON {
            f64::NAN
        } else {
            b.atan2(a).to_degrees().rem_euclid(360.)
        };

        Color::new([l, chroma, hue])
    }
}

impl ColorConversion for OKLCH {
    fn to_XYZ(color: &Color<Self>) -> Color<XYZD65> {
        let temp = OKLCH::to_OKLAB(color);
        OKLAB::to_XYZ(&temp)
    }

    fn from_XYZ(color: &Color<XYZD65>) -> Color<Self> {
        let temp = OKLAB::from_XYZ(color);
        OKLCH::from_OKLAB(&temp)
    }
}

impl ColorConversion for OKLrAB {
    fn to_XYZ(color: &Color<Self>) -> Color<XYZD65> {
        let temp = Color::new(oklr_to_okl(&color.coords));
        OKLAB::to_XYZ(&temp)
    }

    fn from_XYZ(color: &Color<XYZD65>) -> Color<Self> {
        let temp = OKLAB::from_XYZ(color);
        Color::new(okl_to_oklr(&temp.coords))
    }
}

impl ColorConversion for OKLrCH {
    fn to_XYZ(color: &Color<Self>) -> Color<XYZD65> {
        let temp = Color::new(oklr_to_okl(&color.coords));
        OKLCH::to_XYZ(&temp)
    }

    fn from_XYZ(color: &Color<XYZD65>) -> Color<Self> {
        let temp = OKLCH::from_XYZ(color);
        Color::new(okl_to_oklr(&temp.coords))
    }
}

/// Transform an OKL* color point to an OKLr* color point.
fn okl_to_oklr(lxy: &Vector3) -> Vector3 {
    let [l, x, y] = lxy;
    [toe(*l), *x, *y]
}

/// Transform an OKLr* color point to an OKL* color point.
fn oklr_to_okl(lrxy: &Vector3) -> Vector3 {
    let [l, x, y] = lrxy;
    [inv_toe(*l), *x, *y]
}

/// OKLr* lightness toe function for OKL* -> OKLr*.
/// Translated from C++ source at <https://bottosson.github.io/posts/colorpicker/#common-code>
fn toe(x: f64) -> f64 {
    const K1: f64 = 0.206;
    const K2: f64 = 0.03;
    const K3: f64 = (1. + K1) / (1. + K2);

    0.5 * (K3 * x - K1 + ((K3 * x - K1).powi(2) + 4. * K2 * K3 * x).sqrt())
}

/// Inverse lightness toe function for OKLr* -> OKL*.
/// Translated from C++ source at <https://bottosson.github.io/posts/colorpicker/#common-code>
fn inv_toe(x: f64) -> f64 {
    const K1: f64 = 0.206;
    const K2: f64 = 0.03;
    const K3: f64 = (1. + K1) / (1. + K2);

    (x * x + K1 * x) / (K3 * (x + K2))
}

#[cfg(test)]
/// Truncate fp values to a specified precision.
pub(crate) fn set_precision(precision_digits: usize, vector: Vector3) -> Vector3 {
    vector.map(|c| {
        format!("{:.1$}", c, precision_digits)
            .parse::<f64>()
            .unwrap()
    })
}

///////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_roundtrip() {
        let src = Color::<SRGB>::new([0.0, 0.533, 0.776]);
        let result = src.convert_to::<SRGB>();

        // f64 values have ~10 digits of precision (c.f. f64 doc examples)
        const EPSILON: f64 = 1e-10;
        for (a, b) in src.coords.iter().zip(result.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_linear_srgb_roundtrip() {
        let src = Color::<SRGBLinear>::new([1., 0.5, 0.2]);
        let result = src.convert_to::<SRGBLinear>();

        // f64 values have ~10 digits of precision (c.f. f64 doc examples)
        const EPSILON: f64 = 1e-10;
        for (a, b) in src.coords.iter().zip(result.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_oklab_roundtrip() {
        let src = Color::<OKLAB>::new([0.5, -0.2, 0.1]);
        let result = src.convert_to::<OKLAB>();

        // f64 values have ~10 digits of precision (c.f. f64 doc examples)
        const EPSILON: f64 = 1e-10;
        for (a, b) in src.coords.iter().zip(result.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_oklch_roundtrip() {
        let src = Color::<OKLCH>::new([0.968, 0.211, 109.77]);
        let result = src.convert_to::<OKLCH>();

        // f64 values have ~10 digits of precision (c.f. f64 doc examples)
        const EPSILON: f64 = 1e-10;
        for (a, b) in src.coords.iter().zip(result.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_xyz_roundtrip() {
        let src = Color::<XYZD65>::new([0.2, 0.3, 0.4]);
        let result = src.convert_to::<XYZD65>();

        // f64 values have ~10 digits of precision (c.f. f64 doc examples)
        const EPSILON: f64 = 1e-10;
        for (a, b) in src.coords.iter().zip(result.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_srgb_to_oklch() {
        // color yellow in OKLCH and sRGB
        let src = Color::<SRGB>::new([1., 1., 0.]);
        let result = src.convert_to::<OKLCH>();
        // make sure result coords are the same precision as the max precision of the expected coords
        let result_coords = set_precision(3, result.coords);

        let expected_oklch = [0.968, 0.211, 109.77];
        // set epsilon to be max precision of the expected coords
        const EPSILON: f64 = 1e-3;
        for (a, b) in expected_oklch.iter().zip(result_coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_set_precision() {
        let src = [1.2, 2.3451, 3.456789];
        const PRECISION: usize = 3;
        let expected = [1.200, 2.345, 3.457];

        let result = set_precision(PRECISION, src);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_toe_roundtrip() {
        const EPSILON: f64 = 1e-10;

        let original: f64 = 0.5;
        let forward = toe(original);
        let back = inv_toe(forward);

        assert!((original - back).abs() < EPSILON);
    }

    #[test]
    fn test_oklch_oklab_roundtrip() {
        const EPSILON: f64 = 1e-10;

        let original_coords = [0.5, 0.4, 29.2];
        let src = Color::<OKLCH>::new(original_coords);
        let forward = src.convert_to::<OKLAB>();
        let back = forward.convert_to::<OKLCH>();

        for (a, b) in original_coords.iter().zip(back.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_oklrch_oklch_roundtrip() {
        const EPSILON: f64 = 1e-10;

        let original_coords = [0.05, 0.4, 29.2];
        let src = Color::<OKLrCH>::new(original_coords);
        let forward = src.convert_to::<OKLCH>();
        let back = forward.convert_to::<OKLrCH>();

        for (a, b) in original_coords.iter().zip(back.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_oklch_oklrch_roundtrip() {
        // NOTE H value passes through atan2/trig functions twice in roundtrip;
        // accumulated transcendental errors for some L/C values exceeds 1e-10,
        // so loosening to 1e-9 to accommodate errors at the e-10/e-9 boundary.
        const EPSILON: f64 = 1e-9;

        let original_coords = [0.05, 0.4, 29.2];
        let src = Color::<OKLCH>::new(original_coords);
        let forward = src.convert_to::<OKLrCH>();
        let back = forward.convert_to::<OKLCH>();

        for (a, b) in original_coords.iter().zip(back.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_oklab_oklrab_roundtrip() {
        const EPSILON: f64 = 1e-10;

        let original_coords = [0.05, 0.4, -0.4];
        let src = Color::<OKLAB>::new(original_coords);
        let forward = src.convert_to::<OKLrAB>();
        let back = forward.convert_to::<OKLAB>();

        for (a, b) in original_coords.iter().zip(back.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_srgb_linear_srgb_roundtrip() {
        const EPSILON: f64 = 1e-10;

        // #0088c6 — verified against colorjs.io
        let original_coords = [0.0, 0.533, 0.776];
        let src = Color::<SRGB>::new(original_coords);
        let forward = src.convert_to::<SRGBLinear>();
        let back = forward.convert_to::<SRGB>();

        for (a, b) in original_coords.iter().zip(back.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_srgb_xyz_roundtrip() {
        const EPSILON: f64 = 1e-10;

        // #0088c6 — verified against colorjs.io
        let original_coords = [0.0, 0.533, 0.776];
        let src = Color::<SRGB>::new(original_coords);
        let forward = src.convert_to::<XYZD65>();
        let back = forward.convert_to::<SRGB>();

        for (a, b) in original_coords.iter().zip(back.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_linear_srgb_xyz_roundtrip() {
        const EPSILON: f64 = 1e-10;

        // #0088c6 — verified against colorjs.io
        let original_coords = [0.0, 0.533, 0.776];
        let src = Color::<SRGB>::new(original_coords);
        let linear = src.convert_to::<SRGBLinear>();
        let forward = linear.convert_to::<XYZD65>();
        let back = forward.convert_to::<SRGBLinear>();

        for (a, b) in linear.coords.iter().zip(back.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_oklab_xyz_roundtrip() {
        const EPSILON: f64 = 1e-10;

        let original_coords = [0.5, 0.2, -0.05];
        let src = Color::<OKLAB>::new(original_coords);
        let forward = src.convert_to::<XYZD65>();
        let back = forward.convert_to::<OKLAB>();

        for (a, b) in original_coords.iter().zip(back.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_achromatic_nan_roundtrip() {
        const EPSILON: f64 = 1e-10;

        let original_coords = [0.5; 3];
        let src = Color::<SRGB>::new(original_coords);
        let forward = src.convert_to::<OKLCH>();
        let back = forward.convert_to::<SRGB>();

        for (a, b) in original_coords.iter().zip(back.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }

    #[test]
    fn test_linear_gamma_encode_roundtrip() {
        const EPSILON: f64 = 1e-10;

        // #0088c6 — verified against colorjs.io
        let original_coords = [0.0, 0.533, 0.776];
        let src = Color::<SRGB>::new(original_coords);
        let forward = SRGB::linearize(&src);
        let back = SRGB::gamma_encode(&forward);

        for (a, b) in original_coords.iter().zip(back.coords.iter()) {
            assert!((a - b).abs() < EPSILON);
        }
    }
}
