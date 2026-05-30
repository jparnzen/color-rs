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
