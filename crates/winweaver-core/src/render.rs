//! The build of `autounattend.xml` from a profile (`docs/spec/03-features.md` section 3).
//!
//! The runtime templates of `data/templates/` (carried over from the predecessor as data) hold the
//! fixed parts; this module fills their `{{slot}}`s with the passes, the blocks of the rules in
//! effect and the profile data, and embeds the scripts and the profile into the `Extensions`.
//! Everything the generator itself writes is ASCII.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::catalog::{Action, Catalog, ListNames, Phase, RegKind, Rule};
use crate::profile::{Account, Profile};
use crate::resolver;
use crate::resources;

const TEMPLATE_XML: &str = include_str!("../../../data/templates/autounattend.template.xml");
const SETUP_SYSTEM: &str = include_str!("../../../data/templates/Setup-System.runtime.ps1");
const SETUP_USER: &str = include_str!("../../../data/templates/Setup-User.runtime.ps1");
const POST_OOBE: &str = include_str!("../../../data/templates/Post-OOBE.runtime.ps1");
const SECTION_DEFAULT_USER: &str = include_str!("../../../data/templates/section-default-user.ps1");
const SECTION_ACTIVE_SETUP: &str = include_str!("../../../data/templates/section-active-setup.ps1");
const SECTION_POST_OOBE_TASK: &str =
    include_str!("../../../data/templates/section-post-oobe-task.ps1");
const SECTION_COMPUTER_NAME: &str =
    include_str!("../../../data/templates/section-computer-name.ps1");
const RUNTIME_VERSION_FILE: &str = include_str!("../../../data/templates/VERSION");

/// The key of the mode `ask`: Windows Setup shows the key and edition dialog.
pub const ASK_KEY: &str = "00000-00000-00000-00000-00000";
/// The temporary computer name of a template; `section-computer-name.ps1` replaces it.
pub const TEMP_COMPUTER_NAME: &str = "WINWEAVER-TMP";
/// The longest `RunSynchronousCommand/Path` Windows Setup accepts.
pub const MAX_COMMAND: usize = 259;
const ARCHITECTURES: [&str; 2] = ["amd64", "arm64"];
const SCRIPTS_DIR: &str = r"C:\ProgramData\Unattend\Scripts";
const SETUP_SYSTEM_NAME: &str = "Setup-System.ps1";
const SETUP_USER_NAME: &str = "Setup-User.ps1";
const POST_OOBE_NAME: &str = "Post-OOBE.ps1";

/// The version of the runtime templates.
pub fn runtime_version() -> &'static str {
    RUNTIME_VERSION_FILE.trim()
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("rule {rule}: {text}")]
    Rule { rule: String, text: String },
    #[error("{0}")]
    Profile(String),
    #[error("the command of {what} is {len} characters, more than {MAX_COMMAND}: {command}")]
    CommandTooLong {
        what: String,
        len: usize,
        command: String,
    },
    #[error("{0}")]
    Template(String),
}

/// The result of a build.
#[derive(Debug, Clone)]
pub struct Build {
    /// The answer file, CRLF, ASCII.
    pub xml: String,
    /// The embedded scripts by file name, in the order of the `Extensions`.
    pub scripts: Vec<(String, String)>,
    /// The rules in effect, in the order they were applied.
    pub rule_ids: Vec<String>,
    pub warnings: Vec<String>,
}

fn err(rule: &Rule, text: impl Into<String>) -> RenderError {
    RenderError::Rule {
        rule: rule.id.clone(),
        text: text.into(),
    }
}

/// A PowerShell single-quoted literal: quotes doubled, also the typographic ones PowerShell
/// treats as quotes.
pub fn ps_quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for c in text.chars() {
        if c == '\'' || ('\u{2018}'..='\u{201B}').contains(&c) {
            out.push(c);
        }
        out.push(c);
    }
    out.push('\'');
    out
}

/// Text inside an XML element.
pub fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// JSON text with every character outside ASCII escaped as `\uXXXX`.
pub fn ascii_json(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0u16; 2];
            for unit in c.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}

/// Base64 of the UTF-8 bytes of a text (standard alphabet with padding).
pub fn base64(text: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.len();
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let v = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(ALPHABET[((v >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((v >> 12) & 63) as usize] as char);
        out.push(if n > 1 {
            ALPHABET[((v >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if n > 2 {
            ALPHABET[(v & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// The text of a parameter value inside a placeholder.
pub(crate) fn value_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// `{name}` placeholders replaced by the parameter values; other braces stay.
pub fn substitute(text: &str, params: &BTreeMap<String, Value>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) if params.contains_key(&after[..end]) => {
                out.push_str(&value_text(&params[&after[..end]]));
                rest = &after[end + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The items of a list value: the texts of an array, trimmed, without the empty ones.
pub(crate) fn list_items(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The items of the list parameter of a `reg-list`; none for an empty parameter name.
pub(crate) fn list_values(params: &BTreeMap<String, Value>, param: &str) -> Vec<String> {
    if param.is_empty() {
        return Vec::new();
    }
    list_items(params.get(param))
}

/// The value names and the data a `reg-list` writes for its items; an error for a pair without
/// `=` or a name that comes twice (Windows compares value names without regard to case).
pub(crate) fn list_entries(
    names: ListNames,
    prefix: &str,
    items: &[String],
) -> Result<(Vec<String>, Vec<String>), String> {
    let mut out_names: Vec<String> = Vec::new();
    let mut out_values = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let (name, value) = match names {
            ListNames::Items => (item.clone(), item.clone()),
            ListNames::Numbered => (format!("{prefix}{}", i + 1), item.clone()),
            ListNames::Pairs => match item.split_once('=') {
                Some((n, v)) if !n.trim().is_empty() => (n.trim().to_owned(), v.trim().to_owned()),
                _ => return Err(format!("the item {item} is not name=value")),
            },
        };
        if out_names.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
            return Err(format!("the value name {name} comes twice"));
        }
        out_names.push(name);
        out_values.push(value);
    }
    Ok((out_names, out_values))
}

/// A value of an action with its placeholders filled in: a value that is one list parameter
/// alone becomes the list of its items (the lines of a multi-line value).
pub(crate) fn fill_value(value: &Value, params: &BTreeMap<String, Value>) -> Value {
    match value {
        Value::String(text) => {
            if let Some(inner) = text.strip_prefix('{').and_then(|t| t.strip_suffix('}'))
                && let Some(list @ Value::Array(_)) = params.get(inner)
            {
                return Value::from(list_items(Some(list)));
            }
            Value::String(substitute(text, params))
        }
        Value::Array(items) => Value::Array(items.iter().map(|v| fill_value(v, params)).collect()),
        other => other.clone(),
    }
}

pub(crate) fn fill(template: &str, slots: &[(&str, &str)]) -> Result<String, RenderError> {
    let mut text = template.replace("\r\n", "\n");
    for (name, value) in slots {
        text = text.replace(&format!("{{{{{name}}}}}"), value);
    }
    if let Some(i) = text.find("{{") {
        let end = text[i..].find("}}").map_or(text.len(), |e| i + e + 2);
        return Err(RenderError::Template(format!(
            "a slot stayed unfilled: {}",
            &text[i..end]
        )));
    }
    Ok(text)
}

pub(crate) fn reg_path(path: &str) -> String {
    match path.strip_prefix("DU:") {
        Some(rest) => format!("($du + {})", ps_quote(rest)),
        None => ps_quote(path),
    }
}

pub(crate) fn reg_value(rule: &Rule, kind: RegKind, value: &Value) -> Result<String, RenderError> {
    let number = |v: &Value| -> Result<String, RenderError> {
        match v {
            Value::Number(n) => Ok(n.to_string()),
            Value::String(s) if s.trim().parse::<i64>().is_ok() => Ok(s.trim().to_owned()),
            Value::Bool(b) => Ok(if *b { "1" } else { "0" }.to_owned()),
            other => Err(err(rule, format!("the value {other} is not a number"))),
        }
    };
    match kind {
        RegKind::DWord | RegKind::QWord => number(value),
        RegKind::String | RegKind::ExpandString => Ok(ps_quote(&value_text(value))),
        RegKind::MultiString => match value {
            Value::Array(items) => {
                let parts: Vec<String> = items.iter().map(|v| ps_quote(&value_text(v))).collect();
                Ok(format!("@({})", parts.join(",")))
            }
            Value::String(s) if s.is_empty() => Ok("@()".to_owned()),
            Value::String(s) => Ok(format!("@({})", ps_quote(s))),
            other => Err(err(
                rule,
                format!("the value {other} is not a list of texts"),
            )),
        },
        RegKind::Binary => match value {
            Value::Array(items) => {
                let parts: Result<Vec<String>, RenderError> = items.iter().map(number).collect();
                Ok(format!("([byte[]]@({}))", parts?.join(",")))
            }
            other => Err(err(
                rule,
                format!("the value {other} is not a list of bytes"),
            )),
        },
    }
}

/// The script lines of one action.
pub(crate) fn script_lines(
    rule: &Rule,
    action: &Action,
    params: &BTreeMap<String, Value>,
    phase: Phase,
) -> Result<Vec<String>, RenderError> {
    let limited = matches!(phase, Phase::UserFirstLogon | Phase::PostOobe);
    let s = |text: &str| substitute(text, params);
    let line = match action {
        Action::Reg {
            path,
            name,
            kind,
            value,
            why,
            ..
        } => {
            let value = fill_value(value, params);
            let mut text = format!(
                "Set-Reg -Path {} -Name {} -Type {kind:?} -Value {}",
                reg_path(path),
                ps_quote(name),
                reg_value(rule, *kind, &value)?
            );
            if !why.is_empty() {
                text.push_str(&format!(" -Why {}", ps_quote(why)));
            }
            text
        }
        Action::RegRemove { path, name, .. } => {
            format!(
                "Remove-Reg -Path {} -Name {}",
                reg_path(path),
                ps_quote(name)
            )
        }
        Action::Ps { script } => {
            return Ok(script.lines().iter().map(|l| s(l)).collect());
        }
        _ if limited => {
            return Err(err(
                rule,
                format!(
                    "the phase {phase:?} allows only reg, reg-remove and ps actions, not {}",
                    action.kind()
                ),
            ));
        }
        Action::RegList {
            path,
            param,
            names,
            prefix,
            additive,
            kind,
        } => {
            let items = list_values(params, param);
            let (n, v) = list_entries(*names, prefix, &items).map_err(|e| err(rule, e))?;
            reg_list_line(path, *kind, &n, &v, *additive)
        }
        Action::Service { name, start, .. } => {
            format!("Set-ServiceStart -Name {} -Start {start}", ps_quote(name))
        }
        Action::Exe { file, args } => {
            let args: Vec<String> = args.iter().map(|a| ps_quote(&s(a))).collect();
            format!("Invoke-Exe {} @({})", ps_quote(file), args.join(","))
        }
        Action::Feature { name, state, .. } => {
            format!("Set-Feature -Name {} -State {state}", ps_quote(name))
        }
        Action::Capability { pattern } => {
            format!("Remove-Capability -Pattern {}", ps_quote(pattern))
        }
        Action::Appx { names } => {
            let names: Vec<String> = names.iter().map(|n| ps_quote(n)).collect();
            format!("Remove-Apps @({})", names.join(","))
        }
        Action::XmlPeCommand { .. }
        | Action::XmlSpecializeCommand { .. }
        | Action::XmlSetting { .. }
        | Action::XmlOobe { .. } => return Ok(Vec::new()),
    };
    Ok(vec![line])
}

/// The parameter values of a rule in the profile (the defaults where the profile has none).
/// The `Set-RegList` line of a list of values.
pub(crate) fn reg_list_line(
    path: &str,
    kind: RegKind,
    names: &[String],
    values: &[String],
    additive: bool,
) -> String {
    let list = |items: &[String]| {
        items
            .iter()
            .map(|t| ps_quote(t))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "Set-RegList -Path {} -Type {kind:?} -Names @({}) -Values @({}){}",
        reg_path(path),
        list(names),
        list(values),
        if additive { " -Additive" } else { "" }
    )
}

pub(crate) fn params_of(
    catalog: &Catalog,
    profile: &Profile,
    rule: &Rule,
) -> BTreeMap<String, Value> {
    rule.params
        .keys()
        .filter_map(|n| profile.param(catalog, &rule.id, n).map(|v| (n.clone(), v)))
        .collect()
}

/// The block of a rule: the marker and the lines of its script actions.
fn block(rule: &Rule, params: &BTreeMap<String, Value>) -> Result<Option<String>, RenderError> {
    let mut lines = Vec::new();
    for action in &rule.actions {
        lines.extend(script_lines(rule, action, params, rule.phase)?);
    }
    if lines.is_empty() {
        return Ok(None);
    }
    let mut text = format!("# [{}]\n", rule.id);
    for l in lines {
        text.push_str(&l);
        text.push('\n');
    }
    Ok(Some(text))
}

pub(crate) fn join_blocks(blocks: &[String], indent: usize) -> String {
    let pad = " ".repeat(indent);
    blocks
        .iter()
        .map(|b| {
            b.lines()
                .map(|l| {
                    if l.is_empty() {
                        String::new()
                    } else {
                        format!("{pad}{l}")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// A nested XML setting of a component: the element names from the component down.
#[derive(Debug, Clone)]
struct Setting {
    path: Vec<String>,
    value: String,
}

/// Nested elements of settings that share a component, in the order given.
fn setting_lines(settings: &[Setting], indent: usize) -> Result<Vec<String>, RenderError> {
    #[derive(Default)]
    struct Node {
        value: Option<String>,
        children: Vec<(String, Node)>,
    }
    fn insert(node: &mut Node, path: &[String], value: &str) -> Result<(), RenderError> {
        let Some((first, rest)) = path.split_first() else {
            return Ok(());
        };
        let child = match node.children.iter_mut().position(|(n, _)| n == first) {
            Some(i) => &mut node.children[i].1,
            None => {
                node.children.push((first.clone(), Node::default()));
                &mut node.children.last_mut().expect("just pushed").1
            }
        };
        if rest.is_empty() {
            if !child.children.is_empty() || child.value.is_some() {
                return Err(RenderError::Template(format!(
                    "the setting {} is both a value and a parent, or set twice",
                    path.join("/")
                )));
            }
            child.value = Some(value.to_owned());
            Ok(())
        } else {
            if child.value.is_some() {
                return Err(RenderError::Template(format!(
                    "the setting {} is both a value and a parent",
                    path.join("/")
                )));
            }
            insert(child, rest, value)
        }
    }
    fn write(node: &Node, indent: usize, out: &mut Vec<String>) {
        let pad = " ".repeat(indent);
        for (name, child) in &node.children {
            match &child.value {
                Some(v) => out.push(format!("{pad}<{name}>{}</{name}>", xml_escape(v))),
                None => {
                    out.push(format!("{pad}<{name}>"));
                    write(child, indent + 2, out);
                    out.push(format!("{pad}</{name}>"));
                }
            }
        }
    }
    let mut root = Node::default();
    for s in settings {
        insert(&mut root, &s.path, &s.value)?;
    }
    let mut out = Vec::new();
    write(&root, indent, &mut out);
    Ok(out)
}

/// A command of a pass.
#[derive(Debug, Clone)]
struct Command {
    description: String,
    path: String,
}

fn command_lines(commands: &[Command], indent: usize) -> Result<Vec<String>, RenderError> {
    let pad = " ".repeat(indent);
    let mut out = Vec::new();
    if commands.is_empty() {
        return Ok(out);
    }
    out.push(format!("{pad}<RunSynchronous>"));
    for (i, c) in commands.iter().enumerate() {
        let len = c.path.chars().count();
        if len > MAX_COMMAND {
            return Err(RenderError::CommandTooLong {
                what: c.description.clone(),
                len,
                command: c.path.clone(),
            });
        }
        out.push(format!("{pad}  <RunSynchronousCommand wcm:action=\"add\">"));
        out.push(format!("{pad}    <Order>{}</Order>", i + 1));
        out.push(format!(
            "{pad}    <Description>{}</Description>",
            xml_escape(&c.description)
        ));
        out.push(format!("{pad}    <Path>{}</Path>", xml_escape(&c.path)));
        out.push(format!("{pad}  </RunSynchronousCommand>"));
    }
    out.push(format!("{pad}</RunSynchronous>"));
    Ok(out)
}

fn component(name: &str, arch: &str, body: &[String]) -> Vec<String> {
    let mut out = vec![format!(
        "    <component name=\"{name}\" processorArchitecture=\"{arch}\" publicKeyToken=\"31bf3856ad364e35\" language=\"neutral\" versionScope=\"nonSxS\">"
    )];
    out.extend(body.iter().cloned());
    out.push("    </component>".to_owned());
    out
}

/// The input languages of the profile as Windows wants them.
struct InputLanguages {
    input_locale: String,
    tags: Vec<String>,
    fallback: Vec<String>,
    transient: Vec<String>,
}

fn input_languages(profile: &Profile) -> Result<InputLanguages, RenderError> {
    let mut locales = Vec::new();
    let mut fallback = Vec::new();
    let mut transient = Vec::new();
    for tag in &profile.languages.input {
        let kb = resources::keyboard(tag)
            .ok_or_else(|| RenderError::Profile(format!("unknown input language {tag}")))?;
        let effective = if kb.transient {
            transient.push(tag.clone());
            let fb = kb.fallback.clone().unwrap_or_else(|| tag.clone());
            resources::keyboard(&fb).ok_or_else(|| {
                RenderError::Profile(format!("the fallback {fb} of {tag} is unknown"))
            })?
        } else {
            kb
        };
        let lcid = effective
            .lcid
            .clone()
            .ok_or_else(|| RenderError::Profile(format!("no locale id for {tag}")))?;
        locales.push(format!("{lcid}:{}", effective.klid));
        fallback.push(effective.tag.clone());
    }
    Ok(InputLanguages {
        input_locale: locales.join(";"),
        tags: profile.languages.input.clone(),
        fallback,
        transient,
    })
}

pub(crate) fn ps_list(items: &[String]) -> String {
    items
        .iter()
        .map(|s| ps_quote(s))
        .collect::<Vec<_>>()
        .join(",")
}

/// The parts of the computer name template for `section-computer-name.ps1`.
pub fn computer_name_parts(template: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut rest = template;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('{')
            && let Some(end) = after.find('}')
        {
            if !text.is_empty() {
                parts.push(format!("text:{text}"));
                text.clear();
            }
            let (name, length) = match after[..end].split_once(':') {
                Some((n, l)) => (n, l.parse::<usize>().ok()),
                None => (&after[..end], None),
            };
            let default_len = match name {
                "serial" => 15,
                "mac" => 12,
                _ => 6,
            };
            parts.push(format!("{name}:{}", length.unwrap_or(default_len)));
            rest = &after[end + 1..];
        } else {
            let c = rest.chars().next().unwrap_or('?');
            text.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    if !text.is_empty() {
        parts.push(format!("text:{text}"));
    }
    parts
}

/// What a computer name template gives with sample values, the way `section-computer-name.ps1`
/// expands it: the last characters of the serial number `5CG1234XYZ` and of the MAC address
/// `00155D3E5A01` (a shorter value stays whole), fixed random characters, the name cut at 15.
pub fn example_name(template: &str) -> String {
    const SERIAL: &str = "5CG1234XYZ";
    const MAC: &str = "00155D3E5A01";
    const RANDOM: &str = "K7Q2MX9H4TRW3N5";
    let mut name = String::new();
    for part in computer_name_parts(template) {
        let Some((kind, arg)) = part.split_once(':') else {
            continue;
        };
        if kind == "text" {
            name.push_str(arg);
            continue;
        }
        let length = arg.parse::<usize>().unwrap_or(0).min(RANDOM.len());
        let value = match kind {
            "serial" => SERIAL,
            "mac" => MAC,
            _ => &RANDOM[..length],
        };
        name.push_str(&value[value.len().saturating_sub(length)..]);
    }
    name.chars().take(15).collect()
}

/// The texts of an account the answer file can hold (ASCII only) and the lines of Post-OOBE
/// that set the rest.
fn account_xml(a: &Account) -> (String, String) {
    let display = if a.display_name.is_ascii() && !a.display_name.is_empty() {
        a.display_name.clone()
    } else {
        a.name.clone()
    };
    let description = if a.description.is_ascii() {
        a.description.clone()
    } else {
        String::new()
    };
    (display, description)
}

fn account_text_lines(profile: &Profile) -> Vec<String> {
    let mut lines = Vec::new();
    for a in &profile.accounts {
        let mut args = Vec::new();
        if !a.display_name.is_ascii() {
            args.push(format!(
                "-FullName (ConvertFrom-Base64Text {})",
                ps_quote(&base64(&a.display_name))
            ));
        }
        if !a.description.is_ascii() {
            args.push(format!(
                "-Description (ConvertFrom-Base64Text {})",
                ps_quote(&base64(&a.description))
            ));
        }
        if !args.is_empty() {
            lines.push(format!(
                "Set-AccountText -Name {} {}",
                ps_quote(&a.name),
                args.join(" ")
            ));
        }
    }
    lines
}

/// Builds the answer file of a profile with the embedded catalog data.
pub fn build(
    catalog: &Catalog,
    profile: &Profile,
    app_version: &str,
) -> Result<Build, RenderError> {
    let resolution = resolver::resolve(catalog, profile);
    let mut warnings = Vec::new();
    let mut rule_ids = Vec::new();
    let mut pe_commands = Vec::new();
    let mut specialize_commands = Vec::new();
    let mut oobe_elements: Vec<(String, String)> = Vec::new();
    let mut settings: BTreeMap<(String, String), Vec<Setting>> = BTreeMap::new();
    let mut system_blocks = Vec::new();
    let mut du_blocks = Vec::new();
    let mut user_blocks = Vec::new();
    let mut post_blocks = Vec::new();

    for rule in catalog.rules() {
        if !resolution.in_effect(&rule.id) {
            continue;
        }
        rule_ids.push(rule.id.clone());
        let params: BTreeMap<String, Value> = rule
            .params
            .keys()
            .filter_map(|n| profile.param(catalog, &rule.id, n).map(|v| (n.clone(), v)))
            .collect();
        for action in &rule.actions {
            match action {
                Action::XmlPeCommand {
                    command,
                    description,
                } => pe_commands.push(Command {
                    description: description.clone(),
                    path: substitute(command, &params),
                }),
                Action::XmlSpecializeCommand {
                    command,
                    description,
                } => specialize_commands.push(Command {
                    description: description.clone(),
                    path: substitute(command, &params),
                }),
                Action::XmlOobe { element, value } => {
                    oobe_elements.push((element.clone(), substitute(value, &params)));
                }
                Action::XmlSetting {
                    pass,
                    component,
                    path,
                    value,
                } => settings
                    .entry((pass.clone(), component.clone()))
                    .or_default()
                    .push(Setting {
                        path: path.split('/').map(str::to_owned).collect(),
                        value: substitute(value, &params),
                    }),
                _ => {}
            }
        }
        if let Some(b) = block(rule, &params)? {
            match rule.phase {
                Phase::Specialize | Phase::Windowspe | Phase::SpecializeXml | Phase::OobeXml => {
                    system_blocks.push(b);
                }
                Phase::DefaultUser => du_blocks.push(b),
                Phase::UserFirstLogon => user_blocks.push(b),
                Phase::PostOobe => post_blocks.push(b),
            }
        }
    }

    let install = &profile.install;
    let label = format!(
        "WinWeaver {app_version}, catalog {}, {} rules enabled",
        catalog.version(),
        rule_ids.len()
    );
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
    let has_user = !user_blocks.is_empty();
    let has_post = !post_blocks.is_empty() || !account_texts.is_empty();
    let has_system = !system_blocks.is_empty()
        || !du_blocks.is_empty()
        || has_user
        || has_post
        || !name_parts.is_empty();

    // The scripts.
    let mut scripts = Vec::new();
    if has_system {
        let name_section = if name_parts.is_empty() {
            String::new()
        } else {
            fill(
                SECTION_COMPUTER_NAME,
                &[("name_parts", &ps_list(&name_parts))],
            )?
        };
        let du_section = if du_blocks.is_empty() {
            String::new()
        } else {
            fill(
                SECTION_DEFAULT_USER,
                &[("blocks", &join_blocks(&du_blocks, 4))],
            )?
        };
        let active = if has_user {
            SECTION_ACTIVE_SETUP.replace("\r\n", "\n")
        } else {
            String::new()
        };
        let task = if has_post {
            SECTION_POST_OOBE_TASK.replace("\r\n", "\n")
        } else {
            String::new()
        };
        let text = fill(
            SETUP_SYSTEM,
            &[
                ("build_label", &label),
                ("computer_name_section", &name_section),
                ("blocks", &join_blocks(&system_blocks, 0)),
                ("default_user_section", &du_section),
                ("active_setup_section", &active),
                ("post_oobe_section", &task),
            ],
        )?;
        scripts.push((SETUP_SYSTEM_NAME.to_owned(), text));
    }
    if has_user {
        let text = fill(
            SETUP_USER,
            &[
                ("input_languages", &ps_list(&languages.tags)),
                ("input_fallback", &ps_list(&languages.fallback)),
                ("transient_languages", &ps_list(&languages.transient)),
                ("blocks", &join_blocks(&user_blocks, 0)),
            ],
        )?;
        scripts.push((SETUP_USER_NAME.to_owned(), text));
    }
    if has_post {
        let names: Vec<String> = if install.account_mode == "file" {
            profile.accounts.iter().map(|a| a.name.clone()).collect()
        } else {
            Vec::new()
        };
        let text = fill(
            POST_OOBE,
            &[
                ("accounts", &ps_list(&names)),
                ("account_texts", &account_texts.join("\n")),
                ("blocks", &join_blocks(&post_blocks, 0)),
            ],
        )?;
        scripts.push((POST_OOBE_NAME.to_owned(), text));
    }
    for (name, text) in &scripts {
        if text.contains("]]>") {
            return Err(RenderError::Template(format!("{name} holds ]]>")));
        }
    }

    // The passes.
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
    let mut pe_body = Vec::new();
    if install.disk_mode == "partition" {
        pe_body.extend([
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
    pe_body.extend([
        "      <UserData>".to_owned(),
        "        <ProductKey>".to_owned(),
        format!("          <Key>{}</Key>", xml_escape(&key)),
        format!("          <WillShowUI>{will_show}</WillShowUI>"),
        "        </ProductKey>".to_owned(),
        "        <AcceptEula>true</AcceptEula>".to_owned(),
        "      </UserData>".to_owned(),
    ]);
    pe_body.extend(command_lines(&pe_commands, 6)?);
    if let Some(extra) =
        settings.remove(&("windowsPE".to_owned(), "Microsoft-Windows-Setup".to_owned()))
    {
        pe_body.extend(setting_lines(&extra, 6)?);
    }

    let mut deployment = Vec::new();
    if has_system {
        deployment.push(Command {
            description: "Extract embedded scripts from this answer file".to_owned(),
            path: "powershell.exe -NoProfile -WindowStyle Hidden -Command \"try{$x=[xml]::new();$x.Load('C:\\Windows\\Panther\\unattend.xml');$s=[scriptblock]::Create($x.unattend.Extensions.ExtractScript);icm $s -Args $x}catch{$_>C:\\Windows\\Temp\\ua.err};exit 0\"".to_owned(),
        });
    }
    deployment.extend(specialize_commands);
    if has_system {
        deployment.push(Command {
            description: "Apply machine-wide settings".to_owned(),
            path: format!(
                "powershell.exe -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -Command \"try{{$p='{SCRIPTS_DIR}\\{SETUP_SYSTEM_NAME}';if(Test-Path $p){{& $p}}else{{'no script'>C:\\Windows\\Temp\\ua.err}}}}catch{{$_>C:\\Windows\\Temp\\ua.err}};exit 0\""
            ),
        });
    }
    let mut deployment_body = command_lines(&deployment, 6)?;
    if let Some(extra) = settings.remove(&(
        "specialize".to_owned(),
        "Microsoft-Windows-Deployment".to_owned(),
    )) {
        deployment_body.extend(setting_lines(&extra, 6)?);
    }
    let mut shell_specialize = vec![format!(
        "      <TimeZone>{}</TimeZone>",
        xml_escape(&install.time_zone)
    )];
    match install.computer_name_mode.as_str() {
        "fixed" => shell_specialize.push(format!(
            "      <ComputerName>{}</ComputerName>",
            xml_escape(install.computer_name.trim())
        )),
        "template" => shell_specialize.push(format!(
            "      <ComputerName>{TEMP_COMPUTER_NAME}</ComputerName>"
        )),
        _ => {}
    }
    if let Some(extra) = settings.remove(&(
        "specialize".to_owned(),
        "Microsoft-Windows-Shell-Setup".to_owned(),
    )) {
        shell_specialize.extend(setting_lines(&extra, 6)?);
    }
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
    if let Some(extra) = settings.remove(&(
        "specialize".to_owned(),
        "Microsoft-Windows-UnattendedJoin".to_owned(),
    )) {
        join.extend(setting_lines(&extra, 6)?);
    }

    let mut international = vec![
        format!(
            "      <InputLocale>{}</InputLocale>",
            xml_escape(&languages.input_locale)
        ),
        format!(
            "      <SystemLocale>{}</SystemLocale>",
            xml_escape(&profile.languages.system_locale)
        ),
        format!(
            "      <UILanguage>{}</UILanguage>",
            xml_escape(&profile.languages.ui_language)
        ),
        format!(
            "      <UserLocale>{}</UserLocale>",
            xml_escape(&profile.languages.user_locale)
        ),
    ];
    if let Some(extra) = settings.remove(&(
        "oobeSystem".to_owned(),
        "Microsoft-Windows-International-Core".to_owned(),
    )) {
        international.extend(setting_lines(&extra, 6)?);
    }
    let mut shell_oobe = Vec::new();
    if !oobe_elements.is_empty() {
        shell_oobe.push("      <OOBE>".to_owned());
        for (element, value) in &oobe_elements {
            shell_oobe.push(format!(
                "        <{element}>{}</{element}>",
                xml_escape(value)
            ));
        }
        shell_oobe.push("      </OOBE>".to_owned());
    }
    if install.account_mode == "file" {
        shell_oobe.push("      <UserAccounts>".to_owned());
        shell_oobe.push("        <LocalAccounts>".to_owned());
        for a in &profile.accounts {
            let (display, description) = account_xml(a);
            shell_oobe.push("          <LocalAccount wcm:action=\"add\">".to_owned());
            shell_oobe.push("            <Password>".to_owned());
            shell_oobe.push(format!(
                "              <Value>{}</Value>",
                xml_escape(&a.password)
            ));
            shell_oobe.push("              <PlainText>true</PlainText>".to_owned());
            shell_oobe.push("            </Password>".to_owned());
            if !description.is_empty() {
                shell_oobe.push(format!(
                    "            <Description>{}</Description>",
                    xml_escape(&description)
                ));
            }
            shell_oobe.push(format!(
                "            <DisplayName>{}</DisplayName>",
                xml_escape(&display)
            ));
            shell_oobe.push(format!(
                "            <Group>{}</Group>",
                xml_escape(&a.group)
            ));
            shell_oobe.push(format!("            <Name>{}</Name>", xml_escape(&a.name)));
            shell_oobe.push("          </LocalAccount>".to_owned());
        }
        shell_oobe.push("        </LocalAccounts>".to_owned());
        shell_oobe.push("      </UserAccounts>".to_owned());
    }
    if let Some(extra) = settings.remove(&(
        "oobeSystem".to_owned(),
        "Microsoft-Windows-Shell-Setup".to_owned(),
    )) {
        shell_oobe.extend(setting_lines(&extra, 6)?);
    }
    // The components the fixed passes do not write.
    let mut extra: BTreeMap<String, Vec<(String, Vec<String>)>> = BTreeMap::new();
    for ((pass, comp), list) in &settings {
        if !matches!(pass.as_str(), "windowsPE" | "specialize" | "oobeSystem") {
            return Err(RenderError::Template(format!("unknown pass {pass}")));
        }
        extra
            .entry(pass.clone())
            .or_default()
            .push((comp.clone(), setting_lines(list, 6)?));
    }
    let extras = |pass: &str, arch: &str| -> Vec<String> {
        let mut out = Vec::new();
        if let Some(list) = extra.get(pass) {
            for (comp, body) in list {
                out.extend(component(comp, arch, body));
            }
        }
        out
    };

    let mut settings_xml = Vec::new();
    settings_xml.push("  <settings pass=\"windowsPE\">".to_owned());
    for arch in ARCHITECTURES {
        settings_xml.extend(component("Microsoft-Windows-Setup", arch, &pe_body));
        settings_xml.extend(extras("windowsPE", arch));
    }
    settings_xml.push("  </settings>".to_owned());
    settings_xml.push("  <settings pass=\"specialize\">".to_owned());
    for arch in ARCHITECTURES {
        if !deployment_body.is_empty() {
            settings_xml.extend(component(
                "Microsoft-Windows-Deployment",
                arch,
                &deployment_body,
            ));
        }
        settings_xml.extend(component(
            "Microsoft-Windows-Shell-Setup",
            arch,
            &shell_specialize,
        ));
        if !join.is_empty() {
            settings_xml.extend(component("Microsoft-Windows-UnattendedJoin", arch, &join));
        }
        settings_xml.extend(extras("specialize", arch));
    }
    settings_xml.push("  </settings>".to_owned());
    settings_xml.push("  <settings pass=\"oobeSystem\">".to_owned());
    for arch in ARCHITECTURES {
        settings_xml.extend(component(
            "Microsoft-Windows-International-Core",
            arch,
            &international,
        ));
        if !shell_oobe.is_empty() {
            settings_xml.extend(component(
                "Microsoft-Windows-Shell-Setup",
                arch,
                &shell_oobe,
            ));
        }
        settings_xml.extend(extras("oobeSystem", arch));
    }
    settings_xml.push("  </settings>".to_owned());

    // The extensions.
    let mut files = Vec::new();
    for (name, text) in &scripts {
        files.push(format!(
            "    <File path=\"{SCRIPTS_DIR}\\{name}\"><![CDATA["
        ));
        files.push(text.trim_end().to_owned());
        files.push("]]></File>".to_owned());
    }
    let mut profile_json = ascii_json(&profile.to_json());
    if profile_json.contains("]]>") {
        profile_json = profile_json.replace("]]>", "]]\\u003e");
    }
    let mut header = vec![
        format!(
            "    Generated by WinWeaver {app_version}, rules catalog {}, runtime {}.",
            catalog.version(),
            runtime_version()
        ),
        format!(
            "    Profile: {}; modified {}.",
            profile.name.replace("--", "- -"),
            profile.modified
        ),
        format!(
            "    Enabled rules: {} of {}. Disabled rules are absent from this file.",
            rule_ids.len(),
            catalog.rules().len()
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
        for (name, _) in &scripts {
            header.push(format!("      {SCRIPTS_DIR}\\{name}"));
        }
        header.push("      C:\\ProgramData\\Unattend\\Logs\\*.log".to_owned());
    }
    let header_text = header.join("\n");
    if header_text.contains("--") {
        return Err(RenderError::Template(
            "the header comment may not hold two hyphens".into(),
        ));
    }
    let xml = fill(
        TEMPLATE_XML,
        &[
            ("header", &header_text),
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
    if profile.accounts.iter().any(|a| !a.password.is_empty()) {
        warnings.push("The answer file holds the account passwords in plain text".to_owned());
    }
    Ok(Build {
        xml: xml.replace("\r\n", "\n").replace('\n', "\r\n"),
        scripts,
        rule_ids,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(ps_quote("it's"), "'it''s'");
        assert_eq!(xml_escape("a<b>&\"c\""), "a&lt;b&gt;&amp;&quot;c&quot;");
        assert_eq!(ascii_json("a\u{44f}b"), "a\\u044fb");
        assert_eq!(ascii_json("\u{1f600}"), "\\ud83d\\ude00");
        assert_eq!(base64("Admin"), "QWRtaW4=");
        assert_eq!(base64("ab"), "YWI=");
        assert_eq!(base64("abc"), "YWJj");
        assert_eq!(base64(""), "");
        let mut params = BTreeMap::new();
        params.insert("n".to_owned(), Value::from(5));
        params.insert("s".to_owned(), Value::from("x"));
        assert_eq!(
            substitute("/a:{n} {s} {other} {guid-1}", &params),
            "/a:5 x {other} {guid-1}"
        );
        assert_eq!(
            computer_name_parts("PC-{random}"),
            vec!["text:PC-", "random:6"]
        );
        assert_eq!(
            computer_name_parts("{serial:8}-W"),
            vec!["serial:8", "text:-W"]
        );
        assert_eq!(example_name("PC-{random}"), "PC-K7Q2MX");
        assert_eq!(example_name("WS-{serial:6}"), "WS-234XYZ");
        assert_eq!(example_name("WS-{serial}"), "WS-5CG1234XYZ");
        assert_eq!(example_name("{mac:4}-LAB"), "5A01-LAB");
        assert_eq!(example_name("{mac}{random:8}"), "00155D3E5A01K7Q");
        assert_eq!(reg_path("DU:\\Software\\X"), "($du + '\\Software\\X')");
        assert_eq!(reg_path("HKLM:\\X"), "'HKLM:\\X'");
    }

    #[test]
    fn settings_nest() {
        let lines = setting_lines(
            &[
                Setting {
                    path: vec!["DynamicUpdate".into(), "Enable".into()],
                    value: "false".into(),
                },
                Setting {
                    path: vec!["DynamicUpdate".into(), "WillShowUI".into()],
                    value: "Never".into(),
                },
                Setting {
                    path: vec!["Diagnostics".into(), "OptIn".into()],
                    value: "false".into(),
                },
            ],
            0,
        )
        .unwrap();
        assert_eq!(
            lines,
            vec![
                "<DynamicUpdate>",
                "  <Enable>false</Enable>",
                "  <WillShowUI>Never</WillShowUI>",
                "</DynamicUpdate>",
                "<Diagnostics>",
                "  <OptIn>false</OptIn>",
                "</Diagnostics>",
            ]
        );
        let clash = setting_lines(
            &[
                Setting {
                    path: vec!["A".into()],
                    value: "1".into(),
                },
                Setting {
                    path: vec!["A".into(), "B".into()],
                    value: "2".into(),
                },
            ],
            0,
        );
        assert!(clash.is_err());
    }
}
