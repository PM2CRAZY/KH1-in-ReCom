# M6: battles at the KH1 setting (decisions 29, 30)

Steam addresses (Re:CoM image base `0x140000000`). Tags: Checked (decomp / exe / tests) or
Assumption. Code: `crates/kh1-recom` (`battle.rs`, `win/battle.rs`, stage 2 `deck.rs`,
`win/deck.rs`; `sites.rs` `M6_*`). Research:
`m6-cardless-battles.md` and `recom-m6-patch-points.md` (kept with the decomp tools). This note
supersedes `recom-cardless-battle-design.md` §8 ("CBTL is the only gate").

## Status

- **Stage 1 built (0.12.0, not seen in game):** at Combat = KH1, in Sora's story, Heartless and
  Heartless bosses attack without cards, and several can attack at once, each as if alone.
- **Stage 2 built (not versioned yet, not seen in game):** at Combat = KH1, in Sora's story,
  Sora has no deck in battle: an empty revolver built by Re:CoM's own constructor, no Friend-card
  drops, the deck HUD hidden through Re:CoM's own event-hide mask. Sora still collects cards.
  § Stage 2 below.
- Stage 3 (bosses one at a time, starting with the action-0xB check P5; human bosses' hidden
  move lists; Marluxia 3's stock folder hidden) is not built. Human bosses keep Re:CoM's card
  path exactly.

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

## Stage 2: no deck for Sora (decision 29)

At Combat = KH1, in Sora's story, outside minigame rooms. Research: `m6-cardless-battles.md` § 1
and `recom-m6-patch-points.md` §§ 1–3, 5 (kept with the decomp tools); every address below was
re-read for this stage. Tags as above.

### What changes

| Part | Where | What | Tag |
|---|---|---|---|
| Empty revolver | wrapper on the battle revolver build `0x1401F43D0` (`40 57 41 56 41 57 48 83 EC 40`, 10 bytes moved) | with `player+0x3F0 == 0`: `mem = 0x1404ACE54(0x280)`, then Re:CoM's ctor `0x140164D40(mem, player, list, 1, 0xFFFFFFFF)` with `list = {0x000000FF}`, stored at `player+0x3F0`; then the original runs, sees the built revolver (`0x1401F43F5`: `cmp [rcx+0x3F0],0; jne` tail) and only runs its tail (`+0x3E8` = CBTL) | Checked (decomp + exe) |
| No Friend-card drops | wrapper on `0x14017E190` (`40 53 48 83 EC 20 48 8B D9`, 9 bytes) | returns at once: the same result as Re:CoM's own failed roll (id 0, no prize spawned) | Checked (decomp + exe) |
| Deck HUD hidden | 7 of Re:CoM's `call 0x1401B08C0` (system variable (1, 0xE2), Re:CoM's HUD-hide mask) retargeted to a stub near the exe, per battle | the stub (`49 89 D8` `mov r8,rbx`, then `jmp [rip+0]`) calls ours with the caller's object; ours returns Re:CoM's mask, plus bit 2 for the cockpit of this battle's empty revolver, or bit 0x20 for a side-1 card while the main player holds it. Re:CoM's own hide branches then run | Checked (decomp + exe) |

- **The empty revolver is Re:CoM's own "deck of 0 cards".** `0x1401F43D0` builds a saved deck's
  list as its cards plus an end entry `0x000000FF` (byte 3's low nibble from `0x1405A02D0`, which
  is 0; Checked (exe)). The loader `0x14016C970` stops at an id byte of 0 or ≥ 0xCB
  (`0x14016C9A0`), so page 0 gets 0 cards (`+0x70`), a count of 1 (`+0x6C`) and 10
  `CBattleCard` slots, the first of which then holds the **Reload card** (id 0xA0); `+0x98 |= 1`. Re:CoM builds the same for a saved deck
  of 0 cards and for an enemy revolver without a deck. Nothing is null: CBTL's unchecked uses
  (`0x140163C00` case 0x6D, `0x1401F4AB0` mode 3) stay safe. Checked (decomp).
- Skipped with Re:CoM's build: `0x14015A680()`, which refreshes the staging copy `0x14087A650`
  of the active saved deck (from the save's deck index). Its only other user, `0x14015BCB0`,
  refreshes it itself first (`0x14015BA90`). Checked (decomp + exe scan for references).
- **Covers every deck Re:CoM would have dealt Sora:** the saved deck, and the fixed decks of the
  null-deck branch, including the **tutorial deck** `0x1407C2450` (event flag (1, 0x2C); Kingdom
  Key 0–9 and a Shadow enemy card). Not changed: Riku's story (decision 6, `0x1401AAAE0`), a
  main player that isn't `CPlayerSora`, **minigame rooms** (`player+0x394 != 0`, rooms
  1200..1255: Re:CoM's minigame deck `0x1407C2590`; the same test `0x1401F43D0` makes).
- **No shuffle, no draw, no Reload, no stock, no sleights:** the revolver ctor and loader make no
  random call (research, Checked (decomp)); Reload starts only from request bit `0x10`, set only
  in the battle branch of `0x1401F8150`, which KH1 Sora never takes (Stage 9's field branch;
  the use-card button `0x1401E8FF0` is skipped). Stock and sleights need a played card.
  Checked (decomp).
- **The revolver still deals** (state 0 → 2 → 1, `0x140169F40` → `0x140169710`): it is not
  stuck in state 0, so event flag (1, 0x13) fires at the end of the deal, and the card-scatter
  flag `+0x98 & 0x8000` (`CBossMarluxia3Tornado` `0x1403F3B60` reads it) is still cleared in
  state 1. The deal gate's own mask read is not retargeted. Checked (decomp).
- **Battle end, escape, game over, continue:** Re:CoM frees the revolver itself
  (`0x1401F1B20`, `0x1401F1C40`, `0x1401E7C20` set `+0x3F0 = 0`); it is Re:CoM's object, built by
  its ctor with its allocator. A continue re-enters stateInitial, so the wrapper runs again. The
  battle end (`0x1401871D0` → win action 0x1C) reads no deck. Checked (decomp). Not seen in game.
- **Switch point:** the revolver build, called by `CPlayer::stateInitial` (`0x1401FAAD0`) only in
  battle rooms and only when `player+0x418 & 0x20000000` is clear, and by Riku's stateInitial
  (`0x140206A1A`). It decides from the setting at that moment. The retargets follow: ours for a
  battle without a deck, **Re:CoM's bytes otherwise**, all or none, checked writes. The two
  wrappers stay installed and pass through at the Re:CoM setting (as stage 1's). Assumption: the
  setting can't change inside a battle (the Config menu is in the camp).

### Friend cards (decision 29)

- The drop roll `0x1401602A0` gives only **Pluto 0x39** and **Donald, Goofy, Aladdin, Ariel,
  Jack, Peter Pan, Beast 0x31..0x37** (The King 0x38 in Reverse/Rebirth). All are type 5 in both
  card tables (`0x14059F2E0`, `0x140526510`; Checked (exe)). The prize add `0x1401F4AB0` puts type
  5 / 6 cards **only into the battle revolver** (`0x140166300`), never into the collection.
  Checked (decomp).
- So at KH1 **Donald, Goofy and the other Friend cards are gone with the deck**, as decision 29
  expects (decision answer 4: no party for now). Nothing is removed from the collection.
- The hook covers all three callers: the hit handler (`0x14017FC9D`, chance −1 = the world's) and
  the battle setup's room-card 0xAF case (`0x140180366`, `0x1401803F4`, chance 1000). Checked
  (exe). Decided from the setting at the drop (Combat = KH1, Sora's story, not a minigame room
  `0x140235360`), so the battle setup's drops don't depend on whether they come before the
  player's revolver build.
- Skipping the roll skips its global random draws (`0x14029CDC0`): the random sequence differs
  from Re:CoM's. Assumption: no rule depends on the sequence (P1 already changes it).
- Script-given Friend cards (`0x1401F4AB0` from the event VM, mode 2) still go into the empty
  revolver, hidden. Assumption: rare in Sora's-story battles.

### Sora still collects cards

Nothing on the collection path is hooked. Checked (decomp):

- Normal prizes (types 1–4, 7, 9–0xB) go to the collection `0x140158FC0` in `0x1401F4AB0` unless
  mode 3, and **no caller passes mode 3** (its five callers pass 1 or 2). Battle prizes come
  through the pickup `0x140187AA0` (mode 1); HP, Moogle points, the post-battle roulette
  (`0xD6` / `0xD7`) are ids above 0xCA, before any card code.
- Rewards, Moogle shops, chests and event cards use the collection, not the battle revolver,
  which exists only in battle. Assumption (shops not read): they never touch `player+0x3F0`.

### The deck HUD

| Part | Re:CoM's control | At KH1 (stage 2) |
|---|---|---|
| Cockpit frame and the two count digits | `0x1401689F0` (player side only), 4 reads `test al,2` at `0x140168A34`, `0x140168A93`, `0x140168B32`, `0x140168B90` | bit 2 for this battle's empty revolver: Re:CoM's own hide (state 4, slide-out seq 0x44, digits off) |
| Sora's cards, the Reload card and its gauge (`0x140154750` draws it on the card's sprite) | per-frame show / hide `0x140150340` (`0x14015037C`, tests 0x20 for side 1, 0x40 for side 2); deal `0x1401514B0` (`0x140151625`) and refresh `0x1401545A0` (`0x1401545BD`) test the whole mask | bit 0x20 for side-1 cards while the main player holds the empty revolver: Re:CoM's hidden branch; the deal and refresh take their hidden branch too, so there is no one-frame show |
| Stock row, `CEnchantGauge`, selected-card panel `+0x108`, cue `+0x248` | need a stocked or played card, a page switch | never shown at KH1; no change |
| Enemy cards (side 2), Marluxia 3's `CStockFolder`, the enemy cue sprite | bits 0x40 / 4, CBTL | **untouched** (stage 3) |

- `CBattleCard`s are made only by CBTL and the revolvers (`0x14014F8C0`'s callers), so side-1
  cards exist only in battle. Checked (decomp).
- Each retargeted read sits right after `mov edx,0xE2` and ecx = 1, and its function put its
  object in rbx first (`mov rbx,rcx` at `0x1401689FA`, `0x140150359`, `0x1401514CC`,
  `0x1401545AE`) with no later write to rbx before the read. Checked (exe, disassembly; the
  bytes are in `sites::M6_DECK_CHECKED`).
- The stub touches no stack, so our function is entered as the getter would be (the caller's
  return address, alignment and shadow space). It clobbers r8, which is volatile in the x64 ABI;
  the getter itself ends in a jump through a vtable (`0x1401B08DE`), so no caller can rely on r8
  across it. Checked (exe).
- Not used: writing (1, 0xE2) itself (it also hides the HP gauge, and events rewrite it), and bit
  4 (it gates the enemy revolver's deal in `0x140169F40`). Checked (decomp).
- Cost: an OR and two loads in reads Re:CoM already makes every frame. No data-only route:
  Re:CoM re-reads the mask every frame.

### Exe checks

- New sites (`sites::M6_DECK_HOOKS`, `M6_DECK_READERS`, `M6_DECK_CALLEES`, `M6_COCKPIT_CALLS`,
  `M6_CARD_CALLS`, `M6_DECK_CHECKED`), all byte-checked at start; the 163 byte lists of
  `sites.rs` were re-read from the Steam exe (0 mismatches).
- No branch in `.text` and no relocated pointer lands inside any moved or retargeted byte (the
  5 hook prologues, the 7 call sites); the only other references to the hooked starts are their
  `.pdata` entries. Checked (exe scan).

### Logs and trace (stage 2)

- Start: `m6: HUD stubs at …`, `m6: no-deck wrappers in (…)`, `m6: deck readers logged once per
  battle (…)`.
- Each battle start (first 100): `m6: battle without a deck: empty revolver 0x… built by Re:CoM's
  ctor 0x140164D40 from its end-of-deck entry (0 cards, 1 slot(s), flags 0x…: Reload card in);
  tutorial-deck flag (1, 0x2C) set | clear; deck HUD mask reads (cockpit 4, cards 3): ours (n
  written); Friend-card drops off; tutorial flags set: … (battle n)`, or `m6: battle with Re:CoM's
  deck (Combat = Re:CoM | Riku's story | minigame room | …); deck HUD mask reads: Re:CoM's bytes
  (n written)`.
- Drops (first 20): `m6: Friend-card drop roll skipped (chance …)`.
- Once per battle, a reader that read the empty revolver (first 60): `m6: deck reader (battle
  n): the tutorial's hand-empty check (0x1401EA1A0, CAtelTutorial) …`, `… an event script's
  revolver scroll (0x1401EA1E0) …`, `… a revolver value search (0x140167CF0, enemy card value /
  Oogie)`. Pass-through hooks, installed apart (log only).
- F9 trace, `events` cell: `m6deck deck=none|empty|recom tut=[<set flags of (1, 0x11..0x18,
  0x1B)>]`, after stage 1's `m6 …` token.

### The tutorial battles (soft-lock risk): what is known and what isn't

Known (Checked (decomp + exe)):

- `CAtelTutorial` (`0x140137CB0`, made by script command 0xAD sub 0x10) sets event flags while it
  exists: (1, 0xE) on Sora's action 6 (roll), (1, 0xD) on action 7 (jump), and (1, 0x11) while
  `0x1401EA1A0` says "hand empty" (no revolver, or `+0x70 == 0`). KH1's roll and jump still run as
  actions 6 / 7, so 0xD and 0xE still fire. **0x11 fires at once** at KH1 (0 cards).
- Flags only the deck sets: 0x13 deal done (**still fires**); 0x12 reload / page switch
  (`0x140167910`), 0x14 / 0x16 a player card done (`0x14015E0E0` side 1, `0x14015FC53`), 0x15
  stock (`0x14016DB60`), 0x17 a revolver scroll (`0x14016C580`, also from the script scroll):
  **0x12, 0x14, 0x15, 0x16 never fire at KH1**; 0x17 only if a script scrolls.
- `CTutorialHood` (`0x1403A97E0`) sets (1, 0x18) once Sora's target cursor (`player+0x3F8`,
  `+0xE0 == 1`) has been on it until its time delta × 0.02 sums to 60 (Assumption: about one
  second), and is driven by the script's variable (1, 0xE1)
  (steps 1, 2, 3, 6, 7, 9). Its attacks go through P1 (stage 1). `CTutorialLeon` sets (1, 0x1B)
  and attacks only while Sora's action is 0xB (P5, stage 3).
- No exe code reads flags 0xD, 0xE and 0x11..0x17: only event scripts do.

Not known (the event scripts are not in this session): **which of these flags the tutorial
scripts wait for.** If a script step waits for 0x12, 0x14, 0x15 or 0x16 ("reload", "play a
card", "stock"), that tutorial cannot finish at KH1, with or without this stage: Sora's card
input has been off at KH1 since Stage 9, so even the tutorial deck could not be played. If a
step waits for 0x11 ("deck empty") it now comes at once. Nothing in stage 2 fakes these flags.
What the tester's log and trace give us: the battle-start line shows whether (1, 0x2C) was set
and which tutorial flags were set; `deck reader` lines show `CAtelTutorial` running; the F9
trace's `tut=[…]` shows the flags frame by frame. Owner question if it soft-locks:

- (a) fire the flags the script waits for from their KH1 counterparts (for example 0x14 / 0x16
  when a KH1 swing ends), once the script files show which;
- (b) keep Re:CoM's card battle for the tutorial battles only (flag (1, 0x2C)), card input
  included;
- (c) skip the deck tutorial steps at KH1 through the script's own variable.

### Not verified (stage 2)

- Nothing seen in game.
- The cockpit may play its slide-out (seq 0x44) once at battle start: its init `0x14016A840`
  starts it in state 1, and the first update sees bit 2 (Re:CoM's events hide it the same way).
  Assumption.
- Whether something else turns a hidden card's sprite back on while its `+0x264 & 0x80` is clear
  (`0x140150340` re-hides only cards with 0x80): none found; Re:CoM's event hide has the same
  rule. Assumption.
- The tutorial scripts (above). Room cards that read Sora's deck: none found beyond the readers
  listed in research § 5.
- Shops and chests not re-read (they don't use the battle revolver: Assumption).
- Minigame rooms keep Re:CoM's minigame deck, but KH1 Sora has no card input there (Stage 9):
  whether those minigames can be played at KH1 is not checked (not new).

### In-game checklist (stage 2), in priority order

1. Log at start: every `site ok`, `m6: HUD stubs at …`, `m6: no-deck wrappers in`, `m6: deck
   readers logged`.
2. A normal battle at Combat = KH1: no cockpit, no card wheel, no count digits, no Reload card or
   gauge at any time (watch the first second: a brief cockpit slide-out is the known risk).
   Log: `battle without a deck: … (0 cards, 1 slot(s), …: Reload card in)`. F9 trace:
   `m6deck deck=empty`.
3. Sora attacks, jumps, rolls, guards, takes hits (including a card-scatter hit) and wins:
   win pose, rewards and card prizes as usual; prizes land in the collection (check the deck
   editor after the battle).
4. No Friend card drops (hit many enemies; `Friend-card drop roll skipped` lines); HP / Moogle
   prizes still drop.
5. Escape from a battle; lose a battle and Continue; both work, and the next battle again logs
   `battle without a deck`.
6. The very first tutorial battle (the hooded figure, `CTutorialHood`) and Traverse Town's
   Leon (`CTutorialLeon`, which needs stage 3's P5 to attack): does each finish? If not, the log's
   battle-start line (`(1, 0x2C)`, tutorial flags) and an F9 trace of the stuck moment
   (`tut=[…]`).
7. Switch Combat to Re:CoM, next battle: `battle with Re:CoM's deck (Combat = Re:CoM); deck HUD
   mask reads: Re:CoM's bytes (7 written)`, the full deck and HUD as Re:CoM, Friend cards drop.
   Back to KH1: `ours (7 written)`.
8. Bosses with enemy cards (human bosses, Marluxia 3's stock folder) still show their cards
   (stage 2 hides Sora's side only).
9. Oogie Boogie's fight: no hang (it searches Sora's deck: `deck reader … value search`).
10. Riku's story: `battle with Re:CoM's deck (Riku's story)`, everything as Re:CoM.
