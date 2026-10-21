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
