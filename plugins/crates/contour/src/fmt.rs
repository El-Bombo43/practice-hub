//! Text <-> value conversions for parameters and the editor's value fields (units are optional when typing).

/// 82.4 Hz, 1.25 kHz.
pub fn fmt_freq(hz: f32) -> String {
    if hz < 1000.0 {
        format!("{} Hz", trim_zeros(format!("{hz:.1}")))
    } else {
        format!("{} kHz", trim_zeros(format!("{:.2}", hz / 1000.0)))
    }
}

fn trim_zeros(s: String) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// "82.4", "82.4 Hz", "2k", "2.5k", "2.5 kHz", "1e3". Returns Hz.
pub fn parse_freq(text: &str) -> Option<f32> {
    let t = text.trim().to_lowercase().replace(' ', "");
    let (num, mult) = if let Some(n) = t.strip_suffix("khz") {
        (n, 1000.0)
    } else if let Some(n) = t.strip_suffix('k') {
        (n, 1000.0)
    } else if let Some(n) = t.strip_suffix("hz") {
        (n, 1.0)
    } else {
        (t.as_str(), 1.0)
    };
    num.parse::<f32>().ok().filter(|v| v.is_finite()).map(|v| v * mult)
}

pub fn fmt_db(db: f32, digits: usize) -> String {
    format!("{db:.digits$} dB")
}

/// "-3", "-3.5", "-3.5 dB", "+2db".
pub fn parse_db(text: &str) -> Option<f32> {
    let t = text.trim().to_lowercase().replace(' ', "");
    let n = t.strip_suffix("db").unwrap_or(&t);
    n.trim_start_matches('+').parse::<f32>().ok().filter(|v| v.is_finite())
}

pub fn fmt_q(q: f32) -> String {
    format!("Q {q:.2}")
}

/// "1.4", "Q 1.4", "q1.4".
pub fn parse_q(text: &str) -> Option<f32> {
    let t = text.trim().to_lowercase().replace(' ', "");
    t.strip_prefix('q').unwrap_or(&t).parse::<f32>().ok().filter(|v| v.is_finite())
}

pub fn fmt_ms(ms: f32) -> String {
    if ms < 100.0 {
        format!("{ms:.1} ms")
    } else {
        format!("{ms:.0} ms")
    }
}

/// "10", "10 ms", "0.2 s".
pub fn parse_ms(text: &str) -> Option<f32> {
    let t = text.trim().to_lowercase().replace(' ', "");
    if let Some(n) = t.strip_suffix("ms") {
        n.parse::<f32>().ok()
    } else if let Some(n) = t.strip_suffix('s') {
        n.parse::<f32>().ok().map(|s| s * 1000.0)
    } else {
        t.parse::<f32>().ok()
    }
}

pub fn fmt_percent(p: f32) -> String {
    format!("{p:.0} %")
}

pub fn parse_percent(text: &str) -> Option<f32> {
    text.trim().trim_end_matches('%').trim().parse::<f32>().ok()
}

/// 82.4 Hz -> "E2". Equal temperament, A4 = 440 Hz.
pub fn note_name(hz: f32) -> String {
    if hz <= 0.0 {
        return String::new();
    }
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let midi = (69.0 + 12.0 * (hz / 440.0).log2()).round() as i32;
    format!("{}{}", NAMES[midi.rem_euclid(12) as usize], midi.div_euclid(12) - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_formatting() {
        assert_eq!(fmt_freq(82.4), "82.4 Hz");
        assert_eq!(fmt_freq(1000.0), "1 kHz");
        assert_eq!(fmt_freq(1250.0), "1.25 kHz");
        assert_eq!(fmt_freq(20.0), "20 Hz");
        assert_eq!(fmt_freq(30000.0), "30 kHz");
    }

    #[test]
    fn frequency_parsing_accepts_units_or_none() {
        for (s, v) in [("2k", 2000.0), ("2.5 kHz", 2500.0), ("300", 300.0), ("300hz", 300.0), (" 82.4 Hz ", 82.4), ("1.25K", 1250.0)] {
            assert!((parse_freq(s).unwrap() - v).abs() < 1e-3, "{s}");
        }
        assert_eq!(parse_freq("abc"), None);
        assert_eq!(parse_freq(""), None);
    }

    #[test]
    fn round_trips() {
        for hz in [20.0f32, 82.4, 440.0, 999.9, 1250.0, 15000.0] {
            let back = parse_freq(&fmt_freq(hz)).unwrap();
            assert!((back - hz).abs() / hz < 0.006, "{hz} -> {back}");
        }
    }

    #[test]
    fn db_q_ms_percent() {
        assert_eq!(fmt_db(-3.5, 2), "-3.50 dB");
        assert_eq!(parse_db("-3"), Some(-3.0));
        assert_eq!(parse_db("+2 dB"), Some(2.0));
        assert_eq!(parse_db("-3.5db"), Some(-3.5));
        assert_eq!(fmt_q(1.0), "Q 1.00");
        assert_eq!(parse_q("Q 1.4"), Some(1.4));
        assert_eq!(parse_q("0.71"), Some(0.71));
        assert_eq!(fmt_ms(10.0), "10.0 ms");
        assert_eq!(fmt_ms(150.0), "150 ms");
        assert_eq!(parse_ms("0.2 s"), Some(200.0));
        assert_eq!(parse_ms("15"), Some(15.0));
        assert_eq!(fmt_percent(100.0), "100 %");
        assert_eq!(parse_percent("150%"), Some(150.0));
    }

    #[test]
    fn note_names() {
        assert_eq!(note_name(440.0), "A4");
        assert_eq!(note_name(82.41), "E2");
        assert_eq!(note_name(261.63), "C4");
        assert_eq!(note_name(0.0), "");
    }
}
