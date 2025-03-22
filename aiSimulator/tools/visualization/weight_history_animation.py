#!/usr/bin/env python3
"""
Weight History Visualization Tool for EirGrid AI Simulator

This script creates animated visualizations of the weight history data
collected during simulation runs. It shows how action weights evolve over time.

Usage:
    python weight_history_animation.py path/to/weight_history.json
"""

import json
import sys
import os
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation
from matplotlib.cm import get_cmap
from matplotlib.lines import Line2D
import argparse
from datetime import datetime
import matplotlib.dates as mdates
from matplotlib.gridspec import GridSpec
import matplotlib.colors as mcolors

def parse_datetime(timestamp):
    """Parse RFC3339 timestamp to datetime object"""
    # Remove microseconds if present, as they can cause parsing issues
    if '.' in timestamp:
        timestamp = timestamp.split('.')[0] + 'Z'
    return datetime.fromisoformat(timestamp.replace('Z', '+00:00'))

def load_weight_history(filename):
    """Load weight history from JSON file"""
    try:
        with open(filename, 'r') as f:
            return json.load(f)
    except Exception as e:
        print(f"Error loading weight history file: {e}")
        sys.exit(1)

def extract_key_metrics(history):
    """Extract key metrics from weight history for plotting"""
    iterations = [entry['iteration'] for entry in history]
    best_scores = [entry['best_score'] for entry in history]
    timestamps = [parse_datetime(entry['timestamp']) for entry in history]
    
    # Extract learning parameters
    learning_rates = [entry['weights'].get('learning_rate', 0.0) for entry in history]
    exploration_rates = [entry['weights'].get('exploration_rate', 0.0) for entry in history]
    
    return iterations, best_scores, timestamps, learning_rates, exploration_rates

def extract_action_weights(history, action_type=None):
    """
    Extract weights for specific action types across all iterations
    
    Parameters:
        history: The weight history data
        action_type: Optional filter for specific action type (e.g., 'AddGenerator')
    
    Returns:
        Dictionary mapping action names to lists of weight values
    """
    action_weights = {}
    action_count_weights = {}
    
    for entry in history:
        weights = entry['weights']
        iteration = entry['iteration']
        
        # Extract weights by year
        for year, year_weights in weights.get('weights', {}).items():
            for action, weight in year_weights.items():
                # Filter by action type if specified
                if action_type and not action.startswith(action_type):
                    continue
                    
                if action not in action_weights:
                    # Initialize with zeros for previous iterations
                    action_weights[action] = [0.0] * history.index(entry)
                
                # Extend list if needed
                while len(action_weights[action]) < history.index(entry):
                    action_weights[action].append(0.0)
                    
                action_weights[action].append(weight)
        
        # Extract action count weights
        for year, year_weights in weights.get('action_count_weights', {}).items():
            for count, weight in year_weights.items():
                key = f"{year}_{count}"
                if key not in action_count_weights:
                    action_count_weights[key] = [0.0] * history.index(entry)
                
                while len(action_count_weights[key]) < history.index(entry):
                    action_count_weights[key].append(0.0)
                    
                action_count_weights[key].append(weight)
        
        # Pad any missing values for this iteration
        for weights_dict in [action_weights, action_count_weights]:
            for action in weights_dict:
                if len(weights_dict[action]) <= history.index(entry):
                    weights_dict[action].append(weights_dict[action][-1] if weights_dict[action] else 0.0)
    
    return action_weights, action_count_weights

def create_dynamic_bar_chart_animation(history, output_dir):
    """
    Create a dynamic bar chart animation with the following features:
    - Bars represent relative weights of actions
    - Increasing weights are green, decreasing weights are red
    - Animation is exactly 20 seconds long
    - No action labels, only axis labels
    - Smooth tweening between positions
    - Continuous iteration counter from 0 to nearest 100
    - Orders iterations from least extreme to most extreme
    """
    # Extract action weights
    action_weights, action_count_weights = extract_action_weights(history)
    iterations = [entry['iteration'] for entry in history]
    max_iteration = max(iterations)
    # Round max iteration to nearest 100 for title display
    rounded_max_iteration = round(max_iteration / 100) * 100
    
    # Combine all weights into a single dictionary
    all_weights = {}
    all_weights.update(action_weights)
    
    # Get top weights by final value (use all weights for completeness)
    actions = list(all_weights.keys())
    
    # Prepare data structure for animation
    original_frames_data = []
    for i in range(len(iterations)):
        frame_data = []
        for action in actions:
            # Get weight for this iteration if available
            weight = all_weights[action][i] if i < len(all_weights[action]) else 0
            frame_data.append(weight)
        original_frames_data.append(frame_data)
    
    # Calculate "extremeness" for each iteration
    # Extremeness is defined as the standard deviation of weights
    extremeness_scores = []
    for frame_data in original_frames_data:
        # Filter out zeros to focus on active weights
        non_zero_weights = [w for w in frame_data if w > 0.00001]
        if non_zero_weights:
            # Calculate standard deviation as a measure of extremeness
            extremeness = np.std(non_zero_weights)
            # Consider both std dev and max value for extremeness
            max_weight = max(non_zero_weights) if non_zero_weights else 0
            extremeness_scores.append(extremeness * max_weight)
        else:
            extremeness_scores.append(0)
    
    # Create indices sorted by extremeness (least to most extreme)
    sorted_indices = [i for i, _ in sorted(enumerate(extremeness_scores), key=lambda x: x[1])]
    
    # Reorder frames data by extremeness
    frames_data = [original_frames_data[i] for i in sorted_indices]
    # Also reorder iterations to match
    sorted_iterations = [iterations[i] for i in sorted_indices]
    
    # Set up the figure and axis for the bar chart
    fig, ax = plt.figure(figsize=(12, 8)), plt.subplot()
    plt.subplots_adjust(bottom=0.15)
    
    # Calculate how many iterations to show per frame to fit in 20 seconds
    # Using standard 30fps for video
    total_frames = 30 * 20  # 30fps * 20 seconds = 600 frames
    
    # Calculate tweening parameters
    tween_frames = 30  # Number of frames to tween between positions
    num_keyframes = total_frames // tween_frames
    
    # If we have fewer data points than keyframes, we'll repeat frames
    # If we have more, we'll skip some
    if len(frames_data) < num_keyframes:
        repeat_factor = num_keyframes // len(frames_data)
        step = 1
    else:
        repeat_factor = 1
        step = len(frames_data) // num_keyframes
    
    # Function to interpolate between two lists of values
    def interpolate_values(start_values, end_values, t):
        return [start + (end - start) * t for start, end in zip(start_values, end_values)]
    
    # Function to interpolate between two lists of indices
    def interpolate_indices(start_indices, end_indices, t):
        # Convert indices to positions
        start_positions = {idx: i for i, idx in enumerate(start_indices)}
        end_positions = {idx: i for i, idx in enumerate(end_indices)}
        
        # Interpolate positions for each index
        interpolated_positions = {}
        for idx in set(start_indices + end_indices):
            start_pos = start_positions.get(idx, len(start_indices))
            end_pos = end_positions.get(idx, len(end_indices))
            interpolated_positions[idx] = start_pos + (end_pos - start_pos) * t
        
        # Sort indices by interpolated position
        return sorted(interpolated_positions.keys(), 
                     key=lambda x: interpolated_positions[x])
    
    # Store previous frame values for calculating real-time changes
    previous_frame_values = {}
    
    # Function to animate the bars
    def animate(frame_idx):
        nonlocal previous_frame_values
        ax.clear()
        
        # Calculate which keyframe we're between
        keyframe_idx = frame_idx // tween_frames
        tween_progress = (frame_idx % tween_frames) / tween_frames
        
        # Calculate which data frames to interpolate between
        data_idx1 = min((keyframe_idx // repeat_factor) * step, len(frames_data) - 1)
        data_idx2 = min(((keyframe_idx + 1) // repeat_factor) * step, len(frames_data) - 1)
        
        # Get data for the two keyframes
        values1 = frames_data[data_idx1]
        values2 = frames_data[data_idx2]
        
        # Only plot bars with non-zero weights to avoid cluttering
        non_zero_indices1 = [i for i, v in enumerate(values1) if v > 0.00001]
        non_zero_indices2 = [i for i, v in enumerate(values2) if v > 0.00001]
        non_zero_values1 = [values1[i] for i in non_zero_indices1]
        non_zero_values2 = [values2[i] for i in non_zero_indices2]
        
        if not non_zero_values1 and not non_zero_values2:  # If all values are zero
            return []
        
        # Sort by value for better visualization
        sorted_indices1 = sorted(range(len(non_zero_values1)), 
                               key=lambda i: non_zero_values1[i], reverse=True)
        sorted_indices2 = sorted(range(len(non_zero_values2)), 
                               key=lambda i: non_zero_values2[i], reverse=True)
        
        # Interpolate between the two sorted lists
        interpolated_indices = interpolate_indices(
            [non_zero_indices1[i] for i in sorted_indices1],
            [non_zero_indices2[i] for i in sorted_indices2],
            tween_progress
        )
        
        # Get interpolated values
        interpolated_values = []
        for idx in interpolated_indices:
            # Find the value in each keyframe
            val1 = values1[idx] if idx in non_zero_indices1 else 0
            val2 = values2[idx] if idx in non_zero_indices2 else 0
            interpolated_values.append(val1 + (val2 - val1) * tween_progress)
        
        # Normalize values for better display
        max_value = max(interpolated_values) if interpolated_values else 1
        if max_value > 0:
            normalized_values = [v / max_value for v in interpolated_values]
        else:
            normalized_values = interpolated_values
        
        # Determine colors based on weight changes from previous frame
        # This ensures we're highlighting based on what the viewer actually sees
        colors = []
        
        # Create a mapping to track changes by action index
        for i, idx in enumerate(interpolated_indices):
            current_value = interpolated_values[i]
            
            # Check if we have a previous value for this index
            # Use a string key since we'll be accessing from different frames
            idx_key = str(idx)
            if idx_key in previous_frame_values:
                prev_value = previous_frame_values[idx_key]
                change = current_value - prev_value
                
                # Green if increasing, red if decreasing, gray if no change
                if change > 0.00001:  # Small threshold to avoid noise
                    colors.append('green')
                elif change < -0.00001:
                    colors.append('red')
                else:
                    colors.append('gray')
            else:
                # If no previous value, use gray for the first appearance
                colors.append('gray')
                
            # Update previous values for next frame
            previous_frame_values[idx_key] = current_value
        
        # Clean up previous_frame_values by removing indices not in this frame
        current_idx_keys = [str(idx) for idx in interpolated_indices]
        for idx_key in list(previous_frame_values.keys()):
            if idx_key not in current_idx_keys:
                # This action is no longer visible, remove it
                del previous_frame_values[idx_key]
        
        # Create the bars
        bars = ax.bar(range(len(normalized_values)), normalized_values, color=colors)
        
        # Get the actual iteration number for display purposes
        if data_idx1 < len(sorted_indices):
            actual_iteration = sorted_iterations[data_idx1]
        else:
            actual_iteration = sorted_iterations[-1]
            
        # Calculate continuous iteration counter for title - count to nearest 100
        title_iteration = int((frame_idx / total_frames) * rounded_max_iteration)
        
        # Set labels and title
        ax.set_ylabel('Relative Weight')
        ax.set_title(f'Action Weights Evolution (Iteration {title_iteration})')
        
        # Remove x-ticks and labels for cleaner look
        ax.set_xticks([])
        
        # Add grid for better readability
        ax.grid(axis='y', linestyle='--', alpha=0.7)
        
        # Remove box for cleaner appearance
        ax.spines['top'].set_visible(False)
        ax.spines['right'].set_visible(False)
        ax.spines['bottom'].set_visible(False)
        
        return bars
    
    # Create the animation
    anim = animation.FuncAnimation(
        fig, animate, frames=total_frames,
        interval=1000/30,  # 30fps
        blit=True
    )
    
    # Save the animation
    filename = 'weight_bar_chart.mp4'
    anim.save(os.path.join(output_dir, filename), 
             writer='ffmpeg', fps=30, dpi=200,
             extra_args=['-vcodec', 'libx264'])
    
    plt.close(fig)
    
    print(f"Dynamic bar chart animation saved to {os.path.join(output_dir, filename)}")
    print(f"Animation length: 20 seconds")

def main():
    parser = argparse.ArgumentParser(description='Generate weight history visualizations')
    parser.add_argument('weight_file', help='Path to weight_history.json file')
    parser.add_argument('--output-dir', '-o', default='weight_visualizations',
                       help='Directory to save visualization outputs')
    args = parser.parse_args()
    
    # Create output directory if it doesn't exist
    os.makedirs(args.output_dir, exist_ok=True)
    
    print(f"Loading weight history from: {args.weight_file}")
    history = load_weight_history(args.weight_file)
    print(f"Loaded {len(history)} weight history entries")
    
    # Create dynamic bar chart animation
    print("Creating dynamic bar chart animation...")
    create_dynamic_bar_chart_animation(history, args.output_dir)
    
    print(f"Animation saved to {args.output_dir}")
    print("To view animation, use a video player or convert to GIF using:")
    print(f"ffmpeg -i {args.output_dir}/weight_bar_chart.mp4 -vf 'fps=30,scale=800:-1:flags=lanczos' {args.output_dir}/weight_bar_chart.gif")

if __name__ == "__main__":
    main() 