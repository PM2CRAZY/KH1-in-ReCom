//! The few Windows API functions we need.

use std::ffi::c_void;

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    pub fn GetModuleFileNameW(module: *mut c_void, buf: *mut u16, size: u32) -> u32;
    pub fn VirtualAlloc(addr: *mut c_void, size: usize, kind: u32, prot: u32) -> *mut c_void;
    pub fn VirtualFree(addr: *mut c_void, size: usize, kind: u32) -> i32;
    pub fn VirtualProtect(addr: *mut c_void, size: usize, prot: u32, old: *mut u32) -> i32;
    pub fn FlushInstructionCache(process: *mut c_void, addr: *const c_void, size: usize) -> i32;
    pub fn GetCurrentProcess() -> *mut c_void;
}

pub const MEM_COMMIT_RESERVE: u32 = 0x3000;
pub const MEM_RELEASE: u32 = 0x8000;
pub const PAGE_EXECUTE_READWRITE: u32 = 0x40;

#[link(name = "user32")]
extern "system" {
    pub fn GetAsyncKeyState(vk: i32) -> i16;
    pub fn GetForegroundWindow() -> *mut c_void;
    pub fn GetWindowThreadProcessId(window: *mut c_void, pid: *mut u32) -> u32;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetCurrentProcessId() -> u32;
}
