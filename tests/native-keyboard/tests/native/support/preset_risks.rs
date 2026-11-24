//! Acceptance policy for fixed, unmodified Gallery/example fixtures only.
//!
//! The SDK audit remains strict. Preserve its raw findings, record explicitly
//! reviewed stock-palette cases, and reject every other finding. This is not a
//! production waiver for arbitrary applications, copied components or overrides.

use gpui_rhai::{
    AuditContrastEvidence, AuditFinding, AuditRule, RuntimeEngine, ThemeTokenOverrides,
    ThemeVariant,
};

pub(crate) struct PresetRisks {
    stock: Vec<ThemeVariant>,
    recorded: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
struct Paper {
    token: &'static str,
    opacity: f64,
}

#[derive(Clone, Copy, Debug)]
struct IsolatedPaper {
    token: &'static str,
    underlay: Option<&'static str>,
    grayscale: f64,
    wash: Option<u32>,
}

struct PaintWitness {
    ink: &'static str,
    papers: Vec<Paper>,
    fixed_paper: Option<[f64; 3]>,
    fixed_ink: Option<u32>,
    opacity: f64,
    isolated_paper: Option<IsolatedPaper>,
}

impl PaintWitness {
    fn new(ink: &'static str, papers: &[&'static str]) -> Self {
        Self {
            ink,
            papers: papers
                .iter()
                .map(|token| Paper {
                    token,
                    opacity: 1.0,
                })
                .collect(),
            fixed_paper: None,
            fixed_ink: None,
            opacity: 1.0,
            isolated_paper: None,
        }
    }

    // Some preset surfaces are literals in the component recipe, rather
    // than tokens. Keep them in the witness without borrowing another
    // component's coincidentally equal token.
    fn on_literal(ink: &'static str, paper: u32) -> Self {
        let mut witness = Self::new(ink, &[]);
        let [_, r, g, b] = paper.to_be_bytes();
        witness.fixed_paper = Some([r, g, b].map(|v| f64::from(v) / 255.0));
        witness
    }

    fn with_literal_layer(mut self, rgba: u32) -> Self {
        self.fixed_paper = self
            .fixed_paper
            .map(|paper| rgba_over(gpui_rhai::Rgba8::from_rgba_hex(rgba), 1.0, paper));
        self
    }

    fn with_literal_ink(mut self, rgba: u32) -> Self {
        self.fixed_ink = Some(rgba);
        self
    }

    fn isolated_on(mut self, token: &'static str, opacity: f64) -> Self {
        self.opacity = opacity;
        self.isolated_paper = Some(IsolatedPaper {
            token,
            underlay: None,
            grayscale: 0.0,
            wash: None,
        });
        self
    }

    fn with_isolated_underlay(mut self, token: &'static str) -> Self {
        self.isolated_paper
            .as_mut()
            .expect("a group backing")
            .underlay = Some(token);
        self
    }

    fn with_isolated_wash(mut self, rgba: u32) -> Self {
        self.isolated_paper.as_mut().expect("a group backing").wash = Some(rgba);
        self
    }

    fn measure(
        &self,
        theme: &ThemeVariant,
        foreground: gpui_rhai::Rgba8,
    ) -> Option<([f64; 3], f64)> {
        let backing = if let Some(rgb) = self.fixed_paper {
            rgb
        } else {
            backing(theme, &self.papers)?
        };
        if let Some(group) = self.isolated_paper {
            let group_paper = theme.tokens.color(group.token)?;
            let underlay = if let Some(token) = group.underlay {
                Some(theme.tokens.color(token)?)
            } else {
                None
            };
            if !gpui_rhai::EXTENDED_RENDERING {
                // Without the group compositor each constituent draw takes
                // the opacity independently and keeps its original hue.
                let backing =
                    underlay.map_or(backing, |paper| rgba_over(paper, self.opacity, backing));
                let mut background = rgba_over(group_paper, self.opacity, backing);
                if let Some(wash) = group.wash {
                    background = rgba_over(
                        gpui_rhai::Rgba8::from_rgba_hex(wash),
                        self.opacity,
                        background,
                    );
                }
                return Some((
                    background,
                    contrast_on_opacity(foreground, self.opacity, background),
                ));
            }
            let [r, g, b, a] = group_paper.as_rgba_hex().to_be_bytes();
            // The source paper can itself be translucent. Compose the
            // glyph into its premultiplied tile before fading the group
            // onto the host surface, not the other way around.
            let mut a = f64::from(a) / 255.0;
            let mut paper = [r, g, b].map(|v| f64::from(v) / 255.0 * a);
            if let Some(underlay) = underlay {
                let [r, g, b, alpha] = underlay.as_rgba_hex().to_be_bytes();
                let alpha = f64::from(alpha) / 255.;
                let behind = [r, g, b].map(|v| f64::from(v) / 255. * alpha);
                paper = std::array::from_fn(|i| paper[i] + behind[i] * (1. - a));
                a += alpha * (1. - a);
            }
            if let Some(wash) = group.wash {
                let [r, g, b, alpha] = wash.to_be_bytes();
                let alpha = f64::from(alpha) / 255.;
                paper = blend_rgb([r, g, b].map(|v| f64::from(v) / 255.), alpha, paper);
                a = alpha + a * (1. - alpha);
            }
            let [r, g, b, glyph_alpha] = foreground.as_rgba_hex().to_be_bytes();
            let glyph_alpha = f64::from(glyph_alpha) / 255.0;
            let ink = blend_rgb([r, g, b].map(|v| f64::from(v) / 255.0), glyph_alpha, paper);
            let ink_alpha = glyph_alpha + a * (1.0 - glyph_alpha);
            let filtered = |rgb: [f64; 3]| {
                let gray = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
                blend_rgb([gray; 3], group.grayscale, rgb)
            };
            let over = |source: [f64; 3], alpha: f64| {
                let source = filtered(source);
                std::array::from_fn(|i| {
                    source[i] * self.opacity + backing[i] * (1.0 - alpha * self.opacity)
                })
            };
            let background = over(paper, a);
            let painted = over(ink, ink_alpha);
            Some((background, contrast_between(painted, background)))
        } else {
            Some((
                backing,
                contrast_on_opacity(foreground, self.opacity, backing),
            ))
        }
    }
}

impl PresetRisks {
    pub(crate) fn new() -> Self {
        let engine = RuntimeEngine::new();
        let base = gpui_rhai::load_token_base(
            engine.engine(),
            "tokens.rhai",
            gpui_rhai_registry::TOKEN_BASE_SOURCE,
        )
        .unwrap();
        let stock = gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES
            .iter()
            .map(|&(name, source)| {
                gpui_rhai::load_theme_with_layers(
                    engine.engine(),
                    Some(&base),
                    name,
                    source,
                    &ThemeTokenOverrides::default(),
                )
                .unwrap()
            })
            .collect();
        Self {
            stock,
            recorded: Vec::new(),
        }
    }

    pub(crate) fn record(&mut self, theme: &ThemeVariant, finding: &AuditFinding) -> bool {
        let Some(stock) = self.stock.iter().find(|stock| {
            stock.family == theme.family
                && stock.name == theme.name
                && stock.mode == theme.mode
                && stock.tokens.colors == theme.tokens.colors
                && stock.tokens.component_styles == theme.tokens.component_styles
        }) else {
            return false;
        };
        if finding.rule != AuditRule::LowContrastText {
            return false;
        }
        let Some(evidence) = finding.contrast.as_ref() else {
            return false;
        };
        if evidence.ratio >= evidence.minimum {
            return false;
        }
        // Review actual fixture roles, not arbitrary text sharing their RGB.
        let parts = finding.path.split('/').collect::<Vec<_>>();
        if evidence.conservative {
            let Some(paper) = reviewed_context_map_hatch(stock, &parts, evidence) else {
                return false;
            };
            self.recorded.push(format!(
                "{} / {}: {}; original flat and hatched paper {:?}, {:?}; conservative bound retained",
                theme.family, theme.name, finding, paper.0, paper.1,
            ));
            return true;
        }
        let empty_example_combobox = finding
            .path
            .ends_with("/shared-combobox/trigger/shared-combobox-trigger-value");
        let empty_select = parts.contains(&"page-select")
            && [
                "select-empty-combobox-trigger-value",
                "select-error-combobox-trigger-value",
            ]
            .iter()
            .any(|key| parts.last() == Some(key));
        let outside_calendar = parts
            .iter()
            .position(|part| part.starts_with("week-"))
            .is_some_and(|index| {
                if !parts.contains(&"page-calendar") || index < 2 || index + 1 >= parts.len() {
                    return false;
                }
                let month = parts[index - 2];
                let date = parts[index + 1];
                iso_date(month) && iso_date(date) && month[..7] != date[..7]
            });
        let calendar_weekday = parts.contains(&"page-calendar")
            && parts
                .get(parts.len().saturating_sub(4)..)
                .is_some_and(|tail| {
                    tail.len() == 4
                        && iso_date(tail[0])
                        && ["0", "1"].contains(&tail[1])
                        && tail[2].parse::<usize>().is_ok_and(|day| day < 7)
                        && tail[3] == "0"
                });
        let density_segment = ["gallery-density-comfortable", "gallery-density-compact"]
            .iter()
            .any(|key| finding.path.ends_with(&format!("/{key}/0/0/0")));
        let pagination_ellipsis = parts.contains(&"page-pagination")
            && ["pagination-hosts", "pagination-wrapped"]
                .iter()
                .any(|key| parts.windows(4).any(|slice| slice == [*key, "1", "6", "0"]));
        let pagination_size_label = parts.contains(&"page-pagination")
            && ["pagination-hosts", "pagination-wrapped"]
                .iter()
                .any(|key| finding.path.ends_with(&format!("/{key}/0/0/0")));
        let textarea_counter =
            parts.contains(&"page-textarea") && finding.path.ends_with("/textarea.notes/counter/0");
        let field_affix = parts.contains(&"page-input_group")
            && ["/input_group-scroll/0/0/1/0", "/input_group-scroll/0/0/1/2"]
                .iter()
                .any(|tail| finding.path.ends_with(tail));
        let authored_table_header = [
            (
                "table",
                "table-hosts",
                &["host", "region", "cpu", "mem", "status"][..],
            ),
            ("table", "table-pods", &["pod", "ip", "region", "state"][..]),
            (
                "formatted_table",
                "formatted-native",
                &["name", "status", "duration"][..],
            ),
            (
                "data_view",
                "data-view-table",
                &["host", "region", "cpu"][..],
            ),
            (
                "scene.operations",
                "ops-hosts",
                &["host", "cpu", "status"][..],
            ),
            (
                "scene.data",
                "data-table",
                &["host", "region", "cpu", "mem"][..],
            ),
            ("foundations.affine", "affine-table", &["task", "state"][..]),
        ]
        .iter()
        .any(|(page, table, columns)| {
            parts.contains(&format!("page-{page}").as_str())
                && parts
                    .get(parts.len().saturating_sub(5)..)
                    .is_some_and(|tail| {
                        tail.len() == 5
                            && tail[0] == *table
                            && tail[1] == format!("{table}-header")
                            && columns.contains(&tail[2])
                            && tail[3] == "action"
                            && tail[4] == "0"
                    })
        });
        let authored_segment = parts
            .windows(2)
            .any(|pair| pair[0] == "tabs" && pair[1].starts_with("tab:"))
            && finding.path.ends_with("/content/label")
            && [
                "tabs",
                "tab_overflow_menu",
                "sliding_panel",
                "retained_tabs",
                "navigation_stack",
                "scene.settings",
                "effects",
            ]
            .iter()
            .any(|page| parts.contains(&format!("page-{page}").as_str()));
        let authored_toggle = (parts.contains(&"page-toggle_group")
            || parts.contains(&"page-app_shell"))
            && finding.path.ends_with("/0/0/0")
            && parts.iter().any(|part| {
                part.starts_with("tgroup-") || ["width-medium", "width-narrow"].contains(part)
            });
        let authored_badge = finding.path.ends_with("/label")
            && !parts.contains(&"label-entry")
            && [
                "badge",
                "foundations.color",
                "list",
                "table",
                "data_view",
                "scene.operations",
            ]
            .iter()
            .any(|page| parts.contains(&format!("page-{page}").as_str()));
        let mut pairs = Vec::new();
        if empty_example_combobox || empty_select {
            pairs.push(("field.placeholder", vec!["surface.field"]));
        } else if outside_calendar || calendar_weekday {
            pairs.push(("text.subtle", vec!["surface_raised"]));
        } else if density_segment || authored_segment || authored_toggle {
            pairs.push(("tabs.foreground", vec!["tabs.tray"]));
        } else if pagination_ellipsis {
            pairs.push(("pagination.faint", vec!["surface_raised"]));
        } else if pagination_size_label {
            pairs.push(("pagination.muted", vec!["surface_raised"]));
        } else if textarea_counter {
            pairs.push(("field.placeholder", vec!["surface_raised"]));
        } else if field_affix {
            pairs.push(("text.subtle", vec!["surface.field"]));
        } else if authored_table_header {
            pairs.push(("table.header_ink", vec!["table.surface"]));
        }
        if authored_badge {
            for (foreground, fill) in [
                ("badge.neutral_fg", "badge.neutral_bg"),
                ("badge.info_fg", "badge.info_bg"),
                ("badge.success_fg", "badge.success_bg"),
                ("badge.warning_fg", "badge.warning_bg"),
                ("badge.danger_fg", "badge.danger_bg"),
                ("badge.purple_fg", "badge.purple_bg"),
            ] {
                pairs.push((foreground, vec!["surface_raised", fill]));
                // A source-alpha badge retains its alpha on the collection's
                // actual selected surface; do not replace it with an opaque wash.
                pairs.push((foreground, vec!["surface_raised", "selection", fill]));
                pairs.push((foreground, vec!["surface_raised", "table.selection", fill]));
                pairs.push((foreground, vec!["table.surface", fill]));
                pairs.push((foreground, vec!["table.surface", "table.selection", fill]));
            }
        }
        let mut witnesses = pairs
            .iter()
            .map(|(ink, papers)| PaintWitness::new(ink, papers))
            .collect::<Vec<_>>();
        witnesses.extend(reviewed_detail(stock, &parts));
        let accepted = witnesses.iter().find_map(|witness| {
            if evidence.element_opacity != witness.opacity {
                return None;
            }
            let foreground = witness
                .fixed_ink
                .map(gpui_rhai::Rgba8::from_rgba_hex)
                .or_else(|| stock.tokens.color(witness.ink))?;
            let (background, ratio) = witness.measure(stock, foreground)?;
            let actual = evidence.background_srgb?;
            (foreground == evidence.foreground
                && actual
                    .into_iter()
                    .zip(background)
                    .all(|(a, b)| (a - b).abs() < 1e-12)
                && (evidence.ratio - ratio).abs() < 1e-9
                && [3.0, 4.5].contains(&evidence.minimum))
            .then_some((witness, background))
        });
        let Some((witness, background)) = accepted else {
            return false;
        };
        self.recorded.push(format!(
            "{} / {}: {}; {} #{:08x}, opacity {}, backing {:?}, isolated source {:?}, sRGB {background:?}, ratio {:.6}:1, required {:.2}:1",
            theme.family,
            theme.name,
            finding,
            witness.ink,
            evidence.foreground.as_rgba_hex(),
            witness.opacity,
            witness.papers,
            witness.isolated_paper,
            evidence.ratio,
            evidence.minimum
        ));
        true
    }

    pub(crate) fn assert_reported(&self) {
        assert!(
            !self.recorded.is_empty(),
            "the authored-risk positive control must still be reported by the strict audit"
        );
        self.report_if_present();
    }

    pub(crate) fn report_if_present(&self) {
        if !self.recorded.is_empty() {
            eprintln!(
                "Reviewed original-palette risks (not WCAG passes):\n{}",
                self.recorded.join("\n")
            );
        }
    }
}

/// Exact roles in the maintained Gallery specimens. This is not a role-wide
/// or page-wide exemption.
fn reviewed_detail(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    if let Some(plan) = reviewed_plan(stock, parts) {
        return Some(plan);
    }
    if let Some(cause) = reviewed_root_cause(stock, parts) {
        return Some(cause);
    }
    if let Some(chain) = reviewed_fallback_chain(stock, parts) {
        return Some(chain);
    }
    if let Some(stream) = reviewed_collaboration_stream(stock, parts) {
        return Some(stream);
    }
    if let Some(memory) = reviewed_memory_panel(stock, parts) {
        return Some(memory);
    }
    if let Some(bracket) = reviewed_judge_bracket(stock, parts) {
        return Some(bracket);
    }
    if let Some(card) = reviewed_project_card(stock, parts) {
        return Some(card);
    }
    if let Some(card) = reviewed_summary_or_tips_header(stock, parts) {
        return Some(card);
    }
    if let Some(profile) = reviewed_profile_menu(stock, parts) {
        return Some(profile);
    }
    if let Some(trigger) = reviewed_running_now_trigger(stock, parts) {
        return Some(trigger);
    }
    if let Some(budget) = reviewed_context_window(stock, parts) {
        return Some(budget);
    }
    if let Some(card) = reviewed_experiment_card(stock, parts) {
        return Some(card);
    }
    if let Some(plan) = reviewed_rate_planner(stock, parts) {
        return Some(plan);
    }
    if let Some(quota) = reviewed_usage_meter(stock, parts) {
        return Some(quota);
    }
    if let Some(breakdown) = reviewed_compact_breakdown(stock, parts) {
        return Some(breakdown);
    }
    if let Some(ink) = reviewed_receipt_or_chart_ink(stock, parts) {
        return Some(ink);
    }
    if let Some(tile) = reviewed_context_map_tile(stock, parts) {
        return Some(tile);
    }
    if let Some(label) = reviewed_context_map_outer_label(stock, parts) {
        return Some(label);
    }
    if let Some(review) = reviewed_agent_cursors_header_or_idle_tab(stock, parts) {
        return Some(review);
    }
    if let Some(review) = reviewed_parallel_attempts(stock, parts) {
        return Some(review);
    }
    if let Some(review) = reviewed_fan_out_grid(stock, parts) {
        return Some(review);
    }
    if let Some(summary) = reviewed_away_summary_caption(stock, parts) {
        return Some(summary);
    }
    if let Some(mail) = reviewed_email_draft_ink(stock, parts) {
        return Some(mail);
    }
    if let Some(diff) = reviewed_diff_timeline(stock, parts) {
        return Some(diff);
    }
    if let Some(schedule) = reviewed_scheduled_runs(stock, parts) {
        return Some(schedule);
    }
    if let Some(access) = reviewed_tool_access_map(stock, parts) {
        return Some(access);
    }
    if let Some(billing) = reviewed_billing_schedule(stock, parts) {
        return Some(billing);
    }
    if let Some(directory) = reviewed_radar_directory(parts) {
        return Some(directory);
    }
    if let Some(trace) = reviewed_trace_waterfall(stock, parts) {
        return Some(trace);
    }
    if let Some(ledger) = reviewed_ledger(stock, parts) {
        return Some(ledger);
    }
    if let Some(findings) = reviewed_findings(parts) {
        return Some(findings);
    }
    if let Some(recap) = reviewed_recap(stock, parts) {
        return Some(recap);
    }
    if stock.name == "Light" {
        let modern = stock.family == "Modern";
        if parts.contains(&"page-agent_steering")
            && parts.contains(&"steering-demo")
            && parts.ends_with(&[
                "step:read",
                "notes-layout",
                "notes",
                "note:accepted",
                "copy",
                "tag",
                "1",
            ])
        {
            // The accepted note has a different paper from the compose rail.
            // Recognize only the exact stock badge, not arbitrary status text.
            let (ink, paper) = if modern {
                (0x4fb0feff, 0xf5f5f5)
            } else {
                (0x0060e2c3, 0xf7f7f7)
            };
            return Some(PaintWitness::on_literal("", paper).with_literal_ink(ink));
        }
        if modern
            && parts.contains(&"page-agent_steering")
            && parts.contains(&"steering-demo")
            && ["hint:↵", "hint:⌘↵", "hint:Ctrl+↵"]
                .iter()
                .any(|key| parts.ends_with(&["surface", "compose", "hints", key, "words"]))
        {
            return Some(PaintWitness::new("text_muted", &["cite.tray"]));
        }
        if modern && parts.contains(&"page-agent_monitor") && parts.contains(&"monitor-demo") {
            if parts.ends_with(&["run", "footer", "count"]) {
                return Some(PaintWitness::new("text_muted", &["cite.tray"]));
            }
            if parts.ends_with(&["run", "footer", "log", "label-entry", "label"]) {
                return Some(PaintWitness::new("button.ghost_fg", &["cite.tray"]));
            }
        }
        if parts.contains(&"page-agent_fleet") && parts.contains(&"fleet-demo") {
            if modern
                && ["count", "running", "waiting", "failed"]
                    .iter()
                    .any(|key| parts.ends_with(&["fleet", "bar", "stats", key]))
            {
                return Some(PaintWitness::new("text_muted", &["cite.tray"]));
            }
            if parts.ends_with(&[
                "sub:visual",
                "配色",
                "details",
                "trigger",
                "head",
                "status",
                "status-text",
            ]) {
                return Some(PaintWitness::new("tag.warning_fg", &["tag.warning_bg"]));
            }
            if parts.ends_with(&["lead:research", "head", "status", "status-text"]) {
                return Some(PaintWitness::new("tag.success_fg", &["tag.success_bg"]));
            }
        }
        if modern
            && parts.contains(&"page-agent_graph")
            && parts.contains(&"graph-demo")
            && (parts.ends_with(&["graph", "bar", "stats", "count"])
                || parts.ends_with(&["graph", "bar", "stats", "rate"]))
        {
            return Some(PaintWitness::new("text_muted", &["cite.tray"]));
        }
        if parts.contains(&"page-agent_tree") && parts.contains(&"tree-demo") {
            if modern && parts.ends_with(&["tree", "head", "stats", "count"]) {
                return Some(PaintWitness::new("text_muted", &["cite.tray"]));
            }
            if parts.ends_with(&["waiting", "trigger", "label-entry", "label"])
                || parts.ends_with(&["card:build", "metadata", "status", "status-text"])
            {
                return Some(PaintWitness::new("tag.warning_fg", &["tag.warning_bg"]));
            }
            if parts.ends_with(&["card:research", "metadata", "status", "status-text"]) {
                return Some(PaintWitness::new("tag.success_fg", &["tag.success_bg"]));
            }
        }
        if modern
            && parts.contains(&"page-agent_trail")
            && parts.contains(&"trail-demo")
            && parts.ends_with(&["trail", "head", "count"])
        {
            return Some(PaintWitness::new("text_muted", &["cite.tray"]));
        }
        if modern
            && parts.contains(&"page-agent_checkpoints")
            && parts.contains(&"history-demo")
            && (parts.ends_with(&["history", "header", "task"])
                || parts.ends_with(&["history", "header", "summary"]))
        {
            return Some(PaintWitness::new("text_muted", &["cite.tray"]));
        }
        if modern && parts.contains(&"page-agent_plan") && parts.contains(&"plan-demo") {
            if parts.ends_with(&["plan", "head", "estimate"])
                || parts.ends_with(&["plan", "footer", "progress"])
            {
                return Some(PaintWitness::new("text_muted", &["cite.tray"]));
            }
            if parts.ends_with(&["plan", "head", "status"]) {
                return Some(PaintWitness::new("text_muted", &["surface_hover"]));
            }
            if parts.ends_with(&["plan", "footer", "buttons", "1", "label-entry", "label"]) {
                return Some(PaintWitness::new("button.ghost_fg", &["cite.tray"]));
            }
        }
        if parts.contains(&"page-agent_inbox") && parts.contains(&"inbox-demo") {
            if modern
                && (parts.ends_with(&["head", "name", "crumb"])
                    || parts.ends_with(&["inbox", "summary"]))
            {
                return Some(PaintWitness::new("text_muted", &["cite.tray"]));
            }
            if modern && parts.ends_with(&["head", "controls", "0", "label-entry", "label"]) {
                return Some(PaintWitness::new("button.ghost_fg", &["cite.tray"]));
            }
            if modern && parts.ends_with(&["head", "controls", "views", "list", "0"]) {
                return Some(PaintWitness::new("text_muted", &["surface_hover"]));
            }
            if stock.family == "Clean"
                && parts.ends_with(&["card-gate", "plan", "main", "foot", "state"])
            {
                let mut witness =
                    PaintWitness::new("text.warning", &["surface_raised", "text.warning"]);
                witness.papers[1].opacity = 23.0 / 255.0;
                return Some(witness);
            }
        }
        if modern && parts.contains(&"page-agent_handoff") && parts.contains(&"relay-demo") {
            if parts.ends_with(&["relay", "head", "summary"]) {
                return Some(PaintWitness::new("text_muted", &["cite.tray"]));
            }
            if [
                &["grid", "pair-0", "header", "heading"][..],
                &["grid", "pair-0", "header", "count"],
                &["grid", "pair-1", "header", "heading"],
                &["grid", "pair-1", "header", "count"],
            ]
            .iter()
            .any(|tail| parts.ends_with(tail))
            {
                return Some(PaintWitness::new("text_muted", &["surface_hover"]));
            }
        }
        if parts.contains(&"page-agent_lanes") && parts.contains(&"lanes-demo") {
            if modern && parts.ends_with(&["timeline", "head", "summary"]) {
                return Some(PaintWitness::new("text_muted", &["cite.tray"]));
            }
            if stock.family == "Clean"
                && (parts.ends_with(&["agent-verify", "track", "task-verify", "title"])
                    || parts.ends_with(&["agent-verify", "track", "task-verify", "tool-0", "mark"]))
            {
                let mut witness =
                    PaintWitness::new("text.warning", &["surface_raised", "text.warning"]);
                witness.papers[1].opacity = 26.0 / 255.0;
                return Some(witness);
            }
        }
        if modern && parts.contains(&"page-agent_queue") && parts.contains(&"queue-demo") {
            if parts.ends_with(&["head", "heading", "name"])
                || parts.ends_with(&["head", "controls", "concurrency"])
            {
                return Some(PaintWitness::new("text_muted", &["cite.tray"]));
            }
            if parts.ends_with(&["head", "controls", "1", "label-entry", "label"])
                || parts.ends_with(&["head", "controls", "3", "label-entry", "label"])
            {
                return Some(PaintWitness::new("button.ghost_fg", &["cite.tray"]));
            }
            if [
                &["waiting", "section", "count"][..],
                &["waiting-items", "job-compare", "mark", "0"],
                &["worker-worker", "1", "identity", "caption"],
                &["worker-worker", "1", "job-write", "notes", "mark", "0"],
                &["worker-worker", "2", "identity", "caption"],
                &["worker-worker", "2", "idle"],
                &["bins", "done", "section", "count"],
            ]
            .iter()
            .any(|tail| parts.ends_with(tail))
            {
                return Some(PaintWitness::new("text_muted", &["surface_hover"]));
            }
            if parts.ends_with(&["bins", "failed", "section", "count"]) {
                let mut witness =
                    PaintWitness::new("text_muted", &["surface_raised", "text.danger"]);
                witness.papers[1].opacity = 14.0 / 255.0;
                return Some(witness);
            }
        }
        if modern && parts.contains(&"page-delegate_board") && parts.contains(&"delegate-demo") {
            if parts.ends_with(&["board", "head", "hint"])
                || parts.ends_with(&["board", "footer", "summary", "cost"])
                || parts.ends_with(&["board", "footer", "summary", "queued"])
            {
                return Some(PaintWitness::new("text_muted", &["cite.tray"]));
            }
            if parts.ends_with(&["pile-content", "section-label", "count"])
                || parts.ends_with(&["agent-build-content", "identity", "nameplate", "model"])
                || parts.ends_with(&[
                    "agent-plan-content",
                    "queue",
                    "move-constraints",
                    "move-constraints-content",
                    "time",
                ])
            {
                return Some(PaintWitness::new("text_muted", &["surface_hover"]));
            }
        }
        if modern
            && parts.contains(&"page-review_center")
            && parts.contains(&"review-demo")
            && [
                &["head", "progress", "count"][..],
                &["item-letter", "1", "label", "meta"],
                &["item-letter", "1", "label", "name", "age"],
                &["item-backup", "1", "label", "meta"],
                &["item-backup", "1", "label", "name", "age"],
            ]
            .iter()
            .any(|tail| parts.ends_with(tail))
        {
            return Some(PaintWitness::new("text_muted", &["cite.tray"]));
        }
        if parts.contains(&"page-decision_tree") && parts.contains(&"tree-demo") {
            if modern
                && [
                    &["choice-refund", "button", "label-row", "number", "0"][..],
                    &["choice-contact", "button", "label-row", "number", "0"],
                    &["choice-wait", "button", "label-row", "number", "0"],
                    &["custom-choice", "button", "label-row", "number", "0"],
                    &["choice-refund", "leaf", "effects", "effect-0", "span-0"],
                    &["choice-refund", "leaf", "effects", "effect-1"],
                    &["choice-contact", "leaf", "effects", "effect-0"],
                    &["choice-wait", "leaf", "effects", "effect-0"],
                ]
                .iter()
                .any(|tail| parts.ends_with(tail))
            {
                return Some(PaintWitness::new("text_muted", &["surface_hover"]));
            }
            if stock.family == "Clean"
                && parts.ends_with(&[
                    "choice-refund",
                    "leaf",
                    "effects",
                    "effect-0",
                    "span-1",
                    "trigger",
                    "span-text",
                ])
            {
                return Some(PaintWitness::new(
                    "text.warning",
                    &["surface_raised", "surface_hover", "surface_hover"],
                ));
            }
        }
        if modern && parts.contains(&"page-approval_gates") && parts.contains(&"gates-demo") {
            if parts.ends_with(&["summary", "bar", "2"])
                || parts.ends_with(&["holds", "1"])
                || parts.ends_with(&["holds", "2"])
            {
                return Some(PaintWitness::new("text_muted", &["cite.tray"]));
            }
            if parts.ends_with(&[
                "stack",
                "summary",
                "actions",
                "edit",
                "label-entry",
                "label",
            ]) {
                return Some(PaintWitness::new("button.ghost_fg", &["cite.tray"]));
            }
            if parts.ends_with(&["stack", "summary", "body", "head", "2"]) {
                return Some(PaintWitness::new(
                    "text_muted",
                    &["surface_raised", "surface_hover"],
                ));
            }
        }
        if parts.contains(&"page-autonomy_dial") && parts.contains(&"dial-demo") {
            if parts.ends_with(&["action:notes", "1", "0", "risk", "1"]) {
                return Some(PaintWitness::new(
                    "badge.warning_fg",
                    &["surface_raised", "badge.warning_bg"],
                ));
            }
            if parts.ends_with(&["action:archive", "1", "0", "risk", "1"]) {
                return Some(PaintWitness::new(
                    "badge.danger_fg",
                    &["surface_raised", "badge.danger_bg"],
                ));
            }
            if modern
                && (parts.ends_with(&["card", "1", "always", "0"])
                    || parts.ends_with(&["card", "1", "auto", "0"]))
            {
                return Some(PaintWitness::new(
                    "text_muted",
                    &["surface_raised", "selection"],
                ));
            }
            if modern
                && (parts.ends_with(&["groups", "own", "0"])
                    || parts.ends_with(&["groups", "ask", "0"]))
            {
                return Some(PaintWitness::new("text_muted", &["cite.tray"]));
            }
        }
    }
    if stock.name == "Light" && parts.contains(&"decision-demo") {
        let modern = stock.family == "Modern";
        if parts.contains(&"page-decision_card") {
            if parts.ends_with(&["deck", "head", "0"]) {
                return Some(PaintWitness::new(
                    "badge.warning_fg",
                    &[
                        if modern {
                            "cite.tray"
                        } else {
                            "surface_raised"
                        },
                        "badge.warning_bg",
                    ],
                ));
            }
            if modern {
                if parts.ends_with(&["deck", "head", "1"])
                    || parts.ends_with(&["head", "pager", "1"])
                {
                    return Some(PaintWitness::new("text_muted", &["cite.tray"]));
                }
                if parts.ends_with(&["head", "3", "label-entry", "label"]) {
                    return Some(PaintWitness::new("button.ghost_fg", &["cite.tray"]));
                }
                if parts.ends_with(&["body", "own", "0", "0"])
                    || ["review", "accept", "wait"]
                        .iter()
                        .any(|id| parts.ends_with(&["options", id, "0", "0", "0"]))
                    || parts.ends_with(&["options", "review", "0", "2"])
                {
                    return Some(PaintWitness::new(
                        "text_muted",
                        &["surface_raised", "surface_hover"],
                    ));
                }
            }
        }
        if parts.contains(&"page-decision_inbox")
            && ["layout", "note"]
                .iter()
                .any(|id| parts.ends_with(&["decisions", "item", "decision:round", "1", id, "0"]))
        {
            return Some(PaintWitness::new("text.subtle", &["surface_raised"]));
        }
    }
    if stock.name == "Light" {
        let approval = parts.contains(&"page-approval_card") && parts.contains(&"review-demo");
        let permission =
            parts.contains(&"page-permission_request") && parts.contains(&"permission-demo");
        if (approval || permission) && parts.ends_with(&["surface", "header", "risk", "1"]) {
            return Some(PaintWitness::new(
                "badge.warning_fg",
                &["surface_raised", "badge.warning_bg"],
            ));
        }
        if stock.family == "Modern" && approval {
            if parts.ends_with(&["review", "label"]) || parts.ends_with(&["footer", "timer", "1"]) {
                return Some(PaintWitness::new("text_muted", &["cite.tray"]));
            }
            if parts.ends_with(&["footer", "buttons", "0", "label-entry", "label"]) {
                return Some(PaintWitness::new("button.ghost_fg", &["cite.tray"]));
            }
            if ["detail-0", "detail-1", "detail-2"]
                .iter()
                .any(|part| parts.ends_with(&["surface", part, "0"]))
            {
                return Some(PaintWitness::new(
                    "text_muted",
                    &["surface_raised", "surface_hover"],
                ));
            }
        }
        if stock.family == "Modern"
            && permission
            && ["timed", "always"]
                .iter()
                .any(|part| parts.ends_with(&["access", "answer", "0", part, "0"]))
        {
            return Some(PaintWitness::new("text_muted", &["cite.tray", "selection"]));
        }
    }
    if stock.name == "Light" && parts.contains(&"review") {
        if stock.family == "Modern"
            && parts.contains(&"page-comment_thread")
            && parts.ends_with(&["thread", "header", "0", "count"])
        {
            return Some(PaintWitness::new("text_muted", &["cite.tray"]));
        }
        if stock.family == "Clean" {
            let thread_avatar = parts.contains(&"page-comment_thread")
                && parts.ends_with(&[
                    "transcript",
                    "item",
                    "entry:review",
                    "7",
                    "meta",
                    "avatar",
                    "0",
                ]);
            let panel_avatar = parts.contains(&"page-comments_panel")
                && (parts.ends_with(&["panel", "composer", "avatar", "0"])
                    || ["entry:status", "entry:unapproved"]
                        .iter()
                        .any(|key| parts.ends_with(&[key, "meta", "avatar", "0"]))
                    || ["entry:review", "entry:edit"]
                        .iter()
                        .any(|key| parts.ends_with(&[key, "7", "meta", "avatar", "0"])));
            if thread_avatar || panel_avatar {
                return Some(PaintWitness::new(
                    "badge.info_fg",
                    &["surface_raised", "badge.info_bg"],
                ));
            }
        }
    }
    if stock.family == "Modern"
        && stock.name == "Light"
        && parts.contains(&"page-integration_settings")
        && parts.contains(&"store-settings")
    {
        if parts.ends_with(&["settings", "header", "1", "meta"]) {
            return Some(PaintWitness::new("text_muted", &["cite.tray"]));
        }
        if parts.ends_with(&["credential", "generate-new", "label-entry", "label"]) {
            return Some(PaintWitness::new("text_muted", &["surface_hover"]));
        }
    }
    if parts.contains(&"page-integration_picker") && parts.contains(&"connections") {
        let tray = if stock.family == "Modern" {
            "cite.tray"
        } else {
            "surface_raised"
        };
        if parts.ends_with(&["connections", "0", "0", "label-entry", "label"]) {
            return Some(PaintWitness::new("button.ghost_fg", &[tray]));
        }
        if parts.ends_with(&["connections", "0", "2"]) {
            return Some(PaintWitness::new("text.subtle", &[tray]));
        }
        if ["group-0", "group-1"]
            .iter()
            .any(|group| parts.ends_with(&["catalog", group]))
        {
            return Some(PaintWitness::new("text.subtle", &["surface_raised"]));
        }
    }
    if parts.contains(&"page-prompt_library") && parts.contains(&"library") {
        let tray = if stock.family == "Modern" {
            "cite.tray"
        } else {
            "surface_raised"
        };
        if parts.ends_with(&["library", "body", "0", "0", "0", "1"]) {
            return Some(PaintWitness::new("text.subtle", &["surface_raised"]));
        }
        if parts.ends_with(&["library", "foot", "status"]) {
            return Some(PaintWitness::new("text_muted", &[tray]));
        }
        if parts.ends_with(&["library", "foot", "1", "0", "label-entry", "label"]) {
            return Some(PaintWitness::new("button.ghost_fg", &[tray]));
        }
        if parts.ends_with(&["list", "review", "1", "label"]) {
            return Some(PaintWitness::new(
                "badge.success_fg",
                &["surface_raised", "badge.success_bg"],
            ));
        }
        if parts.ends_with(&["list", "review", "7", "1", "label"]) {
            return Some(PaintWitness::new(
                "badge.info_fg",
                &["surface_raised", "selection", "badge.info_bg"],
            ));
        }
        if parts.ends_with(&["body", "1", "review", "7", "0", "1", "label"]) {
            return Some(PaintWitness::new(
                "badge.info_fg",
                &["surface_raised", "badge.info_bg"],
            ));
        }
    }
    if parts.contains(&"page-run_controls")
        && parts.contains(&"demo-run")
        && parts.ends_with(&["tools", "policy", "0", "mode-chat", "0", "0", "0"])
    {
        return Some(PaintWitness::new("tabs.foreground", &["tabs.tray"]));
    }
    if parts.contains(&"page-thinking_state")
        && parts.ends_with(&["release-work", "header", "elapsed"])
    {
        return Some(PaintWitness::new("text.subtle", &["surface_raised"]));
    }
    let cites_in_gallery = (parts.contains(&"page-cited_answer")
        && parts.contains(&"rollout-evidence"))
        || (parts.contains(&"page-ai_answer")
            && parts.contains(&"rollout-answer")
            && parts.contains(&"reply-body"));
    if cites_in_gallery
        && (parts.ends_with(&["sources", "chips", "reports", "trace", "number"])
            || parts.ends_with(&["sources", "chips", "rollback", "number"]))
    {
        return Some(PaintWitness::new(
            "text.subtle",
            &[if stock.family == "Modern" {
                "cite.well"
            } else {
                "surface_raised"
            }],
        ));
    }
    if parts.contains(&"page-cited_answer") {
        if parts.ends_with(&["rollout-evidence", "card", "header", "summary"]) {
            return Some(PaintWitness::new("text.subtle", &["surface_raised"]));
        }
        if parts.ends_with(&["rollout-evidence", "tray", "caption"]) && stock.family == "Modern" {
            return Some(PaintWitness::new("text_muted", &["cite.tray"]));
        }
    }
    if parts.contains(&"page-message_actions")
        && parts.ends_with(&[
            "response-actions",
            "actions",
            "versions",
            "version-count",
            "1",
        ])
        && stock.family == "Clean"
    {
        return Some(PaintWitness::new("text.subtle", &["surface_raised"]));
    }
    if parts.contains(&"page-reaction_bar")
        && parts.ends_with(&["release-reactions", "repost", "count", "digits"])
    {
        return Some(if stock.family == "Clean" {
            PaintWitness::new(
                "reaction.selected",
                &["surface_raised", "reaction.selected_fill"],
            )
        } else {
            PaintWitness::new("reaction.repost", &["surface_raised"])
        });
    }
    if parts.contains(&"page-model_picker")
        && parts.ends_with(&["composer-settings", "trigger", "model-icon", "0"])
        && stock.family == "Modern"
    {
        return Some(PaintWitness::new(
            "text_muted",
            &["surface_raised", "surface_hover"],
        ));
    }
    if parts.contains(&"page-progress_tracker") && parts.contains(&"progress_tracker-scroll") {
        let at = parts
            .iter()
            .position(|part| ["release", "playback"].contains(part))?;
        return match &parts[at + 1..] {
            ["header", "count", "1"] | ["list", "spec", "label", "caption"] => {
                Some(PaintWitness::new("checklist.muted", &["surface.field"]))
            }
            ["header", "count", "number"] if stock.family == "Modern" => {
                Some(PaintWitness::new("checklist.muted", &["surface.field"]))
            }
            _ => None,
        };
    }
    if parts.contains(&"page-setup_guide") && parts.contains(&"setup_guide-scroll") {
        let at = parts.iter().position(|part| *part == "onboarding")?;
        let tail = &parts[at + 1..];
        return match tail {
            ["header", "actions", "0", "label-entry", "label"] if stock.family == "Modern" => Some(
                PaintWitness::new("checklist.setup_fill", &["surface.field"]),
            ),
            ["progress", "count", "1"] if stock.family == "Clean" => {
                Some(PaintWitness::new("checklist.muted", &["surface.field"]))
            }
            [
                "setup-groups-content",
                "setup-groups-measure",
                "groups",
                "workspace",
                "setup-tasks-0-content",
                "setup-tasks-0-measure",
                "tasks",
                "create" | "invite",
                "label",
                "caption",
            ] => Some(PaintWitness::new(
                "checklist.muted",
                &["surface.field", "checklist.open"],
            )),
            _ => None,
        };
    }
    if parts.contains(&"page-stepper_form") && parts.contains(&"workspace-setup") {
        let detail = parts.get(parts.iter().position(|part| *part == "workspace-setup")? + 1..)?;
        return match detail {
            ["header", "steps", "account", "control", "mark", "digit"]
                if stock.family == "Clean" =>
            {
                Some(PaintWitness::new("stepper.progress", &["stepper.tray"]))
            }
            [
                "header",
                "steps",
                "team" | "plan" | "review",
                "control",
                "mark",
                "digit",
            ] => Some(PaintWitness::new("field.placeholder", &["stepper.tray"])),
            [
                "header",
                "steps",
                "team" | "plan" | "review",
                "control",
                "name",
            ] => Some(PaintWitness::new("text.subtle", &["stepper.tray"])),
            _ => None,
        };
    }
    if parts.contains(&"page-filter_menu") && parts.contains(&"deployment-filters") {
        if parts.ends_with(&["header", "clear", "0"]) && stock.family == "Modern" {
            return Some(PaintWitness::new("text.subtle", &["filter_menu.card"]));
        }
        let detail = parts.get(parts.len().checked_sub(4)?..)?;
        return match detail {
            ["card", "hints", "up" | "enter", "legend"] => {
                Some(PaintWitness::new("text.subtle", &["filter_menu.card"]))
            }
            ["groups", "status" | "region" | "access", "header", "total"] => {
                let ink = if stock.family == "Clean" {
                    "field.placeholder"
                } else {
                    "text.subtle"
                };
                Some(PaintWitness::new(ink, &["filter_menu.card"]))
            }
            [
                "groups",
                "status" | "region" | "access",
                "header",
                "selected-count",
            ] => Some(PaintWitness::new("text.subtle", &["filter_menu.card"])),
            _ => None,
        };
    }
    if parts.contains(&"page-tag_input") && parts.contains(&"tag_input-scroll") {
        if parts
            .get(parts.len().checked_sub(8)?..)
            .is_some_and(|tail| {
                matches!(
                    tail,
                    [
                        "recipients",
                        "field",
                        "aya@example.org" | "lin@example.org",
                        "0",
                        "value",
                        "prefix",
                        "0",
                        "0"
                    ]
                )
            })
        {
            return Some(PaintWitness::new(
                "text_muted",
                &["surface.field", "tag_input.chip", "tag_input.avatar"],
            ));
        }
        let detail = parts.get(parts.len().checked_sub(3)?..)?;
        return match detail {
            ["project-labels", "header", "optional-marker"]
            | ["project-labels" | "recipients", "footer", "hint"] => {
                Some(PaintWitness::new("text.subtle", &["surface_raised"]))
            }
            ["project-labels" | "recipients", "footer", "count"] => {
                Some(PaintWitness::new("field.placeholder", &["surface_raised"]))
            }
            _ => None,
        };
    }
    if parts.contains(&"page-otp_input")
        && parts.contains(&"otp_input-scroll")
        && parts.ends_with(&["verification", "resend-row", "resend-hint"])
    {
        return Some(PaintWitness::new("text.subtle", &["surface_raised"]));
    }
    if parts.contains(&"page-number_input") && parts.contains(&"number_input-scroll") {
        let ends_with = |tail| parts.ends_with(tail);
        let paper = if ends_with(&["budget", "field", "0", "unit"])
            || ends_with(&["quota", "field", "0", "unit"])
        {
            Some("surface.field")
        } else if ends_with(&["read-only", "field", "0", "prefix", "0"]) {
            return Some(PaintWitness::new(
                "text.subtle",
                &["surface_raised", "number.read_only"],
            ));
        } else if ends_with(&["budget", "hint"])
            || ends_with(&["quota", "label-row", "optional-marker"])
        {
            Some("surface_raised")
        } else {
            None
        };
        return paper.map(|paper| PaintWitness::new("text.subtle", &[paper]));
    }
    if parts.contains(&"page-switch")
        && parts
            .get(parts.len().checked_sub(7)?..)
            .is_some_and(|tail| {
                matches!(
                    tail,
                    [
                        "switch-scroll",
                        "0",
                        "0",
                        "1",
                        "0" | "1",
                        "text",
                        "description"
                    ]
                )
            })
    {
        return Some(PaintWitness::new("choice.description", &["surface_raised"]));
    }
    if !parts.contains(&"page-command") || !parts.contains(&"command-scroll") {
        return None;
    }
    let at = parts
        .iter()
        .position(|part| ["command-embedded", "command-host"].contains(part))?;
    let host = parts[at] == "command-host";
    let tail = &parts[at + 1..];
    let paper = if host {
        "surface_raised"
    } else {
        "command.surface"
    };
    if tail == ["card", "search", "enter-hint", "1"] {
        return Some(PaintWitness::new("command.placeholder", &[paper]));
    }
    // The embedded Host example has one item and a caller-owned footer.
    // Neither that item's label nor the Host's footer may use this policy.
    if host {
        return None;
    }
    let row = ["card", "list", "viewport", "command-embedded-list", "item"];
    if let Some(tail) = tail.strip_prefix(row.as_slice()) {
        return match tail {
            ["group:Release" | "group:Observe", "0"] => {
                Some(PaintWitness::new("command.group", &[paper]))
            }
            [
                item @ ("item:deploy" | "item:rollback" | "item:logs"),
                "text",
                "hint",
            ] => {
                let papers = if *item == "item:deploy" {
                    vec![paper, "command.selection"]
                } else {
                    vec![paper]
                };
                Some(PaintWitness::new("command.hint", &papers))
            }
            [
                item @ ("item:deploy" | "item:logs"),
                "shortcut",
                "0" | "1",
                "0",
            ] => {
                let active = *item == "item:deploy";
                let opacity = if !active && stock.family == "Modern" {
                    0.85
                } else {
                    1.0
                };
                let papers = if active {
                    vec![paper, "command.selection"]
                } else {
                    vec![paper]
                };
                let mut witness = PaintWitness::new(
                    if active {
                        "command.shortcut_active"
                    } else {
                        "command.shortcut"
                    },
                    &papers,
                );
                // Native opacity multiplies the keycap's own fill as well as its
                // text. Keep that unquantized factor in the independent witness.
                witness.papers.push(Paper {
                    token: "kbd.menu_surface",
                    opacity,
                });
                witness.opacity = opacity;
                Some(witness)
            }
            _ => None,
        };
    }
    let footer_paper = if stock.family == "Modern" {
        "command.tray"
    } else {
        paper
    };
    let is_cap = matches!(
        tail,
        ["footer", "0", "0", "0", "keys", "key-1" | "key-2"]
            | ["footer", "0", "run-hint", "0", "keys", "key-1"]
    );
    let is_label = matches!(
        tail,
        ["footer", "0", "0", "1"] | ["footer", "0", "run-hint", "1"]
    );
    if is_cap {
        Some(PaintWitness::new(
            "command.footer",
            &[footer_paper, "kbd.menu_surface"],
        ))
    } else if is_label {
        Some(PaintWitness::new("command.footer", &[footer_paper]))
    } else {
        None
    }
}

fn reviewed_away_summary_caption(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts.iter().position(|part| *part == "page-away_summary")?;
    let [
        "1",
        "away_summary-scroll",
        "0",
        "0",
        "away-demo",
        "width",
        "summary",
        "body",
        tail @ ..,
    ] = &parts[page + 1..]
    else {
        return None;
    };
    let header = match tail {
        ["header", "0", "1"] | ["header", "counts", "needs_you" | "failed" | "done", "1"] => true,
        [
            "items",
            "item:import" | "item:metadata" | "item:pipeline",
            "body",
            "detail",
        ] => false,
        _ => return None,
    };
    let (ink, header_paper, row_paper) = match (stock.family.as_str(), stock.name.as_str()) {
        ("Modern", "Light") => (0x8a8a8aff, 0xf0f0f0, 0xffffff),
        ("Modern", "Dark") => (0x7c7c82ff, 0x19191c, 0x232326),
        ("Clean", "Light") => (0x00000073, 0xffffff, 0xffffff),
        ("Clean", "Dark") => (0xffffff73, 0x1c1c1f, 0x1c1c1f),
        _ => return None,
    };
    // Use measured recipe colors, not another component's coincidentally
    // equal role. An action, a dirty header or a differently backed row is
    // not one of these fixed Gallery specimens.
    Some(
        PaintWitness::on_literal(
            "AwaySummary quiet caption",
            if header { header_paper } else { row_paper },
        )
        .with_literal_ink(ink),
    )
}

fn reviewed_email_draft_ink(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts.iter().position(|part| *part == "page-email_draft")?;
    let [
        "1",
        "email_draft-scroll",
        "0",
        "0",
        "1",
        "sized",
        "email",
        tail @ ..,
    ] = &parts[page + 1..]
    else {
        return None;
    };
    let role = match tail {
        ["record", "0", "1"] => "record status",
        ["record", "1"] => "account",
        [
            "switcher",
            "draft-grid",
            "meta",
            "to",
            "recipient:renewal:0",
            "trigger",
            "1",
        ] => "address",
        ["switcher", "draft-grid", "meta", "file", "1"] => "attachment",
        ["switcher", "draft-grid", "shortcut", "0"] => "shortcut",
        ["switcher", "draft-grid", "footer-words", "1"] => "badge",
        [
            "switcher",
            "draft-grid",
            "actions",
            "tones",
            "tone:short" | "tone:formal",
            "0",
        ] => "idle tone",
        _ => return None,
    };
    let (ink, paper, wash) = match (stock.family.as_str(), stock.name.as_str(), role) {
        ("Modern", "Light", "record status") => (0xb25e00ff, 0xfdeedd, None),
        ("Modern", "Light", "account" | "shortcut") => (0x8f8f8fff, 0xf0f0f0, None),
        ("Modern", "Light", "address") => (0x8f8f8fff, 0xf5f5f5, None),
        ("Modern", "Light", "attachment" | "idle tone") => (0x8f8f8fff, 0xffffff, None),
        ("Modern", "Light", "badge") => (0x1e8a4fff, 0xe6f5ec, None),
        ("Modern", "Dark", "address") => (0x8f8f8fff, 0x2a2a2d, None),
        ("Clean", "Light", "record status") => (0xb48906ff, 0xffffff, Some(0xe8b0001f)),
        ("Clean", "Light", "badge") => (0x1e8a4fff, 0xffffff, Some(0x5fc88a14)),
        ("Clean", "Light", "address") => (0x00000073, 0xffffff, None),
        ("Clean", "Light", "shortcut") => (0x00000045, 0xffffff, None),
        ("Clean", "Dark", "shortcut") => (0xffffff4d, 0x1c1c1e, None),
        _ => return None,
    };
    let witness = PaintWitness::on_literal(role, paper).with_literal_ink(ink);
    Some(match wash {
        Some(wash) => witness.with_literal_layer(wash),
        None => witness,
    })
}

fn reviewed_plan(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts
        .iter()
        .position(|part| *part == "page-plan_vs_actual")?;
    let specimen = parts[page + 1..]
        .windows(2)
        .position(|pair| pair == ["sizing", "plan-vs-actual"])?;
    let slot = &parts[page + specimen + 3..];
    let paper = ["plan_vs_actual.card"];
    match slot {
        ["head", "title", "task"]
        | ["head", "totals", "0" | "1", "0" | "2"]
        | ["stage", "axis", "0", "0"]
        | [
            "stage",
            "guides",
            "plot",
            "tick-label:30000" | "tick-label:60000" | "tick-label:90000" | "tick-label:120000"
            | "tick-label:150000",
            "1",
        ]
        | [
            "foot-and-inspection",
            "legend",
            "0",
            "0",
            "0" | "1" | "2",
            "1",
        ] => Some(PaintWitness::new("plan_vs_actual.muted", &paper)),
        // These two initial fixture rows are waiting. An otherwise identical
        // path with different ink, opacity or backing still fails the witness.
        [
            "stage",
            "step:review" | "step:publish",
            "1",
            "readout",
            "readout",
            "0",
        ] => Some(PaintWitness::new("plan_vs_actual.quiet", &paper)),
        ["head", "verdict"]
        | ["stage", "step:assets", "1", "readout", "readout", "1"]
        | ["stage", "step:ui", "构建", "1", "readout", "readout", "1"]
        | ["foot-and-inspection", "legend", "1", "eta-badge", "1"]
            if stock.name == "Light" =>
        {
            Some(PaintWitness::new(
                "plan_vs_actual.late",
                &["plan_vs_actual.card", "plan_vs_actual.late_fill"],
            ))
        }
        _ => None,
    }
}

fn reviewed_root_cause(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts
        .iter()
        .position(|part| *part == "page-root_cause_tree")?;
    let specimen = parts[page + 1..]
        .windows(2)
        .position(|pair| pair == ["sized", "case"])?;
    let slot = &parts[page + specimen + 3..];
    let modern = stock.family == "Modern";
    let tray = "root_cause.tray";
    let card = "root_cause.card";
    let gallery_paper = "surface_raised";
    let chrome = if modern { tray } else { card };
    let excluded = if modern { tray } else { gallery_paper };
    let muted = "root_cause.muted";
    let witness = match slot {
        ["head", "summary"] => PaintWitness::new(muted, &[gallery_paper]),
        ["head", "investigating", "1"] if stock.name == "Light" => {
            PaintWitness::new("root_cause.brand", &[gallery_paper, "root_cause.brand_bg"])
        }
        ["diagram", "gate:auth" | "gate:failure", "pill", "0"] => PaintWitness::new(muted, &[card]),
        [
            "diagram",
            "branch:failure",
            "cause:failure",
            "meta",
            "detail",
        ] => PaintWitness::new(muted, &[chrome]),
        [
            "diagram",
            "branch:failure",
            "cause:failure",
            "meta",
            "1",
            "0",
        ] if !modern && stock.name == "Light" => {
            PaintWitness::new("root_cause.danger", &[card, "root_cause.danger_bg"])
        }
        [
            "diagram",
            "branch:artifact",
            "cause:artifact",
            "meta",
            "status",
        ]
        | ["diagram", "branch:runner", "cause:runner", "meta", "status"]
        | [
            "diagram",
            "branch:artifact",
            "cause:artifact",
            "interior",
            "0",
            "name",
        ]
        | [
            "diagram",
            "branch:runner",
            "cause:runner",
            "interior",
            "0",
            "name",
        ] => PaintWitness::new(muted, &[excluded]),
        ["diagram", "branch:auth", "cause:auth", "meta", "status"]
        | ["diagram", "branch:policy", "cause:policy", "meta", "status"]
        | ["diagram", "branch:token", "cause:token", "meta", "status"]
        | ["inspector", "selected:token", "heading", "panel-status"] => {
            PaintWitness::new(muted, &[chrome])
        }
        [
            "diagram",
            "branch:auth",
            "cause:auth",
            "interior",
            "files",
            "1",
        ]
        | [
            "diagram",
            "branch:policy",
            "cause:policy",
            "interior",
            "files",
            "1",
        ]
        | ["inspector", "selected:token", "body", "verdict"]
        | [
            "inspector",
            "selected:token",
            "body",
            "evidence-record",
            "caption",
            "kind",
        ] => PaintWitness::new(muted, &[card]),
        [
            "inspector",
            "selected:token",
            "body",
            "evidence-record",
            "caption",
            "source",
        ] => PaintWitness::new("root_cause.faint", &[card]),
        _ => return None,
    };
    Some(witness)
}

fn reviewed_fallback_chain(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts
        .iter()
        .position(|part| *part == "page-fallback_chain")?;
    let specimen = parts[page + 1..]
        .windows(2)
        .position(|pair| pair == ["sized", "fallback-chain"])?;
    let slot = &parts[page + specimen + 3..];
    let card = "fallback_chain.card";
    let muted = "fallback_chain.muted";
    let quiet = "fallback_chain.quiet";
    let paper = [card];
    match slot {
        ["head", "summary", "state"] => {
            let head = if stock.family == "Modern" {
                "fallback_chain.tray"
            } else {
                "surface_raised"
            };
            return Some(PaintWitness::new(muted, &[head]));
        }
        ["flow", "request", "trigger", "1", "1"] => return Some(PaintWitness::new(muted, &paper)),
        ["flow", "response", "trigger", "1", "1"] => {
            return Some(PaintWitness::new(muted, &["fallback_chain.wait"]));
        }
        _ => {}
    }
    let attempts = slot.strip_prefix(&["flow", "models"])?;
    let (attempt, tail) = match attempts {
        ["attempt:atlas", tail @ ..] => ("atlas", tail),
        ["attempt:nova", "备用", tail @ ..] => ("nova", tail),
        ["attempt:sprout", tail @ ..] => ("sprout", tail),
        _ => return None,
    };
    match tail.strip_prefix(&["trigger", "attempt-content"])? {
        ["1", "0", "1", "cost"] => Some(PaintWitness::new(muted, &paper)),
        ["1", "0", "0", "0", "provider"] => Some(PaintWitness::new(quiet, &paper)),
        ["1", "0", "0", "attempt-status"] if attempt == "nova" => {
            Some(PaintWitness::new("fallback_chain.active", &paper))
        }
        ["logo", "number", "0"] if attempt == "nova" => {
            Some(PaintWitness::new(card, &["fallback_chain.active"]))
        }
        ["1", "0", "0", "attempt-status"] if attempt == "sprout" => {
            Some(PaintWitness::new(muted, &paper))
        }
        ["1", "0", "1", "timing"] if attempt == "sprout" => Some(PaintWitness::new(quiet, &paper)),
        ["1", "0", "0", "0", "model"] if attempt == "sprout" => {
            let mut witness = PaintWitness::new("fallback_chain.ink", &paper);
            witness.opacity = 0.6;
            Some(witness)
        }
        ["logo", "0"] if attempt == "sprout" => Some(
            PaintWitness::new("fallback_chain.text", &paper)
                .isolated_on("fallback_chain.fill", 0.6),
        ),
        ["logo", "number", "0"] if attempt == "sprout" => Some(
            PaintWitness::new(muted, &paper)
                .isolated_on(card, 0.6)
                .with_isolated_underlay("fallback_chain.fill"),
        ),
        _ => None,
    }
}

fn reviewed_collaboration_stream(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts
        .iter()
        .position(|part| *part == "page-collaboration_stream")?;
    let specimen = parts[page + 1..]
        .windows(2)
        .position(|pair| pair == ["sized", "activity"])?;
    let slot = &parts[page + specimen + 3..];
    let muted = "stream.muted";
    let bubble = "stream.bubble";
    match slot {
        ["0", "agents", "you" | "atlas" | "scribe", "1"] | ["0", "live", "1"] => {
            let paper = if stock.family == "Modern" {
                "stream.tray"
            } else {
                "surface_raised"
            };
            return Some(PaintWitness::new(muted, &[paper]));
        }
        ["compose", "1", "mention", "0"] | ["compose", "1", "targets", "0"] => {
            return Some(PaintWitness::new(muted, &["stream.card"]));
        }
        _ => {}
    }
    let tail = slot.strip_prefix(&["panel", "transcript:editorial:0", "item"])?;
    match tail {
        [
            "m:plan" | "m:verify" | "m:folder" | "m:confirm",
            "0",
            "1",
            "body",
            "meta",
            "at",
        ] => Some(PaintWitness::new("stream.faint", &[bubble])),
        [
            "m:plan" | "m:verify" | "m:folder",
            "0",
            "1",
            "body",
            "recipient",
            "0",
            "0",
        ]
        | ["m:confirm", "0", "1", "body", "quote", "1"] => {
            Some(PaintWitness::new(muted, &[bubble, "stream.well"]))
        }
        ["m:confirm", "0", "1", "body", "meta", "1"] if stock.name == "Light" => Some(
            PaintWitness::new("stream.wait_fg", &[bubble, "stream.wait_bg"]),
        ),
        _ => None,
    }
}

fn fixture_tail<'a>(parts: &'a [&'a str], page: &str, root: &str) -> Option<&'a [&'a str]> {
    let page = parts.iter().position(|part| *part == page)?;
    let offset = parts[page + 1..]
        .windows(2)
        .position(|pair| pair == ["sized", root])?;
    Some(&parts[page + offset + 3..])
}

fn reviewed_context_map_outer_label(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts.iter().position(|part| *part == "page-context_map")?;
    let [
        "1",
        "context_map-scroll",
        "0",
        "0",
        "1",
        "sized-map",
        "context-map",
        role @ ..,
    ] = &parts[page + 1..]
    else {
        return None;
    };
    let (paper, faint) = match (stock.family.as_str(), stock.name.as_str()) {
        ("Modern", "Light") => (0xf0f0f0, 0xb8b8b8ff),
        ("Modern", "Dark") => (0x1a1a1c, 0x5c5c5cff),
        ("Clean", "Light") => (0xf3f3f4, 0x00000045),
        ("Clean", "Dark") => (0x111113, 0xffffff47),
        _ => return None,
    };
    // These two labels lie on the known specimen's unpatterned outer paper.
    // The free tile's alternating background is not an exact solid witness.
    match role {
        ["fates", "0"] => {
            Some(PaintWitness::on_literal("ContextMap empty hint", paper).with_literal_ink(faint))
        }
        ["header", "summary", "totals", "0", "1"] => {
            Some(PaintWitness::on_literal("context_map.muted", paper))
        }
        _ => None,
    }
}

/// A non-solid risk is not inferred from an RGB enclosure alone. This
/// exact fixture has separately recorded Metal frames with and without
/// the text ink; both paper phases were underneath each of the two lines.
fn reviewed_context_map_hatch(
    stock: &ThemeVariant,
    parts: &[&str],
    evidence: &AuditContrastEvidence,
) -> Option<([f64; 3], [f64; 3])> {
    let page = parts.iter().position(|part| *part == "page-context_map")?;
    let [
        "1",
        "context_map-scroll",
        "0",
        "0",
        "1",
        "sized-map",
        "context-map",
        "field",
        "free:15:gallery",
        "context:6:writer:",
        "0",
        role,
        "0",
    ] = &parts[page + 1..]
    else {
        return None;
    };
    let (index, opacity) = match *role {
        "title" => (0, 1.0_f64),
        "counts" => (1, 0.7),
        _ => return None,
    };
    // Independently reviewed literals, not current recipe aliases.
    // Audit bounds are reported as bounds; do not substitute pixel ratios.
    let (ink, panel, stripe, audited_bounds) = match (stock.family.as_str(), stock.name.as_str()) {
        ("Modern", "Light") => (0x8a8a8aff, 0xffffff, 0x00000009, [3.194395, 2.139681]),
        ("Modern", "Dark") => (0x8a8a8aff, 0x232326, 0xffffff0a, [4.054123, 2.734845]),
        ("Clean", "Light") => (0x00000073, 0xffffff, 0x00000009, [3.321526, 2.191524]),
        ("Clean", "Dark") => (0xffffff73, 0x1c1c1f, 0xffffff0a, [4.281845, 2.579057]),
        _ => return None,
    };
    if evidence.background_srgb.is_some()
        || !evidence.conservative
        || evidence.minimum.to_bits() != 4.5_f64.to_bits()
        || evidence.element_opacity.to_bits() != opacity.to_bits()
        || evidence.foreground.as_rgba_hex() != ink
        || !evidence.ratio.is_finite()
        || (evidence.ratio - audited_bounds[index]).abs() > 1.0e-6
        || stock.tokens.color("context_map.muted")?.as_rgba_hex() != ink
        || stock.tokens.color("context_map.panel")?.as_rgba_hex() != ((panel << 8) | 0xff)
        || stock.tokens.color("context_map.free_hatch")?.as_rgba_hex() != stripe
    {
        return None;
    }
    let [_, r, g, b] = panel.to_be_bytes();
    let flat = [r, g, b].map(|v| f64::from(v) / 255.);
    Some((
        flat,
        rgba_over(gpui_rhai::Rgba8::from_rgba_hex(stripe), 1., flat),
    ))
}

fn reviewed_agent_cursors_header_or_idle_tab(
    stock: &ThemeVariant,
    parts: &[&str],
) -> Option<PaintWitness> {
    let page = parts
        .iter()
        .position(|part| *part == "page-agent_cursors")?;
    let [
        "1",
        "agent_cursors-scroll",
        "0",
        "0",
        "cursors-demo",
        "review",
        role @ ..,
    ] = &parts[page + 1..]
    else {
        return None;
    };
    // Literal author ink and tray/card; the idle tab additionally has its
    // translucent fill over the actual Gallery stage, not the sheet.
    let (ink, header, stage, tab_wash) = match (stock.family.as_str(), stock.name.as_str()) {
        ("Modern", "Light") => (0x8a8a8aff, 0xf0f0f0, 0xffffff, 0x0000000d),
        ("Modern", "Dark") => (0x7c7c82ff, 0x161618, 0x19191b, 0xffffff0f),
        ("Clean", "Light") => (0x00000073, 0xffffff, 0xffffff, 0x0000000d),
        ("Clean", "Dark") => (0xffffff73, 0x1c1c1f, 0x1b1b1e, 0xffffff0f),
        _ => return None,
    };
    match role {
        ["tabs", "tab:checks", "0"] => Some(
            PaintWitness::on_literal("AgentCursors idle review tab", stage)
                .with_literal_layer(tab_wash)
                .with_literal_ink(ink),
        ),
        [
            "review-surface",
            "sheet",
            "header-stack",
            "header",
            "title" | "summary",
        ] => Some(
            PaintWitness::on_literal("AgentCursors review header", header).with_literal_ink(ink),
        ),
        [
            "review-surface",
            "sheet",
            "header-stack",
            "header",
            "faces",
            "more",
            "label-entry",
            "label",
        ] => Some(PaintWitness::new(
            "agent_cursors.muted",
            &["agent_cursors.card"],
        )),
        [
            "review-surface",
            "sheet",
            "header-stack",
            "header",
            "faces",
            "follow:kim" | "follow:tess",
            "0",
        ] => {
            let face_ink = if stock.name == "Dark" {
                0xe4e4e7ff
            } else {
                0x1f1f22ff
            };
            Some(
                PaintWitness::on_literal("AgentCursors away avatar", header)
                    .isolated_on("agent_cursors.card", 0.4)
                    .with_literal_ink(face_ink),
            )
        }
        _ => None,
    }
}

/// Header/regular card recipe in these two reviewed Gallery fixtures only.
/// Individual roles still decide which of the two papers they paint on.
fn overview_fixture_paper(stock: &ThemeVariant) -> Option<(u32, u32, u32)> {
    match (stock.family.as_str(), stock.name.as_str()) {
        ("Modern", "Light") => Some((0xf0f0f0, 0xffffff, 0x8a8a8aff)),
        ("Clean", "Light") => Some((0xffffff, 0xffffff, 0x00000073)),
        ("Clean", "Dark") => Some((0x1b1b1e, 0x1c1c1f, 0xffffff73)),
        _ => None,
    }
}

fn reviewed_parallel_attempts(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts
        .iter()
        .position(|part| *part == "page-parallel_attempts")?;
    let [
        "1",
        "parallel_attempts-scroll",
        "0",
        "0",
        "1",
        "sized",
        "parallel-attempts",
        role @ ..,
    ] = &parts[page + 1..]
    else {
        return None;
    };
    let (header, card, muted) = overview_fixture_paper(stock)?;
    if matches!(role, ["0", "2"] | ["0", "3", "0", "1"]) {
        return Some(
            PaintWitness::on_literal("ParallelAttempts description or legend", header)
                .with_literal_ink(muted),
        );
    }
    let (id, role) = match role {
        ["attempts:test-race", "0", "attempt:api", "界", role @ ..] => ("api", role),
        ["attempts:test-race", "0", "attempt:trace", role @ ..] => ("trace", role),
        ["attempts:test-race", "0", "attempt:wait", role @ ..] => ("wait", role),
        _ => return None,
    };
    let (description, ink) = match role {
        ["who", "0", "1" | "2"] | ["end", "1", "0"] => ("ParallelAttempts secondary ink", muted),
        ["end", "running", "0"] if id != "wait" => ("ParallelAttempts pending score", muted),
        ["who", "running"] if stock.family == "Modern" && id != "wait" => {
            ("ParallelAttempts running status", 0x0190feff)
        }
        ["who", "failed"] | ["end", "failed", "0"]
            if stock.family == "Clean" && stock.name == "Light" && id == "wait" =>
        {
            ("ParallelAttempts failed status", 0xd72400da)
        }
        _ => return None,
    };
    // This review is for the ordinary, undimmed fixture only. The note in
    // PaintWitness will reject changed opacity, event state or Host paper.
    Some(PaintWitness::on_literal(description, card).with_literal_ink(ink))
}

fn reviewed_fan_out_grid(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts.iter().position(|part| *part == "page-fan_out_grid")?;
    let [
        "1",
        "fan_out_grid-scroll",
        "0",
        "0",
        "1",
        "size",
        "fan-out-grid",
        role @ ..,
    ] = &parts[page + 1..]
    else {
        return None;
    };
    let (header, card, muted) = overview_fixture_paper(stock)?;
    let footer = if stock.family == "Modern" {
        header
    } else {
        card
    };
    let (description, paper, ink) = match role {
        ["0", "2"] | ["0", "3", "0", "1"] | ["0", "3", "2"] => (
            "FanOutGrid job, count denominator or elapsed time",
            header,
            muted,
        ),
        [
            "body",
            "footer",
            "legends",
            "done" | "running" | "failed",
            "2",
        ] => ("FanOutGrid unselected legend label", footer, muted),
        ["body", "footer", "retry", "1"] if stock.family == "Clean" && stock.name == "Light" => {
            ("FanOutGrid retry label", footer, 0xd72400da)
        }
        _ => return None,
    };
    Some(PaintWitness::on_literal(description, paper).with_literal_ink(ink))
}

fn reviewed_context_map_tile(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts.iter().position(|part| *part == "page-context_map")?;
    let rest = &parts[page + 1..];
    let offset = rest
        .windows(3)
        .position(|parts| parts == ["sized-map", "context-map", "field"])?;
    // Only the six specific data tiles of the maintained Writer specimen.
    // The hatched free region and panel/header/status content are not covered.
    let (tile, tail, role) = match &rest[offset + 3..] {
        ["block:15:gallery", "context:6:writer:__free", tail @ ..] => {
            (0, tail, "context_map.system_ink")
        }
        ["block:15:gallery", "context:6:writer:tools", tail @ ..] => {
            (1, tail, "context_map.tools_ink")
        }
        ["block:15:gallery", "context:6:writer:memory", tail @ ..] => {
            (2, tail, "context_map.memory_ink")
        }
        [
            "block:15:gallery",
            "context:6:writer:files",
            "资料",
            tail @ ..,
        ] => (3, tail, "context_map.file_ink"),
        ["block:15:gallery", "context:6:writer:chat", tail @ ..] => {
            (4, tail, "context_map.chat_ink")
        }
        ["block:15:gallery", "context:6:writer:results", tail @ ..] => {
            (5, tail, "context_map.result_ink")
        }
        _ => return None,
    };
    let opacity = match tail {
        ["0", "title", "0"] => 1.0,
        ["0", "counts", "0"] | ["0", "counts", "detail", "0"] => 0.7,
        _ => return None,
    };
    // Reviewed literal ink/plate pairs, not aliases into the candidate
    // palette: editing the preset must not redefine the expected risk.
    let (panel, swatches) = match (stock.family.as_str(), stock.name.as_str()) {
        ("Modern", "Light") => (
            0xffffff,
            [
                (0x5c5c5cff, 0xf3f3f3ff),
                (0x08658cff, 0xe2f3f9ff),
                (0x8a35d6ff, 0xf3ecfcff),
                (0x0190feff, 0xe6f3fdff),
                (0x8a35d6ff, 0xf1eafaff),
                (0xb25e00ff, 0xffecd6ff),
            ],
        ),
        ("Modern", "Dark") => (
            0x232326,
            [
                (0xbdbdbdff, 0x2b2b2fff),
                (0x5cc6eeff, 0x122f3bff),
                (0xd7bcf6ff, 0x38244aff),
                (0x8bcbfeff, 0x1d364dff),
                (0xd1b2f5ff, 0x312041ff),
                (0xffb35cff, 0x3a2712ff),
            ],
        ),
        ("Clean", "Light") => (
            0xffffff,
            [
                (0x0000009c, 0x0000000d),
                (0x08658cff, 0x14b3f016),
                (0x8a35d6ff, 0xa651f01a),
                (0x0070d6ff, 0x0060e214),
                (0x8a35d6ff, 0xa651f017),
                (0xb25e00ff, 0xf28a2e21),
            ],
        ),
        ("Clean", "Dark") => (
            0x1c1c1f,
            [
                (0xffffffb3, 0xffffff12),
                (0x5cc6eeff, 0x5cc6ee29),
                (0xd7bcf6ff, 0xb98af02e),
                (0x8bcbfeff, 0x4fb0fe2e),
                (0xd1b2f5ff, 0xb98af02b),
                (0xffb35cff, 0xf2a15c2b),
            ],
        ),
        _ => return None,
    };
    let (ink, paper) = swatches[tile];
    let mut witness = PaintWitness::on_literal(role, panel)
        .with_literal_layer(paper)
        .with_literal_ink(ink);
    witness.opacity = opacity;
    Some(witness)
}

fn reviewed_memory_panel(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let slot = fixture_tail(parts, "page-memory_panel", "case")?;
    let card = "memory_panel.card";
    let muted = "memory_panel.muted";
    let selected = "memory_panel.fill";
    let modern = stock.family == "Modern";
    let light = stock.name == "Light";
    match slot {
        ["0", "0", "0", "total", "1"] if modern && light => {
            Some(PaintWitness::new(muted, &[card, selected]))
        }
        [
            "0",
            "filters",
            "0",
            "filter-chips",
            "memory-chip:filter:0",
            "chip:0",
            "1",
        ] if modern && light => Some(PaintWitness::new(muted, &[card, selected])),
        [
            "0",
            "filters",
            "0",
            "filter-chips",
            "memory-chip:filter:1",
            "chip:1",
            "1",
        ]
        | [
            "0",
            "filters",
            "0",
            "filter-chips",
            "memory-chip:filter:2",
            "chip:2",
            "1",
        ]
        | [
            "0",
            "filters",
            "0",
            "filter-chips",
            "memory-chip:filter:3",
            "chip:3",
            "1",
        ]
        | [
            "0",
            "memory-list",
            "category:Work" | "category:Preferences" | "category:Projects",
            "group",
        ]
        | [
            "0",
            "memory-list",
            "category:Preferences",
            "item:tea",
            "0",
            "fact",
        ] => Some(PaintWitness::new(muted, &[card])),
        [
            "0",
            "memory-list",
            "category:Work",
            "item:rust",
            "0",
            "meta",
            "0",
            "description",
            tail,
        ]
        | [
            "0",
            "memory-list",
            "category:Preferences",
            "item:tea",
            "0",
            "meta",
            "0",
            "description",
            tail,
        ]
        | [
            "0",
            "memory-list",
            "category:Projects",
            "item:orbit",
            "0",
            "meta",
            "0",
            "description",
            tail,
        ] if ["source", "separator", "date"].contains(tail) => {
            let witness = PaintWitness::new(muted, &[card]);
            Some(if !modern && *tail == "separator" {
                witness.with_literal_ink(if light { 0x00000045 } else { 0xffffff4d })
            } else {
                witness
            })
        }
        [
            "0",
            "memory-list",
            "category:Preferences",
            "item:tea",
            "0",
            "meta",
            "0",
            "1",
        ] if !modern && light => Some(PaintWitness::new(
            "memory_panel.paused",
            &[card, "memory_panel.paused_fill"],
        )),
        ["0", "bottom", "add-memory", "1"] if modern => Some(PaintWitness::new(muted, &[card])),
        ["1", "0"] => Some(PaintWitness::new(
            muted,
            &[if modern { "memory_panel.tray" } else { card }],
        )),
        _ => None,
    }
}

fn reviewed_judge_bracket(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let slot = fixture_tail(parts, "page-judge_bracket", "evaluation")?;
    let paper = "surface_raised";
    let card = "bracket.card";
    let muted = "bracket.muted";
    let faint = "bracket.faint";
    let modern = stock.family == "Modern";
    match slot {
        ["header", "2" | "summary"] => Some(PaintWitness::new(muted, &[paper])),
        ["header", "live", "1"] if stock.name == "Light" => {
            Some(PaintWitness::new("bracket.blue", &[paper, "bracket.tag"]))
        }
        ["bracket", "round:1"] => Some(PaintWitness::new(faint, &[paper])),
        [
            "bracket",
            "match-box:1:0",
            "contest",
            "upper" | "lower",
            "0",
        ] => Some(PaintWitness::new(faint, &[paper, "bracket.hint"])),
        ["match-details", "comparison:0:0", "panel-head", "1"] => Some(PaintWitness::new(
            muted,
            &[if modern { "bracket.tray" } else { card }],
        )),
        [
            "match-details",
            "comparison:0:0",
            "answers",
            "answer:0" | "answer:1",
            "1",
        ] => Some(if modern {
            PaintWitness::new(faint, &[card])
        } else {
            PaintWitness::new(faint, &[card, "bracket.well"])
        }),
        _ => None,
    }
}

fn reviewed_running_now_trigger(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts.iter().position(|part| *part == "page-running_now")?;
    let [
        "1",
        "running_now-scroll",
        "0",
        "0",
        "1",
        "0",
        "running",
        "popover",
        "trigger",
        role @ ..,
    ] = &parts[page + 1..]
    else {
        return None;
    };
    let (paper, quiet, separator) = match (stock.family.as_str(), stock.name.as_str()) {
        ("Modern", "Light") => (0xf0f0f0, Some(0x8a8a8aff), 0xb8b8b8ff),
        ("Modern", "Dark") => (0x19191c, Some(0x7c7c82ff), 0x4a4a50ff),
        ("Clean", "Light") => (0xffffff, None, 0x00000045),
        ("Clean", "Dark") => (0x1c1c1e, None, 0xffffff4d),
        _ => return None,
    };
    match role {
        ["running" | "waiting", "caption"] => Some(
            PaintWitness::on_literal("RunningNow authored trigger word", paper)
                .with_literal_ink(quiet?),
        ),
        ["dot"] => Some(
            PaintWitness::on_literal("RunningNow authored decorative separator", paper)
                .with_literal_ink(separator),
        ),
        _ => None,
    }
}

fn reviewed_context_window(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    if stock.name != "Light" {
        return None;
    }
    let page = parts
        .iter()
        .position(|part| *part == "page-context_window")?;
    let [
        "1",
        "context_window-scroll",
        "0",
        "0",
        "1",
        "1",
        "capacity",
        role @ ..,
    ] = &parts[page + 1..]
    else {
        return None;
    };
    let clean = stock.family == "Clean";
    match role {
        ["footer", "note"] => Some(
            PaintWitness::on_literal(
                "ContextWindow authored footer",
                if clean { 0xffffff } else { 0xf0f0f0 },
            )
            .with_literal_ink(if clean { 0x00000073 } else { 0x8f8f8fff }),
        ),
        ["card", "summary", "number:percent"] if clean => Some(
            PaintWitness::on_literal("ContextWindow authored warning badge", 0xffffff)
                .with_literal_layer(0xe8b0001f)
                .with_literal_ink(0x8f6c00ff),
        ),
        _ => None,
    }
}

// The authored chart labels and receipt caption are not the generic muted role.
// Record only the Gallery specimens whose actual backing was independently checked.
fn reviewed_receipt_or_chart_ink(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts.iter().position(|part| {
        matches!(
            *part,
            "page-cost_waterfall" | "page-breakdown_chart" | "page-smart_recommendation"
        )
    })?;
    let design = (stock.family.as_str(), stock.name.as_str());
    match &parts[page..] {
        [
            "page-cost_waterfall",
            "1",
            "cost_waterfall-scroll",
            "0",
            "0",
            "1",
            "sized",
            "waterfall",
            "header",
            "title-group",
            "task",
        ] => match design {
            ("Modern", _) => Some(PaintWitness::new(
                "cost_waterfall.muted",
                &["cost_waterfall.tray"],
            )),
            ("Clean", "Light") => Some(PaintWitness::on_literal("cost_waterfall.muted", 0xffffff)),
            ("Clean", "Dark") => Some(PaintWitness::on_literal("cost_waterfall.muted", 0x1c1c1e)),
            _ => None,
        },
        [
            "page-cost_waterfall",
            "1",
            "cost_waterfall-scroll",
            "0",
            "0",
            "1",
            "sized",
            "waterfall",
            "header",
            "run-state",
            "label",
        ] if design == ("Clean", "Light") => Some(
            PaintWitness::on_literal("cost_waterfall.status_running", 0xffffff)
                .with_literal_layer(0x008ff519)
                .with_literal_ink(0x006dcbf2),
        ),
        [
            "page-breakdown_chart",
            "1",
            "breakdown_chart-scroll",
            "0",
            "0",
            "1",
            "0",
            "workload",
            "heading",
            "total",
            "unit",
        ] => match design {
            ("Modern", "Light") => Some(PaintWitness::new("breakdown_chart.muted", &["cite.tray"])),
            ("Clean", "Light") => Some(PaintWitness::on_literal("breakdown_chart.muted", 0xffffff)),
            _ => None,
        },
        [
            "page-smart_recommendation",
            "1",
            "smart_recommendation-scroll",
            "0",
            "0",
            "1",
            "frame",
            "case",
            "foot",
            "0",
            "confidence",
            "1",
        ] if design == ("Modern", "Light") => Some(
            PaintWitness::on_literal("smart_recommendation.muted", 0xf0f0f0)
                .with_literal_ink(0x8f8f8fff),
        ),
        _ => None,
    }
}

fn reviewed_profile_menu(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    if stock.name != "Light" {
        return None;
    }
    let page = parts.iter().position(|part| *part == "page-profile_menu")?;
    let [
        "1",
        "profile_menu-scroll",
        "0",
        "0",
        "1",
        "view",
        "profile",
        role @ ..,
    ] = &parts[page + 1..]
    else {
        return None;
    };
    let clean = stock.family == "Clean";
    match role {
        ["card", "actions", "item:apps", "badge"] => Some(
            PaintWitness::on_literal("ProfileMenu authored New badge", 0xffffff)
                .with_literal_layer(if clean { 0xf0443814 } else { 0xfff0f3ff })
                .with_literal_ink(0xe12a1dff),
        ),
        ["setup", "heading", "count"] | ["setup", "next", "words", "hint"] => Some(
            PaintWitness::on_literal(
                "ProfileMenu authored setup metadata",
                if clean { 0xffffff } else { 0xf0f0f0 },
            )
            .with_literal_ink(if clean { 0x00000073 } else { 0x8f8f8fff }),
        ),
        _ => None,
    }
}

fn reviewed_summary_or_tips_header(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    if stock.name != "Light" {
        return None;
    }
    let page = parts
        .iter()
        .position(|part| matches!(*part, "page-change_summary" | "page-tips_panel"))?;
    let clean = stock.family == "Clean";
    let paper = if clean { 0xffffff } else { 0xf0f0f0 };
    match &parts[page..] {
        [
            "page-change_summary",
            "1",
            "change_summary-scroll",
            "0",
            "0",
            "change-summary-demo",
            "width",
            "summary",
            "content",
            "head",
            "title-group",
            "title",
        ] if !clean => Some(
            PaintWitness::on_literal("ChangeSummary authored blue title", paper)
                .with_literal_ink(0x4fb0feff),
        ),
        [
            "page-change_summary",
            "1",
            "change_summary-scroll",
            "0",
            "0",
            "change-summary-demo",
            "width",
            "summary",
            "content",
            "head",
            "period",
        ] => Some(
            PaintWitness::on_literal("ChangeSummary authored period", paper)
                .with_literal_ink(if clean { 0x00000073 } else { 0x737373ff }),
        ),
        [
            "page-tips_panel",
            "1",
            "tips_panel-scroll",
            "0",
            "0",
            "tips-panel-demo",
            "width",
            "tips",
            "head",
            "count",
        ] => Some(
            PaintWitness::on_literal("TipsPanel authored counter", paper)
                .with_literal_ink(if clean { 0x00000073 } else { 0x737373ff }),
        ),
        _ => None,
    }
}

fn reviewed_project_card(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    if stock.name != "Light" {
        return None;
    }
    let page = parts.iter().position(|part| *part == "page-project_card")?;
    let [
        "1",
        "project_card-scroll",
        "0",
        "0",
        "project-card-demo",
        "width",
        "release-card",
        "paper",
        "card",
        "head",
        "1",
        "1",
        slot @ ..,
    ] = &parts[page + 1..]
    else {
        return None;
    };
    let clean = stock.family == "Clean";
    match slot {
        ["status", "label"] if !clean => Some(
            PaintWitness::on_literal("ProjectCard authored amber status", 0xffffff)
                .with_literal_layer(0xe8b00024)
                .with_literal_ink(0xb25e00ff),
        ),
        ["due"] => Some(
            PaintWitness::on_literal("ProjectCard authored due date", 0xffffff)
                .with_literal_ink(if clean { 0x00000073 } else { 0x8f8f8fff }),
        ),
        _ => None,
    }
}

fn reviewed_experiment_card(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    if stock.name != "Light" {
        return None;
    }
    let page = parts
        .iter()
        .position(|part| *part == "page-experiment_card")?;
    let [
        "1",
        "experiment_card-scroll",
        "0",
        "0",
        "experiment-card-demo",
        "width",
        "onboarding-card",
        "paper",
        "card",
        "head",
        slot @ ..,
    ] = &parts[page + 1..]
    else {
        return None;
    };
    let clean = stock.family == "Clean";
    match slot {
        ["titles", "1", "chip", "words"] | ["leader", "variant" | "words"] => Some(
            PaintWitness::on_literal("ExperimentCard green chip", 0xffffff)
                .with_literal_layer(if clean { 0x5fc88a14 } else { 0x5fc88a1c })
                .with_literal_ink(0x1e8a4fff),
        ),
        ["titles", "1", "1"] => {
            Some(
                PaintWitness::on_literal("ExperimentCard note", 0xffffff)
                    .with_literal_ink(if clean { 0x00000073 } else { 0x8f8f8fff }),
            )
        }
        _ => None,
    }
}

fn reviewed_rate_planner(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let slot = fixture_tail(parts, "page-rate_limit_planner", "case")?;
    if stock.name != "Light" {
        return None;
    }
    let clean = stock.family == "Clean";
    match slot {
        ["run:review", "who", "start"] => Some(
            PaintWitness::on_literal("RateLimitPlanner selected start time", 0xffffff)
                .with_literal_layer(0xa651f00d)
                .with_literal_ink(0xa651f0ff),
        ),
        [
            "run:review",
            "lane-track",
            "track",
            "drag-window",
            "drag-item",
            "bar-display",
            "amount",
        ] => Some(
            PaintWitness::on_literal("RateLimitPlanner selected amount", 0xffffff)
                // 5% lane tint, then 15% bar wash; the ink is a separately
                // authored 85/15 premultiplied mix, not a tinted white label.
                .with_literal_layer(0xa651f00d)
                .with_literal_layer(0xa651f026)
                .with_literal_ink(if clean { 0x9046d0fa } else { 0x9148cfff }),
        ),
        [
            "run:digest",
            "lane-track",
            "track",
            "drag-window",
            "drag-item",
            "bar-display",
            "amount",
        ] => Some(
            PaintWitness::on_literal("RateLimitPlanner ordinary amount", 0xffffff)
                .with_literal_layer(0x13b3a014)
                .with_literal_ink(if clean { 0x0e8274f6 } else { 0x148477ff }),
        ),
        ["plot-row", "plot-frame", "load-plot", "checkpoint-flag"] => Some(
            PaintWitness::on_literal("RateLimitPlanner authored checkpoint", 0xffffff)
                .with_literal_ink(if clean { 0x00000073 } else { 0x8f8f8fff }),
        ),
        ["heading", "0", "1"]
        | ["plot-row", "0", "1"]
        | ["axis", "1", "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7"]
        | ["run:index", "who", "1"]
        | ["run:index" | "run:digest" | "run:library", "who", "start"]
        | ["add-row", "add-run", "1"]
        | ["foot", "0"] => Some(PaintWitness::new("rate_plan.muted", &["rate_plan.card"])),
        _ => None,
    }
}

fn reviewed_usage_meter(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    if stock.name != "Light" {
        return None;
    }
    let page = parts.iter().position(|part| *part == "page-usage_meter")?;
    let root = parts[page + 1..]
        .windows(2)
        .position(|pair| pair == ["width", "usage-card"])?;
    let slot = &parts[page + root + 3..];
    let muted = "compact_breakdown.muted";
    let card = ["surface_raised"];
    match slot {
        ["panel", "head", "header-title", "1"]
        | ["panel", "figure", "0", "1"]
        | [
            "panel",
            "legend",
            "row:compute" | "row:search" | "row:storage" | "row:media",
            "share",
        ]
        | ["panel", "daily", "daily-head", "0" | "readout"]
        | [
            "panel",
            "limits",
            "limit-line:0",
            "limit:seats" | "limit:storage" | "limit:projects",
            "limit-head",
            "0",
        ]
        | [
            "panel",
            "limits",
            "limit-line:0",
            "limit:seats" | "limit:storage" | "limit:projects",
            "limit-read",
            "1",
        ] => Some(PaintWitness::new(muted, &card)),
        [
            "panel",
            "head",
            "periods",
            "period:week" | "period:quarter",
            "0",
        ] if stock.family == "Modern" => Some(PaintWitness::on_literal(muted, 0xf4f4f5)),
        ["footer", "status", "remaining"] if stock.family == "Modern" => {
            Some(PaintWitness::on_literal(muted, 0xf0f0f0))
        }
        ["footer", "status", "remaining"] if stock.family == "Clean" => {
            Some(PaintWitness::new(muted, &card))
        }
        ["panel", "figure", "quota-pct", "capacity-percent"] if stock.family == "Clean" => {
            let mut witness = PaintWitness::new(
                "usage_meter.warn_text",
                &["surface_raised", "usage_meter.warn"],
            );
            witness.papers[1].opacity = 31. / 255.;
            Some(witness)
        }
        _ => None,
    }
}

fn reviewed_diff_timeline(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let slot = fixture_tail(parts, "page-diff_timeline", "timeline")?;
    let faint = "diff_timeline.faint";
    let muted = "diff_timeline.muted";
    let card = "diff_timeline.card";
    match slot {
        ["panel", "head", "1", "1", "directory"]
        | ["panel", "body", "axis", "0" | "1" | "2" | "3" | "4"]
        | ["panel", "body", "1", "ticks", "0" | "1" | "2" | "3" | "4"] => {
            return Some(PaintWitness::new(faint, &[card]));
        }
        ["panel", "head", "1", "1", "editing"]
        | ["panel", "head", "1", "0", "1"]
        | ["panel", "head", "totals", "2"] => return Some(PaintWitness::new(muted, &[card])),
        ["panel", "head", "totals", "total:add", "0"] if stock.name == "Light" => {
            let mut witness =
                PaintWitness::new("diff_timeline.add_ink", &[card, "diff_timeline.add"]);
            witness.papers[1].opacity = 36. / 255.;
            return Some(witness);
        }
        ["legend", "0" | "1" | "2", "1", "0"] => {
            return Some(PaintWitness::new(
                muted,
                &[if stock.family == "Clean" {
                    card
                } else {
                    "diff_timeline.tray"
                }],
            ));
        }
        _ => {}
    }
    let tail = slot.strip_prefix(&["file-panel", "files", "rows"])?;
    match tail {
        ["file:src", "lib.rs" | "api.rs", "file-path", "directory"]
        | ["file:config", "default.toml", "file-path", "directory"]
        | ["file:assets", "preview.png", "file-path", "directory"] => {
            Some(PaintWitness::new(faint, &[card]))
        }
        ["file:src", "lib.rs", "counts", "count:add"] => Some(PaintWitness::new(faint, &[card])),
        ["file:assets", "preview.png", "counts", "0"] => Some(PaintWitness::new(muted, &[card])),
        ["file:src", "api.rs", "counts", "count:add"]
        | ["file:config", "default.toml", "counts", "count:add"]
            if stock.name == "Light" =>
        {
            Some(PaintWitness::new("diff_timeline.add_ink", &[card]))
        }
        _ => None,
    }
}

fn reviewed_compact_breakdown(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    if stock.name != "Light" || !parts.contains(&"page-compact_breakdown") {
        return None;
    }
    let start = parts
        .windows(3)
        .position(|items| items == ["compact-breakdown-demo", "card-width", "cost-card"])?;
    let muted = "compact_breakdown.muted";
    let card = "compact_breakdown.card";
    match parts.get(start + 3..)? {
        [
            "panel",
            "heading",
            "periods",
            "period:week" | "period:year",
            "0",
        ] if stock.family == "Modern" => Some(PaintWitness::on_literal(muted, 0xf4f4f5)),
        ["panel", "figure", "delta", "1"] => Some(PaintWitness::new(
            "compact_breakdown.up",
            &[card, "compact_breakdown.up_bg"],
        )),
        ["panel", "caption"]
        | [
            "panel",
            "ranking",
            "item:5:costs:inference"
            | "item:5:costs:compute"
            | "item:5:costs:storage"
            | "item:5:costs:transfer"
            | "item:5:costs:archive",
            "share",
        ] => Some(PaintWitness::new(muted, &[card])),
        ["footer", "0"] => Some(if stock.family == "Modern" {
            PaintWitness::on_literal(muted, 0xf0f0f0)
        } else {
            PaintWitness::new(muted, &[card])
        }),
        _ => None,
    }
}

fn reviewed_scheduled_runs(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts.iter().position(|p| *p == "page-scheduled_runs")?;
    let entry = parts[page + 1..]
        .windows(2)
        .position(|pair| pair == ["0", "plan"])?;
    let slot = &parts[page + entry + 3..];
    let modern = stock.family == "Modern";
    let dark = stock.name == "Dark";
    let card = "scheduled_runs.card";
    let muted = "scheduled_runs.muted";
    // This is the known, opaque Gallery fixture canvas; the SDK doesn't
    // choose this color for other callers embedding a Clean board.
    let clean_stage = if dark { 0x111113 } else { 0xf3f3f4 };
    let bare = |ink| {
        if modern {
            PaintWitness::new(ink, &[card])
        } else {
            PaintWitness::on_literal(ink, clean_stage)
        }
    };
    let day = |ink| {
        if modern {
            PaintWitness::new(ink, &[card, "scheduled_runs.well"])
        } else {
            bare(ink).with_literal_layer(if dark { 0xffffff08 } else { 0x00000006 })
        }
    };
    if let [
        "panel",
        "calendar",
        "columns",
        "am",
        "slot:3:monam",
        "drop:3:monam-content",
        "cards",
        "part:last-run",
        "detail",
        "trigger",
        role @ ..,
    ] = slot
    {
        let historical = |ink| bare(ink).isolated_on(card, 0.45);
        return match role {
            ["title-line", "title"] => Some(historical("scheduled_runs.ink")),
            ["title-line", "0", "0"] => {
                Some(historical("scheduled_runs.ink").with_isolated_underlay(card))
            }
            ["summary"] => Some(historical(muted)),
            ["stats", "usage-pill", "percentage"] => Some(
                historical("scheduled_runs.text").with_isolated_wash(if dark {
                    0xffffff12
                } else if modern {
                    0x0000000f
                } else {
                    0x0000000d
                }),
            ),
            _ => None,
        };
    }
    match slot {
        ["head", "heading-text", "1"] | ["head", "1", "1"] => Some(PaintWitness::on_literal(
            muted,
            if modern {
                if dark { 0x141416 } else { 0xf0f0f0 }
            } else {
                clean_stage
            },
        )),
        ["panel", "stack", "backlog-head", "0"] => {
            Some(PaintWitness::new(muted, &["scheduled_runs.lane"]))
        }
        ["head", "1", "mode-track", "auto", "1"] if modern && stock.name == "Light" => Some(
            PaintWitness::on_literal("scheduled_runs.text", 0xf0f0f0)
                .with_literal_layer(0x0000000f),
        ),
        [
            "panel",
            "calendar",
            "columns",
            "days",
            "mon" | "tue" | "wed",
            "0",
        ] => Some(day(muted)),
        ["panel", "calendar", "columns", "days", "tue" | "wed", "1"] => {
            Some(day("scheduled_runs.text"))
        }
        ["panel", "calendar", "columns", "am" | "pm", "period", "0"] => {
            Some(bare("scheduled_runs.faint"))
        }
        [
            "panel",
            "stack",
            "items",
            "task:audit" | "task:media",
            "detail",
            "trigger",
            "stats",
            "0",
            "0",
        ] => Some(PaintWitness::new(muted, &[card])),
        [
            "panel",
            "stack",
            "items",
            "task:audit" | "task:media" | "task:backup",
            "detail",
            "trigger",
            "summary",
        ]
        | [
            "panel",
            "calendar",
            "columns",
            "pm",
            "slot:3:monpm",
            "drop:3:monpm-content",
            "cards",
            "part:release-window",
            "detail",
            "trigger",
            "summary",
        ] => Some(PaintWitness::new(muted, &[card])),
        _ => None,
    }
}

fn reviewed_radar_directory(parts: &[&str]) -> Option<PaintWitness> {
    if !parts.contains(&"page-conflict_radar") {
        return None;
    }
    let start = parts
        .windows(2)
        .position(|pair| pair == ["sample", "branches"])?;
    let tail = parts.get(start + 2..)?;
    let papers: &[&str] = match tail {
        [
            "card",
            "viewport",
            "0",
            "rows",
            "item",
            "file:guide",
            "1",
            "1",
            "directory",
        ] => &["radar.card", "radar.hit_row"],
        [
            "card",
            "viewport",
            "0",
            "rows",
            "item",
            "file:spec",
            "0",
            "1",
            "directory",
        ] => &["radar.card", "radar.stripe"],
        [
            "card",
            "viewport",
            "0",
            "rows",
            "item",
            "file:readme",
            "0",
            "1",
            "directory",
        ] => &["radar.card"],
        _ => return None,
    };
    Some(PaintWitness::new("radar.faint", papers))
}

fn reviewed_tool_access_map(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let slot = fixture_tail(parts, "page-tool_access_map", "case")?;
    let head = if stock.family == "Modern" {
        "tool_access.tray"
    } else {
        "surface_raised"
    };
    let card = "tool_access.card";
    let muted = "tool_access.muted";
    match slot {
        ["head", "4"] => return Some(PaintWitness::new(muted, &[head])),
        ["head", "running", "badge", "1"] if stock.name == "Light" => {
            return Some(PaintWitness::new(
                "tool_access.live",
                &[head, "tool_access.live_fill"],
            ));
        }
        _ => {}
    }
    match slot.strip_prefix(&["body", "content", "map-stage", "layout"])? {
        ["start", "tool:search" | "tool:read", "0", "count"]
        | ["agent-card", "person:planner", "0", "1", "model"]
        | ["end", "tool:notes", "1", "info"] => Some(PaintWitness::new(muted, &[card])),
        ["start", "tool:read", "1", "badge", "0"]
        | ["agent-card", "person:planner", "0", "agent_status", "1"]
            if stock.name == "Light" =>
        {
            Some(PaintWitness::new(
                "tool_access.live",
                &[card, "tool_access.live_fill"],
            ))
        }
        ["start", "tool:search", "1", "badge", "0"]
            if stock.family == "Clean" && stock.name == "Light" =>
        {
            Some(PaintWitness::new(
                "tool_access.danger",
                &[card, "tool_access.danger_fill"],
            ))
        }
        _ => None,
    }
}

fn reviewed_billing_schedule(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts.iter().position(|p| *p == "page-billing_schedule")?;
    let entry = parts[page + 1..]
        .windows(2)
        .position(|pair| pair == ["surface", "billing-schedule"])?;
    let slot = &parts[page + entry + 3..];
    let modern = stock.family == "Modern";
    let muted = "billing_schedule.muted";
    let card = "billing_schedule.card";
    match slot {
        ["details", "table", "columns", "0" | "1" | "2"]
        | [
            "details",
            "table",
            "lines",
            "line:seed",
            "one",
            "price",
            "0",
            "number:seed",
            "one",
            "field",
            "0",
            "prefix",
            "0",
        ]
        | [
            "details",
            "table",
            "lines",
            "line:seed",
            "one",
            "price",
            "0",
            "1",
        ] if stock.name == "Light" => Some(PaintWitness::new(muted, &[card])),
        ["totals", "invoice" | "total", "0"] if stock.name == "Light" => Some(PaintWitness::new(
            muted,
            &[if modern {
                "billing_schedule.tray"
            } else {
                card
            }],
        )),
        ["details", "summary", "range", "separator"] if !modern => Some(
            PaintWitness::new("billing_schedule.phase_dot", &[card]).with_literal_ink(
                if stock.name == "Light" {
                    0x0000004d
                } else {
                    0xffffff4d
                },
            ),
        ),
        _ => None,
    }
}

/// Authored contrast risks of the fixed Gallery trace, tied to roles and
/// to their actual paper. Ordinary duration text is never an exception.
fn reviewed_trace_waterfall(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    let page = parts
        .iter()
        .position(|part| *part == "page-trace_waterfall")?;
    let specimen = parts[page + 1..]
        .windows(2)
        .position(|path| path == ["sized", "run"])?;
    let tail = &parts[page + specimen + 3..];
    if stock.name == "Light" {
        match tail {
            ["head", "head-status", "chip-text"] if stock.family == "Clean" => {
                return Some(PaintWitness::new(
                    "trace.live_ink",
                    &["trace.tray", "trace.live_bg"],
                ));
            }
            ["card", "stats", "metric:5", "stat-value"]
            | [
                "card",
                "viewport",
                "tree",
                "span:archive",
                "trigger",
                "lane",
                "duration",
                "duration-tag",
                "readout-label",
            ] if stock.family == "Clean" => {
                return Some(PaintWitness::new("trace.error_ink", &["trace.card"]));
            }
            [
                "card",
                "viewport",
                "tree",
                "span:reconcile",
                "trigger",
                "lane",
                "duration",
                "duration-tag",
                "slow-badge",
                "chip-text",
            ] => {
                return Some(PaintWitness::new(
                    "trace.slow_ink",
                    &["trace.card", "trace.slow_bg"],
                ));
            }
            _ => {}
        }
    }
    let paper = match tail {
        ["head", "task-caption"] => "trace.tray",
        ["card", "stats", metric, "stat-label"]
            if matches!(
                *metric,
                "metric:0" | "metric:1" | "metric:2" | "metric:3" | "metric:4" | "metric:5"
            ) =>
        {
            "trace.card"
        }
        ["card", "axis", "axis-caption", "axis-label"] => "trace.card",
        [
            "card",
            "axis",
            "axis-flow",
            "axis-track",
            tick,
            "tick-label",
        ] if tick
            .strip_prefix("tick:")
            .is_some_and(|key| key.parse::<u64>().is_ok()) =>
        {
            "trace.card"
        }
        [
            "card",
            "viewport",
            "tree",
            span,
            "trigger",
            "0",
            "span-labels",
            "span-model-caption",
        ] if matches!(
            *span,
            "span:outline"
                | "span:research"
                | "span:search"
                | "span:reconcile"
                | "span:archive"
                | "span:draft"
        ) =>
        {
            "trace.card"
        }
        _ => return None,
    };
    Some(PaintWitness::new("trace.muted", &[paper]))
}

/// Exact text and backdrop pairs of the initial accepted ledger fixture.
fn reviewed_ledger(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    if !parts.contains(&"page-agent_ledger") {
        return None;
    }
    let demo = parts.iter().position(|part| *part == "ledger-demo")?;
    const PANEL: &str = "agent_ledger.panel";
    const QUIET: &str = "agent_ledger.quiet";
    const FAINT: &str = "agent_ledger.faint";
    let tray = if stock.family == "Clean" {
        "surface_raised"
    } else {
        "agent_ledger.tray"
    };
    match &parts[demo + 1..] {
        ["surface", "bar", "identity", "owner"]
        | ["surface", "bar", "summary", "all" | "undone"] => {
            Some(PaintWitness::new(QUIET, &[tray]))
        }
        ["surface", "entries", "entry:8:ledger", rest @ ..] => {
            let (entry, tail) = match rest {
                ["1:price", "定价", "body", "columns", tail @ ..] => ("price", tail),
                [
                    entry @ ("1:notes" | "1:cdn" | "1:email" | "1:quote" | "1:page"),
                    "body",
                    "columns",
                    tail @ ..,
                ] => (*entry, tail),
                _ => return None,
            };
            match tail {
                ["main", "change", "detail"] | ["main", "change", "before-box", "before"] => {
                    Some(PaintWitness::new(QUIET, &[PANEL]))
                }
                ["main", "change", "arrow"] => Some(PaintWitness::new(FAINT, &[PANEL])),
                ["end", "time-line", "time"] => Some(PaintWitness::new(
                    if entry == "1:quote" { FAINT } else { QUIET },
                    &[PANEL],
                )),
                ["main", "header-line", "words", "verb"] if entry == "1:quote" => {
                    Some(PaintWitness::new(QUIET, &[PANEL]))
                }
                ["icon", "initial"] if entry == "1:quote" => {
                    let mut witness = PaintWitness::new("agent_ledger.text", &[PANEL]);
                    witness.opacity = 0.5;
                    witness.isolated_paper = Some(IsolatedPaper {
                        token: "agent_ledger.icon",
                        underlay: None,
                        grayscale: 1.0,
                        wash: None,
                    });
                    Some(witness)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Exact original-ink roles in the maintained verification specimen.
fn reviewed_findings(parts: &[&str]) -> Option<PaintWitness> {
    if !parts.contains(&"page-agent_findings") {
        return None;
    }
    let demo = parts.iter().position(|part| *part == "findings-demo")?;
    let roles = &parts[demo + 1..];
    const PANEL: &str = "agent_findings.panel";
    const TRAY: &str = "agent_findings.tray";
    const QUIET: &str = "agent_findings.quiet";
    const FAINT: &str = "agent_findings.faint";
    match roles {
        ["surface", "bar", "checks"] | ["surface", "feed-heading", "heading"] => {
            Some(PaintWitness::new(QUIET, &[TRAY]))
        }
        ["surface", "bar", "totals", "warning", "word"]
        | ["surface", "bar", "totals", "warning", "number", "ink"] => {
            Some(PaintWitness::new("agent_findings.warning", &[TRAY]))
        }
        ["surface", "agents", "agent:14:verification", rest @ ..] => {
            let (member, tail) = match rest {
                ["1:storage", "检查", "filter", tail @ ..] => ("storage", tail),
                ["1:layout", "filter", tail @ ..] => ("layout", tail),
                ["1:schema", "filter", tail @ ..] => ("schema", tail),
                _ => return None,
            };
            match tail {
                ["avatar", "0"] => Some(PaintWitness::new(
                    "agent_findings.avatar_ink",
                    &[PANEL, "agent_findings.fill"],
                )),
                ["count"] | ["who", "step"] => Some(PaintWitness::new(QUIET, &[PANEL])),
                ["counts", "critical", "number", "ink"] if member != "storage" => {
                    Some(PaintWitness::new(FAINT, &[PANEL]))
                }
                ["counts", "warning", "number", "ink"] if member != "layout" => {
                    Some(PaintWitness::new(FAINT, &[PANEL]))
                }
                _ => None,
            }
        }
        [
            "surface",
            "feed",
            "clip",
            "entries",
            "finding:14:verification",
            rest @ ..,
        ] => {
            let tail = match rest {
                ["1:path", "边界", "body", tail @ ..]
                | ["1:cache", "older", "body", tail @ ..]
                | ["1:cutoff", "body", tail @ ..] => tail,
                _ => return None,
            };
            match tail {
                ["meta", "author" | "location"] => Some(PaintWitness::new(QUIET, &[PANEL])),
                ["time"] | ["meta", "separator"] => Some(PaintWitness::new(FAINT, &[PANEL])),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Preserve the literal low-contrast ink from the four stock Recap palettes.
/// Match the actual control or textual part and its composited backing.
fn reviewed_recap(stock: &ThemeVariant, parts: &[&str]) -> Option<PaintWitness> {
    if !parts.contains(&"page-agent_recap") {
        return None;
    }
    let clean = stock.family == "Clean";
    let demo = parts.iter().position(|part| *part == "recap-demo")?;
    let roles = &parts[demo + 1..];
    const PANEL: &str = "agent_recap.panel";
    const QUIET: &str = "agent_recap.quiet";
    match roles {
        ["surface", "panel", "head", "identity", "period", "since"]
        | [
            "surface",
            "panel",
            "head",
            "summary",
            "done" | "needs" | "failed",
            "name",
        ] => Some(PaintWitness::new(QUIET, &[PANEL])),
        ["surface", "panel", "times", "since" | "until"] => {
            Some(PaintWitness::new("agent_recap.faint", &[PANEL]))
        }
        [
            "surface",
            "board",
            "done" | "needs" | "failed",
            "lane-head",
            "label",
            "label",
        ] if clean && stock.name == "Light" => {
            Some(PaintWitness::new(QUIET, &["agent_recap.tray"]))
        }
        ["surface", "board", lane, "cards", rest @ ..] => {
            let (card, tail) = match (*lane, rest) {
                ("needs", [card @ "event:release", "签收", tail @ ..])
                | ("needs", [card @ "event:review", tail @ ..])
                | ("failed", [card @ "event:pipeline", tail @ ..])
                | ("done", [card @ ("event:indexed" | "event:report"), tail @ ..]) => (card, tail),
                _ => return None,
            };
            match tail {
                ["meta", "time"] => Some(PaintWitness::new("agent_recap.faint", &[PANEL])),
                ["meta", "detail"] => Some(PaintWitness::new(
                    if *card == "event:pipeline" {
                        "agent_recap.failed"
                    } else {
                        QUIET
                    },
                    &[PANEL],
                )),
                ["footer-pending", "footer", "receipt", "status"] if *lane == "done" => Some(
                    PaintWitness::new("agent_recap.ok", &[PANEL, "agent_recap.ok_bg"]),
                ),
                ["footer-pending", "footer", "review", "label-entry", "label"]
                    if clean && *lane == "needs" =>
                {
                    Some(PaintWitness::new(QUIET, &[PANEL]))
                }
                ["footer-pending", "footer", "action", "label-entry", "label"]
                    if clean && *lane == "done" =>
                {
                    Some(PaintWitness::new(QUIET, &[PANEL]))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn iso_date(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes().iter().enumerate().all(|(index, byte)| {
            if index == 4 || index == 7 {
                *byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
}

fn contrast(foreground: gpui_rhai::Rgba8, background: gpui_rhai::Rgba8) -> f64 {
    let [r, g, b, _] = background.as_rgba_hex().to_be_bytes();
    contrast_on(foreground, [r, g, b].map(|v| f64::from(v) / 255.0))
}

// Independent, unquantized compositing of the authored fixture layers.
fn backing(theme: &ThemeVariant, papers: &[Paper]) -> Option<[f64; 3]> {
    let mut result: Option<[f64; 3]> = None;
    for paper in papers {
        let [r, g, b, a] = theme.tokens.color(paper.token)?.as_rgba_hex().to_be_bytes();
        let front = [r, g, b].map(|v| f64::from(v) / 255.0);
        result = Some(if let Some(back) = result {
            let alpha = f64::from(a) / 255.0 * paper.opacity;
            std::array::from_fn(|i| front[i] * alpha + back[i] * (1.0 - alpha))
        } else if a == 255 && paper.opacity == 1.0 {
            front
        } else {
            return None;
        });
    }
    result
}

fn contrast_on(foreground: gpui_rhai::Rgba8, background: [f64; 3]) -> f64 {
    contrast_on_opacity(foreground, 1.0, background)
}

fn contrast_on_opacity(foreground: gpui_rhai::Rgba8, opacity: f64, background: [f64; 3]) -> f64 {
    contrast_between(rgba_over(foreground, opacity, background), background)
}

fn blend_rgb(front: [f64; 3], alpha: f64, back: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| front[i] * alpha + back[i] * (1.0 - alpha))
}

fn rgba_over(ink: gpui_rhai::Rgba8, opacity: f64, background: [f64; 3]) -> [f64; 3] {
    let [r, g, b, a] = ink.as_rgba_hex().to_be_bytes();
    blend_rgb(
        [r, g, b].map(|v| f64::from(v) / 255.0),
        f64::from(a) / 255.0 * opacity,
        background,
    )
}

fn contrast_between(painted: [f64; 3], background: [f64; 3]) -> f64 {
    let decode = |v: f64| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let mut front = 0.0;
    let mut back = 0.0;
    for (index, weight) in [0.2126, 0.7152, 0.0722].into_iter().enumerate() {
        front += weight * decode(painted[index]);
        back += weight * decode(background[index]);
    }
    (front.max(back) + 0.05) / (front.min(back) + 0.05)
}

#[test]
fn trace_muted_caption_risk_requires_the_stock_role_and_composited_paper() {
    use gpui_rhai::{AuditContrastEvidence, Rgba8};
    let mut policy = PresetRisks::new();
    let clean = policy
        .stock
        .iter()
        .find(|theme| theme.family == "Clean" && theme.name == "Dark")
        .unwrap()
        .clone();
    let prefix = "root/1/page-trace_waterfall/1/trace_waterfall-scroll/0/0/1/sized/run/";
    let mut finding = AuditFinding {
        rule: AuditRule::LowContrastText,
        node: None,
        path: String::new(),
        message: "native trace witness".to_owned(),
        contrast: Some(AuditContrastEvidence {
            foreground: Rgba8::from_rgba_hex(0xffff_ff73),
            element_opacity: 1.0,
            background_srgb: Some([28. / 255., 28. / 255., 31. / 255.]),
            ratio: 4.453_527_232_154_206,
            minimum: 4.5,
            conservative: false,
        }),
    };
    for accepted in [
        "head/task-caption",
        "card/stats/metric:4/stat-label",
        "card/axis/axis-caption/axis-label",
        "card/axis/axis-flow/axis-track/tick:2000/tick-label",
        "card/viewport/tree/span:outline/trigger/0/span-labels/span-model-caption",
    ] {
        finding.path = format!("{prefix}{accepted}");
        assert!(policy.record(&clean, &finding), "{accepted}");
    }
    for rejected in [
        "card/stats/metric:8/stat-label",
        "card/axis/axis-flow/axis-track/tick:bad/tick-label",
        "card/viewport/tree/span:outline/trigger/lane/duration/1/duration-tag/readout-label",
        "card/viewport/tree/span:outline/content/span-model-caption",
        "card/viewport/tree/span:unreviewed/trigger/0/span-labels/span-model-caption",
    ] {
        finding.path = format!("{prefix}{rejected}");
        assert!(!policy.record(&clean, &finding), "{rejected}");
    }
    finding.path = format!("{prefix}card/stats/metric:4/stat-label");
    let evidence = finding.contrast.as_mut().unwrap();
    evidence.background_srgb = Some([35. / 255., 35. / 255., 38. / 255.]);
    evidence.ratio = 4.324_258_147_718_797;
    assert!(
        !policy.record(&clean, &finding),
        "a real ratio on the wrong paper is still a regression"
    );
}

#[test]
fn trace_special_states_require_the_exact_ink_and_fixture_branch() {
    use gpui_rhai::{AuditContrastEvidence, Rgba8};
    let mut policy = PresetRisks::new();
    let prefix = "root/page-trace_waterfall/trace_waterfall-scroll/0/sized/run/";
    for (family, path, ink, paper, ratio) in [
        (
            "Clean",
            "head/head-status/chip-text",
            0x006d_cbf2,
            [
                0.901_960_784_313_725_5,
                0.956_939_638_600_538_2,
                0.996_155_324_875_048_1,
            ],
            4.263_244_447_638_788_5,
        ),
        (
            "Clean",
            "card/stats/metric:5/stat-value",
            0xd724_00da,
            [1.; 3],
            4.271_380_109_309_696,
        ),
        (
            "Clean",
            "card/viewport/tree/span:archive/trigger/lane/duration/duration-tag/readout-label",
            0xd724_00da,
            [1.; 3],
            4.271_380_109_309_696,
        ),
        (
            "Modern",
            "card/viewport/tree/span:reconcile/trigger/lane/duration/duration-tag/slow-badge/chip-text",
            0xb25e_00ff,
            [1., 241. / 255., 224. / 255.],
            4.207_867_989_670_515,
        ),
        (
            "Clean",
            "card/viewport/tree/span:reconcile/trigger/lane/duration/duration-tag/slow-badge/chip-text",
            0xb25e_00ff,
            [1., 0.937_577_854_671_280_3, 0.839_215_686_274_509_8],
            4.134_164_941_982_281,
        ),
    ] {
        let stock = policy
            .stock
            .iter()
            .find(|theme| theme.family == family && theme.name == "Light")
            .unwrap()
            .clone();
        let mut finding = AuditFinding {
            rule: AuditRule::LowContrastText,
            node: None,
            path: format!("{prefix}{path}"),
            message: "authored fixture state".into(),
            contrast: Some(AuditContrastEvidence {
                foreground: Rgba8::from_rgba_hex(ink),
                element_opacity: 1.,
                background_srgb: Some(paper),
                ratio,
                minimum: 4.5,
                conservative: false,
            }),
        };
        assert!(policy.record(&stock, &finding), "{family}: {path}");
        finding.contrast.as_mut().unwrap().element_opacity = 0.9;
        assert!(!policy.record(&stock, &finding));
        finding.contrast.as_mut().unwrap().element_opacity = 1.;
        finding.path = format!(
            "{prefix}card/viewport/tree/span:outline/trigger/lane/duration/duration-tag/readout-label"
        );
        assert!(
            !policy.record(&stock, &finding),
            "ordinary readouts must remain eligible"
        );
    }
}

#[test]
fn ledger_risk_matches_the_selected_profile_and_rejects_the_other_paint_model() {
    use gpui_rhai::{AuditContrastEvidence, Rgba8};
    let mut policy = PresetRisks::new();
    // Browser-resolved inks and paper, projected independently from the
    // fixture policy and the SDK audit. Alpha is encoded in nearest RGBA8.
    for (family, appearance, ink, vendor_paper, vendor_ratio) in [
        (
            "Modern",
            "Light",
            0x6b6b_6bff,
            [1.; 3],
            2.050_472_879_486_149_3,
        ),
        (
            "Modern",
            "Dark",
            0xa1a1_a1ff,
            [
                0.151_546_666_666_666_66,
                0.151_546_666_666_666_66,
                0.157_429_019_607_843_14,
            ],
            2.460_960_293_058_108,
        ),
        (
            "Clean",
            "Light",
            0x0000_009c,
            [1.; 3],
            2.144_357_432_290_223_5,
        ),
        (
            "Clean",
            "Dark",
            0xffff_ff9e,
            [
                0.123_954_117_647_058_83,
                0.123_954_117_647_058_83,
                0.129_836_470_588_235_3,
            ],
            2.754_367_190_772_315,
        ),
    ] {
        let stock = policy
            .stock
            .iter()
            .find(|theme| theme.family == family && theme.name == appearance)
            .unwrap()
            .clone();
        let released = match (family, appearance) {
            ("Modern", "Dark") => (
                [38.5 / 255., 38.5 / 255., 42. / 255.],
                2.527_530_361_627_965_4,
            ),
            ("Clean", "Dark") => (
                [31.5 / 255., 31.5 / 255., 34.5 / 255.],
                2.799_649_252_492_378_6,
            ),
            _ => (vendor_paper, vendor_ratio),
        };
        let vendor = (vendor_paper, vendor_ratio);
        let ((paper, ratio), other) = if gpui_rhai::EXTENDED_RENDERING {
            (vendor, released)
        } else {
            (released, vendor)
        };
        let mut finding = AuditFinding {
            rule: AuditRule::LowContrastText,
            node: None,
            path: "root/page-agent_ledger/ledger-demo/surface/entries/entry:8:ledger/1:quote/body/columns/icon/initial".into(),
            message: "browser-resolved, retained risk".into(),
            contrast: Some(AuditContrastEvidence {
                foreground: Rgba8::from_rgba_hex(ink),
                element_opacity: 0.5,
                background_srgb: Some(paper),
                ratio,
                minimum: 4.5,
                conservative: false,
            }),
        };
        assert!(policy.record(&stock, &finding), "{family}/{appearance}");
        let mut not_the_mark = finding.clone();
        not_the_mark.path = not_the_mark.path.replace("1:quote", "1:page");
        assert!(!policy.record(&stock, &not_the_mark));
        if appearance == "Dark" {
            let evidence = finding.contrast.as_mut().unwrap();
            evidence.background_srgb = Some(other.0);
            evidence.ratio = other.1;
            assert!(
                !policy.record(&stock, &finding),
                "a witness from the other painter is not admissible"
            );
            finding.contrast.as_mut().unwrap().ratio = ratio;
            finding.contrast.as_mut().unwrap().background_srgb = Some(paper);
        }
        let mut changed = stock.clone();
        std::sync::Arc::make_mut(&mut changed.tokens).colors.insert(
            "agent_ledger.text".to_owned(),
            Rgba8::from_rgba_hex(0x7777_77ff),
        );
        assert!(!policy.record(&changed, &finding));
    }
    policy.assert_reported();
}

#[test]
fn palette_risk_policy_rejects_wrong_roles_opacity_colors_and_non_stock_themes() {
    let mut policy = PresetRisks::new();
    let stock = policy
        .stock
        .iter()
        .find(|theme| theme.family == "Modern" && theme.name == "Dark")
        .unwrap()
        .clone();
    let mut finding = AuditFinding {
        rule: AuditRule::LowContrastText,
        node: None,
        path: "root/1/1/shared-combobox/trigger/shared-combobox-trigger-value".to_owned(),
        message: "measured".to_owned(),
        contrast: Some(gpui_rhai::AuditContrastEvidence {
            foreground: gpui_rhai::Rgba8::from_rgba_hex(0x5c5c_63ff),
            element_opacity: 1.0,
            background_srgb: Some([25.0 / 255.0, 25.0 / 255.0, 27.0 / 255.0]),
            ratio: contrast(
                gpui_rhai::Rgba8::from_rgba_hex(0x5c5c_63ff),
                gpui_rhai::Rgba8::from_rgba_hex(0x1919_1bff),
            ),
            minimum: 4.5,
            conservative: false,
        }),
    };
    assert!(policy.record(&stock, &finding));
    finding.path = "root/body".to_owned();
    assert!(!policy.record(&stock, &finding));
    finding.path = "root/page-calendar/calendar/1/2026-10-01/1/week-2/2026-10-11/0".to_owned();
    assert!(!policy.record(&stock, &finding));
    finding.path = "root/page-calendar/calendar/1/2026-10-01/1/week-5/2026-11-01/0".to_owned();
    assert!(
        !policy.record(&stock, &finding),
        "an implementation using placeholder ink on an adjacent-month date is not an authored risk"
    );
    finding.path = "root/1/1/shared-combobox/trigger/shared-combobox-trigger-value".to_owned();
    assert!(policy.record(&stock, &finding));
    finding.contrast.as_mut().unwrap().element_opacity = 0.5;
    assert!(!policy.record(&stock, &finding));
    finding.contrast.as_mut().unwrap().element_opacity = 1.0;
    finding.contrast.as_mut().unwrap().conservative = true;
    assert!(!policy.record(&stock, &finding));
    finding.contrast.as_mut().unwrap().conservative = false;
    let mut changed = stock.clone();
    std::sync::Arc::make_mut(&mut changed.tokens).colors.insert(
        "text_primary".to_owned(),
        gpui_rhai::Rgba8::from_rgba_hex(0),
    );
    assert!(!policy.record(&changed, &finding));
    finding.contrast.as_mut().unwrap().foreground = gpui_rhai::Rgba8::from_rgba_hex(0x7777_77ff);
    assert!(!policy.record(&stock, &finding));

    let light = policy
        .stock
        .iter()
        .find(|theme| theme.family == "Modern" && theme.name == "Light")
        .unwrap()
        .clone();
    for (path, foreground, background) in [
        (
            "root/page-calendar/calendar.date/1/2026-10-01/0/3/0",
            "text.subtle",
            "surface_raised",
        ),
        (
            "root/0/2/0/gallery-density-compact/0/0/0",
            "tabs.foreground",
            "tabs.tray",
        ),
    ] {
        let ink = light.tokens.color(foreground).unwrap();
        let paper = light.tokens.color(background).unwrap();
        let [r, g, b, _] = paper.as_rgba_hex().to_be_bytes();
        finding.path = path.to_owned();
        finding.contrast = Some(gpui_rhai::AuditContrastEvidence {
            foreground: ink,
            element_opacity: 1.0,
            background_srgb: Some([r, g, b].map(|v| f64::from(v) / 255.0)),
            ratio: contrast(ink, paper),
            minimum: 4.5,
            conservative: false,
        });
        assert!(policy.record(&light, &finding), "{path}");
        finding.path = path
            .replace("page-calendar", "page-custom")
            .replace("gallery-density-compact", "my-segment");
        assert!(
            !policy.record(&light, &finding),
            "not the fixed stock fixture"
        );
    }
}

#[test]
fn settings_and_command_risks_require_exact_native_witnesses_not_a_page_waiver() {
    use gpui_rhai::{AuditContrastEvidence, Rgba8};
    let mut policy = PresetRisks::new();
    // Independent values captured from actual stock frames. Do not derive
    // these from reviewed_detail or its compositing helper.
    for (family, appearance, path, ink, paper, opacity, ratio) in [
        (
            "Modern",
            "Light",
            "root/page-switch/1/switch-scroll/0/0/1/0/text/description",
            0x8f8f_8fff,
            [1.0; 3],
            1.0,
            3.233_980_204_335_057_6,
        ),
        (
            "Clean",
            "Light",
            "root/page-switch/1/switch-scroll/0/0/1/1/text/description",
            0x0000_0073,
            [1.0; 3],
            1.0,
            3.362_683_071_052_112,
        ),
        (
            "Modern",
            "Light",
            "root/page-command/1/command-scroll/0/1/command-embedded/card/list/viewport/command-embedded-list/item/group:Release/0",
            0xb5b5_b5ff,
            [1.0; 3],
            1.0,
            2.050_472_879_486_149_3,
        ),
        (
            "Modern",
            "Dark",
            "root/page-command/1/command-scroll/0/1/command-embedded/card/list/viewport/command-embedded-list/item/item:logs/shortcut/0/0",
            0x8b8b_92ff,
            [
                0.131_372_549_019_607_85,
                0.131_372_549_019_607_85,
                0.142_549_019_607_843_1,
            ],
            0.85,
            3.806_226_018_486_605,
        ),
        (
            "Clean",
            "Dark",
            "root/page-command/1/command-scroll/0/3/1/command-host/card/search/enter-hint/1",
            0xffff_ff4d,
            [27.0 / 255.0, 27.0 / 255.0, 30.0 / 255.0],
            1.0,
            2.732_474_419_031_887_3,
        ),
        (
            "Clean",
            "Dark",
            "root/page-command/1/command-scroll/0/1/command-embedded/card/list/viewport/command-embedded-list/item/item:deploy/text/hint",
            0xffff_ff75,
            [
                0.179_623_221_837_754_7,
                0.179_623_221_837_754_7,
                0.186_851_211_072_664_36,
            ],
            1.0,
            4.141_050_626_545_002,
        ),
    ] {
        let stock = policy
            .stock
            .iter()
            .find(|theme| theme.family == family && theme.name == appearance)
            .unwrap()
            .clone();
        let finding = AuditFinding {
            rule: AuditRule::LowContrastText,
            node: None,
            path: path.into(),
            message: "reviewed native witness".into(),
            contrast: Some(AuditContrastEvidence {
                foreground: Rgba8::from_rgba_hex(ink),
                element_opacity: opacity,
                background_srgb: Some(paper),
                ratio,
                minimum: 4.5,
                conservative: false,
            }),
        };
        assert!(
            policy.record(&stock, &finding),
            "{family}/{appearance}/{path}"
        );
        for mutation in 0..7 {
            let mut changed = finding.clone();
            let evidence = changed.contrast.as_mut().unwrap();
            match mutation {
                0 => {
                    changed.path = path
                        .replace("page-switch", "page-custom")
                        .replace("page-command", "page-custom")
                }
                1 => changed.path = format!("{path}/unreviewed-child"),
                2 => evidence.foreground = Rgba8::from_rgba_hex(ink ^ 0x0000_0100),
                3 => evidence.background_srgb.as_mut().unwrap()[0] -= 1.0 / 255.0,
                4 => evidence.element_opacity = if opacity == 1.0 { 0.85 } else { 1.0 },
                5 => evidence.ratio += 0.01,
                6 => evidence.conservative = true,
                _ => unreachable!(),
            }
            assert!(
                !policy.record(&stock, &changed),
                "mutation {mutation}: {path}"
            );
        }
        let mut changed = stock.clone();
        std::sync::Arc::make_mut(&mut changed.tokens)
            .colors
            .insert("text_primary".into(), Rgba8::from_rgb_hex(0xff00ff));
        assert!(
            !policy.record(&changed, &finding),
            "application overrides are still audited"
        );
    }
}

#[test]
fn source_alpha_badges_are_reported_without_waiving_derived_buttons_or_other_backings() {
    let mut policy = PresetRisks::new();
    let stock = policy
        .stock
        .iter()
        .find(|theme| theme.family == "Clean" && theme.name == "Light")
        .unwrap()
        .clone();
    // Independent numeric witness from the native fixture, not an opaque RGB
    // comparison of the original translucent green pair.
    let mut finding = AuditFinding {
        rule: AuditRule::LowContrastText,
        node: None,
        path: "root/page-badge/0/label".to_owned(),
        message: "source alpha".to_owned(),
        contrast: Some(gpui_rhai::AuditContrastEvidence {
            foreground: gpui_rhai::Rgba8::from_rgba_hex(0x007d2ee0),
            element_opacity: 1.0,
            background_srgb: Some([0.9019607843137255, 0.9650134563629373, 0.9215686274509804]),
            ratio: 3.8513596131651107,
            minimum: 4.5,
            conservative: false,
        }),
    };
    assert!(policy.record(&stock, &finding));
    finding.path = "root/page-button/0/label-entry/label".to_owned();
    assert!(
        !policy.record(&stock, &finding),
        "derived status actions need their own readable ink"
    );
    finding.path = "root/page-badge/0/label".to_owned();
    finding
        .contrast
        .as_mut()
        .unwrap()
        .background_srgb
        .as_mut()
        .unwrap()[0] -= 1.0 / 255.0;
    assert!(
        !policy.record(&stock, &finding),
        "an unreviewed backing is not the same source-alpha witness"
    );
}

#[test]
fn original_table_header_risk_is_bound_to_the_exact_stock_fixture_and_card() {
    let mut policy = PresetRisks::new();
    let stock = policy
        .stock
        .iter()
        .find(|theme| theme.family == "Modern" && theme.name == "Light")
        .unwrap()
        .clone();
    let ink = stock.tokens.color("table.header_ink").unwrap();
    let paper = stock.tokens.color("table.surface").unwrap();
    let mut finding = AuditFinding {
        rule: AuditRule::LowContrastText,
        node: None,
        path: "root/page-table/0/table-hosts/table-hosts-header/host/action/0".into(),
        message: "authored table header".into(),
        contrast: Some(gpui_rhai::AuditContrastEvidence {
            foreground: ink,
            element_opacity: 1.0,
            background_srgb: backing(
                &stock,
                &[Paper {
                    token: "table.surface",
                    opacity: 1.0,
                }],
            ),
            ratio: contrast(ink, paper),
            minimum: 4.5,
            conservative: false,
        }),
    };
    assert!(policy.record(&stock, &finding));
    for path in [
        "root/page-table/0/table-hosts/host/action/0",
        "root/page-custom/0/table-hosts/table-hosts-header/host/action/0",
        "root/page-table/0/table-hosts/table-hosts-header/custom/action/0",
        "root/page-table/0/table-hosts/table-hosts-header/host/action/body",
        "root/page-table/0/table-hosts/table-hosts-header/host/action/0/custom/action/0",
    ] {
        finding.path = path.into();
        assert!(
            !policy.record(&stock, &finding),
            "not the authored header: {path}"
        );
    }
    finding.path = "root/page-table/0/table-hosts/table-hosts-header/host/action/0".into();
    finding.contrast.as_mut().unwrap().element_opacity = 0.85;
    assert!(
        !policy.record(&stock, &finding),
        "extra fading is not the original header recipe"
    );
}

#[test]
fn away_summary_risk_separates_authored_tray_from_native_row_paper() {
    let mut policy = PresetRisks::new();
    let stock = policy
        .stock
        .iter()
        .find(|theme| theme.family == "Modern" && theme.name == "Dark")
        .unwrap()
        .clone();
    let ink = gpui_rhai::Rgba8::from_rgba_hex(0x7c7c82ff);
    let prefix = "root/page-away_summary/1/away_summary-scroll/0/0/away-demo/width/summary/body";
    let make = |suffix: &str, paper: [f64; 3]| AuditFinding {
        rule: AuditRule::LowContrastText,
        node: None,
        path: format!("{prefix}/{suffix}"),
        message: "AwaySummary quiet ink".into(),
        contrast: Some(gpui_rhai::AuditContrastEvidence {
            foreground: ink,
            element_opacity: 1.0,
            background_srgb: Some(paper),
            ratio: contrast_on_opacity(ink, 1.0, paper),
            minimum: 4.5,
            conservative: false,
        }),
    };
    let tray = [25., 25., 28.].map(|v| v / 255.);
    let row = [35., 35., 38.].map(|v| v / 255.);
    let head = make("header/counts/failed/1", tray);
    let item = make("items/item:pipeline/body/detail", row);
    assert!(policy.record(&stock, &head));
    assert!(policy.record(&stock, &item));
    for other in [
        make("header/counts/failed/1", row),
        make("items/item:pipeline/body/detail", tray),
        make("items/item:pipeline/body/name", row),
        make("items/item:foreign/body/detail", row),
        make("footer/disclosure/label-entry/label", tray),
        make("header/counts/failed/1/extra", tray),
    ] {
        assert!(!policy.record(&stock, &other), "{other:#?}");
    }
    let mut faded = head;
    let evidence = faded.contrast.as_mut().unwrap();
    evidence.element_opacity = 0.8;
    evidence.ratio = contrast_on_opacity(ink, 0.8, tray);
    assert!(!policy.record(&stock, &faded));
}

#[test]
fn scheduled_runs_risk_matches_the_pill_wash_to_the_painter() {
    let mut policy = PresetRisks::new();
    let stock = policy
        .stock
        .iter()
        .find(|theme| theme.family == "Modern" && theme.name == "Light")
        .unwrap()
        .clone();
    let base = [1.; 3];
    let inner = [1. - 15. / 255.; 3];
    let ink = gpui_rhai::Rgba8::from_rgba_hex(0x6b6b6bff);
    let final_paper = blend_rgb(inner, 0.45, base);
    let isolated = blend_rgb([107. / 255.; 3], 0.45, base);
    let per_draw = blend_rgb([107. / 255.; 3], 0.45, final_paper);
    let (expected_ink, wrong_ink) = if gpui_rhai::EXTENDED_RENDERING {
        (isolated, per_draw)
    } else {
        (per_draw, isolated)
    };
    let mut finding = AuditFinding {
        rule: AuditRule::LowContrastText,
        node: None,
        path: "root/page-scheduled_runs/1/scheduled_runs-scroll/0/0/1/0/plan/panel/calendar/columns/am/slot:3:monam/drop:3:monam-content/cards/part:last-run/detail/trigger/stats/usage-pill/percentage".into(),
        message: "booked history percentage".into(),
        contrast: Some(gpui_rhai::AuditContrastEvidence {
            foreground: ink,
            element_opacity: 0.45,
            background_srgb: Some(final_paper),
            ratio: contrast_between(expected_ink, final_paper),
            minimum: 4.5,
            conservative: false,
        }),
    };
    assert!(policy.record(&stock, &finding));
    for changed in [
        finding.path.replace("part:last-run", "part:future"),
        finding
            .path
            .replace("usage-pill/percentage", "0/percentage"),
    ] {
        let mut other = finding.clone();
        other.path = changed;
        assert!(!policy.record(&stock, &other));
    }
    let evidence = finding.contrast.as_mut().unwrap();
    evidence.ratio = contrast_between(wrong_ink, final_paper);
    assert!(
        !policy.record(&stock, &finding),
        "the evidence must describe the selected paint model"
    );
}

#[test]
fn email_draft_risk_requires_the_authored_paper_and_the_manifest_fixture_role() {
    let mut policy = PresetRisks::new();
    let stock = policy
        .stock
        .iter()
        .find(|theme| theme.family == "Modern" && theme.name == "Light")
        .unwrap()
        .clone();
    let ink = gpui_rhai::Rgba8::from_rgba_hex(0x8f8f8fff);
    let prefix = "root/page-email_draft/1/email_draft-scroll/0/0/1/sized/email";
    let make = |path: &str, paper: f64| {
        let paper = [paper; 3];
        AuditFinding {
            rule: AuditRule::LowContrastText,
            node: None,
            path: format!("{prefix}/{path}"),
            message: "EmailDraft backing positive control".into(),
            contrast: Some(gpui_rhai::AuditContrastEvidence {
                foreground: ink,
                element_opacity: 1.0,
                background_srgb: Some(paper),
                ratio: contrast_on_opacity(ink, 1.0, paper),
                minimum: 4.5,
                conservative: false,
            }),
        }
    };
    let file = "switcher/draft-grid/meta/file/1";
    let address = "switcher/draft-grid/meta/to/recipient:renewal:0/trigger/1";
    let tone = "switcher/draft-grid/actions/tones/tone:short/0";
    assert!(policy.record(&stock, &make(file, 1.0)));
    assert!(policy.record(&stock, &make(address, 245. / 255.)));
    assert!(policy.record(&stock, &make(tone, 1.0)));
    let mut unknown_paper = make(tone, 1.0);
    let uncertain = unknown_paper.contrast.as_mut().unwrap();
    uncertain.background_srgb = None;
    uncertain.conservative = true;
    assert!(!policy.record(&stock, &unknown_paper));
    for invalid in [
        make(file, 245. / 255.),
        make(address, 1.0),
        make(tone, 240. / 255.),
        make("switcher/draft-grid/actions/tones/tone:custom/0", 1.0),
        make(
            "switcher/draft-grid/meta/to/recipient:other:0/trigger/1",
            245. / 255.,
        ),
        make("switcher/draft-grid/meta/file/1/unknown", 1.0),
    ] {
        assert!(!policy.record(&stock, &invalid), "{invalid:#?}");
    }
}

#[test]
fn context_map_risk_requires_the_exact_tile_role_ink_and_composited_panel() {
    let mut policy = PresetRisks::new();
    let stock = policy
        .stock
        .iter()
        .find(|theme| theme.family == "Clean" && theme.name == "Light")
        .unwrap()
        .clone();
    let ink = gpui_rhai::Rgba8::from_rgba_hex(0x0070d6ff);
    // 8% file tint quantized to RGBA8 on the known opaque white map panel.
    let tint = 20. / 255.;
    let paper = [0., 96. / 255., 226. / 255.].map(|v| v * tint + 1. - tint);
    let tile = "root/page-context_map/1/context_map-scroll/0/0/1/sized-map/context-map/field/block:15:gallery/context:6:writer:files/资料/0/";
    let mut finding = AuditFinding {
        rule: AuditRule::LowContrastText,
        node: None,
        path: format!("{tile}counts/detail/0"),
        message: "source-alpha file caption".into(),
        contrast: Some(gpui_rhai::AuditContrastEvidence {
            foreground: ink,
            element_opacity: 0.7,
            background_srgb: Some(paper),
            ratio: contrast_on_opacity(ink, 0.7, paper),
            minimum: 4.5,
            conservative: false,
        }),
    };
    assert!(policy.record(&stock, &finding));
    for invalid in [
        format!("{tile}title/0"), // same ink but not the title's authored opacity
        format!("{tile}counts/detail/0/trailing"),
        finding.path.replace("page-context_map", "page-other"),
        finding.path.replace("writer:files", "reviewer:files"),
        finding.path.replace("files/资料", "files/other"),
        finding.path.replace(
            "block:15:gallery/context:6:writer:files/资料",
            "free:15:gallery/context:6:writer:",
        ),
    ] {
        let mut other = finding.clone();
        other.path = invalid;
        assert!(!policy.record(&stock, &other), "{}", other.path);
    }
    let evidence = finding.contrast.as_mut().unwrap();
    let other_paper = [0.8; 3];
    evidence.background_srgb = Some(other_paper);
    evidence.ratio = contrast_on_opacity(ink, 0.7, other_paper);
    assert!(
        !policy.record(&stock, &finding),
        "a different panel stays an open finding"
    );
}
