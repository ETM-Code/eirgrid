# Weight History Visualization Tool

This tool creates animated visualizations of the weight history data collected during simulation runs, showing how action weights evolve over time.

## Requirements

- Python 3.6+
- Required packages:
  - matplotlib
  - numpy
  - ffmpeg (for saving animations)

You can install the required Python packages with:

```bash
pip install matplotlib numpy
```

You'll also need ffmpeg installed on your system:
- macOS: `brew install ffmpeg`
- Ubuntu: `sudo apt-get install ffmpeg`

## Usage

Run the simulation with the `--track-weight-history` flag to generate the weight history JSON file:

```bash
cargo run -- --iterations 1000 --parallel --track-weight-history
```

This will create a `weight_history.json` file in the checkpoint directory.

Then use the visualization script to generate animations:

```bash
cd aiSimulator/tools/visualization
python weight_history_animation.py /path/to/checkpoints/weight_history.json
```

By default, the visualizations will be saved to a directory called `weight_visualizations`. You can specify a different output directory with the `--output-dir` option:

```bash
python weight_history_animation.py /path/to/checkpoints/weight_history.json --output-dir my_visualizations
```

## Visualization Types

The tool generates several types of visualizations:

1. **Score Progress Plot**: A static image showing how the best score evolves over time
2. **Weight Evolution Animation**: Shows how the weights for top actions evolve over iterations
3. **Generator Weights Animation**: Specific animation focusing on generator type weights
4. **Carbon Offset Weights Animation**: Specific animation focusing on carbon offset type weights
5. **Timeline Animation**: Shows weight changes for different generator types and carbon offset types simultaneously
6. **Heatmap Animation**: A heatmap view of how weights change for the top actions

## Converting MP4 to GIF

If you want to convert the MP4 animations to GIF format for easier sharing:

```bash
ffmpeg -i weight_visualizations/weight_evolution.mp4 -vf "fps=10,scale=800:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse" weight_visualizations/weight_evolution.gif
```

## Example Output

The animations help visualize how the AI learns over time by showing:
- Which actions gain or lose weight
- How quickly the learning occurs
- When significant improvements happen
- Which generator or offset types become preferred

This information can be valuable for understanding the simulation's learning process and potentially identifying ways to improve the learning algorithm. 