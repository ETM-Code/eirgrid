#!/usr/bin/env python3

import json
import csv
from pathlib import Path
from collections import defaultdict

def load_json_settlements():
    """Load settlements from the JSON file"""
    try:
        with open('../mapData/sourceData/settlements.json', 'r') as f:
            data = json.load(f)
            
        # Create a set of all settlement names (including constituents)
        all_settlements = set()
        for settlement in data['settlements']:
            all_settlements.update(settlement['constituent_settlements'])
            
        return all_settlements
    except FileNotFoundError:
        print("Error: settlements.json not found")
        raise SystemExit(1)

def load_csv_settlements():
    """Load settlements from Population.csv"""
    try:
        settlements = {}
        with open('Population.csv', 'r', encoding='utf-8-sig') as f:
            reader = csv.DictReader(f)
            for row in reader:
                # Only process population statistics (F1011C01)
                if row['STATISTIC'] != 'F1011C01':
                    continue
                
                name = row['Electoral Divisions'].strip()
                value = row['VALUE'].strip()
                
                # Skip rows with empty values
                if not value:
                    continue
                
                # Convert to float first, then round to integer
                population = round(float(value))
                settlements[name] = population
                
        return settlements
    except FileNotFoundError:
        print("Error: Population.csv not found")
        raise SystemExit(1)

def analyze_missing_settlements():
    """Find settlements that are in CSV but not in JSON"""
    print("Loading settlements from JSON...")
    json_settlements = load_json_settlements()
    print(f"Found {len(json_settlements)} settlements in JSON (including constituents)")
    
    print("\nLoading settlements from CSV...")
    csv_settlements = load_csv_settlements()
    print(f"Found {len(csv_settlements)} settlements in CSV")
    
    # Find missing settlements
    missing_settlements = {}
    total_missing_population = 0
    
    for name, population in csv_settlements.items():
        if name not in json_settlements:
            missing_settlements[name] = population
            total_missing_population += population
    
    # Group missing settlements by region
    missing_by_region = defaultdict(list)
    for name, population in missing_settlements.items():
        parts = name.split(',')
        region = parts[1].strip() if len(parts) > 1 else 'Unknown'
        missing_by_region[region].append({
            'name': name,
            'population': population
        })
    
    # Prepare output data
    output_data = {
        'missing_settlements': missing_settlements,
        'missing_by_region': dict(missing_by_region),
        'metadata': {
            'total_missing': len(missing_settlements),
            'total_missing_population': total_missing_population,
            'total_settlements_csv': len(csv_settlements),
            'total_settlements_json': len(json_settlements),
            'percentage_missing': (len(missing_settlements) / len(csv_settlements)) * 100
        }
    }
    
    # Save to JSON
    output_file = 'missing_settlements_analysis.json'
    with open(output_file, 'w') as f:
        json.dump(output_data, f, indent=2)
    
    # Print summary
    print(f"\nAnalysis complete:")
    print(f"Total settlements in CSV: {len(csv_settlements)}")
    print(f"Total settlements in JSON: {len(json_settlements)}")
    print(f"Missing settlements: {len(missing_settlements)}")
    print(f"Missing population: {total_missing_population:,}")
    print(f"Percentage missing: {output_data['metadata']['percentage_missing']:.1f}%")
    print("\nMissing settlements by region:")
    for region, settlements in missing_by_region.items():
        region_population = sum(s['population'] for s in settlements)
        print(f"{region}: {len(settlements)} settlements, {region_population:,} population")
    print(f"\nDetailed analysis saved to {output_file}")

if __name__ == "__main__":
    analyze_missing_settlements() 