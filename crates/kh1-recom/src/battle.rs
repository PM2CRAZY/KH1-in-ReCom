//! M6 stage 1, battles at the KH1 setting (decisions 29 / 30; `docs/recom-m6-battles.md`): the
//! testable part. The runtime is `win/battle.rs`.
//!
//! At Combat = KH1, in Sora's story:
//!
//! - **P1**: the Heartless card play (`0x140198870`, enemy vtable `+0x98` of 49 classes) does what
//!   Re:CoM's message 2 does when the card is in place: the enemy enters the attack state its
//!   think chose. No card value, no CBTL side-2 slot, no `CBattleCard`, no cue sprite. CBTL's
//!   lock gates (win pose, escape, death, duels) still stop it: then Re:CoM's own card play runs
//!   and refuses as it always does.
//! - **P2–P4**: three code patches of Re:CoM's other serialisers, switched once per battle when
//!   Re:CoM builds its battle enemy manager (`0x1401859B0`): the think gate (`0x14017D010`), one
//!   grant per frame (`0x14018765F`) and the crowd think-rate (`0x14017F3DE`). At the Re:CoM
//!   setting the bytes are Re:CoM's own.
//!
//! Every offset is Steam, read in the Re:CoM decomp and checked in the exe where noted.

/// Re:CoM's enemy (`CBattleEnemy`) fields this stage reads or writes. Checked (decomp
/// `0x140198870`, `0x140181620`, `0x140185320`, `0x14017E540`; disasm `0x1401816AC`).
pub mod enemy {
    pub const ID: usize = 0x20;
    /// Current / next state pointer; state k lives at `enemy + (k + 0x13) * 0x40`.
    pub const STATE: usize = 0x60;
    pub const NEXT_STATE: usize = 0x68;
    pub const FLAGS: usize = 0x260;
    pub const HP: usize = 0x274;
    /// The attack state the think chose (2..9 from the common think `0x140196EC0`).
    pub const CHOSEN: usize = 0x278;
    /// The cooldown after an attack (`0x14017E540`'s third argument), in Re:CoM time units.
    pub const COOLDOWN: usize = 0x2C8;
    /// `CBossMarluxia3`'s mode flags: with bit 4 its message handler `0x1403F4550` ignores
    /// message 2 (it never reaches `0x140181620`). Checked (disasm).
    pub const MARLUXIA3_MODE: usize = 0xCE4;
    pub const MARLUXIA3_IGNORES_GO: u32 = 4;
    /// The message handler slot (vtable `+0x48`).
    pub const VT_MESSAGE: usize = 0x48;
    /// The task-type slot (vtable `+0x28`): 0x1004 for an enemy (`0x140186B30`).
    pub const VT_TYPE: usize = 0x28;
}

/// `+0x260` bits. Checked (exe: the masks at `0x14052A410` 1, `0x14052A418` 2, `0x14052A458`
/// 0x200, `0x14052A468` 0x800, `0x14052A4E8` 0x8000000; decomp of the users).
pub mod bits {
    /// In an attack (set by a successful card play; cleared at the attack's end `0x14017E540`).
    pub const PLAYING: u64 = 1;
    /// Set when a card play is refused or given up; no reader found (Checked (exe): the mask
    /// `0x14052A418` is only ever OR-ed / AND-ed).
    pub const REFUSED: u64 = 2;
    /// "Attack started" (message 2; cleared by the state end `0x14017DA90`).
    pub const STARTED: u64 = 4;
    /// "Free": the think runs only with it (`0x140185320`); message 2 clears it.
    pub const FREE: u64 = 0x10;
    /// The think's attack request, and the manager's grant.
    pub const REQUEST: u64 = 0x200;
    pub const GRANT: u64 = 0x800;
    /// Set by message 2 when the card record names a sleight (ids 0x6F..0x8E), else cleared.
    pub const SLEIGHT: u64 = 0x2000_0000;
}

/// CBTL (`CBattleCardTaskList`, task 0x16, `*0x14087A818`) fields `0x1401634B0` tests before an
/// enemy may play. Checked (disasm `0x1401634BA..0x1401634E9`).
pub mod cbtl {
    pub const LOCK: usize = 0x208;
    /// `test eax,0xC0000000` then `test eax,0xC0000004` (the second covers the first).
    pub const LOCK_MASK: u32 = 0xC000_0004;
    /// Sora's gimmick card (a byte).
    pub const GIMMICK: usize = 0x1E4;
    /// Duel modes: refused at 2 or more (`cmp [rcx+0x20C],1; ja`).
    pub const DUEL: usize = 0x20C;
    /// The side-2 (enemy) slot in use (`0x140163830`: `+0xB8 + (side-1)*4`).
    pub const SLOT_ENEMY: usize = 0xBC;
}

/// Re:CoM's task ids (`0x1402AD300(id)`): CBTL and the battle enemy manager. Checked (decomp
/// `0x140198870`, `0x14017D010`).
pub const TASK_CBTL: u32 = 0x16;
pub const TASK_ENEMY_MANAGER: u32 = 0x19;
/// The manager's enemy list (`+0x40`: node `+8` object, `+0x10` next) and the enemy task type.
/// Checked (decomp `0x140186B30`).
pub const MANAGER_LIST: usize = 0x40;
pub const NODE_OBJECT: usize = 8;
pub const NODE_NEXT: usize = 0x10;
pub const ENEMY_TYPE: u32 = 0x1004;

/// Re:CoM's generic enemy message handler and `CBossMarluxia3`'s (vtable `+0x48`), as RVAs.
/// Checked (exe: the 49 vtables that hold `0x140198870` at `+0x98`; 48 hold `0x140181620` at
/// `+0x48`, `CBossMarluxia3` holds `0x1403F4550`).
pub const HANDLER_GENERIC: u32 = 0x0018_1620;
pub const HANDLER_MARLUXIA3: u32 = 0x003F_4550;

/// The attack states (Re:CoM runs the "hurt bodies off" `+0x438` when 2..10 start).
pub const ATTACK_STATES: std::ops::RangeInclusive<i32> = 2..=10;

/// The enemy's message handler, as far as message 2 is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handler {
    /// `0x140181620`: case 2 starts the chosen state.
    Generic,
    /// `0x1403F4550`: the generic handler unless its mode bit 4 is set.
    Marluxia3,
    /// Anything else: not read, so Re:CoM's card path stays.
    Other,
}

/// CBTL's gate fields at the moment of the play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gate {
    pub lock: u32,
    pub gimmick: u8,
    pub duel: u32,
}

impl Gate {
    /// `0x1401634B0`'s refusal test, exactly.
    pub fn locked(&self) -> bool {
        self.lock & cbtl::LOCK_MASK != 0 || self.gimmick != 0 || self.duel >= 2
    }
}

/// Why an attack takes Re:CoM's card path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// Not a KH1 battle (Re:CoM setting at battle start, or Riku's story).
    Setting,
    /// HP ≤ 0: message 2 does nothing then (`0x140181620`'s HP test). Not reached in practice:
    /// the enemy update only plays with HP > 0.
    Dead,
    /// A message handler this stage doesn't know.
    Handler,
    /// `CBossMarluxia3` with mode bit 4: its handler would ignore message 2.
    Marluxia3Mode,
    /// No CBTL task.
    NoCbtl,
    /// CBTL locked: Re:CoM refuses the play itself.
    Locked(Gate),
    /// The chosen state is not an attack state.
    State(i32),
}

/// What the card-play wrapper does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Path {
    /// Enter `state` with these `+0x260` flags, return 1.
    Cardless { flags: u64, state: i32 },
    /// Call Re:CoM's card play.
    Recom(Why),
}

/// Everything the decision reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Play {
    pub kh1_battle: bool,
    pub flags: u64,
    pub hp: i32,
    pub chosen: i32,
    pub handler: Handler,
    pub marluxia3_mode: u32,
    pub gate: Option<Gate>,
}

/// The flags after the cardless play: what `0x140198870` clears first (`0x802`), then what
/// message 2 does (`0x140181620` case 2: `& ~0x200`, `& ~0x10`, `| 4`, and bit 29 cleared for a
/// record without a sleight id), with bit 0 the successful play sets.
pub fn cardless_flags(flags: u64) -> u64 {
    (flags & !(bits::GRANT | bits::REFUSED | bits::REQUEST | bits::FREE | bits::SLEIGHT)) | bits::PLAYING | bits::STARTED
}

pub fn decide(p: &Play) -> Path {
    if !p.kh1_battle {
        return Path::Recom(Why::Setting);
    }
    if p.hp <= 0 {
        return Path::Recom(Why::Dead);
    }
    match p.handler {
        Handler::Other => return Path::Recom(Why::Handler),
        Handler::Marluxia3 if p.marluxia3_mode & enemy::MARLUXIA3_IGNORES_GO != 0 => return Path::Recom(Why::Marluxia3Mode),
        _ => {}
    }
    let Some(g) = p.gate else { return Path::Recom(Why::NoCbtl) };
    if g.locked() {
        return Path::Recom(Why::Locked(g));
    }
    if !ATTACK_STATES.contains(&p.chosen) {
        return Path::Recom(Why::State(p.chosen));
    }
    Path::Cardless { flags: cardless_flags(p.flags), state: p.chosen }
}

/// The state object's address for state `k` (what message 2 writes to `+0x68`).
pub fn state_address(enemy: usize, k: i32) -> usize {
    enemy + crate::guard::state_offset(k as usize)
}

/// Whether an enemy is in an attack: bit 0, or its current state is an attack state.
pub fn attacking(flags: u64, state: Option<usize>) -> bool {
    flags & bits::PLAYING != 0 || state.is_some_and(|s| ATTACK_STATES.contains(&(s as i32)))
}

/// One code patch: the bytes Re:CoM has (checked at start, in `sites::M6_PATCHES`) and ours.
pub struct Patch {
    pub site: crate::sites::Site,
    pub new: &'static [u8],
}

/// What the bytes at a patch site are now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bytes {
    Recom,
    Ours,
    Other,
}

pub fn bytes_state(p: &Patch, have: &[u8]) -> Bytes {
    bytes_state_of(p.site.expect, p.new, have)
}

/// What `have` is, given Re:CoM's bytes and ours (shared with M6 stage 2's call retargets,
/// whose new bytes are only known at run time).
pub fn bytes_state_of(recom: &[u8], ours: &[u8], have: &[u8]) -> Bytes {
    if have == recom {
        Bytes::Recom
    } else if have == ours {
        Bytes::Ours
    } else {
        Bytes::Other
    }
}

/// One write of [`plan`]: (patch index, the bytes there now, the bytes to write).
pub type Write<'a> = (usize, &'a [u8], &'a [u8]);

/// The writes that make every patch `ours` (or Re:CoM's). None when any site holds bytes that
/// are neither (then nothing is written at all).
pub fn plan<'a>(patches: &'a [Patch], now: &[Bytes], ours: bool) -> Option<Vec<Write<'a>>> {
    let pairs: Vec<(&'a [u8], &'a [u8])> = patches.iter().map(|p| (p.site.expect, p.new)).collect();
    plan_pairs(&pairs, now, ours)
}

/// [`plan`] over (Re:CoM's bytes, ours) pairs.
pub fn plan_pairs<'a>(pairs: &[(&'a [u8], &'a [u8])], now: &[Bytes], ours: bool) -> Option<Vec<Write<'a>>> {
    if now.len() != pairs.len() || now.contains(&Bytes::Other) {
        return None;
    }
    Some(
        pairs
            .iter()
            .zip(now)
            .enumerate()
            .filter_map(|(i, (&(recom, new), &b))| match (b, ours) {
                (Bytes::Recom, true) => Some((i, recom, new)),
                (Bytes::Ours, false) => Some((i, new, recom)),
                _ => None,
            })
            .collect(),
    )
}

/// The trace's M6 cell for one enemy: `id:s<state>:<P S R G>:<cooldown>`.
pub fn trace_enemy(id: u32, state: Option<usize>, flags: u64, cooldown: i32) -> String {
    let b = |m: u64, c: char| if flags & m != 0 { c } else { '-' };
    let st = state.map_or("?".to_string(), |s| s.to_string());
    format!("{id}:s{st}:{}{}{}{}:{cooldown}", b(bits::PLAYING, 'P'), b(bits::STARTED, 'S'), b(bits::REQUEST, 'R'), b(bits::GRANT, 'G'))
}

/// The trace's M6 token: whether this battle is a KH1 one, how many enemies are in an attack,
/// CBTL's enemy slot, and up to [`TRACE_ENEMIES`] enemies.
pub fn trace_token(kh1_battle: bool, attacking_now: usize, slot_enemy: Option<i32>, enemies: &[String]) -> String {
    format!(
        "m6 battle={} attacking={attacking_now} cbtl_slot2={} enemies=[{}]",
        if kh1_battle { "kh1" } else { "recom" },
        slot_enemy.map_or("none".to_string(), |s| s.to_string()),
        enemies.join(" ")
    )
}

pub const TRACE_ENEMIES: usize = 8;

/// Log caps (first lines only).
pub const ATTACK_LOG_LINES: u32 = 100;
pub const BATTLE_LOG_LINES: u32 = 100;
pub const REFUSAL_LOG_LINES: u32 = 20;
pub const CLASS_LOG_MAX: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sites::M6_PATCHES;

    fn play() -> Play {
        Play { kh1_battle: true, flags: bits::FREE | bits::GRANT, hp: 10, chosen: 3, handler: Handler::Generic, marluxia3_mode: 0, gate: Some(Gate::default()) }
    }

    #[test]
    fn cardless_does_what_the_play_and_message_2_do() {
        // Granted, free, a stale refusal bit and a stale sleight bit: all cleared; 1 and 4 set.
        let f = bits::GRANT | bits::FREE | bits::REFUSED | bits::SLEIGHT | 0x40_0000;
        assert_eq!(cardless_flags(f), 0x40_0000 | bits::PLAYING | bits::STARTED);
        assert_eq!(decide(&play()), Path::Cardless { flags: bits::PLAYING | bits::STARTED, state: 3 });
        // The state object message 2 points `+0x68` at.
        assert_eq!(state_address(0x1000, 3), 0x1000 + (3 + 0x13) * 0x40);
        assert_eq!(state_address(0x1000, 11), 0x1000 + 0x780);
    }

    #[test]
    fn re_com_stops_it_where_it_stops_the_card() {
        let mut p = play();
        p.kh1_battle = false;
        assert_eq!(decide(&p), Path::Recom(Why::Setting));
        for (lock, gimmick, duel) in [(0x4, 0, 0), (0x4000_0000, 0, 0), (0x8000_0000, 0, 0), (0, 1, 0), (0, 0, 2), (0, 0, 7)] {
            let g = Gate { lock, gimmick, duel };
            assert!(g.locked(), "{g:?}");
            let p = Play { gate: Some(g), ..play() };
            assert_eq!(decide(&p), Path::Recom(Why::Locked(g)));
        }
        // Bits CBTL doesn't test, and duel mode 1, don't lock.
        assert!(!Gate { lock: 0x3FFF_FFFB, gimmick: 0, duel: 1 }.locked());
        assert_eq!(decide(&Play { gate: None, ..play() }), Path::Recom(Why::NoCbtl));
        assert_eq!(decide(&Play { hp: 0, ..play() }), Path::Recom(Why::Dead));
        for s in [0, 1, 11, -1] {
            assert_eq!(decide(&Play { chosen: s, ..play() }), Path::Recom(Why::State(s)));
        }
        for s in 2..=10 {
            assert!(matches!(decide(&Play { chosen: s, ..play() }), Path::Cardless { .. }));
        }
    }

    #[test]
    fn message_handlers() {
        assert_eq!(decide(&Play { handler: Handler::Other, ..play() }), Path::Recom(Why::Handler));
        // Marluxia 3: the generic handler unless its mode bit 4 is set.
        assert!(matches!(decide(&Play { handler: Handler::Marluxia3, marluxia3_mode: 0x1B, ..play() }), Path::Cardless { .. }));
        assert_eq!(decide(&Play { handler: Handler::Marluxia3, marluxia3_mode: 4, ..play() }), Path::Recom(Why::Marluxia3Mode));
        // Bit 4 means nothing for the generic handler.
        assert!(matches!(decide(&Play { marluxia3_mode: 4, ..play() }), Path::Cardless { .. }));
    }

    #[test]
    fn attacking_counts_bit_0_or_an_attack_state() {
        assert!(attacking(bits::PLAYING, Some(0)));
        assert!(attacking(0, Some(2)));
        assert!(attacking(0, Some(10)));
        assert!(!attacking(0, Some(11)));
        assert!(!attacking(bits::STARTED | bits::REQUEST, Some(1)));
        assert!(!attacking(0, None));
    }

    #[test]
    fn patches_are_the_three_serialisers() {
        let rvas: Vec<u32> = M6_PATCHES.iter().map(|p| p.site.rva).collect();
        assert_eq!(rvas, vec![0x0017_D010, 0x0018_765F, 0x0017_F3DE]);
        for p in M6_PATCHES {
            assert_eq!(p.site.expect.len(), p.new.len(), "{}", p.site.name);
            assert!(p.site.expect.len() >= 5, "{}", p.site.name);
            assert_ne!(p.site.expect, p.new, "{}", p.site.name);
        }
        // P2: `push rbx; sub rsp,0x20` -> `mov eax,1; ret`: the gate always says yes.
        assert_eq!(M6_PATCHES[0].new, &[0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3]);
        // P3: `not rax; and rax,rcx` -> `xor rax,rcx; or ah,8`.
        assert_eq!(M6_PATCHES[1].site.expect, &[0x48, 0xF7, 0xD0, 0x48, 0x23, 0xC1]);
        assert_eq!(M6_PATCHES[1].new, &[0x48, 0x33, 0xC1, 0x80, 0xCC, 0x08]);
        // P4: only the `jge` (0x7D) becomes `jmp` (0xEB); `cmp ecx,0x64` is rewritten unchanged.
        let diff: Vec<usize> = (0..5).filter(|&i| M6_PATCHES[2].site.expect[i] != M6_PATCHES[2].new[i]).collect();
        assert_eq!(diff, vec![3]);
        assert_eq!((M6_PATCHES[2].site.expect[3], M6_PATCHES[2].new[3]), (0x7D, 0xEB));
    }

    #[test]
    fn grant_all_keeps_the_others_and_grants_them() {
        // What P3 computes, with rax = the 0x200 mask and rcx = flags that have 0x200 (the loop
        // only gets there after `test rax,rcx`): Re:CoM's `not rax; and rax,rcx` drops the
        // request; ours `xor rax,rcx; or ah,8` drops it and sets 0x800 (ah = bits 8..15).
        let grant_recom = |rcx: u64| !bits::REQUEST & rcx;
        let grant_ours = |rcx: u64| {
            let rax = bits::REQUEST ^ rcx;
            rax | (8u64 << 8)
        };
        for f in [bits::REQUEST, bits::REQUEST | bits::FREE, bits::REQUEST | 0x8000_0000_0000 | 4] {
            assert_eq!(grant_recom(f), f & !bits::REQUEST);
            assert_eq!(grant_ours(f), (f & !bits::REQUEST) | bits::GRANT);
        }
    }

    #[test]
    fn plan_is_all_or_nothing() {
        let p = M6_PATCHES;
        let r = Bytes::Recom;
        let o = Bytes::Ours;
        let w = plan(p, &[r, r, r], true).unwrap();
        assert_eq!(w.len(), 3);
        assert!(w.iter().all(|&(i, from, to)| from == p[i].site.expect && to == p[i].new));
        assert!(plan(p, &[o, o, o], true).unwrap().is_empty());
        let back = plan(p, &[o, r, o], false).unwrap();
        assert_eq!(back.iter().map(|w| w.0).collect::<Vec<_>>(), vec![0, 2]);
        assert!(back.iter().all(|&(i, from, to)| from == p[i].new && to == p[i].site.expect));
        assert!(plan(p, &[r, Bytes::Other, r], true).is_none());
        assert!(plan(p, &[r, r], true).is_none());
        assert_eq!(bytes_state(&p[0], p[0].site.expect), Bytes::Recom);
        assert_eq!(bytes_state(&p[0], p[0].new), Bytes::Ours);
        assert_eq!(bytes_state(&p[0], &[0xCC; 6]), Bytes::Other);
    }

    #[test]
    fn trace_cells() {
        assert_eq!(trace_enemy(7, Some(3), bits::PLAYING | bits::STARTED, 0), "7:s3:PS--:0");
        assert_eq!(trace_enemy(9, None, bits::REQUEST | bits::GRANT, 6000), "9:s?:--RG:6000");
        let t = trace_token(true, 2, Some(0), &["7:s3:PS--:0".into(), "9:s1:----:250".into()]);
        assert_eq!(t, "m6 battle=kh1 attacking=2 cbtl_slot2=0 enemies=[7:s3:PS--:0 9:s1:----:250]");
        assert_eq!(trace_token(false, 0, None, &[]), "m6 battle=recom attacking=0 cbtl_slot2=none enemies=[]");
    }
}
