use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::provenance::Store;
use crate::source::{DepGraph, Project};
use crate::toolchain::Toolchain;

pub struct Resolve<'a> {
    pub toolchain: &'a mut Option<Toolchain>,
}

pub struct Compose<'a> {
    pub dest: &'a Path,
    pub store: &'a Store,
    pub artifacts: &'a mut HashMap<String, HashSet<String>>,
    pub toolchain: Option<Toolchain>,
}

pub enum Mode<'a> {
    Lucid,
    Resolve(Resolve<'a>),
    Compose(Compose<'a>),
}

pub struct ToolCtx<'a> {
    pub project: &'a Project,
    pub deps: &'a mut DepGraph,
    pub mode: Mode<'a>,
}

impl<'a> ToolCtx<'a> {
    pub fn lucid(project: &'a Project, deps: &'a mut DepGraph) -> Self {
        Self {
            project,
            deps,
            mode: Mode::Lucid,
        }
    }

    pub fn resolve(
        project: &'a Project,
        deps: &'a mut DepGraph,
        toolchain: &'a mut Option<Toolchain>,
    ) -> Self {
        Self {
            project,
            deps,
            mode: Mode::Resolve(Resolve { toolchain }),
        }
    }

    pub fn compose(project: &'a Project, deps: &'a mut DepGraph, compose: Compose<'a>) -> Self {
        Self {
            project,
            deps,
            mode: Mode::Compose(compose),
        }
    }
}
