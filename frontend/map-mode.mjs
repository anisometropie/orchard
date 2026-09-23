const MAP_MODES = new Set(["normal", "danger", "harvest"]);

export const TREE_INTERACTION_LAYER_ID = "tree-hit-targets";
export const TREE_FILTER_LAYER_IDS = Object.freeze([
  "trees",
  TREE_INTERACTION_LAYER_ID,
]);

export function treeHitTargetLayer() {
  return {
    id: TREE_INTERACTION_LAYER_ID,
    type: "circle",
    source: "orchard",
    paint: {
      "circle-radius": 9,
      "circle-opacity": 0,
      "circle-stroke-opacity": 0,
    },
  };
}

export function mapModePresentation(mode, { harvestEnabled = true } = {}) {
  if (!MAP_MODES.has(mode)) throw new Error(`Unknown map mode: ${mode}`);

  return {
    mode,
    filterPanelVisible: mode !== "danger",
    normalFiltersVisible: mode === "normal",
    harvestFiltersVisible: mode === "harvest",
    plantingDateFilterActive: mode === "normal",
    dangerPinsVisible: mode === "danger",
    harvestPinsVisible: mode === "harvest" && harvestEnabled,
  };
}

export function tourPresentation({
  canWater = false,
  canHarvest = false,
  wateringActive = false,
  harvestActive = false,
} = {}) {
  return {
    wateringLauncherVisible: canWater && !harvestActive,
    harvestLauncherVisible: canHarvest && !wateringActive,
  };
}
