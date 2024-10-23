# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "matplotlib",
#     "pandas",
#     "seaborn",
# ]
# ///
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
from pathlib import Path
import argparse

def process_benchmark_data(file_paths):
    """
    Read and process benchmark CSV files.
    Returns a DataFrame with average runtimes grouped by binary and benchmark.
    """
    # Read and combine all CSV files
    dfs = []
    for file_path in file_paths:
        df = pd.read_csv(file_path)
        dfs.append(df)
    
    combined_df = pd.concat(dfs, ignore_index=True)
    
    # Calculate average runtime for each binary-benchmark combination
    avg_runtime = combined_df.groupby(['Binary', 'Benchmark File'])['Runtime (s)'].mean().reset_index()
    
    return avg_runtime

def create_bar_chart(data, output_path=None):
    """
    Create a grouped bar chart from the processed benchmark data.
    """
    # Set the style
    plt.style.use('seaborn-v0_8')
    sns.set_palette("husl")
    
    # Create figure and axis with larger size
    plt.figure(figsize=(12, 6))
    
    # Create the grouped bar chart
    benchmarks = data['Benchmark File'].unique()
    bar_width = 0.8 / len(data['Binary'].unique())
    
    for i, binary in enumerate(data['Binary'].unique()):
        binary_data = data[data['Binary'] == binary]
        positions = range(len(benchmarks))
        plt.bar([p + i * bar_width for p in positions],
                binary_data['Runtime (s)'],
                bar_width,
                label=Path(binary).name)  # Use only the binary name, not full path
    
    # Customize the chart
    plt.xlabel('Benchmark')
    plt.ylabel('Average Runtime (seconds)')
    plt.title('Benchmark Performance Comparison')
    plt.xticks(range(len(benchmarks)), 
               [Path(b).stem for b in benchmarks],  # Remove .lox extension
               rotation=45)
    plt.legend()
    
    # Adjust layout to prevent label cutoff
    plt.tight_layout()
    
    # Save or show the plot
    if output_path:
        plt.savefig(output_path)
        print(f"Chart saved to {output_path}")
    else:
        plt.show()

def main():
    parser = argparse.ArgumentParser(description='Generate benchmark visualization from CSV files.')
    parser.add_argument('files', nargs='+', help='CSV files containing benchmark data')
    parser.add_argument('--output', '-o', help='Output file path for the chart (optional)')
    
    args = parser.parse_args()
    
    # Process data and create visualization
    data = process_benchmark_data(args.files)
    create_bar_chart(data, args.output)

if __name__ == "__main__":
    main()