use std::{
    f32::consts::PI,
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

use cozy_ui::{
    centered,
    util::{generate_arc, get_set::Operation},
    widgets::knob::knob,
};
use nih_plug::{
    editor::Editor,
    params::{smoothing::AtomicF32, Param},
    util::gain_to_db,
};
use nih_plug_egui::{
    create_egui_editor,
    egui::{
        include_image, pos2, remap_clamp, vec2, Align2, CentralPanel, Color32, FontData,
        FontDefinitions, FontFamily, FontId, Frame, Id, Rect, RichText, Rounding, Sense, Stroke,
        TopBottomPanel, Ui, Vec2, Window,
    },
};
use once_cell::sync::Lazy;

static TRANSLATE_SIN_COS: Lazy<(f32, f32)> = Lazy::new(|| (PI / 4.0).sin_cos());

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
    pre_peak_meter: Arc<(AtomicF32, AtomicF32)>,
    post_peak_meter: Arc<(AtomicF32, AtomicF32)>,
    correcting_angle: Arc<AtomicF32>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        EditorState::default(),
        |ctx, _| {
            cozy_ui::setup(ctx);
            egui_extras::install_image_loaders(ctx);

            let mut fonts = FontDefinitions::default();

            fonts.font_data.insert(
                "0x".to_string(),
                FontData::from_static(include_bytes!("../assets/0xProto-Regular.ttf")),
            );

            fonts
                .families
                .entry(nih_plug_egui::egui::FontFamily::Name("0x".into()))
                .or_default()
                .insert(0, "0x".to_string());
            ctx.set_fonts(fonts);
        },
        move |ctx, setter, state| {
            let corr_angle_debug = correcting_angle.load(Ordering::Relaxed);
            let correcting_angle = if corr_angle_debug == 0.0 {
                0.0
            } else {
                correcting_angle.load(Ordering::Relaxed)
                    + (90.0_f32.to_radians()
                        * params.correction_amount.modulated_normalized_value())
            };

            TopBottomPanel::top("menu").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let button_clicked = ui.button("ABOUT").clicked();
                    if ui.input(|input| input.modifiers.shift) {
                        state.show_debug |= button_clicked;
                    } else {
                        state.show_about |= button_clicked;
                    }
                })
            });

            TopBottomPanel::bottom("controls").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    centered(ctx, ui, |ui| {
                        ui.add(
                            knob(
                                "knob_correct_amount",
                                50.0,
                                |v| match v {
                                    Operation::Get => {
                                        params.correction_amount.unmodulated_normalized_value()
                                    }
                                    Operation::Set(v) => {
                                        setter
                                            .set_parameter_normalized(&params.correction_amount, v);
                                        v
                                    }
                                },
                                || setter.begin_set_parameter(&params.correction_amount),
                                || setter.end_set_parameter(&params.correction_amount),
                            )
                            .label("CORRECTION AMNT")
                            .default_value(params.correction_amount.default_normalized_value())
                            .modulated_value(params.correction_amount.modulated_normalized_value()),
                        );

                        ui.add(
                            knob(
                                "knob_reaction_time",
                                50.0,
                                |v| match v {
                                    Operation::Get => {
                                        params.reaction_time.unmodulated_normalized_value()
                                    }
                                    Operation::Set(v) => {
                                        setter.set_parameter_normalized(&params.reaction_time, v);
                                        v
                                    }
                                },
                                || setter.begin_set_parameter(&params.reaction_time),
                                || setter.end_set_parameter(&params.reaction_time),
                            )
                            .label("REACTION TIME")
                            .description(params.reaction_time.to_string())
                            .default_value(params.reaction_time.default_normalized_value())
                            .modulated_value(params.reaction_time.modulated_normalized_value()),
                        );
                    });
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

                        let scope_rect =
                            Rect::from_center_size(rect.center(), Vec2::splat(rect.height()))
                                .shrink(20.0);

                        let painter = ui.painter_at(rect);
                        let center = rect.center();

                        painter.line_segment(
                            [scope_rect.center_top(), scope_rect.center_bottom()],
                            Stroke::new(1.5, Color32::GRAY.gamma_multiply(0.5)),
                        );
                        painter.line_segment(
                            [scope_rect.left_center(), scope_rect.right_center()],
                            Stroke::new(1.5, Color32::GRAY.gamma_multiply(0.5)),
                        );

                        painter.line_segment(
                            [
                                scope_rect.min + (scope_rect.size() * 0.25),
                                scope_rect.max - (scope_rect.size() * 0.25),
                            ],
                            Stroke::new(1.5, Color32::GRAY.gamma_multiply(0.55)),
                        );
                        painter.line_segment(
                            [
                                scope_rect.min + (scope_rect.size() * vec2(0.75, 0.25)),
                                scope_rect.max - (scope_rect.size() * vec2(0.75, 0.25)),
                            ],
                            Stroke::new(1.5, Color32::GRAY.gamma_multiply(0.55)),
                        );

                        painter.line_segment(
                            [scope_rect.center_top(), scope_rect.left_center()],
                            Stroke::new(1.5, Color32::GRAY),
                        );
                        painter.line_segment(
                            [scope_rect.left_center(), scope_rect.center_bottom()],
                            Stroke::new(1.5, Color32::GRAY),
                        );
                        painter.line_segment(
                            [scope_rect.center_bottom(), scope_rect.right_center()],
                            Stroke::new(1.5, Color32::GRAY),
                        );
                        painter.line_segment(
                            [scope_rect.right_center(), scope_rect.center_top()],
                            Stroke::new(1.5, Color32::GRAY),
                        );

                        let (translate_sin, translate_cos) = *TRANSLATE_SIN_COS;

                        for (left, right) in stereo_data.iter().map(|(left, right)| {
                            (
                                left.load(std::sync::atomic::Ordering::Relaxed)
                                    .clamp(-1.0, 1.0),
                                right
                                    .load(std::sync::atomic::Ordering::Relaxed)
                                    .clamp(-1.0, 1.0),
                            )
                        }) {
                            let dot_x = left * translate_cos - right * translate_sin;
                            let dot_y = left * translate_sin + right * translate_cos;
                            let offset = vec2(
                                dot_x * scope_rect.width() / PI,
                                dot_y * scope_rect.height() / PI,
                            );

                            painter.circle_filled(
                                center + offset,
                                1.5,
                                Color32::WHITE.gamma_multiply((left.abs() + right.abs()) / 2.0),
                            );
                        }

                        generate_arc(
                            &painter,
                            center,
                            scope_rect.height() / 4.0,
                            90.0_f32.to_radians() - correcting_angle,
                            90.0_f32.to_radians(),
                            Stroke::new(2.5, cozy_ui::colors::HIGHLIGHT_COL32),
                        );

                        let peak_rect_pre = Rect::from_center_size(
                            pos2(rect.left() + (rect.width() * 0.1), rect.center().y),
                            vec2(40.0, rect.height() * 0.8),
                        );
                        draw_peak_meters(
                            ui,
                            peak_rect_pre,
                            gain_to_db(pre_peak_meter.0.load(std::sync::atomic::Ordering::Relaxed)),
                            gain_to_db(pre_peak_meter.1.load(std::sync::atomic::Ordering::Relaxed)),
                            Duration::from_millis(300),
                        );
                        ui.painter().text(
                            peak_rect_pre.center_bottom() + vec2(0.0, 10.0),
                            Align2::CENTER_CENTER,
                            "PRE",
                            FontId::new(10.0, FontFamily::Name("0x".into())),
                            Color32::GRAY,
                        );
                        let peak_rect_post = Rect::from_center_size(
                            pos2(rect.left() + (rect.width() * 0.9), rect.center().y),
                            vec2(40.0, rect.height() * 0.8),
                        );
                        draw_peak_meters(
                            ui,
                            peak_rect_post,
                            gain_to_db(
                                post_peak_meter.0.load(std::sync::atomic::Ordering::Relaxed),
                            ),
                            gain_to_db(
                                post_peak_meter.1.load(std::sync::atomic::Ordering::Relaxed),
                            ),
                            Duration::from_millis(300),
                        );
                        ui.painter().text(
                            peak_rect_post.center_bottom() + vec2(0.0, 10.0),
                            Align2::CENTER_CENTER,
                            "POST",
                            FontId::new(10.0, FontFamily::Name("0x".into())),
                            Color32::GRAY,
                        );
                    });
            });

            Window::new("DEBUG")
                .vscroll(true)
                .open(&mut state.show_debug)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "pan angle: {} ({} rad pre-offset)",
                        correcting_angle.to_degrees(),
                        corr_angle_debug
                    ));
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

fn draw_peak_meters(
    ui: &Ui,
    bounds: Rect,
    level_l_dbfs: f32,
    level_r_dbfs: f32,
    hold_time: Duration,
) {
    const MIN_DB: f32 = -90.0;
    const MAX_DB: f32 = 2.0;

    let level_l_dbfs = level_l_dbfs.min(MAX_DB);
    let level_r_dbfs = level_r_dbfs.min(MAX_DB);

    let held_l_id = Id::new(format!("peak_meter_{bounds:?}_peak_l"));
    let held_r_id = Id::new(format!("peak_meter_{bounds:?}_peak_r"));
    let last_held_l_id = Id::new(format!("peak_meter_{bounds:?}_last_peak_l"));
    let last_held_r_id = Id::new(format!("peak_meter_{bounds:?}_last_peak_r"));

    let held_peak_value_db_l = ui.memory_mut(|r| *r.data.get_temp_mut_or(held_l_id, f32::MIN));
    let held_peak_value_db_r = ui.memory_mut(|r| *r.data.get_temp_mut_or(held_r_id, f32::MIN));

    let last_held_peak_value_l = ui.memory_mut(|r| r.data.get_temp(last_held_l_id));
    let last_held_peak_value_r = ui.memory_mut(|r| r.data.get_temp(last_held_r_id));

    let now = Instant::now();
    let time_logic = |now: Instant,
                      level: f32,
                      peak_level: f32,
                      peak_time: Option<Instant>,
                      peak_id,
                      last_held_id| {
        let mut peak_level = peak_level;

        if level > peak_level || peak_time.is_none() {
            peak_level = level;
            ui.memory_mut(|r| r.data.insert_temp(last_held_id, now));
        }

        if let Some(peak_time) = peak_time {
            if now > peak_time + hold_time && peak_level > level {
                let normalized = remap_clamp(peak_level, MIN_DB..=MAX_DB, 0.0..=1.0);
                let step = normalized * 0.992;
                peak_level = remap_clamp(step, 0.0..=1.0, MIN_DB..=MAX_DB);
            }
        }

        ui.memory_mut(|r| r.data.insert_temp(peak_id, peak_level));
    };

    (time_logic)(
        now,
        level_l_dbfs,
        held_peak_value_db_l,
        last_held_peak_value_l,
        held_l_id,
        last_held_l_id,
    );
    (time_logic)(
        now,
        level_r_dbfs,
        held_peak_value_db_r,
        last_held_peak_value_r,
        held_r_id,
        last_held_r_id,
    );

    let held_peak_value_db_l = ui.memory_mut(|r| *r.data.get_temp_mut_or(held_l_id, f32::MIN));
    let held_peak_value_db_r = ui.memory_mut(|r| *r.data.get_temp_mut_or(held_r_id, f32::MIN));

    let peak_width = (bounds.width() - 10.0) / 2.0;

    let (l_bounds, temp) = bounds.split_left_right_at_x(bounds.left() + peak_width);
    let (_, r_bounds) = temp.split_left_right_at_x(temp.left() + 10.0);

    ui.painter().rect_filled(
        Rect::from_two_pos(
            l_bounds.left_bottom(),
            pos2(
                l_bounds.right(),
                remap_clamp(level_l_dbfs, MIN_DB..=MAX_DB, l_bounds.bottom_up_range()),
            ),
        ),
        Rounding::ZERO,
        Color32::GRAY,
    );
    ui.painter().hline(
        l_bounds.x_range(),
        remap_clamp(
            held_peak_value_db_l,
            MIN_DB..=MAX_DB,
            l_bounds.bottom_up_range(),
        ),
        Stroke::new(1.0, Color32::GRAY),
    );
    ui.painter().rect_filled(
        Rect::from_two_pos(
            r_bounds.left_bottom(),
            pos2(
                r_bounds.right(),
                remap_clamp(level_r_dbfs, MIN_DB..=MAX_DB, r_bounds.bottom_up_range()),
            ),
        ),
        Rounding::ZERO,
        Color32::GRAY,
    );
    ui.painter().hline(
        r_bounds.x_range(),
        remap_clamp(
            held_peak_value_db_r,
            MIN_DB..=MAX_DB,
            r_bounds.bottom_up_range(),
        ),
        Stroke::new(1.0, Color32::GRAY),
    );
}
