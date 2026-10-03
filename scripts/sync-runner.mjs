#!/usr/bin/env node
// Builds the Windows dummy-game runner (src-win) and copies it into src-tauri/resources/.
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";

if (process.platform !== "win32") {
  console.error(`This project only supports Windows (current platform: ${process.platform})`);
  process.exit(1);
}

const build = spawnSync("cargo", ["build", "--release"], { cwd: "src-win", stdio: "inherit", shell: false });
if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

mkdirSync("src-tauri/resources", { recursive: true });
copyFileSync("src-win/target/release/src-win.exe", "src-tauri/resources/src-win.exe");
console.log("Copied src-win.exe to src-tauri/resources/");
