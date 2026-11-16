# Exact atomic comparisons, second pass

This clean-room pass fixes Sharpen's Float32 arithmetic and Channels Shuffle's selector
handling. It builds on the [first atomic corrections](atomic-parity-improvements.md).
All graphs and inputs are original procedural probes. No proprietary implementation or
asset was used. Catalog membership and `assets/sbs` are unchanged.

## Solved behavior

- Sharpen now forms each axial second difference before multiplying by strength, adds
  the two weighted differences, then adds the center. The previous algebraically
  equivalent expression rounded differently. All 40 new tiled arithmetic probes match
  bit for bit, across square/rectangular images, fractional strengths, ramps and RGBA
  patterns. All 27 original Sharpen cases match with the reference SSE2 CPU count set to one.
- Channels Shuffle now offers the eight measured source channels. The old Black/White
  choices conflicted with saved channel selectors. Color swizzles use the low three
  selector bits; grayscale selection happens before that swizzle. Oversized and signed
  file values survive import, evaluation and save/load until the connected image kinds
  determine their result. The 33 original cases, ten gray-selector kernel cases and all
  64 new selector/kind cases match SSE2 exactly.

Existing graphs using the former Black/White choices now follow the measured selector
behavior. Use a Uniform Color input when a constant black or white channel is intended.
Defaults and the eight ordinary source choices retain their meanings.

Regression coverage is in `crates/nodes/tests/atomic_filter_parity.rs` and
`crates/formats/src/sbs/tests.rs`. The new ramp, selector and import checks exposed failures
before their corrections; coverage was expanded to mixed input kinds and signed round trips.
The runner also accepts bounded JSON recipes, dumps original inputs with diagnostic pixels,
and records the reference backend and requested CPU count.

## Measured results

The [case ledger](parity/atomic-exact-results.csv) retains failures and unchanged cases.
Before means the preceding atomic-correction working tree described in the first report,
not the older committed catalog audit. Both sides use scalar EXR Y for gray inputs and
Float32 RGBA for color inputs. Comparisons count differing Float32 bit patterns, differing
pixels, maximum finite absolute error and non-finite mismatches separately.

| Original cases, SSE2 automatic CPU allocation | Cases | Exact before → after | Component mismatches before → after | Maximum absolute error after |
|---|---:|---:|---:|---:|
| Sharpen | 27 | 2 → 21 | 65,200 → 832 | 0.0161290467 |
| Channels Shuffle | 33 | 29 → 33 | 1,920 → 0 | 0 |
| Blur | 27 | 10 → 10 | 53,048 → 53,048 | 3.57627869e-7 |
| Directional Blur | 29 | 18 → 18 | 36,646 → 36,646 | 0.00259685516 |

The original 1,817-case suite gains 25 exact images, including two gray-selector kernel
cases outside this table. The 291 new cases improve from 124 to 179 exact images. Across
these 2,108 comparable cases, **80 additional images become exact and none loses exactness**.
There are 779 exact images, 393 exact typed values, 698 image differences, 104 typed
differences, 82 local errors, 32 size differences, 19 reference cooker errors and one
unavailable numeric typed readback. These counts are not a catalog parity percentage.

The 40 new Sharpen arithmetic cases remove 82,087 component mismatches (previous maximum
error 3.57627869e-6). The 64 new Shuffle selector/kind cases remove 27,648 component
mismatches (previous maximum error 0.875). Reference pixels stayed identical between
the before and after captures, including the signed-selector serialization correction.

Four original Sharpen cases have a slightly larger remaining maximum error, changing
from 0.0161290169 to 0.0161290467. Their residual boundary discrepancy is described below;
the corrected arithmetic removes other differing components. No general error-reduction
claim is made beyond the reported measurements.

## Reference settings can change pixels

The runtime is 16.0.7. Its SSE2 and D3D11 renderers were measured separately. The ledger
includes 54 SSE2 thread-control measurements and 98 D3D11 backend-control measurements
in addition to the 2,108 primary cases: 2,260 measurements in total.

- **Thread boundaries:** automatic CPU allocation leaves six original non-tiled Sharpen
  cases different. For a 32×16 ramp, 16 values in the reference change when CPU count is
  set to one. For the 64×64 color case, 384 components change along internal stripe
  boundaries. With one CPU, all 27 original Sharpen images are exact: zero mismatches,
  maximum error zero. The same CPU-count change leaves all 27 reference Blur images
  unchanged. MaterialCraft does not emulate the reference's thread-dependent seams.
- **Integer precision:** for an input component of 0.5 at U8, SSE2 returns 128/255 while
  D3D11 and MaterialCraft return 127/255. A U16 Dot input with negative/HDR components
  passes through SSE2 unchanged in these probes; D3D11 and MaterialCraft clamp and
  quantize it. The same difference appears with explicit absolute node/input formats.
  These observations do not establish a general format-inheritance bug.
- **Negative float values:** constant Float32 Sharpen input -0.5 produces 0 on SSE2 and
  -0.5 on D3D11. MaterialCraft currently matches SSE2's lower clamp. The 32 constant
  precision controls have 16 exact images on SSE2 and 28 on D3D11; two additional D3D11
  Sharpen pattern controls remain different.
- **Invalid selectors:** all 64 selector/kind cases match SSE2, and 63 match D3D11. With
  gray input 1, color input 2 and selector 11, SSE2 returns the converted gray alpha (1),
  while D3D11 returns the gray scalar (0.125). All 32 ordinary selector/kind combinations
  in that group match both backends. The other 32 exercise out-of-range file values.

One deterministic output cannot equal two different reference outputs for the same graph.
Backend/thread agreement therefore cannot be claimed from any one of these configurations.

## Still different or untested

Blur retains small arithmetic differences. This pass tested axis order, direct versus
running accumulation and rounding order without finding a verified improvement suitable
for a production change. Directional Blur retains sampling differences: wider angled
kernels in the reference have position-dependent footprints. Shifted original impulses
confirm this is more than a final storage rounding error. The new angled group reaches
maximum error 0.137573063 at angle 0.333, strength 2. These filters are unchanged this pass.

Non-dyadic half-float conversion, other precision paths, arbitrary angles and strengths,
all other backends, mixed input/output resolutions, non-default Sharpen alpha behavior,
and nonempty Text/SVG rasterization remain unestablished. Earlier isolated non-finite,
processor and nested map probes were replayed; their earlier limits remain. Arbitrary
NaN payloads and combinations of exceptional samples are untested. The 954 inventoried
package function bodies remain untested. See the [catalog report](node-catalog-parity.md)
for the full inventory and typed-value readback limitations.

## Reproduction

Use an empty result directory when changing reference runtime versions. Reference caches
are keyed by graph/input content, backend and CPU-count override, not installed runtime
version. The existing transport-only SBS version-header adjustment remains required.
The runner preserves the original graph, transport graph, raw EXR, logs and report.

```powershell
cargo build -p materialcraft-engine --release --example catalog_parity
$env:MC_PARITY_DUMP_PIXELS = '1'
$env:MC_PARITY_REFERENCE_ENGINE = 'sse2'
Remove-Item Env:MC_PARITY_REFERENCE_CPUS -ErrorAction SilentlyContinue
Remove-Item Env:MC_PARITY_CASE_FILE -ErrorAction SilentlyContinue
target/release/examples/catalog_parity.exe C:/parity/catalog C:/reference/tools

$env:MC_PARITY_CASE_FILE = "$PWD/docs/parity/atomic-exact-recipes.json"
target/release/examples/catalog_parity.exe C:/parity/extra C:/reference/tools

Remove-Item Env:MC_PARITY_CASE_FILE
$env:MC_PARITY_REFERENCE_CPUS = '1'
target/release/examples/catalog_parity.exe C:/parity/sharpen-one-cpu C:/reference/tools atomic_sharpen_
target/release/examples/catalog_parity.exe C:/parity/blur-one-cpu C:/reference/tools atomic_blur_

Remove-Item Env:MC_PARITY_REFERENCE_CPUS
$env:MC_PARITY_REFERENCE_ENGINE = 'd3d11'
$env:MC_PARITY_CASE_FILE = "$PWD/docs/parity/atomic-exact-recipes.json"
target/release/examples/catalog_parity.exe C:/parity/gpu-precision C:/reference/tools exact_precision_
target/release/examples/catalog_parity.exe C:/parity/gpu-sharpen C:/reference/tools exact_sharpen_math_4x4_0.1_
target/release/examples/catalog_parity.exe C:/parity/gpu-shuffle C:/reference/tools exact_shuffle_
```

The [291 recipes](parity/atomic-exact-recipes.json) describe generated ramps, dyadic grids,
constants and isolated impulses. Sizes include 16, 32, 64 and 256 pixel sides, both
rectangular orientations, tiling boundaries, several strengths/angles and all four output
formats. Sixteen exploratory recipes using the wrong local parameter variant (`Int` for
an enum) were replaced by valid enum recipes; they are not counted as parity improvements.
Diagnostic JSON encodes non-finite values as null; the actual comparison uses raw image bits.

## Validation

- Release tests for `materialcraft-nodes`, `materialcraft-formats`, `materialcraft-eval`
  and `materialcraft-engine` pass with the known resource test below excluded. This
  includes the new regressions, `every_node_cooks` and the engine's `panic_hunt`.
- Workspace Clippy with `--all-targets -- -D warnings` passes.
- `cargo xtask layers` passes for 14 crates; `cargo xtask wasm` passes for ten crates.
- Formatting and diff whitespace checks pass.

The separately rerun `byte_only_sbs_import_resolves_library_resource_substitutes` test
still fails at `crates/engine/tests/package_resources.rs:32`, in the existing saved
missing-font-path assertion. No asset or unrelated resource-handling change was made to
hide this failure. A completely green release suite is therefore not claimed.
