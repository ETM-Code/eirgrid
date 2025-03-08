# Map Data for GridAI

This directory contains the geographical and infrastructure data used by the GridAI simulation system to model Ireland's power grid and settlements.

## Directory Structure

- **sourceData/**: Data scraped and processed from publicly available sources.


## Data Sources

The map data consists of two primary categories:

### 1. Map Data (Geographic and Infrastructure)

Data representing Ireland's current infrastructure, sourced from:
- **Global Power Plant Database**: Information on existing power generators
- **Census 2022 F1011**: Population data for settlements
- **Google Maps API**: Geographic coordinates for settlements

### 2. Calculation Data (Forecasting and Estimation)

Data used to forecast future changes and estimate key values:
- Economic projections and cost trends
- Technology efficiency improvements
- Public opinion factors
- Geographic and grid constraints

## Data Processing Flow

1. Raw data from public sources is collected in `sourceData/`
2. Processing scripts in the `mapScraper` directory clean and format this data
3. Processed data is stored in `genData/` for simulation use
4. The best scenarios from simulation runs are stored in `top10/`

## Data Schema (Note, some data is stored in the 'mapScraper' directory)

### Power Generators
```
capacity_mw,latitude,longitude,primary_fuel
92.0,53.338,-6.4875,Wind
25.0,52.474,-6.566,Wind
15.0,51.878,-8.401,Gas
...
```

### Settlements
```
{
  "name": "Dublin",
  "population": 1173179,
  "latitude": 53.3498,
  "longitude": -6.2603,
  "county": "Dublin"
}
```

## Usage Notes

- The simulation uses this data to create a digital representation of Ireland
- Generator locations are used to calculate power generation capacity
- Settlement data informs power demand and public opinion modeling
- Geographic data enables spatial optimization of new generator placement

## Updating the Data

To update or refresh the data:

1. Place new source files in the `sourceData/` directory
2. Run the appropriate processing scripts from the `mapScraper` directory
3. Verify the processed output in the `genData/` directory
4. Re-run simulations with the updated data

## Data Maintenance

Data should be periodically refreshed to account for:
- Changes in the power grid infrastructure
- Population shifts in settlements
- Updates to the Global Power Plant Database
- Revised economic or technology projections

## Format Requirements

When adding custom data, ensure it follows the established format conventions to maintain compatibility with the simulation system. 

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