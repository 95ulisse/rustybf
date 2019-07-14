#!/bin/bash

# This script builds a CSV containing the execution times of all the example programs
# with all the possibile optimizations, one by one.

set -euo pipefail
shopt -s nullglob

# A few handy variables
RUSTYBF="cargo run --release --quiet --"
ROOT="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )/.."
PROGRAMS_DIR="$ROOT/tests/programs"
OUTPUT="$ROOT/scripts/interpreter.csv"

# All the possible optimizations, including the special values
# `none` (which will be our baseline) and `all`.
OPTIMIZATIONS=("none" $($RUSTYBF --list-optimizations) "all")

# Prepare the head of the csv we will create
echo -n "program," > "$OUTPUT"
printf "%s," "${OPTIMIZATIONS[@]}" >> "$OUTPUT"
echo >> "$OUTPUT"

# Sets the format for the output of the `time` builtin
TIMEFORMAT='%3R'

for prog in $PROGRAMS_DIR/*.b; do
    echo "$prog"
    echo -n "$(basename $prog)," >> "$OUTPUT"
    for opt in "${OPTIMIZATIONS[@]}"; do
        
        # Run the program with this optimization measuring time
        COMMAND="$RUSTYBF -e -O $opt $prog"
        echo "  => $opt"
        ({ time $COMMAND < "$prog.in" ; } 2>&1 >/dev/null | tr -d '\n') >> "$OUTPUT"
        echo -n "," >> "$OUTPUT"

    done
    echo >> "$OUTPUT"
done

# Remove all the trailing commas from the file
sed -i -E "s/,$//" "$OUTPUT"