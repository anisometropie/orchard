import { parseAnnualDate } from "./harvest-window.mjs?v=5";

export function harvestObservationPath(schedule) {
  const cultivar = schedule.cultivarId != null;
  const id = Number(cultivar ? schedule.cultivarId : schedule.plantIdentityId);
  if (!Number.isSafeInteger(id) || id <= 0) throw new TypeError("A species or cultivar is required.");
  return `harvest-observations/${cultivar ? "cultivar" : "species"}/${id}`;
}

export function observationsForHarvestWindow(points, schedule, window) {
  const ownerKind = schedule.cultivarId != null ? "cultivar" : "species";
  const ownerId = Number(schedule.cultivarId ?? schedule.plantIdentityId);
  return points.filter((point) => point.kind !== "empty_on_arrival" &&
    point.owner_kind === ownerKind && point.owner_id === ownerId &&
    point.window_start?.slice(5) === window.start && point.window_end?.slice(5) === window.end &&
    point.harvested_parts?.includes(window.harvested_part || "fruit"));
}

export function pendingObservationIds(points) {
  return points.filter((point) => !point.reviewed).map((point) => point.id);
}

export function harvestReviewRequest(ids, windowChange = null, window = null) {
  if (!Array.isArray(ids) || !ids.length || new Set(ids).size !== ids.length ||
      ids.some((id) => !Number.isSafeInteger(id) || id <= 0)) {
    throw new TypeError("At least one unreviewed observation is required.");
  }
  const request = { observation_ids: [...ids], window_change: windowChange };
  if (window) {
    request.window = {
      start: parseAnnualDate(window.start), end: parseAnnualDate(window.end),
      harvested_part: window.harvested_part || "fruit",
    };
    if (!request.window.start || !request.window.end) throw new TypeError("A valid harvest window is required.");
  }
  return request;
}
