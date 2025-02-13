# EirGrid Power System Simulator

This directory contains the core implementation of the EirGrid Power System Simulator, a sophisticated simulation tool for modeling Ireland's power grid from 2025 to 2050. The simulator helps analyze and optimize power generation strategies while considering environmental impact, public opinion, and economic factors.

## Core Components

### Power Generation
- `generator.rs`: Defines various types of power generators and their characteristics
- `generators_loader.rs`: Handles loading existing generator data from CSV files
- `power_storage.rs`: Implements power storage systems (batteries, pumped storage)

### Geographic and Infrastructure
- `map_handler.rs`: Manages the spatial aspects of the power grid
- `settlement.rs`: Represents population centers and their power demands
- `settlements_loader.rs`: Loads settlement data from JSON files
- `poi.rs`: Handles Points of Interest on the map
- `coastline_points.json`: Defines Ireland's coastline for geographic constraints

### Environmental Impact
- `carbon_offset.rs`: Manages carbon offset mechanisms and calculations
- `action_weights.rs`: Evaluates and scores different grid actions based on environmental impact

### Configuration and Constants
- `constants.rs`: Defines system-wide constants and parameters
- `const_funcs.rs`: Contains utility functions for calculations
- `simulation_config.rs`: Handles simulation configuration settings

### Main Simulation
- `main.rs`: Orchestrates the simulation, including initialization and execution

## Generator Types

The simulator supports various types of power generators:

### Renewable Sources
- Wind: Onshore and Offshore
- Solar: Domestic, Commercial, and Utility-scale
- Hydro: Dams, Pumped Storage
- Marine: Tidal and Wave Energy
- Biomass

### Conventional Sources
- Nuclear
- Coal Plants
- Gas: Combined Cycle and Peaker Plants

### Storage Systems
- Pumped Hydro Storage
- Battery Storage Systems

## Key Features

1. **Temporal Simulation**: Models power grid evolution from 2025 to 2050
2. **Geographic Constraints**: Considers location-specific factors for generator placement
3. **Economic Modeling**: Includes capital costs, operating costs, and inflation
4. **Public Opinion**: Simulates public acceptance of different generator types
5. **Environmental Impact**: Tracks CO2 emissions and carbon offsets
6. **Power Storage**: Models energy storage systems for grid stability
7. **Technical Evolution**: Accounts for improving technology efficiencies over time

## Data Sources

- `ireland_generators.csv`: Existing power generators in Ireland
- `settlements.json`: Population centers and their characteristics
- `coastline_points.json`: Geographic data for Ireland's coastline

## Usage

The simulator is configured to run multiple iterations to find optimal strategies for power grid development. It considers:

- Power demand growth
- Population changes
- Technology improvements
- Environmental targets
- Economic constraints
- Public opinion factors

Results are saved as CSV files with timestamps for analysis and comparison.

## Dependencies

The project uses several Rust crates:
- `serde`: For data serialization/deserialization
- `rand`: For randomization in simulations
- `chrono`: For timestamp handling
- `rayon`: For parallel processing
- `anyhow`: For error handling
- `clap`: For command-line argument parsing

## Command-Line Arguments

The simulator supports several command-line arguments for customizing its execution:

```bash
# Basic usage
cargo run [OPTIONS]

OPTIONS:
    -i, --iterations <NUMBER>         Number of simulation iterations to run [default: 1000]
    -p, --parallel                   Run simulations in parallel [default: true]
    -n, --no-continue               Start fresh instead of continuing from checkpoint
    -c, --checkpoint-dir <DIR>       Directory for saving checkpoints [default: "checkpoints"]
    -k, --checkpoint-interval <NUM>  How often to save checkpoints (iterations) [default: 5]
    -r, --progress-interval <NUM>    How often to print progress (seconds) [default: 10]
```

### Examples:

```bash
# Run with default settings (1000 iterations, parallel, continue from checkpoint if exists)
cargo run

# Start a fresh simulation with 2000 iterations
cargo run -- --iterations 2000 --no-continue

# Run sequentially (non-parallel) with custom checkpoint directory
cargo run -- --parallel false --checkpoint-dir my_checkpoints

# Save checkpoints more frequently (every 10 iterations)
cargo run -- --checkpoint-interval 10

# Update progress more frequently (every 5 seconds)
cargo run -- --progress-interval 5
```

The checkpoint system allows the simulation to be resumed if interrupted, making it more resilient to crashes or system shutdowns. Each checkpoint saves:
- Current iteration number
- Latest weights and learning progress
- Best results so far

When continuing from a checkpoint, the simulation will automatically load the latest state and continue from where it left off, unless `--no-continue` is specified.

## File Saving and Checkpoints

The simulator saves files in several locations:

### Checkpoint Files (During Simulation)
Location: `<checkpoint-dir>/<YYYYMMDD_HHMMSS>/` (default: `./checkpoints/YYYYMMDD_HHMMSS/`)
Frequency: Every `checkpoint-interval` iterations (default: 5)
Files saved in each timestamped directory:
- `latest_weights.json`: Current state of the action weights
- `checkpoint_iteration.txt`: Current iteration number
- `best_simulation.csv`: Best simulation results for this run
- `best_weights.json`: Final optimized weights for this run

The timestamp format (YYYYMMDD_HHMMSS) ensures unique directories for each run and makes it easy to track when each simulation was executed. When continuing from a checkpoint, the simulator automatically finds and loads the most recent checkpoint directory.

Example directory structure:
```
checkpoints/
├── 20240315_093000/
│   ├── latest_weights.json
│   ├── checkpoint_iteration.txt
│   ├── best_simulation.csv
│   └── best_weights.json
├── 20240315_100530/
│   ├── latest_weights.json
│   ├── checkpoint_iteration.txt
│   ├── best_simulation.csv
│   └── best_weights.json
└── 20240315_110845/
    ├── latest_weights.json
    ├── checkpoint_iteration.txt
    ├── best_simulation.csv
    └── best_weights.json
```

## Output

The simulation produces detailed metrics including:
- Total population served
- Power generation and usage
- Grid balance
- Public opinion scores
- Operating and capital costs
- CO2 emissions and offsets
- Generator efficiencies
- Storage utilization 