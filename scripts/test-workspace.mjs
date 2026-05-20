import { spawn, spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const workspaceRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const pnpm = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const rustKernelBin = path.join(
  workspaceRoot,
  "crates",
  "target",
  "debug",
  process.platform === "win32" ? "runx.exe" : "runx",
);
const forwardedArgs = process.argv.slice(2).filter((arg) => arg !== "--");

ensureRustKernelBin();

if (forwardedArgs.length > 0) {
  await runVitest(["run", ...forwardedArgs]);
} else {
  await runVitest(["run"]);
}

async function runVitest(args, extraEnv = {}) {
  await new Promise((resolve, reject) => {
    const child = spawn(pnpm, ["exec", "vitest", ...args], {
      cwd: workspaceRoot,
      stdio: "inherit",
      env: { ...process.env, RUNX_KERNEL_EVAL_BIN: rustKernelBin, ...extraEnv },
    });
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`vitest ${args.join(" ")} exited with ${code}`));
      }
    });
  });
}

function ensureRustKernelBin() {
  const result = spawnSync(cargo, ["build", "--quiet", "--manifest-path", "crates/Cargo.toml", "-p", "runx-cli", "--bin", "runx"], {
    cwd: workspaceRoot,
    stdio: "inherit",
    env: process.env,
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
