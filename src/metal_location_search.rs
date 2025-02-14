use metal;
use crate::poi::{Coordinate, POI};
use crate::generator::GeneratorType;
use crate::settlement::Settlement;
use crate::generator::Generator;
use std::error::Error;
use std::mem;
use std::fmt;

#[repr(C)]
#[derive(Copy, Clone)]
struct Candidate {
    x: f32,
    y: f32,
    score: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct MetalSettlement {
    x: f32,
    y: f32,
    population: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct MetalGenerator {
    x: f32,
    y: f32,
    size: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct BufferParams {
    num_settlements: u32,
    num_generators: u32,
    num_coastline_points: u32,
    gen_type: u32,
    penalty_radius: f32,
    size_penalty: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct float2 {
    x: f32,
    y: f32,
}

pub struct MetalLocationSearch {
    device: metal::Device,
    command_queue: metal::CommandQueue,
    pipeline_state: metal::ComputePipelineState,
}

impl MetalLocationSearch {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let device = metal::Device::system_default().ok_or("No Metal device found")?;
        let command_queue = device.new_command_queue();
        
        // Load and compile the Metal shader from embedded source
        let shader_source = include_str!("metal_location_search.metal");
        let compile_options = metal::CompileOptions::new();
        let library = device.new_library_with_source(shader_source, &compile_options)?;
        
        let function = library.get_function("computeSuitability", None)?;
        let pipeline_state = device.new_compute_pipeline_state_with_function(&function)?;

        Ok(Self {
            device,
            command_queue,
            pipeline_state,
        })
    }

    pub fn find_best_location(
        &self,
        gen_type: &GeneratorType,
        settlements: &[Settlement],
        generators: &[Generator],
        coastline_points: &[Coordinate],
        size_penalty: f32,
    ) -> Option<Coordinate> {
        // Create grid of candidate locations
        let grid_step = 1000.0;
        let num_x = (MAP_MAX_X / grid_step) as usize;
        let num_y = (MAP_MAX_Y / grid_step) as usize;
        let num_candidates = num_x * num_y;

        // Create candidates buffer
        let mut candidates: Vec<Candidate> = Vec::with_capacity(num_candidates);
        for i in 0..num_x {
            for j in 0..num_y {
                candidates.push(Candidate {
                    x: (i as f32) * grid_step as f32,
                    y: (j as f32) * grid_step as f32,
                    score: 0.0,
                });
            }
        }

        // Create Metal buffers
        let candidates_buffer = self.device.new_buffer_with_data(
            candidates.as_ptr() as *const _,
            (num_candidates * mem::size_of::<Candidate>()) as u64,
            metal::MTLResourceOptions::StorageModeShared,
        );

        let scores_buffer = self.device.new_buffer(
            (num_candidates * mem::size_of::<f32>()) as u64,
            metal::MTLResourceOptions::StorageModeShared,
        );

        // Convert settlements to Metal format
        let metal_settlements: Vec<MetalSettlement> = settlements.iter()
            .map(|s| MetalSettlement {
                x: s.get_coordinate().x as f32,
                y: s.get_coordinate().y as f32,
                population: s.get_population() as f32,
            })
            .collect();

        let settlements_buffer = self.device.new_buffer_with_data(
            metal_settlements.as_ptr() as *const _,
            (settlements.len() * mem::size_of::<MetalSettlement>()) as u64,
            metal::MTLResourceOptions::StorageModeShared,
        );

        // Convert generators to Metal format
        let metal_generators: Vec<MetalGenerator> = generators.iter()
            .map(|g| MetalGenerator {
                x: g.get_coordinate().x as f32,
                y: g.get_coordinate().y as f32,
                size: g.size as f32,
            })
            .collect();

        let generators_buffer = self.device.new_buffer_with_data(
            metal_generators.as_ptr() as *const _,
            (generators.len() * mem::size_of::<MetalGenerator>()) as u64,
            metal::MTLResourceOptions::StorageModeShared,
        );

        // Convert coastline points to Metal format
        let metal_coastline: Vec<float2> = coastline_points.iter()
            .map(|c| float2 { x: c.x as f32, y: c.y as f32 })
            .collect();

        let coastline_buffer = self.device.new_buffer_with_data(
            metal_coastline.as_ptr() as *const _,
            (coastline_points.len() * mem::size_of::<float2>()) as u64,
            metal::MTLResourceOptions::StorageModeShared,
        );

        // Set up parameters
        let params = BufferParams {
            num_settlements: settlements.len() as u32,
            num_generators: generators.len() as u32,
            num_coastline_points: coastline_points.len() as u32,
            gen_type: match gen_type {
                GeneratorType::OnshoreWind => 0,
                GeneratorType::OffshoreWind => 1,
                GeneratorType::DomesticSolar => 2,
                GeneratorType::CommercialSolar => 3,
                GeneratorType::UtilitySolar => 4,
                GeneratorType::Nuclear => 5,
                GeneratorType::CoalPlant => 6,
                GeneratorType::GasCombinedCycle => 7,
                GeneratorType::GasPeaker => 8,
                GeneratorType::Biomass => 9,
                GeneratorType::HydroDam => 10,
                GeneratorType::PumpedStorage => 11,
                GeneratorType::BatteryStorage => 12,
                GeneratorType::TidalGenerator => 13,
                GeneratorType::WaveEnergy => 14,
            },
            penalty_radius: match gen_type {
                GeneratorType::Nuclear => 12000.0,
                GeneratorType::CoalPlant | GeneratorType::GasCombinedCycle => 8000.0,
                GeneratorType::OnshoreWind | GeneratorType::OffshoreWind => 5000.0,
                GeneratorType::HydroDam | GeneratorType::PumpedStorage => 7000.0,
                GeneratorType::TidalGenerator | GeneratorType::WaveEnergy => 6000.0,
                _ => 3000.0,
            },
            size_penalty,
        };

        let params_buffer = self.device.new_buffer_with_data(
            &params as *const _ as *const _,
            mem::size_of::<BufferParams>() as u64,
            metal::MTLResourceOptions::StorageModeShared,
        );

        // Create command buffer and encoder
        let command_buffer = self.command_queue.new_command_buffer();
        let compute_encoder = command_buffer.new_compute_command_encoder();

        // Set up the compute pipeline
        compute_encoder.set_compute_pipeline_state(&self.pipeline_state);
        compute_encoder.set_buffer(0, Some(&candidates_buffer), 0);
        compute_encoder.set_buffer(1, Some(&scores_buffer), 0);
        compute_encoder.set_buffer(2, Some(&params_buffer), 0);
        compute_encoder.set_buffer(3, Some(&settlements_buffer), 0);
        compute_encoder.set_buffer(4, Some(&generators_buffer), 0);
        compute_encoder.set_buffer(5, Some(&coastline_buffer), 0);

        // Calculate grid size and threadgroup size
        let threadgroup_size = metal::MTLSize {
            width: 256,
            height: 1,
            depth: 1,
        };

        let grid_size = metal::MTLSize {
            width: num_candidates as u64,
            height: 1,
            depth: 1,
        };

        // Dispatch the compute kernel
        compute_encoder.dispatch_threads(grid_size, threadgroup_size);
        compute_encoder.end_encoding();

        // Execute and wait for completion
        command_buffer.commit();
        command_buffer.wait_until_completed();

        // Read back results
        let scores_ptr = scores_buffer.contents() as *const f32;
        let mut best_score = 0.0;
        let mut best_idx = None;

        for i in 0..num_candidates {
            let score = unsafe { *scores_ptr.add(i) };
            if score > best_score {
                best_score = score;
                best_idx = Some(i);
            }
        }

        best_idx.map(|idx| {
            let candidate = &candidates[idx];
            Coordinate::new(candidate.x as f64, candidate.y as f64)
        })
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