use nih_plug::prelude::*;
use std::f32::consts::PI;
use std::sync::Arc;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct Centered {
    params: Arc<CenteredParams>,
}

#[derive(Params)]
struct CenteredParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for Centered {
    fn default() -> Self {
        Self {
            params: Arc::new(CenteredParams::default()),
        }
    }
}

impl Default for CenteredParams {
    fn default() -> Self {
        Self {
            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            gain: FloatParam::new(
                "Gain",
                0.0,
                FloatRange::Linear { min: -180.0, max: 180.0 },
            )
            // Because the gain parameter is stored as linear gain instead of storing the value as
            // decibels, we need logarithmic smoothing
            .with_step_size(0.1),
        }
    }
}

impl Plugin for Centered {
    const NAME: &'static str = "Centered";
    const VENDOR: &'static str = "cozy dsp";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "hi@cozydsp.space";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let t = |x: f32, y: f32| {
            ((y/x).atan() * 180.0) / PI
        };

        let pan_deg = (-45.0 - buffer.iter_samples().map(|mut s| {
            t(*s.get_mut(0).unwrap(), *s.get_mut(1).unwrap())
        })
        .zip((1..))
        .fold(0., |acc, (i, d)| (i + acc * (d - 1) as f32) / d as f32))
        .to_radians();

        for mut channel_samples in buffer.iter_samples() {
            let left = *channel_samples.get_mut(0).unwrap();
            let right = *channel_samples.get_mut(1).unwrap();
            *channel_samples.get_mut(0).unwrap() = (left * pan_deg.cos()) - (right * pan_deg.sin());
            *channel_samples.get_mut(1).unwrap() = (left * -pan_deg.sin()) - (right * pan_deg.cos());
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Centered {
    const CLAP_ID: &'static str = "space.cozydsp.centered";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("an attempt at recentering stereo signals");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for Centered {
    const VST3_CLASS_ID: [u8; 16] = *b"cozydspcentered!";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(Centered);
nih_export_vst3!(Centered);