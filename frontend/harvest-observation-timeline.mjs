const DAY = 86400000;
const dateFormatter = new Intl.DateTimeFormat("en-GB", {
  day: "numeric", month: "short", year: "numeric", timeZone: "UTC",
});

function dateDay(value) {
  if (typeof value !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const timestamp = Date.parse(`${value}T00:00:00Z`);
  if (!Number.isFinite(timestamp) || new Date(timestamp).toISOString().slice(0, 10) !== value) return null;
  return timestamp / DAY;
}

export function harvestTimelineLayout(point) {
  const start = dateDay(point.window_start);
  const end = dateDay(point.window_end);
  const observed = dateDay(point.recorded_on);
  if (start == null || end == null || observed == null || end < start) return null;
  const first = Math.min(start, observed);
  const last = Math.max(end, observed);
  const padding = Math.max(1, Math.ceil((last - first) * 0.12));
  const extent = last - first + padding * 2;
  const position = (day) => (day - first + padding) / extent * 100;
  const gap = observed < start ? start - observed : observed - end;
  const relativeDate = observed < start
    ? `${gap} day${gap === 1 ? "" : "s"} before window`
    : observed > end
      ? `${gap} day${gap === 1 ? "" : "s"} after window`
      : `Day ${observed - start + 1} of ${end - start + 1}`;
  return {
    start: position(start), end: position(end), observed: position(observed), relativeDate,
    startLabel: dateFormatter.format(start * DAY), endLabel: dateFormatter.format(end * DAY),
    observedLabel: dateFormatter.format(observed * DAY),
    singleDay: start === end,
    description: `Saved harvest window ${point.window_start} to ${point.window_end}. Observation ${point.recorded_on}. ${relativeDate}.`,
  };
}

function span(className, text = "") {
  const element = document.createElement("span");
  element.className = className;
  element.textContent = text;
  return element;
}

function positionLabel(element, position, inset) {
  element.style.left = `clamp(${inset}px, ${position}%, calc(100% - ${inset}px))`;
}

export function createHarvestObservationTimeline(point) {
  const layout = harvestTimelineLayout(point);
  if (!layout) return null;
  const timeline = document.createElement("span");
  timeline.className = "harvest-observation-timeline";
  timeline.setAttribute("role", "img");
  timeline.setAttribute("aria-label", layout.description);
  const heading = span("harvest-timeline-heading");
  heading.setAttribute("aria-hidden", "true");
  heading.append(span("harvest-timeline-position", layout.relativeDate));
  const plot = span("harvest-timeline-plot");
  plot.setAttribute("aria-hidden", "true");
  const axis = span("harvest-timeline-axis");
  const window = span("harvest-timeline-window");
  window.style.left = `${layout.start}%`;
  window.style.width = `${layout.end - layout.start}%`;
  window.classList.toggle("harvest-timeline-single-day", layout.singleDay);
  const marker = span("harvest-timeline-marker");
  marker.style.left = `${layout.observed}%`;
  const observedLabel = span("harvest-timeline-observed", `Observed ${layout.observedLabel}`);
  positionLabel(observedLabel, layout.observed, 76);
  plot.append(axis, window, marker, observedLabel);
  if (layout.end - layout.start < 45) {
    const dates = span("harvest-timeline-date", layout.singleDay
      ? layout.startLabel : `${layout.startLabel} – ${layout.endLabel}`);
    positionLabel(dates, (layout.start + layout.end) / 2, 90);
    plot.append(dates);
  } else {
    for (const [position, date] of [[layout.start, layout.startLabel], [layout.end, layout.endLabel]]) {
      const label = span("harvest-timeline-date", date);
      positionLabel(label, position, 40);
      plot.append(label);
    }
  }
  timeline.append(heading, plot);
  return timeline;
}
