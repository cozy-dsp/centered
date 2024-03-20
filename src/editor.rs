use std::{
    f32::consts::{PI, TAU},
    sync::Arc,
};

use cozy_ui::{util::CIRCLE_POINTS, widgets::knob::knob};
use map_range::MapRange;
use nih_plug::{
    editor::Editor,
    params::{smoothing::AtomicF32, Param},
};
use nih_plug_egui::{
    create_egui_editor,
    egui::{epaint::{CubicBezierShape, PathShape}, pos2, CentralPanel, Color32, Painter, RichText, Shape, Stroke, TopBottomPanel, Vec2},
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
        |ctx, ()| {
        },
        move |ctx, setter, ()| {
            let correcting_angle = (correcting_angle
            .load(std::sync::atomic::Ordering::Relaxed)
            .to_degrees() + 90.0) * -1.0;
            TopBottomPanel::top("menu").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "pan angle: {}",
                        180.0 + correcting_angle
                    ));
                })
            });
            TopBottomPanel::bottom("controls").show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.group(|ui| {
                            knob(
                                ui,
                                "knob_correct_amount",
                                25.0,
                                |v| match v {
                                    Some(v) => {
                                        setter
                                            .set_parameter_normalized(&params.correction_amount, v);
                                        v
                                    }
                                    None => params.correction_amount.modulated_normalized_value(),
                                },
                                || setter.begin_set_parameter(&params.correction_amount),
                                || setter.end_set_parameter(&params.correction_amount),
                                params.correction_amount.default_normalized_value(),
                            );
                            ui.label(RichText::new(params.correction_amount.name()).strong());
                        });
                    })
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

                    let center = kurbo::Point {
                        x: center.x as f64,
                        y: center.y as f64
                    };

                    let start = center + kurbo::Vec2 {
                        x: 100.0 * 180.0_f64.to_radians().cos(),
                        y: 100.0 * 180.0_f64.to_radians().sin() 
                    };

                    let p = kurbo::Arc {
                        center,
                        radii: kurbo::Vec2 { x: 100.0, y: 100.0 },
                        start_angle: 180.0_f64.to_radians(),
                        sweep_angle: correcting_angle.to_radians() as f64,
                        x_rotation: 0.0
                    };

                    let mut p_start = pos2(start.x as f32, start.y as f32);

                    p.to_cubic_beziers(0.01, |x, y, z| {
                        if x.is_nan() {
                            return;
                        }

                        let p1 = pos2(x.x as f32, x.y as f32);
                        let p2 = pos2(y.x as f32, y.y as f32);
                        let p3 = pos2(z.x as f32, z.y as f32);

                        painter.add(Shape::CubicBezier(CubicBezierShape {
                            points: [p_start, p1, p2, p3],
                            closed: false,
                            fill: Color32::TRANSPARENT,
                            stroke: Stroke::new(2.5, Color32::GREEN)
                        }));

                        p_start = p3;
                    })

                }
                ui.expand_to_include_rect(painter.clip_rect());
            });
        },
    )
}
