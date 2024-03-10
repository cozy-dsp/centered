use std::sync::Arc;

use nih_plug::{editor::Editor, params::smoothing::AtomicF32};
use nih_plug_egui::{
    create_egui_editor,
    egui::{CentralPanel, Frame, Painter, TopBottomPanel},
};

use crate::CenteredParams;

pub fn editor(
    params: Arc<CenteredParams>,
    stereo_data: Arc<(AtomicF32, AtomicF32)>,
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

                ui.expand_to_include_rect(painter.clip_rect());
            });
        },
    )
}
