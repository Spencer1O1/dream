use std::io::{self, BufRead, IsTerminal, Read, Write};

use serde_json::{json, Value};

use crate::error::DreamError;

use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(Stdout), Box::new(Stdin)]
}

struct Stdout;

impl Tool for Stdout {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "stdout",
            family: Family::Runtime,
            description: "Write observable program output immediately. Multiple calls are the print stream, in order.",
            parameters: object_params(
                &[("text", string_arg("Exact text (bytes) to write"))],
                &["text"],
            ),
        }
    }

    fn call(&self, _ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let text = arg_str(args, "text");
        let mut out = io::stdout().lock();
        out.write_all(text.as_bytes())?;
        out.flush()?;
        Ok(json!({ "ok": true }).to_string())
    }
}

struct Stdin;

impl Tool for Stdin {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "stdin",
            family: Family::Runtime,
            description: "Read from real stdin. Blocks until input is available. EOF is fine in non-interactive use.",
            parameters: object_params(&[], &[]),
        }
    }

    fn call(&self, _ctx: &mut ToolCtx<'_>, _args: &Value) -> Result<String, DreamError> {
        Ok(json!({ "text": read_stdin()? }).to_string())
    }
}

fn read_stdin() -> Result<String, DreamError> {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        Ok(line)
    } else {
        let mut buf = String::new();
        stdin.lock().read_to_string(&mut buf)?;
        Ok(buf)
    }
}
