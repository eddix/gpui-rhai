// Generates expanded semantic surface, border, shadow, and 8x4 categorical
// palette tokens directly into every bundled theme file in registry/themes/.
// Preserves each existing theme's attribution header and calibrated 18-color core.

import fs from "node:fs";
import path from "node:path";

export const CATEGORICAL_HUES = [
  "blue",
  "violet",
  "teal",
  "green",
  "amber",
  "rose",
  "orange",
  "slate",
];

// ---- Color math (sRGB <-> Oklab + WCAG contrast) ---------------------------

function parseHex(hex) {
  const clean = hex.replace(/^0x/i, "").replace(/^#/, "").padEnd(8, "f");
  const n = Number.parseInt(clean, 16);
  return [(n >>> 24) & 255, (n >>> 16) & 255, (n >>> 8) & 255, n & 255];
}

export function toHex([r, g, b, a = 255]) {
  const c = v => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, "0");
  return `0x${c(r)}${c(g)}${c(b)}${c(a)}`;
}

function srgbToLinear(c) {
  const x = c / 255;
  return x <= 0.04045 ? x / 12.92 : Math.pow((x + 0.055) / 1.055, 2.4);
}

function linearToSrgb(x) {
  const clamped = Math.max(0, Math.min(1, x));
  const y = clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * Math.pow(clamped, 1 / 2.4) - 0.055;
  return Math.round(y * 255);
}

export function rgbToOklab([r, g, b]) {
  const lr = srgbToLinear(r);
  const lg = srgbToLinear(g);
  const lb = srgbToLinear(b);
  const l = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
  const m = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
  const s = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}

function oklabToLinearRgb([L, a, b]) {
  const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = L - 0.0894841775 * a - 1.291485548 * b;
  const l = l_ * l_ * l_;
  const m = m_ * m_ * m_;
  const s = s_ * s_ * s_;
  return [
    +4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
}

function inGamut([r, g, b]) {
  return r >= -1e-6 && r <= 1 + 1e-6 && g >= -1e-6 && g <= 1 + 1e-6 && b >= -1e-6 && b <= 1 + 1e-6;
}

export function oklabToRgb([L, a, b]) {
  const clampedL = Math.max(0, Math.min(1, L));
  let rgb = oklabToLinearRgb([clampedL, a, b]);
  if (!inGamut(rgb)) {
    let lo = 0;
    let hi = 1;
    for (let i = 0; i < 18; i++) {
      const mid = (lo + hi) / 2;
      if (inGamut(oklabToLinearRgb([clampedL, a * mid, b * mid]))) lo = mid;
      else hi = mid;
    }
    rgb = oklabToLinearRgb([clampedL, a * lo, b * lo]);
  }
  return [linearToSrgb(rgb[0]), linearToSrgb(rgb[1]), linearToSrgb(rgb[2]), 255];
}

function relLuminance([r, g, b]) {
  return 0.2126 * srgbToLinear(r) + 0.7152 * srgbToLinear(g) + 0.0722 * srgbToLinear(b);
}

export function contrastRatio(c1, c2) {
  const l1 = relLuminance(c1);
  const l2 = relLuminance(c2);
  const hi = Math.max(l1, l2);
  const lo = Math.min(l1, l2);
  return (hi + 0.05) / (lo + 0.05);
}

function mixRgb(base, other, w) {
  return [
    Math.round(base[0] * (1 - w) + other[0] * w),
    Math.round(base[1] * (1 - w) + other[1] * w),
    Math.round(base[2] * (1 - w) + other[2] * w),
    255,
  ];
}

function shiftLightness(rgb, deltaL) {
  const [L, a, b] = rgbToOklab(rgb);
  return oklabToRgb([Math.max(0.05, Math.min(0.995, L + deltaL)), a, b]);
}

// Adjust `color` in Oklab along lightness (preserving hue) toward light or dark
// until it meets `targetRatio` against every background in `backgrounds`.
function calibrateContrast(color, backgrounds, mode, targetRatio) {
  const minContrast = c => Math.min(...backgrounds.map(bg => contrastRatio(c, bg)));
  if (minContrast(color) >= targetRatio) return color;
  const [L0, a, b] = rgbToOklab(color);
  // In dark mode move lighter; in light mode move darker.
  const targetL = mode === "dark" ? 0.96 : 0.16;
  let lo = 0;
  let hi = 1;
  let best = oklabToRgb([targetL, a * 0.2, b * 0.2]);
  for (let i = 0; i < 24; i++) {
    const mid = (lo + hi) / 2;
    const cand = oklabToRgb([L0 + (targetL - L0) * mid, a, b]);
    if (minContrast(cand) >= targetRatio) {
      best = cand;
      hi = mid;
    } else {
      lo = mid;
    }
  }
  if (minContrast(best) < targetRatio) {
    // Reduce chroma slightly toward neutral endpoint if saturated hue cannot reach ratio.
    const endpoint = mode === "dark" ? [255, 255, 255, 255] : [18, 18, 20, 255];
    for (let t = 0.05; t <= 1.0; t += 0.05) {
      const cand = mixRgb(best, endpoint, t);
      if (minContrast(cand) >= targetRatio) return cand;
    }
    return endpoint;
  }
  return best;
}

// ---- Theme-authentic categorical hue anchors (8 non-semantic hues) ---------

const DEFAULT_LIGHT_HUES = {
  blue: "0x2563ebff",
  violet: "0x7c3aedff",
  teal: "0x0d9488ff",
  green: "0x16a34aff",
  amber: "0xd97706ff",
  rose: "0xe11d48ff",
  orange: "0xea580cff",
  slate: "0x64748bff",
};

const DEFAULT_DARK_HUES = {
  blue: "0x60a5faff",
  violet: "0xa78bfaff",
  teal: "0x2dd4bfff",
  green: "0x4ade80ff",
  amber: "0xfbbf24ff",
  rose: "0xfb7185ff",
  orange: "0xfb923cff",
  slate: "0x94a3b8ff",
};

// Iconic community palettes keep their recognizable upstream hue accents.
const THEME_CATEGORICAL_ANCHORS = {
  "neutral_light.rhai": {
    blue: "0x0190feff",
    violet: "0x8a35d6ff",
    teal: "0x0b8576ff",
    green: "0x1e8a4fff",
    amber: "0xb25e00ff",
    rose: "0xd9385aff",
    orange: "0xd95f18ff",
    slate: "0x6b6b72ff",
  },
  "neutral_dark.rhai": {
    blue: "0x4fb0feff",
    violet: "0xb98af0ff",
    teal: "0x4fc3b4ff",
    green: "0x5fc88aff",
    amber: "0xe6c24dff",
    rose: "0xf06db5ff",
    orange: "0xf2a15cff",
    slate: "0xa1a1a6ff",
  },
  "catppuccin_latte.rhai": {
    blue: "0x1e66f5ff",
    violet: "0x8839efff",
    teal: "0x179299ff",
    green: "0x40a02bff",
    amber: "0xdf8e1dff",
    rose: "0xea76cbff",
    orange: "0xfe640bff",
    slate: "0x6c6f85ff",
  },
  "catppuccin_mocha.rhai": {
    blue: "0x89b4faff",
    violet: "0xcba6f7ff",
    teal: "0x94e2d5ff",
    green: "0xa6e3a1ff",
    amber: "0xf9e2afff",
    rose: "0xf5c2e7ff",
    orange: "0xfab387ff",
    slate: "0x9399b2ff",
  },
  "tokyo_night.rhai": {
    blue: "0x7aa2f7ff",
    violet: "0xbb9af7ff",
    teal: "0x73dacaff",
    green: "0x9ece6aff",
    amber: "0xe0af68ff",
    rose: "0xf7768eff",
    orange: "0xff9e64ff",
    slate: "0x9aa5ceff",
  },
  "tokyo_storm.rhai": {
    blue: "0x7aa2f7ff",
    violet: "0xbb9af7ff",
    teal: "0x73dacaff",
    green: "0x9ece6aff",
    amber: "0xe0af68ff",
    rose: "0xf7768eff",
    orange: "0xff9e64ff",
    slate: "0x9aa5ceff",
  },
  "dracula.rhai": {
    blue: "0x8be9fdff",
    violet: "0xbd93f9ff",
    teal: "0x50fa7bff",
    green: "0x69ff94ff",
    amber: "0xf1fa8cff",
    rose: "0xff79c6ff",
    orange: "0xffb86cff",
    slate: "0x95a5d4ff",
  },
  "nord.rhai": {
    blue: "0x81a1c1ff",
    violet: "0xb48eadff",
    teal: "0x88c0d0ff",
    green: "0xa3be8cff",
    amber: "0xebcb8bff",
    rose: "0xbf616aff",
    orange: "0xd08770ff",
    slate: "0x9aa6baff",
  },
  "gruvbox.rhai": {
    blue: "0x83a598ff",
    violet: "0xd3869bff",
    teal: "0x8ec07cff",
    green: "0xb8bb26ff",
    amber: "0xfabd2fff",
    rose: "0xfb4934ff",
    orange: "0xfe8019ff",
    slate: "0xa89984ff",
  },
  "everforest.rhai": {
    blue: "0x7fbbb3ff",
    violet: "0xd699b6ff",
    teal: "0x83c092ff",
    green: "0xa7c080ff",
    amber: "0xdbbc7fff",
    rose: "0xe67e80ff",
    orange: "0xe69875ff",
    slate: "0x9da9a0ff",
  },
  "molokai.rhai": {
    blue: "0x66d9efff",
    violet: "0xae81ffff",
    teal: "0x52d6b3ff",
    green: "0xa6e22eff",
    amber: "0xe6db74ff",
    rose: "0xf92672ff",
    orange: "0xfd971fff",
    slate: "0xa6b2b5ff",
  },
};

export function computeExpandedPaletteColors(fileName, mode, coreColors) {
  const surface = parseHex(coreColors.surface);
  const surfaceRaised = parseHex(coreColors.surface_raised);
  const surfaceHover = parseHex(coreColors.surface_hover);
  const textPrimary = parseHex(coreColors.text_primary);
  const textMuted = parseHex(coreColors.text_muted);
  const border = parseHex(coreColors.border);

  // 1. Expanded 5-level Surface hierarchy
  // Light: surface_sunken < surface < surface_raised <= surface_elevated
  // Dark:  surface_sunken < surface < surface_raised < surface_elevated
  let surfaceSunken;
  let surfaceElevated;
  if (mode === "light") {
    // Subtle recessed tray/well step below page canvas, ensuring text readability.
    surfaceSunken = shiftLightness(surface, -0.024);
    if (contrastRatio(textPrimary, surfaceSunken) < 7.0 || contrastRatio(textMuted, surfaceSunken) < 5.2) {
      surfaceSunken = shiftLightness(surface, -0.015);
    }
    const [raisedL] = rgbToOklab(surfaceRaised);
    surfaceElevated = raisedL >= 0.99 ? [255, 255, 255, 255] : shiftLightness(surfaceRaised, 0.015);
  } else {
    const [pageL] = rgbToOklab(surface);
    const step = pageL < 0.14 ? -0.025 : -0.035;
    surfaceSunken = shiftLightness(surface, step);
    // Keep elevated card/popover clearly lifted above surface_raised.
    surfaceElevated = shiftLightness(surfaceRaised, 0.042);
  }

  // 2. Expanded 3-level Border hierarchy
  // border_subtle: soft decorative outline (replaces hardcoded 0xecececff)
  // border_strong: emphasized/hover functional outline (>= 3.0:1 on surface & surface_raised)
  const borderSubtle = mode === "light"
    ? mixRgb(surfaceRaised, textPrimary, 0.085)
    : mixRgb(surfaceRaised, textPrimary, 0.11);
  const borderStrong = calibrateContrast(
    mixRgb(border, textPrimary, 0.22),
    [surface, surfaceRaised],
    mode,
    3.2,
  );

  // 3. Mode-calibrated Shadow anchors
  const shadowAmbient = mode === "light" ? "0x0000000d" : "0x00000052";
  const shadowKey = mode === "light" ? "0x00000018" : "0x0000007a";

  // 4. 8x4 Non-semantic Categorical Palette
  const baseHues = THEME_CATEGORICAL_ANCHORS[fileName]
    || (mode === "light" ? DEFAULT_LIGHT_HUES : DEFAULT_DARK_HUES);

  const categorical = {};
  for (const hue of CATEGORICAL_HUES) {
    const anchor = parseHex(baseHues[hue]);
    // Solid mark: >= 3.0:1 non-text contrast against surface_raised & surface.
    const solid = calibrateContrast(anchor, [surfaceRaised, surface], mode, 3.0);
    // Soft tinted fill blended cleanly into surface_raised (opaque, theme-native).
    const softWeight = mode === "light" ? 0.12 : 0.20;
    const soft = mixRgb(surfaceRaised, solid, softWeight);
    // Ink foreground: >= 4.65:1 readable text against surface, surface_raised, surface_sunken, and its own soft fill.
    const ink = calibrateContrast(anchor, [surface, surfaceRaised, surfaceSunken, soft], mode, 4.65);
    // Hue-tinted border for chips/tiles/lanes.
    const borderWeight = mode === "light" ? 0.34 : 0.42;
    const hueBorder = mixRgb(surfaceRaised, solid, borderWeight);

    categorical[`palette.${hue}`] = toHex(solid);
    categorical[`palette.${hue}_ink`] = toHex(ink);
    categorical[`palette.${hue}_soft`] = toHex(soft);
    categorical[`palette.${hue}_border`] = toHex(hueBorder);
  }

  return {
    surface_sunken: toHex(surfaceSunken),
    surface_elevated: toHex(surfaceElevated),
    border_subtle: toHex(borderSubtle),
    border_strong: toHex(borderStrong),
    "shadow.ambient": shadowAmbient,
    "shadow.key": shadowKey,
    ...categorical,
  };
}

// Neutral Light & Neutral Dark: extracted pure color themes from the Modern/Clean
// neutral palette, calibrated to satisfy the full readability matrix while keeping
// the crisp neutral character of the design system.
export const NEUTRAL_THEMES = [
  {
    file: "neutral_light.rhai",
    family: "Neutral",
    name: "Light",
    mode: "light",
    header: [
      "// Neutral Light color palette extracted from the Modern/Clean reference.",
      "// Orthogonal to the `design` environment axis (modern / clean).",
    ],
    core: {
      surface: "0xf4f4f5ff",
      surface_raised: "0xffffffff",
      surface_hover: "0xe2e2e5ff",
      text_primary: "0x171717ff",
      text_muted: "0x49494fff",
      accent: "0x0060e2ff",
      accent_hover: "0x004eb8ff",
      on_accent: "0xffffffff",
      danger: "0xc81e30ff",
      on_danger: "0xffffffff",
      warning: "0x9c6000ff",
      on_warning: "0xffffffff",
      success: "0x168046ff",
      on_success: "0xffffffff",
      border: "0xa6a6acff",
      focus_ring: "0x0060e2ff",
      selection: "0xc0d2f0ff",
      disabled: "0x8a8a91ff",
    },
  },
  {
    file: "neutral_dark.rhai",
    family: "Neutral",
    name: "Dark",
    mode: "dark",
    header: [
      "// Neutral Dark color palette extracted from the Modern/Clean reference.",
      "// Orthogonal to the `design` environment axis (modern / clean).",
    ],
    core: {
      surface: "0x111113ff",
      surface_raised: "0x1d1d20ff",
      surface_hover: "0x2a2a2eff",
      text_primary: "0xedededff",
      text_muted: "0xa9a9b0ff",
      accent: "0x4fb0feff",
      accent_hover: "0x6ec0ffff",
      on_accent: "0x0e1116ff",
      danger: "0xff7a8aff",
      on_danger: "0x1b1110ff",
      warning: "0xf5be58ff",
      on_warning: "0x1b140cff",
      success: "0x5fc88aff",
      on_success: "0x0c1912ff",
      border: "0x4f4f56ff",
      focus_ring: "0x4fb0feff",
      selection: "0x111d34ff",
      disabled: "0x76767eff",
    },
  },
];

// Core 18-color definitions + attribution headers for the 19 bundled themes.
// Parsed directly from existing files on disk so we preserve every existing
// attribution comment and every calibrated core color exactly.
const CORE_COLOR_ORDER = [
  "surface_sunken",
  "surface",
  "surface_raised",
  "surface_elevated",
  "surface_hover",
  "text_primary",
  "text_muted",
  "accent",
  "accent_hover",
  "on_accent",
  "danger",
  "on_danger",
  "warning",
  "on_warning",
  "success",
  "on_success",
  "border_subtle",
  "border",
  "border_strong",
  "focus_ring",
  "selection",
  "disabled",
];

export function renderPaletteThemeFile({ header, family, name, mode, core, expanded, extraTokens = [] }) {
  const allColors = { ...core, ...expanded };
  const colorLines = [];
  for (const key of CORE_COLOR_ORDER) {
    colorLines.push(`                ${key}: ${allColors[key]},`);
  }
  colorLines.push(`                "shadow.ambient": ${allColors["shadow.ambient"]},`);
  colorLines.push(`                "shadow.key": ${allColors["shadow.key"]},`);
  for (const hue of CATEGORICAL_HUES) {
    colorLines.push(
      `                "palette.${hue}": ${allColors[`palette.${hue}`]}, `
      + `"palette.${hue}_ink": ${allColors[`palette.${hue}_ink`]}, `
      + `"palette.${hue}_soft": ${allColors[`palette.${hue}_soft`]}, `
      + `"palette.${hue}_border": ${allColors[`palette.${hue}_border`]},`,
    );
  }
  const extraBlock = extraTokens.length > 0
    ? `\n${extraTokens.map(l => `            ${l}`).join("\n")}`
    : "";
  return [
    ...header,
    "fn theme() {",
    "    #{",
    `        family: ${JSON.stringify(family)},`,
    `        name: ${JSON.stringify(name)},`,
    `        mode: ${JSON.stringify(mode)},`,
    "        tokens: #{",
    "            colors: #{",
    ...colorLines,
    "            }," + extraBlock,
    "        },",
    "    }",
    "}",
    "",
  ].join("\n");
}

export function extractExistingPaletteSpec(filePath) {
  const source = fs.readFileSync(filePath, "utf8");
  const lines = source.split("\n");
  const header = [];
  for (const line of lines) {
    if (line.startsWith("//")) header.push(line);
    else break;
  }
  const family = source.match(/family:\s*"([^"]+)"/)?.[1];
  const name = source.match(/name:\s*"([^"]+)"/)?.[1];
  const mode = source.match(/mode:\s*"(light|dark)"/)?.[1];
  if (!family || !name || !mode) {
    throw new Error(`Could not parse theme metadata from ${filePath}`);
  }
  const coreKeys = [
    "surface", "surface_raised", "surface_hover",
    "text_primary", "text_muted",
    "accent", "accent_hover", "on_accent",
    "danger", "on_danger", "warning", "on_warning", "success", "on_success",
    "border", "focus_ring", "selection", "disabled",
  ];
  const core = {};
  for (const key of coreKeys) {
    const match = source.match(new RegExp(`\\b${key}:\\s*(0x[0-9a-fA-F]{8})`));
    if (!match) throw new Error(`Missing core color ${key} in ${filePath}`);
    core[key] = match[1].toLowerCase();
  }
  return { file: path.basename(filePath), family, name, mode, header, core };
}
