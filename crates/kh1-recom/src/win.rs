//! Windows entry point: runs the startup checks on a thread once Panacea loads the DLL.

use std::ffi::c_void;
use std::path::PathBuf;

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn GetModuleFileNameW(module: *mut c_void, buf: *mut u16, size: u32) -> u32;
}

const DLL_PROCESS_ATTACH: u32 = 1;

fn exe_path() -> String {
    let mut buf = vec![0u16; 1024];
    // SAFETY: a null module handle names the exe; the buffer length is passed.
    let n = unsafe { GetModuleFileNameW(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32) } as usize;
    String::from_utf16_lossy(&buf[..n.min(buf.len())])
}

fn startup() {
    // SAFETY: a null name returns the exe's own (already mapped) image base.
    let base = unsafe { GetModuleHandleW(std::ptr::null()) } as *const u8;
    if base.is_null() {
        return;
    }
    // SAFETY: the first 0x1000 bytes of a mapped PE image are its headers, always readable.
    let headers = unsafe { std::slice::from_raw_parts(base, 0x1000) };
    let pe = crate::sites::pe_info(headers);
    let size = pe.as_ref().map(|p| p.size_of_image).unwrap_or(0);
    let read_site = |rva: u32, n: usize| -> Vec<u8> {
        if (rva as u64 + n as u64) > size as u64 {
            return Vec::new();
        }
        // SAFETY: only called after the build check passed, with RVAs inside the image's code
        // section (the .text of the exact Steam build), which is mapped readable.
        unsafe { std::slice::from_raw_parts(base.add(rva as usize), n).to_vec() }
    };
    let exe = exe_path();
    let dir = PathBuf::from(&exe).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let ini_path = dir.join("kh1-recom.ini");
    let ini = match std::fs::read_to_string(&ini_path) {
        Ok(t) => Ok(t),
        Err(_) => {
            let _ = std::fs::write(&ini_path, crate::settings::DEFAULT_FILE);
            Err(format!("{} was missing and has been written with the defaults", ini_path.display()))
        }
    };
    let (report, _ok) = crate::startup_report(&exe, pe, &read_site, ini);
    let _ = std::fs::write(dir.join("kh1-recom.log"), report);
}

/// # Safety
/// Called by the Windows loader.
#[no_mangle]
pub unsafe extern "system" fn DllMain(_module: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        // The work runs on its own thread, after the loader lock is released.
        std::thread::spawn(startup);
    }
    1
}
