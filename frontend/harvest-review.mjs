import { parseAnnualDate } from "./harvest-window.mjs?v=8";

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

export function harvestReviewRequest(ids, windowChange = null, window = null, validateWindow = false) {
  const scopedEdit = window && windowChange?.windows?.length === 1;
  if (!Array.isArray(ids) || (!ids.length && !scopedEdit) || new Set(ids).size !== ids.length ||
      ids.some((id) => !Number.isSafeInteger(id) || id <= 0)) {
    throw new TypeError("At least one unreviewed observation is required.");
  }
  if (validateWindow && !scopedEdit) throw new TypeError("Validation requires editing one specific window.");
  const request = { observation_ids: [...ids], window_change: windowChange };
  if (validateWindow) request.validate_window = true;
  if (window) {
    request.window = {
      start: parseAnnualDate(window.start), end: parseAnnualDate(window.end),
      harvested_part: window.harvested_part || "fruit",
    };
    if (!request.window.start || !request.window.end) throw new TypeError("A valid harvest window is required.");
  }
  return request;
}

export function harvestWindowValidationPath(schedule) {
  return harvestObservationPath(schedule).replace("harvest-observations", "harvest-window-validation");
}
export function harvestWindowValidationRequest(window) {
  const start = parseAnnualDate(window.start), end = parseAnnualDate(window.end);
  if (!start || !end) throw new TypeError("A valid harvest window is required.");
  return { window: { start, end, harvested_part: window.harvested_part || "fruit" } };
}
export function harvestWindowValidationQuery(window) {
  const { start, end, harvested_part } = harvestWindowValidationRequest(window).window;
  return new URLSearchParams({ start_month: start.month, start_day: start.day, end_month: end.month, end_day: end.day, harvested_part }).toString();
}
export function harvestValidationGuidance({ evidence }) {
  const coverage = evidence.start_supported && evidence.end_supported
    ? "Evidence supports both the start and the end."
    : evidence.start_supported ? "The start is supported; more evidence is needed around the end."
    : evidence.end_supported ? "The end is supported; more evidence is needed around the start."
    : "Not enough data to conclude: observations do not yet bracket the start or the end.";
  const conflict = evidence.conflicting_observations
    ? `${evidence.conflicting_observations} observation${evidence.conflicting_observations === 1 ? " suggests" : "s suggest"} the dates need review. ` : "";
  return { text: `${evidence.observation_count} observation${evidence.observation_count === 1 ? "" : "s"}. ${conflict}${coverage} You decide whether to validate.`, tone: conflict || !evidence.start_supported || !evidence.end_supported ? "warning" : "supported" };
}
