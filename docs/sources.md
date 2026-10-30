# Sources

Where the sources of truth are. None of them are in this repo, and none may be added to it.

## Executables (Steam, `D:\Steam\steamapps\common\KINGDOM HEARTS -HD 1.5+2.5 ReMIX-\`)

| Game | File | Bytes | SHA-256 | Tag |
|---|---|---|---|---|
| Re:CoM | `KINGDOM HEARTS Re_Chain of Memories.exe` | 9,665,808 | `acf42e96a168c73301f5c2cc2915d07e6c420f56a59c6286655c970bfee36bbf` | Checked (exe) |
| KH1 FM | `KINGDOM HEARTS FINAL MIX.exe` | 7,490,832 | `d790746245d26159f3ee0e1060e33b2fa2de06941850a4ac724f598722884bac` | Checked (decomp README; size checked against exe) |

Image base for both: `0x140000000`. Ghidra addresses = `0x140000000 + RVA`.

## Decomps (owner's PC, `source\repos\kh2-combat\local\decomps\steam\`)

- `kh1\`: KH1 FM Steam, Ghidra 12.1.4, 15,897 functions. Made 2026-10-23 for KH2-Combat-in-KH1.
- `recom\`: Re:CoM Steam, Ghidra 12.1.4 (headless, full auto-analysis, default analyzers, no
  PDB), 23,981 functions, 0 failed or timed out, 12,861 strings. Made 2026-10-29. Same layout as
  `kh1\`: `<start>.c` in 0x10000 buckets, `functions.tsv`, `strings.tsv`, `failures.tsv`,
  `export.log`. Each function header lists callers and data refs.
- The export script (`ExportDecomp.java`) and `README-recom.txt` sit in `decomps\`.

## Notes on the Re:CoM exe

- **Mixed-mode C++/CLI image.** Checked (exe): it carries .NET metadata, 487 methods, 369 of
  them IL. The IL is the shell only: the WinForms window, the launcher/settings form
  (`KHCForm`), file, texture and setup helpers (`Axa.*`). Gameplay code is native, and the
  Ghidra decomp covers it. Ghidra does not decompile the IL methods. Use an IL decompiler only if
  the shell matters.
- One IL method is named `_loadBattleInfo`. Not looked at yet; check it when mapping
  battle entry. Assumption: it relates to battle setup.
- Re:CoM's strings include `_mset` (at `0x14064F40C`), but no code uses it. Checked (decomp).
  The motion system is not KH2's: see `recom-combat.md`.

## KH1 notes from KH2-Combat-in-KH1

- Its `docs/` hold many KH1 findings. Its addresses are **Epic-build RVAs**, converted to Steam
  through its `steam_map.in` table (`docs/steam-epic-addresses.md` there). Checked (files).
- So no address from those notes is used here until it has been converted to Steam and found in
  the KH1 Steam decomp.
- `0x14029F7C0` (named as KH1's motion request in the 2026-10-29 plan) does not appear in those
  docs. Checked (files). Find the motion request in the KH1 Steam decomp directly.
