import test from "node:test";
import assert from "node:assert/strict";
import { harvestObservationPath, pendingObservationIds, harvestReviewRequest } from "./harvest-review.mjs";

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
