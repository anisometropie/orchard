import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";
import { harvestPartsLabel } from "./run-history.mjs";
import { adaptiveWateringWideZoom, wateringWideCenter } from "./watering.mjs";

const indexHtml = readFileSync(new URL("./index.html", import.meta.url), "utf8");
function functionsBetween(start, end) {
  return indexHtml.slice(indexHtml.indexOf(start), indexHtml.indexOf(end, indexHtml.indexOf(start)));
}

function cameraFixture({ mobile = true } = {}) {
  const frames = new Map();
  let frameId = 0;
  const element = (bottom = 140, height = 64) => ({
    hidden: true,
    dataset: {},
    attributes: {},
    listeners: {},
    getBoundingClientRect: () => ({ bottom, height }),
    setAttribute(name, value) { this.attributes[name] = value; },
    addEventListener(name, callback) { this.listeners[name] = callback; },
    click() { this.listeners.click(); },
  });
  const cameras = [];
  let orchardFits = 0;
  const context = {
    harvestPartsLabel,
    adaptiveWateringWideZoom,
    wateringWideCenter,
    harvestProgress: null,
    harvestRoute: [],
    harvestUndoRunId: null,
    harvestCameraMode: "wide",
    harvestWideZoomLevel: null,
    harvestCameraFrame: null,
    compactMobileLayout: { matches: mobile },
    window: { innerWidth: 375, innerHeight: 800 },
    orchardFeatures: [
      { geometry: { coordinates: [2, 48] } },
      { geometry: { coordinates: [2.002, 48.002] } },
    ],
    maplibregl: { LngLatBounds: class { extend() { return this; } } },
    map: {
      getZoom: () => 21,
      getMinZoom: () => 0,
      getMaxZoom: () => 24,
      cameraForBounds() { orchardFits++; return { zoom: 17 }; },
      getCanvas: () => ({ getBoundingClientRect: () => ({ width: 375, height: 800 }) }),
      easeTo: (camera) => cameras.push(camera),
      fitBounds: (_, camera) => cameras.push({ ...camera, fitted: true }),
      stop() {},
    },
    requestAnimationFrame(callback) { frames.set(++frameId, callback); return frameId; },
    cancelAnimationFrame(id) { frames.delete(id); },
    harvestPartFilters: { querySelectorAll: () => [] },
    harvestMapPartFilters: {},
    tourRouteDisplay: { value: "four" },
    currentHarvestRouteWindow: () => ({ trees: [] }),
  };
  for (const name of ["harvestTreeBanner", "harvestZoomControls", "harvestZoomClose", "harvestZoomWide", "harvestTreeName", "harvestTreeProgress", "harvestTreeError", "harvestPreviousTree", "harvestCompleteTree", "harvestDeferTree", "harvestTargetScope", "harvestSpecies", "harvestJobStatus", "harvestJobError"]) {
    context[name] = element();
  }
  for (const name of ["clearHarvestExtensionDecision", "clearHarvestVisitDraft", "clearHarvestRouteSources", "refreshHarvestJobControls", "renderMapMode", "layoutMapControls", "synchronizeHarvestPartFilters", "closeTreeEditor", "closeTreeTooltip", "closeMobileMenus", "setMapMode"]) {
    context[name] = () => {};
  }
  vm.createContext(context);
  vm.runInContext([
    functionsBetween("function wateringCameraPadding", "function applyWateringCameraMode"),
    functionsBetween("function renderHarvestCameraMode", "function clearHarvestVisitDraft"),
    functionsBetween("function resetCompletedHarvestPresentation", "async function restoreActiveHarvestRun"),
    "function updateHarvestRoutePresentation({cameraDuration}) { applyHarvestCameraMode(harvestCameraMode, {duration: cameraDuration, afterLayout: true}); }",
    functionsBetween('harvestZoomClose.addEventListener("click"', 'rowOrderToggle.addEventListener("click"'),
  ].join("\n"), context);
  return {
    context,
    cameras,
    get orchardFits() { return orchardFits; },
    get pendingFrames() { return frames.size; },
    flush() {
      while (frames.size) {
        const callbacks = [...frames.values()];
        frames.clear();
        callbacks.forEach((callback) => callback());
      }
    },
    show({ runId = 1, treeId = 1 } = {}) {
      context.progress = {
        run_id: runId,
        next_tree: { id: treeId, name: `Tree ${treeId}`, longitude: 2 + treeId / 1000, latitude: 48, harvested_parts: ["fruit"] },
        total_tree_count: 3,
      };
      vm.runInContext("showHarvestProgress(progress)", context);
      this.flush();
    },
  };
}

test("harvest starts wide, + selects the same 10-metre close view, and selection survives the next tree", () => {
  const fixture = cameraFixture();
  fixture.show();
  const { context, cameras } = fixture;
  assert.equal(context.harvestZoomControls.hidden, false);
  assert.equal(context.harvestZoomWide.attributes["aria-pressed"], "true");
  assert.equal(context.harvestZoomClose.attributes["aria-pressed"], "false");
  assert.equal(cameras.at(-1).zoom, 20);

  context.harvestZoomClose.click();
  const expectedCloseZoom = Math.log2((156_543.03392 * Math.cos(48 * Math.PI / 180) * 335) / 10);
  assert.equal(cameras.at(-1).zoom, expectedCloseZoom);
  assert.equal(context.harvestZoomClose.attributes["aria-pressed"], "true");
  fixture.show({ treeId: 2 });
  assert.equal(context.harvestCameraMode, "close");
  assert.equal(cameras.at(-1).zoom, expectedCloseZoom);
  assert.deepEqual([...cameras.at(-1).center], [2.002, 48]);

  context.harvestZoomWide.click();
  assert.equal(cameras.at(-1).zoom, 20);
  assert.equal(fixture.orchardFits, 1);
  assert.equal(context.harvestZoomWide.attributes["aria-pressed"], "true");
  assert.equal(cameras.at(-1).padding.bottom, 100);
  assert.equal(cameras.at(-1).retainPadding, false);
});

test("completion and cancellation hide harvest zoom controls and cancel queued camera movement", () => {
  for (const action of ["showHarvestProgress({ next_tree: null })", "resetCompletedHarvestPresentation()"] ) {
    const fixture = cameraFixture();
    fixture.show();
    fixture.context.harvestZoomClose.click();
    vm.runInContext('applyHarvestCameraMode("close", {afterLayout: true})', fixture.context);
    assert.equal(fixture.pendingFrames, 1);
    const cameraCount = fixture.cameras.length;
    vm.runInContext(action, fixture.context);
    fixture.flush();
    assert.equal(fixture.context.harvestZoomControls.hidden, true);
    assert.equal(fixture.context.harvestCameraMode, "wide");
    assert.equal(fixture.context.harvestWideZoomLevel, null);
    assert.equal(fixture.cameras.length, cameraCount);
  }
});

test("a different harvest tour resets to wide and recomputes the orchard scale", () => {
  const fixture = cameraFixture();
  fixture.show();
  fixture.context.harvestZoomClose.click();
  fixture.show({ runId: 2 });
  assert.equal(fixture.context.harvestCameraMode, "wide");
  assert.equal(fixture.context.harvestZoomWide.attributes["aria-pressed"], "true");
  assert.equal(fixture.orchardFits, 2);
});

test("harvest desktop route fitting remains unchanged", () => {
  const fixture = cameraFixture({ mobile: false });
  fixture.context.currentHarvestRouteWindow = () => ({ trees: [
    { longitude: 2, latitude: 48 },
    { longitude: 2.002, latitude: 48.002 },
  ] });
  fixture.show();
  assert.equal(fixture.cameras.at(-1).fitted, true);
  assert.equal(fixture.cameras.at(-1).maxZoom, 20);
  assert.equal(fixture.orchardFits, 0);
});

test("watering and harvest use the same close zoom and base wide scale", () => {
  const fixture = cameraFixture();
  fixture.show();
  const { context, cameras } = fixture;
  context.wateringProgress = context.harvestProgress;
  context.wateringTreeBanner = context.harvestTreeBanner;
  context.wateringZoomControls = context.harvestZoomControls;
  context.wateringSource = null;
  context.wateringCameraMode = "wide";
  context.wateringWideZoomLevel = null;
  const harvestWide = cameras.at(-1);
  vm.runInContext("focusCurrentWateringTree()", context);
  assert.equal(cameras.at(-1).zoom, harvestWide.zoom);
  assert.deepEqual(cameras.at(-1).padding, harvestWide.padding);
  assert.deepEqual([...cameras.at(-1).center], [...harvestWide.center]);

  context.harvestZoomClose.click();
  const harvestClose = cameras.at(-1);
  context.wateringCameraMode = "close";
  vm.runInContext("focusCurrentWateringTree()", context);
  assert.equal(cameras.at(-1).zoom, harvestClose.zoom);
  assert.deepEqual(cameras.at(-1).padding, harvestClose.padding);
});

test("distant orchard trees cannot turn harvest's minus view into a map of the world", () => {
  const fixture = cameraFixture();
  fixture.context.orchardFeatures.push({ geometry: { coordinates: [-120, -40] } });
  fixture.context.map.cameraForBounds = () => ({ zoom: 2 });
  fixture.show();
  assert.equal(fixture.cameras.at(-1).zoom, 20);
  assert.deepEqual([...fixture.cameras.at(-1).center], [2.001, 48]);
  fixture.context.harvestZoomClose.click();
  fixture.context.harvestZoomWide.click();
  assert.equal(fixture.cameras.at(-1).zoom, 20);
});

test("watering still zooms out below the local base when the source is far from the tree", () => {
  const fixture = cameraFixture();
  fixture.show();
  const { context, cameras } = fixture;
  context.wateringProgress = context.harvestProgress;
  context.wateringTreeBanner = context.harvestTreeBanner;
  context.wateringZoomControls = context.harvestZoomControls;
  context.wateringSource = { longitude: 2.1, latitude: 48 };
  context.wateringCameraMode = "wide";
  context.wateringWideZoomLevel = null;
  vm.runInContext("focusCurrentWateringTree()", context);
  assert.ok(cameras.at(-1).zoom < 20);
});
