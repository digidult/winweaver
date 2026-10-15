//! The look of variant A (`docs/design/mockups/common.css`): the palette of light and dark, Segoe
//! UI and Consolas from the fonts of Windows, the spacing, and the small painted widgets egui has
//! no style for: the buttons of the command bar, icon buttons, cards, section titles and chips.

use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontFamily, FontId, Margin, RichText, Sense, Shape,
    Stroke, StrokeKind,
};

/// The colours of the interface, the tokens of the mockups.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub dark: bool,
    /// The window behind the cards.
    pub bg: Color32,
    pub card: Color32,
    /// Fields and the inside of cards.
    pub card2: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub hover: Color32,
    /// The selected row.
    pub sel: Color32,
    /// Disabled items and the frame of an empty check box.
    pub off: Color32,
    pub warn: Color32,
    pub ok: Color32,
    /// The message bar.
    pub msg_bg: Color32,
    pub msg_fg: Color32,
    pub msg_border: Color32,
}

const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

impl Palette {
    pub const LIGHT: Palette = Palette {
        dark: false,
        bg: hex(0xf3f3f3),
        card: hex(0xffffff),
        card2: hex(0xf9f9f9),
        border: hex(0xe3e3e3),
        text: hex(0x1b1b1b),
        muted: hex(0x6d6d6d),
        accent: hex(0x0067c0),
        hover: hex(0xececec),
        sel: hex(0xd9e7f5),
        off: hex(0x8d8d8d),
        warn: hex(0xc66900),
        ok: hex(0x0f7b0f),
        msg_bg: hex(0xfff4e5),
        msg_fg: hex(0x5a3b00),
        msg_border: hex(0xf0d9b5),
    };

    pub const DARK: Palette = Palette {
        dark: true,
        bg: hex(0x202020),
        card: hex(0x2b2b2b),
        card2: hex(0x272727),
        border: hex(0x3b3b3b),
        text: hex(0xf0f0f0),
        muted: hex(0xa3a3a3),
        accent: hex(0x4cc2ff),
        hover: hex(0x353535),
        sel: hex(0x2f4257),
        off: hex(0x7a7a7a),
        warn: hex(0xffb15c),
        ok: hex(0x6ccb5f),
        msg_bg: hex(0x433418),
        msg_fg: hex(0xffd9a0),
        msg_border: hex(0x5c4620),
    };

    /// The palette of a colour theme: its light or dark base with the colours it names. The
    /// names of the first version (background, foreground, field, select...) are understood too.
    pub fn from_colors(dark: bool, colors: &std::collections::BTreeMap<String, String>) -> Self {
        let mut p = if dark { Self::DARK } else { Self::LIGHT };
        let get = |names: &[&str]| {
            names
                .iter()
                .find_map(|n| colors.get(*n).and_then(|t| crate::themes::color(t)))
        };
        let slots: [(&mut Color32, &[&str]); 15] = [
            (&mut p.bg, &["bg", "background"]),
            (&mut p.card, &["card", "field"]),
            (&mut p.card2, &["card2", "field"]),
            (&mut p.border, &["border"]),
            (&mut p.text, &["text", "foreground"]),
            (&mut p.muted, &["muted"]),
            (&mut p.accent, &["accent", "link"]),
            (&mut p.hover, &["hover", "button_active", "button"]),
            (&mut p.sel, &["sel", "select"]),
            (&mut p.off, &["off", "disabled"]),
            (&mut p.warn, &["warn", "warning"]),
            (&mut p.ok, &["ok"]),
            (&mut p.msg_bg, &["msg_bg"]),
            (&mut p.msg_fg, &["msg_fg"]),
            (&mut p.msg_border, &["msg_border"]),
        ];
        for (slot, names) in slots {
            if let Some(c) = get(names) {
                *slot = c;
            }
        }
        p
    }
}

/// The visuals of egui from a palette.
pub fn visuals(p: &Palette) -> egui::Visuals {
    let mut v = if p.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let r = CornerRadius::same(6);
    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.faint_bg_color = p.card2;
    v.extreme_bg_color = p.card2;
    v.code_bg_color = p.hover;
    v.text_edit_bg_color = Some(p.card2);
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(8);
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = p.sel;
    v.selection.stroke = Stroke::new(1.0, p.text);
    v.warn_fg_color = p.warn;
    v.widgets.noninteractive.bg_fill = p.card;
    v.widgets.noninteractive.weak_bg_fill = p.card;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.noninteractive.corner_radius = r;
    v.widgets.inactive.bg_fill = p.card;
    v.widgets.inactive.weak_bg_fill = p.card;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.hovered.bg_fill = p.hover;
    v.widgets.hovered.weak_bg_fill = p.hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.active.bg_fill = p.sel;
    v.widgets.active.weak_bg_fill = p.sel;
    v.widgets.active.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.open.bg_fill = p.hover;
    v.widgets.open.weak_bg_fill = p.hover;
    v.widgets.open.bg_stroke = Stroke::new(1.0, p.border);
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = r;
        w.fg_stroke = Stroke::new(1.0, p.text);
        w.expansion = 0.0;
    }
    v
}

fn palettes_id() -> egui::Id {
    egui::Id::new("winweaver-palettes")
}

fn fonts_id() -> egui::Id {
    egui::Id::new("winweaver-fonts")
}

/// Sets the visuals of both themes, the spacing and the text sizes, and remembers the palettes
/// for the painted widgets.
pub fn apply(ctx: &egui::Context, light: Palette, dark: Palette) {
    ctx.set_visuals_of(egui::Theme::Light, visuals(&light));
    ctx.set_visuals_of(egui::Theme::Dark, visuals(&dark));
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(10.0, 4.0);
        s.spacing.interact_size.y = 24.0;
        s.spacing.menu_margin = Margin::same(6);
        s.spacing.window_margin = Margin::same(10);
        for (style, size, family) in [
            (egui::TextStyle::Body, 13.0, FontFamily::Proportional),
            (egui::TextStyle::Button, 13.0, FontFamily::Proportional),
            (egui::TextStyle::Small, 11.0, FontFamily::Proportional),
            (egui::TextStyle::Heading, 18.0, FontFamily::Proportional),
            (egui::TextStyle::Monospace, 12.0, FontFamily::Monospace),
        ] {
            s.text_styles.insert(style, FontId::new(size, family));
        }
    });
    ctx.data_mut(|d| d.insert_temp(palettes_id(), (light, dark)));
}

/// The palette of the theme the window shows now.
pub fn palette(ui: &egui::Ui) -> Palette {
    let (light, dark) = ui
        .ctx()
        .data(|d| d.get_temp::<(Palette, Palette)>(palettes_id()))
        .unwrap_or((Palette::LIGHT, Palette::DARK));
    if ui.visuals().dark_mode { dark } else { light }
}

/// The name of the font family for titles (semibold Segoe UI).
pub const SEMIBOLD: &str = "semibold";

/// Loads Segoe UI, its semibold cut and Consolas from the fonts folder of Windows when they are
/// there; the fonts of egui stay as the fallback (symbols, emoji). Call it before the first frame.
pub fn install_fonts(ctx: &egui::Context) {
    let dir = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\Windows"))
        .join("Fonts");
    let read = |name: &str| {
        std::fs::read(dir.join(name))
            .ok()
            .filter(|b| b.len() < 32 * 1024 * 1024)
    };
    let mut defs = egui::FontDefinitions::default();
    let mut semibold = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some(bytes) = read("segoeui.ttf") {
        defs.font_data
            .insert("segoe".into(), Arc::new(egui::FontData::from_owned(bytes)));
        defs.families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe".into());
        semibold.insert(0, "segoe".into());
    }
    if let Some(bytes) = read("seguisb.ttf") {
        defs.font_data.insert(
            "segoe-semibold".into(),
            Arc::new(egui::FontData::from_owned(bytes)),
        );
        semibold.insert(0, "segoe-semibold".into());
    }
    if let Some(bytes) = read("consola.ttf") {
        defs.font_data
            .insert("consolas".into(), Arc::new(egui::FontData::from_owned(bytes)));
        defs.families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "consolas".into());
    }
    defs.families
        .insert(FontFamily::Name(SEMIBOLD.into()), semibold);
    ctx.set_fonts(defs);
    ctx.data_mut(|d| d.insert_temp(fonts_id(), true));
}

/// A semibold font of a size, or the regular one before the fonts are installed.
pub fn strong_font(ui: &egui::Ui, size: f32) -> FontId {
    let ready = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(fonts_id()))
        .unwrap_or(false);
    if ready {
        FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
    } else {
        FontId::proportional(size)
    }
}

/// Text in the semibold font.
pub fn strong(ui: &egui::Ui, text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).font(strong_font(ui, size))
}

/// A small triangle that points down, for split buttons.
fn caret(painter: &egui::Painter, center: egui::Pos2, color: Color32) {
    let s = 3.0;
    painter.add(Shape::convex_polygon(
        vec![
            center + egui::vec2(-s, -s * 0.5),
            center + egui::vec2(s, -s * 0.5),
            center + egui::vec2(0.0, s * 0.7),
        ],
        color,
        Stroke::NONE,
    ));
}

/// A button of the command bar: the icon above the label, or the icon alone when `compact`;
/// `split` adds the caret of a button with a menu.
pub fn tool_button(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    hint: &str,
    enabled: bool,
    compact: bool,
    split: bool,
) -> egui::Response {
    let p = palette(ui);
    let label_font = FontId::proportional(11.0);
    let width = if compact {
        if split { 40.0 } else { 32.0 }
    } else {
        let text = ui
            .painter()
            .layout_no_wrap(label.to_owned(), label_font.clone(), p.text)
            .size()
            .x;
        (text + 20.0 + if split { 12.0 } else { 0.0 }).max(58.0)
    };
    let height = if compact { 28.0 } else { 44.0 };
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), sense);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if enabled && response.hovered() {
            painter.rect_filled(rect, CornerRadius::same(6), p.hover);
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect,
                CornerRadius::same(6),
                Stroke::new(1.5, p.accent),
                StrokeKind::Inside,
            );
        }
        let color = if enabled { p.text } else { p.off };
        let shift = if split { 6.0 } else { 0.0 };
        let icon_y = if compact {
            rect.center().y
        } else {
            rect.top() + 15.0
        };
        painter.text(
            egui::pos2(rect.center().x - shift, icon_y),
            Align2::CENTER_CENTER,
            icon,
            FontId::proportional(if compact { 15.0 } else { 17.0 }),
            color,
        );
        if !compact {
            painter.text(
                egui::pos2(rect.center().x - shift, rect.bottom() - 10.0),
                Align2::CENTER_CENTER,
                label,
                label_font,
                color,
            );
        }
        if split {
            caret(painter, egui::pos2(rect.right() - 8.0, icon_y), p.muted);
        }
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    response.on_hover_text(hint)
}

/// A small square button with an icon, such as back, forward, the star or the search.
pub fn icon_button(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    enabled: bool,
    color: Option<Color32>,
) -> egui::Response {
    let p = palette(ui);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(30.0, 28.0), sense);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if enabled && response.hovered() {
            painter.rect_filled(rect, CornerRadius::same(6), p.hover);
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect,
                CornerRadius::same(6),
                Stroke::new(1.5, p.accent),
                StrokeKind::Inside,
            );
        }
        let c = if enabled {
            color.unwrap_or(p.muted)
        } else {
            p.off.gamma_multiply(0.6)
        };
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            icon,
            FontId::proportional(15.0),
            c,
        );
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    response.on_hover_text(label)
}

/// A frameless text button with the hover background, such as a part of the breadcrumb.
pub fn flat_button(ui: &mut egui::Ui, text: RichText) -> egui::Response {
    ui.add(egui::Button::new(text).frame(false).min_size(egui::vec2(0.0, 24.0)))
}

/// A card: the white rounded box of the detail panel.
pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let p = palette(ui);
    egui::Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// The title of a section in capitals, small and muted.
pub fn section_title(ui: &mut egui::Ui, text: &str) {
    let p = palette(ui);
    ui.label(
        RichText::new(text.to_uppercase())
            .font(strong_font(ui, 11.0))
            .color(p.muted),
    );
}

/// A rounded label of a fact, such as the mechanism or the phase of a rule.
pub fn chip(ui: &mut egui::Ui, text: &str, accent: bool) -> egui::Response {
    let p = palette(ui);
    egui::Frame::new()
        .fill(if accent { p.sel } else { p.hover })
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 1))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(11.0)
                    .color(if accent { p.text } else { p.muted }),
            )
        })
        .inner
}

/// Text the reader can select, with a menu on the right button: copy the selection, select all,
/// copy all. It wraps at the width of the panel and never changes.
pub fn selectable_text(ui: &mut egui::Ui, id: impl std::hash::Hash, text: &str) {
    let id = ui.make_persistent_id(id);
    let mut shown = text;
    let output = egui::TextEdit::multiline(&mut shown)
        .id(id)
        .frame(false)
        .desired_width(f32::INFINITY)
        .desired_rows(1)
        .margin(Margin::ZERO)
        .show(ui);
    let selected: String = output
        .cursor_range
        .map(|r| {
            let r = r.as_sorted_char_range();
            text.chars()
                .skip(r.start.0)
                .take(r.end.0.saturating_sub(r.start.0))
                .collect()
        })
        .unwrap_or_default();
    let galley = output.galley.clone();
    output.response.context_menu(|ui| {
        if ui
            .add_enabled(!selected.is_empty(), egui::Button::new(crate::i18n::tr("Copy")))
            .clicked()
        {
            ui.ctx().copy_text(selected.clone());
            ui.close();
        }
        if ui.button(crate::i18n::tr("Select all")).clicked() {
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                state
                    .cursor
                    .set_char_range(Some(egui::text_selection::CCursorRange::select_all(&galley)));
                state.store(ui.ctx(), id);
            }
            ui.memory_mut(|m| m.request_focus(id));
            ui.close();
        }
        if ui.button(crate::i18n::tr("Copy all")).clicked() {
            ui.ctx().copy_text(text.to_owned());
            ui.close();
        }
    });
}

/// Muted text of a size.
pub fn muted(ui: &egui::Ui, text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).size(size).color(palette(ui).muted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_of_a_theme_replace_the_base() {
        let mut colors = std::collections::BTreeMap::new();
        colors.insert("background".to_owned(), "#000000".to_owned());
        colors.insert("accent".to_owned(), "#00ff41".to_owned());
        let p = Palette::from_colors(true, &colors);
        assert_eq!(p.bg, Color32::from_rgb(0, 0, 0));
        assert_eq!(p.accent, Color32::from_rgb(0, 255, 65));
        assert_eq!(p.card, Palette::DARK.card);
        assert!(visuals(&p).dark_mode);
    }
}
