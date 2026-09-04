#![expect(
    clippy::unnecessary_semicolon,
    clippy::used_underscore_binding,
    clippy::useless_conversion,
    reason = "these lints originate in dynasm's generated assembler writes, not the source templates"
)]

use std::collections::{BTreeMap, VecDeque};

use dynasmrt::aarch64::Assembler;
use dynasmrt::{AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi, ExecutableBuffer, dynasm};
use rhai::{CallFnOptions, Dynamic, Engine, Scope};
use serde::Serialize;
use thiserror::Error;

use crate::typed::{BinaryOp, Instruction, TypedChunk, TypedOp, ValueKind};

const MAX_LOCALS: usize = 64;
const STATUS_OK: u64 = 0;
const STATUS_TOO_MANY_OPERATIONS: u64 = 1;
const STATUS_ARITHMETIC: u64 = 2;

#[repr(C)]
struct JitFrame {
    locals: [i64; MAX_LOCALS],
    operations: u64,
    max_operations: u64,
    status: u64,
    error_pc: u64,
    error_left: i64,
    error_right: i64,
}

impl JitFrame {
    fn new(arguments: &[i64], max_operations: u64) -> Self {
        let mut frame = Self {
            locals: [0; MAX_LOCALS],
            operations: 0,
            max_operations,
            status: STATUS_OK,
            error_pc: 0,
            error_left: 0,
            error_right: 0,
        };
        frame.locals[..arguments.len()].copy_from_slice(arguments);
        frame
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardedBackend {
    Jit,
    GrainFallback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuardedExecution {
    pub backend: GuardedBackend,
    pub value: i64,
    pub operations: Option<u64>,
}

#[derive(Debug, Error)]
pub enum GuardedError {
    #[error(transparent)]
    Jit(#[from] JitError),
    #[error("Grain fallback failed: {0}")]
    Grain(#[source] Box<rhai::EvalAltResult>),
}

impl GuardedExecution {
    const fn jit(execution: JitExecution) -> Self {
        Self {
            backend: GuardedBackend::Jit,
            value: execution.value,
            operations: Some(execution.operations),
        }
    }

    const fn grain(value: i64) -> Self {
        Self {
            backend: GuardedBackend::GrainFallback,
            value,
            operations: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JitExecution {
    pub value: i64,
    pub operations: u64,
}

pub struct CompiledChunk {
    buffer: ExecutableBuffer,
    entry: AssemblyOffset,
    code_size: usize,
    function: String,
    parameters: usize,
    parameter_kinds: Vec<ValueKind>,
    return_kind: ValueKind,
    max_locals: usize,
}

impl rhai::grain::NativeFunction for CompiledChunk {
    fn execute(
        &self,
        frame: &mut rhai::grain::NativeFrame<'_, '_, '_, '_>,
    ) -> Result<Dynamic, Box<rhai::EvalAltResult>> {
        let mut arguments = [0_i64; MAX_LOCALS];
        for (index, kind) in self.parameter_kinds.iter().enumerate() {
            arguments[index] = frame
                .local(index)
                .and_then(|value| kind.encode(value))
                .ok_or_else(|| {
                    Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "scalar entry guard changed after selection".into(),
                        frame.position(),
                    ))
                })?;
        }
        let result =
            self.execute_bits(&arguments[..self.parameters], frame.remaining_operations()?);
        let operations = match &result {
            Ok(result) => result.operations,
            Err(error) => error.operations(),
        };
        frame.charge_operations(operations)?;
        result
            .map(|result| self.return_kind.decode(result.value))
            .map_err(|error| {
                Box::new(match error {
                    JitError::TooManyOperations { .. } => {
                        rhai::EvalAltResult::ErrorTooManyOperations(frame.position())
                    }
                    JitError::Arithmetic {
                        pc, left, right, ..
                    } => return frame.scalar_arithmetic_error(pc, left, right),
                    error => rhai::EvalAltResult::ErrorRuntime(
                        error.to_string().into(),
                        frame.position(),
                    ),
                })
            })
    }
}

impl CompiledChunk {
    pub(crate) fn allocation_size(&self) -> usize {
        self.buffer.size()
    }
    pub(crate) fn fits_scope(&self, existing: usize, maximum: usize) -> bool {
        existing.saturating_add(self.max_locals) <= maximum
    }
    pub(crate) fn guard_matches(&self, arguments: &[Dynamic]) -> bool {
        arguments.len() == self.parameters
            && self
                .parameter_kinds
                .iter()
                .zip(arguments)
                .all(|(kind, value)| kind.encode(value).is_some())
    }
    pub const fn code_size(&self) -> usize {
        self.code_size
    }

    /// Execute already-unboxed integer arguments.
    ///
    /// # Errors
    ///
    /// Returns an error for an arity mismatch, an exhausted operation budget,
    /// checked arithmetic failure, or an invalid native status.
    pub fn execute(
        &self,
        arguments: &[i64],
        max_operations: u64,
    ) -> Result<JitExecution, JitError> {
        if self.return_kind != ValueKind::Int
            || self
                .parameter_kinds
                .iter()
                .any(|kind| *kind != ValueKind::Int)
        {
            return Err(JitError::InvalidTyped(
                "integer API cannot execute a non-integer specialization".into(),
            ));
        }
        self.execute_bits(arguments, max_operations)
    }

    fn execute_bits(
        &self,
        arguments: &[i64],
        max_operations: u64,
    ) -> Result<JitExecution, JitError> {
        if arguments.len() != self.parameters {
            return Err(JitError::ParameterCount {
                expected: self.parameters,
                found: arguments.len(),
            });
        }
        let mut frame = JitFrame::new(arguments, max_operations);
        let value = invoke(&self.buffer, self.entry, &mut frame);
        match frame.status {
            STATUS_OK => Ok(JitExecution {
                value,
                operations: frame.operations,
            }),
            STATUS_TOO_MANY_OPERATIONS => Err(JitError::TooManyOperations {
                limit: max_operations,
                operations: frame.operations,
            }),
            STATUS_ARITHMETIC => Err(JitError::Arithmetic {
                operations: frame.operations,
                pc: u32::try_from(frame.error_pc).unwrap_or(u32::MAX),
                left: frame.error_left,
                right: frame.error_right,
            }),
            status => Err(JitError::UnknownStatus {
                status,
                operations: frame.operations,
            }),
        }
    }

    /// Execute when all Rhai arguments satisfy the integer guard, otherwise
    /// call the original Grain function.
    ///
    /// # Errors
    ///
    /// Returns either a native execution error after the guard matched or the
    /// error produced by the Grain fallback.
    pub fn execute_guarded(
        &self,
        engine: &Engine,
        program: &rhai::grain::Program<'_>,
        arguments: Vec<Dynamic>,
        max_operations: u64,
    ) -> Result<GuardedExecution, GuardedError> {
        if let Some(execution) = self.execute_if_guard_matches(&arguments, max_operations) {
            return execution.map(GuardedExecution::jit).map_err(Into::into);
        }

        rhai::grain::Vm::new(engine)
            .call_fn_with_options::<i64>(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                program,
                &self.function,
                arguments,
            )
            .map(GuardedExecution::grain)
            .map_err(GuardedError::Grain)
    }

    /// Execute borrowed Rhai arguments when the integer specialization guard
    /// matches, leaving fallback to the caller on a miss.
    pub fn execute_if_guard_matches(
        &self,
        arguments: &[Dynamic],
        max_operations: u64,
    ) -> Option<Result<JitExecution, JitError>> {
        if arguments.len() != self.parameters {
            return None;
        }

        let mut integers = [0_i64; MAX_LOCALS];
        for (index, argument) in arguments.iter().enumerate() {
            integers[index] = argument.as_int().ok()?;
        }
        Some(self.execute(&integers[..self.parameters], max_operations))
    }
}

#[derive(Debug, Error)]
pub enum JitError {
    #[error("invalid typed input: {0}")]
    InvalidTyped(String),
    #[error("expected {expected} integer arguments, found {found}")]
    ParameterCount { expected: usize, found: usize },
    #[error("local slot {0} exceeds the experiment limit of {MAX_LOCALS}")]
    TooManyLocals(usize),
    #[error("control-flow paths disagree about local scope depth at bytecode pc {pc}")]
    ScopeMismatch { pc: u32 },
    #[error("bytecode pc {0} is not a valid branch target")]
    MissingTarget(u32),
    #[error("bytecode pc {0} is unreachable in the typed chunk")]
    Unreachable(u32),
    #[error("constant {0} cannot be encoded by the narrow AArch64 experiment")]
    Constant(i64),
    #[error("dynasm could not allocate or finalize executable memory: {0}")]
    Assembly(String),
    #[error("the JIT operation limit {limit} was exceeded")]
    TooManyOperations { limit: u64, operations: u64 },
    #[error("the JIT detected invalid integer arithmetic")]
    Arithmetic {
        operations: u64,
        pc: u32,
        left: i64,
        right: i64,
    },
    #[error("the JIT returned unknown status {status}")]
    UnknownStatus { status: u64, operations: u64 },
}

impl JitError {
    /// Operations executed before a native runtime failure.
    pub const fn operations(&self) -> u64 {
        match *self {
            Self::TooManyOperations { operations, .. }
            | Self::Arithmetic { operations, .. }
            | Self::UnknownStatus { operations, .. } => operations,
            _ => 0,
        }
    }
}

/// Compile one verified typed chunk to `AArch64` code.
///
/// # Errors
///
/// Returns an error when the chunk exceeds the narrow local/constant limits,
/// has invalid control-flow or scope state, or executable memory cannot be
/// assembled and finalized.
pub fn compile(chunk: &TypedChunk) -> Result<CompiledChunk, JitError> {
    let validated = chunk
        .revalidate()
        .map_err(|error| JitError::InvalidTyped(error.to_string()))?;
    let chunk = &validated;
    if chunk.parameters > MAX_LOCALS {
        return Err(JitError::TooManyLocals(chunk.parameters));
    }
    let depths = scope_depths(chunk)?;
    validate_locals(chunk, &depths)?;

    let mut assembler = Assembler::new().map_err(|error| JitError::Assembly(error.to_string()))?;
    let entry = assembler.offset();
    let labels = chunk
        .instructions
        .iter()
        .map(|instruction| (instruction.pc, assembler.new_dynamic_label()))
        .collect::<BTreeMap<_, _>>();
    let too_many_operations = assembler.new_dynamic_label();
    let arithmetic_error = assembler.new_dynamic_label();
    let mut error_sites = Vec::new();

    let operations_offset = u32::try_from(std::mem::offset_of!(JitFrame, operations))
        .map_err(|_| JitError::TooManyLocals(MAX_LOCALS))?;
    let max_operations_offset = u32::try_from(std::mem::offset_of!(JitFrame, max_operations))
        .map_err(|_| JitError::TooManyLocals(MAX_LOCALS))?;
    let status_offset = u32::try_from(std::mem::offset_of!(JitFrame, status))
        .map_err(|_| JitError::TooManyLocals(MAX_LOCALS))?;

    dynasm!(assembler
        ; .arch aarch64
        ; mov x8, x0
        ; mov x9, sp
        ; mov x0, 0
        ; str x0, [x8, #status_offset]
    );

    for (index, instruction) in chunk.instructions.iter().enumerate() {
        let label = labels[&instruction.pc];
        dynasm!(assembler
            ; .arch aarch64
            ; =>label
        );
        let kinds = chunk.binary_kinds(index);
        let checked_integer = checked_integer_operation(instruction, kinds);
        let failure = if checked_integer {
            let label = assembler.new_dynamic_label();
            error_sites.push((label, instruction.pc));
            label
        } else {
            arithmetic_error
        };
        emit_instruction(
            &mut assembler,
            instruction,
            depths[index],
            &labels,
            too_many_operations,
            failure,
            operations_offset,
            max_operations_offset,
            chunk.binary_kinds(index),
        )?;
    }

    emit_error_sites(&mut assembler, &error_sites, arithmetic_error)?;

    dynasm!(assembler
        ; .arch aarch64
        ; =>too_many_operations
        ; mov x0, STATUS_TOO_MANY_OPERATIONS
        ; str x0, [x8, #status_offset]
        ; mov x0, 0
        ; mov sp, x9
        ; ret
        ; =>arithmetic_error
        ; mov x0, STATUS_ARITHMETIC
        ; str x0, [x8, #status_offset]
        ; mov x0, 0
        ; mov sp, x9
        ; ret
    );

    let buffer = assembler
        .finalize()
        .map_err(|error| JitError::Assembly(format!("{error:?}")))?;
    let code_size = buffer.len();
    Ok(CompiledChunk {
        buffer,
        entry,
        code_size,
        function: chunk.function.clone(),
        parameters: chunk.parameters,
        parameter_kinds: chunk.parameter_kinds.clone(),
        return_kind: chunk.return_kind,
        max_locals: chunk.max_locals(),
    })
}

#[expect(
    unsafe_code,
    reason = "the experiment must enter executable memory produced and owned by dynasmrt"
)]
fn invoke(buffer: &ExecutableBuffer, entry: AssemblyOffset, frame: &mut JitFrame) -> i64 {
    type Entry = extern "C" fn(*mut JitFrame) -> i64;
    // SAFETY: `entry` is recorded before emitting one complete AAPCS64 function,
    // `buffer` owns that executable memory for the duration of this call, and
    // the generated function accepts exactly one valid `JitFrame` pointer.
    let function = unsafe { std::mem::transmute::<*const u8, Entry>(buffer.ptr(entry)) };
    function(frame)
}

#[expect(
    clippy::too_many_arguments,
    reason = "the emitter receives the explicit labels and frame offsets it may branch to or access"
)]
fn emit_instruction(
    assembler: &mut Assembler,
    instruction: &Instruction,
    scope_depth: u16,
    labels: &BTreeMap<u32, DynamicLabel>,
    too_many_operations: DynamicLabel,
    arithmetic_error: DynamicLabel,
    operations_offset: u32,
    max_operations_offset: u32,
    operand_kinds: Option<(ValueKind, ValueKind)>,
) -> Result<(), JitError> {
    match instruction.op {
        TypedOp::Constant(value) => {
            let value = value.encoded();
            crate::managed_aarch64::load_immediate(
                assembler,
                0,
                u64::from_ne_bytes(value.to_ne_bytes()),
            );
            push(assembler, 0);
        }
        TypedOp::Unit | TypedOp::Boolean(_) => {
            let value = matches!(instruction.op, TypedOp::Boolean(true));
            dynasm!(assembler
                ; .arch aarch64
                ; mov x0, u64::from(value)
            );
            push(assembler, 0);
        }
        TypedOp::LoadLocal(slot) => {
            load_local(assembler, slot, 0)?;
            push(assembler, 0);
        }
        TypedOp::StoreLocal(slot) => {
            pop(assembler, 0);
            store_local(assembler, slot, 0)?;
        }
        TypedOp::AssignLocal { slot, op } => {
            pop(assembler, 0);
            if let Some(op) = op {
                load_local(assembler, slot, 1)?;
                emit_typed_binary(assembler, op, arithmetic_error, operand_kinds);
            }
            store_local(assembler, slot, 0)?;
        }
        TypedOp::DeclareLocal => {
            pop(assembler, 0);
            store_local(assembler, scope_depth, 0)?;
        }
        TypedOp::Pop => pop(assembler, 0),
        TypedOp::Jump(target) => {
            if target <= instruction.pc {
                emit_tick(
                    assembler,
                    operations_offset,
                    max_operations_offset,
                    too_many_operations,
                );
            }
            let target = target_label(labels, target)?;
            dynasm!(assembler
                ; .arch aarch64
                ; b =>target
            );
        }
        TypedOp::JumpIfTrue(target) | TypedOp::JumpIfFalse(target) => {
            pop(assembler, 0);
            let skip = assembler.new_dynamic_label();
            if matches!(instruction.op, TypedOp::JumpIfTrue(_)) {
                dynasm!(assembler; .arch aarch64; cbz x0, =>skip);
            } else {
                dynasm!(assembler; .arch aarch64; cbnz x0, =>skip);
            }
            if target <= instruction.pc {
                emit_tick(
                    assembler,
                    operations_offset,
                    max_operations_offset,
                    too_many_operations,
                );
            }
            let target = target_label(labels, target)?;
            dynasm!(assembler; .arch aarch64; b =>target; =>skip);
        }
        TypedOp::Binary(op) => {
            pop(assembler, 0);
            pop(assembler, 1);
            emit_typed_binary(assembler, op, arithmetic_error, operand_kinds);
            push(assembler, 0);
        }
        TypedOp::UnwindTo(_) | TypedOp::Checkpoint => {}
        TypedOp::Tick => {
            emit_tick(
                assembler,
                operations_offset,
                max_operations_offset,
                too_many_operations,
            );
        }
        TypedOp::Return => {
            pop(assembler, 0);
            dynasm!(assembler
                ; .arch aarch64
                ; mov sp, x9
                ; ret
            );
        }
    }
    Ok(())
}

fn emit_binary(assembler: &mut Assembler, op: BinaryOp, arithmetic_error: DynamicLabel) {
    if matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder
    ) {
        dynasm!(assembler; .arch aarch64; mov x5, x0);
    }
    match op {
        BinaryOp::Add => dynasm!(assembler
            ; .arch aarch64
            ; adds x0, x1, x0
            ; b.vs =>arithmetic_error
        ),
        BinaryOp::Subtract => dynasm!(assembler
            ; .arch aarch64
            ; subs x0, x1, x0
            ; b.vs =>arithmetic_error
        ),
        BinaryOp::Multiply => dynasm!(assembler
            ; .arch aarch64
            ; mul x2, x1, x0
            ; smulh x3, x1, x0
            ; asr x4, x2, 63
            ; cmp x3, x4
            ; b.ne =>arithmetic_error
            ; mov x0, x2
        ),
        BinaryOp::Divide => {
            emit_division_guard(assembler, arithmetic_error);
            dynasm!(assembler
                ; .arch aarch64
                ; sdiv x0, x1, x0
            );
        }
        BinaryOp::Remainder => {
            emit_division_guard(assembler, arithmetic_error);
            dynasm!(assembler
                ; .arch aarch64
                ; sdiv x2, x1, x0
                ; msub x0, x2, x0, x1
            );
        }
        BinaryOp::Equal => dynasm!(assembler
            ; .arch aarch64
            ; cmp x1, x0
            ; cset x0, eq
        ),
        BinaryOp::NotEqual => dynasm!(assembler
            ; .arch aarch64
            ; cmp x1, x0
            ; cset x0, ne
        ),
        BinaryOp::Less => dynasm!(assembler
            ; .arch aarch64
            ; cmp x1, x0
            ; cset x0, lt
        ),
        BinaryOp::LessOrEqual => dynasm!(assembler
            ; .arch aarch64
            ; cmp x1, x0
            ; cset x0, le
        ),
        BinaryOp::Greater => dynasm!(assembler
            ; .arch aarch64
            ; cmp x1, x0
            ; cset x0, gt
        ),
        BinaryOp::GreaterOrEqual => dynasm!(assembler
            ; .arch aarch64
            ; cmp x1, x0
            ; cset x0, ge
        ),
    }
}

fn emit_tick(
    assembler: &mut Assembler,
    operations_offset: u32,
    max_operations_offset: u32,
    too_many_operations: DynamicLabel,
) {
    dynasm!(assembler
        ; .arch aarch64
        ; ldr x0, [x8, #operations_offset]
        ; add x0, x0, 1
        ; str x0, [x8, #operations_offset]
        ; ldr x1, [x8, #max_operations_offset]
        ; cbz x1, >unlimited
        ; cmp x0, x1
        ; b.hi =>too_many_operations
        ; unlimited:
    );
}

fn checked_integer_operation(
    instruction: &Instruction,
    kinds: Option<(ValueKind, ValueKind)>,
) -> bool {
    let (TypedOp::Binary(op) | TypedOp::AssignLocal { op: Some(op), .. }) = instruction.op else {
        return false;
    };
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder
    ) && !kinds.is_some_and(|(a, b)| a == ValueKind::Float || b == ValueKind::Float)
}

fn emit_error_sites(
    assembler: &mut Assembler,
    sites: &[(DynamicLabel, u32)],
    failure: DynamicLabel,
) -> Result<(), JitError> {
    let pc_offset = u32::try_from(std::mem::offset_of!(JitFrame, error_pc))
        .map_err(|_| JitError::TooManyLocals(MAX_LOCALS))?;
    let left_offset = u32::try_from(std::mem::offset_of!(JitFrame, error_left))
        .map_err(|_| JitError::TooManyLocals(MAX_LOCALS))?;
    let right_offset = u32::try_from(std::mem::offset_of!(JitFrame, error_right))
        .map_err(|_| JitError::TooManyLocals(MAX_LOCALS))?;
    for &(label, pc) in sites {
        dynasm!(assembler; .arch aarch64; =>label);
        crate::managed_aarch64::load_immediate(assembler, 2, u64::from(pc));
        dynasm!(assembler
            ; .arch aarch64
            ; str x2, [x8, #pc_offset]
            ; str x1, [x8, #left_offset]
            ; str x5, [x8, #right_offset]
            ; b =>failure
        );
    }
    Ok(())
}

fn emit_typed_binary(
    assembler: &mut Assembler,
    op: BinaryOp,
    arithmetic_error: DynamicLabel,
    kinds: Option<(ValueKind, ValueKind)>,
) {
    let (left, right) = kinds.unwrap_or((ValueKind::Int, ValueKind::Int));
    if left != ValueKind::Float && right != ValueKind::Float {
        emit_binary(assembler, op, arithmetic_error);
        return;
    }
    if left == ValueKind::Int {
        dynasm!(assembler; .arch aarch64; scvtf d1, x1);
    } else {
        dynasm!(assembler; .arch aarch64; fmov d1, x1);
    }
    if right == ValueKind::Int {
        dynasm!(assembler; .arch aarch64; scvtf d0, x0);
    } else {
        dynasm!(assembler; .arch aarch64; fmov d0, x0);
    }
    match op {
        BinaryOp::Add => dynasm!(assembler; .arch aarch64; fadd d0, d1, d0; fmov x0, d0),
        BinaryOp::Subtract => dynasm!(assembler; .arch aarch64; fsub d0, d1, d0; fmov x0, d0),
        BinaryOp::Multiply => dynasm!(assembler; .arch aarch64; fmul d0, d1, d0; fmov x0, d0),
        BinaryOp::Divide => dynasm!(assembler; .arch aarch64; fdiv d0, d1, d0; fmov x0, d0),
        BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::Less
        | BinaryOp::LessOrEqual
        | BinaryOp::Greater
        | BinaryOp::GreaterOrEqual => emit_float_comparison(assembler, op),
        BinaryOp::Remainder => dynasm!(assembler; .arch aarch64; b =>arithmetic_error),
    }
}

fn emit_float_comparison(assembler: &mut Assembler, op: BinaryOp) {
    // Rhai's checked build uses relative epsilon, including its explicit zero
    // denominator case. Raw IEEE comparisons differ for near-equal values and
    // even for NaN paired with zero; preserve the actual language semantics.
    let product_zero = assembler.new_dynamic_label();
    let denominator_ready = assembler.new_dynamic_label();
    let denominator_zero = assembler.new_dynamic_label();
    let done = assembler.new_dynamic_label();
    dynasm!(assembler
        ; .arch aarch64
        ; fmul d2, d1, d0
        ; fcmp d2, #0.0
        ; b.eq =>product_zero
        ; fabs d3, d1
        ; fabs d4, d0
        ; fmaxnm d2, d3, d4
        ; b =>denominator_ready
        ; =>product_zero
        ; fmov d2, #1.0
        ; =>denominator_ready
        ; fcmp d2, #0.0
        ; b.eq =>denominator_zero
    );
    if matches!(op, BinaryOp::Less | BinaryOp::LessOrEqual) {
        dynasm!(assembler; .arch aarch64; fsub d3, d0, d1);
    } else {
        dynasm!(assembler; .arch aarch64; fsub d3, d1, d0);
    }
    if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
        dynasm!(assembler; .arch aarch64; fabs d3, d3);
    }
    let epsilon = if matches!(op, BinaryOp::GreaterOrEqual | BinaryOp::LessOrEqual) {
        -f64::EPSILON
    } else {
        f64::EPSILON
    };
    crate::managed_aarch64::load_immediate(assembler, 2, epsilon.to_bits());
    dynasm!(assembler; .arch aarch64; fmov d4, x2; fdiv d3, d3, d2; fcmp d3, d4);
    if op == BinaryOp::Equal {
        dynasm!(assembler; .arch aarch64; cset x0, ls);
    } else {
        dynasm!(assembler; .arch aarch64; cset x0, gt);
    }
    let zero_result = u64::from(matches!(
        op,
        BinaryOp::Equal | BinaryOp::GreaterOrEqual | BinaryOp::LessOrEqual
    ));
    dynasm!(assembler; .arch aarch64; b =>done; =>denominator_zero; mov x0, zero_result; =>done);
}

fn emit_division_guard(assembler: &mut Assembler, arithmetic_error: DynamicLabel) {
    dynasm!(assembler
        ; .arch aarch64
        ; cbz x0, =>arithmetic_error
        ; cmn x0, 1
        ; b.ne >division_safe
        ; mov x2, 1
        ; lsl x2, x2, 63
        ; cmp x1, x2
        ; b.eq =>arithmetic_error
        ; division_safe:
    );
}

fn push(assembler: &mut Assembler, register: u8) {
    dynasm!(assembler
        ; .arch aarch64
        ; sub sp, sp, 16
        ; str X(register), [sp]
    );
}

fn pop(assembler: &mut Assembler, register: u8) {
    dynasm!(assembler
        ; .arch aarch64
        ; ldr X(register), [sp]
        ; add sp, sp, 16
    );
}

fn load_local(assembler: &mut Assembler, slot: u16, register: u8) -> Result<(), JitError> {
    let offset = local_offset(slot)?;
    dynasm!(assembler
        ; .arch aarch64
        ; ldr X(register), [x8, #offset]
    );
    Ok(())
}

fn store_local(assembler: &mut Assembler, slot: u16, register: u8) -> Result<(), JitError> {
    let offset = local_offset(slot)?;
    dynasm!(assembler
        ; .arch aarch64
        ; str X(register), [x8, #offset]
    );
    Ok(())
}

fn local_offset(slot: u16) -> Result<u32, JitError> {
    if slot as usize >= MAX_LOCALS {
        return Err(JitError::TooManyLocals(slot as usize));
    }
    u32::try_from(slot as usize * size_of::<i64>())
        .map_err(|_| JitError::TooManyLocals(slot as usize))
}

fn target_label(
    labels: &BTreeMap<u32, DynamicLabel>,
    target: u32,
) -> Result<DynamicLabel, JitError> {
    labels
        .get(&target)
        .copied()
        .ok_or(JitError::MissingTarget(target))
}

fn validate_locals(chunk: &TypedChunk, depths: &[u16]) -> Result<(), JitError> {
    for (instruction, depth) in chunk.instructions.iter().zip(depths) {
        let slot = match instruction.op {
            TypedOp::LoadLocal(slot)
            | TypedOp::StoreLocal(slot)
            | TypedOp::AssignLocal { slot, .. } => Some(slot as usize),
            TypedOp::DeclareLocal => Some(*depth as usize),
            _ => None,
        };
        if slot.is_some_and(|slot| slot >= MAX_LOCALS) {
            return Err(JitError::TooManyLocals(slot.unwrap_or(MAX_LOCALS)));
        }
    }
    Ok(())
}

fn scope_depths(chunk: &TypedChunk) -> Result<Vec<u16>, JitError> {
    let targets = chunk
        .instructions
        .iter()
        .enumerate()
        .map(|(index, instruction)| (instruction.pc, index))
        .collect::<BTreeMap<_, _>>();
    let mut depths = vec![None; chunk.instructions.len()];
    if depths.is_empty() {
        return Ok(Vec::new());
    }
    depths[0] = Some(
        u16::try_from(chunk.parameters).map_err(|_| JitError::TooManyLocals(chunk.parameters))?,
    );
    let mut pending = VecDeque::from([0_usize]);

    while let Some(index) = pending.pop_front() {
        let instruction = &chunk.instructions[index];
        let depth = depths[index].ok_or(JitError::Unreachable(instruction.pc))?;
        let next_depth = match instruction.op {
            TypedOp::DeclareLocal => depth
                .checked_add(1)
                .ok_or(JitError::TooManyLocals(MAX_LOCALS))?,
            TypedOp::UnwindTo(depth) => depth,
            _ => depth,
        };

        let mut successors = Vec::with_capacity(2);
        match instruction.op {
            TypedOp::Jump(target) => successors.push(target_index(&targets, target)?),
            TypedOp::JumpIfTrue(target) | TypedOp::JumpIfFalse(target) => {
                successors.push(target_index(&targets, target)?);
                if index + 1 < chunk.instructions.len() {
                    successors.push(index + 1);
                }
            }
            TypedOp::Return => {}
            _ if index + 1 < chunk.instructions.len() => successors.push(index + 1),
            _ => {}
        }

        for successor in successors {
            match depths[successor] {
                Some(existing) if existing != next_depth => {
                    return Err(JitError::ScopeMismatch {
                        pc: chunk.instructions[successor].pc,
                    });
                }
                Some(_) => {}
                None => {
                    depths[successor] = Some(next_depth);
                    pending.push_back(successor);
                }
            }
        }
    }

    depths
        .into_iter()
        .zip(&chunk.instructions)
        .map(|(depth, instruction)| depth.ok_or(JitError::Unreachable(instruction.pc)))
        .collect()
}

fn target_index(targets: &BTreeMap<u32, usize>, target: u32) -> Result<usize, JitError> {
    targets
        .get(&target)
        .copied()
        .ok_or(JitError::MissingTarget(target))
}
