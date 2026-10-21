//! The answer file: the rules in effect collected by pass and by script, the scripts, the three
//! passes of Windows Setup, the extensions and the header, assembled from the templates.

use std::collections::BTreeMap;

use crate::catalog::{Action, Catalog, Phase};
use crate::profile::Profile;
use crate::resolver;
use crate::resources;

use super::accounts::{account_text_lines, accounts_xml};
use super::languages::{InputLanguages, input_languages};
use super::name::computer_name_parts;
use super::params::{fill, params_of, substitute};
use super::ps::{block, join_blocks};
use super::text::{ascii_json, comment_text, ps_list, xml_escape};
use super::xml::{Command, Setting, command_lines, component, setting_lines};
use super::{
    ARCHITECTURES, ASK_KEY, Build, MAX_COMMAND, POST_OOBE, POST_OOBE_NAME, RenderError, SCRIPTS_DIR,
    SECTION_ACTIVE_SETUP, SECTION_COMPUTER_NAME, SECTION_DEFAULT_USER, SECTION_POST_OOBE_TASK,
    SETUP_SYSTEM, SETUP_SYSTEM_NAME, SETUP_USER, SETUP_USER_NAME, TEMP_COMPUTER_NAME, TEMPLATE_XML,
    runtime_version,
};

/// What the rules in effect give the answer file, by pass and by script.
#[derive(Default)]
struct Collected {
    rule_ids: Vec<String>,
    pe_commands: Vec<Command>,
    specialize_commands: Vec<Command>,
    oobe_elements: Vec<(String, String)>,
    settings: BTreeMap<(String, String), Vec<Setting>>,
    system_blocks: Vec<String>,
    du_blocks: Vec<String>,
    user_blocks: Vec<String>,
    post_blocks: Vec<String>,
}

/// The components a pass gets beyond the fixed ones, by pass: the component and its lines.
type Extra = BTreeMap<String, Vec<(String, Vec<String>)>>;

/// The rules in effect: their commands, settings and OOBE elements for the passes, the blocks of
/// their script actions for the scripts.
fn collect(catalog: &Catalog, profile: &Profile) -> Result<Collected, RenderError> {
    let resolution = resolver::resolve(catalog, profile);
    let mut c = Collected::default();
    for rule in catalog.rules() {
        if !resolution.in_effect(&rule.id) {
            continue;
        }
        c.rule_ids.push(rule.id.clone());
        let params = params_of(catalog, profile, rule);
        for action in &rule.actions {
            match action {
                Action::XmlPeCommand {
                    command,
                    description,
                } => c.pe_commands.push(Command {
                    description: description.clone(),
                    path: substitute(command, &params),
                }),
                Action::XmlSpecializeCommand {
                    command,
                    description,
                } => c.specialize_commands.push(Command {
                    description: description.clone(),
                    path: substitute(command, &params),
                }),
                Action::XmlOobe { element, value } => {
                    c.oobe_elements
                        .push((element.clone(), substitute(value, &params)));
                }
                Action::XmlSetting {
                    pass,
                    component,
                    path,
                    value,
                } => {
                    let key = (pass.clone(), component.clone());
                    c.settings.entry(key).or_default().push(Setting {
                        path: path.split('/').map(str::to_owned).collect(),
                        value: substitute(value, &params),
                    });
                }
                _ => {}
            }
        }
        if let Some(b) = block(rule, &params)? {
            match rule.phase {
                Phase::Specialize | Phase::Windowspe | Phase::SpecializeXml | Phase::OobeXml => {
                    c.system_blocks.push(b);
                }
                Phase::DefaultUser => c.du_blocks.push(b),
                Phase::UserFirstLogon => c.user_blocks.push(b),
                Phase::PostOobe => c.post_blocks.push(b),
            }
        }
    }
    Ok(c)
}

/// The collected rules and the parts of the profile every pass and script shares.
struct Assembly<'a> {
    catalog: &'a Catalog,
    profile: &'a Profile,
    app_version: &'a str,
    rules: Collected,
    languages: InputLanguages,
    account_texts: Vec<String>,
    name_parts: Vec<String>,
    has_system: bool,
    has_user: bool,
    has_post: bool,
}

/// Builds the answer file of a profile with the embedded catalog data.
pub fn build(
    catalog: &Catalog,
    profile: &Profile,
    app_version: &str,
) -> Result<Build, RenderError> {
    let rules = collect(catalog, profile)?;
    let install = &profile.install;
    let languages = input_languages(profile)?;
    let account_texts = if install.account_mode == "file" {
        account_text_lines(profile)
    } else {
        Vec::new()
    };
    let name_parts = if install.computer_name_mode == "template" {
        computer_name_parts(&install.computer_name)
    } else {
        Vec::new()
    };
    let has_user = !rules.user_blocks.is_empty();
    let has_post = !rules.post_blocks.is_empty() || !account_texts.is_empty();
    let has_system = !rules.system_blocks.is_empty()
        || !rules.du_blocks.is_empty()
        || has_user
        || has_post
        || !name_parts.is_empty();
    let mut parts = Assembly {
        catalog,
        profile,
        app_version,
        rules,
        languages,
        account_texts,
        name_parts,
        has_system,
        has_user,
        has_post,
    };
    let scripts = parts.scripts()?;
    let settings_xml = parts.settings_xml()?;
    let (files, profile_json) = parts.extensions(&scripts);
    let header = parts.header(&scripts)?;
    let xml = fill(
        TEMPLATE_XML,
        &[
            ("header", &header),
            ("settings", &settings_xml.join("\n")),
            ("files", &files.join("\n")),
            ("profile_json", profile_json.trim_end()),
        ],
    )?;
    if let Some(c) = xml.chars().find(|c| !c.is_ascii()) {
        return Err(RenderError::Profile(format!(
            "the answer file must be ASCII; the character {c:?} (U+{:04X}) came from the profile",
            c as u32
        )));
    }
    let mut warnings = Vec::new();
    if profile.accounts.iter().any(|a| !a.password.is_empty()) {
        warnings.push("The answer file holds the account passwords in plain text".to_owned());
    }
    Ok(Build {
        xml: xml.replace("\r\n", "\n").replace('\n', "\r\n"),
        scripts,
        rule_ids: parts.rules.rule_ids,
        warnings,
    })
}

impl Assembly<'_> {
    /// The embedded scripts: Setup-System, Setup-User and Post-OOBE, each when it has something
    /// to do.
    fn scripts(&self) -> Result<Vec<(String, String)>, RenderError> {
        let label = format!(
            "WinWeaver {}, catalog {}, {} rules enabled",
            self.app_version,
            self.catalog.version(),
            self.rules.rule_ids.len()
        );
        let mut scripts = Vec::new();
        if self.has_system {
            scripts.push((SETUP_SYSTEM_NAME.to_owned(), self.setup_system(&label)?));
        }
        if self.has_user {
            let l = &self.languages;
            let text = fill(
                SETUP_USER,
                &[
                    ("input_languages", &ps_list(&l.tags)),
                    ("input_tips", &ps_list(&l.tips)),
                    ("input_fallback", &ps_list(&l.fallback)),
                    ("transient_languages", &ps_list(&l.transient)),
                    ("blocks", &join_blocks(&self.rules.user_blocks, 0)),
                ],
            )?;
            scripts.push((SETUP_USER_NAME.to_owned(), text));
        }
        if self.has_post {
            let names: Vec<String> = if self.profile.install.account_mode == "file" {
                self.profile.accounts.iter().map(|a| a.name.clone()).collect()
            } else {
                Vec::new()
            };
            let text = fill(
                POST_OOBE,
                &[
                    ("accounts", &ps_list(&names)),
                    ("account_texts", &self.account_texts.join("\n")),
                    ("blocks", &join_blocks(&self.rules.post_blocks, 0)),
                ],
            )?;
            scripts.push((POST_OOBE_NAME.to_owned(), text));
        }
        for (name, text) in &scripts {
            if text.contains("]]>") {
                return Err(RenderError::Template(format!("{name} holds ]]>")));
            }
        }
        Ok(scripts)
    }

    /// Setup-System.ps1: the computer name, the blocks of the machine, the default user profile,
    /// the Active Setup entry of Setup-User and the scheduled task of Post-OOBE.
    fn setup_system(&self, label: &str) -> Result<String, RenderError> {
        let name_section = if self.name_parts.is_empty() {
            String::new()
        } else {
            fill(
                SECTION_COMPUTER_NAME,
                &[("name_parts", &ps_list(&self.name_parts))],
            )?
        };
        let du_section = if self.rules.du_blocks.is_empty() {
            String::new()
        } else {
            fill(
                SECTION_DEFAULT_USER,
                &[("blocks", &join_blocks(&self.rules.du_blocks, 4))],
            )?
        };
        let active = if self.has_user {
            SECTION_ACTIVE_SETUP.replace("\r\n", "\n")
        } else {
            String::new()
        };
        let task = if self.has_post {
            SECTION_POST_OOBE_TASK.replace("\r\n", "\n")
        } else {
            String::new()
        };
        fill(
            SETUP_SYSTEM,
            &[
                ("build_label", label),
                ("computer_name_section", &name_section),
                ("blocks", &join_blocks(&self.rules.system_blocks, 0)),
                ("default_user_section", &du_section),
                ("active_setup_section", &active),
                ("post_oobe_section", &task),
            ],
        )
    }

    /// The settings of the rules for a component of a pass, as lines, taken out of the collected
    /// ones; the components left over go to `extra_components`.
    fn settings_of(&mut self, pass: &str, component: &str) -> Result<Vec<String>, RenderError> {
        let key = (pass.to_owned(), component.to_owned());
        match self.rules.settings.remove(&key) {
            Some(list) => setting_lines(&list, 6),
            None => Ok(Vec::new()),
        }
    }

    /// The three passes of the answer file, each component for both architectures.
    fn settings_xml(&mut self) -> Result<Vec<String>, RenderError> {
        let pe_body = self.windows_pe()?;
        let (deployment, shell, join) = self.specialize()?;
        let (international, shell_oobe) = self.oobe_system()?;
        let extra = self.extra_components()?;
        let extras = |pass: &str, arch: &str| -> Vec<String> {
            let mut out = Vec::new();
            if let Some(list) = extra.get(pass) {
                for (comp, body) in list {
                    out.extend(component(comp, arch, body));
                }
            }
            out
        };
        let mut out = vec!["  <settings pass=\"windowsPE\">".to_owned()];
        for arch in ARCHITECTURES {
            out.extend(component("Microsoft-Windows-Setup", arch, &pe_body));
            out.extend(extras("windowsPE", arch));
        }
        out.push("  </settings>".to_owned());
        out.push("  <settings pass=\"specialize\">".to_owned());
        for arch in ARCHITECTURES {
            if !deployment.is_empty() {
                out.extend(component("Microsoft-Windows-Deployment", arch, &deployment));
            }
            out.extend(component("Microsoft-Windows-Shell-Setup", arch, &shell));
            if !join.is_empty() {
                out.extend(component("Microsoft-Windows-UnattendedJoin", arch, &join));
            }
            out.extend(extras("specialize", arch));
        }
        out.push("  </settings>".to_owned());
        out.push("  <settings pass=\"oobeSystem\">".to_owned());
        for arch in ARCHITECTURES {
            out.extend(component(
                "Microsoft-Windows-International-Core",
                arch,
                &international,
            ));
            if !shell_oobe.is_empty() {
                out.extend(component("Microsoft-Windows-Shell-Setup", arch, &shell_oobe));
            }
            out.extend(extras("oobeSystem", arch));
        }
        out.push("  </settings>".to_owned());
        Ok(out)
    }

    /// The pass windowsPE: the disk, the product key and the commands of the rules.
    fn windows_pe(&mut self) -> Result<Vec<String>, RenderError> {
        let profile = self.profile;
        let install = &profile.install;
        let key = match install.product_key_mode.as_str() {
            "generic" => resources::generic_key(&install.edition)
                .ok_or_else(|| {
                    RenderError::Profile(format!(
                        "no generic key for the edition {}",
                        install.edition
                    ))
                })?
                .to_owned(),
            "ask" => ASK_KEY.to_owned(),
            "key" => install.product_key.trim().to_uppercase(),
            other => {
                return Err(RenderError::Profile(format!(
                    "unknown product key mode {other}"
                )));
            }
        };
        let will_show = if install.product_key_mode == "ask" {
            "Always"
        } else {
            "OnError"
        };
        let mut body = Vec::new();
        if install.disk_mode == "partition" {
            body.extend([
                "      <ImageInstall>".to_owned(),
                "        <OSImage>".to_owned(),
                "          <InstallTo>".to_owned(),
                format!(
                    "            <DiskID>{}</DiskID>",
                    xml_escape(&install.disk_id)
                ),
                format!(
                    "            <PartitionID>{}</PartitionID>",
                    xml_escape(&install.partition_id)
                ),
                "          </InstallTo>".to_owned(),
                "          <WillShowUI>OnError</WillShowUI>".to_owned(),
                "        </OSImage>".to_owned(),
                "      </ImageInstall>".to_owned(),
            ]);
        }
        body.extend([
            "      <UserData>".to_owned(),
            "        <ProductKey>".to_owned(),
            format!("          <Key>{}</Key>", xml_escape(&key)),
            format!("          <WillShowUI>{will_show}</WillShowUI>"),
            "        </ProductKey>".to_owned(),
            "        <AcceptEula>true</AcceptEula>".to_owned(),
            "      </UserData>".to_owned(),
        ]);
        body.extend(command_lines(&self.rules.pe_commands, 6)?);
        body.extend(self.settings_of("windowsPE", "Microsoft-Windows-Setup")?);
        Ok(body)
    }

    /// The pass specialize: the deployment commands (the scripts extracted and Setup-System run),
    /// the time zone and the computer name, the workgroup.
    fn specialize(&mut self) -> Result<(Vec<String>, Vec<String>, Vec<String>), RenderError> {
        let profile = self.profile;
        let install = &profile.install;
        let mut commands = Vec::new();
        if self.has_system {
            commands.push(Command {
                description: "Extract embedded scripts from this answer file".to_owned(),
                path: "powershell.exe -NoProfile -WindowStyle Hidden -Command \"try{$x=[xml]::new();$x.Load('C:\\Windows\\Panther\\unattend.xml');$s=[scriptblock]::Create($x.unattend.Extensions.ExtractScript);icm $s -Args $x}catch{$_>C:\\Windows\\Temp\\ua.err};exit 0\"".to_owned(),
            });
        }
        commands.append(&mut self.rules.specialize_commands);
        if self.has_system {
            commands.push(Command {
                description: "Apply machine-wide settings".to_owned(),
                path: format!(
                    "powershell.exe -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command \"try{{$p='{SCRIPTS_DIR}\\{SETUP_SYSTEM_NAME}';if(Test-Path $p){{& $p}}else{{'no script'>C:\\Windows\\Temp\\ua.err}}}}catch{{$_>C:\\Windows\\Temp\\ua.err}};exit 0\""
                ),
            });
        }
        let mut deployment = command_lines(&commands, 6)?;
        deployment.extend(self.settings_of("specialize", "Microsoft-Windows-Deployment")?);
        let mut shell = vec![format!(
            "      <TimeZone>{}</TimeZone>",
            xml_escape(&install.time_zone)
        )];
        match install.computer_name_mode.as_str() {
            "fixed" => shell.push(format!(
                "      <ComputerName>{}</ComputerName>",
                xml_escape(install.computer_name.trim())
            )),
            "template" => shell.push(format!(
                "      <ComputerName>{TEMP_COMPUTER_NAME}</ComputerName>"
            )),
            _ => {}
        }
        shell.extend(self.settings_of("specialize", "Microsoft-Windows-Shell-Setup")?);
        let mut join = Vec::new();
        if !install.workgroup.trim().is_empty() {
            join.extend([
                "      <Identification>".to_owned(),
                format!(
                    "        <JoinWorkgroup>{}</JoinWorkgroup>",
                    xml_escape(install.workgroup.trim())
                ),
                "      </Identification>".to_owned(),
            ]);
        }
        join.extend(self.settings_of("specialize", "Microsoft-Windows-UnattendedJoin")?);
        Ok((deployment, shell, join))
    }

    /// The pass oobeSystem: the languages, the OOBE elements of the rules and the accounts.
    fn oobe_system(&mut self) -> Result<(Vec<String>, Vec<String>), RenderError> {
        let profile = self.profile;
        let l = &profile.languages;
        let mut international = vec![
            format!(
                "      <InputLocale>{}</InputLocale>",
                xml_escape(&self.languages.input_locale)
            ),
            format!(
                "      <SystemLocale>{}</SystemLocale>",
                xml_escape(&l.system_locale)
            ),
            format!(
                "      <UILanguage>{}</UILanguage>",
                xml_escape(&l.ui_language)
            ),
            format!(
                "      <UserLocale>{}</UserLocale>",
                xml_escape(&l.user_locale)
            ),
        ];
        international.extend(self.settings_of("oobeSystem", "Microsoft-Windows-International-Core")?);
        let mut shell = Vec::new();
        if !self.rules.oobe_elements.is_empty() {
            shell.push("      <OOBE>".to_owned());
            for (element, value) in &self.rules.oobe_elements {
                shell.push(format!(
                    "        <{element}>{}</{element}>",
                    xml_escape(value)
                ));
            }
            shell.push("      </OOBE>".to_owned());
        }
        if profile.install.account_mode == "file" {
            shell.extend(accounts_xml(profile));
        }
        shell.extend(self.settings_of("oobeSystem", "Microsoft-Windows-Shell-Setup")?);
        Ok((international, shell))
    }

    /// The components of the passes the fixed ones do not write.
    fn extra_components(&self) -> Result<Extra, RenderError> {
        let mut extra = Extra::new();
        for ((pass, comp), list) in &self.rules.settings {
            if !matches!(pass.as_str(), "windowsPE" | "specialize" | "oobeSystem") {
                return Err(RenderError::Template(format!("unknown pass {pass}")));
            }
            extra
                .entry(pass.clone())
                .or_default()
                .push((comp.clone(), setting_lines(list, 6)?));
        }
        Ok(extra)
    }

    /// The embedded files of the Extensions, and the profile as ASCII JSON.
    fn extensions(&self, scripts: &[(String, String)]) -> (Vec<String>, String) {
        let mut files = Vec::new();
        for (name, text) in scripts {
            files.push(format!(
                "    <File path=\"{SCRIPTS_DIR}\\{name}\"><![CDATA["
            ));
            files.push(text.trim_end().to_owned());
            files.push("]]></File>".to_owned());
        }
        let mut profile_json = ascii_json(&self.profile.to_json());
        if profile_json.contains("]]>") {
            profile_json = profile_json.replace("]]>", "]]\\u003e");
        }
        (files, profile_json)
    }

    /// The comment at the top of the answer file.
    fn header(&self, scripts: &[(String, String)]) -> Result<String, RenderError> {
        let mut header = vec![
            format!(
                "    Generated by WinWeaver {}, rules catalog {}, runtime {}.",
                self.app_version,
                self.catalog.version(),
                runtime_version()
            ),
            format!(
                "    Profile: {}; modified {}.",
                comment_text(&self.profile.name),
                comment_text(&self.profile.modified)
            ),
            format!(
                "    Enabled rules: {} of {}. Disabled rules are absent from this file.",
                self.rules.rule_ids.len(),
                self.catalog.rules().len()
            ),
            "    The profile is embedded in Extensions/Profile and can be opened again in WinWeaver."
                .to_owned(),
            String::new(),
            "    Hard limits of Windows Setup respected by this file (a violation aborts the install"
                .to_owned(),
            "    with \"The provided unattend file is not valid\", 0x80220005):".to_owned(),
            format!("      - RunSynchronousCommand/Path is {MAX_COMMAND} characters or shorter;"),
            "      - no XML comments inside <component> blocks;".to_owned(),
            "      - every synchronous command exits with code 0.".to_owned(),
        ];
        if !scripts.is_empty() {
            header.push(String::new());
            header.push("    Post-install artefacts:".to_owned());
            for (name, _) in scripts {
                header.push(format!("      {SCRIPTS_DIR}\\{name}"));
            }
            header.push("      C:\\ProgramData\\Unattend\\Logs\\*.log".to_owned());
        }
        let text = header.join("\n");
        if text.contains("--") {
            return Err(RenderError::Template(
                "the header comment may not hold two hyphens".into(),
            ));
        }
        Ok(text)
    }
}
