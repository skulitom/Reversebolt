//! Bolt-family visual tokens, informed by Agentbolt's Desk and Anode's viewer.
use eframe::egui::{self, Color32, FontId, RichText, Stroke, TextStyle};

pub const BACKGROUND: Color32 = Color32::from_rgb(24, 26, 30);
pub const CHROME: Color32 = Color32::from_rgb(32, 32, 35);
pub const SURFACE: Color32 = Color32::from_rgb(32, 35, 40);
pub const TEXT: Color32 = Color32::from_rgb(235, 236, 239);
pub const MUTED: Color32 = Color32::from_rgb(167, 173, 182);
pub const BORDER: Color32 = Color32::from_rgb(61, 66, 75);
pub const ACCENT: Color32 = Color32::from_rgb(255, 196, 46);
pub const SELECTED: Color32 = Color32::from_rgb(69, 56, 28);
pub const GREEN: Color32 = Color32::from_rgb(115, 211, 158);
pub const WARNING: Color32 = Color32::from_rgb(255, 204, 102);
pub const ERROR: Color32 = Color32::from_rgb(255, 139, 139);

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark));
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.visuals = egui::Visuals::dark();
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.interact_size.y = 32.0;
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(15.0));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(14.0));
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::proportional(24.0));
    style
        .text_styles
        .insert(TextStyle::Small, FontId::proportional(12.0));
    let v = &mut style.visuals;
    v.panel_fill = BACKGROUND;
    v.window_fill = SURFACE;
    v.extreme_bg_color = SURFACE;
    v.text_edit_bg_color = Some(SURFACE);
    v.faint_bg_color = SURFACE;
    v.override_text_color = Some(TEXT);
    v.weak_text_color = Some(MUTED);
    v.selection.bg_fill = SELECTED;
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    for widget in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        widget.corner_radius = 5.into();
        widget.fg_stroke = Stroke::new(1.0, TEXT);
        widget.bg_stroke = Stroke::new(1.0, BORDER);
        widget.bg_fill = SURFACE;
        widget.weak_bg_fill = SURFACE;
    }
    v.widgets.hovered.bg_fill = SELECTED;
    v.widgets.hovered.weak_bg_fill = SELECTED;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.bg_fill = SELECTED;
    v.widgets.active.weak_bg_fill = SELECTED;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.warn_fg_color = WARNING;
    v.error_fg_color = ERROR;
    ctx.set_style_of(egui::Theme::Dark, style);
}

pub fn primary(label: &str) -> egui::Button<'_> {
    egui::Button::new(
        RichText::new(label)
            .strong()
            .color(Color32::from_rgb(41, 31, 9)),
    )
    .fill(ACCENT)
}
