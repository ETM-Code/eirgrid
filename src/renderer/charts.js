/**
 * Charts Module
 * 
 * This module handles the creation and updating of charts to display
 * simulation trends over time.
 */

// Initialize the Charts module
window.Charts = window.Charts || {};

// Chart instances
let emissionsChart = null;
let powerChart = null;

// Register the annotation plugin with Chart.js
document.addEventListener('DOMContentLoaded', () => {
    try {
        if (typeof Chart === 'undefined') {
            console.error('Chart.js not loaded');
            return;
        }
        
        // First check if the annotation plugin is available in the global scope
        if (typeof chartjs_plugin_annotation !== 'undefined') {
            if (Chart.register) {
                Chart.register(chartjs_plugin_annotation);
                console.log('Registered Chart.js Annotation plugin from global scope');
            }
        } 
        // Then check if it's available via Chart.Annotation
        else if (Chart.Annotation) {
            if (Chart.register) {
                Chart.register(Chart.Annotation);
                console.log('Registered Chart.js Annotation plugin from Chart.Annotation');
            }
        } 
        // If we can't find it, try to load it from the CDN dynamically
        else {
            console.warn('Chart.js Annotation plugin not found, attempting to load it dynamically');
            
            // Create a script element to load the plugin
            const script = document.createElement('script');
            script.src = 'https://cdn.jsdelivr.net/npm/chartjs-plugin-annotation@1.4.0/dist/chartjs-plugin-annotation.min.js';
            script.onload = function() {
                console.log('Chart.js Annotation plugin loaded dynamically');
                if (Chart.register && typeof chartjs_plugin_annotation !== 'undefined') {
                    Chart.register(chartjs_plugin_annotation);
                    console.log('Registered Chart.js Annotation plugin after dynamic loading');
                    
                    // If charts were already created, update them
                    if (emissionsChart) updateEmissionsChart(window.DataLoader?.state?.currentYear);
                    if (powerChart) updatePowerChart(window.DataLoader?.state?.currentYear);
                }
            };
            script.onerror = function() {
                console.error('Failed to load Chart.js Annotation plugin dynamically');
            };
            document.head.appendChild(script);
        }
    } catch (error) {
        console.error('Error registering Chart.js Annotation plugin:', error);
    }
});

// Chart data
let chartData = {
    years: [],
    emissions: [],
    offsets: [],
    netEmissions: [],
    powerGeneration: [],
    powerUsage: [],
    powerBalance: []
};

// Chart colors
const CHART_COLORS = {
    emissions: 'rgba(255, 99, 132, 0.7)',
    offsets: 'rgba(75, 192, 192, 0.7)',
    netEmissions: 'rgba(153, 102, 255, 0.7)',
    powerGeneration: 'rgba(54, 162, 235, 0.7)',
    powerUsage: 'rgba(255, 206, 86, 0.7)',
    powerBalance: 'rgba(75, 192, 192, 0.7)'
};

/**
 * Initialize the charts
 * @param {Array} yearlyMetrics - Array of yearly metrics data
 */
function initializeCharts(yearlyMetrics) {
    console.log('Initializing charts...');
    
    // Set initial chart data
    if (yearlyMetrics && Array.isArray(yearlyMetrics) && yearlyMetrics.length > 0) {
        updateCharts(yearlyMetrics, yearlyMetrics[0].year);
    } else {
        console.warn('No metrics data provided for chart initialization');
        // Initialize with empty data
        chartData.years = [];
        chartData.emissions = [];
        chartData.offsets = [];
        chartData.netEmissions = [];
        chartData.powerGeneration = [];
        chartData.powerUsage = [];
        chartData.powerBalance = [];
    }
    
    // Create the emissions chart
    createEmissionsChart();
    
    // Create the power chart
    createPowerChart();
    
    console.log('Charts initialized');
}

/**
 * Update the charts with new data
 * @param {Object} yearlyMetrics - Array of yearly metrics
 * @param {number} currentYear - The current year to highlight
 */
function updateCharts(yearlyMetrics, currentYear) {
    // Check if yearlyMetrics is null or undefined
    if (!yearlyMetrics || !Array.isArray(yearlyMetrics) || yearlyMetrics.length === 0) {
        console.warn('No metrics data available for charts');
        // Set empty arrays to prevent errors
        chartData.years = [];
        chartData.emissions = [];
        chartData.offsets = [];
        chartData.netEmissions = [];
        chartData.powerGeneration = [];
        chartData.powerUsage = [];
        chartData.powerBalance = [];
    } else {
        // Process data for charts
        chartData.years = yearlyMetrics.map(m => m.year);
        chartData.emissions = yearlyMetrics.map(m => m.co2Emissions / 1000000); // Convert to millions
        chartData.offsets = yearlyMetrics.map(m => m.carbonOffset / 1000000); // Convert to millions
        chartData.netEmissions = yearlyMetrics.map(m => m.netEmissions / 1000000); // Convert to millions
        chartData.powerGeneration = yearlyMetrics.map(m => m.totalPowerGeneration);
        chartData.powerUsage = yearlyMetrics.map(m => m.totalPowerUsage);
        chartData.powerBalance = yearlyMetrics.map(m => m.powerBalance);
    }
    
    // Update the emissions chart
    updateEmissionsChart(currentYear);
    
    // Update the power chart
    updatePowerChart(currentYear);
}

/**
 * Create the emissions chart
 */
function createEmissionsChart() {
    try {
        const emissionsElement = document.getElementById('emissions-chart');
        if (!emissionsElement) {
            console.error('Cannot find emissions-chart canvas element');
            return;
        }
        
        const ctx = emissionsElement.getContext('2d');
        if (!ctx) {
            console.error('Failed to get 2D context for emissions chart');
            return;
        }
        
        // Check if Chart.js is available
        if (typeof Chart === 'undefined') {
            console.error('Chart.js library not available');
            return;
        }
        
        // Destroy existing chart before creating a new one
        if (emissionsChart) {
            console.log('Destroying existing emissions chart');
            emissionsChart.destroy();
            emissionsChart = null;
        }
        
        emissionsChart = new Chart(ctx, {
            type: 'line',
            data: {
                labels: chartData.years,
                datasets: [
                    {
                        label: 'CO₂ Emissions',
                        data: chartData.emissions,
                        backgroundColor: CHART_COLORS.emissions,
                        borderColor: CHART_COLORS.emissions,
                        borderWidth: 2,
                        pointRadius: 3,
                        pointHoverRadius: 5,
                        fill: false,
                        tension: 0.1
                    },
                    {
                        label: 'Carbon Offset',
                        data: chartData.offsets,
                        backgroundColor: CHART_COLORS.offsets,
                        borderColor: CHART_COLORS.offsets,
                        borderWidth: 2,
                        pointRadius: 3,
                        pointHoverRadius: 5,
                        fill: false,
                        tension: 0.1
                    },
                    {
                        label: 'Net Emissions',
                        data: chartData.netEmissions,
                        backgroundColor: CHART_COLORS.netEmissions,
                        borderColor: CHART_COLORS.netEmissions,
                        borderWidth: 2,
                        pointRadius: 3,
                        pointHoverRadius: 5,
                        fill: false,
                        tension: 0.1
                    }
                ]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    title: {
                        display: true,
                        text: 'Carbon Emissions & Offsets (Million Tonnes)'
                    },
                    legend: {
                        position: 'bottom'
                    },
                    tooltip: {
                        mode: 'index',
                        intersect: false,
                        callbacks: {
                            label: function(context) {
                                let label = context.dataset.label || '';
                                if (label) {
                                    label += ': ';
                                }
                                if (context.parsed.y !== null) {
                                    label += context.parsed.y.toFixed(2) + ' million tonnes';
                                }
                                return label;
                            }
                        }
                    }
                },
                scales: {
                    x: {
                        title: {
                            display: true,
                            text: 'Year'
                        }
                    },
                    y: {
                        title: {
                            display: true,
                            text: 'Million Tonnes CO₂'
                        },
                        beginAtZero: true
                    }
                }
            }
        });
        
        console.log('Emissions chart created successfully');
    } catch (error) {
        console.error('Error creating emissions chart:', error);
    }
}

/**
 * Create the power chart
 */
function createPowerChart() {
    try {
        const powerElement = document.getElementById('power-chart');
        if (!powerElement) {
            console.error('Cannot find power-chart canvas element');
            return;
        }
        
        const ctx = powerElement.getContext('2d');
        if (!ctx) {
            console.error('Failed to get 2D context for power chart');
            return;
        }
        
        // Check if Chart.js is available
        if (typeof Chart === 'undefined') {
            console.error('Chart.js library not available');
            return;
        }
        
        // Destroy existing chart before creating a new one
        if (powerChart) {
            console.log('Destroying existing power chart');
            powerChart.destroy();
            powerChart = null;
        }
        
        powerChart = new Chart(ctx, {
            type: 'line',
            data: {
                labels: chartData.years,
                datasets: [
                    {
                        label: 'Power Generation',
                        data: chartData.powerGeneration,
                        backgroundColor: CHART_COLORS.powerGeneration,
                        borderColor: CHART_COLORS.powerGeneration,
                        borderWidth: 2,
                        pointRadius: 3,
                        pointHoverRadius: 5,
                        fill: false,
                        tension: 0.1
                    },
                    {
                        label: 'Power Usage',
                        data: chartData.powerUsage,
                        backgroundColor: CHART_COLORS.powerUsage,
                        borderColor: CHART_COLORS.powerUsage,
                        borderWidth: 2,
                        pointRadius: 3,
                        pointHoverRadius: 5,
                        fill: false,
                        tension: 0.1
                    },
                    {
                        label: 'Power Balance',
                        data: chartData.powerBalance,
                        backgroundColor: CHART_COLORS.powerBalance,
                        borderColor: CHART_COLORS.powerBalance,
                        borderWidth: 2,
                        pointRadius: 3,
                        pointHoverRadius: 5,
                        fill: false,
                        tension: 0.1
                    }
                ]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    title: {
                        display: true,
                        text: 'Power Generation & Usage (MW)'
                    },
                    legend: {
                        position: 'bottom'
                    },
                    tooltip: {
                        mode: 'index',
                        intersect: false,
                        callbacks: {
                            label: function(context) {
                                let label = context.dataset.label || '';
                                if (label) {
                                    label += ': ';
                                }
                                if (context.parsed.y !== null) {
                                    label += context.parsed.y.toFixed(2) + ' MW';
                                }
                                return label;
                            }
                        }
                    }
                },
                scales: {
                    x: {
                        title: {
                            display: true,
                            text: 'Year'
                        }
                    },
                    y: {
                        title: {
                            display: true,
                            text: 'Megawatts (MW)'
                        },
                        beginAtZero: true
                    }
                }
            }
        });
        
        console.log('Power chart created successfully');
    } catch (error) {
        console.error('Error creating power chart:', error);
    }
}

/**
 * Update the emissions chart
 * @param {number} currentYear - Current year to highlight
 */
function updateEmissionsChart(currentYear) {
    if (!emissionsChart) {
        console.warn('Emissions chart not initialized');
        return;
    }
    
    // Update the chart data
    emissionsChart.data.labels = chartData.years;
    emissionsChart.data.datasets[0].data = chartData.emissions;
    emissionsChart.data.datasets[1].data = chartData.offsets;
    emissionsChart.data.datasets[2].data = chartData.netEmissions;
    
    // Add vertical line for current year
    addCurrentYearIndicator(emissionsChart, currentYear);
    
    // Update the chart
    emissionsChart.update();
}

/**
 * Update the power chart
 * @param {number} currentYear - Current year to highlight
 */
function updatePowerChart(currentYear) {
    if (!powerChart) {
        console.warn('Power chart not initialized');
        return;
    }
    
    // Update the chart data
    powerChart.data.labels = chartData.years;
    powerChart.data.datasets[0].data = chartData.powerGeneration;
    powerChart.data.datasets[1].data = chartData.powerUsage;
    powerChart.data.datasets[2].data = chartData.powerBalance;
    
    // Add vertical line for current year
    addCurrentYearIndicator(powerChart, currentYear);
    
    // Update the chart
    powerChart.update();
}

/**
 * Add a vertical line to indicate the current year
 * @param {Object} chart - The chart to add the indicator to
 * @param {number} currentYear - The current year to highlight
 */
function addCurrentYearIndicator(chart, currentYear) {
    if (!chart || typeof chart !== 'object') {
        console.warn('Invalid chart object provided to addCurrentYearIndicator');
        return;
    }
    
    // Convert to number to ensure proper comparison
    currentYear = Number(currentYear);
    
    try {
        // Remove any existing annotations
        if (chart.options.plugins.annotation && chart.options.plugins.annotation.annotations) {
            chart.options.plugins.annotation.annotations = {};
        }
        
        if (!chart.options.plugins) {
            chart.options.plugins = {};
        }
        
        if (!chart.options.plugins.annotation) {
            chart.options.plugins.annotation = {};
        }
        
        // Find index of current year in the labels
        let yearIndex = -1;
        if (chart.data && chart.data.labels) {
            yearIndex = chart.data.labels.findIndex(year => Number(year) === currentYear);
        }
        
        if (yearIndex === -1) {
            console.warn(`Year ${currentYear} not found in chart data`);
            return;
        }
        
        // Add annotation
        chart.options.plugins.annotation.annotations = {
            line1: {
                type: 'line',
                xMin: yearIndex,
                xMax: yearIndex,
                borderColor: 'rgba(255, 0, 0, 0.7)',
                borderWidth: 2,
                label: {
                    content: `Current Year (${currentYear})`,
                    display: true,
                    position: 'top'
                }
            }
        };
        
        console.log(`Added year indicator for ${currentYear}`);
    } catch (error) {
        console.error('Error adding year indicator:', error);
    }
}

/**
 * Initialize Charts Module with metrics data
 * @param {Array} metrics - Array of yearly metric objects
 */
window.Charts.init = function(metrics) {
    initializeCharts(metrics);
};

/**
 * Update Charts with current year indicator
 * @param {Array} metrics - Array of yearly metric objects (optional)
 * @param {number} currentYear - Current year to highlight
 */
window.Charts.update = function(metrics, currentYear) {
    if (metrics) {
        // If new metrics provided, update the whole chart
        updateCharts(metrics, currentYear);
    } else {
        // Just update the year indicator on existing charts
        if (emissionsChart) {
            updateEmissionsChart(currentYear);
        }
        if (powerChart) {
            updatePowerChart(currentYear);
        }
    }
}; 