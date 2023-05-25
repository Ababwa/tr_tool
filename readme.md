
# TR Tool

Tool for viewing and modifying Tomb Raider level files. Currently only renders TR4 level geometry.

## Usage

`tr_tool path_to_file/level.tr4`

## Controls

* Right-click to toggle fly-mode
* When in fly-mode:
	* Move mouse to look around
	* WASD to move around
	* Q/E to raise/lower
	* Hold shift to move faster
* Escape to exit

## Planned features

* GUI level file chooser
* Export to .prj2 ([Tomb Editor project file](https://github.com/MontyTRC89/Tomb-Editor))
* Support other TR level files (TR1-3, 5)
* Render upgrades:
	* Render objects
	* Render transparency
	* Render lights
	* Render double-sided faces
	* Room-based rendering (currently renders all rooms at once, including flipmaps, creating many overlaps)
	* Increase performance (currently sluggish)
* Editing:
	* Vertical room split (like Tomb Editor's "Split room", but vertical)

## Structure

TR Tool is broken into 4 Rust crates: one binary crate, and three library crates. This repository only contains the tr_tool binary crate.

* tr_tool: Binary crate that produces the final executable that loads and renders TR level files.
* tr_reader: Library crate that provides data structures for handling TR level file data.
* tr_readable: Library crate that provides a trait and functions for reading TR level file data.
* tr_derive: Procedural macro crate that provides a derive macro that can be used to generate implementations for the tr_readable trait.

The dependency graph, where "->" means "depends on":

tr_tool -> tr_reader -> tr_readable -> tr_derive

Each crate expects its dependent to be adjacent to it in the file system. This can be changed by modifying the dependency path in Cargo.toml.
