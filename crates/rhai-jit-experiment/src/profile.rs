use std::collections::{BTreeMap, HashMap};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use rhai::grain::{
    AcceleratedCall, FunctionAccelerator, FunctionCall, FunctionCallOutcome, Program,
    ProgramIdentity, SharedFunctionAccelerator, shared_function_accelerator,
};
use rhai::{Locked, Shared, locked_read, locked_write};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct FunctionKey {
    name: String,
    arity: usize,
}

impl FunctionKey {
    fn new(name: &str, arity: usize) -> Self {
        Self {
            name: name.to_owned(),
            arity,
        }
    }
}

struct PendingCall {
    program: ProgramIdentity,
    function: FunctionKey,
    started: Instant,
    child_time: Duration,
}

#[derive(Default)]
struct ProfileEntry {
    calls: u64,
    failures: u64,
    operations: u64,
    total_grain_time: Duration,
    max_grain_time: Duration,
    self_grain_time: Duration,
    max_self_grain_time: Duration,
}

#[derive(Default)]
struct ProfilerState {
    libraries: crate::script_library::ScriptLibraries,
    pending: HashMap<ThreadId, Vec<PendingCall>>,
    entries: BTreeMap<ProgramIdentity, BTreeMap<FunctionKey, ProfileEntry>>,
}

/// Aggregate Grain measurements for one direct script function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionProfileStats {
    pub name: String,
    pub arity: usize,
    pub calls: u64,
    pub failures: u64,
    pub operations: u64,
    /// Inclusive duration, including profiled nested helpers.
    pub total_grain_time: Duration,
    pub max_grain_time: Duration,
    /// Exclusive duration after subtracting profiled nested helpers.
    pub self_grain_time: Duration,
    pub max_self_grain_time: Duration,
}

impl FunctionProfileStats {
    /// Mean Grain duration across completed calls.
    #[must_use]
    pub fn average_grain_time(&self) -> Duration {
        if self.calls == 0 {
            return Duration::ZERO;
        }
        let nanos = self.total_grain_time.as_nanos() / u128::from(self.calls);
        Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
    }

    /// Mean exclusive Grain duration across completed calls.
    #[must_use]
    pub fn average_self_grain_time(&self) -> Duration {
        if self.calls == 0 {
            return Duration::ZERO;
        }
        let nanos = self.self_grain_time.as_nanos() / u128::from(self.calls);
        Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
    }
}

/// Observe direct Grain-to-Grain calls without compiling them.
///
/// The profiler is also an accelerator: it always declines, then pairs the
/// VM's completion callback with a local timer. One instance may be shared by
/// multiple programs and threads; nested calls are paired per thread.
#[derive(Clone, Default)]
pub struct FunctionProfiler {
    state: Shared<Locked<ProfilerState>>,
}

impl FunctionProfiler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn accelerator(&self) -> SharedFunctionAccelerator {
        shared_function_accelerator(self.clone())
    }

    /// Aggregate every observed program in descending total-time order.
    ///
    /// Functions with the same name and arity are combined. Use
    /// [`Self::profiles`] when program identity matters.
    #[must_use]
    pub fn current_profiles(&self) -> Vec<FunctionProfileStats> {
        let Some(state) = locked_read(&self.state) else {
            return Vec::new();
        };
        let mut aggregate = BTreeMap::new();
        for entries in state.entries.values() {
            for (key, entry) in entries {
                merge_entry(aggregate.entry(key.clone()).or_default(), entry);
            }
        }
        snapshot_profiles(&aggregate)
    }

    /// Snapshot one program generation in descending total-time order.
    #[must_use]
    pub fn profiles(&self, program: &Program<'_>) -> Vec<FunctionProfileStats> {
        let Some(state) = locked_read(&self.state) else {
            return Vec::new();
        };
        state
            .entries
            .get(program.identity())
            .map_or_else(Vec::new, snapshot_profiles)
    }

    /// Drop every completed profile and unmatched pending observation.
    pub fn clear(&self) {
        if let Some(mut state) = locked_write(&self.state) {
            state.pending.clear();
            state.entries.clear();
        }
    }
}

fn merge_entry(target: &mut ProfileEntry, source: &ProfileEntry) {
    target.calls = target.calls.saturating_add(source.calls);
    target.failures = target.failures.saturating_add(source.failures);
    target.operations = target.operations.saturating_add(source.operations);
    target.total_grain_time = target
        .total_grain_time
        .saturating_add(source.total_grain_time);
    target.max_grain_time = target.max_grain_time.max(source.max_grain_time);
    target.self_grain_time = target
        .self_grain_time
        .saturating_add(source.self_grain_time);
    target.max_self_grain_time = target.max_self_grain_time.max(source.max_self_grain_time);
}

fn snapshot_profiles(entries: &BTreeMap<FunctionKey, ProfileEntry>) -> Vec<FunctionProfileStats> {
    let mut profiles = entries
        .iter()
        .map(|(key, entry)| FunctionProfileStats {
            name: key.name.clone(),
            arity: key.arity,
            calls: entry.calls,
            failures: entry.failures,
            operations: entry.operations,
            total_grain_time: entry.total_grain_time,
            max_grain_time: entry.max_grain_time,
            self_grain_time: entry.self_grain_time,
            max_self_grain_time: entry.max_self_grain_time,
        })
        .collect::<Vec<_>>();
    profiles.sort_by(|left, right| {
        right
            .total_grain_time
            .cmp(&left.total_grain_time)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.arity.cmp(&right.arity))
    });
    profiles
}

impl FunctionAccelerator for FunctionProfiler {
    fn lower_script_library(
        &mut self,
        library: &Shared<rhai::Module>,
        source: Option<&str>,
    ) -> Option<rhai::grain::SharedProgram> {
        let mut state = locked_write(&self.state)?;
        Some(state.libraries.get(library, source, 64))
    }
    fn call(&mut self, call: FunctionCall<'_, '_>) -> AcceleratedCall {
        let Some(mut state) = locked_write(&self.state) else {
            return AcceleratedCall::Declined;
        };
        state
            .pending
            .entry(std::thread::current().id())
            .or_default()
            .push(PendingCall {
                program: call.program.identity().clone(),
                function: FunctionKey::new(call.name, call.arguments.len()),
                started: Instant::now(),
                child_time: Duration::ZERO,
            });
        AcceleratedCall::Declined
    }

    fn finish_grain_call(&mut self, outcome: FunctionCallOutcome<'_, '_>) {
        let Some(mut state) = locked_write(&self.state) else {
            return;
        };
        let thread = std::thread::current().id();
        let Some(pending) = state.pending.get_mut(&thread).and_then(std::vec::Vec::pop) else {
            return;
        };
        if &pending.program != outcome.program.identity()
            || pending.function.name != outcome.name
            || pending.function.arity != outcome.function.params.len()
        {
            state.pending.remove(&thread);
            return;
        }

        let duration = pending.started.elapsed();
        if let Some(parent) = state
            .pending
            .get_mut(&thread)
            .and_then(|stack| stack.last_mut())
        {
            parent.child_time = parent.child_time.saturating_add(duration);
        } else {
            state.pending.remove(&thread);
        }
        let self_duration = duration.saturating_sub(pending.child_time);
        let entry = state
            .entries
            .entry(pending.program)
            .or_default()
            .entry(pending.function)
            .or_default();
        entry.calls = entry.calls.saturating_add(1);
        entry.failures = entry.failures.saturating_add(u64::from(!outcome.succeeded));
        entry.operations = entry.operations.saturating_add(outcome.operations);
        entry.total_grain_time = entry.total_grain_time.saturating_add(duration);
        entry.max_grain_time = entry.max_grain_time.max(duration);
        entry.self_grain_time = entry.self_grain_time.saturating_add(self_duration);
        entry.max_self_grain_time = entry.max_self_grain_time.max(self_duration);
    }
}

#[cfg(test)]
mod tests {
    use rhai::grain::{Compiler, Vm};
    use rhai::{CallFnOptions, Engine, Scope};

    use super::*;

    #[test]
    fn profiler_measures_declined_nested_calls() {
        let engine = Engine::new();
        let ast = engine
            .compile("fn helper(value) { value + 1 } fn entry() { helper(41) }")
            .unwrap();
        let program = Compiler::new().compile(&ast);
        let profiler = FunctionProfiler::new();
        let mut vm = Vm::new(&engine).with_function_accelerator(profiler.accelerator());

        for _ in 0..2 {
            let value: i64 = vm
                .call_fn_with_options(
                    CallFnOptions::new().eval_ast(false),
                    &mut Scope::new(),
                    &program,
                    "entry",
                    (),
                )
                .unwrap();
            assert_eq!(value, 42);
        }

        let snapshot = profiler.profiles(&program);
        assert_eq!(snapshot.len(), 2);
        let helper = snapshot
            .iter()
            .find(|profile| profile.name == "helper")
            .unwrap();
        assert_eq!(
            (
                helper.name.as_str(),
                helper.arity,
                helper.calls,
                helper.failures,
                helper.operations,
            ),
            ("helper", 1, 2, 0, 2)
        );
    }

    #[test]
    fn profiler_accumulates_multiple_programs() {
        let engine = Engine::new();
        let first_ast = engine
            .compile("fn chrome_helper() { 1 } fn entry() { chrome_helper() }")
            .unwrap();
        let second_ast = engine
            .compile("fn panel_helper() { 2 } fn entry() { panel_helper() }")
            .unwrap();
        let first = Compiler::new().compile(&first_ast);
        let second = Compiler::new().compile(&second_ast);
        let profiler = FunctionProfiler::new();

        for program in [&first, &second] {
            let _: i64 = Vm::new(&engine)
                .with_function_accelerator(profiler.accelerator())
                .call_fn_with_options(
                    CallFnOptions::new().eval_ast(false),
                    &mut Scope::new(),
                    program,
                    "entry",
                    (),
                )
                .unwrap();
        }

        let names = profiler
            .current_profiles()
            .into_iter()
            .map(|profile| profile.name)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            names,
            std::collections::BTreeSet::from([
                "chrome_helper".to_owned(),
                "panel_helper".to_owned(),
                "entry".to_owned(),
            ])
        );
        assert!(
            profiler
                .profiles(&first)
                .iter()
                .any(|profile| profile.name == "chrome_helper")
        );
        assert!(
            profiler
                .profiles(&second)
                .iter()
                .any(|profile| profile.name == "panel_helper")
        );
    }

    #[test]
    fn profiler_subtracts_nested_helper_time() {
        let engine = Engine::new();
        let ast = engine
            .compile(
                "fn leaf(value) { value + 1 }\n\
                 fn parent() { leaf(41) }\n\
                 fn entry() { parent() }",
            )
            .unwrap();
        let program = Compiler::new().compile(&ast);
        let profiler = FunctionProfiler::new();

        let _: i64 = Vm::new(&engine)
            .with_function_accelerator(profiler.accelerator())
            .call_fn_with_options(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                &program,
                "entry",
                (),
            )
            .unwrap();

        let parent = profiler
            .profiles(&program)
            .into_iter()
            .find(|profile| profile.name == "parent")
            .unwrap();
        assert!(parent.self_grain_time < parent.total_grain_time);
    }
}
