# Apples, crafting and equipment — proposal, not implemented

This game moves the hero automatically and supports API-driven powers. Progression should preserve that flow instead of introducing a Minecraft-sized inventory or pausing for manual recipes. Current hero health handling deliberately ignores damage, so armor and healing must have useful effects beyond protecting the hero's HP.

## Natural tree rewards

Use `TreeBlock.root` and species metadata to identify actual tree crowns. The world also uses Leaves as Plains terrain, so a generic “every Leaves block drops fruit” rule would produce unlimited apples from the landscape.

- Oak: suggested 8% apple chance on a naturally generated crown block, at most two apples per tree per run. Trees destroyed by fire produce no fruit. Re-streaming a chunk must not reset its harvest ledger.
- Birch: bark/fiber; spruce: resin/cones. These should reinforce wood species rather than every leaf dropping identical apples.
- Golden apples: preferably craft one apple plus eight gold resource units, or find one in a rare forest landmark cache. If random crown drops are desired, use one roll per oak tree (suggested 0.5%), never one roll per individual leaf or reload.
- Ordinary apples automatically feed a nearby injured villager; surplus goes into a small shared pantry. Golden apples give the party a short protection/regeneration effect. Do not consume them while everybody is healthy, or just to heal the currently invulnerable hero.

## Small automatic workshop

Add explicit wood/fiber collection and persistent recipe state first; these resources do not exist in the current resource economy. Show collected ingredients and the next recipe beside the existing resource display.

Choose a priority such as “Rescue / Travel / Mining”; the workshop queues matching recipes as ingredients arrive. Players can pin one recipe, disable automatic spending, and reserve a minimum gold/diamond balance. API commands should expose the same recipe IDs and priority settings with clear validation. No hidden automatic spending of rare resources.

Start with three recipes: villager supply pack, reinforced mining tool, and travel coolant. Unlock others through biome landmarks. Each craft is one atomic debit/output transaction so repeated commands cannot duplicate items. Ingredients and completed crafts must persist across chunk reloads and save/load.

## Three equipment slots

1. Tool — mining damage/ore recovery, with visible tradeoffs (speed versus yield).
2. Suit — environmental resistance and protection for carried/rescued villagers; not a misleading armor number on an invulnerable hero.
3. Charm — a bounded passive bonus such as shorter launcher cooldown, longer slow resistance, or a modest loot bonus.

Auto-equip a clearly better item within the chosen build; keep sidegrades in a small collection and let the user choose. Show the actual stat change. Cap stacked bonuses and apply them through shared combat, mining and power systems. Avoid random durability chores in the first version.

## Implementation order

First define persistence, resource transactions and harvesting identity; then apples and the pantry; then the three slots and three recipes. Balance using actual run length, resource income and villager survival. The odds and costs above are starting proposals, not current game behavior.
