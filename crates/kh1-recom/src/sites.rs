//! The Steam Re:CoM build this mod targets, and the functions later stages will hook.
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

/// Functions later stages will hook. Nothing is hooked in 0.1.0: these are only checked.
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
        for s in SITES {
            assert!(s.expect.len() >= 5, "{}", s.name);
        }
        let mut r: Vec<u32> = SITES.iter().map(|s| s.rva).collect();
        r.sort();
        r.dedup();
        assert_eq!(r.len(), SITES.len());
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
