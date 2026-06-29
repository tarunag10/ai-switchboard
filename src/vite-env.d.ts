/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_CLARITY_PROJECT_ID?: string;
  readonly VITE_HEADROOM_BUILD_FLAVOR?: string;
  readonly VITE_HEADROOM_LOCAL_ONLY?: string;
  readonly VITE_HEADROOM_REMOTE_SERVICES?: string;
  readonly VITE_HEADROOM_REMOTE_TELEMETRY?: string;
  readonly VITE_HEADROOM_SALES_CONTACT_URL?: string;
  readonly VITE_HEADROOM_CONTACT_FORM_URL?: string;
  readonly VITE_SENTRY_DSN?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
