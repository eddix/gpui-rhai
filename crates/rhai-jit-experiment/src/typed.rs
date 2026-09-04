use std::collections::BTreeMap;

use rhai::grain::bytecode::Op;
use rhai::grain::{Function, Program, ProgramView};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoolValue {
    Unit,
    Bool(bool),
    Int(i64),
    /// IEEE-754 bits, retaining Eq even for NaN constants.
    Float(u64),
}

impl PoolValue {
    pub const fn encoded(self) -> i64 {
        match self {
            Self::Unit => 0,
            Self::Bool(value) => value as i64,
            Self::Int(value) => value,
            Self::Float(bits) => i64::from_ne_bytes(bits.to_ne_bytes()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

impl BinaryOp {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Subtract => "subtract",
            Self::Multiply => "multiply",
            Self::Divide => "divide",
            Self::Remainder => "remainder",
            Self::Equal => "equal",
            Self::NotEqual => "not_equal",
            Self::Less => "less",
            Self::LessOrEqual => "less_or_equal",
            Self::Greater => "greater",
            Self::GreaterOrEqual => "greater_or_equal",
        }
    }

    fn parse(syntax: &str) -> Result<Self, TypedError> {
        match syntax {
            "+" => Ok(Self::Add),
            "-" => Ok(Self::Subtract),
            "*" => Ok(Self::Multiply),
            "/" => Ok(Self::Divide),
            "%" => Ok(Self::Remainder),
            "==" => Ok(Self::Equal),
            "!=" => Ok(Self::NotEqual),
            "<" => Ok(Self::Less),
            "<=" => Ok(Self::LessOrEqual),
            ">" => Ok(Self::Greater),
            ">=" => Ok(Self::GreaterOrEqual),
            _ => Err(TypedError::UnsupportedOperator(syntax.to_owned())),
        }
    }

    fn apply(self, left: i64, right: i64) -> Result<i64, TypedError> {
        match self {
            Self::Add => left.checked_add(right),
            Self::Subtract => left.checked_sub(right),
            Self::Multiply => left.checked_mul(right),
            Self::Divide => left.checked_div(right),
            Self::Remainder => left.checked_rem(right),
            Self::Equal => return Ok(i64::from(left == right)),
            Self::NotEqual => return Ok(i64::from(left != right)),
            Self::Less => return Ok(i64::from(left < right)),
            Self::LessOrEqual => return Ok(i64::from(left <= right)),
            Self::Greater => return Ok(i64::from(left > right)),
            Self::GreaterOrEqual => return Ok(i64::from(left >= right)),
        }
        .ok_or(TypedError::Arithmetic)
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "Rhai mixed arithmetic converts integer operands to FLOAT"
    )]
    fn apply_typed(
        self,
        left: i64,
        right: i64,
        kinds: Option<(ValueKind, ValueKind)>,
    ) -> Result<i64, TypedError> {
        let (left_kind, right_kind) = kinds.unwrap_or((ValueKind::Int, ValueKind::Int));
        if left_kind != ValueKind::Float && right_kind != ValueKind::Float {
            return self.apply(left, right);
        }
        let decode = |value: i64, kind| {
            if kind == ValueKind::Float {
                f64::from_bits(u64::from_ne_bytes(value.to_ne_bytes()))
            } else {
                value as f64
            }
        };
        let (left, right) = (decode(left, left_kind), decode(right, right_kind));
        let value = match self {
            Self::Add => left + right,
            Self::Subtract => left - right,
            Self::Multiply => left * right,
            Self::Divide => left / right,
            Self::Remainder => left % right,
            Self::Equal
            | Self::NotEqual
            | Self::Less
            | Self::LessOrEqual
            | Self::Greater
            | Self::GreaterOrEqual => {
                return Ok(i64::from(float_comparison(self, left, right)));
            }
        };
        Ok(i64::from_ne_bytes(value.to_bits().to_ne_bytes()))
    }
}

fn float_comparison(op: BinaryOp, left: f64, right: f64) -> bool {
    let max = if left * right == 0.0 {
        1.0
    } else {
        left.abs().max(right.abs())
    };
    if max == 0.0 {
        return matches!(
            op,
            BinaryOp::Equal | BinaryOp::GreaterOrEqual | BinaryOp::LessOrEqual
        );
    }
    match op {
        BinaryOp::Equal => (left - right).abs() / max <= f64::EPSILON,
        BinaryOp::NotEqual => (left - right).abs() / max > f64::EPSILON,
        BinaryOp::Greater => (left - right) / max > f64::EPSILON,
        BinaryOp::GreaterOrEqual => (left - right) / max > -f64::EPSILON,
        BinaryOp::Less => (right - left) / max > f64::EPSILON,
        BinaryOp::LessOrEqual => (right - left) / max > -f64::EPSILON,
        _ => false,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypedOp {
    Constant(PoolValue),
    Unit,
    Boolean(bool),
    LoadLocal(u16),
    StoreLocal(u16),
    AssignLocal { slot: u16, op: Option<BinaryOp> },
    DeclareLocal,
    Pop,
    Jump(u32),
    JumpIfTrue(u32),
    JumpIfFalse(u32),
    Binary(BinaryOp),
    UnwindTo(u16),
    Tick,
    Checkpoint,
    Return,
}

impl TypedOp {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Constant(_) => "constant",
            Self::Unit => "unit",
            Self::Boolean(_) => "boolean",
            Self::LoadLocal(_) => "load_local",
            Self::StoreLocal(_) => "store_local",
            Self::AssignLocal { op: None, .. } => "assign_local",
            Self::AssignLocal { op: Some(_), .. } => "assign_local_op",
            Self::DeclareLocal => "declare_local",
            Self::Pop => "pop",
            Self::Jump(_) => "jump",
            Self::JumpIfTrue(_) => "jump_if_true",
            Self::JumpIfFalse(_) => "jump_if_false",
            Self::Binary(op) => op.name(),
            Self::UnwindTo(_) => "unwind_to",
            Self::Tick => "tick",
            Self::Checkpoint => "checkpoint",
            Self::Return => "return",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instruction {
    pub pc: u32,
    pub op: TypedOp,
}

#[derive(Clone, Debug)]
pub struct TypedChunk {
    pub function: String,
    pub parameters: usize,
    pub max_stack: u16,
    pub instructions: Vec<Instruction>,
    pc_to_index: BTreeMap<u32, usize>,
    pub(crate) parameter_kinds: Vec<ValueKind>,
    pub(crate) return_kind: ValueKind,
    states: Vec<Option<TypeState>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedExecution {
    pub value: i64,
    pub operations: u64,
    pub opcode_counts: BTreeMap<&'static str, u64>,
}

#[derive(Debug, Error)]
pub enum TypedError {
    #[error("the experiment requires exactly one lowered Grain function, found {0}")]
    FunctionCount(usize),
    #[error("expected {expected} integer arguments, found {found}")]
    ParameterCount { expected: usize, found: usize },
    #[error("name pool index {0} is missing")]
    MissingName(u32),
    #[error("constant pool index {0} is missing")]
    MissingConstant(u32),
    #[error("operator pool index {0} is missing")]
    MissingOperator(u32),
    #[error("assignment operator pool index {0} is missing")]
    MissingAssignOperator(u32),
    #[error("unsupported Grain instruction at bytecode pc {pc}: {op:?}")]
    UnsupportedInstruction { pc: usize, op: Op },
    #[error("unsupported typed operator `{0}`")]
    UnsupportedOperator(String),
    #[error("the typed operand stack underflowed")]
    StackUnderflow,
    #[error("local slot {slot} is outside a scope containing {len} values")]
    MissingLocal { slot: usize, len: usize },
    #[error("jump target {0} is not an instruction in this function")]
    MissingTarget(u32),
    #[error("typed integer arithmetic failed")]
    Arithmetic,
    #[error("operation limit {0} exceeded")]
    TooManyOperations(u64),
    #[error("the typed function returned without a value")]
    MissingReturnValue,
    #[error("typed specialization expected {expected} at bytecode pc {pc}, found {actual}")]
    TypeMismatch {
        pc: u32,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("control-flow paths carry different typed state into bytecode pc {0}")]
    TypeMerge(u32),
}

/// Lower the sole function in a Grain program into the typed experiment IR.
///
/// # Errors
///
/// Returns an error unless the program has exactly one function and every
/// reachable instruction and type-flow edge belongs to the supported subset.
pub fn lower(program: &Program<'_>) -> Result<TypedChunk, TypedError> {
    let view = program.view();
    let [function] = view.functions() else {
        return Err(TypedError::FunctionCount(view.functions().len()));
    };
    lower_function(program, function)
}

/// Lower one selected function from a multi-function Grain program.
///
/// # Errors
///
/// Returns an error for missing pools, unsupported instructions/operators,
/// invalid control flow, or a type-flow mismatch.
pub fn lower_function(
    program: &Program<'_>,
    function: &Function,
) -> Result<TypedChunk, TypedError> {
    let chunk = lower_scalar_function(
        program,
        function,
        &vec![ValueKind::Int; function.params.len()],
    )?;
    require(chunk.return_kind, ValueKind::Int, function.chunk.entry())?;
    Ok(chunk)
}

/// Lower scalar parameters with a proven, uniform scalar return type.
///
/// # Errors
///
/// Rejects unsupported operations and inconsistent type/control-flow merges.
pub fn lower_scalar_function(
    program: &Program<'_>,
    function: &Function,
    parameters: &[ValueKind],
) -> Result<TypedChunk, TypedError> {
    if parameters.len() != function.params.len() {
        return Err(TypedError::ParameterCount {
            expected: function.params.len(),
            found: parameters.len(),
        });
    }
    let view = program.view();
    let function_name = view
        .name(function.name)
        .map(str::to_owned)
        .ok_or(TypedError::MissingName(function.name))?;
    let instructions = function
        .chunk
        .ops(view.code())
        .map(|(pc, op)| lower_op(pc, op, view))
        .collect::<Result<Vec<_>, _>>()?;
    let pc_to_index = instructions
        .iter()
        .enumerate()
        .map(|(index, instruction)| (instruction.pc, index))
        .collect();

    let mut chunk = TypedChunk {
        function: function_name,
        parameters: function.params.len(),
        max_stack: function.chunk.max_stack(),
        instructions,
        pc_to_index,
        parameter_kinds: parameters.to_vec(),
        return_kind: ValueKind::Unit,
        states: Vec::new(),
    };
    chunk.states = validate_types(&chunk)?;
    let mut returns =
        chunk
            .instructions
            .iter()
            .zip(&chunk.states)
            .filter_map(|(instruction, state)| {
                matches!(instruction.op, TypedOp::Return)
                    .then(|| state.as_ref()?.stack.last().copied())
                    .flatten()
            });
    chunk.return_kind = returns.next().ok_or(TypedError::MissingReturnValue)?;
    if returns.any(|kind| kind != chunk.return_kind) {
        return Err(TypedError::TypeMerge(function.chunk.entry()));
    }
    Ok(chunk)
}

fn lower_op(pc: usize, op: Op, view: ProgramView<'_, '_>) -> Result<Instruction, TypedError> {
    let typed = match op {
        Op::Const(index) => TypedOp::Constant(
            view.constant(index)
                .and_then(pool_value)
                .ok_or(TypedError::MissingConstant(index))?,
        ),
        Op::Unit => TypedOp::Unit,
        Op::Bool(value) => TypedOp::Boolean(value),
        Op::LoadLocal(slot) => TypedOp::LoadLocal(slot),
        Op::StoreLocal {
            slot,
            is_const: false,
        } => TypedOp::StoreLocal(slot),
        Op::AssignLocal { slot, op, .. } => TypedOp::AssignLocal {
            slot,
            op: op
                .map(|index| {
                    let operator = view
                        .assign_operator(index)
                        .ok_or(TypedError::MissingAssignOperator(index))?;
                    BinaryOp::parse(operator.binary)
                })
                .transpose()?,
        },
        Op::DeclareLocal {
            is_const: false, ..
        } => TypedOp::DeclareLocal,
        Op::Pop => TypedOp::Pop,
        Op::Jump(target) => TypedOp::Jump(target),
        Op::JumpIfTrue { target } => TypedOp::JumpIfTrue(target),
        Op::JumpIfFalse { target } => TypedOp::JumpIfFalse(target),
        Op::Call {
            argc: 2,
            op: Some(index),
            ..
        } => TypedOp::Binary(BinaryOp::parse(
            view.operator(index)
                .ok_or(TypedError::MissingOperator(index))?,
        )?),
        Op::UnwindTo(depth) => TypedOp::UnwindTo(depth),
        Op::Tick => TypedOp::Tick,
        Op::Checkpoint => TypedOp::Checkpoint,
        Op::Return => TypedOp::Return,
        op => return Err(TypedError::UnsupportedInstruction { pc, op }),
    };
    Ok(Instruction {
        pc: u32::try_from(pc).map_err(|_| TypedError::MissingTarget(u32::MAX))?,
        op: typed,
    })
}

fn pool_value(value: &rhai::Dynamic) -> Option<PoolValue> {
    if value.is_shared() || value.is_read_only() {
        return None;
    }
    if value.is_unit() {
        Some(PoolValue::Unit)
    } else if let Ok(value) = value.as_bool() {
        Some(PoolValue::Bool(value))
    } else if let Ok(value) = value.as_int() {
        Some(PoolValue::Int(value))
    } else {
        value
            .as_float()
            .ok()
            .map(|value| PoolValue::Float(value.to_bits()))
    }
}

impl TypedChunk {
    pub(crate) fn revalidate(&self) -> Result<Self, TypedError> {
        let mut chunk = self.clone();
        chunk.pc_to_index = chunk
            .instructions
            .iter()
            .enumerate()
            .map(|(index, op)| (op.pc, index))
            .collect();
        if chunk.pc_to_index.len() != chunk.instructions.len() {
            return Err(TypedError::TypeMerge(0));
        }
        chunk.states = validate_types(&chunk)?;
        if chunk
            .states
            .iter()
            .flatten()
            .any(|state| state.stack.len() > 64 || state.locals.len() > 64)
        {
            return Err(TypedError::UnsupportedOperator(
                "scalar frame exceeds 64 slots".into(),
            ));
        }
        for (instruction, state) in chunk.instructions.iter().zip(&chunk.states) {
            if matches!(instruction.op, TypedOp::Return)
                && let Some(state) = state
            {
                require(
                    *state.stack.last().ok_or(TypedError::MissingReturnValue)?,
                    chunk.return_kind,
                    instruction.pc,
                )?;
            }
        }
        Ok(chunk)
    }

    pub(crate) fn max_locals(&self) -> usize {
        self.states
            .iter()
            .flatten()
            .map(|state| state.locals.len())
            .max()
            .unwrap_or(self.parameters)
    }
    pub(crate) fn binary_kinds(&self, index: usize) -> Option<(ValueKind, ValueKind)> {
        let state = self.states.get(index)?.as_ref()?;
        match self.instructions.get(index)?.op {
            TypedOp::Binary(_) => Some((
                *state.stack.get(state.stack.len().checked_sub(2)?)?,
                *state.stack.last()?,
            )),
            TypedOp::AssignLocal { slot, op: Some(_) } => {
                Some((*state.locals.get(usize::from(slot))?, *state.stack.last()?))
            }
            _ => None,
        }
    }
    /// Interpret the typed IR for parity and opcode tracing.
    ///
    /// # Errors
    ///
    /// Returns an error for an arity mismatch, malformed typed state, checked
    /// arithmetic failure, or an exhausted operation budget.
    pub fn execute(
        &self,
        arguments: &[i64],
        max_operations: u64,
    ) -> Result<TypedExecution, TypedError> {
        if self.return_kind != ValueKind::Int
            || self
                .parameter_kinds
                .iter()
                .any(|kind| *kind != ValueKind::Int)
        {
            return Err(TypedError::UnsupportedOperator(
                "integer trace called with a non-integer specialization".into(),
            ));
        }
        if arguments.len() != self.parameters {
            return Err(TypedError::ParameterCount {
                expected: self.parameters,
                found: arguments.len(),
            });
        }
        let mut locals = arguments.to_vec();
        let mut stack = Vec::with_capacity(self.max_stack as usize);
        let mut opcode_counts = BTreeMap::new();
        let mut operations = 0_u64;
        let mut index = 0_usize;

        loop {
            let instruction = self
                .instructions
                .get(index)
                .ok_or(TypedError::MissingTarget(u32::MAX))?;
            *opcode_counts.entry(instruction.op.name()).or_insert(0) += 1;

            match instruction.op {
                TypedOp::Constant(value) => stack.push(value.encoded()),
                TypedOp::Unit => stack.push(0),
                TypedOp::Boolean(value) => stack.push(i64::from(value)),
                TypedOp::LoadLocal(slot) => {
                    stack.push(*local(&locals, slot)?);
                }
                TypedOp::StoreLocal(slot) => {
                    let value = pop(&mut stack)?;
                    *local_mut(&mut locals, slot)? = value;
                }
                TypedOp::AssignLocal { slot, op } => {
                    let right = pop(&mut stack)?;
                    let target = local_mut(&mut locals, slot)?;
                    *target = op.map_or(Ok(right), |op| {
                        op.apply_typed(*target, right, self.binary_kinds(index))
                    })?;
                }
                TypedOp::DeclareLocal => locals.push(pop(&mut stack)?),
                TypedOp::Pop => {
                    let _ = pop(&mut stack)?;
                }
                TypedOp::Jump(target) => {
                    if target <= instruction.pc {
                        charge(&mut operations, max_operations)?;
                    }
                    index = self.target(target)?;
                    continue;
                }
                TypedOp::JumpIfTrue(target) => {
                    if pop(&mut stack)? != 0 {
                        if target <= instruction.pc {
                            charge(&mut operations, max_operations)?;
                        }
                        index = self.target(target)?;
                        continue;
                    }
                }
                TypedOp::JumpIfFalse(target) => {
                    if pop(&mut stack)? == 0 {
                        if target <= instruction.pc {
                            charge(&mut operations, max_operations)?;
                        }
                        index = self.target(target)?;
                        continue;
                    }
                }
                TypedOp::Binary(op) => {
                    let right = pop(&mut stack)?;
                    let left = pop(&mut stack)?;
                    stack.push(op.apply_typed(left, right, self.binary_kinds(index))?);
                }
                TypedOp::UnwindTo(depth) => locals.truncate(depth as usize),
                TypedOp::Tick => {
                    charge(&mut operations, max_operations)?;
                }
                TypedOp::Checkpoint => {}
                TypedOp::Return => {
                    return Ok(TypedExecution {
                        value: stack.pop().ok_or(TypedError::MissingReturnValue)?,
                        operations,
                        opcode_counts,
                    });
                }
            }
            index += 1;
        }
    }

    fn target(&self, pc: u32) -> Result<usize, TypedError> {
        self.pc_to_index
            .get(&pc)
            .copied()
            .ok_or(TypedError::MissingTarget(pc))
    }
}

fn pop(stack: &mut Vec<i64>) -> Result<i64, TypedError> {
    stack.pop().ok_or(TypedError::StackUnderflow)
}

fn charge(operations: &mut u64, maximum: u64) -> Result<(), TypedError> {
    *operations = operations.saturating_add(1);
    if maximum > 0 && *operations > maximum {
        Err(TypedError::TooManyOperations(maximum))
    } else {
        Ok(())
    }
}

fn local(locals: &[i64], slot: u16) -> Result<&i64, TypedError> {
    locals.get(slot as usize).ok_or(TypedError::MissingLocal {
        slot: slot as usize,
        len: locals.len(),
    })
}

fn local_mut(locals: &mut [i64], slot: u16) -> Result<&mut i64, TypedError> {
    let len = locals.len();
    locals
        .get_mut(slot as usize)
        .ok_or(TypedError::MissingLocal {
            slot: slot as usize,
            len,
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueKind {
    Unit,
    Bool,
    Int,
    Float,
}

impl ValueKind {
    pub fn of(value: &rhai::Dynamic) -> Option<Self> {
        if value.is_shared() || value.is_read_only() {
            return None;
        }
        if value.is_unit() {
            Some(Self::Unit)
        } else if value.as_bool().is_ok() {
            Some(Self::Bool)
        } else if value.as_int().is_ok() {
            Some(Self::Int)
        } else if value.as_float().is_ok() {
            Some(Self::Float)
        } else {
            None
        }
    }

    pub(crate) fn encode(self, value: &rhai::Dynamic) -> Option<i64> {
        if value.is_shared() || value.is_read_only() {
            return None;
        }
        match self {
            Self::Unit => value.is_unit().then_some(0),
            Self::Bool => value.as_bool().ok().map(i64::from),
            Self::Int => value.as_int().ok(),
            Self::Float => value
                .as_float()
                .ok()
                .map(|value| i64::from_ne_bytes(value.to_bits().to_ne_bytes())),
        }
    }

    pub(crate) fn decode(self, bits: i64) -> rhai::Dynamic {
        match self {
            Self::Unit => rhai::Dynamic::UNIT,
            Self::Bool => (bits != 0).into(),
            Self::Int => bits.into(),
            Self::Float => f64::from_bits(u64::from_ne_bytes(bits.to_ne_bytes())).into(),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Float => "float",
        }
    }
}

impl From<PoolValue> for ValueKind {
    fn from(value: PoolValue) -> Self {
        match value {
            PoolValue::Unit => Self::Unit,
            PoolValue::Bool(_) => Self::Bool,
            PoolValue::Int(_) => Self::Int,
            PoolValue::Float(_) => Self::Float,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TypeState {
    locals: Vec<ValueKind>,
    stack: Vec<ValueKind>,
}

fn validate_types(chunk: &TypedChunk) -> Result<Vec<Option<TypeState>>, TypedError> {
    let mut states = vec![None; chunk.instructions.len()];
    if states.is_empty() {
        return Ok(states);
    }
    states[0] = Some(TypeState {
        locals: chunk.parameter_kinds.clone(),
        stack: Vec::new(),
    });
    let mut pending = std::collections::VecDeque::from([0_usize]);

    while let Some(index) = pending.pop_front() {
        let instruction = &chunk.instructions[index];
        let mut state = states[index]
            .clone()
            .ok_or(TypedError::MissingTarget(instruction.pc))?;
        apply_type(&mut state, instruction)?;

        let mut successors = Vec::with_capacity(2);
        match instruction.op {
            TypedOp::Jump(target) => successors.push(chunk.target(target)?),
            TypedOp::JumpIfTrue(target) | TypedOp::JumpIfFalse(target) => {
                successors.push(chunk.target(target)?);
                if index + 1 < chunk.instructions.len() {
                    successors.push(index + 1);
                }
            }
            TypedOp::Return => {}
            _ if index + 1 < chunk.instructions.len() => successors.push(index + 1),
            _ => {}
        }

        for successor in successors {
            match &states[successor] {
                Some(existing) if *existing != state => {
                    return Err(TypedError::TypeMerge(chunk.instructions[successor].pc));
                }
                Some(_) => {}
                None => {
                    states[successor] = Some(state.clone());
                    pending.push_back(successor);
                }
            }
        }
    }
    Ok(states)
}

fn apply_type(state: &mut TypeState, instruction: &Instruction) -> Result<(), TypedError> {
    match instruction.op {
        TypedOp::Constant(value) => state.stack.push(value.into()),
        TypedOp::Unit => state.stack.push(ValueKind::Unit),
        TypedOp::Boolean(_) => state.stack.push(ValueKind::Bool),
        TypedOp::LoadLocal(slot) => {
            let value = type_local(&state.locals, slot, instruction.pc)?;
            state.stack.push(value);
        }
        TypedOp::StoreLocal(slot) => {
            let value = type_pop(&mut state.stack, instruction.pc)?;
            *type_local_mut(&mut state.locals, slot, instruction.pc)? = value;
        }
        TypedOp::AssignLocal { slot, op } => {
            let right = type_pop(&mut state.stack, instruction.pc)?;
            let target = type_local_mut(&mut state.locals, slot, instruction.pc)?;
            if let Some(op) = op {
                *target = binary_kind(op, *target, right, instruction.pc)?;
            } else {
                *target = right;
            }
        }
        TypedOp::DeclareLocal => {
            state
                .locals
                .push(type_pop(&mut state.stack, instruction.pc)?);
        }
        TypedOp::Pop => {
            let _ = type_pop(&mut state.stack, instruction.pc)?;
        }
        TypedOp::Jump(_) | TypedOp::Tick | TypedOp::Checkpoint => {}
        TypedOp::JumpIfTrue(_) | TypedOp::JumpIfFalse(_) => {
            let condition = type_pop(&mut state.stack, instruction.pc)?;
            require(condition, ValueKind::Bool, instruction.pc)?;
        }
        TypedOp::Binary(op) => {
            let right = type_pop(&mut state.stack, instruction.pc)?;
            let left = type_pop(&mut state.stack, instruction.pc)?;
            state
                .stack
                .push(binary_kind(op, left, right, instruction.pc)?);
        }
        TypedOp::UnwindTo(depth) => {
            if depth as usize > state.locals.len() {
                return Err(TypedError::MissingLocal {
                    slot: depth as usize,
                    len: state.locals.len(),
                });
            }
            state.locals.truncate(depth as usize);
        }
        TypedOp::Return => {
            let _ = type_pop(&mut state.stack, instruction.pc)?;
            if let Some(extra) = state.stack.last().copied() {
                return Err(TypedError::TypeMismatch {
                    pc: instruction.pc,
                    expected: "empty operand stack after return value",
                    actual: extra.name(),
                });
            }
        }
    }
    Ok(())
}

fn type_pop(stack: &mut Vec<ValueKind>, pc: u32) -> Result<ValueKind, TypedError> {
    stack.pop().ok_or(TypedError::TypeMismatch {
        pc,
        expected: "operand",
        actual: "empty stack",
    })
}

fn binary_kind(
    op: BinaryOp,
    left: ValueKind,
    right: ValueKind,
    pc: u32,
) -> Result<ValueKind, TypedError> {
    if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) && left == right {
        return Ok(ValueKind::Bool);
    }
    if !matches!(left, ValueKind::Int | ValueKind::Float)
        || !matches!(right, ValueKind::Int | ValueKind::Float)
    {
        return Err(TypedError::TypeMismatch {
            pc,
            expected: "numeric operands",
            actual: left.name(),
        });
    }
    let kind = if left == ValueKind::Float || right == ValueKind::Float {
        ValueKind::Float
    } else {
        ValueKind::Int
    };
    match op {
        BinaryOp::Remainder if kind == ValueKind::Float => {
            Err(TypedError::UnsupportedOperator("floating remainder".into()))
        }
        BinaryOp::Add
        | BinaryOp::Subtract
        | BinaryOp::Multiply
        | BinaryOp::Divide
        | BinaryOp::Remainder => Ok(kind),
        _ => Ok(ValueKind::Bool),
    }
}

fn type_local(locals: &[ValueKind], slot: u16, pc: u32) -> Result<ValueKind, TypedError> {
    locals
        .get(slot as usize)
        .copied()
        .ok_or(TypedError::TypeMismatch {
            pc,
            expected: "declared local",
            actual: "missing local",
        })
}

fn type_local_mut(
    locals: &mut [ValueKind],
    slot: u16,
    pc: u32,
) -> Result<&mut ValueKind, TypedError> {
    locals
        .get_mut(slot as usize)
        .ok_or(TypedError::TypeMismatch {
            pc,
            expected: "declared local",
            actual: "missing local",
        })
}

fn require(actual: ValueKind, expected: ValueKind, pc: u32) -> Result<(), TypedError> {
    if actual == expected {
        Ok(())
    } else {
        Err(TypedError::TypeMismatch {
            pc,
            expected: expected.name(),
            actual: actual.name(),
        })
    }
}
