export interface ProxySessionAuthStatus {
  available: boolean;
  enforce: boolean;
  fingerprint: string;
  status: string;
  detail: string;
  validatedRequestCount: number;
  rejectedRequestCount: number;
}

export function describeProxySessionAuthStatus(
  status: ProxySessionAuthStatus | null | undefined,
): { label: string; detail: string; tone: "success" | "warning" | "neutral" } {
  if (!status) {
    return {
      label: "Unknown",
      detail: "Proxy session auth has not been checked yet.",
      tone: "neutral",
    };
  }

  switch (status.status) {
    case "session_token_enforced":
      return {
        label: "Enforced",
        detail: `${status.detail} Fingerprint ${status.fingerprint}.`,
        tone: "success",
      };
    case "session_token_available":
      return {
        label: "Available",
        detail: `${status.detail} Enable enforce mode in Settings to require the header on loopback proxy traffic.`,
        tone: "warning",
      };
    case "loopback_validated_unauthenticated":
      return {
        label: "Advisory",
        detail: status.detail,
        tone: "warning",
      };
    default:
      return {
        label: status.status,
        detail: status.detail,
        tone: "neutral",
      };
  }
}
