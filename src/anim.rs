//! Easing curves for the animations.

pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// How far along `t` (0..1) is through the stretch from `start` lasting `len`.
pub fn stage(t: f32, start: f32, len: f32) -> f32 {
    ((t - start) / len).clamp(0.0, 1.0)
}

/// Overshoots a little before settling.
pub fn ease_out_back(t: f32) -> f32 {
    let c = 1.6;
    1.0 + (c + 1.0) * (t - 1.0).powi(3) + c * (t - 1.0).powi(2)
}
