//! A question over the window before an act that is hard to take back: its title, its text, the
//! label of the yes and the act a yes runs.

pub struct Question<T> {
    pub title: String,
    pub text: String,
    pub yes: String,
    pub act: T,
}

impl<T> Question<T> {
    pub fn new(title: &str, text: impl Into<String>, yes: &str, act: impl Into<T>) -> Self {
        Self {
            title: title.to_owned(),
            text: text.into(),
            yes: yes.to_owned(),
            act: act.into(),
        }
    }
}
