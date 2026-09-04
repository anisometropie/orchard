import assert from "node:assert/strict";
import test from "node:test";

import {
  accessHeaders,
  hasOpenOrchard,
  orchardResourceUrl,
  resolveOrchardAccess,
  sharedOrchardUrl,
} from "./orchard-access.mjs";

test("show orchard controls only while an owned or shared orchard is open", () => {
  assert.equal(hasOpenOrchard({ mode: "empty" }), false);
  assert.equal(hasOpenOrchard({ mode: "login-required" }), false);
  assert.equal(hasOpenOrchard({ mode: "editable" }), true);
  assert.equal(hasOpenOrchard({ mode: "read-only" }), true);
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
});
