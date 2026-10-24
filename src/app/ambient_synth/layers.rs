use super::dsp::{Biquad, FastRng};
use std::f32::consts::PI;

pub(super) struct BrownNoiseGen {
    rng: FastRng,
    last_l: f32,
    last_r: f32,
    lp_l: Biquad,
    lp_r: Biquad,
}

impl BrownNoiseGen {
    pub(super) fn new(sample_rate: f32, seed: u32) -> Self {
        Self {
            rng: FastRng::new(seed),
            last_l: 0.0,
            last_r: 0.0,
            lp_l: Biquad::lowpass(sample_rate, 450.0, 0.707),
            lp_r: Biquad::lowpass(sample_rate, 450.0, 0.707),
        }
    }

    #[inline]
    pub(super) fn next_frame(&mut self) -> (f32, f32) {
        let wl = self.rng.next_f32();
        let wr = self.rng.next_f32();
        self.last_l = (self.last_l + wl * 0.05) * 0.985;
        self.last_r = (self.last_r + wr * 0.05) * 0.985;
        let fl = self.lp_l.process(self.last_l) * 0.85;
        let fr = self.lp_r.process(self.last_r) * 0.85;
        (fl, fr)
    }
}

pub(super) struct PinkNoiseGen {
    rng: FastRng,
    b0_l: f32,
    b1_l: f32,
    b2_l: f32,
    b3_l: f32,
    b4_l: f32,
    b5_l: f32,
    b0_r: f32,
    b1_r: f32,
    b2_r: f32,
    b3_r: f32,
    b4_r: f32,
    b5_r: f32,
}

impl PinkNoiseGen {
    pub(super) fn new(seed: u32) -> Self {
        Self {
            rng: FastRng::new(seed),
            b0_l: 0.0,
            b1_l: 0.0,
            b2_l: 0.0,
            b3_l: 0.0,
            b4_l: 0.0,
            b5_l: 0.0,
            b0_r: 0.0,
            b1_r: 0.0,
            b2_r: 0.0,
            b3_r: 0.0,
            b4_r: 0.0,
            b5_r: 0.0,
        }
    }

    #[inline]
    pub(super) fn next_frame(&mut self) -> (f32, f32) {
        let wl = self.rng.next_f32();
        self.b0_l = 0.99886 * self.b0_l + wl * 0.0555179;
        self.b1_l = 0.99332 * self.b1_l + wl * 0.0750759;
        self.b2_l = 0.96900 * self.b2_l + wl * 0.1538520;
        self.b3_l = 0.86650 * self.b3_l + wl * 0.3104856;
        self.b4_l = 0.55000 * self.b4_l + wl * 0.5329522;
        self.b5_l = -0.7616 * self.b5_l - wl * 0.0168980;
        let pink_l =
            (self.b0_l + self.b1_l + self.b2_l + self.b3_l + self.b4_l + self.b5_l + wl * 0.5362)
                * 0.14;

        let wr = self.rng.next_f32();
        self.b0_r = 0.99886 * self.b0_r + wr * 0.0555179;
        self.b1_r = 0.99332 * self.b1_r + wr * 0.0750759;
        self.b2_r = 0.96900 * self.b2_r + wr * 0.1538520;
        self.b3_r = 0.86650 * self.b3_r + wr * 0.3104856;
        self.b4_r = 0.55000 * self.b4_r + wr * 0.5329522;
        self.b5_r = -0.7616 * self.b5_r - wr * 0.0168980;
        let pink_r =
            (self.b0_r + self.b1_r + self.b2_r + self.b3_r + self.b4_r + self.b5_r + wr * 0.5362)
                * 0.14;

        (pink_l, pink_r)
    }
}

#[derive(Clone, Default)]
struct RainDrop {
    active: bool,
    env: f32,
    decay: f32,
    pan: f32,
    amp: f32,
    bp: Biquad,
}

impl RainDrop {
    fn trigger(&mut self, sample_rate: f32, rng: &mut FastRng) {
        let duration = 0.02 + rng.next_f32().abs() * 0.06;
        self.bp = Biquad::bandpass(
            sample_rate,
            1200.0 + rng.next_f32().abs() * 3800.0,
            2.0 + rng.next_f32().abs() * 4.0,
        );
        self.env = 1.0;
        self.decay = (-1.0 / (duration * sample_rate)).exp();
        self.pan = (rng.next_f32() + 1.0) * 0.5;
        self.amp = 0.04 + rng.next_f32().abs() * 0.14;
        self.active = true;
    }

    #[inline]
    fn next_frame(&mut self, noise: f32) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }
        let sample = self.bp.process(noise) * self.env * self.amp;
        self.env *= self.decay;
        if self.env < 0.002 {
            self.active = false;
        }
        (
            sample * (1.0 - self.pan * 0.6),
            sample * (0.4 + self.pan * 0.6),
        )
    }
}

pub(super) struct RainGen {
    rng: FastRng,
    bp_hiss_l: Biquad,
    bp_hiss_r: Biquad,
    bp_rumble_l: Biquad,
    bp_rumble_r: Biquad,
    thunder_lp: Biquad,
    thunder_timer: u32,
    thunder_pos: u32,
    thunder_duration: u32,
    thunder_pan: f32,
    drops: [RainDrop; 8],
    sample_rate: f32,
}

impl RainGen {
    pub(super) fn new(sample_rate: f32, seed: u32) -> Self {
        Self {
            rng: FastRng::new(seed),
            bp_hiss_l: Biquad::bandpass(sample_rate, 1200.0, 1.2),
            bp_hiss_r: Biquad::bandpass(sample_rate, 1350.0, 1.2),
            bp_rumble_l: Biquad::bandpass(sample_rate, 280.0, 1.8),
            bp_rumble_r: Biquad::bandpass(sample_rate, 310.0, 1.8),
            thunder_lp: Biquad::lowpass(sample_rate, 85.0, 1.5),
            thunder_timer: (sample_rate * 25.0) as u32,
            thunder_pos: 0,
            thunder_duration: 0,
            thunder_pan: 0.5,
            drops: std::array::from_fn(|_| RainDrop::default()),
            sample_rate,
        }
    }

    #[inline]
    pub(super) fn next_frame(&mut self) -> (f32, f32) {
        let wl = self.rng.next_f32();
        let wr = self.rng.next_f32();

        let hiss_l = self.bp_hiss_l.process(wl) * 0.42;
        let hiss_r = self.bp_hiss_r.process(wr) * 0.42;
        let body_l = self.bp_rumble_l.process(wl) * 0.35;
        let body_r = self.bp_rumble_r.process(wr) * 0.35;

        if self.rng.next_u32() < 2_500_000
            && let Some(drop) = self.drops.iter_mut().find(|drop| !drop.active)
        {
            drop.trigger(self.sample_rate, &mut self.rng);
        }

        let mut drops_l = 0.0f32;
        let mut drops_r = 0.0f32;
        for drop in &mut self.drops {
            let noise = self.rng.next_f32();
            let (left, right) = drop.next_frame(noise);
            drops_l += left;
            drops_r += right;
        }

        if self.thunder_duration > 0 && self.thunder_pos < self.thunder_duration {
            let frac = self.thunder_pos as f32 / self.thunder_duration as f32;
            let envelope = if frac < 0.22 {
                (frac / 0.22).sin()
            } else {
                ((1.0 - frac) / 0.78).clamp(0.0, 1.0) * (1.0 + (frac * 12.0).sin() * 0.25)
            };
            let thunder_raw = self.rng.next_f32();
            let thunder_filtered = self.thunder_lp.process(thunder_raw) * envelope * 0.48;

            self.thunder_pos += 1;
            let th_l = thunder_filtered * (1.0 - self.thunder_pan * 0.4);
            let th_r = thunder_filtered * (0.6 + self.thunder_pan * 0.4);
            (
                hiss_l + body_l + drops_l + th_l,
                hiss_r + body_r + drops_r + th_r,
            )
        } else {
            if self.thunder_timer > 0 {
                self.thunder_timer -= 1;
            } else {
                let dur_secs = 3.5 + (self.rng.next_f32().abs() * 3.5);
                self.thunder_duration = (dur_secs * self.sample_rate) as u32;
                self.thunder_pos = 0;
                self.thunder_pan = (self.rng.next_f32() + 1.0) * 0.5;
                let next_wait = 24.0 + (self.rng.next_f32().abs() * 30.0);
                self.thunder_timer = (next_wait * self.sample_rate) as u32;
            }
            (hiss_l + body_l + drops_l, hiss_r + body_r + drops_r)
        }
    }
}

#[derive(Clone, Default)]
struct FirePop {
    active: bool,
    env: f32,
    decay: f32,
    pan: f32,
    amp: f32,
    bp: Biquad,
}

impl FirePop {
    fn trigger(&mut self, sample_rate: f32, rng: &mut FastRng) {
        let duration = 0.025 + rng.next_f32().abs() * 0.07;
        self.bp = Biquad::bandpass(
            sample_rate,
            300.0 + rng.next_f32().abs() * 1600.0,
            3.0 + rng.next_f32().abs() * 5.0,
        );
        self.env = 1.0;
        self.decay = (-1.0 / (duration * sample_rate)).exp();
        self.pan = (rng.next_f32() + 1.0) * 0.5;
        self.amp = 0.08 + rng.next_f32().abs() * 0.18;
        self.active = true;
    }

    #[inline]
    fn next_frame(&mut self, noise: f32) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }
        let sample = self.bp.process(noise) * self.env * self.amp;
        self.env *= self.decay;
        if self.env < 0.002 {
            self.active = false;
        }
        (
            sample * (1.0 - self.pan * 0.7),
            sample * (0.3 + self.pan * 0.7),
        )
    }
}

pub(super) struct FireplaceGen {
    rng: FastRng,
    hearth_lp: Biquad,
    hearth_phase: f32,
    pops: [FirePop; 3],
    spark_bp: Biquad,
    spark_timer: u32,
    spark_decay: f32,
    spark_pan: f32,
    sample_rate: f32,
}

impl FireplaceGen {
    pub(super) fn new(sample_rate: f32, seed: u32) -> Self {
        Self {
            rng: FastRng::new(seed),
            hearth_lp: Biquad::lowpass(sample_rate, 140.0, 1.2),
            hearth_phase: 0.0,
            pops: std::array::from_fn(|_| FirePop::default()),
            spark_bp: Biquad::bandpass(sample_rate, 4500.0, 1.0),
            spark_timer: 0,
            spark_decay: 0.0,
            spark_pan: 0.5,
            sample_rate,
        }
    }

    #[inline]
    pub(super) fn next_frame(&mut self) -> (f32, f32) {
        let w = self.rng.next_f32();
        self.hearth_phase += 0.4 * 2.0 * PI / self.sample_rate;
        if self.hearth_phase > 2.0 * PI {
            self.hearth_phase -= 2.0 * PI;
        }
        let breathe = 0.8 + 0.2 * self.hearth_phase.sin();
        let rumble = self.hearth_lp.process(w) * 0.45 * breathe;

        if self.rng.next_u32() < 2_000_000
            && let Some(pop) = self.pops.iter_mut().find(|pop| !pop.active)
        {
            pop.trigger(self.sample_rate, &mut self.rng);
        }

        let mut pops_l = 0.0f32;
        let mut pops_r = 0.0f32;
        for pop in &mut self.pops {
            let noise = self.rng.next_f32();
            let (left, right) = pop.next_frame(noise);
            pops_l += left;
            pops_r += right;
        }

        if self.spark_timer == 0 {
            if self.rng.next_u32() < 2_000_000 {
                self.spark_timer = (0.025 * self.sample_rate) as u32;
                self.spark_decay = 0.05 + (self.rng.next_f32().abs() * 0.10);
                self.spark_pan = (self.rng.next_f32() + 1.0) * 0.5;
                self.spark_bp = Biquad::bandpass(
                    self.sample_rate,
                    2500.0 + self.rng.next_f32().abs() * 4500.0,
                    1.0,
                );
            }
        }

        let mut spark_sample = 0.0;
        if self.spark_timer > 0 {
            self.spark_timer -= 1;
            spark_sample = self.spark_bp.process(self.rng.next_f32()) * self.spark_decay;
            self.spark_decay *= 0.996;
        }

        let (spark_l, spark_r) = (
            spark_sample * (1.0 - self.spark_pan * 0.6),
            spark_sample * (0.4 + self.spark_pan * 0.6),
        );

        (
            rumble * 0.9 + pops_l + spark_l,
            rumble * 0.9 + pops_r + spark_r,
        )
    }
}

pub(super) struct LofiGen {
    rng: FastRng,
    vinyl_phase: f32,
    flutter_phase: f32,
    bass_phase: f32,
    chord_idx: usize,
    chord_timer: u32,
    chord_total_samples: u32,
    lp_l: Biquad,
    lp_r: Biquad,
    note_phases: [f32; 5],
    prev_note_phases: [f32; 5],
    prev_chord_idx: usize,
    sample_rate: f32,
}

impl LofiGen {
    pub(super) fn new(sample_rate: f32, seed: u32) -> Self {
        let chord_dur = (8.0 * sample_rate) as u32;
        Self {
            rng: FastRng::new(seed),
            vinyl_phase: 0.0,
            flutter_phase: 0.0,
            bass_phase: 0.0,
            chord_idx: 0,
            chord_timer: chord_dur,
            chord_total_samples: chord_dur,
            lp_l: Biquad::lowpass(sample_rate, 880.0, 0.707),
            lp_r: Biquad::lowpass(sample_rate, 880.0, 0.707),
            note_phases: [0.0; 5],
            prev_note_phases: [0.0; 5],
            prev_chord_idx: 0,
            sample_rate,
        }
    }

    fn chord_notes(idx: usize) -> [f32; 5] {
        match idx % 8 {
            0 => [130.81, 164.81, 196.00, 246.94, 293.66],
            1 => [110.00, 130.81, 164.81, 196.00, 246.94],
            2 => [146.83, 174.61, 220.00, 261.63, 329.63],
            3 => [98.00, 146.83, 174.61, 246.94, 293.66],
            4 => [123.47, 155.56, 196.00, 233.08, 293.66],
            5 => [130.81, 164.81, 196.00, 220.00, 293.66],
            6 => [110.00, 130.81, 164.81, 220.00, 261.63],
            _ => [146.83, 174.61, 220.00, 261.63, 349.23],
        }
    }

    #[inline]
    pub(super) fn next_frame(&mut self) -> (f32, f32) {
        self.vinyl_phase += 0.55 * 2.0 * PI / self.sample_rate;
        if self.vinyl_phase > 2.0 * PI {
            self.vinyl_phase -= 2.0 * PI;
        }
        let surface = self.rng.next_f32() * (0.018 + 0.005 * self.vinyl_phase.sin());
        let mut click = 0.0;
        if self.rng.next_u32() < 900_000 {
            click = self.rng.next_f32() * 0.05;
        }

        self.flutter_phase += 0.28 * 2.0 * PI / self.sample_rate;
        if self.flutter_phase > 2.0 * PI {
            self.flutter_phase -= 2.0 * PI;
        }
        let vibrato = 1.0 + 0.003 * self.flutter_phase.sin();

        if self.chord_timer > 0 {
            self.chord_timer -= 1;
        } else {
            self.prev_chord_idx = self.chord_idx;
            self.chord_idx = (self.chord_idx + 1) % 8;
            self.chord_timer = self.chord_total_samples;
        }

        let cross_len = (2.2 * self.sample_rate) as f32;
        let elapsed = (self.chord_total_samples - self.chord_timer) as f32;
        let fade_in = (elapsed / cross_len).clamp(0.0, 1.0);
        let fade_out = 1.0 - fade_in;

        let cur_freqs = Self::chord_notes(self.chord_idx);
        let mut cur_chord = 0.0f32;
        for i in 0..5 {
            let f = cur_freqs[i] * vibrato;
            self.note_phases[i] += 2.0 * PI * f / self.sample_rate;
            if self.note_phases[i] > 2.0 * PI {
                self.note_phases[i] -= 2.0 * PI;
            }
            let s1 = self.note_phases[i].sin();
            let s2 = (self.note_phases[i] * 2.0).sin() * 0.24;
            let s3 = (self.note_phases[i] * 3.0).sin() * 0.08;
            cur_chord += (s1 + s2 + s3) * 0.07;
        }

        let mut prev_chord = 0.0f32;
        if fade_in < 1.0 {
            let prev_freqs = Self::chord_notes(self.prev_chord_idx);
            for i in 0..5 {
                let f = prev_freqs[i] * vibrato;
                self.prev_note_phases[i] += 2.0 * PI * f / self.sample_rate;
                if self.prev_note_phases[i] > 2.0 * PI {
                    self.prev_note_phases[i] -= 2.0 * PI;
                }
                let s1 = self.prev_note_phases[i].sin();
                let s2 = (self.prev_note_phases[i] * 2.0).sin() * 0.24;
                let s3 = (self.prev_note_phases[i] * 3.0).sin() * 0.08;
                prev_chord += (s1 + s2 + s3) * 0.07;
            }
        }

        let bass_frequency = cur_freqs[0] * 0.5 * vibrato;
        self.bass_phase += 2.0 * PI * bass_frequency / self.sample_rate;
        if self.bass_phase > 2.0 * PI {
            self.bass_phase -= 2.0 * PI;
        }
        let bass = self.bass_phase.sin() * 0.04 * fade_in;

        let mixed_music = cur_chord * fade_in + prev_chord * fade_out + bass;
        let left = self.lp_l.process(mixed_music) + surface + click;
        let right = self.lp_r.process(mixed_music) + surface - click;
        (left, right)
    }
}

pub(super) struct BinauralPadGen {
    phase_l: f32,
    phase_r: f32,
    phase_sub: f32,
    lfo_phase: f32,
    sample_rate: f32,
}

impl BinauralPadGen {
    pub(super) fn new(sample_rate: f32) -> Self {
        Self {
            phase_l: 0.0,
            phase_r: 0.0,
            phase_sub: 0.0,
            lfo_phase: 0.0,
            sample_rate,
        }
    }

    #[inline]
    pub(super) fn next_frame(&mut self) -> (f32, f32) {
        self.lfo_phase += 0.06 * 2.0 * PI / self.sample_rate;
        if self.lfo_phase > 2.0 * PI {
            self.lfo_phase -= 2.0 * PI;
        }
        let swell = 0.75 + 0.25 * self.lfo_phase.sin();

        self.phase_l += 2.0 * PI * 108.0 / self.sample_rate;
        if self.phase_l > 2.0 * PI {
            self.phase_l -= 2.0 * PI;
        }

        self.phase_r += 2.0 * PI * 118.0 / self.sample_rate;
        if self.phase_r > 2.0 * PI {
            self.phase_r -= 2.0 * PI;
        }

        self.phase_sub += 2.0 * PI * 54.0 / self.sample_rate;
        if self.phase_sub > 2.0 * PI {
            self.phase_sub -= 2.0 * PI;
        }

        let tone_l = self.phase_l.sin() * 0.18;
        let tone_r = self.phase_r.sin() * 0.18;
        let sub = self.phase_sub.sin() * 0.05;

        ((tone_l + sub) * swell, (tone_r + sub) * swell)
    }
}
