//! Inline hooks with trampolines (the mechanism of KH2-Combat-in-KH1's `kh1-hook`, ported), and a
//! small verified data-patch helper.
//!
//! Every hook checks the target's exact first bytes (`sites::HOOKS`, read from the Steam exe)
//! before touching anything, and a set of hooks goes in all or nothing.

use super::log;
use super::sys::*;
use crate::hookplan;
use crate::sites::Site;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Executable memory within rel32 reach below `near`.
unsafe fn alloc_near(near: usize) -> Option<usize> {
    for at in hookplan::near_candidates(near as u64) {
        let p = VirtualAlloc(at as *mut c_void, hookplan::BLOCK, MEM_COMMIT_RESERVE, PAGE_EXECUTE_READWRITE);
        if !p.is_null() {
            return Some(p as usize);
        }
    }
    None
}

/// Places `code` (at most [`hookplan::BLOCK`] bytes) in executable memory within rel32 reach of
/// `near`; its address. The block is never freed (Re:CoM may call into it at any time).
pub unsafe fn place_near(near: usize, code: &[u8]) -> Option<usize> {
    if code.is_empty() || code.len() > hookplan::BLOCK {
        return None;
    }
    let block = alloc_near(near)?;
    std::ptr::copy_nonoverlapping(code.as_ptr(), block as *mut u8, code.len());
    FlushInstructionCache(GetCurrentProcess(), block as *const c_void, code.len());
    Some(block)
}

/// Writes `new` over `at` if the bytes there are `expect` (code or data; same length).
pub unsafe fn patch(at: usize, expect: &[u8], new: &[u8], what: &str) -> bool {
    if expect.len() != new.len() || expect.is_empty() {
        log(&format!("patch {what}: bad lengths"));
        return false;
    }
    let have = std::slice::from_raw_parts(at as *const u8, expect.len());
    if have != expect {
        log(&format!("patch {what} at {at:#x}: unexpected bytes {have:02x?}, not patched"));
        return false;
    }
    let mut old = 0u32;
    if VirtualProtect(at as *mut c_void, new.len(), PAGE_EXECUTE_READWRITE, &mut old) == 0 {
        log(&format!("patch {what}: VirtualProtect failed"));
        return false;
    }
    std::ptr::copy_nonoverlapping(new.as_ptr(), at as *mut u8, new.len());
    let mut tmp = 0u32;
    VirtualProtect(at as *mut c_void, new.len(), old, &mut tmp);
    FlushInstructionCache(GetCurrentProcess(), at as *const c_void, new.len());
    true
}

/// One hook to install: the site (target RVA and the bytes to move), our function, and where
/// the trampoline (the original) is stored.
pub struct Hook {
    pub site: &'static Site,
    pub ours: usize,
    pub original: &'static AtomicUsize,
}

struct Installed {
    target: usize,
    block: usize,
    moved: &'static [u8],
}

/// Installs every hook or none. `base` is the exe's image base.
pub unsafe fn install_all(base: usize, hooks: &[Hook]) -> bool {
    // 1. Check every target first.
    for h in hooks {
        let target = base + h.site.rva as usize;
        let have = std::slice::from_raw_parts(target as *const u8, h.site.expect.len());
        if have != h.site.expect || h.site.expect.len() < 5 {
            log(&format!("hook {} at {target:#x}: unexpected bytes {have:02x?}; no hook installed", h.site.name));
            return false;
        }
    }
    // 2. Install, undoing on any failure.
    let mut done: Vec<Installed> = Vec::new();
    for h in hooks {
        let target = base + h.site.rva as usize;
        match install_one(target, h) {
            Some(i) => done.push(i),
            None => {
                for i in done.iter().rev() {
                    uninstall(i);
                }
                for h in hooks {
                    h.original.store(0, Ordering::Release);
                }
                log("hooks: one failed, all removed");
                return false;
            }
        }
    }
    true
}

unsafe fn install_one(target: usize, h: &Hook) -> Option<Installed> {
    let moved = h.site.expect;
    let Some(block) = alloc_near(target) else {
        log(&format!("hook {}: no memory near {target:#x}", h.site.name));
        return None;
    };
    let tramp = hookplan::trampoline(moved, target as u64);
    std::ptr::copy_nonoverlapping(tramp.as_ptr(), block as *mut u8, tramp.len());
    let stub = block + hookplan::STUB_AT;
    let go = hookplan::abs_jmp(h.ours as u64);
    std::ptr::copy_nonoverlapping(go.as_ptr(), stub as *mut u8, go.len());
    FlushInstructionCache(GetCurrentProcess(), block as *const c_void, hookplan::BLOCK);
    // The original must be callable before the jump goes in.
    h.original.store(block, Ordering::Release);
    let Some(entry) = hookplan::entry_patch(target as u64, moved.len(), stub as u64) else {
        VirtualFree(block as *mut c_void, 0, MEM_RELEASE);
        return None;
    };
    if !patch(target, moved, &entry, h.site.name) {
        VirtualFree(block as *mut c_void, 0, MEM_RELEASE);
        return None;
    }
    log(&format!("hooked {} at {target:#x} (trampoline {block:#x})", h.site.name));
    Some(Installed { target, block, moved })
}

unsafe fn uninstall(i: &Installed) {
    let now = std::slice::from_raw_parts(i.target as *const u8, i.moved.len()).to_vec();
    if patch(i.target, &now, i.moved, "restore") {
        log(&format!("restored {:#x}", i.target));
    }
    // The block is left allocated: a thread could still be inside the trampoline.
    let _ = i.block;
}
