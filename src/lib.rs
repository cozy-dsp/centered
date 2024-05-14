use editor::editor;
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::{atomic::Ordering, Arc};

mod editor;

pub const GONIO_NUM_SAMPLES: usize = 1000;
const PEAK_METER_DECAY_MS: f64 = 150.0;

pub struct Centered {
    params: Arc<CenteredParams>,
    stereo_data: Arc<[(AtomicF32, AtomicF32); GONIO_NUM_SAMPLES]>,
    stereo_data_idx: usize,
    pre_peak_meter: Arc<(AtomicF32, AtomicF32)>,
    post_peak_meter: Arc<(AtomicF32, AtomicF32)>,
    peak_meter_decay_weight: f32,
    correcting_angle: Arc<AtomicF32>,
}

#[derive(Params)]
struct CenteredParams {
    /// The amount to correct the input by, represented as a percent
    #[id = "correction-amount"]
    pub correction_amount: FloatParam,

    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
}

impl Default for Centered {
    fn default() -> Self {
        Self {
            params: Arc::new(CenteredParams::default()),
            // evil hack because AtomicF32 doesn't implement copy
            stereo_data: Arc::new([0; GONIO_NUM_SAMPLES].map(|_| Default::default())),
            pre_peak_meter: Arc::new(Default::default()),
            post_peak_meter: Arc::new(Default::default()),
            peak_meter_decay_weight: 0.0,
            stereo_data_idx: 0,
            correcting_angle: Arc::default(),
        }
    }
}

impl Default for CenteredParams {
    fn default() -> Self {
        Self {
            correction_amount: FloatParam::new(
                "Correction Amount",
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit("%")
            .with_step_size(0.1),

            editor_state: EguiState::from_size(600, 480),
        }
    }
}

impl Plugin for Centered {
    const NAME: &'static str = "Centered";
    const VENDOR: &'static str = "cozy dsp";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "hi@cozydsp.space";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn initialize(
            &mut self,
            audio_io_layout: &AudioIOLayout,
            buffer_config: &BufferConfig,
            context: &mut impl InitContext<Self>,
        ) -> bool {
            self.peak_meter_decay_weight = 0.25f64
            .powf((buffer_config.sample_rate as f64 * PEAK_METER_DECAY_MS / 1000.).recip())
            as f32;

        true
    }

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor(
            self.params.clone(),
            self.stereo_data.clone(),
            self.pre_peak_meter.clone(),
            self.post_peak_meter.clone(),
            self.correcting_angle.clone(),
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if self.params.editor_state.is_open() {
            for mut channel_samples in buffer.iter_samples() {
                let channel_left = *channel_samples.get_mut(0).unwrap();
                let channel_right = *channel_samples.get_mut(1).unwrap();

                let (left, right) = &self.stereo_data[self.stereo_data_idx];
                left.store(
                    channel_left,
                    std::sync::atomic::Ordering::Relaxed,
                );
                right.store(
                    channel_right,
                    std::sync::atomic::Ordering::Relaxed,
                );

                self.stereo_data_idx += 1;
                self.stereo_data_idx %= GONIO_NUM_SAMPLES - 1;
            }

            calc_peak(buffer, [&self.pre_peak_meter.0, &self.pre_peak_meter.1], self.peak_meter_decay_weight);
        }

        let t = |x: f32, y: f32| (y.abs() / x.abs()).atan().to_degrees();

        #[allow(clippy::cast_precision_loss)]
        let pan_deg = (-45.0
            - buffer
                .iter_samples()
                .map(|mut s| t(*s.get_mut(0).unwrap(), *s.get_mut(1).unwrap()))
                .filter(|s| !s.is_nan())
                .zip(1..)
                .fold(0.0_f32, |acc, (i, d)| {
                    // this never approaches 2^23 so it doesn't matter
                    acc.mul_add((d - 1) as f32, i) / d as f32
                }))
        .to_radians()
            * self.params.correction_amount.modulated_normalized_value();
        self.correcting_angle
            .store(pan_deg, std::sync::atomic::Ordering::Relaxed);

        for mut channel_samples in buffer.iter_samples() {
            let left = *channel_samples.get_mut(0).unwrap();
            let right = *channel_samples.get_mut(1).unwrap();
            let (pan_sin, pan_cos) = pan_deg.sin_cos();
            *channel_samples.get_mut(0).unwrap() = left.mul_add(pan_cos, -(right * pan_sin));
            *channel_samples.get_mut(1).unwrap() = left.mul_add(-pan_sin, -(right * pan_cos));
        }

        calc_peak(buffer, [&self.post_peak_meter.0, &self.post_peak_meter.1], self.peak_meter_decay_weight);

        ProcessStatus::Normal
    }
}

fn calc_peak(buffer: &mut Buffer, peak: [&AtomicF32; 2], decay_weight: f32) {
    for mut channel_samples in buffer.iter_samples() {
        for (sample, peak) in channel_samples.iter_mut().zip(peak.iter()) {
            let amp = sample.abs();
            let current_peak = peak.load(Ordering::Relaxed);
            let new_peak = if amp > current_peak {
                amp
            } else {
                current_peak * decay_weight + amp * (1. - decay_weight)
            };

            peak.store(new_peak, Ordering::Relaxed);
        }
    }
}

impl ClapPlugin for Centered {
    const CLAP_ID: &'static str = "space.cozydsp.centered";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("an attempt at recentering stereo signals");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mixing,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for Centered {
    const VST3_CLASS_ID: [u8; 16] = *b"cozydspcentered!";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Stereo,
        Vst3SubCategory::Spatial,
    ];
}

nih_export_clap!(Centered);
nih_export_vst3!(Centered);
