#!/usr/bin/env python3

import json
from pathlib import Path
import numpy as np
from ireland_bounds import IRELAND_BBOX

def load_settlements():
    """Load settlements from the existing JSON file"""
    try:
        with open('../mapData/sourceData/settlements.json', 'r') as f:
            return json.load(f)
    except FileNotFoundError:
        print("Error: settlements.json not found")
        raise SystemExit(1)

def clean_settlement_name(name):
    """Clean settlement name to match Rust format"""
    # Remove everything after comma
    name = name.split(',')[0].strip()
    
    # Remove common qualifiers
    replacements = {
        ' Urban': '',
        ' Rural': '',
        ' Town': '',
        ' Village': '',
        ' ED': '',
        ' (North)': '',
        ' (South)': '',
        ' (East)': '',
        ' (West)': '',
    }
    
    for old, new in replacements.items():
        name = name.replace(old, new)
    
    return name.strip()

def transform_coordinates(lat, lon):
    """Transform lat/lon coordinates to our 0-100000 grid system"""
    # Calculate position as percentage within Ireland's bounds
    lat_range = IRELAND_BBOX["max_lat"] - IRELAND_BBOX["min_lat"]
    lon_range = IRELAND_BBOX["max_lon"] - IRELAND_BBOX["min_lon"]
    
    lat_percent = (lat - IRELAND_BBOX["min_lat"]) / lat_range
    lon_percent = (lon - IRELAND_BBOX["min_lon"]) / lon_range
    
    # Convert to 0-100000 range
    x = lon_percent * 100000
    y = lat_percent * 100000
    
    return x, y

def calculate_power_usage(population):
    """Calculate power usage based on population"""
    # From Rust examples: ~1.67 kW per person
    # Dublin: 1,200,000 people -> 2000.0 MW
    # Cork: 190,000 people -> 350.0 MW
    # This gives us roughly 1.67 kW per person
    return (population * 1.67) / 1000  # Convert to MW

def transform_settlements():
    """Transform settlements data to Rust format"""
    print("Loading settlements data...")
    data = load_settlements()
    
    print("\nTransforming settlements...")
    transformed = []
    
    for settlement in data['settlements']:
        name = clean_settlement_name(settlement['name'])
        x, y = transform_coordinates(settlement['lat'], settlement['lon'])
        population = int(settlement['population'])
        power_usage = calculate_power_usage(population)
        
        transformed.append({
            'name': name,
            'coordinate': {
                'x': x,
                'y': y
            },
            'population': population,
            'power_usage': power_usage
        })
    
    # Sort by population (largest first) to match Rust example
    transformed.sort(key=lambda s: s['population'], reverse=True)
    
    # Validate transformations
    print("\nValidating transformations...")
    for settlement in transformed[:5]:
        print(f"\nSettlement: {settlement['name']}")
        print(f"Population: {settlement['population']:,}")
        print(f"Coordinates: ({settlement['coordinate']['x']:.1f}, {settlement['coordinate']['y']:.1f})")
        print(f"Power Usage: {settlement['power_usage']:.1f} MW")
    
    # Save transformed data
    output_file = '../mapData/sourceData/settlements_rust.json'
    with open(output_file, 'w') as f:
        json.dump({
            'settlements': transformed,
            'metadata': {
                'total_settlements': len(transformed),
                'total_population': sum(s['population'] for s in transformed),
                'total_power_usage': sum(s['power_usage'] for s in transformed),
                'coordinate_system': {
                    'min_x': 0,
                    'max_x': 100000,
                    'min_y': 0,
                    'max_y': 100000
                }
            }
        }, f, indent=2)
    
    print(f"\nTransformed data saved to {output_file}")
    print(f"Total settlements: {len(transformed)}")
    print(f"Total population: {sum(s['population'] for s in transformed):,}")
    print(f"Total power usage: {sum(s['power_usage'] for s in transformed):.1f} MW")

if __name__ == "__main__":
    transform_settlements() 