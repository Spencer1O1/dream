use super::Tool;

/// Composer-only tools. Empty until `write_output_file` lands.
pub fn tools() -> Vec<Box<dyn Tool>> {
    Vec::new()
}
