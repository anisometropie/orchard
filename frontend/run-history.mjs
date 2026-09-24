const HARVEST_PART_LABELS = Object.freeze({
  cone: "cones",
  flower: "flowers",
  fruit: "fruits",
  leaf: "leaves",
  nut: "nuts",
  pod: "pods",
  seed: "seeds",
});

export function wateringTreeOutcomeLabel(tree) {
  if (tree.skipped_at_unix_seconds != null) return "Skipped (dead)";
  return tree.watered_at_unix_seconds == null ? "Not watered" : "Watered";
}

export function harvestPartsLabel(parts) {
  const label = (parts || [])
    .map((part) => HARVEST_PART_LABELS[part] || part)
    .join(", ");
  return label ? `${label[0].toUpperCase()}${label.slice(1)}` : "None";
}

export function harvestRunStatus(run) {
  return run.handled_tree_count === run.total_tree_count
    ? "Completed"
    : "Stopped early";
}

export function visibleHarvestTrees(run) {
  return (run.trees || []).filter(
    (tree) => tree.outcome?.kind !== "not_handled",
  );
}

export function harvestOutcomeLabel(outcome) {
  switch (outcome?.kind) {
    case "harvested_everything":
      return `Harvested everything on ${outcome.recorded_on}`;
    case "done_for_window":
      return `Done for that window on ${outcome.recorded_on}`;
    case "deferred":
      return `Done for now on ${outcome.recorded_on} · retry ${outcome.retry_on}`;
    default:
      return "Not handled";
  }
}
