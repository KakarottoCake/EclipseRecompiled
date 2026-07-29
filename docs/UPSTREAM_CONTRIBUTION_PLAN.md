# Upstream Contribution Plan

This document is the handoff plan for contributing the reusable parts of
Eclipse Recompiled back to
[GameCubeRecompiled](https://github.com/KaiserGranatapfel/GameCubeRecompiled).
It is intentionally written so a new work session can pick up without needing
the history of the original bring-up.

Last audited: July 29, 2026.

## The short version

Do not open one giant pull request from this fork's `main` branch.

`main` is the Eclipse-focused integration branch. It includes general fixes,
experimental runtime work, Eclipse-specific commands, documentation, and build
changes. Upstream will be much easier to review if each generally useful change
is rebuilt or cherry-picked onto its own branch created from `upstream/main`.

The recommended order is:

1. Fix generated Rust identifiers.
2. Add optional symbol-map support.
3. Correct generic controller discovery, routing, and mappings.
4. Discuss, then contribute direct GameCube adapter support.
5. Add safe loose-file DVD overrides.
6. Discuss an optional external/streamed asset archive mode.
7. Work through boot blockers as small, trace-backed runtime changes.

The Eclipse ISO preparation flow, Eclipse documentation, and any
Better Sunshine Engine or Kuribo-specific behavior stay in this fork.

## What upstream looks like right now

The upstream project is active, but it is still an experimental foundation
rather than a finished GameCube recompilation runtime.

At the time of this audit:

- Upstream's recent commits focused on small, testable boot-trace improvements:
  PowerPC control flow, special-purpose-register handling, memory-mapped
  hardware registers, and early framebuffer behavior.
- The roadmap still lists SDK stubs, graphics, DSP audio, input integration,
  and full game boot support as unfinished.
- The two open human-filed issues were
  [texture saving in a recompiled game](https://github.com/KaiserGranatapfel/GameCubeRecompiled/issues/18)
  and
  [Aurora integration](https://github.com/KaiserGranatapfel/GameCubeRecompiled/issues/20).
- The open pull-request queue consisted of dependency-update pull requests.
  Before changing a dependency, check that queue so a contribution does not
  duplicate or conflict with a pending update.
- Upstream's contribution guide asks for atomic commits, tests, formatting,
  documentation, clean-room work, and no proprietary Nintendo material.
  Contributions are dedicated under CC0-1.0.

This suggests that the maintainer is most likely to accept focused,
game-independent changes with a synthetic regression test and a clearly
explained failure mode.

## Repository and branch model

- Fork:
  [KakarottoCake/EclipseRecompiled](https://github.com/KakarottoCake/EclipseRecompiled)
- Upstream:
  [KaiserGranatapfel/GameCubeRecompiled](https://github.com/KaiserGranatapfel/GameCubeRecompiled)
- Fork integration branch: `main`
- Upstream contribution branches: `upstream/<short-topic>`

Never use the fork's `main` as the head of an upstream pull request. Start every
contribution branch at the current upstream tip:

```powershell
git fetch upstream
git switch -c upstream/codegen-identifiers upstream/main
```

After making only the scoped change:

```powershell
cargo fmt --all -- --check
cargo test -p gcrecomp-core
git diff --check
git diff upstream/main...HEAD
git push -u origin upstream/codegen-identifiers
```

Then open the pull request against `KaiserGranatapfel/GameCubeRecompiled:main`.
If upstream changes before review, update the branch from `upstream/main` and
rerun its checks.

## Proposed pull requests

### PR 1: Make generated Rust identifiers always valid

Status: smallest and closest to ready.

The current fork improves `sanitize_identifier` so an empty symbol becomes a
safe fallback and a symbol beginning with a number receives a prefix. This is a
general code-generation correctness fix, not an Eclipse feature.

Scope:

- `gcrecomp-core/src/recompiler/codegen/mod.rs`
- focused cases in `gcrecomp-core/tests/codegen_test.rs`

Before opening:

- Expand the test into explicit cases for empty input, leading digits,
  punctuation, and an already-valid name.
- Confirm there is no change to valid existing identifiers.
- Run the core tests and formatting checks.

Suggested branch: `upstream/codegen-identifiers`

### PR 2: Add optional symbol-map function discovery

Status: useful implementation exists; it needs stronger isolation and tests.

Symbol maps let recompilation reuse known function boundaries and names while
keeping automatically discovered functions for modified code. This is useful
for many decompilation projects, not only Super Mario Sunshine.

Scope:

- a documented, game-independent symbol-map parser
- optional `--symbol-map` arguments for analyze, recompile, and build
- pipeline APIs that accept an optional map
- parser and function-boundary regression tests using tiny synthetic data

Before opening:

- Remove Super Mario Sunshine-specific wording from the reusable API docs.
- Decide and document the accepted format and duplicate-address behavior.
- Test comments, whitespace, uppercase/lowercase hexadecimal prefixes,
  malformed addresses, out-of-image addresses, duplicates, and empty files.
- Keep ISO preparation and Eclipse defaults out of this PR.

Suggested branch: `upstream/symbol-map`

### PR 3: Correct controller discovery and generic mappings

Status: implementation exists, but it should be separated from direct USB
adapter support.

This PR should fix the current controller layer without adding a new hardware
dependency. The reusable changes include stable public controller IDs, correct
backend routing, avoiding duplicate devices, removing synthetic disconnected
XInput pads, sensible GameCube-position face buttons, analog trigger handling,
and radial dead-zone rescaling.

Scope:

- backend naming and routing
- stable IDs across polling
- SDL-first discovery with a fallback rather than duplicate enumeration
- generic layout and dead-zone tests
- no `rusb` dependency and no adapter backend yet

Before opening:

- Add mock-backend tests for connect, disconnect, reconnect, routing, and
  duplicate numeric IDs from different backends.
- Confirm mapping indices against the normalized layouts produced by each
  backend rather than physical platform-specific numbering.
- Explain why the XInput placeholders were false devices.

Suggested branch: `upstream/controller-routing`

### PR 4: Add native GameCube adapter support

Status: proof of concept exists; discuss the architecture upstream before
opening a code PR.

The fork has a direct `rusb` backend for Nintendo and compatible four-port
adapters, including rumble and an input-diagnostic example. It is highly
relevant, but it adds platform policy and dependency questions.

Ask the maintainer first:

- Is a direct USB backend wanted in the core runtime?
- Should it be behind a Cargo feature?
- Is SDL's adapter handling preferred when available?
- What Windows driver experience is acceptable? Direct `libusb` access may
  require WinUSB for the adapter interface.
- Should adapter rumble be exposed by the general controller-backend trait?

If accepted, contribute it separately from generic controller fixes and include:

- Nintendo and Mayflash-compatible vendor/product detection
- all four ports
- safe packet-length and controller-state parsing
- rumble lifecycle behavior
- mocked packet parser tests that require no attached hardware
- a concise platform setup note

Suggested branch: `upstream/gamecube-adapter`

### PR 5: Add safe loose-file DVD overrides

Status: implementation exists; extract it from the broader archive changes.

A loose-file override directory is a useful generic modding primitive. The
runtime can check that directory before reading the original disc archive,
allowing a mod to replace individual files without rebuilding an image.

Scope:

- optional override root
- normalized relative GameCube paths
- directory-traversal rejection
- original archive fallback
- focused tests for override success, fallback, and unsafe paths

Do not bundle external archive streaming into this PR.

Suggested branch: `upstream/dvd-overrides`

### RFC, then PR 6: Make external streamed assets an option

Status: valuable for large games, but it changes an upstream design choice.

The fork avoids embedding a multi-gigabyte disc archive in the executable and
can read archive ranges from a file. Upstream currently favors embedded assets.
Propose an optional mode rather than replacing the existing mode.

The design discussion should cover:

- embedded, external, and possibly memory-mapped archive modes
- how the executable locates its archive
- distribution and portability tradeoffs
- archive integrity/version checks
- random-access performance and error reporting
- keeping proprietary files out of source control and release artifacts

Only implement after the maintainer agrees on the public API.

Suggested discussion title:
`RFC: optional file-backed asset archives for large recompilation projects`

Suggested branch after agreement: `upstream/file-backed-assets`

### Optional infrastructure PRs

The fork also contains Windows CI/toolchain and line-ending fixes. Offer these
only when they solve a reproducible upstream failure. Keep dependency changes
out of feature PRs, especially where an open dependency-update PR already
exists.

Possible small PRs:

- pin a CMake version compatible with the SDL build in upstream CI
- preserve LF for source files used by cross-platform checks
- add a manual workflow trigger if upstream wants it

## Work that remains in the fork

These pieces serve the Eclipse project and should not be proposed upstream as
they are:

- the `prepare` command and scripts tailored to a legally dumped Eclipse disc
- Better Sunshine Engine or Super Mario Sunshine symbol-map defaults
- Eclipse build, play, troubleshooting, and progress documentation
- Eclipse-specific mod directories or examples
- Kuribo-specific loading behavior
- project branding and the fork's layperson-focused README

Some underlying primitives may become upstream contributions, but their
upstream versions must be game-independent.

## Runtime contribution roadmap

Landing the utility PRs will not make Eclipse playable. The larger job is to
complete the runtime in vertical slices, using the first observable boot
divergence to select each slice.

### Phase A: low-risk reusable foundations

Land identifier sanitation, symbol maps, controller routing, and safe DVD
overrides. These improve development velocity without claiming game boot.

### Phase B: connect host input and disc behavior to emulated SDK calls

The most relevant first runtime slice is PAD/SI high-level emulation:

- initialize controller state
- return correct `PADStatus` data and error states
- implement recalibration/origin behavior as needed
- connect motor commands to backend rumble
- test memory layouts and button/axis conversion without game data

Then complete the DVD behavior reached during boot, including asynchronous
completion, callbacks, cancellation, status, and interaction with the emulated
scheduler.

### Phase C: follow boot traces through OS, VI, and GX

For each blocker:

1. Capture the first divergence from expected execution.
2. Reduce it to a PowerPC instruction, hardware register, or SDK behavior.
3. Write a synthetic regression test.
4. Implement only that behavior.
5. Open a focused upstream PR.

Likely areas are OS threads, scheduling, interrupts, timers, cache semantics,
VI timing, EFB/XFB copies, and the GX command path. Avoid a speculative
"implement all of GX" pull request.

### Phase D: audio and mod-loader integration

DSP/audio should follow once boot and rendering are stable enough to expose the
required behavior. Kuribo support begins in the Eclipse fork. If it reveals a
clean, game-independent runtime hook, propose that hook upstream separately
without Kuribo or Eclipse assets.

## Questions to settle with the maintainer

Open one concise discussion before the larger input and asset changes:

1. Which runtime area is currently the highest priority: PAD/SI, DVD, OS, or GX?
2. Is direct `rusb` GameCube adapter support wanted, and should it be optional?
3. Is a simple `name=0xADDRESS` map acceptable, or is another symbol format
   preferred?
4. Would upstream accept optional file-backed archives while retaining embedded
   archives as the default?
5. Does the maintainer prefer SDK high-level emulation, hardware-level SI/DVD
   emulation, or a documented hybrid for the first playable targets?

Link to the relevant existing issue when possible. Do not use an unrelated
issue merely for visibility.

## Pull-request quality gate

Every upstream pull request should meet all of these before it is opened:

- Branch created from the latest `upstream/main`, never this fork's `main`.
- One reviewable concern with no Eclipse-only files.
- No ISO, DOL, extracted asset, Nintendo SDK code, or proprietary symbol data.
- A synthetic unit or regression test for the behavior.
- `cargo fmt --all -- --check` passes.
- Relevant package tests pass.
- `cargo clippy` passes for the touched package, or any existing unrelated
  warning is clearly identified.
- `git diff --check` passes.
- Documentation describes behavior and limitations without overstating
  playability.
- The PR explains the bug or need, the chosen behavior, and exactly how it was
  tested.
- Dependency updates are isolated and checked against existing bot PRs.

## First task for the next session

Start with PR 1 only:

1. Read this file and upstream's current `CONTRIBUTING.md`.
2. Fetch `upstream` and confirm whether `upstream/main` moved.
3. Create `upstream/codegen-identifiers` from `upstream/main`.
4. Reapply only the identifier sanitation and its expanded regression tests.
5. Run formatting, core tests, clippy, and `git diff --check`.
6. Review the complete diff against `upstream/main`.
7. Push the branch and draft a small upstream pull request.

Do not begin PR 2 until PR 1 is ready for review. This first contribution is
deliberately small so it establishes a clean working relationship and confirms
upstream's review preferences before the more architectural changes.

