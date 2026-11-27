// Maintained implementation recipes. This is not a source-extraction tool.
// Run with --check in CI; no browser, network, or external project is required.
import fs from "node:fs";
import path from "node:path";
import {
  CATEGORICAL_HUES,
  NEUTRAL_THEMES,
  computeExpandedPaletteColors,
  extractExistingPaletteSpec,
  renderPaletteThemeFile,
} from "./palette-catalog.mjs";

const root = path.resolve(import.meta.dirname, "..");
const check = process.argv.includes("--check");
const tick = String.fromCharCode(96);
const px = value => `px(${value})`;
const color = name => `theme_color(${JSON.stringify(name)})`;
const rgba = hex => `rgba(0x${hex.replace(/^#/, "").padEnd(8, "f")})`;
const length = name => `theme_length(${JSON.stringify(name)})`;
const readingLength = px => `${length("space.unit")}.times(${px/4})`;
const readingType = (size,line) => `style().font_size(${readingLength(size)}).line_height(${readingLength(line)})`;
const sizeTable = values => `#{ ${["xs", "sm", "md", "lg"].map((key, i) => `${key}: px(${values[i]})`).join(", ")} }`;
const scale = (values, compact) => compact
  ? `by_env(["density","size"], #{comfortable:${sizeTable(values)},compact:${sizeTable(compact)}})`
  : `by_env("size", ${sizeTable(values)})`;
const density = (comfortable, compact) => `by_env("density",#{comfortable:px(${comfortable}),compact:px(${compact})})`;
const compactScale = (values, floor = 20, reduction = 4) => scale(values,values.map(value=>Math.max(floor,value-reduction)));
const role = (size, line, weight = 400, extra = "") => `#{ size: px(${size}), line_height: px(${line}), weight: ${weight}${extra} }`;
// Policy for the built-in recipes, not a restriction on the SDK's shadow API.
// Zero-blur spread still serves as a paint-only outline and is independent of
// how far a cast shadow extends.
const shortBlur = 3;
const shortOffset = 2;
const shadow = (x, y, blur, spread, ink, inset = false) => {
  if (!inset && (blur > shortBlur || Math.abs(x) > shortOffset || Math.abs(y) > shortOffset)) {
    throw new Error(`Built-in outer shadow exceeds short elevation: offset (${x}, ${y}), blur ${blur}`);
  }
  return `shadow(#{x:${x},y:${y},blur:${blur},spread:${spread},color:${ink},inset:${inset}})`;
};
const shortDrop = (ink, x = 0, spread = 0) => shadow(x,shortOffset,shortBlur,spread,ink);
const shadows = layers => `.shadows([${layers.join(",")}])`;
const gradientLayer = (colors, box = "border") => `#{angle:180,origin:"${box}",clip:"${box}",stops:[${colors.map((ink,i)=>`#{offset:${i/(colors.length-1)},color:${ink}}`).join(",")}]}`;
const layeredGradient = layers => `.layered_gradient(layered_gradient([${layers.join(",")}]))`;
const transparent = rgba("00000000");
const t = name => `.typography(${JSON.stringify(name)})`;
const badgeTones = ["neutral", "info", "success", "warning", "danger", "purple"];
// Strong emphasis is a derived variant, unlike the exact subtle-status pair.
// Keep a readable preferred inverse where possible, then try both endpoints.
const strongInk = background => `readable(readable(${color("button.primary_fg")},${rgba("ffffff")},${background},4.5),${rgba("000000")},${background},4.5)`;
const derivedInk = (preferred, backgrounds) => backgrounds.reduce((ink, backing) =>
  `readable(${ink},${color("text_primary")}.over(${color("surface_raised")}),${backing},4.65)`,
  `${preferred}.over(${color("surface_raised")})`);
const metadataInk = () => derivedInk(color("text_muted"), [
  color("surface"), color("surface_raised"),
  `${color("surface_hover")}.over(${color("surface_raised")})`,
  `${color("selection")}.over(${color("surface_raised")})`,
  `${color("table.selection")}.over(${color("surface_raised")})`,
  color("table.surface"), `${color("table.selection")}.over(${color("table.surface")})`,
]);

// Recipe geometry follows the same late-bound reading scale as typography.
// Keep application literals and structural hairlines independent of that scale.
// One public design unit also lets a Host tune geometry without replacing every
// component part. This changes no default dimensions (space.unit is 4px).
function scalableGeometry(styles) {
  return Object.fromEntries(Object.entries(styles).map(([name, value]) => [name,
    typeof value === "string"
      ? value.replace(/\.(padding(?:_[a-z]+)?|margin(?:_[a-z]+)?|gap(?:_[xy])?|radius(?:_[a-z]+)?|(?:min_|max_)?(?:width|height)|flex_basis|top|right|bottom|left|inset_start|inset_end)\(px\((-?[\d.]+)\)\)/g,
        (original, method, raw) => {
          const value = Number(raw);
          if (value === 0 || (/^(?:(?:min_|max_)?(?:width|height))$/.test(method) && Math.abs(value) <= 2)) return original;
          const unit=/^(padding|margin|gap)/.test(method) ? "space.spacing_unit" : "space.unit";
          return `.${method}(${length(unit)}.times(${value / 4}))`;
        })
      : scalableGeometry(value),
  ]));
}

const modules = [];
for (const group of ["components", "layouts", "patterns", "motion", "charts"]) {
  for (const file of fs.readdirSync(path.join(root, "registry", group)).filter(file => file.endsWith(".rhai")).sort()) {
    const text = fs.readFileSync(path.join(root, "registry", group, file), "utf8");
    const definition = text.match(/^[ \t]*define_component\s*\(/m);
    const declared = definition && text.slice(definition.index).match(/\bparts:\s*\[([^\]]*)\]/);
    if (!declared) throw new Error(`missing formal recipe parts in ${group}/${file}`);
    const parts = [...declared[1].matchAll(/"([^"]+)"/g)].map(match => match[1]);
    modules.push({id: `${group}/${file.slice(0, -5)}`, parts});
  }
}

function recipe(preset, dark) {
  const clean = preset === "clean";
  const black = (alpha = 1) => `alpha(rgb(0x000000),${alpha})`;
  const white = (alpha = 1) => `alpha(rgb(0xffffff),${alpha})`;
  const ink = dark ? (clean ? white(.9) : rgba("ededed")) : (clean ? black(.88) : rgba("171717"));
  const strongest = dark ? (clean ? white() : rgba("ededed")) : (clean ? black() : rgba("171717"));
  const muted = clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "a1a1a6" : "737373");
  const diagramBody = clean ? muted : rgba(dark ? "a1a1a6" : "6b6b6b");
  const faint = clean ? (dark ? white(.46) : black(.45)) : rgba(dark ? "8b8b92" : "8f8f8f");
  const placeholder = clean ? (dark ? white(.3) : black(.3)) : rgba(dark ? "5c5c63" : "b5b5b5");
  const panel = rgba(dark ? (clean ? "1b1b1e" : "19191b") : "ffffff");
  const field = rgba(dark ? (clean ? "1c1c1e" : "19191b") : "ffffff");
  const canvas = rgba(dark ? "111113" : "f4f4f5");
  const wash = clean ? (dark ? white(.06) : black(.03)) : rgba(dark ? "26262a" : "f0f0f0");
  const edge = clean ? (dark ? white(.09) : black(.06)) : rgba(dark ? "2a2a2e" : "e5e5e5");
  const edgeStrong = clean ? (dark ? white(.16) : black(.12)) : rgba(dark ? "3a3a40" : "d4d4d4");
  const focus = dark ? `alpha(rgb(0x4d8dff),0.8)` : `alpha(rgb(0x0060e2),0.7)`;
  const selected = clean ? (dark ? white(.08) : black(.05)) : rgba(dark ? "303035" : "e9e9ec");
  const semantic = {
    info: [dark ? "16243f" : "eaf1ff", dark ? "8ab4ff" : "2456d6"],
    success: [dark ? "123020" : "e6f6ed", dark ? "5fc88a" : "1e8a4f"],
    warning: [dark ? "3a2610" : "fff1e0", dark ? "ffb35c" : "b25e00"],
    danger: [dark ? "3b1c14" : "fdecea", dark ? "ff8a7a" : "c4320a"],
    purple: [dark ? "2a1c3d" : "f4ebfd", dark ? "c9a2f5" : "8a35d6"],
    neutral: [dark ? "26262a" : "f0f0f0", dark ? "a1a1a6" : "616161"],
  };
  const alphaColor = (hex, opacity) => `alpha(rgb(0x${hex}),${opacity})`;
  const costPalette = clean ? {
    agent: dark ? [.07,.14,.08,.4].map(white) : [.045,.1,.06,.3].map(black),
    llm: (dark ? [.16,.38,.22,.8] : [.1,.3,.2,.72]).map(a=>alphaColor(dark ? "b98af0" : "a651f0",a)),
    tool: (dark ? [.13,.34,.2,.75] : [.085,.28,.17,.62]).map(a=>alphaColor(dark ? "5cc6ee" : "14b3f0",a)),
    error: [rgba(dark ? "ff35232b" : "f52b0018"),alphaColor(dark ? "ff6b5f" : "f04438",dark ? .4 : .34),
      alphaColor(dark ? "ff6b5f" : "f04438",dark ? .2 : .16)],
  } : {
    agent: (dark ? ["2c2f35","41464f","363a42","7a8291"] : ["eef0f3","d3d8e0","e1e5eb","9aa3b2"]).map(rgba),
    llm: (dark ? ["362248","723fa0","4c2f66","b98af0"] : ["f5eefd","ddc5f8","e9dafa","b98af0"]).map(rgba),
    tool: (dark ? ["16302d","0b8576","1e443f","4fc3b4"] : ["e4f5f3","a9ddd6","c9ebe7","4fc3b4"]).map(rgba),
    error: (dark ? ["3b1c14","743a2a","52281c"] : ["fde8e5","ff8a7a","facfc9"]).map(rgba),
  };
  const costColors = Object.fromEntries(Object.entries(costPalette).flatMap(([kind,paints]) =>
    paints.map((paint,i)=>[`cost_waterfall.${kind}_${["bg","edge","out","mark"][i]}`,paint])));
  const attemptPalette = clean ? {
    running: dark ? [alphaColor("4fb0fe",.24),alphaColor("4fb0fe",.5),rgba("8bcbfe")]
      : [alphaColor("0060e2",.12),alphaColor("0060e2",.34),rgba("0070d6")],
    done: (dark ? [.20,.40] : [.14,.32]).map(a=>alphaColor(dark ? "7dcf9f" : "5fc88a",a)).concat(rgba(dark ? "5fc88a" : "007d2ee0")),
    failed: dark ? [alphaColor("ff6b5f",.22),alphaColor("ff6b5f",.48),rgba("ff8a7a")]
      : [rgba("f52b0020"),alphaColor("f04438",.36),rgba("d72400da")],
    best: (dark ? [.32,.60] : [.24,.48]).map(a=>alphaColor(dark ? "7dcf9f" : "5fc88a",a)),
  } : {
    running: (dark ? ["1d364d","0190fe","8bcbfe"] : ["e3f3ff","a9d9ff","0190fe"]).map(rgba),
    done: (dark ? ["153222","1e8a4f","5fc88a"] : ["ddf2e6","9fd9b7","1e8a4f"]).map(rgba),
    failed: (dark ? ["3d1e16","c4320a","ff8a7a"] : ["fde3df","ff8a7a","c4320a"]).map(rgba),
    best: (dark ? ["1d4a31","3f9a68"] : ["c5ead4","5fc88a"]).map(rgba),
  };
  const attemptColors = Object.fromEntries(Object.entries(attemptPalette).flatMap(([status,paints]) =>
    paints.map((paint,i)=>[`parallel_attempts.${status}_${["bg","edge","ink"][i]}`,paint])));
  // Two card families share their regular wash; the Modern light gray
  // Project label and Project's interaction states have separate paint.
  const statusChipPaint = {
    red: [rgba(dark ? "ff8a7a" : clean ? "f04438" : "c4320a"),
      alphaColor(clean && dark ? "ff6b5f" : "f04438", clean ? (dark ? .14 : .08) : .10)],
    amber: [rgba(dark ? "e6c24d" : clean ? "8f6c00" : "b25e00"),
      alphaColor(clean && dark ? "e6c24d" : "e8b000", clean && !dark ? .10 : .14)],
    green: [rgba(dark ? (clean ? "5fc88a" : "7dcf9f") : "1e8a4f"),
      alphaColor(clean && dark ? "7dcf9f" : "5fc88a", clean ? (dark ? .14 : .08) : .11)],
    blue: [clean && !dark ? alphaColor("0060e2", .90) : rgba(dark ? (clean ? "90cdfe" : "9cd2fe") : "0190fe"),
      clean ? (dark ? alphaColor("4d8dff", .14) : alphaColor("0060e2", .06)) : alphaColor("0190fe", .09)],
    violet: [rgba(dark ? (clean ? "b98af0" : "d6baf6") : clean ? "a651f0" : "8a35d6"),
      alphaColor(clean && dark ? "b98af0" : "a651f0", clean ? (dark ? .18 : .08) : .10)],
    gray: [clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "b4b4bb" : "6b6b72"),
      clean ? (dark ? white(.08) : black(.05)) : alphaColor("6e6e78", .09)],
  };
  const projectActiveWash = {red:["f04438",.18],amber:["e8b000",.24],
    green:["5fc88a",.20],blue:["0190fe",.17],violet:["a651f0",.18],gray:["6e6e78",.17]};
  const fanoutPalette = {
    running: clean && !dark ? [alphaColor("0060e2",.12),alphaColor("0060e2",.36),rgba("0070d6")] : attemptPalette.running,
    done: clean ? (dark ? [.22,.42] : [.20,.38]).map(a=>alphaColor(dark ? "7dcf9f" : "5fc88a",a)).concat(attemptPalette.done[2])
      : attemptPalette.done,
    failed: clean ? dark ? [alphaColor("ff6b5f",.24),alphaColor("ff6b5f",.50),rgba("ff8a7a")]
      : [rgba("f52b002e"),alphaColor("f04438",.48),rgba("d72400da")] : attemptPalette.failed,
    queued: clean ? [dark ? white(.05) : black(.035),dark ? white(.09) : black(.07),muted]
      : [...(dark ? ["2b2b2f","36363b"] : ["f3f3f3","e6e6e6"]).map(rgba),muted],
    retrying: clean ? dark ? [alphaColor("f2a15c",.24),alphaColor("f2a15c",.50),rgba("ffb35c")]
      : [rgba("ff9c0030"),alphaColor("f28a2e",.45),rgba("b25e00")]
      : (dark ? ["3a2712","b25e00","ffb35c"] : ["ffecd6","ffb35c","b25e00"]).map(rgba),
  };
  const colors = {
    ...attemptColors,
    ...Object.fromEntries(Object.entries(statusChipPaint).flatMap(([tone, [ink, wash]]) =>
      [[`experiment_card.chip_${tone}_ink`, ink], [`experiment_card.chip_${tone}_wash`, wash]])),
    ...Object.fromEntries(Object.entries(statusChipPaint).flatMap(([tone, [ink, wash]]) =>
      [[`project_card.${tone}`, tone === "gray" && !clean && !dark ? rgba("5f5f66") : ink],
        [`project_card.${tone}_wash`, wash],
        [`project_card.${tone}_active`, clean ? wash : alphaColor(...projectActiveWash[tone])]])),
    "billing_schedule.card": rgba(dark ? (clean ? "1c1c1e" : "232326") : "ffffff"),
    "billing_schedule.inset": clean ? transparent : rgba(dark ? "1c1c1f" : "f7f7f7"),
    "billing_schedule.tray": clean ? transparent : rgba(dark ? "161618" : "f0f0f0"),
    "billing_schedule.line": dark ? white(clean ? .08 : .07) : clean ? black(.06) : rgba("ececec"),
    "billing_schedule.ink": ink,
    "billing_schedule.text": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "c4c4c4" : "404040"),
    "billing_schedule.muted": clean ? (dark ? white(.46) : black(.45)) : rgba("8f8f8f"),
    "billing_schedule.tint": clean ? (dark ? rgba("4fb0fe") : alphaColor("0060e2",.765)) : rgba(dark ? "c9e7ff" : "0070d6"),
    "plan_vs_actual.card": rgba(dark ? "1c1c1f" : "ffffff"),
    "plan_vs_actual.muted": dark ? white(.45) : black(.45),
    "plan_vs_actual.text": dark ? white(.62) : black(.61),
    "plan_vs_actual.quiet": dark ? white(.28) : black(.27),
    "plan_vs_actual.grid": dark ? white(.05) : black(.04),
    "plan_vs_actual.wash": dark ? white(.07) : black(.045),
    "plan_vs_actual.ghost": dark ? white(.30) : black(.26),
    "plan_vs_actual.link": dark ? white(.55) : black(.50),
    "plan_vs_actual.live": dark ? rgba("4fb0fe") : alphaColor("0060e2",.765),
    "plan_vs_actual.live_line": alphaColor(dark ? "4fb0fe" : "0060e2",dark ? .55 : .40),
    "plan_vs_actual.actual": alphaColor(dark ? "b98af0" : "a651f0",dark ? .16 : .10),
    "plan_vs_actual.actual_edge": alphaColor(dark ? "b98af0" : "a651f0",dark ? .38 : .30),
    "plan_vs_actual.actual_mark": alphaColor(dark ? "b98af0" : "a651f0",dark ? .8 : .72),
    "plan_vs_actual.forecast": dark ? white(.05) : black(.035),
    "plan_vs_actual.forecast_edge": dark ? white(.09) : black(.07),
    "plan_vs_actual.late": rgba(dark ? "ffb35c" : "b25e00"),
    "plan_vs_actual.late_fill": alphaColor(dark ? "f2a15c" : "f28a2e",dark ? .18 : .15),
    "plan_vs_actual.late_edge": alphaColor(dark ? "f2a15c" : "f28a2e",dark ? .45 : .42),
    "plan_vs_actual.late_gap": alphaColor(dark ? "f2a15c" : "f28a2e",.07),
    "plan_vs_actual.ahead": rgba(dark ? "5fc88a" : "007d2ee0"),
    "plan_vs_actual.ahead_fill": alphaColor(dark ? "7dcf9f" : "5fc88a",dark ? .14 : .10),
    "plan_vs_actual.ahead_gap": alphaColor(dark ? "7dcf9f" : "5fc88a",.06),
    "fallback_chain.tray": clean ? transparent : rgba(dark ? "1a1a1c" : "f0f0f0"),
    "fallback_chain.area": rgba(dark ? (clean ? "19191b" : "202023") : (clean ? "fbfbfb" : "ffffff")),
    "fallback_chain.card": rgba(dark ? (clean ? "232327" : "232326") : "ffffff"),
    "fallback_chain.fill": clean ? (dark ? white(.08) : black(.05)) : rgba(dark ? "2c2c30" : "f2f2f2"),
    "fallback_chain.ink": clean ? (dark ? white(.90) : black(.88)) : rgba(dark ? "f0f0f0" : "171717"),
    "fallback_chain.text": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "a1a1a1" : "6b6b6b"),
    "fallback_chain.grid": clean ? (dark ? white(.06) : black(.05)) : rgba(dark ? "2c2c30" : "ededed"),
    "fallback_chain.idle": rgba(dark ? "1f1f22" : (clean ? "f4f4f5" : "f5f5f5")),
    "fallback_chain.wait": rgba(dark ? "1f1f22" : (clean ? "f9f9fa" : "fafafa")),
    "fallback_chain.muted": clean ? (dark ? white(.45) : black(.45)) : rgba("8a8a8a"),
    "fallback_chain.quiet": clean ? (dark ? white(.28) : black(.27)) : rgba(dark ? "5c5c5c" : "c2c2c2"),
    "fallback_chain.rail": clean ? (dark ? white(.2) : black(.14)) : rgba(dark ? "3a3a3e" : "d6d6d6"),
    "fallback_chain.active": rgba(clean && !dark ? "0070d6" : "4fb0fe"),
    "fallback_chain.active_edge": clean ? alphaColor(dark ? "4fb0fe" : "0060e2",dark ? .5 : .3) : rgba(dark ? "0190fe" : "a9d9ff"),
    "fallback_chain.active_fill": clean ? alphaColor(dark ? "4fb0fe" : "0060e2",dark ? .1 : .07) : rgba(dark ? "1d364d" : "edf7ff"),
    "fallback_chain.ok": rgba(dark ? "5fc88a" : "1e8a4f"),
    "fallback_chain.ok_edge": clean ? alphaColor(dark ? "7dcf9f" : "5fc88a",dark ? .45 : .34) : rgba(dark ? "1e8a4f" : "9fd9b7"),
    "fallback_chain.ok_fill": clean ? alphaColor(dark ? "7dcf9f" : "5fc88a",dark ? .1 : .07) : rgba(dark ? "153222" : "e6f6ed"),
    "fallback_chain.error": clean ? rgba(dark ? "ff8a7a" : "c4320a") : rgba(dark ? "ffb35c" : "b25e00"),
    "fallback_chain.error_edge": clean ? alphaColor(dark ? "ff6b5f" : "f04438",dark ? .5 : .38) : rgba(dark ? "b07635" : "ffb35c"),
    "fallback_chain.error_fill": clean ? alphaColor(dark ? "ff6b5f" : "f04438",dark ? .12 : .07) : rgba(dark ? "432d14" : "fdf1e3"),
    "fallback_chain.popover": rgba(dark ? (clean ? "2a2a2f" : "2a2a2e") : "ffffff"),
    "memory_panel.tray": clean ? transparent : rgba(dark ? "111113" : "f0f0f0"),
    "memory_panel.card": rgba(dark ? (clean ? "1c1c1e" : "19191b") : "ffffff"),
    "memory_panel.well": clean ? (dark ? white(.04) : black(.03)) : rgba(dark ? "222225" : "f4f4f5"),
    "memory_panel.hover": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2c2c30" : "ececee"),
    "memory_panel.fill": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "222225" : "f4f4f5"),
    "memory_panel.ink": clean ? (dark ? white(.90) : black(.88)) : rgba(dark ? "ededed" : "171717"),
    "memory_panel.text": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "c8c8cc" : "404040"),
    "memory_panel.muted": clean ? (dark ? white(.46) : black(.45)) : rgba(dark ? "8b8b92" : "8f8f8f"),
    "memory_panel.track": clean ? (dark ? white(.14) : black(.1)) : rgba(dark ? "3a3a40" : "dcdce0"),
    "memory_panel.track_on": clean ? (dark ? rgba("4fb0fe") : alphaColor("0060e2", .765)) : rgba(dark ? "ededed" : "171717"),
    "memory_panel.knob": rgba(dark && !clean ? "19191b" : "ffffff"),
    "memory_panel.danger": rgba(dark ? "ff8a7a" : clean ? "f04438" : "c4320a"),
    "memory_panel.danger_fill": alphaColor(dark ? "ff6b5f" : "f04438", dark ? (clean ? .14 : .16) : (clean ? .08 : .10)),
    "memory_panel.paused": rgba(clean ? (dark ? "e6c24d" : "8f6c00") : (dark ? "b4b4bb" : "5f5f66")),
    "memory_panel.paused_fill": clean ? alphaColor(dark ? "e6c24d" : "e8b000", dark ? .14 : .12) : alphaColor(dark ? "a0a0aa" : "6e6e78", dark ? .14 : .10),
    "memory_panel.solid": dark ? white(.9) : black(.88),
    "memory_panel.on_solid": rgba(dark ? "1c1c1e" : "fcfcfc"),
    "tool_access.tray": clean ? transparent : rgba(dark ? "19191c" : "f0f0f0"),
    "tool_access.card": rgba(dark ? (clean ? "1c1c1f" : "232326") : "ffffff"),
    "tool_access.well": clean ? (dark ? white(.035) : black(.025)) : rgba(dark ? "28282b" : "f7f7f7"),
    "tool_access.fill": clean ? (dark ? white(.07) : black(.05)) : rgba(dark ? "2c2c30" : "f2f2f2"),
    "tool_access.ink": clean ? (dark ? white(.90) : black(.88)) : rgba(dark ? "ededed" : "171717"),
    "tool_access.text": diagramBody,
    "tool_access.muted": clean ? (dark ? white(.45) : black(.45)) : rgba(dark ? "7c7c82" : "8a8a8a"),
    "tool_access.rail": rgba(dark ? (clean ? "3a3a3f" : "3a3a40") : (clean ? "d9d9dc" : "d6d6d6")),
    "tool_access.used": rgba(dark ? "0190fe" : "83c7fe"),
    "tool_access.glint": rgba(dark ? "b9e0ff" : "4fb0fe"),
    "tool_access.live": rgba(dark ? (clean ? "70b8ff" : "8ab4ff") : (clean ? "006dcbf2" : "2f6bff")),
    "tool_access.live_fill": rgba(dark ? (clean ? "0077ff3a" : "1b2b4a") : (clean ? "008ff519" : "eaf1ff")),
    "tool_access.success": rgba(dark ? "5fc88a" : (clean ? "007d2ee0" : "1e8a4f")),
    "tool_access.success_fill": rgba(dark ? (clean ? "00f04a1f" : "163022") : (clean ? "00a43319" : "e8f6ee")),
    "tool_access.warning": rgba(dark ? "ffb35c" : "b25e00"),
    "tool_access.warning_fill": rgba(dark ? (clean ? "fb6a0025" : "3a2712") : (clean ? "ff9c0029" : "fff1e0")),
    "tool_access.danger": rgba(dark ? "ff8a7a" : (clean ? "d72400da" : "c4320a")),
    "tool_access.danger_fill": rgba(dark ? (clean ? "ff35232b" : "3b1c14") : (clean ? "f52b0018" : "fdecea")),
    "tool_access.solid": clean ? (dark ? white(.92) : black(.88)) : rgba(dark ? "ededed" : "171717"),
    "tool_access.on_solid": rgba(dark ? "141416" : (clean ? "fcfcfc" : "ffffff")),
    "trace.tray": rgba(dark ? (clean ? "1c1c1f" : "19191c") : (clean ? "ffffff" : "f0f0f0")),
    "trace.card": rgba(dark ? (clean ? "1c1c1f" : "232326") : "ffffff"),
    "trace.muted": clean ? (dark ? white(.45) : black(.45)) : rgba(dark ? "7c7c82" : "8a8a8a"),
    "trace.text": diagramBody,
    "trace.grid": clean ? (dark ? white(.05) : black(.045)) : rgba(dark ? "2a2a2e" : "f1f1f1"),
    "trace.guide": clean ? (dark ? white(.1) : black(.08)) : rgba(dark ? "34343a" : "e6e6e6"),
    "trace.hover": dark ? white(.025) : black(clean ? .02 : .022),
    "trace.selected": alphaColor(dark ? "4fb0fe" : (clean ? "0060e2" : "0190fe"), dark ? .08 : .05),
    "trace.well": clean ? (dark ? white(.03) : black(.025)) : rgba(dark ? "1d1d20" : "f7f7f7"),
    "trace.accent": clean && !dark ? alphaColor("0060e2", .765) : rgba("4fb0fe"),
    "trace.now": alphaColor(dark ? "4fb0fe" : (clean ? "0060e2" : "0190fe"), dark ? (clean ? .55 : .6) : (clean ? .45 : .55)),
    "trace.agent_bg": clean ? (dark ? white(.07) : black(.045)) : rgba(dark ? "2c2f35" : "eef0f3"),
    "trace.agent_edge": clean ? (dark ? white(.14) : black(.1)) : rgba(dark ? "41464f" : "d3d8e0"),
    "trace.agent_ink": clean ? (dark ? white(.7) : black(.61)) : rgba(dark ? "b8c0cc" : "4b5363"),
    "trace.agent_mark": clean ? (dark ? white(.4) : black(.3)) : rgba(dark ? "7a8291" : "9aa3b2"),
    "trace.llm_bg": clean ? alphaColor(dark ? "b98af0" : "a651f0", dark ? .16 : .10) : rgba(dark ? "362248" : "f5eefd"),
    "trace.llm_edge": clean ? alphaColor(dark ? "b98af0" : "a651f0", dark ? .38 : .30) : rgba(dark ? "723fa0" : "ddc5f8"),
    "trace.llm_ink": rgba(dark ? "d1b3f5" : "8a35d6"),
    "trace.llm_mark": clean ? alphaColor(dark ? "b98af0" : "a651f0", dark ? .8 : .72) : rgba("b98af0"),
    "trace.tool_bg": clean ? alphaColor(dark ? "5cc6ee" : "14b3f0", dark ? .13 : .085) : rgba(dark ? "16302d" : "e4f5f3"),
    "trace.tool_edge": clean ? alphaColor(dark ? "5cc6ee" : "14b3f0", dark ? .34 : .28) : rgba(dark ? "0b8576" : "a9ddd6"),
    "trace.tool_ink": rgba(clean ? (dark ? "5cc6ee" : "08658c") : (dark ? "4fc3b4" : "0b8576")),
    "trace.tool_mark": clean ? alphaColor(dark ? "5cc6ee" : "14b3f0", dark ? .75 : .62) : rgba("4fc3b4"),
    "trace.error_bg": rgba(clean ? (dark ? "ff35232b" : "f52b0018") : (dark ? "3b1c14" : "fde8e5")),
    "trace.error_edge": clean ? alphaColor(dark ? "ff6b5f" : "f04438", dark ? .40 : .34) : rgba(dark ? "743a2a" : "ff8a7a"),
    "trace.error_ink": rgba(dark ? "ff8a7a" : (clean ? "d72400da" : "c4320a")),
    "trace.error_mark": clean ? alphaColor(dark ? "ff6b5f" : "f04438", dark ? .75 : .62) : rgba("f04438"),
    "trace.live_bg": rgba(clean ? (dark ? "0077ff3a" : "008ff519") : (dark ? "16243f" : "eaf1ff")),
    "trace.live_ink": rgba(clean ? (dark ? "70b8ff" : "006dcbf2") : (dark ? "8ab4ff" : "2456d6")),
    "trace.done_bg": rgba(clean ? (dark ? "00f04a1f" : "00a43319") : (dark ? "123020" : "e6f6ed")),
    "trace.done_ink": rgba(dark ? "5fc88a" : (clean ? "007d2ee0" : "1e8a4f")),
    "trace.slow_bg": rgba(clean ? (dark ? "fb6a0025" : "ff9c0029") : (dark ? "3a2610" : "fff1e0")),
    "trace.slow_ink": rgba(dark ? "ffb35c" : "b25e00"),
    "recommendation.tray": rgba(clean ? (dark ? "1c1c1e" : "ffffff") : (dark ? "161618" : "f0f0f0")),
    "recommendation.card": rgba(dark ? (clean ? "1c1c1e" : "232326") : "ffffff"),
    "recommendation.paper": clean ? transparent : rgba(dark ? "232326" : "ffffff"),
    "recommendation.reason": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "c4c4c4" : "404040"),
    "recommendation.muted": clean ? (dark ? white(.62) : black(.61)) : rgba("8f8f8f"),
    "recommendation.line": clean ? (dark ? white(.08) : black(.06)) : dark ? white(.08) : rgba("e5e5e5"),
    "recommendation.hover": clean ? (dark ? white(.04) : black(.03)) : (dark ? white(.06) : rgba("f8f8f8")),
    "recommendation.chip": clean ? alphaColor(dark ? "b98af0" : "a651f0",dark ? .22 : .10) : (dark ? alphaColor("b98af0",.18) : rgba("f4f0ff")),
    "recommendation.chip_ink": rgba(dark ? "b98af0" : "a651f0"),
    "recommendation.idle": dark ? white(.14) : clean ? black(.10) : rgba("d4d4d4"),
    "recommendation.signal0": clean ? (dark ? white(.3) : black(.27)) : rgba("8f8f8f"),
    "recommendation.weak": rgba(dark && clean ? "e6c24d" : "e8b000"),
    "recommendation.good": clean && !dark ? alphaColor("0060e2",.765) : rgba("4fb0fe"),
    "recommendation.strong": rgba(dark ? (clean ? "5fc88a" : "3fb971") : "1e8a4f"),
    "recommendation.success": rgba(dark ? (clean ? "5fc88a" : "3fb971") : "1e8a4f"),
    "recommendation.success_bg": clean ? alphaColor(dark ? "7dcf9f" : "5fc88a",dark ? .14 : .08) : transparent,
    "recommendation.action": rgba(dark ? (clean ? "1c1c1e" : "2a2a2d") : "ffffff"),
    "recommendation.action_hover": clean ? (dark ? white(.04) : black(.03)) : rgba(dark ? "313135" : "f7f7f7"),
    "recommendation.solid": clean ? (dark ? white(.9) : black(.88)) : rgba(dark ? "e4e4e7" : "201e25"),
    "recommendation.on_solid": rgba(dark ? (clean ? "161618" : "121213") : (clean ? "fcfcfc" : "ffffff")),
    "recommendation.popup": rgba(dark ? (clean ? "232326" : "2a2a2d") : "ffffff"),
    "recommendation.selected": clean ? (dark ? alphaColor("4d8dff",.14) : alphaColor("0060e2",.05)) :
      (dark ? alphaColor("4fb0fe",.16) : rgba("f5f8ff")),
    "recommendation.selected_edge": clean ? alphaColor(dark ? "4d8dff" : "0060e2",dark ? .4 : .35) : transparent,
    "recommendation.tag": clean ? alphaColor(dark ? "4d8dff" : "0060e2",dark ? .16 : .08) : (dark ? alphaColor("4fb0fe",.16) : rgba("eef7ff")),
    "rate_plan.card": rgba(dark ? (clean ? "1c1c1e" : "19191b") : "ffffff"),
    "rate_plan.ink": clean ? (dark ? white(.9) : black(.88)) : rgba(dark ? "ededed" : "171717"),
    "rate_plan.muted": clean ? (dark ? white(.46) : black(.45)) : rgba(dark ? "8b8b92" : "8f8f8f"),
    "rate_plan.line": clean ? (dark ? white(.14) : black(.08)) : rgba(dark ? "2a2a2e" : "e6e6e8"),
    "rate_plan.guide": clean ? (dark ? white(.20) : black(.13)) : rgba(dark ? "3a3a40" : "d4d4d8"),
    "rate_plan.stroke": clean ? (dark ? white(.20) : black(.13)) : rgba(dark ? "3a3a40" : "d4d4d8"),
    "rate_plan.actual": clean ? (dark ? white(.46) : black(.45)) : rgba(dark ? "8b8b92" : "8f8f8f"),
    "rate_plan.wash": clean ? (dark ? white(.04) : black(.03)) : rgba(dark ? "111113" : "f0f0f0"),
    "rate_plan.accent": rgba("0190fe"),
    "rate_plan.warm": rgba(dark ? "ffb35c" : "b25e00"),
    "rate_plan.warm_bg": alphaColor(dark ? "f2a15c" : "f28a2e",dark ? .14 : .12),
    "rate_plan.danger": rgba(dark ? "ff8a7a" : "c4320a"),
    "rate_plan.warn": rgba(dark ? "ff6b5f" : "f04438"),
    "rate_plan.danger_bg": alphaColor(dark ? "ff6b5f" : "f04438",dark ? .14 : .10),
    "rate_plan.okay": rgba(dark ? "7dcf9f" : "1e8a4f"),
    "rate_plan.okay_bg": alphaColor(dark ? "7dcf9f" : "5fc88a",.14),
    "rate_plan.hatch": alphaColor(dark ? "ff6b5f" : "f04438",dark ? .34 : .30),
    "rate_plan.hatch_bg": alphaColor(dark ? "ff6b5f" : "f04438",dark ? .07 : .05),
    "rate_plan.popup": rgba(dark ? (clean ? "232326" : "19191b") : "ffffff"),
    "rate_plan.button": clean ? (dark ? white(.9) : black(.88)) : rgba(dark ? "ededed" : "171717"),
    "rate_plan.on_button": rgba(dark ? "161618" : "ffffff"),
    "rate_plan.tag": `${dark ? white(.09) : black(.06)}.over(${color("rate_plan.card")})`,
    "rate_plan.tag_over": `alpha(${color("rate_plan.warn")},${dark ? 0.16 : 0.11}).over(${color("rate_plan.card")})`,
    "rate_plan.blue": rgba("0190fe"), "rate_plan.purple": rgba(dark ? "b98af0" : "a651f0"),
    "rate_plan.teal": rgba(dark ? "4fc3b4" : "13b3a0"), "rate_plan.pink": rgba(dark ? "f477b5" : "ee3e9e"),
    "rate_plan.cyan": rgba(dark ? "5cc6ee" : "14b3f0"), "rate_plan.green": rgba("5fc88a"), "rate_plan.amber": rgba("e8b000"),
    "root_cause.tray": clean ? transparent : rgba(dark ? "19191c" : "f0f0f0"),
    "root_cause.card": rgba(dark ? (clean ? "1c1c1f" : "232326") : "ffffff"),
    "root_cause.meta": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "a1a1a6" : "6b6b6b"),
    "root_cause.muted": clean ? (dark ? white(.45) : black(.45)) : rgba(dark ? "7c7c82" : "8a8a8a"),
    "root_cause.text": diagramBody,
    "root_cause.ink": clean ? (dark ? white(.90) : black(.88)) : rgba(dark ? "ededed" : "171717"),
    "root_cause.shine": dark ? white(.35) : black(.22),
    "root_cause.faint": clean ? (dark ? white(.28) : black(.27)) : rgba(dark ? "4a4a50" : "b8b8b8"),
    "root_cause.line": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2f2f34" : "ececec"),
    "root_cause.fill": clean ? (dark ? white(.07) : black(.05)) : rgba(dark ? "2c2c30" : "f2f2f2"),
    "root_cause.well": clean ? (dark ? white(.035) : black(.024)) : rgba(dark ? "28282b" : "f7f7f7"),
    "root_cause.rail": rgba(dark ? (clean ? "3a3a3f" : "3a3a40") : (clean ? "d9d9dc" : "d6d6d6")),
    "root_cause.path": rgba(dark ? (clean ? "4fb0fe" : "0190fe") : "83c7fe"),
    "root_cause.brand": rgba(dark ? (clean ? "70b8ff" : "8ab4ff") : (clean ? "006dcbf2" : "2f6bff")),
    "root_cause.brand_bg": rgba(dark ? (clean ? "0077ff3a" : "1b2b4a") : (clean ? "008ff519" : "eaf1ff")),
    "root_cause.danger": rgba(dark ? "ff8a7a" : (clean ? "d72400da" : "c4320a")),
    "root_cause.danger_bg": rgba(dark ? (clean ? "ff35232b" : "3b1c14") : (clean ? "f52b0018" : "fdecea")),
    "root_cause.solid": clean ? (dark ? white(.92) : black(.88)) : rgba(dark ? "ededed" : "171717"),
    "root_cause.on_solid": rgba(dark ? "141416" : (clean ? "fcfcfc" : "ffffff")),
    "bracket.tray": clean ? transparent : rgba(dark ? "19191c" : "f0f0f0"),
    "bracket.card": rgba(dark ? (clean ? "1c1c1f" : "232326") : "ffffff"),
    "bracket.ink": clean ? (dark ? white(.9) : black(.88)) : rgba(dark ? "ededed" : "171717"),
    "bracket.muted": clean ? (dark ? white(.45) : black(.45)) : rgba(dark ? "7c7c82" : "8a8a8a"),
    "bracket.faint": clean ? (dark ? white(.28) : black(.27)) : rgba(dark ? "4a4a50" : "b8b8b8"),
    "bracket.line": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2f2f34" : "ececec"),
    "bracket.dash": clean ? (dark ? white(.14) : black(.12)) : rgba(dark ? "34343a" : "dcdcdf"),
    "bracket.hint": clean ? (dark ? white(.015) : black(.012)) : (dark ? white(.02) : rgba("fcfcfc")),
    "bracket.rail": rgba(dark ? "333338" : "e2e2e5"),
    "bracket.won": rgba(dark ? "5a5a61" : "b4b4ba"),
    "bracket.lost": rgba(dark ? "333338" : "e2e2e5"),
    "bracket.path": rgba(dark ? (clean ? "4fb0fe" : "0190fe") : "83c7fe"),
    "bracket.blue": rgba(dark ? (clean ? "70b8ff" : "8ab4ff") : (clean ? "006dcbf2" : "2f6bff")),
    "bracket.tag": rgba(dark ? (clean ? "0077ff3a" : "1b2b4a") : (clean ? "008ff519" : "eaf1ff")),
    "bracket.well": clean ? (dark ? white(.035) : black(.025)) : rgba(dark ? "28282b" : "f7f7f7"),
    "bracket.solid": clean ? (dark ? white(.92) : black(.88)) : rgba(dark ? "ededed" : "171717"),
    "bracket.on_solid": rgba(dark ? "141416" : (clean ? "fcfcfc" : "ffffff")),
    "bracket.shine": dark ? white(.35) : black(.22),
    "stream.tray": clean ? transparent : rgba(dark ? "19191c" : "f0f0f0"),
    "stream.card": rgba(dark ? (clean ? "1c1c1f" : "232326") : "ffffff"),
    "stream.bubble": rgba(dark ? (clean ? "1c1c1f" : "2c2c30") : (clean ? "ffffff" : "f5f5f5")),
    "stream.hover": clean ? (dark ? white(.035) : black(.025)) : rgba(dark ? "313136" : "efefef"),
    "stream.ink": clean ? (dark ? white(.90) : black(.88)) : rgba(dark ? "ededed" : "171717"),
    "stream.text": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "a1a1a6" : "5f5f5f"),
    "stream.muted": clean ? (dark ? white(.45) : black(.45)) : rgba(dark ? "7c7c82" : "8a8a8a"),
    "stream.faint": clean ? (dark ? white(.28) : black(.27)) : rgba(dark ? "4a4a50" : "b8b8b8"),
    "stream.fill": clean ? (dark ? white(.07) : black(.05)) : rgba(dark ? "34343a" : "ebebeb"),
    "stream.well": dark ? white(.05) : black(clean ? .035 : .045),
    "stream.you": clean ? (dark ? white(.05) : black(.022)) : rgba(dark ? "232326" : "ffffff"),
    "stream.line": clean ? (dark ? white(.09) : black(.06)) : rgba(dark ? "2e2e33" : "ececec"),
    "stream.accent": clean && !dark ? alphaColor("0060e2",.765) : rgba("4fb0fe"),
    "stream.solid": clean ? (dark ? white(.92) : black(.88)) : rgba(dark ? "ededed" : "171717"),
    "stream.inverse": rgba(dark ? (clean ? "141416" : "171717") : (clean ? "fcfcfc" : "ffffff")),
    "stream.wait_fg": rgba(dark ? "ffb35c" : "b25e00"),
    "stream.wait_bg": rgba(dark ? (clean ? "fb6a0025" : "3a2814") : (clean ? "ff9c0029" : "fff1e0")),
    "stream.blue": alphaColor("0060e2",.78), "stream.violet": alphaColor("a651f0",.85),
    "stream.cyan": alphaColor("14b3f0",.80), "stream.orange": alphaColor("f28a2e",.82),
    "stream.green": alphaColor("5fc88a",.80), "stream.pink": alphaColor("ee3e9e",.78),
    "stream.shine": white(dark ? .95 : .82),
    "radar.tray": clean ? transparent : rgba(dark ? "19191c" : "f0f0f0"),
    "radar.card": rgba(dark ? (clean ? "1c1c1f" : "232326") : "ffffff"),
    "radar.ink": clean ? (dark ? white(.90) : black(.88)) : rgba(dark ? "f0f0f0" : "171717"),
    "radar.text": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "a1a1a1" : "6b6b6b"),
    "radar.muted": clean ? (dark ? white(.45) : black(.45)) : rgba("8a8a8a"),
    "radar.faint": clean ? (dark ? white(.28) : black(.27)) : rgba(dark ? "5c5c5c" : "c2c2c2"),
    "radar.line": clean ? (dark ? white(.10) : black(.06)) : rgba(dark ? "2e2e31" : "ececec"),
    "radar.fill": dark ? white(.07) : black(clean ? .06 : .05),
    "radar.hover": clean ? (dark ? white(.05) : black(.03)) : rgba(dark ? "28282b" : "fafafa"),
    "radar.stripe": clean ? (dark ? white(.016) : black(.011)) : rgba(dark ? "252528" : "fcfcfc"),
    "radar.edit": clean ? alphaColor(dark ? "0190fe" : "0060e2",dark ? .18 : .12) : rgba(dark ? "1d364d" : "e3f3ff"),
    "radar.edit_edge": clean ? alphaColor(dark ? "0190fe" : "0060e2",dark ? .50 : .32) : rgba(dark ? "0190fe" : "a9d9ff"),
    "radar.hit": clean ? alphaColor(dark ? "f2a15c" : "f28a2e",dark ? .20 : .22) : rgba(dark ? "432d14" : "fde8cf"),
    "radar.hit_edge": clean ? alphaColor(dark ? "f2a15c" : "f28a2e",dark ? .55 : .60) : rgba(dark ? "b07635" : "ffb35c"),
    "radar.hit_ink": rgba(dark ? "ffb35c" : "b25e00"),
    "radar.hit_row": clean ? alphaColor(dark ? "f2a15c" : "f28a2e",dark ? .08 : .06) : rgba(dark ? "332618" : "fff8ef"),
    "radar.kept": clean ? alphaColor(dark ? "7dcf9f" : "5fc88a",dark ? .18 : .20) : rgba(dark ? "153222" : "ddf2e6"),
    "radar.kept_edge": clean ? alphaColor(dark ? "7dcf9f" : "5fc88a",dark ? .50 : .40) : rgba(dark ? "1e8a4f" : "9fd9b7"),
    "radar.kept_ink": rgba(dark ? "7dcf9f" : (clean ? "2ea865" : "1e8a4f")),
    "radar.accent": rgba(dark ? "8bcbfe" : "4fb0fe"),
    "radar.chip": clean ? (dark ? white(.92) : black(.88)) : rgba(dark ? "f5f5f5" : "171717"),
    "radar.inverse": rgba(dark ? (clean ? "111113" : "171717") : (clean ? "fcfcfc" : "ffffff")),
    "radar.sheen": white(dark ? .16 : .80),
    ...Object.fromEntries(Object.entries(fanoutPalette).flatMap(([status,paints]) =>
      paints.map((paint,i)=>[`fan_out_grid.${status}_${["bg","edge","ink"][i]}`,paint]))),
    "fan_out_grid.tray": clean ? transparent : rgba(dark ? "1a1a1c" : "f0f0f0"),
    "fan_out_grid.paper": rgba(dark ? (clean ? "1c1c1f" : "232326") : "ffffff"),
    "fan_out_grid.strong": rgba(dark ? (clean ? "ffffff" : "f5f5f5") : (clean ? "000000" : "171717")),
    "fan_out_grid.ink": clean ? (dark ? white(.9) : black(.88)) : rgba(dark ? "f0f0f0" : "171717"),
    "fan_out_grid.muted": clean ? (dark ? white(.45) : black(.45)) : rgba("8a8a8a"),
    "fan_out_grid.filter_wash": dark ? white(clean ? .08 : .06) : black(clean ? .06 : .05),
    "fan_out_grid.mark": clean && !dark ? alphaColor("0060e2",.765) : rgba("4fb0fe"),
    "fan_out_grid.pin": clean ? alphaColor(dark ? "4fb0fe" : "0060e2",dark ? .45 : .35)
      : dark ? white(.30) : alphaColor("171717",.22),
    "fan_out_grid.shine": white(dark ? (clean ? .3 : .28) : (clean ? .85 : .9)),
    "parallel_attempts.track": clean ? (dark ? white(.05) : black(.035)) : rgba(dark ? "2b2b2f" : "f2f2f2"),
    "parallel_attempts.mark": clean && !dark ? alphaColor("0060e2",.765) : rgba("4fb0fe"),
    "parallel_attempts.shine": white(dark ? (clean ? .3 : .28) : (clean ? .85 : .9)),
    "parallel_attempts.icon": rgba(dark ? (clean ? "232326" : "2a2a2e") : "ffffff"),
    "parallel_attempts.strong": rgba(dark ? (clean ? "ffffff" : "f5f5f5") : (clean ? "000000" : "171717")),
    "parallel_attempts.ink": clean ? (dark ? white(.9) : black(.88)) : rgba(dark ? "f0f0f0" : "171717"),
    "parallel_attempts.text": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "a1a1a1" : "6b6b6b"),
    "parallel_attempts.muted": clean ? (dark ? white(.45) : black(.45)) : rgba("8a8a8a"),
    "parallel_attempts.action_hover": dark ? white(clean ? .08 : .06) : black(clean ? .06 : .05),
    "parallel_attempts.tray": clean ? transparent : rgba(dark ? "1a1a1c" : "f0f0f0"),
    "parallel_attempts.card": rgba(dark ? (clean ? "1c1c1f" : "232326") : "ffffff"),
    "parallel_attempts.wash": clean ? alphaColor(dark ? "7dcf9f" : "5fc88a",dark ? .06 : .035) : rgba(dark ? "173022" : "f3fbf6"),
    surface: canvas, surface_raised: panel, surface_hover: wash,
    text_primary: ink, text_muted: muted,
    accent: rgba(dark ? "8ab4ff" : "2456d6"),
    accent_hover: rgba(dark ? "a8c7ff" : "1746ba"), on_accent: rgba(dark ? "121213" : "ffffff"),
    danger: rgba(semantic.danger[1]), on_danger: rgba(dark ? "161618" : "ffffff"),
    warning: rgba(semantic.warning[1]), on_warning: rgba(dark ? "161618" : "ffffff"),
    success: rgba(semantic.success[1]), on_success: rgba(dark ? "161618" : "ffffff"),
    border: edge, focus_ring: focus, selection: selected, disabled: faint,
    "text.strongest": strongest, "text.secondary": muted, "text.subtle": faint,
    "text.placeholder": placeholder, "text.icon": clean ? (dark ? white(.5) : black(.447)) : muted,
    "surface.field": field, "surface.card": panel, "surface.chrome": panel,
    "surface.tray": rgba(dark ? "151517" : "f4f4f5"),
    "card.tray": clean ? panel : rgba(dark ? "111113" : "f0f0f0"),
    "card.body": clean ? ink : rgba(dark ? "c8c8cc" : "404040"),
    "card.description": clean ? muted : rgba(dark ? "a1a1a6" : "6b6b6b"),
    "card.selected": clean ? `mix(${panel},${ink},${dark ? .06 : .035})` : rgba(dark ? "26262a" : "e6e6e6"),
    "date_range.tray": clean ? panel : rgba(dark ? "111113" : "f0f0f0"),
    "date_range.preset": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2e" : "ebebed"),
    "date_range.chip": clean ? (dark ? white(.9) : black(.88)) : rgba(dark ? "4fb0fe" : "0190fe"),
    "date_range.on_chip": rgba(dark ? (clean ? "111113" : "0d0d10") : (clean ? "fcfcfc" : "ffffff")),
    "date_range.band": clean ? (dark ? white(.08) : black(.06)) : `alpha(rgb(0x${dark ? "4fb0fe" : "0190fe"}),${dark ? .18 : .1})`,
    "date_range.preview": clean ? (dark ? white(.05) : black(.04)) : `alpha(rgb(0x${dark ? "4fb0fe" : "0190fe"}),${dark ? .10 : .06})`,
    "date_range.comparison": clean ? (dark ? "alpha(rgb(0x4d8dff),0.14)" : "alpha(rgb(0x0060e2),0.08)") : rgba(dark ? "26262a" : "ececee"),
    "date_range.comparison_edge": clean ? (dark ? "alpha(rgb(0x4d8dff),0.55)" : "alpha(rgb(0x0060e2),0.4)") : rgba(dark ? "4a4a50" : "b9b9be"),
    "tag_input.chip": clean ? (dark ? white(.07) : black(.045)) : rgba(dark ? "232326" : "f4f4f5"),
    "tag_input.chip_edge": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2f2f34" : "e6e6e8"),
    "tag_input.armed": clean ? (dark ? white(.14) : black(.10)) : rgba(dark ? "303035" : "e4e4e7"),
    "tag_input.avatar": clean ? (dark ? white(.12) : black(.08)) : rgba(dark ? "2c2c31" : "e9e9ec"),
    "tag_input.focus": clean ? (dark ? white(.30) : black(.28)) : rgba(dark ? "4e4e55" : "b4b4bb"),
    "tag_input.flash": dark ? "alpha(rgb(0x4d8dff),0.28)" : "alpha(rgb(0x0060e2),0.14)",
    "tag_input.error_edge": clean
      ? (dark ? "alpha(rgb(0xff6b5f),0.50)" : "alpha(rgb(0xf04438),0.55)")
      : (dark ? "alpha(rgb(0xff6b5f),0.45)" : rgba("ff8a7a")),
    "tag_input.error_fill": clean ? field : `mix(${field},rgb(0x${dark ? "ff6b5f" : "f04438"}),${dark ? .06 : .04})`,
    "filter_menu.tray": clean ? field : rgba(dark ? "161618" : "f0f0f0"),
    "filter_menu.card": rgba(dark ? (clean ? "1c1c1e" : "232326") : "ffffff"),
    "filter_menu.chip": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2d" : "f3f3f3"),
    "filter_menu.chip_hover": clean ? (dark ? white(.13) : black(.1)) : rgba(dark ? "313135" : "ebebeb"),
    "filter_menu.choice": clean ? field : rgba(dark ? "2a2a2d" : "f0f0f0"),
    "filter_menu.choice_edge": clean ? (dark ? white(.28) : black(.22)) : (dark ? white(.1) : rgba("e3e3e3")),
    "filter_menu.choice_on": clean ? (dark ? rgba("4fb0fe") : "alpha(rgb(0x0060e2),0.765)") : (dark ? rgba("f4f4f5") : rgba("323137")),
    "filter_menu.mark": clean ? rgba("ffffff") : rgba(dark ? "121213" : "ffffff"),
    "filter_menu.hover": clean ? (dark ? white(.04) : black(.03)) : (dark ? white(.05) : rgba("f7f7f7")),
    "stepper.tray": clean ? field : rgba(dark ? "111113" : "f0f0f0"),
    "stepper.card": field,
    "stepper.track": clean ? (dark ? white(.08) : black(.06)) : edge,
    "stepper.progress": clean ? (dark ? rgba("4fb0fe") : "alpha(rgb(0x0060e2),0.765)") : ink,
    "stepper.success_fill": dark ? "alpha(rgb(0x5fc88a),0.14)" : "alpha(rgb(0x5fc88a),0.10)",
    "checklist.track": clean ? (dark ? white(.09) : black(.06)) : rgba(dark ? "2a2a2e" : "f0f0f0"),
    "checklist.tracker_fill": clean ? (dark ? rgba("4fb0fe") : "alpha(rgb(0x0060e2),0.765)") : ink,
    "checklist.setup_fill": clean && !dark ? "alpha(rgb(0x0060e2),0.765)" : rgba("4fb0fe"),
    "checklist.ring": clean ? (dark ? white(.28) : black(.2)) : rgba(dark ? "8b8b92" : "8f8f8f"),
    "checklist.muted": clean ? faint : rgba(dark ? "a1a1a6" : "a3a3a3"),
    "checklist.open": clean ? (dark ? white(.06) : black(.035)) : rgba(dark ? "232326" : "f6f6f7"),
    "checklist.locked_icon": clean ? (dark ? white(.30) : black(.27)) : rgba(dark ? "4a4a50" : "d4d4d4"),
    "checklist.current_dot": rgba(dark ? "3a3a3f" : "e1e3e8"),
    "reaction.ink": clean ? muted : ink,
    "reaction.like": clean ? (dark ? rgba("4fb0fe") : black(.88)) : rgba(dark ? "f06db5" : "ee3e9e"),
    "reaction.comment": rgba("4fb0fe"),
    "reaction.repost": clean ? (dark ? rgba("4fb0fe") : black(.88)) : rgba(dark ? "b98af0" : "a651f0"),
    "reaction.share": ink,
    "tracker.sheen": white(.28),
    "reaction.selected": dark ? rgba("4fb0fe") : "alpha(rgb(0x0060e2),0.765)",
    "reaction.selected_fill": dark ? "alpha(rgb(0x4fb0fe),0.09)" : "alpha(rgb(0x0060e2),0.05)",
    "reaction.selected_edge": dark ? "alpha(rgb(0x4fb0fe),0.35)" : "alpha(rgb(0x0060e2),0.35)",
    "cite.highlight": clean ? (dark ? "alpha(rgb(0x4d8dff),0.12)" : "alpha(rgb(0x0060e2),0.07)") :
        (dark ? "alpha(rgb(0x4fb0fe),0.16)" : "alpha(rgb(0x0190fe),0.10)"),
    "cite.well": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "222225" : "f4f4f5"),
    "cite.tray": clean ? transparent : rgba(dark ? "111113" : "f0f0f0"),
    "agent_tree.link": rgba(dark ? "4fb0fe" : "83c7fe"),
    "agent_tree.link_muted": rgba(dark ? "3a3a40" : "d6d6d6"),
    "agent_recap.area": clean ? (dark ? white(.08) : black(.07)) : (dark ? white(.07) : `alpha(${rgba("171717")},0.06)`),
    "agent_recap.cursor": clean ? (dark ? white(.28) : black(.22)) : rgba(dark ? "45454c" : "d4d4d4"),
    "agent_recap.dot": clean ? (dark ? white(.65) : black(.55)) : rgba(dark ? "d4d4d8" : "525252"),
    "agent_recap.failed": rgba(dark ? (clean ? "ff6b5f" : "f2a15c") : (clean ? "f04438" : "b25e00")),
    "agent_recap.grid": clean ? (dark ? white(.05) : black(.045)) : rgba(dark ? "2a2a2e" : "f0f0f0"),
    "agent_recap.needs": rgba(dark ? "f2a15c" : (clean ? "f28a2e" : "e6730e")),
    "agent_recap.stroke": clean ? (dark ? white(.5) : black(.42)) : rgba(dark ? "8a8a93" : "8a8a8a"),
    "agent_recap.panel": rgba(dark ? (clean ? "1c1c1f" : "1f1f22") : "ffffff"),
    "agent_recap.tray": clean ? rgba(dark ? "161618" : "f4f4f5") : rgba(dark ? "141416" : "f0f0f0"),
    "agent_recap.text": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "a1a1aa" : "6b6b6b"),
    "agent_recap.quiet": clean ? (dark ? white(.45) : black(.45)) : rgba(dark ? "8a8a93" : "8a8a8a"),
    "agent_recap.faint": clean ? (dark ? white(.28) : black(.27)) : rgba(dark ? "5a5a62" : "b5b5b5"),
    "agent_recap.soft": clean ? (dark ? white(.07) : black(.05)) : rgba(dark ? "2a2a2e" : "f4f4f4"),
    "agent_recap.ok": rgba(dark ? (clean ? "2fb26a" : "5fc88a") : "1e8a4f"),
    "agent_recap.ok_bg": clean ? (dark ? "alpha(rgb(0x7dcf9f),0.16)" : "alpha(rgb(0x5fc88a),0.10)")
      : (dark ? "alpha(rgb(0x7dcf9f),0.14)" : rgba("e6f6ed")),
    "agent_findings.panel": panel,
    "agent_findings.tray": clean ? panel : rgba(dark ? "111113" : "f0f0f0"),
    "agent_findings.line": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2e" : "ececec"),
    "agent_findings.fill": clean ? (dark ? white(.08) : black(.06)) : (dark ? white(.08) : rgba("ececec")),
    "agent_findings.hover": clean ? (dark ? white(.04) : black(.025)) : (dark ? white(.04) : rgba("f6f6f6")),
    "agent_findings.selected": dark ? white(.06) : (clean ? black(.04) : rgba("f2f2f3")),
    "agent_findings.quiet": clean ? (dark ? white(.45) : black(.45)) : (dark ? white(.45) : rgba("8a8a8a")),
    "agent_findings.faint": clean ? (dark ? white(.28) : black(.27)) : (dark ? white(.28) : rgba("b8b8b8")),
    "agent_findings.critical": rgba(dark ? "ff8a7a" : "c4320a"),
    "agent_findings.warning": rgba(dark ? "ffb35c" : "b25e00"),
    "agent_findings.note": clean && !dark ? "alpha(rgb(0x0060e2),0.8)" : rgba("4fb0fe"),
    "agent_findings.spark": dark ? rgba("4fb0fe") : "alpha(rgb(0x0060e2),0.765)",
    "agent_findings.run": clean ? (dark ? "alpha(rgb(0x4fb0fe),0.18)" : "alpha(rgb(0x0060e2),0.1)")
      : (dark ? rgba("0077ff3a") : "alpha(rgb(0x0190fe),0.12)"),
    "agent_findings.run_edge": clean ? (dark ? "alpha(rgb(0x4fb0fe),0.5)" : "alpha(rgb(0x0060e2),0.32)")
      : (dark ? "alpha(rgb(0x4fb0fe),0.6)" : "alpha(rgb(0x0190fe),0.55)"),
    "agent_findings.done": clean ? (dark ? white(.08) : black(.05)) : (dark ? white(.10) : rgba("e4e4e6")),
    "agent_findings.done_edge": clean ? (dark ? white(.14) : black(.10)) : (dark ? white(.16) : rgba("d6d6d9")),
    "agent_findings.failed": rgba(dark ? "ff35232b" : clean ? "f52b0018" : "fdecea"),
    "agent_findings.failed_edge": dark ? "alpha(rgb(0xff6b5f),0.45)" : `alpha(rgb(0xf04438),${clean ? .34 : .45})`,
    "agent_findings.sheen": dark ? white(.14) : rgba("ffffff"),
    "agent_findings.avatar_ink": dark ? white(.62) : (clean ? black(.61) : rgba("6b6b6b")),
    "agent_ledger.tray": clean ? transparent : rgba(dark ? "1a1a1c" : "f0f0f0"),
    "agent_ledger.panel": rgba(dark ? (clean ? "1c1c1f" : "232326") : "ffffff"),
    "agent_ledger.icon": rgba(dark ? (clean ? "232326" : "2a2a2e") : "ffffff"),
    "agent_ledger.text": dark ? (clean ? white(.62) : rgba("a1a1a1")) : (clean ? black(.61) : rgba("6b6b6b")),
    "agent_ledger.quiet": clean ? (dark ? white(.45) : black(.45)) : rgba("8a8a8a"),
    "agent_ledger.faint": clean ? (dark ? white(.28) : black(.27)) : rgba(dark ? "5c5c5c" : "c2c2c2"),
    "agent_ledger.rule": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2e2e31" : "f0f0f0"),
    "agent_ledger.hover": clean ? (dark ? white(.04) : black(.015)) : (dark ? white(.04) : rgba("fafafa")),
    "agent_ledger.preview": clean ? (dark ? "alpha(rgb(0x4fb0fe),0.1)" : "alpha(rgb(0x0060e2),0.06)") : rgba(dark ? "1b2a38" : "f2f6ff"),
    "agent_ledger.also": clean ? (dark ? "alpha(rgb(0x4fb0fe),0.07)" : "alpha(rgb(0x0060e2),0.045)") : rgba(dark ? "192835" : "f5f8ff"),
    "agent_ledger.also_chip": clean ? (dark ? "alpha(rgb(0x4fb0fe),0.13)" : "alpha(rgb(0x0060e2),0.09)") : rgba(dark ? "1d364d" : "e3f3ff"),
    "agent_ledger.also_ink": rgba(dark ? "8bcbfe" : clean ? "0070d6" : "0190fe"),
    "agent_ledger.kept": clean ? (dark ? "alpha(rgb(0xf2a15c),0.09)" : rgba("ff9c0014")) : rgba(dark ? "2d2216" : "fff7ee"),
    "agent_ledger.kept_chip": clean ? (dark ? "alpha(rgb(0xf2a15c),0.15)" : rgba("ff9c0026")) : rgba(dark ? "3a2712" : "ffecd6"),
    "agent_ledger.kept_ink": rgba(dark ? "ffb35c" : "b25e00"),
    "agent_ledger.strike": clean ? (dark ? white(.5) : black(.45)) : rgba("8a8a8a"),
    "agent_ledger.accent": clean && !dark ? "alpha(rgb(0x0060e2),0.765)" : rgba("4fb0fe"),
    "compact_breakdown.c1": clean && !dark ? "alpha(rgb(0x0060e2),0.765)" : rgba("4fb0fe"),
    "compact_breakdown.c2": clean ? (dark ? "alpha(rgb(0x4d8dff),0.62)" : "alpha(rgb(0x0060e2),0.5)") : rgba("4fb0fe"),
    "compact_breakdown.c3": clean ? (dark ? "alpha(rgb(0x4d8dff),0.4)" : "alpha(rgb(0x0060e2),0.32)") : rgba(dark ? "0070d6" : "b4deff"),
    "compact_breakdown.c4": clean ? (dark ? "alpha(rgb(0x4d8dff),0.22)" : "alpha(rgb(0x0060e2),0.18)") : rgba(dark ? "6b6b73" : "a8a8ae"),
    "compact_breakdown.c5": clean ? (dark ? white(.16) : black(.14)) : rgba(dark ? "3d3d42" : "d4d4d8"),
    "compact_breakdown.muted": clean ? (dark ? white(.46) : black(.45)) : rgba(dark ? "8b8b92" : "8f8f8f"),
    "compact_breakdown.card": rgba(dark ? (clean ? "1c1c1e" : "19191b") : "ffffff"),
    "compact_breakdown.up": rgba(dark ? (clean ? "5fc88a" : "7dcf9f") : "1e8a4f"),
    "compact_breakdown.down": rgba(dark ? (clean ? "ff6b5f" : "ff8a7a") : (clean ? "f04438" : "c4320a")),
    "compact_breakdown.up_bg": alphaColor(dark ? "7dcf9f" : "5fc88a",dark ? (clean ? .14 : .12) : clean ? .08 : .10),
    "compact_breakdown.down_bg": alphaColor(dark ? "ff6b5f" : "f04438",dark ? (clean ? .14 : .12) : .08),
    "usage_meter.accent": clean && !dark ? "alpha(rgb(0x0060e2),0.765)" : rgba("4fb0fe"),
    "usage_meter.track": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2e" : "efeff1"),
    "usage_meter.spark": clean ? (dark ? white(.16) : black(.12)) : rgba(dark ? "3a3a40" : "dcdce0"),
    "usage_meter.warn": rgba(dark ? "e6c24d" : "e8b000"),
    "usage_meter.warn_text": rgba(dark ? (clean ? "e6c24d" : "f2a15c") : (clean ? "8f6c00" : "b25e00")),
    "usage_meter.full": rgba(dark ? "ff6b5f" : "f04438"),
    "usage_meter.full_text": rgba(dark ? "ff8a7a" : "f04438"),
    "usage_meter.fill": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "d1d1d7" : "4a4a4f"),
    "change_summary.icon": clean ? (dark ? white(.5) : black(.447)) : rgba("4fb0fe"),
    "change_summary.good": rgba(dark ? (clean ? "5fc88a" : "41bd74") : "1e8a4f"),
    "change_summary.bad": rgba(dark ? "ff6b5f" : "f04438"),
    "tips_panel.done": clean ? rgba(dark ? "5fc88a" : "1e8a4f") : rgba("4fb0fe"),
    "project_card.accent": clean && !dark ? "alpha(rgb(0x0060e2),0.765)" : rgba("4fb0fe"),
    "notification_stack.art": clean ? (dark ? white(.07) : black(.07)) : (dark ? white(.08) : rgba("e3e3e3")),
    "notification_stack.art_dot": clean ? color("accent") : rgba(dark ? "444449" : "d4d4d4"),
    "notification_stack.badge": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2d" : "ffffff"),
    "notification_stack.peek": rgba(dark ? (clean ? "29292d" : "2a2a2e") : (clean ? "f6f6f7" : "f4f4f5")),
    "away_summary.needs": rgba(dark ? "e6c24d" : "e8b000"),
    "away_summary.done": rgba(dark ? "4fb0fe" : "0190fe"),
    "away_summary.failed": rgba(dark ? "ff6b5f" : "f04438"),
    "away_summary.muted": clean ? (dark ? white(.45) : black(.45)) : rgba(dark ? "7c7c82" : "8a8a8a"),
    "away_summary.text": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "a1a1a6" : "6b6b6b"),
    "away_summary.tray": rgba(dark ? "19191c" : "f0f0f0"),
    "away_summary.card": clean && dark ? rgba("1c1c1f") : rgba(dark ? "232326" : "ffffff"),
    "agent_steering.tag": clean && !dark ? alphaColor("0060e2", .765) : rgba("4fb0fe"),
    "agent_steering.note_bg": clean ? (dark ? white(.04) : black(.03)) : rgba(dark ? "232326" : "f5f5f5"),
    "kpi_chart.first": rgba(dark ? "60a5fa" : "2563eb"),
    "kpi_chart.second": rgba("a3a3a3"),
    "kpi_chart.third": rgba(dark ? "c084fc" : "9333ea"),
    "kpi_chart.grid": clean ? (dark ? white(.08) : black(.06)) : (dark ? white(.06) : rgba("f0f0f0")),
    "kpi_chart.guide": dark ? white(.2) : (clean ? black(.12) : rgba("d9d9d9")),
    "breakdown_chart.c1": rgba("4fb0fe"),
    "breakdown_chart.c2": rgba("a651f0"),
    "breakdown_chart.c3": rgba("ee3e9e"),
    "breakdown_chart.c4": rgba("7bc4fe"),
    "breakdown_chart.c5": rgba("d7bcf6"),
    "breakdown_chart.c6": rgba("a3a3a3"),
    "breakdown_chart.c7": rgba("f6abd4"),
    "breakdown_chart.grid": dark ? white(.08) : (clean ? black(.06) : rgba("ebebeb")),
    "breakdown_chart.muted": clean ? (dark ? white(.46) : black(.45)) : rgba("8f8f8f"),
    "breakdown_chart.rest": dark ? white(.08) : (clean ? black(.06) : rgba("f2f2f2")),
    "breakdown_chart.zero": dark ? white(.18) : (clean ? black(.12) : rgba("dcdcdc")),
    "running_now.teal": rgba(dark ? "4fc3b4" : "13b3a0"),
    "running_now.amber": rgba(dark ? "e6c24d" : "e8b000"),
    "running_now.orange": rgba(dark ? "f2a15c" : "f28a2e"),
    "running_now.blue": rgba(dark ? "4fb0fe" : "0190fe"),
    "running_now.badge": clean ? (dark ? "alpha(rgb(0xe6c24d),0.16)" : "alpha(rgb(0xe8b000),0.14)") : rgba(dark ? "332a0f" : "f9f1d6"),
    "running_now.badge_text": rgba(dark ? "e6c24d" : "8f6c00"),
    "running_now.trigger_ink": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "7c7c82" : "8a8a8a"),
    "running_now.paper": rgba(dark ? (clean ? "1c1c1e" : "19191c") : (clean ? "ffffff" : "f0f0f0")),
    "running_now.faint": clean ? (dark ? white(.30) : black(.27)) : rgba(dark ? "4a4a50" : "b8b8b8"),
    "scheduled_runs.capacity": rgba(dark ? "66cbbb" : "37ae9d"),
    "scheduled_runs.current": rgba(dark ? "ff6b5f" : "f04438"),
    "scheduled_runs.drop": clean ? (dark ? white(.08) : black(.04)) : color("selection"),
    "scheduled_runs.hatch": dark ? white(.06) : black(.05),
    "scheduled_runs.lane": rgba(dark ? "161618" : (clean ? "f4f4f5" : "f4f4f4")),
    "scheduled_runs.card": rgba(dark ? "1c1c1f" : "ffffff"),
    "scheduled_runs.ink": dark ? white(.90) : clean ? black(.88) : rgba("171717"),
    "scheduled_runs.track": rgba(dark ? "363639" : "e7e7e8"),
    "scheduled_runs.well": dark ? white(.03) : clean ? black(.024) : rgba("fafafa"),
    "scheduled_runs.pill": dark ? white(.07) : black(clean ? .05 : .06),
    "scheduled_runs.count_bg": alphaColor("a651f0", .06),
    "scheduled_runs.muted": dark ? white(.45) : clean ? black(.45) : rgba("8a8a8a"),
    "scheduled_runs.text": dark ? white(.62) : clean ? black(.61) : rgba("6b6b6b"),
    "scheduled_runs.faint": dark ? white(.28) : clean ? black(.27) : rgba("b5b5b5"),
    "agent_cursors.card": rgba(dark ? (clean ? "1c1c1f" : "1f1f22") : "ffffff"),
    "agent_cursors.ink": rgba(dark ? "e4e4e7" : "1f1f22"),
    "agent_cursors.tray": rgba(dark ? "161618" : "f0f0f0"),
    "agent_cursors.muted": clean ? (dark ? white(.45) : black(.45)) : rgba(dark ? "7c7c82" : "8a8a8a"),
    "agent_cursors.tab": dark ? white(.06) : black(.05),
    "agent_cursors.tab_ink": ink,
    "agent_cursors.face_edge": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2e2e33" : "ececec"),
    "tool_call.tile": rgba(dark ? (clean ? "252527" : "1d1d20") : "f5f5f5"),
    "tool_call.code": rgba(dark ? (clean ? "212123" : "1d1d20") : "fafafa"),
    "tool_call.error": `alpha(rgb(0x${dark ? "ff6b5f" : "f04438"}),${dark ? (clean ? 0.07 : 0.08) : (clean ? 0.04 : 0.06)})`,
    "tool_call.error_border": `alpha(rgb(0x${dark ? "ff6b5f" : "f04438"}),${dark ? (clean ? 0.18 : 0.20) : (clean ? 0.14 : 0.16)})`,
    "context_window.c1": clean ? (dark ? rgba("4fb0fe") : "alpha(rgb(0x0060e2),0.765)") : rgba(dark ? "e4e4e7" : "26262b"),
    "context_window.c2": clean ? (dark ? "alpha(rgb(0x4d8dff),0.7)" : "alpha(rgb(0x0060e2),0.5)") : rgba(dark ? "8a8a92" : "8d8d95"),
    "context_window.c3": clean ? (dark ? "alpha(rgb(0x4d8dff),0.46)" : "alpha(rgb(0x0060e2),0.26)") : rgba(dark ? "55555c" : "c4c4ca"),
    "context_window.c4": clean ? (dark ? rgba("b98af0") : "alpha(rgb(0xa651f0),0.8)") : rgba("4fb0fe"),
    "context_window.c5": clean ? (dark ? rgba("5fc88a") : "alpha(rgb(0x5fc88a),0.72)") : rgba(dark ? "90cdfe" : "a9d9ff"),
    "context_window.c6": clean ? (dark ? rgba("e6c24d") : "alpha(rgb(0xe8b000),0.8)") : rgba("5f5f66"),
    "context_window.free": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2e" : "ededf0"),
    "context_window.shine": white(dark ? 0.9 : 0.82),
    "context_window.warn": rgba(dark ? "e6c24d" : clean ? "8f6c00" : "b25e00"),
    "context_window.warn_bg": `alpha(rgb(0x${dark ? "e6c24d" : "e8b000"}),${clean ? (dark ? .16 : .12) : .14})`,
    "context_window.full_bg": `alpha(rgb(0x${dark ? "ff6b5f" : "f04438"}),${clean ? (dark ? .14 : .08) : .10})`,
    "context_window.saved": rgba(dark ? (clean ? "5fc88a" : "7dcf9f") : "1e8a4f"),
    "context_window.muted": clean ? (dark ? white(.46) : black(.45)) : rgba(dark ? "8b8b92" : "8f8f8f"),
    "context_map.model": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "a1a1a1" : "6b6b6b"),
    "context_map.muted": clean ? (dark ? white(.45) : black(.45)) : rgba("8a8a8a"),
    "context_map.empty": clean ? (dark ? white(.28) : black(.27)) : rgba(dark ? "5c5c5c" : "b8b8b8"),
    "context_map.tray": rgba(dark ? "1a1a1c" : "f0f0f0"),
    "context_map.shine": clean ? (dark ? white(.35) : black(.22)) : rgba(dark ? "8a8a8a" : "b8b8b8"),
    "context_map.panel": rgba(dark ? (clean ? "1c1c1f" : "232326") : "ffffff"),
    "context_map.capacity_bg": clean ? black(.05) : rgba(dark ? "2a2a2e" : "ffffff"),
    "context_map.capacity_edge": rgba(dark ? "2e2e31" : "ececec"),
    "context_map.capacity_warn_bg": clean ? (dark ? "alpha(rgb(0xf2a15c),0.18)" : rgba("ff9c0024")) : rgba(dark ? "3a2712" : "ffecd6"),
    "context_map.capacity_full_bg": clean ? (dark ? "alpha(rgb(0xff6b5f),0.2)" : rgba("f52b0018")) : rgba(dark ? "3d1e16" : "fde3df"),
    "context_map.capacity_full_ink": dark ? rgba("ff8a7a") : rgba(clean ? "d72400da" : "c4320a"),
    "context_map.free_edge": clean ? (dark ? white(.16) : black(.14)) : rgba(dark ? "3a3a40" : "d6d6d6"),
    "context_map.free_hatch": dark ? white(.04) : black(.035),
    "context_map.system_bg": clean ? (dark ? white(.07) : black(.05)) : rgba(dark ? "2b2b2f" : "f3f3f3"),
    "context_map.system_edge": clean ? (dark ? white(.14) : black(.11)) : rgba(dark ? "3a3a40" : "e1e1e1"),
    "context_map.system_ink": clean ? (dark ? white(.7) : black(.61)) : rgba(dark ? "bdbdbd" : "5c5c5c"),
    "context_map.tools_bg": clean ? `alpha(rgb(0x${dark ? "5cc6ee" : "14b3f0"}),${dark ? .16 : .085})` : rgba(dark ? "122f3b" : "e2f3f9"),
    "context_map.tools_edge": clean ? `alpha(rgb(0x${dark ? "5cc6ee" : "14b3f0"}),${dark ? .42 : .28})` : rgba(dark ? "0a85b8" : "a6d7ea"),
    "context_map.tools_ink": rgba(dark ? "5cc6ee" : "08658c"),
    "context_map.memory_bg": clean ? `alpha(rgb(0x${dark ? "b98af0" : "a651f0"}),${dark ? .18 : .10})` : rgba(dark ? "38244a" : "f3ecfc"),
    "context_map.memory_edge": clean ? `alpha(rgb(0x${dark ? "b98af0" : "a651f0"}),${dark ? .45 : .30})` : rgba(dark ? "8a35d6" : "d4bdf0"),
    "context_map.memory_ink": rgba(dark ? "d7bcf6" : "8a35d6"),
    "context_map.file_bg": clean ? `alpha(rgb(0x${dark ? "4fb0fe" : "0060e2"}),${dark ? .18 : .08})` : rgba(dark ? "1d364d" : "e6f3fd"),
    "context_map.file_edge": clean ? `alpha(rgb(0x${dark ? "4fb0fe" : "0060e2"}),${dark ? .45 : .26})` : rgba(dark ? "0190fe" : "a9d3f5"),
    "context_map.file_ink": rgba(dark ? "8bcbfe" : clean ? "0070d6" : "0190fe"),
    "context_map.chat_bg": clean ? `alpha(rgb(0x${dark ? "b98af0" : "a651f0"}),${dark ? .17 : .09})` : rgba(dark ? "312041" : "f1eafa"),
    "context_map.chat_edge": clean ? `alpha(rgb(0x${dark ? "b98af0" : "a651f0"}),${dark ? .42 : .28})` : rgba(dark ? "714896" : "b98af0"),
    "context_map.chat_ink": rgba(dark ? "d1b2f5" : "8a35d6"),
    "context_map.result_bg": clean ? `alpha(rgb(0x${dark ? "f2a15c" : "f28a2e"}),${dark ? .17 : .13})` : rgba(dark ? "3a2712" : "ffecd6"),
    "context_map.result_edge": clean ? `alpha(rgb(0x${dark ? "f2a15c" : "f28a2e"}),${dark ? .45 : .36})` : rgba(dark ? "b25e00" : "ffb35c"),
    "context_map.result_ink": rgba(dark ? "ffb35c" : "b25e00"),
    "context_map.summary_bg": clean ? `alpha(rgb(0x${dark ? "7dcf9f" : "5fc88a"}),${dark ? .17 : .13})` : rgba(dark ? "153222" : "ddf2e6"),
    "context_map.summary_edge": clean ? `alpha(rgb(0x${dark ? "7dcf9f" : "5fc88a"}),${dark ? .42 : .34})` : rgba(dark ? "1e8a4f" : "9fd9b7"),
    "context_map.summary_ink": rgba(dark ? (clean ? "7dcf9f" : "5fc88a") : (clean ? "16673b" : "1e8a4f")),
    "profile_menu.accent": clean && !dark ? "alpha(rgb(0x0060e2),0.765)" : rgba("4fb0fe"),
    "profile_menu.badge_ink": rgba(dark ? "ff8a7a" : "e12a1d"),
    "profile_menu.badge_bg": dark ? `alpha(rgb(0xff6b5f),${clean ? .14 : .16})` : clean ? "alpha(rgb(0xf04438),0.08)" : rgba("fff0f3"),
    "profile_menu.muted": clean ? (dark ? white(.46) : black(.45)) : rgba("8f8f8f"),
    "profile_menu.setup_ink": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "c4c4c4" : "404040"),
    "profile_menu.track": dark ? white(.1) : black(clean ? .06 : .07),
    "profile_menu.tray": clean ? transparent : rgba(dark ? "161618" : "f0f0f0"),
    "email_draft.card": rgba(dark ? (clean ? "1c1c1e" : "232326") : "ffffff"),
    "email_draft.tray": clean ? transparent : rgba(dark ? "161618" : "f0f0f0"),
    "email_draft.warn": rgba(dark ? (clean ? "e6c24d" : "f2a15c") : (clean ? "b48906" : "b25e00")),
    "email_draft.warn_bg": dark
      ? (clean ? "alpha(rgb(0xe6c24d),0.14)" : "alpha(rgb(0xf2a15c),0.14)")
      : (clean ? "alpha(rgb(0xe8b000),0.12)" : rgba("fdeedd")),
    "email_draft.ok": rgba(dark ? (clean ? "5fc88a" : "7dcf9f") : "1e8a4f"),
    "email_draft.ok_bg": dark ? "alpha(rgb(0x7dcf9f),0.14)" : (clean ? "alpha(rgb(0x5fc88a),0.08)" : rgba("e6f5ec")),
    "email_draft.chip": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2d" : "f5f5f5"),
    "email_draft.account": clean ? (dark ? white(.62) : black(.61)) : rgba("8f8f8f"),
    "email_draft.address": clean ? (dark ? white(.46) : black(.45)) : rgba("8f8f8f"),
    "email_draft.file": clean ? (dark ? white(.62) : black(.61)) : rgba("8f8f8f"),
    "email_draft.hint": clean ? (dark ? white(.30) : black(.27)) : rgba("8f8f8f"),
    "email_draft.tone_ink": clean ? (dark ? white(.62) : black(.61)) : rgba("8f8f8f"),
    "email_draft.tone_hover": clean ? (dark ? white(.04) : black(.03)) : transparent,
    "email_draft.tone_edge": dark ? white(.08) : clean ? black(.06) : rgba("ebebeb"),
    "email_draft.link": clean ? rgba(dark ? "ffffff" : "000000") : rgba(dark ? "c4c4c4" : "404040"),
    "email_draft.link_underline": clean ? (dark ? white(.30) : black(.27)) : alphaColor(dark ? "c4c4c4" : "404040",.35),
    "email_draft.send": clean ? (dark ? white(.92) : black(.88)) : rgba(dark ? "e4e4e7" : "201e25"),
    "email_draft.on_send": rgba(dark ? (clean ? "161618" : "121213") : (clean ? "fcfcfc" : "ffffff")),
    "diff_timeline.add": rgba(dark ? "7dcf9f" : "5fc88a"),
    "diff_timeline.add_ink": rgba(dark ? "7dcf9f" : "1e8a4f"),
    "diff_timeline.del": rgba(dark ? "ff6b5f" : "f04438"),
    "diff_timeline.del_ink": rgba(dark ? "ff8a7a" : "c4320a"),
    "diff_timeline.faint": clean ? (dark ? white(.30) : black(.30)) : rgba(dark ? "5c5c63" : "b5b5b5"),
    "diff_timeline.muted": clean ? (dark ? white(.46) : black(.45)) : rgba(dark ? "8b8b92" : "8f8f8f"),
    "diff_timeline.binary": rgba(dark ? "77777e" : "aaaaaf"),
    "diff_timeline.card": rgba(dark ? (clean ? "1c1c1e" : "19191b") : "ffffff"),
    "diff_timeline.code": rgba(dark ? "90cdfe" : "0190fe"),
    "diff_timeline.grid": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2e" : "e6e6e8"),
    "diff_timeline.image": rgba(dark ? "b98af0" : "a651f0"),
    "diff_timeline.net": clean ? (dark ? white(.28) : black(.27)) : rgba(dark ? "606067" : "b8b8b8"),
    "diff_timeline.sheet": rgba(dark ? "7dcf9f" : "1e8a4f"),
    "diff_timeline.tray": clean ? transparent : rgba(dark ? "111113" : "f0f0f0"),
    "diff_timeline.wash": `alpha(rgb(0x${dark ? "4d8dff" : "0060e2"}),${clean ? .05 : .07})`,
    ...costColors,
    "cost_waterfall.muted": clean ? (dark ? white(.45) : black(.45)) : rgba(dark ? "7c7c82" : "8a8a8a"),
    "cost_waterfall.card": rgba(dark ? (clean ? "1c1c1e" : "232326") : "ffffff"),
    "cost_waterfall.tray": clean ? transparent : rgba(dark ? "19191c" : "f0f0f0"),
    "project_card.ring": clean ? (dark ? white(.10) : black(.08)) : rgba(dark ? "3a3a40" : "dcdce0"),
    "project_card.ring_hot": clean ? (dark ? white(.26) : black(.20)) : rgba(dark ? "6c6c72" : "acacb3"),
    "project_card.card": rgba(dark ? (clean ? "1c1c1e" : "19191b") : "ffffff"),
    "project_card.tray": clean ? transparent : rgba(dark ? "111113" : "f0f0f0"),
    "project_card.muted": clean ? (dark ? white(.46) : black(.45)) : rgba(dark ? "8b8b92" : "8f8f8f"),
    "project_card.option_text": dark ? white(.62) : black(.61),
    "project_card.option_ink": dark ? white(.90) : black(.88),
    "project_card.option_line": dark ? white(.08) : black(.06),
    "project_card.option_fill": dark ? white(.08) : black(.06),
    "project_card.option_hover": dark ? white(.04) : black(.03),
    "experiment_card.control": clean ? (dark ? white(.30) : black(.22)) : rgba(dark ? "6e6e75" : "b4b4ba"),
    "experiment_card.a": clean && !dark ? "alpha(rgb(0x0060e2),0.765)" : rgba("4fb0fe"),
    "experiment_card.b": rgba(dark ? "b98af0" : "a651f0"),
    "experiment_card.c": rgba(clean && !dark ? "e8b000" : "e6c24d"),
    "experiment_card.d": rgba(dark ? "5fc88a" : "1e8a4f"),
    "experiment_card.track": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "3b3b42" : "ededf0"),
    "experiment_card.card": rgba(dark ? (clean ? "1c1c1e" : "19191b") : "ffffff"),
    "experiment_card.tray": clean ? transparent : rgba(dark ? "111113" : "f0f0f0"),
    "experiment_card.muted": clean ? (dark ? white(.46) : black(.45)) : rgba(dark ? "8b8b92" : "8f8f8f"),
    "thinking.sheen": dark ? white(clean ? 1 : .95) : white(.82),
    "thinking.ink": clean ? muted : rgba(dark ? "a1a1a6" : "6b6b6b"),
    "thinking.check": clean ? (dark ? rgba("4fb0fe") : "alpha(rgb(0x0060e2),0.765)") : ink,
    "run.mark": dark ? rgba("4fb0fe") : "alpha(rgb(0x0060e2),0.78)",
    "run.tray": clean ? panel : rgba(dark ? "111113" : "f0f0f0"),
    "run.queue": clean ? `alpha(${ink},0.04).over(${panel})` : transparent,
    "run.steer": clean ? (dark ? white(.08) : "alpha(rgb(0x0060e2),0.10)") :
        (dark ? "alpha(rgb(0x4fb0fe),0.11)" : "alpha(rgb(0x008fff),0.11)"),
    "run.steer_ink": rgba(dark ? "80bbff" : "165bba"),
    "writing.glow_pink": rgba(dark ? "f06db5" : "ee3e9e"),
    "writing.glow_violet": rgba(dark ? "b98af0" : "a651f0"),
    "writing.glow_blue": rgba("4fb0fe"),
    "writing.deck": clean ? panel : rgba(dark ? "1d1d21" : "fbfbfb"),
    "writing.user": dark ? white(0.07) : black(0.06),
    "writing.answer": clean ? transparent : (dark ? "alpha(rgb(0x242427),0.85)" : white(0.85)),
    "writing.listening": clean ? (dark ? "alpha(rgb(0x4fb0fe),0.11)" : "alpha(rgb(0x0060e2),0.08)") : ink,
    "writing.listening_ink": clean ? (dark ? rgba("4fb0fe") : rgba("0060e2")) : rgba(dark ? "19191b" : "ffffff"),
    "catalog.tile_app": rgba("000000"),
    "catalog.mark_app": rgba("ffffff"),
    "catalog.tile_blue": rgba(dark ? "202a3b" : "eaf1ff"),
    "catalog.mark_blue": rgba(dark ? "8ab4ff" : "2456d6"),
    "catalog.tile_violet": rgba(dark ? "302440" : "f4ebfd"),
    "catalog.mark_violet": rgba(dark ? "c9a2f5" : "7935b8"),
    "catalog.tile_amber": rgba(dark ? "3b2b20" : "fff1e0"),
    "catalog.mark_amber": rgba(dark ? "ffb35c" : "a15413"),
    "catalog.tile_green": rgba(dark ? "193126" : "e6f6ed"),
    "catalog.mark_green": rgba(dark ? "5fc88a" : "1e8a4f"),
    "catalog.tile_gray": dark ? rgba("2a2a2e") : rgba("f3f3f5"),
    "catalog.mark_gray": ink,
    "control.border": edge, "control.hover": wash,
    "control.edge_strong": edgeStrong,
    "control.focus_edge": clean ? (dark ? white(.35) : black(.28)) : rgba(dark ? "63636b" : "b5b5b5"),
    "tabs.foreground": clean ? muted : rgba(dark ? "8b8b92" : "737373"),
    "tabs.ink": ink,
    "tabs.hover": clean ? (dark ? white(.78) : black(.75)) : rgba(dark ? "c8c8cc" : "404040"),
    "tabs.tray": clean ? (dark ? white(.05) : black(.04)) : rgba(dark ? "111113" : "f0f0f0"),
    "tabs.pill": rgba(dark ? (clean ? "2c2c2f" : "26262a") : "ffffff"),
    "tabs.line": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2e" : "ececec"),
    "tabs.counter": dark ? white(.08) : black(.06),
    "tabs.counter_fg": clean ? muted : rgba(dark ? "c8c8cc" : "404040"),
    "tabs.icon": clean ? (dark ? white(.46) : black(.447)) : rgba(dark ? "8b8b92" : "737373"),
    "tabbar.background": clean ? (dark ? white(.04) : black(.03)) : rgba(dark ? "1b1b1e" : "f0f0f0"),
    "tabbar.active": panel,
    "collection.selection": selected,
    "table.surface": rgba(dark ? (clean ? "1c1c1e" : "232326") : "ffffff"),
    "table.edge": clean ? (dark ? white(.1) : black(.06)) : (dark ? white(.08) : rgba("e5e5e5")),
    "table.grid": clean ? (dark ? white(.08) : black(.06)) : (dark ? white(.07) : rgba("f0f0f0")),
    "table.header_ink": clean ? muted : rgba("8f8f8f"),
    "table.text": clean ? muted : rgba(dark ? "c4c4c4" : "404040"),
    "table.icon": clean ? (dark ? white(.5) : black(.447)) : rgba(dark ? "6b6b70" : "b5b5b5"),
    "table.header_hover": clean ? (dark ? white(.05) : black(.03)) : (dark ? white(.06) : rgba("f6f6f6")),
    "table.header_hover_ink": clean ? ink : rgba(dark ? "c4c4c4" : "404040"),
    "table.row_hover": clean ? (dark ? white(.025) : black(.015)) : (dark ? white(.03) : rgba("fafafa")),
    "table.sort_ink": clean && !dark ? `alpha(rgb(0x0060e2),0.765)` : rgba("4fb0fe"),
    "table.selection": clean ? (dark ? `alpha(rgb(0x4d8dff),0.14)` : `alpha(rgb(0x0060e2),0.05)`)
      : (dark ? `alpha(rgb(0x4fb0fe),0.12)` : rgba("f5f8ff")),
    "button.primary_fg": clean ? rgba(dark ? "161618" : "fcfcfc") : rgba(dark ? "121213" : "fafafa"),
    "button.primary_bg": clean ? (dark ? white(.9) : black(.88)) : rgba(dark ? "f4f4f5" : "323137"),
    "button.primary_hover": clean ? (dark ? white(.82) : black(.8)) : rgba(dark ? "ffffff" : "3c3b42"),
    "button.primary_bottom": rgba(dark ? "e4e4e7" : "201e25"),
    "button.primary_hover_bottom": rgba(dark ? "ececef" : "29272f"),
    "button.edge_top": rgba(dark ? "ffffff" : "4b4951"),
    "button.edge_mid": rgba(dark ? "ececef" : "31303a"),
    "button.edge_bottom": rgba(dark ? "d4d4d8" : "18171b"),
    "button.ring": dark ? black(.6) : rgba("0d0d0d"),
    "button.secondary_bg": rgba(dark ? (clean ? "1c1c1e" : "232326") : "ffffff"),
    "button.secondary_hover": clean ? field : rgba(dark ? "2a2a2d" : "fafafa"),
    "button.ghost_fg": clean ? muted : rgba(dark ? "8f8f8f" : "737373"),
    "button.ghost_hover_fg": clean ? ink : rgba(dark ? "ededed" : "404040"),
    "button.danger_bg": clean ? `alpha(rgb(0x${dark ? "ff3523" : "f04438"}),0.08)` : rgba(dark ? "3b1c14" : "ffe9e6"),
    "button.danger_hover": clean ? `alpha(rgb(0x${dark ? "ff3523" : "f04438"}),0.12)` : `mix(${rgba(dark ? "3b1c14" : "ffe9e6")},${rgba(semantic.danger[1])},0.12)`,
    "field.read_only": rgba(dark ? "151517" : "fafafa"),
    "field.placeholder": placeholder,
    "field.counter_full": clean ? muted : rgba(dark ? "c8c8cc" : "404040"),
    "field.edge": clean ? (dark ? white(.16) : black(.1)) : rgba(dark ? "3a3a40" : "d4d4d8"),
    "field.line": clean ? (dark ? white(.09) : black(.06)) : rgba(dark ? "2a2a2e" : "e6e6e8"),
    "field.focus_edge": clean ? (dark ? white(.35) : black(.28)) : rgba(dark ? "4e4e55" : "b4b4bb"),
    "number.step_hover": clean ? (dark ? white(.06) : black(.04)) : rgba(dark ? "26262a" : "f4f4f5"),
    "number.step_press": clean ? (dark ? white(.09) : black(.07)) : rgba(dark ? "303035" : "ebebed"),
    "number.read_only": clean ? (dark ? white(.02) : black(.02)) : rgba(dark ? "1b1b1d" : "fafafa"),
    "number.error_edge": clean ? `alpha(rgb(0x${dark ? "ff3523" : "f04438"}),0.55)` : rgba("ff8a7a"),
    "number.error_focus": clean ? "alpha(rgb(0xe5484d),0.8)" : rgba("f04438"),
    "number.error_fill": clean ? field : `alpha(rgb(0x${dark ? "ff3523" : "f04438"}),0.04)`,
    "otp.empty": clean ? field : rgba(dark ? "18181b" : "fafafa"),
    "otp.wait": clean ? (dark ? white(.06) : black(.06)) : rgba(dark ? "252528" : "f2f2f3"),
    "otp.shine": white(dark ? .18 : clean ? .82 : .95),
    "otp.separator": clean ? (dark ? white(.27) : black(.27)) : rgba(dark ? "555560" : "c4c4c8"),
    "otp.focus_line": clean ? `alpha(rgb(0x${dark ? "4d8dff" : "0060e2"}),0.765)` : ink,
    "otp.focus_halo": clean ? `alpha(rgb(0x${dark ? "4d8dff" : "0060e2"}),0.35)` : (dark ? white(.07) : "alpha(rgb(0x171717),0.07)"),
    "otp.invalid_line": rgba(dark ? "ff6454" : "f04438"),
    "otp.invalid_tint": `alpha(rgb(0x${dark ? "ff6454" : "f04438"}),0.08)`,
    "otp.success_tint": "alpha(rgb(0x5fc88a),0.08)",
    "choice.fill": clean ? (dark ? white(.9) : black(.88)) : rgba(dark ? "ededed" : "171717"),
    "choice.on_fill": rgba(dark ? "161618" : "ffffff"),
    "choice.description": faint,
    "choice.edge": clean ? (dark ? white(.22) : black(.18)) : rgba(dark ? "3a3a40" : "d4d4d8"),
    "choice.edge_hover": clean ? (dark ? white(.36) : black(.32)) : rgba(dark ? "55555c" : "a1a1aa"),
    "tag.chip": clean ? (dark ? white(.07) : black(.045)) : rgba(dark ? "232326" : "f4f4f5"),
    "tag.edge": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2f2f34" : "e6e6e8"),
    "tag.facet": clean ? (dark ? white(.14) : black(.1)) : rgba(dark ? "303035" : "e4e4e7"),
    "kbd.surface": clean ? `alpha(${ink},${dark ? .1 : .07})` : rgba(dark ? "232326" : "ffffff"),
    "kbd.ink": clean ? ink : rgba(dark ? "a1a1a6" : "616161"),
    "kbd.edge": clean ? `alpha(${ink},${dark ? .08 : .06})` : rgba(dark ? "34343a" : "e3e3e3"),
    "kbd.depth": rgba(dark ? "1a1a1d" : "d9d9d9"),
    "kbd.menu_surface": clean ? transparent : rgba(dark ? "232326" : "ffffff"),
    "kbd.menu_edge": clean ? (dark ? white(.12) : black(.1)) : rgba(dark ? "2a2a2e" : "ececec"),
    "command.shortcut": faint,
    "command.shortcut_active": clean ? muted : faint,
    "command.surface": rgba(dark ? (clean ? "1c1c1e" : "19191b") : "ffffff"),
    "command.tray": clean ? transparent : rgba(dark ? "111113" : "f0f0f0"),
    "command.line": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2e" : "ececec"),
    "command.shadow_edge": clean ? (dark ? white(.1) : black(.08)) : rgba(dark ? "2a2a2e" : "ececec"),
    "command.ink": ink,
    "command.placeholder": clean ? (dark ? white(.3) : black(.27)) : placeholder,
    "command.icon": clean ? (dark ? white(.5) : black(.447)) : faint,
    "command.hint": clean ? (dark ? white(.46) : black(.45)) : faint,
    "command.logo_surface": rgba(dark ? (clean ? "2a2a2d" : "26262a") : "ffffff"),
    "command.logo_edge": dark ? white(clean ? .12 : .1) : black(clean ? .1 : .09),
    "command.logo_ink": clean ? rgba(dark ? "ffffff" : "000000") : ink,
    "command.group": clean ? faint : placeholder,
    "command.selection": clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "26262a" : "f0f0f1"),
    "command.footer": faint,
    "pagination.ink": ink,
    "pagination.text": clean ? (dark ? white(.62) : black(.61)) : rgba(dark ? "d4d4d8" : "404040"),
    "pagination.muted": clean ? (dark ? white(.45) : black(.45)) : rgba(dark ? "8f8f8f" : "737373"),
    "pagination.faint": clean ? (dark ? white(.28) : black(.27)) : rgba(dark ? "5c5c63" : "a3a3a3"),
    "pagination.wash": clean ? (dark ? white(.06) : black(.04)) : (dark ? white(.06) : rgba("f0f0f0")),
    "pagination.active": clean ? (dark ? white(.09) : black(.06)) : (dark ? white(.1) : rgba("ebebeb")),
    "pagination.surface": rgba(dark ? (clean ? "1b1b1e" : "232326") : "ffffff"),
    "pagination.surface_hover": rgba(dark ? "2a2a2d" : "fafafa"),
    "pagination.edge": clean ? (dark ? white(.1) : black(.06)) : (dark ? white(.1) : rgba("e5e5e5")),
    "pagination.edge_hover": clean ? (dark ? white(.16) : black(.1)) : edgeStrong,
    "tooltip.surface": clean ? (dark ? `alpha(rgb(0xfafafa),0.94)` : `alpha(rgb(0x18181b),0.92)`) : rgba(dark ? "fafafa" : "171717"),
    "tooltip.ink": rgba(dark ? (clean ? "111113" : "171717") : (clean ? "fcfcfc" : "ffffff")),
    "tooltip.muted": clean ? (dark ? black(.5) : white(.6)) : (dark ? black(.6) : white(.72)),
    "tooltip.key": clean ? transparent : (dark ? black(.07) : white(.14)),
    "tooltip.shadow": black(clean ? (dark ? .6 : .24) : (dark ? .7 : .32)),
    "tooltip.shadow_secondary": dark ? white(.06) : black(.12),
    "switch.off": clean ? (dark ? white(.16) : black(.12)) : rgba(dark ? "3a3a3f" : "e2e2e2"),
    "switch.off_hover": clean ? (dark ? white(.22) : black(.18)) : rgba(dark ? "46464c" : "d4d4d8"),
    "switch.on_hover": clean ? (dark ? white(.8) : black(.78)) : rgba(dark ? "d4d4d8" : "2e2e2e"),
    "switch.thumb": rgba("ffffff"),
    // The unchecked loading glyph is derived UI, paired with the white thumb.
    "switch.thumb_off_ink": rgba("161618"),
    "switch.checked_thumb": rgba(dark ? "161618" : "ffffff"),
    "scrollbar.thumb": clean ? (dark ? white(.2) : black(.2)) : rgba(dark ? "4a4a52" : "c2c2c7"),
    "scrollbar.thumb_hover": clean ? (dark ? white(.32) : black(.32)) : rgba(dark ? "63636b" : "a1a1aa"),
  };
  for (const [tone, [bg, fg]] of Object.entries(semantic)) {
    colors[`badge.${tone}_bg`] = rgba(bg);
    colors[`badge.${tone}_fg`] = rgba(fg);
    if (["info","success","warning","danger"].includes(tone)) {
      colors[`tag.${tone}_bg`] = rgba(bg);
      colors[`tag.${tone}_fg`] = rgba(fg);
    }
  }
  if (clean) {
    Object.assign(colors, dark ? {
      "badge.neutral_bg": white(.08), "badge.neutral_fg": white(.62),
      "badge.info_bg": rgba("0077ff3a"), "badge.info_fg": rgba("70b8ff"),
      "badge.success_bg": rgba("00f04a1f"),
      "badge.warning_bg": rgba("fb6a0025"),
      "badge.danger_bg": rgba("ff35232b"),
      "badge.purple_bg": `alpha(rgb(0xb98af0),0.16)`,
    } : {
      "badge.neutral_bg": black(.06), "badge.neutral_fg": black(.61),
      "badge.info_bg": rgba("008ff519"), "badge.info_fg": rgba("006dcbf2"),
      "badge.success_bg": rgba("00a43319"), "badge.success_fg": rgba("007d2ee0"),
      "badge.warning_bg": rgba("ff9c0029"),
      "badge.danger_bg": rgba("f52b0018"), "badge.danger_fg": rgba("d72400da"),
      "badge.purple_bg": `alpha(rgb(0xa651f0),0.11)`,
    });
  }
  for (const tone of badgeTones) {
    colors[`badge.${tone}_strong_bg`] = `${color(`badge.${tone}_fg`)}.over(${color("surface_raised")})`;
    colors[`badge.${tone}_strong_fg`] = strongInk(color(`badge.${tone}_strong_bg`));
  }
  colors["collection.metadata"] = metadataInk();
  for (const tone of ["success", "warning"]) {
    const normal = color(`badge.${tone}_bg`);
    const hover = `mix(${normal},${color(`badge.${tone}_fg`)},0.08)`;
    colors[`button.${tone}_fg`] = derivedInk(color(`badge.${tone}_fg`),
      [`${normal}.over(${color("surface_raised")})`, `${hover}.over(${color("surface_raised")})`]);
  }
  const dimensions = {
    spinner: scale([12,14,14,18]),
    run_prompt_tip: px(clean ? 4 : 5),
    rate_plan_sheen: px(60),
    thinking_h: scale(clean ? [26,26,32,32] : [28,28,34,34]),
    thinking_plain_h: scale(clean ? [16,16,20,20] : [18,18,21,21]),
    thinking_gap: scale(clean ? [5,5,6,6] : [7,7,8,8]),
    thinking_pad_start: scale(clean ? [6,6,8,8] : [10,10,12,12]),
    thinking_pad_end: scale(clean ? [8,8,10,10] : [12,12,15,15]),
    thinking_radius: scale(clean ? [6,6,8,8] : [999,999,999,999]),
    thinking_icon: scale(clean ? [14,14,16,16] : [13,13,15,15]),
    thinking_font: scale(clean ? [12,12,13,13] : [13,13,14,14]),
    thinking_line: scale(clean ? [16,16,20,20] : [18,18,21,21]),
    button_h: compactScale(clean ? [20,24,28,32] : [28,32,40,48]),
    button_pad: compactScale(clean ? [6,8,10,12] : [10,12,16,20],4,2),
    button_icon_pad: compactScale(clean ? [4,6,8,10] : [10,12,16,20],4,2),
    button_gap: compactScale(clean ? [4,4,6,6] : [4,6,8,10],2,2),
    button_icon: scale(clean ? [12,14,16,16] : [14,16,18,20]),
    button_radius: scale(clean ? [6,6,6,6] : [8,10,12,14]),
    icon_button_h: compactScale(clean ? [20,24,28,32] : [28,32,40,48]),
    icon_button_icon: scale(clean ? [12,14,15,16] : [14,16,18,20]),
    field_h: compactScale(clean ? [24,28,32,36] : [28,32,36,40],24),
    field_pad: compactScale(clean ? [6,8,10,12] : [8,10,12,14],4,2),
    field_radius: scale(clean ? [5,6,7,8] : [8,9,10,12]),
    number_step: compactScale(clean ? [16,20,24,26] : [20,24,28,28],16),
    control: compactScale(clean ? [24,28,32,36] : [28,32,36,40],24),
    control_pad: compactScale(clean ? [6,8,10,12] : [8,10,12,14],4,2),
    icon: scale(clean ? [12,14,16,16] : [14,16,18,20]),
    multiline_pad: density(clean ? 7 : 8,clean ? 5 : 6),
    multiline_radius: px(clean ? 8 : 10),
    row: density(clean ? 28 : 32,clean ? 24 : 28), inset: density(clean ? 10 : 12,8),
    command_row: density(clean ? 36 : 40,clean ? 32 : 36),
    command_narrow_row: density(clean ? 40 : 44,clean ? 36 : 40),
    command_tray_pad:px(clean ? 0 : 4),
    command_search_h:density(clean ? 48 : 52,clean ? 44 : 48),
    command_input_h:px(20), command_search_icon:px(clean ? 16 : 18), command_enter_icon:px(clean ? 12 : 14),
    command_search_start:density(clean ? 14 : 16,clean ? 10 : 12), command_search_end:density(clean ? 12 : 16,clean ? 8 : 12),
    command_list_inset:density(clean ? 4 : 6,clean ? 2 : 4),
    command_list_top:density(clean ? 0 : 4,clean ? 0 : 2), command_list_bottom:density(clean ? 4 : 8,clean ? 2 : 6),
    command_footer_h:density(clean ? 40 : 38,clean ? 36 : 34),
    command_item_inset:density(clean ? 8 : 10,clean ? 6 : 8),
    command_icon_slot:px(clean ? 20 : 26), command_item_icon:px(clean ? 16 : 17),
    command_logo:px(clean ? 12 : 14), command_logo_radius:px(clean ? 5 : 7),
    command_group_top:density(10,8), command_group_bottom:density(4,2),
    command_title_inset:density(clean ? 12 : 16,clean ? 8 : 12),
    table_row: density(clean ? 40 : 44,clean ? 36 : 40), table_header: density(clean ? 36 : 44,clean ? 32 : 40),
    table_pad: density(clean ? 12 : 14,clean ? 10 : 12),
    table_header_pad: density(6,4), table_action_pad: density(clean ? 6 : 8,clean ? 4 : 6),
    table_action_h: density(clean ? 28 : 32,clean ? 24 : 28),
    table_sort_icon: px(11), table_sort_reserve: px(clean ? 17 : 19),
    table_grid_x:px(clean ? 0 : 1), table_grid_y:px(clean ? .5 : 1),
    marker: px(clean ? 20 : 22), marker_small: px(clean ? 18 : 20),
    badge_pad: scale(clean ? [6,6,7,7] : [8,8,9,9]),
    badge_lead: scale(clean ? [4,4,5,5] : [6,6,7,7]),
    badge_icon: scale([12,12,13,13]), badge_gap: scale([4,4,5,5]),
    badge_radius: scale(clean ? [4,4,5,5] : [999,999,999,999]),
    tag_h: px(24), tag_radius: px(clean ? 5 : 7), tag_pad: px(8), tag_end: px(3),
    tag_gap: px(5), tag_close: px(18), tag_icon: px(10),
    kbd_h: scale(clean ? [16,16,20,20] : [18,18,22,22]),
    kbd_pad: scale(clean ? [4,4,5,5] : [5,5,6,6]),
    kbd_radius: scale(clean ? [4,4,5,5] : [5,5,6,6]),
    kbd_glyph: scale(clean ? [11.55,11.55,13.2,13.2] : [12.1,12.1,13.2,13.2]),
    mark: px(16), choice_line: px(20), checkbox_radius: px(clean ? 4.5 : 5),
    switch_h: scale([16,16,20,20]), switch_w: scale([26,26,32,32]),
    switch_thumb: scale([12,12,16,16]), switch_inset: px(2),
    titlebar: density(clean ? 36 : 40,clean ? 32 : 36), statusbar: density(24,22),
    label_column: px(128), counter: px(clean ? 16 : 18),
    card_outer_radius: clean ? px(10) : density(20,19), card_inner_radius: px(clean ? 10 : 16),
    card_tray_pad: clean ? px(0) : density(4,3), card_pad: density(16,12),
    popover_pad: px(clean ? 14 : 16), popover_radius: px(clean ? 12 : 16),
    tooltip_radius: px(clean ? 6 : 8),
    pagination_h: compactScale(clean ? [20,24,28,32] : [24,28,32,36],20),
    pagination_pad: px(clean ? 7 : 8), pagination_radius: px(clean ? 6 : 10),
    pagination_icon: scale(clean ? [12,12,14,16] : [12,14,16,18]),
    pagination_select_icon: scale(clean ? [10,10,12,14] : [10,12,14,16]),
    segment_h: compactScale(clean ? [20,24,28,32] : [24,26,30,36]),
    segment_pad: scale(clean ? [2,2,2,2] : [2,2,3,4],[2,2,2,2]),
    segment_radius: scale(clean ? [6,8,8,8] : [8,9,11,13],clean ? [6,8,8,8] : [8,9,10,11]),
    segment_thumb_radius: scale(clean ? [4,6,6,6] : [6,7,8,9]),
    segment_px: compactScale(clean ? [6,8,10,12] : [8,10,12,16],4,2),
    segment_gap: compactScale(clean ? [6,6,8,8] : [12,16,20,24],4,4),
    tabs_track_h: scale(clean ? [16,20,24,28] : [24,26,30,36],clean ? [16,16,20,24] : [20,22,26,32]),
    tabs_track_total_h: scale(clean ? [20,24,28,32] : [28,30,36,44],clean ? [20,20,24,28] : [24,26,30,36]),
    tabs_underline_h: compactScale(clean ? [26,30,34,38] : [30,32,36,42]),
    tabs_bar_h: compactScale(clean ? [20,24,28,32] : [24,28,32,36]),
    segment_icon: scale(clean ? [12,14,16,16] : [12,14,15,16]),
    tabs_content_gap: scale(clean ? [4,5,6,6] : [6,6,6,6]),
    tabs_counter_h: scale(clean ? [12,14,16,16] : [14,16,18,18]),
    tabs_counter_pad: scale(clean ? [3,3,4,4] : [3,4,5,5]),
    tabs_counter_radius: scale(clean ? [3,4,5,5] : [999,999,999,999]),
    status_dot: px(6),
    slider_track: px(4), slider_thumb: px(clean ? 12 : 14),
    handle: px(6), grip: px(2), tree_indent: px(18),
  };
  const type = {
    caption: role(12,16), metadata: '"caption"', body_small: role(13,20), body: role(13,20),
    initials: role(10,16,500),
    otp_cell: role(clean ? 15 : 22.08,clean ? 20 : 26),
    otp_success: role(14,20),
    filter_title: role(15,20),
    stepper_title: role(16,20,600),
    review_title: role(18,20,600),
    catalog_initials: role(12,16,600),
    catalog_initials_large: role(20,24,600),
    choice: role(clean ? 13 : 13.5,20),
    subtitle: role(clean ? 13 : 14,20,500), title: role(15,22,500),
    heading: role(18,26,600), display: role(24,32,600), display_large: role(32,40,600),
    label: role(13,20,500), hint: role(clean ? 12 : 12.5,clean ? 16 : 17),
    code: role(13,20,400,`,family:"Menlo",fallbacks:["DejaVu Sans Mono","Liberation Mono"]`),
    control_small: role(13,20), control_regular: role(clean ? 13 : 13.5,20),
    control_large: role(14,20),
    control: `by_env("size",#{xs:"control_small",sm:"control_small",md:"control_regular",lg:"control_large"})`,
    button_xs: role(clean ? 11 : 12,clean ? 14 : 12,500),
    button_sm: role(clean ? 12 : 13,clean ? 16 : 13,500),
    button_md: role(clean ? 13 : 14,clean ? 20 : 14,500),
    button_lg: role(clean ? 13 : 15,clean ? 20 : 15,500),
    button: `by_env("size",#{xs:"button_xs",sm:"button_sm",md:"button_md",lg:"button_lg"})`,
    badge_small: role(11,16,500), badge_regular: role(12,16,500),
    badge: `by_env("size",#{xs:"badge_small",sm:"badge_small",md:"badge_regular",lg:"badge_regular"})`,
    tag: role(clean ? 12.5 : 13,20),
    key_legend: '"caption"',
    key_legend_cap_small: role(clean ? 10.5 : 11,clean ? 10.5 : 11,500),
    key_legend_cap_regular: role(12,12,500),
    key_legend_cap: 'by_env("size",#{xs:"key_legend_cap_small",sm:"key_legend_cap_small",md:"key_legend_cap_regular",lg:"key_legend_cap_regular"})',
    key_legend_tooltip: role(clean ? 12 : 11,16,500),
    key_legend_menu: role(11,16,500),
    command_body:role(clean ? 13 : 13.5,20), command_search:role(clean ? 13 : 15,20),
    command_enter:role(11.5,16,500), command_footer:role(clean ? 12 : 12.5,clean ? 16 : 18),
    command_group:role(clean ? 10 : 12,clean ? 16 : 14.5,500),
    pagination: role(clean ? 13 : 13.5,20),
    card_footer: role(clean ? 12 : 12.5,clean ? 16 : 18),
    textarea: role(13,clean ? 20 : 21),
    tooltip: role(clean ? 12 : 12.5,clean ? 16 : 18),
    breadcrumb: role(clean ? 13 : 13.5,20),
    hover_card: role(clean ? 13 : 13.5,20),
    counter_small: role(10,12,500), counter_regular: role(11,14,500), counter: '"counter_regular"',
    table_header: role(clean ? 12 : 14,clean ? 16 : 20,clean ? 500 : 450), table_body: role(clean ? 13 : 14,20),
    segment_small: role(clean ? 12 : 12.5,clean ? 16 : 12.5,clean ? 500 : 400),
    segment_regular: role(clean ? 13 : 13.5,clean ? 20 : 13.5,clean ? 500 : 400),
    segment_large: role(clean ? 13 : 14,clean ? 20 : 14,clean ? 500 : 400),
    segment: 'by_env("size",#{xs:"segment_small",sm:"segment_small",md:"segment_regular",lg:"segment_large"})',
    tabs_counter_small: role(clean ? 10 : 10.5,clean ? 14 : 10.5,clean ? 500 : 600),
    tabs_counter_regular: role(11,clean ? 16 : 11,clean ? 500 : 600),
    tabs_counter: 'by_env("size",#{xs:"tabs_counter_small",sm:"tabs_counter_small",md:"tabs_counter_regular",lg:"tabs_counter_regular"})',
  };
  // Keep raw appearance values in the palette, and bind shared recipe paint
  // to semantic roles. Local component colors are still distinct tokens.
  const styled = (() => {
    const ink = color("text_primary");
    const strongest = color("text.strongest");
    const muted = color("text_muted");
    const faint = color("text.subtle");
    const placeholder = color("field.placeholder");
    const panel = color("surface_raised");
    const field = color("surface.field");
    const canvas = color("surface");
    const wash = color("surface_hover");
    const edge = color("border");
    const edgeStrong = color("control.edge_strong");
    const focus = color("focus_ring");
    const selected = color("selection");
    const hairline = shadow(0,0,0,clean ? .5 : 1,edge);
    const smallShadow = [hairline, shadow(0,1,1,-1,black(.1)), shadow(0,1,2,0,black(.05))];
    // Nonmodal elevation: the surface and hairline separate the floating
    // panel. A small cast/contact pair avoids a long, low-contrast halo.
    const floatingCast = (ink = black(.12), contact = black(dark ? .06 : .03)) =>
      [shortDrop(ink),shadow(0,1,0,0,contact)];
    const floatingShadow = [hairline,...floatingCast()];
    // Modals retain the backdrop wash and a stronger near-edge shadow, not a
    // longer shadow. Other panels must not inherit modal elevation.
    const modalShadow = [hairline,...floatingCast(black(dark ? .24 : .16),black(dark ? .12 : .06))];
    const fieldShadow = clean ? smallShadow : [shadow(0,0,0,1,color("field.line")),shadow(0,1,2,0,black(.04))];
    const focusShadow = [shadow(0,0,0,1,color("field.focus_edge")),shadow(0,1,2,0,black(clean ? .06 : .04))];
    const recipes = {};
    const buttonRoot = `style().items_center().justify_center().min_height(${length("metrics.button_h")}).padding_x(${length("metrics.button_pad")}).gap(${length("metrics.button_gap")}).radius(${length("metrics.button_radius")}).border(px(${clean ? 0 : 1})).border_color(${transparent})${t("button")}.font_weight(500).letter_spacing_em(${clean ? 0 : -0.01}).whitespace_nowrap()`;
    const buttonFill = (top, bottom) => layeredGradient([
      gradientLayer([color(top), color(bottom)], "padding"),
      gradientLayer(["top", "mid", "bottom"].map(stop => color(`button.edge_${stop}`))),
    ]);
    recipes.button = {
      primary: clean
        ? `style().background(${color("button.primary_bg")}).text_color(${color("button.primary_fg")}).hover(style().background(${color("button.primary_hover")}))`
        : `style()${buttonFill("button.primary_bg","button.primary_bottom")}.text_color(${color("button.primary_fg")})${shadows([shadow(0,0,0,1,color("button.ring")),shortDrop(black(dark ? .4 : .1))])}.hover(style()${buttonFill("button.primary_hover","button.primary_hover_bottom")})`,
      secondary: `style().background(${color("button.secondary_bg")}).text_color(${strongest}).border_color(${clean ? transparent : edge})${shadows(clean ? smallShadow : [shadow(0,1,2,0,black(.06))])}.hover(style().background(${color("button.secondary_hover")}))`,
      ghost: `style().background(${transparent}).text_color(${color("button.ghost_fg")}).shadows([]).hover(style().background(${wash}).text_color(${color("button.ghost_hover_fg")}))`,
      danger: `style().background(${color("button.danger_bg")}).text_color(${color("danger")}).shadows([]).hover(style().background(${color("button.danger_hover")}))`,
      outline: `style().background(${transparent}).border(px(1)).border_color(${edgeStrong}).text_color(${ink}).hover(style().background(${wash}))`,
      selected: `style().background(${selected}).text_color(${strongest}).shadows([])`,
    };
    for (const tone of ["warning","success"]) recipes.button[tone] = `style().background(${color(`badge.${tone}_bg`)}).text_color(${color(`button.${tone}_fg`)}).hover(style().background(mix(${color(`badge.${tone}_bg`)},${color(`badge.${tone}_fg`)},0.08)))`;
    recipes.field = {
      normal: `style().background(${field}).border(px(0))${shadows(fieldShadow)}.hover(style()${shadows([shadow(0,0,0,clean ? .5 : 1,color("field.edge")),shadow(0,1,2,0,black(.04))])}).focus(style()${shadows(focusShadow)})`,
      invalid: `style().border_color(${color("danger")})${shadows([shadow(0,0,0,1,`alpha(${color("danger")},0.6)`),shadow(0,1,2,0,black(.05))])}.focus(style()${shadows([shadow(0,0,0,1,color("danger")),shadow(0,0,0,3,`alpha(${color("danger")},0.1)`)])})`,
      embedded: `style().border(px(0)).border_bottom(px(1)).border_color(${edge}).radius(px(0)).background(${transparent}).shadows([]).focus(style().shadows([]).border_color(${edge}))`,
      read_only_recipe: `style().background(${color("field.read_only")})`,
    };
    const cardShadow = clean
      ? [shadow(0,0,0,.5,dark ? white(.1) : black(.06)),shadow(0,1,2,0,black(dark ? .4 : .05))]
      : [shadow(0,0,0,1,rgba(dark ? "2a2a2e" : "ececec")),shadow(0,1,2,0,black(dark ? .2 : .04))];
    const cardHover = clean
      ? [shadow(0,0,0,.5,dark ? white(.16) : black(.1)),shortDrop(black(dark ? .4 : .08))]
      : [shadow(0,0,0,1,rgba(dark ? "3a3a40" : "d9d9d9")),shortDrop(black(.08),0,-1)];
    const cardSelected = clean
      ? [shadow(0,0,0,.5,dark ? white(.2) : black(.14)),shadow(0,1,2,0,black(dark ? .4 : .05))]
      : [shadow(0,0,0,1,rgba(dark ? "3a3a40" : "d9d9d9")),shadow(0,1,2,0,black(.04))];
    recipes.card = {
      root_interactive: `style().hover(style()${clean ? shadows(cardHover) : ""})`,
      root_selected: `style().background(${color("card.selected")})${clean ? shadows(cardSelected) : ""}`,
      surface_selected: `style()${clean ? "" : shadows(cardSelected)}`,
      surface_hover: `style()${clean ? "" : shadows(cardHover)}`,
      root_outline: `style().background(${transparent})`,
    };
    recipes.choice = {
      checked: `style().background(${color("choice.fill")}).shadows([])`,
      unchecked: `style().background(${field})${shadows([shadow(0,0,0,1,color("choice.edge"),true),shadow(0,1,1,0,black(.04))])}.group_hover(style()${shadows([shadow(0,0,0,1,color("choice.edge_hover"),true),shadow(0,1,1,0,black(.04))])})`,
    };
    // Tag is derived from the removable-chip geometry, with optional facets.
    // A tone is always a foreground/background pair, never a tinted fill with
    // an unrelated solid-action inverse foreground.
    recipes.tag = {
      normal: `style().text_color(${ink})`,
      facet_neutral: `style().background(${color("tag.facet")}).text_color(${ink})`,
    };
    for (const variant of ["accent","success","warning","danger"]) {
      const tone = variant === "accent" ? "info" : variant;
      const foreground = color(`tag.${tone}_fg`);
      const background = color(`tag.${tone}_bg`);
      // Resolve the contrast search once per theme generation, not for every
      // chip drawn in a scrolling frame. The style consumes named color roles.
      colors[`tag.text_${variant}_ink`] = `readable(${foreground},${ink},${color("tag.chip")}.over(${color("surface_raised")}),4.5)`;
      colors[`tag.facet_${variant}_ink`] = `readable(${foreground},${ink},${background},4.5)`;
      recipes.tag[`text_${variant}`] = `style().text_color(${color(`tag.text_${variant}_ink`)})`;
      recipes.tag[`facet_${variant}`] = `style().background(${background}).text_color(${color(`tag.facet_${variant}_ink`)})`;
    }
    const switchThumbShadow = clean
      ? [shadow(0,1,2,0,black(.18)),shadow(0,0,0,.5,black(.04))]
      : [shadow(0,1,2,0,black(.2))];
    recipes.switch = {
      track_on: `style().background(${color("choice.fill")}).group_hover(style().background(${color("switch.on_hover")}))`,
      track_off: `style().background(${color("switch.off")}).group_hover(style().background(${color("switch.off_hover")}))`,
      thumb_on: `style().background(${color("switch.checked_thumb")})${shadows(dark ? [] : switchThumbShadow)}`,
      thumb_off: `style().background(${color("switch.thumb")})${shadows(switchThumbShadow)}`,
    };
    const parts = Object.fromEntries(modules.map(module => [module.id, {}]));
    const set = (id, name, value) => {
      const module = modules.find(module => module.id === id);
      if (!module?.parts.includes(name)) throw new Error(`unknown recipe part ${id}/${name}`);
      parts[id][name] = value;
    };
    const cset = (name, part, value) => set(`components/${name}`,part,value);
    // Nonvisual roots intentionally have no surface/border/radius recipe. Geometry
    // remains owned by the host and the component's interaction contract.
    for (const name of ["drag_source","draggable","drop_zone","pan_zoom","resizable","rotatable","selection_area","sortable","sliding_panel"]) {
      cset(name,"root","style()");
    }
    for (const name of ["alert_dialog","command_dialog","context_menu","menu","date_picker","date_range_picker","dialog","popover","sheet","select","combobox","tooltip","time_range_picker","split_pane","scroll_area"]) {
      cset(name,"root","style()");
    }
    cset("tab_overflow_menu","trigger","style()");
    cset("button","root",`${buttonRoot}.focus_ring(px(2),px(2),${focus})`);
    for (const part of ["prefix","suffix","spinner"]) cset("button",part,`style().width(${length("metrics.button_icon")}).height(${length("metrics.button_icon")}).flex_shrink(false).items_center().justify_center()`);
    cset("button","label",`style().whitespace_nowrap()`);
    // The caption voice already makes the inline legend secondary. Stacking
    // legacy opacity onto a muted ghost foreground dims that voice twice.
    cset("button","shortcut",`style()${t("caption")}.opacity(1.0)`);
    cset("icon_button","root",`style().width(${length("metrics.icon_button_h")}).height(${length("metrics.icon_button_h")}).min_height(px(0)).padding(px(0)).border(px(${clean ? 0 : 1})).border_color(${transparent}).radius(${length("metrics.button_radius")}).focus_ring(px(2),px(2),${focus})`);
    for (const part of ["icon","spinner"]) cset("icon_button",part,`style().width(${length("metrics.icon_button_icon")}).height(${length("metrics.icon_button_icon")}).flex_shrink(false)`);
    cset("input","root",`style().min_height(${length("metrics.field_h")})${t("control")}.font_weight(400)`);
    const tagInputFocus = [shadow(0,0,0,1,color("tag_input.focus")),shadow(0,1,2,0,black(dark ? .4 : .06))];
    const tagInputError = [shadow(0,0,0,1,color("tag_input.error_edge")),shadow(0,1,2,0,black(dark ? .4 : .04))];
    cset("tag_input","root",`style().gap(px(${clean ? 6 : 7}))`);
    cset("tag_input","header",`style().gap(px(6)).padding_x(px(${clean ? 0 : 2}))`);
    cset("tag_input","label",`style()${t("control")}.font_weight(500).text_color(${ink})`);
    cset("tag_input","optional",`style().text_color(${faint})`);
    cset("tag_input","field",`style().border(px(0)).min_height(px(${clean ? 32 : 36})).max_height(px(${clean ? 156 : 168})).padding_x(px(${clean ? 4 : 6})).padding_y(px(${clean ? 3 : 5})).radius(${length("metrics.field_radius")}).background(${field})${shadows(fieldShadow)}.hover(style()${shadows([shadow(0,0,0,clean ? .5 : 1,edgeStrong),shadow(0,1,2,0,black(dark ? .4 : .06))])}).group_focus(style()${shadows(tagInputFocus)})`);
    cset("tag_input","invalid",`style().border(px(0)).background(${color("tag_input.error_fill")})${shadows(tagInputError)}.hover(style()${shadows(tagInputError)}).group_focus(style()${shadows(tagInputError)})`);
    cset("tag_input","editor",`style()${t("control")}.font_size(px(${clean ? 13 : 13.5})).line_height(px(20)).text_color(${ink})`);
    cset("tag_input","placeholder",`style().text_color(${placeholder})`);
    cset("tag_input","chip",`style().height(px(24)).min_height(px(24)).radius(px(${clean ? 5 : 7})).border(px(0)).background(${color("tag_input.chip")}).text_color(${ink})${t("control")}.font_size(px(${clean ? 12.5 : 13})).line_height(px(20))${shadows([shadow(0,0,0,clean ? .5 : 1,color("tag_input.chip_edge"),true)])}`);
    cset("tag_input","chip_armed",`style().background(${color("tag_input.armed")})`);
    cset("tag_input","chip_avatar",`style().background(${color("tag_input.avatar")}).text_color(${muted}).font_size(px(10)).line_height(px(16))`);
    cset("tag_input","chip_remove",`style().text_color(${faint}).hover(style().background(${color("tag_input.armed")}).text_color(${ink}))`);
    cset("tag_input","chip_flash",`style().radius(px(${clean ? 5 : 7})).background(${color("tag_input.flash")})`);
    cset("tag_input","footer",`style().padding_x(px(${clean ? 0 : 2}))`);
    cset("tag_input","hint",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(${clean ? 16 : 17})).text_color(${faint})`);
    cset("tag_input","count",`style().text_color(${placeholder})`);
    cset("tag_input","count_full",`style().text_color(${muted})`);
    const filterRadius = clean ? 6 : 10;
    const filterChoice = clean ? 14 : 20;
    const filterPaper = color("filter_menu.card");
    const filterHover = color("filter_menu.hover");
    cset("filter_menu","root",`style().max_width(px(${clean ? 400 : 420})).radius(px(${clean ? 10 : 24})).padding(px(${clean ? 0 : 6})).background(${color("filter_menu.tray")})${shadows(clean ? smallShadow : [])}`);
    cset("filter_menu","card",`style().padding(px(0)).padding_x(px(${clean ? 0 : 16})).padding_top(px(${clean ? 0 : 16})).padding_bottom(px(${clean ? 0 : 10})).border(px(${clean ? 0 : 1})).border_color(${edge}).radius(px(${clean ? 10 : 20})).background(${filterPaper})${shadows(clean ? [] : [shadow(0,1,2,0,black(dark ? .4 : .06))])}`);
    cset("filter_menu","header",`style().padding_start(px(${clean ? 16 : 2})).padding_end(px(${clean ? 10 : 2})).padding_top(px(${clean ? 12 : 0})).padding_bottom(px(${clean ? 8 : 0}))`);
    cset("filter_menu","title",`style().font_size(px(15)).line_height(px(24)).font_weight(${clean ? 500 : 600}).letter_spacing_em(${clean ? "0.0" : "-0.01"}).text_color(${strongest})`);
    cset("filter_menu","clear",`style().height(px(28)).padding_x(px(${clean ? 8 : 6})).radius(px(${clean ? 6 : 8})).font_size(px(13)).text_color(${clean ? muted : faint}).hover(style().background(${filterHover}).text_color(${ink}))`);
    cset("filter_menu","chips",`style().gap(px(${clean ? 4 : 6})).padding_x(px(${clean ? 16 : 0})).padding_top(px(${clean ? 0 : 12})).padding_bottom(px(${clean ? 12 : 0}))`);
    cset("filter_menu","chip",`style().height(px(${clean ? 24 : 30})).min_height(px(${clean ? 24 : 30})).border(px(0)).radius(px(${filterRadius})).background(${color("filter_menu.chip")}).font_size(px(${clean ? 12 : 13})).line_height(px(${clean ? 16 : 20})).font_weight(${clean ? 500 : 400})${shadows([])}`);
    cset("filter_menu","chip_body",`style().padding_start(px(${clean ? 8 : 11})).padding_end(px(${clean ? 3 : 4}))`);
    cset("filter_menu","chip_remove",`style().width(px(${clean ? 18 : 22})).height(px(${clean ? 18 : 22})).radius(px(${clean ? 4 : 6})).text_color(${faint}).hover(style().background(${color("filter_menu.chip_hover")}).text_color(${ink}))`);
    cset("filter_menu","chip_icon",`style().width(px(${clean ? 12 : 14})).height(px(${clean ? 12 : 14}))`);
    cset("filter_menu","more",`style().font_size(px(12)).line_height(px(24)).font_feature("tnum",1).text_color(${muted})`);
    cset("filter_menu","groups",`style().padding(px(${clean ? 4 : 0})).padding_top(px(${clean ? 4 : 6})).border_top(px(${clean ? .5 : 0})).border_color(${edge})`);
    cset("filter_menu","trigger",`style().min_height(px(${clean ? 32 : 36})).padding_x(px(${clean ? 8 : 6})).gap(px(${clean ? 8 : 7})).radius(px(${filterRadius})).hover(style().background(${filterHover}))`);
    cset("filter_menu","chevron",`style().text_color(${clean ? faint : ink})`);
    cset("filter_menu","group_label",`style().font_size(px(${clean ? 13 : 14})).font_weight(500).text_color(${ink})`);
    cset("filter_menu","total",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(16)).font_feature("tnum",1).text_color(${clean ? placeholder : faint})`);
    cset("filter_menu","selected_count",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(16)).text_color(${faint})`);
    cset("filter_menu","options",`style().max_height(px(${clean ? 320 : 340})).padding_top(px(${clean ? 0 : 2})).padding_bottom(px(${clean ? 4 : 6}))`);
    cset("filter_menu","option",`style().height(px(${clean ? 32 : 34})).min_height(px(${clean ? 32 : 34})).padding_start(px(${clean ? 32 : 6})).padding_end(px(${clean ? 8 : 6})).gap(px(${clean ? 8 : 12})).radius(px(${filterRadius})).hover(style().background(${filterHover}))`);
    cset("filter_menu","option_label",`style().font_size(px(${clean ? 13 : 14})).text_color(${ink})`);
    cset("filter_menu","choice_control",`style().width(px(${filterChoice})).height(px(${filterChoice})).min_height(px(0))`);
    cset("filter_menu","choice_box",`style().width(px(${filterChoice})).height(px(${filterChoice})).radius(px(${clean ? 4 : 6})).border(px(0)).background(${color("filter_menu.choice")})${shadows([shadow(0,0,0,clean ? .5 : 1,color("filter_menu.choice_edge"),true)])}`);
    cset("filter_menu","choice_on",`style().background(${color("filter_menu.choice_on")})${shadows([])}`);
    cset("filter_menu","choice_mark",`style().width(px(${clean ? 12 : 14})).height(px(${clean ? 12 : 14})).text_color(${color("filter_menu.mark")})`);
    cset("filter_menu","hints",`style().padding_x(px(${clean ? 12 : 8})).padding_top(px(${clean ? 8 : 10})).padding_bottom(px(${clean ? 8 : 4})).border_top(px(${clean ? .5 : 0})).border_color(${edge}).text_color(${clean ? faint : ink})`);
    cset("filter_menu","hint_key",`style().height(px(${clean ? 18 : 22})).min_width(px(${clean ? 18 : 22})).background(${filterPaper}).text_color(${clean ? muted : ink})${shadows(smallShadow)}`);
    const stepperPaper = color("stepper.card");
    const stepperTrack = color("stepper.track");
    const stepperAccent = color("stepper.progress");
    cset("stepper_form","root",`style().max_width(px(520)).padding(px(${clean ? 0 : 6})).gap(px(${clean ? 0 : 4})).radius(px(${clean ? 10 : 22})).background(${color("stepper.tray")})${shadows(clean ? smallShadow : [])}`);
    cset("stepper_form","header",`style().padding_x(px(${clean ? 20 : 6})).padding_top(px(${clean ? 16 : 6})).padding_bottom(px(${clean ? 0 : 4}))`);
    cset("stepper_form","progress",`style().gap(px(${clean ? 8 : 0}))`);
    cset("stepper_form","step",`style().padding(px(${clean ? 0 : 3})).gap(px(${clean ? 5 : 0})).radius(px(${clean ? 6 : 8})).${clean ? "items_start" : "items_center"}().hover(style().background(${wash}))`);
    cset("stepper_form","step_current",`style().text_color(${strongest})`);
    cset("stepper_form","step_done",`style().text_color(${muted})`);
    cset("stepper_form","step_future",`style().text_color(${faint})`);
    cset("stepper_form","marker",`style().width(px(14)).height(px(${clean ? 20 : 26})).font_size(px(${clean ? 12 : 12.5})).text_color(${placeholder})`);
    cset("stepper_form","marker_current",`style().text_color(${stepperAccent})`);
    cset("stepper_form","marker_done",`style().text_color(${stepperAccent})`);
    cset("stepper_form","check",`style().width(px(${clean ? 12 : 13})).height(px(${clean ? 12 : 13}))`);
    cset("stepper_form","step_name",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(16))`);
    cset("stepper_form","bar",`style().${clean ? "block" : "hidden"}().height(px(${clean ? 2 : 0})).background(${stepperTrack})`);
    cset("stepper_form","bar_active",`style().background(${stepperAccent})`);
    cset("stepper_form","compact",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(16)).text_color(${muted})`);
    cset("stepper_form","card",`style().radius(px(${clean ? 10 : 16})).background(${stepperPaper})${shadows(clean ? [] : fieldShadow)}`);
    cset("stepper_form","pane",`style().padding_x(px(20)).padding_top(px(20)).padding_bottom(px(${clean ? 20 : 8}))`);
    cset("stepper_form","heading",`style().font_size(px(${clean ? 15 : 16})).line_height(px(${clean ? 24 : 22})).font_weight(${clean ? 500 : 600}).letter_spacing_em(${clean ? "0.0" : "-0.01"}).text_color(${strongest})`);
    cset("stepper_form","description",`style().text_color(${muted})`);
    cset("stepper_form","body",`style().gap(px(16)).padding_top(px(12))`);
    cset("stepper_form","review_row",`style().min_height(px(${clean ? 34 : 36})).padding_y(px(6)).border_bottom(px(${clean ? .5 : 1})).border_color(${stepperTrack})`);
    cset("stepper_form","edit",`style().height(px(24)).min_height(px(24)).padding_x(px(8)).font_size(px(12)).line_height(px(16))`);
    cset("stepper_form","footer",`style().min_height(px(${clean ? 48 : 52})).padding_x(px(${clean ? 12 : 16})).padding_top(px(${clean ? 6 : 12})).padding_bottom(px(${clean ? 6 : 16})).border_top(px(${clean ? .5 : 0})).border_color(${stepperTrack})`);
    for (const part of ["back", "primary"]) cset("stepper_form",part,`style().height(px(${clean ? 28 : 36})).min_height(px(${clean ? 28 : 36}))`);
    cset("stepper_form","primary_busy",`style().disabled(style().opacity(0.7))`);
    cset("stepper_form","success",`style().padding(px(${clean ? 24 : 30})).gap(px(12))`);
    cset("stepper_form","success_mark",`style().width(px(36)).height(px(36)).background(${color("stepper.success_fill")}).text_color(${color("text.success")})`);
    const checklistSoft = color("checklist.muted");
    const checklistTrack = color("checklist.track");
    const guideBlue = color("checklist.setup_fill");
    const trackerFill = color("checklist.tracker_fill");
    const checklistShadow = clean ? smallShadow :
      [shadow(0,1,2,0,black(dark ? .25 : .03)),shortDrop(black(dark ? .20 : .035))];
    for (const name of ["progress_tracker", "setup_guide"]) {
      const tracker = name === "progress_tracker";
      cset(name,"root",`style().max_width(px(${tracker ? (clean ? 340 : 360) : 320})).border(px(${clean ? 0 : 1})).border_color(${edge}).padding_x(px(${tracker ? (clean ? 6 : 16) : 8})).padding_top(px(${tracker ? (clean ? 14 : 22) : (clean ? 12 : 16)})).padding_bottom(px(${tracker ? (clean ? 6 : 14) : 8})).gap(px(${tracker ? (clean ? 8 : 12) : 8})).radius(px(${clean ? 10 : 20})).background(${field})${shadows(checklistShadow)}`);
      cset(name,"header",`style().padding_x(px(${tracker ? (clean ? 10 : 6) : 8}))`);
      cset(name,"title",`style().font_size(px(${tracker && !clean ? 16 : 15})).line_height(px(24)).font_weight(${tracker || clean ? 500 : 600}).text_color(${strongest})`);
      cset(name,"count",`style().font_size(px(${clean ? 12 : 14})).line_height(px(20)).text_color(${checklistSoft})`);
      cset(name,"digit",`style().text_color(${clean ? ink : checklistSoft})`);
      const height = tracker ? (clean ? 4 : 8) : (clean ? 4 : 5);
      cset(name,"track",`style().height(px(${height})).background(${checklistTrack})`);
      cset(name,"fill",`style().background(${tracker ? trackerFill : guideBlue})`);
      cset(name,"label",`style().font_size(px(${tracker && !clean ? 15 : 13})).line_height(px(20)).text_color(${ink})`);
      cset(name,"label_done",`style().text_color(${checklistSoft})`);
      cset(name,"strike",`style().height(px(${clean ? 1 : 1.5}))`);
      const mark = tracker && !clean ? 20 : 16;
      cset(name,"mark",`style().width(px(${mark})).height(px(${mark})).border(px(${tracker && !clean ? 1.5 : 0})).border_color(${color("checklist.ring")}).background(${!tracker && !clean ? color("checklist.current_dot") : transparent})${shadows(clean ? [shadow(0,0,0,1,color("checklist.ring"),true)] : [])}`);
      cset(name,"mark_done",`style().background(${tracker ? (clean ? trackerFill : color("button.primary_bg")) : (clean ? transparent : guideBlue)}).border(px(0))${shadows([])}`);
      cset(name,"check",`style().width(px(${clean ? 12 : 14})).height(px(${clean ? 12 : 14})).text_color(${clean && !tracker ? guideBlue : (tracker && !clean ? color("button.primary_fg") : rgba("ffffff"))})`);
    }
    cset("progress_tracker","list",`style().gap(px(${clean ? 0 : 2}))`);
    cset("progress_tracker","row",`style().min_height(px(${clean ? 32 : 40})).padding_x(px(${clean ? 10 : 6})).gap(px(${clean ? 10 : 12})).radius(px(${clean ? 6 : 12}))`);
    cset("progress_tracker","icon",`style().width(px(${clean ? 16 : 18})).height(px(${clean ? 16 : 18})).text_color(${muted})`);
    cset("progress_tracker","icon_done",`style().opacity(0.48)`);
    cset("setup_guide","progress",`style().padding_x(px(8))`);
    cset("setup_guide","icon_button",`style().width(px(${clean ? 24 : 26})).height(px(${clean ? 24 : 26})).min_height(px(${clean ? 24 : 26})).radius(px(${clean ? 5 : 7}))`);
    cset("setup_guide","icon",`style().text_color(${muted})`);
    cset("setup_guide","edit",`style().min_height(px(24)).padding_x(px(6)).font_size(px(${clean ? 13 : 14})).text_color(${clean ? muted : guideBlue})`);
    cset("setup_guide","group",`style().radius(px(${clean ? 7 : 12}))`);
    cset("setup_guide","group_open",`style().background(${color("checklist.open")})`);
    cset("setup_guide","group_header",`style().min_height(px(${clean ? 32 : 42})).padding_x(px(${clean ? 8 : 12})).radius(px(${clean ? 6 : 12}))`);
    cset("setup_guide","group_title",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(${clean ? 500 : 600})`);
    cset("setup_guide","group_done",`style().text_color(${checklistSoft})`);
    cset("setup_guide","task",`style().min_height(px(${clean ? 30 : 34})).padding_x(px(${clean ? 6 : 10})).gap(px(8))`);
    cset("setup_guide","mark_locked",`style().background(${transparent}).border(px(0))${shadows([])}`);
    cset("setup_guide","locked_icon",`style().text_color(${color("checklist.locked_icon")})`);
    cset("setup_guide","label_locked",`style().text_color(${checklistSoft})`);
    cset("reaction_bar","root",`style().self_start().max_width(relative(1.0)).${clean ? "justify_start" : "justify_center"}().padding_x(px(${clean ? 0 : 10})).padding_y(px(${clean ? 0 : 4})).gap(px(${clean ? 4 : 2})).radius(px(${clean ? 0 : 25})).background(${clean ? transparent : panel})${shadows(clean ? [] : checklistShadow)}`);
    cset("reaction_bar","action",`style().min_width(px(${clean ? 32 : 42})).height(px(${clean ? 28 : 42})).min_height(px(${clean ? 28 : 42})).padding_x(px(${clean ? 8 : 10})).gap(px(${clean ? 4 : 6})).radius(px(${clean ? 6 : 999})).font_size(px(${clean ? 13 : 14})).font_weight(500)${shadows(clean ? [shadow(0,0,0,.5,edge,true)] : [])}`);
    cset("reaction_bar","action_active",clean ?
        `style().text_color(${color("reaction.selected")}).background(${color("reaction.selected_fill")}).without_hover()${shadows([shadow(0,0,0,.5,color("reaction.selected_edge"),true)])}` : `style()`);
    cset("reaction_bar","icon",`style().width(px(${clean ? 16 : 22})).height(px(${clean ? 16 : 22}))`);
    cset("message_actions","root",`style()`);
    cset("message_actions","bar",`style().min_height(px(${clean ? 28 : 30})).gap(px(2))`);
    cset("message_actions","action",`style().width(px(${clean ? 28 : 30})).height(px(${clean ? 28 : 30})).radius(px(${clean ? 6 : 8})).text_color(${muted})`);
    cset("message_actions","pressed",`style().text_color(${ink}).background(${wash})`);
    cset("message_actions","copied",`style().text_color(${color("text.success")})`);
    cset("message_actions","counter",`style().gap(px(2)).padding_x(px(2)).font_size(px(12)).line_height(px(16))`);
    cset("message_actions","total",`style().text_color(${clean ? faint : muted})`);
    cset("message_actions","feedback",`style().max_width(px(440)).padding_x(px(${clean ? 8 : 14})).padding_y(px(${clean ? 8 : 12})).radius(px(${clean ? 0 : 16})).border(px(${clean ? 0 : 1})).border_color(${edge}).background(${clean ? transparent : panel})${shadows(clean ? [] : checklistShadow)}`);
    cset("message_actions","reason",`style().min_height(px(${clean ? 28 : 30})).radius(px(${clean ? 6 : 8})).padding_x(px(10))${shadows(clean ? [shadow(0,0,0,.5,edge,true)] : [])}`);
    cset("message_actions","heading",`style().font_size(px(${clean ? 13 : 14})).font_weight(500)`);
    cset("message_actions","send",`style().min_height(px(${clean ? 28 : 32})).radius(px(${clean ? 6 : 8}))`);
    cset("message_actions","thanks",`style().disabled(style().opacity(1).background(${color("badge.success_bg")}).text_color(${color("text.success")}))`);
    set("patterns/model_picker","root",`style()`);
    set("patterns/model_picker","trigger",`style().min_height(px(${clean ? 28 : 30})).gap(px(${clean ? 6 : 7})).padding_x(px(${clean ? 6 : 8})).radius(px(${clean ? 6 : 10})).text_color(${clean ? muted : ink})`);
    set("patterns/model_picker","model_icon",`style().width(px(18)).height(px(18)).radius(px(${clean ? 5 : 6})).background(${wash}).text_color(${muted})`);
    set("patterns/model_picker","panel",`style().width(px(${clean ? 320 : 352})).padding(px(${clean ? 4 : 6})).radius(px(${clean ? 10 : 16})).border(px(${clean ? .5 : 1})).border_color(${edge}).background(${panel})${shadows(floatingCast())}`);
    set("patterns/model_picker","heading",`style().min_height(px(24)).font_size(px(11)).text_color(${muted})`);
    set("patterns/model_picker","item",`style().min_height(px(${clean ? 42 : 48})).radius(px(${clean ? 6 : 9})).padding_x(px(7))`);
    set("patterns/model_picker","selected",`style().background(${selected})`);
    set("patterns/model_picker","active",`style().background(${wash})`);
    set("patterns/model_picker","description",`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    set("patterns/model_picker","settings",`style().border_top(px(0.5)).border_color(${edge}).padding_top(px(4))`);
    set("patterns/model_picker","footer",`style().min_height(px(${clean ? 34 : 37})).border_top(px(${clean ? .5 : 0})).border_color(${edge}).items_center().padding_x(px(10)).text_color(${muted})`);
    const citeTray=color("cite.tray"), citeWell=color("cite.well");
    cset("cited_answer","root",`style().width(relative(1.0)).max_width(px(460)).padding(px(${clean ? 0 : 4})).radius(px(${clean ? 0 : 20})).background(${citeTray})`);
    cset("cited_answer","embedded",`style().max_width(relative(1.0)).padding(px(0)).radius(px(0)).background(${transparent})`);
    cset("cited_answer","card",`style().padding_x(px(${clean ? 0 : 16})).padding_y(px(${clean ? 0 : 12})).radius(px(${clean ? 0 : 16})).background(${clean ? transparent : panel})${shadows(clean ? [] : checklistShadow)}`);
    cset("cited_answer","header",`style().min_height(px(${clean ? 16 : 22}))`);
    cset("cited_answer","title",`style().font_size(px(${clean ? 13 : 14})).font_weight(600)`);
    cset("cited_answer","summary",`style().font_size(px(${clean ? 12 : 12.5})).text_color(${faint})`);
    cset("cited_answer","paragraph",`style().padding_y(px(3)).font_size(px(${clean ? 13 : 14})).line_height(px(${clean ? 22 : 23})).text_color(${clean ? ink : color("card.body")})`);
    cset("cited_answer","chip",`style().min_height(px(28)).gap(px(6)).padding_x(px(7)).radius(px(${clean ? 6 : 8})).border(px(0)).background(${clean ? transparent : citeWell})${shadows(clean ? [shadow(0,0,0,.5,edge,true)] : [])}`);
    cset("cited_answer","chip_active",`style().background(${clean ? citeWell : panel}).border(px(0))${shadows([shadow(0,0,0,clean ? .5 : 1,edgeStrong,true)])}`);
    cset("cited_answer","number",`style().font_size(px(${clean ? 11 : 11.5})).font_feature("tnum",1).text_color(${faint})`);
    cset("cited_answer","tag",`style().background(${citeWell}).text_color(${clean ? muted : color("card.body")}).font_weight(${clean ? 500 : 600})`);
    cset("cited_answer","tag_active",`style().background(${clean ? citeWell : ink}).text_color(${clean ? ink : panel}).font_weight(600)`);
    cset("cited_answer","annotation",`style().background(${color("cite.highlight")})`);
    cset("cited_answer","preview",`style().padding_x(px(12)).padding_y(px(10)).radius(px(${clean ? 8 : 12})).border(px(0)).background(${clean ? panel : citeWell})${shadows(clean ? smallShadow : [])}`);
    cset("cited_answer","quote",`style().font_size(px(${clean ? 13 : 14})).line_height(px(21)).border_color(${color("cite.highlight")})`);
    cset("cited_answer","quote_annotation",`style().background(${color("cite.highlight")})`);
    cset("cited_answer","tray",`style().min_height(px(${clean ? 32 : 34})).padding_x(px(${clean ? 0 : 8})).gap(px(4))`);
    cset("cited_answer","caption",`style().text_color(${muted})`);
    cset("ai_answer","root",`style()`);
    cset("ai_answer","framed",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 0 : 20})).background(${citeTray})`);
    cset("ai_answer","card",`style().padding_x(px(${clean ? 20 : 24})).padding_top(px(${clean ? 20 : 22})).padding_bottom(px(${clean ? 16 : 20})).radius(px(${clean ? 10 : 16})).background(${panel})${shadows(checklistShadow)}`);
    cset("ai_answer","question",`style().font_size(px(${clean ? 15 : 18})).line_height(px(24)).font_weight(${clean ? 500 : 600})`);
    cset("ai_answer","follow_ups",`style().padding_top(px(${clean ? 10 : 14})).border_top(px(${clean ? .5 : 1})).border_color(${edge})`);
    cset("ai_answer","follow_button",`style().width(relative(1.0)).justify_start().min_width(px(0)).min_height(px(${clean ? 28 : 32})).padding_x(px(4))`);
    const thinShadow = [shadow(0,0,0,.5,edge,true),shadow(0,1,2,0,black(dark ? .28 : .05))];
    cset("thinking_loader","root",`style().min_height(${length("metrics.thinking_h")}).gap(${length("metrics.thinking_gap")}).padding_start(${length("metrics.thinking_pad_start")}).padding_end(${length("metrics.thinking_pad_end")}).border(px(0)).radius(${length("metrics.thinking_radius")}).background(${panel}).text_color(${color("thinking.ink")})${shadows(clean ? thinShadow : [shadow(0,0,0,1,edge,true),shadow(0,1,2,0,black(dark ? .28 : .04))])}`);
    cset("thinking_loader","plain",`style().padding(px(0)).padding_start(px(0)).padding_end(px(0)).border(px(0)).min_height(${length("metrics.thinking_plain_h")}).background(${transparent})${shadows([])}`);
    cset("thinking_loader","icon",`style().width(${length("metrics.thinking_icon")}).height(${length("metrics.thinking_icon")})`);
    cset("thinking_loader","label",`style().font_size(${length("metrics.thinking_font")}).line_height(${length("metrics.thinking_line")}).font_weight(${clean ? 500 : 450})`);
    cset("thinking_loader","check",`style().text_color(${color("thinking.check")})`);
    cset("thinking_loader","finished",`style().text_color(${ink})`);
    cset("thinking_loader","ray",`style().text_color(${clean ? color("thinking.check") : ink})`);
    cset("thinking_state","root",`style().max_width(px(440))`);
    cset("thinking_state","header",`style().min_height(px(${clean ? 26 : 28})).gap(px(${clean ? 6 : 10})).padding_x(px(${clean ? 0 : 4})).radius(px(${clean ? 6 : 8})).text_color(${color("thinking.ink")})`);
    cset("thinking_state","head_label",`style().font_size(px(${clean ? 13 : 15})).line_height(px(${clean ? 19 : 21})).font_weight(500)`);
    cset("thinking_state","elapsed",`style().font_size(px(12)).text_color(${faint})`);
    cset("thinking_state","row",`style().gap(px(${clean ? 8 : 12})).padding_bottom(px(${clean ? 9 : 12}))`);
    cset("thinking_state","title",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).text_color(${clean ? ink : color("card.body")})`);
    cset("thinking_state","active",`style().text_color(${ink}).font_weight(500)`);
    cset("thinking_state","source",`style().min_height(px(${clean ? 23 : 25})).padding_x(px(7)).radius(px(${clean ? 6 : 8})).background(${clean ? transparent : wash}).text_color(${ink})${shadows(clean ? [shadow(0,0,0,.5,edge,true)] : [])}`);
    cset("thinking_state","done_mark",`style().text_color(${color("thinking.check")})`);
    cset("thinking_state","terminal",`style().font_size(px(${clean ? 13 : 14})).font_weight(500)`);
    const runset=(part,value)=>set("patterns/run_controls",part,value);
    runset("root",`style().padding(px(${clean ? 0 : 4})).gap(px(${clean ? 0 : 4})).radius(px(${clean ? 10 : 20})).background(${color("run.tray")})${shadows(clean ? smallShadow : [])}`);
    runset("log",`style().padding_x(px(16)).padding_top(px(14)).padding_bottom(px(8)).radius(px(${clean ? 10 : 16})).background(${panel})${shadows(clean ? [] : checklistShadow)}`);
    runset("empty",`style().min_height(px(195)).font_size(px(${clean ? 13 : 14}))`);
    runset("prompt",`style().self_end().max_width(relative(0.82)).padding_x(px(${clean ? 12 : 13})).padding_y(px(${clean ? 7 : 8})).radius(px(${clean ? 14 : 16})).background(${wash})${shadows(clean ? [shadow(0,0,0,.5,edge,true)] : [])}`);
    runset("agent",`style().min_height(px(22)).gap(px(8)).padding_top(px(8)).font_size(px(${clean ? 12 : 12.5}))`);
    runset("name",`style().font_weight(${clean ? 500 : 600}).text_color(${ink})`);
    runset("status",`style().font_size(px(12))`);
    runset("answer",`style().padding_start(px(24)).font_size(px(${clean ? 13 : 14})).line_height(px(20)).text_color(${ink})`);
    runset("steer",`style().self_start().margin_start(px(24)).padding_x(px(7)).min_height(px(22)).radius(px(999)).background(${color("run.steer")}).text_color(${color("run.steer_ink")})`);
    runset("after",`style().min_height(px(26)).padding_x(px(12)).gap(px(4))`);
    runset("note",`style().font_size(px(12)).text_color(${muted})`);
    runset("composer",`style().padding(px(${clean ? 4 : 0})).radius(px(${clean ? 10 : 16})).background(${color("run.queue")})`);
    runset("queue",`style().max_height(px(128)).padding_x(px(${clean ? 4 : 6}))`);
    runset("queued",`style().min_height(px(30)).font_size(px(12.5)).text_color(${ink})`);
    runset("queued_index",`style().font_size(px(11.5)).text_color(${faint})`);
    runset("write",`style().padding_x(px(${clean ? 8 : 10})).padding_top(px(${clean ? 10 : 12})).padding_bottom(px(${clean ? 8 : 10})).radius(px(${clean ? 8 : 16})).background(${panel}).focus_within(style().focus_ring(px(1),offset_px(-1),${focus}))${shadows(clean ? smallShadow : checklistShadow)}`);
    runset("editor",`style().padding_x(px(4)).padding_y(px(0)).border(px(0)).background(${transparent})${shadows([])}.focus(style()${shadows([])})`);
    runset("permission_trigger",`style().min_height(px(28)).padding_x(px(6)).radius(px(${clean ? 6 : 7})).text_color(${muted})`);
    runset("permission_panel",`style().padding(px(${clean ? 4 : 5})).radius(px(${clean ? 10 : 16})).background(${panel})${shadows(floatingShadow)}`);
    runset("permission_selected",`style().background(${wash})`);
    runset("permission_alert",`style().text_color(${color("text.danger")})`);
    runset("permission_hint",`style().font_size(px(12)).line_height(px(16)).text_color(${muted})`);
    for (const part of ["send","stop","resume","retry","dismiss"]) {
      runset(part,`style().min_height(px(${clean ? 28 : 30})).radius(px(${clean ? 7 : 999}))`);
    }
    const library=(part,style)=>set("patterns/prompt_library",part,style);
    const picker=(part,style)=>set("patterns/integration_picker",part,style);
    const pickerShell=`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 10 : 20})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`;
    library("root",pickerShell);
    picker("root",pickerShell);
    for (const apply of [library,picker]) {
      apply("card",`style().radius(px(${clean ? 10 : 16})).background(${panel})${shadows(clean ? [] : checklistShadow)}`);
      apply("title",`style().font_size(px(${clean ? 13 : 14})).font_weight(600).line_height(px(20))`);
      apply("total",`style().font_size(px(12)).text_color(${faint}).font_feature("tnum",1)`);
      apply("search_input",`style().padding(px(0)).border(px(0)).background(${transparent})${shadows([])}`);
      apply("search_icon",`style().text_color(${muted})`);
      apply("row",`style().min_height(px(${clean ? 32 : 34})).gap(px(6)).padding_x(px(7)).radius(px(${clean ? 6 : 9})).font_size(px(${clean ? 13 : 14}))`);
      apply("selected",`style().background(${selected}).hover(style().background(${selected}))`);
      apply("detail",`style().padding_x(px(${clean ? 16 : 14})).padding_y(px(12)).gap(px(${clean ? 8 : 10}))`);
      apply("detail_title",`style().font_size(px(${clean ? 15 : 17})).line_height(px(24)).font_weight(600)`);
    }
    library("sidebar",`style().padding_x(px(8)).padding_top(px(12)).padding_bottom(px(8)).gap(px(${clean ? 6 : 4}))`);
    library("search",`style().min_height(px(${clean ? 28 : 30})).padding_x(px(7)).radius(px(${clean ? 6 : 8})).background(${clean ? transparent : wash})${shadows(clean ? [shadow(0,0,0,.5,edge,true)] : [])}`);
    library("body",`style().font_size(px(13)).line_height(px(${clean ? 28 : 26})).text_color(${clean ? ink : color("card.body")})`);
    library("variable",`style().background(${wash}).text_color(${color("text.accent")}).font_weight(500)`);
    library("variable_active",`style().background(${selected}).text_color(${color("text.accent")}).font_weight(600)`);
    library("field_label",`style().font_size(px(12)).text_color(${muted})`);
    library("field_editor",`style().font_size(px(13)).line_height(px(20)).padding_x(px(${clean ? 7 : 8})).padding_y(px(${clean ? 3 : 4})).radius(px(${clean ? 6 : 8}))`);
    library("foot",`style().min_height(px(${clean ? 44 : 40})).padding_x(px(${clean ? 14 : 9})).border_top(px(${clean ? .5 : 0})).border_color(${edge})`);
    library("caption",`style().font_size(px(12)).text_color(${muted})`);
    library("copy",`style().min_height(px(${clean ? 28 : 32})).radius(px(${clean ? 6 : 8}))`);
    library("use",`style().min_height(px(${clean ? 28 : 32})).radius(px(${clean ? 6 : 8}))`);
    picker("tray",`style().min_height(px(${clean ? 34 : 40})).padding_x(px(10)).gap(px(6))`);
    picker("search",`style().min_height(px(${clean ? 40 : 46})).padding_x(px(${clean ? 12 : 14})).border_bottom(px(${clean ? .5 : 1})).border_color(${edge})`);
    picker("list",`style().padding_x(px(${clean ? 4 : 8})).padding_y(px(5))`);
    picker("group",`style().font_size(px(11)).padding_x(px(6)).text_color(${faint}).font_weight(500)`);
    picker("row",`style().min_height(px(${clean ? 40 : 42})).gap(px(${clean ? 6 : 8})).padding_x(px(6)).radius(px(${clean ? 6 : 9})).font_size(px(${clean ? 13 : 14}))`);
    picker("tile",`style().width(px(${clean ? 24 : 26})).height(px(${clean ? 24 : 26})).radius(px(${clean ? 5 : 7}))`);
    picker("large_tile",`style().width(px(${clean ? 46 : 52})).height(px(${clean ? 46 : 52})).radius(px(${clean ? 9 : 12}))${shadows([shortDrop(black(.12))])}`);
    picker("art",`style().min_height(px(${clean ? 96 : 110})).radius(px(${clean ? 8 : 12})).background(${clean ? transparent : wash}).clip()`);
    picker("description",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20))`);
    picker("explore",`style().self_start().min_height(px(${clean ? 28 : 32}))`);
    const iset=(part,value)=>set("patterns/integration_settings",part,value);
    iset("root",`style().padding(px(${clean ? 0 : 6})).gap(px(0)).radius(px(${clean ? 10 : 22})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    iset("header",`style().gap(px(${clean ? 12 : 14})).padding_x(px(16)).padding_y(px(${clean ? 16 : 14}))`);
    iset("logo",`style().width(px(40)).height(px(40)).radius(px(${clean ? 8 : 11})).background(${panel})${shadows(smallShadow)}`);
    iset("name",`style().font_size(px(${clean ? 14 : 15})).line_height(px(22)).font_weight(500)`);
    iset("count",`style().font_size(px(${clean ? 11 : 11.5})).line_height(px(${clean ? 16 : 18})).font_weight(500).padding_x(px(${clean ? 5 : 6})).radius(px(${clean ? 5 : 6})).text_color(${ink}).background(${clean ? wash : panel}).font_feature("tnum",1)${shadows(clean ? [] : [hairline])}`);
    iset("meta",`style().font_size(px(12)).line_height(px(18)).text_color(${muted})`);
    iset("body",`style().padding(px(0)).gap(px(0)).radius(px(${clean ? 0 : 16})).background(${clean ? transparent : panel})${shadows(clean ? [] : checklistShadow)}`);
    for (const part of ["credential","section"]) {
      iset(part,`style().padding_x(px(${clean ? 16 : 18})).padding_y(px(${clean ? 12 : 16})).gap(px(${clean ? 6 : 8}))`);
    }
    iset("section",`style().padding_x(px(${clean ? 16 : 18})).padding_y(px(${clean ? 12 : 16})).gap(px(${clean ? 6 : 8})).border_top(px(${clean ? .5 : 1})).border_color(${edge})`);
    iset("label",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(500)`);
    iset("credential_label",`style().${clean ? "block()" : "hidden()"}.font_size(px(13)).line_height(px(20)).font_weight(500)`);
    iset("key_frame",`style().min_height(px(${clean ? 36 : 42})).padding_x(px(${clean ? 10 : 14})).radius(px(${clean ? 8 : 11})).background(${clean ? panel : wash})${shadows(clean ? smallShadow : [])}`);
    iset("generate_full",`style().min_height(px(${clean ? 36 : 42})).padding_x(px(${clean ? 10 : 14})).radius(px(${clean ? 8 : 11})).background(${wash}).text_color(${muted}).border(px(0))${shadows(clean ? [hairline] : [])}.hover(style().background(${selected}).text_color(${ink}))`);
    iset("site",`style().min_height(px(${clean ? 32 : 42})).padding_x(px(${clean ? 10 : 14})).radius(px(${clean ? 8 : 11})).border(px(0)).background(${field})${shadows(fieldShadow)}.focus_within(style() ${shadows(focusShadow)})`);
    iset("site_invalid",`style().background(${color("number.error_fill")})${shadows([shadow(0,0,0,1,color("number.error_edge"),true)])}.focus_within(style()${shadows([shadow(0,0,0,1,color("number.error_focus"),true)])})`);
    iset("footer",`style().padding_x(px(${clean ? 16 : 12})).padding_y(px(${clean ? 10 : 8})).border_top(px(${clean ? .5 : 0})).border_color(${edge})`);
    for (const part of ["discard","resolve","save"]) {
      iset(part,`style().min_height(px(${clean ? 28 : 32})).radius(px(${clean ? 6 : 10})).font_size(px(13))`);
    }
    for (const module of ["comment_thread","comments_panel"]) {
      const item=(part,value)=>set(`patterns/${module}`,part,value);
      const feed=module==="comments_panel";
      const diameter=clean ? 20 : feed ? 22 : 24;
      const authorGap=clean ? (feed ? 7 : 8) : 9;
      const authorInset=clean ? (feed ? 28 : diameter + authorGap) : 0;
      const authorLength=value=>feed ? px(value) : readingLength(value);
      item("root",`style().padding(px(${clean ? 0 : 6})).radius(px(${clean ? 10 : 22})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
      item("header",`style().min_height(px(${clean && !feed ? 48 : 46})).padding_start(px(${clean ? 16 : 18})).padding_end(px(10)).padding_y(px(${clean && feed ? 4 : 6})).border_bottom(px(${clean && !feed ? .5 : 0})).border_color(${edge})`);
      item("title",`style().font_size(px(${clean && !feed ? 13 : 15})).line_height(px(${clean && !feed ? 20 : 22})).font_weight(500)`);
      item("close",`style().width(px(28)).height(px(28)).min_height(px(28)).radius(px(${clean ? 6 : 8})).text_color(${muted})`);
      item("card",`style().radius(px(${clean ? 0 : 16})).background(${clean ? transparent : panel})${shadows(clean ? [] : checklistShadow)}`);
      item("entry",`style().padding_start(px(${clean || feed ? 16 : 18})).padding_end(px(${clean || feed ? 16 : 18})).padding_top(px(${clean ? 12 : feed ? 14 : 14})).padding_bottom(px(${clean ? 12 : 16})).gap(px(${clean ? 4 : 7}))${feed ? `.border_top(px(${clean ? .5 : 1})).border_color(${edge})` : ""}`);
      item("meta",`style().gap(${authorLength(authorGap)})`);
      item("avatar",`style().width(${authorLength(diameter)}).height(${authorLength(diameter)}).background(${color("badge.info_bg")}).text_color(${color("badge.info_fg")})`);
      item("assistant",`style().background(${wash}).text_color(${ink})`);
      item("author",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(500)`);
      item("time",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(18)).text_color(${muted}).font_feature("tnum",1)`);
      item("body",`style().font_size(px(${clean ? 13 : 14})).line_height(px(${clean ? 20 : 21})).padding_start(${authorLength(authorInset)}).text_color(${clean ? ink : color("card.body")})`);
      item("composer",`style().padding_start(px(${clean || feed ? 16 : 18})).padding_end(px(${clean ? 8 : 9})).padding_y(px(${clean ? 8 : 9})).gap(px(${clean ? 8 : 10})).radius(px(${clean ? 8 : 12}))`);
      item("editor",`style().min_height(px(${clean ? 28 : 32})).padding(px(0)).border(px(0)).background(${transparent})${shadows([])}`);
      item("send",`style().width(px(${clean ? 28 : 30})).height(px(${clean ? 28 : 30})).min_height(px(${clean ? 28 : 30})).radius(px(${clean ? 6 : 999}))${clean ? `.disabled(style().background(${wash}).text_color(${faint}).opacity(1.0)${shadows([])})` : ".disabled(style().opacity(0.3))"}`);
      if (feed) {
        item("bar",`style().padding_start(px(${clean ? 16 : 14})).padding_end(px(12)).padding_y(px(${clean ? 10 : 12})).border_bottom(px(${clean ? .5 : 1})).border_color(${edge})`);
        item("first",`style().border_top(px(0))`);
        item("resolve",`style().radius(px(${clean ? 6 : 8})).text_color(${color("text.success")}).hover(style().background(${wash}))`);
        item("run",`style().min_height(px(${clean ? 28 : 30})).radius(px(${clean ? 6 : 9})).text_color(${color("text.success")})`);
        item("edit_applied",`style().min_height(px(26)).padding_x(px(7)).radius(px(${clean ? 6 : 8})).background(${color("badge.success_bg")}).text_color(${color("text.success")})`);
      } else {
        item("count",`style().font_size(px(${clean ? 12 : 12.5})).text_color(${muted})`);
        item("status",`style().min_height(px(${clean ? 24 : 28})).radius(px(${clean ? 6 : 8})).margin_start(${authorLength(authorInset)}).background(${color("badge.success_bg")}).text_color(${color("text.success")})`);
        item("details",`style().margin_start(${authorLength(authorInset)}).padding_start(px(10)).gap(px(4)).border_color(${edge})`);
        item("bullet",`style().text_color(${muted})`);
      }
    }
    const wr=(part,value)=>set("patterns/writing_assistant",part,value);
    wr("root",`style().padding(px(${clean ? 0 : 8})).radius(px(${clean ? 10 : 32})).border(px(${clean ? 0 : 1})).border_color(${edge}).background(${panel})${shadows(clean ? smallShadow : floatingCast(black(dark ? .18 : .06)))}`);
    wr("header",`style().padding_x(px(${clean ? 16 : 18})).padding_y(px(${clean ? 13 : 16})).border_bottom(px(${clean ? .5 : 0})).border_color(${edge})`);
    wr("name",`style().font_size(px(${clean ? 15 : 20})).font_weight(500).line_height(px(${clean ? 21 : 24})).letter_spacing_em(${clean ? 0 : -.02})`);
    wr("tagline",`style().font_size(px(${clean ? 12 : 13})).line_height(px(18)).text_color(${muted})`);
    wr("deck",`style().radius(px(${clean ? 10 : 24})).background(${color("writing.deck")})${shadows(clean ? [] : [shadow(0,0,0,1,edge,true)])}`);
    wr("body",`style().padding(px(${clean ? 12 : 14})).gap(px(${clean ? 12 : 18}))`);
    wr("welcome",`style().padding_x(px(4)).gap(px(${clean ? 12 : 14}))`);
    wr("greeting",`style().font_size(px(${clean ? 15 : 24})).line_height(px(${clean ? 24 : 28})).font_weight(500).letter_spacing_em(${clean ? 0 : -.025})`);
    wr("suggestion",`style().min_height(px(${clean ? 28 : 32})).padding_x(px(${clean ? 10 : 13})).radius(px(${clean ? 6 : 999})).font_size(px(13)).font_weight(400).border(px(0)).background(${clean ? `alpha(${panel},0.55)` : color("writing.user")}).text_color(${ink}).backdrop_blur(${clean ? 6 : 8}).hover(style().background(${selected}))${shadows(clean ? [hairline] : [])}`);
    for (const glow of ["glow_pink","glow_violet","glow_blue"]) {
      wr(glow,`style().opacity(${clean ? .13 : .28})`);
    }
    wr("glow_chat",`style().opacity(${clean ? .052 : .168})`);
    wr("user",`style().max_width(relative(0.86)).padding_x(px(${clean ? 10 : 13})).padding_y(px(${clean ? 6 : 9})).font_size(px(${clean ? 13 : 13.5})).line_height(px(20)).background(${color("writing.user")})${clean ? ".radius(px(10))" : ""}`);
    wr("assistant",`style().max_width(relative(${clean ? 0.94 : 0.86})).padding_x(px(${clean ? 0 : 13})).padding_y(px(${clean ? 4 : 9})).font_size(px(${clean ? 13 : 13.5})).line_height(px(20)).background(${color("writing.answer")}).backdrop_blur(${clean ? 0 : 10})${shadows(clean ? [] : [hairline])}`);
    wr("field",`style().padding_x(px(${clean ? 6 : 12})).padding_y(px(${clean ? 5 : 8})).min_height(px(${clean ? 34 : 54})).radius(px(${clean ? 6 : 26})).background(${clean ? transparent : field})${shadows(clean ? [] : fieldShadow)}`);
    wr("mic",`style().width(px(${clean ? 28 : 54})).height(px(${clean ? 28 : 54})).min_height(px(${clean ? 28 : 54})).radius(px(${clean ? 6 : 999})).border(px(0)).background(${clean ? transparent : field}).text_color(${clean ? muted : ink})${shadows(clean ? [] : fieldShadow)}.hover(style().background(${wash}))`);
    wr("mic_active",`style().background(${color("writing.listening")}).text_color(${color("writing.listening_ink")}).hover(style().background(${color("writing.listening")}))`);
    wr("bar",`style().background(${color("writing.listening_ink")})`);
    wr("send",`style().width(px(${clean ? 27 : 35})).height(px(${clean ? 27 : 35})).min_height(px(${clean ? 27 : 35})).radius(px(${clean ? 6 : 999}))`);
    const dq=(part,value)=>set("patterns/decision_card",part,value);
    dq("root",`style().padding(px(${clean ? 6 : 4})).gap(px(2)).radius(px(${clean ? 14 : 18})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    dq("head",`style().min_height(px(${clean ? 28 : 32})).padding_start(px(${clean ? 6 : 8})).padding_end(px(4)).gap(px(6))`);
    dq("badge",`style().min_height(px(18)).padding_x(px(6)).radius(px(${clean ? 4 : 5})).font_size(px(11)).line_height(px(18)).font_weight(500).background(${color("badge.warning_bg")}).text_color(${color("badge.warning_fg")})`);
    dq("task",`style().font_size(px(12)).text_color(${muted})`);
    dq("count",`style().font_size(px(12)).text_color(${muted}).font_feature("tnum",1)`);
    dq("page",`style().width(px(23)).height(px(23)).min_height(px(23)).radius(px(${clean ? 5 : 6})).text_color(${muted})`);
    dq("later",`style().min_height(px(25)).radius(px(${clean ? 5 : 6}))`);
    dq("show",`style().min_height(px(25)).radius(px(${clean ? 5 : 6}))`);
    dq("card",`style().padding(px(${clean ? 2 : 6})).gap(px(3)).radius(px(${clean ? 8 : 14})).background(${clean ? transparent : panel})${shadows(clean ? [] : smallShadow)}`);
    dq("question",`style().font_size(px(14)).line_height(px(21)).padding_x(px(6))`);
    dq("option",`style().min_height(px(${clean ? 34 : 36})).padding_x(px(6)).radius(px(${clean ? 7 : 8}))`);
    dq("tile",`style().padding(px(4)).radius(px(${clean ? 8 : 10}))`);
    dq("preview",`style().radius(px(${clean ? 6 : 7})).background(${wash})${shadows([shadow(0,0,0,clean ? .5 : 1,edge,true)])}`);
    dq("option_title",`style().font_size(px(13)).line_height(px(20)).font_weight(500)`);
    dq("number",`style().radius(px(${clean ? 5 : 6})).font_size(px(12)).background(${wash}).text_color(${muted})`);
    dq("summary",`style().font_size(px(12)).text_color(${muted})`);
    dq("recommended",`style().font_size(px(11)).line_height(px(18)).padding_x(px(6)).radius(px(5)).background(${wash}).text_color(${muted})`);
    dq("own",`style().min_height(px(36)).padding_x(px(6)).gap(px(6)).radius(px(${clean ? 7 : 8})).hover(style().background(${wash})).focus_within(style().background(${wash}))`);
    dq("editor",`style().padding(px(0)).border(px(0)).background(${transparent})${shadows([])}`);
    dq("send",`style().width(px(24)).height(px(24)).min_height(px(24)).radius(px(${clean ? 6 : 999}))`);
    const tree=(part,value)=>set("patterns/decision_tree",part,value);
    tree("root",`style().gap(px(${clean ? 12 : 16}))`);
    tree("title",`style().font_size(px(${clean ? 14 : 15})).font_weight(600)`);
    tree("pager",`style().gap(px(${clean ? 6 : 8}))`);
    tree("question",`style().padding_x(px(${clean ? 16 : 17})).padding_y(px(14)).gap(px(9)).radius(px(${clean ? 8 : 14})).background(${panel})${shadows(clean ? smallShadow : floatingShadow)}`);
    tree("avatar",`style().radius(px(${clean ? 6 : 8})).background(${wash})`);
    tree("agent",`style().font_size(px(12)).line_height(px(17)).font_weight(500)`);
    tree("text",`style().font_size(px(${clean ? 15 : 16})).line_height(px(22)).font_weight(600)`);
    tree("context",`style().font_size(px(12)).line_height(px(17)).text_color(${muted})`);
    tree("branches",`style().gap(px(${clean ? 14 : 13}))`);
    tree("choice",`style().padding_x(px(12)).padding_y(px(${clean ? 9 : 10})).radius(px(${clean ? 8 : 13})).background(${panel})${shadows(clean ? smallShadow : floatingShadow)}`);
    tree("active",`style().background(${color("selection")})${shadows([shadow(0,0,0,clean ? .5 : 1,color("accent"),true)])}`);
    tree("muted",`style().background(${wash})`);
    tree("label",`style().font_size(px(13)).line_height(px(18)).font_weight(500)`);
    tree("number",`style().font_size(px(11)).radius(px(${clean ? 4 : 5})).background(${wash}).text_color(${muted})`);
    tree("leaf",`style().padding_x(px(12)).padding_y(px(8)).radius(px(${clean ? 8 : 12})).background(${wash})${shadows([shadow(0,0,0,clean ? .5 : 1,edge,true)])}`);
    tree("effect",`style().font_size(px(12)).line_height(px(17)).text_color(${muted})`);
    tree("flag",`style().radius(px(3)).background(${wash}).border_bottom(px(0.5)).border_color(${color("text.warning")})`);
    tree("recommended",`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    tree("receipt",`style().padding_x(px(10)).padding_y(px(6)).radius(px(${clean ? 6 : 8})).text_color(${color("text.success")}).background(${wash})`);
    const rv=(part,value)=>set("patterns/review_center",part,value);
    rv("root",`style().padding(px(${clean ? 0 : 5})).gap(px(${clean ? 0 : 5})).radius(px(${clean ? 12 : 18})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    rv("head",`style().min_height(px(${clean ? 47 : 44})).padding_x(px(${clean ? 16 : 12})).padding_y(px(6)).border_bottom(px(${clean ? .5 : 0})).border_color(${edge})`);
    rv("title",`style().font_size(px(${clean ? 14 : 15})).font_weight(${clean ? 500 : 600})`);
    rv("count",`style().font_size(px(12)).line_height(px(17)).font_feature("tnum",1).text_color(${muted})`);
    rv("oldest",`style().font_size(px(12)).line_height(px(17)).font_feature("tnum",1)`);
    rv("card",`style().radius(px(${clean ? 10 : 13})).background(${panel})${shadows(clean ? [] : smallShadow)}`);
    rv("queue",`style().background(${color("cite.tray")})`);
    rv("filters",`style().padding(px(8)).gap(px(4)).background(${panel}).border_bottom(px(${clean ? .5 : 1})).border_color(${edge})`);
    rv("filter_active",`style().background(${wash}).radius(px(${clean ? 5 : 7}))`);
    rv("list",`style().padding(px(8)).gap(px(4))`);
    rv("row",`style().min_height(px(${clean ? 46 : 51})).padding_x(px(9)).gap(px(9)).radius(px(${clean ? 7 : 10})).background(${transparent})`);
    rv("row_active",`style().background(${panel})${shadows(smallShadow)}`);
    rv("row_title",`style().font_size(px(13)).line_height(px(18)).font_weight(500)`);
    rv("meta",`style().font_size(px(${clean ? 12 : 11.5})).line_height(px(17)).text_color(${muted})`);
    rv("avatar",`style().width(px(${clean ? 24 : 28})).height(px(${clean ? 24 : 28})).radius(px(${clean ? 6 : 8})).background(${wash})`);
    rv("detail",`style().padding_x(px(${clean ? 22 : 24})).padding_y(px(${clean ? 18 : 20})).gap(px(${clean ? 12 : 15}))`);
    rv("item_title",`style().font_size(px(${clean ? 17 : 18})).line_height(px(25)).font_weight(600)`);
    rv("section_title",`style().font_size(px(12)).line_height(px(17)).text_color(${muted})`);
    rv("why",`style().typography("body").font_size(px(13)).line_height(px(20)).text_color(${ink})`);
    rv("effect",`style().typography("body").font_size(px(13)).line_height(px(20)).text_color(${ink})`);
    rv("field_label",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    rv("flag",`style().radius(px(3)).background(${transparent}).border_bottom(px(0.5)).border_color(${color("text.warning")})`);
    rv("diff",`style().radius(px(${clean ? 6 : 9})).clip()`);
    rv("tool",`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    rv("actions",`style().padding(px(${clean ? 9 : 12})).gap(px(7)).border_top(px(${clean ? .5 : 1})).border_color(${edge})`);
    rv("receipt",`style().padding_x(px(10)).padding_y(px(6)).radius(px(${clean ? 6 : 10})).background(${panel})${shadows(smallShadow)}`);
    const db=(part,value)=>set("patterns/delegate_board",part,value);
    db("root",`style().gap(px(${clean ? 0 : 6})).padding(px(${clean ? 0 : 6})).radius(px(${clean ? 12 : 19})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    db("head",`style().min_height(px(${clean ? 44 : 41})).padding_x(px(${clean ? 13 : 9})).padding_y(px(6)).border_bottom(px(${clean ? .5 : 0})).border_color(${edge})`);
    db("title",`style().font_size(px(${clean ? 14 : 15})).font_weight(${clean ? 500 : 600})`);
    db("hint",`style().font_size(px(12)).line_height(px(17)).text_color(${muted})`);
    db("symbol",`style().width(px(24)).height(px(24)).radius(px(${clean ? 6 : 8})).background(${wash}).text_color(${ink})`);
    db("lanes",`style().gap(px(${clean ? 0 : 6}))`);
    db("pile",`style().padding(px(10)).radius(px(${clean ? 0 : 12})).background(${clean ? panel : wash})`);
    db("agents",`style().padding(px(10)).gap(px(8)).radius(px(${clean ? 0 : 12})).background(${panel})`);
    db("section_title",`style().font_size(px(13)).line_height(px(18)).font_weight(500)`);
    db("count",`style().padding_x(px(5)).radius(px(${clean ? 4 : 5})).typography("caption").text_color(${muted})`);
    db("task",`style().min_height(px(${clean ? 37 : 41})).radius(px(${clean ? 7 : 10})).padding_x(px(10)).background(${panel})${shadows(smallShadow)}`);
    db("queue_task",`style().min_height(px(${clean ? 26 : 28})).radius(px(${clean ? 5 : 6})).padding_x(px(7)).background(${wash})${shadows([])}`);
    db("task_picked",`style().background(${selected})${shadows([shadow(0,0,0,1,focus,true)])}`);
    db("task_suggested",`style().border(px(${clean ? .5 : 1})).border_color(alpha(${color("accent")},0.45))`);
    db("task_title",`style().font_size(px(13)).line_height(px(18)).font_weight(500)`);
    db("tool",`style().width(px(${clean ? 18 : 21})).height(px(${clean ? 18 : 21})).radius(px(${clean ? 4 : 5})).font_size(px(10)).background(${wash})`);
    db("time",`style().font_size(px(11.5)).line_height(px(17)).font_feature("tnum",1).text_color(${muted})`);
    db("agent",`style().padding(px(10)).gap(px(7)).radius(px(${clean ? 8 : 12})).background(${panel})${shadows(clean ? smallShadow : [])}`);
    db("agent_suggested",`style().background(${wash})${shadows([shadow(0,0,0,1,focus,true)])}`);
    db("avatar",`style().width(px(${clean ? 26 : 30})).height(px(${clean ? 26 : 30})).radius(px(${clean ? 6 : 8})).background(${wash}).font_size(px(12))`);
    db("agent_name",`style().font_size(px(13)).line_height(px(18)).font_weight(500)`);
    db("model",`style().font_size(px(12)).line_height(px(16)).text_color(${muted})`);
    db("load",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    db("free",`style().font_size(px(11.5)).line_height(px(16)).text_color(${color("text.success")})`);
    db("running",`style().padding_x(px(5)).radius(px(4)).font_size(px(10.5)).text_color(${color("text.success")})`);
    db("estimate",`style().font_size(px(12)).line_height(px(17)).text_color(${color("accent")})`);
    db("footer",`style().min_height(px(${clean ? 42 : 40})).padding_x(px(${clean ? 12 : 9})).padding_y(px(5)).border_top(px(${clean ? .5 : 0})).border_color(${edge})`);
    db("queued",`style().font_size(px(12)).line_height(px(17)).font_feature("tnum",1).text_color(${muted})`);
    db("cost",`style().font_size(px(12)).line_height(px(17)).font_feature("tnum",1).text_color(${muted})`);
    db("notice",`style().padding_x(px(9)).padding_y(px(4)).font_size(px(12)).line_height(px(17)).text_color(${muted})`);
    const aq=(part,value)=>set("patterns/agent_queue",part,value);
    aq("depth_fill",`style().linear_gradient(linear_gradient(#{angle:180,from:alpha(${color("accent")},0.18),to:alpha(${color("accent")},0.02)}))`);
    aq("root",`style().padding(px(${clean ? 0 : 5})).gap(px(${clean ? 0 : 5})).radius(px(${clean ? 12 : 18})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    aq("head",`style().min_height(px(${clean ? 45 : 42})).padding_x(px(16)).padding_y(px(6)).border_bottom(px(${clean ? .5 : 0})).border_color(${edge})`);
    aq("title",`style().font_size(px(${clean ? 14 : 15})).font_weight(${clean ? 500 : 600})`);
    aq("metadata",`style().font_size(px(12)).line_height(px(17)).text_color(${muted})`);
    aq("state",`style().padding_x(px(6)).padding_y(px(2)).radius(px(5)).font_size(px(11.5)).background(${wash})`);
    aq("card",`style().radius(px(${clean ? 10 : 13})).gap(px(${clean ? 3 : 6})).background(${panel})${shadows(clean ? [] : smallShadow)}`);
    aq("stats",`style().padding_x(px(12)).padding_y(px(8)).gap(px(7)).border_bottom(px(${clean ? .5 : 1})).border_color(${edge})`);
    aq("stat",`style().min_height(px(${clean ? 42 : 47})).padding(px(4)).gap(px(2))`);
    aq("stat_value",`style().font_size(px(${clean ? 19 : 21})).line_height(px(27)).font_weight(600).font_feature("tnum",1)`);
    aq("stat_label",`style().font_size(px(12)).line_height(px(16)).text_color(${muted})`);
    aq("flow",`style().padding_x(px(8)).padding_y(px(${clean ? 4 : 5})).gap(px(${clean ? 6 : 9}))`);
    aq("waiting",`style().padding(px(8)).radius(px(${clean ? 6 : 10})).background(${clean ? transparent : wash})`);
    aq("worker_group",`style().padding(px(8)).gap(px(6))`);
    aq("worker",`style().padding(px(7)).gap(px(4)).radius(px(${clean ? 7 : 10})).background(${wash})`);
    aq("worker_name",`style().font_size(px(12)).line_height(px(17)).text_color(${ink}).font_weight(500)`);
    aq("worker_detail",`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    aq("job",`style().padding_x(px(7)).radius(px(${clean ? 6 : 8})).background(${panel})${shadows(clean ? [hairline] : smallShadow)}`);
    aq("job_open",`style().background(${selected})${shadows([shadow(0,0,0,1,focus,true)])}`);
    aq("job_name",`style().font_size(px(12.5)).line_height(px(18)).font_weight(500)`);
    aq("job_side",`style().font_size(px(11)).line_height(px(16)).font_feature("tnum",1)`);
    aq("section",`style().font_size(px(13)).line_height(px(18)).font_weight(500)`);
    aq("section_count",`style().font_size(px(11.5)).line_height(px(17)).text_color(${muted})`);
    aq("bins",`style().padding_x(px(9)).padding_y(px(4)).gap(px(6))`);
    aq("bin",`style().padding(px(7)).radius(px(${clean ? 7 : 9})).background(${wash})`);
    aq("bin_failed",`style().background(alpha(${color("text.danger")},0.055))`);
    aq("detail",`style().padding(px(10)).radius(px(${clean ? 6 : 9})).background(${panel})`);
    aq("chart",`style().padding_x(px(16)).padding_y(px(${clean ? 9 : 11})).gap(px(5)).border_top(px(${clean ? .5 : 1})).border_color(${edge})`);
    aq("chart_title",`style().font_size(px(12)).line_height(px(17)).font_weight(500)`);
    aq("plot",`style().height(px(${clean ? 99 : 110}))`);
    for(const part of ["axis","scale"]){aq(part,`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);}
    aq("notice",`style().padding_x(px(9)).padding_y(px(4)).font_size(px(12)).line_height(px(17))`);
    const al=(part,value)=>set("patterns/agent_lanes",part,value);
    al("root",`style().padding(px(${clean ? 0 : 5})).gap(px(${clean ? 0 : 5})).radius(px(${clean ? 12 : 18})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    al("head",`style().min_height(px(${clean ? 45 : 42})).padding_x(px(14)).padding_y(px(6)).border_bottom(px(${clean ? .5 : 0})).border_color(${edge})`);
    al("title",`style().font_size(px(${clean ? 14 : 15})).font_weight(${clean ? 500 : 600})`);
    al("summary",`style().font_size(px(12)).line_height(px(17)).text_color(${muted})`);
    al("card",`style().padding(px(${clean ? 8 : 9})).gap(px(8)).radius(px(${clean ? 0 : 13})).background(${panel})${shadows(clean ? [] : smallShadow)}`);
    al("axis",`style().height(px(24))`);
    al("tick",`style().font_size(px(11)).line_height(px(16)).text_color(${muted}).font_feature("tnum",1)`);
    al("lane",`style().border_bottom(px(${clean ? .5 : 1})).border_color(${edge})`);
    al("agent_name",`style().font_size(px(13)).line_height(px(18)).font_weight(500)`);
    al("model",`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    al("avatar",`style().width(px(${clean ? 24 : 26})).height(px(${clean ? 24 : 26})).radius(px(${clean ? 7 : 13})).background(${wash}).text_color(${ink})`);
    al("task",`style().radius(px(${clean ? 6 : 8})).padding_x(px(${clean ? 6 : 7}))`);
    al("task_title",`style().font_size(px(12)).line_height(px(17)).font_weight(500)`);
    al("task_done",`style().background(${wash}).text_color(${ink})`);
    al("task_running",`style().background(alpha(${color("accent")},0.10)).text_color(${color("accent")})`);
    al("task_waiting",`style().background(alpha(${color("text.warning")},0.10))`);
    al("task_failed",`style().background(alpha(${color("text.danger")},0.10))`);
    al("task_open",`style().background(${selected})${shadows([shadow(0,0,0,1,focus,true)])}`);
    al("inspector",`style().padding(px(${clean ? 10 : 12})).radius(px(${clean ? 7 : 10})).background(${panel})${shadows(smallShadow)}`);
    al("notice",`style().padding_x(px(10)).padding_y(px(4)).font_size(px(12)).line_height(px(17))`);
    const ah=(part,value)=>set("patterns/agent_handoff",part,value);
    ah("root",`style().padding(px(${clean ? 0 : 5})).gap(px(${clean ? 5 : 8})).radius(px(${clean ? 12 : 18})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    ah("head",`style().min_height(px(${clean ? 43 : 41})).padding_x(px(14)).padding_y(px(6)).border_bottom(px(${clean ? .5 : 0})).border_color(${edge})`);
    ah("title",`style().font_size(px(${clean ? 14 : 15})).font_weight(${clean ? 500 : 600})`);
    ah("summary",`style().font_size(px(12)).line_height(px(17)).text_color(${muted})`);
    ah("agent",`style().padding(px(${clean ? 10 : 11})).gap(px(7)).radius(px(${clean ? 8 : 12})).background(${panel})${shadows(clean ? [hairline] : smallShadow)}`);
    ah("agent_target",`style().background(${selected})${shadows([shadow(0,0,0,1,focus,true)])}`);
    ah("avatar",`style().width(px(25)).height(px(25)).radius(px(${clean ? 6 : 8})).background(${wash}).text_color(${ink})`);
    ah("name",`style().font_size(px(13)).line_height(px(18)).font_weight(600)`);
    ah("model",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    ah("status",`style().font_size(px(11)).line_height(px(16)).font_feature("tnum",1)`);
    ah("agent_desc",`style().font_size(px(12)).line_height(px(18)).text_color(${muted})`);
    ah("current",`style().padding_x(px(7)).padding_y(px(3)).radius(px(${clean ? 5 : 7})).background(${wash}).font_size(px(11.5)).line_height(px(17)).text_color(${ink})`);
    ah("pair",`style().radius(px(${clean ? 8 : 11})).padding(px(7)).gap(px(6)).background(${wash})`);
    ah("pair_name",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted}).font_weight(500)`);
    ah("pair_count",`style().padding_x(px(3)).font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    ah("packet",`style().padding(px(7)).radius(px(${clean ? 5 : 8})).background(${panel})`);
    ah("latest",`style().border(px(${clean ? .5 : 1})).border_color(${edge})`);
    ah("packet_title",`style().font_size(px(13)).line_height(px(18)).font_weight(500)`);
    ah("packet_meta",`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    ah("task_list",`style().padding(px(8)).radius(px(${clean ? 7 : 10})).background(${wash})`);
    ah("task",`style().radius(px(${clean ? 6 : 8})).background(${panel})`);
    ah("task_title",`style().font_size(px(13)).line_height(px(18))`);
    ah("inspector",`style().padding(px(${clean ? 10 : 12})).radius(px(${clean ? 7 : 10})).background(${panel})${shadows(smallShadow)}`);
    ah("inspector_title",`style().font_size(px(14)).line_height(px(20)).font_weight(600)`);
    ah("item",`style().padding(px(7)).radius(px(${clean ? 5 : 7})).background(${wash})`);
    ah("item_lost",`style().background(alpha(${color("text.danger")},0.055))`);
    ah("item_kind",`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    ah("notice",`style().padding_x(px(9)).padding_y(px(4)).font_size(px(12)).line_height(px(17))`);
    const ai=(part,value)=>set("patterns/agent_inbox",part,value);
    ai("root",`style().padding(px(${clean ? 0 : 5})).gap(px(12)).radius(px(${clean ? 12 : 18})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    ai("head",`style().min_height(px(${clean ? 45 : 42})).padding_x(px(${clean ? 20 : 17})).padding_y(px(6)).border_bottom(px(${clean ? .5 : 0})).border_color(${edge})`);
    ai("title",`style().font_size(px(${clean ? 14 : 15})).font_weight(${clean ? 500 : 600})`);
    ai("crumb",`style().font_size(px(12)).line_height(px(17)).text_color(${muted})`);
    ai("switcher",`style().padding(px(2)).gap(px(2)).radius(px(${clean ? 6 : 8})).background(${wash})`);
    ai("option",`style().min_height(px(25)).padding_x(px(9)).radius(px(${clean ? 5 : 7})).font_size(px(12)).line_height(px(17)).text_color(${muted})`);
    ai("option_active",`style().background(${panel}).text_color(${ink})${shadows(smallShadow)}`);
    ai("lanes",`style().padding_x(px(${clean ? 8 : 3})).gap(px(10))`);
    ai("lane",`style().gap(px(6))`);
    ai("lane_head",`style().min_height(px(${clean ? 29 : 31})).padding_x(px(${clean ? 12 : 14})).gap(px(5))`);
    ai("lane_title",`style().font_size(px(13)).line_height(px(18)).font_weight(500)`);
    ai("count",`style().padding_x(px(5)).radius(px(${clean ? 4 : 7})).background(${panel}).font_size(px(11)).line_height(px(16)).font_feature("tnum",1)`);
    ai("cards",`style().gap(px(${clean ? 5 : 4}))`);
    ai("card",`style().padding(px(2)).radius(px(${clean ? 8 : 12})).background(${panel})${shadows(clean ? [hairline] : smallShadow)}`);
    ai("card_open",`style().background(${panel})${shadows(smallShadow)}`);
    ai("card_highlight",`style().background(${selected})${shadows([shadow(0,0,0,1,focus,true)])}`);
    ai("main",`style().padding(px(${clean ? 10 : 12})).gap(px(5)).radius(px(${clean ? 7 : 10}))`);
    ai("main_list",`style().padding_y(px(${clean ? 8 : 9}))`);
    ai("task_title",`style().font_size(px(13.5)).line_height(px(19)).font_weight(600)`);
    ai("description",`style().font_size(px(12)).line_height(px(18)).text_color(${ink})`);
    ai("state",`style().padding_x(px(6)).padding_y(px(2)).radius(px(${clean ? 5 : 8})).font_size(px(11.5)).line_height(px(16)).font_feature("tnum",1)`);
    ai("state_waiting",`style().background(alpha(${color("text.warning")},0.09))`);
    ai("state_running",`style().background(alpha(${color("accent")},0.08))`);
    ai("state_done",`style().background(alpha(${color("text.success")},0.08))`);
    ai("state_failed",`style().background(alpha(${color("text.danger")},0.08))`);
    ai("metric",`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    ai("body",`style().padding(px(8)).gap(px(6)).radius(px(${clean ? 6 : 8})).background(${wash})`);
    ai("agent",`style().font_size(px(11.5)).line_height(px(16)).text_color(${ink}).font_weight(500)`);
    ai("step",`style().font_size(px(11.5)).line_height(px(16)).text_color(${ink})`);
    ai("step_time",`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    ai("empty",`style().padding(px(11)).radius(px(${clean ? 6 : 9})).border(px(${clean ? .5 : 1})).border_color(${edge})`);
    for(const part of ["summary","notice"]){ai(part,`style().padding_x(px(${clean ? 20 : 17})).padding_y(px(4)).font_size(px(12)).line_height(px(17)).text_color(${muted})`);}
    const ap=(part,value)=>set("patterns/agent_plan",part,value);
    ap("root",`style().padding(px(${clean ? 11 : 9})).gap(px(12)).radius(px(${clean ? 12 : 18})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    ap("head",`style().min_height(px(${clean ? 38 : 40})).padding_x(px(7)).padding_y(px(4)).gap(px(7))`);
    ap("title",`style().font_size(px(${clean ? 14 : 15})).font_weight(${clean ? 500 : 600})`);
    ap("status",`style().padding_x(px(6)).padding_y(px(2)).radius(px(5)).font_size(px(11)).line_height(px(16)).background(${wash})`);
    ap("estimate",`style().font_size(px(11.5)).line_height(px(17)).text_color(${muted})`);
    ap("workspace",`style().background(${panel}).padding(px(${clean ? 6 : 8})).radius(px(${clean ? 7 : 11}))`);
    ap("sort_item",`style().radius(px(${clean ? 5 : 8})).background(${panel})${shadows(clean ? [hairline] : smallShadow)}`);
    ap("step_head",`style().padding_y(px(6)).padding_end(px(6)).gap(px(6))`);
    ap("step_title",`style().font_size(px(13)).line_height(px(18)).font_weight(500)`);
    ap("detail",`style().font_size(px(12)).line_height(px(17)).text_color(${muted})`);
    ap("tool",`style().font_size(px(10.5)).line_height(px(16)).text_color(${muted})`);
    ap("number",`style().min_width(px(16)).font_size(px(11)).line_height(px(16)).font_feature("tnum",1)`);
    ap("marker",`style().width(px(${clean ? 21 : 23})).height(px(${clean ? 21 : 23})).radius(px(12)).font_size(px(11))`);
    ap("elapsed",`style().font_size(px(11)).line_height(px(16)).font_feature("tnum",1).text_color(${muted})`);
    ap("error",`style().font_size(px(12)).line_height(px(18))`);
    ap("editor",`style().padding(px(9)).gap(px(6)).radius(px(${clean ? 6 : 8})).background(${panel})`);
    ap("editor_title",`style().font_size(px(13)).line_height(px(18)).font_weight(500)`);
    ap("field_label",`style().font_size(px(11.5)).line_height(px(17)).text_color(${muted})`);
    ap("footer",`style().padding_x(px(7)).gap(px(6))`);
    for(const part of ["summary","notice"]){ap(part,`style().font_size(px(11.5)).line_height(px(17)).text_color(${muted})`);}
    const ac=(part,value)=>set("patterns/agent_checkpoints",part,value);
    ac("root",`style().padding(px(${clean ? 7 : 4})).gap(px(3)).radius(px(${clean ? 12 : 16})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    ac("head",`style().padding_x(px(${clean ? 7 : 10})).padding_y(px(6)).gap(px(8))`);
    ac("title",`style().font_size(px(14)).line_height(px(20)).font_weight(${clean ? 500 : 600})`);
    ac("body",`style().background(${panel}).radius(px(${clean ? 8 : 12}))${shadows(clean ? [hairline] : smallShadow)}`);
    ac("title_text",`style().font_size(px(14)).line_height(px(20)).letter_spacing_em(${clean ? "0.0" : "-0.006"}).text_color(${ink})`);
    ac("row",`style().min_height(px(32)).radius(px(${clean ? 5 : 7}))`);
    ac("content",`style().radius(px(${clean ? 5 : 7})).hover(style().background(alpha(${wash},0.35)))`);
    ac("icon",`style().background(${clean ? panel : wash}).radius(px(5))`);
    for(const part of ["task","summary","meta","empty"]){
      ac(part,`style().font_size(px(12.5)).line_height(px(18)).text_color(${muted})`);
    }
    ac("meta",`style().font_size(px(12.5)).line_height(px(18)).font_feature("tnum",1).text_color(${muted})`);
    ac("notice",`style().padding_x(px(${clean ? 7 : 10})).padding_y(px(3)).font_size(px(12.5)).line_height(px(18)).text_color(${ink})`);
    for(const part of ["attempt","current","failed"]){
      const highlight=part === "failed" ? color("text.danger") : color("accent");
      ac(part,`style().padding_x(px(5)).padding_y(px(1)).radius(px(4)).font_size(px(11)).line_height(px(16)).text_color(${part === "failed" ? highlight : ink}).background(${part === "attempt" ? wash : `alpha(${highlight},0.10)`})`);
    }
    ac("rewind",`style().min_height(px(23)).font_size(px(11.5)).line_height(px(16))`);
    const at=(part,value)=>set("patterns/agent_trail",part,value);
    at("root",`style().padding(px(${clean ? 6 : 4})).gap(px(2)).radius(px(${clean ? 12 : 16})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    at("head",`style().min_height(px(33)).padding_x(px(${clean ? 8 : 10})).padding_y(px(3)).gap(px(8))`);
    at("title",`style().font_size(px(14)).line_height(px(20)).font_weight(500).text_color(${ink})`);
    at("name",`style().font_size(px(14)).line_height(px(20)).text_color(${ink})`);
    at("body",`style().padding(px(${clean ? 6 : 5})).radius(px(${clean ? 8 : 12})).background(${panel})${shadows(clean ? [hairline] : smallShadow)}`);
    at("row",`style().padding_x(px(4)).padding_y(px(2)).gap(px(6))`);
    at("agent",`style().padding_x(px(6)).padding_y(px(2)).font_size(px(12)).line_height(px(16)).radius(px(5)).background(${clean ? wash : panel}).text_color(${ink})`);
    at("tool",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    at("sublist",`style().padding_start(px(21)).padding_end(px(8)).padding_y(px(2))`);
    at("sub",`style().padding_x(px(4)).gap(px(6))`);
    at("subtitle",`style().font_size(px(13)).line_height(px(18)).text_color(${ink})`);
    for(const part of ["detail","clock","count","subdetail","empty"]){
      at(part,`style().font_size(px(${part === "count" ? 13 : 12})).line_height(px(16)).text_color(${muted}).font_feature("tnum",1)`);
    }
    for(const part of ["pending","waiting","declined","failed","status"]){
      const tone=part === "failed" ? color("text.danger") : ink;
      at(part,`style().padding_x(px(5)).padding_y(px(1)).radius(px(5)).font_size(px(11.5)).line_height(px(16)).text_color(${tone}).background(${part === "failed" ? `alpha(${tone},0.09)` : part === "status" ? `alpha(${color("accent")},0.09)` : wash})`);
    }
    at("fold",`style().font_size(px(11.5)).line_height(px(16))`);
    at("question",`style().font_size(px(12)).line_height(px(18)).text_color(${ink})`);
    at("ask",`style().padding(px(8)).radius(px(${clean ? 5 : 7})).background(${wash})`);
    at("notice",`style().padding_x(px(6)).padding_y(px(5)).font_size(px(12)).line_height(px(17)).text_color(${ink})`);
    const atr=(part,value)=>set("patterns/agent_tree",part,value);
    atr("root",`style().padding(px(${clean ? 8 : 6})).gap(px(8)).radius(px(${clean ? 12 : 16})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    atr("head",`style().min_height(px(33)).padding_x(px(${clean ? 8 : 12})).padding_y(px(5)).gap(px(8))`);
    atr("title",`style().font_size(px(14)).line_height(px(20)).font_weight(${clean ? 500 : 600}).text_color(${ink})`);
    atr("card",`style().padding(px(${clean ? 8 : 9})).gap(px(6)).radius(px(${clean ? 8 : 12})).background(${panel}).border(px(1)).border_color(${edge}).hover(style().border_color(${edgeStrong}))${shadows(clean ? [] : smallShadow)}`);
    atr("selected",`style().border_color(${color("accent")}).hover(style().border_color(${color("accent")}))`);
    atr("related",`style().border_color(alpha(${color("accent")},0.44))`);
    atr("outside",`style().border_color(${edge})`);
    atr("supervisor",`style().border_color(alpha(${color("text_muted")},0.5))`);
    atr("avatar",`style().background(${wash}).text_color(${ink}).font_weight(600)`);
    atr("name",`style().font_size(px(14)).line_height(px(20)).font_weight(600).text_color(${ink})`);
    atr("task",`style().font_size(px(12)).line_height(px(17)).text_color(${ink})`);
    for(const part of ["model","cost","time","count","empty"]){
      atr(part,`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted}).font_feature("tnum",1)`);
    }
    atr("running_count",`style().font_size(px(12.5)).line_height(px(18)).text_color(${color("tag.info_fg")}).font_feature("tnum",1)`);
    atr("total",`style().font_size(px(12.5)).line_height(px(18)).font_weight(600).text_color(${ink}).font_feature("tnum",1)`);
    atr("status",`style().font_size(px(11.5)).line_height(px(16)).padding_x(px(5)).padding_y(px(1)).radius(px(5)).background(${wash}).text_color(${ink})`);
    for(const [phase,tone] of [["running","info"],["waiting","warning"],["failed","danger"],["done","success"]]){
      atr("status_"+phase,`style().background(${color("tag."+tone+"_bg")}).text_color(${color("tag."+tone+"_fg")})`);
    }
    atr("summary_button",`style().border(px(0)).padding_x(px(8)).radius(px(20)).min_height(px(24)).shadows([]).font_size(px(12.5)).line_height(px(18))`);
    for(const [phase,tone] of [["waiting","warning"],["failed","danger"]]){
      const bg=color("tag."+tone+"_bg"), fg=color("tag."+tone+"_fg");
      atr("summary_"+phase,`style().background(${bg}).text_color(${fg}).hover(style().background(${bg}).text_color(${fg})).active(style().background(${bg}).text_color(${fg}))`);
    }
    atr("fold",`style().border(px(0)).padding_x(px(6))`);
    atr("review",`style().padding(px(8)).gap(px(5)).background(${wash}).radius(px(${clean ? 5 : 7}))`);
    atr("review_detail",`style().font_size(px(12)).line_height(px(17)).text_color(${ink})`);
    atr("notice",`style().padding_x(px(6)).padding_y(px(4)).font_size(px(12)).line_height(px(17)).text_color(${ink})`);
    const ag=(part,value)=>set("patterns/agent_graph",part,value);
    ag("root",`style().padding(px(${clean ? 6 : 4})).gap(px(3)).radius(px(${clean ? 14 : 20})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    ag("bar",`style().min_height(px(33)).padding_x(px(${clean ? 8 : 12})).padding_y(px(5)).gap(px(7))`);
    ag("title",`style().font_size(px(14)).line_height(px(20)).font_weight(${clean ? 500 : 600}).text_color(${ink})`);
    ag("count",`style().font_size(px(12.5)).line_height(px(18)).text_color(${muted}).font_feature("tnum",1)`);
    ag("total",`style().font_size(px(12.5)).line_height(px(18)).font_weight(600).text_color(${ink}).font_feature("tnum",1)`);
    ag("panel",`style().background(${panel}).radius(px(${clean ? 10 : 16}))${shadows(clean ? [hairline] : smallShadow)}`);
    ag("node",`style().radius(px(${clean ? 10 : 13})).background(${panel}).border(px(1)).border_color(${edge}).hover(style().border_color(${edgeStrong}))${shadows(smallShadow)}`);
    ag("agent",`style().radius(px(${clean ? 12 : 16}))${shadows([shadow(0,0,0,3,wash),shadow(0,0,0,4,edge),shortDrop(black(.14))])}`);
    ag("tool",`style().background(${panel})${shadows(smallShadow)}`);
    ag("user",`style().background(${strongest}).text_color(${dark ? color("surface_raised") : color("on_accent")})`);
    ag("lead",`style().radius(px(${clean ? 13 : 17}))${shadows([shadow(0,0,0,3,wash),shadow(0,0,0,4,color("agent_tree.link")),shortDrop(black(.17))])}`);
    ag("node_selected",`style().border_color(${color("accent")}).hover(style().border_color(${color("accent")}))`);
    ag("node_neighbor",`style().border_color(alpha(${color("accent")},0.5))`);
    ag("node_quiet",`style().border_color(${edge})${shadows([hairline])}`);
    ag("node_label",`style().font_size(px(11.5)).line_height(px(16)).font_weight(500).text_color(${ink})`);
    ag("inspector",`style().background(${panel}).radius(px(${clean ? 9 : 12})).padding(px(9)).gap(px(5))${shadows(floatingShadow)}`);
    ag("inspector_title",`style().font_size(px(14)).line_height(px(20)).font_weight(600)`);
    ag("status",`style().padding_x(px(5)).padding_y(px(1)).radius(px(6)).font_size(px(11)).line_height(px(16)).background(${wash}).text_color(${ink})`);
    for(const [state,tone] of [["working","info"],["waiting","warning"],["error","danger"],["done","success"]]){
      ag("status_"+state,`style().background(${color("tag."+tone+"_bg")}).text_color(${color("tag."+tone+"_fg")})`);
    }
    ag("calls",`style().padding(px(3)).radius(px(6)).background(${wash})`);
    ag("call",`style().padding_x(px(5)).padding_y(px(3)).radius(px(5)).background(${panel})`);
    ag("call_peer",`style().font_size(px(12)).line_height(px(17)).text_color(${ink})`);
    for(const part of ["detail","call_label","call_end","empty","legend_label","notice","inspect_cost"]){
      ag(part,`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    }
    ag("legend",`style().padding_x(px(8)).padding_y(px(7)).gap(px(14))`);
    ag("notice",`style().padding_x(px(8)).padding_y(px(6)).font_size(px(12)).line_height(px(17)).text_color(${ink})`);
    const af=(part,value)=>set("patterns/agent_fleet",part,value);
    af("root",`style().padding(px(${clean ? 8 : 6})).gap(px(8)).radius(px(${clean ? 12 : 16})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    af("bar",`style().min_height(px(34)).padding_x(px(9)).padding_y(px(4)).gap(px(8))`);
    af("title",`style().font_size(px(14)).line_height(px(20)).font_weight(${clean ? 500 : 600}).text_color(${ink})`);
    af("lead",`style().padding(px(${clean ? 8 : 9})).gap(px(5)).radius(px(${clean ? 8 : 12})).background(${panel}).border(px(1)).border_color(${edge}).hover(style().border_color(${edgeStrong}))${shadows(clean ? [] : smallShadow)}`);
    af("lead_open",`style().border_color(${color("accent")}).hover(style().border_color(${color("accent")}))`);
    af("lead_quiet",`style().border_color(${edge})`);
    af("highlight",`style().border_color(${color("accent")})${shadows([shadow(0,0,0,1,focus)])}`);
    af("sub",`style().padding(px(${clean ? 8 : 9})).gap(px(5)).radius(px(${clean ? 7 : 10})).background(${panel}).border(px(1)).border_color(${edge}).hover(style().border_color(${edgeStrong}))${shadows(clean ? [] : smallShadow)}`);
    af("avatar",`style().width(px(24)).height(px(24)).radius(px(${clean ? 6 : 8})).background(${wash}).text_color(${ink}).font_size(px(10))`);
    af("name",`style().font_size(px(13)).line_height(px(17)).font_weight(600).text_color(${ink})`);
    af("model",`style().font_size(px(11.5)).line_height(px(15)).text_color(${muted})`);
    af("task",`style().font_size(px(12)).line_height(px(17)).text_color(${ink})`);
    af("status",`style().font_size(px(11)).line_height(px(16)).gap(px(3)).padding_x(px(5)).padding_y(px(1)).radius(px(999)).background(${wash}).text_color(${ink})`);
    for(const [phase,tone] of [["running","info"],["waiting","warning"],["done","success"],["failed","danger"]]){
      af("status_"+phase,`style().background(${color("tag."+tone+"_bg")}).text_color(${color("tag."+tone+"_fg")})`);
    }
    for(const part of ["count","time","branch_empty","empty"]){
      af(part,`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted}).font_feature("tnum",1)`);
    }
    for(const part of ["cost","total"]){
      af(part,`style().font_size(px(12)).line_height(px(17)).font_weight(600).text_color(${ink}).font_feature("tnum",1)`);
    }
    af("summary",`style().padding_top(px(3)).border_top(px(${clean ? .5 : 1})).border_color(${edge})`);
    af("summary_value",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    af("inspect_title",`style().font_size(px(14)).line_height(px(20)).font_weight(600)`);
    af("evidence",`style().font_size(px(12)).line_height(px(18)).text_color(${ink})`);
    af("step",`style().padding_x(px(5)).padding_y(px(3)).radius(px(5)).background(${panel})`);
    for(const part of ["step_tool","step_state","step_time"]){
      af(part,`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    }
    af("notice",`style().padding_x(px(6)).padding_y(px(4)).font_size(px(12)).line_height(px(17)).text_color(${ink})`);
    const am=(part,value)=>set("patterns/agent_monitor",part,value);
    am("root",`style().padding(px(${clean ? 7 : 5})).gap(px(7)).radius(px(${clean ? 12 : 19})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? [hairline] : [])}`);
    am("card",`style().padding(px(${clean ? 6 : 8})).gap(px(7)).radius(px(${clean ? 9 : 14})).background(${panel})${shadows(clean ? [] : smallShadow)}`);
    am("card_folded",`style().padding(px(4)).gap(px(3)).background(${wash})${shadows([])}`);
    am("bar",`style().min_height(px(${clean ? 30 : 34})).padding_x(px(4)).gap(px(6))`);
    am("bar_folded",`style().padding_x(px(5)).min_height(px(30)).gap(px(5))`);
    am("agent",`style().gap(px(6)).radius(px(6))`);
    am("mark",`style().background(${wash}).width(px(24)).height(px(24)).radius(px(6))`);
    am("name",`style().font_size(px(14)).line_height(px(20)).font_weight(600).text_color(${ink})`);
    am("role",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    am("cost",`style().font_size(px(12)).line_height(px(16)).font_weight(600).text_color(${ink}).font_feature("tnum",1)`);
    am("status",`style().padding_x(px(6)).padding_y(px(1)).radius(px(6)).font_size(px(11)).line_height(px(16)).background(${wash}).text_color(${ink})`);
    for(const [phase,tone] of [["running","info"],["waiting","warning"],["finished","success"],["failed","danger"]]){
      am("status_"+phase,`style().background(${color("tag."+tone+"_bg")}).text_color(${color("tag."+tone+"_fg")})`);
    }
    am("status_paused",`style().background(${wash}).text_color(${ink})`);
    am("well",`style().min_height(px(${clean ? 34 : 39})).padding_x(px(8)).padding_y(px(5)).gap(px(6)).radius(px(${clean ? 6 : 9})).background(${wash})`);
    am("well_waiting",`style().background(${color("tag.warning_bg")})`);
    am("well_failed",`style().background(${color("tag.danger_bg")})`);
    am("well_label",`style().font_size(px(13)).line_height(px(19)).font_weight(500).text_color(${ink})`);
    am("clock",`style().min_width(px(36)).font_size(px(12)).line_height(px(18)).font_feature("tnum",1).text_color(${ink})`);
    am("command",`style().font_size(px(12)).line_height(px(17)).text_color(${ink})`);
    am("tool",`style().width(px(19)).height(px(19)).radius(px(5)).background(${panel}).text_color(${muted})`);
    am("tool_live",`style().width(px(20)).height(px(20)).radius(px(5)).background(${color("tag.info_bg")}).text_color(${color("tag.info_fg")})`);
    am("strip",`style().gap(px(2)).padding_x(px(4))`);
    am("segment_done",`style().background(alpha(${color("accent")},0.31))`);
    am("segment_running",`style().background(${color("accent")})`);
    am("segment_failed",`style().background(${color("text.danger")})`);
    am("segment_hot",`style().background(${color("accent")}).height(px(5))`);
    am("history",`style().padding_x(px(2))`);
    am("history_content",`style().gap(${length("space.unit")}.times(0.5))`);
    am("history_row",`style().min_height(px(28)).padding_x(px(5)).radius(px(5)).hover(style().background(${wash}))`);
    am("history_hot",`style().background(${wash})`);
    am("history_name",`style().font_size(px(12)).line_height(px(17)).text_color(${ink})`);
    am("footer",`style().padding_x(px(5)).padding_y(px(3)).gap(px(6))`);
    for(const part of ["history_time","counter","outcome","notice"]){
      am(part,`style().font_size(px(11.5)).line_height(px(17)).text_color(${part === "notice" ? ink : muted})`);
    }
    const astr=(part,value)=>set("patterns/agent_steering",part,value);
    astr("root",`style().padding(px(${clean ? 6 : 4})).gap(px(7)).radius(px(${clean ? 12 : 20})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? [hairline] : [])}`);
    astr("card",`style().padding(px(${clean ? 8 : 10})).gap(px(8)).radius(px(${clean ? 9 : 16})).background(${panel})${shadows(clean ? [] : smallShadow)}`);
    astr("header",`style().padding_x(px(4)).padding_y(px(4)).gap(px(8))`);
    astr("title",`style().font_size(px(14)).line_height(px(20)).font_weight(600).text_color(${ink})`);
    astr("agent",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    astr("clock",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted}).font_feature("tnum",1)`);
    astr("state",`style().padding_x(px(6)).padding_y(px(1)).radius(px(6)).font_size(px(11)).line_height(px(16)).background(${wash}).text_color(${ink})`);
    for(const [phase,tone] of [["running","info"],["paused","warning"],["done","success"]]){
      astr("state_"+phase,`style().background(${color("tag."+tone+"_bg")}).text_color(${color("tag."+tone+"_fg")})`);
    }
    astr("plan",`style().padding_x(px(4)).padding_y(px(2))`);
    const stepMarkWidth=16;
    const stepRailGap=clean ? 10 : 12;
    astr("marker",`style().width(px(${stepMarkWidth})).height(px(${stepMarkWidth}))`);
    astr("step_lead",`style().width(px(${stepMarkWidth + stepRailGap}))`);
    astr("step","style().min_height(px(31))");
    astr("step_title",`style().font_size(px(13)).line_height(px(19)).font_weight(500).text_color(${ink})`);
    astr("step_clock",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted}).font_feature("tnum",1)`);
    astr("note",`style().padding_x(px(6)).padding_y(px(3)).gap(px(5)).radius(px(${clean ? 5 : 8})).background(${wash})`);
    astr("notes","style().gap(px(2))");
    astr("note_applied",`style().padding_start(px(${clean ? 6 : 8})).padding_end(px(${clean ? 8 : 10})).padding_y(px(${clean ? 4 : 6})).gap_x(px(6)).radius(px(${clean ? 6 : 10})).background(${color("agent_steering.note_bg")})${clean ? shadows([shadow(0,0,0,.5,edge,true)]) : ""}`);
    const steeringNoteMuted=clean ? (dark ? white(.45) : black(.45)) : rgba(dark ? "8b8b92" : "8a8a8a");
    astr("note_arrow",`style().width(px(12)).height(px(12)).margin_top(px(${clean ? 2 : 3})).text_color(${steeringNoteMuted})`);
    astr("note_text",`${readingType(clean ? 12 : 13,clean ? 16 : 18)}.text_color(${ink})`);
    astr("note_tag",`${readingType(12,clean ? 16 : 18)}.gap(px(3)).font_weight(500).text_color(${color("agent_steering.tag")})`);
    astr("queue",`style().padding_y(px(6)).gap(px(5)).border_top(px(${clean ? .5 : 1})).border_color(${edge})`);
    astr("queue_title",`style().font_size(px(11.5)).line_height(px(16)).font_weight(600).text_color(${muted})`);
    astr("compose",`style().padding_x(px(4)).padding_y(px(2)).gap(px(5))`);
    astr("field",`style().min_height(px(${clean ? 28 : 32})).radius(px(${clean ? 6 : 9}))`);
    astr("hint",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    astr("keycap",`style().padding_x(px(3)).radius(px(3)).font_size(px(11)).line_height(px(15)).text_color(${ink}).background(${panel})`);
    astr("notice",`style().padding_x(px(5)).padding_y(px(4)).font_size(px(12)).line_height(px(17)).text_color(${ink})`);
    const recap=(part,value)=>set("patterns/agent_recap",part,value);
    const rc=name=>color("agent_recap."+name);
    recap("root",`style().padding(px(${clean ? 16 : 4})).gap(px(${clean ? 14 : 4})).radius(px(${clean ? 14 : 20})).background(${rc(clean ? "panel" : "tray")})${shadows(clean ? smallShadow : [])}`);
    recap("panel",`style().padding_x(px(${clean ? 0 : 16})).padding_top(px(${clean ? 0 : 16})).padding_bottom(px(${clean ? 0 : 12})).gap(px(4)).radius(px(${clean ? 0 : 16})).background(${rc("panel")})${shadows(clean ? [] : smallShadow)}`);
    recap("head",`style().gap_x(px(16)).gap_y(px(8))`);
    recap("title",`style().font_size(px(15)).line_height(px(${clean ? 24 : 22})).font_weight(${clean ? 500 : 600}).text_color(${strongest})`);
    recap("since",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(18)).text_color(${rc("quiet")}).font_feature("tnum",1)`);
    recap("pill",`style().font_size(px(${clean ? 11.5 : 12})).line_height(px(18)).padding_x(px(${clean ? 6 : 7})).radius(px(${clean ? 5 : 6})).background(${rc("soft")}).text_color(${rc("text")})`);
    recap("summary",`style().gap_x(px(${clean ? 12 : 14})).gap_y(px(4)).padding_top(px(3))`);
    recap("summary_item",`style().gap(px(5)).text_color(${rc("quiet")})`);
    recap("summary_count",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(16)).font_weight(${clean ? 500 : 600}).text_color(${ink}).font_feature("tnum",1)`);
    recap("summary_label",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(16)).text_color(${rc("quiet")})`);
    recap("plot",`style().height(px(96)).margin_top(px(10))`);
    recap("curve_fill",`style().linear_gradient(linear_gradient(#{angle:180,from:${rc("area")},to:alpha(${rc("area")},0.0)}))`);
    recap("time",`style().font_size(px(${clean ? 10.5 : 11.5})).line_height(px(14)).text_color(${rc("faint")}).font_feature("tnum",1)`);
    recap("tip",`style().max_width(relative(1.0)).padding_x(px(${clean ? 8 : 9})).padding_y(px(${clean ? 3 : 4})).radius(px(${clean ? 6 : 8})).background(${rgba(dark ? "fafafa" : "18181b")}).text_color(${rgba(dark ? "111113" : "fafafa")}).font_size(px(${clean ? 11.5 : 12})).line_height(px(16))`);
    recap("board",`style().gap(px(${clean ? 8 : 4}))`);
    recap("lane",`style().padding(px(${clean ? 4 : 0})).radius(px(${clean ? 14 : 0})).background(${clean ? rc("tray") : transparent})`);
    recap("lane_head",`style().min_height(px(${clean ? 32 : 38})).padding_x(px(${clean ? 9 : 10}))`);
    recap("lane_label",`style().font_weight(500).font_size(px(${clean ? 11.5 : 13})).line_height(px(18)).text_color(${rc(clean ? "quiet" : "text")})`);
    recap("lane_count",`style().min_width(px(${clean ? 20 : 22})).padding_x(px(6)).line_height(px(${clean ? 18 : 20})).font_size(px(${clean ? 11 : 11.5})).radius(px(${clean ? 6 : 99})).background(${rc("soft")}).text_color(${rc("text")}).text_center().font_feature("tnum",1)`);
    recap("card",`style().padding(px(${clean ? 9 : 10})).min_height(px(${clean ? 90 : 96})).gap(px(2)).radius(px(${clean ? 10 : 14})).background(${rc("panel")})${shadows(smallShadow)}`);
    recap("card_hot",`style()${shadows([shadow(0,0,0,1,edgeStrong),shortDrop(black(.05))])}`);
    recap("card_head",`style().min_height(px(22)).gap(px(8))`);
    recap("mark",`style().width(px(20)).height(px(20)).radius(px(6)).font_size(px(10.5)).font_weight(600).text_color(${ink})${shadows([shadow(0,0,0,1,edge,true)])}`);
    recap("card_title",`style().font_size(px(${clean ? 13 : 13.5})).line_height(px(20)).text_color(${ink})`);
    recap("card_quiet",`style().text_color(${rc("quiet")})`);
    recap("meta",`style().min_height(px(18)).padding_start(px(${clean ? 26 : 28})).gap(px(8))`);
    recap("detail",`style().font_size(px(${clean ? 11.5 : 12.5})).line_height(px(16)).text_color(${rc("quiet")})`);
    recap("failure",`style().font_size(px(${clean ? 11.5 : 12.5})).line_height(px(16)).text_color(${rc("failed")})`);
    recap("quantity",`style().padding_x(px(5)).radius(px(${clean ? 4 : 99})).font_size(px(${clean ? 10.5 : 11})).line_height(px(16)).background(${rc("soft")}).text_color(${rc("text")}).font_feature("tnum",1)`);
    recap("footer",`style().min_height(px(${clean ? 24 : 28})).margin_top(px(6)).gap(px(4))`);
    for(const [part,fill,foreground] of [["status","ok_bg","ok"],["undo_state","soft","text"]]){
      recap(part,`style().padding_x(px(${clean ? 6 : 8})).radius(px(${clean ? 5 : 99})).font_size(px(${clean ? 11.5 : 12})).line_height(px(${clean ? 20 : 22})).background(${rc(fill)}).text_color(${rc(foreground)})`);
    }
    recap("primary",`style().min_height(px(${clean ? 24 : 28})).padding_x(px(${clean ? 9 : 10})).radius(px(${clean ? 6 : 8})).background(${clean ? rc("panel") : rgba(dark ? "fafafa" : "171717")}).text_color(${clean ? ink : rgba(dark ? "111113" : "ffffff")})${shadows(smallShadow)}`);
    recap("quiet",`style().min_height(px(${clean ? 24 : 28})).padding_x(px(${clean ? 8 : 10})).radius(px(${clean ? 6 : 8})).text_color(${rc(clean ? "quiet" : "text")}).hover(style().background(${rc("soft")}).text_color(${ink}))`);
    recap("empty",`style().font_size(px(12)).line_height(px(18)).text_color(${rc("faint")}).text_center().padding_y(px(12))`);
    recap("notice",`style().padding_x(px(4)).font_size(px(12)).text_color(${ink})`);
    const findings=(part,value)=>set("patterns/agent_findings",part,value);
    const vf=name=>color("agent_findings."+name);
    findings("root",`style().max_width(px(640)).padding(px(${clean ? 0 : 4})).gap(px(0)).radius(px(${clean ? 10 : 20})).background(${vf("tray")}).letter_spacing_em(${clean ? 0 : -0.006})${shadows(clean ? smallShadow : [])}`);
    findings("bar",`style().padding_start(px(${clean ? 16 : 14})).padding_end(px(${clean ? 16 : 12})).padding_top(px(${clean ? 14 : 10})).padding_bottom(px(${clean ? 14 : 12})).gap(px(10)).border_bottom(px(${clean ? .5 : 0})).border_color(${vf("line")})`);
    findings("spark",`style().width(px(16)).height(px(14)).text_color(${vf("spark")})`);
    findings("title",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(${clean ? 500 : 600}).text_color(${strongest})`);
    findings("completion",`style().font_size(px(12)).line_height(px(16)).text_color(${vf("quiet")}).font_feature("tnum",1)`);
    findings("totals",`style().gap(px(12))`);
    findings("counter",`style().font_size(px(12)).line_height(px(16)).font_feature("tnum",1)`);
    for(const tone of ["critical","warning","note","faint"])
      findings(tone,`style().text_color(${vf(tone)})`);
    const vfPanel=(part,pad)=>findings(part,`style().padding(px(${pad})).radius(px(${clean ? 0 : 16})).background(${clean ? transparent : vf("panel")})${shadows(clean ? [] : smallShadow)}`);
    vfPanel("agents",6);vfPanel("feed",6);
    findings("agent",`style().min_height(px(${clean ? 48 : 52})).padding_x(px(10)).radius(px(${clean ? 8 : 12})).isolated_opacity(true).hover(style().background(${vf("hover")}))`);
    findings("selected",`style().background(${vf("selected")}).hover(style().background(${vf("selected")}))`);
    findings("dim",`style().opacity(0.5).hover(style().opacity(0.8).background(${vf("hover")}))`);
    findings("avatar",`style().width(px(${clean ? 24 : 26})).height(px(${clean ? 24 : 26})).radius(px(${clean ? 6 : 99})).background(${vf("fill")}).font_size(px(10)).line_height(px(16)).text_color(${vf("avatar_ink")})`);
    findings("avatar_logo",`style().background(${vf("panel")})${shadows(smallShadow)}`);
    findings("name",`style().font_size(px(${clean ? 13 : 14})).line_height(px(${clean ? 18 : 19})).text_color(${ink})`);
    findings("step",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(${clean ? 16 : 17})).text_color(${vf("quiet")})`);
    findings("count",`style().font_size(px(12)).line_height(px(16)).font_feature("tnum",1)`);
    findings("track",`style().height(px(${clean ? 6 : 8})).radius(px(${clean ? 3 : 4})).background(${vf("fill")})`);
    findings("indicator",`style().radius(px(${clean ? 3 : 4}))`);
    for(const [part,tone] of [["running","run"],["failed","failed"],["done","done"]]){
      findings(part,`style().background(${vf(tone)})${shadows([shadow(0,0,0,.5,vf(tone+"_edge"),true)])}`);
    }
    findings("sheen",`style().layered_gradient(layered_gradient([#{angle:90,stops:[#{offset:0.0,color:${transparent}},#{offset:0.5,color:${vf("sheen")}},#{offset:1.0,color:${transparent}}]}])).opacity(0.7)`);
    findings("glow",`style().linear_gradient(linear_gradient(#{angle:90,from:${transparent},to:${vf("run_edge")}}))`);
    findings("feed_head",`style().min_height(px(40)).padding_x(px(${clean ? 16 : 14})).gap(px(8)).border_top(px(${clean ? .5 : 0})).border_color(${vf("line")})`);
    findings("label",`style().font_weight(500).font_size(px(${clean ? 10 : 13})).line_height(px(${clean ? 16 : 18})).text_color(${vf("quiet")})${clean ? ".letter_spacing_px(0.5)" : ""}`);
    findings("chip",`style().min_height(px(${clean ? 20 : 22})).padding_start(px(${clean ? 8 : 9})).padding_end(px(6)).radius(px(${clean ? 6 : 99})).background(${clean ? vf("fill") : vf("panel")}).text_color(${ink})${shadows(clean ? [] : smallShadow)}`);
    findings("chip_label",`style().font_size(px(12)).line_height(px(16)).font_weight(500).text_color(${ink})`);
    findings("finding_body",`style().min_height(px(40)).padding_x(px(10)).gap(px(10)).radius(px(${clean ? 8 : 12})).hover(style().background(${vf("hover")}))`);
    findings("finding_title",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).text_color(${ink})`);
    for(const part of ["author","location","separator","time","empty"]){
      findings(part,`style().font_size(px(${part === "location" ? 11.5 : 12})).line_height(px(16)).text_color(${vf(["separator","time"].includes(part) ? "faint" : "quiet")})`);
    }
    const ledger=(part,value)=>set("patterns/agent_ledger",part,value);
    const lc=name=>color("agent_ledger."+name);
    ledger("root",`style().padding(px(${clean ? 0 : 4})).gap(px(${clean ? 16 : 0})).radius(px(${clean ? 0 : 20})).background(${lc("tray")}).letter_spacing_em(${clean ? 0 : -0.006})`);
    ledger("bar",`style().padding_x(px(${clean ? 0 : 12})).padding_top(px(${clean ? 0 : 8})).padding_bottom(px(${clean ? 0 : 12})).gap_x(px(8)).gap_y(px(4)).items_start()`);
    ledger("mark",`style().width(px(16)).height(px(16)).text_color(${lc("accent")})`);
    ledger("title",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(${clean ? 500 : 600}).text_color(${strongest})`);
    for(const part of ["agent","total","undone_total","time","notice"]){
      ledger(part,`style().font_size(px(${clean ? 12 : 13})).line_height(px(${clean ? 16 : 18})).text_color(${lc("quiet")}).font_feature("tnum",1)`);
    }
    ledger("summary",`style().gap(px(14)).justify_between()`);
    ledger("panel",`style().padding(px(${clean ? 4 : 6})).radius(px(${clean ? 10 : 16})).background(${lc("panel")})${shadows(smallShadow)}`);
    ledger("row",`style().min_height(px(${clean ? 56 : 62})).padding_y(px(${clean ? 9 : 10})).padding_start(px(10)).padding_end(px(12)).radius(px(${clean ? 7 : 11})).hover(style().background(${lc("hover")}))`);
    for(const tone of ["preview","also","kept"]){
      ledger(tone,`style().background(${lc(tone)}).hover(style().background(${lc(tone)}))`);
    }
    ledger("divider",`style().height(px(${clean ? 0.5 : 1})).inset_start(px(${clean ? 50 : 54})).inset_end(px(12)).background(${lc("rule")})`);
    ledger("icon",`style().width(px(${clean ? 28 : 32})).height(px(${clean ? 28 : 32})).radius(px(${clean ? 7 : 9})).background(${lc("icon")}).font_size(px(${clean ? 12 : 13})).line_height(px(17)).font_weight(${clean ? 500 : 600}).text_color(${lc("text")})${shadows(smallShadow)}`);
    ledger("icon_undone",`style().isolated_opacity(true).opacity(0.5).foreground_grayscale(1)`);
    ledger("action",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(${clean ? 500 : 600}).text_color(${ink})`);
    ledger("target",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).text_color(${lc("text")})`);
    ledger("quiet",`style().text_color(${lc("quiet")})`);
    for(const part of ["before","after","arrow","detail","empty"]){
      const inkRole=part==="arrow"?"faint":part==="after"?"text":"quiet";
      ledger(part,`style().font_size(px(${clean ? 12 : 13})).line_height(px(${clean ? 17 : 19})).text_color(${lc(inkRole)})`);
    }
    ledger("before_line",`style().background(${lc("faint")})`);
    ledger("strike",`style().height(px(${clean ? 1 : 1.5})).background(${lc("strike")})`);
    ledger("end",`style().min_width(px(${clean ? 108 : 120})).height(px(${clean ? 24 : 28}))`);
    ledger("end_narrow",`style().min_width(px(${clean ? 68 : 76}))`);
    ledger("time",`style().width(relative(1.0)).text_end().font_size(px(${clean ? 12 : 13})).line_height(px(${clean ? 16 : 18})).text_color(${lc("quiet")}).font_feature("tnum",1)`);
    ledger("undone_time",`style().text_color(${lc("faint")})`);
    ledger("command",`style().padding_x(px(7)).radius(px(${clean ? 6 : 8})).text_color(${ink}).background(${clean ? transparent : lc("icon")})${shadows(clean ? [] : smallShadow)}`);
    ledger("command_narrow",`style().width(px(28)).padding_x(px(0))`);
    ledger("command_hot",`style().text_color(${lc("also_ink")}).background(${clean ? lc("also_chip") : lc("icon")})`);
    ledger("command_busy",`style().text_color(${lc("quiet")})`);
    ledger("command_text",`style().inherit_text_color().font_size(px(${clean ? 12 : 13})).line_height(px(${clean ? 16 : 18})).font_weight(500)`);
    ledger("flag",`style().min_height(px(${clean ? 20 : 24})).padding_x(px(${clean ? 7 : 10})).radius(px(${clean ? 5 : 99})).font_size(px(${clean ? 11.5 : 12.5})).line_height(px(${clean ? 20 : 24})).font_weight(500)`);
    ledger("flag_also",`style().background(${lc("also_chip")}).text_color(${lc("also_ink")})`);
    ledger("flag_kept",`style().background(${lc("kept_chip")}).text_color(${lc("kept_ink")})`);
    ledger("locked",`style().text_color(${lc("kept_ink")})`);
    const breakdown=(part,value)=>set("patterns/compact_breakdown",part,value);
    const cb=role=>color("compact_breakdown."+role);
    const metricLabel=clean ? ink : rgba(dark ? "c8c8cc" : "404040");
    const breakdownPanel=rgba(dark ? (clean ? "1c1c1e" : "19191b") : "ffffff");
    const breakdownTray=rgba(dark ? "111113" : "f0f0f0");
    const breakdownLine=clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2e" : "ececec");
    const breakdownWell=clean ? (dark ? white(.04) : black(.03)) : rgba(dark ? "222225" : "f4f4f5");
    breakdown("root",`style().radius(px(${clean ? 10 : 20})).padding(px(${clean ? 0 : 4})).background(${clean ? cb("card") : breakdownTray})${clean ? shadows(smallShadow) : ""}`);
    breakdown("panel",`style().padding_x(px(${clean ? 16 : 14})).padding_top(px(${clean ? 16 : 12})).padding_bottom(px(${clean ? 10 : 8})).border(px(${clean ? 0 : 1})).border_color(${breakdownLine}).radius(px(${clean ? 0 : 16})).background(${cb("card")})${shadows(clean ? [] : smallShadow)}`);
    if(clean)breakdown("panel_cozy","style().padding_x(px(14)).padding_top(px(14)).padding_bottom(px(8))");
    breakdown("head",`style().min_height(px(${clean ? 28 : 30})).gap(px(${clean ? 12 : 10}))`);
    breakdown("head_narrow","style().gap(px(6))");
    breakdown("title",`style().font_size(px(${clean ? 15 : 13.5})).line_height(px(${clean ? 24 : 20})).font_weight(${clean ? 500 : 600}).text_color(${clean ? strongest : ink})`);
    breakdown("periods",clean ? `style().gap(px(4)).padding(px(0)).radius(px(0)).background(${transparent}).border(px(0)).border_color(${transparent})` :
      `style().padding(px(2)).radius(px(10)).background(${breakdownWell}).border(px(1)).border_color(${breakdownLine})`);
    if(clean)breakdown("periods_cozy","style().gap(px(2))");
    breakdown("period",`style().min_height(px(${clean ? 28 : 24})).padding_x(px(${clean ? 10 : 8})).radius(px(${clean ? 6 : 8})).font_size(px(${clean ? 13 : 12})).line_height(px(20)).font_weight(500).border_color(${clean ? breakdownLine : transparent}).text_color(${clean ? muted : cb("muted")}).hover(style().text_color(${ink}))`);
    breakdown("period_tight","style().padding_x(px(6))");
    breakdown("period_active",`style().text_color(${ink})`);
    breakdown("period_fill",`style().radius(px(${clean ? 5 : 7})).background(${clean ? (dark ? white(.08) : black(.06)) : cb("card")})${shadows(clean ? [] : smallShadow)}`);
    breakdown("figure",`style().margin_top(px(${clean ? 14 : 12}))`);
    breakdown("total",`style().font_size(px(${clean ? 24 : 26})).line_height(px(32)).letter_spacing_em(-0.02).font_weight(${clean ? 500 : 600}).text_color(${clean ? strongest : ink})`);
    breakdown("delta",`style().radius(px(${clean ? 5 : 99})).font_size(px(12)).line_height(px(${clean ? 16 : 17.4})).font_weight(500)`);
    breakdown("delta_up",`style().text_color(${cb("up")}).background(${cb("up_bg")})`);
    breakdown("delta_down",`style().text_color(${cb("down")}).background(${cb("down_bg")})`);
    breakdown("bar",`style().height(px(${clean ? 6 : 8})).margin_top(px(${clean ? 16 : 12}))`);
    breakdown("segment",`style().radius(px(${clean ? 1.5 : 2.5}))`);
    breakdown("segment_start",`style().radius_start(px(${clean ? 3 : 4}))`);
    breakdown("segment_end",`style().radius_end(px(${clean ? 3 : 4}))`);
    breakdown("list",`style().margin_top(px(${clean ? 10 : 8}))`);
    breakdown("row",`style().min_height(px(${clean ? 32 : 30})).gap(px(${clean ? 10 : 9})).radius(px(${clean ? 6 : 8}))`);
    breakdown("row_active",`style().background(${breakdownWell})`);
    breakdown("dot",`style().radius(px(${clean ? 2 : 2.5}))`);
    breakdown("name",`${readingType(13,clean ? 20 : 18.85)}.text_color(${metricLabel})`);
    breakdown("amount",`${readingType(13,clean ? 20 : 18.85)}.font_weight(500).text_color(${ink})`);
    breakdown("share",`${readingType(clean ? 12 : 13,clean ? 16 : 18.85)}.text_color(${cb("muted")})`);
    for(const part of ["caption","empty","footer_text"]){
      breakdown(part,`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(${clean ? 16 : 18})).text_color(${cb("muted")})`);
    }
    breakdown("footer",clean ? `style().min_height(px(40)).padding_x(px(16)).padding_top(px(0)).border_top(px(0.5)).border_color(${breakdownLine})` :
      "style().min_height(px(34)).padding_x(px(10)).padding_top(px(4))");
    if(clean)breakdown("footer_cozy","style().padding_x(px(14))");
    const usage=(part,value)=>set("patterns/usage_meter",part,value);
    const um=role=>color("usage_meter."+role);
    usage("root",`style().radius(px(${clean ? 10 : 20})).padding(px(${clean ? 0 : 4})).background(${clean ? breakdownPanel : breakdownTray})${shadows(clean ? smallShadow : [])}`);
    usage("panel",`style().padding_x(px(16)).padding_top(px(${clean ? 16 : 14})).padding_bottom(px(${clean ? 16 : 18})).border(px(${clean ? 0 : 1})).border_color(${breakdownLine}).radius(px(${clean ? 0 : 16})).background(${breakdownPanel})${shadows(clean ? [] : smallShadow)}`);
    usage("panel_cozy",`style().padding_x(px(14)).padding_top(px(${clean ? 14 : 12})).padding_bottom(px(14))`);
    usage("head","style().gap(px(12))");
    usage("head_narrow","style().gap(px(8))");
    usage("title",`style().font_size(px(${clean ? 15 : 14})).line_height(px(${clean ? 24 : 20})).font_weight(${clean ? 500 : 600}).text_color(${clean ? strongest : ink})`);
    usage("plan",`style().font_size(px(12)).line_height(px(16)).text_color(${cb("muted")})`);
    usage("periods",clean ? `style().gap(px(8)).padding(px(0)).radius(px(0)).background(${transparent}).border(px(0)).border_color(${transparent})` :
      `style().padding(px(2)).radius(px(10)).background(${breakdownWell}).border(px(1)).border_color(${breakdownLine})`);
    if(clean)usage("periods_cozy","style().gap(px(4))");
    usage("period",`style().min_height(px(${clean ? 28 : 26})).padding_x(px(${clean ? 10 : 9})).radius(px(${clean ? 6 : 8})).font_size(px(${clean ? 13 : 12})).line_height(px(20)).font_weight(500).border_color(${clean ? breakdownLine : transparent}).text_color(${clean ? muted : cb("muted")}).hover(style().text_color(${ink}))`);
    usage("period_tight","style().padding_x(px(6))");
    usage("period_active",`style().text_color(${ink})`);
    usage("period_fill",`style().radius(px(${clean ? 5 : 7})).background(${clean ? um("fill") : breakdownPanel})${shadows(clean ? [] : smallShadow)}`);
    usage("figure",`style().margin_top(px(${clean ? 14 : 18}))${clean ? ".items_center()" : ""}`);
    usage("readout",`style().gap_x(px(${clean ? 8 : 6}))`);
    usage("total",`style().font_size(px(${clean ? 24 : 28})).line_height(px(${clean ? 32 : 34})).letter_spacing_em(${clean ? -0.01 : -0.02}).font_weight(${clean ? 500 : 600}).text_color(${clean ? strongest : ink})`);
    usage("muted",`style().font_size(px(${clean ? 12 : 13})).line_height(px(${clean ? 16 : 18.85})).text_color(${cb("muted")})`);
    usage("pct",`style().text_color(${clean ? muted : metricLabel}).padding_x(px(${clean ? 6 : 0})).gap_x(px(${clean ? 5 : 6}))
      .radius(px(${clean ? 5 : 0}))${clean ? ".min_height(px(20)).background("+um("fill")+")" : ""}`);
    usage("pct_value",`style().font_size(px(${clean ? 12 : 13})).line_height(px(${clean ? 16 : 18.85})).font_weight(500)`);
    usage("pct_warn",`style().text_color(${um("warn_text")}).background(${clean ? `alpha(${um("warn")},${dark ? 0.16 : 0.12})` : transparent})`);
    usage("pct_limit",`style().text_color(${clean ? um("full_text") : ink}).background(${clean ? `alpha(${um("full")},${dark ? 0.14 : 0.08})` : transparent})`);
    usage("dot",`style().background(${um("warn")})`);
    usage("dot_limit",`style().background(${clean ? um("full") : um("warn")})${shadows(clean ? [] : [shadow(0,0,0,3,`alpha(${um("warn")},0.22)`)])}`);
    usage("bar",`style().height(px(${clean ? 8 : 10})).radius(px(${clean ? 4 : 5})).background(${um("track")})`);
    usage("segment_start",`style().radius_start(px(${clean ? 4 : 5}))`);
    usage("segment_cut",`style().background(${breakdownPanel})`);
    usage("hint",`style().padding_x(px(10)).padding_top(px(${clean ? 6 : 7})).padding_bottom(px(${clean ? 7 : 8})).gap_y(px(${clean ? 2 : 1}))
      .background(${breakdownPanel}).border(px(${clean ? .5 : 1})).border_color(${breakdownLine}).radius(px(${clean ? 8 : 10}))${shadows(smallShadow)}`);
    usage("hint_name",`style().font_size(px(12)).font_weight(400).line_height(px(16)).text_color(${cb("muted")})`);
    usage("hint_amount",`style().font_size(px(13)).font_weight(500).line_height(px(${clean ? 20 : 18.85})).text_color(${ink})`);
    usage("hint_unit",`style().font_size(px(13)).font_weight(400).line_height(px(${clean ? 20 : 18.85})).text_color(${cb("muted")})`);
    usage("legend_label",`${readingType(13,clean ? 20 : 18.85)}.font_weight(400).text_color(${metricLabel})`);
    usage("row",`style().min_height(px(${clean ? 32 : 30})).gap(px(10)).padding_x(px(8)).radius(px(${clean ? 6 : 8}))`);
    usage("row_active",`style().background(${breakdownWell})`);
    usage("swatch",`style().radius(px(${clean ? 2 : 2.5}))`);
    usage("amount",`${readingType(13,clean ? 20 : 18.85)}.font_weight(500).text_color(${ink})`);
    usage("share",`${readingType(clean ? 12 : 13,clean ? 16 : 18.85)}.text_color(${cb("muted")})`);
    usage("day_active",`style().background(${um("accent")})`);
    usage("day_future",`style().background(${um("track")})`);
    usage("spark_few","style().gap(px(5))");
    usage("spark_dense","style().gap(px(1))");
    usage("limits",`style().border_top(px(${clean ? 0.5 : 1})).border_color(${breakdownLine})`);
    usage("limit_readout",readingType(13,clean ? 20 : 18.85));
    usage("limit_readout_cozy",readingType(clean ? 12 : 12.5,clean ? 16 : 18.85));
    usage("limit_track","style().margin_top(px(6)).height(px(4))");
    usage("limit_fill",`style().background(${clean ? um("accent") : um("fill")})`);
    usage("limit_warn",`style().background(${clean ? um("warn") : um("fill")})`);
    usage("limit_full",`style().background(${clean ? um("full") : um("fill")})`);
    usage("limit_unlimited",`style().background(${transparent})`);
    usage("footer",clean ? `style().min_height(px(44)).padding_start(px(16)).padding_end(px(8)).padding_top(px(6)).padding_bottom(px(6)).border_top(px(0.5)).border_color(${breakdownLine})` :
      `style().min_height(px(44)).padding_start(px(12)).padding_end(px(2)).padding_top(px(6)).padding_bottom(px(2)).border_top(px(0)).border_color(${transparent})`);
    usage("footer_cozy",`style().padding_start(px(${clean ? 14 : 10}))`);
    usage("strong",`style().font_size(px(${clean ? 12 : 12.5})).font_weight(500).text_color(${ink})`);
    const quotaQuietShadow=clean ? (dark ? [
      shadow(0,0,0,.5,white(.10)),shadow(0,1,2,0,black(.40)),
    ] : smallShadow) : [shadow(0,0,0,1,breakdownLine),shadow(0,1,2,0,black(.04))];
    const quotaStrongShadow=[shadow(0,1,2,0,black(.12))];
    if(!clean)quotaStrongShadow.push(shortDrop(black(.20)));
    const quotaHover=rgba(dark ? "232326" : clean ? "fafafa" : "f4f4f5");
    const quotaInk=clean ? strongest : ink;
    const quotaStrongFill=clean ? (dark ? white(.90) : black(.88)) : ink;
    const quotaStrongInk=clean ? rgba(dark ? "111113" : "fcfcfc") : breakdownPanel;
    usage("action",`style().min_height(px(${clean ? 28 : 32})).height(px(${clean ? 28 : 32})).padding_x(px(${clean ? 10 : 14})).border(px(0)).radius(px(${clean ? 6 : 14})).font_size(px(13)).line_height(px(20)).font_weight(500).text_color(${quotaInk}).background(${breakdownPanel})${shadows(quotaQuietShadow)}.hover(style().background(${quotaHover}).text_color(${quotaInk})${shadows(quotaQuietShadow)})`);
    usage("action_strong",`style().background(${quotaStrongFill}).text_color(${quotaStrongInk})${shadows(quotaStrongShadow)}.hover(style().background(${quotaStrongFill}).text_color(${quotaStrongInk}).isolated_opacity(true).opacity(0.88)${shadows(quotaStrongShadow)})`);
    const summary=(part,value)=>set("patterns/change_summary",part,value);
    const cs=role=>color("change_summary."+role);
    const productTray=rgba(dark ? "161618" : "f0f0f0");
    const productPanel=rgba(dark ? (clean ? "1c1c1e" : "232326") : "ffffff");
    const productLine=clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2e" : "ececec");
    const quietText=clean ? (dark ? white(.46) : black(.45)) : rgba(dark ? "8f8f8f" : "737373");
    const secondaryText=clean ? (dark ? white(.62) : black(.61)) : quietText;
    const productWash=clean ? (dark ? white(.025) : black(.015)) : rgba(dark ? "2a2a2d" : "fafafa");
    const productEdge=clean ? transparent : (dark ? white(.08) : rgba("e5e5e5"));
    summary("root",`style().padding(px(${clean ? 0 : 6})).radius(px(${clean ? 10 : 22})).background(${clean ? productPanel : productTray})${shadows(clean ? smallShadow : [])}`);
    summary("head",`style().min_height(px(${clean ? 44 : 41})).padding_x(px(${clean ? 16 : 18})).gap(px(12))${clean ? "" : ".padding_top(px(11)).padding_bottom(px(10))"}`);
    summary("head_narrow",`style().padding_x(px(${clean ? 14 : 16}))`);
    summary("heading","style().gap(px(8))");
    summary("title",`${readingType(clean ? 13 : 14,20)}.text_color(${clean ? strongest : cs("icon")})${clean ? ".whitespace_nowrap().text_ellipsis()" : ""}`);
    summary("period",`${readingType(clean ? 12 : 13,clean ? 16 : 20)}.text_color(${quietText})`);
    summary("list",`style().background(${productPanel}).radius(px(${clean ? 0 : 16})).border(px(${clean ? 0 : 1})).border_color(${productEdge})${shadows(clean ? [] : [shadow(0,1,2,0,black(dark ? .4 : .06))])}`);
    summary("row",`style().min_height(px(${clean ? 48 : 52})).gap(px(16)).padding_x(px(${clean ? 16 : 20})).border_top(px(${clean ? .5 : 0})).border_color(${productLine}).hover(style().background(${productWash}))`);
    summary("row_divided",`style().border_top(px(${clean ? .5 : 1})).border_color(${productLine})`);
    summary("row_narrow",`style().items_start().gap(px(4)).padding_x(px(${clean ? 14 : 16})).padding_y(px(${clean ? 10 : 11}))`);
    summary("label",`${readingType(clean ? 13 : 14,20)}.text_color(${ink})`);
    summary("text",`${readingType(clean ? 13 : 14,20)}.text_color(${secondaryText})`);
    summary("trend",`style().radius(px(${clean ? 5 : 6}))`);
    summary("good",`style().text_color(${cs("good")}).background(${clean ? `alpha(rgb(0x${dark ? "7dcf9f" : "5fc88a"}),${dark ? 0.14 : 0.08})` : dark ? "alpha(rgb(0x7dcf9f),0.18)" : rgba("eef9f3")})`);
    summary("bad",`style().text_color(${cs("bad")}).background(${clean ? `alpha(${cs("bad")},${dark ? 0.14 : 0.08})` : dark ? "alpha(rgb(0xff6b5f),0.16)" : rgba("fff0f0")})`);
    summary("neutral",`style().text_color(${clean ? cs("icon") : rgba("8f8f8f")}).background(${clean ? (dark ? white(.04) : black(.04)) : rgba(dark ? "2a2a2d" : "f5f5f5")})`);
    const tips=(part,value)=>set("patterns/tips_panel",part,value);
    const done=color("tips_panel.done");
    tips("root",`style().padding(px(${clean ? 0 : 6})).radius(px(${clean ? 10 : 22})).background(${clean ? productPanel : productTray})${shadows(clean ? smallShadow : [])}`);
    tips("root_no_footer",`style().padding_bottom(px(${clean ? 0 : 8}))`);
    tips("head",`style().padding_x(px(${clean ? 16 : 20})).padding_top(px(${clean ? 12 : 13})).padding_bottom(px(12)).border_bottom(px(${clean ? .5 : 0})).border_color(${productLine})${clean ? "" : ".items_baseline()"}`);
    tips("title",`${readingType(clean ? 15 : 16,clean ? 24 : 22)}.font_weight(${clean ? 500 : 600}).letter_spacing_em(${clean ? 0 : -0.01}).text_color(${strongest})`);
    tips("count",`${readingType(clean ? 12 : 13,clean ? 16 : 20)}.text_color(${quietText})${clean ? ".whitespace_nowrap()" : ""}`);
    tips("body",`style().max_height(px(420)).radius(px(${clean ? 0 : 16})).background(${productPanel}).border(px(${clean ? 0 : 1})).border_color(${productEdge})${shadows(clean ? [] : [shadow(0,1,2,0,black(dark ? .4 : .06))])}`);
    tips("grid",`style().padding(px(${clean ? 12 : 14})).gap(px(${clean ? 8 : 12}))`);
    tips("tip",`style().padding_x(px(${clean ? 12 : 16})).padding_top(px(${clean ? 12 : 16})).padding_bottom(px(${clean ? 12 : 14})).border(px(${clean ? 0 : 1})).border_color(${productLine}).radius(px(${clean ? 8 : 14}))${shadows(clean ? [shadow(0,0,0,.5,productLine,true)] : [])}`);
    tips("tip_done",`style().background(${clean ? (dark ? white(.03) : black(.015)) : productWash})`);
    tips("tip_title",`${readingType(clean ? 13 : 14,20)}.text_color(${strongest})`);
    tips("description",`${readingType(13,20)}.margin_top(px(${clean ? 2 : 6})).margin_bottom(px(${clean ? 12 : 14})).text_color(${secondaryText})`);
    tips("action",`style().min_height(px(${clean ? 28 : 30})).padding_x(px(${clean ? 10 : 12})).padding_y(px(${clean ? 4 : 5})).radius(px(${clean ? 6 : 9}))`);
    tips("action_done",`style().padding_x(px(0)).text_color(${done}).background(${transparent}).disabled(style().opacity(1).text_color(${done}).background(${transparent})${shadows([])})${shadows([])}`);
    tips("mark",`style().text_color(${done})`);
    tips("footer",`style().justify_${clean ? "end" : "start"}().padding_x(px(${clean ? 8 : 10})).padding_top(px(8)).padding_bottom(px(${clean ? 8 : 10})).border_top(px(${clean ? .5 : 0})).border_color(${productLine})`);
    tips("close",`style().min_height(px(${clean ? 28 : 30})).radius(px(${clean ? 6 : 9}))`);
    for(const name of ["project_card","experiment_card"]){
      const paint=(part,value)=>set("patterns/"+name,part,value);
      const experiment=name==="experiment_card";
      paint("root",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 10 : 20})).background(${clean ? productPanel : breakdownTray})`);
      paint("card",`style().radius(px(${clean ? 10 : 16})).background(${productPanel})${shadows(smallShadow)}`);
      paint("head",`style().padding_x(px(12)).padding_y(px(${clean ? 12 : 10})).gap(px(10))`);
      paint("title",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(${clean ? 500 : 600}).text_color(${strongest})`);
      paint("sub","style().gap(px(6))");
      paint("status",`style().min_height(px(20)).padding_x(px(6)).radius(px(${clean ? 5 : 6})).font_size(px(12)).line_height(px(16)).font_weight(500)`);
      paint("foot",`style().min_height(px(${clean ? 44 : 38})).padding_start(px(${clean ? 12 : 10})).padding_end(px(${clean ? 8 : 4})).border_top(px(${clean ? 0.5 : 0})).border_color(${productLine})`);
      paint("caption",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(16)).text_color(${cb("muted")})`);
      paint("action",`style().min_height(px(${clean ? 28 : 26})).radius(px(${clean ? 6 : 10}))`);
      if(experiment){
        const experimentCard = color("experiment_card.card");
        paint("root",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 0 : 20})).background(${color("experiment_card.tray")})`);
        paint("paper", clean ? `style().radius(px(10)).background(${experimentCard})${shadows(smallShadow)}` : "style()");
        paint("card", clean ? `style().radius(px(0)).background(${transparent})${shadows([])}`
          : `style().radius(px(16)).background(${experimentCard})${shadows(smallShadow)}`);
        paint("title",`style().font_size(px(${clean ? 13 : 13.5})).line_height(px(20)).font_weight(${clean ? 500 : 600}).text_color(${strongest})`);
        paint("sub",`style().gap(px(6)).min_height(px(20)).margin_top(px(${clean ? 0 : 1}))`);
        paint("status",`style().min_height(px(20)).padding_start(px(5)).padding_end(px(${clean ? 6 : 7})).gap(px(${clean ? 5 : 4})).radius(px(5)).font_size(px(12)).line_height(px(${clean ? 16 : 20})).font_weight(500)`);
        paint("head_narrow","style().padding_x(px(10)).padding_y(px(10)).gap(px(8))");
        paint("chart",`style().width(px(34)).height(px(34)).radius(px(${clean ? 7 : 9})).padding(px(4)).gap(px(2.5)).background(${color("experiment_card.track")})`);
        paint("column","style().radius(px(2))");
        paint("note",`style().font_size(px(12)).line_height(px(${clean ? 16 : 20})).text_color(${color("experiment_card.muted")})`);
        paint("lead",`style().font_size(px(12)).line_height(px(16)).font_weight(500).min_height(px(24)).gap(px(4)).padding_x(px(${clean ? 7 : 8})).radius(px(6)).font_feature("tnum",1)`);
        paint("lead_idle","style()");
        paint("switch","style().margin_bottom(px(8))");
        paint("row",`style().padding_x(px(6)).padding_y(px(${clean ? 7 : 8})).gap(px(6)).radius(px(${clean ? 7 : 10}))`);
        paint("row_hot",`style().background(${productWash})`);
        paint("row_picked",`style().background(${breakdownWell})${shadows([shadow(0,0,0,clean ? .5 : 1,productLine,true)])}`);
        paint("badge",`style().min_width(px(20)).min_height(px(20)).radius(px(${clean ? 5 : 6})).font_size(px(11))`);
        paint("label",`style().font_size(px(13)).line_height(px(20)).text_color(${secondaryText})`);
        paint("value",`style().font_size(px(12.5)).line_height(px(16)).font_weight(500).text_color(${ink})`);
        paint("lift",`style().min_height(px(20)).radius(px(${clean ? 4 : 5})).font_size(px(11.5)).line_height(px(16)).font_weight(500)`);
        paint("track",`style().height(px(4)).background(${color("experiment_card.track")})`);
      }else{
        const projectCard=color("project_card.card");
        paint("root",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 0 : 20})).background(${color("project_card.tray")})`);
        paint("paper",clean ? `style().radius(px(10)).background(${projectCard})${shadows(smallShadow)}` : "style()");
        paint("card",clean ? `style().radius(px(0)).background(${transparent})${shadows([])}`
          : `style().radius(px(16)).background(${projectCard})${shadows(smallShadow)}`);
        paint("title",`style().font_size(px(${clean ? 13 : 13.5})).line_height(px(20)).font_weight(${clean ? 500 : 600}).text_color(${strongest})`);
        paint("caption",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(16)).text_color(${color("project_card.muted")})`);
        paint("ring","style().width(px(32)).height(px(32))");
        paint("number",`style().font_size(px(12)).line_height(px(16)).font_weight(500).text_color(${strongest})`);
        paint("sub",`style().gap(px(6)).min_height(px(20)).margin_top(px(${clean ? 0 : 1}))`);
        paint("status",`style().min_height(px(20)).padding_start(px(5)).padding_end(px(${clean ? 6 : 7})).gap(px(${clean ? 5 : 4})).radius(px(5))`);
        if (clean) {
          const edge=shadows([shadow(0,0,0,.5,"alpha(current_color(),0.35)",true)]);
          paint("status_live",`style().hover(style()${edge})`);
          paint("status_open",`style()${edge}`);
        }
        paint("status_text",`${readingType(12,clean ? 16 : 20)}.font_weight(500)`);
        paint("due",readingType(12,clean ? 16 : 20));
        paint("people","style().margin_start(offset_px(-3))");
        paint("face",`style().width(px(22)).height(px(22)).border(px(1)).border_color(${projectCard})`);
        paint("face_active",`style().margin_start(px(0)).margin_end(px(1))`);
        paint("assignee","style().width(px(18)).height(px(18))");
        paint("picker","style().padding_x(px(12)).padding_y(px(6)).gap(px(5))");
        const optionEdge=clean ? shadows([shadow(0,0,0,.5,color("project_card.option_line"),true)]) : "";
        paint("option",`style().min_height(px(24)).gap(px(5)).radius(px(6)).padding_start(px(${clean ? 6 : 7})).padding_end(px(${clean ? 7 : 9})).font_size(px(12)).line_height(px(16))${optionEdge}`);
        if (!clean)paint("option_selected",`style()${shadows([shadow(0,0,0,1,"alpha(current_color(),0.34)",true)])}`);
        paint("tasks",`style().padding_x(px(8)).padding_y(px(${clean ? 6 : 8})).border_top(px(${clean ? .5 : 1})).border_color(${productLine})`);
        paint("task",`style().min_height(px(${clean ? 32 : 30})).gap(px(${clean ? 10 : 9})).radius(px(${clean ? 6 : 8}))`);
        paint("task_hot",`style().background(${productWash})`);
        paint("task_done",`style().text_color(${quietText})`);
        paint("label","style().font_size(px(13)).line_height(px(20))");
        paint("check_done",`style().background(${color("project_card.accent")}).border_color(${transparent}).text_color(rgb(0xffffff))`);
      }
    }
    const notice=(part,value)=>set("patterns/notification_stack",part,value);
    notice("root","style()");
    const noticeShadow=clean ? smallShadow : floatingCast(black(dark ? .20 : .08));
    for(const part of ["panel","rear"]){
      const bg=part==="panel" ? productPanel : color("notification_stack.peek");
      notice(part,`style().radius(px(${clean ? 12 : 20})).background(${bg}).border(px(${clean ? 0 : 1})).border_color(${productEdge})${shadows(noticeShadow)}`);
    }
    notice("rear_far","style().opacity(0.83)");
    notice("body",`style().padding_top(px(${clean ? 12 : 18})).padding_bottom(px(${clean ? 16 : 20})).padding_start(px(${clean ? 16 : 20})).padding_end(px(${clean ? 12 : 20})).gap(px(${clean ? 2 : 6}))`);
    notice("top",`style().min_height(px(28)).margin_bottom(px(${clean ? 6 : 8}))`);
    notice("badge",`style().font_size(px(${clean ? 10 : 11})).line_height(px(${clean ? 16 : 18})).font_weight(500).letter_spacing_em(${clean ? .05 : .06}).padding_x(px(${clean ? 6 : 9})).padding_y(px(2)).border(px(${clean ? 0 : 1})).radius(px(${clean ? 5 : 99})).background(${color("notification_stack.badge")}).text_color(${clean ? secondaryText : rgba(dark ? "c4c4c4" : "525252")})`);
    notice("title",`style().font_size(px(${clean ? 15 : 16})).line_height(px(${clean ? 24 : 22})).font_weight(500).letter_spacing_em(${clean ? 0 : -.01}).text_color(${strongest})`);
    notice("text",`style().font_size(px(${clean ? 13 : 14})).line_height(px(${clean ? 20 : 21})).text_color(${secondaryText})`);
    notice("dismiss",`style().width(px(28)).height(px(28)).top(px(${clean ? 12 : 16})).inset_end(px(${clean ? 12 : 16})).radius(px(${clean ? 6 : 8})).hover(style().background(${productWash}).text_color(${ink}))`);
    notice("action",`style().margin_top(px(${clean ? 12 : 10})).min_height(px(${clean ? 28 : 32})).radius(px(${clean ? 6 : 9}))`);
    const away=(part,value)=>set("patterns/away_summary",part,value);
    const awayMuted=color("away_summary.muted");
    const awayText=color("away_summary.text");
    const awayTray=color("away_summary.tray");
    const awayPaper=color("away_summary.card");
    const awayFill=clean ? (dark ? white(.07) : black(.05)) : rgba(dark ? "2c2c30" : "ebebeb");
    const awayRowHover=clean ? (dark ? white(.035) : black(.025)) : rgba(dark ? "28282b" : "f7f7f7");
    away("root","style()");
    away("card",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 12 : 16})).background(${clean ? awayPaper : awayTray})${shadows(clean ? smallShadow : [])}`);
    away("rows",`style().padding(px(4)).radius(px(${clean ? 0 : 12})).background(${awayPaper})${shadows(clean ? [] : smallShadow)}`);
    away("head",`style().items_center().padding_start(px(${clean ? 14 : 10})).padding_end(px(${clean ? 14 : 10})).padding_top(px(${clean ? 10 : 6})).padding_bottom(px(${clean ? 9 : 8})).border_bottom(px(${clean ? .5 : 0})).border_color(${productLine})`);
    away("head_narrow",`style().items_start().padding_x(px(${clean ? 12 : 8})).gap(px(3))`);
    away("title",`style().font_size(px(13)).line_height(px(20)).font_weight(500).text_color(${strongest})`);
    away("when",`${readingType(13,20)}.font_weight(400).font_feature("tnum",1).text_color(${awayMuted})`);
    away("count",`${readingType(12,16)}.font_feature("tnum",1).text_color(${awayMuted})`);
    away("row",`style().min_height(px(48)).gap(px(10)).padding_start(px(10)).padding_end(px(8)).radius(px(${clean ? 6 : 8})).hover(style().background(${awayRowHover}))`);
    away("label",`${readingType(13,20)}.font_weight(500).text_color(${ink})`);
    away("detail",`${readingType(12,16)}.text_color(${awayMuted})`);
    const awayActionType=`${readingType(12,16)}.font_weight(500).text_color(${awayText})`;
    away("action",`${awayActionType}.min_height(px(26)).padding_x(px(10)).border(px(0)).radius(px(6)).hover(style().background(${awayRowHover}).text_color(${ink}))`);
    away("action_needs",`style().background(${awayPaper}).text_color(${ink})${shadows(smallShadow)}`);
    away("foot",`style().gap_x(px(4)).padding_x(px(${clean ? 6 : 2})).padding_top(px(4)).padding_bottom(px(${clean ? 6 : 0})).border_top(px(${clean ? .5 : 0})).border_color(${productLine})`);
    away("divider",`style().border_top(px(0)).gap(px(8))`);
    // Own the pair's hit height here; inheriting Button/sm splits it by density.
    const awayFooterControl = `${awayActionType}.min_height(px(28)).padding_x(px(8)).border(px(0)).radius(px(6)).hover(style().background(${awayFill}).text_color(${ink}))`;
    away("more",`${awayFooterControl}.gap_x(px(4))`);
    away("toggle",`${awayFooterControl}.gap_x(px(4))`);
    away("toggle_folded",`style().gap_x(px(6)).padding_start(px(10)).padding_end(px(8)).radius(px(14)).font_weight(400).background(${clean ? transparent : awayTray}).text_color(${awayMuted})`);
    away("rule",`style().background(${productLine})`);
    const kpi=(part,value)=>set("patterns/kpi_chart",part,value);
    const kpiLift=clean ? smallShadow : floatingCast(black(dark ? .18 : .08));
    kpi("root",`style().radius(px(${clean ? 10 : 20})).background(${productPanel}).border(px(${clean ? 0 : 1})).border_color(${productEdge})${shadows(kpiLift)}`);
    kpi("body",`style().padding_x(px(${clean ? 16 : 20})).padding_top(px(${clean ? 14 : 20})).padding_bottom(px(${clean ? 12 : 4})).gap(px(0))`);
    kpi("title",`style().font_size(px(${clean ? 15 : 14})).line_height(px(${clean ? 24 : 20})).font_weight(${clean ? 500 : 400}).text_color(${clean ? strongest : secondaryText})`);
    kpi("headline",`style().margin_top(px(${clean ? 6 : 8})).gap(px(${clean ? 8 : 10}))`);
    kpi("value",`style().font_size(px(${clean ? 24 : 28})).line_height(px(${clean ? 32 : 34})).font_weight(500).letter_spacing_em(-0.02).text_color(${strongest})`);
    kpi("delta",`style().font_size(px(${clean ? 12 : 14})).line_height(px(${clean ? 16 : 20})).font_weight(${clean ? 500 : 450}).letter_spacing_em(0)${clean ? ".padding_x(px(6)).padding_y(px(2)).radius(px(5))" : ""}`);
    if (clean) {
      kpi("delta_up",`style().background(alpha(${color("success")},0.08))`);
      kpi("delta_down",`style().background(alpha(${color("danger")},0.08))`);
    } else {
      kpi("delta_up","style()");
      kpi("delta_down","style()");
    }
    kpi("chart",`style().margin_top(px(${clean ? 19 : 24})).gap(px(12))`);
    kpi("tick",`style().font_size(px(12)).line_height(px(18)).text_color(${muted})`);
    kpi("tooltip",`style().radius(px(${clean ? 8 : 12})).padding_x(px(12)).padding_y(px(10)).background(${panel}).border(px(${clean ? 0 : 1})).border_color(${edge})${shadows(clean ? floatingShadow : floatingCast())}`);
    kpi("tooltip_title",`style().font_size(px(13)).line_height(px(18)).font_weight(500).text_color(${strongest})`);
    kpi("tooltip_label",`style().font_size(px(12)).line_height(px(18)).text_color(${secondaryText})`);
    kpi("tooltip_value",`style().font_size(px(12)).line_height(px(18)).font_weight(500).text_color(${ink})`);
    kpi("footer",`style().margin_top(px(${clean ? 0 : 12})).padding_x(px(${clean ? 16 : 20})).padding_top(px(${clean ? 10 : 0})).padding_bottom(px(${clean ? 10 : 18})).border_top(px(${clean ? .5 : 0})).border_color(${edge}).font_size(px(${clean ? 12 : 13})).line_height(px(${clean ? 16 : 18})).text_color(${muted})`);
    kpi("body_narrow",`style().padding_x(px(${clean ? 12 : 14})).padding_top(px(${clean ? 12 : 16})).padding_bottom(px(${clean ? 10 : 4}))`);
    kpi("chart_narrow","style().gap(px(8))");
    kpi("plot_narrow","style().height(px(112))");
    kpi("y_axis_narrow","style().height(px(112))");
    kpi("footer_narrow",`style().padding_x(px(${clean ? 12 : 14}))`);
    const breakdownPlot=(part,value)=>set("patterns/breakdown_chart",part,value);
    breakdownPlot("root",clean ? `style().padding(px(0)).radius(px(10)).background(${productPanel})${shadows(smallShadow)}`
      : `style().padding(px(6)).radius(px(22)).background(${color("cite.tray")})`);
    breakdownPlot("heading",`style().padding_x(px(${clean ? 16 : 18})).padding_top(px(${clean ? 14 : 11})).padding_bottom(px(${clean ? 0 : 10}))`);
    if (clean) breakdownPlot("heading_narrow","style().padding_x(px(12)).padding_top(px(12))");
    breakdownPlot("title",`style().font_size(px(${clean ? 15 : 14})).line_height(px(${clean ? 24 : 20})).font_weight(500).text_color(${strongest})`);
    breakdownPlot("total",`${readingType(15,24)}.font_weight(${clean ? 500 : 600}).text_color(${strongest})`);
    breakdownPlot("unit",`${readingType(clean ? 12 : 13,clean ? 16 : 20)}.text_color(${color("breakdown_chart.muted")})`);
    breakdownPlot("panel",clean ? `style().background(${productPanel}).radius(px(0)).border(px(0)).border_color(${edge}).shadows([])`
      : `style().background(${productPanel}).radius(px(16)).border(px(1)).border_color(${edge})${shadows([shadow(0,1,2,0,black(dark ? .22 : .06))])}`);
    breakdownPlot("controls",`style().gap(px(${clean ? 8 : 10})).padding_x(px(${clean ? 16 : 18})).padding_top(px(${clean ? 12 : 16})).padding_bottom(px(${clean ? 0 : 4}))`);
    breakdownPlot("chart",`style().padding_x(px(${clean ? 16 : 18})).padding_top(px(${clean ? 20 : 22})).padding_bottom(px(${clean ? 10 : 12}))`);
    breakdownPlot("plot_frame",`style().height(px(${clean ? 220 : 224}))`);
    breakdownPlot("axis","style().width(px(36))");
    breakdownPlot("tick",`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    breakdownPlot("bar",`style().width(px(${clean ? 12 : 10})).radius(px(${clean ? 3 : 0}))`);
    breakdownPlot("gridline",clean ? `style().height(px(0.5)).background(${color("breakdown_chart.grid")}).border_top(px(0)).border_color(${color("breakdown_chart.grid")}).border_solid()`
      : `style().height(px(1)).background(${transparent}).border_top(px(1)).border_color(${color("breakdown_chart.grid")}).border_dashed()`);
    breakdownPlot("zero",`style().background(${color("breakdown_chart.zero")}).border_color(${color("breakdown_chart.zero")}).border_solid()`);
    breakdownPlot("x_axis",`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    breakdownPlot("x_active",`style().font_weight(500).text_color(${strongest})`);
    breakdownPlot("table_header",`style().height(px(${clean ? 32 : 34})).border_top(px(${clean ? 0.5 : 1})).border_color(${edge}).font_weight(500).background(${clean ? productPanel : (dark ? rgba("1f1f22") : rgba("fcfcfc"))})`);
    breakdownPlot("table_row",`style().height(px(36)).border_top(px(${clean ? 0.5 : 1})).border_color(${edge})`);
    breakdownPlot("row_active",`style().background(${productWash})`);
    breakdownPlot("cell",`style().font_size(px(13)).line_height(px(${clean ? 20 : 18})).text_color(${secondaryText})`);
    breakdownPlot("cell_header",`style().font_size(px(${clean ? 10 : 12})).line_height(px(16)).font_weight(500).text_color(${muted})`);
    breakdownPlot("cell_total",`style().text_color(${strongest})`);
    breakdownPlot("cell_muted","style().opacity(0.45)");
    breakdownPlot("swatch","style().radius(px(2))");
    breakdownPlot("tooltip",`style().radius(px(${clean ? 8 : 12})).padding(px(10)).background(${panel}).border(px(${clean ? 0 : 1})).border_color(${edge})${shadows(smallShadow)}`);
    breakdownPlot("tooltip_title",`style().font_size(px(13)).line_height(px(18)).font_weight(500).text_color(${strongest})`);
    breakdownPlot("footer",`style().padding_x(px(${clean ? 8 : 10})).padding_top(px(${clean ? 8 : 6})).padding_bottom(px(${clean ? 8 : 7})).gap(px(${clean ? 8 : 10})).border_top(px(${clean ? 0.5 : 0})).border_color(${edge})`);
    if (clean) {
        breakdownPlot("footer_narrow","style().padding(px(6))");
        breakdownPlot("legend_narrow","style().gap(px(0))");
        breakdownPlot("legend_chip_narrow","style().gap(px(5)).padding_x(px(6))");
    }
    const legendFocus=`.focus_ring(px(2),offset_px(${clean ? -2 : 2}),theme_color("focus_ring"))`;
    breakdownPlot("legend_chip",`style().padding_x(px(${clean ? 8 : 9})).radius(px(${clean ? 6 : 8})).font_size(px(${clean ? 12 : 12.5})).line_height(px(${clean ? 16 : 20})).font_weight(${clean ? 500 : 400}).text_color(${clean ? muted : strongest}).hover(style().background(${productWash})${clean ? `.text_color(${strongest})` : ""})${legendFocus}`);
    breakdownPlot("legend_hidden","style().isolated_opacity(true)");
    breakdownPlot("legend_hint",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(${clean ? 16 : 20})).text_color(${muted})`);
    breakdownPlot("legend_reset",`style().radius(px(${clean ? 6 : 8})).font_size(px(${clean ? 13 : 12.5})).line_height(px(20)).text_color(${clean ? strongest : color("breakdown_chart.c1")}).hover(style().background(${clean ? productWash : `alpha(${color("breakdown_chart.c1")},0.08)`}))${legendFocus}`);
    const now=(part,value)=>set("patterns/running_now",part,value);
    const nowPaper=color("running_now.paper");
    const nowHover=rgba(dark ? (clean ? "252527" : "222226") : (clean ? "fafafa" : "e8e8e8"));
    now("root","style()");
    now("trigger",`style().min_height(px(${clean ? 28 : 30})).gap(px(8)).padding_start(px(${clean ? 10 : 11})).padding_end(px(${clean ? 8 : 9})).radius(px(${clean ? 6 : 8})).background(${nowPaper})${shadows(clean ? smallShadow : [])}.hover(style().background(${nowHover}))`);
    now("trigger_open",`style().background(${clean ? nowPaper : nowHover})`);
    now("part","style().gap(px(4))");
    now("count",`${readingType(12,16)}.font_weight(500).text_color(${ink})`);
    now("trigger_caption",readingType(12,16));
    now("separator",readingType(12,16));
    now("caption",`style().font_size(px(12)).line_height(px(16)).text_color(${muted})`);
    const nowIcon=clean ? (dark ? white(.46) : black(.447)) : rgba(dark ? "8a8a90" : "8a8a8a");
    now("chevron",`style().width(px(12)).height(px(12)).text_color(${nowIcon})`);
    now("panel",`style().width(px(${clean ? 340 : 348})).padding(px(4)).radius(px(${clean ? 10 : 16})).background(${nowPaper})${shadows(clean ? smallShadow : floatingShadow)}`);
    now("heading",`style().min_height(px(${clean ? 28 : 32})).padding_x(px(${clean ? 8 : 10})).gap(px(6))`);
    now("label",`style().font_size(px(${clean ? 10 : 13})).line_height(px(${clean ? 16 : 20})).font_weight(500).letter_spacing_em(${clean ? 0.05 : 0}).text_color(${clean ? muted : strongest})`);
    now("body",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 0 : 12})).background(${productPanel})${shadows(clean ? [] : smallShadow)}`);
    now("row",`style().height(px(52)).radius(px(${clean ? 6 : 8})).hover(style().background(${productWash}))`);
    now("main",`style().padding_start(px(9)).padding_end(px(3)).gap(px(10)).radius(px(${clean ? 6 : 8}))`);
    now("name",`style().font_size(px(13)).line_height(px(20)).font_weight(500).text_color(${strongest})`);
    now("finished",`style().text_color(${secondaryText})`);
    now("step",`style().font_size(px(12)).line_height(px(16)).text_color(${muted})`);
    now("ring","style().width(px(14)).height(px(14))");
    now("badge",`style().font_size(px(${clean ? 11 : 11.5})).line_height(px(${clean ? 18 : 20})).font_weight(500).padding_y(px(0)).padding_x(px(6)).radius(px(5))`);
    now("action",`style().height(px(28)).radius(px(${clean ? 6 : 7})).hover(style().background(${productWash}).text_color(${ink}))`);
    now("action_text",`style().background(${clean ? productPanel : productWash})${shadows(clean ? smallShadow : [])}`);
    now("action_label",`style().font_size(px(12)).line_height(px(16)).text_color(${strongest})`);
    now("done_mark",`style().text_color(${color("running_now.blue")})`);
    now("footer",`style().min_height(px(32)).padding_x(px(9)).margin_top(px(4)).border_top(px(${clean ? 0.5 : 0})).border_color(${edge}).radius(px(${clean ? 0 : 8}))`);
    const scheduleBoard=(part,value)=>set("patterns/scheduled_runs",part,value);
    const scheduleCard=color("scheduled_runs.card");
    const scheduleInk=color("scheduled_runs.ink");
    const scheduleTray=rgba(dark ? "141416" : "f0f0f0");
    scheduleBoard("root",clean ? `style().padding(px(0)).radius(px(12)).background(${transparent}).shadows([])`
      : `style().padding(px(6)).radius(px(22)).background(${scheduleTray}).shadows([])`);
    scheduleBoard("heading",`style().padding_x(px(${clean ? 16 : 12})).padding_top(px(${clean ? 13 : 9})).padding_bottom(px(${clean ? 8 : 9})).gap(px(8))`);
    scheduleBoard("title",`${readingType(15,24)}.font_weight(${clean ? 500 : 600}).text_color(${strongest})`);
    scheduleBoard("note",`${readingType(clean ? 12 : 12.5,20)}.text_color(${color("scheduled_runs.muted")})`);
    scheduleBoard("panel",clean ? `style().padding(px(8)).gap(px(8)).radius(px(10)).border(px(0)).border_color(${edge}).shadows([]).background(${transparent})`
      : `style().padding(px(8)).gap(px(8)).radius(px(16)).border(px(1)).border_color(${edge})${shadows(smallShadow)}.background(${scheduleCard})`);
    scheduleBoard("stack",`style().background(${color("scheduled_runs.lane")}).radius(px(${clean ? 14 : 16})).padding(px(4))`);
    scheduleBoard("stack_head","style().height(px(30)).min_height(px(30)).padding_start(px(8)).padding_end(px(6))");
    scheduleBoard("stack_count",`${readingType(11,16)}.padding_x(px(5)).radius(px(6)).background(${color("scheduled_runs.count_bg")}).text_color(${color("scheduled_runs.text")})`);
    scheduleBoard("stack_header",`${readingType(11.5,16)}.font_weight(500).text_color(${color("scheduled_runs.muted")})`);
    scheduleBoard("counter",`style().font_size(px(11)).line_height(px(16)).text_color(${color("scheduled_runs.muted")}).font_feature("tnum",1)`);
    scheduleBoard("agent",`style().height(px(26)).min_width(px(26)).padding_x(px(5)).radius(px(7)).border(px(0)).background(${scheduleCard})${shadows(smallShadow)}`);
    scheduleBoard("agent_active",`style().background(${scheduleCard})`);
    scheduleBoard("agent_mark",`${readingType(10,16)}.font_weight(500).text_color(${scheduleInk})`);
    scheduleBoard("agent_name",`${readingType(12,16)}.font_weight(500).text_color(${scheduleInk})`);
    // Both labels share a line height. GPUI's measured text leaves don't give
    // Taffy a text baseline, so use the line-box inset rather than flex baselines.
    scheduleBoard("day",`style().height(px(26)).items_start().padding_top(px(5)).gap_x(px(4)).radius(px(8))`);
    scheduleBoard("day_caption",`${readingType(clean ? 10 : 10.5,16)}.typography("label").font_weight(500).letter_spacing_px(0.5)`);
    scheduleBoard("day_date",`${readingType(12,16)}.font_weight(500)`);
    scheduleBoard("day_current",`style().background(${color("scheduled_runs.well")})`);
    scheduleBoard("time",`style().font_size(px(11)).line_height(px(16)).text_color(${color("scheduled_runs.faint")}).font_feature("tnum",1)`);
    scheduleBoard("cell",`style().height(px(156)).padding(px(4)).radius(px(${clean ? 10 : 12})).border(px(${clean ? 0.5 : 1})).border_color(${edge})`);
    scheduleBoard("past",`style().background(${transparent})`);
    scheduleBoard("target",`style().background(${color("scheduled_runs.drop")}).border_color(${focus})`);
    scheduleBoard("subdued","style().isolated_opacity(true).opacity(0.45)");
    scheduleBoard("bar_fill_past","style().opacity(0.50)");
    scheduleBoard("task",`style().height(px(64)).border(px(${clean ? 0 : 1})).border_color(${edge}).radius(px(${clean ? 7 : 10})).background(${scheduleCard})${shadows(smallShadow)}`);
    scheduleBoard("task_title",`${readingType(clean ? 11.5 : 12,15)}.font_weight(500).text_color(${scheduleInk})`);
    scheduleBoard("task_meta",`${readingType(clean ? 10.5 : 11.5,14)}.text_color(${color("scheduled_runs.muted")})`);
    scheduleBoard("task_metrics","style().height(px(16)).gap(px(10))");
    scheduleBoard("task_stat",`${readingType(clean ? 10.5 : 11.5,16)}.text_color(${color("scheduled_runs.muted")})`);
    scheduleBoard("usage_badge",`style().padding_x(px(5)).radius(px(${clean ? 4 : 999})).background(${color("scheduled_runs.pill")})`);
    scheduleBoard("usage_label",`${readingType(10,15)}.text_color(${color("scheduled_runs.text")})`);
    scheduleBoard("task_part",`${readingType(10,15)}.text_color(${color("scheduled_runs.muted")})`);
    scheduleBoard("task_mark",`${readingType(9,16)}.font_weight(500).text_color(${scheduleInk})`);
    scheduleBoard("task_mark_box",`style().width(px(18)).height(px(18)).radius(px(4)).background(${scheduleCard})${shadows(smallShadow)}`);
    scheduleBoard("bar","style().height(px(3))");
    const modeRail=dark ? white(.07) : black(clean ? .05 : .06);
    const modeHeight=clean ? 24 : 26;
    scheduleBoard("mode_track",`style().padding(px(${clean ? 2 : 3})).gap(px(0)).radius(px(${clean ? 8 : 10})).background(${modeRail})`);
    scheduleBoard("mode",`${readingType(clean ? 12.5 : 13,20)}.height(px(${modeHeight})).min_height(px(${modeHeight})).gap(px(5)).padding_x(px(${clean ? 10 : 11}))
      .radius(px(${clean ? 6 : 7})).border(px(0)).font_weight(500).background(${transparent})
      .text_color(${color("scheduled_runs.text")}).shadows([])`);
    scheduleBoard("mode_active",`style().background(${scheduleCard}).text_color(${scheduleInk})${shadows(smallShadow)}`);
    scheduleBoard("detail",`style().radius(px(${clean ? 10 : 14})).background(${scheduleCard})${shadows(smallShadow)}`);
    scheduleBoard("footer","style().padding(px(8))");
    const toolCard=(part,value)=>set("patterns/tool_call_card",part,value);
    toolCard("root",`style().radius(px(${clean ? 10 : 12})).background(${productPanel}).border(px(${clean ? 0 : 1})).border_color(${edge})${shadows(smallShadow)}`);
    toolCard("header",`style().min_height(px(44)).padding_x(px(${clean ? 14 : 9})).padding_y(px(${clean ? 0 : 8})).gap(px(10)).radius(px(${clean ? 10 : 12})).hover(style().background(${productWash}))`);
    toolCard("header_open",`style().radius_top_left(px(${clean ? 10 : 12})).radius_top_right(px(${clean ? 10 : 12})).radius_bottom_left(px(0)).radius_bottom_right(px(0))`);
    toolCard("icon",`style().width(px(${clean ? 16 : 26})).height(px(${clean ? 16 : 26})).radius(px(${clean ? 0 : 7})).background(${clean ? transparent : color("tool_call.tile")}).text_color(${clean ? muted : strongest})`);
    toolCard("name",`style().typography("code").font_size(px(12)).line_height(px(18)).font_weight(500).text_color(${strongest})`);
    toolCard("summary",`style().font_size(px(12)).line_height(px(18)).text_color(${muted})`);
    toolCard("status",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    toolCard("failure",`style().text_color(${color("text.danger")})`);
    toolCard("body",`style().padding_x(px(${clean ? 14 : 12})).padding_top(px(4)).padding_bottom(px(12)).gap(px(10))`);
    toolCard("section","style().gap(px(4))");
    toolCard("label",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    toolCard("code",`style().padding_x(px(10)).padding_y(px(6)).radius(px(8)).border(px(${clean ? 0.5 : 1})).border_color(${edge}).background(${color("tool_call.code")})`);
    toolCard("copy",`style().width(px(25)).height(px(25)).radius(px(5)).text_color(${muted})`);
    toolCard("copy_done",`style().text_color(${focus})`);
    toolCard("more","style().margin_top(px(0)).radius(px(5))");
    toolCard("error",`style().padding_x(px(10)).padding_y(px(8)).radius(px(8)).border(px(${clean ? 0.5 : 1}))`);
    toolCard("error_text",`style().font_size(px(13)).line_height(px(18)).text_color(${color("text.danger")})`);
    toolCard("retry",`style().radius(px(${clean ? 6 : 8}))`);
    const toolGroup=(part,value)=>set("patterns/tool_call_group",part,value);
    toolGroup("root","style()");
    toolGroup("header",`style().min_height(px(${clean ? 32 : 36})).padding_x(px(8)).gap(px(7)).radius(px(${clean ? 8 : 10})).hover(style().background(${productWash}))`);
    toolGroup("mark",`style().width(px(${clean ? 22 : 24})).height(px(${clean ? 22 : 24})).background(${color("tool_call.tile")}).border(px(0.5)).border_color(${edge})`);
    toolGroup("title",`style().font_size(px(12.5)).line_height(px(17)).font_weight(500).text_color(${strongest})`);
    toolGroup("failed",`style().font_size(px(12)).line_height(px(16)).text_color(${color("text.danger")})`);
    toolGroup("elapsed",`style().font_size(px(12)).line_height(px(16)).text_color(${muted})`);
    toolGroup("list","style().padding_top(px(6)).gap(px(6))");
    const contextWindow=(part,value)=>set("patterns/context_window",part,value);
    contextWindow("root",clean ? `style().padding(px(0)).radius(px(10)).background(${productPanel}).border(px(0)).border_color(${edge})${shadows(smallShadow)}`
      : `style().padding(px(6)).radius(px(22)).background(${color("cite.tray")}).border(px(0)).border_color(${edge}).shadows([])`);
    contextWindow("card",`style().padding_x(px(${clean ? 16 : 14})).padding_top(px(${clean ? 14 : 12})).padding_bottom(px(8)).gap(px(10)).radius(px(${clean ? 0 : 16})).background(${productPanel}).border(px(${clean ? 0 : 1})).border_color(${edge})${shadows(clean ? [] : smallShadow)}`);
    contextWindow("head","style().min_height(px(20)).gap(px(10))");
    contextWindow("title",`style().font_size(px(13)).line_height(px(20)).font_weight(500).text_color(${strongest})`);
    contextWindow("model",`style().font_size(px(11)).line_height(px(16)).font_weight(500).padding_x(px(6)).padding_y(px(2)).radius(px(5)).background(${productWash}).text_color(${muted})`);
    const contextDisplay=`.font_size(px(24)).line_height(px(${clean ? 32 : 30})).font_weight(${clean ? 500 : 600})`;
    contextWindow("figure",clean ? "style().gap_x(px(8)).items_center()" : `style().typography("display").gap_x(px(7)).items_baseline()${contextDisplay}`);
    contextWindow("big",`style()${contextDisplay}.letter_spacing_em(-0.02).text_color(${strongest})`);
    contextWindow("of",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(${clean ? 16 : 20})).padding_top(px(${clean ? 4 : 0})).text_color(${muted})`);
    contextWindow("pill",clean ? `style().font_size(px(12)).line_height(px(20)).padding_x(px(6)).padding_y(px(0)).radius(px(5)).background(${productWash}).text_color(${ink})`
      : `style().font_size(px(12.5)).line_height(px(20)).font_weight(400).padding(px(0)).radius(px(0)).background(${transparent}).text_color(${muted})`);
    contextWindow("warn",`style().background(${clean ? color("context_window.warn_bg") : transparent}).text_color(${color("context_window.warn")})`);
    contextWindow("full",`style().background(${clean ? color("context_window.full_bg") : transparent}).text_color(${color("text.danger")})`);
    contextWindow("field","style().gap(px(2))");
    for(const part of ["tile","spark","dot"])contextWindow(part,"style().radius(px(2))");
    contextWindow("tile_active",`style().border(px(0.5)).border_color(${edge})`);
    contextWindow("active",`style().background(${productWash})`);
    contextWindow("part",`style().padding_x(px(4)).radius(px(${clean ? 6 : 8}))`);
    contextWindow("name",`style().font_size(px(13)).line_height(px(20)).text_color(${strongest})`);
    contextWindow("amount",`style().min_width(px(52)).font_size(px(13)).line_height(px(20)).text_color(${clean ? secondaryText : ink})`);
    contextWindow("compact",`style().height(px(24)).radius(px(${clean ? 5 : 8})).hover(style().background(${productWash}))`);
    contextWindow("done",`style().background(alpha(${color("context_window.saved")},0.08)).text_color(${color("context_window.saved")})`);
    contextWindow("item_name",`style().font_size(px(12)).line_height(px(16)).text_color(${secondaryText})`);
    contextWindow("item_value",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    contextWindow("footer",`style().min_height(px(${clean ? 44 : 38})).padding_start(px(${clean ? 16 : 10})).padding_end(px(${clean ? 8 : 4})).padding_top(px(${clean ? 0 : 4})).gap(px(8)).border_top(px(${clean ? 0.5 : 0})).border_color(${edge})`);
    contextWindow("note",readingType(clean ? 12 : 12.5,clean ? 16 : 20));
    contextWindow("saved",`style().text_color(${color("context_window.saved")})`);
    const contextInk=clean ? strongest : rgba(dark ? "c8c8cc" : "404040");
    const actionPaint=clean ? productPanel : transparent;
    const actionHover=clean ? rgba(dark ? "232326" : "fafafa") : productPanel;
    contextWindow("action",`${readingType(clean ? 13 : 12.5,20)}.font_weight(500).min_height(px(${clean ? 28 : 26})).padding_start(px(${clean ? 10 : 8})).padding_end(px(${clean ? 8 : 6})).gap(px(${clean ? 6 : 4})).border(px(0)).radius(px(${clean ? 6 : 10})).text_color(${contextInk}).background(${actionPaint})${shadows(clean ? smallShadow : [])}.hover(style().background(${actionHover}).text_color(${clean ? strongest : ink}))`);
    contextWindow("action_icon",clean ? `style().text_color(${dark ? white(.50) : black(.447)})` : "style()");
    const contextMap=(part,value)=>set("patterns/context_map",part,value);
    const contextPanel=color("context_map.panel");
    contextMap("root", clean
      ? `style().padding(px(0)).radius(theme_radius("lg")).background(${transparent}).letter_spacing_em(0).shadows([])`
      : `style().padding(px(4)).radius(px(20)).background(${color("context_map.tray")}).letter_spacing_em(-0.006)`);
    contextMap("field",`style().radius(theme_radius("lg")).background(${contextPanel})${shadows(clean ? [] : smallShadow)}`);
    contextMap("header",`style().padding_x(px(${clean ? 10 : 12})).padding_y(px(${clean ? 8 : 10})).gap_x(px(8)).gap_y(px(6))`);
    const contextHeaderType=readingType(clean ? 13 : 14,20);
    contextMap("title",`${contextHeaderType}.font_weight(${clean ? 500 : 600}).text_color(${strongest})`);
    contextMap("model",`${contextHeaderType}.text_color(${color("context_map.model")})`);
    contextMap("status",`style().radius(px(999)).min_height(px(24)).padding_x(px(8)).gap(px(5))`);
    contextMap("summary",`style().gap(px(${clean ? 8 : 10}))`);
    const contextSummaryType=readingType(clean ? 12 : 13,clean ? 16 : 18);
    contextMap("used",`${contextSummaryType}.font_weight(${clean ? 500 : 600}).text_color(${strongest})`);
    contextMap("number",`${contextSummaryType}.text_color(${color("context_map.muted")})`);
    const capacityShadow=clean ? [] : [
      shadow(0,0,0,1,color("context_map.capacity_edge")),
      shadow(0,1,2,0,black(dark ? .4 : .04)),
    ];
    const capacityType=readingType(clean ? 11.5 : 12.5,clean ? 20 : 24);
    contextMap("percent",`${capacityType}.font_weight(${clean ? 500 : 600}).min_width(px(${clean ? 36 : 44})).padding_x(px(${clean ? 6 : 8})).text_center().radius(px(${clean ? 5 : 999})).text_color(${color("context_map.model")}).background(${color("context_map.capacity_bg")})${shadows(capacityShadow)}`);
    contextMap("warn",`style().background(${color("context_map.capacity_warn_bg")}).text_color(${color("context_map.result_ink")})`);
    contextMap("full",`style().background(${color("context_map.capacity_full_bg")}).text_color(${color("context_map.capacity_full_ink")})`);
    contextMap("compare",`style().radius(px(999)).background(${productWash}).padding(px(1))`);
    contextMap("tile",`style().border(px(${clean ? .5 : 1})).focus(style().border_color(${color("focus_ring")}))`);
    contextMap("tile_pinned",`style().border(px(${clean ? 1 : 1.5}))`);
    contextMap("free",`style().border(px(${clean ? 1 : 1.5}))`);
    contextMap("tile_label",`style().margin_start(px(${clean ? 9 : 10})).margin_top(px(${clean ? 7 : 8}))`);
    contextMap("tile_name",`style().font_size(px(${clean ? 12 : 13})).line_height(px(${clean ? 16 : 17})).font_weight(${clean ? 500 : 600})`);
    contextMap("tile_count",`style().font_size(px(${clean ? 11.5 : 12.5})).line_height(px(${clean ? 15 : 17})).font_weight(400).font_feature("tnum",1)`);
    contextMap("tile_measures",`style().opacity(0.7)`);
    contextMap("tooltip",`style().padding_x(px(${clean ? 12 : 14})).padding_y(px(${clean ? 9 : 10})).gap(px(2)).radius(theme_radius("lg")).border(px(0))${shadows(smallShadow)}`);
    contextMap("tip_title",`style().font_size(px(14)).line_height(px(20)).font_weight(600).text_color(${strongest})`);
    contextMap("tip_meta","style().font_size(px(12.5)).line_height(px(18))");
    contextMap("tip_detail","style().font_size(px(13)).line_height(px(19))");
    contextMap("tip_fate","style().font_size(px(12)).line_height(px(16)).font_weight(500)");
    contextMap("tip_items",`style().padding_top(px(9)).margin_top(px(5)).border_top(px(0.5)).border_color(${edge})`);
    contextMap("tip_item","style().font_size(px(13)).line_height(px(20))");
    contextMap("footer",`style().padding_x(px(10)).padding_top(px(10)).padding_bottom(px(8)).gap_x(px(16)).gap_y(px(4))`);
    contextMap("fate",`${contextSummaryType}.text_color(${color("context_map.muted")})`);
    contextMap("fate_empty",`style().text_color(${color("context_map.empty")})`);
    const profileMenu=(part,value)=>set("patterns/profile_menu",part,value);
    const profileHover=clean ? (dark ? white(.08) : black(.06)) : (dark ? rgba("2a2a2d") : rgba("f5f5f5"));
    const profileActive=clean ? (dark ? white(.1) : black(.08)) : rgba(dark ? "313135" : "efefef");
    const profileLine=dark ? white(.08) : clean ? black(.06) : rgba("e5e5e5");
    const profileLift=[shadow(0,0,0,.5,profileLine),...floatingCast()];
    profileMenu("root",clean
      ? `style().max_width(px(260)).padding(px(0)).radius(theme_radius("lg")).background(${productPanel})${shadows(profileLift)}`
      : `style().max_width(px(320)).padding(px(6)).radius(px(22)).background(${color("profile_menu.tray")})`);
    profileMenu("card",clean ? `style().padding(px(4)).radius(theme_radius("lg")).background(${transparent}).border(px(0)).border_color(${transparent}).shadows([])`
      : `style().padding_x(px(8)).padding_top(px(16)).padding_bottom(px(8)).background(${productPanel}).border(px(1)).border_color(${profileLine})${shadows([shadow(0,1,2,0,black(.06))])}`);
    profileMenu("account",`style().gap(px(${clean ? 10 : 14})).padding_x(px(${clean ? 12 : 10})).padding_top(px(${clean ? 8 : 0})).padding_bottom(px(${clean ? 10 : 12})).border_bottom(px(${clean ? .5 : 0})).border_color(${profileLine})`);
    profileMenu("avatar",`style().width(px(${clean ? 32 : 42})).height(px(${clean ? 32 : 42})).radius(px(${clean ? 16 : 21}))`);
    profileMenu("name",`style().font_size(px(${clean ? 13 : 15})).line_height(px(${clean ? 18 : 22})).font_weight(500)`);
    profileMenu("email",`style().font_size(px(${clean ? 12 : 13})).line_height(px(${clean ? 16 : 20})).text_color(${muted})`);
    profileMenu("actions",`style().max_height(px(${clean ? 320 : 380})).gap(px(${clean ? 0 : 2}))`);
    profileMenu("item",`style().min_height(px(${clean ? 32 : 36})).gap(px(${clean ? 10 : 12})).padding_x(px(${clean ? 8 : 12})).radius(px(${clean ? 6 : 12})).hover(style().background(${profileHover})).focus(style().background(${profileHover})).active(style().background(${profileActive}))`);
    profileMenu("icon",`style().width(px(${clean ? 16 : 20})).height(px(${clean ? 16 : 20})).text_color(${muted})`);
    profileMenu("item_label",readingType(clean ? 13 : 14,20));
    profileMenu("item_badge",`${readingType(clean ? 11 : 12,18)}.font_weight(500).padding_x(px(${clean ? 6 : 8})).padding_y(px(${clean ? 0 : 2})).radius(px(${clean ? 5 : 999}))`);
    // Keep one full-width interactive row. Only its non-interactive siblings
    // are inset, which also scales correctly without a negative margin.
    profileMenu("setup",`style().padding_x(px(${clean ? 4 : 10})).padding_top(px(10)).padding_bottom(px(12)).gap(px(0)).border_top(px(${clean ? .5 : 0})).border_color(${profileLine})`);
    if(clean)profileMenu("setup_with_next","style().padding_bottom(px(6))");
    profileMenu("setup_head",`style().margin_x(px(8))${clean ? "" : ".items_baseline()"}`);
    profileMenu("track",`style().margin_x(px(8)).margin_top(px(${clean ? 8 : 7}))`);
    for(const part of ["setup_title","setup_count"])profileMenu(part,readingType(clean ? 12 : 13,clean ? 16 : 18));
    profileMenu("next",`style().min_height(px(${clean ? 28 : 26})).margin_top(px(6)).padding_x(px(8)).gap(px(8)).radius(px(${clean ? 6 : 8})).hover(style().background(${productWash}))`);
    for(const part of ["next_hint","next_label"])profileMenu(part,readingType(13,20));
    profileMenu("next_icon",`style().width(px(${clean ? 16 : 14})).height(px(${clean ? 16 : 14}))`);
    const emailDraft=(part,value)=>set("patterns/email_draft",part,value);
    const draftEdge=dark ? white(.08) : clean ? black(.06) : rgba("e5e5e5");
    const draftShadow=shadow(0,1,2,0,black(dark ? .24 : .06));
    emailDraft("root",clean
      ? `style().max_width(px(480)).padding(px(0)).radius(theme_radius("lg")).background(${color("email_draft.card")})${shadows(smallShadow)}`
      : `style().max_width(px(520)).padding(px(6)).radius(px(20)).background(${color("email_draft.tray")}).shadows([])`);
    emailDraft("record",`style().gap(px(2)).padding_x(px(${clean ? 16 : 10})).padding_top(px(${clean ? 12 : 8})).padding_bottom(px(${clean ? 12 : 10})).border_bottom(px(${clean ? .5 : 0})).border_color(${draftEdge})`);
    const draftRecordType=`${readingType(15,clean ? 24 : 22)}.font_weight(500).text_color(${strongest})`;
    emailDraft("reference",`${draftRecordType}${clean ? "" : ".letter_spacing_em(-0.01)"}`);
    emailDraft("amount",`${draftRecordType}.font_feature("tnum",1)`);
    emailDraft("account",readingType(clean ? 12 : 13,16));
    emailDraft("status",`${readingType(clean ? 12 : 12.5,clean ? 16 : 18)}.padding_x(px(${clean ? 6 : 8})).padding_y(px(2)).radius(px(${clean ? 5 : 8})).font_weight(500)`);
    for(const part of ["card","receipt"])emailDraft(part,clean
      ? `style().background(${transparent}).border(px(0)).border_color(${draftEdge}).radius(px(0)).shadows([])`
      : `style().background(${color("email_draft.card")}).border(px(1)).border_color(${draftEdge}).radius(theme_radius("lg"))${shadows([draftShadow])}`);
    emailDraft("meta",`style().padding_x(px(${clean ? 16 : 14})).padding_y(px(${clean ? 10 : 12})).gap(px(12)).border_bottom(px(${clean ? .5 : 1})).border_color(${draftEdge})`);
    // The Modern hover color is an sRGB mix whose stops total 86%;
    // keep that alpha on the chip rather than blending it over the paper here.
    const recipientWash=clean ? (dark ? white(.04) : black(.03)) : alphaColor(dark ? "38383a" : "e6e6e6",.86);
    const recipientEdge=dark ? white(.08) : clean ? black(.06) : rgba("ebebeb");
    const draftIconInk=clean ? (dark ? white(.50) : black(.447)) : rgba("8f8f8f");
    emailDraft("recipient",`style().min_height(px(${clean ? 28 : 26})).gap(px(${clean ? 6 : 4})).padding_start(px(${clean ? 10 : 8})).padding_end(px(6)).padding_y(px(0)).radius(px(${clean ? 6 : 8})).background(${clean ? transparent : color("email_draft.chip")})${shadows([shadow(0,0,0,clean ? .5 : 1,recipientEdge,true)])}.hover(style().background(${recipientWash}))`);
    emailDraft("recipient_open",clean ? `style().background(${recipientWash})` : "style()");
    emailDraft("recipient_arrow",`style().width(px(${clean ? 16 : 14})).height(px(${clean ? 16 : 14})).text_color(${draftIconInk})`);
    emailDraft("recipient_name",`${readingType(clean ? 13 : 16,20)}.font_weight(500)`);
    emailDraft("recipient_email",`${readingType(clean ? 13 : 16,20)}.font_weight(400)`);
    const draftFileHover=clean ? recipientWash : dark ? white(.05) : rgba("f5f5f5");
    const fileIconSize=clean ? 16 : 15;
    const removeIconSize=clean ? 14 : 13;
    const removeSize=clean ? 24 : 20;
    emailDraft("attachment",`style().gap(px(${clean ? 6 : 5})).background(${transparent})`);
    emailDraft("attachment_icon",`style().width(px(${fileIconSize})).height(px(${fileIconSize}))${clean ? `.text_color(${draftIconInk})` : ""}`);
    emailDraft("file_name",`${readingType(13,clean ? 20 : 16)}.text_color(${color("email_draft.file")})`);
    emailDraft("remove",`style().padding(px(0)).min_height(px(${removeSize})).width(px(${removeSize})).radius(px(6))${clean ? ".margin_start(offset_px(-2))" : ""}.text_color(${draftIconInk}).hover(style().background(${draftFileHover}).text_color(${ink}))`);
    emailDraft("remove_icon",`style().width(px(${removeIconSize})).height(px(${removeIconSize}))`);
    emailDraft("attach",`style().min_height(px(${clean ? 28 : 26})).gap(px(${clean ? 6 : 4})).padding_start(px(6)).padding_end(px(${clean ? 8 : 6})).padding_y(px(${clean ? 0 : 3})).radius(px(${clean ? 6 : 8})).text_color(${color("email_draft.file")}).hover(style().background(${draftFileHover}).text_color(${ink}))`);
    emailDraft("attach_label",`${readingType(clean ? 13 : 16,20)}.font_weight(400)`);
    emailDraft("body",`style().padding_x(px(${clean ? 16 : 14})).padding_top(px(${clean ? 14 : 12})).padding_bottom(px(${clean ? 2 : 4}))`);
    emailDraft("body_text",`style().font_size(px(${clean ? 13 : 15})).line_height(px(${clean ? 20 : 24}))`);
    emailDraft("editor",`style().border(px(0)).padding(px(0)).font_size(px(${clean ? 13 : 15})).line_height(px(${clean ? 20 : 24})).background(${transparent})`);
    emailDraft("link",`style().font_size(px(${clean ? 13 : 15})).line_height(px(${clean ? 20 : 24}))`);
    emailDraft("actions",`style().padding_start(px(${clean ? 16 : 14})).padding_end(px(4)).padding_y(px(${clean ? 14 : 10}))`);
    emailDraft("tones",`style().gap(px(${clean ? 4 : 2}))`);
    const draftToneControl = "style().min_height(px(28))";
    const draftToneActive=color("text_primary");
    const draftToneFill=color("email_draft.chip");
    emailDraft("tone",`${draftToneControl}.padding_x(px(${clean ? 10 : 9})).radius(px(${clean ? 6 : 8})).text_color(${color("email_draft.tone_ink")}).hover(style().background(${color("email_draft.tone_hover")}).text_color(${draftToneActive}))`);
    emailDraft("tone_selected",`style().background(${draftToneFill}).text_color(${draftToneActive}).hover(style().background(${draftToneFill}).text_color(${draftToneActive}))${shadows([shadow(0,0,0,clean ? .5 : 1,color("email_draft.tone_edge"),true)])}`);
    emailDraft("edit",`${draftToneControl}.width(px(28)).padding(px(0)).margin_start(px(4)).radius(px(${clean ? 6 : 8}))`);
    emailDraft("control_label",`style().font_size(px(${clean ? 12 : 13})).line_height(px(18)).font_weight(500)`);
    const draftDecision=`style().min_height(${readingLength(clean ? 28 : 36)}).padding_x(px(${clean ? 10 : 14})).radius(px(${clean ? 6 : 10})).gap(px(${clean ? 6 : 7}))`;
    emailDraft("decision_label",`${readingType(clean ? 13 : 14,20)}.font_weight(500)`);
    emailDraft("buttons","style().padding_start(px(6)).padding_end(px(10)).padding_y(px(10)).gap(px(8))");
    emailDraft("discard",`${draftDecision}.background(${color("email_draft.card")})${shadows(clean ? smallShadow : [hairline])}`);
    const draftSend=clean ? `.background(${color("email_draft.send")})`
      : layeredGradient([gradientLayer([rgba(dark ? "f4f4f5" : "323137"),color("email_draft.send")])]);
    emailDraft("send",`${draftDecision}${draftSend}.hover(style().background(${color("email_draft.send")}))`);
    emailDraft("receipt",`style().padding_start(px(${clean ? 16 : 12})).padding_end(px(${clean ? 12 : 10})).padding_y(px(12)).gap(px(10))${clean ? ".background(rgba(0x00000000)).border(px(0)).border_color("+draftEdge+").radius(px(0)).shadows([])" : ".background("+color("email_draft.card")+").border(px(1)).border_color("+draftEdge+").radius(theme_radius(\"lg\"))"+shadows([draftShadow])}`);
    emailDraft("footer",`style().border_top(px(${clean ? .5 : 0})).border_color(${draftEdge})`);
    emailDraft("footer_text",`style().gap(px(8)).padding_start(px(${clean ? 16 : 10})).padding_end(px(4)).padding_top(px(10)).padding_bottom(px(${clean ? 10 : 6}))`);
    emailDraft("agent",readingType(clean ? 13 : 14,20));
    emailDraft("hint",readingType(12,16));
    emailDraft("hint_rail",`style().padding_start(px(4)).padding_end(px(${clean ? 12 : 10})).padding_top(px(${clean ? 14 : 10})).padding_bottom(px(${clean ? 14 : 6}))`);
    emailDraft("badge",`${readingType(clean ? 12 : 12.5,16)}.font_weight(500).padding_x(px(${clean ? 6 : 8})).padding_y(px(2)).radius(px(${clean ? 5 : 8}))`);
    emailDraft("badge_done",`style().background(${color("email_draft.chip")}).text_color(${muted})`);
    emailDraft("notice","style().padding_x(px(10)).padding_bottom(px(6))");
    const diffTimeline=(part,value)=>set("patterns/diff_timeline",part,value);
    diffTimeline("root",`style().padding(px(${clean ? 0 : 4})).gap(px(4)).radius(px(${clean ? 10 : 21})).background(${color(clean ? "diff_timeline.card" : "diff_timeline.tray")})${clean ? shadows(smallShadow) : ".letter_spacing_em(-0.006)"}`);
    for(const part of ["panel","file_panel"])diffTimeline(part,clean
      ? `style().background(${transparent}).radius(px(0)).shadows([])`
      : `style().background(${color("diff_timeline.card")}).radius(theme_radius("lg"))${shadows(smallShadow)}`);
    diffTimeline("header",`style().gap(px(10)).padding_x(px(16)).padding_top(px(${clean ? 14 : 16})).padding_bottom(px(0))`);
    diffTimeline("mark",`style().width(px(16)).height(px(32)).radius(px(0)).background(${transparent})`);
    diffTimeline("title",`style().font_size(px(${clean ? 13 : 14})).line_height(px(${clean ? 18 : 20})).font_weight(${clean ? 500 : 600})`);
    diffTimeline("context",`style().font_size(px(${clean ? 13 : 14})).line_height(px(${clean ? 18 : 20})).font_weight(400)`);
    diffTimeline("status",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(${clean ? 16 : 18}))`);
    diffTimeline("across",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(16))`);
    for(const part of ["tip_line","tip_time"])diffTimeline(part,`style().font_size(px(${clean ? 11 : 12})).line_height(px(17))`);
    const diffTint=kind=>`alpha(${color("diff_timeline."+kind)},${kind === "add" || dark ? .14 : .10})`;
    for(const kind of ["add","del"])diffTimeline(kind,`style().height(px(${clean ? 20 : 22})).padding_x(px(${clean ? 6 : 8})).radius(px(${clean ? 5 : 999})).font_size(px(${clean ? 12 : 12.5})).line_height(px(16)).font_weight(${clean ? 500 : 600}).background(${diffTint(kind)})`);
    diffTimeline("body",`style().gap(px(8)).padding_x(px(${clean ? 12 : 14})).padding_top(px(8)).padding_bottom(px(4))`);
    diffTimeline("axis","style().width(px(34))");
    diffTimeline("axis_text","style().font_size(px(11)).line_height(px(16))");
    diffTimeline("time_axis","style().height(px(24))");
    diffTimeline("plot","style().height(px(156))");
    diffTimeline("area_add",`style().linear_gradient(linear_gradient(#{angle:180,from:alpha(${color("diff_timeline.add")},${clean && !dark ? .20 : .22}),to:alpha(${color("diff_timeline.add")},0)}))`);
    diffTimeline("area_del",`style().linear_gradient(linear_gradient(#{angle:180,from:alpha(${color("diff_timeline.del")},${dark ? .18 : clean ? .14 : .16}),to:alpha(${color("diff_timeline.del")},0)}))`);
    diffTimeline("tooltip",`style().padding_x(px(${clean ? 10 : 12})).padding_y(px(${clean ? 8 : 10})).border(px(0)).radius(theme_radius("lg"))${shadows(smallShadow)}`);
    diffTimeline("tip_path",`style().font_size(px(${clean ? 12 : 13})).line_height(px(18)).font_weight(500)`);
    const diffCodePaper=clean ? (dark ? white(.04) : black(.03)) : color("diff_timeline.tray");
    const diffCodeLine=clean ? (dark ? white(.14) : black(.08)) : color("diff_timeline.grid");
    diffTimeline("tip_code_panel",`style().background(${diffCodePaper})${shadows([shadow(0,0,0,clean ? .5 : 1,diffCodeLine,true)])}`);
    diffTimeline("tip_code","style().font_size(px(10.5)).line_height(px(18))");
    diffTimeline("tip_code_sign","style().width(px(18))");
    for(const kind of ["add","del"])diffTimeline("tip_code_"+kind,`style().background(${diffTint(kind)})`);
    diffTimeline("file_row",`style().height(px(${clean ? 30 : 32})).padding_x(px(${clean ? 8 : 10})).gap(px(${clean ? 8 : 10})).radius(px(${clean ? 7 : 10}))`);
    diffTimeline("count_measure",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(400)`);
    diffTimeline("file_count",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(16)).font_weight(${clean ? 500 : 600})`);
    diffTimeline("file_bar",`style().height(px(${clean ? 6 : 8}))`);
    diffTimeline("file_edit",`style().radius(px(${clean ? 2 : 3}))`);
    diffTimeline("wash",`style().radius(px(${clean ? 7 : 10})).background(${color("diff_timeline.wash")})`);
    for(const part of ["file_name","file_dir"])diffTimeline(part,`style().font_size(px(${clean ? 12.5 : 13})).line_height(px(18))`);
    for(const part of ["legend","net","net_value"])diffTimeline(part,`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(16))${part==="net_value" ? ".font_weight("+(clean?500:600)+")" : ""}`);
    diffTimeline("footer",`style().padding_x(px(${clean ? 16 : 12})).padding_top(px(${clean ? 6 : 10})).padding_bottom(px(${clean ? 14 : 7})).gap(px(16))`);
    const costWaterfall=(part,value)=>set("patterns/cost_waterfall",part,value);
    const cwEdge=clean ? (dark ? white(.08) : black(.06)) : edge;
    costWaterfall("root",clean
      ? `style().padding(px(0)).radius(theme_radius("lg")).background(${color("cost_waterfall.card")})${shadows(smallShadow)}`
      : `style().padding(px(6)).radius(px(21)).background(${color("cost_waterfall.tray")}).shadows([])`);
    costWaterfall("head",`style().gap(px(8)).padding_x(px(${clean ? 16 : 12})).padding_top(px(${clean ? 10 : 6})).padding_bottom(px(10)).border_bottom(px(${clean ? .5 : 0})).border_color(${cwEdge})`);
    costWaterfall("heading",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(${clean ? 500 : 600})`);
    costWaterfall("switch",`style().gap(px(1)).padding(px(2)).radius(px(999)).background(${productWash})`);
    costWaterfall("switch_button",`style().min_height(px(24)).padding_x(px(9)).radius(px(999))`);
    costWaterfall("selected",`style().background(${productPanel})${shadows(smallShadow)}`);
    costWaterfall("panel",clean ? `style().border(px(0)).border_color(${cwEdge}).background(${transparent}).radius(px(0)).shadows([])`
      : `style().border(px(1)).border_color(${cwEdge}).background(${productPanel}).radius(theme_radius("lg"))${shadows(smallShadow)}`);
    costWaterfall("stats",`style().padding_y(px(${clean ? 12 : 14})).border_bottom(px(${clean ? .5 : 1})).border_color(${cwEdge})`);
    costWaterfall("stat","style().padding_x(px(16))");
    costWaterfall("amount",`style().font_size(px(${clean ? 15 : 16})).line_height(px(24)).font_weight(${clean ? 500 : 600})`);
    for(const part of ["stat_label","sub","model"])costWaterfall(part,`style().font_size(px(${clean ? 11.5 : 12})).line_height(px(17))`);
    costWaterfall("task",readingType(13,20));
    costWaterfall("status",`${readingType(12,16)}.font_weight(500)`);
    costWaterfall("status_pill",`style().min_height(px(${clean ? 20 : 28})).padding_x(px(${clean ? 7 : 11})).gap(px(${clean ? 5 : 6})).radius(px(${clean ? 5 : 999}))`
      + (clean ? "" : `.background(${rgba(dark ? "232326" : "ffffff")})${shadows(smallShadow)}`));
    const costRunStatus=clean
      ? (dark ? [
        ["running","70b8ff","0077ff3a"],["done","5fc88a","00f04a1f"],["failed","ff8a7a","ff35232b"],
      ] : [
        ["running","006dcbf2","008ff519"],["done","007d2ee0","00a43319"],["failed","d72400da","f52b0018"],
      ])
      : (dark ? [
        ["running","8ab4ff"],["done","5fc88a"],["failed","ff8a7a"],
      ] : [
        ["running","2456d6"],["done","1e8a4f"],["failed","c4320a"],
      ]);
    for(const [state,ink,wash] of costRunStatus){
      costWaterfall(`status_${state}`,`style().text_color(${rgba(ink)})` + (wash ? `.background(${rgba(wash)})` : ""));
    }
    costWaterfall("top",`style().min_height(px(23)).padding_x(px(2)).gap(px(5)).radius(px(5))`);
    costWaterfall("header",`style().border_bottom(px(${clean ? .5 : 1})).border_color(${cwEdge})`);
    costWaterfall("row_open",`style().background(${productWash})`);
    costWaterfall("tick",`style().font_size(px(11.5)).line_height(px(16)).text_color(${muted})`);
    costWaterfall("metric","style().font_size(px(12)).line_height(px(16))");
    costWaterfall("column_label",`${readingType(clean ? 10 : 12,16)}${clean ? ".font_weight(500).letter_spacing_em(0.05)" : ""}`);
    costWaterfall("name",`style().font_size(px(${clean ? 12.5 : 13})).line_height(px(20))`);
    costWaterfall("bar",`style().height(px(${clean ? 14 : 16})).radius(px(${clean ? 4 : 5}))`);
    costWaterfall("fold","style().min_height(px(20)).padding(px(0)).items_center().justify_center()");
    costWaterfall("detail",`style().padding_x(px(14)).padding_y(px(10)).gap(px(8)).background(${productWash})`);
    costWaterfall("excerpt",`style().font_size(px(${clean ? 12 : 13})).line_height(px(19))`);
    const parallel=(part,value)=>set("patterns/parallel_attempts",part,value);
    const attemptPanel=color("parallel_attempts.card");
    const attemptWash=color("parallel_attempts.wash");
    const attemptEdge=color("parallel_attempts.done_edge");
    const attemptGlow=color("parallel_attempts.shine");
    const attemptShadows=clean ? smallShadow : [
      shadow(0,0,0,1,color("border")),shadow(0,1,2,0,black(dark ? .4 : .04))];
    parallel("root",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 0 : 20})).letter_spacing_em(${clean ? 0 : -.006})`);
    parallel("header",clean ? "style().padding(px(0)).margin_bottom(px(16))" : "style().padding_x(px(12)).padding_top(px(8)).padding_bottom(px(12)).margin_bottom(px(0))");
    parallel("summary_narrow","style().width(relative(1.0)).justify_between()");
    parallel("panel",`style().gap_y(px(${clean ? 8 : 4}))`);
    parallel("card",`style().gap_x(px(14)).padding_start(px(${clean ? 12 : 14})).padding_end(px(${clean ? 16 : 18})).padding_y(px(${clean ? 12 : 14})).radius(theme_radius("lg")).background(${attemptPanel})${shadows(attemptShadows)}`);
    parallel("card_narrow",`style().gap_x(px(12)).gap_y(px(12)).padding_end(px(${clean ? 14 : 16})).padding_bottom(px(${clean ? 14 : 16}))`);
    parallel("picked",clean
      ? `style().background(${attemptPanel})${layeredGradient([`#{angle:180,stops:[#{offset:0.0,color:${attemptWash}},#{offset:1.0,color:${attemptWash}}]}`])}${shadows([...attemptShadows,shadow(0,0,0,.5,attemptEdge)])}`
      : `style().background(${attemptWash})${shadows([shadow(0,0,0,1,attemptEdge),attemptShadows[1]])}`);
    parallel("quiet",`style().opacity(${clean ? .5 : .55})`);
    const attemptType=(size,line)=>`style().font_size(${length("space.unit")}.times(${size/4})).line_height(${length("space.unit")}.times(${line/4}))`;
    parallel("icon",`${attemptType(clean ? 12 : 13,20)}.width(px(${clean ? 28 : 32})).height(px(${clean ? 28 : 32})).radius(px(${clean ? 7 : 9})).font_weight(${clean ? 500 : 600})${shadows(attemptShadows)}`);
    for (const part of ["name","title","score"]) parallel(part,`${attemptType(clean ? 13 : 14,20)}.font_weight(${clean ? 500 : 600})`);
    parallel("score_running","style().font_weight(400)");
    parallel("score_failed","style().font_weight(500)");
    for (const part of ["line","meta"]) parallel(part,attemptType(clean ? 12 : 13,clean ? 18 : 19));
    parallel("task",`${attemptType(13,20)}.font_weight(400)`);
    for (const part of ["model","approach"]) parallel(part,`${attemptType(clean ? 13 : 14,20)}.font_weight(400)`);
    parallel("stats",attemptType(clean ? 12 : 13,clean ? 16 : 18));
    parallel("stats_value",`style().font_weight(${clean ? 500 : 600})`);
    parallel("line_narrow",`style().min_height(px(${clean ? 36 : 38}))`);
    parallel("track",`style().height(px(${clean ? 8 : 10})).radius(px(${clean ? 4 : 5}))`);
    parallel("fill",`style().border(px(${clean ? .5 : 1})).radius(px(${clean ? 4 : 5}))`);
    parallel("sheen",`style()${layeredGradient([`#{angle:90,stops:[#{offset:0.0,color:${transparent}},#{offset:0.5,color:${attemptGlow}},#{offset:1.0,color:${transparent}}]}`])}.opacity(0.85)`);
    parallel("end",`style().width(px(${clean ? 96 : 100}))`);
    parallel("end_narrow",`style().width(px(${clean ? 84 : 88}))`);
    parallel("best",`${attemptType(clean ? 11 : 12,clean ? 18 : 20)}.padding_x(px(${clean ? 6 : 8})).radius(px(${clean ? 4 : 999}))`);
    parallel("meta_slot",`style().height(px(${clean ? 18 : 19}))`);
    parallel("pick",`${attemptType(clean ? 12 : 13,18)}.min_height(px(${clean ? 24 : 26})).padding_x(px(${clean ? 6 : 9})).radius(px(${clean ? 6 : 999})).inset_end(offset_px(${clean ? -6 : -8})).top(offset_px(${clean ? -3 : -4}))`);
    const fanout=(part,value)=>set("patterns/fan_out_grid",part,value);
    const fanPanel=color("fan_out_grid.paper"), fanInk=color("fan_out_grid.ink"), fanQuiet=color("fan_out_grid.muted");
    const fanHover=color("fan_out_grid.filter_wash"), fanKeyFill=clean ? fanHover : fanPanel;
    const fanPop=[
      shadow(0,0,0,clean ? .5 : 1,dark ? rgba(clean ? "ffffff1f" : "34343a") : clean ? black(.08) : rgba("ececec")),
      ...floatingCast(),
      ...(clean ? [shadow(0,1,2,0,black(.05))] : []),
    ];
    fanout("root",`style().padding(px(${clean ? 0 : 4})).gap_y(px(${clean ? 14 : 0})).radius(px(${clean ? 0 : 20})).letter_spacing_em(${clean ? 0 : -.006})`);
    fanout("header",clean ? "style().padding(px(0))" : "style().padding_x(px(12)).padding_top(px(8)).padding_bottom(px(12))");
    fanout("title",`${readingType(clean ? 13 : 14,20)}.font_weight(${clean ? 500 : 600}).text_color(${color("fan_out_grid.strong")})`);
    fanout("job",`${readingType(13,20)}.text_color(${fanQuiet})`);
    fanout("metric",readingType(clean ? 12 : 13,clean ? 16 : 18));
    fanout("summary_compact","style().width(relative(1.0)).justify_between()");
    const fanPaper=`style().background(${fanPanel}).radius(theme_radius("lg"))${shadows(attemptShadows)}`;
    fanout("body",clean ? `${fanPaper}.padding(px(16)).padding_bottom(px(14)).gap_y(px(14))` : "style()");
    fanout("body_compact",clean ? "style().padding(px(12))" : "style()");
    fanout("panel",clean ? "style()" : `${fanPaper}.padding(px(18))`);
    fanout("panel_compact",clean ? "style()" : "style().padding(px(14))");
    fanout("cell",`style().radius(px(${clean ? 3 : 3.5})).border(px(${clean ? .5 : 1}))`);
    fanout("cell_attention",`style().border(px(${clean ? 1 : 1.5}))`);
    fanout("cell_pinned",`style().border(px(${clean ? 1 : 1.5}))${shadows([shadow(0,0,0,2,color("fan_out_grid.pin"))])}`);
    fanout("tip",`style().max_width(px(${clean ? 280 : 290})).padding_x(px(${clean ? 12 : 14})).padding_top(px(${clean ? 9 : 10})).padding_bottom(px(${clean ? 10 : 11})).gap_y(px(3)).radius(${clean ? 'theme_radius("lg")' : 'px(14)'}).background(${rgba(dark ? "232326" : "ffffff")})${shadows(fanPop)}`);
    fanout("tip_title",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(600)`);
    fanout("tip_status",`style().font_size(px(${clean ? 11 : 12})).line_height(px(${clean ? 18 : 20})).padding_x(px(${clean ? 6 : 8})).radius(px(${clean ? 4 : 999}))`);
    for(const part of ["tip_meta","tip_error","tip_result"])fanout(part,`style().font_size(px(${clean ? 12 : 13})).line_height(px(${clean ? 18 : 19}))`);
    fanout("footer",`style().padding_start(px(${clean ? 0 : 8})).padding_end(px(${clean ? 0 : 6})).padding_top(px(${clean ? 0 : 8})).padding_bottom(px(${clean ? 0 : 4}))`);
    fanout("filter",`style().min_height(px(${clean ? 24 : 28})).padding_x(px(${clean ? 6 : 9})).gap_x(px(${clean ? 6 : 7})).radius(px(${clean ? 6 : 999})).text_color(${fanQuiet}).hover(style().background(${fanHover}).text_color(${fanInk}))`);
    fanout("filter_count",`style().font_weight(${clean ? 400 : 600})`);
    fanout("filter_on",`style().background(${fanKeyFill}).text_color(${fanInk})${shadows(clean ? [] : attemptShadows)}.hover(style().background(${fanKeyFill}).text_color(${fanInk}))`);
    fanout("filter_dim",`style().opacity(${clean ? 0.45 : 0.50}).isolated_opacity(true)`);
    fanout("filter_empty","style().opacity(0.45).isolated_opacity(true)");
    fanout("swatch",`style().width(px(${clean ? 8 : 9})).height(px(${clean ? 8 : 9})).radius(px(${clean ? 2.5 : 3})).border(px(${clean ? .5 : 1}))`);
    fanout("retry",`style().min_height(px(${clean ? 24 : 30})).padding_x(px(${clean ? 6 : 12})).radius(px(${clean ? 6 : 999})).font_size(px(${clean ? 12 : 13})).background(${clean ? transparent : fanPanel})${shadows(clean ? [] : attemptShadows)}`);
    const billing=(part,value)=>set("patterns/billing_schedule",part,value);
    const billingFace=color("billing_schedule.card"),billingLine=color("billing_schedule.line");
    const billingShadows=clean ? (dark ? [shadow(0,0,0,.5,white(.1)),shadow(0,1,2,0,black(.4))] : smallShadow)
      : [shadow(0,0,0,1,dark ? white(.08) : rgba("e5e5e5")),shadow(0,1,2,0,black(dark ? .4 : .06))];
    const fieldShadows=clean ? billingShadows : [shadow(0,0,0,1,dark ? white(.08) : rgba("e5e5e5"),true),shadow(0,1,1,0,black(.03))];
    billing("root",clean ? `style().padding(px(0)).radius(theme_radius("lg")).background(${billingFace})${shadows(billingShadows)}`
      : `style().padding(px(6)).radius(px(20)).background(${color("billing_schedule.tray")}).shadows([])`);
    billing("heading",`style().padding_x(px(${clean ? 14 : 10})).padding_top(px(${clean ? 14 : 10})).padding_bottom(px(${clean ? 2 : 12}))`);
    billing("title",`style().font_size(px(15)).line_height(px(24)).font_weight(500).letter_spacing_em(${clean ? 0 : -.01})`);
    billing("card",clean ? `style().padding(px(0)).gap_y(px(0)).background(${transparent}).radius(px(0)).shadows([])`
      : `style().padding(px(12)).gap_y(px(6)).background(${billingFace}).radius(theme_radius("lg"))${shadows(billingShadows)}`);
    billing("summary",`style().padding_start(px(${clean ? 10 : 0})).padding_end(px(${clean ? 14 : 2})).padding_top(px(${clean ? 0 : 2})).padding_bottom(px(${clean ? 10 : 12})).gap_x(px(${clean ? 6 : 8})).gap_y(px(${clean ? 0 : 2}))`);
    billing("phase_field",`style().padding_start(px(${clean ? 4 : 2})).padding_end(px(4)).radius(px(${clean ? 5 : 6})).background(${transparent}).hover(style().background(${clean ? productWash : wash})).focus(style().background(${clean ? billingFace : wash})${clean ? shadows(billingShadows) : ""})`);
    billing("phase",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).text_color(${color("billing_schedule.ink")})`);
    billing("phase_dot",`style().font_size(px(13)).line_height(px(20)).text_color(${placeholder})`);
    billing("range",`style().gap_x(px(${clean ? 6 : 8}))`);
    for (const part of ["date", "end"])
      billing(part,`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).text_color(${color("billing_schedule.text")})`);
    billing("fields",clean ? "style().padding_x(px(14)).padding_top(px(0)).padding_bottom(px(14)).gap(px(10)).radius(px(0))"
      : "style().padding(px(10)).gap(px(10)).radius(px(12))");
    billing("caption",`style().font_size(px(${clean ? 12 : 13})).line_height(px(17))`);
    billing("field_caption",`style().line_height(px(${clean ? 16 : 17})).text_color(${color("billing_schedule.text")})`);
    for(const part of ["date_trigger","duration"])billing(part,`style().min_height(px(${clean ? 32 : 40})).padding_x(px(${clean ? 10 : 12})).border(px(0)).radius(px(${clean ? 8 : 10})).background(${billingFace})${shadows(fieldShadows)}`);
    billing("menu",`style().border(px(0)).radius(px(${clean ? 10 : 12})).padding(px(5))${shadows(billingShadows)}`);
    billing("head_row",`style().min_height(px(${clean ? 32 : 40})).padding_start(px(${clean ? 14 : 6})).padding_end(px(${clean ? 10 : 4})).gap_x(px(${clean ? 12 : 14})).border_top(px(${clean ? .5 : 0})).border_bottom(px(${clean ? .5 : 1})).border_color(${billingLine})`);
    billing("line",`style().min_height(px(${clean ? 48 : 52})).padding_start(px(${clean ? 14 : 6})).padding_end(px(${clean ? 10 : 4})).padding_y(px(6)).gap_x(px(${clean ? 12 : 14})).border_bottom(px(${clean ? .5 : 1})).border_color(${billingLine})`);
    billing("compact","style().padding_y(px(10)).gap_x(px(0)).gap_y(px(2))");
    billing("name",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20))`);
    billing("inline",`style().min_height(px(24)).padding_x(px(4)).font_size(px(${clean ? 13 : 14}))`);
    billing("timing","style().min_height(px(20)).padding_x(px(4)).font_size(px(12))");
    billing("price_input",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20))`);
    billing("price_field",`style().focus_within(style()${shadows(fieldShadows)})`);
    billing("actions",`style().padding_x(px(${clean ? 14 : 0})).padding_y(px(${clean ? 12 : 4})).gap(px(6))`);
    billing("action",`style().min_height(px(${clean ? 28 : 32})).padding_x(px(${clean ? 8 : 10})).radius(px(${clean ? 7 : 9})).background(${billingFace})${shadows(fieldShadows)}.disabled(style().opacity(0.45))`);
    billing("totals",`style().padding_x(px(${clean ? 14 : 10})).padding_top(px(${clean ? 10 : 12})).padding_bottom(px(${clean ? 10 : 6})).border_top(px(${clean ? .5 : 0})).border_color(${billingLine})`);
    billing("summary_amount",`style().font_size(px(${clean ? 15 : 13})).line_height(px(24)).font_weight(${clean ? 500 : 600})`);
    billing("summary_label",`style().font_size(px(${clean ? 12 : 13})).line_height(px(18))`);
    billing("min_tag",`style().font_size(px(12)).line_height(px(18)).background(${clean ? color("selection") : transparent}).text_color(${color("billing_schedule.tint")})`);
    const planDiff=(part,value)=>set("patterns/plan_vs_actual",part,value);
    const pdShadows=dark ? [shadow(0,0,0,.5,white(.10)),shadow(0,1,2,0,black(.4))] : smallShadow;
    planDiff("root",`style().radius(px(12)).background(${color("plan_vs_actual.card")})${shadows(pdShadows)}`);
    planDiff("head",`style().padding_start(px(16)).padding_end(px(14)).padding_y(px(12)).gap_x(px(8)).gap_y(px(6)).border_bottom(px(0.5)).border_color(${edge})`);
    planDiff("head_narrow","style().padding_start(px(12)).padding_end(px(12)).padding_y(px(10)).gap_y(px(6))");
    planDiff("totals","style().gap_x(px(14)).margin_end(px(4))");
    planDiff("totals_narrow","style().gap_x(px(16)).gap_y(px(4)).margin_end(px(0))");
    planDiff("metric","style().items_baseline().gap_x(px(6))");
    for(const part of ["title","task","step"])planDiff(part,`style().font_size(px(13)).line_height(px(20))`);
    for(const part of ["tick","metric_label","metric_value","detail_line","duration","delta","finish_tag","verdict"])planDiff(part,`style().font_size(px(12)).line_height(px(16)).font_feature("tnum",1)`);
    planDiff("axis",`style().height(px(32)).border_bottom(px(0.5)).border_color(${edge})`);
    planDiff("axis_label","style().font_size(px(10)).line_height(px(16)).letter_spacing_px(0.5).font_weight(500)");
    planDiff("row","style().height(px(32))");
    planDiff("row_hot",`style().background(${dark ? white(.025) : black(.018)})`);
    planDiff("row_dim","style().opacity(0.45).isolated_opacity(true)");
    planDiff("step_icon",`style().radius(px(5)).background(${color("plan_vs_actual.card")})${shadows(pdShadows)}`);
    for(const part of ["plan","actual","over","forecast"])planDiff(part,"style().height(px(14)).radius(px(5))");
    planDiff("verdict", `style().font_size(px(12)).line_height(px(20)).font_weight(500).padding_x(px(8)).radius(px(6))${dark ? `.background(${color("plan_vs_actual.wash")})` : ""}`);
    planDiff("delta","style().font_size(px(11)).line_height(px(16)).padding_x(px(4)).radius(px(4))");
    planDiff("wire_dot","style().radius(px(1))");
    planDiff("now_dot",`style()${shadows([shadow(0,0,0,3,color("plan_vs_actual.card"))])}`);
    planDiff("detail",`style().padding_x(px(16)).padding_y(px(9)).gap(px(8)).border_top(px(0.5)).border_color(${edge})`);
    planDiff("legend",`style().min_height(px(40)).padding_x(px(16)).gap_x(px(14)).border_top(px(0.5)).border_color(${edge})`);
    planDiff("legend_narrow","style().height(px(14)).min_height(px(0))");
    planDiff("finish_tag","style().font_size(px(12)).line_height(px(20)).padding_x(px(6)).radius(px(5))");
    const fallbackChain=(part,value)=>set("patterns/fallback_chain",part,value);
    const memoryPanel=(part,value)=>set("patterns/memory_panel",part,value);
    const toolAccess=(part,value)=>set("patterns/tool_access_map",part,value);
    const trace=(part,value)=>set("patterns/trace_waterfall",part,value);
    const rec=(part,value)=>set("patterns/smart_recommendation",part,value);
    const rate=(part,value)=>set("patterns/rate_limit_planner",part,value);
    const cause=(part,value)=>set("patterns/root_cause_tree",part,value);
    const causeEdge=color("root_cause.line"),causeCard=color("root_cause.card");
    const causeShadow=clean ? smallShadow : [shadow(0,0,0,1,causeEdge),shadow(0,1,2,0,black(dark ? .30 : .04))];
    const causeHover=[shadow(0,0,0,clean ? .5 : 1,dark ? white(.16) : black(.09)),shortDrop(black(dark ? .25 : .12),0,-1)];
    const causeFlat=shadows([shadow(0,0,0,clean ? .5 : 1,causeEdge)]);
    cause("root","style().gap_y(px(0))");
    cause("head","style().padding_x(px(2)).padding_bottom(px(14)).gap_x(px(8))");
    cause("title","style().font_size(px(14)).line_height(px(20)).font_weight(500)");
    cause("summary","style().font_size(px(12)).line_height(px(16)).font_feature(\"tnum\",1)");
    cause("live","style().min_height(px(18)).padding_x(px(5)).font_size(px(11)).line_height(px(16))");
    cause("diagram","style().gap_y(px(6))");
    cause("card",`style().padding(px(${clean ? 3 : 3})).radius(px(${clean ? 10 : 12}))${clean ? ".background("+causeCard+")"+shadows(causeShadow) : ".shadows([])"}`);
    cause("card_selected",`style()${clean ? shadows(causeHover) : ".shadows([])"}`);
    cause("card_excluded",`style()${clean ? ".background("+transparent+")"+causeFlat : ".shadows([])"}`);
    cause("inner",`style().padding_x(px(${clean ? 7 : 8})).padding_y(px(${clean ? 6 : 7})).gap_y(px(6)).radius(px(${clean ? 7 : 9}))
      ${clean ? ".background("+transparent+").shadows([])" : shadows(causeShadow)}`);
    cause("inner_selected",`style()${clean ? ".shadows([])" : shadows(causeHover)}`);
    cause("inner_excluded",`style().background(${transparent})${clean ? ".shadows([])" : causeFlat}`);
    cause("meta",`style().min_height(px(${clean ? 20 : 22})).padding_x(px(${clean ? 7 : 6})).gap_x(px(5))`);
    cause("name","style().font_size(px(13)).line_height(px(18)).font_weight(500)");
    cause("status","style().font_size(px(11)).line_height(px(16))");
    cause("file",`style().min_height(px(18)).radius(px(5)).font_size(px(11)).line_height(px(16))`);
    cause("file_name","style().font_size(px(11)).line_height(px(16)).font_weight(400)");
    cause("badge","style().min_height(px(16)).radius(px(4)).font_size(px(10.5)).line_height(px(16)).padding_x(px(5))");
    cause("gate",`style().min_height(px(16)).radius(px(4)).font_size(px(9.5)).line_height(px(16)).letter_spacing_px(0.5)${shadows(causeShadow)}`);
    cause("tile",`style().radius(px(5))${shadows(causeShadow)}`);
    cause("panel",`style().margin_top(px(18)).padding(px(${clean ? 12 : 4})).radius(px(${clean ? 12 : 14}))${clean ? ".background("+causeCard+")"+shadows(causeShadow) : ".shadows([])"}`);
    cause("panel_head",`style().padding_x(px(${clean ? 2 : 8})).padding_top(px(${clean ? 0 : 6})).padding_bottom(px(4)).gap_x(px(6))`);
    cause("panel_title","style().font_size(px(13)).line_height(px(20)).font_weight(500)");
    cause("panel_body",`style().padding_x(px(${clean ? 2 : 12})).padding_y(px(${clean ? 2 : 10})).gap_y(px(8)).radius(px(10))${clean ? ".background("+transparent+").shadows([])" : shadows(causeShadow)}`);
    cause("verdict","style().font_size(px(12.5)).line_height(px(18))");
    cause("panel_status","style().font_size(px(12)).line_height(px(16))");
    cause("evidence_label","style().font_size(px(10)).line_height(px(16)).letter_spacing_em(0.05)");
    cause("source","style().font_size(px(10)).line_height(px(16)).letter_spacing_em(0)");
    cause("evidence",`style().padding_x(px(10)).padding_y(px(8)).radius(px(7))${shadows([shadow(0,0,0,clean ? .5 : 1,causeEdge,true)])}`);
    cause("button",`style().min_height(px(24)).padding_x(px(8)).radius(px(6)).font_size(px(12)).font_weight(500)${shadows(causeShadow)}`);
    const bracket=(part,value)=>set("patterns/judge_bracket",part,value);
    const bracketCard=color("bracket.card"),bracketEdge=color("bracket.line");
    const bracketShadow=clean ? smallShadow : [shadow(0,0,0,1,bracketEdge),shadow(0,1,2,0,black(dark ? .30 : .04))];
    const bracketHover=[shadow(0,0,0,clean ? .5 : 1,dark ? white(.16) : black(.09)),shortDrop(black(dark ? .25 : .12),0,-1)];
    bracket("root","style().gap_y(px(0))");
    bracket("head","style().padding_x(px(2)).padding_bottom(px(14)).gap_x(px(8))");
    bracket("title",`${readingType(13,20)}.font_weight(500)`);
    bracket("task",readingType(13,20));
    bracket("summary",`${readingType(12,16)}.font_feature("tnum",1)`);
    bracket("live",`${readingType(11,16)}.min_height(${readingLength(18)}).padding_x(px(5))`);
    bracket("round","style().font_size(px(10)).line_height(px(16)).letter_spacing_em(0.08)");
    bracket("match","style().radius(px(11))");
    bracket("slot",`style().radius(px(10)).padding_x(px(10))${shadows(bracketShadow)}`);
    bracket("slot_active",`style()${shadows(bracketHover)}`);
    bracket("slot_empty","style().shadows([])");
    bracket("slot_lost",`style().background(${color("bracket.tray")})${clean ? shadows([shadow(0,0,0,.5,bracketEdge)]) : ".shadows([])"}`);
    bracket("slot_champion",`style()${shadows([...bracketHover,shadow(0,0,0,1,color("bracket.path"),true)])}`);
    bracket("tile",`style().width(px(18)).height(px(18)).radius(px(5))${shadows(bracketShadow)}`);
    bracket("name","style().font_size(px(12.5)).line_height(px(18)).font_weight(500)");
    for(const part of ["score","waiting"]) bracket(part,"style().font_size(px(11.5)).line_height(px(16))");
    bracket("panel",`style().margin_top(px(18)).padding_x(px(${clean ? 14 : 4})).padding_y(px(${clean ? 12 : 4})).radius(px(${clean ? 12 : 16}))${clean ? ".background("+bracketCard+")"+shadows(bracketShadow) : ".shadows([])"}`);
    bracket("panel_head",`style().padding_x(px(${clean ? 0 : 8})).padding_top(px(${clean ? 0 : 6})).padding_bottom(px(4)).gap_x(px(6))`);
    bracket("panel_title","style().font_size(px(13)).line_height(px(20)).font_weight(500)");
    bracket("answer",clean ? `style().padding(px(10)).padding_top(px(9)).radius(px(9)).background(${color("bracket.well")})${shadows([shadow(0,0,0,.5,bracketEdge,true)])}`
      : `style().padding(px(10)).radius(px(10))${shadows(bracketShadow)}`);
    bracket("answer_name",`${readingType(clean ? 12.5 : 13,18)}.font_weight(500)`);
    for(const part of ["panel_status","reason","notice"]) bracket(part,"style().font_size(px(12)).line_height(px(17))");
    bracket("answer_meta","style().font_size(px(11)).line_height(px(16))");
    bracket("output",`style().padding_x(px(10)).padding_y(px(8)).radius(px(7))${shadows([shadow(0,0,0,clean ? .5 : 1,bracketEdge,true)])}`);
    bracket("badge","style().min_height(px(16)).radius(px(4)).font_size(px(10.5)).line_height(px(16)).padding_x(px(5))");
    bracket("button",`style().min_height(px(24)).padding_x(px(8)).radius(px(6)).font_size(px(12)).font_weight(500)${shadows(bracketShadow)}`);
    const stream=(part,value)=>set("patterns/collaboration_stream",part,value);
    const streamLine=color("stream.line"),streamCard=color("stream.card");
    const streamSurface=clean ? smallShadow : [shadow(0,0,0,1,streamLine),shadow(0,1,2,0,black(dark ? .30 : .04))];
    const streamPicked=[shadow(0,0,0,clean ? .5 : 1,dark ? white(.14) : black(.10)),shortDrop(black(dark ? .25 : .12),0,-1)];
    const streamLift=[shadow(0,0,0,clean ? .5 : 1,streamLine),...floatingCast()];
    stream("root",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 0 : 20})).gap_y(px(${clean ? 6 : 4}))
       .font_size(px(${clean ? 13 : 13.5})).line_height(px(20)).letter_spacing_em(${clean ? 0 : -.006})`);
    stream("top",clean ? "style().padding_bottom(px(4)).gap_x(px(12))"
      : "style().padding_start(px(2)).padding_end(px(12)).padding_top(px(2)).padding_bottom(px(6)).gap_x(px(12))");
    stream("tabs",clean ? "style().padding(px(0)).margin_start(offset_px(-8))" : "style().padding(px(2))");
    stream("tab",`style().min_height(px(28)).padding_x(px(8)).radius(px(7)).gap_x(px(7)).font_size(px(12.5)).line_height(px(16))`);
    stream("tab_selected",`style().background(${clean ? color("stream.fill") : streamCard})${clean ? ".shadows([])" : shadows(streamSurface)}`);
    stream("panel",`style().radius(px(${clean ? 0 : 16}))${clean ? ".background("+transparent+").shadows([])" : shadows(streamSurface)}`);
    stream("bubble",`style().padding_x(px(12)).padding_y(px(10)).radius(px(12)).gap_y(px(7))${clean ? shadows(streamSurface)+".hover(style()"+shadows(streamPicked)+")" : ".shadows([])"}`);
    stream("own",`style()${shadows([shadow(0,0,0,clean ? .5 : 1,streamLine,true)])}`);
    stream("active",`style()${clean ? shadows(streamPicked) : ".background("+color("stream.hover")+")"}`);
    stream("body",`style().font_size(px(${clean ? 13 : 13.5})).line_height(px(20))`);
    stream("meta",`style().font_size(px(${clean ? 13 : 13.5})).line_height(px(20))`);
    stream("sender",`style().font_size(px(${clean ? 13 : 13.5})).line_height(px(20)).font_weight(500)`);
    stream("quote","style().padding_x(px(8)).padding_y(px(5)).radius(px(8)).gap_y(px(1))");
    if (clean) stream("quote_own",`style().background(${streamCard})`);
    stream("quote_name","style().font_size(px(11.5)).line_height(px(16))");
    stream("quote_text","style().font_size(px(12)).line_height(px(17))");
    for(const part of ["live","time","targets","preview_meta"]) stream(part,"style().font_size(px(12)).line_height(px(16))");
    stream("target_person",`style().gap(px(5)).text_color(${color("stream.text")})`);
    stream("target_overflow",`style().text_color(${color("stream.muted")}).font_feature("tnum",1)`);
    for(const part of ["badge","needs","reply"]) stream(part,"style().font_size(px(11)).font_weight(500).line_height(px(18)).min_height(px(18)).radius(px(5))");
    stream("chip",`style().min_height(px(24)).radius(px(7)).font_size(px(12)).line_height(px(18)).letter_spacing_em(0)
       ${clean ? ".background("+color("stream.well")+").shadows([]).hover(style().background("+color("stream.fill")+").text_color("+color("stream.ink")+"))"
                : shadows(streamSurface)+".hover(style()"+shadows(streamPicked)+".text_color("+color("stream.ink")+"))"}`);
    stream("details",`style().radius(px(${clean ? 10 : 12})).padding(px(12))${shadows(streamLift)}`);
    stream("latest",`style().radius(px(13)).font_size(px(12)).line_height(px(16)).font_weight(500)${shadows(streamLift)}`);
    stream("composer",`style().padding_x(px(10)).padding_top(px(12)).padding_bottom(px(10)).radius(px(16)).gap_y(px(8))${shadows(streamSurface)}`);
    stream("editor",`style().padding_x(px(4)).padding_y(px(0)).font_size(px(${clean ? 13 : 13.5})).line_height(px(20)).text_color(${color("stream.ink")})`);
    stream("mentions",`style().radius(px(10)).padding(px(4))${shadows(streamLift)}`);
    stream("mention","style().padding_x(px(8)).gap_x(px(8)).font_size(px(12.5)).line_height(px(17))");
    stream("cursor",`style()${shadows([shadow(0,0,0,1,color("focus_ring"),true)])}`);
    const radar=(part,value)=>set("patterns/conflict_radar",part,value);
    const radarLine=color("radar.line");
    const radarCard=clean ? smallShadow : [shadow(0,0,0,1,radarLine),shadow(0,1,2,0,black(dark ? .40 : .04))];
    radar("root",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 10 : 20})).font_size(px(${clean ? 13 : 14}))
       .line_height(px(20)).letter_spacing_em(${clean ? 0 : -.006})`);
    radar("bar",`style().min_height(px(${clean ? 48 : 44})).padding_x(px(${clean ? 12 : 10})).gap_x(px(10))`);
    radar("title","style().font_size(px(14)).line_height(px(20)).font_weight(600)");
    radar("merge",`style().min_height(px(${clean ? 24 : 26})).radius(px(${clean ? 6 : 9})).font_size(px(${clean ? 12 : 13})).line_height(px(18))${shadows(radarCard)}`);
    radar("merge_ready","style().shadows([])");
    radar("card",`style().radius(px(${clean ? 10 : 16}))${shadows(radarCard)}`);
    for(const part of ["row","agent","cell"]) radar(part,`style().border_color(${radarLine})`);
    radar("avatar","style().font_size(px(10)).line_height(px(14)).font_weight(500)");
    for(const [part,edge] of Object.entries({edit:"edit_edge",conflict:"hit_edge",kept:"kept_edge",mixed:"edit_edge"})){
      radar("bar_"+part,`style().border(px(0))${shadows([shadow(0,0,0,.50,color("radar."+edge),true)])}`);
    }
    radar("file","style().padding_start(px(16)).padding_end(px(12)).gap_x(px(10))");
    for(const part of ["directory","path"]) radar(part,`${readingType(clean ? 13 : 14,20)}.font_weight(400)`);
    radar("decision",`style().padding(px(${clean ? 10 : 12})).radius(px(${clean ? 10 : 16}))${shadows(radarCard)}`);
    radar("way",`style().radius(px(${clean ? 7 : 10})).padding(px(10))${shadows(radarCard)}`);
    radar("kept",`style()${shadows([shadow(0,0,0,.5,color("radar.kept_edge"),true)])}`);
    radar("pager","style().radius(px(6))");
    radar("summary","style().font_size(px(12)).line_height(px(17))");
    cause("primary","style().shadows([])");
    const rlEdge=color("rate_plan.line");
    const rlShadow=clean ? smallShadow : [shadow(0,0,0,1,rgba(dark ? "2a2a2e" : "ececec")),shadow(0,1,2,0,black(dark ? .2 : .04))];
    rate("root",`style().radius(px(${clean ? 10 : 20}))${shadows(rlShadow)}`);
    rate("head","style().padding_bottom(px(0))");
    rate("title",`style().font_size(px(${clean ? 15 : 16})).font_weight(500).line_height(px(22))`);
    rate("summary","style().font_size(px(12)).line_height(px(18))");
    rate("badge","style().min_height(px(22)).radius(px(99)).padding_x(px(10)).font_size(px(12)).line_height(px(16)).font_weight(500)");
    rate("button",`style().min_height(px(28)).padding_x(px(9)).radius(px(${clean ? 6 : 10})).font_size(px(12.5))${shadows(rlShadow)}`);
    rate("scale","style().font_size(px(11)).line_height(px(16))");
    rate("plot",`style().border_bottom(px(${clean ? 0.5 : 1})).border_color(${clean ? color("rate_plan.guide") : rlEdge})`);
    rate("plan_fill",`style().linear_gradient(linear_gradient(#{angle:180,from:${color("rate_plan.wash")},to:alpha(${color("rate_plan.wash")},0.0)}))`);
    rate("selected_fill",`style().linear_gradient(linear_gradient(#{angle:180,from:alpha(${color("rate_plan.accent")},0.16),to:alpha(${color("rate_plan.accent")},0.0)}))`);
    rate("tick","style().font_size(px(11)).line_height(px(16))");
    rate("peak","style().min_height(px(18)).font_size(px(11.5)).line_height(px(18)).font_weight(500)");
    rate("tip","style().font_size(px(11)).line_height(px(16))");
    rate("flag","style().font_size(px(11)).line_height(px(16))");
    rate("name",'style().typography("body")');
    rate("start",`style().typography("metadata").line_height(${length("space.unit")}.times(5))`);
    rate("bar",`style().height(px(16)).radius(px(${clean ? 5 : 7}))`);
    rate("bar_text","style().font_size(px(10.5)).line_height(px(16)).font_weight(500)");
    rate("icon",`style().radius(px(${clean ? 5 : 7})).hover(style().background(${rlEdge}))`);
    rate("editor",`style().padding(px(12)).radius(px(${clean ? 10 : 16})).gap_y(px(5))
      ${shadows([shadow(0,0,0,clean ? .5 : 1,rlEdge),...floatingCast()])}`);
    rate("add","style().min_height(px(24)).font_size(px(12.5))");
    rate("foot","style().min_height(px(48)).margin_top(px(2))");
    rate("verdict","style().font_size(px(12.5)).line_height(px(18))");
    rate("fit",`style().min_height(px(28)).font_size(px(12.5)).radius(px(${clean ? 6 : 10}))`);
    const srEdge=color("recommendation.line");
    const srLift=clean ? smallShadow : [shadow(0,0,0,1,srEdge),shadow(0,1,2,0,black(dark ? .4 : .04))];
    const srPop=[shadow(0,0,0,clean ? .5 : 1,dark ? white(.1) : black(.08)),...floatingCast()];
    const srPrimary=[shadow(0,0,0,1,dark ? black(.6) : rgba("0d0d0d")),
      shortDrop(black(dark ? .35 : .1)),shadow(0,1,0,0,dark ? rgba("ffffff") : white(.12),true)];
    const srGradient=(top,bottom)=>`.linear_gradient(linear_gradient(#{angle:180,from:${rgba(top)},to:${rgba(bottom)}}))`;
    rec("root",`style().max_width(px(440)).padding(px(${clean ? 0 : 6})).radius(px(${clean ? 10 : 22}))${clean ? shadows(srLift) : ".shadows([])"}`);
    rec("card",`style().radius(px(${clean ? 10 : 16})).padding_x(px(${clean ? 0 : 8})).padding_top(px(${clean ? 0 : 6})).padding_bottom(px(${clean ? 0 : 8})).border(px(${clean ? 0 : 1})).border_color(${srEdge})${clean ? ".shadows([])" : shadows([shadow(0,1,2,0,black(dark ? .4 : .06))])}`);
    rec("head",`style().min_height(px(${clean ? 44 : 40})).padding_start(px(${clean ? 16 : 10})).padding_end(px(${clean ? 12 : 10})).radius(px(${clean ? 8 : 12})).gap_x(px(${clean ? 8 : 10}))`);
    rec("title",`style().font_size(px(${clean ? 14 : 15})).line_height(px(20)).font_weight(500)`);
    rec("flag",`style().font_size(px(11.5)).line_height(px(16)).padding_x(px(${clean ? 7 : 8})).radius(px(99))`);
    rec("reason",`style().font_size(px(${clean ? 13 : 14})).line_height(px(${clean ? 20 : 21})).padding_x(px(${clean ? 16 : 10})).padding_bottom(px(${clean ? 14 : 8}))`);
    rec("foot",`style().min_height(px(${clean ? 48 : 50})).padding_start(px(16)).padding_end(px(${clean ? 10 : 8})).padding_y(px(${clean ? 10 : 8})).border_top(px(${clean ? .5 : 0})).border_color(${srEdge})`);
    rec("signal",`style().font_size(px(${clean ? 12 : 13})).line_height(px(16)).gap_x(px(8))`);
    rec("actions",`style().gap_x(px(${clean ? 8 : 6}))`);
    rec("button",`style().min_height(px(${clean ? 28 : 32})).padding_x(px(${clean ? 10 : 12})).gap_x(px(6)).radius(px(${clean ? 6 : 10})).font_size(px(13))${shadows(srLift)}`);
    rec("primary",clean ?
      `style().background(${color("recommendation.solid")}).shadows([]).hover(style().background(${dark ? white(.82) : black(.8)}).opacity(1.0).shadows([]))` :
      `style()${srGradient(dark ? "f4f4f5" : "323137",dark ? "e4e4e7" : "201e25")}${shadows(srPrimary)}
        .hover(style()${srGradient(dark ? "ffffff" : "3b3a41",dark ? "ececef" : "26242b")}.opacity(1.0))`);
    for (const part of ["alternative_glyph","undo_glyph"]) rec(part,clean ? "style().flex_row()" : "style().hidden()");
    rec("button_text","style().font_size(px(13)).line_height(px(20)).font_weight(500)");
    rec("receipt",`style().min_height(px(28)).radius(px(${clean ? 6 : 8})).padding_x(px(${clean ? 8 : 2})).font_size(px(13)).line_height(px(20))`);
    rec("menu",`style().width(px(${clean ? 248 : 260})).radius(px(${clean ? 10 : 12})).padding(px(4)).gap_y(px(2))${shadows(srPop)}`);
    rec("option",`style().min_height(px(${clean ? 32 : 34})).padding_x(px(10)).gap_x(px(10)).radius(px(${clean ? 6 : 8}))`);
    rec("option_selected",`style()${clean ? shadows([shadow(0,0,0,.5,color("recommendation.selected_edge"),true)]) : ".shadows([])"}`);
    rec("option_name","style().font_size(px(13)).line_height(px(20))");
    rec("recommended",`style().font_size(px(${clean ? 11 : 10.5})).line_height(px(${clean ? 18 : 16})).padding_x(px(${clean ? 5 : 6})).radius(px(${clean ? 5 : 12}))`);
    const trEdge=clean ? (dark ? white(.1) : black(.06)) : rgba(dark ? "2e2e33" : "ececec");
    const trLift=clean ? smallShadow : [shadow(0,0,0,1,trEdge),shadow(0,1,2,0,black(dark ? .3 : .04))];
    const trPop=[shadow(0,0,0,clean ? .5 : 1,trEdge),...floatingCast()];
    trace("root",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).letter_spacing_em(${clean ? 0 : -0.006}).padding(px(${clean ? 0 : 4})).radius(px(${clean ? 12 : 20}))${clean ? shadows(trLift) : ""}`);
    trace("head",`style().padding_start(px(${clean ? 16 : 12})).padding_end(px(${clean ? 14 : 8})).padding_top(px(${clean ? 12 : 8})).padding_bottom(px(12)).gap_x(px(8))${clean ? ".border_bottom(px(0.5)).border_color("+trEdge+")" : ""}`);
    trace("title",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(${clean ? 500 : 600})`);
    trace("task","style().font_size(px(13)).line_height(px(20)).font_weight(400)");
    trace("status",`style().min_height(px(22)).padding_x(px(9)).gap_x(px(${clean ? 5 : 6})).radius(px(99)).font_size(px(12)).line_height(px(16)).font_weight(500)`);
    trace("head_status",`style()${clean ? ".shadows([])" : shadows(trLift)}`);
    for (const tone of ["live","done","error"]) trace("head_"+tone,`style().background(${color(clean ? "trace."+tone+"_bg" : "trace.card")})`);
    trace("card",`style().radius(px(${clean ? 12 : 16}))${clean ? ".shadows([])" : shadows(trLift)}`);
    trace("stats",`style().padding_x(px(0)).padding_y(px(${clean ? 12 : 14})).gap_x(px(0)).border_bottom(px(${clean ? .5 : 1})).border_color(${trEdge})`);
    trace("stats_narrow",`style().padding_y(px(${clean ? 10 : 12})).gap_y(px(${clean ? 10 : 12}))`);
    trace("stat_rule",`style().border_start(px(${clean ? .5 : 1})).border_color(${trEdge})`);
    trace("stat_label",clean ? "style().font_size(px(10)).line_height(px(16)).font_weight(500).letter_spacing_px(0.5)" : "style().font_size(px(12)).line_height(px(16)).font_weight(400)");
    trace("stat_label_tiny",clean ? "style().letter_spacing_px(0.2)" : "style()");
    trace("stat_value",clean ? "style().font_size(px(15)).line_height(px(24)).font_weight(500).letter_spacing_em(0)" : "style().font_size(px(16)).line_height(px(22)).font_weight(600).letter_spacing_em(-0.01)");
    trace("stat_value_narrow",clean ? "style()" : "style().font_size(px(15))");
    trace("axis_caption","style().inset_start(px(16))");
    trace("axis_label",clean ? "style().font_size(px(10)).line_height(px(16)).font_weight(500).letter_spacing_px(0.5)" : "style().font_size(px(11)).line_height(px(14)).font_weight(400)");
    trace("row",`style().min_height(px(${clean ? 30 : 32})).radius(px(${clean ? 5 : 7}))`);
    trace("tick",`style().font_size(px(${clean ? 12 : 11})).line_height(px(${clean ? 16 : 14})).font_weight(400)`);
    trace("tile",`style().radius(px(${clean ? 5 : 6})).font_size(px(10)).line_height(px(14))${shadows(trLift)}`);
    trace("name","style().font_size(px(13)).line_height(px(20))");
    trace("subtitle","style().font_size(px(12)).line_height(px(20)).font_weight(400)");
    trace("span_label_flow","style().gap_x(px(6)).items_baseline()");
    trace("bar",`style().height(px(${clean ? 14 : 16})).border(px(${clean ? .5 : 1})).radius(px(${clean ? 4 : 5}))`);
    trace("bar_narrow",`style().height(px(${clean ? 9 : 10})).radius(px(3.5))`);
    trace("readout",`style().font_size(px(${clean ? 12 : 11.5})).line_height(px(16)).font_weight(400)`);
    trace("readout_tag",`style().radius(px(${clean ? 4 : 5}))`);
    trace("slow","style().font_size(px(10)).min_height(px(16)).padding_x(px(6)).radius(px(99))");
    trace("rows","style().padding_bottom(px(12))");
    trace("details",`style().padding(px(12)).radius(px(${clean ? 10 : 12}))${shadows(trPop)}`);
    trace("detail_title","style().font_size(px(13)).line_height(px(17))");
    trace("detail_subtitle","style().font_size(px(11.5)).line_height(px(16))");
    trace("detail_stats",`style().padding_x(px(10)).padding_y(px(8)).radius(px(${clean ? 8 : 10}))${shadows([shadow(0,0,0,.5,trEdge,true)])}`);
    trace("detail_stat_label",clean ? "style().font_size(px(10)).line_height(px(16)).font_weight(500).letter_spacing_px(0.5)" : "style().font_size(px(11)).line_height(px(14)).font_weight(400)");
    trace("detail_stat_value",`style().font_size(px(13)).line_height(px(${clean ? 20 : 18})).font_weight(${clean ? 500 : 600})`);
    trace("detail_error",`style().font_size(px(12)).line_height(px(16)).padding_x(px(10)).padding_y(px(8)).radius(px(${clean ? 8 : 10}))`);
    trace("document","style().font_size(px(12)).line_height(px(17)).padding_x(px(10)).padding_y(px(8))");
    const taEdge=clean ? (dark ? white(.1) : black(.06)) : rgba(dark ? "2f2f34" : "ececec");
    const taLift=clean ? smallShadow : [shadow(0,0,0,1,taEdge),shadow(0,1,2,0,black(dark ? .4 : .04))];
    const taRise=[shadow(0,0,0,clean ? .5 : 1,dark ? white(.16) : black(.1)),shortDrop(black(dark ? .25 : .12),0,-1)];
    toolAccess("root",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 0 : 16}))`);
    toolAccess("head",`style().padding_x(px(${clean ? 2 : 10})).padding_top(px(${clean ? 0 : 7})).padding_bottom(px(${clean ? 10 : 7})).gap_x(px(8))`);
    if(clean) toolAccess("mark",`style().text_color(${color("tool_access.muted")})`);
    toolAccess("title",`style().font_size(px(${clean ? 14 : 13.5})).line_height(px(20)).font_weight(${clean ? 500 : 600})`);
    toolAccess("stats","style().font_size(px(12.5)).line_height(px(18))");
    toolAccess("body",`style().padding_x(px(${clean ? 0 : 6})).padding_top(px(${clean ? 0 : 10})).padding_bottom(px(${clean ? 0 : 6})).radius(px(${clean ? 0 : 12}))${clean ? ".background("+transparent+").shadows([])" : shadows(taLift)}`);
    toolAccess("agents","style().gap_x(px(6)).padding_x(px(1)).padding_top(px(1)).padding_bottom(px(4))");
    toolAccess("tab",`style().min_height(px(${clean ? 28 : 30})).padding_start(px(5)).padding_end(px(${clean ? 10 : 12})).gap_x(px(6)).font_size(px(${clean ? 12 : 13})).line_height(px(16)).radius(px(${clean ? 7 : 99}))`);
    toolAccess("tab_active",`style()${shadows(taLift)}`);
    toolAccess("tab_avatar",`style().width(px(${clean ? 18 : 20})).height(px(${clean ? 18 : 20})).font_size(px(9)).line_height(px(12)).font_weight(600).radius(px(${clean ? 5 : 99}))${shadows(taLift)}`);
    toolAccess("avatar",`${readingType(10,14)}.font_weight(500).radius(px(6))${shadows(taLift)}`);
    toolAccess("agent",`style().radius(px(${clean ? 10 : 12})).padding_x(px(10)).padding_top(px(10)).padding_bottom(px(9)).gap_y(px(6))${shadows(taLift)}`);
    toolAccess("person",readingType(13,16));
    toolAccess("model",readingType(11,14));
    toolAccess("task","style().font_size(px(12)).line_height(px(16))");
    toolAccess("agent_status",`style().font_size(px(${clean ? 11 : 11.5})).line_height(px(16)).font_weight(${clean ? 500 : 600})
       .min_height(px(${clean ? 18 : 20})).padding_x(px(${clean ? 6 : 8})).radius(px(${clean ? 4 : 99})).self_start()`);
    toolAccess("badge",`style().font_size(px(10.5)).line_height(px(16)).min_height(px(16)).padding_x(px(6)).radius(px(${clean ? 4 : 99}))`);
    toolAccess("tool",`style().min_height(px(46)).radius(px(${clean ? 9 : 10})).padding_start(px(8)).padding_end(px(10)).padding_y(px(4)).gap_y(px(2))${shadows(taLift)}.hover(style()${shadows(taRise)})`);
    toolAccess("blocked",`style()${clean ? ".background("+transparent+")" : ""}${shadows([shadow(0,0,0,clean ? .5 : 1,taEdge)])}`);
    toolAccess("tile",`style().radius(px(4)).font_size(px(9)).line_height(px(12))${shadows(taLift)}`);
    toolAccess("tool_name",`style().font_size(px(${clean ? 12 : 13})).line_height(px(16))`);
    toolAccess("tool_info","style().font_size(px(11)).line_height(px(16))");
    toolAccess("tool_count","style().font_size(px(11)).line_height(px(16))");
    toolAccess("panel",`style().radius(px(12)).padding_x(px(12)).padding_top(px(4)).padding_bottom(px(12))${shadows(taLift)}`);
    toolAccess("panel_head","style().min_height(px(36)).gap_x(px(6))");
    toolAccess("call",`style().min_height(px(30)).radius(px(${clean ? 5 : 7})).padding_x(px(6)).gap_x(px(8))`);
    toolAccess("detail","style().gap_y(px(7)).padding_top(px(2))");
    toolAccess("section","style().font_size(px(10)).line_height(px(16)).letter_spacing_em(0.05)");
    toolAccess("code",`style().radius(px(${clean ? 5 : 7})).padding_x(px(10)).padding_y(px(8))${shadows([shadow(0,0,0,.5,taEdge,true)])}`);
    toolAccess("button",`style().min_height(px(30)).padding_x(px(12)).radius(px(99)).font_size(px(13))${shadows(taLift)}`);
    toolAccess("primary","style().shadows([])");
    toolAccess("actions","style().gap_x(px(6))");
    const mpEdge=clean ? (dark ? white(.08) : black(.06)) : rgba(dark ? "2a2a2e" : "ececec");
    const mpLift=clean ? smallShadow : [shadow(0,0,0,1,mpEdge),shadow(0,1,2,0,black(dark ? .4 : .04))];
    memoryPanel("root",`style().max_width(px(${clean ? 400 : 380})).padding(px(${clean ? 0 : 4})).radius(px(${clean ? 0 : 20})).gap_y(px(${clean ? 4 : 0}))`);
    memoryPanel("card",`style().padding(px(0)).radius(px(${clean ? 10 : 16})).gap_y(px(0))${shadows(mpLift)}`);
    memoryPanel("head",`style().min_height(px(${clean ? 40 : 48})).padding_top(px(${clean ? 12 : 0})).padding_start(px(${clean ? 16 : 14})).padding_end(px(${clean ? 10 : 12})).gap_x(px(8))`);
    memoryPanel("title",`style().font_size(px(${clean ? 15 : 13.5})).line_height(px(${clean ? 24 : 20})).font_weight(${clean ? 500 : 600}).text_color(${clean ? (dark ? white(1) : black(1)) : color("memory_panel.ink")})`);
    memoryPanel("count",`style().min_width(px(20)).text_center().font_size(px(${clean ? 11 : 12})).font_weight(500).line_height(px(${clean ? 18 : 20})).padding_x(px(${clean ? 5 : 6})).radius(px(5)).background(${color("memory_panel.fill")}).text_color(${color(clean ? "memory_panel.text" : "memory_panel.muted")})`);
    memoryPanel("switch",`style().min_height(px(28)).padding_start(px(8)).padding_end(px(${clean ? 6 : 2})).gap_x(px(8)).border(px(0)).radius(px(${clean ? 6 : 8})).hover(style().background(${color("memory_panel.well")}))`);
    memoryPanel("track",`style().width(px(${clean ? 26 : 28})).height(px(16)).radius(px(${clean ? 5 : 8}))`);
    memoryPanel("knob",`style().width(px(12)).height(px(12)).radius(px(${clean ? 3.5 : 6}))${shadows([shadow(0,0,0,.5,black(.06)),shadow(0,1,2,0,black(.16))])}`);
    memoryPanel("switch_label",`style().font_size(px(${clean ? 13 : 12.5})).line_height(px(20))`);
    memoryPanel("notice","style().padding_x(px(10)).padding_y(px(6)).margin_x(px(10)).line_height(px(18))");
    memoryPanel("filters",`style().padding_start(px(${clean ? 16 : 12})).padding_end(px(10)).padding_top(px(${clean ? 12 : 0})).padding_bottom(px(${clean ? 12 : 10})).border_bottom(px(${clean ? .5 : 0})).border_color(${mpEdge})`);
    memoryPanel("chip",`style().height(px(${clean ? 28 : 26})).padding_x(px(${clean ? 10 : 8})).gap_x(px(${clean ? 6 : 5})).radius(px(${clean ? 6 : 8}))
       .font_size(px(${clean ? 13 : 12.5})).line_height(px(20)).font_weight(${clean ? 500 : 400}).border(px(0))
       ${clean ? shadows([shadow(0,0,0,.5,mpEdge,true)]) : ""}`);
    if(!clean)memoryPanel("chip_selected",`style()${shadows([shadow(0,0,0,1,mpEdge,true)])}`);
    memoryPanel("chip_count",`style().font_size(px(${clean ? 12 : 11.5})).line_height(px(20)).font_weight(400)`);
    memoryPanel("list",`style().padding_x(px(6)).padding_top(px(2)).padding_bottom(px(6)).border_top(px(${clean ? 0 : 1})).border_color(${mpEdge})`);
    memoryPanel("group",`${readingType(clean ? 10 : 12,clean ? 16 : 18)}.padding_x(px(${clean ? 10 : 8})).padding_top(px(${clean ? 12 : 10})).padding_bottom(px(${clean ? 4 : 2})).letter_spacing_px(${clean ? 0.5 : 0})`);
    memoryPanel("item",`style().padding_x(px(${clean ? 10 : 8})).padding_top(px(${clean ? 8 : 6})).padding_bottom(px(${clean ? 7 : 5})).radius(px(${clean ? 8 : 10}))`);
    memoryPanel("fact",readingType(13,20));
    memoryPanel("meta",`style().min_height(px(${clean ? 20 : 24})).margin_top(px(${clean ? 2 : 0})).gap_x(px(6))`);
    memoryPanel("metadata",readingType(12,clean ? 16 : 20));
    if(clean)memoryPanel("metadata_separator",`style().text_color(${dark ? white(.30) : black(.27)})`);
    memoryPanel("paused",`${readingType(11.5,18)}.padding_x(px(6)).radius(px(5))`);
    memoryPanel("plain",`style().radius(px(${clean ? 6 : 8}))`);
    memoryPanel("danger",`style().radius(px(${clean ? 6 : 8}))`);
    memoryPanel("editor",`style().radius(px(${clean ? 6 : 8})).margin_x(px(6)).padding(px(8))`);
    memoryPanel("editor_field","style().padding_x(px(0)).padding_y(px(0)).border(px(0)).shadows([]).font_size(px(13)).line_height(px(20))");
    memoryPanel("new",`style().height(px(32)).padding_x(px(${clean ? 10 : 8})).radius(px(${clean ? 6 : 10})).font_size(px(13)).line_height(px(20)).border(px(0))
      .text_color(${color(clean ? "memory_panel.text" : "memory_panel.muted")}).hover(style().background(${color("memory_panel.well")}).text_color(${color("memory_panel.ink")}))`);
    memoryPanel("bottom",`style().padding_x(px(6)).padding_top(px(${clean ? 6 : 4})).padding_bottom(px(6)).border_top(px(${clean ? .5 : 1})).border_color(${mpEdge})`);
    memoryPanel("foot",`style().min_height(px(${clean ? 44 : 38})).padding_start(px(${clean ? 16 : 10})).padding_end(px(${clean ? 8 : 4})).padding_top(px(${clean ? 0 : 4})).padding_bottom(px(0))
      .gap_x(px(8)).border_top(px(${clean ? .5 : 0})).border_color(${mpEdge})`);
    memoryPanel("caption",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(${clean ? 16 : 20})).flex_grow(1.0)`);
    memoryPanel("action",`style().height(px(${clean ? 28 : 26})).padding_start(px(${clean ? 10 : 8})).padding_end(px(${clean ? 6 : 8})).radius(px(${clean ? 6 : 10})).gap_x(px(${clean ? 4 : 5})).font_size(px(${clean ? 13 : 12.5})).line_height(px(20)).hover(style().background(${color(clean ? "memory_panel.well" : "memory_panel.card")}).text_color(${color("memory_panel.ink")}))`);
    memoryPanel("undo",`style().height(px(26)).padding_x(px(8)).radius(px(${clean ? 6 : 9})).font_size(px(12.5)).background(${color("memory_panel.card")})${shadows(mpLift)}`);
    const fcColor=(what)=>color("fallback_chain."+what);
    const fcShadow=clean ? (dark ? [shadow(0,0,0,.5,white(.10)),shadow(0,1,2,0,black(.4))] : smallShadow)
      : [shadow(0,0,0,1,rgba(dark ? "2e2e31" : "ececec")),shadow(0,1,2,0,black(dark ? .4 : .04))];
    fallbackChain("root",`style().padding(px(${clean ? 0 : 4})).max_width(px(${clean ? 740 : 748})).radius(px(${clean ? 0 : 20})).gap_y(px(${clean ? 10 : 0})).letter_spacing_em(${clean ? 0 : -.006})`);
    fallbackChain("head",`style().min_height(px(${clean ? 28 : 44})).padding_start(px(${clean ? 2 : 14})).padding_end(px(${clean ? 2 : 12})).padding_bottom(px(${clean ? 0 : 4})).gap_x(px(12))`);
    fallbackChain("title",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(${clean ? 500 : 600})`);
    fallbackChain("flow",`style().padding_x(px(16)).padding_y(px(22)).radius(px(${clean ? 12 : 16}))${shadows(fcShadow)}`);
    fallbackChain("tree","style().padding_x(px(14)).padding_y(px(16))");
    fallbackChain("attempts","style().gap_y(px(10))");
    fallbackChain("attempts_tree","style().padding_start(px(40))");
    fallbackChain("endpoint",`style().height(px(52)).padding_x(px(8)).radius(px(${clean ? 10 : 12}))${shadows(fcShadow)}`);
    fallbackChain("end_wait",`style().background(${fcColor("wait")})`);
    fallbackChain("end_failed",`style()${shadows([shadow(0,0,0,1,fcColor("error_edge")),...fcShadow])}`);
    fallbackChain("end_answered",`style()${shadows([shadow(0,0,0,1.5,fcColor("ok_edge")),shadow(0,0,0,5,fcColor("ok_fill"))])}`);
    fallbackChain("attempt",`style().height(px(60)).padding_start(px(10)).padding_end(px(12)).radius(px(${clean ? 10 : 12}))${shadows(fcShadow)}`);
    fallbackChain("attempt_trying",`style()${shadows([shadow(0,0,0,1.5,fcColor("active_edge")),shadow(0,0,0,5,fcColor("active_fill"))])}`);
    fallbackChain("attempt_answered",`style()${shadows([shadow(0,0,0,1.5,fcColor("ok_edge")),shadow(0,0,0,5,fcColor("ok_fill"))])}`);
    fallbackChain("attempt_skipped","style().opacity(0.5).isolated_opacity(true)");
    fallbackChain("logo","style().radius(px(7)).font_size(px(10)).line_height(px(20)).font_weight(500).letter_spacing_px(0.3)");
    fallbackChain("number","style().font_size(px(8.5)).line_height(px(13)).font_weight(500)");
    for(const part of ["state","sum","node_sub","detail_value","detail_label","popup_sub"])
      fallbackChain(part,`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(16))`);
    for(const part of ["attempt_status","cost"]) fallbackChain(part,`style().font_size(px(${clean ? 11.5 : 12})).line_height(px(16))`);
    fallbackChain("model",`style().font_size(px(${clean ? 12.5 : 13})).line_height(px(${clean ? 17 : 18})).font_weight(${clean ? 500 : 600})`);
    fallbackChain("provider",`style().font_size(px(${clean ? 11 : 12})).line_height(px(16))`);
    fallbackChain("timing",`style().font_size(px(${clean ? 11.5 : 12})).line_height(px(${clean ? 17 : 18})).font_weight(${clean ? 400 : 500})`);
    for(const part of ["node_title","popup_title"])fallbackChain(part,`style().font_size(px(${clean ? 12 : 13})).line_height(px(${clean ? 16 : 18})).font_weight(600)`);
    fallbackChain("route","style().font_size(px(10)).line_height(px(16)).letter_spacing_px(-0.1)");
    fallbackChain("retry",`style().height(px(28)).padding_x(px(${clean ? 10 : 12})).radius(px(${clean ? 6 : 9})).background(${fcColor("card")})${shadows(fcShadow)}`);
    fallbackChain("popup",`style().radius(px(${clean ? 10 : 16})).padding(px(12)).background(${fcColor("popover")})${shadows(fcShadow)}`);
    fallbackChain("badge","style().font_size(px(12)).line_height(px(20)).padding_x(px(8))");
    fallbackChain("quote",`style().font_size(px(13)).line_height(px(19)).padding_x(px(11)).padding_y(px(9)).radius(px(${clean ? 7 : 10}))`);
    const cursors=(part,value)=>set("patterns/agent_cursors",part,value);
    const cursorCard=color("agent_cursors.card"), cursorTray=color("agent_cursors.tray"), cursorMuted=color("agent_cursors.muted");
    const cursorFaceSize=clean ? 24 : 28;
    const cursorFaceShadows=[
      shadow(0,0,0,clean ? 1.5 : 2,clean ? cursorCard : cursorTray),
      shadow(0,0,0,clean ? 1 : 1.5,color("agent_cursors.face_edge"),true),
    ];
    cursors("root",`style().gap(px(6)).padding(px(0)).border(px(0)).background(${transparent}).letter_spacing_em(${clean ? 0 : -.006})`);
    cursors("sheet",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 12 : 20})).background(${clean ? cursorCard : cursorTray})${shadows(clean ? smallShadow : [])}`);
    cursors("header",`style().min_height(px(${clean ? 44 : 48})).padding_x(px(${clean ? 14 : 16})).padding_y(px(4)).gap(px(8)).border_bottom(px(${clean ? .5 : 0})).border_color(${edge})`);
    cursors("title",`style().font_size(px(${clean ? 13 : 13.5})).line_height(px(20)).font_weight(500).text_color(${cursorMuted})`);
    cursors("summary",`style().font_size(px(12.5)).line_height(px(17)).font_feature("tnum",1).text_color(${cursorMuted})`);
    cursors("presence",`style().gap(px(${clean ? 3 : 4})).flex_shrink(false)`);
    cursors("presence_face",`style().padding(px(0)).width(px(${cursorFaceSize})).height(px(${cursorFaceSize})).min_height(px(${cursorFaceSize})).radius(px(99))${shadows(cursorFaceShadows)}`);
    cursors("presence_more",`style().width(auto()).min_width(px(24)).padding_x(px(5)).font_size(px(10.5)).line_height(px(10.5)).font_weight(600).letter_spacing_em(0).font_feature("tnum",1).text_color(${cursorMuted})`);
    cursors("presence_following","style().relative().top(offset_px(-1))");
    cursors("tabs",`style().gap(px(6))`);
    const cursorTabInk=color("agent_cursors.tab_ink");
    cursors("tab",`style().min_height(px(${clean ? 28 : 30})).padding_x(px(${clean ? 10 : 12})).radius(px(${clean ? 6 : 9})).font_size(px(13)).line_height(px(18)).font_weight(500).text_color(${cursorMuted}).background(${color("agent_cursors.tab")}).hover(style().text_color(${cursorTabInk}))`);
    cursors("tab_active",`style().background(${cursorCard}).text_color(${color("agent_cursors.tab_ink")})${shadows(smallShadow)}`);
    cursors("stage",`style().radius(px(${clean ? 12 : 16}))`);
    cursors("page",`style().padding_x(px(${clean ? 44 : 48})).padding_top(px(${clean ? 32 : 36})).padding_bottom(px(${clean ? 44 : 48})).gap(px(${clean ? 12 : 14})).background(${cursorCard})`);
    cursors("flow",`style().gap_x(px(24)).gap_y(px(${clean ? 12 : 14}))`);
    cursors("page_narrow",`style().padding_x(px(${clean ? 18 : 20})).padding_top(px(${clean ? 22 : 24})).padding_bottom(px(${clean ? 30 : 32}))`);
    cursors("doc_title",`style().font_size(px(${clean ? 20 : 22})).line_height(px(28)).font_weight(${clean ? 500 : 650}).letter_spacing_em(${clean ? -.01 : -.022}).text_color(${strongest})`);
    cursors("heading",`style().font_size(px(${clean ? 14.5 : 15.5})).line_height(px(${clean ? 22 : 24})).font_weight(${clean ? 500 : 600}).letter_spacing_em(${clean ? -.01 : -.012}).text_color(${strongest})`);
    for(const part of ["paragraph","item"]){
      cursors(part,`style().font_size(px(${clean ? 13.5 : 14})).line_height(px(${clean ? 22 : 23})).text_color(${ink})`);
    }
    cursors("code",`style().padding_x(px(${clean ? 14 : 16})).padding_y(px(${clean ? 12 : 14})).radius(px(${clean ? 8 : 12})).background(${wash}).font_size(px(12.5)).line_height(px(20)).letter_spacing_em(0)${shadows([hairline])}`);
    cursors("image",`style().radius(px(${clean ? 8 : 12})).background(${wash})`);
    cursors("caption",`style().font_size(px(12)).line_height(px(17)).text_color(${muted})`);
    cursors("table",`style().radius(px(${clean ? 8 : 12})).border(px(${clean ? .5 : 1})).border_color(${edge}).background(${panel})`);
    cursors("table_header",`style().min_height(px(${clean ? 34 : 40})).items_center().background(${wash}).border_bottom(px(${clean ? .5 : 1}))`);
    cursors("table_row",`style().min_height(px(${clean ? 34 : 40})).items_center().border_bottom(px(${clean ? .5 : 1}))`);
    cursors("cell",`style().font_size(px(${clean ? 13 : 13.5})).line_height(px(20))`);
    cursors("tone",`style().radius(px(99)).padding_x(px(8)).font_size(px(12)).line_height(px(20)).font_weight(500).self_start().max_width(relative(1.0))`);
    cursors("add_row",`style().min_height(px(${clean ? 28 : 30})).padding_x(px(10)).font_size(px(13)).line_height(px(18)).text_color(${muted})`);
    const flagSurface=rgba(dark ? "f4f4f5" : "141416");
    const flagInk=rgba(dark ? "141416" : "ffffff");
    const flagEdge=`#{angle:90,stops:[#{offset:0,color:${rgba("f06db5")}},#{offset:0.55,color:${rgba("b98af0")}},#{offset:1,color:${rgba("4fb0fe")}}]}`;
    cursors("flag",`style().radius(px(${clean ? 8 : 10})).padding_x(px(${clean ? 9 : 10})).padding_y(px(${clean ? 5 : 6})).border(px(0)).border_bottom(px(1)).border_color(${transparent})${layeredGradient([gradientLayer([flagSurface,flagSurface],"padding"),flagEdge])}${shadows([shortDrop(rgba("ee3e9e40"),0,-1),shortDrop(rgba("0190fe40"),2,-1),shortDrop(black(.24),0,-1)])}`);
    cursors("flag_name",`style().font_size(px(12)).line_height(px(16)).font_weight(${clean ? 500 : 600}).text_color(${flagInk})`);
    cursors("flag_activity",`style().font_size(px(11.5)).line_height(px(16)).text_color(alpha(${flagInk},0.88))`);
    cursors("pin",`style().radius(px(${clean ? 6 : 8})).background(${panel}).text_color(${ink})${shadows(smallShadow)}`);
    cursors("popup",`style().padding(px(10)).gap(px(6)).radius(px(${clean ? 9 : 14})).border(px(0)).background(${panel})${shadows(floatingShadow)}`);
    cursors("popup_title",`style().font_size(px(13)).line_height(px(18)).font_weight(${clean ? 500 : 600}).text_color(${strongest})`);
    cursors("popup_text",`style().font_size(px(12.5)).line_height(px(19)).text_color(${ink})`);
    const di=(part,value)=>set("patterns/decision_inbox",part,value);
    di("root",`style().padding(px(${clean ? 0 : 6})).radius(px(${clean ? 10 : 22})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    di("head",`style().min_height(px(${clean ? 44 : 41})).padding_x(px(${clean ? 16 : 18})).gap(px(8)).border_bottom(px(${clean ? .5 : 0})).border_color(${edge})`);
    di("count",`style().font_size(px(${clean ? 13 : 14})).font_weight(500)${clean ? `.text_color(${ink})` : ""}`);
    di("card",`style().radius(px(${clean ? 0 : 16})).background(${clean ? transparent : panel})${shadows(clean ? [] : smallShadow)}`);
    di("row",`style().padding_x(px(${clean ? 16 : 20})).padding_y(px(${clean ? 12 : 14})).gap(px(${clean ? 6 : 8})).border_top(px(${clean ? .5 : 1})).border_color(${edge})`);
    di("first",`style().border_top(px(0))`);
    di("closed",`style().padding_y(px(${clean ? 12 : 14})).text_color(${faint}).radius(px(0))`);
    di("title",`style().font_size(px(${clean ? 13 : 14})).font_weight(500).line_height(px(20))`);
    di("primary",`style().radius(px(${clean ? 6 : 9}))`);
    di("secondary",`style().radius(px(${clean ? 6 : 9}))`);
    const gates=(part,value)=>set("patterns/approval_gates",part,value);
    gates("root",`style().gap(px(${clean ? 8 : 10}))`);
    for(const part of ["summary","front"]){
      gates(part,`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 10 : 17})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    }
    gates("bar",`style().min_height(px(${clean ? 36 : 38})).padding_x(px(${clean ? 12 : 9})).gap(px(8))`);
    gates("title",`style().font_size(px(${clean ? 13 : 14})).font_weight(500)`);
    gates("task",`style().font_size(px(12)).text_color(${muted})`);
    gates("status",`style().background(${wash}).padding_x(px(7)).radius(px(5)).font_size(px(11.5))`);
    gates("stats",`style().padding_x(px(${clean ? 6 : 4})).padding_y(px(2)).gap(px(5)).radius(px(${clean ? 0 : 12})).background(${panel})`);
    gates("stat",`style().padding_x(px(7)).padding_y(px(5))`);
    gates("stat_label",`style().font_size(px(11)).line_height(px(16)).text_color(${muted})`);
    gates("stat_value",`style().font_size(px(13)).line_height(px(19)).font_feature("tnum",1)`);
    gates("body",`style().padding(px(${clean ? 12 : 13})).gap(px(${clean ? 7 : 9})).radius(px(${clean ? 0 : 13})).background(${panel})${shadows(clean ? [] : smallShadow)}`);
    gates("peek",`style().background(${clean ? wash : color("cite.tray")}).radius(px(${clean ? 10 : 14}))${shadows(smallShadow)}`);
    gates("peek_far",`style().background(${clean ? panel : wash})`);
    gates("avatar",`style().width(px(${clean ? 28 : 32})).height(px(${clean ? 28 : 32})).radius(px(${clean ? 6 : 8})).background(${wash})`);
    gates("agent",`style().font_size(px(13)).line_height(px(20)).font_weight(500)`);
    gates("kind",`style().padding_x(px(5)).radius(px(5)).background(${wash}).font_size(px(11.5))`);
    gates("question",`style().font_size(px(${clean ? 14 : 15})).line_height(px(22)).font_weight(500)`);
    gates("reason",`style().font_size(px(${clean ? 12 : 12.5})).line_height(px(18)).text_color(${muted})`);
    gates("section_title",`style().font_size(px(12)).line_height(px(18)).font_weight(500)`);
    gates("more",`style().min_height(px(28)).radius(px(${clean ? 5 : 7}))`);
    gates("actions",`style().padding_x(px(${clean ? 12 : 4})).padding_y(px(${clean ? 8 : 4})).border_top(px(${clean ? .5 : 0})).border_color(${edge})`);
    for(const part of ["approve","reject","edit"]){gates(part,`style().min_height(px(${clean ? 29 : 33})).radius(px(${clean ? 6 : 8}))`);}
    gates("history",`style().padding(px(${clean ? 10 : 9})).radius(px(${clean ? 10 : 15})).background(${panel})${shadows(smallShadow)}`);
    gates("receipt",`style().min_height(px(${clean ? 31 : 34})).padding_x(px(6)).radius(px(${clean ? 5 : 7}))`);
    gates("holds",`style().padding(px(8)).radius(px(${clean ? 8 : 12})).background(${clean ? transparent : color("cite.tray")})`);
    gates("hold",`style().font_size(px(12)).text_color(${muted})`);
    const dial=(part,value)=>set("patterns/autonomy_dial",part,value);
    dial("root",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 10 : 20})).background(${clean ? panel : color("cite.tray")})${shadows(clean ? smallShadow : [])}`);
    dial("card",`style().padding(px(${clean ? 12 : 14})).gap(px(10)).radius(px(${clean ? 0 : 16})).background(${panel})${shadows(clean ? [] : smallShadow)}`);
    dial("header",`style().gap(px(${clean ? 10 : 12}))`);
    dial("mark",`style().width(px(30)).height(px(30)).radius(px(${clean ? 6 : 8})).background(${wash})`);
    dial("title",`style().font_size(px(${clean ? 14 : 15})).line_height(px(21)).font_weight(500)`);
    dial("subtitle",`style().font_size(px(12)).line_height(px(18)).text_color(${muted})`);
    dial("levels",`style().gap(px(${clean ? 6 : 3})).padding(px(${clean ? 0 : 3})).radius(px(${clean ? 6 : 12})).background(${clean ? transparent : color("selection")})`);
    dial("groups",`style().padding(px(${clean ? 10 : 6})).gap(px(${clean ? 3 : 5})).border_top(px(${clean ? .5 : 0})).border_color(${edge})`);
    dial("group_head",`style().padding_x(px(7)).padding_y(px(${clean ? 4 : 6})).gap(px(6))`);
    dial("group_title",`style().font_size(px(12)).line_height(px(18)).font_weight(500).text_color(${muted})`);
    dial("count",`style().font_size(px(11)).background(${clean ? wash : panel}).text_color(${muted})`);
    dial("row",`style().min_height(px(${clean ? 32 : 42})).padding_start(px(${clean ? 6 : 10})).padding_y(px(${clean ? 2 : 4})).gap(px(7)).radius(px(${clean ? 6 : 12})).background(${panel})${shadows(clean ? [] : smallShadow)}`);
    dial("label",`style().font_size(px(13)).line_height(px(19))`);
    dial("detail",`style().font_size(px(12)).line_height(px(16)).text_color(${muted})`);
    dial("empty",`style().padding_x(px(8)).padding_y(px(5)).font_size(px(12)).text_color(${muted})`);
    dial("risk",`style().font_size(px(10.5)).padding_x(px(5)).radius(px(4))`);
    dial("risk_medium",`style().text_color(${color("badge.warning_fg")}).background(${color("badge.warning_bg")})`);
    dial("risk_high",`style().text_color(${color("badge.danger_fg")}).background(${color("badge.danger_bg")})`);
    for (const module of ["approval_card","permission_request"]) {
      const p=(part,value)=>set(`patterns/${module}`,part,value);
      const access=module==="permission_request";
      const tray=clean && access ? panel : color("cite.tray");
      const skin=clean && access ? smallShadow : [];
      p("root",`style().padding(px(${clean && access ? 0 : 4})).radius(px(${clean ? access ? 10 : 18 : 20})).background(${tray})${shadows(skin)}`);
      p("surface",`style().padding(px(${clean && access ? 14 : 16})).gap(px(${clean ? 7 : 9})).radius(px(${clean && access ? 0 : 16})).background(${clean && access ? transparent : panel})${shadows(clean && access ? [] : smallShadow)}`);
      p("header",`style().gap(px(${clean ? 9 : 12}))`);
      p("mark",`style().width(px(${clean ? 34 : 38})).height(px(${clean ? 34 : 38})).radius(px(${access ? 999 : clean ? 7 : 10})).background(${wash})`);
      p("title",`style().font_size(px(${clean ? 14 : 15})).line_height(px(21)).font_weight(500)`);
      p("description",`style().font_size(px(${clean ? 12 : 13})).line_height(px(18)).text_color(${muted})`);
      p("risk",`style().padding_x(px(7)).padding_y(px(2)).radius(px(${clean ? 5 : 7})).font_size(px(11.5))`);
      p("risk_low",`style().background(${wash}).text_color(${muted})`);
      p("risk_medium",`style().background(${color("badge.warning_bg")}).text_color(${color("badge.warning_fg")})`);
      p("risk_high",`style().background(${color("badge.danger_bg")}).text_color(${color("badge.danger_fg")})`);
      p("receipt",`style().padding_x(px(${clean ? 12 : 16})).padding_y(px(${clean ? 9 : 11})).min_height(px(${clean ? 56 : 60})).radius(px(${clean ? 10 : 16})).background(${panel})${shadows(clean && access ? [] : smallShadow)}`);
      p("receipt_mark",`style().width(px(${clean ? 26 : 30})).height(px(${clean ? 26 : 30})).radius(px(${clean ? 6 : 8}))`);
      p("receipt_ok",`style().background(${color("badge.success_bg")}).text_color(${color("text.success")})`);
      p("receipt_no",`style().background(${wash}).text_color(${muted})`);
      p("receipt_expired",`style().background(${wash}).text_color(${muted})`);
      p("receipt_title",`style().font_size(px(${clean ? 13 : 14})).line_height(px(20)).font_weight(500)`);
      p("receipt_note",`style().font_size(px(12)).line_height(px(16)).text_color(${muted})`);
      if (access) {
        p("reason",`style().margin_start(px(${clean ? 44 : 50})).font_size(px(${clean ? 12 : 13})).line_height(px(18)).text_color(${muted})`);
        p("preview",`style().padding(px(${clean ? 6 : 8})).border_top(px(${clean ? .5 : 1})).border_color(${edge})`);
        p("preview_count",`style().font_feature("tnum",1).text_color(${muted})`);
        p("preview_row",`style().min_height(px(${clean ? 26 : 29})).padding_x(px(4)).gap(px(8))`);
        p("answer",`style().padding(px(${clean ? 14 : 8})).gap(px(${clean ? 9 : 11})).border_top(px(${clean ? .5 : 0})).border_color(${edge})`);
        p("scopes",`style().gap(px(${clean ? 6 : 3})).padding(px(${clean ? 0 : 3})).radius(px(${clean ? 6 : 12})).background(${clean ? transparent : color("selection")})`);
        p("scope_option",`style().flex_grow()`);
        p("deny",`style().min_height(px(${clean ? 34 : 40})).radius(px(${clean ? 6 : 11})).flex_grow()`);
        p("allow",`style().min_height(px(${clean ? 34 : 40})).radius(px(${clean ? 6 : 11})).flex_grow()`);
        p("allow_destructive",`style().background(${color("text.danger")}).text_color(${strongInk(color("text.danger"))}).hover(style().background(${color("text.danger")}))`);
        p("time_track",`style().height(px(2)).radius(px(4))`);
      } else {
        p("label",`style().padding_x(px(10)).padding_y(px(3)).font_size(px(11.5)).text_color(${muted})`);
        p("detail",`style().padding_x(px(8)).padding_y(px(3)).gap(px(10)).radius(px(${clean ? 5 : 7})).background(${wash})`);
        p("detail_label",`style().min_width(px(66)).font_size(px(12)).line_height(px(18))`);
        p("detail_value",`style().font_size(px(13)).line_height(px(18))`);
        p("footer",`style().min_height(px(42)).padding_start(px(9)).padding_end(px(2)).padding_y(px(4))`);
        p("footer_inside",`style().padding_x(px(0)).border_top(px(${clean ? .5 : 0})).border_color(${edge})`);
        p("timer",`style().font_size(px(12.5)).text_color(${muted})`);
        p("timer_urgent",`style().text_color(${color("text.warning")})`);
        for(const name of ["approve","deny","edit"]){p(name,`style().min_height(px(${clean ? 30 : 33})).radius(px(${clean ? 6 : 8}))`);}
      }
    }
    cset("input","placeholder",`style().text_color(${placeholder}).font_weight(400)`);
    cset("number_input","root",`style().gap(px(${clean ? 6 : 7}))`);
    const numberHover = clean
      ? [shadow(0,0,0,.5,edgeStrong),shadow(0,1,1,-1,black(.12)),shadow(0,1,2,0,black(.06))]
      : [shadow(0,0,0,1,edgeStrong),shadow(0,1,2,0,black(.04))];
    cset("number_input","field",`style().min_height(${length("metrics.field_h")}).padding_start(${length("metrics.field_pad")}).padding_end(px(${clean ? 4 : 5})).radius(${length("metrics.field_radius")}).border(px(0)).background(${field})${t("control")}${shadows(fieldShadow)}.hover(style()${shadows(numberHover)}).focus_within(style()${shadows(focusShadow)})`);
    const numberError = (token) => [shadow(0,0,0,1,color(token)),shadow(0,1,2,0,black(clean ? .05 : .04))];
    cset("number_input","field_invalid",`style().background(${color("number.error_fill")})${shadows(numberError("number.error_edge"))}.hover(style()${shadows(numberError("number.error_edge"))}).focus_within(style()${shadows(numberError("number.error_focus"))})`);
    const numberRead = shadows([hairline]);
    cset("number_input","field_read_only",`style().background(${color("number.read_only")})${numberRead}.hover(style()${numberRead}).focus_within(style()${numberRead})`);
    cset("number_input","input",`style()${t("control")}.font_weight(400).text_color(${ink})`);
    cset("number_input","label",`style()${t("label")}.font_weight(500).text_color(${strongest})`);
    cset("number_input","step",`style().width(${length("metrics.number_step")}).height(${length("metrics.number_step")}).radius(px(${clean ? 4 : 6})).text_color(${faint}).hover(style().background(${color("number.step_hover")}).text_color(${ink})).active(style().background(${color("number.step_press")})).focus_ring(px(2),offset_px(-2),${focus})`);
    cset("number_input","placeholder",`style().text_color(${placeholder})`);
    for (const part of ["prefix","unit"]) cset("number_input",part,`style()${t("control")}.text_color(${faint})`);
    for (const part of ["optional","message","error"]) cset("number_input",part,`style()${t("caption")}.text_color(${part==="error" ? color("danger") : faint})`);
    const otpRadius=clean ? 8 : 11.52;
    const otpBorder=shadow(0,0,0,clean ? .5 : 1,edge,!clean);
    cset("otp_input","root",`style().gap(px(10))`);
    cset("otp_input","editor",`style().padding_y(px(${clean ? 3 : 0}))`);
    cset("otp_input","cell",`style().width(px(${clean ? 40 : 48})).height(px(${clean ? 44 : 56.64})).gap(px(8)).radius(px(${otpRadius})).font_size(px(${clean ? 15 : 22.08})).border(px(0)).background(${color("otp.empty")})${shadows(clean ? smallShadow : [otpBorder])}`);
    cset("otp_input","filled",`style().background(${field})${shadows(clean ? smallShadow : [shadow(0,0,0,1,edgeStrong,true)])}`);
    cset("otp_input","active",`style().radius(px(${otpRadius})).border(px(0))${shadows([
      shadow(0,0,0,clean ? .5 : 1.5,color("otp.focus_line"),!clean),shadow(0,0,0,clean ? 2 : 4,color("otp.focus_halo"))
    ])}`);
    cset("otp_input","separator",`style().width(px(${clean ? 8 : 10})).height(px(${clean ? 1 : 2})).text_color(${color("otp.separator")})`);
    cset("otp_input","caret",`style().text_color(${color("otp.focus_line")})`);
    cset("otp_input","verifying",`style().background(${color("otp.wait")})`);
    cset("otp_input","invalid",`style()${clean ? `.background(${color("otp.invalid_tint")})` : ""}${shadows([shadow(0,0,0,1,`alpha(${color("otp.invalid_line")},0.6)`,!clean)])}`);
    cset("otp_input","invalid_active",`style()${shadows([shadow(0,0,0,1.5,color("otp.invalid_line"),!clean),shadow(0,0,0,clean ? 2 : 4,color("otp.invalid_tint"))])}`);
    cset("otp_input","success",`style().background(${clean ? color("otp.success_tint") : field})${shadows(clean ? smallShadow : [shadow(0,0,0,1,edgeStrong,true)])}`);
    cset("otp_input","resend_hint",`style().text_color(${faint})`);
    cset("otp_input","resend",`style().radius(px(${clean ? 6 : 4})).padding_x(px(${clean ? 7 : 2})).text_color(${color("accent")}).hover(style().background(${clean ? wash : transparent})).focus_ring(px(2),px(2),${focus})`);
    cset("input_group","root",`style().min_height(${length("metrics.field_h")}).padding_x(${length("metrics.field_pad")}).gap(px(6)).radius(${length("metrics.field_radius")}).border(px(0)).background(${field})${shadows(fieldShadow)}.focus_within(style()${shadows(focusShadow)})`);
    for (const part of ["prefix","suffix"]) cset("input_group",part,`style().text_color(${faint}).items_center().justify_center()`);
    cset("textarea","root",`style().gap(px(4))`);
    cset("textarea","editor",`style().padding_x(px(${clean ? 10 : 12})).padding_y(px(${clean ? 7 : 8})).radius(px(${clean ? 8 : 10})).border(px(0)).background(${field})${t("textarea")}${shadows(fieldShadow)}.focus(style()${shadows(focusShadow)})`);
    cset("textarea","placeholder",`style().text_color(${placeholder})`);
    cset("textarea","counter",`style()${t("caption")}.text_color(${placeholder})`);
    cset("textarea","limit",`style().text_color(${color("field.counter_full")})`);
    for (const name of ["label","form_field"]) {
      cset(name,"root",`style().gap(px(${clean ? 6 : 7}))`);
      cset(name,name === "label" ? "text" : "label",`style()${t("label")}.font_weight(500).text_color(${ink})`);
      cset(name,"description",`style()${t("hint")}.text_color(${muted})`);
    }
    cset("form_field","error",`style()${t("hint")}.text_color(${color("danger")})`);
    cset("label","required",`style().text_color(${color("danger")})`);
    cset("badge","root",`style().padding_x(${length("metrics.badge_pad")}).gap(${length("metrics.badge_gap")}).radius(${length("metrics.badge_radius")})${t("badge")}.font_weight(500).letter_spacing_px(0)`);
    cset("badge","lamp",`style().width(px(6)).height(px(6)).margin_x(px(1)).radius(px(999))`);
    cset("badge","icon",`style().width(${length("metrics.badge_icon")}).height(${length("metrics.badge_icon")}).flex_shrink(false)`);
    cset("tag","root",`style().padding(px(0)).gap(px(0)).min_height(${length("metrics.tag_h")}).radius(${length("metrics.tag_radius")}).background(${color("tag.chip")}).border(px(0))${t("tag")}.font_weight(400).disabled(style().opacity(0.55))${shadows([shadow(0,0,0,clean ? .5 : 1,color("tag.edge"),true)])}`);
    cset("tag","facet",`style().radius_start(${length("metrics.tag_radius")}).radius_end(px(0)).padding_x(${length("metrics.tag_pad")}).min_height(${length("metrics.tag_h")}).font_weight(500)`);
    cset("tag","value",`style().gap(${length("metrics.tag_gap")}).padding_start(${length("metrics.tag_pad")})`);
    cset("tag","close",`style().width(${length("metrics.tag_close")}).height(${length("metrics.tag_close")}).padding(px(0)).radius(px(4)).border(px(0)).background(${transparent}).text_color(${faint}).hover(style().background(${color("tag.facet")}).text_color(${ink})).focus_ring(px(2),offset_px(-2),${focus})`);
    cset("tag","close_icon",`style().width(${length("metrics.tag_icon")}).height(${length("metrics.tag_icon")}).flex_shrink(false)`);
    cset("avatar","root",`style().radius(px(999)).background(${wash}).border(px(0))`);
    cset("avatar","initials",`style()${t("label")}.font_weight(500)`);
    cset("avatar","presence",`style().radius(px(999)).border(px(2)).border_color(${canvas})`);
    cset("checkbox","root",`style().min_height(${length("metrics.choice_line")}).items_start().gap(px(10))${t("choice")}.font_weight(400).disabled(style().opacity(0.5))`);
    cset("checkbox","control",`style().width(${length("metrics.mark")}).height(${length("metrics.choice_line")}).min_height(${length("metrics.mark")}).flex_shrink(false).items_center().justify_center()`);
    cset("checkbox","box",`style().width(${length("metrics.mark")}).height(${length("metrics.mark")}).flex_shrink(false).radius(${length("metrics.checkbox_radius")}).border(px(0)).group_focus(style().focus_ring(px(2),px(2),${focus}))`);
    cset("radio","root",`style().min_height(${length("metrics.choice_line")}).items_start().gap(px(10))${t("choice")}.font_weight(400).disabled(style().opacity(0.5))`);
    cset("radio","control",`style().width(${length("metrics.mark")}).height(${length("metrics.choice_line")}).min_height(${length("metrics.mark")}).flex_shrink(false).items_center().justify_center()`);
    cset("radio","indicator",`style().width(${length("metrics.mark")}).height(${length("metrics.mark")}).flex_shrink(false).radius(px(999)).border(px(0))`);
    cset("radio","chip",`style().min_height(px(${clean ? 27 : 30})).padding_x(px(9)).radius(px(${clean ? 6 : 9})).font_size(px(13)).font_weight(500).background(${clean ? wash : transparent}).text_color(${muted}).hover(style().text_color(${ink}))`);
    cset("radio","chip_selected",`style().background(${clean ? selected : panel}).text_color(${ink})${shadows(clean ? [] : smallShadow)}`);
    cset("radio_group","root",`style().gap(px(8))`);
    cset("switch","root",`style().min_height(${length("metrics.choice_line")}).items_start().gap(px(10))${t("choice")}.font_weight(400).disabled(style().opacity(0.5))`);
    cset("switch","root_end",`style().width(relative(1)).justify_between().gap(px(16))`);
    cset("switch","text",`style().min_width(px(0)).gap(px(0))`);
    cset("switch","description",`style()${t("hint")}.text_color(${color("choice.description")})`);
    cset("switch","control",`style().width(${length("metrics.switch_w")}).height(${length("metrics.choice_line")}).min_height(${length("metrics.switch_h")}).flex_shrink(false).items_center().justify_center()`);
    cset("switch","track",`style().width(${length("metrics.switch_w")}).height(${length("metrics.switch_h")}).padding(px(2)).border(px(0)).radius(px(999)).group_focus(style().focus_ring(px(2),px(2),${focus}))`);
    cset("switch","thumb",`style().width(${length("metrics.switch_thumb")}).height(${length("metrics.switch_thumb")}).radius(px(999))${shadows(switchThumbShadow)}`);
    cset("card","root",`style().padding(${length(clean ? "metrics.card_pad" : "metrics.card_tray_pad")}).gap(px(${clean ? 12 : 0})).radius(${length("metrics.card_outer_radius")}).background(${color("card.tray")}).border(px(0))${t("body")}.text_color(${color("card.body")})${shadows(clean ? cardShadow : [])}`);
    cset("card","surface",`style().padding(${clean ? "px(0)" : length("metrics.card_pad")}).gap(px(12)).radius(${clean ? "px(0)" : length("metrics.card_inner_radius")}).background(${clean ? transparent : panel})${shadows(clean ? [] : cardShadow)}`);
    cset("card","header",`style()${t("subtitle")}.font_weight(500).letter_spacing_em(${clean ? 0 : -0.01}).text_color(${strongest})`);
    cset("card","content",`style()${t("body")}.text_color(${color("card.body")})`);
    cset("card","footer",`style().flex_row().flex_wrap().items_center().gap(px(8)).min_height(px(${clean ? 28 : 40})).padding_top(px(${clean ? 0 : 4})).padding_start(px(${clean ? 0 : 16})).padding_end(px(${clean ? 0 : 2}))${t("card_footer")}.text_color(${clean ? muted : faint})`);
    const floatingPanel = `style().padding(px(${clean ? 14 : 16})).radius(px(${clean ? 12 : 16})).border(px(0)).background(${panel})${t("body")}.text_color(${ink})${shadows(floatingShadow)}`;
    cset("popover","content",floatingPanel);
    cset("hover_card","root","style().min_width(px(0))");
    cset("hover_card","trigger","style()");
    cset("hover_card","content",`style().max_width(px(320)).padding(px(${clean ? 14 : 16})).radius(px(${clean ? 12 : 16})).border(px(0)).background(${panel})${t("hover_card")}.text_color(${ink})${shadows(floatingShadow)}`);
    const tooltipShadows = [
      ...(!clean && dark ? [shadow(0,0,0,1,color("tooltip.shadow_secondary"))] : []),
      ...floatingCast(color("tooltip.shadow")),
    ];
    cset("tooltip","content",`style().max_width(px(240)).padding_x(px(${clean ? 8 : 9})).padding_y(px(${clean ? 4 : 5})).gap(px(${clean ? 6 : 8})).radius(${length("metrics.tooltip_radius")}).border(px(0)).background(${color("tooltip.surface")})${clean ? ".backdrop_blur(8)" : ""}${t("tooltip")}.font_weight(500).letter_spacing_em(${clean ? 0 : -0.005}).text_color(${color("tooltip.ink")})${shadows(tooltipShadows)}${clean ? ".items_baseline()" : ".items_center()"}`);
    // Logical end-inset compensation is equivalent to the key's optical -3px
    // outer margin while staying theme/scaling aware and mirroring in RTL.
    cset("tooltip","content_with_shortcut",`style().padding_end(px(${clean ? 8 : 6}))`);
    cset("tooltip","shortcut",`style().padding_x(px(${clean ? 0 : 5})).radius(px(${clean ? 0 : 4})).background(${color("tooltip.key")}).opacity(1.0).font_feature("tnum",1).whitespace_nowrap()${t("key_legend_tooltip")}.letter_spacing_em(0.02).text_color(${color("tooltip.muted")})`);
    for (const name of ["dialog","sheet"]) {
      cset(name,"panel",`style().padding(px(${clean ? 20 : 24})).gap(px(16)).radius(px(${clean ? 16 : 20})).border(px(0)).letter_spacing_em(${clean ? 0 : -0.006}).background(${panel})${shadows(modalShadow)}`);
      cset(name,"title",`style()${t("title")}.font_weight(500).letter_spacing_em(${clean ? -0.005 : -0.01}).text_color(${strongest})`);
      cset(name,"content",`style()${t("body")}.text_color(${ink})`);
      cset(name,"actions",`style().gap(px(8))`);
      cset(name,"backdrop",`style().background(alpha(rgb(0x000000),${dark ? .6 : .35}))`);
    }
    cset("alert_dialog","description",`style()${t("body")}.text_color(${muted})`);
    cset("alert_dialog","actions",`style().gap(px(8))`);
    for (const name of ["menu","context_menu"]) {
      cset(name,"content",`style().padding(px(4)).gap(px(2)).radius(px(${clean ? 10 : 12})).border(px(0)).background(${panel})${shadows(floatingShadow)}`);
      cset(name,"item",`style().min_height(px(${clean ? 28 : 32})).padding_x(px(8)).gap(px(8)).radius(px(6)).border(px(0))${t("body")}.font_weight(400).text_color(${ink}).hover(style().background(${wash}))`);
      cset(name,"shortcut",`style()${t("key_legend")}.text_color(${faint})`);
      cset(name,"separator",`style().height(px(1)).background(${edge}).margin_y(px(4))`);
    }
    cset("menu","label",`style().padding_x(px(8)).padding_y(px(4))${t("caption")}.font_weight(500).text_color(${muted})`);
    cset("breadcrumb","root",`style()${t("breadcrumb")}.text_color(${muted})`);
    cset("breadcrumb","item",`style().gap(px(6)).radius(px(6))`);
    cset("breadcrumb","link",`style().hover(style().text_color(${ink}))`);
    cset("breadcrumb","current",`style().font_weight(500).text_color(${ink})`);
    cset("breadcrumb","label","style().whitespace_nowrap().text_ellipsis()");
    cset("breadcrumb","separator",`style().min_width(px(20)).text_color(${placeholder})`);
    cset("breadcrumb","separator_icon",`style().width(px(${clean ? 12 : 14})).height(px(${clean ? 12 : 14}))`);
    cset("breadcrumb","icon","style().width(px(14)).height(px(14))");
    cset("breadcrumb","more",`style().width(px(${clean ? 24 : 26})).height(px(${clean ? 20 : 22})).radius(px(6)).hover(style().background(${wash}).text_color(${ink}))`);
    cset("breadcrumb","more_icon","style().width(px(16)).height(px(16))");
    cset("breadcrumb","popup",`style().min_width(px(${clean ? 168 : 180})).max_height(px(280)).padding(px(${clean ? 4 : 5})).gap(px(1)).radius(px(${clean ? 12 : 14})).background(${panel}).border(px(0))${t("breadcrumb")}${shadows(floatingShadow)}`);
    cset("breadcrumb","popup_item",`style().min_height(px(${clean ? 32 : 34})).padding_x(px(8)).gap(px(8)).radius(px(${clean ? 8 : 9}))${t("breadcrumb")}`);
    cset("breadcrumb","popup_icon",`style().width(px(14)).height(px(14)).text_color(${muted})`);
    for (const name of ["select","combobox"]) {
      cset(name,"trigger",`style().min_height(${length("metrics.field_h")}).padding_x(${length("metrics.field_pad")}).gap(px(6)).radius(${length("metrics.field_radius")}).border(px(0)).background(${field})${t("control")}.font_weight(400)${shadows(fieldShadow)}.focus(style()${shadows(focusShadow)})`);
      cset(name,"panel",`style().padding(px(4)).radius(px(${clean ? 10 : 12})).border(px(0)).background(${panel})${shadows(floatingShadow)}`);
      cset(name,"placeholder",`style().text_color(${placeholder})`);
      cset(name,"indicator",`style().width(px(16)).height(px(16)).text_color(${faint})`);
      cset(name,"clear",`style().width(px(20)).height(px(20)).padding(px(0)).radius(px(4)).border(px(0)).text_color(${faint})`);
      cset(name,"group",`style().padding_x(px(8)).padding_y(px(5))${t("caption")}.font_weight(500).text_color(${muted})`);
      cset(name,"option",`style().min_height(px(${clean ? 28 : 32})).padding_x(px(8)).gap(px(8)).radius(px(6)).border(px(0))${t("body")}.font_weight(400).hover(style().background(${wash}))`);
      for (const part of ["option_selected","option_active"]) cset(name,part,`style().background(${selected}).border(px(0)).font_weight(400)`);
      cset(name,"option_disabled",`style().opacity(0.4)`);
      cset(name,"empty",`style().padding(px(12))${t("body")}.text_color(${muted})`);
    }
    const commandShadow=clean ? (dark
      ? [shadow(0,0,0,.5,color("command.shadow_edge")),shadow(0,1,2,0,black(.4))]
      : [shadow(0,0,0,.5,color("command.shadow_edge")),...floatingCast(black(.16))])
      : [shadow(0,0,0,1,color("command.shadow_edge")),shadow(0,1,2,0,black(.04))];
    cset("command","root",`style().width(relative(1)).max_width(px(600)).gap(px(0)).padding(${length("metrics.command_tray_pad")}).radius(px(${clean ? 12 : 20})).border(px(0)).background(${color(clean ? "command.surface" : "command.tray")})${t("command_body")}.text_color(${color("command.ink")})${shadows(clean ? commandShadow : [])}`);
    cset("command","root_embedded",`style().padding(px(0)).radius(px(0)).border(px(0)).background(${transparent}).shadows([])`);
    cset("command","card",`style().gap(px(0)).radius(px(${clean ? 0 : 16})).background(${clean ? transparent : color("command.surface")})${shadows(clean ? [] : commandShadow)}`);
    cset("command","card_embedded",`style().radius(px(0)).border(px(0)).background(${transparent}).shadows([])`);
    cset("command","search",`style().height(${length("metrics.command_search_h")}).min_height(${length("metrics.command_search_h")}).flex_shrink(false).padding_start(${length("metrics.command_search_start")}).padding_end(${length("metrics.command_search_end")}).gap(px(${clean ? 12 : 10})).border(px(0)).border_bottom(px(${clean ? .5 : 1})).border_color(${color("command.line")})`);
    cset("command","search_input",`style().height(${length("metrics.command_input_h")}).min_height(px(0)).padding(px(0)).border(px(0)).background(${transparent}).radius(px(0)).shadows([])${t("command_search")}.text_color(${color("command.ink")}).focus(style().shadows([]))`);
    cset("command","search_placeholder",`style().text_color(${color("command.placeholder")})`);
    cset("command","search_icon",`style().width(${length("metrics.command_search_icon")}).height(${length("metrics.command_search_icon")}).text_color(${color("command.icon")}).disabled(style().opacity(0.55))`);
    cset("command","search_hint",`style().min_width(px(${clean ? 20 : 0})).min_height(px(${clean ? 20 : 0})).padding_x(px(${clean ? 4 : 0})).radius(px(5)).gap(px(${clean ? 0 : 4})).text_color(${color("command.placeholder")})${t("command_enter")}.disabled(style().opacity(0.55))${shadows(clean ? [shadow(0,0,0,.5,color("kbd.menu_edge"),true)] : [])}`);
    cset("command","search_hint_icon",`style().width(${length("metrics.command_enter_icon")}).height(${length("metrics.command_enter_icon")})`);
    cset("command","search_hint_text",`style()${t("command_enter")}`);
    cset("command","list",`style().padding_x(${length("metrics.command_list_inset")}).padding_top(${length("metrics.command_list_top")}).padding_bottom(${length("metrics.command_list_bottom")}).min_height(px(0))`);
    cset("command","list_viewport",`style().min_height(px(0))`);
    cset("command","item",`style().height(${length("metrics.command_row")}).min_height(${length("metrics.command_row")}).padding_x(${length("metrics.command_item_inset")}).gap(px(${clean ? 10 : 12})).radius(px(${clean ? 6 : 10})).border(px(0))${t("command_body")}.font_weight(400).text_color(${color("command.ink")}).hover(style().background(${transparent})).disabled(style().text_color(${color("command.ink")}).opacity(0.4))`);
    cset("command","item_active",`style().background(${color("command.selection")}).border(px(0)).hover(style().background(${color("command.selection")}))`);
    cset("command","item_narrow",`style().height(${length("metrics.command_narrow_row")}).min_height(${length("metrics.command_narrow_row")})`);
    cset("command","label",`style().whitespace_nowrap().text_ellipsis()`);
    cset("command","text",`style().gap(px(8)).items_baseline()`);
    cset("command","hint",`style().text_color(${color("command.hint")}).whitespace_nowrap().text_ellipsis()`);
    cset("command","icon_slot",`style().width(${length("metrics.command_icon_slot")}).height(${length("metrics.command_icon_slot")}).clip().text_color(${color("command.icon")})`);
    cset("command","icon_slot_active",`style().text_color(${color("command.ink")})`);
    cset("command","logo_slot",`style().radius(${length("metrics.command_logo_radius")}).background(${color("command.logo_surface")}).text_color(${color("command.logo_ink")})${shadows(clean ? [shadow(0,0,0,.5,color("command.logo_edge")),shadow(0,1,1,-1,black(.1))] : [shadow(0,0,0,1,color("command.logo_edge"),true)])}`);
    cset("command","icon",`style().width(${length("metrics.command_item_icon")}).height(${length("metrics.command_item_icon")})`);
    cset("command","logo",`style().width(${length("metrics.command_logo")}).height(${length("metrics.command_logo")})`);
    cset("command","image_fallback",`style().font_size(px(${clean ? 10 : 11})).line_height(px(${clean ? 10 : 11})).font_weight(600).text_color(${color("command.ink")})`);
    cset("command","group",`style().padding_x(${length("metrics.command_item_inset")}).padding_top(${length("metrics.command_group_top")}).padding_bottom(${length("metrics.command_group_bottom")})${t("command_group")}.text_color(${color("command.group")})`);
    cset("command","shortcut",`style().gap(px(${clean ? 2 : 3})).items_center().flex_shrink(false).text_color(${color("command.shortcut")}).opacity(${clean ? 1 : .85})`);
    cset("command","shortcut_active",`style().text_color(${color("command.shortcut_active")}).opacity(1)`);
    cset("command","empty",`style().padding(px(16))${t("body")}.text_color(${muted})`);
    cset("command","footer",`style().min_height(${length("metrics.command_footer_h")}).padding_x(px(12)).padding_y(px(4)).border(px(0)).border_top(px(${clean ? .5 : 0})).border_color(${color("command.line")})${t("command_footer")}.text_color(${color("command.footer")}).disabled(style().opacity(0.55))`);
    cset("command","footer_hint",`style().gap(px(6)).flex_shrink(false)`);
    // Calendar is the same visual family as DatePicker, with independent API.
    for (const name of ["calendar","date_picker"]) {
      const rootPart = name === "calendar" ? "root" : "panel";
      cset(name,rootPart,`style().padding(px(12)).gap(px(8)).radius(px(${clean ? 12 : 16})).border(px(0)).background(${panel})${name === "date_picker" ? shadows(floatingShadow) : ""}`);
      cset(name,"header",`style().gap(px(8)).padding_y(px(4))`);
      cset(name,"month_title",`style()${t("label")}.font_weight(500).text_color(${strongest})`);
      cset(name,"weekday",`style()${t("caption")}.font_weight(400).text_color(${faint})`);
      cset(name,"day",`style().min_height(px(${clean ? 28 : 32})).radius(px(${clean ? 5 : 7})).border(px(0))${t("body")}.font_weight(400).hover(style().background(${wash}))`);
      cset(name,"day_selected",`style().background(${color("button.primary_bg")}).text_color(${color("button.primary_fg")}).border(px(0))`);
      cset(name,"day_today",`style().font_weight(500).border(px(0))`);
      // Adjacent-month dates are readable subtle text, not empty-field hints.
      cset(name,"day_outside",`style().text_color(${faint})`);
      cset(name,"day_disabled",`style().opacity(0.35)`);
    }
    cset("date_picker","trigger",`style().min_height(${length("metrics.field_h")}).padding_x(${length("metrics.field_pad")}).gap(px(8)).radius(${length("metrics.field_radius")}).border(px(${clean ? 0 : 1})).border_color(${edge}).background(${field})${t("control")}${shadows(smallShadow)}`);
    cset("date_picker","trigger_icon",`style().width(px(16)).height(px(16)).text_color(${faint})`);
    cset("date_picker","footer",`style().gap(px(8)).padding_top(px(8))`);
    cset("calendar","banded_range",`style().background(${color("date_range.band")})`);
    cset("calendar","day_preview",`style().background(${color("date_range.preview")})`);
    cset("calendar","day_comparison",`style().background(${color("date_range.comparison")})`);
    cset("calendar","banded_selected",`style().background(${color("date_range.chip")}).text_color(${color("date_range.on_chip")}).font_weight(${clean ? 500 : 600}).border(px(0)).hover(style().background(${color("date_range.chip")}))`);
    cset("calendar","banded_weekday",`style().height(px(${clean ? 24 : 26})).font_size(px(${clean ? 10 : 11})).line_height(px(${clean ? 16 : 18})).font_weight(500).letter_spacing_px(${clean ? "0.5" : "0.0"}).text_color(${faint})`);
    cset("calendar","banded_grid",`style().gap(px(2))`);
    cset("calendar","banded_cell",`style().height(px(32))`);
    cset("calendar","banded_nav",`style().width(px(28)).height(px(28)).radius(px(${clean ? 6 : 8})).text_color(${faint})`);
    cset("calendar","banded_month_title",`style()${t("control")}.font_weight(${clean ? 500 : 600}).text_color(${strongest})`);
    cset("date_range_picker","trigger",`style().min_height(${length("metrics.field_h")}).padding_x(px(10)).gap(px(8)).radius(${length("metrics.field_radius")}).border(px(0)).background(${field})${t("control")}${shadows(fieldShadow)}.hover(style()${shadows(focusShadow)})`);
    cset("date_range_picker","trigger_icon",`style().width(px(16)).height(px(16)).text_color(${faint})`);
    cset("date_range_picker","trigger_comparison",`style()${t("caption")}.text_color(${faint})`);
    cset("date_range_picker","panel",`style().padding(px(${clean ? 0 : 4})).radius(px(${clean ? 10 : 20})).background(${color("date_range.tray")})${shadows(floatingShadow)}`);
    cset("date_range_picker","body",`style().background(${panel}).radius(px(${clean ? 10 : 16}))${shadows(clean ? [] : [hairline])}`);
    cset("date_range_picker","presets",`style().width(px(${clean ? 140 : 136})).padding(px(${clean ? 4 : 6})).gap(px(${clean ? 2 : 1})).border_end(px(${clean ? .5 : 1})).border_color(${edge})`);
    cset("date_range_picker","presets_compact",`style().padding(px(${clean ? 8 : 10})).gap(px(${clean ? 4 : 6})).border_bottom(px(${clean ? .5 : 1})).border_color(${edge})`);
    cset("date_range_picker","preset",`style().height(px(${clean ? 28 : 30})).radius(px(${clean ? 6 : 8})).padding_x(px(10)).font_weight(${clean ? 500 : 400}).text_color(${clean ? muted : ink}).hover(style().background(${wash}).text_color(${ink}))`);
    cset("date_range_picker","preset_compact",`style().height(px(${clean ? 28 : 32})).radius(px(${clean ? 6 : 999})).padding_x(px(${clean ? 10 : 12}))${shadows([shadow(0,0,0,clean ? .5 : 1,edge,true)])}`);
    cset("date_range_picker","preset_selected",`style().background(${color("date_range.preset")}).text_color(${ink}).hover(style().background(${color("date_range.preset")}))`);
    cset("date_range_picker","preset_custom",`style()${t("caption")}.text_color(${faint})`);
    cset("date_range_picker","calendar",`style().padding_x(px(${clean ? 12 : 14})).padding_y(px(10)).background(${panel})`);
    cset("date_range_picker","footer",`style().padding(px(10)).border_top(px(${clean ? .5 : 1})).border_color(${edge}).gap(px(${clean ? 12 : 14}))`);
    cset("date_range_picker","comparison",`style()${t("caption")}.text_color(${faint})`);
    cset("date_range_picker","compare_swatch",`style().background(${color("date_range.comparison")})${shadows([shadow(0,0,0,clean ? .5 : 1,color("date_range.comparison_edge"),true)])}`);
    // The temporal wrapper uses the real Input; do not repaint its native
    // field, state or focus surface with a second, independently tuned recipe.
    cset("time_input","root",`style()`);
    cset("time_range_picker","panel",floatingPanel);
    cset("time_range_picker","trigger",`style().min_height(${length("metrics.field_h")}).radius(${length("metrics.field_radius")}).border(px(1)).border_color(${edge}).background(${field})${t("control")}`);
    cset("time_range_picker","actions",`style().gap(px(8))`);
    // Keyboard cursor/focus belongs to the collection, not this decorative
    // outer frame. Adding a surface edge must not invent a second focus cue.
    const tableShadow = clean ? [shadow(0,0,0,.5,color("table.edge")),
      ...(dark ? [shadow(0,1,2,0,black(.4))] : [shadow(0,1,1,-1,black(.1)),shadow(0,1,2,0,black(.05))])]
      : [shadow(0,1,2,0,black(dark ? .4 : .06))];
    cset("table","root",`style().border(px(${clean ? 0 : 1})).border_color(${color("table.edge")}).radius(px(${clean ? 10 : 16})).letter_spacing_em(${clean ? 0 : -0.01}).background(${color("table.surface")})${shadows(tableShadow)}.focus(style().border_color(${color("table.edge")}))`);
    cset("table","header",`style().min_height(${length("metrics.table_header")}).background(${color("table.surface")}).border(px(0))${t("table_header")}`);
    cset("table","header_cell",`style().padding_x(${length("metrics.table_header_pad")}).border_start(${length("metrics.table_grid_x")}.without_reading_scale()).border_color(${color("table.grid")})${t("table_header")}.text_color(${color("table.header_ink")})`);
    cset("table","header_action",`style().height(${length("metrics.table_action_h")}).min_width(px(0)).padding_x(${length("metrics.table_action_pad")}).gap(px(${clean ? 6 : 8})).radius(px(${clean ? 6 : 8})).border(px(0)).background(${transparent}).hover(style().background(${color("table.header_hover")}).text_color(${color("table.header_hover_ink")})).focus_ring(px(2),offset_px(-2),${focus})`);
    cset("table","sort_icon",`style().width(${length("metrics.table_sort_icon")}).height(${length("metrics.table_sort_icon")}).text_color(${color("table.icon")})`);
    cset("table","sort_icon_active",`style().text_color(${color("table.sort_ink")})`);
    cset("table","row",`style().min_height(${length("metrics.table_row")}).border(px(0))${t("table_body")}.font_weight(400).text_color(${color("table.text")}).hover(style().background(${color("table.row_hover")}))`);
    cset("table","row_selected",`style().background(${color("table.selection")}).border(px(0)).hover(style().background(${color("table.selection")}))`);
    cset("table","cell",`style().padding_x(${length("metrics.table_pad")}).border_top(${length("metrics.table_grid_y")}.without_reading_scale()).border_start(${length("metrics.table_grid_x")}.without_reading_scale()).border_color(${color("table.grid")})${t("table_body")}`);
    cset("table","cell_selected",`style()`);
    cset("table","group_header",`style().min_height(${length("metrics.table_header")}).background(${wash})${t("table_header")}.font_weight(500)`);
    cset("table","group_indicator",`style().width(px(14)).height(px(14)).text_color(${faint})`);
    cset("table","group_label",`style()${t("label")}.text_color(${ink})`);
    cset("table","group_count",`style()${t("caption")}.text_color(${color("collection.metadata")})`);
    for (const name of ["list","selectable_list","tree"]) {
      cset(name,"root",`style().border(px(0)).radius(px(0))${t("body")}`);
      cset(name,"row",`style().min_height(px(${clean ? 28 : 32})).padding_x(${length("metrics.inset")}).gap(px(8)).radius(px(6)).border(px(0))${t("body")}.font_weight(400).hover(style().background(${wash}))`);
      cset(name,"row_selected",`style().background(${selected}).border(px(0)).font_weight(400).hover(style().background(${selected}))`);
    }
    cset("tree","row_active",`style().background(${wash}).border(px(0))`);
    cset("tree","disclosure",`style().width(px(14)).height(px(14)).text_color(${faint})`);
    cset("tree","label",`style()${t("body")}.font_weight(400)`);
    cset("list","title",`style()${t("body")}.font_weight(500)`);
    cset("list","secondary",`style()${t("caption")}.text_color(${muted})`);
    cset("list","meta",`style()${t("body")}.text_color(${muted})`);
    cset("column_browser","root",`style().border(px(1)).border_color(${edge}).radius(px(${clean ? 8 : 12})).background(${panel})`);
    cset("column_browser","heading",`style().padding(px(10))${t("label")}.font_weight(500)`);
    cset("column_browser","path",`style().padding_x(px(10)).padding_y(px(6)).background(${panel}).border_bottom(px(1)).border_color(${edge})`);
    cset("column_browser","path_text",`style()${t("caption")}.text_color(${color("collection.metadata")})`);
    for (const name of ["slider","range_slider"]) {
      cset(name,"root",`style().gap(px(8))${t("body")}`);
      cset(name,"label",`style()${t("label")}.font_weight(500)`);
      cset(name,"value",`style()${t("caption")}.text_color(${muted})`);
      // The component chooses the cross-axis thickness; a visual recipe must
      // not turn a vertical track into a 4px-high horizontal one.
      cset(name,"track",`style().radius(px(999)).background(${edgeStrong})`);
      cset(name,"fill",`style().radius(px(999)).background(${color("button.primary_bg")})`);
      cset(name,"thumb",`style().width(px(${clean ? 12 : 14})).height(px(${clean ? 12 : 14})).radius(px(999)).border(px(0)).background(${panel})${shadows([shadow(0,0,0,1,edgeStrong),shadow(0,1,3,0,black(.14))])}.focus_ring(px(2),px(2),${focus})`);
    }
    cset("progress","root",`style().min_height(px(4)).radius(px(999))`);
    cset("progress","track",`style().height(px(4)).radius(px(999)).background(${wash})`);
    cset("progress","indicator",`style().radius(px(999)).background(${color("button.primary_bg")})`);
    cset("skeleton","root",`style().radius(px(${clean ? 4 : 6})).background(${wash})`);
    cset("spinner","root",`style()`);
    cset("spinner","indicator",`style()`);
    for (const name of ["accordion","collapsible"]) {
      cset(name,"root",`style().gap(px(0)).border(px(0)).radius(px(0))`);
      cset(name,"trigger",`style().min_height(px(${clean ? 36 : 40})).padding_y(px(8)).gap(px(8)).border(px(0))${t("label")}.font_weight(500)`);
      cset(name,"content",`style().padding_bottom(px(12))${t("body")}.text_color(${muted})`);
    }
    cset("accordion","item",`style().border_bottom(px(1)).border_color(${edge})`);
    cset("group_box","root",`style().padding(px(16)).gap(px(12)).radius(px(${clean ? 10 : 16})).border(px(1)).border_color(${edge})`);
    cset("group_box","label",`style()${t("label")}.font_weight(500)`);
    cset("group_box","description",`style()${t("hint")}.text_color(${muted})`);
    cset("empty","root",`style().padding(px(24)).gap(px(8)).border(px(0))${t("body")}`);
    cset("empty","title",`style()${t("subtitle")}.font_weight(500)`);
    cset("empty","description",`style()${t("body")}.text_color(${muted})`);
    cset("empty","icon",`style().width(px(36)).height(px(36)).padding(px(8)).radius(px(${clean ? 8 : 12})).background(${wash}).text_color(${faint})`);
    cset("empty","actions",`style().gap(px(8)).padding_top(px(4))`);
    cset("alert","root",`style().padding(px(12)).gap(px(10)).border(px(1)).border_color(${edge}).radius(px(${clean ? 8 : 12})).background(${panel})`);
    cset("alert","title",`style()${t("label")}.font_weight(500)`);
    cset("alert","description",`style()${t("body")}.text_color(${muted})`);
    cset("alert","lamp",`style().radius(px(999))`);
    cset("toast","toast",`style().padding(px(${clean ? 12 : 16})).gap(px(8)).radius(px(${clean ? 10 : 14})).border(px(0)).background(${panel})${shadows(floatingShadow)}`);
    cset("toast","root","style()");
    cset("toast","title",`style()${t("label")}.font_weight(500)`);
    cset("toast","message",`style()${t("body")}.text_color(${muted})`);
    cset("toast","lamp",`style().radius(px(999))`);
    const current = amount => `alpha(current_color(),${amount})`;
    cset("kbd","root",`style().height(${length("metrics.kbd_h")}).min_width(${length("metrics.kbd_h")}).padding_x(${length("metrics.kbd_pad")}).radius(${length("metrics.kbd_radius")}).border(px(0)).background(${clean ? current(dark ? .10 : .07) : color("kbd.surface")})${t("key_legend_cap")}.font_weight(500).font_feature("tnum",1).whitespace_nowrap()${clean ? ".inherit_text_color()" : `.text_color(${color("kbd.ink")})`}${shadows(clean ? [shadow(0,0,0,.5,current(dark ? .08 : .06),true)] : [shadow(0,0,0,1,color("kbd.edge"),true),shadow(0,-1,0,0,color("kbd.depth"),true)])}`);
    cset("kbd","keys",`style().gap(${length("space.unit")}.times(0.25)).relative().bottom(${clean ? "px(0)" : `${length("space.unit")}.times(0.125)`})`);
    cset("kbd","keys_other",`style().gap(${length("space.unit")}.times(0.75))`);
    cset("kbd","key","style().whitespace_nowrap()");
    cset("kbd","glyph",`style().font_size(${length("metrics.kbd_glyph")}).line_height(${length("metrics.kbd_glyph")})`);
    cset("kbd","inline",`style().padding(px(0)).margin(px(0)).height(auto()).min_width(px(0)).min_height(px(0)).radius(px(0)).border(px(0)).background(${transparent}).shadows([])${t("key_legend")}.font_weight(400)`);
    cset("kbd","menu",`style().height(px(20)).min_height(px(20)).min_width(px(20)).padding_x(px(5)).radius(px(5)).border(px(0)).background(${color("kbd.menu_surface")})${t("key_legend_menu")}.font_weight(500).inherit_text_color()${shadows(clean ? [shadow(0,0,0,.5,color("kbd.menu_edge"),true)] : [shadow(0,0,0,1,color("kbd.menu_edge"),true),shadow(0,1,0,0,color("kbd.menu_edge"))])}`);
    cset("kbd","menu_key",`style()${t("key_legend_menu")}.inherit_text_color()`);
    cset("kbd","menu_keys",`style().bottom(px(0))`);
    cset("divider","root",`style().background(${edge})`);
    cset("icon","root",`style().width(${length("metrics.icon")}).height(${length("metrics.icon")})`);
    cset("button_group","root",`style().gap(px(0))`);
    cset("button_group","item",`style().focus_ring(px(2),offset_px(-2),${focus})`);
    cset("toggle","root",`style().font_weight(500).radius(${length("metrics.button_radius")})`);
    cset("tabs","list","style()");
    cset("tabs","root",`style().gap(px(12))`);
    cset("tabs","tab",`style()${t("segment")}.font_weight(${clean ? 500 : 400}).text_color(${color("tabs.foreground")})`);
    cset("tabs","tab_selected",`style().text_color(${color("tabs.ink")})`);
    cset("tabs","indicator","style()");
    const tabFocus = `.group_focus(style().border(px(2)).border_color(${focus}))`;
    const tabShadow = clean
      ? (dark ? [shadow(0,0,0,.5,white(.1)),shadow(0,1,2,0,black(.4))] : smallShadow)
      : (dark ? [shadow(0,0,0,1,white(.06)),shadow(0,1,2,0,black(.4))]
          : [shadow(0,0,0,.5,black(.08)),shadow(0,1,2,0,black(.08)),shortDrop(black(.06),0,-1)]);
    const segmentSelection = `style().background(${color("tabs.pill")})${shadows(tabShadow)}`;
    recipes.tabs = {
      track_list: `style().padding(${length("metrics.segment_pad")}).gap(px(0)).radius(${length("metrics.segment_radius")}).background(${color("tabs.tray")})`,
      track_tab: `style().min_height(${length("metrics.tabs_track_h")}).padding_x(${length("metrics.segment_px")}).border(px(0)).radius(${length("metrics.segment_thumb_radius")})`,
      track_indicator: `${segmentSelection}.radius(${length("metrics.segment_thumb_radius")}).border(px(0))${tabFocus}`,
      chip_list: `style().padding(px(0)).gap(${length("metrics.segment_pad")}).radius(px(0)).background(${transparent})`,
      chip_tab: `style().min_height(${length("metrics.segment_h")}).padding_x(${length("metrics.segment_px")}).radius(${length("metrics.segment_thumb_radius")})${clean ? `.border(px(0))${shadows([shadow(0,0,0,.5,color("tabs.line"),true)])}` : ""}`,
      chip_indicator: `style().radius(${length("metrics.segment_thumb_radius")}).background(${clean ? color("tabs.counter") : panel}).border(px(${clean ? 0 : 1})).border_color(${edge})${shadows(clean ? [] : smallShadow)}${tabFocus}`,
      underline_list: `style().padding(px(0)).gap(${length("metrics.segment_gap")}).radius(px(0)).background(${transparent})`,
      underline_tab: `style().min_height(${length("metrics.tabs_underline_h")}).padding_x(px(2)).radius(px(6)).border(px(0))`,
      underline_indicator: `style().height(px(${clean ? 1.5 : 2})).radius_top_left(px(${clean ? 1.5 : 2})).radius_top_right(px(${clean ? 1.5 : 2})).radius_bottom_left(px(0)).radius_bottom_right(px(0)).border(px(0)).background(${color("tabs.ink")}).shadows([])`,
      bar_list: `style().padding(px(0)).gap(px(0)).radius(px(0)).background(${transparent})`,
      bar_tab: `style().min_height(${length("metrics.tabs_bar_h")}).radius(px(0))`,
      bar_indicator: `style()`,
    };
    recipes.segmented = {
      option: `${recipes.tabs.track_tab}${t("segment")}.font_weight(500).text_color(${color("tabs.foreground")}).whitespace_nowrap().focus_ring(px(2),offset_px(-2),${focus}).disabled(style().opacity(0.4))`,
      selected: `${segmentSelection}.text_color(${color("tabs.ink")})`,
    };
    for (const variant of ["track","chip","underline","bar"]) {
      const height = length(variant === "track" ? "metrics.tabs_track_total_h" : variant === "chip" ? "metrics.segment_h" : `metrics.tabs_${variant}_h`);
      recipes.tabs[`${variant}_list`] += `.min_height(${height})`;
      recipes.tabs[`${variant}_addon`] = `style().width(${height}).height(${height}).min_height(px(0)).padding(px(0)).border(px(0)).radius(${variant === "track" || variant === "chip" ? length("metrics.segment_thumb_radius") : "px(0)"})`;
      const hoverFill = variant === "bar" || variant === "chip" || (variant === "track" && clean);
      const hoverInk = variant === "underline" || variant === "bar" || !clean;
      const hover = `.hover(style()${hoverFill ? `.background(${wash})` : ""}${hoverInk ? `.text_color(${color("tabs.hover")})` : ""})`;
      recipes.tabs[`${variant}_tab`] += hover;
      if (variant === "track") recipes.segmented.normal = `style()${hover}`;
    }
    cset("toggle_group","root",`${recipes.tabs.track_list}.border(px(0))`);
    cset("toggle_group","item",recipes.segmented.option);
    cset("toggle_group","item_pressed",recipes.segmented.selected);
    cset("toggle_group","label",`style().font_weight(500)`);
    cset("toggle_group","label_pressed",`style().font_weight(${clean ? 500 : 600})`);
    cset("tabs","icon",`style().width(${length("metrics.segment_icon")}).height(${length("metrics.segment_icon")})${clean ? `.text_color(${color("tabs.icon")})` : ""}`);
    cset("tabs","icon_selected",clean ? `style().text_color(${color("tabs.ink")})` : "style()");
    for (const part of ["lamp","dirty"]) cset("tabs",part,`style().width(${length("metrics.status_dot")}).height(${length("metrics.status_dot")}).radius(px(999))`);
    cset("tabs","counter",`style().height(${length("metrics.tabs_counter_h")}).min_width(${length("metrics.tabs_counter_h")}).padding_x(${length("metrics.tabs_counter_pad")}).radius(${length("metrics.tabs_counter_radius")}).border(px(0)).background(${color("tabs.counter")}).text_color(${color("tabs.counter_fg")})${t("tabs_counter")}`);
    cset("tabs","counter_selected",clean ? `style().background(${color("tabs.pill")}).text_color(${color("tabs.ink")})${shadows([shadow(0,0,0,.5,color("tabs.line"))])}` : "style()");
    for (const name of ["title_bar","status_bar"]) {
      cset(name,"root",`style().min_height(${length(name === "title_bar" ? "metrics.titlebar" : "metrics.statusbar")}).padding_x(px(12)).gap(px(8)).border(px(0)).background(${panel})${t(name === "title_bar" ? "body" : "caption")}`);
    }
    cset("title_bar","title",`style()${t("label")}.font_weight(500)`);
    cset("title_bar","subtitle",`style()${t("caption")}.text_color(${muted})`);
    for (const name of ["resizable"]) {
      cset(name,"grip",`style().radius(px(999)).background(${faint})`);
      for (const part of ["grip_hover","grip_drag","grip_focus"]) cset(name,part,`style().radius(px(999)).background(${ink})`);
      cset(name,"grip_disabled",`style().radius(px(999)).background(${placeholder})`);
    }
    // The split grip is a transparent capture zone around an independently painted
    // decoration. Filling this zone defeats flush mode and its hidden-at-rest policy.
    for (const part of ["grip","grip_hover","grip_drag","grip_focus","grip_disabled"])
      cset("split_pane",part,`style().background(${transparent})`);
    for (const name of ["pagination","carousel"]) {
      cset(name,"root",`style().gap(px(8))${t("body")}`);
      cset(name,name === "pagination" ? "summary" : "count",`style()${t("caption")}.text_color(${muted})`);
    }
    cset("pagination","root",`style().flex_wrap().gap_x(px(16)).gap_y(px(8))${t("pagination")}.font_weight(400).font_feature("tnum",1).text_color(${color("pagination.muted")})`);
    cset("pagination","info",`style().gap(px(16))`);
    cset("pagination","page_size_control",`style().gap(px(8)).disabled(style().opacity(0.5))`);
    cset("pagination","summary",`style()${t("pagination")}.text_color(${color("pagination.text")})`);
    cset("pagination","controls",`style().gap(px(${clean ? 2 : 4})).disabled(style().opacity(0.5))`);
    const pageGeometry=`style().min_height(px(0)).height(${length("metrics.pagination_h")}).min_width(${length("metrics.pagination_h")}).padding_x(${length("metrics.pagination_pad")}).radius(${length("metrics.pagination_radius")}).border(px(0))${t("pagination")}.font_weight(400)`;
    cset("pagination","page",`${pageGeometry}.background(${transparent}).text_color(${color("pagination.text")}).hover(style().background(${color("pagination.wash")}).text_color(${color("pagination.ink")})).disabled(style().opacity(1))`);
    cset("pagination","page_current",`style().background(${color("pagination.active")}).text_color(${color("pagination.ink")}).font_weight(500).hover(style().background(${color("pagination.active")}).text_color(${color("pagination.ink")}))`);
    for (const part of ["previous","next"]) cset("pagination",part,`style().width(${length("metrics.pagination_h")}).height(${length("metrics.pagination_h")}).min_height(px(0)).padding(px(0)).radius(${length("metrics.pagination_radius")}).border(px(${clean ? 0 : 1})).border_color(${color("pagination.edge")}).background(${clean ? transparent : color("pagination.surface")}).text_color(${color("pagination.muted")})${shadows(clean ? [] : [shadow(0,1,2,0,black(dark ? .4 : .06))])}.hover(style().background(${color(clean ? "pagination.wash" : "pagination.surface_hover")}).text_color(${color("pagination.ink")})).disabled(style().opacity(${clean ? .4 : .45}).shadows([]))`);
    for (const part of ["previous_icon","next_icon"]) cset("pagination",part,`style().width(${length("metrics.pagination_icon")}).height(${length("metrics.pagination_icon")})`);
    cset("pagination","ellipsis",`style().min_width(px(20)).height(${length("metrics.pagination_h")}).text_color(${color("pagination.faint")})`);
    cset("pagination","page_size",`style().min_height(px(0)).height(${length("metrics.pagination_h")})`);
    cset("pagination","page_size_trigger",`style().min_height(px(0)).height(${length("metrics.pagination_h")}).padding_start(px(${clean ? 10 : 11})).padding_end(px(${clean ? 8 : 9})).gap(px(${clean ? 6 : 5})).radius(${length("metrics.pagination_radius")}).border(px(${clean ? 0 : 1})).border_color(${color("pagination.edge")}).background(${color("pagination.surface")})${t("pagination")}.font_weight(500).text_color(${color("pagination.ink")})${shadows(clean ? smallShadow : [shadow(0,1,2,0,black(dark ? .4 : .06))])}.disabled(style().text_color(${color("pagination.ink")}).opacity(1))`);
    cset("pagination","page_size_indicator",`style().width(${length("metrics.pagination_select_icon")}).height(${length("metrics.pagination_select_icon")})`);
    cset("pagination","page_size_value",`style()${t("pagination")}.font_weight(500)`);
    cset("pagination","page_size_label",`style()${t("pagination")}.text_color(${color("pagination.muted")})`);
    cset("carousel","dot",`style().width(px(5)).height(px(5)).radius(px(999)).background(${edgeStrong})`);
    cset("carousel","dot_selected",`style().width(${length("metrics.icon")}).radius(px(999)).background(${ink})`);
    cset("timeline","root",`style().gap(px(0))${t("body")}`);
    cset("timeline","title",`style()${t("label")}.font_weight(500)`);
    cset("timeline","description",`style()${t("body")}.text_color(${muted})`);
    cset("timeline","lamp",`style().radius(px(999))`);
    for (const part of ["time","meta"]) cset("timeline",part,`style()${t("caption")}.font_weight(400).text_color(${color("collection.metadata")})`);
    cset("timeline","day_header",`style()${t("label")}.font_weight(500).text_color(${color("collection.metadata")})`);
    cset("selectable_card_group","card",`style().padding(px(16)).gap(px(8)).radius(px(${clean ? 10 : 16})).border(px(1)).border_color(${edge}).background(${panel})`);
    cset("selectable_card_group","root",`style().gap(px(12))`);
    cset("selectable_card_group","card_selected",`style().background(${color("collection.selection")}).border_color(${edgeStrong})`);
    cset("selectable_card_group","title",`style()${t("subtitle")}.font_weight(500)`);
    cset("selectable_card_group","indicator",`style().radius(px(999)).text_color(${color("collection.metadata")})`);
    cset("sortable","handle_icon",`style().text_color(${color("collection.metadata")})`);
    for (const name of ["code_viewer","diff_viewer","markdown_viewer"]) {
      // Document content/max-height contracts must not gain extra layout pixels
      // from a decorative frame (including an empty streamed Markdown document).
      cset(name,"root",`style().border(px(0)).radius(px(${clean ? 8 : 12})).background(${panel})${shadows([hairline])}${t(name === "markdown_viewer" ? "body" : "code")}`);
      for (const part of ["loading","error"]) cset(name,part,`style().padding(px(12))${t("body")}.text_color(${part === "error" ? color("danger") : muted})`);
    }
    cset("markdown_viewer","heading",`style()${t("heading")}.font_weight(600).text_color(${strongest})`);
    cset("markdown_viewer","paragraph",`style()${t("body")}.text_color(${ink})`);
    for (const part of ["code_block","inline_code"]) cset("markdown_viewer",part,`style().radius(px(${clean ? 4 : 6})).background(${wash})${t("code")}`);
    cset("markdown_viewer","quote",`style().border_start(px(2)).border_color(${edgeStrong}).padding_start(px(12)).text_color(${muted})`);
    cset("markdown_viewer","rule",`style().background(${edge})`);
    // Every L2 pattern consumes the same component recipes; only its arrangement
    // and explicit structural parts are defined here.
    for (const id of ["layouts/inline","layouts/stack","layouts/toolbar"]) set(id,"root",`style().gap(px(${clean ? 8 : 12}))`);
    set("layouts/region","root",`style().background(${panel})`);
    // Region owns the shared text edge once. Its inset/bleed props decide which slots
    // receive it; a theme must not unconditionally pad those slots a second time.
    set("layouts/region","header",`style().gap(px(8))`);
    set("layouts/region","title",`style()${t("subtitle")}.font_weight(500)`);
    set("layouts/region","body",`style()`);
    set("layouts/region","footer",`style().gap(px(8))`);
    for (const id of ["patterns/app_shell","patterns/data_view","patterns/list_detail","patterns/navigation_stack"]) set(id,"root","style()");
    for (const part of ["sidebar","inspector"]) set("patterns/app_shell",part,`style().background(${color("surface.chrome")}).border_color(${edge})`);
    set("patterns/description_list","root",`style().gap(${length("space.related")})`);
    set("patterns/description_list","term",`style()${t("body")}.text_color(${muted})`);
    set("patterns/description_list","value",`style()${t("body")}.text_color(${ink})`);
    for (const id of ["patterns/form_layout","patterns/section"]) {
      set(id,"root",`style().gap(px(16))`);
      set(id,id === "patterns/form_layout" ? "group_title" : "title",`style()${t("subtitle")}.font_weight(500)`);
      set(id,"description",`style()${t("hint")}.text_color(${muted})`);
    }
    set("patterns/form_layout","label",`style()${t("label")}.font_weight(500)`);
    set("patterns/form_layout","error",`style()${t("hint")}.text_color(${color("danger")})`);
    set("patterns/inline_state","root",`style().padding(px(12)).gap(px(8)).border(px(0))${t("body")}`);
    set("patterns/inline_state","title",`style()${t("label")}.font_weight(500)`);
    set("patterns/inline_state","description",`style()${t("body")}.text_color(${muted})`);
    set("patterns/inline_state","lamp",`style().radius(px(999))`);
    set("patterns/stat","root",`style().gap(px(4))`);
    set("patterns/stat","label",`style()${t("caption")}.font_weight(500).text_color(${muted})`);
    set("patterns/stat","value",`style()${t("display")}.font_weight(500)`);
    set("patterns/stat","description",`style()${t("hint")}.text_color(${muted})`);
    set("patterns/composer","root",`style().padding(px(${clean ? 10 : 12})).gap(px(8)).radius(px(${clean ? 12 : 16})).border(px(0)).background(${field})${shadows(smallShadow)}.focus_within(style()${shadows(focusShadow)})`);
    // Motion wrappers preserve their algorithms/ownership. Only visible cards,
    // strips and text receive visual recipes; Canvas series retain data semantics.
    for (const {id} of modules.filter(module => module.id.startsWith("motion/"))) set(id,"root",`style()${t("body")}`);
    set("motion/reorder_list","item",`style().padding(px(12)).radius(px(${clean ? 8 : 12})).border(px(0)).background(${panel})${shadows(smallShadow)}`);
    set("motion/shared_layout_cards","card",`style().padding(px(16)).radius(px(${clean ? 10 : 16})).border(px(0)).background(${panel})${shadows(smallShadow)}`);
    set("motion/shimmer","root",`style().radius(px(${clean ? 4 : 6}))`);
    for (const {id} of modules.filter(module => module.id.startsWith("charts/"))) set(id,"root",`style().border(px(0)).radius(px(${clean ? 8 : 12})).background(${panel})${t("body")}`);
    for (const {id,parts: names} of modules) {
      for (const name of ["scrollbar_track","scrollbar_thumb","scrollbar_thumb_hover"]) {
        if (names.includes(name)) set(id,name,`style().radius(px(999))${name === "scrollbar_track" ? `.background(${transparent})` : `.background(${color(name === "scrollbar_thumb" ? "scrollbar.thumb" : "scrollbar.thumb_hover")})`}`);
      }
      if (!("root" in parts[id]) && names.includes("root")) throw new Error(`missing explicit root recipe: ${id}`);
    }
    return {parts: scalableGeometry(parts), recipes: scalableGeometry(recipes)};
  })();
  return {colors, dimensions, type, ...styled, clean, dark};
}

function object(entries, indent = 8) {
  return `#{\n${Object.entries(entries).map(([name,value]) => `${" ".repeat(indent)}${JSON.stringify(name)}: ${value},`).join("\n")}\n${" ".repeat(indent-4)}}`;
}

// Combine one Modern expression and one Clean expression along the orthogonal `design` axis,
// flattening any inner `by_env(...)` axes (density / size) so no nested EnvTable is emitted.
function mergeByDesign(modernExpr, cleanExpr) {
  if (modernExpr === cleanExpr) return modernExpr;
  const axisValues = {
    density: ["comfortable", "compact"],
    size: ["xs", "sm", "md", "lg"],
  };
  const expandFixed = (expr, keys) => {
    if (keys.length === 0) return expr;
    const [head, ...rest] = keys;
    const entries = axisValues[head].map(v => `${v}: ${expandFixed(expr, rest)}`).join(", ");
    return `#{ ${entries} }`;
  };
  const envMatch = expr => {
    const m = expr.match(/^by_env\((\[[^\]]+\]|"[^"]+"),\s*(#\{[\s\S]*\})\)$/);
    if (!m) return null;
    const keys = m[1].startsWith("[")
      ? [...m[1].matchAll(/"([^"]+)"/g)].map(x => x[1])
      : [m[1].slice(1, -1)];
    return { keys, body: m[2] };
  };
  const m = envMatch(modernExpr);
  const c = envMatch(cleanExpr);
  if (m && c && JSON.stringify(m.keys) === JSON.stringify(c.keys)) {
    return `by_env(${JSON.stringify(["design", ...m.keys])}, #{ modern: ${m.body}, clean: ${c.body} })`;
  }
  if (m && !c) {
    return `by_env(${JSON.stringify(["design", ...m.keys])}, #{ modern: ${m.body}, clean: ${expandFixed(cleanExpr, m.keys)} })`;
  }
  if (!m && c) {
    return `by_env(${JSON.stringify(["design", ...c.keys])}, #{ modern: ${expandFixed(modernExpr, c.keys)}, clean: ${c.body} })`;
  }
  if (m && c) {
    // Different inner axes (e.g. ["density","size"] vs ["size"]): expand the shorter one.
    const allKeys = ["density", "size"].filter(k => m.keys.includes(k) || c.keys.includes(k));
    const lift = (parsed) => {
      if (JSON.stringify(parsed.keys) === JSON.stringify(allKeys)) return parsed.body;
      if (allKeys.length === 2 && parsed.keys.length === 1 && parsed.keys[0] === "size") {
        return `#{ comfortable: ${parsed.body}, compact: ${parsed.body} }`;
      }
      throw new Error(`Unsupported mixed by_env axes: ${m.keys} vs ${c.keys}`);
    };
    return `by_env(${JSON.stringify(["design", ...allKeys])}, #{ modern: ${lift(m)}, clean: ${lift(c)} })`;
  }
  return `by_env("design", #{ modern: ${modernExpr}, clean: ${cleanExpr} })`;
}

function mergeMapsByDesign(modernMap, cleanMap, fallback = undefined) {
  const keys = [...new Set([...Object.keys(modernMap), ...Object.keys(cleanMap)])];
  return Object.fromEntries(keys.map(key => {
    const mVal = key in modernMap ? modernMap[key] : fallback;
    const cVal = key in cleanMap ? cleanMap[key] : fallback;
    if (mVal === undefined || cVal === undefined) {
      throw new Error(`Missing design variant for key ${key}`);
    }
    return [key, mergeByDesign(mVal, cVal)];
  }));
}

// Convert hardcoded light-mode color literals inside a component style expression
// into semantic tokens (5 surfaces, 3 borders, 2 shadows, 8x4 categorical palette,
// or by_mode for neutral overlay backdrops).
function semanticizeStyle(lightExpr, darkExpr, clean) {
  if (lightExpr === undefined) return darkExpr;
  let out = lightExpr;

  // 1. Modal/sheet backdrop opacity differs by light vs dark mode.
  out = out.replaceAll(
    "background(alpha(rgb(0x000000),0.35))",
    "background(by_mode(alpha(rgb(0x000000),0.35),alpha(rgb(0x000000),0.6)))",
  );

  // 2. Shadow outlines & drop-shadow cast colors (context-aware on shadow() calls).
  out = out.replace(/shadow\(#\{([^}]+)\}\)/g, (full, body) => {
    const isOutline = /\bblur:0\b/.test(body) && /\by:0\b/.test(body);
    const isInsetHighlight = /\binset:true\b/.test(body);
    let updated = body;
    if (isOutline) {
      updated = updated
        .replace(/color:(?:rgba\(0x(?:ecececff|e5e5e5ff|e6e6e6ff|2a2a2eff|2e2e33ff|2f2f34ff|2e2e31ff)\)|alpha\(rgb\(0x000000\),0\.0[4-8]\))/g, `color:${color("border_subtle")}`)
        .replace(/color:(?:rgba\(0x(?:d9d9d9ff|d4d4d4ff|3a3a40ff|0d0d0dff)\)|alpha\(rgb\(0x000000\),0\.(?:09|1|1[0-9]|2)\))/g, `color:${color("border_strong")}`);
    } else if (isInsetHighlight) {
      updated = updated
        .replace(/color:(?:rgba\(0xffffffff\)|alpha\(rgb\(0xffffff\),0\.[0-9]+\)|alpha\(rgb\(0x000000\),0\.0[3-6]\))/g, `color:${color("border_subtle")}`);
    } else {
      updated = updated
        .replace(/color:alpha\(rgb\(0x000000\),0\.0[1-6][0-9]?\)/g, `color:${color("shadow.ambient")}`)
        .replace(/color:alpha\(rgb\(0x000000\),0\.(?:0[7-9]|[1-4][0-9]?)\)/g, `color:${color("shadow.key")}`);
    }
    return `shadow(#{${updated}})`;
  });

  // 3. Context-aware replacements for text_color vs background/border_color.
  out = out
    .replaceAll(`.text_color(${rgba("ffffff")})`, `.text_color(${color("on_accent")})`)
    .replaceAll(".text_color(rgb(0xffffff))", `.text_color(${color("on_accent")})`)
    .replaceAll(".text_color(alpha(rgb(0x000000),1))", `.text_color(${color("text_primary")})`)
    .replaceAll(".text_color(alpha(rgb(0x000000),0.88))", `.text_color(${color("text_primary")})`)
    .replaceAll(".text_color(alpha(rgb(0x000000),0.8))", `.text_color(${color("text_primary")})`)
    .replaceAll(".text_color(alpha(rgb(0x000000),0.61))", `.text_color(${color("text_muted")})`)
    .replaceAll(".text_color(alpha(rgb(0x000000),0.45))", `.text_color(${color("text_muted")})`)
    .replaceAll(".text_color(alpha(rgb(0x000000),0.447))", `.text_color(${color("text_muted")})`)
    .replaceAll(".text_color(alpha(rgb(0x000000),0.3))", `.text_color(${color("text_muted")})`)
    .replaceAll(".text_color(alpha(rgb(0x000000),0.27))", `.text_color(${color("text_muted")})`)
    .replaceAll(".background(alpha(rgb(0x000000),0.015))", `.background(${color("surface_hover")})`)
    .replaceAll(".background(alpha(rgb(0x000000),0.018))", `.background(${color("surface_hover")})`)
    .replaceAll(".background(alpha(rgb(0x000000),0.025))", `.background(${color("surface_hover")})`)
    .replaceAll(".background(alpha(rgb(0x000000),0.03))", `.background(${color("surface_sunken")})`)
    .replaceAll(".background(alpha(rgb(0x000000),0.04))", `.background(${color("surface_sunken")})`)
    .replaceAll(".background(alpha(rgb(0x000000),0.05))", `.background(${color("surface_hover")})`)
    .replaceAll(".background(alpha(rgb(0x000000),0.06))", `.background(${color("surface_hover")})`)
    .replaceAll(".background(alpha(rgb(0x000000),0.08))", `.background(${color("surface_hover")})`)
    .replaceAll(".background(alpha(rgb(0x000000),0.8))", `.background(${color("text_primary")})`)
    .replaceAll(".background(alpha(rgb(0x000000),0.88))", `.background(${color("text_primary")})`)
    .replaceAll(".border_color(alpha(rgb(0x000000),0.06))", `.border_color(${color("border_subtle")})`)
    .replaceAll(".border_color(alpha(rgb(0x000000),0.08))", `.border_color(${color("border_subtle")})`)
    .replaceAll(".border_color(alpha(rgb(0x000000),0.1))", `.border_color(${color("border")})`)
    .replaceAll(".border_color(alpha(rgb(0x000000),0.12))", `.border_color(${color("border_strong")})`)
    .replaceAll(".border_color(alpha(rgb(0x000000),0.14))", `.border_color(${color("border_strong")})`);

  // 4. Exhaustive literal color -> semantic token map.
  const substitutions = [
    // Neutral inks
    [rgba("171717"), color("text_primary")],
    [rgba("18181b"), color("text_primary")],
    [rgba("141416"), color("text_primary")],
    [rgba("0d0d0d"), color("text_primary")],
    [rgba("201e25"), color("text_primary")],
    [rgba("26242b"), color("text_primary")],
    [rgba("323137"), color("text_primary")],
    [rgba("3b3a41"), color("text_primary")],
    [rgba("404040"), color("text_primary")],
    [rgba("000000"), color("text_primary")],
    [rgba("525252"), color("text_muted")],
    [rgba("6b6b6b"), color("text_muted")],
    [rgba("737373"), color("text_muted")],
    [rgba("8a8a8a"), color("text_muted")],
    [rgba("8f8f8f"), color("text_muted")],
    [rgba("b5b5b5"), color("text_muted")],
    // Surfaces (5-level hierarchy)
    [rgba("ffffff"), color("surface_raised")],
    ["rgb(0xffffff)", color("surface_raised")],
    [rgba("fcfcfc"), color("surface_raised")],
    [rgba("f4f4f5"), color("surface")],
    [rgba("fafafa"), `mix(${color("surface_raised")},${color("surface_sunken")},0.25)`],
    [rgba("f7f7f7"), color("surface_sunken")],
    [rgba("f5f5f5"), color("surface_sunken")],
    [rgba("f0f0f0"), color("surface_sunken")],
    [rgba("efefef"), color("surface_sunken")],
    [rgba("ebebeb"), color("surface_hover")],
    [rgba("e8e8e8"), color("surface_hover")],
    [rgba("e9e9ec"), color("selection")],
    // Borders (3-level hierarchy)
    [rgba("ececec"), color("border_subtle")],
    ["rgb(0xe6e6e6)", color("border_subtle")],
    [rgba("e5e5e5"), color("border")],
    [rgba("d9d9d9"), color("border_strong")],
    [rgba("d4d4d4"), color("border_strong")],
    [rgba("b8b8b8"), color("border")],
    // Focus / Accent / Categorical hues
    ["alpha(rgb(0x0060e2),0.7)", color("focus_ring")],
    ["alpha(rgb(0x0060e2),0.765)", color("accent")],
    [rgba("eef9f3"), color("palette.green_soft")],
    ["alpha(rgb(0x5fc88a),0.08)", color("palette.green_soft")],
    [rgba("fff0f0"), color("palette.rose_soft")],
    [rgba("006dcbf2"), color("palette.blue_ink")],
    [rgba("008ff519"), color("palette.blue_soft")],
    [rgba("007d2ee0"), color("palette.green_ink")],
    [rgba("00a43319"), color("palette.green_soft")],
    [rgba("d72400da"), color("text.danger")],
    [rgba("f52b0018"), color("badge.danger_bg")],
    [rgba("1e8a4f"), color("palette.green_ink")],
    [rgba("2456d6"), color("palette.blue_ink")],
    [rgba("c4320a"), color("text.danger")],
    [rgba("4fb0fe"), color("palette.blue")],
    [rgba("0190fe40"), `alpha(${color("palette.blue")},0.25)`],
    [rgba("b98af0"), color("palette.violet")],
    ["rgb(0xa651f0)", color("palette.violet")],
    [rgba("f06db5"), color("palette.rose")],
    [rgba("ee3e9e40"), `alpha(${color("palette.rose")},0.25)`],
    // Remaining translucent black/white tokens (for clean mode shadows/washes)
    ["alpha(rgb(0x000000),0.03)", color("surface_sunken")],
    ["alpha(rgb(0x000000),0.04)", color("shadow.ambient")],
    ["alpha(rgb(0x000000),0.05)", color("shadow.ambient")],
    ["alpha(rgb(0x000000),0.06)", color("border_subtle")],
    ["alpha(rgb(0x000000),0.08)", color("shadow.key")],
    ["alpha(rgb(0x000000),0.1)", color("border")],
    ["alpha(rgb(0x000000),0.12)", color("shadow.key")],
    ["alpha(rgb(0x000000),0.14)", color("shadow.key")],
    ["alpha(rgb(0x000000),0.16)", color("shadow.key")],
    ["alpha(rgb(0x000000),0.17)", color("shadow.key")],
    ["alpha(rgb(0x000000),0.18)", color("shadow.key")],
    ["alpha(rgb(0x000000),0.2)", color("shadow.key")],
    ["alpha(rgb(0x000000),0.24)", color("shadow.key")],
    ["alpha(rgb(0x000000),0.27)", color("text_muted")],
    ["alpha(rgb(0x000000),0.3)", color("text_muted")],
    ["alpha(rgb(0x000000),0.45)", color("text_muted")],
    ["alpha(rgb(0x000000),0.61)", color("text_muted")],
    ["alpha(rgb(0x000000),0.8)", color("text_primary")],
    ["alpha(rgb(0x000000),0.88)", color("text_primary")],
    ["alpha(rgb(0x000000),1)", color("text_primary")],
  ];
  for (const [before, after] of substitutions) {
    out = out.replaceAll(before, after);
  }
  return out;
}

function semanticizePreset(preset) {
  const light = recipe(preset, false);
  const dark = recipe(preset, true);
  const clean = preset === "clean";
  // Keep alternate-palette danger button text readable via semantic text.danger.
  light.recipes.button.danger = light.recipes.button.danger.replace(
    `.text_color(${color("danger")})`, `.text_color(${color("text.danger")})`,
  );
  const parts = Object.fromEntries(Object.entries(light.parts).map(([id, map]) => [
    id,
    Object.fromEntries(Object.entries(map).map(([part, expr]) => [
      part,
      semanticizeStyle(expr, dark.parts[id]?.[part], clean),
    ])),
  ]));
  const recipes = Object.fromEntries(Object.entries(light.recipes).map(([ns, map]) => [
    ns,
    Object.fromEntries(Object.entries(map).map(([name, expr]) => [
      name,
      semanticizeStyle(expr, dark.recipes[ns]?.[name], clean),
    ])),
  ]));
  return { ...light, parts, recipes };
}

function dualDesignTokens(modernData, cleanData, baseColors) {
  const dimensions = mergeMapsByDesign(modernData.dimensions, cleanData.dimensions);
  const type = mergeMapsByDesign(modernData.type, cleanData.type);
  const parts = Object.fromEntries(modules.map(({ id }) => [
    id,
    object(mergeMapsByDesign(modernData.parts[id], cleanData.parts[id], "style()"), 12),
  ]));
  const recipes = Object.fromEntries(
    Object.keys(modernData.recipes).map(ns => [
      ns,
      object(mergeMapsByDesign(modernData.recipes[ns], cleanData.recipes[ns], "style()")),
    ]),
  );

  const variables = {
    environment: '#{ design: #{values:["modern","clean"],"default":"modern"},density: #{values:["comfortable","compact"],"default":"comfortable"},size:#{values:["xs","sm","md","lg"],"default":"md"} }',
    spacing: object({xxs:px(2),xs:px(4),sm:density(8,6),md:density(12,8),lg:density(16,12),xl:density(24,20)}),
    space: object({unit:px(4),spacing_unit:density(4,3),related:density(8,6),group:density(16,12),section:density(24,20)}),
    radius: object({
      xs: mergeByDesign(px(5), px(4)),
      sm: mergeByDesign(px(6), px(4)),
      md: mergeByDesign(scale([8,9,10,12]), scale([5,6,7,8])),
      lg: mergeByDesign(px(16), px(12)),
      pill: px(999),
    }),
    metrics: object(dimensions),
    typography: `#{family:"Inter",fallbacks:["PingFang SC","Noto Sans CJK SC","Noto Sans","Arial"],roles:${object(type,12)}}`,
    motion: `#{durations_ms:#{thinking_sweep:1800}}`,
    design: object({
      name: mergeByDesign(JSON.stringify("modern"), JSON.stringify("clean")),
      layered: mergeByDesign("1.0", "0.0"),
    }),
    command: object({
      search_asset: mergeByDesign(JSON.stringify("app/icons/command_search_modern"), JSON.stringify("app/icons/command_search_clean")),
      enter_asset: mergeByDesign(JSON.stringify("app/icons/command_enter_modern"), JSON.stringify("app/icons/command_enter_clean")),
      enter_label: mergeByDesign(JSON.stringify("ENTER"), JSON.stringify("")),
      group_case: mergeByDesign(JSON.stringify("preserve"), JSON.stringify("upper")),
      narrow_footer: mergeByDesign(JSON.stringify("visible"), JSON.stringify("hidden")),
    }),
    spinner: object({
      stroke_width: mergeByDesign("2.0", "1.8"),
      track_alpha: mergeByDesign("0.2", "0.18"),
      speed_ms: mergeByDesign("750.0", "800.0"),
    }),
    checkbox: object({stroke_width:"1.8",draw_ms:"260.0",draw_delay_ms:"40.0",mixed_draw_ms:"200.0"}),
    agent_findings: object({heading_case: mergeByDesign(JSON.stringify("preserve"), JSON.stringify("upper"))}),
    trace: object({heading_case: mergeByDesign(JSON.stringify("preserve"), JSON.stringify("upper"))}),
    component_styles: object(parts),
    ...recipes,
    colors: `palette_colors() + ${object(baseColors)}`,
  };
  return object(variables, 8);
}

function write(file, content) {
  const target = path.join(root,file);
  if (check) {
    if (!fs.existsSync(target) || fs.readFileSync(target,"utf8") !== content) throw new Error(`outdated ${file}`);
  } else if (!fs.existsSync(target) || fs.readFileSync(target,"utf8") !== content) {
    fs.writeFileSync(target,content);
  }
}

// 1. Generate Neutral Light/Dark and expand all 19 existing theme palettes
// directly with 5 surfaces, 3 borders, 2 shadows, and the 8x4 categorical palette.
for (const spec of NEUTRAL_THEMES) {
  const expanded = computeExpandedPaletteColors(spec.file, spec.mode, spec.core);
  write(`registry/themes/${spec.file}`, renderPaletteThemeFile({ ...spec, expanded }));
}

const existingPaletteFiles = fs.readdirSync(path.join(root, "registry/themes"))
  .filter(file => file.endsWith(".rhai") && !/^(modern|clean|neutral)_/.test(file))
  .sort();
for (const file of existingPaletteFiles) {
  const spec = extractExistingPaletteSpec(path.join(root, "registry/themes", file));
  const expanded = computeExpandedPaletteColors(spec.file, spec.mode, spec.core);
  write(`registry/themes/${file}`, renderPaletteThemeFile({ ...spec, expanded }));
}

// 2. Generate compact Modern/Clean compatibility presets: each sets its default
// `design` environment value and exact reference colors while inheriting the
// orthogonal dual-design geometry, typography, and component recipes from tokens.rhai.
for (const preset of ["modern","clean"]) {
  for (const mode of ["light","dark"]) {
    const isDark = mode === "dark";
    const data = recipe(preset, isDark);
    const neutralSpec = NEUTRAL_THEMES.find(t => t.mode === mode);
    const expanded = computeExpandedPaletteColors(neutralSpec.file, mode, neutralSpec.core);
    const exactSunken = {
      modern_light: "0xf0f0f0ff",
      modern_dark: "0x141416ff",
      clean_light: "0xf3f3f4ff",
      clean_dark: "0x111113ff",
    }[`${preset}_${mode}`];
    const presetColors = {
      surface_sunken: exactSunken,
      surface_elevated: expanded.surface_elevated,
      border_subtle: expanded.border_subtle,
      border_strong: expanded.border_strong,
      "shadow.ambient": expanded["shadow.ambient"],
      "shadow.key": expanded["shadow.key"],
      ...Object.fromEntries(CATEGORICAL_HUES.flatMap(hue => [
        [`palette.${hue}`, expanded[`palette.${hue}`]],
        [`palette.${hue}_ink`, expanded[`palette.${hue}_ink`]],
        [`palette.${hue}_soft`, expanded[`palette.${hue}_soft`]],
        [`palette.${hue}_border`, expanded[`palette.${hue}_border`]],
      ])),
      ...data.colors,
    };
    const presetTokens = object({
      environment: `#{ design: #{values:["modern","clean"],"default":${JSON.stringify(preset)}} }`,
      colors: object(presetColors),
    }, 8);
    write(
      `registry/themes/${preset}_${mode}.rhai`,
      `// Generated by scripts/generate-design-presets.mjs; edit the maintained recipes.\n`
      + `// Compatibility preset: Neutral ${mode === "light" ? "Light" : "Dark"} reference colors with default design="${preset}".\n`
      + `fn theme() {\n    #{ family: "${preset === "modern" ? "Modern" : "Clean"}", name: "${mode === "light" ? "Light" : "Dark"}", mode: "${mode}",\n       tokens: ${presetTokens}\n    }\n}\n`,
    );
  }
}

// 3. Build the orthogonal dual-design (modern | clean) + mode-aware token base.
const modernSemantic = semanticizePreset("modern");
const cleanSemantic = semanticizePreset("clean");
const base = modernSemantic;

// Strict guard: zero hardcoded opaque hex literals allowed in tokens.rhai component_styles or recipes.
for (const [label, sheet] of [
  ["modern.parts", modernSemantic.parts],
  ["clean.parts", cleanSemantic.parts],
  ["modern.recipes", modernSemantic.recipes],
  ["clean.recipes", cleanSemantic.recipes],
]) {
  for (const [id, entries] of Object.entries(sheet)) {
    for (const [part, expr] of Object.entries(entries)) {
      const stripped = expr.replaceAll("by_mode(alpha(rgb(0x000000),0.35),alpha(rgb(0x000000),0.6))", "");
      const leaks = [...stripped.matchAll(/0x([0-9a-fA-F]{6,8})/g)]
        .map(m => m[1].toLowerCase())
        .filter(h => h !== "00000000");
      if (leaks.length > 0) {
        throw new Error(`Hardcoded color leak in tokens.rhai ${label} ${id}.${part}: ${leaks.join(", ")} in ${expr}`);
      }
    }
  }
}

const aliases = {};
const paletteToneSurface = (semantic, foreground, weight = .1) =>
  `readable(mix(${color("surface_hover")},${color(semantic)},${weight}),${color("surface_hover")},${color(foreground)},4.5)`;
for (const key of Object.keys(base.colors).filter(key => key.includes("."))) {
  const recap = {
    "agent_recap.area": `alpha(${color("text_primary")},0.07)`, "agent_recap.cursor": color("border"),
    "agent_recap.dot": color("text_muted"), "agent_recap.failed": color("text.danger"), "agent_recap.grid": color("border_subtle"),
    "agent_recap.needs": color("warning"), "agent_recap.stroke": color("text_muted"), "agent_recap.panel": color("surface_raised"),
    "agent_recap.tray": color("surface_sunken"), "agent_recap.text": color("text_muted"), "agent_recap.quiet": color("text_muted"),
    "agent_recap.faint": color("text_muted"), "agent_recap.soft": color("surface_sunken"), "agent_recap.ok": color("text.success"),
    "agent_recap.ok_bg": color("palette.green_soft"),
  };
  if (key in recap) { aliases[key]=recap[key]; continue; }
  let target = "text_primary";
  if (/^(surface|field)\./.test(key) || /secondary_(bg|hover)/.test(key)) target = "surface_raised";
  if (/\.(card|panel|paper)$/.test(key)) target = "surface_raised";
  if (/\.(popover|popup)$/.test(key)) target = "surface_elevated";
  if (/\.(tray|well|inset|track|free|slot|idle|wait)$/.test(key)) target = "surface_sunken";
  if (/placeholder|subtle|icon|ghost_fg|muted|faint|quiet/.test(key)) target = "text_muted";
  if (/border|edge|line|grid|rule|rail/.test(key)) target = "border_subtle";
  if (/hover|counter|map_missing/.test(key)) target = "surface_hover";
  if (/selection|tabbar.active/.test(key)) target = "selection";
  if (/primary_(bg|bottom|hover)/.test(key)) target = "accent";
  if (key === "button.primary_fg") target = "on_accent";
  if (key === "button.ring") target = "accent";
  if (key === "control.focus_edge") target = "focus_ring";
  if (key === "text.secondary" || key.startsWith("scrollbar.")) target = "text_muted";
  const tone = key.match(/^badge\.(neutral|info|success|warning|danger|purple)_(bg|fg)$/);
  if (tone) {
    if (tone[1] === "purple") {
      aliases[key] = color(tone[2] === "bg" ? "palette.violet_soft" : "palette.violet_ink");
      continue;
    }
    const semantic = {neutral:"text_muted",info:"accent",success:"success",warning:"warning",danger:"danger"}[tone[1]];
    const foreground = {neutral:"text_muted",info:"text.accent",success:"text.success",warning:"text.warning",danger:"text.danger"}[tone[1]];
    aliases[key] = tone[2] === "bg" ? paletteToneSurface(semantic,foreground) : color(foreground);
  } else aliases[key] = color(target);
}
// Upgrade component aliases to use the 5-level Surface hierarchy, 3-level Border hierarchy,
// and the 8x4 non-semantic Categorical Palette across every bundled theme.
for (const [role, fallback] of Object.entries({
  card: "surface_raised", ink: "text_primary", tray: "surface_sunken", muted: "text_muted",
  tab: "surface_sunken", tab_ink: "text_primary", face_edge: "border_subtle",
})) aliases[`agent_cursors.${role}`] = color(fallback);
for (const [role, fallback] of Object.entries({
  card: "surface_raised", tray: "surface_sunken", chip: "surface_sunken",
  chip_hover: "selection", choice: "surface.field", choice_edge: "border",
  choice_on: "choice.fill", mark: "choice.on_fill", hover: "surface_hover",
})) aliases[`filter_menu.${role}`] = color(fallback);
for (const [role, fallback] of Object.entries({
  panel:"surface_raised",tray:"surface_sunken",line:"border_subtle",fill:"surface_sunken",hover:"surface_hover",selected:"selection",
  quiet:"text_muted",faint:"text_muted",critical:"text.danger",warning:"text.warning",note:"palette.blue_ink",spark:"palette.blue",
  run:"palette.blue",run_edge:"focus_ring",done:"surface_sunken",done_edge:"border_subtle",failed:"badge.danger_bg",failed_edge:"danger",sheen:"surface_raised",
  avatar_ink:"text_muted",
})) aliases[`agent_findings.${role}`] = color(fallback);
for (const [role, fallback] of Object.entries({
  tray:"surface_sunken",panel:"surface_raised",icon:"surface_raised",text:"text_muted",quiet:"text_muted",faint:"text_muted",
  rule:"border_subtle",hover:"surface_hover",preview:"selection",also:"palette.blue_soft",also_chip:"palette.blue_soft",also_ink:"palette.blue_ink",
  kept:"palette.amber_soft",kept_chip:"palette.amber_soft",kept_ink:"palette.amber_ink",strike:"text_muted",accent:"accent",
})) aliases[`agent_ledger.${role}`] = color(fallback);
for (const [role,fallback] of Object.entries({
  c1:"palette.1",c2:"palette.2",c3:"palette.3",c4:"palette.4",c5:"palette.5",
  card:"surface_raised",muted:"text_muted",up:"success",down:"danger",up_bg:"palette.green_soft",down_bg:"palette.rose_soft",
})) {
  aliases[`compact_breakdown.${role}`] = color(fallback);
}
for (const [role,fallback] of Object.entries({accent:"accent",track:"surface_sunken",spark:"border",warn:"warning",warn_text:"text.warning",full:"danger",full_text:"text.danger",fill:"surface_sunken"})) {
  aliases[`usage_meter.${role}`] = color(fallback);
}
for (const [role,fallback] of Object.entries({icon:"accent",good:"text.success",bad:"text.danger"})) {
  aliases[`change_summary.${role}`] = color(fallback);
}
aliases["tips_panel.done"] = color("text.success");
for(const [key,fallback] of Object.entries({art:"border_subtle",art_dot:"accent",badge:"surface_sunken",peek:"surface_sunken"})){
  aliases[`notification_stack.${key}`] = color(fallback);
}
for(const [key,fallback] of Object.entries({needs:"warning",done:"accent",failed:"danger",muted:"text_muted",text:"text_muted",tray:"surface_sunken",card:"surface_raised"})){
  aliases[`away_summary.${key}`] = color(fallback);
}
aliases["agent_steering.tag"] = color("palette.blue_ink");
aliases["agent_steering.note_bg"] = color("surface_sunken");
for (const [part,fallback] of Object.entries({first:"palette.1",second:"palette.2",third:"palette.3",grid:"border_subtle",guide:"text_muted"})) {
  aliases[`kpi_chart.${part}`] = color(fallback);
}
for (const [part,fallback] of Object.entries({
  c1:"palette.1",c2:"palette.2",c3:"palette.3",c4:"palette.4",c5:"palette.5",c6:"palette.6",c7:"palette.7",
  grid:"border_subtle",muted:"text_muted",rest:"surface_sunken",zero:"text_muted",
})) {
  aliases[`breakdown_chart.${part}`] = color(fallback);
}
for (const [part,fallback] of Object.entries({
  teal:"palette.teal",amber:"palette.amber",orange:"palette.orange",blue:"palette.blue",
  badge:"palette.amber_soft",badge_text:"palette.amber_ink",trigger_ink:"text_muted",paper:"surface_raised",faint:"text.subtle",
})) {
  aliases[`running_now.${part}`] = color(fallback);
}
for (const [part,fallback] of Object.entries({
  capacity:"palette.green",card:"surface_raised",ink:"text_primary",current:"palette.rose",drop:"selection",
  lane:"surface_sunken",track:"border_subtle",well:"surface_sunken",
  muted:"text_muted",text:"text_primary",faint:"field.placeholder",
})) {
  aliases[`scheduled_runs.${part}`] = color(fallback);
}
aliases["scheduled_runs.pill"] = `alpha(${color("text_primary")},0.06)`;
aliases["scheduled_runs.count_bg"] = `alpha(${color("palette.violet")},0.06)`;
aliases["cost_waterfall.card"] = color("surface_raised");
aliases["parallel_attempts.card"] = color("surface_raised");
aliases["scheduled_runs.hatch"] = `alpha(${color("text_muted")},0.07)`;
for (const [part,fallback] of Object.entries({tile:"surface_sunken",code:"surface_sunken",error:"badge.danger_bg",error_border:"control.danger_hover"})) {
  aliases[`tool_call.${part}`] = color(fallback);
}
for (const [part,fallback] of Object.entries({
  c1:"palette.1",c2:"palette.2",c3:"palette.3",c4:"palette.4",c5:"palette.5",c6:"palette.6",
  free:"surface_sunken",shine:"text_primary",warn:"text.warning",warn_bg:"palette.amber_soft",full_bg:"badge.danger_bg",saved:"text.success",muted:"text_muted",
})) {
  aliases[`context_window.${part}`] = color(fallback);
}
const contextMapHue = {
  system: "slate", tools: "blue", memory: "violet", file: "teal",
  chat: "orange", result: "amber", summary: "green",
};
for (const [kind, hue] of Object.entries(contextMapHue)) {
  aliases[`context_map.${kind}_bg`] = color(`palette.${hue}_soft`);
  aliases[`context_map.${kind}_edge`] = color(`palette.${hue}_border`);
  aliases[`context_map.${kind}_ink`] = color(`palette.${hue}_ink`);
}
aliases["context_map.free_edge"] = color("border_subtle");
aliases["context_map.free_hatch"] = `alpha(${color("text_muted")},0.07)`;
aliases["context_map.model"] = color("text_muted");
aliases["context_map.muted"] = color("text_muted");
aliases["context_map.empty"] = color("text_muted");
aliases["context_map.tray"] = color("surface_sunken");
aliases["context_map.shine"] = color("text_muted");
aliases["context_map.panel"] = color("surface_raised");
aliases["context_map.capacity_bg"] = color("surface_sunken");
aliases["context_map.capacity_edge"] = color("border_subtle");
aliases["context_map.capacity_warn_bg"] = color("palette.amber_soft");
aliases["context_map.capacity_full_bg"] = color("badge.danger_bg");
aliases["context_map.capacity_full_ink"] = color("badge.danger_fg");
for (const [part, fallback] of Object.entries({
  accent:"accent",badge_ink:"text.danger",badge_bg:"badge.danger_bg",muted:"text_muted",setup_ink:"text_primary",track:"surface_sunken",tray:"surface_sunken",
})) aliases[`profile_menu.${part}`] = color(fallback);
for (const [part, fallback] of Object.entries({
  card:"surface_raised",tray:"surface_sunken",warn:"text.warning",warn_bg:"palette.amber_soft",ok:"text.success",
  ok_bg:"palette.green_soft",chip:"surface_sunken",account:"text_muted",address:"text_muted",file:"text_muted",hint:"text_muted",tone_ink:"text_muted",tone_edge:"border_subtle",link:"accent",link_underline:"text_muted",send:"button.primary_bg",on_send:"button.primary_fg",
})) aliases[`email_draft.${part}`] = color(fallback);
aliases["email_draft.tone_hover"] = transparent;
for (const [part, fallback] of Object.entries({
  add:"palette.green",add_ink:"palette.green_ink",binary:"palette.slate_ink",card:"surface_raised",code:"palette.blue",del:"palette.rose",del_ink:"palette.rose_ink",
  faint:"field.placeholder",muted:"text_muted",grid:"border_subtle",image:"palette.violet",net:"text_muted",sheet:"palette.teal",tray:"surface_sunken",wash:"selection",
})) aliases[`diff_timeline.${part}`] = color(fallback);
for (const [kind, set] of Object.entries({
  agent:["palette.slate_soft","palette.slate_border","border_strong","palette.slate"],
  llm:["palette.violet_soft","palette.violet_border","palette.violet_soft","palette.violet"],
  tool:["palette.teal_soft","palette.teal_border","palette.teal_soft","palette.teal"],
  error:["badge.danger_bg","danger","control.danger_hover"],
})) for (const [i, fallback] of set.entries())
  aliases[`cost_waterfall.${kind}_${["bg","edge","out","mark"][i]}`] = color(fallback);
aliases["cost_waterfall.tray"] = color("surface_sunken");
aliases["cost_waterfall.muted"] = color("text_muted");
for (const [part,fallback] of Object.entries({
  best_bg:"palette.green_soft",best_edge:"palette.green_border",done_bg:"palette.green_soft",done_edge:"palette.green_border",done_ink:"palette.green_ink",
  failed_bg:"badge.danger_bg",failed_edge:"danger",failed_ink:"text.danger",running_bg:"palette.blue_soft",running_edge:"palette.blue_border",running_ink:"palette.blue_ink",
  icon:"surface_raised",mark:"accent",shine:"surface_raised",track:"surface_sunken",tray:"surface_sunken",wash:"palette.green_soft",
  strong:"text_primary",ink:"text_primary",text:"text_muted",muted:"text_muted",action_hover:"surface_hover",
})) aliases[`parallel_attempts.${part}`] = color(fallback);
for (const [status, fallback] of Object.entries({
  queued:["surface_sunken","border_subtle","text_muted"],
  running:["palette.blue_soft","palette.blue_border","palette.blue_ink"],
  retrying:["palette.amber_soft","palette.amber_border","palette.amber_ink"],
  done:["palette.green_soft","palette.green_border","palette.green_ink"],
  failed:["badge.danger_bg","danger","text.danger"],
})) {
  for(const [i, paint] of fallback.entries()) aliases[`fan_out_grid.${status}_${["bg","edge","ink"][i]}`] = color(paint);
}
for(const [part,fallback] of Object.entries({mark:"accent",pin:"focus_ring",shine:"surface_raised",tray:"surface_sunken",
  paper:"surface_raised",strong:"text_primary",ink:"text_primary",muted:"text_muted",filter_wash:"surface_sunken"})){
  aliases[`fan_out_grid.${part}`] = color(fallback);
}
for(const [part,fallback] of Object.entries({card:"surface_raised",inset:"surface_sunken",line:"border_subtle",tray:"surface_sunken",
  ink:"text_primary",text:"text_primary",muted:"text_muted",tint:"text.accent"})){
  aliases[`billing_schedule.${part}`] = color(fallback);
}
for(const [part,fallback] of Object.entries({
  actual:"palette.violet_soft",actual_edge:"palette.violet_border",actual_mark:"palette.violet",ahead:"palette.green_ink",ahead_fill:"palette.green_soft",ahead_gap:"palette.green_soft",
  card:"surface_raised",forecast:"surface_sunken",forecast_edge:"border_subtle",ghost:"text_muted",grid:"border_subtle",late:"palette.amber_ink",late_edge:"palette.amber_border",
  late_fill:"palette.amber_soft",late_gap:"palette.amber_soft",link:"text_muted",live:"palette.blue_ink",live_line:"palette.blue",muted:"text_muted",quiet:"text.subtle",wash:"surface_sunken",
})){
  aliases[`plan_vs_actual.${part}`] = color(fallback);
}
for(const [part,fallback] of Object.entries({
  tray:"surface_sunken",area:"surface_raised",card:"surface_raised",grid:"border_subtle",idle:"surface_sunken",wait:"surface_sunken",
  fill:"surface_sunken",ink:"text_primary",text:"text_muted",
  muted:"text_muted",quiet:"text.subtle",rail:"border",active:"palette.blue_ink",active_edge:"palette.blue_border",active_fill:"palette.blue_soft",
  ok:"palette.green_ink",ok_edge:"palette.green_border",ok_fill:"palette.green_soft",error:"text.danger",error_edge:"danger",error_fill:"badge.danger_bg",popover:"surface_elevated",
})){
  aliases[`fallback_chain.${part}`] = color(fallback);
}
for(const [part,fallback] of Object.entries({
  tray:"surface_sunken",card:"surface_raised",well:"surface_sunken",hover:"surface_hover",fill:"surface_sunken",ink:"text_primary",text:"text_muted",muted:"text_muted",track:"switch.off",
  track_on:"accent",knob:"on_accent",danger:"text.danger",danger_fill:"badge.danger_bg",paused:"palette.amber_ink",
  paused_fill:"palette.amber_soft",solid:"text_primary",on_solid:"surface_raised",
})){
  aliases[`memory_panel.${part}`] = color(fallback);
}
for(const [part,fallback] of Object.entries({
  tray:"surface_sunken",card:"surface_raised",well:"surface_sunken",fill:"surface_sunken",ink:"text_primary",text:"text_muted",muted:"text_muted",rail:"border",
  used:"palette.blue",glint:"focus_ring",live:"palette.blue_ink",live_fill:"palette.blue_soft",success:"palette.green_ink",
  success_fill:"palette.green_soft",warning:"palette.amber_ink",warning_fill:"palette.amber_soft",danger:"text.danger",
  danger_fill:"badge.danger_bg",solid:"text_primary",on_solid:"surface_raised",
})) aliases[`tool_access.${part}`] = color(fallback);
for(const [part,fallback] of Object.entries({
  tray:"surface_sunken",card:"surface_raised",muted:"text_muted",text:"text_muted",grid:"border_subtle",guide:"border_subtle",hover:"surface_hover",selected:"selection",
  well:"surface_sunken",accent:"accent",now:"accent",agent_bg:"palette.slate_soft",agent_edge:"palette.slate_border",agent_ink:"palette.slate_ink",agent_mark:"palette.slate",
  llm_bg:"palette.violet_soft",llm_edge:"palette.violet_border",llm_ink:"palette.violet_ink",llm_mark:"palette.violet",
  tool_bg:"palette.teal_soft",tool_edge:"palette.teal_border",tool_ink:"palette.teal_ink",tool_mark:"palette.teal",
  error_bg:"badge.danger_bg",error_edge:"danger",error_ink:"text.danger",error_mark:"danger",
  live_bg:"palette.blue_soft",live_ink:"palette.blue_ink",done_bg:"palette.green_soft",done_ink:"palette.green_ink",slow_bg:"palette.amber_soft",slow_ink:"palette.amber_ink",
})) aliases[`trace.${part}`] = color(fallback);
for(const [part,fallback] of Object.entries({
  tray:"surface_sunken",card:"surface_raised",paper:"surface_raised",reason:"text_primary",muted:"text_muted",line:"border_subtle",
  hover:"surface_hover",chip:"palette.violet_soft",chip_ink:"palette.violet_ink",idle:"border",signal0:"text_muted",
  weak:"palette.amber",good:"palette.blue",strong:"palette.green",success:"palette.green_ink",success_bg:"palette.green_soft",action:"surface_raised",
  action_hover:"surface_hover",solid:"text_primary",on_solid:"surface_raised",popup:"surface_elevated",selected:"selection",selected_edge:"accent",
  tag:"palette.blue_soft",
})) aliases[`recommendation.${part}`] = color(fallback);
for(const [part,fallback] of Object.entries({
  card:"surface_raised",ink:"text_primary",muted:"text_muted",line:"border_subtle",guide:"border_subtle",stroke:"border",actual:"text_muted",wash:"surface_sunken",
  accent:"accent",warm:"palette.amber_ink",warm_bg:"palette.amber_soft",danger:"text.danger",danger_bg:"badge.danger_bg",
  warn:"danger",
  okay:"palette.green_ink",okay_bg:"palette.green_soft",popup:"surface_elevated",button:"text_primary",on_button:"surface_raised",tag:"surface_sunken",tag_over:"badge.danger_bg",
  blue:"palette.blue",purple:"palette.violet",teal:"palette.teal",pink:"palette.rose",cyan:"palette.teal",green:"palette.green",amber:"palette.amber",
})) aliases[`rate_plan.${part}`] = color(fallback);
aliases["rate_plan.hatch"] = `alpha(${color("danger")},0.30)`;
aliases["rate_plan.hatch_bg"] = `alpha(${color("danger")},0.05)`;
for(const [part,fallback] of Object.entries({
  tray:"surface_sunken",card:"surface_raised",meta:"text_muted",muted:"text_muted",text:"text_muted",faint:"text_muted",line:"border_subtle",
  ink:"text_primary",shine:"text_muted",
  fill:"surface_sunken",well:"surface_sunken",rail:"border",path:"accent",brand:"palette.blue_ink",brand_bg:"palette.blue_soft",
  danger:"text.danger",danger_bg:"badge.danger_bg",solid:"text_primary",on_solid:"surface_raised",
})) aliases[`root_cause.${part}`] = color(fallback);
for(const [part,fallback] of Object.entries({
  tray:"surface_sunken",card:"surface_raised",ink:"text_primary",muted:"text_muted",faint:"text_muted",line:"border_subtle",
  dash:"border",hint:"surface",rail:"border",won:"text_muted",lost:"border",path:"accent",blue:"palette.blue_ink",
  tag:"palette.blue_soft",well:"surface_sunken",solid:"text_primary",on_solid:"surface_raised",shine:"text_muted",
})) aliases[`bracket.${part}`] = color(fallback);
for(const [part,fallback] of Object.entries({
  tray:"surface_sunken",card:"surface_raised",bubble:"surface_sunken",hover:"surface_hover",ink:"text_primary",text:"text_primary",
  muted:"text_muted",faint:"text_muted",fill:"surface_sunken",well:"surface_sunken",you:"surface_raised",line:"border_subtle",
  accent:"accent",solid:"text_primary",inverse:"surface_raised",wait_fg:"palette.amber_ink",wait_bg:"palette.amber_soft",
  blue:"palette.blue",violet:"palette.violet",cyan:"palette.teal",orange:"palette.orange",green:"palette.green",pink:"palette.rose",shine:"text_primary",
})) aliases[`stream.${part}`] = color(fallback);
for(const [part,fallback] of Object.entries({
  tray:"surface_sunken",card:"surface_raised",ink:"text_primary",text:"text_primary",muted:"text_muted",faint:"text_muted",line:"border_subtle",
  fill:"surface_sunken",hover:"surface_hover",stripe:"surface",edit:"palette.blue_soft",edit_edge:"palette.blue_border",hit:"palette.amber_soft",
  hit_edge:"palette.amber_border",hit_ink:"palette.amber_ink",hit_row:"palette.amber_soft",kept:"palette.green_soft",kept_edge:"palette.green_border",
  kept_ink:"palette.green_ink",accent:"accent",chip:"text_primary",inverse:"surface_raised",sheen:"surface_raised",
})) aliases[`radar.${part}`] = color(fallback);
for(const [key,fallback] of Object.entries({
  accent:"accent",ring:"border_subtle",ring_hot:"text_muted",card:"surface_raised",tray:"surface_sunken",muted:"text_muted",
  gray:"palette.slate_ink",violet:"palette.violet_ink",amber:"palette.amber_ink",blue:"palette.blue_ink",green:"palette.green_ink",red:"palette.rose_ink",
})){
  aliases[`project_card.${key}`] = color(fallback);
}
const projectToneToHue = {red:"rose",amber:"amber",green:"green",blue:"blue",violet:"violet",gray:"slate"};
for(const [tone, hue] of Object.entries(projectToneToHue)){
  aliases[`project_card.${tone}_wash`]=color(`palette.${hue}_soft`);
  aliases[`project_card.${tone}_active`]=color(`palette.${hue}_border`);
}
for(const [role,fallback] of Object.entries({option_text:"text_muted",option_ink:"text_primary",option_line:"border_subtle",option_fill:"selection",option_hover:"surface_hover"}))
  aliases[`project_card.${role}`]=color(fallback);
for(const [key,fallback] of Object.entries({
  control:"palette.slate",a:"palette.blue",b:"palette.violet",c:"palette.amber",d:"palette.green",
  track:"surface_sunken",card:"surface_raised",tray:"surface_sunken",muted:"text_muted",
})){
  aliases[`experiment_card.${key}`] = color(fallback);
}
for(const [tone, hue] of Object.entries(projectToneToHue)){
  aliases[`experiment_card.chip_${tone}_ink`]=color(`palette.${hue}_ink`);
  aliases[`experiment_card.chip_${tone}_wash`]=color(`palette.${hue}_soft`);
}
for (const [role, fallback] of Object.entries({
  card: "surface_raised", tray: "surface_sunken", track: "border_subtle", progress: "accent",
  success_fill: "palette.green_soft",
})) aliases[`stepper.${role}`] = color(fallback);
for (const [role, fallback] of Object.entries({
  track: "surface_sunken", tracker_fill: "accent", setup_fill: "accent",
  ring: "border", muted: "text_muted", open: "surface_sunken",
  locked_icon: "text_muted", current_dot: "border",
})) aliases[`checklist.${role}`] = color(fallback);
for (const [role, fallback] of Object.entries({
  ink:"text_primary", like:"palette.rose_ink", comment:"palette.blue_ink", repost:"palette.green_ink", share:"palette.violet_ink",
  selected:"accent", selected_fill:"selection", selected_edge:"border_subtle",
})) aliases[`reaction.${role}`] = color(fallback);
for (const [role, fallback] of Object.entries({
  highlight:"selection", well:"surface_sunken", tray:"surface_sunken",
})) aliases[`cite.${role}`] = color(fallback);
for (const [role, fallback] of Object.entries({
  sheen:"surface_raised", ink:"text_muted", check:"text.success",
})) aliases[`thinking.${role}`] = color(fallback);
for (const [role,fallback] of Object.entries({
  mark:"accent", tray:"surface_sunken", queue:"surface_sunken", steer:"selection", steer_ink:"text.accent",
})) aliases[`run.${role}`] = color(fallback);
for (const [tone, hue] of Object.entries({app:"slate",blue:"blue",violet:"violet",amber:"amber",green:"green",gray:"slate"})) {
  aliases[`catalog.tile_${tone}`] = color(`palette.${hue}_soft`);
  aliases[`catalog.mark_${tone}`] = color(`palette.${hue}_ink`);
}
aliases["writing.glow_pink"] = `alpha(${color("palette.rose")},0.22)`;
aliases["writing.glow_violet"] = `alpha(${color("palette.violet")},0.22)`;
aliases["writing.glow_blue"] = `alpha(${color("palette.blue")},0.22)`;
aliases["button.danger_bg"] = paletteToneSurface("danger","text.danger");
aliases["button.danger_hover"] = paletteToneSurface("danger","text.danger",.16);
aliases["card.tray"] = color("surface_sunken");
aliases["kbd.surface"] = color("surface_raised");
aliases["kbd.ink"] = color("text_muted");
aliases["kbd.edge"] = color("border_subtle");
aliases["kbd.depth"] = color("border");
aliases["card.selected"] = color("selection");
aliases["card.body"] = color("text_primary");
aliases["card.description"] = color("text_muted");
aliases["choice.fill"] = color("accent");
aliases["choice.on_fill"] = color("on_accent");
aliases["choice.description"] = color("text_muted");
aliases["switch.off"] = color("surface_hover");
aliases["switch.off_hover"] = color("control.hover");
aliases["switch.on_hover"] = color("accent_hover");
aliases["switch.thumb"] = color("text_primary");
aliases["switch.thumb_off_ink"] = color("surface");
aliases["switch.checked_thumb"] = color("on_accent");
aliases["collection.selection"] = color("selection");
aliases["table.surface"] = color("surface_raised");
aliases["table.edge"] = color("border_subtle");
aliases["table.grid"] = color("border_subtle");
aliases["table.header_ink"] = color("text_muted");
aliases["table.text"] = color("text_primary");
aliases["table.icon"] = color("text_muted");
aliases["table.header_hover"] = color("surface_hover");
aliases["table.header_hover_ink"] = color("text_primary");
aliases["table.row_hover"] = color("surface_hover");
aliases["table.sort_ink"] = color("text.accent");
aliases["kbd.menu_surface"] = color("surface_raised");
aliases["kbd.menu_edge"] = color("border_subtle");
aliases["command.shortcut"] = color("collection.metadata");
aliases["command.shortcut_active"] = color("text_primary");
aliases["command.surface"] = color("surface_elevated");
aliases["command.tray"] = color("surface_sunken");
aliases["command.line"] = color("border_subtle");
aliases["command.shadow_edge"] = color("border_subtle");
aliases["command.ink"] = color("text_primary");
aliases["command.placeholder"] = color("field.placeholder");
aliases["command.icon"] = color("text_muted");
aliases["command.hint"] = color("collection.metadata");
aliases["command.logo_surface"] = color("surface_raised");
aliases["command.logo_edge"] = color("border_subtle");
aliases["command.logo_ink"] = color("text_primary");
aliases["command.group"] = color("collection.metadata");
aliases["command.selection"] = color("selection");
aliases["command.footer"] = color("collection.metadata");
aliases["field.counter_full"] = color("text_muted");
aliases["collection.metadata"] = metadataInk();
for (const tone of ["success", "warning"]) {
  const normal = color(`badge.${tone}_bg`);
  const hover = `mix(${normal},${color(`badge.${tone}_fg`)},0.08)`;
  aliases[`button.${tone}_fg`] = derivedInk(color(`badge.${tone}_fg`),
    [`${normal}.over(${color("surface_raised")})`, `${hover}.over(${color("surface_raised")})`]);
}
aliases["tabs.pill"] = color("surface_raised");
aliases["tabs.ink"] = color("text_primary");
aliases["tabs.hover"] = color("text_primary");
aliases["tabs.counter_fg"] = color("text_primary");
aliases["tabs.icon"] = color("text_muted");
aliases["tabs.line"] = color("border_subtle");
aliases["pagination.ink"] = color("text_primary");
aliases["pagination.text"] = color("text_primary");
aliases["pagination.muted"] = color("text_muted");
aliases["pagination.faint"] = color("text_muted");
aliases["pagination.wash"] = color("surface_hover");
aliases["pagination.active"] = color("selection");
aliases["pagination.surface"] = color("surface_raised");
aliases["pagination.surface_hover"] = color("surface_hover");
aliases["pagination.edge"] = color("border_subtle");
aliases["pagination.edge_hover"] = color("border_strong");
aliases["tooltip.surface"] = color("text_primary");
aliases["tooltip.ink"] = color("surface");
aliases["tooltip.muted"] = `alpha(${color("surface")},0.72)`;
aliases["tooltip.key"] = `alpha(${color("surface")},0.14)`;
aliases["tooltip.shadow"] = color("shadow.key");
aliases["tooltip.shadow_secondary"] = color("shadow.ambient");
aliases["tag.chip"] = color("surface_sunken");
aliases["tag.edge"] = color("border_subtle");
aliases["tracker.sheen"] = `alpha(${color("text_primary")},0.12)`;
for (const tone of badgeTones) {
  aliases[`badge.${tone}_strong_bg`] = `${color(`badge.${tone}_fg`)}.over(${color("surface_raised")})`;
  aliases[`badge.${tone}_strong_fg`] = strongInk(color(`badge.${tone}_strong_bg`));
}
for (const tone of ["info","success","warning","danger"]) {
  const semantic = tone === "info" ? "accent" : tone;
  const foreground = color(`text.${semantic}`);
  aliases[`tag.${tone}_fg`] = foreground;
  aliases[`tag.${tone}_bg`] = paletteToneSurface(semantic,`text.${semantic}`);
  const variant = tone === "info" ? "accent" : tone;
  aliases[`tag.text_${variant}_ink`] = `readable(${color(`tag.${tone}_fg`)},${color("text_primary")},${color("tag.chip")}.over(${color("surface_raised")}),4.5)`;
  aliases[`tag.facet_${variant}_ink`] = `readable(${color(`tag.${tone}_fg`)},${color("text_primary")},${color(`tag.${tone}_bg`)},4.5)`;
}
const paletteHelper = fs.readFileSync(path.join(root,"registry/presets/palette.rhai"),"utf8");
for (const match of paletteHelper.matchAll(/"([^"]+)"\s*:/g)) delete aliases[match[1]];
write("registry/tokens.rhai",`// Generated by scripts/generate-design-presets.mjs; edit the maintained recipes.\n${paletteHelper}\nfn tokens() {\n    ${dualDesignTokens(modernSemantic, cleanSemantic, aliases)}\n}\n`);
write("docs/design/component-coverage.md", `# Modern / Clean module coverage

Generated from the maintained recipes and public SDK modules; this is not a
reference-project survey. Both designs and both appearances declare every
module below. Counts include intentionally empty structural/interaction root
recipes, which must not acquire decorative surfaces.

Recipe presence does **not** mean visual or interaction acceptance. The
canonical presentation/acceptance application is ${tick}gpui-rhai gallery${tick}.
The native ${tick}gallery::module_coverage${tick} gate traverses its committed
composition in all four presets; importing or naming a component does not
count. Opening overlays, virtual content, native painted states, keyboard and
lifecycle behavior remain separate checks. The complete visual/motion gates
in the [design specification](modern-clean-spec.md) are still required.

| Module | Explicit part recipes |
|---|---:|
${modules.map(module => `| ${tick}${module.id}${tick} | ${Object.keys(base.parts[module.id]).length} |`).join("\n")}

Total: **${modules.length} public modules**, excluding internal functions and
without counting each series in the generic Chart module as a separate module.

Application services, editable spreadsheet semantics and other new product
capabilities are not added by this visual migration. Existing native state,
keyboard, controlled-value and lifecycle contracts remain in force.
`);
console.log(`Validated ${modules.length} module recipes in four preset configurations.`);
