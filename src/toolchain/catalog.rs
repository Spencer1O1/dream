/// How Dream starts a composed project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Run {
    Argv(&'static [&'static str]),
    /// `{program} {entrypoint}` in dest. The path comes from `entry`.
    Stem {
        program: &'static str,
    },
}

/// One toolchain Dream will exec. `unsupported` is not a row.
///
/// The row owns exec, wipe, docs, setup names, and the entrypoint. The composer writes setup files.
#[derive(Debug, Clone, Copy)]
pub struct ToolchainSpec {
    pub name: &'static str,
    /// Empty means no configure step.
    pub configure: &'static [&'static str],
    /// Empty means no build step.
    pub build: &'static [&'static str],
    pub run: Run,
    /// Official program names, in order. First is the argv name. Rest are exec fallbacks.
    pub programs: &'static [&'static str],
    pub install_hint: &'static str,
    /// Official docs. Shown on `set_toolchain` as `docs`. Not a fetch.
    pub docs: &'static str,
    /// Dest paths the composer may write without a `.foo`. Project-owned. `--fresh` drops them.
    pub setup: &'static [&'static str],
    /// Lockfiles and build dirs. Project-owned. Composer must not write. `--fresh` drops them.
    pub project: &'static [&'static str],
    /// Dest-relative source the composer should write. `{stem}` is the entry `.foo` stem.
    pub entry: &'static str,
}

impl ToolchainSpec {
    #[allow(clippy::too_many_arguments)]
    const fn row(
        name: &'static str,
        configure: &'static [&'static str],
        build: &'static [&'static str],
        run: Run,
        programs: &'static [&'static str],
        install_hint: &'static str,
        docs: &'static str,
        setup: &'static [&'static str],
        project: &'static [&'static str],
        entry: &'static str,
    ) -> Self {
        Self {
            name,
            configure,
            build,
            run,
            programs,
            install_hint,
            docs,
            setup,
            project,
            entry,
        }
    }

    pub fn run_argv(&self, entry_stem: &str) -> Vec<String> {
        match self.run {
            Run::Argv(argv) => argv.iter().map(|part| (*part).to_string()).collect(),
            Run::Stem { program } => vec![program.into(), self.owned_entry(entry_stem)],
        }
    }

    pub fn owned_entry(&self, entry_stem: &str) -> String {
        self.entry.replace("{stem}", entry_stem)
    }

    pub fn is_setup(&self, rel: &str) -> bool {
        self.setup.contains(&rel)
    }

    pub fn is_wipe(&self, rel: &str) -> bool {
        self.project.contains(&rel)
    }

    /// Dest paths this row owns: setup files, then lockfiles and build dirs.
    pub fn owned_dest(&self) -> Vec<String> {
        let mut paths = Vec::new();
        paths.extend(self.setup.iter().map(|path| (*path).to_string()));
        paths.extend(self.project.iter().map(|path| (*path).to_string()));
        paths
    }

    #[cfg(test)]
    pub(crate) fn test_row(build: &'static [&'static str], run: Run) -> Self {
        Self::row(
            "test",
            &[],
            build,
            run,
            &[],
            "Install the test toolchain from somewhere.",
            "",
            &[],
            &[],
            "",
        )
    }
}

pub static CATALOG: &[ToolchainSpec] = &[
    ToolchainSpec::row(
        "cargo",
        &[],
        &["cargo", "build"],
        Run::Argv(&["cargo", "run"]),
        &["cargo"],
        "Install Rust from https://rustup.rs/",
        "https://doc.rust-lang.org/cargo/",
        &["Cargo.toml"],
        &["Cargo.lock", "target"],
        "src/main.rs",
    ),
    ToolchainSpec::row(
        "go",
        &[],
        &["go", "build", "-o", "target/"],
        Run::Argv(&["go", "run", "."]),
        &["go"],
        "Install Go from https://go.dev/dl/",
        "https://go.dev/doc/",
        &["go.mod"],
        &["go.sum", "target"],
        "{stem}.go",
    ),
    ToolchainSpec::row(
        "python",
        &[],
        &[],
        Run::Stem { program: "python" },
        &["python", "python3", "py"],
        "Install Python 3 from https://www.python.org/downloads/",
        "https://docs.python.org/3/",
        &["pyproject.toml"],
        &["__pycache__"],
        "{stem}.py",
    ),
    ToolchainSpec::row(
        "node",
        &["npm", "install"],
        &[],
        Run::Stem { program: "node" },
        &["node"],
        "Install Node.js from https://nodejs.org/",
        "https://nodejs.org/docs/latest/api/",
        &["package.json"],
        &["package-lock.json", "node_modules"],
        "{stem}.js",
    ),
    ToolchainSpec::row(
        "bun",
        &["bun", "install"],
        &[],
        Run::Stem { program: "bun" },
        &["bun"],
        "Install Bun from https://bun.sh/",
        "https://bun.sh/docs",
        &["package.json"],
        &["bun.lock", "bun.lockb", "node_modules"],
        "{stem}.js",
    ),
    ToolchainSpec::row(
        "deno",
        &[],
        &[],
        Run::Stem { program: "deno" },
        &["deno"],
        "Install Deno from https://deno.land/",
        "https://docs.deno.com/",
        &["deno.json"],
        &["deno.lock", "node_modules"],
        "{stem}.ts",
    ),
    ToolchainSpec::row(
        "ruby",
        &["bundle", "install"],
        &[],
        Run::Stem { program: "ruby" },
        &["ruby"],
        "Install Ruby from https://www.ruby-lang.org/en/downloads/",
        "https://www.ruby-lang.org/en/documentation/",
        &["Gemfile"],
        &["Gemfile.lock", "vendor"],
        "{stem}.rb",
    ),
    ToolchainSpec::row(
        "php",
        &["composer", "install"],
        &[],
        Run::Stem { program: "php" },
        &["php"],
        "Install PHP from https://www.php.net/downloads",
        "https://www.php.net/docs.php",
        &["composer.json"],
        &["composer.lock", "vendor"],
        "{stem}.php",
    ),
    ToolchainSpec::row(
        "dart",
        &["dart", "pub", "get"],
        &[],
        Run::Argv(&["dart", "run"]),
        &["dart"],
        "Install Dart from https://dart.dev/get-dart",
        "https://dart.dev/guides",
        &["pubspec.yaml"],
        &["pubspec.lock", ".dart_tool"],
        "bin/{stem}.dart",
    ),
    ToolchainSpec::row(
        "zig",
        &[],
        &["zig", "build"],
        Run::Argv(&["zig", "build", "run"]),
        &["zig"],
        "Install Zig from https://ziglang.org/download/",
        "https://ziglang.org/documentation/master/",
        &["build.zig", "build.zig.zon"],
        &["zig-out", ".zig-cache"],
        "{stem}.zig",
    ),
    ToolchainSpec::row(
        "cmake",
        &["cmake", "-S", ".", "-B", "build"],
        &["cmake", "--build", "build"],
        Run::Argv(&["cmake", "--build", "build", "--target", "run"]),
        &["cmake"],
        "Install CMake from https://cmake.org/download/",
        "https://cmake.org/documentation/",
        &["CMakeLists.txt"],
        &["build"],
        "{stem}.c",
    ),
    ToolchainSpec::row(
        "maven",
        &[],
        &["mvn", "-q", "package"],
        Run::Argv(&["mvn", "-q", "exec:java"]),
        &["mvn"],
        "Install Maven from https://maven.apache.org/download.cgi",
        "https://maven.apache.org/guides/",
        &["pom.xml"],
        &["target"],
        "src/main/java/App.java",
    ),
    ToolchainSpec::row(
        "gradle",
        &[],
        &["gradle", "-q", "build"],
        Run::Argv(&["gradle", "-q", "run"]),
        &["gradle"],
        "Install Gradle from https://gradle.org/install/",
        "https://docs.gradle.org/",
        &["build.gradle.kts"],
        &[".gradle", "build"],
        "src/main/java/App.java",
    ),
    ToolchainSpec::row(
        "dotnet",
        &[],
        &["dotnet", "build"],
        Run::Argv(&["dotnet", "run"]),
        &["dotnet"],
        "Install .NET from https://dot.net/",
        "https://learn.microsoft.com/dotnet/",
        &["App.csproj"],
        &["obj", "bin"],
        "{stem}.cs",
    ),
    ToolchainSpec::row(
        "swift",
        &[],
        &["swift", "build"],
        Run::Argv(&["swift", "run"]),
        &["swift"],
        "Install Swift from https://www.swift.org/install/",
        "https://www.swift.org/documentation/",
        &["Package.swift"],
        &[".build"],
        "Sources/App/main.swift",
    ),
    ToolchainSpec::row(
        "elixir",
        &["mix", "deps.get"],
        &["mix", "escript.build"],
        Run::Argv(&["target/app"]),
        &["mix"],
        "Install Elixir from https://elixir-lang.org/install.html",
        "https://hexdocs.pm/elixir/",
        &["mix.exs"],
        &["_build", "deps", "mix.lock", "target"],
        "lib/app.ex",
    ),
    ToolchainSpec::row(
        "haskell",
        &[],
        &["cabal", "build"],
        Run::Argv(&["cabal", "run"]),
        &["cabal"],
        "Install Cabal from https://www.haskell.org/cabal/",
        "https://www.haskell.org/cabal/users-guide/",
        &["app.cabal"],
        &["dist-newstyle", "cabal.project.local"],
        "Main.hs",
    ),
    ToolchainSpec::row(
        "nim",
        &[],
        &["nimble", "build"],
        Run::Argv(&["nimble", "run"]),
        &["nim", "nimble"],
        "Install Nim from https://nim-lang.org/install.html",
        "https://nim-lang.org/docs/overview.html",
        &["app.nimble"],
        &["nimbledeps", "target"],
        "{stem}.nim",
    ),
    ToolchainSpec::row(
        "crystal",
        &["shards", "install"],
        &["shards", "build"],
        Run::Argv(&["bin/app"]),
        &["crystal", "shards"],
        "Install Crystal from https://crystal-lang.org/install/",
        "https://crystal-lang.org/reference/latest/",
        &["shard.yml"],
        &["lib", "shard.lock", "bin"],
        "{stem}.cr",
    ),
    ToolchainSpec::row(
        "lua",
        &[],
        &[],
        Run::Stem { program: "lua" },
        &["lua", "lua5.4", "lua5.3"],
        "Install Lua from https://www.lua.org/download.html",
        "https://www.lua.org/manual/5.4/",
        &[],
        &[],
        "{stem}.lua",
    ),
    ToolchainSpec::row(
        "r",
        &[],
        &[],
        Run::Stem { program: "Rscript" },
        &["Rscript"],
        "Install R from https://cran.r-project.org/",
        "https://cran.r-project.org/manuals.html",
        &["DESCRIPTION"],
        &[],
        "{stem}.R",
    ),
    ToolchainSpec::row(
        "perl",
        &[],
        &[],
        Run::Stem { program: "perl" },
        &["perl"],
        "Install Perl from https://www.perl.org/get.html",
        "https://perldoc.perl.org/",
        &["cpanfile"],
        &["local", "cpanfile.snapshot"],
        "{stem}.pl",
    ),
    ToolchainSpec::row(
        "scala",
        &[],
        &["sbt", "-batch", "compile"],
        Run::Argv(&["sbt", "-batch", "run"]),
        &["sbt"],
        "Install sbt from https://www.scala-sbt.org/download.html",
        "https://docs.scala-lang.org/",
        &["build.sbt"],
        &["target", "project/target"],
        "src/main/scala/App.scala",
    ),
    ToolchainSpec::row(
        "ocaml",
        &[],
        &["dune", "build"],
        Run::Argv(&["dune", "exec", "./app.exe"]),
        &["dune"],
        "Install Dune from https://dune.build/install",
        "https://dune.readthedocs.io/",
        &["dune-project", "dune"],
        &["_build"],
        "app.ml",
    ),
    ToolchainSpec::row(
        "make",
        &[],
        &["make"],
        Run::Argv(&["make", "run"]),
        &["make"],
        "Install Make from your system package manager.",
        "https://www.gnu.org/software/make/manual/make.html",
        &["Makefile"],
        &["target"],
        "{stem}.c",
    ),
];

pub fn spec(name: &str) -> Option<&'static ToolchainSpec> {
    CATALOG.iter().find(|spec| spec.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn names_are_unique_and_nonempty() {
        let mut seen = HashSet::new();
        for spec in CATALOG {
            assert!(!spec.name.is_empty());
            assert!(
                seen.insert(spec.name),
                "duplicate toolchain `{}`",
                spec.name
            );
            assert!(!spec.install_hint.is_empty());
            assert!(!spec.docs.is_empty());
            assert!(!spec.entry.is_empty(), "`{}` has no entrypoint", spec.name);
            assert!(
                !spec.owned_entry("hey-you").is_empty(),
                "`{}` owned_entry is empty",
                spec.name
            );
            for path in spec.setup.iter().chain(spec.project) {
                assert!(!path.is_empty(), "empty dest path on `{}`", spec.name);
            }
            for path in spec.setup {
                assert!(
                    !spec.project.contains(path),
                    "`{}` lists `{path}` as setup and wipe",
                    spec.name
                );
            }
        }
        assert!(CATALOG.len() >= 20, "catalog has {} rows", CATALOG.len());
        assert_eq!(
            spec("python").unwrap().run_argv("hey-you"),
            vec!["python".to_string(), "hey-you.py".to_string()]
        );
        assert_eq!(spec("cargo").unwrap().setup, &["Cargo.toml"]);
        assert_eq!(spec("cargo").unwrap().project, &["Cargo.lock", "target"]);
        assert_eq!(
            spec("python").unwrap().programs,
            &["python", "python3", "py"]
        );
        assert_eq!(
            spec("go").unwrap().owned_dest(),
            ["go.mod", "go.sum", "target"]
        );
        assert_eq!(spec("go").unwrap().build, &["go", "build", "-o", "target/"]);
        assert_eq!(
            spec("cargo").unwrap().owned_dest(),
            ["Cargo.toml", "Cargo.lock", "target"]
        );
        assert_eq!(
            spec("node").unwrap().run_argv("hey-you"),
            vec!["node".to_string(), "hey-you.js".to_string()]
        );
        assert_eq!(spec("node").unwrap().configure, &["npm", "install"]);
        assert_eq!(spec("nim").unwrap().run, Run::Argv(&["nimble", "run"]));
        assert_eq!(spec("nim").unwrap().project, &["nimbledeps", "target"]);
        assert_eq!(spec("crystal").unwrap().run, Run::Argv(&["bin/app"]));
        assert_eq!(
            spec("crystal").unwrap().project,
            &["lib", "shard.lock", "bin"]
        );
        assert_eq!(spec("elixir").unwrap().build, &["mix", "escript.build"]);
        assert_eq!(spec("elixir").unwrap().run, Run::Argv(&["target/app"]));
        assert_eq!(spec("cargo").unwrap().owned_entry("hey-you"), "src/main.rs");
        assert_eq!(
            spec("maven").unwrap().owned_entry("hey-you"),
            "src/main/java/App.java"
        );
        assert_eq!(spec("go").unwrap().owned_entry("hey-you"), "hey-you.go");
        assert_eq!(spec("cmake").unwrap().owned_entry("hey-you"), "hey-you.c");
        assert_eq!(spec("zig").unwrap().owned_entry("hey-you"), "hey-you.zig");
        assert_eq!(spec("make").unwrap().owned_entry("hey-you"), "hey-you.c");
        assert_eq!(spec("dotnet").unwrap().owned_entry("hey-you"), "hey-you.cs");
        assert_eq!(
            spec("dart").unwrap().owned_entry("hey-you"),
            "bin/hey-you.dart"
        );
        assert_eq!(spec("nim").unwrap().owned_entry("hey-you"), "hey-you.nim");
        assert_eq!(
            spec("crystal").unwrap().owned_entry("hey-you"),
            "hey-you.cr"
        );
        assert_eq!(spec("ocaml").unwrap().owned_entry("hey-you"), "app.ml");
        assert_eq!(
            spec("scala").unwrap().owned_entry("hey-you"),
            "src/main/scala/App.scala"
        );
        assert_eq!(spec("python").unwrap().owned_entry("hey-you"), "hey-you.py");
        assert!(spec("lua").unwrap().setup.is_empty());
        assert_eq!(
            spec("zig").unwrap().owned_dest(),
            ["build.zig", "build.zig.zon", "zig-out", ".zig-cache"]
        );
        assert_eq!(
            spec("ocaml").unwrap().owned_dest(),
            ["dune-project", "dune", "_build"]
        );
        assert_eq!(spec("make").unwrap().owned_dest(), ["Makefile", "target"]);
        assert!(spec("cargo").unwrap().is_setup("Cargo.toml"));
        assert!(spec("cargo").unwrap().is_wipe("target"));
        assert!(!spec("cargo").unwrap().is_setup("target"));
    }
}
