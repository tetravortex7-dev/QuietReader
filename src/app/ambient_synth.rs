use rodio::Source;
use std::f32::consts::PI;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

mod dsp;
mod layers;

use dsp::{DcBlocker, Limiter, session_seed, smooth_toward};
use layers::{BinauralPadGen, BrownNoiseGen, FireplaceGen, LofiGen, PinkNoiseGen, RainGen};

#[derive(Clone, Copy, Default)]
pub struct AmbientSynthParams {
    pub is_playing: bool,
    pub master_volume: f32,
    pub tone_softness: f32,
    pub duck_during_tts: bool,
    pub duck_amount: f32,
    pub tts_is_active: bool,
    pub brown_noise_vol: f32,
    pub pink_noise_vol: f32,
    pub rain_vol: f32,
    pub fireplace_vol: f32,
    pub lofi_vol: f32,
    pub binaural_vol: f32,
}

pub struct AmbientSharedParams {
    is_playing: AtomicBool,
    master_volume: AtomicU32,
    tone_softness: AtomicU32,
    duck_during_tts: AtomicBool,
    duck_amount: AtomicU32,
    tts_is_active: AtomicBool,
    brown_noise_vol: AtomicU32,
    pink_noise_vol: AtomicU32,
    rain_vol: AtomicU32,
    fireplace_vol: AtomicU32,
    lofi_vol: AtomicU32,
    binaural_vol: AtomicU32,
}

impl AmbientSharedParams {
    pub fn new(params: AmbientSynthParams) -> Self {
        Self {
            is_playing: AtomicBool::new(params.is_playing),
            master_volume: atomic_f32(params.master_volume),
            tone_softness: atomic_f32(params.tone_softness),
            duck_during_tts: AtomicBool::new(params.duck_during_tts),
            duck_amount: atomic_f32(params.duck_amount),
            tts_is_active: AtomicBool::new(params.tts_is_active),
            brown_noise_vol: atomic_f32(params.brown_noise_vol),
            pink_noise_vol: atomic_f32(params.pink_noise_vol),
            rain_vol: atomic_f32(params.rain_vol),
            fireplace_vol: atomic_f32(params.fireplace_vol),
            lofi_vol: atomic_f32(params.lofi_vol),
            binaural_vol: atomic_f32(params.binaural_vol),
        }
    }

    pub fn store(&self, params: AmbientSynthParams) {
        self.is_playing.store(params.is_playing, Ordering::Relaxed);
        self.master_volume
            .store(params.master_volume.to_bits(), Ordering::Relaxed);
        self.tone_softness
            .store(params.tone_softness.to_bits(), Ordering::Relaxed);
        self.duck_during_tts
            .store(params.duck_during_tts, Ordering::Relaxed);
        self.duck_amount
            .store(params.duck_amount.to_bits(), Ordering::Relaxed);
        self.brown_noise_vol
            .store(params.brown_noise_vol.to_bits(), Ordering::Relaxed);
        self.pink_noise_vol
            .store(params.pink_noise_vol.to_bits(), Ordering::Relaxed);
        self.rain_vol
            .store(params.rain_vol.to_bits(), Ordering::Relaxed);
        self.fireplace_vol
            .store(params.fireplace_vol.to_bits(), Ordering::Relaxed);
        self.lofi_vol
            .store(params.lofi_vol.to_bits(), Ordering::Relaxed);
        self.binaural_vol
            .store(params.binaural_vol.to_bits(), Ordering::Relaxed);
    }

    pub fn set_tts_active(&self, active: bool) {
        self.tts_is_active.store(active, Ordering::Relaxed);
    }

    pub fn tts_is_active(&self) -> bool {
        self.tts_is_active.load(Ordering::Relaxed)
    }

    pub(crate) fn snapshot(&self) -> AmbientSynthParams {
        AmbientSynthParams {
            is_playing: self.is_playing.load(Ordering::Relaxed),
            master_volume: load_f32(&self.master_volume),
            tone_softness: load_f32(&self.tone_softness),
            duck_during_tts: self.duck_during_tts.load(Ordering::Relaxed),
            duck_amount: load_f32(&self.duck_amount),
            tts_is_active: self.tts_is_active.load(Ordering::Relaxed),
            brown_noise_vol: load_f32(&self.brown_noise_vol),
            pink_noise_vol: load_f32(&self.pink_noise_vol),
            rain_vol: load_f32(&self.rain_vol),
            fireplace_vol: load_f32(&self.fireplace_vol),
            lofi_vol: load_f32(&self.lofi_vol),
            binaural_vol: load_f32(&self.binaural_vol),
        }
    }
}

fn atomic_f32(value: f32) -> AtomicU32 {
    AtomicU32::new(value.to_bits())
}

fn load_f32(value: &AtomicU32) -> f32 {
    f32::from_bits(value.load(Ordering::Relaxed))
}

pub struct AmbientSynth {
    brown: BrownNoiseGen,
    pink: PinkNoiseGen,
    rain: RainGen,
    fireplace: FireplaceGen,
    lofi: LofiGen,
    binaural: BinauralPadGen,
    master_lp_l: dsp::Biquad,
    master_lp_r: dsp::Biquad,
    dc_l: DcBlocker,
    dc_r: DcBlocker,
    limiter: Limiter,
    shared_params: Arc<AmbientSharedParams>,
    params: AmbientSynthParams,
    param_countdown: u32,
    active_gain: f32,
    duck_gain: f32,
    master_gain: f32,
    layer_gains: [f32; 6],
    current_softness: f32,
    filter_softness: f32,
    sample_rate: u32,
    channel_toggle: bool,
    last_frame: (f32, f32),
}

impl AmbientSynth {
    pub fn new(shared_params: Arc<AmbientSharedParams>, sample_rate: u32) -> Self {
        Self::new_with_seed(shared_params, sample_rate, session_seed(0))
    }

    pub fn new_with_seed(
        shared_params: Arc<AmbientSharedParams>,
        sample_rate: u32,
        seed: u32,
    ) -> Self {
        let sample_rate = sample_rate.max(1);
        let sr = sample_rate as f32;
        let params = shared_params.snapshot();
        let initial_cutoff = 15000.0 * (1.0 - params.tone_softness * 0.90).clamp(0.08, 1.0);
        Self {
            brown: BrownNoiseGen::new(sr, seed ^ 0x1234_5678),
            pink: PinkNoiseGen::new(seed ^ 0x2345_6789),
            rain: RainGen::new(sr, seed ^ 0x3456_789A),
            fireplace: FireplaceGen::new(sr, seed ^ 0x4567_89AB),
            lofi: LofiGen::new(sr, seed ^ 0x5678_9ABC),
            binaural: BinauralPadGen::new(sr),
            master_lp_l: dsp::Biquad::lowpass(sr, initial_cutoff, 0.707),
            master_lp_r: dsp::Biquad::lowpass(sr, initial_cutoff, 0.707),
            dc_l: DcBlocker::default(),
            dc_r: DcBlocker::default(),
            limiter: Limiter::new(sr),
            shared_params,
            params,
            param_countdown: 0,
            active_gain: 0.0,
            duck_gain: 1.0,
            master_gain: params.master_volume,
            layer_gains: [
                params.brown_noise_vol,
                params.pink_noise_vol,
                params.rain_vol,
                params.fireplace_vol,
                params.lofi_vol,
                params.binaural_vol,
            ],
            current_softness: params.tone_softness,
            filter_softness: params.tone_softness,
            sample_rate,
            channel_toggle: false,
            last_frame: (0.0, 0.0),
        }
    }

    #[inline]
    fn update_master_filter(&mut self) {
        let sr = self.sample_rate as f32;
        smooth_toward(
            &mut self.current_softness,
            self.params.tone_softness,
            1.0 / (sr * 0.1),
        );
        if (self.current_softness - self.filter_softness).abs() > 0.01 {
            self.filter_softness = self.current_softness;
            let cutoff = 15000.0 * (1.0 - self.current_softness * 0.90).clamp(0.08, 1.0);
            self.master_lp_l.set_lowpass(sr, cutoff, 0.707);
            self.master_lp_r.set_lowpass(sr, cutoff, 0.707);
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

        if self.param_countdown == 0 {
            self.params = self.shared_params.snapshot();
            self.param_countdown = 32;
        } else {
            self.param_countdown -= 1;
        }

        let p = self.params;
        let sr = self.sample_rate as f32;

        let target_gain = if p.is_playing { 1.0 } else { 0.0 };
        let gain_step = 1.0 / (sr * 1.5);
        smooth_toward(&mut self.active_gain, target_gain, gain_step);

        let target_duck = if p.duck_during_tts && p.tts_is_active {
            p.duck_amount.clamp(0.0, 1.0)
        } else {
            1.0
        };
        smooth_toward(&mut self.duck_gain, target_duck, 1.0 / (sr * 0.4));
        smooth_toward(&mut self.master_gain, p.master_volume, 1.0 / (sr * 0.03));

        let targets = [
            p.brown_noise_vol,
            p.pink_noise_vol,
            p.rain_vol,
            p.fireplace_vol,
            p.lofi_vol,
            p.binaural_vol,
        ];
        for (gain, target) in self.layer_gains.iter_mut().zip(targets) {
            smooth_toward(gain, target, 1.0 / (sr * 0.02));
        }

        if self.active_gain < 0.0001 {
            self.channel_toggle = true;
            self.last_frame = (0.0, 0.0);
            return Some(0.0);
        }

        self.update_master_filter();

        let mut mix_l = 0.0f32;
        let mut mix_r = 0.0f32;

        if self.layer_gains[0] > 0.0001 {
            let (l, r) = self.brown.next_frame();
            mix_l += l * self.layer_gains[0];
            mix_r += r * self.layer_gains[0];
        }
        if self.layer_gains[1] > 0.0001 {
            let (l, r) = self.pink.next_frame();
            mix_l += l * self.layer_gains[1];
            mix_r += r * self.layer_gains[1];
        }
        if self.layer_gains[2] > 0.0001 {
            let (l, r) = self.rain.next_frame();
            mix_l += l * self.layer_gains[2];
            mix_r += r * self.layer_gains[2];
        }
        if self.layer_gains[3] > 0.0001 {
            let (l, r) = self.fireplace.next_frame();
            mix_l += l * self.layer_gains[3];
            mix_r += r * self.layer_gains[3];
        }
        if self.layer_gains[4] > 0.0001 {
            let (l, r) = self.lofi.next_frame();
            mix_l += l * self.layer_gains[4];
            mix_r += r * self.layer_gains[4];
        }
        if self.layer_gains[5] > 0.0001 {
            let (l, r) = self.binaural.next_frame();
            mix_l += l * self.layer_gains[5];
            mix_r += r * self.layer_gains[5];
        }

        let mid = (mix_l + mix_r) * 0.5;
        let side = (mix_l - mix_r) * 0.5 * 0.88;
        mix_l = mid + side;
        mix_r = mid - side;

        let filtered_l = self.master_lp_l.process(mix_l);
        let filtered_r = self.master_lp_r.process(mix_r);
        let volume = self.master_gain * self.active_gain * self.duck_gain;

        let dc_coefficient = (1.0 - 2.0 * PI * 20.0 / sr).clamp(0.9, 0.9999);
        let dc_l = self.dc_l.process(filtered_l * volume, dc_coefficient);
        let dc_r = self.dc_r.process(filtered_r * volume, dc_coefficient);
        let (out_l, out_r) = self.limiter.process_stereo(dc_l, dc_r, 0.95);

        self.last_frame = (out_l, out_r);
        self.channel_toggle = true;
        Some(out_l)
    }
}

impl Source for AmbientSynth {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> std::num::NonZero<u16> {
        std::num::NonZeroU16::new(2).unwrap_or(std::num::NonZeroU16::MAX)
    }

    fn sample_rate(&self) -> std::num::NonZero<u32> {
        std::num::NonZeroU32::new(self.sample_rate).unwrap_or(std::num::NonZeroU32::MAX)
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
