#!/usr/bin/env node
// Builds the Windows dummy-game runner (./runner) and copies it into src-tauri/resources/, where
// Tauri bundles it as data/quest-runner.exe. Run this before `pnpm tauri dev` or `pnpm tauri build`.
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";

const RUNNER_DIR = "runner";
const BINARY = "quest-runner.exe";
const RESOURCES_DIR = "src-tauri/resources";

if (process.platform !== "win32") {
  console.log(`No runner needed on ${process.platform}: the app copies itself as the fake game.`);
  process.exit(0);
}

const build = spawnSync("cargo", ["build", "--release"], { cwd: RUNNER_DIR, stdio: "inherit", shell: false });
if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

mkdirSync(RESOURCES_DIR, { recursive: true });
copyFileSync(`${RUNNER_DIR}/target/release/${BINARY}`, `${RESOURCES_DIR}/${BINARY}`);
console.log(`Copied ${BINARY} to ${RESOURCES_DIR}/`);
