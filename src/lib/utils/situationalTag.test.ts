import { getSituationalTagInfo } from "./situationalTag";

function assert(condition: boolean, msg: string) {
  if (!condition) {
    throw new Error(`Assertion failed: ${msg}`);
  }
}

// Test mapping for standard backend tags
const antiHeal = getSituationalTagInfo("Anti-Heal");
assert(!!antiHeal?.iconUrl?.endsWith("scalehpregen.png"), "Anti-Heal maps to scalehpregen.png");
assert(antiHeal?.title === "Anti-Heal", "Anti-Heal title matches");

const antiShield = getSituationalTagInfo("Anti-Shield");
assert(!!antiShield?.iconUrl?.endsWith("scalehealshield.png"), "Anti-Shield maps to scalehealshield.png");

const armorPen = getSituationalTagInfo("Armor Penetration");
assert(!!armorPen?.iconUrl?.endsWith("scaleapen.png"), "Armor Penetration maps to scaleapen.png");

const magicPen = getSituationalTagInfo("Magic Penetration");
assert(!!magicPen?.iconUrl?.endsWith("scalempen.png"), "Magic Penetration maps to scalempen.png");

const magicResist = getSituationalTagInfo("Magic Resist");
assert(!!magicResist?.iconUrl?.endsWith("scalemr.png"), "Magic Resist maps to scalemr.png");

const armorAntiBurst = getSituationalTagInfo("Armor / Anti-Burst");
assert(!!armorAntiBurst?.iconUrl?.endsWith("scalearmor.png"), "Armor / Anti-Burst maps to scalearmor.png");

const cleanse = getSituationalTagInfo("Cleanse / QSS");
assert(!!cleanse?.iconUrl?.endsWith("scaletenacity.png"), "Cleanse / QSS maps to scaletenacity.png");

// Test synergy and unmapped fallback
const synergy = getSituationalTagInfo("Synergy: +5% WR");
assert(!synergy?.iconUrl, "Synergy tag has no iconUrl");
assert(synergy?.tag === "Synergy: +5% WR", "Synergy tag label preserved");

const nullTag = getSituationalTagInfo(null);
assert(nullTag === null, "Null tag returns null");
