import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const rustKernelBin = path.join(
  workspaceRoot,
  "crates",
  "target",
  "debug",
  process.platform === "win32" ? "runx.exe" : "runx",
);

const commands = [
  ["pnpm", ["boundary:check"]],
  ["pnpm", ["typecheck"]],
  ["node", ["scripts/build-workspace.mjs"]],
  ["node", ["scripts/check-publishable-package-manifests.mjs"]],
  ["pnpm", ["rust:crate-graph"]],
  ["pnpm", ["rust:style"]],
  ["node", ["scripts/check-authoring-package-contract.mjs"]],
  ["node", ["scripts/check-create-skill-package-contract.mjs"]],
  ["pnpm", ["fixtures:kernel:validate"]],
  ["pnpm", ["fixtures:kernel:check"]],
  ["pnpm", ["fixtures:kernel:keys"]],
  ["pnpm", ["fixtures:contracts:check"]],
  ["pnpm", ["fixtures:contracts:keys"]],
  ["pnpm", ["fixtures:adapters:a2a:check"]],
  ["pnpm", ["fixtures:adapters:agent:check"]],
  ["pnpm", ["fixtures:cli-parity:check"]],
  ["pnpm", ["fixtures:cli-help:check"]],
  ["pnpm", ["docs:exit-codes"]],
  ["pnpm", ["exec", "tsx", "packages/cli/src/index.ts", "doctor", "--json"]],
  ["pnpm", ["test:fast"]],
];

const cargoBuild = spawnSync(cargo, ["build", "--quiet", "--manifest-path", "crates/Cargo.toml", "-p", "runx-cli", "--bin", "runx"], {
  cwd: workspaceRoot,
  env: process.env,
  stdio: "inherit",
});
if (cargoBuild.status !== 0) {
  process.exit(cargoBuild.status ?? 1);
}

for (const [command, args] of commands) {
  const result = spawnSync(command, args, {
    cwd: workspaceRoot,
    env: { ...process.env, RUNX_KERNEL_EVAL_BIN: rustKernelBin },
    stdio: "inherit",
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
