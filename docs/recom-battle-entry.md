# Re:CoM battle entry and the field swing

Steam addresses. Tags: Checked (decomp) means read; Assumption means inferred.

## Game flow

- `CGameMain` (instance `0x14087B0B0`) runs 17 states; current id at `+0x580`. It owns the field
  (`CGameSubField`, `+0x5B8`) and the battle (`CGameSubBattle`, `+0x5C0`). Checked.
- Main state 5 (`0x1401AAFB0`) is "field running"; main state 9 (`0x1401AAE40`) is "battle
  running". Checked.

## Field to battle

1. **Contact.** Each map enemy (`CMapEnemy`, vtable `0x14055ACB0`) runs `0x14019E020` every
   frame. If something hits it, the hitter's kind picks a reaction state (stuns). On body
   contact with Sora it calls `0x14019BED0`. Checked.
2. **Encounter type.** `0x14019BED0` turns the enemy's flags (`+0x25C`) into an encounter type
   0–9: plain contact, after a stun, and so on. `CBattleEnemy` setup `0x1401802A0` applies it
   in battle: odd types halve `+0x274`, and types 4/5 start the enemy in a timed reaction.
   Checked. What each type means in play (first strike, "stunned for 4 s"): Assumption.
3. **Battle record.** `0x1401A0060` writes the per-room battle record: enemy group, map-enemy id,
   encounter type and whether the enemy respawns. Checked.
4. **Request.** `0x1402368C0` sets the battle-entry request `0x14087B850` and asks for a move to
   the battle room `0x14087B838`. That room is looked up per field room by `0x14023C600` and is
   1000 + an index. Checked.
5. **Handover.** Main state 5 asks the field to shut down; the field tears down (`0x1401AF420`).
   `0x1401AA000` commits the room change (`0x14087B858` = in battle) and starts the battle: an
   encounter effect, then `0x1401AEBF0` builds a new map, `CBattleEnemyManager` and the player.
   Checked.
6. **Back.** `0x140237160` sets the exit request `0x14087B854` and the target field room
   `0x14087B840`. The field reloads and restores positions. `0x140185CB0` writes each map enemy's
   status back. Checked.

**The battle is a separate room with its own map**, loaded after the field room's objects are
destroyed. Checked. Whether it looks like the field room: Assumption.

The player's position is saved on leaving the field, when not in battle (`0x1407C5B50`..). Checked.

## The field swing is a real cardless attack

- The input handler `0x1401E93A0` sends the attack request to `0x1401F0F70` in the field
  (`player+0x390 == 0`) and to `initAttack` `0x1401F0D90` in battle. Checked.
- **Field swing `0x1401F0F70`:** action `0xA`, **card id `+0x7F4 = 0`**, and a fixed `+0x65C`
  of 4000 (`0x1407C784C`). Checked (read by the lead).
- **Battle attack `0x1401F0D90`:** action `0xB`, `+0x65C` from the current card's entry in the
  card table. Checked.
- Both execs (`0x1401EAEB0` field, `0x1401EA6A0` battle) use **the same collision
  (`0x1401F5DC0` → `CPlayerWeapon::onAttack`) and the same motion helpers**. Only the battle one
  has the card-chain branches. Checked.
- The attack-row code already has a no-card branch (`+0x7F4 == 0` → AP = attack stat).
  Checked; see `recom-combat.md`.
- What a field swing does to a map enemy depends on the hit kind, which picks a stun reaction
  or an immediate battle. Which kind the plain swing produces: not confirmed (Assumption).

**Why this matters for the cardless Attack:** Re:CoM already has a native swing that skips the
deck and shares battle collision. In battle, though, only action `0xB` is dispatched, and the
Battle exe has no case for `0xA`.
