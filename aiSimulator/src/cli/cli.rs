use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short = 'n', long, default_value_t = 1000)]
    iterations: usize,

    #[arg(short, long, default_value_t = true)]
    parallel: bool,

    #[arg(long, default_value_t = false)]
    no_continue: bool,

    #[arg(short, long, default_value = "checkpoints")]
    checkpoint_dir: String,

    #[arg(short = 'i', long, default_value_t = 5)]
    checkpoint_interval: usize,

    #[arg(short = 'r', long, default_value_t = 10)]
    progress_interval: usize,

    #[arg(short = 'C', long, default_value = "cache")]
    cache_dir: String,

    #[arg(long, default_value_t = false)]
    force_full_simulation: bool,

    #[arg(long, default_value_t = false)]
    enable_timing: bool,

    #[arg(long, help = "Random seed for deterministic simulation")]
    seed: Option<u64>,

    #[arg(short, long, default_value_t = true)]
    verbose_state_logging: bool,
    
    #[arg(long, help = "Optimize for cost only, ignoring emissions and public opinion", default_value_t = false)]
    cost_only: bool,
    
    #[arg(long, help = "Enable revenue from energy sales to offset costs", default_value_t = false)]
    enable_energy_sales: bool,
}

// Add getter methods for all fields
impl Args {
    pub fn iterations(&self) -> usize {
        self.iterations
    }

    pub fn parallel(&self) -> bool {
        self.parallel
    }

    pub fn no_continue(&self) -> bool {
        self.no_continue
    }

    pub fn checkpoint_dir(&self) -> &str {
        &self.checkpoint_dir
    }

    pub fn checkpoint_interval(&self) -> usize {
        self.checkpoint_interval
    }

    pub fn progress_interval(&self) -> usize {
        self.progress_interval
    }

    pub fn cache_dir(&self) -> &str {
        &self.cache_dir
    }

    pub fn force_full_simulation(&self) -> bool {
        self.force_full_simulation
    }

    pub fn enable_timing(&self) -> bool {
        self.enable_timing
    }

    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    pub fn verbose_state_logging(&self) -> bool {
        self.verbose_state_logging
    }

    pub fn cost_only(&self) -> bool {
        self.cost_only
    }

    pub fn enable_energy_sales(&self) -> bool {
        self.enable_energy_sales
    }
}
