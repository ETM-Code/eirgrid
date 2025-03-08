use lazy_static::lazy_static;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::Level;
use tracing_subscriber::{EnvFilter, prelude::*};
use tracing_timing::{Builder, Histogram};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, Instant};
use std::cell::RefCell;

// Define categories for different types of operations
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum OperationCategory {
    Simulation,
    PowerCalculation {
        subcategory: PowerCalcType,
    },
    LocationSearch {
        subcategory: LocationSearchType,
    },
    WeightsUpdate {
        subcategory: WeightsUpdateType,
    },
    FileIO {
        subcategory: FileIOType,
    },
    Other,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum PowerCalcType {
    Generation,
    Usage,
    Balance,
    Efficiency,
    Other,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum LocationSearchType {
    GeneratorPlacement,
    SuitabilityCheck,
    ConstraintValidation,
    Other,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum WeightsUpdateType {
    ActionUpdate,
    StrategyOptimization,
    MetricsCalculation,
    Other,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum FileIOType {
    CheckpointSave,
    CheckpointLoad,
    DataLoad,
    ResultsSave,
    Other,
}

impl OperationCategory {
    pub fn as_str(&self) -> String {
        match self {
            OperationCategory::Simulation => "Simulation".to_string(),
            OperationCategory::PowerCalculation { subcategory } => {
                format!("Power Calculation - {}", match subcategory {
                    PowerCalcType::Generation => "Generation",
                    PowerCalcType::Usage => "Usage",
                    PowerCalcType::Balance => "Balance",
                    PowerCalcType::Efficiency => "Efficiency",
                    PowerCalcType::Other => "Other",
                })
            },
            OperationCategory::LocationSearch { subcategory } => {
                format!("Location Search - {}", match subcategory {
                    LocationSearchType::GeneratorPlacement => "Generator Placement",
                    LocationSearchType::SuitabilityCheck => "Suitability Check",
                    LocationSearchType::ConstraintValidation => "Constraint Validation",
                    LocationSearchType::Other => "Other",
                })
            },
            OperationCategory::WeightsUpdate { subcategory } => {
                format!("Weights Update - {}", match subcategory {
                    WeightsUpdateType::ActionUpdate => "Action Update",
                    WeightsUpdateType::StrategyOptimization => "Strategy Optimization",
                    WeightsUpdateType::MetricsCalculation => "Metrics Calculation",
                    WeightsUpdateType::Other => "Other",
                })
            },
            OperationCategory::FileIO { subcategory } => {
                format!("File I/O - {}", match subcategory {
                    FileIOType::CheckpointSave => "Checkpoint Save",
                    FileIOType::CheckpointLoad => "Checkpoint Load",
                    FileIOType::DataLoad => "Data Load",
                    FileIOType::ResultsSave => "Results Save",
                    FileIOType::Other => "Other",
                })
            },
            OperationCategory::Other => "Other Operations".to_string(),
        }
    }
}

thread_local! {
    static TIMING_STACK: RefCell<Vec<(String, OperationCategory, Instant)>> = RefCell::new(Vec::new());
}

lazy_static! {
    static ref TIMING_ENABLED: AtomicBool = AtomicBool::new(false);
    static ref FUNCTION_TIMINGS: Arc<RwLock<HashMap<String, Histogram<u64>>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref CATEGORY_TIMINGS: Arc<RwLock<HashMap<OperationCategory, Histogram<u64>>>> = Arc::new(RwLock::new(HashMap::new()));
    static ref HIERARCHICAL_TIMINGS: Arc<RwLock<HashMap<String, (Duration, usize, Vec<String>)>>> = Arc::new(RwLock::new(HashMap::new()));
}

pub struct TimingGuard {
    function_name: String,
    category: OperationCategory,
    start: Instant,
}

impl Drop for TimingGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        record_timing_end(&self.function_name, duration, &self.category);
    }
}

pub fn start_timing(function_name: &str, category: OperationCategory) -> TimingGuard {
    let guard = TimingGuard {
        function_name: function_name.to_string(),
        category: category.clone(),
        start: Instant::now(),
    };

    TIMING_STACK.with(|stack| {
        stack.borrow_mut().push((function_name.to_string(), category, Instant::now()));
    });

    guard
}

fn record_timing_end(function_name: &str, duration: Duration, category: &OperationCategory) {
    if !is_timing_enabled() {
        return;
    }

    let duration_ns = duration.as_nanos() as u64;
    
    // Pop from timing stack and calculate exclusive time
    TIMING_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        if let Some((_, _, __start)) = stack.pop() {
            let mut hierarchical = HIERARCHICAL_TIMINGS.write();
            let entry = hierarchical
                .entry(function_name.to_string())
                .or_insert((Duration::from_nanos(0), 0, Vec::new()));
            
            entry.0 += duration;
            entry.1 += 1;
            
            // Add parent information
            if let Some((parent_name, _, _)) = stack.last() {
                if !entry.2.contains(parent_name) {
                    entry.2.push(parent_name.clone());
                }
            }
        }
    });

    // Record function-specific timing
    {
        let mut timings = FUNCTION_TIMINGS.write();
        let histogram = timings
            .entry(function_name.to_string())
            .or_insert_with(|| Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3).unwrap());
        
        let _ = histogram.record(duration_ns);
    }

    // Record category timing
    {
        let mut category_timings = CATEGORY_TIMINGS.write();
        let histogram = category_timings
            .entry(category.clone())
            .or_insert_with(|| Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3).unwrap());
        
        let _ = histogram.record(duration_ns);
    }
}

pub fn init_logging(enable_timing: bool) {
    TIMING_ENABLED.store(enable_timing, Ordering::SeqCst);
    
    let env_filter = EnvFilter::from_default_env()
        .add_directive(Level::INFO.into())
        .add_directive("eirgrid=debug".parse().unwrap());

    if enable_timing {
        let histogram = || {
            Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3).unwrap()
        };
        
        let timing_layer = Builder::default().layer(histogram);
        
        let subscriber = tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().pretty())
            .with(timing_layer.boxed());
            
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set up tracing subscriber");
    } else {
        let subscriber = tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().pretty());
            
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set up tracing subscriber");
    }
}

pub fn is_timing_enabled() -> bool {
    TIMING_ENABLED.load(Ordering::SeqCst)
}

pub fn print_timing_report() {
    if !is_timing_enabled() {
        return;
    }

    println!("\nDetailed Performance Report");
    println!("==========================");
    
    // Print hierarchical timing summary
    println!("\nHierarchical Timing Analysis:");
    println!("---------------------------");
    let hierarchical = HIERARCHICAL_TIMINGS.read();
    let mut entries: Vec<_> = hierarchical.iter().collect();
    entries.sort_by(|a, b| b.1.0.cmp(&a.1.0));

    for (function_name, (total_duration, count, parents)) in entries {
        let avg_duration = total_duration.div_f64(*count as f64);
        println!(
            "{}: total={:.2}s, count={}, avg={:.2}ms{}",
            function_name,
            total_duration.as_secs_f64(),
            count,
            avg_duration.as_secs_f64() * 1000.0,
            if !parents.is_empty() {
                format!("\n  Called by: {}", parents.join(", "))
            } else {
                String::new()
            }
        );
    }
    
    // Print category summary
    println!("\nPerformance by Category:");
    println!("------------------------");
    let category_timings = CATEGORY_TIMINGS.read();
    let mut category_vec: Vec<_> = category_timings.iter().collect();
    category_vec.sort_by(|a, b| {
        let b_mean = b.1.mean();
        let a_mean = a.1.mean();
        b_mean.partial_cmp(&a_mean).unwrap_or(std::cmp::Ordering::Equal)
    });

    let total_time: f64 = category_vec.iter()
        .map(|(_, hist)| hist.mean() * (hist.len() as f64))
        .sum();

    for (category, histogram) in category_vec {
        let category_total = histogram.mean() * (histogram.len() as f64);
        let percentage = (category_total / total_time) * 100.0;
        println!(
            "{}: {:.1}% of total time\n  mean={:.2}ms, p95={:.2}ms, p99={:.2}ms, count={}, total={:.2}s",
            category.as_str(),
            percentage,
            histogram.mean() / 1_000_000.0,
            histogram.value_at_quantile(0.95) as f64 / 1_000_000.0,
            histogram.value_at_quantile(0.99) as f64 / 1_000_000.0,
            histogram.len(),
            category_total / 1_000_000_000.0,
        );
    }

    println!("==========================\n");
} 