import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  accessHeaders,
  canHarvestOrchard,
  canWaterOrchard,
  hasOpenOrchard,
  orchardResourceUrl,
  resolveOrchardAccess,
  sharedOrchardUrl,
} from "./orchard-access.mjs";

const indexHtml = readFileSync(new URL("./index.html", import.meta.url), "utf8");

test("show orchard controls only while an owned or shared orchard is open", () => {
  assert.equal(hasOpenOrchard({ mode: "empty" }), false);
  assert.equal(hasOpenOrchard({ mode: "login-required" }), false);
  assert.equal(hasOpenOrchard({ mode: "editable" }), true);
  assert.equal(hasOpenOrchard({ mode: "read-only" }), true);
  assert.equal(hasOpenOrchard({ mode: "watering" }), true);
  assert.equal(hasOpenOrchard({ mode: "harvest-watering" }), true);
});

test("allow only an owner or watering-link visitor to water the open orchard", () => {
  assert.equal(canWaterOrchard({ mode: "empty" }), false);
  assert.equal(canWaterOrchard({ mode: "login-required" }), false);
  assert.equal(canWaterOrchard({ mode: "editable" }), true);
  assert.equal(canWaterOrchard({ mode: "read-only" }), false);
  assert.equal(canWaterOrchard({ mode: "watering" }), true);
  assert.equal(canWaterOrchard({ mode: "harvest-watering" }), true);
});

test("allow only an owner or combined-link visitor to harvest the open orchard", () => {
  assert.equal(canHarvestOrchard({ mode: "empty" }), false);
  assert.equal(canHarvestOrchard({ mode: "login-required" }), false);
  assert.equal(canHarvestOrchard({ mode: "editable" }), true);
  assert.equal(canHarvestOrchard({ mode: "read-only" }), false);
  assert.equal(canHarvestOrchard({ mode: "watering" }), false);
  assert.equal(canHarvestOrchard({ mode: "harvest-watering" }), true);
});

test("a watering fragment opens one orchard with watering access", () => {
  const access = resolveOrchardAccess(
    "#/orchards/42/share/watering/O8xz_watering-token",
    null,
  );

  assert.deepEqual(access, {
    mode: "watering",
    orchardId: 42,
    shareToken: "O8xz_watering-token",
  });
  assert.deepEqual(accessHeaders(access), {
    "x-orchard-share-token": "O8xz_watering-token",
  });
});

test("a combined fragment opens one orchard with harvest and watering access", () => {
  const access = resolveOrchardAccess(
    "#/orchards/42/share/harvest-watering/O8xz-worker-token",
    null,
  );

  assert.deepEqual(access, {
    mode: "harvest-watering",
    orchardId: 42,
    shareToken: "O8xz-worker-token",
  });
  assert.deepEqual(accessHeaders(access), {
    "x-orchard-share-token": "O8xz-worker-token",
  });
});

test("an unlinked visit has no orchard and loads no private tree URL", () => {
  const access = resolveOrchardAccess("", null);

  assert.deepEqual(access, { mode: "empty", orchardId: null });
  assert.throws(() => orchardResourceUrl(access, "trees.geojson"));
});

test("a shared fragment opens one orchard read only without putting the secret in requests", () => {
  const access = resolveOrchardAccess(
    "#/orchards/42/share/O8xz_private-token",
    null,
  );

  assert.deepEqual(access, {
    mode: "read-only",
    orchardId: 42,
    shareToken: "O8xz_private-token",
  });
  assert.equal(
    orchardResourceUrl(access, "trees.geojson"),
    "/api/orchards/42/trees.geojson",
  );
  assert.deepEqual(accessHeaders(access), {
    "x-orchard-share-token": "O8xz_private-token",
  });
});

test("only a session owner can open the editable orchard fragment", () => {
  const session = { orchards: [{ id: 7, name: "North field" }] };

  assert.deepEqual(resolveOrchardAccess("#/orchards/7", session), {
    mode: "editable",
    orchardId: 7,
    orchard: session.orchards[0],
  });
  assert.deepEqual(resolveOrchardAccess("#/orchards/8", session), {
    mode: "empty",
    orchardId: null,
  });
  assert.deepEqual(resolveOrchardAccess("#/orchards/7", null), {
    mode: "login-required",
    orchardId: 7,
  });
});

test("share URLs keep the revocable token in the browser fragment", () => {
  assert.equal(
    sharedOrchardUrl("https://orchard.example", 7, "secret-token"),
    "https://orchard.example/#/orchards/7/share/secret-token",
  );
  assert.equal(
    sharedOrchardUrl("https://orchard.example", 7, "watering-token", "watering"),
    "https://orchard.example/#/orchards/7/share/watering/watering-token",
  );
  assert.equal(
    sharedOrchardUrl(
      "https://orchard.example",
      7,
      "worker-token",
      "harvest-watering",
    ),
    "https://orchard.example/#/orchards/7/share/harvest-watering/worker-token",
  );
});

test("the owner can create the distinct harvest and watering link from the access panel", () => {
  assert.match(indexHtml, /id="share-harvest-watering-orchard"/);
  assert.match(indexHtml, /createShareLink\("harvest-watering"\)/);
  assert.match(indexHtml, /permission === "harvest-watering"[\s\S]*?"share\/harvest-watering"/);
});
