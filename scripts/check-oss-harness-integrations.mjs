import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const required = [
  "docs/integrations/deepseek-harness-maturity-audit.md",
  "docs/integrations/switchyard-evaluation.md",
  "docs/integrations/jcode-evaluation.md",
  "docs/integrations/oss-harness-integration-plan.md",
  "src-tauri/src/deepseek_harness.rs",
  "src-tauri/src/dsh_plugin_maturity.rs",
  "src-tauri/src/dsh_context_prototype.rs",
  "scripts/oss-harness-strategies.mjs",
  "scripts/oss-harness-strategies.node-test.mjs",
];
const failures = [];
for (const relative of required) {
  if (!fs.existsSync(path.join(root, relative))) failures.push(`missing ${relative}`);
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
