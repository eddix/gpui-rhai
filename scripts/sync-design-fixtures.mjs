// Synchronize only the repository-owned UI fixtures, never an arbitrary app.
// Check by default; pass --write to apply the fully validated synchronization.
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const write = process.argv.includes("--write");
if (process.argv.slice(2).some(arg => !["--write", "--check"].includes(arg)) ||
    process.argv.includes("--write") && process.argv.includes("--check")) {
  throw new Error("Usage: node scripts/sync-design-fixtures.mjs [--check | --write]");
}
const copies = new Map();
const changes = new Map();
const bytes = file => fs.readFileSync(path.join(root, file));
const exists = file => fs.existsSync(path.join(root, file));
const plan = (file, content) => {
  const previous = changes.get(file);
  if (previous && !previous.equals(content)) throw new Error(`conflicting sources for ${file}`);
  if (!exists(file) || !bytes(file).equals(content)) changes.set(file, content);
};
const copy = (source, target) => { copies.set(target, source); plan(target, bytes(source)); };
const header = source => {
  const match = source.match(/\/\*\s*gpui-rhai\s*(\{[\s\S]*?\})\s*\*\//);
  if (!match) throw new Error("missing registry metadata header");
  return JSON.parse(match[1]);
};
const checksum = content => {
  let hash = 0xcbf29ce484222325n;
  for (const byte of content) hash = BigInt.asUintN(64, (hash ^ BigInt(byte)) * 0x100000001b3n);
  return hash.toString(16).padStart(16, "0");
};
const fixtures = [
  ["apps/disktree/ui", "modern_dark"],
  ["apps/disktree/skins/minimal", "clean_dark"],
  ["examples/hello_world/ui", "modern_dark"],
];
// The minimal skin is shipped as a standalone file app. Carry the exact
// public dependency closure and assets for its official path component.
const fixtureInstalls = new Map([["apps/disktree/skins/minimal", ["components/breadcrumb"]]]);
const modules = new Map();
for (const group of ["components", "layouts", "patterns", "motion", "charts"]) {
  for (const file of fs.readdirSync(path.join(root, "registry", group)).filter(file => file.endsWith(".rhai"))) {
    const id = `${group}/${file.slice(0, -5)}`;
    const source = `registry/${id}.rhai`;
    if (header(bytes(source).toString()).id !== id) throw new Error(`metadata identity mismatch: ${source}`);
    modules.set(id, source);
  }
}
for (const [fixture, primary] of fixtures) {
  copy("registry/tokens.rhai", `${fixture}/tokens.rhai`);
  copy(`registry/themes/${primary}.rhai`, `${fixture}/theme.rhai`);
  for (const name of ["modern_light", "modern_dark", "clean_light", "clean_dark"]) {
    const target = `${fixture}/themes/${name}.rhai`;
    if (exists(target) || name !== primary) copy(`registry/themes/${name}.rhai`, target);
  }
  for (const [id, source] of modules) {
    const target = `${fixture}/${id}.rhai`;
    if (!exists(target)) continue; // Installing a new component belongs to the CLI, not this tool.
    if (header(bytes(target).toString()).id !== id) throw new Error(`fixture identity mismatch: ${target}`);
    copy(source, target);
  }
  const installed = new Set();
  const install = id => {
    if (installed.has(id)) return;
    installed.add(id);
    const source = modules.get(id);
    if (!source) throw new Error(`unknown canonical fixture module: ${id}`);
    const metadata = header(bytes(source).toString());
    copy(source, `${fixture}/${id}.rhai`);
    for (const dependency of metadata.dependencies) install(dependency);
    for (const asset of metadata.assets ?? []) copy(`registry/assets/${asset}`, `${fixture}/assets/${asset}`);
  };
  for (const id of fixtureInstalls.get(fixture) ?? []) install(id);
  for (const filename of ["Inter-400.ttf", "Inter-500.ttf", "Inter-600.ttf"]) {
    copy(`registry/fonts/${filename}`, `${fixture}/fonts/${filename}`);
  }
  copy("registry/fonts/OFL.txt", `${fixture}/licenses/Inter-OFL-1.1.txt`);
}

// These are installed baselines, not application-owned or legacy theme files.
const baseline = "apps/disktree/.gpui-rhai/baselines";
copy("registry/tokens.rhai", `${baseline}/tokens.rhai`);
for (const [id, source] of modules) {
  const target = `${baseline}/${id}.rhai`;
  if (exists(target)) copy(source, target);
}
for (const name of ["modern_light", "modern_dark", "clean_light", "clean_dark"]) {
  copy(`registry/themes/${name}.rhai`, `${baseline}/themes/${name === "modern_dark" ? "default_dark" : name}.rhai`);
}
const manifest = "apps/disktree/.gpui-rhai/manifest.toml";
const input = bytes(manifest).toString();
const updated = input.replace(/(\[components\."([^"]+)"\]\r?\n)([\s\S]*?)(?=\r?\n\[|$)/g,
  (_, section, id, body) => {
    const source = copies.get(`${baseline}/${id}.rhai`);
    if (!source) throw new Error(`unowned baseline in ${manifest}: ${id}`);
    const content = bytes(source);
    const metadata = header(content.toString());
    for (const field of ["version", "content_hash", "dependencies"]) {
      if (!new RegExp(`^${field} = .+$`, "m").test(body)) throw new Error(`missing ${id}.${field}`);
    }
    return section + body.replace(/^version = .+$/m, `version = ${JSON.stringify(metadata.version)}`)
      .replace(/^content_hash = .+$/m, `content_hash = ${JSON.stringify(checksum(content))}`)
      .replace(/^dependencies = .+$/m, `dependencies = [${metadata.dependencies.map(id => JSON.stringify(id)).join(", ")}]`);
  });
plan(manifest, Buffer.from(updated));

// Validate every input above before the first mutation. No reference directory,
// local backend experiment, legacy palette or user plan is part of this plan.
if (!write && changes.size) throw new Error(`Outdated design fixtures:\n${[...changes.keys()].join("\n")}`);
for (const [file, content] of changes) {
  fs.mkdirSync(path.dirname(path.join(root, file)), {recursive:true});
  fs.writeFileSync(path.join(root, file), content);
}
console.log(`${write ? "Synchronized" : "Checked"} ${copies.size} owned fixture copies; ${changes.size} changes.`);
