//! Windows GUI startup, with optional CLI output to the launching terminal.

use std::ffi::c_void;

type Handle = *mut c_void;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn AttachConsole(process_id: u32) -> i32;
    fn GetStdHandle(kind: u32) -> Handle;
    fn SetStdHandle(kind: u32, handle: Handle) -> i32;
    fn GetFileType(handle: Handle) -> u32;
}

pub fn attach_parent_console() {
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    const STANDARD_HANDLES: [u32; 3] = [(-10_i32) as u32, (-11_i32) as u32, (-12_i32) as u32];
    const FILE_TYPE_DISK: u32 = 1;
    const FILE_TYPE_PIPE: u32 = 3;

    // SAFETY: These calls use Win32's documented pseudo IDs and process-owned handles.
    // No handles are closed or transferred. This runs before parsing or starting workers.
    unsafe {
        let redirected = STANDARD_HANDLES.map(|kind| {
            let handle = GetStdHandle(kind);
            let keep = !handle.is_null()
                && handle as isize != -1
                && matches!(GetFileType(handle), FILE_TYPE_DISK | FILE_TYPE_PIPE);
            (kind, handle, keep)
        });
        // An Explorer launch has no parent console; failure there is expected.
        // Never allocate a console, even when a shortcut includes CLI arguments.
        if AttachConsole(ATTACH_PARENT_PROCESS) != 0 {
            for (kind, handle, keep) in redirected {
                if keep {
                    // Attaching must not send redirected output back to the terminal.
                    SetStdHandle(kind, handle);
                }
            }
        }
    }
}

pub fn show_error(error: &anyhow::Error) {
    rfd::MessageDialog::new()
        .set_title("OJ Agent — 启动失败 / Startup failed")
        .set_description(format!(
            "无法启动应用 / Could not start the application.\n\n{error:#}"
        ))
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}
