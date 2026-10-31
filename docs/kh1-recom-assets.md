# The asset build step: KH1 motions in Re:CoM's motion pack (M2)

Crate `crates/kh1-recom-assets` (lib + bin `kh1-recom-build`). It reads the player's own KH 1.5+2.5
folder and writes a map from every KH1 Sora motion to the Re:CoM slot that plays it, plus an
extended `PLAYER/0001/PL0001.ANN` into Panacea's Re:CoM mod folder. Re:CoM's engine then plays
KH1's motions as its own. Nothing from either game is in the repo.

Owner decision (2026-10-30): "just use RECOM's default animations, and KH1 animations for anything
that isnt included in ReCOM". So:
- a KH1 motion that Re:CoM already has (Square Enix baked it into Re:CoM) plays Re:CoM's own clip
  at Re:CoM's own slot;
- only the KH1 motions Re:CoM lacks get a new 60 fps clip, appended after Re:CoM's 161 slots;
- Re:CoM's 161 slots stay byte-identical. If no KH1 motion needs a new slot, no pack is written,
  and Panacea serves Re:CoM's own.

Tags: Checked (decomp / exe / files / source) or Assumption.

## Use

```
kh1-recom-build [--game <KH 1.5+2.5 folder>] [--mod-dir <mod_path>] [--out <file>] [--loop E,..] [--once E,..] [--dry-run]
```

- `--game` defaults to `.`. It reads:
  - `Image/dt/Recom.hed/.pkg`: `PLAYER/0001/PL0001.ANN` and `.MDL`;
  - `Image/dt/kh1_*.hed/.pkg`: `xa_ex_0010.mset` and `.mdls`;
  - the key table from `KINGDOM HEARTS Re_Chain of Memories.exe` (or the KH1 exe).
- It always writes `<mod_path>/recom/kh1-motions.json` (where the runtime, `crates/kh1-recom`
  `win/motion.rs`, looks first). When some KH1 motion needs a new slot, it also writes
  `<mod_path>/recom/PLAYER/0001/PL0001.ANN` and `<mod_path>/recom/PLAYER/0002/PL0002.ANN` (the
  Keyblade, when PL0002 is in the archive). When none does, it writes no pack and removes a pack
  an earlier run left (only a file with more than 161 slots, i.e. one it wrote). With `--out`, the
  manifest goes next to the pack.
- Manifest (format `kh1-recom-assets motions 2`; read by `crates/kh1-recom` `motions.rs`
  `parse_manifest`, which ignores unknown keys). Top level: `format`, `pack`, `base_slots` (161),
  `slot_count`, `pack_changed`, `fps` (60, the new clips), `kh1_entries` (118), `recom_entries`,
  `new_slots`, the weapon fields, `sources` (sha256 of the inputs) and `slots`.
- `slots` is the **full map: one object per KH1 entry (118), in entry order**. Later runtime
  stages request motions by KH1 entry through it. Fields:
  - `kh1_entry`, `kh1_request`, `requests`, `kh1_frames`;
  - `slot` / `name`: the Re:CoM slot to play: Re:CoM's own clip when it has the motion, else the
    new slot;
  - `source`: `"recom"` or `"kh1"`;
  - for `recom`: `match_kind` (`"bake"` or `"edited"`), `match_error` (degrees), `edited_bones`,
    `root_split_mm`, `ambiguous` (other passing slots; empty for all 118);
  - `frames`, `fps` (30 for Re:CoM's clips, 60 for new ones), `loop` (the clip's own flag),
    `loop_overridden`, `cycle_error_m`, `root_motion` (KH1's request-map bit 15), `bytes` and `keys`
    (new clips; 0 for Re:CoM's);
  - `weapon_name` (the Keyblade clip at that slot, or `null` when Re:CoM has none there, as for
    its own play) and `weapon_match_error` (Re:CoM's Keyblade clip vs KH1's blade, degrees).
  - `kh1_request` is the lowest request whose map word names the entry as its base entry, else the
    lowest that reaches it as a type-3/4 variant. For example, 0 → 0, 2 → 1, 5 → 2, 98–101 →
    200–203.
  - `requests` lists every request that plays the entry.
  - **Runtime impact (for `crates/kh1-recom`, not changed here):** its `plan` refuses a swap whose
    slot is below 161 ("would be one of Re:CoM's own"). With this map, entries 0, 1 and 98–101 are
    on Re:CoM's slots 1, 31, 55–58, so `plan` as it stands fails all-or-nothing. Those swaps are
    now no-ops (or point the row at Re:CoM's own baked clip); entries 2, 3, 5, 6 are on new slots
    161–164.
  - `mod_path` comes from `<game>/panacea_settings.txt`. If an existing folder named `recom` in
    another letter case is there, it is reused.
  - Panacea adds `/recom` for Re:CoM (`OpenKH.cpp`) and serves `<mod_path>/recom/<archive path>`
    in place of the archive file (`Panacea.cpp` `TransformFilePath`). Checked (source).
- Library: `build::build(&Inputs{mset, mdls, ann, mdl, weapon: Option<(ann, mdl)>}, &BuildOptions) -> Built`. `Built` holds
  `ann`, `bytes`, `changed`, `entries` (one per KH1 entry), `weapon` (the extended PL0002) and
  `manifest_json()`. `BuildOptions::reuse_recom` (default true) is the owner's rule; false converts
  every entry. The building blocks are:
  - `matching::{match_all, score, DecodedClip}`: which KH1 motions Re:CoM has;
  - `ann::{Ann, Clip, Curve, Channel}`: read, write and sample;
  - `mdl::RecomSkeleton`;
  - `convert::{Converter, ClipSpec, bone_map}`.
- Run on a fake game folder (`scratch/fakegame2`, `--mod-dir scratch/modout`, Sora + Keyblade,
  release, Linux): 1.45 s in total (0.49 s reading, 0.91 s matching + converting). 49 KH1 entries
  play Re:CoM's clips (39 bakes, 10 with Re:CoM edits), 69 get new slots 161–229. Output:
  `PL0001.ANN` 10,045,360 bytes (230 slots; Re:CoM's own is 1,740,528), `PL0002.ANN` 839,536
  bytes (230 slots), a 52,721-byte manifest. (Before the owner decision, all 118 were converted:
  15,644,304 bytes.)
- Windows build:
  `RUSTC_BOOTSTRAP=1 CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc cargo build --release --target x86_64-pc-windows-gnu -p kh1-recom-assets -Zbuild-std=std,panic_abort`
  gives `kh1-recom-build.exe` (744 KB). Not run on Windows yet.

## Which KH1 motions Re:CoM already has (`matching.rs`). Checked (files)

The rule, run at build time on the player's own files, deterministic:
- Each KH1 entry is posed with `kh1-anim` at t = 2f (Re:CoM frame f at 30 fps = KH1 frame 2f at
  60 Hz) and collapsed onto Re:CoM's bones exactly as the converter does (with the entry's own
  root flag).
- It is compared with every Re:CoM clip of fitting length (|2F − N| ≤ 4) on all 135 bones, **only
  at the clip's key frames** (Re:CoM's clips are key-reduced; at a key the bake is exact up to
  u16 rounding): local rotation (geodesic angle) and local translation.
- A bone agrees when its worst error is ≤ 0.9° and ≤ 1 mm. The translations of `Root` and
  `ex_0010_sk` are left out: where Re:CoM keeps the travel is its own per-clip choice (reported as
  `root_split_mm`).
- **Bake:** every bone agrees. `match_error` = the worst bone rotation.
- **KH1 motion with Re:CoM-only edits** (counts as present): at most 20 bones disagree, the entry
  has no bake, and the clip is not another entry's bake. This covers `kh1-posing.md` §8's "bones
  KH1 leaves unkeyed are animated in Re:CoM".
- This is a different metric from `clip_origin.py`'s (max matrix element on 8 FK-only bones;
  true ~0.003 vs next ≥ 0.5). Its separation, all 118 entries × all fitting clips:

| | value |
|---|---|
| bakes (39): worst bone rotation | ≤ 0.74° (entry 74; the spine-aim residuals of `kh1-posing.md` §8); 98: 0.019°, 117: 0.009°, 4: 0.005°, 8: 0.010° |
| edited (10): disagreeing bones | 4–16 |
| closest other pair | 17 bones: KH1 76/77/78 against each other's bakes 155–157 (variants of one motion; excluded because each clip is another entry's bake) |
| closest clip for an entry Re:CoM lacks | 26 bones: KH1 86/87 against 84's bake (slot 150) |
| every other pair on a clip that is no bake | ≥ 74 bones |

- Found: the 46 pairs of `clip_origin.py`'s table (test `BAKES`), plus 0 → p001_001, 65 →
  p001_043 and 78 → p001_157. No entry passes with two clips, no clip serves two entries (test
  `f_…`).
- Edited, with the bones Re:CoM changed:
  - 0 → p001_001 (idle): keychain `chain1/2/4`, `neckla1`;
  - 18, 20–23 → p001_008, 010–013 (requests 0x11, 0x13–0x16, the ladder set): legs, collars,
    arms, and the hips 22 cm higher;
  - 26 → p001_014 (0x19): waist and arms, 1–5°;
  - 57 → p001_063 (0x3D ground cast): legs;
  - 65, 66 → p001_043/044 (0x4C/0x4D flinch): jacket tails.
- Root split differs (Re:CoM keeps the travel elsewhere than KH1's request bit says): 14, 15, 16,
  74 (bakes; 0.5–1.7 m) and 18–23, 26, 57 (edited). **Open for the runtime:** play these with the
  root flag Re:CoM's own call sites use for those clips, not KH1's bit.
- The Keyblade plays the same slot. Re:CoM has a Keyblade clip at 41 of the 49 slots; their
  `weapon_match_error` is ≤ 0.18° except entry 4 (1.58°, the known `k9` residual below). The
  ladder set (18–23, 26) has no Keyblade clip in Re:CoM, as in Re:CoM's own play.
- What the 69 new slots are (`requests`, from `kh1-movement-motion.md` / `kh1-data-values.md`;
  names for ids not in those notes are not known yet):
  - walk 2, 3 and run 5, 6, 7 (requests 1, 2: KH1's own walk/run; Re:CoM's run is not a KH1 bake);
  - jump apex 9 (5), fall loop 10 (6/0xB), High Jump 12, 13 (9, 0xA);
  - 17 (0x10), 19 (0x12, ladder), 24, 25 (0x17, 0x18), 27–53 (0x1B–0x35, among them the long
    loops 27, 36, 50 of 320–330 frames), 59, 60 (0x3F, 0x40), 62, 63 (0x49, 0x4A), 68 (0x4F);
  - knock-back 69, 70 (0x50, 0x51), 71 (0x6A), recoil bounce 73 (0x6F);
  - 83 (0xA6), 86, 87 (0xB7, 0xB8), 89 (0xBA), push 91–93 (0xBC–0xBE), 94–97 (0xC0–0xC3);
  - combo/abilities: 105 (207, row 12), Hurricane Blast 107 (209), Blitz 108 (210), Vortex 109
    (211), **Guard** 110 (212), Counterattack 111 (213), Aerial Sweep 112 (214), Ripple Drive 113
    (215), Gravity Break 115 (217).
  - Re:CoM already has: idle 0/1, walk loop 4, jump 8, land 11, the glide set 75–78 (0x71–0x74),
    the ground and air combos 98–104, Sliding Dash 106, Stun Impact 114, Zantetsuken 116, Dodge
    Roll 117, among the 49.

## Re:CoM `.ANN` details found while writing the encoder. Checked (files)

These add to `motion-conversion.md` §1–2.

- Every Re:CoM clip has has-blend byte `+0x17 = 2`, a u16 blend at `+0x18` (0, 8, 15 or 30), and
  `00 00` at `+0x1A`.
- Every clip blob ends with 4 zero bytes inside its size. Curve blocks are exactly their content
  (no padding).
- Bytes the game never reads, kept by the reader so that round trips are exact:
  - the const-table slot of an animated channel (usually the frame-0 value);
  - the pad byte of an odd u8 key array (0xBA, 0xFE or 0xFF);
  - the linear bits past the last segment, which are all 1;
  - the file tail: 10 × 0xCD, padding to 16 bytes.
- Constant channels have size field 0.
- **Decode → encode of all 69 clips / 161 slots is byte-identical**, for the whole file and for
  every clip (test `a_round_trip_is_byte_identical`).

## How Re:CoM composes a bone. Checked (decomp)

`FUN_14023EB00(bone, parent)` (bone record 0xE0 bytes):
- `+0x60` is the unscaled world rotation: `R_local · U_parent`;
- the world rows `+0xA0/+0xB0/+0xC0` are those rows × the bone's **own** scale `+0x90/94/98`
  (absolute, not inherited);
- the world position `+0xD0` is `T_local · W_parent` (the parent's **scaled** world) plus the
  parent's position.

KH1's world matrices have the same structure (rows = accumulated scale × unscaled rotation;
`kh1-posing.md`). So for KH1 targets (U, S, P) the converter writes:
- `R = U·Uᵀ_parent` (Euler XYZ, continuous frame to frame);
- `S = S`;
- `T = (P − P_parent)·Uᵀ_parent·diag(1/S_parent)`.

Sampler writes: `FUN_140294F10` writes the rotation and translation, and `FUN_14024CCC0` writes
the scale to `+0x90`.

## Conversion rules

- **Bones.** The bone map is a port of `skel_map.py`. All 134 bones other than `Root` are mapped;
  for example `WeaponR` (49) is KH1 101. Checked (files).
  - `Root` (0): identity rotation, scale 1.
  - `ex_0010_sk` (1): KH1 bone 0's position with an identity rotation, as in Re:CoM's rest pose and
    bakes.
- **Units.** KH1 world matrices are `W·diag(1, −1, −1)`, × 0.01 to metres (Y up). Checked (files).
- **Root travel.** Travel goes in `Root` exactly when the request-map bit 15 of the requests that
  play the entry is set: 30 entries (16, 17, 26, 28, 35, 37, 44, 49, 57, 65, 66, 73, 82, 90, 98–101,
  105, 106, 108–117).
  - Travel = `Pose::root_travel` (the last helper), converted like a point.
  - Otherwise `Root` is 0 and the travel stays in `ex_0010_sk`. This matches KH1: its caller
    subtracts the travel from the model matrix only with root motion (`kh1-posing.md` §2.0).
  - Sora has no entry that is played both with and without bit 15. Checked (files).
- **Frames.** 60 fps (d = 0), poses at t = 0, 1, …, so every 60 Hz game frame is one whole clip
  frame.
  - Play-once: N + 1 frames (Re:CoM's duration is frames − 1, so the clip plays 0..N and holds N,
    as KH1 does).
  - Loop: N frames with loop start 1 (wraps N → 0).
  - u16 key indices at 256+ frames (entries 27, 36, 50).
  - All 118 entries are referenced by the request map. The 69 that Re:CoM lacks go to slots
    161 + i (i = rank among them, in entry order), named `p001_161` … `p001_229`.
  - `--loop/--once` apply to new clips only; Re:CoM's clips keep their own flag.
- **Keys.** A key on every frame, all linear. The only keys left out are those inside runs of
  equal quantised values; that is exact, because a linear segment between equal values returns
  that value.
  - A channel becomes a constant when its spread is ≤ 1e-5° / 0.1 µm / 1e-6 (scale). This is below
    one u16 step of any real channel.
  - Scale channels are used only when some scale ≠ 1.
  - −0.0 constants are written as +0.0, because the loader ORs 1 into non-zero constant bits.
- **Loop or play once. Assumption.**
  - KH1 has no per-motion loop flag: the request call decides, and request bit 0x10000 means
    "hold" (`kh1-movement-motion.md` §5). Re:CoM decides by clip.
  - Rule used: a clip loops when KH1's pose at t = N equals the pose at t = 0, with every mapped
    bone within 1 cm in model space and root travel included. Without travel included, swing 3
    (entry 100, 4.5 mm) would count as a loop.
  - It agrees with all 46 Re:CoM bakes of KH1 motions (test `d_…`): the loops' error is ≤ 6.2 mm
    and the play-once motions' is ≥ 11.3 mm. The thin margin is entries 55 (11.3 mm, play-once)
    and 39 (6.8 mm, loop).
  - Result: 42 loops, namely 0–7, 10, 14, 18, 22–25, 27, 30–34, 36, 39–43, 45, 46, 50–53, 75–78,
    83, 85, 88, 92, 110. Entry 110 is root motion with zero travel.
  - `--loop/--once` override it. If one KH1 entry is played both ways, it needs two slots (not
    done).
- **Default blend in the clip:** 8 frames, Re:CoM's own default.
  - KH1 has **no per-motion default blend**. It is not in the motion header (+0x00..+0x3C) or the
    request-map word (entry, type, root bit, extra id); the code passes it with each request call
    (`FUN_14029F7C0` / `F980` / `FA30`, `kh1-movement-motion.md` §5). Checked (decomp notes +
    files).
  - So the runtime should pass KH1's blend per call. The clip value is used only for blend < 0
    (`FUN_1401A3470`).
  - With row swaps, each Re:CoM call site keeps its own blend and rootFlag. For 98–101 the
    manifest's `root_motion` is true and for 0–6 it is false. The runtime should check that the
    swapped rows' call sites pass the same.
- **Lost by design:** KH1 IK becomes fixed motion, as in Square Enix's PS2 bakes. Between frames,
  Re:CoM slerps (only with haste, the 30 fps mode or blends). Also see `motion-conversion.md`
  "Loss".

## Validation (tests in `tests/validate.rs`; `KH1_TEST_DATA`, skipped when unset)

(b) KH1 entries converted at 60 fps, sampled with the decoder at even frames, against Square
Enix's 30 fps bakes at their key frames. All 135 bones are compared (Root and the 134 mapped
bones): local rotation (geodesic angle), translation and scale.

| KH1 → bake | rotation max (n) | translation max (n) | scale max | every 60 Hz frame vs KH1 pose (world pos / rot) |
|---|---|---|---|---|
| 98 → p001_055 | 0.0187° (1753) | 0.015 mm (9603) | 1.1e-5 | 0.025 mm / 0.0034° |
| 117 → p001_070 | 0.0098° (1329) | 0.013 mm (7560) | 1.8e-7 | 0.045 mm / 0.0052° |
| 4 → p001_020 | 0.0050° (1478) | 0.001 mm (9597) | 7.4e-6 | 0.006 mm / 0.0011° |
| 8 → p001_004 | 0.0093° (843) | 0.001 mm (5175) | 2.0e-5 | 0.018 mm / 0.0018° |

- The 0.0187° on entry 98 is the known right-knee residual (`kh1-posing.md` §8). It is
  `kh1-anim`'s, not the encoder's.
- The last column decodes the new clip, composes it by `FUN_14023EB00`'s rule and compares it
  with KH1's pose at every 60 Hz frame. That is the encoder's own loss (u16 steps).
- (c) The map has 118 entries (49 Re:CoM + 69 new). The pack parses with the Re:CoM-format
  reader: 230 = 161 + 69 slots, Re:CoM's 161 unchanged, names agree with slots; Re:CoM entries
  point at Re:CoM clips with the same frames / fps / loop. A build whose entries are all on
  Re:CoM's clips returns Re:CoM's bytes unchanged.
- (f) The matching (above).
- Square Enix's bakes have N/2 frames, except three play-once ones with N/2 + 1 (slots 10, 11,
  14). Checked (files).

## Loader limits. Checked (decomp)

- `FUN_14029F3A0` (ANN loader): the slot count comes from the file, the pointer array is
  allocated with `count × 8` (`FUN_14023D180`), and each clip is `new`ed and parsed. There is no
  fixed array.
  - `FUN_1404ACE54` / `new` is plain `malloc`, not a fixed heap.
- `FUN_14027ABC0` (clip parser):
  - the key count is kept as s16 (Sora's max is 331);
  - frames and loop start are s16;
  - the u8/u16 key switch is `frames < 0x100`, read from the clip;
  - fps is a byte (60 fits).
- `FUN_1401A47E0` (name → index): index = the last 3 digits (`c[-1] + (c[-2] + c[-3]·10)·10 −
  0x14D0`), checked against the clip's name while < count. Otherwise it searches linearly down,
  then up.
  - So `p001_161`…`p001_999` hit directly, and longer names still resolve.
  - The count comes from the set (`FUN_14024DC50`), so nothing stops at 161.
- `FUN_14024DC60`, `FUN_14024C760`, `FUN_14024E9F0`: an index < 0, ≥ count, or on an empty slot
  gives a null clip, and the start returns early. Not a crash.
- `FUN_14017A240` (EET effects) and `FUN_140239260` (SES sounds) find per-motion events by linear
  search on the motion id. New ids simply have none yet.
- Assumption: Panacea serves a replaced file larger than the archive's (15.6 MB vs 1.7 MB). KH2
  mods do this routinely; check it on the first in-game load.

## What the runtime must do to play the new slots

1. **Action rows.** Sora's action table is at `0x1407C5E80` in `.data`. Checked (exe + decomp):
   - it has 71 rows of `{char* name; int idx}` (`p001_000`…`p001_104`), ended by a null name;
   - `DAT_1407C6300[1]` and `[2]` both point to it (char 2 loads `PL0011`);
   - at load, `FUN_1401F4240` and `FUN_1401FA6E0` fill `idx` by name through `FUN_1402A42B0`
     (a linear search of resource `DAT_1407C6338[char]` = `"PL0001"`, type 0x1D);
   - `FUN_1401F7180` reads `idx` with no bounds check.

   So the runtime should:
   - allocate a table of 71 + (new slots, 69) rows + a null row;
   - copy Re:CoM's 71 rows;
   - append `{"p001_161", −1}` … from the manifest;
   - repoint `DAT_1407C6300[1]` (and `[2]`) before the fill runs, or fill `idx` itself.

   New action ids 71… map to slots 161…, so the manifest's slot and name are all the runtime
   needs. Re:CoM's own ids 0–70 are unchanged. KH1 entries on Re:CoM's slots need no new row
   when an existing row already names that clip (e.g. 98 → `p001_055`).
2. **Play** through `FUN_1401F7180(player, actionId, blend, rootFlag, …)`:
   - rootFlag = the manifest's `root_motion` (Re:CoM samples `Root` only then:
     `FUN_14024CCC0`, `FUN_14024C960`);
   - blend = KH1's blend for that request.
   - KH1's own timing detail: with root motion, its model matrix uses the *previous* frame's
     travel (`kh1-posing.md` §2.0). Compare it in the M2 pose capture.
3. **Char 2 (`PL0011`) shares the table.** Without the same slots in `PL0011.ANN`, its rows
   resolve to −1, which gives a null clip and no motion change.
   - Open: what PL0011 is, whether its skeleton is the same 135 bones, and whether the step
     should extend it too.
4. **The Keyblade** plays the same index on its own set, so the swapped or added slot numbers
   automatically pick `w001_NNN` from the extended PL0002.ANN (below). Nothing more is needed at
   runtime.
5. Hit windows, combo / free flags, sounds (SES), effects (EET) and the trail are not in the
   clip (`motion-conversion.md` "What does not come with the motion").

## The Keyblade's motions (`PLAYER/0002/PL0002.ANN`)

- **How Re:CoM attaches it. Checked (decomp + exe).**
  - `CPlayerWeapon` is built in `FUN_140224BA0(weapon, player, char, bone)`, called from
    `FUN_1401FA6E0` with `bone` = `FUN_1401A4450(player, "WeaponR")`.
  - For chars 1–2 it loads its own resource group `DAT_1405A6910[char] + 1`. The table is
    `{0, 1, 0xB, 0x65, 0x6F, 0xC9, 0xD3}`, so Sora = group 2 (`PLAYER/0002/PL0002.{MDL, ANN, RTM,
    EET}`) and char 2 = group 12 (`PL0012`).
  - It sets its bone 0's parent pointer (`+0x50`) to the player's `WeaponR` bone. `FUN_14023EE10`
    / `FUN_14023F120` then compose bone 0 with `FUN_14023EB00(bone0, WeaponR)`.
  - `FUN_1401F7180` plays **the same motion index** on it, with rootFlag 0, when
    `player+0x418 & 0x40000000` (set for chars 1–2). So its bone 0 is never sampled: it keeps its
    rest local (identity) and sits exactly on `WeaponR`.
  - Chars 3–6 use the `wep0` bone of their own model instead.
- **PL0002. Checked (files).**
  - 158 slots and 57 clips named `w001_NNN`, at the same slot numbers as Sora's clips. Slots 8,
    10–14 and 150–153 are empty, among others; a missing slot is a null clip, which is safe. So
    the KH1 throws baked at Sora's 150–153 have no Keyblade clip, and my earlier guess that PL0002
    holds the throw was wrong.
  - All clips are 30 fps, and all but two have no scale channels.
  - Skeleton: 12 bones `Root`, `ex_5010_sk`, `wep0`, `k9`…`k1`.
  - Decode → encode is byte-identical.
- **What the clips hold. Checked (files).** It is the same kind of conversion as Sora's, in
  `WeaponR` space:
  - `Root` is constant 0;
  - `ex_5010_sk` is the grip: KH1 bone 280's position relative to KH1 101 (= `WeaponR`), with
    identity rotation. In w001_055 it is (−0.0152, −0.0016, 0.0316), KH1's battle grip;
  - `wep0` is KH1 282;
  - `k9`…`k1` are KH1 283…291, the blade bones. The segment lengths 0.2111, 0.0672, …, 0.0326
    equal KH1's ×0.01.
  - So most clips are **not** a static pose: `k9` and the blade chain are animated (283's swing),
    while `wep0` / `ex_5010_sk` are constant in a motion whose grip doesn't change.
- **Conversion** (`Converter::weapon` / `convert_weapon`, `weapon_bone_map`).
  - Bones 2.. are mapped among KH1 bones under the hand bone by rest rotation (< 0.01) and nearest
    rest position within 1 cm. PL0002's rest grip is ≈ 5 mm from KH1's, so Sora's 0.1 mm rule
    can't be used.
  - Blade segment lengths must then agree to 0.1 mm.
  - Bone 1 is the KH1 child of the hand on the way to `wep0` (280). Bone 0 is the hand (101).
  - Targets come from KH1's posed worlds. Locals use `FUN_14023EB00`'s rule, with bone 0's parent
    = `WeaponR`, so bone 0's local is the identity.
  - Frames and loop are the same as Sora's clip at that slot. Clips go to the new slots 161–229
    as `w001_NNN`; slots 158–160 are left empty; Re:CoM's 158 slots are byte-identical.
  - The pack is 839,536 bytes (Re:CoM's own is 124,160).
- **Validation** (test `e_…`, every bake where PL0002 has a clip at a slot whose Sora clip is a
  KH1 bake: 36 clips, at their key frames, all 12 bones):
  - KH1 98 vs w001_055: 0.0069° and 0.0005 mm. KH1 117 vs w001_070: 0.0053°. KH1 8 vs
    w001_004: 0.0055°.
  - **34 of 36 are within 0.05°.**
  - The two exceptions come from `kh1-anim`, not the encoder:
    - **Entry 4** (walk loop, w001_020) is off by 1.58°, all of it on `k9`. Entry 4 is the only
      motion where KH1 283 has a constraint: flags `0x200002E`, copying the orientation of helper
      10 in a 12-helper rig. The rest of the blade and all of Sora's body match. Cause open: a
      data difference from the PS2-era motion, or a helper detail. A PC capture of +0x1B4 bone 283
      in entry 4 would decide it.
    - **Entry 74** is off by 0.18°. It is one of the spine-aim residual motions in
      `kh1-posing.md` §8.
  - Self-check: the decoded weapon clip hung from KH1's hand vs KH1's blade bones, at every 60 Hz
    frame, is ≤ 0.012 mm and ≤ 0.004°.
- The CLI reads PL0002 from `Recom.pkg` when it is there. It writes
  `<mod>/recom/PLAYER/0002/PL0002.ANN`, and adds `weapon_pack`, `weapon_base_slots` and a
  per-slot `weapon_name` to the manifest. The runtime's parser ignores these and still plans all
  12 row swaps (checked).

## Open

- The loop rule is an Assumption. One KH1 entry played both looping and held would need two
  slots.
- The runtime's `plan` refuses Re:CoM slots (see "Use"), and the root split of 14–16, 18–23,
  26, 57, 74 (see the matching section): both are for `crates/kh1-recom`.
- Owner question: KH1's walk/run (2, 3, 5–7) are not in Re:CoM as bakes, so they get KH1 clips,
  while Re:CoM has its own (non-KH1) walk/run. The rule follows "included" = a bake of the same
  KH1 motion; using Re:CoM's own different-looking motions for them would need a mapping by role.
- PL0011 / PL0012: Sora's second model and its Keyblade (see above). Not extended.
- Keyblade entry 4's `k9` residual (1.58°): an oracle question for `kh1-anim`.
- Assumption: OpenKH Mods Manager rebuilds the mod folder on "Build" and may delete files it did
  not place. Run the step after a build, or ship it as the mod's install step (M14).
- Pose capture in game (the M2 "done when") is not done.
- Version / `AGENTS.md` stage entry for this crate: left to the lead (this task was limited to
  the crate and this note).

## KH1 stats (`kh1-stats.json`, 0.7.0)

- `kh1-recom-build` also writes `kh1-stats.json` next to `kh1-motions.json`: Sora's KH1
  Strength for levels 1..100 (decision 18) and the KH1 twin record of 31 Re:CoM enemy classes
  for each Re:CoM world (decision 21). Read from the player's `btltbl.bin` and the twins'
  `.mdls` (part 5, the AI script). Module `stats`; details in `recom-kh1-attack.md` § Damage.
- Format 2 (0.7.5) adds per enemy `lock_radius` (its first kind-0 lock point's radius, KH1 units)
  and `lock_radii` (all of them), read from the twin's `.mdls` part 4 (`stats::lock_radii`). The
  runtime reads format 1 too (radius unknown, 0).
- The local mod's `mod.yml` copies it like the other files.
- Checked (files): the owner's install gives all 31 twins with valid records; the motion pack
  and manifest it rebuilt are byte-identical to the installed ones.

## Sora's motion sounds (`PL0001.SES`, 0.7.2)

- `kh1-recom-build` also reads Re:CoM's `PLAYER/0001/PL0001.SES` and writes it next to the
  motion pack, adding entries for the KH1-only clips (Guard, entry 110; Counterattack, 111).
  Module `ses`. Ids and frames are in `recom-kh1-attack.md` § Guard sounds.
- **Format** (Checked, files + decomp `0x1402387E0`):
  - Header: `"@SES"`, version 1, entry count, entry table offset 0x10.
  - Entries, 0x30 bytes: clip name; +0x20 −1 (filled at load); +0x24 event count; +0x28 event
    offset.
  - Events, 0x50 bytes: group / index (id = group × 1000 + index); bone; offset; +0x18 start
    frame (60 Hz); +0x1C stop; +0x20 variant; +0x24 fade; +0x28 flags (1 footstep, 4 random
    from the table at +0x40).
  - The writer lays out the header, then the entries, then the events, and keeps Re:CoM's
    entries unchanged. Checked (files): it rebuilds Re:CoM's own file byte for byte
    (12,736 bytes, 50 entries).
- Re:CoM binds entries to clips by name, so the new entries only reach the appended clips.
  Re:CoM's own clips and sounds are unchanged.
- The local mod's `mod.yml` copies it like the other files.
