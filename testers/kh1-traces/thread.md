# KH1 traces for the 1:1 check (milestone M1)

> [!tip] Assistant
> **What this is for:** the KH1 Rust rewrite (the playground) has to match real KH1 frame for
> frame before we build Re:CoM's version on it. This needs short recordings of real KH1 FM (Steam).
> A small logger DLL records Sora's state, motion, position and your inputs every frame. The
> playground then replays your exact inputs and compares the results.
>
> **Setup (once):**
> 1. Copy `kh1-trace.dll` (in the game folder's `kh1-playground\` folder) into your Panacea mod
>    folder's `kh1\dll\` folder.
> 2. **Move `kh2style.dll` out of `kh1\dll\` while recording.** It's the KH2-combat mod and takes
>    Sora over, so the recording wouldn't be real KH1. Put it back afterwards.
> 3. Start KH1. `kh1-trace.log` appears next to the game exe and should say the hook went in.
>
> **Recording:** **F9** starts and stops. Each start makes a new `kh1-trace-<n>.csv` next to the
> exe. For each move: stand still on flat, open ground for about a second, press F9, wait half a
> second, do the move, wait until Sora is idle, press F9. Keep the command menu on **Attack**. For
> combos, lock on before pressing F9.
>
> **Please record (one file each):**
> 1. Walk a slow circle (half tilt), then run a circle (full tilt), then stop.
> 2. A full jump (hold the button), then a short jump (tap).
> 3. With High Jump equipped: the same two jumps.
> 4. Glide: jump, then hold jump until you land (Superglide too, if you have it).
> 5. Dodge Roll from a run.
> 6. Guard from standing.
> 7. A 3-hit ground combo on an enemy at touching distance, locked on.
> 8. The same combo starting a few steps away (about 250 units, the lunge range).
> 9. An air combo next to the enemy: jump, then three Attack presses, locked on.
>
> Then put the CSV files and `kh1-trace.log` in this folder (`testers/kh1-traces/`), or tell the
> owner where they are.
>
> **1:1 check status:** none yet. That's what these recordings are for. The logger itself hasn't
> run in KH1 yet, so `kh1-trace.log` is the first thing to check.
