# KH1's Attack inside Re:CoM's attack action (roadmap M5)

Steam addresses (Re:CoM image base `0x140000000`). Tags: Checked (decomp / exe / files / tests) or
Assumption. Code: `crates/kh1-recom` (`attack.rs`, `movement.rs`, `win/attack.rs`). Design:
`recom-attack-design.md`. Oracle: `crates/kh1-core` (`combo.rs`, `controller.rs`, `shape.rs`,
`damage.rs`).

## Status

- Built, `combat = kh1`, main `CPlayerSora` only (Riku and `combat = recom` pass through):
  - KH1's ground and air combo through Re:CoM's own attack action `0xA`, in the field and in
    battle;
  - KH1's rows, hit windows and combo windows; KH1's picker, counter and last-swing-missed rule;
    Combo Plus, Air Combo Plus and Combo Master from `kh1-recom.ini`;
  - KH1's Keyblade sphere chain, tested with Re:CoM's own collision primitives;
  - KH1's card-priority rule (hits land during enemy card plays);
  - KH1's reactions mapped to Re:CoM's kinds;
  - the M4 follow-ups: High Jump on Re:CoM's rows 0x14 / 0x15, the roll's root motion one update
    late, KH1's no-flinch roll protection;
  - Re:CoM's springs keep their own jump (`recom-kh1-movement.md` § Springs);
  - the F9 trace's attack columns.
- Stopped, with the reason below: KH1 damage numbers (KH1's per-enemy stat records aren't in the
  files read so far), Guard's repel and Counterattack, MP charge.
- Checked (tests): on a fake Re:CoM that does what Re:CoM does around the handlers (the motion
  clock, root motion one update late, the mover), the handlers match kh1-core frame for frame
  (position, facing, state, lock, rows started, refusals) for:
  - the ground combo 13 → 11 → 8 and a new combo;
  - the last-swing-missed refusal, and the chain when the swing hits;
  - the air combo 18 → 17 → 16;
  - a landing cutting an air swing;
  - Attack and jump on one frame (the jump wins);
  - the Dodge Roll (now exact, root motion included).
- Not checked in game. Tester thread `testers/recom-attack/thread.md`.

## How it sits in Re:CoM

### The path of one swing

1. **Input.** The request-bits builder `0x1401F8150` runs its field branch in battle too (our
   wrapper sets `+0x390 = 0` for the call; Re:CoM's battle-branch early returns are kept). So the
   confirm press sets `+0x424 & 4` (with Re:CoM's own ability bit `0x41C & 0x40` and lockout
   `+0x654` tests), and there's no card scroll, reload, stock or sleight input. The use-card button
   `0x1401E8FF0` is skipped. A jump on the same frame clears the Attack bit (KH1:
   `if !jump { attack } else { refused }`). Checked (decomp).
2. **Dispatch.** Re:CoM's own command dispatch starts it:
   - the move exe `0x1401EEDA0` calls the field-swing init `0x1401F0F70`, or in battle initAttack
     `0x1401F0D90`, which our wrapper turns into the field-swing init;
   - in the air, M4's jump-exe hook runs Re:CoM's air commands.
3. **KH1's frame decides.** Our field-swing-init wrapper runs KH1's frame (`Kh1Sora::frame`) with
   the press. KH1's command `0x1402A7510` applies the counter rules and the picker
   (`kh1_core::combo`). The row exec `0x1402A68F0` does the hop, the air speed and the facing.
4. **Re:CoM starts it.** `Host::start_swing` runs Re:CoM's own field-swing init, whose row pick
   (Sora's vtable `+0x128`, our slot) takes the chosen KH1 row:
   - `+0x3C8` → the KH1 row, `+0x638` / `+0x630` → the Re:CoM id of the clip;
   - the clip via `0x1401F7180` (root motion as KH1's request map has it);
   - AP, kind and attribute;
   - `+0x63C` / `+0x640` = KH1's counter / length;
   - `+0x654 = 0` (KH1 has no 20-frame lockout);
   - `+0x4E0 = 0` (a swing starts from standing: KH1 sets speed 0, root travel only);
   - `+0x648 = 0` (Re:CoM's own chain gate never fires).
5. **Each frame (action `0xA`).** Our wrapper on the field-swing exe `0x1401EAEB0`:
   - (a) runs KH1's frame first: the combo window, the next press (it chains through steps 3–4 on
     the same frame, as KH1 does), the facing turn π/16, the air arc;
   - (b) runs Re:CoM's own exe: the hit window and `procAttackCollision` → `onAttack`, its end at
     the clip's end;
   - (c) writes KH1's velocity and facing again.
6. **Battle.** The Battle exe had no case for action `0xA` (its table `0x1401FA050` sends ids
   without a case to `0x1401F9CC9`). Its entry for `0xA` now points at a 13-byte stub in the int3
   cave `0x1401FA6C1` (after a `ret`, before `0x1401FA6D0`): `mov rcx,rdi; call 0x1401EAEB0; jmp
   0x1401F9CC9`. That is what the table's own cases do. Checked (disasm `0x1401F9959..0x1401F998A`):
   `rdi` is the player, `esi` and `xmm6` are callee-saved.
7. **The end.** When KH1's swing motion ends into its queued idle / fall, a landing cuts it, or a
   walk-off replaces it, `Kh1Sora::leave_swing` calls Re:CoM's `0x1401F6660`, which ends its attack
   (the weapon's volume, effects, fields), and KH1's next motion plays. If Re:CoM ends it first, the
   next KH1 frame takes over from Re:CoM's state. An air swing Re:CoM ends keeps KH1's arc into the
   fall.

### KH1 rows (`attack::kh1_row_bytes`). Checked (exe + files + tests)

Each KH1 row is a copy of the Re:CoM row (rows 1–7, `0x1407C6C80`, checked at start against the
Steam bytes) that plays the same KH1 motion, with:

| byte | KH1 row value |
|---|---|
| `+1` / `+3` | KH1 combo window start / end (0 / 0 = none) |
| `+4` / `+5` | KH1 hit window, one frame earlier (below) |
| `+6`..`+0xA` | 0: no homing, no stop distance, no speed, no card column |
| `+0xB` | Re:CoM kind: KH1 kind 3 (flinch) → 1, 5 (launch) → 2 |
| `+0xC` / `+0xD` | 0: no push, no Re:CoM hop (KH1's arc runs in the handler) |
| `+0xF` | `0x80` only ("KH1 row"; no Re:CoM chain, early cancel or push) |

| KH1 row | motion | Re:CoM id (clip) | KH1 hit | row `+4/+5` | combo window | kind |
|---|---|---|---|---|---|---|
| 13 | 200 | 1 (`0x30`) | 14–24 | 13–23 | 22–40 | 1 |
| 11 | 201 | 2 (`0x31`) | 18–32 | 17–31 | 26–42 | 1 |
| 14 | 202 | 3 (`0x32`) | 18–30 | 17–29 | 26–54 | 1 |
| 8 | 203 | 4 (`0x33`) | 26–38 | 25–37 | none | 2 |
| 17 | 205 | 5 (`0x34`) | 10–20 | 9–19 | 18–32 | 1 |
| 18 | 204 | 6 (`0x35`) | 6–20 | 5–19 | 18–34 | 1 |
| 16 | 206 | 7 (`0x36`) | 12–38 | 11–37 | none | 2 |
| 1 jump-up | 205 | 5 | 10–20 | 9–19 | 18–32 | 1 |
| 2 hop | 204 | 6 | 6–20 | 5–19 | 18–34 | 1 |

- **The hit window one frame earlier.** Re:CoM tests a row's window in its exe against the clock
  of the last motion update. The weapon's collision runs later in the frame on the new pose
  (`0x140225840` calls the test itself). KH1 tests its flags after this frame's update. So KH1's
  `start ≤ clock(N) < end` is Re:CoM's `start − 1 ≤ clock(N − 1) < end − 1`. Assumption: the
  weapon's task runs after Sora's motion update in the same frame. The trace's `hit_active` column
  (the weapon's `+0x23C`) shows it in game.
- The combo-window test is our own frame's (KH1's flags on Re:CoM's clock after the last update),
  as KH1's think reads them. The row's `+1` / `+3` record the same window for Re:CoM-side readers.
  Nothing of Re:CoM reads them.
- Not the design's gate patch at `0x1401EAF95`. Re:CoM's chain (`+0x650`) starts the next swing on
  the frame *after* the press, KH1's on the press frame. Our frame starts it through Re:CoM's own
  init on the press frame instead.
- Hop row 2 needs KH1's obstacle probe `0x1402C6E70`. Not built yet: the probe returns "no wall"
  (Assumption, as kh1-core's default `World`).

### Targets. Assumption

KH1's picker gets Re:CoM's attack target `player+0x84C` as a body part. Its point is Re:CoM's own
aim point (vtable `+0x50`, as Re:CoM's row pick reads it), with radius 0 (Re:CoM measures centre
to centre). It counts as KH1's lock-on when Re:CoM's lock-on is on (M4's lock-on fields). Without
a lock, KH1 scans every enemy part in its cones; here the scan sees only that target. M12
(lock-on) owns the rest.

## The Keyblade volume

### Re:CoM's. Checked (decomp)

- The weapon's `CCollAttack` (`weapon+0x240`, shape `+0x258`) is a type 7 thick triangle.
- `0x140225840` moves it each frame while `weapon+0x23C ≠ 0` and calls its test (vtable `+0x30`
  = `0x14016F110` → pair dispatch `0x14016EAD0(coll, body list)`). Bodies of type 2 / 4 / 5 are
  tested with `0x140177480` / `0x140176BE0`, then `coll` vtable `+0x38` = `0x14016F1C0` applies the
  hit (serial check: one hit per body per swing).
- Sphere primitive `0x1401777E0(ctx, body, attack)` and column primitive `0x140176FA0(ctx, body,
  attack, 0)` read the attack sphere's `+0x10` / `+0x20` (swept centres), `+0x70` radius, `+0x74`
  height. With `+0x10 = +0x20`, `0x1401782A0` is the plain test `|body − centre| < r₁ + r₂`
  (strict), the same as KH1's sphere test.

### KH1's on it (`attack::blade_points`, `blade_shapes`; our pair-dispatch wrapper)

- For the main Sora's weapon on a KH1 row, our wrapper tests KH1's spheres instead, with those two
  primitives, on the same body list and with the same skip rules. A hit runs Re:CoM's own apply on
  that body (its serial, back bonus, weak parts, effects).
- **Blade.** KH1's line runs from bone 282 (t = 0) to bone 283 (t = 1). Re:CoM's Keyblade bones
  `wep0` (2) and `k9` (3) are those bones × 0.01 (Checked (files) ≤ 0.012 mm,
  `kh1-recom-assets.md`); their world origins are read with `0x1402ABB20(weapon, i) + 0xD0`.
  Kingdom Key: A at t = −2.96 (0.625 m along the blade), B at t = 0. Radius 0 + 1.0 × 40 = 0.40 m.
  So the blade chain is 2 spheres, plus the swept chain from last frame's A.
- The first tested frame sweeps from Sora's centre. Assumption: `player+0x4B0`, the point
  `onAttack` copies as the attack origin.
- Not a new type id `0xD`: the shape stays type 7. Re:CoM's hilt and radius setters
  (`0x14016F760`, `0x140172690`) only write type 7, and the triangle's own tips feed Re:CoM's hit
  effects. The KH1 test is chosen by the row's `0x80` flag in the dispatch.
- Targets stay Re:CoM's own hurt shapes (M10/M11).

## Damage and reactions

- **KH1 enemy stats (0.7.0, decisions 12 and 21).** Research notes: `kh1-enemy-stats.md` (kept
  with the decomp tools). Checked (decomp + files) unless tagged.
  - KH1 fills an enemy's stats block in `0x1402A58E0` from a 0x6C-byte record its AI script
    hands to AI syscall 45 (`0x1402BBDF0`, entry 45 of `0x140529180`): r0 HP, r1 / r2 Strength /
    Defence index into btltbl `0xC8A8` (21 rows), r3 / r4 damage floor / cap per hit
    (`+0xC0` / `+0xC2`, read by `0x1402BFA80`), r5 level, r6 knockback factor, r7 EXP, r9 the
    actor's own damage scale, r10..r25 resistances %.
  - An ordinary Heartless's AI is part 5 of its own `.mdls` (`0x140290D60`, binder
    `0x140287E40`); bosses and a few scripted fights use their room's script. The record is
    picked at init by the LV2 story bit (syscall 198: save `0xDC4` bit 0) and, for the Shadow,
    by world (syscall 105). The script offsets per world / LV were found by evaluating every
    script; room copies of the same scripts are byte-identical.
  - The asset step (`kh1-recom-assets` `stats`) reads each twin's `.mdls` from the player's
    KH1 packages and writes `kh1-stats.json`: Sora's KH1 Strength for levels 1..100 and, for 31
    Re:CoM classes, the twin's record for each Re:CoM world 1..13 (decision 21: the world's
    LV1 record; Twilight Town and Castle Oblivion, which KH1 lacks, get LV2). Pairings named
    only by internal names are tagged `assumed` (Barrel Spider, Sea Neon, Aquatank,
    Screwdiver, Wyvern, Creeper Plant, Neoshadow). Tornado Step isn't identified and keeps
    Re:CoM's numbers.
  - Runtime (`win/stats.rs`): the enemy's class is its vtable (31 Re:CoM `CEnemy*` vtables,
    RTTI-checked at start); the world is Re:CoM's `0x1402339C0(0)` (1 Traverse Town … 13 Castle
    Oblivion, the order of `0x140506AA0`; `0x14087B844` is the floor, not the world). Riku's
    story (`0x1401AAAE0`) keeps Re:CoM's numbers.
  - **HP:** a wrapper on Re:CoM's enemy stats init `0x14017FF50` (it sets `+0x274` from the
    enemy's per-floor row `+0x310`) sets the twin's KH1 HP, and the max-HP getter `0x14017F5A0`
    returns it for that enemy (both at KH1 only).
  - **Damage:** in the enemy hit handler, a KH1 hit's part value (`part+0x92`, read through
    `0x14016FDB0`) becomes KH1's damage (kh1-core's `0x1402BFA80`: curve[power − Defence],
    the row's bounds, the twin's floor / cap and resistance, a crit roll), with Sora's KH1
    Strength at his Re:CoM level (`*0x14087CBF8 + 0x1C`, the menu's level). The trace's hits
    column adds `:kh1 <damage>`.
- **Decision 13 (keep Re:CoM's extras).** Re:CoM's damage path is untouched. Once the KH1 numbers
  are on, they replace the AP, and the back bonus, weak parts and boss falloff still apply.
- **Reactions.** The KH1 row's kind: swings (KH1 kind 3) → Re:CoM 1 (flinch), finishers (kind 5)
  → Re:CoM 2 (blow-away when the enemy's `@ENP+0xC > 0`). Re:CoM's enemies use their own flinch and
  blow-away states. KH1's launch speed (20 + 15 × weight, hop × 1.35) can't be passed to them:
  Re:CoM's blow-away speeds are in each enemy's state code (not read). It comes with the KH1
  enemies (M10/M11).
- **Card priority (decision 14).** Re:CoM's enemy hit handler `0x14017F7E0` returns before damage
  while `enemy+0x260` bit 0 (card playing) is set. With bits `0x2000001` both set, it first ends the
  card (`0x14017E1D0`), then takes the hit.
  - For a KH1 swing's hit, our wrapper sets `0x2000000` for that call (and takes it off after),
    so Re:CoM's own path ends the card and the hit lands.
  - A KH1 hit is recognised by the body part our dispatch hit: the enemy's parts are
    `*(enemy+0xF8)+0x40`, `0x120` bytes each, count `+0x38`, the same objects the dispatch hands
    to the apply (Checked (decomp `0x140154EB0`, `0x14016F1C0`)). Other attackers keep Re:CoM's
    rule.
  - The trace's `hits` column logs each KH1 hit: the HP before and after, and `:card` when it
    broke into a card play.

## Sora's KH1 Strength (decision 18: keep the Rod, give up the Shield)

All Checked (decomp + files) unless tagged.
- **New game** (`0x1402852C0`): each party record (`save + 4 + 0x74 × i`) is filled from btltbl
  file `0x39D8`, 23 bytes per character: HP, MP, AP, Strength, Defence, 16 resistances, 2 more.
  Sora: HP 18, MP 3, AP 3, **Strength 4**, Defence 3, all resistances 100.
- **Dive to the Heart** (`dh01.ard` script; event syscall 481 = `0x1401B03E0` → `0x140284B70`):
  adds btltbl `0x128 + 4 × choice` = (Strength, Defence, MP, AP) deltas. The script computes the
  choice from the kept item (var 256) and the given-up one (var 257): kept 0 → 0 / 1, kept 1 →
  2 / 3, kept 2 → 4 / 5. Rows: (+1, −2, −1, 0), (+1, −1, −1, −2), (−1, +1, −1, 0), (0, +1, −1, −2),
  (−2, 0, 0, 0), (0, −2, 0, 0). Assumption: items 0 / 1 / 2 are Sword / Shield / Rod (the deltas
  fit: keeping the Rod is the only choice without MP −1, giving up the Rod costs AP 2). So the
  owner's path, keep Rod and give up Shield, is row 5: Strength +0, Defence −2.
- **Level-ups** (`0x1402B51B0`): the bonus for reaching level L is btltbl `0x3AC0[L − 2]`
  (Sora's table, from `0x140528CC0`; the same table for every Dive choice). The choice only picks
  the EXP curve (`0x140528028[save + 0x48D]`) and the ability list (`0x140528D10[save + 0x48C]`).
  Bonus 4 = Strength +2 (`0x1402B5660`; 1 HP +3, 2 MP +1, 3 AP +2, 5 Defence +2, 6 / 7 slots).
- **In battle** (`0x1402A5130`): stats `+0x4C` = save Strength + the weapon's `+0x30` (Kingdom Key
  3) + accessories (none here).
- So Sora's KH1 Strength at Re:CoM level L = 4 + 0 + 3 + 2 × (number of 4s in the first L − 1
  entries of `0x3AC0`): 7 at level 1, 9 at 2, 13 at 10, 17 at 20, 27 at 50, 51 at 99. The asset
  step reads these from the player's btltbl.

## Swings and a moving locked target (checked 2026-10-30)

- KH1 takes the aim once, at the Attack command: `0x1402A7510` → `0x1402A38E0` → `0x1402A3B00`
  (the row's face-target flag). The other callers of `0x1402A3B00` are the movement think
  `0x1402A7B30` (runs only while `0x140297870` says Sora is free: flags `& 4`, `+0x88 & 0x20`,
  `+0x444 & 1` all clear), the party AI and summons. A swing keeps the yaw taken at the press, as
  kh1-core does. Checked (decomp). Assumption: a swing sets flag 4 (kh1-core's model).

## Guard, guard success and Counterattack (0.7.1; decision 20)

Research notes `kh1-guard-counter.md` (kept with the decomp tools). Checked (decomp + files)
unless tagged.

- **KH1.** Guard (row 0x3D, record `0x4DA8`) is a pure-guard attack object on the Keyblade
  (clash type 2, ±100°: cosine −0.1736), live on motion 212's hit track (frames 10–36). When it
  stops an enemy attack, `0x1402B4BC0` always gives Sora a recoil: kind 6 (`0x14029D720`) unless
  the enemy out-prioritises the Keyblade by more than 40 (kind 7).
  - Kind 6: request 0x6E at once (the guard motion and its object end), the idle queued, flag
    0x80 (no damage, no reaction) whose 8-frame countdown starts at the next action request or
    the lock release, CB+0x41 − 1, CB+0x50 |= 0x10, a pushback only when the enemy's priority
    is higher. Motion 0x6E (entry 72): 60 frames, no travel, free 28–60, combo window 28–60.
  - Counterattack (ability 19, `0x142D5EC10 & 4`): the menu's first slot becomes command 0x63
    (`0x140294220`) while the request is 0x6E, CB+0x50 holds the clash bit and a target exists;
    the accept (`0x140293480`) takes it from motion time 12. Command step `0x1402A7510` type 99:
    CB+0x41 + 1, row 0x3E (record `0x4E18`, motion 213: 86 frames, hit 8–24, combo window 18–56,
    travel 267.05), aimed at the target, else the stick, else the facing.
- **In Re:CoM (native).** Re:CoM's own body guard does the blocking: the hit apply
  `0x14016F1C0` returns 2 (guarded: no damage, no hit counter) when `body+0x9C & 1`, the hit's
  element bit is in `body+0x8C` (all by default) and the attacker is within `body+0xB0` of the
  body's facing, and it sets `body+0x9C |= 0x20`. Sora's body is `player+0x690`.
  - The KH1 handler sets `+0x72C |= 1` and `+0x740 = 1.7453` (100°) while the guard motion's
    hit track is live, and clears them (angle back to π) after.
  - The damage-check wrapper (`0x1401E6DE0`) catches `+0x72C & 0x20` before Re:CoM clears it
    (`0x1401F4000` / `0x1401FABC0` each frame) and hands it to the next KH1 frame, which runs the
    kind-6 success: KH1's request 0x6E (Re:CoM's bake `p001_081`, row 0x24), the lock, the combo
    counter −1, the clash bit, and flag 0x80 as Re:CoM's `+0x418 & 1` (the damage check skips
    hits) until 8 frames after the next action or the lock release.
  - The Counterattack is KH1's row 0x3E in our attack table, playing the asset step's KH1-only
    clip (`p001_226`) through a second appended action row. ini `counterattack = on | off`.
- **Differences from KH1, by decision 20:** Re:CoM attacks have no KH1 priority, so every
  attack Re:CoM's guard stops gives kind 6 with no pushback. Re:CoM's guard covers Sora's body,
  not the Keyblade's capsule, and measures the angle to the hit point. Re:CoM enemies get no
  reaction yet (their recoil and turned-back shots are the next version, behind a Config
  toggle).
- Assumption: leaving the recoil early in its free frames counts as the lock release for the
  8-frame countdown. Not traced in KH1.
- Note to re-check: Stage 9 found the roll's `0x140297F30(Sora, 12)` leaves damage on (no-flinch
  only); this research reads flag 0x80 as no damage too (`0x1402BEEE0`). The roll is unchanged.

## MP charge (skipped, noted)

- KH1 charges `record +0x2C` × (1 + (enemy level − Sora's level) × 0.2): 3 / 6 / 24 per hit, at
  most 250 %, at least 1. 30 units = 1 MP (`0x1402BF330`, `0x1402A4AE0`). An enemy of level 0
  charges nothing.
- It needs the KH1 twin's level (the missing records above) and Sora's KH1 level. Not stored.
  M7 builds the gauge.

## The M4 follow-ups (decision 17)

- **High Jump** plays Re:CoM's own higher jump: rows 0x14 / 0x15 (`p001_028` / `p001_029`). Only
  Guard is appended to Sora's action table now (row 0x47). Checked (tests).
- **Root motion one update late.** Our wrapper on `0x14024DCA0(ctrl, out, out yaw)` (the entity
  task's root step, `0x1401A59A0`) delays Sora's root travel by one update, only in our KH1 actions
  (avoid 6: roll / Guard; attack `0xA`). Re:CoM's own actions are left alone.
  - **`out` is an absolute position, not a travel** (fixed after 0.6.0's in-game bug):
    - `ctrl+0xA0` = the model's position `ctrl+0xF0` + this update's rotated root travel
      (`0x14024C960`, the anchor copied in `0x14024CCC0`);
    - the entity task sets the body to `P + mask∘(out − P)` (`0x14023E6E0` a − b, `0x14023E630`
      a ∘ b, `0x14023E0D0` a + b, mask at actor `+0x140`). Checked (decomp).
  - 0.6.0 stored and returned it as a travel, so on a swing's or roll's first update (nothing
    pending) it returned (0, 0, 0) and Sora was moved to near the room's origin. The owner's traces
    show it: idle at (−416.35, −750.35, 1363.55), then (10.6, −597.0, −34.7) on the Attack press,
    then walk-off and a fall.
  - Now: travel = `out − anchor` is delayed, and the hook returns `anchor + last travel`. That is
    Re:CoM's own position with only the travel term changed, never more than 0.5 m from Re:CoM's
    own result (a safety net: otherwise Re:CoM's stands).
  - The hook passes through when the state exes aren't counting frames (trace hooks missing).
  - Test `swing_and_roll_away_from_the_origin_keep_soras_position` uses a non-origin start, a
    model offset and Re:CoM's absolute root step: it fails with 0.6.0's logic (the same jump to
    near the origin) and passes now, for the swing and the roll.
  - `debug_attack = on` in the ini logs the anchor / root / output for the first 200 lines.
  - KH1 applies an update's travel in the next physics step, also on the frame a new motion
    starts, so the travel carries across a chain (`RootDelay`, keyed by Sora's frame count from
    the state exes).
  - The roll and the swings now match kh1-core exactly. Checked (tests).
  - Assumption: Re:CoM's clip-start update leaves `ctrl+0xA0` at zero.
- **Roll protection.** KH1's roll sets only the no-flinch timer: `0x1402C9E60` → `0x140297F30(12.0)`
  sets flag `0x80`. Its untouchable frames don't exist:
  - the hit pass needs `flags & 0x8048 == 0x8000` (`0x140292070`);
  - flag `0x40` is set and cleared only by actor-command ops `0x2B` / `0x2A` (`0x14028E6E0`'s queue);
  - those ops are queued only by event-script opcodes (`0x1401C8820`, `0x1401C88C0`,
    `0x1401C8B00`, `0x1401C8B60`, `0x1401C8C00`, `0x1401C8C60`), always next to `0x24` / `0x23`;
  - the roll motion's tracks hold only types 0 (free from 30), 4 and 6, and the mset's third
    section is a 48-byte stub.
  - Checked (decomp + files). This corrects KH2-Combat-in-KH1's `kh1-intangible.md`, which calls
    them "motion events".
  - So at KH1, Sora takes damage during the roll, with no hit reaction for 12 frames.
  - Re:CoM's untouchable bit `+0x418 & 1` is no longer set. Our wrapper on the damage check
    `0x1401E6DE0` sets `+0x420 & 2` for that call only. The check reads `+0x420` once at its start,
    and after applying the damage takes its own no-reaction exit (`(+0x420 & 0x8010002) ≠ 0` →
    return 0). Death still goes through.
  - Checked (decomp): the bit's only other readers are the room-card regen ticks (`0x1401600D0`,
    `0x1401601C0`), which don't run inside the call.

## Hooks and patches (`sites::ATTACK_HOOKS`, `ATTACK_CALLEES`, `ATTACK_DATA`)

All bytes were compared with the Steam exe (54 sites, script check). Hooks are all or nothing,
after the data patches, which are also all or nothing.

| address | what | first bytes (moved) |
|---|---|---|
| `0x1401F8150` | request bits | `48 89 5C 24 10 57 48 83 EC 20` |
| `0x1401E8FF0` | use-card button | `40 53 48 83 EC 20` |
| `0x1401F0D90` | initAttack | `48 89 5C 24 08 48 89 6C 24 10` |
| `0x1401F0F70` | field swing init | `48 89 5C 24 08 48 89 74 24 10` |
| `0x1401EAEB0` | field swing exe | `48 89 5C 24 08 48 89 74 24 10` |
| `0x14016EAD0` | attack pair dispatch | `48 89 5C 24 18 48 89 6C 24 20` |
| `0x14017F7E0` | enemy hit handler | `48 89 5C 24 20 55 56 41 56` |
| `0x1401E6DE0` | player damage check | `40 55 57 48 8D 6C 24 B1` |
| `0x14024DCA0` | root motion step | `0F 28 81 A0 00 00 00` |

Data (checked old values):

- `0x1405A9A60` (`CPlayerSora` vtable `+0x128`): `0x1401F7520` → our row pick. Compared after
  relocation.
- `0x1401FA074` (Battle jump table, id `0xA`): RVA `0x1F9CC9` → `0x1FA6C1`.
- `0x1401FA6C1`: 13 × `CC` → the stub.
- Re:CoM's rows 1–7 must equal `attack::RECOM_ROWS_1_7`.

Called: `0x1401F6660` (end attack), `0x140224FB0` (weapon attack end), `0x1402ABB20` (bone),
`0x1401777E0`, `0x140176FA0`, `0x1401F7180`, `0x1402AD350`; `0x1401E80C0` (springs).

Per frame, our code runs only as cheap tests outside a KH1 swing: the request bits, the damage
check and the root step (a pointer compare), and the pair dispatch only while a weapon volume is
live.

## Trace (F9)

New cells in the existing schema:

- `row` (KH1 row), `combo_counter`, `combo_max` / `air_combo_max`, `combo_master`;
- `hit_active` (Re:CoM's weapon volume live), `swing_hits` (Re:CoM's `+0x644`);
- `hits` (each KH1 hit: enemy, damage, HP after, `:card`);
- `events`: `AttackStarted { row, request, counter, finisher }`, `AttackRefused(…)`, and Re:CoM's
  `attack_id` while in action `0xA`.

## Debug log

`debug_attack = on` in `kh1-recom.ini` writes one line per decision to `kh1-recom.log` (the first
200 lines): `dispatch` (Re:CoM's dispatch reached the field-swing init), `start` (row, Re:CoM id,
counter, Sora's position, the target, lock-on), `exe` (action, motion frame, body velocity,
contact, hit) and `root` (anchor, Re:CoM's root position, our output).

## In game, 0.6.1 (owner's traces 7 and 8, 2026-10-30)

- The swing / guard / roll position jump is gone: no frame-to-frame jump over 20 units in
  either trace. Checked (trace).
- "Sora is missing a lot of his swings": 11 presses gave no swing. Each was refused by one of
  KH1's own rules (`kh1-sora-data.md`):
  - 8 × `NotAccepted`: the press came before the combo window opened (13–19 frames into a
    swing whose window is 22–40 / 26–42 / 26–54) or during the finisher, which has no window.
    KH1 keeps no press for later: `0x140293480` drops a command while Sora can't act. Checked
    (decomp). The owner pressed about every 15 frames, so about half the presses fell early.
  - 3 × `LastSwingMissed`: pressed inside the window after a swing that hit nothing. KH1 ends
    the combo there unless Combo Master is equipped. Checked (decomp).
  - The windows match KH1's files frame for frame (row 13: hit 14–23, window 22–39 in the
    trace, against 14–24 / 22–40). Checked (trace + files).
- Why the swings missed, from the trace: the locked enemy walked out of range during the swing
  (frame 112, about 190–230 units away). In frame 166 it circled Sora at about 80 units while
  Sora's facing stayed on the yaw taken at the press, 50–70° away from it. Whether KH1 keeps
  turning a ground swing toward a moving locked target after the press is not checked yet
  (the row exec's face-target flag, `0x1402A38E0`). Assumption: it doesn't.

## Assumptions to check in game

- The weapon's collision runs after Sora's motion update in the frame (the hit-window lead).
- `player+0x4B0` is Sora's centre (the first sweep).
- Re:CoM's target aim point stands for KH1's body-part centre, radius 0.
- The KH1 swing's Re:CoM clips have KH1's root travel (they are bakes of KH1's motions).
- An enemy's hit-handler parts are the dispatch's body nodes.
- `ctrl+0xA0` is zero on a clip's first update.
