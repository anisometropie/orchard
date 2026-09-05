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
      longitude: -73.318,
      latitude: 12.476,
    }),
    {
      type: "FeatureCollection",
      features: [
        {
          type: "Feature",
          id: 12,
          geometry: { type: "Point", coordinates: [-73.318, 12.476] },
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
    longitude: -73.409,
    latitude: 12.476,
  }), {
    target: "danger",
    water_source: { longitude: -73.409, latitude: 12.476 },
  });
  assert.deepEqual(wateringStartRequest("row", "North"), {
    row_name: "North",
  });
});

test("default the water source a few metres north of Ronde de Bordeaux", () => {
  const source = defaultWaterSource([
    {
      geometry: { coordinates: [-73.4909, 12.2613] },
      properties: { name: "Apple" },
    },
    {
      geometry: { coordinates: [-73.4818, 12.2726] },
      properties: { name: "Figuier ‘Ronde de Bordeaux’" },
    },
  ]);

  assert.equal(source.longitude, -73.4818);
  assert.ok(source.latitude > 12.2726 && source.latitude < 12.2727);
  assert.deepEqual(waterSourceGeoJson(source).features[0].geometry.coordinates, [
    source.longitude,
    source.latitude,
  ]);
});

test("draw every two-can danger trip from the source and back", () => {
  const source = { longitude: -73.5, latitude: 12.25 };
  const route = [
    { longitude: -73.4909, latitude: 12.2613 },
    { longitude: -73.4818, latitude: 12.2726 },
    { longitude: -73.5091, latitude: 12.2839 },
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
            [-73.5, 12.25],
            [-73.4909, 12.2613],
            [-73.4818, 12.2726],
            [-73.5, 12.25],
          ],
        },
      },
      {
        type: "Feature",
        properties: { trip_parity: "odd" },
        geometry: {
          type: "LineString",
          coordinates: [
            [-73.5, 12.25],
            [-73.5091, 12.2839],
            [-73.5, 12.25],
          ],
        },
      },
    ],
  });
});

test("number every danger tree in route order and mark the current tree", () => {
  assert.deepEqual(
    dangerWateringNumberGeoJson([
      { id: 41, longitude: -73.4909, latitude: 12.2613 },
      { id: 57, longitude: -73.4818, latitude: 12.2726 },
      { id: 63, longitude: -73.4727, latitude: 12.2839 },
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
          geometry: { type: "Point", coordinates: [-73.4909, 12.2613] },
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
          geometry: { type: "Point", coordinates: [-73.4818, 12.2726] },
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
          geometry: { type: "Point", coordinates: [-73.4727, 12.2839] },
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
