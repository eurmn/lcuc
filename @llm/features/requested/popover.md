A new feature will be implemented into the application. A popover information upon hovering a rune card. It will trigger on hover after 0.5s or 1s if the player is currently editing the rune page.

Rune information is available under https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perks.json.

the response is an array with multiple entries similar to this one:

[
  {
    "id": 8369,
    "name": "First Strike",
    "majorChangePatchVersion": "11.23",
    "tooltip": "Attacks or abilities against an enemy champion within @GraceWindow.2@s of entering champion combat grant @GoldProcBonus@ gold and <b>First Strike</b> for @Duration@ seconds, causing you to deal <truedamage>@DamageAmp*100@%</truedamage> extra <truedamage>damage</truedamage> against champions, and granting <gold>{{ Item_Melee_Ranged_Split }}</gold> of that damage dealt as <gold>gold</gold>.<br><br>Cooldown: <scaleLevel>@Cooldown@</scaleLevel>s<br><hr><br>Damage Dealt: @f1@<br>Gold Gained: @f2@",
    "shortDesc": "When you initiate champion combat, deal 7% extra damage for 3 seconds and gain gold based on damage dealt.",
    "longDesc": "Attacks or abilities against an enemy champion within 0.25s of entering champion combat grant 10 gold and <b>First Strike</b> for 3 seconds, causing you to deal <truedamage>7%</truedamage> extra <truedamage> damage</truedamage> against champions, and granting <gold>50% (35% for ranged champions)</gold> of bonus damage dealt as <gold>gold</gold>.<br><br>Cooldown: <scaleLevel>25 - 15</scaleLevel>s",
    "recommendationDescriptor": "True Damage, Income",
    "iconPath": "/lol-game-data/assets/v1/perk-images/Styles/Inspiration/FirstStrike/FirstStrike.png",
    "endOfGameStatDescs": [
      "Damage Dealt: @eogvar1@",
      "Gold Gained: @eogvar2@"
    ],
    "recommendationDescriptorAttributes": {
      
    }
  }
]

The tooltip popup will display the rune icon (same as already used in the current version), its name and below it, the description.
The game will display the "longDesc" text as the description.
The descriptions from perks.json uses formatting similar to the HTML. Here are all the tags it uses:
[
    "b",
    "truedamage",
    "gold",
    "scaleLevel",
    "rules",
    "font",
    "speed",
    "i",
    "scaleAP",
    "scaleAD",
    "keywordMajor",
    "statGood",
    "attention",
    "scaleHealth",
    "scaleMana",
    "status",
    "scalehealth",
    "trueDamage"
]

the tags need to be parsed and rendered accordingly. some of them are regular HTML tags (b, i). "font" is always used as <font color="#..."> and it will be used to change the rendered color of the  text within it.
tags that relate to league of legends stats (ad, ap, speed, gold, etc.) will make its text change colors according to the property. if the tag has a property icon related to it, render it before its text with the icon being the same size of the text itself.

the asset icons inserted into the rendered component can be found at 
https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/fonts/texticons/lol/statsicon/(name).png

the list of all icons in this folder is:
[
    "dodgechance.png",
    "onhit.png",
    "scalead.png", // scaleAD
    "scaleadaptiveforce.png", // adaptive force
    "scaleah.png", // ability haste
    "scaleap.png", // scaleAP
    "scaleapen.png",
    "scalearmor.png",
    "scaleas.png",
    "scaleaugmentlevel.arena_2026_s2.png",
    "scalecapstone.arena_2026_s2.png",
    "scalecooldown.png",
    "scalecrit.png",
    "scalecritmult.png",
    "scaleda.png",
    "scaledr.png",
    "scalehealshield.png",
    "scalehealth.png", // scaleHealth
    "scalehpregen.png",
    "scalelevel.png", // scaleLevel
    "scalels.png",
    "scalemana.png", // scaleMana
    "scalemanaregen.png",
    "scalempen.png",
    "scalemr.png",
    "scalems.png", // speed
    "scalerange.png",
    "scalesize.png",
    "scalesv.png",
    "scaletenacity.png"
]

gold icon can be found at https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/fonts/texticons/lol/gameplay/goldcoins.png
true damage can be found at https://raw.communitydragon.org/latest/game/assets/ux/floatingtext/enhancedtruedamageicon.png

use svelte-flowbite for the new popover component as described by FLOWBITE_SVELTE. 