export interface ProxyVerificationRow {
  clientId: string;
  name: string;
  state: "processing" | "waiting" | "testing" | "verified";
  message: string;
  oneClickSupported: boolean;
}
