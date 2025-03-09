use crate::data::poi::Coordinate;
use crate::config::constants::{MAP_MAX_X, MAP_MAX_Y, GRID_CELL_SIZE};
use std::fmt;

#[derive(Clone, Debug)]
pub struct QuadTreeNode {
    boundary: Boundary,
    children: Option<Box<[QuadTreeNode; 4]>>,
    suitability_scores: Vec<(GeneratorSuitabilityType, f64)>,
    total_occupancy: f64,
}

#[derive(Clone, Debug)]
pub struct Boundary {
    center: Coordinate,
    half_width: f64,
    half_height: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GeneratorSuitabilityType {
    Onshore,
    Offshore,
    Urban,
    Rural,
    Coastal,
    Protected,
}

impl QuadTreeNode {
    pub fn new(center: Coordinate, half_width: f64, half_height: f64) -> Self {
        Self {
            boundary: Boundary {
                center,
                half_width,
                half_height,
            },
            children: None,
            suitability_scores: Vec::new(),
            total_occupancy: 0.0,
        }
    }

    pub fn subdivide(&mut self) {
        let x = self.boundary.center.x;
        let y = self.boundary.center.y;
        let hw = self.boundary.half_width / 2.0;
        let hh = self.boundary.half_height / 2.0;

        let children = Box::new([
            // Northwest
            QuadTreeNode::new(
                Coordinate::new(x - hw, y + hh),
                hw,
                hh,
            ),
            // Northeast
            QuadTreeNode::new(
                Coordinate::new(x + hw, y + hh),
                hw,
                hh,
            ),
            // Southwest
            QuadTreeNode::new(
                Coordinate::new(x - hw, y - hh),
                hw,
                hh,
            ),
            // Southeast
            QuadTreeNode::new(
                Coordinate::new(x + hw, y - hh),
                hw,
                hh,
            ),
        ]);

        self.children = Some(children);
    }

    pub fn contains_point(&self, point: &Coordinate) -> bool {
        point.x >= self.boundary.center.x - self.boundary.half_width &&
        point.x <= self.boundary.center.x + self.boundary.half_width &&
        point.y >= self.boundary.center.y - self.boundary.half_height &&
        point.y <= self.boundary.center.y + self.boundary.half_height
    }

    pub fn get_suitability_score(&self, suitability_type: GeneratorSuitabilityType) -> f64 {
        self.suitability_scores
            .iter()
            .find(|(t, _)| *t == suitability_type)
            .map(|(_, score)| *score)
            .unwrap_or(0.0)
    }

    pub fn update_suitability(&mut self, suitability_type: GeneratorSuitabilityType, score: f64) {
        if let Some(existing) = self.suitability_scores
            .iter_mut()
            .find(|(t, _)| *t == suitability_type) {
            existing.1 = score;
        } else {
            self.suitability_scores.push((suitability_type, score));
        }

        // Propagate to children if they exist
        if let Some(children) = &mut self.children {
            for child in children.iter_mut() {
                child.update_suitability(suitability_type, score);
            }
        }
    }

    pub fn update_occupancy(&mut self, delta: f64) {
        self.total_occupancy = (self.total_occupancy + delta).max(0.0);
        
        if let Some(children) = &mut self.children {
            for child in children.iter_mut() {
                child.update_occupancy(delta / 4.0); // Distribute evenly among children
            }
        }
    }

    pub fn intersects_circle(&self, center: &Coordinate, radius: f64) -> bool {
        let dx = center.x - self.boundary.center.x;
        let dy = center.y - self.boundary.center.y;
        let distance_sq = dx * dx + dy * dy;
        let node_radius = self.boundary.half_width.min(self.boundary.half_height);
        let radii_sum = radius + node_radius;
        distance_sq <= radii_sum * radii_sum
    }
}

#[derive(Clone)]
pub struct SpatialIndex {
    root: QuadTreeNode,
}

// Manual Debug implementation for SpatialIndex
impl fmt::Debug for SpatialIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpatialIndex")
            .field("root", &self.root)
            .finish()
    }
}

impl SpatialIndex {
    pub fn new() -> Self {
        Self {
            root: QuadTreeNode::new(
                Coordinate::new(MAP_MAX_X / 2.0, MAP_MAX_Y / 2.0),
                MAP_MAX_X / 2.0,
                MAP_MAX_Y / 2.0,
            ),
        }
    }

    pub fn find_best_location(
        &self,
        suitability_type: GeneratorSuitabilityType,
        min_score: f64,
    ) -> Option<Coordinate> {
        let mut best_node = None;
        let mut best_score = min_score;

        self.search_suitable_locations(&self.root, suitability_type, min_score, &mut best_node, &mut best_score);

        best_node.map(|node| node.boundary.center.clone())
    }

    fn search_suitable_locations<'a>(
        &self,
        node: &'a QuadTreeNode,
        suitability_type: GeneratorSuitabilityType,
        min_score: f64,
        best_node: &mut Option<&'a QuadTreeNode>,
        best_score: &mut f64,
    ) {
        let current_score = node.get_suitability_score(suitability_type);
        
        if current_score < min_score || node.total_occupancy >= 1.0 {
            return;
        }

        if current_score > *best_score {
            *best_node = Some(node);
            *best_score = current_score;
        }

        if let Some(children) = &node.children {
            for child in children.iter() {
                self.search_suitable_locations(child, suitability_type, min_score, best_node, best_score);
            }
        }
    }

    pub fn update_region(
        &mut self,
        center: &Coordinate,
        radius: f64,
        suitability_type: GeneratorSuitabilityType,
        score: f64,
    ) {
        let mut nodes_to_update = vec![&mut self.root];

        while let Some(node) = nodes_to_update.pop() {
            // If this node's boundary doesn't intersect with our circle of influence, skip it
            if !node.intersects_circle(center, radius) {
                continue;
            }

            // Update this node's suitability
            node.update_suitability(suitability_type, score);

            // If this node is smaller than our minimum cell size, don't subdivide further
            if node.boundary.half_width <= GRID_CELL_SIZE / 2.0 {
                continue;
            }

            // Ensure we have children to recurse into
            if node.children.is_none() {
                node.subdivide();
            }

            // Add children to the stack
            if let Some(children) = &mut node.children {
                for child in children.iter_mut() {
                    nodes_to_update.push(child);
                }
            }
        }
    }
} 