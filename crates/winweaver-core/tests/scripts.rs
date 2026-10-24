//! Tests of the stand-alone scripts against the lines they must hold.

use std::path::Path;

use winweaver_core::catalog::Catalog;
use winweaver_core::profile::Profile;
use winweaver_core::scripts;

fn office() -> Profile {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/presets/office.wwprofile");
    Profile::load(&path).unwrap()
}

fn all(catalog: &Catalog) -> Vec<String> {
    catalog.rules().iter().map(|r| r.id.clone()).collect()
}

fn markers(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("# ["))
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_apply_script_applies_and_returns() {
    let catalog = Catalog::embedded().unwrap();
    let (text, plan) = scripts::apply_script(&catalog, &office(), &all(&catalog), "test").unwrap();
    assert!(!text.contains("{{"), "a slot stayed unfilled");
    assert!(text.contains(
        "# WinWeaver test, profile: Office (rules that are off return to Windows defaults)"
    ));
    assert!(text.contains("$accounts = @('Admin','User')"));
    assert!(
        text.contains("if (Mount-DefaultUser) {") && text.contains("    Dismount-DefaultUser\n}")
    );
    assert!(text.trim_end().ends_with("exit 0"));
    let marks = markers(&text);
    let reverts = marks
        .iter()
        .filter(|m| m.ends_with("] Windows defaults"))
        .count();
    assert_eq!(reverts, plan.reverted.len());
    assert!(!plan.applied.is_empty() && !plan.reverted.is_empty());
    assert!(
        !marks.iter().any(|m| m.starts_with("install.bypass-tpm")),
        "installation rules stay out"
    );
    assert!(
        plan.excluded
            .iter()
            .any(|(id, _)| id == "install.bypass-tpm")
    );
    assert!(text.contains(
        "Set-Reg -Path 'HKLM:\\SOFTWARE\\Policies\\Microsoft\\Windows NT\\Printers\\PointAndPrint' -Name 'RestrictDriverInstallationToAdministrators' -Type DWord -Value 1 -Why 'PrintNightmare'"
    ));
    assert!(text.contains("Invoke-Exe 'net.exe' @('accounts','/lockoutthreshold:10','/lockoutduration:15','/lockoutwindow:15')"));
    assert!(!text.contains("WinKickOff"));
}

#[test]
fn the_revert_script_uses_the_windows_defaults() {
    let catalog = Catalog::embedded().unwrap();
    let (text, plan) = scripts::revert_script(&catalog, &office(), &all(&catalog), "test").unwrap();
    assert!(text.contains("# WinWeaver test, profile: Office (return to Windows defaults)"));
    assert!(text.contains(
        "# [lsa.protection] Windows defaults\nRemove-Reg -Path 'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Lsa' -Name 'RunAsPPL'\n"
    ));
    assert!(text.contains(
        "# [printing.spooler-automatic] Windows defaults\nSet-ServiceStart -Name 'Spooler' -Start 2\nSet-ServiceStart -Name 'PrintNotify' -Start 3\n"
    ));
    assert!(text.contains(
        "Set-Reg -Path 'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System' -Name 'EnableLUA' -Type DWord -Value 1 -Why 'Windows default'"
    ));
    assert!(text.contains(
        "    # [default-user.show-file-extensions] Windows defaults\n    Remove-Reg -Path ($du + '\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced') -Name 'HideFileExt'\n"
    ));
    assert!(
        !text.contains("# [apps.remove-quick-assist]"),
        "a removed app cannot come back"
    );
    assert!(
        plan.partial
            .iter()
            .any(|(id, _)| id == "printing.spooler-automatic")
    );
    assert!(plan.applied.is_empty());
}

#[test]
fn the_audit_script_checks_what_the_rules_write() {
    let catalog = Catalog::embedded().unwrap();
    let (text, plan) = scripts::audit_script(&catalog, &office(), &all(&catalog), "test").unwrap();
    assert!(!text.contains("{{"));
    assert!(text.contains("# WinWeaver test, profile: Office\n"));
    for line in [
        "Test-Reg -Rule 'defender.pua' -Path 'HKLM:\\SOFTWARE\\Policies\\Microsoft\\Windows Defender' -Name 'PUAProtection' -Kind DWord -Expected 1 -Note ''",
        "Test-ServiceStart -Rule 'printing.spooler-automatic' -Name 'Spooler' -Start 2",
        "Add-Unknown -Rule 'printing.spooler-automatic' -What 'ps'",
        "Test-Feature -Rule 'network.smb1-off' -Name 'SMB1Protocol' -State Disabled",
        "Test-CapabilityAbsent -Rule 'apps.remove-quick-assist' -Pattern 'App.Support.QuickAssist*'",
        "Test-AppAbsent -Rule 'apps.remove-quick-assist' -Name 'MicrosoftCorporationII.QuickAssist'",
        "Test-RegAbsent -Rule 'update.unblock' -Path 'HKLM:\\SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsUpdate\\AU' -Name 'NoAutoUpdate' -Note ''",
        "Test-Reg -Rule 'default-user.region' -Path 'HKCU:\\Control Panel\\International\\Geo' -Name 'Nation' -Kind String -Expected '241' -Note 'current user instead of the default profile'",
    ] {
        assert!(text.contains(line), "missing: {line}");
    }
    assert!(text.contains("$System = [ordered]@{ computer_name = $env:COMPUTERNAME }"));
    let tests = text
        .lines()
        .filter(|l| l.starts_with("Test-") || l.starts_with("Add-Unknown"))
        .count();
    assert_eq!(
        tests,
        plan.checks.len(),
        "one line per check, so a report pairs with the plan"
    );
}

#[test]
fn undo_and_file_bytes() {
    let undo = scripts::undo_script(&office(), "test").unwrap();
    assert!(undo.contains("# WinWeaver test, profile: Office\n") && !undo.contains("{{"));
    let bytes = scripts::file_bytes("a\nb\r\nc");
    assert_eq!(bytes, b"\xEF\xBB\xBFa\r\nb\r\nc");
    let mut named = office();
    named.name = "Офіс\nтест".into();
    let undo = scripts::undo_script(&named, "test").unwrap();
    assert!(
        undo.contains("# WinWeaver test, profile: ?????????\n"),
        "the label stays one ASCII line"
    );
}
