# KH1 movement inside Re:CoM's player actions (roadmap M4)

Steam addresses (Re:CoM image base `0x140000000`). Tags: Checked (decomp / exe / files / tests)
or Assumption. Code: `crates/kh1-recom` (`movement.rs`, `rows.rs`, `units.rs`, `trace.rs`,
`win/movement.rs`, `win/trace.rs`). The oracle is `crates/kh1-core` (`controller.rs`).

## Status

- Built: KH1's walk / run speed snap, instant direction and facing turn, jump (crouch, double-sine
  rise, release cut, fall cap), High Jump, landing lock and the land-jump, walk-off, air control
  and drag, Glide and Superglide, Dodge Roll, Guard's stance and timing. Field and battle. Re:CoM's
  camera stays (M12). Guard's repel and counter are M5.
- Checked (tests): on a fake Re:CoM that does what Re:CoM does around a handler (the motion clock
  that does not move on a clip's first update, root motion in the motion update, then the mover),
  the handlers match kh1-core frame for frame (position ≤ 0.001 units, facing ≤ 1e-5 rad, state,
  action lock) for walk, run, turns, jump, High Jump, the jump cut, the landing lock, the
  land-jump, walking off a ledge, Glide, Superglide, Guard and a varying frame step (1–3). The
  roll's root motion is the one exception (below).
- Not checked in game. The tester thread is `testers/recom-movement/thread.md`.

## Units. Checked

- KH1: centimetres, y down. Re:CoM: metres, y up. A KH1 point (x, y, z) is Re:CoM (x, −y, −z) /
  100: the skeletons match bone for bone at × 0.01 with KH1's matrices `W · diag(1, −1, −1)`
  (`motion-conversion.md`, `kh1-recom-assets.md`). Checked (files).
- The speeds confirm it. Re:CoM's jump (`0x1401EDA60`) rises at `28 × 0.6 × 0.008166667` =
  0.1372 m/frame (`0x1407C5B60+0x1C` = 28.0, `0x1407C5B38` = 0.6) against gravity
  `0.6 × −0.008166667` = −0.0049 m/frame² (`0x1401FAB40` → `0x14087B398`), so it tops out at
  1.92 m after 28 frames. KH1's jump is 190 units in 28 frames. Checked (exe + decomp).
- Yaw: both games face (sin yaw, 0, cos yaw). KH1 yaw ψ is Re:CoM yaw π − ψ. Checked (decomp:
  Re:CoM `atan2(x, z)` in `0x1401E8C00` / `0x1401F2C20`; the forward `(0, 0, 1)` at `0x1407C5870`
  turned by the body yaw in `0x1401D5670`).
- Time: both count 60 Hz frames (`recom-timing.md`). Re:CoM's movement step `player+0x464` =
  `dt × 0.02`, set by the Field and Battle exes; the step capped at 4.0, KH1's at 5.0. Checked
  (decomp).
- Velocity: the mover moves `pos += v × step` with v per 60 Hz frame, so KH1's displacement d over
  step s is the velocity d / s. Checked (decomp).

Re:CoM's own numbers (`0x1407C5B60`, Sora's parameter block; Checked (exe)): walk 1.3 m/s, run
6.2 m/s × stick, jump speed 28.0 and 32.5 (its higher jump, `+0x41C & 0x200000`), crouch 100
units (2 frames), roll invulnerable for 600 units (12 frames), roll cancel from 1400 units.
KH1's (`kh1-sora-data.md`): walk 2 units = 1.2 m/s, run 8 = 4.8 m/s, jump 190 / 290.

## Re:CoM's movement code. Checked (decomp) unless marked

### Per frame

1. The entity task `0x1401A59A0`: the state machine (`player+0x58`, `0x140242F50`) runs the state
   exe; then the motion advance `0x14024E470` and, with the root flag (`ctrl+0x52 & 8`), the root
   motion, which sets the body position directly; then the mover `0x1401D8190`.
2. The Field exe `0x1401FA0F0` / Battle exe `0x1401F9840`: `+0x460` = dt, `+0x464` = dt × 0.02,
   `+0x538` = 0 (the ground test's cache), `+0x410` += dt (time in action), pad update
   `0x1401FF150`, (battle: use-card button `0x1401E8FF0`), request bits `0x1401F8150`, damage /
   escape interrupts, lock-on `0x1401E8DA0`, then the switch on the action `+0x40C`.
3. Pad wrapper (`player+0x230`) `0x1401FF150`: stick ints from the device (`[0x140AC5D40][port]
   +0x74 / +0x78`, ±4096 from the raw byte with a dead zone of 32 and scale 94, `0x14029C560`),
   `+4` = `atan2(x, y)`, `+8` = magnitude, `+0x34` = camera yaw + angle − π.
4. Request bits `0x1401F8150` into `+0x424` (rebuilt each frame, mask `0xFFF7F400`): 1 jump (the
   cancel button pressed, `+0x41C & 0x10`), 2 avoid (Square `0x80` pressed, `+0x41C & 8`), 4
   attack (field: the confirm button) and the card bits. It copies `+0x384` = stick magnitude,
   `+0x38C` = the stick's world yaw, `+0x4D4` = body yaw.

### The movement actions

| action | init | exe | notes |
|---|---|---|---|
| 2 / 3 / 4 | inline | `0x1401EEDA0` | command dispatch (cards, attack, reload, interactions), then `+0x424 & 2` → avoid init, `& 1` → jump init, stick class `0x1401E9630` → action 2/3/4 and `+0x4DC`, move step `0x1401EF3C0` (facing eased 0.4 × step toward the stick yaw `0x1401D4670`, row pick `0x1401F9380`, velocity via `0x1401D5630`), then the ground test → ledge climb `0x1401E8C00` / hang `0x1401E96C0`, or the fall `0x1401F1F20` |
| 6 avoid | `0x1401F10C0` | `CPlayerSora` vtable `0x1C8` = `0x140217310` | row 0x16 with root motion; `+0x418` bit 0 for 600 units |
| 7 jump | `0x1401F29F0(player, falling)` | `0x1401EDA60` | `+0x604`: 1 crouch, 2 rise, 3 fall, 6 land (`0x1401EE360`); `+0x5D4` vertical speed; `+0x5E0` take-off point |
| 8 glide | `CPlayerSora` `0x1402202C0` | `0x140217B50` | rows 0x1B–0x1E |
| 9 | `0x1401F26A0` | `0x1401ECF40` | ledge hang (row 0xF), not a fall |

- `0x1401F0920` classifies the action for other code by `+0x604` (1/2/5 rising, 3/4 falling, 6
  landing).
- Ground test `0x1401F3C00`: airborne when the body has no contact (`body+0x120 & 5`) and is above
  `+0x5F8`; cached per frame in `+0x538 / +0x53C`.
- Fall from other actions: `0x1401F1F20` = jump init(player, 1), phase 3, row 6 at blend 800.
- `+0x418` bit 0 makes the damage check `0x1401E6DE0` ignore hits (Re:CoM's roll protection).

### The mover (`CMovement`, `player+0xF0`, ctor `0x1401D4200`)

- `+0x30` position, `+0x50` rotation (yaw `+0x54`), `+0x80` velocity per frame, `+0x90`
  acceleration, `+0x118` control flags (bit 2 adds gravity `+0x128 × +0x124 × +0x140`, set only
  by `0x1401D67B0`), `+0x120` contact (bit 0 floor, bit 2), `+0x144` = −0.5 lowest vertical
  speed, `+0x158` collision object, `+0x160` last grounded position.
- `0x1401D8190` → `0x1401D4E00`: on the ground (vertical speed ≤ 0 and contact) the horizontal
  velocity is pushed down the slope (`−|v| / cos 50°`, or −0.05 standing) and integrated
  (`0x1401D4CF0`: `pos += v·dt + a·dt²/2`); in the air `0x1401D7520`; then floor / wall collision
  (`0x1401D5CE0`, segment tests `0x140170330`: ends at `obj[3]+0x10 / +0x20`, hit `+0x30`, normal
  `+0x40`, floors have normal y ≥ cos 50°). Below y = −75 m the body returns to its last ground.
- No player function calls `0x1401D67B0`, and Re:CoM's jump writes the vertical speed itself, so
  the body's own gravity is off for Sora. Assumption, well supported.

### Motions

- `0x1401F7180(player, row, blend, root, 0, 0, start)`: Sora's action table (rows `{name, index}`,
  `0x1407C6300[char]`) → actor vtable `0x70` = `0x1401A3470`. Blend and start in Re:CoM units
  (−1 blend = the clip's own). `+0x474` / `+0x470` = this / last row.
- A clip started by a request does not advance on its first update (`0x14024E9F0` sets
  `ctrl+0x52 & 4`, `0x14024E470` skips the step), like KH1's. Duration = frames × 3000 / fps, one
  frame less for play-once clips. End flags `ctrl+0x50 & 3` (`0x1401A57D0`).
- Blending advances the clip's clock (KH1 holds the new motion at its start frame).
- Root motion is applied in the same update as the clock (`0x1401A59A0` → `0x14024DCA0`), before
  the mover.

## What was built

### Hooks (`sites::MOVE_HOOKS`, each checks its exact first bytes, all or nothing)

| address | function | first bytes (moved) |
|---|---|---|
| `0x1401EEDA0` | move exe (2/3/4) | `40 53 48 83 EC 50 8B 81 24 04 00 00 48 8B D9` |
| `0x1401E9630` | stick class | `F3 0F 10 89 84 03 00 00` |
| `0x1401EF3C0` | move step | `40 53 48 83 EC 50 F3 0F 10 99 64 04 00 00 48 8B D9` |
| `0x1401F29F0` | jump init | `48 89 5C 24 10 48 89 74 24 18` |
| `0x1401F10C0` | avoid init | `40 53 48 83 EC 60` |
| `0x1401EDA60` | jump exe (7) | `4C 8B DC 53 48 81 EC D0 00 00 00` |
| `0x140217310` | `CPlayerSora` avoid exe (6) | `40 53 48 83 EC 30 48 8B 81 F0 00 00 00 48 8B D9` |
| `0x140217B50` | `CPlayerSora` glide exe (8) | `40 53 48 81 EC 80 00 00 00 83 A1 28 04 00 00 F7` |
| `0x1401FA0F0` | Field exe (trace only) | `40 53 48 83 EC 40 33 D2 48 8B D9` |
| `0x1401F9840` | Battle exe (trace only) | `48 89 7C 24 20 55 48 8D 6C 24 A0` |

Called (byte-checked as well, `sites::MOVE_CALLEES`): `0x1401F7180`, `0x1401F3C00`,
`0x140170330`, `0x1402AD350`, `0x1401FF4D0`, `0x14029C1D0`, `0x1401F0F70`, `0x1401F0D90`,
`0x1401E6C40`. All 39 sites' bytes were compared with the Steam exe. Checked (exe).

Every hook passes through to Re:CoM unless `combat = kh1` and the player is the main player's
`CPlayerSora` (`0x14087B380`, vtable `0x1405A9938`). Riku keeps Re:CoM's handlers.

### How the handlers sit in Re:CoM's actions

| KH1 state | Re:CoM action | `+0x604` |
|---|---|---|
| 0, free (idle / walk / run, and the land / roll tail) | 2 / 3 / 4 by KH1's speed (0 / 2 / 8) | 0 |
| 0, locked by the land | 7 | 6 |
| 0, locked by Dodge Roll | 6, `+0x414` = 0 | 3 |
| 0, locked by Guard | 6, `+0x414` = 1 | 0 |
| 1 take-off | 7 | 1 |
| 2 air | 7 | 2 rising / 3 falling |
| 0x15 glide | 8 | 4 |

- Re:CoM's move exe still runs its own command dispatch and its ledge climb / hang checks. Our
  code runs in its two leaves (stick class: KH1's speed snap; move step: KH1's ground frame) and
  in the jump / avoid inits it calls. When Sora is in the air at the start of the move exe, KH1's
  walk-off runs instead (KH1's physics leaves the ground before any ground command).
- The jump exe keeps Re:CoM's air commands (attack card or field swing, magic card) until M5.
- The handler leaves KH1's displacement as the body velocity; Re:CoM's mover moves and collides
  it; the next frame reads the result (landing, walk-off) before its think, in KH1's order.
- KH1-only fields (air arc, jump cut, glide timer, target yaw, no-flinch timer, KH1's request
  queue) live in `movement::Kh1Sora`, one per player, as new fields of Re:CoM's player. The action
  fields `+0x40C / +0x408 / +0x410` tell whether Re:CoM ran something else in between (a card, a
  hit, an event); then the next handler takes over from Re:CoM's state.
- Re:CoM's facing (`body+0x54`, `+0x4D4`), speed (`+0x4DC`), vertical speed (`+0x5D4`) and
  take-off point (`+0x5E0`) are kept up to date for Re:CoM's other readers.

### Input

- KH1's stick: the bytes behind Re:CoM's ints (`units::byte_from_recom_axis`, the inverse of
  `0x14029C560`), then KH1's dead zone 60 / 127.5 and rescale 67.5 (`kh1_core::stick_from_raw`).
  Re:CoM gives 4096 for both 126 and 127 off centre, so bytes 254 / 1 read as 255 / 0 (tilt 1.0
  instead of 0.985). Checked (tests).
- Direction: Re:CoM's camera yaw from the pad wrapper (`+0x34 − +4 + π`), turned by KH1's stick.
  On the axes and diagonals it is exactly Re:CoM's own direction. Checked (tests).
- Jump / Square presses: Re:CoM's request bits 1 / 2 (so Re:CoM's own locks and abilities still
  gate them); jump held: `0x1401FF4D0`; Square held: `0x14029C1D0(0x80, 0)`.
- Lock-on (Guard's facing): `*(player+0x3F8)+0xE0` on and the target `player+0x84C`, in battle.
  Assumption: that pair is Re:CoM's lock-on.

### Motion rows (`rows.rs`; decision 11)

| KH1 request | Re:CoM row |
|---|---|
| 0 idle | 1 (field), 0x2A (battle stance; `0x1401E6AC0`'s rule) |
| 1 walk / 2 run | 2 / 3, or 0x12 / 0x13 (`+0x418 & 2` clear), or 0x2B / 0x2C (battle stance) |
| 4 jump, 5 top, 6 / 0xB fall, 7 / 0xC land | 4, 5, 6, 7 |
| 0x71–0x74 glide | 0x1B–0x1E |
| 0xDC Dodge Roll | 0x16, root motion |
| 9 / 0xA High Jump, its top | Re:CoM's own higher jump rows 0x14 / 0x15 (decision 17, 0.6.0) |
| 212 Guard | new row 0x47 (KH1 entry 110), else none |

- New rows: at start, Sora's table is copied with the KH1-only rows appended (names from
  `kh1-motions.json`, checked against the `PL0001.ANN` header), and `0x1407C6300[1]` / `[2]` are
  repointed to the copy (checked old values). Re:CoM resolves the names at the next player
  set-up. Data only. Checked (decomp `0x1401FA6E0`, `0x1401F7180`).
- A KH1 request with no row keeps KH1's own frame count for its timing (`MotionRec::own_clock`).
- Blends are KH1's (frames × 50). Walk / run start at 0 (Re:CoM's clips are its own, so KH1's
  phase-keeping start is not passed on).

### Abilities

`kh1-recom.ini` keys `high_jump`, `glide`, `superglide`, `dodge_roll`, `guard` (on / off;
defaults off, off, off, on, on) until abilities are in Re:CoM's progression (M9).

### Trace

F9 writes `kh1-recom-trace-<n>.csv` next to the game in kh1-trace's schema (the schema file is
compiled in from `crates/kh1-trace/src/schema.rs`). KH1 units. Inputs: KH1's stick, tilt, raw
bytes, the camera yaw in kh1-core's terms (`in_cam_yaw`, so `kh1-playground --replay` moves the
same way), buttons in KH1's bit meanings (jump 4, Square 0x10, confirm 0x4000000), the lock
target, KH1's ability bits. State: Re:CoM's position, facing and motion clock, and KH1's state,
request, lock, arc, velocities and timers from the handler; the `events` cell has Re:CoM's
action / sub / phase / row. With `combat = recom` it records Re:CoM's own movement the same way
(state from the action). Only the two state-exe hooks run per frame, and they only test a flag
while not recording.

## Springs and other gimmick launches (0.6.0, owner's 0.5.0 recording)

> "sora needs to be able to jump as high as he is supposed to on these springs"

**What Re:CoM does. Checked (decomp + exe).**

- A spring is a gimmick object of type `0x2002`. When Sora is near one, the spring calls
  `0x1401F5370(handle, level)`: `player+0x5FC` = its handle, `player+0x600` = its launch speed from
  `0x1405A0100[level]` = 39.0, 45.9, 52.2, 57.6, 62.7, 67.3, 71.6, 75.8 (Sora's own jump speed is
  28.0, `0x1407C5B60+0x1C`).
- Re:CoM's jump init `0x1401F29F0(player, 0)` first clears `player+0x418` bit `0x1000000`. Then, if
  `player+0x41C & 0x20000` and `0x1401E80C0(player, 0x2002) == 7` (the gimmick is a spring and the
  body's contact bit 4 is on), it sets that bit, plays row 0x14 (Re:CoM's higher jump) and takes the
  spring's speed as the jump speed (`+0x5D0 = +0x600`).
- Its jump exe `0x1401EDA60` then rises at that speed under Re:CoM's gravity, with no jump cut while
  the bit is set (row-0x14 branch), switches to row 0x15 at the top, falls and lands (phase 6,
  `0x1401EE360`).
- The bit is written only by those two lines of the jump init and read only by the jump exe.
- With a speed of 39–75.8 against 28, a spring reaches (39/28)² ≈ 1.9× to 7.3× Re:CoM's jump
  height. At 0.5.0 our jump init ran KH1's jump instead, so the spring gave a KH1-height hop.

**What we do (option: Re:CoM's own spring path runs).** KH1 has no springs, and Re:CoM's spring
path can run whole, so it does:

- KH1's jump start (`Kh1Sora::jump_start`, reached from every ground jump: the jump init hook, a
  jump in a KH1 frame, the land-jump) first asks the host `Host::spring_jump`. The host applies
  Re:CoM's own test (`movement::spring::launches`: the ability bit, then `0x1401E80C0(player,
  0x2002) == 7`). On a spring it calls Re:CoM's own jump init through the trampoline (no hook
  runs), and the KH1 frame stops writing (`FrameOut::recom_spring`).
- The jump exe hook passes every frame through to Re:CoM's own exe while Re:CoM owns the jump
  (`spring::recom_owns_jump`: action 7 and the bit set): Re:CoM's height, arc, no jump cut and its
  landing. Re:CoM then returns Sora to stand, and the next KH1 frame takes over from Re:CoM's state
  (KH1's rules again).
- Every jump we start ourselves clears the bit, as Re:CoM's jump init does (the jump init hook, and
  our handlers entering action 7), so a later KH1 jump or fall is never taken for a spring jump.
- Not chosen: feeding the spring's height into KH1's arc. Re:CoM's own path can run, so nothing
  of ours copies its numbers.
- Tests: `spring_decision_follows_recoms_jump_init`, `jumping_on_a_spring_hands_the_jump_to_recom`,
  `a_land_jump_on_a_spring_is_recoms_too`.
- During the spring's flight, Re:CoM's jump exe also takes Re:CoM's air commands: the Attack
  becomes the KH1 Attack (M5 hooks), and its glide init leads into our KH1 glide. Assumption:
  Glide from a spring jump was not tried in game.

**Other gimmick launches. Checked (decomp).** The move exe's gimmick button (`+0x424 & 0x200`,
`0x1401E80C0`) starts Re:CoM's own actions 0x11, 0x14, 0x15, 0x1E and footing 0x23
(`0x1401F3730`, `0x1401F2F40`, `0x1401F39F0`, `0x1401F1670`, `0x1401F3180` / `0x1401F2310`). None of
them is an action we hook, so they stay Re:CoM's whole. A jump pressed while on a footing goes
through the jump init (spring test included), and a fall they end in goes through `jump init(player,
1)` (KH1's fall from rest). Events and scripts move Sora in the Wait / Hide states, which we don't
hook. No other launch writes the spring speed: the player's `+0x600` is written only by `0x1401F5370`
(through the main-player pointer), called only by the spring gimmick (`0x1401BC460`, `0x1401BC780`,
`0x1401BC990`). The one other `+0x600` store in the decomp (`0x14011CB20`) is on a camp-menu object.

## Where Re:CoM can't match KH1 yet

1. **Root motion** (fixed in 0.6.0, decision 17, option (c)). KH1 applies a motion's root travel
   in the next frame's physics, Re:CoM in the motion update of the same frame. A wrapper on
   Re:CoM's root step `0x14024DCA0` now hands Sora's controller the last update's travel in our
   KH1 actions (roll / Guard, attack), so the roll matches KH1 frame for frame. Checked (tests).
   Details in `recom-kh1-attack.md`.
2. **A queued motion starts one frame later in Re:CoM** (the idle after a land, roll or Guard; the
   fall after the jump top). KH1 starts it in the same update the last one ends; Re:CoM gets the
   request in the next frame's handler, with the start frame moved on by a step so its clock
   catches up. The lock is released on the same frame as KH1. Only the trace's `request` column
   and the look differ.
3. **Re:CoM's clock runs through blends** (KH1 holds the new motion at frame 0). Only the look
   and `motion_frame` of blended motions (idle, walk, run, fall, glide) differ; none of them has a
   window.
4. **Roll protection** (fixed in 0.6.0). KH1's roll has 12 no-flinch frames and **no**
   untouchable frames: the ops `0x2B` / `0x2A` come only from event scripts. Checked (decomp +
   files). At KH1, Sora now takes damage during the roll, with no reaction for those 12 frames.
   This uses Re:CoM's own no-reaction exit in its damage check, and no longer Re:CoM's untouchable
   bit `+0x418 & 1`. Details in `recom-kh1-attack.md`.
5. **Height above the floor** (Glide needs more than 50; Superglide slows below 70) is probed with
   Re:CoM's collision segment test only when those checks run. Assumption: the segment test
   returns the first hit.

## Assumptions to check in game

- `OnInit` runs before the first player set-up (the table extension needs it).
- The pad device's ints come from the bytes as `0x14029C560` computes them (other input devices).
- Re:CoM's mover leaves horizontal travel on flat floors as integrated (collision does not trim
  it), and the body's own gravity and acceleration are off for Sora.
- Re:CoM's lock-on fields above.
- Slopes and steps: Re:CoM's own slope handling decides (KH1's mover on slopes not compared).

## Open questions for the owner

- High Jump's animation: decided (17), Re:CoM's rows 0x14 / 0x15 (0.6.0).
- The root-motion frame: decided (17), fixed in 0.6.0.
- Re:CoM's ledge climb and hang (`0x1401E8C00`, `0x1401E96C0`) still work at KH1 setting (KH1's
  own ledge grabs are not built). Keep them?
- From 0.6.0 (M5) the Attack is KH1's (`recom-kh1-attack.md`); cards are no longer played at KH1
  setting (the deck itself is M6).
