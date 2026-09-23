import assert from "node:assert/strict";
import test from "node:test";

import {
  TREE_FILTER_LAYER_IDS,
  TREE_INTERACTION_LAYER_ID,
  mapModePresentation,
  tourPresentation,
  treeHitTargetLayer,
} from "./map-mode.mjs";

test("give every tree the largest dot's hit area without changing its appearance", () => {
  assert.deepEqual(treeHitTargetLayer(), {
    id: "tree-hit-targets",
    type: "circle",
    source: "orchard",
    paint: {
      "circle-radius": 9,
      "circle-opacity": 0,
      "circle-stroke-opacity": 0,
    },
  });
  assert.equal(TREE_INTERACTION_LAYER_ID, "tree-hit-targets");
  assert.deepEqual(TREE_FILTER_LAYER_IDS, ["trees", "tree-hit-targets"]);
});

test("normal mode shows only planting-date controls and no status pins", () => {
  assert.deepEqual(mapModePresentation("normal"), {
    mode: "normal",
    filterPanelVisible: true,
    normalFiltersVisible: true,
    harvestFiltersVisible: false,
    plantingDateFilterActive: true,
    dangerPinsVisible: false,
    harvestPinsVisible: false,
  });
});

test("danger mode hides filter controls and shows only danger pins", () => {
  assert.deepEqual(mapModePresentation("danger"), {
    mode: "danger",
    filterPanelVisible: false,
    normalFiltersVisible: false,
    harvestFiltersVisible: false,
    plantingDateFilterActive: false,
    dangerPinsVisible: true,
    harvestPinsVisible: false,
  });
});

test("harvest mode shows only harvest controls and optional harvest pins", () => {
  assert.deepEqual(mapModePresentation("harvest"), {
    mode: "harvest",
    filterPanelVisible: true,
    normalFiltersVisible: false,
    harvestFiltersVisible: true,
    plantingDateFilterActive: false,
    dangerPinsVisible: false,
    harvestPinsVisible: true,
  });
  assert.equal(
    mapModePresentation("harvest", { harvestEnabled: false })
      .harvestPinsVisible,
    false,
  );
});

test("an active tour hides the other launcher", () => {
  assert.deepEqual(
    tourPresentation({
      canWater: true,
      canHarvest: true,
      wateringActive: true,
      harvestActive: false,
    }),
    {
      wateringLauncherVisible: true,
      harvestLauncherVisible: false,
    },
  );
  assert.deepEqual(
    tourPresentation({
      canWater: true,
      canHarvest: true,
      wateringActive: false,
      harvestActive: true,
    }),
    {
      wateringLauncherVisible: false,
      harvestLauncherVisible: true,
    },
  );
});

test("both tour launchers show while no tour is active", () => {
  assert.deepEqual(
    tourPresentation({
      canWater: true,
      canHarvest: true,
      wateringActive: false,
      harvestActive: false,
    }),
    {
      wateringLauncherVisible: true,
      harvestLauncherVisible: true,
    },
  );
});
