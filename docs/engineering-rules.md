# Engineering rules

## The one hard scope rule

Work only on repo `PM2CRAZY/KH1-in-ReCom`, branch `testing`.

- Never commit to, push to, merge into or open a PR against any other branch.
- Never rebase or force-push `testing`.

## Read first, every session

1. `docs/design-philosophy-instructions.md`
2. `docs/engineering-rules.md` (this file)
3. `docs/tester-requests.md`. Its rules win if they differ from this summary.
4. The top ~15 stages of `AGENTS.md`.

## Versions and builds

- Any change under `crates/` is a version: bump `VERSION`, and add a dated `Stage <n>` entry at
  the top of `AGENTS.md`. The entry quotes the ask and gives the addresses, the change, and
  Checked / Assumption tags.
- Findings go into `docs/` as plain notes: the address, what it does, and a tag
  (`Checked (exe / decomp / files / trace)` or `Assumption`).
- Run tests and the Windows type-check before pushing. After a push, wait for the build bot's
  commit, then pull with `--rebase`.

## Addresses and sources of truth

- Steam addresses only, for both exes. New hooks check the function's exact first bytes (at
  least 5, read from the Steam exe).
- KH1 logic: the KH1 Steam decomp, plus the KH1 findings in KH2-Combat-in-KH1's `docs/` (notes
  only; re-check any address you depend on). **Those notes write addresses as Epic-build RVAs**
  (see its `docs/steam-epic-addresses.md`): convert each one to its Steam address and check it
  in the KH1 Steam decomp before using it.
- Re:CoM internals: the Re:CoM Steam decomp.
- Constants, tables and prologue bytes: the Steam exes. Check each value against the decomp
  before trusting it.
- What the tester sees: their traces and logs are ground truth.
- Mark every assumption about how either game works, and check it. Never present a guess as
  fact. If a source isn't in your session, ask for it.
- Never put decompiled code or game files in the repo.

Where the decomps live (on the owner's PC, outside any repo): see `docs/sources.md`.

## Don'ts

- Don't change anything outside `testing`.
- Don't write code without a go-ahead.
- Don't add requests nobody wrote in the files.
- Don't leave a waiting thread unanswered.
- Don't ship a running-along system without asking.
- Don't present a guess as fact.
