//! Built-in BASIC functions

use crate::basic::interpreter::{Interpreter, Value};
use crate::basic::parser::Expr;

/// Call a built-in function
pub fn call_builtin(interp: &mut Interpreter, name: &str, args: &[Expr]) -> Result<Value, String> {
    let name_upper = name.to_uppercase();
    let name_base = name_upper.trim_end_matches('$');

    match name_base {
        // Math functions
        "ABS" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            Ok(Value::Float(v.abs()))
        }
        "INT" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            Ok(Value::Integer(v.floor() as i64))
        }
        "FIX" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            Ok(Value::Integer(v.trunc() as i64))
        }
        "SGN" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            let sign = if v > 0.0 { 1 } else if v < 0.0 { -1 } else { 0 };
            Ok(Value::Integer(sign))
        }
        "SQR" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            if v < 0.0 {
                Err("SQR of negative number".to_string())
            } else {
                Ok(Value::Float(v.sqrt()))
            }
        }
        "SIN" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            Ok(Value::Float(v.sin()))
        }
        "COS" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            Ok(Value::Float(v.cos()))
        }
        "TAN" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            Ok(Value::Float(v.tan()))
        }
        "ATN" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            Ok(Value::Float(v.atan()))
        }
        "LOG" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            if v <= 0.0 {
                Err("LOG of non-positive number".to_string())
            } else {
                Ok(Value::Float(v.ln()))
            }
        }
        "EXP" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            Ok(Value::Float(v.exp()))
        }
        "RND" => {
            let r = interp.rnd();
            Ok(Value::Float(r))
        }

        // String functions
        "LEN" => {
            let s = interp.eval_expr(&args[0])?.to_string();
            Ok(Value::Integer(s.len() as i64))
        }
        "LEFT" => {
            let s = interp.eval_expr(&args[0])?.to_string();
            let n = interp.eval_expr(&args[1])?.to_int() as usize;
            let result: String = s.chars().take(n).collect();
            Ok(Value::String(result))
        }
        "RIGHT" => {
            let s = interp.eval_expr(&args[0])?.to_string();
            let n = interp.eval_expr(&args[1])?.to_int() as usize;
            let len = s.chars().count();
            let result: String = s.chars().skip(len.saturating_sub(n)).collect();
            Ok(Value::String(result))
        }
        "MID" => {
            let s = interp.eval_expr(&args[0])?.to_string();
            let start = (interp.eval_expr(&args[1])?.to_int() as usize).saturating_sub(1);
            let len = if args.len() > 2 {
                interp.eval_expr(&args[2])?.to_int() as usize
            } else {
                s.len()
            };
            let result: String = s.chars().skip(start).take(len).collect();
            Ok(Value::String(result))
        }
        "STR" => {
            let v = interp.eval_expr(&args[0])?;
            let s = match v {
                Value::Integer(n) if n >= 0 => format!(" {}", n),
                Value::Float(n) if n >= 0.0 => format!(" {}", n),
                _ => v.to_string(),
            };
            Ok(Value::String(s))
        }
        "VAL" => {
            let s = interp.eval_expr(&args[0])?.to_string();
            let s = s.trim();
            if s.contains('.') || s.contains('E') || s.contains('e') {
                Ok(Value::Float(s.parse().unwrap_or(0.0)))
            } else {
                Ok(Value::Integer(s.parse().unwrap_or(0)))
            }
        }
        "CHR" => {
            let n = interp.eval_expr(&args[0])?.to_int() as u8;
            Ok(Value::String((n as char).to_string()))
        }
        "ASC" => {
            let s = interp.eval_expr(&args[0])?.to_string();
            let c = s.chars().next().unwrap_or('\0');
            Ok(Value::Integer(c as i64))
        }
        "INSTR" => {
            let (start, haystack, needle) = if args.len() == 2 {
                let h = interp.eval_expr(&args[0])?.to_string();
                let n = interp.eval_expr(&args[1])?.to_string();
                (0, h, n)
            } else {
                let s = interp.eval_expr(&args[0])?.to_int() as usize;
                let h = interp.eval_expr(&args[1])?.to_string();
                let n = interp.eval_expr(&args[2])?.to_string();
                (s.saturating_sub(1), h, n)
            };
            let result = haystack[start..].find(&needle)
                .map(|i| i + start + 1)
                .unwrap_or(0);
            Ok(Value::Integer(result as i64))
        }
        "UCASE" => {
            let s = interp.eval_expr(&args[0])?.to_string();
            Ok(Value::String(s.to_uppercase()))
        }
        "LCASE" => {
            let s = interp.eval_expr(&args[0])?.to_string();
            Ok(Value::String(s.to_lowercase()))
        }
        "LTRIM" => {
            let s = interp.eval_expr(&args[0])?.to_string();
            Ok(Value::String(s.trim_start().to_string()))
        }
        "RTRIM" => {
            let s = interp.eval_expr(&args[0])?.to_string();
            Ok(Value::String(s.trim_end().to_string()))
        }
        "SPACE" => {
            let n = interp.eval_expr(&args[0])?.to_int() as usize;
            Ok(Value::String(" ".repeat(n)))
        }
        "STRING" => {
            let n = interp.eval_expr(&args[0])?.to_int() as usize;
            let c = if args.len() > 1 {
                let v = interp.eval_expr(&args[1])?;
                match v {
                    Value::String(s) => s.chars().next().unwrap_or(' '),
                    Value::Integer(n) => (n as u8) as char,
                    Value::Float(n) => (n as u8) as char,
                    _ => ' ',
                }
            } else {
                ' '
            };
            Ok(Value::String(c.to_string().repeat(n)))
        }

        // Type conversion
        "CINT" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            Ok(Value::Integer(v.round() as i64))
        }
        "CLNG" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            Ok(Value::Integer(v.round() as i64))
        }
        "CSNG" | "CDBL" => {
            let v = interp.eval_expr(&args[0])?.to_float();
            Ok(Value::Float(v))
        }

        // System
        "TIMER" => {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64() % 86400.0;
            Ok(Value::Float(secs))
        }
        "DATE" => {
            let now = chrono_lite_date();
            Ok(Value::String(now))
        }
        "TIME" => {
            let now = chrono_lite_time();
            Ok(Value::String(now))
        }
        "INKEY" => {
            // Would need keyboard input
            Ok(Value::String(String::new()))
        }

        // Screen
        "POS" => {
            let col = interp.graphics.cursor_col;
            Ok(Value::Integer(col as i64))
        }
        "CSRLIN" => {
            let row = interp.graphics.cursor_row;
            Ok(Value::Integer(row as i64))
        }
        "POINT" => {
            let x = interp.eval_expr(&args[0])?.to_int() as i32;
            let y = interp.eval_expr(&args[1])?.to_int() as i32;
            let color = interp.graphics.point(x, y);
            Ok(Value::Integer(color as i64))
        }

        _ => Err(format!("Unknown function: {}", name)),
    }
}

fn chrono_lite_date() -> String {
    // Simple date without chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let days = secs / 86400;
    // Approximate - not fully accurate but works for demo
    let year = 1970 + (days / 365);
    let day_of_year = days % 365;
    let month = day_of_year / 30 + 1;
    let day = day_of_year % 30 + 1;
    format!("{:02}-{:02}-{:04}", month, day, year)
}

fn chrono_lite_time() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let day_secs = secs % 86400;
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
