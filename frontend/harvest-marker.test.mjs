import assert from "node:assert/strict";
import test from "node:test";

import {
  HARVEST_PIN_ICONS,
  harvestPinImageName,
  selectHarvestParts,
} from "./harvest-marker.mjs";

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
