# stylo

Upstream: [`stylo`](https://crates.io/crates/stylo) 0.22.0, from
[servo/stylo](https://github.com/servo/stylo).
License: MPL-2.0 (`LICENSE`; the source files carry their original headers).
Copyright belongs to The Servo Project Developers.

`crate/` is the crates.io release with `patches/` applied in order.

## Patches

### 0001 prefers-reduced-motion and prefers-contrast

Servo's media features (`servo/media_features.rs`) had no
`prefers-reduced-motion` or `prefers-contrast` (Media Queries 5 §11.3,
§11.4), so an email's styles for either were parsed as unknown and never
applied. Servo's `Device` (`device/servo.rs`) now keeps both, set with
`set_prefers_reduced_motion` and `set_prefers_contrast` (no preference
until set), and the two features evaluate them, keyword and boolean forms
alike. Their values are new in `queries/values.rs`, after Gecko's. The atoms
come from `stylo_atoms` 0001.

`Device::new` is unchanged, so existing embedders need no change. blitz-dom's
0008 sets both from the embedder.

Remove when upstream's Servo device supports these features.

## Upgrading

```sh
scripts/vendor.sh rebuild stylo <version>
```

Then update the version above and in Picacho's `Cargo.toml`. Keep it in step
with `stylo_atoms`, `stylo_derive` and blitz-dom's `style` dependency.
