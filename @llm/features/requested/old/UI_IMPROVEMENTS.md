# UI Improvements

## 1. Sidebar:

 - Move the champion icon from the left of the rune title to be on the right of the keystone icon.

### 1.1 Champion search bar:
  - The portrait icon from the champions are square and have a small black border on the edges. Apply these styles to better display the icons:
    - button (container):
       - `position: relative;`
    - img (icon):
       - `position absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); height: calc(100% + 5px)`
  - Remove the left navigation arrow when there is nothing to the left to be scrolled to. Same with the right arrow.
  - Remove any champions that the id start with `600` - They are league classic champions and should not be visible to the player.
  - For the "all champions" icon:
    - Remove any other style and apply the following styles: `opacity: 80%; width: 70%; height: 70%;`
    - Use the following image as the asterisk (autofill icon in league): `https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-clash/global/default/assets/images/position-selector/positions/icon-position-fill-blue-hover.png`

## 2. Rune Tree/Rune Builder:
  - For all perk icons besides the stat shards:
    - Remove padding from the container and make the container be a circle in which the image fills it all, no background.
  - Remove the color accent from the pick rate bar and percentage and make them white with the opacity scaling with the pick rate.
  - Add back the name of the rows for each tree (instead of saying tier 1, tier 2, etc.)
  - Remove the WPA label from the right of the pick rate. Also remove the pick rate percentage. Instead, display the raw number of games played with said perk.
  - For the WPA, underneath the pick rate bar, add a system very similar to the pick rate display: the title on top and a bar at the bottom.
    - Since WPA can be negative, the bar will start at the middle and will fill out the left, in red, for negative WPA; and to the right in green for positive; Cap the values at -3 and +3 % WPA (bar will be half filled to the left in red when WPA is -3, and the opposite for positive);
    - The WPA will not be inside a tag like the current implementation and will instead be a regular text, like the pick rate.
  - Remove the texts "PICK 2 PERKS", "Any 2 tiers", "1 per row", "1 Keystone".
    - For primary and secondary tier, add the name of each row.