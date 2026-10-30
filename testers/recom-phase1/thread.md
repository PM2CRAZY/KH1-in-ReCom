# Re:CoM: first launch, Config menu rows, KH1 motions test (milestones M2 + M3)

> [!tip] Assistant
> **What's installed** (in your Panacea mod folder, `Recom\`):
> - `dll\kh1-recom.dll`, the mod.
> - `PLAYER\0001\PL0001.ANN`, Sora's motion pack with KH1's 118 motions added as new slots 161–278
>   (Re:CoM's own 161 are unchanged). It was converted from your own KH1 and Re:CoM files.
> - `PLAYER\0002\PL0002.ANN`, the Keyblade's pack with the same new slots (if present).
> - `kh1-motions.json`, the list of the new slots.
>
> **Please check, in order:**
> 1. **Launch.** Start Re:CoM normally, then look for `kh1-recom.log` next to the game exe. The
>    last line should say `result: all checks passed`. Send the log either way.
> 2. **Config menu.** Open the camp menu → Config. There should be two new rows under Re:CoM's
>    own: **Combat: Re:CoM / KH1** and **KH1 motions test: Off / On**. Check that:
>    - left/right changes them;
>    - up/down moves through all rows and wraps;
>    - closing and reopening Config keeps the values (they're saved in `kh1-recom.ini`);
>    - Re:CoM's own rows still work as before.
>    A screenshot helps. If anything looks wrong, set `config_menu = off` in `kh1-recom.ini` to
>    remove the rows.
> 3. **KH1 motions test.** Set it to **On**, then go through a door (it applies on the next room
>    load). Sora's idle, walk and run, and his first battle swings, should now use KH1's
>    animations at 60 fps. Turn it **Off**, change rooms, and Re:CoM's own should come back.
>    Everything else, including timing and cards, is still Re:CoM's in this test.
>
> **1:1 check status:**
> - The converted motions match KH1's poses to ≤0.045 mm and ≤0.005° per 60 Hz frame. Against
>   Square Enix's own conversions of the same motions they match to ≤0.019°. Checked (files,
>   tests).
> - Not checked in game: how the rows look and behave, the clips playing in Re:CoM, and the
>   Keyblade following them.

> [!tip] Assistant
> **Update (0.4.1), after your Config screenshot and the animation decision:**
> - The Config screen now has one new row, **Combat: Re:CoM / KH1**. The motions-test row is gone:
>   Sora uses Re:CoM's own animations, plus KH1's for the 69 motions Re:CoM doesn't have.
> - Re:CoM's window panel should now reach below **Combat**. Please send a Config screenshot, and
>   check the help line at the bottom with the cursor on **Combat**.
> - Your screenshot showed the rows drawing correctly with Re:CoM's own buttons and highlights.
>   The panel being one row short is what this update fixes.
>
> **1:1 check status:** the matching of KH1 motions to Re:CoM's clips was checked against both
> games' files. The panel fix hasn't been seen in game yet.
