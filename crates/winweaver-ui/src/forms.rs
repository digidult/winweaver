//! The data forms of a profile (`docs/spec/01-ui.md` section 6.3): Installation, Accounts and
//! Languages. A form draws a copy of its part of the profile and, when the copy changed, hands
//! it back as an [`Act`].

use eframe::egui;
use winweaver_core::profile::{Account, Install, Languages};
use winweaver_core::resources::{self, EDITIONS};

use crate::{Act, ProfileDoc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Install,
    Accounts,
    Languages,
}

impl Form {
    pub const ALL: [Form; 3] = [Form::Install, Form::Accounts, Form::Languages];

    pub fn title(self) -> &'static str {
        match self {
            Form::Install => "Installation",
            Form::Accounts => "Accounts",
            Form::Languages => "Languages",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Form::Install => "form.install",
            Form::Accounts => "form.accounts",
            Form::Languages => "form.languages",
        }
    }
}

pub fn form_view(
    ui: &mut egui::Ui,
    doc: &ProfileDoc,
    form: Form,
    language: &str,
    acts: &mut Vec<Act>,
) {
    match form {
        Form::Install => {
            let mut install = doc.profile.install.clone();
            install_form(ui, &mut install, language);
            if install != doc.profile.install {
                acts.push(Act::SetInstall(install));
            }
        }
        Form::Accounts => {
            let mut accounts = doc.profile.accounts.clone();
            accounts_form(ui, &mut accounts);
            if accounts != doc.profile.accounts {
                acts.push(Act::SetAccounts(accounts));
            }
        }
        Form::Languages => {
            let mut languages = doc.profile.languages.clone();
            languages_form(ui, &mut languages);
            if languages != doc.profile.languages {
                acts.push(Act::SetLanguages(languages));
            }
        }
    }
}

fn mode_row(ui: &mut egui::Ui, value: &mut String, options: &[(&str, &str)]) {
    for (id, title) in options {
        ui.radio_value(value, (*id).to_owned(), *title);
    }
}

/// A number kept as text in the profile, edited as a number.
fn number_field(ui: &mut egui::Ui, value: &mut String, range: std::ops::RangeInclusive<u32>) {
    let mut n = value.parse::<u32>().unwrap_or(*range.start());
    if ui.add(egui::DragValue::new(&mut n).range(range)).changed() {
        *value = n.to_string();
    }
}

fn install_form(ui: &mut egui::Ui, i: &mut Install, language: &str) {
    ui.heading("Installation");
    ui.add_space(6.0);
    egui::Grid::new("install-form")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            ui.label("Edition");
            egui::ComboBox::from_id_salt("edition")
                .selected_text(i.edition.clone())
                .show_ui(ui, |ui| {
                    for edition in EDITIONS {
                        ui.selectable_value(&mut i.edition, edition.to_owned(), edition);
                    }
                });
            ui.end_row();

            ui.label("Product key");
            ui.horizontal(|ui| {
                mode_row(
                    ui,
                    &mut i.product_key_mode,
                    &[
                        ("generic", "Generic key of the edition"),
                        ("ask", "Ask during Setup"),
                        ("key", "This key"),
                    ],
                );
                if i.product_key_mode == "key" {
                    ui.add(
                        egui::TextEdit::singleline(&mut i.product_key)
                            .hint_text("XXXXX-XXXXX-XXXXX-XXXXX-XXXXX")
                            .desired_width(240.0),
                    );
                }
            });
            ui.end_row();

            ui.label("Time zone");
            let shown = resources::time_zones()
                .iter()
                .find(|z| z.id == i.time_zone)
                .map_or_else(|| i.time_zone.clone(), |z| z.title_for(language).to_owned());
            egui::ComboBox::from_id_salt("time-zone")
                .selected_text(shown)
                .width(320.0)
                .show_ui(ui, |ui| {
                    for zone in resources::time_zones() {
                        ui.selectable_value(
                            &mut i.time_zone,
                            zone.id.clone(),
                            zone.title_for(language),
                        );
                    }
                });
            ui.end_row();

            ui.label("Accounts");
            ui.horizontal(|ui| {
                mode_row(
                    ui,
                    &mut i.account_mode,
                    &[
                        ("file", "The accounts of this profile"),
                        ("ask", "Windows Setup asks for one"),
                    ],
                );
            });
            ui.end_row();

            ui.label("Computer name");
            ui.horizontal(|ui| {
                mode_row(
                    ui,
                    &mut i.computer_name_mode,
                    &[
                        ("random", "Random (Windows default)"),
                        ("fixed", "This name"),
                        ("template", "A template"),
                    ],
                );
                if i.computer_name_mode != "random" {
                    ui.add(
                        egui::TextEdit::singleline(&mut i.computer_name)
                            .hint_text(if i.computer_name_mode == "template" {
                                "PC-{random}"
                            } else {
                                "PC-01"
                            })
                            .desired_width(180.0),
                    );
                }
            });
            ui.end_row();

            ui.label("Workgroup");
            ui.add(
                egui::TextEdit::singleline(&mut i.workgroup)
                    .hint_text("Empty: WORKGROUP")
                    .desired_width(180.0),
            );
            ui.end_row();

            ui.label("Disk");
            ui.horizontal(|ui| {
                mode_row(
                    ui,
                    &mut i.disk_mode,
                    &[
                        ("ask", "Ask during Setup"),
                        ("partition", "Install to this partition"),
                    ],
                );
                if i.disk_mode == "partition" {
                    ui.label("disk");
                    number_field(ui, &mut i.disk_id, 0..=99);
                    ui.label("partition");
                    number_field(ui, &mut i.partition_id, 1..=128);
                }
            });
            ui.end_row();
        });
    ui.add_space(8.0);
    ui.weak("A template of the computer name may hold {serial}, {mac} and {random}; the expanded name fits 15 characters. The disk is not repartitioned: in the mode partition the chosen partition is installed over without a question.");
}

fn accounts_form(ui: &mut egui::Ui, accounts: &mut Vec<Account>) {
    ui.heading("Accounts");
    ui.weak("The starter accounts without passwords are a decision of the customer: passwords and groups are set after installation. Names and passwords are ASCII; a display name outside ASCII is set after OOBE.");
    ui.add_space(6.0);
    let mut remove: Option<usize> = None;
    for (n, account) in accounts.iter_mut().enumerate() {
        ui.push_id(n, |ui| {
            egui::Grid::new("account")
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Name");
                    ui.add(egui::TextEdit::singleline(&mut account.name).desired_width(200.0));
                    ui.end_row();
                    ui.label("Display name");
                    ui.add(
                        egui::TextEdit::singleline(&mut account.display_name).desired_width(200.0),
                    );
                    ui.end_row();
                    ui.label("Group");
                    egui::ComboBox::from_id_salt("group")
                        .selected_text(account.group.clone())
                        .show_ui(ui, |ui| {
                            for group in ["Administrators", "Users"] {
                                ui.selectable_value(&mut account.group, group.to_owned(), group);
                            }
                        });
                    ui.end_row();
                    ui.label("Description");
                    ui.add(
                        egui::TextEdit::singleline(&mut account.description).desired_width(320.0),
                    );
                    ui.end_row();
                    ui.label("Password");
                    ui.add(
                        egui::TextEdit::singleline(&mut account.password)
                            .password(true)
                            .desired_width(200.0),
                    );
                    ui.end_row();
                });
            if ui.small_button("Remove this account").clicked() {
                remove = Some(n);
            }
            ui.separator();
        });
    }
    if let Some(n) = remove {
        accounts.remove(n);
    }
    if ui.button("Add an account").clicked() {
        accounts.push(Account {
            name: format!("User{}", accounts.len() + 1),
            display_name: String::new(),
            group: "Users".to_owned(),
            description: String::new(),
            password: String::new(),
        });
    }
}

fn languages_form(ui: &mut egui::Ui, l: &mut Languages) {
    ui.heading("Languages");
    ui.add_space(6.0);
    egui::Grid::new("languages-form")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            for (label, value) in [
                ("Display language", &mut l.ui_language),
                ("System locale", &mut l.system_locale),
                ("User locale", &mut l.user_locale),
            ] {
                ui.label(label);
                ui.add(egui::TextEdit::singleline(value).desired_width(120.0));
                ui.end_row();
            }
        });
    ui.weak("The display language is never changed later: the installed Windows keeps the language of the answer file.");
    ui.add_space(8.0);
    ui.strong("Input languages (the first is the default)");
    let mut remove: Option<usize> = None;
    let mut up: Option<usize> = None;
    for (n, tag) in l.input.iter().enumerate() {
        ui.horizontal(|ui| {
            let title = resources::keyboard(tag).map_or("", |k| k.title.as_str());
            ui.label(format!("{}. {tag}", n + 1));
            ui.weak(title);
            if n > 0 && ui.small_button("up").clicked() {
                up = Some(n);
            }
            if ui.small_button("remove").clicked() {
                remove = Some(n);
            }
        });
    }
    if let Some(n) = up {
        l.input.swap(n, n - 1);
    }
    if let Some(n) = remove {
        l.input.remove(n);
    }
    ui.horizontal(|ui| {
        ui.label("Add");
        egui::ComboBox::from_id_salt("add-input")
            .selected_text("an input language")
            .width(300.0)
            .show_ui(ui, |ui| {
                for keyboard in resources::keyboards() {
                    if l.input.contains(&keyboard.tag) {
                        continue;
                    }
                    let label = format!("{} ({}, {})", keyboard.title, keyboard.tag, keyboard.klid);
                    if ui.selectable_label(false, label).clicked() {
                        l.input.push(keyboard.tag.clone());
                    }
                }
            });
    });
}
