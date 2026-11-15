use std::path::Path;
use std::ptr;

use windows_sys::Win32::Foundation::{ERROR_SUCCESS, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT,
};
use windows_sys::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, DACL_SECURITY_INFORMATION, GetAce,
    OWNER_SECURITY_INFORMATION, PSID,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, GetDriveTypeW, GetFileAttributesW, INVALID_FILE_ATTRIBUTES,
};

use crate::wide::{from_wide_pointer, to_wide};

// Constants
const TRUSTED_SIDS: &[&str] = &[
    "S-1-5-18",
    "S-1-5-32-544",
    "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464",
];
const DRIVE_NO_ROOT_DIR: u32 = 1;
const DRIVE_FIXED: u32 = 3;
const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
const DENIED_ACE_TYPES: &[u8] = &[1, 6, 10, 12];
const INHERIT_ONLY_ACE: u8 = 0x08;
const FILE_WRITE_DATA: u32 = 0x0002;
const FILE_APPEND_DATA: u32 = 0x0004;
const FILE_DELETE_CHILD: u32 = 0x0040;
const DELETE: u32 = 0x0001_0000;
const WRITE_DAC: u32 = 0x0004_0000;
const WRITE_OWNER: u32 = 0x0008_0000;
const GENERIC_ALL: u32 = 0x1000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const TAKEOVER_RIGHTS: u32 =
    FILE_DELETE_CHILD | DELETE | WRITE_DAC | WRITE_OWNER | GENERIC_ALL | GENERIC_WRITE;
const CONTENT_RIGHTS: u32 = FILE_WRITE_DATA | FILE_APPEND_DATA;

// Roles
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    File,
    Folder,
    Ancestor,
}

impl Role {
    fn forbidden_rights(self) -> u32 {
        match self {
            Self::File | Self::Folder => TAKEOVER_RIGHTS | CONTENT_RIGHTS,
            Self::Ancestor => TAKEOVER_RIGHTS,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    kind: u8,
    flags: u8,
    mask: u32,
    sid: String,
}

#[derive(Debug, PartialEq, Eq)]
struct Descriptor {
    owner: String,
    entries: Option<Vec<Entry>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Problem {
    Missing(String),
    Unsafe(String),
}

impl Problem {
    pub fn english(&self) -> String {
        match self {
            Self::Missing(path) => not_found(path),
            Self::Unsafe(message) => message.clone(),
        }
    }
}

// Checks
pub fn check_file(path: &str) -> Result<(), String> {
    check_exists(path)?;
    let mut role = Role::File;
    for current in Path::new(path).ancestors() {
        let Some(current) = current.to_str().filter(|text| !text.is_empty()) else {
            break;
        };
        let descriptor = read_descriptor(current).map_err(|code| {
            format!("The permissions of {current} cannot be read (error {code}).")
        })?;
        if let Some(problem) = evaluate(role, &descriptor) {
            return Err(format!("{current} {problem}."));
        }
        role = match role {
            Role::File => Role::Folder,
            Role::Folder | Role::Ancestor => Role::Ancestor,
        };
    }
    Ok(())
}

pub fn check_local_program(path: &str) -> Result<(), Problem> {
    let root: String = path.chars().take(3).collect();
    match unsafe { GetDriveTypeW(to_wide(&root).as_ptr()) } {
        DRIVE_FIXED => {}
        DRIVE_NO_ROOT_DIR => return Err(Problem::Missing(path.to_owned())),
        _ => {
            return Err(Problem::Unsafe(format!(
                "{root} is not a local fixed drive. Use a UNC path (\\\\server\\share\\...) for programs on the network."
            )));
        }
    }
    check_present(path)?;
    check_file(path).map_err(Problem::Unsafe)
}

pub fn check_network_program(path: &str) -> Result<(), Problem> {
    check_present(path)
}

fn check_present(path: &str) -> Result<(), Problem> {
    let attributes = unsafe { GetFileAttributesW(to_wide(path).as_ptr()) };
    if attributes == INVALID_FILE_ATTRIBUTES {
        return Err(Problem::Missing(path.to_owned()));
    }
    if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        return Err(Problem::Unsafe(format!("{path} is a folder, not a file.")));
    }
    Ok(())
}

fn check_exists(path: &str) -> Result<(), String> {
    check_present(path).map_err(|problem| problem.english())
}

fn not_found(path: &str) -> String {
    format!("{path} was not found or cannot be accessed.")
}

fn evaluate(role: Role, descriptor: &Descriptor) -> Option<String> {
    if !is_trusted(&descriptor.owner) {
        return Some(format!(
            "is owned by {}, not by SYSTEM, Administrators or TrustedInstaller",
            descriptor.owner
        ));
    }
    let Some(entries) = &descriptor.entries else {
        return Some("has no access control list and can be modified by everyone".to_owned());
    };
    for entry in entries {
        if DENIED_ACE_TYPES.contains(&entry.kind) || entry.flags & INHERIT_ONLY_ACE != 0 {
            continue;
        }
        if entry.kind != ACCESS_ALLOWED_ACE_TYPE {
            return Some(format!(
                "has an access control entry of the unsupported type {}",
                entry.kind
            ));
        }
        if entry.mask & role.forbidden_rights() != 0 && !is_trusted(&entry.sid) {
            return Some(format!(
                "can be modified by {} (access mask 0x{:08X})",
                entry.sid, entry.mask
            ));
        }
    }
    None
}

fn is_trusted(sid: &str) -> bool {
    TRUSTED_SIDS.contains(&sid)
}

// Windows
fn read_descriptor(path: &str) -> Result<Descriptor, u32> {
    let mut owner: PSID = ptr::null_mut();
    let mut dacl: *mut ACL = ptr::null_mut();
    let mut descriptor = ptr::null_mut();
    let result = unsafe {
        GetNamedSecurityInfoW(
            to_wide(path).as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &raw mut owner,
            ptr::null_mut(),
            &raw mut dacl,
            ptr::null_mut(),
            &raw mut descriptor,
        )
    };
    if result != ERROR_SUCCESS {
        return Err(result);
    }
    let read = (|| {
        let owner = sid_text(owner)?;
        let entries = if dacl.is_null() {
            None
        } else {
            Some(read_entries(dacl)?)
        };
        Ok(Descriptor { owner, entries })
    })();
    unsafe {
        LocalFree(descriptor);
    }
    read
}

fn read_entries(dacl: *const ACL) -> Result<Vec<Entry>, u32> {
    const INVALID_ACL: u32 = 1336;
    let count = unsafe { (*dacl).AceCount };
    let mut entries = Vec::with_capacity(usize::from(count));
    for index in 0..u32::from(count) {
        let mut ace = ptr::null_mut();
        if unsafe { GetAce(dacl, index, &raw mut ace) } == 0 || ace.is_null() {
            return Err(INVALID_ACL);
        }
        let header = unsafe { ptr::read_unaligned(ace.cast::<ACE_HEADER>()) };
        let (mask, sid) = if header.AceType == ACCESS_ALLOWED_ACE_TYPE {
            if usize::from(header.AceSize) < size_of::<ACCESS_ALLOWED_ACE>() {
                return Err(INVALID_ACL);
            }
            let allowed = ace.cast::<ACCESS_ALLOWED_ACE>();
            let mask = unsafe { ptr::read_unaligned(&raw const (*allowed).Mask) };
            let sid = unsafe { &raw mut (*allowed).SidStart }.cast();
            (mask, sid_text(sid)?)
        } else {
            (0, String::new())
        };
        entries.push(Entry {
            kind: header.AceType,
            flags: header.AceFlags,
            mask,
            sid,
        });
    }
    Ok(entries)
}

fn sid_text(sid: PSID) -> Result<String, u32> {
    const INVALID_SID: u32 = 1337;
    if sid.is_null() {
        return Err(INVALID_SID);
    }
    let mut text = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &raw mut text) } == 0 || text.is_null() {
        return Err(INVALID_SID);
    }
    let result = from_wide_pointer(text);
    unsafe {
        LocalFree(text.cast());
    }
    Ok(result)
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    const SYSTEM: &str = "S-1-5-18";
    const ADMINISTRATORS: &str = "S-1-5-32-544";
    const USERS: &str = "S-1-5-32-545";
    const AUTHENTICATED_USERS: &str = "S-1-5-11";
    const READ_EXECUTE: u32 = 0x0012_00A9;
    const MODIFY: u32 = 0x0013_01BF;
    const FULL: u32 = 0x001F_01FF;

    fn allow(sid: &str, mask: u32) -> Entry {
        Entry {
            kind: ACCESS_ALLOWED_ACE_TYPE,
            flags: 0,
            mask,
            sid: sid.to_owned(),
        }
    }

    fn descriptor(entries: Vec<Entry>) -> Descriptor {
        Descriptor {
            owner: ADMINISTRATORS.to_owned(),
            entries: Some(entries),
        }
    }

    fn standard() -> Vec<Entry> {
        vec![
            allow(SYSTEM, FULL),
            allow(ADMINISTRATORS, FULL),
            allow(USERS, READ_EXECUTE),
        ]
    }

    #[test]
    fn accepts_standard_permissions() {
        for role in [Role::File, Role::Folder, Role::Ancestor] {
            assert_eq!(evaluate(role, &descriptor(standard())), None);
        }
    }

    #[test]
    fn rejects_untrusted_owners() {
        let mut descriptor = descriptor(standard());
        descriptor.owner = "S-1-5-21-1-2-3-1001".to_owned();
        assert!(evaluate(Role::File, &descriptor).unwrap().contains("owned"));
    }

    #[test]
    fn rejects_a_missing_access_control_list() {
        let descriptor = Descriptor {
            owner: SYSTEM.to_owned(),
            entries: None,
        };
        assert!(evaluate(Role::File, &descriptor).is_some());
    }

    #[test]
    fn rejects_write_access_for_users() {
        for mask in [
            MODIFY,
            FULL,
            FILE_WRITE_DATA,
            FILE_APPEND_DATA,
            DELETE,
            WRITE_DAC,
            WRITE_OWNER,
            GENERIC_ALL,
            GENERIC_WRITE,
            FILE_DELETE_CHILD,
        ] {
            for role in [Role::File, Role::Folder] {
                let mut entries = standard();
                entries.push(allow(USERS, mask));
                assert!(
                    evaluate(role, &descriptor(entries)).is_some(),
                    "{role:?} {mask:#x}"
                );
            }
        }
    }

    #[test]
    fn allows_creating_folders_in_ancestors_only() {
        let mut entries = standard();
        entries.push(allow(AUTHENTICATED_USERS, FILE_APPEND_DATA));
        assert_eq!(evaluate(Role::Ancestor, &descriptor(entries)), None);
        for mask in [DELETE, FILE_DELETE_CHILD, WRITE_DAC, WRITE_OWNER, MODIFY] {
            let mut entries = standard();
            entries.push(allow(AUTHENTICATED_USERS, mask));
            assert!(
                evaluate(Role::Ancestor, &descriptor(entries)).is_some(),
                "{mask:#x}"
            );
        }
    }

    #[test]
    fn ignores_inherit_only_and_deny_entries() {
        let mut entries = standard();
        entries.push(Entry {
            flags: INHERIT_ONLY_ACE,
            ..allow(AUTHENTICATED_USERS, MODIFY)
        });
        entries.push(Entry {
            kind: 1,
            ..allow(USERS, FULL)
        });
        assert_eq!(evaluate(Role::Folder, &descriptor(entries)), None);
    }

    #[test]
    fn rejects_unsupported_entry_types() {
        let mut entries = standard();
        entries.push(Entry {
            kind: 9,
            ..allow(USERS, 0)
        });
        assert!(evaluate(Role::File, &descriptor(entries)).is_some());
    }

    #[test]
    fn accepts_windows_system_programs() {
        assert_eq!(check_file(r"C:\Windows\System32\mstsc.exe"), Ok(()));
        assert_eq!(
            check_local_program(r"C:\Windows\System32\mstsc.exe"),
            Ok(())
        );
    }

    #[test]
    fn rejects_files_in_the_temporary_folder() {
        let path = std::env::temp_dir().join(format!("urlbridge-test-{}.exe", std::process::id()));
        std::fs::write(&path, b"test").unwrap();
        let result = check_file(path.to_str().unwrap());
        std::fs::remove_file(&path).unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_files_and_folders() {
        assert!(check_file(r"C:\Windows\System32\urlbridge-missing.exe").is_err());
        assert!(check_file(r"C:\Windows\System32").is_err());
    }

    #[test]
    fn tells_missing_programs_from_unsafe_ones() {
        assert!(matches!(
            check_local_program(r"C:\Windows\System32\urlbridge-missing.exe"),
            Err(Problem::Missing(_))
        ));
        assert!(matches!(
            check_local_program(r"C:\urlbridge-missing\tool.exe"),
            Err(Problem::Missing(_))
        ));
        assert!(matches!(
            check_local_program(r"C:\Windows\System32"),
            Err(Problem::Unsafe(_))
        ));
        let path =
            std::env::temp_dir().join(format!("urlbridge-unsafe-{}.exe", std::process::id()));
        std::fs::write(&path, b"test").unwrap();
        let result = check_local_program(path.to_str().unwrap());
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(result, Err(Problem::Unsafe(_))));
        assert!(matches!(
            check_network_program(r"\\urlbridge-missing.invalid\share\tool.exe"),
            Err(Problem::Missing(_))
        ));
    }
}
