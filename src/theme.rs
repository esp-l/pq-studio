//! Single app-wide dark theme, applied once at startup rather than scattered
//! per-widget overrides, so every screen (main window, tree, all block
//! editors, output panel, plot preview chrome) stays visually consistent as
//! more block types are added.
//!
//! Palette: neutral graphite dark theme with a green accent (#018458, and
//! a brighter #2fc48f for text on dark).
//!
//! The rendered plot itself (plotters output, in src/plotting.rs) is
//! deliberately NOT themed by any of this: it always renders on a white,
//! print-safe background regardless of the app's own dark chrome.

use egui::{Color32, CornerRadius, Margin, Stroke, Vec2};

pub const BG_BASE: Color32 = Color32::from_rgb(0x12, 0x14, 0x17);
pub const BG_PANEL: Color32 = Color32::from_rgb(0x1a, 0x1d, 0x21);
pub const BG_WIDGET_INACTIVE: Color32 = Color32::from_rgb(0x24, 0x28, 0x2d);
pub const BG_WIDGET_HOVERED: Color32 = Color32::from_rgb(0x2e, 0x33, 0x39);
pub const BG_EXTREME: Color32 = Color32::from_rgb(0x0b, 0x0d, 0x0f);

pub const STROKE_SUBTLE: Color32 = Color32::from_rgb(0x34, 0x3a, 0x40);
pub const STROKE_STRONG: Color32 = Color32::from_rgb(0x4a, 0x52, 0x59);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xe6, 0xe8, 0xea);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x9a, 0xa1, 0xa8);

pub const ACCENT_BRIGHT: Color32 = Color32::from_rgb(0x2f, 0xc4, 0x8f);
pub const ACCENT_BASE: Color32 = Color32::from_rgb(0x01, 0x84, 0x58);

pub const ERROR: Color32 = Color32::from_rgb(0xe3, 0x49, 0x48);
pub const WARNING: Color32 = Color32::from_rgb(0xed, 0xa1, 0x00);

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = None;
    visuals.weak_text_color = Some(TEXT_SECONDARY);
    visuals.panel_fill = BG_BASE;
    visuals.window_fill = BG_PANEL;
    visuals.extreme_bg_color = BG_EXTREME;
    visuals.code_bg_color = BG_EXTREME;
    visuals.faint_bg_color = BG_WIDGET_INACTIVE;
    visuals.hyperlink_color = ACCENT_BRIGHT;
    visuals.error_fg_color = ERROR;
    visuals.warn_fg_color = WARNING;
    visuals.window_stroke = Stroke::new(1.0, STROKE_SUBTLE);
    visuals.striped = true;
    visuals.disabled_alpha = 0.6;

    visuals.selection.bg_fill = ACCENT_BASE;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT_BRIGHT);

    let corner = CornerRadius::same(4);

    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, STROKE_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.noninteractive.corner_radius = corner;

    visuals.widgets.inactive.bg_fill = BG_WIDGET_INACTIVE;
    visuals.widgets.inactive.weak_bg_fill = BG_WIDGET_INACTIVE;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, STROKE_SUBTLE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = corner;

    visuals.widgets.hovered.bg_fill = BG_WIDGET_HOVERED;
    visuals.widgets.hovered.weak_bg_fill = BG_WIDGET_HOVERED;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_BRIGHT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = corner;

    visuals.widgets.active.bg_fill = ACCENT_BASE;
    visuals.widgets.active.weak_bg_fill = ACCENT_BASE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_BRIGHT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.corner_radius = corner;

    visuals.widgets.open.bg_fill = BG_WIDGET_HOVERED;
    visuals.widgets.open.weak_bg_fill = BG_WIDGET_HOVERED;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, STROKE_STRONG);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.open.corner_radius = corner;

    ctx.set_theme(egui::ThemePreference::Dark);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.visuals = visuals;

    // Generous spacing throughout — nothing cramped or touching edges.
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    style.spacing.window_margin = Margin::same(12);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.indent = 20.0;
    style.spacing.interact_size.y = 26.0;
    style.spacing.scroll.bar_width = 10.0;

    ctx.set_style_of(egui::Theme::Dark, style);
}
