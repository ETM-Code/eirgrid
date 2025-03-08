use std::error::Error;
#[cfg(feature = "metal")]
use metal;
use crate::poi::{Coordinate, POI};
use crate::generator::GeneratorType;
use crate::settlement::Settlement;
use crate::generator::Generator;
use std::fmt;

#[cfg(feature = "metal")]
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
struct Candidate {
    x: f32,
    y: f32,
    score: f32,
}

#[cfg(feature = "metal")]
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
struct MetalSettlement {
    x: f32,
    y: f32,
    population: f32,
}

#[cfg(feature = "metal")]
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
struct MetalGenerator {
    x: f32,
    y: f32,
    size: f32,
}

#[cfg(feature = "metal")]
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
struct BufferParams {
    num_settlements: u32,
    num_generators: u32,
    num_coastline_points: u32,
    gen_type: u32,
    penalty_radius: f32,
    size_penalty: f32,
}

#[cfg(feature = "metal")]
#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
struct float2 {
    x: f32,
    y: f32,
}

#[derive(Default)]
pub struct MetalLocationSearch {
    #[cfg(feature = "metal")]
    device: metal::Device,
    #[cfg(feature = "metal")]
    command_queue: metal::CommandQueue,
    #[cfg(feature = "metal")]
    pipeline_state: metal::ComputePipelineState,
}

impl MetalLocationSearch {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        #[cfg(feature = "metal")]
        {
            let device = metal::Device::system_default().ok_or("No Metal device found")?;
            let command_queue = device.new_command_queue();
            
            let library = device.new_library_with_file("metal_location_search.metal")?;
            let kernel = library.get_function("find_best_location", None)?;
            let pipeline_state = device.new_compute_pipeline_state_with_function(&kernel)?;
            
            Ok(Self {
                device,
                command_queue,
                pipeline_state,
            })
        }
        
        #[cfg(not(feature = "metal"))]
        {
            Ok(Self::default())
        }
    }

    pub fn find_suitable_location(
        &self,
        settlements: &[Settlement],
        generators: &[Generator],
        coastline_points: &[Coordinate],
        gen_type: GeneratorType,
        size_penalty: f32,
    ) -> Option<Coordinate> {
        #[cfg(feature = "metal")]
        {
            // Existing Metal implementation
            // ... (keep the existing Metal implementation)
        }

        #[cfg(not(feature = "metal"))]
        {
            // CPU fallback implementation
            let grid_step = 1000.0;
            let num_x = (MAP_MAX_X / grid_step) as usize;
            let num_y = (MAP_MAX_Y / grid_step) as usize;
            
            let mut best_score = 0.0;
            let mut best_location = None;
            
            for i in 0..num_x {
                for j in 0..num_y {
                    let x = i as f64 * grid_step;
                    let y = j as f64 * grid_step;
                    let location = Coordinate::new(x, y);
                    
                    // Calculate score based on various factors
                    let mut score = 1.0;
                    
                    // Population proximity score
                    for settlement in settlements {
                        let distance = location.distance_to(&settlement.get_coordinate());
                        let population_factor = settlement.get_population() as f64 / 1_000_000.0;
                        score *= (1.0 + population_factor) / (1.0 + distance / 10_000.0);
                    }
                    
                    // Generator proximity penalty
                    for generator in generators {
                        let distance = location.distance_to(&generator.get_coordinate());
                        let penalty_radius = match gen_type {
                            GeneratorType::Nuclear => 12000.0,
                            GeneratorType::CoalPlant | GeneratorType::GasCombinedCycle => 8000.0,
                            GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 5000.0,
                            GeneratorType::HydroDam | GeneratorType::PumpedStorage => 7000.0,
                            GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => 6000.0,
                            _ => 3000.0,
                        };
                        if distance < penalty_radius {
                            score *= distance / penalty_radius;
                        }
                    }
                    
                    // Coastal proximity for relevant generator types
                    if matches!(gen_type, 
                        GeneratorType::OffshoreWind | 
                        GeneratorType::TidalGenerator | 
                        GeneratorType::WaveEnergy) {
                        let min_coastal_distance = coastline_points.iter()
                            .map(|point| location.distance_to(point))
                            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                            .unwrap_or(f64::MAX);
                        
                        score *= 1.0 / (1.0 + min_coastal_distance / 5000.0);
                    }
                    
                    // Apply size penalty
                    score *= 1.0 - (size_penalty as f64 * 0.1);
                    
                    if score > best_score {
                        best_score = score;
                        best_location = Some(location);
                    }
                }
            }
            
            best_location
        }
    }
}

// Manual Debug implementation since metal types don't implement Debug
impl fmt::Debug for MetalLocationSearch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MetalLocationSearch")
            .field("device", &"<metal::Device>")
            .field("command_queue", &"<metal::CommandQueue>")
            .field("pipeline_state", &"<metal::ComputePipelineState>")
            .finish()
    }
}

// Manual Clone implementation since metal types don't implement Clone
impl Clone for MetalLocationSearch {
    fn clone(&self) -> Self {
        // Create a new instance with the same device
        MetalLocationSearch::new().expect("Failed to clone MetalLocationSearch")
    }
}

// Constants from the main crate
const MAP_MAX_X: f64 = 100000.0;
const MAP_MAX_Y: f64 = 100000.0; 