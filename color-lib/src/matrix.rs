// Copyright © 2026 John P. ARNZEN
// SPDX-License-Identifier: MIT

/// matrix of 3 rows of 3 columns
/// references are m[r][c] (type is [[T; c]; r])
pub type Matrix3 = [[f64; 3]; 3];
/// vector of 3 rows (of 1 column)
pub type Vector3 = [f64; 3];

pub fn matrix3_mult_vector3(m: &Matrix3, v: &Vector3) -> Vector3 {
    [
        (m[0][0] * v[0]) + (m[0][1] * v[1]) + (m[0][2] * v[2]),
        (m[1][0] * v[0]) + (m[1][1] * v[1]) + (m[1][2] * v[2]),
        (m[2][0] * v[0]) + (m[2][1] * v[1]) + (m[2][2] * v[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_mult() {
        #[rustfmt::skip]
        let m: Matrix3 = [
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.]
        ];
        let v: Vector3 = [1., 1., 1.];

        assert_eq!(matrix3_mult_vector3(&m, &v), v);
    }

    #[test]
    fn test_simple_mult() {
        #[rustfmt::skip]
        let m: Matrix3 = [
            [1., 0., 0.],
            [0., 2., 0.],
            [0., 0., 3.]
        ];
        let v: Vector3 = [2., 4., 6.];

        assert_eq!(
            matrix3_mult_vector3(&m, &v),
            [m[0][0] * v[0], m[1][1] * v[1], m[2][2] * v[2]]
        );
    }
}
