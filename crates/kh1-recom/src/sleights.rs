//! M12 step 5: the command menu's Sleights row and list (owner, 2026-11-03: "If you press left on
//! the d-pad, instead of summons at the bottom of the command menu, it should say "sleights" In the
//! sleights menu I only want ACTIVELY EQUIPPED sleights to show in the list"; answers: "D-pad Left
//! toggles Summon / Sleights anywhere on the command menu (drops KH1's Left → Attack)", "Cost: MP by
//! tier"). The testable part; the menu's rules are in `crate::command_menu`, the start in
//! `win/summons.rs`, notes `docs/recom-m12-sleights.md` § 8.5.
//!
//! - **The row**: the 4th root row on both of the command menu's pages (owner, 2026-11-04:
//!   "Page 1 (the default) should be Attack, Magic, Items, Sleights. Page 2 should be Attack,
//!   Summon, Party, Sleights"; `crate::command_menu::Page`). Until 0.111.0 D-pad Left switched
//!   this row between Summon and Sleights; it now switches the pages.
//! - **The list**: the equipped sleight abilities (`crate::abilities::equipped_sleights`), in
//!   Re:CoM's id order, as commands [`command`] (ours: KH1's command table has none), named by our
//!   words [`word`] (the abilities' names, English in every language, as Cloud's).
//! - **Usable** ([`usable`]): the summons' rule (`crate::summons::usable`, KH1's `0x140288F30`
//!   without its once-per-battle test): Re:CoM's battle room, KH1's ground state (flag 0x80), no
//!   sleight running, and the owner's MP ([`Cost`]). **Mid-air** too for Re:CoM's air-start
//!   sleights ([`AIR_START`], owner 2026-11-03, "Sleights menu follow-ups"), in a KH1 air state that
//!   lets a command start.
//! - **The start**: Re:CoM's own sleight start for the id (the summons' path, Sora's vtable `+0x130`
//!   = `0x1402247A0`); Re:CoM runs its motion, effects, hits, damage, protection and end.
//! - **Damage = KH1** ([`kh1_scaled`], owner, "Sleights menu follow-ups"): each hit Re:CoM's × the
//!   enemy's KH1-scale max HP (a twin's, a proxy's or a boss twin's) / its Re:CoM max HP; the same
//!   rule for the summons (`win::summons::scale_sleight_hit`).
//! - **KH1's specials** (M12 step 7, owner 2026-11-03: "Strike raid are arcanum and ragnarok in KH1FM
//!   are triangle prompts. Let them be triangle prompts and also show up in the sleights menu"):
//!   [`SPECIALS`] when equipped, under Re:CoM's sleight id of the same name (Strike Raid 8), named by
//!   KH1's own command word (338), usable when the special's own prompt would offer it (its
//!   `0x14028D0F0` score 2: a live target in range, its MP); confirm issues the special's command
//!   exactly as its prompt's Triangle does (`crate::specials`), not Re:CoM's sleight. Sonic Blade
//!   too (owner, 2026-11-03, "KH1 specials follow-ups": "Sonic Blade also in the Sleights list").

use crate::abilities::{self as ab, SleightMp};

/// The KH1 specials (by `crate::specials` index) the list also holds when equipped (M12 step 7):
/// Strike Raid, Ars Arcanum, Ragnarok, and (owner, 2026-11-03, "KH1 specials follow-ups") Sonic
/// Blade: every built special.
pub const SPECIALS: [usize; 4] = [crate::specials::SONIC_BLADE, crate::specials::ARS_ARCANUM, crate::specials::STRIKE_RAID, crate::specials::RAGNAROK];

/// The list's most rows: the sleight abilities and [`SPECIALS`].
pub const MAX: usize = ab::SLEIGHTS.end - ab::SLEIGHTS.start + SPECIALS.len();

/// The Re:CoM sleight id a listed special stands under (Strike Raid 8): its ability's source.
pub fn special_sleight(i: usize) -> Option<u16> {
    if !SPECIALS.contains(&i) {
        return None;
    }
    match ab::ALL.get(ab::special_ability(i)?)?.source {
        ab::Source::Sleight(id) => Some(id),
        _ => None,
    }
}

/// The listed special of a Re:CoM sleight id.
pub fn special_index(id: u16) -> Option<usize> {
    SPECIALS.iter().copied().find(|&i| special_sleight(i) == Some(id))
}

/// Our first sleight command: `0x80 + id` (ids 0x1B..=0x43 → 0x9B..=0xC3). KH1's command table
/// ends below 0x80 and our other commands sit elsewhere (Stagger 0xF0). Checked (code).
pub const COMMAND_BASE: u8 = 0x80;
/// Our words for the sleights' names: `0x8100 + id` (0x8000 = ours, as Cloud's 0x8029). The asset
/// step's `command_menu::SLEIGHT_WORDS` holds the same ids and names.
pub const WORD_BASE: u16 = 0x8100;
/// Our word for the row: "Sleights" (the owner's word; KH1 has none, as "Stagger" 901).
pub const ROW_WORD: u16 = 902;
pub const ROW_TEXT: &str = "Sleights";

/// The sleight ability of a Re:CoM id (one of the 27).
fn ability(id: u16) -> Option<usize> {
    ab::of_sleight(id)
}

/// The command of sleight `id`.
pub fn command(id: u16) -> u8 {
    COMMAND_BASE.wrapping_add(id as u8)
}

/// The sleight of a command (the 27 sleight abilities, the listed specials and (owner,
/// 2026-11-04) the Party list's friend sleights, `crate::party`).
pub fn of_command(c: u8) -> Option<u16> {
    let id = c.checked_sub(COMMAND_BASE)? as u16;
    (ability(id).is_some() || special_index(id).is_some() || crate::party::index(id).is_some() || crate::riku::sleight(id).is_some()).then_some(id)
}

/// The word of sleight `id` (a listed special: KH1's own word of its command).
pub fn word(id: u16) -> u16 {
    match special_index(id).and_then(|i| crate::specials::cmd::name(crate::specials::TABLE[i].1)) {
        Some(w) => w,
        None => WORD_BASE + id,
    }
}

/// Every sleight's word and name (the abilities' names), for the menu's word list.
pub fn words() -> impl Iterator<Item = (u16, &'static str)> {
    ab::SLEIGHTS.filter_map(|i| ab::sleight_of(i).map(|id| (word(id), ab::ALL[i].name)))
}

/// The sleight's name (the Abilities page's; a Party sleight's: the owner's).
pub fn name(id: u16) -> &'static str {
    match ability(id) {
        Some(i) => ab::ALL[i].name,
        None => crate::party::name(id).or_else(|| crate::riku::sleight(id).map(|s| s.name)).unwrap_or("?"),
    }
}

/// What sleight `id` costs (the owner's MP by tier).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cost {
    /// Whole MP points.
    Points(i32),
    /// All of Sora's whole MP; usable with at least 1 (owner: "cost all MP like KH1 summons", and the
    /// summons are "usable as long as you have MP").
    All,
    /// 0.179.0: Dark Riku's own sleights: Dark Mode's DP, this share (%) of its max
    /// (`crate::dark_points::sleight_cost`); no MP.
    Dp(i32),
}

pub fn cost(id: u16) -> Option<Cost> {
    // Riku's (`crate::riku::SLEIGHTS`): his own points.
    if let Some(&(_, pct)) = crate::dark_points::DP_SLEIGHTS.iter().find(|&&(i, _)| i == id) {
        return Some(Cost::Dp(pct));
    }
    if let Some(s) = crate::riku::sleight(id) {
        return Some(Cost::Points(s.mp));
    }
    Some(match ab::sleight_mp(ability(id)?)? {
        SleightMp::Points(n) => Cost::Points(n as i32),
        SleightMp::All => Cost::All,
    })
}

/// The whole MP points sleight `id` takes from `mp` (None: it can't pay). A Party sleight (owner,
/// 2026-11-04: "Same MP as sleights"): its own tier's points (`crate::party::points`).
pub fn pay_points(id: u16, mp: i32) -> Option<i32> {
    pay_points_with(id, mp, crate::enemy_skills_deck::sleight_lock())
}

/// [`pay_points`] with Riku's Sleight Lock on or off (Enemy Skills stage 5,
/// `crate::enemy_skills_deck::discount`: 1 point less, at least 1; Party sleights and the all-MP
/// ones too).
pub fn pay_points_with(id: u16, mp: i32, lock: bool) -> Option<i32> {
    let d = |n: i32| crate::enemy_skills_deck::discount(n, lock);
    if let Some(i) = crate::party::index(id) {
        let n = d(crate::party::ALL[i].mp);
        return (mp >= n).then_some(n);
    }
    match cost(id)? {
        Cost::Points(n) if mp >= d(n) => Some(d(n)),
        Cost::All if mp >= 1 => Some(d(mp)),
        _ => None,
    }
}

/// Re:CoM's sleights of the 27 that start in the air (owner, 2026-11-03, "Sleights menu
/// follow-ups": "Re:CoM's air-start sleights (Homing Blizzara, Warpinator, Warp, Bind, Confuse,
/// Terror, Teleport) usable mid-air"): the ids whose case in `initStockSkill` `0x140220DF0` asks
/// Re:CoM's airborne test `0x1401F3C00` and, in the air, takes its air version (`player+0x418 |=
/// 0x800000`, `+0x478` = 4 (the air clips; Teleport −36), which `initStockSkill`'s end turns into the
/// low-gravity bit `0x2000000`). Of Sora's other ids only the -ra / -ga tiers (0xD..=0x1A) do so; no
/// other of the 27 has an air case. Checked (decomp).
pub const AIR_START: [u16; 7] = [0x24, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x32];

/// Sleight `id` may start mid-air ([`AIR_START`]).
pub fn air_start(id: u16) -> bool {
    AIR_START.contains(&id)
}

/// KH1's usable test for sleight `id` (the summons' gate; `crate::summons::Gate`'s `out` = any
/// Re:CoM sleight running): on the ground, or (an [`AIR_START`] sleight) in a KH1 air state that
/// lets a command start (`Gate::air`); Re:CoM's start then picks its air version itself.
///
/// Outside battle (owner, 2026-11-04, "Sleights outside battle"): a field room too
/// (`crate::field_sleights::room_ok`: the pack ones only where Re:CoM's packs load), and free there.
pub fn usable(g: &crate::summons::Gate, id: u16) -> bool {
    usable_with_dp(g, id, None)
}

/// [`usable`] with Dark Riku's DP (`dp`: Some((DP now, Dark Mode's max)) while he is out, 0.179.0):
/// a DP sleight needs its cost and 1 DP left (`crate::dark_points::can_pay`), the others their MP.
pub fn usable_with_dp(g: &crate::summons::Gate, id: u16, dp: Option<(i32, i32)>) -> bool {
    let pays = if crate::dark_points::pays_dp(id) {
        dp.is_some_and(|(now, cap)| crate::dark_points::sleight_cost(id, cap).is_some_and(|c| crate::dark_points::can_pay(now, c)))
    } else {
        pay_points(id, g.mp).is_some()
    };
    crate::field_sleights::room_ok(g, id) && (g.ground || (g.air && air_start(id))) && !g.out && (crate::field_sleights::free(g) || pays)
}

/// A listed special's usable test (M12 step 7): battle (or, owner 2026-11-04, a field room:
/// `crate::field_sleights::room_ok`), on the ground, nothing running, and its own prompt's offer
/// (`offered`: the bit of its `crate::specials` index, `0x14028D0F0`'s score 2; its MP free in the
/// field, the caller's).
pub fn special_usable(g: &crate::summons::Gate, id: u16, offered: u8) -> bool {
    special_index(id).is_some_and(|i| crate::field_sleights::room_ok(g, id) && g.ground && !g.out && offered & (1 << i) != 0)
}

/// Re:CoM's hit kinds (the hurt body's `+0x80`) whose value isn't damage: Stop's ticks (8), Gravity's
/// percent (0xD), the timed states 0x10 / 0x17 (Shock Impact's, Lethal Frame's) and Magnet's 0x14
/// (their value goes to `+0x2D8` or is a share of HP, `0x14017DC50`). Re:CoM's own mask for them,
/// the hit handler's `0x912100` (`0x14017F7E0`: kinds above 0x17 or with their bit clear take its
/// frozen bonus). Checked (decomp).
pub const NOT_DAMAGE_KINDS: u32 = 0x0091_2100;

/// Whether a hit of Re:CoM kind `kind` carries damage (Re:CoM's rule, [`NOT_DAMAGE_KINDS`]).
pub fn is_damage_kind(kind: u32) -> bool {
    kind > 0x17 || (NOT_DAMAGE_KINDS >> kind) & 1 == 0
}

/// Damage = KH1 (owner, 2026-11-03, "Sleights menu follow-ups": "at Damage = KH1 sleight damage
/// scaled by the enemy's KH1 max HP / its Re:CoM max HP"): a sleight hit's value `value` (Re:CoM's,
/// on the hurt body `+0x92`, before Re:CoM's resistance, weak part and boss falloff, which then apply
/// to it as to any hit) on an enemy whose KH1 max HP is `kh1_max` and Re:CoM max HP `recom_max`:
/// value × kh1_max / recom_max, rounded, at least 1. None (Re:CoM's value stands): not a damage kind,
/// nothing to scale, or no HP to scale by.
pub fn kh1_scaled(value: i16, kind: u32, kh1_max: i32, recom_max: i32) -> Option<i16> {
    if !is_damage_kind(kind) || value <= 0 || kh1_max <= 0 || recom_max <= 0 {
        return None;
    }
    let v = (value as i64 * kh1_max as i64 * 2 + recom_max as i64) / (recom_max as i64 * 2);
    Some(v.clamp(1, i16::MAX as i64) as i16)
}

/// The list's ids from the equipped ones (Re:CoM's order; anything not a sleight ability dropped,
/// at most [`MAX`]).
pub fn list(equipped: &[u16]) -> Vec<u16> {
    let mut v: Vec<u16> = equipped.iter().copied().filter(|&id| ability(id).is_some() || special_index(id).is_some() || crate::riku::sleight(id).is_some()).collect();
    v.sort_unstable();
    v.dedup();
    v.truncate(MAX);
    v
}

/// Re:CoM's sleight dim (owner, 2026-11-03: "any sleight that sora does that dims the room should
/// give him i-frames"): the player's `+0x418` bit 31. Re:CoM's tint `0x1401F7D80` sets it for any
/// colour but white (`exeStockSkill` `0x140218950` calls it at a sleight's first update when its
/// start row has flag 0x20 (`+0xF`; 118 of the 188 rows, Checked (exe)), colour by attack id; our
/// KH1 specials call it with Re:CoM's colours); the sleight's end `0x1401F6660` fades back to white
/// and clears it, as `0x1401E7C20` does. Checked (decomp).
pub const DIM_BIT: u32 = 0x8000_0000;
/// Re:CoM's sleight action (`exeStockSkill`).
pub const STOCK_ACTION: u32 = 0x16;

/// Sora is untouchable for this hit (Combat = KH1, Sora's story; the caller checks those): his
/// sleight (Re:CoM's action 0x16, or a KH1 special) has the room dimmed, in the battle room.
pub fn dim_shield(flags_418: u32, action: u32, kh1_special: bool, in_battle: bool) -> bool {
    in_battle && flags_418 & DIM_BIT != 0 && (action == STOCK_ACTION || kh1_special)
}

/// Owner, 2026-11-04 (§ Party and field follow-ups, "While a sleight runs in the field"): Sora's
/// sleight, KH1 special or summon is running (Re:CoM's sleight action 0x16, which the summons and
/// field sleights also run on, or a KH1 special on its own action), in any room.
pub fn busy(action: u32, kh1_special: bool) -> bool {
    action == STOCK_ACTION || kh1_special
}

/// The camp doesn't open while [`busy`] in a field room (Combat = KH1, Sora's story: the caller
/// checks those; Re:CoM's field never runs action 0x16 itself).
pub fn camp_blocked(kh1: bool, field: bool, action: u32, kh1_special: bool) -> bool {
    kh1 && field && busy(action, kh1_special)
}

/// The field's form of [`dim_shield`]: "untouchable" against a field Heartless's **contact** means
/// no contact battle while the dimming sleight runs (field Heartless never damage Sora in the field:
/// their contact only starts the battle, `0x14019E020` → `0x14019BED0`). A field strike of Sora's
/// own sleight still starts its first-strike battle (the struck state's call `0x14019E8F0`, not this).
pub fn field_contact_shield(kh1: bool, field: bool, flags_418: u32, action: u32, kh1_special: bool) -> bool {
    kh1 && field && flags_418 & DIM_BIT != 0 && busy(action, kh1_special)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_rules_while_a_sleight_runs() {
        // The camp: blocked by a field sleight / summon (0x16) or a KH1 special, at KH1, in the field.
        assert!(camp_blocked(true, true, STOCK_ACTION, false));
        assert!(camp_blocked(true, true, 0x2E, true));
        assert!(!camp_blocked(true, true, 0xA, false), "a swing");
        assert!(!camp_blocked(false, true, STOCK_ACTION, false), "Combat = Re:CoM");
        assert!(!camp_blocked(true, false, STOCK_ACTION, false), "the battle room has no camp");
        // Contact: only while the running sleight dims the room.
        assert!(field_contact_shield(true, true, DIM_BIT, STOCK_ACTION, false));
        assert!(field_contact_shield(true, true, DIM_BIT | 0x1101, 0x2E, true));
        assert!(!field_contact_shield(true, true, 0, STOCK_ACTION, false), "no dim");
        assert!(!field_contact_shield(true, true, DIM_BIT, 0xA, false), "dim left over, no sleight");
        assert!(!field_contact_shield(false, true, DIM_BIT, STOCK_ACTION, false));
        assert!(!field_contact_shield(true, false, DIM_BIT, STOCK_ACTION, false), "battle: dim_shield's");
    }

    #[test]
    fn untouchable_while_the_sleight_dims_the_room() {
        assert!(dim_shield(DIM_BIT, STOCK_ACTION, false, true), "a Re:CoM sleight with its dim on");
        assert!(dim_shield(DIM_BIT | 1, 0x2E, true, true), "a KH1 special (its own action) with Re:CoM's dim");
        assert!(!dim_shield(0, STOCK_ACTION, false, true), "a sleight whose row has no dim (flag 0x20 clear)");
        assert!(!dim_shield(DIM_BIT, 0xA, false, true), "the dim alone (not a sleight)");
        assert!(!dim_shield(DIM_BIT, STOCK_ACTION, false, false), "outside the battle room");
    }
    use crate::abilities::sleight as s;
    use crate::summons::Gate;

    #[test]
    fn commands_and_words_round_trip_for_the_27() {
        assert_eq!(MAX, 27 + SPECIALS.len());
        let ids: Vec<u16> = ab::SLEIGHTS.filter_map(ab::sleight_of).collect();
        for &id in &ids {
            assert_eq!(of_command(command(id)), Some(id));
            assert!(command(id) >= 0x9B && command(id) <= 0xC3);
            assert_ne!(command(id), crate::gimmick::STAGGER);
            assert!(crate::summons::of_command(command(id)).is_none());
            assert!(crate::command_menu::cmd::spell_of(command(id)).is_none());
        }
        // Not the 27: the summons'. Sonic Blade is a listed special (its own test below).
        assert_eq!(of_command(command(s::SONIC_BLADE)), Some(s::SONIC_BLADE));
        assert_eq!(of_command(0x7F), None);
        assert_eq!(of_command(0x35), None);
        assert_eq!(words().count(), 27);
        assert!(words().any(|w| w == (word(s::LETHAL_FRAME), "Lethal Frame")));
        assert!(words().any(|w| w == (word(s::CROSS_SLASH_PLUS), "Cross-slash+")));
        assert_eq!(name(s::FREEZE), "Freeze");
    }

    #[test]
    fn mp_by_tier_and_all_mp() {
        // Owner: AP 2 → 1 MP, 3 → 2, 4 → 2, 5 → 3; Idyll Romp / Cross-slash+ all MP.
        assert_eq!(cost(0x1F), Some(Cost::Points(1)), "Judgment, 2 AP");
        assert_eq!(cost(s::FIRE_RAID), Some(Cost::Points(2)), "3 AP");
        assert_eq!(cost(0x1D), Some(Cost::Points(2)), "Thunder Raid, 4 AP");
        assert_eq!(cost(s::HOLY), Some(Cost::Points(3)), "5 AP");
        assert_eq!(cost(s::IDYLL_ROMP), Some(Cost::All));
        assert_eq!(cost(s::CROSS_SLASH_PLUS), Some(Cost::All));
        assert_eq!(cost(s::SONIC_BLADE), None);
        assert_eq!(pay_points(s::HOLY, 2), None);
        assert_eq!(pay_points(s::HOLY, 3), Some(3));
        assert_eq!(pay_points(s::IDYLL_ROMP, 0), None);
        assert_eq!(pay_points(s::IDYLL_ROMP, 1), Some(1));
        assert_eq!(pay_points(s::CROSS_SLASH_PLUS, 7), Some(7));
    }

    #[test]
    fn usable_is_the_summons_gate_with_the_sleights_mp() {
        let g = Gate { battle: true, ground: true, air: false, mp: 2, out: false, ..Default::default() };
        assert!(usable(&g, s::FIRE_RAID));
        assert!(!usable(&g, s::HOLY));
        assert!(usable(&g, s::IDYLL_ROMP));
        assert!(!usable(&Gate { battle: false, ..g }, s::FIRE_RAID), "battle rooms only");
        assert!(!usable(&Gate { ground: false, ..g }, s::FIRE_RAID), "on the ground");
        assert!(!usable(&Gate { out: true, ..g }, s::FIRE_RAID), "one sleight at a time");
        assert!(!usable(&Gate { mp: 0, ..g }, s::IDYLL_ROMP));
    }

    /// Owner (2026-11-04, "Sleights outside battle"): every sleight in a field room, free; the 18
    /// pack ones only where Re:CoM's packs load; the KH1 specials as their prompt.
    #[test]
    fn sleights_in_the_field_are_free() {
        let f = Gate { field: true, packs: true, ground: true, mp: 0, ..Default::default() };
        for id in ab::SLEIGHTS.filter_map(ab::sleight_of) {
            assert!(usable(&f, id), "{}: free in the field", name(id));
        }
        let none = Gate { packs: false, ..f };
        let ok: Vec<&str> = ab::SLEIGHTS.filter_map(ab::sleight_of).filter(|&id| usable(&none, id)).map(name).collect();
        assert_eq!(ok.len(), 9, "without packs only the 9 pack-less: {ok:?}");
        assert!(!usable(&Gate { out: true, ..f }, s::FIRE_RAID));
        assert!(!usable(&Gate { ground: false, ..f }, s::FIRE_RAID));
        assert!(usable(&Gate { ground: false, air: true, ..f }, 0x24), "Re:CoM's air start in the field too");
        // In battle the MP rule stays (0 MP: no).
        assert!(!usable(&Gate { battle: true, ..f }, s::FIRE_RAID));
        // The specials: their prompt's offer (MP free in the field: the caller's score).
        let bit = 1 << crate::specials::STRIKE_RAID;
        assert!(special_usable(&f, s::STRIKE_RAID, bit));
        assert!(special_usable(&none, s::STRIKE_RAID, bit), "Strike Raid has no pack");
        assert!(!special_usable(&none, s::SONIC_BLADE, 1 << crate::specials::SONIC_BLADE), "Sonic Blade's pack 11009");
        assert!(!special_usable(&Gate { field: false, ..f }, s::STRIKE_RAID, bit));
    }

    /// Owner (2026-11-03, "Sleights menu follow-ups"): Re:CoM's air-start sleights usable mid-air,
    /// the rest ground-only; exactly the ids `initStockSkill` gives an air version.
    #[test]
    fn air_start_sleights_are_recoms_seven() {
        let names: Vec<&str> = AIR_START.iter().map(|&id| name(id)).collect();
        assert_eq!(names, ["Homing Blizzara", "Warpinator", "Warp", "Bind", "Confuse", "Terror", "Teleport"]);
        let air = Gate { battle: true, ground: false, air: true, mp: 3, out: false, ..Default::default() };
        for id in ab::SLEIGHTS.filter_map(ab::sleight_of) {
            assert_eq!(usable(&air, id), air_start(id), "{}", name(id));
            // On the ground every one is usable as before.
            assert!(usable(&Gate { ground: true, air: false, ..air }, id), "{}", name(id));
        }
        // Mid-air still needs a battle room, nothing running and the MP; neither ground nor a KH1 air
        // state that takes a command (e.g. a reaction): none.
        assert!(!usable(&Gate { battle: false, ..air }, 0x24));
        assert!(!usable(&Gate { out: true, ..air }, 0x2C));
        assert!(!usable(&Gate { mp: 0, ..air }, 0x32));
        assert!(!usable(&Gate { air: false, ..air }, 0x24));
        // Not ours: the tiers (also air in Re:CoM) aren't sleight abilities.
        assert!(!air_start(0xD) && !air_start(s::FIRE_RAID) && !air_start(s::IDYLL_ROMP));
    }

    /// Owner (2026-11-03, "Sleights menu follow-ups"): at Damage = KH1 a sleight's damage is Re:CoM's
    /// × (the enemy's KH1 max HP / its Re:CoM max HP); values that aren't damage stay Re:CoM's.
    #[test]
    fn kh1_damage_scales_by_the_enemys_max_hp_ratio() {
        // A Shadow-like twin: KH1 max HP 24 against Re:CoM's 160: × 0.15.
        assert_eq!(kh1_scaled(500, 1, 24, 160), Some(75));
        // Rounded to the nearest (1000 × 37 / 300 = 123.3; 50 × 3 / 4 = 37.5 → 38).
        assert_eq!(kh1_scaled(1000, 1, 37, 300), Some(123));
        assert_eq!(kh1_scaled(50, 3, 3, 4), Some(38));
        // At least 1 (Re:CoM's own floor for a damaging hit); capped at the carrier.
        assert_eq!(kh1_scaled(3, 2, 1, 1000), Some(1));
        assert_eq!(kh1_scaled(i16::MAX, 1, 10, 1), Some(i16::MAX));
        // A KH1 HP above Re:CoM's scales up (the same formula).
        assert_eq!(kh1_scaled(100, 1, 300, 200), Some(150));
        // Not damage (Re:CoM's mask): Stop ticks 8 (Teleport), Gravity % 0xD, timed states 0x10 / 0x17,
        // Magnet 0x14.
        for k in [8u32, 0xD, 0x10, 0x14, 0x17] {
            assert!(!is_damage_kind(k));
            assert_eq!(kh1_scaled(9000, k, 24, 160), None, "kind {k:#x}");
        }
        for k in [0u32, 1, 2, 3, 7, 9, 0xA, 0xB, 0xC, 0xE, 0xF, 0x11, 0x12, 0x13, 0x15, 0x16, 0x1D, 0x1E] {
            assert!(is_damage_kind(k), "kind {k:#x}");
        }
        // Nothing to scale.
        assert_eq!(kh1_scaled(0, 1, 24, 160), None);
        assert_eq!(kh1_scaled(500, 1, 0, 160), None);
        assert_eq!(kh1_scaled(500, 1, 24, 0), None);
    }

    /// M12 step 7 (owner: "also show up in the sleights menu"): Strike Raid in the list under Re:CoM's
    /// sleight 8 (before the magic sleights, Re:CoM's order), KH1's word 338, usable as its prompt.
    #[test]
    fn strike_raid_is_listed_as_its_prompt() {
        assert_eq!(special_sleight(crate::specials::STRIKE_RAID), Some(s::STRIKE_RAID));
        assert_eq!(special_index(s::STRIKE_RAID), Some(crate::specials::STRIKE_RAID));
        assert_eq!(of_command(command(s::STRIKE_RAID)), Some(s::STRIKE_RAID));
        assert_eq!(command(s::STRIKE_RAID), 0x88);
        assert_eq!(word(s::STRIKE_RAID), 338);
        assert_eq!(cost(s::STRIKE_RAID), None, "its MP is KH1's (the special's own)");
        assert_eq!(list(&[s::HOLY, s::STRIKE_RAID, s::FIRE_RAID]), vec![s::STRIKE_RAID, s::FIRE_RAID, s::HOLY]);
        let g = Gate { battle: true, ground: true, air: false, mp: 0, out: false, ..Default::default() };
        let bit = 1 << crate::specials::STRIKE_RAID;
        assert!(special_usable(&g, s::STRIKE_RAID, bit));
        assert!(!special_usable(&g, s::STRIKE_RAID, 0), "its prompt wouldn't offer it");
        assert!(!special_usable(&Gate { ground: false, air: true, ..g }, s::STRIKE_RAID, bit), "ground only (KH1's state 0)");
        assert!(!special_usable(&Gate { out: true, ..g }, s::STRIKE_RAID, bit));
        assert!(!special_usable(&Gate { battle: false, ..g }, s::STRIKE_RAID, bit));
        assert!(!usable(&g, s::STRIKE_RAID), "not by the sleights' MP rule");
        // Ars Arcanum the same way: Re:CoM's sleight 10, KH1's word 331 ("Arcanum").
        assert_eq!(special_sleight(crate::specials::ARS_ARCANUM), Some(s::ARS_ARCANUM));
        assert_eq!((command(s::ARS_ARCANUM), word(s::ARS_ARCANUM)), (0x8A, 331));
        assert_eq!(special_sleight(crate::specials::RAGNAROK), Some(s::RAGNAROK));
        assert_eq!(special_index(s::RAGNAROK), Some(crate::specials::RAGNAROK));
        assert_eq!((command(s::RAGNAROK), word(s::RAGNAROK)), (0x8B, 334));
        assert_eq!(list(&[s::FIRE_RAID, s::ARS_ARCANUM, s::STRIKE_RAID]), vec![s::STRIKE_RAID, s::ARS_ARCANUM, s::FIRE_RAID]);
        assert!(special_usable(&g, s::ARS_ARCANUM, 1 << crate::specials::ARS_ARCANUM) && !special_usable(&g, s::ARS_ARCANUM, bit));
    }

    /// Owner (2026-11-03, "KH1 specials follow-ups": "Sonic Blade also in the Sleights list"): under
    /// Re:CoM's Sonic Blade sleight 9 (between Strike Raid 8 and Ars Arcanum 10, Re:CoM's order),
    /// KH1's own word 319 "Sonic" (its command 0x4B's), usable exactly when its prompt would offer it,
    /// its MP KH1's (2), not the tier rule.
    #[test]
    fn sonic_blade_is_listed_as_its_prompt() {
        use crate::specials::{self as sp, SONIC_BLADE};
        assert_eq!(SPECIALS.len(), sp::BUILT.len());
        assert!(sp::BUILT.iter().all(|i| SPECIALS.contains(i)), "every built special is listed");
        assert_eq!(special_sleight(SONIC_BLADE), Some(s::SONIC_BLADE));
        assert_eq!(s::SONIC_BLADE, sp::recom::SLEIGHT, "Re:CoM's Sonic Blade sleight 9");
        assert_eq!(special_index(s::SONIC_BLADE), Some(SONIC_BLADE));
        assert_eq!(command(s::SONIC_BLADE), 0x89);
        assert_eq!(sp::TABLE[SONIC_BLADE].1, sp::cmd::SONIC_BLADE);
        assert_eq!(word(s::SONIC_BLADE), 319, "KH1's \"Sonic\"");
        assert_eq!(cost(s::SONIC_BLADE), None, "its MP is KH1's (the special's own)");
        assert!(crate::summons::of_command(command(s::SONIC_BLADE)).is_none());
        assert!(crate::command_menu::cmd::spell_of(command(s::SONIC_BLADE)).is_none());
        assert_ne!(command(s::SONIC_BLADE), crate::gimmick::STAGGER);
        assert_eq!(
            list(&[s::RAGNAROK, s::HOLY, s::SONIC_BLADE, s::ARS_ARCANUM, s::STRIKE_RAID]),
            vec![s::STRIKE_RAID, s::SONIC_BLADE, s::ARS_ARCANUM, s::RAGNAROK, s::HOLY]
        );
        let g = Gate { battle: true, ground: true, air: false, mp: 0, out: false, ..Default::default() };
        let bit = 1 << SONIC_BLADE;
        assert!(special_usable(&g, s::SONIC_BLADE, bit));
        assert!(!special_usable(&g, s::SONIC_BLADE, 0), "its prompt wouldn't offer it");
        assert!(!special_usable(&g, s::SONIC_BLADE, 1 << crate::specials::STRIKE_RAID), "another special's bit");
        assert!(!special_usable(&Gate { ground: false, air: true, ..g }, s::SONIC_BLADE, bit), "ground only (KH1's state 0)");
        assert!(!special_usable(&Gate { out: true, ..g }, s::SONIC_BLADE, bit));
        assert!(!special_usable(&Gate { battle: false, ..g }, s::SONIC_BLADE, bit));
        assert!(!usable(&g, s::SONIC_BLADE), "not by the sleights' MP rule");
        // Its prompt's own score is what lights it: range 600, not more than 160 above, 2 MP.
        let near = sp::OfferTarget { gap: 100.0, above: 0.0 };
        assert_eq!(sp::score(SONIC_BLADE, &near, &mut |c| c <= 200), 2);
        assert_eq!(sp::score(SONIC_BLADE, &near, &mut |c| c < 200), 1, "MP short: not usable");
        assert_eq!(sp::score(SONIC_BLADE, &sp::OfferTarget { gap: 600.0, above: 0.0 }, &mut |_| true), 0);
        assert_eq!(sp::score(SONIC_BLADE, &sp::OfferTarget { gap: 100.0, above: 160.5 }, &mut |_| true), 0);
    }

    #[test]
    fn the_list_keeps_only_sleight_abilities_and_specials() {
        // The Party sleights (0x45 Magic LV3) never enter the Sleights list.
        assert_eq!(list(&[s::HOLY, s::FIRE_RAID, s::FIRE_RAID, 0x35, 0x45]), vec![s::FIRE_RAID, s::HOLY]);
    }

    #[test]
    fn party_sleights_are_commands_named_and_paid_by_their_tier() {
        assert_eq!(of_command(command(0x45)), Some(0x45));
        assert_eq!(of_command(command(0x44)), None, "Magic LV2 is not a Party entry");
        assert_eq!(name(0x4A), "Wild Crush");
        assert_eq!(word(0x4E), WORD_BASE + 0x4E);
        assert_eq!(pay_points(0x45, 2), Some(2));
        assert_eq!(pay_points(0x4A, 2), None);
    }
}
