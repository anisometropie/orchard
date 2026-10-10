export function remainingFruitVisit(fruit) {
  const fruit_state = { ready: !!fruit.ready, unripe: !!fruit.unripe, wasted: !!fruit.wasted };
  if (!Object.values(fruit_state).some(Boolean)) throw new TypeError("Select what you observed.");
  return fruit_state.ready || fruit_state.unripe
    ? { kind: "fruit_remaining", fruit_state }
    : { kind: "all_wasted" };
}

export function hasWindowObservations(points, schedule) {
  const kind = schedule.cultivarId != null ? "cultivar" : "species";
  const id = Number(schedule.cultivarId ?? schedule.plantIdentityId);
  return points.some((point) => point.kind !== "empty_on_arrival" && point.owner_kind === kind && point.owner_id === id);
}

export function speciesNeedsHarvestReview(points, species) {
  return points.some((point) => !point.reviewed && point.kind !== "empty_on_arrival" && (
    (point.owner_kind === "species" && point.owner_id === species.plantIdentityId) ||
    (point.owner_kind === "cultivar" && species.cultivars.some((cultivar) => cultivar.id === point.owner_id))
  ));
}

export function harvestVisitLabel(point) {
  switch (point.kind) {
    case "harvested_all": return "Harvested all";
    case "empty_on_arrival": return "Empty on arrival · no production recorded for this window";
    case "all_wasted": return "All wasted · nothing left to collect";
    case "too_soon": return "Not ready yet";
    case "too_late": return "All wasted";
    default: return [point.fruit_state?.ready && "Ready now", point.fruit_state?.unripe && "Not ready yet", point.fruit_state?.wasted && "Some fruit wasted"].filter(Boolean).join(" · ");
  }
}

const EVIDENCE_LABELS = {
  start_too_early: "No ripe fruit on this visit; the start may need to move later",
  start_too_late: "Fruit was already ready or wasted by the start; an earlier start may be needed",
  start_may_be_too_late: "Some fruit was already missed at the start; check whether to start earlier",
  end_too_early: "Fruit is ready after the recorded end; consider a later end",
  harvest_over_within_window: "All fruit was wasted during the window. Near its start, check for earlier dates; near its end, this can be expected",
  ready_within_window: "Ready on a date inside the window",
  end_consistent: "All wasted at the end is consistent with the window ending",
  end_confirmed_by_earlier_visit: "Earlier ripe fruit and waste at the end support the window",
  harvest_over_by_visit: "Harvest was over by this visit after the window",
  later_fruit_possible: "Unripe fruit at the end may need a later visit",
  mixed_ripeness: "Different fruit stages; review them together before changing dates",
  earlier_fruit_missed: "Some earlier fruit was missed while later fruit remains",
  gap_after_earlier_harvest: "Earlier fruit was ready; this may be a gap before later fruit",
  not_ready_yet: "Unripe before the window; this alone does not show the start is wrong",
};
export function harvestEvidenceLabel(point) {
  return (point.evidence || []).map((value) => EVIDENCE_LABELS[value] || value).join(". ");
}

export function firstRecordedFruit(points, plantedOn) {
  const first = points.filter((point) => point.kind !== "empty_on_arrival").map((point) => point.recorded_on).sort()[0];
  if (!first) return null;
  const planted = /^\d{4}-\d{2}-\d{2}$/.test(plantedOn || "") ? new Date(`${plantedOn}T00:00:00Z`) : null;
  const visit = new Date(`${first}T00:00:00Z`);
  const years = planted && Number.isFinite(planted.getTime()) && planted <= visit ? (visit - planted) / (365.2425 * 86400000) : null;
  return { date: first, years };
}
