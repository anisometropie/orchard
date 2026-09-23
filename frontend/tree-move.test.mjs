import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  idleTreeMoveState,
  treeMoveInteractionPolicy,
  treeMovePresentation,
  treeMoveRequest,
  treeMoveTransition,
} from "./tree-move.mjs";

const indexHtml = readFileSync(new URL("./index.html", import.meta.url), "utf8");

test("mobile tree moving responds immediately on a large non-scrolling touch target", () => {
  assert.match(
    indexHtml,
    /html,\s*body\s*{[^}]*overflow:\s*hidden;[^}]*overscroll-behavior:\s*none;/s,
  );
  assert.match(
    indexHtml,
    /\.maplibregl-marker\.tree-move-marker\s*{[^}]*touch-action:\s*none;/s,
  );
  assert.match(
    indexHtml,
    /@media \(pointer: coarse\)[\s\S]*?\.maplibregl-marker\.tree-move-marker\s*{[^}]*height:\s*80px;[^}]*width:\s*64px;/,
  );
  assert.match(
    indexHtml,
    /new maplibregl\.Marker\(\{\s*clickTolerance:\s*Number\.EPSILON,\s*draggable:\s*true,\s*subpixelPositioning:\s*true,\s*\}\)/,
  );
});

test("the move lock leaves the pin reachable without allowing background interaction", () => {
  const state = treeMoveTransition(idleTreeMoveState(), {
    type: "USER_CLICKED_MOVE_TREE",
    treeId: 42,
    treeName: "Test tree",
    position: [12.10, -34.10],
  });

  assert.deepEqual(treeMoveInteractionPolicy(state), {
    backgroundControlsInert: true,
    mapGesturesEnabled: false,
    scrimInterceptsPointer: false,
  });
  assert.deepEqual(treeMoveInteractionPolicy(idleTreeMoveState()), {
    backgroundControlsInert: false,
    mapGesturesEnabled: true,
    scrimInterceptsPointer: false,
  });
});

test("clicking Move locks the interface around one draggable tree pin", () => {
  const state = treeMoveTransition(idleTreeMoveState(), {
    type: "USER_CLICKED_MOVE_TREE",
    treeId: 42,
    treeName: "Test tree",
    position: [12.10, -34.10],
  });

  assert.deepEqual(state, {
    phase: "positioning",
    treeId: 42,
    treeName: "Test tree",
    originalPosition: [12.10, -34.10],
    proposedPosition: [12.10, -34.10],
    error: null,
  });
  assert.deepEqual(treeMovePresentation(state), {
    active: true,
    interactionLocked: true,
    pin: {
      position: [12.10, -34.10],
      draggable: true,
    },
    confirmEnabled: true,
    cancelEnabled: true,
    saving: false,
    error: null,
  });
});

test("dragging proposes a position that OK submits while the interface stays locked", () => {
  const started = treeMoveTransition(idleTreeMoveState(), {
    type: "USER_CLICKED_MOVE_TREE",
    treeId: 42,
    treeName: "Test tree",
    position: [12.10, -34.10],
  });
  const dragged = treeMoveTransition(started, {
    type: "USER_DRAGGED_TREE_MOVE_PIN",
    position: [12.25, -34.24],
  });
  const confirmed = treeMoveTransition(dragged, {
    type: "USER_CONFIRMED_TREE_MOVE",
  });

  assert.deepEqual(treeMoveRequest(confirmed), {
    treeId: 42,
    longitude: 12.25,
    latitude: -34.24,
  });
  assert.deepEqual(treeMovePresentation(confirmed), {
    active: true,
    interactionLocked: true,
    pin: {
      position: [12.25, -34.24],
      draggable: false,
    },
    confirmEnabled: false,
    cancelEnabled: false,
    saving: true,
    error: null,
  });
});

test("Cancel abandons the proposed position and unlocks every other interaction", () => {
  const started = treeMoveTransition(idleTreeMoveState(), {
    type: "USER_CLICKED_MOVE_TREE",
    treeId: 42,
    treeName: "Test tree",
    position: [12.10, -34.10],
  });
  const dragged = treeMoveTransition(started, {
    type: "USER_DRAGGED_TREE_MOVE_PIN",
    position: [12.25, -34.24],
  });

  const cancelled = treeMoveTransition(dragged, {
    type: "USER_CANCELLED_TREE_MOVE",
  });

  assert.deepEqual(cancelled, idleTreeMoveState());
  assert.deepEqual(treeMovePresentation(cancelled), {
    active: false,
    interactionLocked: false,
    pin: null,
    confirmEnabled: false,
    cancelEnabled: false,
    saving: false,
    error: null,
  });
  assert.equal(treeMoveRequest(cancelled), null);
});

test("a failed save keeps the proposed pin locked in place and enables retry or Cancel", () => {
  const started = treeMoveTransition(idleTreeMoveState(), {
    type: "USER_CLICKED_MOVE_TREE",
    treeId: 42,
    treeName: "Test tree",
    position: [12.10, -34.10],
  });
  const dragged = treeMoveTransition(started, {
    type: "USER_DRAGGED_TREE_MOVE_PIN",
    position: [12.25, -34.24],
  });
  const saving = treeMoveTransition(dragged, {
    type: "USER_CONFIRMED_TREE_MOVE",
  });

  const failed = treeMoveTransition(saving, {
    type: "TREE_MOVE_FAILED",
    message: "Could not move this tree. Please try again.",
  });

  assert.deepEqual(treeMovePresentation(failed), {
    active: true,
    interactionLocked: true,
    pin: {
      position: [12.25, -34.24],
      draggable: true,
    },
    confirmEnabled: true,
    cancelEnabled: true,
    saving: false,
    error: "Could not move this tree. Please try again.",
  });
});

test("a successful save removes the pin and unlocks the interface", () => {
  const started = treeMoveTransition(idleTreeMoveState(), {
    type: "USER_CLICKED_MOVE_TREE",
    treeId: 42,
    treeName: "Test tree",
    position: [12.10, -34.10],
  });
  const saving = treeMoveTransition(started, {
    type: "USER_CONFIRMED_TREE_MOVE",
  });

  const saved = treeMoveTransition(saving, {
    type: "TREE_MOVE_SUCCEEDED",
  });

  assert.deepEqual(saved, idleTreeMoveState());
  assert.equal(treeMovePresentation(saved).interactionLocked, false);
});

test("another tree cannot enter the interaction while a move is active", () => {
  const firstTree = treeMoveTransition(idleTreeMoveState(), {
    type: "USER_CLICKED_MOVE_TREE",
    treeId: 42,
    treeName: "First test tree",
    position: [12.10, -34.10],
  });

  const secondTreeClick = treeMoveTransition(firstTree, {
    type: "USER_CLICKED_MOVE_TREE",
    treeId: 84,
    treeName: "Second test tree",
    position: [12.20, -34.20],
  });

  assert.deepEqual(secondTreeClick, firstTree);
  assert.equal(treeMovePresentation(secondTreeClick).interactionLocked, true);
});
