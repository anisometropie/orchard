import test from "node:test";
import assert from "node:assert/strict";
import { observationWindows, observationPeriod, nearestObservationSeason, standaloneObservationRequest, unreviewedObservationCues } from "./standalone-harvest-observation.mjs";

test("any tree can record facts without a period, including empty and wasted observations", () => {
  assert.deepEqual(observationWindows({}, "flower"), []);
  const base = { recordedOn: "2026-04-12", part: "flower", kind: "fruit_remaining", fruitState: { ready: true, unripe: true } };
  assert.deepEqual(standaloneObservationRequest(base), { action_date: "2026-04-12", harvested_part: "flower", kind: "fruit_remaining", fruit_state: { ready: true, unripe: true, wasted: false }, window: null });
  assert.equal(standaloneObservationRequest({ ...base, fruitState: { wasted: true } }).kind, "all_wasted");
  assert.equal(standaloneObservationRequest({ ...base, kind: "empty_on_arrival" }).kind, "empty_on_arrival");
  assert.throws(() => standaloneObservationRequest({ ...base, recordedOn: "2026-02-30" }));
  assert.throws(() => standaloneObservationRequest({ ...base, fruitState: {} }));
});

test("saved periods retain their part and explicit season, including winter and leap dates", () => {
  const winter = { start: "12-10", end: "01-20", harvested_part: "fruit" };
  const properties = { harvest_windows: JSON.stringify([winter, { ...winter, harvested_part: "flower" }]) };
  assert.deepEqual(observationWindows(properties, "fruit"), [winter]);
  assert.equal(nearestObservationSeason(winter, "2026-01-25"), 2025);
  assert.deepEqual(observationPeriod(winter, 2025), { start: "2025-12-10", end: "2026-01-20" });
  const request = standaloneObservationRequest({ recordedOn: "2026-01-25", part: "fruit", kind: "harvested_all", window: winter, startYear: 2025 });
  assert.deepEqual(request.window, { start: { month: 12, day: 10 }, end: { month: 1, day: 20 }, start_year: 2025 });
  assert.throws(() => observationPeriod({ start: "02-29", end: "03-01" }, 2026));
  assert.equal(observationPeriod({ start: "02-29", end: "03-01" }, 2028).start, "2028-02-29");
});

test("creating a window shows only unreviewed unbound cues for its owner and parts", () => {
  const point = { id: 1, owner_kind: "species", owner_id: 8, recorded_on: "2026-04-12", harvested_parts: ["flower"], window_start: null, window_end: null, reviewed: false };
  const points = [point, { ...point, id: 2, harvested_parts: ["fruit"] }, { ...point, id: 3, reviewed: true }, { ...point, id: 4, owner_id: 9 }, { ...point, id: 5, owner_kind: "cultivar" }, { ...point, id: 6, window_start: "2026-04-01", window_end: "2026-04-30" }, { ...point, id: 7, kind: "empty_on_arrival" }];
  assert.deepEqual(unreviewedObservationCues(points, { plantIdentityId: 8 }, ["flower"]).map(p => p.id), [1, 7]);
  assert.deepEqual(unreviewedObservationCues(points, { plantIdentityId: 8 }).map(p => p.id), [1, 2, 7]);
  assert.deepEqual(unreviewedObservationCues(points, { plantIdentityId: 8, cultivarId: 8 }, ["flower"]).map(p => p.id), [5]);
});
