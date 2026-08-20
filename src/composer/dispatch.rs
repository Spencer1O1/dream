use std::path::Path;

use crate::error::DreamError;
use crate::llm::FunctionCall;
use crate::source::{DepGraph, Project};
use crate::tools::reply;
use crate::tools::{Registry, ToolCtx, WriteSlot};

use super::progress;
use super::provenance::Store;

pub(crate) struct ToolIo<'a> {
    pub dest: Option<&'a Path>,
    pub store: Option<&'a Store>,
    pub write: Option<WriteSlot<'a>>,
    pub builder: Option<&'a mut Option<crate::builder::Builder>>,
    pub toolchain: Option<crate::builder::Builder>,
}

pub(crate) fn dispatch(
    registry: &Registry,
    project: &Project,
    deps: &mut DepGraph,
    io: ToolIo<'_>,
    call: &FunctionCall,
) -> Result<String, DreamError> {
    let args = call.parsed_args()?;
    let mut ctx = ToolCtx {
        project,
        deps,
        dest: io.dest,
        store: io.store,
        write: io.write,
        builder: io.builder,
        toolchain: io.toolchain,
    };
    let result = registry.dispatch(&mut ctx, call);
    match &result {
        Ok(output) => match reply::warning_of(output) {
            Some(message) => progress::warning(&call.name, &args, &message),
            None => progress::tool(&call.name, &args),
        },
        Err(err) => progress::rejected(&call.name, &args, err),
    }
    result
}
