//! M6 stage 1, battles at the KH1 setting (decisions 29 / 30; `crate::battle`,
//! `docs/recom-m6-battles.md`).
//!
//! - `manager_ctor` (`0x1401859B0`, once per battle): decides whether this battle is a KH1 one
//!   (Combat = KH1 and Sora's story) and switches the three serialiser patches P2–P4 to match
//!   with checked writes, all or none. At the Re:CoM setting the bytes are Re:CoM's own.
//! - `card_play` (`0x140198870`, enemy vtable `+0x98`, once per granted attack): in a KH1 battle
//!   the enemy enters its chosen attack state as message 2 would, with no card. Re:CoM's own
//!   state code runs the attack. CBTL's lock gates keep Re:CoM's refusal.
//! - `set_play_card` (`0x140163830`, CBTL::setPlayCard): a pass-through that logs, once per
//!   class, who still plays a side-2 card in a KH1 battle (human bosses, Marluxia 3's sleights).
//!
//! Per frame: nothing. The trace (F9 only) reads the enemy list through `trace_token`.

use super::movement::{self as mv, func, orig, rd, wr};
use super::{base, log};
use crate::battle::{self as b, cbtl, enemy, Bytes, Gate, Handler, Path, Play, Why};
use crate::sites::M6_PATCHES;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

pub static ORIG_CARD_PLAY: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_MANAGER_CTOR: AtomicUsize = AtomicUsize::new(0);
pub static ORIG_SET_PLAY_CARD: AtomicUsize = AtomicUsize::new(0);

/// This battle is a KH1 one (decided when Re:CoM built its battle enemy manager).
static KH1_BATTLE: AtomicBool = AtomicBool::new(false);
/// P2–P4 are ours right now.
static PATCHED: AtomicBool = AtomicBool::new(false);
static ATTACK_LINES: AtomicU32 = AtomicU32::new(0);
static BATTLE_LINES: AtomicU32 = AtomicU32::new(0);
static REFUSAL_LINES: AtomicU32 = AtomicU32::new(0);
/// (vtable, path) pairs already logged: 0 cardless, 1 Re:CoM's card play (fallback), 2 a side-2
/// card set in a KH1 battle.
static CLASSES: Mutex<Vec<(usize, u8)>> = Mutex::new(Vec::new());

/// `0x1402AD300` (Checked (exe), `sites::M6_CALLEES`).
const TASK_BY_ID: u32 = 0x002A_D300;

type TaskFn = unsafe extern "win64" fn(u32) -> usize;
type PlayFn = unsafe extern "win64" fn(usize) -> u64;
type CtorFn = unsafe extern "win64" fn(usize, usize, usize, usize) -> usize;
type SetPlayFn = unsafe extern "win64" fn(usize, u32, i32, usize, usize, i32) -> u64;
type TypeFn = unsafe extern "win64" fn(usize) -> u32;

fn capped(counter: &AtomicU32, cap: u32, line: impl FnOnce() -> String) {
    if counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| (n < cap).then_some(n + 1)).is_ok() {
        log(&format!("m6: {}", line()));
    }
}

unsafe fn task(id: u32) -> usize {
    func::<TaskFn>(TASK_BY_ID)(id)
}

unsafe fn class_name(obj: usize) -> String {
    if obj == 0 {
        return "none".into();
    }
    let vt: usize = rd(obj);
    super::stats::rtti_name(base(), vt).map(|n| n.trim_start_matches(".?AV").trim_end_matches("@@").to_string()).unwrap_or_else(|| format!("vt {vt:#x}"))
}

/// Logs a class's path once.
unsafe fn class_once(obj: usize, path: u8, what: impl FnOnce(&str) -> String) {
    let vt: usize = if obj != 0 { rd(obj) } else { 0 };
    let new = CLASSES
        .lock()
        .map(|mut v| {
            let n = !v.contains(&(vt, path)) && v.len() < b::CLASS_LOG_MAX;
            if n {
                v.push((vt, path));
            }
            n
        })
        .unwrap_or(false);
    if new {
        let name = class_name(obj);
        log(&format!("m6: class {name}: {}", what(&name)));
    }
}

/// The enemies in Re:CoM's battle enemy manager (task type 0x1004), at most `max`.
unsafe fn enemies(max: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let m = task(b::TASK_ENEMY_MANAGER);
    if m == 0 {
        return out;
    }
    let mut node: usize = rd(m + b::MANAGER_LIST);
    let mut guard = 0;
    while node != 0 && guard < 256 && out.len() < max {
        let obj: usize = rd(node + b::NODE_OBJECT);
        if obj != 0 {
            let vt: usize = rd(obj);
            let ty: TypeFn = std::mem::transmute_copy(&rd::<usize>(vt + enemy::VT_TYPE));
            if ty(obj) == b::ENEMY_TYPE {
                out.push(obj);
            }
        }
        node = rd(node + b::NODE_NEXT);
        guard += 1;
    }
    out
}

unsafe fn state_of(e: usize) -> Option<usize> {
    crate::guard::state_index(e, rd(e + enemy::STATE))
}

unsafe fn attacking_now() -> usize {
    enemies(256).into_iter().filter(|&e| b::attacking(rd(e + enemy::FLAGS), state_of(e))).count()
}

unsafe fn handler_of(e: usize) -> Handler {
    let vt: usize = rd(e);
    let h: usize = rd(vt + enemy::VT_MESSAGE);
    let bs = base();
    if h == bs + b::HANDLER_GENERIC as usize {
        Handler::Generic
    } else if h == bs + b::HANDLER_MARLUXIA3 as usize {
        Handler::Marluxia3
    } else {
        Handler::Other
    }
}

/// Re:CoM's Heartless card play (`0x140198870`). In a KH1 battle: message 2's work, no card.
pub unsafe extern "win64" fn card_play(e: usize) -> u64 {
    let o: PlayFn = orig(&ORIG_CARD_PLAY);
    if !KH1_BATTLE.load(Ordering::Acquire) {
        return o(e);
    }
    let c = task(b::TASK_CBTL);
    let gate = (c != 0).then(|| Gate { lock: rd(c + cbtl::LOCK), gimmick: rd(c + cbtl::GIMMICK), duel: rd(c + cbtl::DUEL) });
    let handler = handler_of(e);
    let p = Play {
        kh1_battle: true,
        flags: rd(e + enemy::FLAGS),
        hp: rd(e + enemy::HP),
        chosen: rd(e + enemy::CHOSEN),
        handler,
        marluxia3_mode: if handler == Handler::Marluxia3 { rd(e + enemy::MARLUXIA3_MODE) } else { 0 },
        gate,
    };
    match b::decide(&p) {
        Path::Cardless { flags, state } => {
            wr(e + enemy::FLAGS, flags);
            wr(e + enemy::NEXT_STATE, b::state_address(e, state));
            class_once(e, 0, |_| "attacks without a card (P1)".into());
            capped(&ATTACK_LINES, b::ATTACK_LOG_LINES, || {
                format!(
                    "attack without a card: {} id={} state={state} attacking_now={} flags {:#x}->{flags:#x} cooldown={}",
                    class_name(e),
                    rd::<u32>(e + enemy::ID),
                    attacking_now(),
                    p.flags,
                    rd::<i32>(e + enemy::COOLDOWN)
                )
            });
            1
        }
        Path::Recom(why) => {
            match why {
                Why::Locked(g) => capped(&REFUSAL_LINES, b::REFUSAL_LOG_LINES, || {
                    format!("CBTL locked (+0x208={:#x}, gimmick={}, duel={}): Re:CoM refuses {} id={} as it refuses a card", g.lock, g.gimmick, g.duel, class_name(e), rd::<u32>(e + enemy::ID))
                }),
                why => class_once(e, 1, |_| format!("Re:CoM's card play ({why:?}): its card shows")),
            }
            o(e)
        }
    }
}

/// Re:CoM's battle enemy manager ctor (once per battle): the battle's setting and P2–P4.
pub unsafe extern "win64" fn manager_ctor(this: usize, a: usize, c: usize, d: usize) -> usize {
    let kh1 = mv::kh1_on() && !super::stats::riku_story();
    let switched = switch_patches(kh1);
    KH1_BATTLE.store(kh1, Ordering::Release);
    capped(&BATTLE_LINES, b::BATTLE_LOG_LINES, || {
        format!(
            "battle start: {}; no-card attacks (P1) {}; think gate (P2), grant all (P3), crowd pace (P4): {}",
            if kh1 { "KH1 (Combat = KH1, Sora's story)" } else if mv::kh1_on() { "Re:CoM (Riku's story)" } else { "Re:CoM (Combat = Re:CoM)" },
            if kh1 { "on" } else { "off" },
            match switched {
                Some(n) if PATCHED.load(Ordering::Relaxed) => format!("ours ({n} written)"),
                Some(n) => format!("Re:CoM's bytes ({n} written)"),
                None => "UNCHANGED: a site held unexpected bytes (see above); Re:CoM's turn-taking may stay".into(),
            }
        )
    });
    orig::<CtorFn>(&ORIG_MANAGER_CTOR)(this, a, c, d)
}

/// Makes P2–P4 ours (or Re:CoM's), all or none. The number of sites written, or None.
unsafe fn switch_patches(ours: bool) -> Option<usize> {
    let bs = base();
    let now: Vec<Bytes> = M6_PATCHES
        .iter()
        .map(|p| b::bytes_state(p, std::slice::from_raw_parts((bs + p.site.rva as usize) as *const u8, p.site.expect.len())))
        .collect();
    let Some(writes) = b::plan(M6_PATCHES, &now, ours) else {
        for (p, s) in M6_PATCHES.iter().zip(&now) {
            if *s == Bytes::Other {
                log(&format!("m6: {} holds bytes that are neither Re:CoM's nor ours; no patch switched", p.site.name));
            }
        }
        return None;
    };
    let mut done: Vec<b::Write> = Vec::new();
    for (i, from, to) in writes {
        let p = &M6_PATCHES[i];
        if !super::hook::patch(bs + p.site.rva as usize, from, to, p.site.name) {
            // Undo what this call wrote, so the three stay together.
            for &(j, from2, to2) in done.iter().rev() {
                let q = &M6_PATCHES[j];
                super::hook::patch(bs + q.site.rva as usize, to2, from2, q.site.name);
            }
            log("m6: a patch write failed; P2-P4 left as they were");
            return None;
        }
        done.push((i, from, to));
    }
    let n = done.len();
    PATCHED.store(ours, Ordering::Release);
    Some(n)
}

/// CBTL::setPlayCard (`0x140163830(cbtl, owner id, side, card, item, …)`): logs, once per class,
/// who still sets a side-2 card in a KH1 battle. Pass-through.
pub unsafe extern "win64" fn set_play_card(cb: usize, id: u32, side: i32, card: usize, item: usize, p6: i32) -> u64 {
    let r = orig::<SetPlayFn>(&ORIG_SET_PLAY_CARD)(cb, id, side, card, item, p6);
    if side == 2 && r as i32 > 0 && KH1_BATTLE.load(Ordering::Relaxed) {
        let obj = func::<mv::HandleFn>(mv::HANDLE_OBJ)(id);
        class_once(obj, 2, |_| format!("plays a side-2 card at KH1 (Re:CoM's card path: id {id}, {})", if card != 0 { "a revolver card" } else { "a new card" }));
    }
    r
}

/// The trace's M6 token (F9 recording only).
pub unsafe fn trace_token() -> String {
    let es = enemies(256);
    let attacking = es.iter().filter(|&&e| b::attacking(rd(e + enemy::FLAGS), state_of(e))).count();
    let cells: Vec<String> = es
        .iter()
        .take(b::TRACE_ENEMIES)
        .map(|&e| b::trace_enemy(rd(e + enemy::ID), state_of(e), rd(e + enemy::FLAGS), rd(e + enemy::COOLDOWN)))
        .collect();
    let c = task(b::TASK_CBTL);
    let slot = (c != 0).then(|| rd::<i32>(c + cbtl::SLOT_ENEMY));
    b::trace_token(KH1_BATTLE.load(Ordering::Relaxed), attacking, slot, &cells)
}

/// Installs the three wrappers, all or none (needs the movement handlers: the setting).
pub fn install(bs: usize) -> bool {
    let site = |name: &str| crate::sites::M6_HOOKS.iter().find(|h| h.name == name).expect("site");
    let hooks = [
        super::hook::Hook { site: site("enemy card play (0x140198870)"), ours: card_play as *const () as usize, original: &ORIG_CARD_PLAY },
        super::hook::Hook { site: site("battle enemy manager ctor (0x1401859B0)"), ours: manager_ctor as *const () as usize, original: &ORIG_MANAGER_CTOR },
        super::hook::Hook { site: site("CBTL setPlayCard (0x140163830)"), ours: set_play_card as *const () as usize, original: &ORIG_SET_PLAY_CARD },
    ];
    // SAFETY: the targets' bytes are checked again inside, before anything is written.
    unsafe { super::hook::install_all(bs, &hooks) }
}
