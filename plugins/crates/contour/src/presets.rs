//! Factory presets: simple band lists (everything not mentioned is reset to default).

use crate::dsp::FilterType;

#[derive(Clone, Copy)]
pub struct PresetBand {
    pub ftype: FilterType,
    pub freq: f32,
    pub gain: f32,
    pub q: f32,
    pub slope: u32,
    /// (threshold dB, range dB, attack ms, release ms) when the band is dynamic
    pub dynamic: Option<(f32, f32, f32, f32)>,
}

const fn b(ftype: FilterType, freq: f32, gain: f32, q: f32, slope: u32, dynamic: Option<(f32, f32, f32, f32)>) -> PresetBand {
    PresetBand { ftype, freq, gain, q, slope, dynamic }
}

pub struct Preset {
    pub name: &'static str,
    pub bands: &'static [PresetBand],
}

use FilterType::*;

pub const PRESETS: &[Preset] = &[
    Preset { name: "Flat", bands: &[] },
    Preset {
        name: "Guitar - Tighten Lows",
        bands: &[
            b(LowCut, 80.0, 0.0, 0.71, 24, None),
            b(Bell, 180.0, -2.0, 1.4, 12, Some((-24.0, -4.0, 5.0, 120.0))),
            b(Bell, 3200.0, -2.0, 2.0, 12, None),
        ],
    },
    Preset {
        name: "Vocal - Clarity",
        bands: &[
            b(LowCut, 90.0, 0.0, 0.71, 18, None),
            b(Bell, 300.0, -2.5, 1.2, 12, None),
            b(HighShelf, 10_000.0, 2.0, 0.7, 12, None),
            b(Bell, 6500.0, 0.0, 3.0, 12, Some((-24.0, -5.0, 10.0, 150.0))),
        ],
    },
    Preset {
        name: "Bass - Clean",
        bands: &[
            b(LowCut, 35.0, 0.0, 0.71, 24, None),
            b(Bell, 250.0, -3.0, 1.0, 12, None),
            b(Bell, 800.0, 1.5, 1.2, 12, None),
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_match_the_spec_table() {
        assert_eq!(PRESETS.len(), 4);
        assert!(PRESETS[0].bands.is_empty());
        let g = &PRESETS[1];
        assert_eq!(g.bands.len(), 3);
        assert_eq!((g.bands[0].ftype, g.bands[0].freq, g.bands[0].slope), (LowCut, 80.0, 24));
        assert_eq!(g.bands[1].dynamic, Some((-24.0, -4.0, 5.0, 120.0)));
        assert_eq!(PRESETS[2].bands[3].dynamic.map(|d| d.1), Some(-5.0));
        assert_eq!(PRESETS[3].bands[0].freq, 35.0);
        for p in PRESETS {
            assert!(p.bands.len() <= 16);
        }
    }
}
