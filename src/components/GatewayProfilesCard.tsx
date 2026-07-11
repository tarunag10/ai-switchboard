import { useId, useState } from "react";
import {
  gatewayProfileStatus,
  gatewayProfiles,
  type GatewayProfile,
} from "../lib/gatewayProfiles";

export function GatewayProfilesCard({
  onCopyGuidance,
}: {
  onCopyGuidance: (markdown: string, label: string) => void;
}) {
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
          Local-only guides for optional cache, trace, and gateway tooling. Nothing is installed, configured, or routed by Switchboard from this surface.
        </p>
        <div className="gateway-profiles-card__list">
          {gatewayProfiles.map((profile) => (
            <GatewayProfileRow
              key={profile.id}
              profile={profile}
              onCopyGuidance={onCopyGuidance}
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
}: {
  profile: GatewayProfile;
  onCopyGuidance: (markdown: string, label: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const detailsId = useId();
  const status = gatewayProfileStatus(profile);

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
      </div>
    </section>
  );
}
