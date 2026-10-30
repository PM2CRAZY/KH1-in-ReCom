# Re:CoM player: states, actions and the card path

Steam addresses. Offsets are from the start of the `CPlayer` object. Tags: Checked (decomp) means
the code was read; Assumption means inferred.

## Objects

- Main player pointers: `0x14087B380` and `0x14087B390`, set by the `CPlayer` constructor
  `0x1401E4C20`. Checked.
- Vtables: `CPlayer` `0x1405A0358`, `CPlayerSora` `0x1405A9938`, `CPlayerRiku` `0x1405A8868`,
  `CPlayerDRiku` `0x1405A50E8`. Checked (RTTI).
- The player's message handle is at `+0x20`. Checked.

## Player state machine: table-driven, 4 states

- A generic state machine (`CFSM`) is embedded at `+0x58`. Its update `0x140242F50` runs the old
  state's end and the new state's begin when a next state is pending, otherwise the current
  state's exe. Checked (read by the lead).
- `+0x60` is the current state object and `+0x68` the next one. Checked.
- 4 state objects (`CStateTemplate<CPlayer>`, 0x40 bytes) are at `+0x280 + idx × 0x40`. Their
  begin / exe / end handlers are copied from static tables at `0x14087B3A0` / `0x14087B3E0` /
  `0x14087B420`. Each entry is a thunk into the `CPlayer` vtable, so subclasses could override
  it; Sora overrides none. Checked.
- The state changer `0x1401E69B0(player, idx)` writes `+0x68`; some code writes `+0x68`
  directly. `+0x380` mirrors the index. Checked.

| idx | state | begin | exe | end |
|---|---|---|---|---|
| 0 | Field | `0x1401E5DA0` | `0x1401FA0F0` | `0x1401EA3A0` |
| 1 | Battle | `0x1401E5C40` | `0x1401F9840` | `0x1401EA310` |
| 2 | Wait | `0x1401E5F40` | `0x1401FAB10` (empty) | `0x1401EA4A0` (empty) |
| 3 | Hide | `0x1401E5E80` | `0x1401FA680` | `0x1401EA430` |

`+0x390` picks Field (0) or Battle (non-zero). It is "in battle room or minigame room", set when
the player is built. Checked.

## Action layer: a switch, not a table

- `+0x40C` is the current action, `+0x408` the previous one, `+0x414` the action's phase. The
  setter is `0x1401F8D70`. Checked.
- The Field and Battle exes each switch on `+0x40C`. Each action has an init (a plain function
  or a `CPlayer` vtable slot) and an exe. Checked.
- **The Battle exe handles attack action `0xB` only; the Field exe handles attack action `0xA`
  only.** Checked (lead read both switches).

| id | action | init | exe |
|---|---|---|---|
| 2/3/4 | stand / walk / run | inline | `0x1401EEDA0` |
| 6 | dodge roll | `0x1401F10C0` | vtable `0x1C8` |
| 7 | jump | `0x1401F29F0` | `0x1401EDA60` (Assumption: name) |
| 8 | glide (Sora) | vtable `0x1A8` | vtable `0x1B0` |
| 0xA | **field swing (no card)** | `0x1401F0F70` | `0x1401EAEB0` |
| 0xB | **battle attack card** | `0x1401F0D90` "initAttack" | `0x1401EA6A0` |
| 0xE | item card | `0x1401F28F0` | `0x1401ED6E0` |
| 0x12 | card reload | `0x1401F1700` | inline |
| 0x13 | magic card | vtable `0x148` | vtable `0x150` |
| 0x16 | sleight | vtable `0x138` | vtable `0x140` |
| 0x17 | escape | `0x1401F1C40` | inline |
| 0x19 | gimmick card | `0x1401F25D0` | `0x1401ECA70` |
| 0x1A | friend card | vtable `0x160` | vtable `0x168` |
| 0x1B | summon card | vtable `0x170` | vtable `0x178` |

Other ids and some names are Assumptions; see the research notes kept with the decomp tools.

## Card path: button to attack

1. Battle exe `0x1401F9840` updates the pad wrapper (`CPlayerPad` at `+0x230`).
2. `0x1401E8FF0` reads a press of the configured confirm button. That this is the "use card"
   button is an Assumption. It needs `+0x41C & 0x40` ("can use cards").
3. It calls `0x14016CEE0`, which plays the selected card from the deck wheel `CCardRevolver`
   at `+0x3F0`.
4. That calls `CBTL::setPlayCard` `0x140163830` on the card-battle singleton (`CCardBattle`,
   `0x14087A818`). It registers the card for the player's side and calls the card's own
   `BC::setPlayCard` `0x1401521A0`.
5. The card manager's update `0x1401617F0` → `0x14015EAC0` settles the clash with the enemy's
   card, then posts a message to the player with the card record.
6. The player's message handler (vtable `0x48` = `0x1401F5580`) copies the record to `+0x808`,
   sets `+0x7F4 =` card id, and sets a request bit in `+0x428` by card type (high nibble of
   record byte 3; byte 2 is the card value):
   1 Attack `0x10`, 2 Magic `0x100`, 3 Summon `0x800`, 4 Item `0x80`, 5 Friend `0x400`,
   6/0xE Gimmick `0x200`. A sleight sets `0x1000`.
7. Next frame `0x1401F8150` turns those into request bits in `+0x424` (attack = bit `4`).
8. The move exe `0x1401EEDA0` (and the idle exe) sees bit 4 and calls `initAttack(player, 0)`.

Combo hits within one attack replay a card through `0x1401E9180`. Steps 1–8 are Checked
(decomp, research agent); steps 6 and 8 spot-checked by the lead.

## Key fields

| offset | meaning | tag |
|---|---|---|
| `+0x3C8` | current attack row (see `recom-combat.md`) | Checked |
| `+0x3F0` | deck (`CCardRevolver`) | Checked |
| `+0x41C` | ability mask (`0x40` cards, `0x10` jump, `0x8` dodge) | Checked bits; names Assumption |
| `+0x424` | action request bits, rebuilt each frame | Checked |
| `+0x428` | card request bits | Checked |
| `+0x42C` / `+0x430` | HP / max HP | Checked |
| `+0x43C` | attack stat | Checked; name Assumption |
| `+0x630` / `+0x63C` / `+0x640` | attack situation / combo index / combo max (3) | Checked; names Assumption |
| `+0x650` | queued chain action in an attack (1–7) | Checked |
| `+0x7F4` | card id of the current card action (0 = none) | Checked |
