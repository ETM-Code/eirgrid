/**
 * Metrics Panel Module
 * 
 * This module manages the metrics panel in the Irish Power Grid Simulation Visualizer.
 * It displays key information about the current simulation year, including
 * power statistics, emissions data, and energy mix.
 */

// Helper function to log from this module
function log(message, level = 'info') {
  if (window.AppLog) {
    window.AppLog('MetricsPanel', message, level);
  } else {
    console.log(`[MetricsPanel] ${message}`);
  }
}

// Ensure MetricsPanel exists as a global object
window.MetricsPanel = window.MetricsPanel || {};

// DOM elements cache
let elements = {
  populationEl: null,
  powerGenerationEl: null,
  powerUsageEl: null,
  powerBalanceEl: null,
  emissionsEl: null,
  carbonOffsetEl: null,
  netEmissionsEl: null,
  opinionEl: null,
  yearInfoEl: null
};

/**
 * Initialize the metrics panel
 */
window.MetricsPanel.init = function() {
  log('Initializing metrics panel');
  
  // Get references to DOM elements based on the existing HTML structure
  elements.populationEl = document.getElementById('population');
  elements.powerGenerationEl = document.getElementById('power-generation');
  elements.powerUsageEl = document.getElementById('power-usage');
  elements.powerBalanceEl = document.getElementById('power-balance');
  elements.emissionsEl = document.getElementById('emissions');
  elements.carbonOffsetEl = document.getElementById('carbon-offset');
  elements.netEmissionsEl = document.getElementById('net-emissions');
  elements.opinionEl = document.getElementById('opinion');
  elements.yearInfoEl = document.getElementById('current-year');
  
  log('Metrics panel initialized successfully');
};

/**
 * Update the metrics panel with new data for the current year
 * @param {Object} data - Simulation data for the current year
 */
window.MetricsPanel.update = function(data) {
  if (!data) {
    log('No data provided for metrics update', 'warn');
    return;
  }
  
  log(`Updating metrics panel with data for year ${data.year || 'unknown'}`);
  
  try {
    updateMetrics(data);
    if (elements.yearInfoEl) {
      elements.yearInfoEl.textContent = data.year || 'unknown';
    }
    log('Metrics panel updated successfully');
  } catch (error) {
    log(`Error updating metrics panel: ${error.message}`, 'error');
  }
};

/**
 * Update all metrics with new data
 * @param {Object} data - Simulation data for the current year
 */
function updateMetrics(data) {
  // Calculate metrics
  const totalPopulation = data.settlements ? data.settlements.reduce((total, settlement) => total + (settlement.population || 0), 0) : 0;
  
  const totalGeneration = data.generators ? data.generators.reduce((total, generator) => total + (generator.output || 0), 0) : 0;
  
  const totalUsage = data.settlements ? data.settlements.reduce((total, settlement) => total + (settlement.powerUsage || 0), 0) : 0;
  
  const powerBalance = totalGeneration - totalUsage;
  
  const totalEmissions = data.generators ? data.generators.reduce((total, generator) => total + (generator.emissions || 0), 0) : 0;
  
  const totalOffsets = data.carbonOffsets ? data.carbonOffsets.reduce((total, offset) => total + (offset.offsetAmount || 0), 0) : 0;
  
  const netEmissions = totalEmissions - totalOffsets;
  
  // Public opinion calculation (simplified)
  const renewablePercentage = totalGeneration > 0 ? 
    data.generators
      .filter(gen => ['wind', 'solar', 'hydro', 'tidal', 'geothermal'].includes((gen.type || '').toLowerCase()))
      .reduce((total, generator) => total + (generator.output || 0), 0) / totalGeneration * 100 
    : 0;
    
  const carbonNeutralScore = netEmissions <= 0 ? 100 : Math.max(0, 100 - (netEmissions / totalEmissions * 100));
  
  const publicOpinion = Math.round((renewablePercentage * 0.6 + carbonNeutralScore * 0.4));
  
  // Update DOM elements with calculated values
  if (elements.populationEl) {
    elements.populationEl.textContent = totalPopulation.toLocaleString();
    elements.populationEl.classList.toggle('positive', totalPopulation > 0);
  }
  
  if (elements.powerGenerationEl) {
    elements.powerGenerationEl.textContent = `${totalGeneration.toLocaleString()} MW`;
    elements.powerGenerationEl.classList.toggle('positive', totalGeneration > 0);
  }
  
  if (elements.powerUsageEl) {
    elements.powerUsageEl.textContent = `${totalUsage.toLocaleString()} MW`;
  }
  
  if (elements.powerBalanceEl) {
    elements.powerBalanceEl.textContent = `${powerBalance.toLocaleString()} MW`;
    elements.powerBalanceEl.classList.toggle('positive', powerBalance >= 0);
    elements.powerBalanceEl.classList.toggle('negative', powerBalance < 0);
  }
  
  if (elements.emissionsEl) {
    elements.emissionsEl.textContent = `${totalEmissions.toLocaleString()} tonnes`;
    elements.emissionsEl.classList.toggle('negative', totalEmissions > 0);
  }
  
  if (elements.carbonOffsetEl) {
    elements.carbonOffsetEl.textContent = `${totalOffsets.toLocaleString()} tonnes`;
    elements.carbonOffsetEl.classList.toggle('positive', totalOffsets > 0);
  }
  
  if (elements.netEmissionsEl) {
    elements.netEmissionsEl.textContent = `${netEmissions.toLocaleString()} tonnes`;
    elements.netEmissionsEl.classList.toggle('positive', netEmissions <= 0);
    elements.netEmissionsEl.classList.toggle('negative', netEmissions > 0);
  }
  
  if (elements.opinionEl) {
    elements.opinionEl.textContent = `${publicOpinion}%`;
    
    if (publicOpinion >= 75) {
      elements.opinionEl.classList.add('positive');
      elements.opinionEl.classList.remove('negative');
    } else if (publicOpinion < 40) {
      elements.opinionEl.classList.add('negative');
      elements.opinionEl.classList.remove('positive');
    } else {
      elements.opinionEl.classList.remove('positive');
      elements.opinionEl.classList.remove('negative');
    }
  }
} 