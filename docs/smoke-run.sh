#!/usr/bin/env bash
# Scratch runner for docs/smoke.md. Not a product command.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${DREAM_SMOKE_DIR:-/tmp/dream-smoke}"
LOGDIR="$OUT/logs"
RESULTS="$OUT/results.tsv"
EXPECTED=$'far\norigin\nnear'
TIMEOUT_SECS="${DREAM_SMOKE_TIMEOUT:-300}"

mkdir -p "$LOGDIR"
: >"$RESULTS"

set -a
# shellcheck disable=SC1091
source "$ROOT/.env.local"
set +a
unset DREAM_TRACE

ROWS=(cargo go python node bun deno ruby php dart zig cmake maven gradle dotnet swift elixir haskell nim crystal lua r perl scala ocaml make)

(cd "$ROOT" && cargo build -q)

dream="$ROOT/target/debug/dream"
entry="$ROOT/examples/limits/limits.foo"

classify() {
  local name="$1" code="$2" stdout="$3" stderr="$4"
  local got
  got="$(tr -d '\r' <"$stdout" | sed -e 's/[[:space:]]*$//' | sed -e '/^$/d')"
  if [[ "$code" -eq 124 ]]; then
    echo TIMEOUT
    return
  fi
  if grep -qE 'Install |install hint|does not know how to build' "$stderr"; then
    echo MISSING_HINT
    return
  fi
  if grep -qE 'turn limit reached|ComposerError|composition settled' "$stderr"; then
    echo COMPOSE_FAIL
    return
  fi
  if [[ "$code" -eq 0 && "$got" == "$EXPECTED" ]]; then
    echo PASS
    return
  fi
  if [[ "$code" -eq 0 ]]; then
    echo WRONG_OUTPUT
    return
  fi
  echo EXEC_FAIL
}

host_status() {
  local name="$1"
  case "$name" in
    cargo) command -v cargo >/dev/null && echo present || echo missing ;;
    go) command -v go >/dev/null && echo present || echo missing ;;
    python) { command -v python >/dev/null || command -v python3 >/dev/null; } && echo present || echo missing ;;
    node) command -v node >/dev/null && echo present || echo missing ;;
    bun) command -v bun >/dev/null && echo present || echo missing ;;
    deno) command -v deno >/dev/null && echo present || echo missing ;;
    ruby) command -v ruby >/dev/null && echo present || echo missing ;;
    php) command -v php >/dev/null && echo present || echo missing ;;
    dart) command -v dart >/dev/null && echo present || echo missing ;;
    zig) command -v zig >/dev/null && echo present || echo missing ;;
    cmake) command -v cmake >/dev/null && echo present || echo missing ;;
    maven) command -v mvn >/dev/null && echo present || echo missing ;;
    gradle) command -v gradle >/dev/null && echo present || echo missing ;;
    dotnet) command -v dotnet >/dev/null && echo present || echo missing ;;
    swift) command -v swift >/dev/null && echo present || echo missing ;;
    elixir) command -v elixir >/dev/null && echo present || echo missing ;;
    haskell) command -v cabal >/dev/null && echo present || echo missing ;;
    nim) command -v nim >/dev/null && echo present || echo missing ;;
    crystal) command -v crystal >/dev/null && echo present || echo missing ;;
    lua) command -v lua >/dev/null && echo present || echo missing ;;
    r) command -v Rscript >/dev/null && echo present || echo missing ;;
    perl) command -v perl >/dev/null && echo present || echo missing ;;
    scala) command -v sbt >/dev/null && echo present || echo missing ;;
    ocaml) command -v dune >/dev/null && echo present || echo missing ;;
    make) command -v make >/dev/null && echo present || echo missing ;;
    *) echo unknown ;;
  esac
}

store_toolchain() {
  local dest="$1"
  local store="$dest/.dream/provenance.json"
  if [[ -f "$store" ]]; then
    python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("toolchain",""))' "$store" 2>/dev/null || echo "?"
  else
    echo none
  fi
}

run_one() {
  local name="$1"
  local dest="$OUT/$name"
  local stdout="$LOGDIR/$name.stdout"
  local stderr="$LOGDIR/$name.stderr"
  rm -rf "$dest"
  mkdir -p "$dest"
  echo "=== $name ($(host_status "$name")) ==="
  timeout "$TIMEOUT_SECS" "$dream" --strict --no-warn --fresh \
    "$entry" -t "$name" -o "$dest" --build --run \
    >"$stdout" 2>"$stderr"
  local code=$?
  local result
  result="$(classify "$name" "$code" "$stdout" "$stderr")"
  local toolchain
  toolchain="$(store_toolchain "$dest")"
  local note
  note="$(tr -d '\r' <"$stdout" | sed -e 's/[[:space:]]*$//' | sed -e '/^$/d' | tr '\n' '|' | sed 's/|$//')"
  if [[ -z "$note" ]]; then
    note="$(grep -E 'Error|error:|Install |turn limit' "$stderr" | tail -n 3 | tr '\n' ';' )"
  fi
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "$(host_status "$name")" "$result" "$code" "$toolchain" "$note" | tee -a "$RESULTS"
}

for name in "${ROWS[@]}"; do
  run_one "$name"
done

# compose-only: no catalog exec
name=unsupported
dest="$OUT/$name"
stdout="$LOGDIR/$name.stdout"
stderr="$LOGDIR/$name.stderr"
rm -rf "$dest"
mkdir -p "$dest"
echo "=== unsupported (compose only) ==="
timeout "$TIMEOUT_SECS" "$dream" --strict --fresh \
  "$entry" -t unsupported -o "$dest" \
  >"$stdout" 2>"$stderr"
code=$?
if [[ "$code" -eq 0 ]]; then
  result=COMPOSE_ONLY
elif [[ "$code" -eq 124 ]]; then
  result=TIMEOUT
else
  result=COMPOSE_FAIL
fi
printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$name" "n/a" "$result" "$code" "$(store_toolchain "$dest")" "no --build --run" | tee -a "$RESULTS"

echo "DONE $RESULTS"
