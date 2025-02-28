/**
 * Grid Coordinates Module
 * 
 * This module handles marker sizing and related grid utilities
 */

// Helper function to log from this module
function log(message, level = 'info') {
  if (window.AppLog) {
    window.AppLog('GridCoordinates', message, level);
  } else {
    console.log(`[GridCoordinates] ${message}`);
  }
}

// Ensure GridCoordinates exists as a global object
window.GridCoordinates = window.GridCoordinates || {};

// Define constants for the Irish grid
window.GridCoordinates.IRELAND_BOUNDS = [
    -10.67, 51.33, // Southwest
    -5.34, 55.41   // Northeast
];

// Log initialization of the module
log('Grid coordinates module initialized with Ireland bounds: SW: [-10.67, 51.33], NE: [-5.34, 55.41]');

/**
 * Calculate appropriate marker radius based on value and type
 * @param {number} value - Value to scale marker by (e.g., population, power output)
 * @param {string} type - Type of marker (settlement, generator, or offset)
 * @param {number} zoom - Current map zoom level
 * @returns {number} - Radius in pixels
 */
window.GridCoordinates.calculateMarkerRadius = function(value, type, zoom) {
    const minRadius = 3;
    const maxRadius = 25;
    let scaleFactor = 1;
    
    // Make sure value is a valid number
    value = Number(value);
    if (isNaN(value) || value <= 0) {
        log(`Invalid value for marker radius calculation: ${value}, using default minimum radius`, 'warn');
        return minRadius;
    }
    
    // Scale factor based on marker type
    switch (type.toLowerCase()) {
        case 'settlement':
            // Population-based scaling, logarithmic, with better handling for small values
            scaleFactor = value < 1000 ? 1 : Math.log(value / 1000) * 1.5;
            // Ensure scale factor is reasonable
            if (scaleFactor < 0) scaleFactor = 1;
            break;
        case 'generator':
            // Power output-based scaling (MWh)
            scaleFactor = Math.log(value + 1) * 1.2;
            // Ensure scale factor is reasonable
            if (scaleFactor < 0) scaleFactor = 1;
            break;
        case 'offset':
            // CO2 offset-based scaling (tonnes)
            scaleFactor = Math.log(value + 1) * 0.5;
            // Ensure scale factor is reasonable
            if (scaleFactor < 0) scaleFactor = 1;
            break;
        default:
            log(`Unknown marker type: ${type}, using default scale factor`, 'warn');
            scaleFactor = 1;
    }
    
    // Adjust for zoom level
    const zoomFactor = Math.pow(1.2, zoom - 7); // 7 is our default zoom
    
    // Calculate radius with constraints
    const radius = Math.max(minRadius, Math.min(maxRadius, scaleFactor * zoomFactor));
    
    log(`Calculated radius for ${type}: ${radius.toFixed(2)}px (scale factor: ${scaleFactor.toFixed(2)}, zoom factor: ${zoomFactor.toFixed(2)})`, 'debug');
    
    return radius;
}; 