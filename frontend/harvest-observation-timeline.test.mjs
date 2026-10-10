import test from "node:test";
import assert from "node:assert/strict";
import { harvestTimelineLayout } from "./harvest-observation-timeline.mjs";

const point = (recorded_on, window_start = "2026-09-01", window_end = "2026-09-30") => ({
  recorded_on, window_start, window_end,
});

test("the observation tick shows its actual position in the saved window, including both boundaries", () => {
  const middle = harvestTimelineLayout(point("2026-09-08"));
  assert.ok(Math.abs((middle.observed - middle.start) / (middle.end - middle.start) - 7 / 29) < 1e-10);
  assert.equal(middle.relativeDate, "Day 8 of 30");
  assert.equal(harvestTimelineLayout(point("2026-09-01")).observed, middle.start);
  assert.equal(harvestTimelineLayout(point("2026-09-30")).observed, middle.end);
  assert.match(middle.description, /2026-09-01 to 2026-09-30.*2026-09-08/);
});

test("visits outside the window remain outside the coloured range and on the visible axis", () => {
  const before = harvestTimelineLayout(point("2026-08-25"));
  assert.ok(0 < before.observed && before.observed < before.start);
  assert.equal(before.relativeDate, "7 days before window");
  const after = harvestTimelineLayout(point("2026-10-01"));
  assert.ok(after.end < after.observed && after.observed < 100);
  assert.equal(after.relativeDate, "1 day after window");
});

test("concrete dates handle windows crossing New Year and leap day", () => {
  const winter = harvestTimelineLayout(point("2027-01-05", "2026-12-20", "2027-01-10"));
  assert.equal(winter.relativeDate, "Day 17 of 22");
  assert.equal(winter.startLabel, "20 Dec 2026");
  assert.equal(winter.endLabel, "10 Jan 2027");
  assert.equal(harvestTimelineLayout(point("2028-02-29", "2028-02-28", "2028-03-01")).relativeDate, "Day 2 of 3");
});

test("a one-day window is visible at the observation tick; incomplete or invalid dates omit the diagram", () => {
  const single = harvestTimelineLayout(point("2026-09-01", "2026-09-01", "2026-09-01"));
  assert.equal(single.singleDay, true);
  assert.equal(single.observed, 50);
  assert.equal(single.start, single.end);
  assert.equal(single.relativeDate, "Day 1 of 1");
  for (const invalid of [{}, point("2026-02-30"), point("2026-09-01", "2026-10-01", "2026-09-30")]) {
    assert.equal(harvestTimelineLayout(invalid), null);
  }
});
