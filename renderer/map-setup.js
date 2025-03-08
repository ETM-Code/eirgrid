/**
 * Map Setup Module
 * 
 * This module initializes the Leaflet map and sets up the base map layers.
 */

// Helper function to log from this module
function log(message, level = 'info') {
  if (window.AppLog) {
    window.AppLog('MapSetup', message, level);
  } else {
    console.log(`[MapSetup] ${message}`);
  }
}

// Ensure MapSetup exists as a global object
window.MapSetup = window.MapSetup || {};

// Internal state
const state = {
  map: null,
  svgOverlay: null,
  currentZoom: 7 // Default initial zoom level
};

/**
 * Initialize the Leaflet map
 * @returns {Promise} Promise that resolves with the map instance
 */
window.MapSetup.init = function() {
  log('Initializing map');
  
  return new Promise((resolve, reject) => {
    try {
      // Create map container if it doesn't exist
      let mapContainer = document.getElementById('map');
      
      if (!mapContainer) {
        log('Map container not found, creating it', 'warn');
        mapContainer = document.createElement('div');
        mapContainer.id = 'map';
        mapContainer.className = 'map-container';
        document.body.appendChild(mapContainer);
      }
      
      // Create the map centered on Ireland
      state.map = L.map('map', {
        center: [53.4, -8.0], // Center of Ireland
        zoom: 7,
        minZoom: 6,
        maxZoom: 12,
        zoomControl: true,
        attributionControl: true,
      });
      
      // Store the current zoom level for marker scaling
      state.currentZoom = state.map.getZoom();
      state.map.on('zoomend', () => {
        state.currentZoom = state.map.getZoom();
        log(`Map zoom changed to ${state.currentZoom}`, 'debug');
      });
      
      // Add base tile layer
      addBaseTileLayer();
      
      // Setup SVG overlay for D3 visualizations
      setupSvgOverlay();
      
      log('Map initialization complete');
      resolve(state.map);
    } catch (error) {
      log(`Error initializing map: ${error.message}`, 'error');
      reject(error);
    }
  });
};

/**
 * Add the base tile layer to the map
 */
function addBaseTileLayer() {
  // Use OpenStreetMap as the base layer
  L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
    attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
    maxZoom: 19,
  }).addTo(state.map);
  
  log('Base tile layer added');
}

/**
 * Set up the SVG overlay for D3 visualizations
 */
function setupSvgOverlay() {
  log('Setting up SVG overlay');
  
  // Create an SVG overlay for D3 visualizations
  state.svgOverlay = L.svg().addTo(state.map);
  
  // Create layer groups for different types of elements
  const svg = d3.select(state.svgOverlay._container);
  // Set z-index to ensure SVG is above map tiles
  svg
    .style('z-index', '650')
    .style('position', 'relative')
    .style('pointer-events', 'auto');
  
  svg.append('g').attr('class', 'settlements-layer');
  svg.append('g').attr('class', 'generators-layer');
  svg.append('g').attr('class', 'offsets-layer');
  
  log('SVG overlay setup complete');
}

/**
 * Convert lat/lng coordinates to map point
 * @param {number} lat - Latitude
 * @param {number} lng - Longitude
 * @returns {Object} Point object with x, y coordinates
 */
window.MapSetup.latLngToMapPoint = function(lat, lng) {
  if (!state.map) {
    log('Map not initialized', 'error');
    return { x: 0, y: 0 };
  }
  
  // Convert lat/lng to pixel coordinates
  const point = state.map.latLngToLayerPoint([lat, lng]);
  return { x: point.x, y: point.y };
};

/**
 * Get the current zoom level
 * @returns {number} Current zoom level
 */
window.MapSetup.getZoom = function() {
  return state.currentZoom;
};

/**
 * Get the map instance
 * @returns {Object} Leaflet map instance
 */
window.MapSetup.getMap = function() {
  return state.map;
};

/**
 * Get the SVG overlay
 * @returns {Object} D3 selection of the SVG overlay
 */
window.MapSetup.getSvgOverlay = function() {
  if (!state.svgOverlay) {
    log('SVG overlay not initialized', 'warn');
    return null;
  }
  
  return d3.select(state.svgOverlay._container);
}; 