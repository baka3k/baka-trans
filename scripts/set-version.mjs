import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const version = process.argv[2];
const semverPattern = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

if (!version || !semverPattern.test(version)) {
  console.error("Usage: node scripts/set-version.mjs <semver>");
  process.exit(1);
}

const root = resolve(import.meta.dirname, "..");
const branch = execFileSync("git", ["branch", "--show-current"], {
  cwd: root,
  encoding: "utf8",
}).trim();
const status = execFileSync("git", ["status", "--porcelain"], {
  cwd: root,
  encoding: "utf8",
}).trim();

if (branch !== "main") {
  console.error(`Version changes must start on main, not ${branch || "detached HEAD"}.`);
  process.exit(1);
}
if (status) {
  console.error("Commit or stash existing changes before changing the release version.");
  process.exit(1);
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function replaceRequired(path, pattern, replacement) {
  const current = readFileSync(path, "utf8");
  const next = current.replace(pattern, replacement);
  if (next === current && !current.includes(`version = "${version}"`)) {
    throw new Error(`Could not update the version in ${path}`);
  }
  writeFileSync(path, next);
}

const packagePath = resolve(root, "package.json");
const packageLockPath = resolve(root, "package-lock.json");
const tauriConfigPath = resolve(root, "src-tauri/tauri.conf.json");
const cargoTomlPath = resolve(root, "src-tauri/Cargo.toml");
const cargoLockPath = resolve(root, "src-tauri/Cargo.lock");

const packageJson = readJson(packagePath);
packageJson.version = version;
writeJson(packagePath, packageJson);

const packageLock = readJson(packageLockPath);
packageLock.version = version;
packageLock.packages[""].version = version;
writeJson(packageLockPath, packageLock);

const tauriConfig = readJson(tauriConfigPath);
tauriConfig.version = version;
writeJson(tauriConfigPath, tauriConfig);

replaceRequired(
  cargoTomlPath,
  /(\[package\][\s\S]*?\nversion = ")[^"]+("\n)/,
  `$1${version}$2`,
);
replaceRequired(
  cargoLockPath,
  /(\[\[package\]\]\nname = "baka-trans"\nversion = ")[^"]+("\n)/,
  `$1${version}$2`,
);

console.log(`Updated Baka Trans version to ${version}.`);
console.log("Review, commit, and push these files before running the release script.");
