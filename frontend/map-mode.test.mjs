import assert from "node:assert/strict";
import test from "node:test";

import { mapModePresentation, tourPresentation } from "./map-mode.mjs";

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

test("an active tour hides the other launcher and shows one route control", () => {
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
      routeDisplayVisible: true,
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
      routeDisplayVisible: true,
    },
  );
});

test("route display stays hidden until a tour starts", () => {
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
      routeDisplayVisible: false,
    },
  );
});
