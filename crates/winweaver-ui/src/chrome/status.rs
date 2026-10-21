//! The message bar and the status bar.

use super::*;

impl App {
    /// The message bar of the checks and warnings, folded to one line or open with its own
    /// height that the user drags.
    pub(crate) fn messages_panel(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let Some(Doc::Profile(doc)) = self.docs.get(self.active) else {
            return;
        };
        if doc.issues.is_empty() {
            return;
        }
        let p = Self::palette_now(ui);
        let texts = texts_for(&self.texts, &crate::i18n::current());
        let frame = egui::Frame::new()
            .fill(p.msg_bg)
            .stroke(Stroke::new(1.0, p.msg_border))
            .inner_margin(Margin::symmetric(12, 4));
        let open = self.messages_open;
        let header = |ui: &mut egui::Ui, acts: &mut Vec<Act>| {
            ui.horizontal(|ui| {
                let errors = validate::errors(&doc.issues);
                let icon = if errors > 0 {
                    icons::ERROR
                } else {
                    icons::WARNING
                };
                ui.label(RichText::new(icon).color(p.msg_fg));
                ui.label(RichText::new(validate::summary(&doc.issues)).color(p.msg_fg));
                let toggle = if open { tr("Hide") } else { tr("Show") };
                if ui.link(toggle).clicked() {
                    acts.push(Act::Nav(NavAct::ToggleMessages));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if style::icon_button(
                        ui,
                        icons::CLOSE,
                        tr("Hide the messages"),
                        true,
                        Some(p.msg_fg),
                    )
                    .clicked()
                    {
                        acts.push(Act::Profile(ProfileAct::ClearMessages));
                    }
                });
            });
        };
        if !open {
            egui::Panel::top("messages-bar")
                .frame(frame)
                .show_separator_line(false)
                .show(ui, |ui| header(ui, acts));
            return;
        }
        let max = (ui.available_height() * 0.6).max(120.0);
        egui::Panel::top("messages")
            .frame(frame)
            .resizable(true)
            .default_size(150.0)
            .size_range(60.0..=max)
            .show(ui, |ui| {
                header(ui, acts);
                egui::ScrollArea::vertical()
                    .id_salt("messages-list")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        for issue in &doc.issues {
                            ui.horizontal(|ui| {
                                let (tag, color) = match issue.severity {
                                    Severity::Error => (tr("Error"), ui.visuals().error_fg_color),
                                    Severity::Warning => (tr("Warning"), p.warn),
                                    Severity::Info => (tr("Info"), p.muted),
                                };
                                ui.label(
                                    RichText::new(tag).color(color).size(style::size(ui, 12.0)),
                                );
                                let (label, sel) = match &issue.target {
                                    Target::Rule(id) => (
                                        self.catalog.rule(id).map_or_else(
                                            || id.clone(),
                                            |r| views::rtitle(texts, r).to_owned(),
                                        ),
                                        Sel::Rule(id.clone()),
                                    ),
                                    Target::Install => {
                                        (tr("Installation").to_owned(), Sel::Form(Form::Install))
                                    }
                                    Target::Accounts => {
                                        (tr("Accounts").to_owned(), Sel::Form(Form::Accounts))
                                    }
                                    Target::Languages => {
                                        (tr("Languages").to_owned(), Sel::Form(Form::Languages))
                                    }
                                    Target::Profile => (doc.profile.name.clone(), Sel::None),
                                    // Their group, unless the settings hide it from the tree.
                                    Target::Unknown if self.settings.hide_unknown => {
                                        (doc.profile.name.clone(), Sel::None)
                                    }
                                    Target::Unknown => (
                                        tr("Unknown options").to_owned(),
                                        Sel::Group(crate::UNKNOWN_GROUP.to_owned()),
                                    ),
                                };
                                if ui.link(label).clicked() {
                                    acts.push(Act::Profile(ProfileAct::Reveal(sel)));
                                }
                                let link = views::RuleLink::Profile;
                                let text = issue.text.as_str();
                                views::linked_text(
                                    ui,
                                    &self.catalog,
                                    texts,
                                    text,
                                    p.msg_fg,
                                    link,
                                    acts,
                                );
                            });
                        }
                    });
            });
    }

    /// The status bar: the state of the document, the catalog, the MCP server, the last message.
    pub(crate) fn status_bar(&self, ui: &mut egui::Ui, acts: &mut Vec<Act>) {
        let p = Self::palette_now(ui);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 18.0;
            let state = match self.docs.get(self.active) {
                Some(Doc::Profile(d)) if d.dirty => tr("Modified"),
                Some(Doc::Profile(d)) if d.path.is_some() => tr("Saved"),
                Some(Doc::Profile(_)) => tr("Not saved yet"),
                _ => "",
            };
            if !state.is_empty() {
                ui.label(style::muted(ui, state, 11.0));
            }
            ui.label(style::muted(
                ui,
                trf(
                    "Catalog {version}, {n} rules",
                    &[
                        ("version", &self.catalog.version()),
                        ("n", &self.catalog.rules().len()),
                    ],
                ),
                11.0,
            ));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(run) = &self.mcp.run {
                    ui.label(
                        RichText::new(trf(
                            "MCP: {url}, mode {mode}",
                            &[
                                ("url", &run.http.url()),
                                ("mode", &run.server.options().mode.name()),
                            ],
                        ))
                        .size(style::size(ui, 11.0))
                        .color(p.ok),
                    );
                }
                // A file the message names, shown in its folder.
                if let Some((text, path)) = &self.shown_file
                    && text == &self.status
                {
                    let open = ui.small_button(tr("Open the folder"));
                    if open.on_hover_text(path.display().to_string()).clicked() {
                        acts.push(Act::Nav(NavAct::ShowInFolder(path.clone())));
                    }
                }
                ui.label(style::muted(ui, self.status.as_str(), 11.0));
                if self.pc_read.is_some() {
                    ui.add(egui::Spinner::new().size(style::size(ui, 12.0)));
                }
            });
        });
    }
}
