import assert from "node:assert/strict";
import test from "node:test";

import { mapModePresentation } from "./map-mode.mjs";

test("normal mode shows only planting-date controls and no rings", () => {
  assert.deepEqual(mapModePresentation("normal"), {
    mode: "normal",
    filterPanelVisible: true,
    normalFiltersVisible: true,
    harvestFiltersVisible: false,
    plantingDateFilterActive: true,
    dangerRingsVisible: false,
    harvestRingsVisible: false,
  });
});

test("danger mode hides filter controls and shows only danger rings", () => {
  assert.deepEqual(mapModePresentation("danger"), {
    mode: "danger",
    filterPanelVisible: false,
    normalFiltersVisible: false,
    harvestFiltersVisible: false,
    plantingDateFilterActive: false,
    dangerRingsVisible: true,
    harvestRingsVisible: false,
  });
});

test("harvest mode shows only harvest controls and optional harvest rings", () => {
  assert.deepEqual(mapModePresentation("harvest"), {
    mode: "harvest",
    filterPanelVisible: true,
    normalFiltersVisible: false,
    harvestFiltersVisible: true,
    plantingDateFilterActive: false,
    dangerRingsVisible: false,
    harvestRingsVisible: true,
  });
  assert.equal(
    mapModePresentation("harvest", { harvestEnabled: false })
      .harvestRingsVisible,
    false,
  );
});
