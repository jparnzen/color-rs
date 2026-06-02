# color-rs

A Cargo workspace implementing CSS Color 4 color science in Rust — a pure library, WASM bindings for browser use, and a CLI tool for command-line color conversion and gamut mapping.

## Origin

color-rs grew from [color-wasm](https://github.com/jparnzen/color-wasm), an earlier single-crate implementation combining the core color library with WASM bindings. As the project expanded to include a CLI tool, the single-crate structure became limiting — a CLI binary shouldn't carry wasm-bindgen as a dependency. The workspace refactor separates these concerns cleanly while keeping the same color science at the core.

## Workspace structure

```
color-rs/
├── color-lib/     # Pure Rust color science library — no WASM dependencies
├── color-wasm/    # Thin wasm-bindgen wrapper over color-lib
└── color-cli/     # CLI tool for color conversion and gamut mapping
```

## color-lib

The core library. Implements CSS Color 4 color spaces and gamut mapping in pure Rust, with all math in `f64`.

**Color spaces**: sRGB, sRGB-Linear, XYZ-D65, OKLAB, OKLCH, OKLrAB, OKLrCH — connected through an XYZ-D65 hub, matching the colorjs.io architecture and the CSS Color 4 model.

**Type-safe by design**: `Color<S>` uses a phantom type parameter to encode color space at compile time. Converting between incompatible spaces is a compile error, not a runtime panic.

**Gamut mapping**: Both CSS Color 4 algorithms:
- Binary search with local MINDE (`gamutmap_local_minde`) — convergence via ΔEok with JND=0.02
- Raytracing via AABB intersection in linear-light space (`gamutmap_raytrace`)

**Spec fidelity**: Matrix constants expressed as rational fractions (e.g. `506752. / 1228815.`), matching the CSS Color 4 specification rather than truncated approximations. Inline comments link to the relevant spec sections throughout.

**Spec errata**: Two errors found in the CSS Color 4 draft specification during implementation were reported to the CSS Working Group ([issue #10579](https://github.com/w3c/csswg-drafts/issues/10579)):
- [`cast_ray` step 5.5](https://github.com/w3c/csswg-drafts/issues/10579#issuecomment-4122988782): condition inversion (`< 1e-12` should be `> 1e-12`)
- [Raytrace step 11](https://github.com/w3c/csswg-drafts/issues/10579#issuecomment-4122677476): incorrect variable reference (`clip(current)` should be `clip(origin_rgb)`)

**Verified**: All test values confirmed against the [Coloraide](https://facelessuser.github.io/coloraide/) Python reference library.

## color-wasm

Thin wasm-bindgen bindings over color-lib. Exposes `srgb_to_oklch` and `oklch_to_srgb` for browser use. Uses `f32` at the WASM boundary (intentional precision trade-off for JS interop); all internal calculations in color-lib remain `f64`.

Build with [wasm-pack](https://rustwasm.github.io/wasm-pack/):

```bash
cd color-wasm
wasm-pack build
```

## color-cli

A command-line tool for color conversion and gamut mapping. Built because checking color values and gamut mapping behavior from the command line is faster than loading a library in a Node or Python REPL every time — the same motivation behind the [color-palette-explorer](https://github.com/jparnzen/color-palette-explorer) and the [Firefox gamut mapping contribution](https://bugzilla.mozilla.org/show_bug.cgi?id=1847503).

```
USAGE:
    color-cli --format <FORMAT> [--gma <GMA>] <COLOR>

ARGUMENTS:
    <COLOR>    CSS color function string or hex value

OPTIONS:
    --format <FORMAT>    Output color space (required)
                         [possible values: rgb, oklch, oklab, oklrch, oklrab]
    --gma <GMA>          Gamut mapping algorithm (optional)
                         [possible values: clip, minde, raytrace]
    -h, --help           Print help
```

**Examples:**

```bash
# Convert a hex color to OKLCH
color-cli "#0088c6" --format oklch

# Apply raytrace gamut mapping, output as OKLCH
color-cli "oklch(0.7 0.3 200)" --format oklch --gma raytrace

# Clip an out-of-gamut color to sRGB
color-cli "oklch(0.7 0.3 200)" --format rgb --gma clip
```

**Build and install:**

```bash
cargo build --release
cargo install --path color-cli
```

## Getting started

Requires Rust (edition 2024). Clone the repository and build all crates:

```bash
git clone REPO-URL
cd color-rs
cargo build
cargo test
```

## A note on process

The code in this project was hand-written and developed iteratively, with Claude (Anthropic) and Claude Code used for code review, bug identification, and Rust mentorship throughout. The README was drafted with Claude's assistance — the technical knowledge, motivations, and decisions described are my own; the concision is Claude's.

## License

MIT © 2026 John P. ARNZEN — see license header in each source file.
