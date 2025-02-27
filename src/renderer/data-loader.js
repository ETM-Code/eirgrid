/**
 * Data Loader Module
 * 
 * This module handles loading and processing data for the Irish Power Grid Simulation.
 * It can load data from CSV files or generate sample data when needed.
 */

// Helper function to log from this module
function log(message, level = 'info') {
  if (window.AppLog) {
    window.AppLog('DataLoader', message, level);
  } else {
    console.log(`[DataLoader] ${message}`);
  }
}

// Ensure DataLoader exists as a global object
window.DataLoader = window.DataLoader || {};

// Data cache
let yearDataCache = {};
let yearsList = [];
let isDataLoaded = false;
let minYear = null;
let maxYear = null;

// Initialize the module
window.DataLoader.state = {
  data: null,
  startYear: 2025,
  endYear: 2050,
  currentYear: 2025,
  isLoaded: false,
  loadPromise: null
};

/**
 * Initialize the data loader
 * @returns {Promise} Promise that resolves when initialization is complete
 */
window.DataLoader.init = function() {
  log('Initializing data loader');
  
  // Create a promise for loading data
  window.DataLoader.state.loadPromise = new Promise((resolve, reject) => {
    try {
      // Load data from CSV files in yearly_details directory
      loadYearlyDetailsData()
        .then(data => {
          window.DataLoader.state.data = data;
          window.DataLoader.state.isLoaded = true;
          resolve(data);
          log('Successfully loaded data from yearly_details directory');
        })
        .catch(error => {
          log(`Failed to load data from yearly_details: ${error.message}`, 'warn');
          log('Falling back to sample data generation');
          
          // Fall back to sample data if CSV loading fails
          if (window.SampleData && typeof window.SampleData.generate === 'function') {
            try {
              const sampleData = window.SampleData.generate(
                window.DataLoader.state.startYear, 
                window.DataLoader.state.endYear
              );
              window.DataLoader.state.data = sampleData;
              window.DataLoader.state.isLoaded = true;
              resolve(sampleData);
              log('Successfully generated sample data as fallback');
            } catch (sampleError) {
              log(`Failed to generate sample data: ${sampleError.message}`, 'error');
              reject(sampleError);
            }
          } else {
            log('SampleData module not available', 'error');
            reject(new Error('Data loading failed'));
          }
        });
    } catch (error) {
      log(`Critical error in data loader initialization: ${error.message}`, 'error');
      reject(error);
    }
  });
  
  // Process the loaded data to extract years and ranges
  window.DataLoader.state.loadPromise.then(() => {
    processLoadedData();
  });
  
  return window.DataLoader.state.loadPromise;
};

/**
 * Process the loaded data to extract year range and organize data
 */
function processLoadedData() {
  log('Processing loaded data');
  
  // Check if data is loaded
  if (!window.DataLoader.state.data || Object.keys(window.DataLoader.state.data).length === 0) {
    log('No data to process', 'warn');
    return;
  }
  
  // Extract years and sort them
  yearsList = Object.keys(window.DataLoader.state.data).map(Number).sort((a, b) => a - b);
  
  if (yearsList.length > 0) {
    minYear = yearsList[0];
    maxYear = yearsList[yearsList.length - 1];
    
    window.DataLoader.state.startYear = minYear;
    window.DataLoader.state.endYear = maxYear;
    window.DataLoader.state.currentYear = minYear;
  }
  
  log(`Data processed: years range from ${minYear} to ${maxYear}`);
  isDataLoaded = true;
}

/**
 * Load data from CSV files in the yearly_details directory
 * @returns {Promise} Promise that resolves with loaded data
 */
function loadYearlyDetailsData() {
  log('Loading data from yearly_details directory');
  
  return new Promise((resolve, reject) => {
    // Check if Papa Parse is available
    if (typeof Papa === 'undefined') {
      reject(new Error('CSV parsing library (Papa Parse) not available'));
      return;
    }
    
    const baseUrl = 'src/renderer/data/yearly_details/';
    const filesToLoad = [
      { url: `${baseUrl}settlements.csv`, type: 'settlements' },
      { url: `${baseUrl}generators.csv`, type: 'generators' },
      { url: `${baseUrl}carbon_offsets.csv`, type: 'offsets' }
    ];
    
    const processedData = {};
    let filesLoaded = 0;
    let hasErrors = false;
    
    filesToLoad.forEach(file => {
      log(`Loading ${file.type} data from ${file.url}`);
      
      Papa.parse(file.url, {
        download: true,
        header: true,
        dynamicTyping: true,
        skipEmptyLines: true,
        complete: function(results) {
          if (results.errors && results.errors.length > 0) {
            log(`Error parsing ${file.url}: ${results.errors[0].message}`, 'error');
            hasErrors = true;
          } else {
            log(`Successfully parsed ${file.url}: ${results.data.length} rows`);
            
            // Process the CSV data based on file type
            switch(file.type) {
              case 'settlements':
                processSettlementData(results.data, processedData);
                break;
              case 'generators':
                processGeneratorData(results.data, processedData);
                break;
              case 'offsets':
                processCarbonOffsetData(results.data, processedData);
                break;
            }
          }
          
          filesLoaded++;
          if (filesLoaded === filesToLoad.length) {
            finishLoading();
          }
        },
        error: function(error) {
          log(`Error loading ${file.url}: ${error.message}`, 'error');
          hasErrors = true;
          
          filesLoaded++;
          if (filesLoaded === filesToLoad.length) {
            finishLoading();
          }
        }
      });
    });
    
    function finishLoading() {
      if (Object.keys(processedData).length > 0) {
        log(`Data loading complete: ${Object.keys(processedData).length} years of data loaded`);
        resolve(processedData);
      } else {
        reject(new Error('No valid data found in CSV files'));
      }
    }
  });
}

/**
 * Process CSV file data and organize it by year
 * @param {string} fileName - Name of the file being processed
 * @param {Array} data - Array of parsed CSV rows
 * @param {Object} processedData - Object to store processed data
 */
function processFileData(fileName, data, processedData) {
  if (!data || data.length === 0) {
    log(`No data found in ${fileName}`, 'warn');
    return;
  }
  
  // Determine what type of data this file contains based on filename or content
  if (fileName.toLowerCase().includes('settlement')) {
    processSettlementData(data, processedData);
  } else if (fileName.toLowerCase().includes('generator')) {
    processGeneratorData(data, processedData);
  } else if (fileName.toLowerCase().includes('offset') || fileName.toLowerCase().includes('carbon')) {
    processCarbonOffsetData(data, processedData);
  } else {
    // Try to determine type from content
    const firstRow = data[0];
    if (firstRow.hasOwnProperty('population')) {
      processSettlementData(data, processedData);
    } else if (firstRow.hasOwnProperty('output') || firstRow.hasOwnProperty('type')) {
      processGeneratorData(data, processedData);
    } else if (firstRow.hasOwnProperty('offsetAmount')) {
      processCarbonOffsetData(data, processedData);
    } else {
      log(`Could not determine data type for ${fileName}`, 'warn');
    }
  }
}

/**
 * Process settlement data from CSV
 * @param {Array} data - Array of parsed CSV rows
 * @param {Object} processedData - Object to store processed data
 */
function processSettlementData(data, processedData) {
  data.forEach(row => {
    if (!row.Year) return;
    
    const year = Number(row.Year);
    if (!processedData[year]) {
      processedData[year] = {
        settlements: [],
        generators: [],
        carbonOffsets: []
      };
    }
    
    processedData[year].settlements.push({
      id: row['Settlement ID'] || `Settlement_${Math.random().toString(36).substring(2, 10)}`,
      name: row.Name || `Settlement ${row['Settlement ID']}`,
      lat: Number(row.Y) || 0,
      lng: Number(row.X) || 0,
      population: Number(row.Population) || 0,
      powerUsage: Number(row['Power Usage (MW)']) || 0,
      powerPerCapita: Number(row['Power Usage Per Capita (kW)']) || 0
    });
  });
  
  log(`Processed settlement data for ${Object.keys(processedData).length} years`);
}

/**
 * Process generator data from CSV
 * @param {Array} data - Array of parsed CSV rows
 * @param {Object} processedData - Object to store processed data
 */
function processGeneratorData(data, processedData) {
  data.forEach(row => {
    if (!row.Year) return;
    
    const year = Number(row.Year);
    if (!processedData[year]) {
      processedData[year] = {
        settlements: [],
        generators: [],
        carbonOffsets: []
      };
    }
    
    processedData[year].generators.push({
      id: row['Generator ID'] || `Generator_${Math.random().toString(36).substring(2, 10)}`,
      name: row.Type ? `${row.Type} Generator ${row['Generator ID']}` : `Generator ${row['Generator ID']}`,
      type: row.Type || 'unknown',
      lat: Number(row.Y) || 0,
      lng: Number(row.X) || 0,
      output: Number(row['Power Output (MW)']) || 0,
      emissions: Number(row['CO2 Output (tonnes)']) || 0,
      efficiency: Number(row['Efficiency (%)']) || 0,
      yearBuilt: Number(row['Commissioning Year']) || year,
      endOfLifeYear: Number(row['End of Life Year']) || (year + 25)
    });
  });
  
  log(`Processed generator data for ${Object.keys(processedData).length} years`);
}

/**
 * Process carbon offset data from CSV
 * @param {Array} data - Array of parsed CSV rows
 * @param {Object} processedData - Object to store processed data
 */
function processCarbonOffsetData(data, processedData) {
  data.forEach(row => {
    if (!row.Year) return;
    
    const year = Number(row.Year);
    if (!processedData[year]) {
      processedData[year] = {
        settlements: [],
        generators: [],
        carbonOffsets: []
      };
    }
    
    processedData[year].carbonOffsets.push({
      id: row['Offset ID'] || `Offset_${Math.random().toString(36).substring(2, 10)}`,
      name: row.Type ? `${row.Type} Offset ${row['Offset ID']}` : `Offset ${row['Offset ID']}`,
      type: row.Type || 'unknown',
      lat: Number(row.Y) || 0,
      lng: Number(row.X) || 0,
      offsetAmount: Number(row['CO2 Offset (tonnes)']) || 0,
      negativeEmissions: Number(row['Negative CO2 Emissions (tonnes)']) || 0,
      size: Number(row.Size) || 0,
      yearStarted: year,
      powerConsumption: Number(row['Power Consumption (MW)']) || 0,
      captureEfficiency: Number(row['Capture Efficiency (%)']) || 0
    });
  });
  
  log(`Processed carbon offset data for ${Object.keys(processedData).length} years`);
}

/**
 * Get data for a specific year
 * @param {number} year - Year to get data for
 * @returns {Object|null} Data for the specified year, or null if not available
 */
window.DataLoader.getYearData = function(year) {
  if (!window.DataLoader.state.isLoaded) {
    log('Data not loaded yet, cannot get year data', 'warn');
    return null;
  }
  
  if (!window.DataLoader.state.data[year]) {
    log(`No data available for year ${year}`, 'warn');
    return null;
  }
  
  log(`Retrieving data for year ${year}`);
  return window.DataLoader.state.data[year];
};

/**
 * Get the minimum year in the data set
 * @returns {number|null} Minimum year, or null if no data is loaded
 */
window.DataLoader.getMinYear = function() {
  return minYear;
};

/**
 * Get the maximum year in the data set
 * @returns {number|null} Maximum year, or null if no data is loaded
 */
window.DataLoader.getMaxYear = function() {
  return maxYear;
};

/**
 * Get the list of years available in the data set
 * @returns {Array} List of years, sorted
 */
window.DataLoader.getYearsList = function() {
  return [...yearsList];
};

/**
 * Check if data is loaded
 * @returns {boolean} True if data is loaded, false otherwise
 */
window.DataLoader.isLoaded = function() {
  return window.DataLoader.state.isLoaded;
};

/**
 * Gets the available year range
 * @returns {Object} Object with startYear and endYear properties
 */
window.DataLoader.getYearRange = function() {
  return {
    startYear: window.DataLoader.state.startYear,
    endYear: window.DataLoader.state.endYear
  };
};

/**
 * Load data from CSV files when explicitly requested by the user
 * This function should be called in response to a user action (like clicking a button)
 * @returns {Promise} Promise that resolves with loaded data
 */
window.DataLoader.loadCsvData = function() {
  log('Initiating CSV data loading');
  
  return new Promise((resolve, reject) => {
    // Check if Papa Parse is available
    if (typeof Papa === 'undefined') {
      log('Papa Parse library not available', 'error');
      reject(new Error('CSV parsing library not available'));
      return;
    }

    // Create a file input element to allow user to select CSV files
    const fileInput = document.createElement('input');
    fileInput.type = 'file';
    fileInput.accept = '.csv';
    fileInput.multiple = true;
    fileInput.style.display = 'none';
    document.body.appendChild(fileInput);

    // Set a timeout to automatically cancel if user doesn't select files
    const timeoutId = setTimeout(() => {
      document.body.removeChild(fileInput);
      reject(new Error('CSV file selection timed out'));
    }, 60000); // 1 minute timeout

    fileInput.onchange = function(event) {
      clearTimeout(timeoutId);
      const files = event.target.files;
      
      if (!files || files.length === 0) {
        document.body.removeChild(fileInput);
        reject(new Error('No CSV files selected'));
        return;
      }

      log(`${files.length} CSV files selected, processing...`);
      
      // Process each file
      const processedData = {};
      let filesProcessed = 0;
      
      Array.from(files).forEach(file => {
        log(`Parsing CSV file: ${file.name}`);
        
        Papa.parse(file, {
          header: true,
          dynamicTyping: true,
          skipEmptyLines: true,
          complete: function(results) {
            if (results.errors && results.errors.length > 0) {
              log(`Error parsing ${file.name}: ${results.errors[0].message}`, 'error');
            } else {
              log(`Successfully parsed ${file.name}: ${results.data.length} rows`);
              
              // Process the CSV data
              processFileData(file.name, results.data, processedData);
            }
            
            // Check if all files have been processed
            filesProcessed++;
            if (filesProcessed === files.length) {
              document.body.removeChild(fileInput);
              
              if (Object.keys(processedData).length > 0) {
                log(`All CSV files processed, data for ${Object.keys(processedData).length} years loaded`);
                // Update the application data
                window.DataLoader.state.data = processedData;
                window.DataLoader.state.isLoaded = true;
                
                // Process the loaded data
                processLoadedData();
                
                // Trigger a refresh of the UI
                if (window.App && typeof window.App.refreshData === 'function') {
                  window.App.refreshData();
                }
                
                resolve(processedData);
              } else {
                reject(new Error('No valid data found in CSV files'));
              }
            }
          },
          error: function(error) {
            log(`Error parsing ${file.name}: ${error.message}`, 'error');
            filesProcessed++;
            
            if (filesProcessed === files.length) {
              document.body.removeChild(fileInput);
              
              if (Object.keys(processedData).length > 0) {
                log(`Some CSV files processed, data for ${Object.keys(processedData).length} years loaded`);
                resolve(processedData);
              } else {
                reject(new Error('No valid data found in CSV files'));
              }
            }
          }
        });
      });
    };
    
    // Click must be done within a user-initiated event handler
    // We'll rely on the caller to invoke this within a proper user event
    fileInput.click();
  });
}; 