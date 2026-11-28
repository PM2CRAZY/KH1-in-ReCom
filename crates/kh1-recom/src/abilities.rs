//! KH1's abilities at the KH1 setting (roadmap M9; `docs/recom-m9-abilities.md`): the ability
//! state, KH1's AP and the equip screen's rules (stage 9b), and the combo modifiers (stage 9a).
//!
//! - **What they do:** the equipped abilities become the `kh1_core::Abilities` the KH1 frame
//!   passes to KH1's own handlers and pickers ([`to_core`]): High Jump, Glide, Superglide, Dodge
//!   Roll, Guard, Counterattack, Combo Plus / Air Combo Plus (copies), Combo Master, and the nine
//!   combo modifiers (`0x1402A6EE0` / `0x1402A6C70` swap a modifier's row in when its bit in
//!   `0x142D5EC10` is set; Re:CoM's attack action then plays, collides and ends the row).
//! - **Unlocks (owner, M7–M9 choices and the 9b follow-ups):** with Re:CoM's level-up sleights
//!   (Re:CoM's own learned state, `0x14023AB00(sleight)`), or at KH1's learn level on Re:CoM's level
//!   (KH1's **Rod** list, btltbl `0x3BF8`: the mod's Dive path keeps the Rod, decision 18); Gravity
//!   Break at Re:CoM level 30, Combo Master at 50 (owner); High Jump, Glide and Superglide when
//!   Re:CoM gives Sora its own (Re:CoM's ability variable, [`recom`]). Unlocks are never taken back.
//! - **Sora's movement kit (owner, 2026-11-01, "just have guard and dodge roll be a part of sora's
//!   movement kit from the start, dont add them as abilities"):** Guard and Dodge Roll are always on
//!   at Combat = KH1 ([`Source::Kit`]): no AP, not on the Abilities page, never in the block's words
//!   (layout v4 drops them from older blocks and so refunds their AP).
//! - **Equipping (stage 9b):** KH1's rule: a learned ability arrives **unequipped**
//!   (`0x140284E10` writes `id | 0x80`, story grants too: event syscall `0x1401B1110` calls it);
//!   equipping one costs its AP (btltbl `0x33D8`), refused when the AP left is short
//!   (`0x140310650`); shared abilities (High Jump, Glide, Superglide: KH1's shared list `+0x599`)
//!   cost nothing and toggle freely. High Jump, Glide and Superglide arrive **equipped** when Re:CoM
//!   gives them (owner, 2026-11-01; still removable, and once removed never equipped again by us:
//!   only a new unlock is equipped, and the unlocked bit is the "seen" bit).
//! - **AP (M12, owner 2026-11-03):** KH1's starting 3, +2 for each "AP Up" Sora picks on Re:CoM's
//!   level-up screen at Combat = KH1 (it replaces the CP choice there), no cap, nothing else
//!   ([`max_ap`], [`State::ap_ups`]). KH1's own level grants are not used. An older save's past CP
//!   picks count as AP Ups once ([`ap_ups_from_cp`]).
//! - **Sleight abilities (M12 step 4, owner 2026-11-03):** the 25 magic sleights, Idyll Romp and
//!   Cross-slash+ ([`SLEIGHTS`]) are abilities on Sora's page with the owner's AP costs, unlocked
//!   when Re:CoM teaches them (its learned bit, `0x14023AB00`; never set by us), unequipped.
//!   [`equipped_sleights`] gives the Sleights menu (next step) their ids in Re:CoM's order.
//! - **Enemy Skills (owner, 2026-11-03 / -04; `crate::enemy_skills`, plan stage 1):** Re:CoM's enemy
//!   cards as abilities on a third owner row, "Enemy Skills" ([`page::Owner::EnemySkills`]): each one
//!   [`Source::Card`], learned at its card's first pickup (Re:CoM's discovery state `0x14015B3F0`),
//!   unequipped, with the owner's AP. The five KH1 matches are KH1's own abilities whose level grant
//!   the card replaces (Soldier: the Rod level-87 Combo Plus; Hook: Second Chance; Hades: Berserk;
//!   Creeper Plant: Leaf Bracer; Bouncywild: the level-9 Treasure Magnet). Skills whose effect isn't
//!   built yet are shown greyed and never equipped ([`built`]).
//! - **The state** is a block of Re:CoM's own save (Re:CoM saves, loads and zeroes it), after the
//!   items extension ([`BLOCK_FIRST`]), its words' bits 32..64 in an extension after the summon
//!   block ([`EXT_FIRST`], layout v6), and bits 64..128 in the skill extension after that
//!   ([`SKILL_EXT_FIRST`], layout v7).

/// The ability words: bit [`ix`] (bits 0..32 in the block, 32..64 in the extension, 64..128 in the
/// skill extension).
pub type Bits = u128;

/// Where an ability unlocks from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Re:CoM's sleight of this id is learned (its level-up list `0x14010EF10`).
    Sleight(u16),
    /// Re:CoM's level reaches this level (KH1's Rod list btltbl `0x3BF8`, or the owner's level).
    Level(u32),
    /// Sora's movement kit (owner, 2026-11-01): always on at Combat = KH1, never unlocked,
    /// equipped or paid for in the block (Guard, Dodge Roll).
    Kit,
    /// Re:CoM gives Sora its own: any of these bits of Re:CoM's ability variable ([`recom`]).
    Recom(u32),
    /// Enemy Skills: Re:CoM's enemy card of this kind obtained at least once (its discovery state,
    /// `0x14015B3F0`; `crate::enemy_skills`).
    Card(u16),
}

/// Re:CoM's own movement abilities: the bits of its save int variable (set 0, id 0x24), read with
/// Re:CoM's getter `0x1401B08C0(0, 0x24)`. Re:CoM's player setup `0x1401F0C50` copies them into the
/// player's ability word `+0x41C`, which its movement code reads. Checked (decomp + exe):
/// - bit 1 → `+0x41C & 0x200000`: its higher jump (jump init `0x1401F29F0`: row 0x14 and Sora's
///   jump speed 32.5 instead of 28.0, `0x1407C5B60 + 0x20` / `+0x1C`): **High Jump**;
/// - bit 2 → `0x400000` (Sora's story only): its glide (jump exe `0x1401EDA60` starts the glide
///   `CPlayerSora::initGlide` `0x1402202C0` on a jump press after 900 units in the air when
///   `+0x41C & 0xC00000`): **Glide**;
/// - bit 4 → `0x800000` (Sora's story only): the faster glide (glide exe `0x140217B50`: speed 8.0
///   instead of 5.0, `0x1407C5B48` / `0x1407C5B44`; row 0x1C instead of 0x1B), which also allows
///   the glide on its own: **Superglide** (and Glide).
///
/// Where Re:CoM sets them: bit 4 by its reward pick `0x1401BD9D0` in world 13 (Castle Oblivion), the
/// first time, unless byte variable (0, 0x61) is 3 (Checked (decomp)); every bit by the event
/// script machine `0x140145C50`: command 0x2C ORs bits into an int variable of set 0, command
/// 0x41 sets 0x24 to all bits (Checked (decomp)); the exe's only other writers are unreferenced
/// debug functions (`0x1400D1A00`, `0x1400D1FC0`, `0x1400D20B0`). Which events: Assumption (KH
/// wiki: High Jump after Monstro's Room of Beginnings, Glide on entering Neverland, Super Glide
/// in Castle Oblivion's Room of Rewards; Re:CoM's scripts are not in the session).
pub mod recom {
    /// Re:CoM's ability variable: int variable set 0, id 0x24.
    pub const VAR: (u32, u32) = (0, 0x24);
    pub const HIGH_JUMP: u32 = 1;
    pub const GLIDE: u32 = 2;
    pub const SUPERGLIDE: u32 = 4;
}

/// One ability of the state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ability {
    pub name: &'static str,
    /// KH1's ability id (the ability list `save + 0x44`, btltbl `0x33D8` row id − 1). None: not
    /// read yet (the specials, stage 9c).
    pub kh1_id: Option<u8>,
    /// KH1's bit in `0x142D5EC10` (`0x1402A6380`), 0 for the abilities KH1 counts elsewhere.
    pub kh1_bit: u32,
    /// KH1's AP cost (btltbl `0x33D8 + 12 × (id − 1)`, byte +0, read by `0x1402E8120`).
    pub ap: u8,
    /// KH1's sort key (the same record's u16 `+2`, `0x1402E81B0`): the equip screen's order.
    pub key: u16,
    /// KH1's shared list (`save + 0x599`, counted with list 0xB by `0x1402A6380`): no AP.
    pub shared: bool,
    pub source: Source,
}

/// Our index of each ability: its bit in the block's `unlocked` / `equipped` words. Never reorder.
pub mod ix {
    pub const SLIDING_DASH: usize = 0;
    pub const BLITZ: usize = 1;
    pub const STUN_IMPACT: usize = 2;
    pub const ZANTETSUKEN: usize = 3;
    pub const VORTEX: usize = 4;
    pub const AERIAL_SWEEP: usize = 5;
    pub const HURRICANE_BLAST: usize = 6;
    pub const RIPPLE_DRIVE: usize = 7;
    pub const GRAVITY_BREAK: usize = 8;
    pub const STRIKE_RAID: usize = 9;
    pub const SONIC_BLADE: usize = 10;
    pub const ARS_ARCANUM: usize = 11;
    pub const RAGNAROK: usize = 12;
    // Stage 9b.
    pub const GUARD: usize = 13;
    pub const COUNTERATTACK: usize = 14;
    pub const DODGE_ROLL: usize = 15;
    pub const HIGH_JUMP: usize = 16;
    pub const GLIDE: usize = 17;
    pub const SUPERGLIDE: usize = 18;
    pub const COMBO_MASTER: usize = 19;
    /// Three copies (KH1's Rod list gives three).
    pub const COMBO_PLUS: [usize; 3] = [20, 21, 22];
    /// Two copies (KH1's Rod list gives two).
    pub const AIR_COMBO_PLUS: [usize; 2] = [23, 24];
    // M12 step 3: KH1's Rod-list support abilities (`docs/recom-m12-sleights.md` § 3).
    /// MP Haste (KH1 id 23): the hit charge's factor + 1.5 (`0x1402BF330`).
    pub const MP_HASTE: usize = 25;
    /// MP Rage (KH1 id 24) ×2: charge from HP lost × the copies (`0x14029BBD0`).
    pub const MP_RAGE: [usize; 2] = [26, 27];
    /// Leaf Bracer (KH1 id 62): Sora's own Cure can't be interrupted (`0x14028F140`).
    pub const LEAF_BRACER: usize = 28;
    /// Berserk (KH1 id 26): + 4 Strength while HP is low (`0x1402B2DC0`).
    pub const BERSERK: usize = 29;
    /// Second Chance (KH1 id 25): a hit that would kill from above 1 HP leaves 1 (`0x1402BFA80`).
    pub const SECOND_CHANCE: usize = 30;
    // M12 step 4: the sleight abilities ([`super::SLEIGHTS`]), in Re:CoM's id order.
    /// The first sleight ability (Fire Raid, Re:CoM's 0x1B).
    pub const FIRST_SLEIGHT: usize = 31;
    /// Treasure Magnet (KH1 id 5) ×2: KH1's Rod list gives it at 9 and 72 (btltbl `0x3BF8`); the
    /// first copy is Bouncywild's card (Enemy Skills), the second stays by level. Not built yet.
    pub const TREASURE_MAGNET: [usize; 2] = [58, 59];
    /// The first Enemy Skill of its own (`crate::enemy_skills::Kind::Own`, in that table's order).
    pub const FIRST_SKILL: usize = 60;
}

/// The first Enemy Skill of its own ([`ix::FIRST_SKILL`]).
pub const FIRST_SKILL: usize = ix::FIRST_SKILL;

/// Re:CoM's sleight ids (its `initStockSkill` `0x140220DF0` switch; level-up list `0x14010EF10`;
/// names in `SY0002.CTD`'s order, Fira = 0xD). Checked (decomp + files).
pub mod sleight {
    pub const SLIDING_DASH: u16 = 4;
    pub const BLITZ: u16 = 5;
    pub const STUN_IMPACT: u16 = 6;
    pub const ZANTETSUKEN: u16 = 7;
    pub const STRIKE_RAID: u16 = 8;
    pub const SONIC_BLADE: u16 = 9;
    pub const ARS_ARCANUM: u16 = 10;
    pub const RAGNAROK: u16 = 11;
    // M12 step 4: the sleight abilities (`SY0002.CTD`'s names from id 4; Checked (files)).
    pub const FIRE_RAID: u16 = 0x1B;
    pub const FREEZE: u16 = 0x23;
    pub const LETHAL_FRAME: u16 = 0x27;
    pub const HOLY: u16 = 0x33;
    pub const IDYLL_ROMP: u16 = 0x3A;
    pub const CROSS_SLASH_PLUS: u16 = 0x43;
}

const fn a(name: &'static str, id: u8, kh1_bit: u32, ap: u8, key: u16, source: Source) -> Ability {
    Ability { name, kh1_id: Some(id), kh1_bit, ap, key, shared: false, source }
}

const fn shared(name: &'static str, id: u8, kh1_bit: u32, key: u16, bits: u32) -> Ability {
    Ability { name, kh1_id: Some(id), kh1_bit, ap: 0, key, shared: true, source: Source::Recom(bits) }
}

/// The sleight abilities' sort key: after every KH1 ability (KH1's keys end at 801), in Re:CoM's
/// id order.
pub const SLEIGHT_KEY: u16 = 1000;

/// A sleight ability (M12 step 4): no KH1 twin (no KH1 id, bit or row), the owner's AP cost.
const fn sl(name: &'static str, id: u16, ap: u8) -> Ability {
    Ability { name, kh1_id: None, kh1_bit: 0, ap, key: SLEIGHT_KEY + id, shared: false, source: Source::Sleight(id) }
}

/// The abilities before the Enemy Skills of their own ([`ALL`] adds them), by [`ix`]. Checked
/// (decomp + files): KH1 ids and bits
/// (`0x1402A6380`, `0x1402A5130`: Combo Plus 6, Air Combo Plus 7; `0x1402A7510`: Combo Master
/// 0x41), AP costs and sort keys (btltbl `0x33D8`, row id − 1 as `0x14028F8F0` indexes it), learn
/// levels (Rod list `0x3BF8`, entry i = level i + 2, `0x1402B51B0`). Names: English (KH1's own
/// ability names are its sysmsg, not in the asset step; the Archipelago list names the ids:
/// Assumption, strong).
/// Combo Master (0x41) has no row in btltbl's table (row 65 is zero: cost 0, key 0; Checked
/// (files)); KH1 FM gives it by no level list.
/// M12 step 4 (owner, 2026-11-03, "M12 answers"): the 25 magic-category sleights (0x1B..=0x33) and
/// Idyll Romp (0x3A), Cross-slash+ (0x43), ids and names as `SY0002.CTD` lists them from id 4
/// (Checked (files); the CTD writes 0x27 "Lethal Flame", the owner "Lethal Frame"), AP costs the
/// owner's: 2 — Judgment, Bind, Confuse, Terror, Warp, Warpinator, Synchro, Teleport, Gifted
/// Miracle, Idyll Romp; 3 — Fire Raid, Blizzard Raid, Aqua Splash, Freeze, Firaga Burst, Reflect
/// Raid, Homing Blizzara, Lethal Frame, Shock Impact; 4 — Thunder Raid, Raging Storm, Magnet Spiral,
/// Quake, Tornado, Cross-slash+; 5 — Holy, Mega Flare.
const BASE: [Ability; BASE_LEN] = [
    a("Sliding Dash", 54, 0x100_0000, 1, 205, Source::Sleight(sleight::SLIDING_DASH)),
    a("Blitz", 20, 0x8, 2, 208, Source::Sleight(sleight::BLITZ)),
    a("Stun Impact", 57, 0x800_0000, 2, 211, Source::Sleight(sleight::STUN_IMPACT)),
    a("Zantetsuken", 59, 0x2000_0000, 2, 213, Source::Sleight(sleight::ZANTETSUKEN)),
    a("Vortex", 17, 0x1, 1, 206, Source::Level(84)),
    a("Aerial Sweep", 18, 0x2, 2, 207, Source::Level(36)),
    a("Hurricane Blast", 55, 0x200_0000, 2, 209, Source::Level(18)),
    a("Ripple Drive", 56, 0x400_0000, 3, 210, Source::Level(12)),
    // KH1 learns Gravity Break from no level list; the owner chose Re:CoM level 30 (2026-11-01).
    a("Gravity Break", 58, 0x1000_0000, 2, 212, Source::Level(30)),
    // The specials (stage 9c): their state is kept; Sonic Blade (9c.1) is on the equip screen
    // (KH1 id 11, 3 AP, sort key 801: btltbl `0x33D8 + 12 × 10`, Checked (files)); M12 step 7, Strike
    // Raid (KH1 id 13, 3 AP, key 803: `0x33D8 + 12 × 12`, Checked (files)); the others wait.
    a("Strike Raid", 13, 0, 3, 803, Source::Sleight(sleight::STRIKE_RAID)),
    a("Sonic Blade", 11, 0, 3, 801, Source::Sleight(sleight::SONIC_BLADE)),
    // M12 step 7, Ars Arcanum (KH1 id 12, 4 AP, key 802: `0x33D8 + 12 × 11`, Checked (files)).
    a("Ars Arcanum", 12, 0, 4, 802, Source::Sleight(sleight::ARS_ARCANUM)),
    // M12 step 7, Ragnarok (KH1 id 14, 4 AP, key 804: `0x33D8 + 12 × 13`, Checked (files)).
    a("Ragnarok", 14, 0, 4, 804, Source::Sleight(sleight::RAGNAROK)),
    // Guard and Dodge Roll: Sora's movement kit (owner, 2026-11-01: "just have guard and dodge roll
    // be a part of sora's movement kit from the start, dont add them as abilities"). KH1's ids, bits
    // and costs stay for reference (KH1's Rod list has Guard at 33; Dodge Roll comes by story).
    a("Guard", 21, 0x10, 2, 101, Source::Kit),
    // Counterattack at the Rod list's 57.
    a("Counterattack", 19, 0x4, 2, 102, Source::Level(57)),
    // KH1 gives these by story (no level list holds them). High Jump, Glide, Superglide: where
    // Re:CoM gives Sora its own, equipped then (owner); Combo Master: Re:CoM level 50 (owner).
    a("Dodge Roll", 22, 0x20, 1, 100, Source::Kit),
    shared("High Jump", 1, 0x200, 1, recom::HIGH_JUMP),
    shared("Glide", 3, 0x80, 3, recom::GLIDE | recom::SUPERGLIDE),
    shared("Superglide", 4, 0x100, 4, recom::SUPERGLIDE),
    a("Combo Master", 0x41, 0, 0, 0, Source::Level(50)),
    // The Rod list's Combo Plus (87, 93, 99) and Air Combo Plus (24, 45).
    // Enemy Skills (owner): the first copy is the Soldier card's (KH1's Rod level 87 replaced).
    a("Soldier", 6, 0, 1, 201, Source::Card(0x45)),
    a("Combo Plus", 6, 0, 1, 201, Source::Level(93)),
    a("Combo Plus", 6, 0, 1, 201, Source::Level(99)),
    a("Air Combo Plus", 7, 0, 1, 202, Source::Level(24)),
    a("Air Combo Plus", 7, 0, 1, 202, Source::Level(45)),
    // M12 step 3 (owner, 2026-11-03: "As sora levels up in KH1, he receives abilities like MP Haste,
    // MP Rage, Leaf Bracer, … second chance and Berserk … at the levels he would normally get them …
    // regardless of level up choice"): KH1's Rod list (btltbl `0x3BF8`), on Re:CoM's level, unequipped.
    // They set KH1's actor ability block `+0x184` (`0x1402A6380`), not `0x142D5EC10`: kh1_bit 0.
    // MP Haste: id 23 (`+0x184` bit 0), Rod level 21, 3 AP, sort key 304. Checked (decomp + files).
    a("MP Haste", 23, 0, 3, 304, Source::Level(21)),
    // MP Rage: id 24 (`+0x184` bits 1..3, the count of equipped copies), Rod levels 42 and 75, 3 AP
    // each, sort key 305. Checked (decomp + files).
    a("MP Rage", 24, 0, 3, 305, Source::Level(42)),
    a("MP Rage", 24, 0, 3, 305, Source::Level(75)),
    // Leaf Bracer: id 62 (`+0x184` bit 6), Rod level 39, 5 AP, sort key 308. Checked (decomp + files).
    // Enemy Skills (owner): from the Creeper Plant card instead (KH1's Rod level 39 replaced).
    a("Creeper Plant", 62, 0, 5, 308, Source::Card(0x60)),
    // Berserk: id 26 (`+0x184` bit 5), Rod level 60, 1 AP, sort key 306. Checked (decomp + files).
    // Enemy Skills (owner): from the Hades card instead (KH1's Rod level 60 replaced).
    a("Hades", 26, 0, 1, 306, Source::Card(0x68)),
    // Second Chance: id 25 (`+0x184` bit 4), Rod level 90, 5 AP, sort key 307. Checked (decomp + files).
    // Enemy Skills (owner): from the Hook card instead (KH1's Rod level 90 replaced).
    a("Hook", 25, 0, 5, 307, Source::Card(0x6C)),
    // M12 step 4: the sleight abilities, in Re:CoM's id order (ix 31..58).
    sl("Fire Raid", 0x1B, 3),
    sl("Blizzard Raid", 0x1C, 3),
    sl("Thunder Raid", 0x1D, 4),
    sl("Reflect Raid", 0x1E, 3),
    sl("Judgment", 0x1F, 2),
    sl("Firaga Burst", 0x20, 3),
    sl("Raging Storm", 0x21, 4),
    sl("Mega Flare", 0x22, 5),
    sl("Freeze", 0x23, 3),
    sl("Homing Blizzara", 0x24, 3),
    sl("Aqua Splash", 0x25, 3),
    sl("Magnet Spiral", 0x26, 4),
    sl("Lethal Frame", 0x27, 3),
    sl("Shock Impact", 0x28, 3),
    sl("Tornado", 0x29, 4),
    sl("Quake", 0x2A, 4),
    sl("Warpinator", 0x2B, 2),
    sl("Warp", 0x2C, 2),
    sl("Bind", 0x2D, 2),
    sl("Confuse", 0x2E, 2),
    sl("Terror", 0x2F, 2),
    sl("Synchro", 0x30, 2),
    sl("Gifted Miracle", 0x31, 2),
    sl("Teleport", 0x32, 2),
    sl("Holy", 0x33, 5),
    sl("Idyll Romp", 0x3A, 2),
    sl("Cross-slash+", 0x43, 4),
    // Enemy Skills: Treasure Magnet (KH1 id 5, 2 AP, sort key 302: btltbl `0x33D8 + 12 × 4`; Rod
    // levels 9 and 72: Checked (files)). The level-9 copy is Bouncywild's card (owner: "the card
    // replaces the ability"); the level-72 copy stays by level. Not built yet ([`built`]).
    a("Bouncywild", 5, 0, 2, 302, Source::Card(0x4C)),
    a("Treasure Magnet", 5, 0, 2, 302, Source::Level(72)),
];

/// [`BASE`]'s length: KH1's abilities, the sleight abilities and Treasure Magnet.
pub const BASE_LEN: usize = ix::FIRST_SKILL;
/// Every ability: [`BASE`], then the Enemy Skills of their own (`crate::enemy_skills::Kind::Own`, in
/// that table's order), each [`Source::Card`] with the owner's AP.
pub const ALL_LEN: usize = BASE_LEN + crate::enemy_skills::OWN_COUNT;
pub const ALL: [Ability; ALL_LEN] = build_all();

/// The Enemy Skills' sort key base (after every KH1 ability and sleight; the page sorts its Enemy
/// Skills row by card kind).
pub const SKILL_KEY: u16 = 2000;

const fn build_all() -> [Ability; ALL_LEN] {
    let mut out = [BASE[0]; ALL_LEN];
    let mut i = 0;
    while i < BASE_LEN {
        out[i] = BASE[i];
        i += 1;
    }
    let mut k = 0;
    while k < crate::enemy_skills::OWN_COUNT {
        let s = crate::enemy_skills::own_at(k);
        out[BASE_LEN + k] = Ability { name: s.name, kh1_id: None, kh1_bit: 0, ap: s.ap, key: SKILL_KEY + s.card, shared: false, source: Source::Card(s.card) };
        k += 1;
    }
    out
}

/// The sleight abilities (M12 step 4), by [`ix`]: the 27 from [`ix::FIRST_SLEIGHT`].
pub const SLEIGHTS: std::ops::Range<usize> = ix::FIRST_SLEIGHT..ix::TREASURE_MAGNET[0];
/// Their bits.
pub const SLEIGHT_BITS: Bits = ((1 << SLEIGHTS.end) - 1) & !((1 << ix::FIRST_SLEIGHT) - 1);

/// Enemy Skills: every [`Source::Card`] ability (the five KH1 matches and the skills of their own).
pub const CARD_BITS: Bits = {
    let mut b: Bits = 0;
    let mut i = 0;
    while i < ALL_LEN {
        if let Source::Card(_) = ALL[i].source {
            b |= 1 << i;
        }
        i += 1;
    }
    b
};

/// The abilities whose effect isn't built yet: shown greyed, never equipped (every Enemy Skill of
/// its own until its stage of 2..5 builds it, `crate::skill_effects::BUILT`; Treasure Magnet's
/// level-72 copy with Bouncywild's, stage 2).
pub const UNBUILT_BITS: Bits = {
    let mut b: Bits = if crate::skill_effects::built_help(0x4C).is_some() { 0 } else { 1 << ix::TREASURE_MAGNET[0] | 1 << ix::TREASURE_MAGNET[1] };
    let mut k = 0;
    while k < crate::enemy_skills::SKILLS.len() {
        let s = &crate::enemy_skills::SKILLS[k];
        if !s.built {
            if let crate::enemy_skills::Kind::Kh1(i) = s.kind {
                b |= 1 << i;
            }
        }
        k += 1;
    }
    let mut k = 0;
    while k < crate::enemy_skills::OWN_COUNT {
        if !crate::enemy_skills::own_at(k).built {
            b |= 1 << (BASE_LEN + k);
        }
        k += 1;
    }
    b
};

/// Whether ability `i`'s effect is built (an unbuilt one is shown greyed and never equipped).
pub fn built(i: usize) -> bool {
    i < ALL.len() && UNBUILT_BITS & (1 << i) == 0
}

/// The Enemy Skill of ability `i` (a [`Source::Card`] ability).
pub fn skill_of(i: usize) -> Option<&'static crate::enemy_skills::Skill> {
    match ALL.get(i)?.source {
        Source::Card(c) => crate::enemy_skills::of_card(c),
        _ => None,
    }
}

/// The abilities whose source v7 changed to a card (the four KH1 matches built before as level
/// grants): an older block's are taken back at its migration and asked again from their card.
pub const V7_CARD_BITS: Bits = 1 << ix::COMBO_PLUS[0] | 1 << ix::LEAF_BRACER | 1 << ix::BERSERK | 1 << ix::SECOND_CHANCE;
/// The 58 abilities before layout v7 (a v6 block's words hold nothing past them).
pub const V6_BITS: Bits = (1 << 58) - 1;

/// The Re:CoM sleight id of a sleight ability ([`SLEIGHTS`]).
pub fn sleight_of(i: usize) -> Option<u16> {
    match ALL.get(i) {
        Some(Ability { source: Source::Sleight(id), .. }) if SLEIGHTS.contains(&i) => Some(*id),
        _ => None,
    }
}

/// The sleight ability of a Re:CoM sleight id.
pub fn of_sleight(id: u16) -> Option<usize> {
    SLEIGHTS.clone().find(|&i| sleight_of(i) == Some(id))
}

/// The equipped sleight abilities' Re:CoM ids, in Re:CoM's order (its sleight ids ascending; the
/// Sleights menu lists these, owner: "only ACTIVELY EQUIPPED sleights").
pub fn equipped_sleights(equipped: Bits) -> Vec<u16> {
    let mut v: Vec<u16> = SLEIGHTS.clone().filter(|&i| equipped & (1 << i) != 0).filter_map(sleight_of).collect();
    v.sort_unstable();
    v
}

/// What a sleight ability costs to perform (owner, 2026-11-03, "M12 answers": "MP by tier: AP 2 → 1
/// MP, 3 → 2 MP, 4 → 2 MP, 5 → 3 MP; Idyll Romp / Cross-slash+ cost all MP like KH1 summons").
/// Read by the Sleights menu (next step); the page's help line says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SleightMp {
    Points(u8),
    /// All of Sora's MP (KH1's summons' rule).
    All,
}

pub fn sleight_mp(i: usize) -> Option<SleightMp> {
    let id = sleight_of(i)?;
    if id == sleight::IDYLL_ROMP || id == sleight::CROSS_SLASH_PLUS {
        return Some(SleightMp::All);
    }
    Some(SleightMp::Points(match ALL[i].ap {
        0..=2 => 1,
        3 | 4 => 2,
        _ => 3,
    }))
}

/// The combo modifiers (stage 9a), by [`ix`].
pub const MODIFIERS: std::ops::Range<usize> = ix::SLIDING_DASH..ix::GRAVITY_BREAK + 1;
/// The specials (stage 9c): kept; Sonic Blade (9c.1) is on the screen and offered by the command
/// menu, the others wait (not on the screen, nothing reads them).
pub const SPECIALS: std::ops::Range<usize> = ix::STRIKE_RAID..ix::RAGNAROK + 1;
/// The specials built so far (stage 9c.1: Sonic Blade; M12 step 7: Strike Raid, Ars Arcanum,
/// Ragnarok).
pub const SPECIALS_BUILT: Bits = 1 << ix::SONIC_BLADE | 1 << ix::STRIKE_RAID | 1 << ix::ARS_ARCANUM | 1 << ix::RAGNAROK;

/// The ability of a KH1 special (`crate::specials` index: 0 Sonic Blade, 1 Ars Arcanum, 2 Strike
/// Raid, 3 Ragnarok).
pub fn special_ability(i: usize) -> Option<usize> {
    [ix::SONIC_BLADE, ix::ARS_ARCANUM, ix::STRIKE_RAID, ix::RAGNAROK].get(i).copied()
}
/// Every bit of [`ALL`].
pub const ALL_BITS: Bits = (1 << ALL.len()) - 1;
/// The modifiers' bits.
pub const MODIFIER_BITS: Bits = (1 << MODIFIERS.end) - 1;
const SPECIAL_BITS: Bits = ((1 << SPECIALS.end) - 1) & !MODIFIER_BITS;
/// The specials not built yet: not on the screen, never equipped.
pub const SPECIALS_WAITING: Bits = SPECIAL_BITS & !SPECIALS_BUILT;
/// Sora's movement kit ([`Source::Kit`]): always on, never in the block's words.
pub const KIT_BITS: Bits = 1 << ix::GUARD | 1 << ix::DODGE_ROLL;

// ---- AP ----

/// Sora's AP at a new game: the new-game record btltbl `0x39D8` byte 2 (→ record `+9`,
/// `0x1402852C0`), plus the Dive's AP delta for keep Rod / give up Shield (choice 5's fourth byte,
/// btltbl `0x128 + 4 × 5 + 3` = 0; `0x140284B70`). Checked (decomp + files).
pub const START_AP: i32 = 3;
/// AP per "AP Up" pick (owner, M12, 2026-11-03: "it should give you 2 AP each level"; KH1's own
/// AP bonus is also +2, btltbl `0x3AC0` kind 3 → `0x1402B5660` case 3). No cap (owner: "no cap";
/// KH1's record caps at 100, not used).
pub const AP_PER_UP: i32 = 2;

/// Sora's max AP: KH1's starting AP plus 2 per AP Up pick (owner, M12). Nothing else gives AP.
pub fn max_ap(ap_ups: u32) -> i32 {
    START_AP.saturating_add(AP_PER_UP.saturating_mul(ap_ups.min(i32::MAX as u32) as i32))
}

/// Re:CoM's CP per CP Up pick: its level-up confirm adds 0x19 to `stats + 0xC` (`0x14010EF10`
/// case 6, `add dword [rcx+0xC],0x19` at `0x14010F6AB`). Checked (decomp + exe).
pub const RECOM_CP_PER_UP: i32 = 25;
/// Sora's CP at a new game in Sora's story: the new game `0x1401AD2A0` calls `0x14023A040(1, 0,
/// 0)`, which loads CardInit record `index[0]` through `0x14023A250` (CP = bytes 9 / 0xB): level 1,
/// HP 80, **CP 275** (`SY0001.BIN` entry `CardInit.BIN`: index table at +0x10, records at +0x50).
/// Nothing else writes Sora's CP but the level-up pick (+25) and the unreferenced debug givers
/// `0x1400D1A00` (400 / 700): an exe-wide scan of every `[*0x14087CBF8 + 0xC]` access. So the base
/// CP is 275 at every level. Checked (files + decomp + exe).
pub const RECOM_START_CP: i32 = 275;

/// Riku's DP at his new game (fork, 0.164.0): `CardInit.BIN` record `index[15]` (level 1, HP 80,
/// DP 8, AP 10; `0x14023A250`'s Riku branch writes bytes 9 / 0xB to his stats `+0x14`). His
/// level-up DP row adds 2 (`0x14010F70F`, cap 99). Checked (files + disasm). Assumption: nothing
/// else raises it (his floors' decks don't load a CardInit record).
pub const RIKU_START_DP: i32 = 8;
/// Riku's DP per DP Up pick.
pub const RIKU_DP_PER_UP: i32 = 2;

/// Old saves (fork, 0.164.0, owner: "concordo" with the DP row becoming his AP Up): the DP Up picks
/// a Riku save has taken: (DP − 8) ÷ 2, at most level − 1, never below 0.
pub fn ap_ups_from_dp(dp: i32, level: u32) -> u8 {
    (((dp - RIKU_START_DP).max(0) / RIKU_DP_PER_UP) as u32).min(level.saturating_sub(1)).min(u8::MAX as u32) as u8
}

/// Old saves (owner, M12: "each CP Up already taken counts as an AP Up"): the CP Up picks a save
/// has taken, from its CP: (CP − 275) ÷ 25, at most one per level-up (level − 1), never below 0.
pub fn ap_ups_from_cp(cp: i32, level: u32) -> u8 {
    let picks = (cp.saturating_sub(RECOM_START_CP)).max(0) / RECOM_CP_PER_UP;
    picks.min(level.saturating_sub(1).min(255) as i32).clamp(0, 255) as u8
}

/// The AP the equipped abilities use: KH1's `0x1402E8660` sums the costs of Sora's equipped list
/// (the shared list is not counted; nor is the movement kit, which is no ability here).
pub fn used_ap(equipped: Bits) -> i32 {
    ALL.iter().enumerate().filter(|(i, a)| equipped & (1 << i) != 0 && !a.shared && a.source != Source::Kit).map(|(_, a)| a.ap as i32).sum()
}

/// The AP left: KH1's `0x1402E8660` (max − used, never below 0). `max`: [`max_ap`].
pub fn ap_left(equipped: Bits, max: i32) -> i32 {
    (max - used_ap(equipped)).max(0)
}

/// KH1's rule for equipping `i` (`0x140311C20` greys it, `0x140310650` refuses it): a shared
/// ability always; else its cost must not exceed the AP left.
pub fn can_equip(equipped: Bits, i: usize, max: i32) -> bool {
    ALL.get(i).is_some_and(|a| a.shared || a.ap as i32 <= ap_left(equipped, max))
}

/// Unequips from the most expensive down (ties: the last in [`ALL`]) until the used AP fits.
/// Returns the bits taken off. (KH1's AP never goes down, so KH1 has no rule for this; only a
/// migration can need it.)
pub fn fit_ap(equipped: &mut Bits, max: i32) -> Bits {
    let mut off = 0;
    while used_ap(*equipped) > max {
        let worst = ALL.iter().enumerate().filter(|(i, a)| *equipped & (1 << i) != 0 && !a.shared && a.source != Source::Kit && a.ap > 0).max_by_key(|(i, a)| (a.ap, *i));
        let Some((i, _)) = worst else { break };
        *equipped &= !(1 << i);
        off |= 1 << i;
    }
    off
}

// ---- the save block ----

/// Our ability block: right after the items extension, inside Re:CoM's spare tail
/// `save + 0x246B ..= 0x2817` that no Re:CoM code reads or writes (`docs/recom-keyblade-equip.md`).
pub const BLOCK_FIRST: usize = crate::items::EXT_LAST + 1;
pub const BLOCK_LEN: usize = 15;
pub const BLOCK_LAST: usize = BLOCK_FIRST + BLOCK_LEN - 1;

/// Offsets inside the block. Fields never move.
pub mod field {
    /// 0 = never written (an older save, or a new game); 1 = stage 9a (unlocked = equipped);
    /// 2 = stage 9b (KH1's equip rules; the story abilities from the ini); 3 = the 9b follow-ups
    /// (their sources from Re:CoM, the start and levels); 4 = Guard and Dodge Roll in Sora's kit,
    /// not in the words (Re:CoM's movement abilities arrive equipped); 5 = the AP Up count
    /// ([`AP_UPS`]) is valid (M12); 6 = the words' bits 32.. are in the extension
    /// ([`super::EXT_FIRST`], M12 step 4: the sleight abilities); 7 = Enemy Skills: bits 64.. in
    /// the skill extension ([`super::SKILL_EXT_FIRST`]), the KH1 matches from their cards. Same
    /// fields in all.
    pub const VERSION: usize = 0;
    /// u32 LE: unlocked abilities, bits 0..32 of [`super::Bits`] (bit [`super::ix`]).
    pub const UNLOCKED: usize = 1;
    /// u32 LE: equipped abilities, bits 0..32.
    pub const EQUIPPED: usize = 5;
    /// u8 (layout v5): the AP Up picks taken on Re:CoM's level-up screen at Combat = KH1 (M12),
    /// plus, once, the CP Up picks of the save before v5 ([`super::ap_ups_from_cp`]). Was the first
    /// reserved byte (always zero before v5; nothing read it). Checked (code).
    pub const AP_UPS: usize = 9;
    /// 5 bytes kept zero.
    pub const RESERVED: usize = 10;
}

pub const BLOCK_V1: u8 = 1;
pub const BLOCK_V2: u8 = 2;
pub const BLOCK_V3: u8 = 3;
pub const BLOCK_V4: u8 = 4;
pub const BLOCK_V5: u8 = 5;
pub const BLOCK_V6: u8 = 6;
pub const BLOCK_V7: u8 = 7;
/// The layout this build writes.
pub const BLOCK_NOW: u8 = BLOCK_V7;

const _: () = assert!(BLOCK_LAST <= crate::keyblades::SPARE_LAST && field::RESERVED + 5 == BLOCK_LEN && field::AP_UPS + 1 == field::RESERVED);

/// The words' extension (layout v6, M12 step 4): bits 32.. of the unlocked and equipped words,
/// right after the summon block (`crate::summons::BLOCK_LAST`), still inside Re:CoM's spare tail
/// `save + 0x246B ..= 0x2817` (no Re:CoM code reads or writes it, `docs/recom-keyblade-equip.md`;
/// none of our other blocks reaches it: Checked (code)). Read only from a v6 block: before v6 its
/// bytes mean nothing (they are zero: the spare tail's report says so in the log).
pub const EXT_FIRST: usize = crate::summons::BLOCK_LAST + 1;
pub const EXT_LEN: usize = 8;
pub const EXT_LAST: usize = EXT_FIRST + EXT_LEN - 1;

/// Offsets inside the extension.
pub mod ext {
    /// u32 LE: unlocked abilities, bits 32..64.
    pub const UNLOCKED_HIGH: usize = 0;
    /// u32 LE: equipped abilities, bits 32..64.
    pub const EQUIPPED_HIGH: usize = 4;
}

const _: () = assert!(EXT_LAST <= crate::keyblades::SPARE_LAST && ext::EQUIPPED_HIGH + 4 == EXT_LEN && ALL.len() <= 128);

/// The skill extension (layout v7, Enemy Skills): bits 64..128 of the unlocked and equipped words,
/// right after the words' extension ([`EXT_LAST`]), still inside Re:CoM's spare tail `save + 0x246B
/// ..= 0x2817` (no Re:CoM code reads or writes it, `docs/recom-keyblade-equip.md`; none of our
/// other blocks reaches it: Checked (code)). Its own version byte: 0 = never written (read as
/// nothing, whatever the rest holds).
pub const SKILL_EXT_FIRST: usize = EXT_LAST + 1;
pub const SKILL_EXT_LEN: usize = 17;
pub const SKILL_EXT_LAST: usize = SKILL_EXT_FIRST + SKILL_EXT_LEN - 1;
pub const SKILL_EXT_V1: u8 = 1;

/// Offsets inside the skill extension.
pub mod skill_ext {
    /// u8: 0 never written; 1 = [`super::SKILL_EXT_V1`].
    pub const VERSION: usize = 0;
    /// u64 LE: unlocked abilities, bits 64..128.
    pub const UNLOCKED_TOP: usize = 1;
    /// u64 LE: equipped abilities, bits 64..128.
    pub const EQUIPPED_TOP: usize = 9;
}

const _: () = assert!(SKILL_EXT_LAST <= crate::keyblades::SPARE_LAST && skill_ext::EQUIPPED_TOP + 8 == SKILL_EXT_LEN);

/// What a block holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub unlocked: Bits,
    pub equipped: Bits,
    /// AP Up picks (layout v5; 0 in an older block until its migration sets it).
    pub ap_ups: u8,
}

impl State {
    /// Sora's max AP ([`max_ap`]).
    pub fn max_ap(&self) -> i32 {
        max_ap(self.ap_ups as u32)
    }
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn u64_at(b: &[u8], o: usize) -> u64 {
    let mut x = [0u8; 8];
    x.copy_from_slice(&b[o..o + 8]);
    u64::from_le_bytes(x)
}

/// The words as stored (with the kit's bits an older block may still hold, which [`read`] hides):
/// bits 0..32 from the block; bits 32..64 from the extension only at layout v6 and later (before v6
/// the low words never held bit 31 either: 31 abilities; v6 held 58); bits 64..128 from the skill
/// extension only at layout v7 with its version byte set.
pub fn raw(block: &[u8; BLOCK_LEN], ext: &[u8; EXT_LEN], skills: &[u8; SKILL_EXT_LEN]) -> State {
    let v = block[field::VERSION];
    if v == 0 {
        return State::default();
    }
    let (lo_mask, hi) = if v >= BLOCK_V6 { (u32::MAX, true) } else { (0x7FFF_FFFF, false) };
    let top = v >= BLOCK_V7 && skills[skill_ext::VERSION] >= SKILL_EXT_V1;
    let mask: Bits = if v >= BLOCK_V7 { Bits::MAX } else { V6_BITS };
    let word = |lo: usize, high: usize, t: usize| {
        let w = (u32_at(block, lo) & lo_mask) as Bits | if hi { (u32_at(ext, high) as Bits) << 32 } else { 0 } | if top { (u64_at(skills, t) as Bits) << 64 } else { 0 };
        w & mask
    };
    let ap_ups = if v >= BLOCK_V5 { block[field::AP_UPS] } else { 0 };
    State { unlocked: word(field::UNLOCKED, ext::UNLOCKED_HIGH, skill_ext::UNLOCKED_TOP), equipped: word(field::EQUIPPED, ext::EQUIPPED_HIGH, skill_ext::EQUIPPED_TOP), ap_ups }
}

/// The state a block (and its extensions) holds (a block never written: nothing unlocked).
pub fn read(block: &[u8; BLOCK_LEN], ext: &[u8; EXT_LEN], skills: &[u8; SKILL_EXT_LEN]) -> State {
    let r = raw(block, ext, skills);
    // The kit is never in the words (an older block's Guard / Dodge Roll bits read as nothing; the
    // migration to v4 clears them in the save); bits past [`ALL`] mean nothing.
    let unlocked = r.unlocked & ALL_BITS & !KIT_BITS;
    State { unlocked, equipped: r.equipped & unlocked, ap_ups: r.ap_ups }
}

/// The block's version.
pub fn version(block: &[u8; BLOCK_LEN]) -> u8 {
    block[field::VERSION]
}

/// Writes the state as layout [`BLOCK_NOW`] (the AP Up count, the words' bits 32..64 into the
/// extension, 64..128 into the skill extension, its version byte set); the reserved bytes are kept.
pub fn write(s: &State, block: &mut [u8; BLOCK_LEN], ext: &mut [u8; EXT_LEN], skills: &mut [u8; SKILL_EXT_LEN]) {
    block[field::VERSION] = block[field::VERSION].max(BLOCK_NOW);
    skills[skill_ext::VERSION] = skills[skill_ext::VERSION].max(SKILL_EXT_V1);
    skills[skill_ext::UNLOCKED_TOP..skill_ext::UNLOCKED_TOP + 8].copy_from_slice(&((s.unlocked >> 64) as u64).to_le_bytes());
    skills[skill_ext::EQUIPPED_TOP..skill_ext::EQUIPPED_TOP + 8].copy_from_slice(&((s.equipped >> 64) as u64).to_le_bytes());
    block[field::UNLOCKED..field::UNLOCKED + 4].copy_from_slice(&(s.unlocked as u32).to_le_bytes());
    block[field::EQUIPPED..field::EQUIPPED + 4].copy_from_slice(&(s.equipped as u32).to_le_bytes());
    block[field::AP_UPS] = s.ap_ups;
    ext[ext::UNLOCKED_HIGH..ext::UNLOCKED_HIGH + 4].copy_from_slice(&((s.unlocked >> 32) as u32).to_le_bytes());
    ext[ext::EQUIPPED_HIGH..ext::EQUIPPED_HIGH + 4].copy_from_slice(&((s.equipped >> 32) as u32).to_le_bytes());
}

// ---- unlocks ----

/// Whether an ability's source is met: Re:CoM's sleight learned (`learned`, Re:CoM's own
/// `0x14023AB00`), Re:CoM's level, Re:CoM's ability variable (`recom_var`, [`recom`]), or (Enemy
/// Skills) its enemy card obtained at least once (`card`, Re:CoM's discovery state `0x14015B3F0`).
/// The kit is never unlocked (it is always on, outside the block).
pub fn source_met(a: &Ability, level: u32, recom_var: u32, learned: &mut dyn FnMut(u16) -> bool, card: &mut dyn FnMut(u16) -> bool) -> bool {
    match a.source {
        Source::Sleight(id) => learned(id),
        Source::Level(l) => l <= level,
        Source::Kit => false,
        Source::Recom(bits) => recom_var & bits != 0,
        Source::Card(c) => card(c),
    }
}

/// The abilities that arrive equipped: High Jump, Glide and Superglide when Re:CoM gives them
/// (owner, 2026-11-01: "equipped automatically when Re:CoM gives them"; shared, so no AP).
pub fn arrive_equipped_bits() -> Bits {
    ALL.iter().enumerate().filter(|(_, a)| matches!(a.source, Source::Recom(_))).fold(0, |b, (i, _)| b | 1 << i)
}

/// The abilities a layout-v2 block had from the ini's story keys (High Jump, Glide, Superglide,
/// Combo Master): v2 → now takes them back and asks their new source.
pub const V2_STORY_BITS: Bits = 1 << ix::HIGH_JUMP | 1 << ix::GLIDE | 1 << ix::SUPERGLIDE | 1 << ix::COMBO_MASTER;

/// What a sync changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Synced {
    /// Newly unlocked and left unequipped (KH1's rule).
    pub new: Bits,
    /// Newly unlocked and equipped (Re:CoM's High Jump, Glide, Superglide; owner).
    pub new_equipped: Bits,
    /// Taken back (only at the v2 → v3 migration: the ini's story abilities whose new source isn't
    /// met yet).
    pub taken: Bits,
    /// Taken off to fit the AP (only at a migration).
    pub fitted: Bits,
    /// Guard and Dodge Roll found in an older block's words and dropped (the kit; their AP back).
    pub kit_dropped: Bits,
}

/// Brings the state up to Re:CoM's sleights, its level and its ability variable: every ability
/// not yet unlocked whose source is met is unlocked **unequipped** (KH1's `0x140284E10`), but
/// Re:CoM's movement abilities arrive equipped (owner). Never taken back; only abilities not yet
/// unlocked are asked about, so one the player removed is never equipped again. No enemy card
/// counts as obtained ([`sync_with`] asks Re:CoM's discovery state).
pub fn sync(s: &mut State, level: u32, recom_var: u32, learned: &mut dyn FnMut(u16) -> bool) -> Synced {
    sync_with(s, level, recom_var, learned, &mut |_| false)
}

/// [`sync`], with the Enemy Skills' cards: `card(kind)` = Re:CoM's "obtained at least once"
/// (`0x14015B3F0`). A skill arrives unequipped (KH1's rule, owner).
pub fn sync_with(s: &mut State, level: u32, recom_var: u32, learned: &mut dyn FnMut(u16) -> bool, card: &mut dyn FnMut(u16) -> bool) -> Synced {
    let mut out = Synced::default();
    let eq = arrive_equipped_bits();
    for (i, a) in ALL.iter().enumerate() {
        let bit = 1 << i;
        if s.unlocked & bit != 0 || !source_met(a, level, recom_var, learned, card) {
            continue;
        }
        s.unlocked |= bit;
        if eq & bit != 0 && can_equip(s.equipped, i, s.max_ap()) {
            s.equipped |= bit;
            out.new_equipped |= bit;
        } else {
            out.new |= bit;
        }
    }
    out
}

/// The ini's movement abilities at the migration (stage 9a's block or none → now): which of the
/// abilities that now come from KH1's levels the ini had equipped (Counterattack, the first
/// `combo_plus` / `air_combo_plus` copies). The ini's `guard` no longer counts: Guard is Sora's kit.
pub fn ini_equipped(a: &crate::settings::Kh1Abilities) -> Bits {
    let mut b = 0;
    if a.counterattack {
        b |= 1 << ix::COUNTERATTACK;
    }
    // The level copies first (93, 99), then the Soldier card's (Enemy Skills).
    for (k, &i) in [ix::COMBO_PLUS[1], ix::COMBO_PLUS[2], ix::COMBO_PLUS[0]].iter().enumerate() {
        if (k as u8) < a.combo_plus {
            b |= 1 << i;
        }
    }
    for (k, &i) in ix::AIR_COMBO_PLUS.iter().enumerate() {
        if (k as u8) < a.air_combo_plus {
            b |= 1 << i;
        }
    }
    b
}

/// The migration of a block older than [`BLOCK_NOW`] (`from` = its version 0..=3) at its first
/// sync. `raw` is the block's words as stored (with the kit bits [`read`] hides):
/// - every older block: Guard and Dodge Roll leave the words (Sora's kit now; their AP is back);
/// - v2 (9b): the ini's story abilities ([`V2_STORY_BITS`]) are taken back, to be asked again from
///   their new source;
/// - v3: Re:CoM's movement abilities already unlocked (they arrived unequipped by v3's rule) are
///   equipped once, as the owner's new rule gives them;
/// - the sync's unlocks (Re:CoM's movement abilities equipped);
/// - everything equipped before stays equipped where it is still unlocked: the block's own set
///   (v1: 9a equipped every unlock) and, from v0 / v1, the ini's ([`ini_equipped`]; read once per
///   save);
/// - v0..=v4 → v5 (M12): the AP Up count starts at `cp_ups` (the save's past CP Up picks,
///   [`ap_ups_from_cp`]);
/// - v0..=v5 → v6 (M12 step 4): the words' extension starts empty ([`raw`] reads none before v6);
///   the sleight abilities Re:CoM already taught arrive with the sync, unequipped (their notices:
///   none, `crate::notices::sleight_abilities`);
/// - v0..=v6 → v7 (Enemy Skills): the four KH1 matches built before as level grants
///   ([`V7_CARD_BITS`]: Combo Plus 87, Leaf Bracer, Berserk, Second Chance) are taken back and asked
///   again from their card (owner: "That ability then comes from the card, not from its level"; as
///   v2 → v3 did for the ini's story abilities); one whose card Sora already has comes straight back,
///   equipped as it was; the skills of the cards Sora already has arrive unequipped (no notices:
///   `crate::notices::card_abilities`);
/// - then over the max AP, unequipped from the most expensive down ([`fit_ap`]; KH1's AP never
///   drops, so KH1 has no rule here).
#[allow(clippy::too_many_arguments)]
pub fn migrate(s: &mut State, raw: State, from: u8, level: u32, recom_var: u32, ini: Bits, cp_ups: u8, learned: &mut dyn FnMut(u16) -> bool) -> Synced {
    migrate_with(s, raw, from, level, recom_var, ini, cp_ups, learned, &mut |_| false)
}

/// [`migrate`], with the Enemy Skills' cards (`card`, as [`sync_with`]).
#[allow(clippy::too_many_arguments)]
pub fn migrate_with(s: &mut State, raw: State, from: u8, level: u32, recom_var: u32, ini: Bits, cp_ups: u8, learned: &mut dyn FnMut(u16) -> bool, card: &mut dyn FnMut(u16) -> bool) -> Synced {
    if from < BLOCK_V5 {
        s.ap_ups = cp_ups;
    }
    let kit_dropped = (raw.unlocked | raw.equipped) & KIT_BITS;
    s.unlocked &= !KIT_BITS;
    s.equipped &= s.unlocked;
    let mut before = s.equipped;
    if from == BLOCK_V3 {
        before |= s.unlocked & arrive_equipped_bits();
    }
    let mut taken = 0;
    if from == BLOCK_V2 {
        taken = s.unlocked & V2_STORY_BITS;
        s.unlocked &= !V2_STORY_BITS;
        s.equipped &= s.unlocked;
    }
    let mut taken7 = 0;
    if from < BLOCK_V7 {
        taken7 = s.unlocked & V7_CARD_BITS;
        taken |= taken7;
        s.unlocked &= !V7_CARD_BITS;
        s.equipped &= s.unlocked;
    }
    let mut out = sync_with(s, level, recom_var, learned, card);
    out.taken = taken & !s.unlocked;
    out.kit_dropped = kit_dropped;
    let ini = if from < BLOCK_V2 { ini } else { 0 };
    let wanted = (out.new | out.new_equipped) & arrive_equipped_bits();
    s.equipped |= (before | ini | wanted) & s.unlocked;
    let max = s.max_ap();
    out.fitted = fit_ap(&mut s.equipped, max);
    // Report what really arrived equipped after the fit.
    let fresh = out.new | out.new_equipped;
    out.new_equipped = fresh & wanted & s.equipped;
    out.new = fresh & !out.new_equipped;
    // v7: a KH1 match whose card Sora already had came straight back: not new.
    out.new &= !taken7;
    out
}

// ---- use ----

/// The equipped abilities as KH1's bits (`0x142D5EC10` layout), as the trace writes them; Sora's
/// kit (Guard, Dodge Roll) is always in.
pub fn kh1_bits(equipped: Bits) -> u32 {
    ALL.iter().enumerate().filter(|(i, _)| (equipped | KIT_BITS) & (1 << i) != 0).fold(0, |b, (_, a)| b | a.kh1_bit)
}

/// The equipped combo modifiers into the abilities the KH1 pickers read (the rest untouched).
pub fn apply(equipped: Bits, a: &mut kh1_core::Abilities) {
    let on = |i: usize| equipped & (1 << i) != 0;
    a.sliding_dash = on(ix::SLIDING_DASH);
    a.blitz = on(ix::BLITZ);
    a.stun_impact = on(ix::STUN_IMPACT);
    a.zantetsuken = on(ix::ZANTETSUKEN);
    a.vortex = on(ix::VORTEX);
    a.aerial_sweep = on(ix::AERIAL_SWEEP);
    a.hurricane_blast = on(ix::HURRICANE_BLAST);
    a.ripple_drive = on(ix::RIPPLE_DRIVE);
    a.gravity_break = on(ix::GRAVITY_BREAK);
}

/// Every equipped ability as the abilities the KH1 frame passes on (handlers and pickers).
pub fn to_core(equipped: Bits) -> kh1_core::Abilities {
    let on = |i: usize| equipped & (1 << i) != 0;
    let count = |v: &[usize]| v.iter().filter(|&&i| on(i)).count() as u8;
    let mut a = kh1_core::Abilities {
        high_jump: on(ix::HIGH_JUMP),
        glide: on(ix::GLIDE),
        superglide: on(ix::SUPERGLIDE),
        // Sora's movement kit (owner, 2026-11-01): always on at Combat = KH1.
        dodge_roll: true,
        guard: true,
        counterattack: on(ix::COUNTERATTACK),
        combo_master: on(ix::COMBO_MASTER),
        combo_plus: count(&ix::COMBO_PLUS),
        air_combo_plus: count(&ix::AIR_COMBO_PLUS),
        // Enemy Skills stage 2: Bandit's Combo Finish (`crate::skill_effects`).
        combo_finish: crate::skill_effects::play(equipped).combo_finish,
        ..Default::default()
    };
    apply(equipped, &mut a);
    a
}

// ---- M12 step 3: KH1's support abilities (KH1's actor ability block `+0x184`, `0x1402A6380`) ----

/// MP Haste equipped (KH1 `+0x184` bit 0 = `0x140284BE0(23) & 1`): the hit charge's factor + G
/// `+0x95` / 100 = + 1.5 (`0x1402BF330`). Checked (decomp + files).
pub fn mp_haste(equipped: Bits) -> bool {
    equipped & (1 << ix::MP_HASTE) != 0
}

/// The equipped MP Rage copies (KH1 `+0x184` bits 1..3 = `0x140284BE0(24) & 7`: every equipped
/// copy counts): the charge from damage taken is HP lost × this (`0x14029BBD0`). Checked (decomp).
pub fn mp_rage(equipped: Bits) -> u8 {
    ix::MP_RAGE.iter().filter(|&&i| equipped & (1 << i) != 0).count() as u8
}

/// Leaf Bracer equipped (KH1 `+0x184` bit 6 = `0x140284BE0(62) & 1`): a Cure Sora casts on himself
/// sets his "no damage, no reaction" flag 0x80 for the cast (`0x14028F140` → `0x140297F30(actor,
/// 0)`). Checked (decomp).
pub fn leaf_bracer(equipped: Bits) -> bool {
    equipped & (1 << ix::LEAF_BRACER) != 0
}

/// Berserk equipped (KH1 `+0x184` bit 5 = `0x140284BE0(26) & 1`): Strength-source attacks get G
/// `+0xFE` = 4 more Strength while the gauge is in its low-HP state (`0x1402B2DC0`,
/// `0x14026E180`). Checked (decomp + files).
pub fn berserk(equipped: Bits) -> bool {
    equipped & (1 << ix::BERSERK) != 0
}

/// Second Chance equipped (KH1 `+0x184` bit 4 = `0x140284BE0(25) & 1`): the damage `0x1402BFA80`
/// leaves 1 HP when a hit would kill from above 1. Checked (decomp).
pub fn second_chance(equipped: Bits) -> bool {
    equipped & (1 << ix::SECOND_CHANCE) != 0
}

/// The names of the abilities in `bits`, for the log.
pub fn names(bits: Bits) -> Vec<&'static str> {
    ALL.iter().enumerate().filter(|(i, _)| bits & (1 << i) != 0).map(|(_, a)| a.name).collect()
}

/// Whether an ability is on the equip screen: not the specials still waiting (stage 9c), not
/// Sora's movement kit (owner, 2026-11-01).
pub fn on_screen(i: usize) -> bool {
    i < ALL.len() && (SPECIALS_WAITING | KIT_BITS) & (1 << i) == 0
}

// ---- the equip screen (the KH1 camp entry's "Abilities" row) ----

/// KH1's ability screen (Steam, Checked (decomp)): `0x140310B60` the owner list (each party member,
/// then "Shared Abilities"), `0x140311C20` Sora's list, `0x1403120F0` the shared list (rows: icon,
/// name, cost; greyed when unequipped and the cost is over the AP left), `0x140310650` the command
/// window on a confirmed ability (Equip / Remove, Move, Sort): Equip refused with error sound 4 and
/// message 0xCA when the AP is short, else the bit 7 cleared; Remove sets it; a shared ability
/// toggles with no AP test; `0x1403108E0` the panel: AP left / max AP.
pub mod page {
    use super::{ap_left, built, can_equip, on_screen, used_ap, Source, State, ALL};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Owner {
        Sora,
        Shared,
        /// Enemy Skills (owner, 2026-11-03: "let the enemy cards have their own separate section
        /// in the ability tabs called "Enemy Skills""): every [`Source::Card`] ability.
        EnemySkills,
        /// Sleights (owner, 2026-11-04: "I think we should have also have "sleights" have its own
        /// menu in the ability list"): the 27 sleight abilities ([`super::SLEIGHTS`]); KH1's
        /// specials (Sonic Blade, Strike Raid, Ars Arcanum, Ragnarok) stay Sora's.
        Sleights,
    }

    /// The owner rows: Sora, Shared, Enemy Skills, then Sleights (last, so the rows before it keep
    /// their places).
    pub const OWNERS: [Owner; 4] = [Owner::Sora, Owner::Shared, Owner::EnemySkills, Owner::Sleights];

    /// The owner row ability `i` is listed under.
    pub fn owner_of(i: usize) -> Owner {
        if super::SLEIGHTS.contains(&i) {
            return Owner::Sleights;
        }
        match ALL[i] {
            super::Ability { source: Source::Card(_), .. } => Owner::EnemySkills,
            super::Ability { shared: true, .. } => Owner::Shared,
            _ => Owner::Sora,
        }
    }

    /// The order within an owner's list: KH1's sort key; Enemy Skills by card kind (Re:CoM's card
    /// order).
    fn sort_key(i: usize) -> u16 {
        match ALL[i].source {
            Source::Card(c) => c,
            _ => ALL[i].key,
        }
    }

    /// Every row of the page (owners and abilities) is Re:CoM's green row with the green Mickey
    /// symbol: list entry 0x131 through its row set-up `0x140112F10` (owner, 2026-11-01: "have the
    /// abilities be green with the green mickey symbol"; before, the blue magic row 0x133).
    pub const ROW_LOOK: i32 = crate::keyblades::row_look::GREEN;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Label {
        Owner(Owner),
        Ability(usize),
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Row {
        pub label: Label,
        /// KH1's greyed row: unequipped, and the cost is over the AP left.
        pub grey: bool,
        /// Equipped (Re:CoM's row mark).
        pub marked: bool,
    }

    /// The owner's unlocked abilities in KH1's sort order (its Sort option 0, `0x140310260`: by
    /// the record's key; ours keeps that order fixed, ties by our index).
    pub fn list(owner: Owner, s: &State) -> Vec<usize> {
        let mut v: Vec<usize> = (0..ALL.len()).filter(|&i| on_screen(i) && s.unlocked & (1 << i) != 0 && owner_of(i) == owner).collect();
        v.sort_by_key(|&i| (sort_key(i), i));
        v
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Outcome {
        Nothing,
        /// The owner's list opened (sound 2).
        Opened,
        /// An owner with no abilities: nothing to open (sound 5).
        Empty,
        Equipped(usize),
        Removed(usize),
        /// Not enough AP (KH1's message 0xCA; error sound).
        Refused(usize),
        /// Its effect isn't built yet ([`super::built`]): never equipped (error sound).
        NotBuilt(usize),
        /// Back to the owner list (sound 4).
        Back,
        /// Close the page.
        Close,
    }

    /// The screen: the owner list, or (`owner`) that owner's abilities.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Screen {
        pub owner: Option<usize>,
        pub owner_cursor: i32,
        pub cursor: i32,
    }

    impl Screen {
        pub fn rows(&self, s: &State) -> Vec<Row> {
            match self.owner {
                None => OWNERS.iter().map(|&o| Row { label: Label::Owner(o), grey: list(o, s).is_empty(), marked: false }).collect(),
                Some(o) => list(OWNERS[o], s)
                    .into_iter()
                    .map(|i| {
                        let on = s.equipped & (1 << i) != 0;
                        Row { label: Label::Ability(i), grey: !on && (!built(i) || !can_equip(s.equipped, i, s.max_ap())), marked: on }
                    })
                    .collect(),
            }
        }

        /// The window's depth: 0 the owner list, 1 an owner's abilities (the page keeps each
        /// window's scroll by it, `crate::keyblades::list::Scroll`).
        pub fn depth(&self) -> usize {
            usize::from(self.owner.is_some())
        }

        /// The list's cursor (the owner list's or the ability list's).
        pub fn cursor(&self) -> i32 {
            if self.owner.is_some() { self.cursor } else { self.owner_cursor }
        }

        pub fn set_cursor(&mut self, c: i32) {
            if self.owner.is_some() {
                self.cursor = c;
            } else {
                self.owner_cursor = c;
            }
        }

        /// The highlighted ability.
        pub fn ability(&self, s: &State) -> Option<usize> {
            let o = self.owner?;
            list(OWNERS[o], s).get(self.cursor.max(0) as usize).copied()
        }

        /// Confirm: open an owner's list (on its first row), or toggle the highlighted ability
        /// with KH1's rule. `s` is changed in place; the caller writes it.
        pub fn confirm(&mut self, s: &mut State) -> Outcome {
            match self.owner {
                None => {
                    let o = self.owner_cursor.clamp(0, OWNERS.len() as i32 - 1) as usize;
                    if list(OWNERS[o], s).is_empty() {
                        return Outcome::Empty;
                    }
                    self.owner = Some(o);
                    self.cursor = 0;
                    Outcome::Opened
                }
                Some(_) => {
                    let Some(i) = self.ability(s) else { return Outcome::Nothing };
                    let bit = 1 << i;
                    if s.equipped & bit != 0 {
                        s.equipped &= !bit;
                        Outcome::Removed(i)
                    } else if !built(i) {
                        Outcome::NotBuilt(i)
                    } else if can_equip(s.equipped, i, s.max_ap()) {
                        s.equipped |= bit;
                        Outcome::Equipped(i)
                    } else {
                        Outcome::Refused(i)
                    }
                }
            }
        }

        /// Cancel: the ability list goes back to the owner list; the owner list closes the page.
        pub fn cancel(&mut self) -> Outcome {
            if self.owner.take().is_some() {
                Outcome::Back
            } else {
                Outcome::Close
            }
        }

        /// The status panel's values: AP (KH1's panel: AP left / max AP), the highlighted
        /// ability's cost, the AP used / max AP.
        pub fn status(&self, s: &State) -> [String; 3] {
            let max = s.max_ap();
            let cost = self.ability(s).map_or_else(|| "-".to_string(), |i| ALL[i].ap.to_string());
            [format!("{}/{max}", ap_left(s.equipped, max)), cost, format!("{}/{max}", used_ap(s.equipped))]
        }
    }

    /// Texts: NUL-terminated ASCII for Re:CoM's text items (no `%`). KH1's own words (sysmsg) are
    /// not in the asset step: ours, in English (Assumption).
    pub mod text {
        pub const LABEL: &[u8] = b"Abilities\0";
        pub const HELP: &[u8] = b"Equip Sora's abilities with AP.\0";
        pub const SORA: &[u8] = b"Sora\0";
        pub const SHARED: &[u8] = b"Shared\0";
        /// Enemy Skills (owner's words).
        pub const ENEMY_SKILLS: &[u8] = b"Enemy Skills\0";
        /// Sleights (the owner's word, the command menu's "Sleights").
        pub const SLEIGHTS: &[u8] = b"Sleights\0";
        pub const STATUS: [&[u8]; 3] = [b"AP\0", b"Cost\0", b"AP used\0"];
        /// By [`super::OWNERS`].
        pub const OWNER_HELP: [&[u8]; 4] = [b"Sora's abilities. Equipping one uses AP.\0", b"Abilities for movement. They use no AP.\0", b"Skills from enemy cards. Equipping one uses AP.\0", b"Sleights to use from the command menu. Equipping one uses AP.\0"];
        pub const EMPTY_HELP: &[u8] = b"No abilities learned yet.\0";
        /// KH1's message 0xCA (not read).
        pub const NO_AP: &[u8] = b"Not enough AP to equip this ability.\0";
        /// A greyed row whose effect isn't built yet ([`super::super::built`]).
        pub const NOT_BUILT: &[u8] = b"Not built yet: this can't be equipped.\0";

        /// The sleight abilities' help lines (owner, 2026-11-03, "Sleight ability texts": "a short
        /// description for each sleight's help line (what it does + its MP cost)"): what each does
        /// in Re:CoM, then its MP (the owner's tier, [`super::super::sleight_mp`]). Ours, in
        /// English: Re:CoM's own descriptions are its messages, not in the session. What each line
        /// rests on (Re:CoM's `StockSkillList` element, its attack row's hit kind and what Re:CoM's
        /// hit handler `0x14017DC50` does with it; the rest Assumption (wiki)):
        /// `docs/recom-m12-sleights.md` § 8.4.1. By Re:CoM id.
        pub const SLEIGHT_HELP: [(u16, &[u8]); 27] = [
            (0x1B, b"Hurl the Keyblade over and over, wreathed in fire. Uses 2 MP.\0"),
            (0x1C, b"Hurl the Keyblade over and over, wrapped in ice. Uses 2 MP.\0"),
            (0x1D, b"Hurl the Keyblade over and over, charged with thunder. Uses 2 MP.\0"),
            (0x1E, b"Inside a barrier, bounce from enemy to enemy. Uses 2 MP.\0"),
            (0x1F, b"A rapid flurry of Keyblade blows on enemies. Uses 1 MP.\0"),
            (0x20, b"A ring of fire bursts out around Sora. Uses 2 MP.\0"),
            (0x21, b"Flames whirl around Sora as he moves. Uses 2 MP.\0"),
            (0x22, b"A huge blast of fire over the area. Uses 3 MP.\0"),
            (0x23, b"Freezes the enemies around Sora. Ice. Some resist. Uses 2 MP.\0"),
            (0x24, b"Ice shots that home in on enemies. Uses 2 MP.\0"),
            (0x25, b"Water bursts out around Sora. Ice. Uses 2 MP.\0"),
            (0x26, b"Pulls enemies in and holds them. Some resist. Uses 2 MP.\0"),
            (0x27, b"Stops enemies in time while Sora strikes them. Uses 2 MP.\0"),
            (0x28, b"Leap up and crash down with a shock wave. Uses 2 MP.\0"),
            (0x29, b"A tornado that catches enemies and hits many times. Uses 2 MP.\0"),
            (0x2A, b"Shakes the ground: hits only enemies standing on it. Uses 2 MP.\0"),
            (0x2B, b"A strike that warps an enemy away half the time. Uses 1 MP.\0"),
            (0x2C, b"Warps enemies out of the battle. Some resist. Uses 1 MP.\0"),
            (0x2D, b"Binds enemies on the ground in place. Some resist. Uses 1 MP.\0"),
            (0x2E, b"Confuses enemies. Some resist. Uses 1 MP.\0"),
            (0x2F, b"Frightens enemies. Some resist. Uses 1 MP.\0"),
            (0x30, b"Gives other enemies the HP of the one it hits. Uses 1 MP.\0"),
            (0x31, b"A shower of light that restores Sora's HP. Uses 1 MP.\0"),
            (0x32, b"Stops the enemies it hits for a long time. Some resist. Uses 1 MP.\0"),
            (0x33, b"Orbs of holy light strike enemies all around. Uses 3 MP.\0"),
            (0x3A, b"Romp with Bambi; enemies hit drop HP prizes. Uses all MP.\0"),
            (0x43, b"Cloud's Cross-slash: three powerful slashes. Uses all MP.\0"),
        ];

        /// The help line of sleight `id` (one of the 27).
        pub fn sleight_help(id: u16) -> Option<&'static [u8]> {
            SLEIGHT_HELP.iter().find(|(s, _)| *s == id).map(|(_, t)| *t)
        }

        /// The ability's help line (KH1 shows its own description; ours).
        pub fn help(i: usize) -> &'static [u8] {
            use super::super::ix::*;
            // Enemy Skills: the skill's own line (`crate::enemy_skills`; the five KH1 matches too).
            if let Some(s) = super::super::skill_of(i) {
                return s.help;
            }
            match i {
                SLIDING_DASH => b"Dash at a distant locked-on enemy.\0",
                BLITZ => b"A rising strike to end a combo.\0",
                STUN_IMPACT => b"A combo finisher that stuns enemies nearby.\0",
                ZANTETSUKEN => b"A combo finisher that can fell an enemy at once.\0",
                VORTEX => b"Close in on a nearby enemy with a spinning slash.\0",
                AERIAL_SWEEP => b"Leap up and strike an enemy in the air.\0",
                HURRICANE_BLAST => b"A spinning finisher for an air combo.\0",
                RIPPLE_DRIVE => b"A combo finisher that strikes all around. Uses Magic.\0",
                GRAVITY_BREAK => b"A combo finisher that cuts an enemy's HP.\0",
                SONIC_BLADE => b"A flurry of dashing strikes. Triangle at a nearby enemy.\0",
                STRIKE_RAID => b"Hurl the Keyblade at an enemy, up to five times. Triangle.\0",
                ARS_ARCANUM => b"A furious flurry of strikes on one enemy. Triangle.\0",
                RAGNAROK => b"Rising strikes, then a burst of light. Triangle at a nearby enemy.\0",
                GUARD => b"Press Square to block an attack.\0",
                COUNTERATTACK => b"After a block, press Triangle to strike back.\0",
                DODGE_ROLL => b"Press Square while moving to roll.\0",
                HIGH_JUMP => b"Jump higher.\0",
                GLIDE => b"Hold the jump button in the air to glide.\0",
                SUPERGLIDE => b"Press Square in the air to glide at speed.\0",
                COMBO_MASTER => b"Keep a combo going even when a swing misses.\0",
                20..=22 => b"One more swing in a ground combo.\0",
                23..=24 => b"One more swing in an air combo.\0",
                MP_HASTE => b"Gain more MP from each hit on an enemy.\0",
                26..=27 => b"Gain MP when you take damage. Copies add up.\0",
                LEAF_BRACER => b"Nothing can interrupt Sora while he casts Cure.\0",
                BERSERK => b"Raises Strength while Sora's HP is low.\0",
                SECOND_CHANCE => b"Survive a blow that would knock you out, with 1 HP.\0",
                // KH1's level-72 copy (the level-9 one is Bouncywild's card).
                i if i == TREASURE_MAGNET[1] => crate::skill_effects::TREASURE_MAGNET_HELP,
                // M12 step 4: the sleight abilities (Re:CoM's sleights; the owner's MP costs).
                i if super::super::SLEIGHTS.contains(&i) => super::super::sleight_of(i).and_then(sleight_help).unwrap_or(b"\0"),
                _ => b"\0",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::page::{Label, Outcome, Owner, Screen};
    use super::*;

    const NO_EXT: [u8; EXT_LEN] = [0; EXT_LEN];
    const NO_SK: [u8; SKILL_EXT_LEN] = [0; SKILL_EXT_LEN];

    #[test]
    fn block_round_trip_and_new_game() {
        let mut b = [0u8; BLOCK_LEN];
        assert_eq!(read(&b, &NO_EXT, &NO_SK), State::default(), "a block never written: nothing unlocked");
        let s = State { unlocked: 0b1_0101, equipped: 0b101, ap_ups: 7 };
        write(&s, &mut b, &mut [0; EXT_LEN], &mut [0; SKILL_EXT_LEN]);
        assert_eq!(b[field::VERSION], BLOCK_V7);
        assert_eq!(b[field::AP_UPS], 7);
        assert_eq!(read(&b, &NO_EXT, &NO_SK), s);
        // Equipped is only ever a subset of unlocked; unknown bits (and the kit) are dropped.
        b[field::EQUIPPED..field::EQUIPPED + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        b[field::UNLOCKED..field::UNLOCKED + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(read(&b, &NO_EXT, &NO_SK), State { unlocked: 0xFFFF_FFFF & !KIT_BITS, equipped: 0xFFFF_FFFF & !KIT_BITS, ap_ups: 7 });
        let low64: Bits = u64::MAX as Bits;
        assert_eq!(read(&b, &[0xFF; EXT_LEN], &NO_SK), State { unlocked: ALL_BITS & low64 & !KIT_BITS, equipped: ALL_BITS & low64 & !KIT_BITS, ap_ups: 7 }, "the skill extension never written: nothing past bit 63");
        let mut sk = [0xFF; SKILL_EXT_LEN];
        sk[skill_ext::VERSION] = SKILL_EXT_V1;
        assert_eq!(read(&b, &[0xFF; EXT_LEN], &sk), State { unlocked: ALL_BITS & !KIT_BITS, equipped: ALL_BITS & !KIT_BITS, ap_ups: 7 });
        assert!(b[field::RESERVED..].iter().all(|&x| x == 0));
        // A v1 block reads as before (its bits are a subset of v2's).
        let mut v1 = [0u8; BLOCK_LEN];
        v1[field::VERSION] = BLOCK_V1;
        v1[field::UNLOCKED] = 0x81;
        v1[field::EQUIPPED] = 0x81;
        assert_eq!(read(&v1, &NO_EXT, &NO_SK), State { unlocked: 0x81, equipped: 0x81, ap_ups: 0 });
        // Before v5 the count's byte is a reserved zero byte: it reads as no AP Up yet.
        let mut v4 = v1;
        v4[field::VERSION] = BLOCK_V4;
        v4[field::AP_UPS] = 9;
        assert_eq!(read(&v4, &NO_EXT, &NO_SK).ap_ups, 0);
    }

    #[test]
    fn layout_follows_the_items_extension() {
        assert_eq!(BLOCK_FIRST, 0x269F);
        assert_eq!(BLOCK_LAST, 0x26AD);
        // 58 before Enemy Skills; Treasure Magnet's two copies; the 43 skills of their own.
        assert_eq!(ALL.len(), 103);
        assert_eq!(ALL_BITS, (1 << 103) - 1);
        // M12 step 4: the words' extension right after the summon block (save+0x26AE..=0x26B6).
        assert_eq!(crate::summons::BLOCK_LAST, 0x26B6);
        assert_eq!((EXT_FIRST, EXT_LAST), (0x26B7, 0x26BE));
        assert!(EXT_LAST <= crate::keyblades::SPARE_LAST);
        // Enemy Skills: the skill extension right after it (save+0x26BF..=0x26CF).
        assert_eq!((SKILL_EXT_FIRST, SKILL_EXT_LAST), (0x26BF, 0x26CF));
        assert!(SKILL_EXT_LAST <= crate::keyblades::SPARE_LAST);
    }

    /// Layout v7 (Enemy Skills): bits 64.. in the skill extension, read only from a v7 block whose
    /// extension's version byte is set; a v6 block's words hold nothing past its 58 abilities.
    #[test]
    fn skill_extension_round_trip() {
        let top = crate::enemy_skills::ability_of(crate::enemy_skills::of_card(0x7E).unwrap());
        assert!(top >= 64, "Platinum Card's skill is in the extension");
        let s = State { unlocked: 1 << top | 1 << ix::COMBO_PLUS[0] | 1 << ix::TREASURE_MAGNET[0] | 1 << 40, equipped: 1 << ix::COMBO_PLUS[0] | 1 << 40, ap_ups: 2 };
        let (mut b, mut e, mut k) = ([0u8; BLOCK_LEN], [0u8; EXT_LEN], [0u8; SKILL_EXT_LEN]);
        write(&s, &mut b, &mut e, &mut k);
        assert_eq!((b[field::VERSION], k[skill_ext::VERSION]), (BLOCK_V7, SKILL_EXT_V1));
        assert_eq!(read(&b, &e, &k), s);
        assert_eq!(u64_at(&k, skill_ext::UNLOCKED_TOP), 1 << (top - 64));
        assert_eq!(u64_at(&k, skill_ext::EQUIPPED_TOP), 0);
        // Its version byte cleared (an older DLL never writes it): the top bits mean nothing.
        let mut k0 = k;
        k0[skill_ext::VERSION] = 0;
        assert_eq!(read(&b, &e, &k0).unlocked, s.unlocked & u64::MAX as Bits);
        // A v6 block: the extension and bits 58.. mean nothing.
        let mut b6 = b;
        b6[field::VERSION] = BLOCK_V6;
        assert_eq!(raw(&b6, &[0xFF; EXT_LEN], &[0xFF; SKILL_EXT_LEN]).unlocked, s.unlocked & V6_BITS | (V6_BITS & !(u32::MAX as Bits)));
    }

    /// Owner (M12, 2026-11-03): KH1's start 3, +2 per AP Up pick, no cap, nothing else (the
    /// level gives nothing).
    #[test]
    fn max_ap_is_kh1s_start_plus_two_per_ap_up() {
        assert_eq!(max_ap(0), 3);
        assert_eq!(max_ap(1), 5);
        assert_eq!(max_ap(10), 23);
        assert_eq!(max_ap(98), 199, "a pick at every level-up: no cap at KH1's 100");
        assert_eq!(max_ap(u32::MAX), i32::MAX, "saturates");
        let s = State { ap_ups: 4, ..Default::default() };
        assert_eq!(s.max_ap(), 11);
        assert_eq!(State::default().max_ap(), START_AP);
    }

    /// Old saves (owner: "each CP Up already taken counts as an AP Up"): Re:CoM's CP is 275 at a
    /// new game and only its +25 picks raise it.
    #[test]
    fn past_cp_picks_become_ap_ups() {
        assert_eq!(ap_ups_from_cp(275, 1), 0, "a new game");
        assert_eq!(ap_ups_from_cp(275, 30), 0, "every pick was HP or a sleight");
        assert_eq!(ap_ups_from_cp(300, 2), 1);
        assert_eq!(ap_ups_from_cp(525, 30), 10);
        assert_eq!(ap_ups_from_cp(530, 30), 10, "a remainder (not Re:CoM's) rounds down");
        assert_eq!(ap_ups_from_cp(1000, 5), 4, "at most one pick per level-up");
        assert_eq!(ap_ups_from_cp(100, 40), 0, "below the start CP: none");
        assert_eq!(ap_ups_from_cp(275 + 25 * 98, 99), 98);
        assert_eq!(ap_ups_from_cp(i32::MAX, 99), 98);
        assert_eq!(ap_ups_from_cp(i32::MIN, 99), 0);
        assert_eq!(ap_ups_from_cp(400, 0), 0);
    }

    /// The btltbl values, read from the player's file when the test data is there.
    #[test]
    fn ap_tables_match_btltbl() {
        let Ok(dir) = std::env::var("KH1_TEST_DATA") else { return };
        let Ok(b) = std::fs::read(std::path::Path::new(&dir).join("btltbl.bin")) else { return };
        assert_eq!(b[0x39D8 + 2] as i32 + b[0x128 + 4 * 5 + 3] as i8 as i32, START_AP);
        // KH1's own AP bonus (kind 3 of the bonus list btltbl `0x3AC0`) is +2 a time, as AP Up.
        assert!((2..=100usize).any(|l| b[0x3AC0 + l - 2] == 3));
        for a in ALL.iter().filter(|a| a.kh1_id.is_some_and(|id| id < 64)) {
            let row = 0x33D8 + 12 * (a.kh1_id.unwrap() as usize - 1);
            assert_eq!((b[row], u16::from_le_bytes([b[row + 2], b[row + 3]])), (a.ap, a.key), "{}", a.name);
        }
        // Combo Master (0x41) has no row.
        assert!(b[0x33D8 + 12 * 64..0x33D8 + 12 * 65].iter().all(|&x| x == 0));
        // Sonic Blade's row (stage 9c.1): cost 3, key 801.
        let row = 0x33D8 + 12 * 10;
        assert_eq!((b[row], u16::from_le_bytes([b[row + 2], b[row + 3]])), (3, 801));
        // The Rod list's levels for every Level source but the owner's (Gravity Break 30, Combo
        // Master 50).
        let rod = |id: u8| -> Vec<u32> { (0..99).filter(|&i| b[0x3BF8 + i] == 0x80 | id).map(|i| i as u32 + 2).collect() };
        for (i, a) in ALL.iter().enumerate() {
            if let (Source::Level(l), Some(id)) = (a.source, a.kh1_id) {
                if i != ix::GRAVITY_BREAK && i != ix::COMBO_MASTER {
                    assert!(rod(id).contains(&l), "{} at {l}: {:?}", a.name, rod(id));
                }
            }
        }
        assert_eq!(rod(6), vec![87, 93, 99]);
        assert_eq!(rod(21), vec![33], "KH1's Guard level (the owner made it Sora's kit)");
        assert_eq!(rod(7), vec![24, 45]);
        for id in [1, 3, 4, 22, 58, 59] {
            assert!(rod(id).is_empty(), "id {id}");
        }
    }

    #[test]
    fn costs_keys_and_bits() {
        let ap: Vec<(u8, u8)> = ALL[MODIFIERS].iter().map(|a| (a.kh1_id.unwrap(), a.ap)).collect();
        assert_eq!(ap, vec![(54, 1), (20, 2), (57, 2), (59, 2), (17, 1), (18, 2), (55, 2), (56, 3), (58, 2)]);
        let x = |i: usize| (ALL[i].kh1_id.unwrap(), ALL[i].ap, ALL[i].key, ALL[i].kh1_bit, ALL[i].shared);
        assert_eq!(x(ix::GUARD), (21, 2, 101, 0x10, false));
        assert_eq!(x(ix::COUNTERATTACK), (19, 2, 102, 0x4, false));
        assert_eq!(x(ix::DODGE_ROLL), (22, 1, 100, 0x20, false));
        assert_eq!(x(ix::HIGH_JUMP), (1, 0, 1, 0x200, true));
        assert_eq!(x(ix::GLIDE), (3, 0, 3, 0x80, true));
        assert_eq!(x(ix::SUPERGLIDE), (4, 0, 4, 0x100, true));
        assert_eq!(x(ix::COMBO_MASTER), (0x41, 0, 0, 0, false));
        // Stage 9c.1: Sonic Blade, KH1 id 11, 3 AP, sort key 801.
        assert_eq!(x(ix::SONIC_BLADE), (11, 3, 801, 0, false));
        // The movement bits as `settings::Kh1Abilities::bits` has them; the kit is always in.
        let all_move = 1 << ix::HIGH_JUMP | 1 << ix::GLIDE | 1 << ix::SUPERGLIDE | 1 << ix::COUNTERATTACK;
        let ini = crate::settings::Kh1Abilities { high_jump: true, glide: true, superglide: true, dodge_roll: true, guard: true, counterattack: true, combo_plus: 0, air_combo_plus: 0, combo_master: false };
        assert_eq!(kh1_bits(all_move), ini.bits());
        assert_eq!(kh1_bits(MODIFIER_BITS), 0x3F00_0000 | 0x8 | 0x1 | 0x2 | 0x30);
        assert_eq!(kh1_bits(1 << ix::RAGNAROK), 0x30, "Guard and Dodge Roll: Sora's kit");
    }

    #[test]
    fn learn_levels_are_kh1s_rod_list_and_the_owners() {
        let lv = |i: usize| match ALL[i].source {
            Source::Level(l) => l,
            _ => 0,
        };
        assert_eq!([lv(ix::RIPPLE_DRIVE), lv(ix::HURRICANE_BLAST), lv(ix::AERIAL_SWEEP), lv(ix::VORTEX)], [12, 18, 36, 84]);
        assert_eq!(lv(ix::GRAVITY_BREAK), 30, "owner, 2026-11-01");
        assert_eq!(lv(ix::COMBO_MASTER), 50, "owner, 2026-11-01");
        assert_eq!(lv(ix::COUNTERATTACK), 57, "KH1's Rod list");
        assert_eq!((ALL[ix::GUARD].source, ALL[ix::DODGE_ROLL].source), (Source::Kit, Source::Kit), "owner: Sora's movement kit, not abilities");
        assert_eq!(ALL[ix::SONIC_BLADE].source, Source::Sleight(sleight::SONIC_BLADE), "Re:CoM's sleight 9 (level 27)");
        // Enemy Skills (owner): the Rod level-87 copy comes from the Soldier card; 93 and 99 by level.
        assert_eq!(ix::COMBO_PLUS.map(lv), [0, 93, 99]);
        assert_eq!(ALL[ix::COMBO_PLUS[0]].source, Source::Card(0x45));
        assert_eq!(ix::AIR_COMBO_PLUS.map(lv), [24, 45]);
        // Treasure Magnet: KH1's level-9 copy from the Bouncywild card, the level-72 one by level.
        assert_eq!(ALL[ix::TREASURE_MAGNET[0]].source, Source::Card(0x4C));
        assert_eq!(lv(ix::TREASURE_MAGNET[1]), 72);
        // Re:CoM's own movement abilities (its variable (0, 0x24)), equipped when it gives them.
        assert_eq!(ALL[ix::HIGH_JUMP].source, Source::Recom(1));
        assert_eq!(ALL[ix::GLIDE].source, Source::Recom(2 | 4), "Re:CoM glides with either bit");
        assert_eq!(ALL[ix::SUPERGLIDE].source, Source::Recom(4));
        assert_eq!(arrive_equipped_bits(), 1 << ix::HIGH_JUMP | 1 << ix::GLIDE | 1 << ix::SUPERGLIDE);
    }

    const NO_RECOM: u32 = 0;
    const MOVE: Bits = 1 << ix::HIGH_JUMP | 1 << ix::GLIDE | 1 << ix::SUPERGLIDE;

    /// Owner (2026-11-01): Guard and Dodge Roll are Sora's kit: always on, no AP, never in the block.
    #[test]
    fn guard_and_dodge_roll_are_sora_s_kit() {
        let mut s = State::default();
        let r = sync(&mut s, 99, 7, &mut |_| true);
        assert_eq!(s.unlocked & KIT_BITS, 0, "never unlocked in the block");
        assert_eq!(r.new & KIT_BITS, 0);
        let c = to_core(0);
        assert!(c.guard && c.dodge_roll, "always on");
        assert!(!on_screen(ix::GUARD) && !on_screen(ix::DODGE_ROLL));
        assert_eq!(used_ap(KIT_BITS), 0, "no AP");
        // A block that still holds them reads without them.
        let mut b = [0u8; BLOCK_LEN];
        b[field::VERSION] = BLOCK_V3;
        b[field::UNLOCKED..field::UNLOCKED + 4].copy_from_slice(&((KIT_BITS | 1) as u32).to_le_bytes());
        b[field::EQUIPPED..field::EQUIPPED + 4].copy_from_slice(&((KIT_BITS | 1) as u32).to_le_bytes());
        assert_eq!(read(&b, &NO_EXT, &NO_SK), State { unlocked: 1, equipped: 1, ap_ups: 0 });
    }

    #[test]
    fn new_unlocks_arrive_unequipped() {
        let mut s = State::default();
        // A new game: nothing in the block (Guard and Dodge Roll are the kit).
        assert_eq!(sync(&mut s, 1, NO_RECOM, &mut |_| false), Synced::default());
        assert_eq!(s, State::default());
        // Re:CoM's Lv2 sleight and level 12: Sliding Dash, Ripple Drive, unequipped (KH1).
        let r = sync(&mut s, 12, NO_RECOM, &mut |id| id == sleight::SLIDING_DASH);
        assert_eq!(names(r.new), vec!["Sliding Dash", "Ripple Drive"]);
        assert_eq!(s.equipped, 0);
        // Only abilities not yet unlocked are asked about; never taken back.
        let mut asked = Vec::new();
        sync(&mut s, 12, NO_RECOM, &mut |id| {
            asked.push(id);
            false
        });
        assert!(!asked.contains(&sleight::SLIDING_DASH));
        sync(&mut s, 1, NO_RECOM, &mut |_| false);
        assert_eq!(names(s.unlocked), vec!["Sliding Dash", "Ripple Drive"]);
        // Level 30: Gravity Break (owner), Hurricane Blast 18, Air Combo Plus 24, MP Haste 21 (M12).
        let r = sync(&mut s, 30, NO_RECOM, &mut |_| false);
        assert_eq!(names(r.new), vec!["Hurricane Blast", "Gravity Break", "Air Combo Plus", "MP Haste"]);
        // Re:CoM's Sonic Blade sleight (9): Sonic Blade, unequipped (KH1's rule).
        let r = sync(&mut s, 30, NO_RECOM, &mut |id| id == sleight::SONIC_BLADE);
        assert_eq!(names(r.new), vec!["Sonic Blade"]);
        // Level 50: Combo Master (owner), unequipped; MP Rage 42 (M12). Leaf Bracer no longer comes
        // at 39: the Creeper Plant card gives it (Enemy Skills).
        let r = sync(&mut s, 50, NO_RECOM, &mut |_| false);
        assert_eq!(names(r.new), vec!["Aerial Sweep", "Combo Master", "Air Combo Plus", "MP Rage"]);
        // Level 99 and every sleight: everything but Re:CoM's movement abilities, the kit and the
        // Enemy Skills (no card yet); Treasure Magnet's level-72 copy among them.
        let r = sync(&mut s, 99, NO_RECOM, &mut |_| true);
        assert_eq!(s.unlocked, ALL_BITS & !MOVE & !KIT_BITS & !CARD_BITS);
        assert!(names(r.new).contains(&"Treasure Magnet"));
        // Every enemy card: every Enemy Skill, unequipped.
        let r = sync_with(&mut s, 99, NO_RECOM, &mut |_| true, &mut |_| true);
        assert_eq!(r.new, CARD_BITS);
        assert_eq!(s.unlocked, ALL_BITS & !MOVE & !KIT_BITS);
        assert_eq!(s.equipped, 0);
    }

    /// Enemy Skills (owner): a skill unlocks when its card was ever obtained (Re:CoM's discovery
    /// state), unequipped; the four KH1 matches built before replace their level grants; the page
    /// lists them all under "Enemy Skills" by card, the unbuilt ones greyed and refused.
    #[test]
    fn enemy_skills_learn_from_their_cards() {
        let skill = |c: u16| crate::enemy_skills::ability_of(crate::enemy_skills::of_card(c).unwrap());
        let mut s = State { ap_ups: 10, ..Default::default() };
        // Level 99 without the cards: none of the four matches (nor Soldier's Combo Plus).
        sync(&mut s, 99, NO_RECOM, &mut |_| false);
        assert_eq!(s.unlocked & CARD_BITS, 0);
        assert_eq!(to_core(s.unlocked).combo_plus, 2, "only the level copies (93, 99)");
        // The cards Sora has: Hook, Gold Card, Creeper Plant (and Shadow, which has no skill).
        let mut asked = Vec::new();
        let r = sync_with(&mut s, 99, NO_RECOM, &mut |_| false, &mut |c| {
            asked.push(c);
            [0x6C, 0x7D, 0x60, 0x44].contains(&c)
        });
        assert_eq!(names(r.new), vec!["Creeper Plant", "Hook", "Gold Card"]);
        assert!(!asked.contains(&0x44), "a left-out card is never asked");
        assert_eq!(s.equipped, 0, "KH1's rule: unequipped");
        // Hook's skill is Second Chance, Creeper Plant's Leaf Bracer: their effects.
        assert_eq!(skill(0x6C), ix::SECOND_CHANCE);
        assert_eq!(skill(0x60), ix::LEAF_BRACER);
        // The page: a third owner row; Enemy Skills by card kind; Gold Card greyed (not built).
        let mut sc = Screen::default();
        assert_eq!(sc.rows(&s).iter().map(|r| r.label).collect::<Vec<_>>(), vec![Label::Owner(Owner::Sora), Label::Owner(Owner::Shared), Label::Owner(Owner::EnemySkills), Label::Owner(Owner::Sleights)]);
        sc.set_cursor(2);
        assert_eq!(sc.confirm(&mut s), Outcome::Opened);
        let rows = sc.rows(&s);
        assert_eq!(rows.iter().map(|r| r.label).collect::<Vec<_>>(), vec![Label::Ability(ix::LEAF_BRACER), Label::Ability(ix::SECOND_CHANCE), Label::Ability(skill(0x7D))]);
        assert!(!rows[0].grey && !rows[1].grey && rows[2].grey);
        sc.set_cursor(2);
        assert_eq!(sc.confirm(&mut s), Outcome::NotBuilt(skill(0x7D)));
        assert_eq!(s.equipped, 0);
        // Equip Hook (5 AP) and Creeper Plant (5): 10 of 23; their status panel cost.
        sc.set_cursor(1);
        assert_eq!(sc.status(&s)[1], "5");
        assert_eq!(sc.confirm(&mut s), Outcome::Equipped(ix::SECOND_CHANCE));
        sc.set_cursor(0);
        assert_eq!(sc.confirm(&mut s), Outcome::Equipped(ix::LEAF_BRACER));
        assert_eq!(sc.status(&s), ["13/23".to_string(), "5".into(), "10/23".into()]);
        assert!(second_chance(s.equipped) && leaf_bracer(s.equipped));
        // Not on Sora's list.
        assert!(page::list(Owner::Sora, &s).iter().all(|&i| ALL[i].source != Source::Card(0x6C) && i != ix::LEAF_BRACER));
        // Soldier gives one Combo Plus; with 93 and 99, three.
        sync_with(&mut s, 99, NO_RECOM, &mut |_| false, &mut |c| c == 0x45);
        let all_cp = 1 << ix::COMBO_PLUS[0] | 1 << ix::COMBO_PLUS[1] | 1 << ix::COMBO_PLUS[2];
        assert_eq!(s.unlocked & all_cp, all_cp);
        assert_eq!(to_core(all_cp).combo_plus, 3);
        assert_eq!(page::owner_of(ix::COMBO_PLUS[0]), Owner::EnemySkills);
        assert_eq!(page::owner_of(ix::COMBO_PLUS[1]), Owner::Sora);
        // Treasure Magnet (both copies, stage 2) and the four other matches built; every own skill
        // a stage built (`crate::skill_effects::BUILT`) too, the rest not yet.
        assert!(built(ix::TREASURE_MAGNET[0]) && built(ix::TREASURE_MAGNET[1]));
        assert!([ix::COMBO_PLUS[0], ix::LEAF_BRACER, ix::BERSERK, ix::SECOND_CHANCE].iter().all(|&i| built(i)));
        // Every own skill a stage built is equippable; the rest (and nothing else) greyed.
        let own_unbuilt = crate::enemy_skills::SKILLS.iter().filter(|k| k.kind == crate::enemy_skills::Kind::Own && !k.built).count() as u32;
        assert_eq!(UNBUILT_BITS.count_ones(), own_unbuilt);
        assert!(own_unbuilt < 43, "stages 2 and 4 built some");
        assert_eq!(CARD_BITS.count_ones(), 48);
        assert_eq!(page::owner_of(ix::TREASURE_MAGNET[1]), Owner::Sora);
        assert_eq!(kh1_bits(CARD_BITS), kh1_bits(0), "no KH1 0x142D5EC10 bit");
        // The sleights' range ends before Treasure Magnet.
        assert_eq!(SLEIGHT_BITS & (CARD_BITS | 1 << ix::TREASURE_MAGNET[1]), 0);
    }

    /// v6 → v7 (Enemy Skills): the four matches learned by level before are taken back and asked
    /// again from their card; one whose card Sora has comes straight back, as equipped as it was.
    #[test]
    fn migration_v6_to_v7_moves_the_matches_to_their_cards() {
        let had = 1 << ix::COMBO_PLUS[0] | 1 << ix::COMBO_PLUS[1] | 1 << ix::LEAF_BRACER | 1 << ix::BERSERK | 1 << ix::SECOND_CHANCE | 1 << ix::BLITZ;
        let mut s = State { unlocked: had, equipped: 1 << ix::LEAF_BRACER | 1 << ix::BERSERK | 1 << ix::BLITZ, ap_ups: 10 };
        let raw0 = s;
        // Level 95, the Creeper Plant and Gargoyle cards had; not Hades, Hook or Soldier.
        let r = migrate_with(&mut s, raw0, BLOCK_V6, 95, NO_RECOM, 0, 0, &mut |id| id == sleight::BLITZ, &mut |c| c == 0x60 || c == 0x56);
        assert_eq!(names(r.taken), vec!["Soldier", "Hades", "Hook"]);
        assert_eq!(s.unlocked & had, had & !(1 << ix::COMBO_PLUS[0] | 1 << ix::BERSERK | 1 << ix::SECOND_CHANCE));
        assert_eq!(s.equipped, 1 << ix::LEAF_BRACER | 1 << ix::BLITZ, "Creeper Plant back, equipped as before; Berserk's AP back");
        assert_eq!(names(r.new & CARD_BITS), vec!["Gargoyle"], "Leaf Bracer came straight back: not new");
        assert_eq!(s.ap_ups, 10);
        // A v7 block is not migrated again; a card later gives its match back.
        let r = sync_with(&mut s, 95, NO_RECOM, &mut |_| false, &mut |c| c == 0x68);
        assert_eq!(names(r.new), vec!["Hades"]);
        // A new game (v0): nothing taken.
        let mut n = State::default();
        let r = migrate_with(&mut n, State::default(), 0, 1, NO_RECOM, 0, 0, &mut |_| false, &mut |_| false);
        assert_eq!((r.taken, n), (0, State::default()));
    }

    /// Owner (2026-11-01): High Jump / Glide / Superglide are equipped when Re:CoM gives them, still
    /// removable, and a removed one is not equipped again.
    #[test]
    fn movement_abilities_follow_recoms_variable() {
        let mut s = State::default();
        sync(&mut s, 1, NO_RECOM, &mut |_| false);
        // Re:CoM's High Jump (bit 1): unlocked and equipped (shared: free).
        let r = sync(&mut s, 1, recom::HIGH_JUMP, &mut |_| false);
        assert_eq!((names(r.new), names(r.new_equipped)), (vec![], vec!["High Jump"]));
        assert_ne!(s.equipped & 1 << ix::HIGH_JUMP, 0);
        // The player removes it: no later sync equips it again.
        s.equipped &= !(1 << ix::HIGH_JUMP);
        sync(&mut s, 1, 7, &mut |_| false);
        assert_eq!(s.equipped & 1 << ix::HIGH_JUMP, 0, "removed stays removed");
        assert_eq!(s.equipped & MOVE, 1 << ix::GLIDE | 1 << ix::SUPERGLIDE);
        // Bit 4 alone gives Re:CoM's glide too: Glide and Superglide, both equipped.
        let mut t = State::default();
        let r = sync(&mut t, 1, recom::SUPERGLIDE, &mut |_| false);
        assert_eq!(names(r.new_equipped), vec!["Glide", "Superglide"]);
        // Never taken back, whatever the variable says later; other bits mean nothing.
        sync(&mut s, 1, 0xFFFF_FFF8, &mut |_| false);
        assert_eq!(s.unlocked & MOVE, MOVE);
        let mut u = State::default();
        sync(&mut u, 1, 0xFFFF_FFF8, &mut |_| false);
        assert_eq!(u.unlocked, 0);
    }

    #[test]
    fn migration_keeps_the_old_set_within_kh1s_ap() {
        // A 9a save at level 34: 9a equipped every unlock (Sliding Dash, Blitz, Stun Impact,
        // Zantetsuken, Hurricane Blast, Ripple Drive, Aerial Sweep = 1+2+2+2+2+3+2 = 14 AP).
        let nine_a = [ix::SLIDING_DASH, ix::BLITZ, ix::STUN_IMPACT, ix::ZANTETSUKEN, ix::HURRICANE_BLAST, ix::RIPPLE_DRIVE, ix::AERIAL_SWEEP].iter().fold(0, |b, &i| b | 1 << i);
        let mut s = State { unlocked: nine_a, equipped: nine_a, ap_ups: 0 };
        let ini = crate::settings::Kh1Abilities::default(); // guard, counterattack on
        let raw = s;
        // Six past CP picks: 6 AP Ups = 15 AP; 14 wanted (the ini's Guard no longer costs anything).
        let r = migrate(&mut s, raw, BLOCK_V1, 34, NO_RECOM, ini_equipped(&ini), 6, &mut |id| id <= sleight::ZANTETSUKEN);
        assert_eq!((s.ap_ups, s.max_ap()), (6, 15));
        assert_eq!(r.fitted, 0);
        assert_eq!(s.equipped, nine_a);
        assert!(s.equipped & 1 << ix::COUNTERATTACK == 0, "Counterattack not unlocked before 57");
        assert!(s.equipped & 1 << ix::GRAVITY_BREAK == 0, "a new unlock: unequipped");
        // A new game (block never written): nothing in the block.
        let mut n = State::default();
        let raw = n;
        migrate(&mut n, raw, 0, 1, NO_RECOM, ini_equipped(&ini), 0, &mut |_| false);
        assert_eq!(n, State::default());
        // The ini's combo_plus copies at a level that has them.
        let cp = crate::settings::Kh1Abilities { combo_plus: 2, air_combo_plus: 1, ..ini };
        let mut m = State::default();
        let raw = m;
        migrate(&mut m, raw, 0, 99, NO_RECOM, ini_equipped(&cp), 1, &mut |_| false);
        let c = to_core(m.equipped);
        assert_eq!((c.combo_plus, c.air_combo_plus, c.guard, c.counterattack, c.dodge_roll), (2, 1, true, true, true));
    }

    /// v3 → v4 (owner, 2026-11-01): Guard and Dodge Roll leave the block (their AP back); Re:CoM's
    /// movement abilities, unequipped by v3's rule, are equipped once.
    #[test]
    fn v3_blocks_drop_the_kit_and_equip_recoms_movement() {
        let raw = State { unlocked: KIT_BITS | 1 << ix::RIPPLE_DRIVE | 1 << ix::BLITZ | 1 << ix::HIGH_JUMP, equipped: KIT_BITS | 1 << ix::BLITZ, ap_ups: 0 };
        let mut b = [0u8; BLOCK_LEN];
        b[field::VERSION] = BLOCK_V3;
        b[field::UNLOCKED..field::UNLOCKED + 4].copy_from_slice(&(raw.unlocked as u32).to_le_bytes());
        b[field::EQUIPPED..field::EQUIPPED + 4].copy_from_slice(&(raw.equipped as u32).to_le_bytes());
        let mut s = read(&b, &NO_EXT, &NO_SK);
        // One past CP pick: 5 AP. v3 used Guard 2 + Dodge Roll 1 + Blitz 2 = 5: now 2, so Ripple
        // Drive (3) fits when the player equips it.
        let r = migrate(&mut s, raw, BLOCK_V3, 11, recom::HIGH_JUMP, 0, 1, &mut |_| false);
        assert_eq!(names(r.kit_dropped), vec!["Guard", "Dodge Roll"]);
        assert_eq!(s.unlocked & KIT_BITS, 0);
        assert_eq!(used_ap(s.equipped), 2);
        assert!(can_equip(s.equipped, ix::RIPPLE_DRIVE, s.max_ap()));
        assert_ne!(s.equipped & 1 << ix::HIGH_JUMP, 0, "Re:CoM's High Jump equipped once");
        write(&s, &mut b, &mut [0; EXT_LEN], &mut [0; SKILL_EXT_LEN]);
        assert_eq!((b[field::VERSION], u32_at(&b, field::UNLOCKED) as Bits & KIT_BITS, b[field::AP_UPS]), (BLOCK_V7, 0, 1));
    }

    #[test]
    fn v2_story_abilities_move_to_their_new_sources() {
        // A 9b save at level 40 whose ini had High Jump, Glide, Superglide, Combo Master on (all
        // equipped), Guard equipped and Dodge Roll removed in the camp; Re:CoM has given only its
        // High Jump.
        let had = 1 << ix::HIGH_JUMP | 1 << ix::GLIDE | 1 << ix::SUPERGLIDE | 1 << ix::COMBO_MASTER;
        let raw = State { unlocked: had | KIT_BITS, equipped: had | 1 << ix::GUARD, ap_ups: 0 };
        let mut s = State { unlocked: had, equipped: had, ap_ups: 0 };
        let r = migrate(&mut s, raw, BLOCK_V2, 40, recom::HIGH_JUMP, Bits::MAX, 0, &mut |_| false);
        assert_eq!(names(r.taken), vec!["Glide", "Superglide", "Combo Master"]);
        assert_eq!(names(r.kit_dropped), vec!["Guard", "Dodge Roll"]);
        assert!(s.equipped & 1 << ix::HIGH_JUMP != 0, "Re:CoM has it: stays equipped");
        assert_eq!(s.unlocked & V2_STORY_BITS, 1 << ix::HIGH_JUMP);
        assert_eq!(s.equipped & 1 << ix::COUNTERATTACK, 0, "the ini is read only from v0 / v1");
        // Level 50 and Re:CoM's Glide later: Glide back equipped (Re:CoM's), Combo Master unequipped.
        let r = sync(&mut s, 50, recom::HIGH_JUMP | recom::GLIDE, &mut |_| false);
        assert_eq!((names(r.new), names(r.new_equipped)), (vec!["Combo Master", "Air Combo Plus", "MP Rage"], vec!["Glide"]));
        // At the migration itself, a source already met keeps the old equipped state.
        let mut t = State { unlocked: had, equipped: had, ap_ups: 0 };
        let raw = t;
        let r = migrate(&mut t, raw, BLOCK_V2, 50, 7, 0, 0, &mut |_| false);
        assert_eq!(r.taken, 0);
        assert_eq!(t.equipped & V2_STORY_BITS, V2_STORY_BITS);
    }

    #[test]
    fn fit_takes_the_most_expensive_first() {
        let mut e = 1 << ix::RIPPLE_DRIVE | 1 << ix::BLITZ | 1 << ix::SLIDING_DASH | 1 << ix::HIGH_JUMP | KIT_BITS;
        assert_eq!(used_ap(e), 6, "the kit costs nothing");
        assert_eq!(fit_ap(&mut e, max_ap(0)), 1 << ix::RIPPLE_DRIVE);
        assert_eq!(used_ap(e), 3);
        assert!(e & 1 << ix::HIGH_JUMP != 0, "shared: free");
        // Equal costs: the later index goes first.
        let mut e = 1 << ix::BLITZ | 1 << ix::STUN_IMPACT;
        assert_eq!(fit_ap(&mut e, max_ap(0)), 1 << ix::STUN_IMPACT);
    }

    /// M12 step 3: KH1's Rod-list support abilities (btltbl `0x33D8` rows and `0x3BF8` levels:
    /// `ap_tables_match_btltbl` reads them from the player's file). They arrive unequipped, cost
    /// KH1's AP, are on Sora's page, and set no `0x142D5EC10` bit.
    #[test]
    fn rod_support_abilities() {
        let x = |i: usize| (ALL[i].kh1_id.unwrap(), ALL[i].ap, ALL[i].key, ALL[i].source, ALL[i].kh1_bit, ALL[i].shared);
        assert_eq!(x(ix::MP_HASTE), (23, 3, 304, Source::Level(21), 0, false));
        assert!(on_screen(ix::MP_HASTE));
        let mut s = State::default();
        assert_eq!(names(sync(&mut s, 20, NO_RECOM, &mut |_| false).new).contains(&"MP Haste"), false);
        assert!(names(sync(&mut s, 21, NO_RECOM, &mut |_| false).new).contains(&"MP Haste"));
        assert_eq!(s.equipped, 0, "KH1's rule: unequipped");
        assert!(!mp_haste(s.equipped));
        assert!(mp_haste(1 << ix::MP_HASTE));
        // Level 21: 7 AP (11, 17, 20); MP Haste's 3 fit.
        assert!(can_equip(0, ix::MP_HASTE, 21));
        assert_eq!(kh1_bits(1 << ix::MP_HASTE), kh1_bits(0));
        // MP Rage: two copies, Rod levels 42 and 75, 3 AP each; equipped copies add up.
        assert_eq!(ix::MP_RAGE.map(x), [(24, 3, 305, Source::Level(42), 0, false), (24, 3, 305, Source::Level(75), 0, false)]);
        let r = sync(&mut s, 74, NO_RECOM, &mut |_| false);
        assert_eq!(names(r.new).iter().filter(|&&n| n == "MP Rage").count(), 1);
        let r = sync(&mut s, 75, NO_RECOM, &mut |_| false);
        assert_eq!(names(r.new), vec!["MP Rage"]);
        assert_eq!((mp_rage(0), mp_rage(1 << ix::MP_RAGE[1]), mp_rage(1 << ix::MP_RAGE[0] | 1 << ix::MP_RAGE[1])), (0, 1, 2));
        assert_eq!(used_ap(1 << ix::MP_RAGE[0] | 1 << ix::MP_RAGE[1]), 6);
        // Leaf Bracer: KH1's Rod level 39, 5 AP; Enemy Skills: from the Creeper Plant card instead.
        assert_eq!(x(ix::LEAF_BRACER), (62, 5, 308, Source::Card(0x60), 0, false));
        assert!(leaf_bracer(1 << ix::LEAF_BRACER) && !leaf_bracer(0));
        // 17 AP (KH1's 3 + 7 AP Up picks, M12 step 1).
        assert_eq!(max_ap(7), 17);
        assert!(can_equip(0, ix::LEAF_BRACER, 17));
        let thirteen = 1 << ix::MP_HASTE | 1 << ix::MP_RAGE[0] | 1 << ix::RIPPLE_DRIVE | 1 << ix::BLITZ | 1 << ix::STUN_IMPACT;
        assert_eq!(used_ap(thirteen), 13);
        assert!(!can_equip(thirteen, ix::LEAF_BRACER, 17), "4 AP left: refused (KH1's rule)");
        // Berserk: KH1's Rod level 60, 1 AP; Enemy Skills: from the Hades card instead.
        assert_eq!(x(ix::BERSERK), (26, 1, 306, Source::Card(0x68), 0, false));
        assert!(berserk(1 << ix::BERSERK) && !berserk(1 << ix::LEAF_BRACER));
        let mut t = State::default();
        assert!(!names(sync(&mut t, 99, NO_RECOM, &mut |_| false).new).contains(&"Hades"), "no level gives it");
        assert!(names(sync_with(&mut t, 99, NO_RECOM, &mut |_| false, &mut |c| c == 0x68).new).contains(&"Hades"));
        // Second Chance: KH1's Rod level 90, 5 AP; Enemy Skills: from the Hook card instead.
        assert_eq!(x(ix::SECOND_CHANCE), (25, 5, 307, Source::Card(0x6C), 0, false));
        assert!(second_chance(1 << ix::SECOND_CHANCE) && !second_chance(1 << ix::BERSERK));
        assert_eq!(names(sync_with(&mut t, 99, NO_RECOM, &mut |_| false, &mut |c| c == 0x6C).new), vec!["Hook"]);
        // Sora's list keeps MP Haste and MP Rage in KH1's key order; the matches are Enemy Skills.
        sync_with(&mut t, 99, NO_RECOM, &mut |_| false, &mut |c| c == 0x60);
        let all = State { unlocked: t.unlocked, equipped: 0, ap_ups: 0 };
        let l = page::list(page::Owner::Sora, &all);
        let pos = |i: usize| l.iter().position(|&x| x == i).unwrap();
        assert!(pos(ix::MP_HASTE) < pos(ix::MP_RAGE[0]) && pos(ix::MP_RAGE[0]) < pos(ix::MP_RAGE[1]));
        for i in [ix::BERSERK, ix::SECOND_CHANCE, ix::LEAF_BRACER] {
            assert!(!l.contains(&i));
        }
        assert_eq!(page::list(page::Owner::EnemySkills, &all), vec![ix::LEAF_BRACER, ix::BERSERK, ix::SECOND_CHANCE], "by card: Creeper Plant 0x60, Hades 0x68, Hook 0x6C");
    }

    /// M12 step 4 (owner, "M12 answers"): the 25 magic sleights plus Idyll Romp and Cross-slash+,
    /// with the owner's AP costs, in Re:CoM's id order.
    #[test]
    fn sleight_abilities_are_the_owners_27() {
        assert_eq!(SLEIGHTS, 31..58);
        let ids: Vec<u16> = SLEIGHTS.filter_map(sleight_of).collect();
        let want: Vec<u16> = (0x1B..=0x33).chain([0x3A, 0x43]).collect();
        assert_eq!(ids, want, "the 25 magic-category ids, Idyll Romp, Cross-slash+");
        let by_ap = |ap: u8| -> Vec<&str> {
            let mut v: Vec<&str> = SLEIGHTS.filter(|&i| ALL[i].ap == ap).map(|i| ALL[i].name).collect();
            v.sort_unstable();
            v
        };
        let sorted = |mut v: Vec<&'static str>| {
            v.sort_unstable();
            v
        };
        assert_eq!(by_ap(2), sorted(vec!["Judgment", "Bind", "Confuse", "Terror", "Warp", "Warpinator", "Synchro", "Teleport", "Gifted Miracle", "Idyll Romp"]));
        assert_eq!(by_ap(3), sorted(vec!["Fire Raid", "Blizzard Raid", "Aqua Splash", "Freeze", "Firaga Burst", "Reflect Raid", "Homing Blizzara", "Lethal Frame", "Shock Impact"]));
        assert_eq!(by_ap(4), sorted(vec!["Thunder Raid", "Raging Storm", "Magnet Spiral", "Quake", "Tornado", "Cross-slash+"]));
        assert_eq!(by_ap(5), sorted(vec!["Holy", "Mega Flare"]));
        for i in SLEIGHTS {
            let a = &ALL[i];
            assert!(on_screen(i) && !a.shared && a.kh1_id.is_none() && a.kh1_bit == 0, "{}", a.name);
            assert_eq!(a.key, SLEIGHT_KEY + sleight_of(i).unwrap());
            assert_eq!(of_sleight(sleight_of(i).unwrap()), Some(i));
        }
        assert_eq!(SLEIGHT_BITS.count_ones(), 27);
        assert_eq!(SLEIGHT_BITS & (MODIFIER_BITS | KIT_BITS | SPECIALS_WAITING), 0);
        // Not abilities: the generic tiers, the summon levels, friends, the KH1-twin sleights.
        for id in [0xD, 0x1A, 0x35, 0x41, 0x42, 0x44, 0x56, 4, 9, 0xC] {
            assert_eq!(of_sleight(id), None, "{id:#x}");
        }
        assert_eq!(kh1_bits(SLEIGHT_BITS), kh1_bits(0), "no KH1 bit");
        assert_eq!(to_core(SLEIGHT_BITS), to_core(0), "nothing in KH1's frame");
        // MP (owner): AP 2 → 1, 3 → 2, 4 → 2, 5 → 3; Idyll Romp / Cross-slash+ all.
        let mp = |id| sleight_mp(of_sleight(id).unwrap()).unwrap();
        assert_eq!([mp(0x1F), mp(0x1B), mp(0x1D), mp(0x33)], [SleightMp::Points(1), SleightMp::Points(2), SleightMp::Points(2), SleightMp::Points(3)]);
        assert_eq!([mp(sleight::IDYLL_ROMP), mp(sleight::CROSS_SLASH_PLUS)], [SleightMp::All, SleightMp::All]);
        assert_eq!(sleight_mp(ix::SONIC_BLADE), None);
    }

    /// The names as Re:CoM's `SY0002.CTD` lists them from id 4, read from the player's file when
    /// `RECOM_SY0002_CTD` names it (the CTD's "Lethal Flame" is the owner's "Lethal Frame").
    #[test]
    fn sleight_names_match_recoms_ctd() {
        let Ok(path) = std::env::var("RECOM_SY0002_CTD") else { return };
        let Ok(b) = std::fs::read(path) else { return };
        let words: Vec<&[u8]> = b.split(|&c| c == 0).filter(|w| !w.is_empty()).collect();
        let first = words.iter().position(|w| *w == b"Sliding Dash").expect("the sleight names");
        for i in SLEIGHTS {
            let id = sleight_of(i).unwrap() as usize;
            let ctd = std::str::from_utf8(words[first + id - 4]).unwrap();
            let want = if ALL[i].name == "Lethal Frame" { "Lethal Flame" } else { ALL[i].name };
            assert_eq!(ctd, want, "id {id:#x}");
        }
    }

    /// Each sleight ability arrives when Re:CoM teaches it (its learned bit, a read), unequipped,
    /// never taken back; equipping follows KH1's AP rule; the menu's list is in Re:CoM's order.
    #[test]
    fn sleight_abilities_learn_from_recoms_bits() {
        let i = |id| of_sleight(id).unwrap();
        let mut s = State { ap_ups: 1, ..Default::default() };
        let mut asked = Vec::new();
        let r = sync(&mut s, 1, NO_RECOM, &mut |id| {
            asked.push(id);
            id == sleight::FREEZE || id == sleight::HOLY
        });
        assert!(SLEIGHTS.clone().all(|k| asked.contains(&sleight_of(k).unwrap())), "every sleight's learned bit is read");
        assert_eq!(names(r.new), vec!["Freeze", "Holy"]);
        assert_eq!(s.equipped, 0, "KH1's rule: unequipped");
        // Never taken back (Re:CoM never unlearns; a sync only asks about the rest).
        sync(&mut s, 1, NO_RECOM, &mut |_| false);
        assert_eq!(names(s.unlocked & SLEIGHT_BITS), vec!["Freeze", "Holy"]);
        // 5 AP: Holy (5) fits alone; then Freeze (3) is refused; the page greys it.
        assert!(can_equip(0, i(sleight::HOLY), s.max_ap()));
        let mut sc = Screen::default();
        // Owner, 2026-11-04: the sleights on their own "Sleights" owner row (the fourth).
        assert!(sc.rows(&s)[0].grey && !sc.rows(&s)[3].grey, "Sora's row has none; Sleights has two");
        assert_eq!(sc.confirm(&mut s), Outcome::Empty);
        sc.set_cursor(3);
        assert_eq!(sc.confirm(&mut s), Outcome::Opened);
        let l: Vec<Label> = sc.rows(&s).iter().map(|r| r.label).collect();
        assert_eq!(l, vec![Label::Ability(i(sleight::FREEZE)), Label::Ability(i(sleight::HOLY))], "Re:CoM's order");
        sc.set_cursor(1);
        assert_eq!(sc.confirm(&mut s), Outcome::Equipped(i(sleight::HOLY)));
        assert_eq!(sc.status(&s), ["0/5".to_string(), "5".into(), "5/5".into()]);
        sc.set_cursor(0);
        assert!(sc.rows(&s)[0].grey);
        assert_eq!(sc.confirm(&mut s), Outcome::Refused(i(sleight::FREEZE)));
        assert_eq!(equipped_sleights(s.equipped), vec![sleight::HOLY]);
        // The Sleights menu's list: only the equipped ones, Re:CoM's id order.
        let e = 1 << i(sleight::CROSS_SLASH_PLUS) | 1 << i(sleight::FIRE_RAID) | 1 << i(sleight::LETHAL_FRAME) | 1 << ix::SONIC_BLADE | 1 << ix::BLITZ;
        assert_eq!(equipped_sleights(e), vec![0x1B, 0x27, 0x43]);
        assert!(equipped_sleights(0).is_empty());
        // Owner, 2026-11-04: all 27 on the Sleights row, in Re:CoM's order, none on any other row;
        // KH1's specials (Sonic Blade, Strike Raid, Ars Arcanum, Ragnarok) stay Sora's.
        let all = State { unlocked: ALL_BITS & !KIT_BITS & !SPECIALS_WAITING, equipped: 0, ap_ups: 0 };
        let l = page::list(page::Owner::Sleights, &all);
        assert_eq!(l, SLEIGHTS.collect::<Vec<_>>());
        assert!(l.windows(2).all(|w| sleight_of(w[0]) < sleight_of(w[1])));
        for o in [page::Owner::Sora, page::Owner::Shared, page::Owner::EnemySkills] {
            assert!(page::list(o, &all).iter().all(|k| !SLEIGHTS.contains(k)), "{o:?}");
        }
        let sora = page::list(page::Owner::Sora, &all);
        assert!([ix::SONIC_BLADE, ix::STRIKE_RAID].iter().all(|k| sora.contains(k)));
        assert!(SLEIGHTS.clone().all(|k| page::owner_of(k) == page::Owner::Sleights));
    }

    /// Layout v6 (M12 step 4): the words' bits 32.. in the extension, read only from a v6 block; a
    /// v5 block migrates once (its fields kept, the sleights Re:CoM already taught unlocked).
    #[test]
    fn migration_v5_to_v6() {
        let i = |id| of_sleight(id).unwrap();
        // A v5 save: level 40, Blitz and MP Haste unlocked, Blitz equipped, 4 AP Ups (11 AP).
        let mut b = [0u8; BLOCK_LEN];
        b[field::VERSION] = BLOCK_V5;
        let lo = 1u32 << ix::BLITZ | 1 << ix::MP_HASTE;
        b[field::UNLOCKED..field::UNLOCKED + 4].copy_from_slice(&lo.to_le_bytes());
        b[field::EQUIPPED..field::EQUIPPED + 4].copy_from_slice(&(1u32 << ix::BLITZ).to_le_bytes());
        b[field::AP_UPS] = 4;
        // Whatever the extension's bytes say before v6, they mean nothing.
        let junk = [0xFF; EXT_LEN];
        assert_eq!(read(&b, &junk, &NO_SK), read(&b, &NO_EXT, &NO_SK));
        assert_eq!(raw(&b, &junk, &NO_SK).unlocked >> 31, 0);
        let mut s = read(&b, &junk, &NO_SK);
        let raw0 = raw(&b, &junk, &NO_SK);
        assert_eq!((s.ap_ups, s.max_ap()), (4, 11));
        // Re:CoM had taught Freeze (event) and Lethal Frame (level 32) before.
        let r = migrate(&mut s, raw0, BLOCK_V5, 40, NO_RECOM, 0, 9, &mut |id| id == sleight::BLITZ || id == sleight::FREEZE || id == sleight::LETHAL_FRAME);
        assert_eq!(s.ap_ups, 4, "the v5 count is kept (not the CP's)");
        assert_eq!(names(r.new & SLEIGHT_BITS), vec!["Freeze", "Lethal Frame"]);
        assert_eq!(s.equipped, 1 << ix::BLITZ, "the old set kept; the new ones unequipped");
        assert_eq!(r.fitted, 0);
        // Written as v6: the high bits go to the extension, the block's other bytes unchanged.
        let mut e = junk;
        write(&s, &mut b, &mut e, &mut [0; SKILL_EXT_LEN]);
        assert_eq!((b[field::VERSION], b[field::AP_UPS]), (BLOCK_V7, 4));
        assert!(b[field::RESERVED..].iter().all(|&x| x == 0));
        assert_eq!(read(&b, &e, &NO_SK), s);
        let hi = (s.unlocked >> 32) as u32;
        assert_eq!(u32_at(&e, ext::UNLOCKED_HIGH), hi);
        assert_ne!(hi, 0);
        assert_eq!(u32_at(&e, ext::EQUIPPED_HIGH), 0);
        // An equipped sleight survives the round trip (bit 31 in the block, 32.. in the extension).
        s.equipped |= 1 << i(sleight::FIRE_RAID) | 1 << i(sleight::LETHAL_FRAME);
        s.unlocked |= 1 << i(sleight::FIRE_RAID);
        write(&s, &mut b, &mut e, &mut [0; SKILL_EXT_LEN]);
        assert_eq!(read(&b, &e, &NO_SK), s);
        assert_eq!(u32_at(&b, field::EQUIPPED) >> 31, 1, "Fire Raid is bit 31");
        // A v6 block is not migrated again: a sync asks only about what isn't unlocked.
        let mut asked = Vec::new();
        let mut t = read(&b, &e, &NO_SK);
        sync(&mut t, 40, NO_RECOM, &mut |id| {
            asked.push(id);
            false
        });
        assert!(!asked.contains(&sleight::FREEZE) && asked.contains(&sleight::HOLY));
    }

    #[test]
    fn to_core_counts_copies() {
        let e = 1 << ix::COMBO_PLUS[0] | 1 << ix::COMBO_PLUS[2] | 1 << ix::AIR_COMBO_PLUS[1] | 1 << ix::GLIDE | 1 << ix::VORTEX;
        let c = to_core(e);
        assert_eq!((c.combo_plus, c.air_combo_plus, c.glide, c.vortex, c.guard, c.high_jump), (2, 1, true, true, true, false));
    }

    #[test]
    fn the_screen_follows_kh1s_rules() {
        // One AP Up: 5 AP.
        let mut s = State { unlocked: 1 << ix::RIPPLE_DRIVE | 1 << ix::BLITZ | 1 << ix::VORTEX | 1 << ix::GLIDE | 1 << ix::STRIKE_RAID | 1 << ix::SONIC_BLADE, equipped: 1 << ix::GLIDE, ap_ups: 1 };
        let mut sc = Screen::default();
        let rows = sc.rows(&s);
        assert_eq!(rows.iter().map(|r| r.label).collect::<Vec<_>>(), vec![Label::Owner(Owner::Sora), Label::Owner(Owner::Shared), Label::Owner(Owner::EnemySkills), Label::Owner(Owner::Sleights)]);
        assert!(rows[2].grey, "no Enemy Skill yet");
        assert_eq!(sc.confirm(&mut s), Outcome::Opened);
        // Sora's list in KH1's key order; Sonic Blade (801) and, since M12 step 7, Strike Raid (803).
        let l: Vec<Label> = sc.rows(&s).iter().map(|r| r.label).collect();
        assert_eq!(l, vec![Label::Ability(ix::VORTEX), Label::Ability(ix::BLITZ), Label::Ability(ix::RIPPLE_DRIVE), Label::Ability(ix::SONIC_BLADE), Label::Ability(ix::STRIKE_RAID)]);
        // Equip Ripple Drive (3), then Blitz (2) = 5; Vortex (1) is greyed and refused.
        sc.set_cursor(2);
        assert_eq!(sc.confirm(&mut s), Outcome::Equipped(ix::RIPPLE_DRIVE));
        sc.set_cursor(1);
        assert_eq!(sc.confirm(&mut s), Outcome::Equipped(ix::BLITZ));
        assert_eq!(sc.status(&s), ["0/5".to_string(), "2".into(), "5/5".into()]);
        let rows = sc.rows(&s);
        assert!(rows[0].grey && !rows[0].marked && rows[1].marked && rows[2].marked && rows[3].grey);
        sc.set_cursor(0);
        assert_eq!(sc.confirm(&mut s), Outcome::Refused(ix::VORTEX));
        // Remove Ripple Drive: Sonic Blade (3) fits.
        sc.set_cursor(2);
        assert_eq!(sc.confirm(&mut s), Outcome::Removed(ix::RIPPLE_DRIVE));
        sc.set_cursor(3);
        assert_eq!(sc.confirm(&mut s), Outcome::Equipped(ix::SONIC_BLADE));
        // Shared: free toggles.
        assert_eq!(sc.cancel(), Outcome::Back);
        sc.set_cursor(1);
        assert_eq!(sc.confirm(&mut s), Outcome::Opened);
        assert_eq!(sc.confirm(&mut s), Outcome::Removed(ix::GLIDE));
        assert_eq!(sc.confirm(&mut s), Outcome::Equipped(ix::GLIDE));
        assert_eq!(sc.cancel(), Outcome::Back);
        assert_eq!(sc.cursor(), 1, "back on the owner row");
        assert_eq!(sc.cancel(), Outcome::Close);
        // An owner with nothing.
        let mut e = State::default();
        let mut sc = Screen::default();
        assert_eq!(sc.confirm(&mut e), Outcome::Empty);
        assert!(sc.rows(&e)[0].grey);
    }

    /// The page's rows take Re:CoM's list entry 0x131 (the green row with the green Mickey
    /// symbol): `seqs[ROW_LOOK]` of the checked sequence table `0x1407C11F0`.
    #[test]
    fn rows_are_recoms_green_mickey_row() {
        let seqs = crate::sites::KEYBLADE_DATA.iter().find(|s| s.rva == 0x007C_11F0).unwrap().expect;
        let at = 2 * page::ROW_LOOK as usize;
        assert_eq!(u16::from_le_bytes([seqs[at], seqs[at + 1]]), 0x131);
    }

    #[test]
    fn texts_fit() {
        let mut all: Vec<&[u8]> = vec![page::text::LABEL, page::text::HELP, page::text::SORA, page::text::SHARED, page::text::ENEMY_SKILLS, page::text::EMPTY_HELP, page::text::NO_AP, page::text::NOT_BUILT];
        all.extend(page::text::STATUS);
        all.extend(page::text::OWNER_HELP);
        all.extend((0..ALL.len()).filter(|&i| on_screen(i)).map(page::text::help));
        for s in all {
            assert_eq!(s.last(), Some(&0));
            assert!(s.len() > 1);
            assert!(s[..s.len() - 1].iter().all(|&b| (0x20..0x7F).contains(&b) && b != b'%'));
            assert!(s.len() - 1 <= crate::menu::HELP_MAX, "{}", std::str::from_utf8(s).unwrap());
        }
    }

    /// Owner (2026-11-03, "Sleight ability texts"): each of the 27 sleight abilities has its own
    /// one-line description, ending in its MP ("Uses n MP." by the owner's tier, "Uses all MP." for
    /// Idyll Romp / Cross-slash+); the name stays "Lethal Frame".
    #[test]
    fn sleight_help_lines_say_what_and_the_mp() {
        assert_eq!(page::text::SLEIGHT_HELP.len(), 27);
        let mut seen = std::collections::HashSet::new();
        for i in SLEIGHTS {
            let id = sleight_of(i).unwrap();
            let line = std::str::from_utf8(page::text::help(i)).unwrap().trim_end_matches('\0');
            assert_eq!(Some(page::text::help(i)), page::text::sleight_help(id), "{id:#x}");
            let want = match sleight_mp(i).unwrap() {
                SleightMp::Points(n) => format!(" Uses {n} MP."),
                SleightMp::All => " Uses all MP.".to_string(),
            };
            assert!(line.ends_with(&want), "{}: {line:?} should end with {want:?}", ALL[i].name);
            let what = &line[..line.len() - want.len()];
            assert!(what.len() >= 20 && !what.contains("A sleight from Re:CoM"), "{}: {line:?}", ALL[i].name);
            assert!(seen.insert(what.to_string()), "{}: the same description twice", ALL[i].name);
        }
        assert_eq!(ALL[of_sleight(sleight::LETHAL_FRAME).unwrap()].name, "Lethal Frame");
        assert_eq!(page::text::sleight_help(sleight::SONIC_BLADE), None);
    }
}
