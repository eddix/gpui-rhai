// Cargo runner for CI. Partition the test binary's actual inventory, so
// adding a native test never requires editing a separate shard manifest.
import {spawnSync} from "node:child_process";
import {basename} from "node:path";

const [exe, ...args] = process.argv.slice(2);
if (!exe) throw new Error("Expected a Cargo test executable");

function run(args, testThreads) {
  const env = testThreads ? {...process.env, RUST_TEST_THREADS: testThreads} : process.env;
  const result = spawnSync(exe, args, {stdio: "inherit", env});
  if (result.error) throw result.error;
  if (result.signal) console.error(`Test executable terminated by ${result.signal}`);
  return result.status ?? 1;
}

if (!/^native-[0-9a-f]+(?:\.exe)?$/.test(basename(exe))) {
  // The standard shard zero also runs the non-native test targets.
  process.exitCode = run(args);
} else {
  const shard = Number(process.env.GPUI_RHAI_NATIVE_SHARD);
  const count = Number(process.env.GPUI_RHAI_NATIVE_SHARDS);
  if (!Number.isInteger(shard) || !Number.isInteger(count) || shard < 0 || shard >= count) {
    throw new Error("Expected GPUI_RHAI_NATIVE_SHARD in [0, GPUI_RHAI_NATIVE_SHARDS)");
  }
  const list = spawnSync(exe, ["--list"], {encoding: "utf8"});
  if (list.error) throw list.error;
  if (list.status !== 0) throw new Error(`Could not enumerate native tests: ${list.stderr}`);
  const inventory = list.stdout.split(/\r?\n/)
    .filter(line => line.endsWith(": test"))
    .map(line => line.slice(0, -6))
    .sort();
  const tests = inventory.filter((_, i) => i % count === shard);
  if (tests.length === 0) throw new Error(`Native test shard ${shard} is empty`);
  console.log(`Native shard ${shard + 1}/${count}: ${tests.length}/${inventory.length} tests`);
  if (args.includes("--list")) {
    process.exitCode = run([...args, "--exact", ...tests]);
  } else {
    // Exhaustive Gallery audits build hundreds of views in one test. Give
    // them their own process, and recycle the allocator between other batches.
    const exhaustive = name => name.startsWith("gallery::acceptance::every_gallery_page_")
      || name.startsWith("gallery::module_coverage::")
      || name === "gallery::stories::every_gallery_story_case_mounts_draws_and_presents_semantics"
      || name === "gallery::acceptance::gallery_reviewed_text_aligns_in_every_preset_and_density";
    const regular = tests.filter(name => !exhaustive(name));
    const batches = tests.filter(exhaustive).map(name => ({tests: [name], threads: "1"}));
    for (let from = 0; from < regular.length; from += 48) {
      batches.push({tests: regular.slice(from, from + 48), threads: "2"});
    }
    let failed = false;
    for (const [i, batch] of batches.entries()) {
      console.log(`Native shard ${shard + 1}/${count}, batch ${i + 1}/${batches.length}`);
      if (run([...args, "--exact", ...batch.tests], batch.threads) !== 0) failed = true;
    }
    process.exitCode = failed ? 1 : 0;
  }
}
