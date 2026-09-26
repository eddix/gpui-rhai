#!/usr/bin/env python3
import json
import pathlib
import subprocess
import sys


root = pathlib.Path(__file__).resolve().parent.parent
manifest_path = root / "scripts" / "verification-manifest.json"
manifest = json.loads(manifest_path.read_text())
declared = manifest.get("examples")
if not isinstance(declared, list) or any(not isinstance(item, str) for item in declared):
    raise SystemExit("verification manifest examples must be a string array")
if len(declared) != len(set(declared)):
    raise SystemExit("verification manifest contains duplicate examples")

metadata = json.loads(
    subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
)
package = next(package for package in metadata["packages"] if package["name"] == "gpui-rhai")
actual = sorted(
    target["name"] for target in package["targets"] if "example" in target["kind"]
)
declared = sorted(declared)
if declared != actual:
    missing = sorted(set(actual) - set(declared))
    stale = sorted(set(declared) - set(actual))
    raise SystemExit(f"verification manifest mismatch: missing={missing}, stale={stale}")

if "default" not in manifest.get("feature_sets", []):
    raise SystemExit("verification manifest must include the default feature set")
if "--examples" in sys.argv:
    print("\n".join(actual))
else:
    print(f"verification manifest covers {len(actual)} examples")
