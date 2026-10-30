# Feasibility: Re:CoM's content in KH1, instead of KH1's combat in Re:CoM

Owner, 2026-10-30: "would it not be easier to just bring all the maps and unique enemies/bosses
into KH1 as a mod for KH1?" Option (b): scope it first, no code. Notes only.

Tags: Checked (exe / decomp / files) or Assumption.

## What the flip would need

1. Re:CoM's unique maps (Castle Oblivion, Twilight Town) as KH1 maps.
2. Re:CoM's floors, built room by room from map cards, rebuilt in KH1 or fixed in place.
3. Re:CoM's unique bosses and enemies as KH1 enemies: models, motions, AI.
4. Re:CoM's story (events, cutscenes, voices, flags) as KH1 events.
5. Everything inside KH1's memory.

## Findings

### KH1 cannot load new models natively today. Checked (files)

- KH2-Combat-in-KH1 shows KH2's Sora in KH1 by drawing him with its own D3D12 pass and CPU
  skinning (`docs/kh1-kh2-model.md` there). KH1's model is masked off, not replaced. Under this
  project's rules that is not native.
- OpenKH (master, 2026-10-30): `OpenKh.Kh1` reads `.mdls`, part of `.cvbl` and the ARD resource
  list (and writes only that list). There is no KH1 model, motion or map writer. KH1's map
  files (`xx00_NN.bin` / `.img`) are listed as "World Binary Data", undocumented.
- Our own crates read KH1 `.mdls` / `.mset` (`kh1-formats`) but write only Re:CoM `.ANN`.

### KH1's memory is a fixed PS2-style arena. Checked (decomp)

- The game allocator `0x1402858E0(block, count)` hands out 256 KB blocks (`block << 0x12`) from
  `0x140D2ADA0`, with 0x50-byte block records at `0x142869DD0`. The record loops stop at 0x40,
  so 64 blocks = 16 MB.
- Maps load through it: the map path builder `0x14017DA40` takes block `0x34`, then loads
  `<world>00_<room>.img` with `0x14028BC10`.
- Assumption: Castle Oblivion rooms plus an Organization boss fit in the blocks a KH1 room and
  its enemies get. A KH1 PC modder reports that one extra Sora animation crashes the game
  (Nexus, "Playable Kairi" thread). Not tested by us.

### KH1 has free world slots. Checked (exe)

- World prefix table `0x1404DCCA0`, indexed by the world id `0x14233FE84`: 21 entries, `dh di
  dc tw aw tz po xx al lm nm he pi pp xx pc ew xx xx xx zz`. Ids 7, 14, 17, 18, 19 are `xx`
  (unused).
- So Castle Oblivion and Twilight Town could take unused ids without growing this table.
  The other per-world tables (world map, save, gummi) are not checked.

### Re:CoM's content is in Re:CoM-only formats. Checked (files: OpenKH `recom.txt` name list)

- Maps: one folder per room, `MAPnn/xxxx/MPnnxxxx.{AFF,ALF,AZF,mcd,RTM,VTM}`. About 630 room
  folders over MAP00–MAP30 (MAP21 alone has 116).
- Models and motions: `MDL` / `ANN` (501 / 490 under ENEMY and NPC); 151 enemy and 355 NPC
  folders.
- World and event data: `WORLDnn/xxxx/WOnnxxxx.{ESD,GSD,CTD,ABC,CAP,PTD}`, about 1,900 files.
  Assumption: ESD / GSD are the event scripts (the exe names them by path; not read).
- OpenKH has only an archive reader for Re:CoM (`OpenKh.Recom/Dat.cs`). We read `ANN` and
  Re:CoM's skeleton from `MDL`, nothing else.

### Re:CoM's bosses are compiled C++, KH1's enemies are scripts. Checked (exe RTTI; decomp)

- Re:CoM: 20 boss families as native classes, 78 classes in all: `CBossAxel`, `CBossLarxene`,
  `CBossVexen`, `CBossLexaeus`, `CBossZexion`, `CBossMarluxia` (and `CBossMarluxia3…`, 9+ parts),
  `CBossRiku`, `CBossAnsem`, `CBossDark`, `CBossGuardarmor`, `CBossTrick`, `CBossParasite`,
  `CBossBoogie`, `CBossUrsula`, `CBossHades`, `CBossHook`, `CBossJafar`, `CBossCloud`,
  `CBossDead`, `CBossM`. 116 `CEnemy…` / `CBoss…` classes in all.
- KH1: enemies run AI scripts from their `.mdls` (syscall records, Stage 13).
- So each Re:CoM boss becomes a rewrite from the Re:CoM decomp into KH1's AI script.
  Assumption: no public KH1 AI script assembler exists. The other way (our own code driving
  the boss beside KH1) is "running along" under the design rules.

## Comparison

| | Current plan (KH1 into Re:CoM) | Flip (Re:CoM into KH1) |
|---|---|---|
| Sora's combat | Built through 0.7.2: movement, Attack, KH1 damage on 31 classes, Guard, Counterattack. Left: M6–M9. | Free. |
| KH1 enemies | M10–M11: each rebuilt natively in Re:CoM. The largest item left. | Free. |
| Re:CoM's maps and floors | Free. | New KH1 map writer + Re:CoM map decoder + room synthesis or fixed floors. Nothing exists. |
| Re:CoM's bosses | Free (they keep Re:CoM's behaviour, decision 2). | 20 families rewritten as KH1 AI, plus model/motion writers. |
| Story | Free. | ~1,900 Re:CoM event files converted to KH1 events. Formats unread. |
| Re:CoM setting | Kept (toggle). | Gone. |
| Memory | Re:CoM's. | KH1's 16 MB arena. |

## The one decisive test, if the flip is pursued

A KH1 `.mdls` writer that round-trips: decode a KH1 enemy, encode it, and KH1 loads and draws
it unchanged. Then a Re:CoM model (an Organization member) through the same writer, in place of
a KH1 enemy, within the arena. If either fails, the flip has no native path for models. Maps
would need the same test after that.

## Step 1 done (2026-10-30, version 0.7.3)

The model writer exists and is byte-exact on all 500 `.mdls` and 941 `.cvbl` of the owner's
install (`kh1-model-writer.md`). Finding: the PC draws the `.cvbl` HD mesh, so a native new model
needs a `.cvbl` and `.dds` textures as well as the `.mdls`. The in-game check is
`testers/kh1-flip-model/thread.md`.
