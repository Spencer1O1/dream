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

    /// Target language (compose mode). Open-ended string.
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

    /// Drop Dream-owned output and compose again. Leaves unmanaged files.
    #[arg(long)]
    pub fresh: bool,

    /// Entry .foo file.
    #[arg(required = true)]
    pub rest: Vec<String>,
}
