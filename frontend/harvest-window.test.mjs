import assert from "node:assert/strict";
import test from "node:test";

import {
  harvestAvailability,
  harvestAvailabilitySummary,
  parseAnnualDate,
} from "./harvest-window.mjs";

test("parse a valid recurring month and day", () => {
  assert.deepEqual(parseAnnualDate("08-30"), { month: 8, day: 30 });
  assert.equal(parseAnnualDate("02-30"), null);
  assert.equal(parseAnnualDate("8-30"), null);
});

test("mark living fruit trees whose recurring window overlaps the selected weeks", () => {
  const result = harvestAvailability(
    [
      tree(1, "08-20", "09-05"),
      tree(2, "09-10", "09-20"),
      tree(3, "08-20", "09-05", { is_alive: false }),
      tree(4, "08-20", "09-05", { roles: ["pioneer"] }),
    ],
    new Date(2026, 7, 30),
    1,
  );

  assert.deepEqual(
    result.map(({ properties }) => properties.harvest_available),
    [true, false, false, false],
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

function tree(id, harvestStart, harvestEnd, overrides = {}) {
  return {
    id,
    properties: {
      plant_identity_id: id,
      roles: ["fruit"],
      is_alive: true,
      harvest_start: harvestStart,
      harvest_end: harvestEnd,
      ...overrides,
    },
  };
}
