//! The data forms of a profile (`docs/spec/01-ui.md` section 6.3): Installation, Accounts and
//! Languages. A form draws a copy of its part of the profile and, when the copy changed, hands
//! it back as an [`Act`].

use eframe::egui;
use winweaver_core::profile::{Account, Install, Languages};
use winweaver_core::render;
use winweaver_core::resources;
use winweaver_core::validate;

use crate::i18n::{tr, trf};
use crate::{Act, ProfileAct, ProfileDoc, UsersAct};

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
                acts.push(Act::Profile(ProfileAct::SetInstall(Box::new(install))));
            }
        }
        Form::Accounts => {
            let mut accounts = doc.profile.accounts.clone();
            accounts_form(ui, &mut accounts, acts);
            if accounts != doc.profile.accounts {
                acts.push(Act::Profile(ProfileAct::SetAccounts(accounts)));
            }
        }
        Form::Languages => {
            let mut languages = doc.profile.languages.clone();
            languages_form(ui, &mut languages);
            if languages != doc.profile.languages {
                acts.push(Act::Profile(ProfileAct::SetLanguages(languages)));
            }
        }
    }
}

fn mode_row(ui: &mut egui::Ui, value: &mut String, options: &[(&str, &'static str)]) {
    for (id, title) in options {
        ui.radio_value(value, (*id).to_owned(), tr(title));
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
    ui.heading(tr(Form::Install.title()));
    ui.add_space(6.0);
    egui::Grid::new("install-form")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            ui.label(tr("Edition"));
            egui::ComboBox::from_id_salt("edition")
                .selected_text(i.edition.clone())
                .show_ui(ui, |ui| {
                    for edition in resources::editions() {
                        ui.selectable_value(&mut i.edition, edition.to_owned(), edition);
                    }
                });
            ui.end_row();

            ui.label(tr("Product key"));
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
                            .hint_text(tr("XXXXX-XXXXX-XXXXX-XXXXX-XXXXX"))
                            .desired_width(240.0),
                    );
                }
            });
            ui.end_row();

            ui.label(tr("Time zone"));
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

            ui.label(tr("Accounts"));
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

            ui.label(tr("Computer name"));
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

            ui.label(tr("Workgroup"));
            ui.add(
                egui::TextEdit::singleline(&mut i.workgroup)
                    .hint_text(tr("Empty: WORKGROUP"))
                    .desired_width(180.0),
            );
            ui.end_row();

            ui.label(tr("Disk"));
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
                    ui.label(tr("disk"));
                    number_field(ui, &mut i.disk_id, 0..=99);
                    ui.label(tr("partition"));
                    number_field(ui, &mut i.partition_id, 1..=128);
                }
            });
            ui.end_row();
        });
    ui.add_space(8.0);
    if i.computer_name_mode == "template" {
        name_template_help(ui, &i.computer_name);
        ui.add_space(8.0);
    }
    ui.weak(tr("The disk is not repartitioned: in the mode partition the chosen partition is installed over without a question."));
}

/// The placeholders of a computer name template with their lengths, and what the template gives
/// with sample values (`render::example_name`, the way the script of Setup expands it).
fn name_template_help(ui: &mut egui::Ui, template: &str) {
    if validate::is_name_template(template) {
        let name = render::example_name(template);
        let text = trf(
            "{template} gives {name}",
            &[("template", &template), ("name", &name)],
        );
        ui.strong(text);
        let sample = trf(
            "The example takes the serial number {serial} and the MAC address {mac}; a computer gives its own.",
            &[
                ("serial", &render::EXAMPLE_SERIAL),
                ("mac", &render::EXAMPLE_MAC),
            ],
        );
        ui.weak(sample);
    } else if !template.is_empty() {
        let warn = ui.visuals().warn_fg_color;
        ui.colored_label(
            warn,
            tr("This template cannot be expanded: it holds an unknown placeholder, a character other than Latin letters, digits and hyphens, or more than 15 characters."),
        );
    }
    ui.add_space(4.0);
    ui.strong(tr("The placeholders of the template"));
    let rows = [
        (
            "{serial}  {serial:N}",
            tr(
                "The serial number of the computer from its firmware, its letters and digits in capitals; :N keeps its last N characters, all of them (up to 15) without it.",
            ),
        ),
        (
            "{mac}  {mac:N}",
            tr(
                "The MAC address of the first physical network adapter, 12 hexadecimal digits in capitals; :N keeps its last N digits, all 12 without it.",
            ),
        ),
        (
            "{random}  {random:N}",
            tr(
                "Random capitals and digits without the look-alikes I, O, 0 and 1; :N gives N of them, 6 without it.",
            ),
        ),
    ];
    for (code, text) in rows {
        ui.horizontal_wrapped(|ui| {
            ui.monospace(code);
            ui.label(text);
        });
    }
    ui.weak(tr("N is from 1 to 15. A serial number or an address that cannot be read (there is none, or the maker left a placeholder such as Default string) becomes random characters of the same length; a shorter one stays whole."));
    ui.weak(tr("Around the placeholders: Latin letters, digits and hyphens. The name with the full length of each placeholder fits 15 characters, so PC-{serial} is too long and PC-{serial:12} fits. Keep a letter in it: Windows refuses a name of digits alone."));
    ui.add_space(4.0);
    ui.label(tr(
        "Examples with the serial number 5CG1234XYZ and the address 00155D3E5A01:",
    ));
    for example in ["PC-{random}", "WS-{serial:6}", "{mac:4}-LAB"] {
        let name = render::example_name(example);
        let text = trf(
            "{template} gives {name}",
            &[("template", &example), ("name", &name)],
        );
        ui.monospace(text);
    }
}

fn accounts_form(ui: &mut egui::Ui, accounts: &mut Vec<Account>, acts: &mut Vec<Act>) {
    ui.heading(tr("Accounts"));
    ui.weak(tr("The starter accounts without passwords are a decision of the customer: passwords and groups are set after installation. Names and passwords are ASCII; a display name outside ASCII is set after OOBE."));
    ui.add_space(6.0);
    let mut remove: Option<usize> = None;
    for (n, account) in accounts.iter_mut().enumerate() {
        ui.push_id(n, |ui| {
            egui::Grid::new("account")
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.label(tr("Name"));
                    ui.add(egui::TextEdit::singleline(&mut account.name).desired_width(200.0));
                    ui.end_row();
                    ui.label(tr("Display name"));
                    ui.add(
                        egui::TextEdit::singleline(&mut account.display_name).desired_width(200.0),
                    );
                    ui.end_row();
                    ui.label(tr("Group"));
                    egui::ComboBox::from_id_salt("group")
                        .selected_text(account.group.clone())
                        .show_ui(ui, |ui| {
                            for group in ["Administrators", "Users"] {
                                ui.selectable_value(&mut account.group, group.to_owned(), group);
                            }
                        });
                    ui.end_row();
                    ui.label(tr("Description"));
                    ui.add(
                        egui::TextEdit::singleline(&mut account.description).desired_width(320.0),
                    );
                    ui.end_row();
                    ui.label(tr("Password"));
                    ui.add(
                        egui::TextEdit::singleline(&mut account.password)
                            .password(true)
                            .desired_width(200.0),
                    );
                    ui.end_row();
                });
            if ui.small_button(tr("Remove this account")).clicked() {
                remove = Some(n);
            }
            ui.separator();
        });
    }
    if let Some(n) = remove {
        accounts.remove(n);
    }
    ui.horizontal(|ui| {
        if ui.button(tr("Add an account")).clicked() {
            accounts.push(Account {
                name: format!("User{}", accounts.len() + 1),
                display_name: String::new(),
                group: "Users".to_owned(),
                description: String::new(),
                password: String::new(),
            });
        }
        if ui
            .button(tr("From a users file..."))
            .on_hover_text(tr("Replaces the accounts with the ones of a .wwusers file, with their passwords when its master password is given"))
            .clicked()
        {
            acts.push(Act::Users(UsersAct::IntoProfile));
        }
        if ui
            .button(tr("To a users file..."))
            .on_hover_text(tr("Makes a users file of these accounts; their passwords wait for a master password"))
            .clicked()
        {
            acts.push(Act::Users(UsersAct::FromProfile));
        }
    });
}

/// A locale field: a tag typed freely, with the list of the languages of Windows beside it
/// (filtered by what is typed) that fills the tag in.
fn locale_field(ui: &mut egui::Ui, id: &str, value: &mut String, table: &resources::Locales) {
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(value).desired_width(120.0));
        if table.languages.is_empty() {
            return;
        }
        let known = table.language(value).map(|l| l.name.clone());
        let needle = if known.is_some() {
            String::new()
        } else {
            value.trim().to_lowercase()
        };
        let mut chosen = None;
        egui::ComboBox::from_id_salt(id)
            .selected_text(known.unwrap_or_default())
            .width(240.0)
            .height(360.0)
            .show_ui(ui, |ui| {
                for language in &table.languages {
                    if language.lcid.is_empty() {
                        continue;
                    }
                    let text = format!("{} {}", language.name, language.tag).to_lowercase();
                    if !text.contains(&needle) {
                        continue;
                    }
                    let on = language.tag.eq_ignore_ascii_case(value);
                    let label = format!("{} ({})", language.name, language.tag);
                    if ui.selectable_label(on, label).clicked() && !on {
                        chosen = Some(language.tag.clone());
                    }
                }
            });
        if let Some(tag) = chosen {
            *value = tag;
        }
    });
}

fn languages_form(ui: &mut egui::Ui, l: &mut Languages) {
    ui.heading(tr("Languages"));
    ui.add_space(6.0);
    let table = resources::locales();
    egui::Grid::new("languages-form")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            for (label, id, value) in [
                ("Display language", "ui-language", &mut l.ui_language),
                ("System locale", "system-locale", &mut l.system_locale),
                ("User locale", "user-locale", &mut l.user_locale),
            ] {
                ui.label(tr(label));
                locale_field(ui, id, value, table);
                ui.end_row();
            }
        });
    ui.weak(tr("The display language is never changed later: the installed Windows keeps the language of the answer file."));
    ui.add_space(8.0);
    ui.strong(tr("Input languages (the first is the default)"));
    if table.languages.is_empty() {
        ui.weak(tr("The languages and layouts of Windows were not read: the list keeps its items, and new ones cannot be chosen."));
    }
    let mut remove: Option<usize> = None;
    let mut up: Option<usize> = None;
    let mut change: Option<(usize, String)> = None;
    for (n, item) in l.input.iter().enumerate() {
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{}.", n + 1));
            match table.input(item) {
                Some((language, _)) => {
                    ui.label(language.name.as_str());
                    ui.weak(item.as_str());
                    if language.lcid.is_empty() {
                        ui.weak(tr("the layout Windows gives it at the first sign-in"));
                    } else {
                        let id = ("input-layout", n);
                        let own = own_layout(item);
                        if let Some(layout) = layout_list(ui, id, table, language, own) {
                            change = Some((n, input_item(language, layout.as_deref())));
                        }
                    }
                }
                None => {
                    let warn = ui.visuals().warn_fg_color;
                    let text = trf(
                        "{item}: Windows does not know this input language",
                        &[("item", item)],
                    );
                    ui.colored_label(warn, text);
                }
            }
            if n > 0 && ui.small_button(tr("up")).clicked() {
                up = Some(n);
            }
            if ui.small_button(tr("remove")).clicked() {
                remove = Some(n);
            }
        });
    }
    if let Some((n, item)) = change {
        l.input[n] = item;
    }
    if let Some(n) = up {
        l.input.swap(n, n - 1);
    }
    if let Some(n) = remove {
        l.input.remove(n);
    }
    add_input_row(ui, l, table);
}

/// The layout of its own an item names (`00000422` of `0422:00000422`), `None` for a tag.
fn own_layout(item: &str) -> Option<&str> {
    if validate::is_layout(item) {
        item.split_once(':').map(|(_, klid)| klid)
    } else {
        None
    }
}

/// The item of a language with a layout: its tag for the layout Windows gives it first, else
/// the layout id of the two.
fn input_item(language: &resources::InputLanguage, klid: Option<&str>) -> String {
    let first = own_layout(&language.default);
    match klid {
        Some(klid) if !first.is_some_and(|f| f.eq_ignore_ascii_case(klid)) => {
            format!("{}:{}", language.lcid, klid)
        }
        _ => language.tag.clone(),
    }
}

/// A list of the keyboard layouts of Windows for an input language: the one Windows gives it
/// first, then every layout by its name. `Some` with the choice when another one is chosen:
/// `None` inside for the first one, else the id of the layout.
fn layout_list(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    table: &resources::Locales,
    language: &resources::InputLanguage,
    own: Option<&str>,
) -> Option<Option<String>> {
    let name = table.layout_name(&language.default);
    let first = trf("{layout} (the default)", &[("layout", &name)]);
    let shown = match own {
        Some(klid) => table.layout_name(klid),
        None => first.clone(),
    };
    let mut chosen = None;
    egui::ComboBox::from_id_salt(id)
        .selected_text(shown)
        .width(240.0)
        .height(360.0)
        .show_ui(ui, |ui| {
            if ui.selectable_label(own.is_none(), first).clicked() && own.is_some() {
                chosen = Some(None);
            }
            for layout in &table.layouts {
                let on = own.is_some_and(|klid| klid.eq_ignore_ascii_case(&layout.klid));
                if ui.selectable_label(on, layout.name.as_str()).clicked() && !on {
                    chosen = Some(Some(layout.klid.clone()));
                }
            }
        });
    chosen
}

/// What the row that adds an input language holds from frame to frame.
#[derive(Clone, Default)]
struct AddInput {
    filter: String,
    tag: String,
    /// The layout chosen for it, `None` for the one Windows gives it first.
    klid: Option<String>,
}

/// The row that adds an input language: a search, the languages of Windows that match it, a
/// layout and the button.
fn add_input_row(ui: &mut egui::Ui, l: &mut Languages, table: &resources::Locales) {
    if table.languages.is_empty() {
        return;
    }
    let id = egui::Id::new("languages-add-input");
    let mut add: AddInput = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Add"));
        let search = egui::TextEdit::singleline(&mut add.filter)
            .hint_text(tr("Find a language"))
            .desired_width(140.0);
        ui.add(search);
        let needle = add.filter.to_lowercase();
        let chosen = table.language(&add.tag);
        let shown = chosen.map_or_else(|| tr("a language").to_owned(), |c| c.name.clone());
        egui::ComboBox::from_id_salt("add-input-language")
            .selected_text(shown)
            .width(240.0)
            .height(360.0)
            .show_ui(ui, |ui| {
                for language in &table.languages {
                    let usable = !language.default.is_empty() || !language.lcid.is_empty();
                    let text = format!("{} {}", language.name, language.tag).to_lowercase();
                    if !usable || !text.contains(&needle) {
                        continue;
                    }
                    let label = format!("{} ({})", language.name, language.tag);
                    if ui
                        .selectable_label(add.tag == language.tag, label)
                        .clicked()
                    {
                        add.tag = language.tag.clone();
                        add.klid = None;
                    }
                }
            });
        let Some(language) = table.language(&add.tag) else {
            return;
        };
        let own = add.klid.as_deref();
        if !language.lcid.is_empty()
            && let Some(choice) = layout_list(ui, "add-input-layout", table, language, own)
        {
            add.klid = choice;
        }
        let item = input_item(language, add.klid.as_deref());
        let fresh = !l.input.iter().any(|i| i.eq_ignore_ascii_case(&item));
        if ui
            .add_enabled(fresh, egui::Button::new(tr("Add the language")))
            .clicked()
        {
            l.input.push(item);
        }
    });
    ui.data_mut(|d| d.insert_temp(id, add));
}
