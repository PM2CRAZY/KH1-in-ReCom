# Riku in Reverse/Rebirth with KH1's movement (fork branch `Re-Re-Reverse/Rebirth`)

Testing work on the fork, not the original project: the design doc's rule 6 ("Riku's story stays
as it is, for now") is lifted here only behind `riku_kh1_movement` (default off).

Fork owner's direction (2026-11-22): KH1 Riku's boss moves would make him too strong; use Re:CoM's
Riku and his own animations, with KH1's movement system as much as possible ("we have every
animation already in KH recom, we would need just to rearange the animations and the system").

## Findings

| What | Where | Tag |
|---|---|---|
| Per-character action tables `0x1407C6300[player+0x398]`: chars 1/2 Sora `0x1407C5E80`, 3/4 Riku `0x1407C6370`, 5/6 Dark Riku `0x1407C67F0`. Riku's rows 0x00..0x38 mean what Sora's mean (idle 1, walk 2, run 3, jump 4..7, roll 0x16 `p002_056`, hit reactions, stance rows 0x2A..0x2C, swing rows 0x30..). Rows 0, 0x1B..0x1E (Sora's glide) and 0x39..0x46 are `p002_000` (no clip). | exe | Checked (exe) |
| Riku's Re:CoM skeleton `PLAYER/0101/PL0101.MDL` (102 bones) is KH1's Riku: bone 1 `ex_1010_sk` (KH1 `xa_ex_1010`). The model file's first word is the first part's offset: 0x10 for Sora, 0x20 for Riku (`PL0101`, `PL0111`, `PL0201`). | files | Checked (files) |
| KH1 files on Riku's 227-bone skeleton: `xa_ex_1010` (41 motions, AI script), `xa_ex_1011` (1 motion), `xa_ex_1019` (no mset), `xa_ex_1560` (39 motions, AI script), `xa_ex_1568` (12 long motions). `xa_ex_1010` / `1560` are KH1's Riku battle sets (requests 0..7 movement, 72..111 reactions, 200+ attacks). Their entry 6 / 38 is an empty motion whose channel-table offset is 0xFFFFFFFF (the mset reader now skips it). | files | Checked (files); which fight each is: Assumption |
| 9 Re:CoM Riku clips are exact bakes of KH1 Riku motions (`p002_005`, `006`, `007`, `037`..`041`, `043`; ≤ 0.03°). No KH1 Riku attack has a Re:CoM bake. | files (asset matcher) | Checked (files) |
| `CPlayerRiku` vtable `0x1405A8868`: Battle exe `+0xB8` = `0x140216B10` = `jmp 0x1401F9840` (the shared Battle exe); Field exe the shared `0x1401FA0F0`; roll exe `+0x1C8` = `0x140212350` (Sora's `0x140217310`); glide `+0x1A8` / `+0x1B0` `CPlayer`'s defaults; draw weapon `+0xF8` = `0x1402160A0`. Battle begin / end / Wait / Hide are his own. | exe | Checked (exe: vtables + disasm) |

## Built (0.144.0, `crate::riku`, `win::movement`)

At `combat = kh1` with `riku_kh1_movement = on`, for the main `CPlayerRiku`:
- the shared movement hooks (move exe, stick class, move step, jump init, avoid init, jump exe)
  run KH1's movement frame for him, as for Sora;
- rows: only those with a clip in his table (`riku::row_ok`); any other request keeps the current
  motion (logged per row);
- abilities: Dodge Roll (always Re:CoM's own roll: his roll exe runs whole, unhooked) and High
  Jump when Re:CoM's ability word has its higher jump (`+0x41C & 0x200000`); no Glide, Guard,
  Counterattack or combo abilities;
- his attacks stay Re:CoM's: the confirm press is not given to the KH1 frame, so Re:CoM's move exe
  dispatch / the jump exe's air commands play his cards; no KH1 effects or sounds variants;
- no Enemy Skills, command menu, items or specials; gravity KH1's own.

No new hook, patch or site (one vtable value compared). Nothing new per frame outside his frame.

Assumptions (to check in game): Re:CoM's move exe dispatch plays Riku's cards before its move step
as for Sora; Dark Mode changes nothing the KH1 frame relies on; KH1 Sora's jump numbers on Riku.
Not checked: in game; Riku's handlers against the Re:CoM decompile (not on this machine).

## Built (0.145.0): KH1's Attack for Riku, his empty deck, cardless enemies; the Settings list

Fork owner, 2026-11-22: "pode ser mais agressivo e já implementar o combate"; and the camp's
Settings had no Combat / Movement pages in Riku's story.

- **Settings list** (`keyblades::top::layout(.., riku_settings)`, `sub::RIKU_SETTINGS_ROWS`): with
  `riku_kh1_movement` on, Riku's camp Settings opens Game Settings, Combat, Movement (Effects &
  Sound and Camera stay Sora's; Riku's story keeps Re:CoM's camera). The map-only camp unchanged.
- **Attack** (`win::attack`, `win::movement::kh1_fighter`): the request bits (Re:CoM's field branch
  in battle: confirm = Attack, no card input; a jump wins), the use-card button (skipped), initAttack
  → the field swing, the field swing init / exe, and the row pick run for Riku as for Sora. The row
  pick is `CPlayerRiku`'s vtable slot `0x1405A8990` (Re:CoM's `0x1401F7520`, the same value as Sora's
  slot; patched with it, checked old value). KH1's rows 1-7 play action rows 0x30..0x36, all Riku's
  own swings (`p002_044`..`p002_050`). Checked (exe: `0x1407C6C70` rows 1-7). A row with no clip of
  his is refused (`riku::row_ok`).
- **Hits**: Riku's swings hit through Re:CoM's own weapon volume (`weapon+0x240`, Re:CoM's pair
  test; not KH1's Keyblade spheres, which read Sora's Keyblade bones). A hit whose owner (`body+0x98`)
  is Riku's or his weapon's actor id lands during an enemy's attack (the card-interrupt bit, decision
  14). Enemies stay hittable while attacking (decision 26 extended to his battles).
- **Damage**: Re:CoM's card AP at both Damage settings: his Soul Eater card (0x12) at the KH1 row's
  column (`win::damage::swing`). No KH1 numbers, Enemy Skills, Berserk or KH1 effects / sounds for him.
- **Deck** (`win::deck`): his battle revolver is the empty one (0 cards + Reload), its HUD hidden,
  no Friend-card drops. **Enemies** (`win::battle::manager_ctor`): his battles are KH1 battles (P1-P5:
  enemies attack without cards, several at once).

Lost with the deck (expected): Riku's card breaks, Dark Mode (DP from cards), sleights, Duel.

Assumptions: Riku's weapon is a `CPlayerWeapon` with its volume at `+0x240` and Re:CoM's swing exe
turns it on in the KH1 row's hit window as for Sora's; the waiting body's owner is the player's or the
weapon's actor id; KH1 Sora's combo timing on Riku's clips looks acceptable (his clips' lengths differ
from Sora's). Not checked: in game.

## 0.145.1: Riku's hit effect

Fork owner, 2026-11-22: "os ataques estão sem efeitos/feedback quando acertam" (damage landed).

- Cause (Checked (exe + files)): `CPlayerWeapon::onAttack` `0x1402250B0` takes the hit effect from
  Sora's attack-row table by the attack id (`0x1407C6C72 + 16 × weapon+0x23C`) into the effect table
  `0x1405A1480` (24-byte rows: name, id). KH1's rows wrote Sora's ids 1..7, whose effect is
  `ex_5010` (Sora's Keyblade set, `EFFECT/1000/EF1000.EFF`, not loaded in Riku's story).
- Riku's own swings are rows 0x97..0x9D of the same table (action rows 0x30..0x36, the same columns
  as Sora's 1..7), effect `weapon` 12 (0 for the finishers 4 / 7), in his `EFFECT/1500/EF1500.EFF`
  (`ex_1580.dpx`, `weapon.dpx`, …). His Dark Mode rows 0x9E..0xA4 use `ex_1580` 1 / `weapon` 1.
- Fix (`riku::attack_id`): Riku's swing writes his twin id (0x96 + id) into `+0x630` / `+0x634` /
  `+0x638`; the KH1 row (windows, `+0x3C8`) is unchanged. Assumption: his hit sound rides on that
  effect as in Re:CoM.

## 0.146.0: Riku's KH1 MP (first part of option (b))

Fork owner, 2026-11-22: option (b) (a "Dark" list of his sleights paid with KH1 MP, shortcuts),
and "seria interessante, atribuir uma barra de mana dele já que não terão as cartas".

- `win::mp::riku_live` / `live`: Riku's KH1 MP in the same save block (`save+0x2675..`) of his
  story's save; `active()` stays Sora's only (AP Up, abilities, items, shop, notices, menus).
  Max MP: KH1's 3 + level bonuses on Re:CoM's level (Assumption: the level read is Riku's in his
  story), no Keyblade bonus. Charge from his KH1 hits (KH1's row `+0x2C`, no-twin level), the
  refill step in his request bits, the save-point refill.
- The MP bar (`win::mp_bar`) is built for him too, parented to his HP gauge as for Sora.
  Assumption: it lands near his gauge (Re:CoM builds his Dark gauge in the same place); not seen.

## 0.147.0: Riku's command menu, his Sleights list and shortcuts

- **His sleight ids** (Checked (files + exe)): `SY0002.CTD` (`@CTD` at 0x40, 112 u32 name offsets at
  0xB0 from base 0x270; row = id + 1, as Lucky Bounty LV3 0x56 and Sonic Blade 9 confirm): Dark Break
  0x5F, Dark Firaga 0x60, Dark Aura 0x61, MM Miracle LV2 / LV3 0x62 / 0x63, Holy Burst 0x64, Inverse
  Burst 0x65, Impulse 0x66, Maelstrom 0x67, Barrage 0x68, Dark Impulse 0x69, Dark Maelstrom 0x6A,
  Dark Barrage 0x6B, D Mode 0x6D. (My first reading, line order of `strings`, was wrong.)
- **His sleight start** `0x140216AB0` (vtable `+0x130`) takes 0x62..=0x64 (with `0x14020E590`, King
  Mickey's) and 0x66..=0x6B, then `+0x138` (his `initStockSkill` `0x140215C60`, cases 0x62..=0x68
  load their packs `0x140211790` and set `+0x638`), `+0x838` = 10; refuses the rest (0x5F..=0x61,
  0x65). Checked (exe: disasm).
- **Built** (`crate::riku::SLEIGHTS`, `crate::sleights` accepts them, `win::command_menu::riku_world`,
  `win::summons::begin` with his start `RIKU_SLEIGHT_START`): his command menu (Re:CoM's look: the KH1
  HUD look stays Sora's) with Attack, Magic / Items greyed, and Sleights: Impulse 2, Maelstrom 2,
  Barrage 3, Dark Impulse 3, Dark Maelstrom 3, Dark Barrage 4 MP (ours), battle only, started by his
  own sleight start, paid with his KH1 MP. The L1 shortcut list: his slots (the same save bytes),
  filled with his first three sleights while all are empty (no Customize page for him).
- Assumptions: the six start outside Dark Mode (Re:CoM's Riku plays the Dark ones in Dark Mode);
  their packs load in his battles. Re:CoM's "learned" answer for each is logged once
  (`riku: Re:CoM's sleight-learned test`); the list doesn't filter by it (Re:CoM's Riku forms his
  sleights from cards).

## 0.147.1: the Camera setting for Riku

Fork owner, 2026-11-22: "as opções de câmera podem ter para o Riku tbm". `win::camera::init`'s
Riku's-story rule (Re:CoM's camera) holds only with `riku_kh1_movement` off; on, the Camera setting
(Re:CoM / KH1 / KH2) applies to him (the cameras read the player through `cam+PLAYER`, not Sora's
pointer). His Settings list gets the Camera page. Not checked: in game.

Also found (the owner's 0.147.0 log): after an OpenKH Build (which deletes `mod/Recom`), the DLL's
copy of the generated files lands after Panacea listed the folder, so that one session serves
Re:CoM's own `FO0002.SPR` / `EF1000.EFF` (no MP bar, no command menu, no KH1 effects, for Sora too);
the next launch has them (stage 257's rule).

## 0.148.0: Riku's HUD file gets the menu's plates and the MP bar's groups

Fork owner, 2026-11-22 (screenshot: Riku in the field, no command menu, no shortcuts). His log, even
on a second launch: `mp bar: Cockpit:00 has 157 groups, not 162 … no MP bar` and `kh1hud:00 is not in
Re:CoM's 2D registry`.

- Cause (Checked (files)): Riku's story loads its HUD from `FORM/0071/FO0071.SPR` (`gauge.lad`,
  `Cockpit.lad` 152 groups, `layout_card.lad`, `duelmode.lad`; language copies 157), not
  `FORM/0002/FO0002.SPR` (151 / 157), and the asset step patched only `FO0002`. The menu's plates and
  the bar's groups were never in his `Cockpit:00`. (`FORM/0070` is another cockpit, 148 groups, whose
  neutral texel is transparent: not patched.)
- `FO0071`'s atlas has Re:CoM's text plate, the magic plate's colour, the lock-on triangle and the
  opaque neutral texel at `FO0002`'s UVs (the patch finds them by UV in every copy). Checked (files:
  all six copies).
- Fix (asset step `step::riku_form`): the same groups added to `FO0071.SPR` and its language copies,
  written as `raw/FORM/0071/FO0071.SPR`; `kh1-mp-bar.json` `first_groups` gets 152 (157 was there).
  Not fatal when it fails. The step's hash changes: every install rebuilds once.
- The KH1 HUD look (`kh1hud`, in `FO0002`) stays Sora's: Riku's menu is Re:CoM's look.

## 0.149.0: Riku's KH1 HUD, the KH1 menu look, his Customize page

Fork owner, 2026-11-22 (screenshot: Re:CoM's HP gauge top left, the 7.2 MP bar, the menu in Re:CoM's
look): "falta poder configurar os shortcuts igual da com o Sora, e ainda o command menu não aparece
na tela igual do Sora, além disso a HUD ta com o mesmo problema que está com a do Sora".

- `FO0071.SPR`'s `gauge.lad` `gauge:01` equals `FO0002`'s (47 groups, the HP ring groups 7..9 and the
  whole set byte for byte, in the container and its five `<LANG>_gauge.lad`); `FO0071.RTM` has
  `Cockpit.imz`, `layout_card.imz`, `duelmode.imz` and the five language copies of the first and last.
  Checked (files).
- Asset step (`step::riku_hud`): the KH1 HUD set and the turned ring added to `FO0071.SPR` as to
  `FO0002.SPR` (the turned groups start at 47 in both, so `kh1-hud.json`'s `recom_hp` holds for
  both); `kh1hud.imz` added to `FO0071.RTM` (`raw/FORM/0071/FO0071.RTM`). Riku's sleight names
  (`RIKU_WORDS`) join the menu's words, so the KH1 look draws them.
- DLL: `kh1_hud::active` is on in Riku's story with `riku_kh1_movement`; the gauge's face is Re:CoM's
  own (`gauge:00`), Riku's in his story, so no picture is swapped. The KH1 menu look needs `kh1hud:00`
  registered in his story (else Re:CoM's look).
- Customize: his Settings list's first row (moved to the top list's entry 0 in 0.150.0); picks = his six sleights + None; his slots are the block's
  shortcut bytes of his story's save; `crate::unlocks::SHORTCUTS_SET` in the block's spare byte marks
  them set up (no refill of emptied slots).
- Assumption: his Dark gauge (Re:CoM's second bar) doesn't clash with the gauge moved to KH1's place
  (not seen). Not checked: in game.

## 0.150.0: Customize in Review Decks' place; the King's sleights in the Party list

Fork owner, 2026-11-22: Review Decks has no use without cards (as Sora's); healing "igual no caso do
donald e do goofy"; "o que tiver na gameplay do Sora tem que ter o equivalente adaptado a do Riku".

- Camp: `top::Layout::Riku` (Combat = KH1, `riku_kh1_movement`): entry 0 relabelled Customize, opened
  by our confirm (as the KH1 list's Items); Re:CoM's help for entry 0 replaced by Customize's.
- Party: Mickey Miracle (MM Miracle LV3, 0x63) and Holy Burst (0x64), page 2. Available where
  Re:CoM's Friend-card roll gives the King's card in Riku's story: its rows 14..25 hold only card
  0x38 (chance 125 ‰, weight 100), row 23 (world 10) none. Checked (exe: the table, checked at start).
  Taught in his story without Re:CoM's learned bit. Started by his sleight start, battle only, with
  the card-battle object test as Sora's.
- Assumption: MM Miracle is the King's heal (its pack's behaviour not read); the King met bit 0x80 is
  set in Riku's story (the log's `party: this battle` line shows `friends met`).

## Research (2026-11-22, no code): Re:CoM's Dark Mode is a second player, Dark Riku

Fork owner, 2026-11-22: a Dark Points system (gain / loss tables), and a "Dark Form" skin unlocked at
the first Dark Mode.

- Player groups `0x1405A6910[char]`: 1 → 1, 2 → 11 (Sora, Halloween), 3 → 101, 4 → 111 (Riku),
  **5 → 201, 6 → 211 (Dark Riku)**. Checked (exe). `PL0201.MDL`'s skeleton is `ex_1580_sk` (KH1's
  Riku-Ansem, dark suit with coat tails `l_skt*`), clips `p003_*`; Riku's `PL0101` is `ex_1010_sk`,
  `p002_*`. Checked (files). Different skeletons: Riku's clips don't play on the Dark model.
- RTTI `CPlayerDRiku` (vtables `0x1405A50E8`, `0x1405A52E0`); ctor `0x1401FF880` (stores its Riku in
  `+0x8A0`). Checked (exe).
- The player factory `0x14020EC70(char, parent, second)`: chars 1/2 → `0x140216CE0` (Sora), 3/4 →
  `0x140212030` (Riku), 5/6 → `0x1401FF880` (Dark Riku); `second` = 1 stores it at PLM `+0x68`, else
  `+0x60`. Checked (exe: disasm).
- Riku's code at `0x14021223D`: when `+0x8A0` is empty and his ability word `+0x41C & 0x4000000` is
  set, it makes char `+0x398 + 2` (Dark Riku) as the second player and keeps it at `+0x8A0`. Checked
  (exe).
- Dark Riku's switch-in (around `0x1402047B8..0x140204860`): copies from its Riku (`+0x8A0`), shows
  itself (`+0x154 & ~1`), plays a motion and **writes itself to the main player pointer
  `0x14087B380`**. Checked (exe: disasm, the function's bounds not read). So in Dark Mode the main
  player is `CPlayerDRiku`: our Riku handlers (`is_main_riku`, vtable `0x1405A8868`) don't run on it.
- Dark Riku's action table `0x1407C67F0`: idle 1, walk 2, run 3, jump 4..7, roll 0x16, stance
  0x2A..0x2C, swings 0x30..0x36 with clips; no clip on 0, 8..0xD, 0x17..0x1A, 0x1C..0x1E, 0x39..
  Checked (exe).
- Re:CoM's DP come from card breaks; with the empty revolver they never rise, so Re:CoM's own Dark
  Mode can't start at Combat = KH1 (Assumption from Re:CoM's design; the DP field not found yet).
- Owner's decisions (2026-11-22): Dark Riku gets the same KH1 adaptation as Riku (gameplay faithful
  to each: Dark Mode plays as Dark Riku); Dark Mode starts by our DP tables; after the first Dark
  Mode the Dark model becomes a skin the player turns on / off in the menu, Dark Mode or not.
- `CPlayerDRiku`'s vtable against Riku's / Sora's (Checked (exe)): Battle exe `+0xB8` =
  `0x1402068D0` (`jmp 0x1401F9840`, the shared one); Field exe `+0xA0` = `0x1402068E0` (`ret`: he
  exists only in battle); **his own** row pick `+0x128` `0x140204F80`, sleight start `+0x130`
  `0x140206830` (ids 0x5F..0x6B through a jump table), initStockSkill `+0x138` `0x1402042F0`, jump
  init `+0x1B8` `0x140204060`, jump exe `+0x1C0` `0x1402000F0`, roll exe `+0x1C8` `0x1401FFC30`,
  `+0x198` `0x1402046D0` (Riku's `0x140215FB0`): the switch-in that writes `0x14087B380`. Riku shares
  Sora's jump init / exe; Dark Riku does not: KH1's movement needs his own handlers wrapped.
- Dark Riku's swing rows in `0x1407C6C70`: 0x9E..0xA4 (action rows 0x30..0x36, effect 0x15 / 0x16,
  column byte 3), after Riku's 0x97..0x9D. Checked (exe); which class reads them: Assumption.

## 0.153.0: Riku's Guard blocks (our front test after Re:CoM's) and cancels magic shots

Fork owner, 2026-11-22: "a defes do Riku deve poder fazer parry e cancelar o ataque do oponente
caso for uma magia!"; "percebi que a defesa dele não está sendo efetiva contra nada, precisa ser
efetiva tal qual o KH1 é".

- **Found** (Checked (the owner's 0.152.0 log)): Riku's Guards started (two `ParryMiss` lines) but
  no block ever happened: no `guard reactions: block` line, no guard success. Re:CoM's body guard
  (`0x14016F1C0`) never returned 2 for him. Its test (Checked (exe: disasm)): `body+0x9C & 1`, the
  attack's element bit in `body+0x8C` (the body ctor `0x14016F886` sets all), the body owner's
  actor `+8` set, its bone 0 (`0x1402ABB20(actor, 0)`) `+0x50` clear, then
  `|wrap(atan2f(d.x, d.z) − yaw)| ≤ body+0xB0` with d = attack point (`coll+0x60`) − bone 0
  `+0xD0` (`0x14023E6E0`), yaw = mover `+0x54` (`0x1401A4A00`), wrap `0x14016EAA0`. Which of these
  refuses on Riku is **not known** (no decomp in the session; Riku's clip `p002_043` is 28 frames at
  30 fps = 56 game frames, so KH1's live frames 10..36 do run: Checked (files)). The first 40 hits
  while he guards now log those fields (`riku guard:` lines).
- **Change** (`win::attack::riku_guard_block`, `crate::riku::guard_faces`, `win::guard::{riku_block,
  cancel_shot}`): in the damage-check wrapper, a hit that arrived on the main Riku / Dark Riku while
  his KH1 guard object is live, not blocked by Re:CoM, is KH1's block when the attacker (the owner
  the apply wrote to `body+0x98`: an enemy or its shot) is within ±100° of his facing by Re:CoM's
  own angle rule. Then Re:CoM's protection bit for that check (no damage, no reaction), KH1's
  kind-6 success on his next frame (recoil, clink, `GuardSucceeded`: perfect parry / no missed
  parry), and the attacker's reaction: melee as Sora's (Re:CoM's flinch / KH1's clash recoil, with
  Guard reactions on, one frame later than Sora's).
- **Magic**: a blocked shot (`CBullet` / `CHomingBullet`) is **cancelled** for Riku (attack off
  `0x14014DE80`, life `+0x160` = 0): KH1's generic code makes a blocked shot inert. Sora's Guard
  still turns shots back (decision 28). Same when Re:CoM's own block fires for him.
- Assumptions: the attacker's position stands for KH1's hit point in the cone; a plain `CBullet`'s
  life is at `+0x160` (its attack is off either way); the caster itself isn't interrupted.

## 0.153.1: his own action-6 exes run KH1's frame

- Cause of "not effective" (Checked (the owner's 0.153.0 log + exe)): the Guard / recoil are
  Re:CoM's action 6; for Riku its exe is `CPlayerRiku +0x1C8` = `0x140212350` (Dark Riku
  `0x1401FFC30`), never wrapped, so no KH1 frame (guard object, block, parry) ran while he guarded.
- Fix: both wrapped as Sora's `0x140217310` is (same first 16 bytes): his Re:CoM roll keeps his exe,
  everything else in action 6 runs the KH1 frame.
- A shot within 1 m of him is judged by its caster's position for the front test.

## 0.154.0: Riku's Dark Form skin (Dark Riku's model, Riku's own moves)

Fork owner, 2026-11-22: "ele vai desbloquear a skin do Dark Mode e vai poder jogar com ela no modo
sem estar com darkmode ativado, testa com ela liberada já de começo"; "pode colocar a skin mas os
movimentos ser o normal mesmo".

- **0.152.0's blocker avoided**: the Dark mesh (`PL0201.MDL`, multi-bone packets our MDL reader
  refuses) is not rebound. The skin is Dark Riku's model and textures as they are, and Riku's
  motions are moved onto Dark Riku's skeleton instead.
- **Skeletons** (Checked (files)):
  - Riku `ex_1010_sk` has 102 bones; Dark Riku `ex_1580_sk` has 128.
  - Every body bone has the same name in both. Rest positions differ by up to 8 cm, and the
    bones' rest axes differ (bone 1, `hip`, `waist`, the Soul Eater chain), so channels can't be
    copied.
  - Dark-only bones: the coat tails `*_skt*`, `l_uparm2` / `r_uparm2`, and `wep1` / `wep2`, which
    stand for Riku's `k10` / `k11`.
  - Riku-only bones: the belts, cuffs and pants.
- **Retarget** (`kh1_recom_assets::riku_skin`):
  - Rotation, per frame: `U_dark = R_dark · R_rikuᵀ · U_riku`, where `U` is a bone's world
    rotation in the frame and `R` its rest world rotation.
  - Translation: Dark's rest translations; `Root`'s from Riku (root motion); bone 1 is Dark's
    rest plus Riku's offset from his own rest.
  - Dark-only bones keep their rest pose relative to their parent.
  - Result on Riku's idle (Checked (files, test)): feet and head move by the same amount as on
    Riku, within 3 cm.
- **Events**: `.SES` (bone `+0x08`) and `.EET` (bones `+0x24`, `+0x34`, `+0x44`) name bones by
  index. They are remapped by name, and a Riku-only bone goes to its nearest mapped ancestor.
- **Bone lookups by name** (Checked (exe)): `0x1401A4450` with `wep0`, `ex_1010_sk` /
  `ex_1580_sk` (by char), `chest1`, `WeaponR`, `head`, `l_toes`, `r_toes` (strings
  `0x1407C5E10..0x1407C5E50`). The skin's bone 1 is renamed `ex_1010_sk`.
- **Riku's weapon** has no model group: `0x140224BA0` attaches chars 3..6 to a body bone
  (`0x140224C58`). Checked (exe).
- **Files**: player groups 121 (from 101 / 201) and 131 (from 111 / 211), new names not in
  `Recom.hed` (Checked (files)):
  - loose: `PL01x1.MDL` (Dark's, bone renamed), `.ANN` (Riku's 51 clips retargeted), `.EET`,
    `.SES`;
  - raw (with the PC textures): `.RTM` and `.TXA` (Dark's eye blink).
  - The manifest is `kh1-riku-skin.json`. The names also get placeholders before the first
    launch's build.
- **Table rows** (`crate::riku_skin::add_rows`): each skin group copies its Riku group's 11 rows
  in Re:CoM's PLAYER table (same types and order). The skin's own files replace RTM, TXA, MDL,
  ANN, EET and SES; the camera data `.BIN` (identical in 101 and 201), the sound bank `.SND` and
  the effect links stay Riku's.
- **Switch**:
  - Re:CoM's char → group table `0x1405A6910` (`.rdata`: 0, 1, 11, 101, 111, 201, 211) is read by
    the player ctor (`0x1401E4CDA`, kept at `+0x39C`), the group load (`0x140211A5B`,
    `0x140211AE4`) and the unload (`0x1402115EE`, `0x14021161F`). Checked (exe).
  - With `riku_dark_skin = on` (ini, default on while testing; unlocked from the start), chars 3
    and 4 point at 121 / 131. One checked write at start; nothing per frame.
  - Dark Mode (char 5 → 201) is unchanged.
- **Assumptions**:
  - Dark Mode's Dark Riku (group 201) and the skin (121) share texture names (`n_zz120_*`): both
    loaded together in Dark Mode.
  - The coat tails hang at rest (Riku's clips have no tail motion).
  - Nothing else in Re:CoM's Riku code reads a bone by index.
- **Not yet**: the unlock rule (first Dark Mode, save spare bit 4) and a menu switch.
- **Checked**: tests (`riku_skin::tests` in both crates, the asset step run on the local
  install), the Windows build. **Not checked**: in game.

## 0.154.1: the T-pose fixed (Riku's motion pack name); Costumes on Riku's top list

Fork owner, 2026-11-22 (screenshots: Riku in the Dark model in a T-pose in the field and the camp;
Riku's top list), verbatim: "a skin foi implementada mas ficou um Tpose, lembrando que é não é pra
trocar as animações... além disso … é pra colocar uma nova opção no menu chamada Costumes onde ativa
essa skin, chame ela de Dark Mode, e a normal se chamará Default Attire".

- **Cause of the T-pose** (Checked (exe + the owner's 0.154.0 log: `chars 3 / 4 -> groups [121, 131]`,
  the files served)):
  - A player's action rows find their clips by name in the `.ANN` resource named by a second
    per-char table, `0x1407C6338[char]` (strings "PL0001" … "PL0211").
  - Its readers: the player's state init `0x1401FA7F6` and `0x1401F425C`, both calling
    `0x1402A42B0`, the lookup by resource name of type 0x1D.
  - 0.154.0 moved Riku to group 121 but left the name `PL0101`. No clip was found, and the model
    stood in its bind pose.
- **Fix**: the name pointers of chars 3 / 4 (`0x1407C6350` / `0x1407C6358`, the only pointers to
  `0x1405A145C` / `0x1405A1464` in the image; Checked (exe)) are switched with the groups, to our
  `"PL0121"` / `"PL0131"`. Both writes are checked; a failed group write puts the names back.
- **The moves**: Riku's own clips, unchanged in timing, retargeted onto the Dark skeleton
  (0.154.0). No Dark Riku motion is used.
- **Costumes** (`keyblades::top::RIKU_ORDER`, `menu::Page::Costumes`):
  - Riku's top list is now eight entries at the KH1 list's spacing: Customize, World Map, Map
    Cards, World Cards, Status, D Report, **Costumes**, Settings.
  - Costumes is one more of Re:CoM's entry buttons at camp index 7, run by the same code as Sora's
    KH1 list. The list's order and extra buttons per list are `top::order` / `top::extras`.
  - It opens a settings page (Re:CoM's Config screen with our row) with one row, **Costume:
    Default Attire / Dark Mode** (single-value style; ini `riku_dark_skin`).
  - A change switches both tables at once. It shows from the next room or battle (Re:CoM reads
    them when it builds the player).
- **Assumption**: switching back and forth leaves the other group loaded until Re:CoM unloads it
  (its unload reads the table's group now).
- **Checked**: tests (`keyblades::tests` Riku's eight, `riku_skin::tests`), the Windows build.
  **Not checked**: in game.

## 0.154.2: the Soul Eater held the right way round in Dark Mode

Fork owner, 2026-11-22 (screenshot: Dark Mode skin, the blade turned the other way), verbatim: "por
algum motivo a keyblade do Riku ficou invertida mas o modelo e a gameplay ficaram perfeitas"; "mas
ela só inverteu na Skin Dark Mode".

- **Cause** (Checked (files: rest positions and rotations in the hand's frame)):
  - Riku's `k11` / `k10` and Dark Riku's `wep2` / `wep1` both join `r_hand` to the blade root
    `wep0`, but they are different chains. Riku's goes 17 cm along the hand's −z and back; Dark's
    goes 22 cm along +z and back.
  - Mapped bone to bone (0.154.0), Riku's grip turns pushed Dark's chain the other way.
  - At rest, `wep0` in the hand is the same in both: the same rotation, 1.6 cm apart.
- **Fix** (`riku_skin::Grip`):
  - Dark's `wep0` takes Riku's `wep0` pose relative to the hand: his rotation in the hand, his
    position in the hand's axes plus the rest difference.
  - `wep2` / `wep1` follow `wep0` rigidly, as in Dark's rest pose.
  - The blade segments `k9`..`k1` follow `wep0` as Riku's follow his.
- **Checked**: test `riku_idle_on_the_dark_skeleton` covers every frame of the 51 clips. The blade
  root and tip stay where Riku's are in the hand, worst 2.0 cm (the rest difference). The Windows
  build. **Not checked**: in game.
- Asset step: **re-run** (automatic: the step's hash changed). Only `PL0121.ANN` / `PL0131.ANN`
  change.

## 0.154.3: the Dark blade turned over; the menu's model shows the costume

Fork owner, 2026-11-22 (screenshots: the Dark skin's Soul Eater still the other way; the camp showing
Riku's own model), verbatim: "estranho, ainda etá invertida … ao entrar no Menu quero que a skin
também seja alterada".

- **0.154.2's premise was wrong**: the blade already sat as Riku's in the hand. The rest pose is the
  same in both skeletons (Checked (files): `r_hand` and `wep0` world rest rotations 0.1° / 0.0°
  apart), so the 0.154.2 file drew the same as 0.154.0.
- **Real cause** (Checked (files)):
  - The blade mesh hangs on `wep0` in both models (per-bone bounds: Riku x −0.95..0.16, y
    −0.14..0.10; Dark x −0.96..0.16, y −0.12..0.13). Dark Riku's is modelled turned over about
    the blade's axis (`wep0` x).
  - Dark Riku's own clips hold it so. His grip against Riku's idle grip is 137..178° about
    `wep0`'s x in 12 of his clips; for example, `wep1` turns 180° about z in his idle.
  - Riku's clips don't, so his pose on the Dark mesh shows the blade the other way.
- **Fix** (`riku_skin::BLADE_FLIP`): Dark's `wep0` takes Riku's hand-relative rotation turned half a
  turn about its x. The blade segments `k9`..`k1` (trail and hit nodes) stay where Riku's are in
  the world.
  - Test: every frame of the 51 clips; root position within 2.0 cm, rotation exactly Riku's
    turned over (0.00°), tip within 2.0 cm.
- **The menu's model** (Checked (exe)):
  - `CMenuModel` (vtable `0x140513CB0`, ctor `0x140111F10`, resource id = group + 10000) takes
    its group from immediate bytes, not the char table: the camp's
    `mov dword [rsp+0x30], 0x6F650B01` at `0x1400E4701` (1, 11, 101, 111) and the level-up
    screen's `0x14010EF68` (101, 0, 111, 0).
  - Riku's two bytes in each become 121 / 131 when the player's group is queued (`win::models`'
    group-load wrapper, `riku_skin::sync_menu`), so the menu shows the group that is loaded. A
    costume changed in the camp shows at the next room, in the field and in the menu.
  - No branch lands inside those bytes (Checked (exe)).
- **Checked**: tests, the Windows build. **Not checked**: in game. Asset step: **re-run**
  (automatic): `PL0121.ANN` / `PL0131.ANN`.

## 0.154.4: the Soul Eater hidden where Riku hides it (the menu)

Fork owner, 2026-11-22 (screenshot: the camp's Dark model holding the blade, turned over), verbatim:
"ficou bom mas no menu não precisa ele segurar a espada na mão, além disso no menu ela ficou
invertida, mas o modelo na gameplay ficou perfeito agora".

- **Cause** (Checked (files)): 4 of Riku's clips have scale channels. They scale `wep0` and
  `k1`..`k9` to 0 to hide the Soul Eater (the menu's pose among them; 54 hidden frames), and
  `l_hair3`. The retarget wrote no scale channels, so the hidden blade showed, in a grip Riku
  never shows.
- **Fix**: Riku's local scale is copied bone for bone into the Dark clip (9 channels, flag 2 set,
  only for those clips). Translations stay unscaled.
  - Test: every frame where Riku's `wep0` is scaled, Dark's has the same scale (54 frames hidden).
- **Checked**: tests, the Windows build. **Not checked**: in game. Asset step: **re-run**
  (automatic): `PL0121.ANN` / `PL0131.ANN`.

## 0.155.0: the costume changed in place (no door); the HUD face; Halloween Town

Fork owner, 2026-11-22, verbatim: "importante que quando essa skin estiver ativa na halloween town,
assim como a versão transformada essa skin fica com a variação da halloween town, além disso...
seria possível trocar a skin sem ter que passar por uma porta?" (asked to choose, the owner picked
"Trocar no lugar"); "quando trocar a skin para a darkmode também deve alterar a foto no hud".

- **Halloween Town**: already so since 0.154.0 (Checked (files)).
  - Riku's group in Halloween Town is 111 (char 4), and Dark Riku's is 211 (char 6). Both add
    their own Halloween textures (`n_zz110_11/12`, `n_zz120_11/12/13`, `weapon_11`).
  - Skin group 131 is `PL0211`'s model, and the menu bytes cover 111 too.
- **In place** (`win::riku_skin::{release, rebuild, try_swap}`): Re:CoM never rebuilds a live
  player's model; each step is one Re:CoM runs when it builds an entity's model or a player's
  state, in the same order (Checked (exe: disasm)).
  1. **Release**, as the entity's dtor `0x1401A2440` does:
     - the parts `+0xB0` (one 0x170-byte model per MDL part; count `+0xC0`);
     - the matrices `+0xB8`, the sub-models `+0xC8` / `+0xD0`;
     - the motion controller `+0xD8`;
     - the materials `+0xE0` (array dtor `0x1402894E0`, 0x40 each);
     - the texture animation `+0x1A0`;
     - the EET / SES players `+0x108` / `+0x118`.
  2. **Rebuild**, as `0x1401A4E70` does:
     - `0x1401A5110(e, 10000 + group, 1)`;
     - `0x1402A4D70` → `0x140178620` → `0x140178F00` → vtable `+0x10`;
     - `0x140179420` / `0x140179A30`;
     - `0x140238460` / `0x140238710`.
     - The parts bind to the entity's transform node `e+8` (created once, `0x1402AC000`), and the
       motion controller takes its bone count from that node (`0x14024E09C`).
  3. **The player state init's model steps** (`0x1401FA6E0`):
     - `0x1401A3990(e, −1, −1)`, `0x1401A6B40(e, "asiato", 0)`, `0x1401A6B70(e, 1)`;
     - the group `+0x39C`;
     - the action rows' clips bound again by name, `0x1401F4240(e, char)` (the motion pack name
       table already points at the costume);
     - the bones by name, `0x1401A4450`: `wep0`, `ex_1010_sk`, `chest1`, `head`, `l_toes`,
       `r_toes` at `+0x3A0..0x3B4`.
  4. **Riku's weapon object** (`+0x3D0`): its bone `+0x238`, and `0x1402ABD20(w, 1, bone
     matrix)`, which reuses its node and only re-points it.
  5. **The idle row's motion.**
  - **When**: at the camp's close (`win::models::camp_field`), else at Riku's first frame once the
    group is loaded (queued when the costume is chosen). Only for `CPlayerRiku` as the main player,
    in the field (`+0x390` = 0), in actions 1..4. Never in Dark Mode or battle.
  - **Checks**: 21 callees' first 8 bytes are checked before the first change; any mismatch keeps
    the next-room change. The log gives the transform node's matrix count before and after.
- **HUD face** (`win::riku_skin::face`): Re:CoM's own Dark Mode gauge switch (`0x14020BC60`,
  Checked (exe: disasm)) sets the gauge's face set `+0x88` to `"gauge:02"` (`0x1407C7AB0`;
  Riku's is `"gauge:00"`, `0x1407C7AA0`) and replaces its control's (`+0x78`) face layer 4 with
  group 3, or group 0 at ≤ 25 % HP. Later face changes read `+0x88`.
  - With Dark Mode chosen, the same two steps run on Riku's gauge (`player+0x3E0`), checked in his
    move exe (one pointer compare a frame). Only from Riku's own set or ours; never while the
    gauge's Dark state (`+0x90 & 8`) is set.
- **Assumptions**:
  - Nothing else keeps a pointer into the old model (bone-attached effects alive at the change: none
    expected at the camp's close).
  - The parts' bind resizes the transform node (the log shows it).
  - `gauge:02` group 3 is Dark Riku's face in Riku's `FO0071` (Re:CoM's own switch uses it).
- **Checked**: tests, the Windows build. **Not checked**: in game. Asset step: unchanged.

## 0.155.1: the in-place change's transform node sized again

Fork owner, 2026-11-22 (screenshots: Dark model in a T-pose after the change; Riku's own model
garbled after changing back), verbatim: "é bugou mesmo, mas no próprio jogo em teoria já existe uma
transição... quando o personagem usa a transformação o próprio jogo recarrega a forma não seria
possível utilar essa mesma transição?"

- **Cause** (Checked (the owner's log + exe: disasm)):
  - The log shows `transform node 102 -> 102` going to Dark Riku's 128 bones, and `128 -> 128`
    going back to Riku's 102.
  - The parts' bind `0x140270030` grows the entity's node only through `0x14023EBE0`, which
    allocates only at count 0 (`0x14023EC10`) and never resizes.
  - The motion controller takes its bone count from the node (`0x14024E09C`), so it posed the
    wrong count: the T-pose, and the garbled model.
- **Fix** (`riku_skin::clear_node`): before the rebuild, the node's matrix array is released as
  the node's dtor does (`0x14023DE20`):
  - each element's `+0x9C` is cleared;
  - the array goes to a 0x20-byte deleter (vtable `0x140605B38`) for Re:CoM's deferred free
    `0x14028F800`, since the draw thread may still read it this frame;
  - pointer and count are set to 0.
  - The bind then allocates it at the new skeleton's count. The weapon object's node gets the same
    treatment before its attach, which allocates it again on the new `wep0` matrix.
- **The transformation**: Re:CoM's Dark Mode doesn't reload a model. Riku's battle player builds a
  second player, `CPlayerDRiku`, at the battle's start (`0x14021223D`), and the switch only changes
  the main player pointer (`0x1402046D0`, `0x14087B380`). Dark Riku exists only in battle (his Field
  exe is `ret`), so it can't change a costume in the field.
- **HUD face**: the 0.155.0 log says the face layer was replaced, but the owner's screenshot shows
  Riku's face. The log now names the control's layer count and the set, so we can tell which layer
  holds the face on Riku's gauge.
- **Checked**: tests, the Windows build. **Not checked**: in game.

## 0.155.2: the costume change plays Riku's transformation

Fork owner, 2026-11-22, verbatim: "funcionou perfeitamente!, só que eu realmente queria a transição
quando altera-se a skin de uma para outra, a animação e o efeito".

- **Re:CoM's transformation** (Checked (exe: disasm)):
  - Riku's Dark Mode start `0x140215FB0(riku, 1)` enters action 0x20 (exe `0x140215380`).
  - Its sub-state 1, on the ground, requests his action row 0x37 (`p002_057`, blend −1:
    `0x140215402..0x140215415`). The effects and sounds are that clip's EET / SES events.
  - Sub-state 2 waits for the motion's end (`0x1401A57D0`: controller `+0x50 & 3`), then switches
    to Dark Riku (`0x140206810`).
- **Ours** (`win::riku_skin::transition`):
  - A costume change requests the same row with the same blend; the costume's own EET / SES events
    of `p002_057` play.
  - Riku's move exe is held while it plays: no KH1 frame and no Re:CoM move step replace the
    motion.
  - At its end (or after 240 frames), the model changes in place (0.155.1), then the idle row
    plays.
  - Riku's action 0x20 itself is not entered: its end switches to the battle-only Dark Riku.
- The same as 0.155.0: only plain Riku as the main player, in the field, in actions 1..4. A room
  rebuilt mid-way drops the transition, and the next room's player has the costume anyway.
- **Checked**: tests, the Windows build. **Not checked**: in game.

## 0.155.3: Dark Riku's entry flip and its effects; Riku's way-back effect

Fork owner, 2026-11-22, verbatim: "funcionou!, mas a animação ficou incompleta, não deu tempo dele dar
aquele mortal, também pode ter aquele efeito de tela junto, igual a transição do combate".

- **Re:CoM's transformation is two clips** (Checked (exe: disasm + files)):
  1. Riku's row 0x37 `p002_057`: the crouch, 32 frames at 30 fps. Its events: `ex_1580.dpx` 18
     on bone 0, `transform.dpx` 0 on bone 25.
  2. Then Dark Riku's switch-in (`0x1402047B8..0x14020485B`) plays **his** row 0x37 `p003_067`,
     the flip, 38 frames at 30 fps. Its events: `ex_1580.dpx` 18 on bone 0, `transform.dpx` 1 on
     bone 51.
  - 0.155.2 played only the first.
  - The way back: Riku's `"transform"` effect 2 on his bone 0x1B (`0x1402157CD..0x140215811`,
    through `0x1401A2D50`, the effect `+0x4C |= 4`).
- **Asset step** (`riku_skin::DARK_FLIP`): the skin's pack gets Dark Riku's own `p003_067` (his
  skeleton is the skin's) as one more slot. Re:CoM's lookup by name falls back to a linear search
  when the trailing digits' slot holds another clip (`0x1401A47E0`). The skin's `.EET` / `.SES`
  get his records for it. **Re-run** (automatic: the step's hash changed).
- **Game** (`win::riku_skin::transition`):
  - **To Dark Mode**: Riku's crouch → the model changes → row 0x37 points at the flip's slot
    (looked up by `0x1402A42B0`, as the rebind does) and plays with blend −1, as Dark Riku's
    switch-in plays it → at its end, Re:CoM's rebind gives row 0x37 its own clip back → idle.
  - **To Default Attire**: the crouch → the model changes → Riku's `"transform"` effect 2 on bone
    0x1B → idle.
- **Not carried**: the switch-in's camera mode 4 (`0x1401FBDE0`, the sleights' close camera) and the
  Dark gauge's own switch. The effects on screen are the clips' events, as in battle.
- **Checked**: tests (the flip and its events in both skin groups' files), the Windows build.
  **Not checked**: in game.
