# gpui-pre-wgpu

Upstream: [`gpui-pre-wgpu`](https://crates.io/crates/gpui-pre-wgpu) 0.3.12, a
snapshot of Zed's `gpui_wgpu` crate (zed@d2a4fb6) from
[zed-industries/zed](https://github.com/zed-industries/zed).
License: Apache-2.0 (`LICENSE-APACHE`). Copyright belongs to Zed Industries
and the gpui authors.

`crate/` is the crates.io release with `patches/` applied in order.

## Patches

### 0001 Backdrop blur

Draws `gpui-pre` 0001's `BackdropBlur` (`src/backdrop_blur.rs`,
`src/backdrop_blur.wgsl`). At each blur the main render pass ends; the
region, widened by the blur's reach, is copied out of the frame, scaled down
by a power of two that keeps the Gaussian within a few dozen taps, blurred
horizontally then vertically, and drawn back scaled up, clipped to its
rounded rectangle; the main pass then resumes over it. Pipelines and
textures are made when a frame first has a blur.

The window surface is configured with `COPY_SRC` where it supports it
(`surface_configuration`); without it, blurs are left out. `render_frame`
takes the frame's texture for the copy. The headless renderer's target
already allowed copies.

Tests (`wgpu_renderer.rs`, headless): content below a blur mixes across an
edge while what is drawn after it stays sharp; a scaled-down blur keeps to
its rounded rectangle.

Remove with `gpui-pre` 0001.

## Upgrading

```sh
scripts/vendor.sh rebuild gpui-pre-wgpu <version>
```

Keep it in step with `gpui-pre`.
