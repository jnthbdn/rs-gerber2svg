# Gerber2SVG

*Badges*

## Introduction
Gerber2Svg is a library and utility written in [Rust](Rust), used to convert a [Gerber (x2 or x3) ](https://www.ucamco.com/files/downloads/file_en/456/gerber-layer-format-specification-revision-2023-03_en.pdf?75b8486ed12c0ba3d07ee9f48708eb20) file into an [SVG](https://en.wikipedia.org/wiki/SVG) file.

The generated SVG file is not a single compound path but rather a collection of independent elements such as paths, rectangles, circles, and other primitives.
The SVG output preserves the units defined in the source Gerber file. However, certain SVG viewers and editors (including Inkscape) may not interpret these units consistently. This behavior is usually related to DPI or scaling settings applied during file opening or import.

> ℹ️ Useful links 
> - [SVG Viewer](https://www.svgviewer.dev/): online editor/viewer that lets you open SVG files while preserving correct dimensions and units.
> - [Gerber Viewer](https://gerber-viewer.ucamco.com/): official online viewer for Gerber files.
  
  
⚠️ **This work is in progress**, so please be kind 😇. If you discover any bug or mistake, feel free to open an issue or submit a PR! Even typo fixes are welcome 😛

## Usage
Command to display help:
`gerber2svg --help`

| Short option | Long option           | Required | Description                                                           |
| ------------ | --------------------- | -------- | --------------------------------------------------------------------- |
| -i           | --input <gerber-file> | **Yes**  | The Gerber file                                                       |
| -s           | --scale <scale>       | No       | Scale the path and apertures [default: 1.0]                           |
| -o           | --output <svg-file>   | No       | The SVG output file (otherwise SVG will be print  on standard output) |
| -c           | --crop                | No       | Crop the SVG to remove unnecessary space                              |
| -d           | --debug               | No       | Be verbose and print debug info                                       |
| -h           | --help                | No       | Prints help information                                               |
| -V           | --version             | No       | Prints version information                                            |
| -v           | --verbose             | No       | Be more verbose and show gerber comments                              |

With any option other than `--output` or `-o`, the SVG will be printed to standard output.

## To Do

| Task                          | Status |
| ----------------------------- | ------ |
| Upgrade `gerber_parser`       | 🟢    |
| Test with new `gerber_parser` | 🟠    |
| Support Arc segment           | 🟠    |
| Test arc                      | 🔴    |
| Test scale                    | 🔴    |
| Support Region Mode           | 🔴    |
| Support Quadrant Mode         | 🔴    |
| Support Exetnded Code         | ❓    |
| Support Obround aperture      | 🔴    |
| Support Polygon aperture      | 🔴    |
| Support Macro aperture        | 🔴    |
| Finish this list...           | 🔴    |


