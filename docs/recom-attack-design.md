# M5 design: KH1's Attack inside Re:CoM, with no cards

Research notes for ROADMAP M5 (combat = KH1). No code has been written. All addresses are for the Steam
exes. Tags:
- **Checked (exe)**: bytes read from `recomsteam.exe`.
- **Checked (decomp)**: code read in the Steam decomp.
- **Checked (files)**: read from the owner's game files.
- **Assumption**: inferred, not yet proven.

Units: KH1 uses cm with y down. Re:CoM uses metres with y up, so multiply by 0.01 and flip y
(`kh1-recom-assets.md`). Time: both games tick at 60 Hz. One Re:CoM frame is 50 time units
(`recom-timing.md`). Gameplay code reads the frame as `motion time / 50`.

## 1. The attack-row table `0x1407C6C70`

### Findings

- **Size.** The table has **188 rows** (ids `0x00`–`0xBB`), 16 bytes each. It ends at `0x1407C7830`,
  where the card-time table `{0, 1000, 1500, … 4000}` starts. Checked (exe).
  - The "other rows `0x1407C7730..`" used by sleights in `recom-combat.md` are this same table at
    ids `0xAC`–`0xB4`. Checked (exe + decomp, `0x140210xxx`).
- **Who reads it.** No reader checks bounds. Checked (decomp).
  - `0x1401F7520`: the Sora/Riku row pick (vtable `+0x128`).
    - Sora's ids come from `0x1405A1708[situation]`, which gives ids 1–7.
    - Riku's ids come from `0x1405A3080` / `0x1405A30A8` (ids `0x97`–`0xA4`). Which table is used
      depends on `FUN_1401AAB00` (flag `0x60`).
  - `CPlayerWeapon::onAttack` `0x1402250B0` reads the hit effect from byte `+2` **by id**.
  - The weapon volume update `0x140225840` reads the same byte by id, and switches on the id for
    special shapes (`0x61`–`0x63`, `0x73`, `0xB1`, `0xB3`, `0xB4`, `0xB7`, `0xB8`, `0xBA`, `0x30`–`0x34`).
  - Friend code `0x1401DA710` reads rows by `+0x26C` id.
  - Sleight and friend code point `player+0x3C8` straight at rows.
- **What the player code uses.** Sora's attack reads the row only through the pointer
  `player+0x3C8`. The id `player+0x638` is passed to `onAttack`, which uses it for the effect byte,
  the special shapes, and the wide/back/double modifiers (ids < 8). Checked (decomp).
- **Row fields as used by the attack code.** Checked (decomp + disasm `0x1401EA6F1..`):

| byte | meaning |
|---|---|
| +0 | action id, an index into Sora's action table `0x1407C5E80` |
| +2 | hit effect, looked up by id, not through the row pointer |
| +4 / +5 | hit window, start ≤ frame < end |
| +6 / +7 | homing / turn window |
| +8 | homing stop distance × 0.1 m |
| +9 | homing speed (÷60) |
| +0xA | card power column; not read when card id = 0 |
| +0xB | hit kind (`+0x668`) |
| +0xC | forward push ÷60, when flag `0x10` is set |
| +0xD | hop speed or lunge (`+0x5D4` × `DAT_14087B39C`; situations 5/6/7 only) |
| +0xF | flags: 1 chainable, 4 stop homing on reach, 8 early cancel, `0x10` push |

  - Bytes **+1, +3 and +0xE are not read** by Sora's attack paths.
  - Flag **`0x80` is unused in all 188 rows**. Checked (exe).

- **Re:CoM already plays KH1's combo motions.** Checked (exe action table + `kh1-motions.json`):

| Re:CoM id (situation) | action → clip | KH1 motion (row) | Re:CoM hit | KH1 hit | KH1 combo window |
|---|---|---|---|---|---|
| 1 basic | `0x30` → p001_055 | 200 (13) | 12–26 | 14–24 | 22–40 |
| 2 side | `0x31` → p001_056 | 201 (11) | 12–32 | 18–32 | 26–42 |
| 3 far | `0x32` → p001_057 | 202 lunge (14) | 12–46 | 18–30 | 26–54 |
| 4 finisher | `0x33` → p001_058 | 203 (8) | 22–52 | 26–38 | none |
| 5 target above | `0x34` → p001_059 | 205 (17) | 8–26 | 10–20 | 18–32 |
| 6 air | `0x35` → p001_060 | 204 (18 / hop 2) | 4–16 | 6–20 | 18–34 |
| 7 air finisher | `0x36` → p001_061 | 206 (16) | 10–42 | 12–38 | none |

  The clips are Square Enix's 30 fps bakes of KH1's motions, with the same length in 60 Hz frames.
  Under the owner's rule (decision 10), Sora keeps these clips. Only the timing data changes.

- **What KH1 needs that Re:CoM rows lack.**
  - **A combo window.** Re:CoM's chain gate is "frame > row[4] and time > 500". KH1 uses its own
    start and end from the mset flag track `0x50000`.
  - **Hop height and air velocity.** KH1 air rows use hop heights of 20 / 40 / 60, with the eased
    rise and fall of KH1's jump, and set the air velocity to 8 / 4 / 12 per frame. Re:CoM's `+0xD`
    hop uses Re:CoM's gravity instead (`0x1401F64D0`).
  - **No homing.** KH1's ground lunge travel is root motion only, and KH1 sets the facing once.
    Checked (KH1 decomp `0x1402A68F0`).
  - **No "free" frame.** KH1's combo motions have no free track (`kh1-sora-data.md`).
  - Re:CoM keeps its timing in this hard-coded row table. Its EET/SES files are per-clip effect
    and sound tracks, so the trails and sounds of the reused clips stay native. Checked (decomp,
    `CEffectMotion` / `CSEMotion`).
- **A post-attack lockout.** The row pick sets `player+0x654 = duration/speed + 1000` (20 frames
  more than the motion), and the field input refuses Attack while it is non-zero. KH1 has no such
  lockout: a fresh combo can start as soon as the motion ends. Checked (decomp `0x1401F7520` tail,
  `0x1401F8150`).

### Design

- **Leave the table where it is; nothing is relocated.** KH1 rows go in a new 16-byte-row table
  (mod data) in Re:CoM's own layout:
  - `+0x3C8` points at the KH1 row.
  - `+0x638` is set to the Re:CoM id that plays the same clip (1–7 above), so the hit effect, the
    blade shape and the modifiers stay Re:CoM's own.
- **KH1 rows** copy the Re:CoM row of the same clip, with these changes:
  - `+4`/`+5` = KH1's hit window;
  - **flag `0x80` = "KH1 row"**, with `+1`/`+3` = the KH1 combo-window start and end (0/0 = none);
  - `+6`/`+7` = 0, so there is no homing;
  - `+0xD` = KH1 hop height (via the M4 jump mover);
  - `+0xB` = the Re:CoM kind (§5);
  - `+0xF` bit 1 = chainable.
- **New ids above `0xBB`** would make `onAttack` read the card-time table as an effect id. They
  need a relocated table, so they are avoided.

## 2. Starting an attack with no card

### Findings (Checked (decomp))

- **`initAttack` `0x1401F0D90`** (action `0xB`):
  - reads `BattleCardList[+0x7F4].+8` into `+0x65C`. With card 0 this is row 0 of the card file;
    its value is an Assumption, harmless if 0.
  - Then it calls vtable `+0x128`.
- **The row pick `0x1401F7520`** reads the card only when `+0x7F4 ≠ 0`: the power column, the
  attribute, the flags, and the `+0x3E8` AP rewrite `0x14015D5F0`. With card 0, AP = `+0x43C` and
  attribute 0.
- **`exeAttack` `0x1401EA6A0`** chains only through the card bits in `+0x428` (`0x10` = attack). It
  sets those bits from the card message or from `0x1401E9180`, which plays the next card from the
  revolver. **With no card, the battle attack can never chain.**
- **The field swing `0x1401F0F70` / `0x1401EAEB0`** (action `0xA`) has **Re:CoM's own cardless
  combo**:
  - when frame > `row[4]` and time > 500;
  - and the row is chainable (`+0x648`);
  - and the swing has hit (`+0x644`, set by `procAttackCollision`);
  - and `+0x63C < +0x640` (default max 3);
  - then a confirm press sets `+0x650`, and the next exe call chains.
  - That is KH1's rule: "a press is dropped if the last swing hit nothing", and the finisher is the
    press that reaches the max.
- **The Battle exe has no case for `0xA`.** Its jump table is at `0x1401FA050` (ids 1–`0x24`); the
  entry for `0xA` is the default `0x1401F9CC9`. The Field table is at `0x1401FA5F4` (ids 1–`0x1F`);
  its `0xA` entry is `0x1401FA269` → `0x1401EAEB0`. Checked (exe).
- **Input.** `0x1401F8150` sets `+0x424 |= 4` on a confirm press only on the field branch
  (`+0x390 == 0`). In battle, the attack bit comes only from card bits. The battle branch also
  holds the deck scroll, reload, stock and sleight inputs.
- **Re:CoM's own picker** works by situation:
  - distance 3.5 m;
  - angle 45° / 135°;
  - target above;
  - airborne (`0x1401F3C00`).
  
  It differs from KH1's picker:
  - ranges 1.8 / 3.0 m from KH1's 180 / 300;
  - cones 22° / ±120°;
  - "lowest flagged row whose motion differs from the current one";
  - air rows.

### Design

1. **Input.** At KH1, battle input takes the field branch: confirm sets `+0x424 |= 4`. A patch at
   the `+0x390` test in `0x1401F8150` does this, and also skips the card button `0x1401E8FF0`.
   This alone removes the deck scroll, reload, stock and sleights (M6).
2. **The action.** Action `0xA` becomes the one Attack action in both states:
   - The Battle table entry for `0xA` → a 13-byte stub in an exe code cave
     (`mov rcx,rdi; call 0x1401EAEB0; jmp 0x1401F9CC9`). That is data plus one stub, swapped at
     room load.
   - The route sites that pick `0x1401F0D90` or `0x1401F0F70` by `+0x390` (`0x1401E93A0`,
     `0x1401EEDA0`, `0x1401EE360`, `0x1401EDA60`) choose `0x1401F0F70` at KH1.
3. **The KH1 picker** goes in **Sora's vtable slot `+0x128`** (`0x1405A9938 + 0x128`), only at KH1.
   It runs once per swing and does this:
   - computes KH1's row from Re:CoM's target data (lock-on `+0x84C`, the enemies' body shapes);
   - sets `+0x630`, `+0x638`, `+0x3C8` and AP;
   - sets `+0x654` to KH1's value (no 20-frame lockout);
   - plays the clip through `0x1401F7180`.

   Combo Plus = `+0x640`, set to 3 + copies.
4. **The KH1 combo window.** A code-cave patch at `0x1401EAF95` (the gate in `0x1401EAEB0`):
   - if `row.f & 0x80`, the press is accepted when `row[1] ≤ frame < row[3]`, with no 500 test;
   - else Re:CoM's own test runs.

   Re:CoM rows never set `0x80`, so the Re:CoM setting is untouched. Nothing of ours runs per
   frame.
5. **Air rows** need M4's KH1 air mover (hop arc, velocity 8 / 4 / 12, drag). M5 air rows depend on
   M4.

**1:1 check still open.** Re:CoM's press sets `+0x650` and chains on the **next** exe call. Whether
KH1 also starts the next swing one frame after the press depends on when the command menu
(`0x140293480`) runs relative to the actor task. That is not read, so it is to be settled by an M1
trace. The KH1 window ends (inclusive vs exclusive) must follow `kh1-core`'s convention. Assumption
until a trace confirms it.

## 3. Hit collision

### Findings (Checked (decomp))

- The shape types are made by `0x1401720E0`:
  - 1 point;
  - 2/8 sphere (`CSphereCollision`, swept from `+0x10` to `+0x20`);
  - 4/5 column;
  - **7 `CPoly3Collision`**;
  - 9 model;
  - `0xB` circle;
  - `0xC` infinite.
- An attack shape may be one of types `0x1986` (1, 2, 7, 8, `0xB`, `0xC`), per `0x14016F720`.
- **The Keyblade is type 7: a thick triangle.**
  - Its corners are `+0x10` (last frame's tip), `+0x20` (this frame's tip) and `+0x80` (the hilt,
    weapon bone 0, or the player for ids `0x30`–`0x34`).
  - Its radius is `+0x70`: 0.5, or 1.0 when wide.
  - The tip is `(−0.8, 0, 0)` on weapon bone 2 (`0x1407C7E50`).
  - The test against a sphere body (`0x140177480`) runs Re:CoM's swept-sphere primitive
    `0x1401777E0` on the edges, then a triangle-face test.
  - The pair dispatch is `0x14016EAD0`.
- **KH1's volume** (`kh1-sora.md`, `shape.rs`) is different. It is a sphere chain from hilt to tip
  (Kingdom Key radius 40 cm = 0.40 m, spacing = radius), plus a chain from last frame's tip to this
  frame's tip. The first tested frame starts from the owner's body centre.
  - As shapes: KH1 ≈ capsule(hilt, tip) ∪ capsule(last tip, tip).
  - Re:CoM's triangle has both of these edges, plus the third edge and the face. So Re:CoM's shape
    is larger.
  - A sphere chain spaced at its radius also dips 0.13·r between spheres (≈ 5 cm).
- **One hit per swing.** `onAttack` gives the attack a new serial (`0x14016F7D0`). Each body skips
  a serial it has already taken (`0x14016F960`). `onAttack` runs once per window, while
  `weapon+0x23C == 0`.
  - This is the same as KH1: one hit per target per attack object, with interval 0 for swings.

### Design

- **A new shape type, `0xD` "blade chain", added to Re:CoM's collision:**
  - create it in `0x1401720E0`;
  - allow it in `0x14016F720`'s mask;
  - give it new cases in `0x14016EAD0`.

  Its pair test runs KH1's exact sphere list through **Re:CoM's own primitives**: `0x1401777E0`
  for spheres and `0x140176FA0` for columns. `0x140225840` fills the points at KH1's blade
  fractions, with radius = data `+0x5C` + scale × weapon radius (0.40 m for the Kingdom Key). The
  test runs once per tick, only during hit windows.
- The blade ends come from KH1's weapon record (`t = −2.96` bone lengths, Kingdom Key). The asset
  step should compute them on Re:CoM's `WeaponR` bone. Assumption: that the Re:CoM Keyblade's bone
  lengths match KH1's. Check it in the M2 pose capture.
- The targets stay **Re:CoM's own hurt shapes** until M10/M11.

## 4. Damage

### Findings

- **Re:CoM's damage** is `AP` (no card: `+0x43C`), plus a back bonus or `coll+0x4A`, times the
  weak-part multiplier, then `int(hit × resist% × f290 / 100)`. HP is at `+0x274`. Veto
  `0x140180920`. Checked (decomp, `recom-combat.md`).
- **Re:CoM enemy stats.** They are the `@ENP` resource: type `0x15`, id = type × 10000 + variant,
  loaded by `0x14017FF50`. Checked (decomp).
  - Per-world rows of 16 bytes, picked by world (`0x1402337D0`):
    - `+0` HP × difficulty % (`0x14052B4C8` = 75 / 100 / 100 / 100);
    - `+4` attack × difficulty (`0x14052B4D8` = 0.8 / 1.0 / 1.2 / 1.0);
    - `+8` → `+0x2E4`, which is used as the prize count.
  - The header holds:
    - resist floats `+0x14`..`+0x24` (attributes 0–4);
    - `+0xC` (can be blown away);
    - `+0x28` / `+0x38` (reaction gates);
    - `+0x40` (gate for percent-of-HP damage).
  - **No Defence, Strength or level.**
  - Which file holds `@ENP` (`EN####.EPD`?) is an Assumption.
- **KH1's damage** (`damage.rs`, `0x1402BFA80`):
  - `curve[clamp(Str + power − Def, 0, 41)]`;
  - power: 0 for swings, +4 for the finisher;
  - a critical hit chance;
  - min/max limits and the enemy's own damage floor/cap;
  - `× resist[elem]`;
  - at least 1.
- **Numbers to frame the question.** These come from KHWiki
  (https://www.khwiki.com/Shadow), not from the files: Assumption until read from the game data.
  - KH1 Shadow in Traverse Town: HP 12, Str 8, Def 8. The page's two KH1 tables disagree
    slightly.
  - Re:CoM Shadow at 1F: HP 33, AP 2.
  - So a KH1-curve swing at Traverse Town strength (Str ~10–15 → 5–8 damage) takes a Re:CoM
    Shadow down in about 5–7 hits. In KH1 it takes about 2.
  - Soldier and Large Body numbers were not readable from the wiki. The real values need the
    player's `@ENP` files and KH1's enemy data. Where KH1 keeps enemy stats was **not found** in
    `btltbl.bin` (Assumption: per-enemy or AI data).

### Design

- The swing carries KH1's Str + power in the AP field (`+0x660` → `coll+0x48`). A "KH1 damage" mark
  on the attack makes a wrapper on `0x14017DAB0` compute KH1's formula, using the enemy's KH1
  Defence and floor/cap from **a new per-enemy, per-world stats row** (an `@ENP` extension made by
  the asset step).
- The rest stays Re:CoM's: the HP field, the veto, death, and the `0x14017D260` reaction call.
- Whether Re:CoM's back bonus, weak-part multiplier and boss decay `f290` apply to KH1 hits is
  part of the owner question below.

## 5. Reactions

### Findings (Checked (decomp) `0x14017D260`, `0x14017D800`)

- Re:CoM kind 1 → flinch state (`+0x780`), unless the enemy has super armour (bit 21: sound only,
  `0x140181B50`).
- Kinds 2/3/`0x12`/`0x1D`/`0x1E` → blow-away state `+0x7C0`, when `@ENP+0xC > 0`. Otherwise flinch.
- Kinds 4/5/6/… → other states.
- Re:CoM's own swings use kind 1, and its finishers kind 2 (rows 4 and 7).
- KH1:
  - kind 3 is a flinch;
  - kind 5 is a launch: speed 20 + 15 × weight, hop × 1.35, down for rand × 30 frames.
- Re:CoM's blow-away speeds live in each enemy's state code. They were not read.

### Design

- Map KH1 kind 3 → Re:CoM kind 1, and KH1 kind 5 → Re:CoM kind 2. Re:CoM's own enemies then react
  with their own flinch and blow-away states.
- KH1's exact launch arrives with the KH1 enemies (M10/M11).

## 6. The field swing and battle entry

- **Checked (decomp) `0x14019E020`.** A map enemy's hit handler switches on the hitter's kind
  (`body+0x80`):
  - 7 / `0x2A` → stun state 7;
  - 8 → state 6;
  - `0x1C`–`0x1E` / `0x24`–`0x26` → state 5;
  - **any other kind (1, 2) → state 4**. State 4 pauses the world, then after its timer sets flag
    `0x10` and starts the encounter: type 2/3, enemy HP × 0.9 (× 0.2 with room card `0xB1`, halved
    for type 3).
- So the plain Re:CoM swing (kind 1/2) is the first-strike hit. This settles the open point in
  `recom-battle-entry.md`.
- **Design.** KH1 rows carry kind 1 or 2, so a KH1 field hit gives Re:CoM's state 4 and first
  strike natively (owner decision 3).
- Hits also roll a card drop: `0x14017E190` → `0x1401602A0` → vtable `+0x428`. See M6.

## Risks

- **Enemies mid-card take no damage** (M6 §8). This changes the M5 "damage on a Re:CoM enemy" check.
- Air rows depend on M4's KH1 air mover.
- The one-frame chain delay is not yet compared with KH1.
- The asset step must give the blade-chain ends (Keyblade bone lengths).
- Re:CoM enemies keep their own hurt shapes, so contact distances differ from KH1's until M10.

## Owner questions

1. **How much damage KH1 attacks deal to Re:CoM's own enemies until M10/M11:**
   - (a) KH1's formula, with HP and Defence from the KH1 equivalent of that enemy at the matching
     world. For example, a Shadow on 1F uses KH1 Traverse Town's stats. Re:CoM-only enemies get a
     Defence per floor.
   - (b) KH1's formula with Re:CoM's own HP, and a Defence per floor (more hits than KH1).
   - (c) Re:CoM's damage (attack stat × 100 %) until the KH1 enemies exist.
2. **Re:CoM's damage extras on KH1 hits** (back-attack bonus, weak-point multiplier, boss combo
   decay):
   - (a) none, as in KH1;
   - (b) keep them;
   - (c) a Config toggle.
