//! Graph geometry shared by drawing and hit-testing (pure, so it can be unit-tested).

pub const F_MIN: f32 = 10.0;
pub const F_MAX: f32 = 30_000.0;
pub const RANGES_DB: [f32; 4] = [3.0, 6.0, 12.0, 30.0];

fn ln_span() -> f32 {
    (F_MAX / F_MIN).ln()
}

/// 0.0 (10 Hz) to 1.0 (30 kHz), logarithmic.
pub fn freq_to_frac(f: f32) -> f32 {
    (f.clamp(F_MIN, F_MAX) / F_MIN).ln() / ln_span()
}

pub fn frac_to_freq(x: f32) -> f32 {
    F_MIN * (x.clamp(0.0, 1.0) * ln_span()).exp()
}

/// 0.0 at the top (+range) to 1.0 at the bottom (-range).
pub fn gain_to_frac(g: f32, range_db: f32) -> f32 {
    0.5 - g.clamp(-range_db, range_db) / (2.0 * range_db)
}

pub fn frac_to_gain(y: f32, range_db: f32) -> f32 {
    (0.5 - y) * 2.0 * range_db
}

/// Gridline gain spacing for a display range.
pub fn grid_step(range_db: f32) -> f32 {
    if range_db <= 3.0 {
        1.0
    } else if range_db <= 6.0 {
        3.0
    } else if range_db <= 12.0 {
        3.0
    } else {
        6.0
    }
}

/// Lowest band number (0-based) that is not in use.
pub fn first_unused(used: &[bool]) -> Option<usize> {
    used.iter().position(|u| !u)
}

/// Which band (if any) sits within `radius` of the pointer; the nearest wins. Later bands win ties (drawn on top).
pub fn hit_node(points: &[Option<(f32, f32)>], pos: (f32, f32), radius: f32) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (i, p) in points.iter().enumerate() {
        if let Some((x, y)) = p {
            let d = ((x - pos.0).powi(2) + (y - pos.1).powi(2)).sqrt();
            if d <= radius && best.map_or(true, |(_, bd)| d <= bd) {
                best = Some((i, d));
            }
        }
    }
    best.map(|(i, _)| i)
}

/// What type a double-click creates at horizontal position `frac` (0..1) of the graph.
pub fn created_type(frac: f32) -> CreateKind {
    if frac < 0.05 {
        CreateKind::LowCut
    } else if frac > 0.95 {
        CreateKind::HighCut
    } else {
        CreateKind::Bell
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CreateKind {
    LowCut,
    HighCut,
    Bell,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_mapping_round_trips_and_hits_the_ends() {
        assert!((freq_to_frac(10.0)).abs() < 1e-6);
        assert!((freq_to_frac(30_000.0) - 1.0).abs() < 1e-6);
        for f in [20.0, 82.4, 1000.0, 15_000.0] {
            assert!((frac_to_freq(freq_to_frac(f)) - f).abs() / f < 1e-4);
        }
        // 1 kHz sits two thirds of the way across a 10 Hz to 30 kHz axis
        assert!((freq_to_frac(1000.0) - 0.5756).abs() < 0.001);
    }

    #[test]
    fn gain_mapping() {
        assert_eq!(gain_to_frac(0.0, 12.0), 0.5);
        assert_eq!(gain_to_frac(12.0, 12.0), 0.0);
        assert_eq!(gain_to_frac(-12.0, 12.0), 1.0);
        assert_eq!(gain_to_frac(99.0, 12.0), 0.0); // clamped to the display
        assert!((frac_to_gain(0.25, 12.0) - 6.0).abs() < 1e-5);
    }

    #[test]
    fn nearest_node_within_radius_wins() {
        let pts = [Some((100.0, 100.0)), None, Some((110.0, 100.0)), Some((300.0, 300.0))];
        assert_eq!(hit_node(&pts, (112.0, 100.0), 9.0), Some(2));
        assert_eq!(hit_node(&pts, (104.0, 100.0), 9.0), Some(0));
        assert_eq!(hit_node(&pts, (200.0, 200.0), 9.0), None);
    }

    #[test]
    fn new_bands_take_the_lowest_free_slot() {
        assert_eq!(first_unused(&[true, true, false, true]), Some(2));
        assert_eq!(first_unused(&[true, true]), None);
    }

    #[test]
    fn double_click_creates_cuts_at_the_edges() {
        assert_eq!(created_type(0.02), CreateKind::LowCut);
        assert_eq!(created_type(0.5), CreateKind::Bell);
        assert_eq!(created_type(0.97), CreateKind::HighCut);
        assert_eq!(created_type(0.05), CreateKind::Bell);
    }
}
