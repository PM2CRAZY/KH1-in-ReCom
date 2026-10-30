# What replaces each Re:CoM card at KH1 setting

Owner's decision (2026-10-29):

> "All summons and magics should be replaced with their KH1 equivalents. All summons and magics
> that dont have an equivalent should be removed for the time being but taken note of so i can
> reimplement them later. Same for the items."

The Re:CoM card names come from the Steam exe's name table at `0x140506274`..`0x140506340`.
Checked (exe). The KH1 matches come from knowledge of the game. KH1's exe doesn't hold these
names (they're in its data files), so each match is an **Assumption until checked** against KH1's
magic, summon and item tables.

## Magic

| Re:CoM card | KH1 equivalent | At KH1 setting |
|---|---|---|
| Fire | Fire | replaced |
| Blizzard | Blizzard | replaced |
| Thunder | Thunder | replaced |
| Cure | Cure | replaced |
| Gravity | Gravity | replaced |
| Stop | Stop | replaced |
| Aero | Aero | replaced |

## Summons

| Re:CoM card | KH1 equivalent | At KH1 setting |
|---|---|---|
| Simba | Simba | replaced |
| Genie | Genie | replaced |
| Bambi | Bambi | replaced |
| Dumbo | Dumbo | replaced |
| Tinker Bell | Tinker Bell | replaced |
| Mushu | Mushu | replaced |
| **Cloud** | none (Cloud isn't a summon in KH1) | **removed for now, to be reimplemented later** |

## Items

| Re:CoM card | KH1 equivalent | At KH1 setting |
|---|---|---|
| Potion | Potion | replaced |
| Hi-Potion | Hi-Potion | replaced |
| Mega-Potion | Mega-Potion | replaced |
| Ether | Ether | replaced |
| Mega-Ether | Mega-Ether | replaced |
| Elixir | Elixir | replaced |
| Megalixir | Megalixir | replaced |

## Removed for now and noted (no KH1 equivalent yet)

- **Cloud** (summon).
- **Friend cards** (Donald, Goofy, Aladdin, Ariel, Jack, Peter Pan, The Beast, The King, Pluto):
  no party in the arena for now (owner, 2026-10-29).
- **Enemy cards** (Sora's enemy-card abilities): removed for now (owner, 2026-10-29).
- **Sleights with no KH1 equivalent**: removed for now (owner, 2026-10-29). The full sleight-by-
  sleight list is still to be made from Re:CoM's sleight table (`StockSkillList`).
- Map cards (room cards) stay Re:CoM's; they aren't battle cards.

## Sleights

Owner (2026-10-29): those with a KH1 equivalent become KH1's version. Magic sleights such as
Fira and Firaga become KH1's magic levels; physical ones such as Sonic Blade, Strike Raid,
Ars Arcanum and Ragnarok become KH1 abilities. The rest are removed and noted. The table of
each sleight and its KH1 match is still to do.

## Keyblades

Re:CoM's attack cards are Keyblade cards: Kingdom Key, Three Wishes, Crabclaw, … , Bond of
Flame (`0x1405060F0`..). At KH1 setting the attack cards go entirely, and Sora attacks KH1's way.
Owner (2026-10-29): Sora starts with **Kingdom Key, using KH1's stats**.
