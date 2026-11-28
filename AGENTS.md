# AGENTS.md: KH1 combat in Re:Chain of Memories

Read first, every session:

1. `docs/design-philosophy-instructions.md`
2. `docs/engineering-rules.md`
3. `docs/tester-requests.md` (its rules win if they differ)
4. The top ~15 stages below.

Work only on branch `testing`. Stages are newest first.

---

## Stage 437 (fork branch Re-Re-Reverse/Rebirth): Riku's first Ansem fight is fought at Combat = KH1 (Ansem at 240 HP), not skipped, version 0.198.38 (2026-11-27)

- **Ask** (fork owner, verbatim, after a tester's "First Ansem fight works now or rather it skips it."): "mas eu realmente não gostaria que fosse um skip, poderia ser uma fight com o ansem mais fácil para as skills do Riku, ela skipa?"; to the plan: "sim!"; to Ansem's HP (120 / 160 / 240): "24".
- **Found** (reverse-engineer, `knowledge/ReCoM/recom-riku-ansem-tutorial.md` § 5, Checked (decomp + files) unless tagged): room 1100's script ends only at state 40 (a won Duel) → `@000` (the 9 cards, clears (1, 0xA) / (1, 7) / (1, 0x28), sets (1, 2)) → `@_ma` waits 40 frames on (1, 2) and calls 0xF8 → `0x140237160` back to 2102. Ansem's card AI `0x140424CB0` runs only while (1, 7) is set, else `CHumanBoss`'s `0x140199E10`; the lesson boxes come only after a Rapid Break. His damage `0x14017DAB0` has no tutorial check; his death (`CBossAnsem` vtable `0x1406840F0`, `CBossDeadDemo` `0x140187DC0`) sets (1, 2) in Riku's story but skips `@000`'s cards and clears (Assumption: not seen). Row 0 at B12: Ansem (`EN0122`) 320 HP, attack 3 (2 / 3 / 4 a hit by difficulty), the Guardian (`EN0123`) 320 / 3; no KH1 twin or proxy. A KO of Riku ends nothing in 1050..1198; whether the game over comes there: open.
- **Change** (`story_skip::{RIKU_ANSEM_ID, RIKU_ANSEM_HP, riku_ansem_hp, RIKU_FIGHT_SESSION, RIKU_ANSEM_HELD_AT_1}`, `win::story_skip::{riku_fight_armed, riku_ansem_made, riku_won, riku_frame}`, `win::stats::stat_init`): the request still arms in 2102 → 1100; the battle loads and is fought. At Ansem's stats init his HP (and the gauge's max, `max_hp`) is 240 / 320 of Re:CoM's (difficulty kept), at either Damage setting. Every fight frame: (1, 7) and (1, 2) clear (no card lesson, the script not done). When his HP reaches 0 (or sits at 1 for 120 frames, for Re:CoM's 1-HP hold `0x140180920`, seen not read): the lesson's saved writes ((0, 0x24) |= 0x10 | 0x20), byte (1, 4) = 1, the 9 cards (Dark Break, Dark Firaga, Dark Aura, Impulse, Maelstrom, Barrage and their Dark twins), then each frame the ending's clears and (1, 2) = 1: the script leaves through its own exit and 2102 goes on. Leaving 1100 without a win (game over, reload) disarms; 2102 asks for the fight again. All logged (`story skip: Riku's Ansem fight ...`).
- **Checked**: `cargo test -p kh1-recom` (834 passed; `riku_ansem_hp`); the Windows build.
- **Not checked**: in game: the fight itself, the death demo with the Guardian still up (its state 8 waits for no kind-0x1008 objects), the game over in 1100, Ansem's attacks against a cardless Riku.

## Stage 436 (fork branch Re-Re-Reverse/Rebirth): the BBS hair keeps Re:CoM's painted sideburn; the kid's pointed sideburn strand dropped, version 0.198.37 (2026-11-27)

- **Ask** (fork owner, verbatim, with a screenshot: Riku in the default attire, BBS hair, an arrow at the pointed lock at his right temple): "cabelo ta quase perfeito do bbs essa costeleta ainda ta estranha, parece do wolverine, não acho que vc precisava ter tirado a costeleta pinta da cabeça base". Reverses Stage 412's sideburn choice (the owner then: "tem a do cabelo mesmo e tem a da cabeça base ao lado, ficou estranho").
- **Found** (`model-editor`, Checked (files, `sideburn_probe`)): the arrowed lock is the BBS mesh sideburn, a separate strand of 7 triangles a side (UV islands 7 / 17; tip (77, −7, 31) mm, root (89, 27, 16) mm from `head`); removing only the triangles with a corner in `SIDEBURN` left its top triangle across the temple skin (a blue sliver in 205's swings), so the whole island goes.
- **Change** (`kh1-recom-assets/src/bbs_hair.rs` only): `without_sideburn` drops every hair triangle in a UV island with a corner in `SIDEBURN` (14 a group), called by `hair()`; the `painted_temple_window` override removed, so Re:CoM's painted lock shows again; the cheek move (`SIDEBURN_REST`, `face` / `set_face`) removed. All BBS groups: 183 / 193 / 187 / 197, 185 / 195 / 189 / 199, 205 / 206, and the Keyblade groups built on them (212 / 213).
- **Checked**: `cargo nextest run -p kh1-recom-assets` (252 passed; `sideburn_probe`: nothing moved at the temple, 0 strand corners, the painted lock seen from the side; `swaps_rikus_hair_for_the_kid_s`: 14 dropped; `temple_owner_views`); the Windows build. Renders: workspace `local/assets/recom/riku-hair/out/temple/sb_before` / `sb_after`, `temple_owner/sb_after` (the screenshot's angle), `bbs_cam/0101_sb_*`, `0111_sb_*`, `dark/bbs_sb_*`, `dark_mode/bbs_sb_*`.
- **Not checked**: in game. Known: the painted lock is short and a little paler / softer than the BBS strands; a 3-5 mm skin patch in front of its top, between the hair and a side tuft (reads as a hairline).

## Stage 435 (fork branch Re-Re-Reverse/Rebirth): the battle's Dark Mode keeps Riku's Keyblade with every hair (one look table), version 0.198.36 (2026-11-27)

- **Ask** (fork owner, verbatim): "no darkmode a keyblade tbm nn troca, já coloca essas regrs para o futuro"; then "para os MEUS mods coloque a Modularidade como princípio BAZILAR as coisas precisam ser intercambiáveis e compatíveis umas com as outras!"; to the plan: "1", "sim". Rules: workspace `CLAUDE.md` 12, `docs/design-philosophy-instructions.md` § Modularity, § Riku's Dark Mode keeps every look.
- **Found** (read-only investigation, Checked (code)): the blade is parts 0 / 1 of the model on `wep0` (Dark Riku: bone 105); the Dark Mode hair groups were built with no blade (`step.rs`, `build_dark_mode_kh2`), `dark_mode()` hard-coded `blade: 0` and `dark_entries` chose by hair only, so no Keyblade of Heart group existed for Dark Riku.
- **Change** (`model-editor`): assets: `DARK_MODE_LOOKS` (`kh1-recom-assets/src/riku_skin.rs`), one row per (Dark Riku's group, hair, blade), the 4 Soul Eater rows then 6 new ones; one builder `build_dark_mode` (the BBS / KH2 builders take an optional blade, `with_blade(flip = true)`), one loop in `step.rs`; the manifest's `dark_mode` rows carry `"blade"` (older DLLs keep each hair's first row, the Soul Eater's). New groups: 209 / 210 (Re:CoM hair + Keyblade of Heart), 212 / 213 (BBS), 216 / 217 (KH2, on the existing 139-bone pack copies). DLL: `dark_entries` through `group_for` (Riku's own look rule, no copy); the Keyblade follows Riku's choice whatever his attire; fallback: the Keyblade on Re:CoM's hair, then his own group, missing groups logged (`dark_missing`); KH2 rows still wait for all pack copies. Hit volume and trails stay the Soul Eater's. About 142 MB more asset output (HD `.RTM` copies).
- **Checked**: tests (`the_battle_dark_riku_keeps_rikus_keyblade`, `the_battle_dark_riku_keeps_the_keyblade`: each group free in Recom.hed, byte-equal to its Soul Eater twin but the weapon, pack and EET / SES included; the blade's vertices on bone 105 where 161 / 171 have them); the Windows build. Renders: workspace `local/assets/recom/riku-hair/out/dark_mode/` (`recom_heart*`, `bbs_heart*`, `kh2_heart*` `_0.198.36`, `ref161_0.198.36`, `ref171_0.198.36`): the Keyblade in the right hand in every clip, the fist closed on the handle in the guard as on 161.
- **Not checked**: in game (the swap at the next battle; Re:CoM's texture lookup with 161 and 209 loaded together); part 0 (untextured) not drawn by the renderer; no measured hand-to-handle distance.

## Stage 434 (fork branch Re-Re-Reverse/Rebirth): the Dark Mode BBS hair's neck skirt tucked under the hair, version 0.198.35 (2026-11-27)

- **Ask**: Stage 432's (fork owner, verbatim): "percebi que o cabelo do bbs tem alguns espaços vazios no modo dark mode, testei pulando e atacando, já veja ai"; the `model-reviewer`'s pass on 0.198.33 / 0.198.34 (installed at the owner's "instala ai k") found the skirt's coat rising ~4 cm into the middle nape spike from level / low cameras in every pose, and thin dark slivers through the hair (under the left ear, jump f9, guard f0, rise f9).
- **Found** (`model-editor`, Checked (files, renders, `dark_mode_hair_hole_scan` now also counting skirt pixels in front of the hair)): the skirt ended on the hair's outer corners; in idle it was in front of the hair in 15362 px (sphere scan). Binding its top corners to the neck's bones instead did not lower the crossings over the neck and doubled the holes in the air swing (not kept).
- **Change** (`ddd_hair.rs`): each nape skirt top corner is a new vertex rigid on `head`, at the hair corner moved `NECK_SKIRT_TUCK` = 3 cm toward the head centre, with the edge corner's texel (a sweep: 0 / 1.5 / 3 / 4.5 cm → 15362 / 4662 / 2324 / 1417 px in front of the hair, holes unchanged up to 3 cm, growing at 4.5 cm). Test-only tuning switches `NECK_TUCK`, `NECK_TOP=edge`.
- **Checked**: `cargo nextest run -p kh1-recom -p kh1-recom-assets` (1082 passed, 2 skipped); the Windows build; holes worst 11-15 px per clip (as 0.198.33); skirt in front of the hair in idle 4597 px (worst 12 px); the idle wedge and the under-ear sliver gone; jump f9 / guard f0 / rise f9 as 0.198.32. Renders: workspace `local/assets/recom/riku-hair/out/dark_mode/` (`cmp435_reviewer_frames.png`, `cmp435_reviewer_zoom.png`, `cmp435_culled_205.png` / `_206.png`, `cmp435_level_views.png`, `bbs_c435/`, `diff_205_c435/`).
- **Open** (three tries on this defect: the hair texture, the coat texture, the tuck; stopped by the stop rule): from a level camera in the most extreme frames (`p003_057` f10, `p003_036` f4, `p003_037` f0) a small dark spike still rises into the nape hair and coat shows beside the neck behind the jaw (worst 219 px). Not checked: in game.

## Stage 433 (fork branch Re-Re-Reverse/Rebirth): Riku's hairs are earned in his story (BBS at Vexen, KH2 at Riku Replica I), version 0.198.34 (2026-11-27)

- **Ask** (fork owner, verbatim): "modularidade e senso de rencompensa após esforço são os pontos mais importântes para diversão, então por isso as nossas features de costméticos precisam ser desbloqueádas a partir da narrativa ou de ações específicas nos jogos"; on the hair unlocks: "um defeito MUITO comum dos jogos é liberar os cosméticos APÓS o endgame, se acabou a maioria vai dropar o jogo, precisa estar contida no loop principal"; "vish ent tem que ser até antes o do bbsh"; to "BBS at Vexen's win (B10 exit hall), KH2 at Riku Replica I's (B8)": "perfeito, sim!". Rules written: workspace `CLAUDE.md` rules 12-13, `docs/design-philosophy-instructions.md` § Modularity, § Cosmetics are earned, § Riku's Dark Mode keeps every look.
- **Addresses** (Checked (decomp + exe + files), Stage 399 / `knowledge/ReCoM/recom-riku-story-progress.md`): Re:CoM's story int (0, 0x2A) through the variable reader `0x1401B08C0`: the hall scripts set 0x80 at Vexen's win and 0x400 at Riku Replica I's.
- **Change**: `riku_unlocks::hair_unlocked` (pure; `HAIR_BBS_WIN` 0x80, `HAIR_KH2_WIN` 0x400); `win::unlocks::riku_hall_bits`; `win::riku_skin::hair_unlocked` (also `all_unlock`, development builds only): `chosen` wears an unearned hair as Re:CoM's, `refresh` puts the setting back with a log line, and the Costumes page refuses a locked hair with sound 5 and "Riku must defeat Vexen once to wear this hair." / "Riku must defeat Riku Replica once to wear this hair." (`menu::HAIR_*_LOCKED`, in the text-width test).
- **Checked**: `cargo test -p kh1-recom` (833 passed; `hairs_are_earned_at_vexen_and_the_replica`); the Windows build.
- **Not checked**: in game (a save before / after Vexen and Replica I; a mid-story save).

## Stage 432 (fork branch Re-Re-Reverse/Rebirth): the battle's Dark Mode BBS hair has no see-through gaps at the nape, and its sway follows his own slots, version 0.198.33 (2026-11-27)

- **Ask** (fork owner, verbatim): "percebi que o cabelo do bbs tem alguns espaços vazios no modo dark mode, testei pulando e atacando, já veja ai"; offered four ways for the skirt's crossing: "2" (the neck's own texture); then "instala ai k" (installed before a separate `model-reviewer` pass, at the owner's ask).
- **Found** (`model-editor`, Checked (files; tests `dark_mode_neck_open_edges`, `dark_mode_hair_hole_scan`)): without Re:CoM's hair and its nape base, Dark Riku's neck has an open back edge (from `neck1` / `neck2`, 0.8 cm under the head bone and 6.3 cm behind, to the face's back edge 9.2 cm out); the culled inside of the neck shows the background through it while his jumps and swings move `neck1` 3-7 cm in the head's frame (worst 221 px at 2.5 mm / px). KH2's hair (207) hangs over it (worst 34 px). 205 / 206 swayed on Riku's slot map (his stance, slot 21, took the run's sway).
- **Change** (`kh1-recom-assets`): `HairTarget::neck_skirt` (BBS on 201 / 211, so 185 / 189 too, keeping 205 = 185's model), `in_neck_skirt_window` / `NECK_SKIRT_*`: 32 two-sided triangles per group from the open edges to the nearest hair corners, on the edge's own texture (`n_zz120_03` coat / `n_zz120_01` skin on 205; `_13` / `_11` on 206). `BbsHairSource::bake_ann_slots` and `riku_skin::build_dark_mode_bbs` on `dark_slots()`, called from `step.rs`.
- **Checked**: `cargo nextest run -p kh1-recom -p kh1-recom-assets` (1082 passed, 2 skipped without `RECOM_HED`); the Windows build. Worst see-through per clip after: jump 172 → 15 px, air swing 221 → 13, finisher 177 → 13, air finisher 139 → 13 (what remains is the ear interior and forehead from 45° below, as on 185). Renders: workspace `local/assets/recom/riku-hair/out/dark_mode/` (`cmp432b_culled_205.png` / `_206.png`, `bbs_c432/` against `bbs_c432_before/`, `diff_205_c432/`, `diff_206_c432/`).
- **Open**: the skirt still crosses in front of the neck by up to 11.2 cm in the air swing `p003_057`, the finisher and the jump (now drawn as coat), and shows dark coat wedges between the nape spikes from low and behind cameras (`closeup_back_walk_f0_p-15.png`, `orbit_low_180.png`): for the owner to judge in game. Not seen: the game itself.

## Stage 431 (fork branch Re-Re-Reverse/Rebirth): the battle's Dark Mode keeps the KH2 hair (groups 207 / 208, 139-bone copies of his 11 sleight / event packs), version 0.198.32 (2026-11-27)

- **Ask** (fork owner, verbatim): offered, after Stage 429, "(a) build 139-bone copies of the 11 packs, as Stage 395 did for Riku's event packs, plus a second hook at `0x14024ED00`" or "(b) fit KH2's hair on his 18 hair bones (128)": "a".
- **Found** (`model-editor`, Checked (decomp, exe bytes, files)): both play paths, sleights `0x1401F73A0` and events `0x1401F7290`, end in `0x1401A3580`, which at `0x1401A35F3` calls `0x14024ED00(ctrl, id, slot, start)` when the blend is < 0, else `0x14024D860` (the existing `pack_pick` hook); most sleight calls pass −1, and so does Dark Riku's dash `0x1401F7290(…, 1310, …, −1)` in `0x1402007E0` (`CPlayerDRiku` vtable `0x1405A50E8` slot 0x140). His sleight groups (`0x14020F720` → `0x14020FB10`): 1087 / 1088 / 1089 / 1093 / 1311-1313, plus 1310 and the event packs 4003 / 4043 / 4083: all 128 bones. His pack slots differ from Riku's (his 21 / 13 / 14 / 22 / 23 / 96 / 34 play Riku's 31 / 28 / 21 / 32 / 33 / 26 / 44; action tables `0x1407C6300[3]` / `[5]`). Free numbers (PLAYER table `0x1408055D0`, 553 rows; Recom.hed): 207 / 208, 1187-1189, 1193, 1410-1413, 4103 / 4143 / 4183.
- **New hook** `0x14024ED00` (`win::models::pack_set`, `sites::MODEL_HOOKS`): prologue `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20` (15 bytes, Steam exe); its only reference is the call at `0x1401A35F5`; no branch or pointer lands inside the moved bytes. Both hooks share `copy_for`.
- **Change**: assets (`riku_skin.rs`): `DARK_MODE_KH2_HAIR_GROUPS` (207 / 208), `DARK_PACKS` / `DARK_ALT`, `build_dark_packs`, `build_dark_mode_kh2`, `kh2_hair_rig::bake_slots` / `Kh2HairSource::bake_pack_slots` on his own slots (`DARK_SLOT_OF`), manifest key `dark_packs`; 207 / 208 written only when all 11 copies were built. DLL: `riku_skin::{DARK_PACK_SOURCES, dark_packs, dark_mode_usable}` (the KH2 group is not used unless all 11 copies are on disk), `win::riku_skin::{dark_packs, dark_kh2_hair}`; the copies get table rows and are queued while chars 5 / 6 point at 207 / 208. Default and BBS unchanged. About 49 MB more asset output (207 / 208 with HD textures as 184 / 188; the 11 copies 1.2 MB).
- **Checked**: `cargo nextest run -p kh1-recom -p kh1-recom-assets` with `RECOM_GAME` / `RECOM_HED` (1078 passed; `the_battle_dark_riku_gets_the_kh2_hair` with the step's real sources: 139 bones, 56 clips, non-hair channels equal, `.RTM` = 184 / 188's, the copies equal on 201's and 211's model; `dark_riku_slots_follow_rikus_rows`; `the_battle_dark_riku_takes_the_kh2_hair_with_his_pack_copies`; the BBS test now with the step's sources too); the Windows build. Renders: workspace `local/assets/recom/riku-hair/out/dark_mode/` (`kh2_c431/` against `recom_c431/`, `diff_207_c431/`, `cmp431_*.png`).
- **Found, not changed**: Stage 429's BBS groups 205 / 206 used Riku's slot map on Dark Riku's pack (his slots 21 / 28 / 33 get the wrong sway); Stage 395's `queue_if_missing` tests for an `.RTM`, which a pack-only group never has (Riku's event copies may be queued again at each room build); Riku's own KH2 hair (109 bones) has no copies of his 102-bone sleight packs 1092, 1300-1303 (the same mechanism fits: 1192, 1400-1403 free).
- **Reviewed** (`model-reviewer`, 2026-11-27): region × pose PASS for 207 / 208 (idle orbit low / high, walk, run, swing 034, finisher 036, guard 225, cast 227, sleight 043 from a copy, event 200 from a copy); `PL0207.MDL` = the approved `PL0184.MDL` but the root name, `.RTM` equal; the slot map right. Note: in deep bows the hair stands straight back from the head (same rig and bake as 184; no reference for these clips). Stage 429's BBS 205 / 206 confirmed to sway wrong on his slots 21 (battle stance: the run's sway), 28 and 33.
- **Not checked**: in game (the copies loading, a sleight and the 30 DP switch, Halloween Town's 208, lighting).

## Stage 430 (fork branch Re-Re-Reverse/Rebirth): Riku's KH1 damage at Damage = KH1 (his enemies on KH1's records, his Strength from vanilla's ratio), version 0.198.31 (2026-11-27)

- **Ask** (fork owner, verbatim): told his enemies keep Re:CoM's HP and damage in his story (Stages 264 / 0.162.0 / 303): "como assim desfaz oq e porq"; then "psé mas precisa se assemelhar ao do sora nesse ambito, dos danos igualmente ao dos mods de acordo com a gameplay do riku do jogo vanilla"; on the numbers (Strength = 0.7 × his AP, Dark Mode × 1.5, Lv1 = Sora's 7): "ss".
- **Found** (`gameplay-designer` spec; `reverse-engineer`, Checked (files + exe), `knowledge/ReCoM/recom-combat.md`, `recom-riku-combo.md`): `BattleCardList` row 0x01 (Kingdom Key) and 0x12 (Riku's card) both 100 / 100 / 150 by column (0x12 also 150 in column 3); `+0x43C` is copied from `stats + 0x10` (`0x1401F0BC0`): 10 for Sora at every level (`0x14023A386`), Riku's AP (10 at a new game, +1 per AP row; the Riku / Dark Riku swaps copy it); Dark Riku's swing reads column 3 (`0x140204F80`). In Sora's story the enemies' attacks stay Re:CoM's on Sora's Re:CoM HP (`docs/design-philosophy-instructions.md` § Phase 4), so Riku's do too.
- **Change**: `win::stats::sora_rules` (Sora's story, or Riku's with his KH1 combat): the twin, proxy and boss-twin records apply in Riku's story too (KH1 HP at Damage = KH1, Defence, floor, cap, resistances; sleights scale by KH1 / Re:CoM HP as Sora's). `riku::kh1_strength` (0.7 × his swing AP, Dark Riku's × 1.5, half up: AP 10 → 7, Dark 11; AP 30 → 21) and `win::stats::riku_kh1_hit`: his swing at Damage = KH1 is KH1's damage of the row on the record with that Strength and the equipped weapon's KH1 numbers without their own Strength, written as the hit's value (`body + 0x92`, as Sora's swing) before Re:CoM's handler, with KH1's cap at the join (`damage_cap::Source::Kh1`); logged `damage: riku swing row ...: KH1 Strength ...`. Damage = Re:CoM: unchanged (card AP, the share of Re:CoM HP). His HP and the damage he takes stay Re:CoM's.
- **Assumptions**: the ×1.5 swing of `+0x420 & 0x4000000` (trigger `+0x829` unknown) and Dark Riku's +20 power are left out; 0x12 is the Soul Eater by name (no card name read); Berserk not applied to Riku (`berserk_share` stays Sora's).
- **Checked**: `cargo test -p kh1-recom` (831 passed; `rikus_kh1_strength_keeps_vanillas_ratio`); the Windows build.
- **Not checked**: in game (a Traverse Town Shadow in his story: `stats: ... KH1 HP`, 6 hits at AP 10; Dark Mode 3; bosses and Re:CoM-only enemies through their proxies). Known: his Strength stops at 21 (AP 30) where Sora's reaches 51 at Lv99, so late floors may take long; to look at after a test.

## Stage 429 (fork branch Re-Re-Reverse/Rebirth): the battle's Dark Mode keeps the BBS kid hair (groups 205 / 206); KH2's not yet, version 0.198.30 (2026-11-27)

- **Ask** (fork owner, verbatim): "reparei que quando o personagem se transforma no DarkMode o cabelo é resetado para o Default"; asked whether to build the hair variants for the battle's Dark Riku: "SIM pode".
- **Found** (Checked (decomp)): Re:CoM's Dark Mode is Dark Riku (`CPlayerDRiku`, ctor `0x1401FF880`), built by the player factory `0x14020EC70` for chars 5 / 6 only if `0x1405A6910[char] + 10000` is loaded; the player ctor `0x1401E4C20` takes his group from `0x1405A6910[char]` (`+0x39C`) and his rows bind in the pack named `0x1407C6338[char]`; `0x140211960` preloads `[5]` for char 3 and `[6]` for char 4 (`0x140211A5B`). Our `group_load` hook (`0x140211AA0`, which calls `riku_skin::refresh`) runs before `0x140211960` in `0x1401AD040` and in `0x140145C50`'s script ops 0x1C / 0x1D. Nothing wrote chars 5 / 6 and no hair group of his existed (Stage 406). No `mov` / `cmp` immediate of 201 / 211 / 10201 / 10211 in the exe names his PLAYER group (the hits are jump-table series and a category-12 entity); Assumption: no computed reference. PLAYER table rows of 201 / 211 (Checked (exe bytes)): RTM, TXA, MDL, ANN, (BIN 201 only), EET, SES, SND, contiguous.
- **Found** (Checked (files), test `riku_skin::tests::probe_pack_bone_counts`): besides his own pack, Dark Riku plays 128-bone clips from 11 packs: sleights 1087, 1088, 1089, 1093, 1310..1313, event packs 4003, 4043, 4083. BBS's hair keeps him at 128 bones; KH2's makes him 139, so none of those would play (frozen pose; a wait on the motion end never passes, `knowledge/ReCoM/recom-skeleton-limits.md`). And a sleight plays through `0x1401F73A0` → `0x1401A3580`, which with blend < 0 takes `0x14024ED00` (looks the pack up itself), not the hooked `0x14024D860` (`pack_pick`). Recom.hed has files for 214 / 215.
- **Change**:
  - Assets (`kh1_recom_assets::riku_skin::DARK_MODE_BBS_HAIR_GROUPS`, `step.rs`, `keyblade::new_resource_names`): groups **205** (from 201) and **206** (from 211) = Dark Riku's model with BBS kid Riku's hair (`build_hair_from` on his own model, his own pack as the step writes it with the guard and cast clips, his EET / SES, RTM / TXA). The manifest lists them under a new key `dark_mode` (`[{"dark": 201, "skin": 205, "hair": 2}, …]`; older DLLs ignore it). KH2's hair: not built for him.
  - DLL (`riku_skin::{dark_mode, dark_entries, DARK_GROUPS, DARK_ANN_NAMES}`, `win::riku_skin::{apply_dark, dark_recom_names}`): `preload` adds the `dark_mode` groups as skins (same file check; their rows copy Dark Riku's, `add_rows`); `apply` (start, camp, each room / battle build) points chars 5 / 6 (`0x1405A6910 + 20`, `0x1407C6338 + 40`) at the hair's groups and pack names ("PL0205" / "PL0206"), each write checked against what is there (Re:CoM's 201 / 211 and pointers whose text is "PL0201" / "PL0211", read before the first write), names first, groups after, names put back if the groups fail; Re:CoM's hair or KH2's: Dark Riku's own groups. Chars 3 / 4 unchanged.
- **Checked**: `cargo nextest run -p kh1-recom -p kh1-recom-assets` with `RECOM_GAME` / `RECOM_HED` (1074 passed, 0 failed; `keyblade::tests::new_names_are_not_in_recom_hed` counts and checks the twelve new names); tests `kh1-recom` `riku_skin::tests::the_battle_dark_riku_takes_the_hair` (manifest parse and refusals, entries 205 / 206 for BBS, 201 / 211 for Re:CoM's and KH2's, chars 3 / 4 unchanged, rows copy Dark Riku's); `kh1-recom-assets` `the_battle_dark_riku_gets_the_bbs_hair` (groups free; 128 bones, his bone names; his pack's 54 clips (the test builds his pack without the guard / Terra sources; the asset step's pack has 56, with the guard `p003_225` / recoil `p003_226`: rebuilt and rendered by the `model-reviewer`, non-hair channels unchanged; the test to take the step's sources before commit) in the same slots and names, every non-hair channel unchanged; the `.MDL` byte for byte the Dark Form's BBS group 185 / 189 with his root name `ex_1580_sk`; the `.RTM` byte for byte 185 / 189's, so their globally looked-up texture names agree when loaded together; his own EET / SES). Renders (`battle_dark_camera_checks`, workspace `local/assets/recom/riku-hair/out/dark_mode/{bbs,recom}_cam`, `bbs_0211_cam`; sheets `cmp_swing034.png`, `cmp_swing036.png`, `cmp_close.png`): idle orbit, walk, run, his first swing `p003_034` and finisher `p003_036` from four sides, nape and ear close-ups: the hair stays on the head, nape closed, no tab; the same shape as the Dark Form's BBS group in game. The Windows build.
- **Reviewed** (`model-reviewer`, 2026-11-27): region × pose PASS for 205 / 206 (face, ears, nape, back, silhouette; idle, walk, run, swing 034, finisher 036, guard 225 / 226, casts); every diff box off the head is Re:CoM's long hair gone or BBS spikes; the thin ear stalks from behind / below are as the approved Dark Form BBS. Not seen: game lighting, the 30 DP switch, sleight packs, Halloween Town's 206.
- **Not checked**: in game (the switch at 30 DP showing the hair; a sleight in Dark Mode with it; Halloween Town's 206). The user must press Build (new asset output). KH2's hair on the battle's Dark Riku: open (needs 139-bone copies of the 11 packs and the `0x14024ED00` path); Riku's own KH2 hair (109 bones) has no copies of his sleight packs 1092, 1300..1303 (102 bones): his sleights with that hair may freeze (found, not changed).

## Stage 428 (fork branch Re-Re-Reverse/Rebirth): Riku's Guard defends from the press and blocks every hit from in front, version 0.198.29 (2026-11-27)

- **Ask** (fork owner, verbatim, on 0.198.28 in game): "a defesa segue uma merda, eu aperto o X e ela não conta, assim que apertar o X em teoria ela tem que estar defendendo igual o KH1, preciso que vc pare de tanto retrabalho sobre esse ponto"; on the proposal: "a defesa é a defesa e se movendo é esquiva parsa". Rule written: `docs/design-philosophy-instructions.md` § Riku's Guard (final).
- **Found** (the owner's 0.198.28 log): 12 Guards started, each with its guard object on 183 ms (11 frames) after the press (KH1's live window 10..36 of motion 212, Checked (files + decomp), `knowledge/KH1/kh1-sora.md` § Guard in detail), and hits off the blade got through (`hit got through (not on the blade, not in front)`); 5 presses while running were rolls (KH1's Dodge Roll rule, kept).
- **Change** (Riku and Dark Riku only; Sora unchanged): `riku::guard_motion` gives his KH1 guard motion (212, 54 frames) a live window 0..36, set with his swing timings (`Kh1Sora::set_riku_moves`, put back for Sora); `win::attack::blade_guard_pair`: a hit that is not on his blade but comes from in front (KH1's cosine, `guard_faces`) and is not one KH1's types / priorities let through goes to Re:CoM's own pair test with his body guard on (±100°, `set_guard`): its block (2) is his block (`blocked in front (off the blade, Re:CoM's body guard)`, KH1's block records when the enemy is a KH1 one). From behind: through, as before.
- **Checked**: `cargo test -p kh1-recom` (829 passed; `rikus_swing_timings_are_his_clips` checks the guard window 0..36 and Sora's 10..36); the Windows build.
- **Not checked**: in game (that a press right before a hit blocks it; a Defender's strike from in front; a shot from behind gets through).

## Stage 427 (fork branch Re-Re-Reverse/Rebirth): Riku's ground swings close in as Re:CoM's homing moves him; his chain window a frame later, version 0.198.28 (2026-11-27)

- **Ask**: Stage 425's (the approach from his own rows, approved with "ss"); found by the plan's item 8 review (mechanic-reviewer, read-only) before the owner's test.
- **Found** (the review, Checked (decomp + code)): Re:CoM's homing turns and moves him through body `+0x54` (`0x1401F5F40`) and `+0x80` (`0x1401D5630` → `0x1401D5670`); `swing_exe` runs Re:CoM's exe, then `win::movement::reapply` wrote KH1's facing and displacement over both every frame, so 0.198.26's homing bytes and `+0x854` did nothing. `+0x854` was also Re:CoM's `+0x84C`, which can differ from the KH1 soft target his swing was picked for. The chain window opened a frame early: Re:CoM's "frame > `+4`, time > 500" on KH1's frame clock (+`HIT_LEAD`) is the hit start + 1 and frame 12.
- **Change**: `reapply` leaves Riku's ground swings to Re:CoM (its homing and his clip's root); his air swings keep KH1's velocity. `apply_row` writes `+0x854` = the target `target_of` gives (lock, KH1 soft target, Re:CoM's pick). `riku::CHAIN_NOT_BEFORE` = 12, window from max(hit start + 1, 12).
- **Checked**: `cargo test -p kh1-recom` (829 passed; `rikus_swing_timings_are_his_clips` updated); the Windows build.
- **Not checked**: in game (that he closes in on a far target and turns to it; that KH1's position / facing sync takes Re:CoM's result without a jump). Left from the review (low): Riku's Counterattack keeps Sora's motion 213 timing on his finisher row; his clash spheres do not sweep (Sora's pass uses KH1's `sweeps`).

## Stage 426 (fork branch Re-Re-Reverse/Rebirth): Riku's swings clash with enemy attacks (his blade), version 0.198.27 (2026-11-27)

- **Ask**: Stage 422's (fork owner, verbatim): "alguns ataques parece que não pegam"; the plan's item 5 (clash), approved with "ss".
- **Found** (the attack review, read-only): 0.169.0's attack-meets-attack pass (`riku_pair`) only read a weapon volume of shape kind 2 (two spheres); Riku's is kind 7 (the 0.198.22 log: "his weapon volume is shape kind 7: no KH1 clash test for his swings"), so his swings never clashed and traded hits where Sora's clash.
- **Change** (`win::attack::riku_pair`): for a volume that is not kind 2, the spheres are his blade's, as his KH1 Guard builds them (`guard_blade_of`: grip to end on the bone Re:CoM puts his weapon volume on; `atk::blade_shapes` with the KH1 swing record's radius); then 0.169.0's pass as before (clash type / priority, his KH1 recoil, the marked swing meets no hurt body that frame). Kind 2 unchanged.
- **Assumption**: the Guard's blade (bone origin as grip, Re:CoM's `(−0.8, 0, 0)` end) stands for his swing's blade; kind 7's own layout not read.
- **Checked**: `cargo test -p kh1-recom`; the Windows build.
- **Not checked**: in game (a clash with a Shadow's or Soldier's attack: `riku: ... clash` / his recoil).

## Stage 425 (fork branch Re-Re-Reverse/Rebirth): Riku's combo in his own order and rhythm on KH1's rules (Riku and Dark Riku), version 0.198.26 (2026-11-27)

- **Ask**: Stage 422's (fork owner, verbatim): "eu já falei que era para reproduzir o sistema do KH1 ENTRETANTO adaptando totalmente a jogabilidade do Riku! e respeitando as especialidades dele, e até o momento está pouco fluído"; on the item-4 spec (order, rhythm, approach, Dark Mode, each from Re:CoM's Riku): "ss".
- **Found** (reverse-engineer, `knowledge/ReCoM/recom-riku-combo.md`, Checked (decomp + exe + files)): Re:CoM's Riku pick `0x1401F7520` (Dark Riku's own `0x140204F80`, vtable `0x1405A5210 + 0x128`) chooses the situation by the last swing and the target: a new combo opens with 6 airborne (Dark: 7), 1 without a target, 5 when the target's `+0x50` point is more than 1.0 m above him, 3 past 3.5 m, 2 when the target is π/4..3π/4 off his facing, else 1; after 1 / 2 / 6 → 5 (above or airborne) else 3; after 3 / 5 → 6 airborne else 1; the last swing is 4 (7 airborne; Dark: 7 after a 7). The next swing is taken only after the swing has hit (`+0x644`) and starts past the hit start and frame 10, cutting the clip; with nothing pressed the clip plays out. Clip lengths at 60 Hz: Riku 48 / 56 / 64 / 80 / 40 / 38 / 54, Dark 42 / 64 / 48 / 60 / 44 / 60 / 60; the finishers and Dark 0xA3 do not chain. Homing `+6..+9` per row; `+0x854` = `+0x84C`.
- **Change** (the KH1 controller `Kh1Sora`, `riku_moves = Some(dark)` while the player is Riku; `win::movement::run_frame` switches it and logs `riku attack: swing pick and timings: ...`):
  - **Order** (`riku::situation`, `situation_row`, `Kh1Sora::riku_attack`): Re:CoM's pick on KH1's combo counter and length (KH1's abilities still set the length; the counter's last swing is his finisher). Situation → KH1 row of the same clip: 1 → 13, 2 → 11, 3 → 14, 4 → 8, 5 → 17 in the air or KH1's jump-up row 1 from the ground (same clip; KH1's hop toward the target, its counter reset as KH1's jump-up), 6 → 18, 7 → 16. Sora's combo modifiers are not offered to Riku (Sora's clips).
  - **Rhythm** (`riku::swing_motions`, `Kh1Sora::set_riku_moves`): the requests 200..206 take his clip lengths, his hit windows (+`HIT_LEAD`) and a combo window from max(hit start, frame 11) to the clip's end on the rows Re:CoM chains. KH1's "a swing that hit nothing takes no next press" stays: as Re:CoM, a press counts once the swing has hit. His clips are no longer cut at KH1 Sora's motion lengths.
  - **Approach** (`attack::riku_table`, `win::attack::apply_row`): his rows also carry his homing bytes `+6..+9` and `+0x854` = `+0x84C` (Re:CoM's homing: turn to the target until `+6`, close in at 8 / 60 to 2.0 m before `+7` while nothing is hit). Sora unchanged.
  - **Dark Mode**: the same with Dark Riku's rules, rows, windows and clip lengths.
- **Assumptions**: `riku_combo_target`: the target's aim point (the same `+0x50` point Re:CoM reads) against Riku's position for `+0x4B4`; the distance `0x1401A3050` taken as flat; the angle wrapped to ±π; the `0x1401F0620` probe (1 → 6) and Dark `+0x418 & 0x80000` left out; clip frame count = length (±1).
- **Checked**: `cargo test -p kh1-recom` (829 passed; new `rikus_combo_follows_recoms_pick`, `rikus_swing_timings_are_his_clips`, `rikus_ground_combo_is_his_own_order_and_rhythm`, `rikus_far_opener_and_a_miss`, `riku_tables_take_his_own_hit_windows` with the homing bytes); the Windows build.
- **Not checked**: in game (feel, the homing with KH1's facing, the jump-up into his air combo, Dark Mode). Left: the clash for his swings (plan item 5), the DP "hit taken" rule.

## Stage 424 (fork branch Re-Re-Reverse/Rebirth): the KH1 Guard's facing test in 3D (KH1's cosine); a missed parry after KH1's 54 guard frames, version 0.198.25 (2026-11-27)

- **Ask**: Stage 422's (fork owner, verbatim): "os danos a defesa está melhor mas ainda está problemática". The plan's item 6 (Guard), approved with "ss".
- **Found** (the Guard review, read-only): `riku::guard_faces` tested a flat yaw angle (±100°, Re:CoM's body-guard rule), KH1 tests a cosine against the 3D direction, so a height difference changed the outcome; `win::dark_points::GUARD_SPAN` was 60, commented "KH1's guard motion: 60", but motion 212 is 54 frames.
- **Addresses** (Checked (KH1 decomp), 2026-11-27): `0x1402B4640` builds the guard owner's flat facing from its yaw (`y = 0`) and the vector owner → attacker (`+0x50` positions, or the attack's own points `+0x70` / `+0x10` when its `+0x30 & 0x400`), and passes when `0x1402A8C70(facing, d)` (the cosine, `dot / (|a| |b|)`, clamped) ≥ record `+0x64` (−0.1736). Motion 212 is 54 frames (`knowledge/KH1/kh1-sora-data.md`, Checked (files)).
- **Change**: `riku::guard_faces` is KH1's cosine (flat facing · 3D direction / length ≥ cos(half)); a point straight above or below passes (KH1's zero flat part gives 0 ≥ −0.17). It is the Guard blade's facing test for Riku and Sora (0.183.3), so Sora's KH1 Guard follows KH1 too. `GUARD_SPAN` = 54.
- **Checked**: `cargo test -p kh1-recom` (`rikus_guard_faces_the_front`: 110° behind at the same height fails, 3 m above passes, straight above passes; 825 passed); the Windows build.
- **Not checked**: in game. Left from the review: the DP "hit taken" rule (+2) never fired in the 0.198.22 log (it waits for HP to fall inside Re:CoM's damage check; where HP falls is not yet found); a shot's facing is judged from the bullet's own position (KH1 uses the attack's own points only with flag 0x400; Assumption kept).

## Stage 423 (fork branch Re-Re-Reverse/Rebirth): log lines for Riku's EXP and refused presses; Riku's story named in the battle log, version 0.198.24 (2026-11-27)

- **Ask**: Stage 422's (fork owner, verbatim): "tenho a impressão de que só ao tomar danho estou ganhando xp"; "alguns ataques parece que não pegam". The plan's item 7 (logs), approved with "ss".
- **Found** (the Guard / EXP review, read-only): no path grants EXP on damage (the mod's damage check writes none); Re:CoM's EXP comes only from the defeat orbs through the prize add `0x1401F4AB0`, and the 0.198.22 log had no EXP line to tell when they arrived. Assumption (to settle with this log): the orbs are pulled within 5 m and fade after 10 s, so with KH1's knockback EXP may arrive only when Riku walks over the spot, often while being hit.
- **Addresses** (Checked (decomp), 2026-11-27): `0x1401F4AB0`'s switch: kinds 0xCB..0xD1 call `0x1401E5730` with 1 / 10 / 60 / 5 / 30 / 99 / 1400; `0x1401E5730` adds to the prize player's (`DAT_14087B390`) `+0x440` (EXP) and raises `+0x444` (level).
- **Change**: `win::drops::prize_add` logs each EXP prize (`exp: prize 0xcb (+1): EXP a -> b, level x -> y; HP h, action a`, 200 lines); `win::movement::riku_refusals` logs Riku's refused Attack presses (`riku attack: press refused (LastSwingMissed)`, 200 lines); the battle-start line says "Riku's story" in Riku's KH1 battles and the `mp:` lines "player level" (they named Sora for Riku). No gameplay change.
- **Checked**: `cargo test -p kh1-recom` (825 passed); the Windows build.
- **Not checked**: in game (the lines themselves).

## Stage 422 (fork branch Re-Re-Reverse/Rebirth): Riku's and Dark Riku's swings hit in their own Re:CoM hit windows, version 0.198.23 (2026-11-27)

- **Ask** (fork owner, verbatim, on 0.198.22 in game): "as animações de combate do sistema do Riku estão meio estranhas ainda e mal adaptadas os danos a defesa está melhor mas ainda está problemática, alguns ataques parece que não pegam, tenho a impressão de que só ao tomar danho estou ganhando xp preciso de uma validação e ajuste do sistema de combate"; told his clips play in KH1 Sora's rhythm: "então ela é tocada no ritmo do KH1 mas eu já falei que era para reproduzir o sistema do KH1 ENTRETANTO adaptando totalmente a jogabilidade do Riku! e respeitando as especialidades dele, e até o momento está pouco fluído"; on the numbered plan (rule, research, hit windows, Riku's own chain, clash, Guard, logs, review): "ss". This stage is the plan's item 3; the rule (item 1) is in `docs/design-philosophy-instructions.md` § Riku's combat.
- **Found** (two read-only reviews of the 0.198.22 log and code): Riku's KH1 swings wrote KH1 Sora's motion hit frames (minus `HIT_LEAD`) into the row Re:CoM plays (`kh1_row_bytes`), while the clip is Riku's own: his finisher was live on 12 of its 30 swing frames (25-37 against his 18-48), his lunge lost 38 → 29, and every Dark Mode swing was off. A swing that hit nothing also refuses the next press (KH1's rule), so a miss ended the combo. The log: 41 swings, 33 hits, 5 misses with a hittable enemy near.
- **Addresses** (Checked (exe), bytes read 2026-11-27): Re:CoM's attack rows `0x1407C6C70 + id × 16`; Riku's rows 0x97..0x9D hit windows `+4/+5` 12-26, 12-32, 20-38, 18-48, 8-26, 4-16, 10-38; Dark Riku's 0x9E..0xA4 4-22, 4-42, 8-26, 18-40, 8-26, 8-30, 4-50; read by Re:CoM's window test `0x1401F5DC0` (Checked (decomp), the review).
- **Change**: `attack::RIKU_ROWS_1_7` / `DARK_ROWS_1_7` (the exe's bytes) and `attack::riku_table`: a copy of the KH1 table whose `ROW_MAP` swing rows take his own row's `+4/+5` as they are (Re:CoM reads them itself: no `HIT_LEAD`); clip, combo window, kind and flag 0x80 stay the KH1 row's. `win::attack::build_table` builds Riku's and Dark Riku's tables from the live rows (refused and logged when they are not the Steam exe's); `table_for` picks the table by player (Dark Riku in Dark Mode); `kh1_row_of` knows all three. Sora unchanged.
- **Checked**: `cargo test -p kh1-recom` (`riku_tables_take_his_own_hit_windows`; 825 passed); the Windows build.
- **Not checked**: in game. Known left (next items of the plan): KH1's combo window opens before some of his hits end (row 13: window 22, his hit to 26; row 14: 26 / 38; air 18 / 26), so a press in that gap before the blade touches still counts as a miss; his clips are still cut at KH1's motion lengths and chained in KH1 Sora's order; no clash for his swings (his weapon volume is shape kind 7).

## Stage 421 (fork branch Re-Re-Reverse/Rebirth): PL4062's pants and the left hand's fingers 4 / 5 in both level-up packs, version 0.198.22 (2026-11-27)

- **Ask** (fork owner, verbatim, on the two things Stage 420 found and left: `PL4062` with the same pants order, and the left hand's fingers 4 / 5 swapped in PL4042 and PL4062): "corrige ai".
- **Addresses**: none (assets only). Which screen plays group 14062: not identified.
- **Found** (Checked (files)): in `PL4042` `p002_042` and `PL4062` `p002_020` (90 frames each) the slots `l_fing4c/b/a` hold `l_fing5c/b/a`'s channels and the other way round (each slot's constant translation is the other finger's rest offset); `PL4062` has Stage 420's femur rotation too. The right hand is in order.
- **Change** (`pants_order`): `femur_moves` checks the children of `l_femur`, `r_femur`, `l_hand` and `r_hand`, and moves a child's single-child chain (a finger's joints) with it when every chain slot's constant translation is its new bone's rest offset; still only a complete permutation per parent, whole channel blocks, no value changed. `FIXED_PACKS = [4042, 4062]`: the step writes both fixed packs; `build_event_packs` fixes both before the KH2-hair copies (`PL4142`, `PL4162`).
- **Checked**: `the_level_up_pants_are_in_the_models_order` over both packs: 14 moves each (8 pant, 6 finger), every other channel byte-identical, the other event packs untouched; PL4062's panels: clearance 0.4-3.5 cm, stand-off 0.0-6.1 cm, inside PL0101's envelope (Re:CoM's: crossing to -7.5 cm, stand-off 14.5-26.3 cm). Left-hand close-ups, Re:CoM's against fixed, frames 0 / 15 / 24 / 45 / 75 / 89, front / 45 deg / back (the workspace's `local/assets/recom/riku-win/levelup/hands/sheet_4042.png`, `sheet_4062.png`): the fixed fingers close and rest as a hand, no new crossing; the difference is small (fingers 4 and 5 have near-equal shapes). `tools/test-affected.sh`, the Windows build.
- **Not checked**: in game (which screen plays 4062; that Panacea serves both replaced files); PL4062's legs as renders (numbers only; its frame 45 hand close-up shows the panel at the strap).
- The user must press Build (the asset output changed).

## Stage 420 (fork branch Re-Re-Reverse/Rebirth): Riku's level-up pants (Re:CoM's PL4042 channel order fixed), version 0.198.21 (2026-11-27)

- **Ask** (fork owner, verbatim): "a animação do riku quando termina uma luta ta meio estranha na calça com o traje normal" (with a screenshot of the level-up bonus screen: "é na tela de levelup quando escolhe oq upar"); on being told it is Re:CoM's own file: "quero corrigir sim". Done by a `model-editor` agent; reviewed by a `model-reviewer` agent.
- **Addresses**: the level-up screen `0x14010EF10` asks for motion group `0x36DA` = 14042 for Riku (`PLAYER/4042/PL4042.ANN`, one clip `p002_042`, 90 frames, 102 bones). Checked (decomp, files).
- **Cause** (Checked (files)): vanilla, not the mod (with `riku_hair = recom` the mod served no 4042 and no PL0101 model). Re:CoM's `p002_042` stores four children of each femur in a rotated slot order: slot `belt3` holds `belt4`'s channels, `belt4` `cuffs1`'s, `cuffs1` `cuffs2`'s, `cuffs2` `belt3`'s, both sides (each slot's fixed translation is the other bone's rest offset within 1 mm; every `PL0101.ANN` clip uses each bone's own). So the outer panel (`cuffs1`) was drawn on the wrong side of the thigh and the inner one (`cuffs2`) where the strap goes (the jutting top edge). Over the 90 frames: crossing into the thigh down to -7.5 cm (`l_cuffs2`, frame 5) / -6.0 cm (`r_cuffs2`), stand-off from the stance's hang 15-26 cm (PL0101's own clips: at most 4.3-12.8 cm).
- **Change** (`kh1-recom-assets`): new `pants_order` (`femur_moves`, `fix_femur_children`, `fixed_pack`, `fix_bytes`, `FIXED_PACKS = [4042]`): moves whole channel blocks back to their bones, only on a complete swap among a femur's children, no value changed; `step` writes the fixed `PLAYER/4042/PL4042.ANN`; `riku_skin::build_event_packs` fixes 4042 before making the KH2-hair copy (`PL4142`). Riku's Halloween model (`PL0111`) has the same skeleton (fixed too); the BBS hair plays PL4042 itself.
- **Checked**: `pants_order::tests::the_level_up_pants_are_in_the_models_order` (the 8 moves, other channels byte-identical, no crossing past 0.5 cm, stand-off within PL0101's range, Re:CoM's file fails both, other event packs untouched); after: no crossing (closest +0.9 cm), stand-off 0.0-4.6 cm. Renders, textured, frames 0..89 sampled, front-low / ±45 / sides / back: the workspace's `local/assets/recom/riku-win/levelup/{before_0.198.20,after_0.198.21,diff_0.198.21}/`, sheets `riku-win/sheets/lvl_fix_*.png`. `tools/test-affected.sh` (1065 passed), the Windows build.
- **Not changed (found)**: `PL4062` `p002_020` has the same pants shuffle; PL4042 and PL4062 swap the left hand's finger 4 / 5 chains. Dark Riku's packs (4003 / 4043 / 4083) are clean.
- **Assumption / not checked**: that Panacea serves the replaced `PL4042.ANN` as it does `PL0101.ANN`; the blend into the clip; the screen's lighting; in game. The user must press Build (the asset output changed).

## Stage 419 (fork branch Re-Re-Reverse/Rebirth): Thunder's vortex held at the raised grip, not on the blade, version 0.198.20 (2026-11-27)

- **Ask** (fork owner, verbatim, on 0.198.19 in game): "thundaga ainda ta estranho"; to which part (the chest sheet, the vortex on the blade, the motion / timing, Thundara too): "Vórtice na lâmina"; to the options (1 on Root at the raised blade's spot, 2 back to the left hand, 3 no vortex): "1".
- **Addresses**: none (assets only).
- **Change** (`riku_skin::{spell_events, thunder_vortex_at}`): Thunder's vortex (`ex_1580` 11, scale 0.8, levels 2 and 3) on `Root` at the grip's spot on KH1's release frame of Terra's raise, ground (`230`) (-0.11, 1.51, 0.52) m, air (`236`) (0.00, 1.50, 0.41) m (model frame, +z the facing), instead of on `wep0`, where the 60-tick program turned and swung with the blade into the recovery.
- **Checked**: `cast_effects_meet_the_gesture` now also asserts Root unturned on the release and the point within 6 cm of the grip: Riku 1.2 / 1.0 cm, Dark Riku 2.8 / 2.8 cm. `tools/test-affected.sh`, the Windows build.
- **Assumption / not checked**: that an EET offset on `Root` is in Root's frame (as Re:CoM's own Barrage event's (0.1, 0.4, 0.1) on Root reads); how it looks in game. The user must press Build (the asset output changed).

## Stage 418 (fork branch Re-Re-Reverse/Rebirth): Riku's spell effects meet Terra's casts (the raise keyed on the raised blade; Thunder's vortex on the blade), version 0.198.19 (2026-11-27)

- **Ask** (fork owner, verbatim): "nós trocamos as animações do Riku para as do Terra para as Magias, mas não ajustamos os VFX de cada uma, algumas ficaram estranhas, revise todas..."; to the five-part proposal (re-key the raise, Thunder's vortex to the blade, a test, before / after renders, version and test request): "sim".
- **Addresses**: Re:CoM's hand point `0x1401F0B30` (the DLL's shots and hand effects start there): the weapon's bone (`player+0x3D0`, `0x140224F50`: its attach bone or bone 2, row block `+0xA0`) at `0x1407C7E50` = (-0.8, 0, 0), the blade's end. Checked (decomp; the point as in `riku::BLADE_END`). No new hook.
- **Review of every cast effect** (Checked (files): bone positions of the built clips, model space, facing +z): pose-free effects (Root / `chest1`: Gravity, Stop, Aero's gusts, the Dark Firaga flash, Thunder's chest bolt, Cure's sparkle on bone 0) fit any clip. Pose-bound: the Dark Firaga glow (`l_hand`, Fire), Blizzard level 3's shard (`ex_1580` 10, `l_hand`), Thunder levels 2 / 3's vortex (`ex_1580` 11, `l_hand`), Aero level 3's ribbon (`wep0`) and the DLL's shots / hand effects (the blade's end, on KH1's release frame). Thrust (`315` / `316`, Fire / Blizzard): hands and blade 0.37-0.58 m in front at about 1.1 m from the release to the key's end: fit. Raise (`317` / `318`): keyed 10..14, its lowest point: on the release the blade was 0.57 m behind the body at 0.88 m and went up 5 frames later (Terra's 12 -> 13), and the left hand swings back to 0.47 m behind the body while the blade rises.
- **Change** (`kh1-recom-assets` `riku_skin`): `TERRA_RAISE_G` / `TERRA_RAISE_A` keyed 13..17 (the blade up in front at 1.51 m, held to Terra's 18): the swing up ends the charge and the release meets the raised blade; KH1's release and free frames unchanged, so gameplay timing is unchanged (Thunder / Aero, Cure, Gravity / Stop; ground and air; Riku, Dark Riku, the skin). `spell_events`: Thunder's vortex (levels 2, 3) on `wep0`. The Fire / Blizzard clips, the wind-ups and the Guard: unchanged.
- **Checked**: new test `cast_effects_meet_the_gesture` (Riku's and Dark Riku's built casts, release to key end: blade and blade hand >= 0.1 m in front of the chest; the raise's blade over the shoulder; Blizzard's left hand in front on the release): now >= 0.34 m in front and +0.30..+0.33 m over the shoulder; the 10..14 keys fail it (-0.61 m at frame 30). Mesh sheets before / after (`terra_mesh_sheets`, p002_230 and 236, frames release-8..+12): before, the blade behind the back on the release; after, raised in front. `cast_clips_put_the_key_moment_on_kh1s_release`, `terra_motions_on_riku`, `tools/test-affected.sh`, the Windows build.
- **Assumption / not checked**: how each effect looks in game (no effect can be drawn outside the game); Thunder's vortex turning with the blade bone; the raise's swing up now 2-6 frames before the release (Terra's own speed). The user must press Build (the asset output changed).

## Stage 417 (fork branch Re-Re-Reverse/Rebirth): Dark Mode's own three shortcuts for Riku; debug keys only in development builds, version 0.198.18 (2026-11-27)

- **Ask** (a tester, relayed by the fork owner, verbatim): "I don't know if it's possible, but I have a suggestion I wonder if it would be possible to have a separate shortcut menu for Riku's dark mode. As it seems if you switch to dark mode and you have non dark mode moves on your shortcut menu you can't use the shortcut menu it has no shortcuts, but if you have dark mode sleights already on your shortcut menu before switching to dark mode then the shortcut menu works."; to the three options (a separate set; an automatic swap to the twins; both): "3".
- **Cause** (Checked (code + decomp)): Riku had one set of three slots; in Dark Mode a slot works only for an id on Dark Riku's list (`crate::riku::DARK_SLEIGHTS`), through `riku_world` / `riku_learned_sleights(true)`. That list follows Re:CoM: Dark Riku's sleight start `0x140206830` lets 0x66..0x68 through, but his sleight init `0x1402042F0` has no case for them (they would do nothing), and Re:CoM gives him the Dark twins instead (below). (First written as "the start refuses 0x66..0x68", then as "the mod's list alone": both corrected by the `reverse-engineer`, the workspace's `recom-riku-sleights.md` § 4.)
- **Dev: Riku's spells** (fork owner, verbatim: "quero testar as magias habilita"): in development builds `kh1_all_spells = on` also gives Riku all seven KH1 spells at level 3 (`win::command_menu::riku_spell_levels`; not written to the save); a released DLL ignores the key (`DEV_KEYS`).
- **Re:CoM's rule for the first fill** (Checked (exe + decomp), `recom-riku-sleights.md` § 4): no card stack forms 0x66..0x6B; they are duel rewards, and the duel end `0x140162DD0` gives 0x69 / 0x6A / 0x6B instead of 0x66 / 0x67 / 0x68 when the main player is Dark Riku (`+0x420 & 0x400000`); the stack check `0x1401669F0` forms Inverse Burst 0x65 where Riku forms Holy Burst 0x64 (stock flag `+0x98 & 0x100`); Dark Riku runs 0x62 and 99 (0x63, LV3, 0x62's use counter). So `crate::riku::TWINS` = 0x66→0x69, 0x67→0x6A, 0x68→0x6B, 0x64→0x65; 0x62 / 0x63 kept. The Dark Mode flag `+0x41C & 0x4000000` comes from (0, 0x24) 0x08 (`0x1401F0C50`), set only by `WORLD00/2102` right before it gives card 0x6D (and by Re:CoM's debug helpers `0x1400D1A00` / `0x1400D1FC0`, which give both); `riku_dark_mode` now gives both as well.
- **Fill fix**: nothing is written or marked set up while Dark Riku has learned nothing (as Riku's own fill, Stage 414); a slot of Riku's set holding a Dark-only sleight says "Dark Mode only: set it in the Dark Mode set (Left/Right)." (`customize::text::DARK_ONLY_HELP`).
- **In game** (the owner's dev build, log): `riku_dark_mode` set the bit; 30 DP → `dark points: 30 DP: Dark Mode` → `Dark Mode on (DP 8)` → `first Dark Mode`: Dark Mode reached from a new game.
- **Rule** (§ Every KH1 feature follows Re:CoM's own game rules): neither game has one (Re:CoM has no shortcuts, KH1 no Dark Mode). Adapted from Re:CoM giving Dark Mode its own sleights: a set of its own, offered once Riku has reached Dark Mode in this save (the Dark Mode costume's bit `DARK_SEEN`, 0.151.0). Debug, development builds only (see below): `all_unlock = on` shows the rows from the start, as it unlocks the costume (fork owner, verbatim: "SE TIVER conseguiria colocar pra debug desbloquear o darkmode?"; "mas somente para debug enquanto estamos desenvolvendo, não vai chegar de jeito nenhum isso para o pessoal que for instalar!"). The transformation itself needs no unlock past the first battle: Re:CoM gives D Mode (card 0x6D) in `WORLD00/2102` step 170, right after battle 1100 (the workspace's `recom-riku-sleights.md`), and ours starts it at 30 DP.
- **Save** (`crate::riku::dark_block`): `save + 0x26D0 ..= 0x26D3`, right after the mod's last block (`abilities::SKILL_EXT_LAST` 0x26CF), inside the spare tail `0x246B ..= 0x2817` that no Re:CoM code reads or writes (Checked (exe), `docs/recom-keyblade-equip.md`); 3 slot bytes (as Riku's own) and a flags byte (bit 0: set up). Old saves read zero there: set up at first need. Added to `spare_range_nonzero`'s own bytes.
- **Change**: `crate::riku::{dark_twin, dark_fill, TWINS, DARK_KING}`; `win::mp::{riku_dark_shortcuts, set_riku_dark_shortcuts}`; `win::command_menu::riku_dark_shortcuts` (the first time: Riku's set with Impulse / Maelstrom / Barrage → Dark Impulse / Dark Maelstrom / Dark Barrage when taught; spells, Dark Riku's own sleights and the King's 0x62 / 0x63 kept; Holy Burst and the rest emptied; logged); `riku_world` reads it while the main player is Dark Riku. Riku's Customize (`win::keyblades::DarkSet`, `cz_slot`, `cz_picks`, `cz_rows`): rows 3..5 "Dark" + the button, their picker = his spells, Dark Riku's learned sleights, the King's two he can start; their help "In Dark Mode: hold L1 and press ..." (`customize::text::dark_slot_*`); the status panel shows the cursor's set. Riku's own set and Sora's page unchanged.
- **Debug only in development builds** (fork owner, verbatim: "sim mas pode até tirar das releases ,não deixa chegar pro cara que vai baixar, deixa isso só aqui para desenvolvimento mesmo"; "todos os tweaks de debug vc pode tirar alias e só entregar a versão de usuário com os locks e tudo certinho"; "se tiver como até deixa isso por alguma rotina no git se for possível pra nn ficar vc alterando na mão na hora das releases"): the debug keys `all_unlock`, `riku_free_magic`, `riku_dark_mode` (new: the owner, verbatim, "claro que não!!!! o que a gente estava usando era uma SKIN pro RIKU NORMAL usando a mesma do DarkMode, oq eu to falando de testar é a GAMEPLAY do DarkMode que consumimos o Modelo para a SKIN!"; on, Riku's story gets the saved story bit (0, 0x24) 0x08 that `WORLD00/2102` sets after his first battle and the player build `0x1401F0C50` turns into Dark Mode, `+0x41C & 0x4000000`, the workspace's `recom-riku-ansem-tutorial.md` § 4.4 (Checked (decomp); the bit's meaning there an Assumption until seen in game); `win::story_skip::dev_dark_mode`, from Riku's frame, written into the save as the story would, logged `DEV riku_dark_mode: ...`), `kh1_all_spells`, `kh1_all_abilities`, `kh1_test_items`, `debug_attack` (`settings::DEV_KEYS`, 7) are read and written only with the new cargo feature `dev` (`crates/kh1-recom/Cargo.toml`), which only `tools/dev-install.sh` turns on. A released DLL (`tools/build-win.sh`, the release workflow's build) ignores their lines without a warning, never writes them, and the default ini no longer lists them, so every unlock, cost and item follows the game. A development build logs `DEV BUILD (feature dev): ...` at start. Routine (`.github/workflows/patched-release.yml`, step "No development build in a release"): the release stops if `build-win.sh` asks for a cargo feature or the DLL has that marker (tried on both DLLs: the user build passes, the development build is refused); the workflow's `cargo test --workspace` (no feature) runs `settings::dev_key_tests`.
- **Assumption**: "Dark" + the glyph reads well on the row (squeezed as every row); the status panel keeps its three button labels for both sets.
- **Riku's Customize page: two sets of KH1's three rows** (fork owner, verbatim, approving the `ui-ux` agent's review of the page: "ativa o agente responsável"). Replaces the six-row layout and the `DARK_SEEN` gate written above. Each choice from the rules, not a new question (§ Every KH1 feature follows Re:CoM's own game rules; the `ui-ux` agent's "Decide what the rules already decide"):
  1. **Layout** (rule 2, adapt Re:CoM's navigation): back to KH1's three rows (`0x140302950`). Two sets, Riku's and Dark Mode's, switched with Left / Right on the slot list as Re:CoM's Sleights page switches its tabs (`0x1400EB7E0` state 4: `0x140111A70(camp, 0x408000)` index − 1, `0x802000` + 1, wrapping, sound 1, draw mode 10; Checked (decomp)). Wraps (two sets), sound 1, rows, help and status panel rebuilt, the cursor stays on its row; only when the Dark set exists; not in the picker. Square (Re:CoM's Sleights List, camp bit 0x80) and L1 / R1 (pad 4 / 8, paging) unchanged. `win::keyblades::{Page::switch_set, cz_index, LEFT, RIGHT}`, `Content::Customize::dark_shown`; `cz_rows` removed (always `customize::SLOTS`). Which set is on screen: the Dark rows read "Dark" + the button and their help names Dark Mode (our own text; Re:CoM's tab pictures mean its card kinds and are not used). The header plate is not changed (Assumption: re-titling it mid-page would replay its sequence 7; not tried). Log `customize: Riku: Dark Mode set shown (Left)` / `Riku's set shown (Right)`.
  2. **Texts** (`customize::text`): Riku's own set when the Dark set exists, `SWITCH_SLOT_HELP*` ("Hold L1 and press Triangle. Left/Right: Dark Mode set."); the Dark set, `DARK_SLOT_HELP*` ("In Dark Mode: hold L1 and press Triangle. Left/Right: Riku's set."); its picker, `DARK_PICK_HELP*` ("Choose the spell or sleight for Dark Mode's L1 + Triangle."); glyph, word and Circle forms, all in `customize::tests::texts_fit_the_help_box` (≤ `menu::HELP_MAX` 66). Riku without the Dark set and Sora keep their help. Assumption: "Left/Right" in words (Re:CoM's escapes `0xF564..` checked for the shortcut buttons only, no D-pad glyph read).
  3. **Empty slots read "<glyph> None"** (rule 2, KH1's own: its slot list `0x140302950` shows sysmsg 0xD4 "none" on an empty slot; `customize::text::NONE`, our word, as before): `customize::text::slot_row`, in both of Riku's sets and on Sora's page, which is the same code (`Page::row_text`, `Content::Customize { picking: None }`). Words form "Square: None".
  4. **Gate** (rule 1 / 3, Re:CoM's own learned test): the Dark set exists once `0x14023AB00(0x6D)` (`abilities::sleight_learned_fn`, D Mode's card) answers learned, which Re:CoM gives in `WORLD00/2102` step 170 after battle 1100 (the workspace's `knowledge/ReCoM/recom-riku-sleights.md` § 1-2), instead of `dark_points::dark_seen()`. Development builds keep `all_unlock` (feature `dev`).
  5. **Riku's own picker no longer offers Dark Riku's own sleights** 0x5F, 0x60, 0x61, 0x65 (rule 3, "No shortcuts past the game": Riku's own sleight start `0x140216AB0` refuses them, `crate::riku::SLEIGHTS`): it lists `riku_learned_sleights(false)` and the King's; the Dark set's picker is unchanged. A slot of his own set that already holds one keeps its byte and shows greyed (`customize::unequipped`, its existing help line). No save data changes.
  Not touched: `crate::riku::{dark_fill, dark_twin, TWINS, DARK_KING, DARK_SLEIGHTS}`, `win::command_menu::riku_dark_shortcuts` (the first-fill rule waits for a reverse-engineering answer), `settings.rs`, `win/story_skip.rs`. Recorded for the game: the workspace's `knowledge/ReCoM/recom-riku-sleights.md` § 3.5 (the Sleights page's tab keys, Riku's list's group / row-look bytes).
  Checked: every `kh1-recom` test with and without `--features dev` (822 each; `customize::tests::{dark_mode_rows_name_their_buttons, texts_fit_the_help_box, slot_rows_show_the_bound_name_beside_the_button}` updated), the Windows build with `--features dev` (`tools/dev-install.sh`, no new warning) and the Windows check without it. **Not checked**: in game (the switch, the texts' width on the plate, a held Left / Right repeating the switch as Re:CoM's tabs do).
- **Checked**: `settings::dev_key_tests::debug_keys_live_only_in_development_builds` and every `kh1-recom` test with and without `--features dev` (822 each), `riku::tests::dark_modes_set_starts_with_the_twins`, `customize::tests::dark_mode_rows_name_their_buttons`, `keyblades::tests::spare_range_ignores_our_byte`, all affected tests, the Windows build (no new warning). **Not checked**: in game.
- **The Dark Mode set's mark** (fork owner, 2026-11-27, verbatim: "ficou bom, só que era bom colocar talvez algum ícone que ajudasse o usuário a ver se nn ele só vai ter a escrita pra se guiar, não? pode ser o ícone de heartless que já era pra estar funcionando nos sleights que só o Riku em DarkMode consegue usar"; `ui-ux` agent).
  1. **The DP emblem in the command menu** (Stage 321) is not broken: the owner's log (0.198.18) has the manifest's `"dp_icon": 6453` in `kh1-hud.json` (asset step of 2026-11-27 09:52), `hud = kh1` with `hud_kh1_menu` at its default on, and `customize: ... Dark Riku's sleights []` at every page open. Since 0.198.15 the lists show only what Re:CoM has taught (`0x14023AB00`), and Re:CoM teaches Dark Break / Dark Firaga / Dark Aura (0x5F..0x61) at the won end of the Ansem tutorial battle `WORLD00/1100` (the workspace's `recom-riku-sleights.md` § 2); `riku_dark_mode` gives only D Mode (0x6D). So no DP sleight (0x5F / 0x60 / 0x61 / 0x65) was in a list to carry the emblem. Not changed.
  2. **The mark** (rule 2, Re:CoM's own UI data for a Re:CoM camp page): the Dark Mode set's three rows take Re:CoM's camp row **0x12F** (entry 4 of its list table `0x1407C11F0`, already a checked site: red plate 0x8000003C and Dark Riku's wing, sprite group 89 tinted 0x800000A0) instead of the blue magic row 0x133; Riku's own set and Sora's page keep the blue row. That wing is the mark Re:CoM's own Sleights List puts on Dark Riku's sleights (0x5F first: flags 0x08 → sequence 0xE3, 0x60 / 0x61: 0x09 → 0xDC, table `0x1407C1150`; both the same red plate with the wing tinted 0x80000080), and no other sequence of `camp.lad` draws it. Checked (files: the owner's `FORM/0100/FO0100.SPR`, base `camp.lad` and `UK_camp.lad`; `UK_camp.imz` cropped: sprite group 89 is a dark wing, 82 the heart, 85 the Mickey symbol, 84 the magic symbol); Checked (exe: the two tables). The row's icon slot replaces the magic symbol at the row's left; the word "Dark" stays. `keyblades::row_look::DARK`, `customize::slot_look`, `win::keyblades::Page::row_look`; the switch's log line names the row.
  3. **Departs from the ask**: the owner suggested the Heartless emblem of the command menu ("pode ser"). That picture is in `FO0104.RTM`'s level-up atlas and the mod's `kh1hud` set, not in the camp's atlas (`FO0100.RTM` `camp.imz`): a camp row can only draw it through a new `camp.lad` sprite on new pixels in every language's `camp.imz` (an HD pack replacing `camp.imz` would lose them), or through a separate `kh1hud` control on the camp's draw list (not tried in the camp: whether `kh1hud:00` is registered while the camp is open is unknown). Re:CoM's own mark for "sleights only Dark Riku uses" is the wing, so the page uses it with no asset change.
  Checked: `customize::tests::dark_mode_set_takes_recoms_dark_wing_row` (slots 3..5 → variant 4, and entry 4 of the checked table is 0x12F). Assumption: the red wing reads well on the red plate (Re:CoM draws exactly this row); not seen in game.

## Stage 416 (fork branch Re-Re-Reverse/Rebirth): Re:CoM's Sleights List on Riku's KH1 Customize page (Square); the rule "every KH1 feature follows Re:CoM's rules", version 0.198.17 (2026-11-27)

- **Ask** (fork owner, verbatim): the same message as Stage 415 ("... e o alnce do menu listando sleights no jogo, acessível na aba Customize no modo KH1 foi implementado?"); to the plan (Square opens Re:CoM's own list, its "Sleights" prompt bar on the page, our input waits while it runs, first bytes checked): "ss". Then: "coloca no mod já, regras que reforçam todas as novas features e implementações precisarem seguir ou adaptar as regras normais do Jogo quando implementar a jogabilidade do KH1!".
- **Addresses** (the workspace's `knowledge/ReCoM/recom-riku-sleights.md` § 3, Checked (decomp + disasm); first bytes checked in `sites::KEYBLADE_CALLEES`, 20 each, read from the exe): the list's update `0x1400EBE30(camp)` (sub-state `camp+0xAC`: 0 open .. 4 closing, −1 idle; answers 1 the frame it has closed; it reads the pad itself: Square / cancel close, confirm opens Re:CoM's sleight help on a learned row, `?????` on the others); the prompt bars `0x1400DEC60(camp, mode, slot, variant)` (mode 1 show, 4 close; Riku's variant 9); the camp's fresh press `0x140111AE0(camp, bits, 0)` (0 while the input lock `camp+0x8C` is set). Re:CoM's deck page opens the list at `0x1400DBCE3..0x1400DBCFE` (Square 0x80 → `+0xAC = 0`, `+0x8C = 1`) and calls the update every frame (`0x1400DBDAC`). No hook.
- **Change** (`win::keyblades::Page::{sleights_list_open, sleights_list_step, close}`, `SleightsList`): Riku's Customize page (his story, Combat = KH1) shows Re:CoM's slot-0 prompt bar (variant 9) when it opens and closes it with the page (as `0x1400DBD30`). Square (slots or picker) does what the deck page does; while the list runs our page's input is skipped and `0x1400EBE30` runs each frame, then once more after it answers 1 (it lowers the input lock), then our help line comes back. Sora's Customize unchanged.
- **Rule** (`docs/design-philosophy-instructions.md` § Every KH1 feature follows Re:CoM's own game rules, or adapts them): find Re:CoM's rule for the nearest thing first and cite it; follow it or adapt it (written in the Stage); no "counts as learned / available" shortcuts; check a new game and a mid-story save; ask when unsure.
- **Assumption**: the list draws correctly over our page in the Config states 0x4A..0x4C, and its sleight help (`CTechHelp`, `0x14012BF80`) runs there (the camp's trailer runs the prompt bars there, Checked; the rest not seen); the prompt bar's text `0x500909` is Re:CoM's "Sleights" (not read).
- **Checked**: `tools/test-affected.sh` (1058 passed), the Windows build (no new warning). **Not checked**: in game.

## Stage 415 (fork branch Re-Re-Reverse/Rebirth): the King's sleights only once Re:CoM has taught them (B11), version 0.198.16 (2026-11-27)

- **Ask** (fork owner, verbatim, with a screenshot of Riku's Customize picker at 0:34 listing Mickey Miracle and Holy Burst): "algumas habilidade relacionadas ao mickey tbm estão desbloqueadas desde o início do jogo faz sentido e o alnce do menu listando sleights no jogo, acessível na aba Customize no modo KH1 foi implementado?"; to the three-part proposal: "ss".
- **Cause**: `win::party::learned` counted the King's Party sleights as taught whenever Riku's story ran with `riku_kh1_movement` (0.150.0's Assumption "Re:CoM's Riku forms his sleights from cards", shown wrong by Stage 414). Re:CoM teaches 0x62..0x64 in B11's hall `WORLD00/2103` (the workspace's `knowledge/ReCoM/recom-riku-sleights.md`, Checked (files + decomp)).
- **Change** (`win::party::learned`): Re:CoM's own test `0x14023AB00` alone decides, in both stories; so Riku's Customize picker, page 2's Party list and his L1 shortcuts show the King's sleights only after B11. The King met bit stays the battle's availability rule (`sets`). Sora's story unchanged.
- **Not built here**: Re:CoM's Sleights List on Riku's KH1 Customize page (Stage 416).
- **Checked**: `tools/test-affected.sh` (1058 passed), the Windows build (no new warning). **Not checked**: in game.

## Stage 414 (fork branch Re-Re-Reverse/Rebirth): Riku's sleights only once Re:CoM has taught them, version 0.198.15 (2026-11-27)

- **Ask** (fork owner, verbatim, with screenshots of Re:CoM's Sleights List at 0:02, all `?????`, its Shortcuts page's "Sleights" button and our Customize page): "mas eu te passei um print dele no modo recom, e na listagem de sleight que fica  quando vc acessa o menu de shortcuts inclusive falei pra vc implementar esse sleight lists no modo kh1 nessa tela e vc nn fez.. mas minha duvida é no jogo normal os sleights vem daquele jeito ali parece... ou estou enganado?"; to the four-part proposal: "ss" (`docs/design-philosophy-instructions.md` § Riku's sleights).
- **Found** (the workspace's `knowledge/ReCoM/recom-riku-sleights.md`, Checked (files + decomp)): Re:CoM's learned test `0x14023AB00` reads the save's card states (save + 0x2840); a new Reverse/Rebirth game has none (Checked (game: the owner's screenshot)); the Ansem tutorial's end (battle 1100) teaches 0x5F..0x61 and 0x66..0x6B, `WORLD00/2102` D Mode (0x6D), B11's hall `WORLD00/2103` the King's 0x62..0x64 and Inverse Burst (0x65); Riku's level-up teaches none. 0.147.0's "Re:CoM's Riku forms them from cards" (Assumption) was wrong: every list showed all of them from the start.
- **Change** (`crate::riku::learned_ids`; `win::command_menu::riku_learned_sleights`, `riku_world`; `win::keyblades::Page::open`, Riku's Customize): his and Dark Riku's Sleights lists, his L1 shortcuts (a shortcut works only for a listed id) and his Customize picker hold only the ids Re:CoM answers as learned; the one-time shortcut fill waits until something is learned. At Combat = KH1 the story skip of battle 1100 (Stage 374, `story_skip::riku_tutorial`) gives the same nine cards, so they appear right after his first battle, as in Re:CoM. The King's Party sleights keep their own rule (the King met bit, set by the same 2103 script).
- **Not built**: Re:CoM's Sleights List on his KH1 Customize page (the camp's Sleights List states not found yet).
- **Checked**: `riku::tests::only_the_sleights_recom_taught_are_listed`, all tests, the Windows build. **Not checked**: in game.

## Stage 413 (fork branch Re-Re-Reverse/Rebirth): the BBS hair's temple fixed in game (one texture name per group); the skirt work cancelled, version 0.198.14 (2026-11-27)

- **Ask** (fork owner, verbatim, on 0.198.13 in game, a Dark Form side close-up: a yellow triangle at the temple and a darker flat patch in front of the ear): "costeleta ficou até pior...".
- **Addresses**: none new (Re:CoM looks textures up by name across every loaded model, `0x1402A5140`, Checked in `keyblade.rs`).
- **Found** (Checked (files: the installed DLL's strings, the built `.RTM` names; probe `bbs_hair::tests::temple_game_view`, which renders each picture as the game samples it)): 0.198.13 named all four BBS hair pictures `rk_hair_bbs`, so the Dark Form's moved temple triangles drew Riku 0101's picture, whose texels there are the kid's yellow vest: mean colour of those triangles in their own picture (125, 72, 59), in 0101's (166, 96, 50); the owner's patch (143, 67, 46), the yellow (231, 161, 18). TIM2 and DDS agree within 1; no mipmaps. Then (`model-reviewer`, no-go): a skin fleck inside the hair on the model's -x temple of 0101 / 0201: a z-tie, the sideburn pulled to 2 mm dipped 0.4-0.5 mm under the temple skin (2 / 2.5 / 3 mm: 0.5 / 0.3 / 0.1 mm under; 3.5 mm clear on both sides).
- **Change**: one picture name per group (`ddd_hair::RIKU_0101_BBS` `rk_hair_bbs`, `RIKU_0111_BBS` `_ht`, `RIKU_0201_BBS` `_df`, `RIKU_0211_BBS` `_dh`; the Keyblade-of-Heart twins share their base's); `bbs_hair::SIDEBURN_REST` 0.002 → 0.0035 (cheek clearance 2.6-3.5 mm).
- **Checked**: `sideburn_probe` (lock covered, no skin over the hair on any side), `temple_game_view` (0101, 0111, 0201, 0211); 247 asset tests; renders the workspace's `local/assets/recom/riku-hair/out/temple/` (`b12_4/` 0.198.12, `fix3_own/` after); `model-reviewer` go: one sideburn, the temple in the cheek's tone, no yellow, no hole, no gap; leftovers at normal size: a ~5 px orange sliver at the sideburn tip (0101 / 0111) faintly visible, a tip smudge / diagonal step / collar specks barely visible, none worse than 0.198.12.
- **Skirt: cancelled** (fork owner, verbatim: "cancela os ajustes da saia, o jogo simplesmente não suporta direito ent deixe a ultima versão que tinha ficado boa e que eu validei"). `tails.rs` is back to its committed state (0.198.8-0.198.12's motion, the one the owner validated; `riku_skin` calls `animate` as before); Stage 412's clearance and every later try are removed from the code. What was tried and measured (`model-editor`, `model-reviewer`), for whoever comes back to it: (1) push the tails off the legs: crossing gone but the tails stood out like stiff wings (reviewer no-go); (2) retarget the tail swing to Riku's thighs (cycle phase, root follow): idle fixed (-9.6 → -0.3 / -1.0 cm), stance / battle idle / Guard still -5 to -9 cm (the back tips against the calves); (3) retarget + drape: Guard -8.5 cm, 16-125 deg steps; (4-6) a spring-cloth bake with thigh and shin capsules (bone particles, plus the coat's vertices, then the whole coat): -3.5 to -7.8 cm and 180 deg flips; (7) skinning the coat's vertices partly to the femurs / tibias (Re:CoM supports 1-6 bones a vertex, `knowledge/ReCoM/recom-mdl-skinning.md`): best per-vertex fit still -9.4 cm, stretch up to 195 %. **Cause** (Checked (files, probes)): Dark Riku's coat has 45 vertices on 38 tail bones; the legs pass through the middle of its large panels (barycentric 0.17-0.83), so no motion or skinning of its vertices can clear them; only subdividing the coat (or reshaping it) could. The attempts' code is kept outside the repo.
- **Not checked**: in game.

## Stage 412 (fork branch Re-Re-Reverse/Rebirth): Riku's casts end in the next pose; the Dark Form's loose hands; the BBS hair's single sideburn (the skirt's leg clearance written, off), version 0.198.13 (2026-11-27)

- **Ask** (fork owner, verbatim, on 0.198.11-0.198.12 in game): casts: "quero testar também as animações de magia, tem mudanças muito bruscas entre a animação e voltar para o estado idle"; "todas as magias...."; "magia é o riku normal e o ddark mode"; "o sora tem as animações do proprio jogo, e estão ok, as animações de magia que falam são do riku msm que trocamos pelas do terra do BBS". Hands: "as mãos o RIku Darkmode são muito estão muito grosseiras no andar ... também precisa melhorar, parado a mão esquerda tmb fica meio esquisita"; "no riku dark mode a animação de andar até sem esada ficou estranha ninguém caminha fazendo um C com a mão". Skirt: "no geral a animação da saia dele ficou Muito boa, mas em idle ela atravessa a perna esquerda"; "no caminhar a saia chega a atravessar a perna tbm"; "animação de defesa tbm"; "no começo da anim de pulo tbm". Sideburn: "uma coisa que ficou estranha no cabelo BBS tbm é a costeleta do cabelo, tem a do cabelo mesmo e tem a da cabeça base ao lado, ficou estranho". Rule for all: "não atravessar nem mesmo deixar muita folga.. e também deixar natural!". Done by `model-editor` agents (one per piece) and the main session (DLL).
- **Addresses**: action tables `0x1407C6370` / `0x1407C67F0` (what plays after a cast; Checked (exe), the research of this stage, `knowledge/ReCoM/recom-combat.md` § Motion). No new hook.
- **Casts** (`riku_skin::{ease_cast_ends, ease_to_next, RIKU_FALL, DARK_STANCE, DARK_FALL}`, in `build_with` for packs 101, 201 and the skin 121; `riku::idle_after_cast`, `win::movement::play`):
  - Found (Checked (files, test probe)): the Terra cast clips ended in Terra's recovery pose, 50-96 cm (hands, blade) from the next pose; the DLL blends out over 10 frames (`riku::OUT`). After a ground cast the game queues the idle (battle stance 0x2A: `p002_031` / `p003_021`), after an air cast the fall (row 6: `p002_006` / `p003_007`).
  - Change: after the gesture's end (`CastClip::key_out`) every bone eases to the next pose's frame 0, fully in by KH1's free frame; wind-ups 227 / 233 unchanged (Fire's release follows them); scale unchanged. DLL: Riku's idle request after one of his cast rows (current or previous) plays the stance 0x2A, field or battle, as after a swing (the field idle was 86 cm from the eased end).
  - Checked: `cast_ends::casts_end_in_the_next_pose` (last and free frames within 2 cm of the next pose, measured 0.0; frames up to the gesture's end unchanged < 1 mm; no faster move than the clip's own, ≤ 0.25 m a frame; no bone > 30 deg a frame: worst 12.8 in the clip, 4.6 in the blend; blade / hands no nearer the head or chest than before, nearest 25.3 cm); the 5 cast tests; `riku::tests::the_idle_after_a_cast_is_the_stance`. Renders: the workspace's `local/assets/recom/riku-hair/out/cast_ends/`.
- **Hands** (`hands::{loosen, LOOSE, joints, tips, chain, LOOSE_OFF, NO_LOOSE}`, `Clearance::side` for the left hand, the clearance limits shared with `relax`; in `build_with` before `relax`, the skin's pack only):
  - Found (Checked (files)): Dark Riku's own hand is not clawed; the skin plays Riku's idle / walk / run, whose fingers make a fist (index joints 55-84 deg) closed by Riku's thumb; Dark Riku's thumb rests ~70 deg further out (sum of bends 147 vs 77 deg), so on the skin it stays off the fingers: thumb tip to index 1.01-1.07 hand lengths in the walk (Riku 0.84-0.93) = the "C"; the right hand mirrors it (Stage 409).
  - Change: in idle, walk and run the 15 left-finger bones take Dark Riku's own loose hand (his walk `p003_002` / run `p003_003`, lined up by the arm swing; the idle takes his walk hand: his idle's thumb tip is bent back 12 deg); the right mirrors as before.
  - Checked: `loose_skin_hands_are_relaxed` (every frame: joints in range MCP 90 / PIP 100 / DIP 80, back ≤ 5, or no worse than 0.198.12; thumb nearer the index in idle and walk; no finger across the coat; no other bone moved; Riku's pack identical). Thumb to index: idle 0.80+ → 0.60, walk 0.72+ → 0.60, run 0.54-0.90 → 0.54-0.71 (fist kept where the hand passes the coat: weight 0 at a few run frames). Riku's default: 0 changed regions in 64 views.
- **Skirt** (`tails::{leg_segments, clear_legs, ease_pushes, close_loop, NO_CLEAR}`; `riku_skin` passes the model to `animate`):
  - Found (Checked (files, probe `tails_leg_probe`)): Dark Riku's tail motion laid on Riku's legs crossed them by up to 9.7 cm (idle -9.6 / -2.5 cm on +x / -x, walk -9.1 / -8.8, run -8.2 / -8.1, jump -8.4 / -8.1, stance -8.3 / -9.7, Guard -9.1 / -9.4); on his own legs his clips cross ≤ ~1 cm.
  - Change: tails pushed off the legs tips first by the smallest swing that leaves 0.6 cm, then each bone settles back toward the clip's hang (drape on the thigh); pushes eased over 2 frames; loop seams closed. Dark Riku's own clips untouched.
  - Checked: `tails_clear_the_legs`, `skirt_motion_probe`, `dark_skirt_checks` (with the Guard). Deepest crossing +x / -x, widest gap: idle +0.3 / +0.4, 0.8 cm; walk +0.4 / -1.2, 1.2; stance +0.4 / +0.4, 0.7; battle idle +0.5 / +0.3; battle walk +0.4 / -0.7, 1.1; Guard -0.2 / -0.1, 1.1. **Known, second round**: run -3.4 / -1.6 (gap 7.7 cm), jump start -2.6 at frame 1, the blends into the jump (earlier variant: -8.8 / -7.1); a tail standing off the thigh in the stance and Guard front views; a slight flare on one side in idle from behind.
- **Skirt, review** (`model-reviewer`, no-go on the skirt only): the cleared tails stood off the legs like stiff wings in the stance and the Guard (100-170 px past the hip line in 700 px renders), one tail jutting out at idle frames 25 / 50 / 70, flaps at walk frame 14 (sheets in the session scratchpad; frames `out/dark/skirt_clear_after/`). **Shipped off**: `tails::NO_CLEAR` defaults to true, so 0.198.13 builds the skirt as 0.198.12 (crossing as before); `tails_clear_the_legs` turns the clearance on for itself. Second round pending.
- **Sideburn** (`ddd_hair::{HairSource::painted_temple_window, unpaint_temples, LOCK_EDGE, FILL_FROM}`, `HairReport::temple_unpainted`; `bbs_hair::{TEMPLE_WINDOW, SIDEBURN, SIDEBURN_REST}`):
  - Found (Checked (files, `sideburn_probe`)): the second sideburn is a lock painted on Re:CoM's face texture at each temple (3-4 `head` triangles a side, 7.8-9.2 cm out from the head bone), covered by Riku's own hair, uncovered by the BBS swap; on PL0101, PL0111 and PL0201, both sides. The face texture is not edited (the `.TXA` animates eyes and lips by its name).
  - Change: those triangles point at a copy of their face patch in the BBS hair's own texture, the lock and 8 texels around it refilled from the lit skin around it; the BBS sideburn's tip and root sit 2 mm off the cheek (before 6.5 / 7.5 mm at the tip).
  - Checked: `sideburn_probe`, new `bbs_camera_checks` (0101 / 0111); KH2 hair unchanged (0 regions in 81 views). Not clean: a faint shading seam beside the sideburn tip (0101 / 0111); a few skin-coloured specks at the collar edge in the Dark Form's low walk view.
- **Dev tools** (no game change): `tools/test-affected.sh` runs `cargo nextest` when installed (`KH1_TEST_RUNNER=cargo` forces `cargo test`; no doc tests in the workspace); `tools/dev-install.sh` builds the DLL with `KH1_DLL_PROFILE` (machine-local, e.g. a thin-LTO `testing` profile), unset = release as the GitHub release.
- **Checked**: the whole affected test set, 1057 passed (nextest, 6 min 25 s).
- **Not checked**: in game; groups 131 / 211 and the KH2 / BBS hair groups for the hands and casts (rendered: the base groups).

## Stage 411 (fork branch Re-Re-Reverse/Rebirth): the BBS hair's dark shard roots lifted (less "scaly" from behind), version 0.198.12 (2026-11-26)

- **Ask** (fork owner, verbatim, on 0.198.11 in game, a screenshot of the back of the BBS hair from a high camera): "cabelo do bbs parece escamado tem como melhorar?"; then four screenshots (side, back, side, front) and one at the normal camera distance; "ss" to the numbered plan (1 lift the dark roots in the texture, 2 measure whether our crown is shorter than BBS's, 3 the dark wedges).
- **Addresses**: none (assets only).
- **Found** (Checked (renders, the workspace's `local/assets/recom/riku-hair/out/bbs/`)): the in-game back (rows of shards, 3-4 from the crown down) matches the build's back-face-culled render (`hair_back_cull.png`) in outline and rows; no part of the crown is missing in game. The kid's own model drawn the same way (`source_back.png`) has the same rows at the same height, so the fit (`SCALE`, `SHIFT`) did not shorten the crown (plan item 2: no change). The "scales" are the kid's shards painted with dark roots: each row's tips lie over the next row's roots, so every row ends on a dark band. The hair's texels (filled atlas): median brightness 79, 10th percentile 35, 75th 100.
- **Change** (`bbs_hair::{SOFTEN_KNEE 110, SOFTEN_KEEP 0.5, soften}`): the hair's texels (`is_hair_texel`) darker than 110 (mean of r, g, b) move half-way toward 110, colour scaled as a whole (hue kept); brighter texels and non-hair texels unchanged. Applied after `bleed` to the atlas the swap uses (every BBS hair group: Riku, Halloween Town, Dark Form). The classification still reads the original atlas. The dark wedges (plan item 3) are part of the same shards and are lifted with them.
- **Checked**: test `soften_lifts_only_dark_hair_texels`; the BBS hair tests; renders before (`out/bbs_0198_11/`) and after (`out/bbs/`).
- **Not checked**: in game.

## Stage 410 (fork branch Re-Re-Reverse/Rebirth): KH1's Guard holds against every attack it clashes with (priority 98-100 too), and ends when a hit gets through (Sora and Riku), version 0.198.11 (2026-11-26)

- **Ask** (fork owner, verbatim, on 0.198.9 against the Defender): "outro ponto o timing da defesa e a janela ainda está estranho teste com o Defender"; asked what felt off: "Animação fora do tempo", "Golpe passa defendendo"; on the first reading (KH1 lets a priority-100 attack hit): "PRECISA SER IGUAL AO KH1 e no KH1 a defesa funciona", "do jeito que foi usado ela funciona!", "detalhe não é só no caso do defender, eu preciso de uma correção geral".
- **The owner's log** (0.198.9): the Defender's strike (request 209) and spin (208) met Riku's blade from in front during the live window and "got through (… its type or priority)"; the guard object then stayed live 684 / 887 ms (normally 433) while Re:CoM's damage reaction played.
- **KH1** (`mechanic-reviewer`, Checked (decomp)): the Defender's strike / spin / bash rows (`0x15C` / `0x1CC` / `0x3FC`) are type 0, priority 100 (row `+0x3C` copied unchanged to the attack's `+0x68` by `0x1402B2DC0`; only weapon-bone rows add the weapon's `+0x33`, clamped 1..99, `0x1402B3150` / `0x1402B3270`). `0x1402B4A20` / `0x1402B4BC0`: the guard (priority under 98, 70 under) gets a kind-7 clash record; the mace returns 0x10 (unmoved), is not marked and still meets bodies. But the player's records go through his own loop `0x14029BBD0`, the clash record first (the clash pass `0x1402B47A0` runs before the body pass in `0x1402B4460`; records go to the list's tail `0x1402BFE60` and are taken from its head `0x1402BF160`): kind 7's `0x14029D5F0` plays 0x6F, queues the idle, sets flag 0x80 (`0x140297F30`) and lowers the combo counter; the mace's hit record that follows does 0 HP and no reaction (`0x1402BEEE0`: HP only when `actor+0 & 0x1000080` is clear). The enemy gets no record. In every case the guard holds. Any reaction replaces the Guard motion and so ends its hit track (`0x1402B8E10` → `0x14029F980`).
- **Change**: `kh1_core::enemy::guard::Meeting::a_unharmed` (`b` stopped, or `a` got a clash record). `win::attack::kh1_guard`: a KH1 enemy attack that gives the guard a clash record is `Stopped` with that record (kind 7 for 98-100: Riku's recoil row 0x3A / Sora's 0x6F, protection, no Counterattack), the enemy unmoved; general, not the Defender's case only (the Neoshadow's dive and rising slash, the Wyvern's bite …). `kh1_enemy::sweep` (Sora's swing against enemy rows): a row that gave Sora a clash record does him no harm that frame (Re:CoM's test of it against him answers 0). The guard object goes off when Re:CoM's damage check (`win::attack::damage_check`, `0x1401E6DE0`) starts its hit reaction (action 0xC) on the guarding player, with no KH1 clash record pending that frame (first try, at the pair's result 1, was a no-go of the `mechanic-reviewer`: a result 1 is not a reaction, e.g. under the post-block protection or with a second attack in the frame, and it left the port's guard off for the rest of the Guard). The start-up log line now names the guard clip's two sources (Terra's Block with BBS FM, else KH1 Riku's).
- **Review** (`mechanic-reviewer`): GO after the guard-off moved to the damage check. Open (not blocking, older than this Stage): two attacks in one frame, one blocked with a KH1 record, the other still costs HP (KH1: the first record's flag 0x80 makes it harmless).
- **Checked**: tests (`rikus_guard_against_the_neoshadows_windows` now asserts the guard holds against all three Neoshadow attacks; `sweep`'s dive case), all affected crates. **Not checked**: in game.
- **Not changed**: the window (frames 11..36 after the press, KH1's 10..36 one frame later) and the pose (same clip clock, 1-3 frames ahead of the window); the Defender's fireball (Re:CoM bullet, Re:CoM's block and turn-back as before).

## Stage 409 (fork branch Re-Re-Reverse/Rebirth): Riku's right hand relaxed in the field walk and run (weapon put away), version 0.198.10 (2026-11-26)

- **Ask** (fork owner, verbatim): "ficou PERFEITO empunhando a espada, mas sem espada na mão pode deixar o movimento das mãos mais natural, no andar corrida... antes faça uma release daquelas!" (0.198.9 released first: `v0.198.9-patched-20aa45d`).
- **Where** (Checked (renders `hand_grip_checks` with the new `NO_WEAPON`, `HAND_POSES`, `HAND_BONE` switches)): in the field walk and run (`p002_002` / `p002_003`) Riku's right hand keeps the handle's hollow "C" with nothing in it; his left hand hangs loose in the walk and opens with the swing in the run. Same on the Dark Form skin (Riku's clips on Dark Riku's model).
- **When those clips play** (Checked (exe)): `0x1401F9380` picks rows 2 / 3 when player `+0x418` bit 1 is set, else 0x12 / 0x13 (0x2B / 0x2C in the battle stance); the draw-weapon `0x1401F5D30` clears it (`and dword [reg+0x418], 0xFD` at `0x1401F5D4A`) and the instruction at `0x140216090`, just before Riku's draw `0x1402160A0`, sets it. So rows 2 / 3 play only with the weapon put away. The field idle (row 1) also plays with the weapon out (`0x1401F9380` cases 2 / 0x1F) and is not changed. The mod's own picker (`rows::row_for`) follows the same rule.
- **Change** (new `hands.rs`, called in `riku_skin::build_with` after the grip, on Riku's pack (`PL0101` / `PL0111`) and the Dark Form skin's (121 / 131)): in `p002_002` / `p002_003` the 15 right-finger bones take the left twins' rotations at the same frame, mirrored through the rest pose (`P_b = Rw_r · S · Rw_lᵀ`, right local = `P_c · L_l · P_parent⁻¹`; the model is mirror-symmetric at rest within 2e-6 m). The hand, arm and body keep the clip's motion. Dark Riku's own pack (the Dark Mode, battle only) is not changed.
- **Reviews** (`model-reviewer`): (1) no-go: on the Dark Form skin, run frame 11, the opened fingers went ~2 cm through the coat tail. Fix: `hands::Clearance` (right-finger vertices to the coat's `*_skt*` triangles); per frame the mirror's weight is lowered in tenths until the fingers keep `CLEAR` 1.5 cm, eased over ±3 frames. (2) no-go: a tip still showed through the tail from the outer side, 1.2 mm from it: an unsigned distance near the clip's own contact counts a tip on the far side as clear. Fix: the relaxed shape keeps `MIN_GAP` 6 mm where the clip's own hand is nearer (else 1.5 cm), no finger vertex may change side of the coat (parity of the coat crossings from the wrist), and each eased in-between weight is checked again (in-between shapes are no nearer than the clip's own). The skin's run weights: 0 at frames 1-2 and 11-12, 0.29-0.43 elsewhere, up to 0.71 on the back swing; Riku's own pack and the walk stay at 1. At weight 0 the hand is 0.198.9's (renders `hand_0201_v9` = the build with `hands::OFF`, identical to `rv12` at run 11). Test `relaxed_skin_hand_keeps_clear_of_the_coat` (the skin pack against the same build without the relax). (3) `model-reviewer` GO: run 11 from the outer side identical to 0.198.9 (`hand_0201_v9q` with the new `NO_RELAX` switch), no fingertip on the tail beyond 0.198.9's in any of the run's 22 frames, no pop frame to frame; Riku's pack, the walk and the grip clips unchanged.
- **Checked**: test `hands::tests::relaxed_right_hand_mirrors_the_left` (every frame: right fingers within 1 mm of the mirrored left; no other bone moves); renders `hand/hand_0101_relax`, `hand_0201_relax` (`walk0`, `walk12`, `run3`, `run9`, whole body). Not checked: in game; the blend into and out of the walk / run (the game blends the clips; the fingers turn from the "C" to the loose hand over the blend).

## Stage 408 (fork branch Re-Re-Reverse/Rebirth): Riku's right hand closed on his weapon (default attire and Dark Form), version 0.198.9 (2026-11-26)

Fork owner, 2026-11-26, verbatim, two screenshots of the hand on the Soul Eater (default attire and Dark Form): "dps tbm quero verificar se tem como melhorar o encaixe da espada na mão do Riku tanto darkmode quanto skin default, acho que tem muita folga na mão é não é acurado com a realidade"; asked whether the default attire's hand (Re:CoM's own grip) changes in every mode: "Always"; "precisa ficar natural", "e anatomicamente correto", "sem dentruir com as animações, lógico"; the pose of the screenshots: standing in battle, KH1 gameplay; "sim" to taking the grip from Riku's own clips.

- **Addresses**: none (assets only).
- **Found** (Checked (files, renders)): `wep0` sits in the hand in every clip (the blade is not off, test `riku_skin::grip_probe`); the handle is a cylinder of 1.1-2.3 cm radius with rings only at its ends (`handle_profile`). The owner's open hand is the battle stance `p002_031` (Riku's action row 0x2A): an open "C", the thumb tip 2.7 cm from the index (renders `hand_<grp>_before/stance_*` match both screenshots). Riku's own `p002_053` frame 16 holds the handle in a closed fist (thumb 0.2 cm from the index, `closed_grip_search`); Dark Riku's own `p003_008` frame 3 (0.6 cm).
- **Change** (`crate::grip::close`, in `riku_skin::build_with` on Riku's pack (101, 111) and on the Dark Form skin's (121, 131) after the tails; every group built on them takes it): per clip and frame, the 15 right-finger bones turn toward that grip by a weight growing with the frame's thumb-index opening (0 at 0.8 cm, 1 at 1.8), smoothed over 5 frames; a hand the clip already closes is left as it is; nothing else changes. Test renders `kh2_hair::tests::hand_grip_checks` (`out/hand/`, right hand with the weapon, 8 views incl. along the handle; `VIEWS`, `HAND_HALF`).
- **Tried and dropped**: bending each finger at its joints until it met the handle (`grip.rs`'s first version): Dark Riku's hand closed, Riku's did not (his fingers already touch the handle's side; the opening is between the thumb and the index).
- **Review** (`model-reviewer`, no-go): (1) an empty fist in `p002_078..081` (incl. the looped idle `p002_079`), where the hand lies flat along the weapon (opening 6.7-10 cm); (2) the fingers' own motion frozen (guard `p002_225`, item use `p002_034`, `p002_039`, 081: 3-22 deg/frame → 0); (3) up to 1 cm into the handle on the Dark Form skin (`p002_039`, `p002_004` f16); (4) the test's "before" for the skin already had the grip, and its camera moved with the fingers. Fixes (`grip.rs`): the weight fades to 0 between 4.5 and 6 cm of opening (`NOT_HOLDING`, `FADE`: the hand is not around the handle); the grip is an offset from the clip's least-open holding frame (`grip · least⁻¹ · frame`), so the fingers keep their motion; per frame, no finger deeper into the handle (its end rings' median radius) than the clip's own or the grip's own pose plus 3 mm (`MAX_DEPTH`), the weight lowered in tenths; `NO_GRIP` (test) builds the skin without it; the close-ups centred on `r_hand`, new views (`idle79`, `s81`, `item34`, `s39`, `jump16`). Renders `g10`: the battle stance closed on both models (Riku's thumb 2.7 → 0.18 cm from the index; Dark Riku's "C" of 4.5 cm closed); `p002_079` unchanged on both.
- **Second review** (`model-reviewer`, no-go): (A) the per-frame weight closed and opened the fist inside a clip (walk twice a cycle; the skin's item use and guard); (B) `p002_010` opened wider than its own pose (skin up to 10 cm). Fixes: one weight per clip from its median opening (the per-frame limits only lower it); per frame, the opening no more than the clip's own + 5 mm (`MAX_WIDER`), the weight lowered in tenths. Test `grip_close_report` asserts it on Riku's and Dark Riku's packs and lists any frame-to-frame turn of the grip over 12 deg (none). Renders `g11`: the battle stance closed on both models.
- **Third review** (`model-reviewer`, no-go on the Dark Form skin only; Riku's pack passed): the skin's `p002_010` opened past its own clip (2.5-4.8 → 1.0-7.0 cm) and its walk's fingers moved faster (4.0 → 14.7 deg/frame). Cause: the skin was retargeted from Riku's already-gripped pack, so its own grip's limits measured an already-changed clip. Fix: the skin retargets Riku's pack as it was before the grip (`ann_plain`) and takes only Dark Riku's grip. Test `skin_grip_bounds` (the skin's pack from `build_with`, with and without `NO_GRIP`): no clip's fingers faster than their own + 2 deg/frame (it passes; the first version of this test and the "no turn over 12 deg" check above converted the clips' radians to radians again and measured almost nothing; fixed).
- **After the third review**: `skin_grip_bounds` then failed on the skin's jump (`p002_004`: still fingers moving 8.3 deg/frame): the per-frame limits lowered the weight in some frames only. Now the limits pick one weight for the whole clip (the largest, from the clip's own down in tenths, that keeps every frame within them). The tests' switch is per thread (`grip::OFF`; an env var raced between parallel tests). Renders `g12`: the battle stance closed on both models.
- **Fourth / fifth review**: each finger's tip no farther from the handle's axis than the clip's own + 5 mm (fingers 3-5 had stood straight off the handle in the skin's `p002_010` in a build between reviews); `model-reviewer` GO: one constant turn per clip, finger speed equal to the clip's own in every clip checked, the stance closed (Riku 2.7 → 0.2 cm, Dark Riku 4.5 → 0.6), walk / guard / 078 / 079 / 081 untouched, body unchanged (≤ 4e-5). To look at in game: the Dark Form's jump (`p002_004`, fingertips 6 mm onto the braid).
- **In game (owner, 0.198.9)**: "ficou PERFEITO empunhando a espada".
- **Not checked**: in game; the KH2 / BBS hair groups (built on these packs) not rendered for the hand.

## Stage 407 (fork branch Re-Re-Reverse/Rebirth): the Dark Form skin's coat tails move, version 0.198.8 (2026-11-26)

Fork owner, 2026-11-26, verbatim, a screenshot of the Dark Form's coat tails: "no darkmode queria ver se é possível animar a saia do riku fica muito estática"; later "tem como melhorar ainda mais a animação da saia?"; "yes" to the numbered plan (1 Dark Riku's own tail motion where the clip's role is known; 2 a baked follow-through elsewhere; 3 whole-body renders and `model-reviewer`; 4 the owner's test, commit, release).

- **Addresses**: none (assets only).
- **Found** (Checked (files), test `riku_skin::skirt_probe::skirt_motion_probe`): the skin's 38 coat-tail bones (`*_skt*`) stay at rest in all its 64 clips (0.0 deg; the skin plays Riku's motions, which have no tail bones); Dark Riku's own clips turn them up to 10.7 deg idle, 19.2 walk, 110 run, 120-130 jump / fall. His slots pair with Riku's by number (`p002_NNN` / `p003_NNN`); idle, walk, run and jump also by length and loop (100 / 80 loop, 32 / 34 loop, 22 / 20 loop, 17 / 17); the other numbers differ in length or loop (not paired).
- **Change** (`crate::tails::{COPY, SPRING, animate}`, called in `riku_skin::build_with` after the skin's pack is made): for each skin clip whose tails are still at rest: Riku's idle (1, 31, 79), walk (2, 28, 32, 80), run (3, 21, 33) and jump (4, 26) take Dark Riku's slot 1 / 2 / 3 / 4 tail rotations, time-scaled (cycle onto cycle when both loop); every other clip gets a follow-through: each tail chain's root turns about its origin by a damped spring (1.6 Hz, damping 0.35, lever 0.35 m, at most 55 deg) driven by minus the root's world acceleration, looped clips simulated twice and the second pass kept. Clips whose tails already move (Dark Riku's own cast and flip clips in the skin) are kept. All Dark Form groups (121, 131, 161, 171 and the hair groups 184-199) take it (built on 121 / 131). Test renders: `FULL_BODY` (every bone and part 3's list B) and `kh2_hair::tests::dark_skirt_checks` (`out/dark/skirt_<tag>/`, `SKIRT_BEFORE=1` the old still tails).
- **Checked** (renders `skirt_before` / `skirt_after`): run: the tails fan back as on Dark Riku; walk: a light swing; the first ground swing (simulated): the tails lag on the lunge (f6), follow (f12) and settle (f18), not through the legs in the side view. The probe asserts the skin's run and first swing turn the tails more than 10 deg.
- **Review** (`model-reviewer`, no-go): (1) the tails popped where looped clips with a later loop start go back (slot 26 73 deg, 52 55, 57 52, 58 28, 35 11); (2) the rear thigh through the back tail in the first ground swing (frame 12, side), the spring having swung it forward to vertical. Fixes (`tails.rs`): Dark Riku's tails are copied only on clips that loop whole or not at all (slot 26 loops from frame 16: simulated); the follow-through simulates a loop's segment (`loop_start − 1` to the end) until its start repeats (≤ 0.1 deg, at most 12 passes) and eases the frames before it from rest into it; the springs act along the body's forward and side axes (the parent's rest +z / +x carried by its world pose) instead of the world's, with the tip forward at most 15 deg, back 55, sideways 35 (`SWING_LIMITS`); the drive is clamped to what holds the widest swing and the turn to 300 deg/s (`SWING_RATE`; the first fix still took 26-53 deg steps on acceleration spikes). Checked (probe, now a test): every looped skin clip's tail jump at its loop ≤ max(5 deg, 1.5 × its largest step) (slots 26 / 35 / 52 / 57 / 58: 0.1 / 0.1 / 0.1 / 0.3 / 0.1); the simulated clips' largest steps 9-14 deg; renders `skirt_fix`.
- **In game** (fork owner, on 0.198.8 after the second review's go: "andando e correndo ficou perfeito, o problema foi pulando"; "retificando, pulando segurando o B"; "quickjump está perfeito tbm"; "pra vc saber eu segurei B e não soltei"): the held jump goes on through the apex, the fall loop and the landing, which took the follow-through starting from the tails at rest right after the copied jump's spread tails. Checked (exe: the action tables `0x1407C6370` / `0x1407C67F0`, rows 4-7): Riku's jump / apex / fall / land are `p002_004..007`, Dark Riku's `p003_004` / `_006` / `_007` / `_008`. Fix: [`COPY`] also takes Dark Riku's apex (6 → Riku's 5), fall (7 → 6) and land (8 → 7); the copy blends Dark Riku's frames as rotations (its Euler blend across a ±180 deg wrap made a 123 deg step on the apex; now 19). Checked (probe): the held jump's seams equal Dark Riku's own (jump → apex 6.7 deg, apex → fall 17.0, fall → land 39.3 against his 38.4); renders `skirt_jump4` (jump, apex, fall, land).
- **Assumption**: the spring's constants; that copying Dark Riku's idle / walk / run / jump tails onto Riku's body motion reads right (their gait timing differs a little: 32 / 34, 22 / 20 frames).
- **In game, after the held-jump fix**: fork owner, verbatim: "está perfeito agora" (walk, run, quick and held jump, before: "andando e correndo ficou perfeito").

## Stage 406 (fork branch Re-Re-Reverse/Rebirth): KH2's and BBS's hair on the Dark Form skin, version 0.198.7 (2026-11-26)

Fork owner, 2026-11-26, verbatim, on 0.198.6 in game: "muito melhor! ficou quase perfeito, agora implementa todos os cabelos para o a Skin do riku Darkmode tbm , comita e lança uma release"; then "sim" to the numbered plan (1 commit and release 0.198.6 first, done: v0.198.6-patched-e39dd58; 2 research `PL0201`; 3 generalize the swap and put the KH2 and BBS hairs on it; 4 the event packs / clips at the new bone count; 5 camera checks and `model-reviewer`; 6 the owner's test, commit and a second release).

- **Addresses**: none (assets only).
- **Found** (Checked (files), test `dark_probe`): Dark Riku's `PL0201` / `PL0211` (the Dark Form skin's model, groups 121 / 131): part 0 has packets whose header words 2 and 4 hold a count other than the vertex count (49 vertices, 45 there), which the reader refused; part 3 has textures `n_zz120_03` / `_02` / `_01` (`_13` / `_12` / `_11` on 0211) and a list B of 7 packets: the coat tails only (`*_skt*`, `hip`, `waist`, on `_02`), no hair or head; his hair is on `_01` (with the face; 164 hair-bone vertices) and `_02` (with the coat; 90), plus pieces on `head` in `_02`'s hair islands. Every part of both models reads and writes back byte for byte with the change below.
- **Change**:
  - `mdl::MeshPacket::count`: header words 2 / 4's count kept as read when it is not the vertex count, written back the same.
  - `ddd_hair`: `HairTarget::also` (a second hair texture: its triangles in hair islands that touch the head or a hair bone are removed, `HairReport::also_removed`); part 3's list B kept as it is when it has no hair or head vertex (and counted in the part's bounds); `RIKU_0201_KH2 / _BBS`, `RIKU_0211_KH2 / _BBS`.
  - `riku_skin::{DARK_KH2_HAIR_GROUPS (184, 194, 188, 198), DARK_BBS_HAIR_GROUPS (185, 195, 189, 199), build_hair_from}`: the skin's groups 121 / 131 (built first) with each hair, with the Soul Eater and the Keyblade of Heart; RTM / TXA from Dark Riku's 201 / 211; in the asset step (`step.rs`) and the generated files list (`keyblade.rs`). The DLL needed no change: it picks the group by the look's dark / blade / hair (`riku_skin::group_for`), the camp menu's Hair row applies to every costume.
  - Test renders: `kh2_hair::tests::{view_at, dark_camera_checks}` (`out/dark/<kh2|bbs|recom>_<tag>/`, `DARK_HAIR=`); renders framed by the body (no head / hair corner) with fixed margins, so a hair change no longer moves the picture; the run's side and nape views in every camera check.
- **Checked**: test `the_dark_hair_groups_are_built_from_the_dark_skin` (all 8 groups; KH2 139 bones, BBS 128; every clip at the model's bone count; the body's hair changed; the coat's list B kept; the groups are free in Recom.hed); renders `out/dark/*_cam3` against the skin as it is (`recom_cam3`, `render-diff`): changes only on the head and hair; the first build left Dark Riku's `head` pieces of `_02` as dark spikes above the ears (`kh2_cam`), removed (`cam2`); KH2's hair on the Dark Form looks as on Riku (face close-ups side by side), down in the run and over the nape in the walk.
- **Review** (`model-reviewer`, no-go): (1) a flat grey dome between the new hair and the collar in the walk from below (both hairs), and a grey tab in BBS's close-ups: Checked (test `dark_probe`, the `_02` triangles left after the swap): 8 whole `head` / `neck*` triangles of the old hair's base over the nape (|x| ≤ 2.8 cm, 0.8-4.5 cm under the head bone, 6-9 cm behind), in no hair island; now removed with the second texture's hair (`ddd_hair::ALSO_NAPE` [0.04, -0.06, 0.0, -0.05]); renders `out/dark/*_fix1`: the dome and the tab gone, the collar closed under the hair (no hole). (2) The reference render drew the face from the SD TIM2, the others from the HD DDS: now the HD asset; the face's skin pixels in both renders are identical (19 057 / 17 099 / 20 882 in three close-ups, `cam4`).
- **In game** (fork owner, on 0.198.7 after the review's go, a screenshot of the Dark Form with KH2's hair: long straight locks sticking out sideways, "bizarro hein..."; "só aconteceu com o cabelo do KH2 no darkmode"; it stayed after a room change). Ruled out by measurement: the video-memory packet budget (largest packet 107 qw of 208), the PC converter's 48 bones per batch (largest 23), bone indices of 128 and up (Re:CoM's own Sora, PL0405/0407 and enemies skin vertices to them), a runtime writer of hair bones in the DLL (none), the clips (all 139 bones; posed with the game's own `PL0184` files no hair bone leaves the head, test `game_group_probe`). Cause, Checked (files: the game's `PL0184.MDL` against the test's, the rest matrices of the hair bones differ; code): `Kh2HairSource` kept one rig (`OnceCell`) from the first skeleton it saw; the asset step builds Riku's groups first, so the Dark Form got Riku's rig (his bone indices, his fit); the tests used a fresh source. Fix: one rig per skeleton (key: its bone names) and the hair's triangles take the rig of their skeleton; test `one_kh2_source_builds_riku_then_the_dark_form_as_a_fresh_one` (the shared source's Dark Form group is byte for byte the fresh one's, i.e. what the renders and the review saw). BBS's source makes its rig per model and was not affected.
- **Assumption / known**: the KH2 hair makes the Dark Form 139 bones (the skin is 128): Riku's event packs have no copies at those counts, so the boss-end card get waits on them as it already does with the Dark Form skin (the soft-lock audit of 2026-11-25, item 1, not fixed). The Halloween Town Dark models (188 / 189 / 198 / 199) are built and tested, not rendered. The battle's Dark Mode (groups 201 / 211) is not changed.
- **Not checked**: in game. The user must press Build.

## Stage 405 (fork branch Re-Re-Reverse/Rebirth): the long back locks split off KH2 bones 101 / 104 and turned down in the walk and run, version 0.198.6 (2026-11-26)

Fork owner, 2026-11-26, verbatim, on 0.198.5 in game: "sim, e já aproveita e continue com a correção da nuca" (yes to separating the long back locks from the cap, Stage 402's proposal), two screenshots from behind with a box at the nape: "eu testei andando e corrento com a camera baixa"; then a side view of the run: "Cabelo levanta DEMAIS!".

- **Addresses**: none (assets only).
- **Found**: run, low camera (`images/5.png`, x 100-106, y 62): (191,188,211) above the collar, the background (196,193,215): a hole through the nape; walk (`images/6.png`): neck skin and the grey piece above the collar. Checked (fitted vertices per Re:CoM bone): the Re:CoM bones of KH2 101 (`b_hair2`) and 104 (`b_hair1`) carry no cap: all their vertices are 15.7-41.9 cm from the head's centre (the cap's skin is the head bone's lining); [`NO_TILT`] had kept them out of the extra turn. Checked (test `run_lean`, now `LEAN_SLOT` / `LEAN_CLIPS`): KH2's walk tilts the head 30 deg forward (bend 44.5 against the upper chest), Re:CoM's walk (clip 2) 6 deg (11.2); the run 37 / 1.4 (Stage 402).
- **Change** (`kh2_hair_rig::{SPLIT, SPLIT_R [0.14, 0.17], SPLIT_Z -0.03, Rig::split}`, `extend_skeleton`, `Rig::new`, `tilt_hair`; `Kh2HairSource::hair_tris`; `EXTRA_TILT` run -36, walk -24): four bones appended after the 7 extra ones (`k2_hair_101a/b/c`, `k2_hair_104c`; parent `head`, the same rest and KH2 motion as their source; Riku 109 → 113 bones, the event pack copies follow the count). A vertex that follows `b_hair2` / `b_hair1`, 3 cm or more behind the head bone, goes to `101a` (1/3 of the turn) under 17.5 cm from the head's centre, `101b` (2/3) under 19.5 cm, `101c` (all) beyond; 104's all to `104c` (101a 100, 101b 136, 101c 304, 104c 196 corners). The first build had two steps (½ at 14-17 cm, 68 corners, then all): `model-reviewer` found a straight seam across the run's back from the high camera (`run_high_f6` / `f17`), softened with the three steps (`cam_0198_6d`). In the clips on KH2's walk and run, each turns by its fraction of `EXTRA_TILT` about the source's origin; the other back-rooted bones by all of it (as 0.195.1-0.196.5). The amounts are the measured world head-tilt differences (Assumption: that matching the world tilt is the right measure; the torso-relative difference is 20 deg run, 33 walk). `kh2_hair_moves_as_kh2` skips the tilted clips (they leave KH2's motion on purpose). New debug: `BONE_COLOURS` (hair flat-coloured by bone in the camera checks), walk views in `camera_checks` (`walk_{low,high,side90,side270}_f*`).
- **Checked** (renders `cam_0198_6c` against `cam_0198_5` / `cam_walk_base`, `scripts/render-diff.py`): idle views unchanged; changes only in the walk and run views: the back hair lower in the side views, the nape covered down to the collar from below (run 6 / 17, walk); in the run face close-ups only hair moves (side locks by the neck, a fringe edge by a few pixels, a spike by the ear dropped), nothing new over the skin; 3-33 px render noise on the shirt and under the arm with no visible difference. All hair tests (34).
- **Review** (`model-reviewer`, no-go on the two-step build): the run's nape hole of `images/5.png` is closed (312 background pixels above the collar → 0); the walk's grey piece of `images/6.png` is unchanged: Checked (`PICK`, the render's triangle at a pixel): head-bone triangles with the hair's texels, the KH2 hair's lining (Stage 392) seen from below, the neck skin under it; this change does not move it. Also: about 5 hair tips under the swung arm in run 11 (side), against 2 (not settled by renders); from the owner's low camera the run's outline reads as a dome with flatter locks. New debug `PICK=x,y` (camera checks print the triangle drawn there), walk nape close-ups (`closeup_back_walk_f*_p{-15,-25,0}`).
- **After the review** (fork owner: "ele dando no-go tem que prosseguir para o que o agente apontou!"): (1) the seam: three steps (above); (2) the walk's nape: the nape curtain again (`HairSource::nape_curtain_hair`; `kh2_hair::{NAPE_HAIR_BOTTOM -0.055, NAPE_HAIR_BONE "chest1", NAPE_HAIR_TUCK 0.015, NAPE_HAIR_X 0.03}`), now on the hair's texels (one texel per strip, the hair vertex nearest its middle) and ending inside the collar: Checked (files, rest): the vest's top behind the neck is 4.5 cm under the head bone, on `chest1`; the curtain's bottom corners are 1 cm under it, pulled 1.5 cm toward `chest1`'s origin, on `chest1`, so they stretch as the neck bends and the vest covers them. Open head edges within 3 cm of the middle (2): 9 cm and 5 cm wide showed the curtain beside the neck from the front (`cam_0198_6e` / `6f` against `6d`: a flat plate, and with a texel per corner a stripe of other texels). `cam_0198_6g`: front views unchanged; the walk's nape at -15 / -25 deg covered in the middle (no grey piece, no skin there); left: a small skin triangle left of the middle at -15 and a thin skin line at -25; from the side and 25 deg, a narrow dark strip behind the neck where the lining's grey was.
- **Second review** (`model-reviewer`, no-go: the owner's "Cabelo levanta DEMAIS!" reduced, not gone; "make this a number"): test `back_lock_angle`, per back-rooted lock (14) the root → tip direction against the head → chest line: KH2's own run 74.0 deg, walk 72.4; ours with no extra turn 80.0 / 79.8, -24: 73.8 / 74.0, -36: 71.1 / 71.2, and it stops gaining past -60 (68): against the back, KH2 is already matched; KH2's locks look low because its body leans far forward, Riku's is upright. Found on the way (Checked, `head_rest_convention` and a per-bone turn measure in `back_lock_angle`): each lock turned the asked angle about the head's side axis, but about the bone's origin, mid-lock; now about the lock's root (`tilt_hair` `roots`: the bone's KH2 vertex nearest the head bone at rest). New approach (the stop rule): in the walk and run the back-rooted locks (and the split ones) take [`MOVE_KEEP`] 0.5 of KH2's walk / run and 0.5 of KH2's idle at the same time (`rot_toward`, the hair hanging), then the extra turn (run -36, walk -24). Renders `blend_*` (side, run 6: 0.198.5, -36, keep 0.5, keep 0.5 + -36, keep 0.25) → 0.5 + -36 chosen. Assumption: 0.5. `cam_0198_6h` against `6g`: idle and orbit unchanged; run / walk only: back hair in a slant from the side, the nape covered by hair at walk -15 and run -25, no seam from the high camera; in the run face close-ups only the side locks by the neck move (and they cover more of the collar).
- **Not checked**: in game; the motion of the walk / run hair is half KH2's (less lively than KH2's); the seam between the cap and the `a` / `b` bands in motion; jump, fall and flinch keep KH2's motion as it is.

## Stage 404 (fork branch Re-Re-Reverse/Rebirth): 0.198.4's kept Re:CoM nape hair removed; close-ups in the camera checks, version 0.198.5 (2026-11-26)

Fork owner, 2026-11-26, verbatim, on 0.198.4 in game, two screenshots (the face from the front with a ring on the cheek; the back in idle with a ring round the hair and the vest): "você não viu isso??? , já cria um agente e skills especializad@s em validar esse tipo de alteração para esses errors não acontecerem mais".

- **Addresses**: none (assets only).
- **Found**: (1) the face: thin grey-blue lines across the cheek and beside the nose; (2) the back: a row of pale flat "teeth" of hair hanging over the vest under the KH2 hair. Both reproduced in close-ups (`cam_0198_4/closeup_face_idle_f25_y25`, `closeup_back_run_f6` / `f17`) and both gone with Stage 403's kept triangles left out (`cam_nokeep`). Stage 403's checks missed them: the 350 px sheet cells show the face at about 40 px, and `nape_f6` did show the pale block over the vest, misread as "a layer down to the collar".
- **Change**: `HairSource::keep_recom_hair`, `HairReport::kept_recom` and `kh2_hair::{RECOM_NAPE_*, in_recom_nape_window}` removed: the swap is 0.198.3's (renders: the 21 views both versions have are identical, byte for byte). New `kh2_hair::tests::{render_focus, render_view}` close-ups (a world point and a half-width, 700 px) in `camera_checks`: the face from the front and 25 deg to each side (idle 0 / 25 / 50 / 75, run 0 / 6 / 11 / 17) and the back of the neck (idle, run), `closeup_*.png`. Test `recom_own_nape` (Re:CoM's own Riku textured) kept as a reference.
- **Checked** (renders `cam_0198_5`): no lines on the face; no pale block on the vest; the nape as in 0.198.3 (open in the run from below: the owner's box of Stage 403 is back).
- **Not checked**: in game. The user must press Build.

## Stage 403 (fork branch Re-Re-Reverse/Rebirth): Re:CoM's own hair kept over the nape under the KH2 hair, version 0.198.4 (2026-11-26)

Fork owner, 2026-11-26, verbatim, on 0.198.3 in game, a screenshot from behind with a yellow box at the nape over the collar: "de fato a faixa estar vazando foi ajustada"; asked whether the box meant the grey piece above the collar, the bare nape, or both: "c".

- **Addresses**: none (assets only).
- **Found**: the box (x 78-115, y 84-102 of 222x238): neck skin in shadow (101,41,25) at the sides and a grey-blue piece (65,60,72) in the middle, above the gold collar, under the hair. Reproduced (render `cam_0198_3/nape_f6_p-15`: skin and the grey piece above the collar). Checked (render `cam_recom`, Re:CoM's own Riku textured, test `kh2_hair::tests::recom_own_nape`): Re:CoM's own hair comes down to the collar and covers both; its triangles there are on `head`, `b_hair2` and `b_hair3` (atlas texture). The swap removed them with the rest of Re:CoM's hair.
- **Change** (`ddd_hair::HairSource::keep_recom_hair`, `HairReport::kept_recom`; `kh2_hair::{RECOM_NAPE_Y [-0.12, 0.0], RECOM_NAPE_Z -0.02, RECOM_NAPE_X 0.08, in_recom_nape_window}`): Re:CoM hair triangles with their centroid at rest 0-12 cm under the head bone, 2 cm or more behind it and within 8 cm of the middle are kept (71 triangles, Riku's atlas, texels unchanged); their hair-bone corners are moved onto `head` (same world position at rest), since KH2's motion now drives Re:CoM's hair bones. BBS and DDD keep none (the default).
- **Checked** (renders `cam_0198_4`, textured): run 6 / 17 from below: the nape is covered by a layer of Re:CoM's hair down to the collar; no skin and no grey piece; the orbit, the run high / low and the run's sides show no odd piece. All hair tests (34).
- **Not checked**: in game; the kept piece is rigid with the head, so with the head bent far back it may go into the collar (not seen in the renders); its colour is Re:CoM's hair texture, a little lighter than the KH2 hair in the renders. The user must press Build.

## Stage 402 (fork branch Re-Re-Reverse/Rebirth): the nape curtain removed; the camera checks textured; KH2's own Riku in the same cameras, version 0.198.3 (2026-11-26)

Fork owner, 2026-11-26, verbatim, on 0.198.2 in game, a screenshot from behind in the run, an arrow at the top of the collar: "além de não ter ficado bom, vazou pelo pescoço a tentativa de forrar o vão que esta aparecendo não tem como melhorasr a parte de trás do cabelo como um todo?"; then "ss" to the numbered proposal (1 remove the curtain, 2 render KH2's own Riku in the same cameras, 4 texture the test renders; 3, the redo of the back, waits for the comparison).

- **Addresses**: none (assets only).
- **Found**: the screenshot's spot (x 100-128, y 140-146 of 276x264): a gold band with a light edge and dark red-brown pixels (73,21,11) over the vest's collar. Reproduced (render `nape_f6_p-15`, run frame 6, 15 deg from below, now textured): the gold band is Riku's own yellow collar (there without the curtain too); the dark red-brown strip beside and under it is Stage 401's skin curtain, shaded, in front of the collar. 0.198.2's checks drew the body flat grey-blue, so the curtain and the collar looked alike there.
- **Change**: `Kh2HairSource` no longer gives `nape_curtain` (the swap's default, none): no curtain edges (`kh2_camera_checks` asserts 0). The camera checks draw the body with its own textures (`kh2_hair::tests::{body_textures, render_cam}`; Checked (files): part 3's texture table is `n_zz110_02` (vest), `n_zz110_01` (face / skin atlas), `rk_hair_kh2`, so its index 0 is the `.RTM`'s entry 1 and the reverse; they are matched by name). New debug test `kh2_hair_rig::kh2_self_cam::kh2_self_camera_checks`: KH2 Riku with his model, textures and the emulator's matrices of his run (slot 8, `emu_full`, two samples per frame) and idle, yaw every 45 deg at -35 / 0 / +35 deg (`out/kh2/kh2_self_cam/`).
- **Checked** (renders): without the curtain the dark strip is gone (`cam_before_0198_3` vs `cam_0198_3`, run 6 / 17 from below); no background through the nape in the 12 run views and the 8-view orbit. KH2's own run (renders): Riku leans far forward and the back hair fans out in spikes from behind, as ours does from below; his nape is covered by KH2's vest, whose collar comes up the neck (Re:CoM's does not). From above, KH2's fan stays spread, while ours hangs long and smooth (Re:CoM's run is upright). Assumption: the different look comes from the different lean and collar, not from the hair motion (the hair bones follow KH2's own curves, Stage 397).
- **3(a) measured, not built** (fork owner: "a", the run's back hair turned down by the body's difference): test `kh2_hair_rig::tests::run_lean` (Checked, emulator and clips): KH2's run tilts the head 37 deg forward of its rest (32.7-41.1) and bends it 46 deg against the upper chest beyond rest; Re:CoM's run (clip 3) 1.4 deg and 26 deg (clips 21 / 33: 16.5 / 30.5 and 17 / 22). Renders with `EXTRA_TILT` run -20 / -36 / -90 (`cam_tilt*`, `cam_chain*`, new side views `run_side_*` in `kh2_camera_checks`): the long back locks hardly move even at -90. Found (vertex counts per bone, run frame 6): they are on `b_hair2` (KH2 101, 540 vertices) and `b_hair1` (KH2 104, 196), the bones [`NO_TILT`] leaves out as "the skull cap"; they carry the cap and the long back hair together. A per-lock turn about the lock's root was tried and dropped (KH2's kept hair bones have no kept parents, so each lock is one bone already). `EXTRA_TILT` stays 0; the build is 0.198.3's.

## Stage 401 (fork branch Re-Re-Reverse/Rebirth): the nape closed with a curtain of skin, not a copy of the hair; camera checks as renders, version 0.198.2 (2026-11-26)

Fork owner, 2026-11-26, verbatim, on 0.198.1 in game, a screenshot from a low camera behind Riku: "tampo mas ta um pouco estranho e ta estático tbm, sempre que fizer essas alterações tenta fazer um teste no jogo correndo com a camera pra cima e outra para baixo  e outra circulando o boneco em idle".

- **Addresses**: none (assets only).
- **Change** (`ddd_hair::with_hair` + `HairSource::nape_curtain`, `HairReport::nape`; `kh2_hair::{NAPE_BOTTOM -0.095, NAPE_Y [-0.03, 0.06], NAPE_Z -0.02, NAPE_X 0.09, NAPE_BONE "neck3", in_nape_window}`): 0.198.1's copy of the hair over the nape (76 spiky triangles rigid on `neck3`, the "estranho e estático") is removed. Each open head / neck edge of Riku's kept face behind and under the skull base (8 edges, Checked (files): `head` / `neck1`, 0-4 cm under the head bone) gets a two-sided strip of its own skin straight down to 9.5 cm under the head bone (the collar): top corners the edge's own (bones and texel), bottom corners on `neck3` (Checked (files): the neck's base, 10.3 cm under the head, at the collar), so it stretches with the neck. With the hair lifted, the nape shows neck skin instead of the background.
- **Camera checks** (`kh2_hair::tests::{render_body_cam, camera_checks, kh2_camera_checks}`; the owner's three in-game checks, done as renders since the session cannot drive the game): run from behind with the camera low and high (four frames each), and the idle circled every 45 deg, light background so a hole shows; `out/kh2/cam/sheet.png`, plus nape close-ups of the run at 15 / 25 deg from below.
- **Checked** (renders): in the run the nape shows skin under the lifted hair, no background through it; the orbit and the high camera show no odd piece. A small dark triangle at the top of the collar is Riku's own (there before any change). All hair tests.
- **Not checked**: in game. The user must press Build.

## Stage 400 (fork branch Re-Re-Reverse/Rebirth): a nape filler under the KH2 hair, version 0.198.1 (2026-11-26)

Fork owner, 2026-11-26, verbatim, on 0.197.7 in game: "eu testei o do bbs ta perfeito, e o do kh2 se vc correr vai perceber que na nuca pode ser necessário um enchimento ..."; then a screenshot from behind, running, an arrow at the nape.

- **Addresses**: none (assets only).
- **Found**: the screenshot shows the background (light grey) between the lifted hair and the collar, neck skin at its sides: a hole. Checked (files: the swapped model's open edges): Re:CoM's head ends at the back of the skull base (open `head` / `neck1` edges 0-4 cm under the head bone, 3-7 cm behind it) and the next surface is the collar (`chest1` edges about 8 cm lower): Re:CoM's own hair covered the nape. Reproduced (render, run frames 6 and 17 from behind): a dark notch over the collar, the neck skin around it.
- **Change** (`kh2_hair::{NAPE_Y [-0.15, -0.03], NAPE_Z -0.02, NAPE_X 0.09, in_nape_window, nape_filler}`): the KH2 hair triangles over the nape at rest (centroid 3-15 cm under the head bone, behind 2 cm behind it, within 9 cm of the middle) are written again on `neck3`, pulled 8 % toward the head's centre (as the lining), same texels, two-sided, no plain-proxy copy: 76 triangles. They stay over the nape when the hair lifts.
- **Checked**: all hair tests (`kh2_hair_does_not_cross_the_skin`; lining + filler 287 triangles); renders before / after (`out/kh2/rig/`: run 0 / 6 / 17 and idle 75 from behind): the notch and the bare neck are covered by hair in the run; at rest no straight-edged pieces (the Stage 398 issue).
- **Not checked**: in game; whether the filler, rigid on `neck3`, shows when the head turns far from the neck. The user must press Build.

## Stage 399 (fork branch Re-Re-Reverse/Rebirth): Riku learns his spells through his story (floors and bosses), version 0.198.0 (2026-11-26)

Fork owner, 2026-11-26, verbatim: "já foi implementado quando desbloquear as magias do Riku e cada upgrade? ex Fire, Fira, Firaga?"; on the four ways (floor, boss, level, Dark Mode): "mas pela quantidade de magia e upgrade qual dessas estratégias sobra?"; to floors + bosses and the research: "sim"; to the spec's four questions: "dps de vencer malevola, não, deixa o al unlock quieto,  3 recalcula ss,".

- **Read first** (the workspace's notes): `knowledge/ReCoM/recom-riku-story-floors.md` (searcher: his 17 battles, rooms, enemy groups, the player-ordered floors B11-B8 / B7-B4) and `recom-riku-story-progress.md` (reverse-engineer, Checked (decomp + exe + files) unless tagged): the floor reached is Re:CoM's (0, 0x35), the highest floor label (−12 = B12 … −1 = B1; raised only by the floor setter `0x140234630`, set to −666 at a new game); a boss's win is his enemy card's discovery state (≥ 2, `0x14015B3F0`; given by `CBossDeadDemo`'s reward `0x140187AA0`), except Vexen, Riku Replica I and II, whose wins are the hall scripts' bits of (0, 0x2A): 0x80, 0x400, 0x80000000; the spell bytes of our block (`save + 0x267A`) are free in Riku's save (Sora's sync never runs in his story).
- **Rule** (`crate::riku_unlocks::{levels, derive, COUNTED, UPGRADES, FIRE_CARD}`, gameplay-designer's spec, approved): Fire when Maleficent is beaten (card `0x6D`); Blizzard on reaching B11, Thunder B10, Cure B9, Gravity B8, Stop B7, Aero B6; the k-th of 14 counted wins (the eight world bosses, Vexen, Replica I, Lexaeus, Darkside, Zexion, Replica II; not Maleficent, the Ansem tutorial or the final Ansem) gives Fira, Blizzara, Thundara, Cura, Gravira, Firaga, Stopra, Aerora, Curaga, Stopga, Graviga, Blizzaga, Thundaga, Aeroga, whatever boss it was (order-free); an upgrade waits for its spell; at most 3. In normal play Firaga lands at Replica I (B8), ahead of KH1's Stopra / Aerora (Riku has no Stop or Aero yet there).
- **Runtime** (`win::unlocks::{riku_sync, riku_levels}`, `win::command_menu::riku_spell_levels`, `win::models::group_load`): derived from the save at a boss card's first pickup (the existing wrapper on `0x14015C7F0`) and at each room / battle build, merged into Riku's save's spell bytes (never lowered), each rise logged (`unlocks: Riku learned Fire …`, `Riku Fire -> Fira …`); his command menu and Customize read them (0.184.0-0.197.7: all seven at level 3). Old saves are recalculated from their progress (the owner's choice: their spells may drop). `all_unlock` unchanged (the owner: "deixa o al unlock quieto"); `riku_free_magic` unchanged.
- **Checked**: `riku_unlocks` tests (the spec's table, each spell twice with -ra before -ga, no upgrade waits in normal play, waits and caps, cards and bits counted), all tests, the Windows build.
- **Not checked**: in game. Assumptions: Zexion's Dark Mode end gives his card like the others; a lost battle never sets a hall bit (open question (b) of the note); a shortcut holding a spell not learned shows as Re:CoM's blank, greyed slot (read in the code, not seen). The user must press Build (no asset change; the DLL only).

## Stage 398 (fork branch Re-Re-Reverse/Rebirth): the KH2 lining follows its locks; the BBS sway's jerk on `R_kamiB` fixed, version 0.197.7 (2026-11-26)

Fork owner, 2026-11-26, verbatim, on 0.197.6 in game (`tools/status.sh`: game ran 0.197.6), four KH2 screenshots with arrows at straight edges across the locks (nape, behind the ear, the back) and two BBS screenshots boxing the lock above the right ear in idle: "ficou melhor de um jeito e ruim do outro percebi que vc preencheu oq faltava no cabelo do kh2 com o um mas algumas mexas ficaram esquisitas com a animação", "e o do bbs durante a animação de idle tbm tem uma movimentação brusca estranha dessa mecha".

- **Addresses**: none (assets only).
- **KH2, found** (Checked (render): the idle back view, 0.197.0 vs 0.197.6): the lining of Stage 392, rigid on `head`, shows over the locks that swing inward, as pieces with straight edges across the strands (the owner's arrows).
- **KH2, change** (`kh2_hair::LINING_FOLLOWS = true`): each lining corner follows its hair corner's bone, 8 % inside it in every frame; the corners at the roots are on `head` already (KH2's cap bones), so the scalp seam stays covered. Tried on the renders and not kept: rigid at 20 % (clean at rest too, but a far swing can still uncover it).
- **BBS, found** (Checked (files)): in the baked idle clips the `R_kamiB` bone (→ `r_hair4`, the right side above the ear, the owner's box) jumped 72-180 deg between frames (slot 1 frame 99; slots 31, 79); its kid angle z sits at ±180 deg and crosses it between the kid's frames 58 and 59, and `PamClip::local` interpolates the raw angle, so a time between whole frames (58.8) turned the lock the long way. The kid's own clip never steps more than 0.3 deg a frame.
- **BBS, change** (`BbsHairSource::sway_local`): the kid's angles at the two whole frames around the time, interpolated the short way.
- **Checked**: `bbs_hair_sways_as_the_kid` now also asserts every frame step of the five bones (idle < 1 deg, walk < 4, run < 12; the kid's run sweeps 37 deg in a few frames), the written turn equals the kid's (0.000 deg); `kh2_hair_does_not_cross_the_skin`, all hair tests; renders (`out/kh2/rig/`: idle side and back, run side and back) without the straight-edged pieces.
- **Not checked**: in game. The user must press Build.

## Stage 397 (fork branch Re-Re-Reverse/Rebirth): the KH2 hair at KH2's own speed in the idle clips; Dark Mode's hair looked at, not built, version 0.197.6 (2026-11-25)

Fork owner, 2026-11-25, verbatim, after Stage 396's report (Dark Mode and item 5 left, the advice to test first): "prossiga".

- **Addresses**: none (assets only).
- **Item 5, found** (Checked (files)): Riku's looped idle clips `p002_001` (slot 1, 100 frames), `p002_031` (31, 80), `p002_079` (79, 40) have no `PL0101.EET` record and their names are not in `PL0101.SES`; the walk and run clips (2, 28, 32, 80, 3, 21, 33) are in the `.SES` (footstep cues), so their length stays. KH2's idle loop (120 frames) was stretched onto them: x1.20, x1.50, x3.00 (Stage 380).
- **Change** (`kh2_hair_rig::{UNROLL [1, 31, 79], UNROLL_MAX 6, unroll_times, unroll}`, in `bake`): in the KH2-hair groups' `.ANN`, each of those clips is repeated whole until it and KH2's loop close together (slot 1 six times = 600 frames = 5 KH2 loops; 31 three times = 240 = 2; 79 three times = 120 = 1), every body channel sampled at its whole frames and written again; the hair then runs KH2's idle at KH2's speed. Assumption: nothing in Re:CoM counts Riku's idle loops by the clip's length (an idle fidget timed on it would come 3-6 times later).
- **Checked**: `unroll_times_closes_the_loops`; `kh2_hair_moves_as_kh2` (the repeated clips' non-hair channels equal Re:CoM's at every frame within 0.001, the others byte-equal; worst hair vertex 8 mm); all hair tests.
- **Dark Mode, found, not built** (Checked (files: `PL0201.MDL`)): Dark Riku's hair is on two textures (`n_zz120_01` 187 hair-bone triangles, `n_zz120_02` 91), his body part has a list B (`with_hair` handles only `PL0101`'s single list A), and the KH2 rig's bone append rejects his packets ("packet header not as Square writes it"). With the KH2 hair he would have 135 bones, which needs copies of his 128-bone event packs (4003 / 4043 / 4083) as Stage 395 did. Left for its own stage.
- **Not checked**: in game. The user must press Build.

## Stage 396 (fork branch Re-Re-Reverse/Rebirth): the KH2 hair in jump, fall, land, flinches, ledge grab and the first swing as KH2 has it; the ladder clips off the jump, version 0.197.5 (2026-11-25)

Fork owner, 2026-11-25: item 4 of the seven ("todos", Stage 391): KH2's own hair motion in attacks and the other clips that played its idle sway.

- **Addresses**: none (assets only).
- **Found** (asset-analyst, the workspace's `knowledge/KH2/kh2-riku-motion-roles.md`): Riku's clip roles from his action table (`0x1407C6370`, rows 0x00-0x46, Checked (exe)), Sora's and KH1 Riku's bakes (Checked (files, 0.029 deg)): 4 / 5 / 6 / 7 jump / apex / fall / land, 37 / 38 flinch front / behind, 44 / 46 / 45 / 47 the ground swings, 16 ledge grab; 10 / 12 / 13 are rows 9 / 0xB / 0xC, the ladder set (hands above the head), which 0.195.0 had mapped to jump / fall. KH2 Riku has no guard, recoil, cast, card or dodge motion; his only spin (A400) turns the other way from Riku's 47 / 50. That Re:CoM plays rows 4-7 for Riku's jump: Assumption (not logged).
- **Change** (`kh2_hair_rig::{CLIP_MAP (19 pairs), KH2_LAND 22, KH2_HANG 30, KH2_DAMAGE_FRONT 42, KH2_DAMAGE_BACK 50, KH2_SWING 608}`, `Kh2HairSource::motions` reads every slot the map names): added 4 → A003 jump, 5 / 6 → A004 fall, 7 → A005 land, 16 → A100 hang, 37 → A200, 38 → A202, 44 → A301 (low-medium in the note, the only swing kept); removed 10 → jump, 12 / 13 → fall (now KH2's idle sway). The other 30 clips keep the idle sway. The BBS sway (Stage 394) reads only the walk / run pairs: unchanged.
- **Checked**: `kh2_hair_moves_as_kh2` (worst hair vertex off KH2's exact motion: jump 14 mm, fall 10, flinch 11, swing 9; idle / walk / run unchanged 7-10), all hair tests; renders (`out/kh2/rig/c4_f8`, `c6_f8`, `c37_f3`, `c37_f10`, `c44_f6`, `c44_f12`): the hair streams back in the jump, rises in the fall, swings forward in the flinch, whips across in the swing.
- **Not checked**: in game; the blends into and out of these clips. The user must press Build.

## Stage 395 (fork branch Re-Re-Reverse/Rebirth): Riku's event motions play with the KH2 hair (109-bone copies of his event packs), version 0.197.4 (2026-11-25)

Fork owner, 2026-11-25: item 1 of the seven ("todos", Stage 391); to the fix the reverse-engineer recommended (a new hook at `0x14024D860` and new PLAYER groups): "ss".

- **Found** (reverse-engineer, the workspace's `knowledge/ReCoM/recom-skeleton-limits.md` § event packs and `recom-battle-entry.md` § Boss end; Checked (decomp, files), not seen in game): Riku's event motion packs are 102 bones (4002 the boss end's card get, and 4012, 4032, 4042, 4052, 4062, 4072, 4082 of other event states; Dark Riku's 4003 / 4043 / 4083 are 128). `0x14024E9F0` skips a clip whose bone count (`clip+0x10`, i16) differs from the controller's (`+0x40`, i16): with the KH2 hair (109 bones, Stage 379) the motion never ends and its clock stays at 0, so the boss end in Riku's story (`0x140187DC0` states 10 / 0xE / 0x13) never leaves the room. A pack is fetched by global id (`10000 + group`) in `0x14024D860(ctrl, id, slot, start, …)` (callers `0x1401A3580`, `0x140112470`, `0x14045E530`).
- **Assets** (`riku_skin::{EVENT_PACKS, EVENT_ALT 100, build_event_packs}`, `Kh2HairSource::bake_pack`, `kh2_hair_rig::bake(…, by_slot)`): when a KH2 hair group was built, each 102-bone pack of `EVENT_PACKS` gets a copy `PLAYER/41x2/PL41x2.ANN` (4102 … 4182): the 7 extra hair bones appended and the hair's channels baked as the KH2 hair does, with KH2's idle sway in every clip (`by_slot = false`: these slots are not `PL0101`'s). Manifest `kh1-riku-skin.json`: `"event_packs": [[4002, 4102], …]`. New names in `keyblade::new_resource_names` (placeholders for Panacea).
- **DLL** (`riku_skin::{event_packs, add_event_rows, event_alt_id}`, `win::riku_skin::{event_packs, kh2_hair_look}`, `win::models::{pack_pick, clip_bones}`, new hook `sites::MODEL_HOOKS` "motion pack pick (0x14024D860)", 15 bytes `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 30`): the PLAYER table copy gets one `.ANN` row per copy group (its own file only, so no Re:CoM file takes our id); the player-group wrapper (`0x140211AA0`) queues the copies in Riku's story when the look is the KH2 hair; `pack_pick` plays the copy's id when the requested pack is one of Riku's event packs, its clip's bone count differs from the controller's and the copy's clip matches. In every case Re:CoM plays itself nothing changes (the branch is taken only where Re:CoM would play nothing); Re:CoM's hair, BBS (102 bones) and all-Re:CoM settings never take it.
- **Checked**: `the_event_packs_have_kh2_hair_copies` (all eight, 109 bones in every clip, slots / frames / loop / flags as Re:CoM's, every non-hair bone's channels byte-equal), the manifest and row tests in `riku_skin`, `new_names_are_not_in_recom_hed` with this machine's `Recom.hed`, all tests, the Windows build.
- **Not checked**: in game: that the copies load (queued at each room / battle build like the Keyblade groups), that the boss end now plays Riku's card get and leaves the room, the log lines `riku skin: event pack 14002 played from its copy 14102 (109 bones)`. Assumption: the sound / effect lookups of `0x1401A3580` keep Re:CoM's id (they are not changed). Dark Mode with the KH2 hair (135 bones) is still not built (Stage 393). The user must press Build.

## Stage 394 (fork branch Re-Re-Reverse/Rebirth): the BBS hair sways as the kid's does in BBS, version 0.197.3 (2026-11-25)

Fork owner, 2026-11-25: item 6 of the seven ("todos", Stage 391): some sway on the BBS hair, rigid on `head` since 0.194.1 (Re:CoM's own hair-bone motion split the kid's layers, seen in game on 0.194.0).

- **Addresses**: none (assets only).
- **Read first**: the workspace's `knowledge/BBS/bbs-kid-riku-hair-motion.md` (asset-analyst, 2026-11-25, Checked (files)): the kid's hair hangs on `B_kami`, `L_kamiA`, `L_kamiB`, `R_kamiA`, `R_kamiB` under `atama`, keyed in every clip (idle ±1-2 deg, walk ±4-7, run ±13-20); his clips are loose one-clip PAMs in `bbs_fourth`: idle `n02di00001e` (60 frames), walk `00002e` (30), run `00003e` (20), 30 fps; no jump clip found.
- **Change** (`bbs_hair::{SWAY_BONES, SWAY_CLIPS, SwayRig, BbsHairSource::{sway, sway_rig, sway_local}}`, `prepare_mdl`, `bake_ann`): each kid hair bone takes the nearest free Re:CoM hair bone whose parent is `head` (no children; no bone added, Riku keeps 102): `B_kami` → `b_hair2`, `L_kamiA` → `l_hair3`, `L_kamiB` → `l_hair4`, `R_kamiA` → `r_hair3`, `R_kamiB` → `r_hair4`; that bone's rest origin moves to the kid bone's pivot (carried by the fit), its rest rotation stays. The kid's vertices whose dominant bone is a hair bone follow it (the rest, and the lining, stay on `head`). In every clip of the look's `.ANN`, the five bones turn, about their pivot and in Re:CoM's head frame, as the kid's clip turns the kid bone against `atama`, from the kid's idle at frame 0 (the pose the fit was made on): Re:CoM's walk slots (`kh2_hair_rig::CLIP_MAP`'s walk, an Assumption there) take the kid's walk, the run slots his run, every other clip his idle; a looped clip runs a whole number of the kid's loops stretched to its length, a one-shot the kid's own time. Without the three PAMs the hair stays rigid as 0.194.1.
- **Checked**: `bbs_hair_sways_as_the_kid`: the written channels decode to the kid's turn within 0.001 deg (idle and run, every frame); 102 bones; `inv3_inverts`; all hair tests. Renders (the workspace's `local/assets/recom/riku-hair/out/bbs/pose/`: idle, walk 0 / 8 / 16, run 0 / 5 / 10 / 15, side, back, front): the back of the hair shifts between run frames, no split layers or holes seen, nothing into the face.
- **Not checked**: in game (whether the sway reads, whether a lock separates from its neighbours at the run's extremes: the kid's vertices are bound to one bone each here, BBS weights some of them to two). The user must press Build.

## Stage 393 (fork branch Re-Re-Reverse/Rebirth): the KH2 and BBS hairs on the Halloween Town attire, version 0.197.2 (2026-11-25)

Fork owner, 2026-11-25: item 7 of the seven ("todos", Stage 391): the hairs on Dark Mode and Halloween Town, which showed Re:CoM's hair.

- **Addresses**: none (assets, the ini's comment, README).
- **Found** (Checked (files)): `PL0111` (Halloween Town) has `PL0101`'s skeleton (102 bones, the same names in the same order, the head at the same rest point) and its atlas is `n_zz110_11`. Dark Mode's models `PL0201` / `PL0211` have 128 bones under other names and atlas `n_zz120_*`: the KH2 hair's 7 extra bones would give 135, which Dark Riku's event packs (128 bones: 4003, 4043, 4083) would not play (the workspace's `knowledge/ReCoM/recom-skeleton-limits.md`), so Dark Mode waits for item 1 (the event packs).
- **Change** (`riku_skin::{KH2_HAIR_GROUPS, BBS_HAIR_GROUPS}` now (new group, base group, with the Keyblade, atlas), `build_hair(base, …)`, `ddd_hair::{RIKU_0111_KH2, RIKU_0111_BBS}`, `step.rs`): groups 186 / 196 (KH2) and 187 / 197 (BBS) = group 111 with the hair (Soul Eater / Keyblade of Heart); manifest `riku: 111`, so the DLL picks them with `Look::halloween` (no DLL code change). The hair's texture keeps its one name for all groups (`rk_hair_kh2` / `rk_hair_bbs`, the same picture).
- **Checked**: renders (the workspace's `local/assets/recom/riku-hair/out/halloween_kh2/`, `halloween_bbs/`): both hairs sit on the Halloween face as on the default one (same counts: 1336 / 366 Riku triangles, 824 / 632 hair); `the_kh2_hair_groups_are_built_from_rikus_101`, `the_bbs_hair_groups_are_built_from_rikus_101` (all four groups each), `new_names_are_not_in_recom_hed` with this machine's `Recom.hed`.
- **Not checked**: in game (Halloween Town with each hair and Keyblade). The user must press Build.

## Stage 392 (fork branch Re-Re-Reverse/Rebirth): a lining under the KH2 hair, version 0.197.1 (2026-11-25)

Fork owner, 2026-11-25: item 3 of the seven ("todos", Stage 391): the BBS hair's lining, which closed its gap in game (0.194.3: "sumiu o gap"), under the KH2 hair, after the shifts (Stages 381-388) did not close its gaps ("pelo visto deslocar não é a estratégia").

- **Addresses**: none (assets only).
- **Change** (`kh2_hair::{LINING 0.08, LINING_Y -0.03, LINING_Z 0.06, in_lining_window, lining}`, `SrcTri::lining`, `HairSource::lining_in_plain`): the KH2 hair's triangles with their centroid at or above 3 cm under the head bone and behind 6 cm in front of it (the back and sides of the skull, not the long hair below, not the fringe) are written again on `head`, pulled 8 % toward the head's centre (`HEAD_CENTRE`), same texels, two-sided like the hair: 211 triangles. A copy with a corner behind Re:CoM's kept face, or within 3 mm of it (`skin_depth`, `SKIN_MARGIN`), is left out (the first try cut the face at the ears in 33 triangles: `kh2_hair_does_not_cross_the_skin`). The lining gets no flat copy in the plain proxy (part 2; the flat copies showed as flat polygons, 0.196.4); BBS's lining keeps its copy (as 0.194.2, seen in game).
- **Checked**: `kh2_hair_does_not_cross_the_skin`, `kh2_hair_sits_on_the_face`, `swaps_rikus_hair_for_kh2s` (613 + 211), all hair tests; renders before / after (`out/kh2/rig/`: idle side, run back): hair under the locks behind the ear and over the nape in the run.
- **Not checked**: in game (whether the gaps the owner saw are closed; the lining is rigid with the head, so a lock swinging away shows it). The user must press Build.

## Stage 391 (fork branch Re-Re-Reverse/Rebirth): the KH2 and BBS hairs on their own texture at full size, Riku's atlas and its name kept, version 0.197.0 (2026-11-25)

Fork owner, 2026-11-25, verbatim: "como melhorar ainda mais o cabelo do BBS e do KH2?"; to the seven proposals (1 the boss end with 109 bones, 2 the KH2 texture at full size, 3 a lining under the KH2 hair, 4 KH2's attack motions, 5 KH2's speed, 6 BBS sway, 7 Dark Mode / Halloween Town): "todos". This stage is item 2.

- **Addresses**: none (assets only).
- **Found** (Checked (files: `swaps_rikus_hair_for_kh2s`' report)): the KH2 hair's islands went into Re:CoM's 1024 x 1024 atlas at 0.4 of their size (BBS 0.92). And (Checked (files: `PL0182.TXA`'s strings)) Riku's `.TXA` animates `EYE_BLINK`, `EYE_SHUT`, `LIP_*`… on `n_zz110_01.tm2` by name, while the swap renamed that atlas to `rk_hair_kh2` / `rk_hair_bbs`. Assumption (not seen in game): with the hair looks his eyes and lips did not animate.
- **Change** (`ddd_hair::with_hair`, `HairSource::packed_atlas` (default false), `dds_argb`, `HairSwap::{hair_tex, atlas_dds}`): the hair's triangles get a new texture of part 3 (index 3 on `PL0101`), `<new_texture>` = the source's own picture at its own size (KH2 `-5.dds` 1024 x 1024; BBS's filled atlas), written opaque A8R8G8B8 with its TIM2 entry added to the `.RTM` (`keyblade::texture_entry`, as the Keyblade of Heart's); the source's UVs as they are (KH2's in 0..1, Checked (files)). Riku's atlas `n_zz110_01` stays as it is, name included. The skirt's vertices take the hair's texture with its texel. The old packing stays behind `packed_atlas() = true` (no source uses it). Test helper `dt::test_view` (the two pictures side by side for the debug renders).
- **Checked**: renders (the workspace's `local/assets/recom/riku-hair/out/kh2/`, `bbs/`): the KH2 strands sharper than 0.196.16's at the same view, the fit unchanged; the BBS hair unchanged. All hair tests pass (`atlas_scale` 1.0 for both).
- **Not checked**: in game: that Re:CoM draws part 3 with a third texture (Square's own enemy models have up to 4 in a part: `EN0122` part 0, Checked (files)); that Riku blinks again with the hair looks. The user must press Build.

## Stage 390 (fork branch Re-Re-Reverse/Rebirth): KH2 hair: the locks behind and below the ear without sway (experiment), version 0.196.16 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.15 in game, with a screenshot boxing the locks above / behind the ear and the short locks below it: "quero fazer um experimento, deixe a animação dessas mechas mais retas"; "pq parece que a animação que está revelando o gap"; "esse é o cabelo do riku do kh2"; to the plan (the screenshot's side only, factor 0.0, render before / after): "sim".

- **Addresses**: none (assets only).
- **Found** (Checked (files: `P_EH000` bind-pose vertices per bone, head-relative)): on KH2's +x side the short locks below and behind the ear hang on bone 131 (x 6..9, y -6..1, z -3..2 KH2 units from the head), the lock above and behind it on 133 (x 11..12, y 2..8, z -8..0), the side of the back on 106 (x 10..14, y -12..1, z -25..-15). Checked (render): damping them changes only `body_side2` (the view of the owner's screenshot, face to the left), so the screenshot's side is KH2's +x. Assumption: these three bones are the boxed locks (bone positions, not a per-lock render).
- **Change** (`kh2_hair_rig::{DAMP [(131, 0.0), (133, 0.0), (106, 0.0)], rot_toward, damp_hair}`, called in `bake` after the tilt): each frame of every clip, a damped bone's head-relative rotation is turned the factor's share of the way from KH2's bind pose to KH2's motion, and its origin moved the same share (0.0: the lock moves with the head only; 1.0: KH2's motion). Goes against Stage 380's "the same as KH2" for these three bones only, as an experiment: set a factor to 1.0 to undo.
- **Checked**: `rot_toward_ends_and_middle`; `kh2_hair_moves_as_kh2` (damped bones left out of its 2 cm check against KH2's motion; the others still 7-12 mm); renders before / after (idle 25 / 75, walk 8, run 6, fall 8): the lock below the ear hangs straight down and covers more of the neck behind the ear.
- **Not checked**: in game. Whether the gap is gone, and whether the still locks look stiff next to the swaying ones. The user must press Build.

## Stage 389 (fork branch Re-Re-Reverse/Rebirth): KH2 hair: a skirt from the head's open edges to the hair, version 0.196.15 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.14 in game: "eu acho que é pq a cabeça base acaba antes da cabeça original e dai vem o cabelo e fica o vão"; to the plan: "sim".

- **Addresses**: none (assets only).
- **Found** (Checked (files: the 0.196.3 swap at rest, 32 open edges of the kept head, each against the nearest hair triangle)): the head's skin ends before the hair starts. Of the 24 open edges on the sides of the head 19 are more than 5 mm from any hair triangle and 14 more than 10 mm; at the +x ear top (92, 62, -27 mm from the head bone) 11 mm; the other open edges are the neck and chin (28 mm, outside the ear's height). No shift of the hair fixes it: the space moves with it (the owner's trials, Stages 381-388).
- **Change** (`HairSource::skirt_edge`, `kh2_hair::{SKIRT_X 0.05, SKIRT_Y [0.04, 0.17], SKIRT_Z [-0.08, 0.06], in_skirt_window}`, `ddd_hair::with_hair`, `HairReport::skirt`): each open head edge (an edge of one kept triangle only, both ends on the head bone) whose midpoint is in the window, |x| past 5 cm, 4-17 cm up, within 8 cm behind to 6 cm in front, is joined to the nearest hair vertices (at most 30 mm away) by one or two triangles, both facings, that wear the hair vertex's texel (UV) and follow its bone (so they sway with the hair). 14 triangles at rest. DDD and BBS hair are unchanged (the default selects none).
- **Not run**: the crate's tests (position trials without them, as asked); `kh2_hair_sits_on_the_face` was run alone: it passes (2 hair vertices more than 3 mm inside the face, the deepest 5.7 mm, with the shift at 0). Not checked in game. The user must press Build.
- **Risks**: the skirt may cover the top of the ear or look stretched where the nearest hair vertex is far; the neck and chin edges are outside the window.

## Stage 388 (fork branch Re-Re-Reverse/Rebirth): KH2 hair shift zeroed, version 0.196.14 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.13 in game: "pelo visto deslocar não é a estratégia,  os vãos ainda aparecem... pode zerar preciso de uma sugestão". The shifts tried (0.196.7-0.196.13: +6, 0, -24 ("ficou escrota"), +26, y -10, y -5, y -2 mm, in the owner's words relative to 0.196.6's z +12 mm) did not close the gaps.

- **Addresses**: none (assets only). **Change** (`kh2_hair::SHIFT`): [0, -0.002, 0] -> [0, 0, 0]. Position trial: installed with `--no-test`; not checked in game. The user must press Build.

## Stage 387 (fork branch Re-Re-Reverse/Rebirth): KH2 hair shift 3 mm up, version 0.196.13 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.12 in game: "sobe 3".

- **Addresses**: none (assets only). **Change** (`kh2_hair::SHIFT`): [0, -0.005, 0] -> [0, -0.002, 0]. Position trial: installed with `--no-test`; not checked in game. The user must press Build.

## Stage 386 (fork branch Re-Re-Reverse/Rebirth): KH2 hair shift 5 mm up, version 0.196.12 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.11 in game: "desloca 5 pra cima".

- **Addresses**: none (assets only). **Change** (`kh2_hair::SHIFT`): [0, -0.010, 0] -> [0, -0.005, 0]. Position trial: installed with `--no-test`; not checked in game. The user must press Build.

## Stage 385 (fork branch Re-Re-Reverse/Rebirth): KH2 hair shift: none to the front, 10 mm down, version 0.196.11 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.10 in game: "tira o deslocamento deixa em 0 e desloca para baixo uns 10"; before that: "faz a instalação sem testes nenhum até eu ver que ta bom" (position trials are installed with `tools/dev-install.sh --no-test` until he says the look is right).

- **Addresses**: none (assets only). **Change** (`kh2_hair::SHIFT`): [0, 0, 0.026] -> [0, -0.010, 0] (Re:CoM metres, +y up, +z front): no shift to the front, the hair 10 mm down.
- **Not run**: the crate's tests (position trial, as asked); not checked in game. The user must press Build.

## Stage 384 (fork branch Re-Re-Reverse/Rebirth): KH2 hair 50 mm forward again, version 0.196.10 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.9 in game: "teste ficou escrota move 50 pra frente".

- **Addresses**: none (assets only). **Change** (`kh2_hair::SHIFT`): z -0.024 -> 0.026 (50 mm forward; 14 mm in front of 0.196.6's 0.012, which the owner's "mais para trás" of 0.196.7 - 0.196.9 had moved back by 6, 12 and 36 mm). Nothing else.
- **Found by the walk**: 6 mm back (0.196.7) and 12 mm (0.196.8) were seen by the owner before this; his word on 0.196.9 (36 mm back) is the one above. What he saw at 0.196.7 and 0.196.8 was not reported in these words.
- **Checked / not checked**: see the test run below; not in game. The user must press Build.

## Stage 383 (fork branch Re-Re-Reverse/Rebirth): KH2 hair 24 mm further back, version 0.196.9 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.8 in game: "vai mais uns 24 pra tras".

- **Addresses**: none (assets only). **Change** (`kh2_hair::SHIFT`): z 0.0 -> -0.024 (24 mm more; 36 mm further back than 0.196.6's). Nothing else.
- **Checked**: the crate's 186 tests, among them `kh2_hair_sits_on_the_face` and `kh2_hair_does_not_cross_the_skin`. **Not checked**: in game: the forehead and the fringe (the hair is 36 mm back of where KH2's alignment puts it), the nape and the neck, the ear gap (about 36 mm behind Re:CoM's ear: Stage 381's `KH2_EAR` / `EAR_SHIFT` were measured at the old shift).
- The user must press Build.

## Stage 382 (fork branch Re-Re-Reverse/Rebirth): KH2 hair another 6 mm back, version 0.196.8 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.7 in game: "da pra ir mais para trás".

- **Addresses**: none (assets only). **Change** (`kh2_hair::SHIFT`): z 0.006 -> 0.0 (the hair 12 mm further back than 0.196.6's). Nothing else.
- **Checked**: the crate's tests, among them `kh2_hair_sits_on_the_face` and `kh2_hair_does_not_cross_the_skin`. **Not checked**: in game (does the gap close; the forehead and the nape); the ear gap lands about 12 mm behind Re:CoM's ear now (Stage 381: `KH2_EAR` / `EAR_SHIFT` were measured at the old shift).
- The user must press Build.

## Stage 381 (fork branch Re-Re-Reverse/Rebirth): KH2 hair 6 mm further back, the ear-apex triangle drop removed, version 0.196.7 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.6 in game: "eu acho que o encaixe do cabelo deveria ser um pouco mais pra trás pq ainda to vendo o gap e nada doq nos fizemos resolveu".

- **Addresses**: none (assets only).
- **Change** (`kh2_hair::SHIFT`): z 0.012 -> 0.006 (Re:CoM metres, +z is the front): the hair, its fit to the face and the rig's carry (`Carry::shift`) all move 6 mm back together. `drop_ear_apex_fragment` / `EAR_APEX_FRAGMENT` (0.196.4) is gone: it matched the triangle `hair#330` by its position, which this shift changes, and a render without that triangle shows the scenery through its place from low side cameras (a hole of its own); the point stayed in the owner's screenshots with it.
- **Checked**: the crate's 186 tests, among them `kh2_hair_sits_on_the_face` (at most 5 hair vertices more than 3 mm inside the face) and `kh2_hair_does_not_cross_the_skin` (0 crossing triangles).
- **Assumption, not shown**: that the gap above the ear closes or moves with the hair further back. The constants that put KH2's ear gap on Re:CoM's ear (`KH2_EAR`, `EAR_SHIFT`, Stage 375) were measured at z shift 0.012 and are unchanged: with the hair 6 mm back the gap lands about 6 mm behind Re:CoM's ear (they would be KH2_EAR z 0.009, EAR_SHIFT z -0.010 to keep it on the ear). The 0.196.4 plain-copy and proxy drops, the lift out of the skin and the 25 hair bones stay.
- **Not checked**: in game. The user must press Build.

## Stage 380 (fork branch Re-Re-Reverse/Rebirth): KH2 hair: no clip holds the hair still, version 0.196.6 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.5 in game (the game took the 109-bone Riku, nothing odd seen): "aparentemente aceitou nn achei nada estranho mas a animação DESSE CABELO precisa ser a MESMA do KH2"; before: "é só a do cabelo!" (only the hair's animation has to be KH2's, not the head's or the body's).

- **Addresses**: none (assets only).
- **Found** (Checked (files: `kh2_hair_moves_as_kh2`, a probe of every mapped clip)): 37 of Re:CoM's clips of 8 frames or more (attacks, guard, magic, cards, ...: slots 4-8, 11, 14-17, 22, 27, 34-41, 43-54, 56-58, 78, 81) had the hair held still at KH2's first idle frame (`CLIP_MAP` maps 14 slots). The mapped looped clips play KH2's loop stretched to close on Re:CoM's clip length, so the hair's speed against KH2's is: idle 1 x1.20, 31 x1.50, 79 x3.00; walk 2 / 28 x1.19, 32 x1.27, 80 x1.90; run 3 x0.82, 21 / 33 x0.90; fall 12 / 13 x1.25 (jump 10 / 26 one-shots: x1.00).
- **Change** (`kh2_hair_rig::bake`): a clip with no KH2 counterpart plays KH2's idle sway (stretched to the clip's length when it loops, at KH2's own speed when it is a one-shot) instead of the first frame.
- **Checked**: `kh2_hair_moves_as_kh2` asserts that no clip of 8 frames or more holds the hair still (37 without the change, 0 with it), and the 0.196.5 checks (the baked clips within 2 cm of KH2's motion for all 25 bones, every part and clip at the model's bone count); the crate's 187 tests.
- **Not done, the speed**: the hair repeats with its clip, so its speed follows the clip's length. Exact speed would need either clips unrolled to a common multiple of Re:CoM's length and KH2's loop (idle 100 vs 120 frames: 600; footstep and effect timing keyed to the clip is not shown to survive: Assumption) or the DLL driving the hair bones from KH2's motion at KH2's time, outside the clips. Not started: the owner's choice.
- **Not checked**: in game. The user must press Build.

## Stage 379 (fork branch Re-Re-Reverse/Rebirth): KH2 hair: all 25 KH2 hair bones move their own vertices, KH2's walk and run as KH2 has them, version 0.196.5 (2026-11-24)

Fork owner, 2026-11-24, verbatim (words left out are marked [..]), after a day of single spots (the ear-top point, the lock through the ear, the lock below it "por conta da animação está errada"): "mudança de plano cansei de andar em círculos usa a mesma animação do cabelo do KH2 e [..]"; "e encaixa certo na cabeça do riku do kh recom"; to the plan (7 bones appended, every clip 109 channels): "pode fazer aí".

- **Addresses**: none (assets only). Read first: `knowledge/ReCoM/recom-skeleton-limits.md` (Checked (decomp): a model may take more bones when every clip played on it has exactly the model's count; per-bone buffers are allocated at the skeleton's count; the DLL already clears and re-allocates the transform node on a model with another count: `win::riku_skin::clear_node`, Dark Riku's 128 vs Riku's 102).
- **Found** (Checked (files: `kh2_hair_moves_as_kh2`)): KH2's hair hangs on 25 bones, Re:CoM Riku has 18 hair bones; 0.195.0-0.196.4 handed the vertices of the 7 others to the nearest kept bone, so the worst hair vertex was 3.8 cm off KH2's exact motion in idle, 3.5 walk, 4.3 run, 7.9 jump (that number compared the model with itself: the baked clip's own error was not measured). Triangles with corners on a merged bone and a kept bone stretched with the sway (the lock under the ear, the fringe roots).
- **Change** (`kh2_hair_rig::{EXTRA [118, 121, 122, 124, 133, 144, 148], extra_name, extend_skeleton, append_bones, extend_ann}`, `Rig::new` (25 pairs), `Kh2HairSource::{rig, prepare_mdl, bake_ann}`): the 7 KH2 hair bones that [`SELECTED`] leaves out become Re:CoM bones 102..108 (`k2_hair_<kh2 bone>`, parent `head`, rest = KH2's bind pose carried, bounds as a bone without vertices) in all 4 parts of the group's `.MDL`; every clip of its `.ANN` has 109 bones; `hosts` keeps every KH2 bone on itself, so every vertex follows its own KH2 bone's motion. `EXTRA_TILT` (the extra turn of the back hair in walk and run, 0.195.3-0.195.4: "traseira do cabelo ainda ta levantando muito quando anda") is now 0: KH2's walk and run as KH2 has them (the old value is in the git history; the constant still takes a list).
- **Checked**: `kh2_hair_moves_as_kh2` now measures the BAKED clip posed on the model against KH2's exact motion for all 25 bones and asserts under 2 cm: idle 8 mm, walk 8, run 10, jump 12, clip 12 10, clip 33 9 (with the old tilt the walk and run were 9 cm off, by design); every part of the model and every clip have the same bone count (109); `RecomSkeleton`, `Ann` round trips, `selection_is_the_greedy_one`, the hair-group build test (`the_kh2_hair_groups_are_built_from_rikus_101`: groups 182 and 192) pass; all 187 tests of the crate.
- **Not checked**: in game: that the game takes a Riku with 109 bones (the node is allocated at the skeleton's count: Checked (decomp) in the note, not run), the walk / run / jump look, effects and the weapon on the extra-bone model (their bone indices are unchanged: bones are appended). The fit on Re:CoM's head is the same as 0.196.4 (face fit, lift out of the skin, the ear-top triangle and plain copies dropped); the ear-top point and the lock below the ear are to be looked at again with the exact motion.
- `tools/test-affected.sh`, the game-data hair tests, the Windows build. The user must press Build.

## Stage 378 (fork branch Re-Re-Reverse/Rebirth): KH2 hair hung out of the ear and the cheek; two tries at the point reverted, version 0.196.4 (2026-11-24)

Fork owner, 2026-11-24, verbatim (words left out are marked [..]), on 0.196.3 in game, with screenshots: "talvez estejamos com esses problemas pq a cabeça base do Riku de KH2 seja muito diferente da cabeça do KH1"; "uma mecha ta passando por dentro da orelha"; "e tem essa [..] aqui entrando na bochecha dele como eu já te mostrei"; "essa [..] mecha da franja que no meio da animação parece que torce, ela normal está assim"; then on 0.196.4's first build: "o ponto vazio ainda está lá! e vc tirou da pele do rosto base, não precisava tirar, só precisa cobrir o ponto que aparece o fundo!".

- **Addresses**: none (assets only).
- **Found** (Checked (files: 0.196.3's triangles against the kept face, rest and idle frames 0, 25, 50, 75; renders under `local/work/holefind/out/`, `local/assets/recom/riku-hair/out/ear_pen/`)): `FaceFit` moves a vertex along its direction from the head centre and never asks whether it lies behind a local flap; `b_hair2` (2649 / 2650) is rooted 15-17 mm inside the head under the +x ear with its tip 55-78 mm out, so it cuts the ear; `l_hair4` runs 1-3 mm into the cheek; `kh2_hair_sits_on_the_face` skips directions with `d[2] < -0.2`, where the ear is.
- **Change** (`kh2_hair::{SKIN_REACH 30 mm, SKIN_MARGIN 3 mm, SKIN_INSIDE 2 mm, SKIN_TIP 5 mm, SKIN_PASSES 4, skin_depth, lift_out_of_skin}`, called at the end of `Kh2HairSource::hair`): a hair triangle with a corner more than 5 mm outside (or no skin under it) and a corner more than 2 mm behind a kept face triangle has its inside corners lifted along that triangle's outward normal to 3 mm in front of it; decided per vertex position (shared vertices stay shared); repeats up to 4 times. Triangles wholly inside the head stay. DDD and BBS hair unchanged.
- **Checked**: new `kh2_hair_does_not_cross_the_skin` (all directions, no `d[2]` skip): 0 crossing triangles and the deepest vertex 2.3 mm behind skin; with the lift off it is 16 and 17.5 mm (the test fails without the change). Renders (`rig_before_0196_4` / `rig`, idle 0 and 50, +x side): no new patch, the lock lies over the ear.
- **Tried and reverted, in this stage, with the owner's words**: (1) recolouring the hair-coloured texels of the head skin's hair-line strip (atlas x 440-495, y 95-250) to skin: the owner did not want Re:CoM's base skin texture touched ("não precisava tirar"), the strip is Re:CoM's disguise under its hair; (2) keeping 8 of Re:CoM's removed scalp triangles above the +x ear (head-relative x > 0.04, y 0.08-0.15, z -0.06..0.06) with their corners on a flat hair-coloured texel: the render shows them as flat grey polygons standing out over the KH2 hair, so they are not the cover. The point above the ear was then found with a diagnostic build (the owner's screenshot from the game's diagonal camera, the triangles around the ear painted in 16 colours, `local/work/diag_map.txt`): an unpainted flat grey-lilac polygon at the ear top, bounded by the painted temple triangle 1412, the ear triangle 1928 and the hair triangle 2708, none of which it is. No part-3 triangle is there (no render of part 3 showed a hole, with the game's cull rule applied: a one-sided triangle with flag 0 is drawn iff clockwise on the monitor, flag 0x20 iff counter-clockwise, a doubled one always: `knowledge/ReCoM/recom-model-culling.md`). Part 2, the plain low-poly proxy that is drawn without a facing cull, keeps 211 head-bone triangles after the swap and 9 of them lie within 25 mm of that corner, on or outside the ear's skin. **Change** (`HairSource::drop_proxy_triangle`, `kh2_hair::{PROXY_DROP_X 0.06, PROXY_DROP_Y [0.03, 0.17], PROXY_DROP_Z [-0.10, 0.10], in_proxy_drop_window}`, `HairReport::proxy_dropped`): the proxy's head-bone triangles whose centroid is more than 6 cm to the side, 3-17 cm up and within 10 cm of the head's middle in depth are dropped, both sides (10 at the 0.196.3 swap); the proxy hid under Re:CoM's bulky hair and shows through gaps under KH2's. Test: `kh2_hair::tests::proxy_drop_window_is_the_ears`, and `kh2_hair_sits_on_the_face` asserts 4..120 dropped. That alone did not remove the point (the owner's next screenshot, marked in red). A second diagnostic build (every part-3 triangle painted in 16 colours, part 2 emptied: `local/work/diag_map2.txt`) showed no flat polygon any more and, at the ear top, a 23-pixel fragment of the hair triangle `hair#330` (dump 2711: head-bound, 481 mm2, one corner 1.2 mm from the ear skin, the others 8 and 16 mm out), textured in ordinary hair colour. So the flat polygon was the PLAIN copy of that triangle: the swap writes one flat copy of every KH2 hair triangle into part 2 as well (`plain_segs.extend(ddd_segs..)`), drawn without a facing cull. **Change 2** (`HairSource::drop_plain_hair_copy`, `kh2_hair::in_proxy_drop_window`, `HairReport::plain_hair_dropped`): the plain copies of head-bound KH2 hair triangles whose centroid is in the ear windows are not written to part 2 (76 at the 0.196.3 swap); the textured triangles in part 3 stay. `kh2_hair_sits_on_the_face` asserts 1..120 left out. The owner's next screenshot (ringed in red, "ainda está ali, praticamente contornei com o circulo") still showed it, so the textured `hair#330` itself is the light polygon (lighter than the dark locks around it). **Change 3** (`kh2_hair::{EAR_APEX_FRAGMENT [0.1110, 0.0713, -0.0236], EAR_APEX_RADIUS 3 mm, drop_ear_apex_fragment}`, at the end of `Kh2HairSource::hair`): the one head-bound hair triangle with its centroid there is not written (part 3 and the plain copy both); the ear skin lies behind it. `kh2_hair_does_not_cross_the_skin` asserts 0 hair triangles at that apex (1 with the step off). Assumption, not shown in game: the skin shows there and nothing odd opens. The 0.196.3 two-sided copy of the +x temple's head triangles (Stage 377) stays, unproven.
- **Not checked**: in game; the lift on idle frames other than 0 and 50 (a rest-pose change, the bones sway); whether the lifted `b_hair2` lock looks right on top of the ear. **Not done**: the fringe lock that twists (analysis: it is `l_hair2`, its baked rotation steps up to 22 deg per frame in the run, not an Euler error; see the owner's choice on a limit).
- `tools/test-affected.sh`, the game-data hair tests, the Windows build. The user must press Build.

## Stage 377 (fork branch Re-Re-Reverse/Rebirth): KH2 hair: the +x temple's head triangles written twice, version 0.196.3 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.2 in game (screenshot, then a second one with the spot ringed): "tirei um print bem perto para vc ver que o gap ainda está presente no cabelo do riku KH2"; "o gap que eu estou falando é um pequeno ponto que fica transparente mostrando o fundo do cenário, o lance da pele com  a base aparecendo é outra parte do problema"; "é do lado esquerdo da cabeça"; "e ele está parado"; after the investigation: "pode aplicar a correção".

- **Addresses**: none (assets only).
- **The report** (Checked (files: the two screenshots; renders `local/work/holefind/out/`)): Riku's left side (+x in the model), idle: a small flat lilac polygon (pixel (113,104,128), no hair streaks) at the top edge of the ear's gap, between two locks. The orange area around it is Re:CoM's ear / temple / neck, a separate matter the owner did not ask to change.
- **Found** (Checked (files: a z-buffered render of the swapped MDL's triangles, idle frames 0, 25, 50, 75, magenta background)): no enclosed background pixel at the ear with both faces drawn. With one facing sign kept, a 19x38 px sliver opens at the point's place; behind it is exactly one triangle, a single-copy head-bone triangle of Re:CoM's kept face (`head,head,head`, facing flag 0, centroid x +0.09 from the head bone), while every KH2 hair triangle is written twice (both flags). The atlas has alpha 255 everywhere (no cut texel).
- **Change** (`HairSource::two_sided_face` (default false), `kh2_hair::{EAR_TWO_SIDED_X, in_ear_two_sided_window}`, `ddd_hair::with_hair`, `HairReport::face_two_sided`): the kept head-bone-only triangles whose centroid is in the +x ear window (head-relative x > 0.06, y and z the ear's `EAR_FLAT_Y` / `EAR_FLAT_Z`) are written a second time, same vertices, other facing flag (as the hair's `back` copies). 19 triangles at rest. DDD and BBS hair are not touched (the default is false).
- **Checked**: `kh2_hair::tests::ear_two_sided_window_is_the_plus_x_ear`; `kh2_hair_sits_on_the_face` now also asserts the copies equal the face's triangles in the window (19) and 1..60; the hair-inside-the-face count stays 0. The dump of the new MDL has the 19 added triangles with the opposite flag (`local/work/holefind/out_fix/`).
- **Not shown** (Assumption): a render that simulates the game's facing cull by flag gives 0 enclosed pixels at the ear both before and after (`scripts/run_fix.py`), so the renders do not show this change fixing the point; the hole only appears when the cull follows the screen winding, which also drops the hair's back faces and is not how the game culls (`ddd_hair.rs` docs: Checked (game, 0.186.0)). Two readings stay open: the point is this triangle culled in the game (the change should remove it), or it is the triangle drawn as it is (Re:CoM's old hair covered it and KH2's open locks expose it; the change does nothing). Not checked in game.
- **Seen in game** (fork owner, 2026-11-24, verbatim, on 0.196.3, a screenshot with the spot ringed): "ainda está perceptível mas faça o commit e solta uma release do mod". So the point is still visible after the second copy: the cull reading (H2) did not remove it; the open reading is that it is the head triangle drawn as it is (H1: Re:CoM's old hair covered it, KH2's open locks expose it). Next, not done: cover or recolour that triangle's texels, or a systematic fit of KH2's head to Re:CoM's (the owner's remark: KH2 Riku's base head is very different from Re:CoM's).
- **To see in game** (idle, Riku's left side, close): did the small lilac point change? If it is unchanged, it is the visible triangle's own colour and a different fix is needed (cover it with a lock, or recolour its texels).
- `tools/test-affected.sh`, the game-data hair tests, the Windows build. The user must press Build.

## Stage 376 (fork branch Re-Re-Reverse/Rebirth): workflow tools; the hair's checks as tests, version 0.196.2 (2026-11-24)

Fork owner, 2026-11-24, verbatim: "tem alguma forma de otimizarmos o nosso workflow nessa workspace? ainda mais que estamos no fedora 45 até como oportunidade de reduzir consumo de tokens"; to the first three proposals: "sim"; then: "quando falei sobre otimizar nosso workflow na workspace quis dizer geral não esse específico do cabelo mas se as otmizações servirem para tudo pode prosseguir, se não pense melhor".

- **Addresses**: none (tools and tests only; the DLL's behaviour is 0.196.1's).
- **Tools** (`tools/dev-install.sh`: affected tests, Windows DLL, copy to `$KH1_RECOM_INSTALL`, version check against `VERSION`, three summary lines, logs in `target/dev-install/`; `tools/status.sh`: repo / installed / last-run versions and the asset build time). Machine paths from the owner's `~/.bashrc.d/kh.sh` (`KH_WORKSPACE`, `RECOM_GAME`, `KH1_RECOM_INSTALL`, the mingw linker); the assets crate's local tests read `KH_WORKSPACE` (`kh2_motion::tests::{workspace, raw, emu, emu_full}`) instead of fixed paths.
- **Hair checks as tests**: `kh2_loops_close` asserts a looped KH2 motion's seam under 0.05 KH2 units (0.00 now); `baked_euler_is_continuous` asserts clips' first frames within 180 deg (135.9 now); new `kh2_hair_sits_on_the_face` asserts at most 5 hair vertices more than 3 mm inside the face (0 now) and the -x ear flattened. Seen, not asserted: in the run (clips 3, 21) `l_hair1` / `l_hair2` step up to 45 deg in one frame (KH2's run at full speed or an Euler near its gimbal: not settled).
- Workspace: `scripts/compare-sheet.py` (a before / after sheet from two render folders); `CLAUDE.md`, the `release-kh1-recom` and `visual-verify` skills point to the tools.
- **Checked**: `tools/dev-install.sh` itself (tests, build, install, version). **Not checked**: in game (nothing to see).

## Stage 375 (fork branch Re-Re-Reverse/Rebirth): KH2 hair: the ears as KH2's model has them, version 0.196.1 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.196.0 in game (log `kh1-recom 0.196.0`): "não resolveu o problemna da foto que te mandei da orelha e da mexa...................... e te mandei a img de referencia ainda!" (the picture of 0.195.3: a lock into the cheek, the ear showing oddly behind a lock; the reference: KH2 Riku's PS2 model from the side, the ear in the gap between the front lock and the long hair).

- **Addresses**: none (assets only).
- **Found** (Checked (files + render `out/kh2/sbs/head_rest.png`: KH2's whole head carried by the hair's fit next to ours, both sides)): KH2's model has an ear on the +x side only (26 head-bone skin vertices past |x| 0.085, centre head-relative y 0.074 z 0.015); on -x its hair covers the side and its skin has no ear. Re:CoM has both ears (centre y 0.063 z -0.001): on +x KH2's ear gap sat 1.1 cm higher and 1.6 cm further forward than Re:CoM's ear, so a lock crossed the ear; on -x Re:CoM's ear came out where KH2 has none.
- **Change** (`kh2_hair::{ear_shift, EAR_*, KH2_EAR}`, `HairSource::adjust_face`, `ddd_hair::with_hair` applies it to the kept head-bone vertices and to the face the fit reads): +x: the hair around KH2's ear moves by (y -0.011, z -0.016) with a Gaussian of 3.5 cm, so the gap lands on Re:CoM's ear; -x: Re:CoM's ear vertices (|x| past 0.085 m, y 0..0.125, z -0.09..0.04 head-relative) are flattened to |x| 0.085, under the hair. The face fit (0.195.4) stays. Walk's extra turn back to -25 deg (0.195.4's -40 showed no difference at the ridge, which is KH2's walk itself: seen in 0.195.3's renders too).
- **Seen** (renders `rig/ear_v2.png`: the +x side rest / idle 0 / idle 50 / walk; `rig/walk_side2_0195_3_vs_now.png`): on +x the ear shows in the gap, the front lock before it, short locks below, as the reference.
- **Not checked**: in game; the -x side flattened ear's look (hidden in the renders); the cheek lock of the owner's picture (not reproduced in the renders).
- `tools/test-affected.sh`, the game-data hair tests, the Windows build. The user must press Build.

## Stage 374 (fork branch Re-Re-Reverse/Rebirth): Riku's Ansem tutorial done for him at Combat = KH1, version 0.196.0 (2026-11-24)

Fork owner, 2026-11-24, verbatim, relaying a tester: "someone reported Seems the Ansem fight doesn't work properly that stupid tutorial pops up and you can't complete it unless you switch to recom combat again. I'll let you know of any further bugs."; to how Sora's are handled: "a princípio lide da mesma forma então co o Riku"; to the plan (the battle room loads, its lesson is done for it): "de início sim".

- **Research** (searcher; the workspace's `knowledge/ReCoM/recom-riku-ansem-tutorial.md`, Checked (files + decomp)): Riku's first Ansem fight is `WORLD00` battle room 1100, asked by event room 2102 (`0xEE(1100)`), a Rapid Break and Duel lesson. Its script ends only through its own exit (session bit (1, 2), then syscall 0xF8 → Re:CoM's battle exit `0x140237160`, back to 2102 with `save + 0x187E` = 1100, which 2102 needs to go on); a KO does not end it (room 1100 is in the 1050..1198 event battle range). At Combat = KH1 with Riku's KH1 combat there are no cards, so the Rapid Break bit (1, 0x29) is never set and the lesson waits forever. No native call reloads 2102 from 2102 with the previous room 1100, so Sora's way (never loading the battle) does not fit.
- **Addresses** (callees, first bytes checked at start, `sites::STORY_SKIP_CALLEES`, Checked (exe)): system byte set `0x1401B15D0` (`83 F9 02 73 2A`); card give `0x14023AA60` (`40 53 48 83 EC 20 0F B7 D9 E8 92 00 F7 FF`, what syscall 0xAD sub 7 calls). Existing: battle-entry wrapper `0x140236690`, flag set `0x1401B1650`, variable get / set `0x1401B08C0` / `0x1401B16C0`.
- **Change** (`story_skip::riku_tutorial`, `RIKU_TUTORIAL_*`; `win::story_skip::{battle_entry, riku_frame}`; `win::attack::riku_request_bits` calls `riku_frame`): at Combat = KH1, Riku's story, `riku_kh1_movement` on, world 0, 2102 asking for 1100, the request is armed and passes to Re:CoM (the battle room loads). On Riku's frames in room 1100: once, the lesson's saved writes (0, 0x24) |= 0x10, |= 0x20 (read later by Re:CoM for the Rapid Break's bonus and to arm duels), session byte (1, 4) = 1, and the 9 world cards its ending gives (95, 96, 97, 102..107, through `0x14023AA60`); every frame (up to 900), the ending's clears (1, 0xA) / (1, 7) / (1, 0x28) = 0, the duel block (1, 0x2A) = 0 and the script's "done" bit (1, 2) = 1, so its main loop runs its own exit. Back in 2102, (1, 0x2A) = 0 once more and the skip disarms; 2102 then plays its own scene, card 109, Dark Mode bit and floor change. Each step is logged ("story skip: Riku's Ansem tutorial ...").
- **Assumptions**: `save + 0x187A` reads 1100 while in the battle room (the commit writes the room before loading; not traced for the current-room field); the script's start code runs before or within the first frames (the session writes repeat each frame); nothing pending refuses the exit's room change at its first pass; the card give's guards (set-1 bit 0x33, set-0 byte 0x61) are clear here.
- **Seen by the player** (expected, not seen): the arena for under a second, no HUD, then 2102's Ansem scene. Combat = Re:CoM or Riku's KH1 combat off: unchanged (Re:CoM's lesson).
- **Checked**: tests (`story_skip::tests::rikus_tutorial_only_at_kh1_with_his_kh1_combat`), `tools/test-affected.sh`, the Windows build, the callees' bytes in the exe. **Not checked**: in game.
- `docs/recom-story-skips.md` § Riku updated.

## Stage 373 (fork branch Re-Re-Reverse/Rebirth): KH2 hair fitted to Re:CoM's face; the walk's back hair lower, version 0.195.4 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.195.3 in game: "deu uma melhorada mas umas das mechas que marquei estão entrando na bochecha, e ainda aparece ta estranho a marcação que fiz a direita na img", "achei na internet um modelo 3d do riku de ps2 pra vc ter a referencia" (KH2 Riku's PS2 model, side: the ear shows, the front locks before it, the long hair behind); "essa mecha ficou meio esquitia numa parte da animação em idle" (a fringe lock by the eye); "traseira do cabelo ainda ta levantando muito quando anda".

- **Addresses**: none (assets only).
- **Face fit** (`kh2_hair::FaceFit`, `HairSource::set_face`, `ddd_hair::with_hair` passes the face that stays; replaces 0.195.3's `ear_push`): for directions every 3 deg from the head's centre (`ddd_hair::HEAD_CENTRE`), the outermost surface of Re:CoM's kept face and of KH2's head skin (its non-hair groups' head-bone triangles, carried by the hair's fit) by ray; a hair vertex moves along its direction by the difference, Gaussian-averaged over 10 deg, faded where only one face exists, held within -4..+25 mm. Checked (files, rest and idle frames 0 / 25 / 50 / 75 in the rest head's frame): hair vertices inside the face's front half 1-3, all within 2.6 mm (0.195.3: the same); Re:CoM's ear now shows where KH2's would (5 ear vertices up to 9 mm out of the hair at rest, as the PS2 reference shows its ear). Not reproduced in the renders: the strand into the cheek of the owner's picture.
- **Walk** (`EXTRA_TILT`: walk -40 deg, run -25 deg; `TILT_BEHIND`): only hair bones rooted 2 cm or more behind the head bone turn (a first try turned the front locks into the face). Renders `rig/walk_check.png` (top 0.195.3, bottom now: walk 0 / 16, run 6 / 17).
- **Not done**: the fringe lock by the eye in the idle (not found in the renders; likely one of the 7 KH2 bones riding another: 118 on 119, 122 / 124 on 125, 121 / 133 on the head).
- **Not checked**: in game. `tools/test-affected.sh`, the game-data hair tests, the Windows build. The user must press Build.

## Stage 372 (fork branch Re-Re-Reverse/Rebirth): KH2 hair: the side strands opened over Re:CoM's ear; a KH2 / Re:CoM side-by-side simulation, version 0.195.3 (2026-11-24)

Fork owner, 2026-11-24, verbatim: "não consegue rodar uma simulaçãod e um do lado do outro ver as diferenças e ir arrumando no recom? simulando igual tu faz com o neoshadow, mas andando e correndo e pulando?"; to the ear options ((a) shrink Re:CoM's ear into the hair, (b) open the side strands outwards over it, (c) leave the ear out and only move the strand off it): "b".

- **Addresses**: none (assets only).
- **Simulation** (debug test `kh2_and_recom_side_by_side`): KH2 Riku posed by OpenKH's emulator (the workspace's `local/tools/kh2anim` CLI now samples the whole motion in 60 Hz time; dumps in `local/assets/kh2/riku/emu_full`) next to Re:CoM Riku with the baked hair, the same KH2 time per frame, head-centred camera turned to the face's heading, same scale; side and back; idle, walk, run, jump, fall. Frames and GIFs in the workspace's `local/assets/recom/riku-hair/out/kh2/sbs/`. Seen: the hair relative to the head is KH2's; what differs is the posture (KH2 Riku walks and runs bent far forward, the head facing down; Re:CoM's stays upright), so KH2's hair falls along his back and ours streams backwards. Matching KH2's world look would lift the hair further (Stage 370).
- **Ear** (`kh2_hair::ear_push`, `EAR_CENTRE` (0.045, -0.025) from `head` at rest, `EAR_PUSH` 0.016 m, `EAR_REACH` 0.04 m, only hair 0.03 m or more to the side): hair rest positions pushed outwards around the ear by a Gaussian of the distance in y, z. Checked (files: at rest Re:CoM's ear skin came out of the hair's surface by up to 7.3 mm (22 of 33 ear vertices) on one side and 6.6 mm (9 of 33) on the other; now none, the nearest skin 4.2 / 2.8 mm under the hair; renders `rig/ear_before_after.png` (top 0.195.2, bottom now: rest, idle 0, idle 50, the side that shows the ear) and `rig/front_after.png`). Assumption: enough in motion (the strands move about their bones).
- **Not checked**: in game. `tools/test-affected.sh`, the game-data hair tests, the Windows build. The user must press Build.

## Stage 371 (fork branch Re-Re-Reverse/Rebirth): KH2 hair: KH2's motions played whole and at their speed (the loop jump gone), version 0.195.2 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.195.1 in game (log `kh1-recom 0.195.1`; idle screenshot, the ear marked): "resultado ainda ficou muito parecido com o mesmo problema"; "a animação mesmo e o gap + orelha sendo coberta pela mecha".

- **Addresses**: none (assets only).
- **Cause** (Checked (files): test `kh2_loops_close`): KH2's key times are in 60 Hz units while the motion's frame start / end / return are in its own frames (30 fps): the idle ends at frame 120, its hair keys run 0..240 (walk 38 / 76, run 18 / 36, fall 20-40 / 40-80). 0.195.0 / 0.195.1 sampled the keys in motion frames: the first half of every motion at half speed, then a jump back at each loop (the hair's seam 6.4 KH2 units in the idle against 0.58 per frame; walk 5.8 / 0.9, run 7.4 / 4.4). The emulator check had passed because the CLI samples the same 0..frame_end range of 60 Hz time. Now `Kh2Motion::world` / `locals` take motion frames and multiply by `keys_per_frame` (60 / fps); `world_key` keeps the 60 Hz time (the emulator test). Seams now 0.00 on idle, walk, run, fall; worst vertex off KH2's exact motion (25 bones) 3.8 cm idle, 4.3 cm run, 7.9 cm jump. The selection (`SELECTED`) is unchanged; the dropped 124 now follows 125, 148 follows 146.
- **The ear** (Checked (render: `local/assets/recom/riku-hair/out/kh2/rig/ear_rest_vs_idle.png`, the head's other side, rest / idle)): Re:CoM's ear shows between KH2's strands already at rest, so it is the fit (Re:CoM's ear out of KH2's hair, which covers it in KH2), not the motion. Not changed here.
- **Run**: `EXTRA_TILT` -25 deg kept (renders `run_0195_1_vs_2.png`: the run looks as in 0.195.1, now the whole motion).
- Debug: `render_body` view 3 (the other side); the test writes a rest render.
- **Not checked**: in game. `tools/test-affected.sh`, the game-data hair tests, the Windows build. The user must press Build.

## Stage 370 (fork branch Re-Re-Reverse/Rebirth): KH2 hair: no spin on clip blends, the ear strand on its own bone, the run lower, version 0.195.1 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.195.0 in game (screenshots: idle side, run, walk): "resultado muito interessante mas ficou esse gap ai e uma mecha ficou cobrindo a orelha", "a animação quando corre ficou quase perfeito da parte de trás do cabelo mas ele sobe Muito, e uma parte da animação essa mecha desloque de um jeito não natural"; then: "na verdade esse lance de uma mexida brusca é na parte de trás do cabelo inteiro, reparei agora".

- **Addresses**: none (assets only).
- **1. Spin on blends** (Checked (files): test `baked_euler_is_continuous`): 0.195.0 wrote the same hair-bone rotation with Euler angles up to 359 deg apart between clips (each clip's branch followed its own first frame); a blend between two clips crossed a whole turn. Now every clip starts on the branch of the bone's rest angles: the largest difference between clips' first frames is 101 deg (real pose differences), the largest step inside a clip 13.6 deg per frame.
- **2. The ear strand** (Checked (files): test `bone_amplitudes`): KH2 bone 142 (9 corners, by the ear) moves 9-13 KH2 units off the head where 139, which carried it, moves 3-5. `FORCED` keeps 142; the greedy choice then drops 133 (11 corners, 3-9 units, now on the head). `SELECTED`: 101, 104, 106, 108, 110, 112, 114, 116, 119, 125, 127, 129, 131, 135, 137, 139, 142, 146. Worst vertex off KH2's exact motion: 2.7 cm idle, 3.2 cm run, 6.8 cm jump.
- **3. The run lower** (`EXTRA_TILT`, `tilt_hair`): in clips mapped to KH2's run every hair bone but the skull cap (101, 104) turns 25 deg down about its own origin (Assumption: the amount, from the renders). Not KH2's: a posture correction toward KH2's world look was tried and dropped, KH2's head is bent 28-46 deg forward of its rest (emulator) against Re:CoM's 1-31 (clips; test `head_tilts`), so in KH2's own poses the back of the hair sits higher still.
- **Seen** (renders, the workspace's `local/assets/recom/riku-hair/out/kh2/rig/cmp_0195_0_vs_1.png`, top 0.195.0, bottom now: idle, walk, run x2 side, run back): idle and walk unchanged, the run's back hair lower and less spiky upward, less bunched on the back. Not seen: the ear gap (the renders do not show it at their size).
- **Not checked**: in game (the ear, the blend spin gone). `tools/test-affected.sh`, the game-data hair tests, the Windows build. The user must press Build.

## Stage 369 (fork branch Re-Re-Reverse/Rebirth): the KH2 hair moves with KH2's own motion on Re:CoM's 18 hair bones, version 0.195.0 (2026-11-24)

Fork owner, 2026-11-24, verbatim: "de que forma as animações do KH2 foram implementadas porque o resoltado ficou MUITO diferente até do KH2"; "investiga então"; "não gostei muito da ideia de gravar as animações inteiras SÓ para o cabelo tem outra alternativa melhor?"; to the options (A: re-pose Re:CoM's 18 hair bones on KH2's and write KH2's motion into them; B: a DLL hook; C: keep): "A"; to the plan: "ss".

- **Addresses**: none (assets only). Re:CoM's rule that a clip plays only when its bone count equals the skeleton's (the workspace's `knowledge/ReCoM/recom-skeleton-limits.md`, reverse-engineer) is why the skeleton keeps its 102 bones.
- **KH2 motion reader** (new `kh2_motion.rs`; OpenKH `docs/kh2/file/anb/motion.md`, `OpenKh.Kh2/Motion.cs`): `.mset` BAR → ANB BAR → motion entry (type 9), interpolated header at `0x90 + 0x10`, initial pose, forward F-curves (channel 4 bits, pre / post cycle 2 + 2 bits), keys (interpolation 2 bits, time index 14 bits), constant / linear / Hermite (tangents times the segment length), the model's bind pose composition. Checked (test `matches_openkh_emulator_on_the_hair`): the 25 hair bones relative to the head (98) against OpenKH's emulator (`OpenKh.Kh2AnimEmu`, the workspace's `local/tools/kh2anim`) on idle, walk, run, jump, fall, land: worst rotation element 0.0006, translation 0.002 KH2 units. Not read: IK helpers, constraints, expressions: the body's world pose differs from the emulator's (the head's world rotation by up to 30 deg in run), the hair relative to the head does not.
- **Rig** (new `kh2_hair_rig.rs`): 18 of KH2's 25 hair bones kept (`SELECTED`: 101, 104, 106, 108, 110, 112, 114, 116, 119, 125, 127, 129, 131, 133, 135, 137, 139, 146), chosen greedily by the distance their vertices move off KH2's motion when they follow another kept bone or the head instead (idle, walk, run, jump; test `selection_is_the_greedy_one`); the dropped 118 / 122 / 142 / 144 / 148 follow 119 / 125 / 139 / 139 / 127, 121 and 124 the head; 95 follows `neck3`. Each kept bone pairs with the nearest free Re:CoM hair bone at rest. The Re:CoM hair bones' rest locals are their KH2 bones' at KH2's bind pose, carried into Re:CoM's head frame (rotations conjugated by the two heads' rest rotations, origins through 0.188.0's fit: `SCALE` 0.0092, `WIDTH` 1.1, `SHIFT`), written into all 4 parts of the group's MDL (`HairSource::prepare_mdl`); the vertices keep 0.188.0's rest positions. Worst vertex off KH2's exact motion (all 25 bones): 3.6 cm idle, 6.4 cm run.
- **Clips** (`HairSource::bake_ann`, `kh2_hair_rig::bake`; the KH2-hair groups' own `PL0182.ANN` / `PL0192.ANN`): only the 18 hair bones' channels change (test `kh2_hair_moves_as_kh2`: every other channel byte-equal, frames / loop / flags equal). `CLIP_MAP` (Assumption, Re:CoM's clips have numbers only; picked by the feet): idle 1, 31, 79 → KH2 slot 0 (A000); walk 2, 28, 32, 80 → 4 (A001); run 3, 21, 33 → 8 (A002); jump 10, 26 → 14 (A003); fall 12, 13 → 18 (A004) (KH2 slots by OpenKH `MotionSet.GetMotionSetIndex`, battle with weapon). A looped clip runs a whole number of KH2 loops stretched to its own length; other clips take KH2's time held at its end; every unmapped clip holds KH2's idle at its first frame. No gain, no mean removal, no counter-rotation, no tilt (none of 0.191.0's).
- **Seen** (renders, the workspace's `local/assets/recom/riku-hair/out/kh2/rig/`: `rig_sheet.png`; `kh2_vs_ours.png`: top KH2's own model posed by the emulator, bottom ours, idle / walk / run, side): KH2's hair sticks out backwards in KH2 too (Riku's KH2 stance bends the head forward); on Re:CoM's straighter Riku the same head-relative hair streams back more visibly, in idle as well. In the run back view a few strand tips bunch over the upper back.
- **Not checked**: in game; the clip map (jump / fall slots); the blend into an unmapped clip (the hair jumps to the still pose over the clip blend).
- `tools/test-affected.sh`, the game-data hair tests, the Windows build. The user must press Build (the KH2-hair groups' MDL and ANN changed).

## Stage 368 (fork branch Re-Re-Reverse/Rebirth): the BBS hair one-sided, its atlas's black fill filled, version 0.194.3 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.194.2 in game (back screenshots, arrows and boxes): "sumiu o gap mas as pontas de algumas mechas ainda estão escuras"; "de outro angulo"; to the proposal (fill the atlas's black, draw the BBS hair one-sided): "sim".

- **Addresses**: none (assets only).
- **Causes** (Checked (files, renders)): (1) the kid's atlas has a black fill between the shards (value 0, with an anti-aliased rim of 16..80; the hair's texels are 80 and up), copied with the islands and scaled 0.92 into Re:CoM's atlas, so strand tips at an island's edge took in black; (2) the original `n02di00.pmo` drawn two-sided shows flat dark triangles at the back (the shell's inner layer, painted dark and facing in), drawn one-sided it is clean (the workspace's `local/assets/bbs/renders/kid_n02di_back_two_vs_one_sided.png`, top two-sided, bottom one-sided); `with_hair` wrote every triangle with both facing flags (0.186.1, for DDD).
- **Change**: `bbs_hair::bleed` grows the shards' colour into the fill (`BLEED` 8 passes over texels under `BLEED_BELOW` 72 in every channel, the mean of the non-fill 8 neighbours) before the atlas is packed; `ddd_hair::SrcHair::one_sided` (BBS on; KH2 off): each triangle is written once in the source's winding, all with the facing flag `facing` gives the majority of them (the source's outside). The lining of 0.194.2 stays (one-sided too), so the hollow back of Re:CoM's head stays covered. Packets `[124, 49]`.
- **Checked**: the rest renders with the facing cull (back, side, from below, front: no hole, no flat dark triangles at the back), the packed atlas (no black around the shards; a grey rim a little darker than the hair remains), `tools/test-affected.sh`, the Windows build.
- **Not checked**: in game (that the game's cull keeps the side the renders keep: the wrong side would show the hair hollow).
- The user must press Build.

## Stage 367 (fork branch Re-Re-Reverse/Rebirth): a lining under the back of the BBS hair, version 0.194.2 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.194.1 in game (log `kh1-recom 0.194.1`; back screenshots, the nape and the sides marked): "notei um micro gap aqui [...]"; then: "o micro gap que mostrei não mostra a face interna, mostra vazio msm/ branco.. o cabelo do DDD não teve esses problemas"; to the proposal (the kid's skull under the hair): "o gap é quase imperceptível, pode fazer".

- **Addresses**: none (assets only).
- **Cause** (Checked (files, renders)): with Re:CoM's hair removed, what is left of his head is a one-sided face mask (its back and top were hair): drawn from behind with the back faces culled, nothing is under the kid's thin shards, so a crack between them showed the background. BBS has the kid's skull there. The kid's own non-hair skull triangles were tried first: 12 triangles, a thin band at the ears, no cover (dropped).
- **Change** (`bbs_hair`): the hair triangles whose centroid is behind `atama` (240) are written a second time, every vertex pulled `LINING` 0.08 of the way toward the head's centre (`atama` + `LINING_CENTRE` (0, 0.08, -0.03), model units), same UVs and island (no new atlas space), rigid on `head`. 632 triangles in all (KH2's hair: 613), packets `[170, 49]`. A debug dump `tris.txt` (hair, lining, Re:CoM's head) in the test's output folder.
- **Checked** (renders, the workspace's `local/assets/recom/riku-hair/out/bbs/`: `lining_pose.png`, top 0.194.0, bottom now, idle and run, back and side; depth-buffered flat renders of the head and the lining from behind and above, in the session scratchpad): the lining covers the back and top behind the shards, nothing of it shows outside the hair; the look from outside is 0.194.1's. `tools/test-affected.sh`, the Windows build.
- **Not checked**: in game. Assumption: the lining hides the cracks in the engine (the faces' cull, the camera's angles).
- The user must press Build.

## Stage 366 (fork branch Re-Re-Reverse/Rebirth): the BBS hair sits on the head (rigid, lower, wider), version 0.194.1 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.194.0 in game (screenshots, front and back): "o cabelo do BBS no riku ficou muito pra cima e com um gap perceptível [...] e com algumas pontas esquisítas na parte de trás"; to the proposal (all on `head`, height 1.0, a small shift fitted on renders): "sim".

- **Addresses**: none (assets only).
- **Cause** (Checked (files), test `hair_bones_in_riku_clips`): 0.194.0 put each kid bone's corners on the nearest Re:CoM hair bone; those bones are keyed for Re:CoM's long hair (idle clip 1: `l_hair4` 18 deg, `r_hair4` 19 deg off rest against `head`; run clip 3: `b_hair2` 69 deg, `b_hair3` 79 deg), so on the kid's short closed shell the side tufts left the face (the gap at the temples) and the nape's under-layer spread sideways (the spikes at the back). The x1.1 scale about `head` put the crown 0.024 m over Re:CoM's own hair top (1.640 against 1.616).
- **Change** (`bbs_hair`): every hair corner follows `head` rigidly (no sway); `SCALE` per axis (1.15, 1.0, 1.1) (height 1.0; at 1.0 wide Re:CoM's temples showed beside the hair in the renders), `SHIFT` (0, -0.008, 0). Bounds x +-0.149, y 1.334..1.609, z -0.241..0.104; corner bones `head` 1176. Debug: `kh2_hair::tests::render_body` has a front view (2).
- **Checked** (renders, the workspace's `local/assets/recom/riku-hair/out/bbs/`: `cmp_0194_0_vs_now.png` rest front / side / back, top 0.194.0, bottom now; `pose_cmp.png` idle and run, rows 0.194.0 idle, 0.194.0 run, now idle, now run; 0.194.0's renders kept in `out/bbs_0194_0/`): the fringe reaches the brows, the temples are covered, the crown is lower; in idle and run the back stays one compact shell (0.194.0's run streamed the back strands out). `tools/test-affected.sh`, the Windows build.
- **Not checked**: in game. Assumption: the shift and widths read right in the engine's camera and lighting. The hair no longer moves on its own (BBS's own sway on `B_kami` etc. is not studied).
- The user must press Build (the asset output changed).

## Stage 365 (fork branch Re-Re-Reverse/Rebirth): BBS kid Riku's hair (`riku_hair = bbs`), version 0.194.0 (2026-11-24)

Fork owner, 2026-11-24, verbatim: "agora quero implementar o cabelo do kid Riku do BBS".

- **Addresses**: none (assets and settings only).
- **Source** (Checked (files), the workspace's `knowledge/BBS/bbs-kid-riku-hair-models.md`): `chara/npc/n02di/00/n02di00.pmo` in `bbs_fourth` (PMO v1, 35 bones, 1392 triangles), atlas = its texture 0 `n02di00_00`, the entry's HD asset `-0.dds` (1024 x 1024).
- **Change** (new `bbs_hair.rs`; `ddd_hair::RIKU_0101_BBS`; `riku_skin::BBS_HAIR_GROUPS`; `step`; `keyblade::new_resource_names`; `kh1-recom` `settings`, `menu`, `win::riku_skin`): a Rust PMO v1 mesh reader (the skeleton is [`pam::parse_skeleton`], as the Terra work reads it); the hair triangles are the note's colour rule on the head region (corners at or above `atama` minus 0.03) and then every UV island that is mostly hair taken whole; bind positions go to `head + 1.1 * (P - atama)`; each corner follows its dominant-weight kid bone, which maps to the nearest `head` / hair bone of Re:CoM by rest position (`atama` to `head`, `kubi` to `neck3`), as in the KH2 build; the swap is `ddd_hair::with_hair` (two-sided triangles, rebuilt atlas, new texture name `rk_hair_bbs`). Groups 183 (Soul Eater) and 193 (Keyblade of Heart), manifest `hair` 2. `riku_hair = recom | kh2 | bbs` (stored 0 / 1 / 2; the default stays `recom`; `ddd` still a warning); the camp menu's Hair row cycles Re:CoM / KH2 / BBS. Dark Mode and Halloween Town have no hair group: Re:CoM's hair shows, as for KH2. Missing BBS: the step logs a note and skips.
- **Checked**: 392 hair triangles (colour rule and island rule agree: the note's 392, no hole at the crown), 18 islands, 14 atlas rectangles at scale 0.92, 366 Re:CoM hair triangles removed, packets within the cap (`[135, 38]`), bounds x +-0.143, y 1.338..1.640, z -0.241..0.104 (the note's x1.1 numbers); corner bones `head` 894, `b_hair2` 96, `l_hair2` 71, `r_hair2` 46, `l_hair4` 35, `r_hair4` 28, `neck3` 6. Renders in the workspace's `local/assets/recom/riku-hair/out/bbs/` (`sheet.png`: front / side / back / top, unculled and culled; `source_sheet.png`: the triangles straight from the source atlas; `pose_sheet.png`: depth-buffered with the torso, idle, run, jump, attack, cast): the shape matches the note's x1.1 preview (fringe above the brows with spikes toward the eyes, side tufts over the ears, short back with spikes at the nape); no clipping into the back in any pose (the hair is short). `tools/test-affected.sh`, the Windows build.
- **Seen and accepted**: in the painter's renders a few dark grey triangles show on the back and top (the inside faces of the closed shell, whose texture is grey; with the depth buffer they are hidden); in the run the head's back shows the shell's inside a little when the head tilts (the hair triangles of Re:CoM's scalp are removed, as for KH2). The hair follows Re:CoM's own hair-bone clips as the KH2 0.188.0 hair does (it flutters in the run).
- **Not checked**: in game (the draw path, the fit on Re:CoM's face in the engine's lighting, the camp menu row, Halloween Town / Dark Mode fallbacks), `PL0111` (not previewed).
- The user must press Build.

## Stage 364 (fork branch Re-Re-Reverse/Rebirth): Riku's casts move his legs again (tamed), the DDD motion source removed, Riku's KH1 combat on by default, version 0.193.0 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.190.0 in game (screenshots of a cast, the legs rigid in his battle stance): "as animações da cintura para baixo o riku ficou travadasso, ficou bem estranho"; "pode remover o do DDD"; "as releases precisam sempre ir prontas para o pessoal usar com as restrições e com o combate do Riku já ativo por padrão um usuário me deu feedback que ele teve que mexer no INI". Earlier, on 0.189.0: "a de block está perfeita mas todas as de magia estão estranhas, o jeito que ele se move está esquisito".

- **Addresses**: none new, no new hook (assets and settings only). Terra's cast and Guard slots as in Stage 360.
- **1. Casts** (`riku_skin::tame_cast`, `TAME_LEGS`): the ground casts (`315`, `317`) are a middle between 0.189.0 (the body slid 0.8 m and hopped 0.25 m in a frame) and 0.190.0 (legs frozen). Terra's whole body is retargeted; the pelvis and legs (`hip`, `*_femur`, `*_tibia`) turn only 0.7 of the way from Riku's stance to Terra's (eased in and out over 4 source frames), the feet keep the stance's world rotation (flat on the floor; the ankle takes up the change) and the toes the stance's local, `waist` keeps its world rotation (torso, arms and blade are Terra's untouched), the body stays over its stance position (no x / z travel, so no hop) and its height follows the lower foot (lowered up to 0.15 m, raised up to 0.05 m, smoothed over three frames). The air casts (`316`, `318`) and the Guard (`354`) are unchanged from 0.190.0 / 0.189.0. Checked (files, renders of the real PL0101 model posed by the built clips, flat shaded and depth buffered: the workspace's `local/assets/recom/riku-terra/mesh/`, 0 / 0.7 / 1.0 compared side and front): 0 is the rigid stance, 1.0 crouches deep with a foot coming off the floor, 0.7 flexes the knees and shifts the weight with the arms with the feet flat; numbers on the built clips: body height 0.74..0.88 (Fire) and 0.82..0.93 (Thunder) m with at most 0.015 m a frame, no travel, lower foot 0.19..0.21 m (the stance's floor). Test `terra_motions_on_riku` asserts no travel, height limits, at most 0.03 m a frame, the lower foot within 0.04 m of the floor. Assumption: that it reads as natural in game; the leg share 0.7 and the ease are chosen from renders, not timing.
- **2. DDD removed** (`pam.rs` was `ddd.rs`; `step`, `riku_skin`): the DDD archive reader, its constants, the DDD guard and the DDD Block path, the DDD tests and the 2.8 look-up are gone; the cast and Block source is BBS FM's Terra only, then Re:CoM's / KH1's own motions (KH1 Riku's Guard, the 0.184.x spell clips). `pam.rs` keeps the PAM (v1, v2) and PMO skeleton readers the Terra path uses, `Src::Ddd` / `Skel::Ddd` are `Src::Terra` / `Skel::Terra`, `GROUND_CLIPS` = 315, 317, 354. `ddd_hair.rs` keeps its name (the hair code the KH2 hair uses); nothing reads the 2.8 collection. The workspace's DDD notes stay.
- **3. Release defaults** (`settings`): `riku_kh1_movement` defaults to `on` (code, the template and its comment); `ini_version` 3: a file older than 3 whose `riku_kh1_movement` is off gets it switched on once (`migrate`, the same mechanism as 0.143.1's `hud`), a later `off` stays; the camp menu's rewrite does not write that key, so it keeps the user's choice. Other defaults checked for a ready-to-play Riku: `combat = kh1`, `riku_free_magic = off` (spells cost MP), `riku_hair = recom`, `riku_costume = default`, `riku_keyblade = soul_eater`, `all_unlock = off`, the KH1 HUD: unchanged. README row `riku_kh1_movement` now `on`, a line on which games are used (Re:CoM and KH1 FM required; KH2 FM and BBS FM optional parts; no other collection); the release notes' requirements line says the same and that no ini edit is needed. Fresh install checked: no ini writes `DEFAULT_FILE` (`win::init`); a game folder with no `kh2_*` / `bbs_*` archives and no 2.8 folder (symlinked copy): the asset step completes (544 MB written), with only "note:" lines for KH2 (Riku's KH2 hair and the KH2 Keyblades) and BBS (Terra's motions: the old motions stay).
- **Checked**: `tools/test-affected.sh`, the Windows build. **Not checked**: in game; the casts' timing against the spell effect with the new leg motion; the guard (untouched).
- The user must press Build (the asset output changed).

## Stage 363 (fork branch Re-Re-Reverse/Rebirth): back to 0.188.0's KH2 hair, the DDD hair removed, version 0.192.0 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.191.0 in game (screenshots of the hair streaming behind his head): "ficou estranho pra cacete, a primeira versão é a melhor até agora do cabelo do KH2 kkkk ... ficou bizarronte o cabelo ai ó e pode remover o do DDD". Of 0.188.0 he had said: "ficou MUITO bom o cabelo do KH2, mas ainda precisa de alguns ajustes para ficar perfeito".

- **Addresses**: none (assets and settings only).
- **Reverted** (`kh2_hair`, `riku_skin::build_hair`; the hair stages 358, 359, 361b and 362 are marked, not deleted): the KH2 hair is 0.188.0's again: every KH2 bone takes the nearest Re:CoM `head` / hair bone by rest position, the cap bones (98, 101, 104) `head`; `SCALE` 0.0092, `WIDTH` 1.1, `SHIFT` (0, 0, 0.012). Gone: the rigid head binding, `b_hair1` / `b_hair2`, the band cuts, the 0.12 m back shift, the counter-rotation and tilt, `kh2_motion.rs` (KH2's motion reader) and the baking of KH2's clips into the KH2-hair groups' `PL0182.ANN` / `PL0192.ANN` (those groups use Re:CoM's `PL0101.ANN` as in 0.188.0). Kept: the debug tests of the renders (`posed_hair_renders`, with a depth-buffered torso render, `hair_bones_in_riku_clips`).
- **Removed, the DDD hair** (`ddd_hair`, `riku_skin`, `step`, `kh1-recom` settings and menu): `DddHairSource`, its classifier and bone map, `with_ddd_hair`, the groups 181 / 191 (`HAIR_GROUPS`) and the step's block, the tests that needed the 2.8 collection. `HairSource`, `with_hair` and the atlas code stay (KH2 uses them); `ddd::parse_pam`, `PamClip`, `parse_skeleton` and the rest of DDD's animation code stay (the Terra / cast work uses them). `riku_hair` is now `recom | kh2`: stored 0 / 1 (0.186.0 stored DDD as 1 and 0.188.0 KH2 as 2; the manifest's `hair` is 1 for the KH2 groups, so the user must press Build); an ini value `ddd` is a warning and keeps the default; the camp menu's Hair row cycles Re:CoM / KH2. The default stays `riku_hair = recom` (not changed, the owner will decide it separately).
- **Checked**: the restored KH2 hair renders (head and hair front / side / back / top) against the 0.188.0 renders of the workspace's `local/assets/recom/riku-hair/out/kh2/`: identical but for the painter's sort rounding of a few pixels (`restored_vs_0188_0.png`; top row 0.188.0, bottom now); the swap test (613 triangles, 23 islands, 366 Re:CoM hair triangles removed); `tools/test-affected.sh`; the Windows build.
- **Not checked**: in game. Old generated files of the DDD groups (`PL0181`, `PL0191`) may stay in the player's generated folder; nothing refers to them.
- The user must press Build (the manifest numbering changed).

## Stage 362 (fork branch Re-Re-Reverse/Rebirth): the KH2 hair moves like KH2's (its idle, run and jump baked into Re:CoM's hair bones), out of the back, version 0.191.0 (2026-11-24); reverted by Stage 363

Fork owner, 2026-11-24, verbatim: "precisa de mais movimento e a parte que está movimento atrás está entrando nas costas do Riku talvez se fosse um pouco mais suave ficaria melhor"; and: "só no geral o cabelo em si carece de movimentação, tem como usar uma movimentação parecida com a do KH2?" (his "Terra" mention in that message was a slip, it is about Riku's hair); then, to the question "Você quer que eu faça a proposta A?" (bake KH2's motion into Re:CoM's hair bones): "A".

Version note: `VERSION` was already 0.190.0 (Stage 361, Terra, same working tree), so this is the next minor.

- **Addresses**: none (assets only).
- **How KH2 moves this hair** (Checked (files)): the 27 hair bones of `P_EH000.mdlx` are keyed in the clips of `P_EH000.mset` (idle `A000` up to 51 deg in 120 frames at 30 fps, run `A002` 22 frames, jump `A003` 16 frames; the hair needs only the initial pose and the forward F-curves, no constraint or IK). New `kh2_motion` reads them in Rust (OpenKH's `motion.md` and `Motion.cs`); its test compares the hair chain's rotation against `OpenKh.Kh2AnimEmu`'s output on 5 clips: worst element difference 0.0005 (the emulator is the reference, run once from the workspace's `local/tools/kh2anim`; the test needs its output and is skipped without it).
- **Change** (`kh2_hair`, `ddd_hair::HairSource::bake_motion`, `riku_skin::build_hair`, `riku_skin::encode_clip`): (1) each KH2 hair bone is bound to the nearest Re:CoM `head` / hair bone by rest position (the cap bones and the whole vertex set above the jaw to `head`; the long back strands of the cap bone, more than 0.10 m under `head`'s origin, to the nearest hair bone); (2) `bake_motion` writes into the hair bones of the KH2-hair groups' own `PL0101.ANN` (`PL0182`, `PL0192`; the other looks keep Re:CoM's pack) the rotation that the KH2 bone nearest each Re:CoM bone has in KH2's idle (clips that loop from frame 1 with a leg swing under 10 deg and a root bob under 6 cm; `IDLE_GAIN` 1.5, the clip's mean taken out and its ends joined so it loops), run (loops, leg swing 50 deg or more: slot 3; `RUN_GAIN` 0.6) and jump (slots 14, 16, 17, 26: Assumption, no table names them; `JUMP_GAIN` 0.6); the angle is scaled by the gain; (3) in those clips the head's bend off rest against `chest1` is counter-rotated (`COUNTER_BEND` 1.0; not in the idle) and the lowest strands are tilted `TILT_DEG` 10 back, a tip may not swing more than `FORWARD_LIMIT` 0.03 m toward the front, and the lowest hair is moved `BACK` 0.12 m back in the bind pose; (4) every other clip keeps `OTHER_GAIN` 0.2 of Re:CoM's own hair-bone turn (Checked (files): Re:CoM turns them 40 to 120 deg in attacks and casts). The clips are re-encoded; no other bone changes (test `bakes_kh2s_hair_motion_into_the_clips`: only hair bones change in any clip, frames / loop / fps / bone count equal, the idle moves and loops).
- **Cause of the hair going into the back** (Checked (renders: depth-buffered, the torso posed by the same clips, in the workspace's `local/assets/recom/riku-hair/out/kh2/`): the strands bound to `head` (KH2's skull bone also carries the long back strands) turn with the head, which Re:CoM's run and attack clips pitch forward 50 to 70 deg against the chest, so a row of strand tips came out of the back with the layers behind them inside it. A row of strand tips on the back, in the run and in the attack, is gone with the nearest-hair-bone binding below the jaw and the counter-rotation (`cap_cmp.png`: `t10` before, `cap1` after). Neck-bone bands (`neck1`..`chest1`) were tried in 0.188.3 and dropped (the lower hair vanished into the torso).
- **Numbers** (tips = the lowest fifth of the strand vertices, largest distance from their rest position in the head's frame over the clip): idle 2 cm (0.188.3) to 6 cm, run 46 to 32 cm, jump 24 to 23 cm, attack 8 cm, cast 14 cm. A torso-penetration count from rays along the chest's forward axis (inside = odd crossings) was tried and dropped: it rose when the hair was moved away from the back (the torso mesh is open at the neck and shoulders), so the renders are the evidence.
- **Checked**: renders posed by clips 1 (idle, 4 frames), 3 (run, 4 frames), 4, 14, 17, 26, 37, 41, side and back, 0.188.3 against this (`before_after_idle_run_0190.png`, `before_after_other_0190.png`); `tools/test-affected.sh`; the Windows build.
- **Not checked**: in game (that the engine plays the group's own `PL0182.ANN` for the hair look, and how the baked rotations read; the jump slots are an Assumption; walking and guard are not mapped, they keep 0.2 of Re:CoM's turn; a few tips still pass through the back or the arm in the run and attacks by a few centimetres in the renders; the strands' tips are a little spiky from the side in the run).
- The user must press Build (the asset output changed).

## Stage 361 (fork branch Re-Re-Reverse/Rebirth): Riku's casts keep his legs and body, only Terra's upper body moves, version 0.190.0 (2026-11-24)

Fork owner, 2026-11-24, verbatim, on 0.189.0 in game: "a de block está perfeita mas todas as de magia estão estranhas, o jeito que ele se move está esquisito".

- **Addresses**: none (assets only).
- **Cause** (Checked (files), numbers from the built clips on Riku's skeleton, `local/assets/recom/riku-terra/`): the whole-body retarget moved the body. 315 (Fire, Blizzard): the body swung from 0.43 m back to 0.40 m forward in z with one 0.25 m step in a single frame (Terra's hop at his frames 13 to 14, which `plant_feet` turns into a body jump to keep the feet still), and its height changed 0.58..0.85 m (DDD's 315 on Riku: z -0.15..0.21, biggest step 0.095). 318: body 0.66..1.29 m, a foot at 0.03 m, a 0.13 m step in a frame. Terra's crouch and hop were made for his 18 % longer legs.
- **Change** (`riku_skin::CastCtx::clip_on`): the ground casts (`315`, `317`) map Terra's waist, chest, neck, head, arms and hands; `ex_1010_sk`, `hip`, the legs, belts, pants and cuffs are Riku's battle stance (as every unmapped bone already was), so no body travel, no crouch, feet where his stance has them. The air casts (`316`, `318`) keep Terra's leg tuck but the body stays at its stance position (`hold_body`). The Guard (accepted: "a de block está perfeita") is untouched: whole body, `plant_feet`, `put_on_floor`. `TERRA_KEYS` unchanged.
- **Checked**: stats and stick sheets of the variants (A = 0.189.0, B = A without the sideways travel, C = upper body only, F = C on the ground and the tuck in the air) next to DDD's built clips: B left the feet sliding on the floor with the same leans; C and F read as the cast with the arms and blade, the legs still. Test `terra_motions_on_riku` now also asserts the body never leaves its stance position in the four casts and the ground casts' lower foot is on the stance's floor. `tools/test-affected.sh`, the Windows build.
- **Assumption / not checked**: that this reads as natural in game (the legs are still during the ground casts, there is no crouch for 317); mesh renders (stick figures only); the air casts' legs; spell effect timing against the arm.
- The user must press Build (the asset output changed).

## Stage 360 (fork branch Re-Re-Reverse/Rebirth): Riku's casts and Block from BBS FM's Terra (no 2.8 collection needed), version 0.189.0 (2026-11-24)

Fork owner, 2026-11-24, verbatim: "o block do Riku poderia usar a animação do block do Terra.. sobre a magia poderia usar as animações do Terra também"; on identifying the clip first: "era bom vc identificar mesmo qual é antes de implementar... o block do BBS funciona igual o do kh1, vc aperta X ou quadrado e ela executa..., sobre as magias, mesma coisa as animações estão vinculadas as mesmas magias que tem no KH1 mas no BBS"; asked about implementing: "certo e as animações de magia e block do terra pro riku?". Ruling the same day (relayed): only the 1.5+2.5 collection may be required; DDD (2.8) is optional.

- **Addresses**: none new (assets only, no new hook). Terra's Guard start `FUN_14025e8f0` (BBS exe, Ghidra address) plays PAT_TERR slot 63 = clip `354`; casts: PAT_TERR slots 68 / 69 / 70 / 71 = `315` / `317` / `316` / `318`. Checked (decomp + files; `knowledge/BBS/bbs-terra-casts-guard-for-riku.md` sections 7 and 8).
- **Change** (`kh1-recom-assets`): new `terra` (BBS archives `bbs_first..fourth` read with the keys the step has; ARC reader; `read_terra` = `p03ex00.pmo` skeleton + `p03ex_151.arc` (315, 316), `p03ex_152.arc` (317, 318), `p03base.arc` > `p03ex_001.pam` (354)). `ddd::parse_pam` reads PAM version 1 (inline names) as well as 2. `riku_skin`: `CastCtx` / `GuardSource` carry the motion source (`terra` flag); Terra's clips use the existing DDD path (`Src::Ddd(name)` = a clip of whichever source is loaded): bones by name (`RIKU_FROM_DDD`; Terra's names match), the world-rotation delta retarget (Terra's collars are 90 deg off at rest, his `R_buki` 120), blade on Riku's stance grip, `carry_translation` of `center` / `koshi` scaled by `terra::TRAVEL_SCALE` 0.844 (hip height Riku / Terra), `plant_feet` on the ground clips (315, 317, the Guard) then `put_on_floor` (Terra's longer legs sank the lower foot 4 cm under the stance's floor), `from_start` on the air clips. Key moments for Terra's casts (`TERRA_KEYS`, read from the retargeted hands): 315 thrust 9..13, 316 8..13, 317 / 318 raise 10..14. Guard: `terra_guard`, `TERRA_GUARD_KEYS` (4, 24, 35): up over `RIKU_GUARD_UP`, pose through KH1's live window, lowering after (the clip itself lowers its arm from frame 24). The spell to clip mapping is the DDD rows' (Fire / Blizzard on the thrust, the rest on the raise).
- **Priority** (`step`): Terra's files (default, the 1.5+2.5 collection) > DDD Riku's (only when Terra's cannot be read and the 2.8 collection is next to the game) > Re:CoM / KH1 motions (the 0.158 KH1 Riku guard, the 0.184.x spell clips). No setting (the asset cache is stamped by the game's files, a setting would not invalidate it). Dark Riku gets the same through Riku (his clips are retargeted from Riku's, as before).
- **Checked** (files, owner's install): Terra's clips read (51 bones, 36 / 36 / 37 / 37 / 36 frames, 30 fps); built clips: `hip` at rest, the blade in the stance's grip on every frame, the ground clips' lower foot within 0.04 m of the stance's floor (0.000 after `put_on_floor`), air clips start at the body's rest height (`riku_skin::tests::terra_motions_on_riku`); synthetic PAM v1 / ARC tests. Stick renders of the built clips (Riku's skeleton, side and front) in the workspace's `local/assets/recom/riku-terra/`: guard (the weapon arm raised across the front, blade up and a little forward, held to about frame 24), Fire (both hands thrust forward at the start, then the blade raised), air Thunder. The blade is in the right hand in every clip (Riku's `wep0` hangs under `r_hand`; the stance grip is kept).
- **Assumption**: air casts use `316` / `318` (the slot order and the shape match; the game's code that picks them was not found). The guard pose after Terra's frame 35 (the game's motion-end rule; the clip lowers its arm). The spell effect timing against the thrust / raise (the key frames above; not seen in game). Travel scale 0.844 and the floor fix: measured on the skeleton only.
- **Not checked**: in game; the mesh (stick figures only); Terra's keyblade (not used, Riku's Soul Eater stays in his hand).
- Tests: `tools/test-affected.sh` (the only failure is the hair work's `kh2_hair::tests::swaps_rikus_hair_for_kh2s`, 1055 vs 613 triangles, not touched here), the Windows build. The user must press Build (the asset output changed).

## Stage 361b (fork branch Re-Re-Reverse/Rebirth): KH2 hair moves more and stays out of the back, shipped in 0.189.0 (hair change prepared as 0.188.3; superseded by Stage 362) (2026-11-24); reverted by Stage 363

Version note: another session had already bumped `VERSION` to 0.189.0 (Stage 360, Terra) in the same working tree, so this change adds no bump of its own; the DLL built from the tree carries both.

Fork owner, 2026-11-24, verbatim (on 0.188.2): "precisa de mais movimento e a parte que está movimento atrás está entrando nas costas do Riku talvez se fosse um pouco mais suave ficaria melhor"; and: "só no geral o cabelo em si carece de movimentação, tem como usar uma movimentação parecida com a do KH2?" (his "Terra" mention in that message was a slip, it is about Riku's hair).

- **Addresses**: none (assets only).
- **KH2's own movement** (Checked (files), OpenKH's `Kh2AnimEmu` run on `P_EH000.mset` + `.mdlx`, the workspace's `local/tools/kh2anim`): KH2 moves Riku's hair with clip-keyed hair bones (27 bones; angle of the hair bones against the head over a clip: idle `A000` up to 51 deg in 120 frames, walk `A002` 21..39 deg, run `A003` 38..55 deg). That the exe adds no runtime physics on top is not checked (the OpenKH docs list none for hair; the only "PHYSICS" triggers are movement ones).
- **Change** (`kh2_hair`): strands below the jaw line (`SWAY_BELOW` 0.08 m under `head`) follow `b_hair2` (was `b_hair1`) and are moved `BACK` = 0.10 m back (ramping in over 0.2 m); the rest follows `head`. Measured over all `PL0101.ANN` clips (tips = the lowest fifth of the strands, against `head`): tip travel in the run (slot 3) 15 cm with `b_hair1` and 43 cm with `b_hair2`, idle 0 and 1.5 cm; the other hair bones tear (median 23..54 deg, p90 53..96, max 70..127 against `b_hair2`'s median 34, p90 53).
- **Tried and dropped** (renders, z-buffered with the torso, `local/assets/recom/riku-hair/out/kh2/`): the strands cut at 4 cm bands on a neck-bone chain (`neck1`..`chest1`): the neck bones turn 26 deg together in the run, so the lower hair went into the torso and the cut edge was ragged (`body_before_v1.png`, `body_before_after.png`); `chest1` for the lower hair: no motion; `b_hair3`: ragged flare. A numeric torso-penetration metric from the bone clouds disagreed with the renders (it counted 0 cm where the render shows the hair swallowed), so the renders are the evidence.
- **Checked**: renders posed by the clips 1, 3, 4, 17, 37, 41 (side and back, with the torso and a depth buffer), before 0.188.2 / after: `b_cmp.png` (0.188.2, `b_hair1` + 10 cm, `b_hair2` + 10 cm, `b_hair3` + 10 cm) and `before_after_0188_3.png`: in the attack clip 37 the strand tips that poked through the back are gone; in the run the hair streams back. `tools/test-affected.sh`, the Windows build. The user must press Build.
- **Not checked**: in game; the seam at the jaw line (two bones, no slicing) when the head turns hard; the run flare looks spiky at the shoulders in the back view; the amount of motion is the clips', no more can come from them.
- **Proposal, not implemented (rule 9, needs the owner's approval)**: see the report of this stage: (A) bake KH2's keyed hair motion into Re:CoM's hair bone channels in `PL0101.ANN` at build time (asset-only); (B) a spring layer in the DLL (needs a hook in the pose path, none is known).

## Stage 359 (fork branch Re-Re-Reverse/Rebirth): KH2 hair sways again (lower strands on `b_hair1`), version 0.188.2 (2026-11-24); reverted by Stage 363

Fork owner, 2026-11-24, verbatim: "certo ficou melhor mas está sem movimento nenhum no cabelo ficou esquisito" (on 0.188.1, hair rigid on `head`).

- **Addresses**: none (assets only).
- **Change** (`kh2_hair`): vertices above the jaw line (more than `SWAY_BELOW` = 0.08 m below `head`'s origin is the cut) follow `head`; the strands below follow `b_hair1` (`SWAY_BONE`). Checked (files, all 80 `PL0101.ANN` clips, angle vs `head`): `b_hair1` is the mildest of the 18 hair bones (idle 0 deg, slot 3 18 deg, median 7, p90 18, max 36; the others median 23..54, p90 53..96, max 70..127), so the lower hair streams back in the run as one piece without tearing, and the bangs and crown stay on the head.
- **Checked**: posed renders (slots 1, 3, 4) head-only vs this, `local/assets/recom/riku-hair/out/kh2/sway_cmp.png`: strands keep their length and shape, no tearing, the lower hair lifts back in slot 3; `tools/test-affected.sh`, the Windows build. User must press Build.
- **Not checked**: in game. Assumption: the idle has no sway (`b_hair1` does not move in idle; the other bones that do tear apart); a faint seam at the jaw line where two bones meet; clipping into the back in attacks.

## Stage 358 (fork branch Re-Re-Reverse/Rebirth): KH2 hair stays at full length in motion, bangs keep their shape, temple covered, version 0.188.1 (2026-11-24); reverted by Stage 363

Fork owner, 2026-11-24, verbatim: "ficou MUITO bom o cabelo do KH2, mas ainda precisa de alguns ajustes para ficar perfeito"; on whether the hair keeps KH2's length in motion: "o comprimento é oq tu assumiu mesmo" (KH2's full length, also in motion); "mais um angulo para vc verificar" (running seen from behind: the hair looked short and compact); "a franja do cabelo ao correr tbm ficou meio esquisita" (running, side view: the hair streams back as a thin wedge, the bangs look thin and detached).

- **Addresses**: none (assets only).
- **Cause** (Checked (files), posed renders of `PL0101.ANN` on the swapped model, the workspace's `local/assets/recom/riku-hair/out/kh2/before_after_idle_run.png`): 0.188.0 bound every non-cap KH2 vertex to the nearest of Re:CoM's 18 hair bones, and the clips turn those bones far off the head (max angle vs rest relative to `head`: slot 3 up to 79 deg, slot 41 110, slot 47 118, slot 56 108; idle slot 1 up to 19). In slot 3 (Assumption: the run, it matches the screenshots) the groups sweep apart: the back strands flare into a wedge or fold, and the bangs (bound to `f_hair` / `l_hair*` / `r_hair*`, 36..42 deg) tear off the forehead.
- **Change** (`kh2_hair`): every strand follows `head` rigidly (no sway; the clips' sway cannot be kept without tearing, since one bone per vertex gives no blending); `WIDTH` 1.1 to 1.3 so the hair covers the temple and the ear skin. `ddd_hair::{classify_riku, RTri.tex}` are `pub(crate)` for the debug tests only. New debug tests `hair_bones_in_riku_clips`, `posed_hair_renders`, `kept_skin_triangles` (they run only with `RECOM_GAME`).
- **Checked**: posed renders (slots 1, 2, 3, 4, 17, 34, 41) before / after: the strands keep one shape in every frame, the bangs stay on the forehead; idle front / side: the right temple and ear skin gone; `tools/test-affected.sh`, the Windows build.
- **Not checked**: in game (the rigid hair clipping into the shoulders and back when the head tilts, no sway at all). Assumption: the stair-step flat tops on the back in the debug render are the painter's sort, not geometry (not verified with a depth buffer); the top is as round as KH2's sheet in the side view. The neck block under the chin in the head-only render is covered by the collar in game (screenshots 1, 2).
- The user must press Build (the asset output changed).

## Stage 357 (fork branch Re-Re-Reverse/Rebirth): Riku's hair from KH2 FM (`riku_hair = kh2`), the DDD hair optional, version 0.188.0 (2026-11-24); the DDD half removed by Stage 363

Fork owner, 2026-11-24, verbatim: "o cabelo começa sim pelo do Riku do KH2". Ruling the same day (relayed): only the 1.5+2.5 collection (Re:CoM, KH1 FM, KH2 FM, BBS FM) may be required; the DDD (2.8) hair stays but is optional.

- **Addresses**: none (assets and settings only).
- **Change** (`kh1-recom-assets`): `ddd_hair` is generalised with a hair source: `HairSource` (its `hair(names, world)` gives a `SrcHair`: triangles with position in Re:CoM's frame, UV in the source atlas, the Re:CoM bone name, UV island, strip continuation) and `with_hair` (the old `with_ddd_hair` is a wrapper; DDD's reading moved into `impl HairSource for DddHairSource`, same triangles, bones and atlas as 0.186.1). The atlas packer tries scales down to 0.3 (it stopped at 0.7) and averages a k x k grid per pixel when the scale is below 0.5 (k = 1 above, so DDD's atlas is built as before). New `kh2_hair::Kh2HairSource` reads KH2's `obj/P_EH000.mdlx` (kh2_sixth) with the existing `kh2-formats-lite` reader (`mdlx::parse`, `skeleton::bind_pose`) and its hair atlas, the asset `-5.dds` (DXT5, 1024 x 1024; the workspace note said BGRA, it is DXT5, `decode_dds` reads it). Group 3 = the 613 hair triangles. New groups 182 (Soul Eater) and 192 (Keyblade of Heart) in `riku_skin::KH2_HAIR_GROUPS`, built from group 101 by `build_hair` (now takes the source and the atlas target; the KH2 atlas is named `rk_hair_kh2`, so both hairs can be in the game); `keyblade::new_resource_names` lists them; `step` builds them after the DDD ones (KH2 missing: a note and skip, like the other steps; the DDD block already skips without the 2.8 collection) and writes `hair: 2` in `kh1-riku-skin.json`.
- **Fit** (Assumption, chosen from the renders): a KH2 vertex's bind world position `W` becomes `H + (1.1 x, 1 y, 1 z) * 0.0092 * (W - K) + (0, 0, 0.012)` m, `K` = KH2's head bone 98, `H` = Re:CoM's `head` (the note's 0.0086 left the fringe 1.3 cm inside Re:CoM's face; 0.0092 and +1.2 cm forward put it over the brow, the x factor covers the temples and ears). Bones: KH2's cap bones 98 / 101 / 104 (1153 of 1839 corners) follow `head`, rigid with the skull; every other KH2 bone takes the Re:CoM `head` / `*hair*` bone nearest to the KH2 bone's own rest position (b_hair3, l_hair1/2/4/7, r_hair1/2/3/4/7), so a chain keeps one bone. KH2's hair is shoulder length; the long strands end on the last hair bones, not cut. Atlas: the KH2 hair texture is one 867 x 917 island plus 22 smaller ones, so it is placed at scale 0.4 (about 410 px; the PS2 original is 256 x 256).
- **Change** (`kh1-recom`): `riku_hair = recom | kh2 | ddd` (stored 0 / 2 / 1, so 0.186.x inis keep their meaning); the camp menu's Hair row cycles Re:CoM / KH2 / DDD (`HAIR_SWAP` maps the menu order to the stored value); `win::riku_skin::chosen` and the manifest parser take hair 0..=2; a look whose hair has no group still shows Re:CoM's (so DDD without the 2.8 collection shows Re:CoM's). The ini template documents the key. The owner's ini is not touched.
- **Checked** (files, owner's install, `RECOM_GAME`): the swap on PL0101 with KH2's hair (613 triangles, 23 islands, scale 0.4, 366 Re:CoM hair triangles removed, packets within the cap); the head-and-hair renders front / side / back / top, culled and unculled, in the workspace's `local/assets/recom/riku-hair/out/kh2/` look like `local/assets/kh2/renders/P_EH000_sheet.png` (fringe over the brow, long back, lavender colour); the hair is 10 cm or more behind the body's back at every height, so it does not clip the torso at rest. Tests: `tools/test-affected.sh`, the Windows build.
- **Known imperfect** (render): a few skin chips of Re:CoM's face show at the viewer's right temple and ear through gaps of the fringe; the painter's render is not the game's depth buffer.
- **Not checked**: in game (the draw path for a 613-triangle two-sided hair at 0.4 atlas scale, how the hair moves with the head and the hair bones, clipping into the shoulders and the Keyblade pose in motion, the camp menu row). Assumption: the cap on `head` and the strands on hair bones do not tear apart under Re:CoM's hair-bone clips.

## Stage 356 (fork branch Re-Re-Reverse/Rebirth): Riku's spells cost MP again (`riku_free_magic` defaults to off), version 0.187.1 (2026-11-24)

Fork owner, 2026-11-24, verbatim: "pode colocar de volta tbm o custo de mana pra executar as magias, precisa comitar e dar push com as regras para td mundo poder jogar".

- **Change** (`settings`): `riku_free_magic` defaults to off (was on for the 0.184.0 test); the template line reads `off`. Riku's and Dark Riku's spells pay Sora's KH1 MP costs and the menu greys them for MP, as for Sora. The key stays as a test switch.
- The fork owner's own `kh1-recom.ini`: `riku_free_magic = off` (backup `kh1-recom.ini.bak-0.187.1`).
- **Checked**: tests, the Windows build. **Not checked**: in game.

## Stage 355 (fork branch Re-Re-Reverse/Rebirth): Riku's unlocked spells in his Customize picker, version 0.187.0 (2026-11-24)

Fork owner, 2026-11-24, verbatim: "ficou perfeito! o cabelo, precisa poder em customize também adicionar o comando das mangias quando ela são desbloqueadas". Answers to two questions: the seven of Sora's ("As 7 do Sora"); unlocked "Pela história do Riku" (see Assumption).

- **Change** (`win::command_menu::riku_spell_levels`, `win::keyblades::Page::open`): Riku's Customize page no longer passes `levels = [0; 7]` to `customize::picks`; it reads `riku_spell_levels()`, the same answer his command menu uses (Sora's seven at level 3 while the magic is in). The picker lists his unlocked spells in KH1's order, then his sleights, then None; a slot holds a spell as for Sora (`sc::Slot::Spell`).
- **Assumption**: no story rule for Riku's spells exists in the mod yet (0.184.0 unlocked all seven on the owner's ask), so every spell is unlocked. When a rule is written, only `riku_spell_levels` changes and both the menu and the picker follow.
- **Checked**: build, tests. **Not checked**: in game (the picker's rows, an L1 slot casting the spell).

## Stage 354 (fork branch Re-Re-Reverse/Rebirth): DDD hair drawn from both sides (no see-through bands from behind and above), version 0.186.1 (2026-11-24); removed by Stage 363

Coordinator's report of the 0.186.0 test in game, verbatim in meaning (relayed, not the owner's words): the hair "loads and draws in game", looks almost right from the front, but from behind and above there are "empty bands, like missing strips, across the back and top of the head", the scalp and nape skin shows through and "some zig-zag bands look see-through"; gaps at other angles too.

- **Measured** (Checked (files), a debug render in `ddd_hair`'s test, out in the workspace's `local/assets/recom/riku-hair/out/hair_{front,side,back,top,bottom_back}[_cull].png`): drawn two-sided (every triangle, no culling) the output hair is closed from behind and above (one small hole over the left side, see below). Drawn with culling by the facing flag (a triangle passes when its strip-order cross points to the viewer for flag 0x20, away for flag 0) it shows dark bands down both sides of the back and the crown, i.e. the reported symptom. DDD's hair is layered shells and thin shards: of its 343 triangles about a third have their winding front pointing into the head (corrected-order cross vs the outward direction: 59 + 51 of 343 inward), and the flag rule used in 0.186.0 (0x20 when the strip-order cross points away from the head centre; 96.6 % on Re:CoM's own face triangles) cannot give both sides of a shard.
- **Cause** (Checked (files) for the geometry, Assumption that the game culls by the flag): the one-sided copy of every DDD triangle. Not the 164 DDD skin triangles (all lie within 3.4 cm of a kept Re:CoM face triangle; the Re:CoM face island still covers the whole skull), not the 306 degenerate joins (strip joins, no area), not `segments` / `pack` (the output holds exactly the kept and the DDD triangles; the test counts them), not the UV islands (all 22 unanimous), not part 2 (the proxy).
- **Change** (`ddd_hair::with_ddd_hair`): each DDD hair triangle is written twice in part 3, the second copy with the other facing flag (`^ 0x20` on its drawing vertices, same vertices), so exactly one copy passes a facing cull from any side; with no cull the two copies are identical and draw the same pixels. Part 2 keeps one copy. 686 DDD triangles in part 3 instead of 343 (packets grow by about 8; the cap is unchanged). The test renders now also give back, top and bottom views, with and without the culling simulation.
- **Checked**: `cargo test -p kh1-recom-assets` (the swap's test now expects 2 x 343 DDD triangles, 366 removed, the kept ones byte for byte), the culled back and top renders close the bands. **Not checked**: in game (that the engine culls by this flag, and that the second copy shows the other side: both are the Stage 353 Assumption on the flag). Known left: a small hole seen only from straight above over the left side of the head (x about -0.13 m, z -0.14 .. -0.07 m): there Re:CoM's long `r_hair6` strand covered it and DDD's hair has nothing; not changed.

---

## Stage 353 (fork branch Re-Re-Reverse/Rebirth): Riku's hair from DDD, a Hair row on the Costumes page, version 0.186.0 (2026-11-24); the DDD hair removed by Stage 363

Fork owner, 2026-11-23 / 24, verbatim: "é possível implementar uma customização de personagem onde trocaria só o cabelo? e teria uma variação para o cabelo do DDD?"; "sobre o cabelo tente implementar e use um agente para escolher o melhor lugar para implementar no menu"; on the plan (scope: his default attire first): "sim".

- **Research** (the workspace's `knowledge/DDD/ddd-riku-hair.md`, Checked (files) unless marked): Re:CoM Riku's hair is not on its own packets (part 3: 99 greedy strip packets, hair and head triangles share them), so it is removed per triangle; `PL0101`'s atlas `n_zz110_01` (1024²) holds face, hair, skin and trousers. DDD's `p_ex020.pmo` (PMO v4; OpenKH's vertex code is wrong, the layout is in the note) has 343 hair triangles on its atlas `p_ex020_00`; its `atama` / `L_kamiB` sit where Re:CoM's `head` / `l_hair4` do.
- **Change** (asset step: `ddd::{parse_pmo, read_full}`, new `ddd_hair::{with_ddd_hair, DddHairSource, RIKU_0101}`, `riku_skin::{HAIR_GROUPS, build_hair, Made.hair}`, `step.rs`, `keyblade::new_resource_names`): groups 181 (Soul Eater) and 191 (Keyblade of Heart) are group 101's files with Re:CoM's hair triangles (366 of part 3; the island-based classifier) replaced by DDD's 343, each corner on its dominant DDD bone mapped to a Re:CoM hair bone (one bone per vertex: DDD's two-bone blends round to the heavier), on a rebuilt atlas `rk_hair_0101` (Re:CoM's hair islands cleared, DDD's packed in). Part 2 (the untextured twin: a low-poly proxy, 1074 triangles) gets the same swap. Manifest rows carry `"hair": 1`.
- **DLL** (`riku_skin::{Skin.hair, Look.hair (bits 4..5), group_for}`, `win::riku_skin::chosen`, settings `riku_hair = recom | ddd`, `menu::Item::Hair`): the Costumes page's second row "Hair" (Re:CoM / DDD), applied at the next room or battle like the costume (and live from the camp as the Keyblade change is, no transformation). A look with no hair group (Dark Mode, Halloween Town) shows Re:CoM's hair. The ui-ux agent's placement (menu.rs, 5-row limit not reached).
- **Checked**: tests (`the_hair_groups_are_built_from_rikus_101`, the swap's own six in `ddd_hair`, the DLL's manifest / group tests), the agent's renders (front and side: hair on the head, no gap at the scalp, hairline or ears; two or three small notches in the bangs), `tools/test-affected.sh`, the Windows build. **Not checked**: in game; whether Re:CoM's loader and draw path accept the rebuilt packets, how the hair moves with Re:CoM's hair bones (rigid), the 256-colour TIM2's quality (the PC shows the HD DDS), Halloween Town (`PL0111`) and Dark Riku's hair (not built). The asset step's code changed: the first start builds again (needs the 2.8 collection next to the game, else logged and skipped).

---

## Stage 352 (fork branch Re-Re-Reverse/Rebirth): Riku's own effect per spell (Blizzard, Thunder, Aero, Gravity, Stop), version 0.185.8 (2026-11-24)

Fork owner, 2026-11-23, verbatim: asked "os efeitos das magias teria variações melhores?"; offered (a) Riku's own effects that fit each spell or (b) KH1's cast effects: "o audio que toca sobre os efeitos é A" (reads as: the effects, option A).

- **Source** (the searcher's note, the workspace's `knowledge/ReCoM/recom-m13-recom-fx.md` § 8.4, Checked (files + exe): ids, bones, sounds, loops, what is loaded with Riku (`EF1500`, `EF1600`; not the sleight packs); how each program looks and the fit to the spell: Assumption, not seen in game).
- **Change** (asset step, `riku_skin::{spell_events, Fx, FxBones, cast_events}`; DLL `win::movement` request): Fire keeps Dark Firaga's effects and Cure the heal sparkle (the owner liked it). Blizzard: `ex_1580` 38 on Root (+ 39 on the chest, + 10 on the hand at level 3, 38 at 1.5). Thunder: 19 on the chest 10 frames before the release (+ 11 on the hand, + 42 on the chest as a loop). Aero: `speed_break` 0 (+ 5, + the blade's 8 as a loop and 11). Gravity: 36 on Root 30 frames before, 7 on Root (0.5 / 0.7 / 0.9) (+ 3 at level 3). Stop: 39 (+ 18 at 0.6, + 38 at 1.5). The darkness cloak and the blade's aura are gone from these spells. Thunder / Aero and Gravity / Stop share a clip: Aero and Stop play EET variants 4..6 (the DLL passes `level + 3` for requests `0x3C` / `0x89` / `0x3B` / `0x88`), Thunder and Gravity 1..3. Effect sounds come with each program's ESE row.
- **Checked**: tests (`rikus_cast_clips_carry_dark_firagas_events` now checks Thunder's `ex_1580` 19 at its frame 20, more events per level on variants 1..3 and 4..6, no Dark Firaga flash), `tools/test-affected.sh`, the Windows build. **Not checked**: in game. Assumption: 4..7 programs on one cast do not cost frames (Barrage already puts about 17 on one clip). The asset step's code changed: the first start builds again.

---

## Stage 351 (fork branch Re-Re-Reverse/Rebirth): the blade's roll in DDD's clips, one fixed voice per spell, version 0.185.7 (2026-11-23)

Fork owner, 2026-11-23, verbatim (on the DLL after 0.185.4): "nessas animações a lamina ainda está sendo segurada do lado errado!, e os audios ficaram muito aleatórios precisa fazer mais sentido com a magia"; on the effects: "o audio que toca sobre os efeitos é A" (effects: Riku's own, one per spell); also asked for a hair-only customisation (see the next stages).

- **Measured** (Checked (files): a probe on the owner's install, removed after): Riku's Soul Eater mesh (part 1, 508 vertices, all on `wep0`) points along `wep0`'s −x (far 5 % of vertices at (−0.74, −0.05, 0.0) m). In DDD's clips (`315` / `317` / `220`, built on Riku) that tip stays within 20..30° of DDD's blade tip (`R_buki` +x) and within 7..70° of the forearm, the range Re:CoM's own sleights have (Impulse 21..77°, Maelstrom 11..67°, Barrage 14..78°). So the tip's direction is not what is wrong; the side that faces out is (Assumption: the roll about the blade's axis; 0.185.3 and 0.185.4 both read "do lado errado").
- **Change** (asset step, `riku_skin::flip_blade_roll`, `CastCtx::clip_on`): in every DDD clip `wep0` (and the chain under it) is turned half a turn about its own x ([`BLADE_FLIP`]) on top of the stance's grip. If it is still wrong in game, the roll was not it: say which way the tip points.
- **Voices** (`riku_skin::CAST_VOICES`, `cast_events`): 0.185.5's random pick from four lines is replaced by one fixed line per spell, from the sleight that casts the same kind of thing: Fire 3035 (Dark Firaga's own), Blizzard 3036, Thunder / Aero 3033, Cure 3021 (Holy Burst's), Gravity / Stop 3034 (Checked (files) which sleight plays which number, § 8.3 of the workspace's `recom-m13-recom-fx.md`; what each line says: Assumption, not listened to). `ses::randomize` removed.
- **Checked**: tests (grip = the stance's turned by `BLADE_FLIP` on `315` / `317` / `220` / `316` / `318`; the voices fixed per spell in Riku's, Dark Riku's and the skin's SES), the Windows build. **Not checked**: in game. The asset step's code changed: the first start builds again.

---

## Stage 350 (fork branch Re-Re-Reverse/Rebirth): DDD's air casts back on the air rows, version 0.185.6 (2026-11-23)

Fork owner, 2026-11-23, verbatim: asked "da para implementar as animação de quando ele está no ar castando magia e defendendo ou se implementar vai substituir as animações de quando está no chão?"; told the air rows have their own clips (0.185.2 put the ground ones on them) and that KH1's Guard is ground-only: "1 sobre a guarda no ar não precisa implementar", "implementa os pontos anteriores pra eu testar".

- **Measured** (Checked (files): DDD's `center` + `koshi` off rest): `316` / `318` start with the body 0.18 m above DDD's rest and move −0.32..+0.20 m from there (`316` also 0.54 m forward and 0.35 m back, which `retime` drops: no travel on unplanted clips).
- **Change** (asset step, `kh1_recom_assets::riku_skin::{DDD_THRUST_A, DDD_RAISE_A, from_start, CastCtx::clip_on}`): the six air cast clips (`p002_233..238`, Dark Riku's `p003_`) play DDD's air thrust `316` (key 8..12) and raise `318` (as `317`'s keys) again, with Riku's grip (0.185.4) and the body's move from its rest height (`from_start`: frame 0's offset taken off `ex_1010_sk`). The ground clips are unchanged. No air Guard (KH1's Guard needs the ground: the workspace's `kh1-sora.md` § Guard).
- **Checked**: tests (`ddd_ground_motions_keep_rikus_feet_down`: `316` / `318` start at the rest height 0.974, the grip the stance's every frame, the five air sources on `316` / `318`; the ground checks as before), `tools/test-affected.sh`, the Windows build. **Not checked**: in game; Assumption: Re:CoM's air rows keep the body at its rest height. The asset step's code changed: the first start builds again.

---

## Stage 349 (fork branch Re-Re-Reverse/Rebirth): a sound on Riku's Guard, his cast shouts varied, version 0.185.5 (2026-11-23)

Fork owner, 2026-11-23, verbatim (on 0.185.3 in game): "falta audio quando o riku defende"; "os sons na magia ele está usando o mesmo grito, tem como dar uma variada".

- **Cause (Guard)** (Checked (files): the built `PL0101.SES` / `PL0201.SES`): Riku's and Dark Riku's Guard clips (`p002_225`, `p003_225`) had no SES entry, so they played no sound; Sora's (`p001_225`) has a whoosh and a grunt at frame 0 (`ses::add_kh1`).
- **Cause (casts)** (Checked (files)): every cast clip copies Dark Firaga's single voice 3035 (`cast_events`).
- **Change** (asset step, `kh1_recom_assets::ses::{guard_entry, randomize}`, `riku_skin::{CAST_VOICES, cast_events, build_with}`): the Guard gets a whoosh and a random grunt at frame 0 from each one's own swing sounds (Riku: `p002_044`'s 1333 and its grunts 3011 / 3014 / 3013; Dark Riku and the skin: `p003_056`'s 1332, `p003_053`'s 3005 / 3006). The cast voice is random (SES flag 4, Re:CoM's picker never plays the same line twice in a row) from 3035, 3036, 3021, 3027 at 25 % each: the shouts of Riku's and Dark Riku's own attack sleights (the workspace's `recom-m13-recom-fx.md` § 8.3). Which lines suit a cast: Assumption (not listened to). The guard recoil `p002_226` still has none.
- **Checked**: tests (`dark_rikus_own_pack_gets_the_guard` on the owner's install now checks the Guard's whoosh and grunt in Riku's, Dark Riku's and the skin's SES and the cast voices' table), `tools/test-affected.sh`, the Windows build. **Not checked**: in game. The asset step's code changed: the first start builds again.

---

## Stage 348 (fork branch Re-Re-Reverse/Rebirth): Riku's own grip on the Soul Eater in DDD's clips, version 0.185.4 (2026-11-23)

Fork owner, 2026-11-23, verbatim (on 0.185.3 in game, two screenshots: the battle stance, the blade down behind the hand; a DDD clip, the blade up in front of it): "ficou quase perfeita as animações do Riku guarda e magias, só que ele está invertendo a lamina", "posição correta, invertida".

- **Cause**: both 0.185.2 (`wep0` on `R_buki`'s world axes) and 0.185.3 (the rest-relative map from `R_buki`) took the blade's turn from DDD's weapon bone, and DDD holds its blade the other way round in the hand from Riku's battle stance (Stage 346: in the hand's frame (0.37, −0.93, 0.05) against Re:CoM's (−0.35, 0.93, −0.05)), so the blade turned over in his hand when a DDD clip began. Checked (in game, the owner, on 0.185.3).
- **Change** (asset step, `kh1_recom_assets::riku_skin::CastCtx::clip_on`): in DDD's clips `wep0` is no longer mapped to `R_buki`; it (and `k10`, `k11` and the blade bones, as before) takes the battle stance's local pose under its parent, so the grip is the stance's on every frame and the blade follows DDD's hand. Dark Riku's from Riku's, as before. `crate::ddd::RIKU_FROM_DDD` keeps the pair (the bone map's record).
- **Checked**: tests (on the owner's install: `ddd_ground_motions_keep_rikus_feet_down` now checks `wep0`'s rotation relative to `r_hand` is the stance's on every frame of `315` / `317` / `220`, off by 0.0000, the feet checks as before), the Windows build. **Not checked**: in game; whether the Guard `220` reads well with the blade held Riku's way (DDD's own pose holds it across the chest). The asset step's code changed: the first start builds again.

---

## Stage 347 (fork branch Re-Re-Reverse/Rebirth): the Soul Eater back on the rest-relative map, version 0.185.3 (2026-11-23)

Fork owner, 2026-11-23, verbatim (on 0.185.2 in game): "ficou perfeita a animação mas a espada está invertida e chega a atravessar o Riku".

- **Cause**: 0.185.2's `ddd::weapon_aligned` gave `wep0` `R_buki`'s world axes, turning the blade 180° from the rest-relative map of 0.185.0 / 0.185.1. Its reasoning (Riku's blade bones `k9..k1` along `wep0`'s +X against DDD's tip `Z_00` along `R_buki`'s +X) does not hold for the meshes: Checked (in game, the owner). Which of the two bone readings is wrong: Assumption (not checked). The Guard's outward blade in the earlier screenshot was 0.185.0's air clip `221`, not an axis error.
- **Change** (asset step, `kh1_recom_assets::riku_skin::CastCtx::clip_on`; `ddd::weapon_aligned` removed): the blade by the rest-relative map again. The animations (0.185.2's body on `ex_1010_sk`, planted feet, ground clips only, Guard `220`) are unchanged.
- **Checked**: tests (on the owner's install: `ddd_ground_motions_keep_rikus_feet_down` now checks the blade bones stay opposite to DDD's `R_buki` +X, cosine 1.000, with the feet and `hip` checks as before), the Windows build. **Not checked**: in game. The asset step's code changed: the first start builds again.

---

## Stage 346 (fork branch Re-Re-Reverse/Rebirth): DDD's body height where Re:CoM moves it, the Soul Eater along DDD's blade, ground clips only, version 0.185.2 (2026-11-23)

Fork owner, 2026-11-23, verbatim (in game, three screenshots; the DLL installed in OpenKH was still 0.185.0, as 0.185.1 had not been copied there: the Guard with the blade held out to Riku's right and a foot up; Fire and a raise cast crouched above his shadow): "ficou horrível, é para ser a animação de defesa!", "as anims de magia ainda muito desalinhadas"; "acho que na ultima sessão a ia entendeu de usar as animações do RIku do DDD que são executadas no ar, mas não deu muito certo, tem que usar as animações executadas no chão mesmo"; "para a defesa e as magias".

- **Cause (floating)** (Checked (files)): DDD moves the whole body by translating `koshi` and `hara` together (both under `center`); 0.185.0 left the body at its rest height (rotations only) and 0.185.1 carried the move onto Riku's `hip`, but no Re:CoM clip moves `hip` (in all of `PL0101.ANN` only bones 0 / 1 and the blade's `k11` move their translation; the body's height is on `ex_1010_sk`, the parent of `hip` and `waist`). 0.185.1's tests posed every bone and passed (Assumption: the game ignores `hip`'s translation; not seen in game, the screenshots are 0.185.0's).
- **Cause (Guard's blade)** (Checked (files)): DDD's blade runs along `R_buki`'s +X (`w_ri010.pmo`: the tip `Z_00` at x +0.78, the keychain at −X), Riku's along `wep0`'s +X (`k9..k1`); at rest they point opposite ways (`wep0` = diag(−1, 1, −1) · `R_buki`) and the rest-relative retarget kept that turn on every frame: the converted blade was DDD's negated in the hand's frame (Guard frame 18: (−0.04, 1.0) for DDD's (0.04, −1.0)), so the Guard's blade, which DDD holds across the chest, stuck out to Riku's right. The body and arms matched DDD to the millimetre.
- **Change** (asset step, `kh1_recom_assets::riku_skin::{carry_translation, plant_feet, retime, DDD_THRUST_A, DDD_RAISE_A}`, `ddd::{CARRIED_TRANSLATION, weapon_aligned}`): the body's translation (DDD's `center` + `koshi` off rest) goes on `ex_1010_sk`, and the feet planting moves `ex_1010_sk`; the planted clips keep its x / z sway when retimed (`retime(.., keep_sway)`; Root stays put); `wep0` takes `R_buki`'s world axes as they are (DDD's skeleton for the retarget with `R_buki`'s rest turned to `wep0`'s; wrong in game, reverted in 0.185.3); the air casts play the ground clips `315` / `317` too (no more `316` / `318`). The Guard stays `220` (on the ground; its blade now across the chest in front, as DDD's).
- **Checked**: tests (on the owner's install: `ddd_ground_motions_keep_rikus_feet_down`: the lower foot at most 0.21 m on `315` / `317` / `220` and 0.22 m on the cast clips, `hip`'s translation at rest on every built clip and the Guard, Riku's blade within cosine 0.99 of DDD's on every frame (1.000)), the Windows build. **Not checked**: in game. Assumption: the blade turns over in the hand during the blend into and out of a DDD clip (Re:CoM's stance holds it the other way round: in the hand's frame (−0.35, 0.93, −0.05) against DDD's (0.37, −0.93, 0.05)). The asset step's code changed: the first start builds again.

---

## Stage 345 (fork branch Re-Re-Reverse/Rebirth): Riku's feet on the floor in DDD's casts, his Guard on DDD's ground block, version 0.185.1 (2026-11-23)

Fork owner, 2026-11-23, verbatim (on 0.185.0 in game, with two screenshots): "a animação de defesa nada a ver, e precisa ajustar as animações de magia, pq os pé do riku ficam suspensos usando as animações e precisa ajustar".

- **Cause (feet)** (Checked (files): the clips posed on DDD's skeleton and on Riku's, frame by frame): DDD crouches by moving its hip bone `koshi` (0.88 → 0.65 m in `315` / `317`), and the rotation-only retarget kept Riku's `hip` at its rest height, so his feet rose up to 0.5 m whenever DDD crouched. And DDD's ground thrust `315` hops: both feet leave the floor on frames 1..7 (up to 0.45 m) as the hips go 0.6 m forward, then the hips drop the distance back in one frame at 14 (the forward travel is DDD's code's, not the clip's: Assumption). The workspace's DDD note § 6 ("ankles at 0.21 all clip long") measured without `koshi`'s translation; corrected there.
- **Cause (guard)** (Checked (files)): 0.185.0's Guard `221` is in the air the whole clip (hips 1.0..1.19 m, feet 0.4..0.8 m up; its last frame is the air thrust `316`'s first), and its rebound `224` is a back flip (feet up to 2 m), as is `223`; `222` is a 2.4 m dash. `220` (36 frames, no effect) is the only ground one: the hips still, the feet planted, the weapon hand up in front from frame 3 to about 24, the free hand back. Assumption: `220` is DDD's ground Block (its pose; `221`'s guard effect `P_AC_GUD_GUR1` reads as the air block).
- **Change** (asset step, `kh1_recom_assets::riku_skin::{carry_translation, plant_feet, ddd_guard, DDD_GUARD}`, `ddd::{CARRIED_TRANSLATION, GROUND_CLIPS}`): Riku's `hip` takes `koshi`'s translation off rest (the two rests coincide there); on DDD's ground clips (`315`, `317`, `220`) the hip is lowered as much as the lower foot rises above the first frame's floor, a foot standing on the floor does not slide (the hip takes its move), and the slide left at the end is spread back so the clip ends where it began: `315`'s hop becomes a step in and back. The air casts (`316`, `318`) are unchanged. The Guard is `220` on KH1's Guard timing (as 0.185.0 did with `221`); the recoil is KH1 Riku's again (0.158.0's), DDD having no ground one. Dark Riku's from them, as before.
- **Checked**: tests (on the owner's install: `ddd_ground_motions_keep_rikus_feet_down`: on `315` / `317` / `220` the lower foot never above 0.21 m (DDD's own reach 0.38 / 0.26 / 0.21), each foot ends as far from its start as DDD's; the six ground cast clips at most 0.22 m; `ddd_guard_on_riku`), the Windows build. **Not checked**: in game; whether a step in the thrust reads well. The asset step's code changed: the first start builds again. Never installed: 0.185.2 (Stage 346) replaces it.

---

## Stage 344 (fork branch Re-Re-Reverse/Rebirth): Riku's cast motions and Guard from KH Dream Drop Distance, version 0.185.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "tive uma ideia... as animações faltante do RIku vc vai pegar do KH DDD"; "a pega todas as magias b pode"; on the choice (DDD has two cast gestures): "a" (faithful to DDD, ground and air apart); "melhor ainda, a animação de guarda do Riku também está presente no DDD, implementa ela!".

- **DDD** (the workspace's `knowledge/DDD/ddd-riku-animations.md`, Checked (files) on the owner's 2.8 install by the asset-analyst and the main session): `Image/dt/kh3d_*.hed / .pkg` are the 1.5+2.5 archive format and cipher with the same key table; DDD Riku's main pack `chara/pc/p_ex02/anm/rt/bin/p_ex02_000.pam` (PAM v2: 178 clips at 30 fps, names in a string table) and his skeleton in `p_ex020.pmo` (PMO v4, 44 bones); a bone's local S · Rx · Ry · Rz + T, row vectors, radians (Re:CoM's clips' own convention); his rest skeleton is Re:CoM Riku's for the legs, hips, arms and head (same positions and frames). His casts by their effect lists: `315` / `316` (ground / air) the forward thrust (Fire, Blizzard), `317` / `318` the weapon hand raised overhead (Cure, Stop, Zero Gravity, the thunder codes); `221` his Block (`P_AC_GUD_GUR1`), `224` its rebound (`P_AC_COM_REB0`; Assumption: from the effect names).
- **Asset step** (`kh1_recom_assets::ddd`: the reader of the PAM / PMO formats and the bone map `RIKU_FROM_DDD`; `riku_skin::{Src::Ddd, Skel::Ddd, CastCtx::ddd, ddd_guard, CAST_SOURCES 9..18, CAST_CLIPS (12), CURE_CLIPS}`; `step.rs` reads DDD next to the game): twelve cast clips: Fire's wind-up and release, Blizzard (thrust), Thunder / Aero, Cure, Gravity / Stop (raise), on the ground (315 / 317) and in the air (316 / 318), on Riku by the bone map (his fingers, belts, the Soul Eater chain and the bones DDD lacks from his stance; the weapon by DDD's `R_buki`), on Dark Riku through Riku; KH1's timing as before (each key moment on KH1's release frame, its own speed, no travel); Riku's Guard (`p002_225`) is DDD's 221 on KH1's Guard length (up by 10, held to 37, lowered), its recoil (`p002_226`) DDD's 224 at its own speed; Dark Riku's from them. Without the 2.8 collection next to the game: the 0.184.x motions and KH1 Riku's guard, as before (logged).
- **DLL** (`crate::riku::{CAST_ROWS, CAST_CLIPS, DARK_CAST_CLIPS, cast_row}`): rows 0x3B..0x46 (both tables `p002_000` / `p003_000` up to 0x46, Checked (exe)); the air requests play the air rows.
- **Checked**: tests (on the owner's install: `ddd::tests::reads_ddd_rikus_pack_and_skeleton`: 178 clips, 44 bones, clip 000 rebuilds the PMO rest; `ddd_guard_on_riku`: 12° a frame at most; the turn and key-moment tests on the twelve; `riku_every_spell_level_and_air_casts_fully`), the Windows build. **Not checked**: in game. The asset step's code changed: the first start builds again.

---

## Stage 343 (fork branch Re-Re-Reverse/Rebirth): Riku's Cure on his own heal motion, Aero on KH1 Riku-Ansem's sweep, Blizzard after Impulse's leap, version 0.184.9 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "blizzaga ainda ficou estranho mas stopga ficou perfeito, engraçado pq em teoria o blizzaga ta usando a mesma anim de firaga"; "a animação do Sora de Curaga pro RIku ficou muito estranho tenta pensar fora da caixa"; "Aeroga tbm ta flicando um pouco"; "quando disse que a anim do sora pro riku ficou estranho quis dizer que não combinou mesmo".

- **Blizzard** (Checked (files)): its motion is Riku's Impulse (`p002_061`), not Dark Firaga; Impulse's Soul Eater bones are still under the hand the whole clip; its first 8 frames are a leap back (the travel is out, the legs' jump and landing were not). Now from frame 9, after the landing (`CastSource::start`).
- **Cure** (Checked (files)): Riku's own heal motion in Re:CoM, his item use `p002_034` (56 f at 30, on the ground: a crouch 8..16, the left hand raised over his head 18..22, held to 32), whose heal sparkle 0.184.8 already plays; it hides and turns the Soul Eater (scale 0): the blade is kept in his hand as at its first frame (`freeze_blade`, now Riku's `wep0` / `k10` / `k11` too; the worst turn a frame fell from 161° to 27°). Dark Riku: the same motion retargeted. Sora's Re:CoM Cure is out ("não combinou").
- **Aero** (Checked (files)): Maelstrom's spin rose into the air; now KH1 Riku-Ansem's request 223 (`xa_ex_1580.mset`, 73 frames at 60, on the ground: both hands sweep, the left from behind to the front 40..48), its blade held; Maelstrom without KH1's files. Its worst turn: 6° a frame.
- **Asset step**: `kh1_recom_assets::riku_skin::{CAST_SOURCES (9), CastSource::start, ITEM_USE, freeze_blade}`.
- **Checked**: tests (the turn test on the owner's install: every clip within its source's speed), the Windows build. **Not checked**: in game. The asset step's code changed: the first start builds again.

---

## Stage 342 (fork branch Re-Re-Reverse/Rebirth): no shaking blade on level 3; Riku's Cure with his own heal sparkle, version 0.184.8 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "stopga a espada do RIku ta vibrando toda, e o  efeito de curaga ficou muito estranho e o efeito de cura com aqueles sininhos ficaram muito comicos era bom achar outro efeito melhor"; "blizzaga tbm ficou um treme treme da espada muito estranho".

- **Cause (blade)**: both are level 3 on different motions (Stop: KH1 Riku-Ansem's, Blizzard: Riku's Impulse): 0.184.5's level-3 blade aura (`weapon_ora` 0 on `wep0`, the darkness Riku's sleights put on the Soul Eater) reads as a shaking blade (Assumption: from the owner's two reports, not seen).
- **Change** (asset step, `kh1_recom_assets::riku_skin::{cast_events, freeze_blade, CURE_CLIP}`): level 3 keeps the cloak and the 1.5 × flash, without the blade's aura. A KH1 motion converted onto Dark Riku's skeleton keeps its blade bones (`wep0`, `wep1`, `wep2` and under `wep0`) at their first frame under the hand (the conversion's weapon curves are not trusted). Cure's clip keeps only the voice (no dark glow, flash, cloak or aura on a heal).
- **Change** (`win::magic::{release_cure, start_cast}`): Riku's Cure plays his own heal sparkle, `always` 1 (the one his item use `p002_034` plays at frame 37; Checked (files)), on his bone 0 at Re:CoM's Cure offset, once per level (Cura twice, Curaga three times), instead of Re:CoM's `cure_all` (its bells) and its charge on the blade. Sora's Cure is unchanged.
- **Checked**: tests, the Windows build. **Not checked**: in game. The asset step's code changed: the first start builds again.

---

## Stage 341 (fork branch Re-Re-Reverse/Rebirth): Cure and Stop on new motions (Sora's Re:CoM Cure, KH1 Riku-Ansem's outstretched hand), version 0.184.7 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "curaga e stopga ainda ficaram muito estranhos" (earlier: "se precisar pode usar animações do KH1").

- **Cause** (Checked (files): the clips' hips and hands): Cure's Holy Burst start (`p002_075`) and Stop's Barrage crossing (`p002_070`) are sleight phases in the air: the hips rise from 0.7 to 1.4 m, so Riku floated and kicked while casting.
- **Search** (Checked (files), the owner's install): KH1 Riku-Ansem's `xa_ex_1580.mset` (on Dark Riku's skeleton, `ex_1580_sk`) and KH1 Riku's `xa_ex_1010`, every request converted and measured; Sora's Re:CoM cast clips `p001_061..067`. Ground motions with a cast's hand: Riku-Ansem's request 225 (49 frames at 60 fps: the hips still, the left hand reaching out at the shoulder 8..12 and held); Sora's Cure `p001_064` (34 f at 30: a crouch, both hands raised over the head 12..15, held to 24, lowered).
- **Change** (asset step, `kh1_recom_assets::riku_skin::{Src, Skel, CastSource { fps, fallback, .. }, CastCtx, CastClip::times_for}`): sources are a Re:CoM pack's clip on Riku's, Dark Riku's or Sora's skeleton, or a KH1 Riku-Ansem request converted onto Dark Riku's (the 0.158.0 converter); each is brought onto Riku's / Dark Riku's skeleton (Dark ↔ Riku as before; Sora's by bone name, the roots as each other, Riku's own bones from his stance) and played at its own rate (KH1's 60 fps). Cure: Sora's Re:CoM Cure for both; Stop: Riku-Ansem's request 225 for both, Barrage's crossing when KH1's `xa_ex_1580` can't be read. `GuardSource::ansem` carries the KH1 files (`step.rs` reads them).
- **Checked**: tests (on the owner's install: the turn test, Stop's at most 5° a frame; the key moments; Stop's fallback), the Windows build. **Not checked**: in game. Assumption: Sora's raise reads well on Riku (his Soul Eater by bone name). The asset step's code changed: the first start builds again.

---

## Stage 340 (fork branch Re-Re-Reverse/Rebirth): Riku's cast motions at their own speed (no flicker); his KH1 rows blend in and out (the Guard's end too), version 0.184.6 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "as animação ficaram estranhas meio que flicando , lembrando que se precisar pode usar animações do KH1 como disse"; "ajusta tbm a animação de guard pra ficar mais suave quando terminar essas".

- **Cause** (Checked (files): the clips' local rotations per frame on the owner's install): 0.184.4 squeezed each key moment into 4 frames and stretched the charge to fill KH1's release frame: the key moments ran up to 3 × their own speed (Maelstrom's spin, Impulse's thrust), the short Barrage charges at a ninth, then sped up. And every cast clip started and ended in a sleight's pose, entered and left with no blend (KH1's casts ask none): a jump from the stance.
- **Change 1** (asset step, `CastClip::times`, `SRC_STEP`, `out_frame`): the source clips (30 fps) play at their own speed (half a source frame a KH1 frame) through the charge and the key moment, the key moment still starting on KH1's release frame (a longer charge starts later in it, a shorter one holds its first pose); the recovery spreads to KH1's free frame but never faster than the clip (one that doesn't fit stops where KH1's motion ends). The Dark Firaga events follow (a charge event before the clip's start goes on its first frame).
- **Change 2** (`crate::riku::{blend_for, kh1_only_row, CAST_IN, OUT}`, `win::movement::RecomHost::play`): a cast's first motion blends in over 6 frames; leaving any of Riku's KH1-only rows (the cast clips, the Guard, its recoil) for another row blends out over 10 (the Guard's end into the stance included); the Guard into its recoil stays at once (KH1). Visual blends only: KH1's frame keeps its timing. Assumption: the numbers.
- **KH1's motions**: not needed yet (every clip now plays at its own speed); kept as the fallback the owner offered.
- **Checked**: tests (`cast_clips_turn_no_faster_than_their_sources` on the owner's install: no bone turns faster a frame than in its source; `cast_clips_put_the_key_moment_on_kh1s_release`; `rikus_kh1_rows_blend_in_and_out`), the Windows build. **Not checked**: in game. The asset step's code changed: the first start builds again.

---

## Stage 339 (fork branch Re-Re-Reverse/Rebirth): Riku's spell effects grow with the level, version 0.184.5 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "os spells precisam ter diferença dependendo do level dele e os efeitos também"; on the plan (level 1 the hand's glow and the flash, 2 also the darkness cloak, 3 also the blade's aura and a bigger flash): "aprovo, implemente".

- **Re:CoM** (Checked (decomp `0x14017A240`)): a motion's start keeps the EET events whose `+0x5C` is the variant its request passes.
- **Asset step** (`kh1_recom_assets::riku_skin::{cast_events, cloak_of}`): each cast clip's list A holds three variants: 1 the Dark Firaga glow and flash; 2 also the darkness cloak (the `ex_1580` 20, 21, 23..34 programs on the body, from Riku's own Impulse `PL1301` `p002_062` / Dark Riku's Dark Impulse `PL1311` `p003_072`, their own bones; Checked (files)); 3 also `weapon_ora` 0 on the blade and the flash at 1.5 × (its scale fields). All in Riku's `EF1500.EFF`, loaded with him (Checked (files)). Sounds unchanged (variant 0).
- **DLL** (`win::magic::{CAST_LEVEL, cast_level}`, `win::movement::RecomHost::play`): a cast row's request passes the cast's level as its EET variant.
- **Checked**: tests (`rikus_cast_clips_carry_dark_firagas_events`: more effects each level, the blade's aura on 3), the Windows build. **Not checked**: in game.

---

## Stage 338 (fork branch Re-Re-Reverse/Rebirth): one cast motion per spell for Riku and Dark Riku, version 0.184.4 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "além disso as animações precisam variar..."; on the table (Fire Dark Firaga, Blizzard Impulse, Thunder Dark Aura, Cure Holy Burst / Inverse Burst, Gravity / Stop Barrage, Aero Maelstrom): "aprovo, implemente"; "lembrando que vc pode usar alguma animação disponível no KH1 também... caso nenhuma der Match" (every spell found a Re:CoM match).

- **The motions** (Checked (files): frames, hands and hips of each clip; `kh1_recom_assets::riku_skin::CAST_SOURCES`): Dark Firaga `PL1088 p003_043` (thrust 32..36); Impulse `PL1301 p002_061` / Dark Impulse `PL1311 p003_071` (thrust 21..24); Dark Aura's start `PL1089 p003_044` (he rises, arms spread, 24..32); Holy Burst `PL1092 p002_075` / Inverse Burst `PL1093 p003_085` (the blade raised, 18..24); Barrage `PL1303 p002_072` / `PL1313 p003_082` (the hand up and down, 3..7) and `p002_070` / `p003_080` (the arms cross, 3..6); Maelstrom `PL1302 p002_065` / `PL1312 p003_075` (the spin, 10..16). Riku's own clips play on his skeleton as they are; Dark Riku's Dark Firaga and Dark Aura are retargeted onto him (0.184.1's retarget); Dark Riku plays his own.
- **Asset step** (`CastClip { src, .. }`, `CastClip::times`, `riku_cast_clips`, `dark_cast_clips`, `retime_in_place`): eight clips (`p002_227..234`, `p003_227..234`): Fire's wind-up and release (Dark Firaga), Blizzard (Impulse), Thunder (Dark Aura), Cure (Holy / Inverse Burst), Gravity (Barrage's strike), Stop (Barrage's crossing), Aero (Maelstrom); each with its key moment on the four frames after KH1's release frame and KH1's length (0.184.1's numbers); no travel. The Dark Firaga events go on each by their phase (`out_frame`).
- **DLL** (`crate::riku::{CAST_ROWS, CAST_CLIPS, DARK_CAST_CLIPS, cast_row}`): rows 0x3B..0x42 of both tables (`p002_000` / `p003_000` up to 0x46, Checked (exe)), one per clip; each spell's requests (ground and air) its own row.
- **Assumption**: each key moment reads as the cast in game (chosen from the clips' numbers, not seen).
- **Checked**: tests (`cast_clips_put_the_key_moment_on_kh1s_release`: six motions for the six spells; `rikus_cast_clips_follow_dark_firaga` with Dark Riku's eight; `every_kh1_cast_request_has_a_cast_row`: seven rows), the Windows build. **Not checked**: in game. The asset step's code changed: the first start builds again.

---

## Stage 337 (fork branch Re-Re-Reverse/Rebirth): Riku's spells show their effects in the field; every spell and level checked on KH1's Neoshadow, version 0.184.3 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "as magias funcionaram! mas fora do combate só a firaga solta projetil, mas agoras você precisa fazer o fine tuning, teste cada uma com o Neo shadow em cada nível e implemente o que faltar".

- **Cause (field)**: in Riku's story nothing loads the spell effects in the field (Sora's story gets Sora's battle set there since castfix, `win::magic_fx::field_load`); Dark Firaga's `dark_faiga` is Riku's own `EF1500.EFF`, so only Fire showed.
- **Change** (`win::magic_fx::{riku_wanted, riku_sora_set, battle_load, field_load, setting_changed}`): Sora's battle set 1001 is queued for Riku's KH1 spells also at each field room load (the field state's tail call) and when Combat is switched to KH1 in the camp, as in his battles; once loaded it stays (0.184.2).
- **Simulation** (`magic::tests::every_spell_and_level_on_kh1s_neoshadow`): each spell at each level on KH1's Neoshadow (the owner's `kh1-stats.json`, Checked (files): 150 HP, Defence 30, floor 10 / cap 40, level 55, resist % fire 49, blizzard / thunder 50, gravity 0, stop 20) with Riku's max MP 3 (the owner's log), KH1's formulas: Fire 4, Blizzard 5, Thunder 5 a hit at every level (the floor 10 times the resistance: KH1's Defence 30 against power + 3), Gravity 10 (the floor; its % times 0), Aero's wind 5 every 30 frames, Stop 60 frames (1 s), Cure 50 / 150 / 300 HP (Re:CoM's heal). At Damage = Re:CoM (the default) the owner's log shows Re:CoM's sleight AP: Firaga 150, Blizzaga 110, Thundaga 90, Graviga 70, Aeroga 90.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 336 (fork branch Re-Re-Reverse/Rebirth): Riku's spells show their effects and sounds; Fire is Dark Firaga; his cast motions at KH1's length, version 0.184.2 (2026-11-23)

Fork owner, 2026-11-23, verbatim (on 0.184.1): "pois é mas não tem efeito, som projetil cadê o resto?". The plan's item 0.184.2 (Riku's looks: Fire → Dark Firaga, the dark charge on the hand, his voice; the owner: "gostei das sugestões").

- **Cause** (Checked (decomp + exe), `docs/recom-m7-magic.md` § Spell effects in the field): Re:CoM's spell effects (`fire_all` … `airo_all`) and the sounds riding them load with Sora's battle set, PLAYER 1001, whose LINK rows alone load EFFECT 2000..2063. `0x140211960` picks the set by the world's character (`0x1407C7AE4`: 1001 Sora, 1101 Riku, 1201; Checked (decomp)); Riku's 1101 doesn't link them, so his KH1 spells (shots, shards, charges) drew nothing and made no sound.
- **Change 1** (`win::magic_fx::{battle_load, install_riku}`, `sites::RIKU_FX_BRANCH`): `CGameSubBattle` state 1's tail `jmp 0x140211960` (`0x1401AEA33`, `E9 28 2F 06 00`, Checked (exe)) goes to `battle_load`: Re:CoM's load, then, when it chose Riku's set and his KH1 spells are on, Sora's 1001 too (`0x14023CCE0(1001)` unless `0x1402A53E0(1, 1001)`; Re:CoM's loader's own pair, Checked (decomp)). Not unloaded: Re:CoM's unloader `0x140211560` (key `10000 + group`, `0x1402A5B00`) takes the world's set; 1001 stays resident, as Re:CoM keeps it under its system byte 0x60 (Assumption: no memory trouble). Field casts in Riku's story: not covered yet.
- **Change 2** (`crate::magic::{riku_look, DARK_FIRAGA_SET}`, `win::magic::{RIKU_TABLE, row, kh1_cast}`): Riku's spells read a copy of the KH1 rows, every number Sora's, Fire's effect set (`+2`) Dark Firaga's 48 (`dark_faiga`, Riku's own `EF1500.EFF`). Assumption: Re:CoM's Fire ball reads set 48's slots as it reads `fire_all`'s.
- **Change 3** (asset step, `kh1_recom_assets::riku_skin::{cast_events, event_time, out_frame}`): the Dark Firaga clip's events (`PL1088.EET` / `.SES`, `p003_043`; Checked (files)) on the six cast clips at their retimed frames: `dark_faiga` 0 on the left hand from 20 (the charge's dark glow: every spell), `dark_faiga` 1 on Root at 72 (the thrust's flash), the voice 3035 at 11, the thrust's sound 605/2 at 72; 605/1 at 16 (the leap's landing) dropped. In Riku's `PL0101` / `PL0111` and Dark Riku's `PL0201` / `PL0211.EET` / `.SES` (Re:CoM's own events kept), the skin's packs too.
- **Fixed** (0.184.1): the Dark Firaga clip is 30 fps (`flags` bits 2..4; Checked (files)) and the cast clips kept that rate, so each played twice KH1's length; they are 60 fps now (one frame per KH1 frame). Event frames (60 Hz) are taken to the source clip's frames before retiming.
- **Checked**: tests (`rikus_rows_change_only_fires_effect_set`; on the owner's install `rikus_cast_clips_carry_dark_firagas_events`, `rikus_cast_clips_follow_dark_firaga` with 60 fps), the Windows build. **Not checked**: in game. The asset step's code changed: the first start builds again.

---

## Stage 335 (fork branch Re-Re-Reverse/Rebirth): Riku casts with Dark Riku's Dark Firaga motion, on KH1's cast timing, version 0.184.1 (2026-11-23)

Fork owner, 2026-11-23, verbatim (on 0.184.0): "a animação do firaga está completamente errada... as iutras magias  simlesmente sem animação, nice". The plan's item 0.184.1 (the cast motion: Dark Riku's Dark Firaga, the owner's "pode ser").

- **Cause** (Checked (the owner's 0.184.0 log + exe)): every spell cast and released (Fire, Blizzard, Thunder, Cure, Gravity, Aero; free). KH1's cast requests asked Sora's rows: on Riku's table 0x36 / 0x37 / 0x38 are `p002_050` / `p002_057` / `p002_058` (other moves: Fire's "wrong" motion), and 0x39.. `p002_000` (no clip: the current motion kept).
- **The clip** (Checked (files)): Dark Riku's Dark Firaga `p003_043` (`PLAYER/1088/PL1088.ANN`, 52 frames): 0..8 a 2.4 m leap back, 8..31 a crouched charge (left hand drawn back), 32..36 the left hand's thrust, 36..51 the recovery.
- **Asset step** (`kh1_recom_assets::riku_skin::{CAST_CLIPS, CastClip::times, retime_in_place, dark_to_riku_map, riku_cast_clips, dark_cast_clips}`): six clips with KH1's lengths and the thrust on KH1's release frame (Sora's `xa_ex_0010.mset`, Checked (files)): Fire's wind-up (28, the charge), Fire's release (60, free 46, the thrust at 0), Blizzard / Gravity (86, free 68, release 28), Thunder / Aero (68, release 30), Cure (68, release 36), Stop (68, release 28). The leap is taken out (Root and the hips keep their rest x / z; the crouch stays). Riku's (`p002_227..232`): the clip retargeted Dark → Riku (bones by name; his `k10` / `k11` and his own bones from his battle stance; `wep0` by its world rotation), in `PL0101` / `PL0111.ANN`; Dark Riku's (`p003_227..232`) the clip itself, in `PL0201` / `PL0211.ANN`; the Dark Form skin's packs get Dark's under Riku's names. Manifest `"riku_cast": true`.
- **DLL** (`win::riku_skin::{guard_rows, cast_row}`, `crate::riku::{CAST_ROWS, cast_row}`, `win::movement::RecomHost::play`): rows 0x3B..0x40 of both tables (`p002_000` / `p003_000`, Checked (exe)) named after the clips (checked first); every KH1 cast request (ground and air) plays its row.
- **Checked**: tests (`cast_clips_put_the_thrust_on_kh1s_release`; `rikus_cast_clips_follow_dark_firaga` on the owner's install: Riku's blade turns as Dark Riku's, cos 1.000, no travel; `every_kh1_cast_request_has_a_cast_row`), the Windows build. **Not checked**: in game. **Assumption**: Re:CoM's rebind by name binds the new rows for both (as the guard rows). The asset step's code changed: the first start builds again.
- **Left for 0.184.2**: the spells' look (Riku's effects and voice).

---

## Stage 334 (fork branch Re-Re-Reverse/Rebirth): Riku casts Sora's KH1 spells (MP free for the test), version 0.184.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "implementar as mesmas magias que o Sora Usa, só que uma verão que seja mais coerente com o Riku e os efeitos que o personagem dele usa nas iterações do Kingdom Hearts"; "por enquanto só a aparência mas guarde como oportunidade futura esses achados"; "gostei das sugestões implemente inicialmente com custo zero de mana só para testar e depois volta o custo de MP e deixa desbloqueado no menu tbm"; "pode ser" (the plan: 0.184.0 the cast, 0.184.1 a cast motion, 0.184.2 Riku's looks), "SIM e continue!".

- **Re:CoM** (Checked (exe)): Re:CoM's Battle exe runs action 0x13 through vtable `+0x150` (`0x1401F9B4D`). Sora's `initMagic` / `exeMagic` are `0x1402203D0` / `0x140217EE0`; Riku's and Dark Riku's slots `+0x148` / `+0x150` hold the base class's empty `ret`s `0x1401F2F20` / `0x1401EEAA0`: Re:CoM's Riku has no magic action. The Field exe's case 0x13 (our stub, shared with Riku: his Field exe is Sora's `0x1401FA0F0`) calls Sora's exeMagic directly.
- **Change**:
  - `win::magic::riku_slots`: Riku's and Dark Riku's `+0x150` point at `win::magic::exe_magic` (checked old value, not fatal). `exe_magic` runs Sora's own exe only on Sora's object; on anyone else it runs the KH1 frame of a KH1 cast, or nothing (the stub's behaviour), so the Field case no longer reaches Sora's exe with Riku.
  - `win::movement::run_frame`: Riku's KH1 frame takes the Magic command and its target as Sora's, and reports the cast events.
  - `win::command_menu::riku_world`: his Magic list holds Sora's seven spells at level 3 (unlocked), with the target distance for the ranged ones.
  - `riku_free_magic = on` (new ini key, a test setting, on by default for now): `World::free_spells` (the menu never greys a spell for MP), `win::magic::{can_cast, pay_spell}` (no MP test, nothing paid at the release). Off: Sora's KH1 costs.
- **Left for 0.184.1 / 0.184.2**: Riku has no cast clip (his rows 0x37..0x3C are `p002_057` / `p002_058` / `p002_000`): the cast plays on KH1's frame with his current motion kept (logged per row). The spells' look is Re:CoM's Sora effects; their packs come with Sora's battle set (PLAYER 1001, `crate::magic::field_case`), so in Riku's battles they may not be loaded (no effect drawn: Re:CoM's lookup finds no group). Not checked.
- **Checked**: tests (new `free_spells_never_grey_for_mp`; the ini round trip), the Windows build. **Not checked**: in game (the first Re:CoM magic action on Riku's object).

---

## Stage 333 (fork branch Re-Re-Reverse/Rebirth): the Guard's start measured (simulation against the Neoshadow) and every Square press logged, version 0.183.4 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "o timing da defesa segue horrível preciso de uma correção se possível faça o teste do neoshadow com o RIku tbm"; asked what feels wrong they chose "Demora para começar", and for the Neoshadow test "Simulação nos testes".

- **The owner's 0.183.3 log** (Checked (trace)): every guard window lasted 433 ms (26 frames, KH1's); no melee hit was blocked by the blade (the Wyvern's bite, priority 100 in every row, got through "not on the blade" twice and once "on the blade" by its priority); two Wizard shots were blocked. The press itself was not logged, so the delay from the button could not be read.
- **Simulation** (`movement::tests::rikus_guard_against_the_neoshadows_windows`, Riku's abilities on the shared port): the guard object is live on frames 11..=36 after the press. KH1's motion 212 has it on frames 10..36 (Checked (files)): the port is one frame (16 ms) later, because a frame's motion update runs at the next handler call (the oracle's order), the same 26 frames. Against the Neoshadow's windows (Checked (files), the asset step's test): its claw (202, hits 40..=67, priority 40) is blocked when Square is pressed on its frames 4..=29; its rising slash (207) and dive (205) have priority 100, which KH1's Guard never stops (`kh1_core::enemy::guard::meeting`: the attack is not marked).
- **Log** (`win::movement::square_log`, `win::attack::guard_line`): each Square press a KH1 frame sees, in Sora's / Riku's guard timeline: "Guard started", "a roll", or "dropped" with KH1's state, action lock and request (KH1 drops a Square while the lock holds or off the ground: `0x140297870`, `0x1402A7B30`, Checked (decomp)).
- **Not changed**: KH1's start (10 frames) and window. Seen, not changed: after a hit gets through, the guard object stays live while Re:CoM's damage reaction plays (the KH1 frame doesn't run in it): about 680 ms in the log.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 332 (fork branch Re-Re-Reverse/Rebirth): Sora's Guard by the blade too: one KH1 Guard for Sora and Riku, version 0.183.3 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "a mecânica do RIKU precisa ficar igual a do Sora...,  isso é para agora!"; asked which of the two becomes the rule they first chose "Os dois pelo corpo", then: "n~ao mas o do Sora no KH 1 também é pelo corpo não?"; told KH1's guard is the Keyblade's volume (the workspace's `knowledge/KH1/kh1-sora.md` § Guard in detail, Checked (decomp + files)): "então mudança de rota, alternativa a" (both by the blade, as KH1).

- **Before**: Sora's Guard was Re:CoM's body guard (any hit from within ±100°, `set_guard`; the 2026-11-03 Assumption of `docs/recom-m11-enemies.md`), Riku's the blade test (0.182.0 / 0.182.1). KH1 blocks only what touches the Keyblade.
- **Change** (`win::attack::{blade_guard_pair, guard_blade_of, guard_faces_of}`, was `riku_guard_pair` / `riku_blade` / `riku_faces`; `crate::riku::{guard_faces, guard_blade, BLADE_END}` shared): Sora's Guard runs the same path as Riku's: his blade (Re:CoM's weapon line on his weapon bone, the Guard record's radius on the equipped weapon), the facing ±100°, then KH1's types and priorities (0.183.1). A hit off the blade, or from outside the front, gets through (his body guard off for the test). `sora_guard_pair` (0.183.1's body-guard version for Sora) is gone.
- **Kept**: the Yellow Opera's Thunder is never blocked by Sora (KH1: no clash pass), still blocked by Riku (the owner's 0.153.0 rule).
- **Log**: Sora's lines are `sora guard: <ms> ms: …` (300 lines) with the same timeline as Riku's (guard object on / off, blocked by the blade, hit got through, guard success, post-block protection).
- **Assumption**: Re:CoM's weapon line (−0.8 m on the weapon bone) stands for the Keyblade's length, as for his Re:CoM swing volume, whatever KH1 Keyblade model is shown.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 331 (fork branch Re-Re-Reverse/Rebirth): Dark Riku guards in KH1 Riku's guard pose, version 0.183.2 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "certo novamente quero que revalide a mecanica de guard do Riku explorando se ficou alguma coisa fora"; on the findings and the plan (0.183.0 / 0.183.1 / 0.183.2), "vou pela tua recomendação mas quero testar todos de uma vez", then "sim". Item (c) of the plan: Dark Riku's pose.

- **Cause**: the real Dark Mode's Dark Riku guarded with his clash recoil (row 0x24, `p003_033`). Since 0.182.0 the block is decided by where the blade is, so his Guard was judged on a pose that is not KH1's.
- **Asset step** (`kh1_recom_assets::riku_skin::{dark_guard_pack, DARK_GUARD, DARK_RECOIL}`): with KH1 Riku's guard (`xa_ex_1010`), his own packs `PLAYER/0201/PL0201.ANN` and `PLAYER/0211/PL0211.ANN` get the guard (`p002_225`) and its recoil (`p002_226`) retargeted onto his skeleton (`ex_1580_sk`, the Dark Form skin's retarget), named `p003_225` / `p003_226`. Checked (tests: `dark_rikus_own_pack_gets_the_guard` on the owner's install).
- **DLL** (`win::riku_skin::{guard_rows, name_rows, guard_row, recoil_row}`): rows 0x39 / 0x3A of his action table (`0x1407C6300[5]` / `[6]` = `0x1407C67F0`, both `"p003_000"`: Checked (exe)) are named after them, checked first; then his Guard and its recoils play them, as Riku's. Assumption: his player build rebinds rows by name as Riku's does (`0x1401F4240`). Without the files (an asset step before 0.183.2): his clash recoil as before, logged once.
- **Assumption**: Re:CoM's loader takes his pack with two more slots at the end (Sora's `PL0001.ANN` with 161+ slots is the precedent), and nothing but the player loads `PLAYER/0201` / `0211`. Every row is checked before any is renamed (`name_rows`).
- **On the owner's side**: the step's code changed, so the DLL's first-launch build runs again at the next start (its console window; `docs/openkh-github-install.md`) and writes the two packs.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 330 (fork branch Re-Re-Reverse/Rebirth): the Guard against KH1 enemies by KH1's attack types and priorities (Sora and Riku), version 0.183.1 (2026-11-23)

Fork owner, 2026-11-23: the asks of Stage 331 (item (a) of the plan they approved: block only what KH1 blocks).

- **KH1** (Checked (exe + decomp), the workspace's `knowledge/KH1/kh1-sora.md` § Guard in detail): the guard object (Guard row: clash type 2, priority 0 + the weapon's) is tested against an enemy attack only when the types meet (`0x1402B3BE0`, `0x140528CB8`: type 2 meets types 0 and 3). Then `0x1402B4A20` / `0x1402B4BC0` give each side its outcome by priority (`kh1_core::enemy::guard::meeting`): the guard side's record kind 6, or 7 when more than 40 under (request 0x6F, no Counterattack), nothing against priority 0; kind 6 pushes back by `−diff × 2.5` when `diff` < 0 (`0x14029D720`); the enemy side a clash record, nothing at 98–100, a repel for a stronger type 1 / 3 / 4; an attack meets no body only when it is stopped.
- **Change** (`win::attack::{kh1_guard, kh1_block, sora_guard_pair, pair_unguarded}`, `win::kh1_enemy::attack_row`, `Host::guard_clash`, `movement::guard_success`): for a KH1 enemy's attack (the asset step's clash records) while the KH1 guard is live:
  - types that don't meet, or not stopped: the hit gets through (Sora: his body guard off for the test; Riku: also when it is on the blade);
  - stopped, the guard side with a record: the block, and the guard success takes its kind and push (0x6F for kind 7);
  - stopped with no record (priority 0): the attack stops, no recoil, the guard goes on (Riku: no test, no hit recorded);
  - the enemy's reaction (`super::guard::on_result`: its KH1 clash recoil, a shot turned back) only when it gets a clash record.
  - Re:CoM's own attacks have no KH1 type or priority: as before (kind 6, no push).
  - a priority-0 attack lying on Riku's blade for several frames is stopped every frame, its reaction and log line once per attack (collision + serial) per guard; a block with no record leaves an earlier block's guarded bit of the same frame in place.
- **Protection after kind 7**: KH1's flag 0x80 now holds while the strong recoil 0x6F holds the lock, as during 0x6E (`movement::untouchable_step`; it ran out ~8 frames into 0x6F, `swing_clash`'s kind 7 too).
- **Left** (the mechanic review, 2026-11-23):
  - a type 0 attack of priority 98, 99 or 100 (the Neoshadow's dive, the Wyvern's bite) gives the guard side kind 7 in KH1 *and* still hits (not stopped); here it only gets through. A KH1 projectile turned to the other side (`+0x30 & 0x400`) is not modelled (`kh1_core`).
  - KH1 enemies' thrown objects, rings and held attacks that are Re:CoM bullets or records with no clash row (the Bandit's scimitar, priority 0) keep Re:CoM's block: a recoil and the Counterattack, where KH1 gives the guard no record.
  - KH1 meets a pair once while both live (the `+0x9C` mask); not modelled beyond the once-per-attack memo below.
  - Riku's kind 7 plays his one KH1 recoil clip (row 0x3A) for 0x6F. (Sora's guard volume: the blade since 0.183.3.)
  - Riku: when Re:CoM's answer was a hit turned into the block, whether the pending hit still costs HP is not shown; logged (`pending hit … value …`).
- **Log**: `sora guard: …` (60 lines) and `riku guard: … KH1 clash kind …` / `… stopped … with no recoil …` / `hit got through (on the blade, but KH1's Guard doesn't stop it: its type or priority)`.
- **Checked**: tests (new `guard_success_takes_kh1s_clash_kind_and_push`), the Windows build. **Not checked**: in game; the enemies' real types and priorities (their manifests weren't on this machine).

---

## Stage 329 (fork branch Re-Re-Reverse/Rebirth): the Guard turns to Auto Lock's target and restarts the combo (Sora and Riku), version 0.183.0 (2026-11-23)

Fork owner, 2026-11-23: the asks of Stage 331 (item (b) of the plan they approved: turn to the right enemy and restart the combo).

- **KH1** (Checked (decomp), the workspace's `knowledge/KH1/kh1-sora.md` § Guard in detail and `kh1-lock-on.md`): `0x1402A3B00` turns the Guard to the lock pair `0x142DB3518`, which holds the soft target in lock states 1–4 (Auto Lock's orange ring, no R1), and sets the combo counter `CB + 0x41` to 1.
- **Change** (`movement::pad_handler`, `win::movement::read_pad`, `win::attack::soft_target_centre`): on a Square press the frame's `lock_target` is R1's lock, else Auto Lock's soft target (nothing held: the Guard is no command), and the Guard faces it; it sets `combo_counter` to 1. Before, it faced R1's lock only (the soft target was read on Attack presses only) and kept the counter, so a combo went on through a Guard.
- **Checked**: tests (new `guard_faces_the_soft_target_and_restarts_the_combo`), the Windows build. **Not checked**: in game.

---

## Stage 328 (fork branch Re-Re-Reverse/Rebirth): Riku's guard timeline in the log, version 0.182.2 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "TUDO ela não bloqueou, mas eu acho que está ligado ao tempo da defesa que está zoado ainda, além disso até onde eu sei quando vc da um block com o sora, e vem um logo em seguida o block pega o proximo ataque tbm confere ai no KH1".

- **The owner's log** was still 0.182.0's (the OpenKH build's DLL dated before 0.182.1 was copied), so 0.182.1 is not yet tried.
- **KH1** (Checked (files + decomp), `kh1-core` data and `crate::movement::guard_success`, the shared port): after a block, the recoil `0x6E` (60 frames) is free from frame 28 (Guard again from there), and flag 0x80 (no damage, no reaction) holds while the recoil holds the lock, then 8 more frames. So a hit that comes right after a block, in about the first 36 frames, does nothing. Riku has the same rule through the port (Re:CoM's `+0x418 & 1`, which his damage check skips too).
- **Change** (log only, `win::attack::{riku_ms, riku_untouchable_log}`): Riku's lines get a millisecond time, and the guard object on / off, the guard success and the post-block protection on / off are logged (300 lines).
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 327 (fork branch Re-Re-Reverse/Rebirth): Riku's Guard: a hit off the blade gets through (Re:CoM's body guard no longer blocks it), version 0.182.1 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "está um pouco melhor mas está longe de perfeito, se a defesa é no CORPO ainda está pegando a espada, como disse precisa estar igual no KH1".

- **Cause** (Checked (the owner's 0.182.0 log)): no `riku guard: blocked by the blade` line; every block was Re:CoM's own result 2 (`guard reactions: block …`, Wizard shots, a Shadow). Re:CoM's body guard, which `set_guard` turns on (±100°) while his KH1 guard is live as for Sora, does block for Riku, against 0.159.0's note. So a hit off the blade was still blocked by his body, and 0.182.0's blade test only decided the hits Re:CoM had not blocked.
- **Change** (`win::attack::riku_guard_pair`): an attack that doesn't touch his blade, or whose owner is not in front, is tested by Re:CoM with his body's guard bit (`+0x9C & 1`) off for that test, so it hits him, as in KH1. On the blade and in front: Re:CoM's block (2) is kept, a hit (1) becomes the block, as in 0.182.0.
- **Log**: `riku guard: blocked by the blade` (also "(Re:CoM's block)") and `riku guard: hit got through (not on the blade[, not in front])`, 40 lines.
- **Left**: an attack shape that is neither a sphere nor a line, or no weapon bone, keeps Re:CoM's body guard (logged once).
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 326 (fork branch Re-Re-Reverse/Rebirth): Riku's Guard blocks by KH1's rule: an attack that touches his blade from in front, version 0.182.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "a defesa ainda está meio esquisita, mas é mais o uso geral dela mesmo tem como verificar como ela é implementada tanto no Sora quanto no jogo original e ajustar no Riku?"; on what feels odd they chose all four (when it can be used, what it blocks, direction / target, length / rhythm); then "o ponto é usar o KH1 como referência … a adaptação deve ser precisa se não vai ficar nesse vale da estranhesa"; on the plan (1 the blade, 2 the made-up rules), verbatim: "1 e 2 fica IGUAL KH1".

- **KH1** (Checked (decomp + files); the workspace's `knowledge/KH1/kh1-sora.md` § Guard in detail): the guard object is the Guard record `0x4DA8` on the weapon (`bone 0x30`, the blade line, radius 0 + the weapon's: Kingdom Key 40), live on motion 212's hit frames 10–36. It blocks an enemy attack that touches that blade (`0x1402B4A20`) from an owner within ±100° of the facing (`0x1402B4640`, record `+0x64` = cos 100°). The blocked attack meets no body (`+4 & 0x30`). A hit on the body that misses the blade is not blocked.
- **Compared**: when the Guard starts, its turn to the target, its length and its recoil / Counterattack are the shared KH1 port (`crate::movement`), as Sora's (checked against `0x1402A7B30`, `0x1402A3B00`, `0x14029D720`, `0x140294220`): no change. What was not KH1: Riku blocked any hit on his body from in front (0.159.0), a shot within 1 m was judged by its caster, and a melee attacker past 4 m was blocked without a reaction (0.158.2).
- **Change** (`win::attack::{riku_guard_pair, riku_blade, attack_contact}`, `crate::riku::{guard_blade, BLADE_END}`): before Re:CoM's pair test of an enemy attack against the player list, while his KH1 guard is live:
  - his blade is Re:CoM's own weapon line (`0x140225840`, Checked (decomp)): the owner's bone `weapon + 0x238` (weapon `+0x2CC & 4`) or the weapon's bone 2, from its origin (Assumption: the hilt) to (−0.8, 0, 0) in its frame (`0x1407C7E50`, Checked (exe)), as spheres of the Guard record's radius on the equipped weapon (Assumption, as his swing's clash since 0.169.0);
  - the attack's shape (sphere or line) against them, by Re:CoM's overlap tests; its owner within ±100° (a shot by its own place, Assumption);
  - on a block: Re:CoM's test, else its apply on his body with the blade's contact, then the result is Sora's block (2, the guarded bit, the attacker), so the shot turns back, the attacker reacts and the KH1 guard success follows. Not touching the blade: the hit gets through.
  - `riku_pair_block`, the 1 m caster rule and `RIKU_MELEE_REACH` are gone.
- **Left as before**: the Yellow Opera's Thunder is still blocked by Riku (0.153.0, the owner's rule; KH1 doesn't).
- **Not settled**: which enemy attacks KH1 can't guard (priority rule, the knowledge note's open question).
- **Checked**: tests (new `rikus_guard_blade_is_recoms_weapon_line`), the Windows build. **Not checked**: in game.

---

## Stage 325 (fork branch Re-Re-Reverse/Rebirth): one ring: Re:CoM's target marker hidden while KH1's Auto Lock runs unlocked; the crate's version, version 0.181.1 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "funcionou perfeitamente,mas ficou duplicado o anel". Answering what the two rings look like, they chose "Um laranja e outro diferente".

- **Cause**: the owner's log shows our `lockon_d` made once per appearance. The second ring is Re:CoM's own `CLockonMarker` layer, which it draws on its pick `+0x84C` without a lock too. KH1 shows only the orange ring then, and the lock cursor only with a lock (`0x140274B20`, Checked (decomp)).
- **Change** (`win::kh1_hud::recom_marker`, called from `auto_ring` in the `0x140209D30` wrapper): at `hud = kh1` with Auto Lock on, Combat = KH1 (Sora or Riku) and no lock, Re:CoM's marker layer is set not drawn (`0x140247E50(ctrl, 0, 0)`, as H7 did). With an R1 lock, `auto_lock = off`, or the HUD off or Re:CoM's, it is shown again (`0x140247E50(ctrl, 0, 1)`; Assumption: 1 shows, as `win::keyblades` uses it).
- **Version**: `crates/kh1-recom/Cargo.toml` (and `Cargo.lock`) are at 0.181.1. The log's first line and the Combat row's "mod v" read it. Since 0.180.0 only `VERSION` had moved, so the log said 0.179.1.
- **Checked**: tests, the Windows build. **Checked (in game, fork owner, 2026-11-23)**: "ficou perfeita a implementação do anel amarelo e autolock".

---

## Stage 324 (fork branch Re-Re-Reverse/Rebirth): KH1's Auto Lock: the yellow ring's enemy is the attacks' target, KH1's pick, its hold and its menu rule, version 0.181.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "para ambos o anel amarelo não faz lock no oponente ainda igual faz no KH1". Then "no kh tem o anel amarelo e o azul, o azul é quando tu aperta RB e ele locka no oponente, o amarelo ele aparece no oponente usando a regra do jogo, e se vc tiver com o auto lock ativo, os teus ataques sempre serão na direção daquele oponente independente de onde tu esteja olhando, se o anel amarelo estiver ativo ele se comporta assim". The plan was approved with the menu rule ("Tudo, com o menu").

- **KH1** (Checked (decomp + exe + files); the workspace's `knowledge/KH1/kh1-lock-on.md`, researched with the new `searcher` agent):
  - **The command target.** The command run `0x140293480` makes the lock pair the command target whenever the lock state is not 0. So the soft target (state 2) aims a swing as a lock does: the ground and air pickers (`0x1402A6EE0` / `0x1402A6C70`) replace the stick with the direction to it, and the lunge rows follow from the gap.
  - **The pick** `0x1402B0A00`: 3 m flat less the lock radius, ±3.5 m, score `((1 − cos) × 0.25 + 1) × gap + |dy| × 0.2`, plus a line of sight test.
  - **The auto lock.** A command with flag 4 turns state 2 into the auto lock (state 4, `0x1402B08A0(…, 4)`). Its enemy holds until 90 of frame delta after the last such command, L2 / R2, or 15 m / 10 m (`0x1402B11B0`, `0x1402B13F0`).
  - **The menu rule.** A soft target is picked only while the highlighted command (`0x1402931B0` → `0x142D539E8`) has flag 4 (`btltbl.bin` `+0xA58`).
- **Mod before**: `attack::command_attack` aimed only with Re:CoM's lock (`locked`), so with the ring alone Sora swung along the stick. The swing took Re:CoM's pick `+0x84C` while the ring took the soft target, which could be another enemy. The pick was the nearest within 6 m, with no hold and no menu rule.
- **Change**:
  - Pure code:
    - `lock_on::soft::{pick, holds, Held}`: KH1's pick, without the line of sight test (Assumption: no Re:CoM twin), and the hold.
    - `command_menu::cmd::{TARGETED, targets}`: flag 4 of commands 0..0x7F from the owner's file. The sleights count as having it (Assumption).
    - `Menu::highlighted_targets`: the Items, Summon and Party lists' rows have no flag.
    - `trace::PadView::auto_target`: it makes `FrameInput::locked`.
  - DLL:
    - `win::attack::{take_target, target_of, part_of}`: Re:CoM's lock, else the soft target, else Re:CoM's pick (the picker's part only).
    - `win::lock_on::{soft_target, hold, hold_tick, locked, facing}`: the hold, ticked in the lock-on wrapper with `+0x460`. It ends at 90 frames or on an L2 / R2 press (Assumption: a press), and R1's lock replaces it.
    - `win::command_menu::highlighted_targets`.
    - `win::movement::read_pad` and the magic / special commands take their target through `take_target`.
  - The ring uses the same `soft_target`, so the ring and the swing are on one enemy.
- **Not 1:1**:
  - No line of sight test.
  - The facing is the mover's yaw (`+0x54`).
  - Assumption: KH1's delta is in 60 Hz frames (1.5 s).
- **Checked**: tests (`kh1_soft_target_pick_and_hold`, `kh1_flag_4_commands`, `auto_target_aims_like_a_lock`), the Windows build. **Checked (in game, fork owner, 2026-11-23)**: "ficou perfeita a implementação do anel amarelo e autolock".

---

## Stage 323 (fork branch Re-Re-Reverse/Rebirth): Riku lowers his guard as the defence ends, version 0.180.1 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "no Riku a janela de defesa não está com a mesma duração da animação"; then "a defesa acaba antes dele baixar a guada". Answering "(a) the animation follows the defence; (b) the defence follows the animation", they chose (a).

- **Measured** (Checked (files), new `riku_guard_motion_profile` on the owner's install): in the built clip (54 frames), the defence is KH1's Guard window 10..36 (as Sora's), and the weapon hand is held forward through frame 38. It started down only at 39–40 and was lowered by 46, because the way out played 240's tail backwards, and 240 (into the guard) has its arm up for its last frames.
- **Change** (asset step, `riku_skin::riku_kh1_guard`, `settled_from`, `GUARD_ARM`): the way out starts from the 240 frame where the weapon arm (`r_collar`, `r_uparm`, `r_foarm`, `rx_rot`, `r_hand`) reaches the guard (within 5 % of its change). The hand now comes down from frame 37 (0.36 → 0.33 → 0.31 m forward) and is down about 2 frames earlier. The defence window is unchanged.
- **Asset step**: re-run (automatic; its source changed).
- **Checked**: tests (the profile test asserts the hand is lower by frame 38 and ends lowered), the Windows build. **Not checked**: in game.

---

## Stage 322 (fork branch Re-Re-Reverse/Rebirth): KH1's orange auto-target ring on the soft target, version 0.180.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "preciso que vc implemente o anel laranja/amarelo igual funciona no KH 1 inclusive já tem nas configs dentro do jogo das ultimas implementações sobre o auto-lock"; answering "(1) HUD: com qualquer HUD ou só com `hud = kh1`? (2) aprova o plano (projeto Ghidra do KH1)?", verbatim: "1 só com o modo do kh1 ligado, 2 sim".

- **KH1** (Checked (decomp, this machine's new `KH1` Ghidra project; files); the KH modding workspace's `knowledge/KH1/kh1-lock-on.md`): the draw `0x140274B20` shows `lock` object 3 (`lockon_d`, `0x14284F9F8`, set by `0x140274290`) for lock states 1, 2 and 4. State 2 is the soft target, picked each frame by `0x1402B11B0` → `0x1402B0A00` only with Auto Lock On (`0x1402FAFB0` ≠ 1). State 4 is an attack's auto lock (`0x140293480` → `0x1402B08A0(…, 4)`). The ring is centred on the target's projected lock point and not scaled with the distance. Its keys restart only when the cursor kind changes, not when the soft target moves to another enemy. The owner's `UK_uibin_lock.bin`: 30 × 30, colour (112, 80, 12), alpha 0 → 250 → 112. Its texels are in `uitex.bin` picture 10, with `lockon_c`'s.
- **Change**:
  - Asset step: `kh1_hud::cursor::{RING, build_ring}`, a group after the cursor's (manifest `"auto_ring"`).
  - DLL: `kh1_hud::Manifest::auto_ring`, `win::lock_on::ring_target`, `win::movement::main_player`. `win::kh1_hud::marker_update` (Re:CoM's `CLockonMarker` update `0x140209D30`, the H7 site, prologue already checked) is now installed for the ring as well and calls `auto_ring` (and H7's `kh1_cursor` only with `LOCKON_KH1`). At `hud = kh1`, with Auto Lock on and Combat = KH1 (Sora or Riku), our projected control draws the ring on `soft_target`'s lock point while Re:CoM's cursor holds no lock. Re:CoM's own cursor still shows a lock.
  - Off (`auto_lock = off`, `hud = recom`, Combat = Re:CoM): no ring, as Re:CoM.
- **Asset step**: re-run (a new group). An older `kh1-hud.json` logs "no auto_ring" and draws no ring.
- **Not 1:1**:
  - The ring is on our soft target (`SOFT_REACH` 6 m, Assumption since 0.175.0). KH1's pick is 3 m across (less the point's radius) and ±3.5 m up, scored by facing and height, with a line-of-sight test (now Checked, the knowledge note). Not changed here: it is the swing's aim, another system.
  - The ring is drawn wherever Re:CoM's marker update runs. H7 read that it runs each battle frame; whether it runs locked or not, and in the field, is an Assumption. The ring is deleted with the player's HUD (the gauge's dtor, as H7's cursor).
  - KH1 hides the ring when the target is off screen. Here Re:CoM's projected control draws it (Assumption: as its own marker).
- **Checked**: tests (new `auto_ring_from_the_players_file` on the owner's `UK_uibin_lock.bin` and `uitex.bin`; manifest parse), the Windows build. **Checked (in game, fork owner, 2026-11-23)**: "ficou perfeita a implementação do anel amarelo e autolock".

---

## Stage 321 (fork branch Re-Re-Reverse/Rebirth): the DP sleights' mark, Re:CoM's Heartless emblem right of their names, version 0.179.1 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "preciso que no shorcut e no menu de commands tenha algo identificando que consomem DP, pode ser o ícone de um heartless"; on its place: "a direita fica perfeito".

- **Re:CoM** (Checked (files)): the Heartless emblem in `FORM/0104/FO0104.RTM`'s PC asset `UK_levelup.imz` (IMGZ of one 32-bit IMGD, PS2 alpha), about (136, 55)–(215, 172). Assumption: it is Riku's level-up DP icon. KH1's `Icon.tm2` and `uitex.bin` have no such art.
- **Change**: asset step `kh1_hud::menu::{dp_emblem, DP_EMBLEM_*}` (one group after the Summon list's, manifest `"dp_icon"`; `MenuSources::dp_emblem` read in `step::kh1_hud_build`); DLL `kh1_hud::MenuGroups::dp_icon`, `win::command_menu` (`dp_word`, `Row::icon`, `ShortCtl::icons`): the mark 3 units right of a DP sleight's (squeezed) name in the Sleights list and the shortcut panel, KH1's menu look, at the name's brightness. Re:CoM's menu look: not drawn (its words are `C2DFnt` text).
- Asset step: re-run (a new group).
- **Checked**: tests (`dp_emblem_from_the_players_file` on the owner's `FO0104` texture: 36 × 48; manifest parse), the Windows build. **Not checked**: in game.

---

## Stage 320 (fork branch Re-Re-Reverse/Rebirth): Dark Riku's own sleights pay DP (Inverse Burst added); the fork's design intent recorded, version 0.179.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "o MP contina, mas os Sleights que só o Dark Riku Executa, consomem DP"; answers "1 a,2a" (Inverse Burst in his list; Dark Break / Dark Firaga 20 %, Dark Aura / Inverse Burst 30 %); and the intent "não é refazer o KH1 de 1:1 no Recom mas sim reimplementar o Combate, mantendo a originalidade do Kingdom Hearts Recom" (answer 5a: record it).

- **Re:CoM** (Checked (exe + decomp)): Riku's start `0x140216AB0` refuses 0x5F..=0x61 and 0x65; Dark Riku's `0x140206830` (switch on `+0x82C`) takes them, 0x5F..=0x62 / 0x65 through `0x14020E590` then `+0x138`, `+0x838` = 10. Dark Mode's DP: the player's `+0x44C`, max `+0x450` (his DP stat), the gauge setter `0x14020BFA0` (as 0.152.0 / 0.166.0).
- **Change**: `crate::dark_points::{DP_SLEIGHTS, sleight_cost, can_pay}`, `Dp::{sleight_cost, pay_sleight}`; `crate::sleights::{Cost::Dp, usable_with_dp}`; `command_menu::World::dp`; `win::dark_points::{sleight_view, pay_sleight}`; `win::summons::start_sleight` checks the DP before Re:CoM's start and pays after it (Re:CoM's Dark gauge written at once). Cost = 20 % / 30 % of Dark Mode's max, at least 2, only with 1 DP left after (a sleight never ends Dark Mode). His other sleights keep MP. `riku::DARK_SLEIGHTS` gains Inverse Burst (0x65; the asset step's name word `0x8165`); `riku::list` returns a slice.
- **Docs**: `docs/design-philosophy-instructions.md` § Fork branch: the fork owner's intent (Re:CoM's originality ahead of KH1 1:1 on this branch); `docs/recom-riku-kh1.md` § 0.179.0.
- **Assumption**: Inverse Burst's pack loads in his battles as the other three's.
- Asset step: its sources changed (the name), so the first start builds again.
- **Checked**: tests (`dark_rikus_own_sleights_pay_a_share_of_dark_modes_dp`, `rikus_sleights`), the Windows build. **Not checked**: in game.

---

## Stage 319 (fork branch Re-Re-Reverse/Rebirth): one debug key, `all_unlock`, for every costume and cosmetic Keyblade, version 0.178.5 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "pode deixar as condições do INI em OFF e unifique elas para all_unlock=off informando que é só para propósitos de debug!".

- **Change** (`settings`, `win::riku_skin`, `win::sora_costume`): `riku_halloween_unlocked`, `riku_dark_skin_unlocked`, `riku_keyblade_unlocked` and `sora_costume_unlocked` (`Settings`' four debug fields) are one `all_unlock` (default off), its ini comment "FOR DEBUG PURPOSES ONLY". The story requirements are unchanged: Riku's Dark Mode costume and Keyblade of Heart at his first Dark Mode, his and Sora's Halloween Town costumes at reaching Halloween Town. An old file's four keys are ignored without a warning (as `kh1_motions`), so an old `on` no longer unlocks anything.
- The fork owner's own `kh1-recom.ini`: the four keys replaced by `all_unlock = off` (backup `kh1-recom.ini.bak-0.178.4`).
- **Checked**: tests (new `all_unlock_replaces_the_four_debug_keys`), the Windows build. **Not checked**: in game.

---

## Stage 318 (fork branch Re-Re-Reverse/Rebirth): dead helpers removed, duplicated helpers shared (no behaviour change), version 0.178.4 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "busque outras redundância inúteis no mod para eliminar"; then "remova somente redundâncias que não vão quebrar o mod".

- **Removed** (referenced nowhere in the workspace): `attack::attack_events`, `kh1_core::enemy::script::current_motion`, `kh1_anim` `Mat::to_array`, `kh1_formats::mset` `helper_positions`, `kh1-recom-assets` `ann` `default_blend` and `matching` `body_pos_mm`, `kh2_formats_lite::mdlx` `Model::triangle_count`; the now-unused `FrameOut` import in `attack.rs`.
- **Shared**: the 20 enemy modules' test helper `inputs()` (and `enemy::clash_tests`'s) now call one `enemy::test_inputs(spec, ses_required)` (tests only, not in the DLL or the step's output); `win::battle` and `win::guard`'s identical `class_name` are one `win::stats::object_class`. `win::stats`'s own `class_name` / `class_label` differ (no null check, empty name without RTTI) and stay.
- **Kept on purpose**: unused `pub` constants that record Checked addresses or format fields (e.g. `btltbl` `GROUND_2`, `mset` `TYPE2`, `kh2_formats_lite::system` item kinds) and format readers (`objentry::parse_sklt` / `parse_wmst`, `mset` flag getters): documentation of the files, not redundancy. Per-enemy behaviour code that repeats between enemies (`frame`, `new`, `wander` in `kh1-core`): merging it is a refactor of running code, not a cleanup. `patch_container` in `camp_rows` / `shop_tabs`: same text, each on its own module's constants.
- The asset step's sources changed (test helpers, two dead getters), so its hash changes and the first start builds the files again; the output is the same.
- **Checked**: tests, the Windows build. **Not checked**: in game (nothing in it should change).

---

## Stage 317 (fork branch Re-Re-Reverse/Rebirth): Talk next to NPCs as KH1 (row 0, a dimmed Attack that buzzes), version 0.178.3 (2026-11-23)

Fork owner, 2026-11-23, answering "Perto de NPCs: (a) Attack continua normal, e o Talk aparece só na aba (como está); (b) 1:1 com o KH1: o Talk vai para a linha 0 e o Attack fica apagado, com o som de erro", verbatim: "8b".

- **KH1** (Checked (decomp), `docs/recom-m7-command-menu.md`): NPCs (kind 1) put Talk in row 0 through `0x140293000(0, Talk)`, which ungreys it; FM draws row 0 as a dimmed "Attack", its confirm gives sound 4, the context tab shows Talk.
- **Change** (`command_menu::Menu::build`, `Menu::scene`): with Talk as the context command (`recom::field_action` on an `ATEL::CAvatar`, kind 0x4000) and no special follow-up running, Talk takes row 0 (ungreyed, unlocked) instead of entry 3; the scene names row 0 "Attack" at the grey brightness; confirm on it buzzes and sends no Attack (the existing `row0_special` path); Triangle, once the tab is in, sends `Context(Talk)` as before. No new hook.
- **Assumption**: a running special's follow-up keeps row 0 over Talk (KH1's order between the two not read).
- **Checked**: tests (new `talk_takes_row_0_as_a_dimmed_attack_that_buzzes`), the Windows build. **Not checked**: in game.

---

## Stage 316 (fork branch Re-Re-Reverse/Rebirth): Donald's and Goofy's KH1 gauges without rings, version 0.178.2 (2026-11-23)

Fork owner, 2026-11-23, answering "Anéis do Donald e do Pateta (os amigos do Re:CoM não têm HP nem MP): (a) ficam sempre cheios; (b) os anéis são escondidos", verbatim: "4b".

- **KH1 / Re:CoM**: unchanged from H6 (`docs/recom-kh1-hud.md` § H6): KH1's party slots draw `g_c` objects; Re:CoM's friends have no HP or MP (Checked (decomp + strings)).
- **Change** (asset step, `kh1_hud::party::build`): the two party groups hold the face frame (`g_c_fra_a_w`), the disc (`g_c_fra_a_a`) and the face only; the HP / MP ring backs (`g_c_fra_b/c/d_*`) and the full rings (`g_c_bar_*`) are no longer built. The DLL is unchanged (it shows and hides the same two groups). The step's source hash changes, so the first start after the update builds the files again.
- **Assumption**: "os anéis" means the rings and their dark backs; the frame, disc and face stay. Picture 5 is still added to the atlas (now unused by the party).
- **Checked**: tests, the Windows build. **Not checked**: in game; the real-file HUD test (`builds_from_a_dump`) was not run (no `hud-h0` dump on this machine).

---

## Stage 315 (fork branch Re-Re-Reverse/Rebirth): the KH1 HUD hides at once when Re:CoM hides its HUD, version 0.178.1 (2026-11-23)

Fork owner, 2026-11-23, answering "Quando um evento esconde o HUD: (a) fade de 10 frames, como no KH1 (é o que está construído); (b) some na hora, como no Re:CoM", verbatim: "3b" (`docs/design-philosophy-instructions.md` § Open questions round).

- **Re:CoM** (Checked (decomp)): the gauge's HUD state setter `0x14020C190` (`40 55 57 48 83 EC 48`) is called by `0x140208360` and `0x14020A0E0` with the request at the gauge's `+0xAA`, by `0x140209440` / `0x14020A0E0` with 2 and by `0x14020BC60` with 8 / 9. Who writes 7 to `+0xAA` (an event or a menu) is not researched, so events and menus are not told apart.
- **Change** (`win::kh1_hud::Cluster::hide`): state 7 hides the KH1 cluster at once (the units dropped, the held controls hidden), no longer through `fade_out`; state 1 still plays KH1's 10-frame `fade_in`. The command menu already hid at once (`command_menu::Menu::hide`). No new hook, no asset change (`fade_out` stays in the data, unplayed).
- **Assumption**: the fork owner's "as Re:CoM" also covers menu hides (KH1 fades for its menus). Ask if a menu should still fade.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 314 (fork branch Re-Re-Reverse/Rebirth): Riku's Halloween Town costume in every world, version 0.178.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "certo faz o da Halloween town então".

- **Re:CoM** (Checked (exe + files)): Riku is char 3 (group 101, pack `PL0101`), char 4 in Halloween Town (group 111, `PL0111`); one action table for both (`0x1407C6370`); the camp / level-up menu model bytes `MENU_BYTES` (101 / 111).
- **Ours** (`riku_skin::{Look::halloween, group_for_char}`, `win::riku_skin::{halloween_unlocked, costume_open, mark_halloween}`): costume Halloween Town puts char 3 on 111 (Re:CoM's own `PL0111` name) or its skins (the Keyblade of Heart: 151; Dark Mode keeps Riku's 131 only in Halloween Town as before). Costumes' Costume row: Default Attire / Dark Mode / Halloween Town, skipping a locked one; ini `riku_costume = default | dark | halloween` (the old `riku_dark_skin = on` reads as Dark Mode). Changed in place with the crouch and the way-back effect.
- **Unlock**: Riku as char 4 once (Riku's save spare bit 0x20); debug `riku_halloween_unlocked` (on in the owner's ini).
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 313 (fork branch Re-Re-Reverse/Rebirth): the Ansem costume removed, version 0.177.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "pode tirar o ANsem das skins ficou ridículo, no recom tem algum outro modelo do Riku?".

- 0.176.0..0.176.2 reverted: Riku's costumes are Default Attire / Dark Mode again (`riku_dark_skin`), the asset step builds no Ansem groups (181 / 191 / 221 / 231), the Keyblade of Heart groups as before. Stages 310..312 stay as history.
- Re:CoM's other Riku models (Checked (files: every `PLAYER` / `ENEMY` RTM and skeleton)): only Riku (`PL0101`), his Halloween Town look (`PL0111`) and Riku-Ansem (`PL0201` / `PL0211`, `EN0116`: the Dark Mode costume). No other Riku.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 312 (fork branch Re-Re-Reverse/Rebirth): the Ansem costume's limbs posed from Riku's T-pose, version 0.176.2 (2026-11-23)

Fork owner, 2026-11-23 (three screenshots: arms out, hands at the waist, the head bent into the hair), verbatim: "ficou bem bizarro".

- **Cause** (Checked (files)): Riku's rest pose is a T-pose (hands at shoulder height, 0.61 m out); Ansem's an A-pose (hands 1.20 m up, 0.40 m out: arms about 50° down, 70° at worst). The retarget carries each bone's turn from its rest, so every Ansem limb stayed as far off as its rest differs.
- **Fix** (`riku_skin::{rest_like, align, LIMBS}`): before the retarget, each limb's rest rotation (collar, upper arm, forearm, thigh, shin, foot; the hand takes its forearm's, `wep0` its hand's) is turned by the smallest rotation to Riku's rest direction, only when a limb is 20° or more off (Dark Riku's worst is 9.9°: the Dark Mode groups 121..171 byte for byte as before, Checked (files)). Groups 181 / 191 / 221 / 231 change. **Re-run** (automatic).
- Test: in Riku's idle, Ansem's upper arm / thigh 0.0° and forearm 3.4° off Riku's directions.
- Assumptions: the twist about each limb kept as Ansem's rest has it; the head bent in a pose is Riku's own clip (Ansem's taller neck chain: 1 bone against Riku's 3).
- **Checked**: tests, the asset step, the Windows build. **Not checked**: in game.

---

## Stage 311 (fork branch Re-Re-Reverse/Rebirth): the Ansem costume is Ansem's model (0.176.0 took Axel's), version 0.176.1 (2026-11-23)

Fork owner, 2026-11-23 (screenshot: Riku as Axel), verbatim: "você colocou a skin do Axel cara!, é a do Ansem Seeker of Darkness!".

- **Cause**: 0.176.0 picked `ENEMY/0118` (`b_ex110`) by its names, never looking at its textures: it is Axel. Ansem, Seeker of Darkness is `ENEMY/0122` (textures `ex_1630_0x` = KH1's `xa_ex_1630`; silver hair, dark skin, orange eyes in `ex_1630_01`), Checked (files: the texture rendered).
- **Change** (`riku_skin::{ANSEM_ENEMY, ANSEM_RENAMES, ansem_mdl}`): `EN0122`, skeleton `e122_sk` (157 bones); its bones renamed to Riku's where the role is the same (`e122_sk` → `ex_1010_sk`, `lf_*` / `rf_*` → `l_*` / `r_*` legs, `chest2` → `chest1`, `neck` → `neck1`), so Riku's code finds them and the retarget maps them. Riku's `wep0` and weapon added as in 0.176.0. Groups unchanged (181 / 191 / 221 / 231). **Re-run** (automatic).
- Test: the idle's feet within 2.4 cm of Riku's (relative to rest), the blade root 11 cm from the hand as Riku's.
- Assumptions: Ansem's hand rest frame close to Riku's; his coat tails and hair at rest; his `chest2` stands in for Riku's `chest1`.
- **Checked**: tests, the asset step, the Windows build. **Not checked**: in game.

---

## Stage 310 (fork branch Re-Re-Reverse/Rebirth): Riku's Ansem costume (Ansem, Seeker of Darkness's model with Riku's own moves), version 0.176.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "implementa a skin do Ansem para o Riku".

- **Re:CoM** (Checked (files); wrong, see Stage 311: `EN0118` is Axel): Ansem taken as `ENEMY/0118/EN0118` (textures `b_ex110_0x`, skeleton `b_ex110_sk`, 138 bones, 52 named as Riku's), read whole by our MDL reader (2 parts); no `wep0` (his `WeaponR` in the right hand). His card is 0x74.
- **Asset step** (`riku_skin::{ansem_mdl, build_ansem}`): his model + a `wep0` under his `r_hand` at Riku's rest `wep0` relative to Riku's `r_hand`, Riku's weapon parts (PL0101 / PL0111 parts 0 / 1) moved onto it, parts ordered as Riku's, bone 1 renamed `ex_1010_sk`; Riku's clips (with his KH1 guard clips) retargeted by name, events remapped; Ansem's RTM + Riku's `weapon_0x` texture (or the Keyblade of Heart's), Ansem's TXA. Groups 181 / 191 (Soul Eater), 221 / 231 (Keyblade of Heart). **Re-run** (automatic; new outputs). Test: the idle's feet within 2.2 cm of Riku's (relative to rest), the blade root 11 cm from the hand as Riku's.
- **Game**: costumes are now 0 Default Attire / 1 Dark Mode / 2 Ansem (`Look::costume`; manifest rows carry `costume`, older rows read `dark`); ini `riku_costume = default | dark | ansem` (the old `riku_dark_skin = on` reads as Dark Mode); Costumes' Costume row cycles the three, skipping a locked one. Ansem unlocked by his card (Re:CoM's obtained-at-least-once, given at his defeat) or `riku_ansem_unlocked` (debug, on in the owner's ini). A change to or from Ansem plays the crouch and the way-back effect (Dark Mode keeps its flip).
- Assumptions: Ansem's hand rest frame close to Riku's (the blade's angle); his robe / hood / chains / hair held at rest.
- **Checked**: tests, the asset step, the Windows build. **Not checked**: in game.

---

## Stage 309 (fork branch Re-Re-Reverse/Rebirth): KH1's Auto Lock as its soft target; Re:CoM's enemy pace one attacker at a time again, version 0.175.0 (2026-11-23)

Fork owner, 2026-11-23, passing on two testers' reports, verbatim: "Sora doesn't seem to automatically lock on to an enemy when doing a combo, leading to a lot of missed swings…"; "I switched to the kh1 enemy ai where they all attack at once… I moved back to Re:Com's enemy ai but they were still attacking all at once"; and "o teu Auto Lock está diferente da implementação do KH1 lá o auto lock funciona sem lock on ao chegar proximo do oponente os ataques são direcionados a ele".

- **Auto Lock** (`win::lock_on::soft_target`, `win::attack::attack_target`): with no lock, the KH1 swing / spell target is the nearest lockable enemy by KH1's pick (distance less its lock radius) within 6 m (Assumption: KH1's reach not read), when `auto_lock` is on, Sora or Riku. 0.173.0's arming of Re:CoM's lock is removed (KH1's Auto Lock is lock state 2, a soft target; Checked (KH1 exe: `0x14029365F`)).
- **Enemy pace = Re:CoM** (`battle::wanted`, `battle::one_at_a_time`): Re:CoM's three serialisers (P2 think gate, P3 one grant a frame, P4 crowd pace) stay Re:CoM's, and the cardless play (P1) refuses an ordinary enemy while another is attacking (Re:CoM's CBTL enemy slot's rule without cards). Bosses as before. Enemy pace = KH1: all ours as before. Since M6 (0.12.0) P2 / P3 were ours whatever the pace.
- Not changed: KH1-AI enemies (their own scripts) attack as KH1's do.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 308 (fork branch Re-Re-Reverse/Rebirth): Riku's Keyblade change shows his weapon's own appear effect, version 0.174.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "no Riku na transição quando troca de keyblade deve ser o mesmo efeito quando ele está sem e vc aperta o botão de ataque e ela aparece na mão".

- **Re:CoM** (Checked (exe: disasm)): Riku's "draw the weapon" is his vtable `+0xF8` = `0x1402160A0(riku, show)`: with `+0x418 & 2` (weapon put away) it clears it and, with `show`, plays the effect `[0x1407C7C98]` on his weapon bone `+0x3A0` (`0x1401A2D50`) and fades the weapon in (`+0x154 | 0x40000000`, `+0x17C` 0, `+0x180` 1.0, `+0x188` / `+0x18C` 2000, `+0x190` 2).
- **Ours** (`win::riku_skin::weapon_appear`): a Keyblade-only change in place (0.160.0) sets `+0x418 | 2` after the model change and calls that function (was: the transformation's way-back effect).
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 307 (fork branch Re-Re-Reverse/Rebirth): KH1 FM's Auto Lock and Target Lock on the Camera page, version 0.173.0 (2026-11-23)

Fork owner, 2026-11-23 (screenshot: KH1 FM's Config, Auto Lock On / Off and Target Lock Auto / Manual framed), verbatim: "precisa implementar essas duas configs e features".

- **KH1** (Checked (KH1 exe: disasm; no decomp here)): the Config block `*(0x142E1F4D8)`: `+0` Auto Lock (getter `0x1402FAFB0`, 0 = On), `+4` Target Lock (`0x1402FAFA0`, 1 = Manual), `+8` the camera (`0x1402FAFD0`). Auto Lock On: an attack command with no lock locks its target (`0x14029365F` → `0x1402B08A0(…, 4)`); a lock whose target is gone picks the next (`0x1402B1348` → `0x1402B0A00`). Target Lock Manual: locked, R1's release unlocks (`0x1402B069C`); Auto: R1's press.
- **Ours** (`win::lock_on`, Combat = KH1 on a controller, Sora and Riku): Auto Lock (ini `auto_lock`, default on): on a frame with the Attack request and no lock, or after a lock ended without the player's unlock, Re:CoM's lock-on locks its pick (`+0x84C`) as if R1 were pressed; Target Lock (ini `target_lock = auto | manual`, default auto): Manual unlocks on R1's release. Rows on Settings > Camera.
- Assumption: Re:CoM's pick (its nearest, the cursor's) for KH1's attack target / KH1's pick.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 306 (fork branch Re-Re-Reverse/Rebirth): L2's target step written when Re:CoM's cursor doesn't take it, version 0.172.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "L2/trigger esquerdo não está alternando oponentes no modo Kh1".

- The owner's 0.171.0 log: `lock-on: L2 on 0xa3: next 0xa5` again and again: our step chose the next target, the lock stayed on 0xa3 (the history synced from Re:CoM's cursor never moved).
- Re:CoM (Checked (disasm)): its switch writes the new handle to `+0x84C`, zeroes the cursor's `+0xD4` and runs the cursor update `0x140208EC0`, whose locked branch (`0x140209038`) takes `+0x84C` only when `+0xD4` ≤ 0. Why it didn't take ours: not known.
- **Change** (`win::lock_on::stepped`): after Re:CoM's lock-on, a chosen target the cursor (`+0xD4`) or the player's handle (`+0x850`) didn't take is written to both (lock kept); each step logs what Re:CoM left. Assumption: the locked branch follows a written `+0xD4`.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 305 (fork branch Re-Re-Reverse/Rebirth): Riku's Guard blocks only real contacts, as Sora's (0.153.0's fallback removed), version 0.171.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "porra testei denovo a defesa do Riku de longe mirando no inimigo, bizarro precisa corrigir isso definitivamente! veja como está implementada a defesa no Sora como já disse a única diferença é a animação!".

- **Cause** (Checked (the owner's 0.170.0 log)): every far block came from 0.153.0's damage-check fallback (`riku_guard_block`): a hit flagged as arrived (`+0x41C & 0x200`) with value 0, from enemies 4..14 m away, the attacker id the last real hit left on his body; with that attacker in front it blocked and played the guard success each time. None came from the pair test (`riku_pair_block`, 0.159.0). What sets the flag with no hit: not known.
- **Fix**: the fallback is gone (`riku_guard_block` always false; `guard::riku_block` / `cancel_shot` removed). Riku's Guard is Sora's: Re:CoM's own block or the pair-test block of a real contact from in front; the damage check keeps the guarded hit for his KH1 frame as Sora's.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 304 (fork branch Re-Re-Reverse/Rebirth): Sora's Halloween Town costume in every world (Costumes on his lists, unlocked by reaching Halloween Town), version 0.170.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "para o Sora quero que ele também tenha trajes desbloqueáveis de acordo como a história, então se o sora for para a halloween town por exemplo, o jogador pode livremente alterar para este traje a qualquer momento"; of KH1's world costumes "Só Halloween"; "esqueci completamente que kh1 e rechain of memories o sora só troca de aparencia na halloween town... então pode usar o modelo do próprio jogo".

- **KH1** (Checked (files)): Sora's models are `xa_ex_0010` (/ `0012` / `0019` lower-resolution copies), `xa_nm_0010` (Halloween Town), `xa_lm_0010` (Atlantica, merman; not made: no legs, the owner's choice). `xa_ex_0040` / `0050` are Donald / Goofy.
- **Re:CoM** (Checked (exe + files)): Sora is char 1 (group 1, `PL0001`), char 2 in Halloween Town (group 11, `PL0011`), one action table (`0x1407C5E80`), the same 69 clip names in both packs.
- **Ours** (`win::sora_costume`): with `sora_costume = halloween` (ini; Costumes page, "Default Attire / Halloween Town") char 1 → group 11 and pack name `PL0011` (char 2's pointer), the camp's menu-model byte `0x1400E4705` 1 → 11; from the next room or battle (Re:CoM's group queue refresh). Unlocked when Sora is char 2 once (save spare bit 0x10); debug `sora_costume_unlocked`.
- **Camp**: Costumes before Settings on Sora's KH1 list (10 entries, step 21.3) and on his Combat = Re:CoM list (Re:CoM's seven, Costumes, Settings: 8).
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 303 (fork branch Re-Re-Reverse/Rebirth): Sora's combat rules for Riku, part 2: Dark Riku's Guard, KH1's attack-meets-attack, the twins' lock radius / level, the damage cap, version 0.169.0 (2026-11-23)

Fork owner, 2026-11-23: "certo mas esse é só um ponto, tem algo que ficou faltando?"; the plan (4, then 1 + 3, then 2): "sim".

- **Dark Riku's Guard**: the pair-test block (0.159.0) for the real Dark Mode too.
- **Attack meets attack** (`win::attack::riku_pair`): his swing's Re:CoM weapon volume (sphere: swept centres, radius) against the KH1 enemies' live attack rows, as Sora's blade spheres (M11, 0.82.0): KH1's clash outcome (his KH1 recoil, the enemy's), a marked swing meets no hurt body that frame. Assumption: the KH1 row's clash type / priority with the equipped Keyblade's numbers for the Soul Eater's; a non-sphere volume logs and skips.
- **Twins** (`stats::twin_any`): in his story, the KH1 twin's lock radius (KH1's lunge distance) and level (his MP charge), as Sora's; his enemies' HP and damage stay Re:CoM's.
- **Damage cap**: his swings take the enemy's KH1 per-hit cap as the share of Re:CoM's max HP (`damage_cap::Source::Recom`), at both Damage settings (his numbers are Re:CoM's card AP on Re:CoM's HP).
- Left his own: his damage numbers at Damage = KH1 (Re:CoM's), the Neoshadow's held grab (no clip).
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 302 (fork branch Re-Re-Reverse/Rebirth): the Wizard and the Aquatank stay hittable in their dimming moves (only bosses' are untouchable), version 0.168.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "eu acertar uma porrada no wizard enquanto ele casta thunder ainda não interrompe o ataque dele"; "troque essa implementação porque a jogabilidade quando estiver no KH1 deve usar as mesmas regras, com exceção a bosses de fato, um wizard não é um boss...".

- The owner's log: the Thunder was the Wizard's state 4 (the room dims, `_ex_2150` shots for 180 frames), untouchable by 0.100.1's rule ("Same rule" as the bosses), for Sora and Riku. Its other casts (states 2 / 3) were already interrupted.
- Now `revenge::install` no longer arms the Wizard's / Aquatank's windows (`install_normal` kept, not called): their dimming moves are ordinary attacks at Combat = KH1 (hittable, the card interrupt ends them). The bosses' dimming moves and revenge moves stay untouchable.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 301 (fork branch Re-Re-Reverse/Rebirth): Riku's DP only once the story gives him Dark Mode; the Keyblade of Heart unlocks with the Dark Mode costume, version 0.167.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "o Dark Mode precisa começar de acordo com o jogo normal mesmo liberando a partir da história, me equivoquei falando que o dark mode começava com 30 pq vi que era dentro do combate para ativalo!, a keyblade cosmética dele pode liberar junto com o Dark Mode também".

- **DP** (`win::dark_points::ours`): only with Riku's ability word `+0x41C & 0x4000000` (the story's Dark Mode, the bit Re:CoM's own DP rules and its Dark Riku test read; Checked (disasm `0x1401E7255`, `0x14021223D`)) or as Dark Riku. Before it, no DP and the gauge untouched, as Re:CoM. Kept from 0.166.0: the gauge fills to 30 to start it, Dark Mode starts at his DP stat.
- **Keyblade of Heart**: unlocked with the Dark Mode costume (Riku's first Dark Mode, `riku_skin::unlocked`), the 15-DP bit dropped.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 300 (fork branch Re-Re-Reverse/Rebirth): Dark Mode starts at Riku's DP stat again; his DP row is "Dark Up" (DP +2 and Ability Points +2), version 0.166.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "mas sumiu o DP"; "certo mas o DP enchendo e esvaziando tbm liga com o DP maximo oq aconteceu com ele"; chose "DP e Ability juntos".

- **Re:CoM** (Checked (disasm)): outside Dark Mode the gauge fills to a fixed 30 (`0x1401F57A4`, `0x1401E7284`); Dark Riku's switch-in sets DP = max = Riku's DP stat (`+0x450` from `stats+0x14`, `0x1401F0C38`, `0x1402048BD..0x1402048CE`): 8 at a new game, +2 a DP pick, at most 99; it drains to 0. 0.151.0's tables had dropped it (30 in Dark Mode too); 0.164.0 then took the pick for the ability points, wrongly calling it unused.
- **Now**: Dark Mode starts at his DP stat, its gauge's max (`dark_points::Dp::enter_dark`), the owner's tables drain it. His third level-up row is "Dark Up / DP +2, Ability Pts +2" (always offered; Re:CoM's +2 DP kept; the panel shows Re:CoM's DP); the result "DP and Ability Points increased by 2."
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 299 (fork branch Re-Re-Reverse/Rebirth): Riku's level-up names his ability points in full, version 0.165.1 (2026-11-23)

Fork owner, 2026-11-23 (screenshot: Riku's level-up with "AP 10" and "Max AP 3 → 5"), verbatim: "parece que ficou duplicado esse lance de AP". Not a duplicate: "AP" is Re:CoM's attack stat (his Attack Boost row), ours the ability points. On his screen ours now read "Ability Up" / "Ability Points +2", panel "Ability Pts", result "Ability Points increased by 2."; Sora's unchanged. **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 298 (fork branch Re-Re-Reverse/Rebirth): the Enemy Skills act on Riku; his items reach his KH1 frame, version 0.165.0 (2026-11-23)

Fork owner, 2026-11-23: "concordo" (the plan's step 4: the skills' effects on Riku).

- **Fix**: 0.163.0's item use never reached Riku's KH1 frame (his branch of `run_frame` read no item command): now it does, with the item events.
- **Route 1** (`win::enemy_skills::live` / `refresh`, his frame's `kh1_frame`): Re:CoM's own player writes for his equipped skills; Wight Knight's Float on his gravity; Soldier's Combo Plus and Bandit's Combo Finish in his abilities.
- **Per hit / swing / frame**: Second Chance, Jafar / Air Pirate's no-flinch (the damage check's `arrived` for `kh1_fighter`); Card Soldier's haste, Saïx's boost, the Berserk test at his swing's start; Air Soldier / Marluxia's frame step; the support rules on `mp::live`.
- Already his (per battle, his save's skills): the compare stubs (Quick Recovery, Dash, Auto-Life, Invincible, Sleightblind), the deck skills' arming, the route-2 hit rewrite.
- Not acting for Riku (no part of his): the spell / summon boosts (Red Nocturne, Blue Rhapsody, Yellow Opera, Green Requiem, Wizard, Crescendo), Luxord (Sora's blade clash), Marluxia's replay (Sora's Sleights list), Berserk's bonus on his hits (his hits are Re:CoM's weapon hits).
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 297 (fork branch Re-Re-Reverse/Rebirth): Riku's AP Up (his DP row), his Enemy Skills from his own cards, his Abilities page, version 0.164.0 (2026-11-23)

Fork owner, 2026-11-23: Enemy Skills for Riku from "as cartas que o Riku ganha durante a play dele"; the DP row as his AP Up ("concordo").

- **Re:CoM** (Checked (disasm + files)): Riku's level-up rows are HP, AP (`stats+0x10` + 1) and DP (`stats+0x14` + 2, cap 99, `0x14010F70F`), DP offered only once sleight 0x6D is learned and DP < 99 (`0x14010F2D2`). His new game is `CardInit` record 15: level 1, HP 80, DP 8, AP 10. Our DP is the owner's tables (cap 30): Re:CoM's DP stat had no use at KH1.
- **Level-up**: at Combat = KH1 in his story the DP row is "AP Up / Max AP +2", always offered; Re:CoM's +2 DP kept only where Re:CoM offers its row. Old saves: (DP − 8) ÷ 2 past picks (at most level − 1) count as AP Ups, once.
- **Abilities**: his state in his story's save (`abilities::sync` on `mp::live`), only the Enemy Skills (card sources; no KH1 level grants, Re:CoM movement or sleight abilities); learned from his cards' first pickup / Re:CoM's discovery state, with the learned notices at his field camp gate.
- **Camp**: his list is Items, Abilities, Customize, World Map .. D Report, Costumes, Settings (10, step 21.3; Assumption: reads well); the Abilities page is Sora's, owner "Riku".
- Not yet: the skills' effects on Riku (next).
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 296 (fork branch Re-Re-Reverse/Rebirth): KH1's items for Riku (drops, Items list, use, camp Items page with his Keyblade slot, Moogle shop), version 0.163.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "inclusive as features implementadas para o Sora até então, - Fully working MP system, item drops, and moogle shops …"; "vi agora que com o Sora já tem a feature de escolher a keyblade, para não ficar redundante aplique a mesma forma de implementação para o Riku e pode iniciar o fluxo proposto"; item motion: "Uma animação dele".

- **Store**: Sora's item block in Riku's story's save, as his MP (`items::with_inventory` on `mp::live`).
- **Drops**: KH1's tables in Riku's story too (`stats::drops`; his enemies keep Re:CoM's HP / damage).
- **Battle**: his command menu's Items row with his slots; a use runs Sora's KH1 path (the use task in `riku_request_bits`, `exe_item` for `kh1_fighter`). KH1's item motion 0x3E plays his row 0x2D (`p002_034`, 56 frames at 30 fps; Re:CoM's Riku never uses it; Assumption: his own "use" motion, the owner's choice). The heal at KH1's frame.
- **Camp** (Combat = KH1): his list is Items (Review Decks' place), Customize, World Map .. D Report, Costumes, Settings. The Items page is Sora's, owner row "Riku"; his Keyblade slot opens his Keyblade list (Soul Eater / Keyblade of Heart, greyed until 15 DP), as Sora's Keyblade list. The Costumes page's Keyblade row (0.160.0) is gone.
- **Moogle shop**: Sora's KH1 shop where Riku's story has one, lines naming Riku.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 295 (fork branch Re-Re-Reverse/Rebirth): Sora's combat rules for Riku (post-hit invulnerability, KH1's no-flinch, no card duel, the dimming sleight's shield, gimmick cards, R1 lock-on), version 0.162.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "precisa valer, alias fora as específicidades do Riku todas as regras de combate precisam ser iguais as do Sora, porque na verdade não muda mesmo, são os mesmos inimigos, o criador do mod só quis desabilitar a parte do Riku para implementar depois".

- Audit of every Sora-only gate in the combat code (Checked (code)); `mv::kh1_fighter` (Sora or Riku with KH1 movement) now for:
  - the damage check: KH1's post-hit invulnerability (30 frames, decision 31), KH1's no-flinch frames, no card duel (decision 33), a dimming sleight's shield;
  - gimmick cards (drop held, "Stagger" in his context tab, Guard break, finisher break, 50 % HP drop);
  - KH1's lock-on rules on R1 (controller).
- Already shared before (Checked (code)): KH1 battles (M6, cardless enemies, pace), KH1 enemy AI, enemies hittable while attacking, the swing's card interrupt, revenge value, the dim rules, Guard reactions, cooldowns.
- Left Riku's own (his specifics): his damage (Soul Eater's card AP at both Damage settings, Re:CoM HP), Sora's abilities / spells / items / summons / Enemy Skills, the Neoshadow's held grab (no clip of his).
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 294 (fork branch Re-Re-Reverse/Rebirth): a shot the Guard turned back interrupts its caster (Sora and Riku), version 0.161.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "preciso de uma revisão e balanceamento do combate com o riku, quando alguns oponentes estão castando magia o ataque não interrompe, verifique se esse tipo de coisa já é tratado na gameplay com o Sora nesse mod".

- **Found** (Checked (the owner's 0.160.0 log + disasm)): every turned-back shot met its Wizard with "damage 0 … card playing". Decision 28's card-interrupt test read the enemy's hit info `+0xF8`, which Re:CoM's gather `0x140154EB0` fills inside the handler (`0x14017F81F`), so before the handler it never named the shot. Since 0.11.2, for Sora too.
- **Fix**: the attacker is read from the waiting hurt body (`+0x98`), as Riku's swings are.
- **Review** (Checked (code + log)): Riku's swings get Sora's rules already (enemies stay hittable while attacking, `hurt_off` with `riku_on`; his hits carry the card-interrupt bit, `riku_swing_hit`). The deliberate exceptions stand for both: the Wizard's state 4 / the Aquatank's state 3 (the room dims: untouchable, 0.100.1), the bosses' dimming moves, revenge moves, KH1 enemies' own no-reaction hits.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 293 (fork branch Re-Re-Reverse/Rebirth): Riku's Keyblade on the Costumes page (Soul Eater / Keyblade of Heart, cosmetic), unlocked at 15 DP, version 0.160.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "quero implementar para o Riku a troca de Keyblade, vai ser simplesmente cosmética, quero que inicialmente ele possa trocar entre a Soul Eater e a Keyblade of Heart, vai ser uma nova entrada no menu dentro de Costumes, a Keyblade of Heart você pode pegar no KH1 acho, e ela é liberada com 15 de DP, mas inicialmente deixa ela desbloqueada para eu testar"; "no Kingdom Hearts 1 essa é a única keyblade que ele utiliza, numa bossfight inclusive com o mesmo traje do DarkMode".

- **KH1** (Checked (files)): Riku-Ansem's `xa_ex_1580` (voices `rk04`) holds the Keyblade of People's Hearts as its HD mesh `-c0.cvbl` mesh 0 on joint 211, texture `-1c0.dds`.
- **Re:CoM** (Checked (files)): Riku's weapon is parts 0 / 1 of his model on `wep0` (77; Dark Riku's 105), in KH1 joint space × 0.01. Joint 211 → 191: (4.54, −0.52, −0.02) cm, from the Soul Eater held by `xa_ex_1160` (same skeleton as 1580) against `xa_ex_1560`'s (Assumption: the same grip).
- **Asset step** (`riku_skin::{with_blade, build_blade}`, `keyblade::blade_parts`, `mdl::split_parts`): groups 141 / 151 (Default Attire, from 101 / 111) and 161 / 171 (Dark Mode, from 121 / 131, the blade turned over as `BLADE_FLIP`) with the weapon parts replaced (the body kept byte for byte) and the texture added to the RTM. **Re-run** (automatic; new outputs).
- **Game**: the Costumes page gets a second row, **Keyblade: Soul Eater / Keyblade of Heart** (ini `riku_keyblade = soul_eater | heart`). The group comes from the attire × Keyblade; a Keyblade-only change swaps the model at once with Riku's way-back effect. Unlock: Riku's DP reaching 15 once (save spare bit 8); debug `riku_keyblade_unlocked` (on in the owner's ini for the test).
- **Checked**: tests (`riku_skin::tests::the_keyblade_replaces_rikus_weapon_parts`, the look selection), the asset step, the Windows build. **Not checked**: in game.

---

## Stage 292 (fork branch Re-Re-Reverse/Rebirth): Riku's Guard works as Sora's KH1 Guard (blocks at the pair test, turns shots back, Counterattack, the guard up on the live frames), version 0.159.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "a mecânica de guarda precisa funcionar igual a do Kingdom Hearts 1"; "o mod com a campanha do Sora já deve ter essas implementações não? não da para se basear?" (chose: hits get through, magic should come back, timing odd, no Counterattack).

- **Block** (`win::attack::riku_pair_block`): Re:CoM's body guard never answers 2 for Riku (why: not known; the log showed every hit falling to the damage-check fallback). Now a hit on him at the pair test while his KH1 guard is live, from in front (±100°; a shot within 1 m judged by its caster), becomes Re:CoM's block as Sora gets it: result 2 + the guarded bit. Sora's path does the rest. Not for the real Dark Riku.
- **Shots**: turned back as Sora's (0.153.0's cancel removed).
- **Counterattack**: Riku has KH1's Counterattack (Sora's offer and input); it plays his ground finisher's row (`riku::swing_row`, Assumption: it reads as the counter).
- **Timing**: the guard clip is 240 in 10 frames, 241 held over KH1's live window 10..36, the way out in 17 (54 total). Asset step: **re-run** (automatic).
- **Checked**: tests, the asset step, the Windows build. **Not checked**: in game.

---

## Stage 291 (fork branch Re-Re-Reverse/Rebirth): no melee block reaction on a far attacker; KH1 Riku's own guard recoil, version 0.158.2 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "eu apertei a defesa bem longe desse oponente, e parece que deu block naquela distância … prefiro que use a versão do KH1 da animação de quando toma impacto".

- The owner's log: the far Defender's fireball / flame (Re:CoM bullets naming it) was blocked as "melee", flinching it each time. An attacker that is no shot and is past 4 m is now blocked without a reaction on it.
- KH1 Riku's request 110 (his guard recoil) is converted as `p002_226` on row 0x3A (renamed like 0x39); KH1's guard recoils 0x6E / 0x6F play it. Asset step: **re-run** (automatic).
- **Checked**: tests, the asset step, the Windows build. **Not checked**: in game.

---

## Stage 290 (fork branch Re-Re-Reverse/Rebirth): Riku's Guard ends by leaving the guard (the way in backwards), version 0.158.1 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "quando passa um tempo e ele desativa a guarda ficou estranha, sera que nn tem como colocar a mesma animação mas ao contrário?". Riku's 54-frame guard clip is now 240 (30 frames), 241 held (7), then 240 backwards in 17 frames, ending on his pre-guard stance. Asset step: **re-run** (automatic). **Checked**: tests, the asset step, the Windows build. **Not checked**: in game.

---

## Stage 289 (fork branch Re-Re-Reverse/Rebirth): Riku's Guard is KH1 Riku's own guard (Destiny Islands duel), version 0.158.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "no KH1 tem uma animação do Riku defendendo, ela acontece na Destiny Island … é possível usar essa animação? inclusive tem a variação de quando em defesa ele toma impacto".

- KH1's `xa_ex_1010` script guards with request 240 (into the guard) then 241 (held), with guard row `0x550`. Its recoil, the impact while guarding, is request 110 = Re:CoM's `p002_043`, already his block recoil (Checked (files)).
- The asset step converts 240 + 241's first 24 frames (54) onto Riku's skeleton (KH1's own) as `p002_225` (replacing 0.157.0's retargeted Sora Guard). **Re-run**, automatic.
- **Checked**: tests, the asset step, the Windows build. **Not checked**: in game.

---

## Stage 288 (fork branch Re-Re-Reverse/Rebirth): Riku's Guard row named once per action table, version 0.157.1 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "não percebi alteração...". Chars 3 and 4 share one action table. 0.157.0 renamed row 0x39 for char 3, then refused for char 4, as the owner's log shows. Now each distinct table is named once. **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 287 (fork branch Re-Re-Reverse/Rebirth): Riku's Guard plays Sora's KH1 Guard retargeted onto his skeleton, version 0.157.0 (2026-11-23)

Fork owner, 2026-11-23, verbatim: "a animação escolhida de defesa pro Riku ficou estranha, tem alguma melhor?" (chose "Guard do KH1 adaptada").

- Riku has no guard clip in Re:CoM: rows 0x24 / 0x25 are both `p002_043`, the clash recoil (Checked (exe)).
- **Asset step**: Sora's KH1 Guard clip (`p001_225`) is retargeted onto Riku's skeleton by bone name; his extra bones (blade chain, belts, hair) hold his stance `p002_031`. It is added as `p002_225` to `PL0101.ANN` / `PL0111.ANN` and the skin packs (**re-run**, automatic; 2 new outputs).
- **Game**: row 0x39 of Riku's action tables (`p002_000`, checked) is renamed `p002_225`; his Guard plays it. The recoil stays 0x24; the real Dark Mode is unchanged.
- **Checked**: tests, the asset step (the clip in all four packs), the Windows build. **Not checked**: in game.

---

## Stage 286 (fork branch Re-Re-Reverse/Rebirth): Costumes on Riku's list at every Combat setting, version 0.156.2 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "notei que, quando troca o combat de KH1 para Re:CoM a opção de Costume some... essa opção poderia manter no menu sempre".

- Riku's story at Combat = Re:CoM: Re:CoM's seven where they are (Review Decks Re:CoM's own) plus Costumes before Settings. With `riku_kh1_movement` (`Layout::RikuDecks`), Settings opens the short list; without it (`Layout::RikuPlain`), Settings is Re:CoM's page. Combat = KH1 is unchanged.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 285 (fork branch Re-Re-Reverse/Rebirth): debug unlock for the Dark Mode costume; Riku stops while the transformation plays, version 0.156.1 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "deixa no ini do mod uma opção de debug pra deixar desbloqueado o modo desde o início para testes, além disso peguei um bug, quando vc está andando e troca a skin a animação fica tocando enquanto o personagem está se movimentando".

- Ini `riku_dark_skin_unlocked = on | off` (debug, default off): the costume unlocked from the start.
- The transformation zeroes Riku's horizontal velocity (`mover+0x80`) at its start and on every held frame (the held move exe left Re:CoM's mover sliding him).
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 284 (fork branch Re-Re-Reverse/Rebirth): the Dark Mode costume unlocks at Riku's first Dark Mode, version 0.156.0 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "beleza já implementa essa pendência".

- **Unlock**: the save's spare bit `DARK_SEEN` (0.151.0), now also set from any frame of Dark Riku as the main player, at either Combat setting (`win::riku_skin::frame`).
- **Costume**: decided at each room / battle build, before Re:CoM's group queue (`win::riku_skin::refresh` from `win::models::group_load`). Without the unlock it is Default Attire (the setting goes back to off, logged).
- **Costumes page**: Dark Mode is refused before the unlock (sound 5, its own help line). Default `riku_dark_skin = off`.
- **Checked**: tests, the Windows build. **Not checked**: in game. Details: `docs/recom-riku-kh1.md` § 0.156.0.

---

## Stage 283 (fork branch Re-Re-Reverse/Rebirth): the costume change plays Dark Riku's entry flip and its effects; Riku's way-back effect, version 0.155.3 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "funcionou!, mas a animação ficou incompleta, não deu tempo dele dar aquele mortal, também pode ter aquele efeito de tela junto, igual a transição do combate".

- **Re:CoM** (Checked (exe + files)): the transformation is Riku's row 0x37 `p002_057` (crouch), then Dark Riku's own row 0x37 `p003_067` (the flip, `ex_1580` / `transform` effects). The way back spawns `"transform"` effect 2 on Riku's bone 0x1B.
- **Asset step**: the skin's pack and events get Dark Riku's `p003_067` (**re-run**, automatic).
- **Game**: to Dark Mode, crouch → model change → the flip (row 0x37 pointed at its slot, rebound after); to Default Attire, crouch → model change → the `"transform"` effect. The switch-in's camera mode 4 is not carried.
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 282 (fork branch Re-Re-Reverse/Rebirth): the costume change plays Riku's transformation, version 0.155.2 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "funcionou perfeitamente!, só que eu realmente queria a transição quando altera-se a skin de uma para outra, a animação e o efeito".

- Re:CoM's transformation (Checked (exe: disasm)): action 0x20 (`0x140215380`) plays row 0x37 (`p002_057`, blend −1) with its EET / SES events and switches at the motion's end (`0x1401A57D0`).
- Ours: the costume change plays the same row and events, holds Riku's move exe while it plays, and changes the model in place at its end (≤ 240 frames). Re:CoM's action 0x20 is not entered (its end switches to the battle-only Dark Riku).
- **Checked**: tests, the Windows build. **Not checked**: in game. Details: `docs/recom-riku-kh1.md` § 0.155.2.

---

## Stage 281 (fork branch Re-Re-Reverse/Rebirth): the in-place costume change sizes the transform node again, version 0.155.1 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "é bugou mesmo, mas no próprio jogo em teoria já existe uma transição... quando o personagem usa a transformação o próprio jogo recarrega a forma não seria possível utilar essa mesma transição?"

- **Cause** (Checked (the owner's log + exe)): the entity's transform node is sized once (`0x14023EBE0` allocates only at count 0), and the motion controller takes its bone count from it. The log showed 102 kept for Dark Riku's 128 bones and 128 kept for Riku's 102: the T-pose and the garbled model.
- **Fix**: before the rebuild, the node's array is released as the node's dtor does (`0x14023DE20`: deferred free `0x14028F800` through a deleter, vtable `0x140605B38`), so the bind allocates it at the new count. The weapon's node gets the same before its attach.
- **The transformation** (Checked (exe)): a second, battle-only player (`CPlayerDRiku`) swapped in by the main player pointer. Nothing is reloaded, so it can't serve a field costume.
- **HUD face**: diagnostic log of the face layer (layer count, set).
- **Checked**: tests, the Windows build. **Not checked**: in game.

---

## Stage 280 (fork branch Re-Re-Reverse/Rebirth): the costume changed in place (no door), the HUD face follows it; Halloween Town confirmed, version 0.155.0 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "importante que quando essa skin estiver ativa na halloween town … essa skin fica com a variação da halloween town, além disso... seria possível trocar a skin sem ter que passar por uma porta?" (chose "Trocar no lugar"); "quando trocar a skin para a darkmode também deve alterar a foto no hud".

- **Halloween Town**: skin group 131 is `PL0211` (Dark Riku's Halloween textures), Riku's group 111 there (Checked (files)). Already so.
- **In place** (`win::riku_skin`): the model part is released as the entity dtor `0x1401A2440` does and rebuilt as the model init `0x1401A4E70` and the player state init `0x1401FA6E0` build it. That covers parts, matrices, sub-models, motion controller, materials, texture animation, EET / SES players, step-effect defaults, the clips bound by name, the bones by name, the weapon re-attached, and the idle row. Each step and address: Checked (exe: disasm). It runs at the camp's close, or at Riku's frame once the group is loaded; only plain Riku in the field, standing / walking / running. 21 callees' first bytes are checked.
- **HUD face**: Re:CoM's own Dark Mode gauge switch's two face steps (`0x14020BC60`): set `+0x88` to `"gauge:02"`, then the face layer 4 replaced with group 3 / 0. They run when the costume is Dark Mode, never in the Dark gauge state.
- **Assumptions**: no live pointer into the old model at the change; the node resized by the parts' bind (logged).
- **Checked**: tests, the Windows build. **Not checked**: in game. Asset step: unchanged. Details: `docs/recom-riku-kh1.md` § 0.155.0.

---

## Stage 279 (fork branch Re-Re-Reverse/Rebirth): Dark Mode skin hides the Soul Eater where Riku does (the menu), version 0.154.4 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "ficou bom mas no menu não precisa ele segurar a espada na mão, além disso no menu ela ficou invertida, mas o modelo na gameplay ficou perfeito agora".

- **Cause** (Checked (files)): Riku's 4 scaled clips hide the blade (`wep0`, `k1`..`k9` scale 0; 54 frames). The retarget dropped scale channels.
- **Fix**: Riku's local scale is copied bone for bone (`riku_skin::retarget`, 9-channel clips where he has them).
- **Checked**: test (Dark's `wep0` scale = Riku's on every frame), the Windows build. **Not checked**: in game. Asset step: **re-run** (automatic).

---

## Stage 278 (fork branch Re-Re-Reverse/Rebirth): the Dark blade turned over to Riku's grip; the menu's model shows the costume, version 0.154.3 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "estranho, ainda etá invertida … ao entrar no Menu quero que a skin também seja alterada".

- **Blade** (Checked (files)): both blade meshes hang on `wep0` with the same rest pose. Dark Riku's mesh is modelled turned over about the blade axis, and his own clips turn it back (137..178° in 12 clips); 0.154.2's chain fix drew the same as before. Now Dark's `wep0` = Riku's grip turned half a turn about its x (`riku_skin::BLADE_FLIP`), and the chain `k9`..`k1` stays where Riku's is.
  - Test: every frame of the 51 clips; 0.00° from Riku's grip turned over, 2 cm.
- **Menu**: `CMenuModel`'s group comes from immediates (camp `0x1400E4701`, level-up `0x14010EF68`; Checked (exe)). Riku's 101 / 111 there follow the loaded group (`riku_skin::sync_menu` from the group-load wrapper).
- **Checked**: tests, the Windows build. **Not checked**: in game. Asset step: **re-run** (automatic). Details: `docs/recom-riku-kh1.md` § 0.154.3.

---

## Stage 277 (fork branch Re-Re-Reverse/Rebirth): Dark Mode skin's Soul Eater held the right way round, version 0.154.2 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "por algum motivo a keyblade do Riku ficou invertida mas o modelo e a gameplay ficaram perfeitas"; "mas ela só inverteu na Skin Dark Mode".

- **Cause** (Checked (files)): Riku's `k11` / `k10` and Dark Riku's `wep2` / `wep1` are different hand → blade chains (17 cm along −z against 22 cm along +z); mapped bone to bone, they turned the blade over. `wep0` sits the same in both hands at rest.
- **Fix** (`kh1_recom_assets::riku_skin::Grip`): Dark's `wep0` takes Riku's pose in the hand; `wep2` / `wep1` follow it as in Dark's rest; the blade segments follow `wep0`.
- **Checked**: test (every frame of the 51 clips: blade root and tip in the hand within 2 cm of Riku's), the Windows build. **Not checked**: in game. Asset step: **re-run** (automatic).

---

## Stage 276 (fork branch Re-Re-Reverse/Rebirth): the skin's T-pose fixed, Costumes on Riku's top list (Default Attire / Dark Mode), version 0.154.1 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "a skin foi implementada mas ficou um Tpose, lembrando que é não é pra trocar as animações... além disso … é pra colocar uma nova opção no menu chamada Costumes onde ativa essa skin, chame ela de Dark Mode, e a normal se chamará Default Attire".

- **T-pose cause** (Checked (exe + the owner's log)): the player's clips are found by name in the `.ANN` resource named by `0x1407C6338[char]` ("PL0101" for Riku; readers `0x1401FA7F6`, `0x1401F425C` → `0x1402A42B0`). 0.154.0 changed the group, not this name.
- **Fix**: the names of chars 3 / 4 (`0x1407C6350` / `58`, the only pointers to them; Checked (exe)) are switched with the groups, to `PL0121` / `PL0131`. Both writes are checked.
- **Moves**: Riku's own clips (retargeted), as the owner asked.
- **Costumes**:
  - Riku's top list is now eight: Customize, World Map … D Report, **Costumes**, Settings (`top::RIKU_ORDER`). It runs on the same reordered-list code as Sora's KH1 list (`top::order` / `extras` / `list_fix`).
  - Costumes opens a settings page with one row, **Costume: Default Attire / Dark Mode** (ini `riku_dark_skin`), switched live (`win::riku_skin::apply`) and shown from the next room or battle.
- **Assumption**: the other group stays loaded after a switch until Re:CoM unloads it.
- **Checked**: tests, the Windows build. **Not checked**: in game. Asset step: unchanged. Details: `docs/recom-riku-kh1.md` § 0.154.1.

---

## Stage 275 (fork branch Re-Re-Reverse/Rebirth): Riku's Dark Form skin, Dark Riku's model with Riku's own moves, version 0.154.0 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "ele vai desbloquear a skin do Dark Mode e vai poder jogar com ela no modo sem estar com darkmode ativado, testa com ela liberada já de começo e depois a gente bota o requisito do jogo pra liberar ela"; "pode colocar a skin mas os movimentos ser o normal mesmo".

- **Way round 0.152.0's blocker**: the Dark mesh is not rebound. Riku's 51 clips are retargeted onto Dark Riku's 128-bone skeleton by world rotation (`kh1_recom_assets::riku_skin`). Bone names match; rest axes differ. Checked (files).
- **Events**: Riku's `.SES` / `.EET` bone indices are remapped by name.
- **Bone 1**: renamed `ex_1010_sk`, because Riku's code looks his bones up by name (`0x1401A4450`, strings `0x1407C5E10..`). Checked (exe).
- **Asset step** (**re-run**: automatic, the step's hash changed): new player groups 121 / 131 (`PLAYER/01x1/PL01x1.{MDL,ANN,EET,SES}`, `raw/…{RTM,TXA}`), plus `kh1-riku-skin.json`. Placeholders are made for the new names.
- **DLL** (`crate::riku_skin`, `win::riku_skin`):
  - the skin groups' rows copy Riku's in the PLAYER table copy (`win::models`);
  - with `riku_dark_skin = on` (ini, default on; unlocked from the start, as the owner asked for the test), Re:CoM's char → group entries for chars 3 / 4 (`0x1405A6910 + 12`, checked 101 / 111) become 121 / 131. Checked (exe: every reader of the table).
  - Riku's weapon takes no group. Dark Mode is unchanged.
- **Not yet**: the unlock rule and a menu switch.
- **Assumptions**: Dark Riku and the skin share texture names in Dark Mode; no Re:CoM Riku code reads a bone by index; the coat tails at rest.
- **Checked**: tests, the asset step on the local install, the Windows build. **Not checked**: in game. Details: `docs/recom-riku-kh1.md` § 0.154.0.

---

## Stage 274 (fork branch Re-Re-Reverse/Rebirth): Riku's Guard runs KH1's frame (his own action-6 exes wrapped), version 0.153.1 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "BRO, still not effective!!! … i tried rikus guard against Heartless Wizard thunder and ice magic"; "also parry should be effective just like KH1 Sora with this mod bro!".

- **Cause** (Checked (the owner's 0.153.0 log: no `riku guard:` line, no guard success; exe: `CPlayerRiku` / `CPlayerDRiku` vtable `+0x1C8`)): KH1's Guard and its recoil put Riku in Re:CoM's action 6, whose exe is his own (`0x140212350`, Dark Riku `0x1401FFC30`), not Sora's `0x140217310`, the only one wrapped. So the KH1 frame never ran while he guarded: no guard object, no block, no parry.
- **Fix**: two new hooks (`sites::MOVE_HOOKS` "CPlayerRiku / CPlayerDRiku avoid exe (action 6)", first 16 bytes = Sora's, Checked (exe)) with Sora's rule: his Re:CoM roll keeps his exe whole; the Guard / recoil run the KH1 frame (`win::movement::avoid_kh1`). His parry is Sora's kind-6 guard success (recoil, clink, the attacker's reaction, a shot cancelled, the DP parries); no Counterattack (no clip of his).
- A shot within 1 m of him (the Wizard's spells appear near the player, a thunder from above): the front test uses its caster's position.
- Checked: tests, the Windows build, the DLL copied into the owner's OpenKH folders. Not checked: in game.

---

## Stage 273 (fork branch Re-Re-Reverse/Rebirth): Riku's Guard really blocks (KH1's front test after Re:CoM's) and cancels magic shots, version 0.153.0 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "a defes do Riku deve poder fazer parry e cancelar o ataque do oponente caso for uma magia!"; "percebi que a defesa dele não está sendo efetiva contra nada, precisa ser efetiva tal qual o KH1 é".

- **Found** (Checked (the owner's 0.152.0 log)): his Guards started, Re:CoM's body guard (`0x14016F1C0`) never blocked (no block line, no guard success, only missed parries). Its rule read from the exe (Checked (exe: disasm)); which part refuses on Riku: not known (no decomp here); the first 40 guarded hits log its fields (`riku guard:`). His Guard clip `p002_043` is 56 game frames, so KH1's live frames 10..36 run (Checked (files)).
- **Change** (`win::attack::riku_guard_block`, `crate::riku::guard_faces`, `win::guard::{riku_block, cancel_shot}`): in the damage-check wrapper, a hit on Riku / Dark Riku while his KH1 guard is live that Re:CoM didn't block is KH1's block when its attacker is within ±100° of his facing (Re:CoM's own angle rule): Re:CoM's protection bit for the check, KH1's guard success (recoil, clink, parries), the melee attacker's block reaction.
- **Magic**: a shot his Guard blocks is cancelled (attack off `0x14014DE80`, life `+0x160` = 0), KH1's inert blocked shot; Sora's still turn back.
- **Thunder** (owner, verbatim: "remebering, that it should be like the kh1 guard, if an enemy cast like thunder, then it should be effective"): the KH1 Yellow Opera's bolt, which skips Sora's Guard (KH1's Thunder has no clash pass, Stage 178), meets Riku's Guard (the owner's rule, not KH1's). Re:CoM's own thunder shots are bullets: blocked and cancelled as above; one straight down counts as in front.
- No new hook or site; per hit only. Assumptions: the attacker's position for KH1's hit point; a plain `CBullet`'s life at `+0x160`; the caster isn't interrupted. Checked: tests (`riku::tests::rikus_guard_faces_the_front`), the Windows build. Not checked: in game. Details: `docs/recom-riku-kh1.md` § 0.153.0.

---

## Stage 272 (fork branch Re-Re-Reverse/Rebirth): Dark Riku with KH1 combat, DP on Re:CoM's Dark gauge, Riku's Guard and parries, version 0.152.0 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "implementa tudo pra eu testar de uma vez e faz push no final".

- **Dark Riku** (`CPlayerDRiku`, `docs/recom-riku-kh1.md` § Research): `win::movement::is_main_riku` covers him (`is_main_dark_riku` tells them apart). His Battle exe is the shared one, and actions 2..4 / 7 call the shared move / jump exes (Checked (exe: the jump table at `0x1401FA050`)), so KH1's movement runs on him as on Riku. His data: rows with clips (`riku::DARK_NO_CLIP`), swing ids 0x9E..0xA4 (`attack_id_for`), his row-pick slot `0x1405A5210` (old `0x140204F80`) → `win::attack::dark_picker` (falls back to his own), his sleight start `0x140206830` (takes 0x5F..0x63, 0x65..0x6B: its jump table `0x140206894`, Checked (exe)) and his Sleights list `riku::DARK_SLEIGHTS` (Dark Break, Dark Firaga, Dark Aura, Dark Impulse / Maelstrom / Barrage; MP ours). His roll is Re:CoM's own (his roll exe).
- **DP on Re:CoM's gauge** (Checked (exe: disasm)): Re:CoM's DP are the player's `+0x44C` (cap 30, `0x1401F57A4`), the gauge `+0x3E0`, its setter `0x14020BFA0(gauge, dp, max)` (first 10 bytes checked). Ours are written there and shown. Re:CoM's own DP rules in its damage check (`0x1401E7255`: +1 on a hit, −5 in Dark Mode, the way-back flags `+0x424 & 0x100000`, card battle `+0x208 & 0x400000`) are undone around the check: only the owner's tables move them.
- **Riku's Guard** (owner: "falta a defesa"): KH1's Guard rule for Riku and Dark Riku on their block-recoil row 0x24 (`riku::GUARD_ROW`): KH1's Riku has no Guard request (212); his request 110 is that recoil (Checked (files: `xa_ex_1010.mset`'s request table)). Enemies react to his blocks (`win::guard`, as Sora's). **Parries**: a block within 20 frames of the Guard's start = perfect (+5 / +3 in Dark Mode), a Guard that blocks nothing in 60 frames = missed (−3).
- Customize: his picker lists Riku's and Dark Riku's sleights.
- **Not built**: the Dark Form skin. `PL0201.MDL`'s mesh packets are a format our MDL reader refuses (header at `0x2A50`: a vertex count 45 for 49 vertices and 2 quadwords a vertex in a textured packet: multi-bone vertices, Assumption); without it the mesh can't be rebound to Riku's skeleton. Its unlock is recorded (save spare bit 4).
- Asset step: **re-run** (Dark Riku's three new sleight names). Checked: tests, the asset step on the local install, the Windows build. Not checked: in game.

---

## Stage 271 (fork branch Re-Re-Reverse/Rebirth): Riku's Dark Points (the owner's tables) start and end Re:CoM's own Dark Mode, version 0.151.0 (2026-11-22)

Fork owner, 2026-11-22: the DP gain / loss tables (outside and in Dark Mode), "go", "faz ai".

- Re:CoM (Checked (exe: disasm), `docs/recom-riku-kh1.md` § Research): Dark Mode is a second player, `CPlayerDRiku` (vtable `0x1405A50E8`, group 201), made with Riku's battle player when `+0x41C & 0x4000000`, linked both ways at `+0x8A0`. Riku's `+0x198` `0x140215FB0(riku, 1)` = action 0x20, his transformation (exe `+0x1A0` `0x140215380` → `0x140206810`); Dark Riku's `0x1402046D0(dark, 1)` writes him to `0x14087B380`, `(dark, 0)` = his way back (action 0x20 sub-state 4).
- Ours (`crate::dark_points`, `win::dark_points`): DP 0..30 per battle, the owner's two tables (definitions in the module: critical / heavy = Re:CoM's knock-away kind 2, special = a hit from a `CBoss…`, one count per hit taken that took HP, one per swing that landed, finisher = attack ids 4 / 7 / 0x9A / 0x9D / 0xA1 / 0xA4, idle 5 s then every 3 s, in Dark Mode every 5 s, missed dodge 0.5 s after a roll, streak 5). At 30, Re:CoM's `+0x198(riku, 1)` when he stands / walks / runs; at 0 in Dark Mode, `+0x198(dark, 0)`. Both functions' first 10 bytes checked at first use. No Dark Riku in the battle (Dark Mode not unlocked in the story): DP stay at 30 (logged). First Dark Mode kept in the save block's spare byte bit 4 (the skin's unlock).
- Hooks used (no new site): Riku's / Dark Riku's request bits, the enemy hit handler, the damage check, the defeat prize, the battle manager ctor.
- Not yet: parries (no defense), the Dark gauge showing DP (the log shows them: `dark points:` lines), KH1 gameplay on Dark Riku (Re:CoM's own until the next stage), the skin.
- Assumptions: Re:CoM's request bits / damage check run for Dark Riku (shared Battle exe); calling the switches from the request bits is safe (Re:CoM calls `+0x198` from its exes).
- Checked: tests (`dark_points::tests`), the Windows build. Not checked: in game.

---

## Stage 270 (fork branch Re-Re-Reverse/Rebirth): Riku's camp list at Combat = KH1 (Customize in Review Decks' place) and the King's friend sleights in his Party list, version 0.150.0 (2026-11-22)

Fork owner, 2026-11-22 (screenshot: Riku's camp with Review Decks), verbatim: "se não há mais cartas ent nn tem mais pq usar o review deck, igual a gameplay do Sora, certo? , minha sugestão pra cura, igual no caso do donald e do goofy são tratados na gameplay do Sora … poderia substituir o Review Decks quando a gameplay do KH1 estiver ativa, IGUAL a do Sora … o que tiver na gameplay do Sora tem que ter o equivalente adaptado a do Riku só preenchendo os Caveats".

- Camp (`keyblades::top::Layout::Riku`, `riku_fix`; `win::keyblades`): in Riku's story at Combat = KH1 with `riku_kh1_movement`, Re:CoM's entry 0 (Review Decks) is relabelled Customize and opens his Customize page (0.149.0's), through the KH1 list's own confirm path (Re:CoM's confirm skipped, state 0x11 → the Config states); Re:CoM's other six stay where they are; Settings opens Game Settings, Combat, Movement, Camera (Customize left the Settings list). At Combat = Re:CoM his list is 0.147.1's.
- Party (`crate::party::ALL` + Mickey Miracle 0x63 (MM Miracle LV3), Holy Burst 0x64; `win::party::learned`, `win::command_menu::riku_world`, `win::summons::begin`): as Donald / Goofy for Sora, each shown on page 2's Party list in a battle where Re:CoM's Friend-card roll can give the King's card (`0x1401602A0`'s Riku rows 14..25: only card 0x38, Checked (exe, the checked table)) and the King is met (int var (0, 0x22) bit 0x80); counted as taught in his story; started by Riku's own sleight start (`0x140216AB0` takes 0x62..=0x64, Checked (exe)). MP ours: 3 / 2. Usable as shortcuts (his Customize picker lists them). Sora's story: never (no King card in his rows; test).
- Asset step: the two names in `PARTY_WORDS` (re-run, automatic).
- Assumptions: MM Miracle heals (not read); the King's met bit is set in Riku's story; his page byte is per session (the summon block stays Sora's).
- Caveats for parity, not built: Items (Riku has no item motion row known; Re:CoM's Riku uses no items), Abilities / AP Up, Summon row (greyed: no summons for Riku), KH1 drops / Lucky Bounty, Enemy Skills, Guard / Counterattack (no clips).
- Checked: tests (`party::tests::the_kings_sleights_only_in_rikus_battles`, `keyblades` layout tests), the asset step on the local install, the Windows build. Not checked: in game.

---

## Stage 269 (fork branch Re-Re-Reverse/Rebirth): Riku gets the KH1 HUD (Re:CoM's HP gauge at KH1's place with the MP ring), the KH1 look of the command menu, and a Customize page for his shortcuts, version 0.149.0 (2026-11-22)

Fork owner, 2026-11-22 (screenshot: Riku's Re:CoM HP gauge top left, the 7.2 MP bar under it, the command menu in Re:CoM's look), verbatim: "falta poder configurar os shortcuts igual da com o Sora, e ainda o command menu não aparece na tela igual do Sora, além disso a HUD ta com o mesmo problema que está com a do Sora … na prática o fix pode ser o mesmo entretanto trocando a foto do sóra com a do Riku".

- Found: Riku's `FO0071.SPR` `gauge:01` is `FO0002`'s byte for byte (47 groups; groups 7..9 identical), and `FO0071.RTM` holds `Cockpit.imz` with the same five language copies. Checked (files: the local install).
- Asset step: `kh1hud.lad` (+ copies) and Re:CoM's HP ring turned 180° (same first group 47) go into `FO0071.SPR`; `kh1hud.imz` (+ copies) into `FO0071.RTM` (new output `raw/FORM/0071/FO0071.RTM`). Riku's six sleight names are menu words (`command_menu::RIKU_WORDS`, ids `0x8100 + id`), baked in KH1's font. Not fatal: without it his story keeps Re:CoM's HUD. **Re-run** (automatic: the hash changed).
- DLL: `kh1_hud::active` holds in Riku's story with `riku_kh1_movement` (Re:CoM's gauge at KH1's place, its own face, which is Riku's; KH1's MP ring; room plate top left; Re:CoM's pieces hidden as for Sora); the KH1 menu look in his story only when his `FO0071` has the set (registry lookup); the registry check logged once per story.
- Customize for Riku: first row of his Settings list, Re:CoM's camp page with his three L1 slots and his six sleights + None, written to his save block's slot bytes; slots set (by the page or the one-time fill) are marked in the block's spare byte so emptied slots are not filled again.
- No new hook, patch or site. Checked: tests (workspace), the asset step on the local install (`FO0071.SPR` / `.RTM` written and copied into the mod folder), the Windows build. Not checked: in game; Riku's Dark gauge with the turned HP gauge. Details: `docs/recom-riku-kh1.md` § 0.149.0.

---

## Stage 268 (fork branch Re-Re-Reverse/Rebirth): Riku's HUD file (`FO0071.SPR`) gets the command menu's plates and the MP bar's groups, version 0.148.0 (2026-11-22)

Fork owner, 2026-11-22 (screenshot), verbatim: "aqui cara, ta faltando o COMMAND MENU e o Shortcut!!!".

- Cause: Riku's story loads `FORM/0071/FO0071.SPR` for its HUD (Cockpit 152 groups, copies 157); only `FO0002` was patched. Checked (files + the owner's log).
- Fix: the asset step patches `FO0071` (and its language copies) with the same groups (their atlas has the plate, colour, arrow and neutral texel at the same UVs: Checked (files)); `kh1-mp-bar.json` lists first group 152. Asset step: **re-run** (automatic: the hash changed); new output `raw/FORM/0071/FO0071.SPR`.
- Checked: the step run on the owner's install (output written and copied into the mod folder), tests, the Windows build. Not checked: in game. Details: `docs/recom-riku-kh1.md` § 0.148.0.

---

## Stage 267 (fork branch Re-Re-Reverse/Rebirth): the Camera setting for Riku, version 0.147.1 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "as opções de câmera podem ter para o Riku tbm". With `riku_kh1_movement` on, Riku's story no longer forces Re:CoM's camera (`win::camera::init`); his Settings list gets the Camera page. Checked: tests, the Windows build. Not checked: in game.

---

## Stage 266 (fork branch Re-Re-Reverse/Rebirth): Riku's KH1 MP, his command menu with his Sleights list, his shortcuts, version 0.147.0 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "b" (a list of his sleights paid with KH1 MP, shortcuts); "e seria interessante, atribuir uma barra de mana dele já que não terão as cartas, possível?".

- 0.146.0: `win::mp::{riku_live, live}`: his KH1 MP in the same save block (Sora's `active()` unchanged); charge from his KH1 hits; refill; save point; the MP bar built for him (placement not seen).
- 0.147.0: his sleight ids from `SY0002.CTD` (id = row − 1) and his sleight start `0x140216AB0` (takes 0x62..0x64, 0x66..0x6B). Checked (files + exe). His command menu (Re:CoM's look) with Attack and a Sleights list (Impulse, Maelstrom, Barrage and their Dark versions; MP ours), started by his own sleight start; his L1 shortcuts filled with his first three.
- Checked: tests (`riku::tests::rikus_sleights`), the Windows build. Not checked: in game. Details: `docs/recom-riku-kh1.md` § 0.146.0, § 0.147.0.

---

## Stage 265 (fork branch Re-Re-Reverse/Rebirth): Riku's KH1 swings show his own hit effect, version 0.145.1 (2026-11-22)

Fork owner, 2026-11-22, verbatim: "parece que funcionou mas os ataques estão sem efeitos/feedback quando acertam ent por um momento achei que não tivesse dando dano, mas deu sim".

- Cause: `onAttack` `0x1402250B0` reads the hit effect by attack id from Sora's row table; Sora's ids 1..7 name `ex_5010`, which only Sora's `EFFECT/1000` holds. Checked (exe + files).
- Fix (`riku::attack_id`): Riku's swings carry his own twin ids 0x97..0x9D (same clips and columns; effect `weapon` 12 / 0 from his `EFFECT/1500`). Checked (exe). Not checked: in game. Details: `docs/recom-riku-kh1.md` § 0.145.1.

---

## Stage 264 (fork branch Re-Re-Reverse/Rebirth): KH1's Attack for Riku, his empty deck and cardless enemies; the Settings list in Riku's story, version 0.145.0 (2026-11-22)

Fork owner, 2026-11-22 (screenshot: Riku's camp with Re:CoM's menu), verbatim: "certo mas não sei se foi intencional mas no menu ta faltando as opções tbm"; "e sim, vamos prosseguir com os testes, mas pode ser mais agressivo e já implementar o combate".

- All behind `riku_kh1_movement` (default off): the camp's Settings list in Riku's story (Game Settings, Combat, Movement); KH1's Attack on Riku's own swing rows 0x30..0x36 (Checked (exe): Re:CoM's attack rows 1-7); his row pick slot `0x1405A8990` patched like Sora's (checked old value `0x1401F7520`); hits through Re:CoM's own weapon volume, landing during enemy attacks; Re:CoM's card AP of Soul Eater (0x12) at both Damage settings; the empty revolver, no Friend-card drops, and KH1 battles (cardless enemies) in his story.
- Details, assumptions and what is lost (Dark Mode, card breaks, sleights): `docs/recom-riku-kh1.md` § 0.145.0.
- Checked: `cargo test -p kh1-recom` (`keyblades::tests` Riku layout / rows); the Windows build. Not checked: in game.

---

## Stage 263 (fork branch Re-Re-Reverse/Rebirth): Riku moves with KH1's movement in Reverse/Rebirth, behind `riku_kh1_movement` (default off), version 0.144.0 (2026-11-22)

Fork owner, 2026-11-22: "probably the riku from recom, is more compatible with kh1 combat system than yout think, i think if you try to use moves from kh1 boss, it will become too overpower"; "we have every animation already in KH recom, we would need just to rearange the animations and the system"; "ok now start the work". Lifts design rule 6 on this branch only, behind a default-off switch.

- **Research** (`docs/recom-riku-kh1.md`): Riku's action table `0x1407C6370` has Sora's row meanings for 0x00..0x38. Checked (exe). His Battle exe `0x140216B10` is `jmp 0x1401F9840`. Checked (exe). His roll exe is `0x140212350`, and he has no glide. Checked (exe). His skeleton is KH1's `xa_ex_1010` Riku (`ex_1010_sk`). Checked (files).
- **Change** (`crate::riku`, `win::movement::{kh1_riku, kh1_mover, player_abilities}`, `sites::data::RIKU_VTABLE`, ini `riku_kh1_movement`): the shared movement hooks run KH1's frame for the main `CPlayerRiku`. Only rows with a clip of his are played. Re:CoM's own roll, his cards, and High Jump from his ability word are kept. No Guard, Glide, menus, Enemy Skills or KH1 effects.
- Readers fixed: `kh1-formats` mset (an empty motion's 0xFFFFFFFF channel table); `kh1-recom-assets` MDL (the first part's offset from the file's first word: 0x20 in Riku's files).
- No new hook, patch or site. Assumptions: Re:CoM's card dispatch for Riku runs inside the move exe before its move step; Dark Mode doesn't disturb the KH1 frame.
- Checked: tests (`riku::tests::{rikus_rows_cover_kh1_movement, rikus_abilities}`, `cargo test -p kh1-recom -p kh1-formats -p kh1-recom-assets`); the Windows build (no new warnings). Not checked: in game; the Re:CoM decompile (not on this machine).

---

## Stage 262: the KH1 HUD is the default, and older settings files switch to it once (2026-11-05, version 0.143.1)

Owner, 2026-11-05 (a tester's screenshot: Re:CoM's HP gauge and FRIENDS row top left, the room plate bottom right, the command menu in its Re:CoM look, at combat = kh1), verbatim: "people that are trying the mod are seeing this. Lets fix that to be how the hud is supposed to look"

- **Cause** (Checked: code, the owner's own ini): `hud` defaulted to `recom` (`Settings::default`, `DEFAULT_FILE`), and no menu shows it, so a new install wrote `hud = recom` on first launch and showed Re:CoM's HUD; the owner's ini says `hud = kh1`.
- **Fix:** the default is `hud = kh1` (the KH1 HUD pieces at combat = kh1 only, as 0.102.3 made it follow Combat). The file gets `ini_version = 2`; `settings::migrate` updates an older file once at start (`hud = recom` or missing becomes `kh1`, then `ini_version = 2`), logged as `settings: … updated: …`. A version-2 file that says `hud = recom` keeps it.
- The HUD parts' defaults are unchanged (they match the owner's ini, but for `hud_kh1_party`, which only shows with `hud_kh1_gauge`, off by default).
- Checked: workspace tests (new `an_old_file_gets_the_kh1_hud_once`); Windows build. Not checked: in game.

---

## Stage 261: Sora's damage cap: each hit of Sora's side capped at the enemy's KH1 per-hit cap, at Re:CoM's damage join, version 0.143.0 (2026-11-05)

Owner, 2026-11-05, verbatim: "lets add a damage cap to sora". Answers (`design-philosophy-instructions.md`, "Damage cap answers"): KH1's per-hit cap (`+0xC2`) after power and the table, before resistance; (b) at Damage = Re:CoM the same share of HP; % of HP hits never capped; covers swings, specials, spells, Sleights-list sleights, LV3 summons and Party sleights; Re:CoM's extras stay under the cap; Combat = KH1 only.

- **Research** (details, tags and the caps by record in `docs/recom-kh1-attack.md` § Damage cap):
  - KH1 `0x1402BFA80` (single caller `0x1402BF630`): curve → the attack's bounds → floor `+0xC0` / cap `+0xC2` → × `resist[element]` → Aero / 2 → cannot-die → at least 1; element 8 (% of HP) and 9 / 11 / 12 never capped. Checked (KH1 decomp). The floor / cap are the record's r3 / r4 as they are; only r1 / r2 index the 21-row table (§ Damage and reactions corrected). No record has a floor above its cap. Checked (files: 236 records).
  - Re:CoM has no cap: the hit handler `0x14017F7E0` (gather `0x140154EB0` with the weak part, the frozen bonus `0x14015D940`) → `0x14017DC50` → its join `0x14017DF6A` (11 branches land there, none inside) → `0x14017DAB0` `(int)(value × resistance × falloff / 100)`. `+0x2DC` stored there is re-applied by the flatten exe (`0x140184DCB`); `0x140182C50` applies a kind 9 / 0x12 hit on landing through the same join; the stop exe `0x1401843A0` records held hits from the gathered `+0x34` (`0x1401844BD`), not from the join. Checked (decomp + disasm + exe).
  - The Damage = KH1 leaks: our KH1 number was capped, then the weak part, the frozen +100, Back Attack × 2 and the Enemy Skills rewrite went on top; the sleights' / summons' / Party sleights' scaled numbers had no cap; held hits under Stop. Checked (code + decomp).
- **Change** (`crate::damage_cap`, `win::damage_cap`; `kh1_core::damage::compute_damage_cap_later`; `attack::kh1_damage_cap_later`, `magic::spell_damage_cap_later`; `win::stats::{kh1_hit_with, kh1_spell_hit, record_from, recom_max_hp}`; `win::specials::shot_damage`; `win::summons::counted_hit`; `win::attack::kh1_enemy_hit` / `spell_hit` / `ragnarok_shot_hit`; `win::battle::manager_ctor`):
  - New patch `sites::DAMAGE_CAP_PATCH` (`0x14017DF6A`, 23 bytes) + `DAMAGE_CAP_CHECKED` (`0x14017DAB0`, `0x14017DAED`, `0x14017F917`, `0x140184DC0`, `0x1401844BD`): the join jumps to a 99-byte stub near the exe that finds the enemy (`rbx`) in a 16-slot `{enemy, cap}` table and takes `min(xmm6, cap)`, then runs Re:CoM's own instructions and call. Addresses rebased to the running image; nothing per frame (once per damaging hit).
  - The enemy-hit wrapper sets the enemy's slot per hit of Sora's side (and clears it for any other hit; every battle start and Combat = Re:CoM clear the table): Damage = KH1, KH1's number (swings, specials, spells, Ragnarok's shots) is now made **without** the cap and the cap is KH1's cap × the element's resistance (a capped hit truncates exactly as KH1's); Re:CoM's numbers on KH1's HP (sleights, summons, Party sleights) take the raw cap; Damage = Re:CoM: cap × Re:CoM max HP ÷ KH1 max HP, rounded to nearest (halves up), at least 1. Gravity Break, Zantetsuken, Gravity, Stop (and Re:CoM's kinds 8 / 0xD): never. A stopped enemy's gathered value is capped too. No KH1 record (or cap 0): no cap.
  - Log: each enemy's first capped hit a battle (`damage cap: <class> (<twin / proxy / boss twin>), <hit>: 48 -> 30 (KH1 cap 30 of 700 HP; …)`); the F9 trace adds `:cap <before>><after>`; `:kh1 <n>` is now the number before the cap.
- Not capped (outside the owner's list): shots Sora's Guard turned back; any other hit on an enemy.
- Assumptions: no other Re:CoM path applies a Sora-side hit to an enemy's HP outside the join and the stop exe; a deferred landing / flatten re-apply uses the enemy's slot at that time (another hit first replaces it).
- Checked: tests (`kh1_core::damage::tests::{the_cap_later_is_kh1s_cap, a_bonus_before_the_cap_stays_under_it}`, `damage_cap::tests::{the_share_of_hp, the_join_takes_the_smaller, the_stub_replays_recoms_join, log_lines}`, `sites::tests::damage_cap_sites`), `check_sites_rs.py` (1429 sites, 0 mismatches), `cargo test --workspace`, the Windows build (no warnings). 1:1: KH1's rule and order Checked (KH1 decomp); a capped hit equals KH1's number (test). Not checked: in game; no tester trace. Asset step: not needed. Docs: `recom-kh1-attack.md` § Damage cap (new; § Damage and reactions corrected).

---

## Stage 260: the KH1 playground is removed from the repo (2026-11-05, version 0.142.1)

Owner, 2026-11-05, verbatim: "remove the playground from the github also, as its not necessary to the mod"

- `crates/kh1-playground` (the 1:1 Rust demo with the mannequin Sora) is deleted, with its workspace entry, the README section and the build bot's `--exclude kh1-playground`. `Cargo.lock` drops its dependencies. The mod does not use it: no other crate depended on it (only a comment in `kh1-trace`, updated). The KH1 rewrite the mod checks against (`kh1-core`, `kh1-anim`, `kh1-data`, `kh1-formats`) stays.
- Older docs still mention the playground as history.
- Checked: `cargo test --workspace` passes. No DLL change.

---

## Stage 259: the first-launch build shows its progress on Re:CoM's title (Re:CoM's own text) and the title waits for it; no console window, version 0.142.0 (2026-11-05)

Owner, 2026-11-05, verbatim: "I definitely want a progress screen in the game window while everything loads" (0.141.0's build ran in a `rundll32` helper with a console window, inside Panacea's `OnInit`, before the game window existed).

- **Research** (details and tags in `docs/openkh-github-install.md` § 8):
  - Panacea reads a listed file's content and size from disk at each open; its listing (`FakePackageFile`) only says a name exists. Re:CoM's loader `0x1402427D0` sizes through the hooked `GetFileSize` `0x1402C4530`, then `LoadFileWithSize` `0x1402C48F0`; the only unhooked `GetFileInfo` caller `0x1402C4990` seeks. Checked (OpenKH source `b67a743`; decomp; Panacea's patterns at those addresses in the Steam exe).
  - The step's only names `Recom.hed` lacks are the Keyblade groups' 76 files (`PLAYER/00GG/PL00GG.MDL`, `raw/…RTM` for 21..36 / 38..43; `.EET`, `raw/…EFF` for 21..36). Checked (files: the owner's `Recom.hed`, test).
  - None of our outputs opens before the title ends: FORM 2 is linked only from the world groups (category 14), EFFECT 1000 from the player / effect groups, FORM 100 / 109 and PLAYER / ENEMY are asked for by the camp, the shop, rooms and battles; the title asks for (0, 0xD / 0xE) only (FORM 505 / 507 / 111). Checked (exe: every link row; decomp: the request sites). Assumption: no computed request runs before the title.
  - `CTitle` (`0x14039F770`): state 0 `0x1403A3770` waits for the loader's queue, state 1 `0x1403A3820` plays the title (sub-state `+0xAC` 0), waits for a press (10), fades (0x14). Its alert text: `C2DFnt` size 0xF (0x10 with `[0x140AF73B0]+0x274`), centred, on `aleart:00` (`0x1403A23D0`, `%s:%02d` at `0x140606EC8`). Checked (decomp + disasm).
- **Change** (`win::assets`, `win::install_deferred`, `asset_build::{Shown, percent, placeholders, game_name, place_job, run_job_with, title}`, `kh1_recom_assets::step::{STAGES, stage, Progress, set_progress}`, `stamp::place_with`, `keyblade::{new_resource_names, LAST_KH1_CARD}`):
  - At `OnInit`, when a build is needed: the new names (and the last stamp's names `Recom.hed` lacks) not in Panacea's folder are made empty for its listing; the title's wrapper goes in; the rest of the DLL's start waits; a thread starts the build after `OnInit` returns (no process made under the loader lock).
  - The helper (`rundll32 … BuildAssets`, no window, in a job object that ends it with the game) builds, stamps, copies into Panacea's folder, and writes `kh1-recom-build.progress` (stage, percentage; the end) and `kh1-recom-build.log`. In-game fallback on its thread if the helper cannot start (failed-marker guard).
  - New hook `sites::TITLE_HOLD_HOOK` (`0x1403A3820`, 6 bytes; only reference the ctor's `lea` `0x14039F858`; no branch in: exe scan) + `TITLE_HOLD_CHECKED`: while the build runs, sub-state 10 does not run Re:CoM's update (the title stays, no press is taken) and shows two lines of Re:CoM's text (a `C2DSeqCtrl`, priority 0xC, two `C2DFnt`s as the title's alert text, squeezed to 480): "KH1 in Re:CoM: building files from your games... N %" / "Step k of 10: <stage>. …"; then on the game's thread: the cache checked, unfilled empty names removed, the deferred start, "done" (2.5 s) or "KH1 FM not found: playing without KH1 files" / the failure line (7 s); then the title goes on.
  - Progress is the step's stages (enemies one part each) and the copy (one part per file), not time.
  - Kept: the stamp, `generated/`, the copy back after a Build, `asset_build = auto | off`, the failed marker, the start without KH1 files. Without the title hook: the 0.141.0 order (built before the game starts), without the console. Normal starts (nothing to build) unchanged.
- Per frame: nothing outside the build; on the held title one atomic load, the progress file read every 6 frames.
- The step's sources changed (progress reports), so its hash changes: every install rebuilds once after this update.
- Assumptions: the rest of the start is safe at the title (gameplay hooks only, on the game's thread; no other thread runs those functions then); `aleart` loaded at sub-state 10; priority 0xC over the title; the lines' place; `%` drawn as is; the game's `SetReplacePath` after every `OnInit` (0.141.0's).
- Checked: tests (`asset_build::tests::{progress_reads_back_and_counts_stages, placeholders_are_the_new_game_names_only, place_job_copies_with_progress}` + the job round trip, `sites::tests::title_hold_sites`, `keyblade::tests::new_names_are_not_in_recom_hed` (with the owner's `Recom.hed`), `step::tests::progress_starts_with_reading_and_stops_with_the_run`, `stamp::tests::place_copies_only_what_differs`), `check_sites_rs.py` (1423 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game; a full build's time. Asset step: its output unchanged (re-run automatically: the hash). Docs: `openkh-github-install.md` § 8 (§ 3 corrected), `README.md`.

---

## Stage 258: the turned HP ring's start closes on its black line (no gap at partial HP), version 0.141.1 (2026-11-05)

Owner, 2026-11-05 (screenshot, KH1 HUD bottom right), verbatim: "there's a little gap in the green hp bar where the bar starts, lets fix that".

- **Cause** (Checked (files + decomp + screenshots)): Re:CoM's HP gauge (`gauge:01` groups 7 / 8 / 9, turned as 47..49 at KH1's place) draws 0..270° of the ring (frames 0..270) under quadrant scissors whose start edge is one unit off the start line ((0, 1)..: turned ..(0, −1)); a scissor's rows are its corners + 0.25 floored, inclusive (`0x14024A3F0`, `0x1402643D0`; the PC rect `0x1402EC4E0` → `0x1402EAC30`). The start line (the art's centre) is 0.25 off whole units, so Re:CoM's way round the ring starts 0.75 below its black line, and turned it stops 0.25 above it. On the owner's 720p screenshot that is 2 pixel rows (about 1 unit) of background across the band, track included; the PC's sprite / scissor rounding adds to it (Assumption, measured). At full HP (frames 271.., the ring unclipped) no gap (the owner's 1440p recording).
- **Not the cause**: the MP ring and its outline (they reach the line), the 0.102.4 dividers / lifted straights, a UV seam, the ring's start angle.
- **Fix** (asset step `recom_hp::start_filler`): each turned group gets one animation first (before any scissor): the ring's own start-quadrant picture from its unrotated full-ring animation, cut to its first 2 units (a new sprite, same texels and colours), turned with the group, frames 4..=270. Its edge is the picture's straight edge on the start line, as the full ring draws it. From frame 4 (29 · sin 4° ≥ 2: never past the ring's moving end). Re:CoM's groups 7..9 untouched (Combat = Re:CoM); DLL unchanged.
- No hook, patch, site or address; nothing per frame (one more sprite a layer while the ring is partial).
- Checked: tests (`recom_hp::tests::the_start_filler_closes_the_gap`, `the_start_scissor_misses_the_start_line_either_way`, `owner_gauge_files` on the owner's `FO0002.SPR` and five `<LANG>_gauge.lad`), offline render with Re:CoM's rules and placeholder art (before: the gap at frames ≤ 270 with sprite / scissor offsets 0..1 unit; after: none at frames 4..470), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings), the Linux `kh1-recom-build` (built; not run: no Re:CoM PKG in the session).
- Not checked: in game; Re:CoM's real ring art over the 2 filler units (`gauge.imz` not in the session).
- Asset step: **re-run** (the DLL does it at first launch: the step's hash changed). Output `raw/FORM/0002/FO0002.SPR` changes; no `mod.yml` change. Docs: `recom-kh1-hud.md` § The turned HP ring's start.

---

## Stage 257: install from GitHub through OpenKH; the DLL builds the game-made files on first launch, version 0.141.0 (2026-11-05)

Owner, 2026-11-05, verbatim: "I also want people to be able to install the mod from openKH by using the GitHub". Answers (`design-philosophy-instructions.md`, last entry): OpenKH's GitHub install of `PM2CRAZY/KH1-in-ReCom` from the default branch `testing`; "DLL builds on first launch", and again after an update changes the asset step.

- **Research** (Checked (source): OpenKH `b67a743`; GitHub API; details and tags in `docs/openkh-github-install.md` §§ 1–4):
  - The install clones the default branch when `main/mod.yml` is missing (`testing` here: the only branch). `mod.yml` must be at the root. The clone goes to `<openkh>/mods/<selected game>/<owner>/<repo>`, with full history (76 MB).
  - Update is `git fetch` + a hard reset (ignored files stay: Assumption, libgit2's rules).
  - Build deletes `<openkh>/mod/Recom` first. A `copy` asset with a missing source is skipped with a warning.
  - Panacea runs `LoadDLLs` / `OnInit` inside its `DllMain` (loader lock held). It lists `<mod_path>/recom` once, in the game's `SetReplacePath`. A new name written later is never seen; a later `raw/` one dereferences a null file-info.
  - So the files must be in place before `OnInit` returns: no worker thread. Done that way, they are used in the same run.
- **Repository root:**
  - `mod.yml` (`game: Recom`; one asset: `dll/kh1-recom.dll` from the bot's `openkh/kh1-recom.dll`; `build.yml` unchanged). `openkh/mod.yml` (0.1.0 text) removed.
  - `.gitignore` covers `/generated/` and the step's outputs in a mod folder.
  - `README.md`: the player's install steps and requirements.
  - The game-made files are not listed in `mod.yml`: ~180 warnings on the first Build, and a list kept by hand, for nothing the DLL doesn't do anyway.
- **The asset step as a library:**
  - `kh1_recom_assets::step::run` (the old `main.rs` body). Output goes through `say` / `set_sink`, and every written file is recorded.
  - `Dest::Root` (CLI `--root`); `previews` off for the DLL. `main.rs` is now only the argument parser.
  - The build script `step_hash.rs` hashes the step's sources (`KH1_ASSET_STEP_HASH`). `stamp.rs` holds the stamp (step hash, the inputs' names / sizes / times, the files and sizes), the failed marker, and `place` (copy what is missing or differs).
- **The DLL at start** (`asset_build.rs` rules, tested; `win/assets.rs`; called from `win::run` after the exe check, before anything reads the files):
  - It finds the mod's own folder: `mods-manager.yml`'s paths, else `<openkh>/mods/Recom`; the folder whose DLL source has the running DLL's size, else `PM2CRAZY/KH1-in-ReCom`.
  - The cache is `<mod>/generated/` (else `<game>/kh1-recom-generated/`).
  - It builds when the cache has no stamp for this step and these inputs. A failed marker for the same pair stops repeats.
  - The build runs in `rundll32.exe "<dll>",BuildAssets <job>` (export `BuildAssetsW`, this same DLL): a console window with progress, `kh1-recom-build.log`, and a message box on failure (no KH1: it says so; the game then runs without the KH1 files). A crash ends only the helper.
  - If the helper can't start, the step runs in the game under a guard marker.
  - Then the stamped files are copied into `<mod_path>/recom`, and only where missing or different (after a Build).
  - `asset_build = auto | off` in the ini (default auto).
- No hook, patch, site or address; nothing per frame.
- **Size:** about +2.1 MB of DLL (same toolchain: 0.139.4 4,447,217 bytes; 0.141.0 6,583,300).
- Assumptions:
  - the game's `SetReplacePath` runs after every `DllMain` / `OnInit`;
  - `CreateProcess` and the process wait work from inside Panacea's `DllMain`;
  - rundll32 calls the `W` export and accepts the quoted DLL path;
  - a hard reset keeps ignored files.
- Checked: tests (`stamp::tests::*`, `asset_build::tests::*` including `the_repo_root_is_an_openkh_mod`, `settings::…::asset_build_switch`), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings; three old missing-docs warnings in the asset crate fixed, as the DLL now builds it).
- Not checked: in game; a real OpenKH install; the step end to end (this session's game folder is a sparse copy without `PL0001.SES`); a full build's time.
- Asset step: unchanged in what it writes. Docs: `openkh-github-install.md` (new), `kh1-recom-assets.md` (Use, Open), `recom-keyblade-models.md` § 11.

---

## Stage 256: equipped rows say "Equipped" where Re:CoM's "NEW!" badge was (Abilities, the Keyblade list, the Customize picker), in Re:CoM's own text, version 0.140.0 (2026-11-05)

Owner, 2026-11-05 (recording, Abilities > Enemy Skills), verbatim: "also, instead of equipped items/abilities saying "new" when they're equipped, can they just say "equipped""

- **The badge** (Checked (decomp + disasm + files + recording)): Re:CoM's Sleights page (`0x1400E6DF0` mode 9) adds `camp:00` sequence `0x1D + (byte camp+0x21DC+id > 0)` with the row sprite's `+0x40` at (170, −13) (`0x140509308`, `0x140509340`). 0x1E: sprite 31 pulsing alpha 0x40 ↔ 0x80; 0x1D: the same sprite static at alpha 0x20. Sprite 31 is the "NEW!" picture; with the anchor (22, 14) its box is x 149..188, y 8..20 of the plate (glyphs in the recording: 152.5..186.7, 9.7..19.0). A picture in the camp texture, not a flag of Re:CoM's row. Our pages drew 0x1D on every `Page::marked` row (the equipped Keyblade, the Customize picker's current pick, equipped abilities); the Items page never did. Nowhere on our pages did it mean "new", so no "NEW" is kept; Re:CoM's own Sleights page is unchanged.
- **Change** (`crate::keyblades::badge`, `win::keyblades::Page::equipped_word`): those rows get Re:CoM's own text "Equipped" instead of the sequence: a `C2DFnt` (`0x140243F10`, font 0xE, right-aligned 0x41, text by `0x140258DE0`) attached with the sprite's `+0x60` as the status values are, right edge at plate x 187, offset y 8, colour `0x8014432E` (the badge's yellow-green, opaque and calmer: about (128, 180, 64)); static, as 0x1D. Wider than 67 units: the true squeeze. A marked row's name is squeezed to end 4 units before the word. Chosen over an asset-step sprite or a swapped "NEW!" picture: both need a camp texture edit (a `camp:00` sprite cannot read Re:CoM's font texture; `FO0100.RTM` not in the session), this needs no asset.
- No new call, hook, patch or site; nothing per frame (made with the row). Removed: the 0x1D mark call and `camp+0x21D8` read.
- Assumptions: font 0xE sits on its offset as font 0xF (top-aligned 8.9, bottom-aligned 7.4; 8 within a unit of either); the text colour map 13 + 2.49 × value (fitted to one Re:CoM colour); English only.
- Checked: tests (`keyblades::tests::the_equipped_word_takes_the_badges_place`), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-keyblade-equip.md` § "Equipped" instead of "NEW".

---

## Stage 255: the KH1 camera keeps Sora near the centre (KH1's look point 0.5 m ahead of him, not behind), version 0.139.4 (2026-11-05)

Owner, 2026-11-05 (0.139.2, Camera = KH1, Auto on), verbatim: "the KH1 camera isn't right. The camera itself is good but sora isn't the center of it if that make sense" (an 8.6 s recording).

- **Video** (measured per frame, kept outside the repo): Sora's hips at rest 0.49 / 0.57 of the screen; running right 0.65–0.70 across; running at the camera 0.72 down, 1.15–1.25 times his rest size. He runs ahead of the camera.
- **Cause** (Checked (KH1 Steam decomp + disasm)): KH1's follow `0x1401E1920` takes its look target as Sora's matrix `+0xD0` × (0, −100, −50) (`0x140386A40` → `0x1400E5AD0`: Σ vᵢ · rowᵢ). That matrix (`0x1402A0A40`, `+0x374 & 0x1000` clear) has row 2 = (0, 0, 0, 1) − facing (`0x140386D10` from `0x140527FF0`), row 0 = facing × up (`0x1400E82F0`, a × b), row 1 = up; facing `+0x30` = (sin ψ, 0, cos ψ) (`0x140297E10`). So the look target is 1 m up and **0.5 m ahead** of Sora. Stage 1 (0.134.0) took "local +z = facing" as an Assumption and put it 0.5 m **behind**. With the 10 % spring's lag (9 × Re:CoM's 10.33 units a frame), the look point trailed him by 1.43 m instead of 0.43 m.
- **Offline model** (`camera::tests::a_running_sora_stays_near_the_centre`: Re:CoM's run, four directions, Auto, open room): 0.139.2's code settles at 0.70 across running right, 0.72 down and 2.67 m running at the camera, 0.55 down at rest. That matches the video. KH1's offset settles at 0.56 / 0.44 across, 0.61 down and 3.63 m at the camera, 0.49 / 4.46 m away, 0.61 down / 3.56 m at rest.
- **Not the cause** (Checked (decomp)): the springs (KH1's 20 % / 10 % up / 50 % down, halved at step 1.0, on the look point as KH1); Sora's position (Re:CoM's node 0 `+0xD0`, the read of Re:CoM's own subject point `0x1401F0510`); Re:CoM's tail `0x1401FC0D0` / view `0x14024FA40` (no re-smoothing); the sweep `0x1401FBC60` (writes `to` only on contact); the lock camera's anchor and point ahead (Ry(camera yaw) from identity, not Sora's matrix); the pitch assist (his `+0x30` directly). **KH2 (modes 8 / 9)**: its look goal has no actor-local offset (straight above the feet / 30 % to the target); the same path keeps Sora within 0.40..0.60: not affected, nothing changed.
- **Fix** (`camera::kh1::actor_point`, `Input::target` / `above`): KH1's actor matrix rows. No hook, patch, site or record change; nothing new per frame.
- Assumptions: Re:CoM's camera exec once a 60 Hz frame; KH1's slope branch of the matrix (`0x140292010`) not carried; the model's projection (Re:CoM's 53° focal on 640 × 480, 16:9 keeping the height).
- Checked: tests (`camera::tests::the_look_target_is_half_a_metre_ahead_of_sora`, `a_running_sora_stays_near_the_centre`; `offsets_and_conversions` and the lock test's rest updated: the follow's eye now starts inside the lock anchor's 75 dead zone), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game; no KH1 camera trace. Asset step: not needed. Docs: `recom-m12-camera-plan.md` § 18 (§ 3.2 and § 13 corrected).

---

## Stage 254: the Abilities and Items lists keep their scroll on a confirm (equipping no longer drops the row to the bottom line), version 0.139.3 (2026-11-05)

Owner, 2026-11-05 (recording, Abilities > Enemy Skills), verbatim: "When activating an ability it automatically puts it all the way at the bottom of the list."

- **Cause** (Checked (the recording + code)): the order never changed (Aquatank stays between Search Ghost and Wight Knight; the cursor stays on it). Every confirm re-reads the page, and the re-read (`win::keyblades::Page::abilities_read` / `items_read`) rebuilt the list with `list::open_on`, which starts at top 0: with rows 3..10 shown and the cursor on row 7, the window jumped to rows 0..7, the cursor's row on the last line. Not our order (`abilities::page::list`, KH1's fixed sort key) and not Re:CoM's list widget (no sort; the rows are ours).
- **Fix** (`crate::keyblades::list::{keep, Scroll}`; `abilities::page::Screen::depth`, `items_page::Screen::depth`): each window's top row is kept by its depth: a re-read of the same window keeps it (clamped to the rows there, then moved only as far as the cursor needs, draw mode 9's rule); going back to a window restores it; a new window starts at its top. The Items page (Sora's 9-row list, the stock list) had the same jump: fixed with it. The Keyblade list never re-read on equip (it did not move); cancel from it now hands back Sora's list as it was shown.
- No hook, patch or site; nothing per frame.
- Checked: tests (`keyblades::tests::a_reread_keeps_the_window_where_it_was`, `going_back_restores_the_windows_scroll`), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-m9-abilities.md` § The list keeps its scroll on equip.

---

## Stage 253: no crash on a field swing after a battle (KH1 enemies' attack rows read only in a running battle) (2026-11-05, version 0.139.2)

Owner, 2026-11-05 (0.139.1), verbatim: "game crashed"

- **Cause** (Checked: the owner's log + this build's symbols): `CRASH: exception 0xc0000005 at kh1-recom.dll+0x13197f (reading 0x40a00018)`, in `win::kh1_enemy::swing_meets_attacks` (from `win::attack::pair_dispatch`), inside the inlined `live_rows`: it read an enemy's attack set `[enemy+0x100]` = `0x40A00000` (a float, 5.0) from an enemy that was already freed. The log shows a room change in the field, then a swing. `live_rows` kept every runtime of the last battle number, and that number still names the last battle after it ends, so in the field it walked the freed enemies of the last battle.
- **Fix:** `live_rows` returns nothing unless a card battle is running (`win::battle::cbtl_live`: Re:CoM's CBTL task is there), and skips a runtime whose enemy is no longer ours (`ours(en) == Some(class)`: our vtable copy and tail magic).
- The second `CRASH` line (a null read in a system DLL) came after the first, from the same failing frame.
- Checked: workspace tests; Windows build with no warnings. Not checked: in game.

---

## Stage 252: the game no longer aborts at start (camera block rebased); Enemy Skills' rewrite stubs and the KH1 lock rules' stub block fixed (2026-11-05, version 0.139.1)

Owner, 2026-11-05 (0.139.0), verbatim: "my game wont launch now"

- **Cause** (Checked: the owner's log + code): the log ends at `trace: press F9 …`, the line before `camera::install`, and no `camera:` line follows. The camera's stub block (0.134.0, `camera::stub::block`) formed rel32s to Re:CoM's record table `0x14087B660` and return points `0x1401FC521` / `0x1401FBE90` as **Steam VAs**, but the block sits near the **running** image (ASLR: the owner's exe at `0x7FF77ED50000`), so `rel32` asserted ("rel32 out of reach") and the DLL aborted (panic = abort) during start-up. Tests passed because they used the Steam base.
- **Fix:** `stub::block(at, image, …)` moves those VAs to the running image first (`table_read_in`); `win::camera` passes the base. New test: the block near an ASLR base lands on the moved table.
- **Same class, latent** (Checked: code): Enemy Skills stage 3's rewrite stubs (0.129.0, `skill_effects::rewrite_block`) were built from running addresses against Steam VAs and truncated with `as i32`: with Zexion, Lexaeus, Demyx, Defender or Ursula equipped, their jumps back into Re:CoM's rewrite went outside the exe. `win::enemy_skills::install_rewrite` now builds the block in the Steam image's address space (`img`); the block holds no absolute pointer, so the bytes are right wherever ASLR puts the exe.
- **KH1 lock rules** (0.137.0): the owner's log said "no stub block within reach of Re:CoM's lock-on". Not memory: the five 14-byte jumps are 70 bytes, past `hook::place_near`'s 64-byte limit, so the rules never went in (R1 kept Re:CoM's rules). Now placed with `place_built_near`.
- Checked: workspace tests; Windows build with no warnings. Not checked: in game.

---

## Stage 251: M12 camera stage 5: KH2's lock-on camera as mode 9 (look 30 % to the target, turns only when Sora or the target leaves the screen, 380 at 1 a frame; field and battle), version 0.139.0 (2026-11-05)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Camera answers: "KH1 / KH2 lock camera in field rooms too", "KH2's numbers scaled ×0.89"). Stage 5 of `docs/recom-m12-camera-plan.md`. Camera = KH2 only.

- **KH2** (Checked (KH2 Steam decomp + exe)): lock mode 2 `0x140176C70` with `0x140176EC0(cam, point, 0.3)`: look goal 30 % from the chest (170) to the target; the screen test `0x140176710` (512 × 416, margins 32 / 480 / 8 / 408 `0x14071A478`) on the target and Sora's point (100); off screen → goal yaw on the line from the last look goal through Sora (`0x140176A70`) ± 20°; bottom edge − 2 a frame each, else to 380 at 1 (`0x140164290`); far (> 300) and steep → the chest; stick 3° (`0x14071A470`), zoom `0x140176BC0`, R3 to the line and 380; the eye from the chest, its wall rays (`0x140164910`, `0x140176E40`). Its lock start / end `0x140176910` / `0x140176A00` only set the mode.
- **Change** (`camera::kh2::{Cam::lock, screen, Target}`, `camera::dispatch` 8 ↔ 9, `win::camera::kh2_step`): mode 9 on Re:CoM's lock handle `player+0x850` through Re:CoM's `0x1401FBE60` (not with a subject object), field and battle; the target point Re:CoM's own `0x1401FC400`; the screen test with Re:CoM's look-at axes on last frame's view. No new hook, patch or site.
- Per frame (mode 9): KH2's lock math 134 ns (Linux release); 2 Re:CoM sweeps (Re:CoM's lock step: 1). Not measured in game.
- Assumptions: Re:CoM's `CamTrgOfs` point for KH2's camera shape; KH2's handedness is KH1's (screen left / right); the behind bit at a depth under 1.
- Checked: tests (`camera::tests` 3 new + the cost test), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game; no KH2 camera trace. Asset step: not needed. Docs: `recom-m12-camera-plan.md` § 17.

---

## Stage 250: M12 camera stage 4: KH2's follow camera as mode 8 (Camera row Re:CoM / KH1 / KH2; KH2's Auto behind the Auto camera row; lengths × 0.89), version 0.138.0 (2026-11-05)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Camera answers: "KH2's numbers scaled ×0.89", "An "Auto camera: On / Off" row for KH1 and KH2", each game's own Normal, re-place after an event). Stage 4 of `docs/recom-m12-camera-plan.md`. Camera = KH2 only; Camera = Re:CoM / KH1 unchanged.

- **KH2** (Checked (KH2 Steam decomp + exe)): frame `0x140166100`; follow `0x140165110` (look 170 above the feet; eye `0x140164520` at d on the height curve eye.y = look.y + 150 − (d − 250)² · 0.008, yaw ≤ 10° a frame `0x140718BE4`); row 0 `0x140718C00` (FOV 1.5 rad, 420 / 380 / 250..500); right stick `0x14039C180` (60 dead zone per axis, bits, tilt), turn 3° `0x140718BF0`, zoom 8 `0x1401654D0`, option swaps bits 7 / 8; R3 reset (logical 0x20000: behind, 420, a cut with the left stick held); swing-behind (`0x140718BE8` / `BEC`, distance back 0.5); wall `0x140164940` (+5 along the normal); springs `0x140165BD0` from the view `0x140165700` (eye 8.75 %, look 16.25 % a frame, mouse 50 %); re-place `0x140164FA0`. FOV: KH2's focal 320 / tan(0.75) = Re:CoM's 256 · cot(73.39° / 2) (`0x14019DDE0`, `0x14024FCB0`, `0x14023E720`, `0x140821ADC` = 384).
- **Change** (`camera::kh2`, `camera::{Family::Kh2, dispatch, records}`, `win::camera::kh2_step`, `settings::Camera::Kh2`, `menu` Camera row 3 values): mode 8 with its record (FOV 73.39, look 1.513 m, eye 3.738 m back 0.723 m up) in the existing block's slot 2; Re:CoM's 1 / 3 / 4 and our 6 / 7 switch to 8 at Camera = KH2 through Re:CoM's `0x1401FBE60`; KH2's step in its own numbers on a world ÷ 0.89 (every length × 0.89), Re:CoM's sweep for its rays; entries: new camera = KH2's room start, back from another camera = its re-place, Re:CoM's setter = carry on from the drawn view. Left stick from Re:CoM's pad device `+0x74` / `+0x78`. No new hook, patch or site. KH2's follow stays on while locked (stage 5).
- Not carried (owner questions): R2 held swapping Auto / Manual (R2 is Re:CoM's lock-on at Combat = Re:CoM); Select's first person; KH2's map data (no-zoom areas, camera volumes, attribute-5 walls); the mouse's |d · 10| multipliers (1).
- Per frame (Camera = KH2): KH2's math 62 ns (Linux release, 10⁶ frames); 2 Re:CoM sweeps (Re:CoM's follow: 1). Not measured in game.
- Assumptions: KH2 Sora's run value `+0x12C` = 8 (the run terms 0); Re:CoM's keyboard / mouse flag for KH2's mouse mode; the sweep's normal (stage 2's); both PC ports widen the frame alike; KH2's handedness is KH1's; R3 = Re:CoM's 0x400.
- Corrected: the stage 1–3 KH2 notes' "R3 → mode 1" is Select → first person; R3 is the reset.
- Checked: tests (`camera::tests` 7 new + `cost_of_a_kh2_step`; `menu` / `settings`), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game; no KH2 camera trace. Asset step: not needed. Docs: `recom-m12-camera-plan.md` § 16.

---

## Stage 249: KH1's lock rules on R1 at Combat = KH1 on a controller (R1 again unlocks at once; L2 steps through KH1's lock list; R1 with no target does nothing) through Re:CoM's own lock-on, version 0.137.0 (2026-11-05)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Lock-on rules and camera walls), verbatim: "1. B 2. dont worry about iit": KH1's lock rules on R1 at Combat = KH1 (controller): R1 again unlocks at once; L2 steps through targets (KH1's); no hold-style Config row; button prompts as they are. Keyboard / mouse, Combat = Re:CoM and Riku's story: Re:CoM's.

- **KH1** (Checked (KH1 Steam decomp + exe)): pad table `0x140528290` (R1 → `0x400`, L2 → `0x800` and `0x400000`, R2 → `0x400000` only: R2 steps nothing); the lock-on `0x1402B0610` reads pressed edges (`0x14028B430` `+8`): unlocked, R1 locks its pick, nothing without one; locked, R1 unlocks at once (the release only with KH1's hold option `0x1402FAFA0`), else L2 steps its lock list (`0x142DB3520`, cursor `0x142DB3830`, count `0x142DB382C`, room 97, emptied at every unlock `0x1402B0970`): the next entry, else its pick of the unlisted targets appended, else back to the first, else nothing; L2 unlocked nothing. Its pick `0x1402B1540` / `0x1402B0F90`: lowest `(int)max(0, distance − lock-point radius)`, first on a tie. Each frame `0x1402B11B0`: target gone / out of range → unlock (no step); the list drops the gone / out-of-range entries. The camera module never reads L2 / R2 (camera agent, Checked).
- **Re:CoM** (Checked (decomp + disasm + exe)): its lock-on `0x1401E8DA0`'s five input calls: pressed `0x1401E8E44` (unlocked), released `0x1401E8E9C` (with `+0x424 & 0x1000`), held `0x1401E8EFF`, stick flick `0x1401E8F2D`, switch `0x1401E8F3C` (`0x1401CD4B0(+0x850)`, then Re:CoM's own switch: cursor, sound 7). The cursor update `0x140208EC0` takes a new target only within 14 m in a battle room (`+0xF8 & 4`) / 3 m (`0x14020919B`, data `0x140509248` / `0x140509258`), measured from `0x1401F0AB0`; pressing with no target arms the lock on nothing.
- **Change** (`crate::lock_on::{History, nearest}`, `win::lock_on::install_rules`): the five calls retargeted (`sites::LOCK_ON_CALLS`, `call rel32` each to one stub block; all or none); inside a lock-on call the 0.133.0 wrapper marks as KH1's (KH1 Sora, the pad in use) they answer KH1's way: R1 pressed with Re:CoM's pick (`+0x84C`) takeable / else not pressed; locked, R1 pressed = "released after a second press" (Re:CoM's unlock, sound 8); held = L2 pressed; flick = yes; switch = KH1's next from our KH1 lock list over Re:CoM's `LockOnManager` targets (KH1's score with the KH1 twin's lock radius, within Re:CoM's take reach). Otherwise each calls Re:CoM's own callee. The list follows Re:CoM's cursor after each call and drops the gone / out-of-reach entries each locked frame. Side effect: R1 held while locked no longer sets `+0x424 & 0x800`, so at Camera = Re:CoM the right stick turns the camera while R1 is held (0.133.0 blocked it); only an L2 step's frame sets it.
- Not 1:1 (Re:CoM's kept): the lock press's target (Re:CoM's nearest, what its cursor shows); KH1's lock points per part, room priority classes, line of sight, 12 / 15 m flagged ranges (Re:CoM's 14 / 30 m); Re:CoM's sounds. Assumptions: Re:CoM's points (player mover, target vtable `+0x50`) with the KH1 twin's radius order targets as KH1's points would; KH1's hold option off by default.
- Sites: `LOCK_ON_CALLS` (5), `LOCK_ON_RULES_CHECKED` (8). Per frame: at KH1 on the pad two atomic stores and one load more; locked one handle lookup and point read per listed target; each retargeted call one atomic load; an L2 press walks Re:CoM's target list once.
- Checked: tests (`lock_on::tests` 6: buttons, KH1's pick, the step order and wrap, one target / none, the drop, the room; `sites::tests::lock_on_rules_sites`), `check_sites_rs.py` (1420 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Camera: nothing to change (it reads Re:CoM's lock handle `+0x850`; an L2 step runs its KH1 re-lock blend, as KH1's step re-runs its lock camera `0x1401E41A0`). Docs: `recom-lock-on.md` § 0.137.0.

---

## Stage 248: M12 camera stage 3: KH1's lock-on camera as mode 7 (60 / 45-frame blends, re-lock blend, field and battle), version 0.136.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Camera answers: "KH1 / KH2 lock camera in field rooms too"; § Lock-on rules and camera walls: KH1's lock rules on R1, L2 steps through targets). Stage 3 of `docs/recom-m12-camera-plan.md`. Camera = KH1 only.

- **KH1** (Checked (KH1 Steam decomp + exe + disasm)): the lock camera mode 3 `0x1401E2650` (anchor (0, −100, 350) `0x140506DF0` with a height assist; a look-past point radius + 100 beyond the lock point; the eye 90 % past a 75 dead zone, clamped by a ray from Sora's body point; the yaw half the error outside ±5°, at most 10°; the keep-Sora pull to 80 % over 40 frames; the re-lock look blend of 60); the switch blends `0x1401E5360` / the task `0x1401E5D90` (60 in `0x1401E41A0`, 45 out `0x1401E4110`, weight cos(left / total · π/2)).
- **Change** (`camera::dispatch`, `camera::kh1::Follow::{lock_step, lock_on, unlock, switch_blend, view}`, `win::camera`): mode 6 ↔ 7 on Re:CoM's lock handle `player+0x850` through Re:CoM's `0x1401FBE60` (not with a subject object `+0x2B8`, Re:CoM's rule), in field rooms too; the lock point Re:CoM's own `0x1401FC400` (`player+0x870`, the target's `CamTrgOfs` point); KH1's lock radius from the twin (`win::stats::lock_radius`, else 0); a handle change while locked runs KH1's re-lock blend. New callee site `0x1401FC400` (`sites::CAMERA_CALLEES`).
- Per frame in mode 7: 2 Re:CoM sweeps (Re:CoM's lock step: 1); not measured in game.
- Assumptions: Re:CoM's `CamTrgOfs` point for KH1's bone lock points; radius 0 without a twin. Not carried: Select's first person.
- Checked: tests (`camera::tests` 17 + the cost test; the lock's rest pose, close, turn cap; the blends, the re-lock, the unlock; dispatch 6 ↔ 7), `check_sites_rs.py` (1407 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game; no KH1 camera trace. Asset step: not needed. Docs: `recom-m12-camera-plan.md` § 15.

---

## Stage 247: M12 camera stage 2: KH1's Auto camera behind an "Auto camera" row (swing-behind, wall-avoid, pitch assist, eye walk, visibility re-place, auto recenter), version 0.135.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Camera answers: "An "Auto camera: On / Off" row for KH1 and KH2"; § Lock-on rules and camera walls: the near-wall ramp "(a) 8 + 8 flat 2.5 m rays"). Stage 2 of `docs/recom-m12-camera-plan.md`. Camera = KH1 only; Camera = Re:CoM unchanged.

- **KH1** (Checked (KH1 Steam decomp + exe)): mode 1's Auto branch (`0x1401E1920`, Config field camera 0): the swing-behind `0x1401E6FF0` (60-frame ramp, then a second to 90, a wall ramp to 60; 2 % + 2 % of the angle to Sora's back, faded past 120°), the wall-avoid `0x1401E6C10` (side rays 50 off, ±30° rays of 300, a 40-frame swing along the wall from its normal), the pitch assist `0x1401E6670` (ledge and fall looks, the eye's floor ray 55, 3 % back to 10°; **Manual too**), the eye walk (40 sphere; re-place past 700; jump at 1000), the visibility re-place `0x1401E7780` (5 segments), the auto recenter (eye within 40, pitch under 60°, moving), the distance 10 % back; Sora's speeds `0x1401E5D90`; the thresholds `0x1401E6080`.
- **Change** (`camera::kh1::Follow`, `win::camera`): all of it in KH1's order through Re:CoM's camera sweep `0x1401FBC60`; the wall probe `0x140196300` as the owner's (a): 8 flat 2.5 m rays from the look point and 8 from the eye, only while the swing runs; the wall normal from the sweep's shape (`cam+0x1A8 → +0x40`); Sora's KH1 state 0x15 in Re:CoM's glide, 2 in the air (Re:CoM's ground test `0x1401F3C00` read from its cache or its rule, never run), else 0; his air velocity from his mover `+0x80`. **Stage 1 fix**: the pitch assist also runs in Manual (KH1 does): the pitch goes back to 10° at 3 % a frame. Page: Camera's second row **Auto camera Off / On** (ini `auto_camera`, default on).
- No new hook, patch or site. Per frame at Camera = KH1, Auto on: about 5 Re:CoM sweeps standing, + 16 while the swing runs, + 2 at a wall-avoid's start (Re:CoM's follow: 1); not measured in game.
- Assumptions: the shape's `+0x40` is the contact's surface normal; one Re:CoM sweep equals KH1's 20-unit walk; KH1's flag `0x142867368 & 0x10` clear; KH1 states other than 0 / 2 / 0x15 not told apart; the label "Auto" for KH1's Config 0. L2 / R2: KH1's camera never reads them (Checked): no clash with KH1's L2 target step.
- Checked: tests (`camera::tests` 15 + the cost test: swing ramps and gains, wall ramp, eye walk and distance spring, visibility re-place, pitch assist, the Manual pitch return), `check_sites_rs.py` (1406 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game; no KH1 camera trace. Asset step: not needed. Docs: `recom-m12-camera-plan.md` § 14, `recom-config-menu.md`.

---

## Stage 246: M12 camera stage 1: a Settings > Camera page (Re:CoM / KH1) and KH1's follow camera as a new mode 6 of Re:CoM's own player camera, version 0.134.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Shop text, lock-on and camera: "Lets also have an option in the config to switch between Re:CoM camera, KH1 Camera, And KH2 camera."; § Camera answers): independent of Combat, a new Settings > Camera page (ini `camera`, default Re:CoM); sleights, summons and specials keep Re:CoM's camera moves then hand back through Re:CoM's restore; boss fights keep Re:CoM's boss cameras; after an event the camera re-places behind Sora; Re:CoM's inversion rows with KH1's own Normal; KH1's speeds and built-in numbers; Riku's story Re:CoM's; KH1's mouse rule on keyboard and mouse. Stage 1 of `docs/recom-m12-camera-plan.md`.

- **Re:CoM** (Checked (decomp + disasm + exe)): `CPlayerCamera`'s mode `+0x178` picks its record from the table `0x14087B660` (6 slots; read only in the setters `0x1401FC4D0` / `0x1401FBE60`); its field / battle execs `0x1401FEEB0` / `0x1401FECE0` are referenced only by the init `0x1401FB530`'s two `lea`s; sleights ask for 3 / 4 / 5 through `0x1401FBDE0` and give back `+0x17C`; the boss camera count `0x14087B468` (only with `BCam0000`); the drawn camera is the stack's slot 0 `0x140AC4370`.
- **Change** (`crate::camera`, `win::camera`): modes 6..9 with records in Re:CoM's format in a block near the exe; stubs at the two record reads (`sites::CAMERA_PATCHES` `0x1401FC513` 14 bytes, `0x1401FBE85` 11 bytes) read ours for 6..9 and count the mode set; the init's `lea`s (`0x1401FB556`, `0x1401FB587`) point at a dispatcher: Re:CoM's exec for its modes (at Camera = Re:CoM one load and one compare), our step for 6 then Re:CoM's own tail (`0x1401FC0D0`, the eye node, `0x14024FA40`); Re:CoM's 1 / 3 / 4 switch to 6 through Re:CoM's `0x1401FBE60` (never during a sleight's request or Re:CoM's view 2), `+0x17C` = 6. The init wrapper (`sites::CAMERA_HOOKS`, 7 bytes) gives 6 at a new camera when wanted. Riku's story and a battle with Re:CoM's boss camera stay Re:CoM's.
- **KH1 follow** (KH1's mode 1 `0x1401E1920`, Manual branch; Checked (KH1 Steam decomp + exe)): look point 1 m up and 0.5 m behind Sora springing 10 % horizontally, 5 % up / 25 % down a frame; distance 400, pitch 10°, FOV 53; the stick `0x1401E7320` (pad +0.075° a frame² to 2.5°, −0.12° on release; mouse +0.15° to 4.25°, stops on release), pitch −15°..+50°, KH1's radial dead zone; R3 (Re:CoM's reset bit 0x400) KH1's 60-frame recenter `0x1401E62A0` with its 30° wall steps / long way / swap; the eye's wall ray (60 past, 60 off the wall, look ≥ 20 away); KH1's re-place `0x1401E4820` behind Sora at a new camera and on the return from another camera, with the current view's yaw after a sleight's restore. Collision: Re:CoM's camera sweep `0x1401FBC60` (KH1's ray as a 0.005 m sphere; flags it sets put back).
- **Page** (`menu::Page::Camera`, the fifth Settings entry): Camera Re:CoM / KH1; KH2 and the Auto camera row come with their stages (`camera = kh2` warns and reads Re:CoM's).
- Per frame (measured): Camera = Re:CoM one atomic load and one compare; Camera = KH1 the KH1 math 76 ns a frame (Linux release, 10⁶ frames) and 2 Re:CoM camera sweeps against 1 in Re:CoM's follow.
- Assumptions: R3 is Re:CoM's 0x400; KH1's look offset's −z is behind Sora; Re:CoM's mouse feeds its right-stick axes (KH1's mouse multiplier: 1); the camera keeps updating under another camera; Re:CoM's sweep as KH1's ray. At this stage the KH1 follow stays on while locked on (stage 3).
- Checked: tests (`camera::tests` 12 + the ignored cost test, `sites::tests::camera_sites`, the page / list / ini tests), `check_sites_rs.py` (1406 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game; no KH1 camera trace (the recorder has no camera columns). Asset step: not needed. Docs: `recom-m12-camera-plan.md` § 13, `recom-config-menu.md`.

---

## Stage 245: Lock-on on R1 at Combat = KH1 on a controller (Re:CoM's own lock-on reading R1; keyboard / mouse keep Re:CoM's binding), version 0.133.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Shop text, lock-on and camera), verbatim: "Let's have the lock on button be mapped to R1 if combat is set to KH1. If on mouse and keyboard it stays the same." Combat = KH1 in Sora's story; Combat = Re:CoM and Riku's story: Re:CoM's.

- **Re:CoM** (Checked (decomp + disasm + exe)): its lock-on `0x1401E8DA0` (once a frame from the Field / Battle exes) reads its button from the player's pad object `+0x230 + 0x44`, through its three readers only (`0x1401FF740` pressed, `0x1401FF600` released, `0x1401FF4F0` held); the pad update writes `+0x44` = 2 every frame (`0x1401FF19D`; L2 1, **R2 2**, L1 4, R1 8 in Re:CoM's PS2 order): **Re:CoM's lock-on is R2**. R1 is card scroll right / sleight with L1, both off in a KH1 battle (`win::attack::request_bits`). Pad or keyboard: `0x140C2FF64` (1 keyboard / mouse), set each frame by `0x140347EF0`.
- **KH1** (Checked (KH1 Steam decomp + exe)): pad table `0x140528290` R1 `0x800` → logical `0x400`, the lock-on of `0x1402B0610` (R1 again unlocks; L2 steps through the targets).
- **Change** (`crate::lock_on`, `win::lock_on`): a wrapper on Re:CoM's lock-on (`sites::LOCK_ON_HOOK`, 6 bytes; no branch in: rel32 scan) writes `+0x44` = **8 (R1)** for KH1 Sora when the pad is in use (Re:CoM's 2 with the keyboard / mouse) before Re:CoM's own runs: Re:CoM locks, unlocks (press and release) and switches targets (held + right stick) on R1. R2 then does nothing at Combat = KH1 (KH1 gives it no lock-on or battle action).
- Not changed: Re:CoM's text prompts naming the lock-on button (message files not in the session; escape `F5 6B` → `0x14083F522` = R2's glyph: Assumption); KH1's unlock / target-switch rules (owner question).
- Sites: `LOCK_ON_HOOK`, `LOCK_ON_CHECKED` (7). Per frame: one atomic load and a compare in Re:CoM's lock-on; at KH1 one flag read and one store.
- Assumptions: Re:CoM's R1 / R2 are the default-bound shoulder buttons; the device flag is set before the lock-on in a frame.
- Checked: tests (`lock_on::tests`, `sites::tests::lock_on_sites`), `check_sites_rs.py` (1390 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-lock-on.md`.

---

## Stage 244: The Moogle's speech-bubble lines fit the bubble (wrapped onto its lines with Re:CoM's width routine; the true squeeze only when no wrap fits), version 0.132.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Shop text, lock-on and camera), verbatim: "The moogle shop text is over extending past the text bubbles." (screenshot: "Restores HP. 10 Moogle Points. Sora has 11" past both sides of the orange bubble). Decided: wrap onto the bubble's lines as Re:CoM's own two-line lines do; squeeze only if a line still can't fit. Combat = KH1 in Sora's story; Re:CoM's own lines untouched.

- **Cause** (Checked (decomp + disasm + exe)): the bubble is a `CTextWnd` sized from its message's record at its open (`0x1402B1520`: size `+0xB8` / `+0xBC`, text margins `+0xB0` / `+0xB4`); Re:CoM's text call `0x1402B0EF0` sets a new line on its `CText` (`+0xD0`, a `CStrPrim` at `+0x38`) and never resizes it. A new line is byte 10 (`0x14025A380`); nothing wraps on its own.
- **Change** (`crate::text_fit::wrap`, `win::shop::bubble_line`, `win::text_fit::{width_of, set_level, unsqueeze}`): our lines carry a `\n` where we prefer a break ("Restores HP." / "10 points. Sora has 11."; "Sell Potion x3" / "for 6 Moogle Points?"; Elixir "Restores HP and MP."); each is laid out for the bubble it goes in, measured with Re:CoM's width routine on that bubble's own text: our lines wrapped at the box, else the whole line wrapped, else the true squeeze (0.113.0) on the narrowest wrap. Box = the bubble's width − both margins − 2 units a side; lines = what its height − both margins holds at its size and gap, or Re:CoM's own line count there if more. The squeeze uses a copy of `CStrPrim`'s vtable `0x140508FD0` (same draw / measure as C2DFnt's); Re:CoM's vtable goes back when the bubble is handed back or Re:CoM sets another text on it (our measure sees it).
- Sites: `TEXT_FIT_STR_PTRS` (8, checked lazily), `BUBBLE_CHECKED` (4 byte sets). No hook or patch. Per frame: nothing new (the layout runs when a line changes).
- Assumptions: the margins equal on both sides; the bubble's drawn edge is its size; Re:CoM keeps our pointer for a line without `%` (our lines have none: test).
- Checked: tests (`text_fit::tests` wrap 5, `shop::tests::every_line_fits_a_two_line_bubble_at_the_longest_numbers` (107 owned, 200, a 4280 sale; a model bubble as narrow as Re:CoM's own two-line line: ≥ 0.72), `help_lines`, `sale_totals_and_lines`, `sites::tests::bubble_sites`), `check_sites_rs.py` (1382 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game (Re:CoM's fonts and the shop messages' records are not in the session: the log gives each bubble's box and each line's layout). Asset step: not needed. Docs: `recom-m8-items.md` § M8d: the bubble's lines fit; `recom-m12-sleights.md` § 8.8.

---

## Stage 243: KH1's swing effects and sounds really play (the swing's motion request passed variant 0): Gravity Break, Stun Impact, Zantetsuken, Ripple Drive and every KH1 swing with every Keyblade; the set check's entry count; a per-battle finisher log line, version 0.131.0 (2026-11-04)

Owner, 2026-11-04 (0.130.0, after Build), verbatim: "sora's gravity break finisher still doesn't have the proper sound effects or visual effects. Stun Impact also doesn't have its finished visual effects. Ripple drive doesn't have it's visual or sound effects at all." (ini `combat = kh1`, `effects = kh1`, `sounds = kh1`; Keyblade card 0xc.)

- **Cause** (Checked (code + files + decomp)): every KH1 swing (combos, finishers, air rows, Counterattack) is requested once, by our row pick `win::attack::apply_row` (`0x1401F7180(pl, row, 0, root, 0, 0, 0)`); `Kh1Sora::play`, the only caller of `win::fx::variants`, never requests a swing (`attack::row_exec` marks it issued). Re:CoM's motion start keeps only the events at the variant passed (`0x14017A240`), so since 0.109.0 every swing played Re:CoM's variant-0 events with every Keyblade: Gravity Break `p001_229` and Ripple Drive `p001_228` nothing; Stun Impact `p001_101` Re:CoM's `ex_0010` 0 only; Zantetsuken Re:CoM's `ex_0010` 4 / 5; the combo clips Re:CoM's `ex_5010` trail (the Kingdom Key's, the trail the owner saw); request 208 (`p001_103`) Re:CoM's Sonic Rave effects; Re:CoM's SES. Reproduced on local files (`eet::picked`, test `finisher_events_per_group_from_local_files`).
- **Not the cause** (Checked): the Keyblade groups (the EET variant is 1301 for every Keyblade; each `PL00GG.EET` and the shared `PL0002.EET` carry the trail and Gravity Break's blade 2 at 1301: local run on Ultima's group 34); where the sets are (an event's set is looked up by name among all loaded type-5 resources, `0x1402A4920` → `0x1402A43E0(5, …)`, list `0x140AC6730[type]`, no group scope; the id inside it, `0x1402553B0`; its `.ese` by the same name, type 0x3E, `0x1402A4A10`): `kh1_ex_0010.dpx` / `.ese` in `EF1000.EFF` serve any group, a group's `kh1_ex_5xxx.dpx` its own `PL00GG.EFF`.
- **The log's 2282216960 entries**: our check read `+0xC` of resource `+0x68`, an 8-byte wrapper whose first word is the set (`0x14029F9C0` → `0x140255100`; `0x1402553B0` reads `*wrapper + 0xC`). Fixed (`win::effects`). Checked (decomp).
- **Fix** (`win::attack::apply_row`): the swing's request passes `win::fx::variants(request, 0)`: Effects = KH1 → EET 1301; Sounds = KH1 → the equipped Keyblade's SES variant (1301 + card − 1, else the Kingdom Key's 1301); Re:CoM plays the Keyblade with the same variants. Re:CoM binds, spawns, steps and ends KH1's events. At Effects / Sounds = Re:CoM and Combat = Re:CoM: 0, unchanged.
- **Also** (`fx::Manifest::moves`): at Effects = KH1 the EET variant goes to every request with KH1 events (trails, effects or sounds), so 208 / 212 show none of Re:CoM's effects (KH1 has none there).
- **Log** (`win::fx::swing_started`, `fx::swing_line`): per battle, the first swing and each finisher's first play: the variants, the card, and the events Re:CoM bound for Sora and the Keyblade (entity `+0x108` EET player: group `+0x10`, count `+0x24`, slots `+0x830`), each with its set and `.ese` as loaded (existing checked lookups `0x1402A4920`, `0x1402A43E0`, `0x1402A49C0`, bytes compared once). Not per frame.
- No new hook, call site or byte site; nothing per frame (per swing: the variants, as `play`). Asset step: **unchanged** (no re-run, no `mod.yml` change; only a test helper `eet::picked` added).
- Assumptions: the KH1 events play as stages 3 / 0.112.0 / 0.114.0 assumed (start delays, `.ese` at the spawn, attach codes, ±1 frame).
- Checked: tests (`fx::tests` incl. the owner's 0.130.0 manifest through the parser: every finisher × every Keyblade gets EET 1301 and a KH1 SES variant; `the_swing_line_names_the_variants_and_what_re_com_bound`; `kh1_fx::tests::finisher_events_per_group_from_local_files` on Re:CoM's `PL0001/0002.EET`, `PL0002.MDL`, KH1's `xa_ex_0010.mset` / `.mdls`, `btltbl.bin`, `xw_ex_5160.wpn`), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Docs: `recom-m13-plan.md` § 0.131.0 fix (and § Stage 3 built corrected).

---

## Stage 242: M8d step 3: the sell screen as an Items tab first in Re:CoM's own card trade (Re:CoM's stock window fill from our card list, a count step, Re:CoM's yes / no; a six-cell strip from the asset step), version 0.130.0 (2026-11-04)

Owner, 2026-11-04 (answers to the step-3 question, relayed): "our own Up/Down count step on Re:CoM's yes/no, 1..owned, the points box shows the total, Re:CoM's yes/no sells them, stock first then slots"; the Items tab "uses the same green bottle icon, placed first in the trade's strip", help line "Sell items"; Re:CoM's green item icon, no texture edit; the Tent / Camping Set conversion unanswered (Elixir kept). Combat = KH1 in Sora's story; Combat = Re:CoM and Riku's story: Re:CoM's card trade.

- **Card trade** (step 2's hook `0x1403A73E0`; Re:CoM's mode 5, Checked (decomp + disasm + exe), `recom-m8-items.md` § M8d step 3): opens on an **Items tab first in the strip** (before ALL; in the grid when Sora has items). Sora's items as Re:CoM's item cards: a card list of ours in Re:CoM's `CMShopChargeCardData` layout handed to **Re:CoM's own stock window fill** `0x14045DA80` (as tab 4), Re:CoM's big card `0x14045A260` and small-card box `0x14045D9C0`. Left on ALL / Right on tab 3 → Items; from it Left → tab 3, Right → ALL (Re:CoM's tab move). Re:CoM's state 1 replayed for the list (`shop::sell`: its order, sounds, wraps, cursors); help bar "Sell items" (Re:CoM's text set `0x140258DE0`).
- **Count step** (Re:CoM's state 2 while the tab is up): Up / Down 1..owned (fresh press at an end wraps), the charge point window the total; confirm → **Re:CoM's yes / no** (message 7, Re:CoM's finger place; our line "Sell Potion x3 for 6 Moogle Points?"); Re:CoM's states 3 / 4 unchanged. **Sale** (state 5 while up): stock first, then slots (`shop::sell`), the total through **Re:CoM's own add** `0x140239FA0` and its point window; back to the grid (or the tab row with nothing left).
- **Six-cell strip** (asset step `shop_tabs::strip`): 6 groups after step 2's 13 (`m_shop:01` 254..259; Re:CoM's 241 kept): ALL's place split into the Items cell (Re:CoM's bottle) and a narrower ALL; Re:CoM's four tabs untouched; PS2 layout: a separator; PC layouts: pieces cut from Re:CoM's own as new sprite records (no texture edit). The DLL writes Re:CoM's strip table `0x1406904D0` to them and points Re:CoM's ALL finger instruction `0x14045B9D7` (62.0) at Re:CoM's own 50.0 (`0x140509298`) at each trade start (Re:CoM's at Combat = Re:CoM). Manifest format `shop-tabs 2`.
- Per frame: nothing outside the card trade; in it one mutex read, the pad read once more at ALL / tab 3, Re:CoM's state replayed on the Items tab.
- Asset step: **re-run** (no new `mod.yml` entries; an older manifest turns the shop art off).
- Assumptions: the fill / box read only Re:CoM's data fields (copied); the strip control's play arguments; the finger's x in strip coordinates; the help bar takes ASCII; every language's PC layout has the UK pieces (the step refuses otherwise). Sale returns to the grid (Re:CoM's returns to its value box).
- Checked: tests (`shop::tests` 21, `sites::tests::shop_sites`, `shop_tabs::tests` 4 incl. the owner's FO0109.SPR / UK_m_shop.lad, both rendered offline), `check_sites_rs.py` (1370 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings), the Linux `kh1-recom-build` (built; not run: no Re:CoM PKG in the session). Not checked: in game. Docs: `recom-m8-items.md` § M8d step 3, `kh1-recom-assets.md`.

---

## Stage 241: Enemy Skills stage 3: route 2, Re:CoM's hit rewrite run once per equipped skill (14 skills), version 0.129.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Enemy Skills plan answers; § Enemy Skills numbers: "Defender / Ursula −25 %", "Lexaeus 15 %, never bosses", "Zexion 10 %", "Demyx ×1.75", "build everything"). Combat = KH1 in Sora's story; Combat = Re:CoM and Riku's story: Re:CoM's rewrite as is.

- **Re:CoM** (Checked (decomp + exe)): the hit rewrite `0x14015D5F0(CBTL, record)` reads Sora's slot once from the global CBTL (`*0x14087A818 + 0x1E8`, `0x14015D60A`) and handles one card; its callers `0x1401E66A0` (swing / magic / sleight AP; situation `+0x630` 1..7 = attack card, 4 / 7 also the "sleight" flag), `0x1401E67B0`, Sora's damage check `0x1401E6DE0` (mode 2), `0x14015D8A0` (sleight shots), `0x1401F7520`, `0x14017F0E0` (enemy side).
- **Change** (`crate::skill_effects`, `win::enemy_skills`): a wrapper on the rewrite (new hook `sites::ENEMY_SKILL_REWRITE_HOOK`, 6 bytes; no branch in): a side-1 record in a KH1 battle with Sora's slot empty runs Re:CoM's rewrite once per equipped route-2 skill (card order), each with its id in the slot, put back after. The owner's numbers: Re:CoM's five constants (`sites::ENEMY_SKILL_REWRITE_SITES`: Zexion 25 → 10 %, Lexaeus 67 → 15 %, Demyx ×2 → ×1.75, Defender / Ursula ÷2 → ×0.75) switched at each battle start to stubs reading a cell: Re:CoM's value except while our wrapper runs (every i16 result Re:CoM's at Re:CoM's cells: tests), so enemy cards keep Re:CoM's. KH1's attacks: at Damage = Re:CoM swings and spells already reach it (`0x1401E66A0`); at Damage = KH1 each KH1 swing / spell hit's number goes through it with `0x1401E66A0`'s record (`kh1_swing_hit`, `spell_rewrite`; spells with their Re:CoM card's / sleight's status bits, `win::damage::spell_status`); KH1 Cure's heal at its cast (both settings, as `initMagic`). The attack-card skills (Search Ghost's Drain, Zexion, Dragon Maleficent ×1.5) act on KH1 swings, Lexaeus on KH1 finishers; Lexaeus never on a boss (class `CBoss…`: the swing's own kind back, `warp_guard`). Wizard / Crescendo: only their ×1.3 (their card dimming is play-time, no cards at KH1). Hades stays KH1's Berserk. Un-greyed: Red Nocturne, Blue Rhapsody, Yellow Opera, Green Requiem, Search Ghost, Defender, Wizard, Crescendo, Ursula, Dragon Maleficent, Lexaeus, Zexion, Xigbar, Demyx (help lines `skill_effects::BUILT_3`).
- Assumptions: Re:CoM's reaction kinds (0x15 / 0x1F / 10) as Re:CoM's handler applies them (our KH1-AI enemies may react with their KH1 reaction instead); boss = `CBoss…`; category 0x200 = water; several skills stack multiplicatively.
- Per frame: nothing; per rewrite call one atomic load (more only with route-2 skills equipped in a KH1 battle).
- Checked: tests (`skill_effects::tests::route_2_masks_and_records`, `the_rewrite_stubs`, `sites::tests::enemy_skill_rewrite_sites`, `enemy_skill_sites`), `check_sites_rs.py` (1348 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-enemy-skills.md` § Stage 3 (built).

---

## Stage 240: M8d step 2: the shop's buy screen on Re:CoM's own pack screen with an Items tab first (Re:CoM's two-Moogle menu unpatched; the M8c list gone; a five-tab art from the asset step), version 0.128.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Moogle shop redesign at KH1), verbatim: "the item shop looks REALLY bad. … you can keep how the original menu looks, without the "buy, sell, card packs" Ideally, selecting the left side (usually to buy card packs) it should bring up a menu similar to screenshot 4, with all "card packs" replaced with the item card graphics of each item sold."; answers "Items tab, then packs". Combat = KH1 in Sora's story; Combat = Re:CoM and Riku's story: Re:CoM's shop.

- **Removed**: M8c's Buy / Sell / Card Packs list and its mode-3 init hook (`0x1403A53F0`): Re:CoM's two-Moogle menu (mode 3) runs unpatched.
- **Pack screen** (hook `0x1403A6930`, 15 bytes; 8a.3's design): opens on an **Items tab drawn first** (state 0 run with no packs, then ours). Cells = Re:CoM's own packs (ctor `0x14045A9E0`, price plates, never sold out), the pack art hidden, Re:CoM's `C2DCard` of the item card (0x2A / 0x2B / 0x2D / 0x2F) on each; the floor's items (`shop::STOCK`: 1F Potion 10 / Ether 40, 2F+ Hi-Potion 30, 7F+ Elixir 200; one page). Re:CoM's input order, moves, sounds; one item per confirm by KH1's pickup; Re:CoM's payment. Left on tab 0 / Right on tab 3 → Items; from it Left → tab 3, Right → tab 0. The Moogle's line on the Items tab through Re:CoM's text call: "Buy items with Moogle Points." / the item, price and count / the refusal. Checked (decomp + disasm + exe).
- **A full deck**: Re:CoM's mode-2 rule kept; at Combat = KH1 the card trade's Square (hook `0x1403A73E0`, 12 bytes, no branch in: exe scan) opens the pack screen also without room for 5 cards, with its button hint made as Re:CoM's case 5 does (`0x1403A8561`); the packs refuse as Re:CoM's.
- **Five-tab art** (asset step `kh1-recom-assets` `shop_tabs`): 13 groups appended to `m_shop:01` of `FO0109.SPR` and every `<LANG>_m_shop.lad` (Re:CoM's 241 kept): 5 windows / closes 52 apart (tab 0's notch moved), the icons with **Re:CoM's own green item icon** first (the card trade's item tab sprite, × 2.5); no texture edit. The DLL (`kh1-shop-tabs.json`) writes Re:CoM's tab x table `0x14065BA40` and window table `0x140690420` to the art's places at each pack screen (Re:CoM's values at Combat = Re:CoM; their only readers are the shop: exe scan), sets the icon sequence's groups and plays the place-0 window on the Items tab. Without the art: Re:CoM's four tabs, the Items cursor at x 320. Checked (files: the owner's FO0109.SPR / UK_m_shop.lad parsed, patched, rendered offline).
- Not built: a Potion card face as the tab picture (needs an `m_shop.imz` texture edit, PS2 + HD: owner question); the bottom help bar's text (message 0x580B00, file not in the session).
- Per frame: nothing outside the pack screen and the card trade.
- Asset step: **re-run**. New outputs `raw/FORM/0109/FO0109.SPR` and `kh1-shop-tabs.json`: **2 new `mod.yml` entries** (`recom-m8-items.md` § M8d step 2).
- Assumptions: the window control's play arguments (Re:CoM's own); the icons' groups read after their set-up; the help bar holds no card-pack words.
- Checked: tests (`shop::tests` 16, `sites::tests::shop_sites`, `shop_tabs::tests` incl. the owner's files), `check_sites_rs.py` (1318 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings), the Linux `kh1-recom-build` (built; not run: no Re:CoM PKG in the session). Not checked: in game. Docs: `recom-m8-items.md` § M8d step 2 (M8c marked replaced), `kh1-recom-assets.md`.

---

## Stage 239: Enemy Skills stage 2: route 1, the play-time setters (14 skills through Re:CoM's own enemy-card writes and readers), version 0.127.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Enemy Skills plan answers: "Go ahead", stage 2; § Treasure Magnet: "Re:CoM's Bouncywild radius"; § Enemy Skills numbers: "oogie- (every 6s) large body - (reduce damage from front half by 10%) … build everything"). Combat = KH1 in Sora's story; Combat = Re:CoM and Riku's story: Re:CoM's enemy cards untouched (nothing applied, every switched site Re:CoM's).

- **Re:CoM** (Checked (decomp + exe)): the player-side apply `0x140163C00` (its `+0x420` bits, Draw's `+0x54C` / `+0x550`, the Vanish `0x1401F7EC0` / Aero Guard `0x1401F6FB0` setters; bytes in `sites::ENEMY_SKILL_CHECKED`), the Sora-side CBTL flags of `0x140163170` (Bio `0x2000`, Regen `0x400`), the clear `0x140161E10`, the readers (`procAttackCollision` `0x1401F5DC0`, the damage check `0x1401E73C3`, the CBTL update `0x1401617F0`, the prize pull `0x140230C70`).
- **Change** (`crate::skill_effects`, `win::enemy_skills`): for each equipped skill, Re:CoM's writes repeated on the player at hand (never the affinity call `0x1401F8810`), at the first KH1 frame of each player object (one atomic compare a frame), after every Sora-side clear (new hook `sites::ENEMY_SKILL_HOOKS` on `0x140161E10`, 15 bytes, not at the battle's end), and when the page's equip or Combat changes (then also taken off). Field rooms: Draw only. Built: Bouncywild and KH1's level-72 Treasure Magnet (10 m), Large Body (Re:CoM's own barrier values and test for one call, 10 % cut, no block), Bandit (KH1's combo length 1: kh1-core `Abilities::combo_finish`), Fat Bandit (Re:CoM's back hit; × 2 carried over at Damage = KH1), Wight Knight (Sora's own gravity × 0.75: the KH1 controller's, Re:CoM's read at `jump_height = recom`; not the global CBTL bit), Gargoyle, Neoshadow (1 s), Guard Armor (× 1.5: Re:CoM's scale 0.75, our KH1 sphere radii by Re:CoM's live scale), Card Soldier (Re:CoM's motion speed `0x1401F74D0` at 1.25 after our row pick's request), Oogie Boogie (6 s), Xemnas (1 s), Xaldin, Saïx (+20 a combo step, from 0 at each combo), Roxas (second strike at half power: a stub on its power read `0x140225D7C`). The owner's numbers: `sites::ENEMY_SKILL_PATCHES` (Regen `0x14016189D` / `0x1401600F2` 6000 → 18000, Bio `0x140161991` 1000 → 3000, barrier `0x1401E73F3` 6000 → 3000, wide scale `0x140225802` 1.0 → the pool's 0.75 `0x1406C10A8`) and the Roxas stub, switched at each battle start (as M6's). Un-greyed: the 14 (help lines in `skill_effects::BUILT`); Dragon Maleficent waits for stage 3 (its play-time part is the reload charge).
- Not built: Bouncywild's scattered-card test `0x14022F23A` (no card is scattered at KH1: the revolver is empty; Checked (decomp)).
- Assumptions: Float keeps KH1's jump height (KH1 defines it by height) and Re:CoM-run airborne states keep Re:CoM's gravity; KH1 swing windows follow Re:CoM's motion clock under the haste; the second strike's damage is its `+0x288`; class 0 = physical for Large Body. At Damage = KH1: Saïx does nothing, Xaldin's wind / Bio / Roxas' second strike carry Re:CoM's numbers (owner questions).
- Per frame: one atomic compare in the KH1 frame; the damage check one atomic load (two calls of Re:CoM's while Large Body is on and a hit arrives); the pair dispatch two loads; the swing start one load.
- Checked: tests (`skill_effects::tests` 6, `sites::tests::enemy_skill_sites`, `movement::tests::combo_finish_makes_every_swing_a_finisher`, the stage-1 tests), `check_sites_rs.py` (1330 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-enemy-skills.md` § Stage 2 (built).

---

## Stage 238: Enemy Skills stage 5: the deck skills' KH1 meanings (Tornado Step, Aquatank, Barrel Spider, Air Soldier, Jafar, Air Pirate, Riku, Marluxia, Luxord, Black Fungus), version 0.126.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Enemy Skills numbers): "build everything" (stages 2–5); the brief's meanings for the deck cards (plan stage 5). Combat = KH1 in Sora's story; Combat = Re:CoM and Riku's story unchanged.

- **Code** (`crate::enemy_skills_deck`, `win::enemy_skills_deck`): armed once per battle in the battle manager's ctor wrapper (before stage 4's arming) from the equipped skills and Black Fungus's; one call each in our own `win::mp::pay` / `on_kh1_hit`, `win::attack::damage_check` / `request_bits`, `win::kh1_enemy::swing_meets_attacks`, `win::summons::sleight_running` (+ `replay_sleight`), `win::abilities::equipped_live`, `crate::sleights::pay_points`. No Re:CoM code patched, no new site. Per frame: one atomic load unless Air Soldier is armed or a second play waits.
- **Tornado Step**: KH1's hit charge (`0x1402BF330`) + 0.75 on its factor (ours; MP Haste +1.5), KH1's 250 % cap (swing 3 → 5, finisher 6 → 10).
- **Aquatank**: once a battle, the payment that leaves 0 MP points: KH1's full restore (`0x1402A56F0`).
- **Barrel Spider**: once a battle, the payment that empties the charge gauge below max MP: the gauge fills to the next point; KH1's refill makes it the next frame.
- **Air Soldier**: 1 charge unit per 20 frames running in battle (Re:CoM's run action 4; ours: 3 a second).
- **Jafar / Air Pirate**: a hit during a KH1 swing (action 0xA with a KH1 row) / a KH1 item use (action 0xE) takes Re:CoM's no-reaction bit (KH1's roll no-flinch path): damage, no flinch.
- **Riku**: sleights 1 MP point less, at least 1, Party and all-MP ones too (`sleights::pay_points_with`).
- **Marluxia**: a Sleights-list sleight plays twice: Re:CoM's own sleight start again, free, the frame after the first ends (from stand / walk / run within 30 frames; a hit, KO or win drops it). Re:CoM's bit `+0x428 & 0x2000` not used: its replay is the request bits `0x1401F8150`'s battle branch, which KH1 battles skip (Checked (decomp); `recom-enemy-skills.md` § Stage 5).
- **Luxord**: Sora's KH1 swing wins every clash with a KH1 enemy's attack row: the enemy gets KH1's clash record (kind 7 / 6 by KH1's 40 rule) and its row stops; Sora gets none and hits on.
- **Black Fungus**: at each KH1 battle start one random built Enemy Skill not equipped (Re:CoM's random float `0x14029CF00`; any card, as Re:CoM's Random Flush) counts as equipped that battle (logged).
- Abilities page: the ten un-greyed, help lines updated. Every Enemy Skill is now built except Gold Card (and stages 2–3's, on their own branch).
- Assumptions: Re:CoM's sleight start takes the same sleight a frame after its end; Black Fungus's skill acts through every reader of `equipped_live`.
- Checked: tests (`enemy_skills_deck::tests`: Tornado on KH1's charge, refills through KH1's refill step, the running rate, Riku through `pay_points_with`, Luxord, Black Fungus's pool and pick, the replay rule; `enemy_skills::tests`), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-enemy-skills.md` § Stage 5 (built).

---

## Stage 237: Enemy Skills stage 4: Re:CoM's direct compares answer for the equipped skills (Quick Recovery, Dash ×1.25, Auto-Life once, Invincible 5 hits, Sleightblind's resistance); Gold Card greyed, version 0.125.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Enemy Skills numbers): "platinum - (first 5 hits) build everything"; the proposal's Larxene "Dash: run ×1.25", Vexen "Auto-Life: once per battle, ¼ HP". Plan stage 4 (route 3, research § 2.3). Combat = KH1 in Sora's story; Combat = Re:CoM and Riku's story unchanged.

- **How** (`crate::enemy_skills_cmp`, `win::enemy_skills_cmp`): each compare `slot == id` also answers true while the skill is armed: a byte-checked patch per site jumps to a stub next to the exe that runs the moved bytes and Re:CoM's own outcome (a slot read gets the skill's id when no enemy card is there; a join gets the card's write). Stubs read one byte per skill and Platinum's hits left (absolute address) and compare the player with `*0x14087B380`. Armed once per battle in the battle manager's ctor wrapper from the equipped skills. Nothing per frame.
- **Axel, Quick Recovery**: the damage check's slot read `0x1401E7381` → 0x6F: Re:CoM sets `player+0x420` bit 0x4000 (`0x1401E73A8`) and its hit-reaction exe `0x1401EBD80` lets Sora act out of the reaction at once (`0x1401E93A0(pl, 1, 0)`). Checked (decomp + exe).
- **Larxene, Dash**: Re:CoM's stick class join `0x1401E967F`: its factor × 1.25 in battle (Re:CoM's card: × 1.7, `0x1405A08BC`); KH1's run (both `run_speed` settings) × the same through `movement::speeds::RecomNumbers::run_factor` and `win::movement::stick_class`. Owner's number.
- **Vexen, Auto-Life**: the slot reads of the KO init `0x1401F1A7A`, the death sequence `0x140211027` and the game-over controller `0x1401A1A07` → 0x71 while available: Re:CoM's own revive (KO exe `0x1401EC250` sub 3: max HP ÷ 4 = the owner's ¼, 1 s untouchable). Its count call `0x1401EC369` (`0x14015E010(1, 0x28, 0x2E)`) → ours: used for the battle. Once per battle (owner).
- **Platinum Card, Invincible**: the damage check's join `0x1401E71FA` and the HP loss's store `0x1401F9163` (`0x1401F90B0`, Zexion's drain): HP kept for the first 5 hits each battle that would take HP (owner), `lock dec` in the stub; reactions still play.
- **Ansem, Sleightblind** (no deck meaning at KH1): Re:CoM's Ansem affinity record `0x1407C5C50` = 100 / 100 / 50 / 50 / 50 / 100 % (the khwiki's fire / ice / thunder resistance; written by the card's play `0x140163C00` → `0x1401F8810`, read by the damage check `0x1401E7150`). The read's join `0x1401E7163` gives 50 for attributes 2..4 where Re:CoM's value is the default 100. The owner's "boss skills give the skill only" lifted for Ansem alone (brief).
- **Gold Card, Premium Guard**: left greyed ("no meaning without a deck yet"): owner question.
- Sites: `enemy_skills_cmp::PATCH` (8), `CALLS` (1), `CHECKED` (13; in `sites::all_sites`); `check_sites_rs.py` 0 mismatches. Exe scan: no branch or pointer into any moved bytes after their first. Installed after Second Chance (Invincible's first site is in its checked context).
- Abilities page: the five un-greyed with new help lines; Gold Card "(not built yet)".
- Assumptions: Re:CoM's KO / revive flow as in Re:CoM for KH1 Sora; KH1 enemies' fire / ice / thunder hits carry Re:CoM's attributes 2..4; Quick Recovery's commands reach our KH1 handlers.
- Checked: tests (`enemy_skills_cmp::tests`: every stub's layout, jumps and reach, the rules, Ansem's record, sites apart), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-enemy-skills.md` § Stage 4 (built).

---

## Stage 236: M8d step 1: the Tent and the Camping Set removed from the game (saves converted once to Elixirs), version 0.124.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Moogle shop redesign at KH1), verbatim: "just remove tents and camping sets from the game". Shipped first, on its own, before the shop screens (M8d). Combat = KH1 in Sora's story (the only time the store is read); Combat = Re:CoM and Riku's story unchanged.

- **Removed as the Mega items were in 0.105.0** (`crate::items`): `id::REMOVED` now holds the Mega items, the Tent (142) and the Camping Set (143); `id::IN_PLAY` = Potion, Hi-Potion, Ether, Elixir. Not sold (`shop::STOCK` = the four: 1F Potion / Ether, 2F+ Hi-Potion, 7F+ Elixir; Lucky Bounty's tiers unchanged), not dropped (the Barrel Spider's KH1 1 % / 0.5 % give `Dropped::Removed`: nothing), never picked up (`Inventory::pick_up` refuses a removed item: nothing can add one), not usable (`items::effect` none; `can_use` false; `camp_only` / `CAMPING_SET_FULL_MP` gone), not listed (the Items page's Stock / Equip lists and the shop's Sell list read `IN_PLAY`).
- **Owned ones** (checked: the Mega conversion `items::convert_removed` turned each into its plain item, in its slot or added to that item's stock up to 99, the rest lost and logged): the same function now runs on a **v1 or v2** block, and the block becomes **v3** (`ext::VERSION`, `EXT_V3`; every write v3; v0 stays KH1's new game; older DLLs read it as before). **Tent → Elixir, Camping Set → Elixir** (decided: the Mega rule, the item in play that does at least what it did for Sora: only the Elixir fills HP; the Camping Set was built as full HP and MP, the Elixir's effect). The Tents only ever sat in the stock, so they go to the Elixir stock (to 99; the rest lost, logged: `items: save block v2 -> v3: …`). A v1 block gets the Mega and Tent conversions at once.
- Kept: KH1's rows 142 / 143 (`DATA`), their stock bytes 8 / 9 (read for the conversion), their English names and the asset step's words 172 / 173 (unused; no asset re-run).
- No new hook, site or patch; nothing per frame (one byte compare in the store's read, as before).
- Assumption: a save opened in an older DLL (0.105.0..0.123.0) after this one could buy Tents into a v3 block, which is never converted again (they would stay in the stock bytes, unlisted).
- Checked: tests (`items::tests::the_tents_are_kh1s_rows_142_and_143_and_removed`, `a_saves_removed_items_are_converted_once` (v1 and v2), `items_page::tests::the_removed_items_never_show_or_work`, `shop::tests::stock_by_floor` / `the_tents_are_neither_bought_nor_sold`, `drops::tests::only_the_items_in_play_become_prizes`, `party::tests::lucky_bounty_brings_the_floors_items`), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-m8-items.md` § M8d step 1.

---

## Stage 235: Lucky Bounty at KH1 brings 1–2 KH1 items per find (two at 50 %), version 0.123.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Gravity and Lucky Bounty answers, verbatim "1 1-2 items"), on 0.118.0's question (one item, or Re:CoM's 3 / 6 / 9 by level).

- **Re:CoM** (Checked (decomp), 0.118.0): Pluto's card find deals 3 / 6 / 9 cards by level (`0x140166930`); the Party list plays only LV3 (0x56): no split of its own, so the owner's 1–2 at even odds.
- **Change** (`party::lucky_bounty::count` / `find`, `win::party::pluto_find`): after Re:CoM's find, outcome 2 in a KH1 battle brings **two items when Re:CoM's random float is under 0.5, else one**; each picked on its own from the floor's tier (`pick`, Re:CoM's random float each), each a KH1 item prize of its own from his prize point (Re:CoM's own scatter for each), Re:CoM's `pluto` effect once. No new hook or site; nothing per frame. Combat = Re:CoM, Riku's story, outcomes 0 / 1: Re:CoM's.
- Assumption: Re:CoM's random float is uniform on [0, 1).
- Checked: tests (`party::tests::lucky_bounty_brings_the_floors_items`), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-command-pages-party.md` § 1–2 items per find.

---

## Stage 234: KH1's own hit flatten at Damage = KH1 (KH1's numbers in Re:CoM's own flatten state), version 0.122.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Gravity and Lucky Bounty answers, verbatim "2 yes", on 0.117.0's open question: "KH1's own hit flatten at Damage = KH1"): a Gravity-hit enemy flattens as in KH1.

- **KH1** (Checked (decomp)): reaction kind 8 → `0x1402B7DE0`: on the ground request 0x66, timer `+0x404` = 68; in the air / a special movement movement 0x16 / 0x17, request 0x68, timer 75. `0x1402B7260` each frame: timer − step; above 60 the height `+0x480` = 0.1 + 0.9 × (t − 60) / 8 (ground) or / 15; at or under 60 0.1; at 0 request 0x67 (queued, `0x14029F980`); during 0x67 h += (1 − h) × 0.3; after it +0.05 a frame (`0x140297FE0`). Frame step 1.0 a 60 Hz frame = 50 Re:CoM ticks (`recom-timing.md`).
- **Change** (`magic::flatten::kh1`, `win::magic::kh1_flatten_hit` / `flatten_exe` / the begin and end wrappers, `win::attack::spell_hit`; new hook `sites::FLATTEN_EXE_HOOK` 6 bytes on Re:CoM's flatten exe `0x140183BA0` (no branch in: exe scan) + `FLATTEN_EXE_CHECKED` its rest): a KH1 Gravity strike at Damage = KH1 that Re:CoM's hit handler gave its flinch (next state `+0x780`) is re-picked with Re:CoM's own picker `0x14017D260(enemy, 0xD)` → Re:CoM's flatten state `+0xAC0` (KH1's damage unchanged). Its begin sets Re:CoM's timer to KH1's 68 / 75 × 50; its exe runs KH1's squash in phase 0, Re:CoM's phase 1 wait as KH1's 60-frame hold, Re:CoM's get-up motion with the height kept at 0.1 then KH1's +30 % a frame; the end springs +0.05 (0.117.0). Hurt bodies stay on (0.117.0): a hit's reaction knocks it out. Ground / air by Re:CoM's floor contact and flier flag. Per frame: one atomic load per flattened enemy update; a KH1 flatten one lookup and one height write. Combat = Re:CoM, Damage = Re:CoM, Riku's story: unchanged.
- Assumptions: Re:CoM's flinch decision stands for KH1's per-actor reaction rules (bosses Re:CoM flinches flatten); Re:CoM's "off the floor or a flier" for KH1's air / special movements; KH1's queued 0x67 applied by the next frame (±1); Re:CoM's own get-up motion (decision 10). Not carried: KH1's 40-a-frame drop and the hold's pause above the floor (Re:CoM's own gravity in its flatten moves it).
- Checked: tests (`magic::tests::kh1_flatten_in_recoms_state_is_kh1s_frame_by_frame`: 100 frames against KH1's timeline, both variants; `sites::tests::flatten_sites`), `check_sites_rs.py` (1312 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game; no KH1 trace. Asset step: not needed. Docs: `recom-m7-magic.md` § 7.8 (KH1's own hit flatten at Damage = KH1).

---

## Stage 233: KH1 Gravity's sphere drawn at Re:CoM's Gravity size (Re:CoM's own growth and draw rule on KH1's radius and frames), version 0.121.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Gravity and Lucky Bounty answers, verbatim "3 make it the size of re:com gravity"): the sphere is drawn at Re:CoM's Gravity size (over 0.115.0's KH1 size); the hit radius stays KH1's.

- **Re:CoM** (Checked (decomp + disasm + exe; files: the owner's `EF2040.EFF`)): the sphere is `grav_all` 3 / 4 / 5 (set slot `+0x48` of sets 18–20), the same programs as KH1's `xm_gravity01..03.mag` sphere (program 3; same lengths and values, some records 8 bytes later). Re:CoM's size rule (`CHomingBullet` init `0x140158660` / growth `0x140156350`, flag bit 7): radius += `desc+0x1C` an update, **drawn scale = radius ÷ `desc+0x78`**; its Gravity case gives `+0x78` = 1.5 m (row `+4`) and `+0x1C` = 0.04 / 0.06 / 0.08 m (row `+5`), starts at Sora's floor point under the target, moving down 0.05 m a frame, attack off until its fade.
- **Not settled**: how long Re:CoM's own sphere grows (until its mover's floor contact, or 8 m): first frame (≈ 0.08 / 0.12 / 0.16, smaller than 0.112.0's "really small") or never (160 frames). Its own per-level final scale is not used: owner question.
- **Change** (`magic::gravity::sphere_flags` / `growth_step_m` / `recom_effect_scale` / `draw_factor`, `win::magic::release_gravity`): our sphere keeps Re:CoM's flags `0x41E1` (bit 7 given back), Re:CoM's `+0x78` (its row's 1.5 m) and a step `+0x1C` = KH1's hit radius ÷ (N + 2): Re:CoM grows and draws it, linear on KH1's fall frames, at **KH1's hit radius ÷ 1.5 m** at the impact = KH1's scale × 1.133 (1.133 / 1.7 / 1.813; drawn radius 1.7 / 2.55 / 2.72 m, KH1's own draw 1.5 / 2.25 / 2.4 m). Our per-frame scale write (`grow`) removed: less of ours per frame. KH1's timing (2.4 m up, the fall, the impact), the hit radius and the strikes unchanged. No new hook, site or patch. Combat = Re:CoM: Re:CoM's Gravity unchanged.
- Assumptions: one bullet update per 60 Hz frame, the first after the release (±1 step, ±1.6 %); the 8-byte layout shift in Re:CoM's copy changes no size; about 2 more growth steps after the impact (+3 %; KH1's × 1.02 fade not carried).
- Checked: tests (`magic::tests::gravity_sphere_draws_at_recoms_rule_on_kh1s_radius`: flags, Re:CoM's rows, the step, the factor, the sizes), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-m7-magic.md` § 7.8 (The sphere's size (0.121.0)).

---

## Stage 232: While a sleight, special or summon runs in the field: the camp stays shut; no contact battle while it dims the room, version 0.120.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Party and field follow-ups): "While a sleight runs in the field: the camp is blocked, and a room-dimming sleight's i-frames apply in the field too".

- **Re:CoM** (Checked (decomp + disasm)): the camp gate step `0x1401A96C0` opens the camp only after the field menu gate `0x1401AAA90` (its only caller, the call `0x1401A9759`); a field Heartless's update `0x14019E020` runs, outside pause mode 4, its touch test with Sora and then the contact encounter `0x14019BED0` (the call `0x14019E262`; its other caller is the struck state `0x14019E8F0`, the field strike's battle). Field Heartless never damage Sora in the field.
- **Change** (`sleights::busy` / `camp_blocked` / `field_contact_shield`, `win::field_sleights::camp_menu_gate` / `field_contact` / `install_field_rules`; `sites::FIELD_RULE_CALLS` (2 calls, stubs) / `FIELD_RULE_CHECKED`): at Combat = KH1 in Sora's story in a field room, while Sora's action is 0x16 (sleights, summons) or a KH1 special runs, the camp gate's menu gate answers 0 (our field notices wait too); while that sleight also has the room dimmed (`+0x418` bit 31: the battle's i-frame rule), a field Heartless's contact starts no battle (**decided**: "untouchable" in the field = no contact battle-start; the enemy keeps roaming, its bodies on). A sleight's own field strike still starts its first-strike battle. Combat = Re:CoM, Riku's story, battles: Re:CoM's.
- Per frame: the camp gate's call (after Re:CoM's cheaper tests) and touching Heartless's contacts, a few loads each.
- Assumptions: the camp opens in the field only through that gate; a KH1 special's dim is Re:CoM's bit 31 as in battle.
- Checked: tests (`sleights::tests::field_rules_while_a_sleight_runs`, `sites::tests::field_rule_sites`), `check_sites_rs.py` (1310 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-field-sleights.md` § While a sleight runs in the field.

---

## Stage 231: KH1 Gravity in the field strikes field Heartless (Re:CoM's plain field strike: the battle with first strike), version 0.119.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Party and field follow-ups): "KH1 Gravity in the field: strikes field Heartless (Re:CoM's field strike: the battle starts with first strike)".

- **Re:CoM** (Checked (decomp)): the field Heartless hit handler `0x14019E020` switches on the hit's kind: 7 / 0x2A → `+0x440`, 8 → frozen `+0x400`, 0x1C..0x1E / 0x24..0x26 → paralysed `+0x3C0`, 0x29 → nothing; **every other kind → the struck state `+0x380`** (the battle starts with first strike). KH1 Gravity's kind in a field room is the KH1 row's 1 (Re:CoM's 0xD only in battle rooms): a plain strike; `magic::field_kind` keeps it.
- **Change** (`win::magic::gravity_command` / `release_gravity` / `gravity_frame`), as 0.110.0 routed Thunder and Stop: the command's target and the release's point take a field Heartless (`target_enemy`), the impact's list Re:CoM's lock-on targets with field Heartless (`lock_on_targets`), each struck once through Re:CoM's contact and apply (`strike_body`). Battle enemies unchanged. No new hook, site or patch; nothing new per frame.
- Not carried to field Heartless: KH1's squash under the falling sphere (battle enemies only).
- Checked: tests (`magic::tests::gravity_is_a_plain_field_strike`), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-field-sleights.md` § Built, What a field hit does.

---

## Stage 230: Lucky Bounty at KH1: Pluto's card find brings a KH1 item of the floor's tier (a KH1 item prize, KH1's pickup), version 0.118.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Party and field follow-ups): "Lucky Bounty at KH1 brings a random KH1 item (Potion, Hi-Potion, Ether, Elixir by the floor's tier) instead of a deck card (the deck is empty at KH1)".

- **Re:CoM** (Checked (decomp + exe)): Pluto's find `0x1401E2E60` acts on the outcome his action's begin rolled into `+0x472` (`0x1401E2B50`, weights `0x1407C5960`: 5 / 5 / 90 % with fewer than 7 cards): 0 a shot, 1 HP / Moogle prizes, 2 cards from the deck (`0x140166930`: 0 at KH1, nothing).
- **Change** (`party::lucky_bounty`, `win::party::pluto_find`, `win::drops::spawn_item_prize`; hook `sites::PLUTO_HOOKS` (9 bytes; only two direct callers, no branch in: exe scan), `PLUTO_CHECKED` (outcome switch, outcome 2's deal call, `0x1401D9DB0`)): after Re:CoM's find, in a KH1 battle, outcome 2 brings one KH1 item, equally likely among the floor tier's (shop `STOCK` tiers via `0x1402337C0`: 1F Potion / Ether, 2F+ Hi-Potion, 7F+ Elixir; never Tent / Camping Set), by Re:CoM's random float; Re:CoM's `pluto` find effect 1; a KH1 item prize (kind 0xD8, Re:CoM's look) thrown from his prize point; Re:CoM's pickup → KH1's rule (first empty slot, else the stock). Outcomes 0 / 1 Re:CoM's. Combat = Re:CoM, Riku's story: Re:CoM's. Nothing per frame.
- Decided without asking: one item per find (Re:CoM deals 3 / 6 / 9 cards by level): owner question.
- Assumptions: a KH1 battle's deck deals 0; `pluto` effect 1 reads as the find.
- Checked: tests (`party::tests::lucky_bounty_brings_the_floors_items`, `sites::tests::pluto_sites`), `check_sites_rs.py` (1303 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-command-pages-party.md` § Lucky Bounty at KH1.

---

## Stage 229: A Gravity-flattened enemy stays hittable in KH1 battles (Re:CoM's flatten keeps its hurt bodies; KH1's spring-back when a hit cuts it), version 0.117.0 (2026-11-04)

Owner, 2026-11-04: "Also when the enemy is flattened from gravity, sora is unable to attack them, lets fix that".

- **Cause** (Checked (decomp + exe)): at Damage = Re:CoM (default) KH1 Gravity carries Re:CoM's kind 0xD → Re:CoM's flatten state `+0xAC0` (`0x14017D260` case 0xD). Its begin `0x14017C350` turns every hurt body off (`0x1401A2D70`) and every attack body off until its end `0x14017EB80` (about 3 s): nothing can hit it, in Re:CoM too.
- **KH1** (Checked (decomp)): the flatten (`0x1402B7260`, movement 0x16 / 0x17, request 0x66) never makes the actor unhittable (its test `0x140292070`: flags and the body mask, set only by AI-script syscalls); a flinch on it knocks it out (`0x1402B8E10` → `0x1402B8B90`, request 0x4E / 0x4F) and its height springs back +0.05 a frame (`0x140297FE0`).
- **Change** (`magic::flatten`, `win::magic::flatten_begin` / `flatten_end`; hooks on both, `sites::FLATTEN_HOOKS` 11 bytes each + `FLATTEN_CHECKED` the rest of both functions; no branch into the moved bytes, 78 table references each: exe scan): in a KH1 battle, after Re:CoM's begin, Re:CoM's own hurt-bodies-on `0x1401A2DB0(e, −1)` (attack bodies stay off): swings, spells, shots and sleights land through Re:CoM's hit handler; its reaction ends the flatten. A flatten ended before Re:CoM stood it up springs back at KH1's 0.05 a frame (the existing squash list). Per frame: nothing new outside a spring-back (≤ 18 frames). Combat = Re:CoM, Riku's story: Re:CoM's rule.
- Assumptions: Re:CoM's flinch stands for KH1's knock out of the flatten; Re:CoM's base height 1.0. At Damage = KH1 there is no flatten (KH1's own hit flatten not built: open question).
- Checked: tests (`a_cut_flatten_springs_back_at_kh1s_rate`, `sites::tests::flatten_sites`), `check_sites_rs.py` (1299 sites, 0 mismatches), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-m7-magic.md` § 7.8 (The flatten keeps its hurt bodies).

---

## Stage 228: The camp's Settings list gets "Effects & Sound" (Effects, Sounds: KH1 / Re:CoM), version 0.116.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § M13 and camp text answers): "The Effects / Sounds (later Voices) toggles go on a **new Settings page** (e.g. "Effects & Sound")". Sora's story; Riku's story unchanged.

- **Settings list** (`keyblades::sub::SETTINGS_ROWS`): Game Settings, Combat, Movement, **Effects & Sound** (Re:CoM's Deck-submenu buttons, 4 of its 7). The page (`menu::Page::EffectsSound`) is our settings pages' own: Re:CoM's Config screen with our rows (`win::menu::SettingsPage`), pair rows **Effects** and **Sounds**, Re:CoM / KH1, written by `settings::store` to the same ini keys `effects` / `sounds` (the ini template's comments name the page). Voices: left out until voices exist.
- **When**: at once: `win::fx::variants` reads both at each of Sora's KH1 motions (`set_item` needs no call); only at Combat = KH1.
- **Kept and documented** (owner question): an effect's sound rides on the effect (Re:CoM's `.ese`, `0x1402546E0`). At Effects = KH1, Sounds = Re:CoM, KH1's finisher effects keep KH1's own effect sounds (81–88, bank 9000 registered at every KH1 battle start); at Effects = Re:CoM, Sounds = KH1, Re:CoM's effects keep Re:CoM's. The KH1 Keyblade trails have no sound (Checked (files)).
- The label's `&` passes Re:CoM's escape expander as is (`0x1403480C0` copies bytes under 0x81; Checked (decomp)); Assumption: Re:CoM's font has the glyph.
- No hook, call site or byte site; nothing per frame. Asset step: not needed.
- Checked: tests (page rows, labels, keys, ini round trip, cycling, help lengths, the Settings list), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Docs: `recom-config-menu.md`, `recom-m13-plan.md` § The Effects & Sound page built.

---

## Stage 227: KH1 Gravity's sphere drawn at KH1's size (Re:CoM's own sphere growth off), version 0.115.0 (2026-11-04)

Owner, 2026-11-04: "gravity's visual is really small for some reason".

- **Which visual** (Checked (files + decomp)): the KH1 Gravity spell's sphere, not Gravity Break's effects. Re:CoM's own `ex_0010.dpx` is KH1's Sora set with Re:CoM's edits in KH1's units (program 2: 12 words differ, none a size; program 3's bounds 183 against KH1's 207), Re:CoM's own `ex_0010` EET events have scale 1.0 as ours, and KH1 starts Gravity Break's 3 / 2 with no scale (`0x1402ADC20`'s last argument is the blade fraction): Gravity Break, the trail, Stun Impact and Ripple Drive are KH1's size relative to Sora, unchanged.
- **Cause** (Checked (decomp)): Re:CoM's Gravity flags `0x41E1` carry bit 7, Re:CoM's own sphere growth: `CHomingBullet`'s init `0x140158660` sets the effect scale to `desc+0x1C / desc+0x78` (not `desc+0x90`) and every update `0x140156350` adds it again. 7.8's growth read that ~0.024 as the "spawn scale" and multiplied KH1's 0.01 → S by it: at the impact 0.047 / 0.059 / 0.076 instead of 1.0 / 1.5 / 1.6, **21 / 25.5 / 21 times too small** (up to 42 / 42 / 34 by update order).
- **Change** (`magic::gravity::sphere_flags` / `effect_scale`, `win::magic::release_gravity` / `grow`): the KH1 sphere's descriptor drops bit 7 (`0x4161`) and sets `desc+0x90` = KH1's 0.01; the existing per-frame growth (only while a sphere falls) writes KH1's scale itself. No new hook, site or patch. Combat = Re:CoM: Re:CoM's Gravity unchanged.
- Assumption (7.8's (3), kept): Re:CoM's `grav_all` at scale 1 = KH1's sphere at scale 1 (`EF2040.EFF` not in the session). Re:CoM's own rule (scale = hit radius / 1.5 m) would draw it 1.13 times bigger. Not carried: KH1's 20-frame fade growing × 1.02 after the impact.
- Checked: tests (`gravity_sphere_draws_kh1s_scale_not_recoms_self_growth`: flags, 0.01 → S, the old factors), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-m7-magic.md` § 7.8 (The sphere's size), `recom-m13-plan.md` § Stage 3 built (their size).

---

## Stage 226: M13 stage 4: each KH1 Keyblade's own trail (its `.wpn` set at its KH1 reach) and swing sounds (its `.se`), loaded with its own group, version 0.114.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § M13 and camp text answers): "The six KH2 Keyblades swing with Re:CoM's trail at KH1 Effects"; decided: each Keyblade's KH1 set loads with its own Keyblade group, only when equipped; trail length and look follow KH1's own trail data (1:1). Stage 4 of `docs/recom-m13-plan.md`.

- **The blade's reach is KH1's row** (Checked (decomp + files)): KH1's code-`0x30` point is `parent + t × (bone − parent)` on Sora's blade bone (`0x14029EB20` → `0x1401D6320`), with t from the weapon row's `+0x28` near / `+0x24` far. Re:CoM's own Kingdom Key trail nodes B / C (−0.78 / 0 m on `wep0`) are exactly KK's far −3.7 / near 0 × `k9`'s 0.2111 m. So each Keyblade's B / C = its own far / near on that segment (Ultima −1.18 m, One-Winged Angel −1.10 m …).
- **Its trail program** = its own `.wpn` section 0 id 5, run by Re:CoM unchanged (lifetime, look, emission: KH1's). Attach code 0 in 15 sets; `0x30` in Diamond Dust and One-Winged Angel (node A then KH1's blade-bone matrix: `k9` at 0). Checked (files: all 18 `.wpn`).
- **Correction** (Checked (files: btltbl)): `xw_ex_5200` (lifetime 9, attach 0x30) is One-Winged Angel (item 0x66), not Ultima; Ultima is `xw_ex_5160` (lifetime 12, attach 0). Oathkeeper's base is 0x380E (0x3887 is One-Winged Angel's).
- **Asset step**, per KH1 Keyblade group 21–36: `PLAYER/00GG/PL00GG.EET` (Re:CoM's `PL0002.EET` + KH1's events at 1301 with its own set and reach; Gravity Break's blade effect at its reach) and `raw/PLAYER/00GG/PL00GG.EFF` (`kh1_ex_5xxx.dpx` + its PC textures). Every Keyblade's `<model>.se` SCDs in `kh1-sounds.bin`, each tagged with its cards. Each Keyblade's KH1 sound events in `PL0001.SES` / `PL0011.SES` at variant 1301 + card − 1 from its own weapon base. `kh1-fx.json` `keyblades` lists the groups.
- **DLL**: `win::fx::preload` reads `kh1-fx.json` before the Keyblade model table and checks both files next to it. `keyblade_models::plan_table` gives such a group its own EET row and a sixth row, type 0xE `PL00GG.EFF`: Re:CoM's row loader takes any type (`0x14023B7A0` / `0x14023B9D0`, Checked (decomp)), so the set loads and unloads with the group. `fx::variants` picks the equipped Keyblade's (`stats::equipped_card`) sound variant where it has events, else KK's 1301, else 0. `win::sounds::ensure(card)` registers Sora's banks and the playing Keyblade's.
- **KH2 Keyblades** (38–43): unchanged: `PL0002.EET`'s Re:CoM trail at KH1's windows, and the Kingdom Key's KH1 sounds.
- No hook, call site or byte site; nothing per frame (per KH1 motion: one pointer read and small list lookups).
- Assumptions: the code-0 programs read the blade from nodes B / C (as KK's identical program does in Re:CoM); Re:CoM's `.EFF` handler from a PLAYER row as from an EFFECT row; the weapon's EET lookup by our group id finds its own `PL00GG.EET`; banks registered earlier in a session stay (≤ ~0.9 MB per Keyblade).
- Asset step: **re-run**. New outputs: 16 × `PLAYER/00GG/PL00GG.EET` + `raw/PLAYER/00GG/PL00GG.EFF` (GG = 21..36; **32 new `mod.yml` entries**, `recom-keyblade-models.md` § 11), and the changed `PL0001.SES`, `PL0011.SES`, `kh1-sounds.bin`, `kh1-sounds.json` and `kh1-fx.json`. Without the new files a group keeps the shared EET (logged).
- Checked: tests (nodes from reach, blade attach, sound variants, per-card banks, the plan's own rows, both manifests through the DLL parsers, every `.wpn` set in the session), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings), the Linux `kh1-recom-build`. Not checked: in game; the `.se` files (not in the session); a trail against KH1's look. Docs: `recom-m13-plan.md` § Stage 4 built; `recom-m13-kh1-fx.md` § 1 corrected; `recom-keyblade-models.md` § 10.2 / § 11.

---

## Stage 225: True squeeze for our names in Re:CoM's text: an x-scale carried by the text's own vtable copy, version 0.113.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § M13 and camp text answers): "Camp-page long names (Customize, Abilities, Re:CoM's text): **(a) true squeeze**, an x-scale taught to Re:CoM's text drawing". Scope: our names drawn in Re:CoM's own text that overflow their boxes (camp Customize rows and right-hand panel, the Abilities list with its Sleights / Enemy Skills rows, the command menu in Re:CoM's look, Party list, Items page; anything else of ours). Combat = KH1 in Sora's story (where these exist); Combat = Re:CoM and Riku's story unchanged.

- **Re:CoM** (Checked (decomp + disasm + exe)): `D2::C2DFnt` vtable `0x140606C98` (COL `0x140718CF8` at −8, 7 slots; written only by its ctors `0x140243F47` / `0x140243F81`, no absolute pointer to it in the image); draw `+0x18` `0x140289450` → `0x14025AA20` (no direct caller), measure `+0x28` `0x140289440` → `0x14025A380`. The draw expands escapes (`0x1403480C0`, `0x14025AC2B`), aligns from x `+0x28` (`0x14025AC3B..0x14025ACD6`), writes one packet of (data, register) pairs from `[[0x140882DD8]]` (`0x140281180` / `0x140280F30`; header (5, 9), (.., 0x17), (0x356, 0), (colour, 1); per glyph `0x14025A0A0`'s pairs, XYZ register 5 with x + 0x8000 in bits 0..16, `0x14025A286`).
- **Change** (`crate::text_fit`, `win::text_fit`): a squeezed text of ours gets a **copy of Re:CoM's C2DFnt vtable** (COL and slots kept; `+0x18` / `+0x28` ours; marker and the scale in 1/1024 steps before the COL; one copy per step, never freed). Our draw: Re:CoM's draw, then the x of every XYZ pair it wrote scaled about `+0x28` (after a header check): glyphs, spaces, spacing, icons and outline passes narrow together; left / right / centre edges stay. Our measure: Re:CoM's width × the scale. No Re:CoM field used; Re:CoM's dtor (slot kept) drops the pointer; scale 1 = Re:CoM's vtable. Chosen over the patch at `0x14025AC56` (it scales only size-driven glyphs, not spaces / spacing, and would run in every Re:CoM text): no Re:CoM code byte changes.
- **Callers**: `win::text_fit::fit(fnt, box)` measures with Re:CoM's `0x14025A380` on a copy of the object with the text expanded as the draw does, sets `floor(min(1, box / width) × 1024)` (≥ 0.25). Boxes (`text_fit::boxes`): camp list rows **172** (plate x 0..197, Checked (files: `FO0100.SPR` group 270, cursor 0x12D); text from 23: Assumption, the group's 1 × 1 first piece (22, 14) + (1, 2)); camp status values **220 − label width − 8** (right-aligned at 220, label measured; Checked (decomp `0x1400E6DF0`), gap ours); Re:CoM-look command rows and tab **167** (plate right 165, text at −4; Checked (files `FO0002.SPR` + exe)). One width per name on every row in Re:CoM text (its cursor frame and highlighted plate are the plate's size); KH1's look unchanged.
- Not covered: the shop's `CTextWnd` rows (CFntM2, short names), the L1 panel (KH1's look, 0.107.0), Settings rows / top list / help / notices (not seen to overflow; same call if needed).
- Sites: `TEXT_FIT_CHECKED` (11 byte sets, in the startup check) and `TEXT_FIT_PTRS` (8 vtable words, checked at first use: a mismatch only turns the squeeze off). `check_sites_rs.py`: 1295 sites, 0 mismatches. No hook, no patch.
- Per frame: Re:CoM's own texts nothing (their vtable is Re:CoM's); our squeezed texts about 40 instructions per glyph after Re:CoM's draw (counted from the disasm, Assumption); `fit` only when a row is made.
- Assumptions: the camp row text's left edge (23); the per-glyph count; Re:CoM's row marks not a box edge. Not checked: in game; Re:CoM-text widths (font files not in the session: logged in game).
- Checked: tests (`text_fit` 7, `sites::tests::text_fit_sites`), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings). Asset step: not needed. Docs: `recom-m12-sleights.md` § 8.8.

---

## Stage 224: M13: Stun Impact's effect 0 and Zantetsuken's effect 5 on Sora's body point, as KH1 places them (Re:CoM EET events), version 0.112.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § M13 and camp text answers): "Start at the target". **Corrected from the question's wrong premise** (Checked: KH1 decomp + files): KH1 places both on Sora, not on the struck enemy, so they are built as KH1 does it.

- **KH1** (Checked (decomp)): `0x1402A68F0` starts them with `0x1402AE160(Sora, id | 0x800)`. The link is on Sora (`0x1402AE7F0`, mode 2) and is re-placed every frame (`0x1402AE910`). Code `0x8003` (`0x14029EA00` / `0x14029EB20`) is resolved on that actor: his `+0x134` = `.mdls` section 4 `CLS` (`0x140287E40`), body record `[+0x14]` through `0x1402B5E50`. Sora's record (Checked (files)) is `{code 0x8002, fraction 0, lift 85}`: his `+0x50`, 0.85 m up, with his facing. Stage 3's "the target's contact point" note was wrong (`recom-m13-kh1-fx.md` § 3 corrected).
- **Built (asset step only)**: `kh1_fx::sora_body_point` reads the record from the player's `xa_ex_0010.mdls`. `Place::SoraBody` writes one list-A event per effect on Re:CoM's `Root` (bone 0: identity rest at the feet, Y up; Checked (files: `PL0001.MDL`)): offset (0, 0.85, 0), flags 0x28 (attached = follows, linger = KH1's 0x800, bit 3), frame 0, variant 1301, in `PL0001.EET` / `PL0011.EET` (Stun Impact 216, Zantetsuken 218). Sounds 81 / 86 come from `kh1_ex_0010.ese` (Re:CoM's spawn looks it up by the set's name, `0x1402A4A10`).
- **DLL unchanged**: 216 / 218 were already in `kh1-fx.json`'s effects. Per move: nothing of ours (Re:CoM binds and spawns one more list-A event). No hook, call or site.
- Assumptions: KH1's `+0x50` is Sora's position and Re:CoM's `Root` follows it; Sora's scaled-record flag `+0x374 & 0x2000` is clear; the start delay of effect 5 (28 ticks) as stage 3. The step decodes only a code-`0x8002` / fraction-0 record (Sora's) and refuses others (logged).
- Asset step: **re-run** (`PLAYER/0001/PL0001.EET`, `PLAYER/0011/PL0011.EET`, `kh1-fx.json`). No `mod.yml` change.
- Checked: tests (the body point on the player's own file: `[0, 0.85, 0]`; the events, flags and lift; refusals), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings), the Linux `kh1-recom-build`. Not checked: in game. Docs: `recom-m13-plan.md` § 0.112.0 built.

---

## Stage 223: Command menu pages (page 2: Summon, Party), Re:CoM's pointing arrow with KH2's sway, the Party list of friend sleights, version 0.111.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Command menu pages and Party), with a KH2 FM recording: "I like our command menu, but I think it should have the pointing arrow that follows your selection on the command menu so people know that you can press left on the D Pad to switch between the two pages. Also, Page 1 (the default) should be Attack, Magic, Items, Sleights. Page 2 should be Attack, Summon, Party, Sleights The "Party" section should have "Magic LV3" stylized as "Donald Magic" … These friend sleights should ONLY appear if the friend cards drop in their respective battles. … if any of these sleights require an additional unlock requirement, they shouldn't show up until that requirement is met. Lastly, friend sleights should be able to be put in the shortcuts."; answers "Re:CoM's own arrow", "Whole battle, if possible", "Same MP as sleights". Combat = KH1 in Sora's story; Combat = Re:CoM and Riku's story unchanged.

- **Pages** (`command_menu::Page`): page 1 Attack, Magic, Items, Sleights; page 2 Attack, Summon, Party, Sleights. D-pad Left at the root switches them on any row (the cursor stays; Re:CoM's cursor sound 1 in either look), replacing M12 step 5's Summon / Sleights switch. Saved at `save + 0x26B6` (`summons::field::PAGE`): 2 = page 2, else page 1 (M12 step 5's 0 / 1 read page 1). Row 3 is Sleights on both pages. Summon is page 2's row 1 (no spell needed; greyed and locked until learned), no longer the Magic list's last entry. Every list opens beside its own row: KH1's `0x14027B870` rule is `min(row y, 348 − 22 n)` for Magic / Items / Summon (Checked (decomp)), used for Party (row 2) and Sleights (row 3).
- **The arrow** (`command_menu::arrow`, asset `command_menu::group::ARROW`): Re:CoM's own white triangle of `Cockpit:00` (UV (16, 49)–(32, 64), sprite 84 of the base file: one of its lock-on cursor's pointers, group 98; Checked (files: the owner's FO0002.SPR / .RTM rendered offline)), found by UV in each copy, turned −π/2 to point left, in a group the asset step adds after the plates (`kh1-mp-bar.json` `extra_groups` 3). KH2's motion **measured** on the recording (448 × 300, 30 fps, every frame): it **never blinks**; it sways ±1.5 units, period 31.5 frames (60 Hz); after each cursor move it swings in from +6, overshooting −4.8 / +2.7 / −2.7 (frames 13 / 30 / 51). Baked as integer X keys, looped from frame 67 (Re:CoM's loop fields `0x140248F40` / `0x140248BB0`, Checked (decomp)); the DLL makes the control with the rows (each move restarts it): nothing per frame. Beside the root's cursor row only, both pages, field and battle, both looks.
- **Research** (`docs/recom-command-pages-party.md` § 1): Re:CoM's Friend-card drop roll `0x1401602A0` (Checked (decomp + exe)): Pluto in every Sora battle once met (int var (0, 0x22) bit 0x100); else table `0x1407C1F10` (26 × 8: chance ‰, weights, cards) by world `0x1402339C0(0)` (halls, room type `0x140233860` 0 / 99: row 13; Riku + 13): Donald / Goofy everywhere but 100 Acre Wood, Aladdin only Agrabah, Jack Halloween Town, Ariel Atlantica, Peter Pan Neverland, Beast Hollow Bastion, each once met (bits 1, 2, 4, 0x10, 8, 0x20, 0x40). Ids from `SY0002.CTD` (Checked (files)): Magic LV3 0x45, Stardust Blitz 0x46, Goofy Tornado LV3 0x48, Goofy Smash 0x49, Wild Crush 0x4A, Sandstorm LV3 0x4C, Surprise! LV3 0x4E, Spiral Wave LV3 0x50, Hummingbird LV3 0x52, Furious Volley LV3 0x54, Lucky Bounty LV3 0x56; friends by `0x14020F4D0` and the ctors' RTTI (Checked (exe)). Extra unlock = Re:CoM's learned bit `0x14023AB00`. Playing: Re:CoM's own start `0x1402247A0` → `initStockSkill` loads the friend's pack on demand (`0x14020F720`: Donald 0x1C / 0x2C …), `exeStockSkill` category 5 waits for it, builds the friend `0x14020E950` (PLM `+0x70` / `+0x78`), which acts and leaves: **no card and no per-battle data needed**; battle only (the friend branch reads `player+0x3E8 → +0x1E8` without a null test; packs load only in battle rooms). Found: **Lucky Bounty finds nothing 9 times in 10 at KH1** (its card find draws from the empty deck, `0x1401E2E60`).
- **Party** (`crate::party`, `win::party`): the owner's eleven, shown in a battle when Re:CoM taught it and every card it needs can drop there (Wild Crush both), read once per battle's player from Re:CoM's getters, kept for the battle; usable as a Sleights-list sleight (battle, ground, nothing running, MP); started by `win::summons::start_party` (battle room and card battle only, then Re:CoM's sleight start); the cursor preloads its pack. MP (Assumption, ours: no AP tier): 2, Wild Crush 3, Lucky Bounty 1. Its friend's hits count as the sleight's (Damage = KH1 scaling, breaking into attacking enemies). Names squeeze to fit (widths measured on the owner's KH1 font: Furious Volley 100.3 units … Surprise! 65.3).
- **Shortcuts**: a slot byte may hold a Party sleight's id (0x45..0x56); the Customize picker lists the taught ones after the sleights; the L1 list greys one where it isn't available; a slot whose sleight isn't taught has its own help line.
- Sites (`sites::PARTY_CHECKED`): the room-type getter `0x140233860` (35 bytes) and the drop table (208 bytes, no pointer); the other callees are existing checked sites. No hook, no patch. 1258 sites, 0 mismatches (pefile). Per frame: one atomic load in battle; outside battle Re:CoM's learned test ×11 while page 2 shows or a slot holds a Party sleight (not measured).
- Asset step: **must be re-run** (FO0002.SPR's `Cockpit.lad` copies: the arrow group; `kh1-mp-bar.json` extra_groups 3; `kh1-command-menu.json` / `kh1-hud.json`: "Party" 903 and the eleven names). Older outputs: no arrow (logged), Re:CoM's look has the words built in.
- Assumptions: the arrow's place (the plates' heights) and turn direction; the MP tiers; the friend sleights play in our cardless battles as in Re:CoM; the arrow on the root list only.
- Checked: tests (pages, page byte, rows, Summon on page 2, Party list, availability by world, the drop table against its site, shortcuts, the picker, the arrow keys and asset group, word lists against the asset step; the asset arrow test and the font-width dump on the owner's files), `cargo test --workspace --exclude kh1-playground`, the Windows build (no warnings), the Linux `kh1-recom-build`. Not checked: in game; KH2's own arrow data (no KH2 decomp in the session). Docs: `recom-command-pages-party.md`; notes in `recom-m12-sleights.md` § 8.5, `recom-summons.md`.

---

## Stage 222: Sleights outside battle, option (a): Re:CoM's sleight code in field rooms, free; KH1 Thunder / Stop paralyse / freeze field Heartless, version 0.110.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Sleights outside battle, on `recom-field-sleights.md`): "lets sora use sleights outside of battle"; answers: "(a) Plain field strike", "Free in the field: no MP for a sleight used outside battle", "Re:CoM's special field reactions kept, for sleights and spells: Thunder Raid paralyses, Teleport freezes … KH1 Thunder and Stop in the field get Re:CoM's paralyse / freeze too, instead of starting the battle at once", "Which: all sleights, the KH1 specials (Sonic Blade, Strike Raid, Ars Arcanum, Ragnarok; this lifts 'only in battle') and the LV3 summons". Combat = KH1 in Sora's story; Combat = Re:CoM and Riku's story unchanged. Both steps (pack-less and pack sleights) in this version.

- **Field case 0x16** (`field_sleights::field_case`): the Field exe's table entry `0x1401FA648` → a 13-byte stub at `0x1401FB7B2` (`mov rcx,rbx; call exeStockSkill 0x140218950; jmp 0x1401FA599`), as the magic / items cases. The research's cave `0x1401FA6C1` is the Attack's Battle case since M4 (Checked (code): `ATTACK_DATA`); `0x1401FB7B2` was 9c.1's first Field case. Checked (exe + decomp); unreachable at Combat = Re:CoM.
- **List pointers** (`win::field_sleights::prepare`, each field start): Re:CoM's own lookups `0x1402A43E0(1, "BattleCardList" / "StockSkillList", 0)` → `0x1402A49C0` into `*0x14087A808` / `*0x14087A810` (the `CCardBattle` ctor's calls; no reader null-tests them: Checked (decomp)); refused (logged) if not found.
- **Packs in field rooms** (`field_sleights::packs`): a wrapper on the PLM init `0x140210100` (13 bytes checked; once per room build) does the battle branch's writes (`+0x88 |= 1`, ring `+0x98` / `+0xA0` / `+0xA8`, the 16 slots' `+8`) where the field allocated the ring (not world 10, not rooms 2000–2999, ring > 0: the field's own condition `0x1401AF841`); not its `+0x58` (it would build a `CCardBattle`) nor `0x14087B6B0`. Checked (decomp + disasm). Elsewhere the 18 pack sleights, Sonic Blade / Ars Arcanum / Ragnarok and the summons stay greyed; the 9 pack-less sleights and Strike Raid run.
- **Gate** (`summons::Gate` + `field` / `packs`; `field_sleights::room_ok` / `free`): the Sleights list, shortcuts, Summon list, special offer and the listed specials in a field room; **no MP there** (lists and starts); friend sleights never (`exeStockSkill`'s friend branch reads `player+0x3E8` unchecked). `sleight_running` / the summon's `out` by action 0x16 in any room; preloads in battle or a field room with packs (keyed by the room's PLM).
- **KH1 Thunder / Stop on field Heartless**: found that they **never struck them** (their lists took only type 0x1004 with HP; `CMapEnemy` is 0x1002, `0x14019D050`, Checked (exe)); Fire / Blizzard / Aero do (kind 1 through Re:CoM's pass). Now their targets and strike lists also take lockable field Heartless (`win::magic::target_enemy` / `lock_on_targets`), struck through Re:CoM's contact + apply with Re:CoM's own kind (`magic::field_kind`: Thunder 0x1C / 0x1D / 0x1E → state 5 paralysed; Stop 8 → state 6 frozen; `0x14019E020`). Battle enemies unchanged. Found, not changed: KH1 Gravity hits no field Heartless either.
- **Specials**: offer and start in field rooms, targets may be field Heartless, Ragnarok's shots home on them; free.
- **Battle start cutting a sleight**: Re:CoM's player dtor `0x1401E4EB0` clears `+0x418` bit 31 and fades slot 0 (mask 0x1808) to white in 0 frames (`0x1401E51AD`, Checked (decomp + exe)): no fix needed. The i-frames are the field player's (destroyed); ours are per player + action, and the battle manager ctor's wrapper now drops the specials' / summons' running marks (`field_sleights::battle_start`, logs the cut).
- **Summons / Idyll Romp / Cross-slash+**: built (free, where packs load). Static scan of `0x140226000..0x14022F000`: both list pointers (filled), `CCardBattle` uses null-tested (`0x14015D8A0`, task 0x1B), lock-on pick `0x1401CDB90` takes field Heartless; no blocker. Not read: each summon's whole state code (Assumption: runs as in battle).
- Sites: 1 hook, 2 data, 8 checked (+4 of other lists re-checked); `check_sites_rs.py`: 1267 sites, 0 mismatches. Per frame: the menu's build reads about five more values in a field room (not measured); nothing else.
- Assumptions: both lists stay loaded in field rooms; the field player's dtor runs before the battle's player exists; the scene change waits for a queued pack load; the summons run in a field room as in battle. Risks: a pack loading when the battle starts; the camp opening mid-sleight (its gate reads no action); dashing moves touching first (plain battle).
- Checked: tests (`field_sleights`, `sleights`, `summons`, `magic`, `command_menu`, `sites::field_sleight_sites`), the Windows build (no warnings). Not checked: in game. Docs: `recom-field-sleights.md` § Built; notes in `recom-m12-sleights.md`, `recom-m9-abilities.md`, `recom-summons.md`.

---

## Stage 221: M13 stage 3: KH1's Keyblade trail, swing sounds and finisher effects for Sora's combos as Re:CoM's own EET / SES events at a KH1 variant; KH1's trail end; the Kingdom Key's sounds from `xw_ex_5010.se`, version 0.109.0 (2026-11-04)

Owner, 2026-11-04: "also, lets add all of soras effects for real this time. He doesn't have his keyblade trail, gravity break doesnt have it effects/sound, neither does ripple drive or stun impact,". Stage 3 of `docs/recom-m13-plan.md`.

- **Native route** (Checked (decomp)): Re:CoM's motion start keeps the EET / SES events whose variant the request passes (`0x14017A240` walks every record of the motion; `0x140239260`); the Keyblade plays Sora's index with the same variants (`0x1401F7180`). The asset step (`kh1_recom_assets::kh1_fx`, new `eet` reader / writer: Re:CoM's `PL0001.EET` / `PL0002.EET` rebuilt byte for byte) writes KH1's events at **variant 1301**; the DLL passes it in `Kh1Sora::play`'s existing motion request (`win::fx::variants`) for the moves in `kh1-fx.json`, by ini `effects` / `sounds` (kh1 | recom, default kh1). Combat = Re:CoM or the toggle at Re:CoM: variant 0, nothing changes.
- **Trails** (`PL0002.EET` list B, 16 moves): Re:CoM's own `ex_5010.dpx` 5 (= KH1's Kingdom Key set) on Re:CoM's `w001_055` nodes, at KH1's type-7 windows (first whole frame ≥ 1 on → first off; `0x1402C1F20`): 200 4–24 … 215 26–36, 217 28–38, 218 30–50 (Checked (files)).
- **KH1's trail end** (`win::fx`; sites `FX_HOOKS` / `FX_CALLS` / `FX_CHECKED` / `FX_PTRS`, 1263 sites 0 mismatches): KH1 stops the emission at the window's end and kills the trail at a motion change; Re:CoM's list B can only fade / kill and leaves ended ones lingering. KH1 events carry fade −1301: the end call `0x14017ABEF` → `trail_end` (stop emitting `0x140255040` for that fade, else Re:CoM's `0x140254D60`); the EET motion stop `0x14017A430` wrapped (10 bytes, exe scan: no branch in): after Re:CoM's, the KH1 trails in the linger list are killed. Not per frame.
- **Sounds** (`PL0001.SES` / `PL0011.SES` at 1301): KH1's op 0x19 / op 0 keys at their frames through stage 2's map; Re:CoM's steps and voices copied; Re:CoM's whoosh kept for a KH1 id with no bank. The Kingdom Key's SCDs: every SCD of the Keyblade's **`xw_ex_5010.se`** (Checked (files: the owner's install): 40, `se011018`..`22` shared, `se011024`..); the world packs are no longer read (none there, the owner's 0.108.0 run). Banks made sure of at a KH1 sound play after each battle start.
- **Finisher effects** (`kh1_ex_0010.dpx` at frame 0, `0x1402A68F0`'s table): Stun Impact 1, Gravity Break 3 + 2 (on the blade at KH1's fraction 0.8, Keyblade clip), Zantetsuken 4, Ripple Drive 6 (KH1 bone 8 = `belt1`) + 7; `0x18001` = detached at Root (flags 0x29). **Not built**: Stun Impact 0 and Zantetsuken 5, placed at the **target's contact point** (`0x8003`): no EET event places at another actor (owner question).
- Halloween Town: `PL0011.EET` / `PL0012.EET` get the same events when Recom.hed has them. Voices: a later stage (no KH1 voice files in the session; KH1's groups need a design choice).
- Assumptions: Re:CoM's runtime applies a program's start delay `+0x108` (KH1 34 / 36 / 10 / 28 / 34 ticks); the `.ese` sound starts at the spawn; `0x140474EB0` = KH1's stop byte; ±1 frame.
- Checked: tests (variants, manifests, windows, SES / EET builders, Re:CoM's EETs byte for byte, the step's output through the DLL parser), the Windows build (no warnings), sites against the exe. Not checked: in game; the SES path on the real `.se` (not in the session). Asset step: re-run (new outputs `PLAYER/0001/PL0001.EET`, `PLAYER/0002/PL0002.EET`, `PLAYER/0011/PL0011.EET`, `PLAYER/0012/PL0012.EET`, `kh1-fx.json`). Docs: `recom-m13-plan.md` § Stage 3 built.

---

## Stage 220: M13 stage 2: KH1's own sound banks in Re:CoM's (Sora's SCDs, the Kingdom Key's from the world packs) and the KH1 → Re:CoM id map; the KH1 effect set's sounds 81–88 as its `.ese`, version 0.108.0 (2026-11-04)

Owner, 2026-11-04: "also, lets add all of soras effects for real this time. He doesn't have his keyblade trail, gravity break doesnt have it effects/sound, neither does ripple drive or stun impact,". Stage 2 of `docs/recom-m13-plan.md`.

- **KH1** (Checked (decomp + files)): every sound id ≥ 1000 is its own one-sound SCD asset `se0<id>.win32.scd` (number `atoi(name + 2)`, `0x1400DB1F0`; made from entry 0, `0x1400DE460`); ids < 1000 are entries of the system bank 2 (`0x1404D5EB8` = 2, Checked (exe)) = `se000.win32.scd`. Sora's swings / finishers / Guard / Counterattack sounds are weapon sounds (op 0x19 = weapon `+0x34` + arg; Kingdom Key `0x2B10`). Sora's 21 SCDs hold one sound each.
- **Re:CoM** (Checked (decomp)): `0x1402F3080` makes sound `id % 1000` of the first bank entry with the id's bank: one KH1 SCD = one Re:CoM bank (9001 …), id `bank × 1000`.
- **Asset step** (`kh1_recom_assets::sounds`): Sora's `xa_ex_0010.mdls` SCDs + the Kingdom Key's 40, from Sora's file then the world packs `amusic/se<ww>.dat` (KH1's order) → `kh1-sounds.bin` (unchanged SCDs) + `kh1-sounds.json` (id → bank, offset, size, source; ids not found). `EF1000.EFF` gains `kh1_ex_0010.ese`: KH1's program sounds `+0xFC` 81..88 (volume offset 0) as rows on bank 9000 (`effects::kh1_ese_rows`), only when the system bank was taken.
- **DLL** (`crate::sounds`, `win::sounds`; `win::menu_sounds` now shares its map hook, alloc check, SCD reader and registration): at each battle start at Combat = KH1, one walk of Re:CoM's bank list, missing SCDs (and bank 9000) to Re:CoM's `0x1402F5D70`; one log line. The menu sounds' bank is now read without the KH1 menu look too. No new hook or site; nothing per frame; at Combat = Re:CoM nothing registered. Nothing plays the new banks yet (stage 3).
- Local run: 21 banks 9001..9021 from Sora's file; the Kingdom Key's 40 **not found** (no world pack data in the session); `.ese` 8 rows. Assumption: the Kingdom Key's SCDs are in the world packs (the owner's run says).
- Checked: tests (manifests, id map, ESE layout = Re:CoM's `ex_0010.ese`, the step's real output through the DLL parser), the Windows build (no warnings). Not checked: in game. Asset step: re-run. Docs: `recom-m13-plan.md` § Stage 2 built.

---

## Stage 219: Customize rows name their shortcut; long sleight names squeeze to fit in KH1's look; a "Sleights" section in the ability list, version 0.107.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § Customize page and long names, § Sleights list and ability section): "when something is bound to a shortcut, they should also be listed by the button prompt, as well as the list on the right … we need a solution for certain sleights having too long of a name to fit within the black box."; "The same thing for the sleights menu. I think we should have also have "sleights" have its own menu in the ability list"; answer "Squeeze to fit". The Customize Summon / Sleights toggle was **dropped by the owner** ("disregard the toggle for sleights and summons"): not built. Combat = KH1 in Sora's story; Combat = Re:CoM and Riku's story unchanged.

- **Customize rows** (`customize::text::slot_row`, `win::keyblades`): each slot row = its button glyph, a space, the bound spell's (at its level) or sleight's name, in the row's own Re:CoM text (`CBUTTON_SETUP`); an empty slot shows the glyph alone; the right panel unchanged.
- **Re:CoM's text has no x-scale** (Checked (decomp + disasm)): `C2DFnt` / `CFntM2` (`0x140243F10` / `0x140258BC0`) draws (`0x14025AA20`) every glyph's width, height and advance from one size `+0x3C` (read at `0x14025AC3B`); flag 0x8000 halves single-byte glyphs (fixed); `+0x44` letter spacing; Re:CoM's text-width routine `0x14025A380(out[2], fnt, lines, count)` (vtable `+0x28`); `C2DFntObj` `0x140244230` scales the size uniformly from its parent.
- **Squeeze to fit, built in KH1's look** (`kh1_hud::fit`, `win::command_menu`): KH1-look words are `kh1hud` groups laid out from x 0, so each word's control gets Re:CoM's control X scale (`0x1402486D0`, an existing checked callee) = min(1, box / width). Boxes measured on KH1's art (Checked (files: `uitex.bin` pictures 0 / 1 / 3, `UK_uibin_sum / mag / cut` rects)): a Magic / Summon / Sleights row 88 (black inside to x 106, word at 16), the cursor row 67 (the hat icon at 87, word at 18), the SHORT CUT name box 68 (black to 85, word at 16). Lists: only our sleight names (`0x8100 + id`); KH1's own words keep KH1's look. Widths: the asset step's layout in KH1's menu font (`menu.word_widths`, tenths of a unit; Checked (files: `US_font_data_tbl.bin`; the dump test: Hi-Potion 67.7)): e.g. Homing Blizzara 110.4 (× 0.797 row, × 0.607 cursor row, × 0.616 SHORT CUT); the table is in the doc.
- **Not built** (report before any fallback): the boxes drawn as Re:CoM text (Re:CoM look's command menu, the camp's Customize rows and right panel, the Abilities list). Levers for the owner: (a) an x-scale added to Re:CoM's text draw / width routine (`0x14025AC56`, `0x14025A380`; a per-text scale needs a home; every text draw runs it); (b) Re:CoM's own letter spacing `+0x44` (no patch); (c) Re:CoM's own size `+0xA8` (both ways).
- **Ability list** (`abilities::page`): a fourth owner row **"Sleights"** (after Enemy Skills) with the 27 sleight abilities in Re:CoM's order; they leave Sora's row; KH1's specials stay Sora's; equip rules, AP, saves, learning unchanged.
- No new hook, call site or patch; nothing new per frame (the scale is set when a word's control is made). Asset step: re-run (`kh1-hud.json` `word_widths`; older manifests: nothing squeezed; older DLLs ignore the key).
- Assumptions: the control's X scale keeps the word's left edge (the layer's screen offset outside the scale); the 2-unit margins; the Sleights row's place (last).
- Checked: tests (slot rows, widths parsing / fallback, the fit rule and boxes, the Sleights section; the asset dump test on the owner's files), the Windows build (no warnings). Not checked: in game. Docs: `recom-m12-sleights.md` § 8.7, `recom-keyblade-equip.md` § Customize.

---

## Stage 218: The KH1 camp list moved down 10 (Items no longer under the header), version 0.106.1 (2026-11-03)

Owner, 2026-11-04 (screenshot): "you'll notice, items is kind of clipping through the top, there's enough space at the bottom to bring everything down a little bit".

- **Measured** (Checked (owner's 720p screenshot)): the header frame's lower border reaches ≈ 69 units (130 px), the first plate's top was 64; the bottom info panel starts at ≈ 297 (558 px), the last plate ended at 282.
- **Change** (`keyblades::top::KH1_Y0` 78 → 88 = Re:CoM's own first y): the nine at y 88 .. 280 (step 24 kept), plates 74 .. 292, glows 72 .. 294; the highlight bar and glove follow (Re:CoM's offsets). Re:CoM's seven untouched.
- Tests updated. Not checked: in game.

---

## Stage 217: Enemy Skills stage 1: Re:CoM's enemy cards as KH1 abilities on a third "Enemy Skills" row; the five KH1 matches from their cards, version 0.106.0 (2026-11-04)

Owner, 2026-11-03 (`design-philosophy-instructions.md` § Enemy cards as abilities): "all enemy cards that are the same as a KH1 Ability should replace its respective KH1 ability. Soldier card should only replace 1 combo plus. … let the enemy cards have their own separate section in the ability tabs called "Enemy Skills" Have each ability have the name of the original card, and a description of what the ability does."; 2026-11-04 (§ Enemy Skills plan answers): "Go ahead": build in 5 stages, stage 1 first. Combat = KH1 in Sora's story; Combat = Re:CoM and Riku's story unchanged.

- **Table** (`crate::enemy_skills`, pure, tested): 48 skills (0x44..=0x7E minus the 11 left out), each with its card kind, the card's name (Re:CoM's card-info table `0x1405256D0`, name at row `+8`: Checked (exe); ASCII, Saïx → "Saix"), Re:CoM's effect name, the owner's AP (the five matches KH1's: btltbl `0x33D8`, Treasure Magnet id 5 = 2 AP, key 302, Rod levels 9 / 72: Checked (files)), a one-line KH1 description (ours, ≤ 66 characters).
- **Abilities** (`crate::abilities`): words 128 bits (`Bits = u128`); every skill an ability with `Source::Card(kind)`; Treasure Magnet ×2 at 58 / 59, the 43 own skills at 60..102. The KH1 matches are KH1's abilities with their existing effects: Soldier = the Rod level-87 Combo Plus (93 / 99 stay by level), Hook = Second Chance (was 90), Hades = Berserk (60), Creeper Plant = Leaf Bracer (39), Bouncywild = Treasure Magnet's level-9 copy (the level-72 copy stays by level).
- **Not built yet** (`abilities::built`): Treasure Magnet (both copies) and the 43 own skills are shown **greyed and never equipped** (help line "(not built yet)" / "(effect coming)"; confirm: sound 5, "Not built yet: this can't be equipped."). Chosen over equippable-but-inert.
- **Treasure Magnet research** (not built: KH1's rule doesn't map): KH1 `0x1402AC080` / `0x1402AB420`: collect radius² 6400 (0.8 m, height ≤ 0.3 m then flat), one copy 40000 (2 m), two 160000 (4 m) (Checked (decomp)). Re:CoM: prizes pulled within player `+0x550` = 5.0 m (`0x140230C70`: EXP / HP / Moogle prizes `0x140231010`, card prizes `0x14022F2B0`), scattered cards 1.5 m unless Bouncywild (`0x14022F23A`); `+0x54C` has no reader; the win action sets 50 m (`0x1401F9C5E`, `0x1401EC55E`) (Checked (decomp + exe scan)). Re:CoM's base already exceeds KH1's 4 m: owner question (options in the doc).
- **Learning**: Re:CoM's discovery state `0x14015B3F0` (via `win::keyblades::ever_obtained`) in the ability sync (`sync_with`), unequipped; the existing first-obtained wrapper on `0x14015C7F0` runs the sync for an enemy card's first pickup; a "Sora learned X" notice (Re:CoM's ability wording, the card's name; `notices::card_abilities`), none on a save's first sync or from an older block. Older saves at their first KH1 read.
- **Page**: a third owner row "Enemy Skills" after Sora and Shared; rows by card kind under Re:CoM's localized card name (its card-name call `0x140164CA0`; message = the kind for 0x44..=0x7E, Checked (exe); English fallback); AP cost in the panel, the description on the help line; KH1's AP rules, one AP pool.
- **Save, layout v7**: skill extension `save + 0x26BF ..= 0x26CF` (u8 version, u64 unlocked / equipped bits 64..128), in Re:CoM's spare tail, counted as ours by the spare-range report. Migration once: the four matches learned by level before are taken back and asked again from their card (back at once, as equipped, when the card is had; else their AP returns), then the skills of the cards had arrive unequipped without notices.
- No new hook, site or patch (the card-name call, the discovery read and the first-obtained wrapper are existing checked sites). Per frame: `equipped_live` reads 17 more bytes (not measured).
- Assumptions: the KH1 meanings of Ansem / Gold Card / Lexaeus (ours, to settle at their stage); Re:CoM's card-name message is loaded in the camp (English fallback otherwise); our notice font has no ï.
- Checked: tests (the table, AP, names, help lines, learning, the matches' level grants gone, the page, the save round trip, v6 → v7, notices; `ap_tables_match_btltbl` with the player's btltbl), the Windows build (no warnings). Not checked: in game. Asset step: not needed. Docs: `recom-enemy-skills.md` § Stage 1 (built), `recom-m9-abilities.md` (save layout v7).

---

## Stage 216: M8c stages 2–3: the Tent and the Camping Set (camp-only full heals, sold, stocked); the Mega items removed at KH1, version 0.105.0 (2026-11-04)

Owner, 2026-11-04 (`design-philosophy-instructions.md` § KH1 Moogle shop): "The very early game shops should only sell potions, ethers, and tents. Tents and camping sets should have their descriptions changed to say sora instead of "to the party" and should have their cost reduced"; "Lets remove mega potions, mega ethers, and mega elixirs"; answers: Tent = full HP, Camping Set = full HP and MP, from the KH1 camp Items page outside battle ("Camp-only full heal"). Combat = KH1, Sora's story only; Combat = Re:CoM and Riku's story unchanged.

- **KH1** (Checked (files + decomp)): `btltbl.bin` rows 142 / 143 (`0x255C` / `0x2570`): names 172 / 173, kind 12 (menu item: `0x1402CD3A0` takes kinds 0 / 12; slots and the battle menu kind 0 only, so the pickup `0x140284E60` stocks them), party, buy 250 / 500, sell 50 / 100, sort 101 / 102. Effect `0x1402CD780`: 0x8E HP + 9999; **0x8F HP + 9999 and MP + 3 points** (its row's amount: not full). Can-use: 0x8E HP below max, 0x8F HP or MP. The Barrel Spider drops #142 1 % / #143 0.5 % (8a.4's table).
- **Stage 2** (`crate::items`, `items_page`, `shop`, `drops`): our ids are KH1's 142 / 143 with KH1's rows; no slot, no command (`item_of_command` battle items only), not in the command menu or the battle use task. Stock only (to 99) in the unchanged 16 stock bytes: byte 8 Tent, 9 Camping Set (`stock_index`; bytes 8..15 were never written before: Checked (code)). Camp Items page: in the Stock list after the battle items; Use greyed at full (sound 5), Move / Equip greyed, Discard; a use: sound 2 and `win::items::use_in_menu` (Re:CoM's heal `0x1401E59A0` + stats sync `0x1401F0120`; MP through our MP block). Tent HP full; Camping Set HP and MP full (owner's design; `items::CAMPING_SET_FULL_MP`, owner question: KH1's + 3). Shop: built (1F Tent / 100, 2F+ Camping Set / 150; sell 20 / 30; to the stock). Help lines (ours): "Fully restores Sora's HP." / "Fully restores Sora's HP and MP.". Names: KH1's words 172 / 173 (asset step `NAMES`), else English. Drops: `spawnable` = the items in play, so the Barrel Spider's Tents drop. Test inventory: 9 of each item in play.
- **Stage 3**: a Mega drop gives **nothing** (`Dropped::Removed`, logged). A save's Mega items are **converted once** (`items::convert_removed`: Mega-Potion → Hi-Potion, Mega-Ether → Ether, Megalixir → Elixir, slots in place, stock added up to 99, the rest lost and logged) at the store's first read of a v1 block, which becomes v2 (`ext::VERSION`; every write v2; v0 stays KH1's new game; older DLLs read v2 as before). Chosen over hiding (a hidden slot item would hold a slot the page shows empty; four filters). The test inventory never hands them out.
- No new hook, site or patch; nothing new per frame (one byte compare in the store's read). Asset step: optional (KH1's words for the two names; English without).
- Assumptions: `Word.bin` 172 / 173 are the two names; the camp never opens in battle (Re:CoM's rule); the 73-character Sell line fits the Moogle's window; that KH1 refuses at full in this window (not its target list).
- Checked: tests (pure rules: rows, stock bytes, pickup, conversion, page use / refusal, shop buy / sell / lists / texts, drops), the Windows build (no warnings). Not checked: in game. Owner questions: the Camping Set's MP (full or KH1's 3); the Barrel Spider's Tent drops; item descriptions on the camp page. Docs: `recom-m8-items.md` § M8c stages 2–3.

---

## Stage 215: The camp menu at KH1: Items, Abilities, Customize … Settings; Settings opens Game Settings / Combat / Movement; the "KH1" entry gone, version 0.104.0 (2026-11-04)

Owner, 2026-11-04: "in the "KH1" menu, bring the "Combat" and "Movement" sections into the "Settings" menu. When combat is set to "KH1," Replace "Review Decks" with "Items" from the "KH1" Menu When KH1 combat is selected, the menu buttons should look like this: Items Abilities Customize World Map Map Cards World Cards Status Journal Settings. When ReCoM combat is selected, the menu buttons should be default. This should remove the "KH1" section on the camp menu, and if you need to make more space for the buttons, make it work". Answers: Settings opens a short list (Game Settings, Combat, Movement) in Sora's story at both settings; Riku's story Re:CoM's default menu.

- **Lists** (`keyblades::top::layout`): Sora's story + Combat = KH1 (+ Keyblade numbers): nine; Sora's story + Combat = Re:CoM: Re:CoM's seven where Re:CoM puts them, Settings opens the short list; Riku's story and the map camp (`camp+0x20A4`): untouched. Decided at each top-list build (`0x1400E3DE0` mode 0, level 0), so a Combat switch on the Combat page switches the list on the way back.
- **Index map** (`top::KH1_ORDER` = [0, 7, 8, 1, 2, 3, 4, 5, 6]): Re:CoM's entries keep camp indices 0..6 (its entry 1 / 3 checks, state-0x11 table, header titles, help ids, `0x1400D40D0` all still mean the same; Checked (decomp)); entry 0 relabelled "Items"; Abilities / Customize are two more Re:CoM entry buttons at indices 7 / 8 (made as `0x1400E3DE0` makes its own). Count `camp+0x21C4` stays 7.
- **Positions** (measured, Checked (files: `camp.lad` of `FO0100.SPR`) + exe): y = 78 + 24 × slot (78..270), x 86; bar (48, y − 8), glove (150, y − 6), Re:CoM's offsets (`0x140509294`, `0x1405092C0`, `0x1405092F4`, new data sites). Plates 26 high (overlap 2); first plate top 64 below the header's 63 (layout (78 / 196, 51)); last entry where the 8th entry was (270, seen in game since 0.8.0).
- **Wrappers** (the four existing ones; no new hook): entry build: our buttons made / deleted, "Items" put on (and Re:CoM's `0xA80001` label put back), places; cursor set: Re:CoM's work for the nine at their places (`0x1400E07D0` / `0x1400E3D30`'s work replicated, Checked (disasm)); update state 0xF: moves in screen order (Re:CoM's own move with the cursor set redirected; ours where its 7-entry rule refuses), confirm on Items / Abilities / Customize with an index past 6 for Re:CoM's call (`0x1400DAABD`: no deck submenu) then Re:CoM's page-entry confirm sequence; Settings' step 9 undone the same frame (the list keeps the main panel's layer 2, as the Deck submenu); state 0x11 → 0x4A for ours; teardown deletes ours.
- **Settings list**: Re:CoM's Deck-submenu mechanics (as the old "KH1" submenu): Game Settings runs Re:CoM's Config input `0x1400D3EB0` itself (`menu::recom_config`, `camp+0xA8` = 0 first; its draw reads no level / cursor: Checked (decomp)); Combat / Movement our settings pages. Back: cursor on Settings.
- Pages, sounds, help lines and logs of Items (Keyblade slot), Abilities, Customize, Combat, Movement unchanged. Per frame: the pad read in state 0xF while the camp is open; nothing in field / battle.
- Data sites added (4, pefile: 1251 sites, 0 mismatches). Assumptions: the 2-unit plate overlap and the glove's spot read well (no screenshot yet); our labels go with Re:CoM's buttons (as since 0.8.0); "Items" / "Abilities" / "Customize" / "Game Settings" are ASCII. Riku's story loses the old entry (owner's answer).
- Checked: tests (pure rules: lists, index map, positions, moves, fix, sub rows), Windows build, site bytes. Not checked: in game. Docs: `recom-keyblade-equip.md` § The KH1 camp list, `recom-config-menu.md`.

---

## Stage 214: M8c stage 1: KH1's Moogle shop list (Buy / Sell / Card Packs) as Re:CoM's shop mode 3, version 0.103.0 (2026-11-04)

Owner, 2026-11-04: "When Combat is set to KH1. I want the moogle shops to change. When buying from them, they should sell items. no other tabs should appear, it should just be a list like KH1 is"; on the plan: "A 'Card Packs' row" (`design-philosophy-instructions.md` § KH1 Moogle shop). Stage 1: the list, stock by floor, buy and sell.

- **Research** (Checked (decomp + disasm + exe), `recom-m8-items.md` § M8c): `CMoogleShopCore` (ctor `0x1403A4220`, made by event-script command 0x38 in `0x140145C50`) is a `CFSM` (step `0x140242F50`) over 7 mode records at `+0x98 + 0x40·mode` (`+0x10` init, `+0x20` update, `+0x30` end): 0 load, 1 the first visit's free pack, 2 no room for 5 cards (offers the card trade), 3 the two-choice menu (card packs / card trade), 4 the pack screen, 5 the card trade (sell cards for points), 6 leave (the dtor gives the field back). Camp list page outside the camp: not workable (needs a `CCamp`, whose life cycle the field object owns). Mode 3 with a third choice: needs `m_shop.lad` art (not in the session). Built: the shop's own list pattern (mode 2's: a `CTextWnd`'s lines and the shop's finger) as mode 3.
- **Change** (`crate::shop`, `win::shop`): a wrapper on mode 3's init `0x1403A53F0` (10 bytes); at Combat = KH1 in Sora's story (after Re:CoM's first-visit mode 1) it points the shop object's mode-3 update slot `+0x178` at our update and sets the windows up as Re:CoM's mode-3 init does. Our update keeps Re:CoM's mode-3 states (0 / 1 / 2 / 3 / 4 / 100 / 0x65): the rows in a `CTextWnd` opened by Re:CoM's `0x1402B1420`, the shop's finger `+0x3D0`, the highlighted row's line in the Moogle's text window (`0x1402B0EF0`), Re:CoM's sounds. Card Packs → Re:CoM's mode 4 (mode 2 without room: Re:CoM's rule), unpatched. The 8a.3 Items tab and its pack-screen hook `0x1403A6930` are removed.
- **Data** (owner): stock by Re:CoM's floor tiers (`0x1402337C0`, `0x1403A5B10`): 1F Potion, Ether (Tent: table placeholder, not built); 2F+ Hi-Potion (Camping Set: placeholder); 7F+ Elixir. Buy 10 / 30 / 40 / 200 Moogle Points; sell = buy ÷ 5 (KH1's ratio, Checked (files)); Mega-Potion / Mega-Ether / Megalixir never sold, sold back at **KH1's own sell price** 40 / 60 / 100 (KH1 has no buy price for them).
- **Buy**: one per confirm, KH1's pickup rule, refused (sound 5, nothing paid) without room or points; Re:CoM's payment (8a.3: tally, trophy, setter `0x14023A1D0`, point window). **Sell**: one per confirm (stock first, then the last slot holding it: our rule), sound 6 (the card trade's), points through the setter; a row at 0 leaves the list.
- Sites: 1 hook, 29 callees, 7 checked byte sets (37 compared with `recomsteam.exe`: 0 mismatches). Nothing per frame outside the shop's mode 3. Combat = Re:CoM and Riku's story: Re:CoM's init and update.
- Checked: tests (stock, prices, sell list, buy / sell, the list's moves, texts; the shop sites against the decomp), the Windows type-check (`cargo check --target x86_64-pc-windows-gnu -Zbuild-std`, no warnings). Assumptions: the rows' place and the finger's line-up (to judge in game); the help line's fit; Re:CoM's mode-3 text visible during the window's opening frames. Not checked: in game; KH1's own sell source. Owner questions: Card Packs without room; a Trade Cards row; selling slot items.

---

## Stage 213: Re:CoM's HP gauge follows a live Combat / HUD switch, version 0.102.5 (2026-11-03)

Owner, 2026-11-04: "when switching combat from recom to KH1, everything changes except the HP bar, and vice versa" (screenshot: KH1's MP ring bottom right, Re:CoM's HP gauge still top left).

- **Cause** (Checked (code)): the gauge was turned and moved to KH1's place only at its init (`place_recom_hp` from `gauge_made`); the camp's switch ran `kh1_hud::refresh` (ring, hides) but not the gauge.
- **Change** (`win::kh1_hud`): `refresh` places or unplaces each live gauge; placing records Re:CoM's layout position (`0x140248660`: `+0xF0 / +0xF4` = x − 256, y − 192; Checked (decomp)); `unplace_recom_hp` gives layers 1..3 Re:CoM's own groups 7, 8, 9 back through the same layer replace (frames: max HP on layer 1, the HP now on 2 / 3) and the recorded position; a live place sets layers 2 / 3 to the HP now (the init keeps Re:CoM's max). Init / dtor forget the entry. No new hook or site; nothing per frame.
- The room plate still moves at its next set-up. Not checked: in game. Docs: `recom-kh1-hud.md` § 0.102.5.

---

## Stage 212: The MP ring's 90° / 180° dividers drawn as KH1's gaps; its straight part lined up with the arc, version 0.102.4 (2026-11-04)

Owner, 2026-11-04 (KH1 MP ring around Re:CoM's HP ring): "there is a segment line at the top of the MP bar that is too narrow, differently drawn, and extends past the bar"; "once the curve of the MP bar flattens, the thickness of the bar very slightly increases".

- **Causes** (Checked (files + offline render)): the 12 o'clock divider was KH1's tick `g_s` 37 (2 × 14, navy with 1-unit black ends, PS2 texels) placed for KH1's stretched top: on our ring it stuck 0.5 unit past the outer edge, cut the 2-unit outlines and was 2.0 units dark against the gaps' 2.7–2.9. The straight pieces (`g_s` 17 / 21 / 32 / 33) have art flush with their rects, the arc's bottom half ends half a unit inside its quarter (PC art r 40.5..56.5): the straights sat 0.5 unit lower (same thickness), so under Re:CoM's HP bar they read 0.5 thicker. KH1's own data has both (its tick 2 units against 3-unit gaps; its straights 0.5 below its arc: fan `0x14026E450`, Checked (decomp + files)).
- **Change** (asset step `kh1_hud::ring` only): the 90° / 180° dividers are strips of the MP back's own art faded by the cells' measured coverage across KH1's 30° gaps (they compose a gap over the cells, nothing past the back's art); the ring's straight pieces get atlas copies shifted up one PC texel (rects one unit taller). Not KH1's data: owner's ask. The full KH1 gauge keeps KH1's ticks / straights. DLL unchanged.
- Asset step: must be re-run. Checked: tests, offline render of the owner's files (before / after). Not checked: in game; Re:CoM's HP gauge art (placeholder in the render); KH1 on screen. Docs: `recom-kh1-hud.md` § 0.102.4.

---

## Stage 211: Combat = Re:CoM shows Re:CoM's HUD as Re:CoM places it, version 0.102.3 (2026-11-03)

Owner, 2026-11-04: "with combat set to RE:CoM the hud should also go back to the normal positioning".

- **Change** (`win::kh1_hud::active`): KH1's HUD also needs Combat = KH1 (`movement::kh1_on`). At Combat = Re:CoM every piece is off whatever `hud = kh1` / `hud_*` say: Re:CoM's HP gauge at Re:CoM's place (not turned), the room plate where Re:CoM puts it, nothing hidden, no KH1 MP ring / gauge / command-menu look. A switch applies where each piece is set up (the gauge's init, the plate's set-up: the next room or battle).
- Side effect: H6's KH1-look friend gauges (`hud_kh1_party`, only at Combat = Re:CoM) no longer show (off in the owner's ini).
- No new hook or site. Not checked: in game. Docs: `recom-kh1-hud.md` § Combat = Re:CoM.

---

## Stage 210: Sleight and summon hits break into attacking enemies (Combat = KH1), version 0.102.2 (2026-11-03)

Owner, 2026-11-04: "for some reason using mega flare doesnt really do much damage", then "check the traces, some hits from the lethal frame also arent hitting, same goes for the summons at times".

- **Traces 21 / 22** (Checked (trace)): during Lethal Frame and Mega Flare only enemies not in an attack took the hits (defeats, blow-aways, a flinch); every enemy in its attack (`+0x260` bit 0, trace `P`) took none. The trace's `hits` column records KH1 swings only.
- **Cause** (Checked (decomp)): Re:CoM's enemy hit handler `0x14017F7E0` drops a hit on an enemy in an attack unless it carries the card-interrupt bit `0x2000000`; in Re:CoM the sleight's cards break the enemy card first, at KH1 neither has a card. Only KH1 swings (decision 14) and turned shots (decision 28) carried it.
- **Change** (`win::attack::kh1_enemy_hit`, `win::summons::soras_sleight_hit` / `interrupt_note`): while a Sleights-list sleight or a summon runs, a hit whose owner is Sora, a Re:CoM shot he or his summon fired, or his summon actor gets the bit for that call (the KH1 damage scaling's owner test). No new hook or site; nothing per frame. Combat = Re:CoM unchanged.
- Not checked: in game. Docs: `recom-m12-sleights.md` § Sleight and summon hits on attacking enemies.

---

## Stage 209: Sora untouchable while his sleight dims the room (Combat = KH1), version 0.102.1 (2026-11-03)

Owner, 2026-11-03: "any sleight that sora does that dims the room should give him i-frames"; on the plan: "Go ahead, specials included".

- **The dim** = Re:CoM's sleight tint `0x1401F7D80`: `player+0x418` bit 31 set for any colour but white (`exeStockSkill` `0x140218950` at a sleight's first update when its start row has flag 0x20: 118 of 188 rows, Checked (exe); our KH1 specials with Re:CoM's colours); cleared by the sleight's end `0x1401F6660` (fade back to white) or `0x1401E7C20`. Checked (decomp).
- **Rule** (`crate::sleights::dim_shield`): Combat = KH1, Sora's story, battle room, bit 31 set, Sora's action Re:CoM's sleight action 0x16 or a KH1 special.
- **Change** (`win::attack::damage_check`, the wrapper already on `0x1401E6DE0`; `win::summons::sleight_dimmed`): when a hit arrives under the rule, Re:CoM's own protection bit `+0x418 & 1` for that check (the summon i-frames' and Leaf Bracer's path: the check's exit zeroes the hit, no damage, no reaction), put back after. One capped log line per skipped hit. No new hook or site; nothing per frame.
- Combat = Re:CoM unchanged. Not checked: in game; whether KH1 itself makes Sora untouchable during its specials (the owner's rule covers them). Docs: `recom-m12-sleights.md` § Sora untouchable while his sleight dims the room.

---

## Stage 208: M13 stage 1: KH1's Sora effect set in Re:CoM's own effect container, version 0.102.0 (2026-11-03)

Owner, 2026-11-03: "Let's work on m13", then "Go ahead" (stage 1 of `docs/recom-m13-plan.md`). Answers: three toggles (Effects / Sounds / Voices), KH1 voice lines on their own toggle, the specials switch to KH1's.

- **Asset step** (`kh1_recom_assets::effects`, `main.rs` `effects_build`): writes `raw/EFFECT/1000/EF1000.EFF` = Re:CoM's container (PLAYER 1's LINK `31000`) with its rows `ex_0010.dpx`, `ex_5010.dpx` and `ex_0010.ese` byte-identical (moved 0x20), plus KH1's Sora set `xa_ex_0010.mdls` section 1 as **`kh1_ex_0010.dpx`** (ids 0–7, 126 592 bytes) with its PC texture `-8fb40.dds` → `kh1_ex_0010_dpxbb40.dds`; Re:CoM's 4 PC assets and date kept; `kh1-effects.json`. Checked (files: run on the owner's install, every row, asset and the texture compared byte for byte).
- **Container** read by `0x1402A2A60` (0x20-byte rows, 21-character names, until a zero offset). Checked (decomp). Texture names `sprintf("%s%x", set, texture − set)` with `.` → `_` + `.dds` (`0x1402BBBC0`, format at `0x1402BBC42`). Checked (decomp + exe). Same format as KH1 (DPX `0x82`; Re:CoM's `ex_5010.dpx` = KH1's `xw_ex_5010.wpn` section 0). Checked (files).
- **Effect clock** 30 Hz in both (Re:CoM `0x140474F60` steps `0x1404797F0` once per 2.0 of dt × 0.02; KH1 `0x1401ECB50`). Checked (decomp). Re:CoM never reads a program's `+0xFC` sound (only `.ese`). Checked (decomp: no reader).
- **DLL** (`win::effects`, `crate::effects`): reads `kh1-effects.json`; at the first battle of a session (Re:CoM's battle manager ctor, already wrapped) calls Re:CoM's effect-by-name `0x1402A4920` (site `EFFECT_CHECKED`, 16 bytes) once per set and logs whether Re:CoM loaded it. No hook, no patch, nothing per frame.
- **Not yet:** nothing plays the KH1 set (stage 3 writes the per-move events; stage 2 the `.ese` sounds). Keyblade sets move to stage 4 (with their own groups: only the equipped one loaded).
- Assumption: the new texture's tag is unused (Square's `0x48…` pattern copied). Assumption: Re:CoM's own effects are unaffected by their rows moving (the container is read by offset; to check in game: the Kingdom Key trail and Sora's sleight effects).

---

## Stage 207: KH1 specials: the prompt checks Ragnarok last, version 0.101.6 (2026-11-01)

Owner, 2026-11-03, on 0.101.4, answering which special the Triangle prompt picks: "Ragnarok last". `specials::ORDER` is now Trinity, Ars Arcanum, Sonic Blade, Strike Raid, Ragnarok (indices [4, 1, 0, 2, 3]). KH1's walk (`0x14052834C`, [4, 3, 1, 0, 2]) is kept as `KH1_ORDER` for reference. The best score still wins, and the first among equals. Result: the prompt shows Ragnarok only when none of Ars Arcanum, Sonic Blade or Strike Raid is equipped and in range (Strike Raid's 900 covers Ragnarok's 600, and it costs 2 MP against 3). The Sleights list is unchanged. No new addresses. Checked (code + tests: `the_offer_is_kh1s`); Checked (exe) KH1's own order. Not KH1's: the owner's choice.

---

## Stage 206: Sonic Blade in the Sleights list, version 0.101.5 (2026-11-01)

- Ask (owner, 2026-11-03): Sonic Blade "Add it" to the Sleights list.
- Listed under Re:CoM's Sonic Blade sleight id 9 (after Strike Raid, before Ars Arcanum), named by KH1's word 319 "Sonic"; usable exactly when its prompt would offer it (within 6 m, not more than 1.6 m above, 2 MP, on the ground, in battle); choosing it sends its Triangle's command 0x4B. The Customize shortcut picker keeps only the 27 sleight abilities (specials can't be shortcuts).
- Asset step: not needed (word 319 already there). Checked: tests, Windows build. Not checked: in game.

---

## Stage 205: Ragnarok offered against any target, version 0.101.4 (2026-11-01)

- Ask (owner, 2026-11-03): "Any target" (not KH1's: KH1 offers it only for a target at least 160 above Sora).
- `specials::RAGNAROK_HEIGHT_RULE = false`: the prompt and the Sleights list share the check; range 600, 3 MP, the ground rule and the move unchanged; Sonic Blade keeps its KH1 height rule; Ragnarok's help line updated. Side effect: KH1's prompt checks Ragnarok before Ars Arcanum and Sonic Blade, so with Ragnarok equipped and 3+ MP the prompt shows Ragnarok for any enemy within 6 m (the other two still in the Sleights list; owner question).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 204: Fix: the KH1 specials' merge (0.101.0–0.101.2 didn't build), version 0.101.3 (2026-11-01)

- The merge of M12 step 7 into testing hit three conflicts with 0.100.5 / 0.100.6 (`sleights.rs` doc lines, `win/command_menu.rs`'s Sleights-list fill, `docs/recom-m12-sleights.md` § 8.5.3), and 0.101.0–0.101.2 were committed with the conflict markers in them: those three versions didn't compile and were never built or installed.
- Resolved: the command menu fills the Sleights list (and the shortcut picker) with the equipped sleights plus the equipped KH1 specials (Strike Raid, Ars Arcanum, Ragnarok) whenever the bottom row is Sleights or a shortcut holds a sleight; the specials' doc section is § 8.5.4.
- Checked: tests (all green), Windows build. Not checked: in game.

---

## Stage 203: M12 step 7: Ragnarok (KH1 FM's triangle prompt; also in Sleights), version 0.101.2 (2026-11-01)

- Ragnarok (Checked (KH1 decomp + files)): wind-up 0xF0, 5 rising hits on their own (rows 0x36..0x3A, hops 60 / 40 / 40 / 40 / 40, speeds 8 / 4 / 4 / 4 / 8); the charge 0xF6 (Sora hovers, hittable), "Impact" 0x5B lit while t is 10..70 frames; fired by t: 10 shots, 17 and a hop back at 40..50, 25 and the big release at 50..60; then the fall and landing; range 600, 3 MP, 4 AP. Re:CoM side: case 0xB (id 0x1F), tint `0x1407C5D90`, `limit3`, pack 11011. In the Sleights menu as command 0x8B.
- KH1 fires real objects: Re:CoM's own homing bullets made as Re:CoM's Ragnarok makes them; a wrapper on Re:CoM's homing step applies KH1's curve and 5° turn each frame per shot, then Re:CoM's step (owner: "Ship it", as Fire's re-target). Damage = KH1: KH1's record 0x5748 (Magic × 2, power 20).
- Assumptions: the shots start at Re:CoM's hand point; the turn's direction; KH1's 0–7-frame per-shot delay not carried; random life / curve from our stream (KH1's ranges); Re:CoM's opening-strike sounds; Re:CoM's resource wait skipped. Offered only for a target 1.6 m above Sora (KH1) in this version; owner: any target (next).
- Asset step: re-run (slots 257..265). 705 byte checks match. Checked: tests, Windows build. Not checked: in game; no KH1 trace.

---

## Stage 202: M12 step 7: Ars Arcanum (KH1 FM's triangle prompt; also in Sleights), version 0.101.1 (2026-11-01)

- Ars Arcanum (Checked (KH1 decomp + files)): wind-up 0xD2, 7 hits on their own (rows from 0x22), then 5 follow-ups (0x58 / 0x59) ending in the finisher; the first follow-up ends the invincibility; range 300, 3 MP, 4 AP. Re:CoM side: case 10 (ids 0x11..0x1E), tint `0x1407C5D60`, `limit2` (checked by its string), pack 11010. In the Sleights menu as command 0x8A.
- Also: a clash now ends either special (Sonic Blade too) with the recoil, as KH1 does (before, Sonic Blade went on after a clash).
- Asset step: re-run (slots 243..256). Checked: tests, Windows build. Not checked: in game.

---

## Stage 201: M12 step 7: Strike Raid (KH1 FM's triangle prompt; also in Sleights), version 0.101.0 (2026-11-01)

- Ask (owner, 2026-11-03): "Strike raid are arcanum and ragnarok in KH1FM are triangle prompts. Let them be triangle prompts and also show up in the sleights menu."
- Strike Raid (Checked (KH1 decomp + files)): wind-up 0xE6, then up to 5 throws on rows 0x2F..0x33 (high when the target is more than 200 above Sora), each with a catch and a wait; follow-ups 0x5F / 0x60; a clash knocks the Keyblade back; range 900, 2 MP, 3 AP. KH1 throws it inside the motion's weapon bone (not an object): no bullet. Re:CoM side: `initStockSkill` case 8 (id 0xD), tint `0x1407C5D70`.
- Learned with Re:CoM's sleight of the same name, equipped with KH1's AP; a Triangle prompt as Sonic Blade; in the Sleights menu (command 0x88, KH1's name) sending the same command. Damage = Re:CoM: Re:CoM's sleight values; Damage = KH1: KH1's records. Combat = Re:CoM unchanged. `SPECIAL_SR_CHECKED`.
- Asset step: re-run (motion slots 234..242, words). Checked: tests, Windows build. Not checked: in game; no KH1 trace.

---

## Stage 200: Damage = KH1: sleights scale on bosses / Re:CoM-only enemies; summons scale too, version 0.100.6 (2026-11-01)

- Ask (owner, 2026-11-03, on 0.100.4): "Scale them too"; summons "Same rule" (this replaces the earlier "Re:CoM's summon damage at both Damage settings" at Damage = KH1).
- (a) Bosses and Re:CoM-only enemies: the ratio uses the HP the mod already cuts them to (proxy, proxy boss or boss twin) / Re:CoM's max HP (`0x14017F5A0`). (b) Summons: hits from the summon object (`CSummon*` by its vtable, `+0x248` = Sora), its shots (Re:CoM writes the summon's id as the shooter, type 0x1000) and Sora's own hits while it plays; Idyll Romp / Cross-slash+ now scaled too. The Teleport / Lethal Frame / Shock Impact / Magnet Spiral / Gravity exceptions kept. Seven pointer checks (`SUMMON_ACTOR_PTRS`, each vtable's +0 / +0x28; a mismatch turns off only the summon scaling). No new hooks, byte sites or per-frame work.
- Assumptions: Re:CoM's own non-damage list decides per hit; a summon's shot after it has gone, and a boss part whose HP the mod never set, keep Re:CoM's damage (logged).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 199: M12 step 6: sleights as Customize shortcuts, version 0.100.5 (2026-11-01)

- Ask (owner, 2026-11-03): "I also would like sleights to be able to be added to the customize menu, for shortcuts."
- A sleight goes in the existing shortcut byte as its Re:CoM id (the 27, all ≥ 8; older builds read it empty; KH1's fill-on-learn never overwrites a sleight slot). Customize picker: learned spells, then equipped sleights, then None. The L1 list: a sleight slot follows the Sleights list's rules (battle, ground or Re:CoM's air rule, nothing running, MP by tier, KH1's commit gate) and starts through Re:CoM's own sleight start; its packs preloaded when the list comes up. Unequipped later: the byte kept; the Customize row greyed with a help line; in the L1 list a grey dash and a buzz. (The shortcuts held spells only, not items.)
- Assumption: a first use never before its pack loads. Asset step: not needed. Checked: tests, Windows build. Not checked: in game.

---

## Stage 198: Sleights at Damage = KH1: damage scaled to the enemy's KH1 HP, version 0.100.4 (2026-11-01)

- Ask (owner, 2026-11-03): "Scale to KH1 HP".
- Just before Re:CoM's hit handler `0x14017F7E0`, a sleight hit's value × the enemy's KH1 max HP / its Re:CoM max HP (`0x14017F5A0`): Sora's own collisions while the sleight runs and the shots he fired. Durations / HP percentages (Teleport, Lethal Frame, Shock Impact, Magnet Spiral, Gravity) left as they are, by Re:CoM's own list. Enemies without a KH1 twin keep Re:CoM's damage (logged; owner question on bosses / Re:CoM-only enemies). Damage = Re:CoM unchanged. Assumption: every sleight hits through Sora's collisions or shots (others logged, not scaled).
- No new hooks or sites. Checked: tests, Windows build. Not checked: in game.

---

## Stage 197: Sleights: Re:CoM's seven air-start sleights usable mid-air, version 0.100.3 (2026-11-01)

- Ask (owner, 2026-11-03): "Allow Re:CoM's air ones".
- Re:CoM's sleight setup `0x140220DF0` gives an air version only to 0x24, 0x2B–0x2F and 0x32 (Checked (decomp)): exactly Homing Blizzara, Warpinator, Warp, Bind, Confuse, Terror, Teleport. Usable in KH1's take-off, air and flight states; the rest greyed in the air; summons ground-only. Not read: Re:CoM with a ground-only sleight in the air. Assumption: the hand-back to KH1 controls after landing.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 196: Sleight abilities: a short description each, version 0.100.2 (2026-11-01)

- Ask (owner, 2026-11-03): "Write short ones"; the name stays "Lethal Frame".
- Each of the 27 sleight abilities' help line describes it and ends "Uses n MP." (Idyll Romp / Cross-slash+: "Uses all MP."), within the 66-character box. Checked: elements from Re:CoM's sleight table (files); how each hit acts from the decomp (Freeze / Bind / Warp / Confuse / Terror resist checks; Quake grounded enemies only; Teleport Stop's kind for 18000 ticks; Lethal Frame Stop's state; Synchro copies HP). Assumption: each look (thrown Keyblade, ring of fire …) from memory of Re:CoM; sources tabled in `docs/recom-m12-sleights.md` § 8.4.1.
- Checked: tests, Windows build.

---

## Stage 195: Wizard and Aquatank untouchable while they grey the room (on Re:CoM's AI), version 0.100.1 (2026-11-01)

- Ask (owner, 2026-11-03, on 0.99.1): the Wizard and the Aquatank "Same rule".
- Re:CoM (Checked (decomp + exe)): one greying state each; its begin asks the colour fade `0x140232170` for 0.5 grey (mask 0x1800, slot 0, 30 frames), its end for normal: the Wizard's state 4 (begin `0x140195590`, vtable `0x140554A58` +0x148: `e025_052`, about 180 frames of `_ex_2150` shots near Sora, `e025_054`); the Aquatank's state 3 (begin `0x14018AA70`, vtable `0x140532FE8` +0x130: `e020_051`..`053`, one `_thunder_all` shot over Sora). Move identities from motion / shot names (Assumption).
- Built (no new hook, no vtable writes; the bosses' revenge tail doesn't exist on normal Heartless): right after that begin, with Re:CoM's own vtable (Re:CoM's AI) and a fresh grey request, Re:CoM's own hurt-bodies-off; hits arriving anyway dropped by Re:CoM's rule for an attacking enemy, no Guard flinch; Re:CoM's state end `0x14017DA90` turns them back on. Stop / stun / defeat and the 0.83.0 cooldown unchanged. Checks: `DIM_NORMAL_PTRS` (6 vtable pointers), `DIM_NORMAL_CHECKED` (both begins and Re:CoM's state 3 / 4 begin wrappers).
- Assumption: a battle ending mid-move leaves a harmless entry, cleared at the next.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 194: M12 step 5: the Sleights menu (Left toggles Summon / Sleights; equipped sleights; MP by tier), version 0.100.0 (2026-11-01)

- Ask (owner, 2026-11-03): "If you press left on the d-pad, instead of summons at the bottom of the command menu, it should say \"sleights\" In the sleights menu I only want ACTIVELY EQUIPPED sleights to show in the list."; answers: Left anywhere; MP by tier (1 / 2 / 2 / 3; Idyll Romp / Cross-slash+ all MP).
- Command menu at Combat = KH1: Left on any root row switches the 4th row Summon ↔ Sleights (cursor kept; KH1's Left → Attack dropped; Left in an open list still closes it); saved per save at `save+0x26B6` (the summon block's last byte; older saves read Summon).
- The list: only the equipped sleights (`equipped_sleights()`, Re:CoM's order), placed and drawn as the Summon list (KH1's look: the Summon panels, no title). Usable as summons: battle room, on the ground, no sleight running, enough whole MP. Greyed when none usable; locked with the error sound when none equipped or one plays.
- Performing: KH1's commit gate, then Re:CoM's own sleight start (the summons' path, `0x1402247A0` via Sora's vtable `+0x130`); MP paid once Re:CoM took it; its pack preloaded on cursor. Re:CoM's sleight code (`0x140218950`) sets its own protection; Re:CoM's damage at either Damage setting. Checked (decomp): the start covers all 27, reads no card; Synchro an enemy reaction (kind 0x13 in `0x14017DC50`).
- Assumptions: ground-only (the summons' rule); Warp / Warpinator / Teleport not read one by one; a first use never before its pack loads; no MP charge from sleight hits. No new hooks or sites.
- Asset step: re-run for KH1's look ("Sleights" and the 27 names in KH1's font); Re:CoM's look needs none.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 193: Bosses untouchable during moves that dim the room, version 0.99.1 (2026-11-01)

- Ask (owner, 2026-11-03): "anytime bosses use a move that dims the room, just make them invulnerable for that move". Combat = KH1 in Sora's story.
- What dims (Checked (decomp + exe)): Re:CoM's scene colour fade `0x140232170(mask, frames, &rgba, start, slot)` writes one of four colour slots at `[0x14087B7F0]`; the map and sky (`CMap` / `CSky`, mask 0x1800) take it; a boss state asks for 0.5 grey and its end for 1.0.
- Who (every caller read): Riku 2 (spawn `+0x294` > 0), 5, 6, 7; Larxene 3 (at its lightning end), 4, 5, 8; Axel 4, 5; Marluxia 5, 6, 7; Marluxia 2 6; Marluxia 3 7, 9 and its own slot 1 (its idle think darkens before some attacks). Never: Vexen, Cloud, Hades, Hook, Leon, the hooded figure, every Heartless boss.
- Detection: the room colour copied at the boss's state change (the revenge callback) and compared at the attack's hurt-bodies-off step; partway (Larxene 3 / 4, Marluxia 3 7 / 9) four step slots compare before / after Re:CoM's step (the only new per-frame work: two 64-byte reads and a compare while in that state, not measured); Marluxia 3's attack starting under its dark slot 1 counts.
- The change: the revenge move's untouchable window (Re:CoM turns the hurt bodies off; its hit handler drops the hit; no Guard flinch, no revenge count) until the state ends (`0x14017DA90`); works with `boss_revenge = off`. The colour function's 86 first bytes checked.
- Assumptions: the dim the owner sees is this fade (no effect files read); move names from motion names; no field menu leaves Marluxia 3's slot 1 dark; which Riku fights have `+0x294` > 0 not read.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 192: M12 step 4: the 27 sleight abilities on Sora's Abilities page, version 0.99.0 (2026-11-01)

- Ask (owner, 2026-11-03): "When unlocking each sleight (not including friend sleights, the generic magic sleights, and any sleights that have a KH1 combo modifier/finisher equivalent) it will be added as an ability that you can equip with AP, with reasonable AP costs relative to their damage."; answers: added when Re:CoM teaches it; the 25 magic sleights + Idyll Romp and Cross-slash+; the owner's AP costs (2 / 3 / 4 / 5).
- On Sora's Abilities page after the KH1 abilities, in Re:CoM's sleight order; each appears unequipped once Re:CoM's own check (`0x14023AB00`) says Sora learned it (Re:CoM's learned bits read, never set: trophy 0x45 untouched). Equipping: the existing AP rules. `win::abilities::equipped_sleights()` and `abilities::sleight_mp` (the owner's MP by tier) for the next step; nothing performs a sleight yet.
- Save layout v6: ability words 64-bit, bits 32.. in 8 new bytes at `save+0x26B7..=0x26BE` (after the summon block; Checked unused), read only at v6; v5 saves keep everything (sleights already learned arrive without notices).
- Notices: Re:CoM's own learned box with its wording and name; none on a save's first load; none when Re:CoM just showed its own screen for it (the level-up Sleight pick `0x14010EF10` case 6 row 2, its event / chest screen; caught in the existing level-up wrapper, no new hook). Limits: that record lasts the session; a sleight taught with no screen gets its notice at the next Attack press, Abilities page or level-up end.
- Checked (files): names against Re:CoM's `SY0002.CTD` (it says "Lethal Flame"; the page uses the owner's "Lethal Frame": owner question). Help lines generic for now.
- Checked: tests (1053), Windows build. Not checked: in game. Asset step: not needed.

---

## Stage 191: AP Up keeps Re:CoM's +25 CP unseen; no card help after the level-up Sleight pick, version 0.98.9 (2026-11-01)

- Asks (owner, 2026-11-03): AP Up "Keep CP growing"; the level-up Sleight pick's help page "Skip it".
- AP Up at Combat = KH1 keeps Re:CoM's +25 CP (`0x14010F6AB`) while CP < 1625 (Re:CoM's own offer cap: it never passes 1625), undone above it; the row still reads AP Up / Max AP +2. A switch back to Re:CoM finds Re:CoM's CP for the picks taken.
- The level-up Sleight pick: CLevelUp case 8 makes its CTechHelp and goes 8 → 9 (waiting on `0x14087A4B0`): ended before its page at Combat = KH1, as 0.98.1 does after 0xF → 0x10 (Checked (decomp)).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 190: Fix: the mod switched itself off at start (two vtable checks as raw bytes), version 0.98.8 (2026-11-01)

- Owner (2026-11-03, on 0.98.7): "The \"KH1\" tab in the camp menu should ALWAYS be visible, i loaded into a save and it wasnt there". Cause (Checked (log)): `site CHANGED 0x14051b4a8 CTechHelp vtable` and `0x140512c10 CLevelUp vtable`, then `result: checks FAILED. The mod changes nothing on this build.`: 0.98.1 checked the two vtables' slots as raw bytes (`0x14012C230` …), which the loader relocates, so every start failed and the whole mod stayed off (no KH1 tab, no KH1 combat). Since 0.98.1.
- Fix: the two vtables are `sites::M12_PTRS` (four `PtrSite`s: CTechHelp +0 / +8 → `0x14012C230` / `0x14012D330`, CLevelUp +0 / +8 → `0x14010EB80` / `0x14010EF10`), checked lazily by `win::notices` (a mismatch turns the notices off, logged, not the mod). A new test fails any byte site holding an absolute exe pointer.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 189: Tests: the level abilities' test on the new AP rule, version 0.98.7 (2026-11-01)

- The merge of M12 steps 1 and 3 left one test on the old `max_ap(level)` and a `State` without `ap_ups` (test code only; the DLL is unchanged in behaviour): now `max_ap(7)` (3 + 7 AP Up picks) and `ap_ups: 0`. 0.98.6's pushed state failed these two tests; its DLL was built and is the same code.
- Checked: tests (all green), Windows build.

---

## Stage 188: M12 step 3: Second Chance, version 0.98.6 (2026-11-01)

- Second Chance (KH1 ability 25, Rod level 90, 5 AP): KH1's `0x1402BFA80` (Checked (decomp)): a hit that would kill from above 1 HP leaves 1 HP; at 1 HP the next hit kills (a combo's next hit too).
- Re:CoM's damage check already has this rule for one enemy card (`0x1401E720C`): Second Chance sits there: Re:CoM's HP store at `0x1401E7238` (16 bytes checked with 150 bytes of context) jumps to a 62-byte stub leaving 1 HP, armed only for Sora with a hit arriving and Second Chance equipped; Re:CoM's take-all-HP kind 0x22 skipped (Assumption: as KH1's raw hits). Exe scan: no branch or pointer into the patched bytes. 1186 sites match.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 187: M12 step 3: Berserk, version 0.98.5 (2026-11-01)

- Berserk (KH1 ability 26, Rod level 60, 1 AP): KH1's `0x1402B2DC0` (Checked (decomp)): +4 Strength to swings while HP is above 0 and under 20 % of max (under 9 when max HP ≤ 44), tested at the swing's start. Damage = KH1: into KH1's formula. Damage = Re:CoM (owner: a % bonus): the Re:CoM hit × KH1's damage with the +4 / without, for that hit on the enemy's KH1 stats (KH1's table doubles every 4 steps: usually about × 2).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 186: M12 step 3: Leaf Bracer, version 0.98.4 (2026-11-01)

- Leaf Bracer (KH1 ability 62, Rod level 39, 5 AP): KH1 (`0x14028F140` → `0x140297F30`, Checked (decomp)): Cure on oneself blocks damage and flinching for the whole cast motion. Ours: Re:CoM's own protection bit (`+0x418 & 1`, the summons' one) while our KH1 Cure casts (the end ±1 frame, Assumption).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 185: M12 step 3: MP Rage ×2, version 0.98.3 (2026-11-01)

- MP Rage (KH1 ability 24, Rod levels 42 and 75, 3 AP each): KH1's `0x14029BBD0`: MP charge = HP lost × equipped copies (the two stack; Checked (decomp)). Sora's HP is Re:CoM's (several times KH1's): the HP lost first taken as the same share of KH1's max HP at his level (18 + 3 at each of KH1's 23 HP levels) (Assumption, owner question). Combat = KH1; learned notices.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 184: M12 step 3: MP Haste, version 0.98.2 (2026-11-01)

- Ask (owner, 2026-11-03): KH1's level abilities "added to his ability list at the levels he would normally get them".
- MP Haste (KH1 ability 23, Rod level 21, 3 AP; Checked (btltbl)): KH1's `0x1402BF330` adds 1.5 to the MP charge factor per hit (same 250 % cap): at equal levels a swing gives 7 instead of 3, a finisher 15 instead of 6 (Checked (decomp)). In KH1 it speeds MP from hits, not the refill rate. Learned at its level on Re:CoM's level whatever the level-up pick, unequipped, on the Abilities page; Combat = KH1 only; a learned notice.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 183: M12 step 2: "Sora learned X" notices on Re:CoM's sleight-learned screen, version 0.98.1 (2026-11-01)

- Ask (owner, 2026-11-03): "I want there to be a notification that lets the player know that they've obtained those abilities"; answer: Re:CoM's sleight-learned screen.
- `notify_learned(...)` queues a notice shown on Re:CoM's own learned screen (`CLevelUp` sleight mode `0x14010E9C0`) in its "ability" wording (id 0x5A) with our ability name. After a level-up: the same screen reused (the game waits as for Re:CoM's own). Otherwise: when the field would let the camp open (Re:CoM's own check `0x1401A96C0`, minus the Start press); one atomic load a frame while nothing waits.
- At KH1 the card-stock help page `CTechHelp` is ended before it shows (also after Re:CoM's own event / chest sleight screens at KH1).
- Used now for every ability from a level: Combo Plus ×3, Air Combo Plus ×2, Ripple Drive, Hurricane Blast, Gravity Break, Aerial Sweep, Combo Master, Counterattack, Vortex. A save's first sync shows none.
- Assumptions: our English words fit; Re:CoM's ability wording (`0xA80710`) has one slot for the name; the field holds still under a field notice; the skipped help page first runs after its parent's update.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 182: M12 step 1: AP Up replaces CP Up at Combat = KH1 (3 + 2 per pick, no cap), version 0.98.0 (2026-11-01)

- Ask (owner, 2026-11-03): "On level up, instead of having the choice to level up CP, when the combat is set to KH1, it should be AP up, and it should give you 2 AP each level. this should be the only way to obtain AP, aside from the AP you start the game with."; answers: no cap; old saves: each CP Up already taken counts as an AP Up.
- AP = KH1's start 3 (Checked (decomp `0x1402852C0`)) + 2 × AP Up picks; KH1's level grants (`AP_UP_LEVELS`) removed. The count at `save+0x26A8` (the ability block's first reserved byte, layout v5; Checked (code) unused before).
- In the existing level-up wrapper on `0x14010EF10` (no new hook): the CP row always offered (even at CP ≥ 1625), reading "AP Up" / "Max AP +2", the status panel "Max AP" in Re:CoM's own number format; the pick undoes Re:CoM's +25 CP (`0x14010F6AB`) and adds one AP Up.
- Old saves: past CP picks = (CP − 275) / 25, at most level − 1, once, logged; 275 = Sora's new-game CP (Checked (files + decomp): CardInit record 0 in `SY0001.BIN`, `0x1401AD2A0` → `0x14023A040` → `0x14023A250`; exe-wide scan: no other CP writer but an unused debug function). Equipped abilities over the AP: unequipped from the most expensive down (KH1 never lowers AP).
- CP readers: the deck screen / check, the camp status page, the level-up panel; no events, shop or battle code. Combat = Re:CoM and Riku's story unchanged.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 181: Yellow Opera: a Re:CoM effect over the marked spot before the Thunder, version 0.97.3 (2026-11-01)

- Ask (owner, 2026-11-03): "Warning stand-in now".
- Re:CoM's Yellow Opera shows no warning in code (state 2 `0x140196980` / `0x140196A80` only makes the bolt); its effect set `0x140557560` holds `"_ex_2130"` 11 (the bolt) and 12 (the hit) (Checked (decomp + exe)). KH1 (Checked (files) `ex_2130_00.bd` `0x376`): its effect 14 at 200 above the mark, 30 frames before the bolt.
- Built: `Cmd::Warn` at the mark calls Re:CoM's effect call `0x1401A6B20` (the Yellow Opera as the actor) at KH1's cloud point: `"_ex_2130"` 14 first (Re:CoM appears to keep KH1's numbers), else the set's 12 (logged); ended by Re:CoM's effect end `0x1402585B0(handle, 0.0)` at the strike or when Stop / stun cancels it (new checked site `48 83 EC 38 8B D1 0F 29 74 24 20`). Nothing new per frame.
- Assumptions: Re:CoM's pack uses KH1's numbers; a handle isn't reused within 30 frames. Known: a Stop during the 30 frames can leave the warning up until it ends. Not checked: `EN0005.EET` (not in the session).
- Asset step: not needed. Checked: tests, Windows build. Not checked: in game.

---

## Stage 180: Green Requiem: its Cure heals nobody when its target can't be hit (KH1), version 0.97.2 (2026-11-01)

- Ask (owner, 2026-11-03): "Skip unhittable allies" (KH1's rule).
- KH1 (Checked (decomp)): the pick (`sys 102`, `0x1402B80A0`) does no hit test; the test is at the heal: `sys 60` → `0x1402BF330`, whose first call is the can-be-hit check `0x140292070`. A sunk Shadow can be picked, and then nobody is healed (no re-pick, no self-heal).
- Built: the pick unchanged; when the heal lands, a picked enemy on our KH1 AI with `rt.hittable == false` gets nothing (logged); Re:CoM-AI enemies keep Re:CoM's rule. The flag is copied into the enemy's tail `+0xC` at each `Cmd::Hittable` (the Cure runs while the runtime table is locked), read through `kh1_enemy::kh1_hittable`.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 179: Oogie Boogie: a dice throw that ends unbroken counts as Re:CoM's card break, version 0.97.1 (2026-11-01)

- Ask (owner, 2026-11-03): "oogie boogie is impossible to defeat. you're supposed to break one of his attacks which lowers a gate where you can hit him. can we make it so that if he uses the attack that you're supposed to break without being broken, the gate lowers?" (confirmed "After once").
- Re:CoM (Checked (decomp + exe)): the attack is his dice throw, state 2 of `CBossBoogie` (vtable `0x140670A60`): its begin `0x1403D7830` sets `+0xCD8 |= 1`; its step's only attack end `0x14017E540` at `0x1403D9402`. A break sends him to state 13 (begin `0x1403D7A40`): stagger; while the mark is set, the dice marked (a destroyed die may drop his gimmick card), the gate count `*(+0xD10)+0x2E0` + 1 up to 3, at 3 his down time and `+0x260 & 0x2000` cleared (Assumption: what stops him being hurt). The gate lowers a step per count, passable at 3 (Assumption: `CGimmickBBGHedge`, step `0x1403DA7C0`); reset when he jumps back (`0x1403D5B70`). At Combat = KH1 he could never be broken.
- Built (Combat = KH1, Sora's story): the existing attack-end wrapper watches his dice throw ending unbroken (state 2, really ended, alive, mark set, gate not fully down) and calls his own message handler (`+0x48` = `0x140181620`) with message 3 (Re:CoM's card break; direct, a posted one would arrive after state 1 clears the mark, Assumption): Re:CoM's state-13 begin does the rest. His state-13 slot `+0x220` wrapped only to log the gate count. No new hook, no code bytes moved; 7 byte and 5 slot checks.
- Differs from a card break: it lands after the throw (the roll's attack still follows). Owner question: three unbroken throws open the gate fully (Re:CoM's three steps).
- Checked: tests, Windows build. Not checked: in game; HDownTime, card chance and dice weights (parameter file not in the session).

---

## Stage 178: The KH1 Yellow Opera (its charge and Thunder); M11 stops here, version 0.97.0 (2026-11-01)

- Ask (owner, 2026-11-03): "finish the opera and requiem, but stop there" (M11; the last two of the Red Nocturne's family).
- Yellow Opera (KH1 `xa_ex_2130` / `ex_2130.bd` ↔ Re:CoM `CEnemyYellowopera` id 5), a flier, every number from the player's files (Checked (files): 70 + 6 numbers, 26 + 14 words): near Sora a short dart, then half the time the charge; far 6 in 10 the Thunder, then two darts; hit: three flee darts, a drop to 1 m, a Thunder.
- Charge: wind-up 208, the rush 209 steered toward where Sora stood in Re:CoM's rush state 3 (Re:CoM's rush power, id 1), stops on hitting him, recovery 210; Guard: KH1's recoil 211.
- Thunder: casts on KH1's frames (205 from 8, 206 from 2, 207 from 3); 80 frames into 207 it marks Sora's floor spot, 30 frames later the bolt strikes there; after the mark nothing cancels it but Re:CoM's Stop / stun (owner question). Native: Re:CoM's own Yellow Opera bolt bullet with its reaction kind 0x1C, power (id 0, kind 1) and effect set `0x140557560`, made at the strike frame at the top of KH1's area cylinder (2.56 m above the mark), KH1's 0.5 m radius, sweeping to the floor in one frame; Re:CoM's collision pass hits. KH1's Thunder skips the attack-vs-attack test (Checked (decomp)): Sora's body guard is off while Re:CoM tests this bolt; swings pass through.
- Framework: queued motions from a set frame; `Cmd::Strike`; a callback when Re:CoM's own state (Stop, stun, freeze) takes over.
- Re:CoM (Checked (exe), 24 new sites): vtable `0x1405570C8`, ctor `0x1401968B0` (only caller `0x14018906E`), alloc `0x140189055` 0xCD0 → 0xCF0; no Re:CoM EN0005.SES.
- Assumptions: the child's first frame after it is made; ±1 frame on queued starts and the bolt's first collision; flat battle floors; the flee drop's height above the battle floor; Sora's KH1 hit code doesn't make the Thunder a block.
- Asset step: ENEMY/0005 (ANN / ASD), `kh1-yellowopera.json`.
- Checked: tests (12 seeds × 20 000 frames), Windows build. Not checked: in game; no KH1 trace.

---

## Stage 177: T-posed Shadows: KH1's AI only on a motion pack that carries the KH1 clips, version 0.96.1 (2026-11-01)

- Owner (2026-11-03): "switching from recom to KH1 enemy ai in the same room results in the ported enemies T Posing" (screenshot: a Shadow in Halloween Town).
- Cause (Checked (decomp + exe + log)): Re:CoM's factory `0x140188F40` builds `CEnemyShadow` for ids 1 and 32 and `CEnemySearchghost` for 14 and 33, each id with its own files (`ENEMY/0032` …); the asset step extends only 0001 / 0014, so an id-32 Shadow ran KH1's AI on Re:CoM's plain pack: every KH1 slot past the pack's count, Re:CoM's clip lookup (`0x14024DC60` / `0x14024C760`) finds nothing, bind pose. The log's battle 28 Shadow had its actions released with the motion clock at 0. Not the switch: `enemy_ai` is read only by the ctor wrapper and the packs are the same at either setting; KH1 → Re:CoM and Combat switches can't leave an enemy without motions (Checked (code)).
- Fix: a new slot `+0x38` in our vtable copy (no new hook; Re:CoM's spawn `0x1401A8900` calls it right after the factory with (enemy, kind, id); every ported class's `+0x38` runs its init `+0x30` first, Checked (exe + decomp)): Re:CoM's init, then every slot the class's manifest plays checked against the enemy's own motion set; one missing: the swap undone before any state runs (Re:CoM's vtable back, tail marker cleared, runtime dropped): that enemy plays as at `enemy_ai = recom`. One log line per class and id.
- Assumptions: Halloween Town uses id 32 for its Shadows (the log line will say); the set's `+0x68` is the pack's slot count.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 176: The KH1 Green Requiem (its Cure on allies), version 0.96.0 (2026-11-01)

- Ask (owner, 2026-11-03): "finish the opera and requiem, but stop there" (M11).
- Green Requiem (KH1 `xa_ex_2140` / `ex_2140.bd` ↔ Re:CoM `CEnemyGreenrequiem` id 6), a flier, every number from the player's files (Checked (files)): rises in, drifts in small circles; within 10 m flies over and hovers about 1.8 m above the floor next to Sora; rounds: 80 % a Cure, 30 % a short move away, then always a Cure and a move; the cast charges about 2 s, then holds about 4 s circling and facing Sora, the heal landing about 5¼ s in; after its flinch a new Cure at once; a hit, flinch or Re:CoM's Stop cancels it.
- Who (Checked (KH1 decomp)): the most hurt ally 60 %, the second 30 %, the third 10 %; none: itself; allies = the other enemies in Re:CoM's battle list. How much: Re:CoM's own Green Requiem heal (its state 2's call), its power and sparkle effect (KH1's full heal: owner question).
- Re:CoM (Checked (exe), 1129 sites): vtable `0x1405404C8`, ctor `0x14018E570` (only caller `0x140189098`), alloc `0x14018907F` 0xCE0 → 0xD00; Re:CoM's Cure state 2 (`0x14018E640` / `0x14018E9B0`) never entered. Framework: `Cmd::Support`; asset search-table reader, STARTS / NO_SES / SKIPPED entries.
- Assumptions: ±1-frame child timing; flat floors; battle-list order for KH1's actor order on HP ties; KH1's "can the target be hit" not carried; no motion sounds (no Re:CoM .SES).
- Asset step: ENEMY/0006/EN0006.ANN, `kh1-greenrequiem.json`.
- Checked: tests (999), Windows build. Not checked: in game; no KH1 trace.

---

## Stage 175: Parasite Cage: its faint only from Fire at Combat = KH1, version 0.95.1 (2026-11-01)

- Ask (owner, 2026-11-03, on 0.95.0): faint "Fire only".
- Re:CoM's faint test `0x1403C4060` (Checked (decomp + disassembly)): the hit body at `*(cage+0xF8)+0x50`; already fainted (`+0xCD4 & 4`): prizes only; else attribute `+0x84 == 2` (`0x1403C42B4`, rolls FireFaint) and hit kind `+0x80 == 2` (`0x1403C432C`, rolls AtkFaint); every KH1 finisher is kind 2.
- Found: at Damage = KH1 our KH1 Fire set the body's attribute to 0xFF for the hit-handler call (no Re:CoM resistance on top of KH1's), so the cage never saw it as Fire.
- Built: the cage's vtable slot `+0x3E0` (`0x140669600`, old pointer checked) holds `win::battle::cage_hit_callback` calling Re:CoM's (17 first bytes and three reads checked; no code bytes move). In a KH1 battle, outside the faint: a non-Fire kind-2 hit skips the callback; our KH1 Fire reads as attribute 2 for that call (Re:CoM's FireFaint chance). Re:CoM's own Fire and all other hits unchanged; Combat = Re:CoM unchanged.
- Assumptions: a Stop-held Fire hit still matched to its shot (last 32 kept); FireFaint / AtkFaint chances unknown (parameter file not in the session).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 174: Parasite Cage: no flinch from Sora's hits; about 3 s between its attacks, version 0.95.0 (2026-11-01)

- Ask (owner, 2026-11-03): "the parasite cage shouldnt flinch when you attack, let it go through with its attacks, give it about 3 seconds between each attack". Combat = KH1 in Sora's story only.
- No flinch: each call of Re:CoM's hit handler `0x14017F7E0` on `CBossParasiteCage` (vtable `0x140669220`, RTTI checked at start) runs with Re:CoM's own super armour (`+0x260` bits 20 / 21, `RECOM_ARMOUR`), put back after, through the existing enemy-hit wrapper: damage and the damage voice only, no state 11 / 12, its attack goes on (swings, spells, turned-back shots). Kept Re:CoM's: Stop, stun, freeze, Gravity, the defeat, its faint (state 17, `0x1403C4060`), the card break and the used gimmick card letting Sora go. Skipped when Sora's Guard blocked it that frame (Guard reaction and Guard card break unchanged).
- About 3 s: 50 ticks per 60 Hz frame (Checked (decomp)); the 0.83.0 attack-end wrapper (`0x14017E540`) writes 9000 ticks (180 frames) for the cage; its four attack states end there passing (0, 6000) at `0x1403C490D` / `0x1403C49EE` / `0x1403C5149` / `0x1403C523D`. Measured from one attack's end to the first frame its think may start the next (its idle turn and its think's own roll come on top).
- Gimmick cards: 50 % HP and Guard block unchanged; finisher interrupt still works, as rare as before.
- 6 byte and 6 slot checks (1116 sites match). Assumption: the room objects 550–554 it moves are scenery. Unknown: its faint chances and think rate (parameter file not in the session). Not KH1 numbers (owner's choice).
- Checked: tests (990), Windows build. Not checked: in game.

---

## Stage 173: KH1 camp entry hidden again in Riku's story, version 0.94.1 (2026-11-01)

- Ask (owner, 2026-11-03, on 0.93.4): "Hide it" (none of the nine settings acts in Riku's story).
- `win::keyblades::wanted`: the normal top list and Sora's story only (`story(camp) != Riku`, camp `+0x21BC`); the map-only camp unchanged. 0.93.4's per-story help ids (`top::RECOM_HELP`, data sites `0x1400DEA45` / `0x1400DEAA9`) stay.
- Also recorded: Re:CoM's Stop keeps cancelling a caster's spell; a wall still ends the Blue Rhapsody's Blizzard (owner).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 172: The KH1 Blue Rhapsody (its melee, hops and Blizzard), version 0.94.0 (2026-11-01)

- Ask (owner, 2026-11-02): "Now let's bring in the remaining KH1 adjacent heartless" (M11).
- Blue Rhapsody (KH1 `xa_ex_2120` / `ex_2120.bd` ↔ Re:CoM `CEnemyBluerhapsody` id 4), a flier, every number from the player's files (Checked (files): 63 + 4 numbers, 20 + 7 requests): idle legs; flies to 35 above the floor under Sora (`0x78E`); under 2.5 m a back-off and Blizzard; 2.5–5 m one or two hops (60 % two) and its melee; farther hops toward Sora to within 7.5 m; hit state: back-off and Blizzard. Reuses `kh1_core::caster`.
- Framework: `Body::timed_hop` / `flight_hop_done` (KH1's movement state 5, `sys 82` from flight, `0x1402AF150`; back to flight at the height it left, `0x1402AFAA0`); `Cmd::AttackAt` (215 from frame 8); `Cast::direct` (a spell wait without charge).
- Melee: a steered attack in Re:CoM's melee state 3, two new .ASD records (window frames 82..100), Re:CoM id 1, KH1's Guard recoil 216; its body row type 2 (only clashes).
- Blizzard: acts 84 frames into cast 55; a Re:CoM bullet straight at 25 / frame for 60 frames; Re:CoM shot power id 0 and its effect set `0x1407C2F10`; a swing or Guard ends it (KH1's child ends at any clash), never sent back.
- Re:CoM (Checked (exe)): vtable `0x140537CA8`, ctor `0x14018C120` (`40 53 48 83 EC 20`, only caller `0x140189044`), alloc `0x14018902B` 0xCD0 → 0xCF0; state 2 never entered.
- Assumptions: the child's first frame after `sys 94`; apex ±1 frame; request bit 16 as plain; flat battle floors; Re:CoM's sounds. Not ported: element rules, leash, effects (M13).
- Asset step: ENEMY/0004 (ANN / ASD / SES), `kh1-bluerhapsody.json`.
- Checked: tests (985), Windows build. Not checked: in game; no KH1 trace.

---

## Stage 171: KH1 camp entry in Riku's story (Combat and Movement), version 0.93.4 (2026-11-01)

- Ask (owner, 2026-11-03): "Show it" in Riku's story, with only Combat and Movement.
- Why it was hidden (Checked (decomp + disasm)): only our own `wanted` rule (camp `+0x21BC` / `0x1401AAAE0`); Riku's camp is the same CCamp code with per-story data picked by `+0x21BC` (labels `0x1407C1000`, top help ids in the cursor set `0x1400DEA10`, header titles `0x1400D3890`, deck submenu, panels); 7 entries, same positions, same state-0x11 table.
- Fix found: our cursor set wrote Sora's help ids 0x500400 + i; Riku's entries 0 / 5 use 0x50042A / 0x50042C: `top::RECOM_HELP` per story, checked as data sites `0x1400DEA45` (57 B) / `0x1400DEAA9` (49 B). 1077 sites match.
- Built: `wanted` = not the map-only camp (`+0x20A4`, no top list; kept). Riku's story: rows Combat, Movement; Riku help lines. No new hook. None of the nine settings acts in Riku's story (Checked (code): KH1 battles need Sora's story; Guard / movement need CPlayerSora): values kept for Sora's story (owner question).
- Assumption: nothing in Riku's layout at our entry's spot (86, 270). Checked: tests, Windows build. Not checked: in game.

---

## Stage 170: Red Nocturne: KH1's rule for a big Fire whose caster dies, version 0.93.3 (2026-11-01)

- Ask (owner, 2026-11-03): "Match KH1" (a big Fire flies if its caster dies late in the cast).
- Finding (Checked (files `ex_2110_01.bd` `0x734..0x754`; decomp)): KH1 never reads the caster's HP; it checks the caster still exists and, while it does, that it still plays its cast motion 203. The killing hit's reaction replaces 203, so in KH1 a caster killed by a reacting hit throws no fire: the question's premise was wrong.
- Built: KH1's exact rule: a defeated Red Nocturne's waiting spells keep counting until Re:CoM removes it (30 frames later): in practice no fire follows a kill. Owner question: keep, or a non-KH1 rule (a dead caster's waiting big Fire flies at its 90th frame).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 169: Red Nocturne: Re:CoM's stun when its own sent-back Fire hits it, version 0.93.2 (2026-11-01)

- Ask (owner, 2026-11-03): "Re:CoM's stun" instead of KH1's 30 s dizzy.
- Its own sent-back Fire's hit becomes Re:CoM's stun kind 7 before Re:CoM's hit handler `0x14017F7E0`; Re:CoM puts it in its own stun (`+0x900`, state 17, Stun Impact's; its stun gate allows it, `EN0003.EPD`, Checked (files)): about 5 s with its stun motions. KH1's EXP bonus at once. After the stun the takeover rules drop what played and it flees and casts a big Fire (KH1's after-hit; Assumption that KH1 runs it after its dizzy). No new hook. Checked (decomp + files), tests, Windows build. Not checked: in game.

---

## Stage 168: Bouncywild: a slip loses nothing; its thrown banana makes Sora slip, version 0.93.1 (2026-11-01)

- Asks (owner, 2026-11-03): slip money "Nothing lost"; banana "Slip too".
- Re:CoM's slip calls its Moogle-point scatter `0x1401F6190` from one place, `0x1401E7665` (Checked (decomp + exe; no branch into it)): that call goes through `win::kh1_peel::slip_scatter`, which skips the scatter only for a slip from our KH1 Bouncywild's peels / banana (the damage-check wrapper tells the hit by Sora's body `+0x690+0x98`). Every other slip scatters, Re:CoM's own Bouncywild included. 4 byte-checked sites.
- The KH1 banana carries Re:CoM's slip kind 0x18, set as Re:CoM's banana sets it (`0x14018C924`); Re:CoM's banana damage; guarded or batted into an enemy, a normal hit.
- Assumption: the peel is still in our list when Sora's damage check runs. Checked: tests, Windows build. Not checked: in game. Asset step: not needed.

---

## Stage 167: KH1 camp entry always shown; its Combat and Movement settings pages; Re:CoM's Config menu restored, version 0.93.0 (2026-11-01)

- Asks (owner, 2026-11-03): "Under the \"KH1\" tab in the pause menu, I want run speed, jump height, and air speed under a sub menu called \"Movement\" I also want the guard Reactions, combat, and enemy pace to be under a seperate sub menu in the KH1 tab called \"Combat.\" Also, Just have the \"KH1\" Tab in the camp menu always be there, regardless of if the combat is set to ReCoM or KH1"; answers: move the three rows out of Config; at Re:CoM only Combat + Movement; add Enemy AI, Boss revenge ("renamed to Revenge Value"), Dodge roll.
- KH1 entry: shown at any Combat setting in Sora's story (hidden in Riku's story and the map-opening camp, as before). Combat = Re:CoM: Combat, Movement; Combat = KH1: Items, Customize, Abilities, Combat, Movement.
- Combat page: Combat, Guard reactions, Enemy pace, Enemy AI, Revenge Value (ini `boss_revenge`). Movement page: Run speed, Jump height, Air speed, Dodge roll (Re:CoM / KH1 / Off).
- Native: each page is Re:CoM's own Config screen running our rows: its draw `0x1400DF1B0` (now called, not hooked; 10 first bytes checked) keeps the window, cursor, open / close, cleanup, sounds, help line; only its row-building step is ours, at Re:CoM's own row positions. Changes saved through `settings::store`. The input hook `0x1400D3EB0` only routes the KH1 entry's pages.
- Re:CoM's Config menu: our draw hook removed with its extra rows and window stretch (no data patches): its own five rows, as unmodded. `config_menu = off` now means no KH1 entry.
- Effect: at once: Combat (read at each battle start), Guard reactions, Revenge Value, the four movement keys (read every KH1 frame; KH1 only); from the next battle: Enemy pace, Enemy AI. Changing Combat in the page redraws the entry's rows on close.
- Checked: Re:CoM decomp (Config input states, draw modes, camp states 0x4A–0x4C, teardown), Steam exe bytes; tests, Windows build. Assumption: no jump / roll in progress while the camp is open. Not checked: in game. Asset step: not needed.

---

## Stage 166: The KH1 Red Nocturne (its Fire and big Fire; the casters' shared module), version 0.92.0 (2026-11-01)

- Ask (owner, 2026-11-02): "Now let's bring in the remaining KH1 adjacent heartless" (M11).
- Red Nocturne (KH1 `xa_ex_2110` / `ex_2110.bd` ↔ Re:CoM `CEnemyRednocturne` id 3), a flier, every number from the player's files (Checked (files)): rises in, hovers in small circles until Sora is within 10 m; within 7 m 40 % of rounds a strafe casting a Fire, then a slow circle; beyond 7 m half the time a big Fire, then closer; hit close: flees 5–6 m and casts a big Fire. No attack rows of its own: only Re:CoM state 1 (states 2 / 3 never entered).
- Spells (each waits on its caster's motion, as KH1; a reaction mid-cast stops it): the Fire leaves 40 frames into its cast, the big Fire 90; Re:CoM bullets with KH1's numbers: 50 ahead and 80 (big 70) up, 12 (22) a frame, curving 1° (2°) a frame toward Sora, 90 frames; Re:CoM's own Red Nocturne shot powers (id 0 / 1) and effect sets (`0x140548EF0` / `0x140548F60`). Guard and a swing both send them back KH1's way (KH1's Guard meets them as a swing; Checked (decomp)); its own fire on it gives KH1's EXP bonus (EXP/10 + 2).
- `kh1_core::caster`: the routines the family's scripts share byte for byte (Blue Rhapsody 3, Yellow Opera 2) and the spell's wait.
- Re:CoM (Checked (exe), 1055 sites): vtable `0x140548A58`, ctor `0x140191880` (`40 53 48 83 EC 20`, only caller `0x14018901A`), alloc `0x140189001` 0xCD0 → 0xCF0.
- Assumptions: the spell's first check the frame after it is made; ±1-frame alignment; flat battle floors; Sora's aim point for KH1's lock point. Not built: its dizzy from its own fire (owner question).
- Asset step: ENEMY/0003/EN0003.ANN, `kh1-rednocturne.json` (Re:CoM has no EN0003.SES; the .ASD unchanged).
- Checked: tests (12 seeds × 20 000 frames), Windows build. Not checked: in game; no KH1 trace.

---

## Stage 165: The KH1 Bouncywild (its banana and peels), version 0.91.0 (2026-11-01)

- Ask (owner, 2026-11-02): "Now let's bring in the remaining KH1 adjacent heartless" (M11).
- Bouncywild (KH1 `xa_ex_2040` / `ex_2040.bd` ↔ Re:CoM `CEnemyBouncywild` id 8), every number from the player's files (Checked (files), 69 from the script): appearance, wander, bounce-about, a 6 m leap over Sora, the run-away dropping two peels; rounds: a peel toss, a homing banana, taunt, call, look-about; its own launch from its hit handler; it never walks up to Sora. No body attack: all in Re:CoM's state 1.
- Banana: KH1's child script is byte-identical to the Fat Bandit's / Defender's fireball (Checked (files)): the same Re:CoM-bullet code, Re:CoM's homing-banana power (id 1), Re:CoM's banana effect (`0x1407C3010`); a plain flinch (KH1).
- Peels (`win::kh1_peel`): Re:CoM bullets falling by KH1's numbers, lying 120 frames, no damage; they hit with Re:CoM's banana reaction kind 0x18 (Checked (decomp): it plays Sora's row 0x29 `p001_049`, matched as an exact bake of KH1's slip; scatters a tenth of his Moogle points; check-only site `0x14018C924`). No new player reaction.
- Re:CoM (Checked (exe), 23 sites): vtable `0x140538FB8`, ctor `0x14018C4D0` (`40 53 48 83 EC 20`, only caller `0x1401890EC`), alloc `0x1401890D3` 0xCD0 → 0xCF0.
- Assumptions: KH1's escape left out (it never leaves; owner question); the call summons no helpers; a peel's reach against Sora's body; flat floor at its throw height; our own random stream; Guard uses a peel up. Not built: other enemies slipping on peels.
- Asset step: ENEMY/0008 (ANN / ASD / SES), `kh1-bouncywild.json`.
- Checked: tests (956), Windows build. Not checked: in game; no KH1 trace.

---

## Stage 164: Neoshadow grab on KH1's frame, version 0.90.2 (2026-11-01)

- Decision (1:1 rule, 2026-11-03): the hold starts on KH1's frame.
- Cause (Checked (decomp + disassembly)): Sora was held 3 frames late: an enemy's attack check `0x14014E6A0` runs before its state machine and a requested state begins next update; attack windows open after the check; enemies update before Sora.
- Built: the Neoshadow plays 208 and its grab record in the test frame (where KH1 asks for Sora's held motion); the host starts that attack at once and runs Re:CoM's own attack windows and attack check once right after (new checked sites `0x14014E6A0` `4C 8B DC 53 56 41 56 48 83 EC 70`, `0x140523E88` → `0x14014E830`). Sora is held on KH1's frame; 208 starts a frame before KH1's; the let-go on KH1's frame (test + 96). 1031 sites match.
- Assumptions: KH1 applies the held motion in the frame its script asks; the 1 m grab sphere from the last pose still covers Sora.
- Asset step: not needed. Checked: tests (host model, two Neoshadows), Windows build. Not checked: in game.

---

## Stage 163: Gargoyle: Sora's Guard sends its ball back KH1's way, version 0.90.1 (2026-11-01)

- Ask (owner, 2026-11-03): "Guard: KH1's send-back" for the Gargoyle's ball.
- KH1 (Checked (decomp `0x1402B4A20` / `0x1402B4BC0`; files `ex_2220_00.bd`)): Guard meets the ball's row in the same pass as a swing and hands it to Sora with the same flags; the ball's script runs `0x748` alike: row `0x46C` owned by Sora, aimed between the Gargoyle and Sora's target, bouncing, hurting enemies, once.
- Built (`win::kh1_thrown`): when Re:CoM blocks the ball, its step runs the swing's send-back (`send_back_kh1`) instead of Re:CoM's homing turn-back. A wall still ends it; the Fat Bandit's and Defender's shots unchanged; with Guard reactions off a blocked ball ends as before.
- Assumption: Re:CoM's body guard stands in for KH1's guard hitbox along the Keyblade.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 162: The KH1 Defender (its shield, fireballs and flame), version 0.90.0 (2026-11-01)

- Ask (owner, 2026-11-02): "Now let's bring in the remaining KH1 adjacent heartless" (M11).
- Defender (KH1 `xa_ex_2340` / `ex_2340.bd` ↔ Re:CoM `CEnemyDefender` id 26), every number from the player's files (Checked (files): 90 from the script, 13 from the flame child `ex_2340_00.bd`, 6 from the fireball child `ex_2340_01.bd`, 36 requests): its shield kept toward Sora, KH1's repel count and bash; the strike 209, spin 208 and bash 207 in state 2 (Re:CoM's melee id 0); side steps / turn-around held at their end (KH1's request bit 16, asset `HELD_CLIPS`).
- Shield: two guard rows at once: the front (row `0xEC`, type 1, priority 99, 70°, mode 3) stays under the spin's (`0x23C`, priority 100, all round, with 208) and comes back when it ends; a row at 98+ is never moved by Re:CoM attackers (`0x1402B4BC0`).
- Fireball (Re:CoM bullet, KH1's numbers): from bone `0x106C` at t 3 (Re:CoM chin), [8, 10) a frame, turning 1° a frame toward Sora's aim point, 100 frames; Re:CoM's fireball power (id 1), its own effect set `0x14053CF30`; batted: KH1's `0x1B8` re-aim; Guard-turned: Re:CoM's homing at KH1's 1°; one per round (KH1 clears the count, `0x1630`). Flame (212): 110-frame wait, KH1's 8-slot ring, 10-frame tail, 30-frame growth, 10-frame re-hit, on id 1.
- Re:CoM (Checked (exe)): vtable `0x14053CA98`, ctor `0x14018D8E0` (6 exact bytes, only caller `0x1401893E0`), alloc `0x1401893C7` 0xCD0 → 0xCF0; state 3 (Re:CoM's fireball) never entered.
- Assumption: its own batted fireball's big hit when it strikes from the front stands for KH1's shield-part test (owner question).
- Asset step: ENEMY/0026 (ANN / ASD / SES), `kh1-defender.json`.
- Checked: tests (12 seeds × 20 000 frames), Windows build. Not checked: in game; no KH1 trace. The porting session stopped before its report; tests and build re-run at the merge.

---

## Stage 161: The KH1 Gargoyle (flier; its bouncing ball as a Re:CoM bullet), version 0.89.0 (2026-11-01)

- Ask (owner, 2026-11-02): "Now let's bring in the remaining KH1 adjacent heartless" (M11); its shot: "Re:CoM bullet, KH1 numbers".
- Gargoyle (KH1 `xa_ex_2220` / `ex_2220.bd` ↔ Re:CoM `CEnemyGargoyle` id 13), a flier, every number from the player's files (Checked (files)): rises in, roars; close rounds (circling strafe, dive or two-part swipe, back-off while faded); far rounds (ball throw, swoop, approach); KH1's hit state and reactions. All three attacks in Re:CoM state 2 (state 3, Re:CoM's own shot, never entered). Power: dive and swipes on Re:CoM's wing strike (id 0); the ball on Re:CoM's Gargoyle shot (id 1).
- The ball: one of Re:CoM's homing bullets made at the chin at KH1's frame 70 with its attack off; at frame 244 tossed up, bounces twice toward Sora, ends on the third floor touch (Checked (decomp): the child's setup, KH1's attack / floor-contact flags; Re:CoM's bullet end on a floor touch); drawn with Re:CoM's Gargoyle shot effect (`0x14053F630`), attack on through `0x14014DE90`. A swing sends it back (KH1: then it hurts enemies); Guard sends it back Re:CoM's way (decision 28).
- Re:CoM (Checked (exe), 25 sites): vtable `0x14053F198`, ctor `0x14018E150` (`40 53 48 83 EC 20`, only caller `0x1401891BE`), alloc `0x1401891A5` 0xCD0 → 0xCF0.
- Assumptions: the generic rising appearance on every battle spawn (not the perched one); KH1's fades as a hard hide / show (M13); flat battle floors (ball 2 cm above); KH1's floor test one-sided; Re:CoM's sounds. Framework: bone points by bone code, pushes checked in any mode, the converter accepts `ex_2220_sk_2`.
- Asset step: ENEMY/0013 (ANN / ASD / SES), `kh1-gargoyle.json`.
- Checked: tests, Windows build. Not checked: in game; no KH1 trace.

---

## Stage 160: Neoshadow grab from below: Sora's new held reaction, version 0.88.0 (2026-11-01)

- Ask (owner, 2026-11-02): "New Re:CoM reaction" for the Neoshadow's grab (the 1:1 option).
- Sora: Re:CoM's reaction-kind getter `0x140179410` reads a 22-kind copy (data patch of its `lea` at `0x140179417`, old `05 0E 3B 00`); Re:CoM's 21 kinds kept, held kind `0x40` at index 21. In the existing damage-check wrapper (`0x1401E6DE0`) the held kind does no damage; Re:CoM's check runs with its no-reaction bit (its gates apply); if it took the hit: Re:CoM's reaction prelude `0x1401F6990(pl, 6)` and its status entry (action 0x18, sub 0xC) on the held clip's new action row, blend 4. Re:CoM's status exe `0x1401EB0F0` ends it at the clip's end; a damaging hit frees him (action 0xC). Nothing of ours runs during it. Checked (decomp + exe).
- KH1 (Checked (files)): `ex_2420.bd` `0x1AF6`: Sora's request 200 from the Neoshadow's player set (`+0xA4B80`, 126 frames, no root motion), held by its 208 up to 94 frames, freed by the other's dive on `+0x4C`; no damage itself; KH1's grab sets no 0x80 (no post-hit frames, the dive lands); a Sora in his 30 post-hit frames is not grabbable (sys 141).
- Neoshadow: when the host says Sora is grabbable: `+0x4C`, 208 with the grab's record, the hold, release when he's no longer held; else KH1's miss (209).
- Assumptions: Sora is player set 0; the record 1.0 m for 2 frames; the hold starts 1–2 frames after KH1's; no damage face / rumble; guarding gives the miss (flag 0x400 unidentified).
- Asset step must be re-run: held clip on slot 233 in PL0001 / 0002 / 0011.ANN, `held` in kh1-motions.json, EN0027.ASD, four `grab_*` numbers in kh1-neoshadow.json (the DLL refuses an older manifest). No mod.yml change.
- Checked: tests, Windows build. Not checked: in game; no KH1 trace.

---

## Stage 159: jump_height = recom: Re:CoM's crouch, version 0.87.2 (2026-11-01)

- Ask (owner, 2026-11-02): "Re:CoM's crouch" for `jump_height = recom`.
- Re:CoM (Checked (decomp `0x1401EDA60` phase 1, `0x1401F29F0` and its 6 callers; exe `params+0x24` = 100)): the jump init zeroes `+0x5DC`; from the frame after the press, each frame Sora takes off once `+0x5DC` ≥ 100, else `+0x5DC` += dt (50 per 60 Hz frame): the press frame plus 2 crouch frames, the same time at 30 fps. A land jump gets the whole crouch too.
- Built: `speeds::RecomCrouch`, `Kh1Sora::takeoff_input`, `params+0x24` read live. `jump_height = kh1` unchanged (4 frames, 2 for a land jump).
- Assumption: KH1's jump clip still starts at frame 0 (at recom Sora leaves the ground a frame before the clip's lift).
- Checked: tests (both settings, 30 fps, a slow frame, land jumps), Windows build. Not checked: in game.

---

## Stage 158: Rows that only clash: the Soldier's spin-kick chest and the Powerwild's flurry body never hurt Sora, version 0.87.1 (2026-11-01)

- Ask (owner, 2026-11-02, clash question 2): "Fix: block only".
- KH1 (Checked (decomp `0x1402B3BE0`)): an attack is tested against a hurt body only if `((t<<17)−0x20000) & 0xFFF9FFFF ≠ 0 || t == 3`: rows of clash type 1, 2 or 4 never hurt. Soldier `ex_2010` row `0x2AC` is type 1, Powerwild `ex_2030` row `0x1CC` type 2 (Checked (files)); no other ported row is (the Wyvern's charge is type 3, it hurts in KH1 too).
- Re:CoM has no record field that skips the player (Checked (decomp `0x14016F110` / `0x14016EAD0` / `0x14016F1C0`; exe: CCollAttack vtable `0x140523E28`): the existing pair-dispatch wrapper answers 0 for these collisions against the player list (owner `CCollAttack+0x20` → `0x1402AD350` → our enemy → its live record's row type from the manifest). The clash test still sees them (swings bounce off). Sora's KH1 Guard (type 2) never met type 1 / 2 rows in KH1 either.
- Asset step: not needed (manifests since 0.82.0 carry each row's type). Per frame: one load while no class has such a row.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 157: The KH1 Fat Bandit (KH1's waits, aimed fireballs, the fire breath), version 0.87.0 (2026-11-01)

- Asks (owner, 2026-11-02): "it is nearly impossible to attack this fat bandit because it attacks so frequently"; "keep porting heartless" (M11).
- Fat Bandit (KH1 `xa_ex_2060` / `ex_2060.bd` ↔ Re:CoM `CEnemyFatbandit` id 10), every number from the player's files (Checked (files)): KH1's attack choice by distance, wander, hit reactions and waits (75 frames after the belly flop, 60 after a fireball volley, a 226-frame wind-up before the fire breath); in the model a new attack round every 293–324 frames with Sora 2–7 m away (Re:CoM's own: about 33).
- Fireball: a Re:CoM bullet moved by KH1's numbers: straight at 45 / frame for 240 frames, aimed where the head looks (no homing; Checked (decomp)); Re:CoM's fireball power and effect (`0x14053E290` / `0x14053E300`); Guard sends it back, a swing bats it back at Sora's target (KH1). Flame: KH1's chain of up to 16 spheres, each a Re:CoM bullet (radius `0x140172690`), re-hit every 60 frames, up to 140 frames while the breath plays; the fireball's power (owner). Belly flop: Re:CoM's belly power. Front guard: swings from the front bounce off.
- Re:CoM (Checked (exe), 22 sites): vtable `0x14053DDF8`, ctor `0x14018DD70` (`40 53 48 83 EC 20`, only caller `0x140189140`), alloc `0x140189127`; power getter `0x14017F0E0`, hit serial `0x14016F7D0`.
- Assumptions: the head has turned when it fires; the front check ignores height; the flame ignores walls; one flame per enemy; Re:CoM's sounds. Not ported: leash, party messages, the weak-spot stagger (owner: left out), hit attributes. Not on Re:CoM's 0.83.0 cooldown.
- Asset step: ENEMY/0010 (ANN / ASD / SES), `kh1-fatbandit.json`.
- Checked: tests, Windows build. Not checked: in game; no KH1 trace.

---

## Stage 156: Neoshadow: no fixed points (they move about their own place), version 0.86.0 (2026-11-01)

- Ask (owner, 2026-11-02, Neoshadow question 1): "Drop them": no room 29 fixed spots; they wander and attack instead.
- Built (Assumptions: the script's own place rule stands in for the room's points): a lone one, and the hidden one left alone, go to a point 600–1000 from where they are (the script's sink-away rule `0x552C(actor+0x10, 600, 1000)`), come up and call (before: room 29's spots); three or four: the place draw still runs (KH1's frames and random numbers unchanged) but each rises where it sank, together; stuck underground (`0x58AE`): turns toward Sora instead of room 29's (8700, 5300, 100) (owner question 6). Kept: the walk home after a dive (its own spawn, `0x372C`). Everything else (counts, timings, the call, sinks, rises, dives) as KH1.
- The grab from below (owner: build it) researched, not built: KH1's `0x1AF6` (Checked (files + decomp)) plays Sora's request 200 from the Neoshadow's `.mset` player set (126 frames), actions locked, held up to 94 frames by its 208, freed when a hit lands (the other Neoshadow's dive); no damage itself. Re:CoM has no player reaction that holds Sora on an enemy's clip until hit (Checked (decomp + exe)): owner question on the native design.
- Asset step: not needed. Checked: tests (1–5 Neoshadow 20 000-frame fights), Windows build. Not checked: in game.

---

## Stage 155: Crash fix: the clash spark's effect actor, version 0.85.1 (2026-11-01)

- Owner (2026-11-02, on 0.85.0): "game crashed". Crash dump (Checked): access violation reading 0x150 at `0x1401A6B23` (Re:CoM's effect-at-actor `0x1401A6B20`: `mov r8d, [rcx+0x150]` with rcx = 0), called from `win::kh1_enemy::swing_meets_attacks` (Stage 151's attack-meets-attack clash) through `attack::spark_at`, which passed a null actor.
- Fix: `spark_at` plays Re:CoM's guard spark on Sora as the actor (as `guard_clink` does); nothing without a player. Also covers the thrown scimitar's knock-away spark (same call, since 0.75.x).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 154: Large Body: KH1's stomp ring in Re:CoM's circle test, version 0.85.0 (2026-11-01)

- Ask (owner, 2026-11-02): "Research Re:CoM's first": Re:CoM's shockwave is a solid flat disk (`CCircleCollision`, type 11; its body test `0x140177190`, only caller `0x14016EAD0`; Checked (decomp)), not KH1's ring (`ex_2050_00.bd` routine `0x18`: r − 100 < d < r, |Δh| ≤ 50; Checked (files)).
- Built: a wrapper on `0x140177190` (`40 55 56 57 48 83 EC 50`, Checked (exe); no branch or pointer into the 8 moved bytes). A circle with `+0x74` > 0 is our ring (no Re:CoM code writes a circle's `+0x74`: decomp scan): KH1's rule on Re:CoM's body shape: Sora's column bottom within 0.5 m of the ring's height, flat distance strictly in (r − 1 m, r), no body radius. Every other circle: Re:CoM's own test (Re:CoM's Large Body included).
- `ring_step` writes the band and the height window each of the ring's 28 frames (Re:CoM rebuilds the circle when the window opens). Asset step: the ring record's `+0x18` = 0 (the centre is `Root`; Re:CoM's template sits 0.3 m up).
- Assumptions: `Root` on the floor for KH1's stomp spot; the column bottom for KH1's actor `+0x50`; the first ring frame may use Re:CoM's test (radius 0, inside its body). Not checked: KH1's hits per ring (Re:CoM: one per attack).
- Asset step must be re-run (EN0009.ASD).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 153: Movement toggles: run_speed (recom), jump_height, air_speed, version 0.84.0 (2026-11-01)

- Ask (owner, 2026-11-02): "have sora run at the same speed as he does in recom"; then "have run speed, jump height, and air speed all be separate toggles in the settings". Reason found: Re:CoM's homing shots (2.5°/frame for 30 frames) are tuned to be outrun by Re:CoM's Sora (6.2 m/s; KH1's 4.8).
- Ini keys at Combat = KH1, each `recom | kh1`, any combination, logged at start-up:
  - `run_speed` (default recom): run = Re:CoM's stick × 6.2 m/s (10.33 KH1 units at full tilt); walk stays KH1's 2. `kh1`: walk 2, run 8, no stick scaling.
  - `jump_height` (default kh1): `recom` = take-off 13.72 / 15.93 with Re:CoM's gravity 0.49 (read live), peak 198.9 / 266.8; letting go 12+ frames after take-off cuts the speed to 43 %; falls capped at 50. KH1's 4-frame crouch kept.
  - `air_speed` (default kh1): `recom` = 0.8 of the ground speed at take-off, easing 5 % a frame toward 0.8 × ground speed, facing 10 % a frame, drifting to a stop with the stick let go; only while free in the air (air swings and casts keep KH1's).
- Run clip: neither game ties its playback to the speed (Checked (decomp)); Re:CoM's own clip at its own speed.
- Assumptions: Re:CoM's hold after a release cut not ported; take-off air speed from the stick, not the previous frame.
- Checked: Re:CoM's numbers (decomp + exe), tests for all 8 combinations, Windows build. Not checked: in game.

---

## Stage 152: Re:CoM-AI Heartless wait their own attack-end cooldown in KH1 battles, version 0.83.0 (2026-11-01)

- Ask (owner, 2026-11-02, Fat Bandit video): "it attacks so frequently that there's no opening"; answers: keep porting; every not-yet-ported Heartless waits a longer cooldown, each enemy's own value.
- Re:CoM (Checked (exe: all 198 calls; decomp)): the attack end `0x14017E540` is always called with next = 0, so Re:CoM drops the cooldown passed (197 calls pass 6000 ticks = 120 frames; Marluxia 2's `0x1403FBA20` passes 0) and writes 250 (5 frames) at `0x14017E5F5`. The value is the same for every Heartless class.
- Built: a wrapper on `0x14017E540` (15 exact first bytes; nothing branches into them): Re:CoM's attack end, then the passed value written back, only in a KH1 battle (M6's switch), only for the 32 ordinary Heartless classes' own Re:CoM tables (our KH1-AI enemies never use them), only on Re:CoM's "no next state, 250" path. Once per finished attack; a log line per class.
- 120 frames: Air Pirate, Air Soldier, Aquatank, Bandit, Barrel Spider, Black Fungus, Blue Rhapsody, Bouncywild, Creeper Plant, Darkball, Defender, Fat Bandit, Gargoyle, Green Requiem, Heart / Spade Soldier, Large Body, Loudness, Neoshadow, Pirate, Powerwild, Red Nocturne, Screwdiver, Sea Neon, Search Ghost, Shadow, Soldier, Tornado Step, Wight Knight, Wizard, Wyvern, Yellow Opera (the ported ones only while on Re:CoM's AI). Unchanged (250): Heartless bosses, Marluxia 2, human bosses, the bee, the White Mushroom (never calls it). An interrupted attack still gets 250 after the flinch (`0x14017D6F0`; Assumption which path each reaction ends on).
- Checked: tests, Windows build, 935 sites. Not checked: in game.

---

## Stage 151: KH1's attack-meets-attack clashes for every ported enemy, version 0.82.0 (2026-11-01)

- Ask (owner, 2026-11-02, Large Body question 3): "Build them": attack-vs-attack clashes in the shared enemy guard, KH1's order of tests researched first.
- KH1 (Checked (decomp)): each frame attacks against attacks first (`0x1402B47A0` → `0x1402B3BE0` → `0x1402B4A20` → `0x1402B4BC0`, both ways), then against bodies. The lower priority gets kind 6 (7 at 40+ under), equal both 6; a side at 100 never moves and still hits; both ≥ 98: nothing; a type-0 row never repels, types 1 / 3 / 4 repel when higher. Clashing rows miss every body that frame; a clash record does no damage (`0x1402BEEE0`). Sora 0x6E / 0x6F / 0x70; the enemy its row's `+0x40`. Body spheres count (the Large Body's charge belly: type 0, priority 90).
- Built: Re:CoM has no attack-against-attack test; ours runs inside the existing swing test before the bodies, Sora's swing against each KH1 enemy's live Re:CoM attack records with Re:CoM's own overlap tests (callee `0x140177480`, byte-checked). Sora's side through the existing `swing_clash`; the enemy's in its hit-handler wrapper through our state 11 (Re:CoM's flinch when the model lacks the recoil; a higher type 1 / 3 / 4 row repels). No new hook; outside a swing's window two atomic loads per enemy update (not measured).
- With the Kingdom Key (30): both recoil: Shadow, Bandit, Air Soldier, Powerwild, Search Ghost, Darkball dash, Air Pirate swoop, Neoshadow claw, Large Body punch / slam, Soldier kicks; Sora's strong recoil and the enemy's: Pirate (rows at 90), Large Body charge (ends it); Sora's strong recoil, the enemy carries on (rows at 100): Wyvern, Darkball bite / drop, Air Pirate slash, Neoshadow rise slash / dive, Soldier jumping kick / rush leap / spin kick; never: the stomp ring (child row). The Wight Knight (merged after): its attack rows clash the same way beside its body rows (Assumption: the same outcome either way; check in game).
- Asset step: every `kh1-*.json` gains a `clash` list per attack (older manifests load with none). No mod.yml change.
- Assumptions: Re:CoM's update order (at most one frame off); Re:CoM's guard spark for KH1's clash effects (no hit-stop / rumble); recoil push bit and KH1's `+0x88 & 1` not carried.
- Checked: tests (835), Windows build, 585 byte sites. Not checked: in game; no KH1 trace.

---

## Stage 150: The KH1 Wight Knight, version 0.81.0 (2026-11-01)

- Ask (owner, 2026-11-02): "Now let's bring in the remaining KH1 adjacent heartless" (M11).
- Wight Knight (KH1 `xa_ex_2200` / `ex_2200.bd` ↔ Re:CoM `CEnemyWightnight` id 12), every number from the player's files (Checked (files): 66 constants, 39 requests, rows, windows, clips): appearance 69, then engaged at once (no idle); combo: wind-up 200, up to four alternating swipes 201 / 202, recovery 203 / 204 (a landed swipe ends it; Sora past 3.5 m: the leap slash); leap slash: crouch, a 5 m high timed jump at Sora, two strikes on landing (216), a forward flip first beyond 6 m; flips 205 / 206, circling walk with a 4 m hop when stuck, dance 212 and sway 218 (both end in the leap slash).
- Its body rows on the shared enemy guard: the wind-up gives Sora's swings the strong recoil; a swipe or the leap slash's first half second: both recoil (Sora 0x6E, it 217, then it counters). Every hit takes KH1's reaction; a killing flinch becomes a knock-back (KH1's handler).
- Re:CoM (Checked (exe + decomp)): vtable `0x140553748`, ctor `0x140194ED0` (`40 53 48 83 EC 20`, only caller `0x140189194`), alloc `0x14018917B` 0xCD0 → 0xCF0; swipes in state 3 (id 1, Checked (files): `e012_054` moves as KH1's 201), the leap slash in state 2 (id 0, Assumption: owner question).
- Assumptions: `sys 163` always true; attribute 8 / 10 hits never arrive (no attribute passed); the flips' height from Re:CoM's root motion; the apex motion a frame late; the circling turn's sign. Not ported: the leash (KH1 world 11 only), the hanging spawn, effects (M13).
- Asset step: ENEMY/0012 (ANN / ASD / SES), `kh1-wightknight.json`.
- Checked: tests, Windows build, 954 sites. Not checked: in game; no KH1 trace.

---

## Stage 149: Build fix: the merged always-on guard rows (Large Body / Neoshadow), version 0.80.1 (2026-11-01)

- Stage 148's merge left the Neoshadow's `held` test in the guard step after the Large Body's `ALWAYS` mode replaced it: the Windows build failed (0.80.0 was never built or installed). The two are one mode (`HELD = ALWAYS`, `u16::MAX`); the step uses `GuardRow::live`, which is on every frame for such a row.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 148: The KH1 Neoshadow (End of the World's group rules, sink / rise, its dives), version 0.80.0 (2026-11-01)

- Ask (owner, 2026-11-02): "Now let's bring in the remaining KH1 adjacent heartless" (M11).
- Neoshadow (KH1 `xa_ex_2420` / `ex_2420.bd`, only End of the World room 29 ↔ Re:CoM `CEnemyNeoshadow` id 27): every number from the player's files (Checked (files), the asset test reads all 207). The battle's Neoshadows share KH1's room block (`sys 138` → `0x1402BC620`; zeroed at room set-up `0x140287C60`; Checked (decomp)); their count picks the play: one goes to a fixed spot and calls; two: one hides and ambushes from below, the other fights; three or four sink together and rise slashing from a formation; five or more leap up and dive at Sora one by one. The claw, the rising slash, sinking and circling underground (the Shadow's machinery), its hit rules, its death motion 72 (KH1's handler zeroes the killing hit's reaction, `0x4BD0` / `0x5304`).
- Its body row `0x80` in KH1's mode 3 (always on, `0x1402BBC10` / `0x1402B47A0`) while it leaps, dives or rises slashing, priority 100: Sora's swings bounce off (the shared guard's always-on mode, merged with the Large Body's).
- Re:CoM (Checked (exe)): vtable `0x140545128`, ctor `0x140190150` (`40 53 48 83 EC 20`, only caller `0x14018940A`), alloc `0x1401893F1` 0xCE0 → 0xD00; Re:CoM's states 0..3 its own.
- Assumptions (owner questions): room 29's fixed points relative to where Sora started; the grab from below misses (not built); the call brings no one; the dive on Re:CoM's id 1 power; a summoned one can't be hit while rising; a killing hit early in the sink kills it (KH1: 1 HP); the dive's row from frame 0 (KH1: 8); actor `+0x90` / `sys 124` unidentified.
- Asset step: ENEMY/0027 (ANN / ASD / SES), `kh1-neoshadow.json`.
- Checked: tests (20 000-frame fights with one and two, the host model with the real manifest), Windows build, 911 sites. Not checked: in game; no KH1 trace.

---

## Stage 147: The KH1 Large Body (front guard always on, no-reaction hits, rage charge), version 0.79.0 (2026-11-01)

- Ask (owner, 2026-11-02): "Now let's bring in the remaining KH1 adjacent heartless" (M11).
- Large Body (KH1 `xa_ex_2050` / `ex_2050.bd` ↔ Re:CoM `CEnemyLargebody` id 9), every number from the player's files (Checked (files)): appearance, wander; engages within 15 m with its first look (214); close: back step + punch; mid: belly slam; far: walk in, stomp, slam; after hits a punch counter (two counted: the stomp). Below 40 % HP once: the roar, then only charges (1.5 s wind-up, up to 3 s at 12 / frame, then a spot 3–5 m away).
- Reactions: ordinary hits damage without a flinch (Re:CoM's super armour, the framework built for the Pirate); finishers knock it back (KH1's `sys 79`) except in the stomp / slam / roar; any hit in the charge knocks it back.
- Front guard (Checked (decomp)): KH1's belly row always on except in the charge, 70° each side, priority 100: Sora's swings from the front take the strong recoil, no damage; magic and hits from behind land. Shared enemy guard's new always-on row mode.
- Stomp shockwave: Re:CoM's own shockwave record (its Large Body's), grown by KH1's numbers (25 / frame for 28 frames from frame 88) through Re:CoM's radius setter `0x14014E5C0`. Assumption: Re:CoM's circle stands in for KH1's ring (edge 1 m wide, ±0.5 m height): owner question.
- Re:CoM (Checked (exe)): vtable `0x140542AD0`, ctor `0x14018F090` (only caller `0x140189116`), alloc `0x1401890FD` 0xCE0 → 0xD00. Power: slam / stomp ↔ Re:CoM's two attacks (matcher); punch and charge on the belly bump's (Assumption).
- Framework: a script hit handler can end the action (the roar); the HP ratio and playing request reach the armour decision; the guard counter released with the runtime; a priority-100 row stays on through Re:CoM's physical attacks. Merged onto Stage 144's waiting-hit kind (the Large Body reads it).
- Asset step: ENEMY/0009 (ANN / ASD / SES), `kh1-largebody.json`.
- Checked: tests, Windows build, 582 + 290 sites. Not checked: in game; no KH1 trace.

---

## Stage 146: Wyvern: a hit during its dive takes no reaction (the script's 214 follows), version 0.78.3 (2026-11-01)

- Ask (owner, 2026-11-02, Pirate question 3): one version each.
- Wyvern (`ex_2320.bd` `0x2484`, Checked (files)): a non-clash hit during the dive 202 resets the turn rate, zeroes the reaction kind and asks for 214. Now on Re:CoM's super armour: Re:CoM no longer cuts the attack state; the attack state ends the next frame and 214 plays on in state 1 (host model). Guarding the dive still gives Re:CoM's flinch (no KH1 recoil motion).
- Element no-reaction (attribute 2 / 3 / 4) stays left out (owner). Assumption: attribute 8 / 9 exceptions not carried.
- Checked: tests, Windows build, host model. Not checked: in game.

---

## Stage 145: Air Pirate: a hit during its dive takes no reaction (the script's 211 follows), version 0.78.2 (2026-11-01)

- Ask (owner, 2026-11-02, Pirate question 3): one version each.
- Air Pirate (`ex_2210.bd` `0x1F0C`, Checked (files)): a non-clash hit during the dive 206 zeroes the reaction kind and asks for the hit state (211). Now on Re:CoM's super armour: no Re:CoM flinch frame; the behaviour's hit state ends the attack state and 211 plays in state 1 the next frame for its full length (host model). Before: one frame of Re:CoM's flinch state.
- Clash kinds 6 / 7 never armoured. Assumption: KH1's attribute 8 / 9 exceptions not carried.
- Checked: tests, Windows build, host model. Not checked: in game.

---

## Stage 144: Darkball: its no-reaction hits on Re:CoM's super armour (the attack goes on), version 0.78.1 (2026-11-01)

- Ask (owner, 2026-11-02, Pirate question 3): the fliers' "doesn't flinch" move onto the no-reaction armour, one version each.
- Framework: the armour's decision gets the waiting hit's KH1 kind, read without consuming from the enemy's hurt bodies in the order Re:CoM's gather `0x140154EB0` picks them (Checked (decomp): `0x14016F1C0` writes the hit's count / kind / owner on the body before the handler `0x14017F7E0`); armour only while a damaging hit waits; never over a Guard block of the same frame (Re:CoM's flinch for a blocked attack without a KH1 recoil stays). The Pirate ignores the kind (unchanged).
- Darkball (`ex_2280.bd` `0x1E20`, Checked (files)): from its `armor_limit`th light hit in a row the hit takes no reaction: KH1 zeroes the reaction kind (no `sys 42`; `0x1402B8E10` plays nothing for kind 0: no push, Checked (decomp)). Now Re:CoM's super armour keeps its dash / bite going; the counts and the bite mark as its handler. `keeps_reaction` stays the fallback (kind unreadable, same-frame Guard block).
- Assumption: Re:CoM's damage voice plays (KH1: none); KH1's attribute-10 / status exceptions not carried.
- Checked: tests, Windows build, host model. Not checked: in game.

---

## Stage 143: The KH1 Pirate; KH1's no-reaction hits on Re:CoM's own super armour, version 0.78.0 (2026-11-01)

- Ask (owner, 2026-11-02): "Now let's bring in the remaining KH1 adjacent heartless" (M11).
- Pirate (KH1 `xa_ex_2100` ↔ Re:CoM `CEnemyPirate`, id 21): KH1's AI, attacks, reactions and motions; Re:CoM's stats. Every
  number from the player's `ex_2100.bd` (Checked (files)). Wander and taunt; engage within 10 m at its height; close /
  mid / far rounds (side hop + long slash 214, jump slash 210, hops, run in + leap 218 → 213 or stuck blade 205–207, laugh
  212, spin 222 → 217 for 180 f, dizzy 219 after a hit); hit state: get-up slash 235 / 236 after its launch, block
  stance 208 or counter 216 within 3 m. Spin and stance guard from the front (shared enemy guard, rows 0x1CC / 0x46C).
- Re:CoM (Checked (exe + decomp)): vtable `0x140546458`, ctor `0x140190F30` (6 bytes, only caller `0x14018930E`), alloc
  `0x1401892F5` 0xCD0 → 0xCF0; spin / jump slash / get-ups in state 2 (id 0), slash / counter / slam in state 3 (id 1).
  Power: jump slash and get-ups take the spin's, counter and slam the slash's (Assumption: by kind; owner question).
- Framework: KH1's `sys 42` (`0x1402BBD80` → `0x1402B7E60`: push slide, no reaction) on Re:CoM's own super armour
  (`+0x260` bits 20 / 21, for one call of the hit handler `0x14017F7E0` through the existing wrapper); Stop / stun /
  freeze / Gravity / defeat stay Re:CoM's. No new hook; nothing per frame.
- Asset step: ENEMY/0021 (ANN / ASD / SES), `kh1-pirate.json`.
- Checked: tests, Windows build, sites pefile-checked. Not checked: in game; no KH1 Pirate trace.

---

## Stage 142: Search Ghost: KH1's drain rule on Re:CoM's HP / EXP / prizes; its burst's repel rows (the guard step's repel), version 0.77.0 (2026-11-01)

- Asks (owner, 2026-11-02): "KH1's rule on Re:CoM HP"; burst repel "Yes".
- KH1 (Checked (files + decomp)): the orb counts only if it ends 1–3.2 m from the Ghost; then a 568-frame hold
  (`0x2678`); the hover sets HP to max (`0x2ADE`) when the hold is off (at once if the orb started within 1 m); a hit
  during the hold (`0x28A6`, before the damage) ends it and gives `sys 194(EXP / 10 + 1)` (`0x1402BC9A0`) and 3 HP-ball
  prizes (`0x580`).
- Re:CoM: the heal through Re:CoM's enemy heal `0x140181CE0` (max `0x14017F5A0` − current); the EXP bonus KH1's formula on
  Re:CoM's EXP (`+0x2E4`, set for one call of Re:CoM's defeat EXP drop `0x1401810A0`, then put back); 3 of Re:CoM's HP
  prize (kind 0xD3) by the prize factory `0x1401A89C0` at its prize point.
- Burst: four type-1 rows (priority 40, recoil 205, cone cos 1.0) as one guard row on in 204's [36, 48) and [108, 118);
  KH1's priority rule: a Kingdom Key swing (30) is repelled (Sora kind 6; the Ghost hears a no-damage record). The
  guard step's "repel" outcome built for the Large Body / Defender. Finding (Checked (decomp + exe + files)): KH1's cone
  `0x1402B4640` with cos 1.0 passes only an attacker exactly ahead at the same height: in KH1 the burst effectively never
  repels; kept 1:1 with KH1's 3D test before each swing.
- Asset step re-run: `kh1-searchghost.json` (EN0014 files unchanged).
- Checked: tests (764), Windows build, 866 sites. Not checked: in game.

---

## Stage 141: Sora's swings knock the Bandit's thrown scimitar away, version 0.76.3 (2026-11-01)

- Ask (owner, 2026-11-02): "Yes" (swing at the blade, KH1).
- KH1 (Checked (decomp + child script)): two meeting attacks resolve both ways (`0x1402B4A20` → `0x1402B4BC0`, no front
  cone for these types); the blade's row `0x1CC` priority 0: Sora gets nothing, the blade the clash bit; on it
  `ex_2090_00.bd` turns back with a random tilt, fades, slides ×0.9/f for 60 frames without attacking, ends.
- Built (no new hook): in the existing swing pair test, the swing spheres against each flying blade's Re:CoM shape
  (Re:CoM's overlap tests); on contact KH1's repel (`Thrown::repel`), the bullet's attack off (`0x14014DE80`), its life
  raised past the slide, Re:CoM's guard spark at the blade. One flag test per live swing frame while no blade flies.
- Assumptions: own random stream; Re:CoM's spark for KH1's clash effect; the fade after the slide; the swing keeps
  hitting bodies after the blade (KH1 marks it clashed; not built).
- Checked: tests, Windows build, 623 + 250 sites. Not checked: in game.

---

## Stage 140: Re:CoM's "Guard" popup hidden over KH1 enemies that block, version 0.76.2 (2026-11-01)

- Ask (owner, 2026-11-02): "Hide it".
- Re:CoM's enemy hit handler's guarded branch calls the popup `0x14020C9A0` with kind 3 at `0x14017FCE7` (between the
  guard spark and the `+0x3D8` guard event; Checked (decomp + exe)). That call is retargeted at install through a stub
  to `guard_popup`: kind 3 skipped for our KH1 enemies, else Re:CoM's popup. Surrounding bytes checked, no branch
  inside (exe scan). Nothing per frame.
- Checked: tests, Windows build, sites. Not checked: in game.

---

## Stage 139: Fix: steered KH1 attacks (Bandit leap / spin, fliers' attacks) restarting every other frame, version 0.76.1 (2026-11-01)

- Owner (2026-11-02, 0.73.0): "bandits dont spin when they do the move where they're supposed to spin towards you"; the log:
  leap 217 `never ended … released` again and again.
- Cause (Checked (log + code + Re:CoM decomp)): Stage 127's net released any playing attack whenever the AI was polled;
  steered attacks are polled from their own attack state's step, so each was released on its first step, the idle
  played, the AI asked again, Re:CoM re-entered state 3 (`0x140242F50`): the attack restarted every other frame. Every
  class with steered attacks since 0.69.0 (Air Soldier, Darkball, Air Pirate, Wyvern, Bandit). Also the Bandit's leap
  asked for 217 twice in a frame (both saw the host's 216).
- Fix: the host's rules as pure functions (`kh1_core::enemy::host`): the net fires only when Re:CoM's state isn't the
  attack's own; the Bandit's `attack_keep` counts this frame's requests (KH1 sets the motion at once, `0x14029F7C0`).
  Tests: a host model of Re:CoM's states; Bandit leap / spin unbroken; a 12-seed fight never releases; every class
  through the model with its real manifest.
- Checked: tests, Windows build, sites. Not checked: in game.

---

## Stage 138: M11: the KH1 Search Ghost (its sight cone on Re:CoM's own bones), version 0.76.0 (2026-11-01)

- M11 (owner, 2026-11-02): a flier.
- Framework: `enemy::Sight` / `Behaviour::set_sight` (a bone's position and axis + Sora's lock point; the bone resolved
  once at the ctor); `Behaviour::side_frame` (KH1's child objects run while the parent waits); `Cmd::Note` (log-only);
  child-script constants (`.mdls` part 3) in the asset step; the quiet safety net 600 → 900 frames (the Ghost's idle
  search is silent up to 600).
- Search Ghost (`xa_ex_2230` ↔ `CEnemySearchghost` id 14; 121 + 2 constants; Checked (files + decomp)): appearance,
  notice (206); above half HP: in to 4.5 m, burst (204, strikes 36–48 / 108–118), off 5–8 m and watch, warp if within
  2 m; below half: lunge (200), drain (203), hover, roam with three warps, search, idle; the cone: Re:CoM's `l_eye`
  (bone 32) to Sora's `chest1` (`player+0x3A8`), 3 m / 35°; hits: below half 50 % invisible lunge, above half 50 % warp
  5–6 m else burst + hover; warps unhittable at once, hidden after 100 frames, shown 80 after arriving, hittable 80
  later. Vtable `0x14054C408`, ctor `0x1401923F0`, alloc `0x1401891CF` 0xCE0 → 0xD00. Power: drain → id 1 (`e014_051`
  edits KH1's 203), burst → id 0 (Assumption); guarding either plays KH1's recoil 205.
- Not built (owner questions): the drain's heal / EXP bonus; the burst's repel rows. Looks wait for M13.
- Asset step: `ENEMY/0014/EN0014.ANN`, `.ASD` (2 + 6), `.SES`, `kh1-searchghost.json`.
- Checked: tests (760), Windows build, 612 + 250 sites. Not checked: in game, KH1 trace.

---

## Stage 137: Fix: crash when KH1 enemies were rebuilt mid-battle (runtime lookup), version 0.75.2 (2026-11-01)

- Owner (2026-11-02, on 0.73.0): "game crashed". Divide by zero at exe `0x140197B60` (`CHeartless`'s state-1 walk step
  `0x140197AD0`, dividing by `+0xCA4`, which Re:CoM's think sets; our copy replaces the think) called from
  `win::kh1_enemy::think_step` (`kh1-recom.dll+0xb8d11`), as four Air Soldiers entered. Checked (decomp + DLL symbols).
- Cause (Checked (code)): the ctor `retain`ed out an older runtime at the same address (an enemy rebuilt in the same
  battle), shifting every later runtime; the tails' stored indices went stale, the lookup failed, and the fallback
  ran Re:CoM's own state 1.
- Fix: a same-address rebuild replaces its runtime in place; the lookup falls back to the address; a failed lookup
  for one of ours is logged once and never runs Re:CoM's state 1 (the frame is skipped).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 136: M11: the Bandit's thrown scimitar as a Re:CoM homing bullet, version 0.75.1 (2026-11-01)

- Ask (owner, 2026-11-02): "Re:CoM bullet + Re:CoM effect".
- KH1 (Checked (files + decomp)): it leaves at frame 90 of the throw, flies at 10/f, slows 0.5/f after frame 120 and
  comes back, turns up to 3°/f toward Sora, lives at most 300 frames.
- Built (Checked (exe)): a Re:CoM homing bullet made the way Re:CoM's Aquatank makes its shots; KH1's flight steps
  it each frame, then Re:CoM's own step (collides, hits Sora, ends it); look: Re:CoM's Aero level-1 shot set
  `0x1405A2030` (`airo_all` 5); damage: Re:CoM's power for the Bandit's attack id 1 (whirl; Assumption); Sora's
  Guard turns it back (decision 28). No new hooks.
- Not built: swinging at the blade, KH1's slide after a repel, walls (end it here), hiding the held blade.
- Asset step re-run: `kh1-bandit.json` gains the `blade_*` numbers.
- Checked: tests, Windows build, 782 sites. Not checked: in game.

---

## Stage 135: M11: a shared enemy guard (KH1's rule on Re:CoM's body guard); the Bandit's block stance and spin repel, version 0.75.0 (2026-11-01)

- Ask (owner, 2026-11-02): "Shared enemy guard first" for the Bandit's block stance and spin repel (then Large Body,
  Defender).
- KH1 (Checked (decomp + exe)): a guard row (type 2, table `0x140528CB8`) never hits a body and only meets Sora's
  swings (magic never blocked); front half only (`0x1402B4640`), while its request plays with its hit window on; two
  meeting attacks give the lower-priority side kind 6, or 7 when 40+ under (`0x1402B4BC0`); Sora: kind 6 recoil 0x6E
  + push, Counterattack from frame 12; kind 7 0x6F; in the air 0x70; the enemy plays its row's recoil and its script
  hears the clash.
- Re:CoM (Checked (decomp + exe)): its hit apply `0x14016F1C0` blocks a body with its guard bit (`+0x9C` bit 1, mask
  `+0x8C`, half angle `+0xB0`): set on the enemy's hurt bodies while the row is live (Re:CoM's values put back), KH1's
  type rule around each swing, the enemy's side from our vtable slot `+0x3D8`. Bandit: block stance → both kind 6
  (Sora recoils, the Bandit 232, back hop + throw); spin → Sora's strong recoil, the Bandit 213.
- No new hooks. Large Body / Defender shields (a turned cone) need more research.
- Asset step re-run: `kh1-bandit.json` gains `guards` (EN0011 files byte-identical).
- Checked: tests, Windows build, sites. Not checked: in game.

---

## Stage 134: M11: the KH1 Wyvern; motions from a start frame, KH1's loop rule, version 0.74.0 (2026-11-01)

- M11 (owner, 2026-11-02): a flier.
- Framework: `Cmd::MotionAt` (KH1's `sys 13` start frame through Re:CoM's motion play's start time, its third argument;
  Checked (decomp + disasm)); asset step `enemy::LOOPS` (clips KH1 loops though their last pose differs). Other
  enemies' outputs unchanged.
- Wyvern (`xa_ex_2320` ↔ `CEnemyWyvern` id 24; `ex_2320.bd`, 123 constants, 49 request words): appearance, notice
  (215) within 15 m; far: 70 % chase (207 / 208, banks 209 / 210), bite (200), fly past and up, come down (212, idle
  from frame 16), then charge or dive; 30 % charge (204, 40-frame rush 205 at 40/f, 206); close: roar (213) then dive
  (chase to 4 m above, 201, drop 202, land 203), every third close round the far attack; after 3 rounds rises and
  circles; a hit in the dive doesn't flinch (214); 5 counted hits within 3 m roar + dive, 3–10 m roar + charge + circle;
  Olympus rule (100 m search) in Re:CoM's Olympus. All guards: Re:CoM's flinch (recoil 110 missing). Vtable
  `0x140555DC8`, ctor `0x140195ED0`, alloc `0x140189373` 0xCF0 → 0xD10. Power: bite → id 0, charge → id 1 (Checked:
  Re:CoM's clips edit KH1's), dive → id 1 (Assumption).
- Asset step: `ENEMY/0024/EN0024.ANN` (77 kept, KH1 clips 100–127), `.ASD` (3 + 6), `.SES`, `kh1-wyvern.json`.
- Checked: tests, Windows build, 607 + 230 sites. Not checked: in game, KH1 trace.

---

## Stage 133: M11: the KH1 Air Pirate, version 0.73.0 (2026-11-01)

- M11 (owner, 2026-11-02): a flier.
- Framework: `Behaviour::set_target_grounded` (KH1's target state `+0x70`; fed from Sora's mover floor contact);
  the asset converter accepts a bone 1 named `m_ex…_sk` (Re:CoM's Air Pirate skeleton).
- Air Pirate (`xa_ex_2210` ↔ `CEnemyAirpirate` id 22; 126 constants, 49 request words; Checked (files + decomp)):
  appearance, rises to 170 above Sora and circles 4–8 s, stops (203), engages; far: Sora below → dive (205–207) then
  swoop; level → wind-up / take-off, banking flight, swoop or slash; above → climb and slash; 2–5 m: to 1 m above,
  swoop, back off; < 2 m: back off and climb, or swoop and fly off 5–7 m; after 10 rounds circles again; a hit in the
  dive doesn't flinch (211); 1–4 hits counter, 5–9 back off, 10+ circle. Swoop 200 (steered, frames 41–48), slash 212
  (frames 39.3–51); guarding the swoop: KH1's recoil 213, the slash: Re:CoM's flinch (no 110). Vtable `0x1405309E8`,
  ctor `0x14018A0B0`, alloc `0x14018931F` 0xCD0 → 0xCF0. Power: slash → id 0 (`e022_050` edits KH1's 212), swoop →
  id 1 (Assumption).
- Asset step: `ENEMY/0022/EN0022.ANN` (74 kept, KH1 clips 100–124), `.ASD` (2 + 2), `.SES`, `kh1-airpirate.json`.
- Assumptions: the two world-height tests as heights above the battle floor (owner question); others in the doc.
- Checked: tests, Windows build, 603 + 194 sites. Not checked: in game, KH1 trace.

---

## Stage 132: M11: the KH1 Bandit; movement state 7, KH1's hit counter, world-aware scripts; an attack-point fix, version 0.72.0 (2026-11-01)

- M11 (owner, 2026-11-02): a ground enemy.
- Framework (Checked (decomp)): movement state 7 (`sys 169` → `0x1402BB030`, the leap's flight: the flight step,
  airborne for reactions); KH1's per-enemy hit counter (record `+0x24` from `actor+0x3E8`, zeroed when no action
  plays, `0x1402BFE60` / `0x1402B7260`); a script's hit handler may rewrite the reaction kind (KH1 runs it before the
  reaction); KH1's world passed to scripts (decision 21's mapping). Asset step: attack points on bones the motion
  never poses now follow KH1's rule (the Bandit's blade 3.3 m → 1.30 m; the Air Soldier's spin row now ±7 cm around
  the toes: `EN0015.ASD` changes; Soldier / Powerwild byte-identical).
- Bandit (`xa_ex_2090` ↔ `CEnemyBandit` id 11; `ex_2090.bd`, 117 constants): appearance (69 ×2); wanders, engages
  within 15 m or when targeted; < 2 m back hop + spin or strafe + slash; 2–4 m strafe, pause, run-in slash or leap;
  far: run in, leap or throw; the leap dashes at 70 to 50 cm above Sora and carries on past; spin up to 4 rounds;
  counters after the 1st / 2nd hit (block stance 50 %, slash 20 %, back hop + leap 30 %); knock-backs become launches;
  Agrabah's rules in Re:CoM's Agrabah. Vtable `0x140534368`, ctor `0x14018AEB0`, alloc `0x140189151` 0xCD0 → 0xCF0.
  Power: slash → id 0, spin → id 1, leap → id 1 (Assumption).
- Not built yet (owner questions): the thrown blade (the throw motion plays, nothing flies); the block stance / spin
  repel. Asset step: `ENEMY/0011/EN0011.ANN`, `.ASD` (2 + 6), `.SES`, `kh1-bandit.json`.
- Checked: tests, Windows build, 609 + 176 sites. Not checked: in game, KH1 trace.

---

## Stage 131: M11: the KH1 Darkball (warp, model scale and hide through Re:CoM's own calls), version 0.71.0 (2026-11-01)

- M11 (owner, 2026-11-02): a flier.
- Framework (Checked (exe + decomp)): uniform model scale through Re:CoM's getter / setter (`0x1401A4A20` / `0x1401A7050`);
  hiding through Re:CoM's hide bit `enemy+0x154` bit 0 (its draw `0x1401A39B0`); KH1's warp (`sys 86`); the script's
  flight easing writes (stats `+0x14`); a hit handler may cancel a hit's reaction; a steering flier counts as active
  for the quiet watchdog. Asset step: `OWN_CLIPS` (KH1 requests on their own clips so Re:CoM's records stay on its
  clips), `BONE_POINTS` (a bone's position in a frame as a constant).
- Darkball (`xa_ex_2280` ↔ `CEnemyDarkball` id 23; `ex_2280.bd`, 151 constants): grows in 0.8 m up (unhittable until
  grown); wanders, engages within 10 m after notice (205); close: bite (204), warp, circle and 3 dashes (201–203);
  far: warp or circle + dashes; warp: shrinks, vanishes (unhittable), glides, reappears above Sora at twice the size
  (drop 200) or plainly after 4 tries; counters (bite close, dash far), warps away after 5 counted hits, 5th light hit
  in a row bites, from the 6th no flinch. Vtable `0x14053B7A8`, ctor `0x14018D5D0`, alloc `0x140189349` 0xCD0 → 0xCF0.
- Power: dash → Re:CoM id 0, bite → id 1 (exact copies), drop → id 1 (Assumption). Guarding the dash: KH1's recoil
  206; bite / drop: Re:CoM's flinch (no recoil 110 in the model).
- Asset step: `ENEMY/0023/EN0023.ANN` (Re:CoM's 86 kept), `.ASD` (2 + 3), `.SES`, `kh1-darkball.json`.
- Not ported: leash, party retargeting, world rules, KH1 hit attributes (no Gravity stagger), warp smoke / fades,
  no-push flag. Checked: tests, Windows build, 765 sites. Not checked: in game, KH1 trace.

---

## Stage 130: M11: the KH1 Powerwild; KH1's timed jump and scripted launch in the framework, version 0.70.0 (2026-11-01)

- M11 (owner, 2026-11-02): the ground family's first.
- Framework (Checked (KH1 decomp)): movement state 3, the timed jump (`sys 82` → `0x1402AF150`, step `0x1402AFAE0`; apex
  motion 6 frames before the top `0x1402AF630`, no landing motion); scripted launch (`sys 38` → `0x1402B79F0`): a hit
  handler can replace KH1's reaction (its random draw kept). Asset step: one `.ASD` record per hit window (a slot can
  open several times; Shadow / Soldier / Air Soldier outputs unchanged).
- Powerwild (`xa_ex_2030` ↔ `CEnemyPowerwild` id 7): first look; close: back hop / side hop + swipe; mid: sideways
  walk, pause / roar, shuffle into hop + swipe or swipe, maybe the pounce; far: run in; counters: flurry + back hop or
  side hop + pounce; above 40 % HP a launching / knock-back hit plays its own launch (base 270 / 280 in the air, hop
  80, speed 25) then the shake-off (224). All numbers from the player's `ex_2030.bd`. Vtable `0x140547798`, ctor
  `0x140191380`, alloc `0x1401890A9` (0xCD0). Power: swipe 212 → Re:CoM id 0, flurry 214 → id 1, pounce 222 → id 1
  (Assumption: nearest).
- Asset step: `ENEMY/0007/EN0007.ANN`, `.ASD` (Re:CoM's 3 + 9), `.SES`, `kh1-powerwild.json`.
- Checked: tests, Windows build, 757 sites. Not checked: in game, KH1 trace.

---

## Stage 129: Air Soldier: the quiet watchdog can restart it (Behaviour::reset), version 0.69.1 (2026-11-01)

- The Air Soldier's `Behaviour::reset` (Stage 127's quiet watchdog): back to engaged, still flying, no velocity, its
  appearance counted done. Test added.
- Research (no DLL change): KH1's second AI syscall table `0x1405752D0` (`sys1.0` `0x140394D80` a no-op, `sys1.3`
  `0x140394DE0` removes a child); `tools/kh1-ai-script.py` reads both tables. Child objects (`sys 94`, part 3 of the
  `.mdls`): Search Ghost's is a 35° / 3 m target cone with an effect (no projectile); Gargoyle's moves on its own and
  makes attack rows (owner question). Checked (exe + decomp + files).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 128: M11: the KH1 Air Soldier, with KH1's flight in the shared enemy framework, version 0.69.0 (2026-11-01)

- M11 (owner, 2026-11-02, "bring in the remaining KH1 adjacent heartless"): the flying family's first.
- Framework: KH1's flight (movement state 4, step `0x1402CC000`: horizontal eases at 0.1/frame, vertical at once, no
  gravity; fliers' finishers knock back); steered attacks (`Cmd::Attack`: the AI keeps steering while the attack
  plays in Re:CoM's attack state); behaviour inputs (floor height, Sora targeting it `player+0x84C`, the hit's KH1
  kind, an attack landing: result 1 of `0x14016F1C0`); per-class rows (`parse` / `make`, `ctor_ptr`).
- Air Soldier (`xa_ex_2160` ↔ `CEnemyAirsoldier` id 15; Checked (decomp + files)): appearance, air wander,
  engages within 15 m or when targeted; < 3 m strafe + slash or spinning dash; 3–6 m swoop kick / swoop; far: hover,
  90-frame wind-up, charge; hits fill an anger meter (+25 / +10 / +5), full and far: power-up slash. 127 constants
  read from the player's script. Vtable `0x140531CC8`, ctor `0x14018A4E0` (6 bytes), alloc `0x1401891F9` 0xCD0 → 0xCF0.
- Asset step: `ENEMY/0015/EN0015.ANN` (+ KH1 clips from 100), `.ASD` (5 records), `.SES`, `kh1-airsoldier.json`.
- Merge fix: a steered attack's end is our own leave to state 1 (Stage 127's take-over doesn't drop its follow-up).
- Assumptions: flat battle floors (Sora's last standing height); power by limb (hands / charge on Re:CoM's dash punch,
  feet on its kick); the chest body-block row left out. Not ported: leash, group-leader messages, head look, fade.
- Checked: tests, Windows build, 737 sites. Not checked: in game, KH1 trace.

---

## Stage 127: Fix: KH1 enemies freezing after a Re:CoM state (Stun Impact's stun, Stop, freeze…), version 0.68.3 (2026-11-01)

- Owner (2026-11-02): "i used stun impact on 2 soliders (check trace) and once they got out of the stun they are doing
  the same issue of just standing still".
- Cause (Checked (log + trace 20)): Re:CoM's stun cut the kick / spin kick; Re:CoM went back to state 1 with our runtime
  still holding the attack as playing; the AI isn't polled during an action and an attack only ends in its attack
  state: never. The watchdog left playing attacks to that state.
- Fix (all classes): our own returns to state 1 are marked; any other entry means a Re:CoM state ran (stun, Stop,
  freeze, Gravity's flatten, Re:CoM flinch, first-strike stun): the pending action / reaction / queue / clash are
  dropped, idle plays, the AI goes on (KH1: a status ends the current action; that the AI carries on: Assumption).
  The appearance's end calls Re:CoM's default state outside our lock. New nets (logged past the cap): an attack
  still playing after its state was left; a reaction over 600 frames; an AI silent 600 frames in state 1 restarts
  from engaged (`Behaviour::reset`; KH1's longest silent wait 180).
- Checked: tests, Windows build, 717 sites. Not checked: in game.

---

## Stage 126: KH1 enemies: Re:CoM's flinch for a guarded attack with no KH1 recoil; KH1's wander centre, version 0.68.2 (2026-11-01)

- Asks (owner, 2026-11-02, on 0.68.0's questions): guarded attacks whose KH1 recoil the model lacks play "Re:CoM's
  flinch"; the Shadow's wander "Fix it to KH1".
- Every KH1 enemy class: a Guard block of an attack with no recoil motion makes the next state 11 Re:CoM's own flinch
  (its begin / step) until the next state 1. KH1's wander picks its points around the enemy's current place
  (was its home for the Shadow).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 125: Fix: KH1 Soldiers and Shadows freezing; a watchdog for every KH1 enemy, version 0.68.1 (2026-11-01)

- Owner (2026-11-02, video): "this soldier just decides to not attack and stay in one position"; "it seems the shadows
  are doing the same thing".
- Cause 1 (Soldier; Checked (files + log + trace)): the asset step marks a clip looping when its last pose matches its
  first; kick 200 and air kick 204 were, so Re:CoM looped them and the motion clock (`0x1401A47C0`) wrapped before KH1's
  length: the action never ended. `ClipWatch`: the clock going back after half the motion counts as its end.
- Cause 2 (Shadow; Checked (log + code)): when the rise ends and the first attack is requested on the same frame, state
  0's step called Re:CoM's default state (`0x14017D6F0`), overwriting the attack state, which never began. State 0 no
  longer does while an attack is pending; a pending attack state is requested again if something replaced it.
- Watchdog (all classes): an attack state not begun 10 frames after the request, or an action playing 30 frames past
  its motion's length, is released as if its motion ended, with a log line past the cap. Tests: a 20000-frame fight
  per enemy (12 seeds) acts at least every 600 frames.
- Not a refactor regression (M10's paths compared). The Shadow popping up while sunk: cause not found; new logging
  (motions and Re:CoM state changes while unhittable).
- Checked: tests, Windows build, 717 sites. Not checked: in game.

---

## Stage 124: M11 step 1b: the KH1 Soldier on Re:CoM's enemy states, version 0.68.0 (2026-11-01)

- KH1 (Checked (decomp + files)): `xa_ex_2010` script: appearance, wander, engage within 1000; close (< 350) kick or
  double kick (> 280), sometimes back off + spin kick; mid (350–450) jumping kick / spin kick / run-in kick; far run in;
  arc strafe after each round; jump-kick at Sora above; counters when hit. 64 constants + 14 request words read
  from the player's script.
- Re:CoM: `CEnemySoldier` vtable `0x14054EA68`, ctor `0x140193510` hooked (6 bytes), alloc `0x140188FD7` 0xCD0 →
  0xCF0; copy runs states 0, 1, 2 (ground kicks), 3 (airborne attacks), 11, 12, 30 and the think. Re:CoM's HP,
  damage, drops, EXP.
- Asset step: `ENEMY/0002/EN0002.ANN` (Re:CoM's 75 slots kept, KH1 clips from 100), `.ASD` (10 records), `.SES`,
  `kh1-soldier.json`.
- Not ported: KH1's leash (would make a Re:CoM battle unwinnable); scripted-room rules.
- Assumptions (`docs/recom-m11-enemies.md`): power mapping (kick / spin kick); missing recoils stop the attack.
- Checked: tests, Windows build, 717 sites. Not checked: in game, KH1 trace.

---

## Stage 123: M11 step 1a: a shared KH1-enemy framework from the Shadow's code (Shadow unchanged), version 0.67.4 (2026-11-01)

- Ask (owner, 2026-11-02): "Now let's bring in the remaining KH1 adjacent heartless" (M11).
- Framework: `kh1-core` `enemy/` (KH1's enemy body physics, the shared reaction engine, the script runtime, the
  helper routines KH1's enemy scripts share byte for byte, request bit 23); `kh1-recom` `kh1_enemy` (one class table:
  vtable, alloc / tail, ctor and size sites, attack-state map, appearance, manifest; one ctor wrapper per class;
  the Shadow is class 0); `kh1-recom-assets` `enemy` (one Spec per enemy writes .ANN / .ASD / .SES / manifest);
  `kh1-trace` enemy rows gain `kind` / `spawn`; `tools/kh1-ai-script.py` (lists a KH1 AI script from the player's files).
- Shadow unchanged: its tests pass, its four asset outputs hash the same, same manifest, logs and trace token.
- Checked: tests, Windows build, sites. Not checked: in game.

---

## Stage 122: Revenge moves untouchable whenever a boss plays them; Hook's hole closed, version 0.67.3 (2026-11-01)

- Owner (2026-11-02): "you can still hit bosses during their revenge move (or if they use their revenge move naturally
  instead of it being triggered by revenge value)".
- Holes (Checked (decomp + code)): (1) the window opened only for a revenge-started state, so a boss's own use of the
  same attack was hittable: now it opens every time the revenge-move state begins (own choice logged); (2) Captain
  Hook's ctor `0x1403C6340` sets `+0x260 |= 0x2000000` permanently, so his handler always ended the attack and took the
  hit (`0x14017E1D0`): held off for the handler call during the window; (3) every frame counted as an ignored hit (the
  handler runs per frame, `0x140185630`): only real waiting hits counted.
- Checked not holes: all attack states 2..10 begin through Re:CoM's wrappers calling `+0x438` → `0x14017DA80`; our
  blade sweep and `strike_body` skip bodies off (`+0x10` = 0); all damage goes through `0x14017F7E0`.
- Checked: tests (442), Windows build, sites. Not checked: in game (no boss log yet).

---

## Stage 121: Shadow: unhittable from the sink's first frame to rise frame 50, as KH1, version 0.67.2 (2026-11-01)

- Owner (2026-11-02): "i can attack the shadow when its flat on the ground". Trace 18: three hits during sinks.
- KH1 (Checked (decomp + script)): actor flag 0x40 skips the hit test `0x1402B6AF0` (gate `0x140291920`) and the auto
  target `0x1402B0A00`; `ex_2020.bd` sets it on the sink's first frame (request 70, `0x1366..0x138C`), clears it at
  rise frame len(69) − 30 = 50 (`0x12D8..0x1314`), and sets it at init for a floor-spawned Shadow.
- Fix: `kh1_core::shadow` emits `Hittable` at those three points; the DLL uses Re:CoM's own hurt-body switch
  (`0x1401A2D70` off, `0x1401A2DB0` on). 0.67.1 switched them at sink frame 105 and rise frame 0. Lock-on unchanged
  (KH1's lock button can lock a sunk Shadow; Re:CoM's lockable flag stays).
- Trace: defeated Shadows drop out 31 frames after defeat; `h0` / `h1` token. Log cap 4000 → 20000 lines.
- The traces otherwise match KH1's script (sink / rise, leap, claw / swipe, flinches, launch, clash).
- Checked: tests, Windows build, 697 sites, the trace. Not checked: in game.

---

## Stage 120: Fix: crash at the start of every battle with a Shadow, version 0.67.1 (2026-11-01)

- Owner (2026-11-02): "the game crashes when getting into a battle with a shadow". Log: access violation at
  `kh1-recom.dll+0xa4e44` reading 0x54, called from the factory `0x140188fcb` right after the ctor.
- Cause: `win::shadow::ctor` read the facing from the mover `*(enemy+0xF0)+0x54`; Re:CoM's ctor hasn't made the
  mover yet (`+0xF0` = 0; the log's rax 0). Now facing 0 without a mover (state 0's begin takes the spawn's facing
  from the mover, as before, and skips it without one); the hit reader returns no hit without a mover.
- Checked: tests, Windows build, the crash address in the 0.67.0 DLL. Not checked: in game.

---

## Stage 119: M10: the KH1 Shadow (AI, attacks, reactions, motions) on Re:CoM's own enemy states, Re:CoM's stats, version 0.67.0 (2026-11-01)

- Ask (owner, 2026-11-02): "lets move on to doing phase 4, except, i only want the AI, attacks, reactions and motions
  from KH1 equivalent enemies, everything else (emphasis on stats) stay the same as they are in RECoM"; M10 choices in
  `design-philosophy-instructions.md` (native states, `enemy_ai` default kh1, battle only).
- KH1 (Checked (files + decomp)): the Shadow's AI script `ex_2020.bd` (`xa_ex_2020.mdls` part 5, VM `0x1402CC620`,
  pauses while an action motion plays `0x140297870`); attack rows in the script (claw / swipe / leap claw, 35 cm);
  reactions `0x1402B9230` / `0x1402B8B90` / `0x1402B77D0` / `0x1402B8530` / clash `0x1402B9140`.
- Built:
  - `kh1-core` `shadow.rs`: the Shadow's behaviour and reactions (numbers as data); the Dummy's kind-3 flinch fixed.
  - Asset step: `ENEMY/0001/EN0001.ANN` (KH1 clips at slot 100 + entry; Re:CoM's 76 kept), `.ASD` (4 new attack
    records with KH1's hit windows), `.SES` (Re:CoM's Shadow sounds on the new clips), `kh1-shadow.json` (the
    constants read from the player's `ex_2020.bd` / `btltbl.bin`).
  - DLL: ctor hook `0x140192770` and factory size `0x140188FAD` 0xCE0 → 0xD00; battle Shadows get a vtable copy whose
    states 0, 1, 2–4, 11, 12, 30 run the KH1 Shadow, think `+0x3D0` a no-op; Re:CoM's motion play, attack records,
    damage, defeat, prizes, EXP, stats; KH1's clash recoil on a Guard block. ini `enemy_ai = kh1 | recom` (kh1).
  - `kh1-trace`: an enemies CSV per recording (for the owner's KH1 Shadow trace).
- Not built: the first rise's alpha fade (no Re:CoM model alpha field found).
- Assumptions in `docs/recom-m10-shadow.md`. Checked: tests, Windows build, 693 sites. Not checked: in game, KH1 trace.

---

## Stage 118: Silent human-boss item-card page switch at Combat = KH1, version 0.66.1 (2026-11-01)

- Ask (owner, 2026-11-02, on 0.65.2's questions): "1 keep it silent" (first deal), "2 silence it" (page switch).
- The page switch's `call 0x1402AA360` (id 3) at `0x140167A35` in the revolver's page switch `0x140167910` (plays only
  when its third argument is 0) joins the side-checking stub (ninth call): enemy decks silent, Sora's own switch
  (`0x1401F8303`, side 1) passes. Its sound-playing callers: the human boss's `+0x468` `0x140199130` and Hades'
  `0x1403B10C0` (side 2). Menus play sound 3 from their own calls (untouched). Checked (decomp + disasm + exe).
- Checked: tests (434), Windows build, 670 sites. Not checked: in game.

---

## Stage 117: Re:CoM's whole dodge roll at Combat = KH1, version 0.66.0 (2026-11-01)

- Ask (owner, 2026-11-02): "make dodge roll work how it does in ReCoM, not KH1"; chose "The whole roll".
- Re:CoM's roll (Checked (decomp + exe + files)): action 6, start `0x1401F10C0` (untouchable bit `+0x418` 1, turn to the
  stick ≥ 0.3, row 0x16 `p001_070` with root motion), exe `0x140217310` (bit cleared at 600 units = 12 frames,
  moving cancel from 1400 = 28 frames, then stand / walk / run or fall); 3.5 m travel (Re:CoM's clip bakes KH1's).
- At `dodge_roll = recom` (new default): KH1's input rule decides when (Square on the ground; with Guard at run tilt;
  KH1's locks; a command replaces it), then Re:CoM's start runs through the existing hook's original and its exe
  runs untouched; the next KH1 frame takes Sora back fresh. Invincibility Re:CoM's own (decision 17's no-flinch frames
  unused). `kh1` keeps the KH1 roll; `on` = recom; `off` removes the roll (now also with a save loaded).
- No new site. Assumption: Re:CoM calls the root step on a clip's first update.
- Checked: tests (429), Windows build, 629 sites. Not checked: in game.

---

## Stage 116: Silent enemy card stock / reload / refill sounds at Combat = KH1, version 0.65.2 (2026-11-01)

- Ask (owner, 2026-11-02): "make it so that you cant hear enemies stocking cards or refilling their deck".
- Every one goes through Re:CoM's sound play `0x1402AA360(id, -1)` from deck code shared with Sora; the deck / stock
  folder's side is `+0x5C` (1 Sora, 2 enemy). Eight exact calls retargeted to a stub that skips enemy decks (returns
  0, as a play that plays nothing): reload charge 0x13 (`0x140167C01`), gauge step 0x14 (`0x140167BDA`), reload done
  0x15 (`0x140167C93`, `0x140167B01`), deck refill / first deal 0x1B (`0x14016B3E7`, `0x14016A091`), boss stock 0xF
  (`0x14016DCA4`), Marluxia 3's folder 0xF (`0x14016DE3F`). Switched at battle start with stage 2/3's switches;
  Combat = Re:CoM, Riku's story and minigames keep Re:CoM's bytes. Checked (decomp + disasm + exe).
- Not silenced (outside the ask): a human boss's item-card page switch (sound 3). Assumption: no reload voice.
- Checked: tests, Windows build, 652 sites. Not checked: in game.

---

## Stage 115: Revenge: KH2's frame-20 check, Hades untouchable only in his wind-up, Cloud's revenge = his charge, version 0.65.1 (2026-11-01)

- Asks (owner, 2026-11-02, on 0.65.0's questions): Hades "Only the wind-up"; check frame "KH2's frame 20"; Cloud's
  revenge "Homing charge".
- Check timing (bug fix; Checked (disasm)): `+0x68` is zeroed after each change by `0x140242F50`, so 0.65.0 decided on
  frame 1. Now: at frame 20 (Hades 30) in the flinch / blow-away step hooks; an earlier end decided in Re:CoM's
  state-change callback `0x1401A3B80` (the 12 boss classes' state-machine table slot `+0x28` → our wrapper, Re:CoM's
  callback first); once per reaction.
- Hades: his state-5 exe (vtable `+0x168`) wrapped; when `+0xC80` leaves 0 (wind-up `e107_054` → spin `e107_055`) the
  window closes and Re:CoM's `0x1401A2DB0(boss, -1)` turns his hurt bodies back on; the spin takes hits (no flinch).
- Cloud: revenge state 4 (begin `0x1403AE680`, exe `0x1403AF190`): 2 dashes ≥ 50 % HP, 4 below; each turns toward
  Sora ~25 frames then dashes straight at 0.1 m/f (6 m/s): sidestep, Guard (frames 10–36) or Re:CoM's roll avoid it.
- Checked: tests (422), Windows build, sites. Not checked: in game; Cloud's .EPD numbers.

---

## Stage 114: Revenge moves can't be damaged or interrupted, version 0.65.0 (2026-11-01)

- Ask (owner, 2026-11-02): "the humanoid bosses that have revenge value can't be damaged or interrupted during their
  retaliation move".
- Re:CoM's own rule: its hit handler `0x14017F7E0` drops hits while `+0x260` bit 0 is set without `0x2000000`, and
  `0x14017DA80` turns the attacker's hurt bodies off (Checked (decomp)). For the attack a revenge started only, our
  hurt-off wrapper lets Re:CoM's body-off run and every hit source (swings, spells, Stop, turned-back shots, Re:CoM's
  own) reaches the handler without the interrupt bit; the swing / spell record is used up; Guard's flinch on the boss
  skipped. Window ends at Re:CoM's attack-state end `0x14017DA90` (new hook, 8 bytes moved; no branch into them).
- Meter: nothing adds during the window (bodies off: hits never meet the boss, as KH2's add needs contact); keeps
  draining. Revenge tail 0x20 → 0x30 bytes.
- Windows: Vexen 5 s of needles; Hades' state 5 = wind-up + 4 / 8 / 12 s spinning (owner question); Cloud's chase has
  no time limit. Found, not changed: the revenge check fires on the flinch's first frame (`+0x68` zeroed after each
  change by `0x140242F50`), not frame 20 (owner question).
- Checked: tests, Windows build, sites. Not checked: in game; enemy motion lengths.

---

## Stage 113: Gimmick cards: the HP drop at 50 % only, version 0.64.1 (2026-11-01)

- Ask (owner, 2026-11-02): "only drop one at 50% and occasionally when guarding".
- `gimmick::HP_THRESHOLDS` = [50]: the five card-break bosses drop their card once per fight at 50 % of max HP,
  without a stagger (same conditions and spawn as Stage 112). The Guard trigger is unchanged: each block posts
  Re:CoM's card break and the boss's own drop roll decides (Guard Armor 30 %, others their .EPD chance: Assumption).
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 112: Gimmick cards: Guard blocks break the five card-break bosses; cards at 75 / 50 / 25 % HP, version 0.64.0 (2026-11-01)

- Ask (owner, 2026-11-02): "interrupting an attack with a finisher is very rare"; chose (a) Guard + (d) HP thresholds,
  for Guard Armor, Trickmaster, Jafar, Parasite Cage, Dragon Maleficent at Combat = KH1, Sora's story.
- (a) Our Guard's block (`win::guard::on_result`, in the pair-dispatch wrapper `0x14016EAD0`) of the boss's own
  attack or a Re:CoM shot it fired (`CBullet` / `CHomingBullet`, shooter `+0x238`) posts the card-break message 3
  (as Stage 110); once per blocked attack (collision + serial `+0x70`); not in / entering state 13, dead or pending.
  Block detection no longer needs the guard-reactions toggle (it gates only the reactions).
- (d) After each hit (KH1 enemy-hit wrapper on `0x14017F7E0`; `+0x274` and the max-HP getter `0x14017F5A0`): crossing
  75 / 50 / 25 % drops the boss's card without a stagger, one card per hit, once per threshold per fight, not on the
  killing hit, no chance roll; a boss's own condition (Trickmaster's table, Jafar not in state 15, Parasite Cage's card
  unused, Dragon's footing not out) makes it wait. Spawned as each boss's own drop call: `0x140163740` then the prize
  factory `0x1401A89C0(0x1401A8A80(), &desc, 0)` with that boss's descriptor. Checked (decomp + disasm + exe).
- Finisher-interrupt break kept. Assumptions: the bosses' own projectile classes aren't linked for (a); HP lost only
  through the hit handler.
- Checked: tests, Windows build, 600 sites. Not checked: in game.

---

## Stage 111: Summons: usable whenever Sora has the MP; Sora invincible through them; Cloud first, version 0.63.0 (2026-11-01)

- Asks (owner, 2026-11-02): "summons should be able to be used as long as you have MP, also, sora should have iframes
  through all summon animations"; "cloud should also be at the top of the summons list".
- KH1's once-per-battle rule dropped: usable in a battle room, on the ground, with whole MP ≥ cost, no summon out (the
  row stays locked while one is out: a second sleight can't start during action 0x16).
- Invincible: the existing damage-check wrapper (`0x1401E6DE0`) sets Re:CoM's protection bit `player+0x418 & 1`
  for the call while a summon is out (action 0x16 from our summon, battle room, same player), so Re:CoM's own early
  exit drops the hit (no damage, no reaction), as Sonic Blade and the roll; checked every frame by the Battle exe
  (`0x1401F9840`); ends with Re:CoM's end action `0x1401F6660`, the battle or a room change. Checked (decomp).
- Cloud first in the list (`summons::list_order`), the others in learn order; KH1's placement kept.
- No new hook, call or site. Assumptions: every hit on Sora goes through `0x1401E6DE0`; a card break of the sleight
  (not a hit) can still end it.
- Checked: tests, Windows build, 577 sites. Not checked: in game.

---

## Stage 110: Gimmick cards: a finisher that interrupts a card-break boss gives Re:CoM's card break, version 0.62.1 (2026-11-01)

- Ask (owner, 2026-11-02, option c): when a KH1 combo finisher interrupts the attack of Guard Armor, Trickmaster,
  Jafar, Parasite Cage or Dragon Maleficent, the boss gets Re:CoM's own card-break stagger, which rolls its drop.
- Built (no new hook): after Re:CoM's hit handler, for those five classes (vtables RTTI-checked), a kind-5 hit that
  ended the attack on a surviving boss posts Re:CoM's card-break message as its resolver `0x14015EAC0` does:
  `0x1401D2770(CBTL+0x58, 0, CBTL+0x20, boss+0x20, 3, none)`; the base handler `0x140181620` case 3 (`0x140181739`)
  ends the attack and sets state 13; each boss's own state-13 begin plays the stagger and rolls the drop (Guard
  Armor 30 %, Parasite Cage 30 % once, the others their .EPD chance: Assumption). Checked (decomp + disasm + exe).
- Assumptions: Re:CoM's message manager runs in KH1 battles (as Stage 108's message 6); how often vs Re:CoM.
- Checked: tests, Windows build, 567 sites. Not checked: in game.

---

## Stage 109: Summons: KH1's Summon list issuing Re:CoM's LV3 summon sleights (Cloud: Omnislash), version 0.62.0 (2026-11-01)

- Ask (owner, 2026-11-01): "Lets get summons working. I want the summon section on the command menu to work like KH1,
  but each summon should do their lv 3 recom summon equivalent, cloud included"; chose MP from Re:CoM's CP (2 / 3),
  KH1's party rule skipped until the party, Re:CoM's damage at both settings, KH1's once-per-battle kept.
- KH1 (Checked (decomp + files)): Summon = the Magic list's last entry (`0x1402944C0`), row 3 opens the list
  (`0x1402937F0`), place `0x14027B870` (x 27; one entry y 326, more 348 − 22n), learn order `save+0x7D0`, greying
  `0x140288F30` (MP, battle mode, ground, once per battle, party), names Word.bin 286–291.
- Re:CoM (Checked (decomp + exe + files)): LV3 = the same summon card ×3 (`0x1401669F0`): Simba Proud Roar LV3 0x35,
  Genie Showtime LV3 0x3E, Bambi Paradise LV3 0x39, Dumbo Splash LV3 0x37, Tinker Bell Twinkle LV3 0x40, Mushu Flare
  Breath LV3 0x3C, Cloud Omnislash 0x42. Started natively: `player+0x82C` = sleight, Sora's vtable `+0x130`
  (`0x1402247A0`); Re:CoM loads the pack (`0x140211790`), spawns the `CSummon*`, hits, ends (`0x1401F6660`).
- Built at Combat = KH1, Sora's story, battle rooms: KH1's Summon row / list / greying / sounds; MP 2 (Simba, Genie,
  Bambi, Dumbo, Tinker Bell) / 3 (Mushu, Cloud), paid once Re:CoM's action is 0x16; row locked while one is out, no
  Dismiss; each summon learns with its first Re:CoM summon card (`0x14015C7F0` wrapper); learn order in a new save
  block `save+0x26AE..=0x26B6` (older saves: KH1 index order, Cloud last); preload on cursor rest; party rule in
  `summons::party_ready()` (always true until the party). Cloud: command 0x35, word 0x8029 "Cloud" (English).
- No new hook or patch; call sites checked at start.
- Assumptions: the start from our wrapper equals Re:CoM's own (no card record needed); the once-per-battle reset at
  the next battle; Cloud's name in every language; `UK_uibin_sum.bin`'s layout (the asset step reports it).
- Asset step re-run: summon words and the Summon panels.
- Checked: tests, Windows build, 539 sites. Not checked: in game.

---

## Stage 108: Gimmick cards: Re:CoM's drops, a Triangle "Stagger" prompt that plays the card, version 0.61.0 (2026-11-01)

- Ask (owner, 2026-11-01): "certain bosses drop "gimmick cards" in their battles. I want these to drop, and when sora
  picks them up, i want there to be a triangle prompt that says "Stagger," and pressing triangle will activate the card".
- Re:CoM (Checked (decomp + exe)): kinds 0x3A–0x41 (type 6), one held at a time, never stocked; played as action 0x19,
  the boss gets message 6 at frame 50 and runs its own reaction. Drops: Oogie (dice), Ursula (tentacle), Darkside
  (summoned Heartless gone), Jafar (Blizzard on its guard; Assumption) drop at KH1; Guard Armor, Trickmaster, Jafar,
  Parasite Cage, Dragon Maleficent drop on a card break (none at KH1: owner question).
- Built at Combat = KH1, Sora's story: Re:CoM's drops (log-only pass-through on the prize factory `0x1401A89C0`);
  pick-up keeps Re:CoM's one-at-a-time count (CBTL `+0x1D0`) with the deck add skipped for the call; the context tab
  offers "Stagger" (command 0xF0, word 901, English everywhere) when Re:CoM's card play would accept it, priority
  object command > Stagger > Sonic Blade; Triangle makes Re:CoM's play writes (`0x140161D60`, `0x14015D5B0(-1)`,
  `0x1401F5580` message 1, `+0x428` bit 9 → `+0x424 |= 0x40`), Re:CoM's action does the rest; forgotten at
  battle end. `docs/recom-gimmick-cards.md`.
- Assumptions: the .EPD drop chances; each reaction's look; nothing needs the card in the deck at KH1.
- Asset step re-run: the word in KH1's font.
- Checked: tests, Windows build, 553 sites. Not checked: in game.

---

## Stage 107: Damage = Re:CoM: Aero level 1 wind; Gravity Break and Zantetsuken skip Re:CoM's boss falloff, version 0.60.1 (2026-11-01)

- Asks (owner, 2026-11-01, on 0.59.0's questions): 1 (b) Aero level 1 gets a wind with the card's value; 3 (b) "skip
  Re:CoM's reduction for these two"; 2 (a), 4 (a) unchanged.
- Aero level 1 (Damage = Re:CoM, battle rooms): card 0x20 (300 × attack / 100, attribute 1), a wind shaped as KH1's
  Aerora wind (btltbl `0x5CF8`: 80 cm × Sora's size, a hit every 30 frames through `0x14016F7D0`, KH1's flinch;
  using level 2's shape for level 1: Assumption), for the barrier's (18 + max MP) × 60 frames; Re:CoM's Aero card
  effect. Field and Damage = KH1: barrier only, as KH1.
- Correction: `0x14015D940` (named "boss rule" in 0.59.0's question) is Re:CoM's bonus against frozen enemies
  (`enemy+0x2B0`; Checked (decomp + exe + files)); Re:CoM's boss reduction is the falloff (`+0x288` / `+0x28C` /
  `+0x290`). Gravity Break and Zantetsuken now skip the falloff (`+0x290` = 1.0 for the handler call, the three
  fields put back; shared helper `win::revenge::Falloff`); frozen bonus, resistance, weak parts kept.
- No new sites or hooks.
- Checked: tests, Windows build, 529 sites. Not checked: in game.

---

## Stage 106: Boss revenge value (KH2's) for humanoid bosses at Combat = KH1, version 0.60.0 (2026-11-01)

- Ask (owner, 2026-11-01): "lets also have humanoid bosses have a revenge value system, similar to how they work
  in their fights in KH2 … borrow their revenge value"; chose story / AS limits, all humanoids, the boss's own Re:CoM
  attack nearest its KH2 revenge move, Re:CoM's boss falloff off for them.
- KH2 (Checked (decomp + exe + files), `docs/recom-boss-revenge.md`): hit adds ATKP `+0x2A` to enemy `+0xD48`;
  limit `+0xD4C` (default 100) set by AI syscall table 2 #76 `0x140431E30`; decays 1 per 60 Hz frame, paused in
  damage motions; checked by motion trigger 28 at frame 20 of the flinches (Hades 30); no reset.
- Limits: Axel 92, Larxene 100, Vexen 150, Marluxia (all forms; parts count on the body) 100, Hades 80 → 50 below
  30 % HP, Cloud 100, Hook / Riku / Riku Replica / hooded figure / Leon 100 (KH2's default).
- Re:CoM side: the 12 boss bodies' factory sizes (`mov ecx, imm32`) raised by 0x20 for the counter (all-or-nothing);
  hooks on the flinch step `0x140184100` and blow-away step `0x140182E50` (6 bytes each), returning at once for
  other enemies; lazy decay at each hit / check minus flinch frames; the check at KH2's frame (or at the flinch's
  end if shorter: Assumption that Re:CoM's flinches can be shorter). Revenge starts the boss's own attack state
  (Axel 4, Larxene 4, Vexen 6, Marluxia 2, Hades 5, Cloud 6 Omnislash, Hook 3, Riku 4) with P1's message-2 writes;
  hooded figure and Leon have no attack state (counter only, falloff kept). Falloff fields put back for the rest.
- Hit values: KH2 Sora's rows (swing 8, finisher 24, air 12 / 28, Fire 8, Blizzard 16, Thunder 32); 10 for hits
  without a KH2 twin (Assumption).
- ini `boss_revenge = on | off` (default on); Combat = KH1, Sora's story only.
- Assumptions: each 'nearest' attack state (chosen from its motions and code); Sora's dt as the clock.
- Checked: tests, Windows build, 526 sites. Not checked: in game.

---

## Stage 105: Damage = Re:CoM: Re:CoM's Gravity / Stop / Aero; KH1's % for Gravity Break and Zantetsuken, version 0.59.0 (2026-11-01)

- Asks (owner, 2026-11-01, on 0.58.1's questions): Damage ini only; Gravity and Stop "Re:CoM's numbers"; Aero
  "Re:CoM's Aero by level"; Gravity Break / Zantetsuken "KH1's % of Re:CoM HP".
- At Damage = Re:CoM (Checked (decomp + disasm + exe + files)):
  - Gravity: card 0x1E / Gravira 0x11 / Graviga 0x18, 20 / 40 / 70 % of current HP through Re:CoM's kind 0xD
    (`0x14017DC50`, gate `param+0x40`, `0x14017DD4C`), then its flatten state `+0xAC0` (`0x14017D626`, 9000-tick hold
    `0x140183CFB`); KH1's squash leaves a flattened enemy alone.
  - Stop: card 0x1F / Stopra 0x12 / Stopga 0x19, 9000 / 15000 / 21000 ticks through kind 8 (gate `param+0x2C`).
  - Aero's wind: card 0x20 / Aerora 0x13 / Aeroga 0x1A, 300 / 600 / 900 × attack / 100 per wind hit.
  - Gravity Break / Zantetsuken: KH1's rule (`0x1402BFA80`): int(current HP × power / 100 × resist[8]), at least the
    record's floor; power 40 + KH1 Strength / 250; on Re:CoM's current HP.
  - KH1's cast, area and order kept; the card interrupt not set when Re:CoM's gate refuses.
- Field: Gravity / Stop keep KH1's rules (no card table); Aero's wind takes Re:CoM's no-card AP as Fire.
- No new hooks; three check-only sites. Damage = KH1 unchanged.
- Assumption: the live tables match the session's SY0002.BIN (the DLL reads the live ones).
- Checked: tests, Windows build, 491 sites. Not checked: in game.

---

## Stage 104: Sonic Blade: Re:CoM's start tint, version 0.58.1 (2026-11-01)

- Ask (owner, 2026-11-01): "Add Re:CoM's tint" (as base Re:CoM's Sonic Blade).
- At our Sonic Blade's start, before the `limit1` effect: `0x1401F7D80(player, 0x1407C5D50, 20.0, 0)`, as Re:CoM's
  SONICRAVE start (`0x140218E05`; Checked (decomp + disasm)). It sets a flag Re:CoM's end action `0x1401F6660`
  (already our special's end) reads to fade back over 20.0.
- Checked: tests, Windows build, sites. Not checked: in game.

---

## Stage 103: Damage setting: Re:CoM damage per equipped Keyblade and Re:CoM enemy HP (default), version 0.58.0 (2026-11-01)

- Ask (owner, 2026-11-01): "depending on which keyblade sora has equipped, it does recom's damage for the
  corresponding keyblade? lets also have Recom's enemy HP"; chose: KH1's normal swings Re:CoM's normal column,
  KH1's finisher Re:CoM's finisher column; spells and specials Re:CoM's card / sleight damage.
- Re:CoM's formula (Checked (decomp + disasm), row pick `0x1401F7520`, card branch `0x1401F7ACC`): power = card
  `BattleCardList` (`0x14087A808`) power at the row's column + `+0x664` (0 at KH1: no enemy card 0x79), + half
  when `+0x420 & 0x4000000`; AP = attack stat `+0x43C` × power / 100; attribute card `+0xB`; then Re:CoM's
  enemy-card rewrite `0x1401E66A0`. Re:CoM's rows 1–7 columns 0, 0, 1, 2, 1, 0, 2 (Checked (exe)).
- New ini setting `damage = recom | kh1` (default recom; KH1 = decisions 12 / 21 as before). At recom:
  - swings: the equipped Keyblade card (`save+0x2674`) at column 0 for normal swings, 2 for KH1 kind-5 finishers
    (Blitz, Ripple Drive, Hurricane Blast too); other KH1 moves the column of the Re:CoM row they are built on
    (table in `docs/recom-kh1-attack.md` § Re:CoM damage setting); field swings keep Re:CoM's no-card AP.
  - enemy HP: Re:CoM's own (no KH1 HP write or max-HP override); KH1 records kept for level, Stop / Gravity
    resist, drops, lock radius.
  - Fire / Blizzard / Thunder: card 0x1A / 0x1B / 0x1C, then the -ra / -ga sleights 0xD..0xF / 0x14..0x16;
    Sonic Blade sleight 9: dashes row 0xF (col 0), finisher row 0x10 (col 2). Cure, Gravity, Stop, Aero unchanged
    (owner questions).
- No Config row yet (Re:CoM's Config slots are full): owner question.
- Checked: tests, Windows build, 488 sites. Assumptions: `player+0x390` exactly in battle rooms; the rewrite
  changes nothing at KH1 (logged). Not checked: in game.

---

## Stage 102: Sonic Blade: Re:CoM's voices and effects per dash; battle only, version 0.57.0 (2026-11-01)

- Asks (owner, 2026-11-01, on 0.56.1's questions): voices "however it works in base ReCoM"; per-dash effects
  "however it works in base ReCoM"; field use "Only in battle".
- Re:CoM's Sonic Blade (Checked (decomp + exe), `exeStockSkill` `0x140218950` SONICRAVE): start 0x8C, six dashes
  0x8D–0x92, finisher 0x93, 1:1 with KH1's wind-up, rows 0x1B–0x20, 0x21. Each part plays a motion of the
  sleight's pack 11009 (start 0, dashes 1, finisher 2); its voices / sounds / effects are that motion's events
  (the sub-pack play `0x1401A3580` points Sora's sleight event players `+0x110` / `+0x120` at it).
- Built: when a KH1 Sonic Blade motion starts, `win::specials::motion_events` makes the same event calls on
  the matching Re:CoM motion (`0x140179A30` / `0x14017A240` effects, `0x140238710` / `0x140239260` sounds);
  KH1's motion, timing, damage, travel unchanged. The asset step no longer writes our SES entries for them.
- Battle only: offer and start need Re:CoM's battle flag (`player+0x390`); the Field exe entry `0x1401FA648`
  and cave `0x1401FB7B2` are no longer patched (Re:CoM's bytes).
- Assumptions: Re:CoM's sleight motions as long as KH1's (events at Re:CoM's frames on KH1's clip); the
  sleight's resources loaded by the first press (logged if not); `+0x390` set exactly in the Battle exe.
- Checked: tests, Windows build, 476 sites (pefile). Not checked: in game; the sleight's event files.

---

## Stage 101: Stock items on the green item row, version 0.56.2 (2026-11-01)

- Ask (owner, 2026-11-01, screenshot of KH1 > Items > Stock, its rows red with the Keyblade symbol): "have all
  the items also be green and have the items symbol".
- Change: `items_page::row_look` gives every stock item row (`Label::Stocked`: the Stock list and the pick
  from it) the item slots' look: the asset step's green row with the item symbol (variant 7, group 0x161 or
  the language copy's), else Re:CoM's green row 0x131. Keyblade slot, Sora row, commands, Yes / No stay red.
- No new address or hook. `docs/recom-m8-items.md` § Stock items green too.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 100: Item-row symbol in every language's camp menu, version 0.56.1 (2026-11-01)

- Found on the owner's PC (asset step run, 0.56.0): `note: camp item row not built: FORM/0100/FO0100.SPR asset
  FR_camp.lad: camp:00 has 367 animation groups, Re:CoM Steam's has 353`. The entry's language copies have 14
  more groups than the base file (Checked: the run's log), as the MP bar's `<LANG>_Cockpit.lad` do.
- Change: the asset step gives each copy the new group after its own groups (base 353, the language copies
  367); the base file must still be Re:CoM Steam's. `kh1-camp-rows.json` format 2 lists them (`item_rows`).
  The DLL picks the loaded set's last group when it is one of them, before each list set-up (no new hook).
- Assumption: the extra groups come after Re:CoM's and 0x131 is the same green row in every copy (the step
  checks 0x131's icon per copy). `docs/recom-m8-items.md` § Language copies.
- Checked: tests, Windows build. Not checked: the owner's PC assets, in game.

---

## Stage 99: Skip the intro tutorial and the Leon battle at Combat = KH1, version 0.56.0 (2026-11-01)

- Ask (owner, 2026-11-01): "If a player has KH1 Combat enabled, have the player skip the leon training battle.
  Also the intro tutorial."; method (a) "Skip the battle room".
- Hook: Re:CoM's battle-entry request `0x140236690` (10 bytes `48 89 5C 24 08 57 48 83 EC 20`, no branch into
  them; Checked (exe)). Acts only at Combat = KH1, Sora's story, Traverse Town (`save+0x1878` = 1), for event
  room 2000 → battle 1100 (intro) or 2002 → 1101 (Leon). Everything else goes to Re:CoM unchanged.
- It does what each battle script's own exit does, in its order (Checked (files): the ABC scripts):
  - intro: clears its 13 session flags, HUD-hide 0, friends-met (0, 0x22) |= 1 then |= 2, room change
    `0x1402360F0(0, 0, 2001)`;
  - Leon: clears its 10 session flags, keep-music (1, 0x32) = 1, HUD-hide 0, `0x1402360F0(1, 0x140233AE0(), 2003)`.
  - Fallback: no floor for the room (`0x14087CBE2[kind]` < 0) or a refusal → Re:CoM's battle runs.
- Runs once per battle request, never per frame. Saves: only the friends-met bits, as after a played tutorial.
- Assumptions: a scene room may change straight to the next scene room (vanilla only does it from a battle
  room); tutorial Leon gives no EXP. `docs/recom-story-skips.md` § Built.
- Checked: tests, Windows build, site bytes (pefile). Not checked: in game.

---

## Stage 98: Green item symbol on the camp's item rows, version 0.55.0 (2026-11-01)

- Ask (owner, 2026-11-01): "for the item slots, the color is correct, but they have the green mickey symbol, but
  lets use the green item symbol"; the plan "Yes, add it".
- Found (Checked (decomp + files)): Re:CoM's item symbol is `camp.lad` sprite 166 (sprite group 83), drawn only by
  the Deck page's type tabs; no camp row draws it, and no call swaps one sprite inside a playing sequence.
- Built: the asset step (`camp_rows`) appends group 0x161 to `camp:00` in Re:CoM's `FORM/0100/FO0100.SPR` (and
  its PC `camp.lad` assets): a copy of green row 0x131 with its icon's sprite group 85 → 83; writes
  `raw/FORM/0100/FO0100.SPR` + `kh1-camp-rows.json`. The DLL gives our pages' rows an 8-entry copy of Re:CoM's
  table `0x1407C11F0` (+0x161) while the item row is loaded; the Stock row and item slots take it. No new hook.
- Fallback: without the manifest (or a `camp:00` without 354 groups) the item rows keep 0x131.
- Assumption: sprite 166 is green in the texture (`FO0100.RTM` not in the session).
- Checked: tests, Windows build, the session's base `FO0100.SPR` patch. Not checked: the PC assets, in game.

---

## Stage 97: Stop works on attacking enemies; its full duration, version 0.54.1 (2026-11-01)

- Ask (owner, 2026-11-01): "stop doesnt seem to work". Log `kh1-recom-stop-0530.log`: eight Stopga strikes
  reached Re:CoM's handler, none entered its stop state.
- Cause (Checked (decomp + disasm)): Re:CoM's enemy hit handler `0x14017F7E0` drops any hit on an enemy in an
  attack (`+0x260 & 1`, `0x14017F8CB`) unless the card-interrupt bit `0x2000000` is set; every other KH1 hit
  sets it (decision 14), Stop's never did (since 0.23.0). Not a regression from 0.50.0. That each of the
  eight strikes met an attacking enemy: Assumption (the old log didn't print it; the new one does).
- Also: KH1's duration was clipped to the i16 carrier (32767 ticks); Re:CoM's `+0x2D8` is i32
  (`0x14017DCC6`, begin copies it at `0x14017C7F4`). Checked (disasm).
- Fix (native, no new hook): Stop sets the interrupt bit for the handler call on attacking, non-immune
  enemies; when the handler entered the stop and the duration is longer than the carrier, the whole
  duration goes into `+0x2D8`. KH1's numbers unchanged. `docs/recom-m7-magic.md` § Stop on attacking enemies.
- Checked: tests, Windows build. Not checked: in game.

---

## Stage 96: M9 stage 9c.1: KH1's Sonic Blade; Guard and Dodge Roll in Sora's kit, version 0.54.0 (2026-11-01)

- Asks (owner, 2026-11-01): "just have guard and dodge roll be a part of sora's movement kit from the start,
  dont add them as abilities"; High Jump / Glide / Superglide equipped when Re:CoM gives them; Sonic Blade with
  Re:CoM's sleight effects, no scene tint for now.
- Kit: Guard and Dodge Roll always on at Combat = KH1 (no AP, not on the page); save block v4 (their AP back);
  High Jump / Glide / Superglide auto-equipped once on arrival, removable.
- KH1 (Checked (decomp + files)): offer `0x14028D0F0` (order, range 600, < 160 above, 2 MP), start
  `0x14028D800` (MP on the press frame), `0x14039EAD0` state 0x20 (invincible, root motion only), dashes rows
  0x1B–0x20 + finisher 0x21, follow-ups Rave / Blast on a fresh Triangle in the window; `xl_limit.dat`
  motions 0–2 (30 / 54 / 82 f; travel 5.62 m / 7.34 m).
- Built: asset step converts `xl_limit.dat` entries 0–2 into slots (230–232 with the owner's files) + SES
  entries + words 319 / 321 / 322. The special runs on Re:CoM's action 0x16: `initStockSkill`'s writes for
  case 9 without a card, `exeStockSkill` `0x140218950` hooked (11 bytes) to run the KH1 frame then Re:CoM's
  `procAttackCollision` `0x1401F5DC0`; Field exe action-0x16 entry `0x1401FA648` → 13-byte cave stub at
  `0x1401FB7B2`; Re:CoM's `limit1` start effect; invincible via the damage-check path. Offer in the
  context tab ("Sonic"), follow-ups in the Attack row. Root-motion safety net 0.5 → 1.0 m/f.
- Assumptions: dash 1 at frame 31; Re:CoM's lock target for KH1's; Re:CoM's sound 0xD1 for KH1's
  4003 / 4004; nobody reads the sleight id `+0x82C`. `docs/recom-m9-abilities.md` § 9c.1.
- Checked: tests, Windows build, site bytes. Not checked: in game, KH1 trace.

---

## Stage 95: M9 9b follow-ups: Guard and Dodge Roll from the start, Re:CoM's High Jump / Glide / Superglide, Combo Master 50, Rod max MP, blue rows, version 0.53.0 (2026-11-01)

- Asks (owner, 2026-11-01): Guard from the start; High Jump, Glide, Superglide where Re:CoM unlocks them;
  Dodge Roll equipped from the start; Combo Master at level 50; Rod max MP +1 at 30 / 66; Abilities rows
  blue; "in the customize menu, make these slots be blue with the magic symbol".
- Re:CoM (Checked (decomp + exe)): int variable (0, 0x24) bits 1 / 2 / 4 = High Jump / Glide / Superglide
  (copied by `0x1401F0C50` into `player+0x41C`; set by the event script `0x140145C50` cmds 0x2C / 0x41 and
  bit 4 by the reward pick `0x1401BD9D0` in Castle Oblivion). Which events: Assumption (KH wiki: Monstro,
  Neverland, Room of Rewards).
- Built: unlocks as asked (read at Attack presses and the Abilities page, not per frame); save layout v3
  (ini-given High Jump / Glide / Superglide / Combo Master return when their source is met); the five ini
  keys are now only a no-save fallback. Max MP +1 at Re:CoM levels 30 and 66 (Checked: btltbl Rod list,
  bonus kind 2). Abilities rows, Customize slot rows and picker spell rows: Re:CoM list entry 0x133 (blue,
  magic icon) via `0x140112F10`.
- Checked: 532 tests, Windows build, all site bytes. Not checked: in game.

---

## Stage 94: M9 stage 9b: KH1's ability equip screen with AP; green item rows, version 0.52.0 (2026-11-01)

- Asks (owner, 2026-11-01): equip screen with KH1's AP; new unlocks unequipped; Gravity Break at Re:CoM level
  30; Rod list. "lets make "stock" have the item symbol, and make the box green" / "the same for all the item
  slots". Owner's spell-effects decisions recorded (own toggle, Re:CoM's sounds, after the specials).
- KH1 (Checked (decomp + btltbl)): AP = record +9: 3 at start, +2 at 24 levels (`0x1402852C0`, `0x1402B5660`);
  costs btltbl 0x33D8 (row id − 1); equip refused over max AP, greyed rows (`0x140310650`, `0x140311C20`);
  shared abilities (High Jump, Glide, Superglide) free.
- Built: "Abilities" row in the KH1 camp submenu (Sora / Shared → learned abilities, equipped marked, greyed
  when unaffordable, confirm toggles, AP left / max + cost + used / max). Unlocks: Gravity Break at 30; Rod
  list: Guard 33, Counterattack 57, Air Combo Plus 24 / 45, Combo Plus 87 / 93 / 99; High Jump, Glide,
  Superglide, Dodge Roll, Combo Master by their ini keys (owner question). New unlocks unequipped; 9a saves
  migrated within AP. Each KH1 frame reads the equipped set. Items page: Stock and item slots use Re:CoM's
  list entry 0x131 (green box, icon sprite 278) through its own row setup `0x140112F10`.
- Assumption: English names; sprite 278 = the item icon. No new sites.
- Checked: 528 tests, Windows build, all site bytes. Not checked: in game.

---

## Stage 93: M9 stage 9a: KH1's combo-modifier abilities and the ability state, version 0.51.0 (2026-11-01)

- Ask (owner, 2026-11-01): "Finish everything up to M9"; abilities unlock with Re:CoM (its level-up
  sleights) and, without a Re:CoM source, at KH1's learn levels; equip screen with KH1's AP next.
- KH1 (Checked (decomp + files)): picker `0x1402A6EE0` / `0x1402A6C70` conditions for Stun Impact, Gravity
  Break, Zantetsuken, Blitz, Ripple Drive, Sliding Dash, Vortex, Aerial Sweep, Hurricane Blast; their
  motions, windows, travel and attack records; AP costs (btltbl 0x33D8); learn tables.
- Built: ability block at save+0x269F..0x26AD (unlocked, equipped; AP reserved); Re:CoM-sourced unlocks
  via Re:CoM's own sleight-learned `0x14023AB00` (called, 16 bytes checked; storage save+0x2840); others at
  KH1's Rod-list levels on Re:CoM's level (Ripple Drive 12, Hurricane Blast 18, Aerial Sweep 36, Vortex 84);
  unlocked = equipped until 9b. Equipped modifiers passed to KH1's pickers (only rows that exist). Six
  KH1-only clips on extra action rows; Sliding Dash / Stun Impact / Zantetsuken play Re:CoM's own copies
  (rows 0x45 / 0x44 / 0x46). Rows 0, 3–7, 9, 10, 15 via `kh1_row_bytes`. Stun Impact a 2.5 m sphere at
  `belt1` (Re:CoM's collision); Gravity Break / Zantetsuken damage on current HP; Ripple Drive max MP × 3;
  stun → Re:CoM's kind 7. SES entries for the six clips. Debug `kh1_all_abilities`.
  `docs/recom-m9-abilities.md`.
- Not 1:1 yet: stun length / immunity Re:CoM's; no KH1 effects (M13); Re:CoM's blow-away; side scans see
  only Re:CoM's target (M12). Gravity Break has no KH1 learn level: never unlocks yet (owner question).
- Checked: tests, Windows build, 446 site byte sets. Not checked: in game.

---

## Stage 92: Battle-end softlock: KH1 strikes take Re:CoM's contact point; no new KH1 command after the win, version 0.50.0 (2026-11-01)

- Ask (owner, 2026-11-01): "ending a battle with magic still softlocks the game" (log: the battle manager
  waits on 5 EXP prizes that never arrive).
- Cause (Checked (decomp + disasm); the prizes' non-finite positions: Assumption, the old log read the
  wrong field): the manager `0x1401871D0` waits for its children; an EXP prize (update `0x140231010`) is
  collected only within 2 m of Sora's `+0x4B0` (`0x140231318`), no timeout. Re:CoM's defeat `0x14017BBC0`
  puts an enemy's prizes at the killing hit's body contact `+0x50`, written only by Re:CoM's overlap tests
  (`0x1401777E0` / `0x140176FA0`, run by its collision pass `0x14016EAD0`). Our Thunder / Gravity / Stop
  strikes called the hit apply directly: a kill by them dropped prizes at a stale or garbage point (NaN →
  never arrives → softlock).
- Fix: `win::magic::strike_body` runs Re:CoM's own overlap test (its pair rule) before the apply, as the
  collision pass does. After Re:CoM's win (`player+0x428 & 4`, as `0x1401E9180` refuses card plays) no new
  KH1 Attack / Counter / spell / item starts. Battle-end log shows each prize's real position, flags,
  Sora's prize target; a non-finite defeat point is logged.
- Checked: tests, Windows build, site bytes (two new check-only sites in the collision pass). Not checked: in
  game.

---

## Stage 91: KH1 camp: one Items menu with Sora's Keyblade slot and item slots, version 0.49.0 (2026-11-01)

- Ask (owner, 2026-11-01): "I would like one menu inside of the "KH1" menu for "Items" in "items," i want a
  slot for Keyblade, and all the item slots for items. eventually if we add accessories as well we will have
  slots for them there but dont worry about that for now".
- Built: the KH1 submenu is Items, Customize (the Keyblades row goes). Items → Sora: a Keyblade slot first
  (its name; stats panel Strength / Magic / Reach; confirm opens the owned-Keyblade list on the page, equip
  as before; cancel back to the slot), then the item slots (8a.2's KH1 rules unchanged); Stock as before.
  Sora's list is a list of slot kinds (accessories later go between). No new sites.
- Assumption: the Keyblade list keeps our old picker's behaviour (KH1's weapon screen not read); its scroll
  bar is made and deleted while the page stays open.
- Checked: tests, Windows build, all site bytes. Not checked: in game.

---

## Stage 90: KH1 casts in the field: Keyblade shown, spell effects loaded; Customize rows show their buttons, version 0.48.0 (2026-11-01)

- Asks (owner, 2026-11-01): "when casting spells, soras keyblade isnt visible, and the VFX dont appear";
  "for the customize menu, instead of saying 'shortcut 1, shortcut 2 etc' it should be the corresponding
  button".
- Keyblade (Checked (decomp)): Re:CoM draws it in the field only for its swings / Guard (`0x1401F5D30`);
  its magic never does (battle only). A KH1 cast now calls Re:CoM's same draw at the command.
- Effects (Checked (decomp + exe)): spell effects come only with Sora's battle resources (PLAYER group
  1001, effect groups 2000–2060), loaded by the battle scene (`0x1401AEA20` → `0x140211960(0)`) and dropped
  on camp open (`0x140211560`) unless Re:CoM's flag (0, 0x60). At Combat = KH1: the field state's player
  load (`0x1401AF933`, 5 bytes) also runs `0x140211960(0)`; the two camp-open unloads (`0x1401A95BD`,
  `0x1401A99F9`) keep it; the Config switch to KH1 loads it. Re:CoM's own calls only.
- Customize: rows and help use Re:CoM's font button escapes (`F5 67` Triangle, `F5 66` Square, `F5 64`
  confirm, `F5 68` L1; `0x1403480C0` → `0x140345B40`, following the controller / bound key), names if the
  font lacks them.
- Not measured: the field room load time with group 1001.
- Checked: tests, Windows build, all site bytes. Not checked: in game.

---

## Stage 89: M8 stage 8a.4: KH1 item drops from enemies, version 0.47.0 (2026-11-01)

- Ask (owner, 2026-11-01): items dropped by enemies at KH1's rates.
- KH1 (Checked (decomp + exe + files)): each enemy's defeat routine passes a table of three (chance, item)
  pairs to syscall 122 (the path that also grants EXP, syscall 167); `0x1402ABC40` rolls one draw per
  pair, below chance × Lucky Strike. Every KH1 twin's table read from the player's files (asset step,
  `kh1-stats.json` format 6).
- Built: wrapper on Re:CoM's defeat card prize `0x140180D20` (9 bytes): after Re:CoM's prizes, the twin's
  KH1 table rolled; each battle item made by Re:CoM's prize factory `0x1401A89C0` as a new kind 0xD8,
  launched like EXP orbs, ending like them (collected, or ~10 s then fade). Re:CoM's look table copied
  with a 14th row (0xD6's effect), its two loads (`0x140231676`, `0x140230700`) pointed at the copy.
  Wrapper on the prize add `0x1401F4AB0` (10 bytes): kind 0xD8 → KH1's pickup rule, Re:CoM's sound.
  `docs/recom-m8-items.md` § 8a.4.
- Not KH1's: Re:CoM's launch and 10 s life (KH1: stays, ≤ 8); refused pickup loses the item; only battle
  items made; Lucky Strike 1 (no abilities yet); Re:CoM's random float.
- Checked: tests, Windows build, all site bytes against the exe. Not checked: in game.

---

## Stage 88: M8 stage 8a.3: KH1 items in the Moogle shop (a fifth Items tab), version 0.46.0 (2026-11-01)

- Ask (owner, 2026-11-01): items in the Moogle shop; a fifth "Items" tab, one item per purchase, 1 Moogle
  Point per KH1 munny.
- KH1 (Checked (files)): every KH1 item shop sells Potion 60, Hi-Potion 150, Ether 200.
- Built: wrapper on the pack screen update `0x1403A6930` (15 bytes checked): Left from tab 0 / Right from tab
  3 opens tab "Items": Re:CoM's packs released as its close state does, three made with Re:CoM's pack
  constructor (never sold out), pictures = Re:CoM's Potion / Hi-Potion / Ether card faces (`C2DCard`);
  input, sounds, help lines, cursors, payment (tally, trophy, setter, window) Re:CoM's; buy = KH1's pickup
  rule; refused when no room or too few points. Leaving rebuilds Re:CoM's tab. `docs/recom-m8-items.md`
  § 8a.3.
- Not yet: the tab's own art (needs `FORM/0109` dumped); the screen is unreachable when the card
  collection is full (Re:CoM's rule). Owner questions in the doc.
- Checked: 483 tests, Windows build, all 408 site byte sets. Not checked: in game.

---

## Stage 87: M8 stage 8a.2: KH1 Items page in the KH1 camp entry, version 0.45.0 (2026-11-01)

- Ask (owner, 2026-11-01): "Finish everything up to M9" (M8 items).
- KH1's Items screen (Checked (decomp): `0x14030D160`, `0x14030CB50`, `0x14030F330`, `0x14030FA10`, `0x14030D7B0`,
  `0x14030EEB0`, `0x14030F160`): owner (Sora / Stock) → item → Use / Move / Remove / Equip / Discard;
  repeats allowed; 99 cap; cursor to the first empty slot after Equip; stock sorted by the row's key.
- Built: "Items" row in the KH1 camp submenu on the Customize page type (Re:CoM's camp list, cursor,
  sounds), KH1's rules; Use heals via Re:CoM's `0x1401E59A0` then Re:CoM's stats copy `0x1401F0120`
  (callee, 16 bytes checked). `docs/recom-m8-items.md` § 8a.2.
- 8a.3 (Moogle shop) not built: Re:CoM's shop lists only card packs (2×2 pack grids, per-pack data);
  KH1 sells Potion 60 / Hi-Potion 150 / Ether 200 munny at every stage, with a quantity and a
  destination. Owner question (how to sell in the Moogle shop).
- Not KH1's: menu words ours (English), no target list, refusals on the help line.
- Checked: 474 tests, Windows build, all 376 site byte sets. Not checked: in game.

---

## Stage 86: M8 stage 8a.1: KH1 items: Items list, slots, stock, use, version 0.44.0 (2026-11-01)

- Ask (owner, 2026-11-01): "Finish everything up to M9"; items from the Moogle shop and drops, healing
  Re:CoM's item-card amounts (owner's choice; see below).
- KH1 (Checked (decomp + files)): btltbl item rows (0x1A58, 0x14 bytes); use = motion 0x3E (82 f, free from
  74), hand moment f34, heal f65 (task `0x1402C2880`); slot cleared on use, never refilled from stock;
  pickup fills an empty slot, else stock (≤ 99); new game 3 empty slots. A hit does not return the item
  (only death does; the brief's "hit returns it" was wrong).
- Built: Items list in the command menu (KH1's green panels from `exchange/UK_uibin_item.bin`, item
  names in KH1's menu font, six languages); a use runs KH1's frame on Re:CoM's item action 0xE
  (`exeItem` `0x1401ED6E0` hooked, 10 bytes; field exe's action-0xE table entry `0x1401FA628` pointed at
  a 13-byte stub in padding `0x1401FAEA2`); Re:CoM's own item clip (row 0x2D = KH1's motion baked);
  Re:CoM's heal `0x1401E59A0` and effect; MP via `win::mp::add_points`. Save layout v2 at
  save+0x2685..0x269E (slots, stock). Debug `kh1_test_items`. `docs/recom-m8-items.md`.
- Owner's "Re:CoM's item-card amounts": Re:CoM's item cards heal no HP (they reload cards; Checked
  (decomp + files)). Provisional: KH1 × 560/120 (Potion 140, Hi-Potion 280, Elixirs full); Ether +3 MP.
  Owner question.
- Not KH1's / not yet: Sora only (no party list), no hand effect at f34, always 3 slots.
- Checked: 462 tests, Windows build, all 375 site byte sets against the exe, HUD dump test. Not
  checked: in game.

---

## Stage 85: M7 stage 7.13: the KH1 camp entry with Keyblades and Customize, version 0.43.0 (2026-11-01)

- Ask (owner, 2026-11-01): "Finish everything up to M9" (M7 7.13: "KH1" camp entry + Customize, plan
  row 7.13).
- Built: our camp entry renamed "KH1", opening a submenu made the way Re:CoM's Deck submenu is (its
  own 7 entry buttons relabelled with the Deck's calls; cursor, glove, fades, breadcrumb, sounds
  Re:CoM's): Keyblades (unchanged page) and Customize (new): KH1's three shortcut slots, each with a
  picker of learned spells at their level + None, no duplicate check (KH1 `0x140302950`,
  `0x140302740`, `0x1402857C0`; Checked (decomp)). Writes our save slots (save+0x2681..0x2683); the L1
  list reads them at once. Only at Combat = KH1 in Sora's story. No new hook sites.
- Changed: the camp runs our code on `camp+0xBC` (the confirmed top entry) instead of `+0xB8`; the
  main panel's layer 2 is hidden only while a page from the submenu is open (Re:CoM's Deck rule).
- Not KH1's: "Shortcut 1–3" / "None" words are ours (KH1's sysmsg not in the asset step); no glyphs in
  the rows (the help line names the button). `docs/recom-keyblade-equip.md` § Stage 7.13.
- Checked: tests (446), Windows build, all site bytes against the Steam exe. Not checked: in game.

---

## Stage 84: M7 stage 7.12: KH1's shortcut menu on L1, version 0.42.0 (2026-11-01)

- Ask (owner, 2026-11-01): "Finish everything up to M9"; shortcuts on L1 like KH1 (owner's choice). Owner's
  M7–M9 decisions recorded in `docs/design-philosophy-instructions.md`.
- KH1 (Checked (decomp + exe + files)): slots save +0x844..0x846, spells only; L1 held swaps the
  shortcut list in (`0x1402937F0`); Triangle / Square / confirm cast slot 1 / 2 / 3 through the menu's
  commit (`0x140293480`); greyed → error sound 4; busy → dropped; L1 suppresses Attack, roll / guard,
  context; panel `exchange/UK_uibin_cut.bin` at (margin + 16, 282), words 0xE6 / 0x73.
- Built: shortcut list in the command menu logic (`command_menu::step_shortcut`), slots in our save
  block (save+0x2681..0x2683, KH1's fill-on-learn, old saves filled once in KH1's order), casts via
  `magic::issue`, panel groups in `kh1hud:00` from KH1's `cut` layout and `Icon.tm2` glyphs; Square
  withheld from Re:CoM's request bits while the list is up; save / warp point Triangle calls
  (`0x1401C530E`, `0x1401CC31B`, 5 bytes each) retargeted so L1 + Triangle casts instead of saving.
- Not KH1's: Cure / Aero cast on Sora at once (no party list); a jump always takes the frame.
  Assumption: L1 = pad bit 4, the keyboard key is Re:CoM's L1 binding.
- Checked: tests, Windows build, all site bytes against the Steam exe, offline render of the panel.
  Not checked: in game.

---

## Stage 83: MP ring draws; one centre with the HP ring; ticks; room plate top left; save point MP; spell voices, version 0.41.0 (2026-11-01)

- Asks (owner, 2026-11-01): "the blue of the MP bar doesnt show"; "lets also make sure that all the MP
  segments are evenly spaced"; "the MP bar should also be the same thickness as the HP bar, and the
  bars should start from the same place"; "the room indicator should be in the top left of the
  screen"; "standing on a save point should refill MP"; "casting cure makes sora say \"thunder!\" etc".
- MP ring (Checked (decomp): `0x140247850` / `0x14024BB10`, a control's later layers are drawn relative
  to its first layer's first sprite): MP back, MP, charge, ticks now one layer a control (four
  controls). Ring rebuilt about Re:CoM's HP centre (444, 317), quarters r × r; HP control origin
  (444, 317) (was 0.25 up-left); new ring group TICKS (KH1's 90° / 180° ticks on the seams); 562
  groups. `docs/recom-kh1-hud.md` § 0.41.0.
- Room plate: top left (`hud_room_plate_top_left`, old key read too), moved by the gauge margin
  `+0xA0`. Assumption: right-anchored plate.
- Save point: wrapper on `0x1401C5180` (13 bytes checked): KH1's 1 MP every third frame and the charge
  emptied while Sora is on it (`+0x2C0`). Checked (decomp + exe): KH1 `0x1402ACE20` / `0x1402AD0B0`.
  `docs/recom-m7-mp.md`.
- Spell voices: KH1 cast motions pass Re:CoM's own SES variant (`0x14021FEF0(pl, 4)`, vtable `+0x110`
  checked), as `initMagic` does. Checked (decomp + files). Not KH1's: Re:CoM's voice frame (3 / 2,
  KH1 0) and one take a spell. `docs/recom-m7-magic.md` § Spell voices.
- Checked: tests, Windows build, all 349 site byte sets against the Steam exe, offline render of the
  ring. Not checked: in game.

---

## Stage 82: KH1 menu sounds: the SCD's sound table read; asset kept whole, version 0.40.1 (2026-11-01)

- Ask (owner, 2026-11-01): "notice the quality of the menu and the sounds playing … lets try to get
  this 1:1 in ReCoM". 0.40.0's asset run printed menu sounds 1 (move) and 3 (back) as "stream 1 / 3:
  empty".
- Finding (Checked (files): the owner's `se000.win32.scd`): the SCD maps each sound to its streams
  through a table at `tables + 0x20` (16-byte entry per sound, then (u16 track, u16 stream) pairs).
  Move 1 → stream 221, back 3 → 220, error 4 → 219, open 6 → 218 (stereo, 0.1–0.25 s audible); streams
  0–3 and 5 are empty slots. 174 of 193 tracks' waits equal their stream's length exactly. What the game
  plays never depended on this (both drivers resolve the table; same driver body, SCD unchanged); the
  step's printout, manifest `stream` and preview WAVs were wrong.
- Finding: the 1280 bytes past the SCD's size word are the end of stream 222, not padding (0.39.2's
  Assumption was wrong).
- Change: asset step reads the sound table (`streams_of`), prints / previews the right stream, writes
  `streams` per id in the manifest; keeps the asset whole (note when streams pass the size word).
  DLL: `scd_head_ok` takes a size word up to the pack's size (was: equal). Manifest format unchanged.
- Checked: files (table, durations), tests. Assumption: Re:CoM's driver reads past the size word as
  KH1's does (both get the whole asset). Not heard in game.

---

## Stage 81: MP ring drawn as Re:CoM draws its HP ring; closing outline; follows the gauge; room plate top right, version 0.40.0 (2026-10-31)

- Asks (owner, 2026-11-01): "the MP bar doesnt have the outline at the bottom of the bar", "the health
  bar moves, can we make the MP bar move at the same time?", "the bottom right of the screen shows the
  type of room. can we move this to the top right", "a resolution difference between the MP bar and the
  health bar … maybe built the same way".
- MP ring: 561 groups in kh1hud:00 (manifest "mp_ring"): KH1's three quarter-ring pictures whole in
  the atlas, cut per frame by Re:CoM's scissor sprites (`0x14024B910`, reset `0x14024A3F0`) and rotated
  as Re:CoM's HP ring (angles negated on load `0x14024B4E0`); no pre-rendered pictures. Differences:
  the charge's partial end in 1-unit steps without KH1's soft fade; max-MP ends in the right
  quadrants up to 0.9 unit off. Checked (decomp + offline render).
- Outline: a 2-unit black bar under the ring's 9 o'clock start (KH1 closes it with the label plates).
- Shake: the ring's controls hang from the HP control `+0x78` (`0x140248650`), own colour.
  That Re:CoM moves `+0x78` itself: Assumption (no movement keys in its groups; Checked (files)).
- Room plate: wrapper on `CNaviMap`'s plate set-up `0x1401E44A0` (15 bytes checked): its two
  controls (`+0x128`) 200 units up, under the minimap; `hud_room_plate_top_right` (default on).
  Measured on Destiny Islands only. Not seen in game.

---

## Stage 80: KH1 menu sounds: se000's padding left out, version 0.39.2 (2026-10-31)

- Owner's asset run of 0.39.1: "se000.win32.scd: SCD says 15277472 bytes, has 15278752" (refused).
- The asset holds 1280 bytes after the SCD's own size word; the step now keeps the SCD's said size
  (the bank Re:CoM registers) and notes the padding. Checked (owner's run); the padding being
  padding: Assumption (zeros after the last stream).

---

## Stage 79: KH1 menu sounds: entries of se000's one SCD, version 0.39.1 (2026-10-31)

- Owner's asset run of 0.39.0: "amusic/se000.dat has none of KH1's menu sounds (its sound assets:
  [se000.win32.scd])".
- KH1 (Checked (decomp + exe)): bank 2 (`0x1404D5EB8`) is `se000.win32.scd` (number atoi(name+2) = 0,
  registered at start `0x14017A080(2, …)`); `SslSeCreate(2, id)` (`0x1400DE460`) plays entry `id` of it
  (`CreateSound(h, id, 0, 1, 0)` at `0x1400DE50E`). Re:CoM's create does the same with `id % 1000`
  (`0x1402F314E`, the same engine function). So the whole SCD is bank 9000 and sound n plays as
  9000000 + n. Asset step: the SCD unchanged, per-id info + preview WAVs. Not heard in game.

---

## Stage 78: KH1's own command menu sounds, version 0.39.0 (2026-10-31)

- Asks (owner, 2026-11-01): "going through the command menu plays the wrong sounds" → KH1's own sounds
  (owner's choice); "notice the quality of the menu and the sounds playing … lets try to get this 1:1".
- KH1 (Checked (decomp + exe)): `0x14017A040(id)` plays bank 2 sound `id` = the PC asset
  `se<id as 6 digits>.win32.scd` (lookup `0x1400E0A50`, `0x1400DB1F0`); menu ids 1 move, 3 back,
  4 error, 6 open; no sound on a command / spell issue (the owner's video agrees: 16 moves one
  sound, Magic's open another, no confirm sound).
- Asset step: ids 1/3/4/6 from `amusic/se000.dat`'s assets, whole SCDs, into kh1-menu-sounds.bin /
  .json. DLL: hook on Re:CoM's bank map `0x1402A8560` (13 bytes checked): groups 9000–9999 are
  ours; on first play the SCD is registered with Re:CoM's `0x1402F5D70` (Re:CoM's bank list owns
  it) and played by Re:CoM's `0x1402AA360` as 9001000 / 9003000 / 9004000 / 9006000. Re:CoM's ids
  stay the fallback. Not heard in game; se000.dat holding these ids: Assumption (checked per id
  by the step).

---

## Stage 77: KH1 command menu words at KH1's brightness, version 0.38.1 (2026-10-31)

- Ask (owner, 2026-11-01): "for the command menu, lets try to get the resolution of the graphics to be
  greater than how it is … lets try to get this 1:1" (video of KH1's menu).
- Side by side at the same size (owner's 2560 × 1440 screenshot vs the KH1 video): the art is the
  same PC textures at the same density (KH1 PC's menu pictures are 2× too; Checked (files)); the
  visible difference was the words: ours pure white, KH1's off-white. The baked glyphs are KH1's grey
  doubled, so the level is bright / 255 (0xE6 → 230), not bright / 128 (clipped to white). Words in
  rows, the cursor row and the tab. Not seen in game.

---

## Stage 76: Re:CoM's HP gauge at KH1's place, turned 180°, with KH1's MP ring, version 0.38.0 (2026-10-31)

- Ask (owner, 2026-11-01): "Lets just keep the KH1 command menu and the triangle prompts. For the health
  bar, is it possible to just get the Re:CoM health bar, place it in the same place as the KH1 health
  bar, and rotate the ReCoM green health so its in the same orientation?" (mockup confirmed; MP
  "Around the ring").
- Re:CoM's ring is scissored from parts' corners with no min / max (`0x14024A3F0` → `0x1402643D0`):
  a turned control would empty the scissor. So the asset step adds `gauge:01` groups 7–9 again,
  turned 180° (47–49), in gauge.lad and its 5 copies; the DLL swaps layers 1–3 of the HP control
  for them once at init (`0x1402482F0`, layer replace) and moves the control to KH1's ring centre;
  face and disc stay upright. Re:CoM runs the gauge itself after that. Checked (decomp + exe +
  offline render of the owner's UK_gauge.lad).
- MP: the KH1 cluster's MP back / MP / charge / ticks only, 1:1 outside Re:CoM's ring (42–55 vs
  29–43). Keys `hud_recom_hp_kh1_place`, `hud_kh1_mp_ring` (default on); `hud_kh1_gauge` default off.
  Not seen in game; Re:CoM's gauge art not rendered offline (not in session).

---

## Stage 75: HUD pieces as settings, Re:CoM's LEVEL UP, smooth rings, tab seam, prize diagnostics, version 0.37.0 (2026-10-31)

- Asks (owner, 2026-11-01): "I want to be able to pick and choose which elements i want", "I want
  the ReCoM level up message instead of the KH1 level up hud", "the health bar has this grainy harsh
  looking texture ... for the MP, youll notice that line thickness varies", "the triangle prompt hud
  has a weird like line through the middle of it", "sora is frozen after battle again".
- `settings::HudParts` (7 ini keys, defaults = the owner's picks); `kh1_hud::gauge_on` / `menu_on`;
  Re:CoM's pieces hidden per bit. H8 off. Config rows: no free item slots (owner question).
- Rings: 4 × 4 bilinear samples a PC texel, cut edges + half a texel (offline render: even
  outlines). Seam: band 1 unit up under the stripe.
- Stuck battle end: the manager waits on 5 `CPrize` (log); the battle-end line now gives each
  prize's kind, words +0x240..+0x260 and position (diagnostics). Not seen in game.

---

## Stage 74: Context tab at save points and warp points, version 0.36.1 (2026-10-31)

- Ask (owner, 2026-11-01): "standing on a save point and using the purple teleporter thing dont
  prompt the triangle to interact".
- Re:CoM (Checked (decomp + exe)): `CGimmickSavePoint` (kind 0x2004, update `0x1401C5180`, in range
  `+0x2C0` = 1) and `CGimmickWarpPoint` (0x2005, update `0x1401CC280`) read Triangle from the pad
  themselves (`0x14029B550`) on the lock / nearest target (`player+0x850`, else `+0x84C`) with
  Sora's action 1..4; the field action `0x1401E80C0` doesn't handle them.
- KH1: the save point's context command 0x36 "Save" (message 293, `0x1402AD11F`); no warp word
  in KH1: "Examine" (5) built (owner choice). Check-only sites; no hook. Triangle still saves /
  warps as before. Assets: words 5 and 293 added (asset step re-run). Not seen in game.

---

## Stage 73: HUD fixes: HP in KH1 units, Re:CoM lock-on, menu text / battle colour, smooth rings, crash notes, version 0.36.0 (2026-10-31)

- Asks (owner, 2026-11-01): "the font is oddly positioned when hovering over an option. the health
  bar and MP bar are REALLY ugly looking", "lets just use Re:CoM's lock on graphics", "lets have
  Re:CoM's health bar" (+ remove LOCK ON!), "when in battle, the command menu still has a blue
  tint", "losing health doesnt affect the health bar, and my game just crashed".
- HP: Re:CoM HP × 120 / 560 (Re:CoM's cap `0x230` = KH1's `0x78`), rounded up. Checked (decomp).
- Lock-on: H5 / H7 off; Re:CoM's cursor and enemy gauges back (mask bit 0x10 no longer added).
- Menu: cursor-row word y 22 (owner's KH1 screenshot); battle selection gradient red
  (`0x14027A2B0`, Checked (decomp)).
- Rings: PC bake samples bilinearly (offline render). Crash notes: `win/crash.rs` (vectored
  handler, log only). Crash cause: not known yet (no dump). Not seen in game.

---

## Stage 72: KH1 FM context tab (green Triangle prompt), version 0.35.0 (2026-10-31)

- Ask (owner, 2026-11-01): "the menu shouldnt have the context option at the bottom, it should behave
  like the KH1 final mix version, where the green triangle prompt appears when interacting with
  something".
- KH1 FM (`0x140278B40`, `0x140277CF0`, `0x140279FC0`; Checked (decomp + exe + files)): the root
  list is Attack / Magic / Items / row 3 (Summon once learned, else KH1's greyed dash); the context
  is never a row: a green tab (`com` objects 4–7 widened, picture 2's PC-only rows, vertex
  colours) at (list x + 9 + slide, 250) over the COMMAND header, the word at + (18, 7), the
  Triangle (`0F 46` font escape, `Icon.tm2` 0xE411) at (list x + 6 + slide, list y − 3); slide
  x −10 → 0 and alpha 0 → 1 over 14 frames (7.3 read the alpha as an X scale). Triangle runs it.
- Logic: `Scene::tab` replaces the row-3 context; confirm / Right on row 3 is the error sound.
  KH1 look: plate, Triangle and word controls. Re:CoM look: the highlighted plate tinted green
  at (8, 232), no Triangle (none found in `Cockpit`). kh1-hud.json format 3 (DLL rejects 2).
- Checked: tests, Windows build, offline render of the bake. Assumption: the PC's icon remap
  (40 × 48 cell drawn 16 × 19 units). Not seen in game.

---

## Stage 71: KH1 HUD polish: KH1's menu font, fourth row, no seams, version 0.34.0 (2026-10-31)

- Ask (owner, 2026-11-01): "the bars are all choppy, the text is wrong, theres no bottom row to the
  command menu, lets fix this".
- Words: KH1's system font (`sysfont` PC texture + `exchange/US_font_data_tbl.bin`), not
  `Theater`'s `menufont`; "Attack" 44.3 units (44.5 on the owner's screenshot). Checked (files +
  decomp `0x1402D1570`, `0x1402D2050`). Vertical place by eye: Assumption.
- Fourth row: KH1's look always draws root row 3 (context command, else "Summon" greyed) with
  the last-row frame (DLL `win/command_menu.rs` `with_summon_row`).
- Seams: own atlas boxes for every rect drawn from a whole picture, clear texels take their
  neighbours' colour, the PC ring quarters cover their cut edges. Checked: offline bilinear
  render of the bake. Not seen in game.

---

## Stage 70: KH1 HUD: straight HP bar drawn without brackets, version 0.33.1 (2026-10-31)

- Ask (owner, 2026-11-01): "for some reason the hp is segmented".
- Cause: KH1 PC's pictures 5 / 7 are repacked where the straight HP / MP pieces sit; `g_s` keeps
  the PS2 UVs, so the PC copy showed bracket pieces every 26 units (reproduced offline). Asset
  step: rects whose PS2 coverage is lost on PC are redrawn from the PS2 texels at 2× (40 rects on
  the owner's files); everything else keeps the PC art. Checked (files + offline render of the
  bake). Assumption: KH1 PC remaps those UVs somewhere we have not read. No DLL change.

---

## Stage 69: KH1 HUD H8, LEVEL UP!! pop-up, version 0.33.0 (2026-10-31)

- Re:CoM's `0x14020C9A0` kind 4 is the level-up (Checked, caller + sound 0x36); hook
  (`48 89 5C 24 10 56 48 83 EC 20`) shows KH1's LEVEL UP!! box at (16, 16) (drop in, hold, 8-frame
  fade) instead of Re:CoM's popup; Re:CoM's level-up choice stays. Not mapped: EXP (KH1 floats it
  per enemy, Re:CoM counts at battle end), munny / Moogle points, item windows (M8). Not seen in
  game.

---

## Stage 68: KH1 HUD H7, KH1's lock-on cursor, version 0.32.0 (2026-10-31)

- Wrapper on Re:CoM's lock-on marker update `0x140209D30` (`40 57 48 81 EC 80 00 00 00`): Re:CoM's
  cursor layer hidden, KH1's `lockon_c` on our projected control (`0x1402471D0`) at the same
  lock point, not scaled by distance (KH1). Not seen in game.

---

## Stage 67: KH1 HUD H6, Donald's and Goofy's gauges, version 0.31.0 (2026-10-31)

- KH1's geometry and faces top-right, shown while Re:CoM has them out as friends: hooks on
  Donald / Goofy appear `0x1401DDC20` / `0x1401DF9C0`, CFriend exit `0x1401D94D0` and dtor
  `0x1401D90F0` (first bytes checked). Re:CoM's friends have no HP / MP: full rings. At Combat =
  KH1 there are no friend cards, so they show only at Combat = Re:CoM with `hud = kh1`. Not seen
  in game.

---

## Stage 66: KH1 HUD H5, LOCK ON! and the target gauge, version 0.30.0 (2026-10-31)

- KH1's rules: origin (16, 24), +28 under the plate, body min(max HP / 2, 150), one bar per 300 HP,
  marker on the top bar, 5-frame gauge / 6-frame plate fades; follows Re:CoM's lock each battle
  frame from the request-bits wrapper; max HP from `0x14017F5A0`. Not seen in game.

---

## Stage 65: KH1 HUD H4, KH1's command menu look, version 0.29.0 (2026-10-31)

- At `hud = kh1`: KH1's `com` / `com_battle` / `mag` pieces, field look in the field, battle
  look in battle rooms (title red over 20 frames), Magic list in KH1's purple, Summon row greyed;
  7.3's rules unchanged; `hud = recom` keeps 7.3's look. Words: KH1's menu font glyphs (Theater
  `uk/font/menufont`) baked as sprite parts per word and language (Re:CoM's font format isn't
  decoded: owner question). Not seen in game.

---

## Stage 64: KH1 HUD H2b, shake, slides, HUD fade, soft arc ends, version 0.28.1 (2026-10-31)

- Every distinct piece set of Sora's cluster (852 units) gets played groups baked at the asset step:
  KH1's three hit shakes (10/22/38 frames), two size-class slides, the 10-frame fade out / in; the
  DLL swaps the held layers for these units only while a motion runs (play-from-time `0x140248550`,
  layer time `0x140248000`, first bytes checked). KH1's 0.7° soft end baked into the HP, charge and
  danger arcs (Checked, decomp + files). KH1 restarts MP / charge fades from scratch (H2 already).
  Not built: back-arc end strips. Not seen in game.

---

## Stage 63: M7 stage 7.11, spell unlocks, version 0.28.0 (2026-10-31)

> Decision 34: base level with the Re:CoM magic card's first pickup; -ra / -ga at Re:CoM bosses

- Hook on Re:CoM's "first obtained" `0x14015C7F0` (`48 83 EC 28 B8 FF 00 00 00`), which every
  card pickup passes: magic cards (kinds 0x1A–0x20) give level 1; a boss's enemy card's first
  pickup (= its first defeat) raises its spells one level each (table in
  `docs/recom-m7-unlocks.md`: two bosses per spell). Levels at `save+0x267A`, raised only; old
  saves sync once (bit 0 of `save+0x2684`). `kh1_all_spells` still overrides. Owner to review
  the table. Not seen in game.

---

## Stage 62: Thunder's charge effect, version 0.27.4 (2026-10-31)

- Thunder's cast now plays Re:CoM's own Thunder charge (its cast setup at `0x14022062B`, 108
  bytes checked: on the Keyblade's bone 2, kept on the blade) as Cure / Gravity / Stop / Aero do.
  Not seen in game.

---

## Stage 61: Stop's held hits at KH1's pace, version 0.27.3 (2026-10-31)

- KH1 (Checked, decomp): each held hit after the first lands 6–11 frames after the previous
  (`6 − (int)(r·−6)`). In the existing enemy-hit wrapper, Re:CoM's per-landing timer is set to
  `gap·50 − 2·dt` after each landing; Re:CoM's 8-hit cap kept. Checked sites `0x140184910`,
  `0x14018499D`, `0x1401A5A7F`. Not seen in game.

---

## Stage 60: Gravity: KH1's squash under the falling sphere, version 0.27.2 (2026-10-31)

> Decision 38 asked for a slow; KH1 has none: its field `+0x480` only scales the model's height
> (Checked, KH1 decomp + .mag). Built KH1's squash; owner question.

- Enemies whose bodies touch the falling sphere (170 cm × scale) squash toward 0.8 height with
  KH1's wobble `(s−0.8)·0.95+0.8−sin φ·0.05` (φ += π/2 a frame) and spring back 0.05 a frame,
  through Re:CoM's own scale getter / setter (`0x1401A4A20`, `0x1401A7050`). Runs only while a
  sphere falls or an enemy springs back. Not seen in game.

---

## Stage 59: Fire's release step as KH1, version 0.27.1 (2026-10-31)

> Decision 38: Fire's release step is KH1's 15 cm curve

- In the existing root-step wrapper (`0x14024DCA0`), during a KH1 cast playing row 0x38
  (`p001_063`), Re:CoM's per-update travel is replaced by KH1's curve (−15 cm at frame 10, −10 cm
  at 60) along Re:CoM's line. Only Fire's ground release differs (air and other casts match;
  Checked, both games' motion files). Not seen in game.

---

## Stage 58: KH1 HUD H3, Re:CoM's HUD hidden, version 0.27.0 (2026-10-31)

- At `hud = kh1`: Re:CoM's HP gauge, FRIENDS row and prize gauge (EXP+ / NEXT LV / Moogle
  points) drawn at alpha 0 (`0x140248590`) so their logic keeps running (the battle end waits on
  the prize gauge; Re:CoM's low-HP sound plays); enemy gauges and the minimap (room plate assumed
  with it) hidden through their own hide paths: five mask reads (bits 0x10, 8) via our getter while
  the KH1 HUD is up. Kept: level-up choice, Guard / Recover / Resist / LEVEL UP! popups, the
  command menu. Not seen in game.

---

## Stage 57: KH1 HUD H2, Sora's HP/MP cluster, version 0.26.0 (2026-10-31)

- At `hud = kh1`: Sora's cluster bottom-right at KH1's places and margin (+86): back / front /
  face `C2DSeqCtrl`s on `kh1hud:00`; HP on raw Re:CoM HP at 5° per HP to 120 HP then segments,
  MP, charge; KH1's MP/charge fades, hurt face (40/50/60 frames), red danger flash, size-class
  offset. Updates on events only: wrappers on Re:CoM's HP setters `0x14020D6A0` / `0x14020D8E0`
  (first bytes checked) and the MP hooks. The 7.2 bar isn't made at `hud = kh1`.
- Not built: hit shake / size-class slide, arc end strips and 0.7° soft edge, KH1's 10-frame HUD
  fade, MP fade resume (owner questions / gaps in `recom-kh1-hud.md`). Not seen in game.

---

## Stage 56: KH1 HUD H1, the kh1hud sprite set, version 0.25.0 (2026-10-31)

> Decision 39: an option where Re:CoM's HUD is replaced by KH1's, KH1's exact graphics and places

- KH1's HUD is data (Checked, files + decomp): 24 `exchange/<L>_uibin_*.bin` layouts ("TTUI" v2,
  all 120 + com_battle + danger decoded from the owner's dump), `command2/uitex.bin` pictures,
  portraits in `.mdls` section 2. The asset step builds `kh1hud.lad` into `FO0002.SPR` and
  `kh1hud.imz` into `FO0002.RTM` (PS2 1024×512, HD 2048×1024, FR/GR/IT/SP/UK copies), KH1's arcs
  baked by running KH1's fan code (`0x14026B790` / `0x14026E450`) over its ring textures.
- Re:CoM registers `kh1hud:00` (632 groups) as its own; `kh1-hud.json`; interim switch
  `hud = recom | kh1` in the ini (default recom) until the KH1 camp entry. `docs/recom-kh1-hud.md`.

---

## Stage 55: 2D draw lists: no leak from Config flips; pool counter, version 0.24.4 (2026-10-31)

- Re:CoM's 2D pool: 2047 usable entries (`0x140245293`); each C2DSeqCtrl layer (sprite group or
  attached C2DFnt) takes one, only the ctrl frees it; a release looks only in the current draw list
  (`0x1402499C0` → `[0x14087CE70]`). Checked (decomp + exe).
- Leak found: flipping Combat in the camp's Config built / deleted our bar and menu rows on the
  camp's list 1, so their releases failed (~13 entries per flip). Now each gauge records its list
  at init and our adds / clears / deletes run on it (Re:CoM's setter `0x140249800`, switched back
  at once). Normal play already balanced to zero. Log: `2d: …` lines (ours / Re:CoM's free
  count); the battle-end log ends with the pool state. Not seen in game.

---

## Stage 54: Battle end: KH1 yields to Re:CoM's win; battle-end log, version 0.24.3 (2026-10-31)

> "after finishing a battle in the field sora is locked in place and doesnt return to the field"

- Re:CoM's battle end (Checked, decomp + disasm; `recom-m6-battles.md` § Battle end): last enemy
  → `player+0x428 |= 4` (`0x1401F4A60`) → win action 0x1C (`0x1401F1B20`) → manager
  `0x1401871D0` states 1 (children, enemies) → 2/3 (prize gauge `+0x94`) → 4 (win pose done
  `0x1401F3C90`) → 5/6 (roulette, task 0x22) → 7 (flag (1, 2), room script returns Sora).
- Owner's trace 13: win pose finished (0x1C sub 4), no enemies: the manager waits elsewhere; not
  identified. None of our hooks writes the manager, prize gauge, pause flag, roulette or flag.
- Fixed a real deviation: a KH1 cast or swing ending after the win overwrote Re:CoM's win action
  for a frame (0x1401F1B20 ran twice); `Kh1Sora::yield_to_recom` now stops our frame then.
- New `battle end:` log lines and an `end …` trace token name what Re:CoM waits for (manager
  state, children by class, prize gauge states and animation ids, pause flag, roulette, flag
  (1, 2)). 9 checked-only sites. Not seen in game.

---

## Stage 53: MP bar under the FRIENDS row, version 0.24.2 (2026-10-31)

> Owner's video (2026-10-31): the MP bar sits over Re:CoM's FRIENDS row (label and friend cards
> under the HP bar), and after Combat Re:CoM → KH1 the rebuilt bar appeared far off.

- The bar (and its fade pieces) now sits 40 units lower, under the friend cards (measured on the
  video: cards' bottom ≈ 84 units, the bar's top was 50). Assumption until a screenshot.
- The off-place rebuild is what 0.24.1's rest transform addresses (not yet seen: the game was
  running 0.24.0 in the video). Not seen in game.

---

## Stage 52: Command menu installs; MP bar placed at rest, version 0.24.1 (2026-10-31)

> "The command menu isnt there. the MP bar clips through the \"friends\" bar, and also, when switching
> from KH1 to Recom, and then back to KH1, the MP bar is randomly placed somewhere else each time"

- Cause 1 (Checked, owner's log): the command menu's install checked `menu_groups()`, which is
  None until a bar is built on the first player gauge (later), so it logged "no plate groups" and
  never installed. Now it checks the manifest's `extra_groups` at install; the groups are still
  resolved when the bar is built.
- Cause 2 (Assumption, likely): a rebuilt bar's new `C2DSeqCtrl` kept whatever transform the
  reused memory held. Every control we make now gets position (0, 0, 0, 1) and scale 1 at once.
- The overlap with Re:CoM's friend gauge is not fixed yet: screenshot requested. Not seen in game.

---

## Stage 51: M7 stage 7.10, Aero, version 0.24.0 (2026-10-31)

- Aero / Aerora / Aeroga (Checked, KH1 script + decomp): cast at 30 or at once when cut; 2 MP;
  barrier (18 + max MP) × 60 frames halving Sora's damage (÷ 2, ≥ 1) in the existing damage-check
  wrapper; re-cast restarts it. Aerora/Aeroga's wind: Re:CoM's Aero shot as a real attack 0.8 m
  around Sora, collided by Re:CoM, KH1 damage 8 + 2 × max MP, a new hit serial every 30 frames
  (`0x14016F7D0`). Plain Aero: the shot without collision as the aura. Not seen in game.

---

## Stage 50: M7 stage 7.9, Stop, version 0.23.0 (2026-10-31)

- Stop / Stopra / Stopga (Checked, KH1 script + decomp): at 28, every enemy in a 2.5/3.5/5 m,
  ±3 m cylinder, nearest first, one per 4 frames; (2 + max MP) × 60 × KH1 resist[9] frames, 0 =
  immune (Re:CoM-only bosses); 2 MP. Re:CoM's own stop state (kind 8, start `0x14017C740`)
  already holds up to 8 hits and lands them after (Checked, decomp); its gate `+0x2C` is set from
  KH1's resistance per strike; stopped enemies' attacks off with `0x1401A2D90`. Every boss class
  has the stop state; non-enemy parts can't be stopped. Not seen in game.

---

## Stage 49: M7 stage 7.8, Gravity, version 0.22.0 (2026-10-31)

- Gravity / Gravira / Graviga (Checked, KH1 script + decomp): sphere 2.4 m above the target (no
  target: 2 m ahead) falls 2.3 m over 60/70/60 frames growing to 1.0/1.5/1.6; impact at 88/98/88
  hits every enemy within 1.7 × scale m once: current HP × (power + 5 × max MP)% × KH1 resist[8],
  at least the floor (Re:CoM-only bosses 2 %); 1 MP.
- Re:CoM's Gravity shot (collision held) + Re:CoM's hit function at impact, as a flinch carrying
  KH1's number (Re:CoM's Gravity kind can't carry it and flattens for 3 s); Re:CoM's gate `+0x40`
  isn't consulted at KH1. The slow under the sphere isn't built (owner question). Not seen in game.

---

## Stage 48: M7 stage 7.7, Cure, version 0.21.0 (2026-10-31)

- Cure / Cura / Curaga: heal 36 frames after the command, locked for 68, 1 MP paid at the heal;
  cut or not healable = no heal, no MP (Checked, KH1 script + motion). Heal = Re:CoM's own Cure
  case `0x140222946` (effect + heal `0x1401E59A0`) for Re:CoM's 50 / 150 / 300 HP (decision 34);
  Re:CoM's charge effect from `0x14022070A`. Not seen in game.

---

## Stage 47: Fire release root curve as KH1, version 0.20.1 (2026-10-31)

- KH1's Fire release motion (Checked, files): back 1.5 cm a frame to −15 cm at frame 10, then
  0.1 cm a frame forward to −10 cm at 60 (air −42.41 cm at 10, held). The motion record now holds
  that curve (it held −15 cm). In game Sora's travel still comes from Re:CoM's clip `p001_063`
  (−40.5 cm at frame 10): owner question. Not seen in game.

---

## Stage 46: M7 stage 7.6, Thunder, version 0.20.0 (2026-10-31)

- Thunder / Thundara / Thundaga (Checked, decomp + files): release at 30, locked for 68 frames,
  1 MP; every enemy (up to 30) nearest first from the target (no target: Sora; only the marker
  sits 2 m ahead along the camera) whose hurt body overlaps a 6 m radius, ±5 m cylinder; first
  strike at release, then one per 8 frames, each enemy once.
- Each bolt: Re:CoM's own Thunder spawn made at that enemy (effects + bolt shot, its collision
  off), then Re:CoM's own hit function on the enemy's first hurt body (the call its collision pass
  makes after a contact; vtable slots `0x140523E30` / `0x140523E60` compared per bolt). Damage
  KH1 power + max MP × 2, thunder resistance. KH1's 50 % world dim not built (owner question).
  Not seen in game.

---

## Stage 45: M7 stage 7.5, Blizzard, version 0.19.0 (2026-10-31)

- Blizzard / Blizzara / Blizzaga from the command menu (Checked, decomp + files): release at 28
  only while the cast motion plays (else no spell, no MP); motion 86 free at 68 (air 78 / 68);
  cost 30 units; 7 shards (6 on a 22.5° cone + 1 straight, 19.15 cm off axis), 20/30/40 cm a frame,
  26 frames, radius 70/90/120, a hit or wall ends a shard; glide 50 cm over Re:CoM's floor test.
- One hit per enemy per cast: KH1's shared hit bit (Checked, decomp) = Re:CoM's shared hit serial
  (`0x140156770`), so Re:CoM's own hit handler does it. Built from Re:CoM's Blizzard spawn
  (`0x1402224A7`, `0x1402224E9`) and rows `0x1407C6EA0` / `0x1407C6F20`, Re:CoM's effects.
  Damage: KH1 power + max MP, KH1 ice resistance only. Not seen in game.

---

## Stage 44: Fire re-targets as KH1, version 0.18.1 (2026-10-31)

> Decision 36: Fire re-targets as KH1

- KH1's Fire script re-picks only for a ball cast with no target (a dead target: the ball flies
  straight; Checked, decomp + files). The pick `0x1402B0E90`: enemies within 5 m of the ball,
  score distance × (1 + 2 × (1 − cos angle to the ball's heading)), lowest wins, every frame.
- Our target-less balls get their own per-frame step: a wrapper writes KH1's pick into Re:CoM's
  homing target, then runs Re:CoM's homing step (`0x1401575D0`, via the thunk `0x1401560B8`;
  CHomingBullet vtable +0xA8 `0x1405255D8` compared at install). Candidates: Re:CoM's lock-on list
  filtered as its nearest pick (`0x1401CDBCF`) does. Re:CoM's shots never run it. Not seen in game.

---

## Stage 43: M7 stage 7.4, the cardless cast and Fire, version 0.18.0 (2026-10-31)

> Decision 34 (M7 "FULLY implemented"; Re:CoM's spell visuals and sounds)

- Choosing Fire / Fira / Firaga in the command menu casts it through Re:CoM's own magic action
  0x13, its cast clips and its homing-shot runtime, with Re:CoM's Fire effects. One entry point
  `win::magic::issue(spell, level)` for the menu and (7.12) shortcuts.
- KH1's timing (Checked, decomp + files): 28-frame wind-up, release motion 60 (air 52) free from
  46, ball in frame 28; fast path (Fire's release motion from frame 14, swing 1 / lunge windows):
  no wind-up, ball 8 frames after the command; cut before release = no ball, no MP; air hop 30 cm,
  air speed halved. Ball 40 cm/frame × 30 frames, 3°/frame homing, radius 50, one hit.
- MP: can_pay before (not on the fast path, as KH1), `pay(30)` at release. Damage: spell power +
  max MP through KH1's curve, twin/proxy Defence, floor, cap and KH1 fire resistance (Re:CoM's weak
  part and boss falloff on top).
- Re:CoM's init isn't called (it reads the played card's table, not loaded in the field); the cast
  does its writes cardlessly and points the row at KH1 rows built on Re:CoM's Fire rows
  (`0x1407C6E90`, `0x1407C6F00`). Hook: magic exe `0x140217EE0` (`48 89 5C 24 08 48 89 74 24 10`).
  The Field exe's jump table gets an action-0x13 case (`0x1401FA63C` → a 13-byte stub in the int3
  cave `0x1401FB0E1`). `docs/recom-m7-magic.md`. Not seen in game.

---

## Stage 42: HD atlas scale for the bar and the menu plates, version 0.17.1 (2026-10-31)

- The owner's `FO0002.SPR` language copies are HD layouts with every UV × 2 (Checked, files), so
  0.17.0's asset step found no text plate there and wrote the bar's sprites at PS2 UVs. The step
  now scales the bar's and the plates' UVs by each set's atlas scale and finds the plate in every
  copy. Rebuilt and installed on the owner's PC. Not seen in game.

---

## Stage 41: M7 stage 7.3, KH1's command menu, version 0.17.0 (2026-10-31)

> Decision 34: command menu from Re:CoM's pieces, laid out like KH1's, field and battle, plate
> colour cursor, KH1's own words

- Attack / Magic / Items / context row and the Magic list at Combat = KH1 in Sora's story, field
  and battle: each row a Re:CoM `C2DSeqCtrl` playing a Re:CoM text plate (2 groups the asset step
  adds to `Cockpit:00` after the MP bar's: plate, and plate with the magic plate's blue strip as
  the highlight) with a `C2DFnt` label holding KH1's word (`kh1-command-menu.json`, decoded from
  `btltbl.bin`'s `*_Word.bin`, 6 languages; ini `menu_language`, default `us`).
- KH1's rules (Checked, decomp): layout (16, 260), rows 22, Magic list at x + 11; fresh d-pad only;
  Right/confirm opens, Left/cancel back; brightness ±32 a frame, 10-frame fade, 14-frame context
  tab; Counterattack shown as "Counter"; greying by `Mp::can_pay` and the 1100 range check.
- Confirm on Attack is today's Attack path. Spells buzz until 7.4 (cast). Items greyed (M8).
  Field context row from the object Re:CoM's Triangle uses (`0x1401E80C0`, read only): Open /
  Talk / Lift / Jump On. `kh1_all_spells = on` gives all 7 spells at level 3 until 7.11.
- No new hook or patch; runs from the MP bar's gauge wrappers and the request-bits wrapper.
  Checked sites: `0x140248590`, `0x1402476F0` (vtable `0x1406073A0`), `0x1401E80E8`, `0x1401B8A10`.
  `docs/recom-m7-command-menu.md`. Not seen in game.

---

## Stage 40: MP bar in every language copy, version 0.16.1 (2026-10-31)

- The asset step on the owner's files: `FO0002.SPR`'s language copies of `Cockpit.lad` have 157
  groups (the base file 151). Checked (files). Each copy now gets the bar after its own groups;
  `kh1-mp-bar.json` lists the first groups (151, 157) and the DLL picks by the loaded set's count.
- Built on the owner's PC; `raw/FORM/0002/FO0002.SPR` and `kh1-mp-bar.json` added to the mod and
  `mod.yml`. Not seen in game.

---

## Stage 39: M7 stage 7.2, the MP bar, version 0.16.0 (2026-10-31)

> Decision 34: "Re:CoM-style bar" under the HP bar, "Grows with max MP", colour "From KH1's files"

- A straight bar under Re:CoM's HP bar (where Riku's Dark gauge sits), field and battle, at
  Combat = KH1 in Sora's story: one 24-unit slot per max-MP point, 26 apart (KH1's straight
  segments), blue up to MP, KH1's orange charge line on top; KH1's fades (10 frames, ring part
  only). Checked (KH1 decomp: pitch 26, 30 units per slot, caps, fade rule).
- Drawn by Re:CoM: a `C2DSeqCtrl` built as the Dark gauge is, playing 8 animation groups the
  asset step adds to `Cockpit:00` in `FO0002.SPR` (and its language copies), coloured by vertex
  colours from the player's KH1 `command2/uitex.bin` (picture 5 / 7) over Re:CoM's 128-grey texel
  (`FO0002.RTM`). Hooks: CPlayerGauge init `0x140208360` (`48 8B C4 56 57 41 56 48 83 EC 70`), the
  second-bar state `0x14020C190` (`40 55 57 48 83 EC 48`), the deleting dtor `0x140207770`
  (`40 57 48 83 EC 30`). Updates only on MP events. Re:CoM setting / Riku's story: nothing made.
- Asset step: `raw/FORM/0002/FO0002.SPR` + `kh1-mp-bar.json`. Assumptions: texel × vertex / 128
  on PC, vertex order, position (screenshot needed). `docs/recom-m7-mp-bar.md`. Not seen in game.

---

## Stage 38: MP refills where HP refills; KH2 Keyblades' MP, version 0.15.2 (2026-10-31)

> Owner's answers (decision 35): MP refill "Where HP refills"; KH2 Keyblades' MP "Use KH2 Magic"

- Re:CoM's three `HP = max HP` writers (Checked, decomp + exe): the state step `0x1401AC9D0`
  (wrapper), the checkpoint heal in `0x140237CE0` (its call `0x1402374C0(0)` at `0x140237F3A`
  retargeted through a jump placed near it; context bytes checked; no branch into it), the level-up
  HP choice in `0x14010EF10` (wrapper). After each, at KH1, MP goes to full (`restore_full`).
- The six KH2 Keyblades' `magic` (KH2 Magic) is now their max-MP bonus.
- Assumption: no other full heal; what each moment is in play isn't traced. Not seen in game.
- `docs/recom-m7-mp.md` § Refills and the KH2 Keyblades.

---

## Stage 37: KH1 numbers for enemies without a twin, version 0.15.1 (2026-10-31)

> "it feels like any enemy that doesnt have a KH1 counterpart barely takes any damage in
> comparison to the KH1 enemies, why is this and fix it"

- Cause (Checked, owner's F9 traces): twins took KH1 damage against KH1 HP (max-level Sora: 300
  = the cap, one hit, which is KH1's own result against first-visit records); every other enemy,
  every boss included, kept Re:CoM's floor HP and took Re:CoM's cardless AP = Sora's attack stat
  `player+0x43c` = 10 per hit.
- Re:CoM's enemy HP is the `@ENP` row of `ENEMY/<id>/EN<id>.EPD` picked by floor (the i16 at
  `0x14087B844`, getter `0x1402337D0` `0f b7 05 6d 80 64 00 c3`). Checked (decomp + exe + files).
- Asset step (`kh1-stats.json` format 5): Re:CoM HP rows of the twins and boss twins from
  `Recom.pkg`, 11 KH1 boss records (Guard Armor 700, Tutorial Leon 120, Genie Jafar 750, Cloud 300,
  Trickmaster 600, Parasite Cage 450, Oogie 450, giant Ursula 900, Hook 900, Dragon Maleficent 1200,
  Darkside 300) and per world × floor ratios (KH1 HP ÷ Re:CoM HP, lower-median twin).
- Runtime (no new hook; the stats-init wrapper `0x14017FF50` and max-HP getter `0x14017F5A0`):
  boss twins take their KH1 record; other enemies get a proxy: Re:CoM HP × the cell's ratio, the
  median twin's Defence / level / floor / cap, 100 % resistances; Re:CoM-only bosses the world's
  boss ratio and decision 34's Gravity 2 % / Stop immune. MP charge reads the proxy's level.
- The owner's `kh1-stats.json` was rebuilt on their PC with this build's asset step. Not seen in
  game.

---

## Stage 36: M7 stage 7.1, KH1 MP, version 0.15.0 (2026-10-31)

> "lets work on M7. Make sure all of that is FULLY implemented, also including a shortcut menu,
> customize menu, MP bar" (decision 34; plan `docs/recom-m7-plan.md`)

- KH1's MP at Combat = KH1 in Sora's story: max MP 3 + 1 at Sora's Re:CoM levels 12/24/36/48/60
  (btltbl `0x3AC0`) + the equipped KH1 Keyblade's MP bonus (weapon `+0x38`); MP points and the
  charge with KH1's exact rules: refill `0x1402A4CB0` (point k needs 30·k), add `0x1402A4A00` /
  `0x1402A4AE0`, can-pay `0x1402A4C10`, pay `0x1402A5B60`, charge per hit `0x1402BF330` (attack
  `+0x2C` × (1 + 0.2·Δlevel), 1..250 %), level-up `0x1402B51B0`. Checked (decomp + exe + files).
- No new hook: the refill step runs in the existing request-bits wrapper (one flag check per frame
  unless a point is waiting), the hit charge in the existing enemy-hit wrapper.
- Saved per save in the spare range `save+0x2675..=0x2684` (versioned; room for 7.11's spell levels
  and 7.12's shortcuts); all-zero = full MP. `docs/recom-m7-mp.md`.
- No HUD yet (7.2); log `mp:` lines and the F9 trace's `mp` token. Damage-taken charge waits for
  M9 (KH1 abilities). Not seen in game.

---

## Stage 35: no card duel at KH1, version 0.14.1 (2026-10-31)

> Owner: "c" (Marluxia 3's card duel: at KH1, the attack that starts it becomes a plain hit, so
> the duel never starts; decision 33)

- Re:CoM's damage check `0x1401E6DE0` reads the pending hit's reaction kind `player+0x710` once
  (after the hit-pending test `+0x720`); kind `0x21` calls the card-duel init `0x140220C40`
  (action 0x24). Kind 1 is the plain flinch. Damage depends only on kinds 8 and 0x22. Checked
  (decomp).
- Change: in the existing damage-check wrapper, at Combat = KH1 for the main Sora, a pending
  kind `0x21` becomes 1 before Re:CoM's check runs. One read per frame. No new hook.
- Marluxia 3 enters its duel mode (`+0xCE4 |= 4`, `0x1403F3590`) only while Sora's action is 0x24;
  otherwise its state 3 plays its own no-duel branch (motion 0x34). Checked (decomp).
- The hit also arms KH1's post-hit invulnerability (Stage 30). Not seen in game.

---

## Stage 34: M6 stage 3, bosses, version 0.14.0 (2026-10-31)

> "Lets move onto the next thing, go ahead" (M6 stage 3 per decisions 29, 30: bosses react to
> KH1 swings, human bosses keep a hidden move list, Marluxia 3's stock folder hidden)

- **P5:** Re:CoM's "Sora's action" getter `0x14017F720` is rewritten in place (32 bytes, Re:CoM
  `48 8B 05 59 BC 6F 00 48 85 C0 74 07 …`) so action 0xA (KH1 swings, air swings, Counterattack)
  answers 0xB, Re:CoM's card attack. All 15 callers checked (12 compare with 0xB, 3 with 0xC /
  0x22 in Riku's story; none with 0xA); nothing branches into the range. Switched per battle on
  its own (only a KH1 battle in Sora's story). Checked (exe + decomp).
- **Human bosses:** their revolver, CBTL and message 2 run as Re:CoM's; their card UI is hidden
  by Re:CoM's event-hide mask: side-2 cards (0x40 in stage 2's card mask), the enemy cue
  (`0x14015E3F3`, `0x140161A64`), the stock row (`0x140169CE3`, `0x140169D07`, `0x140169D48`,
  `0x14016E2B1`). 16 mask reads switch together. Hiding is visual only. Checked (decomp).
- **Marluxia 3's stock folder** hidden through its own mask reads (`0x1401690A6`,
  `0x1401690DD`, `0x140169103`). The hidden folder parks in state 5; state 6 only plays its
  cards in the card duel. Checked (decomp).
- No boss waits for a Sora card or a card break, except Marluxia 3's card duel, which Sora
  always loses at KH1 (only the Reload card): owner question. Readers of Sora's deck re-checked
  against the empty revolver. 203 site byte lists re-read, 0 mismatches.
- Assumption: bosses react to KH1 swings more often than to Re:CoM's attack cards.
- `crates/kh1-recom/src/boss.rs`, `docs/recom-m6-battles.md` § Stage 3 (per-boss checklist). Not
  seen in game.

---

## Stage 33: Enemy pace in the Config menu, version 0.13.4 (2026-10-31)

> "also do A" (Guard reactions and Enemy pace as Re:CoM's one-value "Game Mode" style rows, so
> both fit in the Config menu)

- Re:CoM's Config items: 21 slots at `camp+0xB38`, Re:CoM uses 0..13. Combat keeps two values
  (14..16); Guard reactions (17, 18) and the new Enemy pace (19, 20) are label + one wide value
  item built like Re:CoM's row 4: value sequence 8 of `0x1407C1240`, x element `0x16`, selected
  0, cursor animation `0x100`. Checked (decomp `0x1400DF1B0`).
- Left / right on a single row flip the setting and redraw (the value shows the other option).
  Enemy pace is written to `kh1-recom.ini` and read at the next battle start.
- Help: "Re:CoM: a crowd shares attacks. KH1: each attacks as if alone." (62 characters).
- Tests: slots 14..20 all used, styles, row 7 toggles, panel grows 3 rows. Assumption: the
  taller panel still clears the camp's bottom bars. Not seen in game.

---

## Stage 32: Config help lines fit the help box, version 0.13.3 (2026-10-31)

> Owner's Config screenshots: the help line is cut at the box's right edge, so the Combat
> row's "(mod v…)" never shows ("how can i check mod version in game?").

- Checked (screenshot): the help box shows about 69 characters of one line; Combat's help showed
  up to "combat." (68), Guard reactions' up to "KH1 com" (69).
- Change: shorter help lines (63 characters each): "Re:CoM: card battles. KH1: Kingdom Hearts
  combat. (mod v0.13.3)" and "On: blocked attackers flinch, blocked shots fly back. KH1 only."
  `menu::HELP_MAX` 90 → 66 (test). Text only; no address changes.

---

## Stage 31: Enemy pace setting, version 0.13.2 (2026-10-31)

> "is there a way to make enemies less aggressive? is there a system within KH1 that makes it so
> that enemies dont attack as frequently?" … "lets try b for now" (decision 32)

- Finding: KH1 has no shared attack limiter as far as checked (Shadow's script, the 26 common
  syscalls: Checked (decomp + files); the other syscalls: Assumption). The extra aggression is
  Stage 28's P4: Re:CoM's think rate `0x14017F380` is no longer multiplied by the enemy count,
  but each Re:CoM enemy's solo rate was tuned to be shared. Checked (decomp).
- Change: `enemy_pace = recom | kh1` in `kh1-recom.ini`, default `recom`. At each battle start
  (manager ctor wrapper `0x1401859B0`) P2 and P3 follow the battle as before and P4
  (`0x14017F3DE`, `83 F9 64 7D 26` <-> `83 F9 64 EB 26`) is ours only at `kh1`
  (`battle::wanted`, `plan_each`). Enemies can still attack while others do (no card slot, P2,
  P3).
- Not in the Config menu: Re:CoM's item array has one free slot left (`recom-config-menu.md`).
  Owner asked.
- Not 1:1 at `recom` (it is Re:CoM's pacing); `kh1` is decision 30. Not seen in game.
- The code went in with the previous commit; this one adds the version and notes.

---

## Stage 30: KH1's post-hit invulnerability, version 0.13.1 (2026-10-31)

> "Lets also carry over sora's KH1 mechanic where if he gets hit he has a period of time where
> he cant be hit again to prevent getting combo locked" (decision 31)

- KH1: every damage reaction sets flag `0x80` with 30 frames (`0x140297F30`), counted down by
  `0x140297FE0` from the reaction's release at 1 per 60 fps frame. Checked (decomp).
- Re:CoM's own hit-immunity timer `player+0x7BC`: the damage check `0x1401E6DE0` skips hits while
  it is above 0; the Battle / Field exes subtract dt. Checked (decomp).
- Change: in the existing damage-check wrapper, at Combat = KH1 for the main Sora, a hit
  reaction (action `0xC`) that starts holds `+0x7BC` at `1500 + dt` (30 frames) each of its
  frames; Re:CoM counts the 30 frames down itself after it. No new hook; one read per frame.
- Checked (test): 30 immune frames after the reaction at 60 and 30 fps. Assumption: leaving `0xC`
  = KH1's lock release; hit order within a frame (±1 frame). Not seen in game.
- `docs/recom-kh1-attack.md` § Post-hit invulnerability.

---

## Stage 29: M6 stage 2, no deck for Sora in battle, version 0.13.0 (2026-10-31)

> "The deck shouldnt be a mechanic when set to KH1. sora can still collect cards just in case
> they want to change from KH1 combat to ReCoM combat, but in battle there shouldnt be a deck to
> draw cards from when KH1 is selected" (decision 29)

- **Empty revolver** (hook `0x1401F43D0`, `40 57 41 56 41 57 48 83 EC 40`): at KH1 in Sora's
  story, outside minigame rooms, the battle revolver is built by Re:CoM's own constructor
  `0x140164D40` from Re:CoM's end-of-deck entry, which is what Re:CoM builds for a saved deck of
  0 cards (0 cards + Reload). It also replaces the tutorial deck. Checked (decomp + exe).
- No draw, Reload, stock or sleights follow from it with no more code (no card to play; Reload
  only from Re:CoM's battle branch, unused by KH1 Sora since Stage 9). Checked (decomp).
- **No Friend-card drops** (hook `0x14017E190`, `40 53 48 83 EC 20 48 8B D9`): the roll returns
  as Re:CoM's failed roll does. Its cards are all type 5, which only ever go into the battle
  revolver; the collection is untouched. Checked (decomp + exe tables).
- **Deck HUD hidden by Re:CoM's own event-hide mask** (variable (1, 0xE2)): seven reads
  (`0x140168A34/A93/B32/B90`, `0x14015037C`, `0x140151625`, `0x1401545BD`) call a stub that adds
  the cockpit bit 2 for this battle's empty revolver, or 0x20 for Sora's side-1 cards. Re:CoM's
  bytes go back at each battle start at the Re:CoM setting. No branch or pointer lands inside the
  patched bytes (exe scan); 163 site byte lists re-checked.
- Log-only readers `0x1401EA1A0`, `0x1401EA1E0`, `0x140167CF0`; the F9 trace gains `m6deck`.
- Assumption: the tutorial battles' scripts may wait for card flags (0x12, 0x14–0x16) that
  can't fire at KH1 (already true since Stage 9). No flags are faked; the log and trace show it.
- `crates/kh1-recom/src/{deck.rs, win/deck.rs}`, `docs/recom-m6-battles.md` § Stage 2. Not seen in
  game.

---

## Stage 28: M6 stage 1, Heartless attack without cards and several at once, version 0.12.0 (2026-10-31)

> "When KH1 is selected, enemies shouldnt play cards, and also, multiple enemies should be able
> to attack at the same time, exactly how it works in KH1" / crowd pace: "Each attacks as if
> alone (KH1)" (decisions 29, 30)

- Research `m6-cardless-battles.md`: Heartless attacks run through the card play `0x140198870`.
  Re:CoM serialises enemy attacks in four places: CBTL's single enemy slot, the think gate
  `0x14017D010`, one grant per frame in `0x140187600`, and think-rate scaling by enemy count
  `0x14017F380`. Checked (decomp + exe).
- **P1:** a wrapper on the card play, at KH1 in Sora's story. With HP above 0, CBTL not locked
  and an attack state 2..10, it does what message 2 does: set the flags, set the next state, no
  card and no CBTL slot. Re:CoM's state code runs the attack. When CBTL is locked (win pose,
  escape, death, duels), Re:CoM's card play refuses as usual. Marluxia 3 keeps Re:CoM's path
  while its own handler ignores message 2.
- **P2–P4:** byte patches written at each battle start (wrapper on the manager constructor
  `0x1401859B0`): the think gate always yes, every request granted, no crowd scaling. Re:CoM's
  bytes are written back at the Re:CoM setting.
- No branch or pointer lands inside the patched bytes (exe scan). 132 site byte lists re-checked.
- Not in this stage: human bosses (deck-picked attacks), the action-0xB check of 12 bosses
  (Tutorial Leon still never attacks at KH1), Sora's deck, Marluxia 3's sleight cards.
  `docs/recom-m6-battles.md`. Not seen in game.

---

## Stage 27: turned-back shots do damage, version 0.11.2 (2026-10-31)

> "it looks like when guarding a projectile, it sends it back but doesnt do any damage. make
> that do damage" (decision 28)

- Owner's 0.11.0 log (Checked): turned-back shots met their shooters for 0 damage. The shooter was
  in its card attack, and Re:CoM's hit handler skips hits then.
- Change: a turned-back shot's hit gets the card-interrupt bit `0x2000000`, as a KH1 swing does,
  so Re:CoM ends the card and applies the shot's damage. No new hook.
- Not seen in game. Next: M6 (decision 29).

---

## Stage 26: Two Become One from KH2, version 0.11.1 (2026-10-31)

> "star seeker and bond of flame are correct. two become one is the wrong keyblade, but the
> correct keyblade exists in KH2. the rest of the keyblades do not exist inside of KH2 or KH1,
> but for right now the placeholder models are fine"

- Finding: the exe's strings at `0x1405060F0` are internal names. The cards Re:CoM shows for
  kinds 0x14..0x17 are Total Eclipse, Midnight Roar, Maverick Flare and Two Become One.
- Card 0x17 now converts KH2's Two Become One (item 0x21F, stat 148, Strength +5, Magic +4,
  `W_EX010_P0`, reach x1.14) in group 42. Checked (files). 0x14..0x16 keep their KH2
  placeholders. Asset step only; the DLL is unchanged apart from its version.
- Not seen in game.

---

## Stage 25: enemy reactions to Guard, version 0.11.0 (2026-10-31)

> decision 27: "Fly back, hurt enemies" / "Flinch" / "Include them" / default "On"

- New Config row "Guard reactions" (row 6, Off / On, default On, `guard_reactions` in the ini).
  It only acts at Combat = KH1.
- Block detection in the pair-dispatch wrapper: result 2 on the player-side list
  (`*0x14087A838`), Sora's KH1 Guard live, his body `+0x9C & 0x20`, and `+0x98` = the attack's
  owner. Checked (decomp).
- **Melee:** after the attacker's hit handler, Re:CoM's reaction picker `0x14017D260(enemy, 1)`.
  If it picked the flinch (state 11), the card ends with `0x14017E1D0` and bit 0 is cleared.
  Super armour keeps Re:CoM's voice-only reaction and the card. Bosses included.
- **Shots:** Re:CoM's list setter `0x14014E5A0(coll, 0)` moves the shot to the enemies' list.
  Its velocity and aim angles are reversed, a homing shot is pointed at its shooter
  (`CBullet +0x258` with flag 0x800, `CHomingBullet +0x2F4`; the research note's `+0x32C` was
  wrong), and that test answers 0 so the bullet isn't ended. Re:CoM then moves it, collides it
  and applies its damage.
- New callees byte-checked: `0x14017D260`, `0x14017E1D0`, `0x14014E5A0`. Classes checked by RTTI
  at start.
- Assumptions: bullet models face their mover angles; the flinch knockback direction (it reads
  the last hitter); a returned shot does nothing to an enemy still in its card (Re:CoM's rule
  kept, to ask). Not seen in game.

---

## Stage 24: the six KH2 Keyblades, step 3 of the Keyblades, version 0.10.0 (2026-10-31)

> "Besides soul eater, every other keyblade should be from KH2 … pull the stats/models (not
> including keyblade abilities) from KH2" / "Measured reach, KK rest" / "perfect, everything is
> great, lets move on" (decision 25)

- New crate `kh2-formats-lite`, ported from KH2-Combat-in-KH1's `kh2-formats` (after OpenKH),
  credited in its `lib.rs`: KH2 archives (`kh1-formats` `egs` now opens `kh2_*`), `.mdlx`
  models and skeletons, `03system.bin`'s items and stats, `00objentry.bin`, `sys.bar` names.
- KH2 data (Checked, decomp + files):
  - Items: `03system.bin` `item` at `0x1087C`, 535 × 24 bytes; stat id at `+6`
    (`0x1403E3EF0`).
  - Stat rows after them, 16 bytes; `+4` / `+5` compared with a character's `+0x188` /
    `+0x18A` (`0x1402FCC80`, `0x1402FD050`). Calling them Strength / Magic follows OpenKH's
    names, and the Kingdom Key's +3 / +1.
  - Models via `went` → `00objentry.bin`: Star Seeker `W_EX010_30`, Monochrome `_60`,
    Follow the Wind `_70`, Hidden Dragon `_40`, Photon Debugger `_90`, Bond of Flame `_K0`.
- Converter: KH2's Kingdom Key mesh is Re:CoM's PL0002 within 0.077 mm on the same bones, and
  all six have PL0002's 12-bone skeleton. So the mapping is the identity (× 0.01, same axes), and
  the asset step checks it on every run. Groups 38..43 (card + 19).
- Numbers: KH2 Strength / Magic; reach = KH1 Kingdom Key's −3.7 × blade length ÷ KH2 Kingdom
  Key's 0.779 m (KH1's own Keyblades follow their models within 0.13 by the same rule, except
  Olympia); the rest the Kingdom Key's KH1 row.
- Runtime: the group rule `our_group_for_card` covers cards 2..0x11 and 0x13..0x18 (Soul Eater
  none); 110 table rows. No new hook.
- Assumptions: the facing flag mapping; list-A triangles on semi-transparent texels; Bond of
  Flame's charm blending; the HD raw route and trail length (carried over). Not seen in game.

---

## Stage 23: Guard and Counterattack in Halloween Town, version 0.9.1 (2026-10-31)

> "check the trace, trying to use guard in battle freezes sora"

- Trace 12 (Checked): Guard at frame 17, then the motion time stayed at 0 for 77 frames until
  a hit. The KH1 step ran normally (1.0). The log names world 6 (Halloween Town).
- Cause: in Halloween Town Sora uses resource group 11. `PL0011.ANN` lacked our appended clips
  `p001_225` / `p001_226`, so rows 0x47 / 0x48 resolved to no clip. Checked (files + decomp: the
  row's index from `0x1407C6300[player+0x398]`, `0x1401F7180`).
- Fix (asset step only): `PL0011.ANN` extended with the same clips at the same slots (same
  skeleton, Checked), and `PL0011.SES` with the same sounds. The DLL is unchanged apart from its
  version. `docs/kh1-recom-assets.md` § Halloween Town's pack.
- Also (owner's request, no code): `combo_master = on` in the owner's `kh1-recom.ini` for
  testing.
- Not seen in game.

---

## Stage 22: KH1 Keyblade models, step 2 of the Keyblades, version 0.9.0 (2026-10-31)

> "it all looks good continue" / swap timing: "When the menu closes" / memory: "Only the equipped
> one" / Halloween: "Equipped Keyblade everywhere" / first test: "Skip, build it all"
> (decision 25)

- Asset step (`docs/recom-keyblade-models.md`):
  - Re:CoM `.MDL` / `.RTM` readers and writers, byte-identical on Re:CoM's own files (PL0002,
    PL0011, PL0101 MDL; five RTMs). Checked (files).
  - KH1 `.wpn` reader (`kh1-formats/wpn.rs`), DDS decoding and a deterministic 256-colour
    quantiser for the TIM2 fallback.
  - The converter turns the HD `.cvbl` mesh of each of the 16 KH1 Keyblades (cards 2..0x11) into
    a Re:CoM MDL with PL0002's skeleton byte for byte (all vertices on joints 2..11) and an RTM
    whose HD `.dds` textures (`xw_ex_50x0_n_tm2.dds`, names unused by Re:CoM's 1,649 textures)
    go through Panacea's raw route.
  - New player resource groups 21..36 = card + 19, free in Re:CoM's table and archive. Checked
    (exe + files).
  - `kh1-stats.json` format 4 adds `group` and `textures`.
- Runtime (`win/models.rs`):
  - Re:CoM's PLAYER resource table is extended with the 16 groups (one checked pointer write
    at `0x1407EBF28`).
  - The weapon's model-init vtable slot `0x1405AADE0` picks the equipped group.
  - Wrappers on "queue the player's group" `0x140211AA0` and on the field camp open/close
    `0x1401A9590`. The weapon is rebuilt at camp close with Re:CoM's own constructor
    `0x140224BA0` and destructor, only when the equipped group changed.
  - Only the equipped group of ours is loaded. Re:CoM's group 2 stays loaded as Re:CoM loads
    it: 100 Acre Wood's Veggie Panic Keyblade builds from it, and the shared ANN / EET keep
    group 2's ids.
  - Halloween Town shows the equipped Keyblade at the KH1 setting.
- Assumptions: HD textures through Panacea's raw route; the group-2-first load order; the
  weapon rebuilt every room and battle; the trail length on longer Keyblades. Not seen in game.

---

## Stage 21: enemies stay hittable while they attack, version 0.8.2 (2026-10-31)

> "sora should be able to hit enemies even if they are in the middle of attacking themselves" /
> "Enemies stay hittable" / "Include bosses" (decision 26)

- Cause (Checked, decomp + disasm): every Re:CoM enemy runs its vtable `+0x438` (default
  `0x14017DA80` = `0x1401A2D70(enemy, -1)`, all hurt bodies off) when an attack state 2..10
  starts. `+0x440` (`0x14017DA90`) switches them back on at its end. Trace 11's missed swing had a
  still Shadow 0.65–0.86 m ahead for the whole hit window (Checked, trace). That the Shadow was
  mid-claw is an Assumption: the trace has no enemy state.
- KH1 (Checked, decomp + files): the Shadow's script never masks its hurt parts.
- Change: a wrapper on `0x14017DA80` (5 bytes `BA FF FF FF FF`; the tail jump checked as a
  callee) skips "bodies off" at the KH1 setting in Sora's story, for every enemy and attacker.
  It runs once per attack start.
- Also: KH1 hits survive the next swing's start; the trace's `hits` column is cleared at F9.
- Correction: Stage 9's "hits land during card plays" (decision 14) never fired in game, because
  the bodies were off. Not seen in game.

---

## Stage 20: the stance idle after Guard and Counterattack in the field, version 0.8.1 (2026-10-31)

> "when sora guards out of combat, the animation needs to smoothly go into the correct idle"
> (owner's video: the Guard ends in the battle crouch, then the field idle pops in)

- Re:CoM's idle choice `0x1401E6AC0`: the stance idle (row 0x2A, `p001_031`) in battle stance,
  or when the previous or current row is 0x24–0x25 or a swing row 0x30–0x33 (or the current row
  is 0x2A); else the field idle (row 1). Checked (decomp).
- Our KH1-only rows (Guard, Counterattack, appended after 0x47) end in the battle stance like
  Re:CoM's swings, but the rule didn't count them, so a field Guard went to the field idle.
  `rows::row_for` now counts them as swing rows. Test added.
- Not seen in game.

---

## Stage 19: Keyblades at the KH1 setting, step 1 (owning and equipping), version 0.8.0 (2026-10-31)

> "a. Besides soul eater, every other keyblade should be from KH2 … incorporated NATIVELY in this
> camp menu along with the KH1 keyblades" / unlock: "Ever earned" / Soul Eater: "Not for Sora" /
> Riku's story: "Hide" / menu: "New camp entry" (decision 25)

- Asset step: `kh1-stats.json` format 3 adds `keyblades` (the 17 KH1 Keyblades with Re:CoM
  cards, by Re:CoM card kind 1..0x11, from KH1's weapon table: radius, reach, Strength, crit,
  priority, magic) and `sora_base_strength` (without a Keyblade). Checked (files: Kingdom Key and
  Ultima Weapon rows; names matched by KH1's item order and stats, Assumption for the names).
- Runtime: the equipped Keyblade's KH1 numbers drive the blade test, the damage and Strength.
  It is one byte at `save + 0x2674` (`*(u64*)0x14087B110`), a range of Re:CoM's save block no
  code uses, which Re:CoM saves, checksums and zeroes itself (`recom-cards-save.md`). 0 = the
  Kingdom Key. Checked (decomp + exe scan); the log checks the range on real saves.
- Camp menu: an 8th entry "Keyblades" (KH1 setting, Sora's story, below Settings), through
  wrappers on the camp update `0x1400D9C80`, the entry build `0x1400E3DE0`, the cursor set
  `0x1400DEA10` and the teardown `0x1400D7C20` (all-or-nothing). Its page runs in Re:CoM's Config
  states 0x4A..0x4C, built from Re:CoM's classes after the Status page `0x1400EB7E0`. Unlocked =
  Re:CoM's "ever obtained" state `0x14015B3F0` (normal or premium); names from Re:CoM's card
  names (`0x140164CA0`). Details and addresses: `docs/recom-keyblade-equip.md`.
- Owner's screenshot: the 8th entry has room at Re:CoM's spacing (y ≈ 507 of 720).
- Assumptions: object ownership on delete; the equipped mark (Re:CoM's row marker 0x1D); the
  header title; the spare range on real saves. Not seen in game.

---

## Stage 18: KH1's lunge distance uses the enemy's body radius, version 0.7.5 (2026-10-31)

> "a then b then c" (owner: (a) the lunge distance, (b) KH1 hurtboxes on twins, (c) the
> equipped Keyblade's size)

- Finding (`hitbox-compare.md`): KH1's ground picker (`0x1402A7320`) subtracts each kind-0 lock
  point's radius (`0x1402B04C0`: point `+0xC` × actor `+0x40`) before its 180 / 300 thresholds.
  The mod used 0, so at 3.00–3.30 m from a Shadow it swung in place and missed where KH1 lunges.
  Checked (decomp).
- Asset step: `kh1-stats.json` format 2 adds each twin's lock radius from its `.mdls` part 4
  (`CLS\x03` lock points, kind = byte 0 & 0xF). Checked (files): all 31 twins have one;
  Shadow 30, Soldier 35, Large Body 100. The Screwdiver has four (170, 40, 40, 40); the first is
  used (Assumption).
- Runtime: Re:CoM's attack target becomes KH1's part with that radius (`win/attack.rs`
  `attack_target`). Data only; no new hook.
- Assumption: the twin's actor scale is 1.0. Not seen in game.

---

## Stage 17: the Keyblade appears on Guard, version 0.7.4 (2026-10-31)

> "when sora doesnt have his keyblade out and does guard, the keyblade should appear"

- The Guard start calls Re:CoM's own "draw the Keyblade", `0x1401F5D30(Sora, 1)` (Sora's
  vtable `+0xF8`, as the swing init `0x1401F0F70` calls it). It plays `kb_appear` only when the
  blade was put away. Re:CoM's own rules put it away again. New byte-checked callee. Checked
  (exe + decomp).
- Owner's trace 10 (0.7.2): 2 of 4 Guards blocked. The 2 that didn't were hit on Guard frame 4
  (startup) and frame 55 (after the motion), outside KH1's live frames 10–36. Checked (trace +
  files). The window stays KH1's.
- Owner: Sora is at max level, so the 300-damage hits (KH1's cap, Strength 51) are right.
- Assumption: KH1's Sora always holds the Keyblade. Not built; to ask.
- Not seen in game.

---

## Stage 16: KH1 model writer, the flip test's step 1, version 0.7.3 (2026-10-30)

> "would it not be easier to just bring all the maps and unique enemies/bosses into KH1 as a mod
> for KH1?" / option (b): "b" (owner, twice: scope it, then run the decisive test)

- Notes: `docs/flip-feasibility.md` (the scope) and `docs/kh1-model-writer.md` (this stage).
- `kh1-formats`:
  - `mobj.rs`: whole `.mdls` files read and written (container, `MOBJ`, both models, PS2
    packets, weighted vertices, textures). Every size, count and offset is rebuilt.
  - `cvbl.rs`: the PC's HD meshes (`.cvbl`) read and written.
  - `egs.rs`: `decode_entry_full` (an entry with its HD assets), `build_plain_entry` (a plain
    entry for Panacea's `<mod_path>/kh1/raw/` route), `Archive::read_raw_entry`,
    `Install::read_full`; `KeyTable::decrypt_head` is public.
- `kh1-recom-assets`: `flip.rs` and the CLI `kh1-flip-test` (`roundtrip`, `install rebuild |
  big | remove`).
- Results: byte-exact on the owner's install, 500 of 500 `.mdls` and 941 of 941 `.cvbl`, and
  every rebuilt entry decodes back (run on the owner's PC through the device link). Checked
  (files). The Shadow's `rebuild` and `big` (×1.5 positions) entries are on the owner's PC.
- Facts found: the PC draws the `.cvbl` HD mesh, not the PS2 packets; HD textures are tagged by
  `.mdls` texture offsets; the block after `MOBJ` holds `_KN5` / `BONE` / `URSL` data; offsets
  past the end of a `.mdls` point at the end rounded up to 0x80. Checked (files + source).
- Assumptions: Panacea's raw route works for KH1 models; the 12 extra inflated bytes of five
  Halloween Town `-1c2.dds` are never read.
- No change to the DLL or to Re:CoM. Not seen in game: `testers/kh1-flip-model/thread.md`.
- Windows type-check not run locally (the Windows target can't be downloaded in this session);
  the DLL doesn't use the changed crates, and the build bot builds and tests the workspace.

---

## Stage 15: sounds for Guard, its success and the Counterattack, version 0.7.2 (2026-10-30)

> "guard and counterattack work on soras end, lets try to get their sounds working go ahead" /
> sound source: "Re:CoM's sounds now"

- Asset step:
  - `PL0001.SES` is Re:CoM's sound file plus entries for `p001_225` (Guard: whoosh 1236 and
    swing grunts at 0) and `p001_226` (Counterattack: whoosh 1237 and finisher grunts at 0,
    steps at 46 and 80).
  - The KH1 frames are from `xa_ex_0010.mset`. SES frames are 60 Hz motion frames
    (`0x1401A47C0`, 50 ticks a frame).
  - Re:CoM's 50 entries are byte for byte. Checked (files + decomp).
- Runtime: at a guard success, Re:CoM's own guard spark and clink, `0x1401A6B20(Sora,
  "always", 0xB, hit point, 0, 0.0f)` (as `0x140182260` does for enemies). The hit point is
  `player+0x6C0`, kept by the damage-check wrapper. New byte-checked callee `0x1401A6B20`; the
  name pointer `0x1407C2C40` is checked against `"always"` at run time. Checked (exe + decomp).
- Assumptions:
  - The Guard whoosh and voice are the nearest Re:CoM sounds (KH1's weapon + 0xD and clip 24
    have no known Re:CoM match).
  - The id of `always` #0xB's clink is unknown.
  - Re:CoM fades motion sounds on the next motion; KH1 appears not to.
- Also: the replay tool's ability map runs without Counterattack (its bit isn't mapped). A
  stale doc line about the world address was fixed.
- Not seen in game.

---

## Stage 14: KH1 Guard blocks, its recoil, and the Counterattack, version 0.7.1 (2026-10-30)

> "Go ahead and" (owner, after "I'll start on Guard"); decision 20 (KH1 for Sora's side; new
> Re:CoM enemy reactions behind a toggle, next version)

- KH1's rules (`docs/recom-kh1-attack.md` § Guard): guard object live on frames 10–36; kind-6
  success `0x14029D720`; Counterattack offered from frame 12 of request 0x6E (`0x140294220`,
  `0x140293480`), row 0x3E. Checked (decomp + files).
- Re:CoM side: Sora's body guard `player+0x72C` bit 0 and `+0x740` = 100° while live; the
  guarded bit `+0x72C & 0x20` read in the damage-check wrapper; flag 0x80 as `+0x418 & 1`; the
  recoil plays Re:CoM's row 0x24 (`p001_081`, KH1 entry 72's bake); the Counterattack plays the
  KH1-only clip `p001_226` through a second appended action row (`rows::KH1_ONLY`) and a KH1 row
  0x3E in the attack table. Checked (decomp).
- kh1-core: motion 0x6E (entry 72: 60 frames, free 28–60, window 28–60), the Counterattack
  ability bit, a `GuardSucceeded` event. The oracle's controller has no clash yet; the handler
  tests check KH1's numbers directly.
- ini: `counterattack = on` (default).
- Not seen in game. Enemy reactions (decision 20's toggle) are the next version.

---

## Stage 13: KH1 damage and HP on Re:CoM's enemies, Sora's KH1 Strength, version 0.7.0 (2026-10-30)

> "Yes do that order, and have it follow the Rod" / Strength path: "Rod, give up Shield" /
> enemy stats: "Same world, LV1"

- **Strength** (decision 18): new game btltbl `0x39D8` (Sora Strength 4), Dive deltas
  `0x128 + 4 × choice` (keep Rod, give up Shield: +0), level bonuses `0x3AC0` (bonus 4 =
  Strength +2, `0x1402B51B0` → `0x1402B5660`), Kingdom Key +3 (`0x1402A5130`). 7 at level 1 … 51
  at 99. Checked (decomp + files).
- **Enemy records** (decisions 12, 21): KH1's AI syscall 45 records, read by the asset step
  from each twin's `.mdls` part 5 at the script offsets found by evaluating the scripts (per
  world for the Shadow, LV1 / LV2 for the rest). 31 Re:CoM classes. Checked (decomp + files).
  Pairings by internal name only: Assumption, tagged in the file.
- **Runtime** (`win/stats.rs`, new hooks `0x14017FF50` enemy stats init and `0x14017F5A0` max HP;
  callees `0x1402339C0` world, `0x1401AAAE0` Riku's story; all byte-checked): KH1 HP on twins,
  KH1 damage written as the hit part's value `+0x92` in the enemy hit handler, then Re:CoM's own
  weak-part / resistance / falloff. Sora's level from `*0x14087CBF8 + 0x1C`. Riku's story and
  classes without a twin keep Re:CoM's numbers. Checked (decomp); not seen in game.
- **Swing aim**: KH1 keeps the yaw taken at the press (`0x1402A7510` → `0x1402A38E0`). The mod
  already does this. Checked (decomp).
- Decisions 19–21 recorded. Guard (decision 20) and M6 are the next versions.
- Assumption: Re:CoM's damage path adds nothing else to the value (`0x14015D5F0` can rewrite
  AP; not read). The log names each class's KH1 HP / Defence per world the first time.

---

## Stage 12: Re:CoM's ledge grab from a KH1 jump, version 0.6.3 (2026-10-30)

> "sora is also unable to ledge grab with KH1 selected"

- Addresses: the jump exe `0x1401EDA60` (hooked since Stage 8). Its ledge tail is at
  `0x1401EE0E6..0x1401EE109`: the wall climb check `0x1401E8C00(pl, 1)`, the climb init
  `0x1401F2C20(pl, n)` when the result is above 1, the hang check `0x1401E8420(pl)`, and an
  inline hang block. That block makes the same writes and calls as `0x1401F26A0` (action 9,
  row 0xF). Checked (disasm + decomp).
- Cause: at KH1 setting our hook ran the KH1 air frame in place of Re:CoM's jump exe, so the
  ledge tail never ran (decision 16 keeps Re:CoM's ledges). Checked (decomp).
- Change: after the KH1 air frame, the hook calls those Re:CoM functions under Re:CoM's
  conditions: action 7, rising or falling, and not landing this frame. Re:CoM's hang, climb
  and drop then run as its own actions 9 / 0x10. Four new callee checks with the Steam bytes
  (`sites::MOVE_CALLEES`). The ground ledge checks already ran, because the original move exe
  runs on the ground. Checked (decomp).
- Assumption: a jump or drop from the hang goes through our jump init and continues as a KH1
  jump or fall. Not seen in game yet. The log says "movement: Re:CoM's ledge hang/climb started
  from a KH1 jump" the first time each happens.
- Also, from the owner's 0.6.1 traces 7–8 (no code change): the position jump is fixed. The
  "missing swings" are KH1's own press rules: presses before the combo window, and presses
  after a swing that hit nothing. Details in `docs/recom-kh1-attack.md` § In game, 0.6.1.
  Checked (trace + decomp).

---

## Stage 11: mod version on the Combat row's help line, version 0.6.2 (2026-10-30)

> "also make it so hovering over the recom/kh1 config option shows the mod version as well"

- Address: the help line `camp+0x9D0`, set with `0x140259000` by our Config draw hook
  (`0x1400DF1B0`), unchanged since Stage 6. Checked (decomp).
- Change: the Combat row's help text now ends with "(mod v<version>)". The version comes from
  `crates/kh1-recom/Cargo.toml` at build time, so it is the same as the log's first line. The
  text stays static, NUL-terminated ASCII with no `%`, so `0x140259000` keeps the pointer as
  before. A unit test checks the suffix. Checked (test).
- Assumption: the longer line (82 characters) fits the help box without being cut off. Not
  seen in game yet.
- Also in this build: the 0.6.1 swing / guard / roll fix (Stage 10). The owner's 0.6.0
  retest (traces 3-6) ran 0.6.0, because the DLL installed on the PC was copied before the
  build bot had built 0.6.1. Those traces show the Stage 10 fault in all 9 moves (attack:
  action 0xA; guard: action 6 sub 1; roll: action 6 sub 0), and the Stage 10 hook covers
  actions 6 and 0xA. Checked (trace).

---

## Stage 10: fix the swing / roll position jump, version 0.6.1 (2026-10-30)

> "sora cant do his ground combo, it either makes him jump or teleport or just jitter
> uncontrollably. this also happens in the field" / "something similar happens with dodge roll as
> well"

- Owner's traces (`testers/recom-attack/owner-0.6.0-attack-*.csv`): on the Attack press, Sora's
  position jumped from the floor at (−416, −750, 1364) to (10.6, −597, −34.7), 153 units below
  the floor. He then free-fell until Re:CoM's out-of-bounds handler respawned him; a press while
  falling gave air row 18. Checked (trace).
- Root cause: the root-motion hook at `0x14024DCA0`. That function returns an absolute position
  (`ctrl+0xF0` + this update's rotated travel, via `0x14024C960`), and the entity task sets
  `P + mask∘(out − P)`. Our one-update delay treated the return value as a travel, so the first
  update of a swing or roll returned (0, 0, 0). Checked (decomp + trace).
- Fix: the hook delays only `out − anchor` and returns `anchor + last travel`. If ours differs
  from Re:CoM's by more than 0.5 m, Re:CoM's stands. It passes through when the frame counter isn't
  running.
- Regression test: swing and roll away from the origin keep Sora's position; travel = KH1's 100.
- New ini key `debug_attack = on` logs the first 200 attack decisions.
- Not yet seen in game.

---

## Stage 9: KH1 Attack inside Re:CoM (M5), spring fix, version 0.6.0 (2026-10-30)

> M5 per the roadmap, plus the owner's report on 0.5.0: "sora needs to be able to jump as high as
> he is supposed to on these springs"

- **Springs:**
  - Re:CoM's jump init launches springs: gimmick `0x2002`, `0x1401E80C0(pl, 0x2002) == 7`,
    ability `+0x41C & 0x20000`, speed from `0x1405A0100` (39–75.8 against a normal 28),
    `+0x418 |= 0x1000000`, row 0x14.
  - Every KH1 ground jump now runs Re:CoM's spring test first. On a spring, Re:CoM's own jump runs
    until landing, then KH1's rules resume. Checked (decomp); not yet seen in game.
- **KH1 Attack** at `combat = kh1` (`docs/recom-kh1-attack.md`):
  - In battle, the confirm press takes Re:CoM's field (cardless) branch, and the use-card button
    is skipped.
  - initAttack becomes the field-swing init. kh1-core's picker chooses the row in Sora's vtable
    slot `+0x128` (`0x1405A9A60`).
  - Our wrapper on the field swing exe runs KH1's combo window, chain, facing and air arc, then
    Re:CoM's exe.
  - Battle jump-table entry `0x1401FA074` points action 0xA at a 13-byte stub in the int3 gap at
    `0x1401FA6C1` (15 free bytes, lead-checked).
  - KH1 rows in Re:CoM's layout reuse Re:CoM's clips 1–7, with KH1's hit and combo windows (flag
    0x80) and no 20-frame lockout.
  - KH1's two-sphere blade plus sweep (radius 0.40 m), tested with Re:CoM's own primitives.
  - Card priority: a KH1 hit interrupts the enemy's card through Re:CoM's `0x2000001` path.
  - Reactions: swings flinch (kind 1), finishers knock away (kind 2).
  - ini: `combo_plus`, `air_combo_plus`, `combo_master`.
  - Hooks also on `0x1401F8150`, `0x1401E8FF0`, `0x1401F0D90`, `0x1401F0F70`, `0x1401EAEB0`,
    `0x14016EAD0`, `0x14017F7E0`, `0x1401E6DE0` and `0x14024DCA0`. 54 sites byte-checked.
- **M4 follow-ups:**
  - High Jump plays Re:CoM's rows 0x14/0x15.
  - Root motion is one update late for Sora (the roll is now exact against kh1-core).
  - **Finding:** KH1's roll has no untouchable frames. The `0x2B`/`0x2A` ops are only queued by
    event scripts, so roll protection = 12 no-flinch frames, damage still taken. This corrects
    KH2-Combat-in-KH1's `kh1-intangible.md`. Checked (decomp).
- **Damage:**
  - Still Re:CoM's (logged FALLBACK). KH1 enemy stats come from 0x6C-byte records passed by an AI
    script call (syscall 45, `0x1402BBDF0` → `0x1402A58E0`). Strength/Defence index a 21-row table
    at btltbl `0xC8A8`. Checked (decomp + files).
  - The per-enemy records are in KH1's enemy script files, not read yet.
- **Not done:** Guard repel / Counterattack (Re:CoM's guard fields at `player+0x690` found, not
  wired); MP charge (needs the enemy's KH1 level).
- 1:1, Checked (tests vs kh1-core): ground 13→11→8, missed-swing refusal, air 18→17→16, landing
  cutting an air swing, jump beating Attack, the roll.
- Assumptions: hit window one frame earlier (Re:CoM's hit test runs after the animation update);
  the target aim point stands in for KH1's body part. Nothing seen in game yet; tester thread
  `testers/recom-attack/thread.md`.

---

## Stage 8: KH1 movement inside Re:CoM's player actions (M4), version 0.5.0 (2026-10-30)

> "looks good enough for now, lets work on phase 2"

- At `combat = kh1`, for the main `CPlayerSora` only (Riku untouched), Re:CoM's own movement
  actions run KH1's rules and numbers (`docs/recom-kh1-movement.md`):
  - actions 2/3/4 are ground movement, 6 Dodge Roll / Guard, 7 take-off / air / landing, 8 glide;
  - Re:CoM still plays the animations, applies root motion, moves and collides Sora.
- Hooks, all byte-checked and all-or-nothing (39 site byte lists re-checked against the exe):
  - move exe `0x1401EEDA0`, stick class `0x1401E9630`, move step `0x1401EF3C0`;
  - jump init `0x1401F29F0`, avoid init `0x1401F10C0`, jump exe `0x1401EDA60`;
  - Sora avoid exe `0x140217310`, Sora glide exe `0x140217B50`;
  - trace-only hooks on Field / Battle exes (one flag test while not recording).
- Units, Checked (exe + decomp):
  - KH1 (x, y, z) cm, y down = Re:CoM (x, −y, −z)/100 m, y up;
  - KH1 yaw ψ = Re:CoM π − ψ;
  - Re:CoM's `+0x464` = dt × 0.02 (60 Hz frames);
  - Re:CoM's jump peaks at 1.92 m at frame 28, KH1's at 190 units at frame 28.
- Animations follow decision 11: Re:CoM's rows, plus three appended rows for the KH1-only High Jump,
  its apex, and Guard (from the asset manifest).
- Abilities in `kh1-recom.ini` (`high_jump`, `glide`, `superglide`, `dodge_roll`, `guard`).
- F9 Re:CoM trace in the kh1-trace schema.
- 1:1, Checked (tests) against kh1-core frame for frame. Known: the Dodge Roll moves one frame
  early (same distance and end point), and follow-on animations start one frame late (the lock
  releases on KH1's frame).
- Owner decisions recorded this stage (design doc 12–15): KH1 damage formula with KH1-twin stats;
  keep Re:CoM's hit extras; KH1 card priority (hits land); enemy cards hidden.
- Not checked in game. Tester thread `testers/recom-movement/thread.md`.

---

## Stage 7: Config window border fits the Combat row, version 0.4.2 (2026-10-30)

> Owner's 0.4.1 screenshots: the fill reached below "Combat" but the border stopped at "Game
> Mode". The help line on Combat was right.

- Re:CoM's 9-slice (`0x14024C270` / `0x14024AD40`) only handles sprites with exactly 9 parts. The
  panel's border and highlight are 8-part sprites, so the earlier bottom offset never reached
  them. Checked (disasm).
- Fix: at the same open/close points, nothing per frame:
  - each border part below the sprite's middle gets its own copy of its shared sprite-set rect,
    moved down 26 units;
  - this moves the bottom corners and edge and stretches the sides;
  - Re:CoM re-reads the rect each update (`0x14024B380`), so it then draws the frame itself.
  - The shared data isn't changed.
- In game (owner's 0.4.1 screenshot): the Combat row works: cursor, highlight, and the help text
  fits on one line. The border fix hasn't been seen yet.

---

## Stage 6: owner's animation rule, Config panel fit, version 0.4.1 (2026-10-30)

> "i wouldn't bother having an option to switch between KH1 and RECOM animations because for all
> intents and purposes they're the same, just use RECOM's default animations, and KH1 animations
> for anything that isnt included in ReCOM"

- Asset step (`kh1-recom-assets`, new `matching.rs`): each KH1 entry is matched against Re:CoM's
  clips at their key frames on all 135 bones.
  - 39 are bakes: every bone ≤0.9° and ≤1 mm; worst 0.74°, the four known pairs <0.02°.
  - 10 are Re:CoM edits: ≤20 bones differ, no other owner.
  - The closest wrong pair is 17 bones; the closest clip for a KH1-only entry is 26 bones.
  - Matches all 46 of `clip_origin`'s pairs, plus 0→p001_001, 65→p001_043, 78→p001_157.
    Checked (files, tests).
  - Only the 69 KH1-only entries get new slots (161–229). `PL0001.ANN` is 10.0 MB and `PL0002.ANN`
    0.84 MB. `kh1-motions.json` maps all 118 KH1 entries to the Re:CoM slot that plays them.
- DLL:
  - The "KH1 motions test" row, its ini key and the action-table row swap are removed. An old ini
    line is ignored.
  - Config panel: in the existing draw hook, Re:CoM's own 9-slice (`C2DSprObj`, vtable
    `0x1406077D8`, corner offsets `+0xA0..+0xAC`, flag 8 at `+0x144`, applied in `0x14024AD40`) is
    given one extra row of bottom offset. Checked (decomp); that the panel art is a 9-part sprite is
    an Assumption, and the log reports it.
- In game (owner's screenshot of 0.4.0): our rows render with Re:CoM's own buttons, font and value
  highlights. The panel was one frame short, which is what this stage fixes. Not yet seen in game.

---

## Stage 5: Phase 1 in Re:CoM: KH1 motions asset step, motion switch, Config rows, version 0.4.0 (2026-10-30)

> "lets finish phase 1" (ROADMAP M2 + M3)

- **Asset step** (`crates/kh1-recom-assets`, CLI `kh1-recom-build`; `docs/kh1-recom-assets.md`):
  - Converts all 118 KH1 Sora motions (via `kh1-anim`) into new 60 fps slots 161–278 of Re:CoM's
    `PLAYER/0001/PL0001.ANN`, and the matching Keyblade clips into `PLAYER/0002/PL0002.ANN`.
    Re:CoM's own slots stay byte-identical.
  - Writes `<mod_path>\Recom\…` plus `kh1-motions.json`. Panacea serves files from
    mod_path + "/recom" by archive path. Checked (OpenKH Panacea source).
  - Validation: Re:CoM pack decode → encode is byte-identical; converted clips match KH1's pose
    ≤0.045 mm / 0.005° per frame. Against Square Enix's own bakes they are ≤0.019° (body) and
    ≤0.007° (Keyblade, entry 98). Checked (files, tests).
  - Known: the Keyblade tip `k9` is off by 1.58° in entry 4 (KH1 bone 283's copy-orientation
    constraint in `kh1-anim`), and by 0.18° in entry 74.
  - Loader: no 161-slot limit; sizes come from the file (`0x14029F3A0`), and the name lookup
    (`0x1401A47E0` / `0x1402A42B0`) handles slots ≥161. Checked (decomp).
- **DLL** (`crates/kh1-recom`; `docs/recom-config-menu.md`, `docs/recom-motion-switch.md`):
  - Hook infrastructure: byte-checked, all-or-nothing. Starts from Panacea's `OnInit`.
  - Config: two rows, "Combat: Re:CoM / KH1" and "KH1 motions test: Off / On", added to the camp
    menu's Config screen using Re:CoM's own item, layout, cursor and help code. Hooks: Config input
    `0x1400D3EB0` (`48 89 5C 24 08 57 48 83 EC 20`) and draw `0x1400DF1B0`
    (`48 8B C4 55 41 54 41 55 41 56`). Bytes re-checked by the lead against the exe.
  - The values are saved in `kh1-recom.ini`: the Config save has no byte that can be proven free.
  - `config_menu = off` is a safety switch.
  - The KH1 motions test repoints 12 name pointers in Sora's action table (`0x1407C5E80`): idle,
    walk, run, swings 1–2, lunge, finisher. Re:CoM resolves and plays them itself from the next
    room load. Data only, nothing per frame.
- Not checked in game: the rows' look and behaviour, the clips playing, the Keyblade following.
  Tester thread `testers/recom-phase1/thread.md`.

## Stage 4: KH1 trace logger, replay and diff (M1), version 0.3.0 (2026-10-30)

> "lets finish phase 1" (ROADMAP M1)

- `crates/kh1-trace` (`docs/kh1-trace.md`):
  - `kh1-trace.dll` for KH1 FM Steam. Build check: PE time stamp `0x669E37CE`, size
    `0x2F91000`.
  - One hook on the per-actor task `0x140297FE0` (`48 8B C4 57 48 83 EC 60`). F9 records
    `kh1-trace-<n>.csv` with about 70 columns: state, motion, position, velocities, combo, and the
    inputs.
  - `kh1-tracediff` compares two traces.
- Playground: `--replay <csv>` feeds the recorded inputs to the controller and diffs the result.
  Replay is deterministic byte for byte (tests).
- Finding: KH1's "ground contact" is +0x2D0 bit 0, not bit 1. Checked (decomp).
- Assumptions: the camera yaw formula (checked against KH1's own move vector on real traces); Sora
  is party slot 0; Panacea calls `OnInit` before frames run.
- Not run in KH1 yet. Tester thread `testers/kh1-traces/thread.md`.

---

## Stage 3: smoother Sora in the playground, PS5 controllers, version 0.2.1 (2026-10-30)

> "lets make the actual character model looking smoother, and make sure ps5 controllers work with
> it" (owner chose a smoother mannequin, not the real mesh)

- `kh1-playground/src/body.rs` replaces the old one-capsule-per-bone figure:
  - smooth tapered limbs (two spheres joined by a cone, with smooth normals) and rounded joints;
  - the pelvis, chest, head and shoes are ellipsoids oriented by the surrounding bones;
  - Sora's 14 real hair spikes, jacket tails, hood and fingers follow their own bones;
  - MSAA ×4, plus a fill light.
- The KH1 bone roles come from matching KH1's skeleton to Re:CoM's named one, e.g. 17/18/20/23 for
  the right leg and 93/94/97/100 for the right arm. Checked (files).
- **Finding:** KH1 doesn't pose the end bones of chains (flags `0x6000000`: hair tips, jacket
  tails, hood). The figure places them from the parent's world matrix and the end bone's rest
  translation. Checked (files: idle pose dump).
- Gamepads:
  - Bevy's gilrs uses Windows.Gaming.Input with SDL's controller database, which includes the
    DualSense `054C:0CE6` and the Edge `0DF2` over USB and Bluetooth (gilrs builds USB-style
    GUIDs). Checked (crate source; the mapping string is in the built exe).
  - The HUD shows each pad's name, ids, stick and held buttons, with PS labels.
  - Not tested with a real pad.
- No change to the rewrite crates or the Re:CoM DLL, apart from the version number.

---

## Stage 2: KH1 battle system Rust rewrite + playground demo, version 0.2.0 (2026-10-30)

> "Lets try to do a full 1:1 Rust Rewrite of the KH1 Battle System, similar to how we did the KH2
> battle system rewrite, with a demo i can test" (owner: reference + demo; first demo movement +
> Attack)

- New crates (`docs/kh1-rewrite.md`):
  - `kh1-formats`: reads PC HED/PKG and KH1's `.mset`, `.mdls` and `btltbl.bin`.
  - `kh1-anim`: KH1's posing pass (curves, start pose, constraints, two-bone IK, frozen-snapshot
    blends).
  - `kh1-core`: the 60 Hz controller and combat.
  - `kh1-data`: install discovery and `SoraKit`.
  - `kh1-playground`: Bevy demo.
- Data is read at runtime from the player's `kh1_first.pkg` (`xa_ex_0010.mset/.mdls`,
  `btltbl.bin`). Checked (files: decoding gives the extracted files' SHA-256).
- `kh1-anim` matches Square Enix's own Re:CoM bakes of the same KH1 motions to ≤0.019° local
  rotation and ≤0.051 mm world position on swing / roll / run / jump. Checked (files, tests).
- `kh1-core` corrections to the notes (cone 22°, air row 17 above, combo 13→14→8 with a close
  target, …) are in `knowledge/KH1/kh1-sora.md`. Checked (decomp).
- Assumptions are marked in the code and the HUD: root travel subtraction, MSVC `rand`, task order,
  dummy stats (Strength 5, Defence 2, 300 HP), and the camera (an orbit camera, not KH1's).
- Headless check: a scripted combo runs rows 13 → 14 → 8 and hits the close dummy for 3, 3 and 7.
- The `kh1-recom` DLL is unchanged apart from its version (0.2.0); it still only checks the game.
- 70 tests (all pass with `KH1_TEST_DATA`); clippy clean.
- Not checked against a KH1 trace yet. Gamepad untested.

---

## Stage 1: the mod's skeleton DLL, version 0.1.0 (2026-10-30)

> "B, and use whichever is least effort and is most native to RECom" (owner: Panacea loads the
> mod; at KH1 setting Sora plays KH1's motions re-baked at 60 fps)

- New crate `crates/kh1-recom`: a cdylib that OpenKH's Panacea loads (`openkh/mod.yml`, copied as
  `dll/kh1-recom.dll`). On load it starts one thread and then nothing runs per frame:
  - checks the exe is the Steam build: PE time stamp `0x669E38DF`, size of image `0xD4A000`.
    Checked (exe).
  - checks the first 16 bytes of each future hook site. Checked (exe bytes = decomp function
    starts):
    - initAttack `0x1401F0D90`
    - field swing init `0x1401F0F70`
    - player motion request `0x1401F7180`
    - attack row pick `0x1401F7520`
    - Battle exe `0x1401F9840`
    - Field exe `0x1401FA0F0`
    - use-card button `0x1401E8FF0`
    - `.ANN` loader `0x14029F3A0`
  - reads `kh1-recom.ini` (`combat = kh1 | recom`, default `kh1`), writing it if missing. Nothing
    reads the setting yet. The Config-menu toggle is still to do.
  - writes `kh1-recom.log` next to the game.
- **No hooks installed and nothing patched.** The game plays exactly as Re:CoM.
- Panacea knows Re:CoM: its `DBGHELP.dll` names the Re:CoM exe, uses a `/recom` mod subfolder, and
  hooks Re:CoM's `Axa::CFileMan`. Checked (files: strings in the owner's Panacea DLL). Whether it
  loads `dll/*.dll` for Re:CoM as it does for KH1: Assumption, to check with the first launch.
- Build: MinGW cross-compile (`tools/build-win.sh`); build bot `.github/workflows/build.yml`
  commits `openkh/kh1-recom.dll` on `testing`. 7 unit tests.
- Not checked in game yet: that Panacea loads the DLL, and the log on the owner's machine.

---

## Stage 0: groundwork (2026-10-29)

> "Where I'd start: a cardless Attack, built into Re:CoM's own battle code. Before that comes a
> short groundwork step (step 0)."

No code. `VERSION` not created yet; the first change under `crates/` starts it.

- Repo seeded: these rules files, `docs/sources.md` (now `knowledge/sources.md`).
- Re:CoM Steam decomp made (see `knowledge/sources.md`). Checked (exe): SHA-256 `acf42e96…6bbf`.
- Finding: Re:CoM's exe is mixed-mode C++/CLI; the .NET part is only the shell. Checked (exe).
- Finding: the KH1 notes in KH2-Combat-in-KH1 use Epic addresses; convert to Steam before use.
  Checked (files).

Step 0 mapping (2026-10-29, notes only):

- `knowledge/ReCoM/recom-timing.md`: both games step by a variable delta in 60 Hz frames. Re:CoM uses 50
  units per frame, so KH1 frames × 50 = Re:CoM time. Checked (decomp + exe, both games).
- `knowledge/ReCoM/recom-player.md`: player state machine (table-driven, 4 states: Field / Battle / Wait /
  Hide), the action switch, and the card path from button to `initAttack`. Checked (decomp).
- `knowledge/ReCoM/recom-combat.md`: motion request and advance, Sora's attack-row table `0x1407C6C70`, the
  hit path and the damage formula. Checked (decomp; rows read from the exe).
- `knowledge/ReCoM/recom-battle-entry.md`: field-to-battle handover (separate battle room) and the native
  cardless field swing (action `0xA`, card id 0) that shares battle collision. Checked (decomp).
- Assumption, well supported: Re:CoM's motion system is not KH2's mset system.

Step 0 still to do:

1. Plan the trace tool with the tester: the same fields from both games (tick, state, motion,
   frame, position, velocity, hit events).
2. KH1's side mapped in `knowledge/KH1/kh1-sora.md` (states, movement, jump, glide, roll, guard, motion
   requests, the Attack combo). Still needed: the numbers in KH1's data files (`btltbl.bin`,
   `xa_ex_0010.mset`) and Re:CoM's (`stockskilllist.bin`, `PL0001.ANN`), from the player's
   extracted files.
3. Open, not read yet: the damage veto `0x140180920`, the AP rewrite `0x14015D5F0`, the
   double-hit spawn `0x140158DD0`, and which hit kind the field swing produces on map enemies.
