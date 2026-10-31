# M6: battles at the KH1 setting (decisions 29, 30)

Steam addresses (Re:CoM image base `0x140000000`). Tags: Checked (decomp / exe / tests) or
Assumption. Code: `crates/kh1-recom` (`battle.rs`, `win/battle.rs`, `sites.rs` `M6_*`). Research:
`m6-cardless-battles.md` and `recom-m6-patch-points.md` (kept with the decomp tools). This note
supersedes `recom-cardless-battle-design.md` §8 ("CBTL is the only gate").

## Status

- **Stage 1 built (not versioned yet, not seen in game):** at Combat = KH1, in Sora's story,
  Heartless and Heartless bosses attack without cards, and several can attack at once, each as if
  alone.
- Stage 2 (no deck or deck HUD for Sora) and stage 3 (bosses one at a time, starting with the
  action-0xB check P5; human bosses' hidden move lists; Marluxia 3's stock folder hidden) are not
  built. Human bosses keep Re:CoM's card path exactly.

## Stage 1: what changes

### P1: the Heartless card play does message 2's work

Re:CoM's path for one Heartless attack (Checked (decomp)):

1. The think (vtable `+0x3D0`, mostly `0x140196EC0`) picks an attack state into `+0x278`, asks
   the gate `0x14017D010` and sets the request `+0x260 |= 0x200`.
2. The battle enemy manager's grant `0x140187600` turns one request per frame into `0x800`.
3. The enemy update `0x140185320` calls vtable `+0x98` = **`0x140198870`** (the card play) when
   `0x800` is set. It clears `0x802`, computes a card value (`+0x448`) and calls
   `0x1401634B0(CBTL, id, card, value)`, which refuses while CBTL is locked and otherwise sets a
   side-2 card (`0x140163830`) and its cue sprite. Success sets bit 0.
4. When the card is in place, CBTL posts message 2. The handler `0x140181620` case 2 clears
   `0x200` and, when this enemy owns side 2, sets `+0x260 = (& ~0x10) | 4` and the next state
   `+0x68 = enemy + (+0x278 + 0x13) × 0x40`; bit 29 follows the record's sleight id.
5. Re:CoM's state code runs the attack and ends it with `0x14017E540` (cooldown `+0x2C8`, bit 0
   cleared, CBTL slot freed if this enemy owns it).

Our wrapper on `0x140198870`, in a KH1 battle (decided at battle start, below):

| Test | Then |
|---|---|
| HP > 0, message handler known (below), CBTL present and not locked, `+0x278` in 2..10 | `+0x260 = (+0x260 & ~(0x800 \| 2 \| 0x200 \| 0x10 \| 0x2000_0000)) \| 1 \| 4`, `+0x68 = enemy + (+0x278 + 0x13) × 0x40`, return 1 |
| CBTL locked (`+0x208 & 0xC0000004`, `+0x1E4 != 0`, or `+0x20C >= 2`, exactly `0x1401634B0`'s tests) | Re:CoM's card play, which refuses as always (`0x200` cleared, bit 1 set, `+0x418`) |
| Anything else (no CBTL, unknown handler, Marluxia 3's mode below, a chosen state outside 2..10) | Re:CoM's card play, logged once per class |

- Re:CoM's own state code then runs, draws, collides and ends the attack. No card value, no CBTL
  slot, no `CBattleCard`, no cue sprite. Once per granted attack; nothing per frame.
- **Message handlers** (Checked (exe): the 49 vtables holding `0x140198870` at `+0x98`): 48 use
  the generic `0x140181620`. `CBossMarluxia3` uses `0x1403F4550`, which runs the generic handler
  only while its `+0xCE4 & 4` is clear (Checked (disasm)); with the bit set it ignores message 2,
  so the wrapper leaves Re:CoM's card play there.
- Correction: the research counted 50 users of `0x140198870`; the 50th data reference is its
  `.pdata` entry. Checked (exe).
- The 49 classes: every `CEnemy*` with a card play (33), `CHeartless`, `CSampleEnemy(2)`,
  `CTutorialHood`, Guard Armor, Parasite Cage, Trickmaster, Jafar, Oogie, Ursula, `CBossMDragon`,
  Dark Side, Marluxia 2, Marluxia 3, the 100 Acre Wood bee and beehive. Checked (exe, RTTI).
- Its only caller is the enemy update (`0x14018537C`, the only `call [reg+0x98]` in the enemy
  code). Checked (exe).
- The wrapper moves exactly the 15 prologue bytes `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20`;
  the next instruction (`mov rax,[rip+…]` at `0x14019887F`) is RIP-relative and stays. No branch
  in the exe and no data pointer lands inside them (only the function start: 49 vtables and
  `.pdata`). Checked (exe).

What the skipped steps gave Re:CoM (Checked (decomp) unless tagged):

- The card value `+0x448` (`0x140198860` → `0x1401983D0`, Oogie `0x1403D91C0`): only the card's
  value; it draws from Re:CoM's global random number, so the random sequence differs from
  Re:CoM's (no gameplay rule depends on the sequence; Assumption).
- Message 1's record copy (`+0x318..+0x338`): no reader found (research, Assumption).
- Bit 29 is cleared, as message 2 does for a card record without a sleight id (Assumption: a
  Heartless record never names one). It only matters for `0x14017E540`'s enchant-0x72 repeat.
- Re:CoM starts the attack one or two frames after the grant (CBTL update, message); P1 starts it
  in the grant frame's update. Assumption (not traced). Re:CoM's own code writes `+0x68` from the
  update path the same way (`0x14017E540`, the hit handler).

### P2–P4: Re:CoM's other serialisers, switched once per battle

| Patch | Site | Re:CoM | Ours | Effect |
|---|---|---|---|---|
| P2 | `0x14017D010` (think gate, 6 callers, all `test eax,eax`) | `40 53 48 83 EC 20` | `B8 01 00 00 00 C3` (`mov eax,1; ret`) | no "wait while a side-2 card plays", no "not the last granted twice in a row" |
| P3 | `0x14018765F` (grant loop of `0x140187600`; rax = mask `0x200`, rcx = flags with it) | `48 F7 D0 48 23 C1` (`not rax; and rax,rcx`) | `48 33 C1 80 CC 08` (`xor rax,rcx; or ah,8`) | every requester's `0x200` becomes `0x800` instead of being dropped; the nearest is still stored in manager `+0xF0` |
| P4 | `0x14017F3DE` (think rate `0x14017F380`) | `83 F9 64 7D 26` (`cmp ecx,0x64; jge`) | `83 F9 64 EB 26` (`jmp`, same target `0x14017F409`) | the think rate is no longer multiplied by the number of enemies; the ×10 after a hit (`r8d`, set before the branch) stays |

- Checked (exe): the bytes, the masks (`0x14052A458` = 0x200, `0x14052A468` = 0x800), the new
  bytes disassembled, no branch or data pointer into any patched byte. Checked (decomp): P2's
  body is side-effect free (`0x1401616A0`, `0x140186020` only read).
- **Switch point:** a wrapper on the battle enemy manager's ctor `0x1401859B0` (`48 89 4C 24 08
  55 56 57 41 54`), called once per battle by the battle build `0x1401AEBF0` and the 100 Acre
  Wood build `0x14042D710`, before any battle enemy updates. The three patched functions only
  run for battle enemies (the gate and the grant need the manager; Checked (decomp)). It decides
  "KH1 battle" = Combat = KH1 and not Riku's story (`0x1401AAAE0`), then writes P2–P4 to match:
  each site must hold Re:CoM's or our bytes, else nothing is written (all or none, checked
  writes, undone on a failed write). At the Re:CoM setting the bytes are Re:CoM's own, byte for
  byte, from the next battle on. P1 follows the same decision for the whole battle.
  - The revolver build `0x1401F43D0` (stage 2's switch) was not used: it is skipped when
    `player+0x418 & 0x20000000`, and the enemies' code doesn't depend on Sora's deck.
  - Minigame rooms (`0x140235360() != 0`) build no manager: no grant, so P1 is never reached
    there; the gate dereferences the manager without a null test, so it isn't run without one;
    the think rate skips the enemy count without a manager, so P4 changes nothing there
    (Checked (decomp)). The decision of the last battle stays meanwhile.
  - Assumption: the battle build runs on the game thread, so no enemy runs the patched code
    while it is written.

### What other code reads, at KH1 (Checked (decomp))

- **Bit 0 keeps its meaning "in an attack"**: P1 sets it, `0x14017E540` / the state end
  `0x14017DA90` clear it. The think stays off during the attack (`0x140185320`).
- **Decision 14 (KH1 hits land) and 28 (turned-back shots)**: the enemy-hit wrapper still adds
  `0x2000000` while bit 0 is set; Re:CoM's `0x2000001` path calls `0x14017E1D0`, which returns 0
  (this enemy owns no side-2 card: `0x140160950(2)` answers only while `+0xBC != 0`), then clears
  bit 0 and takes the hit. So the CBTL part is a no-op; clearing bit 0 is still needed.
- **Decision 27 (Guard reactions)**: the flinch path's `0x14017E1D0` is a no-op the same way, and
  the log says "bit cleared (not CBTL's now-playing)".
- **Decision 26 (`hurt_off`)**: unchanged; it runs when the attack state starts.
- Dark Side (`0x1403E7A30`) waits for children owning side 2: they never do, so it acts while its
  children attack (research §3, KH1-like).
- Exception, Assumption: Marluxia 3 may start a P1 attack while its own sleight card holds side 2;
  a KH1 hit then ends that sleight card through the same `0x2000001` path.

### Human bosses (stage 3, left as Re:CoM)

- `CHumanBoss` users (Leon, Cloud, Hades, Vexen, Riku, Larxene, Axel, Hook, Marluxia 1, and
  Riku's story's) never call `0x140198870` (their `+0x98` is `0x1401829D0`, which clears `0x802`).
  Their deck, CBTL plays and message 2 are untouched.
- P3 can reach them only when a human boss's request is not the nearest one: Re:CoM drops its
  `0x200`; P3 grants it, and `0x1401829D0` then clears it again. The only difference is bit 1
  cleared, which nothing reads (Checked (exe): the mask `0x14052A418` is only OR-ed / AND-ed).
  Solo boss fights are identical. Assumption: Sora's-story human-boss fights are solo.
- P4 changes a human boss's think rate only with 2+ enemies and `+0x128 % 10000 < 100`.
  Assumption: boss ids are ≥ 100 (research) and the fights are solo.
- Tutorial Leon still never attacks at KH1 (P5, action `0xB`, stage 3). Not new.

## Logs and trace

- Start: `m6: wrappers in (…)`.
- Each battle start (first 100): `m6: battle start: KH1 | Re:CoM (…); no-card attacks (P1) on |
  off; think gate (P2), grant all (P3), crowd pace (P4): ours (n written) | Re:CoM's bytes (n
  written) | UNCHANGED…`.
- Each cardless attack (first 100): `m6: attack without a card: <class> id=… state=…
  attacking_now=<enemies in bit 0 or states 2..10, this one included> flags …->… cooldown=…`.
- Once per class: `m6: class <name>: attacks without a card (P1)`, `… Re:CoM's card play (<why>):
  its card shows`, `… plays a side-2 card at KH1 (Re:CoM's card path …)` (from a log-only
  pass-through wrapper on CBTL::setPlayCard `0x140163830`, `48 8B C4 57 41 54 41 55 41 56 41 57`).
- CBTL refusals (first 20): `m6: CBTL locked (+0x208=…, gimmick=…, duel=…): Re:CoM refuses …`.
- F9 trace, `events` cell, every recorded frame: `m6 battle=kh1|recom attacking=<n>
  cbtl_slot2=<CBTL+0xBC> enemies=[id:s<state>:<P S R G>:<cooldown> …]` (up to 8 enemies; P = bit
  0, S = 4, R = 0x200, G = 0x800). Read only while recording.

## Not verified

- Nothing seen in game.
- The frame delay Re:CoM has between grant and attack (Assumption 1–2 frames).
- Whether any Heartless class reads its card record (`+0x318..`) through a non-literal offset.
- Scripts (tutorial flags): files not in this session. `CTutorialHood` takes P1.
- KH1 has no shared attack limiter as far as the Shadow's script and the common syscalls were
  read (research §4.3); rarer syscalls: Assumption.
- Re:CoM enemies keep Re:CoM's AI: what each chooses, think rate and cooldowns are Re:CoM's until
  their KH1 twins' AI (M10). P4 makes crowds attack more often than Re:CoM's tuning (decision 30
  chose that).

## In-game checklist (stage 1), in priority order

1. Log at start: every `site ok`, then `m6: wrappers in`.
2. A room with 3+ Shadows or Soldiers at Combat = KH1: no card over an attacking enemy and no cue
   sprite; `m6: attack without a card` lines, some with `attacking_now` ≥ 2; F9 trace shows
   `attacking=2` or more and `cbtl_slot2=0`.
3. Enemies stop attacking at the win pose and during an escape (`m6: CBTL locked` lines, no
   attack after the win).
4. Sora's hits still land on attacking enemies (decisions 14, 26) and Guard reactions still
   flinch them (decision 27: `card bit cleared (not CBTL's now-playing)`).
5. Combat = Re:CoM in Config, next battle: `battle start: Re:CoM … Re:CoM's bytes (3 written)`,
   enemies play cards one at a time as in Re:CoM. Back to KH1: `ours (3 written)`.
6. Heartless bosses: Guard Armor, Dark Side (its children attack while it acts), Trickmaster,
   Parasite Cage, Jafar, Ursula, Oogie, Marluxia 2, Marluxia 3 (its sleights still show cards
   and its stock folder: stage 3). Each can be finished.
7. Traverse Town's tutorial battle (`CTutorialHood`): no soft-lock.
8. Human-boss fights: unchanged (`m6: class … plays a side-2 card at KH1`). Tutorial Leon's known
   P5 issue is stage 3.
9. Riku's story: `battle start: Re:CoM (Riku's story)`, everything as Re:CoM.
10. Feel: crowds attack as often as a lone enemy (P4).
