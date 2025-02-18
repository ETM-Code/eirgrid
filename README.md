# EirGrid Power System Simulator

A sophisticated simulation tool for modeling Ireland's power grid evolution from 2025 to 2050. The simulator helps analyze and optimize power generation strategies while considering environmental impact, public opinion, and economic factors.

## Features

### Core Capabilities
- **Temporal Simulation**: Models power grid evolution from 2025 to 2050
- **Geographic Optimization**: Uses Metal-accelerated location analysis for optimal generator placement
- **Economic Modeling**: Includes capital costs, operating costs, and inflation factors
- **Public Opinion**: Simulates public acceptance of different generator types
- **Environmental Impact**: Tracks CO2 emissions and carbon offsets
- **Power Storage**: Models energy storage systems for grid stability
- **Technical Evolution**: Accounts for improving technology efficiencies over time

### Performance Features
- **Parallel Processing**: Multi-threaded simulation support
- **Metal Acceleration**: GPU-accelerated location analysis (Apple Silicon)
- **Checkpointing**: Robust save/resume capability
- **Performance Monitoring**: Detailed timing instrumentation
- **Fast Simulation Mode**: Optimized simulation for rapid iteration

## Generator Types

### Renewable Sources
- Wind: Onshore and Offshore
- Solar: Domestic, Commercial, and Utility-scale
- Hydro: Dams and Pumped Storage
- Marine: Tidal and Wave Energy
- Biomass

### Conventional Sources
- Nuclear
- Coal Plants
- Gas: Combined Cycle and Peaker Plants

### Storage Systems
- Pumped Hydro Storage
- Battery Storage Systems

## Project Structure

### Core Components
```
src/
├── main.rs                 # Main simulation orchestration
├── map_handler.rs          # Geographic and spatial management
├── generator.rs            # Power generator implementations
├── settlement.rs           # Population center modeling
├── carbon_offset.rs        # Environmental impact tracking
├── power_storage.rs        # Energy storage systems
├── metal_location_search.rs # GPU-accelerated location optimization
├── logging.rs              # Performance monitoring and timing
└── action_weights.rs       # Strategy optimization
```

### Data Files
- `ireland_generators.csv`: Existing power generator data
- `settlements.json`: Population centers and demographics
- `coastline_points.json`: Geographic constraints

## Usage

### Basic Command
```bash
cargo run -- [OPTIONS]
```

### Common Options
```bash
-n, --iterations <NUM>      Number of iterations [default: 1000]
--parallel                  Enable parallel processing [default: true]
--enable-timing            Enable performance monitoring
--checkpoint-interval <NUM> Save frequency [default: 5]
--progress-interval <NUM>   Progress update frequency [default: 10]
--force-full-simulation    Disable fast simulation mode
```

### Examples
```bash
# Run with timing enabled
cargo run -- -n 10 --enable-timing

# Fast parallel simulation with 1000 iterations
cargo run -- -n 1000 --parallel

# Full simulation mode with frequent checkpoints
cargo run -- --force-full-simulation --checkpoint-interval 2
```

## Performance Monitoring

The simulator includes comprehensive timing instrumentation across various operation categories:

### Operation Categories
- **Simulation**: Core simulation operations
- **Power Calculation**: Generation, usage, and balance computations
- **Location Search**: Generator placement optimization
- **Weights Update**: Strategy optimization
- **File I/O**: Data loading and checkpointing

### Timing Report Example
```
Detailed Performance Report
==========================

Hierarchical Timing Analysis:
---------------------------
run_simulation: total=10.5s, count=100, avg=105.0ms
  ├─ calc_power_generation: total=2.1s, avg=21.0ms
  ├─ handle_power_deficit: total=1.8s, avg=18.0ms
  └─ update_weights: total=0.8s, avg=8.0ms

Performance by Category:
------------------------
Power Calculation: 45.2% of total time
  mean=21.5ms, p95=35.2ms, p99=42.1ms
Location Search: 30.1% of total time
  mean=150.2ms, p95=280.5ms, p99=320.1ms
Simulation: 15.5% of total time
  mean=105.0ms, p95=180.2ms, p99=220.5ms
```

## Output Files

### Checkpoint Directory Structure
```
checkpoints/YYYYMMDD_HHMMSS/
├── latest_weights.json     # Current optimization state
├── checkpoint_iteration.txt # Progress tracking
├── best_simulation.csv     # Best results metrics
├── best_simulation_actions.csv # Detailed action history
└── best_weights.json       # Optimized strategy weights
```

### Metrics Tracked
- Population served and power demand
- Generation capacity and grid balance
- Public opinion scores
- Operating and capital costs
- CO2 emissions and offsets
- Generator efficiencies
- Storage utilization
- Action history with spatial data

## Requirements

- Rust 1.70 or later
- macOS 11.0 or later (for Metal acceleration)
- 8GB RAM minimum (16GB recommended)
- Apple Silicon Mac recommended for optimal performance

## License

This project is proprietary and confidential. All rights reserved. 