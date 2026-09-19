const EMPTY_ACCESS = Object.freeze({ mode: "empty", orchardId: null });

export function resolveOrchardAccess(hash, session) {
  const harvestWatering =
    /^#\/orchards\/(\d+)\/share\/harvest-watering\/([^/]+)$/.exec(hash || "");
  if (harvestWatering) {
    return {
      mode: "shared",
      orchardId: Number(harvestWatering[1]),
      shareToken: decodeFragmentPart(harvestWatering[2]),
      permissions: {},
    };
  }

  const watering = /^#\/orchards\/(\d+)\/share\/watering\/([^/]+)$/.exec(
    hash || "",
  );
  if (watering) {
    return {
      mode: "shared",
      orchardId: Number(watering[1]),
      shareToken: decodeFragmentPart(watering[2]),
      permissions: {},
    };
  }

  const shared = /^#\/orchards\/(\d+)\/share\/([^/]+)$/.exec(hash || "");
  if (shared) {
    return {
      mode: "shared",
      orchardId: Number(shared[1]),
      shareToken: decodeFragmentPart(shared[2]),
      permissions: {},
    };
  }

  const owned = /^#\/orchards\/(\d+)$/.exec(hash || "");
  if (!owned) return { ...EMPTY_ACCESS };
  const orchardId = Number(owned[1]);
  if (!session) return { mode: "login-required", orchardId };
  const orchard = (session.orchards || []).find(({ id }) => id === orchardId);
  return orchard
    ? { mode: "editable", orchardId, orchard }
    : { ...EMPTY_ACCESS };
}

export function hasOpenOrchard(access) {
  return ["editable", "shared"].includes(access.mode);
}

export function canWaterOrchard(access) {
  return access.mode === "editable" || access.permissions?.water === true;
}

export function canHarvestOrchard(access) {
  return access.mode === "editable" || access.permissions?.harvest === true;
}

export function canAddPhotosToOrchard(access) {
  return access.mode === "editable" || access.permissions?.add_photos === true;
}

export function orchardResourceUrl(access, resource) {
  if (access.orchardId == null) throw new Error("No orchard is open.");
  return `/api/orchards/${encodeURIComponent(access.orchardId)}/${resource}`;
}

export function accessHeaders(access) {
  return access.mode === "shared"
    ? { "x-orchard-share-token": access.shareToken }
    : {};
}

export function sharedOrchardUrl(origin, orchardId, shareToken) {
  return `${origin.replace(/\/$/, "")}/#/orchards/${encodeURIComponent(
    orchardId,
  )}/share/${encodeURIComponent(shareToken)}`;
}

function decodeFragmentPart(value) {
  try {
    return decodeURIComponent(value);
  } catch {
    return "";
  }
}
