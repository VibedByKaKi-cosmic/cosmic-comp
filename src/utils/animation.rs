// SPDX-License-Identifier: GPL-3.0-only

//! Central switch for compositor animations.
//!
//! Every animation timing goes through [`duration`] or [`progress`], so the
//! `animations` and `animation_speed` config keys apply everywhere. When
//! animations are disabled, [`duration`] returns zero and [`progress`] reports
//! every animation as finished.

use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};

/// Accepted range for `animation_speed`. Values outside it are clamped.
pub const SPEED_RANGE: std::ops::RangeInclusive<f64> = 0.1..=10.0;

static ENABLED: AtomicBool = AtomicBool::new(true);
static SPEED: AtomicU64 = AtomicU64::new(1f64.to_bits());

/// Applies the animation config. Call on startup and whenever the config changes.
pub fn configure(enabled: bool, speed: f64) {
    let speed = if speed.is_finite() {
        speed.clamp(*SPEED_RANGE.start(), *SPEED_RANGE.end())
    } else {
        1.0
    };
    ENABLED.store(enabled, Ordering::Relaxed);
    SPEED.store(speed.to_bits(), Ordering::Relaxed);
}

/// Whether animations are enabled at all.
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

fn speed() -> f64 {
    f64::from_bits(SPEED.load(Ordering::Relaxed))
}

/// The length of an animation whose base length is `base`.
///
/// Divides `base` by the configured speed. Returns zero when animations are disabled.
pub fn duration(base: Duration) -> Duration {
    if enabled() {
        base.div_f64(speed())
    } else {
        Duration::ZERO
    }
}

/// The fraction of the animation with base length `base` that `elapsed` covers, from 0.0 to 1.0.
pub fn progress(elapsed: Duration, base: Duration) -> f32 {
    fraction(elapsed, duration(base))
}

/// The fraction of `total` that `elapsed` covers, from 0.0 to 1.0.
///
/// Use this for a duration that already went through [`duration`]. A zero-length
/// animation counts as finished.
pub fn fraction(elapsed: Duration, total: Duration) -> f32 {
    if total.is_zero() {
        1.0
    } else {
        (elapsed.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0)
    }
}

/// Scales a spring stiffness so that the spring runs `animation_speed` times faster.
///
/// With a fixed damping ratio, scaling stiffness by speed squared scales the time axis by speed.
pub fn spring_stiffness(stiffness: f64) -> f64 {
    stiffness * speed() * speed()
}
