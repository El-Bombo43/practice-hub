//! Text <-> value conversions for parameters (units optional when typing).

fn trim_zeros(s: String) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

pub fn number(text: &str, suffixes: &[(&str, f32)]) -> Option<f32> {
    let t = text.trim().to_lowercase().replace(' ', "").replace('×', "x");
    for (suf, mult) in suffixes {
        if let Some(n) = t.strip_suffix(suf) {
            return n.parse::<f32>().ok().filter(|v| v.is_finite()).map(|v| v * mult);
        }
    }
    t.parse::<f32>().ok().filter(|v| v.is_finite())
}

pub fn fmt_percent(v: f32) -> String {
    format!("{} %", v.round() as i32)
}
pub fn parse_percent(s: &str) -> Option<f32> {
    number(s, &[("%", 1.0)])
}
pub fn fmt_ms(v: f32) -> String {
    format!("{v:.1} ms")
}
pub fn fmt_ms0(v: f32) -> String {
    format!("{} ms", v.round() as i32)
}
pub fn parse_ms(s: &str) -> Option<f32> {
    number(s, &[("ms", 1.0), ("s", 1000.0)])
}
pub fn fmt_s(v: f32) -> String {
    format!("{v:.2} s")
}
pub fn parse_s(s: &str) -> Option<f32> {
    number(s, &[("ms", 0.001), ("s", 1.0)])
}
pub fn fmt_db(v: f32) -> String {
    format!("{v:.1} dB")
}
pub fn parse_db(s: &str) -> Option<f32> {
    number(s, &[("db", 1.0)])
}
pub fn fmt_hz(v: f32) -> String {
    if v < 1000.0 {
        format!("{} Hz", trim_zeros(format!("{v:.1}")))
    } else {
        format!("{:.1} kHz", v / 1000.0)
    }
}
pub fn fmt_khz2(v: f32) -> String {
    if v < 1000.0 {
        format!("{} Hz", v.round() as i32)
    } else {
        format!("{:.2} kHz", v / 1000.0)
    }
}
pub fn parse_hz(s: &str) -> Option<f32> {
    number(s, &[("khz", 1000.0), ("k", 1000.0), ("hz", 1.0)])
}
pub fn fmt_x(v: f32) -> String {
    format!("{v:.2}×")
}
pub fn parse_x(s: &str) -> Option<f32> {
    number(s, &[("x", 1.0)])
}
pub fn fmt_rate(v: f32) -> String {
    format!("{v:.2} Hz")
}
pub fn fmt_duck(v: f32) -> String {
    if v <= 0.0 {
        "Off".to_string()
    } else {
        fmt_db(v)
    }
}
pub fn parse_duck(s: &str) -> Option<f32> {
    if s.trim().eq_ignore_ascii_case("off") {
        Some(0.0)
    } else {
        parse_db(s)
    }
}

pub fn fmt_ms_auto(v: f32) -> String {
    if v < 100.0 {
        format!("{v:.1} ms")
    } else {
        format!("{} ms", v.round() as i32)
    }
}
pub fn fmt_hz0(v: f32) -> String {
    format!("{} Hz", v.round() as i32)
}
pub fn fmt_ratio(v: f32) -> String {
    format!("{v:.1}:1")
}
pub fn parse_ratio(s: &str) -> Option<f32> {
    let t = s.trim().to_lowercase().replace(' ', "");
    let t = t.strip_suffix(":1").unwrap_or(&t);
    t.parse::<f32>().ok().filter(|v| v.is_finite())
}
pub fn fmt_signed_pct(v: f32) -> String {
    format!("{:+} %", v.round() as i32)
}
