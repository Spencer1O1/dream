use std::collections::HashSet;

/// Units this session has reached: the entry, plus every `.foo` that was read.
/// Session-only. Not a request stack. Re-reading the entry is fine.
#[derive(Debug)]
pub struct DepGraph {
    entry: String,
    read: HashSet<String>,
}

impl DepGraph {
    pub fn new(entry: impl Into<String>) -> Self {
        let entry = entry.into();
        Self {
            read: HashSet::from([entry.clone()]),
            entry,
        }
    }

    pub fn entry(&self) -> &str {
        &self.entry
    }

    pub fn reached(&self, unit: &str) -> bool {
        self.read.contains(unit)
    }

    pub fn record_read(&mut self, path: &str) {
        self.read.insert(path.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reread_of_the_entry_after_other_units_is_ok() {
        let mut graph = DepGraph::new("main.foo");
        assert_eq!(graph.entry(), "main.foo");
        graph.record_read("utils.foo");
        graph.record_read("main.foo");
        assert!(graph.reached("main.foo"));
        assert!(graph.reached("utils.foo"));
    }

    #[test]
    fn allows_reread_of_a_unit() {
        let mut graph = DepGraph::new("main.foo");
        graph.record_read("utils.foo");
        graph.record_read("utils.foo");
        assert!(graph.reached("utils.foo"));
    }

    #[test]
    fn reached_is_entry_or_a_unit_that_was_read() {
        let mut graph = DepGraph::new("main.foo");
        assert!(graph.reached("main.foo"));
        assert!(!graph.reached("utils.foo"));
        graph.record_read("utils.foo");
        assert!(graph.reached("utils.foo"));
        assert!(graph.reached("main.foo"));
    }
}
