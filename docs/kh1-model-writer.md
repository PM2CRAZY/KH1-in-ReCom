# KH1 model writer: step 1 of the "Re:CoM's content in KH1" test

Owner, 2026-10-30, option (b) of `flip-feasibility.md`: "a KH1 model writer that rebuilds an
existing KH1 enemy byte for byte and has KH1 load it. Then put an Organization member's model
through the same writer." This note covers the first half. Tags: Checked (exe / decomp / files /
source) or Assumption.

## What the PC loads for a model. Checked (files + source)

- A model is a `.mdls` PKG entry plus its **HD assets**: `.cvbl` meshes, `.dds` textures and
  `.scd` sounds, each tagged with an offset in the `.mdls` (`orig_offset`) or by its name
  (`-c0.cvbl` = the main model at file `+0xC0`). The 500 `xa_*.mdls` carry 941 `.cvbl`, 9415
  `.dds` and 3218 `.scd`.
- The PC draws the `.cvbl` mesh with the `.dds` textures, not the `.mdls` PS2 packets
  (KH2-Combat-in-KH1 `docs/kh1-tint.md`, `kh1-weapon-second.md`). The `.mdls` still gives the
  skeleton and everything else the game reads. So a native new model needs both writers.
- HD textures are tagged by the PS2 texture data offset in the `.mdls` (Shadow: `-16480.dds` =
  `orig_offset` 0x16480 = `MOBJ` 0x80 + texture data 0x16400) or by a small slot number
  (`-1c0.dds` = 0x1C0). A new `.mdls` with a different layout needs its own asset table, so the
  original entry's table can't be reused.
- Panacea (OpenKH `Panacea.cpp`): a file at `<mod_path>/kh1/raw/<name>` is read as a whole PKG
  entry, HD asset table included (`GetRawFile`, `LoadFile`, `GetRemasteredAsset`); size fields
  of -2 mean plain data. This is the route the test uses: KH1's own loader gets an entry that is
  entirely ours.

## Formats (`kh1-formats`)

- `mobj.rs`: whole `.mdls` files. Container, `MOBJ` header, main model (joints, the 60-word
  table, meshes), the optional second model (441 of 500; plain meshes: 0x50-byte matrix items,
  16-byte vertices), PS2 packets (DMA / VIF tags, matrix, strip, weight and end items),
  vertices with one to four weighted positions, textures and CLUTs. Every size, count and offset
  is rebuilt on write. The layout rules are in the module header.
- `cvbl.rs`: the HD mesh. Header (with the optional 0x20-byte patch entries), mesh table, and
  per mesh the `0x12345678` fill, matrix chunks (0x80 textured, 0xC0 weighted, 0x50 plain) and
  one vertex chunk (48-, 80- or 16-byte vertices). Rebuilt on write.
- `egs.rs`: `decode_entry_full` (an entry's file and all HD assets) and `build_plain_entry` (a
  plain entry for Panacea's raw route).

Kept as bytes, not decoded: `.mdls` sections 1–7 (AI scripts and more, Stage 13), the block
after `MOBJ` (`_KN5`, `BONE` or `URSL`), the 60-word table's meaning, the `.cvbl` matrix data
and patch entries, the 80-byte weighted vertex layout, texture info bytes 0–1. Their sizes and
places are rebuilt; their content is copied.

## Results

- **Byte-exact on the whole install** (owner's Steam files, run on the owner's PC through the
  device link, `kh1-flip-test roundtrip`): 500 of 500 `.mdls`, 941 of 941 `.cvbl`. Every
  rebuilt PKG entry decodes back to the same file and the same 12,633 other HD assets.
  Checked (files).
- One quirk: five Halloween Town models' `-1c2.dds` inflate to 12 bytes more than their header
  says. The reader keeps the header's size, which is what the game asks for. Assumption: the
  12 bytes are never read.
- Test entries for the Shadow (`xa_ex_2020.mdls`), 7,061,408 bytes each:
  - `rebuild`: the model and its HD mesh written by our writers, same content;
  - `big`: every vertex position × 1.5 (1,422 PS2 positions, 1,368 HD positions).
- **Not checked yet: the game.** Whether KH1 loads and draws them is the tester's step
  (`testers/kh1-flip-model/thread.md`).

## What this does not show yet

- A model with a different shape than the one it replaces (new mesh counts, new textures, a
  different skeleton). `big` keeps every count and offset; only the positions change.
- Whether an Organization member's model fits KH1's 16 MB block arena (`flip-feasibility.md`).
- Re:CoM's `.MDL` meshes are not read yet (only its skeleton). That reader is step 2.
