//! Plain DSP, kept free of plugin types so it can be unit-tested directly.

/// Multiply every sample by `gain`.
pub fn apply_gain(buf: &mut [f32], gain: f32) {
    for s in buf {
        *s *= gain;
    }
}

/// Peak of the absolute sample values (0.0 for an empty buffer).
pub fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0_f32, |m, s| m.max(s.abs()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_gain_is_a_no_op() {
        let mut b = [0.25_f32, -0.5, 1.0, 0.0];
        let orig = b;
        apply_gain(&mut b, 1.0);
        assert_eq!(b, orig);
    }

    #[test]
    fn six_db_roughly_doubles() {
        let g = 10.0_f32.powf(6.0 / 20.0);
        let mut b = [0.5_f32; 8];
        apply_gain(&mut b, g);
        assert!((b[0] - 0.5 * g).abs() < 1e-6);
        assert!((peak(&b) - 0.9976).abs() < 1e-3);
    }

    #[test]
    fn peak_ignores_sign_and_handles_empty() {
        assert_eq!(peak(&[]), 0.0);
        assert_eq!(peak(&[0.1, -0.7, 0.3]), 0.7);
    }

    #[test]
    fn never_produces_nan_from_finite_input() {
        let mut b: Vec<f32> = (0..4096).map(|i| ((i as f32) * 0.05).sin()).collect();
        apply_gain(&mut b, 31.6);
        assert!(b.iter().all(|s| s.is_finite()));
    }
}
