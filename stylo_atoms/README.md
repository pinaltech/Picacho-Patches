# stylo_atoms

Upstream: [`stylo_atoms`](https://crates.io/crates/stylo_atoms) 0.22.0, from
[servo/stylo](https://github.com/servo/stylo).
License: MPL-2.0 (`LICENSE`; the source files carry their original headers).
Copyright belongs to The Servo Project Developers.

`crate/` is the crates.io release with `patches/` applied in order.

## Patches

### 0001 Media preference atoms

`static_atoms.txt` gains `prefers-contrast` and `prefers-reduced-motion`, the
names of the media features `stylo` 0001 adds.

Remove with `stylo` 0001.

## Upgrading

```sh
scripts/vendor.sh rebuild stylo_atoms <version>
```

Keep it in step with `stylo`.
