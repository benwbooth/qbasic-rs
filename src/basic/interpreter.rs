//! BASIC interpreter with generator-based execution for clean yield/resume semantics

use crate::basic::graphics::GraphicsMode;
use crate::basic::parser::{BinOp, DimVar, Expr, PrintItem, Stmt, UnaryOp};
use async_recursion::async_recursion;
use genawaiter::rc::{Co, Gen};
use genawaiter::GeneratorState;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

/// Random number generator state (simple linear congruential generator)
fn rnd() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    static mut SEED: u64 = 0;
    static mut INITIALIZED: bool = false;

    unsafe {
        if !INITIALIZED {
            SEED = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;
            INITIALIZED = true;
        }
        // LCG parameters
        SEED = SEED.wrapping_mul(6364136223846793005).wrapping_add(1);
        (SEED >> 33) as f64 / (1u64 << 31) as f64
    }
}

/// Format a number for STR$ function
fn format_number(n: f64) -> String {
    if n == n.trunc() && n.abs() < 1e10 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

/// Convert CP437 (DOS) character code to Unicode
fn cp437_to_unicode(code: u8) -> char {
    match code {
        0 => ' ',
        1 => '☺', 2 => '☻', 3 => '♥', 4 => '♦', 5 => '♣', 6 => '♠', 7 => '•',
        8 => '◘', 9 => '○', 10 => '◙', 11 => '♂', 12 => '♀', 13 => '♪', 14 => '♫', 15 => '☼',
        16 => '►', 17 => '◄', 18 => '↕', 19 => '‼', 20 => '¶', 21 => '§', 22 => '▬', 23 => '↨',
        24 => '↑', 25 => '↓', 26 => '→', 27 => '←', 28 => '∟', 29 => '↔', 30 => '▲', 31 => '▼',
        32..=126 => code as char,
        127 => '⌂',
        128 => 'Ç', 129 => 'ü', 130 => 'é', 131 => 'â', 132 => 'ä', 133 => 'à', 134 => 'å', 135 => 'ç',
        136 => 'ê', 137 => 'ë', 138 => 'è', 139 => 'ï', 140 => 'î', 141 => 'ì', 142 => 'Ä', 143 => 'Å',
        144 => 'É', 145 => 'æ', 146 => 'Æ', 147 => 'ô', 148 => 'ö', 149 => 'ò', 150 => 'û', 151 => 'ù',
        152 => 'ÿ', 153 => 'Ö', 154 => 'Ü', 155 => '¢', 156 => '£', 157 => '¥', 158 => '₧', 159 => 'ƒ',
        160 => 'á', 161 => 'í', 162 => 'ó', 163 => 'ú', 164 => 'ñ', 165 => 'Ñ', 166 => 'ª', 167 => 'º',
        168 => '¿', 169 => '⌐', 170 => '¬', 171 => '½', 172 => '¼', 173 => '¡', 174 => '«', 175 => '»',
        176 => '░', 177 => '▒', 178 => '▓',
        179 => '│', 180 => '┤', 181 => '╡', 182 => '╢', 183 => '╖', 184 => '╕', 185 => '╣',
        186 => '║', 187 => '╗', 188 => '╝', 189 => '╜', 190 => '╛', 191 => '┐',
        192 => '└', 193 => '┴', 194 => '┬', 195 => '├', 196 => '─',
        197 => '┼', 198 => '╞', 199 => '╟', 200 => '╚', 201 => '╔', 202 => '╩', 203 => '╦',
        204 => '╠', 205 => '═', 206 => '╬', 207 => '╧', 208 => '╨', 209 => '╤', 210 => '╥',
        211 => '╙', 212 => '╘', 213 => '╒', 214 => '╓', 215 => '╫', 216 => '╪', 217 => '┘', 218 => '┌',
        219 => '█', 220 => '▄', 221 => '▌', 222 => '▐', 223 => '▀',
        224 => 'α', 225 => 'ß', 226 => 'Γ', 227 => 'π', 228 => 'Σ', 229 => 'σ', 230 => 'µ', 231 => 'τ',
        232 => 'Φ', 233 => 'Θ', 234 => 'Ω', 235 => 'δ', 236 => '∞', 237 => 'φ', 238 => 'ε', 239 => '∩',
        240 => '≡', 241 => '±', 242 => '≥', 243 => '≤', 244 => '⌠', 245 => '⌡', 246 => '÷', 247 => '≈',
        248 => '°', 249 => '∙', 250 => '·', 251 => '√', 252 => 'ⁿ', 253 => '²', 254 => '■', 255 => ' ',
    }
}

/// A BASIC value - can be string, number, or array
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    String(String),
    Float(f64),
    Integer(i64),
    IntArray(Vec<i64>),
    FloatArray(Vec<f64>),
    StringArray(Vec<String>),
}

impl Value {
    pub fn to_float(&self) -> f64 {
        match self {
            Value::Float(f) => *f,
            Value::Integer(i) => *i as f64,
            Value::String(s) => s.parse().unwrap_or(0.0),
            _ => 0.0,
        }
    }

    pub fn to_int(&self) -> i64 {
        match self {
            Value::Float(f) => *f as i64,
            Value::Integer(i) => *i,
            Value::String(s) => s.parse().unwrap_or(0),
            _ => 0,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Float(f) => {
                if *f == f.trunc() && f.abs() < 1e10 {
                    format!("{}", *f as i64)
                } else {
                    format!("{}", f)
                }
            }
            Value::Integer(i) => format!("{}", i),
            _ => String::new(),
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Integer(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            _ => false,
        }
    }
}

/// Result of program execution (matches what app.rs expects)
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    /// Program completed successfully
    Completed,
    /// Program was stopped by user
    Stopped,
    /// Hit a breakpoint at line number
    Breakpoint(usize),
    /// Step mode stopped at line
    Stepped(usize),
    /// Waiting for input (INPUT or INKEY$)
    NeedsInput,
    /// Still running, yielded for UI update
    Running,
}

/// A SUB or FUNCTION definition
#[derive(Clone, Debug)]
pub struct Procedure {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub is_function: bool,
}

/// Pending INPUT statement state
#[derive(Clone, Debug)]
pub struct PendingInput {
    pub prompt: String,
    pub var_names: Vec<String>,
}

/// Why the interpreter yielded control
#[derive(Debug, Clone)]
pub enum YieldReason {
    /// Periodic UI update (every ~16ms)
    UiUpdate,
    /// Waiting for keyboard input (INPUT or INKEY$)
    NeedsInput,
    /// Hit a breakpoint at line number
    Breakpoint(usize),
    /// Step mode stopped at line
    Stepped(usize),
}

/// Internal interpreter state, shared between generator and owner
pub struct InterpreterState {
    // Variables
    pub variables: HashMap<String, Value>,

    // Program state
    pub current_line: usize,
    labels: HashMap<String, usize>,
    data_values: Vec<Value>,
    data_pointer: usize,

    // Subroutine/function support
    gosub_stack: Vec<usize>,
    procedures: HashMap<String, Procedure>,
    call_stack: Vec<HashMap<String, Value>>,
    return_value: Option<Value>,

    // Graphics
    pub graphics: GraphicsMode,

    // I/O
    output_buffer: Vec<String>,
    input_buffer: String,
    input_ready: bool,
    pending_input: Option<PendingInput>,
    last_key: Option<char>,

    // Execution control
    running: bool,
    stop_requested: bool,
    breakpoints: Vec<usize>,
    step_mode: bool,

    // Timing
    last_yield_time: Instant,
    start_time: Instant,

    // Error state
    error: Option<String>,
}

impl InterpreterState {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
            current_line: 0,
            labels: HashMap::new(),
            data_values: Vec::new(),
            data_pointer: 0,
            gosub_stack: Vec::new(),
            procedures: HashMap::new(),
            call_stack: Vec::new(),
            return_value: None,
            graphics: {
                let mut g = GraphicsMode::new(80, 25);
                g.mode = 0;  // Start in text mode
                g
            },
            output_buffer: Vec::new(),
            input_buffer: String::new(),
            input_ready: false,
            pending_input: None,
            last_key: None,
            running: false,
            stop_requested: false,
            breakpoints: Vec::new(),
            step_mode: false,
            last_yield_time: Instant::now(),
            start_time: Instant::now(),
            error: None,
        }
    }

    fn reset(&mut self) {
        self.variables.clear();
        self.current_line = 0;
        self.labels.clear();
        self.data_values.clear();
        self.data_pointer = 0;
        self.gosub_stack.clear();
        self.procedures.clear();
        self.call_stack.clear();
        self.return_value = None;
        self.output_buffer.clear();
        self.input_buffer.clear();
        self.input_ready = false;
        self.pending_input = None;
        self.last_key = None;
        self.running = false;
        self.stop_requested = false;
        self.step_mode = false;
        self.error = None;
        self.start_time = Instant::now();
        self.last_yield_time = Instant::now();
    }

    fn should_yield_for_ui(&self) -> bool {
        self.last_yield_time.elapsed().as_millis() >= 16
    }
}

/// Trait for resumable generators
trait Resumable {
    fn resume_gen(&mut self) -> Option<YieldReason>;
}

/// Wrapper to make Gen implement our Resumable trait
struct GenWrapper<F: std::future::Future<Output = ()>> {
    gen: Gen<YieldReason, (), F>,
}

impl<F: std::future::Future<Output = ()>> Resumable for GenWrapper<F> {
    fn resume_gen(&mut self) -> Option<YieldReason> {
        match self.gen.resume() {
            GeneratorState::Yielded(y) => Some(y),
            GeneratorState::Complete(()) => None,
        }
    }
}

/// Type alias for boxed generator
type BoxedGenerator = Box<dyn Resumable>;

/// BASIC interpreter with generator-based execution
pub struct Interpreter {
    state: Rc<RefCell<InterpreterState>>,
    /// Stored generator for continuation after yield
    generator: Option<BoxedGenerator>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(InterpreterState::new())),
            generator: None,
        }
    }

    // Expose graphics for external access
    pub fn graphics_mut(&mut self) -> std::cell::RefMut<'_, GraphicsMode> {
        std::cell::RefMut::map(self.state.borrow_mut(), |s| &mut s.graphics)
    }

    pub fn graphics(&self) -> std::cell::Ref<'_, GraphicsMode> {
        std::cell::Ref::map(self.state.borrow(), |s| &s.graphics)
    }

    pub fn with_graphics<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&GraphicsMode) -> R,
    {
        f(&self.state.borrow().graphics)
    }

    pub fn with_graphics_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut GraphicsMode) -> R,
    {
        f(&mut self.state.borrow_mut().graphics)
    }

    pub fn reset(&mut self) {
        self.state.borrow_mut().reset();
        self.generator = None;
    }

    pub fn set_breakpoints(&mut self, breakpoints: &[usize]) {
        self.state.borrow_mut().breakpoints = breakpoints.to_vec();
    }

    pub fn set_step_mode(&mut self, enabled: bool) {
        self.state.borrow_mut().step_mode = enabled;
    }

    pub fn request_stop(&mut self) {
        self.state.borrow_mut().stop_requested = true;
    }

    pub fn is_running(&self) -> bool {
        self.state.borrow().running
    }

    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.state.borrow_mut().output_buffer)
    }

    pub fn pending_input(&self) -> Option<PendingInput> {
        self.state.borrow().pending_input.clone()
    }

    pub fn has_pending_input(&self) -> bool {
        self.state.borrow().pending_input.is_some()
    }

    pub fn clear_pending_input(&mut self) {
        self.state.borrow_mut().pending_input = None;
        self.state.borrow_mut().input_buffer.clear();
        self.state.borrow_mut().input_ready = false;
    }

    pub fn add_input_char(&mut self, c: char) {
        self.state.borrow_mut().input_buffer.push(c);
    }

    pub fn delete_input_char(&mut self) {
        self.state.borrow_mut().input_buffer.pop();
    }

    /// Alias for delete_input_char for backwards compatibility
    pub fn backspace_input(&mut self) {
        self.delete_input_char();
    }

    pub fn get_input_buffer(&self) -> String {
        self.state.borrow().input_buffer.clone()
    }

    pub fn complete_input(&mut self) {
        self.state.borrow_mut().input_ready = true;
    }

    pub fn set_last_key(&mut self, key: Option<char>) {
        self.state.borrow_mut().last_key = key;
    }

    /// Set pending key from string (for INKEY$ with escape sequences)
    pub fn set_pending_key(&mut self, key: Option<String>) {
        self.state.borrow_mut().last_key = key.and_then(|s| s.chars().next());
    }

    pub fn get_last_key(&self) -> Option<char> {
        self.state.borrow().last_key
    }

    /// Direct access to graphics for mutation (needed by app.rs)
    pub fn graphics_direct_mut(&mut self) -> std::cell::RefMut<'_, GraphicsMode> {
        std::cell::RefMut::map(self.state.borrow_mut(), |s| &mut s.graphics)
    }

    pub fn current_line(&self) -> usize {
        self.state.borrow().current_line
    }

    pub fn get_error(&self) -> Option<String> {
        self.state.borrow().error.clone()
    }

    /// Execute program synchronously (for tests and simple usage)
    pub fn execute(&mut self, program: &[Stmt]) -> Result<(), String> {
        // Reset state
        {
            let mut s = self.state.borrow_mut();
            s.running = true;
            s.stop_requested = false;
            s.error = None;
            s.start_time = Instant::now();
            s.last_yield_time = Instant::now();
            s.labels.clear();
            s.data_values.clear();
            s.data_pointer = 0;
            s.procedures.clear();
        }

        // Pre-process
        {
            let mut state = self.state.borrow_mut();
            for (idx, stmt) in program.iter().enumerate() {
                match stmt {
                    Stmt::Label(n) => {
                        state.labels.insert(n.to_string(), idx);
                    }
                    Stmt::TextLabel(label) => {
                        state.labels.insert(label.clone(), idx);
                    }
                    Stmt::Data(exprs) => {
                        for expr in exprs {
                            let value = eval_const_expr(expr);
                            state.data_values.push(value);
                        }
                    }
                    Stmt::Sub { name, params, body } => {
                        state.procedures.insert(
                            name.to_uppercase(),
                            Procedure {
                                name: name.clone(),
                                params: params.clone(),
                                body: body.clone(),
                                is_function: false,
                            },
                        );
                    }
                    Stmt::Function { name, params, body } => {
                        state.procedures.insert(
                            name.to_uppercase(),
                            Procedure {
                                name: name.clone(),
                                params: params.clone(),
                                body: body.clone(),
                                is_function: true,
                            },
                        );
                    }
                    _ => {}
                }
            }
        }

        // Create generator and consume all yields
        let gen = create_execution_generator(self.state.clone(), program.to_vec());
        let mut wrapper = GenWrapper { gen };
        while wrapper.resume_gen().is_some() {
            // Consume all yields
        }

        if let Some(err) = self.state.borrow().error.clone() {
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Execute with debug support - stores generator for later continuation
    pub fn execute_with_debug(&mut self, program: &[Stmt]) -> Result<ExecutionResult, String> {
        // Reset state for new execution
        {
            let mut s = self.state.borrow_mut();
            s.running = true;
            s.stop_requested = false;
            s.error = None;
            s.start_time = Instant::now();
            s.last_yield_time = Instant::now();
            s.labels.clear();
            s.data_values.clear();
            s.data_pointer = 0;
            s.procedures.clear();
        }

        // Pre-process: collect labels, DATA statements, and procedures
        {
            let mut state = self.state.borrow_mut();
            for (idx, stmt) in program.iter().enumerate() {
                match stmt {
                    Stmt::Label(n) => {
                        state.labels.insert(n.to_string(), idx);
                    }
                    Stmt::TextLabel(label) => {
                        state.labels.insert(label.clone(), idx);
                    }
                    Stmt::Data(exprs) => {
                        for expr in exprs {
                            let value = eval_const_expr(expr);
                            state.data_values.push(value);
                        }
                    }
                    Stmt::Sub { name, params, body } => {
                        state.procedures.insert(
                            name.to_uppercase(),
                            Procedure {
                                name: name.clone(),
                                params: params.clone(),
                                body: body.clone(),
                                is_function: false,
                            },
                        );
                    }
                    Stmt::Function { name, params, body } => {
                        state.procedures.insert(
                            name.to_uppercase(),
                            Procedure {
                                name: name.clone(),
                                params: params.clone(),
                                body: body.clone(),
                                is_function: true,
                            },
                        );
                    }
                    _ => {}
                }
            }
        }

        // Create and store the generator (clone state to avoid borrowing self)
        let state_clone = self.state.clone();
        let program_owned = program.to_vec();
        let gen = create_execution_generator(state_clone, program_owned);
        self.generator = Some(Box::new(GenWrapper { gen }));

        // Run until first yield or completion
        self.continue_execution(program)
    }

    /// Continue execution after a yield (e.g., after input is provided)
    pub fn continue_execution(&mut self, _program: &[Stmt]) -> Result<ExecutionResult, String> {
        // Run generator until next yield or completion
        loop {
            match self.generator.as_mut().and_then(|g| g.resume_gen()) {
                Some(YieldReason::NeedsInput) => {
                    return Ok(ExecutionResult::NeedsInput);
                }
                Some(YieldReason::Breakpoint(line)) => {
                    return Ok(ExecutionResult::Breakpoint(line));
                }
                Some(YieldReason::Stepped(line)) => {
                    return Ok(ExecutionResult::Stepped(line));
                }
                Some(YieldReason::UiUpdate) => {
                    // Return to the app so it can render, then it will call us again
                    return Ok(ExecutionResult::Running);
                }
                None => {
                    // Generator completed
                    self.generator = None;
                    if let Some(err) = self.state.borrow().error.clone() {
                        return Err(err);
                    } else if self.state.borrow().stop_requested {
                        return Ok(ExecutionResult::Stopped);
                    } else {
                        return Ok(ExecutionResult::Completed);
                    }
                }
            }
        }
    }

    /// Evaluate an expression (for immediate window)
    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        // Use a synchronous evaluation - create a minimal generator context
        let state = self.state.clone();

        // We can't easily use async here, so do a simpler synchronous eval
        eval_expr_sync(&state, expr)
    }

}

/// Create a generator for program execution (standalone to avoid borrow issues)
fn create_execution_generator(
    state: Rc<RefCell<InterpreterState>>,
    program: Vec<Stmt>,
) -> Gen<YieldReason, (), impl std::future::Future<Output = ()>> {
    Gen::new(|co: Co<YieldReason>| async move {
        execute_program(&co, &state, &program).await;
        state.borrow_mut().running = false;
    })
}

/// Evaluate a constant expression (for DATA statements)
fn eval_const_expr(expr: &Expr) -> Value {
    match expr {
        Expr::Integer(n) => Value::Integer(*n),
        Expr::Float(n) => Value::Float(*n),
        Expr::String(s) => Value::String(s.clone()),
        Expr::UnaryOp(UnaryOp::Neg, inner) => {
            let v = eval_const_expr(inner);
            match v {
                Value::Integer(n) => Value::Integer(-n),
                Value::Float(n) => Value::Float(-n),
                _ => Value::Integer(0),
            }
        }
        _ => Value::Integer(0),
    }
}

// For backwards compatibility with tests that access graphics directly
impl std::ops::Deref for Interpreter {
    type Target = Rc<RefCell<InterpreterState>>;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

/// Main program execution loop
async fn execute_program(
    co: &Co<YieldReason>,
    state: &Rc<RefCell<InterpreterState>>,
    program: &[Stmt],
) {
    let mut pos = 0;

    while pos < program.len() {
        // Check stop request
        if state.borrow().stop_requested {
            return;
        }

        // Update current line and check breakpoints
        state.borrow_mut().current_line = pos;

        let should_break = {
            let s = state.borrow();
            s.breakpoints.contains(&pos) || s.step_mode
        };

        if should_break {
            if state.borrow().breakpoints.contains(&pos) {
                co.yield_(YieldReason::Breakpoint(pos)).await;
            } else {
                co.yield_(YieldReason::Stepped(pos)).await;
            }
            // Check if stop was requested during the yield
            if state.borrow().stop_requested {
                return;
            }
        }

        // Execute statement
        let result = execute_stmt(co, state, &program[pos], program).await;

        match result {
            StmtResult::Continue => pos += 1,
            StmtResult::Jump(new_pos) => pos = new_pos,
            StmtResult::End => return,
            StmtResult::Error(e) => {
                state.borrow_mut().error = Some(e);
                return;
            }
        }

        // Periodic UI yield
        if state.borrow().should_yield_for_ui() {
            state.borrow_mut().last_yield_time = Instant::now();
            co.yield_(YieldReason::UiUpdate).await;
        }
    }
}

/// Result of executing a statement
enum StmtResult {
    Continue,
    Jump(usize),
    End,
    Error(String),
}

/// Execute a single statement
#[async_recursion(?Send)]
async fn execute_stmt(
    co: &Co<YieldReason>,
    state: &Rc<RefCell<InterpreterState>>,
    stmt: &Stmt,
    program: &[Stmt],
) -> StmtResult {
    match stmt {
        Stmt::Empty | Stmt::Label(_) | Stmt::TextLabel(_) | Stmt::Data(_) | Stmt::Rem(_) |
        Stmt::Sub { .. } | Stmt::Function { .. } => {
            StmtResult::Continue
        }

        Stmt::Let(name, value) => {
            match eval_expr_core(state, value) {
                Ok(v) => {
                    state.borrow_mut().variables.insert(name.clone(), v);
                    StmtResult::Continue
                }
                Err(e) => StmtResult::Error(e),
            }
        }

        Stmt::ArrayLet(name, indices, value) => {
            let idx_values: Result<Vec<i64>, String> = {
                let mut results = Vec::new();
                for idx in indices {
                    match eval_expr_core(state, idx) {
                        Ok(v) => results.push(v.to_int()),
                        Err(e) => return StmtResult::Error(e),
                    }
                }
                Ok(results)
            };

            match idx_values {
                Ok(indices) => {
                    match eval_expr_core(state, value) {
                        Ok(val) => {
                            let mut s = state.borrow_mut();
                            if indices.len() == 1 {
                                let idx = indices[0] as usize;
                                if let Some(arr) = s.variables.get_mut(name) {
                                    match (arr, &val) {
                                        (Value::IntArray(ref mut a), Value::Integer(v)) => {
                                            if idx < a.len() { a[idx] = *v; }
                                        }
                                        (Value::IntArray(ref mut a), Value::Float(v)) => {
                                            if idx < a.len() { a[idx] = *v as i64; }
                                        }
                                        (Value::FloatArray(ref mut a), Value::Float(v)) => {
                                            if idx < a.len() { a[idx] = *v; }
                                        }
                                        (Value::FloatArray(ref mut a), Value::Integer(v)) => {
                                            if idx < a.len() { a[idx] = *v as f64; }
                                        }
                                        (Value::StringArray(ref mut a), Value::String(v)) => {
                                            if idx < a.len() { a[idx] = v.clone(); }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            StmtResult::Continue
                        }
                        Err(e) => StmtResult::Error(e),
                    }
                }
                Err(e) => StmtResult::Error(e),
            }
        }

        Stmt::Print(items) => {
            let mut line = String::new();
            let mut no_newline = false;

            for item in items {
                match item {
                    PrintItem::Semicolon => no_newline = true,
                    PrintItem::Comma => {
                        // Tab to next 14-column zone
                        let spaces = 14 - (line.len() % 14);
                        line.push_str(&" ".repeat(spaces));
                        no_newline = true;
                    }
                    PrintItem::Tab(expr) => {
                        let col = match eval_expr_core(state, expr) {
                            Ok(v) => v.to_int().max(1) as usize - 1,
                            Err(e) => return StmtResult::Error(e),
                        };
                        while line.len() < col {
                            line.push(' ');
                        }
                        no_newline = true;
                    }
                    PrintItem::Spc(expr) => {
                        let n = match eval_expr_core(state, expr) {
                            Ok(v) => v.to_int().max(0) as usize,
                            Err(e) => return StmtResult::Error(e),
                        };
                        line.push_str(&" ".repeat(n));
                        no_newline = true;
                    }
                    PrintItem::Expr(expr) => {
                        match eval_expr_core(state, expr) {
                            Ok(v) => {
                                line.push_str(&v.to_string());
                                no_newline = false;
                            }
                            Err(e) => return StmtResult::Error(e),
                        }
                    }
                }
            }

            // Handle graphics mode printing
            {
                let mut s = state.borrow_mut();
                if s.graphics.mode > 0 {
                    s.graphics.print_text(&line, !no_newline);
                } else {
                    s.output_buffer.push(line);
                }
            }

            StmtResult::Continue
        }

        Stmt::Input(prompt, vars) => {
            // Output prompt
            if let Some(p) = prompt {
                let mut s = state.borrow_mut();
                if s.graphics.mode > 0 {
                    s.graphics.print_text(p, false);
                    s.graphics.print_text("? ", false);
                } else {
                    s.output_buffer.push(format!("{}? ", p));
                }
            } else {
                let mut s = state.borrow_mut();
                if s.graphics.mode > 0 {
                    s.graphics.print_text("? ", false);
                } else {
                    s.output_buffer.push("? ".to_string());
                }
            }

            // Set up pending input
            {
                let mut s = state.borrow_mut();
                s.pending_input = Some(PendingInput {
                    prompt: prompt.clone().unwrap_or_default(),
                    var_names: vars.clone(),
                });
                s.input_buffer.clear();
                s.input_ready = false;
            }

            // Wait for input
            loop {
                co.yield_(YieldReason::NeedsInput).await;

                let (ready, stop) = {
                    let s = state.borrow();
                    (s.input_ready, s.stop_requested)
                };

                if stop {
                    return StmtResult::End;
                }

                if ready {
                    break;
                }
            }

            // Process input
            {
                let mut s = state.borrow_mut();
                let input = s.input_buffer.clone();
                s.input_buffer.clear();
                s.pending_input = None;

                // Echo input to graphics if active
                if s.graphics.mode > 0 {
                    s.graphics.print_text(&input, true);
                }

                // Split input by commas for multiple variables
                let parts: Vec<&str> = input.split(',').collect();
                for (i, var) in vars.iter().enumerate() {
                    let part = parts.get(i).map(|s| s.trim()).unwrap_or("");
                    let is_numeric = !var.ends_with('$');
                    let value = if is_numeric {
                        if let Ok(n) = part.parse::<i64>() {
                            Value::Integer(n)
                        } else if let Ok(f) = part.parse::<f64>() {
                            Value::Float(f)
                        } else {
                            Value::Integer(0)
                        }
                    } else {
                        Value::String(part.to_string())
                    };
                    s.variables.insert(var.clone(), value);
                }
            }

            StmtResult::Continue
        }

        Stmt::If { condition, then_branch, else_branch } => {
            match eval_expr_core(state, condition) {
                Ok(cond) => {
                    if cond.is_truthy() {
                        for stmt in then_branch {
                            let result = execute_stmt(co, state, stmt, program).await;
                            match result {
                                StmtResult::Continue => {}
                                other => return other,
                            }
                        }
                    } else if let Some(else_stmts) = else_branch {
                        for stmt in else_stmts {
                            let result = execute_stmt(co, state, stmt, program).await;
                            match result {
                                StmtResult::Continue => {}
                                other => return other,
                            }
                        }
                    }
                    StmtResult::Continue
                }
                Err(e) => StmtResult::Error(e),
            }
        }

        Stmt::For { var, start, end, step, body } => {
            let start_val = match eval_expr_core(state, start) {
                Ok(v) => v.to_float(),
                Err(e) => return StmtResult::Error(e),
            };
            let end_val = match eval_expr_core(state, end) {
                Ok(v) => v.to_float(),
                Err(e) => return StmtResult::Error(e),
            };
            let step_val = match step {
                Some(s) => match eval_expr_core(state, s) {
                    Ok(v) => v.to_float(),
                    Err(e) => return StmtResult::Error(e),
                },
                None => 1.0,
            };

            let mut current = start_val;

            loop {
                // Check termination condition
                if step_val > 0.0 && current > end_val {
                    break;
                }
                if step_val < 0.0 && current < end_val {
                    break;
                }
                if step_val == 0.0 {
                    break; // Prevent infinite loop
                }

                // Set loop variable
                state.borrow_mut().variables.insert(var.clone(), Value::Float(current));

                // Execute body
                for stmt in body {
                    if state.borrow().stop_requested {
                        return StmtResult::End;
                    }

                    let result = execute_stmt(co, state, stmt, program).await;
                    match result {
                        StmtResult::Continue => {}
                        StmtResult::Jump(pos) => return StmtResult::Jump(pos),
                        StmtResult::End => return StmtResult::End,
                        StmtResult::Error(e) => return StmtResult::Error(e),
                    }
                }

                // Increment
                current += step_val;

                // Periodic yield for UI updates
                if state.borrow().should_yield_for_ui() {
                    state.borrow_mut().last_yield_time = Instant::now();
                    co.yield_(YieldReason::UiUpdate).await;
                }
            }

            // Set final value
            state.borrow_mut().variables.insert(var.clone(), Value::Float(current));
            StmtResult::Continue
        }

        Stmt::While { condition, body } => {
            loop {
                // Check condition
                let cond = match eval_expr_core(state, condition) {
                    Ok(v) => v.is_truthy(),
                    Err(e) => return StmtResult::Error(e),
                };

                if !cond {
                    break;
                }

                // Execute body
                for stmt in body {
                    if state.borrow().stop_requested {
                        return StmtResult::End;
                    }

                    let result = execute_stmt(co, state, stmt, program).await;
                    match result {
                        StmtResult::Continue => {}
                        StmtResult::Jump(pos) => return StmtResult::Jump(pos),
                        StmtResult::End => return StmtResult::End,
                        StmtResult::Error(e) => return StmtResult::Error(e),
                    }
                }

                // Periodic yield
                if state.borrow().should_yield_for_ui() {
                    state.borrow_mut().last_yield_time = Instant::now();
                    co.yield_(YieldReason::UiUpdate).await;
                }
            }

            StmtResult::Continue
        }

        Stmt::DoLoop { condition, is_while, is_pre_test, body } => {
            loop {
                // Check condition at start (if pre-test)
                if *is_pre_test {
                    if let Some(cond_expr) = condition {
                        let cond = match eval_expr_core(state, cond_expr) {
                            Ok(v) => v.is_truthy(),
                            Err(e) => return StmtResult::Error(e),
                        };

                        // is_while=true means WHILE (continue if true), is_while=false means UNTIL (exit if true)
                        let should_exit = if *is_while { !cond } else { cond };
                        if should_exit {
                            break;
                        }
                    }
                }

                // Execute body
                for stmt in body {
                    if state.borrow().stop_requested {
                        return StmtResult::End;
                    }

                    let result = execute_stmt(co, state, stmt, program).await;
                    match result {
                        StmtResult::Continue => {}
                        StmtResult::Jump(pos) => return StmtResult::Jump(pos),
                        StmtResult::End => return StmtResult::End,
                        StmtResult::Error(e) => return StmtResult::Error(e),
                    }
                }

                // Check condition at end (if post-test)
                if !*is_pre_test {
                    if let Some(cond_expr) = condition {
                        let cond = match eval_expr_core(state, cond_expr) {
                            Ok(v) => v.is_truthy(),
                            Err(e) => return StmtResult::Error(e),
                        };

                        let should_exit = if *is_while { !cond } else { cond };
                        if should_exit {
                            break;
                        }
                    }
                    // No condition = infinite loop (need explicit EXIT DO)
                }

                // Periodic yield
                if state.borrow().should_yield_for_ui() {
                    state.borrow_mut().last_yield_time = Instant::now();
                    co.yield_(YieldReason::UiUpdate).await;
                }
            }

            StmtResult::Continue
        }

        Stmt::GoTo(line) => {
            let label = line.to_string();
            let target = state.borrow().labels.get(&label).copied();
            match target {
                Some(pos) => StmtResult::Jump(pos),
                None => StmtResult::Error(format!("Label not found: {}", line)),
            }
        }

        Stmt::GoToLabel(label) => {
            let target = state.borrow().labels.get(label).copied();
            match target {
                Some(pos) => StmtResult::Jump(pos),
                None => StmtResult::Error(format!("Label not found: {}", label)),
            }
        }

        Stmt::GoSub(line) => {
            let label = line.to_string();
            let (target, current) = {
                let s = state.borrow();
                (s.labels.get(&label).copied(), s.current_line)
            };

            match target {
                Some(pos) => {
                    // Push return address
                    state.borrow_mut().gosub_stack.push(current + 1);

                    // Execute from target until RETURN
                    let result = execute_subroutine(co, state, program, pos).await;

                    // Pop return address
                    state.borrow_mut().gosub_stack.pop();

                    result
                }
                None => StmtResult::Error(format!("Label not found: {}", line)),
            }
        }

        Stmt::GoSubLabel(label) => {
            let (target, current) = {
                let s = state.borrow();
                (s.labels.get(label).copied(), s.current_line)
            };

            match target {
                Some(pos) => {
                    state.borrow_mut().gosub_stack.push(current + 1);
                    let result = execute_subroutine(co, state, program, pos).await;
                    state.borrow_mut().gosub_stack.pop();
                    result
                }
                None => StmtResult::Error(format!("Label not found: {}", label)),
            }
        }

        Stmt::Return(_) => {
            // Return is handled by execute_subroutine
            StmtResult::Continue
        }

        Stmt::Dim(dim_vars) => {
            for dim_var in dim_vars {
                let DimVar { name, dimensions, var_type: _ } = dim_var;
                let sizes: Result<Vec<usize>, String> = {
                    let mut results = Vec::new();
                    for dim in dimensions {
                        match eval_expr_core(state, dim) {
                            Ok(v) => results.push((v.to_int() + 1) as usize), // BASIC arrays are 0 to N
                            Err(e) => return StmtResult::Error(e),
                        }
                    }
                    Ok(results)
                };

                match sizes {
                    Ok(dims) => {
                        let size = dims.iter().product();
                        let is_string = name.ends_with('$');

                        let array = if is_string {
                            Value::StringArray(vec![String::new(); size])
                        } else if name.ends_with('%') {
                            Value::IntArray(vec![0; size])
                        } else {
                            Value::FloatArray(vec![0.0; size])
                        };

                        state.borrow_mut().variables.insert(name.clone(), array);
                    }
                    Err(e) => return StmtResult::Error(e),
                }
            }
            StmtResult::Continue
        }

        Stmt::Call(name, args) => {
            // Evaluate arguments
            let arg_values: Result<Vec<Value>, String> = {
                let mut results = Vec::new();
                for arg in args {
                    match eval_expr_core(state, arg) {
                        Ok(v) => results.push(v),
                        Err(e) => return StmtResult::Error(e),
                    }
                }
                Ok(results)
            };

            let arg_values = match arg_values {
                Ok(v) => v,
                Err(e) => return StmtResult::Error(e),
            };

            // Look up procedure
            let proc = state.borrow().procedures.get(&name.to_uppercase()).cloned();

            match proc {
                Some(procedure) => {
                    // Set up local scope with parameters
                    let mut local_scope = HashMap::new();
                    for (param, value) in procedure.params.iter().zip(arg_values) {
                        local_scope.insert(param.clone(), value);
                    }

                    // Push scope
                    state.borrow_mut().call_stack.push(local_scope);

                    // Execute procedure body
                    for stmt in &procedure.body {
                        if state.borrow().stop_requested {
                            state.borrow_mut().call_stack.pop();
                            return StmtResult::End;
                        }

                        let result = execute_stmt(co, state, stmt, program).await;
                        match result {
                            StmtResult::Continue => {}
                            other => {
                                state.borrow_mut().call_stack.pop();
                                return other;
                            }
                        }
                    }

                    // Pop scope
                    state.borrow_mut().call_stack.pop();
                    StmtResult::Continue
                }
                None => StmtResult::Error(format!("SUB not found: {}", name)),
            }
        }

        Stmt::End | Stmt::Stop => StmtResult::End,

        Stmt::Cls => {
            state.borrow_mut().graphics.cls();
            StmtResult::Continue
        }

        Stmt::Screen(mode) => {
            let mode_val = match eval_expr_core(state, mode) {
                Ok(v) => v.to_int() as u8,
                Err(e) => return StmtResult::Error(e),
            };

            state.borrow_mut().graphics.set_mode(mode_val);
            StmtResult::Continue
        }

        Stmt::Color(fg, bg) => {
            let fg_val = match eval_expr_core(state, fg) {
                Ok(v) => v.to_int() as u8,
                Err(e) => return StmtResult::Error(e),
            };

            let bg_val = if let Some(bg_expr) = bg {
                match eval_expr_core(state, bg_expr) {
                    Ok(v) => v.to_int() as u8,
                    Err(e) => return StmtResult::Error(e),
                }
            } else {
                state.borrow().graphics.background
            };

            state.borrow_mut().graphics.set_color(fg_val, bg_val);
            StmtResult::Continue
        }

        Stmt::Locate(row, col) => {
            let r = match eval_expr_core(state, row) {
                Ok(v) => v.to_int() as u16,
                Err(e) => return StmtResult::Error(e),
            };

            let c = match eval_expr_core(state, col) {
                Ok(v) => v.to_int() as u16,
                Err(e) => return StmtResult::Error(e),
            };

            // BASIC LOCATE is 1-based, locate() expects 1-based
            state.borrow_mut().graphics.locate(r, c);
            StmtResult::Continue
        }

        Stmt::Pset(x, y, color) => {
            let x_val = match eval_expr_core(state, x) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };

            let y_val = match eval_expr_core(state, y) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };

            let color_val = if let Some(c) = color {
                match eval_expr_core(state, c) {
                    Ok(v) => v.to_int() as u8,
                    Err(e) => return StmtResult::Error(e),
                }
            } else {
                state.borrow().graphics.foreground
            };

            state.borrow_mut().graphics.pset(x_val, y_val, color_val);
            StmtResult::Continue
        }

        Stmt::Line { x1, y1, x2, y2, color, box_fill } => {
            let x1_val = match eval_expr_core(state, x1) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };
            let y1_val = match eval_expr_core(state, y1) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };
            let x2_val = match eval_expr_core(state, x2) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };
            let y2_val = match eval_expr_core(state, y2) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };

            let color_val = if let Some(c) = color {
                match eval_expr_core(state, c) {
                    Ok(v) => v.to_int() as u8,
                    Err(e) => return StmtResult::Error(e),
                }
            } else {
                state.borrow().graphics.foreground
            };

            let mut s = state.borrow_mut();
            match box_fill {
                Some(true) => s.graphics.fill_box(x1_val, y1_val, x2_val, y2_val, color_val),
                Some(false) => s.graphics.draw_box(x1_val, y1_val, x2_val, y2_val, color_val),
                None => s.graphics.line(x1_val, y1_val, x2_val, y2_val, color_val),
            }
            StmtResult::Continue
        }

        Stmt::Circle { x, y, radius, color, start_angle, end_angle, aspect } => {
            let x_val = match eval_expr_core(state, x) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };
            let y_val = match eval_expr_core(state, y) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };
            let radius_val = match eval_expr_core(state, radius) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };

            let color_val = if let Some(c) = color {
                match eval_expr_core(state, c) {
                    Ok(v) => v.to_int() as u8,
                    Err(e) => return StmtResult::Error(e),
                }
            } else {
                state.borrow().graphics.foreground
            };

            let start_val = if let Some(s) = start_angle {
                match eval_expr_core(state, s) {
                    Ok(v) => Some(v.to_float()),
                    Err(e) => return StmtResult::Error(e),
                }
            } else {
                None
            };

            let end_val = if let Some(e) = end_angle {
                match eval_expr_core(state, e) {
                    Ok(v) => Some(v.to_float()),
                    Err(e) => return StmtResult::Error(e),
                }
            } else {
                None
            };

            let aspect_val = if let Some(a) = aspect {
                match eval_expr_core(state, a) {
                    Ok(v) => Some(v.to_float()),
                    Err(e) => return StmtResult::Error(e),
                }
            } else {
                None
            };

            state.borrow_mut().graphics.circle_arc(x_val, y_val, radius_val, color_val, start_val, end_val, aspect_val);
            StmtResult::Continue
        }

        Stmt::Bezier { x1, y1, cx, cy, x2, y2, color, thickness } => {
            let x1_val = match eval_expr_core(state, x1) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };
            let y1_val = match eval_expr_core(state, y1) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };
            let cx_val = match eval_expr_core(state, cx) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };
            let cy_val = match eval_expr_core(state, cy) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };
            let x2_val = match eval_expr_core(state, x2) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };
            let y2_val = match eval_expr_core(state, y2) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };

            let color_val = if let Some(c) = color {
                match eval_expr_core(state, c) {
                    Ok(v) => v.to_int() as u8,
                    Err(e) => return StmtResult::Error(e),
                }
            } else {
                state.borrow().graphics.foreground
            };

            let thickness_val = if let Some(t) = thickness {
                match eval_expr_core(state, t) {
                    Ok(v) => v.to_int() as i32,
                    Err(e) => return StmtResult::Error(e),
                }
            } else {
                1
            };

            state.borrow_mut().graphics.bezier(x1_val, y1_val, cx_val, cy_val, x2_val, y2_val, color_val, thickness_val);
            StmtResult::Continue
        }

        Stmt::Paint(x, y, color) => {
            let x_val = match eval_expr_core(state, x) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };
            let y_val = match eval_expr_core(state, y) {
                Ok(v) => v.to_int() as i32,
                Err(e) => return StmtResult::Error(e),
            };
            let color_val = match eval_expr_core(state, color) {
                Ok(v) => v.to_int() as u8,
                Err(e) => return StmtResult::Error(e),
            };

            state.borrow_mut().graphics.paint(x_val, y_val, color_val);
            StmtResult::Continue
        }

        Stmt::Beep => {
            // Beep is a no-op in terminal mode
            StmtResult::Continue
        }

        Stmt::Sound(_, _) => {
            // Sound is a no-op in terminal mode
            StmtResult::Continue
        }

        Stmt::Sleep(duration) => {
            if let Some(dur_expr) = duration {
                let dur = match eval_expr_core(state, dur_expr) {
                    Ok(v) => v.to_float(),
                    Err(e) => return StmtResult::Error(e),
                };

                let target = std::time::Instant::now() + std::time::Duration::from_secs_f64(dur);
                while std::time::Instant::now() < target {
                    if state.borrow().stop_requested {
                        return StmtResult::End;
                    }
                    co.yield_(YieldReason::UiUpdate).await;
                }
            } else {
                // Sleep with no duration waits for keypress
                loop {
                    co.yield_(YieldReason::NeedsInput).await;
                    if state.borrow().stop_requested || state.borrow().last_key.is_some() {
                        state.borrow_mut().last_key = None;
                        break;
                    }
                }
            }
            StmtResult::Continue
        }

        Stmt::Randomize(seed) => {
            if let Some(seed_expr) = seed {
                match eval_expr_core(state, seed_expr) {
                    Ok(_) => {
                        // Seed is currently ignored
                    }
                    Err(e) => return StmtResult::Error(e),
                }
            }
            StmtResult::Continue
        }

        Stmt::Read(vars) => {
            for var in vars {
                let value = {
                    let mut s = state.borrow_mut();
                    if s.data_pointer < s.data_values.len() {
                        let v = s.data_values[s.data_pointer].clone();
                        s.data_pointer += 1;
                        v
                    } else {
                        return StmtResult::Error("Out of DATA".to_string());
                    }
                };
                state.borrow_mut().variables.insert(var.clone(), value);
            }
            StmtResult::Continue
        }

        Stmt::Restore(line) => {
            if let Some(n) = line {
                let label = n.to_string();
                let target = state.borrow().labels.get(&label).copied();
                if let Some(pos) = target {
                    // Count DATA values before this position
                    let mut count = 0;
                    for (idx, stmt) in program.iter().enumerate() {
                        if idx >= pos {
                            break;
                        }
                        if let Stmt::Data(values) = stmt {
                            count += values.len();
                        }
                    }
                    state.borrow_mut().data_pointer = count;
                }
            } else {
                state.borrow_mut().data_pointer = 0;
            }
            StmtResult::Continue
        }

        Stmt::Expression(expr) => {
            // Evaluate the expression for side effects (like function calls)
            match eval_expr_core(state, expr) {
                Ok(_) => StmtResult::Continue,
                Err(e) => StmtResult::Error(e),
            }
        }
    }
}

/// Execute a subroutine from a given position until RETURN
#[async_recursion(?Send)]
async fn execute_subroutine(
    co: &Co<YieldReason>,
    state: &Rc<RefCell<InterpreterState>>,
    program: &[Stmt],
    start_pos: usize,
) -> StmtResult {
    let mut pos = start_pos;

    while pos < program.len() {
        if state.borrow().stop_requested {
            return StmtResult::End;
        }

        let stmt = &program[pos];

        // Check for RETURN
        if matches!(stmt, Stmt::Return(_)) {
            return StmtResult::Continue;
        }

        state.borrow_mut().current_line = pos;

        let result = execute_stmt(co, state, stmt, program).await;

        match result {
            StmtResult::Continue => pos += 1,
            StmtResult::Jump(new_pos) => pos = new_pos,
            StmtResult::End => return StmtResult::End,
            StmtResult::Error(e) => return StmtResult::Error(e),
        }
    }

    StmtResult::Continue
}

/// Evaluate an expression (sync version - doesn't need async since no yields)
fn eval_expr_core(
    state: &Rc<RefCell<InterpreterState>>,
    expr: &Expr,
) -> Result<Value, String> {
    match expr {
        Expr::Integer(n) => Ok(Value::Integer(*n)),

        Expr::Float(n) => Ok(Value::Float(*n)),

        Expr::String(s) => Ok(Value::String(s.clone())),

        Expr::Variable(name) => {
            let name_upper = name.to_uppercase();

            // Special case for INKEY$
            if name_upper == "INKEY$" {
                let key = state.borrow_mut().last_key.take();
                return Ok(Value::String(key.map_or(String::new(), |c| c.to_string())));
            }

            // Special case for screen dimension pseudo-variables
            if name_upper == "SCREENWIDTH" {
                return Ok(Value::Integer(state.borrow().graphics.width as i64));
            }
            if name_upper == "SCREENHEIGHT" {
                return Ok(Value::Integer(state.borrow().graphics.height as i64));
            }

            // Check local scope first
            {
                let s = state.borrow();
                if let Some(local_scope) = s.call_stack.last() {
                    if let Some(v) = local_scope.get(name) {
                        return Ok(v.clone());
                    }
                }
            }

            // Then check global scope
            let var = state.borrow().variables.get(name).cloned();
            match var {
                Some(v) => Ok(v),
                None => {
                    // Return default value based on type suffix
                    if name.ends_with('$') {
                        Ok(Value::String(String::new()))
                    } else {
                        Ok(Value::Integer(0))
                    }
                }
            }
        }

        Expr::ArrayAccess(name, indices) => {
            let idx_values: Vec<i64> = {
                let mut results = Vec::new();
                for idx in indices {
                    results.push(eval_expr_core(state, idx)?.to_int());
                }
                results
            };

            let arr = state.borrow().variables.get(name).cloned();
            match arr {
                Some(Value::IntArray(arr)) => {
                    let idx = idx_values[0] as usize;
                    Ok(arr.get(idx).map(|&v| Value::Integer(v)).unwrap_or(Value::Integer(0)))
                }
                Some(Value::FloatArray(arr)) => {
                    let idx = idx_values[0] as usize;
                    Ok(arr.get(idx).map(|&v| Value::Float(v)).unwrap_or(Value::Float(0.0)))
                }
                Some(Value::StringArray(arr)) => {
                    let idx = idx_values[0] as usize;
                    Ok(arr.get(idx).map(|v| Value::String(v.clone())).unwrap_or(Value::String(String::new())))
                }
                _ => {
                    // Auto-create array
                    let size = (idx_values[0] + 11) as usize;
                    if name.ends_with('$') {
                        let arr = vec![String::new(); size];
                        state.borrow_mut().variables.insert(name.clone(), Value::StringArray(arr));
                        Ok(Value::String(String::new()))
                    } else {
                        let arr = vec![0.0; size];
                        state.borrow_mut().variables.insert(name.clone(), Value::FloatArray(arr));
                        Ok(Value::Float(0.0))
                    }
                }
            }
        }

        Expr::BinaryOp(left, op, right) => {
            let l = eval_expr_core(state, left)?;

            // Short-circuit AND/OR
            match op {
                BinOp::And => {
                    if !l.is_truthy() {
                        return Ok(Value::Integer(0));
                    }
                    let r = eval_expr_core(state, right)?;
                    return Ok(Value::Integer(if r.is_truthy() { -1 } else { 0 }));
                }
                BinOp::Or => {
                    if l.is_truthy() {
                        return Ok(Value::Integer(-1));
                    }
                    let r = eval_expr_core(state, right)?;
                    return Ok(Value::Integer(if r.is_truthy() { -1 } else { 0 }));
                }
                _ => {}
            }

            let r = eval_expr_core(state, right)?;

            match op {
                BinOp::Add => match (&l, &r) {
                    (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
                    (Value::String(a), b) => Ok(Value::String(format!("{}{}", a, b.to_string()))),
                    (a, Value::String(b)) => Ok(Value::String(format!("{}{}", a.to_string(), b))),
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
                    _ => Ok(Value::Float(l.to_float() + r.to_float())),
                },
                BinOp::Sub => match (&l, &r) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                    _ => Ok(Value::Float(l.to_float() - r.to_float())),
                },
                BinOp::Mul => match (&l, &r) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                    _ => Ok(Value::Float(l.to_float() * r.to_float())),
                },
                BinOp::Div => Ok(Value::Float(l.to_float() / r.to_float())),
                BinOp::IntDiv => Ok(Value::Integer(l.to_int() / r.to_int().max(1))),
                BinOp::Mod => Ok(Value::Integer(l.to_int() % r.to_int().max(1))),
                BinOp::Pow => Ok(Value::Float(l.to_float().powf(r.to_float()))),
                BinOp::Eq => match (&l, &r) {
                    (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a == b { -1 } else { 0 })),
                    _ => Ok(Value::Integer(if (l.to_float() - r.to_float()).abs() < f64::EPSILON { -1 } else { 0 })),
                },
                BinOp::Ne => match (&l, &r) {
                    (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a != b { -1 } else { 0 })),
                    _ => Ok(Value::Integer(if (l.to_float() - r.to_float()).abs() >= f64::EPSILON { -1 } else { 0 })),
                },
                BinOp::Lt => match (&l, &r) {
                    (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a < b { -1 } else { 0 })),
                    _ => Ok(Value::Integer(if l.to_float() < r.to_float() { -1 } else { 0 })),
                },
                BinOp::Gt => match (&l, &r) {
                    (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a > b { -1 } else { 0 })),
                    _ => Ok(Value::Integer(if l.to_float() > r.to_float() { -1 } else { 0 })),
                },
                BinOp::Le => match (&l, &r) {
                    (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a <= b { -1 } else { 0 })),
                    _ => Ok(Value::Integer(if l.to_float() <= r.to_float() { -1 } else { 0 })),
                },
                BinOp::Ge => match (&l, &r) {
                    (Value::String(a), Value::String(b)) => Ok(Value::Integer(if a >= b { -1 } else { 0 })),
                    _ => Ok(Value::Integer(if l.to_float() >= r.to_float() { -1 } else { 0 })),
                },
                BinOp::And | BinOp::Or => unreachable!(), // handled above
                BinOp::Xor => {
                    let a = l.is_truthy();
                    let b = r.is_truthy();
                    Ok(Value::Integer(if a != b { -1 } else { 0 }))
                },
                BinOp::Eqv => {
                    let a = l.is_truthy();
                    let b = r.is_truthy();
                    Ok(Value::Integer(if a == b { -1 } else { 0 }))
                },
                BinOp::Imp => {
                    let a = l.is_truthy();
                    let b = r.is_truthy();
                    Ok(Value::Integer(if !a || b { -1 } else { 0 }))
                },
            }
        }

        Expr::UnaryOp(op, operand) => {
            let v = eval_expr_core(state, operand)?;
            match op {
                UnaryOp::Neg => match v {
                    Value::Integer(i) => Ok(Value::Integer(-i)),
                    _ => Ok(Value::Float(-v.to_float())),
                },
                UnaryOp::Not => Ok(Value::Integer(if !v.is_truthy() { -1 } else { 0 })),
            }
        }

        Expr::FunctionCall(name, args) => {
            let name_upper = name.to_uppercase();

            // Check for user-defined functions
            let func = state.borrow().procedures.get(&name_upper).cloned();
            if let Some(procedure) = func {
                if procedure.is_function {
                    // User-defined functions can't be called from sync eval context
                    return Err(format!("Cannot call function {} in sync context", name));
                }
            }

            // Evaluate arguments for built-in functions
            let arg_values: Vec<Value> = {
                let mut results = Vec::new();
                for arg in args {
                    results.push(eval_expr_core(state, arg)?);
                }
                results
            };

            // Built-in functions
            match name_upper.trim_end_matches('$') {
                "ABS" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().abs()).unwrap_or(0.0))),
                "INT" => Ok(Value::Integer(arg_values.first().map(|v| v.to_float().floor() as i64).unwrap_or(0))),
                "FIX" => Ok(Value::Integer(arg_values.first().map(|v| v.to_float().trunc() as i64).unwrap_or(0))),
                "SGN" => {
                    let f = arg_values.first().map(|v| v.to_float()).unwrap_or(0.0);
                    Ok(Value::Integer(if f > 0.0 { 1 } else if f < 0.0 { -1 } else { 0 }))
                },
                "SQR" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().sqrt()).unwrap_or(0.0))),
                "SIN" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().sin()).unwrap_or(0.0))),
                "COS" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().cos()).unwrap_or(0.0))),
                "TAN" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().tan()).unwrap_or(0.0))),
                "ATN" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().atan()).unwrap_or(0.0))),
                "LOG" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().ln()).unwrap_or(0.0))),
                "EXP" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().exp()).unwrap_or(0.0))),

                "LEN" => Ok(Value::Integer(arg_values.first().map(|v| v.to_string().len() as i64).unwrap_or(0))),
                "LEFT" => {
                    let s = arg_values.first().map(|v| v.to_string()).unwrap_or_default();
                    let n = arg_values.get(1).map(|v| v.to_int()).unwrap_or(0) as usize;
                    Ok(Value::String(s.chars().take(n).collect()))
                },
                "RIGHT" => {
                    let s = arg_values.first().map(|v| v.to_string()).unwrap_or_default();
                    let n = arg_values.get(1).map(|v| v.to_int()).unwrap_or(0) as usize;
                    let len = s.chars().count();
                    Ok(Value::String(s.chars().skip(len.saturating_sub(n)).collect()))
                },
                "MID" => {
                    let s = arg_values.first().map(|v| v.to_string()).unwrap_or_default();
                    let start = (arg_values.get(1).map(|v| v.to_int()).unwrap_or(1) - 1).max(0) as usize;
                    let len = arg_values.get(2).map(|v| v.to_int() as usize);
                    let chars: Vec<char> = s.chars().collect();
                    let result: String = match len {
                        Some(l) => chars.iter().skip(start).take(l).collect(),
                        None => chars.iter().skip(start).collect(),
                    };
                    Ok(Value::String(result))
                },
                "INSTR" => {
                    let (start, s1, s2) = if arg_values.len() >= 3 {
                        (
                            arg_values[0].to_int().max(1) as usize - 1,
                            arg_values[1].to_string(),
                            arg_values[2].to_string(),
                        )
                    } else {
                        (
                            0,
                            arg_values.first().map(|v| v.to_string()).unwrap_or_default(),
                            arg_values.get(1).map(|v| v.to_string()).unwrap_or_default(),
                        )
                    };
                    let haystack = &s1[start.min(s1.len())..];
                    let pos = haystack.find(&s2).map(|p| p + start + 1).unwrap_or(0);
                    Ok(Value::Integer(pos as i64))
                },
                "CHR" => {
                    let code = arg_values.first().map(|v| v.to_int()).unwrap_or(0);
                    let ch = cp437_to_unicode(code as u8);
                    Ok(Value::String(ch.to_string()))
                },
                "ASC" => {
                    let s = arg_values.first().map(|v| v.to_string()).unwrap_or_default();
                    let code = s.chars().next().map(|c| c as i64).unwrap_or(0);
                    Ok(Value::Integer(code))
                },
                "STR" => {
                    let n = arg_values.first().map(|v| v.to_float()).unwrap_or(0.0);
                    let s = if n >= 0.0 {
                        format!(" {}", format_number(n))
                    } else {
                        format_number(n)
                    };
                    Ok(Value::String(s))
                },
                "VAL" => {
                    let s = arg_values.first().map(|v| v.to_string()).unwrap_or_default();
                    let n: f64 = s.trim().parse().unwrap_or(0.0);
                    Ok(Value::Float(n))
                },
                "UCASE" => {
                    let s = arg_values.first().map(|v| v.to_string()).unwrap_or_default();
                    Ok(Value::String(s.to_uppercase()))
                },
                "LCASE" => {
                    let s = arg_values.first().map(|v| v.to_string()).unwrap_or_default();
                    Ok(Value::String(s.to_lowercase()))
                },
                "LTRIM" => {
                    let s = arg_values.first().map(|v| v.to_string()).unwrap_or_default();
                    Ok(Value::String(s.trim_start().to_string()))
                },
                "RTRIM" => {
                    let s = arg_values.first().map(|v| v.to_string()).unwrap_or_default();
                    Ok(Value::String(s.trim_end().to_string()))
                },
                "SPACE" => {
                    let n = arg_values.first().map(|v| v.to_int()).unwrap_or(0);
                    Ok(Value::String(" ".repeat(n.max(0) as usize)))
                },
                "STRING" => {
                    let count = arg_values.first().map(|v| v.to_int()).unwrap_or(0).max(0) as usize;
                    let c = match arg_values.get(1) {
                        Some(Value::String(s)) => s.chars().next().unwrap_or(' '),
                        Some(v) => char::from_u32(v.to_int() as u32).unwrap_or(' '),
                        None => ' ',
                    };
                    Ok(Value::String(c.to_string().repeat(count)))
                },

                "RND" => Ok(Value::Float(rnd())),
                "TIMER" => Ok(Value::Float(state.borrow().start_time.elapsed().as_secs_f64())),

                "CINT" => Ok(Value::Integer(arg_values.first().map(|v| v.to_float().round() as i64).unwrap_or(0))),
                "CLNG" => Ok(Value::Integer(arg_values.first().map(|v| v.to_float().round() as i64).unwrap_or(0))),
                "CSNG" | "CDBL" => Ok(Value::Float(arg_values.first().map(|v| v.to_float()).unwrap_or(0.0))),

                "HEX" => {
                    let n = arg_values.first().map(|v| v.to_int()).unwrap_or(0);
                    Ok(Value::String(format!("{:X}", n)))
                },
                "OCT" => {
                    let n = arg_values.first().map(|v| v.to_int()).unwrap_or(0);
                    Ok(Value::String(format!("{:o}", n)))
                },

                "POINT" => {
                    let x = arg_values.first().map(|v| v.to_int()).unwrap_or(0) as i32;
                    let y = arg_values.get(1).map(|v| v.to_int()).unwrap_or(0) as i32;
                    let color = state.borrow().graphics.point(x, y);
                    Ok(Value::Integer(color as i64))
                },

                "CSRLIN" => Ok(Value::Integer(state.borrow().graphics.cursor_row as i64)),
                "POS" => Ok(Value::Integer(state.borrow().graphics.cursor_col as i64)),

                "SCREENWIDTH" => Ok(Value::Integer(state.borrow().graphics.width as i64)),
                "SCREENHEIGHT" => Ok(Value::Integer(state.borrow().graphics.height as i64)),

                "LBOUND" => Ok(Value::Integer(0)),
                "UBOUND" => {
                    if let Some(arr_name) = args.first() {
                        if let Expr::Variable(name) = arr_name {
                            let arr = state.borrow().variables.get(name).cloned();
                            match arr {
                                Some(Value::IntArray(a)) => Ok(Value::Integer((a.len() - 1) as i64)),
                                Some(Value::FloatArray(a)) => Ok(Value::Integer((a.len() - 1) as i64)),
                                Some(Value::StringArray(a)) => Ok(Value::Integer((a.len() - 1) as i64)),
                                _ => Ok(Value::Integer(0)),
                            }
                        } else {
                            Ok(Value::Integer(0))
                        }
                    } else {
                        Ok(Value::Integer(0))
                    }
                },

                _ => Err(format!("Unknown function: {}", name)),
            }
        }

        Expr::Paren(inner) => eval_expr_core(state, inner),
    }
}

/// Synchronous expression evaluation (for immediate window)
fn eval_expr_sync(state: &Rc<RefCell<InterpreterState>>, expr: &Expr) -> Result<Value, String> {
    match expr {
        Expr::Integer(n) => Ok(Value::Integer(*n)),
        Expr::Float(n) => Ok(Value::Float(*n)),
        Expr::String(s) => Ok(Value::String(s.clone())),

        Expr::Variable(name) => {
            let name_upper = name.to_uppercase();

            if name_upper == "INKEY$" {
                let key = state.borrow_mut().last_key.take();
                return Ok(Value::String(key.map_or(String::new(), |c| c.to_string())));
            }

            if name_upper == "SCREENWIDTH" {
                return Ok(Value::Integer(state.borrow().graphics.width as i64));
            }
            if name_upper == "SCREENHEIGHT" {
                return Ok(Value::Integer(state.borrow().graphics.height as i64));
            }

            let var = state.borrow().variables.get(name).cloned();
            match var {
                Some(v) => Ok(v),
                None => {
                    if name.ends_with('$') {
                        Ok(Value::String(String::new()))
                    } else {
                        Ok(Value::Integer(0))
                    }
                }
            }
        }

        Expr::ArrayAccess(name, indices) => {
            let idx_values: Vec<i64> = indices
                .iter()
                .map(|idx| eval_expr_sync(state, idx).map(|v| v.to_int()))
                .collect::<Result<_, _>>()?;

            let arr = state.borrow().variables.get(name).cloned();
            match arr {
                Some(Value::IntArray(arr)) => {
                    let idx = idx_values[0] as usize;
                    Ok(arr.get(idx).map(|&v| Value::Integer(v)).unwrap_or(Value::Integer(0)))
                }
                Some(Value::FloatArray(arr)) => {
                    let idx = idx_values[0] as usize;
                    Ok(arr.get(idx).map(|&v| Value::Float(v)).unwrap_or(Value::Float(0.0)))
                }
                Some(Value::StringArray(arr)) => {
                    let idx = idx_values[0] as usize;
                    Ok(arr.get(idx).map(|v| Value::String(v.clone())).unwrap_or(Value::String(String::new())))
                }
                _ => Ok(Value::Integer(0)),
            }
        }

        Expr::BinaryOp(left, op, right) => {
            let l = eval_expr_sync(state, left)?;
            let r = eval_expr_sync(state, right)?;

            match op {
                BinOp::Add => match (&l, &r) {
                    (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
                    _ => Ok(Value::Float(l.to_float() + r.to_float())),
                },
                BinOp::Sub => match (&l, &r) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                    _ => Ok(Value::Float(l.to_float() - r.to_float())),
                },
                BinOp::Mul => match (&l, &r) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                    _ => Ok(Value::Float(l.to_float() * r.to_float())),
                },
                BinOp::Div => Ok(Value::Float(l.to_float() / r.to_float())),
                BinOp::IntDiv => Ok(Value::Integer(l.to_int() / r.to_int().max(1))),
                BinOp::Mod => Ok(Value::Integer(l.to_int() % r.to_int().max(1))),
                BinOp::Pow => Ok(Value::Float(l.to_float().powf(r.to_float()))),
                BinOp::Eq => Ok(Value::Integer(if (l.to_float() - r.to_float()).abs() < f64::EPSILON { -1 } else { 0 })),
                BinOp::Ne => Ok(Value::Integer(if (l.to_float() - r.to_float()).abs() >= f64::EPSILON { -1 } else { 0 })),
                BinOp::Lt => Ok(Value::Integer(if l.to_float() < r.to_float() { -1 } else { 0 })),
                BinOp::Gt => Ok(Value::Integer(if l.to_float() > r.to_float() { -1 } else { 0 })),
                BinOp::Le => Ok(Value::Integer(if l.to_float() <= r.to_float() { -1 } else { 0 })),
                BinOp::Ge => Ok(Value::Integer(if l.to_float() >= r.to_float() { -1 } else { 0 })),
                BinOp::And => Ok(Value::Integer(if l.is_truthy() && r.is_truthy() { -1 } else { 0 })),
                BinOp::Or => Ok(Value::Integer(if l.is_truthy() || r.is_truthy() { -1 } else { 0 })),
                BinOp::Xor => Ok(Value::Integer(if l.is_truthy() != r.is_truthy() { -1 } else { 0 })),
                BinOp::Eqv => Ok(Value::Integer(if l.is_truthy() == r.is_truthy() { -1 } else { 0 })),
                BinOp::Imp => Ok(Value::Integer(if !l.is_truthy() || r.is_truthy() { -1 } else { 0 })),
            }
        }

        Expr::UnaryOp(op, operand) => {
            let v = eval_expr_sync(state, operand)?;
            match op {
                UnaryOp::Neg => match v {
                    Value::Integer(i) => Ok(Value::Integer(-i)),
                    _ => Ok(Value::Float(-v.to_float())),
                },
                UnaryOp::Not => Ok(Value::Integer(if !v.is_truthy() { -1 } else { 0 })),
            }
        }

        Expr::FunctionCall(name, args) => {
            let name_upper = name.to_uppercase();
            let arg_values: Vec<Value> = args
                .iter()
                .map(|arg| eval_expr_sync(state, arg))
                .collect::<Result<_, _>>()?;

            match name_upper.trim_end_matches('$') {
                "ABS" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().abs()).unwrap_or(0.0))),
                "INT" => Ok(Value::Integer(arg_values.first().map(|v| v.to_float().floor() as i64).unwrap_or(0))),
                "SQR" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().sqrt()).unwrap_or(0.0))),
                "SIN" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().sin()).unwrap_or(0.0))),
                "COS" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().cos()).unwrap_or(0.0))),
                "TAN" => Ok(Value::Float(arg_values.first().map(|v| v.to_float().tan()).unwrap_or(0.0))),
                "LEN" => Ok(Value::Integer(arg_values.first().map(|v| v.to_string().len() as i64).unwrap_or(0))),
                "VAL" => {
                    let s = arg_values.first().map(|v| v.to_string()).unwrap_or_default();
                    Ok(Value::Float(s.trim().parse().unwrap_or(0.0)))
                },
                "STR" => {
                    let n = arg_values.first().map(|v| v.to_float()).unwrap_or(0.0);
                    Ok(Value::String(format_number(n)))
                },
                "RND" => Ok(Value::Float(rnd())),
                _ => Err(format!("Unknown function: {}", name)),
            }
        }

        Expr::Paren(inner) => eval_expr_sync(state, inner),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basic::lexer::Lexer;
    use crate::basic::parser::Parser;

    fn run_basic(code: &str) -> Result<String, String> {
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let stmts = parser.parse()?;
        let mut interp = Interpreter::new();
        interp.execute(&stmts)?;
        Ok(interp.take_output().join("\n"))
    }

    #[test]
    fn test_simple_print() {
        let output = run_basic("PRINT \"Hello\"").expect("Should run");
        assert_eq!(output, "Hello");
    }

    #[test]
    fn test_variable_assignment() {
        let output = run_basic("x = 42\nPRINT x").expect("Should run");
        assert_eq!(output, "42");
    }

    #[test]
    fn test_arithmetic() {
        let output = run_basic("PRINT 2 + 3 * 4").expect("Should run");
        assert_eq!(output, "14");
    }

    #[test]
    fn test_for_loop() {
        let output = run_basic("FOR i = 1 TO 3\nPRINT i\nNEXT i").expect("Should run");
        assert_eq!(output, "1\n2\n3");
    }

    #[test]
    fn test_while_loop() {
        let output = run_basic("x = 0\nWHILE x < 3\nx = x + 1\nPRINT x\nWEND").expect("Should run");
        assert_eq!(output, "1\n2\n3");
    }

    #[test]
    fn test_if_then_else() {
        let output = run_basic("x = 5\nIF x > 3 THEN\nPRINT \"big\"\nELSE\nPRINT \"small\"\nEND IF").expect("Should run");
        assert_eq!(output, "big");
    }

    #[test]
    fn test_gosub() {
        let output = run_basic("GOSUB 100\nPRINT \"after\"\nEND\n100 PRINT \"in sub\"\nRETURN").expect("Should run");
        assert_eq!(output, "in sub\nafter");
    }

    #[test]
    fn test_array() {
        let output = run_basic("DIM a(5)\na(0) = 10\na(1) = 20\nPRINT a(0) + a(1)").expect("Should run");
        assert_eq!(output, "30");
    }

    #[test]
    fn test_string_functions() {
        let output = run_basic("PRINT LEFT$(\"HELLO\", 2)").expect("Should run");
        assert_eq!(output, "HE");
    }

    #[test]
    fn test_rnd() {
        let output = run_basic("x = RND\nIF x >= 0 AND x < 1 THEN PRINT \"OK\"").expect("Should run");
        assert_eq!(output, "OK");
    }

    #[test]
    fn test_nested_for() {
        let output = run_basic("FOR i = 1 TO 2\nFOR j = 1 TO 2\nPRINT i; j\nNEXT j\nNEXT i").expect("Should run");
        assert!(output.contains("1") && output.contains("2"));
    }

    #[test]
    fn test_gosub_with_loop() {
        // This was the problematic case: GOSUB with a loop inside
        let code = r#"
GOSUB 100
PRINT "done"
END
100 FOR i = 1 TO 3
PRINT i
NEXT i
RETURN
"#;
        let output = run_basic(code).expect("Should run");
        assert!(output.contains("1"));
        assert!(output.contains("2"));
        assert!(output.contains("3"));
        assert!(output.contains("done"));
    }

    #[test]
    fn test_data_read() {
        let code = r#"
DATA 10, 20, 30
READ a, b, c
PRINT a + b + c
"#;
        let output = run_basic(code).expect("Should run");
        assert_eq!(output.trim(), "60");
    }

    #[test]
    fn test_and_operator() {
        let code = r#"
a = 5
b = 5
c = 3
d = 3

' Test both conditions true
IF a = 5 AND b = 5 THEN
    PRINT "Both true works"
END IF

' Test one false
IF a = 5 AND c = 99 THEN
    PRINT "Should not print"
ELSE
    PRINT "One false works"
END IF

' Test with variables
IF a = b AND c = d THEN
    PRINT "Variable comparison works"
END IF
"#;
        let output = run_basic(code).expect("Should run");
        assert!(output.contains("Both true works"), "Output: {}", output);
        assert!(output.contains("One false works"), "Output: {}", output);
        assert!(output.contains("Variable comparison works"), "Output: {}", output);
        assert!(!output.contains("Should not print"), "Output: {}", output);
    }

    #[test]
    fn test_gosub_locate_print_updates_graphics() {
        let code = r#"
WIDTH = 78
HEIGHT = 22
foodX = 40
foodY = 10

' Draw food using subroutine (exactly like nibbles.bas)
GOSUB 800
PRINT "Food drawn"
END

' === DRAW FOOD SUBROUTINE ===
800 COLOR 12, 1
LOCATE foodY, foodX
PRINT "*";
RETURN
"#;
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let stmts = parser.parse().expect("Should parse");
        let mut interp = Interpreter::new();
        interp.with_graphics_mut(|g| {
            g.resize(80, 25);
            g.mode = 12;  // Enable graphics mode
        });
        interp.execute(&stmts).expect("Should execute");

        // Check that the food character is in the graphics buffer at the correct position
        interp.with_graphics(|g| {
            let cell = g.get_char(10, 40);  // row 10, col 40
            assert_eq!(cell.char, '*', "Food character should be '*' at position (10, 40)");
            assert_eq!(cell.fg, 12, "Food foreground color should be 12 (bright red)");
            assert_eq!(cell.bg, 1, "Food background color should be 1 (blue)");
        });
    }

    #[test]
    fn test_rnd_variation() {
        // Test that RND produces varied values (RND can be called without parentheses)
        let code = r#"
FOR i = 1 TO 10
    PRINT RND
NEXT i
"#;
        let output = run_basic(code).expect("Should run");
        // Check that values are varied (not all the same)
        let lines: Vec<&str> = output.trim().lines().collect();
        let unique: std::collections::HashSet<&str> = lines.iter().cloned().collect();
        assert!(unique.len() > 1, "Expected varied RND values, got: {}", output);
    }
}
