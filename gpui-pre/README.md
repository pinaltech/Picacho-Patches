# gpui-pre

Upstream: [`gpui-pre`](https://crates.io/crates/gpui-pre) 0.3.12, a snapshot
of Zed's `gpui` crate (zed@d2a4fb6) from
[zed-industries/zed](https://github.com/zed-industries/zed).
License: Apache-2.0 (`LICENSE-APACHE`). Copyright belongs to Zed Industries
and the gpui authors.

`crate/` is the crates.io release with `patches/` applied in order.

## Patches

### 0001 Backdrop blur

A new scene primitive, `BackdropBlur`, and `Window::paint_backdrop_blur`,
which blurs what was painted before it within a rounded rectangle, as CSS's
`backdrop-filter: blur()` does. Picacho covers its window with it while it
closes.

`src/scene.rs`: blurs are kept in `Scene::backdrop_blurs`, sorted by draw
order like the other primitives, and replayed with cached views. The batch
iterator stops batches at each blur's order but yields no batch for it, so
`PrimitiveBatch` is unchanged and renderers that know nothing of blurs (Metal,
DirectX) keep compiling and simply leave them out. A renderer that draws them
places each before the first batch of a greater or equal order
(`Scene::batch_order`). `src/window.rs`: `paint_backdrop_blur`.

Drawn by `gpui-pre-wgpu` 0001 (Linux, and the BSDs).

Remove when upstream has a backdrop blur.

## Upgrading

```sh
scripts/vendor.sh rebuild gpui-pre <version>
```

Keep it in step with `gpui-pre-wgpu`, then update the version above and in
Picacho's `Cargo.toml`.
