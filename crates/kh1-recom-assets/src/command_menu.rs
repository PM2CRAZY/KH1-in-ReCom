//! KH1's command menu (M7 stage 7.3, decision 34): its words and its row plates.
//!
//! - **Words** (owner: "menu words = KH1's own text from the player's KH1 message files"): KH1
//!   names its commands through the command table's `+4` message id (`btltbl.bin` `+0xA58`, 16-byte
//!   records), looked up in the string table `0x142D22F98` (`0x14028F9B0`). On PC that table is
//!   refilled from the `Word.bin` PC asset of `btltbl.bin` (`0x14028F9D0`: entry `k + 1` = the
//!   `k`-th string of `Word.bin`, walked by `0x14028F820`). Checked (decomp + the owner's KH1
//!   files: `btltbl.bin` carries `US_ UK_ FR_ GR_ IT_ SP_ JP_Word.bin`). This step decodes the
//!   strings of the commands the menu shows, in every language, into `kh1-command-menu.json`.
//! - **Plates** (owner: "Re:CoM's own menu pieces … the highlighted row uses another colour of
//!   Re:CoM's command plate"): two animation groups added to `Cockpit:00` after the MP bar's:
//!   Re:CoM's text plate (`Cockpit:00` group 0x19's cap and strip, the plate Re:CoM's
//!   `CStockFolder` puts its `C2DFnt` labels on, `0x1401658F0`) without its card icon, and the
//!   same plate with its strip in another colour of Re:CoM's command plates (the magic plate's
//!   icon colour, group 0x1F) on the atlas's neutral texel. Notes:
//!   `docs/recom-m7-command-menu.md`.
//!
//! KH1's text encoding (Checked (files): decoded from the owner's `US_ UK_ FR_ GR_ IT_ SP_Word.bin`
//! against the known English names): 0x01 space, 0x21..0x2A '0'..'9', 0x2B..0x44 'A'..'Z',
//! 0x45..0x5E 'a'..'z', 0x63 '+', 0x66 '/', 0x68 '.', 0x6E '-', 0x71 '\''; the accented letters
//! the menu's words use: 0xDD 'Ö' (GR "Öffnen"), 0xE4 'á', 0xE9 'é' (FR "Gravité"), 0xED 'í'.
//! M8 stage 8a.1's item names add 0xCF 'Ä' (GR "Äther"), 0xD2 'É' (SP "Éter"), 0xF2 'ó' (SP
//! "Poción"); 0xE8 'è' (FR "colère", not a menu word). These four are read from the table's layout
//! (four forms a vowel: à á â ä 0xE3.., è é ê ë 0xE8.., ì í î ï 0xEC.., ò ó ô ö 0xF1.., ù ú û ü
//! 0xF5..; capitals the same from 0xCC; it fits all seven known letters, ü 0xF8 included):
//! Assumption, checked against the words they spell.
//! 0x6F is the glyph of KH1's blank command (0x42, message 305): read as '-' (GR / SP write that
//! entry with 0x6E '-'; Assumption: a dash). Japanese uses another encoding: not decoded.

use crate::mp_bar::{scaled, uv_scale, Seqd, NEUTRAL_UV};
use crate::{bad, Result};

/// The manifest the DLL reads, next to `kh1-mp-bar.json`.
pub const MANIFEST: &str = "kh1-command-menu.json";
/// Its format line.
pub const FORMAT: &str = "kh1-recom-assets command-menu 1";

/// The languages decoded, as `(manifest key, PC asset prefix)`.
pub const LANGUAGES: &[(&str, &str)] = &[("us", "US_"), ("uk", "UK_"), ("fr", "FR_"), ("gr", "GR_"), ("it", "IT_"), ("sp", "SP_")];

/// The KH1 command names the menu shows (message ids, the command records' `+4`). Checked (files:
/// the owner's `btltbl.bin` command table): 1 Attack, 2 Magic, 3 Items, 4 Talk, 5 Examine (the
/// context tab's word for Re:CoM's warp point), 6 Summon (H4), 7..27 the spells
/// (`level + (spell + 2) × 3`, named by their own id), 28 Lift (command 0x1C), 293 Save (0x36,
/// the save point's), 294 Open (0x37), 305 the blank command (0x42), 313 Jump On (0x45), 343
/// Counter (0x63, the Counterattack); M8 stage 8a.1: 31..34, 36..38 the battle items' commands
/// (`item + 0x1E`: Potion, Hi-Potion, Ether, Elixir, Mega-Potion, Mega-Ether, Megalixir; 35 is an
/// empty row); M9 stage 9c.1: 319 Sonic Blade (0x4B), 321 Rave (0x4D), 322 Blast (0x4E); M8b:
/// 286..291 the summons (commands 0x2F..0x34: Dumbo, Bambi, Genie, Tinker Bell, Mushu, Simba); M12
/// step 7: 338 Strike Raid (0x5E, "Strike"), 339 / 340 its follow-ups (0x5F "Raid", 0x60 "Judgment");
/// 331 Ars Arcanum (0x57, "Arcanum"), 332 / 333 its follow-ups (0x58 "Bash", 0x59 "Finis"); 334
/// Ragnarok (0x5A), 335 its follow-up (0x5B "Impact"); M8c stage 2: 172 Tent, 173 Camping Set
/// (item rows 142 / 143 `+0`: names only, for the camp Items page and the shop; no command).
pub const NAMES: &[u16] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 31, 32, 33, 34, 36, 37, 38, 172, 173, 286, 287, 288, 289, 290, 291, 293, 294, 305, 313, 319, 321, 322, 331, 332, 333, 334, 335, 338, 339, 340, 343];

/// M8b: Cloud's word (the DLL's `crate::summons::CLOUD_NAME`, the same value). KH1's `Word.bin` has
/// no "Cloud" (Checked (files)) and Re:CoM's card-name text is not in our word system: "Cloud" in
/// every language (a name; that Re:CoM's languages all write it so: Assumption).
pub const CLOUD_NAME: u16 = 0x8029;
/// Cloud's text.
pub const CLOUD_TEXT: &str = "Cloud";

/// Our groups after the MP bar's (`mp_bar::group::COUNT`): the plate, the highlighted plate, the
/// pointing arrow (0.111.0).
pub mod group {
    /// Re:CoM's text plate (cap + strip), no icon.
    pub const PLATE: usize = 0;
    /// The same plate with its strip in the highlight colour.
    pub const PLATE_HIGHLIGHT: usize = 1;
    /// 0.111.0: the command menu's pointing arrow ([`super::arrow`]).
    pub const ARROW: usize = 2;
    /// How many.
    pub const COUNT: usize = 3;
}

/// 0.111.0 (owner, 2026-11-04: "the pointing arrow that follows your selection on the command menu";
/// "Re:CoM's own arrow"): the command menu's pointing arrow as a `Cockpit:00` group (the DLL's
/// `crate::command_menu::arrow`, the same numbers; a DLL test compares them).
///
/// - **Art**: Re:CoM's own white triangle (`Cockpit:00`'s sprite at UV (16, 49)–(32, 64), × the
///   copy's atlas scale; one of the three pointers of its group 98, which spin around a target;
///   Checked (files: the owner's `FO0002.SPR` / `.RTM`, sprite 84 of the base file)), found by its
///   UV in each copy, drawn 16 × 10 about the origin and turned a quarter clockwise (the file's z
///   rotation −π/2: Re:CoM negates it on load, `kh1_hud::Step`'s note; Re:CoM's flags 0x20 clear
///   for a turn, `ring::FLAGS_TURNED`), so its point faces left. Not found (another layout): an
///   empty group, so the numbering stays the same in every copy.
/// - **Motion** (KH2's, measured on the owner's recording): an X translation (integer units, the
///   format's) over [`arrow::INTRO`] then [`arrow::SWAY`] looped from [`arrow::LOOP_START`] (the
///   group's loop start `+8`, play-once word 0: Re:CoM's own looping groups, e.g. group 98: Checked
///   (decomp `0x140248BB0` / `0x140248F40`)). Segments with exclusive ends (`kh1_hud::FLAGS_MOVE`).
pub mod arrow {
    /// The triangle's UV in the base file's atlas.
    pub const UV: [i32; 4] = [16, 49, 32, 64];
    /// Its part about the origin (before the turn: 16 wide, 10 high).
    pub const RECT: [i32; 4] = [-8, -5, 8, 5];
    /// The turn (the file's value).
    pub const TURN: f32 = -std::f32::consts::FRAC_PI_2;
    /// The DLL's `crate::command_menu::arrow::INTRO`.
    pub const INTRO: [(i32, i32); 6] = [(0, 6), (3, 7), (13, -4), (30, 3), (51, -2), (LOOP_START, 2)];
    /// The DLL's `crate::command_menu::arrow::LOOP_START`.
    pub const LOOP_START: i32 = 67;
    /// The DLL's `crate::command_menu::arrow::SWAY`.
    pub const SWAY: [(i32, i32); 7] = [(0, 2), (6, 1), (10, 0), (16, -1), (22, 0), (26, 1), (32, 2)];
}

/// The animation group word of the arrow (`+6`, not read by Re:CoM; Re:CoM's group 98's).
const ARROW_WORD: i16 = 995;

/// Adds the arrow group (its index); false when the triangle wasn't found (an empty group).
fn add_arrow(seq: &mut Seqd) -> bool {
    use crate::mp_bar::{agroup, anim, part, sgroup};
    let first = seq.agroups.len();
    let want = scaled(arrow::UV, uv_scale(seq));
    let Some(sp) = (0..seq.sprites.len()).find(|&i| sprite_uv(seq, i) == want) else {
        let mut g = agroup(0, 0, true, ARROW_WORD);
        g[8..12].copy_from_slice(&0i32.to_le_bytes());
        seq.agroups.push(g);
        debug_assert_eq!(seq.agroups.len(), first + 1);
        return false;
    };
    let p = seq.parts.len();
    seq.parts.push(part(arrow::RECT, sp));
    let sg = seq.sgroups.len();
    seq.sgroups.push(sgroup(p, 1));
    let keys: Vec<(i32, i32)> = arrow::INTRO.iter().copied().chain(arrow::SWAY.iter().skip(1).map(|&(f, x)| (arrow::LOOP_START + f, x))).collect();
    let flags = crate::kh1_hud::FLAGS_MOVE & !0x20;
    let one = 0x8080_8080;
    let a0 = seq.anims.len();
    for w in keys.windows(2) {
        let ((f0, x0), (f1, x1)) = (w[0], w[1]);
        let mut a = anim(flags, sg, (f0, f1), 0, (1.0, 1.0), (one, one));
        for (at, v) in [(0x10, x0), (0x14, x1), (0x18, 0), (0x1C, 0)] {
            a[at..at + 4].copy_from_slice(&v.to_le_bytes());
        }
        a[0x40..0x44].copy_from_slice(&arrow::TURN.to_le_bytes());
        a[0x44..0x48].copy_from_slice(&arrow::TURN.to_le_bytes());
        seq.anims.push(a);
    }
    let mut g = agroup(a0, keys.len() - 1, false, ARROW_WORD);
    g[8..12].copy_from_slice(&arrow::LOOP_START.to_le_bytes());
    seq.agroups.push(g);
    true
}

/// Re:CoM's text plate, idle (looping, frames 0..120): 4 animations, the 2nd (cap, sprite UV
/// (229, 110)–(255, 135)) and 3rd (strip, UV (255, 110)–(255, 129)) are the plate. Checked (files:
/// the owner's `FO0002.SPR`).
pub const SOURCE_PLATE: usize = 0x19;
/// The magic plate (idle): its 4th animation is the hat icon, vertex colour 0x80FFA200.
pub const SOURCE_COLOUR: usize = 0x1F;
const CAP_UV: [i32; 4] = [229, 110, 255, 135];
const STRIP_UV: [i32; 4] = [255, 110, 255, 129];

/// What [`add_plates`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plates {
    /// The plate's records were where Re:CoM's base file has them (else two empty groups were
    /// added, so the group numbers stay the same in every copy).
    pub found: bool,
    /// The highlight's vertex colour (ABGR).
    pub colour: u32,
    /// 0.111.0: Re:CoM's arrow triangle was found (else the arrow's group is empty).
    pub arrow: bool,
}

fn i16_at(r: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([r[at], r[at + 1]])
}

fn i32_at(r: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([r[at], r[at + 1], r[at + 2], r[at + 3]])
}

fn u32_at(r: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([r[at], r[at + 1], r[at + 2], r[at + 3]])
}

/// The animations of group `g` (index range).
fn group_anims(seq: &Seqd, g: usize) -> Option<std::ops::Range<usize>> {
    let r = seq.agroups.get(g)?;
    let (f, n) = (i16_at(r, 0), i16_at(r, 2));
    (f >= 0 && n >= 0 && (f as usize + n as usize) <= seq.anims.len()).then(|| f as usize..f as usize + n as usize)
}

/// The first sprite drawn by animation `a` (its sprite group's first part).
fn anim_sprite(seq: &Seqd, a: usize) -> Option<usize> {
    let sg = i32_at(seq.anims.get(a)?, 4);
    let g = seq.sgroups.get(usize::try_from(sg).ok()?)?;
    let p = i16_at(g, 0);
    let part = seq.parts.get(usize::try_from(p).ok()?)?;
    usize::try_from(i32_at(part, 0x10)).ok().filter(|&s| s < seq.sprites.len())
}

fn sprite_uv(seq: &Seqd, s: usize) -> [i32; 4] {
    let r = &seq.sprites[s];
    [i32_at(r, 4), i32_at(r, 8), i32_at(r, 12), i32_at(r, 16)]
}

/// Re:CoM's plate animations (cap, strip) in group `g`, when it is Re:CoM's text plate.
fn plate_at(seq: &Seqd, g: usize) -> Option<(usize, usize)> {
    let r = group_anims(seq, g)?;
    if r.len() != 4 {
        return None;
    }
    let (cap, strip) = (r.start + 1, r.start + 2);
    let k = uv_scale(seq);
    (sprite_uv(seq, anim_sprite(seq, cap)?) == scaled(CAP_UV, k) && sprite_uv(seq, anim_sprite(seq, strip)?) == scaled(STRIP_UV, k)).then_some((cap, strip))
}

/// Re:CoM's text plate in this set: at the base file's group 0x19, else the first group with the
/// same records (the language copies of `Cockpit.lad` have 6 more groups of their own, so their
/// numbering is shifted; Checked (files), 2026-10-31). (group, cap, strip).
fn plate_anims(seq: &Seqd) -> Option<(usize, usize, usize)> {
    if let Some((c, s)) = plate_at(seq, SOURCE_PLATE) {
        return Some((SOURCE_PLATE, c, s));
    }
    (0..seq.agroups.len()).find_map(|g| plate_at(seq, g).map(|(c, s)| (g, c, s)))
}

/// The magic plate's icon colour (its 4th animation's sprite, top-left vertex): group 0x1F, at the
/// same shift as the text plate.
pub fn highlight_colour(seq: &Seqd) -> Option<u32> {
    let shift = plate_anims(seq).map_or(0isize, |(g, _, _)| g as isize - SOURCE_PLATE as isize);
    let r = group_anims(seq, usize::try_from(SOURCE_COLOUR as isize + shift).ok()?)?;
    if r.len() != 4 {
        return None;
    }
    let s = anim_sprite(seq, r.end - 1)?;
    Some(u32_at(&seq.sprites[s], 0x1C))
}

/// Adds the two plate groups to `Cockpit:00`; returns the first one's index and what was found.
/// `colour` overrides the highlight colour (else the magic plate's, see [`highlight_colour`]).
pub fn add_plates(seq: &mut Seqd, colour: Option<u32>) -> Result<(usize, Plates)> {
    let r = add_plate_groups(seq, colour)?;
    // 0.111.0: the pointing arrow after them.
    let arrow = add_arrow(seq);
    Ok((r.0, Plates { arrow, ..r.1 }))
}

fn add_plate_groups(seq: &mut Seqd, colour: Option<u32>) -> Result<(usize, Plates)> {
    let first = seq.agroups.len();
    let Some((plate, cap, strip)) = plate_anims(seq) else {
        // Two empty groups keep every language copy's numbering the same.
        let mut g = seq.agroups[SOURCE_PLATE.min(seq.agroups.len() - 1)];
        g[0..4].copy_from_slice(&[0, 0, 0, 0]);
        seq.agroups.push(g);
        seq.agroups.push(g);
        return Ok((first, Plates { found: false, colour: 0, arrow: false }));
    };
    let colour = match colour.or_else(|| highlight_colour(seq)) {
        Some(c) => c,
        None => return bad("Cockpit:00: no magic plate colour (group 0x1F)"),
    };
    // The highlight strip: one sprite on the neutral texel, one part over the strip's two parts
    // (Re:CoM's (0, −13)…(96, 6) and (96, −13)…(152, 6), placed by the strip animation's X
    // translate 13).
    let strip_sg = i32_at(&seq.anims[strip], 4) as usize;
    let sg = seq.sgroups[strip_sg];
    let (p0, pn) = (i16_at(&sg, 0) as usize, i16_at(&sg, 2) as usize);
    let rects: Vec<[i32; 4]> = (p0..p0 + pn).map(|p| [i32_at(&seq.parts[p], 0), i32_at(&seq.parts[p], 4), i32_at(&seq.parts[p], 8), i32_at(&seq.parts[p], 12)]).collect();
    let bound = rects.iter().fold([i32::MAX, i32::MAX, i32::MIN, i32::MIN], |b, r| [b[0].min(r[0]), b[1].min(r[1]), b[2].max(r[2]), b[3].max(r[3])]);
    let s = seq.sprites.len();
    let mut spr = [0u8; 0x2C];
    for (i, v) in scaled(NEUTRAL_UV, uv_scale(seq)).iter().enumerate() {
        spr[4 + 4 * i..8 + 4 * i].copy_from_slice(&v.to_le_bytes());
    }
    for i in 0..4 {
        spr[0x1C + 4 * i..0x20 + 4 * i].copy_from_slice(&colour.to_le_bytes());
    }
    seq.sprites.push(spr);
    let p = seq.parts.len();
    let mut part = [0u8; 0x14];
    for (i, v) in bound.iter().chain(std::iter::once(&(s as i32))).enumerate() {
        part[4 * i..4 * i + 4].copy_from_slice(&v.to_le_bytes());
    }
    seq.parts.push(part);
    let g = seq.sgroups.len();
    let mut sgr = [0u8; 4];
    sgr[0..2].copy_from_slice(&(p as i16).to_le_bytes());
    sgr[2..4].copy_from_slice(&1i16.to_le_bytes());
    seq.sgroups.push(sgr);
    // The highlight animation: the strip's own record (flags, frames, X translate 13), drawing the
    // new sprite group.
    let mut over = seq.anims[strip];
    over[4..8].copy_from_slice(&(g as i32).to_le_bytes());
    let src = seq.agroups[plate];
    for anims in [vec![seq.anims[cap], seq.anims[strip]], vec![seq.anims[cap], seq.anims[strip], over]] {
        let a0 = seq.anims.len();
        let n = anims.len();
        seq.anims.extend(anims);
        let mut gr = src;
        gr[0..2].copy_from_slice(&(a0 as i16).to_le_bytes());
        gr[2..4].copy_from_slice(&(n as i16).to_le_bytes());
        seq.agroups.push(gr);
    }
    if seq.anims.len() > i16::MAX as usize || seq.parts.len() > i16::MAX as usize {
        return bad("seqd: too many records for 16-bit indices");
    }
    Ok((first, Plates { found: true, colour, arrow: false }))
}

/// What [`add_plates`] would do to the `Cockpit:00` of a `Cockpit.lad` or of a container holding
/// one (`FO0002.SPR` or a language copy); None when it holds neither.
pub fn probe_plates(data: &[u8]) -> Option<Plates> {
    use crate::mp_bar::{Container, Layd, LAYOUT};
    let lad = if data.starts_with(b"LAYD") { data.to_vec() } else { Container::parse(data).ok()?.find(LAYOUT)?.data.clone() };
    let mut s = Seqd::parse(Layd::parse(&lad).ok()?.seqs.first()?).ok()?;
    add_plates(&mut s, None).ok().map(|r| r.1)
}

// ---------------------------------------------------------------------------------------------
// KH1's words.

/// KH1's `Word.bin`: string `k` (0-based) is message `k + 1`. Walked as `0x14028F820` walks it:
/// 0 and 0x10 end a string; 0x15..0x1F take 2 bytes, 0x20..0xFF 1; control codes 3, 7, 0xC, 0xF
/// take 3, 8 takes 6, 0xB 5, 0xD, 0xE, 0x11..0x14 4, the rest 1. Checked (decomp).
pub fn word_strings(d: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let (mut start, mut i) = (0usize, 0usize);
    while i < d.len() {
        let c = d[i];
        if c == 0 || c == 0x10 {
            out.push(&d[start..i]);
            i += 1;
            start = i;
            continue;
        }
        i += match c {
            0x15..=0x1F => 2,
            0x20..=0xFF => 1,
            3 | 7 | 0xC | 0xF => 3,
            8 => 6,
            0xB => 5,
            0xD | 0xE | 0x11..=0x14 => 4,
            _ => 1,
        };
    }
    if start < d.len() {
        out.push(&d[start..d.len().min(i)]);
    }
    out
}

/// One KH1 character (European encoding), when known.
pub fn kh1_char(c: u8) -> Option<char> {
    Some(match c {
        0x01 => ' ',
        0x21..=0x2A => (b'0' + c - 0x21) as char,
        0x2B..=0x44 => (b'A' + c - 0x2B) as char,
        0x45..=0x5E => (b'a' + c - 0x45) as char,
        0x63 => '+',
        0x66 => '/',
        0x68 => '.',
        0x6E | 0x6F => '-',
        0x71 => '\'',
        0xCF => 'Ä',
        0xD2 => 'É',
        0xDD => 'Ö',
        // M9 stage 9c.1: German "Sonicsto?" / "Todessto?" (Sonic Blade 319, Blast 322). Checked
        // (files: the owner's GR_Word.bin; the only reading of both words).
        0xE2 => 'ß',
        0xE4 => 'á',
        0xE8 => 'è',
        0xE9 => 'é',
        0xED => 'í',
        0xF2 => 'ó',
        // M12 step 7: German "Ragnar?k" (Ragnarok 334). Checked (files: the owner's GR_Word.bin; its
        // only word with the byte, read as KH1's German spelling "Ragnarök").
        0xF4 => 'ö',
        _ => return None,
    })
}

/// A KH1 string as text; None when it holds a byte this decoder doesn't know.
pub fn decode(s: &[u8]) -> Option<String> {
    s.iter().map(|&c| kh1_char(c)).collect()
}

/// Our own menu words, not KH1's (message ids above `Word.bin`'s 510–523 strings), the same text
/// in every language: 901 "Stagger", the context tab's command for a held Re:CoM gimmick card
/// (owner's word, 2026-11-01; KH1 has no such word, Checked (files: the six `Word.bin`); English
/// everywhere: no translation was asked for). The DLL's `command_menu::OUR_WORDS` holds the same.
/// M12 step 5: 902 "Sleights", the command menu's 4th row in Sleights mode (owner's word, 2026-11-03).
/// 0.111.0: 903 "Party", the command menu's page 2 row 2 (owner's word).
pub const OUR_WORDS: &[(u16, &str)] = &[(901, "Stagger"), (902, "Sleights"), (903, "Party")];

/// 0.111.0: the Party list's names (the DLL's `crate::party::words`: our id `0x8100 +` the Re:CoM
/// sleight id, the owner's names, in every language).
pub const PARTY_WORDS: &[(u16, &str)] = &[
    (0x8145, "Donald Magic"),
    (0x8146, "Stardust Blitz"),
    (0x8148, "Goofy Tornado"),
    (0x8149, "Goofy Smash"),
    (0x814A, "Wild Crush"),
    (0x8156, "Lucky Bounty"),
    (0x814C, "Sandstorm"),
    (0x814E, "Surprise!"),
    (0x8150, "Spiral Wave"),
    (0x8152, "Hummingbird"),
    (0x8154, "Furious Volley"),
    (0x8163, "Mickey Miracle"),
    (0x8164, "Holy Burst"),
];

/// Riku's Sleights list's names (fork branch Re-Re-Reverse/Rebirth, 0.149.0: his menu in KH1's look;
/// the DLL's `crate::riku::words`: our id `0x8100 +` his Re:CoM sleight id, `SY0002.CTD`'s names).
pub const RIKU_WORDS: &[(u16, &str)] = &[
    (0x815F, "Dark Break"),
    (0x8160, "Dark Firaga"),
    (0x8161, "Dark Aura"),
    (0x8165, "Inverse Burst"),
    (0x8166, "Impulse"),
    (0x8167, "Maelstrom"),
    (0x8168, "Barrage"),
    (0x8169, "Dark Impulse"),
    (0x816A, "Dark Maelstrom"),
    (0x816B, "Dark Barrage"),
];

/// M12 step 5: the Sleights list's names (the DLL's `crate::sleights::words`: our id `0x8100 +` the
/// Re:CoM sleight id, the camp's Abilities page's English names, in every language as Cloud's; a
/// DLL test checks this list against its own). Re:CoM's own sleight names are its messages
/// (`0x80100 + id`), not in our word system.
pub const SLEIGHT_WORDS: &[(u16, &str)] = &[
    (0x811B, "Fire Raid"),
    (0x811C, "Blizzard Raid"),
    (0x811D, "Thunder Raid"),
    (0x811E, "Reflect Raid"),
    (0x811F, "Judgment"),
    (0x8120, "Firaga Burst"),
    (0x8121, "Raging Storm"),
    (0x8122, "Mega Flare"),
    (0x8123, "Freeze"),
    (0x8124, "Homing Blizzara"),
    (0x8125, "Aqua Splash"),
    (0x8126, "Magnet Spiral"),
    (0x8127, "Lethal Frame"),
    (0x8128, "Shock Impact"),
    (0x8129, "Tornado"),
    (0x812A, "Quake"),
    (0x812B, "Warpinator"),
    (0x812C, "Warp"),
    (0x812D, "Bind"),
    (0x812E, "Confuse"),
    (0x812F, "Terror"),
    (0x8130, "Synchro"),
    (0x8131, "Gifted Miracle"),
    (0x8132, "Teleport"),
    (0x8133, "Holy"),
    (0x813A, "Idyll Romp"),
    (0x8143, "Cross-slash+"),
];

/// The menu's words of one language: (message id, text) for each of [`NAMES`] found, then
/// [`OUR_WORDS`], and the ids that could not be decoded.
pub fn words(word_bin: &[u8]) -> (Vec<(u16, String)>, Vec<u16>) {
    let strings = word_strings(word_bin);
    let (mut ok, mut missing) = (Vec::new(), Vec::new());
    for &id in NAMES {
        match strings.get(id as usize - 1).and_then(|s| decode(s)).filter(|t| !t.is_empty()) {
            Some(t) => ok.push((id, t)),
            None => missing.push(id),
        }
    }
    for &(id, t) in OUR_WORDS {
        ok.push((id, t.to_string()));
    }
    ok.push((CLOUD_NAME, CLOUD_TEXT.to_string()));
    for &(id, t) in SLEIGHT_WORDS.iter().chain(PARTY_WORDS).chain(RIKU_WORDS) {
        ok.push((id, t.to_string()));
    }
    (ok, missing)
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7E => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// `kh1-command-menu.json`: the words per language, the plate groups, the sources.
pub fn manifest_json(langs: &[(&str, Vec<(u16, String)>)], plates_first_offset: usize, plates: Plates, sources: &[(&str, String)]) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "{{");
    let _ = writeln!(s, "  \"format\": \"{FORMAT}\",");
    let _ = writeln!(s, "  \"plates\": {{\"after_bar_groups\": {plates_first_offset}, \"count\": {}, \"found\": {}, \"highlight_vertex_colour\": \"{:#010x}\", \"arrow\": {}}},", group::COUNT, plates.found, plates.colour, plates.arrow);
    let _ = writeln!(s, "  \"words\": {{");
    for (i, (lang, w)) in langs.iter().enumerate() {
        let items: Vec<String> = w.iter().map(|(id, t)| format!("\"{id}\": {}", json_str(t))).collect();
        let _ = writeln!(s, "    \"{lang}\": {{{}}}{}", items.join(", "), if i + 1 < langs.len() { "," } else { "" });
    }
    let _ = writeln!(s, "  }},");
    let src: Vec<String> = sources.iter().map(|(k, v)| format!("{k:?}: {v:?}")).collect();
    let _ = writeln!(s, "  \"sources\": {{{}}}", src.join(", "));
    let _ = writeln!(s, "}}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// KH1's encoding of an ASCII word (test helper).
    fn enc(s: &str) -> Vec<u8> {
        s.bytes()
            .map(|b| match b {
                b' ' => 1,
                b'0'..=b'9' => b - b'0' + 0x21,
                b'A'..=b'Z' => b - b'A' + 0x2B,
                b'a'..=b'z' => b - b'a' + 0x45,
                b'+' => 0x63,
                b'-' => 0x6E,
                _ => panic!("{b}"),
            })
            .collect()
    }

    #[test]
    fn walks_and_decodes_words() {
        let mut d = Vec::new();
        for w in ["Attack", "Magic", "Items"] {
            d.extend(enc(w));
            d.push(0);
        }
        // A control code with operands (0xB: 5 bytes) inside a string, and a 2-byte glyph.
        d.extend([0x0B, 0x00, 0x00, 0x00, 0x00]);
        d.extend(enc("Talk"));
        d.push(0x10);
        let s = word_strings(&d);
        assert_eq!(s.len(), 4);
        assert_eq!(decode(s[0]).as_deref(), Some("Attack"));
        assert_eq!(decode(s[2]).as_deref(), Some("Items"));
        assert_eq!(decode(s[3]), None, "control codes are not text");
        assert_eq!(decode(&enc("Brasier +")).as_deref(), Some("Brasier +"));
        assert_eq!(decode(&[0x31, 0x7F]), None, "an unknown byte");
        assert_eq!(decode(&[0x2C, 0x56, 0x45, 0x44, 0x01, 0xE9]).as_deref(), Some("BraZ é"));
        assert_eq!(decode(&[0x22, 0x21]).as_deref(), Some("10"));
    }

    #[test]
    fn words_by_message_id() {
        // Messages 1..=400 as "W<n>" (digits); 7 is Fire.
        let mut d = Vec::new();
        for n in 1..=400u32 {
            let w = if n == 7 { "Fire".to_string() } else { format!("W{n}") };
            d.extend(enc(&w));
            d.push(0);
        }
        let (ok, missing) = words(&d);
        assert!(missing.is_empty(), "{missing:?}");
        assert_eq!(ok.len(), NAMES.len() + OUR_WORDS.len() + 1 + SLEIGHT_WORDS.len() + PARTY_WORDS.len() + RIKU_WORDS.len(), "and Cloud, the sleights, the Party sleights, Riku's");
        assert!(ok.contains(&(903, "Party".into())) && ok.contains(&(0x814E, "Surprise!".into())));
        assert_eq!(SLEIGHT_WORDS.len(), 27);
        assert!(ok.contains(&(902, "Sleights".into())));
        assert!(ok.contains(&(0x8143, "Cross-slash+".into())));
        assert!(SLEIGHT_WORDS.iter().all(|(id, _)| (0x811B..=0x8143).contains(id) && *id != CLOUD_NAME));
        assert!(ok.contains(&(CLOUD_NAME, "Cloud".into())));
        assert!(ok.contains(&(7, "Fire".into())));
        // Ours, after KH1's, above every KH1 message id.
        assert!(ok.contains(&(901, "Stagger".into())));
        assert!(OUR_WORDS.iter().all(|(id, _)| !NAMES.contains(id) && *id > 600));
        assert!(ok.contains(&(343, "W343".into())));
        // A short file misses the late ids.
        let (_, missing) = words(&d[..200]);
        assert!(missing.contains(&343));
    }

    /// With the owner's KH1 files (`KH1_WORD_DIR` = a folder with `btltbl.bin`'s PC assets
    /// `US_Word.bin` …): every menu word decodes in every European language. Skipped when unset.
    #[test]
    fn owner_words() {
        let Ok(dir) = std::env::var("KH1_WORD_DIR") else { return };
        for (lang, prefix) in LANGUAGES {
            let d = std::fs::read(std::path::Path::new(&dir).join(format!("{prefix}Word.bin"))).unwrap();
            let (w, missing) = words(&d);
            assert!(missing.is_empty(), "{lang}: {missing:?}");
            let get = |id: u16| w.iter().find(|x| x.0 == id).unwrap().1.clone();
            if *lang == "us" || *lang == "uk" {
                assert_eq!([get(1), get(2), get(3), get(4), get(7), get(9), get(27), get(294), get(343)], ["Attack", "Magic", "Items", "Talk", "Fire", "Firaga", "Aeroga", "Open", "Counter"]);
                assert_eq!([get(5), get(293)], ["Examine", "Save"]);
                assert_eq!([get(319), get(321), get(322)], ["Sonic", "Rave", "Blast"]);
                assert_eq!([get(338), get(339), get(340)], ["Strike", "Raid", "Judgment"]);
                assert_eq!([get(331), get(332)], ["Arcanum", "Bash"]);
                assert_eq!([get(334), get(335)], ["Ragnarok", "Impact"]);
                assert_eq!([get(286), get(287), get(288), get(289), get(290), get(291)], ["Dumbo", "Bambi", "Genie", "Tinker Bell", "Mushu", "Simba"]);
                assert_eq!(get(CLOUD_NAME), "Cloud");
            }
            if *lang == "fr" {
                assert_eq!([get(1), get(8), get(19)], ["Attaquer", "Brasier +", "Gravité"]);
                assert_eq!([get(5), get(293)], ["Examiner", "Sauvegarde"]);
                assert_eq!([get(319), get(321), get(322)], ["Aquilon", "Combo", "Bourrasque"]);
            }
            if *lang == "us" {
                assert_eq!([get(31), get(32), get(36), get(38)], ["Potion", "Hi-Potion", "Mega-Potion", "Megalixir"]);
            }
            if *lang == "sp" {
                assert_eq!([get(31), get(33), get(37)], ["Poción", "Éter", "Omniéter"]);
            }
            if *lang == "gr" {
                assert_eq!(get(33), "Äther");
                assert_eq!(get(294), "Öffnen");
                assert_eq!([get(5), get(293)], ["Untersuchen", "Speichern"]);
            }
        }
    }

    #[test]
    fn the_arrow_is_recoms_triangle_turned_and_swaying() {
        use crate::mp_bar::sprite;
        let mut s = Seqd { version: 0x160, sprites: vec![sprite([0; 4], [0; 4]), sprite(arrow::UV, [0x8080_8080; 4])], parts: Vec::new(), sgroups: Vec::new(), anims: Vec::new(), agroups: Vec::new() };
        assert!(add_arrow(&mut s));
        assert_eq!(s.agroups.len(), 1);
        let g = s.agroups[0];
        assert_eq!((i16_at(&g, 2), i16_at(&g, 4), i32_at(&g, 8)), (11, 0, arrow::LOOP_START));
        assert_eq!(i32_at(&s.parts[0], 0x10), 1, "Re:CoM's sprite, found by its UV");
        // Segment by segment: the keys, a quarter turn clockwise, translation interpolated.
        let keys: Vec<(i32, i32)> = arrow::INTRO.iter().copied().chain(arrow::SWAY.iter().skip(1).map(|&(f, x)| (arrow::LOOP_START + f, x))).collect();
        for (k, a) in s.anims.iter().enumerate() {
            assert_eq!((i32_at(a, 8), i32_at(a, 0xC), i32_at(a, 0x10), i32_at(a, 0x14)), (keys[k].0, keys[k + 1].0, keys[k].1, keys[k + 1].1));
            assert_eq!(f32::from_le_bytes(a[0x40..0x44].try_into().unwrap()), -std::f32::consts::FRAC_PI_2);
            assert_eq!(u32_at(a, 0) & 0x11020, 0, "translation and turn on");
            assert_ne!(u32_at(a, 0) & 0x8000, 0, "exclusive ends");
        }
        assert_eq!(keys.last().unwrap().0, arrow::LOOP_START + 32);
        // Not there: one empty group (the numbering stays).
        let mut e = Seqd { sprites: vec![sprite([0; 4], [0; 4])], ..s.clone() };
        e.agroups.clear();
        assert!(!add_arrow(&mut e));
        assert_eq!((e.agroups.len(), i16_at(&e.agroups[0], 2)), (1, 0));
    }

    #[test]
    fn manifest_escapes_non_ascii() {
        let j = manifest_json(&[("fr", vec![(19, "Gravité".into())])], 8, Plates { found: true, colour: 0x80FF_A200, arrow: true }, &[("a", "b".into())]);
        assert!(j.contains("\"19\": \"Gravit\\u00e9\""));
        assert!(j.contains("\"highlight_vertex_colour\": \"0x80ffa200\""));
        assert!(j.contains(FORMAT));
    }
}
