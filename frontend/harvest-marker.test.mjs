import assert from "node:assert/strict";
import test from "node:test";

import {
  HARVEST_PIN_ICONS,
  harvestPartSelectedByDefault,
  harvestPinImageName,
  selectHarvestParts,
} from "./harvest-marker.mjs";

test("leave pod and seed out of the default harvest filter", () => {
  assert.equal(harvestPartSelectedByDefault("fruit"), true);
  assert.equal(harvestPartSelectedByDefault("pod"), false);
  assert.equal(harvestPartSelectedByDefault("seed"), false);
});

test("select a representative pin icon for the harvestable parts", () => {
  assert.deepEqual(Object.keys(HARVEST_PIN_ICONS), [
    "cone",
    "flower",
    "fruit",
    "leaf",
    "nut",
    "pod",
    "seed",
    "multiple",
  ]);
  assert.equal(harvestPinImageName(["fruit"]), "harvest-fruit");
  assert.equal(harvestPinImageName(["fruit", "fruit"]), "harvest-fruit");
  assert.equal(
    harvestPinImageName(["flower", "seed"]),
    "harvest-multiple",
  );
  assert.equal(harvestPinImageName([]), null);
});

test("keep only harvestable parts selected by the user", () => {
  assert.deepEqual(
    selectHarvestParts(
      ["fruit", "flower", "fruit", "seed"],
      ["flower", "nut"],
    ),
    ["flower"],
  );
  assert.deepEqual(selectHarvestParts(["fruit"], []), []);
});
