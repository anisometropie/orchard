import assert from "node:assert/strict";
import test from "node:test";

import {
  harvestAvailability,
  harvestAvailabilitySummary,
  harvestLayerFilter,
  parseAnnualDate,
} from "./harvest-window.mjs";

test("parse a valid recurring month and day", () => {
  assert.deepEqual(parseAnnualDate("08-30"), { month: 8, day: 30 });
  assert.equal(parseAnnualDate("02-30"), null);
  assert.equal(parseAnnualDate("8-30"), null);
});

test("mark living trees and expose the parts harvestable in the selected weeks", () => {
  const result = harvestAvailability(
    [
      tree(1, "08-20", "09-05"),
      tree(2, "09-10", "09-20"),
      tree(3, "08-20", "09-05", { is_alive: false }),
      tree(4, "08-20", "09-05", {
        roles: ["pioneer"],
        harvest_windows: [
          { start: "08-20", end: "09-05", harvested_part: "flower" },
        ],
      }),
    ],
    new Date(2026, 7, 30),
    1,
  );

  assert.deepEqual(
    result.map(({ properties }) => properties.harvest_available),
    [true, false, false, true],
  );
  assert.deepEqual(
    result.map(({ properties }) => properties.harvest_available_parts),
    [["fruit"], [], [], ["flower"]],
  );
});

test("support adjustable widths and harvest windows crossing New Year", () => {
  const result = harvestAvailability(
    [tree(1, "12-20", "01-10"), tree(2, "01-20", "02-01")],
    new Date(2026, 11, 28),
    2,
  );

  assert.deepEqual(
    result.map(({ properties }) => properties.harvest_available),
    [true, false],
  );
  assert.deepEqual(harvestAvailabilitySummary(result), {
    trees: 1,
    species: 1,
  });
});

test("match any of a cultivar's separate harvest waves", () => {
  const result = harvestAvailability(
    [tree(1, "06-10", "06-20", { harvest_windows: [
      { start: "06-10", end: "06-20" },
      { start: "08-01", end: "09-20" },
    ] })],
    new Date(2026, 7, 10),
    1,
  );

  assert.equal(result[0].properties.harvest_available, true);
});

test("do not turn February 29 into February 28 in non-leap years", () => {
  const result = harvestAvailability(
    [tree(1, "02-29", "02-29")],
    new Date(2026, 1, 28),
    1,
  );

  assert.equal(result[0].properties.harvest_available, false);
});

test("compose harvest availability with the planting-date filter", () => {
  const plantingFilter = [">=", ["get", "planted_on"], "2024-01-01"];
  assert.deepEqual(harvestLayerFilter(plantingFilter), [
    "all",
    ["==", ["get", "harvest_available"], true],
    plantingFilter,
  ]);
});

function tree(id, harvestStart, harvestEnd, overrides = {}) {
  return {
    id,
    properties: {
      plant_identity_id: id,
      roles: ["fruit"],
      is_alive: true,
      harvest_windows: [
        { start: harvestStart, end: harvestEnd, harvested_part: "fruit" },
      ],
      ...overrides,
    },
  };
}
