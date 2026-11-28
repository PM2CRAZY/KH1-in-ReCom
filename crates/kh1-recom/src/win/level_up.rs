//! M12 step 1: at Combat = KH1 in Sora's story, Re:CoM's level-up screen offers **AP Up** where it
//! offers CP (`crate::level_up`, `docs/recom-m12-sleights.md` § 8). Runs inside the existing
//! level-up wrapper (`super::mp::level_up`, `0x14010EF10`): no new hook, nothing per frame outside
//! that screen.
//!
//! - **Offer** (the init block just ran: state 0 / 1 → 2): row 1 offered (`+0x23C` = 0) whatever
//!   the CP (Re:CoM greys it at CP ≥ 1625; AP has no cap, owner); the cursor on it when HP isn't
//!   offered (Re:CoM's rule: the first offered row).
//! - **Words** (rows built: 2 → 3): row 1's label and value, the status panel's row 1 label, and
//!   its value (the max AP, or "now → +2" while the cursor is on row 1, as Re:CoM prints its CP)
//!   whenever Re:CoM re-prints the panel (states 3, 4 and 6: `0x140110C60(this, 7)`).
//! - **Pick** (6 → 7 with the cursor on row 1): one AP Up is added (`super::abilities::add_ap_up`),
//!   the result line ours. Re:CoM's +25 CP (`0x14010F6AB`) is kept, unseen (owner, 2026-11-03:
//!   CP keeps growing so a switch back to Re:CoM is fair), while Re:CoM itself would have offered
//!   its CP row (CP < 1625, `crate::level_up::keeps_recom_cp`); above Re:CoM's ceiling it is undone.
//! - **The end** (0xB → 0xD): the ability state is synced (the new level's KH1 abilities are
//!   queued as notices), then the notices' step (`super::notices::after_step`).

use super::movement::{func, rd, wr};
use super::log;
use crate::level_up::{off, state, text, CP_ROW, SLEIGHT_ROW};
use std::sync::atomic::{AtomicU32, Ordering};

const TEXT_COPY: u32 = 0x0025_8DE0;
type TextFn = unsafe extern "win64" fn(*mut u8, *const u8);

static LOG_LEFT: AtomicU32 = AtomicU32::new(60);

fn note(line: impl FnOnce() -> String) {
    if LOG_LEFT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1)).is_ok() {
        log(&format!("level-up: {}", line()));
    }
}

/// What the wrapper keeps across Re:CoM's update.
pub(super) struct Before {
    state: i32,
    /// Sora's CP (Riku's DP) before a confirm on the AP Up row (Re:CoM's add is kept only where
    /// Re:CoM itself offers its row).
    cp: Option<i32>,
    /// Riku: Re:CoM would have offered his DP row (Dark Mode learned, 0x6D, and DP < 99:
    /// `0x14010F2D2..0x14010F2ED`, Checked (disasm)).
    recom_dp_row: bool,
}

/// The screen is Riku's (`+0x230` ≠ 0; fork, 0.164.0: his DP row is his AP Up).
unsafe fn riku_screen(this: usize) -> bool {
    rd::<u32>(this + off::CHARACTER) != 0
}

/// The AP Up row: Sora's CP row, Riku's DP row (0.164.0; fork owner: "concordo").
unsafe fn ap_row(this: usize) -> usize {
    if riku_screen(this) {
        SLEIGHT_ROW
    } else {
        CP_ROW
    }
}

/// A level-up (not the sleight-learned mode) at Combat = KH1: Sora's in his story, Riku's in his
/// (0.164.0).
unsafe fn ap_up_screen(this: usize) -> bool {
    rd::<u32>(this + off::SLEIGHT) == 0 && if riku_screen(this) { super::mp::riku_live() } else { super::mp::active() }
}

pub(super) unsafe fn before(this: usize) -> Before {
    let st: i32 = rd(this + off::STATE);
    let on_row = st == state::PICK && ap_up_screen(this) && rd::<i32>(this + off::CURSOR) == ap_row(this) as i32;
    let cp = if !on_row {
        None
    } else if riku_screen(this) {
        super::stats::riku_dp()
    } else {
        super::stats::sora_cp()
    };
    let recom_dp_row = on_row && riku_screen(this) && super::abilities::sleight_learned_fn()(0x6D) & 0xFF != 0 && cp.is_some_and(|d| d < 99);
    Before { state: st, cp, recom_dp_row }
}

/// The C string a text item shows (at most 64 bytes read).
unsafe fn item_text(item: usize) -> Vec<u8> {
    let p: usize = rd(item + off::FNT_TEXT);
    if p == 0 {
        return Vec::new();
    }
    (0..64).map(|k| rd::<u8>(p + k)).take_while(|&b| b != 0).collect()
}

/// Sets a text item's text (Re:CoM's copy `0x140258DE0`), unless it already shows it.
unsafe fn set_text(this: usize, at: usize, want: &[u8]) {
    let item: usize = rd(this + at);
    if item == 0 || crate::level_up::same_text(&item_text(item), want) {
        return;
    }
    func::<TextFn>(TEXT_COPY)(item as *mut u8, want.as_ptr());
}

unsafe fn panel(this: usize) {
    // 0.166.0: Riku's panel keeps Re:CoM's DP row (his pick raises it).
    if riku_screen(this) {
        return;
    }
    let Some(max) = super::abilities::max_ap_now() else { return };
    let row = ap_row(this);
    let on = rd::<i32>(this + off::CURSOR) == row as i32;
    set_text(this, off::PANEL_VALUE + off::PANEL_STEP * row, &crate::level_up::panel_value(max, on));
}

pub(super) unsafe fn after(this: usize, b: Before) {
    let st: i32 = rd(this + off::STATE);
    if b.state != st || b.cp.is_some() || matches!(st, state::OPENING | 4 | state::PICK) {
        step(this, &b, st);
    }
    // The last level's screen ended: the ability state follows the new level now (KH1's level
    // abilities are queued as notices by the sync), before the notices' step looks.
    if b.state == state::NEXT && st == state::END && rd::<u32>(this + off::SLEIGHT) == 0 && super::mp::live() {
        super::abilities::sync();
    }
    super::notices::after_step(this, b.state, st);
}

unsafe fn step(this: usize, b: &Before, st: i32) {
    if !ap_up_screen(this) {
        return;
    }
    let row = ap_row(this);
    let offer = this + off::OFFER + 4 * row;
    // The offers were just made: AP Up is always offered (no cap).
    if (b.state == state::RESOURCES || b.state == state::INIT) && st == state::BUILD {
        let was: i32 = rd(offer);
        if was < 0 {
            wr(offer, 0i32);
            if rd::<i32>(this + off::OFFER) < 0 && (row == CP_ROW || rd::<i32>(this + off::OFFER + 4 * CP_ROW) < 0) {
                wr(this + off::CURSOR, row as i32);
            }
        }
        note(|| {
            format!(
                "Re:CoM level {}: AP Up offered in place of CP Up (Combat = KH1){}; max AP {:?}",
                super::stats::sora_level(),
                if was < 0 { format!(" (Re:CoM would grey it: CP {:?} >= {})", super::stats::sora_cp(), crate::level_up::RECOM_CP_OFFER_CAP) } else { String::new() },
                super::abilities::max_ap_now()
            )
        });
    }
    // The rows were just built: our words on row 1 and the panel.
    if b.state == state::BUILD && st == state::OPENING {
        let riku = riku_screen(this);
        set_text(this, off::ROW_LABEL + off::ROW_STEP * row, if riku { text::RIKU_ROW_LABEL } else { text::ROW_LABEL });
        set_text(this, off::ROW_VALUE + off::ROW_STEP * row, if riku { text::RIKU_ROW_VALUE } else { text::ROW_VALUE });
        if !riku {
            set_text(this, off::PANEL_LABEL + off::PANEL_STEP * row, text::PANEL_LABEL);
        }
        panel(this);
    }
    // Re:CoM re-prints the panel's values on cursor moves and when the rows have opened.
    if matches!(b.state, state::OPENING | 4 | state::PICK) && st != state::PICKED {
        panel(this);
    }
    // The pick: AP Up, not CP.
    if let (Some(dp), state::PICKED) = (b.cp, st) {
        if riku_screen(this) {
            if rd::<i32>(this + off::CURSOR) != row as i32 {
                return;
            }
            // 0.166.0: Re:CoM's +2 DP stays (Dark Mode's DP; Re:CoM's own cap 99), with the AP Up.
            let raised = super::stats::riku_dp();
            let r = super::abilities::add_ap_up();
            set_text(this, off::RESULT, text::RIKU_RESULT);
            log(&format!(
                "level-up: Riku's AP Up (his DP row) picked at Re:CoM level {}: {}; DP {dp} -> {raised:?} {}",
                super::stats::sora_level(),
                r.map_or_else(|| "the ability state isn't live: nothing added".to_string(), |(n, max)| format!("{n} AP Up pick(s), max AP {max}")),
                if b.recom_dp_row { "(Re:CoM's own DP pick)" } else { "(Re:CoM wouldn't offer its DP row: Dark Mode not learned yet or DP 99)" }
            ));
            return;
        }
        let cp = dp;
        if rd::<i32>(this + off::CURSOR) != CP_ROW as i32 {
            return;
        }
        let raised = super::stats::sora_cp();
        let keep = crate::level_up::keeps_recom_cp(cp);
        if !keep {
            super::stats::set_sora_cp(cp);
        }
        let r = super::abilities::add_ap_up();
        set_text(this, off::RESULT, text::RESULT);
        log(&format!(
            "level-up: AP Up picked at Re:CoM level {}: {}; {}",
            super::stats::sora_level(),
            r.map_or_else(|| "the ability state isn't live: nothing added".to_string(), |(n, max)| format!("{n} AP Up pick(s), max AP {max}")),
            if keep {
                format!("Re:CoM's +{} CP kept, unseen: CP {cp} -> {raised:?}", crate::abilities::RECOM_CP_PER_UP)
            } else {
                format!("Re:CoM's CP kept at {cp} (its own ceiling {}: it had made it {raised:?})", crate::level_up::RECOM_CP_OFFER_CAP)
            }
        ));
    }
}
