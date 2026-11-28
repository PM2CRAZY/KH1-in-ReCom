# M6: battles at the KH1 setting (decisions 29, 30)

Steam addresses (Re:CoM image base `0x140000000`). Tags: Checked (decomp / exe / tests) or
Assumption. Code: `crates/kh1-recom` (`battle.rs`, `win/battle.rs`, stage 2 `deck.rs`,
`win/deck.rs`, stage 3 `boss.rs`; `sites.rs` `M6_*`). Research:
`m6-cardless-battles.md` and `recom-m6-patch-points.md` (kept with the decomp tools). This note
supersedes `recom-cardless-battle-design.md` §8 ("CBTL is the only gate").

## Status

- **Stage 1 built (0.12.0, not seen in game):** at Combat = KH1, in Sora's story, Heartless and
  Heartless bosses attack without cards, and several can attack at once, each as if alone.
- **Stage 2 built (not versioned yet, not seen in game):** at Combat = KH1, in Sora's story,
  Sora has no deck in battle: an empty revolver built by Re:CoM's own constructor, no Friend-card
  drops, the deck HUD hidden through Re:CoM's own event-hide mask. Sora still collects cards.
  § Stage 2 below.
- **Stage 3 built (not versioned yet, not seen in game):** at Combat = KH1, in Sora's story,
  bosses read KH1 swings and Counterattack as "Sora is attacking" (P5); human bosses keep their
  decks as a hidden move list; every enemy card, the enemy card cue, the enemy revolver's stock
  row and Marluxia 3's stock folder are hidden through Re:CoM's own event-hide mask. Open:
  Marluxia 3's card duel (Sora can't win it without cards), the enemy enchant gauge. § Stage 3
  below.
- **Enemy card sounds silent (2026-11-02, not versioned yet, not heard in game):** in the same
  battles, the enemy side's stock, Reload, deal and item-card page-switch sounds don't play.
  § Enemy card sounds below.
- **Attack-end cooldown (2026-11-02, not versioned yet, not seen in game):** in the same battles,
  ordinary Heartless still on Re:CoM's AI wait the cooldown their class passes to Re:CoM's
  attack end (6000 ticks = 120 frames) instead of Re:CoM's 250 (5 frames). § Attack-end cooldown
  below.
- **Parasite Cage (2026-11-03, not versioned yet, not seen in game):** in the same battles,
  Sora's hits never flinch it or cut its attacks (Re:CoM's own super armour), and it waits 9000
  ticks (180 frames, about 3 s) after each attack. Its chance faint only from Fire (not from
  finishers). § Parasite Cage below.

## Stage 1: what changes

### P1: the Heartless card play does message 2's work

Re:CoM's path for one Heartless attack (Checked (decomp)):

1. The think (vtable `+0x3D0`, mostly `0x140196EC0`) picks an attack state into `+0x278`, asks
   the gate `0x14017D010` and sets the request `+0x260 |= 0x200`.
2. The battle enemy manager's grant `0x140187600` turns one request per frame into `0x800`.
3. The enemy update `0x140185320` calls vtable `+0x98` = **`0x140198870`** (the card play) when
   `0x800` is set. It clears `0x802`, computes a card value (`+0x448`) and calls
   `0x1401634B0(CBTL, id, card, value)`, which refuses while CBTL is locked and otherwise sets a
   side-2 card (`0x140163830`) and its cue sprite. Success sets bit 0.
4. When the card is in place, CBTL posts message 2. The handler `0x140181620` case 2 clears
   `0x200` and, when this enemy owns side 2, sets `+0x260 = (& ~0x10) | 4` and the next state
   `+0x68 = enemy + (+0x278 + 0x13) × 0x40`; bit 29 follows the record's sleight id.
5. Re:CoM's state code runs the attack and ends it with `0x14017E540` (cooldown `+0x2C8`, bit 0
   cleared, CBTL slot freed if this enemy owns it).

Our wrapper on `0x140198870`, in a KH1 battle (decided at battle start, below):

| Test | Then |
|---|---|
| HP > 0, message handler known (below), CBTL present and not locked, `+0x278` in 2..10 | `+0x260 = (+0x260 & ~(0x800 \| 2 \| 0x200 \| 0x10 \| 0x2000_0000)) \| 1 \| 4`, `+0x68 = enemy + (+0x278 + 0x13) × 0x40`, return 1 |
| CBTL locked (`+0x208 & 0xC0000004`, `+0x1E4 != 0`, or `+0x20C >= 2`, exactly `0x1401634B0`'s tests) | Re:CoM's card play, which refuses as always (`0x200` cleared, bit 1 set, `+0x418`) |
| Anything else (no CBTL, unknown handler, Marluxia 3's mode below, a chosen state outside 2..10) | Re:CoM's card play, logged once per class |

- Re:CoM's own state code then runs, draws, collides and ends the attack. No card value, no CBTL
  slot, no `CBattleCard`, no cue sprite. Once per granted attack; nothing per frame.
- **Message handlers** (Checked (exe): the 49 vtables holding `0x140198870` at `+0x98`): 48 use
  the generic `0x140181620`. `CBossMarluxia3` uses `0x1403F4550`, which runs the generic handler
  only while its `+0xCE4 & 4` is clear (Checked (disasm)); with the bit set it ignores message 2,
  so the wrapper leaves Re:CoM's card play there.
- Correction: the research counted 50 users of `0x140198870`; the 50th data reference is its
  `.pdata` entry. Checked (exe).
- The 49 classes: every `CEnemy*` with a card play (33), `CHeartless`, `CSampleEnemy(2)`,
  `CTutorialHood`, Guard Armor, Parasite Cage, Trickmaster, Jafar, Oogie, Ursula, `CBossMDragon`,
  Dark Side, Marluxia 2, Marluxia 3, the 100 Acre Wood bee and beehive. Checked (exe, RTTI).
- Its only caller is the enemy update (`0x14018537C`, the only `call [reg+0x98]` in the enemy
  code). Checked (exe).
- The wrapper moves exactly the 15 prologue bytes `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20`;
  the next instruction (`mov rax,[rip+…]` at `0x14019887F`) is RIP-relative and stays. No branch
  in the exe and no data pointer lands inside them (only the function start: 49 vtables and
  `.pdata`). Checked (exe).

What the skipped steps gave Re:CoM (Checked (decomp) unless tagged):

- The card value `+0x448` (`0x140198860` → `0x1401983D0`, Oogie `0x1403D91C0`): only the card's
  value; it draws from Re:CoM's global random number, so the random sequence differs from
  Re:CoM's (no gameplay rule depends on the sequence; Assumption).
- Message 1's record copy (`+0x318..+0x338`): no reader found (research, Assumption).
- Bit 29 is cleared, as message 2 does for a card record without a sleight id (Assumption: a
  Heartless record never names one). It only matters for `0x14017E540`'s enchant-0x72 repeat.
- Re:CoM starts the attack one or two frames after the grant (CBTL update, message); P1 starts it
  in the grant frame's update. Assumption (not traced). Re:CoM's own code writes `+0x68` from the
  update path the same way (`0x14017E540`, the hit handler).

### P2–P4: Re:CoM's other serialisers, switched once per battle

| Patch | Site | Re:CoM | Ours | Effect |
|---|---|---|---|---|
| P2 | `0x14017D010` (think gate, 6 callers, all `test eax,eax`) | `40 53 48 83 EC 20` | `B8 01 00 00 00 C3` (`mov eax,1; ret`) | no "wait while a side-2 card plays", no "not the last granted twice in a row" |
| P3 | `0x14018765F` (grant loop of `0x140187600`; rax = mask `0x200`, rcx = flags with it) | `48 F7 D0 48 23 C1` (`not rax; and rax,rcx`) | `48 33 C1 80 CC 08` (`xor rax,rcx; or ah,8`) | every requester's `0x200` becomes `0x800` instead of being dropped; the nearest is still stored in manager `+0xF0` |
| P4 | `0x14017F3DE` (think rate `0x14017F380`) | `83 F9 64 7D 26` (`cmp ecx,0x64; jge`) | `83 F9 64 EB 26` (`jmp`, same target `0x14017F409`) | the think rate is no longer multiplied by the number of enemies; the ×10 after a hit (`r8d`, set before the branch) stays |

- Checked (exe): the bytes, the masks (`0x14052A458` = 0x200, `0x14052A468` = 0x800), the new
  bytes disassembled, no branch or data pointer into any patched byte. Checked (decomp): P2's
  body is side-effect free (`0x1401616A0`, `0x140186020` only read).
- **Switch point:** a wrapper on the battle enemy manager's ctor `0x1401859B0` (`48 89 4C 24 08
  55 56 57 41 54`), called once per battle by the battle build `0x1401AEBF0` and the 100 Acre
  Wood build `0x14042D710`, before any battle enemy updates. The three patched functions only
  run for battle enemies (the gate and the grant need the manager; Checked (decomp)). It decides
  "KH1 battle" = Combat = KH1 and not Riku's story (`0x1401AAAE0`), then writes P2–P4 to match:
  each site must hold Re:CoM's or our bytes, else nothing is written (all or none, checked
  writes, undone on a failed write). At the Re:CoM setting the bytes are Re:CoM's own, byte for
  byte, from the next battle on. P1 follows the same decision for the whole battle.
- **P4 follows `enemy_pace`** (0.13.2, decision 32; `battle::wanted`): in a KH1 battle P2 and P3
  are ours, and P4 is ours only with `enemy_pace = kh1`. With `enemy_pace = recom` (the default)
  Re:CoM's crowd sharing stays: each ordinary enemy's think rate is multiplied by the number of
  enemies, so a crowd shares one attack budget, while several enemies can still attack at once
  (no card slot, P2, P3). Each site is still checked (Re:CoM's or our bytes, else nothing is
  written). Re:CoM setting: all three Re:CoM's.
  - The revolver build `0x1401F43D0` (stage 2's switch) was not used: it is skipped when
    `player+0x418 & 0x20000000`, and the enemies' code doesn't depend on Sora's deck.
  - Minigame rooms (`0x140235360() != 0`) build no manager: no grant, so P1 is never reached
    there; the gate dereferences the manager without a null test, so it isn't run without one;
    the think rate skips the enemy count without a manager, so P4 changes nothing there
    (Checked (decomp)). The decision of the last battle stays meanwhile.
  - Assumption: the battle build runs on the game thread, so no enemy runs the patched code
    while it is written.

### What other code reads, at KH1 (Checked (decomp))

- **Bit 0 keeps its meaning "in an attack"**: P1 sets it, `0x14017E540` / the state end
  `0x14017DA90` clear it. The think stays off during the attack (`0x140185320`).
- **Decision 14 (KH1 hits land) and 28 (turned-back shots)**: the enemy-hit wrapper still adds
  `0x2000000` while bit 0 is set; Re:CoM's `0x2000001` path calls `0x14017E1D0`, which returns 0
  (this enemy owns no side-2 card: `0x140160950(2)` answers only while `+0xBC != 0`), then clears
  bit 0 and takes the hit. So the CBTL part is a no-op; clearing bit 0 is still needed.
- **Decision 27 (Guard reactions)**: the flinch path's `0x14017E1D0` is a no-op the same way, and
  the log says "bit cleared (not CBTL's now-playing)".
- **Decision 26 (`hurt_off`)**: unchanged; it runs when the attack state starts.
- Dark Side (`0x1403E7A30`) waits for children owning side 2: they never do, so it acts while its
  children attack (research §3, KH1-like).
- Exception, Assumption: Marluxia 3 may start a P1 attack while its own sleight card holds side 2;
  a KH1 hit then ends that sleight card through the same `0x2000001` path.

### Human bosses (stage 3, left as Re:CoM)

- `CHumanBoss` users (Leon, Cloud, Hades, Vexen, Riku, Larxene, Axel, Hook, Marluxia 1, and
  Riku's story's) never call `0x140198870` (their `+0x98` is `0x1401829D0`, which clears `0x802`).
  Their deck, CBTL plays and message 2 are untouched.
- P3 can reach them only when a human boss's request is not the nearest one: Re:CoM drops its
  `0x200`; P3 grants it, and `0x1401829D0` then clears it again. The only difference is bit 1
  cleared, which nothing reads (Checked (exe): the mask `0x14052A418` is only OR-ed / AND-ed).
  Solo boss fights are identical. Assumption: Sora's-story human-boss fights are solo.
- P4 changes a human boss's think rate only with 2+ enemies and `+0x128 % 10000 < 100`.
  Assumption: boss ids are ≥ 100 (research) and the fights are solo.
- Tutorial Leon still never attacks at KH1 (P5, action `0xB`, stage 3). Not new.

## Attack-end cooldown (Re:CoM-AI Heartless)

Owner (2026-11-02, "Enemy pace, Re:CoM-AI Heartless" in `design-philosophy-instructions.md`):
every Heartless not yet ported to KH1's AI waits a longer cooldown before each attack, its
class's own value. Code: `crate::battle::pace` (rule, class table, tests), `win/battle.rs`
`attack_end` / `install_pace`, sites `M6_PACE_HOOKS` / `M6_PACE_CHECKED`.

### Re:CoM (Checked (decomp + exe))

- `0x14017E540(enemy, next, cooldown)`: unless enchant 0x72 repeats the attack (early return,
  bit 0 kept), ends the enemy's card (`0x14017E1D0`), clears bit 0, writes `cooldown` to `+0x2C8`
  (`0x14017E5C8`), then with `next` set makes it the next state; **with `next == 0` it writes 250
  over it** (`mov dword [rbx+0x2C8], 0xFA` at `0x14017E5F5`) and goes to the default state
  `+0x2F8` (think on, `0x10`).
- **All 198 call sites pass `next = 0`** (`xor edx,edx`), so Re:CoM never keeps the value: 197
  pass 6000 (`mov r8d, 0x1770`), one passes 0 (`0x1403FBAEC` in `0x1403FBA20`, Marluxia 2's
  shared tail). Checked (exe: every site disassembled; callers mapped to classes through their
  vtable slots and RTTI).
- The enemy update `0x140185320` counts `+0x2C8` down by the task step `+0x14` (50 a frame) and
  runs the think (`+0x3D0`) or the granted card play (`+0x98`) only while it is < 1.
- Other writers of 250 (not changed): the default state `0x14017D6F0` (68 callers: appearance,
  flinch / reaction ends) and `0x14017D260`, `0x1401838C0`, `0x140184C80`, `0x140184CE0`,
  `0x1401851E0`. So an attack Sora interrupts (the hit path clears bit 0 itself, Stage 1) keeps
  Re:CoM's 250 after the flinch. Checked (decomp) for the writers; which one each reaction ends
  in: Assumption (not traced).

### At KH1 (Combat = KH1, Sora's story: the battle decision of P1)

A wrapper on `0x14017E540` (15 bytes moved: `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20`; no
branch or relocated pointer lands inside them, Checked (exe)) calls Re:CoM's attack end, then
writes the passed cooldown back when all of these hold (`pace::kept`):

| Test | Why |
|---|---|
| the battle is a KH1 one | Combat = Re:CoM and Riku's story untouched |
| the enemy's vtable is one of the 32 classes below, **Re:CoM's own** | bosses, the bee and our KH1-AI enemies (they run a copy of the vtable) keep Re:CoM's |
| `next == 0`, bit 0 now clear and `+0x2C8 == 250` | Re:CoM took its "no next state" path (not enchant 0x72's early return) |
| passed > 250 | never shorter than Re:CoM's (our KH1 enemies call it with 0) |

Once per finished attack, nothing per frame. The 32 vtables are checked against their RTTI names
before the hook goes in (else not hooked, Re:CoM's 250 for all).

| Class (Re:CoM) | Vtable | Passed (ticks / frames) | States calling it |
|---|---|---|---|
| Airpirate, Airsoldier, Aquatank, Bandit, Barrelspider, Blackfungus, Bluerhapsody, Bouncywild, Creapplant, Darkball, Defender, Fatbandit, Gargoyle, Largebody, Neoshadow, Pirate, Powerwild, Rednocturne, Screwdiver, Seaneon, Searchghost, Shadow, Soldier, Spadesoldier, Tornadostep, Wightnight, Wyvern, Yellowopera (`CEnemy…`) | `crate::battle::pace::HEARTLESS` | 6000 / 120 | states 2 and 3 (`+0x120`, `+0x138`) |
| Greenrequiem, Heartsoldier | as above | 6000 / 120 | state 2 only |
| Loudness, Wizard | as above | 6000 / 120 | states 2, 3, 4 |

The 12 classes we port to KH1's AI (`kh1_enemy::CLASSES`) are in the list: they get it only while
they run Re:CoM's AI (`enemy_ai = recom`, or no manifest). Not in the list: `CEnemyWhitemushroom`
(no attack end), `CEnemy100AcreBB_Bee` (6000; 100 Acre Wood, not a Heartless).

**Left as Re:CoM (all go through the same attack end, all discarded to 250):** Heartless bosses
Guard Armor, Trickmaster, Jafar, Oogie, Ursula, Dragon Maleficent, Dark Side,
Marluxia 3 (6000 each); Marluxia 2 (0); the human bosses' own states and `CHumanBoss`'s shared
`0x14019AA00` (Leon, Cloud, Hades, Vexen, Riku, Larxene, Axel, Hook, Marluxia, Lexaeus, Zexion,
Ansem: 6000). Checked (exe). The Parasite Cage (6000 passed) waits its own 9000 instead
(§ Parasite Cage).

Logs: at start `m6: attack end 0x14017E540 wrapped: …` (or `not wrapped`); once per class
`m6: class CEnemyFatbandit: waits its own attack-end cooldown at KH1: 6000 ticks (120 frames)
instead of Re:CoM's 250 (5 frames)`. F9 trace: the `m6 … enemies=[id:s<state>:<flags>:<cooldown>]`
cells show it counting down from 6000.

Not checked: anything in game. Assumption: the gap between attacks grows by about 115 frames
(the research's ~25-frame gap at KH1 + 115); the think's own 1-in-20 roll still follows.

Owner test: a battle with a Fat Bandit (or any unported Heartless) at Combat = KH1: after each
attack it waits about two seconds before the next; the log shows the class line; the F9 trace's
cooldown starts at 6000. Then a boss (Guard Armor) and Combat = Re:CoM: as before (cooldown 250).

## Parasite Cage

Owner (2026-11-03, "Parasite Cage" in `design-philosophy-instructions.md`): **"the parasite cage
shouldnt flinch when you attack, let it go through with its attacks, give it about 3 seconds
between each attack"**. At Combat = KH1 in Sora's story (the battle decision of P1); Combat =
Re:CoM and Riku's story unchanged. Code: `crate::battle::cage` (rules, tests), `win/battle.rs`
(`cage_armour_on` / `cage_armour_off`, `attack_end`, `install_cage`), the enemy-hit wrapper
`win::attack::enemy_hit`; sites `CAGE_CHECKED`, `CAGE_PTRS`. Owner (2026-11-03, on 0.95.0, "Parasite
Cage faint / card"): **its chance faint only from Fire at Combat = KH1, not from Sora's
finishers**: its hit callback's vtable slot (`cage_hit_callback`, `cage::callback`; sites
`CAGE_FAINT_CHECKED`, `CAGE_FAINT_SLOT`); § "Faint only from Fire" below.

### Re:CoM's Parasite Cage (Checked (decomp + exe) unless tagged)

- **One body**, `CBossParasiteCage` (vtable `0x140669220`, RTTI checked at start). Its states
  (slot `+0xE8 + 0x18·s`): 0 appearance (`0x1403C29F0`), 1 idle (begin `0x1403C2E00`: think on
  `0x10`, bit 21 cleared; step `0x1403C53A0`: turns to face Sora), attack states 2..5 (begins
  `0x1403C2A40` / `0x1403C2AD0` / `0x1403C2B20` / `0x1403C2B30`, motions e110_052/053, 054, 050,
  051/055; steps `0x1403C48F0`, `0x1403C4930`, `0x1403C4A10`, `0x1403C5220`; state 4 calls
  `0x14024E630` at motion time > 6500 (`+0xCD4 |= 1`; Assumption: the grab that holds Sora, which
  message 6 lets go) and later shoots from `mouth3` (`0x140158DD0`)),
  13 the card-break stagger (`0x1403C2B70`, its gimmick-card drop), 17 its **faint** (begin
  `0x1403C2DD0`, step `0x1403C5290`, end `0x1403C3560`), 30 the defeat (`0x1403C2CF0`). Think
  `+0x3D0` = `0x1403C26F0`, hit callback `+0x3E0` = `0x1403C4060`, message 6 `0x1403C43C0`.
- Its think and message 6 also move room objects of ids 550..554 (task list `0x11`, `+0x2D4`,
  `+0x2C0`): Assumption that these are scenery parts, not enemies Sora hits (not traced); nothing
  here touches them.
- **What a hit did at KH1 before this change**: the hit handler `0x14017F7E0` drops a hit while
  `+0x260 & 1` (in an attack) unless the card-interrupt bit `0x2000000` is set; our wrapper sets
  it for KH1 swings (decision 14) and its hurt bodies stay on in attacks (decision 26), so the
  handler ends Re:CoM's "card" (`0x14017E1D0`, bit 0 cleared), applies the damage, calls the hit
  callback, then the reaction picker `0x14017D260`: kind 1 (KH1's ordinary hits,
  `attack::recom_kind`) → the flinch (state 11, `+0x780`); kind 2 (KH1's finishers) → the flinch
  or the blow-away (state 12, `+0x7C0`) by its `@ENP +0xC`. Either replaced its attack state.
  The cage has no attack state that sets bits 20 / 21 itself.
- **Its faint** (hit callback `0x1403C4060`, before the picker, while `+0xCD4 & 4` is clear): a
  Fire hit (attribute `+0x84` = 2) by its parameter `FireFaint` %, or a kind-2 hit (`+0x80` = 2:
  every KH1 finisher) by `AtkFaint` %: state 17 (`0x14017D1F0`) and bit 21 set by itself (so the
  picker only plays the voice). In the faint (`+0xCD4 & 4`, bit 19) each hit knocks out prizes
  (kind 0xD2, `MDSBNM` + `rand % (MDSBNMR + 1)`, `BDSBNM…` for kind 2). Its parameter file is not
  in this session: the chances and counts are not known.
- **What the faint test reads** (Checked (exe + disasm + decomp)): the hit body
  `*(cage+0xF8)+0x50` (the hurt body Re:CoM's handler gathered; `mov rbp,[rax+0x50]` at
  `0x1403C4091`), the faint bit `test byte [rcx+0xCD4],4` (`0x1403C4080`), then two independent
  tests in order: `cmp [rbp+0x84],2` (`0x1403C42B4`, the attribute: Fire → `FireFaint` roll) and
  `cmp [rbp+0x80],2` (`0x1403C432C`, the Re:CoM hit kind: kind 2 → `AtkFaint` roll). Each roll
  is `rand % 100 <= chance` → state 17 via `0x14017D1F0` and bit 4 cleared, bit 21 set. Nothing
  else runs outside the faint: no card flag or reaction is read. A Fire kind-2 hit rolls both.
  Kind 2 is every KH1 combo finisher (`attack::recom_kind`: KH1 kind 5 → 2). Callers of slot
  `+0x3E0`: the hit handler `0x14017F7E0` (before the picker) and Re:CoM's stop exe
  `0x1401843A0` (a held hit landing); the only data ref to `0x1403C4060` is the cage's slot
  `0x140669600`.
- **Our KH1 Fire before this change** (Checked (code + decomp)): its shots carry Re:CoM's Fire
  attribute 2 (`win::magic` `RECOM_ATTR`), but at Damage = KH1 `spell_hit` writes the body's
  attribute as `0xFF` for the handler's call (Re:CoM's resistance not stacked on KH1's), so the
  callback never saw Fire: our KH1 Fire could not make it faint at Damage = KH1.
- **Its pacing**: all four attack states end with `0x14017E540(cage, 0, 6000)` (`0x1403C490D`,
  `0x1403C49EE`, `0x1403C5149`, `0x1403C523D`; the only four calls in its code), which writes 250
  over the 6000 (`0x14017E5F5`). Its update is Re:CoM's `0x140185320` (vtable `+0x80`), which
  counts `+0x2C8` down by the task step `+0x14` (50 a 60 Hz frame; 100 a frame at 30 fps, the same
  time; `knowledge/ReCoM/recom-timing.md`) in every state and runs the think only while it is < 1 with the
  think bit `0x10`. The think: above 30 % HP an attack 1 in `0x14017F380` (its `@ENP +0x98 +
  level × 4`, × 10 while `+0x2D0` ≥ 1) per call; below 30 % at once while Sora attacks (P5), else 1
  in 30; then state 2..5 by distance and angle (and 41 % a state 3 once it lost 25 % of its max HP
  since its last state 3, while its card is unused).

### Built

- **No flinch**: for each call of the hit handler on the cage (Re:CoM's vtable, checked), the
  enemy-hit wrapper sets Re:CoM's super armour `+0x260` bits 20 / 21 for the call and puts both
  back after (`kh1_core::enemy::host::RECOM_ARMOUR`, the same armour KH1's no-reaction enemies take;
  § "Hits that take no reaction" in `recom-m11-enemies.md`). The picker then plays only the damage
  voice and asks for no state, so the damage lands and its attack state goes on (bit 0 is still
  cleared by the card-interrupt path, as before; its attack state, not bit 0, runs the attack, and
  its think stays off until the attack end, bit `0x10`). Every hit counts: swings, spells, shots
  Sora's Guard turned back.
- **Kept Re:CoM's**: Stop (kinds 8 / 0x17), stun (7), Gravity's flatten (0xD), freeze (0x11), the
  other status kinds, the defeat, its faint from Fire (it writes state 17 itself), the card break (message 3 → state 13), message 6 (the used gimmick card lets Sora go).
  Bit 21 the faint sets is cleared again after the call: in state 17 the picker takes its bit-19
  path and never reads it, and its state 1 begin clears it anyway (`0x1403C2E4B`).
- **Guard**: a call in which Sora's Guard blocked the cage this frame takes no armour
  (`win::guard::block_pending`), so the Guard reaction (Re:CoM's flinch through the picker, Guard
  reactions on) and the Guard card break stay as they were.
- **About 3 s between attacks**: the attack-end wrapper (§ Attack-end cooldown) writes
  `cage::WAIT` = 9000 ticks (180 frames) after Re:CoM's attack end for the cage, on Re:CoM's "no
  next state" path (bit 0 clear, 250 written). **What it measures**: from the frame its attack
  state ends (the attack end's call, at its motion's end) to the first frame its think may start
  the next attack. Then Re:CoM's own time adds on: the idle turn toward Sora (think bit off while
  it turns) and the think's roll (unknown above 30 % HP, its `@ENP` rate; none below 30 % while
  Sora attacks). A faint or a card-break stagger ends through Re:CoM's default state (250), not an
  attack end: the wait follows attacks only.
- **Faint only from Fire** (owner, 2026-11-03, on 0.95.0): the cage's vtable slot `+0x3E0`
  (`0x140669600`, old pointer `0x1403C4060` checked; the callback's first 17 bytes `4C 8B DC 55 57
  48 81 EC A8 00 00 00 41 0F 29 73 D8` and its three reads checked from the Steam exe; no code
  bytes move) holds `win::battle::cage_hit_callback`, which calls Re:CoM's. In a KH1 battle,
  outside its faint (`+0xCD4 & 4` clear), by `cage::callback`:
  - attribute 2 (any Fire Re:CoM put on the body): Re:CoM's callback as is (both rolls if kind 2);
  - attribute `0xFF` and the body's attacker (`+0x98`) one of our KH1 Fire shots
    (`magic::shot_spell`, the last 32 shot ids): the attribute reads 2 for the callback's call and
    `0xFF` again after: Re:CoM's `FireFaint` chance (Re:CoM's resistance still not stacked);
  - kind 2, not Fire: the callback is not called. Outside the faint its only work for that hit
    is the `AtkFaint` roll, so this is the same as the body presenting kind 1 for the call (no
    roll, no state, no flag; the handler's own reads of the kind, the picker included, are
    untouched);
  - anything else: Re:CoM's callback (it rolls nothing).
  In its faint (prizes, `BDSBNM` for kind 2) and at Combat = Re:CoM: Re:CoM's as is. The rule
  takes every hit source (swings, sleights, party, shots Guard turned back), not only Sora's
  swings: the owner's "only from Fire".
- **Logs** (once per battle): `m6: Parasite Cage (battle n): Sora's hits land on Re:CoM's super
  armour …` at its first landed hit; `m6: Parasite Cage (battle n): waits 9000 ticks (180 frames,
  about 3 s) …` at its first attack end; `… a finisher-type hit (kind 2, not Fire) takes no faint
  roll …` at the first one skipped; `… our KH1 Fire's hit rolls Re:CoM's FireFaint …` at the first
  KH1 Fire hit outside its faint. At start `m6: Parasite Cage hit callback 0x1403C4060 wrapped …`
  (or `… not as read: its faint stays Re:CoM's`), and `m6: Parasite Cage (CBossParasiteCage
  0x140669220) checked: …` (or `not as read`: it stays Re:CoM's).
- **Per frame**: no new code hook; per call of the existing enemy-hit wrapper, one atomic load (two
  in a KH1 battle) and one vtable read. The callback wrapper runs only on hits that land on the
  cage: one atomic load at Combat = Re:CoM, a few reads in a KH1 battle (a lock only for an
  `0xFF` attribute).

### Gimmick-card triggers (`recom-gimmick-cards.md`)

- **50 % HP**: unchanged (the damage lands as before).
- **Guard block**: unchanged (no armour for that call; the block posts message 3 as before).
- **Finisher interrupt**: still possible. It fires when a KH1 finisher lands while bit 0 is set and
  bit 0 is clear after the handler; the card-interrupt path still clears it under the armour. As
  before, only the first of Sora's hits landing in one of its attacks finds bit 0 set (before, that
  hit flinched it; now it only clears bit 0), so it fires when a finisher is that first hit: as
  rare as before. The break's stagger (state 13) does cut that attack: it is Re:CoM's card break.

### Not checked / Assumptions

- Assumption: a KH1 Fire hit held by Re:CoM's stop and landed later by its stop exe still finds
  its shot id among our last 32 (else it rolls nothing, as before this change). Not traced.
- Nothing seen in game. The faint chances (`FireFaint`, `AtkFaint`) and the think rate are in its
  parameter file, not in this session.
- KH1's own Parasite Cage (reference): its script `pi_3000.bd` (and `pi_3000_00.bd`) is in the
  player's `xa_pi_3000.mdls`; not decoded this session, so nothing here is KH1's number: the 3 s
  and the no-flinch are the owner's choice. Its KH1 stats row: `recom-kh1-attack.md` (Parasite
  Cage (110)).

### Owner test

1. Combat = KH1, Monstro, the Parasite Cage: hit it during each of its attacks (combo, finisher,
   Fire / Blizzard / Thunder): damage and the damage voice, no flinch or knock-back, the attack
   (the grab and spit included) plays to its end. Log: the `Sora's hits land on Re:CoM's super
   armour` line once.
2. After each attack it waits about 3 s before the next (below 30 % HP while you attack: right
   after the 3 s). Log: the `waits 9000 ticks` line once; F9 trace `enemies=[…:<cooldown>]` starts
   at 9000 after each attack.
3. Stop, Gravity and stun still take it; it can be beaten; a used gimmick card still lets Sora go.
4. Its card: at 50 % HP it drops it; blocking its attack with Guard staggers it (state 13) and rolls
   it; a finisher as the first hit landing in an attack breaks it.
5. Its faint only from Fire: many combos with finishers, no Fire: it never faints. Log: the `a
   finisher-type hit (kind 2, not Fire) takes no faint roll` line once. Then Fire (the KH1 spell)
   on it repeatedly: it may faint (Re:CoM's own chance, note how often); log: the `our KH1 Fire's
   hit rolls Re:CoM's FireFaint` line once. In its faint, hits still knock out prizes.
6. Combat = Re:CoM: as before (cards; 250 between attacks; finishers and Fire may make it faint;
   no `Parasite Cage` lines in battle).

## Oogie Boogie

Owner (2026-11-03, "Oogie Boogie" in `design-philosophy-instructions.md`): **"oogie boogie is
impossible to defeat. you're supposed to break one of his attacks which lowers a gate where you
can hit him. can we make it so that if he uses the attack that you're supposed to break twice
without being broken, the gate lowers?"**, then corrected: the ask re-sent without "twice", and
**"After once"**: each time the attack ends unbroken, the gate lowers. At Combat = KH1 in Sora's
story (the battle decision of P1); Combat = Re:CoM and Riku's story unchanged. Code:
`crate::battle::oogie` (rule, tests), `win/battle.rs` (`attack_end` → `oogie_after_end`,
`oogie_broken_begin`, `install_oogie`); sites `OOGIE_CHECKED`, `OOGIE_PTRS`, `OOGIE_BROKEN_SLOT`.

### Re:CoM's Oogie Boogie fight (Checked (decomp + exe) unless tagged)

- **His class** `CBossBoogie` (vtable `0x140670A60`, RTTI checked at start). States (slot
  `+0xE8 + 0x18·s`): 0 the scripted start (`0x1403D7740` / `0x1403D9250`), 1 idle and moves
  (`0x1403D7EE0` / `0x1403D9E70`), **2 the dice throw** (begin `0x1403D7830`, step `0x1403D9340`),
  3 another throw (`0x1403D7880` / `0x1403D9510`; dice with `+0xCB8` = 1), 4..8 his other attacks (knives `0x1403D78C0`,
  saws `0x1403D7900`, Heartless `0x1403D7940`, healing `0x1403D7980`, a close attack `0x1403D7A00`;
  meanings from their parameter names and objects: Assumption), **13 the card-break stagger**
  (begin `0x1403D7A40`), 30 the defeat. Think `+0x3D0` = `0x1403D74A0`. Handler `+0x48` =
  Re:CoM's base `0x140181620`.
- **The attack to break is state 2, his dice throw.** Its begin sets the mark `+0xCD8 |= 1`
  (motion 0x33); its step throws three dice (`CBossBoogieDice`, kind 0xC id 0xD1, `0x1403D7110`, at
  motion frame 0xED8; it also rolls the result `+0xCE0` from the `Dice1..Dice6` weights) and ends
  through Re:CoM's attack end `0x14017E540(boss, 0, 6000)` at `0x1403D9402` (its only call) once a
  die has landed (`+0xC88 & 2`) or all are gone, or after his stomp (the low-HP variant, `+0xCD8 &
  0x20`, `0x1403D5B70`). Unbroken, his state 1 then plays his reaction to the roll (motion 0x37) and
  his think picks the roll's attack (states 5 / 6 / 4 / 7). His state 1 begin clears the mark.
- **What a break does** (state 13 begin `0x1403D7A40`): the stagger (`0x14017BAA0`), then, only
  while the mark is set: the dice (thrown now if none is out) marked `+0xC88 |= 0x41` (such a die
  can drop his gimmick card when destroyed, `GMCPar` %, `0x1403D7C90`), and **the gate's count**
  `*(boss+0xD10)+0x2E0` + 1, at most 3. At 3: his down time `+0xCF8` = `HDownTime`, `+0x260 &=
  ~0x2000` (Assumption: the bit that keeps him from being hurt; it is set again when he jumps
  back), `+0x154 |= 0x10000`. The mark is cleared. **No other attack's break touches the gate.**
- **The gate** (Assumption: object id 0x28D is `CGimmickBBGHedge`, vtable `0x140671318`: its step
  `0x1403DA7C0` is the only reader of `+0x2E0` against its shown step `+0x2E4`): each count above
  the shown step lowers it one step (motions 2, 3, 4); after the third its collision goes off
  (`0x1401A64B0(hedge, 0)`): the way to him is open. **So Re:CoM needs three breaks** (or his
  gimmick card, `0x1403D8FA0`: count + 3 at once, his down time plus a bonus by the shown step, and
  he plays motions 0x46 / 0x47 for it: Assumption, a stun). Count back to 0: Sora is pushed out (z ≥ −7.5), collision on, the gate rises (motion 5).
- **The gate rises** when his down time runs out (counted down by his update `0x1403DB770`): he
  jumps back with a shockwave (`0x1403D9E70` case 5 → `0x1403D5B70` step 5 sets the count to 0,
  unless Sora's action is 0x20 that frame), and at the fight's scripted start (event variable 0xE1
  = 4). While the gate is down (shown step ≥ 3) his think throws no dice (only his close attack).
- **How often he throws**: idle, no dice out and no roll pending: 4 in 5 a state 2, 1 in 5 a state
  3 (`rand % 5`), while none of his summoned Heartless is alive; with some alive, state 3 (or
  nothing right after his Heartless attack). After an unbroken throw, the roll's attack first.
- **At Combat = KH1 before this change** (Checked (code)): nothing breaks him. Sora plays no cards
  (decision 29), and our KH1 break triggers (finisher interrupt, Guard block) and the HP drops are
  for the five card-break bosses only (`win::gimmick`, by vtable: `gimmick::BREAK_BOSSES`). His
  dice could still drop no card (only marked dice do), so the gate never lowered: the owner's
  "impossible to defeat".

### Built

- **Where**: the attack-end wrapper (`0x14017E540`, § Attack-end cooldown), after Re:CoM's call, in
  a KH1 battle, for Oogie's vtable only: his current state (`+0x60`) is state 2 (the only caller
  there is his step's `0x1403D9402`), the attack really ended (`+0x260 & 1` clear: not Re:CoM's
  enchant-0x72 repeat), HP above 0, the gate object present, the mark set, the gate not already
  down (count < 3).
- **The count**: each such end counts one unbroken throw; at `oogie::THROWS_TO_LOWER` (1, the
  owner's "once") the break runs and the count starts again. It also starts again at any state-13
  begin (a real break or ours) and in a new battle. With 1 nothing is ever carried over.
- **The break: Re:CoM's own.** His message handler (`+0x48` = `0x140181620`, checked) is called with
  **message 3 and no record**, as CBTL's message manager delivers a broken card (the five bosses'
  triggers post the same message; here it is called at once, because a posted message would arrive
  after his state 1 begin cleared the mark: Assumption on the order, not traced). Its case 3
  (`0x140181739`, checked): `+0x418` (a `ret` for him), `+0x260 &= ~0x201`, his attack collision
  ends, `+0x260 &= ~0x10`, next state 13 over the attack end's state 1. So his state 2 goes
  straight to his state-13 begin with the mark still set, and **Re:CoM's begin does all of it**:
  the stagger, the dice marked (a card may drop from them), the gate one step down, the down time
  at the third. Nothing of his is written by us.
- **What differs from a break in a card battle**: there a break comes while the throw plays (the
  card clash), so the roll's attack never follows. Here the throw has played out unbroken, so its
  roll's reaction and attack still follow the stagger (his state 1 reads `+0xCD8 & 2`, which state
  13 leaves). The owner's rule: the attack was used.
- **His state-13 slot** (`+0x220`, `0x140670C80`, old pointer `0x1403D7A40` checked; no code bytes
  move) holds `win::battle::oogie_broken_begin`, which calls Re:CoM's begin as is and logs what it
  did to the gate (the count before and after, the down time). Combat = Re:CoM: Re:CoM's, no line.
- **Per frame**: nothing. Per attack end of any enemy in a KH1 battle: one load and compare (his
  vtable); for him a few reads. Once per break: the handler call and the begin's log.
- **Logs** (`oogie (battle n): …`, at most 100 a battle): `his dice throw (state 2) ended unbroken
  (1 of 1): Re:CoM's card break (his handler, message 3 → state 13) … (count c of 3, shown step s)`;
  then `state 13 (card break, ours (an unbroken throw)): Re:CoM's state-13 begin lowered the gate:
  count c -> c+1 of 3` (at 3: `the gate is down: his down time …, he can be hit`). `… with the gate
  already down …: nothing to lower`; `… did not take message 3 …` if his handler refused it. At
  start: `m6: Oogie Boogie (CBossBoogie 0x140670A60) checked, …` or `… not as read …`.

### Not checked / Assumptions

- Nothing seen in game. `HDownTime`, the `BonusTime`s, `GMCPar` and the dice weights are in his
  parameter file, not in this session.
- Assumption: object 0x28D is the gate (`CGimmickBBGHedge`); its motions 2 / 3 / 4 are its three
  visible steps; Sora can only pass after the third (its collision; partial steps keep it on).
- Assumption: `+0x260 & 0x2000` is what keeps him from being hurt (set at his start and when he
  jumps back, cleared at the third step and by his gimmick card).
- Assumption: a posted message 3 would arrive after his state switch (why the handler is called at
  once); the stagger's look with no record (`+0x308`) is the five bosses' (`0x14017BAA0`).
- KH1's own Oogie fight is not this fight (KH1 has no gate): this is the owner's rule, not a KH1
  number.

### Owner test

1. Combat = KH1, Halloween Town, Oogie Boogie: let him throw his dice (the throw that ends with three
   dice landing). When it ends, he staggers and the gate goes down one step. Log: the `ended
   unbroken (1 of 1): Re:CoM's card break` line, then `count 0 -> 1 of 3`.
2. After his roll's attack, let him throw again, twice more: the gate goes down a step each time;
   after the third it is open (`count 2 -> 3 of 3 …; the gate is down`). Walk up to him and hit him.
3. When his down time ends he jumps back with a shockwave and the gate rises, pushing you out; the
   next throws lower it again.
4. Break a die that he threw (from a staggered throw) to see if his gimmick card drops (Re:CoM's
   chance); picked up and used (Stagger), the gate opens at once and he is stunned.
5. Combat = Re:CoM: as before (break his dice card three times; no `oogie` lines).

## Logs and trace

- Start: `m6: wrappers in (…)`.
- Each battle start (first 100): `m6: battle start: KH1 | Re:CoM (…); no-card attacks (P1) on |
  off; think gate (P2) and grant all (P3): ours | Re:CoM's | UNCHANGED…; crowd pace (P4): ours,
  each enemy as if alone (enemy_pace = kh1) | Re:CoM's crowd sharing (enemy_pace = recom) |
  Re:CoM's (n site(s) written this battle)`.
- Each cardless attack (first 100): `m6: attack without a card: <class> id=… state=…
  attacking_now=<enemies in bit 0 or states 2..10, this one included> flags …->… cooldown=…`.
- Once per class: `m6: class <name>: attacks without a card (P1)`, `… Re:CoM's card play (<why>):
  its card shows`, `… plays a side-2 card at KH1 (Re:CoM's card path …)` (from a log-only
  pass-through wrapper on CBTL::setPlayCard `0x140163830`, `48 8B C4 57 41 54 41 55 41 56 41 57`).
- CBTL refusals (first 20): `m6: CBTL locked (+0x208=…, gimmick=…, duel=…): Re:CoM refuses …`.
- F9 trace, `events` cell, every recorded frame: `m6 battle=kh1|recom attacking=<n>
  cbtl_slot2=<CBTL+0xBC> enemies=[id:s<state>:<P S R G>:<cooldown> …]` (up to 8 enemies; P = bit
  0, S = 4, R = 0x200, G = 0x800). Read only while recording.

## Not verified

- Nothing seen in game.
- The frame delay Re:CoM has between grant and attack (Assumption 1–2 frames).
- Whether any Heartless class reads its card record (`+0x318..`) through a non-literal offset.
- Scripts (tutorial flags): files not in this session. `CTutorialHood` takes P1.
- KH1 has no shared attack limiter as far as the Shadow's script and the common syscalls were
  read (research §4.3); rarer syscalls: Assumption.
- Re:CoM enemies keep Re:CoM's AI: what each chooses and think rate are Re:CoM's until their KH1
  twins' AI (M10); their attack-end cooldown is their class's own (§ Attack-end cooldown). P4 makes crowds attack more often than Re:CoM's tuning (decision 30
  chose that).

## In-game checklist (stage 1), in priority order

1. Log at start: every `site ok`, then `m6: wrappers in`.
2. A room with 3+ Shadows or Soldiers at Combat = KH1: no card over an attacking enemy and no cue
   sprite; `m6: attack without a card` lines, some with `attacking_now` ≥ 2; F9 trace shows
   `attacking=2` or more and `cbtl_slot2=0`.
3. Enemies stop attacking at the win pose and during an escape (`m6: CBTL locked` lines, no
   attack after the win).
4. Sora's hits still land on attacking enemies (decisions 14, 26) and Guard reactions still
   flinch them (decision 27: `card bit cleared (not CBTL's now-playing)`).
5. Combat = Re:CoM in Config, next battle: `battle start: Re:CoM … Re:CoM's bytes (3 written)`,
   enemies play cards one at a time as in Re:CoM. Back to KH1: `ours (3 written)`.
6. Heartless bosses: Guard Armor, Dark Side (its children attack while it acts), Trickmaster,
   Parasite Cage, Jafar, Ursula, Oogie, Marluxia 2, Marluxia 3 (its sleights still show cards
   and its stock folder: stage 3). Each can be finished.
7. Traverse Town's tutorial battle (`CTutorialHood`): no soft-lock.
8. Human-boss fights: unchanged (`m6: class … plays a side-2 card at KH1`). Tutorial Leon's known
   P5 issue is stage 3.
9. Riku's story: `battle start: Re:CoM (Riku's story)`, everything as Re:CoM.
10. Feel: with `enemy_pace = recom` (default) a crowd shares one attack budget; with `kh1`
    crowds attack as often as a lone enemy (P4).

## Stage 2: no deck for Sora (decision 29)

At Combat = KH1, in Sora's story, outside minigame rooms. Research: `m6-cardless-battles.md` § 1
and `recom-m6-patch-points.md` §§ 1–3, 5 (kept with the decomp tools); every address below was
re-read for this stage. Tags as above.

### What changes

| Part | Where | What | Tag |
|---|---|---|---|
| Empty revolver | wrapper on the battle revolver build `0x1401F43D0` (`40 57 41 56 41 57 48 83 EC 40`, 10 bytes moved) | with `player+0x3F0 == 0`: `mem = 0x1404ACE54(0x280)`, then Re:CoM's ctor `0x140164D40(mem, player, list, 1, 0xFFFFFFFF)` with `list = {0x000000FF}`, stored at `player+0x3F0`; then the original runs, sees the built revolver (`0x1401F43F5`: `cmp [rcx+0x3F0],0; jne` tail) and only runs its tail (`+0x3E8` = CBTL) | Checked (decomp + exe) |
| No Friend-card drops | wrapper on `0x14017E190` (`40 53 48 83 EC 20 48 8B D9`, 9 bytes) | returns at once: the same result as Re:CoM's own failed roll (id 0, no prize spawned) | Checked (decomp + exe) |
| Deck HUD hidden | 7 of Re:CoM's `call 0x1401B08C0` (system variable (1, 0xE2), Re:CoM's HUD-hide mask) retargeted to a stub near the exe, per battle | the stub (`49 89 D8` `mov r8,rbx`, then `jmp [rip+0]`) calls ours with the caller's object; ours returns Re:CoM's mask, plus bit 2 for the cockpit of this battle's empty revolver, or bit 0x20 for a side-1 card while the main player holds it. Re:CoM's own hide branches then run | Checked (decomp + exe) |

- **The empty revolver is Re:CoM's own "deck of 0 cards".** `0x1401F43D0` builds a saved deck's
  list as its cards plus an end entry `0x000000FF` (byte 3's low nibble from `0x1405A02D0`, which
  is 0; Checked (exe)). The loader `0x14016C970` stops at an id byte of 0 or ≥ 0xCB
  (`0x14016C9A0`), so page 0 gets 0 cards (`+0x70`), a count of 1 (`+0x6C`) and 10
  `CBattleCard` slots, the first of which then holds the **Reload card** (id 0xA0); `+0x98 |= 1`. Re:CoM builds the same for a saved deck
  of 0 cards and for an enemy revolver without a deck. Nothing is null: CBTL's unchecked uses
  (`0x140163C00` case 0x6D, `0x1401F4AB0` mode 3) stay safe. Checked (decomp).
- Skipped with Re:CoM's build: `0x14015A680()`, which refreshes the staging copy `0x14087A650`
  of the active saved deck (from the save's deck index). Its only other user, `0x14015BCB0`,
  refreshes it itself first (`0x14015BA90`). Checked (decomp + exe scan for references).
- **Covers every deck Re:CoM would have dealt Sora:** the saved deck, and the fixed decks of the
  null-deck branch, including the **tutorial deck** `0x1407C2450` (event flag (1, 0x2C); Kingdom
  Key 0–9 and a Shadow enemy card). Not changed: Riku's story (decision 6, `0x1401AAAE0`), a
  main player that isn't `CPlayerSora`, **minigame rooms** (`player+0x394 != 0`, rooms
  1200..1255: Re:CoM's minigame deck `0x1407C2590`; the same test `0x1401F43D0` makes).
- **No shuffle, no draw, no Reload, no stock, no sleights:** the revolver ctor and loader make no
  random call (research, Checked (decomp)); Reload starts only from request bit `0x10`, set only
  in the battle branch of `0x1401F8150`, which KH1 Sora never takes (Stage 9's field branch;
  the use-card button `0x1401E8FF0` is skipped). Stock and sleights need a played card.
  Checked (decomp).
- **The revolver still deals** (state 0 → 2 → 1, `0x140169F40` → `0x140169710`): it is not
  stuck in state 0, so event flag (1, 0x13) fires at the end of the deal, and the card-scatter
  flag `+0x98 & 0x8000` (`CBossMarluxia3Tornado` `0x1403F3B60` reads it) is still cleared in
  state 1. The deal gate's own mask read is not retargeted. Checked (decomp).
- **Battle end, escape, game over, continue:** Re:CoM frees the revolver itself
  (`0x1401F1B20`, `0x1401F1C40`, `0x1401E7C20` set `+0x3F0 = 0`); it is Re:CoM's object, built by
  its ctor with its allocator. A continue re-enters stateInitial, so the wrapper runs again. The
  battle end (`0x1401871D0` → win action 0x1C) reads no deck. Checked (decomp). Not seen in game.
- **Switch point:** the revolver build, called by `CPlayer::stateInitial` (`0x1401FAAD0`) only in
  battle rooms and only when `player+0x418 & 0x20000000` is clear, and by Riku's stateInitial
  (`0x140206A1A`). It decides from the setting at that moment. The retargets follow: ours for a
  battle without a deck, **Re:CoM's bytes otherwise**, all or none, checked writes. The two
  wrappers stay installed and pass through at the Re:CoM setting (as stage 1's). Assumption: the
  setting can't change inside a battle (the Config menu is in the camp).

### Friend cards (decision 29)

- The drop roll `0x1401602A0` gives only **Pluto 0x39** and **Donald, Goofy, Aladdin, Ariel,
  Jack, Peter Pan, Beast 0x31..0x37** (The King 0x38 in Reverse/Rebirth). All are type 5 in both
  card tables (`0x14059F2E0`, `0x140526510`; Checked (exe)). The prize add `0x1401F4AB0` puts type
  5 / 6 cards **only into the battle revolver** (`0x140166300`), never into the collection.
  Checked (decomp).
- So at KH1 **Donald, Goofy and the other Friend cards are gone with the deck**, as decision 29
  expects (decision answer 4: no party for now). Nothing is removed from the collection.
- The hook covers all three callers: the hit handler (`0x14017FC9D`, chance −1 = the world's) and
  the battle setup's room-card 0xAF case (`0x140180366`, `0x1401803F4`, chance 1000). Checked
  (exe). Decided from the setting at the drop (Combat = KH1, Sora's story, not a minigame room
  `0x140235360`), so the battle setup's drops don't depend on whether they come before the
  player's revolver build.
- Skipping the roll skips its global random draws (`0x14029CDC0`): the random sequence differs
  from Re:CoM's. Assumption: no rule depends on the sequence (P1 already changes it).
- Script-given Friend cards (`0x1401F4AB0` from the event VM, mode 2) still go into the empty
  revolver, hidden. Assumption: rare in Sora's-story battles.

### Sora still collects cards

Nothing on the collection path is hooked. Checked (decomp):

- Normal prizes (types 1–4, 7, 9–0xB) go to the collection `0x140158FC0` in `0x1401F4AB0` unless
  mode 3, and **no caller passes mode 3** (its five callers pass 1 or 2). Battle prizes come
  through the pickup `0x140187AA0` (mode 1); HP, Moogle points, the post-battle roulette
  (`0xD6` / `0xD7`) are ids above 0xCA, before any card code.
- Rewards, Moogle shops, chests and event cards use the collection, not the battle revolver,
  which exists only in battle. Assumption (shops not read): they never touch `player+0x3F0`.

### The deck HUD

| Part | Re:CoM's control | At KH1 (stage 2) |
|---|---|---|
| Cockpit frame and the two count digits | `0x1401689F0` (player side only), 4 reads `test al,2` at `0x140168A34`, `0x140168A93`, `0x140168B32`, `0x140168B90` | bit 2 for this battle's empty revolver: Re:CoM's own hide (state 4, slide-out seq 0x44, digits off) |
| Sora's cards, the Reload card and its gauge (`0x140154750` draws it on the card's sprite) | per-frame show / hide `0x140150340` (`0x14015037C`, tests 0x20 for side 1, 0x40 for side 2); deal `0x1401514B0` (`0x140151625`) and refresh `0x1401545A0` (`0x1401545BD`) test the whole mask | bit 0x20 for side-1 cards while the main player holds the empty revolver: Re:CoM's hidden branch; the deal and refresh take their hidden branch too, so there is no one-frame show |
| Stock row, `CEnchantGauge`, selected-card panel `+0x108`, cue `+0x248` | need a stocked or played card, a page switch | never shown at KH1; no change |
| Enemy cards (side 2), Marluxia 3's `CStockFolder`, the enemy cue sprite | bits 0x40 / 4, CBTL | untouched by stage 2; hidden by stage 3 (§ Stage 3) |

- `CBattleCard`s are made only by CBTL and the revolvers (`0x14014F8C0`'s callers), so side-1
  cards exist only in battle. Checked (decomp).
- Each retargeted read sits right after `mov edx,0xE2` and ecx = 1, and its function put its
  object in rbx first (`mov rbx,rcx` at `0x1401689FA`, `0x140150359`, `0x1401514CC`,
  `0x1401545AE`) with no later write to rbx before the read. Checked (exe, disassembly; the
  bytes are in `sites::M6_DECK_CHECKED`).
- The stub touches no stack, so our function is entered as the getter would be (the caller's
  return address, alignment and shadow space). It clobbers r8, which is volatile in the x64 ABI;
  the getter itself ends in a jump through a vtable (`0x1401B08DE`), so no caller can rely on r8
  across it. Checked (exe).
- Not used: writing (1, 0xE2) itself (it also hides the HP gauge, and events rewrite it), and bit
  4 (it gates the enemy revolver's deal in `0x140169F40`). Checked (decomp).
- Cost: an OR and two loads in reads Re:CoM already makes every frame. No data-only route:
  Re:CoM re-reads the mask every frame.

### Exe checks

- New sites (`sites::M6_DECK_HOOKS`, `M6_DECK_READERS`, `M6_DECK_CALLEES`, `M6_COCKPIT_CALLS`,
  `M6_CARD_CALLS`, `M6_DECK_CHECKED`), all byte-checked at start; the 163 byte lists of
  `sites.rs` were re-read from the Steam exe (0 mismatches).
- No branch in `.text` and no relocated pointer lands inside any moved or retargeted byte (the
  5 hook prologues, the 7 call sites); the only other references to the hooked starts are their
  `.pdata` entries. Checked (exe scan).

### Logs and trace (stage 2)

- Start: `m6: HUD stubs at …`, `m6: no-deck wrappers in (…)`, `m6: deck readers logged once per
  battle (…)`.
- Each battle start (first 100): `m6: battle without a deck: empty revolver 0x… built by Re:CoM's
  ctor 0x140164D40 from its end-of-deck entry (0 cards, 1 slot(s), flags 0x…: Reload card in);
  tutorial-deck flag (1, 0x2C) set | clear; deck HUD mask reads (cockpit 4, cards 3): ours (n
  written); Friend-card drops off; tutorial flags set: … (battle n)`, or `m6: battle with Re:CoM's
  deck (Combat = Re:CoM | Riku's story | minigame room | …); deck HUD mask reads: Re:CoM's bytes
  (n written)`.
- Drops (first 20): `m6: Friend-card drop roll skipped (chance …)`.
- Once per battle, a reader that read the empty revolver (first 60): `m6: deck reader (battle
  n): the tutorial's hand-empty check (0x1401EA1A0, CAtelTutorial) …`, `… an event script's
  revolver scroll (0x1401EA1E0) …`, `… a revolver value search (0x140167CF0, enemy card value /
  Oogie)`. Pass-through hooks, installed apart (log only).
- F9 trace, `events` cell: `m6deck deck=none|empty|recom tut=[<set flags of (1, 0x11..0x18,
  0x1B)>]`, after stage 1's `m6 …` token.

### The tutorial battles (soft-lock risk): what is known and what isn't

Known (Checked (decomp + exe)):

- `CAtelTutorial` (`0x140137CB0`, made by script command 0xAD sub 0x10) sets event flags while it
  exists: (1, 0xE) on Sora's action 6 (roll), (1, 0xD) on action 7 (jump), and (1, 0x11) while
  `0x1401EA1A0` says "hand empty" (no revolver, or `+0x70 == 0`). KH1's roll and jump still run as
  actions 6 / 7, so 0xD and 0xE still fire. **0x11 fires at once** at KH1 (0 cards).
- Flags only the deck sets: 0x13 deal done (**still fires**); 0x12 reload / page switch
  (`0x140167910`), 0x14 / 0x16 a player card done (`0x14015E0E0` side 1, `0x14015FC53`), 0x15
  stock (`0x14016DB60`), 0x17 a revolver scroll (`0x14016C580`, also from the script scroll):
  **0x12, 0x14, 0x15, 0x16 never fire at KH1**; 0x17 only if a script scrolls.
- `CTutorialHood` (`0x1403A97E0`) sets (1, 0x18) once Sora's target cursor (`player+0x3F8`,
  `+0xE0 == 1`) has been on it until its time delta × 0.02 sums to 60 (Assumption: about one
  second), and is driven by the script's variable (1, 0xE1)
  (steps 1, 2, 3, 6, 7, 9). Its attacks go through P1 (stage 1). `CTutorialLeon` sets (1, 0x1B)
  and attacks only while Sora's action is 0xB (P5, stage 3).
- No exe code reads flags 0xD, 0xE and 0x11..0x17: only event scripts do.

Not known (the event scripts are not in this session): **which of these flags the tutorial
scripts wait for.** If a script step waits for 0x12, 0x14, 0x15 or 0x16 ("reload", "play a
card", "stock"), that tutorial cannot finish at KH1, with or without this stage: Sora's card
input has been off at KH1 since Stage 9, so even the tutorial deck could not be played. If a
step waits for 0x11 ("deck empty") it now comes at once. Nothing in stage 2 fakes these flags.
What the tester's log and trace give us: the battle-start line shows whether (1, 0x2C) was set
and which tutorial flags were set; `deck reader` lines show `CAtelTutorial` running; the F9
trace's `tut=[…]` shows the flags frame by frame. Owner question if it soft-locks:

- (a) fire the flags the script waits for from their KH1 counterparts (for example 0x14 / 0x16
  when a KH1 swing ends), once the script files show which;
- (b) keep Re:CoM's card battle for the tutorial battles only (flag (1, 0x2C)), card input
  included;
- (c) skip the deck tutorial steps at KH1 through the script's own variable.

### Not verified (stage 2)

- Nothing seen in game.
- The cockpit may play its slide-out (seq 0x44) once at battle start: its init `0x14016A840`
  starts it in state 1, and the first update sees bit 2 (Re:CoM's events hide it the same way).
  Assumption.
- Whether something else turns a hidden card's sprite back on while its `+0x264 & 0x80` is clear
  (`0x140150340` re-hides only cards with 0x80): none found; Re:CoM's event hide has the same
  rule. Assumption.
- The tutorial scripts (above). Room cards that read Sora's deck: none found beyond the readers
  listed in research § 5.
- Shops and chests not re-read (they don't use the battle revolver: Assumption).
- Minigame rooms keep Re:CoM's minigame deck, but KH1 Sora has no card input there (Stage 9):
  whether those minigames can be played at KH1 is not checked (not new).

### In-game checklist (stage 2), in priority order

1. Log at start: every `site ok`, `m6: HUD stubs at …`, `m6: no-deck wrappers in`, `m6: deck
   readers logged`.
2. A normal battle at Combat = KH1: no cockpit, no card wheel, no count digits, no Reload card or
   gauge at any time (watch the first second: a brief cockpit slide-out is the known risk).
   Log: `battle without a deck: … (0 cards, 1 slot(s), …: Reload card in)`. F9 trace:
   `m6deck deck=empty`.
3. Sora attacks, jumps, rolls, guards, takes hits (including a card-scatter hit) and wins:
   win pose, rewards and card prizes as usual; prizes land in the collection (check the deck
   editor after the battle).
4. No Friend card drops (hit many enemies; `Friend-card drop roll skipped` lines); HP / Moogle
   prizes still drop.
5. Escape from a battle; lose a battle and Continue; both work, and the next battle again logs
   `battle without a deck`.
6. The very first tutorial battle (the hooded figure, `CTutorialHood`) and Traverse Town's
   Leon (`CTutorialLeon`, which needs stage 3's P5 to attack): does each finish? If not, the log's
   battle-start line (`(1, 0x2C)`, tutorial flags) and an F9 trace of the stuck moment
   (`tut=[…]`).
7. Switch Combat to Re:CoM, next battle: `battle with Re:CoM's deck (Combat = Re:CoM); deck HUD
   mask reads: Re:CoM's bytes (7 written)`, the full deck and HUD as Re:CoM, Friend cards drop.
   Back to KH1: `ours (7 written)` (16 since stage 3).
8. (Stage 2 alone) bosses with enemy cards still showed their cards; since stage 3 they are
   hidden (§ Stage 3).
9. Oogie Boogie's fight: no hang (it searches Sora's deck: `deck reader … value search`).
10. Riku's story: `battle with Re:CoM's deck (Riku's story)`, everything as Re:CoM.

## Stage 3: bosses (decisions 29, 30)

At Combat = KH1, in Sora's story. Research: `m6-cardless-battles.md` §§ 2.5, 3, 5.1, 5.2, 6, 7 and
`recom-m6-patch-points.md` §§ 4–6 (kept with the decomp tools); every address below was re-read
for this stage. Code: `boss.rs`, `win/battle.rs` (P5, logs), `win/deck.rs` (enemy card UI);
`sites::M6_BOSS_PATCHES`, `M6_GETTER_CALLERS`, `M6_ENEMY_UI_CALLS`, `M6_ENEMY_UI_CHECKED`,
`M6_BOSS_HOOKS`.

### P5: KH1 swings count as "Sora is attacking"

- Re:CoM's getter `0x14017F720` returns the main player's action `player+0x40C` (2 without a
  player). It is 25 bytes and 7 `int3` up to `0x14017F740`:
  `48 8B 05 59 BC 6F 00 48 85 C0 74 07 8B 80 0C 04 00 00 C3 B8 02 00 00 00 C3 CC×7`.
  Checked (exe).
- **P5** rewrites those 32 bytes in place: the same first instruction (RIP-relative load, same
  place, same displacement) and null test, then `mov eax,[rax+0x40C]; cmp eax,0xA; jne +2;
  mov al,0xB; ret; mov eax,2; ret`:
  `48 8B 05 59 BC 6F 00 48 85 C0 74 0E 8B 80 0C 04 00 00 83 F8 0A 75 02 B0 0B C3 B8 02 00 00 00 C3`.
  Action 0xA answers 0xB; every other answer is unchanged. Checked (exe: the bytes disassembled,
  both jumps land on whole instructions; tests `boss::p5_bytes`).
- **Nothing else reaches those bytes.** The exe has exactly 15 `call rel32` to `0x14017F720` (no
  `jmp`, no `jcc`), as the decomp lists; the only other references are its `.pdata` entry and one
  coincidental 4-byte value inside a `jmp` displacement at `0x14012C22B` (no relocation there).
  No branch lands inside the 32 bytes but the getter's own `je`. Checked (exe scan).
- **Every caller** compares the answer at once (`cmp eax,imm8`, two with one unrelated instruction
  between) and never stores it. None compares with 0xA, so the only test P5 changes is `== 0xB`
  while Sora's action is 0xA. Checked (exe; the 15 contexts are `sites::M6_GETTER_CALLERS`, checked
  at start; test `boss::no_reader_changes_but_the_0xb_ones`):

| Call | Owner (RTTI) | Compares | What it does when equal | Story |
|---|---|---|---|---|
| `0x140199B2B` | `CHumanBoss` card play `0x140199A00` (Leon, Cloud, Hades, Vexen, Riku, Larxene, Axel, Hook, Marluxia 1, Zexion; vtable `+0x480`) | 0xB | holds its card back if it is below Sora's playing value `0x140160910(1)`, which is −1 at KH1 (side 1 never plays): never | Sora's |
| `0x1403A9CFD` | `CTutorialLeon` think `0x1403A9CE0` | 0xB | plays his card (his only "attack"); message 2 then sets event flag (1, 0x1B) | Sora's |
| `0x1403AFA7F`, `0x1403AFDB1` | `CBossCloud` state 1 `0x1403AF8E0` | 0xB | 1 in 10 per call, in range (`0x1401A7810`): a dodge (`0x1403AEAD0`, motion e106_005) | Sora's |
| `0x1403B7E06` | `CBossRiku` state 1 `0x1403B7C80` | 0xB | close (< 25): 1 in 50 a reaction | Sora's |
| `0x1403BD744` | `CBossLarxene` state 1 `0x1403BD320` | 0xB | close (≤ 9): 1 in 100 a reaction | Sora's |
| `0x1403C1998` | `CBossAxel` state 1 `0x1403C1760` | 0xB | 1 in 80 a reaction (e118_003) | Sora's |
| `0x1403C273A` | `CBossParasiteCage` think `0x1403C26F0` | 0xB | below 30% HP: an attack request at once (else 1 in 30) | Sora's |
| `0x1403C9F1A` | `CBossHook` state 1 `0x1403C9AF0` | 0xB | close (≤ 9): 1 in 25 a reaction | Sora's |
| `0x140412B4E` | `CBossMarluxia` (Marluxia 1) state 1 `0x140412850` | 0xB | close (≤ 6.25): 1 in 10 a reaction | Sora's |
| `0x140414F1E` | `CBossLexaeus` card play `0x140414D80` | 0xB | as `CHumanBoss` | Riku's |
| `0x14041801F` | `CBossLexaeus` `0x140417F80` | 0x22 | Riku's action 0x22 | Riku's |
| `0x140424BCD` | `CBossAnsem` card play `0x140424A50` | 0xB | as `CHumanBoss` | Riku's |
| `0x140426890` | `CBossAnsem` `0x1404266C0` | 0xC | longer wait while the player is in damage | Riku's |
| `0x14045FF38` | `CEnemyAnsemShadowUppercut` `0x14045FF00` | 0xC | no shot while the player is in damage | Riku's |

- **Switch:** the battle enemy manager's ctor wrapper (stage 1) switches P5 with P2–P4, as its
  own all-or-none set (a foreign getter leaves P2–P4 working): ours in a KH1 battle (Combat = KH1,
  Sora's story), Re:CoM's exact bytes otherwise, from the next battle on. Riku's story keeps
  Re:CoM's bytes, so Lexaeus, Zexion and Ansem are untouched. Checked (code; tests).
- **What runs as action 0xA at KH1:** every KH1 swing (ground and air) and **Counterattack**: all
  of them start through Re:CoM's field-swing init `0x1401F0F70` (`Host::start_swing`; the
  Counterattack is row 0x3E through the same path, `attack::command_counterattack` →
  `row_exec`). Guard and the roll (action 6), jump (7), glide (8) are not 0xA and don't count.
  Checked (code + `recom-kh1-attack.md`).
- **Counterattack counts (decided here):** it is an attack, and Re:CoM counts every attack Sora
  makes (all its attack cards are action 0xB). Telling it apart would need the getter to read
  the KH1 row (`player+0x3C8`), which doesn't fit in place (32 bytes); it would need a hook. Say if
  the owner wants it excluded.
- **Assumption (not measured):** KH1 swings are free and chain, so Sora is in action 0xA more of
  the time than Re:CoM's Sora is in 0xB. These per-frame rolls (Cloud's dodge 1 in 10 per call,
  Parasite Cage's at-once request below 30% HP) will fire more often than in Re:CoM. Re:CoM's
  own numbers are unchanged.
- Not new: in the field between battles the getter keeps the last battle's bytes; only boss AIs
  call it, and they exist only in battle. Minigame rooms build no manager and have no bosses.
  Assumption: the battle build runs on the game thread (as P2–P4).

### Human bosses keep their decks as a hidden move list (decision 30, option (a))

- `CHumanBoss` users in Sora's story (Tutorial Leon, Cloud, Hades, Vexen, Riku, Larxene, Axel,
  Hook, Marluxia 1) never reach P1: their `+0x98` is `0x1401829D0` (Checked (exe), vtables). They
  pick an attack from the top card of their own revolver (`+0xC88`), play it through
  `0x14016CEE0` → `0x140163830` (side 2) and start it from message 2, all as Re:CoM. Their reloads
  (`0x140199130`) and sleights (`0x14019A1E0`) use their own revolver. All but Leon use the
  generic handler `0x140181620` (Leon: `0x1403A9F40`). Checked (exe + decomp).
- The Heartless bosses (Guard Armor, Trickmaster, Parasite Cage, Jafar, Ursula, Oogie,
  `CBossMDragon`, Dark Side, Marluxia 2 and 3) attack through P1 (stage 1). Those that take
  Re:CoM's card play at KH1 (Marluxia 3 while its `+0xCE4 & 4` mode is set, or CBTL locked) set a
  side-2 card, hidden like the others below.
- **Nothing they need comes from Sora's cards** (Checked (decomp) unless tagged):
  - Their only read of Sora's card is `0x140160910(1)`, −1 at KH1.
  - Boss code (`0x140170000..0x1401A0000`, `0x1403A0000..0x140430000`) calls no CBTL side-1
    query (`0x1401616E0`, `0x140161610`, `0x140161530(1)`, `0x140160950(1)`) and reads no CBTL
    duel / break counter (`+0x20C`, `+0x214`, `+0x216`). Checked (decomp, call and literal search).
  - Message 3 (card broken) never comes at KH1: the generic handler makes it a reaction (state
    13), not a step of the fight. Leon's handler sets flag (1, 0x1B) on messages 2, 3 and 4 alike.
  - Leon's card ends on its own timer (`+0xD00` = 120 after message 2, counted down at 0.02 × dt,
    then `0x14015E0C0`), so his tutorial never waits for a break.
  - **Exception: Marluxia 3's card duel** (below).

### Marluxia 3's card duel (open question)

- A hit with reaction 0x21 (Re:CoM's damage check `0x1401E6DE0`) puts Sora into action 0x24,
  the card duel (`CPlayerSora` vtable `+0x1E0` init `0x140220C40`, `+0x1E8` exe `0x1402184D0`).
  The init sets a countdown of 7 (`+0x458`), locks CBTL and sets duel mode 9 (`0x140164130`).
  Mode 10 sends Marluxia 3 message 0xD, whose handler (`0x1403F4550`, only in its `+0xCE4 & 4`
  mode) sets the countdown time (CBTL `+0xAC`) and mode 0xB with six cards in its stock folder
  (`0x140164040`). Each step of the countdown is a timer (Sora's `+0x160` from CBTL `+0xAC`);
  Sora answers by playing cards from his revolver (`0x14016D8A0`). When the countdown runs out
  (state 10), `0x140163EF0(CBTL, count left)` sends both sides message 0xF / 3 (count > 0) or 0xE
  (count ≤ 0); Sora's 0xF sets `+0x428 |= 0x2000000`, which also ends the duel exe early; 0xE
  sets `+0x428 |= 0x1000020`. Checked (decomp). How Sora's card plays turn into a win (CBTL's
  duel resolution against Marluxia's six cards): not traced, Assumption.
- **At KH1 Sora has no card to play**: the empty revolver's only card is the Reload card, which
  the loader builds with type 8 (byte 3 = 0x88, `0x14016C970`), and the duel play `0x14016D8A0`
  refuses types 6..8. Checked (decomp). So he can never win the duel at KH1. It does end (the
  countdown is timer-driven): no soft-lock found. What "lost" costs Sora in the fight: not read (Assumption: Re:CoM's punishment for
  losing the duel; Marluxia 3 gets `+0xCD4` = 10 instead of 200). Which attacks carry reaction
  0x21: in the attack data files, not read (Assumption: Marluxia 3's only).
- Logged: `m6: Sora's card duel (action 0x24, battle n) starts at KH1 …` (log-only hook on
  `0x140220C40`, `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 40`, 15 bytes moved; referenced only
  by the vtable and `.pdata`; Checked (exe)).
- **Owner's choice (c), built in 0.14.1 (decision 33):** at KH1 the damage-check wrapper turns a
  pending hit's reaction `0x21` (`player+0x710`, read once by `0x1401E6DE0` after its
  hit-pending test `+0x720`) into the plain flinch 1 before Re:CoM's check runs. The damage is
  unchanged (only kinds 8 and 0x22 change it). Sora never enters action 0x24. Marluxia 3 sets
  its duel mode (`+0xCE4 |= 4`, `0x1403F3590`) only while Sora's action is 0x24, so it takes its
  own no-duel branch (state 3: motion 0x34, `+0xCD4` = 100). Checked (decomp). Not seen in game.
  Log: `attack: a card-duel hit (reaction 0x21) taken as a plain flinch at KH1`.
  - The stock folder's hide (state 5 parking) no longer matters: the folder only plays cards in
    the duel.
- The question was:
  - (a) leave it as Re:CoM's duel, always lost at KH1;
  - (b) count a KH1 action as Sora's duel card (for example each KH1 hit on Marluxia 3 during
    the countdown breaks one of its six cards), through Re:CoM's own duel path;
  - (c) at KH1, make reaction 0x21 a normal hit reaction so the duel never starts (Marluxia 3's
    countdown attack becomes a plain hit).

### The enemy card UI, hidden (decisions 15, 30)

All through Re:CoM's own event-hide mask (system variable (1, 0xE2)) at the reads that draw
each part, switched per battle with stage 2's seven reads (16 in all, all or none) at the
battle revolver build: ours in a battle without a deck (Combat = KH1, Sora's story, not a
minigame room), **Re:CoM's bytes otherwise**. A third 17-byte stub in stage 2's block
(`mov r8,rbx; jmp [rip+0]`) calls `enemy_ui_mask`.

| Part | Reads | At KH1 | What Re:CoM does then | Logic touched? |
|---|---|---|---|---|
| Every side-2 `CBattleCard` (enemy attack, sleight, reload cards; Marluxia 3's folder cards) | stage 2's `0x14015037C` (tests 0x40 for side 2), deal `0x140151625` and refresh `0x1401545BD` (whole mask) | `card_mask` adds 0x40 for a card whose side (`+0x26C`) isn't 1 | its hidden branch: `+0x264 & 0x80` off, sprite off | no: "in place" (`+0x264 & 2`, which lets CBTL post message 2) is set by the slide distance in `0x1401542F0`, not by visibility. A new card starts hidden (`0x14014F8C0`): no flash. Checked (decomp) |
| The card-play cue over the enemy (CBTL `+0x220`) | `0x14015E3F3` (in `0x14015E390`), `0x140161A64` (CBTL update `0x1401617F0`), both `test eax,eax` | `enemy_ui_mask` adds 4 (nonzero) | clears CBTL `+0x208` bit 0, stops the sprite | no: bit 0 is set only by `0x1401613D0` (Heartless card play `0x1401634B0`, enemy duel card `0x140163570`) and read only by these two. Human bosses never set it. Checked (decomp) |
| A side-2 revolver's stock row: the sleight name (`+0x250`, table `0x1407C25E0`), the stocked value digits (`+0x270`, `+0x278`), frame `+0x268` | `0x140169CE3`, `0x140169D07`, `0x140169D48` in `0x140169CA0` (`test edi,eax`, edi = 4 for side 2, 2 for side 1); `0x14016E2B1` in its show `0x14016DE70` (`test ebx,eax`) | adds 4: only the enemy side's test sees it; Sora's own row tests 2 | its hidden state 5, digits off | no: `+0x258`, `+0x25C`, `+0x260` are read only by these visual functions (`0x140168060`, `0x14016AF40`, `0x14016B430`, `0x14016DE70`, `0x140169CA0`). Checked (decomp) |
| Marluxia 3's `CStockFolder` (`0x140169070`, CBTL `+0x1A0`) | `0x1401690A6`, `0x1401690DD`, `0x140169103` (`test al,4`) | adds 4 | its hidden state 5 | **yes, in the duel only**: state 6, which plays the folder's cards as "pronounce" cards (`0x140163630`) in duel mode 0xB, is entered from state 3 only, so a hidden folder skips it. Outside the card duel its states are visual. Checked (decomp). This is the change the owner accepted with "hide (changes the fight)" (research § 8 Q3 (b)); with the duel unwinnable at KH1 anyway, it only removes Marluxia's card plays inside that duel |

- Each read is Re:CoM's `call 0x1401B08C0` right after `mov edx,0xE2` and ecx = 1 (`mov ecx,1`,
  or `lea ecx,[rbp-1]` / `lea ecx,[rax-1]` with 2 loaded in the checked bytes). The contexts and
  function starts are `sites::M6_ENEMY_UI_CHECKED`. No branch or pointer lands inside any of the
  nine calls. Checked (exe scan; tests `sites::m6_boss_sites`).
- `enemy_ui_mask` ignores the stub's third argument: at `0x14016E2B1` rbx holds the tested bit,
  not an object. r8 is clobbered as in stage 2 (volatile; the getter ends in a vtable jump).
- **Not hidden:** the enemy-side enchant gauge (`CEnchantGauge` `0x14016BB30`, when a boss plays a
  room-effect card through `0x140163170`): its hidden state 5 skips `0x14015E010(side, 0, 0)`, a
  step of the enchant counter, so hiding it through the mask would change how long the enchant
  lasts. Whether a Sora's-story boss deck holds such a card: in the boss data files, not read.
  Owner question if a tester sees one. Also not touched: the revolver deal gate `0x14016A058` and
  the revolver play `0x14016CF53` (bit 4 there would stop the boss's own plays).
- Assumption: a boss's stock row and Marluxia 3's folder may show for one frame when a card is
  stocked (their show functions `0x14016DE70` / `0x14016E3C0` turn the frame on, and the hide
  comes at the next update's mask read), as with Re:CoM's own event hide.

### Readers of Sora's deck and cards, with the empty revolver (research § 5.1)

| Reader | At KH1 | Tag |
|---|---|---|
| Heartless card play `0x140198870` (`0x140160910(1)`, card value) | skipped by P1; in a fallback −1 | Checked (decomp) |
| `CHumanBoss` / Lexaeus / Ansem card play (`0x140160910(1)` when the answer is 0xB) | −1 now also during KH1 swings: never holds back | Checked (decomp) |
| Oogie `0x1403D91C0` (`CBossBoogie` vtable `+0x448`, a search of Sora's revolver `0x140167CF0`) | called only by the Heartless card play's card value (`0x140198870`), which P1 skips; in a fallback it finds nothing on the empty revolver and returns 1 | Checked (decomp + exe RTTI) |
| Enemy card value `0x140182500`, drop helper `0x140160620`, CBTL `0x14015E5D0`, `0x140161E10`, `0x140162140`, `0x140163C00`, `0x140163EF0` | read or write Sora's revolver behind a null test; the empty revolver is Re:CoM's own object | Checked (decomp, literal search outside player code) |
| Marluxia 3's tornado `0x1403F3B60` (`rev+0x98 & 0x8000`) | the empty revolver still deals (stage 2) | Checked (decomp) |
| Zexion `0x140420ED0`, `0x14041AF00` | Riku's story | not affected |

### Logs and trace (stage 3)

- Start: `m6: bosses (stage 3): P5 … switched with P2-P4 …`, `m6: card duel logged
  (0x140220C40, log only)`, and stage 2's `m6: HUD stubs at … (enemy card UI …); 16 mask reads …`.
  (`win/mod.rs`'s own line still says "the 7 deck HUD mask reads": it is not edited in this
  stage.)
- Battle start (first 100): `m6: battle start (battle n): …; Sora attacking for bosses (P5): ours,
  Sora's action 0xA (KH1 swings, Counterattack) reads as 0xB (k written) | Re:CoM's (k written) |
  UNCHANGED`. Stage 2's line: `card HUD mask reads (cockpit 4, cards 3 with enemy cards, enemy
  card UI 9): ours (16 written)`.
- Per battle (first 40 of each battle, 1200 in all):
  - `m6: boss card (battle n, #k): <class> id=… sets a side-2 revolver card | new card (hidden |
    SHOWN …); Sora's action 0x…, bosses read 0x…; CBTL duel mode …` (every side-2 card: a human
    boss's attack, sleight or reload card, Leon's reaction, a Heartless fallback);
  - `m6: boss attack without a card (battle n): <CBoss…> id=… state=… hp=… cooldown=…` (Heartless
    bosses through P1);
  - `m6: boss takes Re:CoM's card play (battle n, <why>): …` (e.g. Marluxia 3 in its sleight mode).
- `m6: Sora's card duel (action 0x24, battle n) starts at KH1 …` (first 20).
- F9 trace, `events` cell, after stage 1's token: `m6boss p5=ours|recom|other act=<Sora's
  action> ans=<what bosses read> duel=<CBTL +0x20C> ui=hidden|shown`. Read only while recording.

### Exe checks (stage 3)

- New sites, all byte-checked at start: P5 (32 bytes), the 15 caller contexts, 9 calls, 14
  contexts / function starts, 1 log hook. The 203 byte lists of `sites.rs` were re-read from the
  Steam exe: 0 mismatches.

### Not verified (stage 3)

- Nothing seen in game.
- How often the bosses' 0xB reactions fire with KH1's swing rate (Assumption: more often).
- The tutorial scripts: whether Traverse Town's Leon step waits for (1, 0x1B) (which P5 now
  lets fire) or for a card break (which can't happen at KH1). Scripts not in this session.
- What losing Marluxia 3's card duel costs, and which attacks carry reaction 0x21.
- Whether any Sora's-story boss deck holds an enchant (room-effect) card (its gauge stays shown).
- Which bosses appear in Sora's story (game knowledge, not data).
- One-frame shows of the stock row / folder when a card is stocked (Assumption).

### In-game checklist (stage 3), per boss

For each fight at Combat = KH1: no enemy card, card cue, sleight name / stock row or stock
folder at any time; the boss attacks, reloads (a pause) and uses its sleights as in Re:CoM; it
can be beaten; the log shows `boss card` lines (human bosses) or `boss attack without a card`
lines (Heartless bosses), and the F9 trace shows `ans=0xb` while Sora swings.

1. Log at start: every `site ok`, `m6: bosses (stage 3)`, `m6: card duel logged`, `HUD stubs …
   16 mask reads`. Battle start: `P5: ours … (1 written)`, `ours (16 written)`.
2. **Traverse Town, Leon** (`CTutorialLeon`): Leon reacts to KH1 swings (`boss card … CTutorialLeon
   … Sora's action 0xa, bosses read 0xb`), event flag 0x1B in the F9 `tut=[…]`, the fight
   finishes. If it stalls: F9 trace of the stuck moment.
3. **Guard Armor, Trickmaster, Jafar, Ursula, Oogie Boogie, Dragon Maleficent (`CBossMDragon`),
   Darkside**: Heartless bosses, P1 (`boss attack without a card`). Oogie: no hang.
4. **Parasite Cage**: below 30% HP it attacks whenever its cooldown allows while Sora swings
   (P5); compare with Re:CoM.
5. **Cloud** (dodges KH1 swings), **Hades**, **Vexen**, **Larxene**, **Axel**, **Hook**,
   **Riku** (each fight), **Marluxia 1**: `boss card` lines with `(hidden)`, their reactions to
   KH1 swings; reload pauses and sleights still happen; no sleight name shown.
6. **Marluxia 2**: P1, no card.
7. **Marluxia 3**: no stock folder, no sleight cards; when its countdown attack hits, the log's
   `Sora's card duel … starts at KH1` line, then what happens (it is lost; note the cost). Owner
   question above.
8. Combat = Re:CoM, any boss: cards, cue, sleight names, folder as Re:CoM; battle start shows
   `P5: Re:CoM's (1 written)` and `Re:CoM's bytes (16 written)`.
9. Riku's story (Lexaeus, Zexion, Ansem): `Re:CoM (Riku's story)`, P5 Re:CoM's, everything as
   Re:CoM.

## Battle end: Re:CoM's sequence, the KH1 frame's yield, the battle-end log (owner's 0.24.2 soft-lock)

Owner (2026-10-31, 0.24.2): "after finishing a battle in the field sora is locked in place and
doesnt return to the field". Log `kh1-recom.log` (battle 2: the last four enemies fell to a
Thundaga's bolts); F9 trace 13 of the stuck moment (155 frames).

### Re:CoM's battle end. Checked (decomp + disasm)

1. **The last enemy.** An enemy with `+0x154 & 0x40000` dies → `0x140187440(manager, id)` → the
   manager's enemy count `+0x98` − 1; at 0 → `0x1401F4A60`: unless Sora is dead (`+0x7B0`) or in
   action 0x17, **`player+0x428 |= 4` (win pending)** and `+0x420 |= 0x2000`.
2. **Sora's win action `0x1C`** (`0x1401F1B20`: prev = action, action = 0x1C, `+0x414` = 2,
   motion reset `0x1401F6990(8)`, motion stop, CBTL lock `0x140162250`, the revolver freed) is
   entered from Re:CoM's own code only:
   - the move exe `0x1401EEDA0` (actions 2–4) in battle, before its commands;
   - the end action `0x1401F6660`, which every attack / magic exe calls at its end
     (`0x1401F6794`: `test byte [rbx+0x428],4` → `call 0x1401F1B20`, instead of the idle / fall);
   - the damage check `0x1401E6DE0` when a hit arrives.
   Re:CoM's jump exe has no such test: an airborne Sora lands first.
3. **The pose.** The Battle exe's case 0x1C: `+0x414` 0/1 → 2; at 2, `0x1401F6380(pl, 1)` (on the
   ground: motion row 0x2A, done) and `+0x420 & 0x2000` → `+0x414` = 4 (3 in scripted battles).
4. **The manager** (`CBattleEnemyManager`, task 0x19; its step `0x1401871D0`):

   | state `+0x58` | waits for | then |
   |---|---|---|
   | 1 | child tasks `+0x50` ≤ 0, enemy count `+0x98` ≤ 0, room (`0x1402339B0`) not in `0x41A..=0x4AE` | 2, timer `+0x5C` = 6000 ticks (2 s; 15000 with `+0x74`); 4 at once with `+0x70` |
   | 2 | prize display busy (`0x140209830`: `CPrizeGauge` `0x14087B690` `+0x94` ≠ 0) → 3; else the timer runs down by `+0x14` | 4 |
   | 3 | the prize display closed (`+0x94` = 0) | 4 |
   | 4 | `0x1401F3C90`: alive, action 0x1C, `+0x414` > 2 | 5 / 6 (`CRoulette`, prizes 0xD6 / 0xD7: `+0x6C` / `+0x70`), else 7 |
   | 5, 6 | the roulette task 0x22 gone | 6 / 7 |
   | 7 | — : event flag (1, 2) each step (`0x140182240`); the room script returns Sora to the field | — |

   The prize display (`0x1402094F0`, run by `CPrizeGauge`'s update `0x14020A710`, which returns
   at once while the gauge pause `0x14087B808` is set): a card picked up in battle shows with
   `+0x94` = `player+0x390` + 1 (`0x1401F4AB0`); state 2 closes when its sequence ends, or after
   3000 ticks on a confirm press (`0x14029B490(0)`, a plain pad read); state 1 closes when its
   sequence ends or the HUD mask (1, 0xE2) is set. Its EXP display (`+0x5C`) steps in the same
   update, after it.

### What the owner's trace shows. Checked (trace 13)

Every frame: Re:CoM's action 0x1C, `+0x414` = 4, phase 3, row 0x2A, "Re:CoM's own handler", no
enemy in the manager's list. So **Sora's win pose is done** (`0x1401F3C90` answers 1) and Re:CoM's
manager is waiting in state 1, 2, 3, 5 or 6 (or the room script after 7). The trace had no field
for the manager, so which one is **not known**. The owner's earlier screenshot showed the EXP
display ("EXP + 12") still on screen, which fits the prize gauge not stepping (its update returns
early while the gauge pause `0x14087B808` is set, or the task isn't run) — **Assumption**.

### Our hooks on the battle-end path. Checked (decomp + exe + code)

None of ours writes the manager, the prize gauge, the gauge pause, the roulette or flag (1, 2):

- **MP refill wrappers (decision 35):** `0x1401AC9D0` is `void(this)` (a state function stored in
  `+0x68`), wrapped as `fn(usize)`, the original called first with the same argument; the
  level-up screen `0x14010EF10` is `void(this)` (vtable), the same; the call `0x140237F3A` is
  `call 0x1402374C0` with `ecx` = 0 (`void(int)`), retargeted through a near `jmp [rip]` to
  `checkpoint_call(a)`, which calls the original with `a` and returns to `0x140237F3F` (the HP
  fill), registers per the x64 ABI. Behaviour-identical apart from the MP write.
- **MP bar / command menu (`CPlayerGauge` init `0x140208360`, second-bar state `0x14020C190`,
  deleting dtor `0x140207770`):** each calls Re:CoM's first (init, state) or last (dtor) with its
  arguments; `CPrizeGauge` is made inside the init (`0x140206EE0`) and has its own dtors
  (`0x1402072E0`, `0x1402078A0`, not hooked). The command menu reads the confirm press with
  `0x14029B490` (a pure read of the pad's pressed bits) and writes no pad state; it only clears
  `player+0x424` request bits.
- **M6 stage 2 / 3 mask reads:** the 16 retargeted (1, 0xE2) reads are in the cockpit
  (`0x1401689F0`), Sora's cards (`0x140150340`, `0x1401514B0`, `0x1401545A0`), the enemy cue,
  stock row and stock folder; none is in `0x1402094F0`, `0x14020A710`, `0x14020D940`,
  `0x14020DEC0`.
- **Request bits `0x1401F8150`:** keeps Re:CoM's battle-branch early returns while the win is
  pending (`battle_guards_clear` tests `+0x428 & 4`).
- **The KH1 frame at the win (the one deviation found):** when the last enemy dies during a KH1
  cast (or swing), the cast's end calls Re:CoM's end action, which enters the win action; our
  frame then went on in the same frame and **wrote over it**: the idle motion request over the
  win pose, then action 2 (`set_action`), so Re:CoM's win action was entered a second time from
  the move exe the next frame (`0x1401F1B20` twice: CBTL lock and motion stop twice, the
  recorded previous action 2 instead of 0x13). The trace shows Re:CoM recovered (pose done), so
  this is **not** what held the battle, but Re:CoM's own path must run as in Re:CoM.

### Changed (this version)

- **The KH1 frame yields to Re:CoM's win action.** `Host::end_swing` / `end_cast` return whether
  Re:CoM's end action entered action 0x1C (`win::attack::end_swing`, `win::magic::end_cast` read
  `+0x40C` after `0x1401F6660`); the KH1 frame (`Kh1Sora::yield_to_recom`) then writes nothing
  more (no motion in the motion update's replay, no action, no queued idle) and drops its cast /
  swing; `run_frame` clears its sync so a later KH1 frame takes over afresh; `exe_magic`'s
  take-over path no longer runs a KH1 frame after an end that entered the win. Test:
  `movement::tests::a_cast_that_ends_after_the_win_yields_to_re_coms_win_action` (a Thundaga
  ended by the win, a swing ended by the win, and no change without a won battle). Checked
  (decomp `0x1401F6660`, `0x1401F1B20`; tests).
- **The battle-end log and trace** (`crate::battle::end`, `win::battle::end_watch`, from the
  request-bits wrapper; two loads a frame unless the win is pending or Sora is in action 0x1C):
  `battle end: started: Sora action … sub … phase … dead … win-pending …; manager state s timer t
  children n count c roulette a/b; prize gauge card p exp e; gauge pause x; roulette task r; flag
  (1, 2) f; HUD mask m; waiting for <what Re:CoM waits for>; prize gauge 0x…: CPrizeGauge flags …
  banked … step … parent … flags …; EXP ctrl layers … id … done …; card ctrl …; ours: KH1 cast …,
  swing …, lock …; spell threads …`, again on every change (not the timer) and every 10 s while
  nothing changes (60 lines a battle). State 1 with children lists the manager's child tasks by
  class. F9 trace: `end mgr=s<state> t<timer> c<children> n<count> prize=<card>/<exp> pause=…
  roul=… over=… hud=…` in the m6 cell. Sites `sites::BATTLE_END_CHECKED` (9, checked only; the
  getters `0x1401B0870`, `0x1401B08C0`, `0x1402480C0` were already checked).

### Not known yet (needs the new log)

- Which manager state the owner's battle stopped in, and so what Re:CoM waited for. If the log
  says **state 3 / prize gauge card ≠ 0**: is the gauge paused (`gauge pause 1`), its task not
  run (`flags` with 0x10 / 0x8000, or its parent's), or its sequence never ending (`card ctrl …
  done false` with `id 0xffffffff` would mean Re:CoM's 2D instance pool (`0x1402425E0`, 2048
  entries) ran out). Since 0.24.x the line ends with `2D: ours … ; Re:CoM's 2D pool F of 2047
  free` (`docs/recom-m7-mp-bar.md` § 2D instances and draw lists): F = 0 there confirms it. Our
  HUD's entries balance in normal play (Checked, decomp); the one leak found (layers made or
  released while the camp's draw list is current) is fixed there. **State 1 with children**: the class names. **State 5 / 6**: the roulette.
  **State 7 / flag (1, 2) 1**: the room script, not the manager.
- Why battle 1 of the same log returned and battle 2 didn't. Both ended on a Thundaga (battle 1:
  2 bolts, the last 30 frames before the cast's end; battle 2: a second wave, 4 bolts, the last
  14 frames before it). Not explained by anything read so far.

### The prizes that never end (owner's 0.47.0 log)

The 0.37.0 diagnostics answered the question above: **state 1 with children**, the children being
`CPrize`s that Re:CoM's own prize code can never end. Cause (Checked (decomp)): our direct strikes
(Thunder's bolts, Gravity's impact, Stop) applied their attack without Re:CoM's overlap test, so a
kill left the death point, where Re:CoM's defeat makes all the enemy's prizes, at the struck
body's uninitialised (or stale) contact point; a non-finite one makes prizes that are pulled at
once and never arrive (no timeout in that state). Battle 1 vs battle 2 of the 0.24.2 log: whatever
the bodies' memory held. Details and the native fix: `recom-m7-magic.md` § Strikes and Re:CoM's
contact.

## Enemy card sounds (owner, 2026-11-02)

Owner: "Lets also make it so that you cant hear enemies stocking cards or refilling their deck".
Owner's answers (2026-11-02) to the two questions below: (1) the enemies' first deal at battle
start stays silent, no change; (2) silence the human boss's item-card page switch (sound 3) too.
At Combat = KH1, in Sora's story (the battles stage 2 / 3 already switch: no deck for Sora, not a
minigame room), the sounds a side-2 revolver or stock folder plays when it stocks, reloads, deals
or turns to / from its item-card page are skipped. Everything else plays: the enemy's motions and voices, Sora's sounds, every
other sound; Combat = Re:CoM and Riku's story keep Re:CoM's bytes.

### Where Re:CoM plays them (Checked (decomp + disasm + exe))

All go through Re:CoM's 2D sound play `0x1402AA360(id, volume)` (→ `0x1402AA0F0`; id = group ×
1000 + entry, these are group 0, Re:CoM's system bank), called from the shared revolver code
(`CardBattle::CCardRevolver`, `+0x5C` = side: 1 Sora, 2 enemy, set by its ctor `0x140164D40`) and
`CStockFolder` (`+0x5C` = side, ctor `0x1401650D0`). The same code runs Sora's revolver, so each
call is told apart by its object's side.

| Id | When | Call (function) | Object |
|---|---|---|---|
| 0x13 | Reload card charging: a loop, handle kept at revolver `+0x200`, started when it is 0, stopped by the revolver update `0x14016B9A0` / `0x14016AF40` (handle > 0) | `0x140167C01` (reload step `0x140167AA0`) | rbx |
| 0x14 | one Reload gauge step filled (`+0x82` down by one, `+0x84` reset) | `0x140167BDA` (same) | rbx |
| 0x15 | reload done (gauge full) | `0x140167C93` (same) | rbx |
| 0x15 | reload done at once (`+0x98 & 0x80`, the caller's second argument 0) | `0x140167B01` (same) | rbx |
| revolver `+0x204`: 0x1B side 2 (`0x140164DD6`), 0x12 side 1 (`0x140164DF5`) | the deal after a reload (the refill; `0x14016B2B0`, also reached from the revolver update `0x140169450` and `0x14016C000`); handle at `+0x1FC`, stopped when > 0 | `0x14016B3E7` (`0x14016B2B0`) | rbx |
| same | the first deal at battle start (state 0, `0x140169F40`, once the event-hide mask allows: bit 4 for side 2) | `0x14016A091` (`0x140169F40`) | rbx |
| 0xF | stock: a card to the revolver's stock row | `0x14016DCA4` (`0x14016DB60`) | rbx |
| 0xF | stock: a card into Marluxia 3's stock folder (CBTL `+0x190 + 2 × 8`, `0x1401609C0(2)`; caller `0x1403EEBB0` from `0x1403F7110`) | `0x14016DE3F` (`0x14016DD20`) | rdi |
| 3 | page switch, only when its third argument is 0 (`cmp [rsp+0x60],r13d; jne`, `0x140167A26`); for an enemy: a human boss turning to or from its item-card page (below) | `0x140167A35` (`0x140167910`) | rbx |

Who reaches them at KH1: human bosses' think `0x140198B90` → vtable `+0x460` (`0x140199E10`)
reloads with `0x140167AA0(revolver, 0)` and stocks through
`0x14019A2C0` / `0x14019AA40` → `0x14016DB60`; Marluxia 3 (`0x1403F7110`, `CBossMarluxia3`'s
vtable `0x1406785B8` slot `+0x108`, Checked (exe RTTI)) fills its folder.
Heartless don't play cards at KH1 (P1), so their revolvers never empty: only their first deal at
battle start plays (0x1B).

**The page switch (sound 3), added on the owner's answer (2).** Sound 3 is a generic id (Re:CoM's
menus play it from their own calls), so only the one call inside the revolver's page switch
`0x140167910` is retargeted; no other `call 0x1402AA360` changes. That function is only ever
given a revolver (`rbx = rcx` at `0x140167926`, kept to the call), and of its 19 callers only six
pass a third argument of 0, the only case that plays the sound (Checked (exe: every `call
0x140167910` and its `r8d`)):

| Caller | Whose revolver | What |
|---|---|---|
| `0x1401991E9`, `0x1401992A1` (`0x140199130`, `CHumanBoss` vtable `+0x468`) | the boss's (`+0xC88`), side 2 | page 1 (its type-7 item cards, `UseItemCard`) on a 1-in-50 roll; back to page 0 when page 1 is empty |
| `0x1403B112B`, `0x1403B1164`, `0x1403B122A` (`0x1403B10C0`, `CBossHades` vtable `0x140662038` `+0x468`, Checked (exe RTTI)) | Hades' (`+0xC88`), side 2 | his own version of the same |
| `0x1401F8303` (Sora's request bits `0x1401F8150`) | Sora's (`player+0x3F0`), side 1 | Sora's own page switch: the side test passes it to Re:CoM's play, unchanged |

The other 13 callers pass 1 (no sound). An enemy's page switch is therefore only a human boss's
item-card page; Sora's goes through, and the UI never reaches this call.

Not found: a reload voice. The reload path (`0x140199E10`, `0x140167AA0`, `0x14016B2B0`) starts no
motion and plays no voice; it sets `+0x260` bit 7, which bosses' movement code reads to keep away
while reloading (their own walk / run / jump motions). That those motions carry no reload voice in
their sound events: Assumption (the enemies' `.SES` files aren't in the session). No visual effect
is made by these functions; the Reload gauge they draw (`0x140154750`) is on the side-2 Reload
card, already hidden at KH1 (§ Stage 3).

Not part of this (found, unchanged):

- Scroll 0xE (`0x14016C651` in `0x14016C580`, `0x14016C8E5` in `0x14016C860`): side 1 only (its own
  test), Sora's.
- Card-effect 0x20 (`0x1401631F0`, an enemy card's effect taking hold), card-break / clash sounds
  0x16 / 0x17 / 0x18 / 0x14D (resolver `0x14015EAC0`), duel 0x14B–0x14F: not stocking or refilling.

### What we built

| Part | How | Tag |
|---|---|---|
| Retargets | the nine `call 0x1402AA360` above (`sites::M6_ENEMY_SOUND_CALLS`) point at our stub, switched per battle with stage 2 / 3's mask reads (`win::deck::switch_hud`, all or none): ours in a battle without a deck, Re:CoM's bytes otherwise | Checked (exe: the calls and their contexts `M6_ENEMY_SOUND_CHECKED`; no branch or pointer lands inside them) |
| Stubs | their own 34-byte block near `0x140167B01`: `mov r8,rbx` (or `mov r8,rdi` for the folder) then a jump to `win::deck::enemy_card_sound(id, volume, object)`; no stack touched | Checked (disasm: the register holds the object from the function's `mov <reg>,rcx` to the call) |
| Decision | a side-2 object (`+0x5C` ≠ 1) while the battle hides the enemy card UI: nothing plays, the call returns 0. Otherwise Re:CoM's play with the same arguments | Checked (decomp) |
| Handles | 0 is what Re:CoM's play returns when it plays nothing (`0x1402AA0F0` with the sound off): the loop's `+0x200` and the deal's `+0x1FC` stay 0, so Re:CoM's stops (handle > 0) are skipped | Checked (decomp) |
| Off | Combat = Re:CoM, Riku's story, minigame rooms, a battle with Re:CoM's deck: Re:CoM's bytes at the calls | — |

Per frame: nothing new, except while an enemy's Reload card charges: Re:CoM asks for the 0x13
loop each frame its handle is 0 (as it does with its own sound off), so our function runs then
(one side read). Not measured.

### Logs

- At start: `m6: enemy card sound stubs at 0x… (rbx) and 0x… (rdi)` and `m6: enemy card sounds
  (stock, reload, deal, item-card page switch): 9 sound calls switched with them …`; or why they
  stay Re:CoM's.
- Each battle's start line counts them with the mask reads written (up to 25 at KH1: 16 reads + 9
  calls).
- Once per sound per battle (200 lines in all): `m6: enemy card sound 0x13 (Reload card charging
  (loop)) silent at KH1 (battle 4, side-2 object 0x…): Re:CoM's play skipped, the call returns 0
  (no sound, no handle)`.

### Assumptions

- Sora's revolver is built (the switch) before an enemy revolver's first deal; otherwise the
  first KH1 battle after a Re:CoM one could play one enemy deal.
- Answered (owner, 2026-11-02): the enemy's first deal at battle start (same sound, 0x1B, as the
  refill after a reload) stays silent.
- No reload voice in the bosses' keep-away motions (above).

### Owner test list

1. Log at start: `m6: enemy card sound stubs at …` and `… 9 sound calls switched …`.
2. Combat = KH1, a Heartless battle: no card-deal sound from the enemies when the battle starts;
   your own sounds and the enemies' hits and voices as before. Log: `enemy card sound 0x1b (enemy
   deck deal) silent …`.
3. A human boss (Axel, Larxene, Vexen, Riku, Cloud, Hades, Hook, Marluxia): when it runs out of
   cards and backs off to reload, no charging loop, no gauge ticks, no "done" chime, no deal sound;
   log lines for 0x13, 0x14, 0x15, 0x1b.
4. A boss using a sleight (it stocks cards first): no stock sound; log `0xf (stock)`.
5. Marluxia 3: no stock sound when its folder fills.
6. Still heard: the boss's voices and attack sounds, Sora's own sounds, menu sounds.
7. A human boss (or Hades) turning to its item cards, then using one: no page-switch sound; log
   `enemy card sound 0x3 (item-card page switch) silent …`. A 1-in-50 roll when it wants an
   item, so it may take a while; the log line is the proof.
8. Still heard: the same menu sound (3) in the camp and other menus, and Sora's own sounds.
9. Combat = Re:CoM: every enemy card sound back, no `enemy card sound` lines.
