const RELEASE_ARTIFACT_NAME = /^(?:AI Switchboard|AI Switchboard for Mac|Mac AI Switchboard|Mac-AI-Switchboard)[ _-]\d+\.\d+\.\d+(?:[-_][^.]+)?\.dmg$/i;

function versionToken(name) {
  const match = name.match(/^[^0-9]*(\d+\.\d+\.\d+)(?=[_-]|\.dmg$)/i);
  return match?.[1] ?? null;
}

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
      RELEASE_ARTIFACT_NAME.test(candidate.name) &&
      candidate.regularFile === true &&
      Number.isFinite(candidate.mtimeMs),
    );

  if (validCandidates.length === 0) {
    return { candidate: null, reason: "no DMG candidates found" };
  }

  const versionCandidates = expectedVersion
    ? validCandidates.filter(
        (candidate) => versionToken(candidate.name) === expectedVersion,
      )
    : validCandidates;
  if (expectedVersion && versionCandidates.length === 0) {
    return {
      candidate: null,
      reason: `no DMG candidate matches expected version ${expectedVersion}`,
    };
  }

  if (expectedVersion && versionCandidates.length > 1) {
    return {
      candidate: null,
      reason: `multiple DMG candidates match expected version ${expectedVersion}`,
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
