# Warpaint

Warpaint (codename) is a pixel-art-oriented image editor written in rust.

## Features / TODO

I expect to have all of the following features. This is sorted in order of development priority, not general importance.

### tools
- [x] pencil tool (with proper edit and commit phases) (todo: brush size)
- [x] fill tool
- [ ] eraser tool
- [ ] eyedropper tool
- [ ] line tool
- [ ] other shape tools
- [ ] selection tool (arbitrary alpha-mask-based selections, not shape-bsae dselections)
- [ ] transform tool (bilinear transform, not perspective transform)

### layer management stuff
- [x] layers
- [x] layer creation/deletion
- [x] layer groups (implemented but not hooked up)
- [x] layer moving (by buttons)
- [ ] layer moving (by drag and drop)
- [ ] layer merging/flattening/etc

### other layer stuff
- [ ] photoshop blend modes
- [ ] warpaint-specific blend modes like "multiply brightness and color" and "add (signed)"
- [ ] custom blend modes with a scripting language
- [ ] layer masks
- [ ] layer clipping
- [ ] alpha lock
- [ ] layer lock (implemented but not hooked up)

### ui
- [ ] grids
- [ ] rgb and alpha sliders for the color picker
- [x] view transformation (scale, pan, rotation)
- [ ] view mirroring
- [ ] preview panel like clip studio paint and photoshop's navigator panel
- [ ] custom preview windows with a scripting language, for things like tilesets, animations, etc
- [ ] project tabs

### etc
- [ ] undo/redo (!!!!important!!!!) (implement via layer stack analysis + dirty flags/rects)
- [ ] flag to do processing in unclamped rgb instead of clamped rgb
- [ ] flag to do processing in linear rgb instead of sRGB
- [ ] file creation menu
- [ ] opening and saving images
- [ ] opening and saving project files (use sqlite?)
- [ ] option to save the undo/redo buffer to project files (for timelapses)
- [ ] symmetry tool

# License

Copyright 2022 "Wareya" (wareya@gmail.com) and any contributors

Licensed under the Apache License v2.0, with LLVM Exceptions and an
additional custom exception that makes the license more permissive.
The custom exception may be removed, allowing you to use this software
under the SPDX-identified `Apache-2.0 WITH LLVM-exception` license. See
LICENSE.txt and the below License Exceptions section for details.

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

## License Exceptions

This section is not legal code, it's a human-readable summary.

This software is licensed with the LLVM exceptions on top of the
Apache 2.0 license. These exceptions make it compatible with the GNU
GPLv2 and also waive certain restrictions when distributing binaries.

This software is licensed an additional, custom exception that makes the
Apache 2.0 license more permissive by not requiring modified source
files to be marked with prominent notices. This exception can be
removed, turning the license into pure `Apache-2.0 WITH LLVM-exception`.
In other words, as a user or downstream project or dependent, you can
ignore this exception's existence, and as a contributor or maintainer,
it means that you have one less responsibility.

These exceptions do not necessarily apply to any dependencies or
dependents of this software, unless they independently have the same or
similar exceptions.

## Contributing

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you shall be licensed as above,
without any additional terms or conditions.
