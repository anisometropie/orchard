const MAP_MODES = new Set(["normal", "danger", "harvest"]);

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
