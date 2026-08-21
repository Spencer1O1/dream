use crate::error::DreamError;
use crate::llm::FunctionCall;
use crate::tools::reply;
use crate::tools::{Registry, ToolCtx};

use super::progress;

pub(crate) fn dispatch(
    registry: &Registry,
    ctx: &mut ToolCtx<'_>,
    call: &FunctionCall,
) -> Result<String, DreamError> {
    let args = call.parsed_args()?;
    let result = registry.dispatch(ctx, call);
    match &result {
        Ok(output) => match reply::warning_of(output) {
            Some(message) => progress::warning(&call.name, &args, &message),
            None => progress::tool(&call.name, &args),
        },
        Err(_) => progress::tool(&call.name, &args),
    }
    result
}
