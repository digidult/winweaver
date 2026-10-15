//! The bookmark tab: the bookmarks of the open bookmark file, grouped by document, with the
//! comment editable in place (`docs/spec/01-ui.md` section 9).

use std::path::Path;

use crate::i18n::{tr, trf};
use eframe::egui;
use winweaver_core::bookmarks::Bookmarks;

use crate::Act;

pub fn marks_view(
    ui: &mut egui::Ui,
    marks: &Bookmarks,
    path: &Path,
    search: &crate::Search,
    acts: &mut Vec<Act>,
) {
    let words = search.words();
    ui.heading(tr("Bookmarks"));
    ui.weak(trf(
        "{path} ({n} bookmarks)",
        &[("path", &path.display()), ("n", &marks.items.len())],
    ));
    ui.weak(tr("The Bookmark button in the navigation row adds the current position here or removes it. A click on a title goes to the position; the comment is edited in place."));
    ui.add_space(8.0);
    if marks.items.is_empty() {
        ui.label(tr("No bookmarks yet."));
        return;
    }
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            let mut last_path: Option<&str> = None;
            for (i, b) in marks.items.iter().enumerate() {
                let hay = format!("{} {} {} {}", b.title, b.item, b.path, b.comment).to_lowercase();
                if !words.iter().all(|w| hay.contains(w.as_str())) {
                    continue;
                }
                if last_path != Some(b.path.as_str()) {
                    ui.add_space(8.0);
                    ui.strong(if b.path.is_empty() {
                        "Unsaved documents"
                    } else {
                        b.path.as_str()
                    });
                    last_path = Some(b.path.as_str());
                }
                ui.horizontal(|ui| {
                    if ui.link(b.title.as_str()).clicked() {
                        acts.push(Act::GoToMark(i));
                    }
                    if !b.item.is_empty() {
                        ui.weak(b.item.as_str());
                    }
                    ui.weak(b.document.as_str());
                    if ui.small_button(tr("remove")).clicked() {
                        acts.push(Act::RemoveMark(i));
                    }
                });
                let mut comment = b.comment.clone();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut comment)
                            .id_salt(("bookmark-comment", i))
                            .hint_text(tr("Comment"))
                            .desired_width(640.0),
                    )
                    .changed()
                {
                    acts.push(Act::SetMarkComment(i, comment));
                }
            }
        });
}
