//! KH1 FM Steam addresses the logger reads, and the build check. All are Steam RVAs (image base
//! 0x140000000) from the KH1 Steam decomp; `docs/kh1-trace.md` tags each one.

/// The exe's file name.
pub const EXE_NAME: &str = "KINGDOM HEARTS FINAL MIX.exe";

/// PE link time stamp and size of image of the Steam exe (SHA-256 `d7907462…8bac`). Checked (exe).
pub const STEAM_TIMESTAMP: u32 = 0x669E_37CE;
pub const STEAM_SIZE_OF_IMAGE: u32 = 0x02F9_1000;

/// The hooked function: the per-actor task `0x140297FE0` (registered by `0x140297980` for every
/// actor; the task record's +0x18 is the actor). It runs once per frame per actor; Sora's think
/// `0x1402A3300` and physics `0x14029A3C0` run inside it, his motion update `0x14029F0A0` runs
/// later in the frame (task `0x140298730`). Checked (decomp).
pub const HOOK_RVA: usize = 0x0029_7FE0;
/// `mov rax, rsp` / `push rdi` / `sub rsp, 0x60`: three whole instructions, nothing
/// RIP-relative. Checked (exe, disassembled).
pub const HOOK_PROLOGUE: [u8; 8] = [0x48, 0x8B, 0xC4, 0x57, 0x48, 0x83, 0xEC, 0x60];

/// Party actor pointers; slot 0 is Sora (`0x140290D60` fills it by party member index).
pub const SORA_PTR: usize = 0x02D3_7280;
/// The frame step (float), set each frame by `0x14018D1F0`.
pub const FRAME_STEP: usize = 0x0233_FBDC;
/// The logical pad block (`0x14028B0B0` copies pad 0 into it): +4 held, +8 pressed, +0x4C tilt,
/// +0x60 move-stick unit vector (x, y down).
pub const LOGICAL_PAD: usize = 0x0298_CA90;
/// Pointer to the input device record; its +0x18 buffer holds the raw stick bytes, +4/+5 the
/// move stick (read by `0x14028B4F0`), +0x1F = 1 when it is live.
pub const DEVICE_PTR: usize = 0x021A_AF18;
/// Camera yaw, second yaw and mode, as `0x14028C7E0` reads them.
pub const CAM_YAW: usize = 0x0299_BB04;
pub const CAM_YAW2: usize = 0x0299_C194;
pub const CAM_MODE: usize = 0x0299_BB08;
/// The move vector the control step `0x1402A6220` keeps (only when Sora can act).
pub const MOVE_VEC: usize = 0x02D5_EC30;
/// The command word Sora's control step consumes and clears.
pub const COMMAND: usize = 0x02D5_3A40;
/// Lock-on: state word, then the target pair (actor handle, part index).
pub const LOCK_STATE: usize = 0x02DB_3510;
pub const LOCK_TARGET: usize = 0x02DB_3518;
/// Ability bits (built by `0x1402A6380`).
pub const ABILITIES: usize = 0x02D5_EC10;
/// Pointer to the save record; +0x44 is Sora's 48-byte ability list (bit 7 = unequipped).
pub const SAVE_PTR: usize = 0x0286_8BA0;
pub const ABILITY_LIST: usize = 0x44;
pub const COMBO_MASTER_ID: u8 = 0x41;
/// Handle table (`0x14038AF40`: pointer = table[(h & 0x7FFFFFFF) >> 25] | (h & 0x1FFFFFF)).
pub const HANDLE_TABLE: usize = 0x02EE_3980;

/// Actor fields.
pub mod actor {
    pub const FLAGS: usize = 0x00;
    pub const NO_FLINCH: usize = 0x0C;
    pub const POS: usize = 0x10;
    pub const MOVE_DIR: usize = 0x20;
    pub const SPEED: usize = 0x2C;
    pub const FACING: usize = 0x3C;
    pub const STATS: usize = 0x6C;
    pub const STATE: usize = 0x70;
    pub const PAUSE: usize = 0x88;
    pub const COMBO: usize = 0x14C;
    pub const TARGET_YAW: usize = 0x150;
    pub const REQUEST: usize = 0x164;
    pub const ENTRY: usize = 0x168;
    pub const MOTION_FRAME: usize = 0x16C;
    pub const BLEND: usize = 0x170;
    pub const BLEND_CLOCK: usize = 0x174;
    pub const MOTION_FLAGS: usize = 0x1CC;
    pub const CONTACT: usize = 0x2D0;
    pub const PREV_STATE: usize = 0x378;
    pub const SUPPORT: usize = 0x398;
    pub const AIR_T: usize = 0x3B0;
    pub const RISE_T: usize = 0x3B8;
    pub const AIR_VEL: usize = 0x3C0;
    pub const HEIGHT: usize = 0x3D4;
    pub const STAT_EXTRA: usize = 0x444;
    /// Combo record: row, counter, flags, this swing's hit bits.
    pub const CB_ROW: usize = 0x40;
    pub const CB_COUNTER: usize = 0x41;
    pub const CB_FLAGS: usize = 0x42;
    pub const CB_HITS: usize = 0x50;
    /// Stats block: ground / air combo length.
    pub const ST_COMBO_MAX: usize = 0xD4;
    pub const ST_AIR_COMBO_MAX: usize = 0xD5;
}

/// What the PE headers of a loaded image say about its build.
#[derive(Debug, PartialEq, Eq)]
pub struct PeInfo {
    pub timestamp: u32,
    pub size_of_image: u32,
}

/// Reads the link time stamp and size of image from a PE image's first bytes.
pub fn pe_info(headers: &[u8]) -> Option<PeInfo> {
    let u16_at = |o: usize| headers.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |o: usize| headers.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    if u16_at(0)? != 0x5A4D {
        return None;
    }
    let pe = u32_at(0x3C)? as usize;
    if u32_at(pe)? != 0x0000_4550 || u16_at(pe + 24)? != 0x020B {
        return None;
    }
    Some(PeInfo { timestamp: u32_at(pe + 8)?, size_of_image: u32_at(pe + 24 + 56)? })
}

pub fn is_steam_build(i: &PeInfo) -> bool {
    i.timestamp == STEAM_TIMESTAMP && i.size_of_image == STEAM_SIZE_OF_IMAGE
}

/// Whether Combo Master is equipped, from Sora's 48-byte ability list (`0x1402A7510`'s scan: the
/// first byte whose low 7 bits are 0x41; equipped when its bit 7 is clear).
pub fn combo_master(list: &[u8]) -> bool {
    list.iter().take(48).find(|b| *b & 0x7F == COMBO_MASTER_ID).is_some_and(|b| b & 0x80 == 0)
}

/// The kh1-core stick (x right, y forward, length = tilt) from the pad's unit vector (x, y down)
/// and tilt.
pub fn stick(unit_x: f32, unit_y: f32, tilt: f32) -> [f32; 2] {
    if tilt > 0.0 && unit_x.is_finite() && unit_y.is_finite() {
        [unit_x * tilt, -unit_y * tilt]
    } else {
        [0.0, 0.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Steam exe's headers and the hook's first bytes, when the exe is at hand
    /// (`KH1_STEAM_EXE`; skipped otherwise).
    #[test]
    fn steam_exe_matches() {
        let Some(p) = std::env::var_os("KH1_STEAM_EXE") else { return };
        let exe = std::fs::read(p).unwrap();
        let info = pe_info(&exe[..0x1000]).unwrap();
        assert!(is_steam_build(&info), "{info:?}");
        // .text is mapped at 0x1000 from file offset 0x400 in this exe.
        let off = HOOK_RVA - 0x1000 + 0x400;
        assert_eq!(exe[off..off + 8], HOOK_PROLOGUE);
    }

    #[test]
    fn combo_master_scan() {
        let mut l = [0xFFu8; 48];
        assert!(!combo_master(&l));
        l[5] = 0x41;
        assert!(combo_master(&l));
        l[5] = 0xC1;
        assert!(!combo_master(&l));
    }

    #[test]
    fn stick_axes() {
        assert_eq!(stick(0.0, -1.0, 1.0), [0.0, 1.0]);
        assert_eq!(stick(1.0, 0.0, 0.5), [0.5, -0.0]);
        assert_eq!(stick(f32::NAN, 0.0, 0.0), [0.0, 0.0]);
    }
}
