use std::any::TypeId;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use rhai::grain::{
    AcceleratedCall, FunctionAccelerator, FunctionCall, FunctionCallOutcome, NativeFrame,
    NativeFunction, ProgramIdentity, SharedFunctionAccelerator, shared_function_accelerator,
};
use rhai::{Dynamic, EvalAltResult, Locked, Shared, locked_read, locked_write};

use crate::jit_aarch64::{CompiledChunk, compile};
use crate::managed_aarch64::CompiledManaged;
use crate::typed::{ValueKind, lower_scalar_function};

/// Policy for automatic promotion from Grain to native code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdaptiveTierConfig {
    /// Compare warmed self-time samples and fall back when native execution
    /// does not demonstrate a margin over Grain. Disable only for diagnostics.
    pub check_profitability: bool,
    /// Completed Grain calls required before compilation is attempted.
    pub min_calls: u64,
    /// Exclusive Grain time required before compilation is attempted.
    pub min_self_grain_time: Duration,
    /// Maximum executable bytes retained across every Program.
    pub max_code_bytes: usize,
    /// Maximum Program generations retained by this tier.
    pub max_programs: usize,
    /// Maximum runtime signatures retained per function and Program.
    pub max_specializations_per_function: usize,
}

impl Default for AdaptiveTierConfig {
    fn default() -> Self {
        Self {
            check_profitability: true,
            min_calls: 32,
            min_self_grain_time: Duration::from_millis(1),
            max_code_bytes: 16 * 1024 * 1024,
            max_programs: 64,
            max_specializations_per_function: 4,
        }
    }
}

impl AdaptiveTierConfig {
    fn normalized(self) -> Self {
        Self {
            min_calls: self.min_calls.max(1),
            max_code_bytes: self.max_code_bytes.max(1),
            max_programs: self.max_programs.max(1),
            max_specializations_per_function: self.max_specializations_per_function.max(1),
            ..self
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct FunctionKey {
    name_index: u32,
    arity: usize,
}

impl FunctionKey {
    const fn new(name_index: u32, arity: usize) -> Self {
        Self { name_index, arity }
    }
}

const INLINE_ARGUMENT_TYPES: usize = 8;
const INLINE_ARGUMENT_TYPES_U8: u8 = 8;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TypeSignature {
    Inline {
        len: u8,
        types: [TypeId; INLINE_ARGUMENT_TYPES],
    },
    Heap(Vec<TypeId>),
}

impl TypeSignature {
    fn from_arguments(arguments: &[Dynamic]) -> Self {
        if arguments.len() <= INLINE_ARGUMENT_TYPES {
            let mut types = [TypeId::of::<()>(); INLINE_ARGUMENT_TYPES];
            for (slot, argument) in types.iter_mut().zip(arguments) {
                *slot = argument.type_id();
            }
            return Self::Inline {
                len: u8::try_from(arguments.len()).unwrap_or(INLINE_ARGUMENT_TYPES_U8),
                types,
            };
        }
        Self::Heap(arguments.iter().map(Dynamic::type_id).collect())
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SpecializationKey {
    function: FunctionKey,
    argument_types: TypeSignature,
}

impl SpecializationKey {
    fn from_call(call: &FunctionCall<'_, '_>) -> Self {
        Self {
            function: FunctionKey::new(call.function.name, call.arguments.len()),
            argument_types: TypeSignature::from_arguments(call.arguments),
        }
    }
}

enum CompiledExecutable {
    Scalar(CompiledChunk),
    Managed(CompiledManaged),
}

impl CompiledExecutable {
    fn allocation_size(&self) -> usize {
        match self {
            Self::Scalar(code) => code.allocation_size(),
            Self::Managed(code) => code.allocation_size(),
        }
    }
    fn lane(&self) -> &'static str {
        match self {
            Self::Scalar(_) => "scalar",
            Self::Managed(_) => "managed",
        }
    }
    fn code_size(&self) -> usize {
        match self {
            Self::Scalar(compiled) => compiled.code_size(),
            Self::Managed(compiled) => compiled.code_size(),
        }
    }
}

impl NativeFunction for CompiledExecutable {
    fn execute(
        &self,
        frame: &mut NativeFrame<'_, '_, '_, '_>,
    ) -> Result<Dynamic, Box<EvalAltResult>> {
        match self {
            Self::Scalar(compiled) => NativeFunction::execute(compiled, frame),
            Self::Managed(compiled) => compiled.execute(frame),
        }
    }
}

enum AdaptiveState {
    Profiling,
    Compiled(Shared<CompiledExecutable>),
    Rejected(String),
}

const COST_SAMPLES: usize = 16;
const COST_WARMUP: u64 = 2;
const RETRY_NATIVE_AFTER: u64 = 256;

#[derive(Default)]
struct Profitability {
    grain: VecDeque<Duration>,
    native: VecDeque<Duration>,
    grain_seen: u64,
    native_seen: u64,
    decisions: u64,
    suppressed: bool,
}

impl Profitability {
    fn observe(&mut self, native: bool, duration: Duration) {
        let (seen, samples) = if native {
            (&mut self.native_seen, &mut self.native)
        } else {
            (&mut self.grain_seen, &mut self.grain)
        };
        *seen = seen.saturating_add(1);
        if *seen <= COST_WARMUP {
            return;
        }
        if samples.len() == COST_SAMPLES {
            samples.pop_front();
        }
        samples.push_back(duration);
        if native
            && self.native.len() == COST_SAMPLES
            && self.grain.len() == COST_SAMPLES
            && self
                .native_seen
                .saturating_sub(COST_WARMUP)
                .is_multiple_of(COST_SAMPLES as u64)
        {
            let grain = median(&self.grain).unwrap_or_default().as_nanos();
            let native = median(&self.native).unwrap_or_default().as_nanos();
            self.suppressed = native.saturating_mul(100) > grain.saturating_mul(95);
            if self.suppressed {
                self.decisions = 0;
            }
        }
    }

    fn use_grain(&mut self) -> bool {
        self.decisions = self.decisions.saturating_add(1);
        if self.suppressed {
            if self.decisions < RETRY_NATIVE_AFTER {
                return true;
            }
            self.suppressed = false;
            self.decisions = 0;
            self.native.clear();
            self.native_seen = 0;
        }
        // Refresh the reference without ever executing an effect twice. A
        // complete invocation chooses one backend at its entry boundary.
        let interval = if self.grain.len() < COST_SAMPLES {
            16
        } else {
            256
        };
        self.native_seen > COST_WARMUP && self.decisions.is_multiple_of(interval)
    }
}

fn median(samples: &VecDeque<Duration>) -> Option<Duration> {
    if samples.is_empty() {
        return None;
    }
    let mut values = samples.iter().copied().collect::<Vec<_>>();
    let middle = values.len() / 2;
    let (_, median, _) = values.select_nth_unstable(middle);
    Some(*median)
}

struct AdaptiveEntry {
    return_types: BTreeSet<String>,
    profitability: Profitability,
    name: String,
    argument_types: Vec<String>,
    grain_calls: u64,
    grain_failures: u64,
    grain_operations: u64,
    grain_time: Duration,
    self_grain_time: Duration,
    jit_calls: u64,
    jit_failures: u64,
    jit_operations: u64,
    guard_misses: u64,
    compile_attempts: u64,
    compile_time: Duration,
    jit_time: Duration,
    max_jit_time: Duration,
    last_used: u64,
    state: AdaptiveState,
}

impl AdaptiveEntry {
    fn allocation_size(&self) -> usize {
        match &self.state {
            AdaptiveState::Compiled(code) => code.allocation_size(),
            _ => 0,
        }
    }
    fn new(last_used: u64, call: &FunctionCall<'_, '_>) -> Self {
        Self {
            return_types: BTreeSet::new(),
            profitability: Profitability::default(),
            name: call.name.to_owned(),
            argument_types: call
                .arguments
                .iter()
                .map(|argument| call.engine.map_type_name(argument.type_name()).to_owned())
                .collect(),
            grain_calls: 0,
            grain_failures: 0,
            grain_operations: 0,
            grain_time: Duration::ZERO,
            self_grain_time: Duration::ZERO,
            jit_calls: 0,
            jit_failures: 0,
            jit_operations: 0,
            guard_misses: 0,
            compile_attempts: 0,
            compile_time: Duration::ZERO,
            jit_time: Duration::ZERO,
            max_jit_time: Duration::ZERO,
            last_used,
            state: AdaptiveState::Profiling,
        }
    }

    fn code_size(&self) -> usize {
        match &self.state {
            AdaptiveState::Compiled(compiled) => compiled.code_size(),
            AdaptiveState::Profiling | AdaptiveState::Rejected(_) => 0,
        }
    }
}

struct ProgramState {
    index: u64,
    last_used: u64,
    entries: BTreeMap<SpecializationKey, AdaptiveEntry>,
}

struct PendingCall {
    program: ProgramIdentity,
    specialization: SpecializationKey,
    started: Instant,
    child_time: Duration,
}

struct AdaptiveTierState {
    libraries: crate::script_library::ScriptLibraries,
    config: AdaptiveTierConfig,
    sequence: u64,
    next_program_index: u64,
    code_bytes: usize,
    evictions: u64,
    programs: BTreeMap<ProgramIdentity, ProgramState>,
    pending: HashMap<ThreadId, Vec<PendingCall>>,
}

impl AdaptiveTierState {
    fn new(config: AdaptiveTierConfig) -> Self {
        Self {
            libraries: crate::script_library::ScriptLibraries::default(),
            config: config.normalized(),
            sequence: 0,
            next_program_index: 1,
            code_bytes: 0,
            evictions: 0,
            programs: BTreeMap::new(),
            pending: HashMap::new(),
        }
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence = self.sequence.saturating_add(1);
        self.sequence
    }

    fn ensure_program(&mut self, identity: &ProgramIdentity, sequence: u64) {
        if let Some(program) = self.programs.get_mut(identity) {
            program.last_used = sequence;
            return;
        }
        let index = self.next_program_index;
        self.next_program_index = self.next_program_index.saturating_add(1);
        self.programs.insert(
            identity.clone(),
            ProgramState {
                index,
                last_used: sequence,
                entries: BTreeMap::new(),
            },
        );
    }
}

/// Snapshot of one Program/function/runtime-signature tier entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdaptiveFunctionStats {
    /// Backing executable-buffer capacity, distinct from instruction bytes.
    pub allocated_code_bytes: usize,
    pub return_types: Vec<String>,
    pub grain_self_median: Option<Duration>,
    pub jit_self_median: Option<Duration>,
    pub native_lane: Option<&'static str>,
    pub program: u64,
    pub name: String,
    pub arity: usize,
    pub argument_types: Vec<String>,
    pub grain_calls: u64,
    pub grain_failures: u64,
    pub grain_operations: u64,
    pub grain_time: Duration,
    pub self_grain_time: Duration,
    pub jit_calls: u64,
    pub jit_failures: u64,
    pub jit_operations: u64,
    pub guard_misses: u64,
    pub compile_attempts: u64,
    pub compile_time: Duration,
    pub jit_time: Duration,
    pub max_jit_time: Duration,
    pub code_bytes: usize,
    pub state: &'static str,
    pub rejection: Option<String>,
}

impl AdaptiveFunctionStats {
    /// Mean inclusive Grain duration before promotion or rejection.
    #[must_use]
    pub fn average_grain_time(&self) -> Duration {
        average_duration(self.grain_time, self.grain_calls)
    }

    /// Mean exclusive Grain duration before promotion or rejection.
    #[must_use]
    pub fn average_self_grain_time(&self) -> Duration {
        average_duration(self.self_grain_time, self.grain_calls)
    }

    /// Mean native duration across completed and failed JIT calls.
    #[must_use]
    pub fn average_jit_time(&self) -> Duration {
        average_duration(self.jit_time, self.jit_calls)
    }
}

fn average_duration(duration: Duration, calls: u64) -> Duration {
    if calls == 0 {
        return Duration::ZERO;
    }
    let nanos = duration.as_nanos() / u128::from(calls);
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
}

/// Aggregate cache state for an adaptive tier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdaptiveTierSummary {
    pub library_compilations: u64,
    pub library_compile_time: Duration,
    /// Cached executable-buffer capacity; excludes in-flight evicted code and
    /// OS page rounding/Rust metadata, so it is not a process RSS measurement.
    pub allocated_code_bytes: usize,
    pub programs: usize,
    pub specializations: usize,
    pub code_bytes: usize,
    pub evictions: u64,
}

/// Automatically profile, compile, guard and cache direct Grain functions.
#[derive(Clone)]
pub struct AdaptiveFunctionTier {
    state: Shared<Locked<AdaptiveTierState>>,
}

impl AdaptiveFunctionTier {
    #[must_use]
    pub fn new(config: AdaptiveTierConfig) -> Self {
        Self {
            state: Shared::new(Locked::new(AdaptiveTierState::new(config))),
        }
    }

    #[must_use]
    pub fn accelerator(&self) -> SharedFunctionAccelerator {
        shared_function_accelerator(self.clone())
    }

    /// Snapshot all retained specializations, hottest Grain self-time first.
    #[must_use]
    pub fn stats(&self) -> Vec<AdaptiveFunctionStats> {
        let Some(guard) = locked_read(&self.state) else {
            return Vec::new();
        };
        let mut snapshot = guard
            .programs
            .values()
            .flat_map(|program| {
                program
                    .entries
                    .iter()
                    .map(|(key, entry)| adaptive_stats(program.index, key, entry))
            })
            .collect::<Vec<_>>();
        snapshot.sort_by(|left, right| {
            right
                .self_grain_time
                .cmp(&left.self_grain_time)
                .then_with(|| left.program.cmp(&right.program))
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.argument_types.cmp(&right.argument_types))
        });
        snapshot
    }

    #[must_use]
    pub fn summary(&self) -> AdaptiveTierSummary {
        let Some(state) = locked_read(&self.state) else {
            return AdaptiveTierSummary {
                library_compilations: 0,
                library_compile_time: Duration::ZERO,
                allocated_code_bytes: 0,
                programs: 0,
                specializations: 0,
                code_bytes: 0,
                evictions: 0,
            };
        };
        let (library_compilations, library_compile_time) = state.libraries.compilation_stats();
        AdaptiveTierSummary {
            library_compilations,
            library_compile_time,
            allocated_code_bytes: state.code_bytes,
            programs: state.programs.len(),
            specializations: state
                .programs
                .values()
                .map(|program| program.entries.len())
                .sum(),
            code_bytes: state
                .programs
                .values()
                .flat_map(|program| program.entries.values())
                .map(AdaptiveEntry::code_size)
                .sum(),
            evictions: state.evictions,
        }
    }
}

impl FunctionAccelerator for AdaptiveFunctionTier {
    fn lower_script_library(
        &mut self,
        library: &Shared<rhai::Module>,
        source: Option<&str>,
    ) -> Option<rhai::grain::SharedProgram> {
        let mut state = locked_write(&self.state)?;
        let maximum = state.config.max_programs;
        Some(state.libraries.get(library, source, maximum))
    }

    fn call(&mut self, call: FunctionCall<'_, '_>) -> AcceleratedCall {
        let identity = call.program.identity().clone();
        let specialization = SpecializationKey::from_call(&call);
        let thread = std::thread::current().id();
        let Some(mut state) = locked_write(&self.state) else {
            return AcceleratedCall::Declined;
        };
        let sequence = state.next_sequence();
        state.ensure_program(&identity, sequence);
        evict_to_limits(&mut state, &identity, &specialization);
        enforce_specialization_limit(&mut state, &identity, &specialization);
        if specialization_is_ready(&mut state, &identity, &specialization, sequence, &call) {
            compile_and_cache(&mut state, &identity, &specialization, &call);
        }
        match dispatch_cached(&mut state, &identity, &specialization, &call) {
            CacheDispatch::Prepared(executable) => {
                record_pending(&mut state, thread, identity, specialization, false);
                AcceleratedCall::Prepared(executable)
            }
            CacheDispatch::GuardMiss => {
                record_pending(&mut state, thread, identity, specialization, true)
            }
            CacheDispatch::Unavailable => {
                record_pending(&mut state, thread, identity, specialization, false)
            }
        }
    }

    fn finish_grain_call(&mut self, outcome: FunctionCallOutcome<'_, '_>) {
        self.finish_call(&outcome, false);
    }

    fn finish_native_call(&mut self, outcome: FunctionCallOutcome<'_, '_>) {
        self.finish_call(&outcome, true);
    }
}

impl AdaptiveFunctionTier {
    fn finish_call(&self, outcome: &FunctionCallOutcome<'_, '_>, native: bool) {
        let Some(mut state) = locked_write(&self.state) else {
            return;
        };
        let thread = std::thread::current().id();
        let Some(pending) = state.pending.get_mut(&thread).and_then(std::vec::Vec::pop) else {
            return;
        };
        if &pending.program != outcome.program.identity()
            || pending.specialization.function.name_index != outcome.function.name
            || pending.specialization.function.arity != outcome.function.params.len()
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
        let sequence = state.next_sequence();
        let check_profitability = state.config.check_profitability;
        state.ensure_program(&pending.program, sequence);
        let specialization = pending.specialization;
        let Some(program) = state.programs.get_mut(&pending.program) else {
            return;
        };
        let Some(entry) = program.entries.get_mut(&specialization) else {
            return;
        };
        entry.last_used = sequence;
        if let Some(name) = outcome.result_type {
            let name = outcome.engine.map_type_name(name);
            if entry.return_types.len() < 8 && !entry.return_types.contains(name) {
                entry.return_types.insert(name.to_owned());
            }
        }
        if check_profitability && outcome.succeeded {
            entry.profitability.observe(native, self_duration);
        }
        if native {
            entry.jit_calls = entry.jit_calls.saturating_add(1);
            entry.jit_failures = entry
                .jit_failures
                .saturating_add(u64::from(!outcome.succeeded));
            entry.jit_operations = entry
                .jit_operations
                .saturating_add(outcome.operations.saturating_sub(1));
            entry.jit_time = entry.jit_time.saturating_add(duration);
            entry.max_jit_time = entry.max_jit_time.max(duration);
            evict_to_limits(&mut state, &pending.program, &specialization);
            return;
        }
        entry.grain_calls = entry.grain_calls.saturating_add(1);
        entry.grain_failures = entry
            .grain_failures
            .saturating_add(u64::from(!outcome.succeeded));
        entry.grain_operations = entry.grain_operations.saturating_add(outcome.operations);
        entry.grain_time = entry.grain_time.saturating_add(duration);
        entry.self_grain_time = entry.self_grain_time.saturating_add(self_duration);
        evict_to_limits(&mut state, &pending.program, &specialization);
    }
}

enum CacheDispatch {
    Unavailable,
    GuardMiss,
    Prepared(Shared<dyn NativeFunction>),
}

fn specialization_is_ready(
    state: &mut AdaptiveTierState,
    program_id: &ProgramIdentity,
    specialization: &SpecializationKey,
    sequence: u64,
    call: &FunctionCall<'_, '_>,
) -> bool {
    let config = state.config;
    let Some(program) = state.programs.get_mut(program_id) else {
        return false;
    };
    let entry = program
        .entries
        .entry(specialization.clone())
        .or_insert_with(|| AdaptiveEntry::new(sequence, call));
    entry.last_used = sequence;
    matches!(entry.state, AdaptiveState::Profiling)
        && entry.grain_calls >= config.min_calls
        && entry.self_grain_time >= config.min_self_grain_time
}

fn compile_and_cache(
    state: &mut AdaptiveTierState,
    program_id: &ProgramIdentity,
    specialization: &SpecializationKey,
    call: &FunctionCall<'_, '_>,
) {
    let max_code_bytes = state.config.max_code_bytes;
    let started = Instant::now();
    let (new_state, added_code_bytes) = match compile_specialization(call) {
        Ok(compiled) if compiled.allocation_size() <= max_code_bytes => {
            let bytes = compiled.allocation_size();
            (AdaptiveState::Compiled(Shared::new(compiled)), bytes)
        }
        Ok(compiled) => (
            AdaptiveState::Rejected(format!(
                "native allocation size {} exceeds cache limit {max_code_bytes}",
                compiled.allocation_size()
            )),
            0,
        ),
        Err(error) => (AdaptiveState::Rejected(error), 0),
    };
    let Some(entry) = state
        .programs
        .get_mut(program_id)
        .and_then(|program| program.entries.get_mut(specialization))
    else {
        return;
    };
    entry.compile_attempts = entry.compile_attempts.saturating_add(1);
    entry.compile_time = entry.compile_time.saturating_add(started.elapsed());
    entry.state = new_state;
    state.code_bytes = state.code_bytes.saturating_add(added_code_bytes);
    evict_to_limits(state, program_id, specialization);
}

fn dispatch_cached(
    state: &mut AdaptiveTierState,
    program_id: &ProgramIdentity,
    specialization: &SpecializationKey,
    call: &FunctionCall<'_, '_>,
) -> CacheDispatch {
    let check_profitability = state.config.check_profitability;
    let Some(entry) = state
        .programs
        .get_mut(program_id)
        .and_then(|program| program.entries.get_mut(specialization))
    else {
        return CacheDispatch::Unavailable;
    };
    let AdaptiveState::Compiled(compiled) = &entry.state else {
        return CacheDispatch::Unavailable;
    };
    if check_profitability && entry.profitability.use_grain() {
        return CacheDispatch::Unavailable;
    }
    if let CompiledExecutable::Scalar(scalar) = compiled.as_ref() {
        if !call.checked_arithmetic {
            return CacheDispatch::GuardMiss;
        }
        if !call.engine.allow_shadowing() || !scalar.fits_scope(call.scope_len, call.max_variables)
        {
            return CacheDispatch::GuardMiss;
        }
        if !call.engine.fast_operators()
            || !call.can_batch_operations
            || !scalar.guard_matches(call.arguments)
        {
            return CacheDispatch::GuardMiss;
        }
        if remaining_operations(call).is_none() {
            return CacheDispatch::Unavailable;
        }
    }
    CacheDispatch::Prepared(compiled.clone())
}

fn record_pending(
    state: &mut AdaptiveTierState,
    thread: ThreadId,
    program: ProgramIdentity,
    specialization: SpecializationKey,
    guard_miss: bool,
) -> AcceleratedCall {
    if guard_miss
        && let Some(entry) = state
            .programs
            .get_mut(&program)
            .and_then(|program| program.entries.get_mut(&specialization))
    {
        entry.guard_misses = entry.guard_misses.saturating_add(1);
    }
    state.pending.entry(thread).or_default().push(PendingCall {
        program,
        specialization,
        started: Instant::now(),
        child_time: Duration::ZERO,
    });
    AcceleratedCall::Declined
}

fn compile_specialization(call: &FunctionCall<'_, '_>) -> Result<CompiledExecutable, String> {
    if call.engine.fast_operators()
        && call.can_batch_operations
        && call.checked_arithmetic
        && let Some(compiled) = call
            .arguments
            .iter()
            .map(ValueKind::of)
            .collect::<Option<Vec<_>>>()
            .and_then(|kinds| lower_scalar_function(call.program, call.function, &kinds).ok())
            .and_then(|typed| compile(&typed).ok())
    {
        return Ok(CompiledExecutable::Scalar(compiled));
    }
    crate::managed_aarch64::compile(call.program, call.function).map(CompiledExecutable::Managed)
}

fn remaining_operations(call: &FunctionCall<'_, '_>) -> Option<u64> {
    if call.max_operations == 0 {
        return Some(0);
    }
    let remaining = call
        .max_operations
        .checked_sub(call.operation_base.saturating_add(1))?;
    (remaining > 0).then_some(remaining)
}

fn enforce_specialization_limit(
    state: &mut AdaptiveTierState,
    program_id: &ProgramIdentity,
    incoming: &SpecializationKey,
) {
    let Some(program) = state.programs.get(program_id) else {
        return;
    };
    if program.entries.contains_key(incoming) {
        return;
    }
    let count = program
        .entries
        .keys()
        .filter(|key| key.function == incoming.function)
        .count();
    if count < state.config.max_specializations_per_function {
        return;
    }
    let victim = program
        .entries
        .iter()
        .filter(|(key, _)| key.function == incoming.function)
        .min_by_key(|(_, entry)| entry.last_used)
        .map(|(key, _)| key.clone());
    if let Some(victim) = victim {
        remove_entry(state, program_id, &victim);
    }
}

fn evict_to_limits(
    state: &mut AdaptiveTierState,
    current_program: &ProgramIdentity,
    current_specialization: &SpecializationKey,
) {
    while state.code_bytes > state.config.max_code_bytes {
        let victim = state
            .programs
            .iter()
            .flat_map(|(program_id, program)| {
                program.entries.iter().filter_map(move |(key, entry)| {
                    let current = program_id == current_program && key == current_specialization;
                    (!current && entry.code_size() > 0)
                        .then(|| (program_id.clone(), key.clone(), entry.last_used))
                })
            })
            .min_by_key(|(_, _, last_used)| *last_used);
        let Some((program, specialization, _)) = victim else {
            break;
        };
        remove_entry(state, &program, &specialization);
    }

    while state.programs.len() > state.config.max_programs {
        let victim = state
            .programs
            .iter()
            .filter(|(identity, _)| *identity != current_program)
            .min_by_key(|(_, program)| program.last_used)
            .map(|(identity, _)| identity.clone());
        let Some(victim) = victim else {
            break;
        };
        if let Some(program) = state.programs.remove(&victim) {
            let bytes = program
                .entries
                .values()
                .map(AdaptiveEntry::allocation_size)
                .sum::<usize>();
            state.code_bytes = state.code_bytes.saturating_sub(bytes);
            state.evictions = state.evictions.saturating_add(1);
        }
    }
}

fn remove_entry(
    state: &mut AdaptiveTierState,
    program_id: &ProgramIdentity,
    specialization: &SpecializationKey,
) {
    let removed = state
        .programs
        .get_mut(program_id)
        .and_then(|program| program.entries.remove(specialization));
    if let Some(entry) = removed {
        state.code_bytes = state.code_bytes.saturating_sub(entry.allocation_size());
        state.evictions = state.evictions.saturating_add(1);
    }
}

fn adaptive_stats(
    program: u64,
    key: &SpecializationKey,
    entry: &AdaptiveEntry,
) -> AdaptiveFunctionStats {
    AdaptiveFunctionStats {
        allocated_code_bytes: entry.allocation_size(),
        return_types: entry.return_types.iter().cloned().collect(),
        grain_self_median: median(&entry.profitability.grain),
        jit_self_median: median(&entry.profitability.native),
        native_lane: match &entry.state {
            AdaptiveState::Compiled(code) => Some(code.lane()),
            _ => None,
        },
        program,
        name: entry.name.clone(),
        arity: key.function.arity,
        argument_types: entry.argument_types.clone(),
        grain_calls: entry.grain_calls,
        grain_failures: entry.grain_failures,
        grain_operations: entry.grain_operations,
        grain_time: entry.grain_time,
        self_grain_time: entry.self_grain_time,
        jit_calls: entry.jit_calls,
        jit_failures: entry.jit_failures,
        jit_operations: entry.jit_operations,
        guard_misses: entry.guard_misses,
        compile_attempts: entry.compile_attempts,
        compile_time: entry.compile_time,
        jit_time: entry.jit_time,
        max_jit_time: entry.max_jit_time,
        code_bytes: entry.code_size(),
        state: match &entry.state {
            AdaptiveState::Profiling => "profiling",
            AdaptiveState::Compiled(_) if entry.profitability.suppressed => "unprofitable",
            AdaptiveState::Compiled(_) => "compiled",
            AdaptiveState::Rejected(_) => "rejected",
        },
        rejection: match &entry.state {
            AdaptiveState::Rejected(reason) => Some(reason.clone()),
            AdaptiveState::Profiling | AdaptiveState::Compiled(_) => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use rhai::grain::{Compiler, Program, Vm};
    use rhai::{CallFnOptions, Dynamic, Engine, Scope};

    use super::*;

    #[test]
    fn slower_native_samples_fall_back_and_retry_without_recompiling() {
        let mut cost = Profitability::default();
        for _ in 0..COST_WARMUP + COST_SAMPLES as u64 {
            cost.observe(false, Duration::from_nanos(100));
            cost.observe(true, Duration::from_nanos(120));
        }
        assert!(cost.suppressed);
        for _ in 1..RETRY_NATIVE_AFTER {
            assert!(cost.use_grain());
        }
        assert!(!cost.use_grain());
        for _ in 0..COST_WARMUP + COST_SAMPLES as u64 {
            cost.observe(true, Duration::from_nanos(50));
        }
        assert!(!cost.suppressed);
    }

    #[test]
    fn one_cold_outlier_does_not_decide_profitability() {
        let mut cost = Profitability::default();
        for index in 0..COST_WARMUP + COST_SAMPLES as u64 {
            cost.observe(false, Duration::from_nanos(100));
            cost.observe(
                true,
                if index == 4 {
                    Duration::from_secs(1)
                } else {
                    Duration::from_nanos(50)
                },
            );
        }
        assert!(!cost.suppressed);
    }

    fn test_config(min_calls: u64) -> AdaptiveTierConfig {
        AdaptiveTierConfig {
            check_profitability: false,
            min_calls,
            min_self_grain_time: Duration::ZERO,
            max_code_bytes: 1024 * 1024,
            max_programs: 8,
            max_specializations_per_function: 4,
        }
    }

    fn call_entry<T: rhai::Variant + Clone>(
        engine: &Engine,
        program: &Program<'_>,
        tier: &AdaptiveFunctionTier,
        entry: &str,
    ) -> T {
        Vm::new(engine)
            .with_function_accelerator(tier.accelerator())
            .call_fn_with_options(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                program,
                entry,
                (),
            )
            .unwrap()
    }

    #[test]
    fn hot_supported_function_promotes_automatically() {
        let engine = Engine::new();
        let ast = engine
            .compile("fn helper(value) { value + 1 } fn entry() { helper(41) }")
            .unwrap();
        let program = Compiler::new().compile(&ast);
        let tier = AdaptiveFunctionTier::new(test_config(2));

        for _ in 0..4 {
            assert_eq!(call_entry::<i64>(&engine, &program, &tier, "entry"), 42);
        }

        let stats = tier.stats();
        assert_eq!(stats.len(), 2, "the host entry and nested helper both tier");
        assert_eq!(
            (
                stats[0].state,
                stats[0].grain_calls,
                stats[0].jit_calls,
                stats[0].compile_attempts,
            ),
            ("compiled", 2, 2, 1)
        );
    }

    #[test]
    fn cold_function_stays_in_grain() {
        let engine = Engine::new();
        let ast = engine
            .compile("fn helper(value) { value + 1 } fn entry() { helper(41) }")
            .unwrap();
        let program = Compiler::new().compile(&ast);
        let tier = AdaptiveFunctionTier::new(test_config(3));

        for _ in 0..2 {
            let _: i64 = call_entry(&engine, &program, &tier, "entry");
        }

        let stats = tier.stats();
        assert_eq!(
            (stats[0].state, stats[0].grain_calls, stats[0].jit_calls),
            ("profiling", 2, 0)
        );
    }

    #[test]
    fn unsupported_instruction_is_rejected_once() {
        let engine = Engine::new();
        let ast = engine
            .compile("fn helper(value) { try { throw value; } catch (x) { return x + 1; } } fn entry() { helper(1.5) }")
            .unwrap();
        let program = Compiler::new().compile(&ast);
        let tier = AdaptiveFunctionTier::new(test_config(1));

        for _ in 0..3 {
            let value = call_entry::<rhai::FLOAT>(&engine, &program, &tier, "entry");
            assert!((value - 2.5).abs() < rhai::FLOAT::EPSILON);
        }

        let stats = tier
            .stats()
            .into_iter()
            .filter(|entry| entry.name == "helper")
            .collect::<Vec<_>>();
        assert_eq!(
            (
                stats[0].state,
                stats[0].grain_calls,
                stats[0].jit_calls,
                stats[0].compile_attempts,
            ),
            ("rejected", 3, 0, 1)
        );
        assert!(
            stats[0]
                .rejection
                .as_deref()
                .is_some_and(|reason| { reason.contains("unsupported managed instruction") })
        );
    }

    #[test]
    fn multiple_programs_keep_independent_native_caches() {
        let engine = Engine::new();
        let first_ast = engine
            .compile("fn helper(value) { value + 1 } fn entry() { helper(1) }")
            .unwrap();
        let second_ast = engine
            .compile("fn helper(value) { value + 100 } fn entry() { helper(1) }")
            .unwrap();
        let first = Compiler::new().compile(&first_ast);
        let second = Compiler::new().compile(&second_ast);
        let tier = AdaptiveFunctionTier::new(test_config(1));

        for _ in 0..2 {
            assert_eq!(call_entry::<i64>(&engine, &first, &tier, "entry"), 2);
            assert_eq!(call_entry::<i64>(&engine, &second, &tier, "entry"), 101);
        }

        let summary = tier.summary();
        assert_eq!((summary.programs, summary.specializations), (2, 4));
        assert_eq!(
            tier.stats()
                .iter()
                .map(|stats| stats.jit_calls)
                .sum::<u64>(),
            4
        );
    }

    #[test]
    fn program_limit_evicts_the_lru_cache() {
        let engine = Engine::new();
        let first_ast = engine
            .compile("fn first(value) { value + 1 } fn entry() { first(1) }")
            .unwrap();
        let second_ast = engine
            .compile("fn second(value) { value + 2 } fn entry() { second(1) }")
            .unwrap();
        let first = Compiler::new().compile(&first_ast);
        let second = Compiler::new().compile(&second_ast);
        let tier = AdaptiveFunctionTier::new(AdaptiveTierConfig {
            max_programs: 1,
            ..test_config(1)
        });

        for _ in 0..2 {
            let _: i64 = call_entry(&engine, &first, &tier, "entry");
        }
        for _ in 0..2 {
            let _: i64 = call_entry(&engine, &second, &tier, "entry");
        }

        let summary = tier.summary();
        assert_eq!((summary.programs, summary.specializations), (1, 2));
        assert!(summary.evictions >= 1);
        assert!(tier.stats().iter().any(|entry| entry.name == "second"));
    }

    #[test]
    fn runtime_signatures_profile_independently() {
        let engine = Engine::new();
        let ast = engine
            .compile(
                "fn helper(value) { value + 1 }\n\
                 fn int_entry() { helper(1) }\n\
                 fn float_entry() { helper(1.5) }",
            )
            .unwrap();
        let program = Compiler::new().compile(&ast);
        let tier = AdaptiveFunctionTier::new(test_config(10));

        let _: i64 = call_entry(&engine, &program, &tier, "int_entry");
        let _: rhai::FLOAT = call_entry(&engine, &program, &tier, "float_entry");

        let signatures = tier
            .stats()
            .into_iter()
            .filter(|stats| stats.name == "helper")
            .map(|stats| stats.argument_types)
            .collect::<Vec<_>>();
        assert_eq!(signatures.len(), 2);
        assert_ne!(signatures[0], signatures[1]);
    }

    #[test]
    fn specialization_limit_evicts_the_lru_signature() {
        let engine = Engine::new();
        let ast = engine
            .compile(
                "fn helper(value) { value + 1 }\n\
                 fn int_entry() { helper(1) }\n\
                 fn float_entry() { helper(1.5) }",
            )
            .unwrap();
        let program = Compiler::new().compile(&ast);
        let tier = AdaptiveFunctionTier::new(AdaptiveTierConfig {
            max_specializations_per_function: 1,
            ..test_config(10)
        });

        let _: i64 = call_entry(&engine, &program, &tier, "int_entry");
        let _: rhai::FLOAT = call_entry(&engine, &program, &tier, "float_entry");

        let summary = tier.summary();
        assert_eq!(
            summary.specializations, 3,
            "one helper signature and two host entries"
        );
        assert_eq!(summary.evictions, 1);
        assert_eq!(
            tier.stats()
                .iter()
                .find(|entry| entry.name == "helper")
                .unwrap()
                .argument_types,
            vec!["f64"]
        );
    }

    #[test]
    fn oversized_native_function_is_rejected_without_retention() {
        let engine = Engine::new();
        let ast = engine
            .compile("fn helper(value) { value + 1 } fn entry() { helper(1) }")
            .unwrap();
        let program = Compiler::new().compile(&ast);
        let tier = AdaptiveFunctionTier::new(AdaptiveTierConfig {
            max_code_bytes: 1,
            ..test_config(1)
        });

        for _ in 0..2 {
            let _: i64 = call_entry(&engine, &program, &tier, "entry");
        }

        let stats = tier.stats();
        assert_eq!((stats[0].state, stats[0].code_bytes), ("rejected", 0));
        assert!(
            stats[0]
                .rejection
                .as_deref()
                .is_some_and(|reason| reason.contains("exceeds cache limit"))
        );
        assert_eq!(tier.summary().code_bytes, 0);
    }

    #[test]
    fn operation_budget_error_is_not_reexecuted_in_grain() {
        let engine = Engine::new();
        let ast = engine
            .compile(
                "fn helper() { let value = 0; while value < 10 { value += 1; } value }\n\
                 fn entry() { helper() }",
            )
            .unwrap();
        let program = Compiler::new().compile(&ast);
        let tier = AdaptiveFunctionTier::new(test_config(1));

        assert_eq!(call_entry::<i64>(&engine, &program, &tier, "entry"), 10);
        assert_eq!(call_entry::<i64>(&engine, &program, &tier, "entry"), 10);

        let mut limited_engine = Engine::new();
        limited_engine.set_max_operations(4);
        let limited = Vm::new(&limited_engine)
            .with_function_accelerator(tier.accelerator())
            .call_fn_with_options::<Dynamic>(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                &program,
                "entry",
                (),
            );
        assert!(limited.is_err());
        let stats = tier.stats();
        assert_eq!(
            (
                stats[0].grain_calls,
                stats[0].jit_calls,
                stats[0].jit_failures,
            ),
            (1, 2, 1)
        );
    }
}
