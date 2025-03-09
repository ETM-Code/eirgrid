#include <metal_stdlib>
using namespace metal;

struct Candidate {
    float x;
    float y;
    float score;
};

struct Settlement {
    float x;
    float y;
    float population;
};

struct Generator {
    float x;
    float y;
    float size;
};

struct BufferParams {
    uint num_settlements;
    uint num_generators;
    uint num_coastline_points;
    uint gen_type;
    float penalty_radius;
    float size_penalty;
};

// A standard ray‐casting (even–odd rule) algorithm for point‑in‑polygon.
bool is_point_inside_polygon(float2 point, const device float2* polygon, uint num_points) {
    bool inside = false;
    for (uint i = 0, j = num_points - 1; i < num_points; j = i++) {
        float2 pi = polygon[i];
        float2 pj = polygon[j];
        if (((pi.y > point.y) != (pj.y > point.y)) &&
            (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y + 0.00001) + pi.x)) {
            inside = !inside;
        }
    }
    return inside;
}

// Determine if a candidate is within any settlement's "urban area"
// The urban area is defined as a circle of radius sqrt(population)*5.0.
bool is_urban_area(float2 candidate, const device Settlement* settlements, uint num_settlements) {
    for (uint i = 0; i < num_settlements; i++) {
        float2 s = float2(settlements[i].x, settlements[i].y);
        float radius = sqrt(settlements[i].population) * 5.0;
        if (distance(candidate, s) < radius) {
            return true;
        }
    }
    return false;
}

// Compute minimum distance from candidate to the coastline points.
float min_distance_to_polygon(float2 candidate, const device float2* coastline, uint num_points) {
    float min_dist = FLT_MAX;
    for (uint i = 0; i < num_points; i++) {
        float d = distance(candidate, coastline[i]);
        if (d < min_dist) {
            min_dist = d;
        }
    }
    return min_dist;
}

bool is_coastal_region(float2 candidate, const device float2* coastline, uint num_points) {
    float coastal_distance = 8000.0;
    float d = min_distance_to_polygon(candidate, coastline, num_points);
    return (d <= coastal_distance);
}

bool is_near_water(float2 candidate, const device float2* coastline, uint num_points) {
    float water_distance = 5000.0;
    float d = min_distance_to_polygon(candidate, coastline, num_points);
    return (d <= water_distance);
}

float get_nearby_population(float2 candidate, const device Settlement* settlements, uint num_settlements) {
    float total_population = 0.0;
    float radius = 5000.0;
    
    for (uint i = 0; i < num_settlements; i++) {
        float2 s = float2(settlements[i].x, settlements[i].y);
        if (distance(candidate, s) <= radius) {
            total_population += settlements[i].population;
        }
    }
    
    return total_population;
}

float calculate_nearby_generator_penalty(float2 candidate, const device Generator* generators, uint num_generators, float penalty_radius) {
    float penalty = 0.0;
    for (uint i = 0; i < num_generators; i++) {
        float2 g = float2(generators[i].x, generators[i].y);
        float d = distance(candidate, g);
        if (d < penalty_radius) {
            penalty += 0.1 * generators[i].size / (1.0 + d / 1000.0);
        }
    }
    return penalty;
}

// Calculate the suitability score based on generator type.
float calculate_suitability(float2 candidate,
                          const device Settlement* settlements, uint num_settlements,
                          const device Generator* generators, uint num_generators,
                          const device float2* coastline, uint num_coastline_points,
                          uint gen_type, float penalty_radius, float size_penalty) {
    float base_score = 0.0;
    bool is_urban = is_urban_area(candidate, settlements, num_settlements);
    bool is_coastal = is_coastal_region(candidate, coastline, num_coastline_points);
    bool is_water = !is_point_inside_polygon(candidate, coastline, num_coastline_points);
    bool near_water = is_near_water(candidate, coastline, num_coastline_points);
    float nearby_pop = get_nearby_population(candidate, settlements, num_settlements);
    
    switch (gen_type) {
        case 0: // OnshoreWind
            if (is_urban) {
                base_score = 0.0;
            } else if (is_coastal) {
                base_score = 0.7;
            } else {
                base_score = 0.5;
            }
            if (is_water) base_score = 0.0;
            break;
            
        case 1: // OffshoreWind
            if (!is_water) {
                base_score = 0.0;
            } else {
                float shore_distance = min_distance_to_polygon(candidate, coastline, num_coastline_points);
                base_score = (shore_distance < 2000.0) ? 0.3 : ((shore_distance > 10000.0) ? 0.5 : 0.7);
            }
            break;
            
        case 2: // DomesticSolar
        case 3: // CommercialSolar
            if (is_urban) {
                base_score = 0.6;
            } else {
                base_score = 0.4;
            }
            if (is_water) base_score = 0.0;
            break;
            
        case 4: // UtilitySolar
            if (is_urban) {
                base_score = 0.3;
            } else {
                base_score = 0.5;
            }
            if (is_water) base_score = 0.0;
            break;
            
        case 5: // Nuclear
            if (is_urban || is_water || nearby_pop > 10000) {
                base_score = 0.0;
            } else if (near_water) {
                base_score = 0.7;
            } else {
                base_score = 0.4;
            }
            break;
            
        case 6: // CoalPlant
        case 7: // GasCombinedCycle
        case 8: // GasPeaker
            if (is_urban || is_water) {
                base_score = 0.0;
            } else if (near_water) {
                base_score = 0.6;
            } else {
                base_score = 0.4;
            }
            break;
            
        case 9: // Biomass
            if (is_water) {
                base_score = 0.0;
            } else if (is_urban) {
                base_score = 0.3;
            } else {
                base_score = 0.5;
            }
            break;
            
        case 10: // HydroDam
        case 11: // PumpedStorage
            if (!near_water || is_urban) {
                base_score = 0.0;
            } else {
                base_score = 0.7;
            }
            break;
            
        case 12: // BatteryStorage
            if (is_water) {
                base_score = 0.0;
            } else if (is_urban) {
                base_score = 0.6;
            } else {
                base_score = 0.4;
            }
            break;
            
        case 13: // TidalGenerator
        case 14: // WaveEnergy
            if (!is_water) {
                base_score = 0.0;
            } else {
                float shore_distance = min_distance_to_polygon(candidate, coastline, num_coastline_points);
                if (shore_distance < 5000.0) {
                    base_score = 0.8;
                } else {
                    base_score = 0.4;
                }
            }
            break;
    }
    
    // Apply penalties
    float nearby_penalty = calculate_nearby_generator_penalty(candidate, generators, num_generators, penalty_radius);
    float final_score = base_score - nearby_penalty - size_penalty;
    
    // Add coastal bonus for certain types
    if (is_coastal && (gen_type == 0 || gen_type == 13 || gen_type == 14)) {
        final_score *= 1.2;
    }
    
    return max(0.0f, final_score);
}

kernel void computeSuitability(
    device Candidate* candidates [[ buffer(0) ]],
    device float* out_scores [[ buffer(1) ]],
    constant BufferParams& params [[ buffer(2) ]],
    device Settlement* settlements [[ buffer(3) ]],
    device Generator* generators [[ buffer(4) ]],
    device float2* coastline [[ buffer(5) ]],
    uint id [[ thread_position_in_grid ]]
) {
    Candidate candidate = candidates[id];
    float2 pos = float2(candidate.x, candidate.y);
    float score = calculate_suitability(pos,
                                      settlements, params.num_settlements,
                                      generators, params.num_generators,
                                      coastline, params.num_coastline_points,
                                      params.gen_type, params.penalty_radius,
                                      params.size_penalty);
    candidates[id].score = score;
    out_scores[id] = score;
} 