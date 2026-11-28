# M10: KH1's Shadow in Re:CoM's battles

Owner's ask (2026-11-02): "And lets move on to doing phase 4, except, i only want the AI, attacks,
reactions and motions from KH1 equivalent enemies, everything else (emphasis on stats) stay the
same as they are in RECoM". The Shadow is the pilot. The decisions are in
`docs/design-philosophy-instructions.md` § Phase 4 / M10 choices.

Tags: Checked (exe / decomp / files / trace) or Assumption. Every address is Steam (KH1 FM:
`KINGDOM HEARTS FINAL MIX.exe`; Re:CoM: `KINGDOM HEARTS Re_Chain of Memories.exe`), image base
`0x140000000`. Script offsets are offsets in the player's `ex_2020.bd` (part 5 of
`xa_ex_2020.mdls`, 11648 bytes).

M11 (2026-11-02): the Shadow now runs on the shared KH1 enemy framework (`docs/recom-m11-enemies.md`):
`kh1_core::enemy` (body, reactions, script routines), `kh1_recom::kh1_enemy` + `win::kh1_enemy` (one table of
classes; this note's `win::shadow` is its class 0), `kh1_recom_assets::enemy`. Its behaviour, asset outputs
(byte-identical), manifest, log lines and trace token are unchanged. KH1's wander (`0x1BC4`) picks its points
about the current position; M10's port took them about home: fixed to KH1's rule (owner 2026-11-02, M11's
shared `wander`).

Status: in game since 0.67.0 (owner: "Everything seems to work well"); the unhittable-while-sunk fix
below is not seen in game yet; no KH1 trace of the Shadow yet.

## What was built

At Combat = KH1, in Sora's story, with `enemy_ai = kh1` (new ini key, default `kh1`, read at each
battle start; no Config row), every Shadow Re:CoM builds for a battle runs KH1's Shadow on Re:CoM's
own enemy states (decision M10 (a), native states):

| Part | Where | What |
|---|---|---|
| Behaviour | `crates/kh1-core/src/shadow.rs` | KH1's AI (`ex_2020.bd`), body physics, reactions, engine-agnostic |
| Oracle fix | `crates/kh1-core/src/reaction.rs` | KH1's kind-3 flinch for the training dummy (§ Oracle fix) |
| Asset step | `crates/kh1-recom-assets/src/shadow.rs` | `EN0001.ANN` / `.ASD` / `.SES` + `kh1-shadow.json` |
| Runtime | `crates/kh1-recom/src/shadow.rs`, `win/kh1_enemy.rs` (M11: was `win/shadow.rs`) | class swap, the state slots, reactions, clash |
| KH1 trace | `crates/kh1-trace` | enemy rows next to Sora's (`docs/kh1-trace.md` § Enemy rows) |

Re:CoM stays whole: Re:CoM battles, Riku's story, `enemy_ai = recom`, field Shadows
(`CMapEnemyShadow`) and every other enemy are untouched. The extra clips and attack records are
only ever played by our states (Re:CoM's own states play Re:CoM's clip names), so the extended
`EN0001.*` files change nothing for a Re:CoM Shadow.

## KH1's side (the oracle)

### How KH1 runs an enemy's AI. Checked (decomp)

- The script VM `0x1402CC620`; `misc0` yields (the rest of the frame), `misc8` ends a thread,
  `misc3` returns from a call. The enemy update `0x1402B7260` steps the script once per frame
  through `0x1402BCEC0`, only when `0x140297870` says the actor can act (actor flag 4 clear: no
  "action" motion, a request with bit 24) and `+0x88` bit 0 is clear. So an attack or a hit
  reaction pauses the script, which carries on after the instruction that yielded.
- Threads: `sys 20` starts one, `sys 34` restarts one at a new pc (`0x1402BD640` appends it; it runs
  the same frame). Handlers: `sys 33` (4 slots); the hit event (0) comes from `0x1402B7B90`.
- Syscalls the Shadow uses: 6 length, 9 normalise, 10 turn by degrees, 11 face now, 12 / 13
  motion play / request (`0x14029F7C0` / `0x14029F980`), 14 queue (`0x14029FA30`), 22 hit bit,
  23 attack object (`0x1402B2DC0`), 25 random float (`0x14031BA30`), 36 yaw target (the engine
  turns at stats `+0xD0` = π/16 a frame, `0x1402A8300` at the end of `0x1402B7260`), 40 move
  toward (`0x140299D60`), 43 motion length, 44 damage, 45 stats record, 49 target, 50 jump
  (`0x1402B7F20`), 51 current motion, 116 / 117 masks.

### The Shadow's script (`ex_2020.bd`). Checked (files: the player's script disassembled)

| Function | Offset | What it does |
|---|---|---|
| main | `0x26B2` | init `0x27F8`, yield; spawned from the floor → rise; idle; each frame: wait while underground; idle with Sora within 1500 → engaged; a state change restarts the behaviour thread |
| init | `0x27F8` | stats record (per world), walk 2.0 (stats `+4`), run 5.0 (`+8`), hop 150 (`+0x10`), home = position, weave phase = rand × 360, handlers |
| idle | `0xD38` → `0xD50` | clear the target; forever: wander (`0x1BC4`), dig and come up elsewhere (`0xD5E`) |
| wander | `0x1BC4` | two points around the current position (`sys 7` adds `actor+0x10`; M10 took home, fixed 2026-11-02) at [200, 300) and [300, 400); walk to each (timeout 300); 40 %: idle motion and wait rand [90, 180); back home when > 700 away |
| idle dig | `0xD5E` | sink; underground to a point [300, 500) away, then [700, 1000); rise |
| engaged | `0xB72` → `0xB96` | by distance: < 450 close, < 800 mid, else far; then keep the target (search 3000) and loop |
| close | `0xC6A` | < 150 swipe; < 300: 90 % claw else leap; else 30 % taunt else leap; then 80 % taunt; then 70 % dig near |
| mid | `0xC5A` | run (approach `0x2374`: timeout 120, re-aim every 10) to 450; taunt |
| far | `0xC1E` | taunt; underground half the way (`0x1036`); 10 %: run to 800 |
| taunt | `0xDDA` | face the target; request 203 (blend 8); wait rand [15, 75) facing it (stops if request 6 plays) |
| swipe / claw | `0xF18` / `0xF6E` | on the ground; face the target; attack row `+0x8F4` / `+0x884`; request 204 / 200 as an action, 203 queued |
| leap | `0xE12`, `0x17D4`, `0x2142` | on the ground; speed min((dist − 100) / 20, 8); 3 as an action (4 queued, held); jump height 150; in the air: steer toward the target (`+0x20` × 8) every frame; from 16 frames: rows `+0x9D4` + `+0x964` and request 201 as an action, 6 queued |
| dig near | `0xFEA` | sink; underground [200, 300) away (stop within 100), then to the target (stop within 150); rise |
| sink | `0x1328` | on the ground; request 70 (blend 8), 202 queued; height scale `+0x44` 1 → 0 over len(70) + 45 frames facing the target; underground masks |
| rise | `0x114A` | on the ground; masks back; first rise (spawned): face Sora now + alpha fade-in 30 (`0x2570`); request 69, idle queued; height 0 → 1 over len(69) − 30 frames |
| travel | `0x1056`, `0x10C6` | ≤ 60 frames; each frame the goal offset turns by sin(phase) × 60°, phase += rand × 2 + 3, glide (mode 2) at 6; stops when stuck (moved < step × 3 × 0.5) or near |
| hit handler | `0x2988` | damage: 0 → death (`0x2C78`: unhittable, fade-out 30, drops, EXP); > 0 → state "hit" (the attacker becomes the target, then engaged) |

The numbers above are read by the asset step from the player's script at the push instructions'
offsets (`kh1_recom_assets::shadow::SCRIPT_CONSTS`; a name with several pushes must read the same
at each), not written in our code.

KH1's move toward (`0x140299D60`): mode 0 walks at stats `+4` (request 1), 1 runs at `+8`
(request 2), 2 glides at `+0xC` (no motion); the facing turns by the angle to the goal, divided by
`dist / speed × 0.3` when that is over 1; the velocity is the facing × min(speed, dist); it returns
0 once within one frame's travel. Checked (decomp). Not ported: its ledge hop (stats `+0x10`).
Assumption: Re:CoM's battle arenas are flat.

Air (`0x1402AF6A0`): without steering the air velocity decays × 0.95 (√0.95 per half step);
steering adds the direction × 0.1 × speed, capped at max(current, stats `+8`). Arc: `kh1_core::Arc`.
Checked (decomp).

### Reactions (`0x1402B8E10`). Checked (decomp + disasm)

- Kind 3 on the ground (`0x1402B9230`): 0x48 + part (front) / 0x4A + part (behind: facing · push
  ≥ 0); the request already playing alternates (`(r − 0x48) ^ 1`); the facing is kept; then a push
  slide (`0x1402B7E60` → `0x1402C7300`): speed G+0x2C (80) × weight × −ln 0.8, × 0.8 a frame,
  ending below 1.
- Kind 3 in the air (`0x1402B8B90`): 0x4E / 0x4F, hop 20, air velocity dir × G+0x38 (4) × weight,
  request 6 queued.
- Kind 4: ground `0x1402B8CC0`: 0x4C / 0x4D facing against / along the push (root motion 400);
  air: 0x50 / 0x51 when the model has them (the Shadow hasn't), else the air launch.
- Kind 5 (`0x1402B8530`): speed G+0x14 + (G+0x16 − G+0x14) × weight (20 → 35), hop speed × 1.35,
  base 0x52 (ground) / 0x5C (air), +5 from behind; down time rand × 30.
- Landing (top of `0x1402B7260`): base / +1 → +2 then +3 (lying); behind +7 then +8; anything else
  request 7 then the idle. Lying counts the down time, then the get-up (+4 / +9).
- Clash (cases 6 / 7, `0x1402B9140`): the attack row's `+0x40` request (claw / swipe 207, leap 208);
  in the air a 20 hop and request 6 queued.
- The Shadow's knock-back weight (record `+0x18`) is 1.0 in all 14 world records. Checked (files).

### Motions (`xa_ex_2020.mset`). Checked (files)

52 entries, 60 fps. Requests 0x141..0x15F are filler (entry 0 without root motion) and never
played by the Shadow. HIT windows: claw 200 frames [52, 64), leap claw 201 [22, 32), swipe 204
[30, 36). Only entry 6 (request 6, 54 frames) is a bake Re:CoM has (`e001_005`, 0.006°); every
other entry is a new clip (decision M10: a Re:CoM clip only where it bakes exactly the same KH1
motion).

Attack rows: bone code (`+0x10`) through the model's code table (`MOBJ` model header, then 0x10
bytes per mesh, 0x20 bytes, 0x10 bytes, one i32 per code): 29 → bone 55 (child of 54, the right
hand), 28 → bone 74 (child of 73, the left hand). `t` (`+0x54` / `+0x58`): claw and leap 1 / 1 (a
sphere on the bone), swipe 10 → 0.1 (a line along the bone). Radius 35 (`+0x5C`). Checked (files);
that the radius is × actor scale 1.0: Assumption.

## Re:CoM's side

### The state table. Checked (decomp + exe)

`CBattleEnemy`'s ctor `0x14017AF00` builds 31 state objects at `+0x4C0` (0x40 each); each state's
begin / step / end is a thunk `mov rax,[rcx]; jmp [rax + 0xE8 + 0x18 k (+8 / +0x10)]`; attack
states' begins (2..10) also call `+0x438` (hurt bodies off), their ends `+0x440`. So a per-enemy
vtable decides what every state does. `CEnemyShadow` (vtable `0x14054D728`, 139 slots, RTTI COL at
`−8`) overrides states 0..3 and 30, its think `+0x3D0` is Re:CoM's random card attack pick
`0x140196EC0`, its update `+0x80` `0x140193420` (Re:CoM's dig fade writes the model scale, mover
`+0x70`).

### What our copy changes (`win::shadow`)

| Slot | Re:CoM | Ours |
|---|---|---|
| `+0xE8` / `+0xF0` state 0 | `0x140192850` / `0x140192B00` (dig out `e001_011`) | Re:CoM's begin minus its motion; KH1's rise; then Re:CoM's battle-start handshake (`0x1401A1680`, `0x1401A1670`, `0x1401A14B0`, default state `0x14017D6F0`). Re:CoM's alternate appearance (`+0x260 & 0x20000000008`) stays Re:CoM's |
| `+0x100` / `+0x108` state 1 | `0x140192A40` / `0x140192E60` (walk, dig) | KH1's frame with the behaviour |
| `+0x118..` states 2, 3, 4 | claw, leap | KH1's claw (2), leap claw (3), swipe (4): the motion, then Re:CoM's attack end `0x14017E540` |
| `+0x1F0` / `+0x208` states 11, 12 | flinch `0x14017C530` / blow-away `0x14017B9D0` | KH1's reaction and its phases (air, landing, lying, get-up); KH1's clash recoil |
| `+0x3B8` / `+0x3C0` state 30 | `0x140192A00` / `0x140192E30` | Re:CoM's defeat (prizes, EXP, its 1500-unit removal timer = KH1's 30-frame fade) with the killing hit's KH1 reaction motion and body |
| `+0x3D0` think | `0x140196EC0` | nothing |

Ends, the update, the message handler, the hit handler, the attack collision and every other slot
stay Re:CoM's.

- **Class swap:** wrapper on the ctor `0x140192770` (first bytes `40 53 48 83 EC 20`, its only
  caller the enemy factory `0x140188F40`): when the battle is a KH1 one (decided by the battle enemy
  manager's ctor, which runs before the enemies) and `enemy_ai = kh1`, the vtable becomes our
  copy and a 0x20-byte tail after the body (`mov ecx, 0xCE0` → `0xD00` at `0x140188FAD`) holds a
  magic, the battle number and our index. Checked (exe + decomp).
- **Attacks:** the behaviour's action enters the attack state as P1's cardless play does
  (`crate::battle::cardless_flags`, `+0x278`, `+0x68`); the begin plays the KH1 request through
  Re:CoM's motion play (vtable `+0x70` `0x14017D750`), which attaches the clip's records from
  `EN0001.ASD` (`0x14014E010`) and sets their AP from `@ENP` (`0x14014E320`: attack id 0 = Re:CoM's
  claw power for the claw and the swipe, id 1 = its leap power for the leap claw; decision M10).
  Re:CoM collides them (`0x14014E830`). The hurt bodies stay on (Stage 21's wrapper).
- **Body:** each frame the KH1 displacement is written as the mover's velocity (`+0x80`, per 60 Hz
  frame; `0x1401D4CF0` integrates `v t + a t² / 2`, so the vertical part cancels the mover's
  acceleration `+0x90` while in a KH1 arc), × the mover's speed scale `+0x13C` as Re:CoM's own walk
  does (`0x1401D6C30`); the facing goes to `+0x54`. The arc lands on Re:CoM's floor contact
  (`+0x120 & 5`). Root-motion requests play with the root flag (Re:CoM moves them).
- **Sink / rise:** the height ramp is Re:CoM's model scale y (`0x1401A4A20` / `0x1401A7050`, the
  field Re:CoM's own dig squashes). What can hit it follows KH1's flag 0x40 (§ Unhittable while
  sunk): Re:CoM's hurt bodies off (`0x1401A2D70`) from the sink's first frame to rise frame 50, on
  (`0x1401A2DB0`) after. The battle appearance turns both body sets off as Re:CoM's state-0 begin
  does; Re:CoM's state-0 end turns them on.
- **Reactions:** Re:CoM's hit handler `0x14017F7E0` applies Re:CoM's damage and picks state 11 / 12
  / 30 as always. Our begin reads the hit (the hit info `+0xF8`: the attack `+0x50`, its Re:CoM
  kind `+0x80` and attacker handle `+0x98`; the body part's contact point `+0x50`): KH1's kind from
  Sora's KH1 swing row when a swing landed (noted by the enemy-hit wrapper), else Re:CoM's kind 1 →
  3, blow-away kinds → 5 (Assumption: nearest KH1 kind); the push direction from the attacker (for
  Sora's swing turned at most 45° from his facing, KH1's rule); the part from the contact point's
  nearest of the head and hip bones (decision M10). Status states (Stop, freeze, Gravity, stun) stay
  Re:CoM's.
- **Guard:** when Sora's KH1 Guard blocks a Shadow's attack, `win::guard` hands it to
  `guard_block`: Re:CoM's attack end into state 11, which plays KH1's clash recoil (207 / 208).
  Still behind `guard_reactions` (decision 27's toggle).
- **Random:** each Shadow has its own `Kh1Rand` stream (KH1's MSVC `rand`), seeded per enemy.
  Assumption: KH1 shares one CRT stream across everything; draws are logged per enemy.
- **Stats:** HP, attack, defence, resistances, drops, EXP: Re:CoM's (`kh1-stats.json` keeps
  working: `win::stats` maps our vtable copy to `CEnemyShadow`). `enemy_pace` doesn't apply (the
  think is ours, it never asks Re:CoM for a turn).

### Time

Both games step 60 Hz frames by a delta: KH1 `0x14233FBDC` (1.0 a frame), Re:CoM the enemy's
`+0x14` (50 units a frame) × 0.02. Checked (decomp; `knowledge/ReCoM/recom-timing.md`). KH1 frame counts are
used as they are; clip ends are Re:CoM's motion clock `0x1401A47C0` against KH1's length × 50.

## The asset step

`kh1-recom-build` (no new option) now also reads KH1's `xa_ex_2020.mset` / `.mdls` and
`btltbl.bin`, and Re:CoM's `ENEMY/0001/EN0001.ANN`, `.MDL`, `.ASD`, `.SES`, and writes under the
same root as the other outputs (`<mod>/recom`, or next to `--out`):

- `ENEMY/0001/EN0001.ANN`: Re:CoM's 76 slots byte-identical; KH1 entry e at slot 100 + e
  (`e001_1NN`), 60 fps, empty slots between. Loop: KH1's pose at N equals the pose at 0 (Sora's
  rule).
- `ENEMY/0001/EN0001.ASD`: Re:CoM's header, body and three records byte-identical; four records
  added (claw on `r_hand`, swipe as a line on `l_hand`, leap claw on both hands), the trailing
  section moved after them (its offset `+0x18` updated; Re:CoM's loader `0x140179340` / binder
  `0x14014DBE0` read only the counts and offsets `+8..+0x14`). Points: KH1's bone 55 / 74 rest
  offsets in Re:CoM's hand frames (4.15 cm along the hand, the swipe's line 41.5 → 0.42 cm),
  radius 0.35 m, windows × 50 units (claw 2600–3200, swipe 1500–1800, leap claw 1100–1600).
- `ENEMY/0001/EN0001.SES`: Re:CoM's two entries kept; the claw and swipe clips get Re:CoM's claw
  sound (`e001_050`'s) one frame after KH1's hit start, the leap claw Re:CoM's two leap sounds at 0
  and hit start + 6 (where Re:CoM's sit relative to its own windows). Assumption: KH1's own Shadow
  sounds are not mapped.
- `kh1-shadow.json` (format `kh1-shadow 1`): every constant of `kh1_core::shadow::Consts` (the
  script's, btltbl's G+0x14 / +0x16 / +0x18 / +0x2C / +0x38 / +0x3C, the weight, and KH1's exe
  values gravity 0.48469388, fall cap 20, air acceleration 0.1, drag 0.95, turn π/16, down time 30,
  hop 20), the request → slot map with KH1's frame counts and root flags, the clash recoils, the
  head / hip bones (Re:CoM bones of KH1's 30 and 2).

On the session's files: 59 requests on 52 clips (51 new, entry 6 on Re:CoM's bake `e001_005`), 4
attack records; Re:CoM's head / hip bones 14 / 2. The DLL's parser
reads the step's manifest (test `asset_step_manifest_parses`).

### Owner install steps

1. Run the step as before: `kh1-recom-build --game . --out kh1-playground/build-out/PL0001.ANN`
   (or straight into the mod folder without `--out`). It prints a `Shadow (M10): …` line.
2. Copy into the OpenKH mod folder (same relative paths): `ENEMY/0001/EN0001.ANN`,
   `ENEMY/0001/EN0001.ASD`, `ENEMY/0001/EN0001.SES`, `kh1-shadow.json`, and add to `mod.yml`:

```yaml
- {name: ENEMY/0001/EN0001.ANN, method: copy, source: [{name: ENEMY/0001/EN0001.ANN}]}
- {name: ENEMY/0001/EN0001.ASD, method: copy, source: [{name: ENEMY/0001/EN0001.ASD}]}
- {name: ENEMY/0001/EN0001.SES, method: copy, source: [{name: ENEMY/0001/EN0001.SES}]}
- {name: kh1-shadow.json, method: copy, source: [{name: kh1-shadow.json}]}
```

3. Build in the Mods Manager, start the game. `kh1-recom.log` should show `shadow: KH1's Shadow
   (M10) …` at start and `battle N: Shadow … gets KH1's AI` in a Shadow battle.

## Owner test list (Combat = KH1, Sora's story, `enemy_ai = kh1`)

1. A Traverse Town Shadow battle: the Shadows rise out of the floor (KH1's rise: they grow out of
   the floor facing Sora), then walk / wander (KH1's walk), dig and come up elsewhere.
2. Stand close: swipes (very close), claws, leap claws (they jump at Sora and claw in the air),
   taunts in between. Their attacks hurt with Re:CoM's damage.
3. Stand far (8 m+): they taunt, sink, travel half way underground, rise; sometimes run in.
4. Hit one with plain swings: KH1's flinch (head / hips, front / back, alternating), a short push
   back. With a finisher: launched (front / back), lands, lies a moment, gets up.
5. Hit one in the air (after its leap): the air flinch.
6. Guard its claw: it recoils (KH1's clash motion), not Re:CoM's flinch.
7. Kill one: KH1's reaction motion, then Re:CoM's prizes and EXP; the battle ends normally.
8. Stop / Gravity / Blizzard on them: Re:CoM's status states as before.
9. `enemy_ai = recom` in `kh1-recom.ini`: Re:CoM's Shadows again (next battle).
10. Combat = Re:CoM, and Riku's story: Re:CoM's Shadows.

Please send `kh1-recom.log` (`shadow:` lines: state changes, actions, reactions, random draws) and
an F9 trace of a fight (its `m6` column carries a `shadow …` token per Shadow).

## KH1 trace (what to record in KH1)

`kh1-trace.dll` now also writes `kh1-trace-<n>-enemies.csv` next to each `kh1-trace-<n>.csv`: one
row per frame per non-party actor (the Shadow's request, motion frame, position, facing, target
yaw, walk direction and speed, arc, air velocity, push slide, scale, alpha, down time, reaction
base). `docs/kh1-trace.md` § Enemy rows. Record in a room with Shadows only (Traverse Town 1st
District early, or Dive to the Heart), one scenario per file:

1. Stand still far (≥ 10 m) from a Shadow for 10 s (its idle wander and dig).
2. Walk to about 6 m and stand for 10 s (mid range: run in, taunt).
3. Stand at about 3 m for 10 s (claw / leap choices).
4. Stand touching it for 10 s (swipes).
5. Hit it with one plain swing, from the front, then one from behind (flinch, push).
6. A full ground combo ending with the finisher (launch, landing, lying, get-up).
7. Hit it in the air during its leap (air flinch).
8. Guard its claw (clash recoil).
9. Kill one (death).

What stays unverified until those traces exist: the walk / run speeds in motion, the turn rate in
practice, the leap's arc and steering, the weave underground, the reactions' timing and travel, the
landing / lying / get-up flow, and the random choices' distribution against KH1's shared stream.

## Oracle fix

`kh1_core::reaction::Dummy` mapped kind 3 to KH1's kind-4 knock-back (0x4C / 0x4D, facing the
attacker) and kind 3 in the air to the air knock-back (0x50 / 0x51, speed 16). KH1's kind 3 is
`0x1402B9230` (0x48 / 0x4A + part, facing kept, push slide G+0x2C × weight) and `0x1402B8B90`
(0x4E / 0x4F, speed G+0x38 × weight). Fixed; kind 4 added. `kh1-data` fills the new numbers from
`btltbl.bin` (G+0x2C, G+0x38, G+0x3C). Checked (decomp + disasm).

## Not built / open

- KH1's first-rise alpha fade-in (30 frames, `0x2570`): no Re:CoM model alpha field found yet;
  the rise's height ramp plays. Assumption: minor; owner question if visible.
- KH1's ledge hop in its walk (`0x140299D60`'s floor probe): not ported (Assumption: flat arenas).
- A landing during an attack motion (the leap claw ending on the ground) keeps the attack motion
  (KH1's generic landing request 7 is not played over an action). Assumption.
- Hits that reach Re:CoM's handler without a reaction (super armour, its voice-only case) don't
  restart the behaviour's "hit" state. Assumption: none for the Shadow.
- The play's blend argument is taken as Re:CoM clock units (KH1 frames × 50). Assumption.
- KH1's motion request semantics we follow: a new play clears the queue; a queued motion plays
  when the current one reaches KH1's length. Assumption (KH1's queue `0x14029FA30` not traced).

## Fix 0.67.1: crash at the start of every battle with a Shadow

Owner (2026-11-02): "the game crashes when getting into a battle with a shadow". Log: `CRASH: exception
0xc0000005 at kh1-recom.dll+0xa4e44 (reading 0x54)`, stack `0x1402b4a49`, `0x140188fcb` (the factory, right
after the ctor call). `+0xa4e44` is in `win::shadow::ctor`: it read the facing from the mover
(`*(enemy+0xF0)+0x54`), but Re:CoM's ctor doesn't make the mover yet (`+0xF0` = 0; Checked (the log's
registers: rax 0)). The ctor now takes facing 0 when there is no mover; state 0's begin (`appear_begin`)
already takes the spawn's facing from the mover, and now skips that too if the mover is missing; the hit
reader returns "no hit" without a mover. Checked: tests, Windows build. Not checked: in game.

## Fix: unhittable while sunk (owner 2026-11-02, on 0.67.1)

Owner: "Everything seems to work well, except i can attack the shadow when its flat on the ground".

### KH1's rule. Checked (decomp + files)

- KH1's attack-vs-hurt-part test `0x1402B6AF0` runs only for actors that pass `0x140291920`:
  `flags & 0x8048 == 0x8000` (actor `+0`), and the hurt parts not masked in `+0x3A8`. The automatic
  target search `0x1402B0A00` (the lock-on task `0x1402B11B0`'s pick when nothing is locked) needs
  `0x140292070`, the same flag test. So **actor flag 0x40 set = can't be hit, not auto-targeted**.
- The Shadow's script sets and clears it through its flag helpers `0x24DA` (`|=`) / `0x24BE` (`&= ~`)
  on actor `+0`:
  - sink `0x1328`: on its first frame, with request 70 and 202 queued, flags `|= 0x20, 0x40, 0x10`
    (`0x1366..0x138C`); its ramp ends (frame len(70) + 45 = 105) with flags `&= ~0x10`, `|= 2, 0x100`,
    `&= ~1`, the `+0x3AC` mask (clr 8, set 4), and `wp+0x3C` stays 1;
  - rise `0x114A`: at its start masks back (`+0x3AC` clr 4, set 8), flags `|= 1, 0x10`, `&= ~2, 0x100`,
    request 69; at its ramp's end (frame len(69) − 30 = 50) flags `&= ~0x10, ~0x20, ~0x40`
    (`0x12D8..0x1314`; 0x40 kept only when `wp+0x44` = 1, a scripted-room flag `0x2C04` sets in one
    Traverse Town room), then `wp+0x3C` = 0;
  - init `0x27F8`, a Shadow spawned from the floor (`mdl_5a` ∉ {0, 16}): flags `|= 0x100, 0x20, 0x40`
    (`0x28B0..0x28E0`), so it can't be hit until its first rise's frame 50.
- So: **unhittable from the sink's request-70 frame until rise frame 50**, the whole time
  underground in between. Flag 0x10 (`0x140292010`: `flags & 0x8010 == 0x8000`, read by the physics
  steps `0x14029A3C0` / `0x1402998A0`) is the body push against Sora: off during the sink and the
  rise.
- Lock-on: the lock button's pick `0x1402B1540` (from `0x1402B0610`) and a held lock's check
  `0x1402B0030` / `0x1402B13F0` don't read flag 0x40; they read `+0x374` bits 1 / 2 / 0x8000,
  flags 0x200, the distance, and the `+0x3AC` mask of the lock (kind-0) points. The Shadow's
  kind-0 points are its collision block's points 0 and 1 (init masks point 1); the sink / rise
  masks bits 4 / 8 are its kind-2 points 2 and 3 (Checked (files): `xa_ex_2020.mdls` part 4,
  group 3: kinds 0, 0, 2, 2). So in KH1 the lock button **can** lock a sunk Shadow and a held lock
  stays; only the automatic target skips it.

### What changed

- `kh1_core::shadow`: a new command `Hittable(bool)` at exactly those three points (sink frame 0
  off, rise frame 50 on, a floor spawn off at init); `Underground` is log only now (it was the hurt
  switch, at the sink's frame 105 and the rise's frame 0: the owner's bug). Test
  `unhittable_from_the_sink_to_the_rise_ramps_end` checks the frames.
- `win::shadow`: `Hittable` switches only Re:CoM's hurt bodies (`0x1401A2D70` off / `0x1401A2DB0` on,
  every body, as its dig). The lock-on flag (`+0x154` bit 0x10000, the only filter of Re:CoM's lock
  pick `0x1401CDB90`) stays on: Sora can lock a sunk Shadow, as with KH1's lock button.
- No interaction with boss revenge (Stages 106 / 114): its hurt-off and attack-end hooks act only on
  the 12 boss vtables (`revenge::boss_by_vtable`), and our calls go to `0x1401A2D70` / `0x1401A2DB0`
  directly, not through the attack-state `+0x438` / `+0x440` they wrap. The Shadow's tail is its own
  allocation site (`0x140188FAD`), not a revenge tail.

### The owner's log and traces (0.67.1). Checked (trace + log)

- Trace 18: three hits landed during a sink: on a Shadow 26 frames into request 202 (frame 87 of
  the 105-frame sink), on one 28 frames into request 70, on one 4 frames into 202 (frame 65). With
  0.67.1 the hurt bodies went off only at the sink's frame 105 and back on at the rise's frame 0.
  The log also shows a flinch during a rise (`e162`, right after `motion 69`). KH1 can't be hit at
  any of these frames. After such a hit the Shadow flinched, then stood in its idle for the rest of
  the sink loop (45 frames in trace 18) before rising: also gone with the fix.
- Everything else matches KH1's script: the sink 70 runs its 60 frames then 202; the rise 69 hands
  over at frame 51 (ramp 50 + the ground wait); the jump start 3 runs 31 frames, the fall 4 16
  frames, then the leap claw 201 (`leap_attack_after` 16) for its 70 frames, request 6 after; the
  leap speed is always 8 (the cap: every leap was from 300–450 away); the claw 200 runs 93 frames
  (92 + the attack-state frame), the swipe 204 87; flinches 0x48 / 0x49 run 43 frames and alternate
  when hit again in the flinch; a finisher launched (0x52, then 0x53, down time logged); a Guard on
  a leap claw played clash recoil 208 in the air, then the landing 7 (15 frames), then the idle.
  Re:CoM's damage (10 per swing, 15 for the finisher) on Re:CoM's HP 33.
- Not a game bug: the trace's `shadow` token kept listing defeated Shadows (`73` / `84` held to the
  end of the file) because the runtime outlives the Re:CoM object. It now stops listing one 31
  frames after its defeat (Re:CoM's 1500-unit removal timer) and shows `h0` / `h1` (hittable).
- The log cap (4000 `shadow:` lines) filled with the underground weave's draw (one a frame): raised
  to 20000.

### Not built / open (this fix)

- KH1's automatic target (soft target with nothing locked) skips a sunk Shadow; Re:CoM's lock pick
  doesn't distinguish (one flag). Re:CoM's soft target is Re:CoM's (Stage 73). Assumption: minor.
- KH1's flag 0x10 (Sora's body pushes through the Shadow during the sink and the rise): Re:CoM's
  body push isn't mapped to it. Assumption: not noticeable.

Checked: tests (kh1-core 45, DLL 441), Windows build (no warnings), pefile (697 sites, 0 mismatches).
Not checked: in game.
