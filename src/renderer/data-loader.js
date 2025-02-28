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

/**
 * Preprocesses a simulation summary CSV content to extract the proper sections
 * This function handles the complex structure of the simulation_summary.csv file
 * @param {string} csvText - The raw CSV text
 * @returns {Object} Object with processed sections of the CSV
 */
function preprocessSimulationSummary(csvText) {
  log('Preprocessing simulation summary CSV', 'info');
  
  const lines = csvText.split(/\r\n|\r|\n/);
  log(`Total lines in simulation_summary.csv: ${lines.length}`, 'info');
  
  const sections = {
    metadata: [],
    finalMetrics: [],
    actionsTaken: [],
    yearlySummary: []
  };
  
  let currentSection = 'metadata';
  let yearlySummaryStarted = false;
  let yearlySummaryHeaderFound = false;
  let yearlySummaryHeaderLine = -1;
  
  // Process the file line by line
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    
    // Skip empty lines
    if (!line) continue;
    
    // Identify which section we're in
    if (line.includes('Final Metrics')) {
      currentSection = 'finalMetrics';
      log(`Found Final Metrics section at line ${i}`, 'debug');
      continue;
    } else if (line.includes('Actions Taken')) {
      currentSection = 'actionsTaken';
      log(`Found Actions Taken section at line ${i}`, 'debug');
      continue;
    } else if (line.includes('Yearly Summary Metrics')) {
      currentSection = 'yearlySummary';
      yearlySummaryStarted = true;
      yearlySummaryHeaderFound = false; // Reset this flag as we just found the section header
      log(`Found Yearly Summary Metrics section at line ${i}`, 'debug');
      continue; // Skip the section header
    }
    
    // If we're in the yearly summary section
    if (yearlySummaryStarted) {
      // Skip any empty lines after the "Yearly Summary Metrics" header
      if (!line.trim()) continue;
      
      // This should be the header row with column names
      if (!yearlySummaryHeaderFound) {
        yearlySummaryHeaderFound = true;
        yearlySummaryHeaderLine = i;
        sections.yearlySummary = [line]; // Initialize with just the header line
        log(`Found Yearly Summary header at line ${i}: ${line}`, 'debug');
      } else {
        // Add data rows - make sure they have same number of commas as header
        const headerCommas = (sections.yearlySummary[0].match(/,/g) || []).length;
        let rowLine = line;
        
        // Make sure the row has at least the same number of fields as the header
        const rowCommas = (line.match(/,/g) || []).length;
        if (rowCommas < headerCommas) {
          rowLine = line + ','.repeat(headerCommas - rowCommas);
          log(`Fixed row with missing fields: added ${headerCommas - rowCommas} commas to line ${i}`, 'debug');
        }
        
        sections.yearlySummary.push(rowLine);
      }
    } else {
      // Add the line to the current section
      sections[currentSection].push(line);
    }
  }
  
  // Check if we found the yearly summary section
  if (yearlySummaryHeaderFound) {
    log(`Extracted yearly summary section with ${sections.yearlySummary.length} lines starting at line ${yearlySummaryHeaderLine}`, 'info');
    
    // Convert the yearly summary section back to CSV text for proper parsing
    const yearlySummaryCSV = sections.yearlySummary.join('\n');
    
    if (sections.yearlySummary.length > 1) {
      // Log the first few rows for debugging
      const rowsToLog = Math.min(5, sections.yearlySummary.length);
      log(`First ${rowsToLog} rows of yearly summary section:`, 'debug');
      for (let i = 0; i < rowsToLog; i++) {
        log(`Row ${i}: ${sections.yearlySummary[i]}`, 'debug');
      }
    } else {
      log('Warning: Only header row found in yearly summary section, no data rows', 'warn');
    }
    
    return {
      rawSections: sections,
      yearlySummaryCSV: yearlySummaryCSV
    };
  } else {
    log('No yearly summary section header row found in the CSV file', 'error');
    
    // Log the first few lines of the file to help diagnose the issue
    log('First 10 lines of the CSV file:', 'debug');
    for (let i = 0; i < Math.min(10, lines.length); i++) {
      log(`Line ${i}: ${lines[i]}`, 'debug');
    }
    
    // Return an empty CSV as fallback
    return {
      rawSections: sections,
      yearlySummaryCSV: "Year,Population,PowerUsage,PowerGeneration\n"
    };
  }
}

// Initialize the module
window.DataLoader.state = {
  data: null,
  startYear: 2025,
  endYear: 2050,
  currentYear: 2025,
  isLoaded: false,
  loadPromise: null
};

// Ireland's approximate bounding box for coordinate generation
const irelandBoundingBox = {
  north: 55.4,
  south: 51.4,
  west: -10.5,
  east: -6.0
};

// Cache to store generator coordinates by ID
const generatorCoordinatesCache = {};

/**
 * Generate random coordinates within Ireland's bounding box
 * @returns {Object} Object with lat and lng properties
 */
function generateRandomIrishCoordinates() {
  const lat = Math.random() * (irelandBoundingBox.north - irelandBoundingBox.south) + irelandBoundingBox.south;
  const lng = Math.random() * (irelandBoundingBox.east - irelandBoundingBox.west) + irelandBoundingBox.west;
  return { lat, lng };
}

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
  
  // Add debug logging for first year data
  const sampleYear = Object.keys(window.DataLoader.state.data)[0];
  if (sampleYear) {
    const sampleData = window.DataLoader.state.data[sampleYear];
    log(`Sample year data (${sampleYear}) keys: ${Object.keys(sampleData).join(', ')}`, 'debug');
    
    if (sampleData.summaryMetrics) {
      log(`Year ${sampleYear} has summaryMetrics with keys: ${Object.keys(sampleData.summaryMetrics).join(', ')}`, 'debug');
    } else {
      log(`Year ${sampleYear} has NO summaryMetrics!`, 'warn');
    }
  }
  
  // Extract years and sort them
  yearsList = Object.keys(window.DataLoader.state.data).map(Number).sort((a, b) => a - b);
  
  if (yearsList.length > 0) {
    minYear = yearsList[0];
    maxYear = yearsList[yearsList.length - 1];
    
    window.DataLoader.state.startYear = minYear;
    window.DataLoader.state.endYear = maxYear;
    window.DataLoader.state.currentYear = minYear;
    
    // Make sure each year's data has the year property set
    yearsList.forEach(year => {
      if (window.DataLoader.state.data[year]) {
        // Explicitly add the year to the data object for this year
        window.DataLoader.state.data[year].year = Number(year);
        
        // Ensure all objects have lat/lng properly set as numbers
        if (window.DataLoader.state.data[year].settlements) {
          window.DataLoader.state.data[year].settlements.forEach(settlement => {
            settlement.lat = Number(settlement.lat) || 0;
            settlement.lng = Number(settlement.lng) || 0;
          });
        }
        
        if (window.DataLoader.state.data[year].generators) {
          window.DataLoader.state.data[year].generators.forEach(generator => {
            generator.lat = Number(generator.lat) || 0;
            generator.lng = Number(generator.lng) || 0;
          });
        }
        
        if (window.DataLoader.state.data[year].carbonOffsets) {
          window.DataLoader.state.data[year].carbonOffsets.forEach(offset => {
            offset.lat = Number(offset.lat) || 0;
            offset.lng = Number(offset.lng) || 0;
          });
        }
        
        // Ensure summaryMetrics object exists for each year
        if (!window.DataLoader.state.data[year].summaryMetrics) {
          log(`Creating empty summaryMetrics for year ${year}`, 'warn');
          window.DataLoader.state.data[year].summaryMetrics = {};
        } else {
          log(`Found existing summaryMetrics for year ${year} with ${Object.keys(window.DataLoader.state.data[year].summaryMetrics).length} metrics`, 'debug');
        }
      }
    });
    
    log(`Data processed: years range from ${minYear} to ${maxYear}, current year set to ${minYear}`);
  } else {
    log('No valid years found in the data', 'warn');
  }
  
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
      { url: `${baseUrl}carbon_offsets.csv`, type: 'offsets' },
      { url: `${baseUrl}simulation_summary.csv`, type: 'summary' }
    ];
    
    const processedData = {};
    let filesLoaded = 0;
    let hasErrors = false;
    
    filesToLoad.forEach(file => {
      log(`Loading ${file.type} data from ${file.url}`);
      
      // Special handling for simulation_summary.csv
      if (file.type === 'summary') {
        // First, fetch the raw content as text
        fetch(file.url)
          .then(response => {
            if (!response.ok) {
              throw new Error(`Failed to fetch ${file.url}: ${response.status} ${response.statusText}`);
            }
            return response.text();
          })
          .then(csvText => {
            // Preprocess the simulation summary CSV
            log(`Raw simulation_summary.csv content length: ${csvText.length} characters`, 'debug');
            log(`First 100 characters: ${csvText.substring(0, 100)}...`, 'debug');
            
            const preprocessed = preprocessSimulationSummary(csvText);
            
            log(`Yearly summary CSV text length: ${preprocessed.yearlySummaryCSV.length} characters`, 'debug');
            log(`First 100 characters of yearly summary: ${preprocessed.yearlySummaryCSV.substring(0, 100)}...`, 'debug');
            
            log('Parsing yearly summary section with Papa Parse', 'info');
            
            // Parse only the yearly summary section as a CSV
            if (preprocessed.yearlySummaryCSV) {
              // Log the first few lines of the CSV for debugging
              const previewLines = preprocessed.yearlySummaryCSV.split('\n').slice(0, 5);
              log(`CSV Preview (first ${previewLines.length} lines):`, 'debug');
              previewLines.forEach((line, idx) => {
                log(`Line ${idx + 1}: ${line}`, 'debug');
              });
              
              // Instead of just processing the parsed results, let's directly handle the file
              // by adding it to the processed results as is, for debugging
              const summaryLines = preprocessed.yearlySummaryCSV.split('\n');
              log(`Total lines in yearly summary section: ${summaryLines.length}`, 'debug');
              
              // If we have at least a header and some data rows
              if (summaryLines.length >= 2) {
                log(`Directly processing yearly summary CSV (bypassing PapaParse for debugging)`, 'debug');
                
                // Assume first line is header
                const header = summaryLines[0].split(',').map(h => h.trim());
                log(`Header fields: ${header.join(', ')}`, 'debug');
                
                // Process manually
                const dataRows = [];
                for (let i = 1; i < summaryLines.length; i++) {
                  if (!summaryLines[i].trim()) continue;
                  
                  const values = summaryLines[i].split(',').map(v => v.trim());
                  const row = {};
                  
                  header.forEach((field, idx) => {
                    if (idx < values.length) {
                      row[field] = values[idx];
                    }
                  });
                  
                  dataRows.push(row);
                }
                
                log(`Manually parsed ${dataRows.length} data rows`, 'debug');
                
                // Process the manually parsed data
                if (dataRows.length > 0) {
                  log(`Sample row: ${JSON.stringify(dataRows[0])}`, 'debug');
                  processSummaryData(dataRows, processedData);
                }
              }
              
              // Also still use PapaParse for the proper parsing
              Papa.parse(preprocessed.yearlySummaryCSV, {
                header: true,
                dynamicTyping: true,
                skipEmptyLines: true,
                trimHeaders: true,
                comments: "#",
                delimiter: ",",
                quoteChar: '"',
                escapeChar: '"',
                transformHeader: function(header) {
                  return header.trim();
                },
                transform: function(value, field) {
                  if (value === undefined || value === null) return "";
                  return value;
                },
                complete: function(results) {
                  if (results.errors && results.errors.length > 0) {
                    log(`Error parsing yearly summary section: ${results.errors[0].message}`, 'error');
                    results.errors.forEach((err, index) => {
                      log(`CSV Parse Error #${index + 1}: ${err.message} at row ${err.row || 'unknown'}`, 'error');
                    });
                  } else {
                    log(`Successfully parsed yearly summary section: ${results.data.length} rows`, 'info');
                    
                    // Check if we got data with the expected structure
                    if (results.data.length > 0) {
                      const sampleRow = results.data[0];
                      log(`Sample row has ${Object.keys(sampleRow).length} fields`, 'debug');
                      log(`Sample row fields: ${Object.keys(sampleRow).join(', ')}`, 'debug');
                      log(`Sample row data: ${JSON.stringify(sampleRow)}`, 'debug');
                      
                      // Process the data
                      processSummaryData(results.data, processedData);
                    } else {
                      log('No data rows found in yearly summary section', 'warn');
                    }
                  }
                  
                  filesLoaded++;
                  if (filesLoaded === filesToLoad.length) {
                    finishLoading();
                  }
                },
                error: function(error) {
                  log(`Error parsing yearly summary section: ${error.message}`, 'error');
                  hasErrors = true;
                  filesLoaded++;
                  if (filesLoaded === filesToLoad.length) {
                    finishLoading();
                  }
                }
              });
            } else {
              log('No yearly summary section found in simulation_summary.csv', 'error');
              hasErrors = true;
              filesLoaded++;
              if (filesLoaded === filesToLoad.length) {
                finishLoading();
              }
            }
          })
          .catch(error => {
            log(`Error fetching simulation_summary.csv: ${error.message}`, 'error');
            hasErrors = true;
            filesLoaded++;
            if (filesLoaded === filesToLoad.length) {
              finishLoading();
            }
          });
      } else {
        // Regular handling for other CSV files
        Papa.parse(file.url, {
          download: true,
          header: true,
          dynamicTyping: true,
          skipEmptyLines: true,
          comments: "#",  // Allow comments in CSV
          delimiter: ",",
          quoteChar: '"',
          escapeChar: '"',
          beforeFirstChunk: function(chunk) {
            if (file.type === 'settlements') {
              log('Preprocessing settlement CSV data to fix field count issues', 'info');
              
              const lines = chunk.split(/\r\n|\r|\n/);
              
              if (lines.length <= 1) {
                return chunk;
              }
              
              const headerLine = lines[0];
              const expectedFieldCount = (headerLine.match(/,/g) || []).length + 1;
              
              for (let i = 1; i < lines.length; i++) {
                const line = lines[i].trim();
                if (!line) continue;
                
                const fieldCount = (line.match(/,/g) || []).length + 1;
                
                if (fieldCount < expectedFieldCount) {
                  const missingCommas = expectedFieldCount - fieldCount;
                  lines[i] = line + ','.repeat(missingCommas);
                  if (i < 5 || i % 100 === 0) {
                    log(`Fixed row ${i}: Added ${missingCommas} missing fields`, 'debug');
                  }
                }
              }
              
              return lines.join('\n');
            }
            
            return chunk;
          },
          transformHeader: function(header) {
            return header.trim();
          },
          transform: function(value, field) {
            return value === undefined || value === null ? "" : value;
          },
          error: function(error) {
            log(`Error loading ${file.url}: ${error.message}`, 'error');
            hasErrors = true;
            
            filesLoaded++;
            if (filesLoaded === filesToLoad.length) {
              finishLoading();
            }
          },
          complete: function(results) {
            let processedSuccessfully = true;
            
            if (results.errors && results.errors.length > 0) {
              // Log the error, but try to process what data we have
              log(`Error parsing ${file.url}: ${results.errors[0].message}`, 'error');
              // Log all errors for better debugging
              results.errors.forEach((err, index) => {
                log(`CSV Parse Error #${index + 1}: ${err.message} at row ${err.row || 'unknown'}`, 'error');
              });
              
              // Analyze CSV problems
              analyzeCSVProblems(results.data, results.errors, file.type);
              
              log(`Attempting to process partial data from ${file.url}`, 'warn');
              
              if (!results.data || results.data.length === 0) {
                processedSuccessfully = false;
              }
            } else {
              log(`Successfully parsed ${file.url}: ${results.data.length} rows`);
            }
            
            // Try to process the CSV data even if there were errors
            if (processedSuccessfully && results.data && results.data.length > 0) {
              try {
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
              } catch (processingError) {
                log(`Error processing ${file.url}: ${processingError.message}`, 'error');
                hasErrors = true;
              }
            } else {
              log(`Could not process ${file.url} due to parsing errors`, 'error');
              hasErrors = true;
            }
            
            filesLoaded++;
            if (filesLoaded === filesToLoad.length) {
              finishLoading();
            }
          }
        });
      }
    });
    
    function finishLoading() {
      if (Object.keys(processedData).length > 0) {
        const yearsCount = Object.keys(processedData).length;
        log(`Data loading complete: ${yearsCount} years of data loaded` + 
            (hasErrors ? ' (with some errors)' : ''));
              
        // Verify data structure for each year
        Object.keys(processedData).forEach(year => {
          const yearData = processedData[year];
          if (!yearData.settlements) yearData.settlements = [];
          if (!yearData.generators) yearData.generators = [];
          if (!yearData.carbonOffsets) yearData.carbonOffsets = [];
        });
        
        resolve(processedData);
      } else {
        reject(new Error('No valid data found in CSV files'));
      }
    }
  });
}

/**
 * Analyze CSV file for problems
 * @param {Array} data - The parsed CSV data
 * @param {Array} errors - Array of Papa Parse errors
 * @param {string} fileType - Type of file being analyzed
 */
function analyzeCSVProblems(data, errors, fileType) {
  if (!errors || errors.length === 0) return;
  
  log(`Analyzing ${fileType} CSV file for problems...`, 'info');
  log(`Found ${errors.length} errors in ${fileType} file`, 'warn');
  
  // Group errors by type
  const errorsByType = {};
  errors.forEach(err => {
    const type = err.message.split(':')[0];
    if (!errorsByType[type]) errorsByType[type] = [];
    errorsByType[type].push(err);
  });
  
  // Log summary of error types
  Object.keys(errorsByType).forEach(type => {
    log(`${type}: ${errorsByType[type].length} occurrences`, 'warn');
  });
  
  // Sample a few problematic rows
  if (data && data.length > 0) {
    const firstRow = data[0];
    const expectedFields = Object.keys(firstRow);
    
    log(`Expected fields (${expectedFields.length}): ${expectedFields.join(', ')}`, 'info');
    
    // Find some problematic rows to analyze
    const problemRows = [];
    errors.forEach(err => {
      if (err.row !== undefined && data[err.row]) {
        problemRows.push({ row: err.row, data: data[err.row] });
      }
    });
    
    // Log a sample of problematic rows
    if (problemRows.length > 0) {
      log(`Sampling ${Math.min(3, problemRows.length)} problematic rows for analysis:`, 'info');
      problemRows.slice(0, 3).forEach(problem => {
        const fields = Object.keys(problem.data);
        log(`Row ${problem.row} has ${fields.length} fields: ${fields.join(', ')}`, 'warn');
      });
    }
  }
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
  
  log(`Processing file ${fileName} with ${data.length} rows`, 'info');
  
  // Determine what type of data this file contains based on filename or content
  if (fileName.toLowerCase().includes('settlement')) {
    processSettlementData(data, processedData);
  } else if (fileName.toLowerCase().includes('generator')) {
    processGeneratorData(data, processedData);
  } else if (fileName.toLowerCase().includes('offset') || fileName.toLowerCase().includes('carbon')) {
    processCarbonOffsetData(data, processedData);
  } else if (fileName.toLowerCase().includes('summary')) {
    // Explicitly handle summary file
    log(`Processing summary data file: ${fileName}`, 'info');
    processSummaryData(data, processedData);
  } else {
    // Try to determine type from content
    const firstRow = data[0];
    if (firstRow.hasOwnProperty('population') || firstRow.hasOwnProperty('Population')) {
      processSettlementData(data, processedData);
    } else if (firstRow.hasOwnProperty('output') || firstRow.hasOwnProperty('type') || 
               firstRow.hasOwnProperty('Power Output (MW)') || firstRow.hasOwnProperty('Type')) {
      processGeneratorData(data, processedData);
    } else if (firstRow.hasOwnProperty('offsetAmount') || firstRow.hasOwnProperty('CO2 Offset (tonnes)')) {
      processCarbonOffsetData(data, processedData);
    } else if (firstRow.hasOwnProperty('Year') && 
              (firstRow.hasOwnProperty('PowerUsage') || firstRow.hasOwnProperty('Population') || 
               firstRow.hasOwnProperty('CO2Emissions') || firstRow.hasOwnProperty('NetEmissions'))) {
      log(`Identified ${fileName} as summary data based on field names`, 'info');
      processSummaryData(data, processedData);
    } else {
      log(`Could not determine data type for ${fileName}`, 'warn');
      log(`Available fields in first row: ${Object.keys(firstRow).join(', ')}`, 'debug');
    }
  }
}

/**
 * Process settlement data from CSV
 * @param {Array} data - Array of parsed CSV rows
 * @param {Object} processedData - Object to store processed data
 */
function processSettlementData(data, processedData) {
  if (!data || data.length === 0) {
    log('No settlement data found', 'warn');
    return;
  }

  let hasProcessedAny = false;
  
  // Log more detailed information about the fields we're working with
  if (data.length > 0) {
    const firstRow = data[0];
    const availableFields = Object.keys(firstRow);
    log(`Settlement data contains ${availableFields.length} fields: ${availableFields.join(', ')}`, 'info');
    log('Processing settlement data (ignoring Power Usage and Power Per Capita fields)', 'info');
  }
  
  data.forEach(row => {
    if (!row.Year) {
      log('Skipping settlement row without Year value', 'debug');
      return;
    }
    
    const year = Number(row.Year);
    if (!processedData[year]) {
      processedData[year] = {
        settlements: [],
        generators: [],
        carbonOffsets: []
      };
    }
    
    // Create a more robust settlement object with fallbacks for missing fields
    // Note: We intentionally omit Power Usage (MW) and Power Usage Per Capita (kW) fields
    const lng = parseFloat(row.X) || parseFloat(row.Longitude) || 0;
    const lat = parseFloat(row.Y) || parseFloat(row.Latitude) || 0;
    
    // Log the coordinates that we're reading
    // log(`Settlement ${row['Settlement ID'] || row.Name || 'Unknown'}: Latitude=${lat}, Longitude=${lng} (from Y=${row.Y}, X=${row.X}, Lat=${row.Latitude}, Long=${row.Longitude})`, 'debug');
    
    const settlement = {
      id: row['Settlement ID'] || `Settlement_${Math.random().toString(36).substring(2, 10)}`,
      name: row.Name || `Unknown Settlement ${Math.random().toString(36).substring(2, 7)}`,
      lat: lat,
      lng: lng,
      population: parseFloat(row.Population) || 0,
      growthRate: parseFloat(row['Growth Rate (%)']) || 0
      // We are intentionally omitting these fields:
      // powerUsage: parseFloat(row['Power Usage (MW)']) || parseFloat(row['PowerUsage']) || 0,
      // powerPerCapita: parseFloat(row['Power Usage Per Capita (kW)']) || parseFloat(row['PowerPerCapita']) || 0
    };
    
    processedData[year].settlements.push(settlement);
    hasProcessedAny = true;
  });
  
  if (hasProcessedAny) {
    log(`Processed settlement data for ${Object.keys(processedData).length} years`);
  } else {
    log('Failed to process any settlement data - check CSV format', 'warn');
  }
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
    
    // Get or generate generator ID
    const generatorId = row['Generator ID'] || `Generator_${Math.random().toString(36).substring(2, 10)}`;
    
    // Get coordinates from cache or generate new ones
    if (!generatorCoordinatesCache[generatorId]) {
      // Generate random coordinates within Ireland and cache them for this generator
      generatorCoordinatesCache[generatorId] = generateRandomIrishCoordinates();
      log(`Generated random Irish coordinates for generator ${generatorId}: ${generatorCoordinatesCache[generatorId].lat}, ${generatorCoordinatesCache[generatorId].lng}`, 'debug');
    }
    
    // Use the cached coordinates
    const { lat, lng } = generatorCoordinatesCache[generatorId];
    
    // Log the coordinates that we're using
    log(`Generator ${generatorId}: Using coordinates Latitude=${lat}, Longitude=${lng}`, 'debug');

    processedData[year].generators.push({
      id: generatorId,
      name: row.Type ? `${row.Type} Generator ${generatorId}` : `Generator ${generatorId}`,
      type: row.Type || 'unknown',
      lat: lat,
      lng: lng,
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
  if (!data || data.length === 0) {
    log('No carbon offset data found', 'warn');
    return;
  }

  // Log more detailed information about the fields we're working with
  if (data.length > 0) {
    const firstRow = data[0];
    const availableFields = Object.keys(firstRow);
    log(`Carbon offset data contains ${availableFields.length} fields: ${availableFields.join(', ')}`, 'info');
    log(`Sample data for first row: CO2 Offset (tonnes)=${firstRow['CO2 Offset (tonnes)']}`, 'info');
  }

  let processedCount = 0;
  data.forEach(row => {
    if (!row.Year) {
      log('Skipping carbon offset row without Year value', 'debug');
      return;
    }
    
    const year = Number(row.Year);
    if (!processedData[year]) {
      processedData[year] = {
        settlements: [],
        generators: [],
        carbonOffsets: []
      };
    }

    // Correctly handle X/Y as lng/lat
    const lng = parseFloat(row.X) || parseFloat(row.Longitude) || 0;
    const lat = parseFloat(row.Y) || parseFloat(row.Latitude) || 0;
    
    // Log the coordinates that we're reading
    log(`Carbon Offset ${row['Offset ID'] || 'Unknown'}: Latitude=${lat}, Longitude=${lng} (from Y=${row.Y}, X=${row.X}, Lat=${row.Latitude}, Long=${row.Longitude})`, 'debug');
    
    // Get the offset amount and ensure it's a number
    const offsetAmount = parseFloat(row['CO2 Offset (tonnes)']) || 0;
    
    // Log the offset amount for debugging
    log(`Carbon Offset ${row['Offset ID'] || 'Unknown'}: CO2 Offset Amount=${offsetAmount} tonnes (from CSV value: ${row['CO2 Offset (tonnes)']})`, 'debug');
    
    processedData[year].carbonOffsets.push({
      id: row['Offset ID'] || `Offset_${Math.random().toString(36).substring(2, 10)}`,
      name: row.Type ? `${row.Type} Offset ${row['Offset ID']}` : `Offset ${row['Offset ID']}`,
      type: row.Type || 'unknown',
      lat: lat,
      lng: lng,
      offsetAmount: offsetAmount,
      negativeEmissions: Number(row['Negative CO2 Emissions (tonnes)']) || 0,
      size: Number(row.Size) || 0,
      yearStarted: year,
      powerConsumption: Number(row['Power Consumption (MW)']) || 0,
      captureEfficiency: Number(row['Capture Efficiency (%)']) || 0
    });
    processedCount++;
  });
  
  log(`Processed ${processedCount} carbon offset entries for ${Object.keys(processedData).length} years`);
}

/**
 * Process summary data from CSV
 * @param {Array} data - Array of parsed CSV rows
 * @param {Object} processedData - Object to store processed data
 */
function processSummaryData(data, processedData) {
  if (!data || data.length === 0) {
    log('No summary data found', 'warn');
    return;
  }

  log(`Processing summary data: ${data.length} rows`, 'info');
  
  // Log field names for debugging
  if (data.length > 0) {
    const firstRow = data[0];
    const availableFields = Object.keys(firstRow);
    log(`Summary data contains ${availableFields.length} fields: ${availableFields.join(', ')}`, 'info');
    log(`First row sample data: ${JSON.stringify(firstRow).substring(0, 300)}...`, 'debug');
  }
  
  let processedCount = 0;
  let yearProcessed = new Set();
  
  // Process each row in the yearly summary section
  data.forEach((row, index) => {
    log(`Processing summary row ${index + 1}`, 'debug');
    
    // First, normalize field names (case-insensitive and remove spaces)
    const normalizedRow = {};
    
    Object.keys(row).forEach(key => {
      // Skip empty keys
      if (!key || key.trim() === '') return;
      
      const value = row[key];
      
      // Store the original key-value pair
      normalizedRow[key] = value;
      
      // Add lowercase version
      normalizedRow[key.toLowerCase()] = value;
      
      // Add version without spaces
      normalizedRow[key.replace(/\s+/g, '')] = value;
      
      // Add lowercase version without spaces
      normalizedRow[key.toLowerCase().replace(/\s+/g, '')] = value;
    });
    
    // Try to find the year in the row using different possible key names
    const possibleYearKeys = ['Year', 'year', 'YEAR', 'yr', 'YR', 'Yr'];
    let year = null;
    
    for (const keyName of possibleYearKeys) {
      if (normalizedRow[keyName] !== undefined && normalizedRow[keyName] !== null) {
        const yearValue = Number(normalizedRow[keyName]);
        if (!isNaN(yearValue) && yearValue > 2000 && yearValue < 2100) { // Basic validation
          year = yearValue;
          break;
        }
      }
    }
    
    if (year === null) {
      log(`Skipping summary row ${index + 1} - no valid year found`, 'debug');
      return;
    }
    
    log(`Processing summary metrics for year ${year} (row ${index + 1})`, 'debug');
    
    // Initialize the year data structure if not exists
    if (!processedData[year]) {
      log(`Creating new data structure for year ${year}`, 'debug');
      processedData[year] = {
        settlements: [],
        generators: [],
        carbonOffsets: [],
        summaryMetrics: {}
      };
    } else if (!processedData[year].summaryMetrics) {
      log(`Creating summaryMetrics object for existing year ${year}`, 'debug');
      processedData[year].summaryMetrics = {};
    }
    
    // Helper function to extract a numeric value from various possible field names
    function extractMetric(baseName, defaultValue = 0) {
      // Generate variations of the field name
      const variations = [
        baseName,
        baseName.toLowerCase(),
        baseName.toUpperCase(),
        baseName.replace(/\s+/g, ''),
        baseName.toLowerCase().replace(/\s+/g, ''),
        baseName.toUpperCase().replace(/\s+/g, '')
      ];
      
      // Add more variations with spaces
      if (!baseName.includes(' ')) {
        // Add camel case to spaced version (e.g. powerUsage -> Power Usage)
        const spacedName = baseName.replace(/([A-Z])/g, ' $1').trim();
        variations.push(spacedName);
        variations.push(spacedName.toLowerCase());
        variations.push(spacedName.toUpperCase());
      }
      
      // Try each variation
      for (const name of variations) {
        if (normalizedRow[name] !== undefined && normalizedRow[name] !== null) {
          const value = Number(normalizedRow[name]);
          return isNaN(value) ? defaultValue : value;
        }
      }
      
      return defaultValue;
    }
    
    // Extract all metrics we're interested in
    const metrics = {
      population: extractMetric('Population'),
      powerUsage: extractMetric('PowerUsage'),
      powerGeneration: extractMetric('PowerGeneration'),
      powerBalance: extractMetric('PowerBalance'),
      publicOpinion: extractMetric('PublicOpinion'),
      co2Emissions: extractMetric('CO2Emissions'),
      carbonOffset: extractMetric('CarbonOffset'),
      netEmissions: extractMetric('NetEmissions'),
      yearlyCapitalCost: extractMetric('YearlyCapitalCost'),
      totalCapitalCost: extractMetric('TotalCapitalCost'),
      yearlyRevenue: extractMetric('YearlyRevenue'),
      totalRevenue: extractMetric('TotalRevenue'),
      activeGenerators: extractMetric('ActiveGenerators'),
      yearlyUpgradeCosts: extractMetric('YearlyUpgradeCosts'),
      yearlyClosureCosts: extractMetric('YearlyClosureCosts'),
      yearlyTotalCost: extractMetric('YearlyTotalCost'),
      totalCost: extractMetric('TotalCost')
    };
    
    // Check if any metrics were found
    const nonZeroMetrics = Object.keys(metrics).filter(key => metrics[key] !== 0).length;
    
    if (nonZeroMetrics > 0) {
      log(`Found ${nonZeroMetrics} non-zero metrics for year ${year}`, 'debug');
      // Store the metrics in the processed data
      processedData[year].summaryMetrics = metrics;
      yearProcessed.add(year);
      processedCount++;
    } else {
      log(`Warning: All metrics for year ${year} are zero!`, 'warn');
      log(`Row data: ${JSON.stringify(normalizedRow)}`, 'debug');
    }
  });
  
  log(`Successfully processed summary metrics for ${processedCount} rows (${yearProcessed.size} unique years)`);
  
  // Check if we processed metrics for all years
  const allYears = Object.keys(processedData);
  log(`Available years in data: ${allYears.join(', ')}`, 'debug');
  
  const missingYears = allYears.filter(year => !yearProcessed.has(Number(year)));
  if (missingYears.length > 0) {
    log(`Warning: Some years are missing summary metrics: ${missingYears.join(', ')}`, 'warn');
  }
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
  
  // Convert to number if string was passed
  const yearNum = Number(year);
  
  if (!window.DataLoader.state.data[yearNum]) {
    log(`No data available for year ${yearNum}`, 'warn');
    return null;
  }
  
  log(`Retrieving data for year ${yearNum}`);
  return window.DataLoader.state.data[yearNum];
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
          comments: "#",  // Allow comments in CSV
          delimiter: ",",
          quoteChar: '"',
          escapeChar: '"',
          beforeFirstChunk: function(chunk) {
            if (file.name.toLowerCase().includes('settlement')) {
              log('Preprocessing settlement CSV data to fix field count issues', 'info');
              
              const lines = chunk.split(/\r\n|\r|\n/);
              
              if (lines.length <= 1) {
                return chunk;
              }
              
              const headerLine = lines[0];
              const expectedFieldCount = (headerLine.match(/,/g) || []).length + 1;
              
              for (let i = 1; i < lines.length; i++) {
                const line = lines[i].trim();
                if (!line) continue;
                
                const fieldCount = (line.match(/,/g) || []).length + 1;
                
                if (fieldCount < expectedFieldCount) {
                  const missingCommas = expectedFieldCount - fieldCount;
                  lines[i] = line + ','.repeat(missingCommas);
                  if (i < 5 || i % 100 === 0) {
                    log(`Fixed row ${i}: Added ${missingCommas} missing fields`, 'debug');
                  }
                }
              }
              
              return lines.join('\n');
            }
            
            return chunk;
          },
          transformHeader: function(header) {
            return header.trim();
          },
          transform: function(value, field) {
            return value === undefined || value === null ? "" : value;
          },
          complete: function(results) {
            if (results.errors && results.errors.length > 0) {
              log(`Error parsing ${file.name}: ${results.errors[0].message}`, 'error');
              // Log all errors for better debugging
              results.errors.forEach((err, index) => {
                log(`CSV Parse Error #${index + 1}: ${err.message} at row ${err.row || 'unknown'}`, 'error');
              });
              
              // Analyze CSV problems
              analyzeCSVProblems(results.data, results.errors, file.name.toLowerCase().includes('settlement') ? 'settlements' : 
                                             file.name.toLowerCase().includes('generator') ? 'generators' : 
                                             file.name.toLowerCase().includes('offset') ? 'offsets' : 'unknown');
              
              // Still try to process whatever data we have
              if (results.data && results.data.length > 0) {
                log(`Attempting to process partial data from ${file.name} (${results.data.length} rows)`, 'warn');
                processFileData(file.name, results.data, processedData);
              }
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