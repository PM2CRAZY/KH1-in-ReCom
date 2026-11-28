//! The KH1 HUD sprite set `kh1hud` (decision 39; stages H1 / H2, `docs/recom-kh1-hud.md`): KH1's
//! own HUD pictures and layouts turned into Re:CoM 2D data, added to Re:CoM's HUD files so
//! Re:CoM registers, animates and draws them.
//!
//! - `FORM/0002/FO0002.SPR` gets a 4th file `kh1hud.lad` (one SEQD, registered as `kh1hud:00`)
//!   and PC assets `<LANG>_kh1hud.lad` (the same records with the HD atlas' UVs);
//!   `FORM/0002/FO0002.RTM` gets `kh1hud.imz` and PC assets `<LANG>_kh1hud.imz` (the HD atlas,
//!   per language: KH1's language pictures). Re:CoM loads FO0002 with the player HUD and takes a
//!   PC asset whose name contains the file's name (`0x14029FCC0`), as for `Cockpit.lad` /
//!   `UK_Cockpit.imz`. Checked (decomp + files).
//! - Textures: one 32-bit IMGD per atlas (PSMCT32, linear, PS2 alpha 0x80 = opaque), the form of
//!   Re:CoM's own PC atlases (`UK_Cockpit.imz`: 1024 × 1024, header words checked against it).
//! - Sora's gauge cluster (H2): KH1's objects (`exchange/UK_uibin_g_s.bin`) at KH1's positions,
//!   and KH1's arcs (`0x14026B790` / `0x14026E450`: a triangle fan from the gauge centre over the
//!   square of half-size R, UVs fixed in the quarter-ring texture) baked by running KH1's own fan
//!   here into pieces (one per quadrant, per 5° / 30° / 1° step as KH1's values move), so that
//!   Re:CoM only shows pieces by frame. Frames are KH1's quantities (HP, MP, charge units).
//!
//! Nothing from either game is stored in the repository: every texel and position is read from
//! the player's files at this step.

use std::fmt::Write as _;

use kh1_formats::egs::HdAsset;

use crate::mp_bar::{self as mb, agroup, anim, part, sgroup, sprite, Container, Entry, Seqd, FLAGS_COLOUR, FLAGS_STATIC, GROUP_WORD_FADE, GROUP_WORD_SCRUB};
use crate::texture::{decode_dds, Image};
use crate::uibin::{Obj, Uibin};
use crate::{bad, Result};

mod ring;
pub use ring::{group as ring_group, CutStats};

/// The layout file added to `FO0002.SPR` (Re:CoM registers `kh1hud` and `kh1hud:00`).
pub const LAD: &str = "kh1hud.lad";
/// The atlas added to `FO0002.RTM` (registered as `kh1hud:00`, bound to the sequence set).
pub const IMZ: &str = "kh1hud.imz";
/// The sprite set's name in Re:CoM's 2D registry.
pub const SET: &str = "kh1hud:00";
/// The manifest the DLL reads.
pub const MANIFEST: &str = "kh1-hud.json";
/// The manifest's format line.
/// Format 3 (0.35.0): the command menu's FM context tab and Triangle groups follow the Magic
/// list's (the words move up by two); the DLL refuses format 2.
pub const FORMAT: &str = "kh1-recom-assets kh1-hud 3";
/// KH1's layout of Sora's gauge (`0x140268800` loads it; `UK` is English, `0x1400D7380`).
pub const G_S: &str = "exchange/UK_uibin_g_s.bin";
/// The danger flash object (`g_s_bar_b_hp_red`, slot `+0x4E0`).
pub const G_RED: &str = "exchange/UK_uibin_g_red.bin";
/// KH1's gauge motions (`vibe_s/ss/sss` the hit shakes, `slide_s_h/v` the size-class slides; slot
/// 0 copies objects 0..4 to `+0x2A0`, `+0x360`, `+0x420`, `+0x120`, `+0x1E0`, `0x14026E630` area).
pub const G_ACT: &str = "exchange/UK_uibin_g_act.bin";
/// KH1's command menu layouts (H4): the root list (`com`, 12 objects), its battle title colours
/// (`com_battle`), the Magic list (`mag`, 18 objects). Loader `0x140277FE0`; Checked (files).
pub const COM: &str = "exchange/UK_uibin_com.bin";
/// The root title's field / battle colour keys.
pub const COM_BATTLE: &str = "exchange/UK_com_battle.bin";
/// The Magic list.
pub const MAG: &str = "exchange/UK_uibin_mag.bin";
/// The Items list (M8 stage 8a.1, `0x14027B870` case 2, layout `0x142859660`): the Magic list's
/// 17 objects in the same order (`ite_fra_d` … `ite_item`, `ite_sel_ico_d`), green
/// (58, 86, 66), its strip 78 wide (the Magic list's 50). Checked (files: the owner's dump).
pub const ITEM: &str = "exchange/UK_uibin_item.bin";
/// M8b: KH1's Summon list (`0x14027B870` case 3, layout `0x142859860`, built from `uibin_sum.bin`
/// by `0x14027AB70(3, …)`; the name: the exe's string `0x1403EE5F8`). Checked (decomp + exe). Its
/// objects are not in this session: used only when they are the Magic list's 17 in the same order
/// (`sum_` for `mag_`; the title, object 15, any name), else the DLL draws the Summon list with the
/// Magic list's panels. Assumption until the owner's step prints it.
pub const SUM: &str = "exchange/UK_uibin_sum.bin";
/// KH1's shortcut panel (M7 stage 7.12): `cut_fra_a` (the frame), `cut_shortcut` (the title),
/// `cut_ico_a / b / c` (the button glyphs' places). `0x140278180` copies its objects 0..4 to
/// `0x142853960` (`0x14027E1C0(&0x140527B58)`), `0x14027B870` case 5 draws them. Checked (decomp +
/// files).
pub const CUT: &str = "exchange/UK_uibin_cut.bin";
/// `uitex.bin` pictures the menu draws from (`0x140278180` passes picture 2 for the root's panels,
/// 3 for the submenus', 1 for the titles and icons): (index, TIM2 offset, pixel data offset).
pub const MENU_PICTURES: [(usize, usize, usize); 3] = [(1, 0x2080, 0x20C0), (2, 0x4100, 0x4140), (3, 0x6180, 0x61C0)];
/// Picture 10 (the lock-on cursors, H7; PC `-18080.dds` 2×).
pub const CURSOR_PICTURE: (usize, usize, usize) = (10, 0x18040, 0x18080);
/// Picture 0 (the LOCK ON! plate's art at texels 100..125 × 100..123, H5; the PC copy 4×).
pub const PLATE_PICTURE: (usize, usize, usize) = (0, 0, 0x40);
/// The corner of picture 0 taken into the atlas (texels 96..128 square).
pub const PLATE_CORNER: usize = 96;
/// The LOCK ON! plate (`0x1402740F0` loads it; objects 5..10) and the lock-on cursors (0..4, H7).
pub const LOCK: &str = "exchange/UK_uibin_lock.bin";
/// The target's HP gauge (`0x140274E00`).
pub const G_ENM: &str = "exchange/UK_uibin_g_enm.bin";
/// KH1's LEVEL UP!! box (`0x1402724D0` loads it; H8).
pub const LEV_S: &str = "exchange/UK_uibin_lev_s.bin";
/// The party's gauges (slots 1, 2; loader `0x140268800`).
pub const G_C: &str = "exchange/UK_uibin_g_c.bin";
/// KH1's menu words font: the system font `menu/<lang>/sysfont.bin` drawn through its PC texture
/// `<LANG>_sysfont_bin0.dds` (1024 × 1024; the same file for every Western language, Checked
/// (files)), glyphs from `exchange/US_font_data_tbl.bin` (`0x1402D1570` loads it for every
/// language but Japanese / FM; 16-byte entries "3s 2c 3s": `+2` u, `+4` v in 1/16 PS2 texels
/// (+8), `+6` advance (PS2 texels), `+7` page: low nibble 512 PC texels right, high nibble 512
/// down; `+8` the character (U+FF01..FF5E = ASCII); Checked (files + decomp `0x1402D2050`)).
/// Not the PC `menufont` of `Theater` (H4 used it: square "A"; the owner's KH1 screenshot shows
/// this font, 2026-11-01). Drawn at size 14 over the 18-texel base (`0x14054534C`,
/// `0x140545344`): "Attack" advances 57 texels = 44.3 units (44.5 on the owner's screenshot).
pub const SYSFONT: &str = "menu/us/sysfont.bin";
/// Its PC texture's name.
pub const SYSFONT_HD: &str = "US_sysfont_bin0.dds";
/// The glyph table.
pub const FONT_TABLE: &str = "exchange/US_font_data_tbl.bin";
/// The PC's button icons (`Image/dt/Theater`, every language the same file; Checked (files)):
/// a TIM2 1024 × 1024 32-bit picture with PS2 alpha, and `FontIcon.inf` (a 16-byte header
/// `{1024, 1024, count, 0}`, then 16-byte records `{code, x0, y0, x1, y1, 0, 0, 0}`: the
/// PlayStation buttons 0xE410 square, 0xE411 triangle, 0xE412 cross, 0xE413 circle; the Xbox
/// ones 0xE3F0..0xE3F3). KH1 draws the Triangle prompt as glyph `0F 46` (`0x140279FC0` →
/// `0x1402D2050` → `0x1402D4A50` / `0x1402D4BB0`); which glyph the PC shows for it depends on
/// the pad (PlayStation: 0xE411, the green triangle; Assumption: the PC's remap of KH1's PS2 icon
/// records onto this picture is not read).
pub const ICON_FILES: [&str; 2] = ["uk/font/Icon.tm2", "uk/font/FontIcon.inf"];
/// The PlayStation Triangle's record in `FontIcon.inf`.
pub const ICON_TRIANGLE: u16 = 0xE411;
/// The PlayStation Square, Cross and Circle (stage 7.12's shortcut panel: KH1's glyphs 0x45, and
/// 0x41 = the confirm button: `0x1402D4A50` gives 3 (as 0x44) without the confirm swap, 0 (as
/// 0x43) with it; `0x1403EFF90`'s PlayStation rows: 0x43 → 0, 0x44 → 3, 0x45 → 2, 0x46 → 1).
/// Checked (decomp + exe); 0 Circle / 1 Triangle / 2 Square / 3 Cross: Assumption (Triangle = 1
/// is the tab's, the rest by the PlayStation order of `FontIcon.inf`'s names).
pub const ICON_SQUARE: u16 = 0xE410;
/// Cross.
pub const ICON_CROSS: u16 = 0xE412;
/// Circle.
pub const ICON_CIRCLE: u16 = 0xE413;
/// The party's model files whose section 2 is the face strip: Sora (Checked (files)), Donald
/// and Goofy (Checked (files: their 48 × 48 faces in the owner's dump)).
pub const FACES: [&str; 3] = ["xa_ex_0010.mdls", "xa_ex_0040.mdls", "xa_ex_0050.mdls"];
/// `uitex.bin` pictures used: (index, TIM2 offset, pixel data offset = the PC asset tag).
pub const PICTURES: [(usize, usize, usize); 3] = [(5, 0xA280, 0xA2C0), (6, 0xE6C0, 0xE700), (7, 0x10740, 0x10780)];
/// Long hold for held animation steps (as `mp_bar::LONG`).
pub const LONG: i32 = 10_000_000;

/// KH1 constants (Checked (exe) unless noted).
pub mod kh1 {
    /// Sora's gauge centre `0x1405272B0`.
    pub const CENTRE: (i32, i32) = (443, 316);
    /// Size-class offsets `0x140527300` (class 0, 1, 2).
    pub const CLASS_OFFSET: [(i32, i32); 3] = [(24, 24), (0, 24), (0, 0)];
    /// Ring table `0x1405273B0` (1/16 units: u, v, radius): HP back, MP back (picture 7), HP, MP,
    /// charge (picture 5).
    pub const RING_HP_BACK: (i32, i32, i32) = (744, 1288, 688);
    /// MP back ring.
    pub const RING_MP_BACK: (i32, i32, i32) = (1720, 1512, 912);
    /// HP ring.
    pub const RING_HP: (i32, i32, i32) = (680, 1592, 656);
    /// MP ring.
    pub const RING_MP: (i32, i32, i32) = (904, 904, 880);
    /// Charge ring.
    pub const RING_CHARGE: (i32, i32, i32) = (1816, 904, 880);
    /// HP: 5° a point to 270° (54 HP), then 6 HP a straight segment, at most 120 HP.
    pub const HP_DEG: i32 = 5;
    /// HP filling the ring.
    pub const HP_RING: i32 = 54;
    /// HP a straight segment.
    pub const HP_PER_SEG: i32 = 6;
    /// HP drawn at most (`0x78`).
    pub const HP_CAP: i32 = 120;
    /// MP: 30° a point to 270° (9 MP), then a segment a point, at most 20.
    pub const MP_DEG: i32 = 30;
    /// MP filling the ring.
    pub const MP_RING: i32 = 9;
    /// MP drawn at most (`0x14`).
    pub const MP_CAP: i32 = 20;
    /// Charge: 1° a unit to 270, then 30 units a segment, at most 600.
    pub const CHARGE_RING: i32 = 270;
    /// Units a straight segment.
    pub const CHARGE_PER_SEG: i32 = 30;
    /// Units drawn at most (`0x258`).
    pub const CHARGE_CAP: i32 = 600;
    /// Straight segment pitch (Sora; party 17).
    pub const PITCH: i32 = 26;
    /// The HP / MP back caps' x (`0x1402698C0`: 0x19E; `0x140269F50`: 0x1B8).
    pub const HP_CAP_X: i32 = 0x19E;
    /// The MP cap's x.
    pub const MP_CAP_X: i32 = 0x1B8;
    /// Hurt face times (`0x1405270FC`, `0x140527134`, `0x14052726C`), 60 Hz frames.
    pub const HURT_FRAMES: [i32; 3] = [40, 50, 60];
}

/// Object indices in `g_s` (`0x1405270E0`, `0x140527138`, `0x140527168`, `0x140527180`; Checked
/// (exe) and their names Checked (files)).
pub mod obj {
    /// `g_s_fra_a_wa..wd`, drawn with picture 6 (`0x140269190`).
    pub const FACE_FRAME: [usize; 4] = [0, 1, 2, 3];
    /// `g_s_fra_a_a..d` (the face disc), `g_s_fra_a_hp`, `g_s_fra_a_mp` (picture 7).
    pub const DISC: [usize; 4] = [4, 5, 6, 7];
    /// "HP".
    pub const LABEL_HP: usize = 8;
    /// "mp".
    pub const LABEL_MP: usize = 9;
    /// Back segments: first HP (`+0xA8`), further HP (`+0xB0`), MP (`+0xB8`), caps (`+0xC0`, `+0xC8`).
    pub const BACK_HP_FIRST: usize = 16;
    /// Further HP back segments.
    pub const BACK_HP: usize = 18;
    /// MP back segments.
    pub const BACK_MP: usize = 17;
    /// HP back cap.
    pub const CAP_HP: usize = 20;
    /// MP back cap.
    pub const CAP_MP: usize = 21;
    /// Colour segments: HP (`+0x90`), MP (`+0x98`), charge (`+0xA0`).
    pub const SEG_HP: usize = 31;
    /// MP segment.
    pub const SEG_MP: usize = 32;
    /// Charge segment.
    pub const SEG_CHARGE: usize = 33;
    /// Max-MP ticks past 90° / 180° (`+0x78`, `+0x80`).
    pub const TICK_90: usize = 37;
    /// Past 180°.
    pub const TICK_180: usize = 38;
    /// The count `g_s` must have.
    pub const COUNT: usize = 39;
}

/// The animation groups of `kh1hud:00`. Held groups take KH1's value as their frame; the rest
/// play over time. The DLL (`crate::kh1_hud` there) uses the same numbers; the manifest carries
/// the count and the format, which it checks.
pub mod group {
    /// Face frame, disc, labels: frame 0 without "mp", 1 with.
    pub const FRAME: usize = 0;
    /// Back arcs and segments: frame = min(max HP, 120) / min(max MP, 20).
    pub const HP_BACK: usize = 1;
    /// MP back.
    pub const MP_BACK: usize = 2;
    /// MP: frame = min(MP, 20).
    pub const MP: usize = 3;
    /// HP: frame = min(HP, 120).
    pub const HP: usize = 4;
    /// Charge: frame = min(charge, 600).
    pub const CHARGE: usize = 5;
    /// Max-MP ticks: frame = min(max MP, 20).
    pub const TICKS: usize = 6;
    /// Face: frame = expression (0, 1 hurt, 2 danger).
    pub const FACE: usize = 7;
    /// Hurt face for 40 / 50 / 60 frames, then expression 0 or 2 (played): 8 + 2 i + (danger).
    pub const HURT: usize = 8;
    /// MP fades of cell j (played): gain 14 + j, loss 23 + j.
    pub const MP_GAIN: usize = 14;
    /// MP loss fades.
    pub const MP_LOSS: usize = 23;
    /// Charge fades of the piece [30 j, 30 j + t] (t = 1..30, played): gain 32 + 30 j + t − 1,
    /// loss 302 + 30 j + t − 1.
    pub const CHARGE_GAIN: usize = 32;
    /// Charge loss fades.
    pub const CHARGE_LOSS: usize = 302;
    /// Danger flash (looped) of the HP arc piece [90 q, 90 q + 5 k] (k = 1..18): 572 + 18 q + k − 1.
    pub const RED_ARC: usize = 572;
    /// Danger flash of a full straight HP segment, and of a partial one (remainder r = 1..5:
    /// 627 + r − 1).
    pub const RED_SEG: usize = 626;
    /// Partial segment flashes.
    pub const RED_SEG_PART: usize = 627;
    /// The fixed groups (the units' motion groups follow, `motion`).
    pub const COUNT: usize = 632;
}

// ---------------------------------------------------------------------------------------------
// Pictures.

/// RGBA, PS2 alpha (0x80 = opaque), rows top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pic {
    /// Width.
    pub w: usize,
    /// Height.
    pub h: usize,
    /// Pixels.
    pub px: Vec<[u8; 4]>,
}

impl Pic {
    fn blank(w: usize, h: usize) -> Pic {
        Pic { w, h, px: vec![[0; 4]; w * h] }
    }
    /// The bilinear sample at (x, y) texels (texel centres at integers), colour weighted by alpha
    /// (clear texels add no colour).
    fn bilinear(&self, x: f64, y: f64) -> [u8; 4] {
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let mut acc = [0f64; 4];
        for (dx, dy, w) in [(0, 0, (1.0 - fx) * (1.0 - fy)), (1, 0, fx * (1.0 - fy)), (0, 1, (1.0 - fx) * fy), (1, 1, fx * fy)] {
            let t = self.at(x0 + dx, y0 + dy);
            let a = t[3] as f64 * w;
            for c in 0..3 {
                acc[c] += t[c] as f64 * a;
            }
            acc[3] += a;
        }
        if acc[3] <= 0.0 {
            return [0; 4];
        }
        [(acc[0] / acc[3]).round() as u8, (acc[1] / acc[3]).round() as u8, (acc[2] / acc[3]).round() as u8, acc[3].round().min(255.0) as u8]
    }
    fn at(&self, x: i64, y: i64) -> [u8; 4] {
        let x = x.clamp(0, self.w as i64 - 1) as usize;
        let y = y.clamp(0, self.h as i64 - 1) as usize;
        self.px[y * self.w + x]
    }
    /// A copy of the rect (x, y, w, h).
    fn crop(&self, x: usize, y: usize, w: usize, h: usize) -> Pic {
        let mut p = Pic::blank(w, h);
        for j in 0..h {
            for i in 0..w {
                p.px[j * w + i] = self.at((x + i) as i64, (y + j) as i64);
            }
        }
        p
    }
    /// A copy resampled to `w`×`h` by area averaging: each texel is the mean of the source area
    /// it covers, colour weighted by alpha (clear texels add no colour), as [`Pic::bilinear`].
    fn resampled(&self, w: usize, h: usize) -> Pic {
        let (sx, sy) = (self.w as f64 / w as f64, self.h as f64 / h as f64);
        // Each output column's / row's source spans: (source index, coverage).
        let spans = |n: usize, s: f64, max: usize| -> Vec<Vec<(usize, f64)>> {
            (0..n)
                .map(|i| {
                    let (a, b) = (i as f64 * s, (i + 1) as f64 * s);
                    let mut v = Vec::new();
                    let mut k = a.floor() as usize;
                    while (k as f64) < b && k < max {
                        let c = (b.min(k as f64 + 1.0) - a.max(k as f64)).max(0.0);
                        if c > 0.0 {
                            v.push((k, c));
                        }
                        k += 1;
                    }
                    v
                })
                .collect()
        };
        let (xs, ys) = (spans(w, sx, self.w), spans(h, sy, self.h));
        let mut p = Pic::blank(w, h);
        for (j, yrow) in ys.iter().enumerate() {
            for (i, xcol) in xs.iter().enumerate() {
                let (mut acc, mut area) = ([0f64; 4], 0f64);
                for &(y, cy) in yrow {
                    for &(x, cx) in xcol {
                        let t = self.px[y * self.w + x];
                        let a = t[3] as f64 * cx * cy;
                        for c in 0..3 {
                            acc[c] += t[c] as f64 * a;
                        }
                        acc[3] += a;
                        area += cx * cy;
                    }
                }
                p.px[j * w + i] = if acc[3] <= 0.0 || area <= 0.0 {
                    [0; 4]
                } else {
                    let c = |k: usize| (acc[k] / acc[3]).round().clamp(0.0, 255.0) as u8;
                    [c(0), c(1), c(2), (acc[3] / area).round().clamp(0.0, 255.0) as u8]
                };
            }
        }
        p
    }
    /// A copy whose clear texels take the colour of the covered texels beside them (`passes`
    /// texels deep; alpha unchanged). The PC atlas is filtered: a clear texel's colour (black or
    /// white in the pictures) leaked into the edges of the art as a dark or light fringe, which
    /// showed as lines where two pieces meet (the rings' quarters). Coverage stays as KH1's.
    fn bled(&self, passes: usize) -> Pic {
        let mut p = self.clone();
        let (w, h) = (p.w, p.h);
        let mut known: Vec<bool> = p.px.iter().map(|t| t[3] != 0).collect();
        for _ in 0..passes {
            let mut next = known.clone();
            let mut out = p.px.clone();
            for y in 0..h {
                for x in 0..w {
                    if known[y * w + x] {
                        continue;
                    }
                    let mut sum = [0u32; 3];
                    let mut n = 0u32;
                    for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                        let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                        if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                            continue;
                        }
                        let i = ny as usize * w + nx as usize;
                        if known[i] {
                            for c in 0..3 {
                                sum[c] += p.px[i][c] as u32;
                            }
                            n += 1;
                        }
                    }
                    if n > 0 {
                        let o = &mut out[y * w + x];
                        for c in 0..3 {
                            o[c] = ((sum[c] + n / 2) / n) as u8;
                        }
                        next[y * w + x] = true;
                    }
                }
            }
            p.px = out;
            known = next;
        }
        p
    }
    /// [`Pic::blit`] plus `g` texels of the source's edge copied around it (the caller keeps
    /// that margin free).
    fn blit_padded(&mut self, src: &Pic, x: usize, y: usize, g: usize) {
        for j in 0..src.h + 2 * g {
            for i in 0..src.w + 2 * g {
                let (tx, ty) = (x + i, y + j);
                if tx < g || ty < g || tx - g >= self.w || ty - g >= self.h {
                    continue;
                }
                self.px[(ty - g) * self.w + tx - g] = src.at(i as i64 - g as i64, j as i64 - g as i64);
            }
        }
    }
}

/// A PC texture (alpha 0..255) as a PS2-alpha picture.
pub fn from_pc(img: &Image) -> Pic {
    Pic { w: img.w, h: img.h, px: img.px.iter().map(|p| [p[0], p[1], p[2], ((p[3] as u32 * 128 + 127) / 255) as u8]).collect() }
}

/// A KH1 picture: the PS2 one and its PC texture (and the PC texture's scale over the PS2 one).
#[derive(Clone, Debug)]
pub struct Source {
    /// The PS2 picture.
    pub ps2: Pic,
    /// The PC texture.
    pub hd: Pic,
    /// The PC texture's scale over the PS2 picture.
    pub scale: usize,
}

/// KH1 FM PC's own scale of its HUD pictures over the PS2 ones (the faces, pictures 1..7).
const HD_SCALE: usize = 2;
/// KH1 FM PC's scale of picture 0 (the LOCK ON! plate / shortcut panel frame).
const HD_SCALE_PLATE: usize = 4;

/// The PC texture at a whole multiple of the PS2 picture, at most `max` (KH1 FM PC's own scale
/// for that picture). A replaced (texture mod) PC texture may be any size, e.g. Sora's face at
/// 1200×400 over the PS2's 192×64 (6.25×), Donald's at 8×: it is resampled to the nearest whole
/// scale, capped at `max` (the PC atlas then stays the stock 2048 × 1024; a texture mod's 4×-8×
/// pictures made a 2048 × 4096 atlas per language that crashed Re:CoM on load), its height kept
/// in proportion. A stock one (a whole multiple, at most `max`) is returned as it is.
fn fit_hd(ps2: &Pic, hd: Pic, max: usize) -> Pic {
    if ps2.w == 0 || hd.w == 0 || (hd.w % ps2.w == 0 && hd.w / ps2.w <= max && hd.h >= ps2.h * (hd.w / ps2.w)) {
        return hd;
    }
    let r = hd.w as f64 / ps2.w as f64;
    let s = (r.round() as usize).clamp(1, max.max(1));
    let w = ps2.w * s;
    let h = ((hd.h as f64 * w as f64 / hd.w as f64).round() as usize).max(ps2.h * s);
    hd.resampled(w, h)
}

/// [`fit_hd`] for a picture's language copies, all at one size: a texture mod may replace some
/// languages' copies only, so each copy is brought to the largest copy's size.
fn fit_all(ps2: &Pic, per: Vec<Pic>, max: usize) -> Vec<Pic> {
    let per: Vec<Pic> = per.into_iter().map(|p| fit_hd(ps2, p, max)).collect();
    let Some(big) = per.iter().max_by_key(|p| p.w * p.h).map(|p| (p.w, p.h)) else { return per };
    per.into_iter().map(|p| if (p.w, p.h) == big { p } else { p.resampled(big.0, big.1) }).collect()
}

fn hd_scale(ps2: &Pic, hd: &Pic) -> Result<usize> {
    let s = hd.w / ps2.w.max(1);
    if s == 0 || hd.w != ps2.w * s || hd.h < ps2.h * s {
        return bad(format!("PC texture {}×{} is not a whole multiple of the PS2 picture {}×{}", hd.w, hd.h, ps2.w, ps2.h));
    }
    Ok(s)
}

/// Finds a `uitex.bin` PC asset tagged / named for pixel offset `off` (`<LANG>_uitex_bin<hex>` for
/// the language, else `-<hex>`), decoded.
fn uitex_asset(assets: &[HdAsset], off: usize, lang: &str) -> Result<Option<Pic>> {
    // Picture 1's language copies are PNGs (`<LANG>_uitex_bin20c0.png`, Checked (files)).
    let names = [format!("{lang}_uitex_bin{off:x}.dds"), format!("{lang}_uitex_bin{off:x}.png"), format!("-{off:x}.dds")];
    for n in &names {
        if let Some(a) = assets.iter().find(|a| a.name.eq_ignore_ascii_case(n)) {
            let img = if n.ends_with(".png") { crate::texture::decode_png(&a.data) } else { decode_dds(&a.data) };
            return Ok(Some(from_pc(&img.map_err(|e| crate::Error::Format(format!("uitex.bin asset {}: {e}", a.name)))?)));
        }
    }
    Ok(None)
}

/// A face strip from section 2 of a `.mdls` (`{w, h, 0, 0}`, CSM1 CLUT at +0x10, 8-bit pixels at
/// +0x410; Checked (files: Sora 192 × 64, Donald / Goofy 48 × 48)) and its PC texture
/// `-<hex pixel offset>.dds`.
pub fn face(mdls: &[u8], assets: &[HdAsset]) -> Result<Source> {
    let rd = |at: usize| -> Result<usize> { mdls.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize).ok_or_else(|| crate::Error::Format("mdls: short".into())) };
    let n = rd(0)?;
    if !(3..=64).contains(&n) {
        return bad(format!("mdls: {n} sections"));
    }
    let s2 = rd(12)?;
    face_section(mdls.get(s2..).ok_or_else(|| crate::Error::Format("mdls: section 2 outside the file".into()))?, s2, assets)
}

/// A face strip from a `.mdls` section 2 that starts at `at` in its file.
pub fn face_section(sec: &[u8], at: usize, assets: &[HdAsset]) -> Result<Source> {
    let rd = |o: usize| -> Result<usize> { sec.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize).ok_or_else(|| crate::Error::Format("mdls: short section 2".into())) };
    let (w, h) = (rd(0)?, rd(4)?);
    if w == 0 || h == 0 || w > 1024 || h > 1024 || 0x410 + w * h > sec.len() {
        return bad(format!("mdls: section 2 at {at:#x} is not a face ({w}×{h})"));
    }
    let clut = &sec[0x10..0x410];
    let px = sec[0x410..0x410 + w * h]
        .iter()
        .map(|&i| {
            let j = crate::rtm::csm1(i as usize);
            clut[4 * j..4 * j + 4].try_into().unwrap()
        })
        .collect();
    let ps2 = Pic { w, h, px };
    let name = format!("-{:x}.dds", at + 0x410);
    let a = assets.iter().find(|a| a.name.eq_ignore_ascii_case(&name)).ok_or_else(|| crate::Error::Format(format!("mdls: no PC face texture {name}")))?;
    let hd = fit_hd(&ps2, from_pc(&decode_dds(&a.data)?), HD_SCALE);
    let scale = hd_scale(&ps2, &hd)?;
    Ok(Source { ps2, hd, scale })
}

// ---------------------------------------------------------------------------------------------
// KH1's arc fan (`0x14026B790`, vertices `0x14026E450`), all in 1/16 units.

/// A ring: the gauge centre on screen and the quarter-ring's centre and radius in its picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ring {
    /// Screen centre x (1/16 units).
    pub x: i32,
    /// Screen centre y.
    pub y: i32,
    /// Texture centre u.
    pub uc: i32,
    /// Texture centre v.
    pub vc: i32,
    /// Radius.
    pub r: i32,
}

/// The fan vertex at `theta` degrees (0 = 9 o'clock, clockwise): (screen x, y; u, v), 1/16 units.
/// `0x14026E450`: d = trunc(tan(t) · R) with t the angle in the 45° octant (mirrored in odd
/// ones); the right / bottom halves are one unit further out (`+ 0x10` when d ≠ 0 at their start).
pub fn vertex(ring: &Ring, theta: f32) -> [i32; 4] {
    let k = (theta / 45.0) as u32;
    let mut t = theta - k as f32 * 45.0;
    if k & 1 != 0 {
        t = 45.0 - t;
    }
    let d = ((t * std::f32::consts::PI / 180.0).tan() * ring.r as f32) as i32;
    let s = if d != 0 { 0x10 } else { 0 };
    let (x, y, r, uc, vc) = (ring.x, ring.y, ring.r, ring.uc, ring.vc);
    match k {
        0 => [x - r, y - d, uc - r, vc - d],
        1 => [x - d, y - r, uc - d, vc - r],
        2 => [x + s + d, y - r, uc - d, vc - r],
        3 => [x + 0x10 + r, y - d, uc - r, vc - d],
        4 => [x + 0x10 + r, y + s + d, uc - r, vc - d],
        _ => [x + s + d, y + 0x10 + r, uc - d, vc - r],
    }
}

/// The fan `0x14026B790` draws for [a0, a1]: the centre (one unit down; UV the ring's centre), then
/// the vertex at a0, at every 45° past it up to a1, and at a1.
pub fn fan(ring: &Ring, a0: f32, a1: f32) -> Vec<[i32; 4]> {
    fan_to(ring, a0, a1)
}

fn fan_to(ring: &Ring, a0: f32, f: f32) -> Vec<[i32; 4]> {
    let mut v = vec![[ring.x, ring.y + 0x10, ring.uc, ring.vc], vertex(ring, a0)];
    let mut i = (a0 / 45.0) as i32;
    while {
        i += 1;
        i <= (f / 45.0) as i32
    } {
        v.push(vertex(ring, i as f32 * 45.0));
    }
    v.push(vertex(ring, f));
    v
}

/// KH1's soft end (`0x14026B790` mode 4, the HP, flash and charge arcs; Checked (decomp + exe: the
/// callers push 4, the constant 0.7 at `0x14042F7F4` is subtracted from the end)): when 1 < a1 <
/// 270 the fan stops at a1 − 0.7°, and one more triangle (centre, a1 − 0.7°, a1) is drawn with
/// the a1 vertex's alpha 0 (Gouraud: the edge fades out over the last 0.7°).
pub const SOFT_DEG: f32 = 0.7;

/// The fan and, with KH1's soft end, its fading wedge (centre, a1 − 0.7°, a1).
pub fn fan_soft(ring: &Ring, a0: f32, a1: f32, soft: bool) -> (Vec<[i32; 4]>, Option<[[i32; 4]; 3]>) {
    if soft && a1 < 270.0 && a1 > 1.0 {
        let f = a1 - SOFT_DEG;
        let v = fan_to(ring, a0, f);
        let w = [v[0], vertex(ring, f), vertex(ring, a1)];
        (v, Some(w))
    } else {
        (fan_to(ring, a0, a1), None)
    }
}

/// Renders a fan: the texel each sample point inside it shows. Samples at `(x0 + (i + ½) / s − ½,
/// y0 + (j + ½) / s − ½)` units (s = 1: the unit grid, where the GS samples). A point on the last
/// radial edge (the centre to the a1 vertex) is outside, so pieces meeting there don't overlap.
fn render(fan: &[[i32; 4]], wedge: Option<&[[i32; 4]; 3]>, src: &Pic, src_scale: usize, s: usize, rect: (i32, i32, i32, i32)) -> Pic {
    let (x0, y0, w, h) = rect;
    let mut p = Pic::blank(w as usize * s, h as usize * s);
    let c = fan[0];
    let n = fan.len();
    // Triangles (centre, a, b) and whether b's alpha is 0 (the soft wedge's end vertex).
    let mut tris: Vec<([i32; 4], [i32; 4], [i32; 4], bool)> = (1..n - 1).map(|k| (c, fan[k], fan[k + 1], false)).collect();
    if let Some(wd) = wedge {
        tris.push((wd[0], wd[1], wd[2], true));
    }
    let area = |t: &([i32; 4], [i32; 4], [i32; 4], bool)| (t.1[0] - t.0[0]) as i64 * (t.2[1] - t.0[1]) as i64 - (t.1[1] - t.0[1]) as i64 * (t.2[0] - t.0[0]) as i64;
    // The last triangle that has an area (KH1 repeats the end vertex when it is a 45° step).
    let last = (0..tris.len()).rev().find(|&k| area(&tris[k]) != 0).unwrap_or(0);
    if s > 1 {
        render_pc(&tris, last, src, src_scale, s, rect, &mut p);
        return p;
    }
    for j in 0..h as usize * s {
        for i in 0..w as usize * s {
            let px = (x0 as f64 + (i as f64 + 0.5) / s as f64 - 0.5) * 16.0;
            let py = (y0 as f64 + (j as f64 + 0.5) / s as f64 - 0.5) * 16.0;
            for (k, &(c, a, b, soft)) in tris.iter().enumerate() {
                // Barycentric weights of a and b over the triangle (c, a, b).
                let det = ((a[0] - c[0]) as f64) * ((b[1] - c[1]) as f64) - ((a[1] - c[1]) as f64) * ((b[0] - c[0]) as f64);
                if det == 0.0 {
                    continue;
                }
                let la = ((px - c[0] as f64) * (b[1] - c[1]) as f64 - (py - c[1] as f64) * (b[0] - c[0]) as f64) / det;
                let lb = ((a[0] - c[0]) as f64 * (py - c[1] as f64) - (a[1] - c[1]) as f64 * (px - c[0] as f64)) / det;
                let lc = 1.0 - la - lb;
                const E: f64 = 1e-9;
                if la < -E || lb < -E || lc < -E {
                    continue;
                }
                // The last radial edge (c → b of the last triangle, where la = 0) is outside, so
                // pieces meeting there don't overlap on PS2.
                if k == last && la <= E {
                    continue;
                }
                let u = lc * c[2] as f64 + la * a[2] as f64 + lb * b[2] as f64;
                let v = lc * c[3] as f64 + la * a[3] as f64 + lb * b[3] as f64;
                let mut t = src.at((u * src_scale as f64 / 16.0).floor() as i64, (v * src_scale as f64 / 16.0).floor() as i64);
                if soft {
                    // Gouraud alpha: 1 at c and a, 0 at b (the GS's vertex alpha × texel alpha / 128).
                    t[3] = ((t[3] as f64) * (1.0 - lb).clamp(0.0, 1.0)).round() as u8;
                }
                p.px[j * w as usize * s + i] = t;
                break;
            }
        }
    }
    p
}

/// The PC copy of a fan, drawn as a filtered renderer at the PC scale draws it: each texel is the
/// average of 4 × 4 points over its area, each point inside the fan sampling the PC picture
/// bilinearly (alpha-weighted). The fan's two radial cut edges (its start, and its end unless
/// it is KH1's soft end) reach half a texel past the line, so pieces that meet there overlap a
/// little instead of leaving a half-covered texel line. Owner, 2026-11-01: the gauge's edges
/// were "grainy" and the MP outline "line thickness varies" (single samples a texel: the corner
/// rule of 0.34.0 and point samples before it).
fn render_pc(tris: &[([i32; 4], [i32; 4], [i32; 4], bool)], last: usize, src: &Pic, src_scale: usize, s: usize, rect: (i32, i32, i32, i32), p: &mut Pic) {
    let (x0, y0, w, h) = rect;
    const N: usize = 4;
    // Half a PC texel, in 1/16 units.
    let tol = 8.0 / s as f64;
    let dist = |p: [i32; 4], q0: [i32; 4], q1: [i32; 4]| -> f64 {
        let (dx, dy) = ((q1[0] - q0[0]) as f64, (q1[1] - q0[1]) as f64);
        let l = (dx * dx + dy * dy).sqrt();
        if l == 0.0 {
            return 0.0;
        }
        ((p[0] - q0[0]) as f64 * dy - (p[1] - q0[1]) as f64 * dx).abs() / l
    };
    // Barycentric slack across the start edge (c → a of the first triangle: lb ≥ −eb) and the end
    // edge (c → b of the last: la ≥ −ea).
    let first = (0..tris.len()).find(|&k| {
        let t = &tris[k];
        (t.1[0] - t.0[0]) as i64 * (t.2[1] - t.0[1]) as i64 != (t.1[1] - t.0[1]) as i64 * (t.2[0] - t.0[0]) as i64
    });
    let slack: Vec<(f64, f64)> = tris
        .iter()
        .enumerate()
        .map(|(k, &(c, a, b, soft))| {
            let eb = if Some(k) == first { let d = dist(b, c, a); if d > 0.0 { tol / d } else { 0.0 } } else { 0.0 };
            let ea = if k == last && !soft { let d = dist(a, c, b); if d > 0.0 { tol / d } else { 0.0 } } else { 0.0 };
            (ea, eb)
        })
        .collect();
    let ww = w as usize * s;
    for j in 0..h as usize * s {
        for i in 0..ww {
            let mut acc = [0f64; 4];
            for sj in 0..N {
                for si in 0..N {
                    let px = (x0 as f64 + (i as f64 + (si as f64 + 0.5) / N as f64) / s as f64 - 0.5) * 16.0;
                    let py = (y0 as f64 + (j as f64 + (sj as f64 + 0.5) / N as f64) / s as f64 - 0.5) * 16.0;
                    for (k, &(c, a, b, soft)) in tris.iter().enumerate() {
                        let det = ((a[0] - c[0]) as f64) * ((b[1] - c[1]) as f64) - ((a[1] - c[1]) as f64) * ((b[0] - c[0]) as f64);
                        if det == 0.0 {
                            continue;
                        }
                        let la = ((px - c[0] as f64) * (b[1] - c[1]) as f64 - (py - c[1] as f64) * (b[0] - c[0]) as f64) / det;
                        let lb = ((a[0] - c[0]) as f64 * (py - c[1] as f64) - (a[1] - c[1]) as f64 * (px - c[0] as f64)) / det;
                        let lc = 1.0 - la - lb;
                        let (ea, eb) = slack[k];
                        const E: f64 = 1e-9;
                        if la < -E - ea || lb < -E - eb || lc < -E {
                            continue;
                        }
                        let u = lc * c[2] as f64 + la * a[2] as f64 + lb * b[2] as f64;
                        let v = lc * c[3] as f64 + la * a[3] as f64 + lb * b[3] as f64;
                        let t = src.bilinear(u * src_scale as f64 / 16.0 - 0.5, v * src_scale as f64 / 16.0 - 0.5);
                        let mut al = t[3] as f64;
                        if soft {
                            al *= (1.0 - lb).clamp(0.0, 1.0);
                        }
                        for ch in 0..3 {
                            acc[ch] += t[ch] as f64 * al;
                        }
                        acc[3] += al;
                        break;
                    }
                }
            }
            let n = (N * N) as f64;
            if acc[3] > 0.0 {
                p.px[j * ww + i] = [(acc[0] / acc[3]).round() as u8, (acc[1] / acc[3]).round() as u8, (acc[2] / acc[3]).round() as u8, (acc[3] / n).round().min(255.0) as u8];
            }
        }
    }
}

/// A baked arc piece: its screen rect (units) and its PS2 / PC pictures.
#[derive(Clone, Debug)]
pub struct Baked {
    /// x, y, w, h (units).
    pub rect: (i32, i32, i32, i32),
    /// The PS2 picture.
    pub ps2: Pic,
    /// The PC pictures (per language source).
    pub hd: Vec<Pic>,
}

/// Bakes the fan [a0, a1] of `ring` from `src` (per language for the PC pictures), trimmed to
/// what any of them shows. None when it shows nothing.
pub fn bake(ring: &Ring, a0: f32, a1: f32, src: &[&Source]) -> Option<Baked> {
    bake_soft(ring, a0, a1, false, src)
}

/// [`bake`] with KH1's soft end when `soft` (mode 4, see [`fan_soft`]).
pub fn bake_soft(ring: &Ring, a0: f32, a1: f32, soft: bool, src: &[&Source]) -> Option<Baked> {
    let (f, wedge) = fan_soft(ring, a0, a1, soft);
    let mut all = f.clone();
    if let Some(w) = &wedge {
        all.extend_from_slice(w);
    }
    let minx = all.iter().map(|v| v[0]).min().unwrap().div_euclid(16) - 1;
    let maxx = all.iter().map(|v| v[0]).max().unwrap().div_euclid(16) + 2;
    let miny = all.iter().map(|v| v[1]).min().unwrap().div_euclid(16) - 1;
    let maxy = all.iter().map(|v| v[1]).max().unwrap().div_euclid(16) + 2;
    let full = (minx, miny, maxx - minx, maxy - miny);
    let s = src[0].scale;
    let ps2 = render(&f, wedge.as_ref(), &src[0].ps2, 1, 1, full);
    let hds: Vec<Pic> = src.iter().map(|x| render(&f, wedge.as_ref(), &x.hd, x.scale, x.scale, full)).collect();
    // Trim to the units any picture shows.
    let (w, h) = (full.2 as usize, full.3 as usize);
    let mut used = vec![false; w * h];
    for j in 0..h {
        for i in 0..w {
            if ps2.px[j * w + i][3] != 0 {
                used[j * w + i] = true;
            }
        }
    }
    for hd in &hds {
        for j in 0..h * s {
            for i in 0..w * s {
                if hd.px[j * w * s + i][3] != 0 {
                    used[(j / s) * w + i / s] = true;
                }
            }
        }
    }
    let xs: Vec<usize> = (0..w).filter(|&i| (0..h).any(|j| used[j * w + i])).collect();
    let ys: Vec<usize> = (0..h).filter(|&j| (0..w).any(|i| used[j * w + i])).collect();
    let (&ix0, &ix1, &iy0, &iy1) = (xs.first()?, xs.last()?, ys.first()?, ys.last()?);
    let (tw, th) = (ix1 - ix0 + 1, iy1 - iy0 + 1);
    Some(Baked {
        rect: (full.0 + ix0 as i32, full.1 + iy0 as i32, tw as i32, th as i32),
        ps2: ps2.crop(ix0, iy0, tw, th),
        hd: hds.iter().map(|p| p.crop(ix0 * s, iy0 * s, tw * s, th * s)).collect(),
    })
}

// ---------------------------------------------------------------------------------------------
// The atlas.

/// A rect in an atlas: x, y, w, h.
type Slot = (usize, usize, usize, usize);

/// Shelf packing of (w, h) boxes into a power-of-two width; returns each box's place and the
/// atlas size (power-of-two sides).
fn pack(sizes: &[(usize, usize)], width: usize) -> (Vec<Slot>, usize, usize) {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(sizes[i].1), i));
    let mut out = vec![(0, 0, 0, 0); sizes.len()];
    let (mut x, mut y, mut shelf) = (0usize, 0usize, 0usize);
    for i in order {
        let (w, h) = sizes[i];
        if x + w > width {
            x = 0;
            y += shelf;
            shelf = 0;
        }
        out[i] = (x, y, w, h);
        x += w;
        shelf = shelf.max(h);
    }
    let height = (y + shelf).next_power_of_two().max(1);
    (out, width, height)
}

/// What a sprite shows: an atlas box, and the sub-rect of it in PS2 units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sub {
    item: usize,
    rect: [i32; 4],
}

/// The atlas items: each a PS2 picture and its PC pictures (one per language or one shared), at
/// a PC scale.
pub(crate) struct Atlas {
    items: Vec<(Pic, Vec<Pic>, usize)>,
}

impl Atlas {
    fn add(&mut self, ps2: Pic, hd: Vec<Pic>, scale: usize) -> usize {
        self.items.push((ps2, hd, scale));
        self.items.len() - 1
    }
}

/// KH1's PC textures for `uitex.bin` pictures are repacked in places: some pieces moved or were
/// cut down (picture 7's HP back segments are a 1-texel column there, picture 5's straight HP
/// segment is 24 texels, not 26), while `g_s` keeps the PS2 UVs (Checked (files):
/// `UK_uitex_bin10780.dds`, `-a2c0.dds` against `uitex.bin` pictures 7 and 5). How KH1 PC maps
/// those UVs onto its repacked textures is not read (Assumption: a remap we have not found). So
/// for every texel rect a piece draws from a picture taken whole (`raw`; the baked arcs come from
/// the PC rings, which keep the PS2 layout), the PC texture's coverage is averaged down to one
/// per PS2 texel; where more than 1 in 20 texels the PS2 picture covers are clear on PC, that
/// rect of the PC copy is replaced by the PS2 texels at the PC scale (nearest). Only coverage
/// lost counts: the PC redraws colours and edges (faces, labels) and widens some art inside its
/// rect (picture 2's panels fill the `com` rects, the PS2 ones stop short). Returns the rects
/// replaced (`KH1_HUD_REFIT_LOG` set: each one to stderr).
fn fit_layouts(atlas: &mut Atlas, groups: &[Group], raw: &[usize]) -> usize {
    use std::collections::BTreeSet;
    let mut used: BTreeSet<(usize, [i32; 4])> = BTreeSet::new();
    for g in groups {
        for st in &g.steps {
            for p in st.pieces.iter().filter(|p| raw.contains(&p.item)) {
                used.insert((p.item, p.uv));
            }
        }
    }
    let log = std::env::var("KH1_HUD_REFIT_LOG").is_ok();
    let mut fixed = 0;
    for (item, [u0, v0, u1, v1]) in used {
        let (ps2, hds, s) = &mut atlas.items[item];
        let s = *s;
        let (w, h) = ((u1 - u0).max(0) as usize, (v1 - v0).max(0) as usize);
        if w == 0 || h == 0 || u0 < 0 || v0 < 0 || u1 as usize > ps2.w || v1 as usize > ps2.h {
            continue;
        }
        let (u0, v0) = (u0 as usize, v0 as usize);
        for hd in hds.iter_mut() {
            if hd.w < ps2.w * s || hd.h < ps2.h * s {
                continue;
            }
            let mut lost = 0usize;
            for j in 0..h {
                for i in 0..w {
                    let (x, y) = (u0 + i, v0 + j);
                    let mut cov = 0u32;
                    for dy in 0..s {
                        for dx in 0..s {
                            cov += hd.px[(y * s + dy) * hd.w + x * s + dx][3] as u32;
                        }
                    }
                    // Alpha 0..128.
                    if ps2.px[y * ps2.w + x][3] >= 96 && cov / ((s * s) as u32) < 32 {
                        lost += 1;
                    }
                }
            }
            if lost * 20 > w * h {
                for j in 0..h * s {
                    for i in 0..w * s {
                        hd.px[(v0 * s + j) * hd.w + u0 * s + i] = ps2.px[(v0 + j / s) * ps2.w + u0 + i / s];
                    }
                }
                fixed += 1;
                if log {
                    eprintln!("kh1hud refit: item {item} texels {:?}, {lost} of {} lost on PC", [u0, v0, u0 + w, v0 + h], w * h);
                }
            }
        }
    }
    fixed
}

/// Gives every texel rect drawn from a whole picture (`raw`) an atlas box of its own, so the
/// atlas's edge copies (see [`build_atlas`]) sit around exactly what each piece shows. With the
/// rect inside a whole picture, the PC atlas's filtering mixed each piece's edge with the
/// texels beside it there (mostly clear), which showed as thin seams where pieces meet (the
/// straight bars' segments, the rings' quarters; owner: "the bars are all choppy",
/// 2026-11-01). Pictures no piece draws from any more become 1 × 1. Returns the boxes made.
fn split_raw(atlas: &mut Atlas, groups: &mut [Group], raw: &[usize]) -> usize {
    use std::collections::BTreeMap;
    let mut made: BTreeMap<(usize, [i32; 4]), usize> = BTreeMap::new();
    for g in groups.iter_mut() {
        for st in g.steps.iter_mut() {
            for p in st.pieces.iter_mut().filter(|p| raw.contains(&p.item)) {
                let key = (p.item, p.uv);
                let item = match made.get(&key) {
                    Some(&i) => i,
                    None => {
                        let [u0, v0, u1, v1] = p.uv;
                        let (w, h) = ((u1 - u0).max(1) as usize, (v1 - v0).max(1) as usize);
                        let (u0, v0) = (u0.max(0) as usize, v0.max(0) as usize);
                        let (ps2, hds, s) = &atlas.items[p.item];
                        let s = *s;
                        let ps2c = ps2.crop(u0, v0, w, h);
                        let hdc: Vec<Pic> = hds.iter().map(|hd| hd.crop(u0 * s, v0 * s, w * s, h * s)).collect();
                        let i = atlas.add(ps2c, hdc, s);
                        made.insert(key, i);
                        i
                    }
                };
                p.item = item;
                let [u0, v0, u1, v1] = p.uv;
                p.uv = [0, 0, (u1 - u0).max(1), (v1 - v0).max(1)];
            }
        }
    }
    for &r in raw {
        let s = atlas.items[r].2;
        let n = atlas.items[r].1.len();
        atlas.items[r] = (Pic::blank(1, 1), vec![Pic::blank(s, s); n], s);
    }
    made.len()
}

/// The built atlases: the PS2 one and the PC one per language, and each item's places.
struct Packed {
    ps2: Pic,
    hd: Vec<Pic>,
    ps2_slots: Vec<Slot>,
    hd_slots: Vec<Slot>,
    scales: Vec<usize>,
}

fn build_atlas(a: &Atlas, langs: usize) -> Result<Packed> {
    let (ps2_slots, pw, ph) = pack(&a.items.iter().map(|i| (i.0.w + 2, i.0.h + 2)).collect::<Vec<_>>(), 1024);
    let (hd_slots, hw, hh) = pack(&a.items.iter().map(|i| (i.0.w * i.2 + 4, i.0.h * i.2 + 4)).collect::<Vec<_>>(), 2048);
    if ph > 2048 || hh > 4096 {
        return bad(format!("kh1hud atlas too big ({pw}×{ph}, PC {hw}×{hh})"));
    }
    let mut ps2 = Pic::blank(pw, ph);
    let mut hd = vec![Pic::blank(hw, hh); langs];
    for (k, (p, h, _)) in a.items.iter().enumerate() {
        // One texel of gap around each box (the PC atlas is filtered), holding copies of the
        // box's edge texels so filtering at a piece's edge never mixes in a neighbour or clear
        // texels (pieces that meet join without a seam).
        ps2.blit_padded(&p.bled(2), ps2_slots[k].0 + 1, ps2_slots[k].1 + 1, 1);
        let hb: Vec<Pic> = h.iter().map(|x| x.bled(4)).collect();
        for (l, atlas) in hd.iter_mut().enumerate() {
            let pic = if hb.len() == 1 { &hb[0] } else { &hb[l] };
            atlas.blit_padded(pic, hd_slots[k].0 + 2, hd_slots[k].1 + 2, 2);
        }
    }
    let ps2_slots = ps2_slots.iter().map(|s| (s.0 + 1, s.1 + 1, s.2 - 2, s.3 - 2)).collect();
    let hd_slots = hd_slots.iter().map(|s| (s.0 + 2, s.1 + 2, s.2 - 4, s.3 - 4)).collect();
    Ok(Packed { ps2, hd, ps2_slots, hd_slots, scales: a.items.iter().map(|i| i.2).collect() })
}

impl Packed {
    /// A sprite's UV rect (exclusive end) in the PS2 atlas and in the PC one.
    fn uv(&self, s: Sub) -> ([i32; 4], [i32; 4]) {
        let (px, py, ..) = self.ps2_slots[s.item];
        let (hx, hy, ..) = self.hd_slots[s.item];
        let k = self.scales[s.item] as i32;
        let [u0, v0, u1, v1] = s.rect;
        ([px as i32 + u0, py as i32 + v0, px as i32 + u1, py as i32 + v1], [hx as i32 + u0 * k, hy as i32 + v0 * k, hx as i32 + u1 * k, hy as i32 + v1 * k])
    }
}

// ---------------------------------------------------------------------------------------------
// Sprite set: pieces shown by frame.

/// A piece: what it shows, where (part rect, KH1 units; x1 < x0 / y1 < y0 mirror) and its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Piece {
    item: usize,
    uv: [i32; 4],
    rect: [i32; 4],
    colour: u32,
    /// The bottom vertices' colour when not 0 (KH1's 4-vertex-colour objects, flag 0x20, drawn as
    /// a vertical gradient: their top and bottom pairs are equal in the files used).
    grad: u32,
}

/// One animation of a group: its pieces over frames [f0, f1] with a colour start → end, a
/// translation (x0, y0) → (x1, y1) (KH1 units) and a constant z rotation `rot` (radians, as the
/// file stores it: Re:CoM negates it on load, `0x14024B4E0`, so a positive value turns the pieces
/// counter-clockwise on screen about the translated origin; `ring`).
#[derive(Clone, Default)]
pub(crate) struct Step {
    pieces: Vec<Piece>,
    frames: (i32, i32),
    colour: (u32, u32),
    flags: u32,
    tr: [i32; 4],
    rot: f32,
}

pub(crate) struct Group {
    steps: Vec<Step>,
    once: bool,
    word: i16,
}

/// Compiles a held group from the pieces each frame shows: one animation per piece and run of
/// frames (pieces sharing their runs share a sprite group).
fn held(frames: &[Vec<Piece>]) -> Group {
    use std::collections::BTreeMap;
    let mut runs: BTreeMap<Piece, Vec<(i32, i32)>> = BTreeMap::new();
    for (f, ps) in frames.iter().enumerate() {
        for p in ps {
            let r = runs.entry(*p).or_default();
            match r.last_mut() {
                Some(last) if last.1 + 1 == f as i32 => last.1 = f as i32,
                _ => r.push((f as i32, f as i32)),
            }
        }
    }
    // The last frame's pieces stay for any larger frame.
    let lastf = frames.len() as i32 - 1;
    let mut by_runs: BTreeMap<Vec<(i32, i32)>, Vec<Piece>> = BTreeMap::new();
    for (p, mut r) in runs {
        if let Some(l) = r.last_mut() {
            if l.1 == lastf {
                l.1 = LONG;
            }
        }
        by_runs.entry(r).or_default().push(p);
    }
    let one = 0x8080_8080;
    let mut steps = Vec::new();
    for (r, ps) in by_runs {
        for fr in r {
            steps.push(Step { pieces: ps.clone(), frames: fr, colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 });
        }
    }
    Group { steps, once: true, word: GROUP_WORD_SCRUB }
}

/// The SEQD of a set of groups, with the UVs of the PS2 atlas or the PC one.
fn seqd(groups: &[Group], packed: &Packed, hd: bool) -> Result<Vec<u8>> {
    let mut s = Seqd { version: 0x160, sprites: Vec::new(), parts: Vec::new(), sgroups: Vec::new(), anims: Vec::new(), agroups: Vec::new() };
    use std::collections::HashMap;
    let mut spr: HashMap<([i32; 4], u32, u32), usize> = HashMap::new();
    let mut sg_of: HashMap<Vec<Piece>, usize> = HashMap::new();
    for g in groups {
        let a0 = s.anims.len();
        for st in &g.steps {
            let key = st.pieces.clone();
            let sg = match sg_of.get(&key) {
                Some(&i) => i,
                None => {
                    let p0 = s.parts.len();
                    for p in &st.pieces {
                        // A scissor part (`ring`): a sprite whose UVs are all 0, alone in its
                        // sprite group (Re:CoM's `CScissoring`, `0x14024B910`).
                        let uv = if p.item == ring::SCISSOR {
                            [0; 4]
                        } else {
                            let (b, h) = packed.uv(Sub { item: p.item, rect: p.uv });
                            if hd { h } else { b }
                        };
                        let g = if p.grad != 0 { p.grad } else { p.colour };
                        let si = *spr.entry((uv, p.colour, g)).or_insert_with(|| {
                            // Re:CoM's sprite colours: TL, TR, BL, BR (`mp_bar` assumption 3).
                            s.sprites.push(sprite(uv, [p.colour, p.colour, g, g]));
                            s.sprites.len() - 1
                        });
                        s.parts.push(part(p.rect, si));
                    }
                    s.sgroups.push(sgroup(p0, st.pieces.len()));
                    sg_of.insert(key, s.sgroups.len() - 1);
                    s.sgroups.len() - 1
                }
            };
            let mut a = anim(st.flags, sg, st.frames, 0, (1.0, 1.0), st.colour);
            // Translation start / end (`+0x10` x0, `+0x14` x1, `+0x18` y0, `+0x1C` y1, ints in 2D
            // units: `0x14024B4E0` reads them as (x0, y0) → `+0xD0`, (x1, y1) → `+0xD8`; Checked
            // (decomp)).
            let [x0, y0, x1, y1] = st.tr;
            for (at, v) in [(0x10, x0), (0x14, x1), (0x18, y0), (0x1C, y1)] {
                a[at..at + 4].copy_from_slice(&v.to_le_bytes());
            }
            // z rotation start / end (`+0x40`, `+0x44`; `0x14024B4E0` → `+0x58` / `+0x68`).
            if st.rot != 0.0 {
                a[0x40..0x44].copy_from_slice(&st.rot.to_le_bytes());
                a[0x44..0x48].copy_from_slice(&st.rot.to_le_bytes());
            }
            s.anims.push(a);
        }
        s.agroups.push(agroup(a0, g.steps.len(), g.once, g.word));
    }
    // Re:CoM reads an animation group's first animation and count, and a sprite group's first
    // part, as unsigned 16-bit (`0x140248F40`: `*param_4`, `param_4[1]`; `0x140249150`'s part
    // pointer from `*(ushort *)(sgroup)`); Checked (decomp). Parts stay below 0x8000 (our sprite
    // group writer stores the first part as i16).
    if s.anims.len() > u16::MAX as usize || s.parts.len() > i16::MAX as usize || s.sgroups.len() > i16::MAX as usize {
        return bad(format!("kh1hud: too many records for 16-bit indices ({} animations, {} parts, {} sprite groups)", s.anims.len(), s.parts.len(), s.sgroups.len()));
    }
    Ok(s.write())
}

/// A LAYD holding one SEQD (layout tables: one empty entry each, as Re:CoM's registration
/// `0x140244AD0` only stores them).
pub fn layd(seq: &[u8]) -> Vec<u8> {
    let mut d = b"LAYD".to_vec();
    let l1 = 0x20u32;
    let l2 = l1 + 0x18;
    let st = l2 + 0x14;
    let so = (st + 4 + 15) & !15;
    for v in [0x100u32, 1, l1, 1, l2, 1, st] {
        d.extend(v.to_le_bytes());
    }
    d.extend([0u8; 0x18]);
    let mut l2e = vec![0u8; 0x14];
    l2e[2..4].copy_from_slice(&1i16.to_le_bytes());
    d.extend(l2e);
    d.extend(so.to_le_bytes());
    d.resize(so as usize, 0);
    d.extend_from_slice(seq);
    d
}

/// An IMGZ of one 32-bit IMGD (PSMCT32, linear, PS2 alpha): the header Re:CoM's own PC atlas
/// `UK_Cockpit.imz` has (Checked (files): every word but the sizes).
pub fn imgz(p: &Pic) -> Vec<u8> {
    let (w, h) = (p.w, p.h);
    let len = (w * h * 4) as u32;
    let mut g = b"IMGD".to_vec();
    for v in [0x100u32, 0x40, len, 0x40 + len, 0, 0xFFFF_FFFF] {
        g.extend(v.to_le_bytes());
    }
    for v in [w as u16, h as u16, w.trailing_zeros() as u16, h.trailing_zeros() as u16, (w / 64).max(1) as u16, 0] {
        g.extend(v.to_le_bytes());
    }
    g.extend(0xFFFF_FFFFu32.to_le_bytes());
    for v in [0x10u16, 0x10, 1, 0x13, 3, 0] {
        g.extend(v.to_le_bytes());
    }
    g.extend(0u32.to_le_bytes());
    g.extend(3u32.to_le_bytes());
    debug_assert_eq!(g.len(), 0x40);
    for px in &p.px {
        g.extend_from_slice(px);
    }
    let mut z = b"IMGZ".to_vec();
    for v in [0x100u32, 0x10, 1, 0x20, g.len() as u32, 0, 0] {
        z.extend(v.to_le_bytes());
    }
    z.extend(g);
    z
}

// ---------------------------------------------------------------------------------------------
// The build.

/// Bakes and caches arc pieces into the atlas.
struct Baker {
    centre: (i32, i32),
    src5: Vec<Source>,
    src7: Vec<Source>,
    p7: usize,
    /// A 1 × 1 transparent item, for a step that shows nothing.
    empty: usize,
    cache: std::collections::BTreeMap<(i32, i32, i32, i32, i32, bool), Option<(usize, (i32, i32, i32, i32))>>,
}

impl Baker {
    /// Picture 5 itself as an atlas item (the party's full quarter rings are its texels).
    fn src5_item(&mut self, atlas: &mut Atlas, ps2: &Source, per: &[Pic]) -> usize {
        atlas.add(ps2.ps2.clone(), vec![per[0].clone()], ps2.scale)
    }

    /// The piece of the fan [a0, a1] of ring `r` (picture 5, or picture 7 per language).
    fn piece(&mut self, atlas: &mut Atlas, r: (i32, i32, i32), p: usize, a0: i32, a1: i32) -> Piece {
        self.piece_soft(atlas, r, p, a0, a1, false)
    }

    /// [`Baker::piece`] with KH1's soft end (mode 4) when `soft` and 1 < a1 < 270.
    fn piece_soft(&mut self, atlas: &mut Atlas, r: (i32, i32, i32), p: usize, a0: i32, a1: i32, soft: bool) -> Piece {
        let soft = soft && a1 > 1 && a1 < 270;
        let ring = Ring { x: self.centre.0, y: self.centre.1, uc: r.0, vc: r.1, r: r.2 };
        let got = *self.cache.entry((r.0, r.1, r.2, a0, a1, soft)).or_insert_with(|| {
            let s: Vec<&Source> = if p == self.p7 { self.src7.iter().collect() } else { self.src5.iter().take(1).collect() };
            bake_soft(&ring, a0 as f32, a1 as f32, soft, &s).map(|b| {
                let hd = if p == self.p7 { b.hd } else { vec![b.hd[0].clone()] };
                (atlas.add(b.ps2, hd, s[0].scale), b.rect)
            })
        });
        match got {
            Some((it, (x, y, w, h))) => Piece { item: it, uv: [0, 0, w, h], rect: [x, y, x + w, y + h], colour: 0x8080_8080, grad: 0 },
            None => Piece { item: self.empty, uv: [0, 0, 1, 1], rect: [0, 0, 1, 1], colour: 0, grad: 0 },
        }
    }

    /// The arc [0, a] as pieces cut every `step` degrees (KH1 draws one fan; the union of the
    /// cut fans is the same polygon, its boundary points lying on the same square edges).
    fn arc(&mut self, atlas: &mut Atlas, r: (i32, i32, i32), p: usize, a: i32, step: i32) -> Vec<Piece> {
        self.arc_soft(atlas, r, p, a, step, false)
    }

    /// [`Baker::arc`] whose last piece has KH1's soft end (mode 4) when `soft`.
    fn arc_soft(&mut self, atlas: &mut Atlas, r: (i32, i32, i32), p: usize, a: i32, step: i32, soft: bool) -> Vec<Piece> {
        let mut v = Vec::new();
        let mut q0 = 0;
        while q0 < a {
            let q1 = (q0 + step - q0 % step).min(a);
            let pc = self.piece_soft(atlas, r, p, q0, q1, soft && q1 == a);
            if pc.item != self.empty {
                v.push(pc);
            }
            q0 = q1;
        }
        v
    }
}

/// What the step reads.
pub struct Sources<'a> {
    /// `command2/uitex.bin` and its PC assets.
    pub uitex: &'a [u8],
    /// Its PC assets.
    pub uitex_hd: &'a [HdAsset],
    /// `exchange/UK_uibin_g_s.bin`, `…_g_red.bin`.
    pub g_s: &'a [u8],
    /// `g_red`.
    pub g_red: &'a [u8],
    /// `g_act` (the gauge motions).
    pub g_act: &'a [u8],
    /// H4: `com`, `com_battle`, `mag` and the menu font; the command words per language (as the
    /// command menu decodes them). Without them no menu look is built.
    pub menu: Option<MenuSources<'a>>,
    /// H5: `lock` and `g_enm`.
    pub lock: Option<&'a [u8]>,
    /// `g_enm`.
    pub g_enm: Option<&'a [u8]>,
    /// H6: `g_c`.
    pub g_c: Option<&'a [u8]>,
    /// H8: `lev_s`.
    pub lev_s: Option<&'a [u8]>,
    /// The three faces (Sora, Donald, Goofy).
    pub faces: [Source; 3],
}

/// The command menu's inputs (H4).
pub struct MenuSources<'a> {
    /// `com`.
    pub com: &'a [u8],
    /// `com_battle`.
    pub com_battle: &'a [u8],
    /// `mag`.
    pub mag: &'a [u8],
    /// The system font's PC texture (`US_sysfont_bin0.dds`) and glyph table.
    pub font: [&'a [u8]; 2],
    /// (language, [(message id, word)]) as `command_menu::words` decodes them.
    pub words: Vec<(String, Vec<(u16, String)>)>,
    /// The PC's button icons (`Icon.tm2`, `FontIcon.inf`), for the context tab's Triangle. Without
    /// them the Triangle's group is empty.
    pub icon: Option<[&'a [u8]; 2]>,
    /// Stage 7.12: `UK_uibin_cut.bin`, the shortcut panel. Without it no panel is built.
    pub cut: Option<&'a [u8]>,
    /// M8 stage 8a.1: `UK_uibin_item.bin`, the Items list. Without it no Items list is built (the
    /// DLL then draws it with the Magic list's pieces).
    pub item: Option<&'a [u8]>,
    /// M8b: `UK_uibin_sum.bin`, the Summon list ([`SUM`]); same rule.
    pub sum: Option<&'a [u8]>,
    /// 0.179.1: Re:CoM's `FO0104.RTM` level-up texture ([`menu::DP_EMBLEM_ASSET`]), for the DP
    /// sleights' mark. Without it no mark is built.
    pub dp_emblem: Option<&'a [u8]>,
}

/// The built files.
pub struct Built {
    /// `kh1hud.lad` (PS2 atlas UVs) and the PC copy (PC atlas UVs, for every language).
    pub lad: Vec<u8>,
    /// The PC copy.
    pub lad_hd: Vec<u8>,
    /// `kh1hud.imz` and `<LANG>_kh1hud.imz` (in `langs` order).
    pub imz: Vec<u8>,
    /// The PC atlases.
    pub imz_hd: Vec<Vec<u8>>,
    /// Sizes for the log.
    pub atlas: (usize, usize, usize, usize),
    /// Atlas items.
    pub pieces: usize,
    /// Sprites.
    pub sprites: usize,
    /// The motion programs and units (H2b), for the manifest.
    pub motion: motion::Motion,
    /// Groups in all.
    pub groups: usize,
    /// H4: the menu groups (None without menu sources).
    pub menu: Option<menu::Built>,
    /// Stage 7.12: the shortcut panel's groups (None without `UK_uibin_cut.bin` or the menu).
    pub shortcut: Option<shortcut::Built>,
    /// H5: the target groups' first group.
    pub target: Option<usize>,
    /// H6: Donald's and Goofy's gauge groups.
    pub party: Option<[usize; 2]>,
    /// H7: the lock-on cursor's group.
    pub cursor: Option<usize>,
    /// KH1's orange auto-target ring's group (`lockon_d`, 0.180.0).
    pub auto_ring: Option<usize>,
    /// H8: the LEVEL UP!! box's group.
    pub level_up: Option<usize>,
    /// PC texture rects redrawn from the PS2 pictures (repacked on PC, see [`fit_layouts`]).
    pub refit: usize,
    /// KH1's MP ring as Re:CoM's HP ring: its first group ([`ring_group`]'s order) and what its
    /// cuts resolved.
    pub ring: (usize, CutStats),
}

fn colour_of(o: &Obj) -> u32 {
    let c = o.colours[0];
    let b = |v: i32| v.clamp(0, 255) as u32;
    b(c[0]) | b(c[1]) << 8 | b(c[2]) << 16 | b(c[3]) << 24
}

/// The piece an object draws at (dx, dy), with KH1's partial cut (`x += cut`, `w −= cut`, UV kept:
/// `0x1402698C0`, `0x14026BEE0`, `0x14026A970`) and an x override (the caps).
fn obj_piece(o: &Obj, item: usize, dx: i32, dy: i32, cut: i32, x: Option<i32>) -> Piece {
    let (uv, mu, mv) = o.texels();
    let [ox, oy, w, h] = o.rect;
    let x0 = x.unwrap_or(ox) + cut + dx;
    let (x1, y0, y1) = (x0 + w - cut, oy + dy, oy + dy + h);
    let rect = [if mu { x1 } else { x0 }, if mv { y1 } else { y0 }, if mu { x0 } else { x1 }, if mv { y0 } else { y1 }];
    Piece { item, uv, rect, colour: colour_of(o), grad: 0 }
}

/// KH1's cut of a partial straight segment: round((1 − rem / per) · width), at least 1, at most
/// width − 1 (`0x1402698C0`: f32 + 0.5, truncated).
pub fn cut(rem: i32, per: i32, width: i32) -> i32 {
    let c = ((1.0f32 - rem as f32 / per as f32) * width as f32 + 0.5) as i32;
    c.max(1).min(width - 1)
}

/// The straight pieces of HP-like rows (`0x1402698C0` back, `0x14026BEE0` colour): `n` full
/// segments of `obj` (the first of `first` for the back) at (−1 − 26 i, +1), then the partial one.
/// Returns (piece list, the x offset after them and the last cut) for the cap.
fn row(objs: &[Obj], item: usize, first: usize, rest: usize, units: i32, per: i32) -> (Vec<Piece>, i32, i32) {
    let n = units / per;
    let rem = units % per;
    let mut v = Vec::new();
    let mut dx = -1;
    for i in 0..n {
        let (o, x) = if i == 0 { (first, None) } else { (rest, Some(objs[rest].rect[0] + kh1::PITCH)) };
        let x = if first == rest { None } else { x };
        v.push(obj_piece(&objs[o], item, dx, 1, 0, x));
        dx -= kh1::PITCH;
    }
    let mut c = kh1::PITCH;
    if rem != 0 {
        c = cut(rem, per, kh1::PITCH);
        let (o, x) = if n == 0 || first == rest { (first, None) } else { (rest, Some(objs[rest].rect[0] + kh1::PITCH)) };
        v.push(obj_piece(&objs[o], item, dx, 1, c, x));
    }
    (v, dx, c)
}

/// Builds the set.
pub fn build(src: &Sources, langs: &[&str]) -> Result<Built> {
    if langs.is_empty() {
        return bad("kh1hud: no languages");
    }
    let g_s = Uibin::parse(src.g_s)?;
    if g_s.objs.len() != obj::COUNT || g_s.objs[obj::SEG_HP].name != "g_s_bar_e_hp" || g_s.objs[obj::FACE_FRAME[0]].name != "g_s_fra_a_wa" {
        return bad(format!("{G_S}: {} objects, not KH1 FM Steam's 39 (g_s_fra_a_wa … g_s_par_d_mp)", g_s.objs.len()));
    }
    let g_red = Uibin::parse(src.g_red)?;
    let red = g_red.find("g_s_bar_b_hp_red").ok_or_else(|| crate::Error::Format(format!("{G_RED}: no g_s_bar_b_hp_red")))?;
    if g_red.keys_a.len() < 3 || red.keys_a != (0, 3) {
        return bad(format!("{G_RED}: the flash keys are not KH1 FM Steam's (3 colour keys)"));
    }
    let objs = &g_s.objs;
    // Pictures 5, 6, 7 (PS2 and PC per language).
    let tims = mb::tim2_pictures(src.uitex)?;
    let mut pics: Vec<Source> = Vec::new();
    let mut pic_langs: Vec<Vec<Pic>> = Vec::new();
    for (i, off, data) in PICTURES {
        let t = tims.get(i).ok_or_else(|| crate::Error::Format(format!("uitex.bin: no picture {i}")))?;
        if t.offset != off || t.width != 128 || t.height != 128 {
            return bad(format!("uitex.bin: picture {i} at {:#x} {}×{}, KH1 draws it from {off:#x} 128×128", t.offset, t.width, t.height));
        }
        let ps2 = Pic { w: 128, h: 128, px: t.pixels.clone() };
        let mut per = Vec::new();
        for l in langs {
            let hd = uitex_asset(src.uitex_hd, data, l)?.ok_or_else(|| crate::Error::Format(format!("uitex.bin: no PC texture for picture {i} (-{data:x}.dds)")))?;
            per.push(hd);
        }
        let per = fit_all(&ps2, per, HD_SCALE);
        let scale = hd_scale(&ps2, &per[0])?;
        if per.iter().any(|p| p.w != per[0].w || p.h != per[0].h) {
            return bad(format!("uitex.bin: picture {i}'s language textures differ in size"));
        }
        // Only the PS2 picture's area of the PC texture (picture 2's carries extra rows on PC).
        let per: Vec<Pic> = per.into_iter().map(|p| p.crop(0, 0, 128 * scale, 128 * scale)).collect();
        pics.push(Source { ps2: ps2.clone(), hd: per[0].clone(), scale });
        pic_langs.push(per);
    }
    let (p5, p6, p7) = (0usize, 1usize, 2usize);
    let mut atlas = Atlas { items: Vec::new() };
    let it5 = atlas.add(pics[p5].ps2.clone(), pic_langs[p5].clone(), pics[p5].scale);
    let it6 = atlas.add(pics[p6].ps2.clone(), pic_langs[p6].clone(), pics[p6].scale);
    let it7 = atlas.add(pics[p7].ps2.clone(), pic_langs[p7].clone(), pics[p7].scale);
    let faces: Vec<usize> = src.faces.iter().map(|f| atlas.add(f.ps2.clone(), vec![f.hd.clone()], f.scale)).collect();
    let (cx, cy) = kh1::CENTRE;
    let per_lang_src = |p: usize| -> Vec<Source> { pic_langs[p].iter().map(|hd| Source { ps2: pics[p].ps2.clone(), hd: hd.clone(), scale: pics[p].scale }).collect() };
    let empty = atlas.add(Pic::blank(1, 1), vec![Pic::blank(2, 2)], 2);
    let mut bk = Baker { centre: (cx * 16, cy * 16), src5: per_lang_src(p5), src7: per_lang_src(p7), p7, empty, cache: Default::default() };
    let mut groups: Vec<Group> = Vec::new();
    // FRAME: picture 6's face frame, picture 7's disc and labels.
    let frame_pieces = |mp: bool| -> Vec<Piece> {
        let mut v: Vec<Piece> = obj::FACE_FRAME.iter().map(|&i| obj_piece(&objs[i], it6, 0, 0, 0, None)).collect();
        v.extend(obj::DISC.iter().map(|&i| obj_piece(&objs[i], it7, 0, 0, 0, None)));
        v.push(obj_piece(&objs[obj::LABEL_HP], it7, 0, 0, 0, None));
        if mp {
            v.push(obj_piece(&objs[obj::LABEL_MP], it7, 0, 0, 0, None));
        }
        v
    };
    groups.push(held(&[frame_pieces(false), frame_pieces(true)]));
    // HP_BACK: the back arc to min(max, 120) · 5 (≤ 270), then from 54 the straight back and its cap.
    let mut f = Vec::new();
    for m in 0..=kh1::HP_CAP {
        let mut v = if m > 0 { bk.arc(&mut atlas, kh1::RING_HP_BACK, p7, (m * kh1::HP_DEG).min(270), 90) } else { Vec::new() };
        if m >= kh1::HP_RING {
            let (segs, dx, c) = row(objs, it7, obj::BACK_HP_FIRST, obj::BACK_HP, m - kh1::HP_RING, kh1::HP_PER_SEG);
            v.extend(segs);
            v.push(obj_piece(&objs[obj::CAP_HP], it7, dx + c, 1, 0, Some(kh1::HP_CAP_X)));
        }
        f.push(v);
    }
    groups.push(held(&f));
    // MP_BACK: to min(max MP, 20) · 30 (≤ 270), then from 9 a back segment a point and the cap.
    let mut f = Vec::new();
    for m in 0..=kh1::MP_CAP {
        let mut v = if m > 0 { bk.arc(&mut atlas, kh1::RING_MP_BACK, p7, (m * kh1::MP_DEG).min(270), 90) } else { Vec::new() };
        if m >= kh1::MP_RING {
            let mut dx = -1;
            for _ in 0..m - kh1::MP_RING {
                v.push(obj_piece(&objs[obj::BACK_MP], it7, dx, 1, 0, None));
                dx -= kh1::PITCH;
            }
            v.push(obj_piece(&objs[obj::CAP_MP], it7, dx, 1, 0, Some(kh1::MP_CAP_X)));
        }
        f.push(v);
    }
    groups.push(held(&f));
    // MP: cells [30 j, 30 j + 30] for j < min(MP, 9), then a segment a point past 9.
    let cells: Vec<Piece> = (0..kh1::MP_RING).map(|j| bk.piece(&mut atlas, kh1::RING_MP, p5, 30 * j, 30 * j + 30)).collect();
    let mut f = Vec::new();
    for m in 0..=kh1::MP_CAP {
        let mut v: Vec<Piece> = cells.iter().take(m.min(kh1::MP_RING) as usize).filter(|p| p.item != bk.empty).copied().collect();
        let mut dx = -1;
        for _ in kh1::MP_RING..m {
            v.push(obj_piece(&objs[obj::SEG_MP], it5, dx, 1, 0, None));
            dx -= kh1::PITCH;
        }
        f.push(v);
    }
    groups.push(held(&f));
    // HP: the arc to min(HP, 54) · 5, then 6 HP a segment (partial last), at most 120.
    let mut f = Vec::new();
    for h in 0..=kh1::HP_CAP {
        let mut v = if h > 0 { bk.arc_soft(&mut atlas, kh1::RING_HP, p5, (h * kh1::HP_DEG).min(270), 90, true) } else { Vec::new() };
        if h > kh1::HP_RING {
            v.extend(row(objs, it5, obj::SEG_HP, obj::SEG_HP, h - kh1::HP_RING, kh1::HP_PER_SEG).0);
        }
        f.push(v);
    }
    groups.push(held(&f));
    // CHARGE: cells and the partial cell to min(c, 270), then 30 units a segment at (−26 i, +1)
    // with the partial one cut by round((1 − rem / 30) · 24).
    let seg_w = objs[obj::SEG_CHARGE].rect[2];
    let mut f = Vec::new();
    for c in 0..=kh1::CHARGE_CAP {
        let mut v = if c > 0 { bk.arc_soft(&mut atlas, kh1::RING_CHARGE, p5, c.min(270), 30, true) } else { Vec::new() };
        if c > kh1::CHARGE_RING {
            let (n, rem) = ((c - kh1::CHARGE_RING) / kh1::CHARGE_PER_SEG, (c - kh1::CHARGE_RING) % kh1::CHARGE_PER_SEG);
            let mut dx = 0;
            for _ in 0..n {
                v.push(obj_piece(&objs[obj::SEG_CHARGE], it5, dx, 1, 0, None));
                dx -= kh1::PITCH;
            }
            if rem != 0 {
                v.push(obj_piece(&objs[obj::SEG_CHARGE], it5, dx, 1, cut(rem, kh1::CHARGE_PER_SEG, seg_w), None));
            }
        }
        f.push(v);
    }
    groups.push(held(&f));
    // TICKS: picture 7's ticks past 90° / 180° of the max MP arc.
    let f: Vec<Vec<Piece>> = (0..=kh1::MP_CAP)
        .map(|m| {
            let mut v = Vec::new();
            if m * 30 > 90 {
                v.push(obj_piece(&objs[obj::TICK_90], it7, 0, 0, 0, None));
            }
            if m * 30 > 180 {
                v.push(obj_piece(&objs[obj::TICK_180], it7, 0, 0, 0, None));
            }
            v
        })
        .collect();
    groups.push(held(&f));
    // FACE: Sora's strip, cell = expression, h × h centred on the gauge centre (`0x14027F470`).
    let fh = src.faces[0].ps2.h as i32;
    let face_piece = |e: i32| Piece { item: faces[0], uv: [fh * e, 0, fh * (e + 1), fh], rect: [cx - fh / 2, cy - fh / 2, cx + fh / 2, cy + fh / 2], colour: 0x8080_8080, grad: 0 };
    groups.push(held(&[vec![face_piece(0)], vec![face_piece(1)], vec![face_piece(2)]]));
    let one = 0x8080_8080u32;
    // H2b: KH1's gauge motions (`g_act`) and the held groups' units.
    let g_act = Uibin::parse(src.g_act)?;
    let tracks = motion::tracks(&g_act)?;
    // HURT: expression 1 for 40 / 50 / 60 frames, then 0 or 2 (danger), with the hit shake of the
    // same strength baked in (`0x140268D60` starts both together; played from frame 1).
    for (si, n) in kh1::HURT_FRAMES.into_iter().enumerate() {
        for after in [0, 2] {
            let t = &tracks[si];
            let mut steps = motion::steps(t, &[face_piece(1)]);
            steps.pop(); // the hold: expression 1 until the timer ends, then `after`
            steps.push(Step { pieces: vec![face_piece(1)], frames: (t.len, n), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 });
            steps.push(Step { pieces: vec![face_piece(after)], frames: (n + 1, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 });
            groups.push(Group { steps, once: true, word: GROUP_WORD_FADE });
        }
    }
    // Fades (KH1: 12.8 a frame for 10 frames, `0x14027F760`; drawn 115.2 … 12.8).
    let last = mb::FADE_FRAMES - 2;
    let alpha = |a: u8| (a as u32) << 24 | 0x80_8080;
    let grey = |c: u8| 0x8000_0000 | (c as u32) * 0x01_0101;
    let fade = |p: Piece, from: u32, to: u32, then: u32, held: bool| Group {
        steps: vec![
            Step { pieces: vec![p], frames: (0, last), colour: (from, to), flags: FLAGS_COLOUR, tr: [0; 4], rot: 0.0 },
            Step { pieces: vec![p], frames: (last + 1, if held { LONG } else { mb::FADE_FRAMES }), colour: (then, then), flags: FLAGS_COLOUR, tr: [0; 4], rot: 0.0 },
        ],
        once: true,
        word: GROUP_WORD_FADE,
    };
    // MP gain: black over the cell, alpha 115 → 13 (KH1 draws the gained cell RGB × 12.8 … 115.2).
    for c in &cells {
        groups.push(fade(Piece { colour: 0x8000_0000, ..*c }, alpha(mb::FADE_FIRST), alpha(mb::FADE_LAST), 0x0080_8080, false));
    }
    // MP loss: the cell's RGB 115 → 13, then gone.
    for c in &cells {
        groups.push(fade(*c, grey(mb::FADE_FIRST), grey(mb::FADE_LAST), 0, false));
    }
    // Charge: [30 j, 30 j + t] fading in (held) or out.
    let mut charge_pieces = Vec::new();
    for j in 0..9 {
        for t in 1..=30 {
            charge_pieces.push(bk.piece_soft(&mut atlas, kh1::RING_CHARGE, p5, 30 * j, 30 * j + t, t < 30));
        }
    }
    for p in &charge_pieces {
        groups.push(fade(*p, alpha(mb::FADE_LAST), alpha(mb::FADE_FIRST), one, true));
    }
    for p in &charge_pieces {
        groups.push(fade(*p, alpha(mb::FADE_FIRST), alpha(mb::FADE_LAST), 0x0080_8080, false));
    }
    // Danger flash (`g_red`: colour (128, 0, 0, a), a 0 → 128 over 8 key units, back to 0 over 8;
    // players step 0.5 a frame and draw after the step: a = 8, 16 … 128, 120 … 0, looped).
    let rk = &g_red.keys_a;
    let (rr, rg, rb) = (rk[0].vals[0] as u32, rk[0].vals[1] as u32, rk[0].vals[2] as u32);
    let up = rk[0].dur * 2;
    let down = rk[1].dur * 2;
    let peak = rk[1].vals[3] as u32;
    let rc = |a: u32| a << 24 | rb << 16 | rg << 8 | rr;
    let step_a = peak / up as u32;
    let flash = |p: Piece| Group {
        steps: vec![
            Step { pieces: vec![p], frames: (0, up - 1), colour: (rc(step_a), rc(peak)), flags: FLAGS_COLOUR, tr: [0; 4], rot: 0.0 },
            Step { pieces: vec![p], frames: (up, up + down - 1), colour: (rc(peak - step_a), rc(0)), flags: FLAGS_COLOUR, tr: [0; 4], rot: 0.0 },
        ],
        once: false,
        word: GROUP_WORD_FADE,
    };
    for q in 0..3 {
        for k in 1..=18 {
            let p = bk.piece_soft(&mut atlas, kh1::RING_HP, p5, 90 * q, 90 * q + 5 * k, k < 18);
            groups.push(flash(p));
        }
    }
    groups.push(flash(obj_piece(&objs[obj::SEG_HP], it5, -1, 1, 0, None)));
    for rem in 1..kh1::HP_PER_SEG {
        groups.push(flash(obj_piece(&objs[obj::SEG_HP], it5, -1, 1, cut(rem, kh1::HP_PER_SEG, kh1::PITCH), None)));
    }
    if groups.len() != group::COUNT {
        return bad(format!("kh1hud: {} groups, expected {}", groups.len(), group::COUNT));
    }
    // H2b: every unit of the held cluster groups gets its motion programs (played groups).
    let motion = motion::units(&groups[..=group::FACE], &tracks, group::COUNT);
    for (u, pieces) in motion.unit_pieces.iter().enumerate() {
        for p in motion.programs_of(u) {
            groups.push(motion::program_group(&motion.programs[p], &tracks, pieces));
        }
    }
    // H4 / H5: pictures 1, 2, 3 (titles, icons, words; the root's panels; the submenus') and
    // picture 0's LOCK ON! plate corner (texels 96..128 square: the plate's art, at the PC's 4×).
    let mut mp = Vec::new();
    let mut pic2_pc = None;
    for (i, off, data) in MENU_PICTURES {
        let t = tims.get(i).ok_or_else(|| crate::Error::Format(format!("uitex.bin: no picture {i}")))?;
        if t.offset != off || t.width != 128 || t.height != 128 {
            return bad(format!("uitex.bin: picture {i} at {:#x} {}×{}, KH1 draws it from {off:#x} 128×128", t.offset, t.width, t.height));
        }
        let ps2 = Pic { w: 128, h: 128, px: t.pixels.clone() };
        let mut per = Vec::new();
        for l in langs {
            per.push(uitex_asset(src.uitex_hd, data, l)?.ok_or_else(|| crate::Error::Format(format!("uitex.bin: no PC texture for picture {i}")))?);
        }
        let per = fit_all(&ps2, per, HD_SCALE);
        let scale = hd_scale(&ps2, &per[0])?;
        // FM's context tab draws from picture 2's PC-only rows (`0x140278B40`): kept whole.
        if i == 2 {
            pic2_pc = Some((per[0].clone(), scale));
        }
        let per: Vec<Pic> = per.into_iter().map(|p| p.crop(0, 0, 128 * scale, 128 * scale)).collect();
        // One shared PC picture when every language has the same one (pictures 2, 3).
        let per = if per.iter().all(|p| *p == per[0]) { vec![per[0].clone()] } else { per };
        mp.push(atlas.add(ps2, per, scale));
    }
    // Picture 0: its corner (the LOCK ON! plate, H5) and, for the shortcut panel's frame (stage
    // 7.12), the picture whole (taken as `raw`: split to the rects drawn).
    let (p0, p0_whole) = {
        let (i, off, data) = PLATE_PICTURE;
        let t = tims.get(i).ok_or_else(|| crate::Error::Format(format!("uitex.bin: no picture {i}")))?;
        if t.offset != off || t.width != 128 || t.height != 128 {
            return bad(format!("uitex.bin: picture {i} at {:#x} {}×{}", t.offset, t.width, t.height));
        }
        let ps2 = Pic { w: 128, h: 128, px: t.pixels.clone() };
        let mut per = Vec::new();
        for l in langs {
            per.push(uitex_asset(src.uitex_hd, data, l)?.ok_or_else(|| crate::Error::Format(format!("uitex.bin: no PC texture for picture {i}")))?);
        }
        let per = fit_all(&ps2, per, HD_SCALE_PLATE);
        let scale = hd_scale(&ps2, &per[0])?;
        let whole: Vec<Pic> = per.iter().map(|p| p.crop(0, 0, 128 * scale, 128 * scale)).collect();
        let whole = if whole.iter().all(|p| *p == whole[0]) { vec![whole[0].clone()] } else { whole };
        let c = PLATE_CORNER;
        let per: Vec<Pic> = per.into_iter().map(|p| p.crop(c * scale, c * scale, (128 - c) * scale, (128 - c) * scale)).collect();
        let per = if per.iter().all(|p| *p == per[0]) { vec![per[0].clone()] } else { per };
        let corner = atlas.add(ps2.crop(c, c, 128 - c, 128 - c), per, scale);
        (corner, atlas.add(ps2, whole, scale))
    };
    // H4: KH1's command menu pieces and words.
    let menu_built = match &src.menu {
        Some(ms) => {
            let (p2, s) = pic2_pc.as_ref().expect("picture 2 read");
            Some(menu::build(ms, &mut atlas, [mp[0], mp[1], mp[2]], (p2, *s), &mut groups)?)
        }
        None => None,
    };
    // Stage 7.12: KH1's shortcut panel (with the menu: its words are the menu's).
    let shortcut_icon = src.menu.as_ref().and_then(|ms| ms.icon);
    let mut shortcut_built = match src.menu.as_ref().and_then(|ms| ms.cut.map(|c| (c, ms.icon))) {
        Some((cut, icon)) => Some(shortcut::build(cut, &mut atlas, p0_whole, mp[0], icon, &mut groups)?),
        None => None,
    };
    // H5: the LOCK ON! plate and the target's HP gauge.
    let target_built = match (src.lock, src.g_enm) {
        (Some(l), Some(e)) => Some(target::build(l, e, [p0, mp[0], it6], &mut groups)?),
        _ => None,
    };
    // H7: KH1's lock-on cursor (`lock` object 2, `lockon_c`, picture 10).
    let mut cursor_item = None;
    let cursor_built = match src.lock {
        Some(l) => {
            let (i, off, data) = CURSOR_PICTURE;
            let t = tims.get(i).ok_or_else(|| crate::Error::Format(format!("uitex.bin: no picture {i}")))?;
            if t.offset != off || t.width != 128 || t.height != 128 {
                return bad(format!("uitex.bin: picture {i} at {:#x} {}×{}", t.offset, t.width, t.height));
            }
            let ps2 = Pic { w: 128, h: 128, px: t.pixels.clone() };
            let hd = uitex_asset(src.uitex_hd, data, langs[0])?.ok_or_else(|| crate::Error::Format(format!("uitex.bin: no PC texture for picture {i}")))?;
            let hd = fit_hd(&ps2, hd, HD_SCALE);
            let scale = hd_scale(&ps2, &hd)?;
            let item = atlas.add(ps2, vec![hd.crop(0, 0, 128 * scale, 128 * scale)], scale);
            cursor_item = Some(item);
            Some((cursor::build(l, item, &mut groups)?, cursor::build_ring(l, item, &mut groups)?))
        }
        None => None,
    };
    let (cursor_built, auto_ring_built) = (cursor_built.map(|c| c.0), cursor_built.map(|c| c.1));
    let mut party5 = None;
    // H6: Donald's and Goofy's gauges, full (Re:CoM's friends have no HP or MP).
    let party_built = match src.g_c {
        Some(gc) => Some(party::build(gc, [it6, {
            let i = bk.src5_item(&mut atlas, &pics[p5], &pic_langs[p5]);
            party5 = Some(i);
            i
        }], [faces[1], faces[2]], [src.faces[1].ps2.h as i32, src.faces[2].ps2.h as i32], &mut groups)?),
        None => None,
    };
    // H8: KH1's LEVEL UP!! box (`lev_s`, pictures 3 and 1).
    let level_built = match src.lev_s {
        Some(l) => Some(popup::level_up(l, [mp[2], mp[0]], &mut groups)?),
        None => None,
    };
    // KH1's MP ring built as Re:CoM's HP ring (`ring`): KH1's quarter pictures whole, cut by
    // scissor parts and turned, for the MP ring around Re:CoM's HP gauge.
    let ring_first = groups.len();
    let mut ring_stats = CutStats::default();
    {
        let s7 = pics[p7].scale;
        let back = ring::Quarter::new(&mut atlas, kh1::RING_MP_BACK, &pics[p7].ps2, &pic_langs[p7], s7)?;
        // Picture 5 has one PC texture for every language (`-a2c0.dds`).
        let mp_q = ring::Quarter::new(&mut atlas, kh1::RING_MP, &pics[p5].ps2, &pic_langs[p5][..1], pics[p5].scale)?;
        let charge_q = ring::Quarter::new(&mut atlas, kh1::RING_CHARGE, &pics[p5].ps2, &pic_langs[p5][..1], pics[p5].scale)?;
        let (bx, by) = ring::black_texel(&pics[p7].ps2, &pic_langs[p7], kh1::RING_MP_BACK, s7).ok_or_else(|| crate::Error::Format("uitex.bin: no black outline texel at picture 7's MP back end".into()))?;
        let black = atlas.add(pics[p7].ps2.crop(bx, by, 1, 1), pic_langs[p7].iter().map(|p| p.crop(bx * s7, by * s7, s7, s7)).collect(), s7);
        let div = ring::dividers(&mut atlas, &back, &mp_q)?;
        let inputs = ring::Inputs { objs, it5, it7, back, mp: mp_q, charge: charge_q, black, div };
        groups.extend(ring::groups(&inputs, &mut ring_stats)?);
    }
    let ring_end = groups.len();
    // The other pad types' button glyphs (the shortcut panel's and the context tab's), after
    // every other group so no group before them moves.
    if let Some(sc) = shortcut_built.as_mut() {
        if sc.icons {
            sc.pads = shortcut::pad_glyphs(&mut atlas, shortcut_icon, &mut groups)?;
        }
    }
    let groups_total = groups.len();
    // The ring's straight pieces, before they get boxes of their own (`ring::lift_straights`).
    let straights = ring::straight_marks(&groups[ring_first..ring_end], &[it5, it7]);
    let mut raw = vec![it5, it6, it7, p0, p0_whole];
    raw.extend(&faces);
    raw.extend(&mp);
    raw.extend(cursor_item);
    raw.extend(party5);
    let refit = fit_layouts(&mut atlas, &groups, &raw);
    split_raw(&mut atlas, &mut groups, &raw);
    ring::lift_straights(&mut atlas, &mut groups[ring_first..ring_end], &straights)?;
    let packed = build_atlas(&atlas, langs.len())?;
    let base = seqd(&groups, &packed, false)?;
    let hd = seqd(&groups, &packed, true)?;
    let sprites = Seqd::parse(&base)?.sprites.len();
    Ok(Built {
        lad: layd(&base),
        lad_hd: layd(&hd),
        imz: imgz(&packed.ps2),
        imz_hd: packed.hd.iter().map(imgz).collect(),
        atlas: (packed.ps2.w, packed.ps2.h, packed.hd[0].w, packed.hd[0].h),
        pieces: atlas.items.len(),
        sprites,
        motion,
        groups: groups_total,
        menu: menu_built,
        shortcut: shortcut_built,
        target: target_built,
        party: party_built,
        cursor: cursor_built,
        auto_ring: auto_ring_built,
        level_up: level_built,
        refit,
        ring: (ring_first, ring_stats),
    })
}

/// Animation flags of a moving piece: Re:CoM's static flags (`FLAGS_STATIC`) with the translation
/// interpolated (bits 0x10000 and 0x1000 clear: `0x14024B4E0` folds 0x1000 into 0x10000, and
/// `0x14024A9xx` interpolates `+0xB0` from `+0xD0` to `+0xD8` only when that bit is clear) and an
/// exclusive end (0x8000: no extra frame after `f1`, so consecutive segments never draw together).
/// Checked (decomp); Re:CoM's own `Cockpit:00` uses 0x8000 (e.g. 0x4FB71) and moving pieces with
/// both translation bits clear (e.g. 0x6B73).
pub(crate) const FLAGS_MOVE: u32 = 0x6EFF1;

/// H2b: KH1's gauge motions baked into played groups (decision 39: "hit shake / size-class slide
/// baked into every piece's animation data"; Re:CoM plays them, nothing of ours per frame).
///
/// KH1 moves the whole cluster: `0x14026D990` adds the running motion's keyed offset (slot `+0x110`:
/// a hit's shake `0x140268D60`, or a size-class change's slide `0x14026D570`) to the centre every
/// piece is drawn from. Re:CoM animates per layer, and a scrubbed (held) layer never advances in time
/// (`0x140248BB0`: a set frame `+0x38` ≥ 0 freezes it). So each distinct piece set of the held
/// groups (a *unit*) gets one played group per motion program (the motion, then its last offset
/// held to frame 10,000,000: the group's longest animation, which Re:CoM keeps drawn after the end,
/// `0x140248F40`'s bit 4), and the DLL swaps the held layers for the units of what they show while
/// a motion runs.
pub mod motion {
    use super::*;
    use crate::uibin::Key;

    /// `g_act` objects the programs come from (Sora's slot; the party's are `vibe_c…`).
    pub const OBJECTS: [&str; 5] = ["vibe_s", "vibe_ss", "vibe_sss", "slide_s_h", "slide_s_v"];

    /// A KH1 position track at Re:CoM's frames: segments (frames [f0, f1), offset from → to), its
    /// length and its last offset. KH1's players step 0.5 key units a 60 Hz frame (`0x14233FBE0`),
    /// so a key unit is 2 frames; frame f of the group is key time f / 2.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Track {
        /// Segments: (f0, f1), (x0, y0), (x1, y1).
        pub segs: Vec<((i32, i32), (i32, i32), (i32, i32))>,
        /// Frames.
        pub len: i32,
        /// The last key's offset.
        pub end: (i32, i32),
    }

    /// A track from KH1's keys (`0x14027E430`: linear between keys; a key with a duration ≤ 0 ends
    /// the track with its value).
    pub fn track(keys: &[Key]) -> Result<Track> {
        let v = |k: &Key| (k.vals[0].round() as i32, k.vals[1].round() as i32);
        let mut segs = Vec::new();
        let mut t = 0;
        for w in keys.windows(2) {
            if w[0].dur <= 0 {
                break;
            }
            let f1 = t + 2 * w[0].dur;
            segs.push(((t, f1), v(&w[0]), v(&w[1])));
            t = f1;
        }
        let last = keys.get(segs.len()).ok_or_else(|| crate::Error::Format("g_act: a track without keys".into()))?;
        if segs.is_empty() || last.dur > 0 {
            return bad("g_act: a track without an end key");
        }
        Ok(Track { segs, len: t, end: v(last) })
    }

    /// The five tracks of [`OBJECTS`].
    pub fn tracks(u: &Uibin) -> Result<Vec<Track>> {
        OBJECTS
            .iter()
            .map(|n| {
                let o = u.find(n).ok_or_else(|| crate::Error::Format(format!("{G_ACT}: no {n}")))?;
                let k = u.c_keys(o).ok_or_else(|| crate::Error::Format(format!("{G_ACT}: {n} has no position keys")))?;
                track(k)
            })
            .collect()
    }

    /// What a program does.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Kind {
        /// A track of [`OBJECTS`].
        Move(usize),
        /// KH1's gauge fade (`0x140268E30`: alpha ±0.1 a frame, drawn after the step): out or in.
        Fade(bool),
    }

    /// A motion program: its kind, the frame the DLL plays it from (`0x140248550`) and its length.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Program {
        /// Name for the manifest.
        pub name: &'static str,
        /// Kind.
        pub kind: Kind,
        /// Frame to play from: 1 for the tracks (KH1 draws key time 0.5 first), 0 for the fades
        /// (KH1 draws 0.9 / 0.1 first, the group's frame 0).
        pub start: i32,
        /// Frames until the motion is over.
        pub len: i32,
    }

    /// The units: distinct piece sets of the held groups, their runs and programs.
    #[derive(Clone, Debug, Default)]
    pub struct Motion {
        /// Programs, in the order of a unit's groups (the slides last, only for slide units).
        pub programs: Vec<Program>,
        /// Each unit's pieces.
        pub(crate) unit_pieces: Vec<Vec<Piece>>,
        /// Whether a unit has the slide programs.
        pub unit_slides: Vec<bool>,
        /// Each unit's first group.
        pub first: Vec<usize>,
        /// (held group, f0, f1, unit): the unit is shown while the held group's frame is in [f0, f1].
        pub runs: Vec<(usize, i32, i32, usize)>,
    }

    /// Programs every unit has.
    pub const BASE_PROGRAMS: usize = 5;

    impl Motion {
        /// The programs unit `u` has (indices into [`Motion::programs`]).
        pub fn programs_of(&self, u: usize) -> std::ops::Range<usize> {
            0..if self.unit_slides[u] { self.programs.len() } else { BASE_PROGRAMS }
        }
    }

    /// The programs: the three hit shakes, the fades out / in, the two slides.
    pub fn programs(tracks: &[Track]) -> Vec<Program> {
        let mv = |name, t: usize| Program { name, kind: Kind::Move(t), start: 1, len: tracks[t].len };
        vec![
            mv("vibe_s", 0),
            mv("vibe_ss", 1),
            mv("vibe_sss", 2),
            Program { name: "fade_out", kind: Kind::Fade(true), start: 0, len: mb::FADE_FRAMES },
            Program { name: "fade_in", kind: Kind::Fade(false), start: 0, len: mb::FADE_FRAMES },
            mv("slide_s_h", 3),
            mv("slide_s_v", 4),
        ]
    }

    /// The highest frame of a held group a slide can show: a slide plays only while the size class
    /// changes, i.e. while max HP < 37 (class 0 / 1, `0x1402690D0`) on one side of the change, so HP
    /// stays near the ring (≤ 72: one Re:CoM level-up of +15 past 36 and a margin) and the charge
    /// near 6 MP (≤ 210); other groups are small. Assumption (a bound, logged by the DLL when a
    /// slide finds a unit without it: it then moves at once).
    fn slide_limit(held: usize) -> i32 {
        match held {
            group::HP_BACK | group::HP => 72,
            group::CHARGE => 210,
            _ => i32::MAX,
        }
    }

    /// The units of the held groups `held` (indices = group numbers), numbered from group `base`.
    pub(crate) fn units(held: &[Group], tracks: &[Track], base: usize) -> Motion {
        let mut m = Motion { programs: programs(tracks), ..Default::default() };
        let mut of: std::collections::BTreeMap<Vec<Piece>, usize> = Default::default();
        for (gi, g) in held.iter().enumerate() {
            for st in &g.steps {
                let u = *of.entry(st.pieces.clone()).or_insert_with(|| {
                    m.unit_pieces.push(st.pieces.clone());
                    m.unit_slides.push(false);
                    m.unit_pieces.len() - 1
                });
                if st.frames.0 <= slide_limit(gi) {
                    m.unit_slides[u] = true;
                }
                m.runs.push((gi, st.frames.0, st.frames.1, u));
            }
        }
        let mut next = base;
        for u in 0..m.unit_pieces.len() {
            m.first.push(next);
            next += m.programs_of(u).len();
        }
        m
    }

    /// A track's steps on `pieces`: the segments (moving, exclusive ends), then the last offset held.
    pub(crate) fn steps(t: &Track, pieces: &[Piece]) -> Vec<Step> {
        let one = 0x8080_8080;
        let mut v: Vec<Step> = t
            .segs
            .iter()
            .map(|&(fr, a, b)| Step { pieces: pieces.to_vec(), frames: fr, colour: (one, one), flags: FLAGS_MOVE, tr: [a.0, a.1, b.0, b.1], rot: 0.0 })
            .collect();
        v.push(Step { pieces: pieces.to_vec(), frames: (t.len, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [t.end.0, t.end.1, t.end.0, t.end.1], rot: 0.0 });
        v
    }

    /// The played group of program `p` on a unit's pieces.
    pub(crate) fn program_group(p: &Program, tracks: &[Track], pieces: &[Piece]) -> Group {
        let a = |x: u32| x << 24 | 0x80_8080;
        let steps = match p.kind {
            Kind::Move(t) => steps(&tracks[t], pieces),
            Kind::Fade(out) => {
                // 0.9 … 0.0 (out) or 0.1 … 1.0 (in) over frames 0..9, then held.
                let (from, to) = if out { (115, 0) } else { (13, 128) };
                vec![
                    Step { pieces: pieces.to_vec(), frames: (0, p.len - 1), colour: (a(from), a(to)), flags: FLAGS_COLOUR, tr: [0; 4], rot: 0.0 },
                    Step { pieces: pieces.to_vec(), frames: (p.len, LONG), colour: (a(to), a(to)), flags: FLAGS_COLOUR, tr: [0; 4], rot: 0.0 },
                ]
            }
        };
        Group { steps, once: true, word: GROUP_WORD_FADE }
    }
}

/// H4: KH1's command menu look (decision 39: KH1's panels, cursor and COMMAND title, the field look
/// in the field and the battle look in battle rooms, the words in KH1's own menu font).
///
/// KH1 (Steam; Checked (decomp) unless tagged): `0x14027B870` draws the root list at (16, 260)
/// (+ the left margin), a submenu at its parent's x + 11; `0x140278B40` draws a list: its header
/// (`*_fra_top` + the title) at the origin, rows at y + 20 + 22 i (the frame `*_fra_c`, the last
/// row `*_fra_d` + the connector `*_fra_cd`), the cursor row not as a frame but as the selection
/// (`*_sel_sha / act / gra / bla` + the command's icon) at (x + 11, y + 18 + 22 c); words at the
/// row's (x + 16, y + 22 + 22 i), the cursor row's at (x + 29, y + 25 + 22 c) (`0x14027A920` adds
/// (16, 2)). The root's pieces are 24 units wider (its right vertices and their texels, the
/// `param_10` of `0x14027A600` / `0x14027A9F0`: the panels' art in picture 2 runs to texel 125,
/// Checked (files)). In battle (`0x142867364 & 1`, the root record's `+0x1F8` = 0) the root's
/// panels are drawn with RGB (109, 30, 32) and the selection's plate (216, 109, 100)
/// (`0x14027A600`; the colours are `0x140278B40`'s locals), and the title is `com_battle`'s
/// `com_command_start` (blue → white → red over 20 frames, restarted when the mode changes),
/// `com_command_end` in the field.
pub mod menu {
    use super::*;

    /// Groups from the menu's first group: the root's field look, its battle look (each [`LOOK`]
    /// groups), the battle title's animation, the Magic list, then the words.
    pub mod g {
        /// Header frame.
        pub const HEADER: usize = 0;
        /// Title ("COMMAND"), static.
        pub const TITLE: usize = 1;
        /// A row's frame.
        pub const ROW: usize = 2;
        /// The last row's frame and connector.
        pub const ROW_LAST: usize = 3;
        /// The selection (cursor row).
        pub const SEL: usize = 4;
        /// The icons a / b / c (Attack, Magic, Items).
        pub const ICON: usize = 5;
        /// Groups a look.
        pub const LOOK: usize = 8;
        /// The battle look's first group.
        pub const BATTLE: usize = LOOK;
        /// The title turning red (played from frame 1, 20 frames, then held).
        pub const TITLE_TO_BATTLE: usize = 2 * LOOK;
        /// The Magic list: header, title, row, last row, selection, icon.
        pub const MAGIC: usize = 2 * LOOK + 1;
        /// FM's context tab: its four pieces (shadow, green plate, band, stripe), at the tab's
        /// origin ([`super::tab`]).
        pub const TAB: usize = MAGIC + 6;
        /// The Triangle (the PC's PlayStation glyph; empty without `Icon.tm2`).
        pub const TRIANGLE: usize = TAB + 1;
        /// The words.
        pub const WORDS: usize = TAB + 2;
    }

    /// FM's context tab (`0x140278B40`, the block after the root's rows; Checked (decomp) unless
    /// tagged). Four pieces drawn from the root's picture (2) at the tab's origin (the root's
    /// origin + (9, −10) + the slide), their rects `com`'s selection objects 4..7 (`sel_sha_d`,
    /// `sel_act_d`, `sel_gra_d`, `sel_bla_d`) moved by (−17, +2) and widened, their UVs set in the
    /// code (inclusive PS2 texels of picture 2; those rows exist only in the PC texture
    /// `-4140.dds`, 256 × 384: Checked (files)), their colours the code's (vertex order TL, BL,
    /// TR, BR, `0x14027EA50`: top and bottom pairs, a vertical gradient), alphas the objects':
    /// - 0: `sel_sha_d` (its own colour (0, 0, 0, 64): the shadow), w + 47, UV (2, 150)–(126, 174);
    /// - 1: `sel_act_d`, w + 47, the same UV, RGB (110, 210, 100): the green plate;
    /// - 2: `sel_gra_d`, w + 44, UV (2, 135)–(117, 148), top (3, 11, 3), bottom (30, 110, 32);
    /// - 3: `sel_bla_d`, w + 42, h + 2, UV (2, 126)–(114, 133), top (0, 0, 0), bottom (3, 11, 3).
    ///
    /// KH1 multiplies the plate's bright RGB (pieces 1, 2's bottom) by 0.5 while a submenu is on
    /// top; here the whole group's RGB is (the DLL's control colour): pieces 2, 3's dark (3, 11, 3)
    /// become (1, 5, 1) then instead of staying. The tab's opacity multiplies every alpha.
    pub mod tab {
        /// (object, width added, height added, UV (inclusive PS2 texels of picture 2), top RGB,
        /// bottom RGB; None: the object's own colour).
        pub const PIECES: [(usize, i32, i32, [i32; 4], Option<([i32; 3], [i32; 3])>); 4] = [
            (4, 47, 0, [2, 150, 126, 174], None),
            (5, 47, 0, [2, 150, 126, 174], Some(([110, 210, 100], [110, 210, 100]))),
            (6, 44, 0, [2, 135, 117, 148], Some(([3, 11, 3], [30, 110, 32]))),
            (7, 42, 2, [2, 126, 114, 133], Some(([0, 0, 0], [3, 11, 3]))),
        ];
        /// Every piece's move from its object's place.
        pub const MOVE: (i32, i32) = (-17, 2);
        /// The Triangle's quad (units; KH1 15.5 × 18.625, `0x1402D2050`) and the PC texels taken
        /// around the glyph (a PS2 icon cell is 20 × 24 texels: Assumption that the PC's is twice
        /// that, centred on the glyph).
        pub const TRIANGLE_RECT: [i32; 4] = [0, 0, 16, 19];
        /// The PC cell around the glyph.
        pub const TRIANGLE_CELL: (usize, usize) = (40, 48);
    }

    /// The root's extra width (`param_10` = 24 for list type 0, `0x140278B40`).
    pub const ROOT_EXT: i32 = 24;
    /// Battle colours (`0x140278B40`: `local_168`, `local_118`).
    pub const BATTLE_PANEL: [i32; 3] = [109, 30, 32];
    /// The battle selection plate's colour.
    pub const BATTLE_SEL: [i32; 3] = [216, 109, 100];
    /// Units a PC font texel: size 14 over the 18-texel base, PC texels 2 a PS2 texel.
    pub const FONT_UNITS: f32 = 14.0 / 36.0;
    /// A space's advance (PC texels): KH1's space is 5 base texels (`0x142E954F0` = 5,
    /// `0x1402E0000` area; drawn as 5 × size / base, `0x1402D2050` case 1).
    pub const SPACE: i32 = 10;
    /// A glyph cell (PC texels): 18 × 24 PS2 texels.
    pub const CELL: (usize, usize) = (36, 48);
    /// The cell's top in the word (units): KH1 draws from the cell's top; the 48-texel cell is
    /// drawn 19 units tall (18.7). The caps' middle at 8 units, where the H4 words had it
    /// (Assumption: placed by eye against the owner's screenshot).
    pub const CELL_DY: i32 = -2;

    /// The built menu: its first group and the words' groups.
    #[derive(Clone, Debug)]
    pub struct Built {
        /// First group.
        pub first: usize,
        /// (language, [(message id, group)]).
        pub words: Vec<(String, Vec<(u16, usize)>)>,
        /// Each word's width (tenths of a unit: the pen's place after its last glyph, as the word's
        /// pieces are laid out from x 0), by language as [`Built::words`]: the DLL squeezes a word
        /// wider than its box (owner, 2026-11-04: "Squeeze to fit").
        pub widths: Vec<(String, Vec<(u16, u32)>)>,
        /// Glyphs found / characters missing from the font.
        pub glyphs: usize,
        /// Characters without a glyph.
        pub missing: Vec<char>,
        /// The Triangle's group has the PC's PlayStation glyph.
        pub tab_icon: bool,
        /// M8 stage 8a.1: the Items list's first group (header, title, row, last row, selection,
        /// icon: the Magic list's six, [`g::MAGIC`]'s order), after the words.
        pub items_list: Option<usize>,
        /// M8b: the Summon list's first group (the same six), after the Items list's; None without
        /// `uibin_sum.bin` or when it is not laid out as the Magic list ([`SUM`]).
        pub summon_list: Option<usize>,
        /// M8b: why `uibin_sum.bin` was not used (for the step's note).
        pub summon_note: Option<String>,
        /// 0.179.1: the DP sleights' mark (Re:CoM's Heartless emblem, [`dp_emblem`]), one group after
        /// the Summon list's; None without [`MenuSources::dp_emblem`] or when it can't be read.
        pub dp_icon: Option<usize>,
        /// 0.179.1: why there is no mark (for the step's note).
        pub dp_note: Option<String>,
    }

    /// 0.179.1 (fork owner, 2026-11-23, verbatim: "preciso que no shorcut e no menu de commands tenha
    /// algo identificando que consomem DP, pode ser o ícone de um heartless"; "a direita fica
    /// perfeito"): Re:CoM's own Heartless emblem, the DP icon of Riku's level-up screen, drawn right
    /// of a DP sleight's name. Checked (files: `knowledge/ReCoM/recom-ui-textures.md` in the KH
    /// modding workspace): black with a red outline and a purple glow, between the Soul Eater and a
    /// fist in `FO0104.RTM`'s PC texture.
    pub const DP_EMBLEM_FILE: &str = "FORM/0104/FO0104.RTM";
    /// Its PC asset (any language's holds the same art; UK is read).
    pub const DP_EMBLEM_ASSET: &str = "UK_levelup.imz";
    /// The part of the 1024 × 1024 texture searched (x, y, w, h): right of the Soul Eater (it ends at
    /// x 120), left of the fist (from x 239).
    pub const DP_EMBLEM_AREA: [usize; 4] = [128, 40, 104, 140];

    /// The emblem at the menu font's PC height ([`CELL`]`.1`, the width kept in proportion, even),
    /// from `FO0104.RTM`'s `UK_levelup.imz` (an `IMGZ` of one 32-bit `IMGD`, PS2 alpha, as the atlas
    /// keeps it): the covered texels of [`DP_EMBLEM_AREA`], area-averaged.
    pub fn dp_emblem(imz: &[u8]) -> Result<Pic> {
        if imz.len() < 0x60 || &imz[0..4] != b"IMGZ" || &imz[0x20..0x24] != b"IMGD" {
            return bad(format!("{DP_EMBLEM_ASSET}: not an IMGZ of one IMGD"));
        }
        let rd32 = |o: usize| u32::from_le_bytes([imz[o], imz[o + 1], imz[o + 2], imz[o + 3]]) as usize;
        let rd16 = |o: usize| u16::from_le_bytes([imz[o], imz[o + 1]]) as usize;
        let (off, size, w, h) = (0x20 + rd32(0x28), rd32(0x2C), rd16(0x3C), rd16(0x3E));
        let [ax, ay, aw, ah] = DP_EMBLEM_AREA;
        if size != w * h * 4 || imz.len() < off + size || w < ax + aw || h < ay + ah {
            return bad(format!("{DP_EMBLEM_ASSET}: {w}×{h}, {size} bytes (a 1024 × 1024 32-bit picture expected)"));
        }
        let pic = Pic { w, h, px: imz[off..off + size].chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect() };
        let area = pic.crop(ax, ay, aw, ah);
        let (mut x0, mut y0, mut x1, mut y1) = (aw, ah, 0, 0);
        for j in 0..ah {
            for i in 0..aw {
                if area.px[j * aw + i][3] > 2 {
                    (x0, y0, x1, y1) = (x0.min(i), y0.min(j), x1.max(i + 1), y1.max(j + 1));
                }
            }
        }
        if x1 <= x0 + 8 || y1 <= y0 + 8 {
            return bad(format!("{DP_EMBLEM_ASSET}: no emblem in {DP_EMBLEM_AREA:?}"));
        }
        let (bw, bh) = (x1 - x0, y1 - y0);
        let th = CELL.1;
        let tw = ((th * bw + bh / 2) / bh + 1) & !1;
        Ok(area.crop(x0, y0, bw, bh).resampled(tw, th))
    }

    /// The menu font: its PC picture and glyphs (character → x, y, advance in PC texels).
    pub struct Font {
        pic: Pic,
        glyphs: std::collections::BTreeMap<u32, (usize, usize, usize)>,
    }

    /// Reads the system font's PC texture (DDS) and `US_font_data_tbl.bin`.
    pub fn font(dds: &[u8], tbl: &[u8]) -> Result<Font> {
        let img = decode_dds(dds)?;
        let pic = Pic { w: img.w, h: img.h, px: img.px.iter().map(|p| [p[0], p[1], p[2], p[3]]).collect() };
        if pic.w != 1024 || pic.h != 1024 || tbl.len() % 16 != 0 || tbl.is_empty() {
            return bad(format!("sysfont: PC texture {}×{} (1024 × 1024 expected), table {} bytes", pic.w, pic.h, tbl.len()));
        }
        let (cw, ch) = CELL;
        let mut glyphs = std::collections::BTreeMap::new();
        for e in tbl.chunks_exact(16) {
            let u = u16::from_le_bytes([e[2], e[3]]) as usize;
            let v = u16::from_le_bytes([e[4], e[5]]) as usize;
            let (adv, page) = (e[6] as usize, e[7] as usize);
            let mut c = u16::from_le_bytes([e[8], e[9]]) as u32;
            if (0xFF01..=0xFF5E).contains(&c) {
                c -= 0xFEE0;
            }
            if u < 8 || v < 8 || c == 0 {
                continue;
            }
            let x = (u - 8) / 16 * 2 + (page & 15) * 512;
            let y = (v - 8) / 16 * 2 + (page >> 4) * 512;
            if x + cw <= pic.w && y + ch <= pic.h {
                glyphs.entry(c).or_insert((x, y, adv * 2));
            }
        }
        if !glyphs.contains_key(&('A' as u32)) {
            return bad("sysfont: no glyph for A in the table");
        }
        Ok(Font { pic, glyphs })
    }

    impl Font {
        /// A glyph's cell: the PS2-size picture (half) and the PC one.
        fn cell_pics(&self, x: usize, y: usize) -> (Pic, Pic) {
            let (cw, ch) = CELL;
            let mut hd = self.pic.crop(x, y, cw, ch);
            let (hw, hh) = (cw / 2, ch / 2);
            let mut ps2 = Pic::blank(hw, hh);
            for j in 0..hh {
                for i in 0..hw {
                    let mut a = [0u32; 4];
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let p = hd.px[(2 * j + dy) * cw + 2 * i + dx];
                        for k in 0..4 {
                            a[k] += p[k] as u32;
                        }
                    }
                    ps2.px[j * hw + i] = a.map(|v| ((v + 2) / 4) as u8);
                }
            }
            // PS2 alpha (0x80 = opaque) in both; the PC picture's grey glyphs (0x80) are drawn
            // white by KH1's ×2 vertex colour: doubled for the PC copy (as KH1's PC pictures).
            for p in ps2.px.iter_mut() {
                p[3] = ((p[3] as u32 * 128 + 127) / 255) as u8;
            }
            for p in hd.px.iter_mut() {
                for k in 0..3 {
                    p[k] = (p[k] as u32 * 2).min(255) as u8;
                }
                p[3] = ((p[3] as u32 * 128 + 127) / 255) as u8;
            }
            (ps2, hd)
        }
    }

    /// An object's piece: `ext` units (and texels) wider, its RGB replaced by `over`.
    pub(super) fn mpiece(o: &Obj, item: usize, dx: i32, dy: i32, ext: i32, over: Option<[i32; 3]>) -> Piece {
        let (mut uv, ..) = o.texels();
        uv[2] += ext;
        let [x, y, w, h] = o.rect;
        let col = |c: [i32; 4]| {
            let c = match over {
                Some(o) => [o[0], o[1], o[2], c[3]],
                None => c,
            };
            let b = |v: i32| v.clamp(0, 255) as u32;
            b(c[0]) | b(c[1]) << 8 | b(c[2]) << 16 | b(c[3]) << 24
        };
        let grad = if o.flags & 0x20 != 0 { col(o.colours[1]) } else { 0 };
        Piece { item, uv, rect: [x + dx, y + dy, x + dx + w + ext, y + dy + h], colour: col(o.colours[0]), grad }
    }

    pub(super) fn held1(pieces: Vec<Piece>) -> Group {
        let one = 0x8080_8080;
        Group { steps: vec![Step { pieces, frames: (0, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 }], once: true, word: GROUP_WORD_SCRUB }
    }

    /// Builds the menu groups (appended to `groups`) and the glyphs (into `atlas`).
    /// A picture at half size (the PS2 copy of art that exists only on PC; alpha kept PS2).
    pub(super) fn half(hd: &Pic) -> Pic {
        let (hw, hh) = (hd.w / 2, hd.h / 2);
        let mut p = Pic::blank(hw, hh);
        for j in 0..hh {
            for i in 0..hw {
                let mut a = [0u32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let t = hd.px[(2 * j + dy) * hd.w + 2 * i + dx];
                    for k in 0..4 {
                        a[k] += t[k] as u32;
                    }
                }
                p.px[j * hw + i] = a.map(|v| ((v + 2) / 4) as u8);
            }
        }
        p
    }

    /// The single picture of a TIM2 whose image is 32-bit (`Icon.tm2`).
    pub fn tim2_rgba(d: &[u8]) -> Result<Pic> {
        if d.len() < 0x40 || &d[0..4] != b"TIM2" {
            return bad("Icon.tm2: not a TIM2");
        }
        let rd32 = |o: usize| u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]) as usize;
        let rd16 = |o: usize| u16::from_le_bytes([d[o], d[o + 1]]) as usize;
        let (img, hs, kind) = (rd32(0x18), rd16(0x1C), d[0x23]);
        let (w, h) = (rd16(0x24), rd16(0x26));
        if kind != 3 || img != w * h * 4 || d.len() < 0x10 + hs + img {
            return bad(format!("Icon.tm2: image type {kind}, {w}×{h}, {img} bytes (32-bit expected)"));
        }
        let px = d[0x10 + hs..0x10 + hs + img].chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect();
        Ok(Pic { w, h, px })
    }

    /// `FontIcon.inf`'s box for `code` (x0, y0, x1, y1).
    pub fn icon_box(inf: &[u8], code: u16) -> Option<[usize; 4]> {
        let n = u16::from_le_bytes([*inf.get(4)?, *inf.get(5)?]) as usize;
        (0..n).map(|i| 16 + 16 * i).filter(|&o| o + 10 <= inf.len()).map(|o| (0..5).map(|k| u16::from_le_bytes([inf[o + 2 * k], inf[o + 2 * k + 1]]) as usize).collect::<Vec<_>>()).find(|r| r[0] == code as usize).map(|r| [r[1], r[2], r[3], r[4]])
    }

    /// The Triangle's PC cell: [`tab::TRIANGLE_CELL`] centred on the glyph's covered texels in
    /// its `FontIcon.inf` box.
    pub fn triangle_cell(icon: &Pic, inf: &[u8]) -> Result<Pic> {
        icon_cell(icon, inf, ICON_TRIANGLE)
    }

    /// A button glyph's PC cell (`code`'s `FontIcon.inf` record), as [`triangle_cell`].
    pub fn icon_cell(icon: &Pic, inf: &[u8], code: u16) -> Result<Pic> {
        let Some([x0, y0, x1, y1]) = icon_box(inf, code) else { return bad(format!("FontIcon.inf: no record {code:#x}")) };
        if x1 > icon.w || y1 > icon.h || x0 >= x1 || y0 >= y1 {
            return bad(format!("FontIcon.inf: glyph {code:#x}'s box is outside Icon.tm2"));
        }
        let (mut a, mut b) = ((usize::MAX, usize::MAX), (0usize, 0usize));
        for y in y0..y1 {
            for x in x0..x1 {
                if icon.px[y * icon.w + x][3] > 8 {
                    a = (a.0.min(x), a.1.min(y));
                    b = (b.0.max(x + 1), b.1.max(y + 1));
                }
            }
        }
        if a.0 == usize::MAX {
            return bad(format!("Icon.tm2: glyph {code:#x}'s box is empty"));
        }
        let (cw, ch) = tab::TRIANGLE_CELL;
        let cx = ((a.0 + b.0) / 2).saturating_sub(cw / 2).min(icon.w - cw);
        let cy = ((a.1 + b.1) / 2).saturating_sub(ch / 2).min(icon.h - ch);
        Ok(icon.crop(cx, cy, cw, ch))
    }

    pub(crate) fn build(ms: &MenuSources, atlas: &mut Atlas, [p1, p2, p3]: [usize; 3], (pic2, s2): (&Pic, usize), groups: &mut Vec<Group>) -> Result<Built> {
        let com = Uibin::parse(ms.com)?;
        let cb = Uibin::parse(ms.com_battle)?;
        let mag = Uibin::parse(ms.mag)?;
        let need = |u: &Uibin, n: &str, names: &[&str]| -> Result<()> {
            for (i, want) in names.iter().enumerate() {
                if u.objs.get(i).map(|o| o.name.as_str()) != Some(want) {
                    return bad(format!("{n}: object {i} is not {want} (not KH1 FM Steam's layout)"));
                }
            }
            Ok(())
        };
        need(&com, COM, &["com_fra_d", "com_fra_cd", "com_fra_c", "com_fra_top", "com_sel_sha_d", "com_sel_act_d", "com_sel_gra_d", "com_sel_bla_d", "com_command", "com_sel_ico_a", "com_sel_ico_b", "com_sel_ico_c"])?;
        need(&cb, COM_BATTLE, &["com_command_start", "com_command_end"])?;
        need(&mag, MAG, &["mag_fra_d", "mag_fra_2_d", "mag_fra_cd", "mag_fra_c", "mag_fra_2_c", "mag_fra_top", "mag_fra_2_top", "mag_sel_sha_d", "mag_sel_sha_2_d", "mag_sel_act_d", "mag_sel_act_2_d", "mag_sel_gra_d", "mag_sel_gra_2_d", "mag_sel_bla_d", "mag_sel_bla_2_d", "mag_magic", "mag_sel_ico_d"])?;
        let item = match ms.item {
            Some(d) => {
                let u = Uibin::parse(d)?;
                need(&u, ITEM, &["ite_fra_d", "ite_fra_2_d", "ite_fra_cd", "ite_fra_c", "ite_fra_2_c", "ite_fra_top", "ite_fra_2_top", "ite_sel_sha_d", "ite_sel_sha_2_d", "ite_sel_act_d", "ite_sel_act_2_d", "ite_sel_gra_d", "ite_sel_gra_2_d", "ite_sel_bla_d", "ite_sel_bla_2_d", "ite_item", "ite_sel_ico_d"])?;
                Some(u)
            }
            None => None,
        };
        // M8b: the Summon list, when its objects are the Magic list's (`sum_` for `mag_`; the
        // title any name). Not an error otherwise: the DLL uses the Magic list's panels.
        let (sum, summon_note) = match ms.sum {
            Some(d) => match Uibin::parse(d) {
                Ok(u) => {
                    let bad_at = (0..17).find(|&i| {
                        let want = mag.objs[i].name.replacen("mag_", "sum_", 1);
                        i != 15 && u.objs.get(i).map(|o| o.name.as_str()) != Some(want.as_str())
                    });
                    match bad_at {
                        None => (Some(u), None),
                        Some(i) => {
                            let names: Vec<&str> = u.objs.iter().map(|o| o.name.as_str()).collect();
                            (None, Some(format!("{SUM}: object {i} is not the Magic list's (objects {names:?})")))
                        }
                    }
                }
                Err(e) => (None, Some(format!("{SUM}: {e}"))),
            },
            None => (None, None),
        };
        let first = groups.len();
        let o = &com.objs;
        let ext = ROOT_EXT;
        // The title's colours: com_command_end's last key (field), com_command_start's (battle).
        let keys_of = |obj: &Obj| -> Vec<crate::uibin::Key> {
            let (f, n) = obj.keys_a;
            if f < 0 || n <= 0 {
                return Vec::new();
            }
            cb.keys_a.get(f as usize..(f + n) as usize).map(|k| k.to_vec()).unwrap_or_default()
        };
        let start = keys_of(&cb.objs[0]);
        let end = keys_of(&cb.objs[1]);
        if start.len() < 2 || end.len() < 2 {
            return bad(format!("{COM_BATTLE}: the title keys are not KH1's"));
        }
        let rgba = |k: &crate::uibin::Key| -> [i32; 4] { [k.vals[0] as i32, k.vals[1] as i32, k.vals[2] as i32, k.vals[3] as i32] };
        let title_col = [rgba(end.last().unwrap()), rgba(start.last().unwrap())];
        for (look, over, sel_over) in [(0, None, None), (1, Some(BATTLE_PANEL), Some(BATTLE_SEL))] {
            groups.push(held1(vec![mpiece(&o[3], p2, 0, 0, ext, over)]));
            let mut t = mpiece(&o[8], p1, 0, 0, 0, None);
            let c = title_col[look];
            t.colour = (c[0].clamp(0, 255) | c[1].clamp(0, 255) << 8 | c[2].clamp(0, 255) << 16 | c[3].clamp(0, 255) << 24) as u32;
            groups.push(held1(vec![t]));
            groups.push(held1(vec![mpiece(&o[2], p2, 0, 0, ext, over)]));
            groups.push(held1(vec![mpiece(&o[0], p2, 0, 0, ext, over), mpiece(&o[1], p2, 0, 0, 0, over)]));
            // The selection's gradient: black on top, the field's blue (the object's own) or, in
            // battle, the panels' red at the bottom (`0x14027A2B0` with `0x140278B40`'s
            // `local_108` block: vertices (0, 0, 0), (109, 30, 32), (0, 0, 0), (109, 30, 32);
            // Checked (decomp)). Owner, 2026-11-01: "in battle, the command menu still has a blue
            // tint".
            let mut gra = mpiece(&o[6], p2, 0, 0, ext, None);
            // 1 unit higher, under the stripe (as the context tab's band).
            gra.rect[1] -= 1;
            if let Some([r, g, b]) = over {
                let a = (o[6].colours[1][3].clamp(0, 255)) as u32;
                gra.grad = r.clamp(0, 255) as u32 | (g.clamp(0, 255) as u32) << 8 | (b.clamp(0, 255) as u32) << 16 | a << 24;
            }
            groups.push(held1(vec![mpiece(&o[4], p2, 0, 0, ext, None), mpiece(&o[5], p2, 0, 0, ext, sel_over), gra, mpiece(&o[7], p2, 0, 0, ext, None)]));
            for k in 9..12 {
                groups.push(held1(vec![mpiece(&o[k], p1, ext, 0, 0, None)]));
            }
        }
        // The title turning red: com_command_start's colour keys (2 frames a key unit), from frame 1.
        {
            let mut t = mpiece(&o[8], p1, 0, 0, 0, None);
            t.colour = 0x8080_8080;
            let col = |c: [i32; 4]| (c[0].clamp(0, 255) | c[1].clamp(0, 255) << 8 | c[2].clamp(0, 255) << 16 | c[3].clamp(0, 255) << 24) as u32;
            let mut steps = Vec::new();
            let mut f = 0;
            for w in start.windows(2) {
                if w[0].dur <= 0 {
                    break;
                }
                let f1 = f + 2 * w[0].dur;
                steps.push(Step { pieces: vec![t], frames: (f, f1), colour: (col(rgba(&w[0])), col(rgba(&w[1]))), flags: FLAGS_COLOUR | 0x8000, tr: [0; 4], rot: 0.0 });
                f = f1;
            }
            let last = col(rgba(start.last().unwrap()));
            steps.push(Step { pieces: vec![t], frames: (f, LONG), colour: (last, last), flags: FLAGS_COLOUR, tr: [0; 4], rot: 0.0 });
            groups.push(Group { steps, once: true, word: GROUP_WORD_FADE });
        }
        // The Magic list (`0x14027A1C0`: the objects' own colours, no extra width).
        // M8 stage 8a.1: the Items list's six groups are made the same way from `item`, after the
        // words (so every group the DLL already knows keeps its number).
        let sub_list = |m: &[Obj], groups: &mut Vec<Group>| {
            groups.push(held1(vec![mpiece(&m[5], p3, 0, 0, 0, None), mpiece(&m[6], p3, 0, 0, 0, None)]));
            groups.push(held1(vec![mpiece(&m[15], p1, 0, 0, 0, None)]));
            groups.push(held1(vec![mpiece(&m[3], p3, 0, 0, 0, None), mpiece(&m[4], p3, 0, 0, 0, None)]));
            groups.push(held1(vec![mpiece(&m[0], p3, 0, 0, 0, None), mpiece(&m[1], p3, 0, 0, 0, None), mpiece(&m[2], p3, 0, 0, 0, None)]));
            groups.push(held1((7..15).map(|k| mpiece(&m[k], p3, 0, 0, 0, None)).collect()));
            groups.push(held1(vec![mpiece(&m[16], p1, 0, 0, 0, None)]));
        };
        sub_list(&mag.objs, groups);
        // FM's context tab: picture 2's PC-only rows, one atlas item a UV rect.
        debug_assert_eq!(groups.len() - first, g::TAB);
        // The tab's pieces are made at KH1 FM PC's 2×: a texture mod's picture 2 at another whole
        // scale (e.g. 4×, 512 × 768) is resampled to 2× first.
        let pic2_2x;
        let (pic2, s2) = if s2 > 2 {
            pic2_2x = pic2.resampled(pic2.w * 2 / s2, pic2.h * 2 / s2);
            (&pic2_2x, 2)
        } else {
            (pic2, s2)
        };
        if s2 != 2 || pic2.h < 175 * s2 || pic2.w < 128 * s2 {
            return bad(format!("uitex.bin picture 2's PC texture is {}×{}: not KH1 FM PC's 256 × 384 with the context tab's rows", pic2.w, pic2.h));
        }
        let mut tab_items: std::collections::BTreeMap<[i32; 4], usize> = Default::default();
        let mut pieces = Vec::new();
        for (k, dw, dh, uv, cols) in tab::PIECES {
            let [u0, v0, u1, v1] = uv;
            let item = *tab_items.entry(uv).or_insert_with(|| {
                let hd = pic2.crop(u0 as usize * s2, v0 as usize * s2, (u1 - u0 + 1) as usize * s2, (v1 - v0 + 1) as usize * s2);
                atlas.add(half(&hd), vec![hd], s2)
            });
            let ob = &o[k];
            let [x, y, w, h] = ob.rect;
            let (x, y) = (x + tab::MOVE.0, y + tab::MOVE.1);
            let rgba = |c: [i32; 3], a: i32| (c[0].clamp(0, 255) | c[1].clamp(0, 255) << 8 | c[2].clamp(0, 255) << 16 | a.clamp(0, 255) << 24) as u32;
            // Alphas: the object's (top: its first colour; bottom: its second with flag 0x20).
            let (at, ab) = (ob.colours[0][3], if ob.flags & 0x20 != 0 { ob.colours[1][3] } else { ob.colours[0][3] });
            let (colour, grad) = match cols {
                None => (rgba([ob.colours[0][0], ob.colours[0][1], ob.colours[0][2]], at), 0),
                Some((top, bottom)) => (rgba(top, at), rgba(bottom, ab)),
            };
            let grad = if grad == colour { 0 } else { grad };
            // The band (object 6) starts 1 unit higher, under the stripe (7, drawn after it): the
            // PC atlas is filtered and the two meeting edge to edge left a light line between
            // them (owner's screenshot, 2026-11-01: "a weird like line through the middle").
            let up = if k == 6 { 1 } else { 0 };
            pieces.push(Piece { item, uv: [0, 0, u1 - u0 + 1, v1 - v0 + 1], rect: [x, y - up, x + w + dw, y + h + dh], colour, grad });
        }
        groups.push(held1(pieces));
        // The Triangle: the PC's PlayStation glyph.
        let tri = match ms.icon {
            Some([tm2, inf]) => Some(triangle_cell(&tim2_rgba(tm2)?, inf)?),
            None => None,
        };
        let tab_icon = tri.is_some();
        match tri {
            Some(hd) => {
                let (w, h) = (hd.w / 2, hd.h / 2);
                let item = atlas.add(half(&hd), vec![hd], 2);
                groups.push(held1(vec![Piece { item, uv: [0, 0, w as i32, h as i32], rect: tab::TRIANGLE_RECT, colour: 0x8080_8080, grad: 0 }]));
            }
            None => groups.push(held(&[vec![]])),
        }
        debug_assert_eq!(groups.len() - first, g::WORDS);
        // Words: KH1's menu font, a glyph a part, at the font's size.
        let f = font(ms.font[0], ms.font[1])?;
        let mut glyph_item: std::collections::BTreeMap<u32, usize> = Default::default();
        let mut missing = Vec::new();
        let mut words = Vec::new();
        let mut widths = Vec::new();
        for (lang, ws) in &ms.words {
            let mut list = Vec::new();
            let mut wlist = Vec::new();
            for (id, text) in ws {
                let mut pieces = Vec::new();
                let mut x = 0.0f32;
                for ch in text.chars() {
                    let Some(&(gx, gy, w)) = f.glyphs.get(&(ch as u32)) else {
                        if ch != ' ' && !missing.contains(&ch) {
                            missing.push(ch);
                        }
                        x += SPACE as f32 * FONT_UNITS;
                        continue;
                    };
                    let item = *glyph_item.entry(ch as u32).or_insert_with(|| {
                        let (ps2, hd) = f.cell_pics(gx, gy);
                        atlas.add(ps2, vec![hd], 2)
                    });
                    let x0 = x.round() as i32;
                    let (cw, ch) = CELL;
                    let (sw, sh) = ((cw as f32 * FONT_UNITS).round() as i32, (ch as f32 * FONT_UNITS).round() as i32);
                    pieces.push(Piece { item, uv: [0, 0, (cw / 2) as i32, (ch / 2) as i32], rect: [x0, CELL_DY, x0 + sw, CELL_DY + sh], colour: 0x8080_8080, grad: 0 });
                    x += w as f32 * FONT_UNITS;
                }
                list.push((*id, groups.len()));
                wlist.push((*id, (x * 10.0).round() as u32));
                groups.push(held1(pieces));
            }
            words.push((lang.clone(), list));
            widths.push((lang.clone(), wlist));
        }
        let items_list = item.map(|u| {
            let at = groups.len();
            sub_list(&u.objs, groups);
            at
        });
        // M8b: the Summon list's six after the Items list's (every group the DLL knows keeps its
        // number).
        let summon_list = sum.map(|u| {
            let at = groups.len();
            sub_list(&u.objs, groups);
            at
        });
        // 0.179.1: the DP sleights' mark, after the Summon list's (every group the DLL knows keeps
        // its number): one piece at the font's height, from the pen's place (x 0, as a word).
        let (dp_icon, dp_note) = match ms.dp_emblem.map(dp_emblem) {
            Some(Ok(hd)) => {
                let ps2 = half(&hd);
                let item = atlas.add(ps2.clone(), vec![hd.clone()], 2);
                let (w, h) = ((hd.w as f32 * FONT_UNITS).round() as i32, (hd.h as f32 * FONT_UNITS).round() as i32);
                let at = groups.len();
                groups.push(held1(vec![Piece { item, uv: [0, 0, ps2.w as i32, ps2.h as i32], rect: [0, CELL_DY, w, CELL_DY + h], colour: 0x8080_8080, grad: 0 }]));
                (Some(at), None)
            }
            Some(Err(e)) => (None, Some(e.to_string())),
            None => (None, None),
        };
        Ok(Built { first, words, widths, glyphs: glyph_item.len(), missing, tab_icon, items_list, summon_list, summon_note, dp_icon, dp_note })
    }
}

/// M7 stage 7.12: KH1's shortcut panel (`0x14027B870` case 5; Checked (decomp + files) unless
/// tagged). KH1 draws, at (left margin + 16, 0x11A):
/// - `uibin_cut` object 0 `cut_fra_a` (110 × 86, UV (2, 2)–(111, 87), four vertex colours: brown
///   (123, 69, 31) on top, purple (65, 55, 118) below) from `uitex.bin` **picture 0** (sprite def
///   `0x1427F1460`, built by `0x140266580` at offset 0) and object 1 `cut_shortcut` ("SHORT CUT",
///   83 × 8 at (7, 6), (130, 90, 50)) from **picture 1** (def `0x1427F1430`, offset 0x2080), both
///   with `0x14027A9F0`: the objects as the file has them (their colour keys, a 2-frame alpha-in,
///   are never started: nothing calls `0x14027E710` on `0x142853960`), alphas × the menu's fade;
/// - the button glyphs 0x46, 0x45, 0x41 through the font (`0x140279FC0`, size 14, RGB × 1, alpha the
///   fade) at objects 2..4's places (91, 23 / 45 / 67): `cut_ico_a..c`'s own pictures are not
///   drawn;
/// - the three slots' words (the DLL: the menu's word groups).
///
/// Groups from [`Built::first`]: [`g::PANEL`] (frame, then title), then the PC's PlayStation glyphs
/// Triangle, Square, Cross, Circle, each a 16 × 19 quad at its own origin as the context tab's
/// Triangle (`menu::tab::TRIANGLE_RECT`; empty groups without `Icon.tm2`).
pub mod shortcut {
    use super::*;

    /// Groups from the first.
    pub mod g {
        /// The frame and the title.
        pub const PANEL: usize = 0;
        /// Triangle, Square, Cross, Circle.
        pub const GLYPHS: usize = 1;
        /// Groups in all.
        pub const COUNT: usize = 5;
    }

    /// The glyphs' `FontIcon.inf` records, in the groups' order.
    pub const GLYPH_CODES: [u16; 4] = [ICON_TRIANGLE, ICON_SQUARE, ICON_CROSS, ICON_CIRCLE];
    /// The other pad types' glyphs, in [`GLYPH_CODES`]' order (Triangle's place, Square's, Cross's,
    /// Circle's). Re:CoM's controller config (`0x14034E930` → `0x14083F574`) `+0x1C` is the pad
    /// type; `0x140349120` fills its button glyphs from `0xE3F0` (type 1), `0xE410` (type 2, the
    /// PlayStation set above) or `0xE430` (type 3), each set Square's place, Triangle's, Cross's,
    /// Circle's (Checked (exe)). `Icon.tm2`: `0xE3F0..` the Xbox X / Y / A / B, `0xE430..` a
    /// generic pad's left / top / bottom / right (Checked (files)).
    pub const PAD_CODES: [[u16; 4]; 2] = [[0xE3F1, 0xE3F0, 0xE3F2, 0xE3F3], [0xE431, 0xE430, 0xE432, 0xE433]];
    /// The pad glyph groups ([`pad_glyphs`]): pad type 1's four, then type 3's.
    pub const PAD_COUNT: usize = 8;
    /// `cut_ico_a / b / c`'s places (the DLL's `crate::command_menu::shortcut::GLYPH_AT`).
    pub const GLYPH_AT: [(i32, i32); 3] = [(91, 23), (91, 45), (91, 67)];

    /// The built panel.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Built {
        /// First group.
        pub first: usize,
        /// The glyph groups hold the PC's button art.
        pub icons: bool,
        /// The other pad types' glyph groups' first ([`pad_glyphs`]), when built.
        pub pads: Option<usize>,
    }

    /// One glyph's group: the cell drawn 16 × 19 at its own origin (as the context tab's Triangle).
    fn glyph_group(atlas: &mut Atlas, hd: Pic) -> Group {
        let (w, h) = (hd.w / 2, hd.h / 2);
        let item = atlas.add(menu::half(&hd), vec![hd], 2);
        menu::held1(vec![Piece { item, uv: [0, 0, w as i32, h as i32], rect: menu::tab::TRIANGLE_RECT, colour: 0x8080_8080, grad: 0 }])
    }

    /// The other pad types' glyphs ([`PAD_CODES`]), [`PAD_COUNT`] groups appended to `groups`;
    /// their first, or None without `Icon.tm2`.
    pub(crate) fn pad_glyphs(atlas: &mut Atlas, icon: Option<[&[u8]; 2]>, groups: &mut Vec<Group>) -> Result<Option<usize>> {
        let Some([tm2, inf]) = icon else { return Ok(None) };
        let pic = menu::tim2_rgba(tm2)?;
        let cells = PAD_CODES.iter().flatten().map(|&c| menu::icon_cell(&pic, inf, c)).collect::<Result<Vec<Pic>>>()?;
        let first = groups.len();
        for hd in cells {
            groups.push(glyph_group(atlas, hd));
        }
        debug_assert_eq!(groups.len() - first, PAD_COUNT);
        Ok(Some(first))
    }

    /// Builds the panel's groups (appended to `groups`): `p0` an atlas item of `uitex.bin`'s
    /// picture 0 whole (PS2 texels), `p1` picture 1's (the menu's titles); `icon` the PC's
    /// `Icon.tm2` and `FontIcon.inf`.
    pub(crate) fn build(cut: &[u8], atlas: &mut Atlas, p0: usize, p1: usize, icon: Option<[&[u8]; 2]>, groups: &mut Vec<Group>) -> Result<Built> {
        let u = Uibin::parse(cut)?;
        for (i, want) in ["cut_fra_a", "cut_shortcut", "cut_ico_a", "cut_ico_b", "cut_ico_c"].iter().enumerate() {
            if u.objs.get(i).map(|o| o.name.as_str()) != Some(want) {
                return bad(format!("{CUT}: object {i} is not {want} (not KH1 FM Steam's layout)"));
            }
        }
        for (k, &(x, y)) in GLYPH_AT.iter().enumerate() {
            let r = u.objs[2 + k].rect;
            if (r[0], r[1]) != (x, y) {
                return bad(format!("{CUT}: {} at ({}, {}), KH1 FM Steam's is at ({x}, {y})", u.objs[2 + k].name, r[0], r[1]));
            }
        }
        let first = groups.len();
        groups.push(menu::held1(vec![menu::mpiece(&u.objs[0], p0, 0, 0, 0, None), menu::mpiece(&u.objs[1], p1, 0, 0, 0, None)]));
        let pics = match icon {
            Some([tm2, inf]) => {
                let pic = menu::tim2_rgba(tm2)?;
                Some(GLYPH_CODES.iter().map(|&c| menu::icon_cell(&pic, inf, c)).collect::<Result<Vec<Pic>>>()?)
            }
            None => None,
        };
        let icons = pics.is_some();
        for k in 0..GLYPH_CODES.len() {
            match &pics {
                Some(p) => groups.push(glyph_group(atlas, p[k].clone())),
                None => groups.push(held(&[vec![]])),
            }
        }
        debug_assert_eq!(groups.len() - first, g::COUNT);
        Ok(Built { first, icons, pads: None })
    }
}

/// H5: KH1's LOCK ON! plate (`0x1402747F0`) and the target's HP gauge (`0x1402758B0` →
/// `0x140275000`). KH1 (Checked (decomp) unless tagged):
/// - the plate: `lock` objects 5..7 (picture 0) and 8..10 (picture 1, "LOCK ON!") at (left margin
///   + 16, 24); on a lock their colour keys restart (`0x14027E710`) and play (alpha 0 → peak over
///   2 frames, → rest over 10), then hold; at the lock's end alpha − 1/6 a frame (6 frames);
/// - the gauge at (left margin + 16, 24 + 28 while the plate shows, `0x1402743E0`): `g_enm` 0
///   (picture 6) and 1..3 (picture 1, the green "HP") at the origin; 4 (picture 6) and 5 (1)
///   `L` = min(max HP / 2, 150) wide from x + 28; the cap 12 (6) and 13 (1) at x + 28 + L; five bars
///   (one per 300 HP, each min(HP − 300 k + 1, 300) / 2 wide, the texture stretched); with more
///   than one bar a marker (11) at the top bar's end; alpha 0 → 1 by 0.2 a frame and back.
/// The widths are Re:CoM X scales over frames 0..150 (as the MP bar's partial segment), the
/// moving pieces controls of their own, placed on events.
pub mod target {
    use super::*;

    /// Groups from the first: the plate (played), the gauge's fixed part, its length, its cap,
    /// the five bars, the marker.
    pub mod g {
        /// The plate (played from frame 1: its colour keys, then held).
        pub const PLATE: usize = 0;
        /// The gauge's head ("HP").
        pub const HEAD: usize = 1;
        /// Its body, X-scaled: frame = L (0..150).
        pub const BODY: usize = 2;
        /// Its cap.
        pub const CAP: usize = 3;
        /// Bar k (0..4), X-scaled: frame = its length.
        pub const BAR: usize = 4;
        /// The top bar's end marker.
        pub const MARKER: usize = 9;
        /// Groups.
        pub const COUNT: usize = 10;
    }

    /// The bars' colours by stack index: green first, then yellow, orange, magenta, violet
    /// (`g_enm` objects 10, 9, 8, 7, 6). Assumption (KH1's slot order not read).
    pub const BAR_OBJECTS: [usize; 5] = [10, 9, 8, 7, 6];

    fn grow(pieces: Vec<Piece>) -> Group {
        let one = 0x8080_8080;
        Group {
            steps: vec![
                Step { pieces: pieces.clone(), frames: (0, 150), colour: (one, one), flags: crate::mp_bar::FLAGS_GROW_PUB, tr: [0; 4], rot: 0.0 },
                Step { pieces, frames: (151, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 },
            ],
            once: true,
            word: GROUP_WORD_SCRUB,
        }
    }

    /// A piece at `dx` from its object's place (`x0`: replace the object's x).
    fn tp(o: &Obj, item: usize, x0: Option<i32>, dx: i32, uv_off: i32) -> Piece {
        let (mut uv, mu, mv) = o.texels();
        for k in 0..4 {
            uv[k] -= uv_off;
        }
        let [ox, oy, w, h] = o.rect;
        let x = x0.unwrap_or(ox) + dx;
        let rect = [if mu { x + w } else { x }, if mv { oy + h } else { oy }, if mu { x } else { x + w }, if mv { oy } else { oy + h }];
        let c = |c: [i32; 4]| c[0].clamp(0, 255) as u32 | (c[1].clamp(0, 255) as u32) << 8 | (c[2].clamp(0, 255) as u32) << 16 | (c[3].clamp(0, 255) as u32) << 24;
        let grad = if o.flags & 0x20 != 0 { c(o.colours[1]) } else { 0 };
        Piece { item, uv, rect, colour: c(o.colours[0]), grad }
    }

    /// Builds the groups (appended to `groups`); returns the first.
    pub(crate) fn build(lock: &[u8], g_enm: &[u8], [p0, p1, p6]: [usize; 3], groups: &mut Vec<Group>) -> Result<usize> {
        let l = Uibin::parse(lock)?;
        let e = Uibin::parse(g_enm)?;
        let ln = ["lockon_plate_a", "lockon_plate_b", "lockon_plate_c", "lockon_text_a", "lockon_text_b", "lockon_text_c"];
        for (k, n) in ln.iter().enumerate() {
            if l.objs.get(5 + k).map(|o| o.name.as_str()) != Some(n) {
                return bad(format!("{LOCK}: object {} is not {n}", 5 + k));
            }
        }
        let en = ["g_enm_out_a", "g_enm_fra_a", "g_enm_fra_hp", "g_enm_fra_aa", "g_enm_out_b", "g_enm_fra_b", "g_enm_bar_v", "g_enm_bar_m", "g_enm_bar_o", "g_enm_bar_y", "g_enm_bar_g", "g_enm_fra_p", "g_enm_out_c", "g_enm_fra_c"];
        for (k, n) in en.iter().enumerate() {
            if e.objs.get(k).map(|o| o.name.as_str()) != Some(n) {
                return bad(format!("{G_ENM}: object {k} is not {n}"));
            }
        }
        let first = groups.len();
        // The plate: each piece's colour keys (2 frames a key unit), played from frame 1, held.
        let mut steps = Vec::new();
        for (k, o) in l.objs[5..11].iter().enumerate() {
            let piece = if k < 3 { tp(o, p0, None, 0, PLATE_CORNER as i32) } else { tp(o, p1, None, 0, 0) };
            let (f0, n) = o.keys_a;
            let keys = l.keys_a.get(f0.max(0) as usize..(f0 + n).max(0) as usize).unwrap_or(&[]);
            let col = |k: &crate::uibin::Key| k.vals[0].clamp(0.0, 255.0) as u32 | (k.vals[1].clamp(0.0, 255.0) as u32) << 8 | (k.vals[2].clamp(0.0, 255.0) as u32) << 16 | (k.vals[3].clamp(0.0, 255.0) as u32) << 24;
            let mut f = 0;
            for w in keys.windows(2) {
                if w[0].dur <= 0 {
                    break;
                }
                let f1 = f + 2 * w[0].dur;
                // The piece's own colour is the multiplier's base: draw with colour 0x80 and the keys.
                steps.push(Step { pieces: vec![Piece { colour: 0x8080_8080, ..piece }], frames: (f, f1), colour: (col(&w[0]), col(&w[1])), flags: FLAGS_COLOUR | 0x8000, tr: [0; 4], rot: 0.0 });
                f = f1;
            }
            let last = keys.last().map(col).unwrap_or(piece.colour);
            steps.push(Step { pieces: vec![Piece { colour: 0x8080_8080, ..piece }], frames: (f, LONG), colour: (last, last), flags: FLAGS_COLOUR, tr: [0; 4], rot: 0.0 });
        }
        groups.push(Group { steps, once: true, word: GROUP_WORD_FADE });
        let one = 0x8080_8080;
        let held1 = |pieces: Vec<Piece>| Group { steps: vec![Step { pieces, frames: (0, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 }], once: true, word: GROUP_WORD_SCRUB };
        let o = &e.objs;
        groups.push(held1(vec![tp(&o[0], p6, None, 0, 0), tp(&o[1], p1, None, 0, 0), tp(&o[2], p1, None, 0, 0), tp(&o[3], p1, None, 0, 0)]));
        // Body and bars from x = 0 (their control at the origin + 28): X scale = width / 150.
        groups.push(grow(vec![tp(&o[4], p6, Some(0), 0, 0), tp(&o[5], p1, Some(0), 0, 0)]));
        groups.push(held1(vec![tp(&o[12], p6, Some(0), 0, 0), tp(&o[13], p1, Some(0), 0, 0)]));
        for k in BAR_OBJECTS {
            groups.push(grow(vec![tp(&o[k], p1, Some(0), 0, 0)]));
        }
        groups.push(held1(vec![tp(&o[11], p1, Some(0), 0, 0)]));
        debug_assert_eq!(groups.len() - first, g::COUNT);
        Ok(first)
    }
}

/// H6: Donald's and Goofy's gauges (slots 1 and 2; KH1 Checked (decomp + exe + files) unless
/// tagged): `g_c` objects at their places, the slot's offset (Donald (0, 0), Goofy (68, 20):
/// `0x140527330`, `0x140527370`) + the right margin; the face (48 × 48, `xa_ex_0040` / `0050`)
/// centred on (399, 44) (`0x1405272C0`, `0x14027F470`). Drawn: the face frame (`g_c_fra_a_w`),
/// the disc (`g_c_fra_a_a`) and the face, in Sora's order (`0x14026CF90`). No HP / MP rings and
/// no ring backs (`g_c_bar_*`, `g_c_fra_b/c/d_*`): Re:CoM's friends have no HP or MP (fork owner,
/// 2026-11-23, answer 4b; until 0.178.1 the rings were drawn full). Assumption: the party's draw
/// order is Sora's.
pub mod party {
    use super::*;

    /// The face's centre (`0x1405272C0`).
    pub const CENTRE: (i32, i32) = (399, 44);

    /// Builds Donald's and Goofy's groups; returns them.
    pub(crate) fn build(g_c: &[u8], [p6, _p5]: [usize; 2], faces: [usize; 2], face_h: [i32; 2], groups: &mut Vec<Group>) -> Result<[usize; 2]> {
        let u = Uibin::parse(g_c)?;
        for (k, n) in ["g_c_fra_a_w", "g_c_fra_a_a"].iter().enumerate() {
            if u.objs.get(k).map(|o| o.name.as_str()) != Some(n) {
                return bad(format!("{G_C}: object {k} is not {n}"));
            }
        }
        let o = &u.objs;
        let one = 0x8080_8080;
        let mut out = [0usize; 2];
        for m in 0..2 {
            let mut v = vec![obj_piece(&o[0], p6, 0, 0, 0, None), obj_piece(&o[1], p6, 0, 0, 0, None)];
            let h = face_h[m];
            let (cx, cy) = CENTRE;
            v.push(Piece { item: faces[m], uv: [0, 0, h, h], rect: [cx - h / 2, cy - h / 2, cx + h / 2, cy + h / 2], colour: one, grad: 0 });
            out[m] = groups.len();
            groups.push(Group { steps: vec![Step { pieces: v, frames: (0, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 }], once: true, word: GROUP_WORD_SCRUB });
        }
        Ok(out)
    }
}

/// H7: KH1's lock-on cursor (`0x140274B20`; Checked (decomp + files)): a locked target (state 3,
/// sub-state 0) draws `lock` object 2 (`lockon_c`, picture 10, 38 × 38) centred on the target's
/// projected lock point: the quad at (−w / 2 + kx, −h / 2 + ky) of size (w, h) from its position
/// keys (−4, −4, 46, 46 → 2, 2, 34, 34 → 0, 0, 38, 38 over 4 + 4 frames) and its colour keys
/// (alpha 0 → 255 → 192), restarted when the cursor kind changes; drawn after the step (key time
/// 0.5 first). Baked per frame (frame k = key time (k + 1) / 2), then held.
pub mod cursor {
    use super::*;

    /// KH1's cursor of a lock (`DAT_14284F9E0` = object 2).
    pub const OBJECT: &str = "lockon_c";
    /// KH1's auto-target ring (`DAT_14284F9F8` = object 3; drawn for lock states 1, 2 and 4, so
    /// the soft target of Auto Lock On). The owner's `UK_uibin_lock.bin`: 30 × 30 at uv (33, 1) of
    /// the same picture's layout, colour (112, 80, 12), alpha keys 0 → 250 → 112, position keys
    /// (−4, −4, 38, 38) → (2, 2, 26, 26) → (0, 0, 30, 30), 2 + 2 key units. Checked (decomp
    /// `0x140274290` / `0x140274B20`, this machine's KH1 project; files).
    pub const RING: &str = "lockon_d";

    fn lerp(keys: &[crate::uibin::Key], t: f32) -> Vec<f32> {
        let mut at = 0.0;
        for w in keys.windows(2) {
            if w[0].dur <= 0 {
                break;
            }
            let d = w[0].dur as f32;
            if t <= at + d {
                let f = (t - at) / d;
                return w[0].vals.iter().zip(&w[1].vals).map(|(a, b)| a + (b - a) * f).collect();
            }
            at += d;
        }
        keys.last().map(|k| k.vals.clone()).unwrap_or_default()
    }

    /// Builds the cursor's group (appended); returns it.
    pub(crate) fn build(lock: &[u8], item: usize, groups: &mut Vec<Group>) -> Result<usize> {
        build_object(lock, 2, OBJECT, item, groups)
    }

    /// Builds the auto-target ring's group (appended), as the cursor's; returns it.
    pub(crate) fn build_ring(lock: &[u8], item: usize, groups: &mut Vec<Group>) -> Result<usize> {
        build_object(lock, 3, RING, item, groups)
    }

    /// Object `index` (`name`) of `lock` played from its keys' start, baked per frame, then held.
    fn build_object(lock: &[u8], index: usize, name: &str, item: usize, groups: &mut Vec<Group>) -> Result<usize> {
        let u = Uibin::parse(lock)?;
        let o = u.objs.get(index).filter(|o| o.name == name).ok_or_else(|| crate::Error::Format(format!("{LOCK}: object {index} is not {name}")))?;
        let ck = u.c_keys(o).ok_or_else(|| crate::Error::Format(format!("{LOCK}: {name} has no position keys")))?;
        let (f0, n) = o.keys_a;
        let ak = u.keys_a.get(f0.max(0) as usize..(f0 + n).max(0) as usize).unwrap_or(&[]);
        let len = |k: &[crate::uibin::Key]| k.iter().take_while(|k| k.dur > 0).map(|k| k.dur).sum::<i32>();
        let frames = 2 * len(ck).max(len(ak));
        let (uv, ..) = o.texels();
        let col = |v: &[f32]| v[0].clamp(0.0, 255.0) as u32 | (v[1].clamp(0.0, 255.0) as u32) << 8 | (v[2].clamp(0.0, 255.0) as u32) << 16 | (v[3].clamp(0.0, 255.0) as u32) << 24;
        let piece = |t: f32| -> Piece {
            let p = lerp(ck, t);
            let (kx, ky, w, h) = (p[0] as i32, p[1] as i32, p[2] as i32, p[3] as i32);
            let x0 = -w / 2 + kx;
            let y0 = -h / 2 + ky;
            Piece { item, uv, rect: [x0, y0, x0 + w, y0 + h], colour: if ak.is_empty() { colour_of(o) } else { col(&lerp(ak, t)) }, grad: 0 }
        };
        let one = 0x8080_8080;
        let mut steps = Vec::new();
        for k in 0..frames {
            steps.push(Step { pieces: vec![piece((k + 1) as f32 / 2.0)], frames: (k, k), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 });
        }
        steps.push(Step { pieces: vec![piece(frames as f32)], frames: (frames, LONG), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 });
        groups.push(Group { steps, once: true, word: GROUP_WORD_FADE });
        Ok(groups.len() - 1)
    }
}

/// H8: KH1's popups where Re:CoM has the event. Built: the LEVEL UP!! box (`0x140272F10`, Checked
/// (decomp + files)): Sora's box at (left margin + 16, 16); state 1 restarts its objects' keys,
/// 2 steps, 3 holds 91 frames (`> 0x5A`), 4, then 5 fades alpha − 0.125 a frame, 6 ends. Its header
/// is `lev_s` objects 6..11 (frame, picture 3) and 12 (`level_up_s`, "LEVEL UP!!", picture 1),
/// each with colour keys (alpha 0 → 128 → 96 over 10 + 10 frames) and position keys (the drop-in:
/// y 36 → −12 → 6 → −3 → 0 over 10 + 6 + 2 + 2 frames). Baked per frame (frame k = key time
/// (k + 1) / 2), held, faded and ended in the data: Re:CoM plays it once.
pub mod popup {
    use super::*;

    /// The header's objects.
    pub const HEADER: [&str; 7] = ["lev_s_fra_top", "lev_s_fra_2_top", "lev_s_fra_3_top", "lev_s_fra_4_top", "lev_s_fra_5_top", "lev_s_fra_6_top", "level_up_s"];
    /// Frames KH1 holds the box (state 3 counts to 0x5A, then one frame of state 4), after the
    /// first two states' frames.
    pub const HOLD: i32 = 2 + 0x5B + 1;
    /// The fade: − 0.125 a frame.
    pub const FADE: i32 = 8;

    fn lerp(keys: &[crate::uibin::Key], t: f32) -> Option<Vec<f32>> {
        let mut at = 0.0;
        for w in keys.windows(2) {
            if w[0].dur <= 0 {
                break;
            }
            let d = w[0].dur as f32;
            if t <= at + d {
                let f = (t - at) / d;
                return Some(w[0].vals.iter().zip(&w[1].vals).map(|(a, b)| a + (b - a) * f).collect());
            }
            at += d;
        }
        keys.last().map(|k| k.vals.clone())
    }

    /// The LEVEL UP!! box's group (appended); returns it.
    pub(crate) fn level_up(lev_s: &[u8], [p3, p1]: [usize; 2], groups: &mut Vec<Group>) -> Result<usize> {
        let u = Uibin::parse(lev_s)?;
        let mut objs = Vec::new();
        for (k, n) in HEADER.iter().enumerate() {
            let o = u.objs.get(6 + k).filter(|o| o.name == *n).ok_or_else(|| crate::Error::Format(format!("{LEV_S}: object {} is not {n}", 6 + k)))?;
            objs.push(o);
        }
        let keys_of = |o: &Obj, a: bool| -> Vec<crate::uibin::Key> {
            let (f, n) = if a { o.keys_a } else { o.keys_c };
            let src = if a { &u.keys_a } else { &u.keys_c };
            src.get(f.max(0) as usize..(f + n).max(0) as usize).map(|k| k.to_vec()).unwrap_or_default()
        };
        let len = |k: &[crate::uibin::Key]| k.iter().take_while(|k| k.dur > 0).map(|k| k.dur).sum::<i32>();
        let keyed = objs.iter().map(|o| 2 * len(&keys_of(o, true)).max(len(&keys_of(o, false)))).max().unwrap_or(0);
        let col = |v: &[f32], f: f32| v[0].clamp(0.0, 255.0) as u32 | (v[1].clamp(0.0, 255.0) as u32) << 8 | (v[2].clamp(0.0, 255.0) as u32) << 16 | ((v[3] * f).clamp(0.0, 255.0) as u32) << 24;
        let pieces = |t: f32, fade: f32| -> Vec<Piece> {
            objs.iter()
                .enumerate()
                .map(|(k, o)| {
                    let (uv, mu, mv) = o.texels();
                    let r = lerp(&keys_of(o, false), t).map(|v| [v[0] as i32, v[1] as i32, v[2] as i32, v[3] as i32]).unwrap_or(o.rect);
                    let [x, y, w, h] = r;
                    let rect = [if mu { x + w } else { x }, if mv { y + h } else { y }, if mu { x } else { x + w }, if mv { y } else { y + h }];
                    let c = lerp(&keys_of(o, true), t).unwrap_or_else(|| o.colours[0].iter().map(|&v| v as f32).collect());
                    Piece { item: if k == 6 { p1 } else { p3 }, uv, rect, colour: col(&c, fade), grad: 0 }
                })
                .collect()
        };
        let one = 0x8080_8080;
        let mut steps = Vec::new();
        let hold = HOLD.max(keyed);
        for k in 0..hold {
            steps.push(Step { pieces: pieces((k + 1) as f32 / 2.0, 1.0), frames: (k, k), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 });
        }
        for j in 0..FADE {
            let f = 1.0 - 0.125 * (j + 1) as f32;
            steps.push(Step { pieces: pieces(hold as f32, f), frames: (hold + j, hold + j), colour: (one, one), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 });
        }
        // Over: the group's longest animation (kept drawn after the end) shows nothing.
        steps.push(Step { pieces: pieces(hold as f32, 0.0), frames: (hold + FADE, LONG), colour: (0x0080_8080, 0x0080_8080), flags: FLAGS_STATIC, tr: [0; 4], rot: 0.0 });
        groups.push(Group { steps, once: true, word: GROUP_WORD_FADE });
        Ok(groups.len() - 1)
    }
}

// ---------------------------------------------------------------------------------------------
// Adding the set to Re:CoM's files.

/// The languages Re:CoM's `FO0002.SPR` has PC layouts for (`<LANG>_Cockpit.lad`).
pub fn languages(spr_assets: &[HdAsset]) -> Vec<String> {
    let mut v: Vec<String> = spr_assets.iter().filter_map(|a| a.name.strip_suffix("_Cockpit.lad").map(str::to_string)).filter(|l| l.len() == 2).collect();
    v.sort();
    v.dedup();
    v
}

fn entry(name: &str, kind: [u8; 4], data: Vec<u8>) -> Result<Entry> {
    if name.len() >= 0x14 {
        return bad(format!("{name}: name too long for the container"));
    }
    let mut raw = [0u8; 0x18];
    raw[..name.len()].copy_from_slice(name.as_bytes());
    raw[0x14..].copy_from_slice(&kind);
    Ok(Entry { name_raw: raw, flag: true, data })
}

/// Adds `kh1hud.lad` to an `FO0002.SPR` container and `<LANG>_kh1hud.lad` to its PC assets
/// (refusing a file that already has it).
pub fn add_to_spr(spr: &[u8], assets: &mut Vec<HdAsset>, b: &Built, langs: &[String]) -> Result<Vec<u8>> {
    let mut c = Container::parse(spr)?;
    if c.find(LAD).is_some() || assets.iter().any(|a| a.name.ends_with(LAD)) {
        return bad(format!("FO0002.SPR already has {LAD}"));
    }
    let kind: [u8; 4] = c.entries[0].name_raw[0x14..].try_into().unwrap();
    c.entries.push(entry(LAD, kind, b.lad.clone())?);
    for l in langs {
        assets.push(HdAsset { name: format!("{l}_{LAD}"), orig_offset: 0xFFFF, data: b.lad_hd.clone() });
    }
    Ok(c.write())
}

/// Adds `kh1hud.imz` to an `FO0002.RTM` container and `<LANG>_kh1hud.imz` to its PC assets.
pub fn add_to_rtm(rtm: &[u8], assets: &mut Vec<HdAsset>, b: &Built, langs: &[String]) -> Result<Vec<u8>> {
    let mut c = Container::parse(rtm)?;
    if c.find(IMZ).is_some() || assets.iter().any(|a| a.name.ends_with(IMZ)) {
        return bad(format!("FO0002.RTM already has {IMZ}"));
    }
    let atlas = c.find(mb::ATLAS).ok_or_else(|| crate::Error::Format(format!("FO0002.RTM: no {}", mb::ATLAS)))?;
    let kind: [u8; 4] = atlas.name_raw[0x14..].try_into().unwrap();
    c.entries.push(entry(IMZ, kind, b.imz.clone())?);
    for (l, d) in langs.iter().zip(&b.imz_hd) {
        assets.push(HdAsset { name: format!("{l}_{IMZ}"), orig_offset: 0xFFFF, data: d.clone() });
    }
    Ok(c.write())
}

/// `kh1-hud.json`: the set, the group count, the languages and the inputs' sha256; `recom_hp`, the
/// first of Re:CoM's HP ring groups turned 180° in `gauge:01` (`crate::recom_hp`), when added.
pub fn manifest_json(b: &Built, langs: &[String], sources: &[(&str, String)], recom_hp: Option<usize>) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"format\": \"{FORMAT}\",");
    let _ = writeln!(s, "  \"set\": \"{SET}\",");
    let _ = writeln!(s, "  \"groups\": {},", b.groups);
    let _ = writeln!(s, "  \"base_groups\": {},", group::COUNT);
    let m = &b.motion;
    let _ = writeln!(
        s,
        "  \"programs\": [{}],",
        m.programs.iter().map(|p| format!("{{\"name\": \"{}\", \"start\": {}, \"frames\": {}}}", p.name, p.start, p.len)).collect::<Vec<_>>().join(", ")
    );
    let _ = writeln!(s, "  \"units\": [{}],", (0..m.first.len()).map(|u| format!("[{}, {}]", m.first[u], m.programs_of(u).len())).collect::<Vec<_>>().join(", "));
    let _ = writeln!(s, "  \"runs\": [{}],", m.runs.iter().map(|r| format!("[{}, {}, {}, {}]", r.0, r.1, r.2, r.3)).collect::<Vec<_>>().join(", "));
    if let Some(f) = recom_hp {
        let of = crate::recom_hp::TURNED.map(|g| g.to_string()).join(", ");
        let _ = writeln!(s, "  \"recom_hp\": {{\"set\": \"{}\", \"first\": {f}, \"of\": [{of}]}},", crate::recom_hp::SET);
    }
    if let Some(l) = b.level_up {
        let _ = writeln!(s, "  \"level_up\": {l},");
    }
    let _ = writeln!(s, "  \"mp_ring\": {{\"first\": {}, \"count\": {}}},", b.ring.0, ring_group::COUNT);
    if let Some(c) = b.cursor {
        let _ = writeln!(s, "  \"cursor\": {c},");
    }
    if let Some(c) = b.auto_ring {
        let _ = writeln!(s, "  \"auto_ring\": {c},");
    }
    if let Some([d, g]) = b.party {
        let _ = writeln!(s, "  \"party\": [{d}, {g}],");
    }
    if let Some(t) = b.target {
        let _ = writeln!(s, "  \"target\": {{\"first\": {t}}},");
    }
    if let Some(sc) = &b.shortcut {
        match sc.pads {
            Some(p) => {
                let _ = writeln!(s, "  \"shortcut\": {{\"first\": {}, \"icons\": {}, \"pads\": {p}}},", sc.first, sc.icons);
            }
            None => {
                let _ = writeln!(s, "  \"shortcut\": {{\"first\": {}, \"icons\": {}}},", sc.first, sc.icons);
            }
        }
    }
    if let Some(mn) = &b.menu {
        let words = mn.words.iter().map(|(l, w)| format!("\"{l}\": [{}]", w.iter().map(|(id, g)| format!("[{id}, {g}]")).collect::<Vec<_>>().join(", "))).collect::<Vec<_>>().join(", ");
        let items = mn.items_list.map(|g| format!("\"items_list\": {g}, ")).unwrap_or_default();
        let summons = mn.summon_list.map(|g| format!("\"summon_list\": {g}, ")).unwrap_or_default();
        // 0.179.1, optional (older DLLs don't read it).
        let summons = summons + &mn.dp_icon.map(|g| format!("\"dp_icon\": {g}, ")).unwrap_or_default();
        // Squeeze to fit (owner, 2026-11-04): each word's width in tenths of a unit; a key older
        // DLLs don't read.
        let widths = mn.widths.iter().map(|(l, w)| format!("\"{l}\": [{}]", w.iter().map(|(id, x)| format!("[{id}, {x}]")).collect::<Vec<_>>().join(", "))).collect::<Vec<_>>().join(", ");
        let _ = writeln!(s, "  \"menu\": {{\"first\": {}, \"tab_icon\": {}, {items}{summons}\"words\": {{{words}}}, \"word_widths\": {{{widths}}}}},", mn.first, mn.tab_icon);
    }
    let _ = writeln!(s, "  \"languages\": [{}],", langs.iter().map(|l| format!("\"{l}\"")).collect::<Vec<_>>().join(", "));
    let _ = writeln!(s, "  \"atlas\": [{}, {}, {}, {}],", b.atlas.0, b.atlas.1, b.atlas.2, b.atlas.3);
    let _ = writeln!(s, "  \"sources\": {{");
    for (i, (k, v)) in sources.iter().enumerate() {
        let _ = writeln!(s, "    \"{k}\": \"{v}\"{}", if i + 1 < sources.len() { "," } else { "" });
    }
    let _ = writeln!(s, "  }}");
    let _ = write!(s, "}}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp_bar::Layd;

    fn ring() -> Ring {
        Ring { x: 443 * 16, y: 316 * 16, uc: kh1::RING_HP.0, vc: kh1::RING_HP.1, r: kh1::RING_HP.2 }
    }

    #[test]
    fn fan_vertices_follow_kh1() {
        let r = ring();
        // 0°: west edge at the centre's height; 90°: top at the centre column; 180°: east edge one
        // unit out; 270°: bottom one unit down.
        assert_eq!(vertex(&r, 0.0), [r.x - r.r, r.y, r.uc - r.r, r.vc]);
        assert_eq!(vertex(&r, 90.0), [r.x, r.y - r.r, r.uc, r.vc - r.r]);
        assert_eq!(vertex(&r, 180.0), [r.x + 16 + r.r, r.y, r.uc - r.r, r.vc]);
        assert_eq!(vertex(&r, 270.0), [r.x, r.y + 16 + r.r, r.uc, r.vc - r.r]);
        assert_eq!(vertex(&r, 45.0), [r.x - r.r, r.y - r.r, r.uc - r.r, r.vc - r.r]);
        // 45°-steps between the ends, then the end.
        let f = fan(&r, 0.0, 100.0);
        assert_eq!(f.len(), 1 + 1 + 2 + 1);
        assert_eq!(f[0], [r.x, r.y + 16, r.uc, r.vc]);
    }

    fn solid(w: usize, h: usize) -> Pic {
        Pic { w, h, px: (0..w * h).map(|i| [(i % w) as u8, (i / w) as u8, 0, 0x80]).collect() }
    }

    #[test]
    fn baked_quadrants_tile_without_overlap() {
        let src = Source { ps2: solid(128, 128), hd: solid(256, 256), scale: 2 };
        let r = ring();
        let full = bake(&r, 0.0, 90.0, &[&src]).unwrap();
        let a = bake(&r, 0.0, 45.0, &[&src]).unwrap();
        let b = bake(&r, 45.0, 90.0, &[&src]).unwrap();
        // Same texels where the halves draw; no unit drawn by both.
        let at = |p: &Baked, x: i32, y: i32| -> Option<[u8; 4]> {
            let (px, py) = (x - p.rect.0, y - p.rect.1);
            (px >= 0 && py >= 0 && px < p.rect.2 && py < p.rect.3).then(|| p.ps2.px[(py * p.rect.2 + px) as usize]).filter(|c| c[3] != 0)
        };
        let mut n = 0;
        for y in full.rect.1..full.rect.1 + full.rect.3 {
            for x in full.rect.0..full.rect.0 + full.rect.2 {
                let (f, ha, hb) = (at(&full, x, y), at(&a, x, y), at(&b, x, y));
                assert!(!(ha.is_some() && hb.is_some()), "overlap at {x},{y}");
                if let Some(c) = ha.or(hb) {
                    assert_eq!(Some(c), f);
                    n += 1;
                }
            }
        }
        assert!(n > 1000);
        // The quadrant spans the KH1 square: x from 443 − 41 to 443, y from 316 − 41.
        assert_eq!((full.rect.0, full.rect.1), (402, 275));
        // Texel mapping: unit (403, 300) shows texel (403 − 400.5 → 2, 300 − 216.5 → 83).
        assert_eq!(at(&full, 403, 300).unwrap()[..2], [2, 83]);
    }

    #[test]
    fn tracks_follow_kh1_keys() {
        use crate::uibin::Key;
        let k = |dur: i32, x: f32, y: f32| Key { dur, vals: vec![x, y, 128.0, 128.0] };
        // vibe_s: 0, 3, -3, 2, -2, end 0 (1 unit each).
        let t = motion::track(&[k(1, 0.0, 0.0), k(1, 0.0, 3.0), k(1, 0.0, -3.0), k(1, 0.0, 2.0), k(1, 0.0, -2.0), k(-5, 0.0, 0.0)]).unwrap();
        assert_eq!(t.len, 10);
        assert_eq!(t.segs[0], ((0, 2), (0, 0), (0, 3)));
        assert_eq!(t.segs[4], ((8, 10), (0, -2), (0, 0)));
        assert_eq!(t.end, (0, 0));
        // A moving step's frames are exclusive and interpolate (x0, y0) → (x1, y1); the hold keeps
        // the end to LONG.
        let p = Piece { item: 0, uv: [0, 0, 1, 1], rect: [0, 0, 1, 1], colour: 0, grad: 0 };
        let st = motion::steps(&t, &[p]);
        assert_eq!(st.len(), 6);
        assert_eq!((st[1].flags, st[1].frames, st[1].tr), (FLAGS_MOVE, (2, 4), [0, 3, 0, -3]));
        assert_eq!((st[5].flags, st[5].frames), (FLAGS_STATIC, (10, LONG)));
        assert!(motion::track(&[k(1, 0.0, 0.0)]).is_err());
    }

    #[test]
    fn soft_end_fades_the_last_degrees() {
        let src = Source { ps2: solid(128, 128), hd: solid(256, 256), scale: 2 };
        let r = ring();
        let (f, w) = fan_soft(&r, 0.0, 100.0, true);
        assert_eq!(f.last().copied(), Some(vertex(&r, 100.0 - SOFT_DEG)));
        assert_eq!(w.unwrap()[2], vertex(&r, 100.0));
        assert!(fan_soft(&r, 0.0, 270.0, true).1.is_none());
        let soft = bake_soft(&r, 0.0, 100.0, true, &[&src]).unwrap();
        let hard = bake(&r, 0.0, 100.0, &[&src]).unwrap();
        // The same footprint but for the hard end's half-texel overlap (PC copies, 0.37.0: at
        // most one unit more); some texels near the end edge are part-transparent.
        let (a, b) = (soft.rect, hard.rect);
        assert!((a.0 - b.0).abs() <= 1 && (a.1 - b.1).abs() <= 1 && (a.2 - b.2).abs() <= 1 && (a.3 - b.3).abs() <= 1, "{a:?} {b:?}");
        if a != b {
            return;
        }
        assert!(soft.ps2.px.iter().zip(&hard.ps2.px).any(|(a, b)| a[3] < b[3] && a[3] > 0) || soft.hd[0].px.iter().zip(&hard.hd[0].px).any(|(a, b)| a[3] < b[3] && a[3] > 0));
    }

    #[test]
    fn cut_matches_kh1() {
        assert_eq!(cut(1, 6, 26), 22);
        assert_eq!(cut(5, 6, 26), 4);
        assert_eq!(cut(29, 30, 24), 1);
        assert_eq!(cut(1, 30, 24), 23);
    }

    #[test]
    fn layd_and_imgz_read_back() {
        let s = Seqd { version: 0x160, sprites: vec![sprite([0, 0, 1, 1], [0x8080_8080; 4])], parts: vec![part([0, 0, 1, 1], 0)], sgroups: vec![sgroup(0, 1)], anims: vec![anim(FLAGS_STATIC, 0, (0, 1), 0, (1.0, 1.0), (0, 0))], agroups: vec![agroup(0, 1, true, 1)] };
        let l = layd(&s.write());
        let p = Layd::parse(&l).unwrap();
        assert_eq!(p.seqs.len(), 1);
        assert_eq!(Seqd::parse(&p.seqs[0]).unwrap().agroups.len(), 1);
        let z = imgz(&Pic::blank(1024, 512));
        assert_eq!(&z[0x20..0x24], b"IMGD");
        let w = |at: usize| u32::from_le_bytes(z[0x20 + at..0x24 + at].try_into().unwrap());
        // As UK_Cockpit.imz's words: 0x100, 0x40, size, size + 0x40, 0, −1, 1024 / 512, …
        assert_eq!((w(4), w(8), w(0xC), w(0x10), w(0x14), w(0x18)), (0x100, 0x40, 1024 * 512 * 4, 1024 * 512 * 4 + 0x40, 0, 0xFFFF_FFFF));
        assert_eq!(w(0x1C), 512 << 16 | 1024);
        assert_eq!(w(0x20), 9 << 16 | 10);
        assert_eq!((w(0x24), w(0x28), w(0x2C), w(0x30), w(0x34), w(0x38), w(0x3C)), (0x10, 0xFFFF_FFFF, 0x0010_0010, 0x0013_0001, 3, 0, 3));
        assert_eq!(z.len(), 0x20 + 0x40 + 1024 * 512 * 4);
    }

    #[test]
    fn held_groups_extend_the_last_frame() {
        let p = |x: i32| Piece { item: 0, uv: [0, 0, 1, 1], rect: [x, 0, x + 1, 1], colour: 0, grad: 0 };
        let g = held(&[vec![], vec![p(0)], vec![p(0), p(1)]]);
        let mut runs: Vec<(Vec<Piece>, (i32, i32))> = g.steps.iter().map(|s| (s.pieces.clone(), s.frames)).collect();
        runs.sort_by_key(|r| r.1);
        assert_eq!(runs, vec![(vec![p(0)], (1, LONG)), (vec![p(1)], (2, LONG))]);
    }

    #[test]
    fn languages_from_cockpit_copies() {
        let a = |n: &str| HdAsset { name: n.into(), orig_offset: -1, data: vec![] };
        assert_eq!(languages(&[a("UK_Cockpit.lad"), a("FR_gauge.lad"), a("FR_Cockpit.lad"), a("Cockpit.lad")]), vec!["FR", "UK"]);
    }

    /// With `KH1_HUD_DUMP` set to a `hud-h0` dump folder (the player's own files, outside the
    /// repository): builds the set from it and writes the atlases and layouts to `KH1_HUD_OUT`.
    /// 0.179.1: with `KH1_DP_EMBLEM_IMZ` set to the player's `FO0104.RTM` `UK_levelup.imz` (outside
    /// the repository): the emblem is found, at the font's height, mostly opaque in its middle;
    /// `KH1_DP_EMBLEM_OUT` set: its raw RGBA (PS2 alpha) is written there for a look.
    /// 0.180.0: KH1's auto-target ring from the player's `UK_uibin_lock.bin` (`KH1_LOCK_BIN`)
    /// and, with `KH1_UITEX`, its texels covered in `uitex.bin`'s picture 10.
    #[test]
    fn auto_ring_from_the_players_file() {
        let Ok(p) = std::env::var("KH1_LOCK_BIN") else { return };
        let lock = std::fs::read(p).unwrap();
        let mut groups = Vec::new();
        let c = cursor::build(&lock, 0, &mut groups).unwrap();
        let r = cursor::build_ring(&lock, 0, &mut groups).unwrap();
        assert_eq!((c, r), (0, 1));
        let ring = &groups[r];
        let last = ring.steps.last().unwrap().pieces[0];
        // Settles at 30 × 30 centred, alpha 112 of KH1's orange (112, 80, 12).
        assert_eq!(last.rect, [-15, -15, 15, 15]);
        assert_eq!(last.colour, 112 | 80 << 8 | 12 << 16 | 112 << 24);
        assert_eq!(last.uv, [33, 1, 63, 31]);
        // The first frame is the larger, faint one; the cursor's own group is unchanged.
        assert!(ring.steps[0].pieces[0].rect[2] > 15 && ring.steps[0].pieces[0].colour >> 24 < 112);
        assert_eq!(groups[c].steps.last().unwrap().pieces[0].rect, [-19, -19, 19, 19]);
        if let Ok(u) = std::env::var("KH1_UITEX") {
            let tims = mb::tim2_pictures(&std::fs::read(u).unwrap()).unwrap();
            let t = &tims[CURSOR_PICTURE.0];
            let covered = (1..31).flat_map(|y| (33..63).map(move |x| (x, y))).filter(|&(x, y)| t.pixels[y * t.width + x][3] > 0x10).count();
            eprintln!("lockon_d texels covered: {covered} of 900");
            assert!(covered > 100, "the ring's texels are drawn in picture {}", CURSOR_PICTURE.0);
        }
    }

    #[test]
    fn dp_emblem_from_the_players_file() {
        let Ok(p) = std::env::var("KH1_DP_EMBLEM_IMZ") else { return };
        let e = menu::dp_emblem(&std::fs::read(p).unwrap()).unwrap();
        assert_eq!(e.h, menu::CELL.1);
        assert!(e.w % 2 == 0 && e.w > e.h / 2 && e.w < e.h, "{}×{}", e.w, e.h);
        assert!(e.px[(e.h / 2) * e.w + e.w / 2][3] > 0x40, "the middle is covered");
        if let Ok(out) = std::env::var("KH1_DP_EMBLEM_OUT") {
            std::fs::write(out, e.px.iter().flatten().copied().collect::<Vec<u8>>()).unwrap();
        }
        eprintln!("dp emblem {}×{}", e.w, e.h);
    }

    #[test]
    fn builds_from_a_dump() {
        let Ok(dir) = std::env::var("KH1_HUD_DUMP") else { return };
        let d = std::path::Path::new(&dir).join("kh1");
        let rd = |p: &str| std::fs::read(d.join(p)).unwrap();
        let assets = |p: &str| -> Vec<HdAsset> {
            std::fs::read_dir(d.join(p)).map(|r| r.flatten().filter(|e| !e.file_name().to_string_lossy().ends_with(".png")).map(|e| HdAsset { name: e.file_name().to_string_lossy().into_owned(), orig_offset: -1, data: std::fs::read(e.path()).unwrap() }).collect()).unwrap_or_default()
        };
        let face = |n: &str, at: usize| face_section(&rd(&format!("faces/{n}.sec2.bin")), at, &assets(&format!("faces/{n}.hd"))).unwrap();
        let uitex_hd = assets("command2/uitex.bin.hd");
        let (com, com_battle, mag) = (rd("exchange/UK_uibin_com.bin"), rd("exchange/UK_com_battle.bin"), rd("exchange/UK_uibin_mag.bin"));
        let (lock, g_enm) = (rd("exchange/UK_uibin_lock.bin"), rd("exchange/UK_uibin_g_enm.bin"));
        let cut = rd(CUT);
        let item = rd(ITEM);
        // M8b: optional in the dump.
        let sum = std::fs::read(d.join(SUM)).ok();
        let fonts: Vec<Vec<u8>> = vec![rd(&format!("fonts/menu/us/sysfont.bin.hd/{SYSFONT_HD}")), rd("fonts/exchange/US_font_data_tbl.bin")];
        let icon: Vec<Vec<u8>> = ICON_FILES.iter().map(|n| rd(&format!("fonts/{n}"))).collect();
        let src = Sources {
            uitex: &rd("command2/uitex.bin"),
            uitex_hd: &uitex_hd,
            g_s: &rd("exchange/UK_uibin_g_s.bin"),
            g_red: &rd("exchange/UK_uibin_g_red.bin"),
            g_act: &rd("exchange/UK_uibin_g_act.bin"),
            lock: Some(&lock),
            g_c: Some(&rd("exchange/UK_uibin_g_c.bin")),
            lev_s: Some(&rd("exchange/UK_uibin_lev_s.bin")),
            g_enm: Some(&g_enm),
            menu: Some(MenuSources {
                com: &com,
                com_battle: &com_battle,
                mag: &mag,
                font: [&fonts[0], &fonts[1]],
                words: vec![("us".into(), vec![(1, "Attack".into()), (2, "Magic".into()), (3, "Items".into()), (4, "Talk".into()), (6, "Summon".into()), (305, "-".into()), (294, "Open".into()), (313, "Jump On".into()), (8, "Fira".into()), (16, "Cure".into()), (13, "Thunder".into()), (31, "Potion".into()), (32, "Hi-Potion".into()), (38, "Megalixir".into()), (903, "Party".into())].into_iter().chain(crate::command_menu::PARTY_WORDS.iter().map(|&(id, t)| (id, t.to_string()))).collect())],
                icon: Some([&icon[0], &icon[1]]),
                cut: Some(&cut),
                item: Some(&item),
                sum: sum.as_deref(),
                dp_emblem: None,
            }),
            faces: [face("xa_ex_0010.mdls", 0xA2E80), face("xa_ex_0040.mdls", 0x75600), face("xa_ex_0050.mdls", 0x8D400)],
        };
        let langs = ["FR", "GR", "IT", "SP", "UK"];
        let b = build(&src, &langs).unwrap();
        // The owner's FO0002.SPR / .RTM with their PC assets: the set goes in, Re:CoM's stays.
        let r = std::path::Path::new(&dir).join("recom/FORM/0002");
        let ra = |p: &str| -> Vec<HdAsset> {
            let mut v: Vec<HdAsset> = std::fs::read_dir(r.join(p)).unwrap().flatten().filter(|e| !e.file_name().to_string_lossy().ends_with(".png")).map(|e| HdAsset { name: e.file_name().to_string_lossy().into_owned(), orig_offset: 0xFFFF, data: std::fs::read(e.path()).unwrap() }).collect();
            v.sort_by(|a, b| a.name.cmp(&b.name));
            v
        };
        let spr = std::fs::read(r.join("FO0002.SPR")).unwrap();
        let mut sa = ra("FO0002.SPR.hd");
        let ls = languages(&sa);
        assert_eq!(ls, langs);
        let spr2 = add_to_spr(&spr, &mut sa, &b, &ls).unwrap();
        let c = Container::parse(&spr2).unwrap();
        let c0 = Container::parse(&spr).unwrap();
        assert_eq!(c.entries.len(), c0.entries.len() + 1);
        assert_eq!(c.entries[..c0.entries.len()], c0.entries[..]);
        assert!(sa.iter().any(|a| a.name == "UK_kh1hud.lad"));
        assert!(add_to_spr(&spr2, &mut sa.clone(), &b, &ls).is_err());
        let rtm = std::fs::read(r.join("FO0002.RTM")).unwrap();
        let mut rta = ra("FO0002.RTM.hd");
        let rtm2 = add_to_rtm(&rtm, &mut rta, &b, &ls).unwrap();
        let c = Container::parse(&rtm2).unwrap();
        assert_eq!(c.entries.last().unwrap().name(), IMZ);
        assert_eq!(c.entries.last().unwrap().name_raw[0x14..], Container::parse(&rtm).unwrap().entries[0].name_raw[0x14..]);
        assert_eq!(rta.len(), 10);
        let sq = Seqd::parse(&Layd::parse(&b.lad).unwrap().seqs[0]).unwrap();
        eprintln!(
            "atlas {:?}, {} items, {} sprites, lad {} / {} bytes; {} groups, {} units ({} with slides), {} runs, {} animations, {} parts",
            b.atlas,
            b.pieces,
            b.sprites,
            b.lad.len(),
            b.lad_hd.len(),
            b.groups,
            b.motion.first.len(),
            b.motion.unit_slides.iter().filter(|&&x| x).count(),
            b.motion.runs.len(),
            sq.anims.len(),
            sq.parts.len()
        );
        assert_eq!(sq.agroups.len(), b.groups);
        // The MP ring as Re:CoM's HP ring: its groups last; cuts between cells drop no visible
        // texel of KH1's art (the gaps are clear on PC).
        let (rf, rs) = b.ring;
        let ring_anims = sq.anims.len() - u16::from_le_bytes([sq.agroups[rf][0], sq.agroups[rf][1]]) as usize;
        eprintln!("mp ring: groups {rf}..{}, {ring_anims} animations; cuts {rs:?}", rf + ring_group::COUNT);
        assert_eq!(rf + ring_group::COUNT, b.groups);
        assert!(rs.gap_alpha < 16, "{rs:?}");
        let mn = b.menu.as_ref().unwrap();
        eprintln!("refit {}", b.refit);
        assert!(b.refit > 0);
        eprintln!("menu: first group {}, {} glyphs, missing {:?}, words {:?}", mn.first, mn.glyphs, mn.missing, mn.words);
        assert!(mn.missing.is_empty());
        // Squeeze to fit: each word's width (tenths of a unit), the pen after its last glyph.
        let width = |id: u16| mn.widths[0].1.iter().find(|w| w.0 == id).map(|w| w.1);
        eprintln!("word widths: {:?}", mn.widths[0].1);
        assert_eq!(mn.widths.len(), mn.words.len());
        assert!(width(32).is_some_and(|w| (600..760).contains(&w)), "Hi-Potion {:?}", width(32));
        assert!(width(1).is_some_and(|w| w < width(38).unwrap()), "Attack narrower than Megalixir");
        // 0.111.0: "Party" and the Party names ("Surprise!" needs the font's '!').
        assert!(width(903).is_some() && crate::command_menu::PARTY_WORDS.iter().all(|&(id, _)| width(id).is_some()));
        assert!(mn.tab_icon, "the PlayStation triangle from Icon.tm2");
        // M8 stage 8a.1: the Items list's six groups after the words, before the shortcut panel.
        let il = mn.items_list.expect("the Items list from UK_uibin_item.bin");
        assert!(mn.words.iter().all(|(_, w)| w.iter().all(|&(_, g)| g < il)));
        // Stage 7.12: the shortcut panel, its glyphs from Icon.tm2.
        let sc = b.shortcut.unwrap();
        eprintln!("shortcut panel: first group {}", sc.first);
        assert!(sc.icons && sc.first + shortcut::g::COUNT <= b.ring.0);
        // M8b: the Summon list's six after the Items list's, when the owner's dump has it.
        eprintln!("summon list: {:?} {:?}", mn.summon_list, mn.summon_note);
        let after = mn.summon_list.map_or(il + 6, |sl| {
            assert_eq!(sl, il + 6);
            sl + 6
        });
        assert_eq!(after, sc.first);
        assert!(b.target.is_some());
        assert!(b.party.is_some());
        assert!(b.cursor.is_some());
        assert_eq!(b.auto_ring, b.cursor.map(|c| c + 1));
        assert!(b.level_up.is_some());
        // KH1's g_act tracks at 2 frames a key unit: vibe_s 5 units, vibe_ss 11, vibe_sss 19,
        // the slides 10.
        assert_eq!(b.motion.programs.iter().map(|p| p.len).collect::<Vec<_>>(), vec![10, 22, 38, 10, 10, 20, 20]);
        if let Ok(out) = std::env::var("KH1_HUD_OUT") {
            let o = std::path::Path::new(&out);
            std::fs::write(o.join("kh1hud.lad"), &b.lad).unwrap();
            std::fs::write(o.join("UK_kh1hud.lad"), &b.lad_hd).unwrap();
            std::fs::write(o.join("kh1hud.imz"), &b.imz).unwrap();
            std::fs::write(o.join("UK_kh1hud.imz"), &b.imz_hd[4]).unwrap();
            std::fs::write(o.join("kh1-hud.json"), manifest_json(&b, &langs.map(String::from), &[], None)).unwrap();
        }
    }
}

#[cfg(test)]
mod hd_fit_tests {
    use super::*;

    fn solid(w: usize, h: usize, c: [u8; 4]) -> Pic {
        Pic { w, h, px: vec![c; w * h] }
    }

    #[test]
    fn a_stock_texture_is_kept_as_it_is() {
        let ps2 = solid(192, 64, [1, 2, 3, 128]);
        let hd = solid(384, 128, [9, 8, 7, 128]);
        assert_eq!(fit_hd(&ps2, hd.clone(), HD_SCALE), hd);
        // Picture 0 at KH1 FM PC's 4× stays 4×.
        let (p0, hd0) = (solid(128, 128, [0; 4]), solid(512, 512, [5, 5, 5, 128]));
        assert_eq!(fit_hd(&p0, hd0.clone(), HD_SCALE_PLATE), hd0);
    }

    #[test]
    fn a_texture_mod_face_is_fitted_to_the_stock_scale() {
        // A texture mod's Sora face: 1200×400 over the PS2's 192×64 (6.25×).
        let ps2 = solid(192, 64, [0; 4]);
        let hd = fit_hd(&ps2, solid(1200, 400, [200, 100, 50, 128]), HD_SCALE);
        assert_eq!((hd.w, hd.h), (384, 128));
        assert_eq!(hd_scale(&ps2, &hd).unwrap(), 2);
        assert!(hd.px.iter().all(|p| *p == [200, 100, 50, 128]));
        // Donald's / Goofy's at 8×, the uitex pictures at 4× and picture 2 (with its PC-only rows).
        let f = fit_hd(&solid(48, 48, [0; 4]), solid(384, 384, [1; 4]), HD_SCALE);
        assert_eq!((f.w, f.h), (96, 96));
        let p2 = fit_hd(&solid(128, 128, [0; 4]), solid(512, 768, [1; 4]), HD_SCALE);
        assert_eq!((p2.w, p2.h), (256, 384));
    }

    #[test]
    fn language_copies_end_at_one_size() {
        // A texture mod that replaced some languages' copies only.
        let ps2 = solid(128, 128, [0; 4]);
        let per = fit_all(&ps2, vec![solid(512, 512, [1; 4]), solid(256, 256, [2; 4]), solid(300, 300, [3; 4])], HD_SCALE);
        assert!(per.iter().all(|p| (p.w, p.h) == (256, 256)));
    }

    #[test]
    fn resampling_weights_colour_by_alpha() {
        // Half clear (black), half opaque red: the mean keeps the red, alpha halves.
        let mut p = solid(2, 1, [0, 0, 0, 0]);
        p.px[1] = [255, 0, 0, 128];
        let r = p.resampled(1, 1);
        assert_eq!(r.px[0], [255, 0, 0, 64]);
    }
}
