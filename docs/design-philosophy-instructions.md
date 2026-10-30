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

## Known open design questions (ask the owner, don't guess)

- **Battle transitions.** Re:CoM fights in a separate battle instance after field contact; KH1
  fights in the field. Keep the transition, remove it, or make it a toggle?
- **The deck at KH1 setting.** Removed entirely, kept for sleights or summons only, or a toggle
  per card type?
- **Enemy behaviour.** At KH1 setting, do enemies still play cards (and can Sora's attacks
  card-break them), or do they use plain KH1-style attack timing?
- **Card value of a cardless KH1 hit** (asked 2026-10-29, no answer yet): (a) no value, never
  breaks or is broken; (b) a fixed value; (c) a Config toggle between (a) and (b).
- **Party.** Re:CoM's friend cards summon Donald and Goofy temporarily; KH1's party is always
  present. Which, and per world?
- **Progression.** Re:CoM's level-up choices versus KH1's stat growth, AP and abilities.
