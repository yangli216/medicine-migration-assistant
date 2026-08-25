export function inventoryLocationNeedsSourceResolution(location) {
  return (
    location?.sourceKind === "WAREHOUSE" &&
    `${location?.sourceLocationKey || ""}`.startsWith("YKORG:")
  );
}

export function completeInventoryOrganizationIds(
  locations,
  organizationMappings,
  locationMappings,
  resolvedLocations,
  selectedLocationKeys,
) {
  const selectedLocationKeySet =
    selectedLocationKeys === undefined
      ? null
      : new Set(selectedLocationKeys || []);
  const scopedLocations = selectedLocationKeySet
    ? locations.filter((location) =>
        selectedLocationKeySet.has(location.sourceLocationKey),
      )
    : locations;
  const sourceOrganizationIds = [
    ...new Set(
      scopedLocations
        .map((location) => location.organizationId)
        .filter(Boolean),
    ),
  ];
  return sourceOrganizationIds.filter((sourceOrganizationId) => {
    if (!organizationMappings[sourceOrganizationId]) return false;
    const organizationLocations = scopedLocations.filter(
      (location) => location.organizationId === sourceOrganizationId,
    );
    return (
      organizationLocations.length > 0 &&
      organizationLocations.every(
        (location) =>
          locationMappings[location.sourceLocationKey] &&
          (!inventoryLocationNeedsSourceResolution(location) ||
            resolvedLocations[location.sourceLocationKey]),
      )
    );
  });
}
