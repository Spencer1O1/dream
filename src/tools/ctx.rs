use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::provenance::Store;
use crate::source::{DepGraph, Project};
use crate::toolchain::Toolchain;

pub struct Pick<'a> {
    pub toolchain: &'a mut Option<Toolchain>,
}

pub struct Compose<'a> {
    pub dest: &'a Path,
    pub store: &'a Store,
    pub artifacts: &'a mut HashMap<String, HashSet<String>>,
    pub toolchain: Option<Toolchain>,
}

pub struct Repair<'a> {
    pub dest: &'a Path,
    pub store: &'a Store,
    pub toolchain: Option<Toolchain>,
}

pub enum Mode<'a> {
    Lucid,
    Pick(Pick<'a>),
    Compose(Compose<'a>),
    Repair(Repair<'a>),
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

    pub fn pick(
        project: &'a Project,
        deps: &'a mut DepGraph,
        toolchain: &'a mut Option<Toolchain>,
    ) -> Self {
        Self {
            project,
            deps,
            mode: Mode::Pick(Pick { toolchain }),
        }
    }

    pub fn compose(project: &'a Project, deps: &'a mut DepGraph, compose: Compose<'a>) -> Self {
        Self {
            project,
            deps,
            mode: Mode::Compose(compose),
        }
    }

    pub fn repair(
        project: &'a Project,
        deps: &'a mut DepGraph,
        dest: &'a Path,
        store: &'a Store,
        toolchain: Option<Toolchain>,
    ) -> Self {
        Self {
            project,
            deps,
            mode: Mode::Repair(Repair {
                dest,
                store,
                toolchain,
            }),
        }
    }
}
