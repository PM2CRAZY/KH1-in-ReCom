# Design philosophy

A Rust mod (a DLL) that brings Kingdom Hearts 1's combat (Final Mix, PC, Steam build) into
Kingdom Hearts Re:Chain of Memories (PC, Steam build).

Every setting has a Config-menu toggle between **Re:CoM** and **KH1**.

- With every setting on KH1, the game must feel 1:1 with KH1.
- With every setting on Re:CoM, it must play exactly like Re:CoM.

These rules override any habit of the assistant.

## Fork branch `Re-Re-Reverse/Rebirth`: the fork owner's intent (2026-11-23)

Fork owner, 2026-11-23, verbatim: "minha intenção com este mod que fique bem claro não é refazer o
KH1 de 1:1 no Recom mas sim reimplementar o Combate, mantendo a originalidade do Kingdom Hearts
Recom".

On this branch that intent leads, ahead of § 1:1 target below:

- The goal is Re:CoM with a reimplemented, cardless combat, not KH1 rebuilt inside Re:CoM. KH1 is
  the main reference for how the combat works (its numbers stay where they serve), not a target to
  match frame for frame.
- Re:CoM's own identity is kept and preferred: its characters' resources and rules (Riku's DP and
  Dark Mode, his sleights, Re:CoM's sleight ideas), its look, its progression. When KH1 1:1 and
  Re:CoM's originality pull apart, propose the Re:CoM-faithful option first and ask.
- Systems already built 1:1 stay as they are unless the fork owner asks for a change.
- The author's rules for `testing` (`PM2CRAZY/KH1-in-ReCom`) are unchanged: this section is the fork's.

### Every KH1 feature follows Re:CoM's own game rules, or adapts them (2026-11-27)

Fork owner, 2026-11-27, verbatim: "coloca no mod já, regras que reforçam todas as novas features e implementações
precisarem seguir ou adaptar as regras normais do Jogo quando implementar a jogabilidade do KH1!". Asked after the King's
sleights (0.150.0) and Riku's own (0.147.0) were found listed from a new game, against Re:CoM's rule.

This binds every new feature and every change that brings KH1 gameplay in (spells, sleights, abilities, items, menus,
HUD, party, unlocks, shops, story events), in both stories:

1. **Find the rule first.** Before building, find how Re:CoM itself governs the nearest thing: when it is learned or
   unlocked (story script, battle, level, card, room), where it can be used (battle / field, worlds, floors), what it
   costs, how it is shown (hidden, `?????`, greyed), what ends it. Cite it (an address, a script, a `knowledge/` note)
   in the Stage. No rule found: say so and tag the choice `Assumption`.
2. **Follow it, or adapt it.** The KH1 feature runs under that rule (Re:CoM's own tests and flags, read from the game,
   never a copy). When KH1 has its own rule for the same thing, keep Re:CoM's progression and timing and fit KH1's
   piece into it (as KH1 spells unlock through Riku's floors and bosses, Stage 399). An adaptation is written in the Stage and shown to
   the owner.
3. **No shortcuts past the game.** Never "counts as learned / unlocked / available" by default, never a flag forced on
   to make a feature reachable, never a free unlock for testing left in a build. Testing aids go behind a setting that
   is off by default and named as such (`all_unlock`).
4. **New saves and old saves.** Check what a new game shows (0:00) and what a mid-story save shows; both must match
   what Re:CoM would show at that point.
5. **When unsure, ask** the owner before building (§ Known open design questions); do not guess the rule.

### Riku's combat: KH1's system, fully adapted to Riku's own gameplay (2026-11-27)

Fork owner, 2026-11-27, verbatim, on 0.198.22 in game: "as animações de combate do sistema do Riku estão meio estranhas
ainda e mal adaptadas os danos a defesa está melhor mas ainda está problemática, alguns ataques parece que não pegam";
then, told that Riku's own swing clips were played in KH1 Sora's order and timing: "então ela é tocada no ritmo do KH1 mas
eu já falei que era para reproduzir o sistema do KH1 ENTRETANTO adaptando totalmente a jogabilidade do Riku! e
respeitando as especialidades dele, e até o momento está pouco fluído".

This refines 0.162.0's "fora as específicidades do Riku todas as regras de combate precisam ser iguais as do Sora":

- **KH1's system stays**: the rules of the combat (lock-on, the combo counter and its limits, MP and its charge, the
  Guard and its reactions, the post-hit invulnerability, no card duel, KH1 battles and enemies).
- **Riku's moves are his own**: his swings, their order, their rhythm, their hit windows, their clip lengths and their
  travel come from Re:CoM's Riku (and Dark Riku in Dark Mode), never from KH1 Sora's motions. KH1's numbers that only
  describe Sora's animations (his hit frames, motion lengths, root travel) do not apply to Riku.
- **His specialties are kept**: Dark Points and Dark Mode, his sleights, his weapon and his own reactions.
- The goal is a fluid combo: a swing flows into the next as in Re:CoM, not cut by Sora's timing.

### Riku's Guard: defending from the press, everything from in front (2026-11-27, final)

Fork owner, 2026-11-27, verbatim, on 0.198.28 in game: "a defesa segue uma merda, eu aperto o X e ela não conta, assim que
apertar o X em teoria ela tem que estar defendendo igual o KH1, preciso que vc pare de tanto retrabalho sobre esse ponto";
on the proposal (live from the first frame, every hit from in front, not only the blade): "a defesa é a defesa e se movendo
é esquiva parsa". Also: "veja se da para reaproveitar algo da gameplay com o Sora, aliás tome ela de base, mas novamente...
adaptando de forma 100% com a gameplay do riku".

- Riku's Guard is live from the press to the end of KH1's live window (frame 36 of 54), and blocks every hit from in front
  (±100°), on the blade or not. Behind him it is not blocked; KH1's attack types / priorities that pass a guard still pass.
- Square while moving at run speed or more is the roll (KH1's Dodge Roll rule); standing or walking, the Guard.
- Sora's Guard stays KH1's (blade contact, frames 10..36).
- This is settled: no further change to when or what Riku's Guard blocks without a new ask from the owner.
- Sora's gameplay is the base for Riku's, adapted to Riku's own moves (§ Riku's combat).

### Modularity is the base principle (2026-11-27)

Fork owner, 2026-11-27, verbatim: "para os MEUS mods coloque a Modularidade como princípio BAZILAR as coisas precisam ser
intercambiáveis e compatíveis umas com as outras!"; then "pra mim esse é o princípio mais importante da customização, sem modularidade não há customização, é IMPOSSÍVEL haver ser perda de eficiência" (also the workspace's
`CLAUDE.md` rule 12, for all of the owner's mods). The most important principle of customization: an option that is not
modular is not done.

- Every option is a piece that works with every other: any mix the player can choose must work (Riku's hair × Keyblade ×
  costume × Dark Mode; Sora's and Riku's settings; each KH1 / Re:CoM toggle with the others).
- A change is done only when its combinations with the existing pieces work, or its Stage lists the combination that
  doesn't, why, and what falls back.
- Pieces are built apart (own module, own data, own toggle) and joined through one shared rule (e.g. one look table keyed by
  costume, Keyblade and hair), never copied per case.

### Cosmetics are earned: reward after effort (2026-11-27)

Fork owner, 2026-11-27, verbatim: "modularidade e senso de rencompensa após esforço são os pontos mais importântes para diversão, então por isso as nossas features de costméticos precisam ser desbloqueádas a partir da narrativa ou de ações específicas nos jogos" (also the workspace's `CLAUDE.md` rule 13).

- Every cosmetic (costumes, hair, the Keyblade look, any later one) unlocks through Riku's / Sora's story or a specific
  action in game, kept in the save, never free from the start in a released DLL; `all_unlock` stays development-only.
- Today: the Dark Mode costume and the Keyblade of Heart unlock at his first Dark Mode, the Halloween Town costume on
  reaching Halloween Town; the hair choice (`riku_hair`: KH2 / BBS) has no unlock yet (to be chosen by the owner).
- A new cosmetic is not done without its unlock, written in its Stage.

### Riku's Dark Mode keeps every look the player chose (2026-11-27)

Fork owner, 2026-11-27, verbatim, on 0.198.29-0.198.32 in game: "reparei que quando o personagem se transforma no DarkMode o
cabelo é resetado para o Default"; then "no darkmode a keyblade tbm nn troca, já coloca essas regrs para o futuro... além
disso percebi que o cabelo do bbs tem alguns espaços vazios no modo dark mode, testei pulando e atacando".

- Every look chosen for Riku (hair, Keyblade, costume pieces, and any added later) carries into the battle's Dark Mode (Dark
  Riku, Re:CoM's chars 5 / 6), as Dark Riku's own version of that look; the transformation never resets one to Re:CoM's.
- A new look option for Riku is not done until it also works on Dark Riku (model, his own clips and packs, his sleights
  and events), or the Stage says why not and what he shows instead.
- Before a look change goes to the owner it is checked on Dark Riku too: renders of idle, walk, run, **jump**, swings and
  the finisher, guard, a cast, a sleight (`model-change-validation`; jump and attacks are where gaps show).

### Riku's sleights: learned as in Re:CoM, and Re:CoM's Sleights List in his Customize (2026-11-27)

Fork owner, 2026-11-27, verbatim, with three screenshots (Re:CoM's camp Sleights List at 0:02, every row `?????`;
Re:CoM's Shortcuts page with its "Sleights" button; our KH1 Customize page for Riku): "mas eu te passei um print dele
no modo recom, e na listagem de sleight que fica quando vc acessa o menu de shortcuts inclusive falei pra vc
implementar esse sleight lists no modo kh1 nessa tela e vc nn fez.. mas minha duvida é no jogo normal os sleights vem
daquele jeito ali parece... ou estou enganado?"; to the four-part proposal (research, learned-only lists, the Sleights
List in Customize, record the ask): "ss".

- Riku's command menu, shortcuts and Customize list only the sleights Re:CoM has taught (its learned test), as Re:CoM:
  none at a new game, nine at the Ansem tutorial's end, Inverse Burst with the King's at B11 (the workspace's
  `knowledge/ReCoM/recom-riku-sleights.md`). Built 0.198.15.
- His KH1 Customize page gets Re:CoM's own Sleights List (the Shortcuts page's "Sleights" button), `?????` on what is
  not learned. Built 0.198.17 (Square, as on Re:CoM's page; Re:CoM's own list code, `knowledge/ReCoM/recom-riku-sleights.md` § 3).
- Dark Mode has its own three shortcuts (a tester's suggestion, the owner's choice, 2026-11-27): rows "Dark" + the button
  on his Customize once he has reached Dark Mode, filled the first time from his own set with the Dark twins. Built 0.198.18.

## Native always

Re:CoM's own systems own Sora. Native means giving Re:CoM's engine more abilities so it can do
everything KH1 can, and so KH1 runs 1:1 inside Re:CoM. We add to Re:CoM's own systems:

- new states in its player state table;
- new commands in its command/card dispatcher, including commands that need no card;
- new rows in its attack, hit and damage tables;
- new motion slots;
- new effect programs;
- new sound banks;
- new camera modes;
- new HUD elements drawn by Re:CoM's own HUD code;
- new functions or wrappers patched into Re:CoM's own functions.

Re:CoM then runs the KH1 feature itself: it steps it, draws it, plays it, collides it and ends it.

### Not native

Any KH1 logic of ours that runs beside Re:CoM's and copies its results into Re:CoM's fields is
not native, even if it never takes Sora over. Examples:

- our own KH1 player controller driving Sora;
- fake cards slipped into the deck so KH1 attacks "look like" card plays;
- our own hit sweeps or damage maths;
- our own magic or summon runtime;
- our own HUD or renderer.

That code is "running along" or "deprecated", and every change should move it into Re:CoM's
systems. If something can only run along for now, say so and ask before shipping it that way.
Never offer hybrid versions, and never ask which version to build.

## Build what Re:CoM lacks

If KH1 has a feature Re:CoM doesn't, build it into Re:CoM's engine without asking each time.
Examples:

- MP and the magic menu;
- the command menu with free Attack;
- items and quick slots;
- persistent party members with AI;
- AP and abilities;
- Guard;
- combo finishers;
- KH1 summons;
- KH1's level-up stats.

Ask only about open design choices, and before choosing between native designs that change how
the feature plays.

## 1:1 target

With a setting on KH1, every value matches KH1's exe exactly: timings, frame data, momentum,
speeds, windows, damage, MP costs, camera. No approximating.

- KH1's numbers run inside Re:CoM's engine.
- Confirm both games' tick and animation rates in their decomps before converting frame counts.
  Don't assume they differ, or that they match.
- A feature is 1:1 when the tester's trace matches KH1 frame for frame, or the KH1 decomp
  confirms the numbers. Say which you checked and what you couldn't.
- If a real native attempt still can't match KH1, stop and ask. Say what differs, why, and what
  the native options are.

## Re:CoM stays whole

- Nothing Re:CoM does may break or change because of our work. That includes the features KH1
  never had: cards, card breaks, sleights, Reload, stocking, enemy cards, world/room cards,
  Moogle shops and Reverse/Rebirth.
- Replaced systems must still give Re:CoM's events, scripts, bosses and room logic everything
  they read. Bosses and scripted fights that check card values are the main risk here.
- KH1's and Re:CoM's systems share the same native code wherever they can.
- When a choice strongly affects how the game feels, it becomes a Config toggle, not a silent
  change for everyone. Card breaks against KH1-style attacks are an example.

## Little of ours per frame

- As little of our code as possible runs every frame.
- Prefer data that Re:CoM's own code reads (rows, commands, motion slots, attack data, effect
  programs, sound banks) over per-frame hooks that steer it.
- Measure before proposing a performance fix. Don't trade native integration for a small speed
  win without asking.

## Assets

A build step makes them once, from the player's own extracted KH1 and Re:CoM files, into the
mod folder for Re:CoM to load as its own. Nothing from the games goes in the repo. Plan each
step with the tester first.

## Scope is open-ended

Look in KH1 and Re:CoM for related mechanics nobody mentioned. Examples: KH1's MP regen rules,
combo-ender knockback, Trinity, Donald's and Goofy's AI settings, the lock-on camera, the Scan HP
bar. When you find one, say what it is, where it is, and whether it matters for 1:1 feel, then
ask before adding it.

## Decided by the owner (2026-10-29)

> "the goal is to remove attack cards entirely, and have the card battles be replaced entirely by
> the KH1 combat."

At KH1 setting:

- **Sora in the field** moves and attacks like KH1 Sora, natively, with all his abilities. He is
  not a passthrough.
- **Battle transitions stay Re:CoM's.** Attacking a Heartless in the field, or walking into one,
  puts Sora into the battle arena with it, as in Re:CoM.
- **In the arena, no cards are used.** Sora fights as he does in KH1.
- **Enemies with a KH1 equivalent** (Shadow, for example) act, attack and animate exactly 1:1 as
  in KH1, natively inside Re:CoM's enemy systems.
- The card-value question (2026-10-29) is settled: with no cards in battle, there are no card
  breaks against KH1 attacks.

### Answers (owner, 2026-10-29)

1. Magic, summons and items are replaced by their KH1 equivalents. Those with no equivalent are
   removed for now and listed in `docs/card-replacement.md` for later.
2. Re:CoM-only enemies and bosses: keep their Re:CoM behaviour but without card plays, attacking
   on their own timing (option a), for now.
3. First strike: yes, a KH1 field hit keeps Re:CoM's stun / first-strike bonus into the arena.
4. Party: Donald and Goofy don't appear, for now.
5. Progression: Re:CoM's level-up choices, for now. To be revisited.
6. Riku's story (Reverse/Rebirth) stays as it is, for now.
7. Sleights: those with a KH1 equivalent become KH1's version (magic sleights like Fira and
   Firaga become KH1's magic levels; physical ones like Sonic Blade, Strike Raid, Ars Arcanum
   and Ragnarok become KH1 abilities). The rest are removed for now and noted.
8. Enemy cards: removed for now and noted.
9. Keyblade: Sora starts with Kingdom Key, using KH1's stats.
10. Animations (2026-10-30): "i wouldn't bother having an option to switch between KH1 and RECOM
    animations because for all intents and purposes they're the same, just use RECOM's default
    animations, and KH1 animations for anything that isnt included in ReCOM". Re:CoM's own clips
    (KH1 motions baked at 30 fps) play wherever Re:CoM has the motion. Only KH1 motions Re:CoM
    lacks are converted. There's no animation switch. Timing (hit windows, combo windows, travel)
    stays KH1's own data either way.
11. "Included in Re:CoM" means the same kind of move (owner, 2026-10-30, option (a)): where Re:CoM
    has the move (walk, run, jump, fall, land…), Sora uses Re:CoM's animation for it even when it
    isn't a bake of KH1's exact motion. KH1's converted clips are only for moves Re:CoM lacks
    (High Jump, Hurricane Blast, Counterattack, Ripple Drive…). Speeds and timing stay KH1's.
    The function-by-function mapping (KH1 request → Re:CoM action row) is built in M4.
12. Damage to Re:CoM's own enemies at KH1 setting (owner, 2026-10-30): KH1's formula with each
    enemy's KH1 twin's stats (HP, Defence…). This needs KH1's enemy stat tables (not found yet).
13. Re:CoM's damage extras (back-attack bonus, weak points, boss falloff after 5 hits) stay on for
    KH1 hits (owner: "Keep them").
14. Card priority at KH1 setting: KH1's rule. Sora's hits land even while an enemy plays a card.
15. The card shown over an attacking enemy is hidden at KH1 setting.
16. Ledges at KH1 setting (owner, 2026-10-30, option b): Re:CoM's own ledge climb and hang stay as
    they are.
17. (Lead, under decision 11) High Jump uses Re:CoM's own higher-jump animation rows (0x14/0x15).
    The Dodge Roll's one-frame root-motion lead is fixed natively by delaying Re:CoM's root-motion
    step one frame for Sora. Roll protection uses KH1's exact frames: 12 no-flinch frames plus the
    untouchable window from the roll motion's events, not Re:CoM's full invulnerability.
    (Stage 9 finding: KH1's roll has no untouchable window; protection is the 12 no-flinch frames.)
18. Sora's KH1 Strength (owner, 2026-10-30, option a): KH1's level-1 Sora plus the Kingdom Key,
    growing with Re:CoM's level-ups. Path: keep the Rod, give up the Shield (owner, 2026-10-30).
19. Work order after 0.6.3 (owner, 2026-10-30): finish M5 (KH1 damage numbers, Guard's repel and
    Counterattack, the swing-turning check), then M6 (battles without cards); M1's KH1
    recordings when the owner has time.
20. Guard against Re:CoM enemies (owner, 2026-10-30): Sora's side is KH1's (no damage, KH1's
    recoil 0x6E, Counterattack from frame 12). Re:CoM enemies also get new reactions when Sora
    blocks them (Re:CoM's own flinch for the attacker, blocked shots turned back), behind a
    Config toggle. Re:CoM attacks have no KH1 priority: all are blockable with KH1's normal
    recoil for Sora (kind 6, no pushback).
21. Which KH1 record a Re:CoM enemy uses (owner, 2026-10-30): the record of the Re:CoM floor's
    world as on a first KH1 visit (LV1), e.g. an Agrabah Shadow uses KH1's Agrabah Shadow.
    Worlds KH1 doesn't have (Twilight Town, Castle Oblivion) use KH1's LV2 record. HP, Defence,
    damage floor / cap, level and resistances come from that record (decision 12).

22. Sounds for KH1-only moves (owner, 2026-10-30): Re:CoM's own sounds for now ("Re:CoM's
    sounds now"). KH1's own sounds, through a sound bank built from the player's files, can come
    later.
23. Guard's live window stays KH1's frames 10–36 (owner, 2026-10-31, moved on after the trace
    showed the unblocked hits fell outside it).
24. Hurt shapes (owner, 2026-10-31): KH1 hurt shapes on twins (option b of the hitbox plan) are
    skipped for now. Re:CoM's Shadow is a 0.55 m sphere, wider than KH1's 0.35 m ellipsoids.
25. Keyblades at the KH1 setting (owner, 2026-10-31, option a):
    - Each Keyblade card Sora has earned unlocks that Keyblade as a KH1-style item.
    - It is equipped from a **new top-level camp menu entry**, shown only at the KH1 setting and
      drawn by Re:CoM's own menu code. Keyblade abilities are not included.
    - The 16 Keyblades KH1 has use KH1's weapon rows and KH1's models.
    - (Card names corrected 2026-10-31: the six are Star Seeker, Total Eclipse, Midnight Roar,
      Maverick Flare, Two Become One and Bond of Flame. KH2 has Star Seeker, Two Become One and
      Bond of Flame; the other three exist in neither game and keep KH2 placeholders for now:
      owner, "for right now the placeholder models are fine".)
    - Star Seeker, Monochrome, Follow the Wind, Hidden Dragon, Photon Debugger and Bond of Flame
      come from **KH2** (the player's KH2 files): KH2's model, Strength and Magic. Reach is
      measured from the KH2 model's blade length; radius, critical rate / bonus and priority are
      the Kingdom Key's.
    - Unlock rule: **ever earned** (Re:CoM's discovery state `0x14015B3F0`, normal or premium).
      The Kingdom Key is always unlocked.
    - **Soul Eater is not for Sora** (owner, 2026-10-31): Re:CoM never gives Sora its card
      (Riku-only, `0x1401592D0`), so it stays out of his list.
    - The entry is **hidden in Riku's story**.
    - The equipped Keyblade is one byte in Re:CoM's save block (`save + 0x2674`, a range no code
      uses; `docs/recom-keyblade-equip.md`).
    - Order: owning and equipping, then KH1 models, then KH2 models and stats.
    - Models (owner, 2026-10-31): the equipped Keyblade shows **as soon as the camp menu
      closes** (Sora's weapon rebuilt with Re:CoM's own constructor); **only the equipped one is
      loaded**; Halloween Town shows the equipped Keyblade like every other world; no look-only
      test first, build the full swap. Native route: new resource groups for the converted models
      in Re:CoM's player resource table, picked when the weapon is built
      (`docs/recom-keyblade-models.md`).

26. Hits on attacking enemies (owner, 2026-10-31): at the KH1 setting, enemies stay hittable
    while they attack, as in KH1: Re:CoM's "hurt bodies off" at an attack state's start
    (`0x14017DA80`) is skipped, for every attacker (option "Enemies stay hittable", not KH1
    swings only), **bosses included**.

27. Enemy reactions to Guard (owner, 2026-10-31; completes decision 20), behind a new Config row
    "Guard reactions", **on by default**, only at the KH1 setting:
    - a blocked **melee** attacker plays **Re:CoM's own flinch** (state 11 via `0x14017D260(enemy,
      1)`; its card ends first). KH1 plays a per-attack recoil that Re:CoM has no data for, so
      the flinch is a stand-in;
    - a blocked **shot flies back and hurts enemies** with Re:CoM's damage (not KH1: KH1's
      blocked shots change side and never hit again);
    - **bosses and duels included**.
    Research: `guard-enemy-reactions.md` (kept with the decomp tools).

28. Turned-back shots (owner, 2026-10-31): a shot Sora's Guard sends back must do damage
    ("it sends it back but doesnt do any damage. make that do damage").
29. M6, battles at the KH1 setting (owner, 2026-10-31, "go ahead"):
    - **Re:CoM's separate battle rooms stay** (field contact → battle instance, as Re:CoM).
    - **No deck in battle at KH1**: no card drawing, no card UI. Sora still collects cards (drops,
      rewards, shops), so switching back to Re:CoM combat works.
    - **Enemies don't play cards at KH1.**
    - **Several enemies can attack at the same time, exactly as in KH1** (no card-battle turn
      taking).
    - Re:CoM's events, scripts and bosses must still get every value they read (the main risk).

30. M6 details (owner, 2026-10-31), research `m6-cardless-battles.md` (kept with the decomp tools):
    - **Crowd pace: each enemy attacks as if alone (KH1)**. Re:CoM's think-rate scaling by enemy
      count (`0x14017F380`) is off at KH1, as are its other three serialisers (CBTL's single
      enemy slot, the think gate `0x14017D010`, one grant per frame in `0x140187600`).
    - **Human bosses keep their decks as a hidden move list**: same attacks, reload pauses and
      sleights as Re:CoM, no cards shown.
    - **Marluxia 3's stock folder is hidden** at KH1.
    - Staging: (1) Heartless attack without cards, several at once; (2) no deck / deck HUD for
      Sora; (3) bosses one at a time, starting with the "Sora is attacking" check (action 0xB)
      that 12 bosses read.

31. Post-hit invulnerability (owner, 2026-10-31): "Lets also carry over sora's KH1 mechanic
    where if he gets hit he has a period of time where he cant be hit again to prevent getting
    combo locked". At Combat = KH1: KH1's 30 frames from the end of each hit reaction, on Re:CoM's
    own hit-immunity timer `player+0x7BC` (`recom-kh1-attack.md` § Post-hit invulnerability).

32. Enemy pace (owner, 2026-10-31, "lets try b for now"): enemies felt too aggressive at KH1. KH1
    has no shared attack limiter as far as checked (Shadow's script and the 26 common syscalls;
    the rest Assumption), but Re:CoM's enemies are tuned for its crowd sharing. Setting
    `enemy_pace = recom | kh1` (default `recom`): `recom` keeps Re:CoM's crowd sharing (P4 off),
    `kh1` is decision 30's "each attacks as if alone". Interim, until enemies get their KH1
    twins' timing (M10). Config row "Enemy pace" (0.13.4): the owner chose (a), Re:CoM's row 4
    style (one wide value) for Guard reactions and Enemy pace, so all three rows fit in Re:CoM's
    21 item slots (`recom-config-menu.md`).

33. Marluxia 3's card duel (owner, 2026-10-31, "c"): at KH1 the attack that starts Re:CoM's card
    duel (Sora's reaction `0x21`) is a plain hit, so the duel never starts (Sora has no cards to
    duel with). `recom-m6-battles.md` § Marluxia 3's card duel.

34. M7 magic (owner, 2026-10-31, "lets work on M7 ... FULLY implemented, also including a shortcut
    menu, customize menu, MP bar"; answers):
    - **Spell visuals and sounds: Re:CoM's** own effects and sounds.
    - **Max MP grows on level up.**
    - **Spells and spell levels are earned from certain bosses / events.**
    - **Cure heals exactly as Re:CoM's Cure; Aero works as KH1's.**
    - **Stop and Gravity against bosses exactly as in KH1.**
    - **MP bar: Re:CoM-style straight bar** under Re:CoM's HP bar, in KH1 blue (built like Riku's
      Dark gauge).
    - **Command menu: Re:CoM's own menu pieces** (buttons, cursor, text), laid out like KH1's.
    - **Command menu shown in the field and in battle**, as in KH1.
    - **Customize: one "KH1" camp entry** (our entry renamed) with a submenu: Keyblades,
      Customize (later Abilities, Items).
    - **Spell unlocks: each spell's base level when Sora first gets that Re:CoM magic card; the
      -ra / -ga levels at Re:CoM bosses.**
    - **Gravity and Stop on Re:CoM-only bosses** (Larxene, Riku Replica, Axel, Vexen, Marluxia,
      the Riku fights): **KH1's final-boss values** (Gravity 2 %, immune to Stop).
    - Cure heals Re:CoM's amounts: Cure 50, Cura 150, Curaga 300 HP (`SY0002.BIN`
      BattleCardList / StockSkillList), with KH1's cast rules.
    - **MP bar grows with max MP** (KH1's 26 units per MP); colour **read from the player's KH1
      files** at the asset step.
    - **Command menu cursor: plate colour** (the highlighted row uses another colour of Re:CoM's
      command plate); row spacing KH1's 22 units.
    - **Menu words: KH1's own text** from the player's KH1 message files.
    - MP rule: the KH1 exe's (the k-th MP needs 30·k refill units; Fire / Blizzard cost 30 units).
    - Shortcuts saved per save (KH1).
    - Research: `m7-magic.md`, `m7-menus-mp.md` (kept with the decomp tools).

35. M7 MP details (owner, 2026-10-31): **MP refills to full wherever Re:CoM refills Sora's HP**
    (its HP-refill state step, its checkpoint heal, the level-up HP choice); **the six KH2
    Keyblades use their KH2 Magic stat as their max-MP bonus**. `recom-m7-mp.md`.

36. M7 spells (owner, 2026-10-31): **Fire re-targets as KH1** (each frame, a ball with no live
    target turns to the best enemy within 5 m: a wrapper on Re:CoM's homing step for our balls);
    **spell hits use KH1's element resistances only** (no Re:CoM % on top).

37. M7 (owner, 2026-10-31): **Fire re-picks exactly as KH1** (only balls cast with no target; a
    dead target means the ball flies straight); **Thunder's 50 % world dim is left out**.

38. M7 spell details (owner, 2026-10-31): **Fire's release step is KH1's 15 cm curve** (replacing
    Re:CoM's clip travel in the root-step wrapper); **Gravity slows everything about an enemy under
    the falling sphere** (its whole frame step to KH1's factor; research first); **Stop's held hits
    land at KH1's pace** (Re:CoM's 8-hit cap kept).

39. KH1 HUD (owner, 2026-10-31): "Would we be able to instead have an option where the Re:CoM
    [HUD] is replaced entirely by the KH1 hud? ... use the KH1 decomp to get exact positioning
    and graphics". Supersedes decision 34's HUD look for that option:
    - **A switch "HUD: Re:CoM / KH1", in the "KH1" camp entry** (stage 7.13).
    - At KH1 HUD: **KH1's own elements with KH1's graphics and exact positions** from the KH1
      decomp: the HP/MP portrait ring (bottom-right), the command menu with its COMMAND header
      (Attack / Magic / Items / Summon), the lock-on target HP bar, and **Donald's and Goofy's
      gauges top-right, always shown**.
    - **Re:CoM's HUD components are hidden** for now (minimap, room plate, EXP popup, Moogle
      points, FRIENDS row, Re:CoM's HP gauge). **Remind the owner** once the KH1 elements are right
      to place the Re:CoM extras (minimap, room plate, …): they will give exact places.
    - Details (owner, 2026-10-31): **HP ring: KH1's rule on Re:CoM's raw HP** (5° per HP to
      270°, then 26-unit segments per 6 HP); **Donald / Goofy gauges hidden until they join**
      (shown while Re:CoM has them out); **menu words in KH1's font converted into a Re:CoM
      font**; **command menu colour: KH1's field look in the field, the battle look in battle
      rooms**; **Re:CoM's enemy gauges hidden and KH1's lock-on cursor used** (Re:CoM's level-up
      choice kept); **low-HP sound: Re:CoM's**. Research `kh1-hud.md` (kept with the decomp tools);
      plan H0 (read-only dump of KH1's 24 `exchange/*_uibin_*.bin` layouts and HD textures on the
      owner's PC) then H1–H8, one element per version.
    - Hit shake / size-class slide (2026-11-01, owner away: "keep going"): built the native way,
      **baked into every piece's animation data at the asset step** (Re:CoM plays it; nothing of
      ours per frame). Owner to confirm when testing.

- **M7–M9 choices (owner, 2026-11-01: "Finish everything up to M9"):**
    - **Shortcuts (7.12):** L1 as in KH1. Small choices taken as KH1's (Cure / Aero from a
      shortcut cast on Sora, slots filled as spells are learned, L1 + Triangle at a save point
      casts, KH1's panel art).
    - **Items (M8a):** sold in Re:CoM's Moogle shop **and** dropped by enemies (KH1's rates).
      **Healing amounts: Re:CoM's own item-card amounts** (not KH1's 30 / 60 HP).
    - **Summons (M8b):** each unlocks with its first Re:CoM summon card. **Wait for the party:**
      summons are built after persistent Donald and Goofy (KH1's summon needs both and sends them
      away), so M8b follows the party milestone.
    - **Abilities (M9):** unlock with Re:CoM (its level-up sleights: Sliding Dash, Blitz, Stun
      Impact, Zantetsuken, Strike Raid, Sonic Blade, Ars Arcanum, Ragnarok); the five with no
      Re:CoM source (Vortex, Aerial Sweep, Hurricane Blast, Ripple Drive, Gravity Break) at
      **KH1's learn levels** on Re:CoM's level; an **equip screen with KH1's AP** (KH1's costs,
      AP +2 at KH1's levels, on Re:CoM's level).
    - **M9 follow-ups (owner, 2026-11-01):** Gravity Break (no KH1 learn level) unlocks at **Re:CoM
      level 30**; the learn list is **KH1's Rod list**; with the equip screen, a newly unlocked ability
      arrives **unequipped**, as in KH1.
    - **9b follow-ups (owner, 2026-11-01):** **Guard from the start** (still costing AP); High Jump,
      Glide and Superglide unlock **where Re:CoM unlocks them**; **Dodge Roll equipped from the start**;
      **Combo Master at Re:CoM level 50**; KH1's Rod-path **max MP +1 at levels 30 and 66** added; the
      Abilities page rows use Re:CoM's **blue** box.
    - **Later the same day:** **Guard and Dodge Roll are part of Sora's base movement kit from the
      start, not abilities** (no AP, not on the Abilities page); High Jump / Glide / Superglide are
      **equipped automatically when Re:CoM gives them** (still removable). Specials (9c): **Re:CoM's
      sleight effects** until the KH1-effects work; KH1's blue scene tint **skipped for now**.
    - **Spell effects (owner, 2026-11-01):** "lets look at how KH1 handles these FX and bring them in
      here 1:1": KH1's own spell effects (same effect engine as Re:CoM's) are ported natively, behind
      **their own toggle** (KH1 or Re:CoM spell effects); **Re:CoM's spell sounds are kept**; built
      **after the M9 specials**, Fire first as the pilot.
    - **Sonic Blade (owner, 2026-11-01, on 0.56.1's questions):** the dashes' voices "however it works
      in base ReCoM"; the per-dash effects "however it works in base ReCoM"; usable **only in battle**
      (not in the field rooms). Whether the other specials follow the same rule: ask at 9c.2.
    - **Damage and enemy HP (owner, 2026-11-01):** "depending on which keyblade sora has equipped, it
      does recom's damage for the corresponding keyblade … lets also have Recom's enemy HP". Re:CoM's
      formula (attack stat × the equipped Keyblade card's power ÷ 100) for every KH1 swing: **KH1's
      normal swings take Re:CoM's normal column (0), KH1's combo finisher Re:CoM's finisher column
      (2)**; Re:CoM's enemy HP; **spells and specials do Re:CoM's card / sleight damage** (each spell its
      Re:CoM magic card's, Sonic Blade Re:CoM's Sonic Blade sleight's). A Config setting (Damage: Re:CoM
      / KH1), Re:CoM by default; KH1 keeps decisions 12 / 21. Supersedes decision 12 at the default.
      Sonic Blade also gets **Re:CoM's start tint** (as base Re:CoM's).
    - **Damage follow-ups (owner, 2026-11-01, on 0.58.1's questions):** the Damage setting stays
      **ini only** (`damage = recom | kh1`); at Damage = Re:CoM, **Gravity and Stop use Re:CoM's own
      numbers** (Gravity / Gravira / Graviga 20 / 40 / 70 % of current HP with Re:CoM's immunities and
      flatten; Stop 9000 / 15000 / 21000 ticks); **Aero's wind uses Re:CoM's Aero by level** (card,
      then Aerora / Aeroga sleights, as Fire / Blizzard / Thunder); **Gravity Break and Zantetsuken
      take KH1's percentage of Re:CoM's HP**.
    - **Damage follow-ups 2 (owner, 2026-11-01, on 0.59.0's questions):** at Damage = Re:CoM, **Aero
      level 1 gets a wind carrying the Aero card's value** (not KH1, owner's choice); stopped enemies keep
      KH1's held-hit pace and attacks-off; **Gravity Break and Zantetsuken skip Re:CoM's boss falloff
      (`+0x290`), the "boss rule" of the question** ("skip Re:CoM's boss reduction for these two": KH1's
      `resist[8]` and floor alone reduce bosses; the frozen bonus `0x14015D940`, which the question
      mislabelled, stays); Aero's wind in the field keeps Re:CoM's no-card AP.
    - **Boss revenge value (owner, 2026-11-01):** "lets also have humanoid bosses have a revenge value
      system, similar to how they work in their fights in KH2 … borrow their revenge value". Chosen:
      KH2's **story / Absent Silhouette limits** (Axel 92, Larxene 100, Vexen 150, Marluxia 100 for all
      three forms, Hades 80 then 50 below 30 % HP, Cloud 100); **all humanoid bosses** (also Hook,
      Riku, Riku Replica, the hooded figure: KH2's default 100); on revenge the boss plays **its own
      Re:CoM attack nearest its KH2 revenge move** (Larxene: her leap, as KH2's decoy teleport); **Re:CoM's
      boss damage falloff is off for bosses that have revenge**. KH1 hits add KH2 Sora's ATKP revenge
      values (swing 8, finisher 24, air swing 12, air finisher 28, Fire 8, Blizzard 16, Thunder 32).
      Notes: `docs/recom-boss-revenge.md`.
    - **Revenge moves untouchable (owner, 2026-11-02):** "the humanoid bosses that have revenge value
      can't be damaged or interrupted during their retaliation move": while a boss plays the attack its
      revenge started, Sora's hits do no damage and don't interrupt it (Re:CoM's own rule for an attacking
      enemy, without decisions 14 / 26's bypass for that attack).
      Then (2026-11-02, on 0.65.0's questions): **Hades untouchable only during his wind-up** (his spin
      can be hit; his fire-up still doesn't flinch); the revenge check at **KH2's frame 20 of the flinch**
      (Hades 30), or at the flinch's end if shorter (0.65.0 checked on its first frame: a bug).
      **Cloud's revenge move: his homing charge** (state 4, `e106_053`, 2 charges above half HP, 4 below;
      nearest KH2's Sonic Rave), instead of Omnislash.
      Then (2026-11-02): "you can still hit bosses during their revenge move (or if they use their revenge
      move naturally instead of it being triggered by revenge value)": a bug to fix, and **the boss's
      revenge-move attack is untouchable whenever it plays it**, by revenge or by its own choice.
    - **Dodge roll (owner, 2026-11-02):** "make dodge roll work how it does in ReCoM, not KH1" — **the
      whole roll** (Re:CoM's motion, speed, distance, timing and invincibility) at Combat = KH1. Supersedes
      decision 17's KH1 roll frames; an ini key keeps KH1's roll selectable.
    - **Enemy card sounds (owner, 2026-11-02):** "you cant hear enemies stocking cards or refilling their
      deck": at Combat = KH1, the sounds of enemies stocking cards and reloading their deck are silent.
      Then (on 0.65.2's questions): the enemies' first deal at battle start **stays silent**; a human
      boss's item-card page switch (sound 3) is **silenced too**.
    - **Phase 4, KH1 enemies (owner, 2026-11-02):** "lets move on to doing phase 4, except, i only want
      the AI, attacks, reactions and motions from KH1 equivalent enemies, everything else (emphasis on
      stats) stay the same as they are in RECoM". KH1-equivalent enemies take **KH1's AI, attacks
      (timing, hit shapes, behaviour), reactions and motions**; **stats (HP, damage, defence, drops,
      EXP…) and everything else stay Re:CoM's** (the Damage setting still decides Sora's damage to
      them). M10 Shadow pilot first, then M11.
      M10 choices (2026-11-02): native states (KH1's behaviour ported as new Re:CoM states; the
      interpreter would run along: not offered); `enemy_ai = kh1 | recom` ini (default kh1); KH1's
      motions incl. rise and death (Re:CoM's prizes / EXP); KH1's reactions (launch weight as KH1's;
      flinch part from the hit's height, Re:CoM's hurt shapes); KH1's recoil when Sora guards; Re:CoM's
      attack power per attack; Re:CoM's Stop / freeze / Gravity states; **battle enemies only** (field
      Shadows keep Re:CoM's roaming, owner).
      Then (2026-11-02, on 0.67.2's questions): a sunk Shadow **stays lockable** (Re:CoM's one lockable
      flag; KH1's lock button can lock it, its auto-target can't) and **keeps pushing Sora** while it sinks
      and rises (KH1 lets him walk through): both left as they are.
    - **M11 (owner, 2026-11-02):** "it works perfect. Now let's bring in the remaining KH1 adjacent
      heartless": the 30 other ordinary Heartless with a KH1 twin, under M10's rules (KH1 AI / attacks /
      reactions / motions, Re:CoM stats, battle only). Step 1: a shared framework from the Shadow's
      code, proven on the Soldier; then the families.
      Soldier choices (2026-11-02, on 0.68.0's questions): power mapping kept (kicks / double kick / rush /
      air kick on Re:CoM's kick, jumping kick / rush leap / spin kick on its spin kick); **a guarded attack
      whose KH1 recoil motion the model lacks plays Re:CoM's flinch**; battle spawns keep KH1's appearance;
      KH1's wander picks points **around where the enemy stands** (fix the Shadow to it); KH1's leash
      left out (would make a battle unwinnable).
      Gargoyle (2026-11-02): its launched object (attack 219) is **one of Re:CoM's own bullets, moved and
      timed by KH1's numbers**.
      Darkball (2026-11-02, on 0.71.0's questions): the drop on Re:CoM's bite power; warp smoke / trail
      **later with the effects milestone (M13)**; stays lockable while invisible; KH1's element-based
      reactions (Gravity stagger) **left out** for now.
      Bandit / Air Pirate (2026-11-02, on 0.73.0's questions): the Bandit's thrown scimitar is **a Re:CoM
      homing bullet moved by KH1's numbers, drawn with a Re:CoM effect**; **a shared enemy-guard step**
      (Re:CoM's own body-guard flag) for the Bandit's block stance and spin repel, then the Large Body and
      Defender; powers kept (leap on the whirl, swoop on Re:CoM's second sword attack); the Air Pirate's
      world-height checks read as heights above the battle floor; the Bandit's Agrabah rules only in
      Re:CoM's Agrabah.
      Guard / blade follow-ups (2026-11-02, on 0.75.2's questions): **hide Re:CoM's "Guard" popup over
      KH1 enemies**; the block keeps Re:CoM's whole-front body guard; **Sora's swings can knock the thrown
      scimitar away** (KH1); powers kept (the blade on the whirl, the Wyvern's dive on its rush).
      Search Ghost (2026-11-02, on 0.76.0's questions): **KH1's drain rule on Re:CoM's HP, EXP and prizes**
      (a landed drain restores it to full after its hold; a hit during the hold gives KH1's EXP bonus and 3
      prizes instead); **its burst repels Sora's swings** through the shared enemy guard.
    - **Summons now (owner, 2026-11-01):** "Lets get summons working. I want the summon section on the
      command menu to work like KH1, but each summon should do their lv 3 recom summon equivalent, cloud
      included" (supersedes "wait for the party"). KH1's Summon list (Magic list's last entry, learn
      order, KH1's greying) issues **Re:CoM's LV3 sleight** of each summon (same card ×3: Proud Roar /
      Showtime / Paradise / Splash / Twinkle / Flare Breath LV3; **Cloud: Omnislash**), started natively
      without a card. Chosen: **MP cost from Re:CoM's CP** (2 MP for the 20-CP cards Simba..Tinker Bell, 3
      for the 25-CP Mushu and Cloud); **KH1's party rule skipped until the party exists**; **Re:CoM's
      summon damage at both Damage settings**; **KH1's once-per-battle rule kept** (row locked while one
      is out, no Dismiss); battle rooms only; each unlocks with its first Re:CoM summon card.
    - **Summons follow-up (owner, 2026-11-02):** "summons should be able to be used as long as you have
      MP, also, sora should have iframes through all summon animations". KH1's once-per-battle rule is
      **dropped** (usable whenever Sora has the MP); **Sora is invincible through every summon's whole
      animation** (start to Re:CoM's end action); "cloud should also be at the top of the summons list"
      (**Cloud first**, then the others in learn order).
    - **Gimmick card triggers (owner, 2026-11-02):** "interrupting an attack with a finisher is very rare";
      chose **(a) + (d)**: (a) when Sora's KH1 Guard blocks an attack of one of the five card-break bosses
      (Guard Armor, Trickmaster, Jafar, Parasite Cage, Dragon Maleficent), the boss gets Re:CoM's card-break
      stagger and its own drop roll; (d) each of those bosses also drops its gimmick card, without a
      stagger, at **75 %, 50 % and 25 % of its HP**. Then (2026-11-02): "only drop one at 50% and
      occasionally when guarding": the HP drop at **50 % only**.

## Known open design questions (ask the owner, don't guess)

- Revisit later: party, progression.

- **Search Ghost burst cone** (owner, 2026-11-02): keep KH1's cone (cos 1.0, 1:1); the repel rows stay as KH1 has them and practically never fire.

- **Pirate choices** (owner, 2026-11-02): the five attacks Re:CoM's Pirate lacks take power by kind (as built). The Wyvern's element no-reaction (`sys 42` for attribute 2 / 3 / 4 hits) stays left out. The fliers' "doesn't flinch" (Darkball, Air Pirate's dive, Wyvern's dive) move onto the no-reaction armour, one version each.

- **Large Body choices** (owner, 2026-11-02): the stomp shockwave: research how Re:CoM's shockwave tests its hit first (if already a ring, nothing changes). Punch and charge on the belly bump's power (as built). Attack-meets-attack clashes (a swing into a charging belly, fist against blade): build them into the shared enemy guard for every ported enemy, researching KH1's order of tests first.

- **Neoshadow choices** (owner, 2026-11-02): (1) drop room 29's fixed spots: no fixed points, they wander and attack instead. (2) Build the grab from below natively. (3) The call stays, nothing comes. (4) The dive on Re:CoM's come-up strike power (id 1). (5) Fire hits a diving / rising Neoshadow (no turn-back).

- **Enemy pace, Re:CoM-AI Heartless** (owner, 2026-11-02, on a Fat Bandit video: "it attacks so frequently that there's no opening ... lets try to tone it down"; research `docs` § enemy pace): keep porting Heartless to KH1's AI (they get KH1's own waits); meanwhile every Heartless not yet ported waits a longer cooldown before each attack: **each enemy's own value** (the one each class passes to Re:CoM's attack end `0x14017E540`, which Re:CoM overwrites with 250 ticks at `0x14017E5F5`; the Fat Bandit's is 6000 ticks = 120 frames). The KH1-AI enemies feel fine ("mainly un-ported ones").
- **Sora's movement speeds** (owner, same day: "have sora run at the same speed as he does in recom"; then "have run speed, jump height, and air speed all be separate toggles in the settings"): three ini keys at Combat = KH1, each `recom | kh1`: `run_speed` (default recom: Re:CoM's 6.2 m/s instead of KH1's 4.8), `jump_height`, `air_speed` (default kh1: as now). Reason found: Re:CoM's homing shots (2.5°/frame for 30 frames) are tuned to be outrun by Re:CoM's Sora.
- **Large Body stomp ring** (research 2026-11-02): Re:CoM's shockwave is a solid flat disk (`CCircleCollision`, its test `0x140177190`), not KH1's ring; per the owner's "research first", KH1's ring is built into Re:CoM's circle test for our ring only.
- **Clash choices** (owner, 2026-11-02, on 0.82.0): the clash look waits for M13 (Re:CoM's spark until then). Fix: the Soldier's spin-kick chest and the Powerwild's flurry body only block swings (keep their records for the clash test, never hurt Sora), as KH1. Wight Knight: the leap slash on Re:CoM's two-strike claw (id 0, as built). KH1 attack attributes (8 / 10 etc.) stay left out for every enemy.
- **Pace / movement follow-ups** (owner, 2026-11-02, on 0.83.0 / 0.84.0): Heartless bosses keep Re:CoM's 250-tick wait (they get KH1's when ported). A running jump keeps the run speed in the air (as built). `jump_height = recom` also uses Re:CoM's 2–3-frame crouch (to build). `air_speed = recom` eases toward 0.8 × Sora's own current ground speed (as built).
- **Neoshadow grab / stuck turn** (owner, 2026-11-02): the grab as a new Re:CoM player reaction ("held") playing KH1's held clip as a new Sora motion slot, freed by a hit (the 1:1 option: new reaction kind in the damage check `0x1401E6DE0`, a new case in `0x1401EB0F0`, a 0-damage record, asset re-run). A stuck Neoshadow turns toward Sora (as built).
- **Fat Bandit choices** (owner, 2026-11-02): the flame on the fireball's power; a row of Re:CoM fireball effects along it until M13; the weak spot above its head left out; swings bat its fireballs back (KH1's way). All as built.
- **Gargoyle / Defender choices** (owner, 2026-11-03, on 0.90.0): the Gargoyle's ball growth and its fades wait for M13; Sora's Guard sends the Gargoyle's ball back KH1's way (as a swing does), not Re:CoM's homing turn-back; a wall still ends the ball. Defender: its own batted fireball's big hit when it strikes from the front (as built); its strike / spin / bash on Re:CoM's melee power, the flame on the fireball's (as built); one fireball a round (KH1).
- **Neoshadow grab timing** (1:1 rule, no owner question needed): the hold starts on KH1's frame (208 a frame earlier), not 1–2 frames late.
- **Bouncywild choices** (owner, 2026-11-03, on 0.91.0): it never leaves (as built); **a slip loses nothing** (Re:CoM's slip without its Moogle-point scatter); **its thrown banana makes Sora slip too** (Re:CoM's banana rule, not KH1's flinch); other enemies don't slip on peels.
- **Red Nocturne choices** (owner, 2026-11-03, on 0.92.0): hit by its own sent-back Fire it takes **Re:CoM's stun** (instead of KH1's 30 s dizzy; the EXP bonus stays); its idle keeps KH1's fixed facing; a big Fire still flies if the caster dies late in the cast (KH1); the Fat Bandit's and Defender's fireballs keep Re:CoM's Guard turn-back (decision 28).
- **Movement keys in the ini** (owner, 2026-11-03: "where is the toggle in the settings"): the three keys are written into the player's existing `kh1-recom.ini` (an old file has no lines for new keys; the defaults apply until then).
- **KH1 camp tab: settings submenus** (owner, 2026-11-03: "Under the "KH1" tab in the pause menu, I want run speed, jump height, and air speed under a sub menu called "Movement" I also want the guard Reactions, combat, and enemy pace to be under a seperate sub menu in the KH1 tab called "Combat." Also, Just have the "KH1" Tab in the camp menu always be there, regardless of if the combat is set to ReCoM or KH1"; answers):
  - **Movement** submenu: Run speed, Jump height, Air speed, Dodge roll.
  - **Combat** submenu: Combat, Guard reactions, Enemy pace, Enemy AI, Revenge Value (owner, same night: "have \"boss revenge\" be renamed to \"Revenge Value\""; the ini key stays `boss_revenge`).
  - Combat, Guard reactions and Enemy pace **leave Re:CoM's Config menu** (it goes back to only Re:CoM's own rows).
  - The KH1 tab is **always shown**. At Combat = Re:CoM it shows only Combat and Movement; Items, Customize and Abilities appear at Combat = KH1.
  - Damage stays ini-only.
- **KH1 tab follow-ups** (owner, 2026-11-03, on 0.93.0): show the KH1 tab in Riku's story too, with only Combat and Movement (built 0.93.4); English-only rows (as built); row order Items, Customize, Abilities, Combat, Movement (as built).
- **Red Nocturne dead caster** (owner, 2026-11-03, on 0.93.3): keep KH1's rule (a kill cancels the big Fire).
- **Riku tab / casts / Blizzard** (owner, 2026-11-03): hide the KH1 tab in Riku's story again (none of its settings act there); Re:CoM's Stop keeps cancelling a caster's spell (not KH1's paused wait); a wall still ends the Blizzard.
- **Parasite Cage** (owner, 2026-11-03: "the parasite cage shouldnt flinch when you attack, let it go through with its attacks, give it about 3 seconds between each attack"): at Combat = KH1 (Re:CoM unchanged), Sora's hits never flinch it or cut its attacks (damage still lands; Re:CoM's Stop / stun / freeze / Gravity / defeat stay Re:CoM's), and it waits about 3 s (180 frames) after each attack before the next. Its gimmick-card triggers must keep working.
- **M11 stops after the Yellow Opera and Green Requiem** (owner, 2026-11-03: "i suppose finish the opera and requiem, but stop there."). Not ported for now: Wizard, Sea Neon, Screwdiver, Aquatank, Barrel Spider, Creeper Plant, White Mushroom, Black Fungus, Spade / Heart Soldier (they keep Re:CoM's AI with the 0.83.0 cooldown).
- **Parasite Cage faint / card** (owner, 2026-11-03, on 0.95.0): its chance faint (state 17, `0x1403C4060`) only from Fire at Combat = KH1, not from Sora's finishers (to build); the finisher-interrupt card trigger as is.
- **Oogie Boogie** (owner, 2026-11-03: "oogie boogie is impossible to defeat. you're supposed to break one of his attacks which lowers a gate where you can hit him. can we make it so that if he uses the attack that you're supposed to break twice without being broken, the gate lowers?"): at Combat = KH1, the gate lowers as if broken (owner, re-sent ask without "twice", then "After once": each time that attack ends unbroken).
- **Last M11 choices** (owner, 2026-11-03, on 0.96.0–0.97.0): the second Shadow / Search Ghost ids (32 / 33) keep Re:CoM's AI (their packs stay unextended); the Green Requiem heals Re:CoM's amount, skips allies KH1 can't hit (a sunk Shadow, an invisible Darkball; to build), any enemy in the battle (either AI); the Yellow Opera's Thunder: Stop / stun still cancels it after the mark; a Re:CoM effect over the marked spot now as a stand-in for KH1's warning cloud (to build, research which).
- **Oogie gate steps** (owner, 2026-11-03, on 0.97.1): one gate step per unbroken dice throw (as built; three throws open it).
- **Green Requiem pick** (owner, 2026-11-03, on 0.97.3): KH1's way (the pick unchanged; a heal on an ally that can't be hit when it lands is wasted).
- **M12 sleights and progression** (owner, 2026-11-03, verbatim): "i want each spell to be first learned when sora gets their respective spell card. I then want bosses/story events throughout the game to upgrade the spells (similar to KH1 progression.) If you press left on the d-pad, instead of summons at the bottom of the command menu, it should say "sleights" In the sleights menu I only want ACTIVELY EQUIPPED sleights to show in the list. When unlocking each sleight (not including friend sleights, the generic magic sleights, and any sleights that have a KH1 combo modifier/finisher equivalent) it will be added as an ability that you can equip with AP, with reasonable AP costs relative to their damage. On level up, instead of having the choice to level up CP, when the combat is set to KH1, it should be AP up, and it should give you 2 AP each level. this should be the only way to obtain AP, aside from the AP you start the game with. I also would like sleights to be able to be added to the customize menu, for shortcuts. As sora levels up in KH1, he receives abilities like MP Haste, MP Rage, Leaf Bracer, combo plus, air combo plus, second chance and Berserk. I want those to be added to his ability list at the levels he would normally get them (subject to change,) regardless of level up choice, and I want there to be a notification that lets the player know that they've obtained those abilities. Once sora gets the required cards to be able to do a sleight that isn't locked behind a chest, event, or sleight level up (think "idyll romp" for example,) sora should get a notification that that sleight has been added."
  - Research first (`docs/recom-m12-sleights.md`), then a plan and the open questions; no code before the owner's go-ahead.
- **M12 answers** (owner, 2026-11-03, on `docs/recom-m12-sleights.md`):
  - A sleight is **added when Re:CoM teaches it** (its learned bit via `0x14023AA60`: level-up, event, chest, first card); the notice fires then.
  - **Cost: MP by tier**: AP 2 → 1 MP, 3 → 2 MP, 4 → 2 MP, 5 → 3 MP; Idyll Romp / Cross-slash+ cost all MP like KH1 summons.
  - **AP abilities**: the 25 magic sleights plus Idyll Romp and Cross-slash+; the plain LV2 / LV3 summons stay under Summon. **AP costs**: 2 — Judgment, Bind, Confuse, Terror, Warp, Warpinator, Synchro, Teleport, Gifted Miracle, Idyll Romp; 3 — Fire Raid, Blizzard Raid, Aqua Splash, Freeze, Firaga Burst, Reflect Raid, Homing Blizzara, Lethal Frame, Shock Impact; 4 — Thunder Raid, Raging Storm, Magnet Spiral, Quake, Tornado, Cross-slash+; 5 — Holy, Mega Flare.
  - **Friend sleights wait for the party** milestone.
  - **D-pad Left toggles Summon / Sleights anywhere on the command menu** (drops KH1's Left → Attack).
  - **Strike Raid, Ars Arcanum, Ragnarok**: "in KH1FM are triangle prompts. Let them be triangle prompts and also show up in the sleights menu."
  - **AP**: KH1's start 3, +2 per AP Up pick (replacing CP Up at Combat = KH1), **no cap**, nothing else; **old saves: each CP Up already taken counts as an AP Up**.
  - **Berserk at Damage = Re:CoM: a % damage bonus** (the share KH1's +4 Strength gives at that level).
  - **Notices: Re:CoM's sleight-learned screen** (`CLevelUp` sleight mode, its "ability" wording), without its card-stock help page at KH1.
- **M12 step 1/2 follow-ups** (owner, 2026-11-03): AP Up also gives Re:CoM's +25 CP unseen (CP keeps growing for a switch back); skip the card help page after the level-up Sleight pick too; notices for every ability from a level.
- **MP Rage / Berserk** (owner, 2026-11-03, on 0.98.x): MP Rage's HP lost scaled to KH1's share (as built); Berserk at Damage = Re:CoM per hit from KH1's table (as built).
- **Bosses invulnerable during dimming moves** (owner, 2026-11-03: "anytime bosses use a move that dims the room, just make them invulnerable for that move"): at Combat = KH1, a boss is untouchable for the whole of any move during which Re:CoM dims the room (its sleights / big moves), ending with the move.
- **Sleight ability texts** (owner, 2026-11-03, on 0.99.0): a short description for each sleight's help line (what it does + its MP cost); the name stays "Lethal Frame".
- **Dim-move follow-ups** (owner, 2026-11-03, on 0.99.1): Larxene from the grey (as built); Marluxia 3 only during attacks (as built); **the Wizard and the Aquatank get the same rule** (to build); no other moves added.
- **Sleights menu follow-ups** (owner, 2026-11-03, on 0.100.0): Re:CoM's air-start sleights (Homing Blizzara, Warpinator, Warp, Bind, Confuse, Terror, Teleport) usable mid-air (to build); no MP shown in the list (as built); at Damage = KH1 sleight damage scaled by the enemy's KH1 max HP / its Re:CoM max HP (to build).
- **KH1 damage scaling** (owner, 2026-11-03, on 0.100.4): at Damage = KH1, sleights also scale on enemies without a KH1 twin (bosses, Re:CoM-only enemies) by their KH1-scale HP / Re:CoM HP; summons get the same rule.
- **Sleight shortcuts** (owner, 2026-11-03, on 0.100.6): a sleight shortcut opens the L1 list even before any spell is learned (as built).
- **KH1 specials follow-ups** (owner, 2026-11-03, on M12 step 7): Ragnarok's shots ship with the wrapper on Re:CoM's homing step (as Fire's re-target); **Ragnarok offered against any target** at KH1 (not KH1's 1.6 m-above rule; built 0.101.4); **Sonic Blade also in the Sleights list** (built 0.101.5).
- **Prompt order** (owner, 2026-11-03, on 0.101.4, asked which special the Triangle prompt picks now that Ragnarok shows against any target): **Ragnarok last**: the prompt checks Ars Arcanum, Sonic Blade and Strike Raid first (built 0.101.6). KH1's own order stays in the code for reference.

- **M13 effects, sounds and voices** (owner, 2026-11-03: "Let's work on m13"; answers to the plan in `recom-m13-plan.md`):
  - **Three toggles** in KH1 > Combat: **Effects**, **Sounds** and **Voices**, each KH1 / Re:CoM, beside the spell-effects toggle (decision 34, 2026-11-01).
  - **Sora's voice:** "Separate toggle for KH1 voice lines": the Voices toggle picks KH1's battle voice clips or Re:CoM's. Cutscenes keep Re:CoM's.
  - **KH1 specials** (Sonic Blade, Strike Raid, Ars Arcanum, Ragnarok): switch to KH1's effects and voices under those toggles. This replaces "however it works in base ReCoM" for the specials.
  - **Found while researching** (no hit-stop at KH1, room dust and splashes, clash EXP): no preference given, so **not added**; to be asked again when their stage comes up.
- **Sleight dim i-frames** (owner, 2026-11-03: "any sleight that sora does that dims the room should give him i-frames"; then "Go ahead, specials included"): at Combat = KH1 (Sora's story), while Sora's sleight has the room dimmed, every hit on him is skipped (no damage, no flinch). This covers Re:CoM's sleights whose start dims and the four KH1 specials. Summons already had i-frames (2026-11-02). Combat = Re:CoM unchanged. Not checked: whether KH1 itself makes Sora untouchable during its specials (the owner's rule applies either way). Built 0.102.1.
- **Enemy cards as abilities** (owner, 2026-11-03, on the proposal "If we were to convert the boss/enemy cards into abilities"; verbatim: "all enemy cards that are the same as a KH1 Ability should replace its respective KH1 ability. Soldier card should only replace 1 combo plus. 1. C 3. A, roxas and bandit at 7 AP. 4. A Also, let the enemy cards have their own separate section in the ability tabs called "Enemy Skills" Have each ability have the name of the original card, and a description of what the ability does."):
  - **KH1 matches replace the KH1 ability**: Soldier → **one** of the three level Combo Plus; Hook → Second Chance; Hades → Berserk; Creeper Plant → Leaf Bracer; Bouncywild → Treasure Magnet (KH1's). That ability then comes from the card, not from its level.
  - **Groups** (1. C): every card with a KH1 meaning, including the group-3 MP / no-interrupt / sleight versions. **Not** the card-value cards (Shadow, Screwdiver, Powerwild, Sea Neon, Pirate).
  - **Very strong ones** (3. A): included at 5 AP; **Roxas (Double Strike) and Bandit (Combo Finish) at 7 AP**.
  - **Card Soldier's Attack Haste** (4. A): included, as the one deliberate change to KH1's swing timing.
  - **Enemy Skills**: a section of its own on the Abilities page. Each skill is named after its card, with a description of what it does.
  - Plan before code: `docs/recom-enemy-skills.md`.
- **Sleight / summon hits on attacking enemies** (owner, 2026-11-04: "some hits from the lethal frame also arent hitting, same goes for the summons at times"; fixed 0.102.2): at KH1 they break into an enemy's attack as a KH1 swing does (decisions 14 / 26 extended to Sora's sleights and summons).
- **HUD at Combat = Re:CoM** (owner, 2026-11-04: "with combat set to RE:CoM the hud should also go back to the normal positioning"): every KH1 HUD piece follows Combat; at Re:CoM the HUD is Re:CoM's as placed by Re:CoM (built 0.102.3).
- **KH1 Moogle shop** (owner, 2026-11-04, verbatim: "When Combat is set to KH1. I want the moogle shops to change. When buying from them, they should sell items. no other tabs should appear, it should just be a list like KH1 is (see screenshot) It doesn't have to look 1:1, it can use Re:CoM assets, but it has to be a list, and they need to sell items, with rarer items being more available in later worlds. (elixirs and whatnot) all prices should be listed. Lets remove mega potions, mega ethers, and mega elixirs The very early game shops should only sell potions, ethers, and tents. Tents and camping sets should have their descriptions changed to say sora instead of "to the party" and should have their cost reduced. The sell section should be similar, but a list that shows items currently in possession, and their sell price If possible as well, enemies should have the ability to drop items how they do in KH1. prices should be Moogle Points"; answers to the plan):
  - **The shop** at Combat = KH1, Sora's story: a list with rows **Buy**, **Sell** and **Card Packs** (owner: "A 'Card Packs' row": the third row opens Re:CoM's pack screen as before). Buy lists the floor's items with their prices. Sell lists the items Sora has, with the count and the sell price. The 8a.3 "Items" tab goes.
  - **Stock by floor** (Re:CoM's tiers): 1F Potion, Ether, Tent; 2F–6F also Hi-Potion and Camping Set; 7F and up also Elixir.
  - **Buy prices** (Moogle Points, owner): Potion 10, Hi-Potion 30, Ether 40, Tent 100, Camping Set 150, Elixir 200. **Sell prices**: KH1's ratio (sell = buy ÷ 5: KH1's 12 / 60, 30 / 150, 40 / 200) on the owner's buy prices, so nothing sells for more than it costs: Potion 2, Hi-Potion 6, Ether 8, Tent 20, Camping Set 30, Elixir 40. (Ours; KH1's own sell prices would exceed the new buy prices.)
  - **Removed at KH1**: Mega-Potion, Mega-Ether, Megalixir (shop, drops, Items list).
  - **Tent / Camping Set**: new items. Used from the KH1 camp Items page outside battle: Tent = full HP, Camping Set = full HP and MP (owner: "Camp-only full heal"). Descriptions say Sora, not the party.
  - **Drops**: Re:CoM-only enemies (no KH1 twin) take the drop table of a similar KH1 enemy (owner: "Nearest KH1 table"; the pairings to be listed for the owner).
- **Camp menu at KH1** (owner, 2026-11-04, verbatim: "in the "KH1" menu, bring the "Combat" and "Movement" sections into the "Settings" menu. When combat is set to "KH1," Replace "Review Decks" with "Items" from the "KH1" Menu When KH1 combat is selected, the menu buttons should look like this: Items Abilities Customize World Map Map Cards World Cards Status Journal Settings. When ReCoM combat is selected, the menu buttons should be default. This should remove the "KH1" section on the camp menu, and if you need to make more space for the buttons, make it work"; answers):
  - **Settings** opens a short list in Sora's story at both Combat settings: **Game Settings** (Re:CoM's own page, unchanged), **Combat** and **Movement** (our pages). Owner: "Settings opens a short list".
  - **Riku's story**: Re:CoM's default menu, with no Combat or Movement.
  - The "KH1" top entry goes. At Combat = KH1 the top list is the owner's nine entries; at Combat = Re:CoM it is Re:CoM's seven, with Settings opening the short list.
- **Enemy Skills plan answers** (owner, 2026-11-04, on `recom-enemy-skills.md`):
  - "Go ahead": build in 5 stages, stage 1 first.
  - Boss skills give the **skill only**, without the boss's elemental affinities.
  - **Wyvern (Reload Lock) is left out.**
  - **Softer numbers for the strongest permanent skills**: proposed to the owner before the stages that use them (2–4).
- **Treasure Magnet** (owner, 2026-11-04: "Re:CoM's Bouncywild radius"): with either copy equipped (Bouncywild's skill or KH1's level-72 copy), prizes are pulled from 10 m (Re:CoM's Draw: `player+0x550` = 10, plus the scattered-card test `0x14022F23A`). Without it, Re:CoM's 5 m. Built with Enemy Skills stage 2 (Re:CoM's play-time setters).
- **Enemy Skills numbers** (owner, 2026-11-04: "oogie- (every 6s) large body - (reduce damage from front half by 10%) platinum - (first 5 hits) build everything"). The rest of the proposal stands:
  - **Card Soldier** Attack Haste: swings ×1.25.
  - **Larxene** Dash: run ×1.25.
  - **Guard Armor** Wide Attack: hit size ×1.5.
  - **Wight Knight** Float: Sora only, gravity ×0.75.
  - **Oogie Boogie** Regen: every **6 s**, heal max(1, missing HP ÷ 8).
  - **Neoshadow** Bio: every 1 s.
  - **Defender** Protect / **Ursula** Shell: −25 %.
  - **Large Body** Guard: damage from the **front half reduced by 10 %** (not a block).
  - **Xemnas** Quick Barrier: 1 s.
  - **Roxas** Double Strike: second hit at half power.
  - **Lexaeus** Warp Break: 15 %, never bosses.
  - **Zexion** Confu-Strike: 10 %.
  - **Demyx** Water Charge: ×1.75.
  - **Platinum Card** Invincible: the **first 5 hits** each battle do no damage.
  - **Vexen** Auto-Life: once per battle, ¼ HP.
  - Every other skill keeps Re:CoM's numbers.
  - "Build everything": stages 2–5.
- **Moogle shop redesign at KH1** (owner, 2026-11-04, verbatim: "the item shop looks REALLY bad. … I was expecting an item list that looks like screenshot 3 (not exactly mind you) But you can keep how the original menu looks, without the "buy, sell, card packs" Ideally, selecting the left side (usually to buy card packs) it should bring up a menu similar to screenshot 4, with all "card packs" replaced with the item card graphics of each item sold. I'm not sure how to re organize screenshot 5 (the sell page,) but Ideally i would like to keep the feel of the page, with the functionality to sell items of course"; answers). This replaces the 0.103.0 Buy / Sell / Card Packs list:
  - **The two-Moogle menu** (Re:CoM's mode 3) stays as Re:CoM's; the text list goes.
  - **Buy** (left Moogle, Re:CoM's pack screen): an **Items tab first, then Re:CoM's 4 pack tabs** (owner: "Items tab, then packs"). The Items tab's cells show Re:CoM's own item card faces with prices.
  - **Sell** (right Moogle, Re:CoM's Card Trade-in screen): an Items tab beside Re:CoM's card tabs, keeping the page's feel. Re:CoM's count window picks how many; items come **from the stock first, then the quick slots** (owner: "Count window, stock then slots").
  - **Tents and Camping Sets are removed from the game** (owner: "just remove tents and camping sets from the game"), which leaves 4 shop items (Potion, Hi-Potion, Ether, Elixir): one Items tab of 4 cells. (The owner had picked "Two tabs" for 6 items; with 4 items one tab is enough.)
- **Customize page and long names** (owner, 2026-11-04, verbatim: "when something is bound to a shortcut, they should also be listed by the button prompt, as well as the list on the right Also on the customize page, I want a toggle that lets you change whether sleights or summons are on the first page of the command menu by default. … lets sora use sleights outside of battle. Secondly, we need a solution for certain sleights having too long of a name to fit within the black box."; answers):
  - The Customize page's three shortcut rows show the bound spell's or sleight's name beside the button glyph, as well as the list on the right.
  - ~~A Customize row toggling Summon / Sleights for the 4th row~~: **dropped** (owner, 2026-11-04: "disregard the toggle for sleights and summons"). D-pad Left stays the only switch.
  - **Long names squeeze to fit** (owner: "Squeeze to fit"): only a name too long for its box is drawn narrower so it ends inside the box; short names are unchanged.
  - **Sleights outside battle**: wanted. Research first (Re:CoM's field has no sleight action), then options to the owner.
- **Sleights list and ability section** (owner, 2026-11-04, verbatim: "The same thing for the sleights menu. I think we should have also have "sleights" have its own menu in the ability list"):
  - Squeeze to fit also covers the command menu's Sleights list (e.g. "Firaga Burst", "Homing Blizzara").
  - The sleight abilities get their own **"Sleights"** section in the KH1 ability list, like "Enemy Skills", with the same equip rules, AP and order.
- **Sleights outside battle** (owner, 2026-11-04, on `recom-field-sleights.md`):
  - **(a) Plain field strike**: Re:CoM's own sleight code runs in the field; a hit on a field Heartless is Re:CoM's field strike (the battle starts with first strike; the sleight is cut short).
  - **Free in the field**: no MP for a sleight used outside battle.
  - **Re:CoM's special field reactions kept, for sleights and spells**: Thunder Raid paralyses, Teleport freezes (and the others Re:CoM has); KH1 Thunder and Stop in the field get Re:CoM's paralyse / freeze too, instead of starting the battle at once.
  - **Which**: all sleights, the KH1 specials (Sonic Blade, Strike Raid, Ars Arcanum, Ragnarok; this lifts "only in battle") and the LV3 summons.
- **Command menu pages and Party** (owner, 2026-11-04, with a KH2 FM recording, verbatim: "I like our command menu, but I think it should have the pointing arrow that follows your selection on the command menu so people know that you can press left on the D Pad to switch between the two pages. Also, Page 1 (the default) should be Attack, Magic, Items, Sleights. Page 2 should be Attack, Summon, Party, Sleights The "Party" section should have "Magic LV3" stylized as "Donald Magic" "Stardust Blitz" "Goofy Tornado LV3" stylized as "Goofy Tornado" "Goofy Smash" "Wild Crush" "Lucky Bounty" "Sandstorm LV3" stylized as "Sandstorm" "Surprise! LV3" stylized as "Surprise!" "Spiral Wave LV3" Stylized as "Spiral Wave" "Hummingbird LV3" Stylized as "Hummingbird" and "Furious Volley LV3" Stylized as "Furious Volley" These friend sleights should ONLY appear if the friend cards drop in their respective battles. (You shouldn't be able to use sandstorm in traverse town for example) Also, if any of these sleights require an additional unlock requirement, they shouldn't show up until that requirement is met. Lastly, friend sleights should be able to be put in the shortcuts."; answers):
  - **Pages**: page 1 (default) Attack, Magic, Items, Sleights; page 2 Attack, Summon, Party, Sleights. D-pad Left switches pages (replacing the old Summon / Sleights 4th-row switch).
  - **Pointing arrow** like KH2's: beside the selected row, following the cursor, blinking as KH2's does. **Re:CoM's own arrow art** (owner: "Re:CoM's own arrow").
  - **Party list**: Re:CoM's friend sleights, LV3 where Re:CoM has levels, under the owner's names: Donald Magic, Stardust Blitz, Goofy Tornado, Goofy Smash, Wild Crush, Lucky Bounty, Sandstorm, Surprise!, Spiral Wave, Hummingbird, Furious Volley.
  - **Shown only where that friend's card can drop in this battle**, for the **whole battle** (owner: "Whole battle, if possible"); and only once any extra unlock Re:CoM requires is met.
  - **Cost**: the same MP as sleights (owner: "Same MP as sleights").
  - **Shortcuts**: Party sleights can be put in the shortcut slots.
- **M13 and camp text answers** (owner, 2026-11-04):
  - Stun Impact's effect 0 and Zantetsuken's effect 5 (placed at the struck enemy in KH1): **one small start call of ours at the target** when the move starts; Re:CoM plays and ends it (owner: "Start at the target").
    - **Correction (2026-11-04, Checked: KH1 decomp + files):** the question's premise was wrong. KH1 links both effects to **Sora** (`0x1402AE160(Sora, id | 0x800)`, code 0x8003 resolved on Sora's own body record: 0.85 m up, following him), not to the enemy. So 1:1 is a plain EET list-A event on Sora (0.85 m up), with no run-time code of ours; built that way, and the owner was told.
  - The six KH2 Keyblades swing with **Re:CoM's trail** at KH1 Effects.
  - The Effects / Sounds (later Voices) toggles go on a **new Settings page** (e.g. "Effects & Sound").
  - Camp-page long names (Customize, Abilities, Re:CoM's text): **(a) true squeeze**, an x-scale taught to Re:CoM's text drawing.
  - Decided without asking (no change to play): each Keyblade's KH1 set loads with its own Keyblade group, only when equipped; trail length follows KH1's own trail data (1:1), not an approximation.
- **Party and field follow-ups** (owner, 2026-11-04):
  - **Lucky Bounty** at KH1 brings a **random KH1 item** (Potion, Hi-Potion, Ether, Elixir by the floor's tier) instead of a deck card (the deck is empty at KH1).
  - **Party MP**: keep 2 each, Wild Crush 3, Lucky Bounty 1.
  - **KH1 Gravity in the field**: strikes field Heartless (Re:CoM's field strike: the battle starts with first strike).
  - **While a sleight runs in the field**: the camp is blocked, and a room-dimming sleight's i-frames apply in the field too.
- **Gravity** (owner, 2026-11-04, verbatim: "Gravity spell is too small. Also when the enemy is flattened from gravity, sora is unable to attack them, lets fix that"): the KH1 Gravity spell's visual is sized to KH1's; a flattened enemy stays hittable by Sora, as in KH1.
- **Gravity and Lucky Bounty answers** (owner, 2026-11-04, verbatim: "1 1-2 items 2 yes 3 make it the size of re:com gravity"):
  - **Lucky Bounty** brings **1–2 KH1 items** per find.
  - **KH1's own hit flatten at Damage = KH1**: build it (a Gravity-hit enemy flattens as in KH1).
  - **Gravity's sphere is drawn at Re:CoM's Gravity size** (owner's choice over KH1's; the hit radius stays KH1's unless the owner says otherwise).
- **Process** (owner, 2026-11-04, verbatim: "Don't stop anything unless I explicitly say so"): never stop running work (agents, builds) unless the owner explicitly says so. "Re start the shop and enemy skills": restarted from testing at 0.122.0.
- **Shop sell screen and Enemy Skills answers** (owner, 2026-11-04):
  - **Selling quantity: (c) our own Up/Down count step** on Re:CoM's yes / no (1..owned; the points box shows the total). Re:CoM's card trade has no count window (its 0–9 box is the card's ten values: Checked, decomp).
  - **The sell screen's Items tab**: the same green bottle icon as the buy screen's, placed first; help line "Sell items".
  - **Ansem (Sleightblind)**: keep Re:CoM's half damage from Fire, Blizzard and Thunder (an exception to "boss skills give no affinities").
  - **Black Fungus**: picks from every built skill, learned or not (Re:CoM's way).
- **Shop text, lock-on and camera** (owner, 2026-11-04, verbatim: "The moogle shop text is over extending past the text bubbles. Let's have the lock on button be mapped to R1 if combat is set to KH1. If on mouse and keyboard it stays the same. Lets also have an option in the config to switch between Re:CoM camera, KH1 Camera, And KH2 camera."):
  - The Moogle's speech-bubble lines must fit the bubble (e.g. "Restores HP. 10 Moogle Points. Sora has 11" overflows). Decided: wrap onto the bubble's lines as Re:CoM's own two-line lines do; squeeze only if a line still can't fit.
  - **Lock-on on R1** at Combat = KH1 on a controller; keyboard and mouse keep Re:CoM's binding.
  - **A Config option: Re:CoM / KH1 / KH2 camera.** Research both KH1's and KH2's cameras (decomps) and Re:CoM's camera system first; plan with the owner before building.
- **Camera answers** (owner, 2026-11-04, on `recom-m12-camera-plan.md` § 10):
  - **Independent** of Combat: a new Settings > **Camera** page, row Re:CoM / KH1 / KH2, default Re:CoM (Q1 (a), Q2 (a)).
  - **Sleights, summons and specials keep Re:CoM's camera moves**, then hand back (Q3 (a)); **boss fights keep Re:CoM's boss cameras** (Q10 (b)).
  - **An "Auto camera: On / Off" row** for KH1 and KH2 (Q6 (b)).
  - **KH2's numbers scaled ×0.89** for KH1's Sora (Q9 (b)).
  - Taken as recommended or 1:1 without asking (owner may change): after an event the KH1 / KH2 camera re-places behind Sora as those games do (Q4 (b)); Re:CoM's inversion rows drive KH1 / KH2 with each game's own "Normal" (Q5 (a)); KH1's / KH2's own speeds, no sensitivity row (Q7 (a)); KH1's built-in default numbers everywhere (Q8 (a)); Riku's story keeps Re:CoM's camera (Q11 (a)); KH1 / KH2 lock camera in field rooms too (Q12 (a)); KH1's mouse rule on keyboard and mouse (Q13 (a)).
- **Lock-on rules and camera walls** (owner, 2026-11-04, verbatim: "1. B 2. dont worry about iit"):
  - **KH1's lock rules on R1** at Combat = KH1 (controller): R1 again unlocks at once; L2 steps through targets (KH1's). No hold-style Config row.
  - Button prompts: leave as they are.
  - KH1 Auto camera's near-wall ramp: **(a) 8 + 8 flat 2.5 m rays** with Re:CoM's sweep, only while the swing runs.
- **Ability list order and "Equipped" label** (owner, 2026-11-05, with a recording, verbatim: "When activating an ability it automatically puts it all the way at the bottom of the list. also, instead of equipped items/abilities saying "new" when they're equipped, can they just say "equipped""):
  - Equipping (or removing) an ability keeps the list's order; the cursor stays on the row.
  - Equipped abilities and items show **"Equipped"** where they now show Re:CoM's "NEW" badge.
- **GitHub install through OpenKH** (owner, 2026-11-05, verbatim: "I also want people to be able to install the mod from openKH by using the GitHub"; answers):
  - Players install with OpenKH Mods Manager's GitHub install (`PM2CRAZY/KH1-in-ReCom`), from the default branch **`testing`** (owner: "Yes, from testing"); updates come with OpenKH's update.
  - The game-made files are never in the repo: **the DLL builds them on first launch** from the player's own KH1 and Re:CoM files (owner: "DLL builds on first launch"), and again after an update changes the asset step.
- **First-launch build progress** (owner, 2026-11-05, verbatim: "I definitely want a progress screen in the game window while everything loads"): the first-launch asset build shows its progress **in the game window**, drawn by Re:CoM's own 2D / text, not a console window; the game waits until the files are ready.
- **Release build** (owner, 2026-11-05, verbatim: "make a seperate github page for just the release build. only update the release build when i say so. Bring this latest version into the release page"; answer: "`main` branch here"):
  - The release lives on this repo's **`main`** branch, holding only the release files: `mod.yml`, `openkh/kh1-recom.dll`, `README.md`, `.gitignore`, `VERSION`. OpenKH's GitHub install takes `main` first when it has a `mod.yml` (Checked: OpenKH source, `docs/openkh-github-install.md`), so `PM2CRAZY/KH1-in-ReCom` installs the release; existing installs cloned from `testing` keep updating from `testing`.
  - **This is the owner's one exception to "only `testing`"**: `main` is written **only when the owner says so**, only with release files (no source, no game files), never force-pushed. All work still happens on `testing`.
  - First release: 0.142.0.
- **Sora's damage cap** (owner, 2026-11-05, verbatim: "lets add a damage cap to sora"; answers): a cap on the **damage Sora deals** per hit, by **KH1's own rule** (research it and tell the owner the numbers before building), at **Combat = KH1 only**.
- **Damage cap answers** (owner, 2026-11-05; research in `/home/claude/scratch-dmgcap/notes.md`, to be written up in the docs with the build):
  - KH1's cap is the **enemy's per-hit cap** (its stat record `+0xC2`; floor `+0xC0`), applied after power and the damage table, before resistance (Checked: KH1 `0x1402BFA80`). Re:CoM has none.
  - **(b) Same share of HP:** at Damage = Re:CoM, cap = KH1 cap × Re:CoM max HP ÷ KH1 max HP (twin, proxy or boss-twin record); at Damage = KH1, KH1's raw cap. % of HP hits (Gravity, Gravity Break, Zantetsuken) are never capped, as in KH1; resistance still applies after the cap, so weaknesses can go past it, as in KH1.
  - **Covers:** swings, specials, spells, Sleights-list sleights, LV3 summons and Party (friend) sleights.
  - **Re:CoM's extras stay under the cap** (weak parts, the frozen +100, Back Attack, Enemy Skills multipliers), as KH1 puts every bonus before its cap.
  - Combat = KH1 only.
- **Release repository** (owner, 2026-11-05, verbatim: "Can you actually just put the release version of the mod into a seperate github repository?"; answers): the release also lives in its own repository, **`PM2CRAZY/KH1-Combat-in-ReCoM`** (branch `main`; the owner created it), and **`main` in this repo is kept as well** (owner: "Keep main as well"). Both get the same release files, updated **only when the owner says so**, never force-pushed. Releases: 0.142.0; 0.143.0; 0.143.1 (2026-11-05, both copies).
- **Open questions round** (fork owner, 2026-11-23, fork branch `Re-Re-Reverse/Rebirth`; the questions were gathered from
  `ROADMAP.md` § Owner questions waiting, `recom-kh1-hud.md`, `recom-m7-command-menu.md`, `recom-m13-plan.md` § Open and
  `recom-m6-battles.md` § Marluxia 3's card duel; answers verbatim in Portuguese, then what they mean):
  - **Marluxia 3's card duel at Combat = KH1** (verbatim: "pode ser um duelo usando padrões do Marluxia dos outros jogos
    caso necessário, e de forma balanceada!"): a duel Sora can win without cards, built from Marluxia's patterns in his
    other games where needed, balanced. **Research and a design spec first** (which games, which patterns, Checked from
    their files / decomps where available); no code before the fork owner approves the spec.
  - **Re:CoM's HUD extras** (minimap, room plate, EXP popup) at HUD = KH1: **stay hidden** (2a).
  - **Donald's and Goofy's rings** (Re:CoM's friends have no HP / MP): **hidden** (4b; changes what is built).
  - **KH1's orange auto-target ring**: **shown** (5a).
  - **EXP / munny popups and the level-up box**: **build KH1's popups** (6b; Re:CoM counts EXP at the battle end, so this
    needs research first).
  - **Command menu row 3 before the first summon**: KH1's greyed **"–"** (7a, as built).
  - **Near NPCs**: **KH1 1:1**, Talk in row 0 and Attack dimmed with the error buzz (8b; changes what is built).
  - **Context tab in Re:CoM's look**: the **teal-green tint** (9a, as built).
  - **Effects = KH1 with Sounds = Re:CoM**: KH1's effects **keep their own sounds** (10a, as built).
  - **KH1's room dust and splashes**: **add them** (11b; M13, research first).
  - **KH1's clash EXP**: **add it** (12b; M13, research first).
  - **Re:CoM's event hides at HUD = KH1**: the HUD goes **at once, as Re:CoM does** (3b; replaces the built 10-frame
    KH1 fade).
  - **KH1's hand effect when an item is used** (34 frames in): **none**; Re:CoM's own item effect stays at the heal
    (13a, as built).
