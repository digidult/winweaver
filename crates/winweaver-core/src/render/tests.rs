use std::collections::BTreeMap;

use serde_json::Value;

use super::*;
use super::xml::{Setting, setting_lines};

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
