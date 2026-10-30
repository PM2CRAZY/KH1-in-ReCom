# Re:CoM: KH1 Attack, no cards (milestone M5)

> [!tip] Assistant
> **What's new:** with **Combat: KH1**, the confirm button is KH1's Attack, in the field and in
> battle rooms, with no cards. Re:CoM still runs every swing: its own attack action, its own swing
> animations (the same KH1 motions, decision 10), its trails, sounds and hit effects, its collision
> and its damage. KH1 decides the rest:
> - the ground combo (swing, second swing or lunge, finisher) and the air combo, picked by KH1's
>   rules: target range and angle, alternation, finisher;
> - KH1's hit frames and combo windows; no pause after a combo;
> - a press in the window is ignored if the last swing hit nothing (unless Combo Master);
> - KH1's Keyblade hit shape: two balls along the blade plus the sweep since the last frame;
> - hits land even while an enemy is playing a card (KH1's rule; the enemy's card is cancelled);
> - swings make enemies flinch, finishers knock them away (Re:CoM's own reactions).
>
> Abilities in `kh1-recom.ini`: `combo_plus` and `air_combo_plus` (0–9, each adds a swing) and
> `combo_master` (on / off). They're all off by default.
>
> **Also in this version:**
> - **Springs** (owner's 0.5.0 video): on Re:CoM's jump pads Sora now gets Re:CoM's own spring
>   jump, the full height, then back to KH1's rules on landing. Re:CoM's other gimmick moves
>   (footings and the like) were already left to Re:CoM.
> - **High Jump** uses Re:CoM's own higher-jump animation.
> - **Dodge Roll** moves on KH1's exact frames now (it was one frame early).
> - **Roll protection** is KH1's: 12 frames where hits don't stagger Sora. He still takes the
>   damage (KH1's roll has no invincible frames: I checked KH1's code).
>
> **Not in this version:**
> - KH1 damage numbers. Sora still deals Re:CoM's own damage. KH1's damage needs each enemy's KH1
>   stats, which are in KH1's files I haven't got (the log says "FALLBACK").
> - Guard's blocking and Counterattack, MP from hits, and the hop at walls.
>
> **Please check, in order:**
> 1. **Log.** `kh1-recom.log` should show `attack: KH1 Attack in Re:CoM's attack action…`, the
>    damage FALLBACK line, and `result: all checks passed`. Send it.
> 2. **Field:** swing at a Heartless on the map. The battle should start with Re:CoM's first-strike
>    stun, as before.
> 3. **Battle, ground combo:** tap Attack with no enemy close: two swings and a finisher. Then next
>    to an enemy, and from a few steps away (the lunge). Tap too early or too late in a swing and
>    the press should do nothing.
> 4. **Battle, air combo:** jump and attack. Swings in the air keep Sora floating; landing cuts
>    the swing.
> 5. **Hits during an enemy attack:** hit an enemy while its card shows. The hit should land and
>    stop its attack.
> 6. **Springs:** the pillar from the video. Sora should reach the upper ledge.
> 7. **Dodge Roll into an enemy attack:** you take the damage but don't stagger.
> 8. **Combat: Re:CoM:** cards, card breaks and sleights exactly as Re:CoM again.
>
> **Traces (F9):** one short recording per move: a 3-swing combo on a dummy-like enemy, an air
> combo, a roll. The trace now has the attack row, the combo counter, when the hit is live, and each
> hit's damage and the enemy's HP. If you can, record the same combo in KH1 with `kh1-trace`.
>
> **Options for you (owner):**
> - KH1 damage: KH1 keeps each enemy's HP and Defence in its enemy scripts, not in `btltbl.bin`.
>   (a) send me the KH1 file list (or the files) of one world, Traverse Town, so I can find the
>   Shadow's record; (b) keep Re:CoM's damage until the KH1 enemies (M10).
> - Sora's KH1 Strength (KH1's formula needs it): (a) KH1's level-1 Sora plus the Kingdom Key,
>   rising with Re:CoM's level-ups; (b) a fixed value in the ini for now.
>
> **1:1 check:**
> - Checked (tests, against the KH1 rewrite frame for frame):
>   - the ground combo, the missed-swing rule, the chain on a hit, the air combo, a landing
>     cutting an air swing, Attack and jump on the same frame, and the Dodge Roll (now exact);
>   - KH1's hit and combo frames come from your KH1 file (`xa_ex_0010.mset`);
>   - the Keyblade's size from `btltbl.bin`;
>   - Re:CoM's rows and every hooked address from the Steam exe.
> - Not checked:
>   - anything in game;
>   - whether Re:CoM's hit frame lands on KH1's frame (I moved it one frame earlier on a
>     reasoned guess; the trace will show it);
>   - Re:CoM's enemy shapes against KH1's blade;
>   - damage (not KH1's yet).
