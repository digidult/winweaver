use super::*;
use winweaver_core::apply::{Step, plan_undo};

#[test]
#[ignore = "changes this Windows: run in CI or a VM, AGENTS.md section 5"]
fn a_plan_writes_backs_up_and_undoes_in_a_test_key() {
    let key = format!("HKCU:\\Software\\WinWeaverTest-{}", std::process::id());
    let step = |op| Step {
        rule: "test".to_owned(),
        op,
    };
    let plan = Plan {
        format: apply::PLAN_FORMAT.to_owned(),
        version: apply::PLAN_VERSION,
        nonce: "n".to_owned(),
        purpose: "apply".to_owned(),
        steps: vec![
            step(Op::SetValue {
                path: key.clone(),
                name: "D".into(),
                kind: RegKind::DWord,
                data: Value::from(7),
            }),
            step(Op::SetValue {
                path: key.clone(),
                name: "S".into(),
                kind: RegKind::String,
                data: Value::from("text"),
            }),
            step(Op::SetValue {
                path: key.clone(),
                name: "M".into(),
                kind: RegKind::MultiString,
                data: serde_json::json!(["a", "b"]),
            }),
            step(Op::SetValue {
                path: key.clone(),
                name: "B".into(),
                kind: RegKind::Binary,
                data: serde_json::json!([1, 2, 255]),
            }),
            step(Op::SetValue {
                path: key.clone(),
                name: "Q".into(),
                kind: RegKind::QWord,
                data: Value::from("5"),
            }),
            step(Op::DeleteValue {
                path: key.clone(),
                name: "Absent".into(),
            }),
            step(Op::SetValue {
                path: format!("{key}\\L"),
                name: "old".into(),
                kind: RegKind::String,
                data: Value::from("o"),
            }),
            step(Op::SetList {
                path: format!("{key}\\L"),
                kind: RegKind::String,
                names: vec!["1".into(), "2".into()],
                values: vec!["a".into(), "b".into()],
                additive: false,
            }),
            step(Op::ServiceStart {
                name: "WinWeaverNoSuchService".into(),
                start: 4,
            }),
            step(Op::Run {
                file: "hostname.exe".into(),
                args: vec![],
            }),
        ],
        skipped: vec![],
    };
    let out = std::env::temp_dir().join(format!("winweaver-apply-{}", std::process::id()));
    let report = execute(&plan, &out, "test");
    assert_eq!(report.count("failed"), 0, "{report:?}");
    assert_eq!(report.count("skipped"), 1, "the missing service is skipped");
    let m = NativeMachine;
    assert_eq!(m.value(&key, "D"), Ok(Some(Data::Dword(7))));
    assert_eq!(m.value(&key, "S"), Ok(Some(Data::Sz("text".into()))));
    assert_eq!(
        m.value(&key, "M"),
        Ok(Some(Data::MultiSz(vec!["a".into(), "b".into()])))
    );
    assert_eq!(m.value(&key, "B"), Ok(Some(Data::Binary(vec![1, 2, 255]))));
    assert_eq!(m.value(&key, "Q"), Ok(Some(Data::Qword(5))));
    let list = format!("{key}\\L");
    assert_eq!(
        m.value(&list, "old"),
        Ok(None),
        "the list deletes the other values"
    );
    assert_eq!(m.value(&list, "2"), Ok(Some(Data::Sz("b".into()))));
    let entries = apply::read_backup(&out.join(&report.backup)).unwrap();
    assert_eq!(entries.len(), 9);
    assert!(entries.iter().all(|e| !e.existed));
    let undo = execute(&plan_undo(&entries, "n"), &out, "undo");
    assert_eq!(undo.count("failed"), 0, "{undo:?}");
    assert_eq!(m.value(&key, "D"), Ok(None));
    let (root, sub) = hive_path(&key).unwrap();
    root.delete_subkey_all(&sub).unwrap();
    std::fs::remove_dir_all(&out).unwrap();
    assert!(system_program("..\\x.exe").is_err() && system_program("cmd").is_err());
}

#[test]
#[ignore = "changes this Windows: run in CI or a VM, AGENTS.md section 5"]
fn the_default_user_profile_is_loaded_written_and_unloaded() {
    // It writes the default user profile: only on the build machine of CI, as administrator.
    if !crate::native::elevated() {
        return;
    }
    let users = RegKey::predef(HKEY_USERS);
    let loaded_before = users.open_subkey(default_user_subkey()).is_ok();
    let key = format!(
        "{}\\Software\\WinWeaverTest-{}",
        apply::DEFAULT_USER_KEY,
        std::process::id()
    );
    let plan = Plan {
        format: apply::PLAN_FORMAT.to_owned(),
        version: apply::PLAN_VERSION,
        nonce: "n".to_owned(),
        purpose: "apply".to_owned(),
        steps: vec![Step {
            rule: "test".to_owned(),
            op: Op::SetValue {
                path: key.clone(),
                name: "D".into(),
                kind: RegKind::DWord,
                data: Value::from(7),
            },
        }],
        skipped: vec![],
    };
    assert!(plan.uses_default_user());
    let out = std::env::temp_dir().join(format!("winweaver-du-{}", std::process::id()));
    let report = execute(&plan, &out, "du");
    assert_eq!(report.count("failed"), 0, "{report:?}");
    assert_eq!(
        users.open_subkey(default_user_subkey()).is_ok(),
        loaded_before,
        "the hive is unloaded again"
    );
    let entries = apply::read_backup(&out.join(&report.backup)).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, key);
    let undo = execute(&plan_undo(&entries, "n"), &out, "du-undo");
    assert_eq!(undo.count("failed"), 0, "{undo:?}");
    let hive = DefaultUser::mount().unwrap();
    assert_eq!(NativeMachine.value(&key, "D"), Ok(None));
    let sub = key.strip_prefix("HKU:\\").unwrap();
    users.delete_subkey_all(sub).unwrap();
    hive.unmount().unwrap();
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
#[ignore = "changes this Windows: run in CI or a VM, AGENTS.md section 5"]
fn the_native_script_steps_work_and_undo_in_a_test_key() {
    let key = format!("HKCU:\\Software\\WinWeaverSteps-{}", std::process::id());
    let (one, two, deep) = (
        format!("{key}\\A\\One"),
        format!("{key}\\A\\Two"),
        format!("{key}\\Gone\\Deep"),
    );
    for path in [&one, &two, &deep] {
        write_value(path, "x", RegKind::String, &Value::from("old")).unwrap();
    }
    let out = std::env::temp_dir().join(format!("winweaver-steps-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    let kept = out.join("unattend.xml");
    std::fs::write(&kept, "kept").unwrap();
    let step = |op| Step {
        rule: "test".to_owned(),
        op,
    };
    let plan = Plan {
        format: apply::PLAN_FORMAT.to_owned(),
        version: apply::PLAN_VERSION,
        nonce: "n".to_owned(),
        purpose: "apply".to_owned(),
        steps: vec![
            step(Op::SetValueInSubkeys {
                path: format!("{key}\\A"),
                name: "N".into(),
                kind: RegKind::DWord,
                data: Value::from(2),
            }),
            step(Op::DeleteKey {
                path: format!("{key}\\Gone"),
            }),
            step(Op::ServiceStartFrom {
                name: "WinWeaverNoSuchService".into(),
                from: 4,
                start: 3,
            }),
            step(Op::RunFromMedia {
                folder: "WinWeaverNoSuchFolder".into(),
                pattern: "*.cab".into(),
                deep: false,
                file: "hostname.exe".into(),
                args: vec![],
            }),
            // A file that is not a copy of the answer file is never deleted.
            step(Op::DeleteFiles {
                paths: vec![kept.display().to_string()],
            }),
        ],
        skipped: vec![],
    };
    let report = execute(&plan, &out, "steps");
    assert_eq!(report.count("skipped"), 2, "{report:?}");
    assert_eq!(report.count("failed"), 1, "{report:?}");
    assert!(kept.exists());
    let m = NativeMachine;
    assert_eq!(m.value(&one, "N"), Ok(Some(Data::Dword(2))));
    assert_eq!(m.value(&two, "N"), Ok(Some(Data::Dword(2))));
    assert_eq!(m.value(&deep, "x"), Ok(None), "the key is deleted");
    // The undo makes the deleted key again with its value and removes the new values.
    let entries = apply::read_backup(&out.join(&report.backup)).unwrap();
    let undo = execute(&plan_undo(&entries, "n"), &out, "steps-undo");
    assert_eq!(undo.count("failed"), 0, "{undo:?}");
    assert_eq!(m.value(&deep, "x"), Ok(Some(Data::Sz("old".into()))));
    assert_eq!(m.value(&one, "N"), Ok(None));
    let (root, sub) = hive_path(&key).unwrap();
    root.delete_subkey_all(&sub).unwrap();
    std::fs::remove_dir_all(&out).unwrap();
}
