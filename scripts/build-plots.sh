#!/bin/bash

# This script builds a CSV containing the execution times of all the example programs
# with all the possibile optimizations, one by one, then plots their relative execution times.

set -euo pipefail
shopt -s nullglob

# A few handy variables
RUSTYBF="cargo run --release --quiet --"
ROOT="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )/.."
SCRIPTS_DIR="$ROOT/scripts"
PROGRAMS_DIR="$ROOT/tests/programs"
OUTPUT_DIR="$ROOT/data"
OUTPUT_ABSOLUTE="$OUTPUT_DIR/absolute-times.csv"
OUTPUT_RELATIVE="$OUTPUT_DIR/relative-times.csv"

# Create the output dir if not existing already
mkdir -p "$OUTPUT_DIR"

# All the possible optimizations, including the special values
# `none` (which will be our baseline) and `all`.
OPTIMIZATIONS=("none" $($RUSTYBF list-optimizations) "all")

# Prepare the head of the csv we will create
echo -n "program," > "$OUTPUT_ABSOLUTE"
printf "%s," "${OPTIMIZATIONS[@]}" >> "$OUTPUT_ABSOLUTE"
echo >> "$OUTPUT_ABSOLUTE"

# Sets the format for the output of the `time` builtin
TIMEFORMAT='%3R'

for prog in $PROGRAMS_DIR/*.b; do
    echo "$prog"
    echo -n "$(basename $prog)," >> "$OUTPUT_ABSOLUTE"
    for opt in "${OPTIMIZATIONS[@]}"; do
        
        # Run the program with this optimization measuring time
        COMMAND="$RUSTYBF -O $opt exec $prog"
        echo "  => $opt"
        ({ time $COMMAND < "$prog.in" ; } 2>&1 >/dev/null | tr -d '\n') >> "$OUTPUT_ABSOLUTE"
        echo -n "," >> "$OUTPUT_ABSOLUTE"

    done
    echo >> "$OUTPUT_ABSOLUTE"
done

# Remove all the trailing commas from the file
sed -i -E "s/,$//" "$OUTPUT_ABSOLUTE"

# Remove also the `hello_world.b` program, since it's too small to measure the impact of optimizations
sed -i -E "/hello_world.b,/d" "$OUTPUT_ABSOLUTE"

# Compute another csv file with the relative times
"$SCRIPTS_DIR/relativize-time.py" "$OUTPUT_ABSOLUTE" > "$OUTPUT_RELATIVE"

echo "Generating plots..."

# Start by plotting the graph of absolute times
echo "  => Absolute times"
gnuplot <<EOF
    set title "Execution time with and without optimizations"
    set ylabel "Absolute time [s]"
    set key outside
    set key horizontal center bottom
    set grid
    set term png
    set output "$OUTPUT_DIR/absolute.png"
    set datafile separator ","

    set style data histogram
    set boxwidth 1
    set style fill solid

    plot "$OUTPUT_ABSOLUTE" using 2:xtic(1) ti col,\
         "$OUTPUT_ABSOLUTE" using $(( ${#OPTIMIZATIONS[@]} + 1 )):xtic(1) ti col
EOF

# Plot a graph of relative times for each optimization
for i in $(seq 1 $(( ${#OPTIMIZATIONS[@]} - 1 )));do
    opt=${OPTIMIZATIONS[i]}
    echo "  => Relative times for $opt"
    (
        { cat <<EOF
            set title "Relative execution time for optimization $opt"
            set ylabel "Execution time relative to unoptimized version"
            unset key
            set grid
            set term png
            set output "$OUTPUT_DIR/$opt.png"
            set datafile separator ","

            set style data histogram
            set boxwidth 1
            set style fill solid

            plot '-' using 2:xtic(1) ti col
EOF
        } && cut -d ',' -f 1,$(( $i + 1 )) "$OUTPUT_RELATIVE"
    ) | gnuplot
done