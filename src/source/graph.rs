use std::collections::{HashMap, HashSet};

use crate::error::DreamError;

#[derive(Debug)]
pub struct DepGraph {
    current: String,
    edges: HashMap<String, HashSet<String>>,
}

impl DepGraph {
    pub fn new(entry: impl Into<String>) -> Self {
        Self {
            current: entry.into(),
            edges: HashMap::new(),
        }
    }

    pub fn record_read(&mut self, path: &str) -> Result<(), DreamError> {
        if path == self.current {
            return Ok(());
        }
        if reaches(&self.edges, path, &self.current) {
            let mut chain = path_between(&self.edges, path, &self.current).unwrap_or_default();
            chain.push(self.current.clone());
            chain.push(path.to_string());
            return Err(DreamError::new(format!(
                "cycle in source requests: {}",
                chain.join(" -> ")
            )));
        }
        self.edges
            .entry(self.current.clone())
            .or_default()
            .insert(path.to_string());
        self.current = path.to_string();
        Ok(())
    }
}

fn reaches(edges: &HashMap<String, HashSet<String>>, from: &str, to: &str) -> bool {
    path_between(edges, from, to).is_some()
}

fn path_between(
    edges: &HashMap<String, HashSet<String>>,
    from: &str,
    to: &str,
) -> Option<Vec<String>> {
    if from == to {
        return Some(Vec::new());
    }
    let mut stack = vec![(from.to_string(), vec![from.to_string()])];
    let mut seen = HashSet::from([from.to_string()]);
    while let Some((node, trail)) = stack.pop() {
        let Some(nexts) = edges.get(&node) else {
            continue;
        };
        for next in nexts {
            if next == to {
                return Some(trail);
            }
            if seen.insert(next.clone()) {
                let mut next_trail = trail.clone();
                next_trail.push(next.clone());
                stack.push((next.clone(), next_trail));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_request_cycle() {
        let mut graph = DepGraph::new("main.foo");
        graph.record_read("users/a.foo").unwrap();
        graph.record_read("users/b.foo").unwrap();
        let err = graph.record_read("main.foo").unwrap_err();
        assert_eq!(
            err.to_string(),
            "DreamError: cycle in source requests: main.foo -> users/a.foo -> users/b.foo -> main.foo"
        );
    }

    #[test]
    fn allows_reread_of_current_unit() {
        let mut graph = DepGraph::new("main.foo");
        graph.record_read("main.foo").unwrap();
        graph.record_read("users/a.foo").unwrap();
        graph.record_read("users/a.foo").unwrap();
    }
}
