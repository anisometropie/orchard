import assert from "node:assert/strict";
import test from "node:test";

import {
  availableHarvestSpecies,
  harvestConflictMessage,
  harvestRouteNumberGeoJson,
  harvestRoutePathGeoJson,
  harvestRouteWindow,
  harvestStartRequest,
  harvestTargetGeoJson,
  localIsoDate,
} from "./harvest-job.mjs";

test("format the calendar date in the browser's local timezone", () => {
  assert.equal(localIsoDate(new Date(2026, 8, 17, 23, 45)), "2026-09-17");
});

test("build all-species and one-species harvest start requests", () => {
  const onDate = new Date(2026, 8, 17, 23, 45);

  assert.deepEqual(harvestStartRequest("all", null, onDate), {
    target: "all",
    plant_identity_id: null,
    harvested_parts: ["fruit"],
    on_date: "2026-09-17",
  });
  assert.deepEqual(harvestStartRequest("species", "12", onDate), {
    target: "species",
    plant_identity_id: 12,
    harvested_parts: ["fruit"],
    on_date: "2026-09-17",
  });
});

test("include the selected harvested parts in a harvest start request", () => {
  assert.deepEqual(
    harvestStartRequest("all", null, "2026-09-17", ["cone", "flower"]),
    {
      target: "all",
      plant_identity_id: null,
      harvested_parts: ["cone", "flower"],
      on_date: "2026-09-17",
    },
  );
});

test("explain harvest conflicts that require a fresh tour", () => {
  assert.equal(
    harvestConflictMessage("harvest_action_date_outside_run_period"),
    "Today's date is outside the harvest period captured by this tour. Cancel this tour and start a new one.",
  );
  assert.equal(
    harvestConflictMessage("harvest_window_changed"),
    "This tree's harvest window changed after the tour started. Cancel this tour and start a new one.",
  );
  assert.equal(
    harvestConflictMessage("tree_is_not_current"),
    "Harvest progress changed. Reload the page to resume.",
  );
});

test("list species with a living tree in a fruit window on the exact date", () => {
  const features = [
    tree(1, 10, "Apple", "Malus domestica", [
      { start: "09-17", end: "09-17", harvested_part: "fruit" },
    ]),
    tree(2, 10, "Apple", "Malus domestica", JSON.stringify([
      { start: "09-10", end: "09-20" },
    ])),
    tree(3, 20, "Pear", "Pyrus communis", [
      { start: "09-18", end: "09-25", harvested_part: "fruit" },
    ]),
    tree(4, 30, "Lime", "Tilia cordata", [
      { start: "09-10", end: "09-20", harvested_part: "flower" },
    ]),
    tree(5, 40, "Plum", "Prunus domestica", [
      { start: "09-10", end: "09-20", harvested_part: "fruit" },
    ], false),
  ];

  assert.deepEqual(availableHarvestSpecies(features, "2026-09-17"), [
    {
      plantIdentityId: 10,
      label: "Apple · Malus domestica",
      treeCount: 2,
    },
  ]);
});

test("list species only for the selected harvested parts", () => {
  const features = [
    tree(1, 10, "Pine", "Pinus", [
      { start: "09-10", end: "09-20", harvested_part: "cone" },
      { start: "09-10", end: "09-20", harvested_part: "fruit" },
    ]),
    tree(2, 20, "Lime", "Tilia", [
      { start: "09-10", end: "09-20", harvested_part: "flower" },
    ]),
    tree(3, 30, "Apple", "Malus", [
      { start: "09-10", end: "09-20", harvested_part: "fruit" },
    ]),
  ];

  assert.deepEqual(
    availableHarvestSpecies(
      features,
      "2026-09-17",
      null,
      ["cone", "flower"],
    ).map(({ plantIdentityId }) => plantIdentityId),
    [20, 10],
  );
});

test("list species from only the persisted eligible harvest trees", () => {
  const features = [
    tree(1, 10, "Apple", "Malus domestica", [
      { start: "09-10", end: "09-20", harvested_part: "fruit" },
    ]),
    tree(2, 10, "Apple", "Malus domestica", [
      { start: "09-10", end: "09-20", harvested_part: "fruit" },
    ]),
    tree(3, 20, "Pear", "Pyrus communis", [
      { start: "09-10", end: "09-20", harvested_part: "fruit" },
    ]),
  ];

  assert.deepEqual(
    availableHarvestSpecies(features, "2026-09-17", ["2", 3]),
    [
      {
        plantIdentityId: 10,
        label: "Apple · Malus domestica",
        treeCount: 1,
      },
      {
        plantIdentityId: 20,
        label: "Pear · Pyrus communis",
        treeCount: 1,
      },
    ],
  );
  assert.deepEqual(
    availableHarvestSpecies(features, "2026-09-17", new Set([3])),
    [
      {
        plantIdentityId: 20,
        label: "Pear · Pyrus communis",
        treeCount: 1,
      },
    ],
  );
  assert.deepEqual(
    availableHarvestSpecies(features, "2026-09-17", []),
    [],
  );
});

test("match fruit windows that cross New Year", () => {
  const features = [
    tree(1, 20, "Pear", "Pyrus communis", [
      { start: "12-20", end: "01-10", harvested_part: "fruit" },
    ]),
  ];

  assert.equal(availableHarvestSpecies(features, "2026-12-25").length, 1);
  assert.equal(availableHarvestSpecies(features, "2027-01-05").length, 1);
  assert.equal(availableHarvestSpecies(features, "2027-01-11").length, 0);
});

test("match February 29 only in a leap year", () => {
  const features = [
    tree(1, 10, "Apple", "Malus domestica", [
      { start: "02-29", end: "02-29", harvested_part: "fruit" },
    ]),
  ];

  assert.equal(availableHarvestSpecies(features, "2028-02-29").length, 1);
  assert.equal(availableHarvestSpecies(features, "2027-02-28").length, 0);
});

test("show only the current tree or a window of four including it", () => {
  const route = [1, 2, 3, 4, 5, 6].map(routeTree);

  assert.deepEqual(harvestRouteWindow(route, 2, "next"), {
    trees: [route[1]],
    startIndex: 1,
  });
  const visible = harvestRouteWindow(route, 2, "four");
  assert.deepEqual(visible, {
    trees: route.slice(1, 5),
    startIndex: 1,
  });
  assert.deepEqual(
    harvestRouteNumberGeoJson(
      visible.trees,
      2,
      visible.startIndex,
    ).features.map(({ properties }) => ({
      number: properties.route_number,
      current: properties.is_current,
    })),
    [
      { number: 2, current: true },
      { number: 3, current: false },
      { number: 4, current: false },
      { number: 5, current: false },
    ],
  );
  assert.equal(harvestTargetGeoJson(route[1]).features[0].id, 2);
});

test("draw a direct path through the visible harvest route", () => {
  const route = [routeTree(2), routeTree(3), routeTree(4)];

  assert.deepEqual(harvestRoutePathGeoJson(route), {
    type: "FeatureCollection",
    features: [
      {
        type: "Feature",
        properties: {},
        geometry: {
          type: "LineString",
          coordinates: [
            [-73.4818, 12.2726],
            [-73.4727, 12.2839],
            [-73.4636, 12.2952],
          ],
        },
      },
    ],
  });
  assert.deepEqual(harvestRoutePathGeoJson([route[0]]), {
    type: "FeatureCollection",
    features: [],
  });
});

function tree(
  id,
  plantIdentityId,
  commonName,
  taxonName,
  harvestWindows,
  isAlive = true,
) {
  return {
    id,
    properties: {
      plant_identity_id: plantIdentityId,
      plant_identity_name: commonName,
      plant_identity_taxon_name: taxonName,
      harvest_windows: harvestWindows,
      is_alive: isAlive,
    },
  };
}

function routeTree(id) {
  return {
    id,
    name: `Tree ${id}`,
    longitude: -73.5 + 0.91 * id / 100,
    latitude: 12.25 + 1.13 * id / 100,
  };
}
