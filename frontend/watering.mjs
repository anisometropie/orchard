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

export function wateringCancellationNeedsConfirmation(progress) {
  return Number(progress?.watered_tree_count) > 0;
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

export function wateringWideCenter(source, tree) {
  const treeCoordinate = [Number(tree?.longitude), Number(tree?.latitude)];
  if (!treeCoordinate.every(Number.isFinite)) return null;
  const sourceCoordinate = [
    Number(source?.longitude),
    Number(source?.latitude),
  ];
  if (!sourceCoordinate.every(Number.isFinite)) return treeCoordinate;
  return sourceCoordinate.map(
    (coordinate, index) =>
      coordinate + (treeCoordinate[index] - coordinate) / 3,
  );
}

export function adaptiveWateringWideZoom({
  baseZoom,
  minimumZoom,
  source,
  tree,
  usableWidth,
  usableHeight,
}) {
  const center = wateringWideCenter(source, tree);
  if (
    !center ||
    !Number.isFinite(baseZoom) ||
    !Number.isFinite(usableWidth) ||
    !Number.isFinite(usableHeight) ||
    usableWidth <= 0 ||
    usableHeight <= 0
  ) return baseZoom;
  const sourceCoordinate = [
    Number(source?.longitude),
    Number(source?.latitude),
  ];
  const treeCoordinate = [Number(tree?.longitude), Number(tree?.latitude)];
  if (
    !sourceCoordinate.every(Number.isFinite) ||
    !treeCoordinate.every(Number.isFinite)
  ) return baseZoom;

  const toMercator = ([longitude, latitude]) => {
    const limitedLatitude = Math.max(-85.051129, Math.min(85.051129, latitude));
    const latitudeRadians = (limitedLatitude * Math.PI) / 180;
    return [
      (longitude + 180) / 360,
      (1 - Math.asinh(Math.tan(latitudeRadians)) / Math.PI) / 2,
    ];
  };
  const [centerX, centerY] = toMercator(center);
  const [treeX, treeY] = toMercator(treeCoordinate);
  const pixelsAtBaseZoom = 512 * 2 ** baseZoom;
  const horizontalPixels = Math.abs(treeX - centerX) * pixelsAtBaseZoom;
  const verticalPixels = Math.abs(treeY - centerY) * pixelsAtBaseZoom;
  const horizontalScale = horizontalPixels === 0
    ? Number.POSITIVE_INFINITY
    : usableWidth / 2 / horizontalPixels;
  const verticalScale = verticalPixels === 0
    ? Number.POSITIVE_INFINITY
    : usableHeight / 2 / verticalPixels;
  const fittingScale = Math.min(horizontalScale, verticalScale);
  if (!Number.isFinite(fittingScale)) return baseZoom;
  return Math.max(
    Number.isFinite(minimumZoom) ? minimumZoom : 0,
    baseZoom + Math.min(0, Math.log2(fittingScale)),
  );
}

export function wateringRouteWindow(route, currentTreeId, maximumTrees = 4) {
  const completeRoute = Array.isArray(route) ? route : [];
  const currentIndex = completeRoute.findIndex(
    (tree) => String(tree.id) === String(currentTreeId),
  );
  const startIndex = currentIndex < 0 ? 0 : currentIndex;
  return {
    trees: completeRoute.slice(
      startIndex,
      startIndex + Math.max(0, maximumTrees),
    ),
    startIndex,
  };
}

export function dangerWateringPathGeoJson(
  source,
  route,
  startIndex = 0,
  totalRouteLength = route?.length || 0,
) {
  if (!source || !Array.isArray(route)) {
    return { type: "FeatureCollection", features: [] };
  }
  const sourceCoordinate = [source.longitude, source.latitude];
  const features = [];
  for (let index = 0; index < route.length;) {
    const globalIndex = startIndex + index;
    const positionInTrip = globalIndex % 2;
    const trip = route.slice(index, index + 2 - positionInTrip);
    const lastGlobalIndex = globalIndex + trip.length - 1;
    const finishesTrip = lastGlobalIndex % 2 === 1;
    const finishesRoute = lastGlobalIndex === totalRouteLength - 1;
    features.push({
      type: "Feature",
      properties: {
        trip_parity:
          Math.floor(globalIndex / 2) % 2 === 0 ? "even" : "odd",
      },
      geometry: {
        type: "LineString",
        coordinates: [
          ...(positionInTrip === 0 ? [sourceCoordinate] : []),
          ...trip.map((tree) => [tree.longitude, tree.latitude]),
          ...(finishesTrip || finishesRoute ? [sourceCoordinate] : []),
        ],
      },
    });
    index += trip.length;
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

export function dangerWateringNumberGeoJson(
  route,
  currentTreeId = null,
  startIndex = 0,
) {
  return {
    type: "FeatureCollection",
    features: Array.isArray(route)
      ? route.map((tree, index) => {
          const globalIndex = startIndex + index;
          const isCurrent =
            currentTreeId != null && String(tree.id) === String(currentTreeId);
          const marker = dangerWateringNumberMarker(globalIndex, isCurrent);
          return {
            type: "Feature",
            id: tree.id,
            properties: {
              route_number: globalIndex + 1,
              route_number_image: marker.imageName,
              trip_parity:
                Math.floor(globalIndex / 2) % 2 === 0 ? "even" : "odd",
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
