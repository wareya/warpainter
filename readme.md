# Warpaint

Warpaint (codename) is a pixel-art-oriented image editor written in rust.

## Features / TODO

I expect to have all of the following features. This is sorted in order of development priority, not general importance.

### tools
[x] pencil tool (with proper edit and commit phases) (todo: brush size)
[x] fill tool
[ ] eraser tool
[ ] eyedropper tool
[ ] line tool
[ ] other shape tools
[ ] selection tool (arbitrary alpha-mask-based selections, not shape-bsae dselections)
[ ] transform tool (bilinear transform, not perspective transform)

### layer management stuff
[x] layers
[x] layer creation/deletion
[x] layer groups (implemented but not hooked up)
[x] layer moving (by buttons)
[ ] layer moving (by drag and drop)
[ ] layer merging/flattening/etc

### other layer stuff
[ ] photoshop blend modes
[ ] warpaint-specific blend modes like "multiply brightness and color" and "add (signed)"
[ ] "custom" blend modes with a scripting language
[ ] layer masks
[ ] layer clipping
[ ] alpha lock
[ ] layer lock (implemented but not hooked up)

### ui
[ ] grids
[ ] rgb and alpha sliders for the color picker
[x] view transformation (scale, pan, rotation)
[ ] view mirroring
[ ] preview panel like clip studio paint and photoshop's navigator panel
[ ] custom preview windows with a scripting language, for things like tilesets, animations, etc
[ ] project tabs

### etc
[ ] undo/redo (!!!!important!!!!) (implement via layer stack analysis + dirty flags/rects)
[ ] flag to do processing in unclamped rgb instead of clamped rgb
[ ] flag to do processing in linear rgb instead of sRGB
[ ] file creation menu
[ ] opening and saving images
[ ] opening and saving project files (use sqlite?)
[ ] option to save the undo/redo buffer to project files (for timelapses)
[ ] symmetry tool

