//! M6 stage 2, no deck for Sora in battle at the KH1 setting (decision 29; `crate::deck`,
//! `docs/recom-m6-battles.md` § Stage 2).
//!
//! - `build_revolver` (`0x1401F43D0`, once per battle start, from the player's stateInitial):
//!   decides whether this battle has a deck. Without one it first builds the revolver with
//!   Re:CoM's own constructor from Re:CoM's end-of-deck entry (0 cards + Reload), then lets
//!   Re:CoM's build run (it sees a built revolver and only runs its tail). It also switches the
//!   seven HUD-mask call retargets: ours without a deck, Re:CoM's bytes otherwise.
//! - `hit_drop` (`0x14017E190`): no Friend-card drop roll at KH1 in Sora's story.
//! - `cockpit_mask` / `card_mask`: what the retargeted mask reads call (through a stub that passes
//!   the caller's rbx): Re:CoM's getter, plus the hide bit for this battle's empty revolver only.
//! - Three log-only readers (`0x1401EA1A0`, `0x1401EA1E0`, `0x140167CF0`): once per battle, who
//!   read the empty revolver.
//!
//! Per frame: the mask reads Re:CoM already makes (one OR, two loads), and the tutorial's
//! hand-empty check while its watcher exists (one compare).

use super::movement::{self as mv, func, orig, rd, wr};
use super::{base, hook, log, va};
use crate::battle::{self as b, Bytes};
use crate::deck::{self as d, Build, Reader};
use crate::hookplan;
use crate::sites::{data, M6_CARD_CALLS, M6_COCKPIT_CALLS};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;

pub static ORIG_BUILD: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_DROP: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_HAND: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_SCROLL: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_SEARCH: AtomicUsize = AtomicUsize::new(0);

/// This battle's empty revolver (0: this battle has Re:CoM's deck, or none yet).
static EMPTY: AtomicUsize = AtomicUsize::new(0);
/// Battles without a deck so far (the readers' once-per-battle key).
static BATTLE: AtomicU32 = AtomicU32::new(0);
static READER_LAST: [AtomicU32; 3] = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];
static READER_LINES: AtomicU32 = AtomicU32::new(0);
static BATTLE_LINES: AtomicU32 = AtomicU32::new(0);
static DROP_LINES: AtomicU32 = AtomicU32::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// The retargets are ours right now.
static HUD_OURS: AtomicBool = AtomicBool::new(false);

/// One retargeted mask read: Re:CoM's `call 0x1401B08C0` and ours (to the stub).
struct Retarget {
    name: &'static str,
    rva: u32,
    recom: &'static [u8],
    ours: [u8; 5],
}

static RETARGETS: OnceLock<Vec<Retarget>> = OnceLock::new();

// Re:CoM functions (Checked (exe): `sites::M6_DECK_CALLEES`, `sites::CALLEES`).
const CTOR: u32 = 0x0016_4D40;
const NEW: u32 = 0x004A_CE54;
const FLAG_GET: u32 = 0x001B_0870;
const VAR_GET: u32 = 0x001B_08C0;
const MINIGAME: u32 = 0x0023_5360;

type PlFn = unsafe extern "win64" fn(usize);
type NewFn = unsafe extern "win64" fn(usize) -> usize;
type CtorFn = unsafe extern "win64" fn(usize, usize, usize, i32, u32) -> usize;
type GetFn = unsafe extern "win64" fn(u32, u32) -> u32;
type BoolFn = unsafe extern "win64" fn() -> u32;
type DropFn = unsafe extern "win64" fn(usize, i32);
type PlRetFn = unsafe extern "win64" fn(usize) -> u64;
type ScrollFn = unsafe extern "win64" fn(usize, u32) -> u64;
type SearchFn = unsafe extern "win64" fn(usize, i32, i32, i32) -> u64;

fn capped(counter: &AtomicU32, cap: u32, line: impl FnOnce() -> String) {
    if counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| (n < cap).then_some(n + 1)).is_ok() {
        log(&format!("m6: {}", line()));
    }
}

unsafe fn minigame_room() -> bool {
    func::<BoolFn>(MINIGAME)() & 0xFF != 0
}

unsafe fn flag(index: u32) -> bool {
    func::<GetFn>(FLAG_GET)(d::FLAG_TABLE, index) & 0xFF != 0
}

unsafe fn tutorial_flags() -> String {
    let f: Vec<(u32, bool)> = d::TUTORIAL_FLAGS.iter().map(|&i| (i, flag(i))).collect();
    d::flags_str(&f)
}

unsafe fn main_player_revolver() -> usize {
    let pl: usize = rd(va(base(), data::MAIN_PLAYER));
    if pl == 0 {
        0
    } else {
        rd(pl + d::player::REVOLVER)
    }
}

/// Re:CoM's battle revolver build (`0x1401F43D0(player)`), once per battle start.
pub unsafe extern "win64" fn build_revolver(pl: usize) {
    let o: PlFn = orig(&ORIG_BUILD);
    let slot: usize = rd(pl + d::player::REVOLVER);
    if slot != 0 {
        // Re:CoM keeps a built revolver (its callers only call with an empty slot).
        if slot != EMPTY.load(Ordering::Relaxed) {
            EMPTY.store(0, Ordering::Release);
        }
        return o(pl);
    }
    let battle = d::Battle {
        kh1: mv::kh1_on(),
        riku_story: super::stats::riku_story(),
        main_sora: mv::is_main_sora(pl),
        minigame: rd::<i32>(pl + d::player::MINIGAME) != 0,
    };
    let why = match d::decide(&battle) {
        Build::Empty => match switch_hud(true) {
            Some(n) => {
                let rev = build_empty(pl);
                if rev != 0 {
                    EMPTY.store(rev, Ordering::Release);
                    let id = BATTLE.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
                    capped(&BATTLE_LINES, d::BATTLE_LOG_LINES, || {
                        let hud = format!("ours ({n} written)");
                        let flags = tutorial_flags();
                        format!(
                            "{} (battle {id})",
                            d::empty_line(&d::EmptyLine {
                                rev,
                                cards: rd(rev + d::revolver::CARDS),
                                slots: rd(rev + d::revolver::SLOTS),
                                rev_flags: rd(rev + d::revolver::FLAGS),
                                tutorial_deck: flag(d::FLAG_TUTORIAL_DECK),
                                hud: &hud,
                                flags: &flags,
                            })
                        )
                    });
                    return o(pl);
                }
                "Re:CoM's allocator returned 0"
            }
            None => "the HUD mask reads could not be switched (see above)",
        },
        Build::Recom(k) => k.as_str(),
    };
    // Re:CoM's deck in this battle: Re:CoM's bytes at the mask reads.
    EMPTY.store(0, Ordering::Release);
    let hud = match switch_hud(false) {
        Some(n) => format!("Re:CoM's bytes ({n} written)"),
        None => "UNCHANGED (see above)".into(),
    };
    capped(&BATTLE_LINES, d::BATTLE_LOG_LINES, || format!("battle with Re:CoM's deck ({why}); deck HUD mask reads: {hud}"));
    o(pl)
}

/// The empty revolver, built and stored as `0x1401F43D0` does (`mem = new(0x280)`, ctor with
/// side 1), from a list holding only Re:CoM's end-of-deck entry. 0 if the allocation failed.
unsafe fn build_empty(pl: usize) -> usize {
    let mem = func::<NewFn>(NEW)(d::revolver::SIZE);
    if mem == 0 {
        return 0;
    }
    let list = d::EMPTY_LIST;
    let rev = func::<CtorFn>(CTOR)(mem, pl, list.as_ptr() as usize, i32::from(d::SIDE_PLAYER), d::SELECTED_NONE);
    wr(pl + d::player::REVOLVER, rev);
    rev
}

/// Makes the seven mask-read retargets ours (or Re:CoM's), all or none. The number written.
unsafe fn switch_hud(ours: bool) -> Option<usize> {
    let rt = RETARGETS.get()?;
    let bs = base();
    let have = |r: &Retarget| std::slice::from_raw_parts((bs + r.rva as usize) as *const u8, 5);
    let now: Vec<Bytes> = rt.iter().map(|r| b::bytes_state_of(r.recom, &r.ours, have(r))).collect();
    let pairs: Vec<(&[u8], &[u8])> = rt.iter().map(|r| (r.recom, &r.ours[..])).collect();
    let Some(writes) = b::plan_pairs(&pairs, &now, ours) else {
        for (r, s) in rt.iter().zip(&now) {
            if *s == Bytes::Other {
                log(&format!("m6: {} holds bytes that are neither Re:CoM's nor ours; no read switched", r.name));
            }
        }
        return None;
    };
    let mut done: Vec<b::Write> = Vec::new();
    for (i, from, to) in writes {
        let r = &rt[i];
        if !hook::patch(bs + r.rva as usize, from, to, r.name) {
            for &(j, from2, to2) in done.iter().rev() {
                hook::patch(bs + rt[j].rva as usize, to2, from2, rt[j].name);
            }
            log("m6: a mask-read write failed; the reads were left as they were");
            return None;
        }
        done.push((i, from, to));
    }
    HUD_OURS.store(ours, Ordering::Release);
    Some(done.len())
}

/// The cockpit's mask read (`0x1401689F0`, rbx = the revolver).
pub unsafe extern "win64" fn cockpit_mask(table: u32, index: u32, rev: usize) -> u32 {
    func::<GetFn>(VAR_GET)(table, index) | d::cockpit_bits(rev, EMPTY.load(Ordering::Relaxed))
}

/// A card's mask read (`0x140150340`, `0x1401514B0`, `0x1401545A0`, rbx = the card).
pub unsafe extern "win64" fn card_mask(table: u32, index: u32, card: usize) -> u32 {
    let v = func::<GetFn>(VAR_GET)(table, index);
    let e = EMPTY.load(Ordering::Relaxed);
    if e == 0 || card == 0 {
        return v;
    }
    v | d::card_bits(rd(card + d::CARD_SIDE), main_player_revolver(), e)
}

/// An enemy's card drop (`0x14017E190(enemy, chance)`).
pub unsafe extern "win64" fn hit_drop(enemy: usize, chance: i32) {
    if d::drops_off(mv::kh1_on(), super::stats::riku_story(), minigame_room()) {
        capped(&DROP_LINES, d::DROP_LOG_LINES, || format!("Friend-card drop roll skipped (chance {chance}): nothing drops, as when Re:CoM's roll fails"));
        return;
    }
    orig::<DropFn>(&ORIG_DROP)(enemy, chance)
}

unsafe fn reader(r: Reader, rev: usize) {
    let e = EMPTY.load(Ordering::Relaxed);
    if e == 0 || rev != e {
        return;
    }
    let battle = BATTLE.load(Ordering::Relaxed);
    let last = &READER_LAST[r.index()];
    if d::log_reader(last.load(Ordering::Relaxed), battle, READER_LINES.load(Ordering::Relaxed)) {
        last.store(battle, Ordering::Relaxed);
        READER_LINES.fetch_add(1, Ordering::Relaxed);
        log(&format!("m6: deck reader (battle {battle}): {}", r.describe()));
    }
}

/// `0x1401EA1A0(player)`: hand empty? (CAtelTutorial). Pass-through.
pub unsafe extern "win64" fn hand_check(pl: usize) -> u64 {
    let v = orig::<PlRetFn>(&ORIG_HAND)(pl);
    reader(Reader::HandCheck, rd(pl + d::player::REVOLVER));
    v
}

/// `0x1401EA1E0(player, page)`: an event script scrolls the revolver. Pass-through.
pub unsafe extern "win64" fn script_scroll(pl: usize, page: u32) -> u64 {
    let v = orig::<ScrollFn>(&ORIG_SCROLL)(pl, page);
    reader(Reader::ScriptScroll, rd(pl + d::player::REVOLVER));
    v
}

/// `0x140167CF0(revolver, value, …)`: a search over a revolver's card values. Pass-through.
pub unsafe extern "win64" fn value_search(rev: usize, a: i32, b2: i32, c: i32) -> u64 {
    let v = orig::<SearchFn>(&ORIG_SEARCH)(rev, a, b2, c);
    reader(Reader::ValueSearch, rev);
    v
}

/// The trace's stage-2 token (F9 recording only).
pub unsafe fn trace_token() -> String {
    if !INSTALLED.load(Ordering::Relaxed) {
        return "m6deck off".into();
    }
    d::trace_token(main_player_revolver(), EMPTY.load(Ordering::Relaxed), &tutorial_flags())
}

/// The stub block and the retargets, then the two wrappers (all or none), then the readers (log
/// only, apart). Needs the movement handlers (the setting).
pub fn install(bs: usize) -> bool {
    // SAFETY: executable memory of our own near the exe; the call sites' bytes were checked at
    // start and are checked again before every write.
    unsafe {
        let mut code = Vec::with_capacity(34);
        code.extend_from_slice(&hookplan::rbx_arg_stub(cockpit_mask as *const () as u64));
        code.extend_from_slice(&hookplan::rbx_arg_stub(card_mask as *const () as u64));
        let near = bs + M6_CARD_CALLS[0].rva as usize;
        let Some(block) = hook::place_near(near, &code) else {
            log("m6: no memory near the exe for the HUD stubs; no deck change");
            return false;
        };
        let stubs = [(M6_COCKPIT_CALLS, block), (M6_CARD_CALLS, block + 17)];
        let mut v = Vec::new();
        for (calls, stub) in stubs {
            for s in calls {
                let Some(ours) = hookplan::call_rel32((bs + s.rva as usize) as u64, stub as u64) else {
                    log(&format!("m6: {} cannot reach the stub {stub:#x}; no deck change", s.name));
                    return false;
                };
                v.push(Retarget { name: s.name, rva: s.rva, recom: s.expect, ours });
            }
        }
        if RETARGETS.set(v).is_err() {
            return false;
        }
        log(&format!("m6: HUD stubs at {block:#x} (cockpit) and {:#x} (cards)", block + 17));
        let site = |name: &str| crate::sites::M6_DECK_HOOKS.iter().find(|h| h.name == name).expect("site");
        let hooks = [
            hook::Hook { site: site("battle revolver build (0x1401F43D0)"), ours: build_revolver as *const () as usize, original: &ORIG_BUILD },
            hook::Hook { site: site("enemy hit card drop (0x14017E190)"), ours: hit_drop as *const () as usize, original: &ORIG_DROP },
        ];
        if !hook::install_all(bs, &hooks) {
            return false;
        }
        INSTALLED.store(true, Ordering::Release);
        let r: &'static [crate::sites::Site] = crate::sites::M6_DECK_READERS;
        let readers = [
            hook::Hook { site: &r[0], ours: hand_check as *const () as usize, original: &ORIG_HAND },
            hook::Hook { site: &r[1], ours: script_scroll as *const () as usize, original: &ORIG_SCROLL },
            hook::Hook { site: &r[2], ours: value_search as *const () as usize, original: &ORIG_SEARCH },
        ];
        if hook::install_all(bs, &readers) {
            log("m6: deck readers logged once per battle (0x1401EA1A0, 0x1401EA1E0, 0x140167CF0)");
        } else {
            log("m6: deck readers not hooked (log only; the deck work stands)");
        }
        true
    }
}
