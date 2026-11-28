//! The KH1 HUD option (decision 39, `docs/recom-kh1-hud.md`): KH1's HUD elements drawn by Re:CoM's
//! 2D code from the `kh1hud` sprite set the asset step adds to Re:CoM's `FO0002.SPR` / `.RTM`
//! (`kh1-recom-assets` `kh1_hud`). This module holds what is pure: the manifest check and the
//! group numbering (the same as the asset step's).

/// The manifest's format line (as `kh1_recom_assets::kh1_hud::FORMAT`).
/// Format 3 (0.35.0): the menu's FM context tab and Triangle groups (format 2 sets have none:
/// refused, with the log's "run the asset step of this version").
pub const FORMAT: &str = "kh1-recom-assets kh1-hud 3";
/// The sprite set's name in Re:CoM's 2D registry, NUL-terminated.
pub const SET: &[u8] = b"kh1hud:00\0";

/// The animation groups of `kh1hud:00` (as `kh1_recom_assets::kh1_hud::group`).
pub mod group {
    /// Face frame, disc, labels: frame 0 without "mp", 1 with.
    pub const FRAME: u32 = 0;
    /// Back arcs and segments: frame = min(max HP, 120) / min(max MP, 20).
    pub const HP_BACK: u32 = 1;
    /// MP back.
    pub const MP_BACK: u32 = 2;
    /// MP: frame = min(MP, 20).
    pub const MP: u32 = 3;
    /// HP: frame = min(HP, 120).
    pub const HP: u32 = 4;
    /// Charge: frame = min(charge, 600).
    pub const CHARGE: u32 = 5;
    /// Max-MP ticks: frame = min(max MP, 20).
    pub const TICKS: u32 = 6;
    /// Face: frame = expression (0, 1 hurt, 2 danger).
    pub const FACE: u32 = 7;
    /// Hurt face for 40 / 50 / 60 frames then 0 or 2 (played): 8 + 2 i + (danger).
    pub const HURT: u32 = 8;
    /// MP fades of cell j (played): gain 14 + j, loss 23 + j.
    pub const MP_GAIN: u32 = 14;
    /// MP loss fades.
    pub const MP_LOSS: u32 = 23;
    /// Charge fades of [30 j, 30 j + t] (t = 1..30, played): gain 32 + 30 j + t − 1.
    pub const CHARGE_GAIN: u32 = 32;
    /// Charge loss fades: 302 + 30 j + t − 1.
    pub const CHARGE_LOSS: u32 = 302;
    /// Danger flash (looped) of the HP arc piece [90 q, 90 q + 5 k]: 572 + 18 q + k − 1.
    pub const RED_ARC: u32 = 572;
    /// Danger flash of a full straight HP segment.
    pub const RED_SEG: u32 = 626;
    /// Danger flash of a partial one (remainder r = 1..5): 627 + r − 1.
    pub const RED_SEG_PART: u32 = 627;
    /// The fixed groups (the motion units' groups follow: [`super::Motion`]).
    pub const COUNT: u32 = 632;
}

/// H2b: the motion programs of every unit (`kh1_recom_assets::kh1_hud::motion`), in this order.
pub mod program {
    /// The hit shakes `vibe_s`, `vibe_ss`, `vibe_sss` (strength 0, 1, 2).
    pub const VIBE: usize = 0;
    /// KH1's gauge fade out (10 frames).
    pub const FADE_OUT: usize = 3;
    /// KH1's gauge fade in.
    pub const FADE_IN: usize = 4;
    /// Programs every unit has.
    pub const BASE: usize = 5;
    /// The size-class slides (`slide_s_h` to class 0 / 1, `slide_s_v` to class 2; `0x14026D570`).
    pub const SLIDE_H: usize = 5;
    /// To class 2.
    pub const SLIDE_V: usize = 6;
    /// Programs in all.
    pub const COUNT: usize = 7;
    /// The names the manifest must list, in order.
    pub const NAMES: [&str; COUNT] = ["vibe_s", "vibe_ss", "vibe_sss", "fade_out", "fade_in", "slide_s_h", "slide_s_v"];
}

/// The slide KH1 plays for a change to `new_class` (`0x14026D570`: slot `+0x120` below class 2,
/// `+0x1E0` for class 2).
pub fn slide_program(new_class: usize) -> usize {
    if new_class < 2 {
        program::SLIDE_H
    } else {
        program::SLIDE_V
    }
}

/// A motion program: the frame it is played from and its length (frames).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Program {
    /// The group frame to play from.
    pub start: i32,
    /// Frames until it is over.
    pub frames: i32,
}

/// The motion units: each a played group per program, shown instead of the held layers while a
/// motion runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Motion {
    /// The programs ([`program`] order).
    pub programs: Vec<Program>,
    /// Each unit's first group and program count.
    pub units: Vec<(u32, u32)>,
    /// (held group, f0, f1, unit): the unit shows while the held group's frame is in [f0, f1].
    pub runs: Vec<(u32, i32, i32, u32)>,
}

impl Motion {
    /// The units held group `held` shows at `frame`, in the held group's order.
    pub fn units_at(&self, held: u32, frame: i32) -> Vec<usize> {
        self.runs.iter().filter(|r| r.0 == held && r.1 <= frame && frame <= r.2).map(|r| r.3 as usize).collect()
    }

    /// Unit `u`'s group for program `p` (None when the unit has no such program: the slides).
    pub fn group(&self, u: usize, p: usize) -> Option<u32> {
        let &(first, n) = self.units.get(u)?;
        ((p as u32) < n).then_some(first + p as u32)
    }
}

/// H4: the command menu look's groups (`kh1_recom_assets::kh1_hud::menu`).
pub mod menu {
    /// Header frame, title, row, last row, selection, icons a / b / c: a look's groups.
    pub const HEADER: u32 = 0;
    /// Title.
    pub const TITLE: u32 = 1;
    /// A row's frame.
    pub const ROW: u32 = 2;
    /// The last row.
    pub const ROW_LAST: u32 = 3;
    /// The selection.
    pub const SEL: u32 = 4;
    /// Icon a (Attack); b, c follow.
    pub const ICON: u32 = 5;
    /// Groups a look; the battle look follows the field's.
    pub const LOOK: u32 = 8;
    /// The title turning red (played from frame 1).
    pub const TITLE_TO_BATTLE: u32 = 2 * LOOK;
    /// The Magic list's header, title, row, last row, selection, icon.
    pub const MAGIC: u32 = 2 * LOOK + 1;
    /// A submenu list's groups (header, title, row, last row, selection, icon): the Magic list's
    /// and (M8 stage 8a.1) the Items list's.
    pub const LIST_GROUPS: u32 = 6;
    /// FM's context tab: shadow, green plate, band, stripe, at the tab's origin
    /// (`crate::command_menu::tab`).
    pub const TAB: u32 = MAGIC + 6;
    /// The Triangle (the PC's PlayStation glyph; empty when the asset step had no `Icon.tm2`).
    pub const TRIANGLE: u32 = TAB + 1;
    /// The first word group (the words are listed in the manifest).
    pub const WORDS: u32 = TAB + 2;
    /// KH1's root list origin (`0x14027B870`: (0x10, 0x104)), its header's height and row pitch,
    /// the selection's offset, the words' offsets (`0x140278B40`, `0x14027A920` adds (16, 2)).
    pub const HEADER_H: f32 = 20.0;
    /// Row pitch.
    pub const PITCH: f32 = 22.0;
    /// The selection's (x, y) from the list origin + 22 c.
    pub const SEL_AT: (f32, f32) = (11.0, 18.0);
    /// A row's word from the list origin + 22 i.
    pub const WORD_AT: (f32, f32) = (16.0, 22.0);
    /// The cursor row's word.
    /// y: 22, not KH1's 25: KH1's words sit at the cell's top and ours 2 units up (the 0.34.0
    /// font); 25 put "Attack" 3 units low in the cursor plate (owner's screenshot, 2026-11-01:
    /// "the font is oddly positioned when hovering over an option"). Centred as KH1's: Checked
    /// (owner's KH1 screenshot).
    pub const SEL_WORD_AT: (f32, f32) = (29.0, 22.0);
}

/// Squeeze to fit (owner, 2026-11-04: "we need a solution for certain sleights having too long of a
/// name to fit within the black box"; answer "Squeeze to fit"): a KH1-look word wider than its box
/// is drawn narrower with Re:CoM's own control X scale (`0x1402486D0`, the MP bar's partial
/// segment's), so it ends inside the box; a word that fits is unchanged. The word's pieces start at
/// its control's place (x 0), so the scale keeps its left edge (Assumption: the layer's screen
/// offset is outside the scale, as for the MP bar's segments; to see in game).
///
/// The boxes, from the word's left edge, measured on KH1's own art (Checked (files: the owner's
/// `uitex.bin` pictures 0, 1 and 3, `UK_uibin_sum.bin` / `_mag.bin` / `_cut.bin` rects)), less a
/// 2-unit margin:
/// - a Magic / Summon / Sleights list row: the row frame's (`*_fra_c` + `*_fra_2_c`, 0..112) black
///   inside ends at x 106 over the glyphs' rows (frame rows 2..17; 108..110 mid-row); the word at
///   x 16 → 90: box 88;
/// - the cursor row: the selection plate (`*_sel_act_d` + `_2_d`, the word at its x 18) carries
///   the list's hat icon (`*_sel_ico_d`, x 87..103, every texel column drawn): 87 − 18 = 69, box 67;
/// - the shortcut panel (`cut_fra_a`): each name box is black from x 6 to x 85 over the glyphs'
///   rows (87 mid-row), the word at x 16 → 70: box 68.
///
/// In the lists only our sleight names (`crate::sleights::WORD_BASE` words) are squeezed: KH1's own
/// words keep KH1's look (KH1 draws "Tinker Bell", 73.9 units, over its hat icon too). Every KH1
/// spell fits the shortcut box (the longest, "Thundaga", 61.4). Widths: KH1's menu font table
/// (`US_font_data_tbl.bin`, the asset step's layout; Checked (files)).
pub mod fit {
    /// A Magic / Summon / Sleights list row's box (units from the word's left edge).
    pub const LIST_BOX: f32 = 88.0;
    /// The cursor row's box (up to the hat icon).
    pub const CURSOR_BOX: f32 = 67.0;
    /// A shortcut panel name's box.
    pub const SHORTCUT_BOX: f32 = 68.0;

    /// The X scale of a word `width` units wide in a `box_w` box: `min(1, box / width)`; 1 when
    /// the width isn't known (an older asset step).
    pub fn scale(width: Option<f32>, box_w: f32) -> f32 {
        match width {
            Some(w) if w > box_w && box_w > 0.0 => box_w / w,
            _ => 1.0,
        }
    }

    /// A list row's box for word `id` (the cursor row's or another row's), or None for a word the
    /// lists never squeeze (KH1's own).
    pub fn list_box(id: u16, cursor_row: bool) -> Option<f32> {
        (id & 0xFF00 == crate::sleights::WORD_BASE).then_some(if cursor_row { CURSOR_BOX } else { LIST_BOX })
    }
}

/// Stage 7.12: KH1's shortcut panel (`kh1_recom_assets::kh1_hud::shortcut`), groups from its first
/// (`kh1-hud.json` `"shortcut": {"first", "icons"}`). The words are the menu's
/// ([`MenuGroups::word`]); KH1's places are `crate::command_menu::shortcut`.
pub mod shortcut {
    /// The frame (`cut_fra_a`, picture 0, its vertical gradient) with the title (`cut_shortcut`,
    /// picture 1) on it, at the panel's origin.
    pub const PANEL: u32 = 0;
    /// The PC's PlayStation glyphs (`Icon.tm2`), each a 16 × 19 quad at its own origin (as the
    /// context tab's Triangle): Triangle, Square, Cross, Circle (empty without `Icon.tm2`).
    pub const TRIANGLE: u32 = 1;
    /// Square.
    pub const SQUARE: u32 = 2;
    /// Cross (KH1's 0x41 with the confirm on Cross).
    pub const CROSS: u32 = 3;
    /// Circle (KH1's 0x41 with the confirm on Circle).
    pub const CIRCLE: u32 = 4;
    /// Groups.
    pub const COUNT: u32 = 5;
    /// The other pad types' glyph groups (`"pads"`): type 1's four, then type 3's.
    pub const PADS: u32 = 8;
}

/// The shortcut panel's groups of the manifest (stage 7.12).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShortcutGroups {
    /// First group.
    pub first: u32,
    /// The glyph groups hold the PC's button art.
    pub icons: bool,
    /// The other pad types' glyphs (`"pads"`, the asset step's `shortcut::pad_glyphs`): pad type
    /// 1's Triangle / Square / Cross / Circle places (the Xbox Y / X / A / B), then type 3's.
    pub pads: Option<u32>,
}

impl ShortcutGroups {
    /// The group of glyph `k` ([`shortcut::TRIANGLE`]..=[`shortcut::CIRCLE`]) for Re:CoM's pad
    /// type (its controller config `+0x1C`): as `0x140349120` picks its own button glyphs, 2 the
    /// PlayStation set, 3 the generic pad's, any other the Xbox set (Checked (exe)). Without
    /// `"pads"` (an older asset step) the PlayStation set.
    pub fn glyph(&self, k: u32, pad_type: u16) -> u32 {
        let i = k.saturating_sub(shortcut::TRIANGLE).min(3);
        match (pad_type, self.pads) {
            (2, _) | (_, None) => self.first + k,
            (3, Some(p)) => p + 4 + i,
            (_, Some(p)) => p + i,
        }
    }
}

/// H5: the LOCK ON! plate and the target's HP gauge (`kh1_recom_assets::kh1_hud::target`). KH1's
/// rules (`0x1402747F0`, `0x1402758B0`, `0x140275000`; Checked (decomp)).
pub mod target {
    /// The plate (played), the gauge's head, body (frame = L), cap, bars 0..4 (frame = length),
    /// the marker.
    pub const PLATE: u32 = 0;
    /// Head.
    pub const HEAD: u32 = 1;
    /// Body.
    pub const BODY: u32 = 2;
    /// Cap.
    pub const CAP: u32 = 3;
    /// Bar 0.
    pub const BAR: u32 = 4;
    /// Marker.
    pub const MARKER: u32 = 9;
    /// Groups.
    pub const COUNT: u32 = 10;
    /// The origin (`0x140275000`: left margin + 16, 24; + 28 while the plate shows).
    pub const ORIGIN: (f32, f32) = (16.0, 24.0);
    /// The gauge's drop while the plate shows (`0x1402743E0`).
    pub const BELOW_PLATE: f32 = 28.0;
    /// The body / bars start at the origin + 28 (`0x2C` − 16).
    pub const BODY_X: f32 = 28.0;
    /// The gauge's fade a frame (`0x1402758B0`: ± 0.2).
    pub const GAUGE_FADE: f32 = 0.2;
    /// The plate's fade out a frame (`0x1402747F0` state 4: 1 / 6).
    pub const PLATE_FADE: f32 = 1.0 / 6.0;

    /// The body's length: min(max HP / 2, 150).
    pub fn body(max_hp: i32) -> i32 {
        (max_hp.max(0) / 2).min(150)
    }

    /// The five bars' lengths: bar k = min(HP − 300 k + 1, 300) / 2 while HP ≥ 300 k, else 0.
    pub fn bars(hp: i32) -> [i32; 5] {
        core::array::from_fn(|k| {
            let rest = hp - 300 * k as i32;
            if hp < 0 || rest < 0 {
                0
            } else {
                (rest + 1).min(300) / 2
            }
        })
    }

    /// The marker's bar (the top one with a length) when more than one bar shows.
    pub fn marker(bars: &[i32; 5]) -> Option<usize> {
        (1..5).rev().find(|&k| bars[k] > 0)
    }
}

/// H6: the party slots' offsets (`0x140527330` Donald (0, 0), `0x140527370` Goofy (68, 20);
/// Checked (exe)), added to the objects' places with the right margin.
pub const PARTY_OFFSET: [(f32, f32); 2] = [(0.0, 0.0), (68.0, 20.0)];

/// H8: Re:CoM's actor popups (`0x14020C9A0(actor, kind)`, `CCommonGauge`): kind 4 is the level
/// up (its only caller `0x1401E5730` raises Sora's level, then plays sound 0x36; Checked
/// (decomp)). KH1's LEVEL UP!! box at (left margin + 16, 16) (`0x140272F10`) replaces it.
pub const POPUP_LEVEL_UP: i32 = 4;
/// The LEVEL UP!! box's place (Sora's column, `0x140272F10`).
pub const LEVEL_UP_AT: (f32, f32) = (16.0, 16.0);

/// The menu groups of the manifest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuGroups {
    /// First group.
    pub first: u32,
    /// The Triangle's group holds the glyph.
    pub tab_icon: bool,
    /// (language, [(message id, group)]).
    pub words: Vec<(String, Vec<(u16, u32)>)>,
    /// M8 stage 8a.1: KH1's Items list (`exchange/<L>_uibin_item.bin`), its first group (the
    /// same six as the Magic list's, [`menu::MAGIC`]'s order), when the asset step built it.
    pub items_list: Option<u32>,
    /// M8b: KH1's Summon list (`exchange/<L>_uibin_sum.bin`), its first group (the same six), when
    /// the asset step built it.
    pub summon_list: Option<u32>,
    /// Squeeze to fit (owner, 2026-11-04): each word's width (tenths of a unit), by language as
    /// [`MenuGroups::words`]; empty from an asset step before 0.107.0 (no word is squeezed).
    pub widths: Vec<(String, Vec<(u16, u32)>)>,
    /// 0.179.1: the DP sleights' mark (Re:CoM's Heartless emblem), drawn right of their names; None
    /// from an older asset step or without Re:CoM's `FO0104.RTM` art.
    pub dp_icon: Option<u32>,
}

impl MenuGroups {
    /// The width (units) of word `id` in `lang` (else in "us", as [`MenuGroups::word`]).
    pub fn width(&self, lang: &str, id: u16) -> Option<f32> {
        let in_lang = |l: &str| self.words.iter().any(|w| w.0 == l && w.1.iter().any(|x| x.0 == id));
        let l = if in_lang(lang) { lang } else { "us" };
        self.widths.iter().find(|w| w.0 == l).and_then(|w| w.1.iter().find(|x| x.0 == id)).map(|x| x.1 as f32 / 10.0)
    }

    /// The group of word `id` in `lang`, else in "us".
    pub fn word(&self, lang: &str, id: u16) -> Option<u32> {
        let find = |l: &str| self.words.iter().find(|w| w.0 == l).and_then(|w| w.1.iter().find(|x| x.0 == id)).map(|x| x.1);
        find(lang).or_else(|| find("us"))
    }
}

/// What `kh1-hud.json` says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// The set's group count.
    pub groups: u32,
    /// The languages the PC copies were built for.
    pub languages: Vec<String>,
    /// The motion units (H2b).
    pub motion: Motion,
    /// The command menu look (H4), when built.
    pub menu: Option<MenuGroups>,
    /// KH1's shortcut panel (stage 7.12), when built (needs `UK_uibin_cut.bin` and the menu).
    pub shortcut: Option<ShortcutGroups>,
    /// The LOCK ON! plate and target gauge's first group (H5), when built.
    pub target: Option<u32>,
    /// Donald's and Goofy's gauge groups (H6), when built.
    pub party: Option<[u32; 2]>,
    /// KH1's lock-on cursor group (H7), when built.
    pub cursor: Option<u32>,
    /// KH1's orange auto-target ring group (`lockon_d`, 0.180.0), when built.
    pub auto_ring: Option<u32>,
    /// KH1's LEVEL UP!! box group (H8), when built.
    pub level_up: Option<u32>,
    /// The first of Re:CoM's HP ring groups turned 180° in `gauge:01` ([`recom_hp`]), when the
    /// asset step added them.
    pub recom_hp: Option<u32>,
    /// The first of KH1's MP ring groups built as Re:CoM's HP ring ([`mp_ring::group`]), when
    /// built (an older asset step: the cluster's baked MP groups).
    pub mp_ring: Option<u32>,
}

/// Reads `kh1-hud.json`; refuses another format or group count than this DLL's.
pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let v = crate::json::parse(text)?;
    let format = v.get("format").and_then(|x| x.as_str()).ok_or("no format")?;
    if format != FORMAT {
        return Err(format!("format {format:?}, this DLL reads {FORMAT:?} (run the asset step of this version)"));
    }
    let set = v.get("set").and_then(|x| x.as_str()).ok_or("no set")?;
    if set.as_bytes() != &SET[..SET.len() - 1] {
        return Err(format!("set {set:?}, not kh1hud:00"));
    }
    let groups = v.get("groups").and_then(|x| x.as_u32()).ok_or("no groups")?;
    let base = v.get("base_groups").and_then(|x| x.as_u32()).ok_or("no base_groups")?;
    if base != group::COUNT {
        return Err(format!("{base} fixed groups, this DLL expects {}", group::COUNT));
    }
    let languages = v.get("languages").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|l| l.as_str().map(str::to_string)).collect()).unwrap_or_default();
    let mut programs = Vec::new();
    for (i, p) in v.get("programs").and_then(|x| x.as_array()).ok_or("no programs")?.iter().enumerate() {
        let name = p.get("name").and_then(|x| x.as_str()).unwrap_or("");
        if program::NAMES.get(i) != Some(&name) {
            return Err(format!("program {i} is {name:?}, this DLL expects {:?}", program::NAMES.get(i)));
        }
        let start = p.get("start").and_then(|x| x.as_u32()).ok_or("program without start")? as i32;
        let frames = p.get("frames").and_then(|x| x.as_u32()).ok_or("program without frames")? as i32;
        programs.push(Program { start, frames });
    }
    if programs.len() != program::COUNT {
        return Err(format!("{} programs, this DLL expects {}", programs.len(), program::COUNT));
    }
    let nums = |x: &crate::json::Value| -> Option<Vec<u32>> { x.as_array()?.iter().map(|n| n.as_u32()).collect() };
    let mut units = Vec::new();
    for u in v.get("units").and_then(|x| x.as_array()).ok_or("no units")? {
        match nums(u).as_deref() {
            Some(&[first, n]) if (n as usize == program::BASE || n as usize == program::COUNT) && first >= group::COUNT && first + n <= groups => units.push((first, n)),
            _ => return Err("a bad unit".into()),
        }
    }
    let mut runs = Vec::new();
    for r in v.get("runs").and_then(|x| x.as_array()).ok_or("no runs")? {
        match nums(r).as_deref() {
            Some(&[g, f0, f1, u]) if g <= group::FACE && (u as usize) < units.len() => runs.push((g, f0 as i32, f1 as i32, u)),
            _ => return Err("a bad run".into()),
        }
    }
    let menu = match v.get("menu") {
        None => None,
        Some(m) => {
            let first = m.get("first").and_then(|x| x.as_u32()).ok_or("menu without first")?;
            if first + menu::WORDS > groups {
                return Err("menu groups past the set".into());
            }
            let mut words = Vec::new();
            for lang in ["us", "uk", "fr", "gr", "it", "sp"] {
                if let Some(a) = m.get("words").and_then(|w| w.get(lang)).and_then(|x| x.as_array()) {
                    let mut list = Vec::new();
                    for e in a {
                        match nums(e).as_deref() {
                            Some(&[id, g]) if g < groups && id <= u16::MAX as u32 => list.push((id as u16, g)),
                            _ => return Err("a bad menu word".into()),
                        }
                    }
                    words.push((lang.to_string(), list));
                }
            }
            let tab_icon = matches!(m.get("tab_icon"), Some(crate::json::Value::Bool(true)));
            // M8 stage 8a.1, optional (older manifests have none).
            let items_list = match m.get("items_list").map(|x| x.as_u32()) {
                None => None,
                Some(Some(g)) if g + menu::LIST_GROUPS <= groups => Some(g),
                Some(_) => return Err("a bad items_list entry".into()),
            };
            // M8b, optional (older manifests, or a `uibin_sum.bin` not laid out as the Magic list's).
            let summon_list = match m.get("summon_list").map(|x| x.as_u32()) {
                None => None,
                Some(Some(g)) if g + menu::LIST_GROUPS <= groups => Some(g),
                Some(_) => return Err("a bad summon_list entry".into()),
            };
            // 0.179.1, optional.
            let dp_icon = match m.get("dp_icon").map(|x| x.as_u32()) {
                None => None,
                Some(Some(g)) if g < groups => Some(g),
                Some(_) => return Err("a bad dp_icon entry".into()),
            };
            // Squeeze to fit (0.107.0), optional: [id, width in tenths of a unit] per language.
            let mut widths = Vec::new();
            for lang in ["us", "uk", "fr", "gr", "it", "sp"] {
                if let Some(a) = m.get("word_widths").and_then(|w| w.get(lang)).and_then(|x| x.as_array()) {
                    let mut list = Vec::new();
                    for e in a {
                        match nums(e).as_deref() {
                            Some(&[id, w]) if id <= u16::MAX as u32 => list.push((id as u16, w)),
                            _ => return Err("a bad word width".into()),
                        }
                    }
                    widths.push((lang.to_string(), list));
                }
            }
            Some(MenuGroups { first, tab_icon, words, items_list, summon_list, widths, dp_icon })
        }
    };
    let shortcut = match v.get("shortcut") {
        None => None,
        Some(t) => {
            let first = t.get("first").and_then(|x| x.as_u32()).ok_or("shortcut without first")?;
            if first + shortcut::COUNT > groups {
                return Err("shortcut groups past the set".into());
            }
            let icons = matches!(t.get("icons"), Some(crate::json::Value::Bool(true)));
            let pads = match t.get("pads") {
                None => None,
                Some(p) => {
                    let p = p.as_u32().ok_or("shortcut pads not a number")?;
                    if p + shortcut::PADS > groups {
                        return Err("shortcut pad glyphs past the set".into());
                    }
                    Some(p)
                }
            };
            Some(ShortcutGroups { first, icons, pads })
        }
    };
    let target = match v.get("target") {
        None => None,
        Some(t) => {
            let first = t.get("first").and_then(|x| x.as_u32()).ok_or("target without first")?;
            if first + target::COUNT > groups {
                return Err("target groups past the set".into());
            }
            Some(first)
        }
    };
    let party = match v.get("party").map(|p| nums(p)) {
        None => None,
        Some(Some(p)) if p.len() == 2 && p.iter().all(|&g| g < groups) => Some([p[0], p[1]]),
        Some(_) => return Err("a bad party entry".into()),
    };
    let cursor = match v.get("cursor").map(|c| c.as_u32()) {
        None => None,
        Some(Some(c)) if c < groups => Some(c),
        Some(_) => return Err("a bad cursor entry".into()),
    };
    let auto_ring = match v.get("auto_ring").map(|c| c.as_u32()) {
        None => None,
        Some(Some(c)) if c < groups => Some(c),
        Some(_) => return Err("a bad auto_ring entry".into()),
    };
    let level_up = match v.get("level_up").map(|c| c.as_u32()) {
        None => None,
        Some(Some(c)) if c < groups => Some(c),
        Some(_) => return Err("a bad level_up entry".into()),
    };
    let recom_hp = match v.get("recom_hp") {
        None => None,
        Some(r) => {
            let set = r.get("set").and_then(|x| x.as_str()).unwrap_or("");
            let of = r.get("of").and_then(nums).unwrap_or_default();
            let first = r.get("first").and_then(|x| x.as_u32()).ok_or("recom_hp without first")?;
            if set.as_bytes() != &recom_hp::SET[..recom_hp::SET.len() - 1] || of != recom_hp::TURNED || first != recom_hp::BASE_GROUPS {
                return Err(format!("recom_hp {set:?} {of:?} from {first}, this DLL expects gauge:01 {:?} from {}", recom_hp::TURNED, recom_hp::BASE_GROUPS));
            }
            Some(first)
        }
    };
    let mp_ring = match v.get("mp_ring") {
        None => None,
        Some(r) => {
            let first = r.get("first").and_then(|x| x.as_u32()).ok_or("mp_ring without first")?;
            let count = r.get("count").and_then(|x| x.as_u32()).ok_or("mp_ring without count")?;
            if count != mp_ring::group::COUNT || first < group::COUNT || first + count > groups {
                return Err(format!("mp_ring {first} + {count}, this DLL expects {} groups inside the set", mp_ring::group::COUNT));
            }
            Some(first)
        }
    };
    Ok(Manifest { groups, languages, motion: Motion { programs, units, runs }, menu, shortcut, target, party, cursor, auto_ring, level_up, recom_hp, mp_ring })
}

/// Re:CoM's HP gauge at KH1's place, its ring and bar turned 180° (owner, 2026-11-01; asset step
/// `kh1_recom_assets::recom_hp`). Re:CoM's control `+0x78` keeps its five layers; layers 1..3
/// (`gauge:01` 7 max HP, 8 drain, 9 HP) get the turned copies, layer 0 (the disc behind the face)
/// and 4 (the face) stay Re:CoM's, drawn upright about the gauge's origin.
pub mod recom_hp {
    /// The set, NUL-terminated.
    pub const SET: &[u8] = b"gauge:01\0";
    /// Re:CoM Steam's groups in `gauge:01` (the turned ones follow). Checked (files).
    pub const BASE_GROUPS: u32 = 47;
    /// The groups turned, in layer order (layers 1, 2, 3; `0x140208360`). Checked (decomp).
    pub const TURNED: [u32; 3] = [7, 8, 9];
    /// The first layer they replace.
    pub const FIRST_LAYER: u32 = 1;
    /// Where the ring's centre goes in KH1's 512 × 384 space: the centre of KH1's MP ring as our
    /// bake draws it (a circle fit to the MP back ring's outer edge, offline render of the owner's
    /// files: (443.9, 317.0), R 57.5). KH1's objects meet at (444, 316); its fans start at
    /// (443, 317) (`0x14026B790`). Checked (offline render).
    pub const CENTRE: (f32, f32) = (444.0, 317.0);
    /// Re:CoM adds 0.25 to every animation translation (`0x14024B4E0` → `0x1402B4A00`, an add;
    /// Checked (exe)), so its gauge's visual centre is its origin + 0.25; turned in the data, the
    /// ring turns about that point. 0.38.0–0.40.x put the origin 0.25 up-left of [`CENTRE`]; from
    /// 0.41.0 the origin is [`CENTRE`] itself (whole units, as Re:CoM's own layout places it), so
    /// the HP ring and KH1's MP ring, whose sprites sit at [`CENTRE`] and get the same 0.25, share
    /// one centre (owner, 2026-11-01: "the bars should start from the same place").
    pub const BIAS: f32 = 0.25;
    /// Re:CoM's frame for an HP value (`0x140208360`, `0x14020D6A0`, `0x14020D8E0`): trunc(hp /
    /// 560 × 470), in f32 as Re:CoM computes it. Checked (decomp).
    pub fn frame(hp: u32) -> i32 {
        ((hp as f32 / 560.0) * 470.0) as i64 as i32
    }

    /// The layout position for the HP control (`0x140248660` takes KH1-space x, y): KH1's centre
    /// at KH1's right margin, minus the bias, plus Re:CoM's own layer offset (its layers are added
    /// at −`+0xA0`, the left margin), so the ring lands where KH1's right-anchored cluster does.
    pub fn origin(gauge_xoff: f32) -> (f32, f32) {
        (CENTRE.0 + super::right_margin(gauge_xoff) + gauge_xoff, CENTRE.1)
    }
}

/// Re:CoM's room plate at the top left (`hud_room_plate_top_left`; the DLL's `room_plate`).
pub mod room_plate {
    /// Where Re:CoM puts the plate (card, name, star) at 16:9, in KH1's 512 × 384 space (x from −85.33
    /// at the left edge to 597.33 at the right): x 390.5 (its round left end) to 594 at the right edge,
    /// y 321 to 382, measured on the owner's 1440p and 720p screenshots. Its x follows the right edge
    /// (Assumption: the navi layout's layers are offset by the right margin, as Re:CoM's gauges are;
    /// seen at 16:9 only).
    const PLATE_LEFT_169: f32 = 390.5;
    const PLATE_TOP: f32 = 321.0;
    /// The 16:9 margin the measures were taken at (`(682.67 − 512) / 2`).
    const MARGIN_169: f32 = 85.333;
    /// The plate's round left end and its top this far in from the screen's top-left corner (KH1
    /// units), as far as Re:CoM's plate sits from the right edge.
    const PLATE_PAD: f32 = 3.0;
    /// The plate's move (dx, dy) for a screen margin `m` (KH1 units each side of 512): its left end
    /// from `PLATE_LEFT_169 − 85.33 + m` (right-anchored) to `−m + PLATE_PAD`, its top from
    /// `PLATE_TOP` to `PLATE_PAD`.
    pub fn shift(m: f32) -> (f32, f32) {
        (-m + PLATE_PAD - (PLATE_LEFT_169 - MARGIN_169 + m), PLATE_PAD - PLATE_TOP)
    }
}

/// The KH1 MP ring around Re:CoM's HP ring (`hud_kh1_mp_ring`): which of the cluster's held groups
/// it shows ([`frames`]' index → (control, layer)), and their priorities. Controls 0 / 1: MP back,
/// MP (priority 0); controls 2 / 3: charge, the max-MP ticks (priority 1); all under Re:CoM's HP
/// gauge (priority 2): Re:CoM's 2D sprites go to the foreground display list's bucket min(priority,
/// 25), drawn bucket 0 first, in insertion order within a bucket (`0x14024ABB0`, `0x140253F40`,
/// `0x140253AE0`; Checked (decomp)). KH1 draws MP back, MP, (the HP arc over them), charge, ticks
/// (`0x14026CF90`).
pub mod mp_ring {
    /// `frames` index → (control, layer); None for the HP / face parts. One layer a control
    /// (0.41.0): Re:CoM draws a control's later layers relative to its first layer's first live
    /// sprite (`0x14024BB10`: the first layer, `+0x47` = 0 from `0x140247850`, stores that
    /// sprite's matrix × the control's at node `+0x40`; later layers, `+0x47` = −1, multiply by
    /// `+0x40` instead of the control's own). The ring's sprites carry the ring's centre as their
    /// translation, so on one control the MP and the ticks went off screen by (444, 317) (owner,
    /// 2026-11-01: "the blue of the MP bar doesnt show"). Checked (decomp); in game: Assumption.
    pub fn layer_of(k: usize) -> Option<(usize, u32)> {
        match k {
            2 => Some((0, 0)),
            3 => Some((1, 0)),
            5 => Some((2, 0)),
            6 => Some((3, 0)),
            _ => None,
        }
    }
    /// Controls the MP ring uses (MP back, MP, charge, ticks).
    pub const CONTROLS: usize = 4;
    /// Control `ci`'s priority: MP back and MP under the charge and ticks.
    pub fn priority(ci: usize) -> u8 {
        if ci <= 1 {
            PRIORITY_MP
        } else {
            PRIORITY_CHARGE
        }
    }
    /// The MP back / MP control's priority (and the MP fades').
    pub const PRIORITY_MP: u8 = 0;
    /// The charge / ticks control's priority (and the charge fades').
    pub const PRIORITY_CHARGE: u8 = 1;
    /// KH1's size class whose centre is [`super::recom_hp::CENTRE`] (offset (0, 0)): the ring
    /// stays put around Re:CoM's fixed-size ring.
    pub const CLASS: usize = 2;

    /// The ring groups built as Re:CoM's HP ring (asset step `kh1_hud::ring`; owner, 2026-11-01:
    /// "maybe built the same way"): KH1's quarter pictures whole, cut by scissor parts and turned,
    /// so Re:CoM samples them once at screen resolution. Offsets from the manifest's `mp_ring`
    /// first group; frames and fades as the cluster's groups.
    pub mod group {
        /// MP back (+ the closing outline under the ring's start).
        pub const BACK: u32 = 0;
        /// MP.
        pub const MP: u32 = 1;
        /// Charge.
        pub const CHARGE: u32 = 2;
        /// MP gain fades (9), loss fades (9), charge gain fades (270), loss fades (270).
        pub const MP_GAIN: u32 = 3;
        /// MP loss fades.
        pub const MP_LOSS: u32 = 12;
        /// Charge gain fades.
        pub const CHARGE_GAIN: u32 = 21;
        /// Charge loss fades.
        pub const CHARGE_LOSS: u32 = 291;
        /// The ticks, centred on the ring's seams (0.41.0).
        pub const TICKS: u32 = 561;
        /// Groups in all.
        pub const COUNT: u32 = 562;
    }

    /// The ring group (from `first`) that shows what the cluster's group `g` shows; the ticks
    /// (and anything else) stay the cluster's.
    pub fn ring_group(first: u32, g: u32) -> u32 {
        use super::group as c;
        let off = match g {
            c::MP_BACK => group::BACK,
            c::MP => group::MP,
            c::CHARGE => group::CHARGE,
            c::TICKS => group::TICKS,
            _ if (c::MP_GAIN..c::MP_GAIN + 9).contains(&g) => group::MP_GAIN + (g - c::MP_GAIN),
            _ if (c::MP_LOSS..c::MP_LOSS + 9).contains(&g) => group::MP_LOSS + (g - c::MP_LOSS),
            _ if (c::CHARGE_GAIN..c::CHARGE_GAIN + 270).contains(&g) => group::CHARGE_GAIN + (g - c::CHARGE_GAIN),
            _ if (c::CHARGE_LOSS..c::CHARGE_LOSS + 270).contains(&g) => group::CHARGE_LOSS + (g - c::CHARGE_LOSS),
            _ => return g,
        };
        first + off
    }
}

// -------------------------------------------------------------------------------------------------
// H2: Sora's gauge cluster. KH1's rules (Checked (decomp) unless tagged), on Re:CoM's raw HP
// (decision 39) and KH1's MP (`crate::mp`).

/// Re:CoM's HP in KH1's gauge units: Re:CoM's cap 560 (`0x230`, the HP setters `0x14020D6A0` /
/// `0x14020D8E0` clamp to it; Checked (decomp)) is KH1's drawn cap 120 (`0x78`), and every HP
/// scales by the same 120 / 560, rounded up so a living Sora never shows 0. Decision 39's raw HP
/// left the gauge full above 120 HP (Re:CoM's Sora has hundreds): owner, 2026-11-01, "losing
/// health doesnt affect the health bar". The bar still grows with max HP as KH1's does.
pub fn hp_units(recom_hp: i32) -> i32 {
    let h = recom_hp.clamp(0, RECOM_HP_CAP);
    (h * kh1::HP_CAP + RECOM_HP_CAP - 1) / RECOM_HP_CAP
}

/// Re:CoM's HP cap (the gauge's).
pub const RECOM_HP_CAP: i32 = 560;

/// KH1 numbers (as the asset step's `kh1_hud::kh1`).
pub mod kh1 {
    /// HP drawn at most (`0x78`), HP filling the ring, HP a straight segment.
    pub const HP_CAP: i32 = 120;
    /// HP filling the ring (270° at 5° a point).
    pub const HP_RING: i32 = 54;
    /// HP a straight segment.
    pub const HP_PER_SEG: i32 = 6;
    /// MP drawn at most (`0x14`); MP filling the ring.
    pub const MP_CAP: i32 = 20;
    /// MP filling the ring.
    pub const MP_RING: i32 = 9;
    /// Charge units drawn at most (`0x258`); units filling the ring.
    pub const CHARGE_CAP: i32 = 600;
    /// Charge units filling the ring.
    pub const CHARGE_RING: i32 = 270;
    /// Straight segment pitch.
    pub const PITCH: f32 = 26.0;
    /// Size-class offsets `0x140527300` (class 0, 1, 2).
    pub const CLASS_OFFSET: [(f32, f32); 3] = [(24.0, 24.0), (0.0, 24.0), (0.0, 0.0)];
    /// Hurt face frames by hit strength (`0x1405270FC`, `0x140527134`, `0x14052726C`).
    pub const HURT_FRAMES: [i32; 3] = [40, 50, 60];
    /// KH1's right widescreen margin at 16:9: ceil(((16 / 9) / (4 / 3) · 0.5 − 0.5) · 512)
    /// (`0x140121210`, rounded up `0x1401703C0`) = ceil(85.33).
    pub const MARGIN_16_9: f32 = 86.0;
}

/// What the cluster shows (KH1's values: Re:CoM's HP as it is, KH1's MP).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct View {
    /// HP and max HP (Re:CoM's gauge `+0xA4` / `+0xA8`).
    pub hp: i32,
    /// Max HP.
    pub max_hp: i32,
    /// KH1 MP (MP, max, charge), None without KH1 MP (Combat = Re:CoM): no MP pieces, no label.
    pub mp: Option<(i32, i32, u16)>,
}

/// The held groups' frames (FRAME, HP_BACK, MP_BACK, MP, HP, CHARGE, TICKS), in that order.
pub fn frames(v: &View) -> [(u32, i32); 7] {
    let (mp, max, c) = v.mp.map(|(a, b, c)| (a.clamp(0, kh1::MP_CAP), b.clamp(0, kh1::MP_CAP), (c as i32).min(kh1::CHARGE_CAP))).unwrap_or((0, 0, 0));
    [
        (group::FRAME, v.mp.is_some() as i32),
        (group::HP_BACK, v.max_hp.clamp(0, kh1::HP_CAP)),
        (group::MP_BACK, max),
        (group::MP, mp),
        (group::HP, v.hp.clamp(0, kh1::HP_CAP)),
        (group::CHARGE, c),
        (group::TICKS, max),
    ]
}

/// KH1's size class (`0x1402690D0`): 0 when max HP < 19 and max MP < 4; 1 when max HP < 37 and
/// max MP < 7; else 2.
pub fn size_class(max_hp: i32, max_mp: i32) -> usize {
    if max_hp < 0x25 && max_mp < 7 {
        if max_hp < 0x13 && max_mp < 4 {
            0
        } else {
            1
        }
    } else {
        2
    }
}

/// KH1's danger test (`0x14026E180`): HP > 0 and HP / max < 0.2 when max > 44, else HP < 9.
pub fn danger(hp: i32, max: i32) -> bool {
    if hp <= 0 {
        return false;
    }
    if max > 0x2C {
        (hp as f32) / (max as f32) < 0.2
    } else {
        hp < 9
    }
}

/// KH1's hit strength (`0x14029BBD0` → `0x140268D60`): new / old HP < 0.5 → 2, < 0.8 → 1, < 1 → 0;
/// None when HP didn't fall.
pub fn hurt_strength(old: i32, new: i32) -> Option<usize> {
    if old <= 0 || new >= old {
        return None;
    }
    let r = new as f32 / old as f32;
    Some(if r < 0.5 {
        2
    } else if r < 0.8 {
        1
    } else {
        0
    })
}

/// The hurt face group: expression 1 for 40 / 50 / 60 frames, then 2 (danger) or 0.
pub fn hurt_group(strength: usize, danger: bool) -> u32 {
    group::HURT + 2 * strength.min(2) as u32 + danger as u32
}

/// The face's held expression: 2 in danger, else 0 (`0x14026D570`).
pub fn face_frame(danger: bool) -> i32 {
    if danger {
        2
    } else {
        0
    }
}

/// KH1's MP fade cells (`0x14026C6E0`: the ring part only, 10 frames): gained cells (black
/// fading off them) or lost ones (fading to black).
pub fn mp_fade(old: i32, new: i32) -> Vec<u32> {
    let (a, b) = (old.clamp(0, kh1::MP_RING), new.clamp(0, kh1::MP_RING));
    if b > a {
        (a..b).map(|j| group::MP_GAIN + j as u32).collect()
    } else {
        (b..a).map(|j| group::MP_LOSS + j as u32).collect()
    }
}

/// The charge change (`0x14026A770`: alpha fade of the changed ring part, 10 frames): the main
/// layer's frame and the fade pieces ([30 j, 30 j + t] each; the first one also covers the cell's
/// part below the change, drawn the same under it). Past the ring no fade (KH1's straight part).
pub fn charge_plan(old: i32, new: i32) -> (i32, Vec<u32>) {
    let (old, new) = (old.clamp(0, kh1::CHARGE_CAP), new.clamp(0, kh1::CHARGE_CAP));
    let ring = kh1::CHARGE_RING;
    if old == new || old.min(new) >= ring {
        return (new, Vec::new());
    }
    let (lo, hi, gain) = if new > old { (old, new.min(ring), true) } else { (new, old.min(ring), false) };
    let mut pieces = Vec::new();
    let mut j = lo / 30;
    while 30 * j < hi {
        let t = (hi - 30 * j).min(30);
        if t > 0 && 30 * j + t > lo {
            pieces.push(if gain { group::CHARGE_GAIN } else { group::CHARGE_LOSS } + (30 * j + t - 1) as u32);
        }
        j += 1;
    }
    // A gain shows the old value under its fading-in pieces (they hold); crossing the ring's end
    // shows the new value at once (the straight part never fades).
    let main = if gain && new <= ring { old } else { new };
    (main, pieces)
}

/// The danger flash pieces of the max-HP arc and segments (`0x14026BCF0`, `0x14026BEE0` in state
/// 2): (group, x offset) each.
pub fn red_pieces(max_hp: i32) -> Vec<(u32, f32)> {
    let m = max_hp.clamp(0, kh1::HP_CAP);
    let a = (m * 5).min(270);
    let mut v = Vec::new();
    for q in 0..3 {
        let k = ((a - 90 * q).min(90)) / 5;
        if k > 0 {
            v.push((group::RED_ARC + 18 * q as u32 + k as u32 - 1, 0.0));
        }
    }
    if m > kh1::HP_RING {
        let (n, rem) = ((m - kh1::HP_RING) / kh1::HP_PER_SEG, (m - kh1::HP_RING) % kh1::HP_PER_SEG);
        for i in 0..n {
            v.push((group::RED_SEG, -kh1::PITCH * i as f32));
        }
        if rem > 0 {
            v.push((group::RED_SEG_PART + rem as u32 - 1, -kh1::PITCH * n as f32));
        }
    }
    v
}

/// KH1's widescreen margin for a right-anchored piece, from the player gauge's x offset
/// (`+0xA0`: 85.0 at 16:9, 0 at 4:3; Re:CoM has no other ratio, Checked (decomp `0x140208360`)).
pub fn right_margin(gauge_xoff: f32) -> f32 {
    if gauge_xoff > 0.0 {
        kh1::MARGIN_16_9
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_glyphs_follow_the_pad_type() {
        use shortcut::{CIRCLE, CROSS, SQUARE, TRIANGLE};
        let sg = ShortcutGroups { first: 100, icons: true, pads: Some(200) };
        // Type 2: the PlayStation set (the panel's own groups).
        assert_eq!([TRIANGLE, SQUARE, CROSS, CIRCLE].map(|k| sg.glyph(k, 2)), [101, 102, 103, 104]);
        // Type 1 (and any other, as Re:CoM): the Xbox Y / X / A / B.
        assert_eq!([TRIANGLE, SQUARE, CROSS, CIRCLE].map(|k| sg.glyph(k, 1)), [200, 201, 202, 203]);
        assert_eq!(sg.glyph(TRIANGLE, 0), 200);
        assert_eq!(sg.glyph(CIRCLE, 7), 203);
        // Type 3: the generic pad's.
        assert_eq!([TRIANGLE, SQUARE, CROSS, CIRCLE].map(|k| sg.glyph(k, 3)), [204, 205, 206, 207]);
        // An older manifest (no pad glyphs): the PlayStation set whatever the pad.
        let old = ShortcutGroups { pads: None, ..sg };
        assert_eq!(old.glyph(TRIANGLE, 1), 101);
    }

    #[test]
    fn hp_units_scale_recom_cap_to_kh1_cap() {
        assert_eq!((hp_units(0), hp_units(1), hp_units(560), hp_units(600), hp_units(300), hp_units(280)), (0, 1, 120, 120, 65, 60));
    }

    #[test]
    fn manifest_round_trip_and_refusals() {
        let ok = r#"{"format": "kh1-recom-assets kh1-hud 3", "set": "kh1hud:00", "groups": 644, "base_groups": 632, "programs": [{"name": "vibe_s", "start": 1, "frames": 10}, {"name": "vibe_ss", "start": 1, "frames": 22}, {"name": "vibe_sss", "start": 1, "frames": 38}, {"name": "fade_out", "start": 0, "frames": 10}, {"name": "fade_in", "start": 0, "frames": 10}, {"name": "slide_s_h", "start": 1, "frames": 20}, {"name": "slide_s_v", "start": 1, "frames": 20}], "units": [[632, 7], [639, 5]], "runs": [[4, 1, 10000000, 0], [4, 2, 2, 1]], "languages": ["FR", "UK"], "atlas": [1024, 512, 2048, 1024], "sources": {}}"#;
        let m = parse_manifest(ok).unwrap();
        assert_eq!(m.groups, 644);
        assert_eq!(m.languages, vec!["FR", "UK"]);
        assert_eq!(m.motion.programs[2], Program { start: 1, frames: 38 });
        assert_eq!(m.motion.units_at(group::HP, 2), vec![0, 1]);
        assert_eq!(m.motion.units_at(group::HP, 3), vec![0]);
        assert_eq!(m.motion.group(1, program::FADE_IN), Some(643));
        assert_eq!(m.motion.group(1, program::SLIDE_H), None);
        assert_eq!(m.motion.group(0, program::SLIDE_V), Some(638));
        assert_eq!((m.cursor, m.auto_ring), (None, None), "an older step: no ring");
        let ring = parse_manifest(&ok.replace("\"sources\": {}", "\"sources\": {}, \"cursor\": 642, \"auto_ring\": 643")).unwrap();
        assert_eq!((ring.cursor, ring.auto_ring), (Some(642), Some(643)));
        assert!(parse_manifest(&ok.replace("\"sources\": {}", "\"sources\": {}, \"auto_ring\": 644")).is_err(), "past the last group");
        assert!(parse_manifest(&ok.replace("\"base_groups\": 632", "\"base_groups\": 631")).is_err());
        assert!(parse_manifest(&ok.replace("[639, 5]", "[639, 6]")).is_err());
        assert!(parse_manifest(&ok.replace("vibe_ss", "vibe_xx")).is_err());
        assert!(parse_manifest(&ok.replace("hud 3", "hud 2")).is_err(), "format 2 has no context tab");
        assert_eq!(m.menu, None);
        let with = ok.replace("\"languages\"", "\"menu\": {\"first\": 630, \"tab_icon\": true, \"words\": {\"us\": [[1, 643], [6, 642]], \"fr\": [[1, 641]]}}, \"languages\"");
        assert!(parse_manifest(&with).is_err(), "menu groups before the fixed ones would run past");
        let with = with.replace("\"groups\": 644", "\"groups\": 700");
        let mm = parse_manifest(&with).unwrap().menu.unwrap();
        assert_eq!((mm.word("fr", 1), mm.word("fr", 6), mm.word("gr", 1), mm.word("us", 2)), (Some(641), Some(642), Some(643), None));
        assert!(mm.tab_icon && menu::TAB == 23 && menu::WORDS == 25);
        assert_eq!(mm.items_list, None, "M8: optional");
        let it = with.replace("\"tab_icon\": true", "\"tab_icon\": true, \"items_list\": 690");
        assert_eq!(parse_manifest(&it).unwrap().menu.unwrap().items_list, Some(690));
        assert_eq!(mm.summon_list, None, "M8b: optional");
        let su = with.replace("\"tab_icon\": true", "\"tab_icon\": true, \"summon_list\": 694");
        assert_eq!(parse_manifest(&su).unwrap().menu.unwrap().summon_list, Some(694));
        // 0.179.1: the DP sleights' mark, optional.
        assert_eq!(mm.dp_icon, None);
        let dp = with.replace("\"tab_icon\": true", "\"tab_icon\": true, \"dp_icon\": 699");
        assert_eq!(parse_manifest(&dp).unwrap().menu.unwrap().dp_icon, Some(699));
        assert!(parse_manifest(&with.replace("\"tab_icon\": true", "\"tab_icon\": true, \"dp_icon\": 700")).is_err(), "past the last group");
        assert!(parse_manifest(&it.replace("690", "695")).is_err(), "six groups past the set");
        // Squeeze to fit (0.107.0): word widths optional, tenths of a unit; "fr" falls back to
        // "us" as the word does.
        assert!(mm.widths.is_empty() && mm.width("us", 1).is_none(), "an older asset step: nothing squeezed");
        let wi = with.replace("\"fr\": [[1, 641]]}}", "\"fr\": [[1, 641]]}, \"word_widths\": {\"us\": [[1, 520], [6, 1104]], \"fr\": [[1, 600]]}}");
        let mw = parse_manifest(&wi).unwrap().menu.unwrap();
        assert_eq!((mw.width("us", 6), mw.width("fr", 1), mw.width("fr", 6), mw.width("us", 2)), (Some(110.4), Some(60.0), Some(110.4), None));
        assert!(parse_manifest(&wi.replace("[6, 1104]", "[6]")).is_err());
        assert!(parse_manifest(&ok.replace("kh1hud:00", "Cockpit:00")).is_err());
        // Stage 7.12: the shortcut panel, optional (older manifests have none).
        assert_eq!(m.shortcut, None);
        let with = ok.replace("\"languages\"", "\"shortcut\": {\"first\": 639, \"icons\": true}, \"languages\"");
        assert_eq!(parse_manifest(&with).unwrap().shortcut, Some(ShortcutGroups { first: 639, icons: true, pads: None }));
        assert!(parse_manifest(&with.replace("\"first\": 639", "\"first\": 640")).is_err(), "past the set");
        assert_eq!(parse_manifest(&with.replace("true}", "false}")).unwrap().shortcut.map(|s| s.icons), Some(false));
        // The other pad types' glyphs, optional: 8 groups inside the set.
        let pads = with.replace("\"icons\": true}", "\"icons\": true, \"pads\": 600}");
        assert_eq!(parse_manifest(&pads).unwrap().shortcut.and_then(|s| s.pads), Some(600));
        assert!(parse_manifest(&pads.replace("\"pads\": 600", "\"pads\": 637")).is_err(), "pad glyphs past the set");
        assert_eq!(m.recom_hp, None);
        let with = ok.replace("\"languages\"", "\"recom_hp\": {\"set\": \"gauge:01\", \"first\": 47, \"of\": [7, 8, 9]}, \"languages\"");
        assert_eq!(parse_manifest(&with).unwrap().recom_hp, Some(47));
        assert!(parse_manifest(&with.replace("[7, 8, 9]", "[7, 8]")).is_err());
        assert!(parse_manifest(&with.replace("\"first\": 47", "\"first\": 48")).is_err());
        assert_eq!(m.mp_ring, None);
        let with = ok.replace("\"groups\": 644", "\"groups\": 1206").replace("\"languages\"", "\"mp_ring\": {\"first\": 644, \"count\": 562}, \"languages\"");
        assert_eq!(parse_manifest(&with).unwrap().mp_ring, Some(644));
        assert!(parse_manifest(&with.replace("\"count\": 562", "\"count\": 561")).is_err());
        assert!(parse_manifest(&with.replace("\"groups\": 1206", "\"groups\": 1205")).is_err(), "past the set");
    }

    /// Owner, 2026-11-04 ("Squeeze to fit"): only a word wider than its box is squeezed, to end at
    /// the box's edge. The widths are KH1's menu font's (`US_font_data_tbl.bin`, the asset step's
    /// layout: Checked (files)).
    #[test]
    fn long_words_squeeze_to_their_box() {
        use fit::*;
        assert_eq!(scale(None, LIST_BOX), 1.0, "width unknown");
        assert_eq!(scale(Some(73.9), LIST_BOX), 1.0, "KH1's Tinker Bell fits");
        assert_eq!(scale(Some(LIST_BOX), LIST_BOX), 1.0);
        let s = scale(Some(110.4), LIST_BOX);
        assert!((s * 110.4 - LIST_BOX).abs() < 1e-3 && s < 1.0, "Homing Blizzara ends at the box's edge");
        assert!((scale(Some(110.4), SHORTCUT_BOX) * 110.4 - SHORTCUT_BOX).abs() < 1e-3);
        assert_eq!(scale(Some(61.4), SHORTCUT_BOX), 1.0, "KH1's Thundaga fits the shortcut box");
        assert_eq!(scale(Some(10.0), 0.0), 1.0);
        // The lists squeeze only our sleight names; on the cursor row they end before the hat.
        let hb = crate::sleights::word(0x24);
        assert_eq!((list_box(hb, false), list_box(hb, true)), (Some(LIST_BOX), Some(CURSOR_BOX)));
        assert_eq!(list_box(8, true), None, "KH1's Fira");
        assert_eq!(list_box(0x8029, true), None, "Cloud");
        assert_eq!(list_box(crate::sleights::word(crate::abilities::sleight::STRIKE_RAID), true), None, "a special: KH1's own word");
        assert_eq!(scale(Some(87.1), LIST_BOX), 1.0, "Firaga Burst fits a row");
        assert!(scale(Some(87.1), CURSOR_BOX) < 1.0, "but not the cursor row before the hat");
    }

    #[test]
    fn room_plate_goes_top_left() {
        // 16:9: the round end 3 units in from the left edge (−85.33), the top 3 units down.
        let (dx, dy) = room_plate::shift(85.333);
        assert!((390.5 + dx - (-85.333 + 3.0)).abs() < 1e-3 && dy == -318.0, "{dx} {dy}");
        // 4:3 (no margin): from 305.17 to x 3.
        let (dx, _) = room_plate::shift(0.0);
        assert!((390.5 - 85.333 + dx - 3.0).abs() < 1e-3);
    }

    #[test]
    fn mp_ring_groups_stand_for_the_clusters() {
        use mp_ring::{group as r, ring_group};
        let f = 5785;
        assert_eq!(ring_group(f, group::MP_BACK), f + r::BACK);
        assert_eq!(ring_group(f, group::MP), f + r::MP);
        assert_eq!(ring_group(f, group::CHARGE), f + r::CHARGE);
        assert_eq!(ring_group(f, group::TICKS), f + r::TICKS);
        assert_eq!(ring_group(f, group::FACE), group::FACE, "the rest stay the cluster's");
        for j in 0..9 {
            assert_eq!(ring_group(f, group::MP_GAIN + j), f + r::MP_GAIN + j);
            assert_eq!(ring_group(f, group::MP_LOSS + j), f + r::MP_LOSS + j);
        }
        // Every fade the charge plan asks for maps one to one.
        for (old, new) in [(0, 45), (45, 0), (100, 270), (270, 100), (29, 31)] {
            for g in charge_plan(old, new).1 {
                let k = ring_group(f, g) - f;
                assert!((r::CHARGE_GAIN..r::TICKS).contains(&k));
                assert_eq!(k - if g >= group::CHARGE_LOSS { r::CHARGE_LOSS } else { r::CHARGE_GAIN }, g - if g >= group::CHARGE_LOSS { group::CHARGE_LOSS } else { group::CHARGE_GAIN });
            }
        }
        assert_eq!((r::CHARGE_LOSS + 270, r::TICKS + 1), (r::TICKS, r::COUNT));
    }

    #[test]
    fn recom_hp_place_and_frames() {
        // 16:9: Re:CoM's x offset 85, KH1's margin 86: the origin lands KH1's centre + 86 on screen
        // once Re:CoM's own -85 layer offset is added.
        let (x, y) = recom_hp::origin(85.0);
        assert_eq!((x - 85.0, y), (444.0 + 86.0, 317.0));
        assert_eq!(recom_hp::origin(0.0), (444.0, 317.0));
        assert_eq!((recom_hp::frame(300), recom_hp::frame(560), recom_hp::frame(0)), (251, 470, 0));
        assert_eq!([2, 3, 5, 6].map(mp_ring::layer_of), [Some((0, 0)), Some((1, 0)), Some((2, 0)), Some((3, 0))]);
        assert_eq!((0..mp_ring::CONTROLS).map(mp_ring::priority).collect::<Vec<_>>(), vec![0, 0, 1, 1]);
        assert!([0, 1, 4].iter().all(|&k| mp_ring::layer_of(k).is_none()));
    }

    #[test]
    fn target_gauge_rules() {
        assert_eq!(target::body(120), 60);
        assert_eq!(target::body(1000), 150);
        assert_eq!(target::bars(0), [0, 0, 0, 0, 0]);
        assert_eq!(target::bars(1), [1, 0, 0, 0, 0]);
        assert_eq!(target::bars(299), [150, 0, 0, 0, 0]);
        assert_eq!(target::bars(420), [150, 60, 0, 0, 0]);
        assert_eq!(target::bars(1500), [150, 150, 150, 150, 150]);
        assert_eq!(target::marker(&target::bars(420)), Some(1));
        assert_eq!(target::marker(&target::bars(200)), None);
    }

    #[test]
    fn group_numbering_matches_the_asset_step() {
        // 8 held + 6 hurt + 2 × 9 MP fades + 2 × 270 charge fades + 54 + 1 + 5 flashes.
        assert_eq!(group::HURT + 6, group::MP_GAIN);
        assert_eq!(group::MP_GAIN + 9, group::MP_LOSS);
        assert_eq!(group::MP_LOSS + 9, group::CHARGE_GAIN);
        assert_eq!(group::CHARGE_GAIN + 270, group::CHARGE_LOSS);
        assert_eq!(group::CHARGE_LOSS + 270, group::RED_ARC);
        assert_eq!(group::RED_ARC + 54, group::RED_SEG);
        assert_eq!(group::RED_SEG_PART + 5, group::COUNT);
    }

    #[test]
    fn frames_follow_kh1_caps() {
        let v = View { hp: 200, max_hp: 300, mp: Some((25, 30, 700)) };
        assert_eq!(frames(&v), [(0, 1), (1, 120), (2, 20), (3, 20), (4, 120), (5, 600), (6, 20)]);
        let v = View { hp: 30, max_hp: 40, mp: None };
        assert_eq!(frames(&v), [(0, 0), (1, 40), (2, 0), (3, 0), (4, 30), (5, 0), (6, 0)]);
    }

    #[test]
    fn classes_danger_and_hurt() {
        assert_eq!(size_class(18, 3), 0);
        assert_eq!(size_class(19, 3), 1);
        assert_eq!(size_class(36, 6), 1);
        assert_eq!(size_class(37, 3), 2);
        assert_eq!(size_class(20, 7), 2);
        assert!(danger(8, 44) && !danger(9, 44));
        assert!(danger(9, 50) && !danger(10, 50));
        assert!(!danger(0, 50));
        assert_eq!(hurt_strength(100, 85), Some(0));
        assert_eq!(hurt_strength(100, 79), Some(1));
        assert_eq!(hurt_strength(100, 49), Some(2));
        assert_eq!(hurt_strength(100, 100), None);
        assert_eq!(hurt_group(1, true), group::HURT + 3);
    }

    #[test]
    fn mp_and_charge_fades() {
        assert_eq!(mp_fade(3, 5), vec![group::MP_GAIN + 3, group::MP_GAIN + 4]);
        assert_eq!(mp_fade(12, 8), vec![group::MP_LOSS + 8]);
        assert!(mp_fade(10, 12).is_empty());
        // Gain 25 → 65: cells [0, 30] (piece t = 30) and [30, 60] and [60, 65]; main stays 25.
        let (m, p) = charge_plan(25, 65);
        assert_eq!(m, 25);
        assert_eq!(p, vec![group::CHARGE_GAIN + 29, group::CHARGE_GAIN + 59, group::CHARGE_GAIN + 64]);
        // Loss 65 → 40: [30, 60] and [60, 65] fade out; main shows 40.
        let (m, p) = charge_plan(65, 40);
        assert_eq!(m, 40);
        assert_eq!(p, vec![group::CHARGE_LOSS + 59, group::CHARGE_LOSS + 64]);
        // Crossing 270: no wait for the straight part.
        assert_eq!(charge_plan(260, 300).0, 300);
        assert_eq!(charge_plan(300, 330), (330, vec![]));
    }

    #[test]
    fn red_flash_covers_the_max_arc() {
        assert_eq!(red_pieces(10), vec![(group::RED_ARC + 9, 0.0)]);
        let v = red_pieces(70);
        assert_eq!(&v[..3], &[(group::RED_ARC + 17, 0.0), (group::RED_ARC + 35, 0.0), (group::RED_ARC + 53, 0.0)]);
        assert_eq!(&v[3..], &[(group::RED_SEG, 0.0), (group::RED_SEG, -26.0), (group::RED_SEG_PART + 3, -52.0)]);
        assert!(red_pieces(54).len() == 3);
    }
}
