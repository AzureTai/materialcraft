# Atomic filter and background corrections

This is the first-pass record. The [exact-comparison follow-up](atomic-exact-parity.md)
supersedes its Sharpen arithmetic, Shuffle selector and precision/backend conclusions.

This clean-room follow-up corrects six existing atomic nodes: Blur, Directional Blur,
Sharpen, Channels Shuffle, Text and SVG. It changes no catalog membership or bundled
SBS assets. Numerical agreement applies only to the recorded cases, runtime and backend.

## Solved behavior

- Blur now integrates a box footprint with fractional endpoint coverage. The previous
  Gaussian kernel spread a different amount of energy to neighboring pixels. Running
  sums keep each horizontal/vertical pass linear in image size.
- Directional Blur now uses the measured full radius and endpoint coverage, and rotates
  in the measured image-space direction. Oblique kernels remain approximate.
- Sharpen now uses the measured four-neighbor Laplacian, processes alpha, and clamps
  negative lobes while retaining HDR output. Rectangular images weight each axis separately.
- All three filters use the shorter image side for their measured size scaling and honor
  horizontal and vertical tiling independently. Changing tiling invalidates the cook cache.
- Channels Shuffle reads a grayscale input's scalar through every channel selector,
  including alpha. Color inputs retain their actual alpha channel.
- Empty Text/SVG content preserves the background, including hidden RGB under zero alpha.
  A background of the wrong image kind supplies empty pixels instead of an implicit
  color conversion. Matching grayscale backgrounds retain their original scalar bits.

The regressions in `crates/nodes/tests/atomic_filter_parity.rs` and
`crates/eval/tests/filter_tiling.rs` were observed failing before their corrections.
They cover fractional box footprints, full directional extent, sharpen gain and clipping,
gray channel selection, transparent backgrounds, rectangular scaling, independent tiling
and cache invalidation.
The existing save/load fixtures now use a visible blur radius at their 32-pixel preview
size and backgrounds with the same image kind as their target. Their live binding and
connection-preservation assertions remain in place.

## Measured improvement

The [per-case ledger](parity/atomic-improvement-results.csv) records all 1,817 rerun/new
cases, including failures and unchanged processor/map cases. Its before columns use the
implementation at `9f390d4` with corrected input encoding. There are 1,773 comparable
before/after cases; the 44 additional precision/non-finite cases have no before claim.

For the 182 existing cases of the six changed nodes, exact images increase from 25 to
125. Component mismatches decrease from 359,466 to 156,814. The table includes every
existing case for each node, including remaining extension/boundary differences:

| Node | Cases | Exact before → after | Component mismatches before → after | Largest absolute error before → after |
|---|---:|---:|---:|---:|
| Blur | 27 | 2 → 10 | 81,836 → 53,048 | 0.398878 → 3.57628e-7 |
| Directional Blur | 29 | 2 → 18 | 75,182 → 36,646 | 0.400680 → 0.00259686 |
| Sharpen | 27 | 2 → 2 | 69,280 → 65,200 | 14.7046 → 0.0161290 |
| Channels Shuffle | 33 | 17 → 29 | 20,992 → 1,920 | 1 → 1 |
| Text, empty content | 37 | 1 → 37 | 29,067 → 0 | 0.990975 → 0 |
| SVG, empty content | 29 | 1 → 29 | 83,109 → 0 | 1 → 0 |

The four remaining original Shuffle cases select its local White extension. All eight
added grayscale source-channel selectors match exactly after the alpha correction.
An unchanged maximum for that node does not negate the fixed alpha cases.

Of the 158 new kernel/background cases measured both before and after, exact results
increase from 63 to 136. Across all 1,773 comparable cases, **173 additional cases become
exact**, no previously exact case becomes inexact, and no recorded maximum finite error
increases. This does not establish behavior outside those probes.

The final 202-case kernel group has 158 exact images and 44 image differences. Its 12
isolated non-finite cases are exact. The oblique-angle group still reaches error 0.101213
on an impulse at angle 0.333 and strength 2.75; the separate Sharpen detail group reaches
0.0625 in some non-tiled impulse cases. These larger new cases are not hidden by
the table of existing cases above.

The full 1,817-case ledger contains 575 exact images, 393 exact typed values, 611 image
differences, 104 typed differences, 82 local errors, 32 size differences, 19 reference
cooker errors and one unavailable numeric typed readback. Comparable images contain
10,665,728 components, with 1,279,965 component mismatches across 689,232 pixels.
There were no harness or renderer-process failures. Full catalog parity is unestablished.

## Measurement correction

The earlier runner exported every image input as RGBA. Grayscale input bridges in the
reference therefore performed an extra conversion before the node ran. The runner now
writes grayscale inputs as a single Float32 EXR Y channel and color inputs as RGBA.
Its cache version changed so old captures cannot be reused with the new input encoding.

Both the before and after results in this follow-up use the corrected encoding. Gains
caused solely by that correction are not counted as production fixes. For example, all
25 existing Dot cases match with corrected inputs before any production edit. The
[previous results](parity/case-results.csv) remain a historical record and must not be
compared directly to the new totals to estimate the effect of these fixes.

## Reproduction and scope

The reference runtime remains 16.0.7, using its SSE2 renderer. Inputs are original impulses,
coordinate ramps, checks, constant backgrounds and HDR/alpha patterns. No proprietary
implementation, graph, font or image is included. The catalog inventory remains 29 main
entries, 85 function operations, four measured map kinds and 954 loaded package functions.
Those package function bodies remain untested by this work.

```powershell
cargo run -p materialcraft-engine --release --example catalog_parity -- C:/parity/results C:/reference/tools
cargo run -p materialcraft-engine --release --example catalog_parity -- C:/parity/kernels C:/reference/tools kernel_
```

The second command selects the 202 added cases. They include 16, 32, 64 and 256 pixel sides,
both rectangular orientations, centered and boundary impulses, fractional strengths,
several angles, color/gray inputs, HDR and transparent RGB, U8/U16/F16/F32, and isolated
NaN/positive-infinity/negative-infinity inputs. Literal non-finite values are supplied through
original EXR inputs, not serialized as unsupported SBS constants. Existing probes retain
unconnected inputs, four tiling flags, processors, nested maps, seeds and error cases.

Set `MC_PARITY_DUMP_PIXELS=1` for an additional `pixels.json` diagnostic beside each EXR.
The JSON is not a substitute for raw non-finite bit comparison: JSON encodes those values
as null. Raw image comparisons still use Float32 bits, with component/pixel mismatch counts,
maximum finite error and a separate non-finite mismatch count. Typed comparisons retain
the limitations described in [the catalog measurements](node-catalog-parity.md).

The original probe, transport SBS, raw EXR, renderer logs and report are retained in the
chosen output directory. The existing two-field SBS version-header transport adjustment
is still required. Unmodified cross-application file compatibility is not established.

## Remaining limits

Exact full-node parity is not claimed. Oblique Directional Blur still differs in its
sampling footprint and subpixel rounding; its sample budget remains bounded for large
strengths. Sharpen retains some boundary and floating-point arithmetic differences.
Its additional native alpha-mode control is still absent; only the default mode was measured.
Integer/half-float output quantization and bound-image format inheritance still differ;
an error in those paths must not be attributed solely to the filter kernel.

Channels Shuffle's local Black/White selectors have no corresponding measured native
selector in this transport; these extension cases remain different. Text uses empty text
in the reference probes, so font rendering is untested. SVG corrections establish
background handling, not complete SVG rasterization equivalence. Non-finite cases cover
single exceptional samples; arbitrary payloads and interacting exceptional values are
untested. The added filter probes use inputs with the same dimensions as their outputs;
resizing mismatched inputs and mixed source/node tiling remain untested here.
Filter widths beyond the recorded cases, arbitrary angles, backend equivalence,
and other processors/maps retain the limitations in the catalog report.

Release timing is reproducible with:

```powershell
cargo run -p materialcraft-nodes --release --example filter_timing
```

It cooks original RGBA patterns at 512 and 1024 square pixels, three repetitions per
filter, with Blur strength 32, Directional Blur strength 10 at one eighth turn, and
Sharpen strength 10. These are timing measurements, not additional reference comparisons.

| Filter | 512×512 mean milliseconds/cook | 1024×1024 mean milliseconds/cook |
|---|---:|---:|
| Blur | 34.445 | 140.386 |
| Directional Blur | 83.274 | 549.661 |
| Sharpen | 3.367 | 9.425 |

These local release timings are environment-dependent. They establish neither a speed
comparison with the reference nor worst-case performance at the maximum supported size.

## Validation

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo xtask layers`: passed for all 14 checked crates.
- `cargo xtask wasm`: passed for all ten checked crates.
- Release tests for `materialcraft-nodes`, `materialcraft-eval` and `materialcraft-engine`
  pass with `--skip byte_only_sbs_import_resolves_library_resource_substitutes`.
  All seven new regressions pass; each was observed failing before its correction.
- The full release command was also run. Its remaining failure is the previously recorded
  missing-font resource round-trip assertion in `crates/engine/tests/package_resources.rs:32`.
  That unrelated test remains unresolved; it is not counted as passing. Existing ignored
  tests were not enabled.

The final working changes were reviewed. No files under `assets/sbs` changed, and no Git
write, commit, push or remote comment was performed for this follow-up.
