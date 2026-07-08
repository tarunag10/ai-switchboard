export const SALES_CONTACT_URL =
  (import.meta.env.VITE_HEADROOM_SALES_CONTACT_URL ?? "").trim() ||
  "mailto:hello@example.com";

export const CONTACT_FORM_URL = (
  import.meta.env.VITE_HEADROOM_CONTACT_FORM_URL ?? ""
).trim();

export const SUPPORT_ISSUES_URL =
  "https://github.com/tarunag10/mac-ai-switchboard/issues";
