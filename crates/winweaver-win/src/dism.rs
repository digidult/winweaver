//! The optional features and the capabilities of the running Windows, read through the DISM API
//! of the system: `dismapi.dll` of System32, loaded and initialized the first time it is needed
//! and kept for the life of the process, since the API does not promise a second start after
//! `DismShutdown`. DISM serves an administrator only; nothing here changes the image.

use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use windows_sys::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};
use winweaver_core::thispc::{self, Components};

use crate::ffi::{wide, wide_text};

/// `DISM_ONLINE_IMAGE` of dismapi.h: the Windows that runs.
const ONLINE_IMAGE: &str = "DISM_{53BFAE52-B167-4E2F-A258-0A37B57FF845}";
/// `DismLogErrors`.
const LOG_ERRORS: i32 = 0;
/// `DismPackageNone`: the features of every package.
const PACKAGE_NONE: i32 = 0;
/// `DismStateInstalled` of `DismPackageFeatureState`.
const INSTALLED: u32 = 4;
/// `ERROR_SUCCESS_REBOOT_REQUIRED` as an HRESULT: done, and Windows wants a restart.
const REBOOT_REQUIRED: i32 = 3010;

/// `DismFeature` and `DismCapability` have the same layout: a name and a
/// `DismPackageFeatureState`, packed to one byte like every structure of dismapi.h.
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct Item {
    name: *const u16,
    state: u32,
}

type Proc = unsafe extern "system" fn() -> isize;
type Initialize = unsafe extern "system" fn(i32, *const u16, *const u16) -> i32;
type OpenSession = unsafe extern "system" fn(*const u16, *const u16, *const u16, *mut u32) -> i32;
type GetFeatures = unsafe extern "system" fn(u32, *const u16, i32, *mut *mut Item, *mut u32) -> i32;
type GetCapabilities = unsafe extern "system" fn(u32, *mut *mut Item, *mut u32) -> i32;
type Delete = unsafe extern "system" fn(*const c_void) -> i32;
type CloseSession = unsafe extern "system" fn(u32) -> i32;
/// A handle of an event, and a pointer the reader passes as null.
type Event = *mut c_void;
type Ptr = *const c_void;
type RemoveCapability = unsafe extern "system" fn(u32, *const u16, Event, Ptr, Ptr) -> i32;

/// The functions of dismapi.dll that the reader calls.
struct Api {
    initialize: Initialize,
    open_session: OpenSession,
    get_features: GetFeatures,
    /// Windows 10 and later; `None` where DISM has no capabilities.
    get_capabilities: Option<GetCapabilities>,
    /// Windows 10 and later, like the list of capabilities.
    remove_capability: Option<RemoveCapability>,
    delete: Delete,
    close_session: CloseSession,
}

fn failed(call: &str, hr: i32) -> String {
    format!("{call} failed with 0x{:08X}", hr as u32)
}

fn load() -> Result<Api, String> {
    let name = wide("dismapi.dll");
    // SAFETY: the name is a terminated wide string, and only System32 is searched.
    let module = unsafe {
        LoadLibraryExW(
            name.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_SYSTEM32,
        )
    };
    if module.is_null() {
        return Err("the DISM API (dismapi.dll) is not available".to_owned());
    }
    // The module stays loaded for the life of the process, so its functions stay valid.
    let find = |symbol: &str| -> Option<Proc> {
        let ansi: Vec<u8> = symbol.bytes().chain(std::iter::once(0)).collect();
        // SAFETY: the module is loaded and the name is a terminated ANSI string.
        unsafe { GetProcAddress(module, ansi.as_ptr()) }
    };
    let need = |symbol: &str| match find(symbol) {
        Some(p) => Ok(p),
        None => Err(format!("dismapi.dll has no {symbol}")),
    };
    // SAFETY: each pointer is the function of dismapi.h of that name, with the signature of
    // the type it becomes.
    unsafe {
        Ok(Api {
            initialize: std::mem::transmute::<Proc, Initialize>(need("DismInitialize")?),
            open_session: std::mem::transmute::<Proc, OpenSession>(need("DismOpenSession")?),
            get_features: std::mem::transmute::<Proc, GetFeatures>(need("DismGetFeatures")?),
            get_capabilities: find("DismGetCapabilities")
                .map(|p| std::mem::transmute::<Proc, GetCapabilities>(p)),
            remove_capability: find("DismRemoveCapability")
                .map(|p| std::mem::transmute::<Proc, RemoveCapability>(p)),
            delete: std::mem::transmute::<Proc, Delete>(need("DismDelete")?),
            close_session: std::mem::transmute::<Proc, CloseSession>(need("DismCloseSession")?),
        })
    }
}

fn api() -> Result<&'static Api, String> {
    static API: OnceLock<Result<Api, String>> = OnceLock::new();
    API.get_or_init(load).as_ref().map_err(Clone::clone)
}

/// The items of one call of DISM, with their names and states; the array is deleted after.
fn items(
    api: &Api,
    call: &str,
    get: impl FnOnce(*mut *mut Item, *mut u32) -> i32,
) -> Result<Vec<(String, u32)>, String> {
    let mut array: *mut Item = std::ptr::null_mut();
    let mut count = 0u32;
    let hr = get(&mut array, &mut count);
    if hr < 0 {
        return Err(failed(call, hr));
    }
    if array.is_null() {
        return Ok(Vec::new());
    }
    let list = (0..count as usize)
        .map(|i| {
            // SAFETY: DISM gave `count` packed items at `array`; they are read unaligned.
            let item = unsafe { std::ptr::read_unaligned(array.add(i)) };
            // SAFETY: DISM gives terminated names that live until the array is deleted
            // below.
            (unsafe { wide_text(item.name) }, item.state)
        })
        .collect();
    // SAFETY: the array came from DISM and is deleted once.
    unsafe { (api.delete)(array.cast_const().cast()) };
    Ok(list)
}

/// The state of a feature as `Get-WindowsOptionalFeature` names it.
fn feature_state(state: u32) -> &'static str {
    match state {
        1 => "DisablePending",
        2 => "Disabled",
        3 => "DisabledWithPayloadRemoved",
        4 => "Enabled",
        5 => "EnablePending",
        6 => "Superseded",
        7 => "PartiallyInstalled",
        _ => "NotPresent",
    }
}

/// The state of a capability as `Get-WindowsCapability` names it.
fn capability_state(state: u32) -> &'static str {
    match state {
        1 => "UninstallPending",
        2 => "Staged",
        3 => "Removed",
        INSTALLED => "Installed",
        5 => "InstallPending",
        6 => "Superseded",
        7 => "PartiallyInstalled",
        _ => "NotPresent",
    }
}

fn named(list: Vec<(String, u32)>, name: fn(u32) -> &'static str) -> Vec<(String, String)> {
    list.into_iter().map(|(n, s)| (n, name(s).into())).collect()
}

/// Runs `work` in a session of DISM on the running Windows, one session at a time.
fn in_session<T>(work: impl FnOnce(&Api, u32) -> T) -> Result<T, String> {
    // Whether DISM was initialized; a failed start is tried again at the next call.
    static STARTED: Mutex<bool> = Mutex::new(false);
    let mut started = STARTED.lock().unwrap_or_else(|e| e.into_inner());
    let api = api()?;
    if !*started {
        // The log of DISM goes to the temp folder instead of the folder of Windows.
        let path = std::env::temp_dir().join("winweaver-dism.log");
        let log = wide(&path.to_string_lossy());
        // SAFETY: the log path is a terminated wide string; no scratch folder is given.
        let hr = unsafe { (api.initialize)(LOG_ERRORS, log.as_ptr(), std::ptr::null()) };
        if hr < 0 {
            return Err(failed("DismInitialize", hr));
        }
        *started = true;
    }
    let image = wide(ONLINE_IMAGE);
    let mut session = 0u32;
    let null = std::ptr::null();
    // SAFETY: the image is a terminated wide string; the session is written on success.
    let hr = unsafe { (api.open_session)(image.as_ptr(), null, null, &mut session) };
    if hr < 0 {
        return Err(failed("DismOpenSession", hr));
    }
    let done = work(api, session);
    // SAFETY: the session was opened above and is closed once.
    unsafe { (api.close_session)(session) };
    Ok(done)
}

/// The capabilities of an open session with their states.
fn capabilities(api: &Api, session: u32) -> Result<Vec<(String, String)>, String> {
    let Some(get) = api.get_capabilities else {
        return Err("this Windows has no capabilities in DISM".to_owned());
    };
    items(api, "DismGetCapabilities", |array, count| {
        // SAFETY: an open session; DISM writes the array and its count.
        unsafe { get(session, array, count) }
    })
    .map(|list| named(list, capability_state))
}

/// Reads the features and the capabilities in one session of DISM.
pub fn components() -> Components {
    let read = in_session(|api, session| {
        let features = items(api, "DismGetFeatures", |array, count| {
            let null = std::ptr::null();
            // SAFETY: an open session; DISM writes the array and its count.
            unsafe { (api.get_features)(session, null, PACKAGE_NONE, array, count) }
        })
        .map(|list| named(list, feature_state));
        Components {
            features,
            capabilities: capabilities(api, session),
        }
    });
    read.unwrap_or_else(|e| Components {
        features: Err(e.clone()),
        capabilities: Err(e),
    })
}

/// What a removal of capabilities did.
#[derive(Debug, Default)]
pub struct Removal {
    /// The names of the capabilities removed.
    pub removed: Vec<String>,
    /// Whether Windows wants a restart to finish.
    pub restart: bool,
    /// The capabilities that could not be removed, each with its error.
    pub failed: Vec<String>,
}

/// Removes every installed capability whose name matches a pattern of `Get-WindowsCapability
/// -Name`, as `Remove-Capability` of the apply script does.
pub fn remove_capabilities(pattern: &str) -> Result<Removal, String> {
    in_session(|api, session| {
        let Some(remove) = api.remove_capability else {
            return Err("this Windows cannot remove capabilities through DISM".to_owned());
        };
        let mut done = Removal::default();
        for (name, state) in capabilities(api, session)? {
            if state != "Installed" || !thispc::wildcard(pattern, &name) {
                continue;
            }
            let wide_name = wide(&name);
            let (cancel, null) = (std::ptr::null_mut(), std::ptr::null());
            // SAFETY: an open session and a terminated name; no cancel event, no progress.
            let hr = unsafe { remove(session, wide_name.as_ptr(), cancel, null, null) };
            if hr < 0 {
                done.failed
                    .push(format!("{name}: {}", failed("DismRemoveCapability", hr)));
            } else {
                done.restart |= hr == REBOOT_REQUIRED;
                done.removed.push(name);
            }
        }
        Ok(done)
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_states_have_the_names_of_powershell() {
        assert_eq!(feature_state(4), "Enabled");
        assert_eq!(feature_state(2), "Disabled");
        assert_eq!(feature_state(3), "DisabledWithPayloadRemoved");
        assert_eq!(capability_state(INSTALLED), "Installed");
        assert_eq!(capability_state(0), "NotPresent");
        assert_eq!(
            std::mem::size_of::<Item>(),
            std::mem::size_of::<usize>() + 4
        );
    }

    /// DISM answers an administrator, which the CI runner is; elsewhere it must refuse with a
    /// message and never crash.
    #[test]
    #[ignore = "changes this Windows: run in CI or a VM, AGENTS.md section 5"]
    fn the_running_windows_lists_its_features_and_capabilities() {
        let read = components();
        if crate::native::elevated() {
            let features = read.features.expect("features");
            assert!(features.len() > 20, "{features:?}");
            assert!(features.iter().all(|(n, _)| !n.is_empty()), "{features:?}");
            assert!(features.iter().any(|(_, s)| s == "Enabled"), "{features:?}");
            let capabilities = read.capabilities.expect("capabilities");
            assert!(!capabilities.is_empty());
            // Names such as OpenSSH.Client~~~~0.0.1.0 tell that the packed items were read right.
            assert!(
                capabilities.iter().all(|(n, _)| n.contains('~')),
                "{capabilities:?}"
            );
            // A second read in the same process goes on in the same start of DISM.
            assert!(components().features.is_ok());
            // A pattern that matches nothing removes nothing.
            let none = remove_capabilities("WinWeaver.Nothing~~~~*").expect("a removal");
            assert!(
                none.removed.is_empty() && none.failed.is_empty(),
                "{none:?}"
            );
        } else if let Err(e) = read.features {
            assert!(!e.is_empty());
        }
    }
}
