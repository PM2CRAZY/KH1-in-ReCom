# Keyblade models: KH1's Keyblades as Re:CoM weapon models

Decision 25 ("Models"): the equipped Keyblade shows KH1's model at the KH1 setting, through new
resource groups in Re:CoM's PLAYER resource table, picked when Sora's weapon is built. §§ 1–9 cover the
**asset side** (formats, conversion, output); § 10 the **DLL side** (table copy, group pick,
loading, weapon rebuild at camp close) and § 11 the install layout. § 12 covers the six
Keyblades that come from **KH2** (step 3: models, Strength and Magic, reach).

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
| 0x12 | Soul Eater | — (Riku's, not for Sora) | none (37 unused) | — | — | — |
| 0x13 | Star Seeker | KH2 `W_EX010_30` | 38 | 1 (512×256) | 69 056 | 33 920 |
| 0x14 | Monochrome | KH2 `W_EX010_60` | 39 | 1 (512×256) | 74 176 | 33 920 |
| 0x15 | Follow the Wind | KH2 `W_EX010_70` | 40 | 1 (512×256) | 71 808 | 33 920 |
| 0x16 | Hidden Dragon | KH2 `W_EX010_40` | 41 | 1 (512×256) | 72 800 | 33 920 |
| 0x17 | Two Become One | KH2 `W_EX010_P0` | 42 | 1 (512×256) | see build log | 33 920 |
| 0x18 | Bond of Flame | KH2 `W_EX010_K0` | 43 | 1 (512×256) | 74 464 | 33 920 |

`group = 19 + card` for cards 2–17 and 0x13–0x18 (`keyblade::group_for_card`; the DLL's
`keyblade_models::our_group_for_card` requires exactly that). 38–43 were re-checked for step 3:
no row of the PLAYER table (552 rows, pointer at `0x1407EBF28`) has a group in 30..50, and
`Recom.hed` (12 689 entries) has no `PLAYER/00GG/PL00GG.*` for G = 15..100 with any of the 11
extensions the table uses (ANN ASD BIN CAP EET MDL RSD RTM SES SND TXA). Checked (exe + files).
The KH1 model names come from KH1's weapon table (`btltbl.bin`, items of `stats::KH1_KEYBLADES`),
the KH2 ones from KH2's tables (§ 12); the step writes `group` and `textures` into each
`kh1-stats.json` `keyblades` row (format `stats 4`).

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
plus `KH1_TEST_DATA/btltbl.bin`. The tests skip when it is missing. The KH2 Keyblades' tests
(§ 12.6) read `KH1_TEST_DATA/kh2/` (`03system.bin`, `00objentry.bin`, `msg_us_sys.bar`, and per
Keyblade `obj_W_EX010*.mdlx`, `.assets.txt`, `__0.dds`).

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
group 2, cards 2–17 = groups 21–36 and cards 0x13–0x18 = groups 38–43, from `kh1-stats.json`
format 4, `group` per row; nothing in the DLL counts them). At
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
| KH1, Sora's story, card 2–17 or 0x13–0x18 | its group 21–36 / 38–43 |
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
(files: test `d_all_seventeen_convert`, all 17; `c_all_six_convert` for the KH2 ones). The
blade length comes from the equipped Keyblade's row, as before (KH2 Keyblades: the Kingdom Key's
KH1 row with `blade_far` scaled by the KH2 blade, § 12.4).

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

1. Log: `keyblades: models in: PLAYER table 633 rows (553 Re:CoM's + 80 for groups [21..36])`
   (with the KH2 Keyblades, step 3: `663 rows (553 Re:CoM's + 110 for groups [21, …, 36, 38,
   …, 43])`). No `no model swap` line.
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
| `PLAYER/00GG/PL00GG.MDL` (GG = 21..36, 38..43) | `<mod_path>/recom/PLAYER/00GG/PL00GG.MDL` | loose file |
| `raw/PLAYER/00GG/PL00GG.RTM` | `<mod_path>/recom/raw/PLAYER/00GG/PL00GG.RTM` | a whole plain PKG entry with its HD `.dds` (§ 8) |
| `kh1-stats.json` (format `stats 4`, with `group` / `textures`) | `<mod_path>/recom/kh1-stats.json` | as before |
| `PLAYER/0002/PL0002.ANN` (extended) | as before | as before |

For the local OpenKH mod (`mods/Recom/PM2CRAZY/KH1-in-ReCom/`, whose `mod.yml` lists each file
with `method: copy`), put the files in the mod folder under the same relative paths and add:

```yaml
- {name: PLAYER/0021/PL0021.MDL, method: copy, source: [{name: PLAYER/0021/PL0021.MDL}]}
- {name: raw/PLAYER/0021/PL0021.RTM, method: copy, source: [{name: raw/PLAYER/0021/PL0021.RTM}]}
# … the same two lines for 0022, 0023, …, 0036 (16 groups, 32 entries)
# step 3 (KH2 Keyblades): the same two lines for 0038, 0039, …, 0043 (6 groups, 12 entries;
# there is no 0037)
```

The new `kh1-stats.json` (with the six KH2 rows) replaces the old one; without it the DLL never
asks for groups 38–43 and the six are not listed.

Assumption: OpenKH's Mods Manager copies a `raw/…` asset name to `<mod_path>/recom/raw/…`
unchanged (check the built folder once). Panacea finds new names under `<mod_path>/recom` and
`<mod_path>/recom/raw` at start-up (§ 8, Checked source), so the game must be restarted after
installing.

## 12. The six KH2 Keyblades (step 3)

Decision 25: Star Seeker (card 0x13), Monochrome (0x14), Follow the Wind (0x15), Hidden Dragon
(0x16), Photon Debugger (0x17) and Bond of Flame (0x18) come from KH2 FM, read from the player's
own `kh2_*` archives: KH2's model, KH2's Strength and Magic; reach measured from the KH2 model's
blade; radius, critical rate / bonus and priority are the Kingdom Key's (KH1 row). Keyblade
abilities are not included. Soul Eater (0x12) is Riku's and gets nothing.

Code: `crates/kh2-formats-lite` (KH2 readers, ported from KH2-Combat-in-KH1's `kh2-formats`,
Apache-2.0, after OpenKH; licence and changes in its `lib.rs`), `kh1-recom-assets`
`kh2_keyblades.rs` (items, stats, models, blade length, reach) and `keyblade::convert_kh2`,
`check_kh2_skeleton`, `kh2_template_distance`, `kh2_facing`. The archives are read with
`kh1-formats`' `egs::Install::with_archives(…, KH2_ARCHIVES)`. Tests:
`crates/kh1-recom-assets/tests/kh2_keyblades.rs` (`KH1_TEST_DATA/kh2/`, skipped without it),
`keyblade_models::tests::kh2_*` in `kh1-recom`.

Tags as above; **Checked (decomp)** here also means the KH2 FM Steam decomp
(`decomps\steam\kh2\`, exe SHA-256 `9002b2de…49ed`).

### 12.1 Where the numbers are (KH2 FM Steam)

| what | where | tag |
|---|---|---|
| Archives | `Image/dt/kh2_first..sixth.hed/.pkg`, same layout and key table as KH1's / Re:CoM's | Checked (files) |
| Names | `msg/us/sys.bar` (US English; `msg/jp/sys.bar` is not in the owner's Steam archives). An item's name is message `item +8` | Checked (files) |
| Items | `03system.bin` BAR entry `item`: `u32 version, u32 count` (535), 24-byte rows from `+8`. Found by id: `0x1403E39A0` walks the rows comparing the u16 at `+0`. `+2` type (2 = Keyblade), `+4` weapon number, `+8` name | Checked (decomp + files) |
| Stat id | for item types 2..0xD the u16 at `+6` (`0x1403E3EF0` → `0x14041E8F0`) | Checked (decomp) |
| Stat rows | right after the items (`0x1403E39E0`: `items + 8 + count × 0x18`), `u32 count` (151) at `+4`, 16-byte rows from `+8`, bsearch on the u16 id (`0x14041E8F0`) | Checked (decomp + files) |
| Strength / Magic | stat row `+4` / `+5`: the equipment screen compares `+4 / +5 / +6 / +7` against the character's `+0x188 / +0x18A / +0x18C / +0x18E` (`0x1402FCC80`, `0x1402FD050`, `0x1402FCD30`, `0x1402FCBD0`); Strength, Magic, Defence, AP in OpenKH's names | Checked (decomp) / the names: OpenKH + Kingdom Key = +3 / +1 |
| Model | `03system.bin` `went`: word 1 = 70 (base Sora's list), object id = word `70 + weapon number`; `00objentry.bin`: the object's model name (`+8`, 32 bytes). Gives `W_EX010` Kingdom Key, `_10` Oathkeeper, `_20` Oblivion, `_N0` Ultima Weapon | Checked (files) / the exe's use of `went`: not traced, Assumption |
| Mesh + HD | `obj/<model>.mdlx` (BAR: model type 4, textures type 7); one HD asset `-0.dds` per file (512×256 A8R8G8B8, the same format as Re:CoM's own `w_ex010_01_tm2.dds`, alpha 0..255) | Checked (files) |

On the owner's files (03system.bin 109 168 bytes, SHA-256 `fd4b8469…`; `item` entry at file
offset `0x1087C`):

| card | Keyblade | item id (row at file) | stat id (row at file) | Strength | Magic | weapon no. → object | model |
|---|---|---|---|---|---|---|---|
| — | Kingdom Key (reference) | 0x29 (`0x132CC`) | 80 (`0x13ED4`) | 3 | 1 | 1 → 0x67 | `W_EX010` |
| 0x13 | Star Seeker | 0x1E0 (`0x13314`) | 123 (`0x14164`) | 3 | 1 | 4 → 0x769 | `W_EX010_30` |
| 0x14 | Monochrome | 0x1E5 (`0x1335C`) | 128 (`0x141B4`) | 3 | 2 | 7 → 0x76C | `W_EX010_60` |
| 0x15 | Follow the Wind | 0x1E6 (`0x13374`) | 129 (`0x141C4`) | 3 | 1 | 8 → 0x76D | `W_EX010_70` |
| 0x16 | Hidden Dragon | 0x1E1 (`0x1332C`) | 124 (`0x14174`) | 2 | 2 | 5 → 0x76A | `W_EX010_40` |
| 0x17 | Two Become One | 0x21F | 148 | 5 | 4 | | `W_EX010_P0` |
| 0x18 | Bond of Flame | 0x1F2 (`0x1347C`) | 141 (`0x14284`) | 4 | 4 | 20 → 0x779 | `W_EX010_K0` |

The asset step finds each by its English name (`kh2_keyblades::find`, case-insensitive; a
second Keyblade row with the same name must agree) and stops when one is missing. The card ↔ name
pairs are Re:CoM's own Keyblade names at `0x1405060F0` (kinds 0x12..0x18 = Soul Eater, Star
Seeker, Monochrome, Follow the Wind, Hidden Dragon, Photon Debugger, Bond of Flame). Checked
(exe). The Strength / Magic bytes are the stat row's as read; whether KH2 adds anything else to
them in play (e.g. an ability) is not looked at.

### 12.2 The bone mapping and its evidence

KH2's Keyblades are **already on Re:CoM's Keyblade skeleton**. Checked (files):

- All seven `.mdlx` (`W_EX010` and the six) have 12 bones with the same parents and the same
  bind values: bone 0 rotation (0, 3π/2, 3π/2) at (−59.7, 108.0, −2.5), bone 1 identity under it, bone 2 rotation (π, 0, π), and bones 3..11 a chain
  along +x with lengths 21.107, 6.722, 2.954, 2.908, 3.116, 2.979, 3.163, 3.000, 3.262.
- Re:CoM's PL0002 `k9 … k1` (bones 3..11) have identity rest rotations and the same lengths
  × 0.01 (W_EX010: differences ≤ 4.9e-7 m; the six equal W_EX010's to 1e-4). PL0002's bones 0..2 differ from KH2's (Re:CoM's rest pose),
  which does not matter: vertices are stored in their own bone's space, and PL0002.ANN poses
  bones 1..11 itself.
- **KH2's Kingdom Key `W_EX010` is Re:CoM's PL0002 mesh.** Every KH2 corner, in its bone's
  space × 0.01, lies within **0.077 mm** of a PL0002 vertex on the same bone; with UVs, within
  0.0003. Its two groups are 322 + 70 triangles, PL0002's list A + list B. 202 of the 322 opaque
  triangles are PL0002's corner for corner; the other 120 have every corner on a PL0002 list-A
  vertex (Square's strips cut some faces along the other diagonal). All 70 of the translucent
  group are PL0002's list B.

So the mapping is the identity: **KH2 bone j = Re:CoM bone j** (2 = `wep0`, the blade and
guard; 3..11 = `k9 … k1`, the keychain), positions × 0.01, same axes, UVs as they are (KH2's
`u, v` / 4096, origin top left, Re:CoM's convention). The grip needs no transform: the blade's
frame is bone 2's in both, and every KH2 Keyblade has the Kingdom Key's bone 2. All six use only
bones 2..11 (Star Seeker and Bond of Flame 2, 3, 5, 7, 9, 11; Monochrome no bone 10; the rest
all ten). Why one bone-space mesh fits every KH2 Keyblade: KH2 poses all of base Sora's
Keyblades with one weapon moveset (`03system.bin` `wmst`, per moveset and hand, not per
Keyblade; KH2-Combat-in-KH1's `objentry.rs`). Checked (files) / KH2's own posing: from that
project's notes.

The asset step re-checks this on every run and stops otherwise: `kh2_template_distance` (KH2's
Kingdom Key on the player's PL0002, ≤ 0.5 mm), `check_kh2_skeleton` (each Keyblade's 12 bones
equal `W_EX010`'s to 1e-4, bones 3..11 equal PL0002's rest × 100), and a vertex on a bone
outside 2..11 is refused (no remap). No Keyblade needed anything else; none was refused.

### 12.3 Conversion rules (`keyblade::convert_kh2`; the rest is shared with KH1's, § 5)

| step | rule | tag |
|---|---|---|
| mesh | the `.mdlx` model's groups, read as KH2 draws them (VIF unpack into VU memory, index list, bone-local vertices); strips kept as the index list makes them (`StripTriangle`, a new strip at a `0x10` entry after a drawing one and at each packet) | Checked (files: W_EX010 = PL0002) |
| bones, units | bone j → bone j, × 0.01, same axes | Checked (files, 12.2) |
| list A / B | B = the groups KH2 draws in its alpha phase (group attribute bit 0, OpenKH `DrawAlphaPhase`); A = the rest. Reproduces PL0002's split on W_EX010 (322 / 70, the 70 corner for corner). The KH1 rule (any texel with alpha < 255) would move 8 W_EX010 triangles that Square keeps in list A, so it is not used here | Checked (files) |
| list A with alpha | KH2's opaque-phase groups touch semi-transparent texels on some Keyblades (Star Seeker 35 triangles, Monochrome 29, Hidden Dragon 14, Follow the Wind 2, the Kingdom Key 8; the build log's "touch alpha < 255"). Re:CoM's list A packets have PRIM 0x7C (alpha blending on, § 1.3), as Square's PL0002 does with its 8 | Checked (files) / how they look in Re:CoM: Assumption |
| facing | KH2 strip flag `0x20` → Re:CoM `0x20`, `0x30` → 0 (`kh2_facing`). On W_EX010's 202 shared list-A triangles 174 agree with Square's PL0002 flags, 28 don't; on the flat two-sided keychain (list B) 37 / 33 | Checked (files) / the PC's use of the flag: Assumption (§ 1.3) |
| part 1 | every triangle again, as PL0002 | follows PL0002 |
| textures | the HD asset `-<texture index>.dds` (one per file, `-0.dds`), unchanged, as `<model in lower case>_0_tm2.dds`; the TIM2 fallback from it (256×128) | Re:CoM loads A8R8G8B8: Checked (decomp) |
| names | `w_ex010_30_0`, `w_ex010_60_0`, `w_ex010_70_0`, `w_ex010_40_0`, `w_ex010_90_0`, `w_ex010_k0_0`: none is among Re:CoM's 1 649 texture names (case-insensitive, up to the dot; Re:CoM has `w_ex010_01`, `w_ex020_00/01`, `w_ex030_01`, `w_ex200_00/01`) | Checked (files: all RTMs of all resource tables in the owner's Recom.pkg) |
| Bond of Flame | five groups: the blade (opaque) and four translucent keychain groups with priorities 1..4; groups 1/3 and 2/4 have the same outline (bounds, UV box) but not the same triangles (positions and UVs differ, 2/4 the windings too). All go to list B in file order | Checked (files) / that they are the charm's two sides, and how they blend in Re:CoM: Assumption |

### 12.4 Reach (`kh2_keyblades::blade_length`, `blade_far`)

- **Blade length** L = how far the model reaches from bone 2 along the blade: `−min x` of the
  bone-2 vertices × 0.01 (the keychain hangs on +x, bone 3 at +x 0.211 m; the blade runs along
  −x). KH2's Kingdom Key: **0.7790 m**, the same as Re:CoM's PL0002 (same mesh). Checked (files).
- **Rule** (the owner's): `blade_far = KH1 Kingdom Key's blade_far (−3.7) × L / L(KH2 Kingdom
  Key)`; `blade_near` 0, fraction scale 0.8 and radius 40 are the Kingdom Key's KH1 row, so the
  hit spheres span `|blade_far| × 0.8` along `wep0 → k9` as for KH1's Keyblades (§ 10.6).
- **Why this is well founded**, Checked (files): KH1's own weapon rows track their models the
  same way. Each KH1 Keyblade's `|blade_far|` is within 0.13 of its tip's distance from `wep0`
  over `|wep0 → k9|` (0.2111 m), Olympia aside (3.69 vs 3.5):

  | KH1 Keyblade | tip / `|wep0→k9|` | `blade_far` |
  |---|---|---|
  | Kingdom Key | 3.691 | −3.7 |
  | Three Wishes / Crabclaw / Pumpkinhead / Fairy Harp | 3.698 / 3.837 / 4.623 / 3.248 | −3.8 / −3.9 / −4.6 / −3.3 |
  | Wishing Star / Spellbinder / Metal Chocobo / Olympia | 3.230 / 3.690 / 5.052 / 3.688 | −3.3 / −3.7 / −5.1 / −3.5 |
  | Lionheart / Lady Luck / Divine Rose / Oathkeeper | 4.613 / 3.759 / 3.267 / 3.851 | −4.6 / −3.7 / −3.3 / −3.8 |
  | Oblivion / Ultima Weapon / Diamond Dust / One-Winged Angel | 4.911 / 5.520 / 3.922 / 5.124 | −4.9 / −5.6 / −3.8 / −5.2 |

  and KH2's Kingdom Key reaches as far as KH1's (3.691). So scaling KH1's Kingdom Key by the KH2
  blade's length is the same as measuring each KH2 blade the way KH1's rows follow theirs. The
  values are kept exact (not rounded to KH1's 0.1 steps).
- Not KH1: KH2's own hit shapes (its weapon collision) are not used; the owner chose KH1's
  shape with this reach.

### 12.5 The six (owner's files; `kh1-stats.json` rows, `source` `kh2`)

| card | Keyblade | group | Strength | Magic | blade L (m) | `blade_far` | page "Reach" | triangles (A / B) |
|---|---|---|---|---|---|---|---|---|
| 0x13 | Star Seeker | 38 | +3 | +1 | 1.0345 | −4.9131 | x1.33 | 524 (470 / 54) |
| 0x14 | Monochrome | 39 | +3 | +2 | 0.7786 | −3.6980 | x1.00 | 564 (522 / 42) |
| 0x15 | Follow the Wind | 40 | +3 | +1 | 1.0397 | −4.9382 | x1.33 | 558 (522 / 36) |
| 0x16 | Hidden Dragon | 41 | +2 | +2 | 0.8997 | −4.2732 | x1.15 | 554 (488 / 66) |
| 0x17 | Two Become One | 42 | +5 | +4 | 0.8870 | −4.2129 | x1.14 | 568 (526 / 42) |
| 0x18 | Bond of Flame | 43 | +4 | +4 | 0.9100 | −4.3223 | x1.17 | 586 (540 / 46) |

All rows: radius 40, `blade_near` 0, fraction scale 0.8, crit 20, crit bonus 0, priority 30
(KH1's Kingdom Key row, read from the player's `btltbl.bin`). Strength is added to Sora's KH1 base
Strength as for KH1's Keyblades (`strength_with`). The rows also carry `kh2_item`, `kh2_stat`,
`model`, `blade_length` and `kh2_kingdom_key_blade_length` for the record.

### 12.6 Validation

- Tests (`tests/kh2_keyblades.rs`, the owner's files): the seven items, stat rows and models
  (12.1); W_EX010 = PL0002 (12.2: 0.077 mm, lists, facing); all six convert with PL0002's
  skeleton byte for byte, bones 2..11 only, MDL read back and re-written byte for byte, part 1 with
  every triangle, UVs within 0..1, unique texture names, HD texture copied unchanged, TIM2 256×128,
  the raw RTM entry laid out as Panacea reads it; the stats rows. `kh1-recom`:
  `keyblade_models::tests::kh2_keyblades_get_groups_38_to_43`, `kh2_rows_parse`.
- Owner's PC (2026-10-31, Linux build of `kh1-recom-build`): `--game . --out
  kh1-playground/build-out/PL0001.ANN` converted 22 Keyblades in 0.9 s (4.5 s the whole step) and
  wrote `PLAYER/0038..0043/PL00GG.MDL` + `raw/PLAYER/0038..0043/PL00GG.RTM`. The 36 files of the
  0.9.0 run (motion packs, SES, manifest, the 16 KH1 Keyblades) came out byte-identical; only
  `kh1-stats.json` changed (the six rows appended). The runtime's `parse_stats` and
  `Models::from_stats` read that file: 23 Keyblades, model groups 21..36 and 38..43, camp-page
  values as in 12.5. Checked (run).

### 12.7 Runtime

No new hook, address or code path. Generalised, with tests: `keyblade_models::OUR_GROUPS`
(21..=36) became `our_group_for_card` (`19 + card` for cards 2..=0x11 and 0x13..=0x18; Soul
Eater none), which `Models::from_stats` requires of every row. The table extension (`plan_table`,
now 110 rows for 22 groups), the pick, the load and unload, the camp list (`unlocked`, every
card with numbers, 1..=0x18 but Soul Eater), the save byte (1..=0x18) and the rebuild at camp
close already took any group from `kh1-stats.json`. Checked (code + tests).

### 12.8 Assumptions and what to see in game

Assumptions: the six render like the KH1 ones through the same route (raw RTM with HD, list A /
B, facing flag); Bond of Flame's four translucent keychain groups blend acceptably in Re:CoM's
list B; KH2's semi-transparent edges in list A look right; the trail length on longer blades
(§ 9.6); the KH2 names are in `msg/us/sys.bar` on every install language.

In-game checklist (KH1 setting, Sora's story; files of this run installed):

1. Log: `keyblades: models in: PLAYER table 663 rows (553 Re:CoM's + 110 for groups [21, …, 36,
   38, …, 43])`.
2. Camp → Keyblades: each of the six earned shows with Re:CoM's card name; Strength / Magic /
   Reach as in 12.5 (e.g. Bond of Flame +4 / +4 / x1.17).
3. Equip each, close the camp: `rebuilt at camp close with group 38..43`; the KH2 model in
   Sora's hand, grip and blade where the Kingdom Key's are, keychain hanging and swinging from the
   pommel.
4. HD texture (not the 256-colour TIM2); Bond of Flame's charm and the translucent keychains;
   both sides of flat parts.
5. Swing at a twin enemy: hits reach along the whole blade (Star Seeker / Follow the Wind about a
   third further than the Kingdom Key); damage follows the new Strength.
6. Room change, battle, save / load, Halloween Town: the equipped KH2 Keyblade stays.
7. Back to a KH1 Keyblade or the Kingdom Key: `group 38..43 unloaded`.

## In game, 0.9.0 (owner, 2026-10-31)

- Owner: "perfect, everything is great". Checked (owner).
- Log (Checked): the PLAYER table had 633 rows. Each equip (Three Wishes, Crabclaw,
  Pumpkinhead, Diamond Dust, One-Winged Angel) queued its group, the weapon was rebuilt at camp
  close with that group, and the previous group was unloaded. No fallback lines.

## Correction: Re:CoM's real card names (0.11.1, owner 2026-10-31)

- The strings at `0x1405060F0` are the exe's internal names, not the names the cards show.
  Re:CoM's own card names (`0x140164CA0`, shown on the camp page) for kinds 0x14..0x17 are Total
  Eclipse, Midnight Roar, Maverick Flare and Two Become One. The names are from the owner's camp
  page and Re:CoM's list; the exact message ids are not read.
- Owner: Star Seeker and Bond of Flame were the right Keyblades. Two Become One wasn't (it had
  KH2's Photon Debugger), but KH2 has it.
- Card 0x17 now takes **KH2's Two Become One**: item 0x21F, stat row 148, Strength +5,
  Magic +4, model `W_EX010_P0`, blade 0.887 m → reach −4.213 (x1.14). It maps like the others:
  bones {2, 3, 5, 7, 9, 11}, 568 triangles. Checked (files).
  - Its UVs reach 1.00024, one step of KH2's 1/4096 fixed point past the edge. They are kept
    as KH2 stores them.
- Total Eclipse (0x14), Midnight Roar (0x15) and Maverick Flare (0x16) exist in neither KH1 nor
  KH2. For now they keep the KH2 placeholders Monochrome, Follow the Wind and Hidden Dragon:
  their models and numbers. Owner: "for right now the placeholder models are fine".
  `kh2_keyblades::RECOM_NAMES` marks them.

