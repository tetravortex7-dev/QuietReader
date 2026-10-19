use rodio::Source;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct FastRng {
    state: u32,
}

impl FastRng {
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x9E3779B9 } else { seed },
        }
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / 4294967295.0 * 2.0 - 1.0
    }
}

#[derive(Clone, Default)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub fn lowpass(sample_rate: f32, cutoff: f32, q: f32) -> Self {
        let w0 = (2.0 * PI * cutoff / sample_rate).clamp(0.001, PI - 0.01);
        let alpha = w0.sin() / (2.0 * q.max(0.1));
        let cos_w0 = w0.cos();

        let a0 = 1.0 + alpha;
        let b0 = (1.0 - cos_w0) / 2.0 / a0;
        let b1 = (1.0 - cos_w0) / a0;
        let b2 = (1.0 - cos_w0) / 2.0 / a0;
        let a1 = -2.0 * cos_w0 / a0;
        let a2 = (1.0 - alpha) / a0;

        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn bandpass(sample_rate: f32, center: f32, q: f32) -> Self {
        let w0 = (2.0 * PI * center / sample_rate).clamp(0.001, PI - 0.01);
        let alpha = w0.sin() / (2.0 * q.max(0.1));
        let cos_w0 = w0.cos();

        let a0 = 1.0 + alpha;
        let b0 = alpha / a0;
        let b1 = 0.0;
        let b2 = -alpha / a0;
        let a1 = -2.0 * cos_w0 / a0;
        let a2 = (1.0 - alpha) / a0;

        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            z1: 0.0,
            z2: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let out = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * out + self.z2;
        self.z2 = self.b2 * input - self.a2 * out;
        out
    }
}

pub struct BrownNoiseGen {
    rng: FastRng,
    last_l: f32,
    last_r: f32,
    lp_l: Biquad,
    lp_r: Biquad,
}

impl BrownNoiseGen {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            rng: FastRng::new(12345),
            last_l: 0.0,
            last_r: 0.0,
            lp_l: Biquad::lowpass(sample_rate, 450.0, 0.707),
            lp_r: Biquad::lowpass(sample_rate, 450.0, 0.707),
        }
    }

    #[inline]
    pub fn next_frame(&mut self) -> (f32, f32) {
        let wl = self.rng.next_f32();
        let wr = self.rng.next_f32();
        self.last_l = (self.last_l + wl * 0.05) * 0.985;
        self.last_r = (self.last_r + wr * 0.05) * 0.985;
        let fl = self.lp_l.process(self.last_l) * 0.85;
        let fr = self.lp_r.process(self.last_r) * 0.85;
        (fl, fr)
    }
}

pub struct PinkNoiseGen {
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
    pub fn new() -> Self {
        Self {
            rng: FastRng::new(54321),
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
    pub fn next_frame(&mut self) -> (f32, f32) {
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

pub struct RainGen {
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
    sample_rate: f32,
}

impl RainGen {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            rng: FastRng::new(98765),
            bp_hiss_l: Biquad::bandpass(sample_rate, 1200.0, 1.2),
            bp_hiss_r: Biquad::bandpass(sample_rate, 1350.0, 1.2),
            bp_rumble_l: Biquad::bandpass(sample_rate, 280.0, 1.8),
            bp_rumble_r: Biquad::bandpass(sample_rate, 310.0, 1.8),
            thunder_lp: Biquad::lowpass(sample_rate, 85.0, 1.5),
            thunder_timer: (sample_rate * 25.0) as u32,
            thunder_pos: 0,
            thunder_duration: 0,
            thunder_pan: 0.5,
            sample_rate,
        }
    }

    #[inline]
    pub fn next_frame(&mut self) -> (f32, f32) {
        let wl = self.rng.next_f32();
        let wr = self.rng.next_f32();

        let hiss_l = self.bp_hiss_l.process(wl) * 0.42;
        let hiss_r = self.bp_hiss_r.process(wr) * 0.42;
        let body_l = self.bp_rumble_l.process(wl) * 0.35;
        let body_r = self.bp_rumble_r.process(wr) * 0.35;

        let mut drops_l = 0.0f32;
        let mut drops_r = 0.0f32;
        if self.rng.next_u32() < 8000000 {
            let drop_amp = (self.rng.next_f32().abs() * 0.25).min(0.2);
            drops_l += drop_amp;
        }
        if self.rng.next_u32() < 8000000 {
            let drop_amp = (self.rng.next_f32().abs() * 0.25).min(0.2);
            drops_r += drop_amp;
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

pub struct FireplaceGen {
    rng: FastRng,
    hearth_lp: Biquad,
    hearth_phase: f32,
    pop_freq: f32,
    pop_decay: f32,
    pop_phase: f32,
    pop_pan: f32,
    spark_timer: u32,
    spark_decay: f32,
    spark_pan: f32,
    sample_rate: f32,
}

impl FireplaceGen {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            rng: FastRng::new(24680),
            hearth_lp: Biquad::lowpass(sample_rate, 140.0, 1.2),
            hearth_phase: 0.0,
            pop_freq: 0.0,
            pop_decay: 0.0,
            pop_phase: 0.0,
            pop_pan: 0.5,
            spark_timer: 0,
            spark_decay: 0.0,
            spark_pan: 0.5,
            sample_rate,
        }
    }

    #[inline]
    pub fn next_frame(&mut self) -> (f32, f32) {
        let w = self.rng.next_f32();
        self.hearth_phase += 0.4 * 2.0 * PI / self.sample_rate;
        if self.hearth_phase > 2.0 * PI {
            self.hearth_phase -= 2.0 * PI;
        }
        let breathe = 0.8 + 0.2 * self.hearth_phase.sin();
        let rumble = self.hearth_lp.process(w) * 0.45 * breathe;

        if self.pop_decay <= 0.001 {
            if self.rng.next_u32() < 12000000 {
                self.pop_freq = 450.0 + (self.rng.next_f32().abs() * 950.0);
                self.pop_decay = 0.22 + (self.rng.next_f32().abs() * 0.18);
                self.pop_phase = 0.0;
                self.pop_pan = (self.rng.next_f32() + 1.0) * 0.5;
            }
        }

        let mut pop_sample = 0.0;
        if self.pop_decay > 0.001 {
            self.pop_phase += 2.0 * PI * self.pop_freq / self.sample_rate;
            pop_sample = self.pop_phase.sin() * self.pop_decay;
            self.pop_decay *= 0.991;
        }

        if self.spark_timer == 0 {
            if self.rng.next_u32() < 4000000 {
                self.spark_timer = (0.035 * self.sample_rate) as u32;
                self.spark_decay = 0.12 + (self.rng.next_f32().abs() * 0.12);
                self.spark_pan = (self.rng.next_f32() + 1.0) * 0.5;
            }
        }

        let mut spark_sample = 0.0;
        if self.spark_timer > 0 {
            self.spark_timer -= 1;
            spark_sample = self.rng.next_f32() * self.spark_decay;
            self.spark_decay *= 0.996;
        }

        let l = rumble * 0.9
            + (pop_sample * (1.0 - self.pop_pan * 0.5))
            + (spark_sample * (1.0 - self.spark_pan * 0.5));
        let r = rumble * 0.9
            + (pop_sample * (0.5 + self.pop_pan * 0.5))
            + (spark_sample * (0.5 + self.spark_pan * 0.5));
        (l, r)
    }
}

pub struct LofiGen {
    rng: FastRng,
    vinyl_phase: f32,
    flutter_phase: f32,
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
    pub fn new(sample_rate: f32) -> Self {
        let chord_dur = (9.0 * sample_rate) as u32;
        Self {
            rng: FastRng::new(13579),
            vinyl_phase: 0.0,
            flutter_phase: 0.0,
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
        match idx % 4 {
            0 => [130.81, 164.81, 196.00, 246.94, 293.66],
            1 => [110.00, 130.81, 164.81, 196.00, 246.94],
            2 => [146.83, 174.61, 220.00, 261.63, 329.63],
            _ => [98.00, 146.83, 174.61, 246.94, 329.63],
        }
    }

    #[inline]
    pub fn next_frame(&mut self) -> (f32, f32) {
        self.vinyl_phase += 0.55 * 2.0 * PI / self.sample_rate;
        if self.vinyl_phase > 2.0 * PI {
            self.vinyl_phase -= 2.0 * PI;
        }
        let surface = self.rng.next_f32() * (0.018 + 0.005 * self.vinyl_phase.sin());
        let mut click = 0.0;
        if self.rng.next_u32() < 1200000 {
            click = self.rng.next_f32() * 0.06;
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
            self.chord_idx = (self.chord_idx + 1) % 4;
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

        let mixed_music = cur_chord * fade_in + prev_chord * fade_out;
        let l = self.lp_l.process(mixed_music) + surface + click;
        let r = self.lp_r.process(mixed_music) + surface - click;
        (l, r)
    }
}

pub struct BinauralPadGen {
    phase_l: f32,
    phase_r: f32,
    phase_sub: f32,
    phase_harm: f32,
    lfo_phase: f32,
    sample_rate: f32,
}

impl BinauralPadGen {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase_l: 0.0,
            phase_r: 0.0,
            phase_sub: 0.0,
            phase_harm: 0.0,
            lfo_phase: 0.0,
            sample_rate,
        }
    }

    #[inline]
    pub fn next_frame(&mut self) -> (f32, f32) {
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

        self.phase_harm += 2.0 * PI * 162.0 / self.sample_rate;
        if self.phase_harm > 2.0 * PI {
            self.phase_harm -= 2.0 * PI;
        }

        let tone_l = self.phase_l.sin() * 0.16;
        let tone_r = self.phase_r.sin() * 0.16;
        let sub = self.phase_sub.sin() * 0.12;
        let harm = self.phase_harm.sin() * 0.08 * swell;

        ((tone_l + sub + harm) * swell, (tone_r + sub + harm) * swell)
    }
}

#[derive(Clone, Default)]
pub struct AmbientSynthParams {
    pub is_playing: bool,
    pub master_volume: f32,
    pub tone_softness: f32,
    pub duck_during_tts: bool,
    pub tts_is_active: bool,
    pub brown_noise_vol: f32,
    pub pink_noise_vol: f32,
    pub rain_vol: f32,
    pub fireplace_vol: f32,
    pub lofi_vol: f32,
    pub binaural_vol: f32,
}

pub struct AmbientSynth {
    brown: BrownNoiseGen,
    pink: PinkNoiseGen,
    rain: RainGen,
    fireplace: FireplaceGen,
    lofi: LofiGen,
    binaural: BinauralPadGen,
    master_lp_l: Biquad,
    master_lp_r: Biquad,
    shared_params: Arc<Mutex<AmbientSynthParams>>,
    active_gain: f32,
    duck_gain: f32,
    current_softness: f32,
    sample_rate: u32,
    channel_toggle: bool,
    last_frame: (f32, f32),
}

impl AmbientSynth {
    pub fn new(shared_params: Arc<Mutex<AmbientSynthParams>>, sample_rate: u32) -> Self {
        let sr = sample_rate as f32;
        Self {
            brown: BrownNoiseGen::new(sr),
            pink: PinkNoiseGen::new(),
            rain: RainGen::new(sr),
            fireplace: FireplaceGen::new(sr),
            lofi: LofiGen::new(sr),
            binaural: BinauralPadGen::new(sr),
            master_lp_l: Biquad::lowpass(sr, 6000.0, 0.707),
            master_lp_r: Biquad::lowpass(sr, 6000.0, 0.707),
            shared_params,
            active_gain: 0.0,
            duck_gain: 1.0,
            current_softness: 0.5,
            sample_rate,
            channel_toggle: false,
            last_frame: (0.0, 0.0),
        }
    }

    #[inline]
    fn update_master_filter(&mut self, softness: f32) {
        if (self.current_softness - softness).abs() > 0.015 {
            self.current_softness = softness;
            let cutoff = 15000.0 * (1.0 - softness * 0.90).clamp(0.08, 1.0);
            let sr = self.sample_rate as f32;
            self.master_lp_l = Biquad::lowpass(sr, cutoff, 0.707);
            self.master_lp_r = Biquad::lowpass(sr, cutoff, 0.707);
        }
    }
}

impl Iterator for AmbientSynth {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.channel_toggle {
            self.channel_toggle = false;
            return Some(self.last_frame.1);
        }

        let p = if let Ok(guard) = self.shared_params.lock() {
            guard.clone()
        } else {
            AmbientSynthParams::default()
        };

        let target_gain = if p.is_playing { 1.0 } else { 0.0 };
        let gain_step = 1.0 / (self.sample_rate as f32 * 1.5);
        if (self.active_gain - target_gain).abs() > gain_step {
            if self.active_gain < target_gain {
                self.active_gain += gain_step;
            } else {
                self.active_gain -= gain_step;
            }
        } else {
            self.active_gain = target_gain;
        }

        let target_duck = if p.duck_during_tts && p.tts_is_active {
            0.52
        } else {
            1.0
        };
        let duck_step = 1.0 / (self.sample_rate as f32 * 0.4);
        if (self.duck_gain - target_duck).abs() > duck_step {
            if self.duck_gain < target_duck {
                self.duck_gain += duck_step;
            } else {
                self.duck_gain -= duck_step;
            }
        } else {
            self.duck_gain = target_duck;
        }

        if self.active_gain < 0.0001 {
            self.channel_toggle = true;
            self.last_frame = (0.0, 0.0);
            return Some(0.0);
        }

        self.update_master_filter(p.tone_softness);

        let mut mix_l = 0.0f32;
        let mut mix_r = 0.0f32;

        if p.brown_noise_vol > 0.001 {
            let (l, r) = self.brown.next_frame();
            mix_l += l * p.brown_noise_vol;
            mix_r += r * p.brown_noise_vol;
        }
        if p.pink_noise_vol > 0.001 {
            let (l, r) = self.pink.next_frame();
            mix_l += l * p.pink_noise_vol;
            mix_r += r * p.pink_noise_vol;
        }
        if p.rain_vol > 0.001 {
            let (l, r) = self.rain.next_frame();
            mix_l += l * p.rain_vol;
            mix_r += r * p.rain_vol;
        }
        if p.fireplace_vol > 0.001 {
            let (l, r) = self.fireplace.next_frame();
            mix_l += l * p.fireplace_vol;
            mix_r += r * p.fireplace_vol;
        }
        if p.lofi_vol > 0.001 {
            let (l, r) = self.lofi.next_frame();
            mix_l += l * p.lofi_vol;
            mix_r += r * p.lofi_vol;
        }
        if p.binaural_vol > 0.001 {
            let (l, r) = self.binaural.next_frame();
            mix_l += l * p.binaural_vol;
            mix_r += r * p.binaural_vol;
        }

        let filtered_l = self.master_lp_l.process(mix_l);
        let filtered_r = self.master_lp_r.process(mix_r);

        let vol = p.master_volume * self.active_gain * self.duck_gain;
        let sat_l = (filtered_l * vol) / (1.0 + (filtered_l * vol).abs() * 0.5);
        let sat_r = (filtered_r * vol) / (1.0 + (filtered_r * vol).abs() * 0.5);

        self.last_frame = (sat_l.clamp(-0.98, 0.98), sat_r.clamp(-0.98, 0.98));
        self.channel_toggle = true;
        Some(self.last_frame.0)
    }
}

impl Source for AmbientSynth {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> std::num::NonZero<u16> {
        std::num::NonZero::new(2).unwrap()
    }

    fn sample_rate(&self) -> std::num::NonZero<u32> {
        std::num::NonZero::new(self.sample_rate).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
