# Design philosophy

A Rust mod (a DLL) that brings Kingdom Hearts 1's combat (Final Mix, PC, Steam build) into
Kingdom Hearts Re:Chain of Memories (PC, Steam build).

Every setting has a Config-menu toggle between **Re:CoM** and **KH1**.

- With every setting on KH1, the game must feel 1:1 with KH1.
- With every setting on Re:CoM, it must play exactly like Re:CoM.

These rules override any habit of the assistant.

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
    - Star Seeker, Monochrome, Follow the Wind, Hidden Dragon, Photon Debugger and Bond of Flame
      come from **KH2** (the player's KH2 files): KH2's model, Strength and Magic. Reach is
      measured from the KH2 model's blade length; radius, critical rate / bonus and priority are
      the Kingdom Key's.
    - Soul Eater uses **KH1 Riku's** Soul Eater model. Its numbers are still open.
    - Order: owning and equipping, then KH1 models, then KH2 models and stats, then Soul Eater.

## Known open design questions (ask the owner, don't guess)

- Revisit later: party, progression.
