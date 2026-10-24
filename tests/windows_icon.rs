#![cfg(windows)]

use std::{ffi::c_void, os::windows::ffi::OsStrExt};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryExW(path: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
    fn FindResourceW(module: *mut c_void, name: *const u16, kind: *const u16) -> *mut c_void;
    fn SizeofResource(module: *mut c_void, resource: *mut c_void) -> u32;
    fn LoadResource(module: *mut c_void, resource: *mut c_void) -> *mut c_void;
    fn LockResource(resource: *mut c_void) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn ExtractIconExW(
        path: *const u16,
        index: i32,
        large: *mut *mut c_void,
        small: *mut *mut c_void,
        count: u32,
    ) -> u32;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn DestroyIcon(icon: *mut c_void) -> i32;
}

fn resource(module: *mut c_void, kind: u16, id: u16) -> Vec<u8> {
    // SAFETY: The live data-file module owns these resource handles. Integer
    // resource identifiers follow MAKEINTRESOURCEW; bytes are copied before unload.
    unsafe {
        let entry = FindResourceW(
            module,
            id as usize as *const u16,
            kind as usize as *const u16,
        );
        assert!(!entry.is_null(), "missing resource type {kind}, ID {id}");
        let len = SizeofResource(module, entry) as usize;
        assert!(len > 0);
        let loaded = LoadResource(module, entry);
        assert!(!loaded.is_null());
        let data = LockResource(loaded);
        assert!(!data.is_null());
        std::slice::from_raw_parts(data.cast::<u8>(), len).to_vec()
    }
}

#[test]
fn executable_embeds_the_same_icon_as_the_slint_window() {
    let path: Vec<u16> = std::ffi::OsStr::new(env!("CARGO_BIN_EXE_oj-agent"))
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: The path is NUL-terminated. DATAFILE | IMAGE_RESOURCE maps the
    // executable for resource inspection without executing its entry point.
    let module = unsafe { LoadLibraryExW(path.as_ptr(), std::ptr::null_mut(), 0x02 | 0x20) };
    assert!(!module.is_null(), "{}", std::io::Error::last_os_error());
    let group = resource(module, 14, 1); // RT_GROUP_ICON, first shell icon.
    assert_eq!(u16::from_le_bytes(group[2..4].try_into().unwrap()), 1);
    assert_eq!(u16::from_le_bytes(group[4..6].try_into().unwrap()), 1);
    assert_eq!(&group[6..8], &[0, 0]); // Native 256 x 256 pixels.
    let id = u16::from_le_bytes(group[18..20].try_into().unwrap());
    let icon = resource(module, 3, id); // RT_ICON.
    // SAFETY: All resource bytes were copied; this balances LoadLibraryExW.
    assert_ne!(unsafe { FreeLibrary(module) }, 0);
    assert_eq!(icon, include_bytes!("../ui/icons/app-icon.png"));

    let mut large = std::ptr::null_mut();
    let mut small = std::ptr::null_mut();
    // SAFETY: The valid path and output pointers let the Windows shell extract
    // both display sizes without opening a window. Both owned icons are released.
    unsafe {
        assert_eq!(
            ExtractIconExW(path.as_ptr(), 0, &mut large, &mut small, 1),
            2 // One large icon and one small icon.
        );
        assert!(!large.is_null() && !small.is_null());
        assert_ne!(DestroyIcon(large), 0);
        assert_ne!(DestroyIcon(small), 0);
    }
}
