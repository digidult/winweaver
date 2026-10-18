//! The removal of apps through the package API of Windows (`Windows.Management.Deployment`), as
//! `Remove-Apps` of the apply script does it: the provisioned package goes first, so that accounts
//! created later do not get the app, then the package of every user. The elevated helper runs it;
//! nothing here starts PowerShell.

use std::time::Duration;

use windows::ApplicationModel::Package;
use windows::Management::Deployment::{PackageManager, RemovalOptions};
use windows::core::HSTRING;

/// Waits until an operation of the package API ends (its status leaves `Started`, which is 0)
/// and gives its error when it failed.
macro_rules! wait {
    ($op:expr) => {{
        let op = $op;
        while op.Status()?.0 == 0 {
            std::thread::sleep(Duration::from_millis(100));
        }
        op.GetResults().map(|_| ())
    }};
}

/// What a removal of apps did.
#[derive(Debug, Default)]
pub struct AppRemoval {
    /// The apps found provisioned or installed for some user.
    pub found: Vec<String>,
    /// What was removed: `deprovisioned <family>` and `removed <full name>`.
    pub done: Vec<String>,
    /// What could not be removed, each with its error.
    pub failed: Vec<String>,
}

/// A package of one of the apps asked for.
struct Found {
    name: String,
    family: HSTRING,
    full: HSTRING,
}

/// The package when its name is one of `names` (without regard to case, as `Get-AppxPackage
/// -Name`).
fn found(names: &[String], package: &Package) -> windows::core::Result<Option<Found>> {
    let id = package.Id()?;
    let name = id.Name()?.to_string();
    if !names.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
        return Ok(None);
    }
    Ok(Some(Found {
        name,
        family: id.FamilyName()?,
        full: id.FullName()?,
    }))
}

fn deprovision(manager: &PackageManager, family: &HSTRING) -> windows::core::Result<()> {
    let op = manager.DeprovisionPackageForAllUsersAsync(family)?;
    wait!(op)
}

fn remove(manager: &PackageManager, full: &HSTRING) -> windows::core::Result<()> {
    let op = manager.RemovePackageWithOptionsAsync(full, RemovalOptions::RemoveForAllUsers)?;
    wait!(op)
}

fn note(list: &mut Vec<String>, name: &str) {
    if !list.iter().any(|n| n == name) {
        list.push(name.to_owned());
    }
}

fn run(names: &[String]) -> windows::core::Result<AppRemoval> {
    let manager = PackageManager::new()?;
    let mut out = AppRemoval::default();
    let provisioned = manager.FindProvisionedPackages()?;
    for i in 0..provisioned.Size()? {
        let Some(app) = found(names, &provisioned.GetAt(i)?)? else {
            continue;
        };
        note(&mut out.found, &app.name);
        match deprovision(&manager, &app.family) {
            Ok(()) => out.done.push(format!("deprovisioned {}", app.family)),
            Err(e) => out.failed.push(format!("deprovision {}: {e}", app.family)),
        }
    }
    // The packages of every user are listed first and removed after, so the list does not change
    // while it is read.
    let mut installed = Vec::new();
    let packages = manager.FindPackages()?.First()?;
    while packages.HasCurrent()? {
        if let Some(app) = found(names, &packages.Current()?)? {
            installed.push(app);
        }
        packages.MoveNext()?;
    }
    for app in installed {
        note(&mut out.found, &app.name);
        match remove(&manager, &app.full) {
            Ok(()) => out.done.push(format!("removed {}", app.full)),
            Err(e) => out.failed.push(format!("remove {}: {e}", app.full)),
        }
    }
    Ok(out)
}

/// Removes the provisioned packages and the packages of every user of the apps with these names.
pub fn remove_apps(names: &[String]) -> Result<AppRemoval, String> {
    run(names).map_err(|e| format!("the package API of Windows: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The package API answers an administrator, which the CI runner is: a name of no app
    /// removes nothing.
    #[test]
    fn a_name_of_no_app_removes_nothing() {
        if std::env::var_os("GITHUB_ACTIONS").is_none() || !crate::native::elevated() {
            return;
        }
        let none = remove_apps(&["WinWeaver.NoSuchApp".to_owned()]).expect("the package API");
        assert!(none.found.is_empty(), "{none:?}");
        assert!(none.done.is_empty() && none.failed.is_empty(), "{none:?}");
    }
}
