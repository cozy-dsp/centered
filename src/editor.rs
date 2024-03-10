use std::{
    f32::consts::{PI, TAU},
    sync::Arc,
};

use nih_plug::{editor::Editor, params::smoothing::AtomicF32};
use nih_plug_egui::{
    create_egui_editor,
    egui::{CentralPanel, Color32, Painter, TopBottomPanel, Vec2},
};

const DEG_45: f32 = 45.0_f32 * (PI / 180.0_f32);
const DEG_90: f32 = 90.0_f32 * (PI / 180.0_f32);
const DEG_270: f32 = 270.0_f32 * (PI / 180.0_f32);

use crate::{CenteredParams, GONIO_NUM_SAMPLES};

// shut up clippy this is an arc
#[allow(clippy::needless_pass_by_value)]
pub fn editor(
    params: Arc<CenteredParams>,
    stereo_data: Arc<[(AtomicF32, AtomicF32); GONIO_NUM_SAMPLES]>,
    correcting_angle: Arc<AtomicF32>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |_, ()| {},
        move |ctx, _, ()| {
            TopBottomPanel::top("sex").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("my bals");
                    ui.label(format!(
                        "pan angle: {}",
                        correcting_angle
                            .load(std::sync::atomic::Ordering::Relaxed)
                            .to_degrees()
                    ));
                })
            });
            CentralPanel::default().show(ctx, |ui| {
                let painter = Painter::new(
                    ui.ctx().clone(),
                    ui.layer_id(),
                    ui.available_rect_before_wrap(),
                );
                let center = painter.clip_rect().center();

                for (left, right) in stereo_data.iter() {
                    let y = left.load(std::sync::atomic::Ordering::Relaxed);
                    let x = right.load(std::sync::atomic::Ordering::Relaxed);

                    let radius = x.hypot(y).log2();
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
                }
                ui.expand_to_include_rect(painter.clip_rect());
            });
        },
    )
}
