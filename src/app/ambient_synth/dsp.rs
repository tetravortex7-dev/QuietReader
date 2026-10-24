use std::f32::consts::PI;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub(super) struct FastRng {
    state: u32,
}

impl FastRng {
    pub(super) fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x9E37_79B9 } else { seed },
        }
    }

    #[inline]
    pub(super) fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    #[inline]
    pub(super) fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / 4_294_967_295.0 * 2.0 - 1.0
    }
}

pub(super) fn session_seed(salt: u32) -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    let mixed = nanos ^ ((std::process::id() as u64) << 32) ^ (salt as u64) ^ 0x9E37_79B9_7F4A_7C15;
    let mut state = (mixed as u32) ^ ((mixed >> 32) as u32);
    state ^= state << 13;
    state ^= state >> 17;
    state ^= state << 5;
    if state == 0 { 0x9E37_79B9 } else { state }
}

#[derive(Clone, Default)]
pub(super) struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub(super) fn lowpass(sample_rate: f32, cutoff: f32, q: f32) -> Self {
        let mut filter = Self::default();
        filter.set_lowpass(sample_rate, cutoff, q);
        filter
    }

    pub(super) fn bandpass(sample_rate: f32, center: f32, q: f32) -> Self {
        let mut filter = Self::default();
        filter.set_bandpass(sample_rate, center, q);
        filter
    }

    pub(super) fn set_lowpass(&mut self, sample_rate: f32, cutoff: f32, q: f32) {
        let w0 = (2.0 * PI * cutoff / sample_rate).clamp(0.001, PI - 0.01);
        let alpha = w0.sin() / (2.0 * q.max(0.1));
        let cos_w0 = w0.cos();
        let a0 = 1.0 + alpha;
        self.b0 = (1.0 - cos_w0) / 2.0 / a0;
        self.b1 = (1.0 - cos_w0) / a0;
        self.b2 = (1.0 - cos_w0) / 2.0 / a0;
        self.a1 = -2.0 * cos_w0 / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    pub(super) fn set_bandpass(&mut self, sample_rate: f32, center: f32, q: f32) {
        let w0 = (2.0 * PI * center / sample_rate).clamp(0.001, PI - 0.01);
        let alpha = w0.sin() / (2.0 * q.max(0.1));
        let cos_w0 = w0.cos();
        let a0 = 1.0 + alpha;
        self.b0 = alpha / a0;
        self.b1 = 0.0;
        self.b2 = -alpha / a0;
        self.a1 = -2.0 * cos_w0 / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    #[inline]
    pub(super) fn process(&mut self, input: f32) -> f32 {
        let out = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * out + self.z2;
        self.z2 = self.b2 * input - self.a2 * out;
        out
    }
}

#[derive(Clone, Default)]
pub(super) struct DcBlocker {
    previous_input: f32,
    previous_output: f32,
}

impl DcBlocker {
    #[inline]
    pub(super) fn process(&mut self, input: f32, coefficient: f32) -> f32 {
        let output = input - self.previous_input + coefficient * self.previous_output;
        self.previous_input = input;
        self.previous_output = output;
        output
    }
}

pub(super) struct Limiter {
    gain: f32,
    attack_step: f32,
    release_step: f32,
}

impl Limiter {
    pub(super) fn new(sample_rate: f32) -> Self {
        Self {
            gain: 1.0,
            attack_step: 1.0 / (sample_rate * 0.001).max(1.0),
            release_step: 1.0 / (sample_rate * 0.2).max(1.0),
        }
    }

    #[inline]
    pub(super) fn process_stereo(&mut self, left: f32, right: f32, threshold: f32) -> (f32, f32) {
        let peak = left.abs().max(right.abs());
        let target = if peak > threshold {
            threshold / peak
        } else {
            1.0
        };
        if target < self.gain {
            self.gain = (self.gain - self.attack_step).max(target);
        } else {
            self.gain = (self.gain + self.release_step).min(target);
        }
        (
            (left * self.gain).clamp(-0.99, 0.99),
            (right * self.gain).clamp(-0.99, 0.99),
        )
    }
}

#[inline]
pub(super) fn smooth_toward(current: &mut f32, target: f32, step: f32) {
    if (*current - target).abs() <= step {
        *current = target;
    } else if *current < target {
        *current += step;
    } else {
        *current -= step;
    }
}
