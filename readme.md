# TR Tool

Tool for viewing Tomb Raider level files. Currently only renders TR4 level geometry.

## Usage

`tr_tool <path_to_level_file.tr4>`

## Controls

* Right-click to toggle fly-mode
* When in fly-mode:
	* Move mouse to look around
	* WASD to move around
	* Q/E to raise/lower
	* Hold shift to move faster

## Todo

* GUI level file chooser
* Export to .prj2 ([Tomb Editor project file](https://github.com/MontyTRC89/Tomb-Editor))
* Support other TR level files (TR1-3, 5)
* Render upgrades:
	* Render objects
	* Render transparency
	* Render lights
	* Room-based rendering (currently renders all rooms at once, including flipmaps, creating overlaping geometry)
* Editing:
	* Vertical room split (like Tomb Editor's "Split room", but vertical)
* Remove white screen on startup
* Documentation for `tr_reader`

## Structure

In addition to the `tr_tool` binary, this repository also contains `tr_reader`, a library used for deserializing Tomb Raider level files, and `tr_derive`, which provides a procedural macro for `tr_reader`.
