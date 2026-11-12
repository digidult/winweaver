//! The messages of the core a person reads in the window: the English text with its `{name}`
//! placeholders, which is the key of its translation, and the values that fill them, kept apart.
//!
//! The core translates nothing: the command line, the MCP tools and the logs show the English
//! text filled ([`Message::english`]), and the window translates the key and fills the values
//! after ([`Message::shown`]). A message is made with [`tr`] or [`trf`], the names of the
//! functions of the window, so the test of the translations of the window
//! (`every_text_of_the_interface_is_translated`) finds the literals of a file of the core it
//! reads as it finds the ones of the window, and a new message cannot be left untranslated.

use std::fmt::Display;

/// A message with its values apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The English text with its `{name}` placeholders: the key of the translation.
    pub key: &'static str,
    /// The values of the placeholders, by name.
    pub args: Vec<(&'static str, String)>,
}

/// A message without values. Translated by the window, not here.
pub fn tr(key: &'static str) -> Message {
    Message {
        key,
        args: Vec::new(),
    }
}

/// A message with the values of its `{name}` placeholders. Translated by the window, not here.
pub fn trf(key: &'static str, args: &[(&'static str, &dyn Display)]) -> Message {
    Message {
        key,
        args: args
            .iter()
            .map(|(name, value)| (*name, value.to_string()))
            .collect(),
    }
}

impl Message {
    /// The message in English.
    pub fn english(&self) -> String {
        fill(self.key, &self.args)
    }

    /// The message in the language of `translate`, which gives the text of a key.
    pub fn shown(&self, translate: impl Fn(&'static str) -> String) -> String {
        fill(&translate(self.key), &self.args)
    }
}

/// A text with its `{name}` placeholders filled, in one pass: a value is put in as it is and
/// never read again, so a value that holds `{other}` is not filled. Braces that name no value
/// (`{serial}` in a text about computer names) stay.
pub fn fill(text: &str, args: &[(&'static str, String)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let value = after
            .find('}')
            .and_then(|end| args.iter().find(|(name, _)| *name == &after[..end]));
        match value {
            Some((name, value)) => {
                out.push_str(value);
                rest = &after[name.len() + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_is_filled_in_one_pass_in_any_language() {
        let plain = tr("The time zone is empty");
        assert_eq!(plain.english(), "The time zone is empty");
        let m = trf(
            "Parameter {name}: {value} is outside {min}..{max}",
            &[
                ("name", &"start"),
                ("value", &"{max}"),
                ("min", &0),
                ("max", &23),
            ],
        );
        assert_eq!(m.english(), "Parameter start: {max} is outside 0..23");
        let german = |key: &'static str| match key {
            "Parameter {name}: {value} is outside {min}..{max}" => {
                "Parameter {name}: {value} liegt nicht in {min}..{max}".to_owned()
            }
            other => other.to_owned(),
        };
        assert_eq!(
            m.shown(german),
            "Parameter start: {max} liegt nicht in 0..23"
        );
        assert_eq!(plain.shown(german), "The time zone is empty");
        // Braces that name no value stay as they are.
        let template = trf(
            "Use {serial} or {random:6} in {field}",
            &[("field", &"the name")],
        );
        assert_eq!(template.english(), "Use {serial} or {random:6} in the name");
        assert_eq!(fill("{a} {b", &[("a", "1".to_owned())]), "1 {b");
    }
}
