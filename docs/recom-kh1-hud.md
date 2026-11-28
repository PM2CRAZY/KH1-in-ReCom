# The KH1 HUD option (decision 39)

Owner (2026-10-31): "Would we be able to instead have an option where the Re:CoM [HUD] is
replaced entirely by the KH1 hud? ... use the KH1 decomp to get exact positioning and graphics".
Details (decision 39): HP ring on Re:CoM's raw HP (KH1's rule), party gauges hidden until they
join, menu words in KH1's font converted into a Re:CoM font, field / battle command menu looks,
Re:CoM's enemy gauges hidden and KH1's lock-on cursor used, Re:CoM's low-HP sound, Re:CoM's
extras hidden for now (the owner places them later).

Stages: H0 dump (done, outside the repo) → **H1 the `kh1hud` set** → **H2 Sora's cluster** →
**H3 hiding Re:CoM's HUD** → **H2b the cluster's motions** → H4 command menu look → H5 LOCK ON! +
target gauge → H6 party gauges → H7 lock-on cursor → H8 popups.

Steam addresses (image base 0x140000000), both exes. Tags: **Checked (decomp)**, **Checked
(exe)**, **Checked (files)** = the owner's files (the H0 dump of 2026-10-31), **Assumption**.

## KH1's HUD data (H0 dump, Checked (files) + decomp)

- KH1's HUD pieces are sprite objects in `exchange/<LANG>_uibin_*.bin` (UK, GR, FR, SP, IT on PC;
  English is `UK`: `0x1400D7380` maps the language index to JP, UK, UK, GR, FR, SP, IT, FM).
  Header `"TTUI"`, 2, then four (offset, size) pairs: objects (0xC0), colour keys (0x20), vertex
  colour keys (0x50), position keys (0x20) (`0x14027D9D0`). Object: `+0x00` **x, y, w, h**
  (`0x14027EA50` draws (x, y)–(x + w, y + h)); `+0x10` u0, v0, u1, v1 inclusive (u1 < u0
  mirrors, `0x14027E7D0`); `+0x20` RGBA or four vertex colours (flag 0x20); `+0x60` flags (0x02
  colour keys, 0x40 vertex-colour keys, 0x04 position keys; +1 bit each while running); key
  players `+0x64`, `+0x78`, `+0x8C`; `+0xA0` the name. The research note's "(x, y, z, w)" and
  "0x50 keys = scale / rotation" were wrong: the 0x50 keys are four keyed vertex colours.
- Positions are absolute in KH1's 512 × 384 space; the code adds the size-class offset and the
  widescreen margin (`0x140269190`, `0x14026CE90`), and for the straight segments (−1, +1)
  (`0x1402698C0`, `0x14026BEE0`, `0x14026C6E0`; the charge's (0, +1), `0x14026A970`).
- The 4:3 space is centred on a 16:9 screen: left pieces at x − 86, right ones at x + 86
  (`0x14232DCC8` / `0x14232DCCC`). The owner's KH1 screenshot (2048 × 1152) puts Sora's face at
  1838 px = 612.7 units = 443 + 86 + 85.33. Checked (files).
- `command2/uitex.bin`'s PC assets: one per picture, named `-<pixel offset>.dds` (picture 0
  512 × 512, picture 2 256 × 384 with PC-only rows below, the others 2×; DXT5 or A8R8G8B8), plus
  per language `<LANG>_uitex_bin40.dds`, `…10780.dds` (picture 7: "mp" / "HP"), `…16c40.dds`,
  `…20c0.png` (picture 1, words). Checked (files).
- Faces: section 2 of `xa_ex_0010.mdls` (Sora, 192 × 64, three cells), `xa_ex_0040.mdls`
  (Donald, 48 × 48), `xa_ex_0050.mdls` (Goofy, 48 × 48), PC `-<pixel offset>.dds` at 2×.
  Checked (files).
- Fonts: the PC menu font is `<lang>/font/menufont.tm2` / `.inf` / `.cod` in `Image/dt/Theater.pkg`
  (16 MB TIM2), the slot-1 font of the table `0x1404D9150` the command menu's text uses
  (`0x14027A920` → `0x1402D1CD0`, font 1). For H4.

## Re:CoM's PC 2D files (Checked (files + decomp))

- `FO0002.SPR` holds `gauge.lad`, `Cockpit.lad`, `layout_card.lad`; its PKG entry's PC assets
  are `<LANG>_Cockpit.lad` / `<LANG>_gauge.lad` (FR, GR, IT, SP, UK). `FO0002.RTM` holds
  `Cockpit.imz`, `layout_card.imz`; its PC assets are `<LANG>_Cockpit.imz`: a 1024 × 1024
  32-bit IMGD (PSMCT32, linear, RGBA with PS2 alpha 0x80 = opaque).
- Re:CoM takes a PC asset whose name contains the file's name and ends in `.lad` / `.sqd` / `.spd`
  (`0x14029FCC0`: `strstr`, then the extension) in place of the file. The PC layouts' UVs
  address the PC atlas in its own texels (`UK_Cockpit.lad`'s sprite 1: PS2 (3, 160)–(126, 177),
  PC (520, 92)–(924, 150): a re-laid atlas, not a plain × 2).
- A `.lad`'s registration (`0x140244AD0`) stores its two layout tables and registers each SEQD
  as `<name>:NN`; nothing else in it is needed to make a set.

## H1: the `kh1hud` set (asset step + DLL check)

- **Asset step** (`kh1-recom-assets` `kh1_hud`, `uibin`; bin `kh1-recom-build`): reads KH1's
  `command2/uitex.bin` (+ PC assets), `exchange/UK_uibin_g_s.bin`, `UK_uibin_g_red.bin`, the three
  faces, Re:CoM's `FO0002.SPR` (after the MP bar's patch) and `FO0002.RTM`, and writes
  - `kh1hud.lad` into `FO0002.SPR` (one SEQD → `kh1hud:00`) and `<LANG>_kh1hud.lad` assets (the
    same records with the PC atlas' UVs) for every language Re:CoM has `<LANG>_Cockpit.lad` for;
  - `kh1hud.imz` into `FO0002.RTM` (PS2 atlas 1024 × 512, 32-bit) and `<LANG>_kh1hud.imz`
    (PC atlas 2048 × 1024, 32-bit, that language's KH1 picture 7);
  - `raw/FORM/0002/FO0002.RTM` (new) and `raw/FORM/0002/FO0002.SPR` (as before, now with
    `kh1hud.lad`), and `kh1-hud.json` (format `kh1-recom-assets kh1-hud 1`, the group count, the
    languages, the inputs' sha256). Re:CoM's own files and records stay byte for byte.
- **Atlas:** KH1's pictures 5, 6, 7 and the three faces, PS2 and PC; and KH1's arcs baked: the
  asset step runs KH1's own fan (`0x14026B790`, vertices `0x14026E450`) over the ring textures
  and stores each piece a frame needs (§ H2). One texel of gap (two on PC) around each box.
- **Texture mods** (`fit_hd`, `fit_all`): a KH1 texture mod may replace the PC pictures at any
  size (a tester's: Sora's face 1200 × 400 over the PS2's 192 × 64 = 6.25×, Donald's / Goofy's
  8×, `uitex.bin` pictures 4×, picture 7 replaced for some languages only). Each PC picture is
  resampled (area average, colour weighted by alpha) to KH1 FM PC's own scale for it: 2×, picture
  0 4×. Before: the step refused a non-whole scale and skipped the whole HUD; kept at the mod's
  scales the PC atlas was 2048 × 4096 a language and Re:CoM crashed after the title (exe
  `0x1403440AC`, null write). After: the stock 2048 × 1024. Stock pictures are untouched: the
  step's output on stock files is byte for byte unchanged. Checked (files: stock and texture-mod
  installs; in game on the texture-mod install).
- **DLL** (`kh1_hud`, `win/kh1_hud.rs`): reads `kh1-hud.json` (refuses another format or group
  count) after the MP bar and command menu install; at each player gauge's init (the MP bar's
  wrapper `0x140208360`) checks once that Re:CoM's 2D registry holds `kh1hud:00` with the
  manifest's group count and logs it.
- **Switch:** `hud = recom | kh1` in `kh1-recom.ini`, default `recom`; interim until the KH1 camp
  entry has its row.
- Checked (files): the set built from the owner's dump and drawn by an independent script (SEQD
  parse, PC atlas) over the owner's KH1 screenshot: ring radii, positions, labels, face, segment
  pitch line up with KH1's.
- Assumption: Re:CoM's IMGD reader takes a 32-bit picture in the PS2 (non-PC) file too (only
  used when no language copy matches).

## H2: Sora's gauge cluster (bottom right)

### KH1's rules (Checked (decomp + exe) unless tagged)

| what | where |
|---|---|
| Slot 0's objects: `+0x10` face disc `g_s_fra_a_a..d` + "HP" / "mp" labels (picture 7; "mp" skipped without MP), `+0x50` face frame `g_s_fra_a_wa..wd` (picture 6), `+0x78` / `+0x80` max-MP ticks (objects 37, 38: past 90° / 180°), `+0x90..+0xC8` the straight pieces: colour HP 31, MP 32, charge 33; back HP 16 (first) / 18 (further, x + 26), MP 17; caps 20 (x := 0x19E), 21 (x := 0x1B8) | index lists `0x1405270E0`, `0x140527138`, `0x140527168`, `0x140527180`; `0x14026E630` |
| Centre (443, 316) + size class offset (24, 24) / (0, 24) / (0, 0): class 0 when max HP < 19 and max MP < 4, 1 when max HP < 37 and max MP < 7 | `0x1405272B0`, `0x140527300`, `0x1402690D0` |
| Draw order: face frame, disc + labels, HP back arc (to min(max HP, 120)·5°) + back segments + cap, MP back arc (min(max MP, 20)·30°) + segments + cap, MP arc, danger flash, HP arc + segments, charge arc + segments, ticks, face | `0x140269190`, `0x1402698C0`, `0x140269F50`, `0x14026CF90` |
| Arcs: a triangle fan from (centre x, centre y + 1) over the square of half-size R; vertex at θ: d = trunc(tan t · R) (1/16 units), right / bottom halves one unit out; UVs fixed in the quarter-ring texture; rings (1/16): HP back (744, 1288) r688, MP back (1720, 1512) r912 (picture 7), HP (680, 1592) r656, MP (904, 904) r880, charge (1816, 904) r880 (picture 5) | `0x14026B790`, `0x14026E450`, `0x1405273B0` |
| Straight segments at (−1 − 26 i, +1) from the object (charge: (−26 i, +1)); a partial one is the object with x + cut, w − cut, UV kept: cut = (int)((1 − rem / per) · w + 0.5), 1..w−1 (HP per 6 HP, w 26; charge per 30, w 24) | `0x1402698C0`, `0x14026BEE0`, `0x14026A970` |
| MP fade: the changed arc RGB 0 → 128 (gain) / 128 → 0 (loss), 12.8 a frame, 10 frames; charge: alpha the same way; ring part only | `0x14026C6E0`, `0x14026A770` |
| Danger: HP > 0 and HP / max < 0.2 (max > 44) else HP < 9; flash = `g_red`'s `g_s_bar_b_hp_red` keys: colour (128, 0, 0, a), a 0 → 128 over 8 key units and back over 8 (16 + 16 frames), restarted when done, on the max-HP arc and segments under the HP arc | `0x14026E180`, `0x14026D570`, `0x14026BCF0` |
| Face: cell = expression of the h × h strip centred on the gauge centre; 1 for 40 / 50 / 60 frames after a hit (new / old HP ≥ 0.8 / ≥ 0.5 / < 0.5), else 2 in danger, else 0 | `0x14027F470`, `0x140268D60`, `0x14029BBD0` |

### Native build

- **Data** (asset step, in the H1 set): the arcs are baked by running KH1's fan in the asset step
  (the same vertices, triangles, UVs and sampling points as the GS draws, PS2 and PC pictures) into
  pieces: per quadrant every 5° (HP and HP back, 54 + 54 pieces), every 30° (MP back), per 30° cell
  (MP), per cell and degree (charge, 270). Held groups take KH1's values as frames (frame →
  pieces, compiled into Re:CoM animations). Fades, the hurt face and the danger flash are played
  groups (`crate::kh1_hud::group`).
- **Controls** (`win/kh1_hud.rs`): at `hud = kh1` in Sora's story, at the player gauge's init:
  three `C2DSeqCtrl` (Re:CoM's `operator new` `0x1404ACE54` + ctor `0x140247260`), no parent,
  layout position `0x140248660` = the size-class offset, every layer at KH1's right margin (86 at
  16:9, 0 at 4:3, from the gauge's `+0xA0`) with flag 1: back (priority 2: frame, HP back, MP back,
  MP), front (3: HP, charge, ticks), face (3). Pieces (fades, flash) are pooled controls, flag 0
  (played), priority 2 (MP fades, flash: under the HP ring) or 3 (charge fades).
- **Events only:** Re:CoM's HP setters `0x14020D6A0` / `0x14020D8E0` (wrapped, `sites::KH1_HUD_HOOKS`:
  `48 89 5C 24 10 57 48 83 EC 40`, `45 33 C0 0F B7 C2 85 D2`) give HP and max HP (`+0xA4`,
  `+0xA8`: Re:CoM's raw HP, decision 39); KH1's MP reads / writes (`win/mp.rs`) give MP and charge;
  the gauge's HUD state (7 / 1) hides and shows ours; the dtor deletes ours. Nothing per frame.
- **The 7.2 MP bar** is not made at `hud = kh1` (the command menu still finds its groups).
- **Re:CoM's HP gauge** keeps running (its low-HP sound 9 at ≤ 25 % is the owner's choice); H3
  hides it.

### Not 1:1 yet (to ask / later; H2b below builds 1, 2 (soft edge), 4 and settles 5)

1. **Hit shake and size-class slide** (`g_act`: `vibe_s/ss/sss`, `slide_s_h/v`, position keys):
   they move every piece at once; Re:CoM's 2D animates per layer, so a native shake needs the shake
   keys in every held group or the controls moved over the shake's frames (per frame while it
   runs). Not built; the class offset changes at once.
2. **The arcs' end strips** (`0x14026B790` mode 1–3: a piece at the back arcs' end; mode 4: a
   0.7° soft edge on the HP, charge and flash arcs): their angles are float arguments the decomp
   lost; not baked.
3. **Charge fade overlap:** a fading piece starts at its 30° cell's start, so for 10 frames it is
   drawn over the part of that cell already shown (same texels; only edge texels get a little
   more opaque). KH1 fades exactly [old, new].
4. **The gauge's own fade** (`+0xA9C`, ±0.1 a frame, menus / events) is not reproduced: ours shows
   and hides with Re:CoM's HUD state at once.
5. KH1 restarts a running MP fade from its last value; ours drops it.
6. Assumptions: same-priority controls draw in creation order (the MP bar's assumption 4); a
   played group holds its last frame (5); a group word `+4` of 0 loops (the danger flash);
   `0x14029BBD0`'s `iVar4` is the HP after the hit (strength rule).

### What the first in-game test must check (H2)

1. `hud = kh1` in `kh1-recom.ini`, Combat = KH1, Sora's story. Log: `kh1 hud: Sora's KH1 gauge at
   hud = kh1 (H2)…`, then per room `kh1 hud: built on the player gauge …: HP h/m, MP Some((…)),
   class c, margin 86`.
2. **Screenshot** in the field at full HP / MP: the cluster bottom right as KH1's (ring sizes,
   "mpHP" labels, face, segment pitch); Re:CoM's HUD is still there (H3).
3. Take hits: the green arc shrinks at 5° per HP; Sora's face grimaces for 40–60 frames.
4. Low HP (< 20 % of max, or < 9 HP when max ≤ 44): the max-HP arc flashes red under the green;
   the face shows the exhausted look; Re:CoM's low-HP sound plays.
5. Cast a spell: the MP cell goes black then disappears over 1/6 s; refill: cells rise from black.
6. Swing at enemies: the orange charge line grows along the outer ring, fading in.
7. Events / cutscenes hide it with Re:CoM's HUD; a battle and back: it is rebuilt.
8. `hud = recom`: Re:CoM's HUD and the 7.2 MP bar as before.

## H2b: the cluster's motions baked into the data (decision 39, 2026-11-01)

### KH1's rules (Checked (decomp + exe + files))

| what | where |
|---|---|
| One motion pointer per slot (`+0x110`): `0x14026D990` adds its keyed offset (player C, x / y) to the centre every piece of the cluster is drawn from | `0x14026D990` |
| A hit (`0x14029BBD0` → `0x140268D60(slot, strength)`) starts `vibe_s` / `vibe_ss` / `vibe_sss` (slot `+0x2A0` / `+0x360` / `+0x420`, `g_act` objects 0, 1, 2) **and the hurt-face timer, only when no motion runs** (pointer 0) | `0x140268D60` |
| A size-class change starts `slide_s_h` (new class 0 / 1, slot `+0x120` = `g_act` 3) or `slide_s_v` (class 2, `+0x1E0` = 4), over a running shake; the drawn class stays the old one until the slide ends, then the new one | `0x14026D570`, `0x14026D990` |
| Keys: `vibe_s` y 0, 3, −3, 2, −2, end 0 (1 unit each); `vibe_ss` 0, ±6, ±4, ±3, ±2, ±1 (11 units); `vibe_sss` 0, ±9 … ±1 (19); `slide_s_h` x 0, −4, 32, −36, −18, end −24 (units 2, 2, 4, 1, 1); `slide_s_v` the same in y. Players step 0.5 a frame, drawn after the step: 2 frames a unit (10, 22, 38, 20 frames) | `exchange/UK_uibin_g_act.bin` (Checked (files)), `0x14027E430` |
| The gauge fade (`+0xA9C`, `0x140268E30`): ±0.1 a frame, stepped before drawing (0.9 … 0.0 out, 0.1 … 1.0 in), multiplies every vertex alpha | `0x14026D570`, `0x14026B790` |
| MP and charge fades: every change sets the fade afresh (`0x14027F760`: 128 → 0 for a loss, 0 → 128 for a gain) and draws only the latest change's arc. **H2's gap 5 was wrong**: KH1 does not resume a running fade; H2 already restarts it, as KH1 | `0x14026C6E0`, `0x14026A770`, `0x14027F760` |
| Soft end (mode 4: the HP arc, the max-HP flash, the charge and its fade arcs): when 1 < end < 270 the fan stops 0.7° short and a last triangle (centre, end − 0.7°, end) fades to alpha 0 at the end vertex | `0x14026B790` (`0x14042F7F4` = 0.7; callers push 4: `0x14026BDF6`, `0x14026BE57`, `0x14026A770`) |

### Re:CoM's side (Checked (decomp + exe))

- A layer whose frame was set never advances (`0x140248BB0`: `+0x38` ≥ 0 holds the time), so a
  held layer cannot move by itself. A played layer runs to its group's end; after the end only
  the group's longest animation stays drawn (`0x140248F40` marks it, bit 4 of `+0x144`).
- `0x140248550(ctrl, layer, time)` plays a layer from a time (f32 in xmm2, units of 1/50 frame:
  `0x140246710` → the `C2DSeqObj`'s vtable `+0x30` `0x140249360` sets `+0x38` = −1, `+0x30` =
  time); `0x140248000(ctrl, layer)` returns its time (`0x140246250` → vtable `+0x68`
  `0x140246D60` → `+0x30`). Re:CoM's own code uses both (16 and 3 callers).
- Animation flags (`0x14024B4E0` shuffles them; `0x14024A9xx` reads): bit 0 linear (no curve);
  translation interpolated only when 0x10000 and 0x1000 are clear (else the start offset stays);
  0x8000 = no extra frame after `f1` (end exclusive). Translation: `+0x10` x0, `+0x14` x1, `+0x18`
  y0, `+0x1C` y1, ints in 2D units (+0.25). Re:CoM's `Cockpit:00` has both kinds (0x6B73 moving,
  0x4FB71 exclusive).
- An animation group's first animation and count are read as unsigned 16-bit (`0x140248F40`), so
  a SEQD holds up to 65535 animations.

### Native build

- **Data (asset step):** every distinct piece set of the held groups FRAME … FACE (a *unit*: 852
  on the owner's files) gets one played group per program: `vibe_s`, `vibe_ss`, `vibe_sss` (the
  KH1 keys as moving animations of 2 frames a key unit, flags 0x6EFF1, then the last offset held to
  frame 10,000,000), `fade_out` / `fade_in` (alpha 115 → 0 / 13 → 128 over frames 0..9, then
  held), and for the 423 units a slide can show (HP ≤ 72, charge ≤ 210, the rest: Assumption, a
  bound) `slide_s_h` / `slide_s_v`. The hurt-face groups carry the shake of their strength. The
  manifest (format `kh1-hud 2`) lists the programs (start frame, length), each unit's first group
  and each run (held group, frames, unit). 5738 groups, 43,030 animations, `kh1hud.lad` 6.4 MB (×6
  copies in `FO0002.SPR`).
- **Soft ends** are baked into the HP and charge arcs' last pieces, the charge fade pieces below a
  full cell and the flash pieces below a full quadrant.
- **DLL (events only):** a hit with no motion running → the held back / front controls hide and
  the units of what they show play `vibe_<strength>` (from frame 1: KH1 draws key time 0.5 first),
  with the shaken hurt face; a size-class change → the slide at the old class's place (the held
  controls already at the new one); Re:CoM's HUD hide (state 7) → hidden at once (0.178.1, fork
  owner's answer 3b; `fade_out` is built but no longer played), show (state 1) → `fade_in` on the
  units (face included). A view change while a motion runs rebuilds its units at
  the same time (`0x140248000` on the first unit layer); a view change after it ended gives the
  screen back to the held layers. Nothing per frame.

### Not done / to check

1. **Back arcs' end strips** (modes 1–3, `0x1402694C0` mode 1, `0x140269F50` computed): black
   strips built by `0x14026E270` whose float arguments the decomp lost; not baked yet (needs the
   disassembly of `0x14026B790`'s tail).
2. While a motion runs, the MP / charge fade pieces and the danger flash stay where they are (KH1
   moves them too): visible only when MP or the flash changes during the 10–38 frames.
3. A slide for a change that isn't KH1's (class 0 → 2 or downwards) ends at the slide's last
   offset and snaps to the new class at the next HP / MP event (KH1 snaps at once).
4. Answered (fork owner, 2026-11-23, 3b): every Re:CoM hide (events and menus alike) is at once;
   the fade-in still runs on every Re:CoM show. KH1 hides at once for its events (world flag 8) and
   fades for its menus; telling Re:CoM's event hides from its menu hides (the request comes through
   the gauge's `+0xAA`, written elsewhere) is not researched.
5. Assumptions: Re:CoM's 2D time step is 50 units a 60 Hz frame (as the MP bar's fades); a played
   layer added after the held ones at the same priority draws over them (creation order).

### What the in-game test must check (H2b)

1. Take a hit: the whole cluster (rings, segments, labels, face) shakes up and down for 10 / 22 /
   38 frames (by how much HP the hit took), the face grimaces; a second hit during the shake
   neither restarts it nor the grimace.
2. Open the camp / start an event: the cluster goes at once (0.178.1); closing it fades it back in
   over 1/6 s.
3. Low HP (under 54 HP): the green arc's end is soft over its last 0.7°.
4. Log: `kh1 hud: … built …` as before; no `no slide units` line in normal play.

## H4: KH1's command menu look (`hud = kh1`)

7.3's menu (its rules, cursor, fades, tab, request bits) is unchanged; at `hud = kh1` its rows are
drawn with KH1's own pieces instead of Re:CoM's plates. `hud = recom` keeps 7.3's look.

### KH1's rules (Checked (decomp + exe + files) unless tagged)

| what | where |
|---|---|
| Root list at (16, 260) + left margin; a submenu at its parent's x + 11 (7.3) | `0x14027B870` |
| A list: header (`*_fra_top` and the title) at the origin; rows at y + 20 + 22 i (`*_fra_c`; the last row `*_fra_d` + the connector `*_fra_cd`); the cursor row is not drawn as a frame but as the selection (`*_sel_sha`, `_act`, `_gra`, `_bla` + the command's icon) at (x + 11, y + 18 + 22 c) | `0x140278B40` |
| Words at the row's (x + 16, y + 22 + 22 i), the cursor row's at (x + 29, y + 25 + 22 c) (`0x14027A920` adds (16, 2)) | `0x140278B40`, `0x14027A920` |
| The root's pieces 24 units (and texels) wider (`param_10` of `0x14027A600` / `0x14027A9F0` for list type 0; picture 2's panel art runs to texel 125) | `0x140278B40`; Checked (files) |
| Battle (`0x142867364 & 1` → the root record's `+0x1F8` = 0): the root's panels RGB (109, 30, 32), the selection plate (216, 109, 100) (`0x14027A600` replaces the vertex RGB; `0x140278B40`'s `local_168` / `local_118`); the field look is the objects' own colours | `0x14027A0B0`, `0x14027A600` |
| Title: `com_battle` `com_command_start` in battle (blue → (128, 128, 255, 255) → red (200, 80, 80), 2 + 18 frames), `com_command_end` in the field (back to (86, 126, 223)), restarted when the mode changes; × 0.5 while a submenu is open | `0x14027B870`, `0x140278180`, `0x140278B40` |
| Pictures: the root's panels picture 2, the submenus' 3, titles and icons 1 (PC: `-4140.dds` 256 × 384, `-61c0.dds`, per language `<L>_uitex_bin20c0.png`) | `0x140278180`; Checked (files) |
| Words: font 1 (`menufont` of `<lang>/font/`, Theater: 2048 × 2048 32-bit, 32-texel cells, `.cod` 8-byte glyphs: code, x, y, 0, width) drawn at size 14 (`0x14054534C` over the cell, `0x1402D1DB0`) | Checked (files + decomp); the PC path's scale: Assumption (strong: "Attack" = 42 units here, 43 on the owner's screenshot) |

### Native build

- **Data (asset step):** pictures 1, 2, 3 join the atlas (picture 1 per language from its PNG:
  a small PNG reader, `miniz_oxide` from the lockfile); after the motion groups: the root's field
  look and battle look (header, title, row, last row, selection, three icons each), the red
  title's colour keys (played), the Magic list's six, then one group per word and language: KH1's
  glyphs (32-texel cells into the atlas, the PS2 copy at half size) as parts at the font's
  advances × 14 / 32. `kh1-hud.json` `"menu": {"first", "words": {lang: [[id, group]]}}`.
  Message 6 ("Summon") joins the decoded words.
- **DLL:** `win/command_menu.rs` `show_kh1`: per row a plate control (the frame / last frame /
  selection + icon) and a word control (colour = KH1's brightness / 128, alpha the fade), a header
  control for the root and one for the Magic list; KH1's left margin −86. The look is the battle
  one while `player + 0x390` (Re:CoM's battle room) is set; switching rebuilds and plays the red
  title from frame 1. Root row 3 with no context command reads "Summon", greyed (locked as 7.3's
  blank row). (Superseded 0.35.0, § FM context tab: row 3 is never the context command; it
  shows KH1's blank dash until summons.) Nothing new per frame: the rows rebuild only when 7.3's did, plus a look change.

### Owner questions (H4)

1. **Font:** KH1's glyphs are Re:CoM sprite parts in the `kh1hud` set (a group per word and
   language), not a `C2DFnt` font: Re:CoM's font format is not reverse-engineered. (a) keep;
   (b) research `C2DFnt`'s font files and convert KH1's into one.
2. **Icons:** a = Keyblade on Attack / Counter, b = hat on Magic, c = potion on Items (by their
   pictures; KH1's choice not read). Summon has none.
3. Root row 3 shows "Summon" greyed when no context command is in it (the brief's choice); KH1
   shows it only once a summon is learned. (0.35.0: KH1's rule, the blank dash until summons;
   § FM context tab.)

### Not 1:1 / to check

1. KH1's slide-in of a list (the objects' C keys −11 → 2 → 0 over 4 frames) is not played
   (as 7.3).
2. The 13 KH1 adds with the root's width (`param_9`, to `0x1428669A0` / `B0`) is not applied:
   its meaning (UV or clip) is not settled. Assumption: the art at +24 texels is what shows.
3. A space in a word advances 9 texels (no glyph in the font). Assumption.
4. The PC atlas's glyph texels are doubled (PS2 range in the font). If the words look too bright
   or dark on screen, that is why.

### What the in-game test must check (H4)

1. `hud = kh1`, field: the KH1 menu bottom-left as the owner's KH1 screenshot (blue in the field):
   COMMAND tab, Attack highlighted with the Keyblade icon, Magic / Items / Summon below in KH1's
   font; Summon greyed.
2. A battle: the tab and panels red, the title turning red over 1/3 s; back in the field: blue.
3. Magic open: the purple Magic list at x + 11 with its header, the root's title dimmed.
4. A chest: row 3 slides in reading Open; Triangle opens it once in (7.3's rules). (0.35.0: the
   green Triangle tab over the header instead; § FM context tab.)
5. `hud = recom`: 7.3's Re:CoM plates as before.

## H5: LOCK ON! and the target's HP gauge (top left)

### KH1's rules (Checked (decomp + files) unless tagged)

| what | where |
|---|---|
| The plate: `lock` objects 5..7 (`lockon_plate_a/b/c`, picture 0) and 8..10 (`lockon_text_a/b/c`, picture 1) at (left margin + 16, 24); on a lock their colour keys restart and play (alpha 0 → 150 / 250 over 2 frames, → 100 / 150 over 10), then hold; when the lock ends alpha − 1/6 a frame | `0x1402747F0`, `0x1402740F0` |
| The gauge's origin (left margin + 16, 24), + 28 while the plate shows, (65 / 80 in minigames) | `0x140275000`, `0x1402743E0` |
| `g_enm` 0 (picture 6) + 1, 2, 3 (picture 1: the green "HP") at the origin; 4 (6) + 5 (1) from x + 28, `L` = min(max HP / 2, 150) wide (their texture stretched); the cap 12 (6) + 13 (1) at x + 28 + L | `0x140275000` |
| Five bars, one per 300 HP: bar k = min(HP − 300 k + 1, 300) / 2 wide while HP ≥ 300 k (texture stretched); with more than one bar, the marker (11) at the top bar's end | `0x140275000` |
| Fade: state 1 draws alpha 0, then + 0.2 a frame to 1; the lock's end: 1, then − 0.2 a frame | `0x1402758B0` |

### Native build

- **Data:** picture 0's corner (texels 96..128, PC 4×) joins the atlas; groups after the menu's:
  the plate (each piece's colour keys, played), the head, the body and the five bars as X-scale
  groups over frames 0..150 (frame = width, as the MP bar's partial segment), the cap, the
  marker. `kh1-hud.json` `"target": {"first"}`. The bars' colours by stack index (green, yellow,
  orange, magenta, violet: objects 10, 9, 8, 7, 6): Assumption (KH1's slot order not read; the
  owner's screenshot shows yellow over green).
- **DLL** (`win/kh1_hud.rs` `lock_tick`, called each of Sora's frames from the request-bits
  wrapper before the command menu): reads Re:CoM's lock (in battle, `*(player+0x3F8)+0xE0`, the
  handle `player+0x84C`, an enemy) and the target's HP (`+0x274`); max HP from Re:CoM's getter
  `0x14017F5A0` (KH1's for proxied enemies, `win/stats.rs`). A lock makes the plate (played from
  frame 1) and the gauge controls (KH1's left margin −86); HP / target changes set the frames
  and the cap / marker places; the fades write the controls' colours for their 5 / 6 frames.
  Hidden at once with Re:CoM's HUD, deleted with the player gauge. Per frame otherwise: three
  reads.

### Not 1:1 / to check

1. KH1 shows the gauge while the lock-on cursor reports HP; here while Re:CoM's lock-on is on in
   battle (Re:CoM has no lock in the field).
2. The minigame heights (65 / 80) are not used (Re:CoM's minigames are its own).
3. When the target changes during a lock KH1 keeps the plate; so does ours.

### What the in-game test must check (H5)

1. Lock on in battle: "LOCK ON!" fades in top-left as the owner's KH1 screenshot, the HP gauge
   under it fades in over 5 frames; its length = the enemy's max HP / 2 (≤ 150).
2. Hit the enemy: the bar shortens; an enemy over 300 HP shows stacked bars and the marker.
3. Release the lock: both fade out (6 / 5 frames).
4. Log: `kh1 hud: lock on: target … HP h/m (…)`.

## H6: Donald's and Goofy's gauges (top right)

### KH1 (Checked (decomp + exe + files) unless tagged)

- Slots 1 and 2 draw with Sora's functions; their objects are `g_c` (32) at absolute places
  around (400, 44), plus the slot offset (Donald `0x140527330` (0, 0), Goofy `0x140527370`
  (68, 20)) and the right margin; the face 48 × 48 (`xa_ex_0040` / `0050` section 2) centred on
  `0x1405272C0` (399, 44). Rings: HP r26 (`0x140527400`, picture 5's `g_c_bar_*_hp` quarters),
  MP r32 (`_mp`), backs `g_c_fra_*` (picture 6); straight parts 17 apart past 54 HP / 9 MP.
- Re:CoM (Checked (decomp + strings)): `Friend::CFriend` (ctor `0x1401D9000`, base dtor
  `0x1401D90F0` run by every friend's), `CFriendDonald::beginStateAppear` `0x1401DDC20`,
  `CFriendGoofy::beginStateAppear` `0x1401DF9C0`, `CFriend::beginStateExit` `0x1401D94D0`.
  Re:CoM's friends have no HP or MP of their own.

### Native build

- **Data:** two held groups (Donald, Goofy): face frame, disc, face. `kh1-hud.json` `"party":
  [donald, goofy]`. Until 0.178.1 they also held the HP and MP backs and full rings; 0.178.2 drops
  them (fork owner, 2026-11-23, answer 4b: Re:CoM's friends have no HP / MP).
- **DLL:** per player gauge two controls at the slot offsets (right margin +86), hidden; four
  wrappers (`sites::KH1_HUD_PARTY_HOOKS`, first bytes checked): a friend's appear shows its gauge,
  `CFriend`'s exit or dtor hides it; Re:CoM's HUD hide hides them. Nothing per frame.

### Owner questions (H6)

1. **What the gauges show** (Re:CoM's friends have no HP / MP): answered (fork owner,
   2026-11-23, 4b): **no rings**, only the face in its frame and disc (0.178.2).
2. Only while the friend is out (decision 39); with Combat = KH1 there are no friend cards
   (decision 4), so they show only at Combat = Re:CoM with `hud = kh1`.

### Not done / to check

1. KH1's partial rings and straight parts for the party (not drawn: no rings since 0.178.2).
2. Draw order of the party's pieces: Sora's (Assumption).
3. KH1's `g_help` / `g_lose` circles (a member called / down) are not used.

### What the in-game test must check (H6)

1. `hud = kh1`, Combat = Re:CoM, a battle with a Donald card: his gauge appears top-right at
   KH1's place when he appears (his face in its frame and disc, no rings since 0.178.2), and goes
   when he leaves. The same for Goofy (68, 20 further).
2. Log: `kh1 hud: Donald out: KH1 gauge shown (H6)` / `gone: gauge hidden`.

## H7: KH1's lock-on cursor in place of Re:CoM's

### KH1 (Checked (decomp + files))

- `0x140274B20`: lock state 3 (a target locked) with sub-state 0 draws `lock` object 2
  (`lockon_c`, picture 10, 38 × 38), sub-states 1 / 2 objects 1 / 0, states 1, 2, 4 object 3
  (`lockon_d`, the orange auto-target ring); centred on the target's projected lock point
  (`0x1402B04C0`, `0x140277510`): the quad at (−w / 2 + kx, −h / 2 + ky), (w, h) from its
  position keys (−4, −4, 46, 46 → 2, 2, 34, 34 → 0, 0, 38, 38, 4 + 4 frames) with its colour keys
  (alpha 0 → 255 → 192), restarted when the kind changes. Not scaled with distance.

### Re:CoM (Checked (decomp + exe))

- `Gauge::CLockonMarker` (ctor `0x140206DF0`, init `0x140208120`): its control `+0x60` (made
  with `0x1402471D0(ctrl, 2)`: a projected control) plays `Cockpit:00` group `+0x80` (0x63,
  0x92 with `+0xF8 & 0x10`, 0x62 with `+0xE8`); its update `0x140209D30` anchors it each frame at
  the lock point `+0x90` (`0x140248570`) and scales it with the distance (1.0 under 3.5 m, 0.5
  past 14 m). Target handle `+0xD0`, found `+0xEC`.

### Native build

- **Data:** picture 10 joins the atlas; one played group: `lockon_c` baked per frame (frame k =
  key time (k + 1) / 2) then held. `kh1-hud.json` `"cursor": group`.
- **DLL:** a wrapper on `0x140209D30` (`sites::KH1_HUD_CURSOR_HOOKS`, `40 57 48 81 EC 80 00 00
  00`): after Re:CoM's update at `hud = kh1`, Re:CoM's cursor layer is set not drawn
  (`0x140247E50(ctrl, 0, 0)` when `0x140248090` says it is), and our projected control
  (`0x1402471D0(c, 2)`, priority 2) gets the same lock point each frame (`0x140248570`); the
  cursor group plays from its start when a lock begins or moves to another target. At `hud =
  recom` Re:CoM's update runs alone (one lock and two loads of ours).

### Not 1:1 / owner questions (H7)

1. Only the locked cursor (`lockon_c`) is used: Re:CoM's marker groups 0x92 / 0x62 (other states
   of its marker) also show `lockon_c`; KH1's auto-target ring (`lockon_d`) has no Re:CoM event
   wired. Owner question: (a) keep; (b) map 0x92 / 0x62 to KH1's other kinds once their meaning
   is read.
2. Per frame while a battle runs: the wrapper copies the lock point (Re:CoM projects and draws).
3. Assumption: Re:CoM's marker layer-add flag 1 and priority 2 suit a projected layer of ours.

### 0.180.0: KH1's orange auto-target ring (`lockon_d`)

The fork owner's 5a ("shown") is now built: `lock` object 3 as its own group (`"auto_ring"`) is drawn
by the same `0x140209D30` wrapper on the soft target while nothing is locked, at `hud = kh1` with Auto
Lock on. H7's lock cursor stays off (`LOCKON_KH1`). KH1's side is in the KH modding workspace's
`knowledge/KH1/kh1-lock-on.md`, and the change is AGENTS.md Stage 322. In game: walk up to an enemy in
battle without R1. The orange ring pops in (38 → 26 → 30, fading to alpha 112) and stays as long as
the swings aim there. It jumps to the next enemy without replaying. It goes away on R1 (Re:CoM's
cursor) and is gone with `auto_lock = off` or `hud = recom`.

### What the in-game test must check (H7)

1. Lock on: KH1's white ring pops in (shrinks from 46 to 34, settles at 38 over 8 frames) on the
   target, the same size near and far; Re:CoM's marker is gone. Switching targets replays it.
2. `hud = recom`: Re:CoM's marker as before.

## H8: KH1's popups where Re:CoM has the event

### What maps (Checked (decomp) unless tagged)

| KH1 popup (task) | Re:CoM event | built? |
|---|---|---|
| LEVEL UP!! box (`lev_s`, `0x140272F10`: Sora's at (left margin + 16, 16), keys, hold 91 frames, fade − 0.125 a frame) | the level up: `0x1401E5730` raises the level, then `0x14020C9A0(Sora, 4)` (CCommonGauge "LEVEL UP!") and sound 0x36 | **yes**: KH1's box replaces Re:CoM's popup at `hud = kh1` (Re:CoM's level-up choice screen stays, decision 39) |
| EXP popups (`exp`: "EXP" + digits over a defeated enemy, `0x1402719F0`) | Re:CoM counts EXP at the battle end (`CPrizeGauge`'s EXP + popup, hidden by H3) | no: KH1's per-enemy floating EXP needs Re:CoM's per-enemy EXP and a projected popup per kill; owner question |
| munny counter (`mani`) | Moogle points (`CPrizeGauge` TOTAL MOOGLE P) | no: a different currency; owner question |
| item / "FULL GET!!" windows (`get*`, `full`) | Re:CoM's card and prize pickups | no: items are M8 |
| HP-orb pickups | Re:CoM's HP prizes heal with its "Recover" popup (`0x14020C9A0` kind 0/1: Assumption) | none in KH1 (HP balls heal silently): Re:CoM's "Recover" stays (H3 kept it) |
| Tech!! (`tech`), "!" bubbles (`feel`), Danger! (`danger`) | none | no |

### Native build (LEVEL UP!!)

- **Data:** `lev_s` objects 6..12 (the box's header and "LEVEL UP!!", pictures 3 and 1) baked per
  frame from their colour and position keys (frame k = key time (k + 1) / 2), held to KH1's frame
  94, faded over 8 frames, then nothing (one played group). `kh1-hud.json` `"level_up": group`.
- **DLL:** a wrapper on `0x14020C9A0` (`sites::KH1_HUD_POPUP_HOOKS`, `48 89 5C 24 10 56 48 83 EC
  20`): after Re:CoM's, kind 4 at `hud = kh1` draws Re:CoM's level-up control at alpha 0 and plays
  KH1's box on a control of ours at (16, 16) with the left margin. Nothing per frame.

### Not 1:1 / to check (H8)

1. Only the box's header is drawn (KH1 lists the stat gains in rows under it; Re:CoM's choice
   screen does that here). Which `lev_s` pieces close a box with no rows: not read.
2. Flipping `hud` back to `recom` mid-room leaves Re:CoM's LEVEL UP! control at alpha 0 until the
   next room's HUD.

### What the in-game test must check (H8)

1. Level up at a battle's end with `hud = kh1`: KH1's red LEVEL UP!! box drops in top-left, stays
   ~1.5 s and fades; Re:CoM's "LEVEL UP!" over Sora does not show; Re:CoM's level-up choice
   follows as before.
2. Log: `kh1 hud: LEVEL UP!!: KH1's box (H8) for Re:CoM's level up`.

## H3: Re:CoM's HUD hidden at `hud = kh1` (visual only)

### What owns what (Checked (decomp + exe: RTTI))

| piece | object | how it hides here | its logic |
|---|---|---|---|
| HP gauge (ring, bar, drain, low HP) | `CPlayerGauge` `+0x78` (`gauge:01`, `gauge:00`) | colour alpha 0 | runs: HP drain, Re:CoM's low-HP sound 9 (≤ 25 %, `0x14020A0E0` case 4) |
| FRIENDS row (label, friend cards) | `CFriendGauge` (vtable `0x1405A6788`, `+0x60`): 3 controls `+0x78`, `C2DCard`s `+0x70` parented to them with `+0x138` = 1 | colour alpha 0 on the 3 (the cards take it) | runs |
| EXP + / NEXT LV / TOTAL MOOGLE P popups and their numbers | `CPrizeGauge` (vtable `0x1405A6718`, `+0x70`, global `[0x14087B690]`): controls `+0x70` ×6, `+0x88` ×6, `+0xA0` ×2, `C2DCard` `+0xA8`; `Cockpit:00` groups 0x51, 0x54, 0x69 | colour alpha 0 | runs (the battle end waits on it) |
| enemy HP gauges | `CEnemyGauge` (vtable `0x1405A66A8`), update `0x1402099B0` | its own hide path: mask bit 0x10 (toggles its layers) | — |
| minimap (+ the area layout it plays, and the room plate: its controls `+0x128`, see § The MP ring built as Re:CoM's HP ring) | `CNaviMap` (vtable `0x14059F2A0`), 3D pieces (`+0x58`, lights) | its own hide path: mask bit 8 (`0x1401E4700` ×3, `0x1401E43E0`) | — |
| kept: "Guard" / "Recover" / "Resist" / "LEVEL UP!" | `CCommonGauge` (vtable `0x1405A66E0`, 7 controls `+0x60`: `Cockpit:00` 0xC, 0xD, 0x8F, 0x3D) | not touched (not on the owner's list) | |
| kept: the level-up choice, the command menu, the camp | | not touched | |

- **Why not the mask for all:** with the mask set, `CPrizeGauge`'s update (`0x14020A710`) drops
  its popup state (`+0x5C` = 0) and `CPlayerGauge` goes to state 7, which stops Re:CoM's low-HP
  sound (`0x1402AB700(…, 9)`). So those three are hidden by colour only. Checked (decomp).
- **Colour:** `0x140248590(ctrl, rgba)` writes `+0x120`; with `+0x138` = 1 Re:CoM's sprite update
  multiplies it in, with the parent's (7.3's notes). The gauge classes never write a control's
  colour (no call of `0x140248590`, no write to `+0x120` in their code); they set `+0x138` = 1
  themselves on some. Checked (decomp). The arrays' counts are checked against the count word
  Re:CoM's array new puts before them.
- **Mask reads** (`sites::KH1_HUD_MASK_CALLS`, contexts `KH1_HUD_MASK_CHECKED`): the five
  `call 0x1401B08C0` go to two stubs (near the exe) that jump to our getters: Re:CoM's value,
  plus the bit while the KH1 HUD is up (an atomic set on the gauge's init and the Config / camp
  refresh). At `hud = recom` the getters return Re:CoM's value unchanged and nothing of Re:CoM's
  controls is written.
- **When:** at each player gauge's init (after Re:CoM's) and after the Config / camp refresh.

### Not done / to check

1. The room plate is Assumed to be `CNaviMap`'s area layout (`0x140245940` at its `+0x114`,
   stopped on bit 8). If it still shows: screenshot, and its owner is searched next.
2. Re:CoM's lock-on cursor stays until KH1's (H7).
3. A `CFriendGauge` / `CPrizeGauge` made later by other code (their alternative ctors
   `0x140206D90` / `0x140206C70` have no caller in the decomp) would show.

### What the first in-game test must check (H3)

1. Log: `kh1 hud: enemy-gauge and minimap mask reads (5) through our getters at …`.
2. **Screenshot** at `hud = kh1` in the field and in a battle: no Re:CoM HP gauge, FRIENDS row,
   EXP / Moogle points, minimap, room plate, enemy gauges; the KH1 cluster and the command menu
   are there; "Guard" / "Recover" popups still show.
3. Win a battle: it ends as before (EXP and prizes counted: the camp shows the new EXP).
4. Low HP: Re:CoM's low-HP sound plays with the KH1 ring.
5. Level up: Re:CoM's level-up choice appears and works.
6. `hud = recom`: everything of Re:CoM's is back as before.

## Logs to send (H1)

- `kh1 hud: kh1hud:00 (632 groups, PC copies ["FR", "GR", "IT", "SP", "UK"]) from …`
- `kh1 hud: Re:CoM registered kh1hud:00 with 632 groups (…); hud = …` on the first room.

## PC textures repacked against the PS2 UVs (0.33.1)

- KH1 PC's `uitex.bin` picture 7 (`UK_uitex_bin10780.dds`, `-10780.dds`) no longer holds the HP
  back segments at `g_s` objects 16 / 18's UVs (8, 83)–(33, 101) / (8, 104)–(33, 122): there the
  PC texture has a 1-texel column and a bracket piece. Picture 5's (`-a2c0.dds`) straight HP
  segment is 24 texels wide at (2, 102) (26 on PS2) and the MP / charge segments are shifted.
  `g_s` (all languages, 2024 dates) keeps the PS2 UVs. Checked (files). Drawing those UVs on
  the PC textures made the straight HP bar show dark brackets every 26 units (owner's
  screenshot, 2026-11-01; reproduced offline from the bake).
- How KH1 PC draws them correctly is not read: Assumption, a UV remap we have not found.
- Picture 2 goes the other way: the PC panels fill the `com` rects (to u 126), the PS2 ones stop
  near u 102. The PC art is kept there.
- The asset step now checks each texel rect a piece draws from a whole picture: where more than
  1 in 20 texels the PS2 picture covers are clear on PC, that rect of the PC copy is redrawn from
  the PS2 texels at 2× (nearest). On the owner's files: 40 rects (picture 5's three straight
  segments, picture 7's two HP back segments, the HP cap and two MP pieces, 5 languages);
  faces, rings, labels and the menu pictures keep the PC art. `KH1_HUD_REFIT_LOG=1` lists them.

## Polish pass (0.34.0, owner 2026-11-01: "the bars are all choppy, the text is wrong, theres no bottom row to the command menu")

- **Words font.** KH1's command words use the system font: `menu/<lang>/sysfont.bin` through its
  PC texture `<LANG>_sysfont_bin0.dds` (1024 × 1024, one file for every Western language),
  glyphs from `exchange/US_font_data_tbl.bin` (`0x1402D1570` loads it unless Japanese / FM).
  Entries are 16 bytes: `+2` u and `+4` v in 1/16 PS2 texels (+8), `+6` advance, `+7` page (low
  nibble: 512 PC texels right; high nibble: 512 down), `+8` the character (U+FF01..FF5E =
  ASCII). Drawn at size 14 over the 18-texel base (`0x14054534C`, `0x140545344`); space 5 base
  texels (`0x142E954F0`). "Attack" = 57 texels = 44.3 units, 44.5 on the owner's KH1 screenshot.
  Checked (files + decomp + screenshot). H4's `Theater` `menufont` was the wrong font (square "A").
  The glyph cell is 36 × 48 PC texels, drawn 14 × 19 units, 2 units up (placed by eye:
  Assumption).
- **Fourth row.** KH1's look always draws the root's row 3 as the last row (`com_fra_d`): the
  context command when there is one, else "Summon" greyed (M8 brings summons). 7.3's rules are
  unchanged; only the drawing adds the row. (0.35.0: the context command is never this row, and
  before summons KH1 shows the blank dash there; § FM context tab.)
- **Seams.** Three causes, all in how the PC atlas is filtered:
  (1) pieces drawn from inside a whole picture mixed in the clear texels around their rect:
  every such rect now has its own atlas box;
  (2) clear texels' colour (black / white) leaked into edges: clear texels now take their
  covered neighbours' colour (alpha unchanged);
  (3) the baked ring quarters left their cut edge's texels clear (GS centre sampling): the PC
  copies now take a texel if its centre or any corner is inside the fan, so neighbouring
  pieces overlap by up to a texel.
  Checked by an offline bilinear render of the bake (no seam left at the quarters or at the
  bar's join).

## FM context tab (0.35.0, owner 2026-11-01: "the menu shouldnt have the context option at the bottom, it should behave like the KH1 final mix version, where the green triangle prompt appears when interacting with something")

7.3 ported the context command as the root's row 3 sliding in under Items. KH1 FM never draws
it as a row: it has its own green tab with the Triangle, over the COMMAND header. Tags:
**Checked (decomp)** = KH1 Steam decomp, **(exe)** = bytes of the Steam exe, **(files)** = the
owner's H0 dump, **Assumption**.

### KH1's rules

| what | where | tag |
|---|---|---|
| The root list's resources: `0x14027AB70(0, &0x142859260, &0x140527B28, uibin_com …)`; the index table at `0x140527B28` is 0..11 (identity), so `[6]` = obj 0 `com_fra_d` (last row), `[7]` = 1 `com_fra_cd`, `[5]` = 2 `com_fra_c` (rows), `[3]` = 3 `com_fra_top`, `[8]` = objects 4..7 (`com_sel_sha_d`, `_act_d`, `_gra_d`, `_bla_d`), `[4]` = 8 `com_command`, `[9]` = 9..11 (the icons); `[0]` = the root's picture (2). | `0x140278180`, `0x14027AB70` | Checked (decomp + exe) |
| The file at `0x14285AD60` is `exchange/<LANG>_uibin_com.bin` (`0x140277FE0`: `"exchange/%s_%s"` with `uibin_com.bin`, `uibin_item`, `uibin_mag`, `uibin_sum`, `uibin_par`, `uibin_i_par`, `uibin_cut`, `com_battle.bin` into `0x14285AD60`, `C560`, `E160`, `0160`, `1D60`, `3960`, `5560`, `6160`). | `0x140277FE0` | Checked (exe disassembly) |
| The list draw draws rows `0 .. count − 2` (the root: 0..2), then, for the root only, the tab (`0x140277CF0` steps it), then the last row (index `count − 1` = 3) with `com_fra_d`, then the cursor row's selection. | `0x140278B40` | Checked (decomp) |
| The tab shows while its "scale" `0x142866D14` > 0. That value is copied to `0x142866A40` / `0x142866A44`, which only multiply the alpha of the tab's vertices and of its word and Triangle: an **opacity**, not an X scale (7.3's reading). It goes 0 → 1 by 1/7 and x `0x140527CA8` −10 → 0 by 10/7 a half-frame step (14 frames), and fades out by 1/7. Not multiplied by the menu's fade (`0x142866A40` is replaced while the tab draws); reset when the fade is 0. | `0x140277CF0`, `0x140278B40`, `0x14027A9F0`, `0x14027A920` | Checked (decomp) |
| Origin: (list x + 9 + x offset, list y − 10) = (25 + x, 250) before the left margin. | `0x140278B40` | Checked (decomp) |
| Pieces: the four `[8]` objects copied, flags `& ~0x46` (no keys), x − 17, y + 2, UVs and widths set in code; the root's picture `*puVar23`; vertex order TL, BL, TR, BR (`0x14027EA50`): k 0 `sel_sha_d` w + 47, UV (2, 150)–(126, 174), its own colour (0, 0, 0, 64) (the shadow; `0x14027A9F0`); k 1 `sel_act_d` w + 47, same UV, RGB (110, 210, 100) × f; k 2 `sel_gra_d` w + 44, UV (2, 135)–(117, 148), top (3, 11, 3), bottom (30, 110, 32) × f; k 3 `sel_bla_d` w + 42, h + 2, UV (2, 126)–(114, 133), top (0, 0, 0), bottom (3, 11, 3). Alphas: the objects' (64, 128, 128, 128) × the opacity. f = 0.5 while a submenu is on top (`param_1[4]` = 0), else 1. | `0x140278B40` | Checked (decomp + files) |
| So the screen rects: shadow (11, 255)–(158, 279), plate (8, 252)–(155, 276), band (14, 261)–(150, 274), stripe (14, 254)–(148, 263) (+ x offset): over the header (260–280), 8 units above it and wider than the menu. | | Checked (decomp + files) |
| Those UVs lie below picture 2's PS2 128 × 128: the art exists only in the PC texture `-4140.dds` (256 × 384): three white, alpha-shaped shapes (stripe, band, the arrow plate) at PC rows 252..349. The colour is all vertex colour. | | Checked (files) |
| Word: entry 3's command (`+0x17`) at `com_fra_d`'s place (0, 0) + (2, 5) + `0x14027A920`'s (16, 2) = origin + (18, 7), brightness `+0x27` × f, alpha the opacity. When entry 3 is empty (0, 0x42, 0x7C or greyed) and row 0 holds Talk or a `0x62C0183` command, the tab shows row 0's word (`+0x24`) and row 0 draws "Attack" at half brightness (the cursor's confirm there is the error sound). | `0x140278B40` | Checked (decomp) |
| Triangle: `0x140279FC0(&0x142853960 + cfg[+0xC] × 0xC0, 0x46, list x + x offset − 0x53 (Spanish − 0x57) + cfg[+0x10], list y − 0x30 + cfg[+0x14])`, cfg = `CLanguageCtrlToUI` (`0x14280CA60`) `+0x20` = `CLanguageControlToUiCommand`: English (`0x14027C8E0`) `+0xC` 2, `+0x10` −2, `+0x14` 22; German (`0x14027C500`) −8; French / Italian (`0x14027C400` / `0x14027C610`) −8; Spanish (`0x14027C7F0`) −13; Japanese (`0x14027C690`) 0. `0x142853960` holds `uibin_cut` objects 0..4 (`0x14027E1C0(&0x140527B58 = {0..4, −1})`); object 2 `cut_ico_a` is at (91, 23). So the Triangle sits at (list x + 6 + x offset, list y − 3) in English (French / German / Italian + 0, Spanish − 9). | `0x14027C1A0`, `0x140278180` | Checked (decomp + exe + files) |
| `0x140279FC0` draws the 2-byte string `0F 46` through the font (`0x1402D2050`, size 14), RGB fixed ×1 (`0x808080`), alpha the opacity (`0x142866A44`): the Triangle slides and fades with the tab and is not dimmed. Escape `0F`: with a pad, `0x1402D4A50(0x46)` = `0x1403EFF90` row 5 (1, 31, 61 by pad type) + 100 → the record at `0x14053B850 + id × 0x14` (w 320, h 384 sixteenths: 15.5 × 18.625 units at size 14, dropped −37 sixteenths); with keys, `0x1402D4BB0` → action `0xE3D3` → the binding `0x1404D8DAA` (default `0xE3F1`). | `0x1402D2050`, `0x1402D4A50`, `0x1402D4BB0`, `0x140138370` | Checked (decomp + exe) |
| The PC's button art is `Image/dt/Theater` `<lang>/font/Icon.tm2` (TIM2 1024 × 1024 32-bit, PS2 alpha; the same file in every language) with `FontIcon.inf` (16-byte header `{1024, 1024, 178, 0}`, 16-byte records `{code, x0, y0, x1, y1}`): Xbox B / A / Y / X = `0xE3F3` / `F2` / `F1` / `F0`, PlayStation circle / cross / triangle / square = `0xE413` / `12` / `11` / `10`. The PlayStation Triangle (green) is `0xE411`, box (450, 11)–(502, 63). The exe's PS2-era records (`0x14053B850`) do not line up with this picture (id 101 lands on empty texels): how the PC maps KH1's records onto it is not read. | | Checked (files); the mapping: Assumption |
| The 4th root row: when row 1 is Magic and the Magic list's last entry is Summon (6, or 0x4F), that word (0x73 / 0xE6 by its grey bit); else the blank command 0x42 (the dash, message 305) at 0x73, 0xE6 under the cursor. Not eased. Summon is appended to the Magic list by `0x1402944C0` once a summon is learned; the Magic list's cursor and draw skip it. Confirm / Right on row 3 opens Summon through it, else the error sound. So before summons KH1 FM shows "–" in the 4th row, not "Summon". | `0x140278B40`, `0x1402944C0`, `0x1402931B0`, `0x1402937F0` | Checked (decomp) |

### Native build

- **Asset step** (`kh1_hud::menu::tab`, format `kh1-recom-assets kh1-hud 3`): after the Magic list's
  groups, group `first + 23` holds the four pieces (one atlas box per UV rect from picture 2's
  PC rows, the PS2 copy at half size; the colours as vertical gradients, Re:CoM's sprite TL / TR
  and BL / BR colours) and `first + 24` the Triangle (a 40 × 48 PC texel cell centred on
  `Icon.tm2`'s `0xE411` glyph, drawn 16 × 19 units); the words start at `first + 25`.
  `"menu": {"first", "tab_icon", "words"}`. Without `Icon.tm2` the Triangle group is empty
  (`tab_icon` false, the build log says so). Format 2 sets are refused by the DLL.
- **DLL** (`win/command_menu.rs`): the tab is `Scene::tab` (`command_menu::TabView`: word, x offset,
  opacity, brightness, dim), never a row. KH1's look: three controls (pieces, Triangle, word) at
  `command_menu::tab`'s places, priority 2, made after the rows (a row rebuild remakes them, so
  they draw over the header and under the Magic list); while it slides only the layout position
  and colours are written (14 frames). Re:CoM's look: the highlighted plate × (1, 1, 0.39) with
  KH1's word in a `C2DFnt`, its top-left (8, 232) (28 above the first row, as KH1's over its
  header), no Triangle (Re:CoM's `Cockpit` has no button glyph we identified).
- **Logic** (`command_menu.rs`): the root's 4th entry still holds the context command (KH1's
  build), the scene's 4th row shows `ROW3_CMD` (the blank command until summons, M8); the cursor
  wraps over four rows; confirm / Right on row 3 is the error sound; Triangle runs the context
  command once the tab is in (7.3's gate).

### Checked / not checked

- Checked (decomp + exe + files): every number above.
- Checked (offline bilinear render of the bake, `UK_` files, gradients): the tab in (green arrow
  plate, dark band, the PlayStation Triangle, "Open") over the COMMAND header; mid-slide (x −5,
  opacity 0.5: the title shows through); dimmed under the Magic list.
- Not checked: in game; a KH1 FM screenshot of the tab (to confirm the place and the Triangle's
  size); the PC's icon remap (the Triangle's cell and size are Assumptions); the Re:CoM look's
  tab (not rendered: its plates are in the MP bar's `Cockpit` set).

### Assumptions

1. The Triangle's PC cell is 40 × 48 texels centred on the glyph (twice a PS2 icon cell), drawn
   16 × 19 units (KH1's quad 15.5 × 18.625).
2. The PlayStation glyph (the owner's ask: "the green triangle prompt"); KH1 PC picks the glyph
   by the pad in use (Xbox Y with an Xbox pad).
3. Controls made later draw over earlier ones of the same priority (as the MP bar's pieces).
4. While a submenu is on top the whole plate's RGB is halved (KH1 halves only the bright
   colours; the dark ones differ by 1–5 levels).

## 0.36.0 (owner, 2026-11-01)

- **HP units.** Re:CoM's HP (cap 560, `0x14020D6A0` / `0x14020D8E0` clamp to `0x230`; Checked
  (decomp)) drawn in KH1's units: × 120 / 560, rounded up (`kh1_hud::hp_units`). Decision 39's
  raw HP left the gauge full above 120 (Sora had 300–560 HP): "losing health doesnt affect the
  health bar". The bar still grows with max HP.
- **Lock-on.** H5 (LOCK ON! + target gauge) and H7 (KH1's cursor) are off (`LOCKON_KH1`):
  Re:CoM's lock-on cursor and enemy HP gauges show again; no LOCK ON! plate (owner's choice).
- **Menu.** The cursor row's word at y + 22 (was 25: 3 units low in the plate, against the
  owner's KH1 screenshot). Battle: the selection's gradient goes black → the panels' red
  (109, 30, 32) (`0x14027A2B0` with `local_108`; Checked (decomp)); it stayed blue.
- **Rings.** The PC copies of the baked arcs sample the PC pictures bilinearly (alpha-weighted),
  as KH1 PC's filtered draw does; point samples of mirrored quarters made the outlines uneven
  and left dark specks (offline render before / after).
- **Crash notes.** `win/crash.rs`: a last vectored handler writes a fatal exception's code,
  place (exe Steam address / DLL offset), registers and return addresses to `kh1-recom.log`,
  then lets the crash go on as before (the owner's crash after a Curaga left an empty
  `CrashDump.dmp`).

## 0.37.0 (owner, 2026-11-01)

- **Pick and choose** ("I want to be able to pick and choose which elements i want"): at
  `hud = kh1`, `kh1-recom.ini` keys `hud_kh1_gauge` (KH1's rings and face; off: Re:CoM's HP
  gauge and the MP bar), `hud_kh1_menu` (KH1's command menu look; off: 7.3's look),
  `hud_kh1_party` (Donald's / Goofy's gauges, drawn with the gauge), `hud_hide_recom_hp`,
  `hud_hide_minimap`, `hud_hide_friends`, `hud_hide_prizes` (Re:CoM's EXP / NEXT LV / Moogle
  pop-ups). Defaults: the owner's picks (gauge, menu, party on; Re:CoM's HP gauge and FRIENDS row
  hidden; minimap and pop-ups shown). The Config menu has no free item slots left (7 of 21 used
  by our 3 rows): owner question.
- **LEVEL UP**: Re:CoM's popup (H8 off, `LEVEL_UP_KH1`).
- **Rings**: the PC copies are drawn at 4 × 4 points a texel, each point sampling the PC picture
  bilinearly; the radial cut edges reach half a texel past the line (no half-covered seams).
  "grainy" edges and the MP outline's varying thickness came from one sample a texel.
- **Tab / selection seam**: the band (`com_sel_gra_d`) starts 1 unit higher, under the stripe.

## Re:CoM's HP gauge at KH1's place, KH1's MP ring around it (owner, 2026-11-01)

- Asks: "okay clearly this isn't working. Lets just keep the KH1 command menu and the triangle
  prompts. For the health bar, is it possible to just get the Re:CoM health bar, place it in the
  same place as the KH1 health bar, and rotate the ReCoM green health so its in the same
  orientation?"; the mockup (ring + bar turned 180° to the bottom right, the bar running left
  along the bottom, the face upright): "Yes, like the mockup"; MP: "Around the ring".
- The baked KH1 gauge (H2 / H2b) stays in the code behind `hud_kh1_gauge` (default now off).
  New keys (default on, only at `hud = kh1` with `hud_kh1_gauge = off`): `hud_recom_hp_kh1_place`,
  `hud_kh1_mp_ring`. `hud_hide_recom_hp` default now off (on, it would hide the gauge entirely).

### Re:CoM's HP gauge (Checked (decomp + exe + files) unless tagged)

| what | where | tag |
|---|---|---|
| `CPlayerGauge` init: control `+0x78` (priority 2), layers 0 `gauge:01` 0xE (the disc behind the face), 1..3 `gauge:01` 7 (max HP), 8 (drain, red), 9 (HP), 4 `gauge:00` 3 (the face; set name from `+0x88`), all flag 1 at x offset −`+0xA0`; frames of 1..3 = trunc(`+0xA8` / 560 × 470); layout position `gauge.lad` element 1 = (60, 60) | `0x140208360` (disassembly: flag 1 in `[rsp+0x20]`, offset in xmm3) | Checked (decomp + exe) |
| The HP setter replaces layer 0 (0xF, 10 at ≤ 25 %) and layer 4 (`+0x88` group 3, 0, or 2 at 0 HP) through vtable `+0x20`; the update `0x14020A0E0` replaces layer 0 (0x10, 0xE, 0xF, 10) and layer 4 and sets layer 2's frame (the drain); no code of Re:CoM's adds layers 1..3 again or moves `+0x78` after the init (CPlayerGauge's functions searched) | `0x14020D6A0`, `0x14020A0E0` | Checked (decomp) |
| Re:CoM's 2D space is 512 × 384 like KH1's (`DAT_140821ADC` = 384, read only); a layout position is (x − 256, y − 192) (`0x140248660`) | exe `.data`, `0x140248660` | Checked (exe + decomp) |
| Groups 7..9: quarter-ring sprites in up to three quadrants with mirrored parts, z rotations (frames 0..270), zero-UV scissor parts, then a 200-unit bar (271..470). The PC copies (`<LANG>_gauge.lad`) differ: 43-unit quarters, the bar a 1-texel column of the ring (0, 0)–(200, −43). The ring starts at 3 o'clock and runs clockwise to 12, the bar along the top to the right (offline render: Re:CoM negates z angles on load, `0x14024B4E0` `^ 0x80000000`) | `gauge.lad`, `UK_gauge.lad` | Checked (files + decomp + offline render) |
| A control's rotation exists (`0x1402486C0` → `+0x100`, z only used: `0x1402EDB10`) but cannot turn this gauge: a scissor part's GS scissor is built from its transformed top-left and bottom-right corners as they are (`0x14024A3F0` → `0x140265F00` → `0x1402643D0` writes `SCISSOR_1`), so a turned scissor is inverted (empty on the GS). The sprite clip test itself takes min / max (no culling of turned or mirrored quads; Re:CoM's own data mirrors parts) | as listed | Checked (decomp + exe) |
| Layer replace (vtable `+0x20` `0x1402482F0`): the new instance takes the old one's place in the current draw list (`0x140249EB0`) and its visibility (old `+0x45`, `0x140247000`); the frame is not kept | as listed | Checked (decomp) |
| Draw order: Re:CoM's 2D sprites go to the foreground display list (`CDispListFG`, 26 buckets): bucket = min(priority, 25), buckets drawn 0 first, FIFO inside a bucket (`0x14024ABB0`, `0x140253F40`, `0x140253AE0`). So a higher priority draws over (the H2 assumption, now checked) | as listed | Checked (decomp) |
| Translations get + 0.25 on load (`0x1402B4A00` → `0x1402ED290`, an add) | | Checked (exe) |
| Prize pop-ups (`CPrizeGauge` `0x1402087C0`): absolute `Cockpit.lad` elements 10 / 16 = (0, 130) / (0, 182) + their own offsets, no parent; FRIENDS row (`0x14020CEF0`): element 22 = (0, 0) + offsets, no parent. Neither is a child of `+0x78`: they stay on the left, upright | as listed, `Cockpit.lad` / `UK_Cockpit.lad` | Checked (decomp + files) |
| The gauge's textures `gauge:00` / `gauge:01` are `gauge.imz` in `FORM/0060..0062` (texture name = set name, `0x140249C90`), not in this session's dump | `forms.txt` | Checked (files listing); the pictures: not seen |

### Native build

- **Data (asset step, `kh1_recom_assets::recom_hp`):** `gauge:01` groups 7, 8, 9 added again as
  47, 48, 49, turned 180° in the data: image parts' corners negated (each corner keeps its UV),
  scissor parts negated and reordered (still top-left → bottom-right), translations and pivots
  negated, angles and scales kept; refused if a group bounces or rotates a scissor (none do). In
  `gauge.lad` and all five `<LANG>_gauge.lad`; Re:CoM's records byte for byte. Offline render
  (owner's `UK_gauge.lad`, frames 60..470, flat colours and a UV-gradient texture): the turned
  groups are pixel for pixel Re:CoM's turned 180° about the origin + 0.25. `kh1-hud.json`
  `"recom_hp"` (format unchanged, an optional key).
- **DLL (`win/kh1_hud.rs` `place_recom_hp`, at the player gauge's init, after Re:CoM's):** checks
  the registry (`gauge:01` = 50 groups) and the vtable slot (`sites::KH1_HUD_PLACE_PTRS`), replaces
  layers 1..3 with 47..49 through the control's own vtable `+0x20` (Re:CoM's x offset, flag 1),
  sets their frames back to trunc(max HP / 560 × 470) (what Re:CoM's init just set), and moves the
  control to KH1's centre: layout (444 + margin + `+0xA0` − 0.25, 317 − 0.25) (margin 86 at 16:9,
  0 at 4:3). Layer 0 (the disc: four mirrored quadrants, symmetric) and layer 4 (the face) are not
  touched: the face stays upright. From then on Re:CoM's own code scrubs, swaps, hides and draws
  the gauge; nothing of ours runs per frame or on HP events for it.
- **KH1's centre:** a circle fit to the outer edge of our baked MP back ring (owner's files):
  (443.9, 317.0), R 57.5; KH1's objects meet at (444, 316), its fans start at (443, 317). The ring
  origin goes to (444, 317) − 0.25 (Re:CoM's translation bias). Checked (offline render).
- **MP ring (`hud_kh1_mp_ring`, only with Re:CoM's gauge at KH1's place; `Cluster` with
  `mp_only`):** the cluster's held groups MP back,
  MP, charge and ticks (KH1's absolute places, size class 2 = KH1's centre), with KH1's MP and
  charge fades, on KH1's MP reads / writes as before; control 0 (MP back, MP, MP fades) priority 0,
  control 1 (charge, ticks, charge fades) priority 1, both under Re:CoM's gauge (2), at KH1's
  right margin. No HP / face / labels / flash / shake; hides and shows at once with Re:CoM's HUD
  state (7 / 1), as Re:CoM's gauge does. No scaling: KH1's MP band is 42–55 units from the centre
  (MP back 40–57), Re:CoM's ring 29–43 (parts and scissors; its texture edge not measured):
  KH1's own HP ring is 29–41 (back 27–43), so the MP ring sits on Re:CoM's ring as on KH1's.
  Checked (files: picture 5 / 7 radial profiles; Re:CoM part rects).
- **Past 9 MP:** KH1's straight MP segments (26 apart, 24 × 12 at y + 43..+ 55) run left along the
  bottom under the turned HP bar (its back reaches + 44), as in KH1 (KH1's HP segments are at
  + 30..+ 41). Built as KH1's; they may run past Re:CoM's bar (KH1: up to 11 segments = 286 units;
  Re:CoM's bar is 200 at 560 max HP).
- **The 7.2 MP bar** is not made while the MP ring is on. With the ring off and Re:CoM's gauge at
  KH1's place, the bar (a child of `+0x78`) is mirrored (X scale −1, running left from the centre)
  and dropped to + 58 (under the turned bar). Assumption until a screenshot.
- Preview: `/tmp/rhp-preview.png` (not in the repo): KH1's real MP ring art, Re:CoM's gauge drawn
  as flat placeholders (no `gauge.imz` in the session).

### Checked / not checked

- Checked (decomp + exe): every address above; the replace function's first bytes and vtable slot.
- Checked (files + offline render): the turned groups = Re:CoM's turned 180° (geometry, scissors,
  UVs) on the owner's `UK_gauge.lad`; the MP ring concentric with the turned ring; nothing off
  screen at 16:9.
- Not checked: in game; Re:CoM's gauge textures (the ring's real band, the bar's look turned); the
  GS emulation's handling of the scissors (unchanged data shape, so as Re:CoM's).
- Assumptions: (1) Re:CoM's ring visual centre is its origin + 0.25 (from the translation bias);
  (2) the MP ring's 1–3 units under Re:CoM's ring edge are hidden by it (KH1 draws its MP back over
  its HP back, then the HP arc over both: only the MP back's inner black edge differs); (3) the
  prize pop-ups and FRIENDS row staying on the left read well with the gauge bottom right.

### Owner questions

1. **Prize pop-ups (EXP + / NEXT LV / Moogle points):** they are not part of the HP gauge and stay
   on the left (Re:CoM's (0, 130) / (0, 182)). (a) keep (built); (b) move them next to the gauge
   bottom right; (c) hide them (`hud_hide_prizes`).
2. **MP straight segments past 9 MP** run past Re:CoM's HP bar when MP is high and HP low (KH1
   does the same with its own bar): (a) keep KH1's (built); (b) cap the drawn MP at 9 (ring only).

### What the in-game test must check

1. `hud = kh1`, `hud_kh1_gauge = off` (keys new: default on). Log: `kh1 hud: Re:CoM's HP gauge … at
   KH1's place, turned 180° (gauge:01 groups 47..49 …)` and `KH1's MP ring around Re:CoM's HP
   ring …`; no `gauge:01 has 47 groups` line (that means the old `FO0002.SPR` is served).
2. Screenshot: Re:CoM's ring bottom right where KH1's is, the bar running left along the bottom,
   Sora's face upright, the blue MP ring outside the green ring.
3. Take hits: the green ring shrinks from the bar's end toward 9 o'clock; the red drain follows;
   the face changes as in Re:CoM; low-HP sound and flashing disc as in Re:CoM.
4. Cast / refill / swing: KH1's MP cells fade, the orange charge line grows on the ring.
5. Camp / events: the gauge and the ring hide and come back together.
6. `hud_kh1_mp_ring = off`: the MP bar runs left under the turned HP bar. `hud_recom_hp_kh1_place =
   off`: Re:CoM's gauge top left as before, the MP bar under it (the ring needs the gauge at KH1's
   place). `hud = recom`: everything Re:CoM's.

## The MP ring built as Re:CoM's HP ring; it follows the gauge; the room plate up top (owner, 2026-11-01)

- Asks (after 0.38.0, "it looks great! few minor things"): "the MP bar doesnt have the outline at
  the bottom of the bar"; "in the video you'll notice that the health bar moves, can we make the
  MP bar move at the same time?"; "the bottom right of the screen shows the type of room. can we
  move this to the top right of the screen?"; "there also just seems to be a resolution
  difference between the MP bar and the health bar, if it's possible to have these two bars
  looking similar quality, maybe built the same way that would be perfect".

### Why the MP ring looked softer (Checked (files + offline render))

- The 0.34–0.38 MP ring was baked: each fan piece rendered by our step into new pictures (4 × 4
  bilinear samples a PC texel, half a texel of slack at cut edges), then filtered again by Re:CoM
  when drawn: two filterings, about one PC texel of blur. Re:CoM's ring is its texture sampled
  once. The pictures themselves have the same density (2 PC texels a unit both).

### How Re:CoM builds its HP ring (Checked (decomp + files) unless tagged)

| what | where | tag |
|---|---|---|
| A sprite whose UVs are all 0 and that is alone in its sprite group becomes a `CScissoring` | `0x14024B910` | Checked (decomp) |
| Its GS scissor = the part's transformed top-left and bottom-right corners `>> 4` (whole 2D units, inclusive; the reset is (0, 0)–(511, 383)), set for the sprites after it in the layer; the layer pushes a reset after its last sprite when it drew a scissor | `0x14024A3F0`, `0x140265F00`, `0x140265ED0`, `0x140248A90` | Checked (decomp) |
| A layer's sprite objects are its group's animations in order (appended at the tail); inactive ones skip | `0x140248F40`, `0x140249150` | Checked (decomp) |
| z rotation: file `+0x40` / `+0x44` negated on load (`+0x58` / `+0x68`); interpolated only when flags 0x20 and 0x100 are clear (0x77ED1); constant at 0x77FD1 (the HP ring's cap); the matrix is scale · rot(rows (c, s), (−s, c)) + translation, row vectors: a positive file angle turns counter-clockwise on screen | `0x14024B4E0`, `0x14024A7E0`, `0x14024BB10`, `0x1402EDB10`, `0x1402ECEB0` | Checked (decomp); with this reading Re:CoM's own group 9 grows clockwise from 3 o'clock in the offline render (the other reading draws nothing in frames 0..90) |
| `gauge:01` 7..9: half / three-quarter ring pictures turned inside quadrant scissors, so the moving end is a picture's straight edge | `UK_gauge.lad` | Checked (files) |

### KH1's ring pictures (Checked (files + decomp))

- KH1's fan (`0x14026B790`, vertices `0x14026E450`) maps one quarter picture onto three quadrants:
  top-left as is; top-right mirrored in u, bottom-right in u and v; the right and bottom halves
  stretched over R + 1 units. MP ring (picture 5, centre (56.5, 56.5), R 55), charge (picture 5,
  (113.5, 56.5), R 55), MP back (picture 7, (107.5, 94.5), R 57); on PC whole texels at 2×.
- The MP and charge cells are separated by clear gaps about 2° wide at every 30°, on PC
  (Checked (files): radial profiles). The MP back is a uniform band: black outline 2 units (4 PC
  texels) at r 55..57 and 41..43, navy between; its ends have no outline (KH1 closes the start
  with its "mp" / "HP" label plates, `g_s` 8 / 9 at y 316..331).

### Native build (asset step `kh1_hud::ring`, DLL `win/kh1_hud.rs`)

- **561 new groups** at the end of `kh1hud:00` (manifest `"mp_ring": {"first", "count"}`): MP back,
  MP, charge (held: frames as the cluster's), MP gain / loss fades (9 + 9), charge gain / loss
  fades (270 + 270). KH1's three quarter pictures are atlas items cropped whole at the PC
  texture's texels; parts are KH1's quadrant rects about KH1's centre (443, 316) (`tr`).
- **MP cells and whole charge cells:** the quarter pictures stay put; a cut between cells is a set
  of scissor rects whose edges run inside KH1's clear gaps. The step computes them from the
  picture's alpha (each texel's angle; filtering spread; Re:CoM's whole-unit cells with the
  + 0.25 translation), merges rows into rects, and counts scissor cells holding texels on both
  sides: 0 on the owner's files (Checked (files)).
- **MP back:** whole quadrants plain; the quadrant where max MP ends: its scissor and the quarter
  turned counter-clockwise by 90° − (30° or 60°), Re:CoM's way. Turned unstretched (about KH1's
  centre; bottom-right about centre + (1, 0)): up to 0.9 unit from KH1's stretched half at the
  end.
- **Closing outline (ask 1):** a 2-unit black bar under the ring's start, r 57..40
  (centre-relative (−57, 0)–(−40, 2)), from one texel of picture 7's black outline (black and
  thickness measured from the picture at build time), drawn last in the MP back layer after a
  full-screen scissor. KH1 draws its 15-unit label plates there; at this layout only the outline.
- **Order inside a group:** plain pieces, then (scissor, image) pairs, then the outline pair; at
  any frame the active animations are whole pairs (unit test).
- **Size:** +6,184 animations (49,396 of 65,535), lad +0.93 MB, atlas unchanged (owner's files).
- **DLL:** the MP-ring cluster takes `mp_ring`'s groups for MP back, MP, charge and every fade
  (`mp_ring::ring_group`); the ticks stay. An older `kh1-hud.json` without `mp_ring`: the baked
  groups, as 0.38.0. The full KH1 gauge (`hud_kh1_gauge`) keeps its baked rings.

### What is not KH1's (scissors are whole units and axis-aligned)

1. The charge's end inside a cell (1° a unit): a staircase of 1-unit steps across the 4.5-unit
   orange band (up to half a unit off a radial line), and KH1's 0.7° fade at the end
   (`0x14026B790` mode 4) is gone. The charge fades' pieces end the same way.
2. The MP back's end quadrant turned unstretched (≤ 0.9 unit).
3. KH1's fan centre is one unit below the quadrant corner (`fan` vertex 0): its affine pieces shear
   the picture by up to 0.3 unit near the band; the parts don't.
4. The ring's visual centre is now KH1's (443.75, 316.75) after Re:CoM's + 0.25, Re:CoM's HP
   ring's (444, 317) (fitted to the old bake): a quarter unit apart.

### The MP ring follows the HP gauge (ask 2)

- Owner's video (30 fps, 1440p): on a hit the HP ring, the disc and the face move together
  (−5, −12, −11, −3, −3, 3, 7, 8, 5, 2, −1, −3, −2, −1, 0 px, x only) while the MP ring and the
  command menu stay (Checked (video)). None of the gauge's groups has a translation key
  (`gauge:01` 10, 14..16, 47..49; Checked (files)), so the move is the HP control `+0x78`'s
  transform. The code that moves it was not found (Assumption).
- Built: the MP ring's controls (and their pooled fade controls) hang from `+0x78`
  (`0x140248650` → `+0x130`, as the 7.2 MP bar and Re:CoM's Dark gauge), their positions made
  relative to `+0x78`'s at the build (`+0xF0`). `+0x138` stays 0: their colour is their own
  (Re:CoM's gauge code never colours `+0x78`; H3's colour hide of the HP gauge does not hide the
  ring). Hide / show stays ours on Re:CoM's HUD state. A child reads its parent's matrix when
  drawn (`0x14024BB10`); if the parent updates later in the frame the ring trails by a frame
  (Assumption, as Re:CoM's own Dark gauge).

### The room plate under the minimap (ask 3)

| what | where | tag |
|---|---|---|
| The plate (room card, name, star): `CNaviMap` `+0x128`, two `C2DSeqCtrl` (count word 2 before them), made by the init `0x1401E39D0` when the room has a card | `0x1401E39D0` | Checked (decomp) |
| `0x1401E44A0(navimap, area set, group)`: clears [0], adds group 4 (appear) or 5 (held) of `<world>_navi.lad`, attaches the room name (`0x140164CA0`), places [0] at layout (0, 0); [1] too for rooms with the extra object `+0x120`, which hangs from [1]. Called only by the minimap update `0x1401E4700` | `0x1401E44A0`, `0x1401E4700` | Checked (decomp) |
| The plate's place is in the navi layouts' groups (`tr_navi.lad` … in `FORM/3001..`) | `0x1407C59F0` table | Checked (exe); the files are not in this session |
| On the owner's 1440p screenshot: plate y 321..382 units, x from 396 to the right edge; minimap down to y 114 | screenshot | Checked (screenshot) |

- Built: `hud_room_plate_top_right` (default on; only at `hud = kh1` with Re:CoM's gauge at KH1's
  place, `place_wanted`): a wrapper on `0x1401E44A0` (`40 55 56 57 41 56 41 57 48 81 EC 80 00 00
  00`, Steam exe) calls Re:CoM's, then puts both controls at layout (0, −200): the plate's top at
  y 121, under the minimap, its x Re:CoM's. Once per plate set-up, nothing per frame. A setting
  change shows at the next room. Assumption: the same place in every world's navi layout.

### Checked / not checked

- Checked: tests (assets: cut rects keep whole cells, staircases, pair order, turn direction; DLL:
  manifest, group mapping, the ini key); the Windows type-check; the owner's files (cuts: 0
  conflicts); offline render (`/tmp/mp-ring-preview.png`, `/tmp/mp-ring-zoom.png`, not in the
  repo) with Re:CoM's scissor and rotation rules: KH1's art at its own sharpness, the outline
  under the start, the max-MP end straight, cell cuts clean, the charge staircase visible at 12×.
- Not checked: in game; Re:CoM's gauge texture (placeholder colours in the preview); the PC
  renderer's scissor edge exactly as modelled (whole units, inclusive); the shake's code; the
  plate's place in other worlds.

### Owner questions

1. **The charge's end inside a cell** (a 1-unit staircase, no 0.7° fade): (a) keep (built); (b)
   that one partial cell from pre-cut copies of KH1's art at its own texels (KH1's exact edge
   and fade, no resampling; 270 small atlas boxes).
2. **The room plate's height:** (a) under the minimap at y 121 (built); (b) another place
   (say where).

### What the in-game test must check

1. Log: `KH1's MP ring … built as Re:CoM's HP ring (groups N..) … hung from Re:CoM's HP control`,
   `CNaviMap room plate 0x1401E44A0 wrapped`, `room plate (group 4) under the minimap`.
2. Screenshot at 1440p: the MP ring as sharp as the green ring; a black line closing the MP
   ring's bottom at 9 o'clock; the room plate top right under the minimap.
3. MP 1..9 and max MP 1..9: whole cells, the MP back's end a straight edge; charge filling (look
   at its end); casts and refills fade as before.
4. Take a hit: the MP ring moves with the green ring and the face.
5. `hud_room_plate_top_right = off`: the plate bottom right (next room).

## 0.41.0: the MP ring draws; one centre with the HP ring; ticks; room plate top left (owner, 2026-11-01)

Asks: "the blue of the MP bar doesnt show"; "lets also make sure that all the MP segments are
evenly spaced"; "the MP bar should also be the same thickness as the HP bar, and the bars should
start from the same place, (the MP bar is currently starting a few pixels above where the HP bar
starts)"; "the room indicator should be in the top left of the screen".

- **Why only the MP back drew** (Checked (decomp); in game: Assumption until seen): Re:CoM draws
  a control's later layers relative to its first layer's first live sprite. `0x140247850` marks
  a control's first layer (`+0x47` = 0); in `0x14024BB10` that layer's first sprite stores its
  matrix × the control's at the node `+0x40`, and every sprite of a later layer (`+0x47` = −1)
  multiplies by node `+0x40` instead of the control's own matrix (when `+0x7C` ≠ 0). The ring's
  sprites carry the ring centre as their translation (for the turned quarters), so the MP (layer
  1 of control 0) and the ticks (layer 1 of control 1) were drawn a second (444, 317) away, off
  screen; the charge (control 1's first layer) only drew when charged. The baked groups of 0.38
  had zero translations, so the same layout drew. **Fix:** one layer a control: MP back, MP,
  charge, ticks on four controls (priorities 0, 0, 1, 1; all hung from Re:CoM's HP control).
- **Evenly spaced segments:** KH1's quarter picture has gaps at 30° and 60° only; KH1 marks 90°
  and 180° with two tick objects (`g_s` 37 / 38) over the ring (the owner's KH1 video: a divider
  every 30°). Those are the ticks layer that did not draw. New ring group `TICKS` (562 groups):
  the two ticks centred on this ring's seams.
- **One centre:** Re:CoM's HP ring is a circle about (444, 317) (`recom_hp::CENTRE`); KH1's fan
  draws its top-left quadrant about (443, 316) and stretches the other halves by one unit, so the
  MP ring sat one unit up-left of the HP ring on its left and top (thicker by one unit there,
  and its start one unit above the HP ring's). Now the ring's quarters are r × r about (444, 317)
  (the right and bottom edges are where KH1 draws them), and the HP control's origin is
  (444, 317) itself (0.38–0.40: 0.25 up-left), so both rings get Re:CoM's +0.25 and share a
  centre. Thickness: MP back r 43..57 outside the HP ring's r 29..43: both 14 units. The
  straight segments past 270° move one unit right with the ring's 6 o'clock end.
- **Room plate top left** (`hud_room_plate_top_left`, default on; 0.40.x's
  `hud_room_plate_top_right` is the same switch): the plate's round left end 3 units in from the
  left edge, its top 3 units down: move = (−m + 3 − (305.2 + m), 3 − 321), m = the player gauge's
  margin (`+0xA0`). Assumption: the plate is right-anchored by m (seen at 16:9 only).
- Checked: decomp (layers, scissors), tests, offline render of the rebuilt ring (concentric with
  the HP ring's 29 / 43 circles, dividers every 30°, start on the centre line). Not checked: in
  game.

## Stage 7.12: the shortcut panel in `kh1hud`

Five groups after the command menu's words (`kh1-hud.json` `"shortcut": {"first", "icons"}`, an
optional key: the format stays 3): the panel (`UK_uibin_cut.bin`'s `cut_fra_a` from picture 0,
`cut_shortcut` from picture 1) and the PC's PlayStation glyphs Triangle / Square / Cross / Circle
(`Icon.tm2` `0xE411` / `0xE410` / `0xE412` / `0xE413`). Drawn in both menu looks (owner,
2026-11-01). KH1's rules and the DLL side: `recom-m7-command-menu.md` § 7.12. Checked (decomp +
files + offline render).

**The pad in use** (a tester with an Xbox pad saw the PlayStation glyphs). Re:CoM's own button
glyphs follow its pad type: the controller config (`0x14034E930` → `0x14083F574`) `+0x1C`, which
`0x140349120` reads to fill its glyphs from `0xE3F0` (type 1), `0xE410` (2) or `0xE430` (3), each
set Square's place, Triangle's, Cross's, Circle's; any type other than 2 and 3 reads as 1
(Checked (exe): `sites::COMMAND_MENU_CHECKED`). In `Icon.tm2`, `0xE3F0..` are the Xbox X / Y / A / B
and `0xE430..` a generic pad's left / top / bottom / right (Checked (files)).
- Asset step (`shortcut::pad_glyphs`): 8 more groups after every other group (no group before them
  moves), type 1's Triangle / Square / Cross / Circle places, then type 3's; `kh1-hud.json`
  `"shortcut": {…, "pads": <first>}` (optional: older manifests have none).
- DLL (`ShortcutGroups::glyph`, `win::command_menu::PAD_TYPE` = `0x14083F590`): the panel's glyphs
  and the context tab's Triangle are picked by the pad type when they are made (type 2, or no
  `"pads"`: the PlayStation groups as before). With the keyboard in use the pad type's glyphs stay
  (Re:CoM shows the bound keys; not done here).

## Combat = Re:CoM: Re:CoM's HUD (0.102.3)

Owner, 2026-11-04: "with combat set to RE:CoM the hud should also go back to the normal positioning".

- `win::kh1_hud::active()` also needs Combat = KH1. At Combat = Re:CoM, Re:CoM's HUD is shown as Re:CoM places it,
  whatever `hud = kh1` and the `hud_*` pieces say:
  - Re:CoM's HP gauge stays at its own place (not KH1's, not turned);
  - the room plate stays where Re:CoM puts it;
  - nothing is hidden (FRIENDS, prizes, minimap, HP);
  - no KH1 MP ring, gauge or command-menu look.
- A switch takes effect where each piece is set up: the HP gauge at its init, the room plate at its set-up. In
  practice that is the next room or battle.
- **Side effect:** the KH1-look Donald / Goofy gauges for Re:CoM's friend cards (H6, `hud_kh1_party`) showed only
  at Combat = Re:CoM, so they no longer show. They were off in the owner's ini.
- Not checked: in game.

## 0.102.4: the MP ring's 90° / 180° dividers and its straight part (owner, 2026-11-04)

Owner (screenshots, `hud = kh1`, `hud_kh1_mp_ring = on`, `hud_recom_hp_kh1_place = on`,
`hud_kh1_gauge = off`): "there is a segment line at the top of the MP bar that is too narrow,
differently drawn, and extends past the bar"; "once the curve of the MP bar flattens, the
thickness of the bar very slightly increases".

### KH1's data (Checked (files + decomp) unless tagged)

| what | where | tag |
|---|---|---|
| KH1's fan: left / top edges at centre − R, right / bottom at centre + R + 1, apex (centre x, centre y + 1) = (443, 317) with the ring's texture centre. From the apex the bottom half is drawn 1:1 (57 units for the back's 57), the top half over 58 | `0x14026E450`, `0x14026B790` | Checked (decomp) |
| The MP back's PC art (`UK_uitex_bin10780.dds`, the same visible texels in all five languages) spans r 40.5..56.5 of its quarter: black 40.5..42.5 and 54.5..56.5, navy (24, 28, 42) between; the outermost PC texel row (56.5..57) is clear (alpha 13 / 255). The MP cells (`-a2c0.dds`) span r 42.5..54.5 | picture 7 / 5 | Checked (files) |
| KH1's 30° / 60° gaps in the MP cells: clear within ±1 unit of the gap's centre line, half covered at ±1.5, whole by ±2.25, the same at every radius (parallel sides) | `-a2c0.dds` | Checked (files) |
| KH1's ticks (`g_s` 37 / 38): 2 × 14 units, navy (24, 28, 42) with one black texel at each end (PS2 picture 7 (70, 97) / (74, 97); clear on PC, so redrawn from the PS2 texels by the 0.33.1 refit), at (444, 260) / (486, 316). So KH1's own tick is narrower than its gaps: 2 units against 3 at half coverage (2 fully clear) | `UK_uibin_g_s.bin`, picture 7 | Checked (files) |
| KH1's straight pieces' art is flush with its rects: back segment 17 at 358..374 (black 358..360, 372..374), MP 32 at 360..372, charge 33 at 368..372; KH1's arc bottom half ends at 373.5 (back) / 371.5 (MP). So in KH1's own data the straight part sits half a unit below the arc it continues | `g_s`, pictures 5 / 7 | Checked (files + decomp); in KH1 on screen: not seen |

### Causes (Checked (offline render of the owner's files, Re:CoM's draw rules))

1. **The 12 o'clock divider** was KH1's tick 37 drawn as it is, centred on our seam: x 443.25..445.25,
   y 260.25..274.25 (with Re:CoM's + 0.25). Our ring (one centre with Re:CoM's HP ring, unstretched,
   0.41.0) has its art at y 260.75..276.75 there: the tick's top stuck 0.5 unit out past the outer
   edge (alpha 0.95 beyond r 56.4), its 1-unit black ends cut the ring's 2-unit outlines (navy notch
   at 261.25..262.75), it stopped 0.5 unit short of the cells' inner end (274.25 against 274.75),
   and it was 2.0 units dark against the gaps' 2.7–2.9 (luminance < 60). The 3 o'clock tick (r
   42..56) did not stick out but had the same narrow, hard look. KH1's tick fits KH1's own
   stretched top (outer edge 259.5) within half a unit.
2. **The straight part**: arc bottom 357.75..373.75 (back) / 359.75..371.75 (MP) / 367.75..371.6
   (charge); straights 358.25..374.25 / 360.25..372.25 / 368.25..372.25: the same thickness (16 / 12
   / 4), half a unit lower. Under Re:CoM's HP bar, which hides both tops, the straight part reads
   half a unit thicker (the owner's second screenshot: its bottom one pixel lower than the arc's).

### Built (asset step only)

- **Dividers** (`ring::dividers`, group `TICKS`, same frames: max MP past 90° / 180°): a strip of
  the MP back's own art on each seam, ±3 units (12 o'clock: centre-relative (−3, −57)–(3, −40);
  3 o'clock: (40, −3)–(57, 3)), each texel the back's radial profile at that edge, alpha × (1 − the
  cells' coverage across KH1's 30° / 60° gaps, measured from picture 5 at build time), on the
  back's opaque texels only. Over the cells it composes what a gap shows; it adds nothing past
  the back's art (offline render: 0 alpha added beyond r 56.4 or inside 40.6). Dark width 2.6–2.9
  units, the 30° gaps 2.7–2.9. Two new atlas items (12 × 34 PC texels, one for all languages).
  Not KH1's tick: KH1's own tick is 2 units, narrower than its gaps; owner's ask ("looks like the
  others").
- **Straights** (`ring::lift_straights`): the ring's back segments, cap, MP and charge segments
  get copies of their atlas boxes with half a unit (one PC texel) of clear texels above and below,
  their rects one unit taller at the top: their art moves up half a unit and lines up with the
  arc (offline render: 357.75..373.75 / 359.75..371.75 / 367.75..371.75, as the arc). Parts and
  translations are whole units, hence the texture shift. The PS2 copy (the base `kh1hud.lad`,
  used only without a language copy) keeps its old place. Not KH1's data (KH1 has the half-unit
  step); owner's ask. The full KH1 gauge (`hud_kh1_gauge`) keeps KH1's ticks and straights.
- DLL: unchanged (same groups, frames, manifest format). Atlas 556 items (was 550).
- Asset step: must be re-run.

### Not checked

- In game; Re:CoM's HP gauge art (the offline render draws it as a flat placeholder); the PC
  renderer's sampling exactly as modelled (bilinear, texel centres); KH1's look on screen (only its
  data and draw code: whether KH1 PC itself shows the half-unit step or its tick narrower than its
  gaps was not seen in a KH1 screenshot).
- Assumption: the charge's 30° gaps fade like the MP cells' (the dividers use the MP cells'
  profile over the charge line too, as KH1's ticks cover it).

## Re:CoM's HP gauge follows a live switch (0.102.5)

Owner, 2026-11-04: "when switching combat from recom to KH1, everything changes except the HP bar, and vice
versa" (screenshot: Combat = KH1, KH1's MP ring bottom right, Re:CoM's HP gauge still top left).

- **Cause:** the gauge was turned and moved only at its init (`place_recom_hp`, from `gauge_made`). A switch in the
  camp ran `mp_bar::refresh` → `kh1_hud::refresh`, which rebuilt the ring and hides but not the gauge. Checked (code).
- **Change** (`win::kh1_hud`):
  - `refresh` now places or unplaces each live gauge at once.
  - Placing records Re:CoM's layout position (`0x140248660` keeps it at `+0xF0 / +0xF4` as x − 256, y − 192;
    Checked (decomp)).
  - Unplacing (`unplace_recom_hp`) puts Re:CoM's own groups 7, 8, 9 back on layers 1..3 through the same layer
    replace. Their frames are the max-HP frame on layer 1 and the current HP on layers 2 and 3. Then it restores
    the recorded position.
  - A live place sets layers 2 and 3 to the current HP, not the max, as the init does.
  - A gauge's init or dtor forgets its entry.
  - Nothing per frame.
- The room plate still moves at its next set-up (it isn't re-placed live).
- Not checked: in game.

## The turned HP ring's start: no gap above the black line (0.141.1)

Owner, 2026-11-05 (screenshot, 720p, `hud = kh1`, Re:CoM's HP gauge at KH1's place, HP in the first
quarter of the ring): "there's a little gap in the green hp bar where the bar starts, lets fix that".
On the screenshot the green stops two pixels (about one unit) above the black start line at 9 o'clock,
and the background shows through across the whole band (r 29..43), dark track included. At full HP (the
owner's 1440p camera recording, the ring and the bar) there is no gap.

### Cause (Checked (files + decomp + screenshots); the PC rounding: Assumption)

| what | where | tag |
|---|---|---|
| `gauge:01` groups 7 / 8 / 9 (track, drain, HP): frames 0..270 draw 0..270° of the ring (one degree a frame) as rotating pictures under quadrant scissors; 271..470 the whole ring unclipped and the bar. The start's black line is group 7's first animation, part (0, −2, 43, 0) of sprite 16, drawn before any scissor: on Re:CoM's way round above the 3 o'clock line, turned (−43, 0)..(0, 2) under the 9 o'clock line | `UK_gauge.lad` (all five copies the same), `gauge.lad` | Checked (files) |
| The scissors' edge at the start is one unit off the line: (0, 1)..(43, 44) and (−43, 1)..(43, 44) in Re:CoM's groups; turned (−43, −44)..(0, −1) and (−43, −44)..(43, −1) | same | Checked (files) |
| A scissor's rows are its corners plus the translation (+ 0.25) floored, inclusive (`0x14024A3F0` `>> 4`; SCISSOR_2 by `0x1402643D0`); the PC port makes the rect (x0, y0, x1 − x0 + 1, y1 − y0 + 1) (`0x1402EC4E0`) and scales it to pixels with `ceil(v · s + o − 0.5)` (`0x1402EAC30`) | as listed | Checked (decomp) |
| So the art's centre (the start line) is 0.25 off whole units: on Re:CoM's way round the ring starts 0.75 unit below its line (seamless on the PS2's integer sampling), turned it stops 0.25 unit above it | `recom_hp::tests::the_start_scissor_misses_the_start_line_either_way` | Checked (arithmetic on the files' numbers) |
| On the owner's 720p screenshot the gap is 2 pixel rows (≈ 1.07 units), and on the 1440p recording the sprites sit about 0.6 unit lower than `c · 3.75` would put them: the PC port's sprite and scissor rounding adds to the 0.25 | the screenshot; `kh1-camera.mp4` frame 30 | Assumption (measured, the code path not traced) |

Not the cause: the MP ring (its art and closing outline reach the line; Checked (screenshot)); the
0.102.4 dividers / lifted straights (elsewhere on the ring); a UV seam (the full ring has none); the
ring's start angle (the picture covers the start at every frame ≥ 1; Checked (files)).

### Built (asset step only, `recom_hp::start_filler`)

- Each turned group (47, 48, 49) gets one more animation, **first in the group** (before any scissor, so
  unclipped): the ring's own start-quadrant picture from the group's unrotated full-ring animation (part
  (0, 0, q, q), q = 43 on PC, 41 / 43 in the PS2 copy), cut to its first 2 units (a new sprite: the same
  texels and colours, the UV rows of those units: PC v 10..14 of 10..96) and turned with the group:
  part (0, 0, −q, −2), frames 4..=270, the full ring's record (flags 0x77FF1, no rotation).
- Its bottom edge is the picture's own straight edge at the start line, where the black line and KH1's
  MP ring outline start: what the full ring (frames 271..) draws there. Above, it lies under the
  scissored ring (the same art).
- From frame 4: the first degree whose arc covers 2 units at the inner edge (29 · sin 4° = 2.02), so it
  never shows past the ring's moving end. Frames 1..3 (under 0.7 % of 560 HP) keep the old start.
- 2 units, not 1: covers the measured ≈ 1.07-unit gap with room for the PC rounding.
- Re:CoM's own groups 7..9 are unchanged (Combat = Re:CoM draws them); the DLL is unchanged (same
  groups 47..49, same frames).

### Checked / not checked

- Checked: tests (`recom_hp::tests::the_start_filler_closes_the_gap`,
  `the_start_scissor_misses_the_start_line_either_way`, `owner_gauge_files` on the owner's
  `FO0002.SPR` and its five `<LANG>_gauge.lad`); offline render of the turned groups with Re:CoM's
  rules (whole-unit inclusive scissors, + 0.25, z angles negated) and placeholder art (no `gauge.imz` in
  the session), with the sprites shifted 0 / 0.5 / 1.0 unit against the scissors: before, a gap of 0.25
  / 0.75 / 1.25 unit at frames ≤ 270, none at 271+ (as the screenshot and the recording); after, none at
  frames 4..470, with every offset.
- Not checked: in game; Re:CoM's real ring art over the 2 filler units (assumed radially uniform, as the
  screenshots show it).
