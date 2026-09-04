//! Baseline native control flow with Rust-owned managed values.
//!
//! Each operation calls a statically specialized Rhai helper. Native branches
//! replace interpreter dispatch; helpers retain Rhai ownership, receiver and
//! error semantics. This is distinct from the unboxed numeric fast lane.

#![expect(
    clippy::unnecessary_semicolon,
    clippy::used_underscore_binding,
    clippy::useless_conversion,
    reason = "dynasm generates these constructs"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

use dynasmrt::aarch64::Assembler;
use dynasmrt::{AssemblyOffset, DynasmApi, DynasmLabelApi, ExecutableBuffer, dynasm};
use rhai::grain::bytecode::{Op, Root, Step, Tail, code::tag};
use rhai::grain::{Function, NativeFrame, NativeFunction, NativeStep, Program};
use rhai::{Dynamic, EvalAltResult, Position};

type ResultValue = Result<Dynamic, Box<EvalAltResult>>;
type StepFn = unsafe extern "C" fn(*mut (), usize) -> usize;

/// Owns executable code but no invocation state or borrowed Rhai values.
pub struct CompiledManaged {
    buffer: ExecutableBuffer,
    entry: AssemblyOffset,
    // The machine code embeds pointers into these immutable, pinned sites.
    _map_reads: Box<[MapRead]>,
}

struct MapRead {
    pc: usize,
    slot: usize,
    key: String,
}

impl CompiledManaged {
    pub(crate) fn allocation_size(&self) -> usize {
        self.buffer.size()
    }
    pub fn code_size(&self) -> usize {
        self.buffer.len()
    }
}

struct Invocation<'frame, 'vm, 'engine, 'artifact, 'scope> {
    runtime: &'frame mut NativeFrame<'vm, 'engine, 'artifact, 'scope>,
    result: Option<ResultValue>,
}

impl NativeFunction for CompiledManaged {
    fn execute(&self, runtime: &mut NativeFrame<'_, '_, '_, '_>) -> ResultValue {
        let mut invocation = Invocation {
            runtime,
            result: None,
        };
        invoke(&self.buffer, self.entry, &mut invocation);
        invocation.result.unwrap_or_else(|| {
            Err(Box::new(EvalAltResult::ErrorRuntime(
                "native body returned no result".into(),
                Position::NONE,
            )))
        })
    }
}

#[expect(
    unsafe_code,
    reason = "one audited entry into owned AAPCS64 executable memory"
)]
fn invoke(
    buffer: &ExecutableBuffer,
    entry: AssemblyOffset,
    frame: &mut Invocation<'_, '_, '_, '_, '_>,
) {
    type Entry = unsafe extern "C" fn(*mut ());
    // SAFETY: the emitted function uses AAPCS64 and accepts this opaque frame.
    // The immutable code owner and the stack frame both outlive the call. Every
    // helper catches Rust unwinding before returning through generated code.
    unsafe {
        let function = std::mem::transmute::<*const u8, Entry>(buffer.ptr(entry));
        function(std::ptr::from_mut(frame).cast());
    }
}

#[expect(
    unsafe_code,
    reason = "generated code passes the live Invocation supplied by invoke"
)]
unsafe extern "C" fn step<const TAG: u16>(opaque: *mut (), pc: usize) -> usize {
    // SAFETY: only our emitted code calls this helper, with invoke's pointer.
    let frame = unsafe { &mut *opaque.cast::<Invocation<'_, '_, '_, '_, '_>>() };
    let result = catch_unwind(AssertUnwindSafe(|| frame.runtime.step::<TAG>(pc)));
    finish_step(frame, result)
}

#[expect(
    unsafe_code,
    reason = "same live invocation pointer and unwind fence as step"
)]
unsafe extern "C" fn pair<const FIRST: u16, const SECOND: u16>(
    opaque: *mut (),
    pc: usize,
) -> usize {
    // SAFETY: emitted code supplies invoke's live frame, as for the single helper.
    let frame = unsafe { &mut *opaque.cast::<Invocation<'_, '_, '_, '_, '_>>() };
    let result = catch_unwind(AssertUnwindSafe(|| {
        match frame.runtime.step::<FIRST>(pc)? {
            NativeStep::Continue(next) => frame.runtime.step::<SECOND>(next),
            returned @ NativeStep::Return(_) => Ok(returned),
        }
    }));
    finish_step(frame, result)
}

#[expect(
    unsafe_code,
    reason = "the executable owns each immutable MapRead site throughout invocation"
)]
unsafe extern "C" fn map_read<const NEXT: u16>(opaque: *mut (), site: usize) -> usize {
    // SAFETY: the generated call embeds a pointer to its owner's boxed site,
    // and passes the same Invocation pointer as every other helper.
    let site = unsafe { &*(site as *const MapRead) };
    let frame = unsafe { &mut *opaque.cast::<Invocation<'_, '_, '_, '_, '_>>() };
    let result = catch_unwind(AssertUnwindSafe(|| {
        frame.runtime.set_position(site.pc);
        let value = frame
            .runtime
            .local(site.slot)
            .filter(|root| !root.is_shared())
            .and_then(|root| root.as_map_ref().ok()?.get(site.key.as_str()).cloned());
        let outcome = if let Some(value) = value {
            frame.runtime.push_value(value);
            NativeStep::Continue(site.pc + 3)
        } else {
            frame.runtime.step::<{ tag::CHAIN as u16 }>(site.pc)?
        };
        if NEXT < 256
            && let NativeStep::Continue(next) = outcome
        {
            return frame.runtime.step::<NEXT>(next);
        }
        Ok(outcome)
    }));
    finish_step(frame, result)
}

fn finish_step(
    frame: &mut Invocation<'_, '_, '_, '_, '_>,
    result: std::thread::Result<Result<NativeStep, Box<EvalAltResult>>>,
) -> usize {
    match result {
        Ok(Ok(NativeStep::Continue(next))) => next,
        Ok(Ok(NativeStep::Return(value))) => {
            frame.result = Some(Ok(value));
            usize::MAX
        }
        Ok(Err(error)) => {
            frame.result = Some(Err(error));
            usize::MAX
        }
        Err(payload) => {
            // A panic may have skipped evaluator cleanup. End this invocation
            // with an uncatchable system error instead of resuming a script
            // catch block with potentially incomplete runtime state.
            if let Err(second) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
                // A panic payload may itself have a panicking destructor. Do
                // not let that second unwind cross the generated-code boundary.
                std::mem::forget(second);
            }
            frame.result = Some(Err(Box::new(EvalAltResult::ErrorSystem(
                "panic in native runtime helper".into(),
                Box::new(std::io::Error::other("native runtime helper panicked")),
            ))));
            usize::MAX
        }
    }
}

fn paired_helper(first: u8, second: u8) -> Option<StepFn> {
    macro_rules! pairs {
        ($(($first:ident, $second:ident)),* $(,)?) => {
            match (first, second) {
                $((tag::$first, tag::$second) => Some(pair::<{tag::$first as u16}, {tag::$second as u16}> as StepFn),)*
                _ => None,
            }
        };
    }
    pairs!(
        (CHAIN, ASSIGN_LOCAL_OP),
        (CONST, DECLARE_LOCAL),
        (CONST, DECLARE_CONST),
        (CONST, ASSIGN_LOCAL_OP),
        (LOAD_LOCAL, CHECK_MAP_SIZE),
        (CHAIN, CHECK_MAP_SIZE),
        (LOAD_LOCAL, INTERPOLATE_APPEND),
        (CONST, INTERPOLATE_APPEND),
        (LOAD_LOCAL, RETURN),
        (MAKE_MAP, RETURN),
        (CALL, DECLARE_LOCAL),
        (CONST, CALL),
        (LOAD_LOCAL, CALL_OP),
        (LOAD_LOCAL, LOAD_LOCAL),
        (CONST, CONST),
        (CHAIN, POP),
    )
}

fn helper(instruction: u8) -> Option<StepFn> {
    macro_rules! handlers {
        ($($name:ident),* $(,)?) => {
            match instruction {
                $(tag::$name => Some(step::<{ tag::$name as u16 }> as StepFn),)*
                _ => None,
            }
        };
    }
    handlers!(
        CONST,
        UNIT,
        FALSE,
        TRUE,
        LOAD_LOCAL,
        STORE_LOCAL,
        STORE_CONST,
        ASSIGN_LOCAL,
        ASSIGN_LOCAL_OP,
        DECLARE_LOCAL,
        DECLARE_CONST,
        POP,
        JUMP,
        JUMP_IF_TRUE,
        JUMP_IF_FALSE,
        SKIP_IF_NOT_UNIT,
        CALL,
        CALL_OP,
        UNWIND_TO,
        TICK,
        RETURN,
        CHAIN,
        SWITCH,
        EVAL_AST,
        EVAL_AST_KEEP,
        MAKE_ARRAY,
        LOAD_NAMED,
        ASSIGN_NAMED,
        ASSIGN_NAMED_OP,
        THROW,
        ITER_INIT,
        ITER_NEXT,
        ITER_NEXT_INDEXED,
        ITER_DROP,
        STORE_SHARED,
        INTERPOLATE_START,
        INTERPOLATE_APPEND,
        INTERPOLATE_END,
        MAKE_FN_PTR,
        MAKE_CLOSURE,
        CURRY,
        CALL_FN_PTR,
        CALL_FN_PTR_METHOD,
        CALL_FN_PTR_ON_LOCAL,
        CALL_FN_PTR_ON_NAMED,
        CHECKPOINT,
        CHECK_ARRAY_SIZE,
        CHECK_MAP_SIZE,
        MAKE_MAP,
        CALL_LOCAL_REF,
        CALL_NAMED_REF,
        ROTATE,
        STATEMENT,
    )
}

pub(crate) fn load_immediate(assembler: &mut Assembler, register: u8, value: u64) {
    let lo = (value & 0xffff) as u32;
    let mid = ((value >> 16) & 0xffff) as u32;
    let hi = ((value >> 32) & 0xffff) as u32;
    let top = ((value >> 48) & 0xffff) as u32;
    dynasm!(assembler
        ; .arch aarch64
        ; movz X(register), lo
    );
    if mid != 0 {
        dynasm!(assembler; .arch aarch64; movk X(register), mid, LSL #16);
    }
    if hi != 0 {
        dynasm!(assembler; .arch aarch64; movk X(register), hi, LSL #32);
    }
    if top != 0 {
        dynasm!(assembler; .arch aarch64; movk X(register), top, LSL #48);
    }
}

/// Compile verified bytecode without making assumptions about Dynamic layout.
///
/// # Errors
///
/// Rejects unsupported instructions before invocation and invalid branch
/// targets, or reports executable-memory allocation/finalization failure.
pub fn compile(program: &Program<'_>, function: &Function) -> Result<CompiledManaged, String> {
    let NativePlan {
        instructions,
        helpers,
        targets,
        mut omitted,
        map_reads,
    } = analyze(program, function)?;
    let map_sites = map_reads
        .iter()
        .map(|site| (site.pc, site))
        .collect::<BTreeMap<_, _>>();
    let mut assembler = Assembler::new().map_err(|error| error.to_string())?;
    let entry = assembler.offset();
    let done = assembler.new_dynamic_label();
    let labels = instructions
        .iter()
        .map(|(pc, _)| (*pc, assembler.new_dynamic_label()))
        .collect::<BTreeMap<_, _>>();
    dynasm!(assembler
        ; .arch aarch64
        ; stp x29, x30, [sp, #-32]!
        ; stp x19, x20, [sp, #16]
        ; mov x29, sp
        ; mov x19, x0
    );
    for (index, ((pc, op), mut helper)) in instructions.iter().zip(helpers).enumerate() {
        let label = labels[pc];
        dynasm!(assembler; .arch aarch64; =>label);
        if omitted.contains(&index) {
            continue;
        }
        let mut operand = *pc as u64;
        let map_site = map_sites.get(pc);
        if let Some(site) = map_site {
            helper = map_read::<256>;
            operand = std::ptr::from_ref(*site) as usize as u64;
        }
        if let Some((next, _)) = instructions.get(index + 1)
            && !targets.contains(next)
            && !omitted.contains(&(index + 1))
        {
            let pair = if map_site.is_some() {
                match program.code()[*next] {
                    tag::ASSIGN_LOCAL_OP => {
                        Some(map_read::<{ tag::ASSIGN_LOCAL_OP as u16 }> as StepFn)
                    }
                    tag::RETURN => Some(map_read::<{ tag::RETURN as u16 }> as StepFn),
                    _ => None,
                }
            } else {
                paired_helper(program.code()[*pc], program.code()[*next])
            };
            if let Some(pair) = pair {
                helper = pair;
                omitted.insert(index + 1);
            }
        }
        dynasm!(assembler; .arch aarch64; mov x0, x19);
        load_immediate(&mut assembler, 1, operand);
        load_immediate(&mut assembler, 16, helper as usize as u64);
        dynasm!(assembler
            ; .arch aarch64
            ; blr x16
            ; cmn x0, #1
            ; b.eq =>done
        );
        emit_control_flow(&mut assembler, op, program, &labels, done)?;
    }
    dynasm!(assembler
        ; .arch aarch64
        ; =>done
        ; ldp x19, x20, [sp, #16]
        ; ldp x29, x30, [sp], #32
        ; ret
    );
    let buffer = assembler.finalize().map_err(|error| format!("{error:?}"))?;
    drop(map_sites);
    Ok(CompiledManaged {
        buffer,
        entry,
        _map_reads: map_reads,
    })
}

struct NativePlan {
    instructions: Vec<(usize, Op)>,
    helpers: Vec<StepFn>,
    targets: BTreeSet<usize>,
    omitted: BTreeSet<usize>,
    map_reads: Box<[MapRead]>,
}

fn analyze(program: &Program<'_>, function: &Function) -> Result<NativePlan, String> {
    let name = program.view().name(function.name).unwrap_or("");
    if rhai::FnPtr::new(name).is_err() {
        return Err("anonymous function is outside the managed lane".into());
    }
    let instructions = function.chunk.ops(program.code()).collect::<Vec<_>>();
    for (pc, op) in &instructions {
        if let Op::EvalAst {
            residual: index, ..
        } = *op
            && matches!(
                program.view().residual(index),
                None | Some(rhai::Expr::Stmt(_))
            )
        {
            return Err(format!(
                "residual statement block at {pc} is outside the managed lane"
            ));
        }
        if let Op::MakeClosure(index) = *op {
            let name = program.view().name(index).unwrap_or("");
            if rhai::FnPtr::new(name).is_err() {
                return Err(format!(
                    "anonymous closure at {pc} is outside the managed lane"
                ));
            }
        }
    }
    let helpers = instructions
        .iter()
        .map(|(pc, op)| {
            helper(program.code()[*pc])
                .ok_or_else(|| format!("unsupported managed instruction at {pc}: {op:?}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut targets = BTreeSet::new();
    for (_, op) in &instructions {
        match *op {
            Op::Jump(target)
            | Op::JumpIfTrue { target }
            | Op::JumpIfFalse { target }
            | Op::SkipIfNotUnit { target }
            | Op::IterNext { exit: target, .. } => {
                targets.insert(target as usize);
            }
            Op::Switch(index) => {
                let table = program
                    .view()
                    .switch(index)
                    .ok_or_else(|| format!("missing switch {index}"))?;
                targets.extend(table.cases.iter().map(|case| case.target as usize));
                targets.extend(table.ranges.iter().map(|range| range.target as usize));
                targets.insert(table.default as usize);
            }
            _ => {}
        }
    }
    let mut omitted = BTreeSet::new();
    for (index, (_, op)) in instructions.iter().enumerate() {
        if !matches!(op, Op::Unit | Op::Bool(_)) {
            continue;
        }
        if let Some((pc, Op::Pop)) = instructions.get(index + 1) {
            if !targets.contains(pc) {
                omitted.extend([index, index + 1]);
            }
        } else if let (Some((middle, Op::UnwindTo(_))), Some((end, Op::Pop))) =
            (instructions.get(index + 1), instructions.get(index + 2))
            && !targets.contains(middle)
            && !targets.contains(end)
        {
            omitted.extend([index, index + 2]);
        }
    }
    let map_reads = map_read_sites(program, &instructions);
    Ok(NativePlan {
        instructions,
        helpers,
        targets,
        omitted,
        map_reads,
    })
}

fn map_read_sites(program: &Program<'_>, instructions: &[(usize, Op)]) -> Box<[MapRead]> {
    instructions
        .iter()
        .filter_map(|(pc, op)| {
            let Op::Chain(index) = *op else {
                return None;
            };
            let chain = program.view().chain(index)?;
            let Root::Local { slot, .. } = chain.root else {
                return None;
            };
            let [Step::Property { name, .. }] = chain.steps.as_slice() else {
                return None;
            };
            if chain.tail != Tail::Read || chain.operands != 0 {
                return None;
            }
            Some(MapRead {
                pc: *pc,
                slot: usize::from(slot),
                key: program.view().name(*name)?.to_owned(),
            })
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn emit_control_flow(
    assembler: &mut Assembler,
    op: &Op,
    program: &Program<'_>,
    labels: &BTreeMap<usize, dynasmrt::DynamicLabel>,
    done: dynasmrt::DynamicLabel,
) -> Result<(), String> {
    let target = match *op {
        Op::Jump(target)
        | Op::JumpIfTrue { target }
        | Op::JumpIfFalse { target }
        | Op::SkipIfNotUnit { target } => Some(target),
        Op::IterNext { exit, .. } => Some(exit),
        _ => None,
    };
    if let Op::Switch(index) = *op {
        let table = program
            .view()
            .switch(index)
            .ok_or_else(|| format!("missing switch {index}"))?;
        let targets = table
            .cases
            .iter()
            .map(|case| case.target)
            .chain(table.ranges.iter().map(|range| range.target))
            .chain([table.default])
            .collect::<std::collections::BTreeSet<_>>();
        for target in targets {
            let label = *labels
                .get(&(target as usize))
                .ok_or_else(|| format!("invalid switch target {target}"))?;
            load_immediate(assembler, 1, u64::from(target));
            dynasm!(assembler; .arch aarch64; cmp x0, x1; b.eq =>label);
        }
        dynasm!(assembler; .arch aarch64; b =>done);
    }
    if let Some(target) = target {
        let target_label = *labels
            .get(&(target as usize))
            .ok_or_else(|| format!("invalid native branch target {target}"))?;
        if matches!(op, Op::Jump(_)) {
            dynasm!(assembler; .arch aarch64; b =>target_label);
        } else {
            load_immediate(assembler, 1, u64::from(target));
            dynasm!(assembler; .arch aarch64; cmp x0, x1; b.eq =>target_label);
        }
    }
    Ok(())
}
