#!/usr/bin/env node

import { createHash } from "node:crypto";
import {
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";

const args = parseArgs(process.argv.slice(2));
const assetsDir = resolve(
  args["assets-dir"] ?? process.env.RELEASE_ASSETS_DIR ?? "release-assets",
);
const outputPath = resolve(
  args.output ??
    process.env.RELEASE_MANIFEST_OUTPUT ??
    join(assetsDir, "release-manifest.json"),
);
const version = args.version ?? process.env.RELEASE_VERSION;
const tag = args.tag ?? process.env.RELEASE_TAG ?? (version ? `v${version}` : "");
const releaseDate = args.date ?? process.env.RELEASE_DATE ?? new Date().toISOString();
const repository = args.repository ?? process.env.GITHUB_REPOSITORY ?? "";
const commit = args.commit ?? process.env.GITHUB_SHA ?? "";

if (!version) {
  fail("Missing release version. Set RELEASE_VERSION or pass --version.");
}

if (!tag) {
  fail("Missing release tag. Set RELEASE_TAG or pass --tag.");
}

const files = walk(assetsDir)
  .filter((file) => {
    const name = basename(file).toLowerCase();
    return (
      name !== "release-manifest.json" &&
      !name.endsWith(".sha256") &&
      isInstallerAsset(name)
    );
  })
  .sort((a, b) => basename(a).localeCompare(basename(b)));

if (files.length === 0) {
  fail(`No release assets found in ${assetsDir}.`);
}

const assets = files.map((file) => {
  const name = basename(file);
  return {
    name,
    platform: detectPlatform(name),
    arch: detectArch(name),
    sizeBytes: statSync(file).size,
    sha256: sha256(file),
    downloadUrl: repository
      ? `https://github.com/${repository}/releases/download/${tag}/${encodeURIComponent(name)}`
      : null,
  };
});

const manifest = {
  schemaVersion: 1,
  version,
  tag,
  releaseDate,
  source: {
    repository: repository || null,
    commit: commit || null,
  },
  assets,
};

mkdirSync(dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Wrote ${outputPath}`);

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

function walk(root) {
  return readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const path = join(root, entry.name);
    if (entry.isDirectory()) {
      return walk(path);
    }
    if (entry.isFile()) {
      return [path];
    }
    return [];
  });
}

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function isInstallerAsset(name) {
  return (
    name.endsWith(".dmg") ||
    name.endsWith(".exe") ||
    name.endsWith(".msi") ||
    name.endsWith(".app.tar.gz")
  );
}

function detectPlatform(name) {
  const lower = name.toLowerCase();
  if (lower.includes("windows") || lower.endsWith(".exe")) {
    return "windows";
  }
  if (lower.includes("macos") || lower.endsWith(".dmg")) {
    return "macos";
  }
  return "unknown";
}

function detectArch(name) {
  const lower = name.toLowerCase();
  if (lower.includes("arm64") || lower.includes("aarch64")) {
    return "arm64";
  }
  if (lower.includes("x64") || lower.includes("x86_64")) {
    return "x64";
  }
  return "unknown";
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
