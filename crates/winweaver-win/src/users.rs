//! The local accounts of a users file on this PC: read into a file, and created or changed by a
//! plan (`winweaver_core::users::Plan`) through the network management API, with the groups by
//! their identifiers, the folder of a profile with its access list and its registration. An
//! account is never deleted, and a profile that exists elsewhere is never moved.

use windows_sys::Win32::NetworkManagement::NetManagement::{
    LOCALGROUP_MEMBERS_INFO_0, NERR_Success, NetLocalGroupAddMembers, NetLocalGroupDelMembers,
    NetUserAdd, NetUserGetInfo, NetUserSetInfo, UF_ACCOUNTDISABLE, UF_DONT_EXPIRE_PASSWD,
    UF_NORMAL_ACCOUNT, UF_SCRIPT, USER_INFO_1, USER_INFO_20, USER_INFO_1003, USER_INFO_1007,
    USER_INFO_1011, USER_PRIV_USER,
};
use windows_sys::Win32::Security::{WinBuiltinAdministratorsSid, WinBuiltinUsersSid};
use winreg::RegKey;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, RegType};
use winweaver_core::registry::Registry;
use winweaver_core::users::{Operation, Outcome, Plan, State, Status, User};
use winweaver_core::userstore::Passwords;
use zeroize::Zeroize;

use crate::accounts::{self, DISABLED, NEVER_EXPIRES};
use crate::ffi::{
    DaclError, NetBuffer, Sid, USER_NOT_FOUND, each_user, net_error, set_dacl, user_name,
    utf16_bytes, wide, wide_text,
};
use crate::native::{builtin_group, groups_of};
use crate::registry::NativeRegistry;

const PROFILE_LIST: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList";
/// The account is already in the group (ERROR_MEMBER_IN_ALIAS) or not in it
/// (ERROR_MEMBER_NOT_IN_ALIAS): no change was needed.
const IN_ALIAS: u32 = 1378;
const NOT_IN_ALIAS: u32 = 1377;

/// The names of the two groups in the language of this Windows: Administrators, Users.
fn group_names() -> Result<(String, String), String> {
    let admins = builtin_group(WinBuiltinAdministratorsSid)
        .ok_or("the name of the group Administrators could not be read")?;
    let users =
        builtin_group(WinBuiltinUsersSid).ok_or("the name of the group Users could not be read")?;
    Ok((admins, users))
}

/// The accounts of this PC as a users file lists them: the normal accounts but the built-in
/// ones, with their group, their flags and the folder of their profile; no password.
pub fn read_this_pc() -> Result<Vec<User>, String> {
    let (admins, users) = group_names()?;
    let mut found = Vec::new();
    each_user(|entry| {
        if matches!(entry.usri20_user_id, 500 | 501 | 503 | 504) {
            return;
        }
        // SAFETY: the texts are null-terminated strings inside the buffer of the call.
        let (name, full_name, description) = unsafe {
            (
                wide_text(entry.usri20_name),
                wide_text(entry.usri20_full_name),
                wide_text(entry.usri20_comment),
            )
        };
        let groups = groups_of(&name);
        let in_group = |g: &str| groups.iter().any(|x| x.eq_ignore_ascii_case(g));
        let disabled = entry.usri20_flags & UF_ACCOUNTDISABLE != 0;
        found.push(User {
            profile_path: NativeRegistry
                .account_sid(&name)
                .and_then(|sid| profile_of(&sid))
                .unwrap_or_default(),
            name,
            full_name,
            description,
            admin: in_group(&admins),
            password_never_expires: entry.usri20_flags & UF_DONT_EXPIRE_PASSWD != 0,
            apply_password: false,
            state: if disabled {
                State::Disabled
            } else {
                State::Enabled
            },
            force_group: !in_group(&admins) && !in_group(&users),
        });
    })
    .map_err(|status| format!("the local accounts were not read ({})", net_error(status)))?;
    Ok(found)
}

/// The folder of the profile of an identifier, when Windows registered one.
fn profile_of(sid: &str) -> Option<String> {
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(format!("{PROFILE_LIST}\\{sid}"), KEY_READ)
        .ok()?;
    key.get_value::<String, _>("ProfileImagePath")
        .ok()
        .filter(|p| !p.is_empty())
}

/// What an account holds: its flags, its full name and its description; `None` when there is
/// no such account.
fn info(name: &str) -> Result<Option<(u32, String, String)>, String> {
    let name_w = wide(name);
    let mut buffer = NetBuffer::default();
    // SAFETY: the name is null-terminated; the call allocates the buffer of level 20.
    let status = unsafe { NetUserGetInfo(std::ptr::null(), name_w.as_ptr(), 20, buffer.out()) };
    if status == USER_NOT_FOUND {
        return Ok(None);
    }
    if status != NERR_Success || buffer.get().is_null() {
        return Err(format!(
            "the account {name} was not read ({})",
            net_error(status)
        ));
    }
    // SAFETY: on success the buffer holds one entry of level 20 with null-terminated texts.
    let found = unsafe {
        let entry = &*buffer.get().cast::<USER_INFO_20>();
        (
            entry.usri20_flags,
            wide_text(entry.usri20_full_name),
            wide_text(entry.usri20_comment),
        )
    };
    Ok(Some(found))
}

/// A level of NetUserSetInfo with one structure.
fn set_info(name: &str, level: u32, info: *const u8, what: &str) -> Result<(), String> {
    let name_w = wide(name);
    let mut bad = 0u32;
    // SAFETY: the name is null-terminated and the structure is the one of the level.
    let status =
        unsafe { NetUserSetInfo(std::ptr::null(), name_w.as_ptr(), level, info, &mut bad) };
    if status != NERR_Success {
        return Err(format!("{what} was not set ({})", net_error(status)));
    }
    Ok(())
}

/// Sets the password of an account; the buffer is wiped after the call.
fn set_password(name: &str, password: &str) -> Result<(), String> {
    let mut password_w = wide(password);
    let info = USER_INFO_1003 {
        usri1003_password: password_w.as_mut_ptr(),
    };
    let result = set_info(name, 1003, (&raw const info).cast(), "the password");
    password_w.zeroize();
    result
}

fn set_full_name(name: &str, full_name: &str) -> Result<(), String> {
    let mut text = wide(full_name);
    let info = USER_INFO_1011 {
        usri1011_full_name: text.as_mut_ptr(),
    };
    set_info(name, 1011, (&raw const info).cast(), "the full name")
}

fn set_description(name: &str, description: &str) -> Result<(), String> {
    let mut text = wide(description);
    let info = USER_INFO_1007 {
        usri1007_comment: text.as_mut_ptr(),
    };
    set_info(name, 1007, (&raw const info).cast(), "the description")
}

/// Creates an account with its password, its description and its flags.
fn create(user: &User, password: &str) -> Result<(), String> {
    let mut name_w = wide(&user.name);
    let mut password_w = wide(password);
    let mut comment_w = wide(&user.description);
    let mut flags = UF_SCRIPT | UF_NORMAL_ACCOUNT;
    if user.state == State::Disabled {
        flags |= UF_ACCOUNTDISABLE;
    }
    if user.password_never_expires {
        flags |= UF_DONT_EXPIRE_PASSWD;
    }
    let info = USER_INFO_1 {
        usri1_name: name_w.as_mut_ptr(),
        usri1_password: password_w.as_mut_ptr(),
        usri1_password_age: 0,
        usri1_priv: USER_PRIV_USER,
        usri1_home_dir: std::ptr::null_mut(),
        usri1_comment: comment_w.as_mut_ptr(),
        usri1_flags: flags,
        usri1_script_path: std::ptr::null_mut(),
    };
    let mut bad = 0u32;
    let buffer = (&raw const info).cast::<u8>();
    // SAFETY: every text is null-terminated and the structure is the one of level 1.
    let status = unsafe { NetUserAdd(std::ptr::null(), 1, buffer, &mut bad) };
    password_w.zeroize();
    if status != NERR_Success {
        return Err(format!(
            "the account was not created ({})",
            net_error(status)
        ));
    }
    Ok(())
}

/// Adds an account to a group or removes it; no change is no error.
fn membership(group: &str, sid: &Sid, add: bool) -> Result<(), String> {
    let group_w = wide(group);
    let member = LOCALGROUP_MEMBERS_INFO_0 { lgrmi0_sid: sid.0 };
    let buffer = (&raw const member).cast::<u8>();
    // SAFETY: the name is null-terminated and the structure is one entry of level 0.
    let status = unsafe {
        if add {
            NetLocalGroupAddMembers(std::ptr::null(), group_w.as_ptr(), 0, buffer, 1)
        } else {
            NetLocalGroupDelMembers(std::ptr::null(), group_w.as_ptr(), 0, buffer, 1)
        }
    };
    if status == NERR_Success || status == IN_ALIAS || status == NOT_IN_ALIAS {
        return Ok(());
    }
    let what = if add { "added to" } else { "removed from" };
    Err(format!(
        "not {what} the group {group} ({})",
        net_error(status)
    ))
}

/// The identifier of the account this process runs as, which is never removed from
/// Administrators.
fn current_sid() -> Option<String> {
    let name = user_name();
    if name.is_empty() {
        return None;
    }
    NativeRegistry.account_sid(&name)
}

/// Brings the groups of an account in line with `admin`: the wanted group is added, and with
/// `strict` the other one is removed. The changes, as the script names them.
fn set_groups(name: &str, sid: &str, admin: bool, strict: bool) -> Result<Vec<String>, String> {
    let (admins, users) = group_names()?;
    let (wanted, other) = if admin {
        (&admins, &users)
    } else {
        (&users, &admins)
    };
    let groups = groups_of(name);
    let in_group = |g: &str| groups.iter().any(|x| x.eq_ignore_ascii_case(g));
    let psid = Sid::parse(sid)?;
    let mut changes = Vec::new();
    if !in_group(wanted) {
        membership(wanted, &psid, true)?;
        changes.push(if admin { "+Administrators" } else { "+Users" }.to_owned());
    }
    if strict && in_group(other) {
        if !admin && current_sid().is_some_and(|c| c.eq_ignore_ascii_case(sid)) {
            changes.push("Administrators kept (current user)".to_owned());
        } else {
            membership(other, &psid, false)?;
            changes.push(if admin { "-Users" } else { "-Administrators" }.to_owned());
        }
    }
    Ok(changes)
}

/// Gives a folder the access list of an SDDL text, protected from the entries of its parent.
fn set_folder_access(folder: &str, dacl: &str) -> Result<(), String> {
    use windows_sys::Win32::Security::Authorization::SE_FILE_OBJECT;
    match set_dacl(SE_FILE_OBJECT, folder, dacl, true) {
        Ok(()) => Ok(()),
        Err(DaclError::NotTaken) => {
            Err("Windows does not take the access list of the folder".to_owned())
        }
        Err(DaclError::Empty) => Err("the access list of the folder is empty".to_owned()),
        Err(DaclError::Status(code)) => Err(format!(
            "the access list of the folder was not set (error {code})"
        )),
    }
}

/// Makes an identifier the owner of a folder, which needs a privilege Windows may refuse.
fn set_folder_owner(folder: &str, sid: &str) -> Result<(), String> {
    use windows_sys::Win32::Security::Authorization::{SE_FILE_OBJECT, SetNamedSecurityInfoW};
    use windows_sys::Win32::Security::OWNER_SECURITY_INFORMATION;
    let mut name = wide(folder);
    let psid = Sid::parse(sid)?;
    let none = std::ptr::null_mut();
    // SAFETY: the name is null-terminated and the identifier lives as long as `psid`.
    let status = unsafe {
        SetNamedSecurityInfoW(
            name.as_mut_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            psid.0,
            none,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if status == 0 {
        Ok(())
    } else {
        Err(format!("error {status}"))
    }
}

/// Whether the identifier has a profile registered in another folder.
fn profile_elsewhere(sid: &str, folder: &str) -> bool {
    profile_of(sid).is_some_and(|p| !p.eq_ignore_ascii_case(folder.trim_end_matches('\\')))
}

/// Makes the folder of a profile, lets only SYSTEM, Administrators and the account into it, and
/// registers it for the identifier. A refused owner is named, not an error.
fn make_profile(sid: &str, folder: &str) -> Result<Option<String>, String> {
    std::fs::create_dir_all(folder).map_err(|e| format!("the folder {folder}: {e}"))?;
    let dacl = format!("D:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;FA;;;{sid})");
    set_folder_access(folder, &dacl)?;
    let warning = match set_folder_owner(folder, sid) {
        Ok(()) => None,
        Err(e) => Some(format!("the owner of the folder was not changed: {e}")),
    };
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .create_subkey(format!("{PROFILE_LIST}\\{sid}"))
        .map_err(|e| format!("the profile was not registered: {e}"))?
        .0;
    let path = winreg::RegValue {
        bytes: utf16_bytes(folder).into(),
        vtype: RegType::REG_EXPAND_SZ,
    };
    key.set_raw_value("ProfileImagePath", &path)
        .and_then(|()| key.set_value("Flags", &0u32))
        .and_then(|()| key.set_value("State", &0u32))
        .map_err(|e| format!("the profile was not registered: {e}"))?;
    Ok(warning)
}

/// The changes to an account that exists, as the plan orders them.
fn update(user: &User, password: &str, update_existing: bool) -> Result<Vec<String>, String> {
    let Some((flags, full_name, description)) = info(&user.name)? else {
        return Err("the account disappeared".to_owned());
    };
    let mut changes = Vec::new();
    if user.apply_password {
        set_password(&user.name, password)?;
        changes.push("Password".to_owned());
    }
    let disabled = flags & UF_ACCOUNTDISABLE != 0;
    match user.state {
        State::Enabled if disabled => {
            accounts::set_flag(&user.name, DISABLED, false)?;
            changes.push("Enabled".to_owned());
        }
        State::Disabled if !disabled => {
            accounts::set_flag(&user.name, DISABLED, true)?;
            changes.push("Disabled".to_owned());
        }
        _ => {}
    }
    let sid = NativeRegistry
        .account_sid(&user.name)
        .ok_or("the identifier of the account was not read")?;
    if user.force_group {
        changes.extend(set_groups(&user.name, &sid, user.admin, true)?);
    } else if update_existing {
        changes.extend(set_groups(&user.name, &sid, user.admin, false)?);
    }
    if !update_existing {
        return Ok(changes);
    }
    let never = flags & UF_DONT_EXPIRE_PASSWD != 0;
    if never != user.password_never_expires {
        let on = user.password_never_expires;
        accounts::set_flag(&user.name, NEVER_EXPIRES, on)?;
        changes.push("PasswordNeverExpires".to_owned());
    }
    if !user.full_name.is_empty() && user.full_name != full_name {
        set_full_name(&user.name, &user.full_name)?;
        changes.push("FullName".to_owned());
    }
    if !user.description.is_empty() && user.description != description {
        set_description(&user.name, &user.description)?;
        changes.push("Description".to_owned());
    }
    if !user.profile_path.is_empty() {
        if profile_elsewhere(&sid, &user.profile_path) {
            changes.push("ProfilePath kept (an existing profile)".to_owned());
        } else {
            if let Some(warning) = make_profile(&sid, &user.profile_path)? {
                changes.push(warning);
            }
            changes.push("ProfilePath".to_owned());
        }
    }
    Ok(changes)
}

/// Creates an account and configures it; a failure after the creation says so, since the next
/// run would report the account as existing.
fn create_and_configure(user: &User, password: &str) -> Result<Vec<String>, String> {
    create(user, password)?;
    let configure = || -> Result<Vec<String>, String> {
        let sid = NativeRegistry
            .account_sid(&user.name)
            .ok_or("the identifier of the account was not read")?;
        let mut notes = set_groups(&user.name, &sid, user.admin, true)?;
        notes.retain(|n| n.contains("kept"));
        if !user.full_name.is_empty() {
            set_full_name(&user.name, &user.full_name)?;
        }
        if !user.profile_path.is_empty()
            && let Some(warning) = make_profile(&sid, &user.profile_path)?
        {
            notes.push(warning);
        }
        Ok(notes)
    };
    configure().map_err(|e| format!("the account was created, but its configuration failed: {e}"))
}

/// One account of the plan.
fn one(plan: &Plan, user: &User, password: &str) -> Outcome {
    let outcome = |status: Status, message: String| Outcome {
        name: user.name.clone(),
        status,
        message,
    };
    let exists = match info(&user.name) {
        Ok(found) => found.is_some(),
        Err(e) => return outcome(Status::Error, e),
    };
    let done: Result<(Status, String), String> = match plan.operation {
        Operation::Passwords if !exists => {
            Ok((Status::NotFound, "create the account first".to_owned()))
        }
        Operation::Passwords => {
            set_password(&user.name, password).map(|()| (Status::PasswordSet, String::new()))
        }
        Operation::Create { update_existing } if exists => update(user, password, update_existing)
            .map(|changes| {
                if changes.is_empty() {
                    (Status::AlreadyExists, "left as it is".to_owned())
                } else {
                    (Status::Updated, changes.join(", "))
                }
            }),
        Operation::Create { .. } => {
            create_and_configure(user, password).map(|notes| (Status::Created, notes.join(", ")))
        }
    };
    match done {
        Ok((status, message)) => outcome(status, message),
        Err(e) => outcome(Status::Error, e),
    }
}

/// Runs a plan on this PC: one outcome an account, in the order of the plan; an error of one
/// account does not stop the others. Needs the rights of an administrator.
pub fn run(plan: &Plan, passwords: &Passwords) -> Vec<Outcome> {
    plan.users
        .iter()
        .map(|user| match passwords.get(&user.name) {
            Some(password) => one(plan, user, password),
            None => Outcome {
                name: user.name.clone(),
                status: Status::Skipped,
                message: "no password".to_owned(),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On the CI runner, as administrator: an account is created with its group, its flags and
    /// its password, updated by its orders, read back, and deleted again (the program never
    /// deletes one; the test cleans up with NetUserDel).
    #[test]
    fn an_account_is_created_updated_and_read_on_the_runner() {
        use windows_sys::Win32::NetworkManagement::NetManagement::NetUserDel;
        if std::env::var("GITHUB_ACTIONS").is_err() {
            return;
        }
        let name = "WwUsersTest";
        let mut passwords = Passwords::default();
        passwords.set(name, "Kx7!Secret-Pass1");
        let user = User {
            name: name.to_owned(),
            full_name: "WinWeaver test".to_owned(),
            description: "made by a test".to_owned(),
            admin: false,
            password_never_expires: true,
            state: State::Disabled,
            ..User::default()
        };
        let plan = Plan {
            operation: Operation::Create {
                update_existing: false,
            },
            users: vec![user.clone()],
        };
        let outcomes = run(&plan, &passwords);
        assert_eq!(outcomes[0].status, Status::Created, "{outcomes:?}");
        let (flags, full_name, description) = info(name).unwrap().unwrap();
        assert!(flags & UF_DONT_EXPIRE_PASSWD != 0 && flags & UF_ACCOUNTDISABLE != 0);
        assert_eq!(full_name, "WinWeaver test");
        assert_eq!(description, "made by a test");
        let read = read_this_pc().unwrap();
        let found = read.iter().find(|u| u.name == name).unwrap();
        assert!(!found.admin && found.password_never_expires && !found.force_group);
        assert_eq!(found.state, State::Disabled);
        let again = run(&plan, &passwords);
        assert_eq!(again[0].status, Status::AlreadyExists);
        let changed = Plan {
            operation: Operation::Create {
                update_existing: false,
            },
            users: vec![User {
                admin: true,
                state: State::Enabled,
                force_group: true,
                ..user
            }],
        };
        let updated = run(&changed, &passwords);
        assert_eq!(updated[0].status, Status::Updated, "{updated:?}");
        assert!(
            updated[0].message.contains("+Administrators"),
            "{updated:?}"
        );
        assert!(updated[0].message.contains("Enabled"));
        // The policy of the PC may refuse a password used before, and one that holds a part of
        // the full name of the account.
        passwords.set(name, "Kx7!Secret-Pass2");
        let only = Plan {
            operation: Operation::Passwords,
            users: changed.users.clone(),
        };
        let set = run(&only, &passwords);
        assert_eq!(set[0].status, Status::PasswordSet, "{set:?}");
        let name_w = wide(name);
        // SAFETY: the name is null-terminated.
        assert_eq!(
            unsafe { NetUserDel(std::ptr::null(), name_w.as_ptr()) },
            NERR_Success
        );
        assert_eq!(run(&only, &passwords)[0].status, Status::NotFound);
    }
}
