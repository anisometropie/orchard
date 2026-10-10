export function harvestObservationPath(schedule) {
  const cultivar = schedule.cultivarId != null;
  const id = Number(cultivar ? schedule.cultivarId : schedule.plantIdentityId);
  if (!Number.isSafeInteger(id) || id <= 0) throw new TypeError("A species or cultivar is required.");
  return `harvest-observations/${cultivar ? "cultivar" : "species"}/${id}`;
}

export function pendingObservationIds(points) {
  return points.filter((point) => !point.reviewed).map((point) => point.id);
}

export function harvestReviewRequest(ids, windowChange = null) {
  if (!Array.isArray(ids) || !ids.length || new Set(ids).size !== ids.length ||
      ids.some((id) => !Number.isSafeInteger(id) || id <= 0)) {
    throw new TypeError("Select at least one unreviewed observation.");
  }
  return { observation_ids: [...ids], window_change: windowChange };
}
