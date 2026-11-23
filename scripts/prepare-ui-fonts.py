"""Build native Inter faces from the licensed webfont package, without redrawing.

Run: uv run --no-project --with fonttools python scripts/prepare-ui-fonts.py
     /path/to/node_modules/@fontsource/inter

The subset merge preserves the source cmap, advances and font metrics. It does
not synthesize weights or substitute a platform font. The generated assets
are distributed under OFL-1.1, separately from the Rust source license.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from fontTools.merge import Merger
from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.ttLib import TTFont


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package", type=Path)
    args = parser.parse_args()
    package = args.package.resolve(strict=True)
    metadata = json.loads((package / "package.json").read_text())
    if metadata["name"] != "@fontsource/inter" or metadata["version"] != "5.3.0":
        raise ValueError("Expected the reference Inter package version 5.3.0")
    output = Path(__file__).resolve().parents[1] / "registry" / "fonts"
    output.mkdir(exist_ok=True)
    records = []
    for weight in (400, 500, 600):
        sources = sorted((package / "files").glob(f"inter-*-{weight}-normal.woff"))
        if len(sources) != 7:
            raise ValueError(f"Expected seven script subsets for Inter {weight}")
        merged = Merger().merge([str(source) for source in sources])
        merged.flavor = None
        # Deterministic conversion: do not record the build machine's clock.
        merged.recalcTimestamp = False
        merged["head"].created = 0
        merged["head"].modified = 0
        if merged["OS/2"].usWeightClass != weight:
            raise ValueError("The merged face changed its weight")
        # Web @font-face assigns one CSS family independently of the font's
        # internal legacy names. Native family lookup cannot: the 500/600
        # sources call themselves "Inter Medium" / "Inter SemiBold", making
        # `Inter` weight 500/600 silently resolve the regular face. Normalize
        # family/style metadata only; keep outlines, advances and unique
        # PostScript names exactly as supplied.
        subfamily = {400: "Regular", 500: "Medium", 600: "SemiBold"}[weight]
        names = merged["name"]
        encodings = {(record.platformID, record.platEncID, record.langID)
                     for record in names.names if record.nameID in (1, 2, 4, 6)}
        for platform_id, encoding_id, language_id in sorted(encodings):
            for name_id, value in ((1, "Inter"), (2, subfamily),
                                   (16, "Inter"), (17, subfamily)):
                names.setName(value, name_id, platform_id, encoding_id, language_id)
        selection = merged["OS/2"].fsSelection & ~(1 << 6)
        merged["OS/2"].fsSelection = selection | ((1 << 6) if weight == 400 else 0)
        if names.getDebugName(1) != "Inter" or names.getDebugName(2) != subfamily:
            raise ValueError("The native family/style normalization failed")
        cmap = merged.getBestCmap()
        merged_glyphs = merged.getGlyphSet()
        for source in sources:
            original = TTFont(source)
            if names.getDebugName(6) != original["name"].getDebugName(6):
                raise ValueError("Changed the face's unique PostScript name")
            original_glyphs = original.getGlyphSet()
            for codepoint, glyph in original.getBestCmap().items():
                if codepoint not in cmap:
                    raise ValueError(f"Missing U+{codepoint:04X}")
                if merged["hmtx"][cmap[codepoint]] != original["hmtx"][glyph]:
                    raise ValueError(f"Changed advance/bearing U+{codepoint:04X}")
                expected = DecomposingRecordingPen(original_glyphs)
                actual = DecomposingRecordingPen(merged_glyphs)
                original_glyphs[glyph].draw(expected)
                merged_glyphs[cmap[codepoint]].draw(actual)
                if actual.value != expected.value:
                    raise ValueError(f"Changed outline U+{codepoint:04X}")
            for metric in ("sTypoAscender", "sTypoDescender", "sTypoLineGap"):
                if getattr(merged["OS/2"], metric) != getattr(original["OS/2"], metric):
                    raise ValueError(f"Changed {metric}")
        target = output / f"Inter-{weight}.ttf"
        merged.save(target)
        records.append({"file": target.name, "weight": weight, "codepoints": len(cmap),
                        "sha256": hashlib.sha256(target.read_bytes()).hexdigest()})
    (output / "manifest.json").write_text(json.dumps({
        "family": "Inter", "package": "@fontsource/inter", "version": "5.3.0",
        "license": "OFL-1.1", "faces": records,
    }, indent=2) + "\n")
    print(json.dumps(records, indent=2))


if __name__ == "__main__":
    main()
