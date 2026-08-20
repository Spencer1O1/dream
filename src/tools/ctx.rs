use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::builder::Builder;
use crate::provenance::{Dependency, Store};
use crate::source::{DepGraph, Project};

pub struct Pick<'a> {
    pub builder: &'a mut Option<Builder>,
}

pub struct Compose<'a> {
    pub dest: &'a Path,
    pub store: &'a Store,
    pub artifacts: &'a mut HashMap<String, HashSet<String>>,
    pub dependencies: &'a mut HashMap<String, Vec<Dependency>>,
    pub fresh: bool,
    pub toolchain: Option<Builder>,
}

pub struct Repair<'a> {
    pub dest: &'a Path,
    pub store: &'a Store,
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
        builder: &'a mut Option<Builder>,
    ) -> Self {
        Self {
            project,
            deps,
            mode: Mode::Pick(Pick { builder }),
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
    ) -> Self {
        Self {
            project,
            deps,
            mode: Mode::Repair(Repair { dest, store }),
        }
    }
}
