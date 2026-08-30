import assert from "node:assert/strict";
import test from "node:test";

import {
  filterTreeFeatures,
  summarizeSpecies,
  taxonomyOptions,
} from "./orchard-inventory.mjs";

const trees = [
  tree(1, {
    identityId: 10,
    name: "Apple",
    taxonName: "Malus domestica",
    botanicalName: "Malus domestica ‘Gala’",
    cultivar: "Gala",
    genera: ["Malus"],
    species: ["Malus domestica"],
    roles: ["fruit", "pioneer"],
  }),
  tree(2, {
    identityId: 10,
    name: "Apple",
    taxonName: "Malus domestica",
    botanicalName: "Malus domestica ‘Golden Delicious’",
    cultivar: "Golden Delicious",
    genera: ["Malus"],
    species: ["Malus domestica"],
    roles: ["fruit"],
  }),
  tree(3, {
    identityId: 20,
    name: "European pear",
    taxonName: "Pyrus communis",
    botanicalName: "Pyrus communis",
    genera: ["Pyrus"],
    species: ["Pyrus communis"],
    roles: ["fruit"],
  }),
  tree(4, {
    identityId: 30,
    name: "Apple × pear",
    taxonName: "Malus domestica × Pyrus communis",
    botanicalName: "Malus domestica × Pyrus communis",
    genera: ["Malus", "Pyrus"],
    species: ["Malus domestica", "Pyrus communis"],
    roles: ["pioneer"],
  }),
  tree(5, {
    identityId: 10,
    name: "Apple",
    taxonName: "Malus domestica",
    botanicalName: "Malus domestica ‘Gala’",
    cultivar: "Gala",
    genera: ["Malus"],
    species: ["Malus domestica"],
    roles: ["fruit"],
  }),
];

test("filter trees by role, genus, and species", () => {
  assert.deepEqual(
    filterTreeFeatures(trees, {
      role: "pioneer",
      genus: "Malus",
      species: "Malus domestica",
    }).map(({ id }) => id),
    [1, 4],
  );

  assert.deepEqual(
    filterTreeFeatures(trees, { role: "fruit", genus: "Pyrus" }).map(
      ({ id }) => id,
    ),
    [3],
  );
});

test("list unique genus and species filter options", () => {
  assert.deepEqual(taxonomyOptions(trees), {
    genera: ["Malus", "Pyrus"],
    species: ["Malus domestica", "Pyrus communis"],
  });
  assert.deepEqual(taxonomyOptions(trees, "Malus").species, [
    "Malus domestica",
  ]);
});

test("count planted trees by species and detail their cultivars", () => {
  assert.deepEqual(summarizeSpecies(trees), [
    {
      plantIdentityId: 10,
      taxonName: "Malus domestica",
      names: ["Apple"],
      count: 3,
      harvestStart: null,
      harvestEnd: null,
      cultivars: [
        { name: "Gala", count: 2 },
        { name: "Golden Delicious", count: 1 },
      ],
    },
    {
      plantIdentityId: 30,
      taxonName: "Malus domestica × Pyrus communis",
      names: ["Apple × pear"],
      count: 1,
      harvestStart: null,
      harvestEnd: null,
      cultivars: [],
    },
    {
      plantIdentityId: 20,
      taxonName: "Pyrus communis",
      names: ["European pear"],
      count: 1,
      harvestStart: null,
      harvestEnd: null,
      cultivars: [],
    },
  ]);
});

function tree(id, identity) {
  return {
    id,
    properties: {
      plant_identity_id: identity.identityId,
      plant_identity_name: identity.name,
      plant_identity_taxon_name: identity.taxonName,
      plant_identity_botanical_name: identity.botanicalName,
      plant_identity_cultivar: identity.cultivar ?? null,
      harvest_start: identity.harvestStart ?? null,
      harvest_end: identity.harvestEnd ?? null,
      botanical_genera: identity.genera,
      botanical_species: identity.species,
      roles: identity.roles,
    },
  };
}
