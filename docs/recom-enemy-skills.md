# Enemy Skills: Re:CoM's enemy cards as KH1 abilities (plan)

The owner's decisions are in `design-philosophy-instructions.md` § "Enemy cards as abilities" (2026-11-03). The
research is below. Plan first; no code until the owner's go-ahead.

## Design (native)

- **Learning:** a skill unlocks at its card's first pickup, from Re:CoM's discovery state, as spells, summons and
  Keyblades do. It arrives unequipped and gets a notice.
- **Abilities page:** a third owner row, "Enemy Skills", with the Sora and Shared rows. Each skill is named after
  its card, has the owner's AP cost and a one-line description on the help line. The equipped skills are saved in a
  new extension of our save block.
- **Effects** run through Re:CoM's own code, by the three routes in § 3 of the research:
  1. **Play-time setters** (about 17 cards). For each equipped skill, Re:CoM's own player-side apply
     `0x140163C00(CBTL, id)` (or its setters) runs at a battle's start and again after every Sora-side clear
     `0x140161E10`. The boss cases' elemental affinity call `0x1401F8810` is skipped, so a boss skill gives the
     skill only.
  2. **The hit rewrite** `0x14015D5F0` (15 cards). A wrapper runs Re:CoM's own rewrite once per equipped skill of
     that group, each with its id in Sora's slot, which at KH1 is otherwise empty (Sora plays no cards). The slot is
     put back afterwards.
  3. **Direct compares** (about 43 sites). Each compare `slot == id` also answers true for an equipped skill (a
     checked patch per site, in batches).
- **Durations:** none. An equipped skill is permanent, except Auto-Life (once per battle). Second Chance is KH1's
  per-hit rule (already built). Re:CoM's counters touch only the slot, so a skill is never counted down.
- **The five KH1 matches** take the KH1 ability's place: its level grant goes, the card gives it. Soldier gives one
  Combo Plus (the Rod level-87 copy); Hook gives Second Chance; Hades Berserk; Creeper Plant Leaf Bracer;
  Bouncywild Treasure Magnet (KH1's; not built yet).
- **Combat = Re:CoM and Riku's story:** enemy cards as Re:CoM has them, untouched.

## Stages (one system per version)

1. The Enemy Skills section: learning, save, names, AP, descriptions, notices; the five KH1 matches (Treasure Magnet
   built as KH1's).
2. Route 1: the play-time skills.
3. Route 2: the hit-rewrite skills.
4. Route 3: the direct-compare skills, in batches.
5. The deck-to-KH1 meanings (MP recharge, no-interrupt, sleight MP, clash wins).

## Stage 1 (built, 0.106.0)

Ask (owner, 2026-11-04, "Go ahead": stage 1 of the plan): the Enemy Skills section, learning, save,
names, AP, descriptions, notices; the five KH1 matches. Combat = KH1 in Sora's story only; Combat =
Re:CoM and Riku's story unchanged (the ability state isn't live there: `win::mp::active`).

Code: `crates/kh1-recom/src/enemy_skills.rs` (the table, tested), `abilities.rs` (the abilities, the
page, the save), `notices.rs` (`card_abilities`), `win/abilities.rs` (the sync), `win/unlocks.rs`
(the first pickup), `win/keyblades.rs` (the page's rows).

### The table (`enemy_skills::SKILLS`)

48 skills, one per enemy card kept (0x44..=0x7E minus the 11 left out). Each: the card kind, the
card's name, Re:CoM's effect name, the AP, the help line, and whether its effect is built.

- **Names**: Re:CoM's card-info table `0x1405256D0` (16-byte rows, name pointer at `+8`; e.g. row
  0x4C → "Bouncywild", 0x7E → "Platinum Card"). Checked (exe). The page shows Re:CoM's own
  localized name through its card-name call `0x140164CA0` (message `[0x140528560 + kind × 4]`, which
  is the kind itself for 0x44..=0x7E: Checked (exe)); our English is the fallback and the notice's
  name (ASCII: Saïx is "Saix"; Re:CoM's string carries its own byte for the ï).
- **AP**: the owner's table; the five matches KH1's (btltbl `0x33D8`: Combo Plus 1, Second Chance 5,
  Berserk 1, Leaf Bracer 5, Treasure Magnet 2 with sort key 302; Rod levels of Treasure Magnet 9 and
  72). Checked (files: the player's `btltbl.bin`; `ap_tables_match_btltbl`).
- **Help lines**: ours, "Effect: what it does at KH1." within the 66-character box; the 43 skills
  whose effect comes in stages 2..5 end "(effect coming)". The KH1 meaning of three is ours, not
  the owner's (Assumption, to settle at their stage): Ansem "hides sleights from the enemy", Gold
  Card "sleights keep their premium cards", Lexaeus "sleights may warp enemies away".

### The abilities

- Every skill is an ability of `abilities::ALL` with `Source::Card(kind)`. The words widen to 128
  bits (`abilities::Bits = u128`): 58 before, Treasure Magnet ×2 (58, 59), the 43 skills of their
  own (60..102, in the table's order, never to be reordered).
- **The five KH1 matches** are KH1's own abilities, so their effects are the existing ones: Soldier
  = `ix::COMBO_PLUS[0]` (the Rod level-87 copy; 93 and 99 stay by level), Hook = Second Chance
  (was level 90), Hades = Berserk (60), Creeper Plant = Leaf Bracer (39), Bouncywild =
  `ix::TREASURE_MAGNET[0]`. KH1 gives Treasure Magnet at levels 9 and 72; by the owner's rule (the
  card replaces the ability) **the card gives one copy, replacing the level-9 one; the level-72 copy
  stays by level** (on Sora's list as "Treasure Magnet").
- **Built or not** (`abilities::built`, `UNBUILT_BITS`): the four matches act; Treasure Magnet (both
  copies) and the 43 own skills are shown **greyed and never equipped** (chosen over "equippable,
  effect coming": a skill that does nothing would cost AP, and switch on unannounced when its stage
  lands). Confirming one: error sound 5, "Not built yet: this can't be equipped."; the help line
  still describes it. `equipped_live` and the debug `kh1_all_abilities` drop them too.

### Learning

- A skill unlocks when its card was ever obtained: Re:CoM's discovery state (`0x14015B3F0`, state 2
  or 3; `win::keyblades::ever_obtained`), asked in the ability sync (`abilities::sync_with`). It
  arrives **unequipped** (KH1's rule).
- **At the first pickup**: the existing wrapper on `0x14015C7F0` (`win::unlocks::first_obtained`) runs
  the ability sync for an enemy card with a skill; logged `unlocks: <card> card 0x.. obtained for the
  first time: Enemy Skill …`. At Combat = Re:CoM the sync does nothing; the next KH1 read learns it
  from the discovery state.
- **Notice**: Re:CoM's learned screen with its ability wording and the card's name
  (`notices::card_abilities`, `Learned::Ability`), for a block at v7. None at a save's first sync
  or from an older block (the cards were picked up before this build).
- **Older saves** get their skills at the first KH1 read (the sync at the first Attack press, the
  Abilities page, or the first KH1 frame's `equipped_live`), as the spells do.

### The page

- A third owner row **"Enemy Skills"** after "Sora" and "Shared" (`page::OWNERS`); greyed until a
  skill is learned. Its rows: the learned skills by card kind, under the card's name (the five
  matches here, not on Sora's list). The status panel shows the AP cost; the help line the
  description. Equip / remove: KH1's AP rules, the same AP pool as Sora's abilities.

### The save (layout v7)

- The **skill extension** `save + 0x26BF ..= 0x26CF` (17 bytes) right after the words' extension
  (`0x26B7..=0x26BE`): u8 version (1), u64 unlocked bits 64..128, u64 equipped bits 64..128
  (`abilities::SKILL_EXT_FIRST`). Inside Re:CoM's spare tail; the spare-range report counts it as
  ours. Checked (code): no block of ours and no Re:CoM code uses it.
- The block's version becomes **7**. Read only from a v7 block whose extension version is set; a v6
  block's words hold nothing past its 58 abilities (masked).
- **Migration (once, at the first KH1 sync of a block older than v7)**: the four matches that came
  by level before (Combo Plus 87, Leaf Bracer, Berserk, Second Chance) are **taken back and asked
  again from their card** (as v2 → v3 did for the ini's story abilities); one whose card Sora already
  has comes straight back, as equipped as it was; the rest give their AP back. Then every skill whose
  card Sora has arrives unequipped, without notices. Logged (`abilities: block v6 -> v7 …; … taken
  back [...]`). Older DLLs read a v7 block as before (they never look past bit 63).

### Treasure Magnet: KH1's rule, and why it isn't built

- **KH1** (Checked (decomp)): prizes are collected (and then fly to Sora) when a party member is
  within a radius; `0x1402AC080` (HP / MP / munny prizes) and `0x1402AB420` (item drops, from
  `0x1402C08D0`) test distance² against **6400 (80 cm)** in the normal mode, with the height
  difference first required ≤ 30 cm (50 less for actor kind 0x21) and then left out (a flat distance),
  14400 / 40000 in the two other modes; with Treasure Magnet equipped (`0x140284BE0(5, member)`, the
  count of equipped copies) **one copy: 40000 (2 m), two or more: 160000 (4 m)**, whatever the mode.
- **Re:CoM** (Checked (decomp + exe)): its prizes are pulled toward Sora when within the player's
  `+0x550` (default **5.0 m**, `0x140161E10` / `0x140164C20`): `0x140230C70`, used by the EXP / HP /
  Moogle-point prizes (`0x140231010`, kinds ≥ 0xCB) and the card prizes (`0x14022F2B0`). The cards
  scattered by `0x1401682D0` (`0x14022EE30`) use a fixed **1.5 m** (2.25) unless Sora's slot is
  Bouncywild (`0x14022F23A`, then `+0x550`). Re:CoM's Draw sets `+0x550` = 10 m (`+0x54C` = 30 has
  no reader: exe scan); Re:CoM's win action (Battle exe action 0x1C, `0x1401F9C5E`, and
  `0x1401EC55E`) sets both to 50 m to pull every prize at a battle's end.
- **Why not mapped**: Re:CoM's normal radius (5 m) is already wider than KH1's Treasure Magnet with
  two copies (4 m). Setting `+0x550` to KH1's values while it's equipped would shrink Re:CoM's pull
  (to 2 m) instead of widening it, and KH1's own base (0.8 m, flat, ±0.3 m height) is not Re:CoM's at
  Combat = KH1 today. So the ability ships greyed, "(not built yet)" (the brief's rule). Native
  options for the owner:
  (a) KH1's whole rule at Combat = KH1: `+0x550` = 0.8 m without it, 2 m / 4 m with one / two copies
      (every prize pull then KH1's distance; Re:CoM's 50 m win pull kept; the height test not
      carried);
  (b) KH1's ratio on Re:CoM's base: 5 m × 2.5 = 12.5 m / × 5 = 25 m;
  (c) Re:CoM's own Draw: 10 m with either copy.
  **Owner (2026-11-04): (c), "Re:CoM's Bouncywild radius". Built in stage 2 (below).**

### Checked / not checked

- Checked: tests (the table, AP, names, help lines, learning from the cards, the four matches' level
  grants gone, the page's third row and greyed rows, the save round trip, v6 → v7, notices), the
  Windows build (no warnings); KH1's Treasure Magnet rule (decomp); Re:CoM's radius readers (decomp
  + exe scan of every `+0x54C` / `+0x550` access).
- Not checked: in game (the page's third row, the card-name call for enemy kinds in the camp, the
  notices); whether every enemy card's first pickup reaches `0x14015C7F0` (the doc's rule: types
  1–4 and 7, Checked (decomp) in `recom-m7-unlocks.md`).

## Stage 2 (built, 0.127.0): route 1, the play-time setters

Ask (owner, 2026-11-04, "Go ahead" on the plan; "Enemy Skills numbers"; "Treasure Magnet": "Re:CoM's
Bouncywild radius"): the skills that act through Re:CoM's play-time writes. Combat = KH1 in Sora's story
only; Combat = Re:CoM and Riku's story: nothing applied, every switched site Re:CoM's.

Code: `crates/kh1-recom/src/skill_effects.rs` (pure, tested: which skills are built, their help lines,
the rules and numbers), `win/enemy_skills.rs` (the runtime), `sites::ENEMY_SKILL_HOOKS` / `_CALLEES` /
`_CHECKED` / `_PATCHES` / `_STUB_SITES`. One line each in `win::attack` (swing start, damage check, pair
dispatch, KH1 hit), `win::movement` (the KH1 frame, Re:CoM's gravity read), `win::battle` (battle start),
`win::abilities` (the page's equip), `win::mod` (Combat switched), `abilities::to_core`; kh1-core's
`Abilities::combo_finish`.

### When (one wrapper, no per-frame hook)

- **Re:CoM's writes repeated, not its switch called**: `0x140163C00` reads only `*0x14087B380` and would
  run the bosses' affinity call `0x1401F8810` (owner: "skill only"); its cases are plain `or
  [player+0x420], imm` / two float stores / tail calls to the setters (Checked (exe):
  `sites::ENEMY_SKILL_CHECKED` holds those bytes, the tests read the bits and floats from them). So
  the writes are repeated on the player at hand, the setters (`0x1401F7EC0` Vanish, `0x1401F6FB0` Aero
  Guard) called as Re:CoM's cases call them, each once (Aero Guard's makes its wind object at every
  call: Checked (decomp)).
- **A KH1 battle's start / the field**: the first KH1 frame of each player object, or after a change
  (`win::enemy_skills::kh1_frame`, one atomic compare a frame; a generation bumped by each battle
  manager ctor, the battle's end, the page's equip and a Combat switch). In a field room only Draw.
- **After every Sora-side clear** `0x140161E10` (hook, 15 bytes; Checked (exe): 10 direct calls, no
  branch into the bytes): Re:CoM's clear, then the skills back. Not at the battle's end
  (`0x140162250` sets CBTL `+0x208 |= 0xC0000004` before its clears: then only the generation moves,
  so Re:CoM's 50 m end-of-battle pull `0x1401F9C5E` is never shrunk). Reasons it runs at KH1: an
  enemy's Parasite Cage (Dispel) clears Sora's side (`0x140163170` case 0x64), the battle's end.
- **The page / Combat switched** (`refresh`): applied, and what is no longer equipped taken off
  (only ever ours: Sora plays no card at KH1).

### Per skill (Re:CoM's reader, the number)

| skill | how | number | tag |
|---|---|---|---|
| Bouncywild, KH1's level-72 Treasure Magnet | `+0x54C` = 30.0, `+0x550` = 10.0 (Re:CoM's Draw, `0x140163D44`); field and battle | prizes pulled from 10 m (5 m without) | Checked (exe + decomp) |
| Large Body | **not** Re:CoM's barrier (it blocks): in the damage-check wrapper, before Re:CoM's check, Re:CoM's own Large Body barrier values for one call (its setter `0x1401F8FF0(p, 1)`: class mask 1, half-angle π/2), its barrier test `0x14016EED0` on the hit Re:CoM's apply recorded (body `+0x30` the attack's point, `+0x84` its class), the four fields put back at once | a front hit's value × 90 / 100 (ours: Re:CoM has no cut) | Checked (decomp); "class 0 = physical": Assumption |
| Bandit | Re:CoM's `+0x640` = 1 is overwritten by the KH1 swing's own length; so KH1's combo length 1 (kh1-core `Abilities::combo_finish`, ground and air): KH1's picker makes every swing the finisher | — | Checked (tests: KH1's controller as the oracle) |
| Fat Bandit | `+0x420 \|= 0x100000` → Re:CoM's `procAttackCollision` `0x1401F5DC0` → `setBackAttack` (weapon `+0x28C` = collision `+0x4C` = 1); Re:CoM's apply `0x14016F1C0` then doubles a hit from behind (`+0x4C` = `+0x48`). At Damage = KH1 KH1's number replaces the part's value: the doubling is carried over (`back_attack_kh1`) | × 2 from behind | Checked (decomp) |
| Wight Knight | not Re:CoM's CBTL `0x4000` (every actor's gravity, `0x1401A59A0`): Sora's own gravity, the KH1 controller's (`params.gravity`, KH1's `0x1402AF960` arc) and at `jump_height = recom` Re:CoM's read of `0x14087B398` | × 0.75, field and battle | Checked (code); Assumption: KH1 defines its jump by height, so the jump keeps its height and lasts 1 / √0.75 = 1.155 times as long (Re:CoM's Float, fixed take-off speed, jumps higher); Re:CoM-run airborne states (hit reactions, sleights) keep Re:CoM's gravity |
| Gargoyle | Re:CoM's Vanish setter `0x1401F7EC0(p, 1)`; Re:CoM's per-frame pulse | Re:CoM's | Checked (decomp) |
| Neoshadow | CBTL `+0x208 \|= 0x2000`, `+0x1FC` = 0 (Sora's Bio, `0x1401632B6`); Re:CoM's tick `0x1401617F0`: every enemy object (type 0x1004) `vt+0xD8(…, 1)` = HP − 1, never below 1 (`0x1401814C0`, Checked (exe)) | every 1 s: the tick's `cmp [+0x1FC], 1000` → 3000 (switched) | Checked (exe) |
| Guard Armor | `+0x420 \|= 0x80` → `setWideAttack` `0x1402257C0`: the weapon collider's scale; its `movss xmm1, [1.0]` points at the pool's 0.75 (`0x1406C10A8`) in a KH1 battle (switched); our KH1 sphere radii × Re:CoM's live scale ÷ its base 0.5 while its flag (weapon `+0x2CC & 1`) is on (`0x140172690`'s field: type 2 `+0x74`, types 4/5/7/8/0xB `+0x70`) | × 1.5 | Checked (decomp + exe); the collider's type for Sora's weapon not read (both fields handled) |
| Card Soldier | `+0x420 \|= 0x100`; Re:CoM's reader is its row pick (`0x1401F7A27`), which KH1 swings replace: our row pick calls Re:CoM's own motion speed `0x1401F74D0` after the motion request (which resets it to 1.0, `0x1401F7180`), as Re:CoM's does | × 1.25 (Re:CoM 2.0: the setter's parameter) | Checked (decomp); Assumption: every KH1 swing window follows Re:CoM's motion clock (the KH1 frame reads it) |
| Oogie Boogie | CBTL `+0x208 \|= 0x400`, `+0x1F8` = 0; Re:CoM's tick heals max(1, missing ÷ 8) (`0x1401E59A0`) | every 6 s: `cmp [+0x1F8], 6000` → 18000 at `0x14016189D` and `0x1401600F2` (switched) | Checked (exe) |
| Xemnas | `+0x420 \|= 0x1000` → Sora's damage check `0x1401E73C3`: the barrier on all classes, all round | 1 s: `mov [+0x7E4], 6000` → 3000 at `0x1401E73F3` (switched) | Checked (exe) |
| Xaldin | Re:CoM's Aero Guard setter `0x1401F6FB0(p, 1)` (its wind, power Sora's `+0x43C`) | Re:CoM's | Checked (decomp) |
| Saïx | `+0x420 \|= 0x40000000`; our swing start already repeats `initAttack`'s bonus (`+0x664 + 20`) at Damage = Re:CoM; now from 0 at a combo's first swing | + 20 power a combo step | Checked (code); decided: per combo (a permanent skill would grow without end); **Damage = KH1: no effect** (the bonus is card power) |
| Roxas | `+0x420 \|= 0x20000000` → `procAttackCollision` → `setDoubleHit`; the weapon update `0x140225840` spawns the second strike with power weapon `+0x288`; in a KH1 battle its read `0x140225D7C` jumps to a stub that halves it (Re:CoM's ÷ 2) | half power | Checked (exe: no branch into the 10 bytes); **Damage = KH1**: the strike carries half of Re:CoM's cardless swing AP (the attack stat) through Re:CoM's hit path, not KH1's formula: owner question |
| Dragon Maleficent | its play-time part is the reload charge (revolver `+0x8C` = 67): no meaning at KH1, not applied; its attack × 1.5 is stage 3's | — | — |

### Switched sites (`sites::ENEMY_SKILL_PATCHES`, `ENEMY_SKILL_STUB_SITES`)

At each battle start (`battle::manager_ctor`, as M6's P2–P5): ours in a KH1 battle, Re:CoM's
otherwise. Each touches Sora's side only (the enemy side's Regen `0x800` / Bio `0x1000` have their own
compares; `setWideAttack` / `setDoubleHit` are `CPlayerWeapon`'s; the barrier write is the player's
damage check). Only the constant changes (tests); no other checked site overlaps one. A test checks
every site; `check_sites_rs.py`: 1330 sites, 0 mismatches.

### Not built here

- Bouncywild's scattered-card test `0x14022F23A` (Re:CoM's 1.5 m pull of the cards its revolver
  scatters when Sora is hit, `0x1401682D0` from the damage check through `0x14016CD80`): at KH1 the
  revolver is empty, so no card is scattered (Checked (decomp)); every prize that exists uses `+0x550`.
- Damage = KH1: Xaldin's wind, Neoshadow's Bio and Roxas' second strike carry Re:CoM's numbers.

### Checked / not checked

- Checked: tests (`skill_effects` 6, `sites::tests::enemy_skill_sites`,
  `movement::tests::combo_finish_makes_every_swing_a_finisher`, the stage-1 tests updated),
  `check_sites_rs.py`, `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings).
- Not checked: in game (none of it); the second strike's bullet taking `+0x288` as its damage
  (Assumption); Sora's weapon collider type; KH1 swings reaching the weapon update's hit result
  (`+0x2C8`) that spawns the second strike (Assumption: our pair dispatch returns Re:CoM's results).
## Stage 3 (built, 0.129.0): route 2, the hit rewrite `0x14015D5F0`

Ask (owner, 2026-11-04, "build everything"; the Enemy Skills numbers): the skills Re:CoM's hit rewrite
tests. Combat = KH1 in Sora's story only; Combat = Re:CoM and Riku's story: Re:CoM's rewrite as is.

Code: `skill_effects::BUILT_3` / `rewrite_mask` / `hit_record` / `rewrite_block` (pure, tested),
`win::enemy_skills::rewrite` / `kh1_swing_hit` / `spell_rewrite` / `warp_guard`;
`sites::ENEMY_SKILL_REWRITE_HOOK` / `_SITES` / `_CHECKED`. One line each in `win::attack` (`enemy_hit`,
`kh1_enemy_hit`, `spell_hit`), `win::magic` (Cure's cast), `win::damage::spell_status`.

### The wrapper (Checked (decomp + exe))

- Re:CoM's rewrite reads Sora's slot from the global CBTL (`*0x14087A818 + 0x1E8`, `0x14015D60A`), once
  per record, and handles one card. Our wrapper (hook, 6 bytes `push rbx; sub rsp,0x20`; 6 direct
  callers, no branch in): for a **side-1 record in a KH1 battle with Sora's slot empty** (Sora plays
  no card at KH1), Re:CoM's rewrite once per equipped route-2 skill, in card order, each with its id
  in the slot; the slot put back after. Every other record (the enemy side's `0x14017F0E0`, a real
  card in the slot, Combat = Re:CoM): Re:CoM's, once. Several skills stack (each run on the last
  run's result, Re:CoM's truncation each time; Re:CoM itself can't hold two).
- **The owner's numbers**: Re:CoM's constants in the rewrite (Zexion `cmp r8d,25` `0x14015D70A`,
  Lexaeus `cmp r8d,67` `0x14015D74C`, Demyx `add ax,ax` `0x14015D831`, Defender / Ursula `÷ 2`
  `0x14015D66A` / `0x14015D64D`) are switched at each battle start (KH1 battle: a jump to a stub of
  ours; else Re:CoM's bytes). Each stub is Re:CoM's instruction with its constant read from a cell:
  Re:CoM's value (25, 67, 200 %, 50 %, 50 %) except while our wrapper runs Sora's skills (10, 15,
  175 %, 75 %, 75 %). At Re:CoM's cells every result is Re:CoM's own for every i16 input (tests), so
  enemy cards in a KH1 battle keep Re:CoM's numbers. Demyx and Defender / Ursula use Re:CoM's ÷ 100
  (`0x14015D7D6`'s magic division).

### How KH1's attacks reach it

| attack | Damage = Re:CoM | Damage = KH1 |
|---|---|---|
| KH1 swing | at its start, the card AP through `0x1401E66A0` (existing): situation `+0x630` = the swing's Re:CoM row 1..7 → **attack card** (`+0x1E`), finishers (rows 4 / 7) also `+0x1A` | per hit (`kh1_swing_hit`): KH1's number in the same record (`0x1401E66A0`'s rule on `+0x630`), the kind it writes on the hit's body (`+0x80`) |
| KH1 Fire / Blizzard / Thunder / Gravity / Stop / Aero | at the cast, Re:CoM's card / sleight AP and status bits through `0x1401E66A0` (existing; situation 0: not an attack card) | per hit (`spell_rewrite`): KH1's number with the Re:CoM card's / sleight's status bits (`+9` / `+0xB`; e.g. Fire 0xC = magic 4 + fire 8, Gravity 0x104, Cure 0x44: Checked (files `SY0002.BIN`)); Stop keeps KH1's ticks (not run) |
| KH1 Cure | at the cast, the heal through the rewrite (as `initMagic` runs it for Re:CoM's Cure), both settings | the same |
| Re:CoM's sleights, KH1 summons (LV3 sleights), friend sleights | Re:CoM's own calls (`0x1401E66A0`, `0x14015D8A0`) | the same (then the existing KH1-HP scaling) |
| Hits on Sora (mode 2) | Sora's damage check `0x1401E6DE0` (attribute `+0x714`) | the same |

So the "attack-card hit" skills fire on **KH1 swings** (Search Ghost's Drain, Zexion's confuse,
Dragon Maleficent's × 1.5), and Lexaeus' Warp Break on **KH1 combo finishers** (Re:CoM's "sleight"
flag is set for its finisher rows 4 / 7). The category skills fire on spells, Cure, sleights and
summons (not on swings: Re:CoM's attack-card branch returns before its category switch).

### Per skill

| skill | Re:CoM's test | number | at KH1 |
|---|---|---|---|
| Red Nocturne / Blue Rhapsody / Yellow Opera / Green Requiem | category 8 / 0x10 / 0x20 / 0x40 | × 1.5 | Fire / Blizzard / Thunder spells (and their sleights); Cure's heal |
| Search Ghost | attack card → kind 0x15 | Re:CoM's Drain (`0x14017DC50` case 0x15: Sora + max(1, min(20, enemy HP ÷ 8)), the enemy's EXP × ¾) | KH1 swings |
| Defender | mode 2, attribute 0 | damage × 0.75 (Re:CoM ÷ 2) | hits on Sora |
| Wizard | category 4 | × 1.3 | magic cards' status (spells, Cure, magic sleights). Re:CoM's card dimming (`0x14016D200`) is its play-time part: no cards at KH1, not applied; only its × 1.3 |
| Crescendo | category 0x80 | × 1.3 | summons (summon cards' status 0x80). Its card dimming: as Wizard's |
| Ursula | mode 2, attribute 2..4 | damage × 0.75 | Fire / Blizzard / Thunder hits on Sora |
| Dragon Maleficent | attack card | × 1.5 | KH1 swings (its reload part: none, stage 2) |
| Lexaeus | attack card with `+0x1A` → kind 0x1F (Re:CoM's gate: param `+0x30`) | 15 % (Re:CoM 67), never bosses | KH1 finishers; a boss (class `CBoss…`) keeps the swing's own kind (`warp_guard` at Damage = Re:CoM, `kh1_swing_hit` at Damage = KH1) |
| Zexion | attack card → kind 10 | 10 % (Re:CoM 25) | KH1 swings |
| Xigbar | categories 0x118 (Fire, Blizzard, Gravity) | × 1.5 | those spells and sleights |
| Demyx | categories 0x210 (Blizzard, water) | × 1.75 (Re:CoM × 2) | Blizzard spells, water sleights |
| Hades | (KH1's Berserk, stage 1) | — | left out of the loop |

### Assumptions / not checked

- Not checked: in game (none of it).
- Assumptions: Re:CoM's reaction kinds 0x15 / 0x1F / 10 on Re:CoM's handler for KH1 enemies (Re:CoM's
  AI enemies: Re:CoM's own; our KH1-AI enemies take their KH1 reaction from the swing's KH1 kind, so a
  confuse or warp may not show on them); "boss" = Re:CoM's class name `CBoss…`; category 0x200 =
  water; Cure's boost only in battle (the record needs Re:CoM's card battle); at Damage = KH1 a KH1
  spell's category bits are its Re:CoM card's / sleight's.
- Checked: tests (`skill_effects::tests::route_2_masks_and_records`, `the_rewrite_stubs` (every i16 at
  Re:CoM's cells against Re:CoM's own arithmetic), `sites::tests::enemy_skill_rewrite_sites`,
  `enemy_skill_sites` (no other site overlaps a switched one)), `check_sites_rs.py` (1342 sites, 0
  mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings).

## Stage 4 (built): route 3, the direct compares

Ask (manager brief, 2026-11-04, on the owner's "Enemy Skills numbers": "Larxene - Dash: run ×1.25",
"Vexen - Auto-Life: once per battle, ¼ HP", "platinum - (first 5 hits)", "build everything"): Axel,
Larxene, Vexen, Platinum Card, Ansem and Gold Card. Combat = KH1 in Sora's story; Combat = Re:CoM
and Riku's story: Re:CoM's compares untouched (nothing armed).

Code: `crates/kh1-recom/src/enemy_skills_cmp.rs` (the sites, the stubs, the rules, tested),
`win/enemy_skills_cmp.rs` (install, arming, the revive callback); one call each in
`win::battle::manager_ctor` (arming), `win::mod` (install after Second Chance), the run factor in
`movement::speeds::RecomNumbers::run_factor` (`win::movement`).

### How (native)

- Each Re:CoM compare `slot == id` (§ 2.3) also answers true while the skill is armed, through a
  byte-checked patch per site, as Second Chance's stub: the site's instructions move into a stub
  next to the exe, which runs them and then Re:CoM's own outcome for the card. Re:CoM runs the
  effect. Where the site is the slot read (`movzx R, byte [rax+0x1E8]`), the stub puts the skill's
  id in R when the real slot holds no enemy card (≤ 0x43), so Re:CoM's own compares take the card's
  branch; where the compare's join is patched, the stub does the card's write.
- The stubs read one byte per skill and Platinum's hits left (`enemy_skills_cmp::Armed`, an
  absolute address) and compare the player with Re:CoM's main player `*0x14087B380` (rip-relative),
  so only Sora's own checks take it. Armed once per battle at the battle manager's ctor from the
  equipped skills. Nothing of ours per frame; the stubs run only where Re:CoM reaches the compare.
- Sites (`PATCH`, `CALLS`, `CHECKED`: 22, `check_sites_rs.py`: 0 mismatches). Exe scan (rel8 /
  rel32 / jcc, 8-byte pointers, 4-byte RVAs, 2026-11-04): no branch or pointer into any moved
  bytes after their first (one hit, `0x1401E71A9`, is inside a `mov eax, imm32`). Installed after
  Second Chance (Invincible's first site is inside its checked context `0x1401E71A2`).

### Per skill

| skill | Re:CoM (Checked (decomp + exe)) | patched (moved bytes) | at KH1 |
|---|---|---|---|
| Axel, Quick Recovery | the damage check `0x1401E6DE0` sets `player+0x420` bit 0x4000 when the slot is 0x6F (`0x1401E73A8 cmp edx,0x2C; cmovne`); the hit-reaction exe `0x1401EBD80` then calls `0x1401E93A0(pl, 1, 0)` every frame of the reaction: move (stick ≥ 0.3), roll, jump, commands (attack / magic / item) cut it short | slot read `0x1401E7381` (`0F B6 88 E8 01 00 00`) → 0x6F (player `rdi`) | Re:CoM's rule: Sora can act at once out of a hit reaction (Re:CoM's reaction runs at KH1) |
| Larxene, Dash | stick class `0x1401E9630`: walk / run × `xmm2` = 1.0 (`0x1406C10C0`), 1.7 (`0x1405A08BC`) with the slot 0x70; battle only (`player+0x3E8` set) | the join `0x1401E967F` (`movss xmm0,[0.9]`, re-aimed): `xmm2 × 1.25` (player `rcx`, card battle set) | **run × 1.25** (owner), walk too in Re:CoM's class. KH1's run in the move exe is ours: `RecomNumbers::run_factor` (1.25 in battle) multiplies `Kh1Sora::run_speed` at both `run_speed` settings, and `win::movement::stick_class`'s speed |
| Vexen, Auto-Life | KO init `0x1401F17F0` (`0x1401F1A85`: with 0x71 no death clear `0x140162250`), the death sequence `0x140210F10` (`0x140211032`: flag 0x10, music fade instead of the game over; its case 10 calls `0x1401F48A0(pl, 1)` → KO sub 3), the game-over controller `0x1401A16E0` (`0x1401A1A13`: `+0x88` = 1); the KO exe `0x1401EC250` sub 3 heals max HP ÷ 4 (`0x1401E59A0(pl, max/4, 6000, 1)`), sound 0x5C, motion 0x28, then 3000 dt (1 s) untouchable `+0x7BC`, and counts `0x14015E010(1, 0x28, 0x2E)` | slot reads `0x1401F1A7A` (player `rbx`), `0x140211027`, `0x1401A1A07` → 0x71; the count call `0x1401EC369` → ours | **once per battle** (owner): armed at the battle start, disarmed by Re:CoM's revive (our count callback, then Re:CoM's count); **¼ HP** = Re:CoM's own max ÷ 4 |
| Platinum Card, Invincible | the damage check `0x1401E71F3` (`cmp cl,0x7E; cmove ebx,r8d`: HP kept) and the HP loss `0x1401F90B0` (`0x1401F9129`; its only caller Zexion's `0x14041E630`, 10 a tick) | the damage check's join `0x1401E71FA` (`mov rax,[rdi+0x3E8]`) and the HP loss's store `0x1401F9163` (11 bytes): HP kept while hits are left (`lock dec`) | **the first 5 hits each battle** that would take HP do none (owner); a hit that takes no HP isn't counted; Re:CoM's reaction still plays |
| Ansem, Sleightblind | the card hides sleights from a deck (`0x140153F48`, `0x14016DEAA`): no meaning at KH1. Its play also writes Ansem's affinity record `0x1407C5C50` (100 / 100 / **50 / 50 / 50** / 100 %, attributes 0..5; 2 Fire, 3 Blizzard, 4 Thunder) into `player+0x7C4..0x7D8` (`0x140163C00` case 0x74 → `0x1401F8810`), which the damage check reads (`0x1401E7150`): the khwiki's fire / ice / thunder resistance | the read's join `0x1401E7163` (`cvttss2si r14d, xmm0`): 50 for attributes 2..4 where Re:CoM's value is its default 100 (`0x1407C5B90`) | **Fire, Blizzard and Thunder hits do half damage** (Re:CoM's Ansem record). The owner's "boss skills give the skill only" is lifted for Ansem (brief: "build that meaning") |
| Gold Card, Premium Guard | `0x14016D0B9 cmp cl,0x7D` in the play path `0x14016CEE0`: premium cards play as normal cards | — | **left greyed**: no deck, no premium cards at KH1. Owner question |

### Tags

- Checked (decomp + exe): every site, the compares' outcomes, the revive flow and its numbers,
  Ansem's record (data site), the exe scan.
- Assumption: Re:CoM's KO / revive flow runs as in Re:CoM for KH1 Sora (the KH1 frame yields to
  actions 0x1D and takes Sora back at the revive's stand `0x1401F3180`); KH1 enemies' fire / ice /
  thunder hits carry Re:CoM's attribute 2..4 in `player+0x714` (our ported enemies use Re:CoM's
  attack powers); Quick Recovery's `0x1401E93A0` commands go through our KH1 handlers (the roll /
  move / attack inits are hooked).
- Not checked: in game. KH1 has none of these skills (no 1:1 to check).

## Stage 5 (built): the deck-to-KH1 meanings

Ask (manager brief, 2026-11-04, on the owner's "build everything"): Tornado Step, Aquatank, Barrel
Spider, Air Soldier, Jafar, Air Pirate, Riku, Marluxia, Luxord, Black Fungus, on our KH1 systems.
Their Re:CoM effects act on the deck (reload, card breaks, sleight stocks), which Sora doesn't have
at KH1. Combat = KH1 in Sora's story; Combat = Re:CoM and Riku's story: nothing armed.

Code: `crates/kh1-recom/src/enemy_skills_deck.rs` (the rules, tested), `win/enemy_skills_deck.rs`
(arming, the per-frame step, the hooks into our code); one call each in `win::battle::manager_ctor`
(before stage 4's arming), `win::attack::request_bits` (the frame), `win::attack::damage_check`
(no reaction), `win::mp::pay` / `on_kh1_hit`, `win::kh1_enemy::swing_meets_attacks`,
`win::summons::sleight_running` (+ `replay_sleight`), `win::abilities::equipped_live` (Black
Fungus), `crate::sleights::pay_points` (Riku). No Re:CoM code patched; no new site.

- **Armed once per battle** at the battle manager's ctor from the equipped skills (and Black
  Fungus's). Per frame: one atomic load (`enemy_skills_deck::frame`) unless Air Soldier is armed
  or a second play waits; then two loads and, every 20 frames of running, an MP read.

| skill | Re:CoM (research) | at KH1 (built) | numbers |
|---|---|---|---|
| Tornado Step, Reload Haste | reload counter reset to 1 (`0x140167C54`) | KH1's hit charge (`0x1402BF330`) gets a bonus on its factor, as MP Haste's | **+0.75** (ours; MP Haste +1.5), KH1's 250 % cap: at equal levels a swing 3 → 5 (MP Haste 7), a finisher 6 → 10 (15); with both, the cap |
| Aquatank, Auto-Reload | an empty deck reloads by itself (`0x1401695E3`) | once a battle, the payment that leaves 0 MP points: KH1's full restore (`0x1402A56F0`: MP = max, charge 0) | once / battle |
| Barrel Spider, Quickload | the reload completes at once (`0x140167C9B`) | once a battle, the payment that empties the charge gauge below max MP: the gauge fills to the next point ((MP + 1) × 30), KH1's refill makes it the next frame | once / battle; Aquatank first when both apply |
| Air Soldier, Reload Kinesis | reload while not idle (`0x1401E6C98`) | MP charge while Sora runs (Re:CoM's run action 4) in battle | **1 unit / 20 frames** (ours: 3 a second, half of steady swinging; point 1 from 0 MP in 10 s) |
| Jafar, Attack Bracer | attack plays can't be broken (`0x14015DAF0`) | a hit during a KH1 swing (Re:CoM's attack action 0xA with a KH1 row) takes Re:CoM's no-reaction bit `+0x420 & 2` for the check (KH1's roll no-flinch path): damage, no flinch | — |
| Air Pirate, Item Bracer | item plays can't be broken (`0x14015DAE8`) | the same during a KH1 item use (Re:CoM's item action 0xE) | — |
| Riku, Sleight Lock | stocked cards locked (`0x14015266F`) | a sleight's MP: 1 point less, at least 1, Party sleights and the all-MP ones too (`sleights::pay_points_with`) | Holy 3 → 2, Magic LV3 2 → 1, Lucky Bounty 1, Idyll Romp all − 1 |
| Marluxia, Double Sleight | `+0x428 \|= 0x2000` at a sleight play (`0x1401F5B22`) | when a Sleights-list sleight ends, Re:CoM's own sleight start for it again, free (`summons::replay_sleight`), from Sora's free ground actions 2 / 3 / 4 within 30 frames; dropped by a hit reaction, the KO or the win | once per play (not the replay), battle only, not Party sleights |
| Luxord, Omni Break | plays never broken (`0x14015DAF4`, `0x140164999`) | Sora's KH1 swing wins every meeting with a KH1 enemy's attack row: the enemy gets KH1's clash record (kind 7 when its row is more than 40 under Sora's, else 6) and its row stops; Sora gets none and goes on into the body | — (KH1's own rule would leave rows at 98+ unmoved: ours) |
| Black Fungus, Random Flush | the slot becomes a random enemy card (`0x140163170` case 0x5F) | at each KH1 battle start, one random built Enemy Skill not equipped (any, learned or not, as Re:CoM's draws from all cards; never Black Fungus) counts as equipped for that battle, by Re:CoM's random float `0x14029CF00`; logged | one a battle |

### Marluxia: why not Re:CoM's Double Sleight bit (Checked (decomp))

Re:CoM's bit 0x2000 makes the sleight's end `0x1401F6660` keep the play (no `0x14015E0E0`), stand
Sora (`0x1401F2890`) and set `+0x428 |= 0x41000`; the replay is the request bits `0x1401F8150`'s
**battle branch**: with bit 0x1000 it calls Sora's sleight start `vt+0x130` again (the id still in
`+0x82C`). At KH1 our request-bits wrapper runs Re:CoM's **field** branch in battle (the KH1 Attack,
`win::attack::request_bits`'s `forced`), so the replay never comes, and `0x1000` would stay set:
other exes (`0x1402176C0`, `0x140217EE0` through `0x1401EF330`) would then call the sleight start at
their own moments. So the second play is Re:CoM's same start called once more by us, at the next
frame after the first ends (what Re:CoM's battle branch does a frame later).

### Tags

- Checked (code): every hook point is our own code; Re:CoM's actions 0xA / 0xE / 4 and
  `+0x420 & 2` as earlier stages use them; Re:CoM's Double Sleight path (decomp above).
- Ours (brief "pick"): Tornado Step's +0.75, Air Soldier's rate, Barrel Spider's meaning, Luxord's
  kinds, Marluxia's 30-frame wait, Black Fungus's pool (all built skills).
- Assumption: Re:CoM's sleight start accepts the same sleight again a frame after its end (its pack
  is still loaded); Black Fungus's skill acts through every reader of the equipped skills (the KH1
  frame, stage 4's stubs, the MP hooks; stages 2–3's readers if they read `equipped_live`).
- Not checked: in game. KH1 has none of these (no 1:1 to check).

## Left out (owner's choice 1. C, or no meaning at KH1)

- Card values: Shadow, Screwdiver, Powerwild, Sea Neon, Pirate.
- Need an enemy card to work against: Value Break, Dispel, Mimic.
- Needs the party: Hyper Healing.
- Riku's story only: Darkball.
- Open: Wyvern (Reload Lock).

## Research: Re:CoM enemy cards: how their effects are implemented (research report)

Status: complete (research only; nothing in the repo changed). Sources: Re:CoM Steam decomp
(`/home/claude/re/out/recom/`), Steam exe (capstone), `SY0002.BIN` → `BattleCardList.bin`
(owner's extract, `/home/claude/re/recomsys/`).

## 1. Mechanism

### 1.1 The active slot (one per side)

- `CCardBattle` ("CBTL") singleton `*0x14087A818`; the player's `+0x3E8` points at the same object
  (Sora's damage check and others read `player+0x3E8 → +0x1E8`). Checked (decomp).
- Per side s = side − 1 (side 1 = Sora's side, side 2 = enemies' side):
  - `CBTL+0x1E8 + s*4` (dword, the byte at +0 = enemy card id 0x44..0x7E; copied whole from the card
    object's `+0x260`) = the **active enemy card**, "NowEnchant". 0 = none.
  - `CBTL+0x1F0 + s*4` (int) = **remaining count**.
  - `CBTL+0x178 + s*8` = the card object that was played; `CBTL+0x250 + s*8` = the on-screen
    enchant gauge object (0x88 bytes, built by `0x140165030`, updated by `0x1401677F0`).
  - Checked (decomp `0x140163170`, `0x140161E10`).
- **Concurrency: exactly one active enemy card per side.** Playing a new one while one is active
  first clears the old one (`0x140163170` calls the clear `0x140161E10` when `+0x178` is set), then
  overwrites the slot. Sora can never have two enemy-card effects at once in Re:CoM. Checked (decomp).

### 1.2 Set: `0x140163170` "CBTL set enemy card" (side, card object)

Callers `0x14016CEE0` (the card-play path) and itself (Random Flush / Mimic re-entry). It:
1. returns if `CBTL+0xB8+s*4` ≠ 0 (a play in progress);
2. clears the previous card (`0x140161E10`) if any; stores the card object and the id dword;
3. plays sound 0x20 (`0x1402AA360(0x20)`), builds the gauge object;
4. **count = `BattleCardList[id].+9`** → `CBTL+0x1F0+s*4`;
5. **side 1 only (Sora's side): calls `0x140163C00(CBTL, id)`**, the player-side "apply" switch;
6. then a per-card switch for side-independent set-up (both sides). Checked (decomp + exe).

Cases in step 6 (Checked (decomp)):

| id | what it does at play |
|---|---|
| 0x4B, 0x57 | revolver scroll `0x14016C580(rev, 0, 1, 300)` (card-value cards, not wanted) |
| 0x55 Wight Knight | `CBTL+0x208 \|= 0x4000` |
| 0x59 Darkball | if `0x1401AAAE0()` ≠ 0: `CBTL+0x208 \|= 0x80000` |
| 0x5C Wizard | `0x14016D200(rev, 1, 3)` (rev = card object `+0x2C8`) |
| 0x5D Neoshadow | `CBTL+0x208 \|= 0x1000 << (other side)`; `CBTL+0x1F8 + other*4 = 0` |
| 0x5F Black Fungus | re-rolls a random card id (`rand % 0x32 + 0x44`, or `% 0x39` with story flag `0x1401B08C0(0,0x2D)`), never 0x5F / 0x66 / (0x59 without `+0x208 & 0x20000`), writes it into the slot and re-enters `0x140163170` |
| 0x62 Crescendo | `0x14016D200(rev, 1, 2)` |
| 0x64 Parasite Cage | clears the **other** side's active enemy card (`0x140161E10(other)`), then clears its own slot |
| 0x66 Darkside | if the other side has an active card, copies its id into own slot and re-enters `0x140163170` (Mimic); else clears its own slot |
| 0x6A Oogie Boogie | `CBTL+0x208 \|= 0x400 << s`; `CBTL+0x1F8 + s*4 = 0` |

### 1.3 Player-side apply: `0x140163C00` (side 1 only)

Writes Sora's fields at play time (Checked (decomp)). Also called from `0x140164C20`
(re-apply with `CBTL+0x1E8`, side 1). Player = `*0x14087B380`.

| id | write |
|---|---|
| 0x45 Soldier | `player+0x640` (combo max) = 4 (default 3) |
| 0x46 Large Body | `0x1401F8FF0(player, 1)` |
| 0x4C Bouncywild | `player+0x54C` = 30.0f, `+0x550` = 10.0f (defaults 5.0f / 5.0f) |
| 0x4E Bandit | `player+0x640` = 1 |
| 0x4F Fat Bandit | `player+0x420 \|= 0x100000` |
| 0x56 Gargoyle | `0x1401F7EC0(player, 1)` |
| 0x63 Guard Armor | `player+0x420 \|= 0x80` |
| 0x67 Card Soldier | `player+0x420 \|= 0x100` |
| 0x68..0x7B bosses (Hades, Hook, Riku, Axel, Larxene, Vexen, Marluxia, Lexaeus, Ansem, Zexion, Xemnas, Xigbar, Xaldin, Saïx, Demyx, Luxord) | `0x1401F8810(player, rec)`: six floats → `player+0x7C4..+0x7D8` = **damage-taken % per attribute** (see § 1.6) |
| 0x6D Dragon Maleficent | if `0x1401AAB00()` ≠ 0: revolver `+0x8C` = 0x43 (default 100) |
| 0x76 Xemnas | also `player+0x420 \|= 0x1000` |
| 0x78 Xaldin | also `0x1401F6FB0(player, 1)` |
| 0x79 Saïx | also `player+0x420 \|= 0x40000000` |
| 0x7C Roxas | `player+0x420 \|= 0x20000000` (its affinity record is all 100) |

### 1.4 Clear: `0x140161E10` "CBTL::resetNowEnchant" (side)

Side 1: resets everything 0x140163C00 set (affinities to `0x1407C5B90` = all 100, `+0x640` = 3,
`+0x54C/+0x550` = 5.0f, `0x1401F7EC0(p,0)`, clears `+0x420` bits 0x100000 / 0x80 / 0x100 /
0x20000000 / 0x40000000 / 0x1000, `0x1401F8FF0(p,0)`, `0x1401F6FB0(p,0)`, revolver `+0x8C` = 100 and
`0x14016D200(rev,0,0)`), and `CBTL+0x208 &= 0xFFF7BFFF` (0x4000 and 0x80000). Both sides: clears
`+0x208` bits `0x1000 << other` and `0x400 << s`; zeroes `+0x1E8`, `+0x1EB`, `+0x1F0`; frees the gauge;
`+0x178` = `+0x250` = 0. Callers: `0x14015E010`, `0x14015E0E0`, `0x14015E5D0`, `0x140161FC0`,
`0x140162250`, `0x140163170`, `0x140163EF0`. Checked (decomp).

### 1.5 Counting: `0x14015E010(side, kind, effect)`

`BattleCardList` (`*0x14087A808`, 16-byte rows; file `BattleCardList.bin`) for enemy cards:
`+0 u16` = effect index (id − 0x43), **`+8` = count kind**, **`+9` = count**, `+0xA` = CP cost
(Assumption: value fits), `+0xB` = 5. Checked (files).

`0x14015E010`: if side's slot ≠ 0, `BattleCardList[slot].+8 == kind`, and (`effect` == 0 or
`effect == slot − 0x43`), and the gauge exists: `+0x1F0` −= 1, update gauge; at < 1 → clear
(`0x140161E10`). Checked (decomp). `0x14015E0E0` / `0x140161FC0` ("resetNowPlaying", the end of a
card play) do the same for kind `'*'` (0x2A) inline.

Count kinds and their counting sites (Checked (exe): argument registers before each call):

| kind | sites (call address: side, kind, effect) | meaning |
|---|---|---|
| 0x24 `$` | `0x14015F1A3`, `0x14015F1D2`, `0x14015F21A` (resolver `0x14015EAC0`; side r13, 0): only when the played card's type nibble = 0x10 (attack card) | per attack card played. Checked (exe) |
| 0x25 `%` | `0x14015F34A`, `0x14015FB53` (resolver, after `0x140163760`) | per sleight. Assumption (strong: `0x140163760` is on the sleight path; to confirm) |
| 0x26 `&` | `0x14015ED59`, `0x14015EEAB`, `0x14015F142`, `0x14015FDC3`, `0x14015FF16`, `0x140160047` (resolver) | per card break / clash result. Assumption (sites are in the clash resolver beside the break sounds) |
| 0x27 `'` | `0x1401E73BE` (Sora's damage check, side 1, any), `0x1401F915E` (side r14, any), `0x14017DB5B`, `0x14017DC04` (side 2, any; enemy damage) | per hit taken. Checked (decomp: in the damage checks) |
| 0x28 `(` | per-card triggers with an explicit effect index: `0x1401E7233` (1, 0x28, 0x29 = Hook), `0x1401F914F` (r14, 0x28, 0x29 = Hook), `0x140160161` / `0x140161909` (1, 0x28, 0x27 = Oogie), `0x14016019D` / `0x140161942` (2, 0x28, 0x27 = Oogie), `0x14018097D` (2, 0x28, 0x29 = Hook), `0x140180AE3` (2, 0x28, 0x2E = Vexen), `0x1401EC369` (1, 0x28, 0x2E = Vexen) | per use of that card's own trigger. Checked (exe) |
| 0x29 `)` | `0x140168376`, `0x14016A5C3`, `0x14016B40C` (revolver functions; side = revolver `+0x5C`) | per reload / revolver event. Assumption (to confirm) |
| 0x2A `*` | `0x14015E0E0` / `0x140161FC0` inline; `0x14015FC69` (resolver, when a combo count > 1) | per card play ended. Checked (decomp) |
| 0 | `0x14016BF63` | never matches an enemy card (all have kind ≠ 0) |

### 1.6 The boss affinity record (`player+0x7C4..+0x7D8`)

Read in Sora's damage check `0x1401E6DE0` (after the hit rewrite): index = the hit's attribute
`player+0x714` (0..5 → `+0x7C4 + 4·attr`; 6 and up → `+0x7D8`).
Value v: 0 → no damage (reaction 5); ≥ 1000 → 100 % and `+0x420 |= 1`, reaction 3; else damage ×
v / 100. Attributes 2 Fire, 3 Blizzard, 4 Thunder (Checked (notes, m7-magic)); 0, 1, 5 not
identified. Checked (decomp + exe for the values below).


### 1.7 The hit rewrite `0x14015D5F0(CBTL, rec)` (one function, 17 cards)

A 0x20-byte record (built by `0x140160F90` + the caller): `+0` side, `+4` handle, `+8` reaction
kind (in/out), `+0xC` attribute, `+0x10` mode (1 = this side's outgoing hit, 2 = damage this side
takes), `+0x12` u16 category bits (card `+9` nibbles → `1 << (n−1)`), `+0x14` AP / damage (in/out),
`+0x16` HP, `+0x18` max HP, `+0x1A` "sleight" (attack kind 4 / 7), `+0x1C` out: guarded, `+0x1E`
"attack card" (attack kinds 1..7, or the side's current play is one attack card). Checked (decomp +
exe). It reads the slot once at **`0x14015D60A`** (`movzx edx, byte ptr [r10+0x1E8]`,
`41 0F B6 92 E8 01 00 00`), returns at once if < 0x44, and re-reads it at `0x14015D6AE` (same bytes).
Callers (Checked (decomp)): Sora's row pick `0x1401F7520` (KH1 attack-card swing), `0x1401E66A0`
(37 sites: Sora's magic / summon / sleight / item AP, mode 1), `0x1401E67B0` (mode 2), Sora's damage
check `0x1401E6DE0` (mode 2), `0x14015D8A0` (18 sites, spells / sleight shots, mode 1, side 1),
`0x14017F0E0` (enemy side). ×1.5 and ×1.3 are `imul 0x96` / `imul 0x82` then a truncating ÷100.

## 2. Per-card sweep

Tags: **C-d** = Checked (decomp), **C-x** = Checked (exe: address and bytes read with capstone),
**C-f** = Checked (files: `BattleCardList.bin`), **A** = Assumption. "Count" = `+9` with kind `+8`
(§ 1.5; C-f). Side 1 = Sora. Only Sora-side code is listed (the enemy side has its own separate
compares, e.g. `0x14017C6E2` Axel, `0x14017E56E` Marluxia, `0x14017EFB3` / `0x140185653` Larxene,
`0x140180949` Hook, `0x140180AA3` Vexen, all on `CBTL+0x1EC`; C-x).

Effect names: the exe's pointer table `0x1407C2278` (8 bytes per effect, index = id − 0x44; e.g.
`+0x28·8` → "Second Chance" for 0x6C, `+0x38·8` → "Double Strike" for 0x7C). C-x.

### 2.1 Cards that act through the play-time writes (§ 1.3 / § 1.2), read elsewhere natively

| id | card | effect | where Sora's code tests it | what it does (Re:CoM numbers) | count |
|---|---|---|---|---|---|
| 0x45 | Soldier | Combo Plus | set `player+0x640` = 4 in `0x140163C00`; read by Re:CoM's combo chain (`0x1401E9180`: index `+0x63C` < `+0x640`) | combo length 3 → 4 cards. C-d | `)` 3 |
| 0x46 | Large Body | Guard | `0x1401F8FF0(p,1)`: `+0x420 \|= 0x200`, barrier `+0x72C \|= 2`, class mask `+0x718 \|= 1`, half-angle `+0x73C` = π/2. Tested by the barrier test `0x14016EED0` (struct `player+0x690`: `+0x9C & 2`, `+0x88` bit[hit class `+0x50`], `\|angle\|` ≤ `+0xAC`) | blocks class-0 hits from the front 180°. C-d; "class 0 = physical": A | `)` 1 |
| 0x4C | Bouncywild | Draw | `+0x54C` = 30.0, `+0x550` = 10.0 (default 5.0 / 5.0); `0x14022F23A` `cmp cl,0x4C` (`80 F9 4C`) in `0x14022EE30` picks radius² = `+0x550`² (100) instead of the constant 2.25 | pick-up / attraction radius 1.5 → 10 (units: A). C-x | `)` 5 |
| 0x4E | Bandit | Combo Finish | `player+0x640` = 1 | every card is the combo's last (finisher). C-d (meaning "finisher": A) | `)` 1 |
| 0x4F | Fat Bandit | Back Attack | `+0x420 \|= 0x100000`; read at `0x1401F5533` (`bt eax,0x14`) in `0x1401F54B0` and at `0x1401F5E6F` in `0x1401F5DC0` → `CPlayerWeapon::setBackAttack` `0x140225320` (weapon `+0x2CC \|= 0x10`, `+0x28C` = 1) | weapon's back-attack mode; the bonus itself is read from weapon `+0x28C` (not traced: A) | `)` 2 |
| 0x56 | Gargoyle | Vanish | `0x1401F7EC0(p,1)`: `+0x420 \|= 0x40`; read per frame in `0x1401F9840` (`0x1401F9D11` `test byte [rdi+0x420],0x40`), `0x1401EB6E0`, `0x1401F0AB0` | Sora's alpha pulses (cos of `+0x7DC`·0.0006, 0.1–0.95); the position stored at `0x1407C5B10` only refreshes when alpha > 0.9 (enemies aiming at it: A) | `)` 1 |
| 0x63 | Guard Armor | Wide Attack | `+0x420 \|= 0x80`; read at `0x1401F5518` (`test al,al; jns`) in `0x1401F54B0` (and `0x1401F5DC0`) → `CPlayerWeapon::setWideAttack` `0x1402257C0`: weapon `+0x2CC \|= 1`, collider scale (`0x140172690`) 0.5 → 1.0 | weapon hit size ×2. C-d / C-x. Only for attack kinds `+0x638` < 8 or 0x97..0xA4 | `$` 30 |
| 0x67 | Card Soldier | Attack Haste | `+0x420 \|= 0x100`; read in the row pick `0x1401F7A27` (`test dword [rbx+0x420],0x100`, `F7 83 20 04 00 00 00 01 00 00`): motion speed `+0x47C` = 2.0f (`0x1401A65A0`, also the weapon `+0x3D0`); also `0x140221FAC` (Sora `setAttack_inFriend`), `0x1402053CD` (Riku) | swing motion ×2.0. C-x | `$` 30 |
| 0x6D | Dragon Maleficent | Overdrive | rewrite (attack card: AP ×1.5, `0x14015D6D3` `cmp dl,0x6D`); and (Sora's story, `0x1401AAB00` = flag 0x60 clear: A "Sora's story") revolver `+0x8C` = 67 (default 100), read in `0x140167AA0` (reload charge += `+0x8C`·dt/100) | attack-card AP ×1.5; Reload charges at 67 % speed. C-d | `$` 30 |
| 0x76 | Xemnas | Quick Barrier | `+0x420 \|= 0x1000`; read in Sora's damage check `0x1401E73C3` (`test dword [rdi+0x420],0x1000`, `F7 87 20 04 00 00 00 10 00 00`) | each hit taken while no barrier: barrier timer `+0x7E4` = 6000 dt (= 2.0 s at 3000 dt/s), barrier on all classes 0..5 (`+0x718 \|= 0x3F`), half-angle π (all round); a hit that still reaches the check while it runs ends it (A: blocked hits don't reach it); timer counted down in `0x1401F9840`. C-d | `)` 3 |
| 0x78 | Xaldin | Aero Guard | `0x1401F6FB0(p,1)`: `+0x7E8` = 6000, `+0x420 \|= 0x80000000`; per frame `0x1401F9840` / `0x1401EA4C0` keep a wind object (`0x140158DD0`, power = Sora's `+0x43C`) around Sora, re-hitting every 6000 dt | orbiting wind attack. C-d (the object's hit data: not traced) | `'` 3 (hits taken) |
| 0x79 | Saïx | Combo Boost | `+0x420 \|= 0x40000000`; read in initAttack `0x1401F0EBA` (`F7 83 20 04 00 00 00 00 00 40`) → `+0x664` += 20 per attack-card in the combo (added to the card power) | +20 power per combo step (cumulative). C-x | `$` 20 |
| 0x7C | Roxas | Double Strike | `+0x420 \|= 0x20000000`; read at `0x1401F5550` (`bt eax,0x1D`) → `0x140225420` (spawns a second weapon hit) | second strike per swing. C-d (its numbers not traced) | `$` 20 |
| 0x68–0x7B bosses | (secondary) | elemental affinity | `0x1401F8810` → `player+0x7C4..0x7D8`, read in `0x1401E6DE0` | % per attribute [0..5] (2 Fire, 3 Blizzard, 4 Thunder): Hades 100/100/50/1000/100/100, Hook 100/100/1000/100/50/100, Riku 100/100/50/50/50/100, Axel 100/100/0/1000/100/100, Larxene 100/150/100/100/0/100, Vexen 100/100/1000/0/100/100, Marluxia 150/50/50/50/50/100, Lexaeus 50/150/100/0/100/100, Ansem 100/100/50/50/50/100, Zexion / Xemnas / Xigbar / Xaldin / Saïx / Luxord 100/50/50/50/50/100, Demyx 100/50/100/0/50/100, Roxas all 100. 0 = immune, ≥1000 = 100 % with reaction 3 and `+0x420 \|= 1` (A: "weak / stagger"). C-x (floats) / C-d | — |

### 2.2 Cards tested by the hit rewrite `0x14015D5F0` (§ 1.7)

| id | card | effect | test site (bytes) | what it does | count |
|---|---|---|---|---|---|
| 0x47 | Red Nocturne | Fire Boost | jump table `0x14015D79D` `add eax,-0x47` (`83 C0 B9`), case → `0x14015D7C6` `test byte [rbx+0x12],8` | non-attack-card hit with category bit 8: AP ×1.5 | `)` 1 |
| 0x48 | Blue Rhapsody | Blizzard Boost | case → `0x14015D7F1` (bit 0x10) | ×1.5 | `)` 1 |
| 0x49 | Yellow Opera | Thunder Boost | case → `0x14015D7F7` (bit 0x20) | ×1.5 | `)` 1 |
| 0x4A | Green Requiem | Cure Boost | case → `0x14015D7FD` (bit 0x40) | ×1.5 (Cure's power) | `)` 1 |
| 0x51 | Search Ghost | Drain | `0x14015D6C1` `cmp dl,0x51` (`80 FA 51`) | attack-card hit: reaction kind = 0x15 (drain: A, the 0x15 handler not traced) | `)` 1 |
| 0x5A | Defender | Protect | `0x14015D630` `cmp dl,0x5A` (`80 FA 5A`), mode 2 | damage taken with attribute 0: ÷2 (`sar` after `cdq; sub`), rec `+0x1C` = 1 | `)` 1 |
| 0x5C | Wizard | Magic Boost | case → `0x14015D803` (bit 4) | ×1.3; also dims Sora's type-3 cards (`0x14016D200(rev,1,3)`: card `+0x264 \|= 0x80000`, alpha → 0.4) and refuses them at `0x14016D003` / `0x14016DC3D` / `0x14016D95B` (`cmp cl/al,0x5C`) | `)` 1 |
| 0x62 | Crescendo | Summon Boost | case → `0x14015D815` (bit 0x80) | ×1.3; dims / refuses type-2 cards (`0x14016CFD5`, `0x14016DC27`, `0x14016D96A`). Type 2 = Magic, 3 = Summon per notes (A here: the pairing reads backwards; verify in game) | `)` 1 |
| 0x68 | Hades | Berserk | `0x14015D6CA` `cmp dl,0x68` (`80 FA 68`) | attack-card hit with HP `+0x16` < max HP `+0x18` ÷ 4: AP ×2 | `$` 30 |
| 0x6B | Ursula | Shell | `0x14015D635` `cmp dl,0x6B` (`80 FA 6B`), mode 2 | damage taken with attribute 2..4 (Fire / Blizzard / Thunder): ÷2 | `'` 5 |
| 0x6D | Dragon Maleficent | Overdrive | `0x14015D6D3` | see 2.1 | |
| 0x73 | Lexaeus | Warp Break | `0x14015D6DC` `cmp dl,0x73` (`80 FA 73`) | attack-card **sleight** hit (`+0x1A` > 0): `rand % 100 < 67` → reaction kind 0x1F (A: card-break / "warp" reaction) | `$` 50 |
| 0x75 | Zexion | Confu-Strike | `0x14015D6E1` `cmp dl,0x75` (`80 FA 75`) | attack-card hit: `rand % 100 < 25` → reaction kind 10 (confuse: A) | `$` 50 |
| 0x77 | Xigbar | Shot Charge | case → `0x14015D81B` (bits 0x118) | ×1.5 | `)` 2 |
| 0x7A | Demyx | Water Charge | case → `0x14015D826` (bits 0x210) | ×2 | `)` 2 |

### 2.3 Cards with their own compare sites in Sora's code

| id | card | effect | site(s) (bytes) | what it does | count |
|---|---|---|---|---|---|
| 0x4D | Air Soldier | Reload Kinesis | `0x1401E6C98` `cmp cl,0x4D` (`80 F9 4D`) in `0x1401E6C40` (reload start) | Sora's story: a reload request with param 0 (A: "while not idle / moving") is refused unless it is active | `)` 3 |
| 0x50 | Barrel Spider | Quickload | `0x140167C9B` `cmp edi,0xD` (effect index, after `0x140167C31` `sub edi,0x43`) in `0x140167AA0` | on reload: `rev+0x152` = 0x50 and `0x14016A450` (instant reload: A) | `)` 3 |
| 0x54 | Aquatank | Auto-Reload | `0x1401695E3` `cmp r8b,0x54` (`41 80 F8 54`) in `0x140169450` | empty revolver → reload `0x14016B2B0` by itself | `)` 1 |
| 0x58 | Air Pirate | Item Bracer | `0x14015DAE8` `cmp al,0x58` (`3C 58`, side 1) / `0x14015DBB8` (side 2) in `0x14015D970` | a play containing an Item card (type 0x40): `CBTL+0x208 \|= 0x100 << side` = that play can't be broken (`0x140164910` / `0x140164A50`) | `)` 3 |
| 0x5B | Wyvern | Reload Lock | `0x140167C34` `cmp edi,0x18` (`83 FF 18`), `0x14016839C` `cmp eax,0x18` (`83 F8 18`) | the reload counter `rev+0x80` doesn't grow (each reload takes 3000·(n−1) dt more otherwise) | `)` 1 |
| 0x5E | White Mushroom | Hyper Healing | `0x140203EEA`, `0x140215AEA`, `0x14022007A` (initFriend of Riku-dark / Riku / Sora), `0x140218B75` (Sora battle exe) — all `cmp cl,0x5E` (`80 F9 5E`) | on a Friend card: heal max HP ÷ 4 (`0x1401E59A0(p, max/4, 6000, 1, 0)`) | `)` 3 |
| 0x60 | Creeper Plant | Leaf Bracer | `0x14015DAEC` `cmp al,0x60` (`3C 60`) / `0x14015DBBC` | a play containing card 0x1D (Cure) can't be broken | `)` 1 |
| 0x61 | Tornado Step | Reload Haste | `0x140167C54` `cmp edi,0x1E` (`83 FF 1E`), `0x1401683BD` `cmp eax,0x1E` (`83 F8 1E`) | reload counter reset to 1 (fastest reload) | `)` 2 |
| 0x65 | Trickmaster | Value Break | `0x14016376E` `cmp byte [rcx+rax*4+0x1E8],0x65` (`80 BC 81 E8 01 00 00 65`) in `0x140163760` (resolver) | own play value is subtracted from the opponent's cards one by one (`0x140152150`) | `%` 10 |
| 0x69 | Jafar | Attack Bracer | `0x14015DAF0` `cmp al,0x69` (`3C 69`) / `0x14015DBC0` | a play containing an Attack card can't be broken | `&` 20 |
| 0x6C | Hook | Second Chance | `0x1401E7222` `cmp al,0x6C` (`3C 6C`) in Sora's damage check; **`0x1401F913D` `cmp ecx,0x6C` (`83 F9 6C`) in `0x1401F90B0`** (a second HP-loss path, caller `0x14041E630`) | new HP < 1 and old HP > 1 → HP 1; counted `(1,0x28,0x29)` | `(` 3 |
| 0x6E | Riku | Sleight Lock | `0x14015266F` `cmp ecx,0x2B` (`83 F9 2B`) in `0x140152490` (`BC::setPlayStock`) | stocked card (types other than 1..3): card `+0x264 \|= 0x20` | `*` 5 |
| 0x6F | Axel | Quick Recovery | `0x1401E73A8` `cmp edx,0x2C` (`83 FA 2C`) in Sora's damage check: `+0x420` bit 0x4000 on/off each hit; read at `0x1401EC1E6` (`bt eax,0xE`) in the hit-reaction state `0x1401EBD80` and decomp `0x1401E0000.c:8060` | lets Sora leave the hit reaction early (to move `0x1401E93A0`) (exact window: A) | `'` 10 |
| 0x70 | Larxene | Dash | `0x1401E9672` `cmp dl,0x70` (`80 FA 70`) in the move-speed function `0x1401E9630` | ground speed factor 1.0 → **1.7** (float at `0x1405A08BC`; default 1.0 at `0x1406C10C0`) × walk `+0x10` / run `+0x14` of the param table | `&` 15 |
| 0x71 | Vexen | Auto-Life | `0x1401F1A85` `cmp al,0x71` (`3C 71`) in `0x1401F17F0` (KO); `0x140211032` (`3C 71`) in `0x140210F10` (KO state: music fade, flag 0x10); `0x1401A1A13` `cmp cl,0x71` (`80 F9 71`) in `0x1401A16E0` (game-over controller, `+0x88` = 1); revive in `0x1401EC250` | on KO: no game over; revive with max HP ÷ 4 (`0x1401E59A0(p, max/4, 6000, 1)`), sound 0x5C, motion 0x28, then 3000 dt (1 s) of `+0x7BC` invulnerability; counted `(1,0x28,0x2E)` at `0x1401EC369` | `(` 1 |
| 0x72 | Marluxia | Double Sleight | `0x1401F5B22` `cmp cl,0x72` (`80 F9 72`) in the card-message handler `0x1401F5580` | a sleight play also sets `+0x428` bit 0x2000 (second sleight) | `*` 3 |
| 0x74 | Ansem | Sleightblind | `0x140153F48` `cmp r10d,0x31` (`41 83 FA 31`) in `0x140153E80`; `0x14016DEAA` `cmp dl,0x74` (`80 FA 74`) in `0x14016DE70` (side-2 revolver only) | card `+0x264 \|= 0x10`; the enemy revolver's sleight preview is hidden | `*` 10 |
| 0x7B | Luxord | Omni Break | `0x14015DAF4` (`3C 7B`) / `0x14015DBC4` (`CBTL+0x208 \|= 0x100000` for side 1); `0x140164999`, `0x140164B43` (`41 80 BC 9x E8 01 00 00 7B`) in the break tests | the side's plays are never broken | `&` 15 |
| 0x7D | Gold Card | Premium Guard | `0x14016D0B9` `cmp cl,0x7D` (`80 F9 7D`) in the play path `0x14016CEE0` | premium cards are played as normal cards (the premium branch is skipped) | `)` 5 |
| 0x7E | Platinum Card | Invincible | `0x1401E71F3` `cmp cl,0x7E` (`80 F9 7E`) in Sora's damage check; **`0x1401F9129` `cmp al,0x7E` (`3C 7E`)** in `0x1401F90B0` | HP isn't lowered by the hit | `&` 20 |

### 2.4 Cards handled at play time only (CBTL flags / slot tricks)

| id | card | effect | where | what | count |
|---|---|---|---|---|---|
| 0x55 | Wight Knight | Float | `CBTL+0x208 \|= 0x4000` (`0x140163170`); **central getter `0x1401615E0`** (returns 1 if the bit is set), called by `0x1401A59A0` (every actor's gravity scale 1.0 → 0.5 unless actor flag 0x200000) and `0x1401FAB40` (Sora's gravity `+0x468` × 0.5) | gravity halved for everyone (either side's card) | `)` 3 |
| 0x5D | Neoshadow | Bio | `CBTL+0x208 \|= 0x1000 << other side` | Sora's: every 1000 dt each enemy object (type 0x1004) gets `vt+0xD8(…,1)` (in `0x1401617F0`; 1 damage: A) | `)` 1 |
| 0x5F | Black Fungus | Random Flush | `0x140163170` | the slot becomes a random enemy card (0x44..0x75, or 0x44..0x7C with story flag 0x2D), never 0x5F / 0x66 / 0x59 | `&` 1 (the new card's own count then applies) |
| 0x64 | Parasite Cage | Dispel | `0x140163170` | clears the opponent's active enemy card, then its own | `&` 1 |
| 0x66 | Darkside | Mimic | `0x140163170` | copies the opponent's active enemy card | `&` 1 |
| 0x6A | Oogie Boogie | Regen | `CBTL+0x208 \|= 0x400 << s`; tick in `0x1401600D0` / `0x1401617F0` | every 6000 dt (2 s), not KO'd, HP < max: heal max(1, (max − HP) ÷ 8) (`0x1401E59A0(p, n, 3000, 1, 0)`), counted `(1,0x28,0x27)` | `(` 10 |

### 2.5 Not in the wanted list / not found

- 0x44 Shadow (Incrementor), 0x4B Powerwild (Retrograde), 0x52 Sea Neon (Random Values),
  0x53 Screwdiver (Decrementor), 0x57 Pirate (All Zeros): card-value effects (`0x140151A70`
  switch on id − 0x44, revolver scroll at play). Not swept. Note: the brief's table lists 0x53, the
  design doc excludes Screwdiver.
- **0x59 Darkball (Duel Trigger)**: not in the brief's table; `CBTL+0x208 \|= 0x80000` (Sora's
  story), read by the break tests `0x140164910` / `0x140164A50` with `+0x20C` == 1 (duel). Card-only.
- **Not traced to the end** (the test site is found, the downstream number is not): Drain's reaction
  0x15, Lexaeus' 0x1F, Zexion's 10; Back Attack's bonus on weapon `+0x28C`; Double Strike's second
  hit `0x140225420`; Aero Guard's wind object hit data; Quick Recovery's exact window; Quickload's
  `0x14016A450`; Bio's damage amount.

## 3. Choke point assessment

**There is no single getter.** Re:CoM keeps one byte per side (`CBTL+0x1E8` for Sora) and reads it
in three different ways (Checked (decomp + exe), all sites in § 2):

1. **Play-time writers** `0x140163170` (both sides) and `0x140163C00` (Sora only) turn 0x45, 0x46,
   0x4C, 0x4E, 0x4F, 0x55, 0x56, 0x5D, 0x63, 0x67, 0x6A, 0x6D (reload), 0x76, 0x78, 0x79, 0x7C and
   the boss affinities into **player fields / flag bits / CBTL flags**. Their readers never look at the
   card: weapon setters `0x1401F54B0` / `0x1401F5DC0` (Wide, Back Attack, Double Strike), row pick
   `0x1401F7A27` (Haste), initAttack `0x1401F0EBA` (Combo Boost), the damage check (Xemnas,
   affinities), per-frame `0x1401F9840` (Vanish, Aero Guard, barrier timer), combo code (`+0x640`),
   the Float getter `0x1401615E0`, the Regen / Bio tick `0x1401600D0` / `0x1401617F0`.
   - Native lever: **call the same Re:CoM setters** (`0x1401F8FF0`, `0x1401F7EC0`, `0x1401F6FB0`,
     `0x1401F8810`, the `+0x420` bits, `+0x640`, `+0x54C/+0x550`, `CBTL+0x208` bits) for an equipped
     skill; no compare site needs a patch. Re:CoM's own `0x140164C20` already re-applies
     `0x140163C00` from the slot to a new player object, so "apply this card's player-side effect"
     exists natively as one call: `0x140163C00(CBTL, id)` (entry `lea eax,[rdx-0x45]; cmp eax,0x37`
     at `0x140163C14`, `8D 42 BB 83 F8 37`).
   - **Risk** (Checked (decomp)): the side-1 clear `0x140161E10` resets *all* of those fields at once
     (affinities to 100, `+0x640` = 3, radius 5.0, clears `+0x420` bits 0x100000 / 0x80 / 0x100 /
     0x1000 / 0x20000000 / 0x40000000, Guard / Vanish / Aero off, reload rate 100, `CBTL+0x208` bits
     0x4000 / 0x80000 / 0x400 / 0x2000). It runs whenever Sora's slot ends or is replaced, at battle
     end (`0x140162250`), and when an **enemy plays Parasite Cage (Dispel)**, which clears the
     opponent's side (`0x140163170` case 0x64). So field-based skills need re-applying after every
     side-1 clear (one wrapper on `0x140161E10`, or Re:CoM's re-apply after it), or the clear must
     learn to restore equipped skills.
   - Note: `0x140163C00` also applies the boss **affinity** record; an equipped boss skill would bring
     the boss's elemental resistances/weaknesses with it unless the skill skips `0x1401F8810`.

2. **The hit rewrite `0x14015D5F0`**: one function, one slot read (`0x14015D60A`, re-read at
   `0x14015D6AE`), covering 0x47–0x4A, 0x51, 0x5A, 0x5C, 0x62, 0x68, 0x6B, 0x6D, 0x73, 0x75, 0x77,
   0x7A. This is the closest thing to a choke point: every Sora hit (row pick, magic / sleight AP via
   `0x1401E66A0` and `0x14015D8A0`) and Sora's damage check go through it. But it handles **one card
   per call**: a set of equipped skills means running its body once per equipped skill (e.g. a
   wrapper that calls it with each skill's id in a scratch slot), or rewriting its tests as bit tests.

3. **Direct compare sites**: one compare per site, `cmp <reg8>, id` after
   `movzx <reg>, byte [<CBTL>+0x1E8]` (or `+0x1E4 + side·4`, or effect index `id − 0x43`). Per card:
   Second Chance 2 (`0x1401E7222`, `0x1401F913D`), Invincible 2 (`0x1401E71F3`, `0x1401F9129`),
   Auto-Life 3 (+ revive count), Hyper Healing 4, Quick Recovery 1 (sets a bit read once), Dash 1,
   Reload Kinesis 1, Draw 1 (+ fields), Double Sleight 1, Item / Leaf / Attack Bracer 2 each
   (`0x14015D970`, sides 1 / 2), Omni Break 4 (`0x14015DAF4`, `0x14015DBC4`, `0x140164999`, `0x140164B43`), Value Break 1, Reload Lock 2, Reload Haste 2,
   Quickload 1, Auto-Reload 1, Sleight Lock 1, Sleightblind 2, Gold 1, Wizard 3, Crescendo 3 (plus their rewrite cases and play-time card dimming).
   Each would need its own patch (a jump to "slot == id **or** skill equipped") — about **43 sites**
   for the whole list, each a 2–8-byte compare (`3C xx`, `80 F9 xx`, `80 FA xx`, `83 F9 xx`, …), most
   with the `cmp <reg8>,0x43; jbe` guard just before.

**Concurrency** (Checked (decomp)): one active card per side. Equipped skills (many at once) can't
be expressed through the slot. Native options are (a) the setter route for class 1, plus (b) a
per-skill "equipped" mask tested next to each class-2/3 compare; the counters (`0x14015E010`,
kinds `$ % & ' ( ) *`) only ever touch the slot, so a permanently equipped skill is never counted
down, and a real enemy card played meanwhile keeps its own count.

**Counting summary** (§ 1.5): `$` attack cards played, `%` sleights (A), `&` breaks (A), `'` hits
taken, `(` the card's own trigger, `)` revolver / reload events (A), `*` card plays ended. Durations
are counts, not time; the only timers are Regen's 6000 dt tick, Bio's 1000 dt tick, Xemnas'
2 s barrier, Aero Guard's 6000 dt re-hit and Auto-Life's 1 s invulnerability (dt = 3000 per second,
`knowledge/ReCoM/recom-timing.md`).

## 4. What I could not check

- No in-game trace: every number is from the decomp and exe, none confirmed in play.
- Meanings tagged A above (reaction kinds 0x15 / 0x1F / 10, attribute classes 0 / 1 / 5, the barrier
  class mask, the `)` / `%` / `&` count meanings, Wizard / Crescendo card-type pairing).
- Wide Attack's bit is read only through `test al,al; jns` in the two weapon setters; a broader
  hunt for other readers found none (scans of every `+0x420` / `+0x3C8` test / bt / byte access).
- The enemy-side implementations (`CBTL+0x1EC`) were listed only where met, not swept.
