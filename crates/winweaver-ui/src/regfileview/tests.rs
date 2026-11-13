use super::*;

fn sample() -> RegFile {
    let text = [
        "Windows Registry Editor Version 5.00",
        "",
        "; The head of the file",
        "",
        "; Rule a.b",
        "[HKEY_CURRENT_USER\\Software\\A]",
        "\"One\"=dword:00000001",
        "; between the values",
        "\"Two\"=-",
        "",
        "; alone",
        "",
        "[-HKEY_CURRENT_USER\\Software\\Old]",
        "",
        "",
    ]
    .join("\r\n");
    RegFile::parse(&text).unwrap()
}

#[test]
fn the_outline_keeps_the_comments_in_their_places() {
    let file = sample();
    let found = items(&file.lines);
    // The head, the key with the comment above it and the one between its values, the
    // comment alone, the deleted key.
    assert_eq!(found.len(), 4, "{found:?}");
    assert_eq!(found[0], Item::Notes { lines: 1..2 });
    assert_eq!(
        found[1],
        Item::Key {
            line: 4,
            lines: 3..8,
        }
    );
    assert_eq!(found[2], Item::Notes { lines: 9..10 });
    assert!(matches!(found[3], Item::Key { line: 11, .. }));
    assert_eq!(rule_of_comment("; Rule a.b"), Some("a.b"));
    let long = "HKEY_LOCAL_MACHINE\\SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate\\AU";
    assert_eq!(short_path(long), "HKLM\\...\\Windows\\WindowsUpdate\\AU");
    let short = short_path("HKEY_CURRENT_USER\\Software\\X");
    assert_eq!(short, "HKCU\\Software\\X");
    assert_eq!(rule_of_comment("; The head"), None);
    assert_eq!(
        catalog_key("HKEY_USERS\\UnattendDefault\\Software\\X"),
        "hkey_current_user\\software\\x"
    );
}

#[test]
fn a_value_is_edited_added_and_removed() {
    let mut doc = RegFileDoc::new(sample(), PathBuf::from("x.reg"));
    // A new value of the key goes after its last value.
    doc.at = Some(4);
    doc.start_new(EditKind::Value);
    let edit = doc.edit.as_mut().unwrap();
    assert_eq!(edit.at, 8);
    edit.name = "Three".to_owned();
    edit.type_name = "REG_DWORD".to_owned();
    edit.text = "0x10".to_owned();
    assert!(doc.apply_edit());
    assert_eq!(
        doc.file.lines[8],
        Line::Value {
            name: "Three".to_owned(),
            data: Some(Data::Dword(16)),
        }
    );
    // Bad data stays in the editor with its error.
    doc.start_edit(8);
    doc.edit.as_mut().unwrap().text = "x".to_owned();
    assert!(!doc.apply_edit());
    assert!(doc.edit.as_ref().unwrap().error.is_some());
    doc.edit = None;
    // The key goes with its values; its comments stay.
    doc.line = None;
    let (lines, _) = doc.removal().unwrap();
    assert_eq!(lines, vec![4, 5, 7, 8]);
    doc.remove_lines(&lines);
    let text = doc.file.to_text();
    assert!(text.contains("; Rule a.b"), "{text}");
    assert!(text.contains("; between the values"), "{text}");
    assert!(!text.contains("\"One\""), "{text}");
    // A new key goes before the blank lines at the end, after a blank line.
    doc.start_new(EditKind::Key);
    doc.edit.as_mut().unwrap().name = "HKLM\\Software\\New".to_owned();
    assert!(doc.apply_edit());
    let text = doc.file.to_text();
    let key = "[HKEY_LOCAL_MACHINE\\Software\\New]";
    assert!(text.contains(key), "{text}");
    assert_eq!(doc.file.lines.last(), Some(&Line::Blank));
}

/// A list that replaces the values of its key deletes the key and writes it again: the tree shows
/// the key made anew with its value, the last entry of its path, not the deletion; a key written
/// under a deleted one is made anew too, and one deleted after it was written shows deleted.
#[test]
fn a_key_deleted_and_written_again_shows_made_anew() {
    let text = [
        "Windows Registry Editor Version 5.00",
        "",
        "; The list replaces the values of its key: the key is made anew",
        "[-HKEY_LOCAL_MACHINE\\Software\\List]",
        "",
        "[HKEY_LOCAL_MACHINE\\Software\\List]",
        "\"1\"=\"one\"",
        "",
        "[-HKEY_CURRENT_USER\\Software\\Parent]",
        "",
        "[HKEY_CURRENT_USER\\Software\\Parent\\Child]",
        "\"A\"=dword:00000001",
        "",
        "[HKEY_CURRENT_USER\\Software\\Gone]",
        "\"B\"=dword:00000002",
        "",
        "[-HKEY_CURRENT_USER\\Software\\Gone]",
        "",
    ]
    .join("\r\n");
    let file = RegFile::parse(&text).unwrap();
    let lines = &file.lines;
    let found = items(lines);
    let made = made_anew(lines);
    let key_line = |path: &str, delete: bool| {
        lines
            .iter()
            .position(
                |l| matches!(l, Line::Key { path: p, delete: d } if p == path && *d == delete),
            )
            .unwrap()
    };
    let list = key_line("HKEY_LOCAL_MACHINE\\Software\\List", false);
    let child = key_line("HKEY_CURRENT_USER\\Software\\Parent\\Child", false);
    let gone = key_line("HKEY_CURRENT_USER\\Software\\Gone", false);
    assert!(made.contains(&list) && made.contains(&child), "{made:?}");
    assert!(!made.contains(&gone), "written before its deletion");
    let rows = tree_rows(lines, &found, &HashSet::new(), &made);
    let row = |title: &str| {
        rows.iter()
            .find(|r| r.title == title)
            .unwrap_or_else(|| panic!("no row {title}"))
    };
    // The list: one row, the last entry of its path, made anew with its one value.
    assert_eq!(rows.iter().filter(|r| r.title == "List").count(), 1);
    let shown = row("List");
    assert_eq!(shown.icon, Some(icons::REFRESH));
    assert_eq!(shown.right, "1");
    assert!(!shown.muted);
    let Node::Item(i) = shown.key else {
        panic!("the list is an entry")
    };
    assert_eq!(found[i].anchor(), list);
    // The parent deleted, its child made anew under it; a key deleted after it was written.
    assert_eq!(row("Parent").icon, Some(icons::DELETE));
    assert_eq!(row("Child").icon, Some(icons::REFRESH));
    assert_eq!(row("Gone").icon, Some(icons::DELETE));
    // The list of the entries keeps both lines of the list in their order.
    let deletion = found
        .iter()
        .position(|it| it.anchor() == key_line("HKEY_LOCAL_MACHINE\\Software\\List", true))
        .unwrap();
    let first = outline_row(lines, &found[deletion], deletion, &made);
    assert_eq!(first.icon, Some(icons::DELETE));
}
