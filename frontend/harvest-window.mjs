const DAY_IN_MILLISECONDS = 24 * 60 * 60 * 1000;

export function parseAnnualDate(value) {
  const match = /^(\d{2})-(\d{2})$/.exec(value.trim());
  if (!match) return null;
  const month = Number(match[1]);
  const day = Number(match[2]);
  if (!isValidAnnualDate(month, day)) return null;
  return { month, day };
}

export function formatHarvestWindows(windows) {
  const configuredWindows = normalizeHarvestWindows(windows).filter(
    (window) => window?.start && window?.end,
  );
  return configuredWindows.length > 0
    ? configuredWindows
        .map(({ start, end }) => `${start} → ${end}`)
        .join(" · ")
    : "Not set";
}

export function harvestAvailability(features, startDate, weekCount) {
  const weeks = Math.max(1, Number(weekCount) || 1);
  const selectionStart = startOfUtcDay(startDate);
  const selectionEnd = new Date(
    selectionStart.getTime() + (weeks * 7 - 1) * DAY_IN_MILLISECONDS,
  );

  return features.map((feature) => {
    const properties = feature.properties || {};
    const availableParts =
      properties.is_alive === false
        ? []
        : [
            ...new Set(
              harvestWindows(properties)
                .filter(({ start, end }) =>
                  recurringWindowOverlaps(
                    start,
                    end,
                    selectionStart,
                    selectionEnd,
                  ),
                )
                .map(({ harvested_part: harvestedPart }) =>
                  harvestedPart || "fruit",
                ),
            ),
          ];
    return {
      ...feature,
      properties: {
        ...properties,
        harvest_available: availableParts.length > 0,
        harvest_available_parts: availableParts,
      },
    };
  });
}

export function harvestLayerFilter(plantingDateFilter = null) {
  const availabilityFilter = ["==", ["get", "harvest_available"], true];
  return plantingDateFilter
    ? ["all", availabilityFilter, plantingDateFilter]
    : availabilityFilter;
}

function harvestWindows(properties) {
  return normalizeHarvestWindows(properties.harvest_windows);
}

function normalizeHarvestWindows(windows) {
  if (Array.isArray(windows)) return windows;
  if (typeof windows !== "string") return [];
  try {
    const parsed = JSON.parse(windows);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export function harvestAvailabilitySummary(features) {
  const available = features.filter(
    ({ properties = {} }) => properties.harvest_available === true,
  );
  return {
    trees: available.length,
    species: new Set(
      available.map(({ properties }) => properties.plant_identity_id),
    ).size,
  };
}

function recurringWindowOverlaps(
  harvestStartValue,
  harvestEndValue,
  selectionStart,
  selectionEnd,
) {
  const harvestStart = parseAnnualDate(harvestStartValue || "");
  const harvestEnd = parseAnnualDate(harvestEndValue || "");
  if (!harvestStart || !harvestEnd) return false;

  const firstYear = selectionStart.getUTCFullYear() - 1;
  const lastYear = selectionEnd.getUTCFullYear() + 1;
  for (let year = firstYear; year <= lastYear; year += 1) {
    const recurringStart = annualDateInYear(harvestStart, year);
    let recurringEnd = annualDateInYear(harvestEnd, year);
    if (!recurringStart || !recurringEnd) continue;
    if (recurringEnd < recurringStart) {
      recurringEnd = annualDateInYear(harvestEnd, year + 1);
      if (!recurringEnd) continue;
    }
    if (recurringStart <= selectionEnd && recurringEnd >= selectionStart) {
      return true;
    }
  }
  return false;
}

function annualDateInYear(annualDate, year) {
  const lastDay = new Date(Date.UTC(year, annualDate.month, 0)).getUTCDate();
  if (annualDate.day > lastDay) return null;
  return new Date(Date.UTC(year, annualDate.month - 1, annualDate.day));
}

function startOfUtcDay(date) {
  return new Date(
    Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()),
  );
}

function isValidAnnualDate(month, day) {
  if (!Number.isInteger(month) || month < 1 || month > 12) return false;
  if (!Number.isInteger(day) || day < 1) return false;
  const lastDay = new Date(Date.UTC(2000, month, 0)).getUTCDate();
  return day <= lastDay;
}
