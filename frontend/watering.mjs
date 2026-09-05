export function orchardRows(features) {
  const rows = new Map();
  for (const feature of features) {
    const name = feature.properties?.row_name;
    if (typeof name !== "string" || name.trim() === "") continue;
    const trees = rows.get(name) || [];
    trees.push(feature);
    rows.set(name, trees);
  }

  return [...rows.entries()]
    .sort(([left], [right]) => left.localeCompare(right, undefined, { numeric: true }))
    .map(([name, trees]) => {
      const ranks = trees
        .map((tree) => Number(tree.properties?.row_rank))
        .sort((left, right) => left - right);
      return {
        name,
        treeCount: trees.length,
        livingTreeCount: trees.filter((tree) => tree.properties?.is_alive !== false)
          .length,
        isOrdered: ranks.every((rank, index) => rank === index + 1),
      };
    });
}

export function treeIdsInRow(features, rowName) {
  return features
    .filter((feature) => feature.properties?.row_name === rowName)
    .map((feature) => Number(feature.id));
}

export function appendManualTree(orderedTreeIds, treeId, allowedTreeIds) {
  const normalizedTreeId = Number(treeId);
  if (
    !allowedTreeIds.includes(normalizedTreeId) ||
    orderedTreeIds.includes(normalizedTreeId)
  ) {
    return orderedTreeIds;
  }
  return [...orderedTreeIds, normalizedTreeId];
}

export function wateringTargetGeoJson(tree) {
  return {
    type: "FeatureCollection",
    features: tree
      ? [
          {
            type: "Feature",
            id: tree.id,
            geometry: {
              type: "Point",
              coordinates: [tree.longitude, tree.latitude],
            },
            properties: { name: tree.name },
          },
        ]
      : [],
  };
}
