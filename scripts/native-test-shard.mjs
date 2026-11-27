// Cargo runner for CI. Partition the test binary's actual inventory, so
// adding a native test never requires editing a separate shard manifest.
import {spawnSync} from "node:child_process";
import {basename} from "node:path";

const [exe, ...args] = process.argv.slice(2);
if (!exe) throw new Error("Expected a Cargo test executable");

function run(args, testThreads, extraEnv = {}) {
  const env = {...process.env, ...extraEnv};
  if (testThreads) env.RUST_TEST_THREADS = testThreads;
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
    // Gallery matrices and multi-page paint audits build many large scenes.
    // Do not overlap their peak memory or retain their allocations into a
    // regular batch.
    const isolated = name => name.startsWith("gallery::acceptance::every_gallery_page_")
      || name.startsWith("gallery::module_coverage::")
      || name === "gallery::stories::every_gallery_story_case_mounts_draws_and_presents_semantics"
      || name === "gallery::acceptance::gallery_reviewed_text_aligns_in_every_preset_and_density"
      || name === "gallery::acceptance::scheduling_tool_and_billing_risks_keep_their_stock_surface"
      || name === "gallery::acceptance::memory_rate_and_judge_stock_captions_match_only_their_actual_paint";
    // The ordinary Fast suite disables scroll layers while the SDK owns the
    // virtual rows; this direct renderer contract intentionally needs them on.
    const layerContract = "perf::invariants::repeating_fill_period_changes_the_promoted_tiles_without_moving_the_scroll";
    const regular = tests.filter(name => !isolated(name) && name !== layerContract);
    const batches = tests.filter(isolated).map(name => ({tests: [name], threads: "1"}));
    if (tests.includes(layerContract)) {
      batches.push({tests: [layerContract], threads: "1", env: {GPUI_SCROLL_LAYERS: "1"}});
    }
    for (let from = 0; from < regular.length; from += 48) {
      batches.push({tests: regular.slice(from, from + 48), threads: "2"});
    }
    let failed = false;
    for (const [i, batch] of batches.entries()) {
      console.log(`Native shard ${shard + 1}/${count}, batch ${i + 1}/${batches.length}`);
      if (run([...args, "--exact", ...batch.tests], batch.threads, batch.env) !== 0) failed = true;
    }
    process.exitCode = failed ? 1 : 0;
  }
}
