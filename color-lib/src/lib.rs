// Copyright © 2026 John P. ARNZEN
// SPDX-License-Identifier: MIT

mod gamutmap;
mod matrix;
mod spaces;

// Exported items. NOTE These are importable as "rooted" at the crate root,
// and not by module path.
pub use crate::spaces::convert::{
    BoundedColorSpace, Color, ColorConversion, OKLAB, OKLCH, OKLrAB, OKLrCH, RGBSpace, SRGB,
    SRGBLinear, XYZD65,
};
pub use matrix::Vector3;
