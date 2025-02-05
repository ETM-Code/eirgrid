#!/usr/bin/env python3
import csv


def filter_ireland_generators(input_csv, output_csv):
    primary_fuels = set()  # collect unique primary fuels
    with open(input_csv, newline='', encoding='utf-8') as infile, open(output_csv, 'w', newline='', encoding='utf-8') as outfile:
        reader = csv.DictReader(infile)
        fieldnames = ['capacity_mw', 'latitude', 'longitude', 'primary_fuel']
        writer = csv.DictWriter(outfile, fieldnames=fieldnames)
        writer.writeheader()
        
        for row in reader:
            # Filter rows where the country_long field is 'Ireland'
            if row.get('country_long', '').strip().lower() == 'ireland':
                fuel = row.get('primary_fuel', '').strip()
                primary_fuels.add(fuel)
                writer.writerow({
                    'capacity_mw': row.get('capacity_mw', ''),
                    'latitude': row.get('latitude', ''),
                    'longitude': row.get('longitude', ''),
                    'primary_fuel': fuel
                })
    
    print("Primary fuels found:", ", ".join(sorted(primary_fuels)))


if __name__ == "__main__":
    input_csv = 'global_power_plant_database.csv'
    output_csv = 'ireland_generators.csv'
    filter_ireland_generators(input_csv, output_csv)
    print(f"Filtered data written to {output_csv}") 