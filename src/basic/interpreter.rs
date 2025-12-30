//! BASIC interpreter - executes the AST

use std::collections::HashMap;
use std::io::{self, Write};
use crate::basic::parser::{Stmt, Expr, BinOp, UnaryOp, PrintItem};
use crate::basic::builtins;
use crate::basic::graphics::GraphicsMode;

/// Runtime value types
#[derive(Clone, Debug)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>, Vec<usize>), // Values and dimensions
}

impl Value {
    pub fn to_int(&self) -> i64 {
        match self {
            Value::Integer(n) => *n,
            Value::Float(n) => *n as i64,
            Value::String(s) => s.parse().unwrap_or(0),
            Value::Array(_, _) => 0,
        }
    }

    pub fn to_float(&self) -> f64 {
        match self {
            Value::Integer(n) => *n as f64,
            Value::Float(n) => *n,
            Value::String(s) => s.parse().unwrap_or(0.0),
            Value::Array(_, _) => 0.0,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Value::Integer(n) => n.to_string(),
            Value::Float(n) => format!("{}", n),
            Value::String(s) => s.clone(),
            Value::Array(_, _) => "[Array]".to_string(),
        }
    }

    pub fn is_true(&self) -> bool {
        self.to_int() != 0
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Integer(0)
    }
}

/// Execution result with potential breakpoint hit
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    /// Completed normally
    Completed,
    /// Hit a breakpoint at the given line (0-indexed)
    Breakpoint(usize),
    /// In step mode, stopped after one statement at line
    Stepped(usize),
    /// Needs keyboard input (for INKEY$) - yields to allow UI update
    NeedsInput,
}

/// A procedure (SUB or FUNCTION) definition
#[derive(Clone)]
#[allow(dead_code)]
pub struct Procedure {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub is_function: bool,
}

/// Execution state
pub struct Interpreter {
    /// Global variables
    variables: HashMap<String, Value>,

    /// DATA values
    data: Vec<Value>,
    data_ptr: usize,

    /// GOSUB return stack
    gosub_stack: Vec<usize>,

    /// Line labels for GOTO/GOSUB
    labels: HashMap<i64, usize>,

    /// Current execution position (statement index)
    current_pos: usize,

    /// Should stop execution
    should_stop: bool,

    /// Graphics mode
    pub graphics: GraphicsMode,

    /// Output buffer for PRINT statements
    output_buffer: Vec<String>,

    /// Input callback (returns user input)
    input_fn: Option<Box<dyn Fn(&str) -> String>>,

    /// Random number generator state
    rng_state: u64,

    /// Breakpoint lines (0-indexed)
    breakpoints: std::collections::HashSet<usize>,

    /// Step mode - stop after each statement
    step_mode: bool,

    /// Source line mapping (statement index -> source line)
    line_mapping: Vec<usize>,

    /// User-defined procedures (SUBs and FUNCTIONs)
    pub procedures: HashMap<String, Procedure>,

    /// Call stack for local variable scopes
    call_stack: Vec<HashMap<String, Value>>,

    /// Pending key for INKEY$
    pub pending_key: Option<String>,

    /// Flag set when INKEY$ was called and returned empty - signals need to yield
    pub needs_input: bool,

    /// Last time we yielded for display update (in milliseconds since epoch)
    last_yield_time: u128,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            data: Vec::new(),
            data_ptr: 0,
            gosub_stack: Vec::new(),
            labels: HashMap::new(),
            current_pos: 0,
            should_stop: false,
            graphics: GraphicsMode::new(80, 25),
            output_buffer: Vec::new(),
            input_fn: None,
            rng_state: 12345,
            breakpoints: std::collections::HashSet::new(),
            step_mode: false,
            line_mapping: Vec::new(),
            procedures: HashMap::new(),
            call_stack: Vec::new(),
            pending_key: None,
            needs_input: false,
            last_yield_time: 0,
        }
    }

    fn current_time_millis() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }

    /// Set breakpoints (0-indexed line numbers)
    pub fn set_breakpoints(&mut self, lines: &[usize]) {
        self.breakpoints.clear();
        for &line in lines {
            self.breakpoints.insert(line);
        }
    }

    /// Enable or disable step mode
    pub fn set_step_mode(&mut self, enabled: bool) {
        self.step_mode = enabled;
    }

    /// Get output buffer
    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output_buffer)
    }

    /// Get a variable value, checking local scope first
    fn get_variable(&self, name: &str) -> Option<Value> {
        let name_upper = name.to_uppercase();
        // Check local scope first (call stack)
        if let Some(local_vars) = self.call_stack.last() {
            if let Some(value) = local_vars.get(&name_upper) {
                return Some(value.clone());
            }
        }
        // Fall back to global scope
        self.variables.get(&name_upper).cloned()
    }

    /// Set a variable value, using local scope if in a procedure
    fn set_variable(&mut self, name: &str, value: Value) {
        let name_upper = name.to_uppercase();
        // If we're in a procedure and this is a local variable, update local scope
        if let Some(local_vars) = self.call_stack.last_mut() {
            if local_vars.contains_key(&name_upper) {
                local_vars.insert(name_upper, value);
                return;
            }
        }
        // Otherwise update global scope
        self.variables.insert(name_upper, value);
    }

    /// Clear state for new execution
    pub fn reset(&mut self) {
        self.variables.clear();
        self.data.clear();
        self.data_ptr = 0;
        self.gosub_stack.clear();
        self.labels.clear();
        self.current_pos = 0;
        self.should_stop = false;
        self.output_buffer.clear();
        self.line_mapping.clear();
        self.procedures.clear();
        self.call_stack.clear();
        self.pending_key = None;
        self.needs_input = false;
        self.last_yield_time = 0;
        // Reset graphics to default text mode (white on black)
        self.graphics = GraphicsMode::new(80, 25);
        self.graphics.set_color(7, 0);  // Light gray on black
        self.graphics.cls();
    }

    /// Build line mapping from statement index to source line
    pub fn build_line_mapping(&mut self, program: &[Stmt]) {
        self.line_mapping.clear();
        // For simplicity, map each statement to its index
        // In a real implementation, we'd get line numbers from the parser
        for i in 0..program.len() {
            self.line_mapping.push(i);
        }
    }

    /// Execute a program (list of statements)
    pub fn execute(&mut self, program: &[Stmt]) -> Result<(), String> {
        match self.execute_with_debug(program)? {
            ExecutionResult::Completed => Ok(()),
            ExecutionResult::Breakpoint(_) => Ok(()), // Treat as completed for non-debug mode
            ExecutionResult::Stepped(_) => Ok(()),
            ExecutionResult::NeedsInput => Ok(()), // Treat as completed for non-debug mode
        }
    }

    /// Execute with debug support (breakpoints and stepping)
    pub fn execute_with_debug(&mut self, program: &[Stmt]) -> Result<ExecutionResult, String> {
        // First pass: collect labels and DATA
        self.labels.clear();
        self.data.clear();
        self.build_line_mapping(program);

        for (i, stmt) in program.iter().enumerate() {
            if let Stmt::Label(line) = stmt {
                self.labels.insert(*line, i);
            }
            if let Stmt::Data(values) = stmt {
                for val in values {
                    let v = self.eval_expr(val)?;
                    self.data.push(v);
                }
            }
        }

        // Execute
        self.current_pos = 0;
        self.should_stop = false;

        while self.current_pos < program.len() && !self.should_stop {
            // Check for breakpoint
            let current_line = if self.current_pos < self.line_mapping.len() {
                self.line_mapping[self.current_pos]
            } else {
                self.current_pos
            };

            if self.breakpoints.contains(&current_line) {
                return Ok(ExecutionResult::Breakpoint(current_line));
            }

            let stmt = &program[self.current_pos];
            self.current_pos += 1;

            match self.execute_stmt(stmt, program) {
                Ok(Some(new_pos)) => {
                    self.current_pos = new_pos;
                }
                Ok(None) => {}
                Err(e) => return Err(e),
            }

            // Check if we need to yield for keyboard input
            if self.needs_input {
                return Ok(ExecutionResult::NeedsInput);
            }

            // Check for step mode
            if self.step_mode {
                let new_line = if self.current_pos < self.line_mapping.len() {
                    self.line_mapping[self.current_pos]
                } else {
                    self.current_pos
                };
                return Ok(ExecutionResult::Stepped(new_line));
            }
        }

        Ok(ExecutionResult::Completed)
    }

    /// Continue execution from current position
    pub fn continue_execution(&mut self, program: &[Stmt]) -> Result<ExecutionResult, String> {
        while self.current_pos < program.len() && !self.should_stop {
            // Check for breakpoint
            let current_line = if self.current_pos < self.line_mapping.len() {
                self.line_mapping[self.current_pos]
            } else {
                self.current_pos
            };

            if self.breakpoints.contains(&current_line) {
                return Ok(ExecutionResult::Breakpoint(current_line));
            }

            let stmt = &program[self.current_pos];
            self.current_pos += 1;

            match self.execute_stmt(stmt, program) {
                Ok(Some(new_pos)) => {
                    self.current_pos = new_pos;
                }
                Ok(None) => {}
                Err(e) => return Err(e),
            }

            // Check if we need to yield for keyboard input
            if self.needs_input {
                return Ok(ExecutionResult::NeedsInput);
            }

            // Check for step mode
            if self.step_mode {
                let new_line = if self.current_pos < self.line_mapping.len() {
                    self.line_mapping[self.current_pos]
                } else {
                    self.current_pos
                };
                return Ok(ExecutionResult::Stepped(new_line));
            }
        }

        Ok(ExecutionResult::Completed)
    }

    /// Execute a single statement
    /// Returns Ok(Some(pos)) to jump to pos, Ok(None) to continue normally
    fn execute_stmt(&mut self, stmt: &Stmt, program: &[Stmt]) -> Result<Option<usize>, String> {
        match stmt {
            Stmt::Empty | Stmt::Label(_) | Stmt::Rem(_) | Stmt::Data(_) => Ok(None),

            Stmt::Let(name, expr) => {
                let value = self.eval_expr(expr)?;
                self.set_variable(name, value);
                Ok(None)
            }

            Stmt::ArrayLet(name, indices, expr) => {
                let value = self.eval_expr(expr)?;
                let idx: Vec<usize> = indices.iter()
                    .map(|e| self.eval_expr(e).map(|v| v.to_int() as usize))
                    .collect::<Result<Vec<_>, _>>()?;

                let name_upper = name.to_uppercase();

                // Get or create array
                if !self.variables.contains_key(&name_upper) {
                    // Auto-dimension to 10
                    let size = 11usize.pow(idx.len() as u32);
                    let dims = vec![11; idx.len()];
                    self.variables.insert(name_upper.clone(), Value::Array(vec![Value::Integer(0); size], dims));
                }

                if let Some(Value::Array(arr, dims)) = self.variables.get_mut(&name_upper) {
                    // Calculate linear index
                    let mut linear_idx = 0;
                    let mut multiplier = 1;
                    for (i, &dim_idx) in idx.iter().rev().enumerate() {
                        if i < dims.len() {
                            linear_idx += dim_idx * multiplier;
                            multiplier *= dims[dims.len() - 1 - i];
                        }
                    }
                    if linear_idx < arr.len() {
                        arr[linear_idx] = value;
                    }
                }
                Ok(None)
            }

            Stmt::Print(items) => {
                let mut output = String::new();

                for item in items {
                    match item {
                        PrintItem::Expr(expr) => {
                            let val = self.eval_expr(expr)?;
                            let s = val.to_string();
                            output.push_str(&s);
                        }
                        PrintItem::Tab(expr) => {
                            let tab_col = self.eval_expr(expr)?.to_int() as usize;
                            let current_col = self.graphics.cursor_col as usize;
                            while output.len() + current_col < tab_col {
                                output.push(' ');
                            }
                        }
                        PrintItem::Spc(expr) => {
                            let spaces = self.eval_expr(expr)?.to_int() as usize;
                            for _ in 0..spaces {
                                output.push(' ');
                            }
                        }
                        PrintItem::Comma => {
                            // Tab to next 14-column zone
                            let current_col = self.graphics.cursor_col as usize + output.len();
                            let next_zone = ((current_col / 14) + 1) * 14;
                            while output.len() + self.graphics.cursor_col as usize - 1 < next_zone {
                                output.push(' ');
                            }
                        }
                        PrintItem::Semicolon => {
                            // No space, continue at current position
                        }
                    }
                }

                // Print to text screen
                let no_newline = items.last().map(|i| matches!(i, PrintItem::Semicolon)).unwrap_or(false);
                self.graphics.print_text(&output, !no_newline);

                // Also add to output buffer for debugging
                if !no_newline {
                    self.output_buffer.push(output);
                }
                Ok(None)
            }

            Stmt::Input(prompt, vars) => {
                if let Some(p) = prompt {
                    self.output_buffer.push(p.clone());
                }

                // For now, just set to default values
                // Real implementation would wait for input
                for var in vars {
                    if let Some(input_fn) = &self.input_fn {
                        let input = input_fn(&format!("{}? ", var));
                        let value = if var.ends_with('$') {
                            Value::String(input)
                        } else if input.contains('.') {
                            Value::Float(input.parse().unwrap_or(0.0))
                        } else {
                            Value::Integer(input.parse().unwrap_or(0))
                        };
                        self.variables.insert(var.to_uppercase(), value);
                    }
                }
                Ok(None)
            }

            Stmt::If { condition, then_branch, else_branch } => {
                let cond = self.eval_expr(condition)?;
                if cond.is_true() {
                    for stmt in then_branch {
                        if let Some(pos) = self.execute_stmt(stmt, program)? {
                            return Ok(Some(pos));
                        }
                    }
                } else if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        if let Some(pos) = self.execute_stmt(stmt, program)? {
                            return Ok(Some(pos));
                        }
                    }
                }
                Ok(None)
            }

            Stmt::For { var, start, end, step, body } => {
                let var_name = var.to_uppercase();
                let start_val = self.eval_expr(start)?.to_float();
                let end_val = self.eval_expr(end)?.to_float();
                let step_val = step.as_ref().map(|s| self.eval_expr(s).map(|v| v.to_float())).transpose()?.unwrap_or(1.0);

                self.variables.insert(var_name.clone(), Value::Float(start_val));

                loop {
                    let current = self.variables.get(&var_name).cloned().unwrap_or(Value::Float(0.0)).to_float();

                    // Check termination
                    let done = if step_val >= 0.0 {
                        current > end_val
                    } else {
                        current < end_val
                    };
                    if done {
                        break;
                    }

                    // Execute body
                    for stmt in body {
                        if let Some(pos) = self.execute_stmt(stmt, program)? {
                            return Ok(Some(pos));
                        }
                    }

                    // Increment
                    let new_val = current + step_val;
                    self.variables.insert(var_name.clone(), Value::Float(new_val));
                }
                Ok(None)
            }

            Stmt::While { condition, body } => {
                loop {
                    let cond = self.eval_expr(condition)?;
                    if !cond.is_true() {
                        break;
                    }
                    for stmt in body {
                        if let Some(pos) = self.execute_stmt(stmt, program)? {
                            return Ok(Some(pos));
                        }
                    }
                }
                Ok(None)
            }

            Stmt::DoLoop { condition, is_while, is_pre_test, body } => {
                loop {
                    // Pre-test
                    if *is_pre_test {
                        if let Some(cond) = condition {
                            let result = self.eval_expr(cond)?.is_true();
                            let should_continue = if *is_while { result } else { !result };
                            if !should_continue {
                                break;
                            }
                        }
                    }

                    // Execute body
                    for stmt in body {
                        if let Some(pos) = self.execute_stmt(stmt, program)? {
                            return Ok(Some(pos));
                        }
                    }

                    // Post-test
                    if !*is_pre_test {
                        if let Some(cond) = condition {
                            let result = self.eval_expr(cond)?.is_true();
                            let should_continue = if *is_while { result } else { !result };
                            if !should_continue {
                                break;
                            }
                        }
                    }

                    // Infinite loop if no condition
                    if condition.is_none() {
                        break;
                    }
                }
                Ok(None)
            }

            Stmt::GoTo(line) => {
                if let Some(&pos) = self.labels.get(line) {
                    Ok(Some(pos))
                } else {
                    Err(format!("Line {} not found", line))
                }
            }

            Stmt::GoSub(line) => {
                if let Some(&pos) = self.labels.get(line) {
                    self.gosub_stack.push(self.current_pos);
                    Ok(Some(pos))
                } else {
                    Err(format!("Line {} not found", line))
                }
            }

            Stmt::Return(_) => {
                if let Some(pos) = self.gosub_stack.pop() {
                    Ok(Some(pos))
                } else {
                    Err("RETURN without GOSUB".to_string())
                }
            }

            Stmt::Dim(vars) => {
                for var in vars {
                    let name = var.name.to_uppercase();
                    if !var.dimensions.is_empty() {
                        let dims: Vec<usize> = var.dimensions.iter()
                            .map(|e| self.eval_expr(e).map(|v| (v.to_int() + 1) as usize))
                            .collect::<Result<Vec<_>, _>>()?;
                        let size: usize = dims.iter().product();
                        let default = if name.ends_with('$') {
                            Value::String(String::new())
                        } else {
                            Value::Integer(0)
                        };
                        self.variables.insert(name, Value::Array(vec![default; size], dims));
                    }
                }
                Ok(None)
            }

            Stmt::End => {
                self.should_stop = true;
                Ok(None)
            }

            Stmt::Stop => {
                self.should_stop = true;
                Ok(None)
            }

            Stmt::Cls => {
                self.graphics.cls();
                Ok(None)
            }

            Stmt::Screen(mode) => {
                let m = self.eval_expr(mode)?.to_int() as u8;
                self.graphics.set_mode(m);
                Ok(None)
            }

            Stmt::Color(fg, bg) => {
                let fg_val = self.eval_expr(fg)?.to_int() as u8;
                let bg_val = bg.as_ref().map(|e| self.eval_expr(e).map(|v| v.to_int() as u8)).transpose()?.unwrap_or(self.graphics.background);
                self.graphics.set_color(fg_val, bg_val);
                Ok(None)
            }

            Stmt::Locate(row, col) => {
                let r = self.eval_expr(row)?.to_int() as u16;
                let c = self.eval_expr(col)?.to_int() as u16;
                self.graphics.locate(r, c);
                Ok(None)
            }

            Stmt::Pset(x, y, color) => {
                let px = self.eval_expr(x)?.to_int() as i32;
                let py = self.eval_expr(y)?.to_int() as i32;
                let c = color.as_ref().map(|e| self.eval_expr(e).map(|v| v.to_int() as u8)).transpose()?.unwrap_or(self.graphics.foreground);
                self.graphics.pset(px, py, c);
                Ok(None)
            }

            Stmt::Line { x1, y1, x2, y2, color, box_fill } => {
                let px1 = self.eval_expr(x1)?.to_int() as i32;
                let py1 = self.eval_expr(y1)?.to_int() as i32;
                let px2 = self.eval_expr(x2)?.to_int() as i32;
                let py2 = self.eval_expr(y2)?.to_int() as i32;
                let c = color.as_ref().map(|e| self.eval_expr(e).map(|v| v.to_int() as u8)).transpose()?.unwrap_or(self.graphics.foreground);

                match box_fill {
                    None => self.graphics.line(px1, py1, px2, py2, c),
                    Some(false) => self.graphics.draw_box(px1, py1, px2, py2, c),
                    Some(true) => self.graphics.fill_box(px1, py1, px2, py2, c),
                }
                Ok(None)
            }

            Stmt::Circle { x, y, radius, color } => {
                let cx = self.eval_expr(x)?.to_int() as i32;
                let cy = self.eval_expr(y)?.to_int() as i32;
                let r = self.eval_expr(radius)?.to_int() as i32;
                let c = color.as_ref().map(|e| self.eval_expr(e).map(|v| v.to_int() as u8)).transpose()?.unwrap_or(self.graphics.foreground);
                self.graphics.circle(cx, cy, r, c);
                Ok(None)
            }

            Stmt::Paint(x, y, color) => {
                let px = self.eval_expr(x)?.to_int() as i32;
                let py = self.eval_expr(y)?.to_int() as i32;
                let c = self.eval_expr(color)?.to_int() as u8;
                self.graphics.paint(px, py, c);
                Ok(None)
            }

            Stmt::Beep => {
                print!("\x07"); // Terminal bell
                let _ = io::stdout().flush();
                Ok(None)
            }

            Stmt::Sound(_, _) | Stmt::Sleep(_) => {
                // Skip for now
                Ok(None)
            }

            Stmt::Randomize(seed) => {
                if let Some(s) = seed {
                    self.rng_state = self.eval_expr(s)?.to_int() as u64;
                } else {
                    self.rng_state = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                }
                Ok(None)
            }

            Stmt::Read(vars) => {
                for var in vars {
                    if self.data_ptr < self.data.len() {
                        let value = self.data[self.data_ptr].clone();
                        self.data_ptr += 1;
                        self.variables.insert(var.to_uppercase(), value);
                    } else {
                        return Err("Out of DATA".to_string());
                    }
                }
                Ok(None)
            }

            Stmt::Restore(line) => {
                if let Some(_line_num) = line {
                    // Find DATA at or after this line
                    self.data_ptr = 0; // For simplicity, reset to start
                } else {
                    self.data_ptr = 0;
                }
                Ok(None)
            }

            Stmt::Expression(expr) => {
                // Just evaluate for side effects
                self.eval_expr(expr)?;
                Ok(None)
            }

            Stmt::Sub { name, params, body } => {
                // Register the SUB for later calls
                self.procedures.insert(name.to_uppercase(), Procedure {
                    name: name.to_uppercase(),
                    params: params.clone(),
                    body: body.clone(),
                    is_function: false,
                });
                Ok(None)
            }

            Stmt::Function { name, params, body } => {
                // Register the FUNCTION for later calls
                self.procedures.insert(name.to_uppercase(), Procedure {
                    name: name.to_uppercase(),
                    params: params.clone(),
                    body: body.clone(),
                    is_function: true,
                });
                Ok(None)
            }

            Stmt::Call(name, args) => {
                self.call_procedure(name, args)?;
                Ok(None)
            }
        }
    }

    /// Call a user-defined procedure (SUB or FUNCTION)
    fn call_procedure(&mut self, name: &str, args: &[Expr]) -> Result<Value, String> {
        let name_upper = name.to_uppercase();

        // Look up the procedure
        let proc = self.procedures.get(&name_upper)
            .ok_or_else(|| format!("Undefined procedure: {}", name))?
            .clone();

        // Evaluate arguments
        let mut arg_values = Vec::new();
        for arg in args {
            arg_values.push(self.eval_expr(arg)?);
        }

        // Create local scope
        let mut local_vars: HashMap<String, Value> = HashMap::new();

        // Bind parameters to arguments
        for (i, param) in proc.params.iter().enumerate() {
            let value = arg_values.get(i).cloned().unwrap_or(Value::Integer(0));
            local_vars.insert(param.to_uppercase(), value);
        }

        // For functions, initialize return variable
        if proc.is_function {
            local_vars.insert(name_upper.clone(), Value::Integer(0));
        }

        // Push local scope
        self.call_stack.push(local_vars);

        // Execute the procedure body
        for stmt in &proc.body {
            match self.execute_stmt(stmt, &[])? {
                Some(_) => {} // Ignore jumps in procedures
                None => {}
            }

            // Check for RETURN or EXIT SUB
            if matches!(stmt, Stmt::Return(_)) {
                break;
            }
        }

        // Pop local scope and get return value
        let local_vars = self.call_stack.pop().unwrap_or_default();

        // For functions, return the function value
        if proc.is_function {
            Ok(local_vars.get(&name_upper).cloned().unwrap_or(Value::Integer(0)))
        } else {
            Ok(Value::Integer(0))
        }
    }

    /// Evaluate an expression
    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Integer(n) => Ok(Value::Integer(*n)),
            Expr::Float(n) => Ok(Value::Float(*n)),
            Expr::String(s) => Ok(Value::String(s.clone())),

            Expr::Variable(name) => {
                let name_upper = name.to_uppercase();
                let name_base = name_upper.trim_end_matches('$');

                // Check for parameter-less builtin functions
                match name_base {
                    "RND" => return Ok(Value::Float(self.rnd())),
                    "TIMER" => {
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let secs = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs_f64() % 86400.0)
                            .unwrap_or(0.0);
                        return Ok(Value::Float(secs));
                    }
                    "INKEY" => {
                        // Non-blocking key check - return pending key or empty string
                        if let Some(key) = self.pending_key.take() {
                            self.needs_input = false;
                            return Ok(Value::String(key));
                        } else {
                            // No key available - yield for display update if enough time passed
                            // Yield every ~16ms (~60fps) to keep display responsive
                            let now = Self::current_time_millis();
                            if now - self.last_yield_time >= 16 {
                                self.last_yield_time = now;
                                self.needs_input = true;
                            }
                            return Ok(Value::String(String::new()));
                        }
                    }
                    _ => {}
                }

                Ok(self.get_variable(name).unwrap_or_else(|| {
                    if name.to_uppercase().ends_with('$') {
                        Value::String(String::new())
                    } else {
                        Value::Integer(0)
                    }
                }))
            }

            Expr::ArrayAccess(name, indices) => {
                let name_upper = name.to_uppercase();

                // Check if this is a user-defined function call
                if self.procedures.contains_key(&name_upper) {
                    return self.call_procedure(name, indices);
                }

                // Otherwise treat as array access
                let idx: Vec<usize> = indices.iter()
                    .map(|e| self.eval_expr(e).map(|v| v.to_int() as usize))
                    .collect::<Result<Vec<_>, _>>()?;

                if let Some(Value::Array(arr, dims)) = self.variables.get(&name_upper) {
                    let mut linear_idx = 0;
                    let mut multiplier = 1;
                    for (i, &dim_idx) in idx.iter().rev().enumerate() {
                        if i < dims.len() {
                            linear_idx += dim_idx * multiplier;
                            multiplier *= dims[dims.len() - 1 - i];
                        }
                    }
                    Ok(arr.get(linear_idx).cloned().unwrap_or(Value::Integer(0)))
                } else {
                    Ok(Value::Integer(0))
                }
            }

            Expr::BinaryOp(left, op, right) => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;

                match op {
                    BinOp::Add => {
                        // String concatenation
                        if matches!(&l, Value::String(_)) || matches!(&r, Value::String(_)) {
                            Ok(Value::String(l.to_string() + &r.to_string()))
                        } else if matches!(&l, Value::Float(_)) || matches!(&r, Value::Float(_)) {
                            Ok(Value::Float(l.to_float() + r.to_float()))
                        } else {
                            Ok(Value::Integer(l.to_int() + r.to_int()))
                        }
                    }
                    BinOp::Sub => Ok(Value::Float(l.to_float() - r.to_float())),
                    BinOp::Mul => Ok(Value::Float(l.to_float() * r.to_float())),
                    BinOp::Div => {
                        let rv = r.to_float();
                        if rv == 0.0 {
                            Err("Division by zero".to_string())
                        } else {
                            Ok(Value::Float(l.to_float() / rv))
                        }
                    }
                    BinOp::IntDiv => {
                        let rv = r.to_int();
                        if rv == 0 {
                            Err("Division by zero".to_string())
                        } else {
                            Ok(Value::Integer(l.to_int() / rv))
                        }
                    }
                    BinOp::Mod => {
                        let rv = r.to_int();
                        if rv == 0 {
                            Err("Division by zero".to_string())
                        } else {
                            Ok(Value::Integer(l.to_int() % rv))
                        }
                    }
                    BinOp::Pow => Ok(Value::Float(l.to_float().powf(r.to_float()))),
                    BinOp::Eq => {
                        let result = if matches!(&l, Value::String(_)) || matches!(&r, Value::String(_)) {
                            l.to_string() == r.to_string()
                        } else {
                            (l.to_float() - r.to_float()).abs() < f64::EPSILON
                        };
                        Ok(Value::Integer(if result { -1 } else { 0 }))
                    }
                    BinOp::Ne => {
                        let result = if matches!(&l, Value::String(_)) || matches!(&r, Value::String(_)) {
                            l.to_string() != r.to_string()
                        } else {
                            (l.to_float() - r.to_float()).abs() >= f64::EPSILON
                        };
                        Ok(Value::Integer(if result { -1 } else { 0 }))
                    }
                    BinOp::Lt => Ok(Value::Integer(if l.to_float() < r.to_float() { -1 } else { 0 })),
                    BinOp::Le => Ok(Value::Integer(if l.to_float() <= r.to_float() { -1 } else { 0 })),
                    BinOp::Gt => Ok(Value::Integer(if l.to_float() > r.to_float() { -1 } else { 0 })),
                    BinOp::Ge => Ok(Value::Integer(if l.to_float() >= r.to_float() { -1 } else { 0 })),
                    BinOp::And => Ok(Value::Integer(l.to_int() & r.to_int())),
                    BinOp::Or => Ok(Value::Integer(l.to_int() | r.to_int())),
                    BinOp::Xor => Ok(Value::Integer(l.to_int() ^ r.to_int())),
                    BinOp::Eqv => Ok(Value::Integer(!(l.to_int() ^ r.to_int()))),
                    BinOp::Imp => Ok(Value::Integer(!l.to_int() | r.to_int())),
                }
            }

            Expr::UnaryOp(op, expr) => {
                let v = self.eval_expr(expr)?;
                match op {
                    UnaryOp::Neg => Ok(Value::Float(-v.to_float())),
                    UnaryOp::Not => Ok(Value::Integer(!v.to_int())),
                }
            }

            Expr::FunctionCall(name, args) => {
                // Check for user-defined function first
                let name_upper = name.to_uppercase();
                if self.procedures.contains_key(&name_upper) {
                    self.call_procedure(name, args)
                } else {
                    // Fall back to built-in functions
                    builtins::call_builtin(self, name, args)
                }
            }

            Expr::Paren(expr) => self.eval_expr(expr),
        }
    }

    /// Get random number (0 to 1)
    pub fn rnd(&mut self) -> f64 {
        // Simple LCG
        self.rng_state = self.rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.rng_state >> 33) as f64 / (1u64 << 31) as f64
    }

}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
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
    fn test_sub_call() {
        let code = r#"
SUB Greet
    PRINT "Hello from sub"
END SUB

CALL Greet
PRINT "Done"
"#;
        let output = run_basic(code).expect("Should run");
        assert!(output.contains("Hello from sub"), "Output: {}", output);
        assert!(output.contains("Done"), "Output: {}", output);
    }

    #[test]
    fn test_sub_with_params() {
        let code = r#"
SUB PrintNum(x)
    PRINT x
END SUB

CALL PrintNum(42)
"#;
        let output = run_basic(code).expect("Should run");
        assert!(output.contains("42"), "Output: {}", output);
    }

    #[test]
    fn test_function_call() {
        let code = r#"
FUNCTION Twice(x)
    Twice = x * 2
END FUNCTION

PRINT Twice(5)
"#;
        let output = run_basic(code).expect("Should run");
        assert!(output.contains("10"), "Output: {}", output);
    }
}
