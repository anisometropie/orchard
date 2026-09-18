import { parseAnnualDate } from "./harvest-window.mjs";
import {
  dangerWateringNumberGeoJson,
  wateringRouteWindow,
  wateringTargetGeoJson,
} from "./watering.mjs";

const HARVEST_ROUTE_WINDOW_SIZES = Object.freeze({
  next: 1,
  four: 4,
});
const HARVESTED_PARTS = new Set([
  "cone",
  "flower",
  "fruit",
  "leaf",
  "nut",
  "pod",
  "seed",
]);

export function localIsoDate(date = new Date()) {
  if (!(date instanceof Date) || Number.isNaN(date.getTime())) {
    throw new TypeError("A valid date is required.");
  }
  const year = String(date.getFullYear()).padStart(4, "0");
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function harvestStartRequest(
  scope,
  plantIdentityId,
  onDate,
  harvestedParts = ["fruit"],
) {
  const on_date = normalizeIsoDate(onDate);
  const harvested_parts = normalizeHarvestedParts(harvestedParts);
  if (scope === "all") {
    return { target: "all", plant_identity_id: null, harvested_parts, on_date };
  }
  if (scope !== "species") throw new Error(`Unknown harvest scope: ${scope}`);

  const normalizedPlantIdentityId = Number(plantIdentityId);
  if (
    !Number.isSafeInteger(normalizedPlantIdentityId) ||
    normalizedPlantIdentityId <= 0
  ) {
    throw new TypeError("A valid plant identity is required.");
  }
  return {
    target: "species",
    plant_identity_id: normalizedPlantIdentityId,
    harvested_parts,
    on_date,
  };
}

export function harvestConflictMessage(code) {
  if (code === "harvest_action_date_outside_run_period") {
    return "Today's date is outside the harvest period captured by this tour. Cancel this tour and start a new one.";
  }
  if (code === "harvest_window_changed") {
    return "This tree's harvest window changed after the tour started. Cancel this tour and start a new one.";
  }
  return "Harvest progress changed. Reload the page to resume.";
}

export function availableHarvestSpecies(
  features,
  onDate,
  eligibleTreeIds = null,
  harvestedParts = ["fruit"],
) {
  const date = isoDateParts(normalizeIsoDate(onDate));
  const eligibleTrees = normalizeTreeIds(eligibleTreeIds);
  const selectedParts = new Set(normalizeHarvestedParts(harvestedParts, true));
  const species = new Map();

  for (const feature of Array.isArray(features) ? features : []) {
    if (eligibleTrees && !eligibleTrees.has(String(feature.id))) continue;
    const properties = feature.properties || {};
    if (properties.is_alive === false) continue;
    if (
      !harvestWindows(properties.harvest_windows).some((window) =>
        isHarvestWindowOnDate(window, date, selectedParts),
      )
    ) continue;

    const plantIdentityId = Number(properties.plant_identity_id);
    if (!Number.isSafeInteger(plantIdentityId) || plantIdentityId <= 0) continue;
    const current = species.get(plantIdentityId);
    if (current) {
      current.treeCount += 1;
      continue;
    }
    species.set(plantIdentityId, {
      plantIdentityId,
      label: speciesLabel(properties, plantIdentityId),
      treeCount: 1,
    });
  }

  return [...species.values()].sort((left, right) =>
    left.label.localeCompare(right.label, undefined, { numeric: true }),
  );
}

function normalizeTreeIds(treeIds) {
  if (treeIds == null) return null;
  if (!Array.isArray(treeIds) && !(treeIds instanceof Set)) {
    throw new TypeError("Eligible tree IDs must be an array or Set.");
  }
  return new Set([...treeIds].map(String));
}

export function harvestRouteWindow(route, currentTreeId, mode = "next") {
  const maximumTrees = HARVEST_ROUTE_WINDOW_SIZES[mode];
  if (maximumTrees == null) throw new Error(`Unknown harvest route mode: ${mode}`);
  return wateringRouteWindow(route, currentTreeId, maximumTrees);
}

export function harvestRouteNumberGeoJson(
  route,
  currentTreeId = null,
  startIndex = 0,
) {
  return dangerWateringNumberGeoJson(route, currentTreeId, startIndex, 1);
}

export function harvestTargetGeoJson(tree) {
  return wateringTargetGeoJson(tree);
}

export function harvestRoutePathGeoJson(route) {
  const coordinates = (Array.isArray(route) ? route : [])
    .map((tree) => [Number(tree?.longitude), Number(tree?.latitude)])
    .filter((coordinate) => coordinate.every(Number.isFinite));
  return {
    type: "FeatureCollection",
    features: coordinates.length < 2
      ? []
      : [
          {
            type: "Feature",
            properties: {},
            geometry: { type: "LineString", coordinates },
          },
        ],
  };
}

function isHarvestWindowOnDate(window, date, selectedParts) {
  if (!window || !selectedParts.has(window.harvested_part || "fruit")) {
    return false;
  }
  const start = parseAnnualDate(window.start || "");
  const end = parseAnnualDate(window.end || "");
  if (!start || !end) return false;

  const selectedDate = Date.UTC(date.year, date.month - 1, date.day);
  for (const startYear of [date.year - 1, date.year]) {
    const recurringStart = annualDateInYear(start, startYear);
    const crossesNewYear = compareAnnualDates(end, start) < 0;
    const recurringEnd = annualDateInYear(
      end,
      crossesNewYear ? startYear + 1 : startYear,
    );
    if (
      recurringStart != null &&
      recurringEnd != null &&
      recurringStart <= selectedDate &&
      selectedDate <= recurringEnd
    ) return true;
  }
  return false;
}

function normalizeHarvestedParts(value, allowEmpty = false) {
  if (!Array.isArray(value)) {
    throw new TypeError("Harvested parts must be an array.");
  }
  const parts = [...new Set(value)];
  if (parts.some((part) => !HARVESTED_PARTS.has(part))) {
    throw new TypeError("A harvested part is invalid.");
  }
  if (!allowEmpty && parts.length === 0) {
    throw new TypeError("Select at least one harvested part.");
  }
  return parts;
}

function annualDateInYear(annualDate, year) {
  const lastDay = new Date(Date.UTC(year, annualDate.month, 0)).getUTCDate();
  return annualDate.day <= lastDay
    ? Date.UTC(year, annualDate.month - 1, annualDate.day)
    : null;
}

function compareAnnualDates(left, right) {
  return left.month - right.month || left.day - right.day;
}

function harvestWindows(value) {
  if (Array.isArray(value)) return value;
  if (typeof value !== "string") return [];
  try {
    const parsed = JSON.parse(value);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function speciesLabel(properties, plantIdentityId) {
  const commonName = String(properties.plant_identity_name || "").trim();
  const taxonName = String(properties.plant_identity_taxon_name || "").trim();
  if (commonName && taxonName && commonName !== taxonName) {
    return `${commonName} · ${taxonName}`;
  }
  return commonName || taxonName || `Species ${plantIdentityId}`;
}

function normalizeIsoDate(value) {
  if (value instanceof Date) return localIsoDate(value);
  if (typeof value !== "string") throw new TypeError("A valid date is required.");
  const date = isoDateParts(value);
  const lastDay = new Date(Date.UTC(date.year, date.month, 0)).getUTCDate();
  if (date.day > lastDay) throw new TypeError("A valid date is required.");
  return value;
}

function isoDateParts(value) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) throw new TypeError("A valid date is required.");
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  if (month < 1 || month > 12 || day < 1) {
    throw new TypeError("A valid date is required.");
  }
  return { year, month, day };
}
