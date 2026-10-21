//! The input languages of a profile as the answer file and the script of the first sign-in
//! name them.

use crate::profile::Profile;
use crate::resources;
use crate::validate;

use super::RenderError;

/// The input languages of the profile as Windows wants them: the input profiles of the answer
/// file, and for the script of the first sign-in the tag of each item, its layout when it names
/// one (empty for the one Windows gives the language first) and its tag with a language without
/// a locale id replaced by the language it belongs to.
pub(super) struct InputLanguages {
    pub(super) input_locale: String,
    pub(super) tags: Vec<String>,
    pub(super) tips: Vec<String>,
    pub(super) fallback: Vec<String>,
    pub(super) transient: Vec<String>,
}

pub(super) fn input_languages(profile: &Profile) -> Result<InputLanguages, RenderError> {
    let table = resources::locales();
    let mut locales = Vec::new();
    let mut tags = Vec::new();
    let mut tips = Vec::new();
    let mut fallback = Vec::new();
    let mut transient = Vec::new();
    for item in &profile.languages.input {
        let no_layout = || RenderError::Profile(format!("no layout for the input language {item}"));
        let Some((language, input)) = table.input(item) else {
            let why = format!("Windows does not know the input language {item}");
            return Err(RenderError::Profile(why));
        };
        if language.lcid.is_empty() {
            // Windows numbers it at the first sign-in: the answer file names the language it
            // belongs to, and the script of the first sign-in the language itself.
            let parent = table
                .language(&language.parent)
                .filter(|p| !p.default.is_empty())
                .ok_or_else(no_layout)?;
            transient.push(language.tag.clone());
            locales.push(parent.default.clone());
            tips.push(String::new());
            fallback.push(parent.tag.clone());
        } else {
            if input.is_empty() {
                return Err(no_layout());
            }
            let own = validate::is_layout(item);
            locales.push(input.clone());
            tips.push(if own { input } else { String::new() });
            fallback.push(language.tag.clone());
        }
        tags.push(language.tag.clone());
    }
    // A language named with a layout of its own and also by its tag keeps the layout of its tag.
    let own: Vec<&String> = tags
        .iter()
        .zip(&tips)
        .filter(|(_, tip)| !tip.is_empty())
        .map(|(tag, _)| tag)
        .collect();
    for ((tag, tip), input) in tags.iter().zip(tips.iter_mut()).zip(&locales) {
        if tip.is_empty() && own.contains(&tag) {
            *tip = input.clone();
        }
    }
    Ok(InputLanguages {
        input_locale: locales.join(";"),
        tags,
        tips,
        fallback,
        transient,
    })
}
