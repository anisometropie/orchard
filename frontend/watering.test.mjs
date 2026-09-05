import test from "node:test";
import assert from "node:assert/strict";

import {
  appendManualTree,
  defaultWaterSource,
  dangerWateringNumberGeoJson,
  dangerWateringNumberMarker,
  dangerWateringPathGeoJson,
  dangerTreeCount,
  orchardRows,
  treeIdsInRow,
  wateringStartRequest,
  waterSourceGeoJson,
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
  assert.deepEqual(wateringStartRequest("danger", "North", {
    longitude: 5.1,
    latitude: 45.2,
  }), {
    target: "danger",
    water_source: { longitude: 5.1, latitude: 45.2 },
  });
  assert.deepEqual(wateringStartRequest("row", "North"), {
    row_name: "North",
  });
});

test("default the water source a few metres north of Ronde de Bordeaux", () => {
  const source = defaultWaterSource([
    {
      geometry: { coordinates: [5.01, 45.01] },
      properties: { name: "Apple" },
    },
    {
      geometry: { coordinates: [5.02, 45.02] },
      properties: { name: "Figuier ‘Ronde de Bordeaux’" },
    },
  ]);

  assert.equal(source.longitude, 5.02);
  assert.ok(source.latitude > 45.02004 && source.latitude < 45.02005);
  assert.deepEqual(waterSourceGeoJson(source).features[0].geometry.coordinates, [
    source.longitude,
    source.latitude,
  ]);
});

test("draw every two-can danger trip from the source and back", () => {
  const source = { longitude: 5.0, latitude: 45.0 };
  const route = [
    { longitude: 5.01, latitude: 45.01 },
    { longitude: 5.02, latitude: 45.02 },
    { longitude: 4.99, latitude: 45.03 },
  ];

  assert.deepEqual(dangerWateringPathGeoJson(source, route), {
    type: "FeatureCollection",
    features: [
      {
        type: "Feature",
        properties: { trip_parity: "even" },
        geometry: {
          type: "LineString",
          coordinates: [
            [5.0, 45.0],
            [5.01, 45.01],
            [5.02, 45.02],
            [5.0, 45.0],
          ],
        },
      },
      {
        type: "Feature",
        properties: { trip_parity: "odd" },
        geometry: {
          type: "LineString",
          coordinates: [
            [5.0, 45.0],
            [4.99, 45.03],
            [5.0, 45.0],
          ],
        },
      },
    ],
  });
});

test("number every danger tree in route order and mark the current tree", () => {
  assert.deepEqual(
    dangerWateringNumberGeoJson([
      { id: 41, longitude: 5.01, latitude: 45.01 },
      { id: 57, longitude: 5.02, latitude: 45.02 },
      { id: 63, longitude: 5.03, latitude: 45.03 },
    ], 57),
    {
      type: "FeatureCollection",
      features: [
        {
          type: "Feature",
          id: 41,
          properties: {
            route_number: 1,
            route_number_image: "watering-route-number-1",
            trip_parity: "even",
            is_current: false,
            route_number_scale: 1,
          },
          geometry: { type: "Point", coordinates: [5.01, 45.01] },
        },
        {
          type: "Feature",
          id: 57,
          properties: {
            route_number: 2,
            route_number_image: "watering-route-current-2",
            trip_parity: "even",
            is_current: true,
            route_number_scale: 1.35,
          },
          geometry: { type: "Point", coordinates: [5.02, 45.02] },
        },
        {
          type: "Feature",
          id: 63,
          properties: {
            route_number: 3,
            route_number_image: "watering-route-number-3",
            trip_parity: "odd",
            is_current: false,
            route_number_scale: 1,
          },
          geometry: { type: "Point", coordinates: [5.03, 45.03] },
        },
      ],
    },
  );
});

test("make the current route number larger and red", () => {
  assert.deepEqual(dangerWateringNumberMarker(1, false), {
    imageName: "watering-route-number-2",
    fillColor: "#1677b8",
    scale: 1,
  });
  assert.deepEqual(dangerWateringNumberMarker(1, true), {
    imageName: "watering-route-current-2",
    fillColor: "#d51f2e",
    scale: 1.35,
  });
});
