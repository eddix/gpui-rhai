// Compile shared schema data into standalone literal component declarations.
// Runtime module-purity rules remain unchanged; installed sources need no generator.
import fs from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const check = process.argv.includes("--check");
const contracts = [{id:"command-items", targets:["command", "command_dialog"]}];

function rhai(value) {
  if (value === null) return "()";
  if (Array.isArray(value)) return `[${value.map(rhai).join(", ")}]`;
  if (typeof value === "object") return `#{${Object.entries(value).map(([key,item])=>`${JSON.stringify(key)}: ${rhai(item)}`).join(", ")}}`;
  return JSON.stringify(value);
}

let stale = 0;
for (const {id,targets} of contracts) {
  const value = JSON.parse(fs.readFileSync(path.join(root, "registry/contracts",`${id}.json`),"utf8"));
  const start = `/* @contract ${id}:start */`;
  const end = `/* @contract ${id}:end */`;
  const generated = `${start} ${rhai(value)} ${end}`;
  for (const target of targets) {
    const file = path.join(root,"registry/components",`${target}.rhai`);
    const source = fs.readFileSync(file,"utf8");
    const first = source.indexOf(start), last = source.indexOf(end,first);
    if (first < 0 || last < first || source.indexOf(start,first+1) >= 0) throw new Error(`Invalid contract anchors: ${target}/${id}`);
    const next = source.slice(0,first)+generated+source.slice(last+end.length);
    if (next !== source) {
      stale++;
      if (check) process.stderr.write(`Stale component contract: ${target}/${id}\n`);
      else fs.writeFileSync(file,next);
    }
  }
}
if (check && stale) process.exitCode = 1;
else process.stdout.write(`Checked ${contracts.length} shared component contract; ${stale} ${check ? "stale" : "updated"} copies.\n`);
