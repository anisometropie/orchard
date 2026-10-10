import test from "node:test";
import assert from "node:assert/strict";
import { remainingFruitVisit, hasWindowObservations, speciesNeedsHarvestReview, firstRecordedFruit } from "./harvest-visit.mjs";

test("fruit states can combine, but waste alone finishes the tree", () => {
  assert.deepEqual(remainingFruitVisit({ unripe: true, wasted: true }), { kind: "fruit_remaining", fruit_state: { ready: false, unripe: true, wasted: true } });
  assert.deepEqual(remainingFruitVisit({ ready: true, unripe: true, wasted: true }), { kind: "fruit_remaining", fruit_state: { ready: true, unripe: true, wasted: true } });
  assert.deepEqual(remainingFruitVisit({ wasted: true }), { kind: "all_wasted" });
  assert.throws(() => remainingFruitVisit({}));
});
test("review icon requires fruit evidence for the exact species or cultivar, including reviewed evidence", () => {
  const points = [{ kind: "empty_on_arrival", owner_kind: "species", owner_id: 1 }, { kind: "harvested_all", reviewed: true, owner_kind: "cultivar", owner_id: 2 }];
  assert.equal(hasWindowObservations(points, { plantIdentityId: 1 }), false);
  assert.equal(hasWindowObservations(points, { plantIdentityId: 1, cultivarId: 2 }), true);
  assert.equal(hasWindowObservations(points, { plantIdentityId: 2 }), false);
});
test("first recorded fruit keeps absence history separate and measures from planting only if known", () => {
  const points = [{ kind: "empty_on_arrival", recorded_on: "2024-09-01" }, { kind: "all_wasted", recorded_on: "2026-09-01" }, { kind: "harvested_all", recorded_on: "2025-09-01" }];
  assert.equal(firstRecordedFruit(points, "2020-09-01").date, "2025-09-01");
  assert.ok(Math.abs(firstRecordedFruit(points, "2020-09-01").years - 5) < .01);
  assert.equal(firstRecordedFruit(points, null).years, null);
  assert.equal(firstRecordedFruit([points[0]], null), null);
});

test("species need review for their own or a planted cultivar's pending fruit observations", () => {
  const species = [
    { plantIdentityId: 1, cultivars: [] },
    { plantIdentityId: 2, cultivars: [{ id: 1 }] },
    { plantIdentityId: 3, cultivars: [{ id: 2 }] },
    { plantIdentityId: 4, cultivars: [] },
    { plantIdentityId: 5, cultivars: [] },
  ];
  const points = [
    { kind: "fruit_remaining", reviewed: false, owner_kind: "species", owner_id: 1 },
    { kind: "all_wasted", reviewed: false, owner_kind: "cultivar", owner_id: 1 },
    { kind: "harvested_all", reviewed: true, owner_kind: "cultivar", owner_id: 2 },
    { kind: "empty_on_arrival", reviewed: false, owner_kind: "species", owner_id: 4 },
    { kind: "fruit_remaining", reviewed: false, owner_kind: "species", owner_id: 99 },
  ];
  assert.deepEqual(species.filter((summary) => speciesNeedsHarvestReview(points, summary)), species.slice(0, 2));
  points[0].reviewed = true;
  assert.deepEqual(species.filter((summary) => speciesNeedsHarvestReview(points, summary)), [species[1]]);
  points[1].reviewed = true;
  assert.deepEqual(species.filter((summary) => speciesNeedsHarvestReview(points, summary)), []);
});
