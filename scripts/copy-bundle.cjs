const fs = require("node:fs");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const bundleDir = path.join(root, "src-tauri", "target", "release", "bundle");
const outDir = path.join(root, "build");

function findApps(dir, results = []) {
  if (!fs.existsSync(dir)) return results;
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name.endsWith(".app")) {
        results.push(fullPath);
      } else {
        findApps(fullPath, results);
      }
    }
  }
  return results;
}

function findFilesByExt(dir, exts, results = []) {
  if (!fs.existsSync(dir)) return results;
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      findFilesByExt(fullPath, exts, results);
    } else {
      const lower = entry.name.toLowerCase();
      if (exts.some((ext) => lower.endsWith(ext))) {
        results.push(fullPath);
      }
    }
  }
  return results;
}

const apps = findApps(bundleDir);
const dmgs = findFilesByExt(bundleDir, [".dmg"]);
const artifacts = [...apps, ...dmgs];

if (artifacts.length === 0) {
  console.error(`No .app or .dmg found under ${bundleDir}`);
  process.exit(1);
}

fs.mkdirSync(outDir, { recursive: true });

for (const artifactPath of artifacts) {
  const destPath = path.join(outDir, path.basename(artifactPath));
  if (fs.existsSync(destPath)) {
    fs.rmSync(destPath, { recursive: true, force: true });
  }
  const stat = fs.statSync(artifactPath);
  if (stat.isDirectory()) {
    fs.cpSync(artifactPath, destPath, { recursive: true });
  } else {
    fs.copyFileSync(artifactPath, destPath);
  }
  console.log(`Copied ${artifactPath} -> ${destPath}`);
}
