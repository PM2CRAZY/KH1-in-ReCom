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
    Site { name: "initAttack (battle attack init)", rva: 0x001F_0D90, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x48] },
    Site { name: "field swing init", rva: 0x001F_0F70, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8B] },
    Site { name: "player motion request", rva: 0x001F_7180, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x48] },
    Site { name: "attack row pick", rva: 0x001F_7520, expect: &[0x4C, 0x8B, 0xDC, 0x49, 0x89, 0x5B, 0x20, 0x55, 0x57, 0x41, 0x57, 0x48, 0x81, 0xEC, 0xD0, 0x00] },
    Site { name: "player Battle state exe", rva: 0x001F_9840, expect: &[0x48, 0x89, 0x7C, 0x24, 0x20, 0x55, 0x48, 0x8D, 0x6C, 0x24, 0xA0, 0x48, 0x81, 0xEC, 0x60, 0x01] },
    Site { name: "player Field state exe", rva: 0x001F_A0F0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40, 0x33, 0xD2, 0x48, 0x8B, 0xD9, 0x8D, 0x4A, 0x01, 0xE8, 0x6D] },
    Site { name: "use-card button", rva: 0x001E_8FF0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x83, 0xB9, 0xB0, 0x07, 0x00, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
    Site { name: "motion pack (.ANN) loader", rva: 0x0029_F3A0, expect: &[0x40, 0x53, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC] },
];

/// Functions this version hooks. `expect` is exactly the bytes moved to the trampoline: whole,
/// position-independent instructions (read from the Steam exe, checked against the decomp).
pub const HOOKS: &[Site] = &[
    // mov [rsp+8],rbx; push rdi; sub rsp,0x20
    Site { name: "Config screen input (CCamp)", rva: 0x000D_3EB0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20] },
    // mov rax,rsp; push rbp; push r12; push r13; push r14
    Site { name: "Config screen draw (CCamp)", rva: 0x000D_F1B0, expect: &[0x48, 0x8B, 0xC4, 0x55, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56] },
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
    Site { name: "player stateInitial (resolves action names)", rva: 0x001F_A6E0, expect: &[0x40, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x50] },
    Site { name: "motion name resolve (0x1402A42B0)", rva: 0x002A_42B0, expect: &[0x40, 0x56, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x4C, 0x8B, 0xF2] },
];

/// Data this version reads or calls through, with its address (Steam). Checked (exe).
pub mod data {
    /// Sequence ids for label items (`u16[]`).
    pub const LABEL_SEQ: u64 = 0x1_407C_1230;
    /// Per value item, a pointer to its sequence ids (9 pointers).
    pub const VALUE_SEQ: u64 = 0x1_407C_1240;
    /// Pointer to the global whose `+0x274` picks the label font (0xC for 3 and 6).
    pub const FONT_GLOBAL: u64 = 0x1_40AF_73B0;
}

/// Every site the startup check reads: checked-only, hooked, called.
pub fn all_sites() -> impl Iterator<Item = &'static Site> {
    SITES.iter().chain(HOOKS).chain(CALLEES)
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
        // The two hooked prologues hold no rip-relative operand or branch, so they can run from
        // the trampoline. Each is 10 bytes (whole instructions, decoded in the module docs).
        for h in HOOKS {
            assert_eq!(h.expect.len(), 10, "{}", h.name);
            assert!(!h.expect.windows(2).any(|w| w == [0x8B, 0x05] || w == [0x8D, 0x05]), "{}", h.name);
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
