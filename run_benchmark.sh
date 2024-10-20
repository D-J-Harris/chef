#!/bin/bash

# Array of binaries to test
BINARIES=("./target/release/chef")

# Array of benchmark files
BENCHMARK_FILES=("equality.lox")

# Number of runs per test
NUM_RUNS=5

# Results file
RESULTS_FILE="benchmark_results.csv"

# Create/clear results file with header
echo "Binary,Benchmark File,Run Number,Runtime (s)" > "$RESULTS_FILE"

# Function to extract the final line (runtime) from output
get_runtime() {
    echo "$1" | tail -n 1
}

# Run benchmarks
for binary in "${BINARIES[@]}"; do
    # Check if binary exists and is executable
    if [ ! -x "./$binary" ]; then
        echo "Error: $binary not found or not executable"
        continue
    fi
    
    echo "Testing $binary..."
    
    for benchmark in "${BENCHMARK_FILES[@]}"; do
        # Check if benchmark file exists
        if [ ! -f "./tests/benchmark/$benchmark" ]; then
            echo "Error: Benchmark file $benchmark not found"
            continue
        fi
        
        echo "  Running with $benchmark..."
        
        for ((run=1; run<=NUM_RUNS; run++)); do
            echo "    Run $run of $NUM_RUNS..."
            output=$(./"$binary" "./tests/benchmark/$benchmark")
            runtime=$(get_runtime "$output")
            echo "$binary,$benchmark,$run,$runtime" >> "$RESULTS_FILE"
        done
    done
done

echo -e "\Results saved in $RESULTS_FILE"