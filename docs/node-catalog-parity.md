# Native catalog measurements

The [atomic corrections follow-up](atomic-parity-improvements.md) records subsequent filter
and background fixes. It also corrects grayscale probe input encoding; the figures below
retain the original run and must not be used as the before baseline for those fixes.
The [second atomic pass](atomic-exact-parity.md) fixes Sharpen arithmetic and Shuffle
selectors, and separates backend/thread-dependent differences from local defects.

This is a clean-room audit of MaterialCraft's actual catalogs on 2026-11-15, based on
`main` at `271890a`. Catalog coverage is not behavioral parity. Exact results apply only
to the individual generated cases, reference runtime and backend recorded here.

## Inventory

| Catalog | Actual entries | Evidence |
|---|---:|---|
| Main graph | 29: 25 primitives and four input/output items | MaterialCraft catalog and reference runtime definition enumeration |
| Function graph | 85 | MaterialCraft function catalog and reference runtime definition enumeration |
| FX-Map graph | 4 | MaterialCraft pattern definitions and original render probes for each kind |
| Loaded package functions | 954, including 689 hidden entries | Actual loaded package inventory; behavior untested in this audit |

The earlier 26-primitives/30-total documentation count included `motion_blur`. Commit
`a0032d6` removed that entry and its SBS mapping before the audited base, leaving 25 primitives
and four input/output items. That former MaterialCraft entry was a wrapper around Directional
Blur and had no separate counterpart in the enumerated reference atomic catalog. The current
count includes Dot/reroute among the 25 primitives. This audit changed no catalog membership.

[Native entries](parity/native-catalog.csv), [default comparisons](parity/native-defaults.csv),
[package function inventory](parity/packaged-functions.csv), and
[per-case results](parity/case-results.csv) are machine-readable. Hidden package functions
are included so the visible Library does not hide audit omissions. Package inventories
establish presence only; they do not establish equivalence, provenance or redistribution rights.
Earlier [SBS root assessments](sbs-root-nodes.md) and [parity corrections](parity-fixes.md)
are separate measurements; their cases were not rerun or promoted by this audit.
The reference SDK returned no definitions for its FX-Map graph resource, so the four-kind
inventory is not claimed as a complete enumeration of that runtime's map definitions.

The [native coverage index](parity/native-coverage.csv) links all 118 native entries to their
baseline cases, including failures. The default ledger has 91 equal observations and 45
unmeasured entries after corrections; absence of a primitive default is not an equality claim.

## Recorded results

The SSE2 run contains 1,615 cases. There is no overall parity percentage: fixtures overlap,
boundary/rejection probes are included, and most behavior combinations remain untested.

| Result | Cases |
|---|---:|
| Exact raw image components | 254 |
| Exact typed values | 384 |
| Image component differences | 730 |
| Typed value differences | 113 |
| Local cook/evaluation errors | 82 |
| Output size differences | 32 |
| Reference cooker errors | 19 |
| Numeric typed readback unavailable | 1 |

Comparable images contain 4,184,576 measured components, with 1,565,349 component mismatches
across 838,708 pixels. Typed reports describe 874 components, including one unavailable
numeric component. Each case reports its
own maximum error and mismatch count; mixing integer errors, normalized pixels and HDR errors
into a single maximum would be misleading. All compared images had their required channels;
none of the exact image cases relied on an invented alpha channel. No harness or renderer
process failed in this run.

Five cooker errors are the two constant-string output probes, two string-get output probes
and unconnected passthrough. Fourteen more reject Pixel Processor outputs of Float2, Float3,
Boolean or integer types in gray/color mode. Their local errors are recorded independently;
Float2/Float3 are accepted locally, while the reference transport is rejected. Strings are
also tested through original equality graphs that
produce a legal material value; rejection of a string material output does not demonstrate
that a string operation is unsupported. The unavailable numeric readback is the nested
function assignment case. Missing-resource output sizes account for part of the size group;
for example, the missing Bitmap produces 16×16 locally versus 256×256 in its reference probe.

## Method and reproduction

The reference desktop runtime was 16.0.7 with MCP bridge 1.1.0. MCP opened original scratch
packages and enumerated graph definitions. The public SDK inspected native port types and
simple numeric/Boolean defaults. Nonempty product strings, descriptions and complex asset
defaults were excluded. The bridge cannot author function bodies or return raw pixels;
original SBS probes were therefore cooked and rendered with the installed command-line tools.
An original SDK-authored loop also isolated the loop-limit serialization failure.

Run from the repository root, with an absolute writable output directory and the directory
containing the reference `sbscooker.exe` and `sbsrender.exe` tools:

```powershell
cargo run -p materialcraft-engine --release --example catalog_parity -- C:/parity/results C:/reference/tools
```

An optional final argument selects case names by substring, for example `function_dot_`,
`control_loop_`, `pixel_sample_`, `fx_iterate_` or `precision_`. Omitting the reference-tool
directory inventories the catalogs and cooks the original probes locally, marking comparisons
untested. `MC_PARITY_REFERENCE_ENGINE` selects a different installed backend; the recorded
baseline uses `sse2`. Backend equivalence is untested.

The example is in [catalog_parity.rs](../crates/engine/examples/catalog_parity.rs). Each case
retains `probe.sbs`, the transport copy `reference.sbs`, any original procedural resources,
reference EXR, cooker/renderer logs and exit status. `inventory.json`, `library-inventory.json`
and `report.json` record catalogs and results. Reference results are reused only when the probe,
input pixels and selected backend have the same SHA-256 key; local cooking runs again.
Use a new directory when changing the installed runtime. No installed library graph, preset,
font, image or other proprietary asset is used as a probe input. Image inputs are generated
coordinate ramps, a checker and an alpha ramp; resource probes use original PGM and SVG data.
Text probes use empty text, so reference glyph/font rendering is untested.

**Transport limitation:** MaterialCraft writes its application version `0.4.1` into the SBS
format/updater headers. The reference rejects that header. The runner preserves the original
file and changes only those two fields in a separate transport copy to `1.1.0.202502`, measured
from an empty reference package. Numerical results therefore do not establish that an
unmodified MaterialCraft file opens in the reference app. This remains a separate format gap.

Reference images are exported as Float32 EXR without display transforms and compared with the
actual CPU cook, before any preview or texture export. Only channels present in the reference
EXR are compared. Gray compares its one scalar channel; color compares available RGBA channels.
Missing alpha is explicitly unmeasured. Image components use Float32 bit equality, including
signed zero and NaN payloads. The report records component and pixel mismatch counts, sample
counts and maximum finite absolute error. A non-finite mismatch makes the overall maximum
error unavailable. Size/kind mismatches and cook errors are recorded before numerical comparison;
error-placeholder images never count as numerical evidence.

Typed reference results use the renderer's typed JSON values. Integers retain exact 32-bit
comparisons; floats compare numeric Float32 values because JSON loses signed zero. Boolean
and string comparisons have no numerical error metric. A numeric JSON `null` cannot identify
the non-finite value or its bits: it is marked `unmeasurable_value`, with no invented mismatch
count or maximum error. Type matches are recorded independently.

## Measured scope

Every native main-graph entry has a default/unconnected probe; every function operation has
nominal and unconnected probes. Not every nominal fixture is within every reference port's
accepted type: deliberately invalid scalar dot products, mixed comparisons, conversions and
integer vector operations test rejection/default behavior. These cases do not establish
support for those types. `function_dot_nominal` uses valid Float2 operands.

The additional cases cover scalar and vector arithmetic, 32-bit integer boundaries and values
above Float32's exact-integer range, casts, swizzles, comparison types, strings, Boolean logic,
division by zero, negative roots and logarithms, overflow, underflow and signed zero. Finite
original inputs generate the non-finite intermediates; literal non-finite SBS constants are
not validated.

Connected image cases use 16×16, 32×16 and 64×64, gray/color inputs, all four tiling flags,
parameter endpoints and every catalog blend mode. Processor cases include pixel position,
scope variables, nine numeric/Boolean output types in gray/color mode, scalar/vector random
values, seeds 0/1/7/−1, nearest/bilinear samplers at
interior, origin, unit and outside coordinates, and both Value Processor and Pixel Processor.
Precision cases cover U8, U16, F16 and F32, negative/HDR values and a 256×256 coordinate ramp.
Local fallback regressions additionally use 3×5; that odd size is not a reference measurement.

Execution probes cover lazy branches, sequences, shared variable reads with and without an
observable snapshot, constant and stateful loops with zero/positive/negative limits, original
reusable function calls, assignments crossing calls, Value Processor feeding Pixel Processor,
nested material processors, and FX-Map sampling a Pixel Processor. Map cases cover all four
node kinds, shapes 2–18, nested quadrants, depth/roughness, switch branches, iteration counts,
assignment state, scope variables, seeds and three additional precisions.

## Narrow corrections

The regression groups failed before their respective corrections. Existing tests that intended
mid-gray Uniform Color or neutral FX-Map weighting now set those fixture parameters explicitly.

| Correction | Measured behavior and regression |
|---|---|
| Fresh defaults | Uniform black/opaque; Blur 10; Distance combine true; Emboss angle zero and transparent black shadow; map roughness 1 and color mode; dynamic gradient position zero; grayscale matte 1; Normal flags false/true; Text size 0.25 and empty string; Float/Float4 constants zero; loop limit 32. `fresh_catalog_defaults_match_runtime_inspection` |
| Missing textures | Channels Shuffle and HSL transparent black; Emboss and missing dynamic gradient gray black, with measured image kinds. `absent_textures_match_measured_zero_pixels_and_kinds` |
| Unwired function operands | Zero values use operand types rather than result types, including casts, logic, samplers, Cartesian construction, swizzles and missing vector lanes. `function/unconnected.test.rs` |
| Grayscale image-input defaults | Write scalar native defaults, preserving the first channel of legacy color authoring values. `parity_input_defaults.rs` |
| Loop-limit serialization | Write the function's scalar integer tag (`constantValueInt1`); the enum-style tag caused a reference cooker error. `parity_function_loop.rs` |

The Normal default mapping was additionally measured with a vertical height ramp for each
Boolean setting; residual errors of up to 5.960464477539063e−8 remain, so this is not exact
normal-kernel parity. Defaults aligning does not imply kernel equivalence.

## Remaining differences and untested behavior

The per-case CSV is the authoritative numerical ledger. Remaining differences include filter
kernels and boundary handling, scalar/vector/type conversions and invalid-input behavior,
overflow policy, scope availability, random sequences outside the measured FX context, sampler
coordinates/addressing, precision/quantization, node output sizing, map shape rasterization,
roughness/depth weighting and assignment scope. These need independent minimal investigations;
a render mismatch alone does not identify which layer is responsible.

Concrete examples: `control_observable_snapshot_pixelfalse` returns 0 locally and 0.75 in the
reference (one value mismatch, maximum error 0.75); the corresponding Pixel Processor differs
at all 256 pixels. An immediately true constant loop returns its initial 0.25 locally and zero
in the reference. Stateful loop limits −1/0/1/3 match the recorded cases in both processor
contexts after the serialization fix. The F16 constant 0.1 differs by 2.4415552616119385e−5;
F32 constant probes match. Those results warrant separate execution/precision investigations,
without overriding the existing shared-node caching invariants from one graph alone.

In particular, the four measured FX scalar random seeds match exactly; this does not generalize
to Value Processor or Pixel Processor random streams. Relative-to-input tiling differs when
the graph has no image input. Sampler addressing is separate from node tiling in the reference;
the current model uses the evaluation context's tiling flag. Explicit sampler address modes
and their serialized settings remain untested. Integer readback above 2²⁴ can differ even
with a corrected native scalar constant tag; backend computation versus readback precision
has not been isolated, so changing the local integer representation is not justified.

Untested: all 954 loaded package function bodies; other runtime versions and rendering backends;
font/glyph equivalence; every curve/gradient knot or complex default; exhaustive enum/parameter
combinations; explicit address modes; literal NaN/Infinity input serialization; all resolutions
and aspect ratios beyond the listed fixtures; deeply nested processors/maps, nested loops,
execution-budget equivalence, malformed-package/error-message equivalence, dynamic resource
selection, and arbitrary SBS round trips. Local cycle, allocation and execution guards are
covered by repository safety tests, without a reference error-semantics parity claim.

The raw captures live in the caller's chosen output directory and are intentionally not bundled
as repository assets. Re-running the example reconstructs every tracked result case from
original source. Broad parity remains unestablished.

## Validation

Passed: `cargo fmt --all --check`; `cargo test -p materialcraft-nodes --release`;
`cargo test -p materialcraft-formats --release`;
`cargo clippy --workspace --all-targets -- -D warnings`; `cargo xtask layers` (14 crates);
and `cargo xtask wasm` (all ten packages). The example builds in release and completed all
1,615 recorded cases. Existing manifest-level unused dependency/package-field warnings remain.

The engine release suite passes with
`-- --skip byte_only_sbs_import_resolves_library_resource_substitutes`, including `panic_hunt`.
The unfiltered run fails that resource/font test: the normal resource root exposes a font-byte
expectation mismatch; an isolated root with an original generated EXR and no font also fails
its saved-path expectation at `package_resources.rs:32`. Neither resource resolution nor those
existing local assets were changed. This is an unresolved validation failure, so the full
required release run is not reported green. Repository-ignored tests otherwise remain ignored.

Two ignored UI snapshot cases were explicitly run in release and their images inspected:
`function_nodes_show_their_values_and_symbols` and
`fresh_embedded_graphs_open_only_after_explicit_editing`. They show the zero function constants
and the fresh processor/map editing workflow. Other UI snapshot cases were not rerun.
