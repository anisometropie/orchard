export const HARVEST_PIN_ICONS = Object.freeze({
  cone: "🌲",
  flower: "🌸",
  fruit: "🍎",
  leaf: "🍃",
  nut: "🌰",
  pod: "🫛",
  seed: "🌱",
  multiple: "🧺",
});

export function harvestPinImageName(parts) {
  const knownParts = [
    ...new Set(
      (Array.isArray(parts) ? parts : []).filter(
        (part) => part !== "multiple" && HARVEST_PIN_ICONS[part],
      ),
    ),
  ];
  if (knownParts.length === 0) return null;
  return knownParts.length === 1
    ? `harvest-${knownParts[0]}`
    : "harvest-multiple";
}

export function selectHarvestParts(availableParts, selectedParts) {
  const selected = new Set(selectedParts);
  return [...new Set(Array.isArray(availableParts) ? availableParts : [])].filter(
    (part) => selected.has(part),
  );
}
