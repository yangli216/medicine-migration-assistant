export function completeInventoryOrganizationIds(
  locations,
  organizationMappings,
  locationMappings,
  resolvedLocations,
) {
  const sourceOrganizationIds = [
    ...new Set(
      locations.map((location) => location.organizationId).filter(Boolean),
    ),
  ];
  return sourceOrganizationIds.filter((sourceOrganizationId) => {
    if (!organizationMappings[sourceOrganizationId]) return false;
    const organizationLocations = locations.filter(
      (location) => location.organizationId === sourceOrganizationId,
    );
    return (
      organizationLocations.length > 0 &&
      organizationLocations.every(
        (location) =>
          locationMappings[location.sourceLocationKey] &&
          (location.mappingStatus !== "SOURCE_LOCATION_AMBIGUOUS" ||
            resolvedLocations[location.sourceLocationKey]),
      )
    );
  });
}
