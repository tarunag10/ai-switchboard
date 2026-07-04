import {
  privacyNoticeParagraphs,
  privacyNoticeTitle,
  termsOfUseParagraphs,
  termsOfUseTitle,
} from "../lib/legalText";
import {
  legalResourceParagraphs,
  legalResourcesTitle,
} from "../lib/legalResources";

export interface SettingsLegalPanelProps {
  requiredTermsVersion: number;
}

export function SettingsLegalPanel({
  requiredTermsVersion,
}: SettingsLegalPanelProps) {
  return (
    <>
      <article
        className="soft-card panel-card settings-legal-card"
        aria-labelledby="settings-legal-title"
      >
        <div className="panel-card__header">
          <div>
            <span className="panel-eyebrow">Legal</span>
            <h2 id="settings-legal-title">{termsOfUseTitle}</h2>
          </div>
        </div>
        <div className="settings-legal-copy">
          {termsOfUseParagraphs.map((paragraph) => (
            <p key={paragraph}>{paragraph}</p>
          ))}
        </div>
        <p className="settings-account-notice">
          Terms version {requiredTermsVersion}. The required version is bumped
          when bundled Terms or Privacy wording changes materially, so users are
          prompted to review the new text.
        </p>
      </article>

      <article
        className="soft-card panel-card settings-legal-card"
        aria-labelledby="settings-privacy-title"
      >
        <div className="panel-card__header">
          <div>
            <span className="panel-eyebrow">Privacy and network</span>
            <h2 id="settings-privacy-title">{privacyNoticeTitle}</h2>
          </div>
        </div>
        <div className="settings-legal-copy">
          {privacyNoticeParagraphs.map((paragraph) => (
            <p key={paragraph}>{paragraph}</p>
          ))}
        </div>
      </article>

      <article
        className="soft-card panel-card settings-legal-card"
        aria-labelledby="settings-resources-title"
      >
        <div className="panel-card__header">
          <div>
            <span className="panel-eyebrow">License and provenance</span>
            <h2 id="settings-resources-title">{legalResourcesTitle}</h2>
          </div>
        </div>
        <div className="settings-legal-copy">
          {legalResourceParagraphs.map((paragraph) => (
            <p key={paragraph}>{paragraph}</p>
          ))}
        </div>
      </article>
    </>
  );
}
