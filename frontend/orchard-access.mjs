const EMPTY_ACCESS = Object.freeze({ mode: "empty", orchardId: null });

export function resolveOrchardAccess(hash, session) {
  const harvestWatering =
    /^#\/orchards\/(\d+)\/share\/harvest-watering\/([^/]+)$/.exec(hash || "");
  if (harvestWatering) {
    return {
      mode: "harvest-watering",
      orchardId: Number(harvestWatering[1]),
      shareToken: decodeFragmentPart(harvestWatering[2]),
    };
  }

  const watering = /^#\/orchards\/(\d+)\/share\/watering\/([^/]+)$/.exec(
    hash || "",
  );
  if (watering) {
    return {
      mode: "watering",
      orchardId: Number(watering[1]),
      shareToken: decodeFragmentPart(watering[2]),
    };
  }

  const shared = /^#\/orchards\/(\d+)\/share\/([^/]+)$/.exec(hash || "");
  if (shared) {
    return {
      mode: "read-only",
      orchardId: Number(shared[1]),
      shareToken: decodeFragmentPart(shared[2]),
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
  return ["editable", "read-only", "watering", "harvest-watering"].includes(
    access.mode,
  );
}

export function canWaterOrchard(access) {
  return ["editable", "watering", "harvest-watering"].includes(access.mode);
}

export function canHarvestOrchard(access) {
  return ["editable", "harvest-watering"].includes(access.mode);
}

export function orchardResourceUrl(access, resource) {
  if (access.orchardId == null) throw new Error("No orchard is open.");
  return `/api/orchards/${encodeURIComponent(access.orchardId)}/${resource}`;
}

export function accessHeaders(access) {
  return ["read-only", "watering", "harvest-watering"].includes(access.mode)
    ? { "x-orchard-share-token": access.shareToken }
    : {};
}

export function sharedOrchardUrl(
  origin,
  orchardId,
  shareToken,
  permission = "view",
) {
  const permissionPath =
    permission === "watering"
      ? "/watering"
      : permission === "harvest-watering"
        ? "/harvest-watering"
        : "";
  return `${origin.replace(/\/$/, "")}/#/orchards/${encodeURIComponent(
    orchardId,
  )}/share${permissionPath}/${encodeURIComponent(shareToken)}`;
}

function decodeFragmentPart(value) {
  try {
    return decodeURIComponent(value);
  } catch {
    return "";
  }
}
