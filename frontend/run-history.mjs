import { harvestVisitLabel } from "./harvest-visit.mjs?v=3";

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
    case "too_soon":
      return `Too soon / not ready on ${outcome.recorded_on} · retry ${outcome.retry_on}`;
    case "too_late":
      return `All wasted on ${outcome.recorded_on}`;
    case "empty_on_arrival":
      return `Empty on arrival on ${outcome.recorded_on}`;
    case "fruit_remaining":
      return `Fruit left on ${outcome.recorded_on} · retry ${outcome.retry_on}`;
    case "deferred":
      return `Done for now on ${outcome.recorded_on} · retry ${outcome.retry_on}`;
    default:
      return "Not handled";
  }
}

export function harvestTreeHistoryDetails(tree) {
  const observationKinds = {
    harvested_everything: ["harvested_all"],
    empty_on_arrival: ["empty_on_arrival"],
    fruit_remaining: ["fruit_remaining"],
    too_soon: ["too_soon"],
    too_late: ["too_late", "all_wasted"],
  };
  const observationMatchesOutcome = tree.observation &&
    observationKinds[tree.outcome?.kind]?.includes(tree.observation.kind);
  const outcome = observationMatchesOutcome
    ? `${harvestVisitLabel(tree.observation)} on ${tree.outcome.recorded_on}${tree.outcome.retry_on ? ` · retry ${tree.outcome.retry_on}` : ""}`
    : harvestOutcomeLabel(tree.outcome);
  return [
    outcome,
    `${harvestPartsLabel(tree.harvested_parts)} · window ${tree.window_start} to ${tree.window_end}`,
    ...(tree.observation && !observationMatchesOutcome
      ? [`classified as: ${harvestVisitLabel(tree.observation)}`]
      : []),
  ];
}
