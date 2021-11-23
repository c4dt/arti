#!/bin/bash

set -e

SCRIPT_NAME=$(basename "$0")

function usage()
{
    cat <<EOF
${SCRIPT_NAME}: Generate coverage using grcov.

Usage:
  with_coverage [opts] <command> [args...] : Run <command> with [args].
  with_coverage -i [opts]                  : Run bash interactively.

Options:
  -h: Print this message.
  -i: Run an interactive shell after the command (if any)

Notes:
  You need to have grcov, rust-nightly, and llvm-tools-preview installed.
EOF
}

interactive=no

while getopts "hi" opt ; do
    case "$opt" in
	h) usage
	   exit 0
	   ;;
	i) interactive=yes
	   ;;
	*) echo "Unknown option."
	   exit 1
	   ;;
    esac
done

# Remove the flags we parsed.
shift $((OPTIND-1))

# Make sure that we'll be doing _something_.
if [ $# -eq 0 ] && [ $interactive = "no" ]; then
    echo "No command specified: Use the -i flag if you want a shell."
    echo
    echo "Run ${SCRIPT_NAME} -h for help."
    exit 1
fi

# Validate that +nightly is installed.  This will log a message to stderr
# if it isn't.
cargo +nightly -h >/dev/null

# Validate that grcov is installed.
if [ "$(which grcov 2>/dev/null)" = "" ]; then
    echo "grcov appears not to be installed.  Try 'cargo install grcov'." >&2
    exit 1
fi

COVERAGE_BASEDIR=$(git rev-parse --show-toplevel)
export RUSTFLAGS="-Z instrument-coverage"
export LLVM_PROFILE_FILE=$COVERAGE_BASEDIR/coverage_meta/%p-%m.profraw
export RUSTUP_TOOLCHAIN=nightly

if [ -d "$COVERAGE_BASEDIR/coverage" ]; then
    rm -r "$COVERAGE_BASEDIR/coverage" || true
fi
if [ -d "$COVERAGE_BASEDIR/coverage_meta" ]; then
    rm -r "$COVERAGE_BASEDIR/coverage_meta" || true
fi

mkdir -p "$COVERAGE_BASEDIR/coverage"

if [ $# -ne 0 ]; then
    "$@"
fi

if [ $interactive = "yes" ] ; then
    echo "Launching a bash shell."
    echo "Exit this shell when you are ready to genate a coverage report."
    # when run interactivelly, don't die on error
    bash || true
fi

echo "Generating report..."

grcov "$COVERAGE_BASEDIR/coverage_meta" --binary-path "$COVERAGE_BASEDIR/target/debug/" \
	-s "$COVERAGE_BASEDIR/crates/" -o "$COVERAGE_BASEDIR/coverage" -t html --branch \
	--ignore-not-existing --excl-start '^mod test' --excl-stop '^}' \
	--ignore="*/tests/*" --ignore="*/examples/*"

# Extract coverage information and print it to the command line.
awk '{if (match($0, /<p class="heading">([^<]*)<\/p>/, groups)) {
		last_match=groups[1]
	} else if (match($0, /<abbr title="[0-9]* \/ [0-9]*">([^<]*)<\/abbr>/, groups)) {
	    print last_match " " groups[1]
	}}' "$COVERAGE_BASEDIR/coverage/index.html"

echo "Full report: $COVERAGE_BASEDIR/coverage/index.html"
