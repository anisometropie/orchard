import { normalizeHarvestWindows, parseAnnualDate } from "./harvest-window.mjs?v=6";
import { remainingFruitVisit } from "./harvest-visit.mjs?v=3";

const PARTS = ["fruit", "flower", "leaf", "seed", "cone", "nut", "pod"];

export function observationWindows(properties, part) {
  return normalizeHarvestWindows(properties.harvest_windows)
    .filter((window) => (window.harvested_part || "fruit") === part &&
      parseAnnualDate(window.start || "") && parseAnnualDate(window.end || ""));
}

export function observationPeriod(window, startYear) {
  const year = Number(startYear);
  if (!Number.isInteger(year) || year < 1 || year > 9999) throw new TypeError("Choose a valid season start year.");
  const start = parseAnnualDate(window.start || ""), end = parseAnnualDate(window.end || "");
  if (!start || !end) throw new TypeError("Choose a valid harvest period.");
  const endYear = window.end < window.start ? year + 1 : year;
  const isoDate = (year, date) => {
    const value = `${String(year).padStart(4, "0")}-${String(date.month).padStart(2, "0")}-${String(date.day).padStart(2, "0")}`;
    if (year > 9999 || new Date(`${value}T00:00:00Z`).toISOString().slice(0, 10) !== value) {
      throw new TypeError("This period does not exist in the selected year.");
    }
    return value;
  };
  return { start: isoDate(year, start), end: isoDate(endYear, end) };
}

export function nearestObservationSeason(window, recordedOn) {
  const year = Number(recordedOn.slice(0, 4));
  const day = Date.parse(`${recordedOn}T00:00:00Z`);
  const candidates = [year - 1, year, year + 1].flatMap((startYear) => {
    try {
      const period = observationPeriod(window, startYear);
      const start = Date.parse(period.start), end = Date.parse(period.end);
      return [{ startYear, distance: Math.max(start - day, day - end, 0) }];
    } catch { return []; }
  });
  return candidates.sort((a, b) => a.distance - b.distance || Math.abs(a.startYear - year) - Math.abs(b.startYear - year))[0]?.startYear ?? year;
}

export function standaloneObservationRequest({ recordedOn, part, kind, fruitState, window = null, startYear }) {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(recordedOn) || Number(recordedOn.slice(0, 4)) < 1 || !Number.isFinite(Date.parse(`${recordedOn}T00:00:00Z`)) ||
      new Date(`${recordedOn}T00:00:00Z`).toISOString().slice(0, 10) !== recordedOn) {
    throw new TypeError("Choose a valid observation date.");
  }
  if (!PARTS.includes(part)) throw new TypeError("Choose the harvested part.");
  if (!["fruit_remaining", "harvested_all", "empty_on_arrival", "all_wasted"].includes(kind)) throw new TypeError("Choose what you observed.");
  const visit = kind === "fruit_remaining" ? remainingFruitVisit(fruitState || {}) : { kind };
  if (window) observationPeriod(window, startYear);
  return {
    action_date: recordedOn,
    harvested_part: part,
    ...visit,
    window: window ? { start: parseAnnualDate(window.start), end: parseAnnualDate(window.end), start_year: Number(startYear) } : null,
  };
}

export function unreviewedObservationCues(points, schedule, parts = null) {
  const kind = schedule.cultivarId != null ? "cultivar" : "species";
  const id = Number(schedule.cultivarId ?? schedule.plantIdentityId);
  return points.filter((point) => !point.reviewed && point.window_start == null && point.window_end == null &&
    point.owner_kind === kind && point.owner_id === id &&
    (!parts || point.harvested_parts?.some((part) => parts.includes(part))))
    .sort((a, b) => a.recorded_on.localeCompare(b.recorded_on) || a.id - b.id);
}
