# Map Scraper for GridAI

This directory contains Python scripts for collecting, processing, and transforming geographical and infrastructure data for use in the GridAI simulation system.

## Overview

The Map Scraper is responsible for creating the digital environment in which the AI operates by:

1. Extracting Irish power generator data from global datasets
2. Retrieving and processing settlement data from census reports
3. Obtaining geographic coordinates for settlements via Google Maps API
4. Transforming raw data into simulation-ready formats

## Key Components

### Data Collection Scripts

- **`filter_generators_in_ireland.py`**: Extracts Irish power plant data from the Global Power Plant Database
- **`settlements.py`**: Processes settlement data from CSO census reports and obtains geographic coordinates
- **`ireland_bounds.py`**: Defines geographic boundaries and coordinate transformations for Ireland

### Data Analysis Scripts

- **`analyze_missing_settlements.py`**: Identifies and analyzes settlements missing from the dataset
- **`transform_settlements.py`**: Transforms settlement data into formats suitable for simulation

### Data Files

- **`global_power_plant_database.csv`**: The source database of global power plants
- **`ireland_generators.csv`**: Filtered dataset containing only Irish generators
- **`Population.csv`**: Census population data for Irish settlements
- **`unmatched_settlements.json`**: Settlements that couldn't be automatically matched

## Prerequisites

- Python 3.8 or higher
- Dependencies listed in `requirements.txt`:
  ```
  requests
  shapely
  numpy
  python-dotenv
  ```
- Google Maps API key stored in `.env` file

## Installation

1. Create a virtual environment (recommended):
   ```bash
   python -m venv venv
   source venv/bin/activate  # On Windows: venv\Scripts\activate
   ```

2. Install dependencies:
   ```bash
   pip install -r requirements.txt
   ```

3. Configure Google Maps API:
   Create a `.env` file with your API key:
   ```
   GOOGLE_MAPS_API_KEY=your_api_key_here
   ```

## Usage

### Generating Generator Data

```bash
# Filter Irish generators from the global database
python filter_generators_in_ireland.py
```

This will create `ireland_generators.csv` containing only Irish power plants with relevant fields.

### Processing Settlement Data

```bash
# Process settlement data with position lookup
python settlements.py

# Analyze any missing settlements
python analyze_missing_settlements.py

# Transform settlement data for simulation
python transform_settlements.py
```

### Data Processing Flow

1. Start with the raw Global Power Plant Database and CSO census data
2. Run `filter_generators_in_ireland.py` to extract Irish generators
3. Run `settlements.py` to process settlement data and obtain coordinates
4. If needed, run `analyze_missing_settlements.py` to handle missing data
5. Run `transform_settlements.py` to create simulation-ready formats
6. Generated files are moved to the `mapData/genData/` directory for use in simulation

## Checkpointing

The scripts include checkpointing functionality that allows for resuming interrupted processing:

- Checkpoints are stored in the `checkpoints/` directory
- Use the `--append` flag with `settlements.py` to resume from a checkpoint:
  ```bash
  python settlements.py --append
  ```

## Output Files

- **`ireland_generators.csv`**: Clean dataset of Irish power generators
- **`settlements.json`**: Processed settlement data with coordinates
- **`missing_settlements_analysis.json`**: Analysis of unmatched settlements

## Data Processing Details

### Settlement Data Compression

The original 20,000+ settlements from the CSO are compressed to approximately 130 representative settlements by:

1. Grouping settlements by county
2. Preserving population distribution
3. Creating weighted centroids for larger areas

### Coordinate Transformation

The scripts transform between different coordinate systems:

- Geographic coordinates (latitude/longitude)
- Simulation grid coordinates
- UTM coordinates for distance calculations

## Troubleshooting

- If API requests fail, check your Google Maps API key and quota limits
- For "unmatched settlements," try modifying the search terms in `clean_search_name()` function
- If processing is interrupted, use the `--append` flag to resume from the last checkpoint 

## License

### GridAI Research and Academic License

This software is licensed under the following terms:

**Permitted Uses:**
- Individual use for personal experimentation and learning
- Academic and research use in non-commercial settings
- Educational use in classroom or course settings

**Requirements:**
- Attribution: Any use of this software must include proper citation and acknowledgment to the original authors: Eoghan Collins
- Notification: We appreciate being informed about research conducted using this software at [eoghancollins@gmail.com]

**Prohibited Uses:**
- Commercial use of any kind without explicit written permission
- Redistribution, in whole or in part, on any public repository or platform
- Modification and redistribution as a derivative work
- Use in production environments or for commercial policy decision-making
- Any use that does not include proper attribution

**No Warranty:** This software is provided "as is" without warranty of any kind, express or implied.

For permissions beyond the scope of this license, please contact [eoghancollins@gmail.com]. 