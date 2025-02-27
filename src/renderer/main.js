/**
 * Main JavaScript module
 * 
 * This is the main entry point for the Irish Power Grid Simulation Visualizer.
 * It initializes all components and handles the application lifecycle.
 */

// Helper function to log from this module
function log(message, level = 'info') {
  if (window.AppLog) {
    window.AppLog('Main', message, level);
  } else {
    console.log(`[Main] ${message}`);
  }
}

// Global App object
window.App = window.App || {
  initialized: false,
  map: null,
  currentYear: null,
  yearData: {},
  
  /**
   * Refresh the application with newly loaded data
   * This is called when CSV data is loaded by the user
   */
  refreshData: function() {
    log('Refreshing application with new data');
    
    // Update year range in the timeline
    updateYearRange();
    
    // Set current year to the first year
    const minYear = window.DataLoader.getMinYear();
    if (minYear) {
      handleYearChange(minYear);
    }
    
    // Update data source information
    const dataSourceElement = document.getElementById('data-source');
    if (dataSourceElement) {
      dataSourceElement.textContent = 'Using custom CSV data';
    }
    
    // Generate yearly metrics for charts
    if (window.Charts) {
      const yearsList = window.DataLoader.getYearsList();
      const yearlyMetrics = [];
      
      yearsList.forEach(year => {
        const data = window.DataLoader.getYearData(year);
        if (data) {
          // Calculate metrics for the year
          const totalEmissions = data.generators.reduce((total, gen) => total + (gen.emissions || 0), 0);
          const totalOffsets = data.carbonOffsets.reduce((total, offset) => total + (offset.offsetAmount || 0), 0);
          const totalGeneration = data.generators.reduce((total, gen) => total + (gen.output || 0), 0);
          const totalUsage = data.settlements.reduce((total, settlement) => total + (settlement.powerUsage || 0), 0);
          
          yearlyMetrics.push({
            year,
            co2Emissions: totalEmissions,
            carbonOffset: totalOffsets,
            netEmissions: totalEmissions - totalOffsets,
            totalPowerGeneration: totalGeneration,
            totalPowerUsage: totalUsage,
            powerBalance: totalGeneration - totalUsage
          });
        }
      });
      
      // Update charts with new data
      window.Charts.init(yearlyMetrics);
    }
    
    log('Application refreshed with new data');
  }
};

// Wait for DOM to be ready
document.addEventListener('DOMContentLoaded', function() {
  log('DOM loaded, initializing application');
  initApp();
});

/**
 * Initialize the application
 */
function initApp() {
  log('Initializing Irish Power Grid Simulation Visualizer');
  
  try {
    // Initialize components in sequence
    initMap()
      .then(initDataLoader)
      .then(initVisualization)
      .then(initMetricsPanel)
      .then(initCharts)
      .then(initTimeline)
      .then(() => {
        // Setup is complete
        window.App.initialized = true;
        
        // Hide loading overlay
        const loadingOverlay = document.getElementById('loading-overlay');
        if (loadingOverlay) {
          loadingOverlay.style.display = 'none';
        }
        
        // Setup other UI elements
        setupUiListeners();
        setupFileInput();
        
        log('Application initialization complete');
      })
      .catch(error => {
        log(`Error initializing application: ${error.message}`, 'error');
        showError('Failed to initialize application: ' + error.message);
      });
  } catch (error) {
    log(`Error initializing application: ${error.message}`, 'error');
    showError('Failed to initialize application: ' + error.message);
  }
}

/**
 * Initialize the map
 * @returns {Promise}
 */
function initMap() {
  log('Initializing map');
  
  return new Promise((resolve, reject) => {
    if (!window.MapSetup) {
      reject(new Error('MapSetup module not available'));
      return;
    }
    
    window.MapSetup.init()
      .then(map => {
        window.App.map = map;
        log('Map initialized successfully');
        resolve();
      })
      .catch(error => {
        log(`Map initialization failed: ${error.message}`, 'error');
        reject(error);
      });
  });
}

/**
 * Initialize the data loader
 * @returns {Promise}
 */
function initDataLoader() {
  log('Initializing data loader');
  
  return new Promise((resolve, reject) => {
    if (!window.DataLoader) {
      reject(new Error('DataLoader module not available'));
      return;
    }
    
    window.DataLoader.init()
      .then(data => {
        log('Data loaded successfully');
        
        // Update timeline with year range
        updateYearRange();
        
        // Set current year to the first year
        const minYear = window.DataLoader.getMinYear();
        if (minYear) {
          handleYearChange(minYear);
        }
        
        resolve();
      })
      .catch(error => {
        log(`Data loading failed: ${error.message}`, 'error');
        reject(error);
      });
  });
}

/**
 * Initialize the visualization
 * @returns {Promise}
 */
function initVisualization() {
  log('Initializing visualization');
  
  return new Promise((resolve, reject) => {
    try {
      if (!window.Visualization) {
        reject(new Error('Visualization module not available'));
        return;
      }
      
      if (!window.App.map) {
        reject(new Error('Map not initialized'));
        return;
      }
      
      window.Visualization.init(window.App.map);
      log('Visualization initialized successfully');
      resolve();
    } catch (error) {
      log(`Visualization initialization failed: ${error.message}`, 'error');
      reject(error);
    }
  });
}

/**
 * Initialize the metrics panel
 * @returns {Promise}
 */
function initMetricsPanel() {
  log('Initializing metrics panel');
  
  return new Promise((resolve, reject) => {
    try {
      if (!window.MetricsPanel) {
        reject(new Error('MetricsPanel module not available'));
        return;
      }
      
      window.MetricsPanel.init();
      log('Metrics panel initialized successfully');
      resolve();
    } catch (error) {
      log(`Metrics panel initialization failed: ${error.message}`, 'error');
      reject(error);
    }
  });
}

/**
 * Initialize the charts
 * @returns {Promise}
 */
function initCharts() {
  log('Initializing charts');
  
  return new Promise((resolve, reject) => {
    try {
      if (!window.Charts) {
        reject(new Error('Charts module not available'));
        return;
      }
      
      // Generate yearly metrics data for charts
      const yearsList = window.DataLoader.getYearsList();
      const yearlyMetrics = [];
      
      yearsList.forEach(year => {
        const data = window.DataLoader.getYearData(year);
        if (data) {
          // Add safety checks for all data properties
          // Calculate metrics for the year safely
          const totalEmissions = Array.isArray(data.generators) 
            ? data.generators.reduce((total, gen) => total + (gen.emissions || 0), 0)
            : 0;
            
          const totalOffsets = Array.isArray(data.carbonOffsets) 
            ? data.carbonOffsets.reduce((total, offset) => total + (offset.offsetAmount || 0), 0)
            : 0;
            
          const totalGeneration = Array.isArray(data.generators)
            ? data.generators.reduce((total, gen) => total + (gen.output || 0), 0)
            : 0;
            
          const totalUsage = Array.isArray(data.settlements)
            ? data.settlements.reduce((total, settlement) => total + (settlement.powerUsage || 0), 0)
            : 0;
          
          yearlyMetrics.push({
            year,
            co2Emissions: totalEmissions,
            carbonOffset: totalOffsets,
            netEmissions: totalEmissions - totalOffsets,
            totalPowerGeneration: totalGeneration,
            totalPowerUsage: totalUsage,
            powerBalance: totalGeneration - totalUsage
          });
        }
      });
      
      window.Charts.init(yearlyMetrics);
      log('Charts initialized successfully');
      resolve();
    } catch (error) {
      log(`Charts initialization failed: ${error.message}`, 'error');
      reject(error);
    }
  });
}

/**
 * Initialize the timeline
 * @returns {Promise}
 */
function initTimeline() {
  log('Initializing timeline');
  
  return new Promise((resolve, reject) => {
    try {
      if (!window.Timeline) {
        reject(new Error('Timeline module not available'));
        return;
      }
      
      // Initialize timeline with year change handler
      window.Timeline.init(handleYearChange);
      
      // Update the timeline year range
      updateYearRange();
      
      log('Timeline initialized successfully');
      resolve();
    } catch (error) {
      log(`Timeline initialization failed: ${error.message}`, 'error');
      reject(error);
    }
  });
}

/**
 * Update the year range in the timeline
 */
function updateYearRange() {
  log('Updating year range');
  
  const minYear = window.DataLoader.getMinYear();
  const maxYear = window.DataLoader.getMaxYear();
  
  if (minYear && maxYear) {
    log(`Setting year range: ${minYear}-${maxYear}`);
    window.Timeline.updateYearRange(minYear, maxYear);
    
    // Update timeline labels
    updateTimelineLabels(minYear, maxYear);
  } else {
    log('Could not determine year range', 'warn');
  }
}

/**
 * Update timeline labels with relevant years
 * @param {number} minYear - Start year
 * @param {number} maxYear - End year
 */
function updateTimelineLabels(minYear, maxYear) {
  log(`Updating timeline labels for range ${minYear}-${maxYear}`);
  
  const labelsContainer = document.querySelector('.timeline-labels');
  if (!labelsContainer) {
    log('Timeline labels container not found', 'warn');
    return;
  }
  
  // Clear existing labels
  labelsContainer.innerHTML = '';
  
  // Determine how many labels to show (adapt based on range)
  const range = maxYear - minYear;
  const labelStep = range <= 10 ? 1 : range <= 30 ? 5 : 10;
  
  // Create labels
  for (let year = minYear; year <= maxYear; year += labelStep) {
    const label = document.createElement('div');
    label.className = 'timeline-label';
    label.textContent = year.toString();
    
    // Calculate position as percentage
    const position = ((year - minYear) / range) * 100;
    label.style.left = `${position}%`;
    
    labelsContainer.appendChild(label);
  }
  
  log(`Timeline labels updated with step ${labelStep}`);
}

/**
 * Handle year change events from the timeline
 * @param {number} year - New year value
 */
function handleYearChange(year) {
  log(`Year changed to ${year}`);
  
  window.App.currentYear = year;
  
  // Update current year display in header
  const yearDisplay = document.getElementById('current-year');
  if (yearDisplay) {
    yearDisplay.textContent = year.toString();
  }
  
  // Get data for the current year
  const yearData = window.DataLoader.getYearData(year);
  if (yearData) {
    window.App.yearData = yearData;
    
    // Ensure all required arrays exist to prevent errors
    yearData.settlements = Array.isArray(yearData.settlements) ? yearData.settlements : [];
    yearData.generators = Array.isArray(yearData.generators) ? yearData.generators : [];
    yearData.carbonOffsets = Array.isArray(yearData.carbonOffsets) ? yearData.carbonOffsets : [];
    
    // Update UI with new data
    updateUI(yearData);
    
    // Update charts year indicator
    window.Charts.update(null, year);
  } else {
    log(`No data available for year ${year}`, 'warn');
  }
}

/**
 * Update UI components with new data
 * @param {Object} data - Data for the current year
 */
function updateUI(data) {
  log(`Updating UI with data for year ${data.year}`);
  
  // Update visualization
  window.Visualization.update(data);
  
  // Update metrics panel
  window.MetricsPanel.update(data);
  
  log('UI updated successfully');
}

/**
 * Set up UI event listeners
 */
function setupUiListeners() {
  log('Setting up UI event listeners');
  
  // Toggle sidebar
  const sidebarToggle = document.getElementById('sidebar-toggle');
  const sidebar = document.getElementById('sidebar');
  
  if (sidebarToggle && sidebar) {
    sidebarToggle.addEventListener('click', function() {
      sidebar.classList.toggle('collapsed');
      sidebarToggle.querySelector('.toggle-icon').textContent = 
        sidebar.classList.contains('collapsed') ? '▶' : '◀';
    });
  }
  
  // Debug toggle
  const debugToggle = document.getElementById('debug-toggle');
  if (debugToggle) {
    debugToggle.addEventListener('change', function() {
      document.body.classList.toggle('debug-mode', this.checked);
    });
  }
  
  // Layer visibility toggles
  setupLayerToggles();
  
  log('UI event listeners setup complete');
}

/**
 * Set up layer visibility toggles
 */
function setupLayerToggles() {
  // Settlement visibility
  const showSettlements = document.getElementById('show-settlements');
  if (showSettlements) {
    showSettlements.addEventListener('change', function() {
      document.body.classList.toggle('hide-settlements', !this.checked);
    });
  }
  
  // Generator visibility
  const showGenerators = document.getElementById('show-generators');
  if (showGenerators) {
    showGenerators.addEventListener('change', function() {
      document.body.classList.toggle('hide-generators', !this.checked);
    });
  }
  
  // Offsets visibility
  const showOffsets = document.getElementById('show-offsets');
  if (showOffsets) {
    showOffsets.addEventListener('change', function() {
      document.body.classList.toggle('hide-offsets', !this.checked);
    });
  }
}

/**
 * Set up file input for loading custom data
 */
function setupFileInput() {
  log('Setting up file input for custom data loading');
  
  // Create a load data button in the header
  const headerElement = document.querySelector('.header');
  if (headerElement) {
    // Check if button already exists to prevent duplicates
    if (document.getElementById('load-csv-button')) {
      log('Load CSV button already exists', 'warn');
      return;
    }
    
    // Create button with distinctive styling
    const loadButton = document.createElement('button');
    loadButton.id = 'load-csv-button';
    loadButton.className = 'load-button';
    loadButton.innerHTML = '<span class="load-icon">📂</span> Load CSV Data';
    
    // Add inline styles to ensure button is visible regardless of CSS loading issues
    loadButton.style.display = 'flex';
    loadButton.style.alignItems = 'center';
    loadButton.style.gap = '8px';
    loadButton.style.backgroundColor = '#2563eb';
    loadButton.style.color = '#ffffff';
    loadButton.style.padding = '8px 16px';
    loadButton.style.borderRadius = '4px';
    loadButton.style.fontSize = '14px';
    loadButton.style.fontWeight = '500';
    loadButton.style.marginLeft = '16px';
    loadButton.style.boxShadow = '0 1px 2px 0 rgba(0, 0, 0, 0.05)';
    
    // Add click event listener
    loadButton.addEventListener('click', function(event) {
      // Show loading overlay
      const loadingOverlay = document.getElementById('loading-overlay');
      if (loadingOverlay) {
        loadingOverlay.style.display = 'flex';
        loadingOverlay.innerHTML = `
          <div class="spinner"></div>
          <div class="loading-message">Loading CSV files...</div>
        `;
      }
      
      // Load CSV data - this will now work because it's triggered by a direct user action
      window.DataLoader.loadCsvData()
        .then(data => {
          log('Successfully loaded CSV data from user selection');
          
          // Hide loading overlay
          if (loadingOverlay) {
            loadingOverlay.style.display = 'none';
          }
        })
        .catch(error => {
          log(`Failed to load CSV data: ${error.message}`, 'warn');
          
          // Update loading overlay with error
          if (loadingOverlay) {
            loadingOverlay.innerHTML = `
              <div class="error-icon">⚠️</div>
              <div class="error-message">Error loading CSV data: ${error.message}</div>
              <button id="dismiss-error">Dismiss</button>
            `;
            
            // Add inline styles to error message
            const errorIcon = loadingOverlay.querySelector('.error-icon');
            const errorMessage = loadingOverlay.querySelector('.error-message');
            const dismissButton = loadingOverlay.querySelector('#dismiss-error');
            
            if (errorIcon) {
              errorIcon.style.fontSize = '40px';
              errorIcon.style.marginBottom = '16px';
            }
            
            if (errorMessage) {
              errorMessage.style.fontSize = '16px';
              errorMessage.style.color = '#ef4444';
              errorMessage.style.textAlign = 'center';
              errorMessage.style.maxWidth = '400px';
              errorMessage.style.marginBottom = '16px';
            }
            
            if (dismissButton) {
              dismissButton.style.padding = '8px 16px';
              dismissButton.style.backgroundColor = '#2563eb';
              dismissButton.style.color = '#ffffff';
              dismissButton.style.borderRadius = '4px';
              dismissButton.style.border = 'none';
              dismissButton.style.cursor = 'pointer';
              
              dismissButton.addEventListener('click', function() {
                loadingOverlay.style.display = 'none';
              });
            }
          }
        });
    });
    
    // Add to header
    headerElement.appendChild(loadButton);
    log('Load CSV button added to header');
  } else {
    log('Header element not found, cannot add load button', 'error');
  }
}

/**
 * Show error message
 * @param {string} message - Error message to display
 */
function showError(message) {
  log(`Displaying error: ${message}`, 'error');
  
  // Hide loading overlay
  const loadingOverlay = document.getElementById('loading-overlay');
  if (loadingOverlay) {
    loadingOverlay.classList.add('error');
    loadingOverlay.innerHTML = `
      <div class="error-icon">⚠️</div>
      <div class="error-message">${message}</div>
      <button id="retry-button">Retry</button>
    `;
    
    // Add retry button listener
    const retryButton = document.getElementById('retry-button');
    if (retryButton) {
      retryButton.addEventListener('click', function() {
        window.location.reload();
      });
    }
  } else {
    // Fallback to alert if overlay not found
    alert(`Error: ${message}`);
  }
} 