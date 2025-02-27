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
    // Check if annotations plugin is available in Chart.js
    if (!chart.options.plugins) {
        chart.options.plugins = {};
    }
    
    // Check if the annotation plugin exists
    const hasAnnotationPlugin = typeof Chart !== 'undefined' && 
                               Chart.Annotation && 
                               typeof Chart.Annotation === 'object';
    
    if (!hasAnnotationPlugin) {
        console.warn('Chart.js Annotation plugin not available, skipping year indicator');
        return;
    }
    
    // Remove any existing annotation
    if (chart.options.plugins.annotation) {
        chart.options.plugins.annotation.annotations = {};
    } else {
        chart.options.plugins.annotation = {
            annotations: {}
        };
    }
    
    // Get the index of the current year
    const yearIndex = chartData.years.indexOf(currentYear);
    if (yearIndex === -1) return; // Year not found in data
    
    try {
        // Add annotation for current year
        chart.options.plugins.annotation.annotations.currentYear = {
            type: 'line',
            xMin: currentYear,
            xMax: currentYear,
            borderColor: 'rgba(255, 0, 0, 0.7)',
            borderWidth: 2,
            label: {
                enabled: true,
                content: 'Current Year',
                position: 'top',
                backgroundColor: 'rgba(255, 0, 0, 0.7)'
            }
        };
    } catch (error) {
        console.error('Error adding year indicator to chart:', error);
    }
}

// Export the charts functionality - DO NOT redefine the entire window.Charts object
// Instead, assign the functions to the existing object
window.Charts.init = initializeCharts;
window.Charts.update = updateCharts; 