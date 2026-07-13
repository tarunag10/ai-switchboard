export type ToolStatus =
  "not_installed" | "installing" | "healthy" | "degraded";

export interface ManagedTool {
  id: string;
  name: string;
  description: string;
  runtime: "python" | "binary" | "plugin";
  required: boolean;
  enabled: boolean;
  status: ToolStatus;
  sourceUrl: string;
  version: string;
  checksum?: string | null;
  metadata?: Record<string, unknown> | null;
}

export interface PipelineStageMetric {
  stageId: string;
  stageName: string;
  applied: boolean;
  estimatedTokensSaved: number;
  addedLatencyMs: number;
  notes: string[];
}

export interface UsageEvent {
  id: string;
  timestamp: string;
  client: string;
  workspace: string;
  upstreamTarget: string;
  stages: PipelineStageMetric[];
  estimatedInputTokens: number;
  estimatedOutputTokens: number;
  estimatedCostSavingsUsd: number;
  latencyMs: number;
  outcome: "success" | "bypassed" | "error";
}

export interface DailyInsight {
  id: string;
  category: "savings" | "workflow" | "health";
  severity: "info" | "warning" | "critical";
  title: string;
  recommendation: string;
  evidence: string;
  relatedWorkspace?: string | null;
}

export interface ClientStatus {
  id: string;
  name: string;
  installed: boolean;
  configured: boolean;
  health: "healthy" | "attention" | "not_detected";
  notes: string[];
}

export type LaunchExperience = "first_run" | "resume" | "dashboard";

export interface DailySavingsPoint {
  date: string;
  estimatedSavingsUsd: number;
  estimatedTokensSaved: number;
  actualCostUsd: number;
  totalTokensSent: number;
}

// Counterfactual output-token reduction from the proxy's output shaper.
// `method` is "estimated" (synthetic control vs a learned baseline) or
// "measured" (A/B holdout); the percentage carries a 95% confidence band.
export interface OutputReduction {
  method: string;
  reductionPercent: number;
  ciLowPercent: number;
  ciHighPercent: number;
  requests: number;
}

export interface ProviderSavingsPoint {
  provider: string;
  estimatedSavingsUsd: number;
  estimatedTokensSaved: number;
  actualCostUsd: number;
  totalTokensSent: number;
}

export interface HourlySavingsPoint {
  hour: string;
  estimatedSavingsUsd: number;
  estimatedTokensSaved: number;
  actualCostUsd: number;
  totalTokensSent: number;
  byProvider: ProviderSavingsPoint[];
}

export interface SavingsAttributionEvent {
  schemaVersion: number;
  id: string;
  observedAt: string;
  scope: "session";
  source:
    | "headroom_engine"
    | "rtk"
    | "repo_intelligence"
    | "caveman"
    | "ponytail"
    | "markitdown"
    | "compact_chinese";
  confidence: "measured" | "estimated" | "inferred";
  deltaTokensSaved: number;
  deltaUsd: number;
  totalTokensSent: number;
  requestDelta: number;
  evidence: string[];
}

export interface SavingsAttributionCounter {
  source: SavingsAttributionEvent["source"];
  scope: SavingsAttributionEvent["scope"];
  eventCount: number;
  runtimeEventCount: number;
  measuredEventCount: number;
  estimatedEventCount: number;
  inferredEventCount: number;
  deltaTokensSaved: number;
  totalTokensSent: number;
  lastSeenAt: string | null;
}

export interface MeasuredSavingsAttributionRequest {
  source: SavingsAttributionEvent["source"];
  label: string;
  baselineTokens: number;
  optimizedTokens: number;
  requestDelta?: number;
  detail?: string;
}

export interface DashboardState {
  appVersion: string;
  launchExperience: LaunchExperience;
  bootstrapComplete: boolean;
  pythonRuntimeInstalled: boolean;
  lifetimeRequests: number;
  lifetimeEstimatedSavingsUsd: number;
  lifetimeEstimatedTokensSaved: number;
  sessionRequests: number;
  sessionEstimatedSavingsUsd: number;
  sessionEstimatedTokensSaved: number;
  sessionSavingsPct: number;
  outputReduction: OutputReduction | null;
  dailySavings: DailySavingsPoint[];
  hourlySavings: HourlySavingsPoint[];
  savingsHistoryLoaded: boolean;
  tools: ManagedTool[];
  clients: ClientStatus[];
  recentUsage: UsageEvent[];
  insights: DailyInsight[];
  requiredTermsVersion: number;
  acceptedTermsVersion: number;
  termsUrl: string;
}

export interface BootstrapProgress {
  running: boolean;
  complete: boolean;
  failed: boolean;
  currentStep: string;
  message: string;
  currentStepEtaSeconds: number;
  overallPercent: number;
}

export interface ResearchCandidate {
  name: string;
  category: string;
  repository: string;
  runtime: string;
  license: string;
  localOnlyFit: string;
  installMethod: string;
  maintenance: string;
  decision: "include" | "defer" | "research";
  notes: string;
}

export interface ClientSetupResult {
  clientId: string;
  applied: boolean;
  alreadyConfigured: boolean;
  summary: string;
  changedFiles: string[];
  backupFiles: string[];
  nextSteps: string[];
  verification: ClientSetupVerification;
}

export interface ClientSetupVerification {
  clientId: string;
  verified: boolean;
  proxyReachable: boolean;
  checks: string[];
  failures: string[];
}

export interface ClientConnectorStatus {
  clientId: string;
  name: string;
  supportStatus?: "managed" | "planned";
  setupPhase?: "managed" | "detect" | "guide" | "adapt";
  setupHint?: string;
  category?: string;
  detectionSources?: string[];
  detectionEvidence?: string[];
  configLocations?: string[];
  automationGates?: string[];
  manualWorkflow?: string[];
  configCreationSteps?: string[];
  configCreationStepDetails?: ClientConnectorConfigCreationStep[];
  configDryRunPreview?: ClientConnectorConfigDryRunPreview | null;
  automationPath?: ClientConnectorAutomationStage[];
  installed: boolean;
  enabled: boolean;
  verified: boolean;
  setupVerification?: ClientSetupVerification | null;
  lastConfiguredAt?: string | null;
}

export interface ClientConnectorConfigCreationStep {
  id: string;
  label: string;
  detail: string;
  requiredEvidence?: string[];
}

export interface ClientConnectorConfigDryRunPreview {
  target: string;
  marker: string;
  backupPath: string;
  currentState: string;
  proposedState: string;
  applyBlockedReason: string;
  rollbackPreview: string;
  confirmationPhrase: string;
  writes: string[];
}

export interface ClientConnectorAutomationStage {
  id: string;
  label: string;
  status: "ready" | "blocked" | string;
  evidence: string;
}

export interface ManagedRollbackPreview {
  recordId: string;
  owner: string;
  targetPath: string;
  marker: string;
  backupPath: string | null;
  markerPresent: boolean;
  backupExists: boolean;
  status: "ready" | "blocked";
  confirmationPhrase: string;
  proposedAction: string;
  blockedReason: string | null;
  evidence: string[];
}

export interface ManagedRollbackExecutionResult {
  recordId: string;
  owner: string;
  targetPath: string;
  restoredFrom: string;
  safetyBackupPath: string | null;
  marker: string;
  verification: string[];
}

export interface ManagedConfigApplyPreview {
  recordId: string;
  owner: string;
  targetPath: string;
  marker: string;
  backupPath: string;
  status: "ready" | "blocked";
  confirmationPhrase: string;
  currentState: string;
  proposedState: string;
  rollbackPreview: string;
  blockedReason: string | null;
  evidence: string[];
}

export interface ManagedConfigApplyResult {
  recordId: string;
  owner: string;
  targetPath: string;
  changed: boolean;
  backupPath: string | null;
  marker: string;
  verification: string[];
}

export interface ManagedFootprintItem {
  id: string;
  category: string;
  path: string;
  exists: boolean;
  managed: boolean;
  action: string;
  reversible: boolean;
  backupPaths: string[];
  notes: string[];
}

export interface ManagedFootprintReport {
  generatedAt: string;
  items: ManagedFootprintItem[];
}

export interface UninstallTarget {
  id: string;
  category: string;
  path: string;
  exists: boolean;
  managed: boolean;
  action: string;
  requiresConfirmation: boolean;
  notes: string[];
}

export interface UninstallDryRunReport {
  generatedAt: string;
  targets: UninstallTarget[];
  removedOnUninstall: string[];
  preserved: string[];
}

export type CodexThreadRetaggingMode = "ask" | "enabled" | "disabled";

export interface CodexThreadRetaggingSettings {
  codexThreadRetagging: CodexThreadRetaggingMode;
}

export interface CodexThreadRetaggingReport {
  path: string;
  fromProvider: string;
  toProvider: string;
  rowsChanged: number;
  backupPath?: string | null;
  skippedReason?: string | null;
}

export interface CodexThreadRetaggingRunReport {
  mode: CodexThreadRetaggingMode;
  reports: CodexThreadRetaggingReport[];
}

export interface CodexDbRestoreResult {
  restoredPath: string;
  backupPath: string;
}

export interface ManagedRollbackUndoAllPreview {
  status: "ready" | "blocked";
  confirmationPhrase: string;
  ready: ManagedRollbackPreview[];
  blocked: ManagedRollbackPreview[];
  evidence: string[];
}

export interface ManagedRollbackUndoAllExecutionResult {
  confirmationPhrase: string;
  executed: ManagedRollbackExecutionResult[];
  blocked: ManagedRollbackPreview[];
  verification: string[];
}

export interface ReleaseEvidenceCommandResult {
  commandId: string;
  label: string;
  command: string;
  summaryPath: string | null;
  stdout: string;
  stderr: string;
}

export interface RuntimeStatus {
  platform: string;
  supportTier: string;
  installed: boolean;
  running: boolean;
  starting: boolean;
  paused: boolean;
  /** True when the watchdog auto-paused after giving up on a wedged proxy,
   *  distinct from a deliberate user pause. Drives the "stopped unexpectedly"
   *  banner + Resume button. */
  autoPaused: boolean;
  proxyReachable: boolean;
  proxyBindAddress?: string | null;
  proxyAuthStatus?: string | null;
  proxyAuthDetail?: string | null;
  headroomPid?: number | null;
  launchAgentStatus?: {
    installed: boolean;
    path?: string | null;
    label: string;
    loaded?: boolean | null;
    loadDetail?: string | null;
    legacyInstalled: boolean;
    legacyPath?: string | null;
    legacyLabel: string;
    legacyLoaded?: boolean | null;
    legacyLoadDetail?: string | null;
  } | null;
  backendStatus?: {
    reachable: boolean;
    bindAddress: string;
    port: number;
    defaultPort: number;
    fallbackRangeStart: number;
    fallbackRangeEnd: number;
  } | null;
  mcpConfigured?: boolean | null;
  mcpError?: string | null;
  repoMemoryMcpConfigured?: boolean | null;
  repoMemoryMcpError?: string | null;
  repoMemoryMcpActive?: boolean | null;
  repoMemoryMcpLastStartedAt?: string | null;
  repoMemoryMcpLastCheckedAt?: string | null;
  repoMemoryMcpSupervisionStatus?: string | null;
  repoMemoryMcpService?: {
    managedByApp: boolean;
    readOnly: boolean;
    transport: string;
    command: string;
    descriptorPath: string;
    descriptorPresent: boolean;
    scriptPath: string;
    scriptPresent: boolean;
    nodeAvailable: boolean;
  } | null;
  mlInstalled?: boolean | null;
  kompressEnabled?: boolean | null;
  headroomLearnSupported: boolean;
  headroomLearnDisabledReason?: string | null;
  startupError?: string | null;
  startupErrorHint?: string | null;
  runtimeUpgradeFailure?: RuntimeUpgradeFailure | null;
  rtk: {
    installed: boolean;
    enabled: boolean;
    version?: string | null;
    pathConfigured: boolean;
    hookConfigured: boolean;
    totalCommands?: number | null;
    totalInput?: number | null;
    totalOutput?: number | null;
    totalSaved?: number | null;
    avgSavingsPct?: number | null;
    totalTimeMs?: number | null;
    avgTimeMs?: number | null;
    daily?: RtkDailyStats[];
    commandFamilies?: RtkCommandFamilyStats[];
  };
}

export type SwitchboardMode = "off" | "rtk" | "headroom" | "full";
export type SavingsMode = "balanced" | "aggressive";

export interface SwitchboardState {
  mode: SwitchboardMode;
  desiredMode?: SwitchboardMode;
  effectiveMode?: SwitchboardMode;
  savingsMode: SavingsMode;
  needsAttention?: boolean;
  localOnly: boolean;
  remoteServicesEnabled: boolean;
  runtime: RuntimeStatus;
  clients: ClientConnectorStatus[];
  enabledClients: ClientConnectorStatus[];
  rtkEnabled: boolean;
  headroomEnabled: boolean;
  summary: string;
}

export type DoctorSeverity = "ok" | "warning" | "error";

export interface DoctorIssue {
  id: string;
  title: string;
  body: string;
  severity: DoctorSeverity;
  repairAction?: string | null;
}

export interface DoctorReport {
  status: DoctorSeverity;
  summary: string;
  issues: DoctorIssue[];
}

export interface RuntimeUpgradeProgress {
  running: boolean;
  complete: boolean;
  failed: boolean;
  currentStep: string;
  message: string;
  overallPercent: number;
  fromVersion?: string | null;
  toVersion?: string | null;
}

export type UpgradeFailurePhase = "install" | "boot_validation";

export interface RuntimeUpgradeFailure {
  appVersion: string;
  targetHeadroomVersion: string;
  fallbackHeadroomVersion?: string | null;
  failurePhase: UpgradeFailurePhase;
  attempts: number;
  firstAttemptAt: string;
  lastAttemptAt: string;
  errorMessage: string;
  errorHint?: string | null;
  rollbackRestored: boolean;
}

export interface AppUpdateConfiguration {
  enabled: boolean;
  currentVersion: string;
  endpointCount: number;
  configurationError?: string | null;
  betaChannelEnabled: boolean;
}

export interface AvailableAppUpdate {
  currentVersion: string;
  version: string;
  publishedAt?: string | null;
  notes?: string | null;
}

export interface ClaudeCodeProject {
  id: string;
  projectPath: string;
  displayName: string;
  lastWorkedAt: string;
  sessionCount: number;
  lastLearnRanAt: string | null;
  hasPersistedLearnings: boolean;
  activeDaysSinceLastLearn: number;
  lastLearnPatternCount: number | null;
}

export interface HeadroomLearnStatus {
  running: boolean;
  projectPath?: string | null;
  projectDisplayName?: string | null;
  startedAt?: string | null;
  finishedAt?: string | null;
  elapsedSeconds?: number | null;
  progressPercent: number;
  summary: string;
  success?: boolean | null;
  error?: string | null;
  lastRunAt?: string | null;
  outputTail: string[];
}

export interface HeadroomLearnPrereqStatus {
  claudeCliAvailable: boolean;
  claudeCliPath?: string | null;
  codexCliAvailable: boolean;
  codexCliPath?: string | null;
  codexLoggedIn: boolean;
}

// A single entry in `requestMessages`. Intentionally loose — the proxy passes
// through whatever shape the upstream provider uses (Anthropic: `content` is a
// string or structured blocks list; OpenAI: string-only). The UI extracts
// displayable text in `ActivityFeed.tsx`.
export interface TransformationRequestMessage {
  role?: string;
  content?:
    string | Array<{ type?: string; text?: string; [k: string]: unknown }>;
  [k: string]: unknown;
}

export interface TransformationFeedEvent {
  requestId?: string | null;
  timestamp?: string | null;
  provider?: string | null;
  model?: string | null;
  inputTokensOriginal?: number | null;
  inputTokensOptimized?: number | null;
  tokensSaved?: number | null;
  savingsPercent?: number | null;
  transformsApplied: string[];
  workspace?: string | null;
  turnId?: string | null;
  // Populated only when the proxy was started with `--log-messages` (or
  // `HEADROOM_LOG_MESSAGES=1`), reflected in
  // `TransformationFeedResponse.logFullMessages`. Both fields are
  // pass-through from the proxy's `RequestLogger` — the desktop renders
  // them, it does not reinterpret them.
  //
  // `compressedMessages` is the post-compression message list that was
  // actually sent upstream; paired with `requestMessages` it lets consumers
  // see what Headroom's pipeline stripped, replaced, or kept. Absent on
  // proxies that predate the field.
  requestMessages?: TransformationRequestMessage[] | null;
  compressedMessages?: TransformationRequestMessage[] | null;
}

export interface TransformationFeedResponse {
  logFullMessages: boolean;
  fullMessageLoggingExpiresAt?: string | null;
  messageLogRetentionHours: number;
  proxyReachable: boolean;
  transformations: TransformationFeedEvent[];
}

export interface MessageLoggingSettings {
  fullMessageLogging: boolean;
  fullMessageLoggingExpiresAt: string | null;
  messageLogRetentionHours: number;
}

export interface PurgeResult {
  purged: boolean;
  removedPaths: string[];
  notes: string[];
}

export interface LiveLearning {
  id: string;
  content: string;
  category: string;
  importance: number;
  evidenceCount: number;
  createdAt: string;
}

export interface AppliedSection {
  title: string;
  bullets: string[];
}

export interface AppliedPatterns {
  claudeMd: AppliedSection[];
  memoryMd: AppliedSection[];
}

export interface RtkTodayStats {
  date: string;
  savedTokens: number;
  commands: number;
  inputTokens?: number;
  outputTokens?: number;
  savingsPct?: number | null;
  totalTimeMs?: number;
  avgTimeMs?: number | null;
}

export interface RtkDailyStats {
  date: string;
  savedTokens: number;
  commands: number;
  inputTokens?: number;
  outputTokens?: number;
  savingsPct?: number | null;
  totalTimeMs?: number;
  avgTimeMs?: number | null;
}

export interface RtkCommandFamilyStats {
  family: string;
  commands: number;
  inputTokens: number;
  outputTokens: number;
  savedTokens: number;
  savingsPct?: number | null;
  totalTimeMs: number;
  avgTimeMs?: number | null;
  lastObservedAt?: string | null;
}

export type RecordTag = "daily" | "weekly" | "allTime";

export interface RecordEvent {
  observedAt: string;
  tags: RecordTag[];
  tokensSaved: number;
  savingsPercent: number | null;
  model: string | null;
  provider: string | null;
  requestId: string | null;
  previousRecord: number | null;
  day: string | null;
  workspace?: string | null;
  inputTokensOriginal?: number | null;
  inputTokensOptimized?: number | null;
  // Carried forward from the record-setting transformation so the record row
  // can surface the same request/compressed detail as the compression card.
  // Populated only when the proxy's `log_full_messages` is enabled;
  // `compressedMessages` additionally requires a proxy that carries the
  // field (see TransformationFeedEvent above).
  requestMessages?: TransformationRequestMessage[] | null;
  compressedMessages?: TransformationRequestMessage[] | null;
}

export interface WeeklyRecapEvent {
  observedAt: string;
  weekStart: string;
  weekEnd: string;
  totalTokensSaved: number;
  totalSavingsUsd: number;
  activeDays: number;
}

export interface LearningsMilestoneEvent {
  observedAt: string;
  patternsToday: number;
  remindersToday: number;
  learningsToday: number;
  projectPath: string | null;
  projectDisplayName: string | null;
}

export interface TrainSuggestionEvent {
  observedAt: string;
  projectPath: string;
  projectDisplayName: string;
  sessionCount: number;
  activeDaysSinceLastLearn: number;
  // "never_trained" | "stale"
  kind: string;
}

export interface ActivityFeedSnapshot {
  transformation: TransformationFeedEvent | null;
  record: RecordEvent | null;
  rtkToday: RtkTodayStats | null;
  learningsMilestone: LearningsMilestoneEvent | null;
  weeklyRecap: WeeklyRecapEvent | null;
  trainSuggestion: TrainSuggestionEvent | null;
}

export interface ActivityFeedResponse {
  tiles: ActivityFeedSnapshot;
  proxyReachable: boolean;
}

export type ClaudeAuthMethod = "claude_ai_oauth" | "api_key" | "unknown";

export type ClaudePlanTier = "free" | "pro" | "max5x" | "max20x" | "unknown";

export type HeadroomSubscriptionTier = "pro" | "max5x" | "max20x";

export type BillingPeriod = "annual" | "monthly";

export type PricingGateReason =
  | "sign_in_required"
  | "weekly_usage_limit_reached"
  | "codex_weekly_usage_limit_reached";

export interface ClaudeAccountProfile {
  authMethod: ClaudeAuthMethod;
  email?: string | null;
  displayName?: string | null;
  accountUuid?: string | null;
  organizationUuid?: string | null;
  billingType?: string | null;
  accountCreatedAt?: string | null;
  subscriptionCreatedAt?: string | null;
  hasExtraUsageEnabled: boolean;
  planTier: ClaudePlanTier;
  planDetectionSource?: string | null;
  weeklyUtilizationPct?: number | null;
  fiveHourUtilizationPct?: number | null;
  extraUsageMonthlyLimit?: number | null;
  profileFetchError?: string | null;
}

export interface CodexUsageWindow {
  usedPercent: number;
  windowLabel?: string | null;
  windowMinutes?: number | null;
  secondsUntilReset?: number | null;
}

export interface CodexUsage {
  limitName?: string | null;
  primary?: CodexUsageWindow | null;
  secondary?: CodexUsageWindow | null;
  creditsBalance?: string | null;
  creditsUnlimited: boolean;
  optimizationAllowed: boolean;
  shouldNudge: boolean;
  nudgeLevel: number;
  gateReason?: PricingGateReason | null;
  recommendedSubscriptionTier?: HeadroomSubscriptionTier | null;
  weeklyUsedPercent?: number | null;
  gateMessage: string;
}

export interface HeadroomAccountProfile {
  email: string;
  trialStartedAt?: string | null;
  trialEndsAt?: string | null;
  trialActive: boolean;
  subscriptionActive: boolean;
  subscriptionTier?: HeadroomSubscriptionTier | null;
  subscriptionStartedAt?: string | null;
  subscriptionRenewsAt?: string | null;
  subscriptionAmountCents?: number | null;
  subscriptionBillingPeriod?: string | null;
  subscriptionDiscountDuration?: string | null;
  subscriptionDiscountDurationInMonths?: number | null;
  subscriptionCancelAtPeriodEnd?: boolean;
  subscriptionEndsAt?: string | null;
  inviteCode?: string | null;
  acceptedInvitesCount: number;
  inviteBonusPercent: number;
}

export interface HeadroomPricingStatus {
  authenticated: boolean;
  localGraceStartedAt: string;
  localGraceEndsAt: string;
  localGraceActive: boolean;
  accountSyncError?: string | null;
  needsAuthentication: boolean;
  optimizationAllowed: boolean;
  shouldNudge: boolean;
  nudgeLevel: number;
  gateReason?: PricingGateReason | null;
  gateMessage: string;
  nudgeThresholdPercent?: number | null;
  effectiveNudgeThresholdsPercent?: number[] | null;
  disableThresholdPercent?: number | null;
  effectiveDisableThresholdPercent?: number | null;
  recommendedSubscriptionTier?: HeadroomSubscriptionTier | null;
  tierMismatch?: TierMismatch | null;
  claude: ClaudeAccountProfile;
  codex?: CodexUsage | null;
  account?: HeadroomAccountProfile | null;
  launchDiscountActive: boolean;
  activePercentOff?: number;
  pricingCohorts?: PricingCohort[];
}

export interface PricingCohort {
  key: string;
  label: string;
  percentOff: number;
  capacity?: number | null;
  status: "sold_out" | "active" | "upcoming";
  spotsLeft?: number | null;
}

export type TierRecommendationSource = "claude" | "codex" | "both";

export interface TierMismatch {
  paidTier: HeadroomSubscriptionTier;
  recommendedTier: HeadroomSubscriptionTier;
  recommendedSource: TierRecommendationSource;
  graceEndsAt: string;
  clamped: boolean;
}

export interface HeadroomAuthCodeRequest {
  email: string;
  expiresInSeconds: number;
}
