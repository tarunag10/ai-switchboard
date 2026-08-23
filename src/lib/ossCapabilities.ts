export interface OssProviderCapability {
  id: string;
  label: string;
  modelFamilies: string[];
  contextLimit: number;
  authSource: "none" | "keychain" | "environment" | "manual";
}

export interface OssToolCapability {
  id: string;
  label: string;
  providerId: string;
  capabilities: string[];
  requiresApproval: boolean;
  writesEnabled: false;
}

export interface OssCapabilityRegistry {
  schemaVersion: number;
  registryMode: "metadata_only";
  writesEnabled: false;
  approvalMode: "fail_closed";
  providers: OssProviderCapability[];
  tools: OssToolCapability[];
}
