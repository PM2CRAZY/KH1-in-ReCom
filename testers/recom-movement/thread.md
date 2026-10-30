# Re:CoM: KH1 movement (milestone M4)

> [!tip] Assistant
> **What's new:** with **Combat: KH1** (camp menu → Config, or `combat = kh1` in
> `kh1-recom.ini`), Sora now moves like KH1's Sora, in the field and in battle rooms. Re:CoM still
> runs him: its own actions, animations, collision and camera. Only the rules and numbers are
> KH1's:
> - walk / run snap to KH1's two speeds (no speed-up), the stick turns his direction at once and
>   his body turns 11.25° a frame;
> - KH1's jump: a short crouch, the same height and arc, a shorter jump when you let go early,
>   a 12-frame landing pause (you can jump out of it);
> - High Jump, Glide, Superglide, Dodge Roll and Guard (the Guard pose only; blocking comes with
>   the Attack, next milestone).
>
> Cards, the field swing and Re:CoM's ledge climbing still work as before. With **Combat: Re:CoM**
> everything is Re:CoM's.
>
> **Abilities** are switched in `kh1-recom.ini` for now: `high_jump`, `glide`, `superglide`,
> `dodge_roll`, `guard` (on / off). By default only Dodge Roll and Guard are on. With both on,
> Square while standing or walking is Guard, while running it's Dodge Roll (KH1's rule).
>
> **Please check, in order:**
> 1. **Log.** `kh1-recom.log` should list `movement: KH1 handlers in Re:CoM's actions…` and
>    `trace: press F9…`, and end with `result: all checks passed`. If the asset step has run, it
>    also lists the KH1-only clips it found (High Jump, Guard). Send the log.
> 2. **Walk and run** in a field room: small stick tilt walks, more than 80% runs. Turn sharply:
>    Sora should change direction at once and his body follow quickly.
> 3. **Jump**: tap and hold jump, then tap and release quickly. Land while holding the stick:
>    there's a short pause before he runs off. Press jump during that pause.
> 4. **Walk off a ledge** if a room has one.
> 5. **High Jump / Glide / Superglide**: turn them on in the ini, restart, try them. Glide: jump,
>    then press and hold jump in the air. Superglide: Square in the air, hold it.
> 6. **Dodge Roll and Guard**: Square while running (roll) and standing (Guard).
> 7. **A battle room**: the same moves, plus check that cards still play and battles can be won.
> 8. **Combat: Re:CoM**: everything should be exactly Re:CoM's again.
>
> **Recording a trace (F9):** press F9 in game to start, do one move (for example: stand still a
> second, run forward, jump, land, stop), press F9 again to stop. Each recording is a new
> `kh1-recom-trace-<n>.csv` next to the game exe. Short recordings of one move each are best. Put
> them in this folder with a line saying what you did. If you can, record the same move in KH1
> with `kh1-trace` too: I can then compare them frame by frame.
>
> **1:1 check status:**
> - Checked (tests): the KH1 code inside Re:CoM's actions matches the KH1 rewrite frame for frame
>   (position, facing, state, landing / Guard / roll locks) for walk, run, turns, jump, High Jump,
>   the short jump, landing, the land-jump, walking off a ledge, Glide, Superglide, Guard, and
>   uneven frame times. KH1's numbers come from KH1's exe and your KH1 files.
> - Known difference: during the Dodge Roll, Re:CoM moves Sora one frame earlier than KH1 (same
>   distance, same end point). I've listed ways to fix it for the owner.
> - Not checked: anything in game yet. The traces will show whether Re:CoM's collision, floors and
>   slopes give the same positions as KH1, and whether the stick reads the same on your pad.
