#!/usr/bin/env python3
"""Compare committed PNGs in RGB, with known-different and identical controls.

Requires Pillow. Reads Git blobs, never writes or edits a baseline image.
This compares stored captures; it does not generate a fresh GPU capture.
"""

import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess

from PIL import Image, ImageChops

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--repo", type=Path, default=Path.cwd())
parser.add_argument("--base", default="48cdd0b4a7546b68f553745ce7ecaf4020f2d2b8")
parser.add_argument("--head", default="2c5e8a079304bb0c8910ddbd73eca772f49fd453")
args = parser.parse_args()


def git(*command):
    return subprocess.check_output(["git", *command], cwd=args.repo)


def image_at(revision, path):
    data = git("show", f"{revision}:{path}")
    return data, Image.open(io.BytesIO(data)).convert("RGB")


files = sorted(
    path for path in git("ls-tree", "-r", "--name-only", args.head, "tests/visual/macos").decode().splitlines()
    if path.endswith(".png")
)
rows = []
for path in files:
    old, before = image_at(args.base, path)
    new, after = image_at(args.head, path)
    if before.size != after.size:
        raise SystemExit(f"Dimensions changed: {path}")
    bbox = ImageChops.difference(before, after).getbbox()
    rows.append({
        "path": path, "size": before.size, "rgb_changed": bbox is not None, "bbox": bbox,
        "old_sha256": hashlib.sha256(old).hexdigest(),
        "new_sha256": hashlib.sha256(new).hexdigest(),
    })

_, dark = image_at(args.head, "tests/visual/macos/gallery/button.comfortable.default-dark.en.png")
_, light = image_at(args.head, "tests/visual/macos/gallery/button.comfortable.default-light.en.png")
positive = ImageChops.difference(dark, light).getbbox()
negative = ImageChops.difference(dark, dark.copy()).getbbox()
assert positive is not None and negative is None
print(json.dumps({
    "base": args.base, "head": args.head, "mode": "RGB", "fresh_capture": False,
    "positive_control_bbox": positive, "negative_control_bbox": negative,
    "count": len(rows), "changed": sum(row["rgb_changed"] for row in rows), "files": rows,
}, indent=2))
