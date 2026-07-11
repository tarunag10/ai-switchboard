import { useEffect, useId, useState } from "react";
import {
  emptyGatewayProfileLocalState,
  gatewayDoctorSummary,
  gatewayProfileConfigPreview,
  gatewayProfileStorageKey,
  gatewayProfileStatus,
  gatewayProfiles,
  parseGatewayProfileLocalState,
  type GatewayProfile,
  type GatewayProfileLocalState,
} from "../lib/gatewayProfiles";

export function GatewayProfilesCard({
  onCopyGuidance,
}: {
  onCopyGuidance: (markdown: string, label: string) => void;
}) {
  const [localState, setLocalState] = useState<GatewayProfileLocalState>(() =>
    typeof window === "undefined"
      ? emptyGatewayProfileLocalState()
      : parseGatewayProfileLocalState(window.localStorage.getItem(gatewayProfileStorageKey)),
  );
  useEffect(() => {
    window.localStorage.setItem(gatewayProfileStorageKey, JSON.stringify(localState));
  }, [localState]);

  return (
    <li className="addon-card addon-card--planned gateway-profiles-card">
      <div className="addon-card__body">
        <div className="addon-card__heading">
          <span className="addon-card__name">Gateway & observability profiles</span>
          <span className="addon-card__badge addon-card__badge--planned">
            Guided only
          </span>
        </div>
        <p className="addon-card__description">
          Local profile lifecycle for optional cache, trace, and gateway tooling. Enablement is a Switchboard-only record: nothing is installed, configured, or routed.
        </p>
        <div className="gateway-profiles-card__list">
          {gatewayProfiles.map((profile) => (
            <GatewayProfileRow
              key={profile.id}
              profile={profile}
              onCopyGuidance={onCopyGuidance}
              localState={localState}
              setLocalState={setLocalState}
            />
          ))}
        </div>
      </div>
    </li>
  );
}

function GatewayProfileRow({
  profile,
  onCopyGuidance,
  localState,
  setLocalState,
}: {
  profile: GatewayProfile;
  onCopyGuidance: (markdown: string, label: string) => void;
  localState: GatewayProfileLocalState;
  setLocalState: React.Dispatch<React.SetStateAction<GatewayProfileLocalState>>;
}) {
  const [open, setOpen] = useState(false);
  const detailsId = useId();
  const status = gatewayProfileStatus(profile);
  const lifecycle = localState.profiles[profile.id] ?? "disabled";
  const reviewed = localState.reviewedChecks[profile.id] ?? [];
  const receipt = (action: "enabled" | "disabled" | "evidence-reviewed", detail: string) => ({
    id: `${profile.id}-${Date.now()}-${action}`,
    profileId: profile.id,
    action,
    detail,
    createdAt: new Date().toISOString(),
  });
  const setLifecycle = (next: "enabled" | "disabled") => setLocalState((current) => ({
    ...current,
    profiles: { ...current.profiles, [profile.id]: next },
    receipts: [receipt(next, `Local ${next} record only; no traffic or configuration changed.`), ...current.receipts].slice(0, 30),
  }));
  const toggleReviewed = (label: string) => setLocalState((current) => {
    const existing = current.reviewedChecks[profile.id] ?? [];
    const next = existing.includes(label) ? existing.filter((item) => item !== label) : [...existing, label];
    return {
      ...current,
      reviewedChecks: { ...current.reviewedChecks, [profile.id]: next },
      receipts: [receipt("evidence-reviewed", `Local evidence review updated for ${label}; no live check was run.`), ...current.receipts].slice(0, 30),
    };
  });

  return (
    <section className="gateway-profile">
      <div className="gateway-profile__heading">
        <div>
          <strong>{profile.name}</strong>
          <span>{profile.category} · {profile.trafficBoundary} boundary</span>
        </div>
        <span className={`gateway-profile__state gateway-profile__state--${profile.state}`}>
          {status.label}
        </span>
      </div>
      <p>{status.detail}</p>
      <div className="gateway-profile__facts" aria-label={`${profile.name} privacy facts`}>
        <span>{profile.canSeePromptsAndOutputs ? "Can see prompts/outputs" : "No prompt/output visibility"}</span>
        <span>{profile.canModifyProviderRouting ? "Can alter routing manually" : "Does not alter routing"}</span>
        <span>{profile.needsSecrets ? "Secrets required outside repo" : "No Switchboard secret"}</span>
      </div>
      <div className="gateway-profile__actions">
        <button
          type="button"
          className="addon-card__action"
          disabled={profile.state === "gated"}
          onClick={() => setLifecycle(lifecycle === "enabled" ? "disabled" : "enabled")}
        >
          {lifecycle === "enabled" ? "Disable local profile" : "Enable local profile"}
        </button>
        <button
          type="button"
          className="addon-card__action"
          aria-controls={detailsId}
          aria-expanded={open}
          onClick={() => setOpen((value) => !value)}
        >
          {open ? "Hide evidence" : "View privacy & Doctor"}
        </button>
        <button
          type="button"
          className="addon-card__action addon-card__action--primary"
          onClick={() => onCopyGuidance(profile.setupGuidance, `${profile.name} setup and Doctor guide`)}
        >
          Copy setup & Doctor guide
        </button>
        <button
          type="button"
          className="addon-card__action"
          onClick={() => onCopyGuidance(gatewayProfileConfigPreview(profile), `${profile.name} config preview`)}
        >
          Copy config preview
        </button>
      </div>
      <div id={detailsId} hidden={!open} className="gateway-profile__details">
        <p><strong>Disclosure:</strong> {profile.disclosure}</p>
        <p><strong>Privacy caveat:</strong> {profile.privacyCaveat}</p>
        <div className="addon-card__evidence-grid">
          <section>
            <h4>Required evidence</h4>
            <ul>{profile.requiredEvidence.map((item) => <li key={item}>{item}</li>)}</ul>
          </section>
          <section>
            <h4>Doctor checks</h4>
            <ul>{profile.doctorChecks.map((check) => <li key={check.label}><strong>{check.label}:</strong> {check.evidence}</li>)}</ul>
          </section>
        </div>
        <p><strong>Rollback:</strong> {profile.rollbackGuidance}</p>
        <p><strong>Off mode:</strong> {profile.offModeGuidance}</p>
        <p><strong>Savings evidence:</strong> {profile.savingsEvidence}.</p>
        <p><strong>Local lifecycle:</strong> {lifecycle}. This is a local Switchboard receipt only; it never proves a service is running.</p>
        <p><strong>Doctor evidence:</strong> {gatewayDoctorSummary(profile, localState)}</p>
        <ul className="gateway-profile__checks">
          {profile.doctorChecks.map((check) => (
            <li key={check.label}>
              <label>
                <input type="checkbox" checked={reviewed.includes(check.label)} onChange={() => toggleReviewed(check.label)} />
                I reviewed: {check.label}
              </label>
              <span>{check.evidence}</span>
            </li>
          ))}
        </ul>
        {localState.receipts.filter((item) => item.profileId === profile.id).slice(0, 3).map((item) => (
          <p key={item.id} className="gateway-profile__receipt"><strong>Receipt:</strong> {item.action} · {new Date(item.createdAt).toLocaleString()} — {item.detail}</p>
        ))}
      </div>
    </section>
  );
}
