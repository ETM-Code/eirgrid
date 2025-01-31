#!/usr/bin/env python3

import requests
import json
import time
from shapely.geometry import Point, MultiPoint
from shapely.ops import nearest_points
import numpy as np
from collections import defaultdict
import os
from ireland_bounds import IRELAND_BBOX, calculate_grid_transformation, transform_coordinates
import difflib
import csv
import heapq
import pickle
from pathlib import Path
import argparse

# Add checkpoint directory
CHECKPOINT_DIR = Path('checkpoints')
CHECKPOINT_DIR.mkdir(exist_ok=True)

def parse_args():
    """Parse command line arguments"""
    parser = argparse.ArgumentParser(description='Process settlements data')
    parser.add_argument('--append', action='store_true',
                       help='Append mode: process only missing settlements and append to existing data')
    return parser.parse_args()

def save_checkpoint(stage, data):
    """Save checkpoint data to a file"""
    checkpoint_file = CHECKPOINT_DIR / f"{stage}.pickle"
    with open(checkpoint_file, 'wb') as f:
        pickle.dump(data, f)
    print(f"\nCheckpoint saved: {stage}")

def load_checkpoint(stage):
    """Load checkpoint data from a file"""
    checkpoint_file = CHECKPOINT_DIR / f"{stage}.pickle"
    if checkpoint_file.exists():
        with open(checkpoint_file, 'rb') as f:
            data = pickle.load(f)
        print(f"\nCheckpoint loaded: {stage}")
        return data
    return None

def clear_checkpoints():
    """Clear all checkpoint files"""
    for checkpoint_file in CHECKPOINT_DIR.glob("*.pickle"):
        checkpoint_file.unlink()
    print("\nAll checkpoints cleared")

def clean_search_name(name):
    """Clean settlement name for Google Maps search while preserving administrative regions"""
    # Split at comma but preserve the administrative region
    parts = [part.strip() for part in name.split(',')]
    name = parts[0]
    admin_region = parts[1] if len(parts) > 1 else ''
    
    # Remove electoral division qualifiers while preserving administrative region
    name = (name
           .replace('-Blakestown', '')
           .replace('-Esker', '')
           .replace('-Knockmaroon', '')
           .replace('Rural', '')
           .replace('Urban', '')
           .replace('/Monkstown Rural/Douglas', '')  # Special case for Carrigaline
           .replace('(South)', '')
           .replace('(North)', '')
           .replace('(East)', '')
           .replace('(West)', ''))
    
    # Remove any remaining parentheses and their contents
    name = ' '.join(part for part in name.split() if '(' not in part and ')' not in part)
    
    # Combine with administrative region for search
    if admin_region:
        return f"{name.strip()}, {admin_region}"
    return name.strip()

def fetch_google_maps_location(name, api_key):
    """Fetch a specific location from Google Maps API"""
    base_url = "https://places.googleapis.com/v1/places:searchText"
    
    # Clean the name for searching
    search_name = clean_search_name(name)
    
    # Add Ireland to the search query to bias results
    search_query = f"{search_name}, Ireland"
    
    print(f"Searching for: {search_query}")  # Debug print
    
    # Define the request body
    request_body = {
        "textQuery": search_query,
        "locationBias": {
            "rectangle": {
                "low": {
                    "latitude": IRELAND_BBOX["min_lat"],
                    "longitude": IRELAND_BBOX["min_lon"]
                },
                "high": {
                    "latitude": IRELAND_BBOX["max_lat"],
                    "longitude": IRELAND_BBOX["max_lon"]
                }
            }
        },
        "languageCode": "en",
        "regionCode": "IE",  # Set region code to Ireland
        "strictTypeFiltering": False  # Allow more flexible type matching
    }
    
    headers = {
        'Content-Type': 'application/json',
        'X-Goog-Api-Key': api_key,
        'X-Goog-FieldMask': 'places.displayName,places.location,places.formattedAddress,places.types'
    }
    
    try:
        response = requests.post(base_url, json=request_body, headers=headers)
        response.raise_for_status()
        data = response.json()
        
        if 'places' in data and data['places']:
            # Get the first (most relevant) result
            place = data['places'][0]
            
            # Debug print
            print(f"Found match: {place.get('displayName', {}).get('text', 'No name')} at {place.get('formattedAddress', 'No address')}")
            
            lat = place['location']['latitude']
            lon = place['location']['longitude']
            return {
                'lat': lat,
                'lon': lon
            }
        return None
        
    except requests.exceptions.RequestException as e:
        print(f"Error fetching location '{name}' from Google Maps: {e}")
        if hasattr(e.response, 'text'):
            print(f"Response content: {e.response.text}")
        return None

def fetch_cso_settlements():
    """Fetch settlement data from Population.csv file"""
    try:
        print("Reading settlement data from Population.csv...")
        settlements = []
        
        with open('Population.csv', 'r', encoding='utf-8-sig') as f:  # utf-8-sig handles the BOM
            reader = csv.DictReader(f)
            for row in reader:
                # Only process population statistics (F1011C01)
                if row['STATISTIC'] != 'F1011C01':
                    continue
                    
                # Extract name and population from CSV
                name = row['Electoral Divisions'].strip()
                value = row['VALUE'].strip()
                
                # Skip rows with empty values
                if not value:
                    continue
                    
                # Convert to float first, then round to integer
                population = round(float(value))
                
                settlements.append({
                    'name': name,
                    'population': population
                })
        
        if not settlements:
            raise RuntimeError("No settlement data found in Population.csv")
            
        print(f"Found {len(settlements)} settlements in CSV data")
        return settlements
        
    except FileNotFoundError:
        print("Error: Population.csv file not found")
        raise SystemExit(1)
    except (KeyError, ValueError) as e:
        print(f"Error processing CSV data: {e}")
        print(f"Problematic row: {row if 'row' in locals() else 'Unknown'}")
        print(f"Available columns: {list(row.keys()) if 'row' in locals() else 'Unknown'}")
        raise SystemExit(1)

def normalize_name(name):
    """Normalize place names for better matching between CSO and OSM data"""
    # Convert to lowercase
    name = name.lower()
    
    # Remove common prefixes/suffixes and special characters
    replacements = {
        'city and suburbs': '',
        'legal town': '',
        'town': '',
        'village': '',
        'suburb': '',
        'townland': '',
        '(part)': '',
        'county': '',
        'co.': '',
        'rural': '',
        'urban': '',
    }
    
    for old, new in replacements.items():
        name = name.replace(old, new)
    
    # Remove special characters and extra whitespace
    name = ''.join(c for c in name if c.isalnum() or c.isspace())
    name = ' '.join(name.split())
    
    return name

def match_settlements(cso_settlements, osm_locations):
    """Match CSO settlements with OSM locations"""
    matched_settlements = []
    unmatched_settlements = []
    
    for settlement in cso_settlements:
        # Try exact match first
        location = osm_locations.get(settlement['normalized_name'])
        
        if not location:
            # Try fuzzy matching
            best_match = None
            best_ratio = 0
            norm_name = settlement['normalized_name']
            
            for osm_norm_name, osm_data in osm_locations.items():
                # Calculate similarity ratio
                ratio = difflib.SequenceMatcher(None, norm_name, osm_norm_name).ratio()
                if ratio > 0.85 and ratio > best_ratio:  # 85% similarity threshold
                    best_ratio = ratio
                    best_match = osm_data
            
            location = best_match
        
        if location:
            matched_settlements.append({
                'name': settlement['name'],
                'population': settlement['population'],
                'lat': location['lat'],
                'lon': location['lon'],
                'type': location['type']
            })
        else:
            unmatched_settlements.append(settlement['name'])
    
    if unmatched_settlements:
        print(f"\nWarning: Could not find coordinates for {len(unmatched_settlements)} settlements:")
        print(", ".join(unmatched_settlements[:10]))
        if len(unmatched_settlements) > 10:
            print(f"...and {len(unmatched_settlements) - 10} more")
    
    return matched_settlements

def calculate_distance(point1, point2):
    """Calculate distance between two points in kilometers"""
    # Convert to radians
    lat1, lon1 = np.radians(point1)
    lat2, lon2 = np.radians(point2)
    
    # Haversine formula
    dlat = lat2 - lat1
    dlon = lon2 - lon1
    a = np.sin(dlat/2)**2 + np.cos(lat1) * np.cos(lat2) * np.sin(dlon/2)**2
    c = 2 * np.arcsin(np.sqrt(a))
    r = 6371  # Earth's radius in kilometers
    
    return c * r

def get_admin_region(name):
    """Extract administrative region from settlement name"""
    parts = [part.strip() for part in name.split(',')]
    return parts[1] if len(parts) > 1 else ''

def group_settlements(settlements, max_distance=30):
    """Group settlements that are within max_distance km of each other and in the same administrative region"""
    groups = []
    used = set()
    
    # Sort settlements by population (largest to smallest)
    sorted_settlements = sorted(settlements, key=lambda x: x['population'], reverse=True)
    
    for i, settlement1 in enumerate(sorted_settlements):
        if i in used:
            continue
            
        admin_region1 = get_admin_region(settlement1['name'])
        group = [settlement1]
        used.add(i)
        
        for j, settlement2 in enumerate(sorted_settlements):
            if j in used:
                continue
                
            admin_region2 = get_admin_region(settlement2['name'])
            
            # Skip if not in same administrative region
            if admin_region1 != admin_region2:
                continue
                
            # Check if settlement2 is within max_distance of any settlement in the group
            for s in group:
                dist = calculate_distance(
                    (s['lat'], s['lon']),
                    (settlement2['lat'], settlement2['lon'])
                )
                if dist <= max_distance:
                    group.append(settlement2)
                    used.add(j)
                    break
        
        groups.append(group)
    
    return groups

def process_settlement_group(group):
    """Process a group of settlements into a single settlement for simulation"""
    # Calculate weighted center based on population
    total_pop = sum(s['population'] for s in group)
    weighted_lat = sum(s['lat'] * s['population'] for s in group) / total_pop
    weighted_lon = sum(s['lon'] * s['population'] for s in group) / total_pop
    
    # Use the name of the largest settlement in the group
    main_settlement = max(group, key=lambda x: x['population'])
    
    # Ensure all settlements are listed as constituents, including the main settlement
    constituent_settlements = []
    for settlement in group:
        # Add the current settlement
        constituent_settlements.append(settlement['name'])
        # Add any existing constituents
        if 'constituent_settlements' in settlement:
            constituent_settlements.extend(settlement['constituent_settlements'])
    
    # Remove duplicates while preserving order
    constituent_settlements = list(dict.fromkeys(constituent_settlements))
    
    return {
        'name': main_settlement['name'],
        'lat': weighted_lat,
        'lon': weighted_lon,
        'population': total_pop,
        'constituent_settlements': constituent_settlements
    }

def clean_settlement_name(name):
    """Clean settlement name by removing administrative qualifiers and special characters"""
    # Remove county/administrative parts after comma
    name = name.split(',')[0].strip()
    
    # List of words to remove
    remove_words = [
        'urban', 'rural', 'town',
        'electoral division', 'ed', 'district',
        'north', 'south', 'east', 'west',
        'upper', 'lower'
    ]
    
    # Convert to lowercase for processing
    name = name.lower()
    
    # Remove specified words when they are whole words
    for word in remove_words:
        name = ' '.join(part for part in name.split() if part != word)
    
    # Clean special characters
    name = (name
            .replace('-', ' ')
            .replace('/', ' ')
            .replace("'", '')
            .replace('"', ''))
    
    # Remove extra whitespace
    name = ' '.join(name.split())
    
    return name

def fetch_locations_batch(names, batch_size=50):
    """Fetch multiple locations from Google Maps API with checkpoint support"""
    # Load API key from environment variable
    api_key = os.getenv('GOOGLE_MAPS_API_KEY')
    if not api_key:
        raise ValueError("GOOGLE_MAPS_API_KEY environment variable not set")
    
    # Try to load checkpoint
    results = load_checkpoint('locations')
    if results is None:
        results = {}
    
    # Filter out names that are already processed
    names_to_process = [name for name in names if name not in results]
    if not names_to_process:
        print("All locations already processed, using checkpoint data")
        return results
    
    total_batches = (len(names_to_process) + batch_size - 1) // batch_size
    print(f"\nProcessing {len(names_to_process)} remaining settlements in {total_batches} batches of {batch_size}")
    
    try:
        # Process names in batches
        for i in range(0, len(names_to_process), batch_size):
            batch = names_to_process[i:i + batch_size]
            current_batch = i // batch_size + 1
            print(f"\nBatch {current_batch}/{total_batches}")
            print(f"Processing settlements: {', '.join(batch)}")
            
            # Process each name in the batch
            for name in batch:
                location = fetch_google_maps_location(name, api_key)
                if location:
                    results[name] = location
                time.sleep(0.1)  # Small delay to respect API rate limits
            
            # Save checkpoint after each batch
            save_checkpoint('locations', results)
            
            # Report matches for this batch
            matched = set(name for name in batch if name in results)
            unmatched = set(batch) - matched
            
            print(f"Found coordinates for {len(matched)}/{len(batch)} settlements")
            if matched:
                print("Matched settlements:", ", ".join(matched))
            if unmatched:
                print("Unmatched settlements:", ", ".join(unmatched))
            
            # Add a small delay between batches
            time.sleep(1)
        
        # Final summary
        print(f"\nGoogle Maps Processing Complete:")
        print(f"Total settlements processed: {len(names)}")
        print(f"Successfully found coordinates: {len(results)}")
        print(f"Failed to find coordinates: {len(names) - len(results)}")
        
        return results
        
    except Exception as e:
        print(f"\nError during location fetching: {e}")
        print("Progress saved in checkpoint. Run script again to resume.")
        raise

def pregroup_small_settlements(settlements, min_population=1000, max_settlements=1000):
    """Group settlements until we have max_settlements or fewer groups"""
    print(f"\nStarting with {len(settlements)} settlements")
    print(f"Target: {max_settlements} settlements or fewer")
    
    # Sort settlements by population (largest to smallest)
    settlements_by_pop = sorted(settlements, key=lambda x: x['population'], reverse=True)
    print("Sorted settlements by population")
    
    # Track total population for validation
    total_initial_population = sum(s['population'] for s in settlements)
    
    # If we have fewer settlements than max_settlements, return them all
    if len(settlements) <= max_settlements:
        print("Already have fewer settlements than target, returning all")
        return settlements
    
    # Always keep settlements above the minimum population
    result = [s for s in settlements_by_pop if s['population'] >= min_population]
    remaining = [s for s in settlements_by_pop if s['population'] < min_population]
    
    print(f"Found {len(result)} settlements above minimum population of {min_population:,}")
    print(f"Have {len(remaining)} settlements to process")
    
    # Group remaining settlements by administrative region
    admin_regions = defaultdict(list)
    for settlement in remaining:
        admin_region = get_admin_region(settlement['name'])
        admin_regions[admin_region].append(settlement)
    
    # Process each administrative region separately
    for admin_region, region_settlements in admin_regions.items():
        print(f"\nProcessing {len(region_settlements)} settlements in {admin_region if admin_region else 'No Region'}")
        
        # Sort region settlements by population
        region_settlements.sort(key=lambda x: x['population'], reverse=True)
        
        while region_settlements:
            current = region_settlements.pop(0)
            group = [current]
            total_pop = current['population']
            
            # Try to find more settlements in the same region to group with
            i = 0
            while i < len(region_settlements):
                if total_pop >= min_population:
                    break
                
                # Add next settlement in the region
                group.append(region_settlements.pop(i))
                total_pop += group[-1]['population']
            
            # Create merged settlement
            if len(group) > 1:
                main_settlement = max(group, key=lambda x: x['population'])
                constituent_settlements = []
                for s in group:
                    constituent_settlements.append(s['name'])
                    if 'constituent_settlements' in s:
                        constituent_settlements.extend(s['constituent_settlements'])
                
                # Remove duplicates while preserving order
                constituent_settlements = list(dict.fromkeys(constituent_settlements))
                
                merged = {
                    'name': main_settlement['name'],
                    'population': total_pop,
                    'constituent_settlements': constituent_settlements
                }
                result.append(merged)
            else:
                result.append(current)
    
    # Validate total population
    total_final_population = sum(s['population'] for s in result)
    population_difference = total_initial_population - total_final_population
    
    if abs(population_difference) > 0:
        print(f"\nDistributing remaining population: {population_difference:,}")
        # Distribute any remaining population proportionally
        total_result_population = sum(s['population'] for s in result)
        for settlement in result:
            # Calculate proportion of total population
            proportion = settlement['population'] / total_result_population
            # Add proportional share of remaining population
            settlement['population'] += round(population_difference * proportion)
    
    print(f"\nFinal grouping results:")
    print(f"Original settlements: {len(settlements)}")
    print(f"Final groups: {len(result)}")
    print(f"Original total population: {total_initial_population:,}")
    print(f"Final total population: {sum(s['population'] for s in result):,}")
    
    return result

def save_unmatched_settlements(settlements):
    """Save unmatched settlements to a JSON file for review"""
    output_file = 'unmatched_settlements.json'
    with open(output_file, 'w') as f:
        json.dump(settlements, f, indent=2)
    print(f"\nUnmatched settlements saved to {output_file}")

def load_existing_settlements():
    """Load existing settlements from settlements.json"""
    try:
        with open('../mapData/sourceData/settlements.json', 'r') as f:
            return json.load(f)
    except FileNotFoundError:
        print("No existing settlements.json found")
        return None

def load_missing_settlements():
    """Load missing settlements from missing_settlements_analysis.json"""
    try:
        with open('missing_settlements_analysis.json', 'r') as f:
            data = json.load(f)
            settlements = []
            for name, population in data['missing_settlements'].items():
                settlements.append({
                    'name': name,
                    'population': population
                })
            return settlements
    except FileNotFoundError:
        print("Error: missing_settlements_analysis.json not found")
        raise SystemExit(1)

def main():
    args = parse_args()
    
    try:
        # Create output directory if it doesn't exist
        os.makedirs('../mapData/sourceData', exist_ok=True)
        
        # Check for Google Maps API key
        if not os.getenv('GOOGLE_MAPS_API_KEY'):
            print("Error: GOOGLE_MAPS_API_KEY environment variable not set")
            raise SystemExit(1)
        
        if args.append:
            print("\nRunning in append mode")
            # Load existing settlements
            existing_data = load_existing_settlements()
            if not existing_data:
                print("Error: Cannot run in append mode without existing settlements.json")
                raise SystemExit(1)
            
            # Load missing settlements
            print("\nLoading missing settlements...")
            cso_settlements = load_missing_settlements()
            
            # Clear existing checkpoints to start fresh for append mode
            clear_checkpoints()
        else:
            # Stage 1: Fetch CSO settlements
            cso_settlements = load_checkpoint('cso_settlements')
            if cso_settlements is None:
                cso_settlements = fetch_cso_settlements()
                save_checkpoint('cso_settlements', cso_settlements)
        
        total_ireland_population = sum(s['population'] for s in cso_settlements)
        if args.append:
            print(f"\nProcessing {len(cso_settlements)} missing settlements")
            print(f"Population to process: {total_ireland_population:,}")
        else:
            print(f"\nTotal Ireland population (CSO data): {total_ireland_population:,}")
        
        # Stage 2: Pre-group small settlements
        grouped_cso_settlements = load_checkpoint('grouped_settlements')
        if grouped_cso_settlements is None:
            print("\nPre-grouping small settlements...")
            grouped_cso_settlements = pregroup_small_settlements(cso_settlements)
            save_checkpoint('grouped_settlements', grouped_cso_settlements)
        
        # Stage 3: Fetch coordinates
        print("\nFetching coordinates from Google Maps API in batches...")
        settlement_names = [s['name'] for s in grouped_cso_settlements]
        locations = fetch_locations_batch(settlement_names)
        
        # Stage 4: Process settlements with coordinates
        settlements = []
        not_found = []
        not_found_details = []  # Store detailed information about unmatched settlements
        
        for settlement in grouped_cso_settlements:
            if settlement['name'] in locations:
                coords = locations[settlement['name']]
                new_settlement = {
                    'name': settlement['name'],
                    'population': settlement['population'],
                    'lat': coords['lat'],
                    'lon': coords['lon'],
                    'constituent_settlements': settlement.get('constituent_settlements', [settlement['name']])
                }
                settlements.append(new_settlement)
            else:
                not_found.append(settlement['name'])
                not_found_details.append({
                    'name': settlement['name'],
                    'population': settlement['population'],
                    'constituent_settlements': settlement.get('constituent_settlements', [settlement['name']])
                })
        
        save_checkpoint('processed_settlements', (settlements, not_found))
        
        # Save unmatched settlements to file
        if not_found_details:
            save_unmatched_settlements(not_found_details)
        
        if not_found:
            print(f"\nWarning: Could not find coordinates for {len(not_found)} settlements:")
            print(", ".join(not_found[:10]))
            if len(not_found) > 10:
                print(f"...and {len(not_found) - 10} more")
            
            # Calculate failure rate
            failure_rate = len(not_found) / len(settlement_names)
            if failure_rate > 0.1:  # More than 10% failure rate
                print(f"\nHigh failure rate ({failure_rate:.1%}). Keeping checkpoints for review.")
                print("Run the script again after reviewing unmatched_settlements.json")
                raise SystemExit(1)
        
        if not settlements:
            print("Error: No settlements could be matched with coordinates")
            raise SystemExit(1)
        
        # Stage 5: Group and process final settlements
        final_data = load_checkpoint('final_settlements')
        if final_data is None:
            # Group settlements by distance
            print("\nGrouping settlements by distance...")
            grouped_settlements = group_settlements(settlements)
            
            # Process groups into final settlements
            final_settlements = [process_settlement_group(group) for group in grouped_settlements]
            
            # Transform coordinates to grid system
            transform = calculate_grid_transformation()
            for settlement in final_settlements:
                grid_coords = transform_coordinates([(settlement['lon'], settlement['lat'])], transform)[0]
                settlement['grid_x'] = grid_coords[0]
                settlement['grid_y'] = grid_coords[1]
            
            if args.append:
                # Combine with existing settlements
                print("\nAppending new settlements to existing data...")
                
                # Calculate the population that was previously distributed
                original_total = sum(s['population'] for s in existing_data['settlements'])
                new_total = sum(s['population'] for s in final_settlements)
                total_with_new = original_total + new_total
                
                # Calculate how much population was previously distributed
                distributed_population = total_with_new - existing_data['metadata']['total_ireland_population']
                if distributed_population > 0:
                    print(f"\nUndistributing {distributed_population:,} population from existing settlements...")
                    # Calculate proportion to undistribute from each existing settlement
                    for settlement in existing_data['settlements']:
                        proportion = settlement['population'] / original_total
                        settlement['population'] -= round(distributed_population * proportion)
                
                # Combine settlements
                final_data = {
                    'settlements': existing_data['settlements'] + final_settlements,
                    'metadata': {
                        'total_population': sum(s['population'] for s in existing_data['settlements']) + 
                                          sum(s['population'] for s in final_settlements),
                        'total_settlements': len(existing_data['settlements']) + len(final_settlements),
                        'total_ireland_population': existing_data['metadata']['total_ireland_population'],
                        'grid_transform': transform,
                        'data_sources': {
                            'population': 'CSO Ireland Census 2022',
                            'coordinates': 'Google Maps'
                        },
                        'unmatched_settlements': existing_data['metadata']['unmatched_settlements'] + not_found,
                        'failure_rate': len(not_found) / len(settlement_names)
                    }
                }
            else:
                final_data = {
                    'settlements': final_settlements,
                    'metadata': {
                        'total_population': sum(s['population'] for s in final_settlements),
                        'total_settlements': len(final_settlements),
                        'total_ireland_population': total_ireland_population,
                        'grid_transform': transform,
                        'data_sources': {
                            'population': 'CSO Ireland Census 2022',
                            'coordinates': 'Google Maps'
                        },
                        'unmatched_settlements': not_found,
                        'failure_rate': len(not_found) / len(settlement_names)
                    }
                }
            save_checkpoint('final_settlements', final_data)
        
        # Save final output
        with open('../mapData/sourceData/settlements.json', 'w') as f:
            json.dump(final_data, f, indent=2)
        
        print(f"\nProcessed {len(final_data['settlements'])} settlement groups")
        print(f"Total population in grouped settlements: {final_data['metadata']['total_population']:,}")
        print(f"Total population in Ireland (from CSO): {final_data['metadata']['total_ireland_population']:,}")
        print("Data saved to mapData/sourceData/settlements.json")
        
        # Only clear checkpoints if failure rate is acceptable
        if final_data['metadata']['failure_rate'] <= 0.1:
            clear_checkpoints()
        else:
            print("\nKeeping checkpoints due to high failure rate")
        
    except Exception as e:
        print(f"\nAn unexpected error occurred: {e}")
        print("Progress saved in checkpoints. Run script again to resume.")
        raise SystemExit(1)

if __name__ == "__main__":
    main() 