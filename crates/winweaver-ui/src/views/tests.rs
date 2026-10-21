use super::*;

#[test]
fn a_text_links_the_rules_and_keys_it_names() {
    let catalog = Catalog::embedded().unwrap();
    let id = catalog.rules()[0].id.clone();
    let text = format!("Conflicts with {id}: see HKLM:\\Software\\X, then.");
    let found = pieces(&catalog, &text);
    let kinds: Vec<&str> = found
        .iter()
        .map(|p| match p {
            Piece::Text(_) => "text",
            Piece::Rule(_) => "rule",
            Piece::Key(_) => "key",
        })
        .collect();
    assert_eq!(kinds, ["text", "rule", "text", "key", "text"]);
    let Piece::Key(key) = &found[3] else {
        panic!("a key");
    };
    assert_eq!(key, "HKEY_LOCAL_MACHINE\\Software\\X");
    let Piece::Text(end) = &found[4] else {
        panic!("the text after it");
    };
    assert_eq!(end, ", then.");
}
