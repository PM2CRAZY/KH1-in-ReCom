# M6 design: battles without cards (combat = KH1)

Research notes for ROADMAP M6. No code has been written. All addresses are for the Steam exe.

Tags:
- **Checked (exe)**: bytes read from the exe.
- **Checked (decomp)**: code read in the decomp.
- **Assumption**: inferred, not yet proven.

## 7. Turning off the deck

### What the card system is made of (Checked (decomp))

- **The player's deck.** `CCardRevolver` (`CardBattle::CCardRevolver`) sits at `player+0x3F0`,
  0x280 bytes. Its constructor is `0x140164D40`, and `0x1401F43D0` creates it.
  - It owns a `CEnchantGauge` and a `CStockFolder`, the sleight stock UI.
  - **Re:CoM always creates one.** When there is no deck (`0x14015A680()` returns 0, or event flag 1),
    it builds an *empty* revolver (deck = 0). An empty revolver is a native state.
- **The card battle.** `CCardBattle` (CBTL, `0x14087A818`) is the central object:
  - side 1 is the player and side 2 the enemies;
  - `+0xB8` / `+0xBC` mark a card as playing on that side;
  - `+0x1B0` / `+0x1B4` hold the playing values;
  - `+0x20C` is its state;
  - `+0x1EC` is the room-card battle effect.
  - Each played card is a `CBattleCard` object, drawn with `layout_card.lad`.
- **The player's card input** has three parts:
  - the card button `0x1401E8FF0` (play, stock with Triangle, sleight with L1+R1);
  - the battle branch of `0x1401F8150` (reload when the deck is empty, deck scroll);
  - the combo replay `0x1401E9180` inside `exeAttack`.
- **The player's card state:**
  - `+0x7F4` card id;
  - `+0x428` card request bits;
  - `+0x808..` the play record;
  - `+0x3E8` the player's card-ability object.

### Every place outside the player that reads the player's card state (Checked (decomp))

| Reader | What it reads | What it does |
|---|---|---|
| `0x140198870` (vtable slot in 50 `CBattleEnemy` classes) | `0x140160910(1)`, the player's playing value | the enemy gives up its card (95 %) if its value is lower; otherwise it plays |
| `0x140199A00` (`CHumanBoss`: Tutorial Leon, Cloud, Hades, Vexen, Riku, Larxene, Axel, Hook, Marluxia, Zexion) | the same value, only when `0x14017F720() == 0xB` | the boss holds back a card that would be broken |
| `CBossLexaeus` `0x140414D80`, `CBossAnsem` `0x140424A50` / `0x140422DD0` … | the value, and `player+0x428 & 0x41000` (sleight request) | sleight / counter logic. These are Riku's-story bosses, which stay Re:CoM's (decision 6) |
| `CBossBoogie` `0x1403D91C0` | the cards in the player's deck (`0x140167CF0`) | Oogie picks one of Sora's cards |
| `CBossMarluxia3Tornado` `0x1403F3B60` | revolver `+0x98 & 0x8000` | a boss flag |
| `CBossZexion` `0x140420ED0`, `0x14041AF00` | `0x140167770`: more than 6 attack cards in the deck | Zexion's book trap (Riku's story) |
| card drop on hit, `0x14017E190` → `0x1401602A0` | revolver count < 10 | enemies drop cards into Sora's deck when hit |
| CBTL `0x14015E5D0`, `0x140161E10`, `0x140162140`, `0x140163C00`, `0x140163EF0` | the revolver (scroll, `+0x8C`) | `0x140163C00` and `0x1401F4AB0` (case 3) **use the revolver without a null check** |
| enemy `0x140182500` | the revolver (null-checked) | — |

- With no player card playing, `0x140160910(1)` returns **−1**, so every value check passes.
  Checked (decomp).
- No event-script command that checks card values was found. The script case 10 battle start
  (`0x140145C50`) does not read cards. Assumption: the other script commands were not read.
- **The highest risk is the tutorials.** Tutorial Leon and the help layouts (`gamehelp.lad`,
  `help_stock.lad`) may wait for a card play, reload or stock before they move on. Not read.

### Design

- **At KH1, keep the revolver but empty.** Use the same path Re:CoM uses when there is no deck.
  Nothing becomes null, so the unchecked uses above stay safe. Every `≠ 0` check sees a deck with
  no cards.
- **Input.** Battle input takes the field branch (M5 §2). This alone removes card play, reload,
  stock, sleights and the deck scroll, and the card button `0x1401E8FF0` is skipped.
- **What the player-side state reports:**
  - `+0x7F4 = 0`;
  - no card bits in `+0x428`;
  - CBTL side 1 never playing (`+0xB8 = 0`, value −1).

  Re:CoM already reports exactly this whenever Sora is not playing a card, so nothing has to be
  faked.
- **Card drops on hit must stop at KH1.** Otherwise they fill the empty revolver. The patch goes at
  `0x14017E190`, or at the drop roll `0x1401602A0`.
- **The deck UI is hidden.** The revolver and its `CEnchantGauge` / `CStockFolder` are not drawn
  (their task's draw flag). Assumption: the exact flag is not read yet.
- **Sleights** become KH1 abilities in M9 (decision 7). Until then, none are available at KH1.
- **To test:** every Sora-story boss, plus the tutorial battle.

## 8. Enemies without cards (owner option a)

### Findings (Checked (decomp))

- **How an enemy attacks.** The AI calls vtable `0x140198870`. It works in these steps:
  1. it builds a card value (vtable `+0x448`);
  2. it compares that value with the player's (above);
  3. it calls `CBTL::setPlayCard` (side 2) through `0x1401634B0`. This is refused while CBTL is busy
     (`+0x208 & 0xC0000004`, `+0x1E4`, `+0x20C ≥ 2`);
  4. on success it sets enemy flag `+0x260` bit 0 ("card playing") and runs the attack;
  5. when the action ends, `0x14017E540` → `0x14017E1D0` ends the card in CBTL.
- **Without player cards, enemies still attack on their own timing.** The player's value is −1, so
  they never give up. CBTL is the only gate: it lets one enemy card play at a time. Assumption:
  there is a single side-2 slot. That is Re:CoM's own pacing, so **option (a) needs no change to
  enemy AI**.
- **"Card priority" is a problem.** `0x14017F7E0`, the enemy hit handler, **returns before damage
  while bit 0 is set** (unless bits `0x2000001` are both set, in which case the card is ended first).
  - So an enemy that is attacking with a card takes **no damage and no flinch** from Sora's hits.
  - In Re:CoM, only a card break ends that. With no cards, KH1 Sora cannot hurt an enemy during its
    attacks.
  - KH1 has no such rule. This strongly affects how the game feels.
  - What bit `0x2000000` marks: Assumption ("interruptible card").
- **Boss sleights and card breaks.** With no player cards, the player never breaks a card and is
  never broken. Bosses' sleights always go through. Card duels (action `0x24`, `duelmode.lad`) are
  expected never to start. Assumption: a duel needs the player to play a sleight; the trigger was
  not read.

### Design

- Leave the AI and CBTL as they are; enemies keep their Re:CoM attack pacing.
- At KH1, the enemy card display (the `CBattleCard` shown when an enemy plays) is hidden. See the
  owner question.
- The card-priority rule needs an owner decision (below). If KH1's rule is chosen: at KH1, a
  one-condition patch in `0x14017F7E0` treats bit 0 as interruptible (the `0x2000001` path). The
  hit then ends the enemy's card and lands. This is Re:CoM's own break path, without a card.

## 9. HUD

### Findings (Checked (decomp))

- **`Gauge::CPlayerGauge`.** Its constructor is `0x140206E50` and its init `0x140208360`.
  - It holds a sprite-layout object (0x170 bytes, `0x140247260`) built from `gauge.lad`:
    sequences `gauge:01` `0xE`/7/8/9 and `gauge:00` 3.
  - The HP bar's parts 1–3 get width `maxHP / 560 × 470` (`0x140247C20`). So the bar grows with max
    HP, like KH1's.
  - It has these children:
    - `CCommonGauge` (`0x14087B6A8`);
    - an object from `0x140206EE0`;
    - in the field only, `0x1401E3770` and `CFriendGauge`.
  - Siblings: `CEnemyGauge` (built by `0x140207C70` from `gauge.lad` groups 0–6), `CPrizeGauge`,
    `CEscapeGauge`, `CLockonMarker`, and `Cockpit.lad` parts.
- Everything is drawn through Re:CoM's 2D layout system (`.lad` files, sequences by name).
- Re:CoM has no MP. Whether `gauge.lad` contains a spare bar sprite is unknown: the file is not in
  this session.

### Design

- **The MP gauge** is a second layout object inside `CPlayerGauge`, built in a wrapper on its init
  and updated in its own update (vtable). It uses Re:CoM's layout calls with KH1's rules for MP
  (bar length by max MP, MP charge). The KH1 HUD maths are not yet read from the KH1 decomp.
  - The sprites are either Re:CoM's HP bar sequences recoloured, or KH1's MP bar added to
    `gauge.lad` by the asset step. Plan that with the tester; the file is needed.
- **Hiding the deck:** see §7. The enemy card display: see §8.
- The HP gauge stays Re:CoM's. The MP gauge waits for M7's MP.

## Risks

- Tutorials and scripted fights that may wait for a card action. Each needs testing.
- Card priority makes attacking enemies untouchable.
- `@ENP` and `gauge.lad` were not read: the files are not in the session.
- An empty revolver may still show a "reload" prompt. Not read.

## Owner questions

1. **Hitting an enemy during its attack** (Re:CoM's "card priority"):
   - (a) KH1's rule: Sora's hits land and can interrupt.
   - (b) Re:CoM's rule: an attacking enemy cannot be hurt.
   - (c) A Config toggle.
2. **The enemy's card shown above it when it attacks:**
   - (a) Hidden at KH1.
   - (b) Kept, for timing cues.
3. **The MP gauge's look:**
   - (a) KH1's MP bar, taken from the player's KH1 files.
   - (b) Re:CoM's HP bar style in blue.
