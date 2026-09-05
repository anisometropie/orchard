import test from "node:test";
import assert from "node:assert/strict";

import {
  appendManualTree,
  dangerTreeCount,
  orchardRows,
  treeIdsInRow,
  wateringStartRequest,
  wateringTargetGeoJson,
} from "./watering.mjs";

const feature = (id, rowName, rowRank, isAlive = true) => ({
  id,
  properties: {
    row_name: rowName,
    row_rank: rowRank,
    is_alive: isAlive,
  },
});

test("list named rows and show whether every tree has a complete saved order", () => {
  const rows = orchardRows([
    feature(1, "South", 2),
    feature(2, "North", 1),
    feature(3, "South", 1, false),
    feature(4, "Unordered", null),
    feature(5, null, null),
  ]);

  assert.deepEqual(rows, [
    { name: "North", treeCount: 1, livingTreeCount: 1, isOrdered: true },
    { name: "South", treeCount: 2, livingTreeCount: 1, isOrdered: true },
    {
      name: "Unordered",
      treeCount: 1,
      livingTreeCount: 1,
      isOrdered: false,
    },
  ]);
});

test("manual ordering accepts each tree in the selected row exactly once", () => {
  const features = [feature(1, "North", null), feature(2, "North", null)];
  const allowed = treeIdsInRow(features, "North");

  assert.deepEqual(appendManualTree([], 2, allowed), [2]);
  assert.deepEqual(appendManualTree([2], 2, allowed), [2]);
  assert.deepEqual(appendManualTree([2], 9, allowed), [2]);
  assert.deepEqual(appendManualTree([2], 1, allowed), [2, 1]);
});

test("put only the current watering tree in the target pin source", () => {
  assert.deepEqual(wateringTargetGeoJson(null), {
    type: "FeatureCollection",
    features: [],
  });
  assert.deepEqual(
    wateringTargetGeoJson({
      id: 12,
      name: "Apple two",
      longitude: 5.2,
      latitude: 45.2,
    }),
    {
      type: "FeatureCollection",
      features: [
        {
          type: "Feature",
          id: 12,
          geometry: { type: "Point", coordinates: [5.2, 45.2] },
          properties: { name: "Apple two" },
        },
      ],
    },
  );
});

test("start danger watering without depending on saved row order", () => {
  const features = [
    { properties: { is_alive: true, is_in_danger: true } },
    { properties: { is_alive: true, is_in_danger: false } },
    { properties: { is_alive: false, is_in_danger: true } },
  ];

  assert.equal(dangerTreeCount(features), 1);
  assert.deepEqual(wateringStartRequest("danger", "North"), {
    target: "danger",
  });
  assert.deepEqual(wateringStartRequest("row", "North"), {
    row_name: "North",
  });
});
