import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  accessHeaders,
  canAddPhotosToOrchard,
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
  assert.equal(hasOpenOrchard({ mode: "shared" }), true);
});

test("allow only an owner or watering-link visitor to water the open orchard", () => {
  assert.equal(canWaterOrchard({ mode: "empty" }), false);
  assert.equal(canWaterOrchard({ mode: "login-required" }), false);
  assert.equal(canWaterOrchard({ mode: "editable" }), true);
  assert.equal(canWaterOrchard({ mode: "shared", permissions: {} }), false);
  assert.equal(
    canWaterOrchard({ mode: "shared", permissions: { water: true } }),
    true,
  );
});

test("allow only an owner or combined-link visitor to harvest the open orchard", () => {
  assert.equal(canHarvestOrchard({ mode: "empty" }), false);
  assert.equal(canHarvestOrchard({ mode: "login-required" }), false);
  assert.equal(canHarvestOrchard({ mode: "editable" }), true);
  assert.equal(canHarvestOrchard({ mode: "shared", permissions: {} }), false);
  assert.equal(
    canHarvestOrchard({ mode: "shared", permissions: { harvest: true } }),
    true,
  );
});

test("allow only an owner or photo-enabled link visitor to add photos", () => {
  assert.equal(canAddPhotosToOrchard({ mode: "empty" }), false);
  assert.equal(canAddPhotosToOrchard({ mode: "editable" }), true);
  assert.equal(
    canAddPhotosToOrchard({ mode: "shared", permissions: {} }),
    false,
  );
  assert.equal(
    canAddPhotosToOrchard({
      mode: "shared",
      permissions: { add_photos: true },
    }),
    true,
  );
});

test("a watering fragment opens one orchard with watering access", () => {
  const access = resolveOrchardAccess(
    "#/orchards/42/share/watering/O8xz_watering-token",
    null,
  );

  assert.deepEqual(access, {
    mode: "shared",
    orchardId: 42,
    shareToken: "O8xz_watering-token",
    permissions: {},
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
    mode: "shared",
    orchardId: 42,
    shareToken: "O8xz-worker-token",
    permissions: {},
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
    mode: "shared",
    orchardId: 42,
    shareToken: "O8xz_private-token",
    permissions: {},
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
    "https://orchard.example/#/orchards/7/share/watering-token",
  );
  assert.equal(
    sharedOrchardUrl(
      "https://orchard.example",
      7,
      "worker-token",
      "harvest-watering",
    ),
    "https://orchard.example/#/orchards/7/share/worker-token",
  );
});

test("the owner can choose capabilities and manage issued links", () => {
  assert.match(indexHtml, /id="new-share-harvest"/);
  assert.match(indexHtml, /id="new-share-water"/);
  assert.match(indexHtml, /id="new-share-add-photos"/);
  assert.match(indexHtml, /id="issued-share-tokens"/);
  assert.match(indexHtml, /share-tokens/);
  assert.match(indexHtml, /newlyCreatedShareLinks\.get\(String\(share\.id\)\)/);
  assert.match(indexHtml, /if \(link\) row\.append\(linkBox\)/);
  assert.doesNotMatch(indexHtml, /id="share-result"/);
});
