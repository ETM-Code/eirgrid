# GridAI: Designing the Future of the Irish Power Grid

**Using Policy Gradient Reinforcement Learning to Design the Future of the Irish Power Grid**

Authors: Eoghan Collins

![GridAI Logo](./media/Grid%20AI.png)

## Overview

GridAI is a sophisticated AI-driven simulation system that explores and identifies optimal energy strategies for Ireland from 2025 to 2050. Using custom-built Policy Gradient Reinforcement Learning, the system simulates over 100,000 iterations to determine the most effective, cost-efficient energy strategy while achieving 100% power reliability and driving Ireland into net-negative emissions by 2047.

This repository contains the complete codebase for the GridAI project, including data processing, AI simulation, and visualization components.

## Project Structure

The project is organized into four main components:

### 1. AI Simulator (`aiSimulator/`)
A custom Policy Gradient Reinforcement Learning model written in Rust that simulates various energy investment strategies and identifies optimal solutions, paired with a large, advanced simulation of the Irish power grid.

Features:
- Multi-objective optimization balancing power reliability, CO₂ emissions, cost, and public opinion
- Temporal adaptation with year-dependent action weights
- Contrast learning and experience replay
- Metal-accelerated spatial analysis
- Checkpointing for resumable simulations

### 2. Map Data (`mapData/`)
Storage for various data files used by the simulation:
- Source data from public datasets
- Generated data processed for simulation use
- Top 10 optimized scenarios

### 3. Map Scraper (`mapScraper/`)
Python scripts for collecting and preprocessing geographical and infrastructure data:
- Power plant database processing
- Settlement location via Google Maps API
- Census data processing

### 4. Renderer (`renderer/`)
Web-based visualization tools for exploring simulation results:
- Interactive map showing settlements, generators, and carbon offsets
- Timeline for navigating through simulation years
- Metrics panel and trend charts
- Data export capabilities

## Key Findings

The simulation reveals several key insights for Ireland's energy future, based off the simulation:

1. **Early Investment**: Solar and wind energy should be scaled early, with significant investment in solar by 2025 and wind by 2026.

2. **Phased Adoption**: More advanced technologies like offshore wind and battery storage become optimal later (2040s) as their costs decrease and efficiency improves.

3. **Carbon Offsetting**: Strategic implementation of carbon offsets beginning in 2034 and scaling aggressively by 2047 is cost-effective.

4. **Technology Evolution**: Wave energy becomes viable by 2050, suggesting it could be a key component post-2050.

5. **Net Negative**: The optimal strategy achieves net-negative emissions by 2047 while maintaining 100% power reliability.

## Tabulated Results

### Actions
| Year | Action Type       | Generator Type   | Count | Estimated Cost (€) |
|------|------------------|-----------------|-------|---------------------|
| 2025 | Add Generator   | Utility Solar   | x12   | 12000000.0         |
| 2026 | Add Generator   | Onshore Wind    | x8    | 7920000.0          |
| 2027 | Add Generator   | Commercial Solar | x1    | 94090.0            |
| 2029 | Add Generator   | Onshore Wind    | x2    | 1921192.0          |
| 2030 | Add Generator   | Commercial Solar | x5    | 429367.0           |
| 2030 | Add Generator   | Onshore Wind    | x3    | 2852970.2          |
| 2033 | Add Generator   | Utility Solar   | x1    | 783743.4           |
| 2033 | Add Generator   | Onshore Wind    | x2    | 1845489.4          |
| 2034 | Add Generator   | Offshore Wind   | x1    | 1827034.5          |
| 2034 | Add Carbon Offset | Forest       | x1    | 1000000.0          |
| 2035 | Add Generator   | Commercial Solar | x4    | 294969.6           |
| 2035 | Add Generator   | Onshore Wind    | x2    | 1808764.2          |
| 2036 | Add Generator   | Utility Solar   | x1    | 715301.4           |
| 2039 | Add Generator   | Offshore Wind   | x8    | 13799933.0         |
| 2039 | Add Generator   | Onshore Wind    | x1    | 868745.8           |
| 2040 | Add Generator   | Battery Storage | x1    | 500000000.0        |
| 2042 | Add Generator   | Domestic Solar  | x2    | 11916.5            |
| 2042 | Add Generator   | Offshore Wind   | x1    | 1685886.4          |
| 2044 | Add Generator   | Offshore Wind   | x4    | 6609349.0          |
| 2047 | Add Carbon Offset | Forest       | x3    | 3000000.0          |
| 2048 | Add Generator   | Domestic Solar  | x1    | 4963.1             |
| 2049 | Add Generator   | Battery Storage | x3    | 1500000000.0       |
| 2050 | Add Generator   | Wave Energy     | x2    | 2000000000.0       |
| 2050 | Add Generator   | Utility Solar   | x4    | 1867898.8          |


### Outcomes
| Year | Pop.   | Power Usage (MW) | Power Generation (MW) | Public Opinion | Annual Net Cost (€ adj.) | Net Emissions (Kilotonne CO₂/day) |
|------|--------|-----------------|------------------|----------------|----------------------|----------------------|
| 2025 | 5149136 | 5252.12         | 7390.91          | 0.7732         | 2.3913E+10          | 110.7               |
| 2026 | 5200628 | 5516.83         | 8776.91          | 0.7849         | 314427745           | 150.3               |
| 2027 | 5252636 | 5792.73         | 8786.81          | 0.7823         | 562847534           | 150.8               |
| 2028 | 5305160 | 6080.27         | 8786.81          | 0.7776         | 831433687           | 150.8               |
| 2029 | 5358215 | 6379.89         | 9133.31          | 0.7767         | 965236515           | 160.7               |
| 2030 | 5411800 | 6692.07         | 9702.56          | 0.7869         | 1019685284          | 178.02              |
| 2031 | 5465919 | 7017.28         | 9702.56          | 0.7824         | 1342987905          | 178.02              |
| 2032 | 5520574 | 7356.03         | 9702.56          | 0.7778         | 1687290395          | 178.02              |
| 2033 | 5575778 | 7708.84         | 10108.46         | 0.7784         | 1876397699          | 190.89              |
| 2034 | 5631527 | 8076.24         | 10385.66         | 0.8208         | 2146201944          | -159.14             |
| 2035 | 5687845 | 8458.81         | 10771.76         | 0.8263         | 2394470311          | -170.6              |
| 2036 | 5744726 | 8857.13         | 10831.16         | 0.8255         | 2814030410          | -188.74             |
| 2037 | 5802180 | 9271.79         | 10831.16         | 0.8236         | 3290058012          | -207.85             |
| 2038 | 5860199 | 9703.41         | 10831.16         | 0.8216         | 3798955716          | -225.14             |
| 2039 | 5918800 | 10152.65        | 13499.21         | 0.8303         | 3174950932          | -164.55             |
| 2040 | 5977982 | 10620.16        | 13994.21         | 0.8267         | 3552944520          | -173.76             |
| 2041 | 6037760 | 11106.66        | 13994.21         | 0.8247         | 4177614415          | -186.57             |
| 2042 | 6098141 | 11612.86        | 14275.37         | 0.826          | 4724226252          | -190.04             |
| 2043 | 6159121 | 12139.5         | 14275.37         | 0.824          | 5442838451          | -200.52             |
| 2044 | 6220709 | 12687.36        | 15384.17         | 0.826          | 5728909389          | -178.33             |
| 2045 | 6282917 | 13257.24        | 15384.17         | 0.8241         | 6558170880          | -186.92             |
| 2046 | 6345748 | 13849.97        | 15384.17         | 0.8221         | 7450056502          | -194.68             |
| 2047 | 6409208 | 14466.42        | 15384.17         | 0.82           | 8410160185          | -2241.35            |
| 2048 | 6473298 | 15107.45        | 15386.15         | 0.8191         | 9443492509          | -2271.79            |
| 2049 | 6538030 | 15774.02        | 16871.15         | 0.8129         | 9950408776          | -2284.58            |
| 2050 | 6603409 | 16467.06        | 17306.75         | 0.8066         | 1.103E+10           | -2295.73            |


## Getting Started

### Prerequisites
- **AI Simulator**: Rust 1.70+
- **Map Scraper**: Python 3.8+ with required packages (see `mapScraper/requirements.txt`)
- **Renderer**: Modern web browser supporting JavaScript ES6

### Running the Simulation

The simulation supports numerous command-line arguments to customize its behavior. Here's a comprehensive guide:

#### Basic Usage

```bash
# Clone the repository
git clone https://github.com/ETM-Code/eirgrid.git
cd eirgrid

# Run a simulation with default parameters
cargo run -- -n 1000

# Run with timing enabled and increased iterations
cargo run -- -n 10000 --enable-timing

# Resume from checkpoint with parallel processing
cargo run -- --parallel
```

#### Core Simulation Arguments

| Argument | Short | Description | Default |
|----------|-------|-------------|---------|
| `--iterations <NUM>` | `-n` | Number of simulation iterations to run | 1000 |
| `--parallel` | `-p` | Enable parallel processing for faster simulation | true |
| `--no-continue` | | Do not continue from previous checkpoint | false |
| `--force-full-simulation` | | Disable fast simulation mode for higher fidelity | false |
| `--seed <NUM>` | | Random seed for deterministic simulation | random |

#### Checkpoint and Logging Options

| Argument | Short | Description | Default |
|----------|-------|-------------|---------|
| `--checkpoint-dir <DIR>` | `-c` | Directory for storing/loading checkpoints | "checkpoints" |
| `--checkpoint-interval <NUM>` | `-i` | Save checkpoint every N iterations | 5 |
| `--progress-interval <NUM>` | `-r` | Display progress every N iterations | 10 |
| `--cache-dir <DIR>` | `-C` | Directory for caching computation results | "cache" |
| `--enable-timing` | | Enable detailed performance timing | false |
| `--verbose-state-logging` | `-v` | Enable detailed state logging | true |

#### Simulation Behavior Modifiers

| Argument | Short | Description | Default |
|----------|-------|-------------|---------|
| `--cost-only` | | Optimize for cost only, ignoring emissions and public opinion | false |
| `--enable-energy-sales` | | Include revenue from energy sales to offset costs | false |

#### Example Commands for Common Scenarios

**Fast Initial Exploration**
```bash
# Run many quick iterations to explore the solution space
cargo run -- -n 5000 --force-full-simulation=false
```

**Detailed Analysis Run**
```bash
# Run fewer iterations but with full simulation fidelity
cargo run -- -n 100 --force-full-simulation --verbose-state-logging
```

**Deterministic Run (for debugging or reproducibility)**
```bash
# Use a specific random seed for reproducible results
cargo run -- -n 1000 --seed 12345
```

**Frequent Checkpointing (for unstable environments)**
```bash
# Save progress more frequently
cargo run -- -n 1000 --checkpoint-interval 1 --progress-interval 1
```

**Economic Analysis**
```bash
# Focus on cost optimization only
cargo run -- -n 1000 --cost-only
```

**Revenue Modeling**
```bash
# Include energy sales revenue in calculations
cargo run -- -n 1000 --enable-energy-sales
```

**Performance Tuning**
```bash
# Run with detailed timing information
cargo run -- -n 100 --enable-timing --force-full-simulation
```

**Resume Long-Running Simulation**
```bash
# Continue from last checkpoint (default behavior)
cargo run -- -n 10000

# Force start from scratch
cargo run -- -n 10000 --no-continue
```

#### Analyzing Results

After running a simulation, results are saved in the checkpoint directory. The most important files are:

- `best_simulation.csv`: Summary metrics of the best run
- `best_simulation_actions.csv`: Detailed actions from the best run
- `/yearly_details`: Year-by-year metrics for the entire simulation period (generators, carbon offsets, settlements, etc)

For visualization, these files can be loaded into the renderer component.

### Visualization

To visualize simulation results:
1. Navigate to the `renderer` directory
2. Place all csvs exported into the "data/yearly_details" folder"
3. Serve the directory with a local HTTP server or open `index.html` directly
4. Load simulation CSV data via the interface or use sample data

## Resources

- **Paper**: See the attached paper in this repository for detailed methodology and results


## License

### GridAI Research and Academic License

This software is licensed under the following terms:

**Permitted Uses:**
- Individual use for personal experimentation and learning
- Academic and research use in non-commercial settings
- Educational use in classroom or course settings

**Requirements:**
- Attribution: Any use of this software must include proper citation and acknowledgment to the original authors: Eoghan Collins
- Notification: I appreciate being informed about research conducted using this software at [eoghancollins@gmail.com]

**Prohibited Uses:**
- Commercial use of any kind without explicit written permission
- Redistribution, in whole or in part, on any public repository or platform
- Modification and redistribution as a derivative work
- Use in production environments or for commercial policy decision-making
- Any use that does not include proper attribution

**No Warranty:** This software is provided "as is" without warranty of any kind, express or implied.

For permissions beyond the scope of this license, please contact [eoghancollins@gmail.com], I'd love to work with you! 
