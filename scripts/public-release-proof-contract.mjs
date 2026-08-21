export function normalizeSha256(value) {
  if (typeof value !== "string") return null;
  const digest = value.trim().replace(/^sha256:/i, "").toLowerCase();
  return /^[0-9a-f]{64}$/.test(digest) ? digest : null;
}

export function verifyChecksumText(checksumText, expectedDigest, expectedFilename = null) {
  const expected = normalizeSha256(expectedDigest);
  if (!expected) return { ok: false, reason: "expected DMG SHA-256 is missing or malformed" };
  if (typeof checksumText !== "string" || checksumText.trim() === "") {
    return { ok: false, reason: "checksum asset is empty" };
  }
  const rows = checksumText.split(/\r?\n/).map((line) =>
    line.trim().match(/^([0-9a-fA-F]{64})\s+[* ]?(.+?)\s*$/),
  ).filter(Boolean);
  const row = expectedFilename
    ? rows.find((candidate) => candidate[2].endsWith(expectedFilename))
    : rows[0];
  if (!row) return { ok: false, reason: "checksum asset contains no matching DMG entry" };
  const digest = row[1].toLowerCase();
  if (digest !== expected) return { ok: false, reason: "checksum asset does not match the signed DMG digest", digest, filename: row[2] };
  return { ok: true, digest, filename: row[2] };
}

export function validateChecksumAssetEvidence({ state, url, verification } = {}) {
  if (state !== "uploaded") return { ok: false, reason: "checksum asset is not uploaded" };
  if (typeof url !== "string" || !url.startsWith("https://")) return { ok: false, reason: "checksum asset URL is missing or not HTTPS" };
  if (!verification?.ok || !normalizeSha256(verification.digest)) return { ok: false, reason: verification?.reason ?? "checksum content is unverified" };
  return { ok: true, digest: normalizeSha256(verification.digest), filename: verification.filename ?? null };
}
