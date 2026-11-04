//! Scales, note masks and the note decision with hysteresis.

pub const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/// Scale intervals in semitones from the key. Order matches the `Scale` parameter (Custom has none).
pub const SCALES: [(&str, &[u8]); 12] = [
    ("Chromatic", &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
    ("Major", &[0, 2, 4, 5, 7, 9, 11]),
    ("Natural Minor", &[0, 2, 3, 5, 7, 8, 10]),
    ("Harmonic Minor", &[0, 2, 3, 5, 7, 8, 11]),
    ("Melodic Minor", &[0, 2, 3, 5, 7, 9, 11]),
    ("Major Pentatonic", &[0, 2, 4, 7, 9]),
    ("Minor Pentatonic", &[0, 3, 5, 7, 10]),
    ("Blues", &[0, 3, 5, 6, 7, 10]),
    ("Dorian", &[0, 2, 3, 5, 7, 9, 10]),
    ("Phrygian", &[0, 1, 3, 5, 7, 8, 10]),
    ("Lydian", &[0, 2, 4, 6, 7, 9, 11]),
    ("Mixolydian", &[0, 2, 4, 5, 7, 9, 10]),
];

/// The 12 note flags (C = index 0) for a key and a scale index into `SCALES`.
pub fn mask_for(key: usize, scale: usize) -> [bool; 12] {
    let mut m = [false; 12];
    for iv in SCALES[scale.min(11)].1 {
        m[(key + *iv as usize) % 12] = true;
    }
    m
}

/// MIDI note number (float) of a frequency.
pub fn hz_to_midi(hz: f32) -> f32 {
    69.0 + 12.0 * (hz / 440.0).log2()
}
pub fn midi_to_hz(m: f32) -> f32 {
    440.0 * 2f32.powf((m - 69.0) / 12.0)
}

/// The nearest enabled note (as a MIDI integer, any octave) to `m`.
pub fn nearest_enabled(m: f32, mask: &[bool; 12]) -> Option<i32> {
    if !mask.iter().any(|b| *b) {
        return None;
    }
    let base = m.round() as i32;
    let mut best: Option<(f32, i32)> = None;
    for d in -7..=7 {
        let n = base + d;
        if mask[n.rem_euclid(12) as usize] {
            let dist = (n as f32 - m).abs();
            if best.map_or(true, |(bd, _)| dist < bd) {
                best = Some((dist, n));
            }
        }
    }
    best.map(|(_, n)| n)
}

/// Keep the current target unless the pitch has moved at least `hyst` semitones past the halfway point to a better one.
pub fn decide(m: f32, current: Option<i32>, mask: &[bool; 12], hyst: f32) -> Option<i32> {
    let cand = nearest_enabled(m, mask)?;
    match current {
        Some(cur) if mask[cur.rem_euclid(12) as usize] && cur != cand => {
            // beyond the midpoint by `hyst` on the candidate's side: distance to current exceeds distance to cand by 2*hyst
            if (m - cur as f32).abs() - (m - cand as f32).abs() > 2.0 * hyst {
                Some(cand)
            } else {
                Some(cur)
            }
        }
        Some(cur) if mask[cur.rem_euclid(12) as usize] => Some(cur),
        _ => Some(cand),
    }
}
