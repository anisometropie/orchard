import test from "node:test";
import assert from "node:assert/strict";
import { harvestObservationPath, pendingObservationIds, harvestReviewRequest, observationsForHarvestWindow } from "./harvest-review.mjs";

test("review is scoped to the icon's exact species or cultivar", () => {
  assert.equal(harvestObservationPath({ plantIdentityId: 4 }), "harvest-observations/species/4");
  assert.equal(harvestObservationPath({ plantIdentityId: 4, cultivarId: 9 }), "harvest-observations/cultivar/9");
  assert.throws(() => harvestObservationPath({ cultivarId: 0 }));
});

test("keep windows reviews only selected points; window edits are explicitly submitted", () => {
  const points = [{ id: 1, reviewed: true }, { id: 2, reviewed: false }, { id: 3, reviewed: false }];
  assert.deepEqual(pendingObservationIds(points), [2, 3]);
  assert.deepEqual(harvestReviewRequest([2]), { observation_ids: [2], window_change: null });
  const change = { reference_region: "Paris", windows: [{ start: { month: 9, day: 1 }, end: { month: 10, day: 5 }, harvested_part: "fruit" }] };
  assert.deepEqual(harvestReviewRequest([2], change), { observation_ids: [2], window_change: change });
  for (const ids of [[], [2, 2], [0], ["2"]]) assert.throws(() => harvestReviewRequest(ids));
  assert.deepEqual(points.map((point) => point.reviewed), [true, false, false]);
});

test("review one recurring window at a time, even when the same tree was observed in two windows", () => {
  const schedule = { plantIdentityId: 4 };
  const window = { start: "09-01", end: "09-30", harvested_part: "fruit" };
  const point = { tree_id: 1, owner_kind: "species", owner_id: 4, kind: "fruit_remaining",
    window_start: "2026-09-01", window_end: "2026-09-30", harvested_parts: ["fruit"], reviewed: false };
  const points = [
    { ...point, id: 1, recorded_on: "2026-08-28" },
    { ...point, id: 2, window_start: "2026-06-01", window_end: "2026-06-30" },
    { ...point, id: 3, window_start: "2025-09-01", window_end: "2025-09-30", reviewed: true },
    { ...point, id: 4, harvested_parts: ["flower"] },
    { ...point, id: 5, owner_kind: "cultivar", owner_id: 4 },
    { ...point, id: 6, owner_id: 9 },
    { ...point, id: 7, kind: "empty_on_arrival" },
  ];
  const scoped = observationsForHarvestWindow(points, schedule, window);
  assert.deepEqual(scoped.map(({ id }) => id), [1, 3]);
  assert.deepEqual(pendingObservationIds(scoped), [1]);
  assert.deepEqual(observationsForHarvestWindow(points, schedule,
    { start: "06-01", end: "06-30", harvested_part: "fruit" }).map(({ id }) => id), [2]);
  assert.deepEqual(observationsForHarvestWindow(points, { ...schedule, cultivarId: 4 }, window).map(({ id }) => id), [5]);
});

test("a winter window uses the saved period, including observations outside its dates", () => {
  const points = [{ id: 1, owner_kind: "species", owner_id: 4, kind: "too_late",
    recorded_on: "2027-01-20", window_start: "2026-12-20", window_end: "2027-01-10", harvested_parts: ["fruit"] }];
  assert.deepEqual(observationsForHarvestWindow(points, { plantIdentityId: 4 },
    { start: "12-20", end: "01-10", harvested_part: "fruit" }), points);
});

test("a scoped review submits the target window separately from its replacement", () => {
  const window = { start: "09-01", end: "09-30", harvested_part: "fruit", reference_region: "France" };
  const change = { reference_region: "Paris", windows: [{ start: { month: 9, day: 5 }, end: { month: 10, day: 5 }, harvested_part: "fruit" }] };
  assert.deepEqual(harvestReviewRequest([2], change, window), {
    observation_ids: [2], window_change: change,
    window: { start: { month: 9, day: 1 }, end: { month: 9, day: 30 }, harvested_part: "fruit" },
  });
});
