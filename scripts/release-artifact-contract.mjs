export function selectReleaseArtifact(
  candidates,
  { expectedVersion = null } = {},
) {
  const validCandidates = (Array.isArray(candidates) ? candidates : [])
    .filter((candidate) =>
      candidate &&
      typeof candidate.path === "string" &&
      candidate.path.length > 0 &&
      typeof candidate.name === "string" &&
      candidate.name.toLowerCase().includes("switchboard") &&
      candidate.name.endsWith(".dmg") &&
      Number.isFinite(candidate.mtimeMs),
    );

  if (validCandidates.length === 0) {
    return { candidate: null, reason: "no DMG candidates found" };
  }

  const versionCandidates = expectedVersion
    ? validCandidates.filter((candidate) => candidate.name.includes(expectedVersion))
    : validCandidates;
  if (expectedVersion && versionCandidates.length === 0) {
    return {
      candidate: null,
      reason: `no DMG candidate matches expected version ${expectedVersion}`,
    };
  }

  const ordered = [...versionCandidates].sort(
    (left, right) => right.mtimeMs - left.mtimeMs || left.name.localeCompare(right.name),
  );
  const newest = ordered[0];
  const tied = ordered.filter((candidate) => candidate.mtimeMs === newest.mtimeMs);
  if (tied.length > 1) {
    return {
      candidate: null,
      reason: "multiple DMG candidates share the newest modification time",
    };
  }
  return { candidate: newest, reason: null };
}
