/**
 * Visualization Module
 * 
 * This module handles the visualization of data on the map using D3.js.
 * It creates and updates markers for settlements, generators, and carbon offsets.
 */

// Helper function to log from this module
function log(message, level = 'info') {
  if (window.AppLog) {
    window.AppLog('Visualization', message, level);
  } else {
    console.log(`[Visualization] ${message}`);
  }
}

// Ensure Visualization exists as a global object
window.Visualization = window.Visualization || {};

// Internal state
window.Visualization.state = window.Visualization.state || {
  map: null,
  svg: null,
  currentData: null,
  tooltip: null,
  pendingData: null,
  previousEntities: {
    generators: new Set(),
    offsets: new Set()
  }
};

// Map specific generator types to general categories
function mapGeneratorTypeToCategory(type) {
  const typeStr = String(type).toLowerCase();
  
  if (typeStr.includes('wind')) {
    return 'wind';
  } else if (typeStr.includes('solar')) {
    return 'solar';
  } else if (typeStr.includes('hydro') || typeStr.includes('pumped') || 
             typeStr.includes('tidal') || typeStr.includes('wave')) {
    return 'hydro';
  } else if (typeStr.includes('nuclear')) {
    return 'nuclear';
  } else if (typeStr.includes('coal')) {
    return 'coal';
  } else if (typeStr.includes('gas')) {
    return 'gas';
  } else if (typeStr.includes('biomass')) {
    return 'biomass';
  } else if (typeStr.includes('battery') || typeStr.includes('storage')) {
    return 'storage';
  } else {
    return 'unknown';
  }
}

// Track which entities are new
function identifyNewEntities(data) {
  const newGenerators = new Set();
  const newOffsets = new Set();
  
  if (Array.isArray(data.generators)) {
    data.generators.forEach(generator => {
      if (!window.Visualization.state.previousEntities.generators.has(generator.id)) {
        newGenerators.add(generator.id);
        window.Visualization.state.previousEntities.generators.add(generator.id);
      }
    });
  }
  
  if (Array.isArray(data.carbonOffsets)) {
    data.carbonOffsets.forEach(offset => {
      if (!window.Visualization.state.previousEntities.offsets.has(offset.id)) {
        newOffsets.add(offset.id);
        window.Visualization.state.previousEntities.offsets.add(offset.id);
      }
    });
  }
  
  return { newGenerators, newOffsets };
}

/**
 * Initialize the visualization
 * @param {Object} mapInstance - The Leaflet map instance
 */
window.Visualization.init = function(mapInstance) {
  log('Initializing visualization module');
  
  window.Visualization.state.map = mapInstance;
  window.Visualization.state.svg = window.MapSetup.getSvgOverlay();
  window.Visualization.state.tooltip = document.getElementById('tooltip');
  
  if (!window.Visualization.state.svg) {
    log('SVG overlay not found - creating it', 'warn');
    // Create SVG layers if they don't exist
    createSvgLayers();
  }
  
  // Set up event handlers for map move/zoom
  window.Visualization.state.map.on('moveend', updateMarkerPositions);
  window.Visualization.state.map.on('zoomend', updateMarkerSizes);
  
  log('Visualization module initialized successfully');
  
  // Check if there's pending data to visualize from before initialization was complete
  if (window.Visualization.state.pendingData) {
    log('Processing pending visualization data');
    const pendingData = window.Visualization.state.pendingData;
    window.Visualization.state.pendingData = null; // Clear pending data
    window.Visualization.update(pendingData); // Now update with the data
  }
};

/**
 * Create SVG layers for visualization
 */
function createSvgLayers() {
  log('Creating SVG layers for visualization');
  
  // This would typically be handled by MapSetup, but we'll add fallback here
  try {
    const svgOverlay = L.svg().addTo(window.Visualization.state.map);
    window.Visualization.state.svg = d3.select(svgOverlay._container);
    
    // Set z-index to ensure SVG is above map tiles
    window.Visualization.state.svg
      .style('z-index', '650')
      .style('position', 'relative')
      .style('pointer-events', 'auto');
    
    // Create layer groups
    window.Visualization.state.svg.append('g').attr('class', 'settlements-layer');
    window.Visualization.state.svg.append('g').attr('class', 'generators-layer');
    window.Visualization.state.svg.append('g').attr('class', 'offsets-layer');
    
    log('SVG layers created successfully');
  } catch (error) {
    log(`Error creating SVG layers: ${error.message}`, 'error');
  }
}

/**
 * Update the visualization with new data
 * @param {Object} data - The data to visualize
 */
window.Visualization.update = function(data) {
  if (!data) {
    log('No data provided for visualization update', 'warn');
    return;
  }
  
  log(`Updating visualization with data for year ${data.year}`);
  
  if (!window.Visualization.state.map || !window.Visualization.state.svg) {
    log('Map or SVG not initialized, will update when initialization completes', 'warn');
    // Store the data for later update once initialization is complete
    window.Visualization.state.pendingData = data;
    return;
  }
  
  window.Visualization.state.currentData = data;
  
  // Identify new entities that have been added in this update
  const { newGenerators, newOffsets } = identifyNewEntities(data);
  
  // Update each layer with new data
  try {
    if (Array.isArray(data.settlements)) {
      updateSettlementsLayer(data.settlements);
    }
    
    if (Array.isArray(data.generators)) {
      updateGeneratorsLayer(data.generators, newGenerators);
    }
    
    if (Array.isArray(data.carbonOffsets)) {
      updateOffsetsLayer(data.carbonOffsets, newOffsets);
    }
    
    // Update marker positions in case the map has moved
    try {
      updateMarkerPositions();
    } catch (posError) {
      log(`Error updating marker positions: ${posError.message}`, 'warn');
    }
    
    const settlementCount = Array.isArray(data.settlements) ? data.settlements.length : 0;
    const generatorCount = Array.isArray(data.generators) ? data.generators.length : 0;
    const offsetCount = Array.isArray(data.carbonOffsets) ? data.carbonOffsets.length : 0;
    
    log(`Visualization updated with ${settlementCount} settlements, ${generatorCount} generators, ${offsetCount} carbon offsets`);
  } catch (error) {
    log(`Error updating visualization: ${error.message}`, 'error');
  }
}

/**
 * Update marker positions when the map moves or zooms
 */
function updateMarkerPositions() {
  if (!window.Visualization.state.map || !window.Visualization.state.svg || !window.Visualization.state.currentData) return;
  
  log('Updating marker positions', 'debug');
  
  // Helper function to convert lat/lng to map point
  const latLngToPoint = (lat, lng) => {
    const point = window.Visualization.state.map.latLngToLayerPoint([lat, lng]);
    return { x: point.x, y: point.y };
  };
  
  // Update settlements
  window.Visualization.state.svg.select('.settlements-layer')
    .selectAll('circle')
    .attr('cx', d => latLngToPoint(d.lat, d.lng).x)
    .attr('cy', d => latLngToPoint(d.lat, d.lng).y);
  
  // Update generators
  window.Visualization.state.svg.select('.generators-layer')
    .selectAll('circle')
    .attr('cx', d => latLngToPoint(d.lat, d.lng).x)
    .attr('cy', d => latLngToPoint(d.lat, d.lng).y);
  
  // Update carbon offsets
  window.Visualization.state.svg.select('.offsets-layer')
    .selectAll('circle')
    .attr('cx', d => latLngToPoint(d.lat, d.lng).x)
    .attr('cy', d => latLngToPoint(d.lat, d.lng).y);
}

/**
 * Update marker sizes based on zoom level
 */
function updateMarkerSizes() {
  if (!window.Visualization.state.map || !window.Visualization.state.svg || !window.Visualization.state.currentData) return;
  
  log('Updating marker sizes based on zoom level', 'debug');
  
  const zoom = window.Visualization.state.map.getZoom();
  
  // Update settlements
  window.Visualization.state.svg.select('.settlements-layer')
    .selectAll('circle')
    .attr('r', d => window.GridCoordinates.calculateMarkerRadius(d.population, 'settlement', zoom));
  
  // Update generators
  window.Visualization.state.svg.select('.generators-layer')
    .selectAll('circle')
    .attr('r', d => window.GridCoordinates.calculateMarkerRadius(d.output, 'generator', zoom));
  
  // Update carbon offsets
  window.Visualization.state.svg.select('.offsets-layer')
    .selectAll('circle')
    .attr('r', d => window.GridCoordinates.calculateMarkerRadius(d.offsetAmount, 'offset', zoom));
}

/**
 * Update the settlements layer
 * @param {Array} settlements - Array of settlement objects
 */
function updateSettlementsLayer(settlements) {
  if (!window.Visualization.state.map || !window.Visualization.state.svg) return;
  
  log(`Updating settlements layer with ${settlements.length} settlements`, 'debug');
  
  const zoom = window.Visualization.state.map.getZoom();
  const settlementsLayer = window.Visualization.state.svg.select('.settlements-layer');
  
  // Helper function to convert lat/lng to map point
  const latLngToPoint = (lat, lng) => {
    const point = window.Visualization.state.map.latLngToLayerPoint([lat, lng]);
    return { x: point.x, y: point.y };
  };
  
  // Join settlement data
  const settlementMarkers = settlementsLayer
    .selectAll('circle')
    .data(settlements, d => d.id);
  
  // Remove old markers
  settlementMarkers.exit().remove();
  
  // Add new markers
  settlementMarkers.enter()
    .append('circle')
    .attr('class', 'settlement-marker')
    .merge(settlementMarkers)
    .attr('cx', d => latLngToPoint(d.lat, d.lng).x)
    .attr('cy', d => latLngToPoint(d.lat, d.lng).y)
    .attr('r', d => window.GridCoordinates.calculateMarkerRadius(d.population, 'settlement', zoom))
    .on('mouseover', showSettlementTooltip)
    .on('mouseout', hideTooltip);
}

/**
 * Update the generators layer
 * @param {Array} generators - Array of generator objects
 * @param {Set} newGenerators - Set of IDs for newly added generators
 */
function updateGeneratorsLayer(generators, newGenerators) {
  if (!window.Visualization.state.map || !window.Visualization.state.svg) return;
  
  log(`Updating generators layer with ${generators.length} generators`, 'debug');
  
  const zoom = window.Visualization.state.map.getZoom();
  const generatorsLayer = window.Visualization.state.svg.select('.generators-layer');
  
  // Helper function to convert lat/lng to map point
  const latLngToPoint = (lat, lng) => {
    const point = window.Visualization.state.map.latLngToLayerPoint([lat, lng]);
    return { x: point.x, y: point.y };
  };
  
  // Join generator data
  const generatorMarkers = generatorsLayer
    .selectAll('circle')
    .data(generators, d => d.id);
  
  // Remove old markers
  generatorMarkers.exit().remove();
  
  // Add new markers
  const newMarkers = generatorMarkers.enter()
    .append('circle')
    .attr('class', d => {
      // Map the specific generator type to a general category for CSS styling
      const category = mapGeneratorTypeToCategory(d.type);
      // Check if this is a new generator to add highlight class
      const isNew = newGenerators && newGenerators.has(d.id);
      return `generator-marker ${category}-marker${isNew ? ' new-entity' : ''}`;
    })
    .merge(generatorMarkers)
    .attr('cx', d => latLngToPoint(d.lat, d.lng).x)
    .attr('cy', d => latLngToPoint(d.lat, d.lng).y)
    .attr('r', d => window.GridCoordinates.calculateMarkerRadius(d.output, 'generator', zoom))
    .on('mouseover', showGeneratorTooltip)
    .on('mouseout', hideTooltip);
  
  // Remove the highlight class after a delay
  if (newGenerators && newGenerators.size > 0) {
    setTimeout(() => {
      generatorsLayer.selectAll('.new-entity')
        .classed('new-entity', false);
    }, 2000); // Remove highlight after 2 seconds
  }
}

/**
 * Update the carbon offsets layer
 * @param {Array} offsets - Array of carbon offset objects
 * @param {Set} newOffsets - Set of IDs for newly added offsets
 */
function updateOffsetsLayer(offsets, newOffsets) {
  if (!window.Visualization.state.map || !window.Visualization.state.svg) return;
  
  log(`Updating offsets layer with ${offsets.length} carbon offsets`, 'debug');
  
  const zoom = window.Visualization.state.map.getZoom();
  const offsetsLayer = window.Visualization.state.svg.select('.offsets-layer');
  
  // Helper function to convert lat/lng to map point
  const latLngToPoint = (lat, lng) => {
    const point = window.Visualization.state.map.latLngToLayerPoint([lat, lng]);
    return { x: point.x, y: point.y };
  };
  
  // Join offset data
  const offsetMarkers = offsetsLayer
    .selectAll('circle')
    .data(offsets, d => d.id);
  
  // Remove old markers
  offsetMarkers.exit().remove();
  
  // Add new markers
  offsetMarkers.enter()
    .append('circle')
    .attr('class', d => {
      // Check if this is a new offset to add highlight class
      const isNew = newOffsets && newOffsets.has(d.id);
      return `offset-marker${isNew ? ' new-entity' : ''}`;
    })
    .merge(offsetMarkers)
    .attr('cx', d => latLngToPoint(d.lat, d.lng).x)
    .attr('cy', d => latLngToPoint(d.lat, d.lng).y)
    .attr('r', d => window.GridCoordinates.calculateMarkerRadius(d.offsetAmount, 'offset', zoom))
    .on('mouseover', showOffsetTooltip)
    .on('mouseout', hideTooltip);
  
  // Remove the highlight class after a delay
  if (newOffsets && newOffsets.size > 0) {
    setTimeout(() => {
      offsetsLayer.selectAll('.new-entity')
        .classed('new-entity', false);
    }, 2000); // Remove highlight after 2 seconds
  }
}

/**
 * Show tooltip for settlement
 * @param {Event} event - Mouse event
 * @param {Object} d - Settlement data
 */
function showSettlementTooltip(event, d) {
  if (!window.Visualization.state.tooltip) return;
  
  const formatNumber = num => num.toLocaleString();
  
  window.Visualization.state.tooltip.style.display = 'block';
  window.Visualization.state.tooltip.style.left = (event.pageX + 15) + 'px';
  window.Visualization.state.tooltip.style.top = (event.pageY - 20) + 'px';
  window.Visualization.state.tooltip.innerHTML = `
    <div class="popup-header settlement">${d.name}</div>
    <div class="popup-content">
      <div class="popup-row">
        <span class="popup-label">Population:</span>
        <span class="popup-value">${formatNumber(d.population)}</span>
      </div>
      <div class="popup-row">
        <span class="popup-label">Power Usage:</span>
        <span class="popup-value">${d.powerUsage?.toFixed(2) || 0} MWh</span>
      </div>
    </div>
  `;
}

/**
 * Show tooltip for generator
 * @param {Event} event - Mouse event
 * @param {Object} d - Generator data
 */
function showGeneratorTooltip(event, d) {
  if (!window.Visualization.state.tooltip) return;
  
  const formatNumber = num => num.toLocaleString();
  
  window.Visualization.state.tooltip.style.display = 'block';
  window.Visualization.state.tooltip.style.left = (event.pageX + 15) + 'px';
  window.Visualization.state.tooltip.style.top = (event.pageY - 20) + 'px';
  window.Visualization.state.tooltip.innerHTML = `
    <div class="popup-header ${d.type}">${d.name}</div>
    <div class="popup-content">
      <div class="popup-row">
        <span class="popup-label">Type:</span>
        <span class="popup-value">${d.type}</span>
      </div>
      <div class="popup-row">
        <span class="popup-label">Output:</span>
        <span class="popup-value">${d.output?.toFixed(2) || 0} MWh</span>
      </div>
      ${d.emissions ? `
      <div class="popup-row">
        <span class="popup-label">Emissions:</span>
        <span class="popup-value">${formatNumber(d.emissions)} tonnes</span>
      </div>
      ` : ''}
    </div>
  `;
}

/**
 * Show tooltip for carbon offset
 * @param {Event} event - Mouse event
 * @param {Object} d - Carbon offset data
 */
function showOffsetTooltip(event, d) {
  if (!window.Visualization.state.tooltip) return;
  
  const formatNumber = num => num.toLocaleString();
  
  window.Visualization.state.tooltip.style.display = 'block';
  window.Visualization.state.tooltip.style.left = (event.pageX + 15) + 'px';
  window.Visualization.state.tooltip.style.top = (event.pageY - 20) + 'px';
  window.Visualization.state.tooltip.innerHTML = `
    <div class="popup-header offset">${d.name}</div>
    <div class="popup-content">
      <div class="popup-row">
        <span class="popup-label">Type:</span>
        <span class="popup-value">${d.type}</span>
      </div>
      <div class="popup-row">
        <span class="popup-label">Offset:</span>
        <span class="popup-value">${formatNumber(d.offsetAmount)} tonnes</span>
      </div>
      <div class="popup-row">
        <span class="popup-label">Area:</span>
        <span class="popup-value">${d.area?.toFixed(1) || 0} hectares</span>
      </div>
    </div>
  `;
}

/**
 * Hide tooltip
 */
function hideTooltip() {
  if (window.Visualization.state.tooltip) {
    window.Visualization.state.tooltip.style.display = 'none';
  }
} 