//! The accounts of a profile in the answer file and in Post-OOBE.

use crate::profile::{Account, Profile};

use super::text::{base64, ps_quote, xml_escape};

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

pub(super) fn account_text_lines(profile: &Profile) -> Vec<String> {
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

/// The `UserAccounts` of the pass oobeSystem: every account of the profile with its password
/// in plain text, as Windows Setup wants it.
pub(super) fn accounts_xml(profile: &Profile) -> Vec<String> {
    let mut out = vec![
        "      <UserAccounts>".to_owned(),
        "        <LocalAccounts>".to_owned(),
    ];
    for a in &profile.accounts {
        let (display, description) = account_xml(a);
        out.push("          <LocalAccount wcm:action=\"add\">".to_owned());
        out.push("            <Password>".to_owned());
        out.push(format!(
            "              <Value>{}</Value>",
            xml_escape(&a.password)
        ));
        out.push("              <PlainText>true</PlainText>".to_owned());
        out.push("            </Password>".to_owned());
        if !description.is_empty() {
            out.push(format!(
                "            <Description>{}</Description>",
                xml_escape(&description)
            ));
        }
        out.push(format!(
            "            <DisplayName>{}</DisplayName>",
            xml_escape(&display)
        ));
        out.push(format!(
            "            <Group>{}</Group>",
            xml_escape(&a.group)
        ));
        out.push(format!("            <Name>{}</Name>", xml_escape(&a.name)));
        out.push("          </LocalAccount>".to_owned());
    }
    out.push("        </LocalAccounts>".to_owned());
    out.push("      </UserAccounts>".to_owned());
    out
}
