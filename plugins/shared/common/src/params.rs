//! Parameter builders shared by the plugins.
use crate::fmt::*;
use nih_plug::prelude::*;
use std::sync::Arc;

/// A skewed range whose midpoint is the geometric mean of the ends (close to a logarithmic scale).
pub fn log_range(min: f32, max: f32) -> FloatRange {
    let mid = (min * max).sqrt();
    let k = ((mid - min) / (max - min)).ln() / 0.5f32.ln();
    FloatRange::Skewed { min, max, factor: FloatRange::skew_factor(-(k.log2())) }
}

pub fn db_param(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min, max })
        .with_smoother(SmoothingStyle::Linear(20.0))
        .with_value_to_string(Arc::new(fmt_db))
        .with_string_to_value(Arc::new(|t| parse_db(t)))
}

pub fn pct_param(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min, max })
        .with_value_to_string(Arc::new(fmt_percent))
        .with_string_to_value(Arc::new(|t| parse_percent(t)))
}

pub fn ms_log_param(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, log_range(min, max))
        .with_value_to_string(Arc::new(fmt_ms_auto))
        .with_string_to_value(Arc::new(|t| parse_ms(t)))
}

pub fn hz_log_param(name: &'static str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, log_range(min, max))
        .with_value_to_string(Arc::new(fmt_hz))
        .with_string_to_value(Arc::new(|t| parse_hz(t)))
}
