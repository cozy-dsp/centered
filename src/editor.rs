use std::{f32::consts::{PI, TAU}, sync::Arc};

use nih_plug::{editor::Editor, params::smoothing::AtomicF32};
use nih_plug_egui::{
    create_egui_editor,
    egui::{CentralPanel, Color32, Frame, Painter, Pos2, TopBottomPanel, Vec2},
};

use crate::{CenteredParams, GONIO_NUM_SAMPLES};

pub fn editor(
    params: Arc<CenteredParams>,
    stereo_data: Arc<[(AtomicF32, AtomicF32); GONIO_NUM_SAMPLES]>,
    correcting_angle: Arc<AtomicF32>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |_, _| {},
        move |ctx, _, _| {
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

                    let radius = (x*x + y*y).sqrt().log10();
                    let mut angle = (y/x).atan();

                    match (x, y) {
                        (x , y) if (x < 0.0 && y > 0.0) || (x < 0.0 && y < 0.0) => {
                            angle += PI;
                        }
                        (x, y) if x > 0.0 && y < 0.0 => {
                            angle += TAU;
                        }
                        _ => {}
                    }

                    if x == 0.0 {
                        angle = if y > 0.0 {90.0_f32.to_radians()} else {270.0_f32.to_radians()}
                    } else if y == 0.0 {
                        angle = if x > 0.0 { 0.0 } else { PI }
                    }

                    angle += 45.0_f32.to_radians();

                    let (sin, cos) = angle.sin_cos();

                    let offset = Vec2::new(radius * cos, radius * sin) * Vec2::splat(100.0);

                    painter.circle_filled(center + offset, 1.5, Color32::RED);
                }
                ui.expand_to_include_rect(painter.clip_rect());
            });
        },
    )
}
