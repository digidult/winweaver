use super::*;
use winweaver_core::diff::ItemKey;

mod catalogs;
mod files;
mod pc;
mod registry;
mod window;

/// The key of the first rule of the diff in the last tab.
pub(super) fn first_rule_key(app: &App) -> Option<String> {
    match app.docs.last() {
        Some(Doc::Diff(d)) => d
            .diff
            .keys()
            .into_iter()
            .find(|k| ItemKey::rule(k).is_some()),
        _ => None,
    }
}

pub(super) const SAMPLE_ADMX: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<policyDefinitions xmlns="http://schemas.microsoft.com/GroupPolicy/2006/07/PolicyDefinitions" revision="1.0" schemaVersion="1.0">
  <policyNamespaces>
    <target prefix="sample" namespace="Test.Policies.Sample" />
    <using prefix="windows" namespace="Microsoft.Policies.Windows" />
  </policyNamespaces>
  <resources minRequiredRevision="1.0" />
  <categories>
    <category name="Sample" displayName="$(string.Sample)">
      <parentCategory ref="windows:WindowsComponents" />
    </category>
  </categories>
  <policies>
    <policy name="TurnOff" class="Machine" displayName="$(string.TurnOff)" key="Software\Policies\Test" valueName="Off">
      <parentCategory ref="Sample" />
      <enabledValue><decimal value="1" /></enabledValue>
      <disabledValue><decimal value="0" /></disabledValue>
    </policy>
  </policies>
</policyDefinitions>"#;

pub(super) const SAMPLE_ADML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<policyDefinitionResources revision="1.0" schemaVersion="1.0">
  <resources>
    <stringTable>
      <string id="Sample">Sample</string>
      <string id="TurnOff">Turn the sample off</string>
    </stringTable>
  </resources>
</policyDefinitionResources>"#;

/// Writes the sample templates as the catalog file `test` into `<dir>/imports`.
pub(super) fn sample_import(dir: &Path) {
    let admx = dir.join("admx");
    std::fs::create_dir_all(admx.join("en-US")).unwrap();
    std::fs::write(admx.join("sample.admx"), SAMPLE_ADMX).unwrap();
    std::fs::write(admx.join("en-US").join("sample.adml"), SAMPLE_ADML).unwrap();
    let templates = admx::read_folder(&admx);
    let file = admx::to_catalog_file(&templates, "test", "Test templates", "", "now");
    assert!(
        file.rules.iter().any(|r| r.id == "admx.sample.turnoff"),
        "{:?}",
        file.skipped
    );
    let imports = dir.join("imports");
    std::fs::create_dir_all(&imports).unwrap();
    file.save(&imports.join("test.wwcatalog")).unwrap();
}

/// Waits for a read of this PC as the frames do.
pub(super) fn wait_pc_read(app: &mut App) {
    let ctx = egui::Context::default();
    let started = std::time::Instant::now();
    while app.pc_read.is_some() {
        assert!(started.elapsed() < std::time::Duration::from_secs(60));
        std::thread::sleep(std::time::Duration::from_millis(10));
        app.poll_pc_read(&ctx);
    }
}
