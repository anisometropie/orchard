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

export function dangerTreeCount(features) {
  return features.filter(
    (feature) =>
      feature.properties?.is_alive !== false &&
      feature.properties?.is_in_danger === true,
  ).length;
}

export function defaultWaterSource(features) {
  const referenceTree = features.find((feature) =>
    String(feature.properties?.name || "").includes("Ronde de Bordeaux"),
  );
  if (referenceTree) {
    return {
      longitude: Number(referenceTree.geometry.coordinates[0]),
      latitude: Number(referenceTree.geometry.coordinates[1]) + 5 / 111_320,
    };
  }
  const coordinates = features
    .map((feature) => feature.geometry?.coordinates)
    .filter(
      (coordinate) =>
        Array.isArray(coordinate) &&
        Number.isFinite(Number(coordinate[0])) &&
        Number.isFinite(Number(coordinate[1])),
    );
  if (coordinates.length === 0) return null;
  const longitudes = coordinates.map((coordinate) => Number(coordinate[0]));
  const latitudes = coordinates.map((coordinate) => Number(coordinate[1]));
  return {
    longitude: (Math.min(...longitudes) + Math.max(...longitudes)) / 2,
    latitude: Math.max(...latitudes) + 5 / 111_320,
  };
}

export function wateringStartRequest(target, rowName, waterSource) {
  return target === "danger"
    ? { target: "danger", water_source: waterSource }
    : { row_name: rowName };
}

export function waterSourceGeoJson(source) {
  return {
    type: "FeatureCollection",
    features: source
      ? [
          {
            type: "Feature",
            geometry: {
              type: "Point",
              coordinates: [source.longitude, source.latitude],
            },
            properties: {},
          },
        ]
      : [],
  };
}

export function dangerWateringPathGeoJson(source, route) {
  if (!source || !Array.isArray(route)) {
    return { type: "FeatureCollection", features: [] };
  }
  const sourceCoordinate = [source.longitude, source.latitude];
  const features = [];
  for (let index = 0; index < route.length; index += 2) {
    const trip = route.slice(index, index + 2);
    features.push({
      type: "Feature",
      properties: { trip_parity: index % 4 === 0 ? "even" : "odd" },
      geometry: {
        type: "LineString",
        coordinates: [
          sourceCoordinate,
          ...trip.map((tree) => [tree.longitude, tree.latitude]),
          sourceCoordinate,
        ],
      },
    });
  }
  return { type: "FeatureCollection", features };
}

export function dangerWateringNumberMarker(index, isCurrent = false) {
  const routeNumber = index + 1;
  return {
    imageName: isCurrent
      ? `watering-route-current-${routeNumber}`
      : `watering-route-number-${routeNumber}`,
    fillColor: isCurrent
      ? "#d51f2e"
      : Math.floor(index / 2) % 2 === 0
        ? "#1677b8"
        : "#8b3fb0",
    scale: isCurrent ? 1.35 : 1,
  };
}

export function dangerWateringNumberGeoJson(route, currentTreeId = null) {
  return {
    type: "FeatureCollection",
    features: Array.isArray(route)
      ? route.map((tree, index) => {
          const isCurrent =
            currentTreeId != null && String(tree.id) === String(currentTreeId);
          const marker = dangerWateringNumberMarker(index, isCurrent);
          return {
            type: "Feature",
            id: tree.id,
            properties: {
              route_number: index + 1,
              route_number_image: marker.imageName,
              trip_parity: Math.floor(index / 2) % 2 === 0 ? "even" : "odd",
              is_current: isCurrent,
              route_number_scale: marker.scale,
            },
            geometry: {
              type: "Point",
              coordinates: [tree.longitude, tree.latitude],
            },
          };
        })
      : [],
  };
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
