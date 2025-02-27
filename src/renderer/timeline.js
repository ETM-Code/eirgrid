/**
 * Timeline Module
 * 
 * This module handles the timeline slider and time controls for the simulation.
 * It allows users to navigate through different years of the simulation.
 */

// Helper function to log from this module
function log(message, level = 'info') {
  if (window.AppLog) {
    window.AppLog('Timeline', message, level);
  } else {
    console.log(`[Timeline] ${message}`);
  }
}

// Ensure Timeline exists as a global object
window.Timeline = window.Timeline || {};

// Initialize internal state 
window.Timeline.state = window.Timeline.state || {
  slider: null,
  isPlaying: false,
  playInterval: null,
  currentYear: 2025,
  yearDisplay: null,
  playButton: null,
  pauseButton: null,
  resetButton: null,
  minYear: 2025,
  maxYear: 2050,
  onYearChangeCallback: null,
  ANIMATION_SPEED: 1500 // Time in ms between years
};

/**
 * Initialize the timeline controls
 * @param {Function} onYearChange - Callback function when year changes
 */
window.Timeline.init = function(onYearChange) {
    log('Initializing timeline controls');
    
    // Store the callback function
    window.Timeline.state.onYearChangeCallback = onYearChange;
    
    // Get DOM elements
    window.Timeline.state.yearDisplay = document.getElementById('current-year');
    window.Timeline.state.playButton = document.getElementById('play-pause');
    window.Timeline.state.resetButton = document.getElementById('reset');
    
    // Initialize the slider
    initSlider();
    
    // Set up button event listeners
    if (window.Timeline.state.playButton) {
        window.Timeline.state.playButton.addEventListener('click', togglePlayback);
    }
    
    if (window.Timeline.state.resetButton) {
        window.Timeline.state.resetButton.addEventListener('click', resetTimeline);
    }
    
    // Update the year display
    updateYearDisplay();
    
    log('Timeline initialized successfully');
};

/**
 * Initialize the noUiSlider timeline slider
 */
function initSlider() {
    const sliderElement = document.getElementById('timeline-slider');
    
    if (!sliderElement) {
        log('Timeline slider element not found', 'warn');
        return;
    }
    
    log('Creating timeline slider');
    
    // Initialize noUiSlider
    window.Timeline.state.slider = noUiSlider.create(sliderElement, {
        start: [window.Timeline.state.minYear],
        step: 1,
        range: {
            'min': [window.Timeline.state.minYear],
            'max': [window.Timeline.state.maxYear]
        },
        pips: {
            mode: 'positions',
            values: [0, 20, 40, 60, 80, 100],
            density: 4,
            format: {
                to: value => Math.round(value)
            }
        }
    });
    
    // Add event listener for slider changes
    window.Timeline.state.slider.on('update', (values, handle) => {
        const year = Math.round(values[handle]);
        if (year !== window.Timeline.state.currentYear) {
            window.Timeline.setYear(year);
        }
    });
    
    log('Timeline slider created');
}

/**
 * Toggle play/pause of the timeline
 */
function togglePlayback() {
    log('Toggling playback');
    
    if (window.Timeline.state.isPlaying) {
        pausePlayback();
    } else {
        startPlayback();
    }
}

/**
 * Start the timeline playback
 */
function startPlayback() {
    if (window.Timeline.state.isPlaying) return;
    
    log('Starting timeline playback');
    
    window.Timeline.state.isPlaying = true;
    
    if (window.Timeline.state.playButton) {
        window.Timeline.state.playButton.querySelector('.play-icon').textContent = '⏸';
    }
    
    // Clear any existing interval
    if (window.Timeline.state.playInterval) {
        clearInterval(window.Timeline.state.playInterval);
    }
    
    // Start a new interval
    window.Timeline.state.playInterval = setInterval(() => {
        // Increment the year
        const nextYear = window.Timeline.state.currentYear + 1;
        
        // Check if we've reached the end
        if (nextYear > window.Timeline.state.maxYear) {
            pausePlayback();
            return;
        }
        
        // Update the year
        window.Timeline.setYear(nextYear);
    }, window.Timeline.state.ANIMATION_SPEED);
}

/**
 * Pause the timeline playback
 */
function pausePlayback() {
    if (!window.Timeline.state.isPlaying) return;
    
    log('Pausing timeline playback');
    
    window.Timeline.state.isPlaying = false;
    
    if (window.Timeline.state.playButton) {
        window.Timeline.state.playButton.querySelector('.play-icon').textContent = '▶';
    }
    
    // Clear the interval
    if (window.Timeline.state.playInterval) {
        clearInterval(window.Timeline.state.playInterval);
        window.Timeline.state.playInterval = null;
    }
}

/**
 * Reset the timeline to the start
 */
function resetTimeline() {
    log('Resetting timeline');
    
    // Pause playback first
    pausePlayback();
    
    // Reset to the minimum year
    window.Timeline.setYear(window.Timeline.state.minYear);
}

/**
 * Update the year display in the UI
 */
function updateYearDisplay() {
    if (window.Timeline.state.yearDisplay) {
        window.Timeline.state.yearDisplay.textContent = window.Timeline.state.currentYear;
    }
}

/**
 * Set the current year
 * @param {number} year - Year to set
 */
window.Timeline.setYear = function(year) {
  // Ensure the year is within bounds
  const validYear = Math.max(window.Timeline.state.minYear, Math.min(window.Timeline.state.maxYear, year));
  
  // Update current year
  window.Timeline.state.currentYear = validYear;
  
  // Update the display
  updateYearDisplay();
  
  // Update the slider if it doesn't match the year
  if (window.Timeline.state.slider) {
    const sliderValue = parseFloat(window.Timeline.state.slider.get());
    if (sliderValue !== validYear) {
      window.Timeline.state.slider.set(validYear);
    }
  }
  
  // Call the callback function
  if (window.Timeline.state.onYearChangeCallback && typeof window.Timeline.state.onYearChangeCallback === 'function') {
    window.Timeline.state.onYearChangeCallback(validYear);
  }
  
  log(`Year set to ${validYear}`);
};

/**
 * Get the current year
 * @returns {number} Current year
 */
window.Timeline.getCurrentYear = function() {
  return window.Timeline.state.currentYear;
};

/**
 * Update the year range for the timeline
 * @param {number} min - Minimum year
 * @param {number} max - Maximum year
 */
window.Timeline.updateYearRange = function(min, max) {
  window.Timeline.state.minYear = min;
  window.Timeline.state.maxYear = max;
  
  // Update slider range
  if (window.Timeline.state.slider) {
    window.Timeline.state.slider.updateOptions({
      range: {
        'min': [window.Timeline.state.minYear],
        'max': [window.Timeline.state.maxYear]
      }
    });
  }
  
  // Reset to min year if current year is out of bounds
  if (window.Timeline.state.currentYear < window.Timeline.state.minYear || 
      window.Timeline.state.currentYear > window.Timeline.state.maxYear) {
    window.Timeline.setYear(window.Timeline.state.minYear);
  }
  
  log(`Year range updated: ${min}-${max}`);
};

// Add additional methods to Timeline global object
window.Timeline.startPlayback = startPlayback;
window.Timeline.pausePlayback = pausePlayback;
window.Timeline.resetTimeline = resetTimeline; 