use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "dream",
    about = "Dream is executable pseudocode.",
    disable_help_subcommand = true
)]
pub(super) struct Raw {
    /// Stricter prompt. Not a parser.
    #[arg(long)]
    pub strict: bool,

    /// Interpret immediately instead of composing.
    #[arg(long)]
    pub lucid: bool,

    /// Toolchain hint. A catalog name, or any string.
    #[arg(short = 't', long = "target")]
    pub target: Option<String>,

    /// Output directory (compose mode).
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    /// Compose, then build if Dream knows a toolchain.
    #[arg(long)]
    pub build: bool,

    /// Compose, build, and run. Implies --build.
    #[arg(long)]
    pub run: bool,

    /// Treat toolchain warnings as a failed build.
    #[arg(long = "no-warn")]
    pub no_warn: bool,

    /// Drop Dream-owned output files and compose again. Leaves unmanaged files.
    #[arg(long)]
    pub fresh: bool,

    /// Entry .foo, directory, or `lock`/`unlock`/`inspect` plus a path.
    #[arg(required = true)]
    pub rest: Vec<String>,
}
