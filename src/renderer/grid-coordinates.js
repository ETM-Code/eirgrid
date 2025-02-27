/**
 * Grid Coordinates Module
 * 
 * This module handles coordinate conversions between Irish grid coordinates and lat/lng
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
 * Irish grid to WGS84 conversion
 * @param {number} x - Grid x coordinate (east-west)
 * @param {number} y - Grid y coordinate (north-south)
 * @returns {Object} - {lat, lng}
 */
window.GridCoordinates.gridToLatLng = function(x, y) {
    log(`Converting grid coordinates (${x}, ${y}) to lat/lng`, 'debug');
    
    // Simple linear mapping for sample purposes
    // In a real implementation, this would use proper Irish Grid coordinate system
    const bounds = window.GridCoordinates.IRELAND_BOUNDS;
    
    // Calculate normalized position (0-1) in the grid
    const normX = x / 100; // Assuming grid is 0-100 in x direction
    const normY = y / 100; // Assuming grid is 0-100 in y direction
    
    // Map to lat/lng using linear interpolation
    const lng = bounds[0] + normX * (bounds[2] - bounds[0]);
    const lat = bounds[1] + normY * (bounds[3] - bounds[1]);
    
    log(`Converted to lat/lng: (${lat.toFixed(6)}, ${lng.toFixed(6)})`, 'debug');
    return { lat, lng };
};

/**
 * WGS84 to Irish grid conversion
 * @param {number} lat - Latitude
 * @param {number} lng - Longitude
 * @returns {Object} - {x, y}
 */
window.GridCoordinates.latLngToGrid = function(lat, lng) {
    log(`Converting lat/lng (${lat}, ${lng}) to grid coordinates`, 'debug');
    
    // Simple linear mapping for sample purposes
    // In a real implementation, this would use proper Irish Grid coordinate system
    const bounds = window.GridCoordinates.IRELAND_BOUNDS;
    
    // Check if coordinates are within Ireland bounds
    if (lng < bounds[0] || lng > bounds[2] || lat < bounds[1] || lat > bounds[3]) {
        log('Warning: Coordinates outside Ireland bounds', 'warn');
    }
    
    // Calculate normalized position (0-1) in lat/lng space
    const normLng = (lng - bounds[0]) / (bounds[2] - bounds[0]);
    const normLat = (lat - bounds[1]) / (bounds[3] - bounds[1]);
    
    // Map to grid coordinates using linear interpolation
    const x = Math.floor(normLng * 100); // Assuming grid is 0-100 in x direction
    const y = Math.floor(normLat * 100); // Assuming grid is 0-100 in y direction
    
    log(`Converted to grid: (${x}, ${y})`, 'debug');
    return { x, y };
};

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
    
    if (!value || value < 0) {
        log(`Invalid value for marker radius calculation: ${value}`, 'warn');
        return minRadius;
    }
    
    // Scale factor based on marker type
    switch (type.toLowerCase()) {
        case 'settlement':
            // Population-based scaling, logarithmic
            scaleFactor = Math.log(value / 1000) * 1.5;
            break;
        case 'generator':
            // Power output-based scaling (MWh)
            scaleFactor = Math.log(value + 1) * 1.2;
            break;
        case 'offset':
            // CO2 offset-based scaling (tonnes)
            scaleFactor = Math.log(value) * 0.5;
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