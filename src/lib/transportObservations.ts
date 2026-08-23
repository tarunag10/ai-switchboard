import { invoke } from "@tauri-apps/api/core";

export type TransportRoute = "ingress" | "headroom" | "direct_anthropic" | "direct_openai" | "cache";
export type TransportOutcome =
  | "success"
  | "upstream_http_error"
  | "connect_failure"
  | "write_failure"
  | "read_failure"
  | "timeout"
  | "client_disconnect"
  | "local_rejection";

export interface TransportObservation {
  eventId: string;
  startedAtMs: number;
  completedAtMs: number | null;
  route: TransportRoute;
  requestClass: string;
  streaming: boolean;
  statusCode: number | null;
  terminalOutcome: TransportOutcome | null;
}

export async function loadTransportObservations(): Promise<TransportObservation[]> {
  return invoke<TransportObservation[]>("get_transport_observations");
}
