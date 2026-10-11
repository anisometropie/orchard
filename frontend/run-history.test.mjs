import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

import {
  harvestOutcomeLabel,
  harvestPartsLabel,
  harvestRunStatus,
  harvestTreeHistoryDetails,
  visibleHarvestTrees,
  wateringTreeOutcomeLabel,
} from "./run-history.mjs";

const indexHtml = fs.readFileSync(new URL("./index.html", import.meta.url), "utf8");

test("distinguish dead trees skipped during watering from watered and unfinished trees", () => {
  assert.equal(wateringTreeOutcomeLabel({ skipped_at_unix_seconds: 123, watered_at_unix_seconds: null }), "Skipped (dead)");
  assert.equal(wateringTreeOutcomeLabel({ skipped_at_unix_seconds: null, watered_at_unix_seconds: 123 }), "Watered");
  assert.equal(wateringTreeOutcomeLabel({ skipped_at_unix_seconds: null, watered_at_unix_seconds: null }), "Not watered");
});

test("describe harvest outcomes and distinguish completed from stopped tours", () => {
  assert.equal(harvestPartsLabel(["flower", "fruit"]), "Flowers, fruits");
  assert.equal(
    harvestOutcomeLabel({
      kind: "deferred",
      recorded_on: "2026-09-18",
      retry_on: "2026-09-25",
    }),
    "Done for now on 2026-09-18 · retry 2026-09-25",
  );
  assert.deepEqual(
    visibleHarvestTrees({
      trees: [
        { tree_id: 1, outcome: { kind: "harvested_everything" } },
        { tree_id: 2, outcome: { kind: "not_handled" } },
      ],
    }).map(({ tree_id }) => tree_id),
    [1],
  );
  assert.equal(
    harvestRunStatus({ handled_tree_count: 2, total_tree_count: 2 }),
    "Completed",
  );
  assert.equal(
    harvestRunStatus({ handled_tree_count: 1, total_tree_count: 2 }),
    "Stopped early",
  );
});

test("provide an owner history panel backed by the run-history endpoint", () => {
  assert.match(indexHtml, /id="history-toggle"/);
  assert.match(indexHtml, /id="history-panel"/);
  assert.match(indexHtml, /id="watering-history-tab"/);
  assert.match(indexHtml, /id="harvest-history-tab"/);
  assert.match(indexHtml, /id="watering-history-view"/);
  assert.match(indexHtml, /id="harvest-history-view"/);
  assert.match(indexHtml, /orchardResourceUrl\(currentAccess, "run-history"\)/);
});

test("show a normal harvested-all visit once with its modern wording", () => {
  assert.deepEqual(harvestTreeHistoryDetails({
    outcome: { kind: "harvested_everything", recorded_on: "2026-10-11" },
    observation: { kind: "harvested_all" },
    harvested_parts: ["fruit"],
    window_start: "2026-08-20", window_end: "2026-10-15",
  }), [
    "Harvested all on 2026-10-11",
    "Fruits · window 2026-08-20 to 2026-10-15",
  ]);
});

test("keep the original legacy outcome and a classification that adds different facts", () => {
  const tree = {
    outcome: { kind: "deferred", recorded_on: "2026-10-11", retry_on: "2026-10-18" },
    harvested_parts: ["fruit"],
    window_start: "2026-09-20", window_end: "2026-10-20",
  };
  assert.deepEqual(harvestTreeHistoryDetails(tree), [
    "Done for now on 2026-10-11 · retry 2026-10-18",
    "Fruits · window 2026-09-20 to 2026-10-20",
  ]);
  assert.deepEqual(harvestTreeHistoryDetails({
    ...tree, observation: { kind: "fruit_remaining", fruit_state: { ready: true, unripe: true } },
  }), [
    "Done for now on 2026-10-11 · retry 2026-10-18",
    "Fruits · window 2026-09-20 to 2026-10-20",
    "classified as: Ready now · Not ready yet",
  ]);
});

test("show modern fruit facts and retry date without a classification label", () => {
  assert.deepEqual(harvestTreeHistoryDetails({
    outcome: { kind: "fruit_remaining", recorded_on: "2026-10-11", retry_on: "2026-10-18" },
    observation: { kind: "fruit_remaining", fruit_state: { ready: true, unripe: true, wasted: true } },
    harvested_parts: ["fruit"],
    window_start: "2026-09-20", window_end: "2026-10-20",
  }), [
    "Ready now · Not ready yet · Some fruit wasted on 2026-10-11 · retry 2026-10-18",
    "Fruits · window 2026-09-20 to 2026-10-20",
  ]);
});
