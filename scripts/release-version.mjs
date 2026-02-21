#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const nextVersion = process.argv[2];

if (!/^\d+\.\d+\.\d+$/.test(nextVersion || "")) {
  console.error("Usage: npm run release:prepare -- <major.minor.patch>");
  process.exit(1);
}

const root = process.cwd();
const packageJsonPath = path.join(root, "package.json");
const tauriConfigPath = path.join(root, "src-tauri", "tauri.conf.json");
const cargoTomlPath = path.join(root, "src-tauri", "Cargo.toml");

const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
packageJson.version = nextVersion;
fs.writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);

const tauriConfig = JSON.parse(fs.readFileSync(tauriConfigPath, "utf8"));
tauriConfig.version = nextVersion;
fs.writeFileSync(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 2)}\n`);

const cargoToml = fs.readFileSync(cargoTomlPath, "utf8");
const updatedCargoToml = cargoToml.replace(
  /(\[package\][\s\S]*?^version\s*=\s*")([^"]+)(")/m,
  `$1${nextVersion}$3`
);

if (updatedCargoToml === cargoToml) {
  console.error("Failed to update version in src-tauri/Cargo.toml");
  process.exit(1);
}

fs.writeFileSync(cargoTomlPath, updatedCargoToml);
console.log(`Version updated to ${nextVersion}`);
