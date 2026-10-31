# Keyblade models: KH1's Keyblades as Re:CoM weapon models

Decision 25 ("Models"): the equipped Keyblade shows KH1's model at the KH1 setting, through new
resource groups in Re:CoM's PLAYER resource table, picked when Sora's weapon is built. §§ 1–9 cover the
**asset side** (formats, conversion, output); § 10 the **DLL side** (table copy, group pick,
loading, weapon rebuild at camp close) and § 11 the install layout.

Steam addresses only (image base 0x140000000). Tags: **Checked (exe)** = bytes of
`recomsteam.exe`, **Checked (decomp)** = Ghidra C of that exe, **Checked (files)** = the owner's
extracted files, **Checked (source)** = OpenKH source, **Assumption** = not proven.

Code: `crates/kh1-formats/src/wpn.rs` (KH1 `.wpn`), `crates/kh1-recom-assets/src/{mdl,rtm,texture,keyblade}.rs`
(Re:CoM `.MDL` / `.RTM`, textures, the converter), wired into `kh1-recom-build`
(`docs/kh1-recom-assets.md`). Tests: `crates/kh1-recom-assets/tests/keyblades.rs`.

Why the Re:CoM formats live in `kh1-recom-assets`: that crate already holds Re:CoM's `.ANN` and
`.MDL` skeleton readers; `kh1-formats` is KH1's files only, so the `.wpn` reader went there.

---

## 1. Re:CoM `.MDL` (resource type 0xB)

### 1.1 File

| where | what | tag |
|---|---|---|
| +0 | u32 offsets of the **parts** (absolute), 0-terminated, padded to 16. PL0002: `0x10, 0x8730, 0`. | Checked (files) |
| loader | type 0xB (and 0x2A) → `CRsrcMDL` in the resource dispatcher `0x1402A06E0`: walks the offset list, calls `0x1402A1E40(part, 1, id)` per part, counts parts into `rsrc+0x68` (read back by `0x1402A41D0`). A part that fails (texture not yet loaded) gets bit 31 set in its offset and `rsrc+0x6A` bit 0, and is retried later (`0x1402A4BD0` → `0x1402A1CF0`). | Checked (decomp) |
| parts | each starts 16-aligned; the file ends 16-aligned. | Checked (files) |

`mdl::Mdl` reads and writes it; **PL0002.MDL, PL0011.MDL and PL0101.MDL re-encode byte for byte**
(test `a_…`). Anything not laid out as Square lays it out is refused, not re-laid silently.

### 1.2 Part header (offsets from the part start)

| off | type | PL0002 part 0 / part 1 | meaning | tag |
|---|---|---|---|---|
| +0 | u16 | 12 / 12 | bone count | Checked (files) |
| +2 | u8,u8 | 01 03 / 01 01 | byte 3: 3 textured part, 1 untextured. **Overwritten at load** (`0x1402750E0` writes 0x2342, `0x140274B00` 0x2344/0x2345); written anyway. | Checked (files, decomp) |
| +4 | u32 | 0x30 | bone name table: 20 B per bone `char name[16]; s16 parent; s16 index`, zero-padded to 16 | Checked (files) |
| +8 | u32 | 0x120 | rest matrices: 4×4 f32 per bone, row vectors, parent-relative, metres | Checked (files) |
| +0xC | u32 | 1 / 0 | texture count | Checked (decomp `0x1402A1EF0`) |
| +0x10 | u32 | 0x420 | texture table: 32 B each, `char name[16]` + 16 zero bytes the loader fills (`0x140272080`) | Checked |
| +0x14 | u32 | 0x440 / 0x420 | **list A**: DMA chain, first draw pass (`0x14026E6A0`) | Checked |
| +0x18 | u32 | 0x6DD0 / 0 | **list B**: DMA chain, second pass (`0x14026EB00`, its own callback `0x140270C20`); 0 = none | Checked |
| +0x1C | u32 | 0x8600 / 0x5330 | per-bone bounds `minX maxX minY maxY minZ maxZ` (f32, bone space); a bone without vertices has `+16 700 000 / −16 700 000` | Checked (files) |
| +0x20 | 16 B | `CC`×16 | filler | Checked (files) |

**Texture names are global.** The loader looks each name up among all loaded textures
(`0x1402A5140` → `0x1402A43E0`, resource types 2, 0x10, 0x800, compared up to the `.`); a name
not found fails the part (retry later). Checked (decomp).

### 1.3 Mesh lists: PS2 DMA chains the PC converts at load

```
DMA cnt   : 0x10000000 | (num+1), 0
VIF       : 0x01000404 (STCYCL 4,4), 0x6C000000 | num<<16 | 0x8000 (UNPACK V4-32, FLG, addr 0)
header qw0: num, 0x00010000|nv, ((0xF3 - bone_count)<<16)|nv, 0x9C000000|fmt
header qw1: 0x8000|nv, GIF word1, GIF regs, 0
nv vertices, 3 qw each (fmt 6) or 2 qw each (fmt 0x10)
VIF       : 0x17000000 (MSCNT), 3 × NOP
...
DMA ret   : 0x60000000, 0, 0, 0
```

- `num` = 2 + 3·nv (fmt 6) or 2 + 2·nv (fmt 0x10); one UNPACK, so `num` ≤ 255; Square used ≤ 0xE0
  (74 / 111 vertices). Checked (files).
- fmt 6 (textured): GIF word1 `0x303E4000` (PRIM 0x7C: strip, Gouraud, texture, fog, alpha),
  regs `0x412`. fmt 0x10 (untextured): `0x20264000` (PRIM 0x4C), regs `0x41`. The PC picks the
  converter from the first packet's format (`0x1401D0BF0`): 6 → `0x140275190`, 0x10 →
  `0x1402768A0`. Checked (files, decomp `0x140274B00`).
- Vertex: q0 = `strip flag | weight 1.0 | 0 | kick` (0xFFFF / 0xA000 on the two vertices that
  start a strip; then 0 or 0x20 / 0x2000, one triangle per vertex); q1 = `x y z (bone space, m) |
  bone<<16 | bone*4`; q2 = `s t | 1.0 | texture index`. One bone per vertex, weight 1.0, no
  normals. Checked (files).
- The PC converter keys a packet by its PRIM bits and **its first vertex's texture index**, and
  reads each vertex's bone as `(q1.w & 0xFFFF) >> 2`, up to 0x30 bones. So **one texture per
  packet**. Checked (decomp `0x140275190`).
- **Facing flag**: a triangle whose `cross(b−a, c−a)` points against the normals has flag 0
  (Re:CoM) / `0x20000` (KH1 cvbl), the other way 0x20 / `0x10000`. Checked (files, statistics on
  PL0002 and KH1 5010) / whether and how the PC culls with it: **Assumption**.

### 1.4 What the parts and lists are

- Part 0 (textured) list A: every PL0002 list-A triangle samples only opaque texels; list B:
  the triangles that touch transparent texels, drawn in the later pass. Checked (files) / list
  B's render state: **Assumption**.
- Part 1 (untextured, fmt 0x10, list A only): all of part 0's triangles again, positions only
  (PL0002: 392 = 322 + 70). Riku's PL0101 has two such pairs. Its use (shadow? silhouette?):
  **Assumption**.
- PL0002: bones `Root, ex_5010_sk, wep0, k9…k1`; texture `w_ex010_01`; 25 packets, 1296
  vertices, 784 triangles, 56 192 bytes. PL0012.MDL is byte-identical to PL0002.MDL. Checked (files).
- **The PC draws the MDL's own PS2 mesh.** Re:CoM's MDLs have no HD assets; only textures do.
  Checked (files, decomp `0x1402750E0` → `0x140274B00` → `0x140275190`).

## 2. Re:CoM `.RTM` (resource type 0xC) and TIM2

- Header: 32-byte entries `u32 data offset | char name[20] ("w_ex010_01.tm2") | u32 0x01020000 |
  u32 0x80000000|size`, one all-zero entry, then the data, unpadded. Loader
  `CRsrcPackT<CRsrcDataDD>` (`0x1402A2150`); each entry becomes its own texture resource named by
  the entry (`0x1402A2A60`, `0x1402A51B0`). Checked (decomp).
- Bit 31 = "offset-first" layout. 1395 of the 1421 RTMs in Re:CoM's resource tables use it; 26
  enemy packs (e.g. `ENEMY/0300/EN0300.RTM`) use a name-first layout (`0x1402A0600` converts it).
  `rtm::Rtm` reads and writes the first only; **PL0001/0002/0011/0012/0101.RTM re-encode byte for
  byte**. Checked (files, decomp).
- TIM2: version 4, one picture, 8-bit indexed (image type 5), 256-colour 32-bit CLUT (type 3),
  header 0x30, GsTex0 0, GsTex1 0x260, image then CLUT. Alpha 0..0x80. The CLUT is in the PS2's
  **CSM1 order** (entries 8–15 and 16–23 of every 32 swapped): decoding PL0002's picture against
  its HD texture gives a mean error of 8 with the swap and 62 without. Checked (files).
- **HD texture**: the RTM's HD asset `<texture>_tm2.dds` (PL0002.RTM: `w_ex010_01_tm2.dds`,
  1024×512 A8R8G8B8, tag `0x2F7D2`). Every RTM HD asset carries tag `0x2F7D2` (one `0x47D2`), so
  the PC can't match by offset; it matches by name (fallback in `0x1402BB960`: base name +
  `.dds`/`.png`). Checked (files) / the name derivation not fully traced: **Assumption**.
- DDS formats Re:CoM's loader takes (`0x1402D9760`): A8R8G8B8, DXT5/4, DXT3/2, DXT1, 16-bit.
  Checked (decomp).

## 3. How Re:CoM loads Sora's weapon (for the runtime step)

- Resource tables: `PTR 0x1407EBF20[category]` (categories 0–4 and 10–14 set); category 1
  (PLAYER) = **0x1408055D0** (pointer at **0x1407EBF28**, `.data`), 552 rows of 16 B `u16 group |
  u16 type | u32 0 | char *name`, ended by group 0xFFFF. Types: 0xB MDL, 0xC RTM, 0x13 TXA, 0x17
  BIN, 0x1A EET, 0x1D ANN, 0x27 LINK (`"CGGGG"`: load category C group G too), 0x2C SND, 0x30 SES.
  Checked (exe).
- Group 1 (PL0001) has LINK `"10002"`; group 2 = `PL0002.RTM, .MDL, .ANN, .EET` + LINK `"30509"`;
  group 11 links `"10012"`; group 12 = `PL0012.RTM, .MDL, .ANN, .EET`. Checked (exe).
- Loader `0x14023CD10(category, group, flags, …)`; worker `0x14023BF70` walks a group's contiguous
  rows. A file whose basename is loaded already is not reloaded, only tagged with the new id
  (`0x1402A43E0`, `0x1402A3070`); id = category·10000 + group. Checked (decomp).
- The weapon is built in `CPlayer::stateInitial` (`0x1401FA6E0`) and `CPlayerDRiku::stateInitial`
  (`0x1402068F0`): `0x140224BA0(weapon, player, char, WeaponR)`. For chars 1–2 that calls
  CPlayerWeapon vtable `0x1405AADB0` slot +0x30 (**0x1405AADE0** = `0x1401A4E70`, first bytes
  `40 56 57 41 56 48 83 EC 30`) with group `0x1405A6910[char] + 1` (2 or 12). Checked (decomp, exe).
- Halloween Town Sora (char 2) uses group 12: same MDL bytes as PL0002, another RTM. So Re:CoM
  already shows another Keyblade by giving the weapon another group. Checked (decomp, files).

## 4. KH1 `.wpn` (`kh1_formats::wpn`)

- Container: `u32 2 | u32 0x80 | u32 MENV offset | u32` (not the file size), zeros to 0x80.
  Section 0 = weapon data (OpenKH "MainFile"); section 1 = `"MENV"` with `MOBJ`'s header layout,
  model at MENV + 0x40 in all 18 files read. `mobj::Model::parse(…, false)` reads it (no
  skeleton). Checked (files).
- HD assets: the mesh is `-<model file offset, hex>.cvbl`; the texture of cvbl slot `s` is
  `-<model file offset + 0x100 + s, hex>.dds`, DXT5, no mipmaps. Checked (files: 18 of 18) / the
  rule is read off the names: **Assumption** beyond these files.
- The `.cvbl` (what KH1 PC draws): 48-byte vertices `normal xyz | flags | u v | 2 words | position
  xyz | 1.0`; flags low byte = matrix slot (→ joint through the mesh's `0x11` chunks),
  `0x10000000` = strip (re)start (draws nothing, enters the vertex queue), `0x10000` / `0x20000` =
  draws a triangle with the two vertices before it, in the two facings. Mesh flag bit 0 (vertex
  format 0x7C) = translucent mesh (Ultima Weapon, Diamond Dust). Checked (files) / the flag
  meanings beyond what the converted triangles show: **Assumption**.

## 5. Conversion rules (`keyblade::convert`)

| step | rule | tag |
|---|---|---|
| mesh | the `.cvbl`, not the PS2 packets: it is what KH1 PC draws and its UVs belong to the HD textures (the PS2 UVs belong to the PS2 textures). | Checked (files) |
| bones | KH1 joint j (2–11) = Re:CoM bone j (`wep0, k9 … k1`); any other joint is refused, no remap. | Checked (files: all 17) |
| units | position × 0.01, same axes, bone space. | Checked (files: §7) |
| skeleton | PL0002's 12 bones, rest matrices, part flags, filler and GIF words, copied from Re:CoM's own PL0002.MDL (read at build time). So PL0002.ANN drives the new model unchanged. | Checked (files) |
| part 0 list A / B | B = triangles touching a texel with alpha < 255 (texel centres inside the triangle and its corners, on the HD texture), plus all triangles of a translucent cvbl mesh; A = the rest. On PL0002 this gives Square's split for 322 of 322 list-A and 67 of 70 list-B triangles. | Checked (files, test `b_…`) |
| strips | consecutive triangles of one cvbl strip in the same list stay a strip; ≤ 0xE0 qw per packet (as Square), one texture per packet; a long strip continues in the next packet from its last two vertices. | Checked (tests) |
| facing | cvbl `0x20000` → 0, `0x10000` → 0x20. | Checked (files) / PC use: Assumption |
| part 1 | every triangle again, fmt 0x10, list A only, as PL0002. | follows PL0002 |
| bounds | per bone min/max of the part's vertices; ±16 700 000 for empty bones, as Square's. | Checked (test `c_…`) |
| UVs | s = u, t = v, q = 1. PL0002's own UVs use this convention against its HD texture (all list-A vertices on opaque texels). | Checked (files) |
| textures | one per cvbl texture slot used, in slot order, named `<KH1 model>_<n>` (e.g. `xw_ex_5030_0`). None of the 30 names is among the 1649 texture names of Re:CoM's 1421 RTMs, and Re:CoM has no name starting `xw_`. | Checked (files: the owner's Recom.pkg, all RTMs the resource tables list) |
| TIM2 | the HD texture halved (2×2 box) until both sides ≤ 256, alpha → PS2 (255 → 0x80), 256 colours by median cut (deterministic), nearest-colour mapping, CSM1 CLUT. | — |
| HD | KH1's DXT5 `.dds` unchanged, named `<texture>_tm2.dds`, tag `0x2F7D2` as Square's. | Re:CoM loads DXT5: Checked (decomp) / tag use: Assumption |

## 6. Resource groups

Re:CoM's PLAYER table uses groups 1, 2, 11–14, 101, 111, 201, 211, 301–307, 401–409, 451, 452,
458, then 1001 and up (Checked (exe), all 552 rows), and the archive has no `PLAYER/00GG/PL00GG.*`
file for G = 3–10 or 15–100 (Checked (files): MD5 of every name with the 8 PLAYER extensions
against the owner's `Recom.hed`). So **21–36 are free**.

| card | Keyblade | KH1 model | group | textures | MDL bytes | RTM bytes |
|---|---|---|---|---|---|---|
| 1 | Kingdom Key | xw_ex_5010 | **2 (Re:CoM's own PL0002)** | `w_ex010_01` | — | — |
| 2 | Three Wishes | xw_ex_5030 | 21 | 2 | 63 792 | 133 344 |
| 3 | Crabclaw | xw_ex_5060 | 22 | 2 | 63 168 | 133 344 |
| 4 | Pumpkinhead | xw_ex_5050 | 23 | 2 | 69 952 | 133 344 |
| 5 | Fairy Harp | xw_ex_5040 | 24 | 2 | 65 600 | 133 344 |
| 6 | Wishing Star | xw_ex_5150 | 25 | 2 | 73 632 | 133 344 |
| 7 | Spellbinder | xw_ex_5080 | 26 | 2 | 60 128 | 133 344 |
| 8 | Metal Chocobo | xw_ex_5110 | 27 | 2 | 53 664 | 133 344 |
| 9 | Olympia | xw_ex_5090 | 28 | 2 | 59 936 | 133 344 |
| 10 | Lionheart | xw_ex_5100 | 29 | 2 | 62 304 | 133 344 |
| 11 | Lady Luck | xw_ex_5140 | 30 | 2 | 59 776 | 133 344 |
| 12 | Divine Rose | xw_ex_5070 | 31 | 2 | 63 808 | 133 344 |
| 13 | Oathkeeper | xw_ex_5120 | 32 | 1 (1024×512) | 79 712 | 33 920 |
| 14 | Oblivion | xw_ex_5130 | 33 | 1 (1024×512) | 75 456 | 33 920 |
| 15 | Ultima Weapon | xw_ex_5160 | 34 | 2 | 85 920 | 133 344 |
| 16 | Diamond Dust | xw_ex_5190 | 35 | 2 | 76 192 | 133 344 |
| 17 | One-Winged Angel | xw_ex_5200 | 36 | 2 | 76 992 | 133 344 |

`group = 19 + card` for cards 2–17 (`keyblade::group_for_card`). The model names come from KH1's
weapon table (`btltbl.bin`, items of `stats::KH1_KEYBLADES`); the step writes `group` and
`textures` into each `kh1-stats.json` `keyblades` row (format `stats 4`).

**The Kingdom Key keeps Re:CoM's PL0002** (as asked). Found while checking it: KH1 PC's own
Kingdom Key is not quite Re:CoM's. Its cvbl re-tessellates the flat cut-out parts on `k9` and `k1`
(same plane, outline within 0.21 mm, vertices up to 12 mm apart), and Re:CoM's texture is its own
re-laid 1024×512 one. Converting 5010 too (a 17th group) would make the Kingdom Key match KH1 PC as
the other 16 do. That is a choice for the owner.

## 7. Validation (test `crates/kh1-recom-assets/tests/keyblades.rs`; run on the owner's PC too)

- **Kingdom Key (5010) vs PL0002** (positions, per bone, nearest vertex both ways):
  - KH1's PS2 mesh × 0.01 on PL0002: ≤ 0.08 mm on every bone 2–11. This proves the transform
    (joint = bone, × 0.01, same axes).
  - The cvbl equals the PS2 mesh exactly (0.0 mm) on bones 2 and 4–10, so there **ours lands on
    PL0002's vertices within 0.08 mm**.
  - On `k9` (3) and `k1` (11) KH1 PC's cvbl has its own vertices (7.3 / 12.4 mm from the PS2
    ones): flat (z = 0), bounds within 1 mm of PL0002's. The test accepts exactly those two bones.
  - Checked (files).
- **All 17**: every cvbl vertex is on joints 2–11 (none refused). The PS2 meshes also use only
  2–11. UVs are within 0.003…0.997. 100% of list-A vertices land on opaque texels of our own TIM2.
  Every MDL re-reads and re-writes byte for byte. Part 1 has every triangle. Texture names are
  unique. The raw RTM entry reads back by Panacea's rules. Checked (files).
- Triangles spanning two bones (rigid per vertex, as KH1 PC draws them): Oathkeeper 24, One-Winged
  Angel 40, none elsewhere. Checked (files).
- Owner's PC: `kh1-recom-build --game . --out kh1-playground/build-out/PL0001.ANN` converted all 16
  in 0.7 s. Checked (run).

| Keyblade | cvbl meshes | triangles | list A | list B (translucent mesh) |
|---|---|---|---|---|
| Kingdom Key 5010 (test only) | 1 | 426 | 358 | 68 |
| Three Wishes 5030 | 2 | 419 | 326 | 93 |
| Crabclaw 5060 | 2 | 420 | 370 | 50 |
| Pumpkinhead 5050 | 2 | 446 | 370 | 76 |
| Fairy Harp 5040 | 2 | 432 | 378 | 54 |
| Wishing Star 5150 | 2 | 454 | 323 | 131 |
| Spellbinder 5080 | 2 | 424 | 353 | 71 |
| Metal Chocobo 5110 | 2 | 376 | 296 | 80 |
| Olympia 5090 | 2 | 430 | 386 | 44 |
| Lionheart 5100 | 2 | 398 | 302 | 96 |
| Lady Luck 5140 | 2 | 412 | 364 | 48 |
| Divine Rose 5070 | 2 | 420 | 326 | 94 |
| Oathkeeper 5120 | 1 | 478 | 221 | 257 |
| Oblivion 5130 | 1 | 460 | 262 | 198 |
| Ultima Weapon 5160 | 3 | 564 | 199 | 365 (44) |
| Diamond Dust 5190 | 3 | 488 | 235 | 253 (54) |
| One-Winged Angel 5200 | 2 | 516 | 435 | 81 |

Test data: `KH1_TEST_DATA/keyblades/` holds the files as the extractor writes them
(`PLAYER_0002_PL0002.{MDL,RTM}` + `PLAYER_0002_PL0002.RTM__w_ex010_01_tm2.dds`, optionally
PL0011/PL0101/PL0001/PL0012 for the round trips; per Keyblade `xw_ex_5NN0.wpn`, its
`.assets.txt` (`name<TAB>tag<TAB>size`) and the assets as `xw_ex_5NN0.wpn__<name without '-'>`),
plus `KH1_TEST_DATA/btltbl.bin`. The tests skip when it is missing.

## 8. Output, and Panacea's HD route

Per converted Keyblade, under Panacea's `<mod_path>/recom/` (or next to `--out`):

- `PLAYER/00GG/PL00GG.MDL`: a loose file (Panacea's `TransformFilePath` serves
  `<mod_path>/recom/<archive path>`). MDLs have no HD assets.
- `raw/PLAYER/00GG/PL00GG.RTM`: a whole plain PKG entry (`egs::build_plain_entry`: header, one
  0x30-byte asset header per HD texture, the RTM, the `<texture>_tm2.dds` files).

Why the RTM goes through `raw/`. Checked (source: OpenKH `OpenKh.Research.Panacea/Panacea.cpp`,
commit b67a7434, 2026-10-22):

- New names are known to the game. `FakePackageFile` ("ModFiles", added in `SetReplacePath`)
  scans `m_ModPath` and `m_ModPath\raw` at start-up. `GetFileInfo` finds new file names through it,
  with a zero remastered count.
- **The loose route gives a new name no HD assets.** `LoadFile` → `GetRemasteredFiles` starts from
  the PKG entry's assets (none for a new name). `ScanRemasteredFolder` for Re:CoM (the `default`
  case) builds `assetoffs` from those entries, then `entries.resize(assetoffs.size())`. So
  `<mod>/recom/remastered/PLAYER/00GG/PL00GG.RTM/*.dds` would be cut to zero. (This is also why
  PL0002.RTM's loose route keeps only one HD texture.)
- **The raw route carries them.** `GetRawFile` is checked first in `LoadFile` /
  `LoadFileWithMalloc` / `GetFileSize`. With `useHdAsset` it reads the asset headers from the
  file into `RemasteredData[CurrentFileName]` and recomputes their offsets.
  `GetRemasteredCount` / `GetRemasteredEntry` / `GetRemasteredAsset` then serve them.
- In game: **Assumption** until seen. If the HD textures don't show, the TIM2 inside the RTM is
  what Re:CoM falls back to (a 256-colour, ≤ 256-pixel version of the same texture).

The step writes only into the folder it is given. It installs nothing else, and it does not touch
the resource table (the DLL's job).

## 9. Not verified in game (asset side; the full test list is § 10.8)

1. **Loading**: with the DLL's table rows for group G, `PL00GG.MDL` (loose) and the raw
   `PL00GG.RTM` load, and the part loads don't stay in the retry state (texture names found).
2. **HD textures through the raw route**: the Keyblade shows KH1 PC's texture, not the 256-colour
   TIM2. Panacea's debug log (`GetRemasteredEntry(0) = "xw_ex_…_tm2.dds"`) shows it.
3. **Placement and motion**: the blade sits in Sora's hand like Re:CoM's Kingdom Key and follows
   PL0002.ANN (`k9…k1` swing). Units, axes and bones are Checked (files) for the Kingdom Key.
4. **Facing / culling and list B**: cut-out parts (keychains, crowns, Ultima's crystal) draw with
   transparent edges and from both sides.
5. Part 1's role (untextured copy) and whether it shows (e.g. as a shadow).
6. Trail length: CPlayerWeapon `+0x2C0` takes its trail from the model (`0x140235270`,
   `model+0x158`). Longer Keyblades may keep the Kingdom Key's trail. Not traced. KH1's own trail
   data is in `.wpn` section 0.
7. Memory: only the equipped group is loaded (decision 25); each RTM entry is 0.13–0.66 MB.

## 10. Runtime (DLL): `crates/kh1-recom/src/keyblade_models.rs` (rules, tested) and `src/win/models.rs`

At `combat = kh1`, in Sora's story, Sora's weapon shows the equipped Keyblade (card 1 = Re:CoM's
group 2, cards 2–17 = groups 21–36 from `kh1-stats.json` format 4, `group` per row). At
`combat = recom` or in Riku's story every path below hands Re:CoM its own values.

### 10.1 What Re:CoM does (the facts the runtime stands on)

| fact | address | tag |
|---|---|---|
| Only one piece of code reads the resource tables: the loader worker, `lea r14,[0x1407EBF20]` at `0x14023BF9D`, per request `[r14 + category·8]`; the only pointer to the PLAYER table is `0x1407EBF28`. | `0x14023BF70` | Checked (exe: every rip reference and every qword equal to `0x1408055D0`) |
| Loads are queued (`0x14023CD10`; the player's groups through `0x14023CCE0(group)` = `(1, group, 0x100, 0, 0)`) and run on the "RCF thread", one request at a time, started and popped by the main loop's pump `0x14023CB70`. `0x14023CA80(c, g)` = 0 while (c, g) is queued; `(0, 0)` = 1 when nothing is queued or running. Room and battle builders wait with `0x14023CA80(0, 0)` before building the player. | `0x14023CD10`, `0x14023CB70`, `0x14023CA80`, `0x1401AD040` | Checked (decomp) |
| `0x140211AA0(char)` stores the char in `0x1407C7AE0` and queues group `0x1405A6910[char]` when its RTM (type 0xC, id 10000 + group) is not loaded. Called by the room builder `0x1401AE7E0`, the battle builder `0x1401AF920` and script command 0x1C (`0x140145C50`). | `0x140211AA0` | Checked (decomp) |
| Resource ids: a new file gets primary id `+0x2C` = its group's id and one tag = the link root's id (or its own) (`0x1402A1BC0`); a file whose name is already loaded only gets the new id as a tag (`0x14023B9D0` → `0x1402A3070`). Lookups by id with `r8 = 1` check the primary then the tags (`0x1402A3E20`). `0x1402A5B00(id)` removes the tag and deletes a file left with no tag (or whose primary is the id), unless a pack still holds it (`+0x42`). | `0x1402A1BC0`, `0x1402A3E20`, `0x1402A5B00` | Checked (decomp) |
| No file name is in two groups of any vanilla table. | all 10 tables | Checked (exe) |
| CPlayerWeapon is built only in `stateInitial` (`0x1401FA6E0`, `0x1401F4590` for Dark Riku): `new(0x300)`, `0x140224BA0(weapon, player, player+0x398 char, player+0x3A0 WeaponR)`, stored at `player+0x3D0`, then the player's vtable `+0x100` (`0x1401F5480`: put the Keyblade away). For chars 1–2 the ctor calls vtable `+0x30` (`0x1405AADE0` → `0x1401A4E70`) with `(weapon, 1, 0x1405A6910[char] + 1, 0)` = group 2 or 12; the owner (`weapon+0x230`) is set before. | `0x140224BA0` | Checked (decomp + exe) |
| The weapon is a CTask child of the player (`0x1402AB850` → `0x1402ABBA0`). Re:CoM destroys it only through the player's CTask dtor (`0x1402AB950`), which calls each child's virtual dtor with 1; the child's own CTask dtor unlinks it from the parent's list. CPlayerWeapon's dtor `0x140224EC0` (vtable slot 0) destroys its CCollAttack (`weapon+0x240`, which leaves the collision lists in `0x140172740`) and its CEntity part (model `+0xF0`, parts, `0x1401A2440`). Effects are attached by the entity's id (`0x1401A2D50` passes `entity+0x150`), not by pointer. No vanilla code deletes a weapon on its own. | `0x1402AB950`, `0x140224EC0`, `0x140172740`, `0x1401A2D50` | Checked (decomp) |
| Camp close: the camp's state 0x50 tears it down (`0x1400D7C20`) and kills its task; the field's state (`0x1401AAFB0`, `0x1401ABE00`) sees the camp's task id no longer resolve and calls `0x1401A9590(field, 0)`, which un-pauses the field's tasks (0xF, 1, 0xB). That is the first point after the camp is gone; the player is not mid-update there. A camp left for another scene (title, game over) does not reach it. | `0x1401A9590` | Checked (decomp) |
| CPlayerSora serves chars 1 and 2 (factory `0x14020EC70`). | | Checked (decomp) |
| Re:CoM's 100 Acre Wood "Veggie Panic" Keyblade (`CMini100AcreVegPanicKeyBrade`, ctor `0x140438450`) builds its model from group 2 (`0x1401A4E60(obj, 1, 2)`), relying on group 2 being loaded with the player. | `0x140438450`, `0x14043DF10` | Checked (decomp) |
| A weapon is built once per player object; the player is made per room / battle scene (`0x1401AEBF0` and the other builders create a CPlayerManager and the player). | | Checked (decomp) / "every room and every battle": **Assumption** |

### 10.2 Rows (data Re:CoM's loader reads)

- At start the DLL copies Re:CoM's 552 rows, appends one group per converted Keyblade laid out
  as group 2 (`RTM, MDL, ANN, EET, LINK`), with its own `PLAYER/00GG/PL00GG.RTM` / `.MDL` paths and
  group 2's own name pointers for `PLAYER/0002/PL0002.ANN`, `PLAYER/0002/PL0002.EET` and LINK
  `"30509"` (EF0509.EFF, the effect pack group 2 links), then Re:CoM's end row. Re:CoM's rows keep
  their order and indices. Group 2 must read exactly so (types and names), and no new group may be
  one of Re:CoM's; else nothing is installed. Tested (`plan_table`).
- Installed with one checked pointer write at `0x1407EBF28` (old value `base + 0x8055D0`;
  the exe is relocated, so pointer checks are `base + RVA`, `sites::MODEL_PTRS`).
- **Shared ANN / EET and their id.** Our groups name Re:CoM's `PL0002.ANN` (the mod's extended one
  when the asset step wrote it) and `PL0002.EET`. A shared file keeps the primary id of the group
  that loaded it first; Re:CoM's own group-2 lookups (its Keyblade at the Re:CoM setting, the
  Veggie Panic Keyblade) need it to be 10002. So whenever the DLL queues one of our groups it
  first queues group 2 if group 2's RTM is not loaded (the queue is FIFO; a group's LINK
  sub-requests go right after it). Checked (decomp) / never seen in game: **Assumption** that this
  ordering always holds. The other native layout (own `PL00GG.ANN` / `.EET` copies per group, as
  Re:CoM's Halloween group 12 has its own `PL0012.ANN` / `.EET`) needs 16 copies from the asset
  step; not done.

### 10.3 Which group is built (the pick)

`0x1405AADE0` (CPlayerWeapon vtable `+0x30`, `.rdata`) now holds `weapon_model`, which calls
Re:CoM's `0x1401A4E70` with:

| case | group |
|---|---|
| category ≠ 1, group not 2 / 12, owner not CPlayerSora | Re:CoM's (unchanged) |
| `combat = recom`, or Riku's story (`0x1401AAAE0`) | Re:CoM's 2 / 12 (Halloween too) |
| KH1, Sora's story, Kingdom Key equipped | 2, in every world (Halloween Town too, owner) |
| KH1, Sora's story, card 2–17 | its group 21–36 |
| KH1, a card without a converted model, or a format 1–3 stats file | Re:CoM's (logged once) |
| the wanted group's RTM / MDL / ANN not found, or still queued | Re:CoM's this time; the group is queued (logged once per group) |

The slot is CPlayerWeapon's own (the shared function `0x1401A4E70` is not hooked; other classes
keep calling it directly through their own vtables). Checked (exe: the slot holds
`base + 0x1A4E70`).

### 10.4 Loading only the equipped one

- `0x140211AA0` is wrapped (11 bytes moved, `40 53 48 83 EC 20 48 89 7C 24 30`): after Re:CoM's
  own work, at KH1 in Sora's story, the wanted group (when it is not Re:CoM's own 2 / 12 for this
  char) is queued with `0x14023CCE0` if its RTM is not loaded and it is not queued. The builder's
  wait then covers it, so a save loaded with Three Wishes equipped builds Three Wishes at once.
- The equip confirm on the Keyblades page and the Combat switch in Config queue it too
  (`models::prefetch`), so it is in when the camp closes.
- Unload: `0x1402A5B00(10000 + g)` for our groups only, at camp close, when the weapon's group is
  known, the group is neither the weapon's nor the wanted one, and the loader is idle
  (`0x14023CA80(0, 0)`; the unload walks lists the loader thread fills). Shared files lose only
  our tag (they keep group 2's or the player's).
- **Not followed literally**: Re:CoM's group 2 (Kingdom Key) stays loaded as Re:CoM loads it
  (LINK `"10002"` of the player's group 1), because Re:CoM's Veggie Panic Keyblade builds from it
  and the shared ANN / EET need it (10.2). So at most our one equipped group + Re:CoM's own group
  2 are in memory; the other 15 never load. In Halloween Town with the Kingdom Key at KH1 the
  DLL queues group 2 (vanilla Halloween uses 12).

### 10.5 Rebuild at camp close

`0x1401A9590` is wrapped (15 bytes moved, `40 53 48 83 EC 30 48 C7 44 24 20 FE FF FF FF`). After
Re:CoM's close work (`open = 0`), when the main player is CPlayerSora (char 1 or 2) and
`player+0x3D0` is a CPlayerWeapon (its vtable):

| the weapon's group (seen by the slot) vs wanted | action |
|---|---|
| same | nothing |
| differs, files in, Sora not in Re:CoM's attack action 0xA | rebuild now |
| differs, files not in yet | queue; shows from Re:CoM's next weapon build (next room or battle) or the next camp close |
| not built through our slot, or attacking | nothing (logged) |

The rebuild is `stateInitial`'s own sequence: `operator new(0x300)`, `0x140224BA0(new, player,
player+0x398, player+0x3A0)` (our slot picks the group), then the old weapon's virtual dtor with 1
(Re:CoM's child-destroy path, 10.1), `player+0x3D0 = new`, the player's vtable `+0x100` (Keyblade
put away, as after `stateInitial`). The new one is built first, so `player+0x3D0` never holds a
dead or null weapon. It runs once per camp close, only when the group differs. Checked (decomp:
every step is a Re:CoM call in Re:CoM's order) / in game: **Assumption**.

Not handled as Re:CoM would at a room change: the new weapon is appended at the end of the
player's child list (built first in `stateInitial`, so updated before the player's later children
there). Assumption: the order of the player's children does not matter for the weapon (its bone 0
follows the player's WeaponR matrix pointer).

### 10.6 The KH1 blade test on the new models

`win/attack.rs` reads the weapon's bones 2 and 3 through `0x1402ABB20` (bone index × 0xE0 into the
actor's matrices, which come from the MDL's skeleton). Every converted MDL carries PL0002's 12
bones, names, parents and rest matrices byte for byte, bone 2 = `wep0`, bone 3 = `k9`. Checked
(files: test `d_all_seventeen_convert`, all 17). The blade length comes from the equipped
Keyblade's KH1 row, as before.

### 10.7 Addresses (Steam; first bytes checked at start, pointers at install)

| what | address | bytes / value |
|---|---|---|
| hook: player group load | `0x140211AA0` | `40 53 48 83 EC 20 48 89 7C 24 30` |
| hook: field camp open/close | `0x1401A9590` | `40 53 48 83 EC 30 48 C7 44 24 20 FE FF FF FF` |
| callee: queue player group | `0x14023CCE0` | `48 83 EC 38 45 33 C9 C7 44 24 20 00 00 00 00 8B D1 41 B8 00 01 00 00` |
| callee: load queued? | `0x14023CA80` | `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20 8B 05 DF 01 64 00` |
| callee: resource by type and id | `0x1402A3E20` | `40 53 56 57 41 54 41 55 48 83 EC 20 45 8B E0 8B DA` |
| callee: unload by id | `0x1402A5B00` | `85 C9 0F 8E 5D 02 00 00 48 8B C4 41 57 48 83 EC 50` |
| callee: CPlayerWeapon ctor | `0x140224BA0` | `48 8B C4 48 89 48 08 56 57 41 54 41 56 41 57 48 83 EC 40` |
| callee: model init (slot +0x30) | `0x1401A4E70` | `40 56 57 41 56 48 83 EC 30 48 C7 44 24 20 FE FF FF FF` |
| via vtable: CPlayerWeapon dtor | `0x140224EC0` | `48 89 5C 24 08 57 48 83 EC 20 48 8D 05 DF 5E 38 00` |
| via vtable: put Keyblade away | `0x1401F5480` | `40 53 48 83 EC 20 48 8B D9 48 8B 89 D0 03 00 00 48 85 C9` |
| patch: CPlayerWeapon vtable +0x30 | `0x1405AADE0` | `base + 0x1A4E70` → `weapon_model` |
| check: CPlayerWeapon vtable +0 | `0x1405AADB0` | `base + 0x224EC0` |
| check: CPlayerSora vtable +0x100 | `0x1405A9A38` | `base + 0x1F5480` |
| patch: PLAYER table pointer | `0x1407EBF28` | `base + 0x8055D0` → our copy |
| read: char of the player group | `0x1407C7AE0` | (data) |

All function bytes Checked (exe: the 112 sites of `sites.rs` re-read from `recomsteam.exe`);
all four pointers Checked (exe, relocated entries). All or nothing: the stats groups, the four
pointer checks and the table plan are checked first; then both hooks (all or none); then the
table pointer; then the slot. The wrappers act only after the last write.

Logs: `keyblades: models in: …`, `keyblades: weapon group N built for card K` (when the built
group changes), `keyblades: Sora's weapon rebuilt at camp close with group N`, `group N queued`,
`group N unloaded`, `group N not loaded when the weapon was built; Re:CoM's group M this time`,
`camp closed; group N still loading …`, and one line for each refusal.

Per frame: nothing. The wrappers run per room / battle build, per weapon build, per camp close.

### 10.8 In-game test, in priority order

1. Log: `keyblades: models in: PLAYER table 633 rows (553 Re:CoM's + 80 for groups [21..36])`.
   No `no model swap` line.
2. KH1, Sora's story, equip Three Wishes, close the camp: the Keyblade changes (draw it with an
   attack); log `rebuilt at camp close with group 21`. No crash on the close or on the next swing.
3. The blade test and damage with it (hits land along the new blade; Strength from its KH1 row).
4. Room change and a battle: still Three Wishes (`weapon group 21 built for card 0x2` once).
5. Save, quit, load: Three Wishes from the first room (no fallback line).
6. Back to the Kingdom Key: rebuilt with group 2; `group 21 unloaded`.
7. Combat = Re:CoM in Config, close: Re:CoM's Kingdom Key; back to KH1: the equipped one.
8. Halloween Town at KH1: the equipped Keyblade (Kingdom Key = Re:CoM's normal one, not the
   Halloween one); at Re:CoM: the Halloween one.
9. Riku's story: unchanged.
10. 100 Acre Wood Veggie Panic after switching Keyblades: its Keyblade still shows.
11. Textures: KH1 PC's HD textures (not the 256-colour TIM2); translucent parts (Ultima's
    crystal), both sides of flat parts; the trail length (§ 9.6).
12. Open and close the camp without equipping: no rebuild line.

## 11. Install layout (Panacea, OpenKH mod folder)

The asset step writes, under `<mod_path>/recom/` (or next to `--out`):

| file | where Panacea must find it | how |
|---|---|---|
| `PLAYER/00GG/PL00GG.MDL` (GG = 21..36) | `<mod_path>/recom/PLAYER/00GG/PL00GG.MDL` | loose file |
| `raw/PLAYER/00GG/PL00GG.RTM` | `<mod_path>/recom/raw/PLAYER/00GG/PL00GG.RTM` | a whole plain PKG entry with its HD `.dds` (§ 8) |
| `kh1-stats.json` (format `stats 4`, with `group` / `textures`) | `<mod_path>/recom/kh1-stats.json` | as before |
| `PLAYER/0002/PL0002.ANN` (extended) | as before | as before |

For the local OpenKH mod (`mods/Recom/PM2CRAZY/KH1-in-ReCom/`, whose `mod.yml` lists each file
with `method: copy`), put the 32 files in the mod folder under the same relative paths and add:

```yaml
- {name: PLAYER/0021/PL0021.MDL, method: copy, source: [{name: PLAYER/0021/PL0021.MDL}]}
- {name: raw/PLAYER/0021/PL0021.RTM, method: copy, source: [{name: raw/PLAYER/0021/PL0021.RTM}]}
# … the same two lines for 0022, 0023, …, 0036 (16 groups, 32 entries)
```

Assumption: OpenKH's Mods Manager copies a `raw/…` asset name to `<mod_path>/recom/raw/…`
unchanged (check the built folder once). Panacea finds new names under `<mod_path>/recom` and
`<mod_path>/recom/raw` at start-up (§ 8, Checked source), so the game must be restarted after
installing.

## In game, 0.9.0 (owner, 2026-10-31)

- Owner: "perfect, everything is great". Checked (owner).
- Log (Checked): the PLAYER table had 633 rows. Each equip (Three Wishes, Crabclaw,
  Pumpkinhead, Diamond Dust, One-Winged Angel) queued its group, the weapon was rebuilt at camp
  close with that group, and the previous group was unloaded. No fallback lines.
