//! Exact line overlap for display and Host proposals. Never edits or merges files.
use std::collections::{BTreeMap, BTreeSet};

use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Position};
use serde::{Deserialize, Serialize};

type Error = EvalAltResult;
const MAX_LINE: u32 = 100_000_000;
const MAX_FILES: usize = 64;
const MAX_EDITS: usize = 256;
const MAX_FILE_EDITS: usize = 24;
const MAX_AGENTS: usize = 16;

#[derive(Deserialize)]
struct Agent {
    id: String,
    status: String,
}

#[derive(Deserialize)]
struct File {
    id: String,
    path: String,
    edits: Vec<Edit>,
}

#[derive(Deserialize)]
struct Edit {
    id: String,
    agent: String,
    start: u32,
    end: u32,
    revision: u32,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    active: bool,
}

#[derive(Deserialize)]
struct Receipt {
    agent: String,
    witness: String,
}

#[derive(Serialize)]
struct Way<'a> {
    agent: &'a str,
    summaries: Vec<&'a str>,
}

#[derive(Serialize)]
struct Band<'a> {
    id: String,
    witness: String,
    start: u32,
    end: u32,
    options: Vec<Way<'a>>,
    accepted: Option<&'a str>,
}

#[derive(Default, Serialize)]
struct Stats {
    files: usize,
    conflicts: usize,
    kept: usize,
}

#[derive(Default, Serialize)]
struct Cell {
    role: &'static str,
    active: bool,
    lines: Vec<[u32; 2]>,
}

#[derive(Serialize)]
struct FileRow<'a> {
    id: &'a str,
    path: &'a str,
    bands: Vec<Band<'a>>,
    cells: Vec<Cell>,
    open: usize,
}

#[derive(Serialize)]
struct Radar<'a> {
    files: Vec<FileRow<'a>>,
    stats: Vec<Stats>,
    open: usize,
    running: usize,
    branches: usize,
    ready: bool,
}

fn error(message: &'static str) -> Error {
    EvalAltResult::ErrorRuntime(message.into(), Position::NONE)
}

fn decode<T: serde::de::DeserializeOwned>(value: &Dynamic) -> Result<T, Error> {
    rhai::serde::from_dynamic(value).map_err(|_| error("Invalid ConflictRadar snapshot"))
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 96 && !value.chars().any(char::is_control)
}

fn agent_ids(agents: &[Agent]) -> Result<BTreeMap<&str, usize>, Error> {
    let mut ids = BTreeMap::new();
    for (i, agent) in agents.iter().enumerate() {
        if !valid_id(&agent.id)
            || !matches!(agent.status.as_str(), "running" | "done")
            || ids.insert(agent.id.as_str(), i).is_some()
        {
            return Err(error("Invalid or duplicate ConflictRadar agent"));
        }
    }
    Ok(ids)
}

fn validate_files(files: &[File], agents: &BTreeMap<&str, usize>) -> Result<(), Error> {
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut total = 0;
    for file in files {
        total += file.edits.len();
        if total > MAX_EDITS
            || file.edits.len() > MAX_FILE_EDITS
            || !valid_id(&file.id)
            || !ids.insert(&file.id)
            || file.path.trim().is_empty()
            || file.path.len() > 2_048
            || !paths.insert(&file.path)
        {
            return Err(error("Invalid, duplicate or oversized ConflictRadar file"));
        }
        let mut edits = BTreeSet::new();
        for edit in &file.edits {
            if !valid_id(&edit.id)
                || !edits.insert(&edit.id)
                || !agents.contains_key(edit.agent.as_str())
                || edit.start == 0
                || edit.start > edit.end
                || edit.end > MAX_LINE
                || edit.summary.as_ref().is_some_and(|text| text.len() > 2048)
            {
                return Err(error(
                    "Invalid ConflictRadar edit identity, agent or line range",
                ));
            }
        }
    }
    Ok(())
}

fn band_witness(file: &File, active: &[&Edit]) -> String {
    // Canonically delimit raw keys by byte length; no lossy concatenation or hash.
    use std::fmt::Write as _;
    let mut witness = format!("{}:{};", file.path.len(), file.path);
    for edit in active {
        write!(
            witness,
            "{}:{};{}:{};{};{};{};",
            edit.id.len(),
            edit.id,
            edit.agent.len(),
            edit.agent,
            edit.revision,
            edit.start,
            edit.end
        )
        .expect("String writes cannot fail");
    }
    witness
}

fn make_band<'a>(
    file: &'a File,
    start: u32,
    end: u32,
    active: &mut [&'a Edit],
    receipts: &'a BTreeMap<String, Receipt>,
) -> Option<Band<'a>> {
    let mut contributors = BTreeMap::<&str, Vec<&str>>::new();
    active.sort_unstable_by_key(|edit| edit.id.as_str());
    for edit in active.iter() {
        let summaries = contributors.entry(edit.agent.as_str()).or_default();
        if let Some(text) = edit.summary.as_deref().filter(|text| !text.is_empty())
            && !summaries.contains(&text)
        {
            summaries.push(text);
        }
    }
    if contributors.len() < 2 {
        return None;
    }
    let id = format!("{}:{}:{start}:{end}", file.id.len(), file.id);
    let witness = band_witness(file, active);
    let accepted = receipts
        .get(&id)
        .filter(|receipt| {
            receipt.witness == witness && contributors.contains_key(receipt.agent.as_str())
        })
        .map(|receipt| receipt.agent.as_str());
    let options = contributors
        .into_iter()
        .map(|(agent, summaries)| Way { agent, summaries })
        .collect();
    Some(Band {
        id,
        witness,
        start,
        end,
        options,
        accepted,
    })
}

fn line_bands<'a>(file: &'a File, receipts: &'a BTreeMap<String, Receipt>) -> Vec<Band<'a>> {
    let edges: BTreeSet<_> = file
        .edits
        .iter()
        .flat_map(|edit| [edit.start, edit.end + 1])
        .collect();
    let edges: Vec<_> = edges.into_iter().collect();
    edges
        .windows(2)
        .filter_map(|pair| {
            let start = pair[0];
            let end = pair[1] - 1;
            let mut active: Vec<_> = file
                .edits
                .iter()
                .filter(|edit| edit.start <= start && edit.end >= end)
                .collect();
            make_band(file, start, end, &mut active, receipts)
        })
        .collect()
}

fn cells(file: &File, bands: &[Band<'_>], agents: &[Agent], stats: &mut [Stats]) -> Vec<Cell> {
    agents
        .iter()
        .zip(stats)
        .map(|(agent, stats)| {
            let mut result = Cell {
                role: "none",
                ..Cell::default()
            };
            for edit in file.edits.iter().filter(|edit| edit.agent == agent.id) {
                result.role = "edit";
                result.active |= edit.active;
                result.lines.push([edit.start, edit.end]);
            }
            if result.lines.is_empty() {
                return result;
            }
            stats.files += 1;
            let (mut open, mut kept, mut left) = (0, 0, 0);
            for band in bands
                .iter()
                .filter(|b| b.options.iter().any(|way| way.agent == agent.id))
            {
                match band.accepted {
                    Some(id) if id == agent.id => kept += 1,
                    Some(_) => left += 1,
                    None => open += 1,
                }
            }
            stats.conflicts += open;
            stats.kept += kept;
            result.role = match (open, kept, left) {
                (1.., _, _) => "conflict",
                (_, 1.., 1..) => "mixed",
                (_, 1.., _) => "kept",
                (_, _, 1..) => "lost",
                _ => "edit",
            };
            result
        })
        .collect()
}

fn project<'a>(
    agents: &[Agent],
    files: &'a [File],
    receipts: &'a BTreeMap<String, Receipt>,
) -> Result<Radar<'a>, Error> {
    validate_files(files, &agent_ids(agents)?)?;
    let mut stats: Vec<_> = agents.iter().map(|_| Stats::default()).collect();
    let files: Vec<_> = files
        .iter()
        .map(|file| {
            let bands = line_bands(file, receipts);
            let cells = cells(file, &bands, agents, &mut stats);
            let open = bands.iter().filter(|band| band.accepted.is_none()).count();
            FileRow {
                id: &file.id,
                path: &file.path,
                bands,
                cells,
                open,
            }
        })
        .collect();
    let open = files.iter().map(|row| row.open).sum();
    let running = agents.iter().filter(|agent| agent.status != "done").count();
    let branches = stats.iter().filter(|stat| stat.files > 0).count();
    Ok(Radar {
        files,
        stats,
        open,
        running,
        branches,
        ready: open == 0 && running == 0 && branches > 0,
    })
}

fn from_script(agents: Array, files: Array, receipts: Map) -> Result<Dynamic, Box<Error>> {
    if agents.len() > MAX_AGENTS || files.len() > MAX_FILES || receipts.len() > 2 * MAX_EDITS {
        return Err(error("ConflictRadar snapshot exceeds presentation limits").into());
    }
    // Check nested cardinality before deserializing potentially large arrays.
    let mut edits = 0;
    for raw in &files {
        let file = raw
            .read_lock::<Map>()
            .ok_or_else(|| error("Invalid ConflictRadar file"))?;
        let list = file
            .get("edits")
            .and_then(Dynamic::read_lock::<Array>)
            .ok_or_else(|| error("Invalid ConflictRadar edits"))?;
        edits += list.len();
        if list.len() > MAX_FILE_EDITS || edits > MAX_EDITS {
            return Err(error("ConflictRadar edit limit exceeded").into());
        }
    }
    let agents: Vec<Agent> = decode(&Dynamic::from_array(agents))?;
    let files: Vec<File> = decode(&Dynamic::from_array(files))?;
    let receipts: BTreeMap<String, Receipt> = decode(&Dynamic::from_map(receipts))?;
    rhai::serde::to_dynamic(project(&agents, &files, &receipts)?)
        .map_err(|_| error("Could not prepare ConflictRadar presentation").into())
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("conflict_radar_project", from_script);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evaluate(source: &str) -> bool {
        let mut engine = Engine::new();
        register(&mut engine);
        engine.eval(source).unwrap()
    }

    #[test]
    fn conflict_radar_uses_exact_inclusive_bands_instead_of_an_envelope() {
        assert!(evaluate(
            r#"
            let agents=[#{id:"a",status:"done"},#{id:"b",status:"done"},#{id:"c",status:"done"}];
            let files=[#{id:"src:7",path:"src/app.rs",edits:[
                #{id:"a1",agent:"a",start:1,end:20,revision:0},
                #{id:"b1",agent:"b",start:10,end:10,revision:1},
                #{id:"a2",agent:"a",start:21,end:50,revision:0},
                #{id:"c1",agent:"c",start:32,end:34,revision:2}
            ]}];
            let p=conflict_radar_project(agents,files,#{});
            p.open==2 && !p.ready && p.files[0].bands[0].start==10 && p.files[0].bands[0].end==10
                && p.files[0].bands[1].start==32 && p.files[0].bands[1].end==34
                && p.files[0].bands[0].options.len==2 && p.files[0].bands[1].options.len==2
        "#
        ));
    }

    #[test]
    fn conflict_radar_canonical_receipt_is_order_independent_but_not_revision_blind() {
        assert!(evaluate(
            r#"
            let agents=[#{id:"a",status:"done"},#{id:"b",status:"done"}];
            let a=#{id:"edit:a",agent:"a",start:1,end:12,revision:0};
            let b=#{id:"edit:b",agent:"b",start:10,end:20,revision:0};
            let f=#{id:"f",path:"src/a.rs",edits:[a,b]};
            let cluster=conflict_radar_project(agents,[f],#{}).files[0].bands[0];
            let accepted=#{};accepted[cluster.id]=#{agent:"a",witness:cluster.witness};
            f.edits=[b,a];
            let good=conflict_radar_project(agents,[f],accepted);
            b.revision=1;f.edits=[b,a];
            let changed=conflict_radar_project(agents,[f],accepted);
            good.ready && good.files[0].cells[0].role=="kept" && !changed.ready && changed.open==1
        "#
        ));
    }

    #[test]
    fn conflict_radar_never_enables_empty_or_running_merges() {
        assert!(evaluate(
            r#"
            let agents=[#{id:"a",status:"running"}];
            let entries=[#{id:"f",path:"readme",edits:[#{id:"a",agent:"a",start:3,end:4,revision:0}]}];
            let live=conflict_radar_project(agents,entries,#{});
            agents[0].status="done";
            let empty=conflict_radar_project(agents,[],#{});
            let ready=conflict_radar_project(agents,entries,#{});
            !live.ready && live.open==0 && !empty.ready && ready.ready && ready.branches==1
        "#
        ));
    }
}
