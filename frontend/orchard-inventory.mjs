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
    const cultivar = properties.plant_identity_cultivar;
    let summary = species.get(taxonName);

    if (!summary) {
      summary = {
        taxonName,
        names: new Set(),
        count: 0,
        cultivarCounts: new Map(),
      };
      species.set(taxonName, summary);
    }
    summary.names.add(name);
    summary.count += 1;
    if (cultivar) {
      summary.cultivarCounts.set(
        cultivar,
        (summary.cultivarCounts.get(cultivar) || 0) + 1,
      );
    }
  });

  return [...species.values()]
    .map(({ taxonName, names, count, cultivarCounts }) => ({
      taxonName,
      names: [...names].sort(compareText),
      count,
      cultivars: [...cultivarCounts]
        .map(([name, cultivarCount]) => ({ name, count: cultivarCount }))
        .sort((left, right) => compareText(left.name, right.name)),
    }))
    .sort((left, right) => compareText(left.taxonName, right.taxonName));
}

function arrayProperty(value) {
  return Array.isArray(value) ? value : [];
}

function compareText(left, right) {
  return left.localeCompare(right);
}
