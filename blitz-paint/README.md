# blitz-paint

Upstream: [`blitz-paint`](https://crates.io/crates/blitz-paint) 0.3.0-beta.4,
from [DioxusLabs/blitz](https://github.com/DioxusLabs/blitz).
License: MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). Copyright belongs
to the Blitz authors.

`crate/` is the crates.io release with `patches/` applied in order.

## Patches

### 0001 Inline gradient backgrounds

`draw_inline_backgrounds` in `src/text.rs`, and the row and row-group colors
in `draw_table_row_backgrounds` (`src/render/background.rs`). Upstream paints
an inline box's `background-color` but not its `background-image`. Emails'
dark styles commonly set `background: linear-gradient(c, c)` on an element
whose light `background-color` comes from a later rule; browsers paint the
gradient over that color, so the element is dark. Without the gradient, a
`<span>` such as a tracking number showed white text on a white box. The patch
paints an inline box's gradient layers over its background color, and gives a
table row or row group whose top layer is a one-color gradient that color
(behind a cell's rounded corners, the light color showed as triangles).

Remove when upstream paints inline background images.

### 0002 Anonymous table rows

`draw_table_row_backgrounds` in `src/render/background.rs`. A block directly
in a table is laid out in an anonymous row of its own (blitz-dom's 0005),
whose `node_id` is the block itself. Its background is the block's, painted
as the cell's; as a row's it would be painted again behind it. Rows marked
`anonymous` paint no row background. Needs blitz-dom's 0005.

Remove with blitz-dom's 0005.

### 0003 SVG image layers

`render_svg_image` in `src/render.rs` (used for an `<img>`'s SVG and an SVG
background layer) and `is_image_layer` in `src/lib.rs`. A raster image is
recorded as one image fill, but an SVG image as ordinary fills and strokes,
which a consumer of the recorded scene couldn't tell from the document's
own drawing. Picacho recolors the scene for dark reading and leaves images
alone, so SVG logos were recolored and photos weren't. SVG images are now
drawn inside an even-odd layer clipped to the image's viewport (nothing else
pushes an even-odd layer), which `is_image_layer` recognizes. Clipping to the
viewport is what browsers do for SVG used as an image.

Remove when upstream marks image content in the scene it records.

## Upgrading

```sh
scripts/vendor.sh rebuild blitz-paint <version>
```

Then update the version above and in Picacho's `Cargo.toml`.
