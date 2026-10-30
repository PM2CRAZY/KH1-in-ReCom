# Re:CoM combat: motion, Keyblade hits, damage

Steam addresses. Tags: Checked (decomp / exe) means read; Assumption means inferred.

## Motion

- **Request.** The player wrapper `0x1401F7180(player, actionId, blend, rootFlag, …, start)`
  maps the action through a per-character table (`0x1407C6300[player+0x398]`, 16-byte rows
  `{name, index}`; Sora's table is `0x1407C5E80`, names like `p001_055`), then calls the actor's
  vtable `0x70` = `0x1401A3470(actor, idx, start, root, blend, …)`. Checked (decomp + disasm).
- A blend below 0 means "the clip's default blend". The motion index also drives the actor's
  effect track (`CEffectMotion`, `actor+0x108`) and sound track (`CSEMotion`, `actor+0x118`).
  Checked.
- **Controller** at `actor+0xD8`: `+0x30` clip, `+0x38` motion index, `+0x3C` time (units),
  `+0x48` duration, `+0x4C` speed (float), `+0x50` end/loop events, `+0x54`/`+0x58` blend
  total/remaining. Checked.
- **Clip header:** `+0x12` frame count, `+0x14` loop-start frame, `+0x18` default blend in
  frames (× 50), `+0x1A` fps byte. Checked.
- **Advance.** `0x14024E470` adds `dt × speed` to the time each update, loops or clamps at the
  end, and blends by remaining/total. Gameplay reads the frame as `time / 50`. Checked.
- Player-side speed `player+0x47C` (1.0; 1.5 or 2.0 in haste states). Checked.

### KH2 engine?

- No `.mdlx`, `.mset`, `.a.fm` or BAR strings. Sora's files are
  `PLAYER/0001/PL0001.{MDL,ANN,RTM,TXA,EET,SES,BIN}` (`.BIN` is the player camera settings;
  see `motion-conversion.md`). The one `_mset` string (`0x14064F40C`)
  has no code references. Checked (strings, decomp).
- Timing (3000 units per second), a per-clip fps byte and a name-keyed action table also differ
  from KH2.
- Conclusion: Re:CoM's motion system is not KH2's mset/anb system. The `.ANN` format is now decoded
  (`motion-conversion.md`), and Re:CoM's Sora skeleton and many clips turn out to be KH1's. Checked (files).

## Keyblade swing to hit

- **Attack row.** `0x1401F7520` (vtable `0x128`, run by initAttack) picks an attack id from the
  combo step and situation (tables `0x1405A1708`, `0x1405A3080`, `0x1405A30A8`). It sets
  `player+0x3C8` to the 16-byte row `0x1407C6C70 + id × 0x10`, plays its motion with blend 0
  and root motion, and advances the combo counter `+0x63C`. Checked.
- **Row layout:** `+0` action id (motion), `+2` hit-effect id (table `0x1405A1480`), **`+4`/`+5`
  hit window in 60 Hz frames (start ≤ frame < end)**, `+6`/`+7` homing/lunge window, `+0xA`
  card-power column, `+0xB` hit kind, `+0xD` lunge distance, `+0xF` flags. Checked.
  - Rows 1–3, read from the exe by the lead: active frames 12–26, 12–32 and 12–46. Checked (exe).
- **Power.** With no card (`+0x7F4 == 0`): AP = attack stat `+0x43C`, attribute 0. With a card:
  AP = stat × (card's power at column `row+0xA` + bonus `+0x664`) / 100, attribute from the card.
  The card table `BattleCardList` is loaded at `0x14087A808`. Checked.
- **Collision.** `0x1401F5DC0` ("procAttackCollision") calls `CPlayerWeapon::onAttack`
  `0x1402250B0` while the frame is in the window. It applies the Wide (`0x1402257C0`), Back
  (`0x140225320`) and Double-hit (`0x140225420`) modifiers. onAttack fills the weapon's
  `CCollAttack` at `weapon+0x240`: kind, AP, attribute, radius 0.5 (1.0 wide). `0x140225840`
  moves the volume each update from the previous to the current blade point. Checked; that it
  is a swept capsule is an Assumption.
- **Per target.** `0x14016F1C0` does the guard / reflect checks, otherwise writes the hit to the
  victim body, with a back-attack bonus when more than 90° behind. Checked.
- **Enemies** use `CAttackMotion` instead: hit timing, radius and bones per motion name, stored
  in the character's data file (resource type `0x24`). Sora's Keyblade uses the hard-coded row
  table. Checked.

## Damage

1. `0x140154EB0` takes the body's hit value and multiplies it by a weak-point float on flagged
   parts. Checked.
2. `0x14017F7E0` gets the resistance percent for the attribute from the enemy params
   (`enemy+0x308`; default 100), then calls `0x14017DC50`. Checked.
3. `0x14017DC50` handles special kinds (`0xD` percent of HP, `0x11` × 0.1, some kinds deal no HP
   damage). Checked.
4. `0x14017DAB0`: **`dmg = int(AP × resist × enemy.f290 / 100)`**, then `HP (enemy+0x274) -= dmg`,
   clamped to max HP. Checked (read by the lead).
   - `0x140180920` can veto the damage (`dmg = 0`). Not read. Assumption: scripted HP locks,
     which matters for bosses.
   - `enemy+0x290` is a float multiplier. For enemy ids 100–199 it drops 0.25 per hit after 5
     hits, down to 0.05. Checked. Assumption: boss combo decay.
- No enemy defense stat and no critical-hit roll on this path. Checked for these functions only.
- Not read: `0x140158DD0` (second hit of a double hit), `0x14015D5F0` (can rewrite AP).
