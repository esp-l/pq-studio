//! The plot window, where the chart takes most of the space and every plot
//! setting sits in sections beside it, and the small chart preview in the
//! output panel that opens it. Every change redraws the chart at once.

use crate::plotting::{
    self, DataTable, LegendPosition, LineType, MarkerShape, PlotAction, PlotState, STANDARD_COLUMNS,
};
use crate::theme;
use egui::{FontId, RichText, TextStyle, vec2};

/// Width of the settings column in the plot window.
const SETTINGS_WIDTH: f32 = 380.0;

/// The chart preview in the output panel, with the export buttons. All
/// settings are in the plot window.
pub fn panel(ui: &mut egui::Ui, state: &mut PlotState, max_height: f32) -> PlotAction {
    let Some(table) = state.table.clone() else {
        match &state.note {
            Some(note) => ui.colored_label(theme::WARNING, note),
            None => ui.label("Run a simulation with a Selected Output block to see a plot here."),
        };
        return PlotAction::None;
    };
    let mut action = PlotAction::None;
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Plot").strong());
        if ui
            .selectable_label(state.window_open, "⛶ Plot window")
            .on_hover_text("The chart with all its settings: series, styles, axes, legend and export")
            .clicked()
        {
            state.window_open = !state.window_open;
            state.window_dismissed = !state.window_open;
            state.dirty = true;
        }
        action = action_buttons(ui, state, &table);
    });
    if state.window_open {
        ui.label(
            RichText::new("The chart and all its settings are in the plot window.")
                .color(theme::TEXT_SECONDARY),
        );
        return action;
    }
    let size = vec2(ui.available_width(), (max_height - 60.0).max(120.0));
    if chart_image(ui, state, &table, size)
        .on_hover_text("Click to open the plot window with all settings")
        .clicked()
    {
        state.window_open = true;
        state.window_dismissed = false;
        state.dirty = true;
    }
    ui.label(
        RichText::new("Click the chart or ⛶ Plot window for series, styles, axes, legend and export size.")
            .small()
            .color(theme::TEXT_SECONDARY),
    );
    action
}

/// Export plot / Export table / Copy table.
fn action_buttons(ui: &mut egui::Ui, state: &PlotState, table: &DataTable) -> PlotAction {
    let mut action = PlotAction::None;
    let (w, h) = state.config.figure_size();
    let scale = state.config.export_scale();
    if ui
        .add_enabled(!state.y_cols.is_empty(), egui::Button::new("Export plot…"))
        .on_hover_text(format!(
            "Save the chart as PNG ({} × {} px) or SVG — the same drawing as on screen",
            w * scale,
            h * scale
        ))
        .clicked()
    {
        action = PlotAction::ExportPlot;
    }
    if ui
        .add_enabled(!state.y_cols.is_empty(), egui::Button::new("Export plotted data…"))
        .on_hover_text(PLOTTED_DATA_TIP)
        .clicked()
    {
        action = PlotAction::ExportData;
    }
    if ui
        .button("Export table…")
        .on_hover_text("Save every row and column of the Selected Output as a CSV file for a spreadsheet")
        .clicked()
    {
        action = PlotAction::ExportTable;
    }
    if ui
        .button("Copy table")
        .on_hover_text("Copy the whole table, tab-separated, to paste straight into a spreadsheet")
        .clicked()
    {
        ui.ctx().copy_text(plotting::table_text(table, &state.config.sim_names, '\t'));
    }
    action
}

/// The chart, as large as fits in `max` at the figure's aspect ratio. It is
/// the export drawing rendered at the scale the screen needs, so text,
/// lines and legend sit exactly where they will in the exported file.
fn chart_image(ui: &mut egui::Ui, state: &mut PlotState, table: &DataTable, max: egui::Vec2) -> egui::Response {
    if state.y_cols.is_empty() {
        return ui.label("Tick one or more Y series to plot them.");
    }
    let (fw, fh) = state.config.figure_size();
    let aspect = fw as f32 / fh as f32;
    let width = max.x.min(max.y * aspect).max(160.0);
    let size = vec2(width, width / aspect);
    let wanted = ((width * ui.ctx().pixels_per_point()) as f64 / fw as f64 * 20.0).round() / 20.0;
    let wanted = wanted.clamp(0.2, 4.0);
    let stale = state.texture.as_ref().is_none_or(|(_, scale)| (scale - wanted).abs() > 1e-6);
    if state.dirty || stale {
        state.assign_styles();
        state.texture = plotting::render_rgb(table, state.x_col, &state.y_cols, &state.config, wanted)
            .ok()
            .map(|(buffer, w, h)| {
                let image = egui::ColorImage::from_rgb([w as usize, h as usize], &buffer);
                (ui.ctx().load_texture("plot_chart", image, egui::TextureOptions::LINEAR), wanted)
            });
        state.dirty = false;
    }
    match &state.texture {
        Some((texture, _)) => ui.add(egui::Image::new((texture.id(), size)).sense(egui::Sense::click())),
        None => ui.label("The chart could not be drawn."),
    }
}

/// The plot window; call once per frame.
pub fn window(ctx: &egui::Context, state: &mut PlotState) -> PlotAction {
    let Some(table) = state.table.clone().filter(|_| state.window_open) else {
        return PlotAction::None;
    };
    let mut open = true;
    let mut action = PlotAction::None;
    let screen = ctx.content_rect();
    egui::Window::new("Plot")
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .default_rect(screen.shrink2(vec2(24.0, 30.0)))
        .min_size([720.0, 460.0])
        .show(ctx, |ui| {
            larger_controls(ui.style_mut());
            ui.horizontal(|ui| {
                action = action_buttons(ui, state, &table);
                ui.separator();
                ui.label(
                    RichText::new("Changes apply at once and are saved with the project.")
                        .color(theme::TEXT_SECONDARY),
                );
            });
            ui.add_space(4.0);
            let height = ui.available_height();
            ui.horizontal_top(|ui| {
                let chart_width = (ui.available_width() - SETTINGS_WIDTH - 20.0).max(240.0);
                ui.allocate_ui_with_layout(
                    vec2(chart_width, height),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        chart_image(ui, state, &table, vec2(chart_width, height));
                    },
                );
                ui.separator();
                ui.allocate_ui_with_layout(
                    vec2(SETTINGS_WIDTH, height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("plot_settings")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let asked = settings(ui, state, &table);
                                if asked != PlotAction::None {
                                    action = asked;
                                }
                            });
                    },
                );
            });
        });
    if !open {
        state.window_open = false;
        state.window_dismissed = true;
    }
    if state.dirty {
        ctx.request_repaint();
    }
    action
}

/// Bigger text and controls than the rest of the app, for a window that is
/// mostly about reading and tuning a figure.
fn larger_controls(style: &mut egui::Style) {
    style.spacing.item_spacing = vec2(10.0, 9.0);
    style.spacing.interact_size.y = 30.0;
    style.spacing.button_padding = vec2(12.0, 6.0);
    for (text_style, size) in [(TextStyle::Body, 15.0), (TextStyle::Button, 15.0), (TextStyle::Monospace, 14.0)] {
        style.text_styles.insert(text_style, FontId::proportional(size));
    }
    style.text_styles.insert(TextStyle::Small, FontId::proportional(13.0));
    style.text_styles.insert(TextStyle::Heading, FontId::proportional(17.0));
}

fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::CollapsingHeader::new(RichText::new(title).strong().size(16.5).color(theme::ACCENT_BRIGHT))
        .id_salt(("plot_section", title))
        .default_open(true)
        .show(ui, body);
    ui.add_space(4.0);
}

fn hint(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(theme::TEXT_SECONDARY));
}

/// All settings, in sections. Returns an export the user asked for.
fn settings(ui: &mut egui::Ui, state: &mut PlotState, table: &DataTable) -> PlotAction {
    section(ui, "Presets", |ui| presets_section(ui, state));
    section(ui, "Data", |ui| data_section(ui, state, table));
    section(ui, "Series style", |ui| style_section(ui, state, table));
    section(ui, "Axes", |ui| axes_section(ui, state, table));
    section(ui, "Legend", |ui| legend_section(ui, state));
    let mut export = PlotAction::None;
    section(ui, "Export", |ui| export = export_section(ui, state));
    export
}

fn presets_section(ui: &mut egui::Ui, state: &mut PlotState) {
    if state.presets.is_empty() {
        hint(ui, "No presets yet. Set up a plot, name it below and save it to reuse it in other plots and projects.");
    } else {
        state.preset_choice = state.preset_choice.min(state.presets.len() - 1);
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("plot_preset_choice")
                .width(180.0)
                .selected_text(state.presets[state.preset_choice].name.clone())
                .show_ui(ui, |ui| {
                    for (i, preset) in state.presets.iter().enumerate() {
                        ui.selectable_value(&mut state.preset_choice, i, &preset.name);
                    }
                });
            if ui.button("Apply").on_hover_text("Use this preset's columns, axes, legend and export settings").clicked() {
                let preset = state.presets[state.preset_choice].config.clone();
                state.apply_preset(&preset);
            }
            if ui.button("Delete").clicked() {
                state.presets.remove(state.preset_choice);
                save_presets(state);
            }
        });
    }
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut state.preset_name).hint_text("name, e.g. SI vs T").desired_width(180.0));
        let name = state.preset_name.trim().to_string();
        let exists = state.presets.iter().any(|p| p.name == name);
        let label = if exists { "Replace" } else { "Save current" };
        if ui
            .add_enabled(!name.is_empty(), egui::Button::new(label))
            .on_hover_text("Save the X/Y columns, axis titles and ranges, legend and export settings under this name")
            .clicked()
        {
            let preset = plotting::preset_from(state, &name);
            match state.presets.iter().position(|p| p.name == name) {
                Some(i) => state.presets[i] = preset,
                None => state.presets.push(preset),
            }
            state.preset_choice = state.presets.iter().position(|p| p.name == name).unwrap_or(0);
            state.preset_name.clear();
            save_presets(state);
        }
    });
    hint(ui, "Presets are shared by all projects. The chart title, the simulations shown and the series colours are not part of a preset.");
}

fn save_presets(state: &mut PlotState) {
    if let Err(e) = plotting::write_presets(&state.presets) {
        state.note = Some(format!("Could not save the plot presets: {e}"));
    }
}

fn data_section(ui: &mut egui::Ui, state: &mut PlotState, table: &DataTable) {
    ui.horizontal(|ui| {
        ui.label("X axis");
        egui::ComboBox::from_id_salt("plot_x_col")
            .width(220.0)
            .selected_text(table.headers.get(state.x_col).cloned().unwrap_or_default())
            .show_ui(ui, |ui| {
                for (i, h) in table.headers.iter().enumerate() {
                    if ui.selectable_value(&mut state.x_col, i, h).clicked() {
                        state.changed();
                    }
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Y series");
        if ui.button("Defaults").on_hover_text("The automatic choice for this table").clicked() {
            state.y_cols = plotting::default_axes(table).1;
            state.changed();
        }
        if ui.button("Clear").clicked() {
            state.y_cols.clear();
            state.changed();
        }
    });
    // The input's own columns (si_, totals, d_ …) first, PHREEQC's standard
    // columns after them.
    let mut order: Vec<usize> = (0..table.headers.len()).collect();
    order.sort_by_key(|&i| STANDARD_COLUMNS.contains(&table.headers[i].as_str()));
    egui::Frame::group(ui.style()).show(ui, |ui| {
        egui::ScrollArea::vertical().max_height(240.0).id_salt("y_series_scroll").show(ui, |ui| {
            ui.set_width(ui.available_width());
            for i in order {
                let mut checked = state.y_cols.contains(&i);
                if ui.checkbox(&mut checked, &table.headers[i]).changed() {
                    if checked {
                        state.y_cols.push(i);
                    } else {
                        state.y_cols.retain(|&c| c != i);
                    }
                    state.changed();
                }
            }
        });
    });

    let sims = plotting::plotted_simulations(table);
    if sims.len() > 1 {
        ui.horizontal(|ui| {
            ui.label("Simulations shown");
            if ui.button("All").clicked() {
                state.config.hidden_sims.clear();
                state.changed();
            }
            if ui.button("None").clicked() {
                state.config.hidden_sims.extend(sims.iter().copied());
                state.changed();
            }
        });
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::ScrollArea::vertical().max_height(200.0).id_salt("sim_filter_scroll").show(ui, |ui| {
                ui.set_width(ui.available_width());
                for &sim in &sims {
                    let name = match state.config.sim_names.get(&sim) {
                        Some(name) => format!("{sim}: {name}"),
                        None => format!("Simulation {sim}"),
                    };
                    let mut shown = !state.config.hidden_sims.contains(&sim);
                    if ui.checkbox(&mut shown, name).changed() {
                        if shown {
                            state.config.hidden_sims.remove(&sim);
                        } else {
                            state.config.hidden_sims.insert(sim);
                        }
                        state.changed();
                    }
                }
            });
        });
    }
}

fn style_section(ui: &mut egui::Ui, state: &mut PlotState, table: &DataTable) {
    state.assign_styles();
    let series = plotting::build_series(table, state.x_col, &state.y_cols, &state.config);
    let styles = plotting::chart_styles(&state.config, &series);
    if styles.is_empty() {
        hint(ui, "No series on the chart.");
        return;
    }
    hint(
        ui,
        "A column keeps its style in every plot and every project, also after unticking and ticking it again. \
         With one column over several simulations, each simulation has its own style instead; with several \
         columns over several simulations, each simulation gets its own marker shape.",
    );
    if ui.button("Reset all to default").clicked() {
        for (key, _, _) in &styles {
            if let Some(entry) = state.config.series.get_mut(key) {
                entry.custom = None;
            }
        }
        state.style_changed();
    }
    for (i, (key, auto_label, shown)) in styles.iter().enumerate() {
        let entry = state.config.series.entry(key.clone()).or_default();
        let custom = entry.custom.is_some();
        let mut style = *shown;
        let mut label = entry.label.clone();
        let mut reset = false;
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                colour_button(ui, ("plot_colour", i), &mut style.color);
                ui.add(egui::TextEdit::singleline(&mut label).hint_text(auto_label).desired_width(f32::INFINITY))
                    .on_hover_text("Legend label; leave empty for the automatic one");
            });
            ui.horizontal(|ui| {
                ui.label("Line");
                egui::ComboBox::from_id_salt(("plot_line", i))
                    .width(96.0)
                    .selected_text(style.line.label())
                    .show_ui(ui, |ui| {
                        for line in LineType::ALL {
                            ui.selectable_value(&mut style.line, line, line.label());
                        }
                    });
                ui.label("width");
                ui.add(egui::DragValue::new(&mut style.line_width).range(0.5..=10.0).speed(0.1).max_decimals(1));
            });
            ui.horizontal(|ui| {
                ui.label("Marker");
                egui::ComboBox::from_id_salt(("plot_marker", i))
                    .width(96.0)
                    .selected_text(style.marker.label())
                    .show_ui(ui, |ui| {
                        for marker in MarkerShape::ALL {
                            ui.selectable_value(&mut style.marker, marker, marker.label());
                        }
                    });
                ui.label("size");
                ui.add(egui::DragValue::new(&mut style.marker_size).range(0.0..=20.0).speed(0.1).max_decimals(1));
            });
            if ui
                .add_enabled(custom, egui::Button::new("Reset to default"))
                .on_hover_text("Back to the automatic style: a solid line with circles in this series' own colour")
                .clicked()
            {
                reset = true;
            }
        });
        let entry = state.config.series.entry(key.clone()).or_default();
        let mut changed = false;
        if reset {
            entry.custom = None;
            changed = true;
        } else if style != *shown {
            entry.custom = Some(style);
            changed = true;
        }
        if label != entry.label {
            entry.label = label;
            changed = true;
        }
        if changed {
            state.style_changed();
        }
    }
}

/// Ready-made colours offered above the colour picker.
const PRESETS: [(&str, &[[u8; 3]]); 3] = [
    (
        "Basic",
        &[
            [0x00, 0x00, 0x00], [0x40, 0x40, 0x40], [0x80, 0x80, 0x80], [0xb0, 0xb0, 0xb0],
            [0x1f, 0x3a, 0x93], [0x8b, 0x00, 0x00], [0x00, 0x64, 0x00], [0x6a, 0x0d, 0xad],
            [0x8b, 0x45, 0x13], [0x00, 0x80, 0x80],
        ],
    ),
    (
        "Matplotlib (tab10)",
        &[
            [0x1f, 0x77, 0xb4], [0xff, 0x7f, 0x0e], [0x2c, 0xa0, 0x2c], [0xd6, 0x27, 0x28],
            [0x94, 0x67, 0xbd], [0x8c, 0x56, 0x4b], [0xe3, 0x77, 0xc2], [0x7f, 0x7f, 0x7f],
            [0xbc, 0xbd, 0x22], [0x17, 0xbe, 0xcf],
        ],
    ),
    (
        "Colour-blind safe (Okabe–Ito)",
        &[
            [0x00, 0x00, 0x00], [0xe6, 0x9f, 0x00], [0x56, 0xb4, 0xe9], [0x00, 0x9e, 0x73],
            [0xf0, 0xe4, 0x42], [0x00, 0x72, 0xb2], [0xd5, 0x5e, 0x00], [0xcc, 0x79, 0xa7],
        ],
    ),
];

fn swatch(ui: &mut egui::Ui, colour: [u8; 3], size: egui::Vec2, selected: bool) -> egui::Response {
    let fill = egui::Color32::from_rgb(colour[0], colour[1], colour[2]);
    let stroke = if selected {
        egui::Stroke::new(2.0, theme::ACCENT_BRIGHT)
    } else {
        egui::Stroke::new(1.0, theme::STROKE_STRONG)
    };
    ui.add(egui::Button::new("").fill(fill).stroke(stroke).min_size(size))
}

/// A colour swatch that opens ready-made colours (basic, Matplotlib,
/// colour-blind safe, PQ Studio's own) above the full colour picker.
fn colour_button(ui: &mut egui::Ui, id: impl std::hash::Hash + std::fmt::Debug, colour: &mut [u8; 3]) {
    let button = swatch(ui, *colour, vec2(44.0, 28.0), false).on_hover_text("Colour: ready-made colours or any colour");
    let popup_id = ui.make_persistent_id(id);
    // Visual QA only: `PQ_STUDIO_SCREENSHOT_COLOUR_POPUP=1` keeps the first
    // series' colour popup open for a screenshot.
    if std::env::var("PQ_STUDIO_SCREENSHOT_COLOUR_POPUP").is_ok()
        && popup_id == ui.make_persistent_id(("plot_colour", 0usize))
    {
        egui::Popup::open_id(ui.ctx(), popup_id);
    }
    egui::Popup::from_toggle_button_response(&button)
        .id(popup_id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_max_width(330.0);
            let mut picked = false;
            let groups = PRESETS.iter().map(|(name, colours)| (*name, colours.to_vec())).chain(std::iter::once((
                "PQ Studio",
                plotting::CATEGORICAL_PALETTE.iter().map(|&(r, g, b)| [r, g, b]).collect(),
            )));
            for (name, colours) in groups {
                ui.label(RichText::new(name).small().color(theme::TEXT_SECONDARY));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
                    for c in colours {
                        let hex = format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
                        if swatch(ui, c, vec2(26.0, 22.0), c == *colour).on_hover_text(hex).clicked() {
                            *colour = c;
                            picked = true;
                        }
                    }
                });
            }
            ui.separator();
            ui.label(RichText::new("Any colour").small().color(theme::TEXT_SECONDARY));
            let mut c32 = egui::Color32::from_rgb(colour[0], colour[1], colour[2]);
            if egui::widgets::color_picker::color_picker_color32(ui, &mut c32, egui::widgets::color_picker::Alpha::Opaque) {
                *colour = [c32.r(), c32.g(), c32.b()];
            }
            if picked {
                egui::Popup::close_id(ui.ctx(), popup_id);
            }
        });
}

/// A text field that marks the chart changed when edited.
fn text_field(ui: &mut egui::Ui, state_changed: &mut bool, value: &mut String, hint_text: &str, width: f32) {
    if ui
        .add(egui::TextEdit::singleline(value).hint_text(hint_text).desired_width(width))
        .changed()
    {
        *state_changed = true;
    }
}

fn axes_section(ui: &mut egui::Ui, state: &mut PlotState, table: &DataTable) {
    // Shown greyed out in the empty fields, so it's clear what "auto" means.
    let auto_title = match state.config.document_title.trim() {
        "" => "PHREEQC results".to_string(),
        t => t.to_string(),
    };
    let auto_x = plotting::default_x_label(table, state.x_col);
    let auto_y = plotting::default_y_label(table, &state.y_cols);
    let mut changed = false;
    let c = &mut state.config;
    egui::Grid::new("plot_axes_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
        ui.label("Chart title");
        text_field(ui, &mut changed, &mut c.chart_title, &auto_title, 250.0);
        ui.end_row();

        ui.label("X title");
        text_field(ui, &mut changed, &mut c.x_axis_title, &auto_x, 250.0);
        ui.end_row();
        ui.label("X range");
        ui.horizontal(|ui| {
            text_field(ui, &mut changed, &mut c.x_min, "auto", 70.0);
            ui.label("to");
            text_field(ui, &mut changed, &mut c.x_max, "auto", 70.0);
            changed |= ui.checkbox(&mut c.x_log, "log").changed();
        });
        ui.end_row();

        ui.label("Y title");
        text_field(ui, &mut changed, &mut c.y_axis_title, &auto_y, 250.0);
        ui.end_row();
        ui.label("Y range");
        ui.horizontal(|ui| {
            text_field(ui, &mut changed, &mut c.y_min, "auto", 70.0);
            ui.label("to");
            text_field(ui, &mut changed, &mut c.y_max, "auto", 70.0);
            changed |= ui.checkbox(&mut c.y_log, "log").changed();
        });
        ui.end_row();

        ui.label("Text sizes");
        ui.horizontal(|ui| {
            for (label, value, tip) in [
                ("title", &mut c.title_font_size, "Chart title size"),
                ("axes", &mut c.axis_font_size, "Axis title size"),
                ("ticks", &mut c.tick_font_size, "Tick label size"),
            ] {
                changed |= ui
                    .add(egui::DragValue::new(value).range(8.0..=60.0).speed(0.2).max_decimals(0).prefix(format!("{label} ")))
                    .on_hover_text(tip)
                    .changed();
            }
        });
        ui.end_row();

        ui.label("Grid");
        changed |= ui.checkbox(&mut c.grid, "grid lines").changed();
        ui.end_row();

        ui.label("SI = 0");
        changed |= ui
            .checkbox(&mut c.zero_line, "dashed line at saturation")
            .on_hover_text("A dashed line at SI = 0 when saturation indices are plotted: above it the mineral is supersaturated, below it undersaturated")
            .changed();
        ui.end_row();
    });
    hint(ui, "Empty range fields are automatic. log plots log10 of the values.");
    if changed {
        state.changed();
    }
}

fn legend_section(ui: &mut egui::Ui, state: &mut PlotState) {
    let mut changed = false;
    let legend = &mut state.config.legend;
    egui::Grid::new("plot_legend_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
        ui.label("Position");
        egui::ComboBox::from_id_salt("plot_legend_position")
            .width(180.0)
            .selected_text(legend.position.label())
            .show_ui(ui, |ui| {
                for position in LegendPosition::ALL {
                    changed |= ui.selectable_value(&mut legend.position, position, position.label()).changed();
                }
            });
        ui.end_row();

        ui.label("Font size");
        changed |= ui.add(egui::DragValue::new(&mut legend.font_size).range(8.0..=48.0).speed(0.2).max_decimals(0)).changed();
        ui.end_row();

        ui.label("Frame");
        changed |= ui.checkbox(&mut legend.frame, "border around the legend").changed();
        ui.end_row();

        ui.label("Background");
        changed |= ui
            .add(egui::Slider::new(&mut legend.opacity, 0.0..=1.0).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)))
            .on_hover_text("How opaque the white legend background is over the curves")
            .changed();
        ui.end_row();
    });
    hint(ui, "The legend is drawn over the curves and never changes the axis ranges. It is left out when only one series is shown, since the Y axis title names it.");
    if changed {
        state.changed();
    }
}

/// Returns the export the user asked for.
fn export_section(ui: &mut egui::Ui, state: &mut PlotState) -> PlotAction {
    let mut changed = false;
    let c = &mut state.config;
    egui::Grid::new("plot_export_grid").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
        ui.label("Figure size");
        ui.horizontal(|ui| {
            changed |= ui.add(egui::DragValue::new(&mut c.export_width).range(300..=4000).speed(5).suffix(" px")).changed();
            ui.label("×");
            changed |= ui.add(egui::DragValue::new(&mut c.export_height).range(200..=4000).speed(5).suffix(" px")).changed();
        });
        ui.end_row();

        ui.label("Shape");
        ui.horizontal(|ui| {
            for (label, w, h) in [("3:2", 1200, 800), ("4:3", 1200, 900), ("16:9", 1280, 720), ("1:1", 1000, 1000)] {
                if ui.button(label).on_hover_text(format!("{w} × {h} px")).clicked() {
                    c.export_width = w;
                    c.export_height = h;
                    changed = true;
                }
            }
        });
        ui.end_row();

        ui.label("Resolution");
        egui::ComboBox::from_id_salt("plot_export_scale")
            .width(180.0)
            .selected_text(scale_label(c.export_scale()))
            .show_ui(ui, |ui| {
                for scale in 1..=4 {
                    changed |= ui.selectable_value(&mut c.export_scale, scale, scale_label(scale)).changed();
                }
            });
        ui.end_row();
    });
    let (w, h) = state.config.figure_size();
    let scale = state.config.export_scale();
    hint(
        ui,
        &format!(
            "PNG: {} × {} px, the chart exactly as shown. SVG: {w} × {h}. Text keeps its size, so a larger figure fits more; raise the resolution for a sharper image instead.",
            w * scale,
            h * scale
        ),
    );
    if changed {
        state.changed();
    }
    let mut action = PlotAction::None;
    ui.horizontal_wrapped(|ui| {
        if ui.add_enabled(!state.y_cols.is_empty(), egui::Button::new("Export plot…")).clicked() {
            action = PlotAction::ExportPlot;
        }
        if ui
            .add_enabled(!state.y_cols.is_empty(), egui::Button::new("Export plotted data…"))
            .on_hover_text(PLOTTED_DATA_TIP)
            .clicked()
        {
            action = PlotAction::ExportData;
        }
    });
    action
}

const PLOTTED_DATA_TIP: &str = "Save just what the chart shows (X, the ticked Y series, the simulations shown) as an Excel workbook (.xlsx, one sheet per simulation too) or CSV, to plot it elsewhere";

fn scale_label(scale: u32) -> String {
    format!("{scale}× ({} dpi)", 96 * scale)
}
