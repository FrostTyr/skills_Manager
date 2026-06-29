#!/usr/bin/env node

import { readFileSync } from "node:fs";

const args = parseArgs(process.argv.slice(2));
const packageJson = JSON.parse(readFileSync("package.json", "utf8"));
const tauriConfig = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const cargoToml = readFileSync("src-tauri/Cargo.toml", "utf8");
const cargoVersion = cargoToml.match(/^version = "([^"]+)"/m)?.[1];

const versions = {
  "package.json": packageJson.version,
  "src-tauri/tauri.conf.json": tauriConfig.version,
  "src-tauri/Cargo.toml": cargoVersion,
};

const missing = Object.entries(versions).filter(([, version]) => !version);
if (missing.length > 0) {
  fail(`Missing version in ${missing.map(([file]) => file).join(", ")}.`);
}

const uniqueVersions = new Set(Object.values(versions));
if (uniqueVersions.size !== 1) {
  fail(
    `Version mismatch: ${Object.entries(versions)
      .map(([file, version]) => `${file}=${version}`)
      .join(", ")}.`,
  );
}

const version = packageJson.version;
const tag = args.tag ?? process.env.RELEASE_TAG ?? "";
if (tag) {
  const tagVersion = tag.startsWith("v") ? tag.slice(1) : tag;
  if (tagVersion !== version) {
    fail(`Tag ${tag} does not match package version ${version}.`);
  }
}

console.log(`Version ${version} is consistent.`);

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (!arg.startsWith("--")) {
      fail(`Unexpected argument: ${arg}`);
    }
    const key = arg.slice(2);
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) {
      fail(`Missing value for --${key}.`);
    }
    parsed[key] = value;
    index += 1;
  }
  return parsed;
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
