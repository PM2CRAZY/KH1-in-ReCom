# AGENTS.md: KH1 combat in Re:Chain of Memories

Read first, every session:

1. `docs/design-philosophy-instructions.md`
2. `docs/engineering-rules.md`
3. `docs/tester-requests.md` (its rules win if they differ)
4. The top ~15 stages below.

Work only on branch `testing`. Stages are newest first.

---

## Stage 0: groundwork (2026-10-29)

> "Where I'd start: a cardless Attack, built into Re:CoM's own battle code. Before that comes a
> short groundwork step (step 0)."

No code. `VERSION` not created yet; the first change under `crates/` starts it.

- Repo seeded: these rules files, `docs/sources.md`.
- Re:CoM Steam decomp made (see `docs/sources.md`). Checked (exe): SHA-256 `acf42e96…6bbf`.
- Finding: Re:CoM's exe is mixed-mode C++/CLI; the .NET part is only the shell. Checked (exe).
- Finding: the KH1 notes in KH2-Combat-in-KH1 use Epic addresses; convert to Steam before use.
  Checked (files).

Step 0 still to do (notes in `docs/` only):

1. Map Re:CoM's player state table, card/command dispatch, motion request and blend, attack
   and hit data, damage formula, and battle-entry transition.
2. Confirm both games' tick rates and how each steps animation.
3. Plan the trace tool with the tester: the same fields from both games (tick, state, motion,
   frame, position, velocity, hit events).
4. Carry the KH1 notes over, converted to Steam addresses.
5. Check the assumptions: Re:CoM built on KH2's engine (`.mset`, motion code); whether its
   player state machine is table-driven like KH1's.
