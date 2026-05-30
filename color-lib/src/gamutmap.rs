/***
MIT License

Copyright (c) 2026, John P. ARNZEN

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

use crate::{matrix::Vector3, spaces::convert::*};

impl<S: ColorConversion> Color<S> {
    /// Returns true if, when passed a color, that color is inside the gamut of destination.
    // For HSL and HWB, it returns true if the color is inside the gamut of sRGB.
    // NOTE HSL and HWB are currently unimplemented, but I plan to include them in the future.
    pub fn in_gamut<Dest: BoundedColorSpace>(&self) -> bool {
        Dest::in_gamut(&self.convert_to::<Dest>())
    }

    /// Converts color to destination, clamps each component to the bounds of the reference range
    /// for that component and returns the result.
    pub fn clip<Dest: BoundedColorSpace>(&self) -> Color<Dest> {
        Dest::clip(&self.convert_to::<Dest>())
    }

    /// Gamut mapping by binary search using local MINDE
    /// <https://drafts.csswg.org/css-color-4/#pseudo-binsearch>
    pub fn gamutmap_local_minde<Dest: BoundedColorSpace>(&self) -> Color<Dest> {
        const MIN_L: f64 = 0.;
        const MAX_L: f64 = 1.;

        // 1. if destination has no gamut limits (XYZ-D65, XYZ-D50, Lab, LCH, Oklab, OkLCh) convert origin to destination and return it as the gamut mapped color
        // NOTE I'm only mapping to bounded/RGB display spaces, at least for now, so I'm removing this test.

        // 2. let origin_OkLCh be origin converted from origin color space to the OkLCh color space
        let origin_oklch = self.convert_to::<OKLCH>();

        // 3. if the Lightness of origin_OkLCh is greater than or equal to 100%, convert `oklab(1 0 0 / origin.alpha)` to destination and return it as the gamut mapped color
        if origin_oklch.coords[0] >= MAX_L {
            return Color::<OKLAB>::new([1., 0., 0.]).convert_to::<Dest>();
        }

        // 4. if the Lightness of origin_OkLCh is less than than or equal to 0%, convert `oklab(0 0 0 / origin.alpha)` to destination and return it as the gamut mapped color
        if origin_oklch.coords[0] <= MIN_L {
            return Color::<OKLAB>::new([0.; 3]).convert_to::<Dest>();
        }

        // 5. let inGamut(color) be a function which returns true if, when passed a color, that color is inside the gamut of destination. For HSL and HWB, it returns true if the color is inside the gamut of sRGB.
        // 6. if inGamut(origin_OkLCh) is true, convert origin_OkLCh to destination and return it as the gamut mapped color
        if origin_oklch.in_gamut::<Dest>() {
            return origin_oklch.convert_to::<Dest>();
        }

        // 7. otherwise, let delta(one, two) be a function which returns the deltaEOK of color one compared to color two
        // 8. let JND be 0.02
        const JND: f64 = 0.02;

        // 9. let epsilon be 0.0001
        const EPSILON: f64 = 0.0001;

        // 10. let clip(color) be a function which converts color to destination, clamps each component to the bounds of the reference range for that component and returns the result
        // 11. set current to origin_OkLCh
        let mut current = origin_oklch.clone();

        // 12. set clipped to clip(current)
        let mut clipped = current.clip::<Dest>();

        // 13. set E to delta(clipped, current)
        let e = delta_eok(&clipped, &current);

        // 14. if E < JND
        //     14.1. return clipped as the gamut mapped color
        if e < JND {
            return clipped;
        }

        // 15. set min to zero
        let mut min = 0.;

        // 16. set max to the OkLCh chroma of origin_OkLCh
        let mut max = origin_oklch.coords[1];

        // 17. let min_inGamut be a boolean that represents when min is still in gamut, and set it to true
        let mut min_in_gamut = true;

        // 18. while (max - min is greater than epsilon) repeat the following steps
        while (max - min) > EPSILON {
            // 18.1. set chroma to (min + max) /2
            let chroma = (min + max) / 2.;

            // 18.2. set the chroma component of current to chroma
            current.coords[1] = chroma;

            // 18.3. if min_inGamut is true and also if inGamut(current) is true, set min to chroma and continue to repeat these steps
            if min_in_gamut && current.in_gamut::<Dest>() {
                min = chroma;
                continue;
            }

            // 18.4. otherwise, carry out these steps:
            // 18.4.1. set clipped to clip(current)
            clipped = current.clip::<Dest>();

            // 18.4.2. set E to delta(clipped, current)
            let e = delta_eok(&clipped, &current);

            // 18.4.3. if E < JND
            if e < JND {
                // 18.4.3.1. if (JND - E < epsilon) return clipped as the gamut mapped color
                if (JND - e) < EPSILON {
                    return clipped;
                }

                // 18.4.3.2. otherwise,
                // 18.4.3.2.1. set min_inGamut to false
                min_in_gamut = false;

                // 18.4.3.2.2. set min to chroma
                min = chroma;
            }
            // 18.4.4. otherwise, set max to chroma and continue to repeat these steps
            else {
                max = chroma;
            }
        }

        // 19. return clipped as the gamut mapped color
        clipped
    }

    /// Gamut mapping using raytracing
    /// <https://drafts.csswg.org/css-color-4/#pseudo-raytrace>
    pub fn gamutmap_raytrace<Dest: BoundedColorSpace + RGBSpace>(&self) -> Color<Dest> {
        const MIN_L: f64 = 0.;
        const MAX_L: f64 = 1.;

        // 1. if destination has no gamut limits (XYZ-D65, XYZ-D50, Lab, LCH, Oklab, OkLCh) convert origin to destination and return it as the gamut mapped color
        // NOTE I'm only mapping to bounded/RGB display spaces, at least for now, so I'm removing this test.

        // 2. let origin_OkLCh be origin converted from origin color space to the OkLCh color space
        let origin_oklch = self.convert_to::<OKLCH>();

        // 3. if the Lightness of origin_OkLCh is greater than or equal to 100%, convert `oklab(1 0 0 / origin.alpha)` to destination and return it as the gamut mapped color
        if origin_oklch.coords[0] >= MAX_L {
            return Color::<OKLAB>::new([1., 0., 0.]).convert_to();
        }

        // 4. if the Lightness of origin_OkLCh is less than than or equal to 0%, convert `oklab(0 0 0 / origin.alpha)` to destination and return it as the gamut mapped color
        if origin_oklch.coords[0] <= MIN_L {
            return Color::<OKLAB>::new([0.; 3]).convert_to();
        }

        // 5. let l_origin be the OkLCh lightness component of origin_OkLCh
        // 6. let h_origin be the OkLCh hue component of origin_OkLCh
        let [l_origin, _, h_origin] = origin_oklch.coords;

        // 7. let anchor be an achromatic OkLCh color formed with l_origin as lightness, 0 as chroma and h_origin as hue, converted to the linear-light form of destination
        let mut anchor =
            Dest::linearize(&Color::<OKLCH>::new([l_origin, 0., h_origin]).convert_to::<Dest>());

        // 8. let origin_rgb be origin_OkLCh converted to the linear-light form of destination
        let mut origin_rgb = Dest::linearize(&origin_oklch.convert_to::<Dest>());

        // 9. if origin_rgb is not in gamut
        if !origin_rgb.in_gamut::<Dest::Linear>() {
            // 9.1. let low be 0.0 + 1E-6 1
            const LOW: f64 = 1e-6;

            // 9.2. let high be 1.0 - 1E-6 2
            const HIGH: f64 = 1. - LOW;

            // 9.3. let last be origin_rgb
            let mut last = origin_rgb.clone();

            // 9.4. for (i=0; i<4; i++)
            for i in 0..4 {
                // 9.4.1. if (i > 0)
                if i > 0 {
                    // 9.4.1.1. let current_OkLCh be origin_rgb converted to OkLCh
                    let mut current_oklch = origin_rgb.convert_to::<OKLCH>();

                    // 9.4.1.2. let the lightness of current_OkLCh be l_origin
                    current_oklch.coords[0] = l_origin;

                    // 9.4.1.3. let the hue of current_OkLCh be h_origin 3
                    current_oklch.coords[2] = h_origin;

                    // 9.4.1.4. let origin_rgb be current_OkLCh converted to the linear-light form of destination
                    origin_rgb = Dest::linearize(&current_oklch.convert_to::<Dest>());
                }

                // 9.4.2. Cast a ray from start = anchor to end = origin_rgb and let intersection be the intersection of this ray with the gamut boundary
                // 9.4.3. if an intersection was not found, let origin_rgb be last and exit the loop 5
                let Some(intersection) = cast_ray(&anchor.coords, &origin_rgb.coords) else {
                    origin_rgb = last;
                    break;
                };

                // 9.4.4. if (i >0) AND (each component of origin_rgb is between low and high) then let anchor be origin_rgb ^4
                if i > 0 && origin_rgb.coords.iter().all(|c| (LOW..=HIGH).contains(c)) {
                    anchor = origin_rgb.clone();
                }

                // 9.4.5. let origin_rgb be intersection
                origin_rgb.coords = intersection;

                // 9.4.6. let last be intersection
                last.coords = intersection;
            }
        }

        // 10. let clip(color) be a function which converts color to destination, clamps each component to the bounds of the reference range for that component and returns the result
        // 11. set clipped to clip(current)
        // 12. return clipped as the gamut mapped color
        // NOTE there is a typo in the CSS Color 4 working draft using `current`, which doesn't exist.
        // It should be `origin_rgb` instead of `current`.
        // Reported to the editorial staff on github. See <https://github.com/w3c/csswg-drafts/issues/10579#issuecomment-4122677476>.
        origin_rgb.clip::<Dest>()
    }
}

/// Ray-casting helper for raytracing gamut mapping algorithm.
/// <https://drafts.csswg.org/css-color-4/#pseudo-raytrace>
fn cast_ray(start: &Vector3, end: &Vector3) -> Option<Vector3> {
    // 1. let bmin and bmax be 3-element arrays with the gamut’s lower and upper bounds, respectively ^6
    const BMIN: Vector3 = [0.; 3];
    const BMAX: Vector3 = [1.; 3];

    // 2. let tfar be infinity (or some very large number)
    let mut tfar = f64::INFINITY;

    // 3. let tnear be -infinity (or some very large, negative number)
    let mut tnear = f64::NEG_INFINITY;

    // 4. let direction be a 3-element array
    let mut direction: Vector3 = [0.; 3];

    // 5. for (i = 0; i < 3; i++):
    for i in 0..3 {
        // 5.1. let a be start [i]
        let a = start[i];

        // 5.2. let b be end [i]
        let b = end[i];

        // 5.3. let d be b - a
        let d = b - a;

        // 5.4. let direction [i] be d
        direction[i] = d;

        // 5.5. if abs(d) < 1E-12 ***
        // NOTE there is a typo in the spec: comparison should be greater-than, not less-than.
        // I reported this to the CSS Color 4 working group on github.
        // See <https://github.com/w3c/csswg-drafts/issues/10579#issuecomment-4122988782>.
        // NOTE also that the epsilon value has adjusted during the draft process.
        // Pulling out the value to cover any future adjustments.
        const EPSILON: f64 = 1e-12;
        if d.abs() > EPSILON {
            // 5.5.1. let inv_d be 1 / d
            let inv_d = d.recip();

            // 5.5.2. let t1 be (bmin [i] - a ) * inv_d
            let t1 = (BMIN[i] - a) * inv_d;

            // 5.5.3. let t2 be (bmax [i] - a ) * inv_d
            let t2 = (BMAX[i] - a) * inv_d;

            // 5.5.4. let tnear be max(min(t1, t2), tnear )
            tnear = t1.min(t2).max(tnear);

            // 5.5.5. let tfar be min(max(t1, t2), tfar )
            tfar = t1.max(t2).min(tfar);
        }
        // 5.6. else if (a < bmin[i] or a > bmax[i])
        else if !(BMIN[i]..=BMAX[i]).contains(&a) {
            // 5.6.1. return INTERSECTION NOT FOUND
            return None;
        }
    }

    // 6. if (tnear > tfar or tfar < 0)
    //     return INTERSECTION NOT FOUND
    if tnear > tfar || tfar < 0. {
        return None;
    }

    // 7. if tnear < 0
    //     let tnear be tfar 7
    if tnear < 0. {
        tnear = tfar;
    }

    // 8. if tnear is infinite (or matches the initial very large value)
    //     return INTERSECTION NOT FOUND
    if tnear.is_infinite() {
        return None;
    }

    // 9. for (i =0; i < 3; i++):
    //     let result [i] be start [i] + direction [i] * tnear
    let result = std::array::from_fn(|i| start[i] + direction[i] * tnear);

    // 10. return result
    Some(result)
}

/// Euclidean distance to calculate color difference in OKLAB space
/// <https://drafts.csswg.org/css-color-4/#color-difference-OK>
fn delta_eok<T: ColorConversion, U: ColorConversion>(
    reference: &Color<T>,
    sample: &Color<U>,
) -> f64 {
    // make sure both colors are in OKLAB space for distance calculation
    let oklab_reference = reference.convert_to::<OKLAB>();
    let oklab_sample = sample.convert_to::<OKLAB>();

    let [l1, a1, b1] = oklab_reference.coords;
    let [l2, a2, b2] = oklab_sample.coords;

    let (delta_l, delta_a, delta_b) = (l1 - l2, a1 - a2, b1 - b2);

    (delta_l.powi(2) + delta_a.powi(2) + delta_b.powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_gamut_true() {
        assert!(Color::<SRGB>::new([1., 1., 0.]).in_gamut::<SRGB>())
    }

    #[test]
    fn test_in_gamut_false() {
        assert!(!Color::<SRGB>::new([1.1, 0., -0.2]).in_gamut::<SRGB>())
    }

    #[test]
    fn test_clip() {
        assert_eq!(
            Color::<SRGB>::new([1.1, 0., -0.2]).clip::<SRGB>().coords,
            [1., 0., 0.]
        )
    }

    #[test]
    fn test_gamutmap_local_minde() {
        // a very light, hyper-chroma yellow
        let src = Color::<OKLCH>::new([0.99, 0.8, 110.]);
        let result = src.gamutmap_local_minde::<SRGB>();
        // values confirmed from spec-compliant Coloraide python library
        let expected_coords = [1., 1., 0.33857];
        assert_eq!(set_precision(5, result.coords), expected_coords)
    }

    #[test]
    fn test_gamutmap_raytrace() {
        // a very light, hyper-chroma yellow
        let src = Color::<OKLCH>::new([0.99, 0.8, 110.]);
        let result = src.gamutmap_raytrace::<SRGB>();
        // values confirmed from spec-compliant Coloraide python library
        let expected_coords = [0.99213, 1., 0.87064];
        assert_eq!(set_precision(5, result.coords), expected_coords)
    }
}
