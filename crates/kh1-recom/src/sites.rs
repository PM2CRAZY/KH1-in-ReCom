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
    // 0.153.0: Riku's (`CPlayerRiku +0x1C8`) and Dark Riku's (`CPlayerDRiku +0x1C8`) own avoid exes (action 6): the same
    // first 16 bytes as Sora's. Checked (exe).
    Site { name: "CPlayerRiku avoid exe (action 6)", rva: 0x0021_2350, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x48, 0x8B, 0x81, 0xF0, 0x00, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
    Site { name: "CPlayerDRiku avoid exe (action 6)", rva: 0x001F_FC30, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x48, 0x8B, 0x81, 0xF0, 0x00, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
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
    // Re:CoM's Config draw, called (no longer hooked, 2026-11-03) for the KH1 entry's settings pages
    // (modes 0, 1, 3, 4, 5, 6). mov rax,rsp; push rbp; push r12; push r13; push r14
    Site { name: "Config screen draw (CCamp, 0x1400DF1B0)", rva: 0x000D_F1B0, expect: &[0x48, 0x8B, 0xC4, 0x55, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56] },
];

/// Re:CoM functions the KH1 movement handlers call.
pub const MOVE_CALLEES: &[Site] = &[
    Site { name: "player motion request (0x1401F7180)", rva: 0x001F_7180, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x48] },
    Site { name: "player ground test (0x1401F3C00)", rva: 0x001F_3C00, expect: &[0x48, 0x83, 0xEC, 0x18, 0x83, 0xB9, 0x38, 0x05, 0x00, 0x00, 0x00, 0x74, 0x0B, 0x8B, 0x81, 0x3C] },
    Site { name: "collision segment test (0x140170330)", rva: 0x0017_0330, expect: &[0x48, 0x8B, 0xC4, 0x55, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8D, 0x68, 0xB8] },
    Site { name: "handle to object (0x1402AD350)", rva: 0x002A_D350, expect: &[0x48, 0x83, 0xEC, 0x28, 0x8B, 0xD1, 0x48, 0x8B, 0x0D, 0x33, 0x9D, 0x84, 0x00, 0x48, 0x85, 0xC9] },
    Site { name: "pad: jump held (0x1401FF4D0)", rva: 0x001F_F4D0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x19, 0x33, 0xC9, 0x83, 0xE3, 0x01] },
    Site { name: "pad: buttons held (0x14029C1D0)", rva: 0x0029_C1D0, expect: &[0x83, 0xFA, 0x02, 0x73, 0x1A, 0x8B, 0xC2, 0x48, 0x8D, 0x15] },
    // push rdi; sub rsp,0x40; mov qword [rsp+0x20],-2
    Site { name: "gimmick check (0x1401E80C0)", rva: 0x001E_80C0, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF] },
    Site { name: "reload check (0x1401E6C40)", rva: 0x001E_6C40, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x60] },
    // mov [rsp+0x10],rbx; mov [rsp+0x18],rsi; push rdi; sub rsp,0x50
    Site { name: "ledge climb check (0x1401E8C00)", rva: 0x001E_8C00, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x50] },
    // mov [rsp+0x10],rbx; mov [rsp+0x18],rsi; push rdi; sub rsp,0xc0
    Site { name: "ledge climb init (0x1401F2C20)", rva: 0x001F_2C20, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57, 0x48, 0x81, 0xEC, 0xC0, 0x00] },
    // push rbp; push rbx; lea rbp,[rsp-0x28]; sub rsp,0x128
    Site { name: "ledge hang check (0x1401E8420)", rva: 0x001E_8420, expect: &[0x40, 0x55, 0x53, 0x48, 0x8D, 0x6C, 0x24, 0xD8, 0x48, 0x81, 0xEC, 0x28, 0x01, 0x00, 0x00] },
    // mov [rsp+0x10],rbx; mov [rsp+0x18],rsi; push rdi; sub rsp,0xc0
    // push rbx; sub rsp,0x20; mov rbx,rcx; mov rcx,[rcx+0x3D0] (the weapon)
    Site { name: "player draw weapon (0x1401F5D30)", rva: 0x001F_5D30, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0x48, 0x8B, 0x89, 0xD0, 0x03, 0x00, 0x00] },
    Site { name: "ledge hang init (0x1401F26A0)", rva: 0x001F_26A0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57, 0x48, 0x81, 0xEC, 0xC0, 0x00] },
];

/// KH1's Attack (M5, `crate::attack`, `docs/recom-kh1-attack.md`). Same rule as [`HOOKS`]: the bytes
/// moved are whole, position-independent instructions (decoded in the comments).
pub const ATTACK_HOOKS: &[Site] = &[
    // mov [rsp+0x10],rbx; push rdi; sub rsp,0x20
    Site { name: "request bits (0x1401F8150)", rva: 0x001F_8150, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20] },
    // push rbx; sub rsp,0x20
    Site { name: "use-card button (0x1401E8FF0)", rva: 0x001E_8FF0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
    // mov [rsp+8],rbx; mov [rsp+0x10],rbp
    Site { name: "initAttack (0x1401F0D90)", rva: 0x001F_0D90, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10] },
    // mov [rsp+8],rbx; mov [rsp+0x10],rsi
    Site { name: "field swing init (0x1401F0F70)", rva: 0x001F_0F70, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10] },
    // mov [rsp+8],rbx; mov [rsp+0x10],rsi
    Site { name: "field swing exe (0x1401EAEB0)", rva: 0x001E_AEB0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10] },
    // mov [rsp+0x18],rbx; mov [rsp+0x20],rbp
    Site { name: "attack pair dispatch (0x14016EAD0)", rva: 0x0016_EAD0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x48, 0x89, 0x6C, 0x24, 0x20] },
    // mov [rsp+0x20],rbx; push rbp; push rsi; push r14
    Site { name: "enemy hit handler (0x14017F7E0)", rva: 0x0017_F7E0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x20, 0x55, 0x56, 0x41, 0x56] },
    // push rbp; push rdi; lea rbp,[rsp-0x4f]
    Site { name: "player damage check (0x1401E6DE0)", rva: 0x001E_6DE0, expect: &[0x40, 0x55, 0x57, 0x48, 0x8D, 0x6C, 0x24, 0xB1] },
    // movaps xmm0,[rcx+0xa0]
    Site { name: "root motion step (0x14024DCA0)", rva: 0x0024_DCA0, expect: &[0x0F, 0x28, 0x81, 0xA0, 0x00, 0x00, 0x00] },
    // mov edx,-1 (then `jmp 0x1401A2D70`, checked in `ATTACK_CALLEES`): the enemies' attack-state
    // "hurt bodies off" (vtable +0x438 default). Only these 5 bytes move.
    Site { name: "enemy attack hurt off (0x14017DA80)", rva: 0x0017_DA80, expect: &[0xBA, 0xFF, 0xFF, 0xFF, 0xFF] },
];

/// KH1 enemy stats (`win::stats`, decisions 12 / 21). Same rule as [`HOOKS`].
pub const STATS_HOOKS: &[Site] = &[
    // mov [rsp+0x20],r9d; mov [rsp+0x10],edx
    Site { name: "enemy stats init (0x14017FF50)", rva: 0x0017_FF50, expect: &[0x44, 0x89, 0x4C, 0x24, 0x20, 0x89, 0x54, 0x24, 0x10] },
    // sub rsp,0x28; mov rax,[rcx+0x310]
    Site { name: "enemy max HP (0x14017F5A0)", rva: 0x0017_F5A0, expect: &[0x48, 0x83, 0xEC, 0x28, 0x48, 0x8B, 0x81, 0x10, 0x03, 0x00, 0x00] },
];

/// Re:CoM functions the stats code calls.
pub const STATS_CALLEES: &[Site] = &[
    // push rbx; sub rsp,0x20; mov rax,[rip+…] (the save block)
    Site { name: "current world (0x1402339C0)", rva: 0x0023_39C0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0x05, 0x43, 0x77, 0x64, 0x00, 0x33, 0xDB] },
    // sub rsp,0x28; mov edx,0x60; xor ecx,ecx (byte variable 0x60: Riku's story)
    Site { name: "Riku's story (0x1401AAAE0)", rva: 0x001A_AAE0, expect: &[0x48, 0x83, 0xEC, 0x28, 0xBA, 0x60, 0x00, 0x00, 0x00, 0x33, 0xC9] },
];

/// The "Damage" setting at Re:CoM (`crate::damage`, `win::damage`; `docs/recom-kh1-attack.md`
/// § Re:CoM damage setting): the Re:CoM function it calls, and the code whose arithmetic it repeats
/// (checked only; nothing is patched). Checked against the Steam exe at start like every list.
pub const DAMAGE_CHECKED: &[Site] = &[
    // push rbx; push rbp; push rsi; push rdi; sub rsp,0x58; mov rax,[rip+…] (the stack cookie): the
    // enemy-card AP rewrite (player, &AP +0x660, &kind +0x668, attribute, status bits) that
    // initMagic, initStockSkill and exeStockSkill call, and the row pick inlines. Called.
    Site { name: "enemy-card AP rewrite (0x1401E66A0)", rva: 0x001E_66A0, expect: &[0x40, 0x53, 0x55, 0x56, 0x57, 0x48, 0x83, 0xEC, 0x58, 0x48, 0x8B, 0x05, 0x18, 0xC4, 0x67, 0x00] },
    // The row pick's card branch: movzx eax,[row+0xA]; lea rcx,[rax+card*8]; mov rax,[0x14087A808];
    // movzx r9d,word [rax+rcx*2]; add r9d,[rbx+0x664]; test [rbx+0x420],0x4000000.
    Site {
        name: "row pick card power (0x1401F7ACC)",
        rva: 0x001F_7ACC,
        expect: &[
            0x41, 0x0F, 0xB6, 0x40, 0x0A, 0x4A, 0x8D, 0x0C, 0xD0, 0x48, 0x8B, 0x05, 0x2C, 0x2D, 0x68, 0x00, 0x44, 0x0F, 0xB7, 0x0C, 0x48, 0x44, 0x03, 0x8B, 0x64, 0x06, 0x00, 0x00, 0xF7, 0x83, 0x20, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04,
        ],
    },
    // initAttack: +0x664 = (+0x420 & 0x40000000) ? +0x664 + 20 : 0 (mov eax,[rbx+0x664]; add eax,0x14;
    // jmp; mov eax,r14d (0); mov [rbx+0x664],eax).
    Site { name: "initAttack bonus (0x1401F0EEA)", rva: 0x001F_0EEA, expect: &[0x8B, 0x83, 0x64, 0x06, 0x00, 0x00, 0x83, 0xC0, 0x14, 0xEB, 0x03, 0x41, 0x8B, 0xC6, 0x89, 0x83, 0x64, 0x06, 0x00, 0x00] },
    // The CCardBattle ctor's stores of BattleCardList and StockSkillList (mov [rip+…],rax →
    // 0x14087A808 / 0x14087A810).
    Site { name: "BattleCardList set (0x140160849)", rva: 0x0016_0849, expect: &[0x48, 0x89, 0x05, 0xB8, 0x9F, 0x71, 0x00] },
    Site { name: "StockSkillList set (0x140160879)", rva: 0x0016_0879, expect: &[0x48, 0x89, 0x05, 0x90, 0x9F, 0x71, 0x00] },
    // exeStockSkill's Sonic Blade dash AP: movzx eax,[row 0xF +0xA]; lea rcx,[rax+id*8]; mov rax,
    // [0x14087A810]; …; movzx edx,word [rax+rcx*2]; mov eax,0x51EB851F; imul edx,[r14+0x43C].
    Site {
        name: "Sonic Blade dash AP (0x140219B27)",
        rva: 0x0021_9B27,
        expect: &[
            0x0F, 0xB6, 0x05, 0x3C, 0xD2, 0x5A, 0x00, 0x4A, 0x8D, 0x0C, 0xC0, 0x48, 0x8B, 0x05, 0xD7, 0x0C, 0x66, 0x00, 0x4D, 0x03, 0xC0, 0x0F, 0xB7, 0x14, 0x48, 0xB8, 0x1F, 0x85, 0xEB, 0x51, 0x41, 0x0F, 0xAF, 0x96, 0x3C, 0x04, 0x00, 0x00,
        ],
    },
    // Damage = Re:CoM's Gravity and Stop (owner, 2026-11-01, follow-ups) go through Re:CoM's own
    // kinds; the gates and effects they rely on, checked only.
    // (Kind 8's gate and duration, `0x14017DCB5`, is `MAGIC_CHECKED`'s "kind specials: Stop gate".)
    // Kind 0xD (Gravity): mov rax,[rbx+0x308]; cmp dword [rax+0x40],0; je …; cmp dword
    // [rbx+0x2B0],0; movd xmm0,[rbx+0x274]; cvtdq2ps; mulss xmm0,xmm6 (the gate, then current HP ×
    // AP, ÷ 100.0 next).
    Site {
        name: "kind 0xD Gravity gate and % of HP (0x14017DD4C)",
        rva: 0x0017_DD4C,
        expect: &[
            0x48, 0x8B, 0x83, 0x08, 0x03, 0x00, 0x00, 0x83, 0x78, 0x40, 0x00, 0x0F, 0x84, 0xAF, 0x01, 0x00, 0x00, 0x83, 0xBB, 0xB0, 0x02, 0x00, 0x00, 0x00, 0x66, 0x0F, 0x6E, 0x83, 0x74, 0x02, 0x00, 0x00, 0x0F, 0x5B, 0xC0, 0xF3, 0x0F, 0x59, 0xC6,
        ],
    },
    // The reaction picker's case 0xD: and r8,~0x10; lea rax,[rbx+0xAC0]; mov [rbx+0x260],r8;
    // mov [rbx+0x68],rax (the flatten state, `win::magic::flattening`).
    Site { name: "reaction 0xD flatten state (0x14017D626)", rva: 0x0017_D626, expect: &[0x49, 0x83, 0xE0, 0xEF, 0x48, 0x8D, 0x83, 0xC0, 0x0A, 0x00, 0x00, 0x4C, 0x89, 0x83, 0x60, 0x02, 0x00, 0x00, 0x48, 0x89, 0x43, 0x68] },
    // The flatten exe's hold: mov dword [rbx+0x160],9000 (3 seconds).
    Site { name: "flatten hold 9000 ticks (0x140183CFB)", rva: 0x0018_3CFB, expect: &[0xC7, 0x83, 0x60, 0x01, 0x00, 0x00, 0x28, 0x23, 0x00, 0x00] },
];

/// Re:CoM functions the Attack calls.
pub const ATTACK_CALLEES: &[Site] = &[
    // The rest of `0x14017DA80` after the 5 bytes our hook moves: `jmp 0x1401A2D70` (all bodies
    // off), left in place and reached through the trampoline.
    Site { name: "enemy attack hurt off, its jump (0x14017DA85)", rva: 0x0017_DA85, expect: &[0xE9, 0xE6, 0x52, 0x02, 0x00] },
    Site { name: "end attack (0x1401F6660)", rva: 0x001F_6660, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x6C, 0x24, 0x18, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83] },
    Site { name: "weapon attack end (0x140224FB0)", rva: 0x0022_4FB0, expect: &[0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x33, 0xF6, 0x48, 0x8B, 0xF9, 0x89] },
    Site { name: "actor bone (0x1402ABB20)", rva: 0x002A_BB20, expect: &[0x48, 0x8B, 0x49, 0x08, 0x48, 0x85, 0xC9, 0x74, 0x26, 0x8B, 0x41, 0x10, 0x85, 0xC0, 0x74, 0x1A] },
    Site { name: "sphere vs sphere (0x1401777E0)", rva: 0x0017_77E0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x20, 0x55, 0x56, 0x57, 0x48, 0x83, 0xEC, 0x40] },
    // mov r10d,r8d; mov r8d,[rcx+0x150]; mov r11,rdx; mov edx,r10d (a thunk to 0x1402588B0)
    Site { name: "effect at actor (0x1401A6B20)", rva: 0x001A_6B20, expect: &[0x45, 0x8B, 0xD0, 0x44, 0x8B, 0x81, 0x50, 0x01, 0x00, 0x00, 0x4C, 0x8B, 0xDA, 0x41, 0x8B, 0xD2] },
    Site { name: "sphere vs column (0x140176FA0)", rva: 0x0017_6FA0, expect: &[0x40, 0x53, 0x55, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x50] },
    // Guard reactions (decision 27, `win::guard`).
    // push rbx; sub rsp,0x20; mov r8,[rcx+0x260]; mov rbx,rcx: the enemy reaction picker.
    Site { name: "enemy reaction picker (0x14017D260)", rva: 0x0017_D260, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x4C, 0x8B, 0x81, 0x60, 0x02, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
    // mov [rsp+8],rbx; mov [rsp+0x10],rsi; push rdi; sub rsp,0x20; mov rdi,rcx: the enemy's
    // "card done" (CBTL resetNowPlaying when it is the side-2 card now playing).
    Site { name: "enemy card end (0x14017E1D0)", rva: 0x0017_E1D0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xF9] },
    // The whole function: mov rax,[rcx+0x18]; test rax,rax; je ret; or edx,1; mov [rax+0x64],edx;
    // ret. CCollAttack's list setter (vtable +0x20).
    Site { name: "attack list setter (0x14014E5A0)", rva: 0x0014_E5A0, expect: &[0x48, 0x8B, 0x41, 0x18, 0x48, 0x85, 0xC0, 0x74, 0x06, 0x83, 0xCA, 0x01, 0x89, 0x50, 0x64, 0xC3] },
];

/// Exe bytes the Attack patches (not functions; checked the same way).
pub const ATTACK_DATA: &[Site] = &[
    // The Battle exe's jump table (`0x1401FA050`, ids 1..=0x24), entry for action 0xA: the default
    // `0x1401F9CC9` (as an RVA), then the entry for 0xB (`0x1401F99E9`, not changed).
    Site { name: "Battle exe jump table, action 0xA", rva: 0x001F_A074, expect: &[0xC9, 0x9C, 0x1F, 0x00, 0xE9, 0x99, 0x1F, 0x00] },
    // int3 padding after a `ret` (`0x1401FA6C0`), before `0x1401FA6D0`: the 13-byte stub.
    Site { name: "code cave 0x1401FA6C1", rva: 0x001F_A6C1, expect: &[0xCC; 13] },
    // Re:CoM's attack rows 1-7 (`0x1407C6C80`), the KH1 rows' templates.
    Site { name: "attack rows 1 (0x1407C6C80)", rva: 0x007C_6C80, expect: &crate::attack::RECOM_ROWS_1_7[0] },
];

/// The camp menu's 8th entry, "Keyblades" (`win::keyblades`, decision 25,
/// `docs/recom-keyblade-equip.md`). Installed all or nothing, after the Config hooks (the page runs
/// through the Config input hook). Same rule as [`HOOKS`].
pub const KEYBLADE_HOOKS: &[Site] = &[
    // mov rax,rsp; push rbp; push r12; push r13; push r14; push r15
    Site { name: "camp update (0x1400D9C80)", rva: 0x000D_9C80, expect: &[0x48, 0x8B, 0xC4, 0x55, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57] },
    // mov rax,rsp; push rbp; push rsi; push rdi; push r12; push r13; push r14
    Site { name: "camp entry build (0x1400E3DE0)", rva: 0x000E_3DE0, expect: &[0x48, 0x8B, 0xC4, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56] },
    // mov rax,rsp; mov [rax+0x18],rbx; mov [rax+0x20],rsi; push rbp; push rdi; push r14
    Site { name: "camp cursor set (0x1400DEA10)", rva: 0x000D_EA10, expect: &[0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x18, 0x48, 0x89, 0x70, 0x20, 0x55, 0x57, 0x41, 0x56] },
    // mov [rsp+8],rbx; mov [rsp+0x10],rbp; mov [rsp+0x18],rsi
    Site { name: "camp teardown (0x1400D7C20)", rva: 0x000D_7C20, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18] },
];

/// Re:CoM functions the Keyblades entry and page call (the camp's own helpers and Re:CoM's menu
/// classes). Checked like the hook sites.
pub const KEYBLADE_CALLEES: &[Site] = &[
    Site { name: "sequence ended (0x1402480C0)", rva: 0x0024_80C0, expect: &[0x0F, 0xB6, 0x81, 0x6D, 0x01, 0x00, 0x00, 0x3B, 0xD0, 0x72, 0x03] },
    // jmp 0x14029C220 (held with repeat) / jmp 0x14029C240 (fresh press)
    Site { name: "pad repeat (0x14029B2E0)", rva: 0x0029_B2E0, expect: &[0xE9, 0x3B, 0x0F, 0x00, 0x00] },
    Site { name: "pad fresh (0x14029B2F0)", rva: 0x0029_B2F0, expect: &[0xE9, 0x4B, 0x0F, 0x00, 0x00] },
    Site { name: "pad cancel (0x14029B410)", rva: 0x0029_B410, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0xD9, 0xE8, 0x83, 0xB7, 0x0A, 0x00] },
    Site { name: "pad confirm (0x14029B490)", rva: 0x0029_B490, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0xD9, 0xE8, 0x13, 0xB7, 0x0A, 0x00] },
    Site { name: "button guide (0x1402B57A0)", rva: 0x002B_57A0, expect: &[0x0F, 0xB7, 0x05, 0xED, 0x1B, 0x84, 0x00, 0x85, 0xC9] },
    Site { name: "camp confirm flash (0x1400D3A70)", rva: 0x000D_3A70, expect: &[0x48, 0x83, 0xEC, 0x48, 0x4C, 0x8B, 0xC1, 0x48, 0x8B, 0x89, 0x28, 0x01, 0x00, 0x00] },
    Site { name: "sprite priority (0x140247AA0)", rva: 0x0024_7AA0, expect: &[0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x20] },
    Site { name: "sprite visible (0x140247DC0)", rva: 0x0024_7DC0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x48, 0x89, 0x6C, 0x24, 0x20, 0x57, 0x48, 0x83, 0xEC, 0x20] },
    Site { name: "camp highlight bar (0x1400E07D0)", rva: 0x000E_07D0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40, 0x48, 0x8B, 0x05, 0xEB, 0x22, 0x78, 0x00] },
    Site { name: "camp glide target (0x1400E3D30)", rva: 0x000E_3D30, expect: &[0x48, 0x83, 0xEC, 0x38, 0x85, 0xD2, 0x0F, 0x88, 0x8E, 0x00, 0x00, 0x00] },
    Site { name: "message text (0x140240160)", rva: 0x0024_0160, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0x05, 0x13, 0xCC, 0x63, 0x00, 0x44, 0x8B, 0xDA] },
    Site { name: "camp header (0x1400E4220)", rva: 0x000E_4220, expect: &[0x48, 0x8B, 0xC4, 0x55, 0x56, 0x57, 0x41, 0x56, 0x41, 0x57, 0x48, 0x81, 0xEC, 0xC0, 0x00, 0x00] },
    Site { name: "C2DFnt ctor (0x140243F10)", rva: 0x0024_3F10, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x8B, 0x84, 0x24, 0x80, 0x00, 0x00, 0x00] },
    Site { name: "C2DSeqCtrl ctor (0x140247260)", rva: 0x0024_7260, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8D, 0x05, 0xD3, 0x00, 0x3C, 0x00] },
    Site { name: "sprite layout position (0x140248660)", rva: 0x0024_8660, expect: &[0xF3, 0x0F, 0x10, 0x02, 0x4C, 0x8B, 0xC2, 0xF3, 0x0F, 0x5C, 0x05, 0x89, 0x47, 0x2C, 0x00] },
    Site { name: "camp panel (0x1400E6700)", rva: 0x000E_6700, expect: &[0x48, 0x8B, 0xC4, 0x55, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8D, 0x68, 0xA1] },
    Site { name: "sprite slot visible (0x140247E50)", rva: 0x0024_7E50, expect: &[0x0F, 0xB6, 0x81, 0x6D, 0x01, 0x00, 0x00, 0x3B, 0xD0, 0x7D, 0x59] },
    Site { name: "camp side panel (0x1400E46D0)", rva: 0x000E_46D0, expect: &[0x85, 0xD2, 0x0F, 0x88, 0x01, 0x02, 0x00, 0x00, 0x4C, 0x8B, 0xDC, 0x57, 0x48, 0x81, 0xEC, 0x80] },
    Site { name: "camp glide step (0x1400DE100)", rva: 0x000D_E100, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x83, 0xB9, 0xCC, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "camp: pad decide (0x140111C30)", rva: 0x0011_1C30, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0xFA, 0x48, 0x8B, 0xD9, 0xBA, 0x1C, 0x00, 0x00, 0x00] },
    Site { name: "CButtonCursor ctor (0x1400D25F0)", rva: 0x000D_25F0, expect: &[0x48, 0x8B, 0xC4, 0x48, 0x89, 0x48, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x60] },
    Site { name: "CButtonCursor glide (0x1400DE010)", rva: 0x000D_E010, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x83, 0xB9, 0xB0, 0x01, 0x00, 0x00, 0x00] },
    Site { name: "CButtonCursor hide (0x1400E5CF0)", rva: 0x000E_5CF0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x33, 0xD2, 0x48, 0x8B, 0xD9] },
    Site { name: "CScrollBar ctor (0x140112AB0)", rva: 0x0011_2AB0, expect: &[0x48, 0x8D, 0x05, 0xA1, 0x12, 0x40, 0x00, 0x48, 0x89, 0x01] },
    Site { name: "CScrollBar setup (0x1401130C0)", rva: 0x0011_30C0, expect: &[0x48, 0x8B, 0xC4, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x81] },
    Site { name: "CScrollBar update (0x140112CB0)", rva: 0x0011_2CB0, expect: &[0x4C, 0x8B, 0xDC, 0x56, 0x48, 0x81, 0xEC, 0x90, 0x00, 0x00, 0x00] },
    Site { name: "text copy (0x140258DE0)", rva: 0x0025_8DE0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xF9, 0x48, 0x8B, 0xDA] },
    // test cl,cl; je; mov rax,[0x14087A7E8] (the card collection)
    Site { name: "card ever obtained (0x14015B3F0)", rva: 0x0015_B3F0, expect: &[0x84, 0xC9, 0x74, 0x37, 0x48, 0x8B, 0x05, 0xED, 0xF3, 0x71, 0x00] },
    // sub rsp,0x28; movzx eax,byte [rcx]; cmp al,0xDB; jae
    Site { name: "card name (0x140164CA0)", rva: 0x0016_4CA0, expect: &[0x48, 0x83, 0xEC, 0x28, 0x0F, 0xB6, 0x01, 0x3C, 0xDB, 0x73, 0x1D] },
];

/// Re:CoM data the Keyblades page passes to Re:CoM's own code or copies, checked the same way.
pub const KEYBLADE_DATA: &[Site] = &[
    // The entry buttons' first position (86.0, 88.0, 0.0, 1.0); the step 26.0 is at 0x140509270.
    Site { name: "camp entry position (0x140509430)", rva: 0x0050_9430, expect: &[0x00, 0x00, 0xAC, 0x42, 0x00, 0x00, 0xB0, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x3F] },
    Site { name: "camp entry step (0x140509270)", rva: 0x0050_9270, expect: &[0x00, 0x00, 0xD0, 0x41, 0x00, 0x00, 0xE0, 0x41] },
    // The highlight bar's x 48.0 (`0x1400E07D0`; then 50.0), the bar's / glove's first y 80.0 / 82.0
    // (`0x1400E07D0` / `0x1400E3D30`), the glove's x 150.0 (`0x1400E3D30`; then 157.0): the KH1 list's places
    // keep Re:CoM's offsets (`keyblades::top::bar_pos` / `glove_pos`).
    Site { name: "camp highlight bar x (0x140509294)", rva: 0x0050_9294, expect: &[0x00, 0x00, 0x40, 0x42, 0x00, 0x00, 0x48, 0x42] },
    Site { name: "camp bar / glove first y (0x1405092C0)", rva: 0x0050_92C0, expect: &[0x00, 0x00, 0xA0, 0x42, 0x00, 0x00, 0xA4, 0x42] },
    Site { name: "camp glove x (0x1405092F4)", rva: 0x0050_92F4, expect: &[0x00, 0x00, 0x16, 0x43, 0x00, 0x00, 0x1D, 0x43] },
    // The top entries' label ids in Sora's story (`0x1400E3DE0`): 0xA80001..0xA80007
    // (`keyblades::top::RECOM_LABELS`; entry 0 gets its own back when the list leaves KH1's).
    Site { name: "camp entry labels (0x1407C1000)", rva: 0x007C_1000, expect: &[0x01, 0x00, 0xA8, 0x00, 0x02, 0x00, 0xA8, 0x00, 0x03, 0x00, 0xA8, 0x00, 0x04, 0x00, 0xA8, 0x00, 0x05, 0x00, 0xA8, 0x00, 0x06, 0x00, 0xA8, 0x00, 0x07, 0x00, 0xA8, 0x00] },
    // The cursor set's help ids for the top entries 0..6 (`mov dword [rbp+d], id`, read at
    // `[rbp + ((level + story*2)*7 + i)*4 - 0x49]`, `0x1400DEBF4..0x1400DEC0D`): Sora's story
    // 0x500400..0x500406 (with the three moves between them), Riku's 0x50042A 0x500401..0x500404
    // 0x50042C 0x500406 (`keyblades::top::RECOM_HELP`).
    Site { name: "camp cursor set: Sora's top help ids (0x1400DEA45)", rva: 0x000D_EA45, expect: &[0xC7, 0x45, 0xB7, 0x00, 0x04, 0x50, 0x00, 0x41, 0x8B, 0xF0, 0xC7, 0x45, 0xBB, 0x01, 0x04, 0x50, 0x00, 0x8B, 0xFA, 0xC7, 0x45, 0xBF, 0x02, 0x04, 0x50, 0x00, 0x48, 0x8B, 0xD9, 0xC7, 0x45, 0xC3, 0x03, 0x04, 0x50, 0x00, 0xC7, 0x45, 0xC7, 0x04, 0x04, 0x50, 0x00, 0xC7, 0x45, 0xCB, 0x05, 0x04, 0x50, 0x00, 0xC7, 0x45, 0xCF, 0x06, 0x04, 0x50, 0x00] },
    Site { name: "camp cursor set: Riku's top help ids (0x1400DEAA9)", rva: 0x000D_EAA9, expect: &[0xC7, 0x45, 0xEF, 0x2A, 0x04, 0x50, 0x00, 0xC7, 0x45, 0xF3, 0x01, 0x04, 0x50, 0x00, 0xC7, 0x45, 0xF7, 0x02, 0x04, 0x50, 0x00, 0xC7, 0x45, 0xFB, 0x03, 0x04, 0x50, 0x00, 0xC7, 0x45, 0xFF, 0x04, 0x04, 0x50, 0x00, 0xC7, 0x45, 0x03, 0x2C, 0x04, 0x50, 0x00, 0xC7, 0x45, 0x07, 0x06, 0x04, 0x50, 0x00] },
    // Status rows' sequence ids (kind 0x10): 0x12C ×3, 0x12B ×2, 0x129, 0x12A ×3.
    Site { name: "status row sequences (0x1407C11D0)", rva: 0x007C_11D0, expect: &[0x2C, 0x01, 0x2C, 0x01, 0x2C, 0x01, 0x2B, 0x01, 0x2B, 0x01, 0x29, 0x01] },
    // List rows' sequence ids (kind 0x11): 0x12E 0x133 0x131 0x132 0x12F 0x160 0x130.
    Site { name: "list row sequences (0x1407C11F0)", rva: 0x007C_11F0, expect: &[0x2E, 0x01, 0x33, 0x01, 0x31, 0x01, 0x32, 0x01, 0x2F, 0x01, 0x60, 0x01, 0x30, 0x01] },
    // Scroll bar sequence ids (kind 0x14): 0x120 0x11D 0x123.
    Site { name: "scroll bar sequences (0x1407C1228)", rva: 0x007C_1228, expect: &[0x20, 0x01, 0x1D, 0x01, 0x23, 0x01] },
    // The list cursor's second-sprite offsets: y +168.0 ×4 (0x140509520), x +284.0 ×4 (0x140509530).
    Site { name: "list cursor offsets (0x140509520)", rva: 0x0050_9520, expect: &[0x00, 0x00, 0x28, 0x43, 0x00, 0x00, 0x28, 0x43, 0x00, 0x00, 0x28, 0x43, 0x00, 0x00, 0x28, 0x43, 0x00, 0x00, 0x8E, 0x43, 0x00, 0x00, 0x8E, 0x43, 0x00, 0x00, 0x8E, 0x43, 0x00, 0x00, 0x8E, 0x43] },
];

/// The equipped Keyblade's model (`win::models`, decision 25 "Models",
/// `docs/recom-keyblade-models.md` § Runtime). Same rule as [`HOOKS`].
pub const MODEL_HOOKS: &[Site] = &[
    // push rbx; sub rsp,0x20; mov [rsp+0x30],rdi: Re:CoM's "queue the player's resource group if
    // it is not loaded" (`0x140211AA0(char)`), called by the room and battle builders.
    Site { name: "player group load (0x140211AA0)", rva: 0x0021_1AA0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x89, 0x7C, 0x24, 0x30] },
    // push rbx; sub rsp,0x30; mov qword [rsp+0x20],-2: the field's camp open / close
    // (`0x1401A9590(field, open)`); with open = 0 the field resumes after the camp is gone.
    Site { name: "field camp open/close (0x1401A9590)", rva: 0x001A_9590, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF] },
];

/// Re:CoM functions the model code calls (or reaches through a vtable it checks).
pub const MODEL_CALLEES: &[Site] = &[
    // sub rsp,0x38; xor r9d,r9d; mov dword [rsp+0x20],0; mov edx,ecx; mov r8d,0x100
    Site { name: "queue player group (0x14023CCE0)", rva: 0x0023_CCE0, expect: &[0x48, 0x83, 0xEC, 0x38, 0x45, 0x33, 0xC9, 0xC7, 0x44, 0x24, 0x20, 0x00, 0x00, 0x00, 0x00, 0x8B, 0xD1, 0x41, 0xB8, 0x00, 0x01, 0x00, 0x00] },
    // ...; mov eax,[0x14087CC74] (the loader's running flag)
    Site { name: "load queued? (0x14023CA80)", rva: 0x0023_CA80, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x05, 0xDF, 0x01, 0x64, 0x00] },
    Site { name: "resource by type and id (0x1402A3E20)", rva: 0x002A_3E20, expect: &[0x40, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x48, 0x83, 0xEC, 0x20, 0x45, 0x8B, 0xE0, 0x8B, 0xDA] },
    // test ecx,ecx; jle; mov rax,rsp; push r15; sub rsp,0x50
    Site { name: "unload resources by id (0x1402A5B00)", rva: 0x002A_5B00, expect: &[0x85, 0xC9, 0x0F, 0x8E, 0x5D, 0x02, 0x00, 0x00, 0x48, 0x8B, 0xC4, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x50] },
    Site { name: "CPlayerWeapon ctor (0x140224BA0)", rva: 0x0022_4BA0, expect: &[0x48, 0x8B, 0xC4, 0x48, 0x89, 0x48, 0x08, 0x56, 0x57, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x40] },
    // The model init CPlayerWeapon's vtable slot +0x30 holds (shared by many classes).
    Site { name: "entity model init (0x1401A4E70)", rva: 0x001A_4E70, expect: &[0x40, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF] },
    // ...; lea rax,[CPlayerWeapon vtable 0x1405AADB0]: the scalar deleting dtor (vtable slot 0).
    Site { name: "CPlayerWeapon dtor (0x140224EC0)", rva: 0x0022_4EC0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8D, 0x05, 0xDF, 0x5E, 0x38, 0x00] },
    // push rbx; sub rsp,0x20; mov rbx,rcx; mov rcx,[rcx+0x3D0]; test rcx,rcx: put the Keyblade
    // away (`CPlayerSora` vtable +0x100, as stateInitial calls it after building the weapon).
    Site { name: "player put weapon away (0x1401F5480)", rva: 0x001F_5480, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0x48, 0x8B, 0x89, 0xD0, 0x03, 0x00, 0x00, 0x48, 0x85, 0xC9] },
];

/// M6 stage 1, battles at the KH1 setting (`win::battle`, `crate::battle`,
/// `docs/recom-m6-battles.md`). Same rule as [`HOOKS`]. Checked (exe): no branch in the exe and no
/// pointer in its data lands inside the moved bytes (only the function starts are referenced).
pub const M6_HOOKS: &[Site] = &[
    // mov [rsp+8],rbx; mov [rsp+0x10],rsi; push rdi; sub rsp,0x20 (15 bytes; the next
    // instruction, `mov rax,[rip+…]`, is RIP-relative and stays). The Heartless card play,
    // vtable +0x98 of 49 enemy classes; its only caller is the enemy update `0x14018537C`.
    Site { name: "enemy card play (0x140198870)", rva: 0x0019_8870, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20] },
    // mov [rsp+8],rcx; push rbp; push rsi; push rdi; push r12: CBattleEnemyManager's ctor,
    // called once per battle by the battle build `0x1401AEBF0` and the 100 Acre Wood one
    // `0x14042D710`.
    Site { name: "battle enemy manager ctor (0x1401859B0)", rva: 0x0018_59B0, expect: &[0x48, 0x89, 0x4C, 0x24, 0x08, 0x55, 0x56, 0x57, 0x41, 0x54] },
    // mov rax,rsp; push rdi; push r12; push r13; push r14; push r15: CBTL::setPlayCard (log only:
    // which classes still play a side-2 card in a KH1 battle).
    Site { name: "CBTL setPlayCard (0x140163830)", rva: 0x0016_3830, expect: &[0x48, 0x8B, 0xC4, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57] },
];

/// Re:CoM-AI Heartless wait their own attack-end cooldown at KH1 (owner, 2026-11-02; `crate::battle::pace`,
/// `win::battle::attack_end`, `docs/recom-m6-battles.md` § Attack-end cooldown): the wrapper on the
/// enemy attack end. Same rule as [`HOOKS`]. Checked (exe): no branch in `.text` and no relocated
/// pointer lands inside the moved bytes; the start is called directly from 198 sites (and from
/// our KH1 enemies' states, which the wrapper passes through).
pub const M6_PACE_HOOKS: &[Site] = &[
    // mov [rsp+8],rbx; mov [rsp+0x10],rsi; push rdi; sub rsp,0x20 (15 bytes, position-free).
    Site { name: "enemy attack end (0x14017E540)", rva: 0x0017_E540, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20] },
];

/// The attack end's body the wrapper relies on. Not called, not patched. Checked (exe).
pub const M6_PACE_CHECKED: &[Site] = &[
    // mov rbx,rcx; mov esi,r8d (the cooldown argument kept in esi).
    Site { name: "attack end: enemy and cooldown (0x14017E54F)", rva: 0x0017_E54F, expect: &[0x48, 0x8B, 0xD9, 0x41, 0x8B, 0xF0] },
    // mov rax,[rbx+0x260]; and rax,~1; mov [rbx+0x2C8],esi; mov [rbx+0x260],rax; test rdi,rdi;
    // je; (next given) mov [rbx+0x68],rdi; …; ret; (no next) mov rcx,[rbx+0x2F8];
    // mov dword [rbx+0x2C8],0xFA (Re:CoM's 250 at 0x14017E5F5); test rcx,rcx.
    Site { name: "attack end: cooldown, then 250 without a next state (0x14017E5BD)", rva: 0x0017_E5BD, expect: &[0x48, 0x8B, 0x83, 0x60, 0x02, 0x00, 0x00, 0x48, 0x83, 0xE0, 0xFE, 0x89, 0xB3, 0xC8, 0x02, 0x00, 0x00, 0x48, 0x89, 0x83, 0x60, 0x02, 0x00, 0x00, 0x48, 0x85, 0xFF, 0x74, 0x14, 0x48, 0x89, 0x7B, 0x68, 0x48, 0x8B, 0x5C, 0x24, 0x30, 0x48, 0x8B, 0x74, 0x24, 0x38, 0x48, 0x83, 0xC4, 0x20, 0x5F, 0xC3, 0x48, 0x8B, 0x8B, 0xF8, 0x02, 0x00, 0x00, 0xC7, 0x83, 0xC8, 0x02, 0x00, 0x00, 0xFA, 0x00, 0x00, 0x00, 0x48, 0x85, 0xC9] },
];

/// Re:CoM functions the M6 code calls.
pub const M6_CALLEES: &[Site] = &[
    // sub rsp,0x28; mov eax,ecx; test ecx,ecx; jne; mov rax,[rip+…]: a task by id (0 if none).
    Site { name: "task by id (0x1402AD300)", rva: 0x002A_D300, expect: &[0x48, 0x83, 0xEC, 0x28, 0x8B, 0xC1, 0x85, 0xC9, 0x75, 0x0C, 0x48, 0x8B, 0x05, 0x8F, 0x9D, 0x84] },
];

/// Re:CoM code the M6 card-play wrapper does the work of, or relies on: checked so that a build
/// where it differs is never patched. Not called.
pub const M6_CHECKED: &[Site] = &[
    // 0x140198870 after the moved bytes: clear 0x800 | 2 (masks at 0x14052A468 / 0x14052A418),
    // then CBTL = task 0x16.
    Site { name: "card play: clear 0x802, get CBTL (0x14019887F)", rva: 0x0019_887F, expect: &[0x48, 0x8B, 0x05, 0xE2, 0x1B, 0x39, 0x00, 0x48, 0x8B, 0xD9, 0x48, 0x0B, 0x05, 0x88, 0x1B, 0x39, 0x00, 0x48, 0xF7, 0xD0, 0x48, 0x21, 0x81, 0x60, 0x02, 0x00, 0x00, 0xB9, 0x16, 0x00, 0x00, 0x00] },
    // 0x1401634B0's gates: test [+0x208] 0xC0000000, 0xC0000004; cmp byte [+0x1E4],0; cmp
    // [+0x20C],1; ja.
    Site { name: "CBTL enemy-card gates (0x1401634BA)", rva: 0x0016_34BA, expect: &[0x8B, 0x81, 0x08, 0x02, 0x00, 0x00, 0x8B, 0xFA, 0x48, 0x8B, 0xD9, 0xA9, 0x00, 0x00, 0x00, 0xC0, 0x0F, 0x85, 0x86, 0x00, 0x00, 0x00, 0xA9, 0x04, 0x00, 0x00, 0xC0, 0x75, 0x7F, 0x80, 0xB9, 0xE4, 0x01, 0x00, 0x00, 0x00, 0x75, 0x76, 0x83, 0xB9, 0x0C, 0x02, 0x00, 0x00, 0x01, 0x77, 0x6D] },
    // 0x140181620 case 2: and [+0x260],~0x200; call 0x1401809E0; if set: [+0x260] = (| 4) & ~0x10,
    // [+0x68] = enemy + ([+0x278] + 0x13) << 6.
    Site { name: "enemy message 2 (0x140181695)", rva: 0x0018_1695, expect: &[0x48, 0x81, 0xA3, 0x60, 0x02, 0x00, 0x00, 0xFF, 0xFD, 0xFF, 0xFF, 0x48, 0x8B, 0xCB, 0xE8, 0x38, 0xF3, 0xFF, 0xFF, 0x85, 0xC0, 0x74, 0x2C, 0x48, 0x8B, 0x83, 0x60, 0x02, 0x00, 0x00, 0x48, 0x83, 0xC8, 0x04, 0x48, 0x83, 0xE0, 0xEF, 0x48, 0x89, 0x83, 0x60, 0x02, 0x00, 0x00, 0x48, 0x63, 0x83, 0x78, 0x02, 0x00, 0x00, 0x48, 0x83, 0xC0, 0x13, 0x48, 0xC1, 0xE0, 0x06, 0x48, 0x03, 0xC3, 0x48, 0x89, 0x43, 0x68] },
    // CBossMarluxia3's handler: test byte [+0xCE4],4; …; jne (own cases); else jmp 0x140181620.
    Site { name: "Marluxia 3 message handler (0x1403F4550)", rva: 0x003F_4550, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0xF6, 0x81, 0xE4, 0x0C, 0x00, 0x00, 0x04, 0x8B, 0xFA, 0x48, 0x8B, 0xD9, 0x75, 0x0F, 0x48, 0x8B, 0x5C, 0x24, 0x38, 0x48, 0x83, 0xC4, 0x20, 0x5F, 0xE9, 0xA9, 0xD0, 0xD8, 0xFF] },
    // The rest of the think gate P2 replaces: mov rbx,rcx; call 0x1401616A0 (side 2 playing); …;
    // call 0x140186020 (not the last granted); mov eax,1 / xor eax,eax.
    Site { name: "think gate body (0x14017D016)", rva: 0x0017_D016, expect: &[0x48, 0x8B, 0xD9, 0xE8, 0x82, 0x46, 0xFE, 0xFF, 0x85, 0xC0, 0x75, 0x22, 0x8D, 0x48, 0x19, 0xE8, 0xD6, 0x02, 0x13, 0x00, 0x8B, 0x53, 0x20, 0x48, 0x8B, 0xC8, 0xE8, 0xEB, 0x8F, 0x00, 0x00, 0x85, 0xC0, 0x74, 0x0B, 0xB8, 0x01, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3, 0x33, 0xC0, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3] },
    // The grant loop around P3: rcx = [rbx+0x260]; rax = [0x14052A458] (0x200); test rax,rcx; je;
    // (P3's 6 bytes); mov rcx,rbx; mov [rbx+0x260],rax.
    Site { name: "manager grant loop (0x14018764C)", rva: 0x0018_764C, expect: &[0x48, 0x8B, 0x8B, 0x60, 0x02, 0x00, 0x00, 0x48, 0x8B, 0x05, 0xFE, 0x2D, 0x3A, 0x00, 0x48, 0x85, 0xC8, 0x74, 0x27, 0x48, 0xF7, 0xD0, 0x48, 0x23, 0xC1, 0x48, 0x8B, 0xCB, 0x48, 0x89, 0x83, 0x60, 0x02, 0x00, 0x00] },
    // The think rate before P4: r8d = edi (1, or 10 while +0x2D0 > 0) is set before the id test,
    // so the jump P4 makes keeps that factor and drops only the enemy count.
    Site { name: "think rate factor (0x14017F3C4)", rva: 0x0017_F3C4, expect: &[0x0F, 0x4E, 0xFE, 0xF7, 0xE9, 0x44, 0x8B, 0xC7, 0xC1, 0xFA, 0x0C, 0x8B, 0xC2, 0xC1, 0xE8, 0x1F, 0x03, 0xD0, 0x69, 0xC2, 0x10, 0x27, 0x00, 0x00, 0x2B, 0xC8] },
];

/// One code patch: Re:CoM's bytes (checked at start, the [`Site`]) and ours.
pub use crate::battle::Patch;

/// The three serialiser patches (M6 P2–P4, decision 30), switched at each battle start.
pub const M6_PATCHES: &[Patch] = &[
    // P2, the think gate `0x14017D010` (6 callers, all thinks): push rbx; sub rsp,0x20 → mov
    // eax,1; ret. No "wait while an enemy card plays", no "not the last granted twice".
    Patch { site: Site { name: "P2 think gate (0x14017D010)", rva: 0x0017_D010, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] }, new: &[0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3] },
    // P3, the manager's grant `0x140187600`: not rax; and rax,rcx → xor rax,rcx; or ah,8. Every
    // requester's 0x200 becomes 0x800 instead of being dropped; the nearest is still stored.
    Patch { site: Site { name: "P3 grant all (0x14018765F)", rva: 0x0018_765F, expect: &[0x48, 0xF7, 0xD0, 0x48, 0x23, 0xC1] }, new: &[0x48, 0x33, 0xC1, 0x80, 0xCC, 0x08] },
    // P4, the think rate `0x14017F380`: cmp ecx,0x64; jge → cmp ecx,0x64; jmp. No multiply by
    // the number of enemies.
    Patch { site: Site { name: "P4 crowd pace (0x14017F3DE)", rva: 0x0017_F3DE, expect: &[0x83, 0xF9, 0x64, 0x7D, 0x26] }, new: &[0x83, 0xF9, 0x64, 0xEB, 0x26] },
];

/// M6 stage 2, no deck for Sora in battle (`win::deck`, `crate::deck`, `docs/recom-m6-battles.md`
/// § Stage 2). Same rule as [`HOOKS`]. Checked (exe): no branch in the exe and no relocated
/// pointer lands inside the moved bytes; both are reached only by direct calls.
pub const M6_DECK_HOOKS: &[Site] = &[
    // push rdi; push r14; push r15; sub rsp,0x40: the battle revolver build (`player+0x3F0`),
    // called by CPlayer::stateInitial (0x1401FAAD0, battle rooms only) and Riku's (0x140206A1A).
    Site { name: "battle revolver build (0x1401F43D0)", rva: 0x001F_43D0, expect: &[0x40, 0x57, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x40] },
    // push rbx; sub rsp,0x20; mov rbx,rcx: an enemy's card drop (the roll 0x1401602A0 gives only
    // Friend cards 0x31..0x39); called by the hit handler 0x14017FC9D and the battle setup
    // 0x140180366 / 0x1401803F4.
    Site { name: "enemy hit card drop (0x14017E190)", rva: 0x0017_E190, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9] },
];

/// Pass-through hooks that only log, once per battle, a reader of the empty revolver. Installed
/// apart: if they fail, the deck work stands. Same rule as [`HOOKS`] (exe-scanned like
/// [`M6_DECK_HOOKS`]).
pub const M6_DECK_READERS: &[Site] = &[
    // sub rsp,0x28; mov rcx,[rcx+0x3F0]: "hand empty?" (only caller CAtelTutorial 0x140137CB0,
    // which then sets event flag (1, 0x11)).
    Site { name: "tutorial hand-empty check (0x1401EA1A0)", rva: 0x001E_A1A0, expect: &[0x48, 0x83, 0xEC, 0x28, 0x48, 0x8B, 0x89, 0xF0, 0x03, 0x00, 0x00] },
    // sub rsp,0x28; mov rcx,[rcx+0x3F0]: the event script's revolver scroll (only caller the
    // script VM 0x140145C50).
    Site { name: "script revolver scroll (0x1401EA1E0)", rva: 0x001E_A1E0, expect: &[0x48, 0x83, 0xEC, 0x28, 0x48, 0x8B, 0x89, 0xF0, 0x03, 0x00, 0x00] },
    // mov [rsp+8],rbx; mov [rsp+0x10],rdi: a value search over a revolver's cards (callers the
    // enemy card value 0x140182500 and Oogie's 0x1403D91C0).
    Site { name: "revolver value search (0x140167CF0)", rva: 0x0016_7CF0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x7C, 0x24, 0x10] },
];

/// Re:CoM functions the stage-2 code calls.
pub const M6_DECK_CALLEES: &[Site] = &[
    // push rbp; push rsi; push rdi; sub rsp,0x60; mov qword [rsp+0x28],-2: CCardRevolver's ctor
    // (mem, player, list, side, selected), as 0x1401F43D0 calls it.
    Site { name: "CCardRevolver ctor (0x140164D40)", rva: 0x0016_4D40, expect: &[0x40, 0x55, 0x56, 0x57, 0x48, 0x83, 0xEC, 0x60, 0x48, 0xC7, 0x44, 0x24, 0x28, 0xFE, 0xFF, 0xFF] },
    // cmp ecx,2; jae; mov eax,ecx; lea rcx,[rax+rax*2]: a system flag (table, index) -> 0/1.
    Site { name: "system flag get (0x1401B0870)", rva: 0x001B_0870, expect: &[0x83, 0xF9, 0x02, 0x73, 0x40, 0x8B, 0xC1, 0x48, 0x8D, 0x0C, 0x40] },
    // cmp ecx,2; jae; mov eax,ecx; lea rcx,[rax+rax*2]: a system variable (table, index); (1,
    // 0xE2) is Re:CoM's HUD-hide mask.
    Site { name: "system variable get (0x1401B08C0)", rva: 0x001B_08C0, expect: &[0x83, 0xF9, 0x02, 0x73, 0x1D, 0x8B, 0xC1, 0x48, 0x8D, 0x0C, 0x40] },
    // mov rax,[rip+…]; movzx ecx,word [rax+0x187A]; mov eax,0x4B0: room 1200..1255 (minigames).
    Site { name: "minigame room? (0x140235360)", rva: 0x0023_5360, expect: &[0x48, 0x8B, 0x05, 0xA9, 0x5D, 0x64, 0x00, 0x0F, 0xB7, 0x88, 0x7A, 0x18, 0x00, 0x00, 0xB8, 0xB0, 0x04, 0x00, 0x00] },
];

/// Re:CoM's HUD-hide mask reads (`call 0x1401B08C0` with ecx = 1, edx = 0xE2) whose target the
/// stage-2 code points at a stub, per battle, in the player's cockpit update `0x1401689F0`
/// (each read is followed by `test al,2`; rbx = the revolver). `expect` is Re:CoM's call.
pub const M6_COCKPIT_CALLS: &[Site] = &[
    Site { name: "cockpit mask read 1 (0x140168A34)", rva: 0x0016_8A34, expect: &[0xE8, 0x87, 0x7E, 0x04, 0x00] },
    Site { name: "cockpit mask read 2 (0x140168A93)", rva: 0x0016_8A93, expect: &[0xE8, 0x28, 0x7E, 0x04, 0x00] },
    Site { name: "cockpit mask read 3 (0x140168B32)", rva: 0x0016_8B32, expect: &[0xE8, 0x89, 0x7D, 0x04, 0x00] },
    Site { name: "cockpit mask read 4 (0x140168B90)", rva: 0x0016_8B90, expect: &[0xE8, 0x2B, 0x7D, 0x04, 0x00] },
];

/// The same mask reads in CBattleCard code (rbx = the card): the per-frame show / hide
/// `0x140150340` (tests 0x20 for side 1), the deal `0x1401514B0` and refresh `0x1401545A0` (test
/// the whole mask).
pub const M6_CARD_CALLS: &[Site] = &[
    Site { name: "card mask read, show/hide (0x14015037C)", rva: 0x0015_037C, expect: &[0xE8, 0x3F, 0x05, 0x06, 0x00] },
    Site { name: "card mask read, deal (0x140151625)", rva: 0x0015_1625, expect: &[0xE8, 0x96, 0xF2, 0x05, 0x00] },
    Site { name: "card mask read, refresh (0x1401545BD)", rva: 0x0015_45BD, expect: &[0xE8, 0xFE, 0xC2, 0x05, 0x00] },
];

/// Re:CoM code stage 2 relies on (not called, not patched): checked so a build where it differs
/// is never touched.
pub const M6_DECK_CHECKED: &[Site] = &[
    // 0x1401F43D0: mov rdi,rcx; cmp qword [rcx+0x3F0],0; jne tail: a built revolver is kept.
    Site { name: "revolver build: slot test (0x1401F43F2)", rva: 0x001F_43F2, expect: &[0x48, 0x8B, 0xF9, 0x48, 0x83, 0xB9, 0xF0, 0x03, 0x00, 0x00, 0x00, 0x0F, 0x85, 0x54, 0x01, 0x00, 0x00] },
    // Its tail: mov ecx,0x16; call 0x1402AD300; mov [rdi+0x3E8],rax (CBTL, always run).
    Site { name: "revolver build: tail (0x1401F4557)", rva: 0x001F_4557, expect: &[0xB9, 0x16, 0x00, 0x00, 0x00, 0xE8, 0x9F, 0x8D, 0x0B, 0x00, 0x48, 0x89, 0x87, 0xE8, 0x03, 0x00, 0x00] },
    // The deck loader 0x14016C970: an entry whose id byte is 0 or >= 0xCB ends the list.
    Site { name: "deck loader: end of list (0x14016C9A0)", rva: 0x0016_C9A0, expect: &[0x8B, 0x0C, 0x96, 0x89, 0x4C, 0x24, 0x70, 0x84, 0xC9, 0x74, 0x2B, 0x80, 0xF9, 0xCB, 0x73, 0x26] },
    // The functions whose mask reads are retargeted put their object in rbx first (rbx is not
    // written again before the reads: Checked (disasm)).
    Site { name: "cockpit update start (0x1401689F0)", rva: 0x0016_89F0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40, 0x83, 0x79, 0x5C, 0x01, 0x48, 0x8B, 0xD9] },
    Site { name: "card show/hide: rbx = card (0x140150359)", rva: 0x0015_0359, expect: &[0x48, 0x8B, 0xD9, 0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0x55, 0x05, 0x06, 0x00, 0x0F, 0xB6, 0xBB, 0x6C, 0x02, 0x00, 0x00] },
    Site { name: "card deal start (0x1401514B0)", rva: 0x0015_14B0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x80, 0xB9, 0x6C, 0x02, 0x00, 0x00, 0x01] },
    // mov rbx,rcx; mov [rcx+0x68],rax
    Site { name: "card deal: rbx = card (0x1401514CC)", rva: 0x0015_14CC, expect: &[0x48, 0x8B, 0xD9, 0x48, 0x89, 0x41, 0x68] },
    Site { name: "card refresh start (0x1401545A0)", rva: 0x0015_45A0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x83, 0xB9, 0x90, 0x02, 0x00, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
    // Each read's context: mov edx,0xE2; ecx = 1; the call; the test after it.
    Site { name: "cockpit read 1 context (0x140168A2C)", rva: 0x0016_8A2C, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0x8D, 0x48, 0xFD, 0xE8, 0x87, 0x7E, 0x04, 0x00, 0xA8, 0x02] },
    Site { name: "cockpit read 2 context (0x140168A89)", rva: 0x0016_8A89, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0x28, 0x7E, 0x04, 0x00, 0xA8, 0x02] },
    Site { name: "cockpit read 3 context (0x140168B2A)", rva: 0x0016_8B2A, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0x8D, 0x48, 0xFC, 0xE8, 0x89, 0x7D, 0x04, 0x00, 0xA8, 0x02] },
    Site { name: "cockpit read 4 context (0x140168B86)", rva: 0x0016_8B86, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0x2B, 0x7D, 0x04, 0x00, 0xA8, 0x02] },
    Site { name: "card read show/hide context (0x140150372)", rva: 0x0015_0372, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0x3F, 0x05, 0x06, 0x00, 0x40, 0x80, 0xFF, 0x01] },
    Site { name: "card read deal context (0x14015161B)", rva: 0x0015_161B, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0x96, 0xF2, 0x05, 0x00, 0x85, 0xC0] },
    Site { name: "card read refresh context (0x1401545B3)", rva: 0x0015_45B3, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0xFE, 0xC2, 0x05, 0x00, 0x85, 0xC0] },
];

/// M6 stage 3, P5 (`crate::boss`, `win::battle`, `docs/recom-m6-battles.md` § Stage 3): Re:CoM's
/// "Sora's action" getter `0x14017F720`, rewritten in place per battle. `expect` is the whole
/// function (25 bytes) and its 7 `int3` up to the next function `0x14017F740`. Checked (exe): no
/// branch lands inside it but its own `je`, and the only references are its 15 callers (below)
/// and its `.pdata` entry.
pub const M6_BOSS_PATCHES: &[Patch] = &[
    // mov rax,[rip+…] (main player); test rax,rax; je → mov eax,[rax+0x40C]; ret / mov eax,2;
    // ret. Ours: the same first 10 bytes; je +0x0E; mov eax,[rax+0x40C]; cmp eax,0xA; jne +2;
    // mov al,0xB; ret; mov eax,2; ret.
    Patch {
        site: Site {
            name: "P5 Sora's action getter (0x14017F720)",
            rva: 0x0017_F720,
            expect: &[0x48, 0x8B, 0x05, 0x59, 0xBC, 0x6F, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x07, 0x8B, 0x80, 0x0C, 0x04, 0x00, 0x00, 0xC3, 0xB8, 0x02, 0x00, 0x00, 0x00, 0xC3, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
        },
        new: &[0x48, 0x8B, 0x05, 0x59, 0xBC, 0x6F, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x0E, 0x8B, 0x80, 0x0C, 0x04, 0x00, 0x00, 0x83, 0xF8, 0x0A, 0x75, 0x02, 0xB0, 0x0B, 0xC3, 0xB8, 0x02, 0x00, 0x00, 0x00, 0xC3],
    },
];

/// Every caller of the getter (Checked (exe): all 15 `call rel32` to it; no `jmp`, no other
/// reference): the call and the compare after it. Checked so that a build where any reader
/// differs is never patched. Not called. Order and owners: `crate::boss::READERS`.
pub const M6_GETTER_CALLERS: &[Site] = &[
    Site { name: "CHumanBoss card play reads Sora's action (0x140199B2B)", rva: 0x0019_9B2B, expect: &[0xE8, 0xF0, 0x5B, 0xFE, 0xFF, 0x83, 0xF8, 0x0B] },
    Site { name: "Tutorial Leon think reads Sora's action (0x1403A9CFD)", rva: 0x003A_9CFD, expect: &[0xE8, 0x1E, 0x5A, 0xDD, 0xFF, 0x83, 0xF8, 0x0B] },
    // call; mov rcx,rbx; cmp eax,0xB
    Site { name: "Cloud state 1 reads Sora's action 1 (0x1403AFA7F)", rva: 0x003A_FA7F, expect: &[0xE8, 0x9C, 0xFC, 0xDC, 0xFF, 0x48, 0x8B, 0xCB, 0x83, 0xF8, 0x0B] },
    Site { name: "Cloud state 1 reads Sora's action 2 (0x1403AFDB1)", rva: 0x003A_FDB1, expect: &[0xE8, 0x6A, 0xF9, 0xDC, 0xFF, 0x83, 0xF8, 0x0B] },
    // call; movss xmm0,[rip+…]; cmp eax,0xB
    Site { name: "Riku state 1 reads Sora's action (0x1403B7E06)", rva: 0x003B_7E06, expect: &[0xE8, 0x15, 0x79, 0xDC, 0xFF, 0xF3, 0x0F, 0x10, 0x05, 0x65, 0x0A, 0x1A, 0x00, 0x83, 0xF8, 0x0B] },
    Site { name: "Larxene state 1 reads Sora's action (0x1403BD744)", rva: 0x003B_D744, expect: &[0xE8, 0xD7, 0x1F, 0xDC, 0xFF, 0x83, 0xF8, 0x0B] },
    Site { name: "Axel state 1 reads Sora's action (0x1403C1998)", rva: 0x003C_1998, expect: &[0xE8, 0x83, 0xDD, 0xDB, 0xFF, 0x83, 0xF8, 0x0B] },
    Site { name: "Parasite Cage think reads Sora's action (0x1403C273A)", rva: 0x003C_273A, expect: &[0xE8, 0xE1, 0xCF, 0xDB, 0xFF, 0x83, 0xF8, 0x0B] },
    Site { name: "Hook state 1 reads Sora's action (0x1403C9F1A)", rva: 0x003C_9F1A, expect: &[0xE8, 0x01, 0x58, 0xDB, 0xFF, 0x83, 0xF8, 0x0B] },
    Site { name: "Marluxia 1 state 1 reads Sora's action (0x140412B4E)", rva: 0x0041_2B4E, expect: &[0xE8, 0xCD, 0xCB, 0xD6, 0xFF, 0x83, 0xF8, 0x0B] },
    Site { name: "Lexaeus card play reads Sora's action (0x140414F1E)", rva: 0x0041_4F1E, expect: &[0xE8, 0xFD, 0xA7, 0xD6, 0xFF, 0x83, 0xF8, 0x0B] },
    Site { name: "Lexaeus reads Sora's action, 0x22 (0x14041801F)", rva: 0x0041_801F, expect: &[0xE8, 0xFC, 0x76, 0xD6, 0xFF, 0x83, 0xF8, 0x22] },
    Site { name: "Ansem card play reads Sora's action (0x140424BCD)", rva: 0x0042_4BCD, expect: &[0xE8, 0x4E, 0xAB, 0xD5, 0xFF, 0x83, 0xF8, 0x0B] },
    Site { name: "Ansem reads Sora's action, 0xC (0x140426890)", rva: 0x0042_6890, expect: &[0xE8, 0x8B, 0x8E, 0xD5, 0xFF, 0x83, 0xF8, 0x0C] },
    Site { name: "Ansem shadow uppercut reads Sora's action, 0xC (0x14045FF38)", rva: 0x0045_FF38, expect: &[0xE8, 0xE3, 0xF7, 0xD1, 0xFF, 0x83, 0xF8, 0x0C] },
];

/// M6 stage 3: Re:CoM's HUD-hide mask reads (`call 0x1401B08C0`, ecx = 1, edx = 0xE2) that draw
/// the enemy side's card UI, retargeted per battle with stage 2's reads (`win::deck`) to a stub
/// that adds bit 4. `expect` is Re:CoM's call. Checked (exe): no branch or pointer lands inside
/// any of them.
pub const M6_ENEMY_UI_CALLS: &[Site] = &[
    // The card-play cue sprite (CBTL +0x220): any nonzero mask stops it (`test eax,eax`).
    Site { name: "enemy cue mask read (0x14015E3F3)", rva: 0x0015_E3F3, expect: &[0xE8, 0xC8, 0x24, 0x05, 0x00] },
    Site { name: "enemy cue mask read, CBTL update (0x140161A64)", rva: 0x0016_1A64, expect: &[0xE8, 0x57, 0xEE, 0x04, 0x00] },
    // The revolver's stock row 0x140169CA0 (`test edi,eax`, edi = 4 for side 2, 2 for side 1).
    Site { name: "stock row mask read 1 (0x140169CE3)", rva: 0x0016_9CE3, expect: &[0xE8, 0xD8, 0x6B, 0x04, 0x00] },
    Site { name: "stock row mask read 2 (0x140169D07)", rva: 0x0016_9D07, expect: &[0xE8, 0xB4, 0x6B, 0x04, 0x00] },
    Site { name: "stock row mask read 3 (0x140169D48)", rva: 0x0016_9D48, expect: &[0xE8, 0x73, 0x6B, 0x04, 0x00] },
    // The stock row's show 0x14016DE70: its value digits (`test ebx,eax`, ebx = 4 / 2).
    Site { name: "stock row show mask read (0x14016E2B1)", rva: 0x0016_E2B1, expect: &[0xE8, 0x0A, 0x26, 0x04, 0x00] },
    // Marluxia 3's CStockFolder 0x140169070 (`test al,4`).
    Site { name: "stock folder mask read 1 (0x1401690A6)", rva: 0x0016_90A6, expect: &[0xE8, 0x15, 0x78, 0x04, 0x00] },
    Site { name: "stock folder mask read 2 (0x1401690DD)", rva: 0x0016_90DD, expect: &[0xE8, 0xDE, 0x77, 0x04, 0x00] },
    Site { name: "stock folder mask read 3 (0x140169103)", rva: 0x0016_9103, expect: &[0xE8, 0xB8, 0x77, 0x04, 0x00] },
];

/// The context of each read above (mov edx,0xE2; ecx = 1; the call; the test after it) and the
/// starts of their functions. Checked so a build where they differ is never touched.
pub const M6_ENEMY_UI_CHECKED: &[Site] = &[
    Site { name: "cue read context (0x14015E3E9)", rva: 0x0015_E3E9, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0xC8, 0x24, 0x05, 0x00, 0x85, 0xC0] },
    Site { name: "cue read context, CBTL update (0x140161A5A)", rva: 0x0016_1A5A, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0x57, 0xEE, 0x04, 0x00, 0x85, 0xC0] },
    // lea ecx,[rbp-1] with ebp = 2 (0x140169CBE).
    Site { name: "stock row read 1 context (0x140169CDB)", rva: 0x0016_9CDB, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0x8D, 0x4D, 0xFF, 0xE8, 0xD8, 0x6B, 0x04, 0x00, 0x85, 0xC7] },
    Site { name: "stock row read 2 context (0x140169CFD)", rva: 0x0016_9CFD, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0xB4, 0x6B, 0x04, 0x00, 0x85, 0xC7] },
    Site { name: "stock row read 3 context (0x140169D3E)", rva: 0x0016_9D3E, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0x73, 0x6B, 0x04, 0x00, 0x85, 0xC7] },
    // mov eax,2; mov ebx,4; mov edx,0xE2; cmove ebx,eax; lea ecx,[rax-1]; the call; test ebx,eax.
    Site { name: "stock row show read context (0x14016E29C)", rva: 0x0016_E29C, expect: &[0xB8, 0x02, 0x00, 0x00, 0x00, 0xBB, 0x04, 0x00, 0x00, 0x00, 0xBA, 0xE2, 0x00, 0x00, 0x00, 0x0F, 0x44, 0xD8, 0x8D, 0x48, 0xFF, 0xE8, 0x0A, 0x26, 0x04, 0x00, 0x85, 0xC3] },
    Site { name: "stock folder read 1 context (0x14016909C)", rva: 0x0016_909C, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0x15, 0x78, 0x04, 0x00, 0xA8, 0x04] },
    Site { name: "stock folder read 2 context (0x1401690D3)", rva: 0x0016_90D3, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0xDE, 0x77, 0x04, 0x00, 0xA8, 0x04] },
    Site { name: "stock folder read 3 context (0x1401690F9)", rva: 0x0016_90F9, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0xB8, 0x77, 0x04, 0x00, 0xA8, 0x04] },
    // The functions: 0x14015E390 (size 0xB2), the CBTL update 0x1401617F0 (0x363), the stock row
    // 0x140169CA0 (0x23A; cmp [rcx+0x5C],1 / edi = 4 / ebp = 2 / cmove edi,ebp), its show
    // 0x14016DE70 (0x54B), the stock folder 0x140169070 (0x3D3).
    Site { name: "cue update start (0x14015E390)", rva: 0x0015_E390, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40] },
    Site { name: "CBTL update start (0x1401617F0)", rva: 0x0016_17F0, expect: &[0x40, 0x53, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x58] },
    Site { name: "stock row start (0x140169CA0)", rva: 0x0016_9CA0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x83, 0x79, 0x5C, 0x01, 0xBF, 0x04, 0x00, 0x00, 0x00, 0x8B, 0x81, 0x58, 0x02, 0x00, 0x00, 0xBD, 0x02, 0x00, 0x00, 0x00, 0x0F, 0x44, 0xFD] },
    Site { name: "stock row show start (0x14016DE70)", rva: 0x0016_DE70, expect: &[0x40, 0x55, 0x57, 0x48, 0x83, 0xEC, 0x78] },
    Site { name: "stock folder start (0x140169070)", rva: 0x0016_9070, expect: &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x55, 0x48, 0x83, 0xEC, 0x30] },
];

/// Enemy card sounds at KH1 (owner, 2026-11-02, `crate::boss::sound`): Re:CoM's `call
/// 0x1402AA360` (its 2D sound play: ecx = id, edx = -1) at the nine places a revolver or stock
/// folder plays its reload, deal, stock or page-switch sound, retargeted per battle with the mask reads
/// (`win::deck`) to a stub that hands our function the object (rbx, or rdi at the folder), which
/// skips the play for a side-2 object. `expect` is Re:CoM's call. Checked (exe): no branch or
/// pointer lands inside any of them; the object register is not written between the function's
/// `mov <reg>,rcx` and the call (disasm). Order: `crate::boss::SOUND_SITES`.
pub const M6_ENEMY_SOUND_CALLS: &[Site] = &[
    Site { name: "reload done at once, 0x15 (0x140167B01)", rva: 0x0016_7B01, expect: &[0xE8, 0x5A, 0x28, 0x14, 0x00] },
    Site { name: "reload gauge step, 0x14 (0x140167BDA)", rva: 0x0016_7BDA, expect: &[0xE8, 0x81, 0x27, 0x14, 0x00] },
    Site { name: "reload charge loop, 0x13 (0x140167C01)", rva: 0x0016_7C01, expect: &[0xE8, 0x5A, 0x27, 0x14, 0x00] },
    Site { name: "reload done, 0x15 (0x140167C93)", rva: 0x0016_7C93, expect: &[0xE8, 0xC8, 0x26, 0x14, 0x00] },
    Site { name: "deal after a reload, +0x204 (0x14016B3E7)", rva: 0x0016_B3E7, expect: &[0xE8, 0x74, 0xEF, 0x13, 0x00] },
    Site { name: "first deal, +0x204 (0x14016A091)", rva: 0x0016_A091, expect: &[0xE8, 0xCA, 0x02, 0x14, 0x00] },
    Site { name: "stock, 0xF (0x14016DCA4)", rva: 0x0016_DCA4, expect: &[0xE8, 0xB7, 0xC6, 0x13, 0x00] },
    Site { name: "stock folder add, 0xF (0x14016DE3F)", rva: 0x0016_DE3F, expect: &[0xE8, 0x1C, 0xC5, 0x13, 0x00] },
    // Sound 3 is a generic id; only this call (in the revolver's page switch) is retargeted.
    Site { name: "revolver page switch, 3 (0x140167A35)", rva: 0x0016_7A35, expect: &[0xE8, 0x26, 0x29, 0x14, 0x00] },
];

/// Each call's context (the id and volume loads, the call, and what uses its result) and the
/// start of its function up to `mov rbx,rcx` / `mov rdi,rcx`. Checked so a build where they
/// differ is never touched.
pub const M6_ENEMY_SOUND_CHECKED: &[Site] = &[
    // test edi,edi; jne +0xB; lea edx,[rdi-1]; lea ecx,[rdi+0x15]: id 0x15 with edi = 0.
    Site { name: "reload done at once context (0x140167AF7)", rva: 0x0016_7AF7, expect: &[0x85, 0xFF, 0x75, 0x0B, 0x8D, 0x57, 0xFF, 0x8D, 0x4F, 0x15, 0xE8, 0x5A, 0x28, 0x14, 0x00] },
    // mov edx,-1; imul edi,ecx,0x64; lea ecx,[rdx+0x15] (0x14); mov [rbx+0x84],eax; the call.
    Site { name: "reload gauge step context (0x140167BC9)", rva: 0x0016_7BC9, expect: &[0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x6B, 0xF9, 0x64, 0x8D, 0x4A, 0x15, 0x89, 0x83, 0x84, 0x00, 0x00, 0x00, 0xE8, 0x81, 0x27, 0x14, 0x00] },
    // mov edx,-1; lea ecx,[rdx+0x14] (0x13); the call; mov [rbx+0x200],eax (the loop's handle).
    Site { name: "reload charge loop context (0x140167BF9)", rva: 0x0016_7BF9, expect: &[0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A, 0x14, 0xE8, 0x5A, 0x27, 0x14, 0x00, 0x89, 0x83, 0x00, 0x02, 0x00, 0x00] },
    // mov edx,-1; lea ecx,[rdx+0x16] (0x15); the call.
    Site { name: "reload done context (0x140167C8B)", rva: 0x0016_7C8B, expect: &[0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A, 0x16, 0xE8, 0xC8, 0x26, 0x14, 0x00] },
    // mov ecx,[rbx+0x204] (the revolver's deal sound); mov edx,-1; the call.
    Site { name: "deal after a reload context (0x14016B3DC)", rva: 0x0016_B3DC, expect: &[0x8B, 0x8B, 0x04, 0x02, 0x00, 0x00, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0xE8, 0x74, 0xEF, 0x13, 0x00] },
    Site { name: "first deal context (0x14016A086)", rva: 0x0016_A086, expect: &[0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8B, 0x8B, 0x04, 0x02, 0x00, 0x00, 0xE8, 0xCA, 0x02, 0x14, 0x00] },
    // mov edx,-1; lea ecx,[rdx+0x10] (0xF); the call.
    Site { name: "stock context (0x14016DC9C)", rva: 0x0016_DC9C, expect: &[0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A, 0x10, 0xE8, 0xB7, 0xC6, 0x13, 0x00] },
    Site { name: "stock folder add context (0x14016DE37)", rva: 0x0016_DE37, expect: &[0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A, 0x10, 0xE8, 0x1C, 0xC5, 0x13, 0x00] },
    // cmp [rsp+0x60],r13d (r13d = 0; the third argument, spilled at entry); jne past the call:
    // the page switch plays its sound only when called with 0.
    Site { name: "page switch sound guard (0x140167A26)", rva: 0x0016_7A26, expect: &[0x44, 0x39, 0x6C, 0x24, 0x60, 0x75, 0x0D] },
    // mov edx,-1; lea ecx,[rdx+4] (3); the call.
    Site { name: "page switch context (0x140167A2D)", rva: 0x0016_7A2D, expect: &[0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A, 0x04, 0xE8, 0x26, 0x29, 0x14, 0x00] },
    // The functions, each up to its `mov rbx,rcx` (rdi at the folder): the reload step
    // 0x140167AA0 (size 0x246), the deal 0x14016B2B0 (0x171), the first deal 0x140169F40 (0x206),
    // the stock 0x14016DB60 (0x180), the folder add 0x14016DD20 (0x14B).
    Site { name: "reload step start (0x140167AA0)", rva: 0x0016_7AA0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x6C, 0x24, 0x20, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0xFA, 0x48, 0x8B, 0xD9] },
    Site { name: "revolver deal start (0x14016B2B0)", rva: 0x0016_B2B0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x48, 0x89, 0x74, 0x24, 0x48, 0x48, 0x8B, 0xD9] },
    Site { name: "revolver first deal start (0x140169F40)", rva: 0x0016_9F40, expect: &[0x48, 0x89, 0x5C, 0x24, 0x20, 0x56, 0x57, 0x41, 0x54, 0x48, 0x83, 0xEC, 0x20, 0x83, 0x79, 0x5C, 0x01, 0x41, 0xBC, 0x02, 0x00, 0x00, 0x00, 0x4C, 0x89, 0x7C, 0x24, 0x50, 0x48, 0x8B, 0xD9] },
    Site { name: "revolver stock start (0x14016DB60)", rva: 0x0016_DB60, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0xBA, 0x0A, 0x00, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
    // mov [rsp+0x18],r8d; push rbx; push r12; sub rsp,0x38; cmp [rcx+0x60],1; mov eax,r8d;
    // mov r12d,edx; mov rbx,rcx: the page switch 0x140167910 (size 0x184).
    Site { name: "revolver page switch start (0x140167910)", rva: 0x0016_7910, expect: &[0x44, 0x89, 0x44, 0x24, 0x18, 0x53, 0x41, 0x54, 0x48, 0x83, 0xEC, 0x38, 0x83, 0x79, 0x60, 0x01, 0x41, 0x8B, 0xC0, 0x44, 0x8B, 0xE2, 0x48, 0x8B, 0xD9] },
    Site { name: "stock folder add start (0x14016DD20)", rva: 0x0016_DD20, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x50, 0x48, 0xC7, 0x44, 0x24, 0x40, 0xFE, 0xFF, 0xFF, 0xFF, 0x48, 0x89, 0x5C, 0x24, 0x60, 0x48, 0x89, 0x74, 0x24, 0x68, 0x41, 0x8B, 0xD8, 0x8B, 0xF2, 0x48, 0x8B, 0xF9] },
    // The revolver ctor 0x140164D40 sets the deal sound: mov dword [rdi+0x204],0x1B (side 2) /
    // 0x12 (side 1).
    Site { name: "revolver ctor: enemy deal sound 0x1B (0x140164DD6)", rva: 0x0016_4DD6, expect: &[0xC7, 0x87, 0x04, 0x02, 0x00, 0x00, 0x1B, 0x00, 0x00, 0x00] },
    Site { name: "revolver ctor: Sora's deal sound 0x12 (0x140164DF5)", rva: 0x0016_4DF5, expect: &[0xC7, 0x87, 0x04, 0x02, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00] },
];

/// M6 stage 3, log only: Sora's card-duel init (`CPlayerSora` vtable `+0x1E0`, action 0x24, from
/// hit reaction 0x21 in `0x1401E6DE0`; Marluxia 3's countdown). A pass-through, installed apart.
/// Same rule as [`HOOKS`]; Checked (exe): referenced only by the vtable and `.pdata`, no branch
/// inside the moved bytes.
pub const M6_BOSS_HOOKS: &[Site] = &[
    // mov [rsp+8],rbx; mov [rsp+0x10],rsi; push rdi; sub rsp,0x40
    Site { name: "Sora card duel init (0x140220C40)", rva: 0x0022_0C40, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x40] },
];

/// Boss revenge (`crate::revenge`, `docs/recom-boss-revenge.md`): the steps of Re:CoM's flinch
/// (state 11, vtable `+0x1F8` of every boss class) and blow-away (state 12, `+0x210`). Same rule
/// as [`HOOKS`]. Checked (exe): the only direct reference to each start is a call / jmp to the
/// start itself (`0x140428A64` jmp, the vtables, `.pdata`); no branch lands inside the moved
/// bytes.
pub const REVENGE_HOOKS: &[Site] = &[
    // rex push rdi; sub rsp,0x40
    Site { name: "enemy flinch step (0x140184100)", rva: 0x0018_4100, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x40] },
    // rex push rbx; sub rsp,0x40
    Site { name: "enemy blow-away step (0x140182E50)", rva: 0x0018_2E50, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40] },
    // and qword [rcx+0x260],-6 (then `mov edx,-1; jmp 0x1401A2DB0`, checked in
    // [`REVENGE_CHECKED`]): Re:CoM's attack-state end (vtable +0x440 default; clears bits 0 / 2,
    // hurt bodies back on). Revenge moves untouchable (2026-11-02): closes the window. Only these 8
    // bytes move; the function is a 0x12-byte leaf with no branch into it but its entry (Checked
    // (exe): rel32 scan).
    Site { name: "enemy attack-state end (0x14017DA90)", rva: 0x0017_DA90, expect: &[0x48, 0x83, 0xA1, 0x60, 0x02, 0x00, 0x00, 0xFA] },
];

/// Revenge moves untouchable: bytes the window relies on, checked at install (not patched).
pub const REVENGE_CHECKED: &[Site] = &[
    // The rest of `0x14017DA90` after the 8 bytes our hook moves: mov edx,-1; jmp 0x1401A2DB0
    // (every hurt body back on), reached through the trampoline.
    Site { name: "enemy attack-state end, its tail (0x14017DA98)", rva: 0x0017_DA98, expect: &[0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0xE9, 0x0E, 0x53, 0x02, 0x00] },
    // Vexen's +0x438: push rbx; sub rsp,0x20; mov rbx,rcx; call 0x14017DA80 (the hurt-off hook,
    // enemy still in rcx), then his own extras.
    Site { name: "CBossVexen hurt off calls 0x14017DA80 (0x1403B3260)", rva: 0x003B_3260, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0xE8, 0x12, 0xA8, 0xDC, 0xFF] },
    // Marluxia 2's +0x438: mov [rsp+8],rbx; push rdi; sub rsp,0x20; mov rbx,rcx; call 0x14017DA80,
    // then its two parts' bodies off.
    Site { name: "CBossMarluxia2 hurt off calls 0x14017DA80 (0x140400410)", rva: 0x0040_0410, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0xE8, 0x5E, 0xD6, 0xD7, 0xFF] },
    // Marluxia 2's +0x440: mov [rsp+8],rbx; mov [rsp+0x10],rsi; push rdi; sub rsp,0x20;
    // mov rsi,rcx; call 0x14017DA90, then its parts.
    // mov rcx,[rcx+0xF8]; test rcx,rcx; jnz 0x140155490: every hurt body back on (the call
    // `0x14017DA90` ends with). Called when Hades' wind-up ends.
    Site { name: "hurt bodies on (0x1401A2DB0)", rva: 0x001A_2DB0, expect: &[0x48, 0x8B, 0x89, 0xF8, 0x00, 0x00, 0x00, 0x48, 0x85, 0xC9, 0x0F, 0x85, 0xE0, 0x26, 0xFB, 0xFF] },
    Site { name: "CBossMarluxia2 attack end calls 0x14017DA90 (0x140400460)", rva: 0x0040_0460, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xF1, 0xE8, 0x19, 0xD6, 0xD7, 0xFF] },
];

/// Boss revenge: the enemy factory `0x140188F40`'s `mov ecx, size` for each revenge boss body
/// (`crate::revenge::BOSSES`), raised by the counter's 0x30-byte tail at install. Checked (exe:
/// each is the size passed to the allocator `0x1404ACE54` right before the class's ctor).
pub const REVENGE_ALLOC: &[Site] = &[
    Site { name: "alloc CTutorialHood (0x140189517)", rva: 0x0018_9517, expect: &[0xB9, 0xE0, 0x0C, 0x00, 0x00] },
    Site { name: "alloc CTutorialLeon (0x140189541)", rva: 0x0018_9541, expect: &[0xB9, 0x10, 0x0D, 0x00, 0x00] },
    Site { name: "alloc CBossCloud (0x140189595)", rva: 0x0018_9595, expect: &[0xB9, 0x30, 0x0D, 0x00, 0x00] },
    Site { name: "alloc CBossHades (0x1401895BF)", rva: 0x0018_95BF, expect: &[0xB9, 0x80, 0x0D, 0x00, 0x00] },
    Site { name: "alloc CBossHook (0x140189613)", rva: 0x0018_9613, expect: &[0xB9, 0x90, 0x0D, 0x00, 0x00] },
    Site { name: "alloc CBossVexen (0x14018963D)", rva: 0x0018_963D, expect: &[0xB9, 0x70, 0x0D, 0x00, 0x00] },
    Site { name: "alloc CBossRiku (0x140189667)", rva: 0x0018_9667, expect: &[0xB9, 0x70, 0x0D, 0x00, 0x00] },
    Site { name: "alloc CBossLarxene (0x140189691)", rva: 0x0018_9691, expect: &[0xB9, 0xA0, 0x0D, 0x00, 0x00] },
    Site { name: "alloc CBossAxel (0x1401896BB)", rva: 0x0018_96BB, expect: &[0xB9, 0x70, 0x0D, 0x00, 0x00] },
    Site { name: "alloc CBossMarluxia (0x140189A2D)", rva: 0x0018_9A2D, expect: &[0xB9, 0x50, 0x0D, 0x00, 0x00] },
    Site { name: "alloc CBossMarluxia2 (0x14018995B)", rva: 0x0018_995B, expect: &[0xB9, 0xF0, 0x0D, 0x00, 0x00] },
    Site { name: "alloc CBossMarluxia3 (0x1401899D9)", rva: 0x0018_99D9, expect: &[0xB9, 0x50, 0x0F, 0x00, 0x00] },
];

/// Boss revenge: each revenge boss class's state 11 step (vtable `+0x1F8`) and state 12 step
/// (`+0x210`) are the two hooked functions, so the check sees every boss flinch. Checked (exe).
/// Revenge moves untouchable (2026-11-02): each class's attack-state begin extra `+0x438` reaches the
/// hurt-off hook `0x14017DA80` (directly, or through Vexen's / Marluxia 2's own, which call it:
/// [`REVENGE_CHECKED`]) and its end `+0x440` the attack-state end hook `0x14017DA90`. Checked (exe).
pub const REVENGE_PTRS: &[PtrSite] = &[
    PtrSite { name: "CTutorialHood +0x1F8", rva: 0x0065_CD00, target: 0x0018_4100 },
    PtrSite { name: "CTutorialLeon +0x1F8", rva: 0x0065_DFE0, target: 0x0018_4100 },
    PtrSite { name: "CBossCloud +0x1F8", rva: 0x0066_0DA0, target: 0x0018_4100 },
    PtrSite { name: "CBossHades +0x1F8", rva: 0x0066_2230, target: 0x0018_4100 },
    PtrSite { name: "CBossHook +0x1F8", rva: 0x0066_B010, target: 0x0018_4100 },
    PtrSite { name: "CBossVexen +0x1F8", rva: 0x0066_38A8, target: 0x0018_4100 },
    PtrSite { name: "CBossRiku +0x1F8", rva: 0x0066_4E48, target: 0x0018_4100 },
    PtrSite { name: "CBossLarxene +0x1F8", rva: 0x0066_6790, target: 0x0018_4100 },
    PtrSite { name: "CBossAxel +0x1F8", rva: 0x0066_7ED0, target: 0x0018_4100 },
    PtrSite { name: "CBossMarluxia +0x1F8", rva: 0x0067_ED58, target: 0x0018_4100 },
    PtrSite { name: "CBossMarluxia2 +0x1F8", rva: 0x0067_B188, target: 0x0018_4100 },
    PtrSite { name: "CBossMarluxia3 +0x1F8", rva: 0x0067_87B0, target: 0x0018_4100 },
    PtrSite { name: "CTutorialHood +0x210", rva: 0x0065_CD18, target: 0x0018_2E50 },
    PtrSite { name: "CTutorialLeon +0x210", rva: 0x0065_DFF8, target: 0x0018_2E50 },
    PtrSite { name: "CBossCloud +0x210", rva: 0x0066_0DB8, target: 0x0018_2E50 },
    PtrSite { name: "CBossHades +0x210", rva: 0x0066_2248, target: 0x0018_2E50 },
    PtrSite { name: "CBossHook +0x210", rva: 0x0066_B028, target: 0x0018_2E50 },
    PtrSite { name: "CBossVexen +0x210", rva: 0x0066_38C0, target: 0x0018_2E50 },
    PtrSite { name: "CBossRiku +0x210", rva: 0x0066_4E60, target: 0x0018_2E50 },
    PtrSite { name: "CBossLarxene +0x210", rva: 0x0066_67A8, target: 0x0018_2E50 },
    PtrSite { name: "CBossAxel +0x210", rva: 0x0066_7EE8, target: 0x0018_2E50 },
    PtrSite { name: "CBossMarluxia +0x210", rva: 0x0067_ED70, target: 0x0018_2E50 },
    PtrSite { name: "CBossMarluxia2 +0x210", rva: 0x0067_B1A0, target: 0x0018_2E50 },
    PtrSite { name: "CBossMarluxia3 +0x210", rva: 0x0067_87C8, target: 0x0018_2E50 },
    PtrSite { name: "CTutorialHood +0x438", rva: 0x0065_CF40, target: 0x0017_DA80 },
    PtrSite { name: "CTutorialLeon +0x438", rva: 0x0065_E220, target: 0x0017_DA80 },
    PtrSite { name: "CBossCloud +0x438", rva: 0x0066_0FE0, target: 0x0017_DA80 },
    PtrSite { name: "CBossHades +0x438", rva: 0x0066_2470, target: 0x0017_DA80 },
    PtrSite { name: "CBossHook +0x438", rva: 0x0066_B250, target: 0x0017_DA80 },
    PtrSite { name: "CBossVexen +0x438", rva: 0x0066_3AE8, target: 0x003B_3260 },
    PtrSite { name: "CBossRiku +0x438", rva: 0x0066_5088, target: 0x0017_DA80 },
    PtrSite { name: "CBossLarxene +0x438", rva: 0x0066_69D0, target: 0x0017_DA80 },
    PtrSite { name: "CBossAxel +0x438", rva: 0x0066_8110, target: 0x0017_DA80 },
    PtrSite { name: "CBossMarluxia +0x438", rva: 0x0067_EF98, target: 0x0017_DA80 },
    PtrSite { name: "CBossMarluxia2 +0x438", rva: 0x0067_B3C8, target: 0x0040_0410 },
    PtrSite { name: "CBossMarluxia3 +0x438", rva: 0x0067_89F0, target: 0x0017_DA80 },
    PtrSite { name: "CTutorialHood +0x440", rva: 0x0065_CF48, target: 0x0017_DA90 },
    PtrSite { name: "CTutorialLeon +0x440", rva: 0x0065_E228, target: 0x0017_DA90 },
    PtrSite { name: "CBossCloud +0x440", rva: 0x0066_0FE8, target: 0x0017_DA90 },
    PtrSite { name: "CBossHades +0x440", rva: 0x0066_2478, target: 0x0017_DA90 },
    PtrSite { name: "CBossHook +0x440", rva: 0x0066_B258, target: 0x0017_DA90 },
    PtrSite { name: "CBossVexen +0x440", rva: 0x0066_3AF0, target: 0x0017_DA90 },
    PtrSite { name: "CBossRiku +0x440", rva: 0x0066_5090, target: 0x0017_DA90 },
    PtrSite { name: "CBossLarxene +0x440", rva: 0x0066_69D8, target: 0x0017_DA90 },
    PtrSite { name: "CBossAxel +0x440", rva: 0x0066_8118, target: 0x0017_DA90 },
    PtrSite { name: "CBossMarluxia +0x440", rva: 0x0067_EFA0, target: 0x0017_DA90 },
    PtrSite { name: "CBossMarluxia2 +0x440", rva: 0x0067_B3D0, target: 0x0040_0460 },
    PtrSite { name: "CBossMarluxia3 +0x440", rva: 0x0067_89F8, target: 0x0017_DA90 },
];

/// Revenge check timing (owner, 2026-11-02: KH2's frame 20, or the reaction's end if sooner): each
/// revenge boss class's state-machine vtable (the `CFSM` base at `+0x58`, RTTI COL offset 0x58)
/// slot `+0x28`, the state-change callback `0x1401A3B80` that `0x140242F50` calls between the old
/// state's end and the new state's begin. Ours (`win::revenge::state_change`) is written in the
/// slot; it calls Re:CoM's. Only these 12 classes' tables change. Checked (exe + disasm).
pub const REVENGE_FSM_SLOTS: &[PtrSite] = &[
    PtrSite { name: "CTutorialHood FSM +0x28", rva: 0x0065_CF90, target: 0x001A_3B80 },
    PtrSite { name: "CTutorialLeon FSM +0x28", rva: 0x0065_E2D0, target: 0x001A_3B80 },
    PtrSite { name: "CBossCloud FSM +0x28", rva: 0x0066_1090, target: 0x001A_3B80 },
    PtrSite { name: "CBossHades FSM +0x28", rva: 0x0066_2520, target: 0x001A_3B80 },
    PtrSite { name: "CBossHook FSM +0x28", rva: 0x0066_B300, target: 0x001A_3B80 },
    PtrSite { name: "CBossVexen FSM +0x28", rva: 0x0066_3B98, target: 0x001A_3B80 },
    PtrSite { name: "CBossRiku FSM +0x28", rva: 0x0066_5138, target: 0x001A_3B80 },
    PtrSite { name: "CBossLarxene FSM +0x28", rva: 0x0066_6A80, target: 0x001A_3B80 },
    PtrSite { name: "CBossAxel FSM +0x28", rva: 0x0066_81C0, target: 0x001A_3B80 },
    PtrSite { name: "CBossMarluxia FSM +0x28", rva: 0x0067_F048, target: 0x001A_3B80 },
    PtrSite { name: "CBossMarluxia2 FSM +0x28", rva: 0x0067_B418, target: 0x001A_3B80 },
    PtrSite { name: "CBossMarluxia3 FSM +0x28", rva: 0x0067_8A40, target: 0x001A_3B80 },
];

/// Hades untouchable only in his wind-up (owner, 2026-11-02): CBossHades' vtable slot of state 5's
/// exe (`+0x168`, reached by the state handler `0x140182BC0`: `jmp [rax+0x168]`), written with ours
/// (`win::revenge::windup_exe`), which calls his. Checked (exe + disasm).
pub const REVENGE_WINDUP_SLOTS: &[PtrSite] = &[
    PtrSite { name: "CBossHades +0x168 (state 5 exe)", rva: 0x0066_21A0, target: 0x003B_1DB0 },
];

/// Bosses untouchable during room-dimming moves (owner, 2026-11-03; `crate::dim`): the four attack
/// states whose step asks for the dark room partway through, their exes' vtable slots
/// (`crate::dim::EXES`), written with ours (`win::revenge::dim_exe_*`), which call Re:CoM's.
/// Reached by Re:CoM's state handlers (`jmp [rax+0x138]` `0x140182B84` / `0x14019B1C7`, `+0x150`
/// `0x140182BA4` / `0x14019B1EB`, `+0x198` `0x140182C04`, `+0x1C8` `0x140182C44`). Checked (exe).
pub const DIM_EXE_SLOTS: &[PtrSite] = &[
    PtrSite { name: "CBossLarxene +0x138 (state 3 exe)", rva: 0x0066_66D0, target: 0x003B_BAF0 },
    PtrSite { name: "CBossLarxene +0x150 (state 4 exe)", rva: 0x0066_66E8, target: 0x003B_C1D0 },
    PtrSite { name: "CBossMarluxia3 +0x198 (state 7 exe)", rva: 0x0067_8750, target: 0x003F_5F90 },
    PtrSite { name: "CBossMarluxia3 +0x1C8 (state 9 exe)", rva: 0x0067_8780, target: 0x003F_6960 },
];

/// Bosses untouchable during room-dimming moves: Re:CoM's scene colour setter `0x140232170` (read,
/// not hooked), whose bytes give the table's global and the slot layout `crate::dim` reads:
/// movsxd rax,[rsp+0x28] (slot); shl rax,6 (0x40 a slot); add rax,[rip+0x649670] (the table
/// `0x14087B7F0`); … mov [rax+0x34],0 (progress); … movdqa [rax+0x10] (target); or [rax+0x3C],1;
/// movdqa [rax+0x20] (current); mov [rax+0x30],ecx (mask); ret. Checked (exe + decomp).
pub const DIM_CHECKED: &[Site] = &[Site {
    name: "scene colour setter (0x140232170)",
    rva: 0x0023_2170,
    expect: &[
        0x48, 0x63, 0x44, 0x24, 0x28, 0x48, 0xC1, 0xE0, 0x06, 0x48, 0x03, 0x05, 0x70, 0x96, 0x64, 0x00, 0x4D, 0x85, 0xC9, 0x74, 0x09, 0x41, 0x0F, 0x28, 0x01, 0x66, 0x0F, 0x7F, 0x40, 0x20, 0x0F, 0x28, 0x40, 0x20, 0x66, 0x0F, 0x7F, 0x00, 0x0F, 0x57, 0xC0, 0x0F, 0x2F, 0xC1, 0xC7, 0x40, 0x34, 0x00, 0x00, 0x00, 0x00,
        0x72, 0x21, 0xC7, 0x40, 0x38, 0x00, 0x00, 0x80, 0x3F, 0x41, 0x0F, 0x28, 0x00, 0x66, 0x0F, 0x7F, 0x40, 0x10, 0x41, 0x0F, 0x28, 0x08, 0x83, 0x48, 0x3C, 0x01, 0x66, 0x0F, 0x7F, 0x48, 0x20, 0x89, 0x48, 0x30, 0xC3,
    ],
}];

/// The Wizard and the Aquatank untouchable during their room-dimming moves (owner, 2026-11-03;
/// `crate::dim::NORMAL`): read, never written. Each class's vtable holds the dimming begin and Re:CoM's
/// own "hurt bodies off" / attack-state end (both hooked: `win::attack::hurt_off`,
/// `win::revenge::attack_end`). Checked (exe; RTTI checked at install).
pub const DIM_NORMAL_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyWizard +0x148 (state 4 begin)", rva: 0x0055_4BA0, target: 0x0019_5590 },
    PtrSite { name: "CEnemyWizard +0x438 (hurt bodies off)", rva: 0x0055_4E90, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyWizard +0x440 (attack-state end)", rva: 0x0055_4E98, target: 0x0017_DA90 },
    PtrSite { name: "CEnemyAquatank +0x130 (state 3 begin)", rva: 0x0053_3118, target: 0x0018_AA70 },
    PtrSite { name: "CEnemyAquatank +0x438 (hurt bodies off)", rva: 0x0053_3420, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyAquatank +0x440 (attack-state end)", rva: 0x0053_3428, target: 0x0017_DA90 },
];

/// The two dimming begins and Re:CoM's begin wrappers of states 3 / 4 (read, not hooked). The
/// begins (whole): motion play `0x1401A3750`, then `movaps xmm0,[0x1405094E0]` (0.5, 0.5, 0.5,
/// 1.0), `movss xmm1,[0x1406C120C]` (30.0), `mov ecx,0x1800`, `[rsp+0x20]` = 0 (slot 0), `call
/// 0x140232170`. The wrappers: `call [rax+0x130]` / `[rax+0x148]` (the class begin), then `jmp
/// [rax+0x438]` (hurt bodies off): the hurt-off hook runs right after the begin's request. Every
/// displacement is RIP-relative (no absolute pointer). Checked (exe + decomp).
pub const DIM_NORMAL_CHECKED: &[Site] = &[
    Site {
        name: "CEnemyWizard state 4 begin (0x140195590)",
        rva: 0x0019_5590,
        expect: &[
            0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x60, 0x48, 0x8B, 0x05, 0x27, 0xD5, 0x6C, 0x00, 0x48, 0x33, 0xC4, 0x48, 0x89, 0x44, 0x24, 0x50, 0x48, 0x8B, 0x15, 0x68, 0xDF, 0x62, 0x00, 0x33, 0xFF, 0x89, 0x7C, 0x24, 0x30, 0x45, 0x33, 0xC9, 0x89, 0x7C, 0x24, 0x28, 0x45, 0x33, 0xC0, 0xC7, 0x44,
            0x24, 0x20, 0xFF, 0xFF, 0xFF, 0xFF, 0x48, 0x8B, 0xD9, 0xE8, 0x80, 0xE1, 0x00, 0x00, 0x0F, 0x28, 0x05, 0x09, 0x3F, 0x37, 0x00, 0x4C, 0x8D, 0x44, 0x24, 0x40, 0xF3, 0x0F, 0x10, 0x0D, 0x28, 0xBC, 0x52, 0x00, 0x45, 0x33, 0xC9, 0xB9, 0x00, 0x18, 0x00, 0x00, 0x0F, 0x29, 0x44, 0x24, 0x40, 0x89, 0xBB, 0xB0,
            0x0C, 0x00, 0x00, 0x89, 0x7C, 0x24, 0x20, 0xE8, 0x70, 0xCB, 0x09, 0x00, 0x48, 0x8B, 0x4C, 0x24, 0x50, 0x48, 0x33, 0xCC, 0xE8, 0x23, 0x78, 0x31, 0x00, 0x48, 0x8B, 0x5C, 0x24, 0x78, 0x48, 0x83, 0xC4, 0x60, 0x5F, 0xC3,
        ],
    },
    Site {
        name: "CEnemyAquatank state 3 begin (0x14018AA70)",
        rva: 0x0018_AA70,
        expect: &[
            0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x60, 0x48, 0x8B, 0x05, 0x47, 0x80, 0x6D, 0x00, 0x48, 0x33, 0xC4, 0x48, 0x89, 0x44, 0x24, 0x50, 0x48, 0x8B, 0x15, 0xB0, 0x82, 0x63, 0x00, 0x33, 0xFF, 0x89, 0x7C, 0x24, 0x30, 0x45, 0x33, 0xC9, 0x89, 0x7C, 0x24, 0x28, 0x45, 0x33, 0xC0, 0xC7, 0x44,
            0x24, 0x20, 0xFF, 0xFF, 0xFF, 0xFF, 0x48, 0x8B, 0xD9, 0xE8, 0xA0, 0x8C, 0x01, 0x00, 0x0F, 0x28, 0x05, 0x29, 0xEA, 0x37, 0x00, 0x4C, 0x8D, 0x44, 0x24, 0x40, 0xF3, 0x0F, 0x10, 0x0D, 0x48, 0x67, 0x53, 0x00, 0x45, 0x33, 0xC9, 0xB9, 0x00, 0x18, 0x00, 0x00, 0x0F, 0x29, 0x44, 0x24, 0x40, 0x89, 0xBB, 0xB0,
            0x0C, 0x00, 0x00, 0x89, 0x7C, 0x24, 0x20, 0xE8, 0x90, 0x76, 0x0A, 0x00, 0x48, 0x8B, 0x4C, 0x24, 0x50, 0x48, 0x33, 0xCC, 0xE8, 0x43, 0x23, 0x32, 0x00, 0x48, 0x8B, 0x5C, 0x24, 0x78, 0x48, 0x83, 0xC4, 0x60, 0x5F, 0xC3,
        ],
    },
    Site {
        name: "enemy state 3 begin wrapper (0x14017B760)",
        rva: 0x0017_B760,
        expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0x01, 0x48, 0x8B, 0xD9, 0xFF, 0x90, 0x30, 0x01, 0x00, 0x00, 0x48, 0x8B, 0x03, 0x48, 0x8B, 0xCB, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0x48, 0xFF, 0xA0, 0x38, 0x04, 0x00, 0x00],
    },
    Site {
        name: "enemy state 4 begin wrapper (0x14017B7A0)",
        rva: 0x0017_B7A0,
        expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0x01, 0x48, 0x8B, 0xD9, 0xFF, 0x90, 0x48, 0x01, 0x00, 0x00, 0x48, 0x8B, 0x03, 0x48, 0x8B, 0xCB, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0x48, 0xFF, 0xA0, 0x38, 0x04, 0x00, 0x00],
    },
];

/// Decision 35 (MP to full where Re:CoM fills HP). Moved bytes are whole, position-independent
/// instructions.
pub const MP_HOOKS: &[Site] = &[
    // push rbx; sub rsp,0x20; mov rbx,rcx
    Site { name: "HP refill state step (0x1401AC9D0)", rva: 0x001A_C9D0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9] },
    // mov rax,rsp; push rbp; push r14; push r15; lea rbp,[rax-0x5F]
    Site { name: "level-up screen (0x14010EF10)", rva: 0x0010_EF10, expect: &[0x48, 0x8B, 0xC4, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8D, 0x68, 0xA1] },
    // KH1's save point refill (owner, 2026-11-01: "standing on a save point should refill MP"):
    // `CGimmickSavePoint` update (vtable slot 0x140584768). push rdi; sub rsp,0x40;
    // cmp byte [rcx+0x2C3],1. Bytes read from the Steam exe.
    Site { name: "save point update (0x1401C5180)", rva: 0x001C_5180, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x80, 0xB9, 0xC3, 0x02, 0x00, 0x00, 0x01] },
];

/// The call `0x1402374C0(0)` in `0x140237CE0`'s HP-filling branch (retargeted, decision 35).
pub const MP_CHECKPOINT_CALL: Site = Site { name: "checkpoint call (0x140237F3A)", rva: 0x0023_7F3A, expect: &[0xE8, 0x81, 0xF5, 0xFF, 0xFF] };

/// Its context, checked only: `xor ecx,ecx; call 0x1401B1410; xor ecx,ecx` before it and the HP
/// fill `mov rcx,[0x14087CBF8]; mov eax,[rcx+4]; mov [rcx],eax` after it.
pub const MP_CHECKED: &[Site] = &[
    Site { name: "checkpoint snapshot call (0x140237F31)", rva: 0x0023_7F31, expect: &[0x33, 0xC9, 0xE8, 0xD8, 0x94, 0xF7, 0xFF, 0x33, 0xC9] },
    Site { name: "checkpoint HP fill (0x140237F3F)", rva: 0x0023_7F3F, expect: &[0x48, 0x8B, 0x0D, 0xB2, 0x4C, 0x64, 0x00, 0x8B, 0x41, 0x04, 0x89, 0x01] },
];

/// Re:CoM's battle end (`crate::battle::end`, `docs/recom-m6-battles.md` § Battle end): read
/// for the log and the trace, and the win-action entry the KH1 frame yields to. The watch calls
/// three pure getters already checked by earlier stages: the system flag get `0x1401B0870`
/// (`M6_DECK_CALLEES`), the system variable get `0x1401B08C0` (`M6_DECK_CALLEES`) and the
/// sequence-ended test `0x1402480C0` (`KEYBLADE_CALLEES`).
///
/// Re:CoM code whose fields and globals the battle-end watch reads, or whose behaviour the KH1
/// frame's yield relies on. Checked only.
pub const BATTLE_END_CHECKED: &[Site] = &[
    // The manager's step: mov rbx,rcx; mov eax,[rcx+0x58]; dec eax; cmp eax,7 (states 1..8).
    Site { name: "battle manager step (0x1401871D0)", rva: 0x0018_71D0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x48, 0x8B, 0xD9, 0x8B, 0x41, 0x58, 0xFF, 0xC8, 0x83, 0xF8, 0x07] },
    // State 1: cmp [rbx+0x50],0; jg; cmp [rbx+0x98],0; jg; call (the room).
    Site { name: "battle manager state 1 counts (0x140187215)", rva: 0x0018_7215, expect: &[0x83, 0x7B, 0x50, 0x00, 0x0F, 0x8F, 0xC7, 0x01, 0x00, 0x00, 0x83, 0xBB, 0x98, 0x00, 0x00, 0x00, 0x00, 0x0F, 0x8F, 0xBA, 0x01, 0x00, 0x00, 0xE8] },
    // State 2: mov eax,[rbx+0x14]; sub [rbx+0x5C],eax; cmp [rbx+0x5C],0; jg.
    Site { name: "battle manager state 2 timer (0x14018728A)", rva: 0x0018_728A, expect: &[0x8B, 0x43, 0x14, 0x29, 0x43, 0x5C, 0x83, 0x7B, 0x5C, 0x00, 0x0F, 0x8F, 0x4C, 0x01, 0x00, 0x00] },
    // mov rax,[0x14087B690]; test; je; cmp [rax+0x94],0: the prize display busy.
    Site { name: "prize display busy (0x140209830)", rva: 0x0020_9830, expect: &[0x48, 0x8B, 0x05, 0x59, 0x1E, 0x67, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x0F, 0x83, 0xB8, 0x94, 0x00, 0x00, 0x00, 0x00] },
    // mov eax,[0x14087B808]; ret: the gauge pause.
    Site { name: "gauge pause getter (0x140232050)", rva: 0x0023_2050, expect: &[0x8B, 0x05, 0xB2, 0x97, 0x64, 0x00, 0xC3] },
    // Sora's win pose done: [0x14087B390]; +0x7B0 = 0, +0x40C = 0x1C, +0x414 > 2.
    Site { name: "win pose done (0x1401F3C90)", rva: 0x001F_3C90, expect: &[0x48, 0x8B, 0x05, 0xF9, 0x76, 0x68, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x1E, 0x83, 0xB8, 0xB0, 0x07, 0x00, 0x00, 0x00, 0x75, 0x12, 0x83, 0xB8, 0x0C, 0x04, 0x00, 0x00, 0x1C, 0x75, 0x09, 0x83, 0xB8, 0x14, 0x04, 0x00, 0x00, 0x02, 0x7F, 0x03] },
    // The last enemy: unless dead or in action 0x17, `player+0x428 |= 4`, `+0x420 |= 0x2000`.
    Site { name: "win pending (0x1401F4A60)", rva: 0x001F_4A60, expect: &[0x48, 0x83, 0xEC, 0x28, 0x48, 0x8B, 0x05, 0x25, 0x69, 0x68, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x36, 0x83, 0xB8, 0xB0, 0x07, 0x00, 0x00, 0x00, 0x75, 0x2D, 0x83, 0xB8, 0x0C, 0x04, 0x00, 0x00, 0x17, 0x74, 0x24, 0xE8, 0xF9, 0x9B, 0x01, 0x00, 0x48, 0x8B, 0x05, 0x02, 0x69, 0x68, 0x00, 0x83, 0x88, 0x28, 0x04, 0x00, 0x00, 0x04] },
    // The end action's win branch: test byte [rbx+0x428],4; je; mov rcx,rbx; call 0x1401F1B20.
    Site { name: "end action: win branch (0x1401F6794)", rva: 0x001F_6794, expect: &[0xF6, 0x83, 0x28, 0x04, 0x00, 0x00, 0x04, 0x74, 0x0D, 0x48, 0x8B, 0xCB, 0xE8, 0x7B, 0xB3, 0xFF, 0xFF] },
    // The win action's entry: prev = action; action = 0x1C (8-byte write: +0x410 = 0).
    Site { name: "win action entry (0x1401F1B20)", rva: 0x001F_1B20, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x81, 0x0C, 0x04, 0x00, 0x00, 0x33, 0xFF, 0xF7, 0x81, 0x28, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x48, 0x8B, 0xD9, 0x89, 0x81, 0x08, 0x04, 0x00, 0x00, 0x48, 0xC7, 0x81, 0x0C, 0x04, 0x00, 0x00, 0x1C, 0x00, 0x00, 0x00] },
];

/// A pointer in Re:CoM's data that holds another address of the image. The exe is relocated
/// (ASLR, `DYNAMIC_BASE`), so these are checked at run time against `base + target`, not as
/// bytes in the startup report.
pub struct PtrSite {
    pub name: &'static str,
    pub rva: u32,
    pub target: u32,
}

/// The pointers the model code checks before it patches or calls through them. Checked (exe,
/// with its relocation table: all four are relocated).
pub const MODEL_PTRS: &[PtrSite] = &[
    PtrSite { name: "CPlayerWeapon vtable +0x30 (0x1405AADE0)", rva: 0x005A_ADE0, target: 0x001A_4E70 },
    PtrSite { name: "CPlayerWeapon vtable +0 (0x1405AADB0)", rva: 0x005A_ADB0, target: 0x0022_4EC0 },
    PtrSite { name: "CPlayerSora vtable +0x100 (0x1405A9A38)", rva: 0x005A_9A38, target: 0x001F_5480 },
    PtrSite { name: "PLAYER resource table pointer (0x1407EBF28)", rva: 0x007E_BF28, target: 0x0080_55D0 },
];

/// The MP bar (M7 stage 7.2, `win::mp_bar`, `docs/recom-m7-mp-bar.md`): three wrappers on
/// `Gauge::CPlayerGauge` (vtable `0x1405A6638`). Same rule as [`HOOKS`]. Checked (exe + decomp): the
/// init's only caller is the ctor `0x140206E50`; the second-bar state `0x14020C190` is called by
/// the init, the update `0x14020A0E0` (states 7 / 1: Re:CoM's HUD hide / show), `0x140209440` and
/// `0x14020BC60`; the deleting dtor is reached only through the vtable (slot 0).
pub const MP_BAR_HOOKS: &[Site] = &[
    // mov rax,rsp; push rsi; push rdi; push r14; sub rsp,0x70 (rax is the entry rsp: the next
    // instruction `mov [rax-0x50],-2` runs from Re:CoM's code on the same stack)
    Site { name: "CPlayerGauge init (0x140208360)", rva: 0x0020_8360, expect: &[0x48, 0x8B, 0xC4, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x70] },
    // push rbp; push rdi; sub rsp,0x48
    Site { name: "CPlayerGauge second-bar state (0x14020C190)", rva: 0x0020_C190, expect: &[0x40, 0x55, 0x57, 0x48, 0x83, 0xEC, 0x48] },
    // push rdi; sub rsp,0x30
    Site { name: "CPlayerGauge deleting dtor (0x140207770)", rva: 0x0020_7770, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x30] },
];

/// The KH1 HUD (decision 39, H2): Re:CoM's player-gauge HP setters, wrapped for HP events.
/// Bytes read from the Steam exe (2026-10-31).
pub const KH1_HUD_HOOKS: &[Site] = &[
    // mov [rsp+0x10],rbx; push rdi; sub rsp,0x40
    Site { name: "CPlayerGauge HP (0x14020D6A0)", rva: 0x0020_D6A0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x40] },
    // xor r8d,r8d; movzx eax,dx; test edx,edx (the cmovs after it reads the flags: a jump keeps them)
    Site { name: "CPlayerGauge max HP (0x14020D8E0)", rva: 0x0020_D8E0, expect: &[0x45, 0x33, 0xC0, 0x0F, 0xB7, 0xC2, 0x85, 0xD2] },
];

/// The KH1 HUD (decision 39, H6): Re:CoM's friends (`Friend::CFriend`) as they come and go,
/// wrapped for Donald's and Goofy's gauges. Bytes read from the Steam exe (2026-11-01).
pub const KH1_HUD_PARTY_HOOKS: &[Site] = &[
    // push rbx; sub rsp,0x40; mov rbx,rcx ("CFriendDonald:: beginStateAppear()")
    Site { name: "CFriendDonald appear (0x1401DDC20)", rva: 0x001D_DC20, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40, 0x48, 0x8B, 0xD9] },
    // mov [rsp+8],rbx; push rdi; sub rsp,0x40; mov rbx,rcx ("CFriendGoofy:: beginStateAppear()")
    Site { name: "CFriendGoofy appear (0x1401DF9C0)", rva: 0x001D_F9C0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x48, 0x8B, 0xD9] },
    // mov [rsp+8],rbx; mov [rsp+0x10],rsi; push rdi; sub rsp,0x40 ("CFriend:: beginStateExit()")
    Site { name: "CFriend exit (0x1401D94D0)", rva: 0x001D_94D0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x40] },
    // push rbx; sub rsp,0x30; mov qword [rsp+0x20],-2 (the base dtor every friend's runs, "out")
    Site { name: "CFriend dtor (0x1401D90F0)", rva: 0x001D_90F0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF] },
];

/// The KH1 HUD (decision 39, H7): Re:CoM's lock-on marker (`Gauge::CLockonMarker`) update, wrapped
/// so KH1's cursor takes its place at `hud = kh1`. Bytes read from the Steam exe (2026-11-01).
pub const KH1_HUD_CURSOR_HOOKS: &[Site] = &[
    // push rdi; sub rsp,0x80 (the next instruction is rip-relative: not moved)
    Site { name: "CLockonMarker update (0x140209D30)", rva: 0x0020_9D30, expect: &[0x40, 0x57, 0x48, 0x81, 0xEC, 0x80, 0x00, 0x00, 0x00] },
];

/// The KH1 HUD (decision 39, H8): Re:CoM's actor popups (`CCommonGauge`, `0x14020C9A0(actor,
/// kind)`), wrapped for KH1's LEVEL UP!! box. Bytes read from the Steam exe (2026-11-01).
pub const KH1_HUD_POPUP_HOOKS: &[Site] = &[
    // mov [rsp+0x10],rbx; push rsi; sub rsp,0x20 (the next instruction is rip-relative)
    Site { name: "CCommonGauge popup (0x14020C9A0)", rva: 0x0020_C9A0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x56, 0x48, 0x83, 0xEC, 0x20] },
];

/// The KH1 HUD (owner, 2026-11-01: "the bottom right of the screen shows the type of room. can we
/// move this to the top right of the screen?"): `Gauge::CNaviMap`'s room plate set-up
/// `0x1401E44A0(navimap, area set, group)` (the room card's plate and name on its controls
/// `+0x128` [0] and [1], each placed at layout (0, 0)), called from the minimap's update
/// `0x1401E4700` when a room's plate appears (group 4) and settles (group 5). Wrapped so the
/// plate's controls go under the minimap at `hud_room_plate_top_right`. Bytes read from the
/// Steam exe (2026-11-01).
pub const KH1_HUD_PLATE_HOOKS: &[Site] = &[
    // push rbp; push rsi; push rdi; push r14; push r15; sub rsp,0x80
    Site { name: "CNaviMap room plate (0x1401E44A0)", rva: 0x001E_44A0, expect: &[0x40, 0x55, 0x56, 0x57, 0x41, 0x56, 0x41, 0x57, 0x48, 0x81, 0xEC, 0x80, 0x00, 0x00, 0x00] },
];

/// The KH1 HUD (decision 39, H3): Re:CoM's HUD-hide mask reads (`call 0x1401B08C0`, ecx = 1,
/// edx = 0xE2) of the enemy gauge (`CEnemyGauge` update `0x1402099B0`, bit 0x10) and the minimap
/// (`CNaviMap` `0x1401E4700` ×3, `0x1401E43E0`, bit 8), retargeted to our getter, which adds the
/// bit while the KH1 HUD is up so each runs its own hide path. Bytes read from the Steam exe.
pub const KH1_HUD_MASK_CALLS: &[Site] = &[
    Site { name: "enemy gauge mask read (0x1402099CE)", rva: 0x0020_99CE, expect: &[0xE8, 0xED, 0x6E, 0xFA, 0xFF] },
    Site { name: "minimap mask read 1 (0x1401E4767)", rva: 0x001E_4767, expect: &[0xE8, 0x54, 0xC1, 0xFC, 0xFF] },
    Site { name: "minimap mask read 2 (0x1401E4805)", rva: 0x001E_4805, expect: &[0xE8, 0xB6, 0xC0, 0xFC, 0xFF] },
    Site { name: "minimap mask read 3 (0x1401E4958)", rva: 0x001E_4958, expect: &[0xE8, 0x63, 0xBF, 0xFC, 0xFF] },
    Site { name: "minimap mask read 4 (0x1401E440E)", rva: 0x001E_440E, expect: &[0xE8, 0xAD, 0xC4, 0xFC, 0xFF] },
];

/// Each read's context (mov edx,0xE2; ecx = 1; the call; the test after it), checked only.
pub const KH1_HUD_MASK_CHECKED: &[Site] = &[
    // mov edx,0xE2; lea ecx,[rax+1]; call; test al,0x10
    Site { name: "enemy gauge mask read context (0x1402099C6)", rva: 0x0020_99C6, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0x8D, 0x48, 0x01, 0xE8, 0xED, 0x6E, 0xFA, 0xFF, 0xA8, 0x10] },
    // mov edx,0xE2; mov ecx,1; call; mov ecx,[rdi+0x110]
    Site { name: "minimap mask read 1 context (0x1401E475D)", rva: 0x001E_475D, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0x54, 0xC1, 0xFC, 0xFF, 0x8B, 0x8F, 0x10, 0x01, 0x00, 0x00] },
    Site { name: "minimap mask read 2 context (0x1401E47FB)", rva: 0x001E_47FB, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0xB6, 0xC0, 0xFC, 0xFF, 0x8B, 0x8F, 0x10, 0x01, 0x00, 0x00] },
    // …; call; test al,8
    Site { name: "minimap mask read 3 context (0x1401E494E)", rva: 0x001E_494E, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0x63, 0xBF, 0xFC, 0xFF, 0xA8, 0x08] },
    // mov edx,0xE2; mov ebx,1; mov ecx,ebx; call; test al,8
    Site { name: "minimap mask read 4 context (0x1401E4402)", rva: 0x001E_4402, expect: &[0xBA, 0xE2, 0x00, 0x00, 0x00, 0xBB, 0x01, 0x00, 0x00, 0x00, 0x8B, 0xCB, 0xE8, 0xAD, 0xC4, 0xFC, 0xFF, 0xA8, 0x08] },
];

/// The KH1 HUD (decision 39) from H2b on: Re:CoM's `D2::C2DSeqCtrl` functions it calls besides the
/// MP bar's. Bytes read from the Steam exe (2026-11-01).
pub const KH1_HUD_CALLEES: &[Site] = &[
    // movzx eax,byte [rcx+0x16D]; cmp edx,eax; jae; mov eax,edx; movaps xmm1,xmm2; mov ecx,
    // [rcx+rax*4+0x144]; jmp 0x140246710: play layer `edx` from time `xmm2` (f32, units of 1/50
    // frame; the C2DSeqObj's vtable +0x30 `0x140249360` sets +0x38 = −1 and +0x30 = time).
    Site { name: "C2DSeqCtrl play from (0x140248550)", rva: 0x0024_8550, expect: &[0x0F, 0xB6, 0x81, 0x6D, 0x01, 0x00, 0x00, 0x3B, 0xD0, 0x73, 0x11, 0x8B, 0xC2, 0x0F, 0x28, 0xCA, 0x8B, 0x8C, 0x81, 0x44, 0x01, 0x00, 0x00, 0xE9, 0xA4, 0xE1, 0xFF, 0xFF, 0xC3] },
    // movzx eax,byte [rcx+0x16D]; cmp edx,eax; jb; xor eax,eax; ret; mov eax,edx; mov ecx,
    // [rcx+rax*4+0x144]; jmp 0x140246250: layer `edx`'s play time (the C2DSeqObj's +0x30, through
    // the set's vtable +0x68 `0x140246D60`).
    Site { name: "C2DSeqCtrl layer time (0x140248000)", rva: 0x0024_8000, expect: &[0x0F, 0xB6, 0x81, 0x6D, 0x01, 0x00, 0x00, 0x3B, 0xD0, 0x72, 0x03, 0x33, 0xC0, 0xC3, 0x8B, 0xC2, 0x8B, 0x8C, 0x81, 0x44, 0x01, 0x00, 0x00, 0xE9, 0x34, 0xE2, 0xFF, 0xFF] },
    // H7: movzbl eax,[rcx+0x16D]; cmp edx,eax; jl: a layer's visible bit (`+0x140`).
    Site { name: "C2DSeqCtrl layer visible? (0x140248090)", rva: 0x0024_8090, expect: &[0x0F, 0xB6, 0x81, 0x6D, 0x01, 0x00, 0x00, 0x3B, 0xD0, 0x7C, 0x03] },
    // H7: mov [rsp+8],rbx; push rdi; sub rsp,0x20; lea rax,[C2DSeqCtrl vftable]: the ctor with a
    // mode (2: `+0x13B` = 1, the lock-on marker's projected control, `0x140208120`).
    Site { name: "C2DSeqCtrl ctor with mode (0x1402471D0)", rva: 0x0024_71D0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8D, 0x05, 0x5F, 0x01, 0x3C, 0x00] },
    // H7: movaps xmm0,[rdx]; movdqa [rcx+0xF0],xmm0; mov dword [rcx+0xFC],1.0: the 3D anchor
    // Re:CoM projects the control to (the lock-on marker's own call, `0x140209D30`).
    Site { name: "C2DSeqCtrl 3D anchor (0x140248570)", rva: 0x0024_8570, expect: &[0x0F, 0x28, 0x02, 0x66, 0x0F, 0x7F, 0x81, 0xF0, 0x00, 0x00, 0x00, 0xC7, 0x81, 0xFC, 0x00, 0x00] },
    // Re:CoM's HP gauge at KH1's place: mov [rsp+8],rbx; mov [rsp+0x10],rbp; mov [rsp+0x18],rsi; mov [rsp+0x20],rdi; push r14;
    // sub rsp,0x30; movzx eax,byte [rcx+0x16D]: vtable +0x20, replace a layer (ctrl, layer, set,
    // group, x offset (stack, f32), flag (stack)); Re:CoM's HP setter swaps the gauge's disc and
    // face with it. Re:CoM's HP gauge at KH1's place puts the turned ring groups on layers 1..3.
    Site { name: "C2DSeqCtrl replace layer (0x1402482F0)", rva: 0x0024_82F0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x48, 0x89, 0x7C, 0x24, 0x20, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x30, 0x0F, 0xB6, 0x81, 0x6D, 0x01, 0x00, 0x00] },
];

/// The vtable slot Re:CoM's HP gauge at KH1's place calls through (`hud_recom_hp_kh1_place`).
/// Checked (exe; relocated pointer).
pub const KH1_HUD_PLACE_PTRS: &[PtrSite] = &[PtrSite { name: "C2DSeqCtrl vtable +0x20 (0x140607360)", rva: 0x0060_7360, target: 0x0024_82F0 }];

/// Re:CoM's `D2::C2DSeqCtrl` functions the MP bar calls (the ctor `0x140247260` and `operator
/// new` are already checked for the Keyblades page).
pub const MP_BAR_CALLEES: &[Site] = &[
    // movzx eax,byte [rcx+0x16D]; cmp edx,eax; jae: a layer's frame (× 50 units, `0x140245870`).
    Site { name: "C2DSeqCtrl frame (0x140247C20)", rva: 0x0024_7C20, expect: &[0x0F, 0xB6, 0x81, 0x6D, 0x01, 0x00, 0x00, 0x3B, 0xD0, 0x73, 0x11] },
    Site { name: "C2DSeqCtrl clear layers (0x140247C90)", rva: 0x0024_7C90, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57] },
    Site { name: "C2DSeqCtrl all layers visible (0x140247EC0)", rva: 0x0024_7EC0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x48, 0x89, 0x6C, 0x24, 0x20, 0x56, 0x48, 0x83, 0xEC, 0x20] },
    // lea rax,[rdx+0x10]; mov [rcx+0x130],rax; ret: the parent's transform.
    Site { name: "C2DSeqCtrl parent (0x140248650)", rva: 0x0024_8650, expect: &[0x48, 0x8D, 0x42, 0x10, 0x48, 0x89, 0x81, 0x30, 0x01, 0x00, 0x00, 0xC3] },
    // movaps xmm0,[rdx]; movdqa [rcx+0x110],xmm0: the scale (16-byte aligned argument).
    Site { name: "C2DSeqCtrl scale (0x1402486D0)", rva: 0x0024_86D0, expect: &[0x0F, 0x28, 0x02, 0x66, 0x0F, 0x7F, 0x81, 0x10, 0x01, 0x00, 0x00] },
    // vtable +8: add a layer (ctrl, set, group, x offset, flag); 10 at most (`cmp [rcx+0x16D],0xA`).
    Site { name: "C2DSeqCtrl add layer (0x140247850)", rva: 0x0024_7850, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x80, 0xB9, 0x6D, 0x01, 0x00, 0x00, 0x0A] },
    // vtable +0: the (vector) deleting dtor; flag 1 = destroy + delete.
    Site { name: "C2DSeqCtrl deleting dtor (0x14015D230)", rva: 0x0015_D230, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF] },
];

/// Re:CoM code the MP bar's set check relies on (not called): the 2D registry walk and the
/// SEQD's group count.
pub const MP_BAR_CHECKED: &[Site] = &[
    // 0x140245A20: `cmp [0x14087CE38],0` (the 2D manager), then `mov rax,[0x14087CE48]`, the
    // first node `[rax+8]`, the next node `[node+8]`.
    Site {
        name: "2D set lookup (0x140245A20)",
        rva: 0x0024_5A20,
        expect: &[
            0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0x83, 0x3D, 0x01, 0x74, 0x63, 0x00, 0x00, 0x49, 0x8B, 0xF8, 0x8B, 0xF2, 0x48, 0x8B, 0xD9, 0x75, 0x12, 0x33, 0xC0, 0x48, 0x8B, 0x5C, 0x24, 0x40, 0x48, 0x8B, 0x74,
            0x24, 0x48, 0x48, 0x83, 0xC4, 0x30, 0x5F, 0xC3, 0x48, 0x8B, 0x05, 0xEE, 0x73, 0x63, 0x00, 0x45, 0x33, 0xDB, 0x4C, 0x8B, 0x50, 0x08, 0x4D, 0x85, 0xD2, 0x74, 0x06, 0x4D, 0x8B, 0x42, 0x08,
        ],
    },
    // lea rax,[r10+0x14]: the node's name.
    Site { name: "2D set lookup: name at +0x14 (0x140245A80)", rva: 0x0024_5A80, expect: &[0x49, 0x8D, 0x42, 0x14, 0x4C, 0x8B, 0xCB] },
    // C2DSeq load: +0xE0 animations, +0xE8 groups, +0xF0 / +0xF4 their counts (SEQD +0x20 / +0x28).
    Site {
        name: "C2DSeq load: group count +0xF4 (0x140246DFE)",
        rva: 0x0024_6DFE,
        expect: &[
            0x8B, 0x43, 0x24, 0x48, 0x03, 0xC3, 0x48, 0x89, 0x87, 0xE0, 0x00, 0x00, 0x00, 0x8B, 0x43, 0x2C, 0x48, 0x03, 0xC3, 0x48, 0x89, 0x87, 0xE8, 0x00, 0x00, 0x00, 0x8B, 0x43, 0x20, 0x89, 0x87, 0xF0, 0x00, 0x00, 0x00, 0x8B, 0x43, 0x28, 0x89, 0x87, 0xF4, 0x00, 0x00, 0x00,
        ],
    },
    // C2DSeq vtable +0x10, the layer instantiation (its start; a group index at or past +0xF4 adds
    // nothing: the check below).
    Site { name: "C2DSeq instantiate (0x140246A40)", rva: 0x0024_6A40, expect: &[0x48, 0x8B, 0xC4, 0x57, 0x48, 0x81, 0xEC, 0x80, 0x00, 0x00, 0x00, 0x48, 0xC7, 0x40, 0xD8, 0xFE, 0xFF, 0xFF, 0xFF] },
    // mov rdi,rcx; cmp edx,[rcx+0xF4]; jb
    Site { name: "C2DSeq instantiate: group bound (0x140246A6B)", rva: 0x0024_6A6B, expect: &[0x48, 0x8B, 0xF9, 0x3B, 0x91, 0xF4, 0x00, 0x00, 0x00, 0x72, 0x07] },
];

/// The vtable slots the MP bar calls through or compares. Checked (exe; relocated pointers).
pub const MP_BAR_PTRS: &[PtrSite] = &[
    PtrSite { name: "C2DSeqCtrl vtable +0 (0x140607340)", rva: 0x0060_7340, target: 0x0015_D230 },
    PtrSite { name: "C2DSeqCtrl vtable +8 (0x140607348)", rva: 0x0060_7348, target: 0x0024_7850 },
    PtrSite { name: "C2DSeq vtable +0x10 (0x140607270)", rva: 0x0060_7270, target: 0x0024_6A40 },
];

/// The command menu (M7 stage 7.3, `win::command_menu`, `docs/recom-m7-command-menu.md`): no hook
/// of its own (it runs from the MP bar's gauge wrappers and the request-bits wrapper). Re:CoM's
/// `C2DSeqCtrl` functions it calls beyond the MP bar's and the Keyblades page's.
pub const COMMAND_MENU_CALLEES: &[Site] = &[
    // movaps xmm0,[rdx]; movdqa [rcx+0x120],xmm0; ret: the control's colour (RGBA floats, 1.0 =
    // ×1), used when `+0x138` is set (`0x14024BB10`). Re:CoM's camp and HUD call it.
    Site { name: "C2DSeqCtrl colour (0x140248590)", rva: 0x0024_8590, expect: &[0x0F, 0x28, 0x02, 0x66, 0x0F, 0x7F, 0x81, 0x20, 0x01, 0x00, 0x00, 0xC3] },
    // vtable +0x60: add a text item (ctrl, set, item, x, y, screen x offset); `CStockFolder`
    // (`0x1401658F0`) puts its labels on `Cockpit:00` plates with it.
    Site { name: "C2DSeqCtrl add text (0x1402476F0)", rva: 0x0024_76F0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x48, 0x8B, 0x01, 0x0F, 0x29, 0x74, 0x24, 0x20] },
];

/// Re:CoM code the context row's reading relies on (not called).
pub const COMMAND_MENU_CHECKED: &[Site] = &[
    // The field action pick `0x1401E80C0`: the object is player +0x5FC's handle when > 0, else
    // +0x84C's (mov esi,[rcx+0x5FC]; test; jg; mov esi,[rcx+0x84C]; test; jg).
    Site { name: "field action pick: handles (0x1401E80E8)", rva: 0x001E_80E8, expect: &[0x8B, 0xB1, 0xFC, 0x05, 0x00, 0x00, 0x85, 0xF6, 0x7F, 0x11, 0x8B, 0xB1, 0x4C, 0x08, 0x00, 0x00, 0x85, 0xF6, 0x7F, 0x07] },
    // CGimmickDoor's type getter (vtable 0x140572478 +0x28): 0x1006, or 0x1005 while
    // `0x1401B0870(1)` (sub rsp,0x28; mov edx,0x24; lea ecx,[rdx-0x23]).
    Site { name: "door type getter (0x1401B8A10)", rva: 0x001B_8A10, expect: &[0x48, 0x83, 0xEC, 0x28, 0xBA, 0x24, 0x00, 0x00, 0x00, 0x8D, 0x4A, 0xDD] },
    // The save and warp points (the tab's Save / Examine): their type getters (mov eax,0x2004 /
    // 0x2005; ret), CGimmickSavePoint vtable 0x1405846C8 +0x28, CGimmickWarpPoint 0x14058F7E0 +0x28.
    Site { name: "save point type getter (0x1401C50B0)", rva: 0x001C_50B0, expect: &[0xB8, 0x04, 0x20, 0x00, 0x00, 0xC3] },
    Site { name: "warp point type getter (0x1401CC1E0)", rva: 0x001C_C1E0, expect: &[0xB8, 0x05, 0x20, 0x00, 0x00, 0xC3] },
    // The save point's update 0x1401C5180, Sora in range (< 0.8, < 0.2 above): mov byte
    // [rdi+0x2C0],1; then the object of player +0x850 (unless 0 / −1) else +0x84C; call
    // [vtable+0x28]; cmp eax,0x2004.
    Site {
        name: "save point update: in range, handle pick (0x1401C52CC)",
        rva: 0x001C_52CC,
        expect: &[
            0xC6, 0x87, 0xC0, 0x02, 0x00, 0x00, 0x01, 0x48, 0x8B, 0x15, 0xA6, 0x60, 0x6B, 0x00, 0x8B, 0x8A, 0x50, 0x08, 0x00, 0x00, 0x8D, 0x41, 0x01, 0x83, 0xF8, 0x01, 0x77, 0x06, 0x8B, 0x8A, 0x4C, 0x08, 0x00, 0x00, 0xE8, 0x5D, 0x80,
            0x0E, 0x00, 0x48, 0x85, 0xC0, 0x0F, 0x84, 0x80, 0x00, 0x00, 0x00, 0x48, 0x8B, 0x10, 0x48, 0x8B, 0xC8, 0xFF, 0x52, 0x28, 0x3D, 0x04, 0x20, 0x00, 0x00,
        ],
    },
    // …after Triangle (0x14029B550): mov r8d,[player+0x40C]; dec; cmp 3; ja (action 1..4).
    Site { name: "save point update: action 1..4 (0x1401C531E)", rva: 0x001C_531E, expect: &[0x44, 0x8B, 0x80, 0x0C, 0x04, 0x00, 0x00, 0x41, 0xFF, 0xC8, 0x41, 0x83, 0xF8, 0x03, 0x77, 0x4E] },
    // Out of range: cmp byte [rdi+0x2C0],1; jne; and [rdi+0x154],~0x10000; …; mov byte [rdi+0x2C0],0.
    Site {
        name: "save point update: out of range (0x1401C5356)",
        rva: 0x001C_5356,
        expect: &[
            0x80, 0xBF, 0xC0, 0x02, 0x00, 0x00, 0x01, 0x75, 0x1D, 0x81, 0xA7, 0x54, 0x01, 0x00, 0x00, 0xFF, 0xFF, 0xFE, 0xFF, 0x48, 0x8B, 0x0D, 0x10, 0x60, 0x6B, 0x00, 0xE8, 0x1B, 0x16, 0x02, 0x00, 0xC6, 0x87, 0xC0, 0x02, 0x00, 0x00, 0x00,
        ],
    },
    // The warp point's update 0x1401CC280: call 0x1402345D0; jne (stopped); the same handle pick;
    // cmp eax,0x2005.
    Site {
        name: "warp point update: gate, handle pick (0x1401CC2D7)",
        rva: 0x001C_C2D7,
        expect: &[
            0xE8, 0xF4, 0x82, 0x06, 0x00, 0x85, 0xC0, 0x0F, 0x85, 0x9D, 0x00, 0x00, 0x00, 0x48, 0x8B, 0x15, 0x95, 0xF0, 0x6A, 0x00, 0x8B, 0x8A, 0x50, 0x08, 0x00, 0x00, 0x8D, 0x41, 0x01, 0x83, 0xF8, 0x01, 0x77, 0x06, 0x8B, 0x8A, 0x4C, 0x08,
            0x00, 0x00, 0xE8, 0x4C, 0x10, 0x0E, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x78, 0x48, 0x8B, 0x10, 0x48, 0x8B, 0xC8, 0xFF, 0x52, 0x28, 0x3D, 0x05, 0x20, 0x00, 0x00,
        ],
    },
    // …after Triangle: mov ecx,[player+0x40C]; dec; cmp 3; ja.
    Site { name: "warp point update: action 1..4 (0x1401CC32B)", rva: 0x001C_C32B, expect: &[0x8B, 0x88, 0x0C, 0x04, 0x00, 0x00, 0xFF, 0xC9, 0x83, 0xF9, 0x03, 0x77, 0x49] },
    // mov eax,[0x14087B84C]; and eax,1; ret: the warp point's stop flag.
    Site { name: "warp point stop flag (0x1402345D0)", rva: 0x0023_45D0, expect: &[0x8B, 0x05, 0x76, 0x72, 0x64, 0x00, 0x83, 0xE0, 0x01, 0xC3] },
    // Stage 7.12. The retargeted calls' context: jne (not a save / warp point); xor ecx,ecx (port
    // 0); call 0x14029B550; test eax,eax; je (no Triangle).
    Site { name: "save point update: Triangle read (0x1401C530A)", rva: 0x001C_530A, expect: &[0x75, 0x70, 0x33, 0xC9, 0xE8, 0x3D, 0x62, 0x0D, 0x00, 0x85, 0xC0, 0x74, 0x65] },
    Site { name: "warp point update: Triangle read (0x1401CC317)", rva: 0x001C_C317, expect: &[0x75, 0x68, 0x33, 0xC9, 0xE8, 0x30, 0xF2, 0x0C, 0x00, 0x85, 0xC0, 0x74, 0x5D] },
    // The callee, whole: sub rsp,0x28; mov edx,ecx; mov ecx,0x10; call 0x14029C240 (fresh);
    // xor ecx,ecx; test eax,eax; setne cl; mov eax,ecx; add rsp,0x28; ret (0 / 1 in eax). Its
    // callers: 0x140142A30 (not ours), the save and warp points.
    Site {
        name: "point Triangle read (0x14029B550)",
        rva: 0x0029_B550,
        expect: &[0x48, 0x83, 0xEC, 0x28, 0x8B, 0xD1, 0xB9, 0x10, 0x00, 0x00, 0x00, 0xE8, 0xE0, 0x0C, 0x00, 0x00, 0x33, 0xC9, 0x85, 0xC0, 0x0F, 0x95, 0xC1, 0x8B, 0xC1, 0x48, 0x83, 0xC4, 0x28, 0xC3],
    },
    // movzx eax,word [0x14083F3F0]; ret: Re:CoM's confirm key (PS2 order), the shortcut panel's
    // third glyph.
    Site { name: "confirm key getter (0x140346BB0)", rva: 0x0034_6BB0, expect: &[0x0F, 0xB7, 0x05, 0x39, 0x88, 0x4F, 0x00, 0xC3] },
];

/// Stage 7.12: the save and warp points' fresh-Triangle calls (`call 0x14029B550`), retargeted to
/// `win::command_menu::point_triangle` (the bytes the patch replaces; context in
/// [`COMMAND_MENU_CHECKED`]).
pub const COMMAND_MENU_POINT_CALLS: &[Site] = &[
    Site { name: "save point Triangle call (0x1401C530E)", rva: 0x001C_530E, expect: &[0xE8, 0x3D, 0x62, 0x0D, 0x00] },
    Site { name: "warp point Triangle call (0x1401CC31B)", rva: 0x001C_C31B, expect: &[0xE8, 0x30, 0xF2, 0x0C, 0x00] },
];

/// Our HUD controls' 2D instances (`win::d2`, `crate::d2`): Re:CoM's draw-list setter, called to
/// add and release our layers on the list they belong to.
pub const HUD_2D_CALLEES: &[Site] = &[
    // cmp ecx,3; jae; mov eax,ecx; lea rdx,[0x14087CE80]; mov [0x140820F94],ecx;
    // mov rax,[rdx+rax*8]; mov [0x14087CE70],rax; ret: the current list and its index.
    Site {
        name: "2D draw list set (0x140249800)",
        rva: 0x0024_9800,
        expect: &[
            0x83, 0xF9, 0x03, 0x73, 0x1A, 0x8B, 0xC1, 0x48, 0x8D, 0x15, 0x72, 0x36, 0x63, 0x00, 0x89, 0x0D, 0x80, 0x77, 0x5D, 0x00, 0x48, 0x8B, 0x04, 0xC2, 0x48, 0x89, 0x05, 0x51, 0x36, 0x63, 0x00, 0xC3,
        ],
    },
];

/// Re:CoM code `win::d2` relies on (not called): where a layer's pool entry is taken and
/// released (both on the current list), and the pool's fields it reads for the log.
pub const HUD_2D_CHECKED: &[Site] = &[
    // An instance's entry: cmp [rcx+0xC],0; …; mov rbx,[0x14087CE70]; mov rcx,[rbx+8];
    // call [vtable+8] (the pool's allocation) into the current list.
    Site {
        name: "2D instance register (0x140249770)",
        rva: 0x0024_9770,
        expect: &[
            0x40, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x83, 0x79, 0x0C, 0x00, 0x48, 0x8B, 0xF9, 0x74, 0x0B, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF, 0x48, 0x83, 0xC4, 0x20, 0x5F, 0xC3, 0x48, 0x89, 0x5C, 0x24, 0x30, 0x48, 0x8B, 0x1D, 0xDA, 0x36, 0x63, 0x00, 0x48, 0x8B,
            0x4B, 0x08, 0x48, 0x8B, 0x01, 0xFF, 0x50, 0x08,
        ],
    },
    // Its release: mov edx,[rcx+0xC]; …; mov rcx,[0x14087CE70]; call 0x140249E40 (the handle
    // looked for in the current list only).
    Site {
        name: "2D instance release (0x1402499C0)",
        rva: 0x0024_99C0,
        expect: &[
            0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x51, 0x0C, 0x48, 0x8B, 0xD9, 0x85, 0xD2, 0x75, 0x08, 0x33, 0xC0, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3, 0x48, 0x8B, 0x0D, 0x91, 0x34, 0x63, 0x00, 0xE8, 0x5C, 0x04, 0x00, 0x00,
        ],
    },
    // The list walk: mov r9d,edx; xor r8d,r8d; mov rdx,[rcx+0x10] (the list's first node).
    Site { name: "2D list release walk (0x140249E40)", rva: 0x0024_9E40, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x44, 0x8B, 0xCA, 0x45, 0x33, 0xC0, 0x48, 0x8B, 0x51, 0x10] },
    // mov rax,[0x14087CE58]; ret: the pool object.
    Site { name: "2D pool getter (0x140245F70)", rva: 0x0024_5F70, expect: &[0x48, 0x8B, 0x05, 0xE1, 0x6E, 0x63, 0x00, 0xC3] },
    // Its vtable +8: add rcx,8; jmp 0x140242560 (the pool at +8).
    Site { name: "2D pool allocate (0x140245B00)", rva: 0x0024_5B00, expect: &[0x48, 0x83, 0xC1, 0x08, 0xE9, 0x57, 0xCA, 0xFF, 0xFF] },
    // cmp [rcx+0x24],0 (free entries); mov rdx,rcx; jne; xor eax,eax; ret; mov eax,[rdx+0x1C].
    Site { name: "pool allocate: free count +0x24 (0x140242560)", rva: 0x0024_2560, expect: &[0x83, 0x79, 0x24, 0x00, 0x48, 0x8B, 0xD1, 0x75, 0x03, 0x33, 0xC0, 0xC3, 0x8B, 0x42, 0x1C] },
    // lea edx,[r9+0x20]; mov r8d,0x800; lea rcx,[rsi+8]; call 0x1402425E0: 2048 entries.
    Site { name: "2D manager ctor: pool of 0x800 (0x140245293)", rva: 0x0024_5293, expect: &[0x41, 0x8D, 0x51, 0x20, 0x41, 0xB8, 0x00, 0x08, 0x00, 0x00, 0x48, 0x8D, 0x4E, 0x08, 0xE8, 0x3A, 0xD3, 0xFF, 0xFF] },
];

/// KH1's items (M8 stage 8a.1, `win::items`, `docs/recom-m8-items.md`). Same rule as [`HOOKS`].
pub const ITEM_HOOKS: &[Site] = &[
    // mov [rsp+0x10],rbx; push rdi; sub rsp,0x60: exeItem (action 0xE's exe; the Battle exe calls it
    // directly at 0x1401F9A13, the Field exe through our new case).
    Site { name: "exeItem (0x1401ED6E0)", rva: 0x001E_D6E0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x60] },
];

/// Exe bytes the items patch.
pub const ITEM_DATA: &[Site] = &[
    // The Field exe's jump table (`0x1401FA5F4`), entry for action 0xE: the default `0x1401FA599`
    // (an RVA), then 0xF's (the default too, not changed).
    Site { name: "Field exe jump table, action 0xE", rva: 0x001F_A628, expect: &[0x99, 0xA5, 0x1F, 0x00, 0x99, 0xA5, 0x1F, 0x00] },
    // int3 padding after a `pop rdi; ret` (`0x1401FAEA0`), before `0x1401FAEB0`: the 13-byte stub.
    Site { name: "code cave 0x1401FAEA2", rva: 0x001F_AEA2, expect: &[0xCC; 14] },
];

/// Re:CoM code the items rely on or replicate (not patched).
pub const ITEM_CHECKED: &[Site] = &[
    // initItem: its log call, then prev action = action; action (8 bytes) = 0xE. The KH1 use does
    // these writes cardlessly (initItem reads BattleCardList, below).
    Site {
        name: "initItem (0x1401F28F0)",
        rva: 0x001F_28F0,
        expect: &[
            0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0xDA, 0x48, 0x8B, 0xF9, 0x44, 0x8B, 0xC2, 0xB9, 0x03, 0x00, 0x00, 0x00, 0x48, 0x8D, 0x15, 0x45, 0xDF, 0x3A, 0x00, 0xE8, 0xF8, 0xE0, 0x04, 0x00, 0x8B, 0x87,
            0x0C, 0x04, 0x00, 0x00, 0x33, 0xF6, 0x48, 0x8B, 0xCF, 0x89, 0x87, 0x08, 0x04, 0x00, 0x00, 0x48, 0xC7, 0x87, 0x0C, 0x04, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00,
        ],
    },
    // initItem's tail: phase 3, sub 1, then +0x65C = 0x1407C7830[BattleCardList[+0x7F4] + 8] (the
    // card read that keeps it from being called cardlessly).
    Site {
        name: "initItem card read (0x1401F29A3)",
        rva: 0x001F_29A3,
        expect: &[
            0xC7, 0x87, 0x04, 0x06, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xC7, 0x87, 0x14, 0x04, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x48, 0x8B, 0x05, 0x4A, 0x7E, 0x68, 0x00, 0x0F, 0xB6, 0x4C, 0xC8, 0x08, 0x48, 0x8D, 0x05, 0x66, 0x4E, 0x5D, 0x00, 0x8B, 0x04,
            0x88, 0x89, 0x87, 0x5C, 0x06, 0x00, 0x00,
        ],
    },
    // exeItem sub 2: the effects at the clip's frame 56 (cmp edx,0x38; jl), the body offset
    // (0x1405A0910: (0, 0.85, 0, 1)).
    Site { name: "exeItem frame 56 (0x1401ED7F8)", rva: 0x001E_D7F8, expect: &[0x83, 0xFA, 0x38, 0x0F, 0x8C, 0x0F, 0x02, 0x00, 0x00, 0x0F, 0x28, 0x05, 0x08, 0x31, 0x3B, 0x00, 0x0F, 0x29, 0x44, 0x24, 0x40] },
    // exeItem's switch on the card (+0x7F4 − 0x2A, 7 cases: the item cards Potion … Megalixir).
    Site {
        name: "exeItem effect switch (0x1401ED82E)",
        rva: 0x001E_D82E,
        expect: &[0x8B, 0x83, 0xF4, 0x07, 0x00, 0x00, 0x33, 0xFF, 0x83, 0xC0, 0xD6, 0x83, 0xF8, 0x06, 0x0F, 0x87, 0xD7, 0x00, 0x00, 0x00, 0x48, 0x8D, 0x15, 0xB7, 0x27, 0xE1, 0xFF, 0x8B, 0x8C, 0x82, 0x28, 0xDA, 0x1E, 0x00, 0x48, 0x03, 0xCA, 0xFF, 0xE1],
    },
    // exeItem's effect call: 0x1401A2D50(player, name, id, bone 0, &offset, 0, 0.0), then the
    // effect object's +0x4C |= 4 (kept on the player): what the KH1 use's heal does.
    Site {
        name: "exeItem effect call (0x1401ED8F4)",
        rva: 0x001E_D8F4,
        expect: &[
            0x0F, 0x57, 0xC0, 0x4C, 0x8B, 0xC8, 0xF3, 0x0F, 0x11, 0x44, 0x24, 0x30, 0x48, 0x8D, 0x44, 0x24, 0x40, 0x48, 0x89, 0x7C, 0x24, 0x28, 0x48, 0x8B, 0xCB, 0x48, 0x89, 0x44, 0x24, 0x20, 0xE8, 0x39, 0x54, 0xFB, 0xFF, 0x8B, 0xF8, 0x8B, 0xCF, 0xE8,
            0xD0, 0xA5, 0x06, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x04, 0x83, 0x48, 0x4C, 0x04,
        ],
    },
    // exeItem's card effect: the revolver (+0x3F0) reload 0x14016C910: never reached for a KH1 item.
    Site {
        name: "exeItem card reload (0x1401ED929)",
        rva: 0x001E_D929,
        expect: &[
            0x48, 0x8B, 0x8B, 0xF0, 0x03, 0x00, 0x00, 0x48, 0x85, 0xC9, 0x0F, 0x84, 0xD7, 0x00, 0x00, 0x00, 0x44, 0x8B, 0x83, 0x38, 0x08, 0x00, 0x00, 0x33, 0xD2, 0x45, 0x85, 0xC0, 0x7E, 0x0D, 0x41, 0xFF, 0xC8, 0xE8, 0xC1, 0xEF, 0xF7, 0xFF, 0xE9, 0xBC, 0x00,
            0x00, 0x00, 0x45, 0x33, 0xC0, 0xE8, 0xB4, 0xEF, 0xF7, 0xFF,
        ],
    },
    // The Battle exe's case 0xE: mov rcx,rdi; call exeItem; jmp to its end.
    Site { name: "Battle exe case 0xE (0x1401F9A10)", rva: 0x001F_9A10, expect: &[0x48, 0x8B, 0xCF, 0xE8, 0xC8, 0x3C, 0xFF, 0xFF, 0xE9, 0xAC, 0x02, 0x00, 0x00] },
];

/// Re:CoM functions the camp's Items page calls (M8 stage 8a.2, `win::items::use_in_menu`,
/// `docs/recom-m8-items.md` § Stage 8a.2). The heal `0x1401E59A0` is [`MAGIC_CALLEES`]'.
pub const ITEM_PAGE_CALLEES: &[Site] = &[
    // sub rsp,0x28; mov rax,[0x14087B390] (the current player); test rax,rax; jne: the player →
    // save stats copy the camp runs as it opens (`0x1400D9C80` state 2).
    Site { name: "player -> save stats (0x1401F0120)", rva: 0x001F_0120, expect: &[0x48, 0x83, 0xEC, 0x28, 0x48, 0x8B, 0x05, 0x65, 0xB2, 0x68, 0x00, 0x48, 0x85, 0xC0, 0x75, 0x05] },
];

/// Enemy Skills stage 2 (`win::enemy_skills`, `crate::skill_effects`, `docs/recom-enemy-skills.md`
/// § Stage 2 (built)): the Sora-side clear `CBTL::resetNowEnchant` `0x140161E10(CBTL, side)`, wrapped
/// to put the equipped skills back after Re:CoM resets Sora's fields. Same rule as [`HOOKS`]: `mov
/// [rsp+8],rbx; mov [rsp+0x10],rsi; push rdi; sub rsp,0x20` (15 bytes). Checked (exe): only direct
/// calls (10), no branch into the moved bytes.
pub const ENEMY_SKILL_HOOKS: &[Site] = &[Site {
    name: "CBTL clear resetNowEnchant (0x140161E10)",
    rva: 0x0016_1E10,
    expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20],
}];

/// Enemy Skills stage 2: the Re:CoM functions the skills call (only called, never hooked).
pub const ENEMY_SKILL_CALLEES: &[Site] = &[
    // Gargoyle's Vanish setter (player, on): `+0x420 |= 0x40`, the pulse from 0.
    Site { name: "Vanish setter (0x1401F7EC0)", rva: 0x001F_7EC0, expect: &[0x48, 0x83, 0xEC, 0x58, 0x48, 0x8B, 0x05, 0xFD, 0xAB, 0x66, 0x00, 0x48, 0x33, 0xC4, 0x48, 0x89] },
    // Xaldin's Aero Guard setter (player, on): the wind object, `+0x420 |= 0x80000000`.
    Site { name: "Aero Guard setter (0x1401F6FB0)", rva: 0x001F_6FB0, expect: &[0x40, 0x57, 0x48, 0x81, 0xEC, 0x20, 0x01, 0x00, 0x00, 0x48, 0x8B, 0x05, 0x08, 0xBB, 0x66, 0x00] },
    // Re:CoM's player motion speed (player, speed): `+0x47C`, its motion and the weapon's.
    Site { name: "player motion speed (0x1401F74D0)", rva: 0x001F_74D0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0xF3, 0x0F, 0x10, 0x81, 0x7C, 0x04, 0x00, 0x00, 0x48, 0x8B] },
    // The barrier test (attack, body): the front test of Large Body's Guard.
    Site { name: "barrier test (0x14016EED0)", rva: 0x0016_EED0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x70, 0x48, 0x8B, 0x05, 0xE7, 0x3B, 0x6F] },
    // Large Body's Guard setter (player, on), whole: `+0x420 |= 0x200`, `+0x72C |= 2`, `+0x718 |= 1`,
    // `+0x73C` = π/2 (0x3FC90FDB).
    Site {
        name: "Large Body barrier setter (0x1401F8FF0)",
        rva: 0x001F_8FF0,
        expect: &[
            0x8B, 0x81, 0x20, 0x04, 0x00, 0x00, 0x85, 0xD2, 0x75, 0x18, 0x0F, 0xBA, 0xF0, 0x09, 0x89, 0x81, 0x20, 0x04, 0x00, 0x00, 0x83, 0xA1, 0x2C, 0x07, 0x00, 0x00, 0xFD, 0x89, 0x91, 0x18, 0x07, 0x00, 0x00, 0xC3, 0x0F, 0xBA, 0xE8, 0x09, 0x89, 0x81,
            0x20, 0x04, 0x00, 0x00, 0x83, 0x89, 0x2C, 0x07, 0x00, 0x00, 0x02, 0x83, 0x89, 0x18, 0x07, 0x00, 0x00, 0x01, 0xC7, 0x81, 0x3C, 0x07, 0x00, 0x00, 0xDB, 0x0F, 0xC9, 0x3F, 0xC3,
        ],
    },
];

/// Enemy Skills stage 2: Re:CoM's player-side apply `0x140163C00`, the writes the skills repeat
/// (read only): Fat Bandit / Guard Armor / Card Soldier's `+0x420` bits, Bouncywild's Draw (`+0x54C` =
/// 30.0, `+0x550` = 10.0), Xemnas', Saïx's and Roxas' bits after their affinity calls.
pub const ENEMY_SKILL_CHECKED: &[Site] = &[
    Site {
        name: "apply: Fat Bandit, Guard Armor, Card Soldier bits (0x140163CED)",
        rva: 0x0016_3CED,
        expect: &[
            0x81, 0x89, 0x20, 0x04, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x48, 0x83, 0xC4, 0x28, 0xC3, 0x81, 0x89, 0x20, 0x04, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC4, 0x28, 0xC3, 0x81, 0x89, 0x20, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x48, 0x83, 0xC4, 0x28, 0xC3,
        ],
    },
    Site { name: "apply: Bouncywild's Draw (0x140163D44)", rva: 0x0016_3D44, expect: &[0xC7, 0x81, 0x4C, 0x05, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x41, 0xC7, 0x81, 0x50, 0x05, 0x00, 0x00, 0x00, 0x00, 0x20, 0x41, 0x48, 0x83, 0xC4, 0x28, 0xC3] },
    Site { name: "apply: Xemnas' bit (0x140163D9C)", rva: 0x0016_3D9C, expect: &[0x81, 0x88, 0x20, 0x04, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00] },
    Site { name: "apply: Saix's bit (0x140163DEF)", rva: 0x0016_3DEF, expect: &[0x81, 0x88, 0x20, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40] },
    Site { name: "apply: Roxas' bit (0x140163E31)", rva: 0x0016_3E31, expect: &[0x81, 0x88, 0x20, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20] },
    // A 0.75f in Re:CoM's float pool (beside its 0.5 at 0x1406C108C and 1.0 at 0x1406C10C0; then
    // √3/2): Guard Armor's scale at KH1 points here.
    Site { name: "float pool 0.75 (0x1406C10A8)", rva: 0x006C_10A8, expect: &[0x00, 0x00, 0x40, 0x3F, 0xD7, 0xB3, 0x5D, 0x3F] },
];

/// Enemy Skills stage 2: Re:CoM's constants the owner's numbers replace, switched at each battle start
/// (`win::enemy_skills::battle_start`, as M6's): ours in a KH1 battle (Combat = KH1, Sora's story),
/// Re:CoM's otherwise. Each touches Sora's side only (the enemy side has its own sites).
pub const ENEMY_SKILL_PATCHES: &[Patch] = &[
    // Oogie Boogie: Sora's Regen tick in the CBTL update `0x1401617F0` (`cmp dword [rbx+0x1F8],
    // 6000`): 18000 (6 s).
    Patch { site: Site { name: "Regen tick, CBTL update (0x14016189D)", rva: 0x0016_189D, expect: &[0x81, 0xBB, 0xF8, 0x01, 0x00, 0x00, 0x70, 0x17, 0x00, 0x00] }, new: &[0x81, 0xBB, 0xF8, 0x01, 0x00, 0x00, 0x50, 0x46, 0x00, 0x00] },
    // The same in `0x1401600D0` (`cmp dword [rcx+0x1F8], 6000`).
    Patch { site: Site { name: "Regen tick (0x1401600F2)", rva: 0x0016_00F2, expect: &[0x81, 0xB9, 0xF8, 0x01, 0x00, 0x00, 0x70, 0x17, 0x00, 0x00] }, new: &[0x81, 0xB9, 0xF8, 0x01, 0x00, 0x00, 0x50, 0x46, 0x00, 0x00] },
    // Neoshadow: Sora's Bio tick (`+0x208 & 0x2000`; `cmp dword [rbx+0x1FC], 1000`): 3000 (1 s).
    Patch { site: Site { name: "Bio tick (0x140161991)", rva: 0x0016_1991, expect: &[0x81, 0xBB, 0xFC, 0x01, 0x00, 0x00, 0xE8, 0x03, 0x00, 0x00] }, new: &[0x81, 0xBB, 0xFC, 0x01, 0x00, 0x00, 0xB8, 0x0B, 0x00, 0x00] },
    // Xemnas: the barrier timer Sora's damage check sets (`mov dword [rdi+0x7E4], 6000`): 3000 (1 s).
    Patch { site: Site { name: "Quick Barrier timer (0x1401E73F3)", rva: 0x001E_73F3, expect: &[0xC7, 0x87, 0xE4, 0x07, 0x00, 0x00, 0x70, 0x17, 0x00, 0x00] }, new: &[0xC7, 0x87, 0xE4, 0x07, 0x00, 0x00, 0xB8, 0x0B, 0x00, 0x00] },
    // Guard Armor: `setWideAttack`'s scale (`movss xmm1,[rip+disp]`: 1.0 at 0x1406C10C0) → the pool's
    // 0.75 at 0x1406C10A8 (disp 0x49B89E from the next instruction 0x14022580A).
    Patch { site: Site { name: "Wide Attack scale (0x140225802)", rva: 0x0022_5802, expect: &[0xF3, 0x0F, 0x10, 0x0D, 0xB6, 0xB8, 0x49, 0x00] }, new: &[0xF3, 0x0F, 0x10, 0x0D, 0x9E, 0xB8, 0x49, 0x00] },
];

/// Enemy Skills stage 2, Roxas: the second strike's power read in the weapon update `0x140225840`
/// (`movsx eax, word [rbx+0x288]; mov [rbp-0x60], eax`, 10 bytes): in a KH1 battle a jump to a stub
/// of ours that halves it (`cdq; sub eax,edx; sar eax,1`, Re:CoM's ÷ 2) and comes back at
/// `0x140225D86`. Checked (exe): no branch into the 10 bytes.
pub const ENEMY_SKILL_STUB_SITES: &[Site] = &[Site { name: "Double Strike power read (0x140225D7C)", rva: 0x0022_5D7C, expect: &[0x0F, 0xBF, 0x83, 0x88, 0x02, 0x00, 0x00, 0x89, 0x45, 0xA0] }];

/// Enemy Skills stage 3 (`win::enemy_skills`, `docs/recom-enemy-skills.md` § Stage 3 (built)): Re:CoM's
/// hit rewrite `0x14015D5F0(CBTL, record)`, wrapped to run once per equipped skill of route 2 with its id
/// in Sora's slot. `push rbx; sub rsp,0x20` (6 bytes). Checked (exe): 6 direct calls, no branch into the
/// moved bytes.
pub const ENEMY_SKILL_REWRITE_HOOK: &[Site] = &[Site { name: "enemy-card hit rewrite (0x14015D5F0)", rva: 0x0015_D5F0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] }];

/// Enemy Skills stage 3: the rewrite's constants the owner's numbers replace, each switched at a battle
/// start to a jump to a stub of ours reading its constant from a cell (Re:CoM's value but while our
/// wrapper runs Sora's skills): Zexion's 25 %, Lexaeus' 67 %, Demyx's × 2, Defender's and Ursula's ÷ 2.
/// Checked (exe): no branch or jump-table entry into any of them.
pub const ENEMY_SKILL_REWRITE_SITES: &[Site] = &[
    // cmp r8d,0x19; jge 0x14015D83C (Zexion's roll).
    Site { name: "rewrite: Zexion's chance (0x14015D70A)", rva: 0x0015_D70A, expect: &[0x41, 0x83, 0xF8, 0x19, 0x0F, 0x8D, 0x28, 0x01, 0x00, 0x00] },
    // cmp r8d,0x43; jge 0x14015D83C (Lexaeus' roll).
    Site { name: "rewrite: Lexaeus' chance (0x14015D74C)", rva: 0x0015_D74C, expect: &[0x41, 0x83, 0xF8, 0x43, 0x0F, 0x8D, 0xE6, 0x00, 0x00, 0x00] },
    // movzx eax,word [rbx+0x14]; add ax,ax; mov [rbx+0x14],ax (Demyx's × 2).
    Site { name: "rewrite: Demyx's factor (0x14015D831)", rva: 0x0015_D831, expect: &[0x0F, 0xB7, 0x43, 0x14, 0x66, 0x03, 0xC0, 0x66, 0x89, 0x43, 0x14] },
    // movsx eax,word [rbx+0x14]; cdq; sub eax,edx; sar eax,1 (Defender's ÷ 2).
    Site { name: "rewrite: Defender's cut (0x14015D66A)", rva: 0x0015_D66A, expect: &[0x0F, 0xBF, 0x43, 0x14, 0x99, 0x2B, 0xC2, 0xD1, 0xF8] },
    // The same for Ursula.
    Site { name: "rewrite: Ursula's cut (0x14015D64D)", rva: 0x0015_D64D, expect: &[0x0F, 0xBF, 0x43, 0x14, 0x99, 0x2B, 0xC2, 0xD1, 0xF8] },
];

/// Enemy Skills stage 3: where the stubs go back to (read only).
pub const ENEMY_SKILL_REWRITE_CHECKED: &[Site] = &[
    Site { name: "rewrite: Zexion's confuse (0x14015D714)", rva: 0x0015_D714, expect: &[0xC7, 0x43, 0x08, 0x0A, 0x00, 0x00, 0x00] },
    Site { name: "rewrite: Lexaeus' warp (0x14015D756)", rva: 0x0015_D756, expect: &[0xC7, 0x43, 0x08, 0x1F, 0x00, 0x00, 0x00] },
    Site { name: "rewrite: Defender's store (0x14015D673)", rva: 0x0015_D673, expect: &[0x66, 0x89, 0x43, 0x14, 0xB8, 0x01, 0x00, 0x00, 0x00] },
    Site { name: "rewrite: Ursula's store (0x14015D656)", rva: 0x0015_D656, expect: &[0x66, 0x89, 0x43, 0x14, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3] },
    // mov eax,0x51EB851F; imul ecx; sar edx,5; mov eax,edx; shr eax,0x1F; add edx,eax; mov [rbx+0x14],dx; ret.
    Site {
        name: "rewrite: the ÷ 100 tail (0x14015D7D6)",
        rva: 0x0015_D7D6,
        expect: &[0xB8, 0x1F, 0x85, 0xEB, 0x51, 0xF7, 0xE9, 0xC1, 0xFA, 0x05, 0x8B, 0xC2, 0xC1, 0xE8, 0x1F, 0x03, 0xD0, 0x66, 0x89, 0x53, 0x14, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3],
    },
    Site { name: "rewrite: its return (0x14015D83C)", rva: 0x0015_D83C, expect: &[0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3] },
];

/// M9 stage 9a (`win::abilities`, `docs/recom-m9-abilities.md`): Re:CoM's "this sleight is
/// learned" test, called for the abilities that unlock with Re:CoM's sleights. Only called, never
/// hooked.
pub const ABILITY_CALLEES: &[Site] = &[
    // push rbx; sub rsp,0x20; movzx ebx,cx; call 0x1401AAB00 (Re:CoM's story test); test eax,eax:
    // the 2-bit state of sleight `cx` at `*0x14087CC08` (= save + 0x2840) is not 0, for the ids
    // the story allows (Sora: 4..0x56, 0x5A..0x5C).
    Site { name: "sleight learned (0x14023AB00)", rva: 0x0023_AB00, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x0F, 0xB7, 0xD9, 0xE8, 0xF2, 0xFF, 0xF6, 0xFF, 0x85, 0xC0] },
];

/// M12 (`win::level_up`, `win::notices`, `crate::level_up`, `docs/recom-m12-sleights.md` § 8):
/// Re:CoM's level-up screen offers AP Up for CP Up at Combat = KH1, and its sleight-learned mode
/// shows our "Sora learned X" notices. Called only, never hooked (the screen's update is the
/// existing `MP_HOOKS` wrapper; `new`, the task root, the text copy and the flag get are earlier
/// stages' checked callees).
pub const M12_CALLEES: &[Site] = &[
    // mov [rsp+8],rcx; push rdi; sub rsp,0x30; mov qword [rsp+0x20],-2: CLevelUp(this, parent, id).
    Site { name: "CLevelUp ctor (0x14010E9C0)", rva: 0x0010_E9C0, expect: &[0x48, 0x89, 0x4C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE] },
    // push rbx; sub rsp,0x20; cmp dword [rcx+0x5A4],0; mov rbx,rcx: the field's menu gate.
    Site { name: "field menu gate (0x1401AAA90)", rva: 0x001A_AA90, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x83, 0xB9, 0xA4, 0x05, 0x00, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
];

/// M12: Re:CoM code and data whose behaviour the AP Up row and the notices rely on. Checked only.
/// M12 step 2: the two vtables the notices rely on, as relocated pointers (0.98.1 checked them as raw bytes,
/// which the loader relocates: every start failed its checks, 2026-11-03). Checked lazily by
/// `win::notices` (a mismatch turns the notices off, not the mod). Checked (exe).
pub const M12_PTRS: &[PtrSite] = &[
    // CTechHelp's vtable: the dtor 0x14012C230, the update 0x14012D330.
    PtrSite { name: "CTechHelp vtable +0 dtor (0x14051B4A8)", rva: 0x0051_B4A8, target: 0x0012_C230 },
    PtrSite { name: "CTechHelp vtable +8 update (0x14051B4B0)", rva: 0x0051_B4B0, target: 0x0012_D330 },
    // CLevelUp's vtable: the dtor 0x14010EB80, the update 0x14010EF10.
    PtrSite { name: "CLevelUp vtable +0 dtor (0x140512C10)", rva: 0x0051_2C10, target: 0x0010_EB80 },
    PtrSite { name: "CLevelUp vtable +8 update (0x140512C18)", rva: 0x0051_2C18, target: 0x0010_EF10 },
];

pub const M12_CHECKED: &[Site] = &[
    // mov rax,[0x14087CBF8]; cmp dword [rax+0xC],0x659; jge: the CP row offered while CP < 1625.
    Site { name: "level-up CP offer (0x14010F209)", rva: 0x0010_F209, expect: &[0x48, 0x8B, 0x05, 0xE8, 0xD9, 0x76, 0x00, 0x81, 0x78, 0x0C, 0x59, 0x06, 0x00, 0x00, 0x7D, 0x06] },
    // mov rcx,[0x14087CBF8]; add dword [rcx+0xC],0x19: the CP pick.
    Site { name: "level-up CP pick (0x14010F6A4)", rva: 0x0010_F6A4, expect: &[0x48, 0x8B, 0x0D, 0x4D, 0xD5, 0x76, 0x00, 0x83, 0x41, 0x0C, 0x19] },
    // cmp dword [rbx+0x258],0x5A: the sleight-learned line's ability wording from id 0x5A.
    Site { name: "learned line ability wording (0x1401102DD)", rva: 0x0011_02DD, expect: &[0x83, 0xBB, 0x58, 0x02, 0x00, 0x00, 0x5A] },
    // push rdi; sub rsp,0x30; mov qword [rsp+0x20],-2; mov ...: the field step that opens the camp.
    Site { name: "field camp gate step (0x1401A96C0)", rva: 0x001A_96C0, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x48] },
    Site { name: "CTechHelp ctor (0x14012BF80)", rva: 0x0012_BF80, expect: &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x48, 0x89, 0x7C, 0x24, 0x20, 0x55, 0x48, 0x8D, 0x6C, 0x24, 0xF0] },
    Site { name: "CTechHelp update (0x14012D330)", rva: 0x0012_D330, expect: &[0x48, 0x8B, 0xC4, 0x57, 0x48, 0x81, 0xEC, 0x90, 0x00, 0x00, 0x00, 0x48, 0xC7, 0x40, 0xB8, 0xFE] },
    Site { name: "CTechHelp dtor (0x14012C230)", rva: 0x0012_C230, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x48] },
    // "%4d \xF9Y\x81\xA8\xF9R%4d": the panel's preview format (`crate::level_up::panel_value`).
    Site { name: "level-up panel preview format (0x140512C70)", rva: 0x0051_2C70, expect: &[0x25, 0x34, 0x64, 0x20, 0xF9, 0x59, 0x81, 0xA8, 0xF9, 0x52, 0x25, 0x34, 0x64] },
];

/// M9 stage 9c.1 (`win::specials`, `docs/recom-m9-abilities.md` § 9c.1): KH1's specials on
/// Re:CoM's sleight action `0x16`. Same rule as [`HOOKS`].
pub const SPECIAL_HOOKS: &[Site] = &[
    // mov rax,rsp; mov [rax+0x10],rbx; mov [rax+0x18],rsi: CPlayerSora's exeStockSkill (Sora's
    // vtable +0x140; the Battle exe calls it at 0x1401F9B64; battle only: the Field exe has no
    // case for action 0x16 and is not patched).
    Site { name: "exeStockSkill (0x140218950)", rva: 0x0021_8950, expect: &[0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x10, 0x48, 0x89, 0x70, 0x18] },
];

/// Re:CoM functions the specials call, and code they replicate (not patched).
pub const SPECIAL_CHECKED: &[Site] = &[
    // initStockSkill: its log call, then 0x1401F8D70(player, 0x16) (`mov edx,0x16`). The KH1
    // special does its writes cardlessly (it reads the card's BattleCardList row for the AP).
    Site {
        name: "initStockSkill (0x140220DF0)",
        rva: 0x0022_0DF0,
        expect: &[
            0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x8B, 0xDA, 0x48, 0x8B, 0xF9, 0x48, 0x8D, 0x15, 0x6D, 0x8D, 0x38, 0x00, 0xB9, 0x03, 0x00, 0x00, 0x00, 0xE8, 0xFB, 0xFB, 0x01, 0x00, 0xBA, 0x16, 0x00,
        ],
    },
    // The Battle exe's case 0x16: mov rax,[rdi]; mov rcx,rdi; call [rax+0x140]; jmp to its end.
    Site { name: "Battle exe case 0x16 (0x1401F9B5E)", rva: 0x001F_9B5E, expect: &[0x48, 0x8B, 0x07, 0x48, 0x8B, 0xCF, 0xFF, 0x90, 0x40, 0x01, 0x00, 0x00, 0xE9, 0x5A, 0x01, 0x00, 0x00] },
    // procAttackCollision(player, frame): the row's window (+0x3C8 → +4 / +5), the weapon's
    // attack (`0x1402250B0`) with +0x638 / +0x660 / +0x668 / +0x66C; called by the swing exe and
    // exeStockSkill, here by our exeStockSkill wrapper for a KH1 special row.
    Site { name: "procAttackCollision (0x1401F5DC0)", rva: 0x001F_5DC0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x48, 0x8B, 0xD9, 0x48, 0x8B, 0x89, 0xC8, 0x03, 0x00, 0x00, 0x0F, 0xB6, 0x41, 0x04] },
    // The sleight resource load (`initStockSkill` calls it with (player+0x3B8, 9, 0) for Sonic
    // Blade: group 0x3F1 through `0x14020F720` / `0x14020FB10`).
    Site { name: "sleight resources (0x140211790)", rva: 0x0021_1790, expect: &[0x40, 0x53, 0x41, 0x54, 0x48, 0x83, 0xEC, 0x38, 0x48, 0x8B, 0xD9, 0xE8, 0x80, 0xDF, 0xFF, 0xFF] },
    // Base Re:CoM's Sonic Blade events (owner: its voices and effects; `crate::specials::recom_events`).
    // 0x1401A3580 (the sub-pack motion play exeStockSkill's 0x1401F73A0 calls): Sora's own players
    // stopped unless ctrl+0x52 & 0x200, then the sleight's set to the pack and started.
    Site { name: "sub-pack motion play (0x1401A3580)", rva: 0x001A_3580, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x30] },
    // The effect events' pack (player +0x110, edx = pack) and start (edx = motion, r8d = variant,
    // r9d = 1: stop first).
    Site { name: "effect events pack (0x140179A30)", rva: 0x0017_9A30, expect: &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x56, 0x48, 0x83, 0xEC, 0x20, 0x45, 0x33, 0xC0, 0x48, 0x8B, 0xF1, 0x8B, 0xDA] },
    Site { name: "effect events start (0x14017A240)", rva: 0x0017_A240, expect: &[0x89, 0x54, 0x24, 0x10, 0x57, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x38, 0x48, 0x8B, 0x41, 0x18, 0x45, 0x8B, 0xF8] },
    // The sound events' pack (player +0x120) and start (`CSEMotion`).
    Site { name: "sound events pack (0x140238710)", rva: 0x0023_8710, expect: &[0x41, 0x56, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x48, 0x89, 0x5C, 0x24, 0x40] },
    Site { name: "sound events start (0x140239260)", rva: 0x0023_9260, expect: &[0x44, 0x89, 0x44, 0x24, 0x18, 0x89, 0x54, 0x24, 0x10, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x68, 0x48, 0x83, 0x79] },
    // A sleight's resources loaded (rcx = player+0x3B8, edx = sleight id; 1 = in).
    Site { name: "sleight resources ready (0x1402106E0)", rva: 0x0021_06E0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0xE8, 0x32, 0xF0, 0xFF, 0xFF, 0x85, 0xC0] },
    // Re:CoM's attack rows 0xE..=0x10 (Sonic Blade's start, dash, finisher): byte 0 = the motion
    // in the sleight pack (0, 1, 2), byte 1 = the start's EET variant (0), byte 0xF flags (0x40: a
    // sleight-pack motion).
    Site {
        name: "Re:CoM rows 0xE..=0x10 (0x1407C6D50)",
        rva: 0x007C_6D50,
        expect: &[
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x60, 0x01, 0x00, 0x01, 0x00, 0x03, 0x22, 0x03, 0x14, 0x0F, 0x14, 0x00, 0x03, 0x00, 0x00, 0x00, 0x64, 0x02, 0x00, 0x04, 0x00, 0x0F, 0x3C, 0x0F, 0x20, 0x0F, 0x14, 0x02, 0x03, 0x00, 0x00, 0x00, 0x64,
        ],
    },
];

/// Re:CoM's Sonic Blade start tint (owner, 2026-11-01: "Add Re:CoM's tint"; `win::specials`,
/// `docs/recom-m9-abilities.md` § 9c.1): what `exeStockSkill` calls at its SONICRAVE start, and how
/// Re:CoM's end action ends it. Checked at start; a mismatch only leaves the tint out.
pub const SPECIAL_TINT_CHECKED: &[Site] = &[
    // mov [rsp+8],rbx; push rdi; sub rsp,0x30; movss xmm0,[rdx+4]: the screen tint (player, colour,
    // time, flags): blend 0x1808 (flags & 1: 0x187C), `+0x418` bit 31 set unless r + g + b = 3.
    Site { name: "screen tint (0x1401F7D80)", rva: 0x001F_7D80, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x30, 0xF3, 0x0F, 0x10, 0x42, 0x04, 0x41] },
    // exeStockSkill's start switch, attack id 0xE (× 10 = 0x8C): lea rdx,[0x1407C5D50].
    Site { name: "SONICRAVE start tint colour (0x140218E05)", rva: 0x0021_8E05, expect: &[0x48, 0x8D, 0x15, 0x44, 0xCF, 0x5A, 0x00] },
    // The call it jumps to: xor r9d,r9d (flags 0); movaps xmm2,xmm6 (the time); mov rcx,r14; call
    // 0x1401F7D80.
    Site { name: "sleight start tint call (0x140218D0E)", rva: 0x0021_8D0E, expect: &[0x45, 0x33, 0xC9, 0x0F, 0x28, 0xD6, 0x49, 0x8B, 0xCE, 0xE8, 0x64, 0xF0, 0xFD, 0xFF] },
    // xmm6 = [0x1406C1208] (`0x140218CA9`) = 20.0 (and the next constant, 30.0).
    Site { name: "sleight tint time 20.0 (0x1406C1208)", rva: 0x006C_1208, expect: &[0x00, 0x00, 0xA0, 0x41, 0x00, 0x00, 0xF0, 0x41] },
    // The colour (0.5, 0.5, 0.75, 1.0).
    Site { name: "Sonic Blade tint colour (0x1407C5D50)", rva: 0x007C_5D50, expect: &[0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x40, 0x3F, 0x00, 0x00, 0x80, 0x3F] },
    // The end action `0x1401F6660`: mov eax,[rbx+0x418]; xor r14d,r14d; test eax,eax; jns: with bit
    // 31 set, the tint back to (1, 1, 1, 1) over 20.0 (blend 0x1808) and the bit cleared.
    Site { name: "end action tint reset (0x1401F66D4)", rva: 0x001F_66D4, expect: &[0x8B, 0x83, 0x18, 0x04, 0x00, 0x00, 0x45, 0x33, 0xF6, 0x85, 0xC0, 0x79, 0x35] },
];

/// M12 step 7 (`win::specials`, `docs/recom-m9-abilities.md` § Strike Raid): Re:CoM's Strike Raid
/// bits the KH1 Strike Raid copies or calls (no hook, no patch). Checked at start; a mismatch leaves
/// Strike Raid out.
pub const SPECIAL_SR_CHECKED: &[Site] = &[
    // exeStockSkill's start for a row without flag 0x20: sub ecx,0x82 (attack id 0xD × 10); je
    // 0x140218F28.
    Site { name: "Strike Raid start dispatch (0x140218E62)", rva: 0x0021_8E62, expect: &[0x81, 0xE9, 0x82, 0x00, 0x00, 0x00, 0x0F, 0x84, 0xBA, 0x00, 0x00, 0x00] },
    // lea rdx,[0x1407C5D70]: its tint colour.
    Site { name: "Strike Raid tint colour lea (0x140218F28)", rva: 0x0021_8F28, expect: &[0x48, 0x8D, 0x15, 0x41, 0xCE, 0x5A, 0x00] },
    // xor r9d,r9d (flags 0); movaps xmm2,xmm6 (20.0, `0x140218CA9`); mov rcx,r14; call 0x1401F7D80.
    Site { name: "Strike Raid tint call (0x140218F4A)", rva: 0x0021_8F4A, expect: &[0x45, 0x33, 0xC9, 0x0F, 0x28, 0xD6, 0x49, 0x8B, 0xCE, 0xE8, 0x28, 0xEE, 0xFD, 0xFF] },
    // The colour (0.6, 0.5, 0.75, 1.0).
    Site { name: "Strike Raid tint colour (0x1407C5D70)", rva: 0x007C_5D70, expect: &[0x9A, 0x99, 0x19, 0x3F, 0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x40, 0x3F, 0x00, 0x00, 0x80, 0x3F] },
    // Re:CoM's attack row 0xD (Strike Raid): byte 0 = Sora's action row 0x3F (`p001_074`), byte 1 =
    // its EET variant 1, byte 0xA = its StockSkillList column 0, byte 0xF = 0 (no flag 0x20: no
    // resource wait; no 0x40: Sora's own motions).
    Site { name: "Re:CoM row 0xD (0x1407C6D40)", rva: 0x007C_6D40, expect: &[0x3F, 0x01, 0x07, 0x28, 0x08, 0x4C, 0x0C, 0x28, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00] },
    // Sora's own motion play (his vtable +0x70): its event part (`EET start(+0x108, slot, p6, 1)`,
    // `SES start(+0x118, slot, p7, 1)`) is what `motion_events` does for Re:CoM's Strike Raid clips.
    Site { name: "own motion play (0x1401A3470)", rva: 0x001A_3470, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x20] },
    // The action-row play: movsxd rax,[rcx+0x398]; lea rcx,[0x1407C6300]: the slot of a row is
    // `[0x1407C6300 + 8 × +0x398]` + row × 16 + 8 (`0x1401F71EC`: mov esi,[rax+rdx*8+8]).
    Site { name: "action-row slot lookup (0x1401F71BB)", rva: 0x001F_71BB, expect: &[0x48, 0x63, 0x81, 0x98, 0x03, 0x00, 0x00, 0x48, 0x8D, 0x0D, 0x37, 0xF1, 0x5C, 0x00] },
];

/// M12 step 7 (`win::specials`, `docs/recom-m9-abilities.md` § Ars Arcanum): Re:CoM's Ars Arcanum bits the
/// KH1 Ars Arcanum copies (no hook, no patch). Checked at start; a mismatch leaves Ars Arcanum out.
pub const SPECIAL_AA_CHECKED: &[Site] = &[
    // initStockSkill case 10: mov [rdi+0x638],0x11; mov edx,0xA; jmp 0x140220F0D (its resources).
    Site { name: "initStockSkill case 10 (0x140220F73)", rva: 0x0022_0F73, expect: &[0xC7, 0x87, 0x38, 0x06, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0xBA, 0x0A, 0x00, 0x00, 0x00, 0xEB, 0x89] },
    // exeStockSkill's start for a row with flag 0x20, id × 10 after 0x5A / 0x8C: sub ecx,0x1E (0xAA);
    // je 0x140218DF9.
    Site { name: "Ars Arcanum start dispatch (0x140218CF2)", rva: 0x0021_8CF2, expect: &[0x83, 0xE9, 0x1E, 0x0F, 0x84, 0xFE, 0x00, 0x00, 0x00] },
    // lea rdx,[0x1407C5D60]; jmp 0x140218D0E (the tint call `SPECIAL_TINT_CHECKED` checks).
    Site { name: "Ars Arcanum tint colour lea (0x140218DF9)", rva: 0x0021_8DF9, expect: &[0x48, 0x8D, 0x15, 0x60, 0xCF, 0x5A, 0x00, 0xE9, 0x09, 0xFF, 0xFF, 0xFF] },
    // The colour (0.75, 0.5, 0.6, 1.0).
    Site { name: "Ars Arcanum tint colour (0x1407C5D60)", rva: 0x007C_5D60, expect: &[0x00, 0x00, 0x40, 0x3F, 0x00, 0x00, 0x00, 0x3F, 0x9A, 0x99, 0x19, 0x3F, 0x00, 0x00, 0x80, 0x3F] },
    // Re:CoM's attack rows 0x11..=0x1E (the start, twelve hits, the finisher): byte 0 = the motion in
    // the sleight's pack (0..=13), byte 1 = the EET variant 0, byte 0xA the StockSkillList column (the
    // seventh hit 1, the finisher 2), byte 0xF flags (0x60 the start, 0x68 the hits).
    Site {
        name: "Re:CoM rows 0x11..=0x1E (0x1407C6D80)",
        rva: 0x007C_6D80,
        expect: &[
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x60, //
            0x01, 0x00, 0x01, 0x00, 0x07, 0x10, 0x00, 0x10, 0x0F, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, //
            0x02, 0x00, 0x01, 0x00, 0x06, 0x10, 0x00, 0x0E, 0x0F, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, //
            0x03, 0x00, 0x01, 0x00, 0x04, 0x10, 0x00, 0x0A, 0x0F, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, //
            0x04, 0x00, 0x01, 0x00, 0x02, 0x10, 0x00, 0x08, 0x0F, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, //
            0x05, 0x00, 0x01, 0x00, 0x02, 0x10, 0x00, 0x0A, 0x0F, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, //
            0x06, 0x00, 0x01, 0x00, 0x02, 0x10, 0x00, 0x0A, 0x0F, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, //
            0x07, 0x00, 0x05, 0x00, 0x0C, 0x1A, 0x00, 0x18, 0x0F, 0x14, 0x01, 0x02, 0x00, 0x00, 0x00, 0x68, //
            0x08, 0x00, 0x01, 0x00, 0x08, 0x16, 0x00, 0x1A, 0x0F, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, //
            0x09, 0x00, 0x01, 0x00, 0x08, 0x22, 0x00, 0x22, 0x0F, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, //
            0x0A, 0x00, 0x01, 0x00, 0x08, 0x28, 0x00, 0x2C, 0x0F, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, //
            0x0B, 0x00, 0x01, 0x00, 0x08, 0x18, 0x00, 0x1C, 0x0F, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, //
            0x0C, 0x00, 0x01, 0x00, 0x12, 0x28, 0x12, 0x22, 0x0F, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x68, //
            0x0D, 0x00, 0x06, 0x00, 0x16, 0x3C, 0x0C, 0x2E, 0x0F, 0x14, 0x02, 0x03, 0x00, 0x00, 0x00, 0x68,
        ],
    },
];

/// M12 step 7 (`win::specials`, Ragnarok): Re:CoM's Ragnarok start, its tint and its rows, as read
/// from the Steam exe. Its `limit3` name pointer (`0x1407C7E20`, an absolute pointer) is checked at
/// start by the string it points to, not here.
pub const SPECIAL_RAG_CHECKED: &[Site] = &[
    // initStockSkill case 0xB: mov [rdi+0x638],0x1F; mov edx,0xB; jmp (its resources).
    Site { name: "initStockSkill case 0xB (0x140220F84)", rva: 0x0022_0F84, expect: &[0xC7, 0x87, 0x38, 0x06, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00, 0xBA, 0x0B, 0x00, 0x00, 0x00, 0xE9, 0x75, 0xFF, 0xFF, 0xFF] },
    // exeStockSkill's start, id × 10 = 0x136 after 0xAA: cmp ecx,0x8C; je 0x140218E29.
    Site { name: "Ragnarok start dispatch (0x140218CFB)", rva: 0x0021_8CFB, expect: &[0x81, 0xF9, 0x8C, 0x00, 0x00, 0x00, 0x0F, 0x84, 0x22, 0x01, 0x00, 0x00] },
    // lea rdx,[0x1407C5D90]; jmp 0x140218D0E (the tint call).
    Site { name: "Ragnarok tint colour lea (0x140218E29)", rva: 0x0021_8E29, expect: &[0x48, 0x8D, 0x15, 0x60, 0xCF, 0x5A, 0x00, 0xE9, 0xD9, 0xFE, 0xFF, 0xFF] },
    // The colour (0.5, 0.75, 0.5, 1.0).
    Site { name: "Ragnarok tint colour (0x1407C5D90)", rva: 0x007C_5D90, expect: &[0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x40, 0x3F, 0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x80, 0x3F] },
    // Re:CoM's rows 0x1F (the strike: motion 0, column 0) and 0x20 (the shots: effect set 2, kind 2,
    // column 0).
    Site {
        name: "Re:CoM rows 0x1F..=0x20 (0x1407C6E60)",
        rva: 0x007C_6E60,
        expect: &[
            0x00, 0x00, 0x00, 0x00, 0x46, 0x64, 0x00, 0x78, 0x00, 0x10, 0x00, 0x00, 0x08, 0x0C, 0x10, 0x60, //
            0x02, 0x00, 0x02, 0x00, 0x0F, 0x14, 0x00, 0x78, 0x1E, 0x08, 0x00, 0x02, 0x10, 0x04, 0x00, 0x40,
        ],
    },
];

/// CPlayerSora's exeStockSkill slot (vtable `+0x140`), which the Battle exe calls through.
pub const SPECIAL_PTRS: &[PtrSite] = &[PtrSite { name: "CPlayerSora vtable +0x140 (0x1405A9A78)", rva: 0x005A_9A78, target: 0x0021_8950 }];

/// M13 (`win::effects`): Re:CoM's effect-by-name, called once at the first battle to check that the
/// KH1 sets the asset step added are loaded.
pub const EFFECT_CHECKED: &[Site] = &[
    // push rbx; sub rsp,0x40; mov rax,[rip+0x5BE19B] (the cookie, rip-relative); xor rax,rsp:
    // `0x1402A43E0(5, name)`, then the name with `_` added or removed; returns the set (`+0x68`).
    Site { name: "effect by name (0x1402A4920)", rva: 0x002A_4920, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40, 0x48, 0x8B, 0x05, 0x9B, 0xE1, 0x5B, 0x00, 0x48, 0x33, 0xC4] },
];

/// M13 stage 3 (`win::fx`): KH1's trail end. Re:CoM's EET motion stop (CEffectMotion vtable
/// slot `0x14052A338`, its only reference: Checked (exe scan)), wrapped: after it, the KH1 trails still
/// lingering on that motion player are killed (KH1: a motion change kills the trail). Same rule as
/// [`HOOKS`]: whole instructions, nothing rip-relative, no branch into them (exe scan).
pub const FX_HOOKS: &[Site] = &[
    // push rsi; push r14; push r15; sub rsp,0x30 (then xor r15d,r15d: not moved).
    Site { name: "EET motion stop (0x14017A430)", rva: 0x0017_A430, expect: &[0x40, 0x56, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x30] },
];

/// M13 stage 3: the list-B update's stop call at the end frame (`0x14017AB70`: `call 0x140254D60`
/// with rcx = the effect, xmm1 = the event's fade `+0x60`), retargeted to `win::fx::trail_end`.
pub const FX_CALLS: &[Site] = &[Site { name: "EET list-B end stop call (0x14017ABEF)", rva: 0x0017_ABEF, expect: &[0xE8, 0x6C, 0xA1, 0x0D, 0x00] }];

/// M13 stage 3: the call's context and the Re:CoM functions `win::fx` calls (not patched).
pub const FX_CHECKED: &[Site] = &[
    // mov rcx,[r14+0x10] (the event); movss xmm1,[rcx+0x60] (its fade); mov rcx,rax (the effect).
    Site { name: "EET list-B end: the fade read (0x14017ABE3)", rva: 0x0017_ABE3, expect: &[0x49, 0x8B, 0x4E, 0x10, 0xF3, 0x0F, 0x10, 0x49, 0x60, 0x48, 0x8B, 0xC8] },
    // mov r9,[r14+0x10]; mov r8,[r14+8]; mov edx,[r14+0x18]; mov rcx,[r14+0x28]; call 0x14017A1A0 (to the linger list).
    Site { name: "EET list-B end: the linger add (0x14017ABF4)", rva: 0x0017_ABF4, expect: &[0x4D, 0x8B, 0x4E, 0x10, 0x4D, 0x8B, 0x46, 0x08, 0x41, 0x8B, 0x56, 0x18, 0x49, 0x8B, 0x4E, 0x28, 0xE8, 0x97, 0xF5, 0xFF, 0xFF] },
    // push rbx; sub rsp,0x20; mov eax,[rcx+0x4C]; xorps xmm0,xmm0: the effect's fade-out (≤ 0 kills).
    Site { name: "effect fade out (0x140254D60)", rva: 0x0025_4D60, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x41, 0x4C, 0x0F, 0x57, 0xC0] },
    // push rbx; sub rsp,0x20; mov rbx,rcx: the effect's stop emitting (its particle instance `+0x58`).
    Site { name: "effect stop emitting (0x140255040)", rva: 0x0025_5040, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9] },
    // (Handle → effect `0x140257EF0` is `MAGIC_CALLEES`' "effect object" site, checked at start.)
];

/// M13 stage 3: CEffectMotion's motion-stop slot (the hooked function's only reference). Checked (exe).
pub const FX_PTRS: &[PtrSite] = &[PtrSite { name: "CEffectMotion vtable motion stop (0x14052A338)", rva: 0x0052_A338, target: 0x0017_A430 }];

/// M8b (`win::summons`, `docs/recom-summons.md`): Re:CoM code the summons call (no hook, no patch):
/// each KH1 summon starts Re:CoM's LV3 sleight through Sora's own sleight start. Checked at start;
/// a mismatch: no summons. They also call `initStockSkill` (through the start), the sleight
/// resource load (the preload) and the weapon's attack end, checked from their own lists
/// ([`SUMMON_ALSO`]).
pub const SUMMON_CHECKED: &[Site] = &[
    // push rbx; sub rsp,0x20; mov rbx,rcx; mov ecx,[rcx+0x82C]; lea eax,[rcx-4]; cmp eax,0x55; ja:
    // CPlayerSora's sleight start (vtable +0x130): `0x14020E590` (the use counter, LV3 counted as
    // its LV2), then initStockSkill (vtable +0x138), `+0x838` = 10; 1 = started. Re:CoM calls it
    // from its request bits `0x1401F8150` for a played sleight.
    Site { name: "sleight start (0x1402247A0)", rva: 0x0022_47A0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0x8B, 0x89, 0x2C, 0x08, 0x00, 0x00, 0x8D, 0x41, 0xFC, 0x83, 0xF8, 0x55, 0x77, 0x4B] },
    // xor r8d,r8d; add edx,-0x34; cmp edx,0xF; ja: the sleight → summon actor class (0x34..0x43 →
    // 1 Simba, 2 Genie, 3 Bambi, 4 Dumbo, 5 Tinker Bell, 6 Mushu, 7 Cloud).
    Site { name: "summon class of a sleight (0x14020FF40)", rva: 0x0020_FF40, expect: &[0x45, 0x33, 0xC0, 0x83, 0xC2, 0xCC, 0x83, 0xFA, 0x0F, 0x77, 0x57] },
    // The card-info rows of the summon cards 0x23..0x29 (type 0x33 = summon; icon page / cell;
    // byte 3 = premium + Sora's story), first 8 bytes (no pointer).
    Site { name: "card info 0x23 Simba (0x140525900)", rva: 0x0052_5900, expect: &[0x33, 0x05, 0x07, 0x03, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x24 Genie (0x140525910)", rva: 0x0052_5910, expect: &[0x33, 0x06, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x25 Bambi (0x140525920)", rva: 0x0052_5920, expect: &[0x33, 0x06, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x26 Dumbo (0x140525930)", rva: 0x0052_5930, expect: &[0x33, 0x06, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x27 Tinker Bell (0x140525940)", rva: 0x0052_5940, expect: &[0x33, 0x06, 0x03, 0x03, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x28 Mushu (0x140525950)", rva: 0x0052_5950, expect: &[0x33, 0x06, 0x04, 0x03, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x29 Cloud (0x140525960)", rva: 0x0052_5960, expect: &[0x33, 0x06, 0x05, 0x03, 0x00, 0x00, 0x00, 0x00] },
];

/// 0.111.0, the command menu's Party list (`crate::party`, `win::party`): Re:CoM's room-type getter
/// (called) and its Friend-card drop table (read through `crate::party::DROP_TABLE`, a copy these
/// bytes prove). The other callees are earlier stages' checked sites (the world `0x1402339C0`, Riku's
/// story `0x1401AAAE0`, the minigame room `0x140235360`, the int variable get `0x1401B08C0`, the
/// sleight-learned test `0x14023AB00`). Never hooked.
pub const PARTY_CHECKED: &[Site] = &[
    // mov rax,[0x14087B110]; movzx ecx,byte [rax+1]; test ecx,ecx; je; cmp ecx,0x63; je; cmp cl,0x20;
    // mov eax,-1; cmovb eax,ecx; ret; mov eax,ecx; ret: the save block's room-type byte (0 / 99
    // as they are, 1..0x1F, else −1).
    Site { name: "room type (0x140233860)", rva: 0x0023_3860, expect: &[0x48, 0x8B, 0x05, 0xA9, 0x78, 0x64, 0x00, 0x0F, 0xB6, 0x48, 0x01, 0x85, 0xC9, 0x74, 0x11, 0x83, 0xF9, 0x63, 0x74, 0x0C, 0x80, 0xF9, 0x20, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F, 0x42, 0xC1, 0xC3, 0x8B, 0xC1, 0xC3] },
    // 26 rows of {chance ‰, three weights %, three Friend cards, 0} (no pointer).
    Site { name: "Friend-card drop table (0x1407C1F10)", rva: 0x007C_1F10, expect: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x96, 0x32, 0x32, 0x00, 0x31, 0x32, 0x00, 0x00, 0x96, 0x1E, 0x14, 0x32, 0x31, 0x32, 0x33, 0x00, 0x96, 0x3C, 0x28, 0x00, 0x31, 0x32, 0x00, 0x00, 0x96, 0x28, 0x3C, 0x00, 0x31, 0x32, 0x00, 0x00, 0x96, 0x32, 0x32, 0x00, 0x31, 0x32, 0x00, 0x00, 0x96, 0x14, 0x1E, 0x32, 0x31, 0x32, 0x35, 0x00, 0x96, 0x14, 0x14, 0x3C, 0x31, 0x32, 0x34, 0x00, 0x96, 0x14, 0x1E, 0x32, 0x31, 0x32, 0x36, 0x00, 0x96, 0x1E, 0x1E, 0x28, 0x31, 0x32, 0x37, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x96, 0x32, 0x32, 0x00, 0x31, 0x32, 0x00, 0x00, 0x96, 0x32, 0x32, 0x00, 0x31, 0x32, 0x00, 0x00, 0x96, 0x32, 0x32, 0x00, 0x31, 0x32, 0x00, 0x00, 0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0x7D, 0x64, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00] },
];

/// The RVAs of the other lists' sites the summons call or rely on (`initStockSkill`, the sleight
/// resource load, the weapon's attack end).
pub const SUMMON_ALSO: [u32; 3] = [0x0022_0DF0, 0x0021_1790, 0x0022_4FB0];

/// While a sleight, special or summon runs in the field (owner, 2026-11-04; `crate::sleights::camp_blocked`
/// / `field_contact_shield`, `win::field_sleights::camp_menu_gate` / `field_contact`): two calls
/// retargeted (each to a stub jumping to ours, which calls Re:CoM's callee unless the rule holds).
pub const FIELD_RULE_CALLS: &[Site] = &[
    // The camp gate step `0x1401A96C0`: call 0x1401AAA90 (the field's menu gate), its only caller.
    Site { name: "camp gate's menu-gate call (0x1401A9759)", rva: 0x001A_9759, expect: &[0xE8, 0x32, 0x13, 0x00, 0x00] },
    // The field Heartless update `0x14019E020`: call 0x14019BED0 (the contact encounter). Its other
    // caller, the struck state's exe `0x14019E8F0` (the field strike's battle), is not changed.
    Site { name: "field contact's encounter call (0x14019E262)", rva: 0x0019_E262, expect: &[0xE8, 0x69, 0xDC, 0xFF, 0xFF] },
];

/// The calls' context and the contact encounter (not patched).
pub const FIELD_RULE_CHECKED: &[Site] = &[
    // mov rcx,[0x14087B0B0] (the game state) before the call.
    Site { name: "camp gate: the game state for the menu gate (0x1401A9752)", rva: 0x001A_9752, expect: &[0x48, 0x8B, 0x0D, 0x57, 0x19, 0x6D, 0x00] },
    // test eax,eax; je 0x1401A985E (no camp) after it.
    Site { name: "camp gate: no camp when the menu gate says 0 (0x1401A975E)", rva: 0x001A_975E, expect: &[0x85, 0xC0, 0x0F, 0x84, 0xF8, 0x00, 0x00, 0x00] },
    // xor edi,edi; xor r9d,r9d; mov [rsp+0x20],rdi; xor r8d,r8d; lea edx,[rdi+1]; call 0x140154970
    // (the bodies' touch test with Sora); test eax,eax; je; mov rcx,rbx (the enemy).
    Site {
        name: "field contact: Sora's touch test (0x14019E246)",
        rva: 0x0019_E246,
        expect: &[0x33, 0xFF, 0x45, 0x33, 0xC9, 0x48, 0x89, 0x7C, 0x24, 0x20, 0x45, 0x33, 0xC0, 0x8D, 0x57, 0x01, 0xE8, 0x15, 0x67, 0xFB, 0xFF, 0x85, 0xC0, 0x74, 0x2A, 0x48, 0x8B, 0xCB],
    },
    // test eax,eax; je (no encounter: the bodies stay on) after it; mov rcx,[rbx+0xF8] (then its bodies off).
    Site { name: "field contact: bodies off only after an encounter (0x14019E267)", rva: 0x0019_E267, expect: &[0x85, 0xC0, 0x74, 0x1E, 0x48, 0x8B, 0x8B, 0xF8, 0x00, 0x00, 0x00] },
    // mov edx,[rcx+0x25C]; mov eax,edx; and eax,0x40; test dl,0x10; je: the encounter type from the
    // enemy's flags, then `0x14019C210` (the battle request). Called.
    Site { name: "contact encounter (0x14019BED0)", rva: 0x0019_BED0, expect: &[0x8B, 0x91, 0x5C, 0x02, 0x00, 0x00, 0x8B, 0xC2, 0x83, 0xE0, 0x40, 0xF6, 0xC2, 0x10, 0x74, 0x12] },
];

/// Lucky Bounty at Combat = KH1 brings a KH1 item (owner, 2026-11-04; `crate::party::lucky_bounty`,
/// `win::party::pluto_find`): Pluto's find (`CFriendPluto`, called only from `0x1401E338E` and
/// `0x1401E36A9`; no other reference, no branch into the 9 moved bytes: exe scan).
pub const PLUTO_HOOKS: &[Site] = &[
    // mov [rsp+0x20],rsi; push rbp; push rdi; push r15 (then lea rbp,[rsp−0xA0]: not moved).
    Site { name: "Pluto's find (0x1401E2E60)", rva: 0x001E_2E60, expect: &[0x48, 0x89, 0x74, 0x24, 0x20, 0x55, 0x57, 0x41, 0x57] },
];

/// What the wrapper relies on in Re:CoM's find and calls (not patched).
pub const PLUTO_CHECKED: &[Site] = &[
    // movzx r8d,word [rsi+0x472] (the outcome); …; je outcome 0; sub ecx,1; je outcome 1; cmp ecx,1;
    // lea ecx,[r15+3]; je outcome 2 (the cards).
    Site {
        name: "Pluto's find: the outcome switch (0x1401E2EE3)",
        rva: 0x001E_2EE3,
        expect: &[
            0x44, 0x0F, 0xB7, 0x86, 0x72, 0x04, 0x00, 0x00, 0xC7, 0x45, 0x38, 0x00, 0x00, 0x80, 0x3E, 0x41, 0x8B, 0xC8, 0x45, 0x85, 0xC0, 0x0F, 0x84, 0x24, 0x03, 0x00, 0x00, 0x48, 0x89, 0x9C, 0x24, 0xC8, 0x01, 0x00, 0x00, 0x83, 0xE9, 0x01, 0x0F, 0x84, 0x10, 0x01, 0x00,
            0x00, 0x83, 0xF9, 0x01, 0x41, 0x8D, 0x4F, 0x03, 0x74, 0x11,
        ],
    },
    // Outcome 2: mov rax,[rsi+0x248] (the player); mov r14,[rax+0x3F0] (the card battle's deck);
    // test; je (none); mov rcx,r14; call 0x140166930 (cards to deal: 0 at KH1).
    Site { name: "Pluto's find: outcome 2 deals from the deck (0x1401E2F69)", rva: 0x001E_2F69, expect: &[0x48, 0x8B, 0x86, 0x48, 0x02, 0x00, 0x00, 0x4C, 0x8B, 0xB0, 0xF0, 0x03, 0x00, 0x00, 0x4D, 0x85, 0xF6, 0x0F, 0x84, 0x92, 0x00, 0x00, 0x00, 0x49, 0x8B, 0xCE, 0xE8, 0xA8, 0x39, 0xF8, 0xFF] },
    // mov [rsp+0x18],rbx; push rdi; sub rsp,0x70; the cookie; mov rdi,rdx; mov rbx,rcx;
    // mov edx,[rcx+0x2A0]; call 0x1402ABB20: a friend's effect point (the find's first call). Called.
    Site { name: "friend effect point (0x1401D9DB0)", rva: 0x001D_9DB0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x57, 0x48, 0x83, 0xEC, 0x70, 0x48, 0x8B, 0x05, 0x07, 0x8D, 0x68, 0x00, 0x48, 0x33, 0xC4, 0x48, 0x89, 0x44, 0x24, 0x60, 0x48, 0x8B, 0xFA, 0x48, 0x8B, 0xD9, 0x8B, 0x91, 0xA0, 0x02, 0x00, 0x00, 0xE8, 0x46, 0x1D, 0x0D, 0x00] },
];

/// A Gravity-flattened enemy stays hittable at Combat = KH1 (owner, 2026-11-04; `crate::magic::flatten`,
/// `win::magic::flatten_begin` / `flatten_end`, `docs/recom-m7-magic.md` § The flatten keeps its hurt
/// bodies): Re:CoM's flatten state's begin and end (state `+0xAC0`, the reaction picker's case 0xD
/// `0x14017D626`; each the begin / end of that state in 78 class tables, no other reference: exe scan;
/// no branch into the 11 moved bytes: exe scan).
pub const FLATTEN_HOOKS: &[Site] = &[
    // push rbx; sub rsp,0x20; mov edx,-1 (then the hurt bodies off: `FLATTEN_CHECKED`).
    Site { name: "flatten state begin (0x14017C350)", rva: 0x0017_C350, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF] },
    // push rbx; sub rsp,0x20; mov edx,-1 (then the hurt bodies on).
    Site { name: "flatten state end (0x14017EB80)", rva: 0x0017_EB80, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF] },
];

/// KH1's own hit flatten at Damage = KH1 (owner, 2026-11-04; `crate::magic::flatten::kh1`,
/// `win::magic::flatten_exe`): Re:CoM's flatten state's exe `0x140183BA0` (that state's exe in the
/// same 79 class tables as its begin / end; no branch into the 6 moved bytes: exe scan).
pub const FLATTEN_EXE_HOOK: &[Site] = &[
    // push rbx; sub rsp,0x60 (then the cookie: `FLATTEN_EXE_CHECKED`).
    Site { name: "flatten state exe (0x140183BA0)", rva: 0x0018_3BA0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x60] },
];

/// The rest of the exe (not patched): the phase `+0x270` switch (0 the squash: `+0x160` / 250.0,
/// at least 0.1, through `0x1401A7050`, then 9000 ticks; 1 the hold, then 1.0 and the get-up motion
/// (vtable `+0x70` with the param's `+0x5C`); 2 the motion's end `0x1401A57D0`, then `+0x2F8`), each
/// phase's `inc [rbx+0x270]`.
pub const FLATTEN_EXE_CHECKED: &[Site] = &[Site {
    name: "flatten state exe: its phases (0x140183BA6)",
    rva: 0x0018_3BA6,
    expect: &[
            0x48, 0x8B, 0x05, 0x1B, 0xEF, 0x6D, 0x00, 0x48, 0x33, 0xC4, 0x48, 0x89, 0x44, 0x24, 0x50, 0x48, 0x8B, 0xD9, 0x8B, 0x89, 0x70, 0x02, 0x00, 0x00,
            0x85, 0xC9, 0x0F, 0x84, 0xE0, 0x00, 0x00, 0x00, 0x83, 0xE9, 0x01, 0x74, 0x52, 0x83, 0xF9, 0x01, 0x0F, 0x85, 0x37, 0x01, 0x00, 0x00, 0x48, 0x8B,
            0xCB, 0xE8, 0xF4, 0x1B, 0x02, 0x00, 0x85, 0xC0, 0x0F, 0x84, 0x27, 0x01, 0x00, 0x00, 0x48, 0x8B, 0x83, 0xF8, 0x02, 0x00, 0x00, 0xC7, 0x83, 0xC8,
            0x02, 0x00, 0x00, 0xFA, 0x00, 0x00, 0x00, 0x48, 0x85, 0xC0, 0x0F, 0x84, 0x0D, 0x01, 0x00, 0x00, 0x48, 0x83, 0x8B, 0x60, 0x02, 0x00, 0x00, 0x10,
            0x48, 0x89, 0x43, 0x68, 0x48, 0x8B, 0x4C, 0x24, 0x50, 0x48, 0x33, 0xCC, 0xE8, 0x19, 0x92, 0x32, 0x00, 0x48, 0x83, 0xC4, 0x60, 0x5B, 0xC3, 0x83,
            0xBB, 0x60, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x8F, 0xE1, 0x00, 0x00, 0x00, 0x48, 0x8B, 0xCB, 0xE8, 0xEE, 0x0D, 0x02, 0x00, 0xF3, 0x0F, 0x10, 0x0D,
            0x86, 0xD4, 0x53, 0x00, 0x48, 0x8D, 0x54, 0x24, 0x40, 0x48, 0x8B, 0xCB, 0x0F, 0x28, 0x00, 0x66, 0x0F, 0x7F, 0x44, 0x24, 0x40, 0xF3, 0x0F, 0x11,
            0x4C, 0x24, 0x44, 0xE8, 0xFA, 0x33, 0x02, 0x00, 0x48, 0x8B, 0x83, 0x08, 0x03, 0x00, 0x00, 0x33, 0xC9, 0x8B, 0x50, 0x5C, 0x48, 0x8B, 0x83, 0xF8,
            0x00, 0x00, 0x00, 0x4C, 0x8B, 0x40, 0x50, 0x4D, 0x85, 0xC0, 0x74, 0x0F, 0x49, 0x63, 0x80, 0x84, 0x00, 0x00, 0x00, 0x0F, 0xBF, 0x8C, 0x43, 0xE8,
            0x02, 0x00, 0x00, 0x48, 0x8B, 0x03, 0x45, 0x33, 0xC9, 0x89, 0x4C, 0x24, 0x30, 0x45, 0x33, 0xC0, 0xC7, 0x44, 0x24, 0x28, 0x00, 0x00, 0x00, 0x00,
            0x48, 0x8B, 0xCB, 0xC7, 0x44, 0x24, 0x20, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x50, 0x70, 0xEB, 0x5F, 0x48, 0x8B, 0xCB, 0xE8, 0x72, 0x0D, 0x02, 0x00,
            0x66, 0x0F, 0x6E, 0x8B, 0x60, 0x01, 0x00, 0x00, 0x0F, 0x5B, 0xC9, 0x0F, 0x28, 0x00, 0xF3, 0x0F, 0x5E, 0x0D, 0x6C, 0xD5, 0x53, 0x00, 0x66, 0x0F,
            0x7F, 0x44, 0x24, 0x40, 0xF3, 0x0F, 0x10, 0x05, 0x66, 0xD3, 0x53, 0x00, 0x0F, 0x2F, 0xC1, 0xF3, 0x0F, 0x11, 0x4C, 0x24, 0x44, 0x76, 0x08, 0xC7,
            0x44, 0x24, 0x44, 0xCD, 0xCC, 0xCC, 0x3D, 0x48, 0x8D, 0x54, 0x24, 0x40, 0x48, 0x8B, 0xCB, 0xE8, 0x5E, 0x33, 0x02, 0x00, 0x83, 0xBB, 0x60, 0x01,
            0x00, 0x00, 0x00, 0x7F, 0x10, 0xC7, 0x83, 0x60, 0x01, 0x00, 0x00, 0x28, 0x23, 0x00, 0x00, 0xFF, 0x83, 0x70, 0x02, 0x00, 0x00, 0x48, 0x8B, 0x4C,
            0x24, 0x50, 0x48, 0x33, 0xCC, 0xE8, 0x18, 0x91, 0x32, 0x00, 0x48, 0x83, 0xC4, 0x60, 0x5B, 0xC3,
    ],
}];

/// The rest of both functions after the moved bytes (not patched).
pub const FLATTEN_CHECKED: &[Site] = &[
    // mov [rcx+0x270],0; mov rbx,rcx; mov [rcx+0x160],250; call 0x1401A2D70 (every hurt body off);
    // mov edx,-1; mov rcx,rbx; call 0x1401A2D90 (every attack body off); the mover's 0x1401D67B0(1);
    // mover +0x124 = 5.0; ret.
    Site {
        name: "flatten state begin: bodies off (0x14017C35B)",
        rva: 0x0017_C35B,
        expect: &[
            0xC7, 0x81, 0x70, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x48, 0x8B, 0xD9, 0xC7, 0x81, 0x60, 0x01, 0x00, 0x00, 0xFA, 0x00, 0x00, 0x00, 0xE8, 0xF9, 0x69, 0x02, 0x00, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x48, 0x8B, 0xCB, 0xE8, 0x0C, 0x6A, 0x02, 0x00, 0x48, 0x8B,
            0x8B, 0xF0, 0x00, 0x00, 0x00, 0xBA, 0x01, 0x00, 0x00, 0x00, 0xE8, 0x1B, 0xA4, 0x05, 0x00, 0x48, 0x8B, 0x83, 0xF0, 0x00, 0x00, 0x00, 0xC7, 0x80, 0x24, 0x01, 0x00, 0x00, 0x00, 0x00, 0xA0, 0x40, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3,
        ],
    },
    // mov rbx,rcx; call 0x1401A2DB0 (every hurt body on); mov edx,-1; mov rcx,rbx; call 0x1401A2DD0
    // (every attack body on); the mover's 0x1401D67B0(0) when `+0x260` bit 8; mover +0x124 = 1.0; ret.
    Site {
        name: "flatten state end: bodies on (0x14017EB8B)",
        rva: 0x0017_EB8B,
        expect: &[
            0x48, 0x8B, 0xD9, 0xE8, 0x1D, 0x42, 0x02, 0x00, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x48, 0x8B, 0xCB, 0xE8, 0x30, 0x42, 0x02, 0x00, 0x8B, 0x83, 0x60, 0x02, 0x00, 0x00, 0x48, 0x0F, 0xBA, 0xE0, 0x08, 0x73, 0x0E, 0x48, 0x8B, 0x8B, 0xF0, 0x00, 0x00, 0x00, 0x33,
            0xD2, 0xE8, 0xF5, 0x7B, 0x05, 0x00, 0x48, 0x8B, 0x83, 0xF0, 0x00, 0x00, 0x00, 0xC7, 0x80, 0x24, 0x01, 0x00, 0x00, 0x00, 0x00, 0x80, 0x3F, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3,
        ],
    },
];

/// Sleights outside battle (owner, 2026-11-04; `crate::field_sleights`, `win::field_sleights`,
/// `docs/recom-field-sleights.md` § Built): the PLM init wrapper (field sleight packs).
pub const FIELD_SLEIGHT_HOOKS: &[Site] = &[
    // mov [rsp+0x18],rbx; push rsi; push rdi; push r14; sub rsp,0x20: PLM::CPlayerManager's init
    // (its ctor `0x14020E1E0`'s only call; no branch into these 13 bytes: exe scan).
    Site { name: "PLM init (0x140210100)", rva: 0x0021_0100, expect: &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x20] },
];

/// Exe bytes the field case patches.
pub const FIELD_SLEIGHT_DATA: &[Site] = &[
    // The Field exe's jump table (`0x1401FA5F4`), entry for action 0x16: the default `0x1401FA599`
    // (an RVA), then 0x17's (the default too, not changed).
    Site { name: "Field exe jump table, action 0x16", rva: 0x001F_A648, expect: &[0x99, 0xA5, 0x1F, 0x00, 0x99, 0xA5, 0x1F, 0x00] },
    // int3 padding after a `pop r14; ret` (`0x1401FB7AF`), before `0x1401FB7C0`: the 13-byte stub.
    Site { name: "code cave 0x1401FB7B2", rva: 0x001F_B7B2, expect: &[0xCC; 14] },
];

/// Re:CoM code the field sleights call or rely on (not patched).
pub const FIELD_SLEIGHT_CHECKED: &[Site] = &[
    // push r15; sub rsp,0x60; mov rax,[rip+…] (the cookie); xor rax,rsp: the resource lookup by
    // name (group, name, case-insensitive), the `CCardBattle` ctor's for both lists. Called.
    Site { name: "resource lookup (0x1402A43E0)", rva: 0x002A_43E0, expect: &[0x41, 0x57, 0x48, 0x83, 0xEC, 0x60, 0x48, 0x8B, 0x05, 0xDB, 0xE6, 0x5B, 0x00, 0x48, 0x33, 0xC4] },
    // The whole function: mov rax,[rcx+8]; test rax,rax; je; mov rax,[rax+8]; ret; ret. Called.
    Site { name: "resource data (0x1402A49C0)", rva: 0x002A_49C0, expect: &[0x48, 0x8B, 0x41, 0x08, 0x48, 0x85, 0xC0, 0x74, 0x05, 0x48, 0x8B, 0x40, 0x08, 0xC3, 0xC3] },
    // The inline `CCardBattle` build in `0x1401607A0`: lookup(1, "BattleCardList", 0) → data →
    // [0x14087A808] (else 0); lookup(1, "StockSkillList", 0) → data → [0x14087A810]: the calls we
    // repeat (the stores are `DAMAGE_CHECKED`'s two sites inside these bytes' tail).
    Site {
        name: "CCardBattle list lookups (0x140160830)",
        rva: 0x0016_0830,
        expect: &[
            0x0C, 0x1A, 0x66, 0x00, 0x8D, 0x4F, 0x01, 0xE8, 0xA4, 0x3B, 0x14, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x11, 0x48, 0x8B, 0xC8, 0xE8, 0x77, 0x41, 0x14, 0x00,
        ],
    },
    // mov rax,[0x14087B110]; mov edx,999; movzx ecx,word [rax+0x187A]; mov eax,2000; sub cx,ax;
    // xor eax,eax; cmp cx,dx; setbe al; ret: a room 2000–2999. Called.
    Site { name: "room 2000-2999 test (0x140235240)", rva: 0x0023_5240, expect: &[0x48, 0x8B, 0x05, 0xC9, 0x5E, 0x64, 0x00, 0xBA, 0xE7, 0x03, 0x00, 0x00, 0x0F, 0xB7, 0x88, 0x7A, 0x18, 0x00, 0x00, 0xB8] },
    // `CGameSubField` state 0: mov ecx,1; call world; cmp eax,10; je; call 0x140235240; test eax,eax;
    // jne; xor ecx,ecx; call 0x140210BD0: the field's ring allocation and its condition, which the
    // wrapper repeats.
    Site {
        name: "field ring allocation (0x1401AF841)",
        rva: 0x001A_F841,
        expect: &[0xB9, 0x01, 0x00, 0x00, 0x00, 0xE8, 0x75, 0x41, 0x08, 0x00, 0x83, 0xF8, 0x0A, 0x74, 0x10, 0xE8, 0xEB, 0x59, 0x08, 0x00, 0x85, 0xC0, 0x75, 0x07, 0x33, 0xC9, 0xE8, 0x70, 0x13, 0x06, 0x00],
    },
    // The PLM init's battle branch: mov [rdi+0x98],rcx (the ring); mov eax,[0x1407C7AE8]; add
    // rax,[0x14087B6C0]; mov [rdi+0xA0],rax; mov eax,[0x1407C7AE8]; mov [rdi+0xA8],eax: the writes
    // the wrapper does for a field PLM.
    Site {
        name: "PLM init ring writes (0x1402103E6)",
        rva: 0x0021_03E6,
        expect: &[
            0x48, 0x89, 0x8F, 0x98, 0x00, 0x00, 0x00, 0x8B, 0x05, 0xF5, 0x76, 0x5B, 0x00, 0x48, 0x03, 0x05, 0xC6, 0xB2, 0x66, 0x00, 0x48, 0x89, 0x87, 0xA0, 0x00, 0x00, 0x00, 0x8B, 0x05, 0xE1, 0x76, 0x5B, 0x00, 0x89,
            0x87, 0xA8, 0x00, 0x00, 0x00,
        ],
    },
    // The player dtor `0x1401E4EB0`'s dim exit: mov eax,[rbx+0x418]; test eax,eax; jns; (white);
    // btr eax,31; mov [rbx+0x418],eax; …; xorps xmm1,xmm1 (0 frames); mov ecx,0x1808; call
    // 0x140232170: a field player destroyed with its sleight's dim on (the battle start cutting it)
    // fades the room back at once. Relied on (not called).
    Site {
        name: "player dtor dim exit (0x1401E51AD)",
        rva: 0x001E_51AD,
        expect: &[
            0x8B, 0x83, 0x18, 0x04, 0x00, 0x00, 0x85, 0xC0, 0x79, 0x30, 0x0F, 0x28, 0x05, 0x92, 0xC2, 0x4D, 0x00, 0x0F, 0x29, 0x44, 0x24, 0x40, 0x0F, 0xBA, 0xF0, 0x1F, 0x89, 0x83, 0x18, 0x04, 0x00, 0x00, 0x89,
            0x6C, 0x24, 0x20, 0x45, 0x33, 0xC9, 0x4C, 0x8D, 0x44, 0x24, 0x40, 0x0F, 0x57, 0xC9, 0xB9, 0x08, 0x18, 0x00, 0x00, 0xE8, 0x8A, 0xCF, 0x04, 0x00,
        ],
    },
    // mov eax,0x1002; ret: CMapEnemy's type getter (vtable `0x14055ACB0` +0x28): the field Heartless
    // our KH1 Thunder / Stop strike in a field room (`win::magic::target_enemy`).
    Site { name: "map enemy type (0x14019D050)", rva: 0x0019_D050, expect: &[0xB8, 0x02, 0x10, 0x00, 0x00, 0xC3] },
];

/// The other lists' sites the field sleights call (the current world, `STATS`'s; exeStockSkill,
/// `SPECIAL_HOOKS`'; the Field exe's switch and default, `MAGIC_CHECKED`'s).
pub const FIELD_SLEIGHT_ALSO: [u32; 4] = [0x0023_39C0, 0x0021_8950, 0x001F_A1B0, 0x001F_A599];

/// CPlayerSora's sleight start slot (vtable `+0x130`), which the summons call through.
pub const SUMMON_PTRS: &[PtrSite] = &[PtrSite { name: "CPlayerSora vtable +0x130 (0x1405A9A68)", rva: 0x005A_9A68, target: 0x0022_47A0 }];

/// M12 follow-ups (Damage = KH1, summons scaled as sleights): the summon actors' vtables, which
/// `win::summons` compares a hit's owner against (`crate::summons::ACTOR_VTABLES`): each one's slot
/// `+0` and its type getter `+0x28`. Checked lazily at the summons' install; a mismatch turns only the
/// summon scaling off (logged). Checked (exe: RTTI and the vtables, relocated pointers).
pub const SUMMON_ACTOR_PTRS: &[PtrSite] = &[
    PtrSite { name: "CSummonDumbo vtable +0 (0x1405AEF98)", rva: 0x005A_EF98, target: 0x0022_ABD0 },
    PtrSite { name: "CSummonDumbo vtable +0x28 (0x1405AEFC0)", rva: 0x005A_EFC0, target: 0x001A_4A30 },
    PtrSite { name: "CSummonBambi vtable +0 (0x1405ACED8)", rva: 0x005A_CED8, target: 0x0022_7EC0 },
    PtrSite { name: "CSummonBambi vtable +0x28 (0x1405ACF00)", rva: 0x005A_CF00, target: 0x001A_4A30 },
    PtrSite { name: "CSummonGenie vtable +0 (0x1405AFFD8)", rva: 0x005A_FFD8, target: 0x0022_B460 },
    PtrSite { name: "CSummonGenie vtable +0x28 (0x1405B0000)", rva: 0x005B_0000, target: 0x001A_4A30 },
    PtrSite { name: "CSummonTink vtable +0 (0x1405B2F68)", rva: 0x005B_2F68, target: 0x0022_DAB0 },
    PtrSite { name: "CSummonTink vtable +0x28 (0x1405B2F90)", rva: 0x005B_2F90, target: 0x001A_4A30 },
    PtrSite { name: "CSummonMushu vtable +0 (0x1405B0FC0)", rva: 0x005B_0FC0, target: 0x0022_C540 },
    PtrSite { name: "CSummonMushu vtable +0x28 (0x1405B0FE8)", rva: 0x005B_0FE8, target: 0x001A_4A30 },
    PtrSite { name: "CSummonSinba vtable +0 (0x1405B1FA8)", rva: 0x005B_1FA8, target: 0x0022_D130 },
    PtrSite { name: "CSummonSinba vtable +0x28 (0x1405B1FD0)", rva: 0x005B_1FD0, target: 0x001A_4A30 },
    PtrSite { name: "CSummonCloud vtable +0 (0x1405ADF70)", rva: 0x005A_DF70, target: 0x0022_8F00 },
    PtrSite { name: "CSummonCloud vtable +0x28 (0x1405ADF98)", rva: 0x005A_DF98, target: 0x001A_4A30 },
];

/// M8d (`win::shop`, `docs/recom-m8-items.md` § M8d): the Moogle shop's pack screen update
/// (`CMoogleShopCore` mode 4, the Items tab) and its card trade update (mode 5, its Square without
/// room for 5 cards), wrapped. Same rule as [`HOOKS`]. Checked (exe: both reached only through the
/// shop's mode records, the ctor's `lea`s at `0x1403A4495` / `0x1403A44EA`; no branch into the
/// moved bytes).
pub const SHOP_HOOKS: &[Site] = &[
    // mov rax,rsp; push rsi; push rdi; push r12; push r14; push r15; sub rsp,0x50
    Site { name: "pack screen update (0x1403A6930)", rva: 0x003A_6930, expect: &[0x48, 0x8B, 0xC4, 0x56, 0x57, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x50] },
    // mov rax,rsp; push rdi; push r14; push r15; sub rsp,0x70
    Site { name: "card trade update (0x1403A73E0)", rva: 0x003A_73E0, expect: &[0x48, 0x8B, 0xC4, 0x57, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x70] },
];

/// Re:CoM functions the Items tab calls: the pack screen's own refresh, the pack class, the shop's
/// cursors and Moogle Point window, the payment's functions, Re:CoM's card picture (`C2DCard`), the
/// floor getter, the text window's text call and the button message ctor (the card trade's
/// Square hint). Checked like the hook sites.
pub const SHOP_CALLEES: &[Site] = &[
    // mov rax,[rip+0x647949] (0x14087B110); movzx eax,byte [rax]; ret
    Site { name: "floor byte (0x1402337C0)", rva: 0x0023_37C0, expect: &[0x48, 0x8B, 0x05, 0x49, 0x79, 0x64, 0x00, 0x0F, 0xB6, 0x00, 0xC3] },
    Site { name: "CTextWnd text (0x1402B0EF0)", rva: 0x002B_0EF0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48] },
    Site { name: "CMShopButtonMessage ctor (0x14045A940)", rva: 0x0045_A940, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x41, 0x8B, 0xD8, 0x48, 0x8B, 0xF9] },
    Site { name: "pack screen refresh (0x1403A92F0)", rva: 0x003A_92F0, expect: &[0x40, 0x56, 0x41, 0x54, 0x48, 0x83, 0xEC, 0x28, 0x48, 0x63, 0x81, 0x80, 0x02, 0x00, 0x00, 0x4C] },
    Site { name: "CMShopCardPack ctor (0x14045A9E0)", rva: 0x0045_A9E0, expect: &[0x40, 0x53, 0x55, 0x56, 0x57, 0x41, 0x56, 0x48, 0x81, 0xEC, 0xA0, 0x00, 0x00, 0x00, 0x48, 0xC7] },
    Site { name: "pack price (0x14045D120)", rva: 0x0045_D120, expect: &[0x48, 0x81, 0xC1, 0x98, 0x00, 0x00, 0x00, 0x41, 0xB8, 0x01, 0x00, 0x00, 0x00, 0xE9, 0xAE, 0xC7, 0xFF, 0xFF] },
    Site { name: "pack sold out (0x14045D3F0)", rva: 0x0045_D3F0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x83, 0x79, 0x70, 0x01, 0x41, 0x8B] },
    Site { name: "shop help line (0x1404573A0)", rva: 0x0045_73A0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x83, 0x79, 0x58, 0x01, 0x48, 0x8B, 0xD9, 0x74, 0x08, 0x33] },
    Site { name: "shop message place (0x140459AF0)", rva: 0x0045_9AF0, expect: &[0x48, 0x83, 0xEC, 0x48, 0x48, 0x8B, 0x05, 0xCD, 0x8F, 0x40, 0x00, 0x48, 0x33, 0xC4, 0x48, 0x89] },
    Site { name: "shop cursor place (0x140459A00)", rva: 0x0045_9A00, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40, 0x48, 0x8B, 0x05, 0xBB, 0x90, 0x40, 0x00, 0x48, 0x33, 0xC4] },
    Site { name: "shop cursor visible (0x1404597D0)", rva: 0x0045_97D0, expect: &[0x48, 0x8B, 0x89, 0xA0, 0x00, 0x00, 0x00, 0xE9, 0xE4, 0xE5, 0xDE, 0xFF] },
    Site { name: "Moogle Point window (0x1404598A0)", rva: 0x0045_98A0, expect: &[0x48, 0x83, 0xC1, 0x78, 0x39, 0x51, 0x14, 0x74, 0x08, 0x89, 0x51, 0x14, 0xE9, 0x8F, 0x0D, 0x00, 0x00, 0xC3] },
    Site { name: "shop show state (0x1404598D0)", rva: 0x0045_98D0, expect: &[0x89, 0x51, 0x60, 0xC3, 0xCC, 0xCC, 0xCC, 0xCC] },
    Site { name: "Moogle Points set (0x14023A1D0)", rva: 0x0023_A1D0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0xF6, 0x05, 0xAC, 0xD1, 0x8B, 0x00, 0x20, 0x8B, 0xD9, 0x75] },
    Site { name: "system variable set (0x1401B16C0)", rva: 0x001B_16C0, expect: &[0x83, 0xF9, 0x02, 0x73, 0x1D, 0x8B, 0xC1, 0x48, 0x8D, 0x0C, 0x40, 0x48, 0x8D, 0x05, 0x0E, 0x9A] },
    Site { name: "trophy unlock (0x1402BA6F0)", rva: 0x002B_A6F0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0x0D, 0xE3, 0x7A, 0x56, 0x00, 0x8B, 0xDA, 0x48] },
    Site { name: "C2DCard ctor (0x1402432A0)", rva: 0x0024_32A0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0xE8, 0xB2, 0x3F, 0x00, 0x00, 0x48, 0x8D] },
    Site { name: "C2DCard set card (0x140243560)", rva: 0x0024_3560, expect: &[0x40, 0x53, 0x56, 0x57, 0x48, 0x83, 0xEC, 0x70, 0x48, 0x8B, 0x05, 0x59, 0xF5, 0x61, 0x00, 0x48] },
    Site { name: "C2DCard priority (0x140243380)", rva: 0x0024_3380, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0xFA, 0x48, 0x8B, 0xD9, 0xE8] },
    Site { name: "C2DCard attach (0x140243DD0)", rva: 0x0024_3DD0, expect: &[0x48, 0x89, 0x91, 0x78, 0x01, 0x00, 0x00, 0xE9, 0x74, 0x48, 0x00, 0x00] },
    Site { name: "C2DCard visible (0x140243A60)", rva: 0x0024_3A60, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0x8B, 0xFA, 0x8B] },
    Site { name: "C2DCard deleting dtor (0x1402073D0)", rva: 0x0020_73D0, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x48] },
    // M8d step 3: the card trade's stock window, its 0–9 box and big card, its charge point
    // window, its finger places, the shop cursors, the text window's message, the help bar, the
    // Moogle Point add.
    Site { name: "stock window fill (0x14045DA80)", rva: 0x0045_DA80, expect: &[0x40, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x81, 0xEC, 0xC0] },
    Site { name: "rank box set (0x14045D9C0)", rva: 0x0045_D9C0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x48] },
    Site { name: "card info set (0x14045A260)", rva: 0x0045_A260, expect: &[0x4C, 0x8B, 0xDC, 0x55, 0x48, 0x81, 0xEC, 0x70, 0x01, 0x00, 0x00, 0x48, 0x8B, 0x05, 0x56, 0x88] },
    Site { name: "charge point window (0x14045D2C0)", rva: 0x0045_D2C0, expect: &[0x48, 0x83, 0xC1, 0x78, 0x41, 0xB8, 0x01, 0x00, 0x00, 0x00, 0xE9, 0x71, 0xC6, 0xFF, 0xFF] },
    Site { name: "trade tab finger place (0x14045B950)", rva: 0x0045_B950, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x50] },
    Site { name: "trade cell finger place (0x14045B8B0)", rva: 0x0045_B8B0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x50, 0x48, 0x8B, 0x05, 0x07, 0x72, 0x40] },
    Site { name: "rank box card spin (0x14045B810)", rva: 0x0045_B810, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x83, 0x79, 0x78, 0x00, 0x48, 0x8B, 0xD9, 0x7E, 0x41, 0x48] },
    Site { name: "shop cursor move (0x140459960)", rva: 0x0045_9960, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x83, 0x79, 0x60, 0x00, 0x48, 0x8B] },
    Site { name: "shop cursor look (0x140459D60)", rva: 0x0045_9D60, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x56, 0x48, 0x83, 0xEC, 0x60, 0x48, 0x63, 0xF2, 0x48, 0x8B, 0xD9] },
    Site { name: "shop text window message (0x1404595A0)", rva: 0x0045_95A0, expect: &[0x40, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x50, 0x48, 0xC7, 0x44, 0x24, 0x40, 0xFE, 0xFF] },
    Site { name: "shop help bar (0x140459CC0)", rva: 0x0045_9CC0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0x83, 0xEA, 0x02, 0x74, 0x60, 0x83, 0xEA] },
    Site { name: "Moogle Points add (0x140239FA0)", rva: 0x0023_9FA0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0xF6, 0x05, 0xDC, 0xD3, 0x8B, 0x00, 0x20, 0x48, 0x8B, 0x05] },
];

/// The pack screen's code the Items tab replicates or relies on (not patched), and its data: the
/// input state's head (the help line, the 0x80 button), the tab row, the grid's confirm (the pack
/// buy), the grid's moves, the payment (state 4), the packs' creation (state 0) and release (state
/// 0x65); the tab / cell places and the move table (`0x14065BA40..0x14065BA7F`, the first 16 bytes
/// written at run time at Combat = KH1 with the five-tab art), the cursor y and the button
/// message's y, the card class rows of the four item cards (`0x14068E0B0 + kind × 16`; their first
/// 8 bytes: the rest holds a relocated pointer); the window's tab play and its group tables
/// (`0x140690420`, written like the tab x), the icons' set-up stores, the card trade's Square and
/// its button's creation, and the card trade's sale sound.
pub const SHOP_CHECKED: &[Site] = &[
    Site {
        name: "pack screen state 2 head (0x1403A6BC9)",
        rva: 0x003A_6BC9,
        expect: &[
            0xE8, 0xD2, 0xEE, 0xFF, 0xFF, 0x8B, 0xD0, 0x48, 0x8B, 0x8F, 0xE8, 0x03, 0x00, 0x00, 0xE8, 0xC4, 0x07, 0x0B, 0x00, 0x83,
            0xBF, 0x74, 0x02, 0x00, 0x00, 0x00, 0x7F, 0x36, 0x33, 0xD2, 0xB9, 0x80, 0x00, 0x00, 0x00, 0xE8, 0xFF, 0x46, 0xEF, 0xFF,
            0x85, 0xC0, 0x74, 0x26, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A, 0x04, 0xE8, 0x5E, 0x37, 0xF0, 0xFF, 0xC7, 0x87, 0x5C,
            0x02, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0xC7, 0x87, 0x60, 0x02, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00, 0xE9, 0x11, 0x07,
            0x00, 0x00,
        ],
    },
    Site {
        name: "pack screen tab row (0x1403A6C1B)",
        rva: 0x003A_6C1B,
        expect: &[
            0x83, 0xBF, 0x4C, 0x03, 0x00, 0x00, 0x00, 0x0F, 0x85, 0x4C, 0x01, 0x00, 0x00, 0x83, 0xBF, 0x74, 0x02, 0x00, 0x00, 0x00,
            0x7F, 0x34, 0x33, 0xC9, 0xE8, 0xD8, 0x47, 0xEF, 0xFF, 0x85, 0xC0, 0x74, 0x29, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A,
            0x05, 0xE8, 0x17, 0x37, 0xF0, 0xFF, 0x41, 0xBE, 0x03, 0x00, 0x00, 0x00, 0x44, 0x89, 0xB7, 0x5C, 0x02, 0x00, 0x00, 0xC7,
            0x87, 0x60, 0x02, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00, 0xE9, 0xC7, 0x06, 0x00, 0x00, 0x33, 0xDB, 0x39, 0x9F, 0x74, 0x02,
            0x00, 0x00, 0x0F, 0x8F, 0xC1, 0x00, 0x00, 0x00, 0x33, 0xD2, 0xB9, 0x00, 0x80, 0x40, 0x00, 0xE8, 0x61, 0x46, 0xEF, 0xFF,
            0x85, 0xC0, 0x74, 0x16, 0x8D, 0x53, 0xFF, 0x8D, 0x4B, 0x01, 0xE8, 0xD2, 0x36, 0xF0, 0xFF, 0xFF, 0x8F, 0x80, 0x02, 0x00,
            0x00, 0xE9, 0x9B, 0x00, 0x00, 0x00, 0x39, 0x9F, 0x74, 0x02, 0x00, 0x00, 0x0F, 0x8F, 0x8F, 0x00, 0x00, 0x00, 0x33, 0xD2,
            0xB9, 0x00, 0x20, 0x80, 0x00, 0xE8, 0x2F, 0x46, 0xEF, 0xFF, 0x85, 0xC0, 0x74, 0x15, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D,
            0x4A, 0x02, 0xE8, 0x9E, 0x36, 0xF0, 0xFF, 0xFF, 0x87, 0x80, 0x02, 0x00, 0x00, 0xEB, 0x6A, 0x39, 0x9F, 0x74, 0x02, 0x00,
            0x00, 0x7F, 0x62, 0x33, 0xD2, 0xB9, 0x00, 0x40, 0x20, 0x00, 0xE8, 0x02, 0x46, 0xEF, 0xFF, 0x85, 0xC0, 0x75, 0x13, 0x39,
            0x9F, 0x74, 0x02, 0x00, 0x00, 0x7F, 0x4A, 0x33, 0xC9, 0xE8, 0x9F, 0x47, 0xEF, 0xFF, 0x85, 0xC0, 0x74, 0x3F, 0xBA, 0xFF,
            0xFF, 0xFF, 0xFF, 0x39, 0x9F, 0x48, 0x03, 0x00, 0x00, 0x7E, 0x28, 0xBE, 0x01, 0x00, 0x00, 0x00, 0x8B, 0xCE, 0xE8, 0x52,
            0x36, 0xF0, 0xFF, 0x89, 0xB7, 0x4C, 0x03, 0x00, 0x00, 0x89, 0x9F, 0x84, 0x02, 0x00, 0x00, 0x33, 0xD2, 0x48, 0x8B, 0x8F,
            0x20, 0x04, 0x00, 0x00, 0xE8, 0xA8, 0x2B, 0x0B, 0x00, 0xEB, 0x0A, 0xB9, 0x05, 0x00, 0x00, 0x00, 0xE8, 0x2C, 0x36, 0xF0,
            0xFF, 0x8B, 0x87, 0x80, 0x02, 0x00, 0x00, 0x83, 0xF8, 0x03, 0x7E, 0x13, 0x89, 0x9F, 0x80, 0x02, 0x00, 0x00, 0x48, 0x8B,
            0xCF, 0xE8, 0xA3, 0x25, 0x00, 0x00, 0xE9, 0xDA, 0x05, 0x00, 0x00, 0x85, 0xC0, 0x0F, 0x89, 0x0D, 0x02, 0x00, 0x00, 0x41,
            0xBE, 0x03, 0x00, 0x00, 0x00, 0x44, 0x89, 0xB7, 0x80, 0x02, 0x00, 0x00, 0x48, 0x8B, 0xCF, 0xE8, 0x81, 0x25, 0x00, 0x00,
            0xE9, 0xB8, 0x05, 0x00, 0x00,
        ],
    },
    Site {
        name: "pack screen grid confirm (0x1403A6D74)",
        rva: 0x003A_6D74,
        expect: &[
            0x83, 0xBF, 0x74, 0x02, 0x00, 0x00, 0x00, 0x0F, 0x8F, 0xD3, 0x00, 0x00, 0x00, 0x33, 0xC9, 0xE8, 0x08, 0x47, 0xEF, 0xFF,
            0x85, 0xC0, 0x0F, 0x84, 0xC4, 0x00, 0x00, 0x00, 0x48, 0x8B, 0x87, 0x50, 0x03, 0x00, 0x00, 0x8B, 0x50, 0x44, 0x83, 0xC2,
            0x05, 0x48, 0x8B, 0x05, 0x44, 0x3A, 0x4D, 0x00, 0x0F, 0xB7, 0x48, 0x06, 0x3B, 0xCA, 0x0F, 0x8C, 0x8A, 0x00, 0x00, 0x00,
            0x48, 0x63, 0x8F, 0x80, 0x02, 0x00, 0x00, 0x48, 0x63, 0x87, 0x84, 0x02, 0x00, 0x00, 0x48, 0x8D, 0x14, 0x88, 0x83, 0xBC,
            0x97, 0x08, 0x03, 0x00, 0x00, 0x00, 0x75, 0x6E, 0x48, 0x8B, 0x05, 0x25, 0x5E, 0x4D, 0x00, 0x8B, 0x48, 0x20, 0x39, 0x8C,
            0x97, 0xC8, 0x02, 0x00, 0x00, 0x7F, 0x5B, 0xBD, 0xFF, 0xFF, 0xFF, 0xFF, 0x8B, 0xD5, 0x41, 0xBE, 0x02, 0x00, 0x00, 0x00,
            0x41, 0x8B, 0xCE, 0xE8, 0x6C, 0x35, 0xF0, 0xFF, 0x8B, 0xD5, 0x8D, 0x4D, 0x59, 0xE8, 0x62, 0x35, 0xF0, 0xFF, 0x33, 0xD2,
            0x48, 0x8B, 0x8F, 0xD0, 0x03, 0x00, 0x00, 0xE8, 0xC4, 0x29, 0x0B, 0x00, 0x41, 0xB8, 0x90, 0x01, 0x00, 0x00, 0x41, 0x8B,
            0xD6, 0x48, 0x8B, 0x8F, 0xA0, 0x03, 0x00, 0x00, 0xE8, 0x3F, 0x77, 0x0B, 0x00, 0x33, 0xD2, 0x48, 0x8B, 0x8F, 0xE8, 0x03,
            0x00, 0x00, 0xE8, 0xD1, 0x05, 0x0B, 0x00, 0xFF, 0x87, 0x60, 0x02, 0x00, 0x00, 0xE9, 0xF2, 0x04, 0x00, 0x00, 0xBA, 0xFF,
            0xFF, 0xFF, 0xFF, 0x8D, 0x4A, 0x06, 0xE8, 0x19, 0x35, 0xF0, 0xFF, 0x48, 0x8B, 0xCF, 0xE8, 0xA1, 0x24, 0x00, 0x00, 0xE9,
            0xD8, 0x04, 0x00, 0x00,
        ],
    },
    Site {
        name: "pack screen grid moves (0x1403A6E54)",
        rva: 0x003A_6E54,
        expect: &[
            0xBD, 0xFF, 0xFF, 0xFF, 0xFF, 0x44, 0x8B, 0xF5, 0x8B, 0x8F, 0x74, 0x02, 0x00, 0x00, 0x8D, 0x75, 0x02, 0x33, 0xDB, 0x85,
            0xC9, 0x0F, 0x8F, 0x8B, 0x00, 0x00, 0x00, 0x33, 0xD2, 0xB9, 0x00, 0x40, 0x20, 0x00, 0xE8, 0x65, 0x44, 0xEF, 0xFF, 0x8B,
            0x8F, 0x74, 0x02, 0x00, 0x00, 0x85, 0xC0, 0x74, 0x05, 0x44, 0x8B, 0xF3, 0xEB, 0x61, 0x85, 0xC9, 0x7F, 0x6C, 0x33, 0xD2,
            0xB9, 0x00, 0x20, 0x80, 0x00, 0xE8, 0x46, 0x44, 0xEF, 0xFF, 0x8B, 0x8F, 0x74, 0x02, 0x00, 0x00, 0x85, 0xC0, 0x74, 0x05,
            0x44, 0x8B, 0xF6, 0xEB, 0x42, 0x85, 0xC9, 0x7F, 0x4D, 0x33, 0xD2, 0xB9, 0x00, 0x10, 0x10, 0x00, 0xE8, 0x27, 0x44, 0xEF,
            0xFF, 0x8B, 0x8F, 0x74, 0x02, 0x00, 0x00, 0x85, 0xC0, 0x74, 0x08, 0x41, 0xBE, 0x02, 0x00, 0x00, 0x00, 0xEB, 0x20, 0x85,
            0xC9, 0x7F, 0x2B, 0x33, 0xD2, 0xB9, 0x00, 0x80, 0x40, 0x00, 0xE8, 0x05, 0x44, 0xEF, 0xFF, 0x8B, 0x8F, 0x74, 0x02, 0x00,
            0x00, 0x85, 0xC0, 0x74, 0x06, 0x41, 0xBE, 0x03, 0x00, 0x00, 0x00, 0x85, 0xC9, 0x7F, 0x0B, 0x33, 0xC9, 0xE8, 0x1A, 0x45,
            0xEF, 0xFF, 0x85, 0xC0, 0x75, 0x22, 0x45, 0x85, 0xF6, 0x78, 0x68, 0x4C, 0x63, 0x87, 0x84, 0x02, 0x00, 0x00, 0x49, 0x63,
            0xC6, 0x4A, 0x8D, 0x14, 0x80, 0x46, 0x0F, 0xBE, 0xB4, 0x22, 0x70, 0xBA, 0x65, 0x00, 0x41, 0x83, 0xFE, 0x40, 0x7C, 0x2A,
            0x8B, 0xD5, 0x8B, 0xCE, 0xE8, 0x3B, 0x34, 0xF0, 0xFF, 0x89, 0x9F, 0x4C, 0x03, 0x00, 0x00, 0x8B, 0xD5, 0x48, 0x8B, 0x8F,
            0x20, 0x04, 0x00, 0x00, 0xE8, 0x97, 0x29, 0x0B, 0x00, 0x48, 0x8B, 0xCF, 0xE8, 0xAF, 0x23, 0x00, 0x00, 0xE9, 0xE6, 0x03,
            0x00, 0x00, 0x45, 0x85, 0xF6, 0x74, 0x1C, 0x43, 0x8D, 0x04, 0x30, 0x3B, 0x87, 0x48, 0x03, 0x00, 0x00, 0x7D, 0x10, 0x8B,
            0xD5, 0x8B, 0xCE, 0xE8, 0x00, 0x34, 0xF0, 0xFF, 0x44, 0x01, 0xB7, 0x84, 0x02, 0x00, 0x00, 0x48, 0x8B, 0xCF, 0xE8, 0x81,
            0x23, 0x00, 0x00, 0xE9, 0xB8, 0x03, 0x00, 0x00,
        ],
    },
    Site {
        name: "pack screen payment (0x1403A7041)",
        rva: 0x003A_7041,
        expect: &[
            0x48, 0x63, 0x8F, 0x80, 0x02, 0x00, 0x00, 0x48, 0x63, 0x87, 0x84, 0x02, 0x00, 0x00, 0x48, 0x8D, 0x0C, 0x88, 0x48, 0x8B,
            0x05, 0x9E, 0x5B, 0x4D, 0x00, 0x8B, 0x50, 0x20, 0x2B, 0x94, 0x8F, 0xC8, 0x02, 0x00, 0x00, 0xBE, 0x9F, 0x86, 0x01, 0x00,
            0x3B, 0xD6, 0x7F, 0x09, 0x8B, 0xF2, 0x33, 0xDB, 0x85, 0xD2, 0x0F, 0x48, 0xF3, 0xF6, 0x05, 0x0C, 0x03, 0x75, 0x00, 0x10,
            0x75, 0x72, 0xBA, 0xB6, 0x00, 0x00, 0x00, 0x33, 0xC9, 0xE8, 0x35, 0x98, 0xE0, 0xFF, 0x44, 0x8B, 0xF0, 0x4C, 0x63, 0x87,
            0x80, 0x02, 0x00, 0x00, 0x48, 0x63, 0x8F, 0x84, 0x02, 0x00, 0x00, 0x4E, 0x8D, 0x04, 0x81, 0x42, 0x8B, 0x9C, 0x87, 0xC8,
            0x02, 0x00, 0x00, 0x03, 0xD8, 0x44, 0x8B, 0xC3, 0xBA, 0xB6, 0x00, 0x00, 0x00, 0x33, 0xC9, 0xE8, 0x07, 0xA6, 0xE0, 0xFF,
            0x81, 0xFB, 0x20, 0x4E, 0x00, 0x00, 0x7C, 0x30, 0x41, 0x81, 0xFE, 0x20, 0x4E, 0x00, 0x00, 0x7D, 0x27, 0x48, 0x8B, 0x0D,
            0x97, 0x03, 0x75, 0x00, 0x48, 0x85, 0xC9, 0x74, 0x0A, 0xBA, 0x63, 0x00, 0x00, 0x00, 0xE8, 0x10, 0x36, 0xF1, 0xFF, 0x0F,
            0xB6, 0x05, 0xA2, 0x02, 0x75, 0x00, 0x0F, 0xBA, 0xE8, 0x04, 0x88, 0x05, 0x98, 0x02, 0x75, 0x00, 0x8B, 0xCE, 0xE8, 0xD8,
            0x30, 0xE9, 0xFF, 0x8B, 0xD6, 0x48, 0x8B, 0x8F, 0xC0, 0x03, 0x00, 0x00, 0xE8, 0x9A, 0x27, 0x0B, 0x00,
        ],
    },
    Site {
        name: "pack screen pack creation (0x1403A6ABC)",
        rva: 0x003A_6ABC,
        expect: &[
            0x8B, 0xF3, 0x39, 0x9F, 0x48, 0x03, 0x00, 0x00, 0x7E, 0x6B, 0x4C, 0x8B, 0xFB, 0x4C, 0x8D, 0xB7, 0x28, 0x04, 0x00, 0x00,
            0xB9, 0xB8, 0x00, 0x00, 0x00, 0xE8, 0x7A, 0x63, 0x10, 0x00, 0x48, 0x89, 0x84, 0x24, 0x80, 0x00, 0x00, 0x00, 0x48, 0x85,
            0xC0, 0x74, 0x33, 0x48, 0x63, 0x8F, 0x80, 0x02, 0x00, 0x00, 0x4D, 0x8D, 0x04, 0x8F, 0x42, 0x8B, 0x8C, 0x87, 0x08, 0x03,
            0x00, 0x00, 0x89, 0x4C, 0x24, 0x20, 0x46, 0x8B, 0x8C, 0x87, 0xC8, 0x02, 0x00, 0x00, 0x44, 0x8B, 0xC6, 0x48, 0x8B, 0x97,
            0x68, 0x02, 0x00, 0x00, 0x48, 0x8B, 0xC8, 0xE8, 0xC8, 0x3E, 0x0B, 0x00, 0xEB, 0x03, 0x48, 0x8B, 0xC3, 0x49, 0x89, 0x06,
        ],
    },
    Site {
        name: "pack screen release (0x1403A72B8)",
        rva: 0x003A_72B8,
        expect: &[
            0x39, 0x9F, 0x48, 0x03, 0x00, 0x00, 0x7E, 0x17, 0x49, 0x8B, 0x06, 0x89, 0x70, 0x6C, 0x49, 0x89, 0x1E, 0xFF, 0xC1, 0x4D,
            0x8D, 0x76, 0x08, 0x3B, 0x8F, 0x48, 0x03, 0x00, 0x00, 0x7C, 0xE9,
        ],
    },
    Site { name: "card class row 0x2A (0x14068E350)", rva: 0x0068_E350, expect: &[0x44, 0x06, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card class row 0x2B (0x14068E360)", rva: 0x0068_E360, expect: &[0x44, 0x06, 0x07, 0x06, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card class row 0x2D (0x14068E380)", rva: 0x0068_E380, expect: &[0x44, 0x07, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00] },
    Site {
        name: "pack window tab play (0x14045D250)",
        rva: 0x0045_D250,
        expect: &[
            0x48, 0x83, 0xEC, 0x48, 0x83, 0x79, 0x58, 0x01, 0x75, 0x55, 0x83, 0x79, 0x60, 0x00, 0x75, 0x4F, 0x39, 0x51, 0x5C, 0x74,
            0x40, 0x4C, 0x8B, 0x05, 0xDC, 0xC9, 0x3F, 0x00, 0x0F, 0x57, 0xC0, 0x89, 0x51, 0x5C, 0x41, 0xB9, 0x01, 0x00, 0x00, 0x00,
            0x48, 0x8B, 0x49, 0x68, 0x48, 0x63, 0xC2, 0x48, 0x8D, 0x15, 0x9A, 0x31, 0x23, 0x00, 0xC7, 0x44, 0x24, 0x30, 0x00, 0x00,
            0x00, 0x00, 0xF3, 0x0F, 0x11, 0x44, 0x24, 0x28, 0x4C, 0x8B, 0x11, 0x0F, 0xBF, 0x14, 0x42, 0x89, 0x54, 0x24, 0x20, 0x33,
            0xD2, 0x41, 0xFF, 0x52, 0x18, 0xB8, 0x01, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC4, 0x48, 0xC3, 0x33, 0xC0, 0x48, 0x83, 0xC4,
            0x48, 0xC3,
        ],
    },
    Site {
        name: "CMShopGSeq set-up groups (0x140457EC1)",
        rva: 0x0045_7EC1,
        expect: &[
            0x0F, 0xB7, 0x84, 0x24, 0xA0, 0x00, 0x00, 0x00, 0x66, 0x89, 0x43, 0x76, 0xB8, 0x01, 0x00, 0x00, 0x00, 0x66, 0x89, 0x7B,
            0x70, 0x66, 0x89, 0x73, 0x72, 0x66, 0x89, 0x6B, 0x74,
        ],
    },
    Site { name: "card class row 0x2F (0x14068E3A0)", rva: 0x0068_E3A0, expect: &[0x44, 0x07, 0x03, 0x02, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "pack window groups (0x140690420)", rva: 0x0069_0420, expect: &[0x42, 0x00, 0x44, 0x00, 0x46, 0x00, 0x48, 0x00, 0x43, 0x00, 0x45, 0x00, 0x47, 0x00, 0x49, 0x00] },
    Site {
        name: "card trade button creation (0x1403A8561)",
        rva: 0x003A_8561,
        expect: &[
            0xB9, 0x88, 0x00, 0x00, 0x00, 0xE8, 0xE9, 0x48, 0x10, 0x00, 0x48, 0x89, 0x44, 0x24, 0x40, 0x48, 0x85, 0xC0, 0x74, 0x18,
            0x41, 0xB8, 0x01, 0x00, 0x00, 0x00, 0x48, 0x8B, 0x97, 0x68, 0x02, 0x00, 0x00, 0x48, 0x8B, 0xC8, 0xE8, 0xB6, 0x23, 0x0B,
            0x00, 0x48, 0x8B, 0xF0, 0x48, 0x89, 0xB7, 0x48, 0x04, 0x00, 0x00, 0x48, 0x8B, 0x06, 0x48, 0x8B, 0xCE, 0xFF, 0x50, 0x38,
            0x33, 0xD2, 0x48, 0x8B, 0x8F, 0x48, 0x04, 0x00, 0x00, 0xE8, 0x25, 0x13, 0x0B, 0x00,
        ],
    },
    Site {
        name: "card trade pack button (0x1403A752A)",
        rva: 0x003A_752A,
        expect: &[
            0x48, 0x8B, 0x05, 0xB7, 0x32, 0x4D, 0x00, 0x0F, 0xB7, 0x50, 0x06, 0x48, 0x8B, 0x87, 0x50, 0x03, 0x00, 0x00, 0x8B, 0x48,
            0x44, 0x83, 0xC1, 0x05, 0x3B, 0xD1, 0x7C, 0x40, 0x83, 0xBF, 0x74, 0x02, 0x00, 0x00, 0x00, 0x7F, 0x37, 0x33, 0xD2, 0xB9,
            0x80, 0x00, 0x00, 0x00, 0xE8, 0x95, 0x3D, 0xEF, 0xFF, 0x85, 0xC0, 0x74, 0x27, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A,
            0x04, 0xE8, 0xF4, 0x2D, 0xF0, 0xFF, 0xBB, 0x04, 0x00, 0x00, 0x00, 0x89, 0x9F, 0x5C, 0x02, 0x00, 0x00, 0xC7, 0x87, 0x60,
            0x02, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00, 0xE9, 0x25, 0x11, 0x00, 0x00,
        ],
    },
    Site { name: "card trade sale sound (0x1403A8095)", rva: 0x003A_8095, expect: &[0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A, 0x07, 0xE8, 0xBE, 0x22, 0xF0, 0xFF] },
    Site {
        name: "pack screen tables (0x14065BA40)",
        rva: 0x0065_BA40,
        expect: &[
            0x00, 0x00, 0x80, 0x42, 0x00, 0x00, 0x00, 0x43, 0x00, 0x00, 0x40, 0x43, 0x00, 0x00, 0x80, 0x43, 0x00, 0x00, 0x94, 0x42,
            0x00, 0x00, 0x60, 0x43, 0x00, 0x00, 0x94, 0x42, 0x00, 0x00, 0x60, 0x43, 0x00, 0x00, 0x27, 0x43, 0x00, 0x00, 0x27, 0x43,
            0x00, 0x00, 0x7C, 0x43, 0x00, 0x00, 0x7C, 0x43, 0x02, 0x01, 0x40, 0x00, 0x02, 0x00, 0x40, 0xFF, 0x00, 0x01, 0xFE, 0x00,
            0x00, 0x00, 0xFE, 0xFF,
        ],
    },
    Site { name: "pack screen cursor y (0x14065BCE0)", rva: 0x0065_BCE0, expect: &[0x00, 0x00, 0x54, 0x42, 0x00, 0x00, 0x8A, 0x42] },
    // M8d step 3: the card trade's tab row, its grid confirm, its yes / no, its sale's points (the
    // Items tab replays them), the ALL finger instruction (patched to the art's place), its two
    // constants, the strip table, the yes / no finger's place.
    Site {
        name: "card trade tab row (0x1403A75A5)",
        rva: 0x003A_75A5,
        expect: &[
            0x39, 0x87, 0x74, 0x02, 0x00, 0x00, 0x0F, 0x8F, 0x0D, 0x02, 0x00, 0x00, 0x33, 0xD2, 0xB9, 0x00, 0x80, 0x40, 0x00, 0xE8,
            0x23, 0x3D, 0xEF, 0xFF, 0x85, 0xC0, 0x74, 0x17, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A, 0x02, 0xE8, 0x92, 0x2D, 0xF0,
            0xFF, 0x8B, 0x9F, 0x5C, 0x03, 0x00, 0x00, 0xFF, 0xCB, 0xEB, 0x32, 0x83, 0xBF, 0x74, 0x02, 0x00, 0x00, 0x00, 0x0F, 0x8F,
            0xD9, 0x01, 0x00, 0x00, 0x33, 0xD2, 0xB9, 0x00, 0x20, 0x80, 0x00, 0xE8, 0xEF, 0x3C, 0xEF, 0xFF, 0x85, 0xC0, 0x74, 0x72,
            0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A, 0x02, 0xE8, 0x5E, 0x2D, 0xF0, 0xFF, 0x8B, 0x9F, 0x5C, 0x03, 0x00, 0x00, 0xFF,
            0xC3, 0x33, 0xF6, 0x83, 0xFB, 0x04, 0x89, 0x9F, 0x5C, 0x03, 0x00, 0x00, 0x7E, 0x0A, 0x8B, 0xDE, 0x89, 0xB7, 0x5C, 0x03,
            0x00, 0x00, 0xEB, 0x0F, 0x85, 0xDB, 0x79, 0x0B, 0xBB, 0x04, 0x00, 0x00, 0x00, 0x89, 0x9F, 0x5C, 0x03, 0x00, 0x00, 0x89,
            0x74, 0x24, 0x20, 0x45, 0x33, 0xC9, 0x44, 0x8B, 0xC3, 0x48, 0x8B, 0x97, 0x50, 0x03, 0x00, 0x00, 0x48, 0x8B, 0x8F, 0x68,
            0x04, 0x00, 0x00, 0xE8, 0x33, 0x64, 0x0B, 0x00, 0x44, 0x8B, 0x87, 0x5C, 0x03, 0x00, 0x00, 0x33, 0xD2, 0x48, 0x8B, 0x8F,
            0x58, 0x04, 0x00, 0x00, 0xE8, 0x5E, 0x63, 0x0B, 0x00, 0xE9, 0x57, 0x01, 0x00, 0x00, 0x83, 0xBF, 0x74, 0x02, 0x00, 0x00,
            0x00, 0x0F, 0x8F, 0x4A, 0x01, 0x00, 0x00, 0x33, 0xD2, 0xB9, 0x00, 0x50, 0x30, 0x00, 0xE8, 0x70, 0x3C, 0xEF, 0xFF, 0x85,
            0xC0, 0x0F, 0x84, 0x36, 0x01, 0x00, 0x00,
        ],
    },
    Site {
        name: "card trade grid confirm (0x1403A7857)",
        rva: 0x003A_7857,
        expect: &[
            0x83, 0xBF, 0x74, 0x02, 0x00, 0x00, 0x00, 0x0F, 0x8F, 0x85, 0x00, 0x00, 0x00, 0x33, 0xC9, 0xE8, 0x25, 0x3C, 0xEF, 0xFF,
            0x85, 0xC0, 0x74, 0x7A, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A, 0x03, 0xE8, 0xE4, 0x2A, 0xF0, 0xFF, 0x44, 0x8B, 0x8F,
            0x64, 0x03, 0x00, 0x00, 0x44, 0x8B, 0x87, 0x60, 0x03, 0x00, 0x00, 0x8B, 0x97, 0x5C, 0x03, 0x00, 0x00, 0x48, 0x8B, 0x8F,
            0x50, 0x03, 0x00, 0x00, 0xE8, 0x54, 0x43, 0x0B, 0x00, 0x48, 0x8B, 0xD0, 0x48, 0x8B, 0x8F, 0x58, 0x04, 0x00, 0x00, 0xE8,
            0xC5, 0x42, 0x0B, 0x00, 0x89, 0x87, 0x68, 0x03, 0x00, 0x00, 0xBA, 0x04, 0x00, 0x00, 0x00, 0x48, 0x8B, 0x8F, 0xD8, 0x03,
            0x00, 0x00, 0xE8, 0x9E, 0x24, 0x0B, 0x00, 0x48, 0x8B, 0x87, 0xE0, 0x03, 0x00, 0x00, 0xC7, 0x40, 0x60, 0x01, 0x00, 0x00,
            0x00, 0xC7, 0x87, 0x74, 0x02, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0xC7, 0x87, 0x60, 0x02, 0x00, 0x00, 0x02, 0x00, 0x00,
            0x00, 0xE9, 0xC2, 0x0D, 0x00, 0x00,
        ],
    },
    Site {
        name: "card trade yes / no open (0x1403A7D5E)",
        rva: 0x003A_7D5E,
        expect: &[
            0x33, 0xC9, 0xE8, 0x2B, 0x37, 0xEF, 0xFF, 0x85, 0xC0, 0x74, 0x78, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x8D, 0x4A, 0x03, 0xE8,
            0xEA, 0x25, 0xF0, 0xFF, 0x33, 0xF6, 0x89, 0xB7, 0x70, 0x02, 0x00, 0x00, 0x8D, 0x56, 0x07, 0x48, 0x8B, 0x8F, 0xE8, 0x03,
            0x00, 0x00, 0xE8, 0x13, 0x18, 0x0B, 0x00, 0x8D, 0x56, 0x06, 0x48, 0x8B, 0x8F, 0xE0, 0x03, 0x00, 0x00, 0xE8, 0xC4, 0x1F,
            0x0B, 0x00, 0x48, 0x63, 0x87, 0x70, 0x02, 0x00, 0x00, 0x45, 0x33, 0xC9, 0xF3, 0x41, 0x0F, 0x10, 0x94, 0x87, 0x38, 0xBA,
            0x65, 0x00, 0xF3, 0x0F, 0x10, 0x0D, 0x34, 0x3F, 0x2B, 0x00, 0x48, 0x8B, 0x8F, 0xD0, 0x03, 0x00, 0x00, 0xE8, 0x3C, 0x1C,
            0x0B, 0x00,
        ],
    },
    Site {
        name: "card trade sale points (0x1403A8295)",
        rva: 0x003A_8295,
        expect: &[
            0xE8, 0x06, 0xCA, 0xDB, 0xFF, 0x8B, 0x4C, 0x24, 0x30, 0xE8, 0x0D, 0x24, 0xDB, 0xFF, 0x8B, 0xC8, 0xE8, 0xF6, 0x1C, 0xE9,
            0xFF, 0x48, 0x8B, 0x05, 0x47, 0x49, 0x4D, 0x00, 0x8B, 0x50, 0x20, 0x48, 0x8B, 0x8F, 0xC0, 0x03, 0x00, 0x00, 0xE8, 0xE0,
            0x15, 0x0B, 0x00,
        ],
    },
    Site { name: "card trade ALL finger (0x14045B9D7)", rva: 0x0045_B9D7, expect: &[0xF3, 0x0F, 0x5C, 0x0D, 0xC9, 0xD8, 0x0A, 0x00] },
    Site { name: "float 50.0 (0x140509298)", rva: 0x0050_9298, expect: &[0x00, 0x00, 0x48, 0x42, 0x00, 0x00, 0x5C, 0x42] },
    Site { name: "float 62.0 (0x1405092A8)", rva: 0x0050_92A8, expect: &[0x00, 0x00, 0x78, 0x42, 0x00, 0x00, 0x80, 0x42] },
    Site { name: "card trade strip groups (0x1406904D0)", rva: 0x0069_04D0, expect: &[0xB6, 0x00, 0xB9, 0x00, 0xB8, 0x00, 0xB7, 0x00, 0xB5, 0x00] },
    Site { name: "yes / no finger y (0x14065BA38)", rva: 0x0065_BA38, expect: &[0x00, 0x00, 0x56, 0x43, 0x00, 0x00, 0x6D, 0x43] },
    Site { name: "yes / no finger x (0x14065BCEC)", rva: 0x0065_BCEC, expect: &[0x00, 0x00, 0x68, 0x43, 0x00, 0x80, 0xD9, 0x43] },
];

/// KH1's item drops from enemies (M8 stage 8a.4, `win::drops`, `docs/recom-m8-items.md` § Stage
/// 8a.4). Same rule as [`HOOKS`].
pub const DROP_HOOKS: &[Site] = &[
    // push rbp; push rsi; push rdi; lea rbp,[rsp-0x47]: the card prize at an enemy's defeat
    // `0x140180D20(enemy, &prize point)`, called last by every defeat path (`0x14017BBC0`,
    // `0x140181300`, `0x1401832F0`) after the EXP / roulette prizes.
    Site { name: "defeat card prize (0x140180D20)", rva: 0x0018_0D20, expect: &[0x40, 0x55, 0x56, 0x57, 0x48, 0x8D, 0x6C, 0x24, 0xB9] },
    // mov [rsp+0x10],rbx; push rdi; sub rsp,0x30: the prize add `0x1401F4AB0(kind, arg, mode)`,
    // which a collected prize's reward `0x140230F90` tail-calls.
    Site { name: "prize add (0x1401F4AB0)", rva: 0x001F_4AB0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x30] },
];

/// The two `lea rax,[rip+…]` that load Re:CoM's prize look table (`0x1405B3150`): retargeted at
/// the same table with the KH1 item row after Re:CoM's 13 (`crate::drops::look_table`).
pub const DROP_DATA: &[Site] = &[
    Site { name: "prize init: look table lea (0x140231676)", rva: 0x0023_1676, expect: &[0x48, 0x8D, 0x05, 0xD3, 0x1A, 0x38, 0x00] },
    Site { name: "prize look set-up: look table lea (0x140230700)", rva: 0x0023_0700, expect: &[0x48, 0x8D, 0x05, 0x49, 0x2A, 0x38, 0x00] },
];

/// Re:CoM functions a drop calls (as the EXP drop `0x1401810A0` does). The entity factory getter
/// `0x1401A8A80` is [`MAGIC_CALLEES`]'.
pub const DROP_CALLEES: &[Site] = &[
    // (The prize factory `0x1401A89C0` is [`GIMMICK_HOOKS`]': a drop calls it through the hook.)
    // push rbx; sub rsp,0x20; movss xmm0,[rcx]; mov rbx,rcx: the angle pair's normaliser.
    Site { name: "angle normalise (0x140293F20)", rva: 0x0029_3F20, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0xF3, 0x0F, 0x10, 0x01, 0x48, 0x8B, 0xD9] },
    // sub rsp,0x28; call 0x14029CDD0; xorps xmm0,xmm0; mov eax,eax; cvtsi2ss xmm0,rax: Re:CoM's
    // random float (u32 × 2^-32, [0, 1)).
    Site { name: "random float (0x14029CF00)", rva: 0x0029_CF00, expect: &[0x48, 0x83, 0xEC, 0x28, 0xE8, 0xC7, 0xFE, 0xFF, 0xFF, 0x0F, 0x57, 0xC0, 0x8B, 0xC0, 0xF3, 0x48, 0x0F, 0x2A, 0xC0] },
];

/// Re:CoM's gimmick cards at Combat = KH1 (`win::gimmick`, `docs/recom-gimmick-cards.md`). Same
/// rule as [`HOOKS`].
pub const GIMMICK_HOOKS: &[Site] = &[
    // push rdi; sub rsp,0x30; mov qword [rsp+0x20],-2: the prize factory
    // `0x1401A89C0(factory, &descriptor, parent)` (0x300-byte CPrize, ctor 0x14022E8D0, init
    // 0x1402315B0; parent: the battle manager task 0x19). Wrapped for the drop log only (the
    // descriptor's `+0x20` kind); KH1's item drops (`win::drops`) call it too.
    Site { name: "prize factory (0x1401A89C0)", rva: 0x001A_89C0, expect: &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF] },
];

/// Re:CoM functions a gimmick card's play calls (`win::gimmick::use_card`, the pickup).
pub const GIMMICK_CALLEES: &[Site] = &[
    // sub rsp,0x68; (cookie); mov r8,[CBTL]; test; je; test rcx,rcx; je; mov eax,[rcx];
    // mov [r8+0x1E4],eax; mov r8,[CBTL]; or dword [r8+0x208],0x8000; test edx,edx; jne (no post):
    // the gimmick card now playing (the revolver play's type-6 step passes (&card, 1)).
    Site {
        name: "CBTL gimmick now (0x140161D60)",
        rva: 0x0016_1D60,
        expect: &[
            0x48, 0x83, 0xEC, 0x68, 0x48, 0x8B, 0x05, 0x5D, 0x0D, 0x70, 0x00, 0x48, 0x33, 0xC4, 0x48, 0x89, 0x44, 0x24, 0x58, 0x4C, 0x8B, 0x05, 0x9E, 0x8A, 0x71, 0x00, 0x4D, 0x85, 0xC0, 0x74, 0x74, 0x48, 0x85, 0xC9, 0x74, 0x10, 0x8B, 0x01, 0x41, 0x89, 0x80, 0xE4, 0x01, 0x00, 0x00, 0x4C,
            0x8B, 0x05, 0x84, 0x8A, 0x71, 0x00, 0x41, 0x81, 0x88, 0x08, 0x02, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x85, 0xD2, 0x75, 0x50,
        ],
    },
    // mov rax,[CBTL]; test; je; xor ecx,ecx; mov [rax+0x1E4],ecx; mov rax,[CBTL];
    // and dword [rax+0x208],0xFFFF7FFF; ret: no gimmick card playing (the undo of a refused play).
    Site {
        name: "CBTL gimmick clear (0x140161D30)",
        rva: 0x0016_1D30,
        expect: &[0x48, 0x8B, 0x05, 0xE1, 0x8A, 0x71, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x19, 0x33, 0xC9, 0x89, 0x88, 0xE4, 0x01, 0x00, 0x00, 0x48, 0x8B, 0x05, 0xCD, 0x8A, 0x71, 0x00, 0x81, 0xA0, 0x08, 0x02, 0x00, 0x00, 0xFF, 0x7F, 0xFF, 0xFF, 0xC3],
    },
    // mov rax,[CBTL]; test; je; add [rax+0x1D0],ecx; ret: Sora's gimmick card count (the revolver
    // add's +1, the used card's −1 `0x1401501B0`).
    Site { name: "CBTL gimmick count add (0x14015D5B0)", rva: 0x0015_D5B0, expect: &[0x48, 0x8B, 0x05, 0x61, 0xD2, 0x71, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x06, 0x01, 0x88, 0xD0, 0x01, 0x00, 0x00, 0xC3] },
    // push rbx; sub rsp,0x20; mov [rcx+0x7FC],edx; mov rbx,rcx; test edx,edx; je; test r8,r8; je;
    // mov r9d,[rcx+0x428]; mov [rsp+0x38],rdi; test r9b,4: Sora's card message handler (vtable
    // +0x48, CBTL's message delivery calls it).
    Site {
        name: "player card message handler (0x1401F5580)",
        rva: 0x001F_5580,
        expect: &[
            0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x89, 0x91, 0xFC, 0x07, 0x00, 0x00, 0x48, 0x8B, 0xD9, 0x85, 0xD2, 0x0F, 0x84, 0xDB, 0x00, 0x00, 0x00, 0x4D, 0x85, 0xC0, 0x0F, 0x84, 0xD2, 0x00, 0x00, 0x00, 0x44, 0x8B, 0x89, 0x28, 0x04, 0x00, 0x00, 0x48, 0x89, 0x7C, 0x24, 0x38, 0x41, 0xF6,
            0xC1, 0x04,
        ],
    },
];

/// Option (c), the card break (owner, 2026-11-01): CBTL's message post, as the card-battle
/// resolver `0x14015EAC0` posts a break: (queue CBTL +0x58, delay 0, from CBTL's handle +0x20, to
/// the enemy's handle, message 3, data).
pub const GIMMICK_BREAK_CALLEES: &[Site] = &[
    // push rbx; mov eax,[rcx+0x60]; mov ebx,r9d; mov r11,rcx; cmp [rcx+0xE0],eax; jge (queue full).
    Site { name: "message post (0x1401D2770)", rva: 0x001D_2770, expect: &[0x40, 0x53, 0x8B, 0x41, 0x60, 0x41, 0x8B, 0xD9, 0x4C, 0x8B, 0xD9, 0x39, 0x81, 0xE0, 0x00, 0x00, 0x00, 0x0F, 0x8D, 0x9A, 0x00, 0x00, 0x00] },
];

/// The enemy side of a card break (not patched, not called).
pub const GIMMICK_BREAK_CHECKED: &[Site] = &[
    // The enemy message handler's case 3 (jump-table entry 2): unless +0x260 & 4, vtable +0x418;
    // +0x260 &= ~0x201; its attack collision +0x108 ends (0x140179D90, 10.0); +0x260 &= ~0x10;
    // next state = enemy + 0x800 (state 13), stored at 0x140181994 (mov [rbx+0x68],rax).
    Site {
        name: "enemy message 3: card broken (0x140181739)",
        rva: 0x0018_1739,
        expect: &[
            0x48, 0x8B, 0x83, 0x60, 0x02, 0x00, 0x00, 0xA8, 0x04, 0x75, 0x13, 0x48, 0x8B, 0x03, 0x48, 0x8B, 0xCB, 0xFF, 0x90, 0x18, 0x04, 0x00, 0x00, 0x48, 0x8B, 0x83, 0x60, 0x02, 0x00, 0x00, 0x48, 0x8B, 0x8B, 0x08, 0x01, 0x00, 0x00, 0x48, 0x25, 0xFE, 0xFD, 0xFF, 0xFF, 0x48,
            0x89, 0x83, 0x60, 0x02, 0x00, 0x00, 0x48, 0x85, 0xC9, 0x74, 0x14, 0xF3, 0x0F, 0x10, 0x0D, 0x7C, 0xFA, 0x53, 0x00, 0xE8, 0x13, 0x86, 0xFF, 0xFF, 0x48, 0x8B, 0x83, 0x60, 0x02, 0x00, 0x00, 0x48, 0x83, 0xE0, 0xEF, 0x48, 0x89, 0x83, 0x60, 0x02, 0x00, 0x00, 0x48, 0x8D, 0x83,
            0x00, 0x08, 0x00, 0x00, 0xE9, 0xF9, 0x01, 0x00, 0x00,
        ],
    },
    // Its jump table (messages 1..12, RVAs): entry 2 (message 3) and 3.
    Site { name: "enemy message table 3..4 (0x1401819B0)", rva: 0x0018_19B0, expect: &[0x39, 0x17, 0x18, 0x00, 0x9B, 0x17, 0x18, 0x00] },
    // The store of the next state: mov [rbx+0x68],rax.
    Site { name: "enemy message: next state (0x140181994)", rva: 0x0018_1994, expect: &[0x48, 0x89, 0x43, 0x68, 0x48, 0x8B, 0x5C, 0x24, 0x30] },
];

/// Trigger (d), the HP drops: the Re:CoM functions a boss's own drop code calls that the DLL calls
/// too (the prize factory, angle normaliser, random float, entity factory and actor position are
/// already checked elsewhere).
pub const GIMMICK_HP_CALLEES: &[Site] = &[
    // mov rax,[CBTL]; test; je; mov [rax+0x200],ecx; ret: the gimmick card's receiver.
    Site { name: "CBTL gimmick receiver set (0x140163740)", rva: 0x0016_3740, expect: &[0x48, 0x8B, 0x05, 0xD1, 0x70, 0x71, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x06, 0x89, 0x88, 0x00, 0x02, 0x00, 0x00, 0xC3] },
    // push rbx; sub rsp,0x20; mov rbx,rcx; mov rcx,[main player]; test; je; test edx,edx: Sora's
    // point (his position, unless +0x420 & 0x40), Dragon Maleficent's aim.
    Site { name: "Sora's point (0x1401F0AB0)", rva: 0x001F_0AB0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0x48, 0x8B, 0x0D, 0xC0, 0xA8, 0x68, 0x00, 0x48, 0x85, 0xC9, 0x74, 0x36, 0x85, 0xD2] },
];

/// Trigger (d): the five bosses' own drop code the HP drops copy (not patched, not called).
pub const GIMMICK_HP_CHECKED: &[Site] = &[
    // Guard Armor: the receiver call, the position (0x1401A49B0) and its y + [0x1406C11D8].
    Site {
        name: "Guard Armor drop: receiver, position (0x1403AB31A)",
        rva: 0x003A_B31A,
        expect: &[
            0xE8, 0x21, 0x84, 0xDB, 0xFF, 0x33, 0xDB, 0x48, 0x8B, 0xCF, 0x89, 0x5C, 0x24, 0x5C, 0xE8, 0x83, 0x96, 0xDF, 0xFF, 0x0F, 0x28, 0x00, 0x0F, 0x29, 0x44, 0x24, 0x30, 0x0F, 0xC6, 0xC0, 0x55, 0xF3, 0x0F, 0x58, 0x05, 0x97, 0x5E, 0x31, 0x00, 0xF3, 0x0F, 0x11, 0x44, 0x24, 0x34,
        ],
    },
    // ... the speed [0x140509234], the random, × [360] × [π/180] as the yaw, the pitch [0x14052BA48],
    // the w [0x1406C10C0].
    Site {
        name: "Guard Armor drop: speed, yaw, pitch (0x1403AB347)",
        rva: 0x003A_B347,
        expect: &[
            0xF3, 0x0F, 0x10, 0x05, 0xE5, 0xDE, 0x15, 0x00, 0xF3, 0x0F, 0x11, 0x44, 0x24, 0x58, 0xE8, 0xA6, 0x1B, 0xEF, 0xFF, 0xF3, 0x0F, 0x59, 0x05, 0xFA, 0xE9, 0x17, 0x00, 0x48, 0x8D, 0x4C, 0x24, 0x40, 0xF3, 0x0F, 0x10, 0x0D, 0xD9, 0x06, 0x18, 0x00, 0xF3, 0x0F, 0x11, 0x4C, 0x24, 0x40,
            0xF3, 0x0F, 0x59, 0x05, 0x17, 0x5A, 0x16, 0x00, 0xF3, 0x0F, 0x10, 0x0D, 0x3B, 0x5D, 0x31, 0x00,
        ],
    },
    // The four kind stores (mov qword [desc+0x20], kind).
    Site { name: "Guard Armor drop: kind 0x3A (0x1403AB3A4)", rva: 0x003A_B3A4, expect: &[0x48, 0xC7, 0x44, 0x24, 0x50, 0x3A, 0x00, 0x00, 0x00] },
    Site { name: "Parasite Cage drop: kind 0x3E (0x1403C2CA7)", rva: 0x003C_2CA7, expect: &[0x48, 0xC7, 0x44, 0x24, 0x60, 0x3E, 0x00, 0x00, 0x00] },
    Site { name: "Trickmaster drop: kind 0x3B (0x1403CB2E4)", rva: 0x003C_B2E4, expect: &[0x48, 0xC7, 0x44, 0x24, 0x60, 0x3B, 0x00, 0x00, 0x00] },
    Site { name: "Dragon drop: kind 0x40 (0x1403E1802)", rva: 0x003E_1802, expect: &[0x48, 0xC7, 0x45, 0xF7, 0x40, 0x00, 0x00, 0x00] },
    // Trickmaster's y + [0x1405455BC] (4.5).
    Site { name: "Trickmaster drop: y add (0x1403CB279)", rva: 0x003C_B279, expect: &[0xF3, 0x0F, 0x58, 0x05, 0x3B, 0xA3, 0x17, 0x00] },
    // The own conditions: Parasite Cage test byte [rdi+0xCD4],2; Trickmaster's table mov ecx,[rax+
    // 0x2C8]; Jafar cmp dword [rcx+0xE5C],0xF; Dragon mov ecx,[rdi+0xD18].
    Site { name: "Parasite Cage drop: used card (0x1403C2BDB)", rva: 0x003C_2BDB, expect: &[0xF6, 0x87, 0xD4, 0x0C, 0x00, 0x00, 0x02] },
    Site { name: "Trickmaster drop: table flags (0x1403CB1F4)", rva: 0x003C_B1F4, expect: &[0x8B, 0x88, 0xC8, 0x02, 0x00, 0x00] },
    Site { name: "Jafar drop: state 15 (0x1403CFE29)", rva: 0x003C_FE29, expect: &[0x83, 0xB9, 0x5C, 0x0E, 0x00, 0x00, 0x0F] },
    Site { name: "Dragon drop: footing (0x1403E16BD)", rva: 0x003E_16BD, expect: &[0x8B, 0x8F, 0x18, 0x0D, 0x00, 0x00] },
    // Jafar's descriptor: position [0x14066F780], angles [0x1406C1430], speed, flags 0x10001, kind
    // 0x3C, the factory.
    Site {
        name: "Jafar drop: descriptor (0x1403CFE75)",
        rva: 0x003C_FE75,
        expect: &[
            0x0F, 0x28, 0x05, 0x04, 0xF9, 0x29, 0x00, 0x0F, 0x28, 0x0D, 0xAD, 0x15, 0x2F, 0x00, 0x0F, 0x29, 0x44, 0x24, 0x20, 0xF3, 0x0F, 0x10, 0x05, 0xA4, 0x93, 0x13, 0x00, 0xF3, 0x0F, 0x11, 0x44, 0x24, 0x48, 0x0F, 0x29, 0x4C, 0x24, 0x30, 0xC7, 0x44, 0x24, 0x4C, 0x01, 0x00, 0x01, 0x00,
            0x48, 0xC7, 0x44, 0x24, 0x40, 0x3C, 0x00, 0x00, 0x00,
        ],
    },
    // Dragon's spread: 0x1401A6530([0x14059D250], [0x140599198]).
    Site { name: "Dragon drop: spread (0x1403E1772)", rva: 0x003E_1772, expect: &[0xF3, 0x0F, 0x10, 0x0D, 0x1E, 0x7A, 0x1B, 0x00, 0xF3, 0x0F, 0x10, 0x05, 0xCE, 0xBA, 0x1B, 0x00, 0xE8, 0xA9, 0x4D, 0xDC, 0xFF] },
    // The constants: 5.0, 4.5, 0.2, −70° in radians, Jafar's point and angles, ±π/4.
    Site { name: "drop constant 5.0 (0x1406C11D8)", rva: 0x006C_11D8, expect: &[0x00, 0x00, 0xA0, 0x40, 0x00, 0x00, 0xC0, 0x40] },
    Site { name: "drop constant 4.5 (0x1405455BC)", rva: 0x0054_55BC, expect: &[0x00, 0x00, 0x90, 0x40, 0x00, 0x00, 0x80, 0x3F] },
    Site { name: "drop constant 0.2 (0x140509234)", rva: 0x0050_9234, expect: &[0xCD, 0xCC, 0x4C, 0x3E, 0xD7, 0xA3, 0xF0, 0x3E] },
    Site { name: "drop constant -70 deg (0x14052BA48)", rva: 0x0052_BA48, expect: &[0xAA, 0x61, 0x9C, 0xBF, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "Jafar drop point (0x14066F780)", rva: 0x0066_F780, expect: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xA0, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x3F] },
    Site { name: "Jafar drop angles (0x1406C1430)", rva: 0x006C_1430, expect: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x3F] },
    Site { name: "drop constant pi/4 (0x140599198)", rva: 0x0059_9198, expect: &[0xDB, 0x0F, 0x49, 0x3F, 0xCD, 0xCC, 0x6C, 0x40] },
    Site { name: "drop constant -pi/4 (0x14059D250)", rva: 0x0059_D250, expect: &[0xDB, 0x0F, 0x49, 0xBF, 0x00, 0x00, 0x00, 0x00] },
];

/// The five card-break bosses' message handler (`+0x48`, Re:CoM's base `0x140181620`) and
/// state-13 begin (`+0x220`, their drop). Checked (exe; relocated pointers).
pub const GIMMICK_BREAK_PTRS: &[PtrSite] = &[
    PtrSite { name: "CBossGuardarmor +0x48", rva: 0x0065_F500, target: 0x0018_1620 },
    PtrSite { name: "CBossGuardarmor +0x220", rva: 0x0065_F6D8, target: 0x003A_B290 },
    PtrSite { name: "CBossTrickMaster +0x48", rva: 0x0066_C640, target: 0x0018_1620 },
    PtrSite { name: "CBossTrickMaster +0x220", rva: 0x0066_C818, target: 0x003C_B1C0 },
    PtrSite { name: "CBossJafar +0x48", rva: 0x0066_E268, target: 0x0018_1620 },
    PtrSite { name: "CBossJafar +0x220", rva: 0x0066_E440, target: 0x003D_0D80 },
    PtrSite { name: "CBossParasiteCage +0x48", rva: 0x0066_9268, target: 0x0018_1620 },
    PtrSite { name: "CBossParasiteCage +0x220", rva: 0x0066_9440, target: 0x003C_2B70 },
    PtrSite { name: "CBossMDragon +0x48", rva: 0x0067_45B8, target: 0x0018_1620 },
    PtrSite { name: "CBossMDragon +0x220", rva: 0x0067_4790, target: 0x003E_1690 },
];

/// The Parasite Cage at Combat = KH1 (owner, 2026-11-03; `crate::battle::cage`): what its armour
/// and its wait rely on. Not patched, not called. Checked (exe).
pub const CAGE_CHECKED: &[Site] = &[
    // Its four attack states end with xor edx,edx; mov r8d,0x1770; …; mov rcx,rbx; call 0x14017E540.
    Site { name: "Parasite Cage state 2 attack end (0x1403C4902)", rva: 0x003C_4902, expect: &[0x33, 0xD2, 0x41, 0xB8, 0x70, 0x17, 0x00, 0x00, 0x48, 0x8B, 0xCB, 0xE8, 0x2E, 0x9C, 0xDB, 0xFF] },
    Site { name: "Parasite Cage state 3 attack end (0x1403C49DD)", rva: 0x003C_49DD, expect: &[0x33, 0xD2, 0x41, 0xB8, 0x70, 0x17, 0x00, 0x00, 0x89, 0x83, 0xD8, 0x0C, 0x00, 0x00, 0x48, 0x8B, 0xCB, 0xE8, 0x4D, 0x9B, 0xDB, 0xFF] },
    Site { name: "Parasite Cage state 4 attack end (0x1403C513E)", rva: 0x003C_513E, expect: &[0x33, 0xD2, 0x41, 0xB8, 0x70, 0x17, 0x00, 0x00, 0x48, 0x8B, 0xCB, 0xE8, 0xF2, 0x93, 0xDB, 0xFF] },
    Site { name: "Parasite Cage state 5 attack end (0x1403C5232)", rva: 0x003C_5232, expect: &[0x33, 0xD2, 0x41, 0xB8, 0x70, 0x17, 0x00, 0x00, 0x48, 0x8B, 0xCB, 0xE8, 0xFE, 0x92, 0xDB, 0xFF] },
    // Its state 1 begin clears bit 21 (mov rax,[0x14052A4B8]; not rax; and [rbx+0x260],rax), and the mask is 0x200000.
    Site { name: "Parasite Cage state 1 begin clears bit 21 (0x1403C2E4B)", rva: 0x003C_2E4B, expect: &[0x48, 0x8B, 0x05, 0x66, 0x76, 0x16, 0x00, 0x48, 0xF7, 0xD0, 0x48, 0x21, 0x83, 0x60, 0x02, 0x00, 0x00] },
    Site { name: "flag mask 0x200000 (0x14052A4B8)", rva: 0x0052_A4B8, expect: &[0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00] },
];

/// `CBossParasiteCage`'s slots the cage relies on: Re:CoM's enemy update (it counts `+0x2C8`), its
/// four attack-state steps (each ends through the attack end) and its hit callback (its faint).
/// Checked (exe; relocated pointers).
pub const CAGE_PTRS: &[PtrSite] = &[
    PtrSite { name: "CBossParasiteCage +0x80", rva: 0x0066_92A0, target: 0x0018_5320 },
    PtrSite { name: "CBossParasiteCage +0x120", rva: 0x0066_9340, target: 0x003C_48F0 },
    PtrSite { name: "CBossParasiteCage +0x138", rva: 0x0066_9358, target: 0x003C_4930 },
    PtrSite { name: "CBossParasiteCage +0x150", rva: 0x0066_9370, target: 0x003C_4A10 },
    PtrSite { name: "CBossParasiteCage +0x168", rva: 0x0066_9388, target: 0x003C_5220 },
    PtrSite { name: "CBossParasiteCage +0x3E0", rva: 0x0066_9600, target: 0x003C_4060 },
];

/// The Parasite Cage's faint only from Fire at Combat = KH1 (owner, 2026-11-03, "Parasite Cage faint /
/// card"): its hit callback `0x1403C4060` (vtable `+0x3E0`, written with ours,
/// `win::battle::cage_hit_callback`, which calls it). Its first bytes and the three reads the rule
/// mirrors. Checked (exe + disasm).
pub const CAGE_FAINT_CHECKED: &[Site] = &[
    // mov r11,rsp; push rbp; push rdi; sub rsp,0xA8; movaps [r11-0x28],xmm6
    Site { name: "Parasite Cage hit callback (0x1403C4060)", rva: 0x003C_4060, expect: &[0x4C, 0x8B, 0xDC, 0x55, 0x57, 0x48, 0x81, 0xEC, 0xA8, 0x00, 0x00, 0x00, 0x41, 0x0F, 0x29, 0x73, 0xD8] },
    // test byte [rcx+0xCD4],4; mov rdi,rcx; mov rax,[rcx+0xF8]; mov rbp,[rax+0x50]: in its faint?, the hit body.
    Site { name: "Parasite Cage hit callback: faint test, hit body (0x1403C4080)", rva: 0x003C_4080, expect: &[0xF6, 0x81, 0xD4, 0x0C, 0x00, 0x00, 0x04, 0x48, 0x8B, 0xF9, 0x48, 0x8B, 0x81, 0xF8, 0x00, 0x00, 0x00, 0x48, 0x8B, 0x68, 0x50] },
    // cmp dword [rbp+0x84],2; jne: Fire (attribute 2) → its FireFaint roll.
    Site { name: "Parasite Cage hit callback: Fire test (0x1403C42B4)", rva: 0x003C_42B4, expect: &[0x83, 0xBD, 0x84, 0x00, 0x00, 0x00, 0x02, 0x75, 0x6F] },
    // cmp dword [rbp+0x80],2; jne: kind 2 (finisher-type) → its AtkFaint roll.
    Site { name: "Parasite Cage hit callback: kind-2 test (0x1403C432C)", rva: 0x003C_432C, expect: &[0x83, 0xBD, 0x80, 0x00, 0x00, 0x00, 0x02, 0x0F, 0x85, 0x5C, 0xFF, 0xFF, 0xFF] },
];

/// Its slot (also in [`CAGE_PTRS`], checked first). Checked (exe; relocated pointer).
pub const CAGE_FAINT_SLOT: PtrSite = PtrSite { name: "CBossParasiteCage +0x3E0 (hit callback)", rva: 0x0066_9600, target: 0x003C_4060 };

/// Re:CoM code the gimmick cards rely on (not patched, not called).
pub const GIMMICK_CHECKED: &[Site] = &[
    // The count getter (−1 without CBTL, else +0x1D0) and the "now playing" getter (byte +0x1E4):
    // the fields the DLL reads.
    Site { name: "CBTL gimmick count (0x140160760)", rva: 0x0016_0760, expect: &[0x48, 0x8B, 0x05, 0xB1, 0xA0, 0x71, 0x00, 0x48, 0x85, 0xC0, 0x75, 0x06, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF, 0xC3, 0x8B, 0x80, 0xD0, 0x01, 0x00, 0x00, 0xC3] },
    Site { name: "CBTL gimmick playing (0x1401616C0)", rva: 0x0016_16C0, expect: &[0x48, 0x8B, 0x05, 0x51, 0x91, 0x71, 0x00, 0x48, 0x85, 0xC0, 0x75, 0x01, 0xC3, 0x0F, 0xB6, 0x80, 0xE4, 0x01, 0x00, 0x00, 0xC3] },
    // The revolver play's event gates: mov edx,8; lea ecx,[rdx-7]; call 0x1401B0870 (flag (1, 8)),
    // then the mask (1, 0xE2) read: Sora's side tests bit 2.
    Site { name: "revolver play: event gates (0x14016CF25)", rva: 0x0016_CF25, expect: &[0xBA, 0x08, 0x00, 0x00, 0x00, 0x8D, 0x4A, 0xF9, 0xE8, 0x3E, 0x39, 0x04, 0x00] },
    // The handler's type 6 case (jump-table entry 6 → 0x1401F58E8): bts r9d,9; mov [rbx+0x428],r9d.
    Site { name: "card handler: type 6 (0x1401F58E8)", rva: 0x001F_58E8, expect: &[0x48, 0x8B, 0x74, 0x24, 0x30, 0x41, 0x0F, 0xBA, 0xE9, 0x09, 0x48, 0x8B, 0x7C, 0x24, 0x38, 0x44, 0x89, 0x8B, 0x28, 0x04, 0x00, 0x00] },
    // Its jump table's entries for types 6 and 7 (RVAs 0x1F58E8, 0x1F5904).
    Site { name: "card handler: type table 6..7 (0x1401F5CEC)", rva: 0x001F_5CEC, expect: &[0xE8, 0x58, 0x1F, 0x00, 0x04, 0x59, 0x1F, 0x00] },
    // The request builder's battle branch: bt eax,9; jae; or dword [rbx+0x424],0x40 (`+0x428`
    // bit 9 → the gimmick request), which the field branch the KH1 Attack takes skips.
    Site { name: "request builder: gimmick (0x1401F85F1)", rva: 0x001F_85F1, expect: &[0x0F, 0xBA, 0xE0, 0x09, 0x73, 0x07, 0x83, 0x8B, 0x24, 0x04, 0x00, 0x00, 0x40] },
    // The move exe's battle branch: test al,0x40; je; xor edx,edx; mov rcx,rbx; add rsp,0x50;
    // pop rbx; jmp 0x1401F25D0 (the gimmick init).
    Site { name: "move exe: gimmick request (0x1401EEE39)", rva: 0x001E_EE39, expect: &[0xA8, 0x40, 0x74, 0x0F, 0x33, 0xD2, 0x48, 0x8B, 0xCB, 0x48, 0x83, 0xC4, 0x50, 0x5B, 0xE9, 0x84, 0x37, 0x00, 0x00] },
    // The gimmick init: action 0x19 (mov qword [rcx+0x40C],0x19).
    Site {
        name: "gimmick init (0x1401F25D0)",
        rva: 0x001F_25D0,
        expect: &[
            0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x81, 0x0C, 0x04, 0x00, 0x00, 0x8B, 0xFA, 0x89, 0x81, 0x08, 0x04, 0x00, 0x00, 0x48, 0x8B, 0xD9, 0x48, 0xC7, 0x81, 0x0C, 0x04, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00,
        ],
    },
    // The gimmick exe (the Battle exe's case 0x19 calls it).
    Site { name: "gimmick exe (0x1401ECA70)", rva: 0x001E_CA70, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x50, 0x48, 0x8B, 0xD9, 0xE8, 0x3E, 0x7D] },
    // The revolver play's type-6 step: the card dword (word = kind, byte 3 = 0x140528C30[kind]
    // byte 0), then 0x140161D60(&card, 1).
    Site {
        name: "revolver play: gimmick step (0x14016D008)",
        rva: 0x0016_D008,
        expect: &[
            0x0F, 0xB6, 0x83, 0x60, 0x02, 0x00, 0x00, 0x48, 0x8D, 0x8C, 0x24, 0x88, 0x00, 0x00, 0x00, 0x89, 0xB4, 0x24, 0x88, 0x00, 0x00, 0x00, 0x66, 0x89, 0x84, 0x24, 0x88, 0x00, 0x00, 0x00, 0x48, 0x03, 0xC0, 0x0F, 0xB6, 0x84, 0xC2, 0x30, 0x8C, 0x52, 0x00, 0xBA, 0x01, 0x00, 0x00, 0x00, 0x88,
            0x84, 0x24, 0x8B, 0x00, 0x00, 0x00, 0xE8, 0x1E, 0x4D, 0xFF, 0xFF,
        ],
    },
    // The prize add's type 5 / 6 branch: the main player [0x14087B390]; its revolver +0x3F0, when
    // set, takes the card (0x140166300); then 0x14010A0C0(4, kind) either way.
    Site {
        name: "prize add: type 5 / 6 (0x1401F4BDA)",
        rva: 0x001F_4BDA,
        expect: &[
            0x48, 0x8B, 0x0D, 0xAF, 0x67, 0x68, 0x00, 0x48, 0x85, 0xC9, 0x74, 0x37, 0x48, 0x8B, 0x89, 0xF0, 0x03, 0x00, 0x00, 0x48, 0x85, 0xC9, 0x74, 0x1A, 0x48, 0x8D, 0x54, 0x24, 0x40, 0x83, 0xFB, 0x03, 0x75, 0x0B, 0x44, 0x8D, 0x43, 0xFE, 0xE8, 0xFB, 0x16, 0xF7, 0xFF, 0xEB,
            0x16, 0xE8, 0xF4, 0x16, 0xF7, 0xFF, 0x0F, 0xB6, 0x54, 0x24, 0x40, 0x45, 0x33, 0xC0, 0x41, 0x8D, 0x48, 0x04, 0xE8, 0xA3, 0x54, 0xF1, 0xFF,
        ],
    },
    // The card-info rows of the eight gimmick cards (`0x140528C30 + kind × 16`, the copy the
    // revolver play reads): type byte 0x66, icon 8 / 6, Sora's and Riku's story (6).
    Site { name: "card info 0x3A Guard Armor (0x140528FD0)", rva: 0x0052_8FD0, expect: &[0x66, 0x08, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x3B Trickmaster (0x140528FE0)", rva: 0x0052_8FE0, expect: &[0x66, 0x08, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x3C Jafar (0x140528FF0)", rva: 0x0052_8FF0, expect: &[0x66, 0x08, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x3D Oogie Boogie (0x140529000)", rva: 0x0052_9000, expect: &[0x66, 0x08, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x3E Parasite Cage (0x140529010)", rva: 0x0052_9010, expect: &[0x66, 0x08, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x3F Ursula (0x140529020)", rva: 0x0052_9020, expect: &[0x66, 0x08, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x40 Dragon Maleficent (0x140529030)", rva: 0x0052_9030, expect: &[0x66, 0x08, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "card info 0x41 Darkside (0x140529040)", rva: 0x0052_9040, expect: &[0x66, 0x08, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00] },
];

/// Sora's card message handler slot (CBTL's message delivery calls `vtable +0x48`). Checked (exe;
/// relocated pointer).
pub const GIMMICK_PTRS: &[PtrSite] = &[PtrSite { name: "CPlayerSora vtable +0x48 (0x1405A9980)", rva: 0x005A_9980, target: 0x001F_5580 }];

/// Re:CoM code the drops rely on (not patched).
pub const DROP_CHECKED: &[Site] = &[
    // The prize init's look index: movsxd rbx,[rdi+0x244]; sub rbx,0xCB; shl rbx,4 (then the lea).
    Site { name: "prize init: look index (0x140231664)", rva: 0x0023_1664, expect: &[0x48, 0x63, 0x9F, 0x44, 0x02, 0x00, 0x00, 0x48, 0x81, 0xEB, 0xCB, 0x00, 0x00, 0x00, 0x48, 0xC1, 0xE3, 0x04] },
    // ... add rbx,rax; movss xmm1,[rbx+0xC] (the row's scale).
    Site { name: "prize init: look row scale (0x14023167D)", rva: 0x0023_167D, expect: &[0x48, 0x03, 0xD8, 0xF3, 0x0F, 0x10, 0x4B, 0x0C] },
    // mov r8d,[rbx+8]; mov rdx,[rbx]: the row's effect id and set name.
    Site { name: "prize init: look row effect (0x1402316D6)", rva: 0x0023_16D6, expect: &[0x44, 0x8B, 0x43, 0x08, 0x48, 0x8B, 0x13] },
    // The look set-up's index around its lea: movsxd rbx,[rcx+0x244] / sub rbx,0xCB.
    Site { name: "prize look set-up: kind (0x1402306F9)", rva: 0x0023_06F9, expect: &[0x48, 0x63, 0x99, 0x44, 0x02, 0x00, 0x00] },
    Site { name: "prize look set-up: index (0x140230707)", rva: 0x0023_0707, expect: &[0x48, 0x81, 0xEB, 0xCB, 0x00, 0x00, 0x00] },
    // The reward: +0x260 = 0 → prize add(kind +0x244, arg +0x248, mode 1) as a tail jump.
    Site {
        name: "prize reward (0x140230F90)",
        rva: 0x0023_0F90,
        expect: &[
            0x48, 0x83, 0xEC, 0x28, 0x83, 0xB9, 0x60, 0x02, 0x00, 0x00, 0x00, 0x75, 0x1B, 0x8B, 0x91, 0x48, 0x02, 0x00, 0x00, 0x41, 0xB8, 0x01, 0x00, 0x00, 0x00, 0x8B, 0x89, 0x44, 0x02, 0x00, 0x00, 0x48, 0x83, 0xC4, 0x28, 0xE9, 0xF8, 0x3A, 0xFC,
            0xFF,
        ],
    },
    // The prize add's switch for kinds 0xCB..=0xD7 (above: `ja` to the default, no reward).
    Site { name: "prize add: special kinds 0xCB..0xD7 (0x1401F503D)", rva: 0x001F_503D, expect: &[0x41, 0x81, 0xC1, 0x35, 0xFF, 0xFF, 0xFF, 0x41, 0x83, 0xF9, 0x0C, 0x0F, 0x87, 0xB8, 0x01, 0x00, 0x00] },
    // The bounce sound (kinds 0xCB..0xD5, else none) and the pickup sound (else sound 6).
    Site { name: "prize bounce sound (0x140230E70)", rva: 0x0023_0E70, expect: &[0x48, 0x83, 0xEC, 0x38, 0x8B, 0x81, 0x44, 0x02, 0x00, 0x00, 0x05, 0x35, 0xFF, 0xFF, 0xFF, 0x83, 0xF8, 0x0A, 0x77, 0x41] },
    Site { name: "prize pickup sound (0x140230F00)", rva: 0x0023_0F00, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x81, 0x44, 0x02, 0x00, 0x00, 0x05, 0x35, 0xFF, 0xFF, 0xFF, 0x83, 0xF8, 0x0A, 0x77, 0x2B] },
    // The EXP drop's descriptor: speed 0.25 at +0x28, flags 1 at +0x2C, the position at +0x00.
    Site { name: "EXP drop descriptor (0x14018112D)", rva: 0x0018_112D, expect: &[0xF3, 0x0F, 0x11, 0x4C, 0x24, 0x48, 0xC7, 0x44, 0x24, 0x4C, 0x01, 0x00, 0x00, 0x00, 0x66, 0x0F, 0x7F, 0x44, 0x24, 0x20] },
];

/// The vtable slot the menu calls through. Checked (exe; relocated pointer).
pub const COMMAND_MENU_PTRS: &[PtrSite] = &[PtrSite { name: "C2DSeqCtrl vtable +0x60 (0x1406073A0)", rva: 0x0060_73A0, target: 0x0024_76F0 }];

/// KH1's cardless cast and Fire (M7 stage 7.4, `win::magic`, `docs/recom-m7-magic.md`). Same rule
/// as [`HOOKS`].
pub const MAGIC_HOOKS: &[Site] = &[
    // mov [rsp+8],rbx; mov [rsp+0x10],rsi: CPlayerSora::exeMagic (vtable +0x150, action 0x13's
    // exe; the Battle exe calls it through the slot, the Field exe's new case directly).
    Site { name: "exeMagic (0x140217EE0)", rva: 0x0021_7EE0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10] },
];

/// Re:CoM functions the cast calls: what `initMagic` and the spawn's Fire case call. Checked like
/// the hook sites.
pub const MAGIC_CALLEES: &[Site] = &[
    // xor eax,eax; mov [rcx],edx; mov [rcx+8],eax; mov [rcx+4],r8d; mov dword [rcx+0xC],0x80000002
    Site { name: "shot descriptor init (0x140156790)", rva: 0x0015_6790, expect: &[0x33, 0xC0, 0x89, 0x11, 0x89, 0x41, 0x08, 0x44, 0x89, 0x41, 0x04, 0xC7, 0x41, 0x0C, 0x02, 0x00, 0x00, 0x80] },
    // push rbx; sub rsp,0x80 (then the cookie): the player's hand point (the weapon's).
    Site { name: "player hand point (0x1401F0B30)", rva: 0x001F_0B30, expect: &[0x40, 0x53, 0x48, 0x81, 0xEC, 0x80, 0x00, 0x00, 0x00] },
    // mov rax,[rcx+0xF0]; add rax,0x50; ret: the actor's angles (its mover's +0x50).
    Site { name: "actor angles (0x1401A4A00)", rva: 0x001A_4A00, expect: &[0x48, 0x8B, 0x81, 0xF0, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC0, 0x50, 0xC3] },
    // push rbx; sub rsp,0x30; mov qword [rsp+0x20],-2: the entity factory (made on first use).
    Site { name: "entity factory (0x1401A8A80)", rva: 0x001A_8A80, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF] },
    // push r14; sub rsp,0x30; mov qword [rsp+0x20],-2; mov [rsp+0x40],rbx: the shot manager.
    Site { name: "shot manager (0x140158DD0)", rva: 0x0015_8DD0, expect: &[0x41, 0x56, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x48, 0x89, 0x5C, 0x24, 0x40] },
    // mov [rsp+8],rbx; push rdi; sub rsp,0x30; mov rbx,rcx; movaps [rsp+0x20],xmm6: the shot's
    // mover set-up (radius, height, floor mode).
    Site { name: "shot mover set-up (0x140157DA0)", rva: 0x0015_7DA0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0x8B, 0xD9, 0x0F, 0x29, 0x74, 0x24, 0x20] },
    // mov [rsp+8],rbx; push rdi; sub rsp,0x20; mov rbx,rcx; xor edi,edi; mov ecx,[rcx+0x84C]:
    // the homing target and its aim point (`+0x854`, `+0x860`).
    Site { name: "homing target (0x1401F68A0)", rva: 0x001F_68A0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0x33, 0xFF, 0x8B, 0x89, 0x4C, 0x08, 0x00, 0x00] },
    // The whole function: the target cursor's "busy" bit (8) and its saved value.
    Site { name: "target cursor busy (0x140209800)", rva: 0x0020_9800, expect: &[0x85, 0xD2, 0x74, 0x14, 0x83, 0x89, 0xF8, 0x00, 0x00, 0x00, 0x08, 0x8B, 0x81, 0xD0, 0x00, 0x00, 0x00, 0x89, 0x81, 0xD8, 0x00, 0x00, 0x00, 0xC3] },
    // 7.4b: mov rax,[rcx]; jmp [rax+0xA8]: the vcall thunk every bullet's flight state runs as
    // its exe; our re-target wrapper calls it (→ CHomingBullet's homing step).
    Site { name: "bullet homing thunk (0x1401560B8)", rva: 0x0015_60B8, expect: &[0x48, 0x8B, 0x01, 0xFF, 0xA0, 0xA8, 0x00, 0x00, 0x00] },
    // 7.5. The whole function: mov rax,[rcx+0x2A8]; mov eax,[rax+0x70]; ret: a shot's hit serial.
    Site { name: "shot hit serial (0x140156770)", rva: 0x0015_6770, expect: &[0x48, 0x8B, 0x81, 0xA8, 0x02, 0x00, 0x00, 0x8B, 0x40, 0x70, 0xC3] },
    // 7.6. The whole function: mov dword [rcx+0x10],0; ret: CCollAttack's "off" (vtable +8).
    Site { name: "CCollAttack off (0x14014DE80)", rva: 0x0014_DE80, expect: &[0xC7, 0x41, 0x10, 0x00, 0x00, 0x00, 0x00, 0xC3] },
    // push rbp; push rbx; push rdi; lea rbp,[rsp-0x47]; sub rsp,0xD0: CCollAttack's apply
    // (vtable +0x38): Re:CoM's hit on a body once its collision pass found the contact.
    Site { name: "CCollAttack apply (0x14016F1C0)", rva: 0x0016_F1C0, expect: &[0x40, 0x55, 0x53, 0x57, 0x48, 0x8D, 0x6C, 0x24, 0xB9, 0x48, 0x81, 0xEC, 0xD0, 0x00, 0x00, 0x00] },
    // 7.7. mov r10d,r8d; mov r11,rdx; mov r8d,[rcx+0x150]; mov edx,r10d; mov rcx,r11 (then jmp
    // 0x1402577D0): an effect of a set on a bone (the Cure / Stop cases, initMagic's charges).
    Site { name: "effect on bone (0x1401A2D50)", rva: 0x001A_2D50, expect: &[0x45, 0x8B, 0xD0, 0x4C, 0x8B, 0xDA, 0x44, 0x8B, 0x81, 0x50, 0x01, 0x00, 0x00, 0x41, 0x8B, 0xD2, 0x49, 0x8B, 0xCB] },
    // sub rsp,0x28; mov edx,ecx; mov rcx,[effect manager]; test rcx,rcx: an effect's object.
    Site { name: "effect object (0x140257EF0)", rva: 0x0025_7EF0, expect: &[0x48, 0x83, 0xEC, 0x28, 0x8B, 0xD1, 0x48, 0x8B, 0x0D, 0x13, 0x50, 0x62, 0x00, 0x48, 0x85, 0xC9] },
    // mov [rsp+8],rbx; mov [rsp+0x10],rbp; mov [rsp+0x18],rsi: Re:CoM's heal (HP += amount).
    // 7.9. mov rcx,[rcx+0xF8]; test rcx,rcx; jnz …; ret: every attack body of an enemy off.
    Site { name: "enemy attack bodies off (0x1401A2D90)", rva: 0x001A_2D90, expect: &[0x48, 0x8B, 0x89, 0xF8, 0x00, 0x00, 0x00, 0x48, 0x85, 0xC9, 0x0F, 0x85, 0x80, 0x26, 0xFB, 0xFF, 0xC3] },
    // 7.10. mov eax,[serial]; mov edx,1; add eax,1; cmovz eax,edx; mov [serial],eax; mov
    // [rcx+0x70],eax; ret: a new hit serial for an attack (Re:CoM's multi-hit attacks call it).
    Site { name: "attack new serial (0x14016F7D0)", rva: 0x0016_F7D0, expect: &[0x8B, 0x05, 0x7A, 0x31, 0x65, 0x00, 0xBA, 0x01, 0x00, 0x00, 0x00, 0x83, 0xC0, 0x01, 0x0F, 0x44, 0xC2, 0x89, 0x05, 0x69, 0x31, 0x65, 0x00, 0x89, 0x41, 0x70, 0xC3] },
    // mov rax,[rcx+0xF0]; add rax,0x30; ret: an actor's position (its mover's +0x30).
    Site { name: "actor position (0x1401A49B0)", rva: 0x001A_49B0, expect: &[0x48, 0x8B, 0x81, 0xF0, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC0, 0x30, 0xC3] },
    Site { name: "player heal (0x1401E59A0)", rva: 0x001E_59A0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x48] },
    // M7 f2. The whole functions: mov rax,[rcx+0xF0]; add rax,0x70; ret (an actor's scale, its
    // mover's +0x70) and mov rax,[rcx+0xF0]; movaps xmm0,[rdx]; movdqa [rax+0x70],xmm0; ret (sets
    // it): what Re:CoM's own Gravity squash writes the height with (0x140183BA0).
    Site { name: "actor scale (0x1401A4A20)", rva: 0x001A_4A20, expect: &[0x48, 0x8B, 0x81, 0xF0, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC0, 0x70, 0xC3] },
    Site { name: "actor scale set (0x1401A7050)", rva: 0x001A_7050, expect: &[0x48, 0x8B, 0x81, 0xF0, 0x00, 0x00, 0x00, 0x0F, 0x28, 0x02, 0x66, 0x0F, 0x7F, 0x40, 0x70, 0xC3] },
];

/// Exe bytes the cast patches or relies on.
pub const MAGIC_DATA: &[Site] = &[
    // The Field exe's jump table (`0x1401FA5F4`, actions 1..=0x1F), entry for action 0x13: the
    // default `0x1401FA599` (an RVA), then the entry for 0x14 (`0x1401FA2D8`, not changed).
    Site { name: "Field exe jump table, action 0x13", rva: 0x001F_A63C, expect: &[0x99, 0xA5, 0x1F, 0x00, 0xD8, 0xA2, 0x1F, 0x00] },
    // int3 padding after a `ret` (`0x1401FB0E0`), before `0x1401FB0F0`: the 13-byte stub.
    Site { name: "code cave 0x1401FB0E1", rva: 0x001F_B0E1, expect: &[0xCC; 15] },
    // Re:CoM's attack rows 0x22 (Fire) and 0x29 / 0x2A (Fira, Firaga), the KH1 rows' templates.
    Site { name: "attack row 0x22 (0x1407C6E90)", rva: 0x007C_6E90, expect: &crate::magic::RECOM_FIRE_ROWS[0] },
    Site { name: "attack rows 0x29-0x2A (0x1407C6F00)", rva: 0x007C_6F00, expect: &[0x37, 0x01, 0x04, 0x00, 0x1E, 0x64, 0x00, 0x00, 0x0B, 0x0F, 0x00, 0x02, 0x0E, 0x14, 0x19, 0x00, 0x37, 0x01, 0x05, 0x00, 0x28, 0x7D, 0x00, 0x00, 0x0C, 0x0F, 0x00, 0x03, 0x0E, 0x14, 0x1E, 0x00] },
    // 7.6: Re:CoM's Thunder rows 0x24 and 0x2E / 0x2F, the KH1 Thunder rows' templates.
    Site { name: "attack row 0x24 (0x1407C6EB0)", rva: 0x007C_6EB0, expect: &crate::magic::RECOM_THUNDER_ROWS[0] },
    Site { name: "attack rows 0x2E-0x2F (0x1407C6F50)", rva: 0x007C_6F50, expect: &[0x39, 0x00, 0x0D, 0x1E, 0x14, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x1D, 0x08, 0x00, 0x00, 0x00, 0x39, 0x00, 0x0E, 0x1E, 0x28, 0x00, 0x00, 0x00, 0x00, 0x28, 0x00, 0x1E, 0x08, 0x00, 0x00, 0x00] },
    // The Thunder case's constants: the bolt effect's drop (−6.0, then −8.0 next to it) and the
    // bolt's pitch (π/2, then 0).
    Site { name: "Thunder case: -6.0 (0x14050932C)", rva: 0x0050_932C, expect: &[0x00, 0x00, 0xC0, 0xC0, 0x00, 0x00, 0x00, 0xC1] },
    Site { name: "Thunder case: pi/2 (0x1406C10F0)", rva: 0x006C_10F0, expect: &[0xDB, 0x0F, 0xC9, 0x3F, 0x00, 0x00, 0x00, 0x00] },
    // 7.5: Re:CoM's Blizzard rows 0x23 and 0x2B / 0x2C, the KH1 Blizzard rows' templates.
    Site { name: "attack row 0x23 (0x1407C6EA0)", rva: 0x007C_6EA0, expect: &crate::magic::RECOM_BLIZZARD_ROWS[0] },
    // 7.7: Re:CoM's Cure rows 0x25 and 0x36 / 0x37, the KH1 Cure rows' templates; initMagic's charge
    // offset (−0.8, 0, 0, 1) and the Cure case's body offset (0, 0.85, 0, 1).
    Site { name: "attack row 0x25 (0x1407C6EC0)", rva: 0x007C_6EC0, expect: &crate::magic::RECOM_CURE_ROWS[0] },
    Site { name: "attack rows 0x36-0x37 (0x1407C6FD0)", rva: 0x007C_6FD0, expect: &[0x39, 0x00, 0x10, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x00, 0x39, 0x00, 0x11, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "initMagic charge offset (0x1407C7E50)", rva: 0x007C_7E50, expect: &[0xCD, 0xCC, 0x4C, 0xBF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x3F] },
    Site { name: "Cure case body offset (0x1405A0910)", rva: 0x005A_0910, expect: &[0x00, 0x00, 0x00, 0x00, 0x9A, 0x99, 0x59, 0x3F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x3F] },
    // 7.8: Re:CoM's Gravity rows 0x26 and 0x38 / 0x39, the KH1 Gravity rows' templates; the Gravity
    // case's descriptor `+0xB8` data.
    Site { name: "attack row 0x26 (0x1407C6ED0)", rva: 0x007C_6ED0, expect: &crate::magic::RECOM_GRAVITY_ROWS[0] },
    Site { name: "attack rows 0x38-0x39 (0x1407C6FF0)", rva: 0x007C_6FF0, expect: &[0x3A, 0x00, 0x13, 0x20, 0x96, 0x06, 0x00, 0x00, 0x00, 0x05, 0x00, 0x0D, 0x08, 0x14, 0x32, 0x00, 0x3A, 0x00, 0x14, 0x20, 0x96, 0x08, 0x00, 0x00, 0x00, 0x05, 0x00, 0x0D, 0x08, 0x1E, 0x32, 0x00] },
    Site { name: "Gravity case desc +0xB8 (0x1407C78D0)", rva: 0x007C_78D0, expect: &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x3F] },
    // 7.9: Re:CoM's Stop rows 0x27 and 0x3A / 0x3B, the KH1 Stop rows' templates.
    Site { name: "attack row 0x27 (0x1407C6EE0)", rva: 0x007C_6EE0, expect: &crate::magic::RECOM_STOP_ROWS[0] },
    Site { name: "attack rows 0x3A-0x3B (0x1407C7010)", rva: 0x007C_7010, expect: &[0x39, 0x00, 0x16, 0x1E, 0x14, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x08, 0x08, 0x00, 0x08, 0x00, 0x39, 0x00, 0x17, 0x1E, 0x28, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x08, 0x08, 0x00, 0x08, 0x00] },
    // 7.10: Re:CoM's Aero rows 0x28 and 0x3C / 0x3D, the KH1 Aero rows' templates; the Aero case's
    // pitch (−π/2).
    Site { name: "attack row 0x28 (0x1407C6EF0)", rva: 0x007C_6EF0, expect: &crate::magic::RECOM_AERO_ROWS[0] },
    Site { name: "attack rows 0x3C-0x3D (0x1407C7030)", rva: 0x007C_7030, expect: &[0x39, 0x00, 0x19, 0x1E, 0x14, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x02, 0x08, 0x00, 0x0A, 0x00, 0x39, 0x00, 0x1A, 0x1E, 0x1E, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x03, 0x08, 0x00, 0x0A, 0x00] },
    Site { name: "Aero case: -pi/2 (0x140520910)", rva: 0x0052_0910, expect: &[0xDB, 0x0F, 0xC9, 0xBF, 0x00, 0x00, 0x00, 0x00] },
    Site { name: "attack rows 0x2B-0x2C (0x1407C6F20)", rva: 0x007C_6F20, expect: &[0x3A, 0x01, 0x08, 0x1E, 0x0F, 0x4B, 0x00, 0x00, 0x00, 0x14, 0x00, 0x02, 0x0A, 0x00, 0x0F, 0x00, 0x3A, 0x01, 0x0A, 0x1E, 0x14, 0x64, 0x00, 0x00, 0x00, 0x14, 0x00, 0x03, 0x0A, 0x00, 0x14, 0x00] },
];

/// Re:CoM code the cast's routing relies on (not patched).
pub const MAGIC_CHECKED: &[Site] = &[
    // The Field exe's switch: mov eax,[rbx+0x40C]; dec eax; cmp eax,0x1E; ja 0x1401FA599; lea
    // r8,[image base]; cdqe; mov ecx,[r8+rax*4+0x1FA5F4]: the player is rbx, the table above.
    Site { name: "Field exe switch (0x1401FA1B0)", rva: 0x001F_A1B0, expect: &[0x8B, 0x83, 0x0C, 0x04, 0x00, 0x00, 0xFF, 0xC8, 0x83, 0xF8, 0x1E, 0x0F, 0x87, 0xD8, 0x03, 0x00, 0x00, 0x4C, 0x8D, 0x05, 0x38, 0x5E, 0xE0, 0xFF, 0x48, 0x98, 0x41, 0x8B, 0x8C, 0x80, 0xF4, 0xA5, 0x1F, 0x00] },
    // The Field exe's default (`mov ecx,[rbx+0x460]; sub [rbx+0x7BC],ecx`): the timers after the
    // action step, where every case goes on.
    Site { name: "Field exe after the action (0x1401FA599)", rva: 0x001F_A599, expect: &[0x8B, 0x8B, 0x60, 0x04, 0x00, 0x00, 0x29, 0x8B, 0xBC, 0x07, 0x00, 0x00] },
    // The Battle exe's case 0x13: mov rax,[rdi]; mov rcx,rdi; call [rax+0x150] (exeMagic).
    Site { name: "Battle exe case 0x13 (0x1401F9B4D)", rva: 0x001F_9B4D, expect: &[0x48, 0x8B, 0x07, 0x48, 0x8B, 0xCF, 0xFF, 0x90, 0x50, 0x01, 0x00, 0x00] },
    // 7.4b. CHomingBullet's homing step (push rbx; sub rsp,0x50; the cookie; cmp dword
    // [rcx+0x2F0],0): reached through the thunk; turns toward `+0x2F4`'s aim point.
    Site { name: "CHomingBullet homing step (0x1401575D0)", rva: 0x0015_75D0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x50, 0x48, 0x8B, 0x05, 0xEB, 0xB4, 0x70, 0x00, 0x48, 0x33, 0xC4, 0x48, 0x89, 0x44, 0x24, 0x30, 0x83, 0xB9, 0xF0, 0x02, 0x00, 0x00, 0x00] },
    // The bullet init's flight state: lea rax,[0x1401560B8] (the exe), and movups [rsi+0x2D0],
    // xmm1 (the exe member pointer into the state object at +0x2B0).
    Site { name: "bullet init: exe thunk (0x1401585D5)", rva: 0x0015_85D5, expect: &[0x48, 0x8D, 0x05, 0xDC, 0xDA, 0xFF, 0xFF] },
    Site { name: "bullet init: exe store +0x2D0 (0x140158609)", rva: 0x0015_8609, expect: &[0x0F, 0x11, 0x8E, 0xD0, 0x02, 0x00, 0x00] },
    // CStateTemplate<CBullet>'s exe (its vtable 0x140525408 +0x10): rcx = owner + adjust, jmp
    // [state+0x20]. So the member pointer at +0x2D0 is called with the bullet.
    Site { name: "CStateTemplate<CBullet> exe (0x140156740)", rva: 0x0015_6740, expect: &[0x48, 0x8B, 0x51, 0x20, 0x48, 0x8B, 0xC1, 0x48, 0x85, 0xD2, 0x74, 0x0B, 0x48, 0x63, 0x49, 0x28, 0x48, 0x03, 0x48, 0x08, 0x48, 0xFF, 0xE2, 0xC3] },
    // Re:CoM's own nearest pick for homing shots: mov r14,[0x14087B1E0] (the LockOnManager),
    // then its list walk: rbx = [r14+0x88]; stop at the node with no next; handle → object;
    // skip null / the shot; test [obj+0x154],0x10000.
    Site { name: "nearest pick: lock-on manager (0x1401CDBCF)", rva: 0x001C_DBCF, expect: &[0x4C, 0x8B, 0x35, 0x0A, 0xD6, 0x6A, 0x00] },
    // 7.5. Re:CoM's Blizzard case: call 0x140156770 on the first shard; mov [rbp-0x3C],eax (its
    // descriptor's +0x74: the serial every later shard gets) ...
    Site { name: "Blizzard case: shared serial (0x1402224A7)", rva: 0x0022_24A7, expect: &[0xE8, 0xC4, 0x42, 0xF3, 0xFF, 0x89, 0x45, 0xC4] },
    // ... and lea rcx,[0x1405A1790]: the side shards' effect sets (the row's set + 1).
    Site { name: "Blizzard case: side-shard sets (0x1402224E9)", rva: 0x0022_24E9, expect: &[0x48, 0x8D, 0x0D, 0xA0, 0xF2, 0x37, 0x00] },
    // 7.6. Re:CoM's collision pass (`0x14016EAD0`) after its overlap test: mov rax,[rsi]; mov
    // rdx,rbx; mov rcx,rsi; call [rax+0x38]: the attack's apply on the body, the call a bolt's
    // direct strike makes.
    Site { name: "collision pass: apply call (0x14016EC51)", rva: 0x0016_EC51, expect: &[0x48, 0x8B, 0x06, 0x48, 0x8B, 0xD3, 0x48, 0x8B, 0xCE, 0xFF, 0x50, 0x38] },
    // The bullet init's "collision held" (flag bit 21 with +0x268 > 0: the attack's "off"), read
    // before +0x268 is filled from desc+0x8C: so a bolt's attack is also switched off by us.
    Site { name: "bullet init: collision held (0x1401583E8)", rva: 0x0015_83E8, expect: &[0x8B, 0x86, 0x30, 0x02, 0x00, 0x00, 0x48, 0x0F, 0xBA, 0xE0, 0x15, 0x73, 0x16, 0x83, 0xBE, 0x68, 0x02, 0x00, 0x00, 0x00, 0x7E, 0x0D, 0x48, 0x8B, 0x8E, 0xA8, 0x02, 0x00, 0x00, 0x48, 0x8B, 0x01, 0xFF, 0x50, 0x08] },
    // Re:CoM's Thunder case: the hand effect (id 1, Sora's angles as the rotation, the set's +0x18
    // name, 0x1401A6B20) ...
    Site { name: "Thunder case: hand effect (0x14022276C)", rva: 0x0022_276C, expect: &[0x4C, 0x8D, 0x4D, 0x30, 0x41, 0xB8, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x28, 0x00, 0x48, 0x8B, 0x87, 0xC8, 0x03, 0x00, 0x00, 0x66, 0x0F, 0x7F, 0x45, 0x40, 0x0F, 0xB6, 0x48, 0x02, 0x48, 0x8D, 0x45, 0x40, 0x48, 0x89, 0x44, 0x24, 0x20, 0x48, 0x8D, 0x14, 0x49, 0x48, 0x8B, 0xCF, 0x48, 0xC1, 0xE2, 0x05, 0x48, 0x8B, 0x54, 0x1A, 0x18, 0xE8, 0x79, 0x43, 0xF8, 0xFF] },
    // ... the bolt effect 6 m under Sora's height (addss xmm0,[−6.0]), its id set − 10 (lea
    // r8d,[rcx−10]) ...
    Site { name: "Thunder case: bolt effect drop (0x1402227F3)", rva: 0x0022_27F3, expect: &[0xF3, 0x0F, 0x58, 0x05, 0x31, 0x6B, 0x2E, 0x00] },
    Site { name: "Thunder case: bolt effect id (0x140222820)", rva: 0x0022_2820, expect: &[0x44, 0x8D, 0x41, 0xF6, 0x48, 0x8B, 0xCF] },
    // ... the bolt: pitch π/2 (movss xmm1,[π/2]), flags 0x47, life 800.
    Site { name: "Thunder case: bolt pitch (0x140222867)", rva: 0x0022_2867, expect: &[0xF3, 0x0F, 0x10, 0x0D, 0x81, 0xE8, 0x49, 0x00] },
    Site { name: "Thunder case: bolt flags (0x140222898)", rva: 0x0022_2898, expect: &[0x48, 0xC7, 0x45, 0xB0, 0x47, 0x00, 0x00, 0x00] },
    Site { name: "Thunder case: bolt life (0x1402228BC)", rva: 0x0022_28BC, expect: &[0xC7, 0x45, 0x8C, 0x20, 0x03, 0x00, 0x00] },
    // 7.7. Re:CoM's Cure case (`0x140222170` case 0x25), whole: the offset (movaps [0x1405A0910]),
    // the bone 0x1402ABB20(player, 0), the set's +0x18 name, id set − 14 (lea r8d,[rbx−0xE]), the
    // effect 0x1401A2D50, its object 0x140257EF0 |= 4, then the heal 0x1401E59A0(player, [+0x660],
    // 0, 1, 0).
    // M7 f4. initMagic's Thunder charge (case 0x1C): mov rcx,[rdi+0x3D0] (the weapon); bone 2
    // (0x1402ABB20), the offset [0x1407C7E50], the set's +0x18 name ([rdx+r14+0x5A1748]), id 0,
    // 0x1401A2D50, its object 0x140257EF0 and `or dword [rax+0x4C],4` (kept on the Keyblade).
    Site { name: "initMagic: Thunder charge (0x14022062B)", rva: 0x0022_062B, expect: &[0x48, 0x8B, 0x8F, 0xD0, 0x03, 0x00, 0x00, 0xBA, 0x02, 0x00, 0x00, 0x00, 0xF3, 0x0F, 0x11, 0x87, 0x04, 0x05, 0x00, 0x00, 0xE8, 0xDC, 0xB4, 0x08, 0x00, 0x4C, 0x8B, 0xC8, 0x0F, 0x57, 0xC0, 0x48, 0x8B, 0x87, 0xC8, 0x03, 0x00, 0x00, 0x45, 0x33, 0xC0, 0xF3, 0x0F, 0x11, 0x44, 0x24, 0x30, 0x48, 0x89, 0x6C, 0x24, 0x28, 0x0F, 0xB6, 0x48, 0x02, 0x48, 0x8D, 0x05, 0xE6, 0x77, 0x5A, 0x00, 0x48, 0x89, 0x44, 0x24, 0x20, 0x48, 0x8D, 0x14, 0x49, 0x48, 0x8B, 0xCF, 0x48, 0xC1, 0xE2, 0x05, 0x4A, 0x8B, 0x94, 0x32, 0x48, 0x17, 0x5A, 0x00, 0xE8, 0xC9, 0x26, 0xF8, 0xFF, 0x8B, 0xC8, 0xE8, 0x62, 0x78, 0x03, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x04, 0x83, 0x48, 0x4C, 0x04] },
    Site { name: "Cure case (0x140222946)", rva: 0x0022_2946, expect: &[0x48, 0x8B, 0x87, 0xC8, 0x03, 0x00, 0x00, 0x33, 0xD2, 0x0F, 0x28, 0x05, 0xBA, 0xDF, 0x37, 0x00, 0x48, 0x8B, 0xCF, 0x0F, 0x29, 0x45, 0x30, 0x0F, 0xB6, 0x58, 0x02, 0xE8, 0xBA, 0x91, 0x08, 0x00, 0x48, 0x8D, 0x14, 0x5B, 0x0F, 0x57, 0xC0, 0xF3, 0x0F, 0x11, 0x44, 0x24, 0x30, 0x44, 0x8D, 0x43, 0xF2, 0x48, 0xC1, 0xE2, 0x05, 0x48, 0x8D, 0x1D, 0xAE, 0xED, 0x37, 0x00, 0x4C, 0x8B, 0xC8, 0x45, 0x33, 0xF6, 0x48, 0x8D, 0x45, 0x30, 0x4C, 0x89, 0x74, 0x24, 0x28, 0x48, 0x8B, 0xCF, 0x48, 0x89, 0x44, 0x24, 0x20, 0x48, 0x8B, 0x54, 0x1A, 0x18, 0xE8, 0xAD, 0x03, 0xF8, 0xFF, 0x8B, 0xC8, 0xE8, 0x46, 0x55, 0x03, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x04, 0x83, 0x48, 0x4C, 0x04, 0x8B, 0x97, 0x60, 0x06, 0x00, 0x00, 0x41, 0xB9, 0x01, 0x00, 0x00, 0x00, 0x45, 0x33, 0xC0, 0x44, 0x89, 0x74, 0x24, 0x20, 0x48, 0x8B, 0xCF, 0xE8, 0xD1, 0x2F, 0xFC, 0xFF] },
    // initMagic's charge for Cure (case 0x1D, shared by Aero): the weapon's bone 2, id 0, the
    // offset [0x1407C7E50], the set's +0x18 name, 0x1401A2D50.
    Site { name: "initMagic: Cure charge (0x14022070A)", rva: 0x0022_070A, expect: &[0xBA, 0x02, 0x00, 0x00, 0x00, 0xE8, 0x0C, 0xB4, 0x08, 0x00, 0x4C, 0x8B, 0xC8, 0x0F, 0x57, 0xC0, 0x48, 0x8B, 0x87, 0xC8, 0x03, 0x00, 0x00, 0x45, 0x33, 0xC0, 0xF3, 0x0F, 0x11, 0x44, 0x24, 0x30, 0x48, 0x89, 0x6C, 0x24, 0x28, 0x0F, 0xB6, 0x48, 0x02, 0x48, 0x8D, 0x05, 0x16, 0x77, 0x5A, 0x00, 0x48, 0x89, 0x44, 0x24, 0x20, 0x48, 0x8D, 0x14, 0x49, 0x48, 0x8B, 0xCF, 0x48, 0xC1, 0xE2, 0x05, 0x4A, 0x8B, 0x94, 0x32, 0x48, 0x17, 0x5A, 0x00, 0xE8, 0xF9, 0x25, 0xF8, 0xFF] },
    // 7.8. initMagic's Gravity charge (case 0x1E): the weapon's bone 2, id 0, the offset
    // [0x1407C7E50], the set's +0x30 name (`[rdx+r14+0x5A1760]`), 0x1401A2D50.
    Site { name: "initMagic: Gravity charge (0x1402207A5)", rva: 0x0022_07A5, expect: &[0x48, 0x8B, 0x8F, 0xD0, 0x03, 0x00, 0x00, 0xE8, 0x6F, 0xB3, 0x08, 0x00, 0x4C, 0x8B, 0xC8, 0x0F, 0x57, 0xF6, 0x48, 0x8B, 0x87, 0xC8, 0x03, 0x00, 0x00, 0x45, 0x33, 0xC0, 0xF3, 0x0F, 0x11, 0x74, 0x24, 0x30, 0x48, 0x89, 0x6C, 0x24, 0x28, 0x0F, 0xB6, 0x48, 0x02, 0x48, 0x8D, 0x05, 0x79, 0x76, 0x5A, 0x00, 0x48, 0x89, 0x44, 0x24, 0x20, 0x48, 0x8D, 0x14, 0x49, 0x48, 0x8B, 0xCF, 0x48, 0xC1, 0xE2, 0x05, 0x4A, 0x8B, 0x94, 0x32, 0x60, 0x17, 0x5A, 0x00, 0xE8, 0x5C, 0x25, 0xF8, 0xFF] },
    // Re:CoM's Gravity case (`0x140222170` case 0x26): the hand effect (id 1, the set's +0x30
    // name, 0x1401A6B20) ...
    Site { name: "Gravity case: hand effect (0x1402229F8)", rva: 0x0022_29F8, expect: &[0x4C, 0x8D, 0x4D, 0x30, 0x41, 0xB8, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x28, 0x00, 0x48, 0x8B, 0x87, 0xC8, 0x03, 0x00, 0x00, 0x66, 0x0F, 0x7F, 0x45, 0x40, 0x0F, 0xB6, 0x48, 0x02, 0x48, 0x8D, 0x45, 0x40, 0x48, 0x89, 0x44, 0x24, 0x20, 0x48, 0x8D, 0x14, 0x49, 0x48, 0x8B, 0xCF, 0x48, 0xC1, 0xE2, 0x05, 0x48, 0x8B, 0x54, 0x1A, 0x30, 0xE8, 0xED, 0x40, 0xF8, 0xFF] },
    // ... the sphere's flags 0x41E1 and its descriptor +0xB8.
    Site { name: "Gravity case: flags (0x140222AB7)", rva: 0x0022_2AB7, expect: &[0x48, 0xC7, 0x45, 0xB0, 0xE1, 0x41, 0x00, 0x00] },
    Site { name: "Gravity case: desc +0xB8 (0x140222B09)", rva: 0x0022_2B09, expect: &[0x48, 0x8D, 0x05, 0xC0, 0x4D, 0x5A, 0x00] },
    // 7.9. Re:CoM's kind specials, case 8 (Stop): mov rax,[rbx+0x308]; cmp dword [rax+0x2C],0 (the
    // gate KH1's resist[9] replaces); je …; cvttss2si eax,xmm6; mov [rbx+0x2D8],eax (the value).
    Site { name: "kind specials: Stop gate (0x14017DCB5)", rva: 0x0017_DCB5, expect: &[0x48, 0x8B, 0x83, 0x08, 0x03, 0x00, 0x00, 0x83, 0x78, 0x2C, 0x00, 0x0F, 0x84, 0x46, 0x02, 0x00, 0x00, 0xF3, 0x0F, 0x2C, 0xC6, 0x89, 0x83, 0xD8, 0x02, 0x00, 0x00] },
    // The reaction picker's case 8: next state = +0x940 (the stop state).
    Site { name: "reaction picker: stop state (0x14017D28A)", rva: 0x0017_D28A, expect: &[0x49, 0x83, 0xE0, 0xEF, 0x48, 0x8D, 0x83, 0x40, 0x09, 0x00, 0x00, 0x4C, 0x89, 0x83, 0x60, 0x02, 0x00, 0x00, 0x48, 0x89, 0x43, 0x68] },
    // The enemy hit handler's test for the stop state (lea rcx,[rsi+0x940]).
    Site { name: "enemy hit: stop state test (0x14017F937)", rva: 0x0017_F937, expect: &[0x48, 0x8B, 0x56, 0x60, 0x48, 0x8D, 0x8E, 0x40, 0x09, 0x00, 0x00] },
    // CBattleEnemy's stop state (vtable +0x298 / +0x2A0 / +0x2A8 through the state table's thunks):
    // the begin (timer from +0x2D8, motion frozen, flags 0x84000), the exe (holds up to 8 hits and
    // lands them at the end), the end.
    Site { name: "enemy stop begin (0x14017C740)", rva: 0x0017_C740, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0x8B, 0xD9, 0x33, 0xFF, 0x48] },
    Site { name: "enemy stop exe (0x1401843A0)", rva: 0x0018_43A0, expect: &[0x4C, 0x8B, 0xDC, 0x57, 0x48, 0x81, 0xEC, 0xB0, 0x00, 0x00, 0x00, 0x48, 0x8B, 0x05, 0x16, 0xE7] },
    Site { name: "enemy stop end (0x14017ED30)", rva: 0x0017_ED30, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0x81, 0xD8, 0x00, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
    // M7 f3. The stop exe's phase 1 after a landing: mov eax,0x2EE (or half the reaction motion,
    // the branch before); mov rcx,rdi; mov [rdi+0x160],eax (the per-landing timer) ...
    Site { name: "enemy stop exe: landing timer (0x140184910)", rva: 0x0018_4910, expect: &[0xB8, 0xEE, 0x02, 0x00, 0x00, 0x48, 0x8B, 0xCF, 0x89, 0x87, 0x60, 0x01, 0x00, 0x00] },
    // ... and its phase 2: cmp dword [rcx+0x160],0; jg (wait); inc word [rcx+0x33E]; mov dword
    // [rcx+0x270],1 (the next held hit lands in phase 1, the next frame).
    Site { name: "enemy stop exe: next landing (0x14018499D)", rva: 0x0018_499D, expect: &[0x83, 0xB9, 0x60, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x8F, 0x2B, 0x02, 0x00, 0x00, 0x66, 0xFF, 0x81, 0x3E, 0x03, 0x00, 0x00, 0xC7, 0x81, 0x70, 0x02, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00] },
    // The actor update (`0x1401A59A0`): mov eax,[rdi+0x160]; test eax,eax; jle; sub eax,[rdi+0x14];
    // mov [rdi+0x160],eax (the timer, by the actor's dt), then call [vtable+0x78] (an enemy's
    // pre-update: its hit handler, our wrapper) and the state machine's exe (`[rdi+0x58]` +8).
    Site { name: "actor update: timer, pre-update, state (0x1401A5A7F)", rva: 0x001A_5A7F, expect: &[0x8B, 0x87, 0x60, 0x01, 0x00, 0x00, 0x85, 0xC0, 0x7E, 0x09, 0x2B, 0x47, 0x14, 0x89, 0x87, 0x60, 0x01, 0x00, 0x00, 0x48, 0x8B, 0x07, 0x48, 0x8B, 0xCF, 0xFF, 0x50, 0x78, 0x48, 0x8B, 0x47, 0x58, 0x48, 0x8D, 0x4F, 0x58, 0xFF, 0x50, 0x08] },
    // Re:CoM's Stop case with a target: +0xC = 0x80000000, flags 5, then (target) the target's aim
    // point, speed 0, flags 0x25.
    Site { name: "Stop case: flags (0x140222BC8)", rva: 0x0022_2BC8, expect: &[0xC7, 0x44, 0x24, 0x5C, 0x00, 0x00, 0x00, 0x80, 0x48, 0xC7, 0x45, 0xB0, 0x05, 0x00, 0x00, 0x00, 0x7E, 0x1C, 0x0F, 0x28, 0x8F, 0x60, 0x08, 0x00, 0x00, 0x0F, 0x29, 0x4D, 0x30, 0xC7, 0x45, 0x80, 0x00, 0x00, 0x00, 0x00, 0x48, 0xC7, 0x45, 0xB0, 0x25, 0x00, 0x00, 0x00] },
    // initMagic's Stop charge (case 0x1F): Sora's bone 0, id 0, the (0, 0.85, 0) offset on the
    // stack, the set's +0x18 name, 0x1401A2D50, the object's +0x4C |= 4.
    Site { name: "initMagic: Stop charge (0x140220926)", rva: 0x0022_0926, expect: &[0x48, 0x8B, 0xCF, 0x0F, 0x29, 0x44, 0x24, 0x50, 0xE8, 0xED, 0xB1, 0x08, 0x00, 0x4C, 0x8B, 0xC8, 0x0F, 0x57, 0xC0, 0x48, 0x8B, 0x87, 0xC8, 0x03, 0x00, 0x00, 0x45, 0x33, 0xC0, 0xF3, 0x0F, 0x11, 0x44, 0x24, 0x30, 0x48, 0x89, 0x6C, 0x24, 0x28, 0x0F, 0xB6, 0x48, 0x02, 0x48, 0x8D, 0x44, 0x24, 0x50, 0x48, 0x89, 0x44, 0x24, 0x20, 0x48, 0x8D, 0x14, 0x49, 0x48, 0x8B, 0xCF, 0x48, 0xC1, 0xE2, 0x05, 0x4A, 0x8B, 0x94, 0x32, 0x48, 0x17, 0x5A, 0x00, 0xE8, 0xDC, 0x23, 0xF8, 0xFF, 0x8B, 0xC8, 0xE8, 0x75, 0x75, 0x03, 0x00, 0x48, 0x85, 0xC0, 0x0F, 0x84, 0xE5, 0x00, 0x00, 0x00, 0x83, 0x48, 0x4C, 0x04] },
    // 7.10. Re:CoM's Aero case: its position (0x1401A49B0), the ground test, then the effects
    // set − 0x16 (on the ground) and set − 0x11 at it (0x1401A6B20) ...
    Site { name: "Aero case: effects (0x140222D2F)", rva: 0x0022_2D2F, expect: &[0x48, 0x8B, 0xCF, 0xE8, 0x79, 0x1C, 0xF8, 0xFF, 0x48, 0x8B, 0xCF, 0x0F, 0x28, 0x00, 0x66, 0x0F, 0x7F, 0x45, 0x30, 0xE8, 0xB9, 0x0E, 0xFD, 0xFF, 0x85, 0xC0, 0x75, 0x37, 0x48, 0x8B, 0x87, 0xC8, 0x03, 0x00, 0x00, 0x4C, 0x8D, 0x4D, 0x30, 0xF3, 0x0F, 0x11, 0x74, 0x24, 0x28, 0x0F, 0xB6, 0x48, 0x02, 0x48, 0x8D, 0x45, 0x40, 0x48, 0x89, 0x44, 0x24, 0x20, 0x48, 0x8D, 0x14, 0x49, 0x48, 0xC1, 0xE2, 0x05, 0x44, 0x8D, 0x41, 0xEA, 0x48, 0x8B, 0xCF, 0x48, 0x8B, 0x54, 0x1A, 0x18, 0xE8, 0x9E, 0x3D, 0xF8, 0xFF, 0x48, 0x8B, 0x87, 0xC8, 0x03, 0x00, 0x00, 0x4C, 0x8D, 0x4D, 0x30, 0xF3, 0x0F, 0x11, 0x74, 0x24, 0x28, 0x0F, 0xB6, 0x48, 0x02, 0x48, 0x8D, 0x45, 0x40, 0x48, 0x89, 0x44, 0x24, 0x20, 0x48, 0x8D, 0x14, 0x49, 0x48, 0xC1, 0xE2, 0x05, 0x44, 0x8D, 0x41, 0xEF, 0x48, 0x8B, 0xCF, 0x48, 0x8B, 0x54, 0x1A, 0x18, 0xE8, 0x67, 0x3D, 0xF8, 0xFF] },
    // ... and its shot's flags 7.
    Site { name: "Aero case: flags (0x140222DFE)", rva: 0x0022_2DFE, expect: &[0xC7, 0x44, 0x24, 0x5C, 0x00, 0x00, 0x00, 0x80, 0x48, 0xC7, 0x45, 0xB0, 0x07, 0x00, 0x00, 0x00] },
    // The player damage check: the pending-hit count (`movzx eax,word [rdi+0x720]`, then zeroed)
    // and the hit value read (`lea rcx,[rdi+0x690]` … `call 0x14016FDB0`, i.e. `+0x722`), which
    // the Aero barrier halves before the check.
    Site { name: "damage check: pending hit (0x1401E6FBE)", rva: 0x001E_6FBE, expect: &[0x0F, 0xB7, 0x87, 0x20, 0x07, 0x00, 0x00, 0x45, 0x33, 0xFF, 0x66, 0x44, 0x89, 0xBF, 0x20, 0x07, 0x00, 0x00] },
    Site { name: "damage check: hit value (0x1401E70B0)", rva: 0x001E_70B0, expect: &[0x48, 0x8D, 0x8F, 0x90, 0x06, 0x00, 0x00, 0xF3, 0x0F, 0x5A, 0xC8, 0x41, 0x8B, 0xC7, 0x0F, 0x54, 0x0D, 0x0B, 0xA4, 0x4D, 0x00, 0x66, 0x0F, 0x5A, 0xD1, 0xF3, 0x0F, 0x10, 0x0D, 0x1F, 0xA0, 0x4D, 0x00, 0x0F, 0x2F, 0xCA, 0x0F, 0x96, 0xC0, 0x89, 0x45, 0xA7, 0x8B, 0x87, 0x20, 0x04, 0x00, 0x00, 0x89, 0x45, 0xB3, 0xE8, 0xC8, 0x8C, 0xF8, 0xFF] },
    Site { name: "nearest pick: list walk (0x1401CDC26)", rva: 0x001C_DC26, expect: &[0x49, 0x8B, 0x9E, 0x88, 0x00, 0x00, 0x00, 0x48, 0x8D, 0x7B, 0x10, 0x48, 0x83, 0x3F, 0x00, 0x74, 0x58, 0x8B, 0x0B, 0xE8, 0x12, 0xF7, 0x0D, 0x00, 0x48, 0x8B, 0xC8, 0x48, 0x85, 0xC0, 0x74, 0x3C, 0x49, 0x3B, 0xC7, 0x74, 0x37, 0xF7, 0x80, 0x54, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00] },
    // Battle-end fix: the collision pass's calls of the two tests: mov r8,[rsi+0x18] (the attack
    // shape); [xor r9d,r9d;] mov rcx,[0x14087A870] (the collision context); call the test. So the
    // tests take (context, body shape, attack shape[, 0]) and the context is that global's value.
    Site { name: "collision pass: sphere test call (0x14016EBAF)", rva: 0x0016_EBAF, expect: &[0x4C, 0x8B, 0x46, 0x18, 0x48, 0x8B, 0x0D, 0xB6, 0xBC, 0x70, 0x00, 0xE8, 0x21, 0x8C, 0x00, 0x00] },
    Site { name: "collision pass: capsule test call (0x14016EBE5)", rva: 0x0016_EBE5, expect: &[0x4C, 0x8B, 0x46, 0x18, 0x45, 0x33, 0xC9, 0x48, 0x8B, 0x0D, 0x7D, 0xBC, 0x70, 0x00, 0xE8, 0xA8, 0x83, 0x00, 0x00] },
];

/// KH1 casts in the field (`crate::magic::field_fx`, `docs/recom-m7-magic.md` § Spell effects in the
/// field): the branches retargeted through a jump placed near them. Each is one whole `jmp` /
/// `call rel32` (the bytes checked by `patch`).
pub const MAGIC_FX_BRANCHES: &[Site] = &[
    // CGameSubField state 1 (0x1401AF920): `jmp 0x140211AA0` (the player's group, tail call).
    Site { name: "field state 1: player group jump (0x1401AF933)", rva: 0x001A_F933, expect: &[0xE9, 0x68, 0x21, 0x06, 0x00] },
    // The camp openings: `call 0x140211560` (the battle set's unload).
    Site { name: "camp open: battle set unload (0x1401A95BD)", rva: 0x001A_95BD, expect: &[0xE8, 0x9E, 0x7F, 0x06, 0x00] },
    Site { name: "camp open (menu button): battle set unload (0x1401A99F9)", rva: 0x001A_99F9, expect: &[0xE8, 0x62, 0x7B, 0x06, 0x00] },
];

/// What the field's battle set calls (Re:CoM's battle-set loader and unloader), and the code
/// around the retargeted branches, checked only.
pub const MAGIC_FX_CHECKED: &[Site] = &[
    // push rbx; sub rsp,0x20; mov [rsp+0x30],rdi; test ecx,ecx; jne (char given).
    Site { name: "battle set load (0x140211960)", rva: 0x0021_1960, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x89, 0x7C, 0x24, 0x30, 0x85, 0xC9, 0x75, 0x2D] },
    // sub rsp,0x28; mov eax,[0x1407C7AE0]; test ecx,ecx; cmovne eax,ecx.
    Site { name: "battle set unload (0x140211560)", rva: 0x0021_1560, expect: &[0x48, 0x83, 0xEC, 0x28, 0x8B, 0x05, 0x76, 0x65, 0x5B, 0x00, 0x85, 0xC9, 0x0F, 0x45, 0xC1, 0x89] },
    // CGameSubField state 1 up to its tail jump: sub rsp,0x28; call 0x140235240 (battle room?);
    // test eax,eax; jne ret; xor ecx,ecx; add rsp,0x28.
    Site { name: "field state 1 (0x1401AF920)", rva: 0x001A_F920, expect: &[0x48, 0x83, 0xEC, 0x28, 0xE8, 0x17, 0x59, 0x08, 0x00, 0x85, 0xC0, 0x75, 0x0B, 0x33, 0xC9, 0x48, 0x83, 0xC4, 0x28] },
    // CGameSubBattle state 1 (Re:CoM's own load of the battle set, the call the field now makes):
    // sub rsp,0x28; call 0x140235360 (minigame room?); test; jne; xor ecx,ecx; add rsp,0x28;
    // jmp 0x140211960.
    Site { name: "battle state 1 (0x1401AEA20)", rva: 0x001A_EA20, expect: &[0x48, 0x83, 0xEC, 0x28, 0xE8, 0x37, 0x69, 0x08, 0x00, 0x85, 0xC0, 0x75, 0x0B, 0x33, 0xC9, 0x48, 0x83, 0xC4, 0x28, 0xE9, 0x28, 0x2F, 0x06, 0x00] },
    // Before each unload: mov edx,0x60 / lea edx,[rdi+0x60]; xor ecx,ecx; call 0x1401B0800
    // (system byte (0, 0x60)); test al,al; jne past the unload; xor ecx,ecx.
    Site { name: "camp open: system byte 0x60 test (0x1401A95AB)", rva: 0x001A_95AB, expect: &[0xBA, 0x60, 0x00, 0x00, 0x00, 0x33, 0xC9, 0xE8, 0x49, 0x72, 0x00, 0x00, 0x84, 0xC0, 0x75, 0x07, 0x33, 0xC9] },
    Site { name: "camp open (menu button): system byte 0x60 test (0x1401A99E9)", rva: 0x001A_99E9, expect: &[0x8D, 0x57, 0x60, 0x33, 0xC9, 0xE8, 0x0D, 0x6E, 0x00, 0x00, 0x84, 0xC0, 0x75, 0x07, 0x33, 0xC9] },
];

/// The Customize page's button glyphs (`crate::customize::text::glyph`, `docs/recom-keyblade-equip.md`
/// § Stage 7.13): Re:CoM's text code that turns the escapes into the mapped button's glyph and
/// the font flag the page reads. Checked only.
pub const CUSTOMIZE_GLYPHS_CHECKED: &[Site] = &[
    // The escape expander: movzx eax,cx; add eax,-0xF564; cmp eax,0x20; ja default.
    Site { name: "text escape expander (0x140345B40)", rva: 0x0034_5B40, expect: &[0x0F, 0xB7, 0xC1, 0x05, 0x9C, 0x0A, 0xFF, 0xFF, 0x83, 0xF8, 0x20, 0x0F, 0x87, 0xBF, 0x03, 0x00] },
    // Its jump table, escapes 0xF564..0xF568 (RVAs of the cases).
    Site { name: "text escape table 0xF564..0xF568 (0x140345F18)", rva: 0x0034_5F18, expect: &[0x67, 0x5B, 0x34, 0x00, 0x7D, 0x5B, 0x34, 0x00, 0x43, 0x5C, 0x34, 0x00, 0x59, 0x5C, 0x34, 0x00, 0x93, 0x5B, 0x34, 0x00] },
    // The cases the page's escapes reach: movzx eax,word [0x14083F510 / F51C / F518 / F51A].
    Site { name: "escape 0xF564: confirm glyph (0x140345B67)", rva: 0x0034_5B67, expect: &[0x0F, 0xB7, 0x05, 0xA2, 0x99, 0x4F, 0x00] },
    Site { name: "escape 0xF568: L1 glyph (0x140345B93)", rva: 0x0034_5B93, expect: &[0x0F, 0xB7, 0x05, 0x82, 0x99, 0x4F, 0x00] },
    Site { name: "escape 0xF566: Square glyph (0x140345C43)", rva: 0x0034_5C43, expect: &[0x0F, 0xB7, 0x05, 0xCE, 0x98, 0x4F, 0x00] },
    Site { name: "escape 0xF567: Triangle glyph (0x140345C59)", rva: 0x0034_5C59, expect: &[0x0F, 0xB7, 0x05, 0xBA, 0x98, 0x4F, 0x00] },
    // The C2DFnt draw's font: cmp edx,[0x14087CFB8] (count); lea rbx,[base+0x87CFC0]; imul 0x158.
    Site { name: "C2DFnt draw: font table (0x14025AACE)", rva: 0x0025_AACE, expect: &[0x3B, 0x15, 0xE4, 0x24, 0x62, 0x00, 0x0F, 0x83, 0x95, 0x0C, 0x00, 0x00, 0x48, 0x89, 0x9C, 0x24, 0x38, 0x0A, 0x00, 0x00, 0x4C, 0x8D, 0x0D, 0x17, 0x55, 0xDA, 0xFF, 0x8B, 0xC2, 0x49, 0x8D, 0x99, 0xC0, 0xCF, 0x87, 0x00, 0x48, 0x69, 0xC8, 0x58, 0x01, 0x00, 0x00] },
    // The glyph picker: cmp dword [rcx+0x150],0 (button sheet); ... ; mov ecx,0x55C.
    Site { name: "glyph picker: button sheet test (0x14025A0B4)", rva: 0x0025_A0B4, expect: &[0x83, 0xB9, 0x50, 0x01, 0x00, 0x00, 0x00, 0x41, 0x8B, 0xF1, 0x41, 0x8B, 0xE8, 0x44, 0x0F, 0xB7, 0xD2, 0x4C, 0x8B, 0xD9, 0x74, 0x6D, 0xB9, 0x5C, 0x05, 0x00, 0x00] },
    // Font set-up: the "sysfont" gets +0x150 = 1.
    Site { name: "font set-up: sysfont button flag (0x14025C400)", rva: 0x0025_C400, expect: &[0xC7, 0x84, 0x2F, 0x50, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00] },
];

/// M7 stage 7.11, spell unlocks (`crate::unlocks`, `docs/recom-m7-unlocks.md`): Re:CoM's "first
/// obtained" of a card id, the one place every collection add turns a card's discovery state from
/// 1 to 2. Same rule as [`HOOKS`].
pub const UNLOCK_HOOKS: &[Site] = &[
    // sub rsp,0x28; mov eax,0xFF (then movzx r9d,cx: the id in cx).
    Site { name: "card first obtained (0x14015C7F0)", rva: 0x0015_C7F0, expect: &[0x48, 0x83, 0xEC, 0x28, 0xB8, 0xFF, 0x00, 0x00, 0x00] },
];

/// Re:CoM code the unlocks rely on (not patched).
pub const UNLOCK_CHECKED: &[Site] = &[
    // The collection add for types 1-4 and 7 (`0x14015CDB0`, from the add `0x140158FC0`): movzx
    // ecx,bx; call 0x14015CAE0 (discovery 0 -> 1); movzx ecx,bx; call 0x14015C7F0 (1 -> 2).
    Site { name: "card add: discovery, first obtained (0x14015CDFB)", rva: 0x0015_CDFB, expect: &[0x0F, 0xB7, 0xCB, 0xE8, 0xDD, 0xFC, 0xFF, 0xFF, 0x0F, 0xB7, 0xCB, 0xE8, 0xE5, 0xF9, 0xFF, 0xFF] },
];

/// KH1's menu sounds (`crate::menu_sounds`, `docs/recom-m7-command-menu.md` § Sounds): Re:CoM's
/// sound id → bank map, extended with the groups 9000..9999 (each its own bank). Same rule as
/// [`HOOKS`].
pub const MENU_SOUND_HOOKS: &[Site] = &[
    // and ecx,0x0FFFFFFF; mov eax,0x10624DD3 (÷1000); mul ecx (then mov r9d,-1: not moved).
    Site { name: "sound id to bank (0x1402A8560)", rva: 0x002A_8560, expect: &[0x81, 0xE1, 0xFF, 0xFF, 0xFF, 0x0F, 0xB8, 0xD3, 0x4D, 0x62, 0x10, 0xF7, 0xE1] },
];

/// Re:CoM's sound-bank registration the menu sounds call (what its loader does with a pack's PC
/// SCD: `(name, bank, data, tag)` → the driver's handle; the entry frees the data with
/// `_aligned_free` when released, `0x1402F0610`): through the "sound system up" test
/// (`cmp qword [0x140C158B0],0`) and the read of the bank list's head (`mov rax,[0x140822370]`),
/// which the DLL walks.
pub const MENU_SOUND_CALLEES: &[Site] = &[Site {
    name: "sound bank register (0x1402F5D70)",
    rva: 0x002F_5D70,
    expect: &[
        0x40, 0x57, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x48, 0xC7, 0x44, 0x24, 0x30, 0xFE, 0xFF, 0xFF, 0xFF, 0x48, 0x89, 0x5C, 0x24, 0x60, 0x48, 0x89, 0x6C, 0x24, 0x68, 0x48, 0x89, 0x74, 0x24, 0x70, 0x41, 0x8B, 0xE9, 0x4D, 0x8B, 0xF8, 0x44,
        0x8B, 0xF2, 0x48, 0x8B, 0xF1, 0x48, 0x83, 0x3D, 0x0A, 0xFB, 0x91, 0x00, 0x00, 0x75, 0x07, 0x33, 0xC0, 0xE9, 0x89, 0x01, 0x00, 0x00, 0x33, 0xFF, 0x48, 0x85, 0xF6, 0x0F, 0x84, 0x1B, 0x01, 0x00, 0x00, 0x44, 0x8D, 0x47, 0x08, 0x48, 0x8D, 0x15,
        0x33, 0x51, 0x34, 0x00, 0xFF, 0x15, 0x6D, 0xED, 0x20, 0x00, 0x85, 0xC0, 0x75, 0x54, 0x48, 0x8B, 0x05, 0x9A, 0xC5, 0x52, 0x00,
    ],
}];

/// `CPlayerSora`'s exeMagic slot (vtable `0x1405A9938` + 0x150). Checked (exe; relocated).
pub const MAGIC_PTRS: &[PtrSite] = &[PtrSite { name: "CPlayerSora vtable +0x150 (0x1405A9A88)", rva: 0x005A_9A88, target: 0x0021_7EE0 }];

/// `CHomingBullet`'s homing slot (vtable `0x140525530` + 0xA8), which the thunk jumps through
/// (7.4b re-target). Checked (exe; relocated).
pub const MAGIC_RETARGET_PTRS: &[PtrSite] = &[PtrSite { name: "CHomingBullet vtable +0xA8 (0x1405255D8)", rva: 0x0052_55D8, target: 0x0015_75D0 }];

/// `CCollAttack`'s "off" and apply slots (vtable `0x140523E28` + 8 / + 0x38), which a Thunder bolt's
/// direct strike calls through (7.6). Checked (exe; relocated).
pub const MAGIC_THUNDER_PTRS: &[PtrSite] = &[
    PtrSite { name: "CCollAttack vtable +8 (0x140523E30)", rva: 0x0052_3E30, target: 0x0014_DE80 },
    PtrSite { name: "CCollAttack vtable +0x38 (0x140523E60)", rva: 0x0052_3E60, target: 0x0016_F1C0 },
];

/// Data this version reads or calls through, with its address (Steam). Checked (exe).
pub mod data {
    /// Sequence ids for label items (`u16[]`).
    pub const LABEL_SEQ: u64 = 0x1_407C_1230;
    /// Per value item, a pointer to its sequence ids (9 pointers).
    pub const VALUE_SEQ: u64 = 0x1_407C_1240;
    /// Pointer to the global whose `+0x274` picks the label font (0xC for 3 and 6).
    pub const FONT_GLOBAL: u64 = 0x1_40AF_73B0;
    /// The main player (`CPlayer` ctor `0x1401E4C20`).
    pub const MAIN_PLAYER: u64 = 0x1_4087_B380;
    /// `CPlayerSora` vtable (RTTI). Riku's are `0x1405A8868` / `0x1405A50E8`.
    pub const SORA_VTABLE: u64 = 0x1_405A_9938;
    /// `CPlayerRiku` vtable (RTTI; `docs/recom-player.md`). Its Battle exe slot `+0xB8` is a
    /// `jmp` to the shared Battle exe `0x1401F9840`. Checked (exe).
    pub const RIKU_VTABLE: u64 = 0x1_405A_8868;
    /// `CPlayerDRiku` vtable (RTTI; Dark Mode's player, `docs/recom-riku-kh1.md`). Its Battle exe
    /// `+0xB8` is `jmp 0x1401F9840` too; its Field exe is `ret`. Checked (exe).
    pub const DRIKU_VTABLE: u64 = 0x1_405A_50E8;
    /// Pad devices, one pointer per port (`0x14029BF60` reads `[port]+0x74`).
    pub const PAD_DEVICES: u64 = 0x1_40AC_5D40;
    /// Sora's action table and the per-character table pointers (`docs/recom-motion-switch.md`).
    pub const SORA_TABLE: u64 = 0x1_407C_5E80;
    pub const CHAR_TABLES: u64 = 0x1_407C_6300;
    /// `CPlayerSora` vtable slot of the magic-card init (`+0x148`, `initMagic`).
    pub const VT_INIT_MAGIC: usize = 0x148;
}

/// Story skips at Combat = KH1 (`win::story_skip`, `crate::story_skip`,
/// `docs/recom-story-skips.md`). Same rule as [`HOOKS`]. Checked (exe): no branch lands inside the
/// moved bytes; the only other reference to the function is its `.pdata` entry.
pub const STORY_SKIP_HOOKS: &[Site] = &[
    // mov [rsp+8],rbx; push rdi; sub rsp,0x20 (10 bytes; the next instruction, `movzx edi,cx`,
    // stays). Re:CoM's battle-entry request: script syscall 0xEE (`0x1401490D6`) passes a battle
    // room; case 10 (`0x1401460D6`), the gimmicks and `0x140237020` pass 0, `0x1401CA9D4` 1030.
    Site { name: "battle entry request (0x140236690)", rva: 0x0023_6690, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20] },
];

/// Re:CoM functions the story skip calls (with the system variable getter / setter of earlier
/// stages).
pub const STORY_SKIP_CALLEES: &[Site] = &[
    // push rsi; push rdi; push r12; push r13; push r14; sub rsp,0x30: the room change
    // (kind, door, room) script syscall 0xF7 calls.
    Site { name: "room change (0x1402360F0)", rva: 0x0023_60F0, expect: &[0x40, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x30] },
    // mov rax,[rip+…] (the save block); movzx ecx,byte [rax+2]; movzx eax,cl; cmp cl,4: the
    // saved door (syscall 0xF7's door −1).
    Site { name: "saved door (0x140233AE0)", rva: 0x0023_3AE0, expect: &[0x48, 0x8B, 0x05, 0x29, 0x76, 0x64, 0x00, 0x0F, 0xB6, 0x48, 0x02, 0x0F, 0xB6, 0xC1, 0x80, 0xF9, 0x04] },
    // mov r11d,r8d; mov r9d,edx; cmp ecx,2; jae: a system flag set (table, index, value), as
    // script op 0x6E calls it.
    Site { name: "system flag set (0x1401B1650)", rva: 0x001B_1650, expect: &[0x45, 0x8B, 0xD8, 0x44, 0x8B, 0xCA, 0x83, 0xF9, 0x02, 0x73, 0x57] },
];

/// M10, KH1's Shadow (`crate::shadow`, `win::shadow`, `docs/recom-m10-shadow.md`): the wrapper on
/// `CEnemyShadow`'s ctor. Same rule as [`HOOKS`]. Checked (exe): its only reference is the call in
/// the enemy factory `0x140188F40` (id 1) and `.pdata`; no branch lands inside the moved bytes.
pub const SHADOW_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyShadow ctor (0x140192770)", rva: 0x0019_2770, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M10: the factory's `mov ecx, 0xCE0` before `CEnemyShadow`'s ctor (the allocator `0x1404ACE54`
/// call follows), raised by our 0x20-byte tail. Checked (exe).
pub const SHADOW_ALLOC: &[Site] = &[Site { name: "alloc CEnemyShadow (0x140188FAD)", rva: 0x0018_8FAD, expect: &[0xB9, 0xE0, 0x0C, 0x00, 0x00] }];

/// M10: Re:CoM functions the KH1 Shadow's states call (the others it calls are checked in their
/// own lists: `0x1401A2D90`, `0x1401A2DB0`, `0x1402ABB20`, `0x1402AD350`, `0x1401A4A20`,
/// `0x1401A7050`). Checked (exe + decomp).
/// The enemy attack end `0x14017E540` they call is hooked (and checked) in [`M6_PACE_HOOKS`] /
/// [`M6_PACE_CHECKED`].
pub const SHADOW_CALLEES: &[Site] = &[
    // mov rax,[rcx+0x2F8]; mov dword [rcx+0x2C8],0xFA; test rax,rax
    Site { name: "enemy default state (0x14017D6F0)", rva: 0x0017_D6F0, expect: &[0x48, 0x8B, 0x81, 0xF8, 0x02, 0x00, 0x00, 0xC7, 0x81, 0xC8, 0x02, 0x00, 0x00, 0xFA, 0x00, 0x00, 0x00, 0x48, 0x85, 0xC0] },
    // sub rsp,0x28; call 0x1402336C0; cmp ax,5
    Site { name: "battle intro room (0x1401A1680)", rva: 0x001A_1680, expect: &[0x48, 0x83, 0xEC, 0x28, 0xE8, 0x37, 0x20, 0x09, 0x00, 0x66, 0x83, 0xF8, 0x05] },
    // mov eax,[rip+…] (0x1407C3CF8); ret
    Site { name: "battle intro flag (0x1401A1670)", rva: 0x001A_1670, expect: &[0x8B, 0x05, 0x82, 0x26, 0x62, 0x00, 0xC3] },
    // inc dword [rip+…] (0x14087B0A0); ret
    Site { name: "battle intro count (0x1401A14B0)", rva: 0x001A_14B0, expect: &[0xFF, 0x05, 0xEA, 0x9B, 0x6D, 0x00, 0xC3] },
    // mov rcx,[rcx+0xF8]; test rcx,rcx; jnz 0x140155380 (every hurt body off)
    Site { name: "enemy hurt bodies off (0x1401A2D70)", rva: 0x001A_2D70, expect: &[0x48, 0x8B, 0x89, 0xF8, 0x00, 0x00, 0x00, 0x48, 0x85, 0xC9, 0x0F, 0x85, 0x00, 0x26, 0xFB, 0xFF] },
    // mov rcx,[rcx+0xF8]; test rcx,rcx; jnz 0x140155570 (the other set back on)
    Site { name: "enemy attack bodies on (0x1401A2DD0)", rva: 0x001A_2DD0, expect: &[0x48, 0x8B, 0x89, 0xF8, 0x00, 0x00, 0x00, 0x48, 0x85, 0xC9, 0x0F, 0x85, 0x90, 0x27, 0xFB, 0xFF] },
    // mov rax,[rcx+0xD8]; test rax,rax; je; mov eax,[rax+0x3C]; ret (the motion clock)
    Site { name: "actor motion clock (0x1401A47C0)", rva: 0x001A_47C0, expect: &[0x48, 0x8B, 0x81, 0xD8, 0x00, 0x00, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x04, 0x8B, 0x40, 0x3C, 0xC3] },
];

/// M10: `CEnemyShadow`'s vtable slots the copy keeps or replaces, each checked against Re:CoM's
/// function before the copy is made (Checked (exe), relocated pointers).
pub const SHADOW_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyShadow +0x70 motion play", rva: 0x0054_D798, target: 0x0017_D750 },
    PtrSite { name: "CEnemyShadow +0x80 update", rva: 0x0054_D7A8, target: 0x0019_3420 },
    PtrSite { name: "CEnemyShadow +0xE8 state 0 begin", rva: 0x0054_D810, target: 0x0019_2850 },
    PtrSite { name: "CEnemyShadow +0xF0 state 0 step", rva: 0x0054_D818, target: 0x0019_2B00 },
    PtrSite { name: "CEnemyShadow +0xF8 state 0 end", rva: 0x0054_D820, target: 0x0019_2A60 },
    PtrSite { name: "CEnemyShadow +0x100 state 1 begin", rva: 0x0054_D828, target: 0x0019_2A40 },
    PtrSite { name: "CEnemyShadow +0x108 state 1 step", rva: 0x0054_D830, target: 0x0019_2E60 },
    PtrSite { name: "CEnemyShadow +0x118 state 2 begin", rva: 0x0054_D840, target: 0x0019_2960 },
    PtrSite { name: "CEnemyShadow +0x130 state 3 begin", rva: 0x0054_D858, target: 0x0019_29B0 },
    PtrSite { name: "CEnemyShadow +0x148 state 4 begin", rva: 0x0054_D870, target: 0x0017_B790 },
    PtrSite { name: "CEnemyShadow +0x1F0 state 11 begin", rva: 0x0054_D918, target: 0x0017_C530 },
    PtrSite { name: "CEnemyShadow +0x208 state 12 begin", rva: 0x0054_D930, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyShadow +0x3B8 state 30 begin", rva: 0x0054_DAE0, target: 0x0019_2A00 },
    PtrSite { name: "CEnemyShadow +0x3C0 state 30 step", rva: 0x0054_DAE8, target: 0x0019_2E30 },
    PtrSite { name: "CEnemyShadow +0x3D0 think", rva: 0x0054_DAF8, target: 0x0019_6EC0 },
    PtrSite { name: "CEnemyShadow +0x438 attack hurt off", rva: 0x0054_DB60, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyShadow +0x440 attack end", rva: 0x0054_DB68, target: 0x0017_DA90 },
];

/// M11, the KH1 enemy framework (`crate::kh1_enemy`, `win::kh1_enemy`): the Soldier's ctor (same
/// shape as the Shadow's). Same rule as [`HOOKS`]. Checked (exe + decomp): its only caller is the
/// enemy factory `0x140188F40` (id 2); no branch lands inside the moved bytes (exe scan).
pub const SOLDIER_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemySoldier ctor (0x140193510)", rva: 0x0019_3510, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11: the factory's `mov ecx, 0xCD0` before `CEnemySoldier`'s ctor (the allocator `0x1404ACE54`
/// call follows), raised by the framework's 0x20-byte tail. Checked (exe).
pub const SOLDIER_ALLOC: &[Site] = &[Site { name: "alloc CEnemySoldier (0x140188FD7)", rva: 0x0018_8FD7, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11: `CEnemySoldier`'s vtable slots the copy keeps or replaces, checked against Re:CoM's
/// functions before the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12
/// / 30 are `CBattleEnemy`'s, 1..3 the Soldier's own).
pub const SOLDIER_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemySoldier +0x70 motion play", rva: 0x0054_EAD8, target: 0x0017_D750 },
    PtrSite { name: "CEnemySoldier +0x80 update", rva: 0x0054_EAE8, target: 0x0018_5320 },
    PtrSite { name: "CEnemySoldier +0xE8 state 0 begin", rva: 0x0054_EB50, target: 0x0017_B590 },
    PtrSite { name: "CEnemySoldier +0xF0 state 0 step", rva: 0x0054_EB58, target: 0x0018_29E0 },
    PtrSite { name: "CEnemySoldier +0xF8 state 0 end", rva: 0x0054_EB60, target: 0x0017_E630 },
    PtrSite { name: "CEnemySoldier +0x100 state 1 begin", rva: 0x0054_EB68, target: 0x0019_3680 },
    PtrSite { name: "CEnemySoldier +0x108 state 1 step", rva: 0x0054_EB70, target: 0x0019_3A10 },
    PtrSite { name: "CEnemySoldier +0x118 state 2 begin", rva: 0x0054_EB80, target: 0x0019_35E0 },
    PtrSite { name: "CEnemySoldier +0x120 state 2 step", rva: 0x0054_EB88, target: 0x0019_36B0 },
    PtrSite { name: "CEnemySoldier +0x130 state 3 begin", rva: 0x0054_EB98, target: 0x0019_3630 },
    PtrSite { name: "CEnemySoldier +0x138 state 3 step", rva: 0x0054_EBA0, target: 0x0019_3940 },
    PtrSite { name: "CEnemySoldier +0x1F0 state 11 begin", rva: 0x0054_EC58, target: 0x0017_C530 },
    PtrSite { name: "CEnemySoldier +0x208 state 12 begin", rva: 0x0054_EC70, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemySoldier +0x3B8 state 30 begin", rva: 0x0054_EE20, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemySoldier +0x3C0 state 30 step", rva: 0x0054_EE28, target: 0x0018_3260 },
    PtrSite { name: "CEnemySoldier +0x3D0 think", rva: 0x0054_EE38, target: 0x0019_6EC0 },
    PtrSite { name: "CEnemySoldier +0x438 attack hurt off", rva: 0x0054_EEA0, target: 0x0017_DA80 },
    PtrSite { name: "CEnemySoldier +0x440 attack end", rva: 0x0054_EEA8, target: 0x0017_DA90 },
];

/// M11 fliers: the Air Soldier's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe +
/// decomp): its only caller is the enemy factory `0x140188F40` (id 15, the call at `0x140189212`); no
/// branch lands inside the moved bytes (exe scan).
pub const AIRSOLDIER_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyAirsoldier ctor (0x14018A4E0)", rva: 0x0018_A4E0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 fliers: the factory's `mov ecx, 0xCD0` before `CEnemyAirsoldier`'s ctor (fields used up to
/// `+0xCC0`, its ctor's flag), raised by the framework's 0x20-byte tail. Checked (exe).
pub const AIRSOLDIER_ALLOC: &[Site] = &[Site { name: "alloc CEnemyAirsoldier (0x1401891F9)", rva: 0x0018_91F9, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 fliers: `CEnemyAirsoldier`'s vtable slots the copy keeps or replaces, checked against Re:CoM's
/// functions before the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 /
/// 30 are `CBattleEnemy`'s, 1..3 and the think `0x14018A5C0` the Air Soldier's own).
pub const AIRSOLDIER_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyAirsoldier +0x70 motion play", rva: 0x0053_1D38, target: 0x0017_D750 },
    PtrSite { name: "CEnemyAirsoldier +0x80 update", rva: 0x0053_1D48, target: 0x0018_5320 },
    PtrSite { name: "CEnemyAirsoldier +0xE8 state 0 begin", rva: 0x0053_1DB0, target: 0x0017_B590 },
    PtrSite { name: "CEnemyAirsoldier +0xF0 state 0 step", rva: 0x0053_1DB8, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyAirsoldier +0xF8 state 0 end", rva: 0x0053_1DC0, target: 0x0017_E630 },
    PtrSite { name: "CEnemyAirsoldier +0x100 state 1 begin", rva: 0x0053_1DC8, target: 0x0018_A740 },
    PtrSite { name: "CEnemyAirsoldier +0x108 state 1 step", rva: 0x0053_1DD0, target: 0x0018_A950 },
    PtrSite { name: "CEnemyAirsoldier +0x118 state 2 begin", rva: 0x0053_1DE0, target: 0x0018_A6A0 },
    PtrSite { name: "CEnemyAirsoldier +0x120 state 2 step", rva: 0x0053_1DE8, target: 0x0018_A7D0 },
    PtrSite { name: "CEnemyAirsoldier +0x130 state 3 begin", rva: 0x0053_1DF8, target: 0x0018_A6F0 },
    PtrSite { name: "CEnemyAirsoldier +0x138 state 3 step", rva: 0x0053_1E00, target: 0x0018_A8D0 },
    PtrSite { name: "CEnemyAirsoldier +0x1F0 state 11 begin", rva: 0x0053_1EB8, target: 0x0017_C530 },
    PtrSite { name: "CEnemyAirsoldier +0x208 state 12 begin", rva: 0x0053_1ED0, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyAirsoldier +0x3B8 state 30 begin", rva: 0x0053_2080, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyAirsoldier +0x3C0 state 30 step", rva: 0x0053_2088, target: 0x0018_3260 },
    PtrSite { name: "CEnemyAirsoldier +0x3D0 think", rva: 0x0053_2098, target: 0x0018_A5C0 },
    PtrSite { name: "CEnemyAirsoldier +0x438 attack hurt off", rva: 0x0053_2100, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyAirsoldier +0x440 attack end", rva: 0x0053_2108, target: 0x0017_DA90 },
];

/// M11 ground melee: the Powerwild's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked
/// (exe): its only caller is the enemy factory `0x140188F40` (id 7, the call at `0x1401890C2`); no
/// branch lands inside the moved bytes (exe scan).
pub const POWERWILD_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d)
    Site { name: "CEnemyPowerwild ctor (0x140191380)", rva: 0x0019_1380, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 ground melee: the factory's `mov ecx, 0xCD0` before `CEnemyPowerwild`'s ctor, raised by the
/// framework's 0x20-byte tail. Checked (exe).
pub const POWERWILD_ALLOC: &[Site] = &[Site { name: "alloc CEnemyPowerwild (0x1401890A9)", rva: 0x0018_90A9, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 ground melee: `CEnemyPowerwild`'s vtable slots the copy keeps or replaces, checked against
/// Re:CoM's functions before the copy is made (Checked (exe), relocated pointers: states 0 / 11 /
/// 12 / 30 are `CBattleEnemy`'s, 1..3 the Powerwild's own).
pub const POWERWILD_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyPowerwild +0x70 motion play", rva: 0x0054_7808, target: 0x0017_D750 },
    PtrSite { name: "CEnemyPowerwild +0x80 update", rva: 0x0054_7818, target: 0x0018_5320 },
    PtrSite { name: "CEnemyPowerwild +0xE8 state 0 begin", rva: 0x0054_7880, target: 0x0017_B590 },
    PtrSite { name: "CEnemyPowerwild +0xF0 state 0 step", rva: 0x0054_7888, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyPowerwild +0xF8 state 0 end", rva: 0x0054_7890, target: 0x0017_E630 },
    PtrSite { name: "CEnemyPowerwild +0x100 state 1 begin", rva: 0x0054_7898, target: 0x0019_14F0 },
    PtrSite { name: "CEnemyPowerwild +0x108 state 1 step", rva: 0x0054_78A0, target: 0x0019_1870 },
    PtrSite { name: "CEnemyPowerwild +0x118 state 2 begin", rva: 0x0054_78B0, target: 0x0019_1450 },
    PtrSite { name: "CEnemyPowerwild +0x120 state 2 step", rva: 0x0054_78B8, target: 0x0019_1520 },
    PtrSite { name: "CEnemyPowerwild +0x130 state 3 begin", rva: 0x0054_78C8, target: 0x0019_14A0 },
    PtrSite { name: "CEnemyPowerwild +0x138 state 3 step", rva: 0x0054_78D0, target: 0x0019_15F0 },
    PtrSite { name: "CEnemyPowerwild +0x1F0 state 11 begin", rva: 0x0054_7988, target: 0x0017_C530 },
    PtrSite { name: "CEnemyPowerwild +0x208 state 12 begin", rva: 0x0054_79A0, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyPowerwild +0x3B8 state 30 begin", rva: 0x0054_7B50, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyPowerwild +0x3C0 state 30 step", rva: 0x0054_7B58, target: 0x0018_3260 },
    PtrSite { name: "CEnemyPowerwild +0x3D0 think", rva: 0x0054_7B68, target: 0x0019_6EC0 },
    PtrSite { name: "CEnemyPowerwild +0x438 attack hurt off", rva: 0x0054_7BD0, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyPowerwild +0x440 attack end", rva: 0x0054_7BD8, target: 0x0017_DA90 },
];

/// M11 fliers: the Darkball's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe +
/// decomp): its only caller is the enemy factory `0x140188F40` (case 23, the call at `0x140189362`);
/// no branch lands inside the moved bytes (exe scan: rel32 calls / jumps, jcc, rel8 jumps).
pub const DARKBALL_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyDarkball ctor (0x14018D5D0)", rva: 0x0018_D5D0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 fliers: the factory's `mov ecx, 0xCD0` before `CEnemyDarkball`'s ctor (its states use fields up
/// to `+0xCB0`), raised by the framework's 0x20-byte tail. Checked (exe).
pub const DARKBALL_ALLOC: &[Site] = &[Site { name: "alloc CEnemyDarkball (0x140189349)", rva: 0x0018_9349, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 fliers: `CEnemyDarkball`'s vtable slots the copy keeps or replaces, checked against Re:CoM's
/// functions before the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 /
/// 30 and the think are `CBattleEnemy`'s / `CHeartless`'s, 1..3 the Darkball's own: 1 the flying walk
/// `0x14018D720` / `0x14018D8D0` → `0x140197450`, 2 its dash `e023_050..052` `0x14018D6A0` /
/// `0x14018D780`, 3 its bite `e023_053` `0x14018D6F0` / `0x14018D8A0`).
pub const DARKBALL_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyDarkball +0x70 motion play", rva: 0x0053_B818, target: 0x0017_D750 },
    PtrSite { name: "CEnemyDarkball +0x80 update", rva: 0x0053_B828, target: 0x0018_5320 },
    PtrSite { name: "CEnemyDarkball +0xE8 state 0 begin", rva: 0x0053_B890, target: 0x0017_B590 },
    PtrSite { name: "CEnemyDarkball +0xF0 state 0 step", rva: 0x0053_B898, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyDarkball +0xF8 state 0 end", rva: 0x0053_B8A0, target: 0x0017_E630 },
    PtrSite { name: "CEnemyDarkball +0x100 state 1 begin", rva: 0x0053_B8A8, target: 0x0018_D720 },
    PtrSite { name: "CEnemyDarkball +0x108 state 1 step", rva: 0x0053_B8B0, target: 0x0018_D8D0 },
    PtrSite { name: "CEnemyDarkball +0x118 state 2 begin", rva: 0x0053_B8C0, target: 0x0018_D6A0 },
    PtrSite { name: "CEnemyDarkball +0x120 state 2 step", rva: 0x0053_B8C8, target: 0x0018_D780 },
    PtrSite { name: "CEnemyDarkball +0x130 state 3 begin", rva: 0x0053_B8D8, target: 0x0018_D6F0 },
    PtrSite { name: "CEnemyDarkball +0x138 state 3 step", rva: 0x0053_B8E0, target: 0x0018_D8A0 },
    PtrSite { name: "CEnemyDarkball +0x1F0 state 11 begin", rva: 0x0053_B998, target: 0x0017_C530 },
    PtrSite { name: "CEnemyDarkball +0x208 state 12 begin", rva: 0x0053_B9B0, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyDarkball +0x3B8 state 30 begin", rva: 0x0053_BB60, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyDarkball +0x3C0 state 30 step", rva: 0x0053_BB68, target: 0x0018_3260 },
    PtrSite { name: "CEnemyDarkball +0x3D0 think", rva: 0x0053_BB78, target: 0x0019_6EC0 },
    PtrSite { name: "CEnemyDarkball +0x438 attack hurt off", rva: 0x0053_BBE0, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyDarkball +0x440 attack end", rva: 0x0053_BBE8, target: 0x0017_DA90 },
];

/// M11 ground melee: the Bandit's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked
/// (exe): its only caller is the enemy factory `0x140188F40` (id 11, the call at `0x14018916A`); no
/// branch lands inside the moved bytes and no pointer names it (exe scan).
pub const BANDIT_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d)
    Site { name: "CEnemyBandit ctor (0x14018AEB0)", rva: 0x0018_AEB0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 ground melee: the factory's `mov ecx, 0xCD0` before `CEnemyBandit`'s ctor, raised by the
/// framework's 0x20-byte tail. Checked (exe).
pub const BANDIT_ALLOC: &[Site] = &[Site { name: "alloc CEnemyBandit (0x140189151)", rva: 0x0018_9151, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11, the shared enemy guard (owner 2026-11-02: "hide Re:CoM's "Guard" popup over KH1 enemies"):
/// the enemy hit handler's (`0x14017F7E0`) guarded branch (`+0xF8 → +8 & 0x18`) calls the guard spark
/// `0x140182260`, then `mov ecx,[rsi+0x20]; lea edx,[rbp+3]; call 0x14020C9A0` (`CCommonGauge`'s popup
/// 3, "Guard", over the enemy's handle), then `call [rax+0x3D8]` (the guard event). The call is
/// retargeted once at install, through a stub near it, to `win::kh1_enemy::guard_popup`, which skips
/// it for our KH1 enemies. Bytes read from the Steam exe; no branch lands inside the call (exe scan).
pub const M11_GUARD_POPUP_CALLS: &[Site] = &[Site { name: "enemy hit guard popup 3 (0x14017FCE7)", rva: 0x0017_FCE7, expect: &[0xE8, 0xB4, 0xCC, 0x08, 0x00] }];

/// The call's context: the spark call and the popup's handle / kind loads before it, the guard
/// event's call after it.
pub const M11_GUARD_POPUP_CHECKED: &[Site] = &[
    Site { name: "guard spark call, popup handle / kind (0x14017FCDC)", rva: 0x0017_FCDC, expect: &[0xE8, 0x7F, 0x25, 0x00, 0x00, 0x8B, 0x4E, 0x20, 0x8D, 0x55, 0x03] },
    Site { name: "guard event call after the popup (0x14017FCEC)", rva: 0x0017_FCEC, expect: &[0x48, 0x8B, 0x06, 0x48, 0x8B, 0xCE, 0xFF, 0x90, 0xD8, 0x03, 0x00, 0x00] },
];

/// M11 ground melee: `CEnemyBandit`'s vtable slots the copy keeps or replaces, checked against
/// Re:CoM's functions before the copy is made (Checked (exe), relocated pointers: states 0 / 11 /
/// 12 / 30 are `CBattleEnemy`'s, 1..3 the Bandit's own).
pub const BANDIT_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyBandit +0x70 motion play", rva: 0x0053_43D8, target: 0x0017_D750 },
    PtrSite { name: "CEnemyBandit +0x80 update", rva: 0x0053_43E8, target: 0x0018_5320 },
    PtrSite { name: "CEnemyBandit +0xE8 state 0 begin", rva: 0x0053_4450, target: 0x0017_B590 },
    PtrSite { name: "CEnemyBandit +0xF0 state 0 step", rva: 0x0053_4458, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyBandit +0xF8 state 0 end", rva: 0x0053_4460, target: 0x0017_E630 },
    PtrSite { name: "CEnemyBandit +0x100 state 1 begin", rva: 0x0053_4468, target: 0x0018_AFF0 },
    PtrSite { name: "CEnemyBandit +0x108 state 1 step", rva: 0x0053_4470, target: 0x0018_B3A0 },
    PtrSite { name: "CEnemyBandit +0x118 state 2 begin", rva: 0x0053_4480, target: 0x0018_AF70 },
    PtrSite { name: "CEnemyBandit +0x120 state 2 step", rva: 0x0053_4488, target: 0x0018_B120 },
    PtrSite { name: "CEnemyBandit +0x130 state 3 begin", rva: 0x0053_4498, target: 0x0018_AFA0 },
    PtrSite { name: "CEnemyBandit +0x138 state 3 step", rva: 0x0053_44A0, target: 0x0018_B160 },
    PtrSite { name: "CEnemyBandit +0x1F0 state 11 begin", rva: 0x0053_4558, target: 0x0017_C530 },
    PtrSite { name: "CEnemyBandit +0x208 state 12 begin", rva: 0x0053_4570, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyBandit +0x3B8 state 30 begin", rva: 0x0053_4720, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyBandit +0x3C0 state 30 step", rva: 0x0053_4728, target: 0x0018_3260 },
    PtrSite { name: "CEnemyBandit +0x3D0 think", rva: 0x0053_4738, target: 0x0019_6EC0 },
    // The shared enemy-guard step: the hit handler's guard event (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyBandit +0x3D8 guard event", rva: 0x0053_4740, target: 0x0018_1600 },
    PtrSite { name: "CEnemyBandit +0x438 attack hurt off", rva: 0x0053_47A0, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyBandit +0x440 attack end", rva: 0x0053_47A8, target: 0x0017_DA90 },
];

/// M11, the shared enemy-guard step and the thrown objects (`win::kh1_enemy`, `win::kh1_thrown`;
/// `docs/recom-m11-enemies.md` § Enemy guard, § The Bandit's thrown scimitar): Re:CoM's functions
/// a KH1 enemy's thrown object takes its power with, as Re:CoM's attack records do (`0x14014E320`).
/// Called only. Bytes read from the Steam exe (2026-11-02).
pub const THROWN_CALLEES: &[Site] = &[
    // push rbx; push rbp; push rsi; push rdi; push r12; push r13; push r15; sub rsp,0x70: an enemy
    // attack's AP (`@ENP +4` × difficulty × the record's `+0x10`, Re:CoM's enemy-card rewrite).
    Site { name: "enemy attack AP (0x14017F0E0)", rva: 0x0017_F0E0, expect: &[0x40, 0x53, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x70] },
    // mov [rsp+8],rbx; mov [rsp+0x10],rbp; mov [rsp+0x18],rsi; push rdi: an enemy attack's attribute.
    Site { name: "enemy attack attribute (0x14017F050)", rva: 0x0017_F050, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57] },
    // The whole function: movsxd rax,[rcx+0x68]; lea rcx,[0x14052A220]; mov eax,[rcx+rax*4]; ret.
    Site { name: "attack record kind (0x140179410)", rva: 0x0017_9410, expect: &[0x48, 0x63, 0x41, 0x68, 0x48, 0x8D, 0x0D, 0x05, 0x0E, 0x3B, 0x00, 0x8B, 0x04, 0x81, 0xC3] },
    // sub rsp,0x38; mov edx,ecx; movaps [rsp+0x20],xmm6: Re:CoM's effect end by handle (`0x1402585B0(handle, fade)`,
    // fade in xmm1, ≤ 0 at once through `0x140254D60`; 186 Re:CoM callers). M11, the Yellow Opera's warning
    // (`win::kh1_thrown::warn`). Checked (exe + decomp + disasm, 2026-11-03).
    Site { name: "effect end (0x1402585B0)", rva: 0x0025_85B0, expect: &[0x48, 0x83, 0xEC, 0x38, 0x8B, 0xD1, 0x0F, 0x29, 0x74, 0x24, 0x20] },
];

/// The thrown blade's look: Re:CoM's Aero level-1 shot effect set (`0x1405A2030`, entry 24 of
/// `0x1405A1730`): its slot 1's name pointer (relocated) is `"airo_all"`. Checked (exe).
pub const THROWN_PTRS: &[PtrSite] = &[PtrSite { name: "Aero level 1 shot set slot 1 name (0x1405A2048)", rva: 0x005A_2048, target: 0x005A_3150 }];

/// M11 fliers: the Air Pirate's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe +
/// decomp): its only caller is the enemy factory `0x140188F40` (case 22, the call at `0x140189338`);
/// no branch lands inside the moved bytes (exe scan: rel32 calls / jumps, jcc, rel8 jumps).
pub const AIRPIRATE_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyAirpirate ctor (0x14018A0B0)", rva: 0x0018_A0B0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 fliers: the factory's `mov ecx, 0xCD0` before `CEnemyAirpirate`'s ctor (its states use fields up
/// to `+0xCC0`), raised by the framework's 0x20-byte tail. Checked (exe).
pub const AIRPIRATE_ALLOC: &[Site] = &[Site { name: "alloc CEnemyAirpirate (0x14018931F)", rva: 0x0018_931F, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 fliers: `CEnemyAirpirate`'s vtable slots the copy keeps or replaces, checked against Re:CoM's
/// functions before the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 /
/// 30 are `CBattleEnemy`'s, 1..3 and the think the Air Pirate's own: 1 the flying walk `0x14018A310` →
/// `0x1401979F0` / `0x14018A4D0` → `0x140197450`, 2 `e022_050` `0x14018A270` / `0x14018A370`, 3
/// `e022_051` `0x14018A2C0` / `0x14018A420`, the think `0x14018A190`).
pub const AIRPIRATE_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyAirpirate +0x70 motion play", rva: 0x0053_0A58, target: 0x0017_D750 },
    PtrSite { name: "CEnemyAirpirate +0x80 update", rva: 0x0053_0A68, target: 0x0018_5320 },
    PtrSite { name: "CEnemyAirpirate +0xE8 state 0 begin", rva: 0x0053_0AD0, target: 0x0017_B590 },
    PtrSite { name: "CEnemyAirpirate +0xF0 state 0 step", rva: 0x0053_0AD8, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyAirpirate +0xF8 state 0 end", rva: 0x0053_0AE0, target: 0x0017_E630 },
    PtrSite { name: "CEnemyAirpirate +0x100 state 1 begin", rva: 0x0053_0AE8, target: 0x0018_A310 },
    PtrSite { name: "CEnemyAirpirate +0x108 state 1 step", rva: 0x0053_0AF0, target: 0x0018_A4D0 },
    PtrSite { name: "CEnemyAirpirate +0x118 state 2 begin", rva: 0x0053_0B00, target: 0x0018_A270 },
    PtrSite { name: "CEnemyAirpirate +0x120 state 2 step", rva: 0x0053_0B08, target: 0x0018_A370 },
    PtrSite { name: "CEnemyAirpirate +0x130 state 3 begin", rva: 0x0053_0B18, target: 0x0018_A2C0 },
    PtrSite { name: "CEnemyAirpirate +0x138 state 3 step", rva: 0x0053_0B20, target: 0x0018_A420 },
    PtrSite { name: "CEnemyAirpirate +0x1F0 state 11 begin", rva: 0x0053_0BD8, target: 0x0017_C530 },
    PtrSite { name: "CEnemyAirpirate +0x208 state 12 begin", rva: 0x0053_0BF0, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyAirpirate +0x3B8 state 30 begin", rva: 0x0053_0DA0, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyAirpirate +0x3C0 state 30 step", rva: 0x0053_0DA8, target: 0x0018_3260 },
    PtrSite { name: "CEnemyAirpirate +0x3D0 think", rva: 0x0053_0DB8, target: 0x0018_A190 },
    PtrSite { name: "CEnemyAirpirate +0x438 attack hurt off", rva: 0x0053_0E20, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyAirpirate +0x440 attack end", rva: 0x0053_0E28, target: 0x0017_DA90 },
];

/// M11 fliers: the Wyvern's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp):
/// its only caller is the enemy factory `0x140188F40` (case 24, the call at `0x14018938C`); no branch lands
/// inside the moved bytes and no pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps,
/// rip-relative operands, 8-byte pointers).
pub const WYVERN_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyWyvern ctor (0x140195ED0)", rva: 0x0019_5ED0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 fliers: the factory's `mov ecx, 0xCF0` before `CEnemyWyvern`'s ctor (its states use fields up to
/// `+0xCE8`), raised by the framework's 0x20-byte tail. Checked (exe).
pub const WYVERN_ALLOC: &[Site] = &[Site { name: "alloc CEnemyWyvern (0x140189373)", rva: 0x0018_9373, expect: &[0xB9, 0xF0, 0x0C, 0x00, 0x00] }];

/// M11 fliers: `CEnemyWyvern`'s vtable slots the copy keeps or replaces, checked against Re:CoM's functions
/// before the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 / 30 are
/// `CBattleEnemy`'s, 1..3 the Wyvern's own: 1 the flying walk `0x1401960B0` → `0x1401979F0` /
/// `0x1401964E0`, 2 `e024_002` → `e024_050` `0x140195FB0` / `0x1401961B0`, 3 `e024_051` → `052` → `053`
/// `0x140196060` / `0x140196350`; the think `CHeartless`'s `0x140196EC0`).
pub const WYVERN_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyWyvern +0x70 motion play", rva: 0x0055_5E38, target: 0x0017_D750 },
    PtrSite { name: "CEnemyWyvern +0x80 update", rva: 0x0055_5E48, target: 0x0018_5320 },
    PtrSite { name: "CEnemyWyvern +0xE8 state 0 begin", rva: 0x0055_5EB0, target: 0x0017_B590 },
    PtrSite { name: "CEnemyWyvern +0xF0 state 0 step", rva: 0x0055_5EB8, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyWyvern +0xF8 state 0 end", rva: 0x0055_5EC0, target: 0x0017_E630 },
    PtrSite { name: "CEnemyWyvern +0x100 state 1 begin", rva: 0x0055_5EC8, target: 0x0019_60B0 },
    PtrSite { name: "CEnemyWyvern +0x108 state 1 step", rva: 0x0055_5ED0, target: 0x0019_64E0 },
    PtrSite { name: "CEnemyWyvern +0x118 state 2 begin", rva: 0x0055_5EE0, target: 0x0019_5FB0 },
    PtrSite { name: "CEnemyWyvern +0x120 state 2 step", rva: 0x0055_5EE8, target: 0x0019_61B0 },
    PtrSite { name: "CEnemyWyvern +0x130 state 3 begin", rva: 0x0055_5EF8, target: 0x0019_6060 },
    PtrSite { name: "CEnemyWyvern +0x138 state 3 step", rva: 0x0055_5F00, target: 0x0019_6350 },
    PtrSite { name: "CEnemyWyvern +0x1F0 state 11 begin", rva: 0x0055_5FB8, target: 0x0017_C530 },
    PtrSite { name: "CEnemyWyvern +0x208 state 12 begin", rva: 0x0055_5FD0, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyWyvern +0x3B8 state 30 begin", rva: 0x0055_6180, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyWyvern +0x3C0 state 30 step", rva: 0x0055_6188, target: 0x0018_3260 },
    PtrSite { name: "CEnemyWyvern +0x3D0 think", rva: 0x0055_6198, target: 0x0019_6EC0 },
    PtrSite { name: "CEnemyWyvern +0x438 attack hurt off", rva: 0x0055_6200, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyWyvern +0x440 attack end", rva: 0x0055_6208, target: 0x0017_DA90 },
];

/// M11 fliers: the Search Ghost's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp):
/// its only caller is the enemy factory `0x140188F40` (case 14, the call at `0x1401891E8`); no branch lands inside
/// the moved bytes and no pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, rip-relative
/// operands, 8-byte pointers).
pub const SEARCHGHOST_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemySearchghost ctor (0x1401923F0)", rva: 0x0019_23F0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 fliers: the factory's `mov ecx, 0xCE0` before `CEnemySearchghost`'s ctor (its fields go up to `+0xCD0`,
/// the drain effect's handle its ctor zeroes and its dtor releases), raised by the framework's 0x20-byte tail.
/// Checked (exe + decomp).
pub const SEARCHGHOST_ALLOC: &[Site] = &[Site { name: "alloc CEnemySearchghost (0x1401891CF)", rva: 0x0018_91CF, expect: &[0xB9, 0xE0, 0x0C, 0x00, 0x00] }];

/// M11 fliers: `CEnemySearchghost`'s vtable slots the copy keeps or replaces, checked against Re:CoM's functions
/// before the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 / 30 are
/// `CBattleEnemy`'s, 1..3 the Search Ghost's own: 1 the flying walk `0x1401925B0` → `0x1401979F0` with
/// `e014_001` / `0x140192760`, 2 `e014_050` `0x140192510` / `0x1401925F0`, 3 its drain `e014_051`
/// `0x140192560` / `0x140192630` (a landed drain heals it 1/8 of its max HP, `0x140181CE0`); the think
/// `CHeartless`'s `0x140196EC0`).
pub const SEARCHGHOST_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemySearchghost +0x70 motion play", rva: 0x0054_C478, target: 0x0017_D750 },
    PtrSite { name: "CEnemySearchghost +0x80 update", rva: 0x0054_C488, target: 0x0018_5320 },
    PtrSite { name: "CEnemySearchghost +0xE8 state 0 begin", rva: 0x0054_C4F0, target: 0x0017_B590 },
    PtrSite { name: "CEnemySearchghost +0xF0 state 0 step", rva: 0x0054_C4F8, target: 0x0018_29E0 },
    PtrSite { name: "CEnemySearchghost +0xF8 state 0 end", rva: 0x0054_C500, target: 0x0017_E630 },
    PtrSite { name: "CEnemySearchghost +0x100 state 1 begin", rva: 0x0054_C508, target: 0x0019_25B0 },
    PtrSite { name: "CEnemySearchghost +0x108 state 1 step", rva: 0x0054_C510, target: 0x0019_2760 },
    PtrSite { name: "CEnemySearchghost +0x118 state 2 begin", rva: 0x0054_C520, target: 0x0019_2510 },
    PtrSite { name: "CEnemySearchghost +0x120 state 2 step", rva: 0x0054_C528, target: 0x0019_25F0 },
    PtrSite { name: "CEnemySearchghost +0x130 state 3 begin", rva: 0x0054_C538, target: 0x0019_2560 },
    PtrSite { name: "CEnemySearchghost +0x138 state 3 step", rva: 0x0054_C540, target: 0x0019_2630 },
    PtrSite { name: "CEnemySearchghost +0x1F0 state 11 begin", rva: 0x0054_C5F8, target: 0x0017_C530 },
    PtrSite { name: "CEnemySearchghost +0x208 state 12 begin", rva: 0x0054_C610, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemySearchghost +0x3B8 state 30 begin", rva: 0x0054_C7C0, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemySearchghost +0x3C0 state 30 step", rva: 0x0054_C7C8, target: 0x0018_3260 },
    PtrSite { name: "CEnemySearchghost +0x3D0 think", rva: 0x0054_C7D8, target: 0x0019_6EC0 },
    // The shared enemy-guard step: the hit handler's guard event (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemySearchghost +0x3D8 guard event", rva: 0x0054_C7E0, target: 0x0018_1600 },
    // Its prize point (`0x1401A4700`: the `+0x198` data's node and offset, else bone 0): where Re:CoM's defeat
    // makes its prizes (`0x14017BBC0`) and its drain heal reads (`0x140192630`); our reward's place.
    PtrSite { name: "CEnemySearchghost +0x50 prize point", rva: 0x0054_C458, target: 0x001A_4700 },
    PtrSite { name: "CEnemySearchghost +0x438 attack hurt off", rva: 0x0054_C840, target: 0x0017_DA80 },
    PtrSite { name: "CEnemySearchghost +0x440 attack end", rva: 0x0054_C848, target: 0x0017_DA90 },
];

/// M11, a KH1 script's heal and reward on Re:CoM's own HP, EXP and prizes (`win::kh1_enemy`'s
/// `Cmd::HealFull` / `Cmd::Reward`; `docs/recom-m11-enemies.md` § The Search Ghost). Called only. Bytes read
/// from the Steam exe (2026-11-02).
pub const KH1_ENEMY_REWARD_CALLEES: &[Site] = &[
    // push rbx; push rbp; push rsi; push rdi; push r12; push r14; push r15; sub rsp,0xA0: Re:CoM's enemy heal
    // (HP + n clamped to `0x14017F5A0`, the HP gauge through `+0x430`, `0x14020C9A0`), the one its Search Ghost's
    // drain calls (`0x140192630`).
    Site { name: "enemy heal (0x140181CE0)", rva: 0x0018_1CE0, expect: &[0x40, 0x53, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57, 0x48, 0x81, 0xEC, 0xA0, 0x00, 0x00, 0x00] },
    // mov r11,rsp; sub rsp,0xB8: Re:CoM's defeat EXP drop (`+0x2E4` split into EXP prizes by the table
    // `0x1407C2C78`, each by the prize factory), the first of its defeat prizes (`0x14017BBC0`).
    Site { name: "defeat EXP drop (0x1401810A0)", rva: 0x0018_10A0, expect: &[0x4C, 0x8B, 0xDC, 0x48, 0x81, 0xEC, 0xB8, 0x00, 0x00, 0x00] },
];

/// M11 ground melee: the Pirate's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp):
/// its only caller is the enemy factory `0x140188F40` (case 21, the call at `0x14018930E`); no branch lands
/// inside the moved bytes and no pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, 8-byte
/// pointers; 2026-11-02).
pub const PIRATE_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyPirate ctor (0x140190F30)", rva: 0x0019_0F30, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 ground melee: the factory's `mov ecx, 0xCD0` before `CEnemyPirate`'s ctor, raised by the framework's
/// 0x20-byte tail. Checked (exe).
pub const PIRATE_ALLOC: &[Site] = &[Site { name: "alloc CEnemyPirate (0x1401892F5)", rva: 0x0018_92F5, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 ground melee: `CEnemyPirate`'s vtable slots the copy keeps or replaces, checked against Re:CoM's
/// functions before the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 / 30
/// are `CBattleEnemy`'s, 1..3 the Pirate's own: 2 its spin `e021_050` `0x140191000` / `0x1401910C0`, 3 its
/// slash `e021_051` `0x140191040` / `0x140191120`; the think `CHeartless`'s `0x140196EC0`).
pub const PIRATE_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyPirate +0x70 motion play", rva: 0x0054_64C8, target: 0x0017_D750 },
    PtrSite { name: "CEnemyPirate +0x80 update", rva: 0x0054_64D8, target: 0x0018_5320 },
    PtrSite { name: "CEnemyPirate +0xE8 state 0 begin", rva: 0x0054_6540, target: 0x0017_B590 },
    PtrSite { name: "CEnemyPirate +0xF0 state 0 step", rva: 0x0054_6548, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyPirate +0xF8 state 0 end", rva: 0x0054_6550, target: 0x0017_E630 },
    PtrSite { name: "CEnemyPirate +0x100 state 1 begin", rva: 0x0054_6558, target: 0x0019_1090 },
    PtrSite { name: "CEnemyPirate +0x108 state 1 step", rva: 0x0054_6560, target: 0x0019_1370 },
    PtrSite { name: "CEnemyPirate +0x118 state 2 begin", rva: 0x0054_6570, target: 0x0019_1000 },
    PtrSite { name: "CEnemyPirate +0x120 state 2 step", rva: 0x0054_6578, target: 0x0019_10C0 },
    PtrSite { name: "CEnemyPirate +0x130 state 3 begin", rva: 0x0054_6588, target: 0x0019_1040 },
    PtrSite { name: "CEnemyPirate +0x138 state 3 step", rva: 0x0054_6590, target: 0x0019_1120 },
    PtrSite { name: "CEnemyPirate +0x1F0 state 11 begin", rva: 0x0054_6648, target: 0x0017_C530 },
    PtrSite { name: "CEnemyPirate +0x208 state 12 begin", rva: 0x0054_6660, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyPirate +0x3B8 state 30 begin", rva: 0x0054_6810, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyPirate +0x3C0 state 30 step", rva: 0x0054_6818, target: 0x0018_3260 },
    PtrSite { name: "CEnemyPirate +0x3D0 think", rva: 0x0054_6828, target: 0x0019_6EC0 },
    // The shared enemy-guard step: the hit handler's guard event (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyPirate +0x3D8 guard event", rva: 0x0054_6830, target: 0x0018_1600 },
    PtrSite { name: "CEnemyPirate +0x438 attack hurt off", rva: 0x0054_6890, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyPirate +0x440 attack end", rva: 0x0054_6898, target: 0x0017_DA90 },
];

/// M11 ground melee: the Large Body's ctor (the Soldier's shape: `40 53 48 83 EC 20`, then `45 33 C9 45 33 C0 48 8B D9
/// E8` into `CHeartless`'s base ctor `0x140196DF0`). Same rule as [`HOOKS`]. Checked (exe + decomp): its only caller is
/// the enemy factory `0x140188F40` (case 9, the call at `0x140189116`); no branch lands inside the moved bytes and no
/// pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, 8-byte pointers; 2026-11-02).
pub const LARGEBODY_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyLargebody ctor (0x14018F090)", rva: 0x0018_F090, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 ground melee: the factory's `mov ecx, 0xCE0` before `CEnemyLargebody`'s ctor (its states use fields up to
/// `+0xCD0`), raised by the framework's 0x20-byte tail. Checked (exe).
pub const LARGEBODY_ALLOC: &[Site] = &[Site { name: "alloc CEnemyLargebody (0x1401890FD)", rva: 0x0018_90FD, expect: &[0xB9, 0xE0, 0x0C, 0x00, 0x00] }];

/// M11 ground melee: `CEnemyLargebody`'s vtable slots the copy keeps or replaces, checked against Re:CoM's functions
/// before the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 / 30 are `CBattleEnemy`'s,
/// 1..3 the Large Body's own: 1 `0x14018F1F0` / the walk thunk `0x14018F3E0`, 2 its belly bump `e009_050`
/// `0x14018F150` / `0x14018F220`, 3 its stomp `e009_053` `0x14018F1A0` / `0x14018F280`; the think `CHeartless`'s
/// `0x140196EC0`).
pub const LARGEBODY_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyLargebody +0x70 motion play", rva: 0x0054_2B40, target: 0x0017_D750 },
    PtrSite { name: "CEnemyLargebody +0x80 update", rva: 0x0054_2B50, target: 0x0018_5320 },
    PtrSite { name: "CEnemyLargebody +0xE8 state 0 begin", rva: 0x0054_2BB8, target: 0x0017_B590 },
    PtrSite { name: "CEnemyLargebody +0xF0 state 0 step", rva: 0x0054_2BC0, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyLargebody +0xF8 state 0 end", rva: 0x0054_2BC8, target: 0x0017_E630 },
    PtrSite { name: "CEnemyLargebody +0x100 state 1 begin", rva: 0x0054_2BD0, target: 0x0018_F1F0 },
    PtrSite { name: "CEnemyLargebody +0x108 state 1 step", rva: 0x0054_2BD8, target: 0x0018_F3E0 },
    PtrSite { name: "CEnemyLargebody +0x118 state 2 begin", rva: 0x0054_2BE8, target: 0x0018_F150 },
    PtrSite { name: "CEnemyLargebody +0x120 state 2 step", rva: 0x0054_2BF0, target: 0x0018_F220 },
    PtrSite { name: "CEnemyLargebody +0x130 state 3 begin", rva: 0x0054_2C00, target: 0x0018_F1A0 },
    PtrSite { name: "CEnemyLargebody +0x138 state 3 step", rva: 0x0054_2C08, target: 0x0018_F280 },
    PtrSite { name: "CEnemyLargebody +0x1F0 state 11 begin", rva: 0x0054_2CC0, target: 0x0017_C530 },
    PtrSite { name: "CEnemyLargebody +0x208 state 12 begin", rva: 0x0054_2CD8, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyLargebody +0x3B8 state 30 begin", rva: 0x0054_2E88, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyLargebody +0x3C0 state 30 step", rva: 0x0054_2E90, target: 0x0018_3260 },
    PtrSite { name: "CEnemyLargebody +0x3D0 think", rva: 0x0054_2EA0, target: 0x0019_6EC0 },
    // The shared enemy-guard step: the hit handler's guard event (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyLargebody +0x3D8 guard event", rva: 0x0054_2EA8, target: 0x0018_1600 },
    PtrSite { name: "CEnemyLargebody +0x438 attack hurt off", rva: 0x0054_2F08, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyLargebody +0x440 attack end", rva: 0x0054_2F10, target: 0x0017_DA90 },
];

/// M11 (the Large Body's stomp ring, `kh1_recom_assets::enemy::RINGS`): Re:CoM's attack-set radius setter, the call
/// its own Large Body's state 3 step makes each frame to grow its shockwave (`0x14018F280`: `0x14014E5C0(enemy+0x100,
/// +0xCD0)`): each record's collision radius through `0x140172690` (types 2, 4, 5, 7, 8, 11: `+0x70`), the radius in
/// xmm1 (`movaps xmm6, xmm1`). Called, never patched. Checked (exe + decomp).
pub const KH1_ENEMY_RING_CALLEES: &[Site] = &[
    // push rbx; sub rsp,0x30; mov eax,[rcx+0x14]; mov rbx,[rcx+0x18]
    Site { name: "attack set radius (0x14014E5C0)", rva: 0x0014_E5C0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x8B, 0x41, 0x14, 0x48, 0x8B, 0x59, 0x18] },
];

/// M11 (the Large Body's stomp ring as KH1's ring, `docs/recom-m11-enemies.md` § The stomp's ring): Re:CoM's
/// circle-against-body test, the pair dispatch's cases 0xB2 / 0xB4 / 0xB5 (`0x14016EAD0`: a type-11
/// `CCircleCollision` attack against a sphere or column body; its only caller), wrapped by
/// `win::kh1_enemy::circle_test`: a circle with a band (`+0x74` > 0, set only on our ring by `ring_step`) takes KH1's
/// ring rule, every other circle Re:CoM's test (the trampoline). Re:CoM's own circles keep `+0x74` at 0 (the
/// factory `0x1401720E0`; its only other writers, `0x140172690` / `0x140172720`, never write it on type 11: decomp
/// scan). The moved bytes are whole instructions with no rip operand; no branch or pointer lands inside them (exe
/// scan, pefile + capstone, 2026-11-02). Checked (exe + decomp).
pub const KH1_ENEMY_RING_HOOKS: &[Site] = &[
    // rex push rbp; push rsi; push rdi; sub rsp,0x50
    Site { name: "circle vs body (0x140177190)", rva: 0x0017_7190, expect: &[0x40, 0x55, 0x56, 0x57, 0x48, 0x83, 0xEC, 0x50] },
];

/// M11, attack meets attack (`docs/recom-m11-enemies.md` § Attack meets attack): Re:CoM's line-against-sphere overlap
/// test, the pair dispatch's case 0x72 (`0x14016EAD0`: a type-7 `CPoly3Collision` attack against a sphere body), which
/// `win::attack::attack_meets` calls with Sora's swing sphere as the body and a copy of a KH1 enemy row's line record
/// as the attack (the sphere records use `0x1401777E0`, in [`ATTACK_CALLEES`]). Called, never patched. Checked (exe +
/// decomp).
pub const KH1_ENEMY_CLASH_CALLEES: &[Site] = &[
    // push rbp; push rbx; push rsi; push rdi; push r12; push r14; push r15; lea rbp,[rsp-0x27]
    Site { name: "line vs sphere (0x140177480)", rva: 0x0017_7480, expect: &[0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x54, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8D, 0x6C, 0x24, 0xD9] },
];

/// M11 ground melee: the Neoshadow's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp):
/// its only caller is the enemy factory `0x140188F40` (case 27, the call at `0x14018940A`); no branch lands inside
/// the moved bytes and no pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, 8-byte pointers;
/// 2026-11-02).
pub const NEOSHADOW_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyNeoshadow ctor (0x140190150)", rva: 0x0019_0150, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 ground melee: the factory's `mov ecx, 0xCE0` before `CEnemyNeoshadow`'s ctor, raised by the framework's
/// 0x20-byte tail. Checked (exe).
pub const NEOSHADOW_ALLOC: &[Site] = &[Site { name: "alloc CEnemyNeoshadow (0x1401893F1)", rva: 0x0018_93F1, expect: &[0xB9, 0xE0, 0x0C, 0x00, 0x00] }];

/// M11 ground melee: `CEnemyNeoshadow`'s vtable slots the copy keeps or replaces, checked against Re:CoM's
/// functions before the copy is made (Checked (exe), relocated pointers; decomp: states 0..3 the Neoshadow's own:
/// 0 its rise `e027_011` `0x140190220` / `0x140190480` (end `0x140190400`), 1 `0x1401903E0` / `0x140190860`, 2 its
/// claw `e027_050` `0x140190330` / `0x140190540`, 3 its sink and come-up strike `e027_051..053` `0x140190380` /
/// `0x1401905A0`; 11 / 12 / 30 `CBattleEnemy`'s; the think `CHeartless`'s `0x140196EC0`).
pub const NEOSHADOW_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyNeoshadow +0x70 motion play", rva: 0x0054_5198, target: 0x0017_D750 },
    PtrSite { name: "CEnemyNeoshadow +0x80 update", rva: 0x0054_51A8, target: 0x0019_0E50 },
    PtrSite { name: "CEnemyNeoshadow +0xE8 state 0 begin", rva: 0x0054_5210, target: 0x0019_0220 },
    PtrSite { name: "CEnemyNeoshadow +0xF0 state 0 step", rva: 0x0054_5218, target: 0x0019_0480 },
    PtrSite { name: "CEnemyNeoshadow +0xF8 state 0 end", rva: 0x0054_5220, target: 0x0019_0400 },
    PtrSite { name: "CEnemyNeoshadow +0x100 state 1 begin", rva: 0x0054_5228, target: 0x0019_03E0 },
    PtrSite { name: "CEnemyNeoshadow +0x108 state 1 step", rva: 0x0054_5230, target: 0x0019_0860 },
    PtrSite { name: "CEnemyNeoshadow +0x118 state 2 begin", rva: 0x0054_5240, target: 0x0019_0330 },
    PtrSite { name: "CEnemyNeoshadow +0x120 state 2 step", rva: 0x0054_5248, target: 0x0019_0540 },
    PtrSite { name: "CEnemyNeoshadow +0x130 state 3 begin", rva: 0x0054_5258, target: 0x0019_0380 },
    PtrSite { name: "CEnemyNeoshadow +0x138 state 3 step", rva: 0x0054_5260, target: 0x0019_05A0 },
    PtrSite { name: "CEnemyNeoshadow +0x1F0 state 11 begin", rva: 0x0054_5318, target: 0x0017_C530 },
    PtrSite { name: "CEnemyNeoshadow +0x208 state 12 begin", rva: 0x0054_5330, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyNeoshadow +0x3B8 state 30 begin", rva: 0x0054_54E0, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyNeoshadow +0x3C0 state 30 step", rva: 0x0054_54E8, target: 0x0018_3260 },
    PtrSite { name: "CEnemyNeoshadow +0x3D0 think", rva: 0x0054_54F8, target: 0x0019_6EC0 },
    // The shared enemy-guard step: the hit handler's guard event (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyNeoshadow +0x3D8 guard event", rva: 0x0054_5500, target: 0x0018_1600 },
    PtrSite { name: "CEnemyNeoshadow +0x438 attack hurt off", rva: 0x0054_5560, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyNeoshadow +0x440 attack end", rva: 0x0054_5568, target: 0x0017_DA90 },
];
/// M11 ground melee: the Wight Knight's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp):
/// its only caller is the enemy factory `0x140188F40` (case 12, the call at `0x140189194`); no branch lands inside the
/// moved bytes and no pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, 8-byte pointers; 2026-11-02).
pub const WIGHTKNIGHT_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyWightnight ctor (0x140194ED0)", rva: 0x0019_4ED0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 ground melee: the factory's `mov ecx, 0xCD0` before `CEnemyWightnight`'s ctor, raised by the framework's
/// 0x20-byte tail. Checked (exe).
pub const WIGHTKNIGHT_ALLOC: &[Site] = &[Site { name: "alloc CEnemyWightnight (0x14018917B)", rva: 0x0018_917B, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 ground melee: `CEnemyWightnight`'s vtable slots the copy keeps or replaces, checked against Re:CoM's functions
/// before the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 / 30 are `CBattleEnemy`'s,
/// 1..3 the Wight Knight's own: 1 `0x140195040` / `0x140195410`, 2 its two-strike claw `e012_052` `0x140194FA0` /
/// `0x140195090` (then sometimes its jump `e012_003..007`), 3 its wind-up `e012_053` and lunge `e012_054` `0x140194FF0` /
/// `0x140195320`; the attack states' ends its own `ret`s, as `CBattleEnemy`'s; the think `CHeartless`'s `0x140196EC0`).
pub const WIGHTKNIGHT_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyWightnight +0x70 motion play", rva: 0x0055_37B8, target: 0x0017_D750 },
    PtrSite { name: "CEnemyWightnight +0x80 update", rva: 0x0055_37C8, target: 0x0018_5320 },
    PtrSite { name: "CEnemyWightnight +0xE8 state 0 begin", rva: 0x0055_3830, target: 0x0017_B590 },
    PtrSite { name: "CEnemyWightnight +0xF0 state 0 step", rva: 0x0055_3838, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyWightnight +0xF8 state 0 end", rva: 0x0055_3840, target: 0x0017_E630 },
    PtrSite { name: "CEnemyWightnight +0x100 state 1 begin", rva: 0x0055_3848, target: 0x0019_5040 },
    PtrSite { name: "CEnemyWightnight +0x108 state 1 step", rva: 0x0055_3850, target: 0x0019_5410 },
    PtrSite { name: "CEnemyWightnight +0x118 state 2 begin", rva: 0x0055_3860, target: 0x0019_4FA0 },
    PtrSite { name: "CEnemyWightnight +0x120 state 2 step", rva: 0x0055_3868, target: 0x0019_5090 },
    PtrSite { name: "CEnemyWightnight +0x130 state 3 begin", rva: 0x0055_3878, target: 0x0019_4FF0 },
    PtrSite { name: "CEnemyWightnight +0x138 state 3 step", rva: 0x0055_3880, target: 0x0019_5320 },
    PtrSite { name: "CEnemyWightnight +0x1F0 state 11 begin", rva: 0x0055_3938, target: 0x0017_C530 },
    PtrSite { name: "CEnemyWightnight +0x208 state 12 begin", rva: 0x0055_3950, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyWightnight +0x3B8 state 30 begin", rva: 0x0055_3B00, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyWightnight +0x3C0 state 30 step", rva: 0x0055_3B08, target: 0x0018_3260 },
    PtrSite { name: "CEnemyWightnight +0x3D0 think", rva: 0x0055_3B18, target: 0x0019_6EC0 },
    // The shared enemy-guard step: the hit handler's guard event (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyWightnight +0x3D8 guard event", rva: 0x0055_3B20, target: 0x0018_1600 },
    PtrSite { name: "CEnemyWightnight +0x438 attack hurt off", rva: 0x0055_3B80, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyWightnight +0x440 attack end", rva: 0x0055_3B88, target: 0x0017_DA90 },
];

/// M11 casters: the Fat Bandit's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp): its only
/// caller is the enemy factory `0x140188F40` (case 10, the call at `0x140189140`); no branch lands inside the moved bytes
/// and no pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, 8-byte pointers; 2026-11-02).
pub const FATBANDIT_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyFatbandit ctor (0x14018DD70)", rva: 0x0018_DD70, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 casters: the factory's `mov ecx, 0xCD0` before `CEnemyFatbandit`'s ctor, raised by the framework's 0x20-byte
/// tail. Checked (exe).
pub const FATBANDIT_ALLOC: &[Site] = &[Site { name: "alloc CEnemyFatbandit (0x140189127)", rva: 0x0018_9127, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 casters: `CEnemyFatbandit`'s vtable slots the copy keeps or replaces, checked against Re:CoM's functions before
/// the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 / 30 are `CBattleEnemy`'s, 1..3 the
/// Fat Bandit's own: 1 `0x14018DEB0` / the walk thunk `0x14018E140`, 2 its belly `e010_050` `0x14018DE30` /
/// `0x14018DEE0`, 3 its fireball `e010_051` `0x14018DE60` / `0x14018DF20` (a `CBullet` from `head2`, homing 30 frames);
/// the attack states' ends `CBattleEnemy`'s `0x14017E6C0` / `0x14017E700`; the think `CHeartless`'s `0x140196EC0`).
pub const FATBANDIT_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyFatbandit +0x70 motion play", rva: 0x0053_DE68, target: 0x0017_D750 },
    PtrSite { name: "CEnemyFatbandit +0x80 update", rva: 0x0053_DE78, target: 0x0018_5320 },
    PtrSite { name: "CEnemyFatbandit +0xE8 state 0 begin", rva: 0x0053_DEE0, target: 0x0017_B590 },
    PtrSite { name: "CEnemyFatbandit +0xF0 state 0 step", rva: 0x0053_DEE8, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyFatbandit +0xF8 state 0 end", rva: 0x0053_DEF0, target: 0x0017_E630 },
    PtrSite { name: "CEnemyFatbandit +0x100 state 1 begin", rva: 0x0053_DEF8, target: 0x0018_DEB0 },
    PtrSite { name: "CEnemyFatbandit +0x108 state 1 step", rva: 0x0053_DF00, target: 0x0018_E140 },
    PtrSite { name: "CEnemyFatbandit +0x118 state 2 begin", rva: 0x0053_DF10, target: 0x0018_DE30 },
    PtrSite { name: "CEnemyFatbandit +0x120 state 2 step", rva: 0x0053_DF18, target: 0x0018_DEE0 },
    PtrSite { name: "CEnemyFatbandit +0x130 state 3 begin", rva: 0x0053_DF28, target: 0x0018_DE60 },
    PtrSite { name: "CEnemyFatbandit +0x138 state 3 step", rva: 0x0053_DF30, target: 0x0018_DF20 },
    PtrSite { name: "CEnemyFatbandit +0x1F0 state 11 begin", rva: 0x0053_DFE8, target: 0x0017_C530 },
    PtrSite { name: "CEnemyFatbandit +0x208 state 12 begin", rva: 0x0053_E000, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyFatbandit +0x3B8 state 30 begin", rva: 0x0053_E1B0, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyFatbandit +0x3C0 state 30 step", rva: 0x0053_E1B8, target: 0x0018_3260 },
    PtrSite { name: "CEnemyFatbandit +0x3D0 think", rva: 0x0053_E1C8, target: 0x0019_6EC0 },
    // The shared enemy-guard step: the hit handler's guard event (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyFatbandit +0x3D8 guard event", rva: 0x0053_E1D0, target: 0x0018_1600 },
    PtrSite { name: "CEnemyFatbandit +0x438 attack hurt off", rva: 0x0053_E230, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyFatbandit +0x440 attack end", rva: 0x0053_E238, target: 0x0017_DA90 },
];

/// M11 casters: the Fat Bandit's own fireball's effect sets (`0x14018DF20`'s descriptor `+0xA8` = `0x14053E290`: slots
/// 1..3 `"_ex_2060"` 7 / 15 / 5, the model's own effects; `+0xB8` = `0x14053E300`), copied into its KH1 fireballs and
/// flame (owner 2026-11-02: "drawn with a Re:CoM effect (its own fireball's)"). Pointer-checked: slot 1's name pointer.
/// Checked (exe + decomp).
pub const FATBANDIT_FIRE_PTRS: &[PtrSite] = &[PtrSite { name: "the Fat Bandit's fireball effect set slot 1 name", rva: 0x0053_E2A8, target: 0x0053_E2F0 }];

/// M11 fliers: the Gargoyle's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp): its only caller
/// is the enemy factory `0x140188F40` (case 13, the call at `0x1401891BE`); no branch lands inside the moved bytes and no
/// pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, 8-byte pointers; 2026-11-02).
pub const GARGOYLE_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyGargoyle ctor (0x14018E150)", rva: 0x0018_E150, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 fliers: the factory's `mov ecx, 0xCD0` before `CEnemyGargoyle`'s ctor, raised by the framework's 0x20-byte tail.
/// Checked (exe).
pub const GARGOYLE_ALLOC: &[Site] = &[Site { name: "alloc CEnemyGargoyle (0x1401891A5)", rva: 0x0018_91A5, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 fliers: `CEnemyGargoyle`'s vtable slots the copy keeps or replaces, checked against Re:CoM's functions before the
/// copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 / 30 are `CBattleEnemy`'s, 1..3 the
/// Gargoyle's own: 1 `0x14018E2C0` / `0x14018E560` (→ `0x140197450`), its end `0x14018E300` (→ `0x1401979C0`, kept), 2 its
/// wing strike `e013_050` `0x14018E220` / `0x14018E310`, 3 its shot `e013_051` `0x14018E270` / `0x14018E350`; the attack
/// states' ends `CBattleEnemy`'s `0x14017E6C0` / `0x14017E700`; the think `CHeartless`'s `0x140196EC0`).
pub const GARGOYLE_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyGargoyle +0x70 motion play", rva: 0x0053_F208, target: 0x0017_D750 },
    PtrSite { name: "CEnemyGargoyle +0x80 update", rva: 0x0053_F218, target: 0x0018_5320 },
    PtrSite { name: "CEnemyGargoyle +0xE8 state 0 begin", rva: 0x0053_F280, target: 0x0017_B590 },
    PtrSite { name: "CEnemyGargoyle +0xF0 state 0 step", rva: 0x0053_F288, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyGargoyle +0xF8 state 0 end", rva: 0x0053_F290, target: 0x0017_E630 },
    PtrSite { name: "CEnemyGargoyle +0x100 state 1 begin", rva: 0x0053_F298, target: 0x0018_E2C0 },
    PtrSite { name: "CEnemyGargoyle +0x108 state 1 step", rva: 0x0053_F2A0, target: 0x0018_E560 },
    PtrSite { name: "CEnemyGargoyle +0x110 state 1 end", rva: 0x0053_F2A8, target: 0x0018_E300 },
    PtrSite { name: "CEnemyGargoyle +0x118 state 2 begin", rva: 0x0053_F2B0, target: 0x0018_E220 },
    PtrSite { name: "CEnemyGargoyle +0x120 state 2 step", rva: 0x0053_F2B8, target: 0x0018_E310 },
    PtrSite { name: "CEnemyGargoyle +0x128 state 2 end", rva: 0x0053_F2C0, target: 0x0017_E6C0 },
    PtrSite { name: "CEnemyGargoyle +0x130 state 3 begin", rva: 0x0053_F2C8, target: 0x0018_E270 },
    PtrSite { name: "CEnemyGargoyle +0x138 state 3 step", rva: 0x0053_F2D0, target: 0x0018_E350 },
    PtrSite { name: "CEnemyGargoyle +0x1F0 state 11 begin", rva: 0x0053_F388, target: 0x0017_C530 },
    PtrSite { name: "CEnemyGargoyle +0x208 state 12 begin", rva: 0x0053_F3A0, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyGargoyle +0x3B8 state 30 begin", rva: 0x0053_F550, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyGargoyle +0x3C0 state 30 step", rva: 0x0053_F558, target: 0x0018_3260 },
    PtrSite { name: "CEnemyGargoyle +0x3D0 think", rva: 0x0053_F568, target: 0x0019_6EC0 },
    // The shared enemy-guard step's slot (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyGargoyle +0x3D8 guard event", rva: 0x0053_F570, target: 0x0018_1600 },
    PtrSite { name: "CEnemyGargoyle +0x438 attack hurt off", rva: 0x0053_F5D0, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyGargoyle +0x440 attack end", rva: 0x0053_F5D8, target: 0x0017_DA90 },
];

/// M11 fliers: the Gargoyle's own shot's effect set (`0x14018E350`'s descriptor `+0xA8` = `0x14053F630`: slots 1..3
/// `"_ex_2220"` 8 / 9 / 7, the model's own effects), copied into its KH1 ball (owner 2026-11-02: projectiles drawn with a
/// Re:CoM effect). Pointer-checked: slot 1's name pointer. Checked (exe + decomp).
pub const GARGOYLE_BALL_PTRS: &[PtrSite] = &[PtrSite { name: "the Gargoyle's shot effect set slot 1 name", rva: 0x0053_F648, target: 0x0053_F690 }];

/// M11 fliers: `CCollAttack`'s "on" (vtable `0x140523E28` + 0x10 = `0x14014DE90`: `mov dword [rcx+0x10], 1; ret`), which
/// the Gargoyle's ball calls at its launch (its attack held off through its charge, `win::magic::hold_off`'s "off" at
/// +8). Pointer-checked before each call. Checked (exe + decomp).
pub const COLL_ON_PTRS: &[PtrSite] = &[PtrSite { name: "CCollAttack +0x10 on", rva: 0x0052_3E38, target: 0x0014_DE90 }];

/// M11 casters: Re:CoM's collision radius setter (`0x140172690(coll, r)`: a sphere's `+0x70` / `+0x74`, the others'
/// `+0x70`; 148 call sites, the attack set's `0x14014E5C0` among them), which the Fat Bandit's flame calls on its
/// spheres' bullets each frame (`win::kh1_flame`). Called only. Checked (exe + decomp).
pub const FLAME_CALLEES: &[Site] = &[
    // mov rdx,[rcx+0x18]; mov r8,rcx; test rdx,rdx
    Site { name: "collision radius setter (0x140172690)", rva: 0x0017_2690, expect: &[0x48, 0x8B, 0x51, 0x18, 0x4C, 0x8B, 0xC1, 0x48, 0x85, 0xD2] },
];

/// M11, the Neoshadow's grab (`win::held`, `docs/recom-kh1-attack.md` § The held reaction): Re:CoM's reaction prelude,
/// which its damage check `0x1401E6DE0` runs before every reaction it starts (`0x1401F6990(pl, 6)`); the held entry
/// runs it too. Called, never patched. The kind getter `0x140179410` (its table's `lea` displacement is patched) is
/// [`THROWN_CALLEES`]' site. Bytes read from the Steam exe (2026-11-02).
pub const HELD_CALLEES: &[Site] = &[
    // mov [rsp+0x10],rbx; mov [rsp+0x18],rbp; push rsi; push rdi; push r14; sub rsp,..
    Site { name: "player reaction prelude (0x1401F6990)", rva: 0x001F_6990, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x48, 0x89, 0x6C, 0x24, 0x18, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83] },
];

/// M11, the Neoshadow's grab on KH1's frame (`win::kh1_enemy::grab_test_now`, `kh1_core::enemy::host::starts_at_once`):
/// Re:CoM's attack check on an enemy's attack set (`0x14014E6A0(set) -> u32`: each live entry's collision against its
/// list, as the enemy's slot `+0x78` `0x140185630` calls it with `rcx` = `enemy+0x100`, the "connected" bit `+0x260 &
/// 0x40` from its result). Called only, once per grab. Checked (exe + decomp + disasm of `0x14018567B..0x1401856A1`).
pub const GRAB_CALLEES: &[Site] = &[
    // mov r11,rsp; push rbx; push rsi; push r14; sub rsp,0x70
    Site { name: "enemy attack check (0x14014E6A0)", rva: 0x0014_E6A0, expect: &[0x4C, 0x8B, 0xDC, 0x53, 0x56, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x70] },
];

/// M11, the Neoshadow's grab: `CAttackMotion`'s attack windows (vtable `0x140523E70` + 0x18 = `0x14014E830`: each entry's
/// window opened / closed by the motion clock, its collision set up with a new serial at the opening), the enemy's slot
/// `+0x90` (`0x140180C90`) calls it after the state machine. Pointer-checked before the call. Checked (exe + decomp).
pub const ATTACK_WINDOWS_PTRS: &[PtrSite] = &[PtrSite { name: "CAttackMotion +0x18 windows", rva: 0x0052_3E88, target: 0x0014_E830 }];

/// M11 casters: the Defender's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp): its only caller
/// is the enemy factory `0x140188F40` (case 26, the call at `0x1401893E0`); no branch lands inside the moved bytes and no
/// pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, 8-byte pointers; 2026-11-02).
pub const DEFENDER_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyDefender ctor (0x14018D8E0)", rva: 0x0018_D8E0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 casters: the factory's `mov ecx, 0xCD0` before `CEnemyDefender`'s ctor, raised by the framework's 0x20-byte tail.
/// Checked (exe).
pub const DEFENDER_ALLOC: &[Site] = &[Site { name: "alloc CEnemyDefender (0x1401893C7)", rva: 0x0018_93C7, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 casters: `CEnemyDefender`'s vtable slots the copy keeps or replaces, checked against Re:CoM's functions before the
/// copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 / 30 are `CBattleEnemy`'s, 1..3 the
/// Defender's own: 1 `0x14018DA40` / the walk thunk `0x14018DD60`, 2 its melee `e026_050` `0x14018D9A0` / `0x14018DA90`,
/// 3 its fireball `e026_051` → `e026_052` `0x14018D9F0` / `0x14018DAF0` (a `CBullet` from `lb_wep0`, homing 2° a frame
/// for 30 frames); the think `CHeartless`'s `0x140196EC0`).
pub const DEFENDER_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyDefender +0x70 motion play", rva: 0x0053_CB08, target: 0x0017_D750 },
    PtrSite { name: "CEnemyDefender +0x80 update", rva: 0x0053_CB18, target: 0x0018_5320 },
    PtrSite { name: "CEnemyDefender +0xE8 state 0 begin", rva: 0x0053_CB80, target: 0x0017_B590 },
    PtrSite { name: "CEnemyDefender +0xF0 state 0 step", rva: 0x0053_CB88, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyDefender +0xF8 state 0 end", rva: 0x0053_CB90, target: 0x0017_E630 },
    PtrSite { name: "CEnemyDefender +0x100 state 1 begin", rva: 0x0053_CB98, target: 0x0018_DA40 },
    PtrSite { name: "CEnemyDefender +0x108 state 1 step", rva: 0x0053_CBA0, target: 0x0018_DD60 },
    PtrSite { name: "CEnemyDefender +0x118 state 2 begin", rva: 0x0053_CBB0, target: 0x0018_D9A0 },
    PtrSite { name: "CEnemyDefender +0x120 state 2 step", rva: 0x0053_CBB8, target: 0x0018_DA90 },
    PtrSite { name: "CEnemyDefender +0x130 state 3 begin", rva: 0x0053_CBC8, target: 0x0018_D9F0 },
    PtrSite { name: "CEnemyDefender +0x138 state 3 step", rva: 0x0053_CBD0, target: 0x0018_DAF0 },
    PtrSite { name: "CEnemyDefender +0x1F0 state 11 begin", rva: 0x0053_CC88, target: 0x0017_C530 },
    PtrSite { name: "CEnemyDefender +0x208 state 12 begin", rva: 0x0053_CCA0, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyDefender +0x3B8 state 30 begin", rva: 0x0053_CE50, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyDefender +0x3C0 state 30 step", rva: 0x0053_CE58, target: 0x0018_3260 },
    PtrSite { name: "CEnemyDefender +0x3D0 think", rva: 0x0053_CE68, target: 0x0019_6EC0 },
    // The shared enemy-guard step: the hit handler's guard event (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyDefender +0x3D8 guard event", rva: 0x0053_CE70, target: 0x0018_1600 },
    PtrSite { name: "CEnemyDefender +0x438 attack hurt off", rva: 0x0053_CED0, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyDefender +0x440 attack end", rva: 0x0053_CED8, target: 0x0017_DA90 },
];

/// M11 casters: the Defender's own fireball's effect set (`0x14018DAF0`'s descriptor `+0xA8` = `0x14053CF30`: slots 1..3
/// `"_ex_2340"` 12 / 13 / 11, the model's own effects; no `+0xB8`), copied into its KH1 fireballs and flame (owner
/// 2026-11-02: "drawn with a Re:CoM effect (its own)"). Pointer-checked: slot 1's name pointer. Checked (exe + decomp).
pub const DEFENDER_FIRE_PTRS: &[PtrSite] = &[PtrSite { name: "the Defender's fireball effect set slot 1 name", rva: 0x0053_CF48, target: 0x0053_CF90 }];

/// M11 casters: the Bouncywild's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp): its only caller
/// is the enemy factory `0x140188F40` (case 8, the call at `0x1401890EC`); no branch lands inside the moved bytes and no
/// pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, 8-byte pointers; 2026-11-03).
pub const BOUNCYWILD_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyBouncywild ctor (0x14018C4D0)", rva: 0x0018_C4D0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 casters: the factory's `mov ecx, 0xCD0` before `CEnemyBouncywild`'s ctor, raised by the framework's 0x20-byte tail.
/// Checked (exe).
pub const BOUNCYWILD_ALLOC: &[Site] = &[Site { name: "alloc CEnemyBouncywild (0x1401890D3)", rva: 0x0018_90D3, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 casters: `CEnemyBouncywild`'s vtable slots the copy keeps or replaces, checked against Re:CoM's functions before the
/// copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 / 30 are `CBattleEnemy`'s, 1..3 the
/// Bouncywild's own: 1 `0x14018C630` (`e008_001` / `e008_002`) / the walk thunk `0x14018CA30`, 2 `e008_050` → `_051` →
/// `_052` `0x14018C590` / `0x14018C660` (a straight `CBullet` from `wep0`, attack id 0), 3 `e008_053` `0x14018C5E0` /
/// `0x14018C870` (a homing banana from `l_fing1a`, attack id 1, Sora's slip kind 0x18); the think `CHeartless`'s
/// `0x140196EC0`).
pub const BOUNCYWILD_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyBouncywild +0x70 motion play", rva: 0x0053_9028, target: 0x0017_D750 },
    PtrSite { name: "CEnemyBouncywild +0x80 update", rva: 0x0053_9038, target: 0x0018_5320 },
    PtrSite { name: "CEnemyBouncywild +0xE8 state 0 begin", rva: 0x0053_90A0, target: 0x0017_B590 },
    PtrSite { name: "CEnemyBouncywild +0xF0 state 0 step", rva: 0x0053_90A8, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyBouncywild +0xF8 state 0 end", rva: 0x0053_90B0, target: 0x0017_E630 },
    PtrSite { name: "CEnemyBouncywild +0x100 state 1 begin", rva: 0x0053_90B8, target: 0x0018_C630 },
    PtrSite { name: "CEnemyBouncywild +0x108 state 1 step", rva: 0x0053_90C0, target: 0x0018_CA30 },
    PtrSite { name: "CEnemyBouncywild +0x118 state 2 begin", rva: 0x0053_90D0, target: 0x0018_C590 },
    PtrSite { name: "CEnemyBouncywild +0x120 state 2 step", rva: 0x0053_90D8, target: 0x0018_C660 },
    PtrSite { name: "CEnemyBouncywild +0x130 state 3 begin", rva: 0x0053_90E8, target: 0x0018_C5E0 },
    PtrSite { name: "CEnemyBouncywild +0x138 state 3 step", rva: 0x0053_90F0, target: 0x0018_C870 },
    PtrSite { name: "CEnemyBouncywild +0x1F0 state 11 begin", rva: 0x0053_91A8, target: 0x0017_C530 },
    PtrSite { name: "CEnemyBouncywild +0x208 state 12 begin", rva: 0x0053_91C0, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyBouncywild +0x3B8 state 30 begin", rva: 0x0053_9370, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyBouncywild +0x3C0 state 30 step", rva: 0x0053_9378, target: 0x0018_3260 },
    PtrSite { name: "CEnemyBouncywild +0x3D0 think", rva: 0x0053_9388, target: 0x0019_6EC0 },
    // The shared enemy-guard step: the hit handler's guard event (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyBouncywild +0x3D8 guard event", rva: 0x0053_9390, target: 0x0018_1600 },
    PtrSite { name: "CEnemyBouncywild +0x438 attack hurt off", rva: 0x0053_93F0, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyBouncywild +0x440 attack end", rva: 0x0053_93F8, target: 0x0017_DA90 },
];

/// M11 casters: the Bouncywild's own homing banana's effect set (`0x14018C870`'s descriptor `+0xA8` = `0x1407C3010`: slots
/// 1 / 3 `"_ex_2040"` 5 / 10, the model's own effects), copied into its KH1 banana and peels (owner 2026-11-02: "drawn
/// with a Re:CoM effect (its own)"). Pointer-checked: slot 1's name pointer. Checked (exe + decomp).
pub const BOUNCYWILD_FIRE_PTRS: &[PtrSite] = &[PtrSite { name: "the Bouncywild's banana effect set slot 1 name", rva: 0x007C_3028, target: 0x0053_9448 }];

/// M11 casters: the reaction kind Re:CoM's own banana carries (`0x14018C870`: its descriptor's `+0x68` = 0x18), which the
/// player damage check `0x1401E6DE0` takes as Sora's slip (row 0x29 `p001_049`, KH1's slip clip baked; `0x1401F6190`
/// scatters a tenth of his Moogle points): the KH1 peels' hits carry it. Checked only (decomp + exe), never patched.
pub const BOUNCYWILD_CHECKED: &[Site] = &[
    // mov dword [rsp+0x98],0x18
    Site { name: "the Bouncywild banana's slip kind (0x14018C924)", rva: 0x0018_C924, expect: &[0xC7, 0x84, 0x24, 0x98, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00] },
];

/// M12 step 3, Second Chance (KH1's `0x1402BFA80` rule, `crate::kh1_support::second_chance`): the
/// player damage check's HP store at `0x1401E7238` (`test ebx,ebx; mov r12d,r15d; cmovns r12d,ebx;
/// mov [rdi+0x42C],r12d`), moved into our stub and replaced by a jump to it. Bytes read from the
/// Steam exe; no jump in the function lands inside these 16 bytes (only on their first, from Re:CoM's
/// card-0x6C rule above them; Checked (disasm of `0x1401E6DE0..0x1401E7B8C`)).
pub const SECOND_CHANCE_PATCH: &[Site] = &[Site { name: "damage check: HP store (0x1401E7238)", rva: 0x001E_7238, expect: &[0x85, 0xDB, 0x45, 0x8B, 0xE7, 0x44, 0x0F, 0x49, 0xE3, 0x44, 0x89, 0xA7, 0x2C, 0x04, 0x00, 0x00] }];

/// Its context, checked only: from `mov r13d,[rbp-0x51]` (the reaction kind), `mov r8d,[rdi+0x42C]`
/// (the HP before), `mov ebx,r8d` and the damage (`ebx` −= it; kind 0x22 = all HP, kind 8 = none)
/// through Re:CoM's card 0x7E (no loss) and card 0x6C (HP to 1) rules to the store; and the call
/// right after the store (`0x1401AAB00`, the stub's jump back). So `ebx` = the new HP, `r8d` = the HP
/// before (`r8d` is reused only on the card-0x6C path, where `ebx` is already 1), `r13d` = the kind.
pub const SECOND_CHANCE_CHECKED: &[Site] = &[
    Site { name: "damage check: HP before to the card rules (0x1401E71A2)", rva: 0x001E_71A2, expect: &[0x44, 0x8B, 0x6D, 0xAF, 0xB8, 0x1F, 0x85, 0xEB, 0x51, 0x44, 0x8B, 0x87, 0x2C, 0x04, 0x00, 0x00, 0x41, 0x8B, 0xD8, 0x45, 0x0F, 0xAF, 0xF4, 0x41, 0xF7, 0xEE, 0xC1, 0xFA, 0x05, 0x8B, 0xC2, 0xC1, 0xE8, 0x1F, 0x03, 0xD0, 0x41, 0x8B, 0xC0, 0x41, 0x83, 0xFD, 0x22, 0x0F, 0x45, 0xC2, 0x89, 0x45, 0xAF, 0x41, 0x83, 0xFD, 0x08, 0x74, 0x21, 0x2B, 0xD8, 0x48, 0x8B, 0x87, 0xE8, 0x03, 0x00, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x13, 0x0F, 0xB6, 0x88, 0xE8, 0x01, 0x00, 0x00, 0x80, 0xF9, 0x43, 0x76, 0x07, 0x80, 0xF9, 0x7E, 0x41, 0x0F, 0x44, 0xD8, 0x48, 0x8B, 0x87, 0xE8, 0x03, 0x00, 0x00, 0x41, 0xBE, 0x29, 0x00, 0x00, 0x00, 0x48, 0x85, 0xC0, 0x74, 0x2C, 0x83, 0xFB, 0x01, 0x7D, 0x27, 0x41, 0x83, 0xF8, 0x01, 0x7E, 0x21, 0x0F, 0xB6, 0x80, 0xE8, 0x01, 0x00, 0x00, 0x3C, 0x43, 0x76, 0x16, 0x3C, 0x6C, 0x75, 0x12, 0xBB, 0x01, 0x00, 0x00, 0x00, 0x45, 0x8B, 0xC6, 0x8B, 0xCB, 0x8D, 0x53, 0x27, 0xE8, 0xD8, 0x6D, 0xF7, 0xFF] },
    Site { name: "damage check: call after the HP store (0x1401E7248)", rva: 0x001E_7248, expect: &[0xE8, 0xB3, 0x38, 0xFC, 0xFF] },
];

/// Sora's damage cap (owner, 2026-11-05; `crate::damage_cap`, `win::damage_cap`): Re:CoM's enemy damage
/// apply `0x14017DC50`'s join `0x14017DF6A` (`movaps xmm2,xmm9; movss [rbx+0x2DC],xmm6; movaps xmm1,xmm6;
/// mov rcx,rbx; call 0x14017DAB0`), moved into our stub and replaced by a jump to it. Eleven branches of
/// the function land on its first byte, none inside (Checked (exe: disasm of `0x14017DC50..0x14017DFC0`,
/// rel32 scan of `.text`; the jump table `0x14017DFC0` has no entry inside)).
pub const DAMAGE_CAP_PATCH: &[Site] = &[Site { name: "enemy damage apply: the join (0x14017DF6A)", rva: 0x0017_DF6A, expect: &[0x41, 0x0F, 0x28, 0xD1, 0xF3, 0x0F, 0x11, 0xB3, 0xDC, 0x02, 0x00, 0x00, 0x0F, 0x28, 0xCE, 0x48, 0x8B, 0xCB, 0xE8, 0x2F, 0xFB, 0xFF, 0xFF] }];

/// Its context, checked only: the call's target `0x14017DAB0` (its only callers the join and
/// `0x140184DCB`) and its `(int)(value × resistance × [+0x290] / 100)`; the hit handler's frozen bonus
/// (`call 0x14015D940`) before its `call 0x14017DC50` (so the bonus is in the join's value); the flatten
/// exe `0x140184D20` applying `+0x2DC` again; the stop exe `0x1401843A0` reading the held hit's gathered
/// value `+0xF8 → +0x34` (not the join's), which the wrapper caps while the enemy is stopped.
pub const DAMAGE_CAP_CHECKED: &[Site] = &[
    Site { name: "enemy damage apply: HP store (0x14017DAB0)", rva: 0x0017_DAB0, expect: &[0x48, 0x89, 0x74, 0x24, 0x20, 0x57, 0x48, 0x83, 0xEC, 0x20] },
    Site { name: "enemy damage apply: value x resistance x falloff (0x14017DAED)", rva: 0x0017_DAED, expect: &[0xF2, 0x0F, 0x2C, 0xF3, 0x48, 0x0F, 0xBA, 0xE0, 0x0A] },
    Site { name: "enemy hit handler: frozen bonus, then the apply (0x14017F917)", rva: 0x0017_F917, expect: &[0xE8, 0x24, 0xE0, 0xFD, 0xFF, 0x66, 0x0F, 0x6E, 0x4C, 0x24, 0x40, 0x0F, 0x5B, 0xC9, 0x45, 0x8B, 0x8E, 0x80, 0x00, 0x00, 0x00, 0x0F, 0x28, 0xD6, 0x48, 0x8B, 0xCE, 0xE8, 0x19, 0xE3, 0xFF, 0xFF] },
    Site { name: "enemy flatten exe: +0x2DC applied again (0x140184DC0)", rva: 0x0018_4DC0, expect: &[0xF3, 0x0F, 0x10, 0x8B, 0xDC, 0x02, 0x00, 0x00, 0x48, 0x8B, 0xCB, 0xE8, 0xE0, 0x8C, 0xFF, 0xFF] },
    Site { name: "enemy stop exe: the held hit's gathered value (0x1401844BD)", rva: 0x0018_44BD, expect: &[0x66, 0x0F, 0x6E, 0x4B, 0x34, 0x48, 0x0F, 0xBF, 0xC5] },
];

/// M11, the KH1 Bouncywild's slip without its Moogle points (owner 2026-11-03, "a slip loses nothing"): the player damage
/// check `0x1401E6DE0`'s case 0x18 (Re:CoM's slip; case 0x27 falls into it) runs `mov rcx,rdi` and its writes (`+0x414` = 6,
/// `+0x5CC` = 0, `+0x7B8` = 0), then `call 0x1401F6190` (a tenth of Sora's Moogle points scattered as prizes), then `jmp
/// 0x1401E7901` (the reaction's tail). The call is retargeted once at install, through a stub near it, to
/// `win::kh1_peel::slip_scatter`, which skips it only while the damage-check wrapper takes a slip from one of our KH1
/// Bouncywild's peels or bananas; every other slip runs Re:CoM's scatter. Case 0x28's own call (`0x1401E78C3`) is
/// untouched. Bytes read from the Steam exe; no branch or pointer lands inside the call (exe scan: rel32 / rel8 / jcc,
/// 8-byte pointers, 4-byte RVAs; 2026-11-03).
pub const SLIP_SCATTER_CALLS: &[Site] = &[Site { name: "player damage check slip scatter call (0x1401E7665)", rva: 0x001E_7665, expect: &[0xE8, 0x26, 0xEB, 0x00, 0x00] }];

/// The call's context (the case's writes before it, its jump after it) and its callee, called by the stub.
pub const SLIP_SCATTER_CHECKED: &[Site] = &[
    // mov rcx,rdi; mov dword [rdi+0x414],6; mov [rdi+0x5CC],r15d; mov ebx,r15d; mov [rdi+0x7B8],r15d
    Site {
        name: "slip case writes before the scatter call (0x1401E7647)",
        rva: 0x001E_7647,
        expect: &[0x48, 0x8B, 0xCF, 0xC7, 0x87, 0x14, 0x04, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x44, 0x89, 0xBF, 0xCC, 0x05, 0x00, 0x00, 0x41, 0x8B, 0xDF, 0x44, 0x89, 0xBF, 0xB8, 0x07, 0x00, 0x00],
    },
    Site { name: "slip case jump after the scatter call (0x1401E766A)", rva: 0x001E_766A, expect: &[0xE9, 0x92, 0x02, 0x00, 0x00] },
    // mov [rsp+0x18],rbx; push rbp; push rdi; push r14; sub rsp,0xB0
    Site { name: "Moogle point scatter (0x1401F6190)", rva: 0x001F_6190, expect: &[0x48, 0x89, 0x5C, 0x24, 0x18, 0x55, 0x57, 0x41, 0x56, 0x48, 0x81, 0xEC, 0xB0, 0x00, 0x00, 0x00] },
];

/// M11 casters: the Red Nocturne's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp): its only
/// caller is the enemy factory `0x140188F40` (case 3, the call at `0x14018901A`); no branch lands inside the moved bytes
/// and no pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, 8-byte pointers; 2026-11-03).
pub const REDNOCTURNE_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyRednocturne ctor (0x140191880)", rva: 0x0019_1880, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 casters: the factory's `mov ecx, 0xCD0` before `CEnemyRednocturne`'s ctor, raised by the framework's 0x20-byte
/// tail. Checked (exe).
pub const REDNOCTURNE_ALLOC: &[Site] = &[Site { name: "alloc CEnemyRednocturne (0x140189001)", rva: 0x0018_9001, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 casters: `CEnemyRednocturne`'s vtable slots the copy keeps or replaces, checked against Re:CoM's functions before
/// the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 / 30 are `CBattleEnemy`'s, 1..3 the
/// Red Nocturne's own: 1 `0x1401919F0` / the walk thunk `0x140191D80` / `0x140191BF0`, 2 its small shot `e003_051`
/// `0x140191950` / `0x140191C00`, 3 its big shot `e003_053` `0x1401919A0` / `0x140191CC0` (a `CBullet` from its aim point,
/// homing 2.5° a frame for 1500 ticks); the think `CHeartless`'s `0x140196EC0`).
pub const REDNOCTURNE_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyRednocturne +0x70 motion play", rva: 0x0054_8AC8, target: 0x0017_D750 },
    PtrSite { name: "CEnemyRednocturne +0x80 update", rva: 0x0054_8AD8, target: 0x0018_5320 },
    PtrSite { name: "CEnemyRednocturne +0xE8 state 0 begin", rva: 0x0054_8B40, target: 0x0017_B590 },
    PtrSite { name: "CEnemyRednocturne +0xF0 state 0 step", rva: 0x0054_8B48, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyRednocturne +0xF8 state 0 end", rva: 0x0054_8B50, target: 0x0017_E630 },
    PtrSite { name: "CEnemyRednocturne +0x100 state 1 begin", rva: 0x0054_8B58, target: 0x0019_19F0 },
    PtrSite { name: "CEnemyRednocturne +0x108 state 1 step", rva: 0x0054_8B60, target: 0x0019_1D80 },
    PtrSite { name: "CEnemyRednocturne +0x110 state 1 end", rva: 0x0054_8B68, target: 0x0019_1BF0 },
    PtrSite { name: "CEnemyRednocturne +0x118 state 2 begin", rva: 0x0054_8B70, target: 0x0019_1950 },
    PtrSite { name: "CEnemyRednocturne +0x120 state 2 step", rva: 0x0054_8B78, target: 0x0019_1C00 },
    PtrSite { name: "CEnemyRednocturne +0x130 state 3 begin", rva: 0x0054_8B88, target: 0x0019_19A0 },
    PtrSite { name: "CEnemyRednocturne +0x138 state 3 step", rva: 0x0054_8B90, target: 0x0019_1CC0 },
    PtrSite { name: "CEnemyRednocturne +0x1F0 state 11 begin", rva: 0x0054_8C48, target: 0x0017_C530 },
    PtrSite { name: "CEnemyRednocturne +0x208 state 12 begin", rva: 0x0054_8C60, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyRednocturne +0x3B8 state 30 begin", rva: 0x0054_8E10, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyRednocturne +0x3C0 state 30 step", rva: 0x0054_8E18, target: 0x0018_3260 },
    PtrSite { name: "CEnemyRednocturne +0x3D0 think", rva: 0x0054_8E28, target: 0x0019_6EC0 },
    // The shared enemy-guard step's slot (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyRednocturne +0x3D8 guard event", rva: 0x0054_8E30, target: 0x0018_1600 },
    PtrSite { name: "CEnemyRednocturne +0x438 attack hurt off", rva: 0x0054_8E90, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyRednocturne +0x440 attack end", rva: 0x0054_8E98, target: 0x0017_DA90 },
];

/// M11 casters: the Red Nocturne's own shots' effect sets (`0x140191A30`'s descriptor `+0xA8`: state 2's small shot
/// `0x140548EF0`, slots 1..3 `"_ex_2110"` 8 / 9 / 7; state 3's big shot `0x140548F60`, 12 / 13 / 11: the model's own
/// effects), copied into its KH1 Fire and big Fire (owner 2026-11-02: projectiles drawn with a Re:CoM effect, its own).
/// Pointer-checked: each set's slot 1 name pointer. Checked (exe + decomp).
pub const REDNOCTURNE_FIRE_PTRS: &[PtrSite] = &[
    PtrSite { name: "the Red Nocturne's small shot effect set slot 1 name", rva: 0x0054_8F08, target: 0x0054_8F50 },
    PtrSite { name: "the Red Nocturne's big shot effect set slot 1 name", rva: 0x0054_8F78, target: 0x0054_8F50 },
];

/// M11 casters: the Blue Rhapsody's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp): its only
/// caller is the enemy factory `0x140188F40` (case 4, the call at `0x140189044`); no branch lands inside the moved bytes
/// and no pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, 8-byte pointers; 2026-11-03).
pub const BLUERHAPSODY_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyBluerhapsody ctor (0x14018C120)", rva: 0x0018_C120, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 casters: the factory's `mov ecx, 0xCD0` before `CEnemyBluerhapsody`'s ctor, raised by the framework's 0x20-byte
/// tail. Checked (exe).
pub const BLUERHAPSODY_ALLOC: &[Site] = &[Site { name: "alloc CEnemyBluerhapsody (0x14018902B)", rva: 0x0018_902B, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 casters: `CEnemyBluerhapsody`'s vtable slots the copy keeps or replaces, checked against Re:CoM's functions before
/// the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 / 30 are `CBattleEnemy`'s, 1..3 the
/// Blue Rhapsody's own: 1 `0x14018C280` / its bounce step `0x14018C460` / `0x14018C2C0`, 2 its Blizzard shot `e004_050`
/// `0x14018C200` / `0x14018C2D0` (a `CBullet` from its aim point past clip time 0x1099), 3 its body attack `e004_051`
/// `0x14018C250` / `0x14018C430`; the think `CHeartless`'s `0x140196EC0`).
pub const BLUERHAPSODY_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyBluerhapsody +0x70 motion play", rva: 0x0053_7D18, target: 0x0017_D750 },
    PtrSite { name: "CEnemyBluerhapsody +0x80 update", rva: 0x0053_7D28, target: 0x0018_5320 },
    PtrSite { name: "CEnemyBluerhapsody +0xE8 state 0 begin", rva: 0x0053_7D90, target: 0x0017_B590 },
    PtrSite { name: "CEnemyBluerhapsody +0xF0 state 0 step", rva: 0x0053_7D98, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyBluerhapsody +0xF8 state 0 end", rva: 0x0053_7DA0, target: 0x0017_E630 },
    PtrSite { name: "CEnemyBluerhapsody +0x100 state 1 begin", rva: 0x0053_7DA8, target: 0x0018_C280 },
    PtrSite { name: "CEnemyBluerhapsody +0x108 state 1 step", rva: 0x0053_7DB0, target: 0x0018_C460 },
    PtrSite { name: "CEnemyBluerhapsody +0x110 state 1 end", rva: 0x0053_7DB8, target: 0x0018_C2C0 },
    PtrSite { name: "CEnemyBluerhapsody +0x118 state 2 begin", rva: 0x0053_7DC0, target: 0x0018_C200 },
    PtrSite { name: "CEnemyBluerhapsody +0x120 state 2 step", rva: 0x0053_7DC8, target: 0x0018_C2D0 },
    PtrSite { name: "CEnemyBluerhapsody +0x130 state 3 begin", rva: 0x0053_7DD8, target: 0x0018_C250 },
    PtrSite { name: "CEnemyBluerhapsody +0x138 state 3 step", rva: 0x0053_7DE0, target: 0x0018_C430 },
    PtrSite { name: "CEnemyBluerhapsody +0x1F0 state 11 begin", rva: 0x0053_7E98, target: 0x0017_C530 },
    PtrSite { name: "CEnemyBluerhapsody +0x208 state 12 begin", rva: 0x0053_7EB0, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyBluerhapsody +0x3B8 state 30 begin", rva: 0x0053_8060, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyBluerhapsody +0x3C0 state 30 step", rva: 0x0053_8068, target: 0x0018_3260 },
    PtrSite { name: "CEnemyBluerhapsody +0x3D0 think", rva: 0x0053_8078, target: 0x0019_6EC0 },
    // The shared enemy-guard step's slot (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyBluerhapsody +0x3D8 guard event", rva: 0x0053_8080, target: 0x0018_1600 },
    PtrSite { name: "CEnemyBluerhapsody +0x438 attack hurt off", rva: 0x0053_80E0, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyBluerhapsody +0x440 attack end", rva: 0x0053_80E8, target: 0x0017_DA90 },
];

/// M11 casters: the Blue Rhapsody's own Blizzard shot's effect set (`0x14018C2D0`'s descriptor `+0xA8` = `0x1407C2F10`:
/// slots 1..3 `"_ex_2120"` 9 / 10 / 8, the model's own effects), copied into its KH1 Blizzard (owner 2026-11-02:
/// projectiles drawn with a Re:CoM effect, its own). Pointer-checked: slot 1's name pointer. Checked (exe + decomp).
/// Oogie Boogie at Combat = KH1 (owner, 2026-11-03: the gate lowers after his dice throw ends once
/// unbroken; `crate::battle::oogie`, `win::battle::attack_end` / `oogie_broken_begin`). Re:CoM code the
/// rule relies on (not patched; his state-13 begin is called through his slot). Checked (exe + disasm).
pub const OOGIE_CHECKED: &[Site] = &[
    // push rbx; sub rsp,0x20; mov rbx,rcx; call 0x14017BAA0 (the stagger); test byte [rbx+0xCD8],1;
    // je: his state-13 begin, the gate work only while state 2's mark is set.
    Site { name: "Oogie state 13 begin (0x1403D7A40)", rva: 0x003D_7A40, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0xE8, 0x52, 0x40, 0xDA, 0xFF, 0xF6, 0x83, 0xD8, 0x0C, 0x00, 0x00, 0x01, 0x0F, 0x84, 0xDB, 0x00, 0x00, 0x00] },
    // mov rcx,[rbx+0xD10]; (restores); mov eax,[rcx+0x2E0]; inc eax; mov [rcx+0x2E0],eax; cmp eax,3;
    // jle; mov dword [rcx+0x2E0],3; jmp; jl (below 3: done): the gate's count + 1, at most 3.
    Site {
        name: "Oogie state 13 begin: gate count + 1 (0x1403D7ACD)",
        rva: 0x003D_7ACD,
        expect: &[
            0x48, 0x8B, 0x8B, 0x10, 0x0D, 0x00, 0x00, 0x48, 0x8B, 0x7C, 0x24, 0x40, 0x48, 0x8B, 0x74, 0x24, 0x38, 0x48, 0x8B, 0x6C, 0x24, 0x30, 0x8B, 0x81, 0xE0, 0x02, 0x00, 0x00, 0xFF, 0xC0, 0x89, 0x81,
            0xE0, 0x02, 0x00, 0x00, 0x83, 0xF8, 0x03, 0x7E, 0x0C, 0xC7, 0x81, 0xE0, 0x02, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0xEB, 0x02, 0x7C, 0x32,
        ],
    },
    // mov rdx,[HDownTime]; mov rcx,rbx; call 0x14017F4E0; movss [rbx+0xCF8],xmm0: the down time at 3.
    Site { name: "Oogie state 13 begin: down time (0x1403D7B04)", rva: 0x003D_7B04, expect: &[0x48, 0x8B, 0x15, 0x0D, 0xB2, 0x46, 0x00, 0x48, 0x8B, 0xCB, 0xE8, 0xCD, 0x79, 0xDA, 0xFF, 0xF3, 0x0F, 0x11, 0x83, 0xF8, 0x0C, 0x00, 0x00] },
    // push rbx; sub rsp,0x40; or dword [rcx+0xCD8],1: his state 2 begin sets the mark.
    Site { name: "Oogie state 2 begin (0x1403D7830)", rva: 0x003D_7830, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40, 0x83, 0x89, 0xD8, 0x0C, 0x00, 0x00, 0x01] },
    // and dword [rcx+0xCD8],~1: his state 1 begin clears it.
    Site { name: "Oogie state 1 begin (0x1403D7EE0)", rva: 0x003D_7EE0, expect: &[0x83, 0xA1, 0xD8, 0x0C, 0x00, 0x00, 0xFE] },
    // xor edx,edx; mov r8d,0x1770; mov rcx,rbx; call 0x14017E540: his state 2 step's only attack end.
    Site { name: "Oogie state 2 attack end (0x1403D93F7)", rva: 0x003D_93F7, expect: &[0x33, 0xD2, 0x41, 0xB8, 0x70, 0x17, 0x00, 0x00, 0x48, 0x8B, 0xCB, 0xE8, 0x39, 0x51, 0xDA, 0xFF] },
    // mov [rsp+8],rbx; mov [rsp+0x10],rsi; push rdi; sub rsp,0x20: the enemy message
    // handler we call with message 3 (its case 3: `GIMMICK_BREAK_CHECKED`).
    Site { name: "enemy message handler (0x140181620)", rva: 0x0018_1620, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20] },
];

/// `CBossBoogie`'s slots the rule relies on: the message handler (Re:CoM's base), his state 1 begin,
/// state 2 begin and step, state 13 begin (written with ours, [`OOGIE_BROKEN_SLOT`]). Checked (exe;
/// relocated pointers).
pub const OOGIE_PTRS: &[PtrSite] = &[
    PtrSite { name: "CBossBoogie +0x48", rva: 0x0067_0AA8, target: 0x0018_1620 },
    PtrSite { name: "CBossBoogie +0x100", rva: 0x0067_0B60, target: 0x003D_7EE0 },
    PtrSite { name: "CBossBoogie +0x118", rva: 0x0067_0B78, target: 0x003D_7830 },
    PtrSite { name: "CBossBoogie +0x120", rva: 0x0067_0B80, target: 0x003D_9340 },
    PtrSite { name: "CBossBoogie +0x220", rva: 0x0067_0C80, target: 0x003D_7A40 },
];

/// His state-13 begin slot (also in [`OOGIE_PTRS`], checked first).
pub const OOGIE_BROKEN_SLOT: PtrSite = PtrSite { name: "CBossBoogie +0x220 (state 13 begin)", rva: 0x0067_0C80, target: 0x003D_7A40 };

pub const BLUERHAPSODY_FIRE_PTRS: &[PtrSite] = &[PtrSite { name: "the Blue Rhapsody's Blizzard shot effect set slot 1 name", rva: 0x007C_2F28, target: 0x0053_8138 }];

/// M11 casters: the Green Requiem's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp): its only
/// caller is the enemy factory `0x140188F40` (case 6, the call at `0x140189098`); no branch lands inside the moved bytes
/// and no pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, 8-byte pointers; 2026-11-03).
pub const GREENREQUIEM_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyGreenrequiem ctor (0x14018E570)", rva: 0x0018_E570, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 casters: the factory's `mov ecx, 0xCE0` before `CEnemyGreenrequiem`'s ctor, raised by the framework's 0x20-byte
/// tail. Checked (exe).
pub const GREENREQUIEM_ALLOC: &[Site] = &[Site { name: "alloc CEnemyGreenrequiem (0x14018907F)", rva: 0x0018_907F, expect: &[0xB9, 0xE0, 0x0C, 0x00, 0x00] }];

/// M11 casters: `CEnemyGreenrequiem`'s vtable slots the copy keeps or replaces, checked against Re:CoM's functions before
/// the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 3 / 11 / 12 / 30 are `CBattleEnemy`'s, 1 and 2
/// the Green Requiem's own: 1 `0x14018E780` / `0x14018EC60` / `0x14018E7C0` (the family's flying walk with `e003_001` /
/// `e003_002`), 2 its Cure `e006_051` `0x14018E640` / `0x14018E9B0` (the ally with the most HP missing, or itself; past
/// clip time 6000 one heal by `0x140181CE0`); the think `CHeartless`'s `0x140196EC0`).
pub const GREENREQUIEM_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyGreenrequiem +0x70 motion play", rva: 0x0054_0538, target: 0x0017_D750 },
    PtrSite { name: "CEnemyGreenrequiem +0x80 update", rva: 0x0054_0548, target: 0x0018_5320 },
    PtrSite { name: "CEnemyGreenrequiem +0xE8 state 0 begin", rva: 0x0054_05B0, target: 0x0017_B590 },
    PtrSite { name: "CEnemyGreenrequiem +0xF0 state 0 step", rva: 0x0054_05B8, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyGreenrequiem +0xF8 state 0 end", rva: 0x0054_05C0, target: 0x0017_E630 },
    PtrSite { name: "CEnemyGreenrequiem +0x100 state 1 begin", rva: 0x0054_05C8, target: 0x0018_E780 },
    PtrSite { name: "CEnemyGreenrequiem +0x108 state 1 step", rva: 0x0054_05D0, target: 0x0018_EC60 },
    PtrSite { name: "CEnemyGreenrequiem +0x110 state 1 end", rva: 0x0054_05D8, target: 0x0018_E7C0 },
    PtrSite { name: "CEnemyGreenrequiem +0x118 state 2 begin", rva: 0x0054_05E0, target: 0x0018_E640 },
    PtrSite { name: "CEnemyGreenrequiem +0x120 state 2 step", rva: 0x0054_05E8, target: 0x0018_E9B0 },
    PtrSite { name: "CEnemyGreenrequiem +0x130 state 3 begin", rva: 0x0054_05F8, target: 0x0017_B750 },
    PtrSite { name: "CEnemyGreenrequiem +0x138 state 3 step", rva: 0x0054_0600, target: 0x0018_2B70 },
    PtrSite { name: "CEnemyGreenrequiem +0x1F0 state 11 begin", rva: 0x0054_06B8, target: 0x0017_C530 },
    PtrSite { name: "CEnemyGreenrequiem +0x208 state 12 begin", rva: 0x0054_06D0, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyGreenrequiem +0x3B8 state 30 begin", rva: 0x0054_0880, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyGreenrequiem +0x3C0 state 30 step", rva: 0x0054_0888, target: 0x0018_3260 },
    PtrSite { name: "CEnemyGreenrequiem +0x3D0 think", rva: 0x0054_0898, target: 0x0019_6EC0 },
    // The shared enemy-guard step's slot (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyGreenrequiem +0x3D8 guard event", rva: 0x0054_08A0, target: 0x0018_1600 },
    PtrSite { name: "CEnemyGreenrequiem +0x438 attack hurt off", rva: 0x0054_0900, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyGreenrequiem +0x440 attack end", rva: 0x0054_0908, target: 0x0017_DA90 },
];

/// M11 casters: the Green Requiem's Cure on Re:CoM's own heal (`win::kh1_cure`), made as its state 2 step `0x14018E9B0`
/// makes it: its power `0x14017F0E0(enemy, 0, 0x19, 0, 0, 0)` and the heal `0x140181CE0(target, power, "_ex_2140", effect,
/// &(0, 0, 0, 1), 0x1402ABB20(target, 1))` with the effect 8 / 9 / 10 for a target of Re:CoM's enemy id 3 / 4 / 5 (`+0x128
/// % 10000`), else 7. Check-only: the effect set's name the call passes (`lea r8` at `0x14018EAB8` → `0x140540958`), the
/// effect's switch and the power call's arguments (Steam exe, 2026-11-03). The callees are checked in their own lists
/// (`KH1_ENEMY_REWARD_CALLEES`, `THROWN_CALLEES`, …).
pub const GREENREQUIEM_CURE_CHECKED: &[Site] = &[
    Site { name: "the Green Requiem's heal effect set name \"_ex_2140\" (0x140540958)", rva: 0x0054_0958, expect: b"_ex_2140\0" },
    // mov ecx,[rax+0x128]; lea esi,[r14+7]; … % 10000; 3 → 8, 4 → 9, 5 → 10 (lea esi,[rcx+9]); else 7.
    Site {
        name: "the Green Requiem's heal effect by its target's class (0x14018EA34)",
        rva: 0x0018_EA34,
        expect: &[
            0x8B, 0x88, 0x28, 0x01, 0x00, 0x00, 0x41, 0x8D, 0x76, 0x07, 0xB8, 0xAD, 0x8B, 0xDB, 0x68, 0xF7, 0xE9, 0xC1, 0xFA, 0x0C, 0x8B, 0xC2, 0xC1, 0xE8, 0x1F, 0x03, 0xD0, 0x69, 0xC2, 0x10, 0x27, 0x00, 0x00, 0x2B, 0xC8, 0x83, 0xE9, 0x03, 0x74, 0x16, 0x83, 0xE9, 0x01,
            0x74, 0x0A, 0x83, 0xF9, 0x01, 0x75, 0x11, 0x8D, 0x71, 0x09, 0xEB, 0x0C, 0xBE, 0x09, 0x00, 0x00, 0x00, 0xEB, 0x05, 0xBE, 0x08, 0x00, 0x00, 0x00,
        ],
    },
    // xor r9d,r9d; mov [rsp+0x28],r14d; xor edx,edx; mov [rsp+0x20],r14; mov rcx,rdi; mov rbx,rax; lea r8d,[r9+0x19];
    // call 0x14017F0E0.
    Site {
        name: "the Green Requiem's heal power call (0x14018EA84)",
        rva: 0x0018_EA84,
        expect: &[0x45, 0x33, 0xC9, 0x44, 0x89, 0x74, 0x24, 0x28, 0x33, 0xD2, 0x4C, 0x89, 0x74, 0x24, 0x20, 0x48, 0x8B, 0xCF, 0x48, 0x8B, 0xD8, 0x45, 0x8D, 0x41, 0x19, 0xE8, 0x3E, 0x06, 0xFF, 0xFF],
    },
];

/// M11 casters: the Yellow Opera's ctor (the Soldier's shape). Same rule as [`HOOKS`]. Checked (exe + decomp): its only
/// caller is the enemy factory `0x140188F40` (case 5, the call at `0x14018906E`); no branch lands inside the moved bytes
/// and no pointer names them (exe scan: rel32 calls / jumps, jcc, rel8 jumps, 8-byte pointers; 2026-11-03).
pub const YELLOWOPERA_HOOKS: &[Site] = &[
    // rex push rbx; sub rsp,0x20 (then xor r9d,r9d; xor r8d,r8d; mov rbx,rcx; call 0x140196DF0)
    Site { name: "CEnemyYellowopera ctor (0x1401968B0)", rva: 0x0019_68B0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// M11 casters: the factory's `mov ecx, 0xCD0` before `CEnemyYellowopera`'s ctor, raised by the framework's 0x20-byte
/// tail. Checked (exe).
pub const YELLOWOPERA_ALLOC: &[Site] = &[Site { name: "alloc CEnemyYellowopera (0x140189055)", rva: 0x0018_9055, expect: &[0xB9, 0xD0, 0x0C, 0x00, 0x00] }];

/// M11 casters: `CEnemyYellowopera`'s vtable slots the copy keeps or replaces, checked against Re:CoM's functions before
/// the copy is made (Checked (exe), relocated pointers; decomp: states 0 / 11 / 12 / 30 are `CBattleEnemy`'s, 1..3 the
/// Yellow Opera's own: 1 `0x140196A20` (`0x1401979F0` with `e003_001` / `e003_002`) / its step `0x140196DE0` →
/// `0x140197450` / its end `0x140196A70` → `0x1401979C0`, 2 its Thunder `e005_052` `0x140196980` / `0x140196A80` (a
/// `CBullet` 4 m above Sora past clip time 0xD15), 3 its rush `e005_053` → `e005_054` → `e005_055` `0x1401969D0` /
/// `0x140196C10`; the think `CHeartless`'s `0x140196EC0`).
pub const YELLOWOPERA_PTRS: &[PtrSite] = &[
    PtrSite { name: "CEnemyYellowopera +0x70 motion play", rva: 0x0055_7138, target: 0x0017_D750 },
    PtrSite { name: "CEnemyYellowopera +0x80 update", rva: 0x0055_7148, target: 0x0018_5320 },
    PtrSite { name: "CEnemyYellowopera +0xE8 state 0 begin", rva: 0x0055_71B0, target: 0x0017_B590 },
    PtrSite { name: "CEnemyYellowopera +0xF0 state 0 step", rva: 0x0055_71B8, target: 0x0018_29E0 },
    PtrSite { name: "CEnemyYellowopera +0xF8 state 0 end", rva: 0x0055_71C0, target: 0x0017_E630 },
    PtrSite { name: "CEnemyYellowopera +0x100 state 1 begin", rva: 0x0055_71C8, target: 0x0019_6A20 },
    PtrSite { name: "CEnemyYellowopera +0x108 state 1 step", rva: 0x0055_71D0, target: 0x0019_6DE0 },
    PtrSite { name: "CEnemyYellowopera +0x110 state 1 end", rva: 0x0055_71D8, target: 0x0019_6A70 },
    PtrSite { name: "CEnemyYellowopera +0x118 state 2 begin", rva: 0x0055_71E0, target: 0x0019_6980 },
    PtrSite { name: "CEnemyYellowopera +0x120 state 2 step", rva: 0x0055_71E8, target: 0x0019_6A80 },
    PtrSite { name: "CEnemyYellowopera +0x130 state 3 begin", rva: 0x0055_71F8, target: 0x0019_69D0 },
    PtrSite { name: "CEnemyYellowopera +0x138 state 3 step", rva: 0x0055_7200, target: 0x0019_6C10 },
    PtrSite { name: "CEnemyYellowopera +0x1F0 state 11 begin", rva: 0x0055_72B8, target: 0x0017_C530 },
    PtrSite { name: "CEnemyYellowopera +0x208 state 12 begin", rva: 0x0055_72D0, target: 0x0017_B9D0 },
    PtrSite { name: "CEnemyYellowopera +0x3B8 state 30 begin", rva: 0x0055_7480, target: 0x0017_BBC0 },
    PtrSite { name: "CEnemyYellowopera +0x3C0 state 30 step", rva: 0x0055_7488, target: 0x0018_3260 },
    PtrSite { name: "CEnemyYellowopera +0x3D0 think", rva: 0x0055_7498, target: 0x0019_6EC0 },
    // The shared enemy-guard step's slot (`CBattleEnemy`'s no-op `ret 0`).
    PtrSite { name: "CEnemyYellowopera +0x3D8 guard event", rva: 0x0055_74A0, target: 0x0018_1600 },
    PtrSite { name: "CEnemyYellowopera +0x438 attack hurt off", rva: 0x0055_7500, target: 0x0017_DA80 },
    PtrSite { name: "CEnemyYellowopera +0x440 attack end", rva: 0x0055_7508, target: 0x0017_DA90 },
];

/// M11 casters: the Yellow Opera's own Thunder's effect set (`0x140196A80`'s descriptor `+0xA8` = `0x140557560`: slots 0 / 1
/// `"_ex_2130"` 11 / 12, the model's own effects, KH1's bolt and hit effect numbers), copied into its KH1 Thunder (owner
/// 2026-11-02: strikes drawn with their own Re:CoM effect). Pointer-checked: the two slots' name pointers. Checked (exe +
/// decomp).
pub const YELLOWOPERA_THUNDER_PTRS: &[PtrSite] = &[
    PtrSite { name: "the Yellow Opera's Thunder effect set slot 0 name", rva: 0x0055_7560, target: 0x0055_75C0 },
    PtrSite { name: "the Yellow Opera's Thunder effect set slot 1 name", rva: 0x0055_7578, target: 0x0055_75C0 },
];

/// 0.113.0, true squeeze for Re:CoM's text (`crate::text_fit`, `win::text_fit`,
/// `docs/recom-m12-sleights.md` § 8.8): what the vtable copies and the packet squeeze rely on.
/// Checked only; nothing here is patched or hooked (our texts get a copy of the vtable instead).
pub const TEXT_FIT_CHECKED: &[Site] = &[
    // C2DFnt vtable +0x18: mov rax,[rcx+0x58]; test rax,rax; je ret; cmp byte [rax],0; jne 0x14025AA20; ret.
    Site { name: "C2DFnt draw thunk (0x140289450)", rva: 0x0028_9450, expect: &[0x48, 0x8B, 0x41, 0x58, 0x48, 0x85, 0xC0, 0x74, 0x09, 0x80, 0x38, 0x00, 0x0F, 0x85, 0xBE, 0x15, 0xFD, 0xFF, 0xC3] },
    // C2DFnt vtable +0x28: swaps rcx / rdx (this, out) and jumps to the width routine 0x14025A380.
    Site { name: "C2DFnt measure thunk (0x140289440)", rva: 0x0028_9440, expect: &[0x48, 0x8B, 0xC2, 0x48, 0x8B, 0xD1, 0x48, 0x8B, 0xC8, 0xE9, 0x32, 0x0F, 0xFD, 0xFF] },
    // The draw expands the button escapes first: mov rcx,[r15+0x58]; lea rdx,[rbp+0xD0] (its 2048-byte buffer); call 0x1403480C0.
    Site { name: "C2DFnt draw: text copy (0x14025AC2B)", rva: 0x0025_AC2B, expect: &[0x49, 0x8B, 0x4F, 0x58, 0x48, 0x8D, 0x95, 0xD0, 0x00, 0x00, 0x00, 0xE8, 0x85, 0xD4, 0x0E, 0x00] },
    // The text copy (src, dst): mov r11,rsp; push rbp; push rdi; push r14; sub rsp,0x70.
    Site { name: "text escape copy (0x1403480C0)", rva: 0x0034_80C0, expect: &[0x4C, 0x8B, 0xDC, 0x55, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x70] },
    // Re:CoM's text-width routine (out[2], fnt, lines, count): mov [rsp+0x20],r9; mov [rsp+0x18],r8; mov [rsp+8],rcx; push rbp.
    Site { name: "text width routine (0x14025A380)", rva: 0x0025_A380, expect: &[0x4C, 0x89, 0x4C, 0x24, 0x20, 0x4C, 0x89, 0x44, 0x24, 0x18, 0x48, 0x89, 0x4C, 0x24, 0x08, 0x55] },
    // The draw's set-up: size +0x3C, y +0x2C, spacing +0x44, x +0x28 (r12d), lines +0x80; no lines and
    // flags & 3: x −= width (right) or width / 2 (centre), from the width routine; lines: x += lines[0].
    Site { name: "C2DFnt draw: origin and alignment (0x14025AC3B)", rva: 0x0025_AC3B, expect: &[0x41, 0x8B, 0x47, 0x3C, 0x0F, 0xB7, 0x8B, 0x1C, 0x01, 0x00, 0x00, 0x45, 0x0F, 0xBF, 0x4F, 0x2C, 0x49, 0x8B, 0x5F, 0x44, 0x44, 0x2B, 0xC8, 0x45, 0x8B, 0x67, 0x28, 0x89, 0x44, 0x24, 0x60, 0x89, 0x44, 0x24, 0x6C, 0x89, 0x45, 0xE4, 0xC1, 0xF8, 0x04, 0x89, 0x45, 0xF4, 0x49, 0x8B, 0x87, 0x80, 0x00, 0x00, 0x00, 0x89, 0x4D, 0xE8, 0x49, 0x8B, 0x8F, 0x88, 0x00, 0x00, 0x00, 0x44, 0x89, 0x4C, 0x24, 0x58, 0x48, 0x89, 0x5D, 0x28, 0x48, 0x89, 0x45, 0x20, 0x48, 0x89, 0x4D, 0x30, 0x48, 0x85, 0xC0, 0x75, 0x36, 0x41, 0xF6, 0xC6, 0x03, 0x74, 0x2A, 0x45, 0x33, 0xC9, 0x48, 0x8D, 0x4D, 0xB0, 0x45, 0x33, 0xC0, 0x49, 0x8B, 0xD7, 0xE8, 0xDA, 0xF6, 0xFF, 0xFF, 0x41, 0xF6, 0xC6, 0x02, 0x74, 0x0E, 0x8B, 0x45, 0xB0, 0xD1, 0xF8, 0x44, 0x2B, 0xE0, 0x44, 0x89, 0x65, 0xB8, 0xEB, 0x11, 0x44, 0x2B, 0x65, 0xB0, 0x44, 0x89, 0x65, 0xB8, 0xEB, 0x07, 0x44, 0x89, 0x65, 0xB8, 0x44, 0x03, 0x20, 0x41, 0x8B, 0xC6, 0x48, 0xC7, 0x45, 0x40, 0x0E, 0x00, 0x00, 0x00] },
    // The draw opens its packet (call 0x140281180) and writes the header pairs (5, 9), ([0x14087CFB0], 0x17),
    // (0x356, 0) at the pointer it returns.
    Site { name: "C2DFnt draw: packet header (0x14025AD01)", rva: 0x0025_AD01, expect: &[0x48, 0xB8, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x10, 0x48, 0x89, 0x45, 0x38, 0xE8, 0x6C, 0x64, 0x02, 0x00, 0x48, 0xC7, 0x45, 0xC8, 0x05, 0x00, 0x00, 0x00, 0x4C, 0x8B, 0xF8, 0x48, 0xC7, 0x45, 0xD0, 0x09, 0x00, 0x00, 0x00, 0x33, 0xD2, 0x0F, 0x10, 0x45, 0xC8, 0x48, 0xC7, 0x45, 0xD0, 0x17, 0x00, 0x00, 0x00, 0x89, 0x55, 0x98, 0x0F, 0x11, 0x00, 0x48, 0x8B, 0x0D, 0x6E, 0x22, 0x62, 0x00, 0x48, 0x89, 0x4D, 0xC8, 0x0F, 0x10, 0x45, 0xC8, 0x48, 0xC7, 0x45, 0xC8, 0x56, 0x03, 0x00, 0x00, 0x48, 0x89, 0x55, 0xD0, 0x0F, 0x11, 0x40, 0x10] },
    // The draw's end: the closing pair (.., 9) and 0x140280F30 (the write pointer after it).
    Site { name: "C2DFnt draw: packet end (0x14025B719)", rva: 0x0025_B719, expect: &[0xE8, 0x62, 0x4A, 0x02, 0x00, 0x48, 0x89, 0x45, 0x10, 0x49, 0x8D, 0x4F, 0x10, 0x48, 0xC7, 0x45, 0x18, 0x09, 0x00, 0x00, 0x00, 0x0F, 0x10, 0x45, 0x10, 0x41, 0x0F, 0x11, 0x07, 0xE8, 0xF5, 0x57, 0x02, 0x00] },
    // Packet open: r8 = [0x140882DD8]; [r8+0x28] = [r8] (start); ...; [r8] += 0x10; returns it.
    Site { name: "packet open (0x140281180)", rva: 0x0028_1180, expect: &[0x4C, 0x8B, 0x05, 0x51, 0x1C, 0x60, 0x00, 0x49, 0x8B, 0x00, 0x49, 0x89, 0x40, 0x28, 0x48, 0x8B, 0x01, 0x49, 0x89, 0x40, 0x50, 0x48, 0x8B, 0x41, 0x08, 0x49, 0x89, 0x40, 0x58, 0x8B, 0xC2, 0xC1, 0xE8, 0x10, 0x66, 0x41, 0x89, 0x40, 0x52, 0x48, 0xB8, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0xC0, 0xFF, 0xFF, 0x49, 0x21, 0x40, 0x50, 0x8B, 0xC2, 0x25, 0xFF, 0x3F, 0x00, 0x00, 0x48, 0xC1, 0xE0, 0x20, 0x49, 0x09, 0x40, 0x50, 0x49, 0x8B, 0x00, 0x48, 0x83, 0xC0, 0x10, 0x49, 0x89, 0x00, 0xC3] },
    // Packet close: [[0x140882DD8]] = end; jmp 0x140281E40 (GIF tag, 16-byte alignment).
    Site { name: "packet close (0x140280F30)", rva: 0x0028_0F30, expect: &[0x48, 0x8B, 0xC1, 0x48, 0x8B, 0x0D, 0x9E, 0x1E, 0x60, 0x00, 0x48, 0x89, 0x01, 0xE9, 0xFE, 0x0E, 0x00, 0x00] },
    // The glyph quad's XYZ2 pairs: x (rbp) + 0x8000 & 0x1FFFF in bits 0..16, y (rsi) at 17; register 5 at
    // +0x20 and +0x40 of its 0x50 bytes.
    Site { name: "glyph quad: XYZ2 packing (0x14025A286)", rva: 0x0025_A286, expect: &[0x8D, 0x86, 0x00, 0x80, 0x00, 0x00, 0x25, 0xFF, 0xFF, 0x01, 0x00, 0x48, 0x0B, 0xC8, 0x8D, 0x85, 0x00, 0x80, 0x00, 0x00, 0x25, 0xFF, 0xFF, 0x01, 0x00, 0x48, 0xC1, 0xE1, 0x11, 0x48, 0x0B, 0xC8, 0x48, 0x8B, 0x02, 0x48, 0x89, 0x4C, 0x24, 0x08, 0x0F, 0x10, 0x44, 0x24, 0x08, 0x8B, 0x4C, 0x24, 0x30, 0xC1, 0xF9, 0x04, 0x0F, 0x11, 0x40, 0x20, 0x48, 0x8B, 0x04, 0x24, 0x48, 0x89, 0x44, 0x24, 0x08, 0x48, 0x8B, 0x02, 0x48, 0xC1, 0xE1, 0x11, 0x48, 0xC7, 0x44, 0x24, 0x10, 0x03, 0x00, 0x00, 0x00, 0x0F, 0x10, 0x44, 0x24, 0x08, 0x48, 0xC7, 0x44, 0x24, 0x10, 0x05, 0x00, 0x00, 0x00, 0x0F, 0x11, 0x40, 0x30, 0x8D, 0x86, 0x00, 0x80, 0x00, 0x00, 0x03, 0xC3, 0x25, 0xFF, 0xFF, 0x01, 0x00, 0x48, 0x0B, 0xC8, 0x8D, 0x85, 0x00, 0x80, 0x00, 0x00, 0x41, 0x03, 0xC0, 0x48, 0xC1, 0xE1, 0x11, 0x25, 0xFF, 0xFF, 0x01, 0x00, 0x48, 0x0B, 0xC8, 0x48, 0x8B, 0x02, 0x48, 0x89, 0x4C, 0x24, 0x08, 0x0F, 0x10, 0x44, 0x24, 0x08, 0x0F, 0x11, 0x40, 0x40, 0x48, 0x83, 0x02, 0x50] },
];

/// Re:CoM's C2DFnt vtable (`0x140606C98`, written only by its two ctors `0x140243F47` / `0x140243F81`;
/// no absolute pointer to it in the image: Checked (exe scan)): its COL and 7 slots, copied into our
/// squeezed texts' vtables (`crate::text_fit::vtable`). Checked lazily by `win::text_fit` (a mismatch
/// turns the squeeze off, logged).
pub const TEXT_FIT_PTRS: &[PtrSite] = &[
    PtrSite { name: "C2DFnt vtable −8: its COL (0x140606C90)", rva: 0x0060_6C90, target: 0x0071_8CF8 },
    PtrSite { name: "C2DFnt vtable +0: deleting dtor (0x140606C98)", rva: 0x0060_6C98, target: 0x0024_3FB0 },
    PtrSite { name: "C2DFnt vtable +8: register (0x140606CA0)", rva: 0x0060_6CA0, target: 0x0026_1D70 },
    PtrSite { name: "C2DFnt vtable +0x10: packet size (0x140606CA8)", rva: 0x0060_6CA8, target: 0x0028_9430 },
    PtrSite { name: "C2DFnt vtable +0x18: draw (0x140606CB0)", rva: 0x0060_6CB0, target: 0x0028_9450 },
    PtrSite { name: "C2DFnt vtable +0x20: depth (0x140606CB8)", rva: 0x0060_6CB8, target: 0x000D_3D00 },
    PtrSite { name: "C2DFnt vtable +0x28: measure (0x140606CC0)", rva: 0x0060_6CC0, target: 0x0028_9440 },
    PtrSite { name: "C2DFnt vtable +0x30 (0x140606CC8)", rva: 0x0060_6CC8, target: 0x0024_3FF0 },
];

/// The first-launch build's progress screen (`win::assets`, `docs/openkh-github-install.md` § 8):
/// `CTitle`'s state-1 update `0x1403A3820` (the title with its "press" wait). Its only reference is
/// the `lea` in `CTitle`'s ctor `0x14039F770` (`TITLE_HOLD_CHECKED`); no branch into the 6 moved
/// bytes (exe scan: the rel32 / rel8 hits inside them are inside other instructions). Checked (exe +
/// decomp).
pub const TITLE_HOLD_HOOK: &[Site] = &[
    // push rbx; sub rsp,0x20 (then mov eax,[rcx+0xAC]).
    Site { name: "CTitle state 1 update (0x1403A3820)", rva: 0x003A_3820, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// What the hold relies on. Checked only.
pub const TITLE_HOLD_CHECKED: &[Site] = &[
    // mov eax,[rcx+0xAC]; mov rbx,rcx; test eax,eax; je (sub 0: start the title); cmp eax,0xA; je
    // (sub 10: wait for the title's sequence or a press); cmp eax,0x14 (sub 20: the fade).
    Site {
        name: "CTitle state 1 update: its sub-states (0x1403A3826)",
        rva: 0x003A_3826,
        expect: &[0x8B, 0x81, 0xAC, 0x00, 0x00, 0x00, 0x48, 0x8B, 0xD9, 0x85, 0xC0, 0x0F, 0x84, 0xF1, 0x00, 0x00, 0x00, 0x83, 0xF8, 0x0A, 0x74, 0x47, 0x83, 0xF8, 0x14],
    },
    // CTitle's ctor stores the state-1 update in its state record: lea rax,[rip+0x3FC1] (0x1403A3820).
    Site { name: "CTitle ctor: state 1's update (0x14039F858)", rva: 0x0039_F858, expect: &[0x48, 0x8D, 0x05, 0xC1, 0x3F, 0x00, 0x00, 0x48, 0x89, 0x45, 0xD8] },
];

/// Lock-on on R1 at Combat = KH1 on a controller (`crate::lock_on`, `win::lock_on`,
/// `docs/recom-lock-on.md`): Re:CoM's lock-on `0x1401E8DA0` (its two callers are the player's Field
/// and Battle exes, `0x1401F994B` / `0x1401FA1A2`; no branch into the 6 moved bytes: exe scan).
pub const LOCK_ON_HOOK: &[Site] = &[
    // push rbx; sub rsp,0x20 (then mov rax,[rcx+0x3F8]).
    Site { name: "lock-on (0x1401E8DA0)", rva: 0x001E_8DA0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20] },
];

/// What the lock-on remap relies on. Checked only.
pub const LOCK_ON_CHECKED: &[Site] = &[
    // The rest of the lock-on's head: mov rax,[rcx+0x3F8]; mov rbx,rcx.
    Site { name: "lock-on: its head (0x1401E8DA6)", rva: 0x001E_8DA6, expect: &[0x48, 0x8B, 0x81, 0xF8, 0x03, 0x00, 0x00, 0x48, 0x8B, 0xD9] },
    // The pad object's update 0x1401FF150 writes its buttons each frame: +0x38 = 4 (L1), +0x3C = 8 (R1),
    // +0x40 = edi (1, L2), +0x44 = 2 (R2: the lock-on).
    Site { name: "player pad update: the action buttons (0x1401FF19D)", rva: 0x001F_F19D, expect: &[0xC7, 0x41, 0x38, 0x04, 0x00, 0x00, 0x00, 0xC7, 0x41, 0x3C, 0x08, 0x00, 0x00, 0x00, 0x89, 0x79, 0x40, 0xC7, 0x41, 0x44, 0x02, 0x00, 0x00, 0x00] },
    // The three readers of +0x44 (each mov ecx,[rcx+0x44]; call the pad held / released / pressed test),
    // all called only from the lock-on.
    Site { name: "lock button held (0x1401FF4F0)", rva: 0x001F_F4F0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x19, 0x33, 0xD2, 0x8B, 0x49, 0x44, 0x83, 0xE3, 0x01, 0xE8, 0xBB, 0xBD, 0x09, 0x00, 0x0F, 0xAF, 0xC3, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3] },
    Site { name: "lock button released (0x1401FF600)", rva: 0x001F_F600, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x19, 0x33, 0xD2, 0x8B, 0x49, 0x44, 0x83, 0xE3, 0x01, 0xE8, 0xBB, 0xBC, 0x09, 0x00, 0x83, 0xF3, 0x01, 0x0B, 0xC3, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3] },
    Site { name: "lock button pressed (0x1401FF740)", rva: 0x001F_F740, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x19, 0x33, 0xD2, 0x8B, 0x49, 0x44, 0x83, 0xE3, 0x01, 0xE8, 0x9B, 0xBB, 0x09, 0x00, 0x0F, 0xAF, 0xC3, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3] },
    // The device in use: any key or mouse input -> [0x140C2FF64] = 1 (then the key glyphs), any pad
    // input -> 0 (0x140347EF0).
    Site { name: "device in use (0x140347EF0)", rva: 0x0034_7EF0, expect: &[0x48, 0x83, 0xEC, 0x28, 0xE8, 0x27, 0xF5, 0xFF, 0xFF, 0x85, 0xC0, 0x74, 0x20, 0x83, 0x3D, 0x60, 0x80, 0x8E, 0x00, 0x00, 0x0F, 0x85, 0xE3, 0x00, 0x00, 0x00, 0xC7, 0x05, 0x50, 0x80, 0x8E, 0x00, 0x01, 0x00, 0x00, 0x00] },
    // Its getter: mov eax,[0x140C2FF64]; ret.
    Site { name: "keyboard / mouse in use getter (0x1403471C0)", rva: 0x0034_71C0, expect: &[0x8B, 0x05, 0x9E, 0x8D, 0x8E, 0x00, 0xC3] },
];

/// 0.137.0, KH1's lock rules (`crate::lock_on`, `win::lock_on::install_rules`): the lock-on's five
/// input calls, retargeted through a stub block near the exe to KH1's answers (Re:CoM's own callee
/// otherwise). Each `call rel32`, Checked (exe + disasm `0x1401E8DA0`).
pub const LOCK_ON_CALLS: &[Site] = &[
    // Unlocked: call 0x1401FF740 (the lock button pressed).
    Site { name: "lock-on: lock pressed call (0x1401E8E44)", rva: 0x001E_8E44, expect: &[0xE8, 0xF7, 0x68, 0x01, 0x00] },
    // Locked: call 0x1401FF600 (released), then test [rbx+0x424], 0x1000 (pressed again).
    Site { name: "lock-on: lock released call (0x1401E8E9C)", rva: 0x001E_8E9C, expect: &[0xE8, 0x5F, 0x67, 0x01, 0x00] },
    // Locked: call 0x1401FF4F0 (held).
    Site { name: "lock-on: lock held call (0x1401E8EFF)", rva: 0x001E_8EFF, expect: &[0xE8, 0xEC, 0x65, 0x01, 0x00] },
    // Held: call 0x1401FF4A0 (the right stick's direction changed).
    Site { name: "lock-on: stick flick call (0x1401E8F2D)", rva: 0x001E_8F2D, expect: &[0xE8, 0x6E, 0x65, 0x01, 0x00] },
    // Flicked: call 0x1401CD4B0 (the target that way), with ecx = [rbx+0x850].
    Site { name: "lock-on: target switch call (0x1401E8F3C)", rva: 0x001E_8F3C, expect: &[0xE8, 0x6F, 0x45, 0xFE, 0xFF] },
];

/// What KH1's lock rules rely on. Checked only.
pub const LOCK_ON_RULES_CHECKED: &[Site] = &[
    // cmp eax,-1; lea rdi,[rbx+0x84C]; cmove eax,ebp; mov [rdi],eax: Re:CoM's pick to +0x84C.
    Site { name: "lock-on: its pick to +0x84C (0x1401E8DF3)", rva: 0x001E_8DF3, expect: &[0x83, 0xF8, 0xFF, 0x48, 0x8D, 0xBB, 0x4C, 0x08, 0x00, 0x00, 0x0F, 0x44, 0xC5, 0x89, 0x07] },
    // After the released call: test eax,eax; je; test dword [rbx+0x424], 0x1000; je.
    Site { name: "lock-on: pressed again test (0x1401E8EA1)", rva: 0x001E_8EA1, expect: &[0x85, 0xC0, 0x74, 0x53, 0xF7, 0x83, 0x24, 0x04, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x74, 0x47] },
    // mov rax,[rbx+0x3F8]; lea rcx,[rbx+0x230]; cmp [rax+0xE0],ebp: the cursor's locked flag.
    Site { name: "lock-on: cursor locked test (0x1401E8E2E)", rva: 0x001E_8E2E, expect: &[0x48, 0x8B, 0x83, 0xF8, 0x03, 0x00, 0x00, 0x48, 0x8D, 0x8B, 0x30, 0x02, 0x00, 0x00, 0x39, 0xA8, 0xE0, 0x00, 0x00, 0x00] },
    // The switch: push rbx; sub rsp,0x30; mov qword [rsp+0x20],-2; mov ebx,ecx.
    Site { name: "lock-on target switch (0x1401CD4B0)", rva: 0x001C_D4B0, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF, 0x8B] },
    // The flick: enabled, stick past 0, its direction sector changed (+0x2C != +0x30).
    Site { name: "lock-on stick flick (0x1401FF4A0)", rva: 0x001F_F4A0, expect: &[0xF6, 0x01, 0x01, 0x74, 0x1B, 0xF3, 0x0F, 0x10, 0x49, 0x20, 0x0F, 0x57, 0xC0, 0x0F, 0x2F, 0xC8, 0x76, 0x0E, 0x8B, 0x41, 0x30, 0x39, 0x41, 0x2C, 0x74, 0x06, 0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3, 0x33, 0xC0, 0xC3] },
    // The pad's pressed test `(mask, pad)` `0x14029B2F0` (jmp 0x14029C240) is checked as "pad fresh".
    // cmp edx,2; jae; … mov eax,[rax+0x14]; and eax,ecx; ret: pad edx's pressed word & mask.
    Site { name: "pad pressed (0x14029C240)", rva: 0x0029_C240, expect: &[0x83, 0xFA, 0x02, 0x73, 0x13, 0x8B, 0xC2, 0x48, 0x8D, 0x15, 0xF2, 0x9A, 0x82, 0x00, 0x48, 0x8B, 0x04, 0xC2, 0x8B, 0x40, 0x14, 0x23, 0xC1, 0xC3, 0x33, 0xC0, 0xC3] },
    // The cursor's measure point `0x1401F0AB0` (the main player's mover +0x30) is checked in
    // `GIMMICK_HP_CALLEES` ("Sora's point").
    // The cursor update's reach while locked: battle (+0xF8 & 4) 30 m else 3 m; and 14 m while
    // +0xF4 (a lock or a switch this frame).
    Site { name: "cursor locked reach (0x14020919B)", rva: 0x0020_919B, expect: &[0x41, 0xF6, 0x06, 0x04, 0xF3, 0x0F, 0x11, 0x87, 0x00, 0x01, 0x00, 0x00, 0x74, 0x0A, 0xF3, 0x0F, 0x10, 0x0D, 0x5B, 0x80, 0x4B, 0x00, 0xEB, 0x08, 0xF3, 0x0F, 0x10, 0x0D, 0x8D, 0x00, 0x30, 0x00, 0x0F, 0x2F, 0xC1, 0x0F, 0x87, 0xA0, 0xFD, 0xFF, 0xFF, 0x0F, 0x2F, 0x05, 0x8D, 0x00, 0x30, 0x00, 0x76, 0x0C, 0x39, 0xAF, 0xF4, 0x00, 0x00, 0x00, 0x0F, 0x85, 0x8B, 0xFD, 0xFF, 0xFF] },
    // Its constants: 3.0, 3.5, 7.0, 9.0, 14.0 (the take reach: 3.0 at +0, 14.0 at +0x10).
    Site { name: "cursor reach constants (0x140509248)", rva: 0x0050_9248, expect: &[0x00, 0x00, 0x40, 0x40, 0x00, 0x00, 0x60, 0x40, 0x00, 0x00, 0xE0, 0x40, 0x00, 0x00, 0x10, 0x41, 0x00, 0x00, 0x60, 0x41] },
];

/// 0.132.0, the Moogle's speech bubble (`win::shop::bubble_line`, `crate::text_fit::wrap`,
/// `docs/recom-m8-items.md` § M8d text fit): Re:CoM's `CStrPrim` vtable (`0x140508FD0`, a
/// `CTextWnd`'s text; every code reference a store, no absolute pointer: Checked (exe scan)), copied
/// for a squeezed bubble line as C2DFnt's is. Checked lazily (a mismatch turns only the bubble's
/// squeeze off; the wrap stays).
pub const TEXT_FIT_STR_PTRS: &[PtrSite] = &[
    PtrSite { name: "CStrPrim vtable −8: its COL (0x140508FC8)", rva: 0x0050_8FC8, target: 0x0070_B5E8 },
    PtrSite { name: "CStrPrim vtable +0: deleting dtor (0x140508FD0)", rva: 0x0050_8FD0, target: 0x0029_A080 },
    PtrSite { name: "CStrPrim vtable +8: register (0x140508FD8)", rva: 0x0050_8FD8, target: 0x0026_1D70 },
    PtrSite { name: "CStrPrim vtable +0x10: packet size (0x140508FE0)", rva: 0x0050_8FE0, target: 0x0028_9430 },
    PtrSite { name: "CStrPrim vtable +0x18: draw (0x140508FE8)", rva: 0x0050_8FE8, target: 0x0028_9450 },
    PtrSite { name: "CStrPrim vtable +0x20: depth (0x140508FF0)", rva: 0x0050_8FF0, target: 0x000D_3D00 },
    PtrSite { name: "CStrPrim vtable +0x28: measure (0x140508FF8)", rva: 0x0050_8FF8, target: 0x0028_9440 },
    PtrSite { name: "CStrPrim vtable +0x30 (0x140509000)", rva: 0x0050_9000, target: 0x0025_8E80 },
];

/// 0.132.0: the `CTextWnd` fields the bubble's box is read from (`crate::text_fit::recom::WND_*`,
/// `CTEXT_*`, `SIZE`, `GAP`). Checked only.
pub const BUBBLE_CHECKED: &[Site] = &[
    // The open 0x1402B1520: movzx eax,[rsi+0x28]; movzx ecx,[rsi+0x2A] (the record's margins);
    // mov [rbp+0xB0],eax; mov [rbp+0xB4],ecx.
    Site { name: "CTextWnd open: the text margins (0x1402B15AD)", rva: 0x002B_15AD, expect: &[0x0F, 0xB7, 0x46, 0x28, 0x0F, 0xB7, 0x4E, 0x2A, 0x89, 0x85, 0xB0, 0x00, 0x00, 0x00, 0x89, 0x8D, 0xB4, 0x00, 0x00, 0x00] },
    // 0x1402B1F70: mov rax,[rdx]; mov [rsi+0xA8],rax (place); mov rax,[r8]; mov [rsi+0xB8],rax (size).
    Site { name: "CTextWnd set-up: place and size (0x1402B1FA2)", rva: 0x002B_1FA2, expect: &[0x48, 0x8B, 0x02, 0x48, 0x89, 0x86, 0xA8, 0x00, 0x00, 0x00, 0x49, 0x8B, 0x00, 0x48, 0x89, 0x86, 0xB8, 0x00, 0x00, 0x00] },
    // CText set-up 0x1402B1D50: its CStrPrim (+0x38) gets size << 4 at +0x3C, spacing << 4 at +0x44,
    // line gap << 4 at +0x48; then its measure (vtable +0x28) on +0x28, the line count to +0x54.
    Site { name: "CText set-up: font size, gap and line count (0x1402B1E7E)", rva: 0x002B_1E7E, expect: &[0x48, 0x8B, 0x4E, 0x38, 0x8B, 0x84, 0x24, 0x80, 0x00, 0x00, 0x00, 0xC1, 0xE0, 0x04, 0x89, 0x41, 0x3C, 0x48, 0x8B, 0x4E, 0x38, 0x8B, 0x84, 0x24, 0x98, 0x00, 0x00, 0x00, 0xC1, 0xE0, 0x04, 0x89, 0x41, 0x44, 0x8B, 0x84, 0x24, 0x9C, 0x00, 0x00, 0x00, 0xC1, 0xE0, 0x04, 0x89, 0x41, 0x48, 0x48, 0x8B, 0x4E, 0x38, 0x48, 0x8B, 0x01, 0x45, 0x33, 0xC9, 0x45, 0x33, 0xC0, 0x48, 0x8D, 0x56, 0x28, 0xFF, 0x50, 0x28, 0x89, 0x46, 0x54] },
    // The text call 0x1402B0EF0: rbx = [rcx+0xD0] (its CText); shown = [rcx+0x88] == 5.
    Site { name: "CTextWnd text: its CText and shown state (0x1402B0EFF)", rva: 0x002B_0EFF, expect: &[0x48, 0x8B, 0x99, 0xD0, 0x00, 0x00, 0x00, 0x4D, 0x8B, 0xD0, 0x48, 0x8B, 0xFA, 0x48, 0x8B, 0xF1, 0x48, 0x85, 0xDB, 0x75, 0x05, 0x48, 0x85, 0xD2, 0x74, 0x46, 0x83, 0xB9, 0x88, 0x00, 0x00, 0x00, 0x05] },
];

/// Every site the startup check reads: checked-only, hooked, called.
/// M12 camera (`crate::camera`, `win::camera`, `docs/recom-m12-camera-plan.md`): Re:CoM's player
/// camera `PlayerCamera::CPlayerCamera`. Its init `0x1401FB530` (only caller the ctor `0x1401FB140`)
/// is wrapped: after Re:CoM's base mode (field 1, battle 3) the KH1 camera's mode when the Camera
/// setting asks for it. Same rule as [`HOOKS`]. Checked (exe + decomp).
pub const CAMERA_HOOKS: &[Site] = &[
    // mov r11,rsp; mov [r11+8],rbx
    Site { name: "CPlayerCamera init (0x1401FB530)", rva: 0x001F_B530, expect: &[0x4C, 0x8B, 0xDC, 0x49, 0x89, 0x5B, 0x08] },
];

/// The camera's patched bytes. The init's two `lea rax,[rip+…]` are the only references to the field
/// and battle execs `0x1401FEEB0` / `0x1401FECE0` (exe xref scan, plan § 2.2): pointed at our
/// dispatcher's jumps. The two record-table reads (`0x14087B660`, 6 slots; its only indexed readers,
/// Checked (decomp)) in the mode setters `0x1401FC4D0` / `0x1401FBE60`: a jump to a stub that reads
/// our records for modes 6..9 and Re:CoM's table otherwise (`camera::stub`). Checked (exe).
pub const CAMERA_PATCHES: &[Site] = &[
    // lea rax,[rip+0x3953] (0x1401FEEB0)
    Site { name: "camera init: field exec lea (0x1401FB556)", rva: 0x001F_B556, expect: &[0x48, 0x8D, 0x05, 0x53, 0x39, 0x00, 0x00] },
    // lea rax,[rip+0x3752] (0x1401FECE0)
    Site { name: "camera init: battle exec lea (0x1401FB587)", rva: 0x001F_B587, expect: &[0x48, 0x8D, 0x05, 0x52, 0x37, 0x00, 0x00] },
    // lea rdx,[rip+0x67F146] (0x14087B660); mov rsi,rcx; mov rdx,[rdx+rax*8]
    Site { name: "camera mode set: record read (0x1401FC513)", rva: 0x001F_C513, expect: &[0x48, 0x8D, 0x15, 0x46, 0xF1, 0x67, 0x00, 0x48, 0x8B, 0xF1, 0x48, 0x8B, 0x14, 0xC2] },
    // lea rdx,[rip+0x67F7D4] (0x14087B660); mov rdx,[rdx+rax*8]
    Site { name: "camera mode keep: record read (0x1401FBE85)", rva: 0x001F_BE85, expect: &[0x48, 0x8D, 0x15, 0xD4, 0xF7, 0x67, 0x00, 0x48, 0x8B, 0x14, 0xC2] },
];

/// Re:CoM's camera functions the dispatcher and the KH1 step call. Checked (exe + decomp).
pub const CAMERA_CALLEES: &[Site] = &[
    Site { name: "camera mode set, re-place (0x1401FC4D0)", rva: 0x001F_C4D0, expect: &[0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x10, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56] },
    Site { name: "camera mode set, keep (0x1401FBE60)", rva: 0x001F_BE60, expect: &[0x48, 0x89, 0x5C, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x50] },
    Site { name: "camera field exec (0x1401FEEB0)", rva: 0x001F_EEB0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0xF6] },
    Site { name: "camera battle exec (0x1401FECE0)", rva: 0x001F_ECE0, expect: &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48] },
    Site { name: "camera collision tail (0x1401FC0D0)", rva: 0x001F_C0D0, expect: &[0x48, 0x83, 0xEC, 0x28, 0xF6, 0x81, 0x88, 0x01, 0x00, 0x00, 0x20, 0x0F, 0x85, 0x87, 0x00, 0x00] },
    Site { name: "camera view write (0x14024FA40)", rva: 0x0024_FA40, expect: &[0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x56, 0x48, 0x8B, 0xEC, 0x48, 0x83, 0xEC, 0x60] },
    Site { name: "camera sweep (0x1401FBC60)", rva: 0x001F_BC60, expect: &[0x40, 0x53, 0x55, 0x56, 0x48, 0x83, 0xEC, 0x60, 0x0F, 0x29, 0x7C, 0x24, 0x40] },
    Site { name: "camera angles (0x1401FB890)", rva: 0x001F_B890, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x60, 0x0F, 0x29, 0x74, 0x24, 0x50] },
    // jmp 0x14029BFA0 / jmp 0x14029BFC0 (the pad's +0x6C / +0x70)
    Site { name: "pad right stick x (0x14029B280)", rva: 0x0029_B280, expect: &[0xE9, 0x1B, 0x0D, 0x00, 0x00] },
    Site { name: "pad right stick y (0x14029B290)", rva: 0x0029_B290, expect: &[0xE9, 0x2B, 0x0D, 0x00, 0x00] },
    // The keyboard / mouse getter `0x1403471C0` is checked in `LOCK_ON_CHECKED`.
    Site { name: "camera lock target point (0x1401FC400)", rva: 0x001F_C400, expect: &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0x8B, 0x89, 0xC0, 0x02, 0x00, 0x00, 0x85] },
    Site { name: "camera record override (0x1401FEB50)", rva: 0x001F_EB50, expect: &[0x48, 0x83, 0xEC, 0x28, 0x4C, 0x8B, 0x05, 0x05, 0xC9, 0x67, 0x00, 0x4D, 0x85, 0xC0] },
];

pub fn all_sites() -> impl Iterator<Item = &'static Site> {
    // One flat list (a long `chain` of iterators overflows the type checker).
    const LISTS: &[&[Site]] = &[
        SITES,
        HOOKS,
        CALLEES,
        MOVE_HOOKS,
        TRACE_HOOKS,
        MOVE_CALLEES,
        ATTACK_HOOKS,
        ATTACK_CALLEES,
        ATTACK_DATA,
        STATS_HOOKS,
        STATS_CALLEES,
        DAMAGE_CHECKED,
        KEYBLADE_HOOKS,
        KEYBLADE_CALLEES,
        KEYBLADE_DATA,
        MODEL_HOOKS,
        MODEL_CALLEES,
        M6_HOOKS,
        M6_PACE_HOOKS,
        M6_PACE_CHECKED,
        CAGE_CHECKED,
        CAGE_FAINT_CHECKED,
        OOGIE_CHECKED,
        M6_CALLEES,
        M6_CHECKED,
        M6_DECK_HOOKS,
        M6_DECK_READERS,
        M6_DECK_CALLEES,
        M6_COCKPIT_CALLS,
        M6_CARD_CALLS,
        M6_DECK_CHECKED,
        M6_GETTER_CALLERS,
        M6_ENEMY_UI_CALLS,
        M6_ENEMY_UI_CHECKED,
        M6_ENEMY_SOUND_CALLS,
        M6_ENEMY_SOUND_CHECKED,
        M6_BOSS_HOOKS,
        REVENGE_HOOKS,
        REVENGE_ALLOC,
        DIM_CHECKED,
        DIM_NORMAL_CHECKED,
        REVENGE_CHECKED,
        MP_HOOKS,
        MP_CHECKED,
        MP_BAR_HOOKS,
        KH1_HUD_HOOKS,
        KH1_HUD_PARTY_HOOKS,
        KH1_HUD_CURSOR_HOOKS,
        KH1_HUD_POPUP_HOOKS,
        KH1_HUD_PLATE_HOOKS,
        KH1_HUD_MASK_CALLS,
        KH1_HUD_MASK_CHECKED,
        KH1_HUD_CALLEES,
        MP_BAR_CALLEES,
        MP_BAR_CHECKED,
        COMMAND_MENU_CALLEES,
        COMMAND_MENU_CHECKED,
        COMMAND_MENU_POINT_CALLS,
        HUD_2D_CALLEES,
        HUD_2D_CHECKED,
        MAGIC_HOOKS,
        MAGIC_CALLEES,
        MAGIC_DATA,
        MAGIC_CHECKED,
        BATTLE_END_CHECKED,
        UNLOCK_HOOKS,
        UNLOCK_CHECKED,
        MENU_SOUND_HOOKS,
        MENU_SOUND_CALLEES,
        ITEM_HOOKS,
        ITEM_DATA,
        ITEM_CHECKED,
        ITEM_PAGE_CALLEES,
        DROP_HOOKS,
        DROP_DATA,
        DROP_CALLEES,
        DROP_CHECKED,
        GIMMICK_HOOKS,
        GIMMICK_CALLEES,
        GIMMICK_CHECKED,
        GIMMICK_BREAK_CALLEES,
        GIMMICK_BREAK_CHECKED,
        GIMMICK_HP_CALLEES,
        GIMMICK_HP_CHECKED,
        SHOP_HOOKS,
        SHOP_CALLEES,
        SHOP_CHECKED,
        MAGIC_FX_BRANCHES,
        MAGIC_FX_CHECKED,
        CUSTOMIZE_GLYPHS_CHECKED,
        ABILITY_CALLEES,
        ENEMY_SKILL_HOOKS,
        ENEMY_SKILL_CALLEES,
        ENEMY_SKILL_CHECKED,
        ENEMY_SKILL_STUB_SITES,
        ENEMY_SKILL_REWRITE_HOOK,
        ENEMY_SKILL_REWRITE_SITES,
        ENEMY_SKILL_REWRITE_CHECKED,
        M12_CALLEES,
        M12_CHECKED,
        SPECIAL_HOOKS,
        SPECIAL_CHECKED,
        SPECIAL_TINT_CHECKED,
        SPECIAL_SR_CHECKED,
        SPECIAL_AA_CHECKED,
        SPECIAL_RAG_CHECKED,
        SUMMON_CHECKED,
        FLATTEN_HOOKS,
        FLATTEN_CHECKED,
        FLATTEN_EXE_HOOK,
        FLATTEN_EXE_CHECKED,
        PLUTO_HOOKS,
        PLUTO_CHECKED,
        FIELD_RULE_CALLS,
        FIELD_RULE_CHECKED,
        FIELD_SLEIGHT_HOOKS,
        FIELD_SLEIGHT_DATA,
        FIELD_SLEIGHT_CHECKED,
        PARTY_CHECKED,
        EFFECT_CHECKED,
        FX_HOOKS,
        FX_CALLS,
        FX_CHECKED,
        STORY_SKIP_HOOKS,
        STORY_SKIP_CALLEES,
        SHADOW_HOOKS,
        SHADOW_ALLOC,
        SHADOW_CALLEES,
        SOLDIER_HOOKS,
        SOLDIER_ALLOC,
        AIRSOLDIER_HOOKS,
        AIRSOLDIER_ALLOC,
        POWERWILD_HOOKS,
        POWERWILD_ALLOC,
        DARKBALL_HOOKS,
        DARKBALL_ALLOC,
        BANDIT_HOOKS,
        BANDIT_ALLOC,
        M11_GUARD_POPUP_CALLS,
        M11_GUARD_POPUP_CHECKED,
        THROWN_CALLEES,
        AIRPIRATE_HOOKS,
        AIRPIRATE_ALLOC,
        WYVERN_HOOKS,
        WYVERN_ALLOC,
        SEARCHGHOST_HOOKS,
        SEARCHGHOST_ALLOC,
        KH1_ENEMY_REWARD_CALLEES,
        PIRATE_HOOKS,
        PIRATE_ALLOC,
        LARGEBODY_HOOKS,
        LARGEBODY_ALLOC,
        KH1_ENEMY_RING_CALLEES,
        KH1_ENEMY_RING_HOOKS,
        KH1_ENEMY_CLASH_CALLEES,
        NEOSHADOW_HOOKS,
        NEOSHADOW_ALLOC,
        WIGHTKNIGHT_HOOKS,
        WIGHTKNIGHT_ALLOC,
        FATBANDIT_HOOKS,
        FATBANDIT_ALLOC,
        FLAME_CALLEES,
        GARGOYLE_HOOKS,
        GARGOYLE_ALLOC,
        DEFENDER_HOOKS,
        DEFENDER_ALLOC,
        REDNOCTURNE_HOOKS,
        REDNOCTURNE_ALLOC,
        BLUERHAPSODY_HOOKS,
        BLUERHAPSODY_ALLOC,
        GREENREQUIEM_HOOKS,
        GREENREQUIEM_ALLOC,
        GREENREQUIEM_CURE_CHECKED,
        YELLOWOPERA_HOOKS,
        YELLOWOPERA_ALLOC,
        HELD_CALLEES,
        GRAB_CALLEES,
        BOUNCYWILD_HOOKS,
        BOUNCYWILD_ALLOC,
        BOUNCYWILD_CHECKED,
        SLIP_SCATTER_CALLS,
        SLIP_SCATTER_CHECKED,
        SECOND_CHANCE_PATCH,
        SECOND_CHANCE_CHECKED,
        DAMAGE_CAP_PATCH,
        DAMAGE_CAP_CHECKED,
        crate::enemy_skills_cmp::PATCH,
        crate::enemy_skills_cmp::CALLS,
        crate::enemy_skills_cmp::CHECKED,
        TEXT_FIT_CHECKED,
        BUBBLE_CHECKED,
        TITLE_HOLD_HOOK,
        TITLE_HOLD_CHECKED,
        LOCK_ON_HOOK,
        LOCK_ON_CHECKED,
        LOCK_ON_CALLS,
        LOCK_ON_RULES_CHECKED,
        CAMERA_HOOKS,
        CAMERA_PATCHES,
        CAMERA_CALLEES,
    ];
    LISTS
        .iter()
        .flat_map(|l| l.iter())
        .chain(M6_PATCHES.iter().map(|p| &p.site))
        .chain(M6_BOSS_PATCHES.iter().map(|p| &p.site))
        .chain(ENEMY_SKILL_PATCHES.iter().map(|p| &p.site))
        .chain(std::iter::once(&MP_CHECKPOINT_CALL))
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

    /// No byte site holds an absolute exe pointer (the loader relocates those; a byte check of one fails on every
    /// start, as 0.98.1's vtable checks did): pointers go in `PtrSite`s.
    #[test]
    fn no_byte_site_holds_a_relocated_pointer() {
        for s in all_sites() {
            for w in s.expect.windows(8) {
                let v = u64::from_le_bytes(w.try_into().unwrap());
                assert!(!(0x1_4000_1000..0x1_4100_0000).contains(&v), "{}: an absolute pointer {v:#x} in its bytes", s.name);
            }
        }
    }

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
    fn dim_sites() {
        // The setter's table global (rip-relative at +9, 7 bytes) and the slot size / fields.
        let s = &DIM_CHECKED[0];
        let d = i32::from_le_bytes(s.expect[12..16].try_into().unwrap()) as i64;
        assert_eq!(0x1_4000_0000i64 + s.rva as i64 + 16 + d, 0x1_4000_0000i64 + crate::dim::ENV_TABLE as i64);
        assert_eq!(&s.expect[5..9], &[0x48, 0xC1, 0xE0, 0x06]);
        assert_eq!(1usize << 6, crate::dim::SLOT_SIZE);
        assert!(s.expect.windows(7).any(|w| w == [0xC7, 0x40, crate::dim::slot::T as u8, 0, 0, 0, 0]));
        assert!(s.expect.windows(4).any(|w| w == [0x66, 0x0F, 0x7F, 0x40]) && s.expect.windows(5).any(|w| w == [0x0F, 0x7F, 0x40, crate::dim::slot::TARGET as u8, 0x41]));
        assert!(s.expect.windows(4).any(|w| w == [0x83, 0x48, crate::dim::slot::FLAGS as u8, 0x01]));
        assert!(s.expect.ends_with(&[0x89, 0x48, crate::dim::slot::MASK as u8, 0xC3]));
        assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1);
        // The exe slots are the dim table's, never byte sites, and not another list's pointer.
        let ours: Vec<(u32, u32)> = DIM_EXE_SLOTS.iter().map(|p| (p.rva, p.target)).collect();
        let want: Vec<(u32, u32)> = crate::dim::EXES.iter().map(|x| (x.slot_rva(), x.exe)).collect();
        assert_eq!(ours, want);
        for p in DIM_EXE_SLOTS {
            assert!(!all_sites().any(|s| s.rva == p.rva), "{}", p.name);
            assert!(!REVENGE_PTRS.iter().chain(REVENGE_FSM_SLOTS).chain(REVENGE_WINDUP_SLOTS).any(|q| q.rva == p.rva), "{}", p.name);
            assert_eq!(p.rva % 8, 0, "{}", p.name);
        }
    }

    #[test]
    fn dim_normal_sites() {
        use crate::dim::NORMAL;
        // Each class: its begin slot, then Re:CoM's hurt-off and attack end, read only.
        assert_eq!(DIM_NORMAL_PTRS.len(), 3 * NORMAL.len());
        for (n, p) in NORMAL.iter().zip(DIM_NORMAL_PTRS.chunks(3)) {
            assert_eq!((p[0].rva, p[0].target), (n.begin_slot(), n.begin), "{}", n.name);
            assert_eq!((p[1].rva, p[1].target), (n.vtable + 0x438, 0x0017_DA80), "{}", n.name);
            assert_eq!((p[2].rva, p[2].target), (n.vtable + 0x440, 0x0017_DA90), "{}", n.name);
            assert!(p.iter().all(|q| q.name.starts_with(n.class)));
            // The begin is checked byte for byte and calls the colour setter (rel32 at +0x6C).
            let s = DIM_NORMAL_CHECKED.iter().find(|s| s.rva == n.begin).expect("begin bytes");
            let d = i32::from_le_bytes(s.expect[0x6C..0x70].try_into().unwrap()) as i64;
            assert_eq!(s.expect[0x6B], 0xE8);
            assert_eq!(s.rva as i64 + 0x70 + d, DIM_CHECKED[0].rva as i64, "{}", n.name);
            // mask 0x1800, slot 0.
            assert_eq!(&s.expect[0x57..0x5C], &[0xB9, 0x00, 0x18, 0x00, 0x00]);
            assert_eq!(&s.expect[0x67..0x6B], &[0x89, 0x7C, 0x24, 0x20]);
            // The wrapper of its state calls this slot, then jumps to +0x438.
            let w = DIM_NORMAL_CHECKED.iter().find(|s| s.expect[14..18] == (n.begin_slot() - n.vtable).to_le_bytes()).expect("wrapper");
            assert!(w.expect.ends_with(&[0x48, 0xFF, 0xA0, 0x38, 0x04, 0x00, 0x00]));
        }
        for p in DIM_NORMAL_PTRS {
            assert_eq!(p.rva % 8, 0, "{}", p.name);
            assert!(!all_sites().any(|s| s.rva <= p.rva && p.rva < s.rva + s.expect.len() as u32), "{}", p.name);
            assert!(!REVENGE_PTRS.iter().chain(REVENGE_FSM_SLOTS).chain(REVENGE_WINDUP_SLOTS).chain(DIM_EXE_SLOTS).any(|q| q.rva == p.rva), "{}", p.name);
        }
        for s in DIM_NORMAL_CHECKED {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
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

    /// 0.113.0: what the true squeeze relies on, as the exe's bytes say it.
    #[test]
    fn text_fit_sites() {
        use crate::text_fit::{packet, recom, vtable};
        let find = |n: &str| TEXT_FIT_CHECKED.iter().find(|s| s.name.starts_with(n)).unwrap();
        let rel = |s: &Site, at: usize| s.rva as i64 + at as i64 + 4 + i32::from_le_bytes(s.expect[at..at + 4].try_into().unwrap()) as i64;
        let draw = find("C2DFnt draw thunk");
        assert_eq!(draw.rva, recom::DRAW);
        assert_eq!(&draw.expect[..4], &[0x48, 0x8B, 0x41, recom::TEXT as u8], "reads the text pointer");
        assert_eq!((draw.expect[12], draw.expect[13]), (0x0F, 0x85));
        assert_eq!(rel(draw, 14), recom::DRAW_BODY as i64, "jne to the draw");
        let m = find("C2DFnt measure thunk");
        assert_eq!(m.rva, recom::MEASURE);
        assert_eq!(m.expect[9], 0xE9);
        assert_eq!(rel(m, 10), recom::WIDTH as i64, "jmp to the width routine");
        assert_eq!(find("text width routine").rva, recom::WIDTH);
        let tc = find("C2DFnt draw: text copy");
        assert_eq!(&tc.expect[..4], &[0x49, 0x8B, 0x4F, recom::TEXT as u8]);
        assert_eq!(rel(tc, 12), recom::EXPAND as i64);
        assert_eq!(find("text escape copy").rva, recom::EXPAND);
        // The draw's origin is +0x28 (mov r12d,[r15+0x28]); right / centre alignment subtract the
        // width routine's width from it (call at +0x66).
        let o = find("C2DFnt draw: origin and alignment");
        assert!(o.expect.windows(4).any(|w| w == [0x45, 0x8B, 0x67, recom::X as u8]));
        assert_eq!(o.expect[0x66], 0xE8);
        assert_eq!(rel(o, 0x67), recom::WIDTH as i64);
        // The header pairs, as `packet::HEADER` expects them: mov qword [rbp−0x38], 5 / [rbp−0x30], 9 /
        // [rbp−0x30], 0x17 / [rbp−0x38], 0x356; the call opens the packet.
        let h = find("C2DFnt draw: packet header");
        for imm in [5u32, 9, 0x17, 0x356] {
            assert!(h.expect.windows(4).any(|w| w == imm.to_le_bytes()), "{imm:#x}");
        }
        assert!(packet::HEADER.iter().any(|&(i, v)| (i, v) == (2, 5)) && packet::HEADER.iter().any(|&(i, v)| (i, v) == (6, 0x356)));
        assert_eq!(h.expect[14], 0xE8);
        let open = find("packet open");
        assert_eq!(rel(h, 15), open.rva as i64);
        // Open and close read the same holder (mov r8 / rcx, [rip+..]).
        assert_eq!(rel(open, 3), recom::PACKET_HOLDER as i64);
        let close = find("packet close");
        assert_eq!(rel(close, 6), recom::PACKET_HOLDER as i64);
        assert_eq!(rel(find("C2DFnt draw: packet end"), 30), close.rva as i64);
        // XYZ2: x + 0x8000 & 0x1FFFF in the low 17 bits, register 5.
        let x = find("glyph quad: XYZ2 packing");
        assert!(x.expect.starts_with(&[0x8D, 0x86, 0x00, 0x80, 0x00, 0x00, 0x25, 0xFF, 0xFF, 0x01, 0x00]));
        assert!(x.expect.windows(11).any(|w| w == [0x8D, 0x85, 0x00, 0x80, 0x00, 0x00, 0x25, 0xFF, 0xFF, 0x01, 0x00]));
        assert!(x.expect.windows(8).any(|w| w == [0x48, 0xC7, 0x44, 0x24, 0x10, packet::XYZ as u8, 0, 0]));
        // The vtable: COL then 7 consecutive slots, draw and measure where `vtable` puts them.
        assert_eq!(TEXT_FIT_PTRS.len(), 1 + vtable::SLOTS);
        assert_eq!(TEXT_FIT_PTRS[0].rva, recom::VTABLE - 8);
        for (i, p) in TEXT_FIT_PTRS[1..].iter().enumerate() {
            assert_eq!(p.rva, recom::VTABLE + 8 * i as u32, "{}", p.name);
        }
        assert_eq!(TEXT_FIT_PTRS[1 + vtable::DRAW].target, recom::DRAW);
        assert_eq!(TEXT_FIT_PTRS[1 + vtable::MEASURE].target, recom::MEASURE);
        for p in TEXT_FIT_PTRS {
            assert!(!all_sites().any(|s| s.rva == p.rva), "{}", p.name);
        }
    }

    /// The first-launch build's hold: the hooked function is CTitle's state-1 update, its
    /// sub-state field and values as `crate::asset_build::title` says them.
    #[test]
    fn title_hold_sites() {
        use crate::asset_build::title;
        assert_eq!(TITLE_HOLD_HOOK[0].rva, title::STATE1_UPDATE);
        let find = |n: &str| TITLE_HOLD_CHECKED.iter().find(|s| s.name.starts_with(n)).unwrap();
        let lea = find("CTitle ctor");
        assert_eq!(&lea.expect[..3], &[0x48, 0x8D, 0x05]);
        let target = lea.rva as i64 + 7 + i32::from_le_bytes(lea.expect[3..7].try_into().unwrap()) as i64;
        assert_eq!(target, title::STATE1_UPDATE as i64, "the ctor's lea names the hooked update");
        let sub = find("CTitle state 1 update: its sub-states");
        assert_eq!(sub.rva, title::STATE1_UPDATE + TITLE_HOLD_HOOK[0].expect.len() as u32, "right after the moved bytes");
        assert_eq!(&sub.expect[..6], &[0x8B, 0x81, title::SUB as u8, (title::SUB >> 8) as u8, 0, 0]);
        assert_eq!(&sub.expect[17..20], &[0x83, 0xF8, title::WAIT as u8]);
        assert_eq!(&sub.expect[22..25], &[0x83, 0xF8, 0x14]);
    }

    /// Lock-on on R1: Re:CoM's lock-on, its button field and the device flag, as the exe says them.
    #[test]
    fn lock_on_sites() {
        use crate::lock_on::{bits, recom};
        assert_eq!(LOCK_ON_HOOK[0].rva, recom::LOCK_ON);
        let find = |n: &str| LOCK_ON_CHECKED.iter().find(|s| s.name.starts_with(n)).unwrap();
        let rel = |s: &Site, at: usize| s.rva as i64 + at as i64 + 4 + i32::from_le_bytes(s.expect[at..at + 4].try_into().unwrap()) as i64;
        let up = find("player pad update");
        // mov dword [rcx+0x44], 2: Re:CoM's lock-on button is R2; R1 is +0x3C's 8.
        assert_eq!(&up.expect[17..21], &[0xC7, 0x41, recom::LOCK_BUTTON as u8, bits::R2 as u8]);
        assert_eq!(&up.expect[7..11], &[0xC7, 0x41, 0x3C, bits::R1 as u8]);
        assert_eq!(&up.expect[0..4], &[0xC7, 0x41, 0x38, bits::L1 as u8]);
        // The readers: +0x44, then the pad's held / released / pressed tests.
        for (n, target) in [("lock button held", 0x0029_B2C0i64), ("lock button released", 0x0029_B2D0), ("lock button pressed", 0x0029_B2F0)] {
            let r = find(n);
            assert_eq!(&r.expect[10..13], &[0x8B, 0x49, recom::LOCK_BUTTON as u8], "{n}");
            assert_eq!(r.expect[16], 0xE8);
            assert_eq!(rel(r, 17), target, "{n}");
        }
        // The device flag: the detection writes it (1 on keys / mouse), the getter reads it.
        let d = find("device in use");
        assert_eq!(rel(d, 15) + 1, recom::KBM_IN_USE as i64, "cmp dword [rip+..], 0 (an imm8 after the disp)");
        assert_eq!(rel(d, 28) + 4, recom::KBM_IN_USE as i64, "mov dword [rip+..], 1 (an imm32 after the disp)");
        assert_eq!(rel(find("keyboard / mouse in use getter"), 2), recom::KBM_IN_USE as i64);
    }

    /// 0.137.0, KH1's lock rules: the five calls go where `crate::lock_on::recom` says, inside the
    /// hooked lock-on; the fields and reaches as the exe's bytes say them.
    #[test]
    fn lock_on_rules_sites() {
        use crate::hookplan::call_target;
        use crate::lock_on::{recom, take_reach};
        let calls = [
            (recom::CALL_PRESSED, recom::PRESSED),
            (recom::CALL_RELEASED, recom::RELEASED),
            (recom::CALL_HELD, recom::HELD),
            (recom::CALL_FLICK, recom::FLICK),
            (recom::CALL_SWITCH, recom::SWITCH),
        ];
        assert_eq!(LOCK_ON_CALLS.len(), calls.len());
        for (s, (at, to)) in LOCK_ON_CALLS.iter().zip(calls) {
            assert_eq!(s.rva, at, "{}", s.name);
            assert_eq!(call_target(s.rva as u64, s.expect), Some(to as u64), "{}", s.name);
            // Inside Re:CoM's lock-on (0x24D bytes), past the hook's moved bytes.
            assert!(s.rva >= recom::LOCK_ON + LOCK_ON_HOOK[0].expect.len() as u32 && s.rva + 5 <= recom::LOCK_ON + 0x24D, "{}", s.name);
            assert_eq!(all_sites().filter(|x| x.rva < s.rva + 5 && s.rva < x.rva + x.expect.len() as u32).count(), 1, "{}: no other site overlaps", s.name);
        }
        // The callees are the ones `LOCK_ON_CHECKED` / here check.
        let find = |n: &str| LOCK_ON_RULES_CHECKED.iter().chain(LOCK_ON_CHECKED).find(|s| s.name.starts_with(n)).unwrap();
        assert_eq!(find("lock button pressed").rva, recom::PRESSED);
        assert_eq!(find("lock button released").rva, recom::RELEASED);
        assert_eq!(find("lock button held").rva, recom::HELD);
        assert_eq!(find("lock-on stick flick").rva, recom::FLICK);
        assert_eq!(find("lock-on target switch").rva, recom::SWITCH);
        assert!(GIMMICK_HP_CALLEES.iter().any(|s| s.rva == recom::PLAYER_POINT && s.name.starts_with("Sora's point")));
        let thunk = all_sites().find(|s| s.rva == recom::PAD_PRESSED).unwrap();
        assert_eq!(thunk.rva, recom::PAD_PRESSED);
        assert_eq!(thunk.rva as i64 + 5 + i32::from_le_bytes(thunk.expect[1..5].try_into().unwrap()) as i64, find("pad pressed (").rva as i64);
        // The lock button's pressed reader goes through the same thunk.
        let p = find("lock button pressed");
        assert_eq!(p.rva as i64 + 21 + i32::from_le_bytes(p.expect[17..21].try_into().unwrap()) as i64, recom::PAD_PRESSED as i64);
        // The pick store (+0x84C), the pressed-again bit, the cursor's locked flag.
        assert_eq!(&find("lock-on: its pick").expect[6..8], &(recom::PICK as u16).to_le_bytes());
        assert_eq!(&find("lock-on: pressed again").expect[6..12], &[(recom::REQUEST & 0xFF) as u8, (recom::REQUEST >> 8) as u8, 0, 0, 0, 0x10]);
        assert_eq!(recom::SECOND_PRESS, 0x1000);
        let lk = find("lock-on: cursor locked");
        assert_eq!(&lk.expect[3..5], &(recom::CURSOR as u16).to_le_bytes());
        assert_eq!(&lk.expect[10..12], &(recom::PAD as u16).to_le_bytes());
        assert_eq!(&lk.expect[16..18], &(recom::CURSOR_LOCKED as u16).to_le_bytes());
        // The reach: test byte [r14 = cursor+0xF8], 4; the take reach's 14.0 / 3.0 as data.
        let r = find("cursor locked reach");
        assert_eq!(&r.expect[0..4], &[0x41, 0xF6, 0x06, recom::CURSOR_BATTLE as u8]);
        let k = find("cursor reach constants");
        let data = |rva: u32| f32::from_le_bytes(k.expect[(rva - k.rva) as usize..][..4].try_into().unwrap());
        // movss xmm1,[rip+..] → 3.0 (no battle flag); comiss xmm0,[rip+..] → 14.0.
        assert_eq!(r.rva as i64 + 32 + i32::from_le_bytes(r.expect[28..32].try_into().unwrap()) as i64, 0x0050_9248);
        assert_eq!(r.rva as i64 + 48 + i32::from_le_bytes(r.expect[44..48].try_into().unwrap()) as i64, 0x0050_9258);
        assert_eq!(data(0x0050_9248), take_reach(false));
        assert_eq!(data(0x0050_9258), take_reach(true));
    }

    /// 0.132.0: the bubble's text class and fields, as the exe's bytes say them.
    #[test]
    fn bubble_sites() {
        use crate::text_fit::{recom, vtable};
        assert_eq!(TEXT_FIT_STR_PTRS.len(), 1 + vtable::SLOTS);
        assert_eq!(TEXT_FIT_STR_PTRS[0].rva, recom::STR_VTABLE - 8);
        assert_eq!(TEXT_FIT_STR_PTRS[0].target, recom::STR_COL);
        assert_eq!(TEXT_FIT_PTRS[0].target, recom::COL);
        for (i, p) in TEXT_FIT_STR_PTRS[1..].iter().enumerate() {
            assert_eq!(p.rva, recom::STR_VTABLE + 8 * i as u32, "{}", p.name);
            // Both classes share every slot but the dtor and +0x30: the same draw and measure.
            if i != 0 && i != 6 {
                assert_eq!(p.target, TEXT_FIT_PTRS[1 + i].target, "{}", p.name);
            }
        }
        assert_eq!(TEXT_FIT_STR_PTRS[1 + vtable::DRAW].target, recom::DRAW);
        assert_eq!(TEXT_FIT_STR_PTRS[1 + vtable::MEASURE].target, recom::MEASURE);
        let find = |n: &str| BUBBLE_CHECKED.iter().find(|s| s.name.starts_with(n)).unwrap();
        let m = find("CTextWnd open: the text margins");
        assert_eq!(&m.expect[10..12], &(recom::WND_MARGINS as u16).to_le_bytes());
        assert_eq!(&m.expect[16..18], &(recom::WND_MARGINS as u16 + 4).to_le_bytes());
        let z = find("CTextWnd set-up: place and size");
        assert_eq!(&z.expect[16..18], &(recom::WND_SIZE as u16).to_le_bytes());
        let c = find("CText set-up: font size");
        assert_eq!(c.expect[3], recom::CTEXT_FNT as u8);
        assert_eq!(c.expect[16], recom::SIZE as u8);
        assert_eq!(c.expect[46], recom::GAP as u8);
        assert_eq!(*c.expect.last().unwrap(), recom::CTEXT_LINES as u8);
        let t = find("CTextWnd text: its CText");
        assert_eq!(&t.expect[3..5], &(recom::WND_TEXT as u16).to_le_bytes());
        for p in TEXT_FIT_STR_PTRS {
            assert!(!all_sites().any(|s| s.rva == p.rva), "{}", p.name);
        }
    }

    #[test]
    fn damage_cap_sites() {
        use crate::damage_cap::{APPLY, BACK, JOIN};
        let p = &DAMAGE_CAP_PATCH[0];
        assert_eq!(p.expect, &crate::damage_cap::JOIN_BYTES[..], "the stub replays the site's bytes");
        assert_eq!(0x1_4000_0000 + p.rva as u64, JOIN);
        assert_eq!(0x1_4000_0000 + p.rva as u64 + p.expect.len() as u64, BACK);
        assert_eq!(crate::hookplan::call_target(JOIN + 18, &p.expect[18..]), Some(APPLY));
        assert_eq!(0x1_4000_0000 + DAMAGE_CAP_CHECKED[0].rva as u64, APPLY);
        // The handler's two calls: the frozen bonus, then the apply.
        let h = &DAMAGE_CAP_CHECKED[2];
        let at = 0x1_4000_0000 + h.rva as u64;
        assert_eq!(crate::hookplan::call_target(at, &h.expect[..5]), Some(0x1_4015_D940));
        assert_eq!(crate::hookplan::call_target(at + 27, &h.expect[27..]), Some(0x1_4017_DC50));
        // The flatten exe's call to the same HP store.
        let f = &DAMAGE_CAP_CHECKED[3];
        assert_eq!(crate::hookplan::call_target(0x1_4000_0000 + f.rva as u64 + 11, &f.expect[11..]), Some(APPLY));
        assert_eq!(crate::damage_cap::stub(0x7FF0_0000_0000, JOIN - 0x100_0000, APPLY, BACK).map(|s| s.len()), Some(crate::damage_cap::STUB_LEN));
    }

    #[test]
    fn camera_sites() {
        let rel = |s: &Site, at: usize| (0x1_4000_0000u64 + s.rva as u64 + at as u64 + 4).wrapping_add(i32::from_le_bytes(s.expect[at..at + 4].try_into().unwrap()) as i64 as u64);
        // The init's leas reach the two execs; the record reads reach Re:CoM's table.
        assert_eq!(rel(&CAMERA_PATCHES[0], 3), 0x1_401F_EEB0);
        assert_eq!(rel(&CAMERA_PATCHES[1], 3), 0x1_401F_ECE0);
        assert_eq!(rel(&CAMERA_PATCHES[2], 3), crate::camera::stub::RECOM_TABLE);
        assert_eq!(rel(&CAMERA_PATCHES[3], 3), crate::camera::stub::RECOM_TABLE);
        assert_eq!(0x1_4000_0000 + CAMERA_PATCHES[2].rva as u64 + CAMERA_PATCHES[2].expect.len() as u64, crate::camera::stub::SET_PLACE_BACK);
        assert_eq!(0x1_4000_0000 + CAMERA_PATCHES[3].rva as u64 + CAMERA_PATCHES[3].expect.len() as u64, crate::camera::stub::SET_KEEP_BACK);
        assert_eq!(0x1_4000_0000 + CAMERA_PATCHES[2].rva as u64, crate::camera::stub::SET_PLACE_READ);
        assert_eq!(0x1_4000_0000 + CAMERA_PATCHES[3].rva as u64, crate::camera::stub::SET_KEEP_READ);
        // The stub replays the replaced bytes: its own Re:CoM path is the same lea + mov.
        for (p, at) in [(&CAMERA_PATCHES[0], 0x1_401F_EEB0u64), (&CAMERA_PATCHES[1], 0x1_401F_ECE0)] {
            assert_eq!(crate::camera::stub::lea_rax(0x1_4000_0000 + p.rva as u64, at).as_slice(), p.expect);
        }
        // Each table read starts after the setter's own first bytes (no overlap with the callee sites).
        let set = CAMERA_CALLEES.iter().find(|c| c.rva == 0x001F_C4D0).unwrap();
        assert!(set.rva + set.expect.len() as u32 <= CAMERA_PATCHES[2].rva);
        let keep = CAMERA_CALLEES.iter().find(|c| c.rva == 0x001F_BE60).unwrap();
        assert!(keep.rva + keep.expect.len() as u32 <= CAMERA_PATCHES[3].rva);
        let init = &CAMERA_HOOKS[0];
        assert!(init.rva + init.expect.len() as u32 <= CAMERA_PATCHES[0].rva);
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
        let hooks: &[&[Site]] = &[HOOKS, MOVE_HOOKS, TRACE_HOOKS, ATTACK_HOOKS, STATS_HOOKS, KEYBLADE_HOOKS, MODEL_HOOKS, M6_HOOKS, M6_PACE_HOOKS, M6_DECK_HOOKS, M6_DECK_READERS, M6_BOSS_HOOKS, MP_HOOKS, MP_BAR_HOOKS, KH1_HUD_HOOKS, KH1_HUD_PARTY_HOOKS, KH1_HUD_CURSOR_HOOKS, KH1_HUD_POPUP_HOOKS, KH1_HUD_PLATE_HOOKS, MAGIC_HOOKS, UNLOCK_HOOKS, MENU_SOUND_HOOKS, ITEM_HOOKS, DROP_HOOKS, GIMMICK_HOOKS, SHOP_HOOKS, SPECIAL_HOOKS, STORY_SKIP_HOOKS, REVENGE_HOOKS, SHADOW_HOOKS, SOLDIER_HOOKS, AIRSOLDIER_HOOKS, POWERWILD_HOOKS, DARKBALL_HOOKS, BANDIT_HOOKS, AIRPIRATE_HOOKS, WYVERN_HOOKS, SEARCHGHOST_HOOKS, PIRATE_HOOKS, LARGEBODY_HOOKS, NEOSHADOW_HOOKS, WIGHTKNIGHT_HOOKS, FATBANDIT_HOOKS, GARGOYLE_HOOKS, KH1_ENEMY_RING_HOOKS, DEFENDER_HOOKS, BOUNCYWILD_HOOKS, REDNOCTURNE_HOOKS, BLUERHAPSODY_HOOKS, GREENREQUIEM_HOOKS, YELLOWOPERA_HOOKS, CAMERA_HOOKS, LOCK_ON_HOOK];
        for h in hooks.iter().flat_map(|l| l.iter()) {
            assert!(h.expect.len() >= 5 && h.expect.len() + 14 <= crate::hookplan::STUB_AT, "{}", h.name);
            assert!(!h.expect.windows(2).any(|w| w == [0x8B, 0x05] || w == [0x8D, 0x05] || w == [0x8B, 0x0D] || w == [0x10, 0x05]), "{}", h.name);
            assert!(!h.expect.contains(&0xE8) && !h.expect.contains(&0xE9), "{}", h.name);
        }
    }

    #[test]
    fn gimmick_sites() {
        for s in GIMMICK_HOOKS.iter().chain(GIMMICK_CALLEES).chain(GIMMICK_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        let find = |n: &str| GIMMICK_HOOKS.iter().chain(GIMMICK_CALLEES).chain(GIMMICK_CHECKED).find(|s| s.name.starts_with(n)).unwrap();
        // A rip-relative target: the 4-byte displacement at `off + disp` of an instruction `len` long.
        let target = |s: &Site, off: usize, len: usize, disp: usize| {
            let d = i32::from_le_bytes(s.expect[off + disp..off + disp + 4].try_into().unwrap()) as i64;
            (0x1_4000_0000i64 + s.rva as i64 + off as i64 + len as i64 + d) as u64
        };
        const CBTL: u64 = 0x1_4087_A818;
        assert_eq!(target(find("CBTL gimmick now"), 0x13, 7, 3), CBTL);
        assert_eq!(target(find("CBTL gimmick now"), 0x2D, 7, 3), CBTL);
        assert_eq!(target(find("CBTL gimmick clear"), 0, 7, 3), CBTL);
        assert_eq!(target(find("CBTL gimmick count add"), 0, 7, 3), CBTL);
        assert_eq!(target(find("CBTL gimmick count ("), 0, 7, 3), CBTL);
        assert_eq!(target(find("CBTL gimmick playing"), 0, 7, 3), CBTL);
        // The fields the DLL reads and the setters write.
        let u32_at = |s: &Site, at: usize| u32::from_le_bytes(s.expect[at..at + 4].try_into().unwrap()) as usize;
        use crate::gimmick::{cbtl, player};
        assert_eq!(u32_at(find("CBTL gimmick now"), 0x29), cbtl::NOW);
        assert_eq!(u32_at(find("CBTL gimmick now"), 0x37), cbtl::FLAGS);
        assert_eq!(u32_at(find("CBTL gimmick count add"), 0x0E), cbtl::COUNT);
        assert_eq!(u32_at(find("CBTL gimmick count ("), 0x14), cbtl::COUNT);
        assert_eq!(u32_at(find("CBTL gimmick playing"), 0x10), cbtl::NOW);
        assert_eq!(u32_at(find("player card message handler"), 0x23), player::CARD);
        assert_eq!(find("player card message handler").expect[0x2F], player::CARD_WON as u8);
        assert_eq!(u32_at(find("card handler: type 6"), 0x12), player::CARD);
        assert_eq!(find("card handler: type 6").expect[9], 9);
        // The revolver play's flag gate: system flag (1, 8) through 0x1401B0870.
        let e = find("revolver play: event gates");
        // mov edx,8; lea ecx,[rdx−7]: (1, 8).
        assert_eq!(((e.expect[1] as i32 + e.expect[7] as i8 as i32) as u32, e.expect[1] as u32), crate::gimmick::EVENT_FLAG);
        assert_eq!(target(e, 8, 5, 1), 0x1_401B_0870);
        // The type table: entry 6 is the type 6 case.
        assert_eq!(u32_at(find("card handler: type table"), 0), 0x001F_58E8);
        // The builder's bit 9 → 0x40, the move exe's 0x40 → the gimmick init (jmp rel32).
        let b = find("request builder: gimmick");
        assert_eq!((b.expect[3], u32_at(b, 8), b.expect[12] as u32), (9, player::REQUEST, player::REQUEST_GIMMICK));
        let m = find("move exe: gimmick request");
        assert_eq!(m.expect[1] as u32, player::REQUEST_GIMMICK);
        assert_eq!(target(m, 0x0E, 5, 1), 0x1_401F_25D0);
        assert_eq!(find("gimmick init").expect[39] as u32, player::GIMMICK_ACTION);
        // The revolver step reads the card-info copy at 0x140528C30 and calls 0x140161D60(&card, 1).
        let r = find("revolver play: gimmick step");
        assert_eq!(u32_at(r, 0x25), 0x0052_8C30);
        assert_eq!(target(r, 0x35, 5, 1), 0x1_4016_1D60);
        // The prize add's branch: the main player, its revolver +0x3F0, the add, then the notice.
        let a = find("prize add: type 5 / 6");
        assert_eq!(target(a, 0, 7, 3), 0x1_4087_B390);
        assert_eq!(u32_at(a, 0x0F), player::REVOLVER);
        assert_eq!(target(a, 0x2D, 5, 1), 0x1_4016_6300);
        assert_eq!(target(a, 0x3E, 5, 1), 0x1_4010_A0C0);
        // The eight cards' rows.
        for (k, name) in crate::gimmick::BOSSES {
            let s = GIMMICK_CHECKED.iter().find(|s| s.rva == 0x0052_8C30 + k as u32 * 16).unwrap();
            assert!(s.name.contains(name) && s.expect[0] & 0xF == crate::gimmick::TYPE, "{}", s.name);
        }
        assert_eq!(GIMMICK_PTRS[0].target, find("player card message handler").rva);
        // Option (c): the break's sites, each the five bosses' two slots.
        for s in GIMMICK_BREAK_CALLEES.iter().chain(GIMMICK_BREAK_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        for b in crate::gimmick::BREAK_BOSSES {
            let h = GIMMICK_BREAK_PTRS.iter().find(|p| p.rva == b.vtable + 0x48).unwrap();
            let st = GIMMICK_BREAK_PTRS.iter().find(|p| p.rva == b.vtable + crate::gimmick::STAGGER_SLOT).unwrap();
            assert!(h.name.starts_with(b.rtti) && st.name.starts_with(b.rtti));
            assert_eq!((h.target, st.target), (0x0018_1620, b.stagger));
        }
        let m3 = GIMMICK_BREAK_CHECKED.iter().find(|s| s.rva == 0x0018_1739).unwrap();
        // lea rax,[rbx+0x800]: state 13; jmp to the store at 0x140181994.
        assert_eq!(u32_at(m3, 0x59), crate::gimmick::state_at(crate::gimmick::STATE_BROKEN));
        assert_eq!(target(m3, 0x5D, 5, 1), 0x1_4018_1994);
        // Trigger (d): the drop code's constants are what `gimmick::drop_desc` uses.
        for s in GIMMICK_HP_CALLEES.iter().chain(GIMMICK_HP_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        let hp = |n: &str| GIMMICK_HP_CHECKED.iter().chain(GIMMICK_HP_CALLEES).find(|s| s.name.starts_with(n)).unwrap();
        let f32_at = |s: &Site, at: usize| f32::from_le_bytes(s.expect[at..at + 4].try_into().unwrap());
        let ga = hp("Guard Armor drop: receiver");
        assert_eq!(target(ga, 0, 5, 1), 0x1_4016_3740);
        assert_eq!(target(ga, 0x0E, 5, 1), 0x1_401A_49B0);
        assert_eq!(target(ga, 0x1F, 8, 4), 0x1_406C_11D8);
        let gs = hp("Guard Armor drop: speed");
        assert_eq!(target(gs, 0, 8, 4), 0x1_4050_9234);
        assert_eq!(target(gs, 0x0E, 5, 1), 0x1_4029_CF00);
        assert_eq!(target(gs, 0x20, 8, 4), 0x1_4052_BA48);
        assert_eq!(target(hp("Trickmaster drop: y add"), 0, 8, 4), 0x1_4054_55BC);
        assert_eq!(f32_at(hp("drop constant 5.0"), 0), 5.0);
        assert_eq!(f32_at(hp("drop constant 4.5"), 0), 4.5);
        assert_eq!(f32_at(hp("drop constant 0.2"), 0), crate::gimmick::DROP_SPEED);
        assert_eq!(f32_at(hp("drop constant -70"), 0), crate::gimmick::DROP_PITCH);
        assert_eq!(f32_at(hp("drop constant pi/4"), 0), std::f32::consts::FRAC_PI_4);
        assert_eq!(f32_at(hp("drop constant -pi/4"), 0), -std::f32::consts::FRAC_PI_4);
        let jp = hp("Jafar drop point");
        assert_eq!([f32_at(jp, 0), f32_at(jp, 4), f32_at(jp, 8), f32_at(jp, 12)], [0.0, 20.0, 0.0, 1.0]);
        let j = hp("Jafar drop: descriptor");
        assert_eq!((target(j, 0, 7, 3), target(j, 7, 7, 3), target(j, 0x13, 8, 4)), (0x1_4066_F780, 0x1_406C_1430, 0x1_4050_9234));
        assert_eq!((u32_at(j, 0x2A), j.expect[0x33]), (0x1_0001, 0x3C));
        let d = hp("Dragon drop: spread");
        assert_eq!((target(d, 0, 8, 4), target(d, 8, 8, 4), target(d, 0x10, 5, 1)), (0x1_4059_9198, 0x1_4059_D250, 0x1_401A_6530));
        for (n, k) in [("Guard Armor drop: kind", 0x3Au8), ("Parasite Cage drop: kind", 0x3E), ("Trickmaster drop: kind", 0x3B)] {
            assert_eq!(hp(n).expect[5], k, "{n}");
        }
        assert_eq!(hp("Dragon drop: kind").expect[4], 0x40);
        assert_eq!(u32_at(hp("Parasite Cage drop: used"), 2), 0xCD4);
        assert_eq!(u32_at(hp("Trickmaster drop: table"), 2), 0x2C8);
        assert_eq!((u32_at(hp("Jafar drop: state 15"), 2), hp("Jafar drop: state 15").expect[6]), (0xE5C, 0xF));
        assert_eq!(u32_at(hp("Dragon drop: footing"), 2), 0xD18);
        assert_eq!(target(hp("CBTL gimmick receiver set"), 0, 7, 3), CBTL);
        assert_eq!(target(hp("Sora's point"), 9, 7, 3), 0x1_4087_B380);
        let t = GIMMICK_BREAK_CHECKED.iter().find(|s| s.rva == 0x0018_19B0).unwrap();
        // Entry (message − 1) = 2 of the table at 0x1401819A8.
        assert_eq!((t.rva - 0x0018_19A8) / 4 + 1, crate::gimmick::MESSAGE_BROKEN);
        assert_eq!(u32_at(t, 0), m3.rva as usize);
    }

    #[test]
    fn keyblade_wrappers_are_the_four_camp_functions() {
        let rvas: Vec<u32> = KEYBLADE_HOOKS.iter().map(|h| h.rva).collect();
        assert_eq!(rvas, vec![0x000D_9C80, 0x000E_3DE0, 0x000D_EA10, 0x000D_7C20]);
        // Each moves at least a whole 5-byte jump's worth of its prologue, and never the Config
        // input (already hooked) or a callee.
        for h in KEYBLADE_HOOKS {
            assert!(h.expect.len() >= 12, "{}", h.name);
            assert!(!HOOKS.iter().chain(CALLEES).chain(KEYBLADE_CALLEES).any(|s| s.rva == h.rva), "{}", h.name);
        }
    }

    /// M12 follow-ups: the summon actors' vtable checks are `crate::summons::ACTOR_VTABLES`, two per
    /// class (slot +0 and the type getter +0x28), pointers only (never byte sites).
    #[test]
    fn summon_actor_vtables_are_checked_pointers() {
        assert_eq!(SUMMON_ACTOR_PTRS.len(), 2 * crate::summons::COUNT);
        for ((class, vt, slot0), p) in crate::summons::ACTOR_VTABLES.iter().zip(SUMMON_ACTOR_PTRS.chunks(2)) {
            assert!(p[0].name.starts_with(class) && p[1].name.starts_with(class), "{class}");
            assert_eq!((p[0].rva, p[0].target), (*vt, *slot0), "{class}");
            assert_eq!((p[1].rva, p[1].target), (*vt + 0x28, crate::summons::ACTOR_TYPE_GETTER), "{class}");
            assert!(!all_sites().any(|s| s.rva == p[0].rva || s.rva == p[1].rva), "{class}");
        }
        for (s, (class, _, _)) in crate::summons::ALL.iter().zip(crate::summons::ACTOR_VTABLES) {
            assert_eq!(s.class, class);
        }
    }

    #[test]
    fn model_pointer_sites_are_data_not_code_sites() {
        // Relocated pointers are never in the byte-checked lists, and never at a hooked address.
        for p in MODEL_PTRS {
            assert!(!all_sites().any(|s| s.rva == p.rva), "{}", p.name);
            assert!(all_sites().any(|s| s.rva == p.target) || p.target == 0x0080_55D0, "{}: target not byte-checked", p.name);
            assert_eq!(p.rva % 8, 0, "{}", p.name);
        }
        // The slot is CPlayerWeapon's +0x30 (vtable 0x1405AADB0) and the put-away is CPlayerSora's +0x100.
        assert_eq!(MODEL_PTRS[0].rva - MODEL_PTRS[1].rva, 0x30);
        assert_eq!(0x1_4000_0000u64 + MODEL_PTRS[2].rva as u64 - 0x100, data::SORA_VTABLE);
    }

    #[test]
    fn guard_reaction_callees_are_checked() {
        // Decision 27 calls these three Re:CoM functions; each is byte-checked at start, and none
        // is hooked.
        for (rva, f) in [(0x0017_D260, crate::guard::func::PICKER), (0x0017_E1D0, crate::guard::func::END_CARD), (0x0014_E5A0, crate::guard::func::SET_LIST)] {
            assert_eq!(rva, f);
            assert!(ATTACK_CALLEES.iter().any(|s| s.rva == rva && s.expect.len() >= 16), "{rva:#x}");
            assert!(!HOOKS.iter().chain(MOVE_HOOKS).chain(ATTACK_HOOKS).chain(STATS_HOOKS).chain(KEYBLADE_HOOKS).chain(MODEL_HOOKS).any(|s| s.rva == rva));
        }
        // The handle lookup it also calls is already checked.
        assert!(MOVE_CALLEES.iter().any(|s| s.rva == 0x002A_D350));
    }

    #[test]
    fn m6_pace_sites() {
        // The attack-end wrapper: the 15 position-free prologue bytes, checked body right after
        // them, Re:CoM's 250 at 0x14017E5F5 and the passed value's store inside it.
        let h = &M6_PACE_HOOKS[0];
        assert_eq!((M6_PACE_HOOKS.len(), h.rva, h.expect.len()), (1, 0x0017_E540, 15));
        assert_eq!(M6_PACE_CHECKED[0].rva, h.rva + 15);
        let body = &M6_PACE_CHECKED[1];
        let at = |rva: u32, bytes: &[u8]| &body.expect[(rva - body.rva) as usize..(rva - body.rva) as usize + bytes.len()] == bytes;
        assert!(at(0x0017_E5F5, &[0xC7, 0x83, 0xC8, 0x02, 0x00, 0x00, 0xFA, 0x00, 0x00, 0x00]));
        assert!(at(0x0017_E5C8, &[0x89, 0xB3, 0xC8, 0x02, 0x00, 0x00]));
        assert_eq!(crate::battle::enemy::COOLDOWN, 0x2C8);
        assert_eq!(0xFA, crate::battle::pace::RECOM_COOLDOWN);
        // No other stage hooks it; the KH1 enemies call it (checked here now, not in SHADOW_CALLEES).
        let hooks: &[&[Site]] = &[HOOKS, MOVE_HOOKS, TRACE_HOOKS, ATTACK_HOOKS, STATS_HOOKS, KEYBLADE_HOOKS, MODEL_HOOKS, M6_HOOKS, M6_DECK_HOOKS, M6_DECK_READERS, M6_BOSS_HOOKS, REVENGE_HOOKS];
        assert!(!hooks.iter().flat_map(|l| l.iter()).any(|s| s.rva == h.rva));
        assert!(!SHADOW_CALLEES.iter().any(|s| s.rva == h.rva));
        assert_eq!(all_sites().filter(|s| s.rva == h.rva).count(), 1);
    }

    #[test]
    fn m6_sites() {
        // The three wrappers: the card play (exactly the 15 bytes before the RIP-relative load),
        // the manager ctor, setPlayCard. None is hooked by another stage or called.
        let rvas: Vec<u32> = M6_HOOKS.iter().map(|h| h.rva).collect();
        assert_eq!(rvas, vec![0x0019_8870, 0x0018_59B0, 0x0016_3830]);
        assert_eq!(M6_HOOKS[0].expect.len(), 15);
        // The card play's checked body starts right after the moved bytes, with that load.
        assert_eq!(M6_CHECKED[0].rva, M6_HOOKS[0].rva + 15);
        assert_eq!(&M6_CHECKED[0].expect[..3], &[0x48, 0x8B, 0x05]);
        let others = || HOOKS.iter().chain(MOVE_HOOKS).chain(TRACE_HOOKS).chain(ATTACK_HOOKS).chain(STATS_HOOKS).chain(KEYBLADE_HOOKS).chain(MODEL_HOOKS);
        for h in M6_HOOKS {
            assert!(!others().any(|s| s.rva == h.rva), "{}", h.name);
            assert!(!CALLEES.iter().chain(MOVE_CALLEES).chain(ATTACK_CALLEES).chain(STATS_CALLEES).chain(M6_CALLEES).any(|s| s.rva == h.rva), "{}", h.name);
        }
        // The task getter is `crate::battle`'s, and Re:CoM's handlers are the ones it compares.
        assert_eq!(M6_CALLEES[0].rva, 0x002A_D300);
        assert!(M6_CHECKED.iter().any(|s| s.rva == crate::battle::HANDLER_MARLUXIA3));
        // The message-2 block ends with `mov [rbx+0x68],rax` (the next state).
        let m2 = M6_CHECKED.iter().find(|s| s.rva == 0x0018_1695).unwrap();
        assert_eq!(&m2.expect[m2.expect.len() - 4..], &[0x48, 0x89, 0x43, 0x68]);
        // The patches sit inside the checked context of their functions.
        let inside = |p: &Patch, ctx: u32| {
            let c = M6_CHECKED.iter().find(|s| s.rva == ctx).unwrap();
            let off = (p.site.rva - c.rva) as usize;
            &c.expect[off..off + p.site.expect.len()] == p.site.expect
        };
        assert!(inside(&M6_PATCHES[1], 0x0018_764C));
        assert_eq!(M6_CHECKED.iter().find(|s| s.rva == 0x0017_D016).unwrap().rva, M6_PATCHES[0].site.rva + 6);
        let f = M6_CHECKED.iter().find(|s| s.rva == 0x0017_F3C4).unwrap();
        assert_eq!(f.rva + f.expect.len() as u32, M6_PATCHES[2].site.rva);
    }

    #[test]
    fn m6_deck_sites() {
        // The two stage-2 wrappers and the three log-only readers: hooked by no other stage, and
        // the readers are not the hand-empty / scroll pair confused (same bytes, two addresses).
        let hooked = || HOOKS.iter().chain(MOVE_HOOKS).chain(TRACE_HOOKS).chain(ATTACK_HOOKS).chain(STATS_HOOKS).chain(KEYBLADE_HOOKS).chain(MODEL_HOOKS).chain(M6_HOOKS);
        let called = || CALLEES.iter().chain(MOVE_CALLEES).chain(ATTACK_CALLEES).chain(STATS_CALLEES).chain(KEYBLADE_CALLEES).chain(MODEL_CALLEES).chain(M6_CALLEES);
        for h in M6_DECK_HOOKS.iter().chain(M6_DECK_READERS) {
            assert!(!hooked().any(|s| s.rva == h.rva), "{}", h.name);
            assert!(!called().chain(M6_DECK_CALLEES).any(|s| s.rva == h.rva), "{}", h.name);
        }
        assert_eq!(M6_DECK_HOOKS.iter().map(|h| h.rva).collect::<Vec<_>>(), vec![0x001F_43D0, 0x0017_E190]);
        assert_eq!(M6_DECK_READERS[0].expect, M6_DECK_READERS[1].expect);
        // The drop hook is not the enemy card end the Attack calls (0x14017E1D0, next function).
        assert!(ATTACK_CALLEES.iter().any(|s| s.rva == 0x0017_E1D0));
        assert!(0x0017_E190 + (M6_DECK_HOOKS[1].expect.len() as u32) < 0x0017_E1D0);
        // The allocator the wrapper calls is the one already checked.
        assert!(CALLEES.iter().any(|s| s.rva == 0x004A_CE54));
        // Every retargeted read is Re:CoM's `call 0x1401B08C0` (the checked getter), and sits in
        // its checked context right after `mov edx,0xE2` / ecx = 1.
        let getter = M6_DECK_CALLEES.iter().find(|s| s.rva == 0x001B_08C0).unwrap();
        for c in M6_COCKPIT_CALLS.iter().chain(M6_CARD_CALLS) {
            assert_eq!(c.expect.len(), 5, "{}", c.name);
            assert_eq!(crate::hookplan::call_target(0x1_4000_0000 + c.rva as u64, c.expect), Some(0x1_4000_0000 + getter.rva as u64), "{}", c.name);
            let ctx = M6_DECK_CHECKED.iter().find(|s| s.rva < c.rva && c.rva + 5 <= s.rva + s.expect.len() as u32 && s.expect[0] == 0xBA).unwrap_or_else(|| panic!("{}", c.name));
            let off = (c.rva - ctx.rva) as usize;
            assert_eq!(&ctx.expect[off..off + 5], c.expect, "{}", c.name);
            assert_eq!(&ctx.expect[..5], &[0xBA, 0xE2, 0x00, 0x00, 0x00], "{}", c.name);
        }
        // The cockpit reads are inside 0x1401689F0 (size 0x63B), the card reads inside their three
        // functions.
        assert!(M6_COCKPIT_CALLS.iter().all(|c| (0x0016_89F0..0x0016_89F0 + 0x63B).contains(&c.rva)));
        for (c, (start, size)) in M6_CARD_CALLS.iter().zip([(0x0015_0340u32, 0x223u32), (0x0015_14B0, 0x253), (0x0015_45A0, 0x96)]) {
            assert!((start..start + size).contains(&c.rva), "{}", c.name);
        }
    }

    #[test]
    fn m6_boss_sites() {
        // P5 is the getter's whole body: no other stage hooks, patches or calls it.
        let p5 = &M6_BOSS_PATCHES[0];
        assert_eq!(M6_BOSS_PATCHES.len(), 1);
        let all_hooks = || HOOKS.iter().chain(MOVE_HOOKS).chain(TRACE_HOOKS).chain(ATTACK_HOOKS).chain(STATS_HOOKS).chain(KEYBLADE_HOOKS).chain(MODEL_HOOKS).chain(M6_HOOKS).chain(M6_DECK_HOOKS).chain(M6_DECK_READERS);
        let all_callees = || CALLEES.iter().chain(MOVE_CALLEES).chain(ATTACK_CALLEES).chain(STATS_CALLEES).chain(KEYBLADE_CALLEES).chain(MODEL_CALLEES).chain(M6_CALLEES).chain(M6_DECK_CALLEES);
        let overlaps = |a: &Site, b: &Site| a.rva < b.rva + b.expect.len() as u32 && b.rva < a.rva + a.expect.len() as u32;
        for s in all_hooks().chain(all_callees()).chain(M6_PATCHES.iter().map(|p| &p.site)) {
            assert!(!overlaps(s, &p5.site), "{}", s.name);
        }
        // The duel log hook is no other stage's hook or callee.
        for h in M6_BOSS_HOOKS {
            assert!(!all_hooks().chain(all_callees()).any(|s| s.rva == h.rva), "{}", h.name);
        }
        // Every enemy-UI read is Re:CoM's `call 0x1401B08C0` (the checked getter), inside its
        // checked context right after `mov edx,0xE2` (and ecx = 1), and none is a stage-2 read.
        let getter = M6_DECK_CALLEES.iter().find(|s| s.rva == 0x001B_08C0).unwrap();
        for c in M6_ENEMY_UI_CALLS {
            assert_eq!(c.expect.len(), 5, "{}", c.name);
            assert_eq!(crate::hookplan::call_target(0x1_4000_0000 + c.rva as u64, c.expect), Some(0x1_4000_0000 + getter.rva as u64), "{}", c.name);
            assert!(!M6_COCKPIT_CALLS.iter().chain(M6_CARD_CALLS).any(|s| overlaps(s, c)), "{}", c.name);
            let ctx = M6_ENEMY_UI_CHECKED.iter().find(|s| s.rva < c.rva && c.rva + 5 <= s.rva + s.expect.len() as u32).unwrap_or_else(|| panic!("{}", c.name));
            let off = (c.rva - ctx.rva) as usize;
            assert_eq!(&ctx.expect[off..off + 5], c.expect, "{}", c.name);
            let e = ctx.expect;
            let dx = e.windows(5).position(|w| w == [0xBA, 0xE2, 0x00, 0x00, 0x00]).unwrap_or_else(|| panic!("{}", c.name));
            assert!(dx < off, "{}", c.name);
            // ecx = 1: `mov ecx,1`, or `lea ecx,[r-1]` with r = 2 set in the checked bytes.
            let before = &e[..off];
            assert!(before.ends_with(&[0xB9, 0x01, 0x00, 0x00, 0x00]) || before.ends_with(&[0x8D, 0x4D, 0xFF]) || before.ends_with(&[0x8D, 0x48, 0xFF]), "{}", c.name);
        }
        // The two `lea` forms: ebp = 2 at the stock row's start, eax = 2 in the show's context.
        let row = M6_ENEMY_UI_CHECKED.iter().find(|s| s.rva == 0x0016_9CA0).unwrap();
        assert!(row.expect.windows(5).any(|w| w == [0xBD, 0x02, 0x00, 0x00, 0x00]));
        let show = M6_ENEMY_UI_CHECKED.iter().find(|s| s.rva == 0x0016_E29C).unwrap();
        assert_eq!(&show.expect[..5], &[0xB8, 0x02, 0x00, 0x00, 0x00]);
        // Each read sits in its function: the cue (0x14015E390, 0xB2; CBTL update 0x1401617F0,
        // 0x363), the stock row (0x140169CA0, 0x23A), its show (0x14016DE70, 0x54B), the folder
        // (0x140169070, 0x3D3).
        let funcs = [(0x0015_E390u32, 0xB2u32), (0x0016_17F0, 0x363), (0x0016_9CA0, 0x23A), (0x0016_9CA0, 0x23A), (0x0016_9CA0, 0x23A), (0x0016_DE70, 0x54B), (0x0016_9070, 0x3D3), (0x0016_9070, 0x3D3), (0x0016_9070, 0x3D3)];
        for (c, (start, size)) in M6_ENEMY_UI_CALLS.iter().zip(funcs) {
            assert!((start..start + size).contains(&c.rva), "{}", c.name);
            assert!(M6_ENEMY_UI_CHECKED.iter().any(|s| s.rva == start), "{}", c.name);
        }
    }

    #[test]
    fn m6_enemy_sound_sites() {
        // Every retargeted call is Re:CoM's `call 0x1402AA360` (the checked sound play), sits in
        // its checked context, and is no other stage's retarget.
        let play = CALLEES.iter().find(|s| s.rva == 0x002A_A360).unwrap();
        let overlaps = |a: &Site, b: &Site| a.rva < b.rva + b.expect.len() as u32 && b.rva < a.rva + a.expect.len() as u32;
        assert_eq!(M6_ENEMY_SOUND_CALLS.len(), crate::boss::SOUND_SITES.len());
        for (c, s) in M6_ENEMY_SOUND_CALLS.iter().zip(crate::boss::SOUND_SITES) {
            assert_eq!(c.rva, s.call, "{}", c.name);
            assert_eq!(c.expect.len(), 5, "{}", c.name);
            assert_eq!(crate::hookplan::call_target(0x1_4000_0000 + c.rva as u64, c.expect), Some(0x1_4000_0000 + play.rva as u64), "{}", c.name);
            assert!(!M6_COCKPIT_CALLS.iter().chain(M6_CARD_CALLS).chain(M6_ENEMY_UI_CALLS).chain(KH1_HUD_MASK_CALLS).any(|m| overlaps(m, c)), "{}", c.name);
            let ctx = M6_ENEMY_SOUND_CHECKED.iter().find(|x| x.rva < c.rva && c.rva + 5 <= x.rva + x.expect.len() as u32).unwrap_or_else(|| panic!("{}", c.name));
            let off = (c.rva - ctx.rva) as usize;
            assert_eq!(&ctx.expect[off..off + 5], c.expect, "{}", c.name);
            let before = &ctx.expect[..off];
            // The id the checked bytes load, as `crate::boss::SOUND_SITES` names it.
            match s.id {
                // lea ecx,[rdx+k] after mov edx,-1: k - 1.
                Some(id) if before.starts_with(&[0xBA, 0xFF, 0xFF, 0xFF, 0xFF]) => {
                    let lea = before.windows(3).rposition(|w| w[0] == 0x8D && w[1] == 0x4A).unwrap_or_else(|| panic!("{}", c.name));
                    assert_eq!(before[lea + 2] as u32 - 1, id, "{}", c.name);
                }
                // jne skips it unless edi = 0; lea ecx,[rdi+0x15].
                Some(id) => {
                    assert!(before.starts_with(&[0x85, 0xFF, 0x75]), "{}", c.name);
                    assert!(before.ends_with(&[0x8D, 0x4F, id as u8]), "{}", c.name);
                }
                // mov ecx,[rbx+0x204]: the revolver's deal sound.
                None => assert!(before.windows(6).any(|w| w == [0x8B, 0x8B, 0x04, 0x02, 0x00, 0x00]), "{}", c.name),
            }
            assert!(before.windows(5).any(|w| w == [0xBA, 0xFF, 0xFF, 0xFF, 0xFF]) || before.windows(3).any(|w| w == [0x8D, 0x57, 0xFF]), "{} volume -1", c.name);
        }
        // Each call is inside its function, whose checked start ends with `mov <reg>,rcx`.
        for s in crate::boss::SOUND_SITES {
            let f = M6_ENEMY_SOUND_CHECKED.iter().find(|x| x.rva == s.function).unwrap_or_else(|| panic!("{}", s.what));
            assert!((s.function..s.function + s.size).contains(&s.call), "{}", s.what);
            let mov: &[u8] = match s.reg {
                crate::boss::ObjReg::Rbx => &[0x48, 0x8B, 0xD9],
                crate::boss::ObjReg::Rdi => &[0x48, 0x8B, 0xF9],
            };
            assert!(f.expect.ends_with(mov), "{}", s.what);
        }
        // The page switch: its guard (`jne +0xD`) ends where the play's context starts and skips
        // exactly it (mov edx,-1; lea ecx,[rdx+4]; call = 13 bytes).
        let guard = M6_ENEMY_SOUND_CHECKED.iter().find(|x| x.rva == 0x0016_7A26).unwrap();
        let page = M6_ENEMY_SOUND_CHECKED.iter().find(|x| x.rva == 0x0016_7A2D).unwrap();
        assert_eq!(guard.rva + guard.expect.len() as u32, page.rva);
        assert_eq!(&guard.expect[5..], &[0x75, page.expect.len() as u8]);
        assert_eq!(crate::boss::SOUND_SITES.iter().filter(|s| s.id == Some(crate::boss::sound::PAGE_SWITCH)).count(), 1);
        // The ctor's two deal ids are the ones `crate::boss::sound` names.
        let ctor = |rva: u32| M6_ENEMY_SOUND_CHECKED.iter().find(|x| x.rva == rva).unwrap().expect[6] as u32;
        assert_eq!(ctor(0x0016_4DD6), crate::boss::sound::DEAL_ENEMY);
        assert_eq!(ctor(0x0016_4DF5), crate::boss::sound::DEAL_PLAYER);
    }

    #[test]
    fn mp_bar_sites() {
        // The three wrappers are no other stage's hook or callee, and move whole, position-free
        // instructions (checked with the other hooks below).
        let others = || HOOKS.iter().chain(MOVE_HOOKS).chain(TRACE_HOOKS).chain(ATTACK_HOOKS).chain(STATS_HOOKS).chain(KEYBLADE_HOOKS).chain(MODEL_HOOKS).chain(M6_HOOKS).chain(M6_DECK_HOOKS).chain(M6_DECK_READERS).chain(M6_BOSS_HOOKS);
        let called = || CALLEES.iter().chain(MOVE_CALLEES).chain(ATTACK_CALLEES).chain(STATS_CALLEES).chain(KEYBLADE_CALLEES).chain(MODEL_CALLEES).chain(M6_CALLEES).chain(M6_DECK_CALLEES).chain(MP_BAR_CALLEES);
        for h in MP_BAR_HOOKS {
            assert!(!others().chain(called()).any(|s| s.rva == h.rva), "{}", h.name);
            assert!(h.expect.len() >= 5 && h.expect.len() + 14 <= crate::hookplan::STUB_AT, "{}", h.name);
            assert!(!h.expect.contains(&0xE8) && !h.expect.contains(&0xE9), "{}", h.name);
        }
        // The ctor and allocator the bar also calls are the Keyblades page's checked ones.
        assert!(KEYBLADE_CALLEES.iter().any(|s| s.rva == 0x0024_7260));
        assert!(CALLEES.iter().any(|s| s.rva == 0x004A_CE54));
        // The vtable slots point at checked functions; C2DSeqCtrl's vtable is the ctor's
        // (`lea rax,[0x140607340]` in 0x140247260, checked bytes 0x48 0x8D 0x05 0xD3 0x00 0x3C 0x00).
        for p in MP_BAR_PTRS {
            assert!(all_sites().any(|s| s.rva == p.target), "{}", p.name);
            assert!(!all_sites().any(|s| s.rva == p.rva), "{}", p.name);
        }
        let ctor = KEYBLADE_CALLEES.iter().find(|s| s.rva == 0x0024_7260).unwrap();
        assert_eq!(&ctor.expect[6..9], &[0x48, 0x8D, 0x05]);
        let disp = i32::from_le_bytes(ctor.expect[9..13].try_into().unwrap()) as i64;
        assert_eq!((0x1_4024_7266i64 + 7 + disp) as u64, 0x1_4000_0000 + MP_BAR_PTRS[0].rva as u64);
        assert_eq!(MP_BAR_PTRS[1].rva - MP_BAR_PTRS[0].rva, 8);
        // The bound check sits inside the instantiation, after its checked start.
        assert!(MP_BAR_CHECKED[3].rva + MP_BAR_CHECKED[3].expect.len() as u32 <= MP_BAR_CHECKED[4].rva);
    }

    #[test]
    fn kh1_hud_place_sites() {
        // The replace slot is the C2DSeqCtrl vtable's +0x20 and points at the checked function.
        for p in KH1_HUD_PLACE_PTRS {
            assert!(all_sites().any(|s| s.rva == p.target), "{}", p.name);
            assert!(!all_sites().any(|s| s.rva == p.rva), "{}", p.name);
            assert_eq!(p.rva - MP_BAR_PTRS[0].rva, 0x20);
        }
    }

    #[test]
    fn command_menu_sites() {
        // New callees are no other stage's site; the slot points at the checked add-text.
        for c in COMMAND_MENU_CALLEES.iter().chain(COMMAND_MENU_CHECKED) {
            assert!(c.expect.len() >= 5, "{}", c.name);
            assert_eq!(all_sites().filter(|s| s.rva == c.rva).count(), 1, "{}", c.name);
        }
        for p in COMMAND_MENU_PTRS {
            assert!(COMMAND_MENU_CALLEES.iter().any(|s| s.rva == p.target), "{}", p.name);
            assert!(!all_sites().any(|s| s.rva == p.rva), "{}", p.name);
            // C2DSeqCtrl's vtable (0x140607340) + 0x60.
            assert_eq!(p.rva, MP_BAR_PTRS[0].rva + 0x60);
        }
        // What the menu also calls is checked by earlier stages: operator new, the C2DSeqCtrl
        // ctor / add layer / layout position / raw position / scale / deleting dtor, the C2DFnt
        // ctor, the menu sound, the pad reads, handle → object.
        for rva in [0x004A_CE54u32, 0x0024_7260, 0x0024_7850, 0x0024_8660, 0x0024_85B0, 0x0024_86D0, 0x0015_D230, 0x0024_3F10, 0x002A_A360, 0x0029_B2F0, 0x0029_B410, 0x0029_B490, 0x002A_D350] {
            assert!(all_sites().any(|s| s.rva == rva), "{rva:#x}");
        }
        // The save / warp points: their updates read the main player `[0x14087B380]`, call the
        // handle lookup 0x1402AD350, and the warp's stop flag is `0x14087B84C`.
        let rip = |s: &Site, off: usize, len: usize| {
            let d = i32::from_le_bytes(s.expect[off + len - 4..off + len].try_into().unwrap()) as i64;
            (0x1_4000_0000i64 + s.rva as i64 + off as i64 + len as i64 + d) as u64
        };
        let find = |n: &str| COMMAND_MENU_CHECKED.iter().find(|s| s.name.starts_with(n)).unwrap();
        let save = find("save point update: in range");
        assert_eq!((rip(save, 7, 7), rip(save, 34, 5)), (0x1_4087_B380, 0x1_402A_D350));
        let warp = find("warp point update: gate");
        assert_eq!((rip(warp, 0, 5), rip(warp, 13, 7), rip(warp, 40, 5)), (0x1_4023_45D0, 0x1_4087_B380, 0x1_402A_D350));
        assert_eq!(rip(find("warp point stop flag"), 0, 6), 0x1_4087_B84C);
        assert_eq!(find("save point type getter").expect[1..5], crate::command_menu::recom::kind::SAVE_POINT.to_le_bytes());
        assert_eq!(find("warp point type getter").expect[1..5], crate::command_menu::recom::kind::WARP_POINT.to_le_bytes());
        // Stage 7.12: both retargeted calls go to the checked Triangle read, and are the bytes the
        // context sites hold at +4; the confirm key getter reads 0x14083F3F0; the held-pad read
        // the shortcut list uses is the movement's checked site.
        for (c, ctx) in COMMAND_MENU_POINT_CALLS.iter().zip(["save point update: Triangle read", "warp point update: Triangle read"]) {
            let ctx = find(ctx);
            assert_eq!(crate::hookplan::call_target(0x1_4000_0000 + c.rva as u64, c.expect), Some(0x1_4029_B550), "{}", c.name);
            assert_eq!((ctx.rva + 4, &ctx.expect[4..9]), (c.rva, c.expect), "{}", c.name);
            assert_eq!(all_sites().filter(|s| s.rva == c.rva).count(), 1, "{}", c.name);
        }
        assert!(all_sites().any(|s| s.rva == 0x0029_B550) && all_sites().any(|s| s.rva == 0x0029_C1D0));
        assert_eq!(rip(find("confirm key getter"), 0, 7), 0x1_4083_F3F0);
    }

    #[test]
    fn magic_field_fx_sites() {
        use crate::magic::field_fx as fx;
        let find = |rva: u32| all_sites().find(|s| s.rva == rva).unwrap_or_else(|| panic!("{rva:#x}"));
        for s in MAGIC_FX_BRANCHES.iter().chain(MAGIC_FX_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        // The field state's tail jump goes to the player's group load (the models' hooked site,
        // still called first); the camp openings' calls go to the battle set's unload.
        let jump = find(fx::FIELD_JUMP_RVA);
        assert_eq!(jump.expect[0], 0xE9);
        assert_eq!(fx::branch_target(jump.rva, jump.expect), Some(fx::PLAYER_GROUP_RVA));
        assert!(MODEL_HOOKS.iter().any(|s| s.rva == fx::PLAYER_GROUP_RVA));
        for rva in fx::CAMP_CALL_RVAS {
            let c = find(rva);
            assert_eq!(c.expect[0], 0xE8, "{}", c.name);
            assert_eq!(fx::branch_target(c.rva, c.expect), Some(fx::UNLOAD_RVA), "{}", c.name);
        }
        // Each checked context ends where its branch starts.
        let ctx_end = |name: &str| {
            let s = MAGIC_FX_CHECKED.iter().find(|s| s.name.starts_with(name)).unwrap();
            s.rva + s.expect.len() as u32
        };
        assert_eq!(ctx_end("field state 1"), fx::FIELD_JUMP_RVA);
        assert_eq!(ctx_end("camp open: system byte"), fx::CAMP_CALL_RVAS[0]);
        assert_eq!(ctx_end("camp open (menu button): system byte"), fx::CAMP_CALL_RVAS[1]);
        // Both tests call the system byte getter 0x1401B0800 with (0, 0x60).
        let t0 = MAGIC_FX_CHECKED.iter().find(|s| s.name.starts_with("camp open: system byte")).unwrap();
        let t1 = MAGIC_FX_CHECKED.iter().find(|s| s.name.starts_with("camp open (menu button): system byte")).unwrap();
        assert_eq!(fx::branch_target(t0.rva + 7, &t0.expect[7..12]), Some(0x001B_0800));
        assert_eq!(fx::branch_target(t1.rva + 5, &t1.expect[5..10]), Some(0x001B_0800));
        // Re:CoM's battle state 1 is the field's state 1 with the battle set's load as its tail.
        let battle = MAGIC_FX_CHECKED.iter().find(|s| s.name.starts_with("battle state 1")).unwrap();
        let field = MAGIC_FX_CHECKED.iter().find(|s| s.name.starts_with("field state 1")).unwrap();
        assert_eq!(fx::branch_target(battle.rva + 19, &battle.expect[19..24]), Some(fx::LOAD_RVA));
        assert_eq!(battle.expect[9..19], field.expect[9..19]);
        // The loader and unloader are checked.
        find(fx::LOAD_RVA);
        find(fx::UNLOAD_RVA);
    }

    #[test]
    fn customize_glyph_sites() {
        for s in CUSTOMIZE_GLYPHS_CHECKED {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        let find = |n: &str| CUSTOMIZE_GLYPHS_CHECKED.iter().find(|s| s.name.starts_with(n)).unwrap();
        // The table's entries are the four cases' RVAs (escapes 0xF564, 0xF565, 0xF566, 0xF567,
        // 0xF568; 0xF565 is cancel, not used), and each case reads its glyph word.
        let table = find("text escape table");
        let entry = |k: usize| u32::from_le_bytes(table.expect[4 * k..4 * k + 4].try_into().unwrap());
        let rip = |s: &Site| {
            let d = i32::from_le_bytes(s.expect[3..7].try_into().unwrap()) as i64;
            (0x1_4000_0000i64 + s.rva as i64 + 7 + d) as u64
        };
        for (k, case, word) in [(0, "escape 0xF564", 0x1_4083_F510u64), (2, "escape 0xF566", 0x1_4083_F518), (3, "escape 0xF567", 0x1_4083_F51A), (4, "escape 0xF568", 0x1_4083_F51C)] {
            let c = find(case);
            assert_eq!(entry(k), c.rva, "{case}");
            assert_eq!(rip(c), word, "{case}");
        }
        // The page's escapes are those codes.
        use crate::customize::text::glyph;
        for (g, k) in [(glyph::CONFIRM, 0), (glyph::SQUARE, 2), (glyph::TRIANGLE, 3), (glyph::L1, 4)] {
            assert_eq!(u16::from_be_bytes(g), 0xF564 + k);
        }
        // The draw's font table: count 0x14087CFB8, base 0x14087CFC0, 0x158 a font.
        let draw = find("C2DFnt draw");
        let d = i32::from_le_bytes(draw.expect[2..6].try_into().unwrap()) as i64;
        assert_eq!((0x1_4000_0000i64 + draw.rva as i64 + 6 + d) as u64, 0x1_4087_CFB8);
        assert_eq!(u32::from_le_bytes(draw.expect[32..36].try_into().unwrap()), 0x0087_CFC0);
        assert_eq!(u32::from_le_bytes(draw.expect[39..43].try_into().unwrap()), 0x158);
        // The button flag is +0x150 in both the set-up and the picker.
        assert_eq!(u32::from_le_bytes(find("font set-up").expect[3..7].try_into().unwrap()), 0x150);
        assert_eq!(u32::from_le_bytes(find("glyph picker").expect[2..6].try_into().unwrap()), 0x150);
    }

    #[test]
    fn hud_2d_sites() {
        for c in HUD_2D_CALLEES.iter().chain(HUD_2D_CHECKED) {
            assert!(c.expect.len() >= 5, "{}", c.name);
            assert_eq!(all_sites().filter(|s| s.rva == c.rva).count(), 1, "{}", c.name);
        }
        // A rip-relative operand's target: the instruction at `at` (bytes `off..off + len` of
        // the site), its disp32 in its last 4 bytes.
        let rip = |s: &Site, off: usize, len: usize| {
            let d = i32::from_le_bytes(s.expect[off + len - 4..off + len].try_into().unwrap()) as i64;
            (0x1_4000_0000i64 + s.rva as i64 + off as i64 + len as i64 + d) as u64
        };
        let find = |n: &str| HUD_2D_CALLEES.iter().chain(HUD_2D_CHECKED).find(|s| s.name.starts_with(n)).unwrap();
        // The setter writes the index `0x140820F94` and the current list `0x14087CE70` from the
        // lists `0x14087CE80` (`win::d2` reads the index).
        let set = find("2D draw list set");
        assert_eq!(rip(set, 7, 7), 0x1_4087_CE80);
        assert_eq!(rip(set, 14, 6), 0x1_4082_0F94);
        assert_eq!(rip(set, 24, 7), 0x1_4087_CE70);
        // Register and release both use the current list.
        assert_eq!(rip(find("2D instance register"), 31, 7), 0x1_4087_CE70);
        let rel = find("2D instance release");
        assert_eq!(rip(rel, 24, 7), 0x1_4087_CE70);
        assert_eq!(rip(rel, 31, 5), 0x1_4024_9E40);
        assert_eq!(find("2D list release walk").rva, 0x0024_9E40);
        // The pool object `[0x14087CE58]`; its allocation reaches the free count's function.
        assert_eq!(rip(find("2D pool getter"), 0, 7), 0x1_4087_CE58);
        assert_eq!(rip(find("2D pool allocate"), 4, 5), 0x1_4024_2560);
        assert_eq!(find("pool allocate: free count").rva, 0x0024_2560);
        // 0x800 entries, made by 0x1402425E0.
        let ctor = find("2D manager ctor");
        assert_eq!(u32::from_le_bytes(ctor.expect[6..10].try_into().unwrap()), crate::d2::POOL_ENTRIES as u32);
        assert_eq!(rip(ctor, 14, 5), 0x1_4024_25E0);
        // The clear, delete and add layer win::d2 also calls are the MP bar's checked ones.
        for rva in [0x0024_7C90u32, 0x0015_D230, 0x0024_7850, 0x0024_76F0] {
            assert!(all_sites().any(|s| s.rva == rva), "{rva:#x}");
        }
    }

    #[test]
    fn ability_sites() {
        for s in ABILITY_CALLEES {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        // Its call at +9 goes to Re:CoM's story test 0x1401AAB00.
        let f = &ABILITY_CALLEES[0];
        let rel = i32::from_le_bytes(f.expect[10..14].try_into().unwrap()) as i64;
        assert_eq!(0x1_4000_0000 + f.rva as i64 + 9 + 5 + rel, 0x1_401A_AB00);
    }

    #[test]
    fn m12_sites() {
        for s in M12_CALLEES.iter().chain(M12_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        let find = |n: &str| M12_CALLEES.iter().chain(M12_CHECKED).find(|s| s.name.starts_with(n)).unwrap();
        let rip = |s: &Site, len: usize| (0x1_4000_0000i64 + s.rva as i64 + len as i64 + i32::from_le_bytes(s.expect[3..7].try_into().unwrap()) as i64) as u64;
        // Both CP sites read Sora's stats pointer; the cap is 1625, the pick +25.
        assert_eq!(rip(find("level-up CP offer"), 7), 0x1_4087_CBF8);
        assert_eq!(rip(find("level-up CP pick"), 7), 0x1_4087_CBF8);
        assert_eq!(i32::from_le_bytes(find("level-up CP offer").expect[10..14].try_into().unwrap()), crate::level_up::RECOM_CP_OFFER_CAP);
        assert_eq!(find("level-up CP pick").expect[10] as i32, crate::abilities::RECOM_CP_PER_UP);
        assert_eq!(find("learned line ability wording").expect[6] as u16, crate::level_up::ABILITY_WORDING_ID);
        assert_eq!(u32::from_le_bytes(find("field menu gate").expect[8..12].try_into().unwrap()), 0x5A4);
        // The vtables as relocated pointers (raw bytes would be relocated by the loader).
        assert_eq!(0x1_4000_0000 + M12_PTRS[0].rva as u64, crate::level_up::tech_help::VTABLE);
        assert_eq!((M12_PTRS[0].target, M12_PTRS[1].target, M12_PTRS[1].rva - M12_PTRS[0].rva), (0x0012_C230, 0x0012_D330, 8));
        assert_eq!((M12_PTRS[3].target, M12_PTRS[3].rva - M12_PTRS[2].rva), (0x0010_EF10, 8));
        assert!(all_sites().all(|s| !s.name.contains("vtable (0x14051B4A8)") && !s.name.contains("vtable (0x140512C10)")));
        assert!(all_sites().any(|s| s.rva == 0x0010_EF10), "the update's wrapper site");
        // The panel's preview: ours prints the same bytes between the numbers.
        let f = find("level-up panel preview format").expect;
        assert_eq!(&crate::level_up::panel_value(1, true)[4..11], &f[3..10]);
        // The callees win::notices / win::level_up also use are earlier stages' checked ones.
        for rva in [0x004A_CE54u32, 0x002A_D300, 0x0025_8DE0, 0x001B_0870] {
            assert!(all_sites().any(|s| s.rva == rva), "{rva:#x}");
        }
    }

    #[test]
    fn item_page_sites() {
        // The stats copy is no other stage's site and reads the player pointer the heal uses
        // (`mov rax,[rip+rel]` at +4, 7 bytes).
        for s in ITEM_PAGE_CALLEES {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        let sync = &ITEM_PAGE_CALLEES[0];
        let rel = i32::from_le_bytes(sync.expect[7..11].try_into().unwrap()) as i64;
        assert_eq!(0x1_4000_0000 + sync.rva as i64 + 4 + 7 + rel, 0x1_4087_B390);
        assert!(all_sites().any(|s| s.rva == 0x001E_59A0), "the heal is checked");
    }

    /// M9 stage 9c.1: the specials' sites agree with `crate::specials` and each other. (The Field
    /// exe's action-0x16 entry `0x1401FA648` and the cave `0x1401FB7B2` are the field sleights' now:
    /// `field_sleight_sites`.)
    #[test]
    fn special_sites() {
        assert_eq!(SPECIAL_HOOKS[0].rva, 0x0021_8950);
        assert_eq!(SPECIAL_PTRS[0].target, SPECIAL_HOOKS[0].rva);
        // M8b: the sleight start slot sits 0x10 before exeStockSkill's (+0x130 / +0x140), and the
        // card-info rows are the summon cards' (0x1405256D0 + kind × 16).
        assert_eq!(SUMMON_PTRS[0].rva + 0x10, SPECIAL_PTRS[0].rva);
        assert_eq!(0x1_4000_0000u64 + SUMMON_PTRS[0].rva as u64, data::SORA_VTABLE + 0x130);
        assert_eq!(SUMMON_PTRS[0].target, SUMMON_CHECKED[0].rva);
        for rva in SUMMON_ALSO {
            assert!(all_sites().any(|c| c.rva == rva), "{rva:#x}");
        }
        for s in crate::summons::ALL {
            let rva = 0x0052_56D0 + s.card as u32 * 16;
            assert!(SUMMON_CHECKED.iter().any(|c| c.rva == rva && c.expect[0] == 0x33), "{}", s.name);
        }
        assert_eq!(0x1_4000_0000u64 + SPECIAL_PTRS[0].rva as u64, data::SORA_VTABLE + 0x140);
        // Only the field sleights' own sites touch the Field exe's action-0x16 entry and its cave.
        assert!(
            all_sites().filter(|s| (0x001F_A648..0x001F_A64C).contains(&s.rva) || (0x001F_B7B2..0x001F_B7C0).contains(&s.rva)).all(|s| FIELD_SLEIGHT_DATA.iter().any(|d| d.rva == s.rva && d.name == s.name)),
            "the Field exe's 0x16 entry and cave are the field sleights'"
        );
        // Re:CoM's rows 0xE..=0x10: their sleight motions are `recom_events::MOTION`, the start's
        // EET variant 0, all three sleight-pack motions (flag 0x40).
        let rows = SPECIAL_CHECKED.iter().find(|s| s.name.starts_with("Re:CoM rows 0xE")).unwrap();
        assert_eq!(0x1_4000_0000u64 + rows.rva as u64, crate::attack::RECOM_ATTACK_ROWS + 0xE * 16);
        for (i, r) in rows.expect.chunks(16).enumerate() {
            assert_eq!(r[0] as u32, crate::specials::recom_events::MOTION[i]);
            assert!(r[15] & 0x40 != 0);
        }
        assert_eq!(rows.expect[1] as u32, crate::specials::recom_events::EET_VARIANT);
        // initStockSkill sets action 0x16; the Battle case calls vtable +0x140.
        let init = SPECIAL_CHECKED.iter().find(|s| s.name.starts_with("initStockSkill")).unwrap();
        assert_eq!(&init.expect[init.expect.len() - 3..], &[0xBA, crate::specials::RECOM_ACTION as u8, 0x00]);
        let case = SPECIAL_CHECKED.iter().find(|s| s.name.starts_with("Battle exe case 0x16")).unwrap();
        assert_eq!(u32::from_le_bytes(case.expect[8..12].try_into().unwrap()), 0x140);
    }

    /// The field rules' two call sites reach the Re:CoM callees our stubs call.
    #[test]
    fn field_rule_sites() {
        for s in FIELD_RULE_CALLS.iter().chain(FIELD_RULE_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        let to = |s: &Site| (s.rva as i64 + 5 + i32::from_le_bytes(s.expect[1..5].try_into().unwrap()) as i64) as u32;
        assert_eq!(to(&FIELD_RULE_CALLS[0]), 0x001A_AA90, "the menu gate");
        assert_eq!(to(&FIELD_RULE_CALLS[1]), 0x0019_BED0, "the contact encounter");
        assert!(all_sites().any(|c| c.rva == 0x001A_AA90), "the menu gate is checked");
        // Each call sits between its checked context.
        assert_eq!(FIELD_RULE_CHECKED[0].rva + 7, FIELD_RULE_CALLS[0].rva);
        assert_eq!(FIELD_RULE_CALLS[0].rva + 5, FIELD_RULE_CHECKED[1].rva);
        assert_eq!(FIELD_RULE_CHECKED[2].rva + FIELD_RULE_CHECKED[2].expect.len() as u32, FIELD_RULE_CALLS[1].rva);
        assert_eq!(FIELD_RULE_CALLS[1].rva + 5, FIELD_RULE_CHECKED[3].rva);
        assert_eq!(FIELD_RULE_CHECKED[4].rva, to(&FIELD_RULE_CALLS[1]));
    }

    /// Enemy Skills stage 2: the clear's hook, the callees, Re:CoM's apply bytes and the switched
    /// sites (ours differ only in their constant; no other site's bytes overlap one, since they are
    /// rewritten at run time).
    #[test]
    fn enemy_skill_sites() {
        use crate::skill_effects::{imm32, numbers};
        for s in ENEMY_SKILL_HOOKS.iter().chain(ENEMY_SKILL_CALLEES).chain(ENEMY_SKILL_CHECKED).chain(ENEMY_SKILL_STUB_SITES) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        // The clear's 15 bytes: whole instructions ending in `sub rsp,0x20`.
        assert!(ENEMY_SKILL_HOOKS[0].expect.ends_with(&[0x57, 0x48, 0x83, 0xEC, 0x20]));
        // Re:CoM's apply: the bits and Draw's floats route 1 writes.
        let a = &ENEMY_SKILL_CHECKED[0].expect;
        assert_eq!(u32::from_le_bytes(a[6..10].try_into().unwrap()), crate::skill_effects::flag420::BACK_ATTACK);
        assert_eq!(u32::from_le_bytes(a[21..25].try_into().unwrap()), crate::skill_effects::flag420::WIDE);
        assert_eq!(u32::from_le_bytes(a[36..40].try_into().unwrap()), crate::skill_effects::flag420::HASTE);
        let d = &ENEMY_SKILL_CHECKED[1].expect;
        assert_eq!(u16::from_le_bytes(d[2..4].try_into().unwrap()) as usize, crate::skill_effects::draw::FIELD_54C);
        assert_eq!(f32::from_le_bytes(d[6..10].try_into().unwrap()), crate::skill_effects::draw::ON_54C);
        assert_eq!(u16::from_le_bytes(d[12..14].try_into().unwrap()) as usize, crate::skill_effects::draw::RADIUS);
        assert_eq!(f32::from_le_bytes(d[16..20].try_into().unwrap()), crate::skill_effects::draw::ON);
        for (i, bit) in [(2, crate::skill_effects::flag420::QUICK_BARRIER), (3, crate::skill_effects::flag420::COMBO_BOOST), (4, crate::skill_effects::flag420::DOUBLE_STRIKE)] {
            assert_eq!(u32::from_le_bytes(ENEMY_SKILL_CHECKED[i].expect[6..10].try_into().unwrap()), bit);
        }
        // The Large Body setter writes π/2 into the barrier's half-angle.
        assert!(ENEMY_SKILL_CALLEES[4].expect.ends_with(&[0xC7, 0x81, 0x3C, 0x07, 0x00, 0x00, 0xDB, 0x0F, 0xC9, 0x3F, 0xC3]));
        // The switched sites: Re:CoM's constant → the owner's.
        let p = ENEMY_SKILL_PATCHES;
        assert_eq!((&p[0].site.expect[6..], &p[0].new[6..]), (&imm32(6000)[..], &imm32(numbers::REGEN_TICKS)[..]));
        assert_eq!((&p[1].site.expect[6..], &p[1].new[6..]), (&imm32(6000)[..], &imm32(numbers::REGEN_TICKS)[..]));
        assert_eq!((&p[2].site.expect[6..], &p[2].new[6..]), (&imm32(1000)[..], &imm32(numbers::BIO_TICKS)[..]));
        assert_eq!((&p[3].site.expect[6..], &p[3].new[6..]), (&imm32(6000)[..], &imm32(numbers::BARRIER_TICKS)[..]));
        for q in p {
            assert_eq!(q.site.expect.len(), q.new.len(), "{}", q.site.name);
            assert_eq!(q.site.expect[..q.site.expect.len() - 4], q.new[..q.new.len() - 4], "{}: only the constant changes", q.site.name);
        }
        // Guard Armor's scale: Re:CoM's 1.0 at 0x1406C10C0 → the pool's 0.75 (a checked site).
        let w = &p[4];
        let next = 0x1_4000_0000i64 + w.site.rva as i64 + 8;
        assert_eq!(next + i32::from_le_bytes(w.site.expect[4..8].try_into().unwrap()) as i64, 0x1_406C_10C0);
        let ours = next + i32::from_le_bytes(w.new[4..8].try_into().unwrap()) as i64;
        assert_eq!(ours, 0x1_4000_0000 + ENEMY_SKILL_CHECKED[5].rva as i64);
        assert_eq!(f32::from_le_bytes(ENEMY_SKILL_CHECKED[5].expect[..4].try_into().unwrap()), numbers::WIDE_SCALE);
        // No other site's bytes overlap a site rewritten at run time.
        let switched: Vec<&Site> = p.iter().map(|q| &q.site).chain(ENEMY_SKILL_STUB_SITES).chain(ENEMY_SKILL_REWRITE_SITES).chain(ENEMY_SKILL_REWRITE_HOOK).collect();
        for x in &switched {
            let (a0, a1) = (x.rva, x.rva + x.expect.len() as u32);
            for s in all_sites().filter(|s| s.rva != x.rva) {
                let (b0, b1) = (s.rva, s.rva + s.expect.len() as u32);
                assert!(a1 <= b0 || b1 <= a0, "{} overlaps {}", x.name, s.name);
            }
        }
    }

    /// Enemy Skills stage 3: the rewrite's hook, its stub sites and where the stubs go back; the stubs'
    /// targets are those sites (`crate::skill_effects::rewrite`).
    #[test]
    fn enemy_skill_rewrite_sites() {
        use crate::skill_effects::rewrite as rw;
        for s in ENEMY_SKILL_REWRITE_HOOK.iter().chain(ENEMY_SKILL_REWRITE_SITES).chain(ENEMY_SKILL_REWRITE_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        let va = |s: &Site| 0x1_4000_0000u64 + s.rva as u64;
        let (z, l, dm, df, du) = (&ENEMY_SKILL_REWRITE_SITES[0], &ENEMY_SKILL_REWRITE_SITES[1], &ENEMY_SKILL_REWRITE_SITES[2], &ENEMY_SKILL_REWRITE_SITES[3], &ENEMY_SKILL_REWRITE_SITES[4]);
        // The rolls' constants are Re:CoM's cells; their jge goes to the return; the stubs come back
        // right after each site.
        assert_eq!(z.expect[3] as i32, rw::RECOM_CELLS[0]);
        assert_eq!(l.expect[3] as i32, rw::RECOM_CELLS[1]);
        for s in [z, l] {
            assert_eq!(va(s) + 10 + i32::from_le_bytes(s.expect[6..10].try_into().unwrap()) as u64, rw::SKIP);
        }
        assert_eq!(va(z) + 10, rw::RET_ZEXION);
        assert_eq!(va(l) + 10, rw::RET_LEXAEUS);
        assert_eq!(va(dm) + 11, rw::SKIP, "Demyx falls through to the return");
        assert_eq!(va(df) + 9, rw::RET_DEFENDER);
        assert_eq!(va(du) + 9, rw::RET_URSULA);
        let c = ENEMY_SKILL_REWRITE_CHECKED;
        assert_eq!((va(&c[0]), va(&c[1]), va(&c[2]), va(&c[3]), va(&c[4]), va(&c[5])), (rw::RET_ZEXION, rw::RET_LEXAEUS, rw::RET_DEFENDER, rw::RET_URSULA, rw::DIV100_TAIL, rw::SKIP));
        assert_eq!(c[0].expect[3] as u32, rw::CONFUSE);
        assert_eq!(c[1].expect[3] as u32, rw::WARP);
    }

    /// Lucky Bounty at KH1: Pluto's find, its outcome switch and the calls it makes.
    #[test]
    fn pluto_sites() {
        for s in PLUTO_HOOKS.iter().chain(PLUTO_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        // The outcome read is the field `crate::party::lucky_bounty` tests, at `+0x472`.
        assert_eq!(&PLUTO_CHECKED[0].expect[4..6], &[0x72, 0x04]);
        let d = &PLUTO_CHECKED[1];
        let call = (d.rva as i64 + 26 + 5 + i32::from_le_bytes(d.expect[27..31].try_into().unwrap()) as i64) as u32;
        assert_eq!(call, 0x0016_6930, "the deck's deal count");
        let e = &PLUTO_CHECKED[2];
        let bone = (e.rva as i64 + 37 + 5 + i32::from_le_bytes(e.expect[38..42].try_into().unwrap()) as i64) as u32;
        assert_eq!(bone, 0x002A_BB20);
        // The callees the wrapper uses are checked elsewhere.
        for rva in [0x002A_BB20u32, 0x001A_6B20, 0x001A_89C0, 0x0029_CF00, 0x0023_37C0] {
            assert!(all_sites().any(|c| c.rva == rva), "{rva:#x}");
        }
    }

    /// A Gravity-flattened enemy stays hittable: the two wrappers' sites and the calls they rely on.
    #[test]
    fn flatten_sites() {
        for s in FLATTEN_HOOKS.iter().chain(FLATTEN_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        // The checked rest starts right after the moved bytes; the calls go to Re:CoM's body switches.
        for (h, c) in FLATTEN_HOOKS.iter().zip(FLATTEN_CHECKED) {
            assert_eq!(h.rva + h.expect.len() as u32, c.rva, "{}", h.name);
        }
        let call = |s: &Site, at: usize| (s.rva as i64 + at as i64 + 5 + i32::from_le_bytes(s.expect[at + 1..at + 5].try_into().unwrap()) as i64) as u32;
        assert_eq!(call(&FLATTEN_CHECKED[0], 23), 0x001A_2D70, "hurt bodies off");
        assert_eq!(call(&FLATTEN_CHECKED[0], 36), 0x001A_2D90, "attack bodies off");
        assert_eq!(call(&FLATTEN_CHECKED[1], 3), 0x001A_2DB0, "hurt bodies on");
        assert_eq!(call(&FLATTEN_CHECKED[1], 16), 0x001A_2DD0, "attack bodies on");
        assert_eq!(crate::magic::flatten::HURT_ON_RVA, 0x001A_2DB0);
        // KH1's own hit flatten: the exe's hook and its checked rest, the fields and calls it relies on.
        for s in FLATTEN_EXE_HOOK.iter().chain(FLATTEN_EXE_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        let (h, c) = (&FLATTEN_EXE_HOOK[0], &FLATTEN_EXE_CHECKED[0]);
        assert_eq!(h.rva + h.expect.len() as u32, c.rva);
        let e = c.expect;
        // mov ecx,[rcx+0x270] (the phase) at exe +0x18.
        assert_eq!(&e[0x12..0x18], &[0x8B, 0x89, 0x70, 0x02, 0x00, 0x00]);
        // mov [rbx+0x160],9000 and inc [rbx+0x270] near its end (exe +0x15B / +0x165).
        assert_eq!(&e[0x15B - 6..0x15B - 6 + 10], &[0xC7, 0x83, 0x60, 0x01, 0x00, 0x00, 0x28, 0x23, 0x00, 0x00]);
        assert_eq!(&e[0x165 - 6..0x165 - 6 + 6], &[0xFF, 0x83, 0x70, 0x02, 0x00, 0x00]);
        let call = |at: usize| (c.rva as i64 + at as i64 + 5 + i32::from_le_bytes(e[at + 1..at + 5].try_into().unwrap()) as i64) as u32;
        assert_eq!(call(0x14D - 6), 0x001A_7050, "the squash's height setter");
        assert_eq!(call(0x37 - 6), 0x001A_57D0, "the get-up motion's end");
        assert_eq!(crate::magic::flatten::kh1::STATE, 0xAC0);
        assert_eq!(crate::guard::func::PICKER, 0x0017_D260);
    }

    /// Sleights outside battle (owner, 2026-11-04): the field case's entry and stub agree with
    /// `crate::field_sleights`, its stub goes where the Battle exe's case calls, every site is its
    /// own, and the other lists' sites it relies on are checked.
    #[test]
    fn field_sleight_sites() {
        use crate::field_sleights::{field_case as fc, lists, packs};
        for s in FIELD_SLEIGHT_HOOKS.iter().chain(FIELD_SLEIGHT_DATA).chain(FIELD_SLEIGHT_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        for rva in FIELD_SLEIGHT_ALSO {
            assert!(all_sites().any(|c| c.rva == rva), "{rva:#x}");
        }
        assert_eq!(FIELD_SLEIGHT_HOOKS[0].rva, packs::PLM_INIT_RVA);
        assert!(FIELD_SLEIGHT_HOOKS[0].expect.len() >= 5);
        assert_eq!(FIELD_SLEIGHT_DATA[0].rva, fc::TABLE_ENTRY_RVA);
        assert_eq!(u32::from_le_bytes(FIELD_SLEIGHT_DATA[0].expect[..4].try_into().unwrap()), fc::DEFAULT_RVA);
        assert_eq!(FIELD_SLEIGHT_DATA[1].rva, fc::CAVE_RVA);
        assert_eq!(FIELD_SLEIGHT_DATA[1].expect.len(), fc::CAVE_LEN);
        // The Field exe's switch table base (`MAGIC_CHECKED`'s switch) + (0x16 − 1) × 4.
        let sw = MAGIC_CHECKED[0].expect;
        let base = u32::from_le_bytes(sw[30..34].try_into().unwrap());
        assert_eq!(base + (fc::ACTION - 1) * 4, fc::TABLE_ENTRY_RVA);
        // The stub's call is exeStockSkill, the function the Battle exe's case reaches through the
        // vtable slot (`SPECIAL_PTRS`) and the specials hook.
        assert_eq!(fc::EXE_STOCK_RVA, SPECIAL_PTRS[0].target);
        assert_eq!(fc::EXE_STOCK_RVA, SPECIAL_HOOKS[0].rva);
        // The lookups in the ctor's bytes call the two functions we call.
        let l = FIELD_SLEIGHT_CHECKED.iter().find(|s| s.name.starts_with("CCardBattle list lookups")).unwrap();
        let call = |at: usize| (l.rva as i64 + at as i64 + 5 + i32::from_le_bytes(l.expect[at + 1..at + 5].try_into().unwrap()) as i64) as u32;
        assert_eq!(call(7), lists::LOOKUP_RVA);
        assert_eq!(call(20), lists::DATA_RVA);
        // The field's ring allocation calls the world getter, the room test and the allocator.
        let a = FIELD_SLEIGHT_CHECKED.iter().find(|s| s.name.starts_with("field ring allocation")).unwrap();
        let call = |at: usize| (a.rva as i64 + at as i64 + 5 + i32::from_le_bytes(a.expect[at + 1..at + 5].try_into().unwrap()) as i64) as u32;
        assert_eq!((call(5), call(15), call(26)), (packs::WORLD_RVA, packs::ROOM_2000_RVA, 0x0021_0BD0));
        assert_eq!(a.expect[12], packs::NO_RING_WORLD as u8);
        // The battle branch's writes are the PLM fields the wrapper writes.
        let w = FIELD_SLEIGHT_CHECKED.iter().find(|s| s.name.starts_with("PLM init ring writes")).unwrap();
        assert_eq!((w.expect[3] as usize, w.expect[23] as usize, w.expect[35] as usize), (packs::CURSOR, packs::END, packs::SPACE));
        // A rip-relative operand: its displacement at `disp`, the instruction `len` long from `at`.
        let rip = |at: usize, disp: usize, len: usize| 0x1_4000_0000u64 + (w.rva as i64 + at as i64 + len as i64 + i32::from_le_bytes(w.expect[disp..disp + 4].try_into().unwrap()) as i64) as u64;
        assert_eq!(rip(7, 9, 6), packs::RING_SIZE);
        assert_eq!(rip(13, 16, 7), packs::RING);
        // The dtor's fade: mask 0x1808, Re:CoM's colour fade.
        let d = FIELD_SLEIGHT_CHECKED.iter().find(|s| s.name.starts_with("player dtor dim exit")).unwrap();
        assert_eq!(u32::from_le_bytes(d.expect[48..52].try_into().unwrap()), 0x1808);
        assert_eq!((d.rva as i64 + 52 + 5 + i32::from_le_bytes(d.expect[53..57].try_into().unwrap()) as i64) as u32, 0x0023_2170);
    }

    #[test]
    fn item_sites() {
        // Each item site is no other stage's; the hook is the function the Battle exe's case calls;
        // the Field case's table entry and stub agree with `crate::items::field_case`.
        for s in ITEM_HOOKS.iter().chain(ITEM_DATA).chain(ITEM_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        use crate::items::field_case as fc;
        assert_eq!(ITEM_HOOKS[0].rva, fc::EXE_ITEM_RVA);
        let call = ITEM_CHECKED.iter().find(|s| s.name.starts_with("Battle exe case 0xE")).unwrap();
        let rel = i32::from_le_bytes(call.expect[4..8].try_into().unwrap()) as i64;
        assert_eq!((call.rva as i64 + 3 + 5 + rel) as u32, fc::EXE_ITEM_RVA);
        assert_eq!(ITEM_DATA[0].rva, fc::TABLE_ENTRY_RVA);
        assert_eq!(u32::from_le_bytes(ITEM_DATA[0].expect[..4].try_into().unwrap()), fc::DEFAULT_RVA);
        assert_eq!(ITEM_DATA[1].rva, fc::CAVE_RVA);
        assert!(fc::STUB_LEN <= ITEM_DATA[1].expect.len());
        // Clear of the other caves (the Attack's, magic's).
        for other in [crate::attack::battle_case::CAVE_RVA, crate::magic::field_case::CAVE_RVA] {
            assert!(fc::CAVE_RVA + fc::STUB_LEN as u32 <= other || other + 15 <= fc::CAVE_RVA);
        }
        // The same switch as magic's case: the base is the table's.
        let sw = MAGIC_CHECKED[0].expect;
        let base = u32::from_le_bytes(sw[30..34].try_into().unwrap());
        assert_eq!(base + (crate::items::RECOM_ITEM_ACTION - 1) * 4, fc::TABLE_ENTRY_RVA);
        // exeItem's effect call goes to the effect-on-bone function the magic cast calls, and the
        // object getter; the frame check is 56 (0x38).
        let ec = ITEM_CHECKED.iter().find(|s| s.name.starts_with("exeItem effect call")).unwrap();
        let rel = i32::from_le_bytes(ec.expect[31..35].try_into().unwrap()) as i64;
        assert_eq!((ec.rva as i64 + 30 + 5 + rel) as u32, 0x001A_2D50);
        let rel = i32::from_le_bytes(ec.expect[40..44].try_into().unwrap()) as i64;
        assert_eq!((ec.rva as i64 + 39 + 5 + rel) as u32, 0x0025_7EF0);
        let f56 = ITEM_CHECKED.iter().find(|s| s.name.starts_with("exeItem frame 56")).unwrap();
        assert_eq!(f56.expect[2], 0x38);
        // The reload calls 0x14016C910 (the card effect a KH1 item never reaches).
        let rl = ITEM_CHECKED.iter().find(|s| s.name.starts_with("exeItem card reload")).unwrap();
        let rel = i32::from_le_bytes(rl.expect[34..38].try_into().unwrap()) as i64;
        assert_eq!((rl.rva as i64 + 33 + 5 + rel) as u32, 0x0016_C910);
    }

    #[test]
    fn magic_sites() {
        // The exeMagic hook is no other stage's site; its slot holds it; the callees are new.
        for s in MAGIC_HOOKS.iter().chain(MAGIC_CALLEES).chain(MAGIC_DATA).chain(MAGIC_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        assert_eq!(MAGIC_PTRS[0].target, MAGIC_HOOKS[0].rva);
        assert_eq!(0x1_4000_0000u64 + MAGIC_PTRS[0].rva as u64, data::SORA_VTABLE + 0x150);
        assert!(!all_sites().any(|s| s.rva == MAGIC_PTRS[0].rva));
        // The Field exe's table entry for action 0x13 is the switch's base + (0x13 − 1) × 4, and
        // holds the default the switch's `ja` goes to.
        let sw = MAGIC_CHECKED[0].expect;
        let base = u32::from_le_bytes(sw[30..34].try_into().unwrap());
        assert_eq!(base + (crate::magic::RECOM_MAGIC_ACTION - 1) * 4, MAGIC_DATA[0].rva);
        let ja = i32::from_le_bytes(sw[13..17].try_into().unwrap()) as i64;
        let default = (0x001F_A1B0i64 + 17 + ja) as u32;
        assert_eq!(default, MAGIC_CHECKED[1].rva);
        assert_eq!(u32::from_le_bytes(MAGIC_DATA[0].expect[..4].try_into().unwrap()), default);
        // The stub fits the cave, and both stay clear of the Attack's cave.
        assert!(crate::magic::field_case::STUB_LEN <= MAGIC_DATA[1].expect.len());
        assert_ne!(MAGIC_DATA[1].rva, crate::attack::battle_case::CAVE_RVA);
        // 7.4b: the thunk jumps through the homing slot; the init stores the thunk; the walk calls
        // the handle lookup; the manager global is where the pick reads it.
        let site = |n: &str| all_sites().find(|s| s.name == n).unwrap_or_else(|| panic!("{n}"));
        let thunk = site("bullet homing thunk (0x1401560B8)");
        assert_eq!(thunk.expect[5] as u64, 0x1_4000_0000 + MAGIC_RETARGET_PTRS[0].rva as u64 - crate::guard::bullet::HOMING_VTABLE);
        assert_eq!(MAGIC_RETARGET_PTRS[0].target, site("CHomingBullet homing step (0x1401575D0)").rva);
        let lea = site("bullet init: exe thunk (0x1401585D5)");
        assert_eq!((lea.rva as i64 + 7 + i32::from_le_bytes(lea.expect[3..7].try_into().unwrap()) as i64) as u32, thunk.rva);
        let store = site("bullet init: exe store +0x2D0 (0x140158609)");
        assert_eq!(u32::from_le_bytes(store.expect[3..7].try_into().unwrap()), 0x2D0);
        let ld = site("nearest pick: lock-on manager (0x1401CDBCF)");
        assert_eq!(0x1_4000_0000 + (ld.rva as i64 + 7 + i32::from_le_bytes(ld.expect[3..7].try_into().unwrap()) as i64) as u64, 0x1_4087_B1E0);
        let walk = site("nearest pick: list walk (0x1401CDC26)");
        assert_eq!(walk.expect[3], 0x88);
        assert_eq!((walk.rva as i64 + 24 + i32::from_le_bytes(walk.expect[20..24].try_into().unwrap()) as i64) as u32, 0x002A_D350);
        assert!(!all_sites().any(|s| s.rva == MAGIC_RETARGET_PTRS[0].rva));
        // 7.6: the Thunder case's effect call lands on 0x1401A6B20, its constants are where it
        // reads them; the collision pass's apply call goes through vtable +0x38.
        let hand = site("Thunder case: hand effect (0x14022276C)");
        let n = hand.expect.len();
        assert_eq!((hand.rva as i64 + n as i64 + i32::from_le_bytes(hand.expect[n - 4..].try_into().unwrap()) as i64) as u32, 0x001A_6B20);
        let rip = |s: &Site, at: usize, len: i64| (s.rva as i64 + len + i32::from_le_bytes(s.expect[at..at + 4].try_into().unwrap()) as i64) as u32;
        assert_eq!(rip(site("Thunder case: bolt effect drop (0x1402227F3)"), 4, 8), site("Thunder case: -6.0 (0x14050932C)").rva);
        assert_eq!(rip(site("Thunder case: bolt pitch (0x140222867)"), 4, 8), site("Thunder case: pi/2 (0x1406C10F0)").rva);
        assert_eq!(f32::from_le_bytes(site("Thunder case: pi/2 (0x1406C10F0)").expect[..4].try_into().unwrap()), std::f32::consts::FRAC_PI_2);
        assert_eq!(f32::from_le_bytes(site("Thunder case: -6.0 (0x14050932C)").expect[..4].try_into().unwrap()), -6.0);
        assert_eq!(site("collision pass: apply call (0x14016EC51)").expect[11], 0x38);
        assert_eq!(MAGIC_THUNDER_PTRS[1].rva - MAGIC_THUNDER_PTRS[0].rva, 0x30);
        for p in MAGIC_THUNDER_PTRS {
            assert!(all_sites().any(|s| s.rva == p.target), "{}", p.name);
            assert!(!all_sites().any(|s| s.rva == p.rva), "{}", p.name);
        }
        // 7.5: the Blizzard case calls the serial getter and leas the side-shard sets
        // (0x1405A1790 = the effect sets + 0x60).
        let ser = site("Blizzard case: shared serial (0x1402224A7)");
        assert_eq!((ser.rva as i64 + 5 + i32::from_le_bytes(ser.expect[1..5].try_into().unwrap()) as i64) as u32, site("shot hit serial (0x140156770)").rva);
        let sets = site("Blizzard case: side-shard sets (0x1402224E9)");
        assert_eq!(0x1_4000_0000 + (sets.rva as i64 + 7 + i32::from_le_bytes(sets.expect[3..7].try_into().unwrap()) as i64) as u64, 0x1_405A_1730 + 0x60);
        // 7.7: the Cure case's and the charge's calls land on the bone, effect, object and heal
        // functions; their RIP loads on the offsets.
        let call = |s: &Site, at: usize| (s.rva as i64 + at as i64 + 5 + i32::from_le_bytes(s.expect[at + 1..at + 5].try_into().unwrap()) as i64) as u32;
        let cure = site("Cure case (0x140222946)");
        assert_eq!(rip(cure, 12, 16), site("Cure case body offset (0x1405A0910)").rva);
        assert_eq!((call(cure, 27), call(cure, 88), call(cure, 95), call(cure, 132)), (0x002A_BB20, 0x001A_2D50, 0x0025_7EF0, 0x001E_59A0));
        assert_eq!(cure.expect[48] as i8, -0xE);
        let ch = site("initMagic: Cure charge (0x14022070A)");
        assert_eq!((call(ch, 5), call(ch, 72)), (0x002A_BB20, 0x001A_2D50));
        assert_eq!(rip(ch, 44, 48), site("initMagic charge offset (0x1407C7E50)").rva);
        for rva in [0x002A_BB20u32, 0x001A_2D50, 0x0025_7EF0, 0x001E_59A0] {
            assert!(all_sites().any(|s| s.rva == rva), "{rva:#x}");
        }
        // 7.8: the Gravity charge and hand effect calls; the flags; the +0xB8 lea; the sets' +0x30.
        let gc = site("initMagic: Gravity charge (0x1402207A5)");
        assert_eq!((call(gc, 7), call(gc, 74)), (0x002A_BB20, 0x001A_2D50));
        assert_eq!(rip(gc, 46, 50), site("initMagic charge offset (0x1407C7E50)").rva);
        assert_eq!(u32::from_le_bytes(gc.expect[70..74].try_into().unwrap()), 0x005A_1730 + 0x30);
        let gh = site("Gravity case: hand effect (0x1402229F8)");
        assert_eq!((call(gh, 54), gh.expect[53], gh.expect[6]), (0x001A_6B20, 0x30, 1));
        assert_eq!(u32::from_le_bytes(site("Gravity case: flags (0x140222AB7)").expect[4..8].try_into().unwrap()), 0x41E1);
        assert_eq!(rip(site("Gravity case: desc +0xB8 (0x140222B09)"), 3, 7), site("Gravity case desc +0xB8 (0x1407C78D0)").rva);
        // 7.9: the Stop gate reads param +0x2C and stores +0x2D8; the picker's case 8 and the hit
        // handler's test name +0x940; the Stop case's flags; the Stop charge's calls.
        let g = site("kind specials: Stop gate (0x14017DCB5)");
        assert_eq!((g.expect[3], g.expect[9], u32::from_le_bytes(g.expect[23..27].try_into().unwrap())), (0x08, 0x2C, 0x2D8));
        assert_eq!(u32::from_le_bytes(site("reaction picker: stop state (0x14017D28A)").expect[7..11].try_into().unwrap()), 0x940);
        assert_eq!(u32::from_le_bytes(site("enemy hit: stop state test (0x14017F937)").expect[7..11].try_into().unwrap()), 0x940);
        let sf = site("Stop case: flags (0x140222BC8)");
        assert_eq!((sf.expect[12], sf.expect[40]), (0x05, 0x25));
        let sc = site("initMagic: Stop charge (0x140220926)");
        assert_eq!((call(sc, 8), call(sc, 73), call(sc, 80)), (0x002A_BB20, 0x001A_2D50, 0x0025_7EF0));
        // 7.10: the Aero case's calls and ids; the damage check reads +0x690's value through
        // 0x14016FDB0 (+0x92: the barrier halves +0x722).
        let ae = site("Aero case: effects (0x140222D2F)");
        assert_eq!((call(ae, 3), call(ae, 19), call(ae, 78), call(ae, 133)), (0x001A_49B0, 0x001F_3C00, 0x001A_6B20, 0x001A_6B20));
        assert_eq!((ae.expect[69] as i8, ae.expect[124] as i8), (-0x16, -0x11));
        assert_eq!(site("Aero case: flags (0x140222DFE)").expect[12], 7);
        assert_eq!(f32::from_le_bytes(site("Aero case: -pi/2 (0x140520910)").expect[..4].try_into().unwrap()), -std::f32::consts::FRAC_PI_2);
        let hv = site("damage check: hit value (0x1401E70B0)");
        assert_eq!((u32::from_le_bytes(hv.expect[3..7].try_into().unwrap()), call(hv, 51)), (0x690, 0x0016_FDB0));
        assert_eq!(0x690 + 0x92, 0x722);
        assert_eq!(u32::from_le_bytes(site("damage check: pending hit (0x1401E6FBE)").expect[3..7].try_into().unwrap()), 0x720);
        // M7 f4: the Thunder charge: the weapon, bone 2, the offset, the set's +0x18 name, the
        // effect, its object, kept on it.
        let tc = site("initMagic: Thunder charge (0x14022062B)");
        assert_eq!((u32::from_le_bytes(tc.expect[3..7].try_into().unwrap()), tc.expect[8], call(tc, 20), call(tc, 87), call(tc, 94)), (0x3D0, 2, 0x002A_BB20, 0x001A_2D50, 0x0025_7EF0));
        assert_eq!(rip(tc, 59, 63), site("initMagic charge offset (0x1407C7E50)").rva);
        assert_eq!(u32::from_le_bytes(tc.expect[83..87].try_into().unwrap()), 0x005A_1730 + 0x18);
        assert_eq!(&tc.expect[104..108], &[0x83, 0x48, 0x4C, 0x04]);
        // M7 f3: the landing timer and the phase-2 wait use +0x160; the next landing bumps +0x33E
        // and sets phase (+0x270) 1; the actor update counts +0x160 down by +0x14 before +0x78.
        let lt = site("enemy stop exe: landing timer (0x140184910)");
        assert_eq!((u32::from_le_bytes(lt.expect[1..5].try_into().unwrap()), u32::from_le_bytes(lt.expect[10..14].try_into().unwrap())), (750, 0x160));
        let nl = site("enemy stop exe: next landing (0x14018499D)");
        assert_eq!((u32::from_le_bytes(nl.expect[2..6].try_into().unwrap()), u32::from_le_bytes(nl.expect[16..20].try_into().unwrap()), u32::from_le_bytes(nl.expect[22..26].try_into().unwrap()), nl.expect[26]), (0x160, 0x33E, 0x270, 1));
        let au = site("actor update: timer, pre-update, state (0x1401A5A7F)");
        assert_eq!((u32::from_le_bytes(au.expect[2..6].try_into().unwrap()), au.expect[12], au.expect[27], au.expect[31]), (0x160, 0x14, 0x78, 0x58));
        // M7 f2: the scale getter and setter both use the mover's +0x70.
        let sg = site("actor scale (0x1401A4A20)");
        let ss = site("actor scale set (0x1401A7050)");
        assert_eq!((sg.expect[3], sg.expect[10], ss.expect[3], ss.expect[14]), (0xF0, 0x70, 0xF0, 0x70));
        // The callees the cast also uses are checked by earlier stages: end attack, ground test,
        // handle → object.
        for rva in [0x001F_6660u32, 0x001F_3C00, 0x002A_D350] {
            assert!(all_sites().any(|s| s.rva == rva), "{rva:#x}");
        }
    }

    #[test]
    fn unlock_sites() {
        // The hook is the function the add calls after the discovery step; the reader the unlocks
        // call (Keyblades' `0x14015B3F0`) reads the collection global `0x14087A7E8`.
        let site = |n: &str| all_sites().find(|s| s.name == n).unwrap_or_else(|| panic!("{n}"));
        let add = site("card add: discovery, first obtained (0x14015CDFB)");
        let call = |s: &Site, at: usize| (s.rva as i64 + at as i64 + 5 + i32::from_le_bytes(s.expect[at + 1..at + 5].try_into().unwrap()) as i64) as u32;
        assert_eq!((call(add, 3), call(add, 11)), (0x0015_CAE0, UNLOCK_HOOKS[0].rva));
        let ever = site("card ever obtained (0x14015B3F0)");
        assert_eq!(0x1_4000_0000 + (ever.rva as i64 + 11 + i32::from_le_bytes(ever.expect[7..11].try_into().unwrap()) as i64) as u64, 0x1_4087_A7E8);
        for s in UNLOCK_HOOKS.iter().chain(UNLOCK_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
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

    #[test]
    fn kh1_hud_mask_sites() {
        // Each retargeted call sits inside its checked context, as `call rel32` to 0x1401B08C0.
        for (c, ctx) in KH1_HUD_MASK_CALLS.iter().zip(KH1_HUD_MASK_CHECKED) {
            let at = (c.rva - ctx.rva) as usize;
            assert_eq!(&ctx.expect[at..at + 5], c.expect, "{}", c.name);
            let rel = i32::from_le_bytes(c.expect[1..5].try_into().unwrap());
            assert_eq!((c.rva as i64 + 5 + rel as i64) as u32, 0x001B_08C0, "{}", c.name);
            assert_eq!(&ctx.expect[..5], &[0xBA, 0xE2, 0x00, 0x00, 0x00], "{}", ctx.name);
        }
        // Not hooked or retargeted by another stage.
        for c in KH1_HUD_MASK_CALLS {
            assert!(!M6_COCKPIT_CALLS.iter().chain(M6_CARD_CALLS).chain(M6_ENEMY_UI_CALLS).any(|m| m.rva == c.rva), "{}", c.name);
        }
    }

    #[test]
    fn kh1_hud_callees() {
        // Both end in a jmp rel32 to the 2D manager's handle functions.
        let jmp = |site: &Site, at: usize| -> u32 { (site.rva as i64 + at as i64 + 5 + i32::from_le_bytes(site.expect[at + 1..at + 5].try_into().unwrap()) as i64) as u32 };
        let play = KH1_HUD_CALLEES.iter().find(|s| s.rva == 0x0024_8550).unwrap();
        assert_eq!(jmp(play, 23), 0x0024_6710);
        let time = KH1_HUD_CALLEES.iter().find(|s| s.rva == 0x0024_8000).unwrap();
        assert_eq!(jmp(time, 23), 0x0024_6250);
        assert!(KH1_HUD_CALLEES.iter().all(|s| s.expect.len() >= 5));
        // The mode ctor writes C2DSeqCtrl's vtable (lea rip+disp at +10, 7 bytes).
        let m = KH1_HUD_CALLEES.iter().find(|s| s.rva == 0x0024_71D0).unwrap();
        assert_eq!(m.rva as i64 + 17 + i32::from_le_bytes(m.expect[13..17].try_into().unwrap()) as i64, 0x0060_7340);
        // H6 / H7 wrappers move whole instructions only (no rip-relative operand, no call).
        for h in KH1_HUD_PARTY_HOOKS.iter().chain(KH1_HUD_CURSOR_HOOKS).chain(KH1_HUD_POPUP_HOOKS) {
            assert!(h.expect.len() >= 5 && !h.expect.contains(&0xE8) && !h.expect.contains(&0xE9), "{}", h.name);
        }
    }

    #[test]
    fn menu_sound_sites_name_the_globals_the_dll_reads() {
        let rip = |s: &Site, off: usize, len: usize, disp_at: usize| {
            let d = i32::from_le_bytes(s.expect[disp_at..disp_at + 4].try_into().unwrap()) as i64;
            (0x1_4000_0000i64 + s.rva as i64 + off as i64 + len as i64 + d) as u64
        };
        let r = &MENU_SOUND_CALLEES[0];
        // cmp qword [rip+d],0 at +0x2E (8 bytes); mov rax,[rip+d] at +0x5F (7 bytes).
        assert_eq!(rip(r, 0x2E, 8, 0x31), 0x1_40C1_58B0);
        assert_eq!(rip(r, 0x5F, 7, 0x62), 0x1_4082_2370);
        assert_eq!(r.expect.len(), 0x66);
        // The map's hook moves its first three whole instructions only.
        assert_eq!(MENU_SOUND_HOOKS[0].expect.len(), 13);
    }

    #[test]
    fn story_skip_sites() {
        for s in STORY_SKIP_HOOKS.iter().chain(STORY_SKIP_CALLEES) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        // Reused: the system variable get / set, Riku's story.
        for rva in [0x001B_08C0u32, 0x001B_16C0, 0x001A_AAE0] {
            assert!(all_sites().any(|s| s.rva == rva), "{rva:#x}");
        }
        // The hook moves exactly mov [rsp+8],rbx; push rdi; sub rsp,0x20.
        assert_eq!(STORY_SKIP_HOOKS[0].expect, [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20]);
        // The saved door reads the save block pointer 0x14087B110 (rip-relative at +0, 7 bytes).
        let d = STORY_SKIP_CALLEES.iter().find(|s| s.rva == 0x0023_3AE0).unwrap();
        let disp = i32::from_le_bytes(d.expect[3..7].try_into().unwrap()) as i64;
        assert_eq!(0x1_4000_0000i64 + d.rva as i64 + 7 + disp, crate::keyblades::SAVE_PTR as i64);
    }

    #[test]
    fn shop_sites() {
        use crate::shop;
        // Each shop site is no other stage's; the reused callees are checked elsewhere.
        for s in SHOP_HOOKS.iter().chain(SHOP_CALLEES).chain(SHOP_CHECKED) {
            assert_eq!(all_sites().filter(|x| x.rva == s.rva).count(), 1, "{}", s.name);
        }
        for rva in [0x004A_CE54u32, 0x002A_A360, 0x0029_B2E0, 0x0029_B2F0, 0x0029_B410, 0x0029_B490, 0x001B_08C0, 0x0024_7E50, 0x0024_85B0, 0x0025_8DE0] {
            assert!(all_sites().any(|s| s.rva == rva), "{rva:#x}");
        }
        assert_eq!(SHOP_HOOKS[0].rva, 0x003A_6930);
        assert_eq!(SHOP_HOOKS[1].rva, 0x003A_73E0);
        let find = |n: &str| SHOP_CALLEES.iter().chain(SHOP_CHECKED).find(|s| s.name.starts_with(n)).unwrap();
        // A target: the 4-byte displacement at `disp` of the instruction at `off`, `len` long.
        let target = |s: &Site, off: usize, len: usize, disp: usize| {
            let d = i32::from_le_bytes(s.expect[off + disp..off + disp + 4].try_into().unwrap()) as i64;
            (0x1_4000_0000i64 + s.rva as i64 + off as i64 + len as i64 + d) as u64
        };
        let call = |s: &Site, at: u64| {
            let off = (at - 0x1_4000_0000 - s.rva as u64) as usize;
            assert_eq!(s.expect[off], 0xE8, "{} at {at:#x}", s.name);
            target(s, off, 5, 1)
        };
        // The state's head: the help line from 0x1403A5AA0, the 0x80 button (pad fresh), sound 3,
        // mode 5, state 100.
        let head = find("pack screen state 2 head");
        assert_eq!(call(head, 0x1_403A_6BC9), 0x1_403A_5AA0);
        assert_eq!(call(head, 0x1_403A_6BD7), 0x1_4045_73A0);
        assert_eq!(call(head, 0x1_403A_6BEC), 0x1_4029_B2F0);
        assert_eq!(head.expect[0x1F..0x23], [0x80, 0, 0, 0]);
        assert_eq!(head.expect[0x33], 4); // lea ecx,[rdx+4] with edx −1: sound 3
        assert_eq!(head.expect[0x3F], shop::mode::TRADE as u8);
        // The tab row: cancel, Left (0x408000), Right (0x802000), Down (0x204000), confirm, the
        // wrap at 3 and Re:CoM's refresh.
        let row = find("pack screen tab row");
        assert_eq!(call(row, 0x1_403A_6C33), 0x1_4029_B410);
        assert_eq!(row.expect[0x30], shop::mode::MENU as u8);
        assert_eq!(call(row, 0x1_403A_6C7A), 0x1_4029_B2E0);
        assert_eq!(row.expect[0x5B..0x5F], 0x0040_8000u32.to_le_bytes());
        assert_eq!(row.expect[0x8D..0x91], 0x0080_2000u32.to_le_bytes());
        assert_eq!(row.expect[0xBA..0xBE], 0x0020_4000u32.to_le_bytes());
        assert_eq!(call(row, 0x1_403A_6CEC), 0x1_4029_B490);
        assert_eq!(row.expect[0x121], (shop::RECOM_TABS - 1) as u8);
        assert_eq!(call(row, 0x1_403A_6D48), 0x1_403A_92F0);
        // The grid's confirm: the room check, the sold-out bit, price against [0x14087CBF8]+0x20,
        // sounds 2 and 0x58, else sound 5.
        let confirm = find("pack screen grid confirm");
        assert_eq!(call(confirm, 0x1_403A_6D83), 0x1_4029_B490);
        assert_eq!(target(confirm, 0x58, 7, 3), 0x1_4087_CBF8);
        assert_eq!(confirm.expect[0x74], shop::sound::BUY as u8);
        assert_eq!(confirm.expect[0x84], (shop::sound::BUY_TOO as i32 + 1) as u8); // lea ecx,[rbp+0x59], ebp −1
        assert_eq!(confirm.expect[0xCD], (shop::sound::ERROR + 1) as u8); // lea ecx,[rdx+6], edx −1
        // The moves: the four masks in Re:CoM's order, cancel, the table and the 0x40 limit.
        let moves = find("pack screen grid moves");
        for (off, mask) in [(0x1E, 0x0020_4000u32), (0x3D, 0x0080_2000), (0x5C, 0x0010_1000), (0x7E, 0x0040_8000)] {
            assert_eq!(moves.expect[off..off + 4], mask.to_le_bytes(), "{off:#x}");
        }
        assert_eq!(call(moves, 0x1_403A_6EF1), 0x1_4029_B410);
        assert_eq!(moves.expect[0xBE..0xC2], 0x0065_BA70u32.to_le_bytes());
        assert_eq!(moves.expect[0xC5], 0x40);
        // The payment: points − price clamped to 99999, the tally (0, 0xB6), its trophy at 20000
        // (99, flag 0x140AF7389 & 0x10, object 0x140AF7468), Re:CoM's setter and the window.
        let pay = find("pack screen payment");
        assert_eq!(target(pay, 0x12, 7, 3), 0x1_4087_CBF8);
        assert_eq!(pay.expect[0x24..0x28], (shop::MAX_POINTS as u32).to_le_bytes());
        assert_eq!(target(pay, 0x35, 7, 2), 0x1_40AF_7389);
        assert_eq!(pay.expect[0x3F], shop::SPENT_VAR.1 as u8);
        assert_eq!(call(pay, 0x1_403A_7086), 0x1_401B_08C0);
        assert_eq!(call(pay, 0x1_403A_70B4), 0x1_401B_16C0);
        assert_eq!(pay.expect[0x7A..0x7E], (shop::SPENT_TROPHY_AT as u32).to_le_bytes());
        assert_eq!(target(pay, 0x89, 7, 3), 0x1_40AF_7468);
        assert_eq!(pay.expect[0x96], shop::SPENT_TROPHY as u8);
        assert_eq!(call(pay, 0x1_403A_70DB), 0x1_402B_A6F0);
        assert_eq!(call(pay, 0x1_403A_70F3), 0x1_4023_A1D0);
        assert_eq!(call(pay, 0x1_403A_7101), 0x1_4045_98A0);
        // The packs: 0xB8 bytes, the ctor (alloc, +0x268, index, price, sold out); released with
        // +0x6C = 1.
        let make = find("pack screen pack creation");
        assert_eq!(make.expect[0x15..0x19], 0xB8u32.to_le_bytes());
        assert_eq!(call(make, 0x1_403A_6AD5), 0x1_404A_CE54);
        assert_eq!(call(make, 0x1_403A_6B13), 0x1_4045_A9E0);
        let release = find("pack screen release");
        assert_eq!(release.expect[0x0B..0x0E], [0x89, 0x70, 0x6C]);
        // The data: the tab and cell places, the moves, the cursor y and message y.
        let t = find("pack screen tables").expect;
        let f = |i: usize| f32::from_le_bytes(t[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!([f(0), f(1), f(2), f(3)], shop::RECOM_TAB_X);
        assert_eq!([f(4), f(5), f(6), f(7)], shop::CELL_X);
        assert_eq!([f(8), f(9), f(10), f(11)], shop::CELL_Y);
        assert_eq!(t[48..64].iter().map(|&b| b as i8).collect::<Vec<_>>(), shop::MOVES);
        let y = find("pack screen cursor y").expect;
        assert_eq!(f32::from_le_bytes(y[..4].try_into().unwrap()), shop::TAB_CURSOR_Y);
        assert_eq!(f32::from_le_bytes(y[4..].try_into().unwrap()), shop::TAB_MESSAGE_Y);
        // The four item cards' class rows: class 0x44.
        for kind in [crate::items::id::POTION, crate::items::id::HI_POTION, crate::items::id::ETHER, crate::items::id::ELIXIR].map(|i| shop::card_kind(i).unwrap()) {
            let row = SHOP_CHECKED.iter().find(|s| s.rva == 0x0068_E0B0 + kind as u32 * 16).unwrap();
            assert_eq!(row.expect[0], 0x44, "{}", row.name);
        }
        // The window's tab play: shown (+0x58 == 1, +0x60 == 0), a new tab (+0x5C), the lad
        // pointer, set 1, the group table 0x140690420 by tab, the control's +0x18.
        let play = find("pack window tab play");
        assert_eq!(play.expect[0x04..0x08], [0x83, 0x79, 0x58, 0x01]);
        assert_eq!(play.expect[0x0A..0x0E], [0x83, 0x79, 0x60, 0x00]);
        assert_eq!(play.expect[0x10..0x13], [0x39, 0x51, 0x5C]);
        assert_eq!(target(play, 0x15, 7, 3), 0x1_4085_9C48);
        assert_eq!(play.expect[0x22..0x28], [0x41, 0xB9, 0x01, 0x00, 0x00, 0x00]);
        assert_eq!(play.expect[0x28..0x2C], [0x48, 0x8B, 0x49, 0x68]);
        assert_eq!(target(play, 0x2F, 7, 3), 0x1_4069_0420);
        assert_eq!(play.expect[0x51..0x54], [0x41, 0xFF, 0x52]);
        assert_eq!(play.expect[0x54], 0x18);
        // The window table: Re:CoM's 66 / 68 / 70 / 72 and their closes, as `shop::tabs`.
        let w = find("pack window groups").expect;
        let g = |i: usize| i16::from_le_bytes([w[2 * i], w[2 * i + 1]]);
        assert_eq!([g(0), g(1), g(2), g(3)], shop::tabs::RECOM_WINDOW);
        assert_eq!([g(4), g(5), g(6), g(7)], shop::tabs::RECOM_CLOSE);
        // The icons' set-up stores the close at +0x76, the set at +0x70, appear +0x72, loop +0x74.
        let gs = find("CMShopGSeq set-up groups").expect;
        assert_eq!(gs[8..12], [0x66, 0x89, 0x43, 0x76]);
        assert_eq!(gs[17..21], [0x66, 0x89, 0x7B, 0x70]);
        assert_eq!(gs[21..25], [0x66, 0x89, 0x73, 0x72]);
        assert_eq!(gs[25..29], [0x66, 0x89, 0x6B, 0x74]);
        // The card trade's Square: Re:CoM's room rule, the lock, the 0x80 button, sound 3, mode 4,
        // state 100.
        let sq = find("card trade pack button");
        assert_eq!(target(sq, 0, 7, 3), 0x1_4087_A7E8);
        assert_eq!(sq.expect[0x14], 0x44);
        assert_eq!(sq.expect[0x17], 5);
        assert_eq!(sq.expect[0x28..0x2C], [0x80, 0, 0, 0]);
        assert_eq!(call(sq, 0x1_403A_7556), 0x1_4029_B2F0);
        assert_eq!(sq.expect[0x3C], (shop::sound::OTHER + 1) as u8);
        assert_eq!(sq.expect[0x43], shop::mode::PACKS as u8);
        assert_eq!(sq.expect[0x53], 0x64);
        // Its button: 0x88 bytes, the ctor (alloc, +0x268, 1), +0x448, its +0x38, show 0.
        let btn = find("card trade button creation");
        assert_eq!(btn.expect[1..5], 0x88u32.to_le_bytes());
        assert_eq!(call(btn, 0x1_403A_8566), 0x1_404A_CE54);
        assert_eq!(btn.expect[0x14..0x1A], [0x41, 0xB8, 1, 0, 0, 0]);
        assert_eq!(call(btn, 0x1_403A_8585), 0x1_4045_A940);
        assert_eq!(btn.expect[0x2C..0x33], [0x48, 0x89, 0xB7, 0x48, 0x04, 0x00, 0x00]);
        assert_eq!(btn.expect[0x39..0x3C], [0xFF, 0x50, 0x38]);
        assert_eq!(call(btn, 0x1_403A_85A6), 0x1_4045_98D0);
        // The card trade's sale sound (6).
        let sale = find("card trade sale sound");
        assert_eq!(sale.expect[7], (shop::sound::SALE + 1) as u8);
        // M8d step 3. The tab row: Left (0x408000) then Right (0x802000) repeats, sound 1 each;
        // the wrap at 4; Re:CoM's fill and blank box; Up / Down fresh (0x305000), sound 0xE; a
        // fresh Down clears the scroll and the cell together (one qword).
        let row = find("card trade tab row");
        assert_eq!(row.expect[0x0F..0x13], 0x0040_8000u32.to_le_bytes());
        assert_eq!(call(row, 0x1_403A_75B8), 0x1_4029_B2E0);
        assert_eq!(row.expect[0x23], 2); // lea ecx,[rdx+2], edx −1: sound 1
        assert_eq!(row.expect[0x43..0x47], 0x0080_2000u32.to_le_bytes());
        assert_eq!(row.expect[0x69], 4); // cmp ebx,4
        assert_eq!(call(row, 0x1_403A_7648), 0x1_4045_DA80);
        assert_eq!(call(row, 0x1_403A_765D), 0x1_4045_D9C0);
        assert_eq!(row.expect[0xD2..0xD6], 0x0030_5000u32.to_le_bytes());
        assert_eq!(call(row, 0x1_403A_767B), 0x1_4029_B2F0);
        let grid = find("card trade grid confirm");
        assert_eq!(call(grid, 0x1_403A_7866), 0x1_4029_B490);
        assert_eq!(grid.expect[0x1F], 3); // sound 2
        assert_eq!(grid.expect[0x5B], 4); // the grid finger's look 4 (a chosen card)
        let ask = find("card trade yes / no open");
        assert_eq!(call(ask, 0x1_403A_7D88), 0x1_4045_95A0);
        assert_eq!(ask.expect[0x22], 7); // lea edx,[rsi+7]: message 7
        assert_eq!(target(ask, 0x52, 8, 4), 0x1_4065_BCEC); // the yes / no finger's x
        assert_eq!(ask.expect[0x4E..0x52], 0x0065_BA38u32.to_le_bytes()); // its y table (an RVA on r15 = the image)
        let pts = find("card trade sale points");
        assert_eq!(call(pts, 0x1_403A_82A5), 0x1_4023_9FA0);
        assert_eq!(call(pts, 0x1_403A_82BB), 0x1_4045_98A0);
        // The ALL finger: subss xmm1,[rip → 62.0]; ours → 50.0 (Re:CoM's float pool).
        let fin = find("card trade ALL finger");
        assert_eq!(fin.expect[..4], [0xF3, 0x0F, 0x5C, 0x0D]);
        assert_eq!(target(fin, 0, 8, 4), 0x1_4050_92A8);
        let (c50, c62) = (find("float 50.0"), find("float 62.0"));
        assert_eq!(f32::from_le_bytes(c50.expect[..4].try_into().unwrap()), -shop::tabs::ALL_FINGER as f32);
        assert_eq!(f32::from_le_bytes(c62.expect[..4].try_into().unwrap()), -shop::tabs::RECOM_ALL_FINGER as f32);
        let ours = (c50.rva as i64 - (fin.rva as i64 + 8)) as i32;
        assert_eq!(ours.to_le_bytes(), [0xB9, 0xD8, 0x0A, 0x00], "win::shop's ALL_FINGER_OURS");
        let strip = find("card trade strip groups");
        let groups: Vec<i16> = strip.expect.chunks(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
        assert_eq!(groups, shop::tabs::RECOM_STRIP);
        let y = find("yes / no finger y");
        assert_eq!([0, 4].map(|o| f32::from_le_bytes(y.expect[o..o + 4].try_into().unwrap())), [214.0, 237.0]);
        assert_eq!(f32::from_le_bytes(find("yes / no finger x").expect[..4].try_into().unwrap()), 232.0);
    }

    #[test]
    fn m11_guard_popup_call() {
        let c = &M11_GUARD_POPUP_CALLS[0];
        assert_eq!(crate::hookplan::call_target(0x1_4000_0000 + c.rva as u64, c.expect), Some(0x1_4020_C9A0));
        let (a, b) = (&M11_GUARD_POPUP_CHECKED[0], &M11_GUARD_POPUP_CHECKED[1]);
        assert_eq!(a.rva + a.expect.len() as u32, c.rva);
        assert_eq!(c.rva + 5, b.rva);
        assert_eq!(&a.expect[5..], &[0x8B, 0x4E, 0x20, 0x8D, 0x55, 0x03], "ecx = [rsi+0x20] (the handle), edx = 3");
    }

    /// The slip's scatter call targets `0x1401F6190` and sits between its context; the callee is its own site.
    #[test]
    fn m11_slip_scatter_call() {
        let c = &SLIP_SCATTER_CALLS[0];
        assert_eq!(crate::hookplan::call_target(0x1_4000_0000 + c.rva as u64, c.expect), Some(0x1_401F_6190));
        let (a, b, callee) = (&SLIP_SCATTER_CHECKED[0], &SLIP_SCATTER_CHECKED[1], &SLIP_SCATTER_CHECKED[2]);
        assert_eq!(a.rva + a.expect.len() as u32, c.rva);
        assert_eq!(c.rva + 5, b.rva);
        assert_eq!(callee.rva, 0x001F_6190);
        assert_eq!(&a.expect[3..13], &[0xC7, 0x87, 0x14, 0x04, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00], "+0x414 = 6: the slip's action");
    }

    /// The stomp ring's hook: Re:CoM's circle-against-body test, its prologue moved whole (push rbp / rsi / rdi, sub
    /// rsp 0x50: 8 bytes, room for the 5-byte jump), listed once among every site, and apart from the ring's callee.
    #[test]
    fn m11_ring_hook() {
        let h = &KH1_ENEMY_RING_HOOKS[0];
        assert_eq!(h.rva, 0x0017_7190);
        assert_eq!(h.expect, &[0x40, 0x55, 0x56, 0x57, 0x48, 0x83, 0xEC, 0x50]);
        assert_eq!(all_sites().filter(|s| s.rva == h.rva).count(), 1);
        assert_ne!(h.rva, KH1_ENEMY_RING_CALLEES[0].rva);
        assert_eq!(KH1_ENEMY_RING_HOOKS.len(), 1);
    }
}
