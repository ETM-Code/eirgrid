import requests
import json
import time
from shapely.geometry import LineString, box, Point
import numpy as np
import os
import dotenv
import googlemaps

dotenv.load_dotenv()
GoogleAPIKey = os.getenv('GOOGLE_KEY')

# Ireland's approximate bounding box
IRELAND_BBOX = {
    'min_lat': 51.4,  # Southernmost point (Mizen Head)
    'max_lat': 55.4,  # Northernmost point (Malin Head) 
    'min_lon': -10.6, # Westernmost point (Dunmore Head)
    'max_lon': -5.9   # Easternmost point (Wicklow Head)
}

def fetch_coastline():
    """Fetch Ireland's coastline using Overpass API"""
    overpass_url = "http://overpass-api.de/api/interpreter"
    
    # Query to get Ireland's coastline
    query = f"""
    [out:json];
    way["natural"="coastline"](
        {IRELAND_BBOX['min_lat']},{IRELAND_BBOX['min_lon']},
        {IRELAND_BBOX['max_lat']},{IRELAND_BBOX['max_lon']}
    );
    (._;>;);
    out body;
    """
    
    response = requests.post(overpass_url, data=query)
    return response.json()

def process_coastline_data(data):
    """Process the coastline data into a list of coordinates"""
    nodes = {node['id']: (node['lon'], node['lat']) for node in data['elements'] if node['type'] == 'node'}
    coastline_coords = []
    
    for element in data['elements']:
        if element['type'] == 'way' and 'nodes' in element:
            way_coords = [nodes[node_id] for node_id in element['nodes']]
            coastline_coords.extend(way_coords)
    
    return coastline_coords

def sample_coastline(coords, num_samples=100):
    """Sample points along the coastline"""
    line = LineString(coords)
    distances = np.linspace(0, line.length, num_samples)
    points = [line.interpolate(distance) for distance in distances]
    return [(point.x, point.y) for point in points]

def calculate_grid_transformation():
    """Calculate transformation between lat/lon and 50000x50000 grid"""
    lon_range = IRELAND_BBOX['max_lon'] - IRELAND_BBOX['min_lon']
    lat_range = IRELAND_BBOX['max_lat'] - IRELAND_BBOX['min_lat']
    
    # Scale factors to transform to 50000x50000 grid
    scale_x = 50000.0 / lon_range
    scale_y = 50000.0 / lat_range
    
    return {
        'origin': (IRELAND_BBOX['min_lon'], IRELAND_BBOX['min_lat']),
        'scale': (scale_x, scale_y)
    }

def transform_coordinates(coords, transform):
    """Transform lat/lon coordinates to grid coordinates"""
    origin = transform['origin']
    scale = transform['scale']
    
    transformed = []
    for lon, lat in coords:
        x = (lon - origin[0]) * scale[0]
        y = (lat - origin[1]) * scale[1]
        transformed.append((x, y))
    
    return transformed

def main():
    # Create output directory if it doesn't exist
    os.makedirs('../mapData/sourceData', exist_ok=True)
    
    # Fetch coastline data
    print("Fetching coastline data...")
    coastline_data = fetch_coastline()
    
    # Process coastline coordinates
    print("Processing coastline data...")
    coastline_coords = process_coastline_data(coastline_data)
    
    # Sample points along coastline
    print("Sampling coastline...")
    sampled_points = sample_coastline(coastline_coords, num_samples=200)
    
    # Calculate transformation
    transform = calculate_grid_transformation()
    
    # Transform sampled points to grid coordinates
    grid_points = transform_coordinates(sampled_points, transform)
    
    # Save transformation parameters
    transform_data = {
        'bbox': IRELAND_BBOX,
        'transform': transform,
        'grid_size': 50000.0
    }
    
    with open('../mapData/sourceData/grid_transform.json', 'w') as f:
        json.dump(transform_data, f, indent=2)
    
    # Save coastline points
    coastline_data = {
        'original_coords': sampled_points,
        'grid_coords': grid_points
    }
    
    with open('../mapData/sourceData/coastline_points.json', 'w') as f:
        json.dump(coastline_data, f, indent=2)
    
    print("Data saved to mapData directory")

if __name__ == "__main__":
    main() 