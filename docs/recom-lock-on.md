# Lock-on on R1 at Combat = KH1 (0.133.0), with KH1's lock rules (0.137.0)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Shop text, lock-on and camera), verbatim:
"Let's have the lock on button be mapped to R1 if combat is set to KH1. If on mouse and keyboard it
stays the same." Code: `crates/kh1-recom/src/lock_on.rs` (pure), `src/win/lock_on.rs`; sites
`sites::LOCK_ON_HOOK`, `LOCK_ON_CHECKED` (test `sites::tests::lock_on_sites`).

## Re:CoM's lock-on (Checked (decomp + disasm + exe))

| what | where |
|---|---|
| The lock-on, once a frame from the player's Field and Battle exes. Off unless the target cursor (`player+0x3F8`) allows it (`+0xF0`) and `player+0x418 & 0x10000` is clear. Not locked: the button **pressed** locks (sound 7, `0x140208EC0`). Locked: pressed again sets `+0x424 & 0x1000`, then its **release** unlocks (sound 8); **held** with a right-stick flick (`0x1401FF4A0`: the stick's direction changed) switches to the next target that way (`0x1401CD4B0`). The handle goes to `player+0x850` (the camera's mode 3 ↔ 4 reads it) and the target to `+0x84C`. | `0x1401E8DA0` (callers `0x1401F994B`, `0x1401FA1A2`) |
| Its button is the player's pad object (`player+0x230`) field **`+0x44`**, read only by the lock-on's three readers: pressed `0x1401FF740`, released `0x1401FF600`, held `0x1401FF4F0` (each `mov ecx,[rcx+0x44]` then the pad test; each called only from the lock-on). | `0x1401FF4F0`, `0x1401FF600`, `0x1401FF740` |
| The pad object's update writes its action buttons every frame: `+0x38` = 4, `+0x3C` = 8, `+0x40` = 1, **`+0x44` = 2**. | `0x1401FF150` (`0x1401FF19D`) |
| Button order (Re:CoM's pad word): PS2 order with the two pad bytes swapped by the pad read (`0x14029C690`): L2 1, **R2 2**, L1 4, **R1 8**, Triangle 0x10, Circle 0x20, Cross 0x40, Square 0x80, Select 0x100, L3 0x200, R3 0x400, Start 0x800, Up 0x1000, Right 0x2000, Down 0x4000, Left 0x8000. Consistent with every reader we use (L1 = 4 for the shortcut list, Triangle 0x10 for stock, L1 + R1 = 4 + 8 for a sleight `0x1401FF840`). So **Re:CoM's lock-on is R2** on a pad (Assumption: R2 / R1 here are the buttons Re:CoM's own controls call R2 / R1 with the default binding). | `0x14029C690`, `0x1401FF840` |
| What R1 does in Re:CoM: card scroll right (`+0x3C` with D-pad Down, `0x1401FF530` / `0x1401FF780` in the request bits `0x1401F8150`), sleight with L1. At Combat = KH1 in battle the request bits run Re:CoM's field branch (no card scroll, reload, stock or sleight input: `win::attack::request_bits`) and the use-card button is skipped, so **R1 does nothing at KH1** before this version; ours: none (the command menu counts it only as "another button", the L1 list is L1 held). | `0x1401F8150`, `win::attack` |
| Pad and keyboard: the PC port reads XInput (`XINPUT1_4` ordinals 2 / 3), DirectInput 8 and HID pads and the keyboard into the same PS2 pad word through its bindings (`0x14083F3F0..`). Which device is in use: **`0x140C2FF64`** (1 keyboard / mouse, 0 pad), set each frame by `0x140347EF0` (any key or mouse input → 1 and the key glyphs `0x140349F40`; any pad button or stick → 0 and the pad glyphs `0x140349120`); getter `0x1403471C0`, also read by Re:CoM's glyph picker `0x140345B40`. | `0x140347EF0`, `0x1403471C0` |

## KH1's (Checked (KH1 Steam decomp + exe))

| what | where |
|---|---|
| The pad step remaps the PS2 pad word through a 19-pair table: **R1 `0x800` → logical `0x400`** (the lock-on), L2 `0x100` → `0x800` and `0x400000`, R2 `0x200` → `0x400000`, L1 `0x400` → `0x40000` (the shortcut list), R3 `0x4` → `0x200` (camera recentre), Select `0x1` → `0x8000` (first person). | `0x14028B0B0`, table `0x140528290` |
| The lock-on: not locked, `0x400` pressed locks (state 3, the lock-on camera `0x1401E41A0`); locked, `0x400` pressed unlocks (sound 0x45), or its **release** when KH1's config `*(0x142E1F4D8)+4` is 1 (`0x1402FAFA0`; a hold-style option: Assumption); **`0x800` pressed (L2) steps to the next target** in its lock list. | `0x1402B0610` |
| L2 / R2's `0x400000`: a special camera mode's input (`0x14028C7E0`, mode 0xB) and the end of a held soft target (`0x1402B11B0`); no lock-on. | |

## Built

At Combat = KH1 in Sora's story (the main `CPlayerSora`; Riku's story and Combat = Re:CoM:
Re:CoM's), a wrapper on Re:CoM's lock-on writes its button field `+0x44` right before Re:CoM's
own runs: **R1 (8) when the pad is in use, Re:CoM's R2 (2) when the keyboard / mouse is**
(`crate::lock_on::button`). Re:CoM then locks, unlocks and switches targets as it does (press;
press and release; held with a right-stick flick), now on R1; its pad update writes 2 again the
next frame and nothing else reads the field.

- **The old button (R2)** does nothing at Combat = KH1 on a pad (KH1 gives R2 no lock-on or
  battle action; its camera use belongs to the camera work, `recom-m12-camera-plan.md`).
- **Keyboard and mouse**: Re:CoM's lock-on key, unchanged.
- **Button prompts**: Re:CoM draws button glyphs through its text escapes (`0x140345B40`: `F5 64`
  … `F5 6F` → the glyph table `0x14083F510..0x14083F52A`, filled per device by `0x140349120` /
  `0x1403488E0`). Escape `F5 6B` → `0x14083F522`, by its place in that table R2's glyph:
  Assumption. Whether any of Re:CoM's texts name the lock-on button (the tutorials, `CTechHelp`)
  is in its message files, not in the session: **not changed** (owner question, below).

Sites: hook `0x1401E8DA0` (6 bytes; its only code references are the two calls: rel32 scan; the decomp's other references are unwind data); checked: its
head, the pad update's stores, the three readers and their pad tests, the device detection and
its getter. `check_sites_rs.py`: 1390 sites, 0 mismatches.

Per frame: Re:CoM's lock-on runs once a frame; ours before it: one atomic load and the main-Sora
compare; at KH1 one flag read and one store.

## 1:1 checks

Checked (KH1 decomp + exe): KH1's lock-on button is R1 (raw `0x800` → `0x400`). Checked (Re:CoM
decomp + disasm + exe): Re:CoM's lock-on is R2 and reads only `+0x44`. Tests:
`lock_on::tests::r1_only_on_the_pad_at_combat_kh1`, `sites::tests::lock_on_sites`.
**Not KH1's in 0.133.0** (KH1's built in 0.137.0, below): how the lock ends and switches (KH1: R1 again
unlocks, L2 steps through the targets; Re:CoM: press and release unlocks, held + right stick
switches). **Not checked**: in game; the device flag's timing against the lock-on in the same frame
(Assumption: the input poll runs first); a controller remapped in Re:CoM's own settings (R1 is
whatever Re:CoM's R1 is).

## What to test (Combat = KH1, Sora's story, a battle, a controller)

1. R1: locks on (Re:CoM's sound and cursor). R1 again, then let go: unlocks. Hold R1 and flick the
   right stick: the next target that way.
2. R2: nothing.
3. Keyboard / mouse: Re:CoM's lock-on key as before; a controller press after it: R1 again (log
   `lock-on: the controller in use …` / `keyboard / mouse in use …`).
4. Combat = Re:CoM or Riku's story: R2 as before; R1 Re:CoM's (card scroll).
5. L1 held (the shortcut list) and R1: the lock (no sleight: Re:CoM's card input is off at KH1).

## Owner questions

1. ~~KH1's lock behaviour too?~~ Answered 2026-11-04: "1. B": KH1's rules, no hold-style row
   (built 0.137.0, below).
2. ~~Prompts~~: "dont worry about it": left as they are.

---

# 0.137.0: KH1's lock rules on R1

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Lock-on rules and camera walls), verbatim:
"1. B 2. dont worry about iit": **KH1's lock rules on R1** at Combat = KH1 on a controller: R1
again unlocks at once; L2 steps through targets. No hold-style Config row. Prompts as they are.

## KH1's lock-on, in full (Checked (KH1 Steam decomp + exe))

| what | where |
|---|---|
| Pad table (19 pairs, read from the exe): R1 `0x800` → `0x400`; L2 `0x100` → `0x800` **and** `0x400000`; R2 `0x200` → `0x400000` only; L1 `0x400` → `0x40000`. So **R2 steps nothing**. | `0x140528290`, `0x14028B0B0` |
| Edges: `0x14028B430` keeps held at `+4`, **pressed** (new this frame) at `+8`, released at `+0xC`. The lock-on reads `+8`. | `0x14028B430` |
| The lock-on (state `0x142DB3510`: 0 none, 2 soft target, 3 locked, 1 / 4 other locks). **Not locked**: R1 pressed locks its pick (`0x1402B1540`), or the soft target in state 1 / 2 / 4; **no target: nothing**. **Locked (3)**: R1 pressed **unlocks at once** (sound 0x45, `0x1402B0970`); the release instead only when KH1's config `*(0x142E1F4D8)+4` = 1 (`0x1402FAFA0`: KH1's hold option, not built: owner). Else **L2 pressed steps** (below). L2 unlocked: nothing (state 0 reads only `0x400`). | `0x1402B0610` |
| **The lock list**: entries `0x142DB3520` (8 bytes: entity, lock point; room for 97 up to the count `0x142DB382C`), cursor `0x142DB3830`; a lock with no list starts it with its target; every unlock empties it. **L2**: cursor not at the last entry → the next entry; at the last → KH1's pick among the targets **not in the list**, appended and locked; none → **back to the first entry** (if not already on it); else nothing. Each step plays the lock sound (0x44). | `0x1402B0610` |
| **KH1's pick**: each target's lock points (`0x1402B0110`), not Sora, alive / targetable (`0x1402B0030`); score `(int)max(0, |point − Sora's point (0x8003)| − the point's radius)` (`0x1402B04C0`), the room's priority class in the top byte (`0x142D539E8` + 9 / 10 / 11 by kind), targets flagged `+0x130 → +4 & 0x1000` only to 12 m, a clear line from the camera / Sora (`0x140193B60`); the lowest score wins, the first on a tie. **Nearest first** (by the lock sphere's surface), no angle. | `0x1402B1540`, `0x1402B0F90` |
| **Each frame** (`0x1402B11B0`, a task): Sora gone / in a state without the lock flag → unlock. Locked: the target gone or out of range (flagged targets: 15 m across or 10 m up) → **unlock** (no step to another target; then the soft target). The list drops the entries gone or out of range (the cursor stays on its entry). | `0x1402B11B0`, `0x1402B13F0` |

## Re:CoM's lock-on, the parts KH1's rules use (Checked (decomp + disasm + exe))

| what | where |
|---|---|
| The lock-on: its nearest lockable target to `+0x84C` (`0x1401CDB90`: Re:CoM's `LockOnManager` list `0x14087B1E0 + 0x88`, `+0x154 & 0x10000`, vtable `+0x50` points, plain distance). Unlocked: `call 0x1401FF740` (pressed) at **`0x1401E8E44`** → lock (cursor `+0xE0` ^= 1, `+0xD4` = 0, `+0xF4` = 1, its update `0x140208EC0`, sound 7). Locked: `call 0x1401FF600` (released) at **`0x1401E8E9C`** with `+0x424 & 0x1000` → unlock (sound 8); else `call 0x1401FF4F0` (held) at **`0x1401E8EFF`**, then the pressed reader (sets `0x1000`), `call 0x1401FF4A0` (stick flick) at **`0x1401E8F2D`**, `call 0x1401CD4B0(+0x850)` (the target that way) at **`0x1401E8F3C`** → switch (cursor `+0xE0` = 1, `+0xD4` = 0, `+0xF4` = 1, update, sound 7); held also sets `+0x424 & 0x800` (Re:CoM's camera step `0x1401FCF90` then ignores the right stick that frame; the request bits `0x1401F8150` clear it every frame). | `0x1401E8DA0` |
| The cursor update: a new target (a lock, a switch: `+0xF4` set) only within **14 m** in a battle room (`+0xF8 & 4`, set by the cursor init `0x140208120` with the enemy gauge), **3 m** elsewhere; holds to 30 m / 3 m; the target gone or not lockable → full unlock (`+0xF8 & 2`, a sleight's forced lock `0x14020B6A0`: re-take the nearest instead). Measured from `0x1401F0AB0(out, 0)` (the main player's mover `+0x30`) to the target's vtable `+0x50`. Pressing with no target at all **arms** the lock (locked on nothing; takes the next target that comes). | `0x140208EC0` (`0x14020919B`; data `0x140509248` 3.0, `0x140509258` 14.0) |
| The pad's pressed test `(mask, pad)`: `0x14029B2F0` → `0x14029C240` (pad 0's pressed word `+0x14`). L2 (`+0x40` = 1) is read only by the battle branch of the request bits (`0x1401FF6C0`), which Combat = KH1 does not run (`win::attack::request_bits`). | |

## Built

Combat = KH1, Sora's story, **the controller in use**: the lock-on wrapper (0.133.0) marks Re:CoM's
lock-on call as KH1's, and the lock-on's five calls (`sites::LOCK_ON_CALLS`, each `call rel32`
retargeted at install to one stub block near the exe; all or none) answer KH1's way. Outside such a
call (keyboard / mouse, Combat = Re:CoM, Riku's story) each answers with Re:CoM's own callee.

| call | KH1's answer |
|---|---|
| pressed, unlocked (`0x1401E8E44`) | R1 pressed **and** Re:CoM's pick (`+0x84C`) is a target its cursor takes (lockable, within 14 m / 3 m): **R1 with no target does nothing** (no armed lock, no lock sound). |
| released, locked (`0x1401E8E9C`) | R1 **pressed** (and `+0x424 \|= 0x1000`): **R1 again unlocks at once**, by Re:CoM's unlock and its sound 8. |
| held, locked (`0x1401E8EFF`) | **L2 pressed** (pad enabled). R1 held does nothing (Re:CoM's camera keeps the right stick). |
| flick (`0x1401E8F2D`) | yes (reached only on L2's press). |
| switch (`0x1401E8F3C`) | **KH1's next target** (`lock_on::History::step` with `lock_on::nearest`), or none (no switch, nothing happens). Re:CoM then switches: its cursor, `+0x84C` / `+0x850`, sound 7. |

**KH1's lock list** (`lock_on::History`, ours: Re:CoM has none): after each lock-on call its cursor
syncs it (a new lock → `[target]`; unlocked → empty; a target Re:CoM changed otherwise → restarted);
while locked, each frame drops the entries no longer lockable or past Re:CoM's take reach (the
current one stays); room 97 as KH1's. **KH1's pick**: Re:CoM's `LockOnManager` list walked as its
own picks do (lockable, not Sora), each at its vtable `+0x50` point; score `(int)max(0, distance ×
100 − the KH1 twin's lock-point radius)` (`stats::lock_radius`, 0 without a twin), the lowest, the
first on a tie, only within Re:CoM's take reach (a farther one would make Re:CoM drop the lock).

Not changed: the lock press's target (Re:CoM's pick, what its cursor shows), the target dying or
leaving range (Re:CoM's full unlock = KH1's; reaches below), sounds (Re:CoM's 7 / 8), prompts,
keyboard / mouse (Re:CoM's rules on its key), Combat = Re:CoM, Riku's story.

Sites: `LOCK_ON_CALLS` (5), `LOCK_ON_RULES_CHECKED` (8: the pick store, the pressed-again test, the
cursor test, the switch and flick heads, the pad pressed test, the cursor's reach test and
constants); `0x1401F0AB0` and `0x14029B2F0` are checked where they already were (`GIMMICK_HP_CALLEES`
"Sora's point", "pad fresh"). Test `sites::tests::lock_on_rules_sites` (each call's target, inside
the hooked function, no overlap; fields and reaches). `check_sites_rs.py`: 1420 sites, 0 mismatches.

Per frame: Re:CoM's lock-on once; at KH1 on the pad two atomic stores more; unlocked one load;
locked a lookup and a point read per listed target (KH1's drop). Each retargeted call adds an
atomic load. An L2 press walks Re:CoM's target list once.

## 1:1 checks (0.137.0)

- **Checked (KH1 decomp + exe)**: the buttons (R1 lock / unlock, L2 step, R2 none); press edges, not
  releases; R1 locked unlocks at once; no target, nothing; L2 unlocked nothing; the list's order
  (next entry / new pick appended / back to the first / nothing), its reset at unlock, its per-frame
  drop; the pick's score (nearest by the lock sphere's surface, whole units, first on a tie); the
  target dying → unlock, no auto-step. Tests: `lock_on::tests` (6).
- **Not 1:1 (Re:CoM's kept)**: (1) the lock press's target is Re:CoM's nearest (plain distance, what
  its cursor shows), not KH1's pick (KH1: its soft target when there is one, else its pick); (2) KH1's
  lock points (a boss's several parts are separate entries in KH1; Re:CoM has one point an actor);
  (3) KH1's pick's priority class per room, its line-of-sight test and its 12 m limit for flagged
  targets: none (Re:CoM's 14 m take reach instead); (4) ranges: Re:CoM's (holds to 30 m in battle,
  14 m to take), KH1's 15 m / 10 m only for flagged targets; (5) the measure: Re:CoM's points (its
  player's mover, the target's vtable `+0x50`) and the KH1 twin's radius, against KH1's Sora point
  0x8003 and its lock points (Assumption: close enough to order the same); (6) sounds Re:CoM's.
- **Not checked**: in game; KH1's hold option's default (Assumption: press, the owner's choice).

## What to test (0.137.0: Combat = KH1, Sora's story, a battle, a controller)

1. Several enemies. R1: locks (Re:CoM's sound and cursor). **R1 again: unlocks at once** on the
   press (sound 8), no wait for the release.
2. Locked, **L2**: the next nearest enemy not yet visited; keep pressing: each enemy once, nearest
   first, then back to the first one and round again. Log: `lock-on: L2 on 0x…: next 0x…; list […]`.
3. One enemy, locked, L2: nothing. Not locked, L2: nothing.
4. No enemy near (or all past 14 m), R1: nothing (no lock sound, no lock waiting for a target).
5. Locked, hold R1 and move the right stick: the camera turns, no target switch. R2: nothing.
6. The locked enemy dies: the lock ends (as before); R1 locks the next.
7. Keyboard / mouse: Re:CoM's key and rules (press, press and release, hold + stick) as before.
   Combat = Re:CoM or Riku's story: R2 and Re:CoM's rules as before.
8. Log at start: `lock-on: KH1's lock rules on R1 …`; on a device change `… KH1's rules (R1 again
   unlocks, L2 steps)`.

## 0.173.0 (fork): KH1 FM's Auto Lock and Target Lock

- KH1's Config block `*(0x142E1F4D8)` (Checked (KH1 exe: disasm)):
  - `+0` Auto Lock, 0 = On (getter `0x1402FAFB0`; readers `0x14029365F`, the attack command's soft
    lock state 4 via `0x1402B08A0`; `0x1402B1348`, a new pick `0x1402B0A00` after the lock is lost);
  - `+4` Target Lock, 1 = Manual (`0x1402FAFA0`; read only by the locked state's unlock test
    `0x1402B069C`: Manual = R1's release `[pad+4] & 0x400` clear, Auto = its press `[pad+8] & 0x400`);
  - `+8` the camera's Auto / Manual (`0x1402FAFD0`).
- Ours:
  - `lock_on` arms `AUTO_LOCK` on a frame with the Attack request (`+0x424 & 4`) and no lock, or
    when the last frame's lock ended without the player's unlock (`PLAYER_UNLOCK`, set by
    `lock_released`).
  - `lock_pressed` then answers "pressed" for Re:CoM's pick, its reach rules as for R1.
  - `lock_released` reads R1's release at Manual.
  - Settings > Camera rows "Auto Lock" (Off / On) and "Target Lock" (Auto / Manual).
