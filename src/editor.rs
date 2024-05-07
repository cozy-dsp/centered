use std::{
    f32::consts::{PI, TAU},
    sync::Arc,
};

use cozy_ui::{
    util::{generate_arc, get_set::Operation},
    widgets::knob::knob,
};
use nih_plug::{
    editor::Editor,
    params::{smoothing::AtomicF32, Param},
};
use nih_plug_egui::{
    create_egui_editor,
    egui::{
        include_image, CentralPanel, Color32, Frame, RichText, Sense, Stroke, TopBottomPanel, Vec2,
        Window,
    },
};

const DEG_45: f32 = 45.0_f32 * (PI / 180.0_f32);
const DEG_90: f32 = 90.0_f32 * (PI / 180.0_f32);
const DEG_270: f32 = 270.0_f32 * (PI / 180.0_f32);

use crate::{CenteredParams, GONIO_NUM_SAMPLES};

#[derive(Default)]
struct EditorState {
    show_debug: bool,
    show_about: bool,
}

// shut up clippy this is an arc
#[allow(clippy::needless_pass_by_value)]
pub fn editor(
    params: Arc<CenteredParams>,
    stereo_data: Arc<[(AtomicF32, AtomicF32); GONIO_NUM_SAMPLES]>,
    correcting_angle: Arc<AtomicF32>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        EditorState::default(),
        |ctx, _| {
            cozy_ui::setup(ctx);
            egui_extras::install_image_loaders(ctx);
        },
        move |ctx, setter, state| {
            let correcting_angle = (correcting_angle.load(std::sync::atomic::Ordering::Relaxed))
                + 45.0_f32.to_radians() * params.correction_amount.modulated_normalized_value();
            TopBottomPanel::top("menu").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("ABOUT").clicked() {
                        if ui.input(|input| input.modifiers.shift) {
                            state.show_debug = !state.show_debug;
                        } else {
                            state.show_about = !state.show_about;
                        }
                    }
                })
            });
            TopBottomPanel::bottom("controls").show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.group(|ui| {
                            ui.add(
                                knob(
                                    "knob_correct_amount",
                                    50.0,
                                    |v| match v {
                                        Operation::Get => {
                                            params.correction_amount.unmodulated_normalized_value()
                                        }
                                        Operation::Set(v) => {
                                            setter.set_parameter_normalized(
                                                &params.correction_amount,
                                                v,
                                            );
                                            v
                                        }
                                    },
                                    || setter.begin_set_parameter(&params.correction_amount),
                                    || setter.end_set_parameter(&params.correction_amount),
                                )
                                .label(RichText::new(params.correction_amount.name()).strong())
                                .default_value(params.correction_amount.default_normalized_value())
                                .modulated_value(
                                    params.correction_amount.modulated_normalized_value(),
                                ),
                            );
                        });
                    })
                })
            });
            CentralPanel::default().show(ctx, |ui| {
                Frame::canvas(ui.style())
                    .stroke(Stroke::new(2.0, Color32::DARK_GRAY))
                    .show(ui, |ui| {
                        let (rect, _) = ui.allocate_at_least(
                            ui.available_size_before_wrap(),
                            Sense::focusable_noninteractive(),
                        );
                        let painter = ui.painter_at(rect);
                        let center = rect.center();

                        for (left, right) in stereo_data.iter() {
                            // treating left and right as the x and y, perhaps swapping these would yield some results?
                            let y = left.load(std::sync::atomic::Ordering::Relaxed);
                            let x = right.load(std::sync::atomic::Ordering::Relaxed);

                            // pythagorus rolling in his tomb, the ln is natual log, the data looks like a nifty flower if you do this
                            let radius = x.hypot(y).ln();
                            let mut angle = (y / x).atan();

                            match (x, y) {
                                // y != 0.0 doesn't produce correct results for some reason. floats!
                                #[allow(clippy::double_comparisons)]
                                (x, y) if (y < 0.0 || y > 0.0) && x < 0.0 => {
                                    angle += PI;
                                }
                                (x, y) if x > 0.0 && y < 0.0 => {
                                    angle += TAU;
                                }
                                _ => {}
                            }

                            if x == 0.0 {
                                angle = if y > 0.0 { DEG_90 } else { DEG_270 }
                            } else if y == 0.0 {
                                angle = if x > 0.0 { 0.0 } else { PI }
                            }

                            angle += DEG_45;

                            let (sin, cos) = angle.sin_cos();

                            let offset = Vec2::new(radius * cos, radius * sin) * 10.0;

                            painter.circle_filled(center + offset, 1.5, Color32::RED);

                            generate_arc(
                                &painter,
                                center,
                                100.0,
                                90.0_f32.to_radians(),
                                90.0_f32.to_radians() + correcting_angle,
                                Stroke::new(2.5, Color32::GREEN),
                            )
                        }
                    });
            });

            Window::new("DEBUG")
                .vscroll(true)
                .open(&mut state.show_debug)
                .show(ctx, |ui| {
                    ui.label(format!("pan angle: {}", correcting_angle.to_degrees()));
                });

            Window::new("ABOUT")
                .vscroll(true)
                .open(&mut state.show_about)
                .show(ctx, |ui| {
                    ui.image(include_image!("../assets/Cozy_logo.png"));
                    ui.vertical_centered(|ui| {
                        ui.heading(RichText::new("CENTERED").strong());
                        ui.label(
                            RichText::new(format!("Version {}", env!("VERGEN_GIT_DESCRIBE")))
                                .italics(),
                        );
                        ui.hyperlink_to("Homepage", env!("CARGO_PKG_HOMEPAGE"));
                        ui.separator();
                        ui.heading(RichText::new("Credits"));
                        ui.label("Plugin by joe sorensen");
                        ui.label("cozy dsp branding and design by gordo");
                    });
                });
        },
    )
}
