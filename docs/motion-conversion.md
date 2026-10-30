# Converting KH1 Sora's motions into Re:CoM's motion format

Research only; no converter exists yet. Tags: Checked (decomp / files) or Assumption. Files read: the
owner's KH1 `xa_ex_0010.mset` / `.mdls`, and Re:CoM's `PLAYER/0001/PL0001.{ANN,MDL,BIN,EET,SES,RTM}`
decrypted from `Image/dt/Recom.pkg`. None of them are in this repo.

## The headline

**Re:CoM's Sora is KH1's Sora.** Re:CoM's 135-bone skeleton matches KH1's exactly (×0.01, cm → m,
same axes): 134 bones at distance 0, plus one special case (below). About 50 of Re:CoM's Sora clips
are KH1 motions re-sampled at 30 fps. For example, `p001_055`–`p001_058` are KH1's ground swing 1,
swing 2, lunge and finisher (KH1 entries 98–101), and `p001_070` is the Dodge Roll (KH1 entry 117).
Match error 0.003 against 0.7–0.9 for the next-best motion. Checked (files; re-run by the lead).

So the conversion is a **re-bake at 60 fps** of KH1's own motions onto the same skeleton. No
retargeting is needed. It can be frame-for-frame.

## Getting Re:CoM's files

- The PC archives `Image/dt/Recom.hed` / `.pkg` use the same format as KH1's and KH2's (OpenKH
  `Egs`). Files are found by MD5 of their plain path, e.g. `PLAYER/0001/PL0001.ANN`. The Re:CoM
  exe carries the decryption table. Checked (files): six Sora files decoded to exactly the sizes
  the index gives.

## Re:CoM `.ANN` (motion pack)

- `u32 count`, then `count × {u32 size; char name[16]}`, then the clips back to back in slot order.
  Size 0 = empty slot. Clip index = slot number. Loader `0x14029F3A0`; the name lookup
  `0x1401A47E0` reads the 3 trailing digits as the index. Checked (decomp + files). Sora has 161
  slots.
- **Clip** (parser `0x14027ABC0`, sampler `0x14027BEC0`): `+0x10` bone count, `+0x12` frame count,
  `+0x14` loop start (1-based; 0 = plays once), `+0x16` flags (bit 1 = scale channels; bits 2–4 =
  d, fps = 60 / (d + 1)), `+0x17` an optional blend length. Then a constant-or-animated mask,
  constant floats, and curve blocks. Channels per bone: translation xyz, rotation xyz (Euler,
  degrees), optional scale xyz. Checked (decomp + files).
- **Curves:** key frames (u8, or u16 at 256+ frames), u16 values scaled into a float range,
  linear or Hermite segments. Sora's clips use only linear. Sampled at whole frames with a
  quaternion slerp in between; rotation order is the same as KH1's. Checked (decomp + files).
- **Root motion:** bone 0 `Root` carries the travel and is only applied when the motion is played
  with the root flag (`0x14024C960`). Checked (decomp). In `p001_055`, Root goes 0 → 1.0 m forward,
  KH1's 100 units. Checked (files).
- **The fps bits matter:** Re:CoM's stock Sora clips are 30 fps (d = 1). A 60 fps clip (d = 0)
  keeps every KH1 frame.

## Skeletons

- **Re:CoM `PL0001.MDL`:** 135 bones `{name[16], parent, index}`, rest pose as parent-relative
  matrices, metres, Y up. Keyblade bone `WeaponR` (49), found by name in `0x1401F4590`. Checked
  (files + decomp).
- **KH1 `xa_ex_0010.mdls`:** 293 bones in cm. The extra bones sit on other bones, are end bones, or
  are helpers. Checked (files).
- Special cases: Re:CoM's `ex_0010_sk` (hips) has an identity rotation, because KH1 bone 0's
  rotation was moved into `hip`. Re:CoM's `Root` is KH1's helper 10 (root travel). Checked (files).

## KH1 posing is constraint-driven: the hard part

- KH1 doesn't pose Sora by plain keyframes alone. Each motion has a starting-pose table (fingers,
  applied on every motion change, `0x14038F550`) and per-bone constraint flags: aim, copy
  orientation, take a helper's position, two-bone IK (`0x1403900B0`, run from `0x140391C70`). The
  hips, spine, arms and legs follow helpers. Checked (decomp + files).
- So the converter must compute KH1's **final** pose every frame, not just read curves. Two ways:
  - (a) Re-implement KH1's posing pass in the build tool, from the decomp. It runs offline from the
    player's files. Not done yet.
  - (b) Capture KH1's own bone matrices in game (as a 1:1 check of (a), or as the source).
- After that: collapse onto the 135 Re:CoM bones, write 60 fps clips with keys on every frame (or
  only drop keys where nothing is lost), and put root travel in `Root` exactly for the motions KH1
  plays with root motion (KH1's request-map bit 15).
- One KH1 tangent detail: Hermite tangents are per 1/256 frame. With that, KH1's curves match
  Re:CoM's own bake to 0.01°. Checked (files).

## Loss

At 60 fps and speed 1.0 the result is frame-for-frame. The error is u16 rounding, about 0.003° on a
360° rotation range. Between frames Re:CoM slerps while KH1 evaluates its curves. That only shows
with haste, 30 fps mode, uneven frame times and blends. KH1's IK becomes fixed motion, as on PS2.
Assumption until checked against a trace.

## What does not come with the motion

KH1 puts these in the motion; Re:CoM keeps them elsewhere:

- hit windows → Re:CoM's attack rows `0x1407C6C70` `+4`/`+5` (60 Hz frames, so the numbers carry
  over directly);
- combo window and "free to act" → Re:CoM player action code;
- sounds and voices → `PL0001.SES` (keyed by motion name);
- effects → `PL0001.EET` (`.dpx` effect programs; KH1's effects would need new effect programs);
- the Keyblade trail → not traced yet.

## Correction

`PL0001.BIN` is the player camera settings (`PCamData.bin`), not attack data. This reverses an
assumption in `recom-combat.md`.

## Open

- Hermite segment types 3/4 and which tangent belongs to which side of a key: unverified.
- How KH1 keeps root travel from moving Sora twice: not traced.
- Blends in the two games: not compared.
- Which file holds the Keyblade's own motions in Re:CoM: not found.
- How the mod's new clips and files get loaded (new slots after Re:CoM's 161; new rows in Sora's
  action table `0x1407C6300[1]`). Re:CoM's own slots must stay unchanged.
