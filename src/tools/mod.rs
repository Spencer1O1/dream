//! Tool catalog.
//!
//! Each tool owns its name, family, description, parameters, and behavior.
//! The description says what the tool does. Parameter descriptions say what
//! to write. `Registry::instructions` staples a preamble to the tool list.
//!
//! No Dream law, no goals, no dest in agent-facing English.

mod args;
mod catalog;
mod composer;
mod control;
mod ctx;
mod files;
mod foo;
mod http;
mod list_foo;
mod read_foo;
mod read_setup;
mod read_source;
mod registry;
mod remove_setup;
mod remove_source;
pub(crate) mod reply;
mod runtime;
mod tool;
mod toolchain;
mod write_setup;
mod write_source;

pub use ctx::{Compose, Mode, ToolCtx};
pub use registry::Registry;
pub use tool::{Family, Tool, ToolSpec};

pub(crate) use args::{
    arg_str, enum_arg, nullable_string_arg, object_array_arg, object_params, string_arg,
};
