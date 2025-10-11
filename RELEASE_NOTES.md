# Release Notes

## Bug Fixes

- Fixed snowball physics: Added Mass component to prevent NaN values in Avian physics engine
- Fixed cactus transparency: Changed alpha mode from Opaque to Blend to properly render transparent pixels
- Fixed redstone ore texture: Now uses correct `redstone_ore.png` instead of coal texture
- Fixed Y coordinate display: Now shows block coordinates (Y / 64) instead of raw world units

## New Features
- Added snowball splash effect: White particles radiate outward when snowballs hit characters or blocks
- Added freeze visual overlay: Progressive ice effect (6 stages) that gets more intense as freeze stacks increase
- Added snowball hit sound: Plays `hurt1.ogg` when snowballs hit characters or blocks
- Added full freeze mechanics at 6 stacks:
  - Character becomes completely frozen (ice cube behavior)
  - Mass increased 5x for fast falling
  - No bouncing (restitution = 0)
  - Cannot deal melee damage
  - AI movement disabled
  - No horizontal/vertical movement (only gravity)
- Increased snowball speed from 300 to 500 for faster projectiles
- Increased freeze stack duration from 6 to 12 seconds per stack

