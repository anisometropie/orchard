import { formatHarvestWindows } from "./harvest-window.mjs?v=4";

export function treeTooltipFields(properties = {}) {
  return [
    ["Name", properties.name],
    ["Latin name", properties.latin_name],
    ["Date", properties.planted_on],
    ["Harvest", formatHarvestWindows(properties.harvest_windows)],
  ].filter(([, value]) => value != null && value !== "");
}

export function filterTreeFeatures(
  features,
  { role = "", genus = "", species = "" } = {},
) {
  return features.filter(({ properties = {} }) => {
    const roles = arrayProperty(properties.roles);
    const genera = arrayProperty(properties.botanical_genera);
    const speciesNames = arrayProperty(properties.botanical_species);

    return (
      (!role || roles.includes(role)) &&
      (!genus || genera.includes(genus)) &&
      (!species || speciesNames.includes(species))
    );
  });
}

export function filterTreeFeaturesByIdentity(features, selection) {
  if (!selection) return features;

  return features.filter(({ properties = {} }) => {
    const sameIdentity =
      selection.plantIdentityId != null
        ? properties.plant_identity_id === selection.plantIdentityId
        : properties.plant_identity_taxon_name === selection.taxonName;
    if (!sameIdentity) return false;

    if (selection.cultivarId != null) {
      return properties.plant_cultivar_id === selection.cultivarId;
    }
    if (selection.cultivarName) {
      return properties.plant_identity_cultivar === selection.cultivarName;
    }
    return true;
  });
}

export function taxonomyOptions(features, selectedGenus = "") {
  const genera = new Set();
  const species = new Set();

  features.forEach(({ properties = {} }) => {
    arrayProperty(properties.botanical_genera).forEach((genus) =>
      genera.add(genus),
    );
    arrayProperty(properties.botanical_species)
      .filter(
        (speciesName) =>
          !selectedGenus || speciesName.startsWith(`${selectedGenus} `),
      )
      .forEach((speciesName) => species.add(speciesName));
  });

  return {
    genera: [...genera].sort(compareText),
    species: [...species].sort(compareText),
  };
}

export function summarizeSpecies(features) {
  const species = new Map();

  features.forEach(({ properties = {} }) => {
    const name = properties.plant_identity_name || "Unknown identity";
    const taxonName = properties.plant_identity_taxon_name || "Unknown taxon";
    const plantIdentityId = properties.plant_identity_id ?? null;
    const cultivar = properties.plant_identity_cultivar;
    const cultivarId = properties.plant_cultivar_id ?? null;
    const harvestWindows = arrayProperty(properties.harvest_windows);
    const summaryKey = plantIdentityId ?? taxonName;
    let summary = species.get(summaryKey);

    if (!summary) {
      summary = {
        plantIdentityId,
        taxonName,
        names: new Set(),
        count: 0,
        cultivarSummaries: new Map(),
        cultivarless: null,
      };
      species.set(summaryKey, summary);
    }
    summary.names.add(name);
    summary.count += 1;
    if (cultivar) {
      const cultivarKey = cultivarId ?? cultivar;
      let cultivarSummary = summary.cultivarSummaries.get(cultivarKey);
      if (!cultivarSummary) {
        cultivarSummary = {
          id: cultivarId,
          name: cultivar,
          count: 0,
          harvestWindows,
        };
        summary.cultivarSummaries.set(cultivarKey, cultivarSummary);
      }
      cultivarSummary.count += 1;
    } else {
      summary.cultivarless ??= { count: 0, harvestWindows };
      summary.cultivarless.count += 1;
    }
  });

  return [...species.values()]
    .map(({
      plantIdentityId,
      taxonName,
      names,
      count,
      cultivarSummaries,
      cultivarless,
    }) => ({
      plantIdentityId,
      taxonName,
      names: [...names].sort(compareText),
      count,
      cultivarless,
      cultivars: [...cultivarSummaries.values()]
        .sort((left, right) => compareText(left.name, right.name)),
    }))
    .sort((left, right) => compareText(left.taxonName, right.taxonName));
}

export function harvestScheduleEndpoint(
  { plantIdentityId, cultivarId },
  orchardId,
) {
  if (orchardId == null) throw new Error("No orchard is open.");
  const orchardPrefix = `/api/orchards/${encodeURIComponent(orchardId)}`;
  if (cultivarId != null) {
    return `${orchardPrefix}/plant-cultivars/${encodeURIComponent(cultivarId)}/harvest-windows`;
  }
  if (plantIdentityId != null) {
    return `${orchardPrefix}/plant-identities/${encodeURIComponent(plantIdentityId)}/harvest-windows`;
  }
  throw new Error("This harvest schedule has no editable ID.");
}

export function harvestSchedulesForSpecies(speciesSummary) {
  const schedules = [];
  if (speciesSummary.cultivarless) {
    schedules.push({
      label: null,
      count: speciesSummary.cultivarless.count,
      plantIdentityId: speciesSummary.plantIdentityId,
      cultivarId: null,
      harvestWindows: speciesSummary.cultivarless.harvestWindows,
    });
  }
  speciesSummary.cultivars.forEach((cultivar) => {
    schedules.push({
      label: cultivar.name,
      count: cultivar.count,
      plantIdentityId: speciesSummary.plantIdentityId,
      cultivarId: cultivar.id,
      harvestWindows: cultivar.harvestWindows,
    });
  });
  return schedules;
}

export function harvestWindowSpan(schedules) {
  const windows = schedules.flatMap(({ harvestWindows = [] }) =>
    harvestWindows.filter(({ start, end }) => start && end),
  );
  if (windows.length === 0) return null;

  return {
    start: windows.reduce(
      (earliest, window) => window.start < earliest ? window.start : earliest,
      windows[0].start,
    ),
    end: windows.reduce(
      (latest, window) => window.end > latest ? window.end : latest,
      windows[0].end,
    ),
  };
}

function arrayProperty(value) {
  return Array.isArray(value) ? value : [];
}

function compareText(left, right) {
  return left.localeCompare(right);
}
