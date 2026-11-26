# blitz-dom

Upstream: [`blitz-dom`](https://crates.io/crates/blitz-dom) 0.3.0-beta.4, from
[DioxusLabs/blitz](https://github.com/DioxusLabs/blitz).
License: MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). Copyright belongs
to the Blitz authors.

`crate/` is the crates.io release with `patches/` applied in order.

## Patches

### 0001 Unresolvable URLs

`Document::resolve_url` in `src/document.rs`. Upstream panics when an
`img src`, stylesheet `href` or form `action` can't be resolved against the
document's base URL. Picacho gives emails the base `about:blank` (so relative
resources have no sender-controlled origin), against which every relative
address fails, as does any malformed one such as `http://[`. An email could
therefore stop itself from being shown. The patch resolves such addresses to
`about:invalid`, which Picacho's resource and link providers refuse.

Remove when upstream stops panicking on unresolvable URLs.

### 0002 Tables restacked for narrow screens

Emails restack their layout for narrow screens by giving tables or cells
another `display` in a media query, which upstream laid out wrongly:

- A row group or row outside a table (its table made `display: block`) is
  wrapped in an anonymous table in CSS. Upstream laid it out as a flex-like
  flow, putting its rows side by side. It is now laid out as a table of its
  own (`is_orphan_table_part` in `src/layout/construct.rs`; a row as the root
  in `build_table_context`, `src/layout/table.rs`).
- A block or inline box directly in a row (a cell made `display: block`) is
  wrapped in an anonymous cell in CSS. Upstream dropped it, so the row's
  content vanished. It is now laid out as that cell (`collect_table_cells`).
  Unlike CSS, consecutive inline boxes each get a cell rather than sharing one,
  and the anonymous table is as wide as its container, not shrink-to-fit.

Remove when upstream wraps orphaned table parts in anonymous boxes.

### 0003 Legacy table attributes

`synthesize_presentational_hints_for_legacy_attributes` in `src/stylo.rs` maps
`cellspacing` (table `border-spacing`), `cellpadding` (padding of the table's
own cells) and `valign` (`vertical-align` of table parts). Nearly every HTML
email sets `cellspacing="0" cellpadding="0"`; without them colored cells sat
apart, with seams of the page between them.

Remove when upstream supports these attributes.

### 0004 Selection hits

`Node::hit_inner` in `src/node/node.rs` and `find_text_position` in
`src/document.rs`. An inline root's children (inline blocks, images, floats)
are laid out and painted relative to its border box, but upstream hit-tested
them relative to its content box: a pointer reached them shifted right and
down by the inline root's padding and border. Responsive emails put their
columns in inline blocks inside padded cells, so a selection began and ended
well to the right of the pointer, and links there were clicked off target.
Its text hit also took the line box's vertical offset away twice, so text in
a vertically centered cell was selected a line or more away. A press on a box
inside an inline root (an image) now finds the text position in the inline
root's coordinates rather than the box's.

Remove when upstream hit-tests inline roots' children against their border
box.

### 0005 Blocks in tables

`collect_table_cells` in `src/layout/table.rs`. Email templates (Klaviyo's
among them) lay columns out as `display: table-cell` divs in a
`display: table` div, and below 480 pixels make the columns
`display: block` to stack them. A block or inline box directly in a table or
row group is wrapped in an anonymous row and cell in CSS. Upstream dropped it,
so on a phone such an email showed almost nothing. Each such box now gets an
anonymous row of its own and is laid out as its cell (`TableRow::anonymous`,
whose `node_id` is the box), so restacked columns stack. Unlike CSS,
consecutive such boxes each get a row rather than sharing one. blitz-paint's
0002 depends on `TableRow::anonymous`.

Remove when upstream wraps blocks in tables in anonymous rows and cells.

### 0006 Email legacy attributes

`synthesize_presentational_hints_for_legacy_attributes` in `src/stylo.rs`.
Email HTML is written with the presentational attributes of old HTML, which
upstream mapped only partly:

- `align` gave every element a `text-align`. On a table it centers or floats
  the table, not its content: `align="center"` now gives it auto margins
  (left and right float it, in the UA sheet) and its cells' text stays put.
  On an image (and other replaced elements) left and right float it and the
  other keywords set `vertical-align`. Keywords match case-insensitively, and
  `justify` justifies.
- `background` on the body and table parts is a background image.
- `<font color size face>` and `<body text>` set the color, the legacy font
  size (1 to 7, or relative to 3) and the font family.
- `bgcolor` read only `#rgb` and `#rrggbb`; it now follows the legacy color
  rules, so `bgcolor="white"` and `bgcolor="ffffff"` work too.

Attribute values written into CSS are quoted, so an attribute can't add
declarations of its own.

Remove when upstream maps these attributes.

### 0007 Media device pixel ratio

`make_device` in `src/stylo_device.rs` and
`BaseDocument::set_media_device_pixel_ratio` in `src/document.rs`. Picacho
lays emails out in CSS pixels (a viewport scale of 1) and rasterizes them at
the screen's scale itself, so `resolution` and `-webkit-device-pixel-ratio`
media queries always saw 1, and emails' sharper images for HiDPI screens were
never used. The new setter gives `@media` rules the ratio to see without
changing the layout's scale; `None` keeps upstream's behavior.

Remove when upstream lets the media device pixel ratio differ from the
layout scale.

### 0008 Media preferences

`BaseDocument::set_media_preferences` (`src/document.rs`), with
`MediaPreferences` and `Contrast` (`src/stylo_device.rs`, exported): what
`prefers-reduced-motion` and `prefers-contrast` media queries see (Media
Queries 5 §11.3, §11.4), from the embedder's or the system's settings. They
are kept on the document and applied each time the device is rebuilt, as
0007's pixel ratio is. Needs `stylo` 0001.

Remove when upstream passes these preferences to Stylo.

### 0009 SVG images stay inside

`parse_svg_image` in `src/util.rs` and `SvgImageData::from_data` in
`src/node/svg.rs`. Upstream parsed every SVG (an `<svg>` element written in
the document, or an SVG image) with usvg's default options, whose string
resolver reads whatever path an `<image>` or `<feImage>` names from disk
(`std::fs::read`), whatever `resources_dir`, past the document's net
provider. An email could have local files shown (and printed into a PDF),
`/dev/zero` read without end, and on Windows a UNC path
(`\\host\share\x.png`) opened over SMB, handing that host the reader's
NTLM hash, all with remote content blocked. Now `<image>` loads only
pictures inside the SVG (`data:` URLs), not gzip-compressed ones, and
`from_data` refuses gzip (SVGZ), which usvg inflated without limit: the
embedder inflates it, within its limits, before handing it over.

Keep: Picacho relies on it for mail.

## Upgrading

```sh
scripts/vendor.sh rebuild blitz-dom <version>
```

Then update the version above and in Picacho's `Cargo.toml`.
