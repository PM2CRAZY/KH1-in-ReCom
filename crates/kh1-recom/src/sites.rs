//! The Steam Re:CoM build this mod targets, the functions it hooks or calls, and those later
//! stages will hook.
//!
//! Every address is a Steam RVA (image base 0x140000000). Each site's expected first bytes were
//! read from the Steam exe (`KINGDOM HEARTS Re_Chain of Memories.exe`, SHA-256 `acf42e96…6bbf`)
//! and match the function starts in the Steam decomp. See `docs/recom-player.md` and
//! `docs/recom-combat.md`.

/// PE header values of the Steam exe: link time stamp and size of image. Checked (exe).
pub const STEAM_TIMESTAMP: u32 = 0x669E_38DF;
pub const STEAM_SIZE_OF_IMAGE: u32 = 0x00D4_A000;

/// The exe's file name.
pub const EXE_NAME: &str = "KINGDOM HEARTS Re_Chain of Memories.exe";

pub struct Site {
    pub name: &'static str,
    pub rva: u32,
    /// The function's first bytes in the Steam exe (at least 5; a hook must check them all).
    pub expect: &'static [u8],
}

/// Functions later stages will hook. These are only checked.
pub const SITES: &[Site] = &[
    Site { name: "attack row pick", rva: 0x001F_7520, expect: &[0x4C, 0x8B, 0xDC, 0x49, 0x89, 0x5B, 0x20, 0x55, 0x57, 0x41, 0x57, 0x48, 0x81, 0xEC, 0xD0, 0x00] },
    Site { name: "use-card button", rva: 0x001E_8FF0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x83, 0xB9, 0xB0, 0x07, 0x00, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
    Site { name: "motion pack (.ANN) loader", rva: 0x0029_F3A0, expect: &[0x40, 0x53, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC] },
    Site { name: "player stateInitial (resolves action names)", rva: 0x001F_A6E0, expect: &[0x40, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x50] },
    Site { name: "motion name resolve (0x1402A42B0)", rva: 0x002A_42B0, expect: &[0x40, 0x56, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x4C, 0x8B, 0xF2] },
    // Not called: Re:CoM's mover, whose velocity field (`body+0x80`) the KH1 handlers write.
    Site { name: "CMovement update (0x1401D8190)", rva: 0x001D_8190, expect: &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x57, 0x48, 0x81, 0xEC, 0xE0, 0x00, 0x00, 0x00] },
];

/// Functions this version hooks. `expect` is exactly the bytes moved to the trampoline: whole,
/// position-independent instructions (read from the Steam exe, checked against the decomp).
pub const HOOKS: &[Site] = &[
    // mov [rsp+8],rbx; push rdi; sub rsp,0x20
    Site { name: "Config screen input (CCamp)", rva: 0x000D_3EB0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20] },
    // mov rax,rsp; push rbp; push r12; push r13; push r14
    Site { name: "Config screen draw (CCamp)", rva: 0x000D_F1B0, expect: &[0x48, 0x8B, 0xC4, 0x55, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56] },
];

/// The KH1 movement handlers (`movement`, `docs/recom-kh1-movement.md`). Same rule as [`HOOKS`].
pub const MOVE_HOOKS: &[Site] = &[
    // push rbx; sub rsp,0x50; mov eax,[rcx+0x424]; mov rbx,rcx
    Site { name: "move exe (actions 2/3/4)", rva: 0x001E_EDA0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x50, 0x8B, 0x81, 0x24, 0x04, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
    // movss xmm1,[rcx+0x384]
    Site { name: "stick class (0x1401E9630)", rva: 0x001E_9630, expect: &[0xF3, 0x0F, 0x10, 0x89, 0x84, 0x03, 0x00, 0x00] },
    // push rbx; sub rsp,0x50; movss xmm3,[rcx+0x464]; mov rbx,rcx
    Site { name: "move step (0x1401EF3C0)", rva: 0x001E_F3C0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x50, 0xF3, 0x0F, 0x10, 0x99, 0x64, 0x04, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
    // mov [rsp+0x10],rbx; mov [rsp+0x18],rsi
    Site { name: "jump init (0x1401F29F0)", rva: 0x001F_29F0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18] },
    // push rbx; sub rsp,0x60
    Site { name: "avoid init (0x1401F10C0)", rva: 0x001F_10C0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x60] },
    // mov r11,rsp; push rbx; sub rsp,0xD0
    Site { name: "jump exe (action 7)", rva: 0x001E_DA60, expect: &[0x4C, 0x8B, 0xDC, 0x53, 0x48, 0x81, 0xEC, 0xD0, 0x00, 0x00, 0x00] },
    // push rbx; sub rsp,0x30; mov rax,[rcx+0xF0]; mov rbx,rcx
    Site { name: "CPlayerSora avoid exe (action 6)", rva: 0x0021_7310, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x48, 0x8B, 0x81, 0xF0, 0x00, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
    // push rbx; sub rsp,0x80; and dword [rcx+0x428],-9
    Site { name: "CPlayerSora glide exe (action 8)", rva: 0x0021_7B50, expect: &[0x40, 0x53, 0x48, 0x81, 0xEC, 0x80, 0x00, 0x00, 0x00, 0x83, 0xA1, 0x28, 0x04, 0x00, 0x00, 0xF7] },
];

/// The trace's sampling points: the player's Field and Battle state exes. They only read while
/// F9 recording is on. Same rule as [`HOOKS`].
pub const TRACE_HOOKS: &[Site] = &[
    // push rbx; sub rsp,0x40; xor edx,edx; mov rbx,rcx
    Site { name: "player Field state exe", rva: 0x001F_A0F0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40, 0x33, 0xD2, 0x48, 0x8B, 0xD9] },
    // mov [rsp+0x20],rdi; push rbp; lea rbp,[rsp-0x60]
    Site { name: "player Battle state exe", rva: 0x001F_9840, expect: &[0x48, 0x89, 0x7C, 0x24, 0x20, 0x55, 0x48, 0x8D, 0x6C, 0x24, 0xA0] },
];

/// Re:CoM functions this version calls (from the hooks). Checked like the hook sites, so a
/// different build never gets called into.
pub const CALLEES: &[Site] = &[
    Site { name: "camp: pad direction (0x140111A70)", rva: 0x0011_1A70, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0xF2] },
    Site { name: "camp: pad confirm (0x140111B90)", rva: 0x0011_1B90, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0xFA, 0x48, 0x8B, 0xD9] },
    Site { name: "menu sound (0x1402AA360)", rva: 0x002A_A360, expect: &[0x48, 0x83, 0xEC, 0x48, 0x48, 0x8B, 0x05, 0x5D, 0x87, 0x5B, 0x00] },
    Site { name: "operator new (0x1404ACE54)", rva: 0x004A_CE54, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0xEB, 0x0F] },
    Site { name: "CButton ctor (0x140112A90)", rva: 0x0011_2A90, expect: &[0x48, 0x8D, 0x05, 0xD9, 0x12, 0x40, 0x00, 0x48, 0xC7, 0x41, 0x18, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "CButton setup (0x140112F10)", rva: 0x0011_2F10, expect: &[0x48, 0x8B, 0xC4, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x60] },
    Site { name: "layout position (0x140246100)", rva: 0x0024_6100, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x83, 0x3D, 0x21, 0x6D, 0x63, 0x00, 0x00] },
    Site { name: "text set (0x140259000)", rva: 0x0025_9000, expect: &[0x48, 0x85, 0xD2, 0x0F, 0x84, 0x91, 0x00, 0x00, 0x00, 0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x30] },
    Site { name: "sprite position (0x1402485B0)", rva: 0x0024_85B0, expect: &[0x8B, 0x02, 0x89, 0x81, 0xF0, 0x00, 0x00, 0x00, 0x8B, 0x42, 0x04, 0x89, 0x81, 0xF4, 0x00, 0x00, 0x00] },
    Site { name: "sequence by handle (0x140246210)", rva: 0x0024_6210, expect: &[0x48, 0x83, 0xEC, 0x28, 0x48, 0x83, 0x3D, 0x1C, 0x6C, 0x63, 0x00, 0x00, 0x8B, 0xD1] },
    // Not called: the C2DSprObj 9-slice code that reads the edge offsets we set on the Config
    // panel. Checked so a build where these fields mean something else is never written to.
    Site { name: "C2DSprObj 9-slice corners (0x14024AD40)", rva: 0x0024_AD40, expect: &[0x48, 0x83, 0xB9, 0x20, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x84, 0x0B, 0x06, 0x00, 0x00] },
    Site { name: "C2DSprObj resize (0x14024C270)", rva: 0x0024_C270, expect: &[0x40, 0x53, 0x48, 0x81, 0xEC, 0x90, 0x00, 0x00, 0x00, 0x0F, 0x29, 0xB4, 0x24, 0x80, 0x00, 0x00] },
];

/// Re:CoM functions the KH1 movement handlers call.
pub const MOVE_CALLEES: &[Site] = &[
    Site { name: "player motion request (0x1401F7180)", rva: 0x001F_7180, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x48] },
    Site { name: "player ground test (0x1401F3C00)", rva: 0x001F_3C00, expect: &[0x48, 0x83, 0xEC, 0x18, 0x83, 0xB9, 0x38, 0x05, 0x00, 0x00, 0x00, 0x74, 0x0B, 0x8B, 0x81, 0x3C] },
    Site { name: "collision segment test (0x140170330)", rva: 0x0017_0330, expect: &[0x48, 0x8B, 0xC4, 0x55, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8D, 0x68, 0xB8] },
    Site { name: "handle to object (0x1402AD350)", rva: 0x002A_D350, expect: &[0x48, 0x83, 0xEC, 0x28, 0x8B, 0xD1, 0x48, 0x8B, 0x0D, 0x33, 0x9D, 0x84, 0x00, 0x48, 0x85, 0xC9] },
    Site { name: "pad: jump held (0x1401FF4D0)", rva: 0x001F_F4D0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x19, 0x33, 0xC9, 0x83, 0xE3, 0x01] },
    Site { name: "pad: buttons held (0x14029C1D0)", rva: 0x0029_C1D0, expect: &[0x83, 0xFA, 0x02, 0x73, 0x1A, 0x8B, 0xC2, 0x48, 0x8D, 0x15] },
    Site { name: "field swing init (0x1401F0F70)", rva: 0x001F_0F70, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8B] },
    Site { name: "initAttack (0x1401F0D90)", rva: 0x001F_0D90, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x48] },
    Site { name: "reload check (0x1401E6C40)", rva: 0x001E_6C40, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x60] },
];

/// Data this version reads or calls through, with its address (Steam). Checked (exe).
pub mod data {
    /// Sequence ids for label items (`u16[]`).
    pub const LABEL_SEQ: u64 = 0x1_407C_1230;
    /// Per value item, a pointer to its sequence ids (9 pointers).
    pub const VALUE_SEQ: u64 = 0x1_407C_1240;
    /// Pointer to the global whose `+0x274` picks the label font (0xC for 3 and 6).
    pub const FONT_GLOBAL: u64 = 0x1_40AF_73B0;
    /// `D2::C2DSprObj` vtable (RTTI at `0x1406077D0`; set by its ctor `0x140249FC0`; slot
    /// `+0x38` is the resize `0x14024C270`).
    pub const SPR_OBJ_VTABLE: u64 = 0x1_4060_77D8;
    /// The main player (`CPlayer` ctor `0x1401E4C20`).
    pub const MAIN_PLAYER: u64 = 0x1_4087_B380;
    /// `CPlayerSora` vtable (RTTI). Riku's are `0x1405A8868` / `0x1405A50E8`.
    pub const SORA_VTABLE: u64 = 0x1_405A_9938;
    /// Pad devices, one pointer per port (`0x14029BF60` reads `[port]+0x74`).
    pub const PAD_DEVICES: u64 = 0x1_40AC_5D40;
    /// Sora's action table and the per-character table pointers (`docs/recom-motion-switch.md`).
    pub const SORA_TABLE: u64 = 0x1_407C_5E80;
    pub const CHAR_TABLES: u64 = 0x1_407C_6300;
    /// `CPlayerSora` vtable slot of the magic-card init (`+0x148`, `initMagic`).
    pub const VT_INIT_MAGIC: usize = 0x148;
}

/// Every site the startup check reads: checked-only, hooked, called.
pub fn all_sites() -> impl Iterator<Item = &'static Site> {
    SITES.iter().chain(HOOKS).chain(CALLEES).chain(MOVE_HOOKS).chain(TRACE_HOOKS).chain(MOVE_CALLEES)
}

/// What the PE headers of a loaded image say about its build.
#[derive(Debug, PartialEq, Eq)]
pub struct PeInfo {
    pub timestamp: u32,
    pub size_of_image: u32,
}

/// Reads the link time stamp and size of image from the start of a PE image (the mapped headers).
pub fn pe_info(headers: &[u8]) -> Option<PeInfo> {
    let u16_at = |o: usize| headers.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |o: usize| headers.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    if u16_at(0)? != 0x5A4D {
        return None; // "MZ"
    }
    let pe = u32_at(0x3C)? as usize;
    if u32_at(pe)? != 0x0000_4550 {
        return None; // "PE\0\0"
    }
    let timestamp = u32_at(pe + 8)?;
    // Optional header starts at pe + 24; SizeOfImage is at +56 in PE32+.
    if u16_at(pe + 24)? != 0x020B {
        return None; // not PE32+
    }
    let size_of_image = u32_at(pe + 24 + 56)?;
    Some(PeInfo { timestamp, size_of_image })
}

pub fn is_steam_build(info: &PeInfo) -> bool {
    info.timestamp == STEAM_TIMESTAMP && info.size_of_image == STEAM_SIZE_OF_IMAGE
}

/// The result of checking one site's bytes.
#[derive(Debug, PartialEq, Eq)]
pub enum SiteCheck {
    Ok,
    /// The bytes found there, when they differ.
    Differs(Vec<u8>),
}

pub fn check_site(site: &Site, found: &[u8]) -> SiteCheck {
    if found.len() >= site.expect.len() && found[..site.expect.len()] == *site.expect {
        SiteCheck::Ok
    } else {
        SiteCheck::Differs(found.iter().take(site.expect.len()).copied().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_headers(ts: u32, soi: u32) -> Vec<u8> {
        let mut h = vec![0u8; 0x200];
        h[0] = b'M';
        h[1] = b'Z';
        h[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        h[0x80..0x84].copy_from_slice(b"PE\0\0");
        h[0x88..0x8C].copy_from_slice(&ts.to_le_bytes());
        h[0x98..0x9A].copy_from_slice(&0x020Bu16.to_le_bytes());
        h[0x98 + 56..0x98 + 60].copy_from_slice(&soi.to_le_bytes());
        h
    }

    #[test]
    fn reads_pe_info() {
        let h = fake_headers(STEAM_TIMESTAMP, STEAM_SIZE_OF_IMAGE);
        let i = pe_info(&h).unwrap();
        assert!(is_steam_build(&i));
        let other = pe_info(&fake_headers(1, STEAM_SIZE_OF_IMAGE)).unwrap();
        assert!(!is_steam_build(&other));
        assert_eq!(pe_info(&[0u8; 0x40]), None);
    }

    #[test]
    fn sites_have_enough_bytes_and_are_unique() {
        for s in all_sites() {
            assert!(s.expect.len() >= 5, "{}", s.name);
        }
        let mut r: Vec<u32> = all_sites().map(|s| s.rva).collect();
        let n = r.len();
        r.sort();
        r.dedup();
        assert_eq!(r.len(), n);
    }

    #[test]
    fn hooks_move_whole_instructions_without_rip_operands() {
        // The hooked prologues hold no rip-relative operand or branch, so they can run from the
        // trampoline (whole instructions, decoded in the comments above each site). The
        // trampoline (moved bytes + a 14-byte jump) must fit before the stub.
        for h in HOOKS.iter().chain(MOVE_HOOKS).chain(TRACE_HOOKS) {
            assert!(h.expect.len() >= 5 && h.expect.len() + 14 <= crate::hookplan::STUB_AT, "{}", h.name);
            assert!(!h.expect.windows(2).any(|w| w == [0x8B, 0x05] || w == [0x8D, 0x05] || w == [0x8B, 0x0D] || w == [0x10, 0x05]), "{}", h.name);
            assert!(!h.expect.contains(&0xE8) && !h.expect.contains(&0xE9), "{}", h.name);
        }
    }

    #[test]
    fn check_site_compares_prefix() {
        let s = &SITES[0];
        assert_eq!(check_site(s, s.expect), SiteCheck::Ok);
        let mut bad = s.expect.to_vec();
        bad[2] ^= 0xFF;
        assert!(matches!(check_site(s, &bad), SiteCheck::Differs(_)));
        assert!(matches!(check_site(s, &s.expect[..3]), SiteCheck::Differs(_)));
    }
}
