//! Dest path helpers: package stem, init marks, exec mkdirs.
//! Not the Dream source tree (`source/`). Not exec (`toolchain/`).

mod init;
mod name;

pub use init::{ensure_output_dirs, init};
pub use name::from_entry;
