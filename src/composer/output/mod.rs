mod dest;
mod files;
mod replace;

pub use dest::resolve_output_dir;
pub use files::{remove_file, require_files, write_file};
pub use replace::replace_output;
