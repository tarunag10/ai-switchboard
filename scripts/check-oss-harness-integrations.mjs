import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const required = [
  "THIRD_PARTY_NOTICES.md",
  "third_party/oss-integrations.json",
  "scripts/check-self-contained-oss-inventory.mjs",
  "scripts/check-self-contained-oss-inventory.node-test.mjs",
  "docs/integrations/deepseek-harness-maturity-audit.md",
  "docs/integrations/switchyard-evaluation.md",
  "docs/integrations/jcode-evaluation.md",
  "docs/integrations/oss-harness-integration-plan.md",
  "src-tauri/src/deepseek_harness.rs",
  "src-tauri/src/dsh_plugin_maturity.rs",
  "src-tauri/src/dsh_context_prototype.rs",
  "scripts/oss-harness-strategies.mjs",
  "scripts/oss-harness-strategies.node-test.mjs",
  "scripts/oss-session-events.mjs",
  "scripts/oss-session-events.node-test.mjs",
  "scripts/oss-provider-registry.mjs",
  "scripts/oss-provider-registry.node-test.mjs",
  "src-tauri/src/oss_capabilities.rs",
  "src-tauri/src/workbench_kernel/mod.rs",
  "src/lib/ossCapabilities.ts",
  "src/lib/workbench.ts",
  "src/lib/workbench.test.ts",
  "src/components/AddonsView.integration.test.tsx",
  "src/components/WorkbenchView.test.tsx",
];
const failures = [];
for (const relative of required) {
  if (!fs.existsSync(path.join(root, relative))) failures.push(`missing ${relative}`);
}
const tauriBundle = fs.readFileSync(path.join(root, "src-tauri/tauri.conf.json"), "utf8");
for (const resource of [
  "../LICENSE",
  "../NOTICE",
  "../THIRD_PARTY_NOTICES.md",
  "../third_party/ponytail/",
  "../third_party/oss-integrations.json",
]) {
  if (!tauriBundle.includes(resource)) failures.push(`Tauri bundle missing ${resource}`);
}
const plan = fs.readFileSync(path.join(root, "docs/integrations/oss-harness-integration-plan.md"), "utf8").toLowerCase();
for (const phrase of ["redacted", "observe-only", "fail-closed", "licenses", "replay"]) {
  if (!plan.includes(phrase)) failures.push(`plan missing boundary: ${phrase}`);
}
const maturity = fs.readFileSync(path.join(root, "docs/integrations/deepseek-harness-maturity-audit.md"), "utf8");
if (!maturity.includes("Experimental / Developer Preview")) failures.push("DeepSeek audit must remain experimental");
const switchyard = fs.readFileSync(path.join(root, "docs/integrations/switchyard-evaluation.md"), "utf8");
if (!switchyard.includes("not added as a mandatory runtime or embedded")) failures.push("Switchyard must remain optional");
if (failures.length) {
  console.error(`OSS harness integration check failed:\n- ${failures.join("\n- ")}`);
  process.exit(1);
}
console.log(JSON.stringify({ ok: true, requiredFiles: required.length, automaticPromotion: "disabled" }, null, 2));
