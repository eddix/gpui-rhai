use std::collections::{BTreeMap, BTreeSet};

use rhai::grain::{
    AcceleratedCall, FunctionAccelerator, FunctionCall, Program, ProgramIdentity,
    SharedFunctionAccelerator, shared_function_accelerator,
};
use rhai::{Dynamic, EvalAltResult, Locked, Position, Shared, locked_read, locked_write};
use serde::Serialize;

use crate::jit_aarch64::{CompiledChunk, compile};
use crate::typed::lower_function;

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

enum TierState {
    Cold,
    Compiled(CompiledChunk),
    Rejected(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TierBackend {
    GrainCold,
    Jit,
    GrainRejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FunctionTierStats {
    pub calls: u64,
    pub compile_attempts: u64,
    pub jit_calls: u64,
    pub guard_misses: u64,
    pub state: &'static str,
    pub rejection: Option<String>,
}

struct FunctionTierEntry {
    calls: u64,
    compile_attempts: u64,
    jit_calls: u64,
    guard_misses: u64,
    state: TierState,
}

impl Default for FunctionTierEntry {
    fn default() -> Self {
        Self {
            calls: 0,
            compile_attempts: 0,
            jit_calls: 0,
            guard_misses: 0,
            state: TierState::Cold,
        }
    }
}

struct FunctionTierState {
    threshold: u64,
    targets: BTreeSet<FunctionKey>,
    active_program: Option<ProgramIdentity>,
    entries: BTreeMap<FunctionKey, FunctionTierEntry>,
}

/// Accelerator state shared between a Grain VM and the experiment harness.
///
/// The target allow-list keeps the first integration deliberately narrow: a
/// surrounding Rhai workload remains in Grain while only the chosen pure
/// helper becomes eligible for native compilation.
#[derive(Clone)]
pub struct FunctionTier {
    state: Shared<Locked<FunctionTierState>>,
}

impl FunctionTier {
    pub fn for_function(threshold: u64, name: &str, arity: usize) -> Self {
        Self {
            state: Shared::new(Locked::new(FunctionTierState {
                threshold: threshold.max(1),
                targets: BTreeSet::from([FunctionKey::new(name, arity)]),
                active_program: None,
                entries: BTreeMap::new(),
            })),
        }
    }

    pub fn accelerator(&self) -> SharedFunctionAccelerator {
        shared_function_accelerator(self.clone())
    }

    pub fn stats(
        &self,
        program: &Program<'_>,
        function: &str,
        arity: usize,
    ) -> Option<FunctionTierStats> {
        let state = locked_read(&self.state)?;
        if state.active_program.as_ref() != Some(program.identity()) {
            return None;
        }
        state
            .entries
            .get(&FunctionKey::new(function, arity))
            .map(function_tier_stats)
    }

    /// Snapshot one function from the active program generation.
    #[must_use]
    pub fn current_stats(&self, function: &str, arity: usize) -> Option<FunctionTierStats> {
        locked_read(&self.state)?
            .entries
            .get(&FunctionKey::new(function, arity))
            .map(function_tier_stats)
    }
}

fn function_tier_stats(entry: &FunctionTierEntry) -> FunctionTierStats {
    FunctionTierStats {
        calls: entry.calls,
        compile_attempts: entry.compile_attempts,
        jit_calls: entry.jit_calls,
        guard_misses: entry.guard_misses,
        state: tier_state_name(&entry.state),
        rejection: match &entry.state {
            TierState::Rejected(reason) => Some(reason.clone()),
            _ => None,
        },
    }
}

impl FunctionAccelerator for FunctionTier {
    fn call(&mut self, call: FunctionCall<'_, '_>) -> AcceleratedCall {
        // The typed IR hard-codes Rhai's primitive integer operators. When
        // fast operators are disabled, Grain deliberately goes through normal
        // function dispatch where a registered primitive override may win.
        if !call.engine.fast_operators() || !call.can_batch_operations || !call.checked_arithmetic {
            return AcceleratedCall::Declined;
        }

        let key = FunctionKey::new(call.name, call.arguments.len());
        let Some(mut state) = locked_write(&self.state) else {
            return AcceleratedCall::Declined;
        };
        if !state.targets.contains(&key) {
            return AcceleratedCall::Declined;
        }

        let identity = call.program.identity();
        if state.active_program.as_ref() != Some(identity) {
            state.entries.clear();
            state.active_program = Some(identity.clone());
        }

        let threshold = state.threshold;
        let entry = state.entries.entry(key).or_default();
        entry.calls = entry.calls.saturating_add(1);

        if entry.calls >= threshold && matches!(entry.state, TierState::Cold) {
            entry.compile_attempts = entry.compile_attempts.saturating_add(1);
            entry.state = match lower_function(call.program, call.function)
                .map_err(|error| error.to_string())
                .and_then(|typed| compile(&typed).map_err(|error| error.to_string()))
            {
                Ok(compiled) => TierState::Compiled(compiled),
                Err(error) => TierState::Rejected(error),
            };
        }

        let TierState::Compiled(compiled) = &entry.state else {
            return AcceleratedCall::Declined;
        };
        if !call.engine.allow_shadowing()
            || !compiled.fits_scope(call.scope_len, call.max_variables)
        {
            return AcceleratedCall::Declined;
        }

        let remaining = if call.max_operations == 0 {
            0
        } else {
            let Some(remaining) = call
                .max_operations
                .checked_sub(call.operation_base.saturating_add(1))
            else {
                return AcceleratedCall::Declined;
            };
            // The JIT uses zero to mean unlimited. Let Grain enforce the rare
            // exactly-zero remainder rather than weakening the host limit.
            if remaining == 0 {
                return AcceleratedCall::Declined;
            }
            remaining
        };

        let Some(execution) = compiled.execute_if_guard_matches(call.arguments, remaining) else {
            entry.guard_misses = entry.guard_misses.saturating_add(1);
            return AcceleratedCall::Declined;
        };

        entry.jit_calls = entry.jit_calls.saturating_add(1);
        match execution {
            Ok(execution) => AcceleratedCall::Completed {
                result: Ok(Dynamic::from(execution.value)),
                operations: execution.operations,
            },
            Err(error) => {
                let operations = error.operations();
                let error = match error {
                    crate::jit_aarch64::JitError::Arithmetic {
                        pc, left, right, ..
                    } => call.scalar_arithmetic_error(pc, left, right),
                    error => Box::new(jit_eval_error(error, call.position)),
                };
                AcceleratedCall::Completed {
                    result: Err(error),
                    operations,
                }
            }
        }
    }
}

fn tier_state_name(state: &TierState) -> &'static str {
    match state {
        TierState::Cold => "cold",
        TierState::Compiled(_) => "compiled",
        TierState::Rejected(_) => "rejected",
    }
}

fn jit_eval_error(error: crate::jit_aarch64::JitError, position: Position) -> EvalAltResult {
    match error {
        crate::jit_aarch64::JitError::TooManyOperations { .. } => {
            EvalAltResult::ErrorTooManyOperations(position)
        }
        crate::jit_aarch64::JitError::Arithmetic { .. } => EvalAltResult::ErrorArithmetic(
            "integer arithmetic failed in the experimental JIT".to_owned(),
            position,
        ),
        error => EvalAltResult::ErrorRuntime(error.to_string().into(), position),
    }
}
