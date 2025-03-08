/**
 * sample-data.js
 * 
 * This module generates realistic sample data for the Irish power grid simulation
 * when real data is not available, allowing the visualization to function for demos.
 */

// Helper function to log from this module
function log(message, level = 'info') {
  if (window.AppLog) {
    window.AppLog('SampleData', message, level);
  } else {
    console.log(`[SampleData] ${message}`);
  }
}

// Module for generating sample data
window.SampleData = window.SampleData || {};

// Constants for sample data generation
window.SampleData.constants = {
  GENERATOR_TYPES: ['wind', 'solar', 'hydro', 'nuclear', 'coal', 'gas', 'biomass'],
  GENERATOR_EMISSIONS: {
    wind: 0,
    solar: 0,
    hydro: 0,
    nuclear: 0,
    coal: 820,
    gas: 490,
    biomass: 230
  },
  OFFSET_TYPES: ['forest', 'bog', 'carbon_capture'],
  START_YEAR: 2023,
  END_YEAR: 2050,
  NUM_SETTLEMENTS: 25,
  NUM_GENERATORS: 35,
  NUM_OFFSETS: 20
};

/**
 * Generate sample data for the simulation
 * @param {number} startYear - First year to generate data for
 * @param {number} endYear - Last year to generate data for
 * @returns {Object} Sample data objects for the simulation
 */
window.SampleData.generate = function(startYear, endYear) {
  log(`Generating sample data from ${startYear} to ${endYear}`);
  
  const years = {};
  
  // Constants for easier access
  const constants = window.SampleData.constants;
  
  // Generate random settlements
  const settlements = generateSettlements(constants.NUM_SETTLEMENTS);
  
  // Generate random generators
  const generators = generateGenerators(constants.NUM_GENERATORS, constants);
  
  // Generate random carbon offsets
  const carbonOffsets = generateCarbonOffsets(constants.NUM_OFFSETS, constants);
  
  // For each year, generate varying data
  for (let year = startYear; year <= endYear; year++) {
    years[year] = generateYearData(year, settlements, generators, carbonOffsets, constants);
  }
  
  log(`Successfully generated sample data for ${Object.keys(years).length} years`);
  return years;
};

/**
 * Generate settlement data
 * @param {number} count - Number of settlements to generate
 * @returns {Array} Array of settlement objects
 */
function generateSettlements(count) {
  log(`Generating ${count} settlements`);
  
  // Define the boundaries of Ireland (rough approximation)
  const bounds = {
    north: 55.38,
    south: 51.42,
    west: -10.47,
    east: -6.0
  };
  
  const settlements = [];
  for (let i = 0; i < count; i++) {
    const settlement = {
      id: `S${i.toString().padStart(3, '0')}`,
      name: `Settlement ${i + 1}`,
      population: Math.floor(Math.random() * 50000) + 1000,
      energyDemand: Math.floor(Math.random() * 200) + 50, // MWh
      lat: bounds.south + Math.random() * (bounds.north - bounds.south),
      lng: bounds.west + Math.random() * (bounds.east - bounds.west),
    };
    settlements.push(settlement);
  }
  
  return settlements;
}

/**
 * Generate power generator data
 * @param {number} count - Number of generators to generate
 * @param {Object} constants - Constants for data generation
 * @returns {Array} Array of generator objects
 */
function generateGenerators(count, constants) {
  log(`Generating ${count} power generators`);
  
  // Define the boundaries of Ireland (rough approximation)
  const bounds = {
    north: 55.38,
    south: 51.42,
    west: -10.47,
    east: -6.0
  };
  
  const generators = [];
  for (let i = 0; i < count; i++) {
    const type = constants.GENERATOR_TYPES[Math.floor(Math.random() * constants.GENERATOR_TYPES.length)];
    const generator = {
      id: `G${i.toString().padStart(3, '0')}`,
      name: `${type.charAt(0).toUpperCase() + type.slice(1)} Generator ${i + 1}`,
      type: type,
      capacity: Math.floor(Math.random() * 500) + 50, // MW
      output: Math.floor(Math.random() * 500) + 50, // MWh
      emissions: constants.GENERATOR_EMISSIONS[type] * (Math.floor(Math.random() * 500) + 50) / 1000, // Tonnes
      lat: bounds.south + Math.random() * (bounds.north - bounds.south),
      lng: bounds.west + Math.random() * (bounds.east - bounds.west),
    };
    generators.push(generator);
  }
  
  return generators;
}

/**
 * Generate carbon offset data
 * @param {number} count - Number of offsets to generate
 * @param {Object} constants - Constants for data generation
 * @returns {Array} Array of carbon offset objects
 */
function generateCarbonOffsets(count, constants) {
  log(`Generating ${count} carbon offsets`);
  
  // Define the boundaries of Ireland (rough approximation)
  const bounds = {
    north: 55.38,
    south: 51.42,
    west: -10.47,
    east: -6.0
  };
  
  const offsets = [];
  for (let i = 0; i < count; i++) {
    const type = constants.OFFSET_TYPES[Math.floor(Math.random() * constants.OFFSET_TYPES.length)];
    const offset = {
      id: `O${i.toString().padStart(3, '0')}`,
      name: `${type.charAt(0).toUpperCase() + type.slice(1).replace('_', ' ')} Project ${i + 1}`,
      type: type,
      capacity: Math.floor(Math.random() * 1000) + 100, // Tonnes CO2
      sequestration: Math.floor(Math.random() * 1000) + 100, // Tonnes CO2
      lat: bounds.south + Math.random() * (bounds.north - bounds.south),
      lng: bounds.west + Math.random() * (bounds.east - bounds.west),
    };
    offsets.push(offset);
  }
  
  return offsets;
}

/**
 * Generate data for a specific year
 * @param {number} year - The year to generate data for
 * @param {Array} baseSettlements - Base settlement data
 * @param {Array} baseGenerators - Base generator data
 * @param {Array} baseOffsets - Base carbon offset data
 * @param {Object} constants - Constants for data generation
 * @returns {Object} Year data
 */
function generateYearData(year, baseSettlements, baseGenerators, baseOffsets, constants) {
  log(`Generating data for year ${year}`);
  
  // Calculate progress factor (0 at start year, 1 at end year)
  const yearProgress = (year - constants.START_YEAR) / (constants.END_YEAR - constants.START_YEAR);
  
  // Adapt settlements for this year
  const settlements = baseSettlements.map(s => {
    // Population grows over time
    const growthFactor = 1 + (yearProgress * 0.2); // 20% growth by end year
    const population = Math.floor(s.population * growthFactor);
    
    // Energy demand changes with population but also becomes more efficient
    const efficiencyFactor = 1 - (yearProgress * 0.3); // 30% more efficient by end year
    const energyDemand = Math.floor(s.energyDemand * growthFactor * efficiencyFactor);
    
    return {
      ...s,
      population,
      energyDemand
    };
  });
  
  // Adapt generators for this year
  let generators = baseGenerators.map(g => {
    if (!g) return null;
    
    let outputFactor = 1;
    
    // Adjust output based on generator type and year
    if (g.type === 'wind' || g.type === 'solar') {
      outputFactor = 1 + (yearProgress * 1.5); // Wind/solar increase over time
    } else if (g.type === 'coal') {
      outputFactor = 1 - (yearProgress * 0.9); // Almost gone by end year
      if (year > constants.START_YEAR + Math.floor((constants.END_YEAR - constants.START_YEAR) * 0.7)) {
        return null; // Coal plants decommissioned in later years
      }
    } else if (g.type === 'gas') {
      outputFactor = 1 - (yearProgress * 0.5); // Reduced gas over time
    }
    
    // Calculate new output
    const output = Math.floor(g.output * outputFactor);
    
    // Calculate new emissions (improved efficiency over time)
    const efficiencyFactor = 1 - (yearProgress * 0.2); // 20% more efficient by end year
    const emissions = g.emissions * outputFactor * efficiencyFactor;
    
    return {
      ...g,
      output,
      emissions
    };
  }).filter(g => g !== null); // Remove decommissioned generators
  
  // Add new renewable generators over time
  if (year > constants.START_YEAR && year % 5 === 0) {
    const numNewGenerators = Math.floor((year - constants.START_YEAR) / 5);
    log(`Adding ${numNewGenerators} new renewable generators for year ${year}`);
    
    // Define the boundaries of Ireland (rough approximation)
    const bounds = {
      north: 55.38,
      south: 51.42,
      west: -10.47,
      east: -6.0
    };
    
    for (let i = 0; i < numNewGenerators; i++) {
      // Only add renewables as time goes on
      const types = ['wind', 'solar', 'hydro', 'biomass'];
      const type = types[Math.floor(Math.random() * types.length)];
      
      const newGenerator = {
        id: `G${baseGenerators.length + i}_${year}`,
        name: `New ${type.charAt(0).toUpperCase() + type.slice(1)} ${year}`,
        type: type,
        capacity: Math.floor(Math.random() * 800) + 200, // Newer generators have more capacity
        output: Math.floor(Math.random() * 800) + 200,
        emissions: constants.GENERATOR_EMISSIONS[type] * (Math.floor(Math.random() * 800) + 200) / 1000,
        lat: bounds.south + Math.random() * (bounds.north - bounds.south),
        lng: bounds.west + Math.random() * (bounds.east - bounds.west),
      };
      
      generators.push(newGenerator);
    }
  }
  
  // Adapt carbon offsets for this year
  let offsets = baseOffsets.map(o => {
    // Carbon sequestration improves over time
    const sequestrationFactor = 1 + (yearProgress * 0.8); // 80% more effective by end year
    const sequestration = Math.floor(o.sequestration * sequestrationFactor);
    
    return {
      ...o,
      sequestration
    };
  });
  
  // Add new carbon offset projects over time
  if (year > constants.START_YEAR && year % 3 === 0) {
    const numNewOffsets = Math.floor((year - constants.START_YEAR) / 3);
    log(`Adding ${numNewOffsets} new carbon offset projects for year ${year}`);
    
    // Define the boundaries of Ireland (rough approximation)
    const bounds = {
      north: 55.38,
      south: 51.42,
      west: -10.47,
      east: -6.0
    };
    
    for (let i = 0; i < numNewOffsets; i++) {
      const type = constants.OFFSET_TYPES[Math.floor(Math.random() * constants.OFFSET_TYPES.length)];
      const newOffset = {
        id: `O${baseOffsets.length + i}_${year}`,
        name: `New ${type.charAt(0).toUpperCase() + type.slice(1).replace('_', ' ')} ${year}`,
        type: type,
        capacity: Math.floor(Math.random() * 2000) + 500, // Newer projects have more capacity
        sequestration: Math.floor(Math.random() * 2000) + 500,
        lat: bounds.south + Math.random() * (bounds.north - bounds.south),
        lng: bounds.west + Math.random() * (bounds.east - bounds.west),
      };
      
      offsets.push(newOffset);
    }
  }
  
  // Generate summary statistics
  const totalDemand = settlements.reduce((sum, s) => sum + s.energyDemand, 0);
  const totalOutput = generators.reduce((sum, g) => sum + g.output, 0);
  const totalEmissions = generators.reduce((sum, g) => sum + g.emissions, 0);
  const totalSequestration = offsets.reduce((sum, o) => sum + o.sequestration, 0);
  const netEmissions = totalEmissions - totalSequestration;
  
  return {
    year: year,
    settlements: settlements,
    generators: generators,
    offsets: offsets,
    summary: {
      demand: totalDemand,
      output: totalOutput,
      emissions: totalEmissions,
      sequestration: totalSequestration,
      netEmissions: netEmissions,
      renewable: generators.filter(g => ['wind', 'solar', 'hydro'].includes(g.type)).reduce((sum, g) => sum + g.output, 0) / totalOutput
    }
  };
}

/**
 * Generate sample data for the simulation
 * @returns {Object} Sample data objects for the simulation
 */
window.SampleData.generateSampleData = function() {
  log('Starting sample data generation');
  
  // Array to hold all generated data
  let settlements = [];
  let generators = [];
  let offsets = [];
  let summary = [];
  
  log(`Generating data for years ${constants.START_YEAR} to ${constants.END_YEAR}`);
  
  // Define the boundaries of Ireland (rough approximation)
  const bounds = {
    north: 55.4,
    south: 51.4,
    west: -10.5,
    east: -6.0
  };
  
  // Generate settlements
  log(`Generating ${constants.NUM_SETTLEMENTS} settlements`);
  let baseSettlements = [];
  for (let i = 0; i < constants.NUM_SETTLEMENTS; i++) {
    const settlement = {
      id: `S${i.toString().padStart(3, '0')}`,
      name: `Settlement ${i + 1}`,
      population: Math.floor(Math.random() * 100000) + 5000,
      powerUsage: Math.floor(Math.random() * 1000) + 200, // MWh
      lat: bounds.south + Math.random() * (bounds.north - bounds.south),
      lng: bounds.west + Math.random() * (bounds.east - bounds.west),
      gridX: i % 10, // For compatibility with grid coordinate system
      gridY: Math.floor(i / 10)
    };
    baseSettlements.push(settlement);
  }
  log('Base settlements generated');
  
  // Generate generators
  log(`Generating ${constants.NUM_GENERATORS} power generators`);
  let baseGenerators = [];
  for (let i = 0; i < constants.NUM_GENERATORS; i++) {
    const type = constants.GENERATOR_TYPES[Math.floor(Math.random() * constants.GENERATOR_TYPES.length)];
    const generator = {
      id: `G${i.toString().padStart(3, '0')}`,
      name: `${type.charAt(0).toUpperCase() + type.slice(1)} Generator ${i + 1}`,
      type: type,
      capacity: Math.floor(Math.random() * 500) + 50, // MW
      output: Math.floor(Math.random() * 500) + 50, // MWh
      emissions: constants.GENERATOR_EMISSIONS[type] * (Math.floor(Math.random() * 500) + 50) / 1000, // Tonnes
      lat: bounds.south + Math.random() * (bounds.north - bounds.south),
      lng: bounds.west + Math.random() * (bounds.east - bounds.west),
      gridX: i % 10, 
      gridY: Math.floor(i / 10)
    };
    baseGenerators.push(generator);
  }
  log('Base generators created');
  
  // Generate carbon offsets
  log(`Generating ${constants.NUM_OFFSETS} carbon offsets`);
  let baseOffsets = [];
  for (let i = 0; i < constants.NUM_OFFSETS; i++) {
    const type = constants.OFFSET_TYPES[Math.floor(Math.random() * constants.OFFSET_TYPES.length)];
    const offset = {
      id: `O${i.toString().padStart(3, '0')}`,
      name: `${type.charAt(0).toUpperCase() + type.slice(1).replace('_', ' ')} Offset ${i + 1}`,
      type: type,
      offsetAmount: Math.floor(Math.random() * 1000) + 200, // Tonnes
      area: Math.floor(Math.random() * 500) + 50, // Hectares
      lat: bounds.south + Math.random() * (bounds.north - bounds.south),
      lng: bounds.west + Math.random() * (bounds.east - bounds.west),
      gridX: i % 10,
      gridY: Math.floor(i / 10)
    };
    baseOffsets.push(offset);
  }
  log('Base carbon offsets created');
  
  // Generate data for each year
  log('Creating yearly data progression');
  for (let year = constants.START_YEAR; year <= constants.END_YEAR; year++) {
    const yearProgress = (year - constants.START_YEAR) / (constants.END_YEAR - constants.START_YEAR);
    log(`Generating data for year ${year} (progress factor: ${yearProgress.toFixed(2)})`);
    
    // Modify settlements for this year (population growth, changing power usage)
    const yearSettlements = baseSettlements.map(s => {
      const populationGrowthFactor = 1 + (yearProgress * 0.5); // Up to 50% increase by end year
      const powerUsageEfficiencyFactor = 1 - (yearProgress * 0.3); // Up to 30% decrease by end year
      
      return {
        ...s,
        year: year,
        population: Math.floor(s.population * populationGrowthFactor),
        powerUsage: Math.floor(s.powerUsage * populationGrowthFactor * powerUsageEfficiencyFactor)
      };
    });
    settlements = settlements.concat(yearSettlements);
    log(`Generated ${yearSettlements.length} settlements for year ${year}`);
    
    // Modify generators for this year (phasing out fossil fuels, increasing renewables)
    const yearGenerators = baseGenerators.map(g => {
      let outputFactor = 1.0;
      let emissionsFactor = 1.0;
      
      // Gradually phase out fossil fuels
      if (g.type === 'coal') {
        outputFactor = 1 - (yearProgress * 0.9); // Almost gone by end year
        if (year > constants.START_YEAR + Math.floor((constants.END_YEAR - constants.START_YEAR) * 0.7)) {
          return null; // Coal plants decommissioned in later years
        }
      } else if (g.type === 'gas') {
        outputFactor = 1 - (yearProgress * 0.5); // Reduced by end year
      }
      
      // Increase renewable energy
      if (g.type === 'wind' || g.type === 'solar') {
        outputFactor = 1 + (yearProgress * 2.0); // Up to 3x by end year
      }
      
      // Improved efficiency reduces emissions
      emissionsFactor = 1 - (yearProgress * 0.2); // Up to 20% reduction by end year
      
      return g ? {
        ...g,
        year: year,
        output: Math.floor(g.output * outputFactor),
        emissions: g.emissions * outputFactor * emissionsFactor
      } : null;
    }).filter(g => g !== null);
    
    // Add new renewable generators over time
    if (year > constants.START_YEAR && year % 5 === 0) {
      const numNewGenerators = Math.floor((year - constants.START_YEAR) / 5);
      log(`Adding ${numNewGenerators} new renewable generators for year ${year}`);
      
      for (let i = 0; i < numNewGenerators; i++) {
        const type = Math.random() > 0.5 ? 'wind' : 'solar';
        const newGenerator = {
          id: `G${baseGenerators.length + i}_${year}`,
          name: `New ${type.charAt(0).toUpperCase() + type.slice(1)} Generator ${i + 1}`,
          year: year,
          type: type,
          capacity: Math.floor(Math.random() * 300) + 200, // Higher capacity for newer plants
          output: Math.floor(Math.random() * 300) + 200,
          emissions: 0, // Renewables have zero emissions
          lat: bounds.south + Math.random() * (bounds.north - bounds.south),
          lng: bounds.west + Math.random() * (bounds.east - bounds.west),
          gridX: (baseGenerators.length + i) % 10,
          gridY: Math.floor((baseGenerators.length + i) / 10)
        };
        yearGenerators.push(newGenerator);
      }
    }
    
    generators = generators.concat(yearGenerators);
    log(`Generated ${yearGenerators.length} generators for year ${year}`);
    
    // Modify carbon offsets for this year (increasing over time)
    const yearOffsets = baseOffsets.map(o => {
      const offsetFactor = 1 + (yearProgress * 1.5); // Up to 2.5x by end year
      const areaFactor = 1 + (yearProgress * 0.5); // Up to 1.5x by end year
      
      return {
        ...o,
        year: year,
        offsetAmount: Math.floor(o.offsetAmount * offsetFactor),
        area: Math.floor(o.area * areaFactor)
      };
    });
    
    // Add new carbon offset projects over time
    if (year > constants.START_YEAR && year % 3 === 0) {
      const numNewOffsets = Math.floor((year - constants.START_YEAR) / 3);
      log(`Adding ${numNewOffsets} new carbon offset projects for year ${year}`);
      
      for (let i = 0; i < numNewOffsets; i++) {
        const type = constants.OFFSET_TYPES[Math.floor(Math.random() * constants.OFFSET_TYPES.length)];
        const newOffset = {
          id: `O${baseOffsets.length + i}_${year}`,
          name: `New ${type.charAt(0).toUpperCase() + type.slice(1).replace('_', ' ')} Project ${i + 1}`,
          year: year,
          type: type,
          offsetAmount: Math.floor(Math.random() * 1500) + 500, // Higher offset for newer projects
          area: Math.floor(Math.random() * 700) + 300,
          lat: bounds.south + Math.random() * (bounds.north - bounds.south),
          lng: bounds.west + Math.random() * (bounds.east - bounds.west),
          gridX: (baseOffsets.length + i) % 10,
          gridY: Math.floor((baseOffsets.length + i) / 10)
        };
        yearOffsets.push(newOffset);
      }
    }
    
    offsets = offsets.concat(yearOffsets);
    log(`Generated ${yearOffsets.length} carbon offsets for year ${year}`);
    
    // Create yearly summary data
    const totalPopulation = yearSettlements.reduce((sum, s) => sum + s.population, 0);
    const totalPowerUsage = yearSettlements.reduce((sum, s) => sum + s.powerUsage, 0);
    const totalPowerGeneration = yearGenerators.reduce((sum, g) => sum + g.output, 0);
    const totalEmissions = yearGenerators.reduce((sum, g) => sum + g.emissions, 0);
    const totalOffsets = yearOffsets.reduce((sum, o) => sum + o.offsetAmount, 0);
    
    const yearSummary = {
      year: year,
      totalPopulation: totalPopulation,
      totalPowerUsage: totalPowerUsage,
      totalPowerGeneration: totalPowerGeneration,
      powerBalance: totalPowerGeneration - totalPowerUsage,
      totalEmissions: totalEmissions,
      totalOffsets: totalOffsets,
      netEmissions: totalEmissions - totalOffsets,
      publicOpinion: Math.min(1, Math.max(0, 0.5 + (yearProgress * 0.3) - 
                               (totalEmissions - totalOffsets) / 100000))
    };
    
    summary.push(yearSummary);
    log(`Generated summary for year ${year}: population=${totalPopulation}, powerBalance=${yearSummary.powerBalance}, netEmissions=${yearSummary.netEmissions}`);
  }
  
  log(`Sample data generation complete: ${settlements.length} total settlement entries, ${generators.length} total generator entries, ${offsets.length} total offset entries`);
  
  return {
    summary: summary,
    settlements: settlements,
    generators: generators,
    offsets: offsets,
    startYear: constants.START_YEAR,
    endYear: constants.END_YEAR
  };
}; 