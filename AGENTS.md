1. Project Goal

QuietReader is a modular ebook reader for Windows, with Android planned next and Linux and macOS planned last. Architecture must support multiple document formats, decoupled UI, maintainability, and clean extensibility without unnecessary refactoring. All development must strictly follow the rules in this document.

2. Core Principles

- SOLID:
  - SRP: One clear responsibility per module.
  - OCP: Extensible for new formats and features with minimal changes to existing code.
  - LSP: Implementations must strictly honor interface contracts.
  - ISP: Modules depend only on capabilities they actually use.
  - DIP: High-level logic depends on abstractions, not concrete implementations.
  - Avoid speculative abstractions introduced solely for formal compliance.
- General:
  - High cohesion, low coupling, clear boundaries.
  - Reuse existing functionality; preserve existing behavior unless change is explicitly requested.
  - Focus strictly on requested tasks; avoid monolithic modules and unrelated refactoring.

3. Modular Architecture

Strict separation between four layers:

3.1. Application Core
- Contains application state, navigation, progress, bookmarks, history, and search coordination, plus shared core services (library, OPDS/catalog sync, dictionary, audio/TTS coordination).
- Framework-agnostic, format-agnostic, and platform-agnostic.
- Coordinates backends and services via stable abstractions; Core modules (`core`, `opds`, `dictionary`, `settings`) must never import Presentation (`app`) types, and UI must not depend on concrete backends.

3.2. Document Backends
- Handles parsing, metadata extraction, and content access behind format-agnostic interfaces (`Document` trait).
- Backends must not depend on GUI frameworks, GUI resources, or hardcoded single-language user-facing error strings (return typed/localizable errors mapped by localization).
- Implemented and planned formats:
  - Flowable and structured text: EPUB, FB2, FB2.ZIP, MOBI, AZW, AZW3, PRC, TXT, MD, LOG, HTML, HTM, XHTML, DOCX, RTF, DOC, DOT.
  - Fixed-layout: PDF, DJVU, DJV.
  - Comics and manga: CBZ, CBR, CB7, CBT, ZIP, RAR, 7Z, TAR, image folders.
- Format lists define project scope, not automatic parity of optional features. Verify actual implementation before assuming availability.
- Adding a format should go through the central registry (`src/document/registry.rs`) plus shared extension, file-association/MIME, and localization format tables without rewriting UI or unrelated backends; consolidating extension metadata into a single source of truth is the architectural goal.
- Optional features (TOC, search, covers, copy permissions) must not be mandatory for all formats.
- Shared dependencies and helpers allowed where logical; keep format dependencies isolated.

3.3. Presentation and UI
- Library, reader, toolbars, sidebars, themes, settings, fullscreen, and viewing modes.
- No parsing, decoding, or direct dependency on concrete document backends; communicates only with Core and `Document` / `LocalizedDocument` interfaces.
- Visual styling remains separate from document processing. Viewing modes control presentation, not decoding.

3.4. Platform Services
- OS-specific features (file associations, config paths, system integration, screen-sleep prevention).
- Hide behind common interfaces using conditional compilation.
- Isolate Windows, Linux, and macOS implementations; never leak OS logic into Core, backends, or UI.

4. Document and Rendering Contracts

- Uniform behavior and consistent error handling across document types.
- Document types expose only capabilities they support.
- Decouple decoding, content preparation, layout, and UI presentation.
- Heavy operations must never block the UI.
- Gracefully handle missing, corrupted, or unsupported content without crashing or panicking.

5. Cross-Platform Requirements

- Platform phases are fixed: Windows is the current and only release target; Android is the next target; Linux and macOS are planned last.
- Do not report missing Linux, macOS, or Android builds as a current defect, and do not add platform work outside the active phase without an explicit request.
- Zero platform-specific assumptions or hardcoded paths in shared code.
- Audit dependencies for maintenance, binary size, and compatibility with the active platform phase.
- Isolate and document necessary OS-specific exceptions.

6. Reliability and Data Safety

- Never silently discard user settings, bookmarks, progress, or history.
- Version persistent data; migrations must preserve recoverability on failure.
- Handle file and OS errors gracefully without uncontrolled crashes or panics.
- Avoid full-document in-memory loading; use streaming, incremental reading, or caching.
- Enforce archive safety: prevent path traversal, decompression bombs, and excessive memory/file consumption.

7. Development Rules for AI Agents

- Before changes: Inspect codebase, reuse existing abstractions, make minimal surgical changes. Avoid unrelated refactoring or weakened boundaries.
- After changes: Run formatting, compilation, and tests. Add/update tests for fixes and features. Document new features in `README.md`.
- Reporting: State exact changes and completed checks for the active platform phase. Mention missing checks and unresolved issues that affect Windows or the current task; do not list inactive platforms as defects.
- Broader architectural changes require explicit justification and controlled scope.

8. Architecture Decision Rule

Maintainability, modularity, cross-platform compatibility, and correctness take priority over premature optimization and over-engineering. New features must fit the architecture without cross-layer coupling.

9. Binary Size Optimization

- Use release builds for production with stripping (`strip = true`) and optimizations enabled.
- Minimize dependencies; disable unused defaults (`default-features = false`) and keep non-Windows windowing features (such as `eframe`'s `x11` and `wayland`) gated out of Windows builds where possible. Do not use `panic = "abort"` purely for size.
- Mandatory release settings in `Cargo.toml`: `opt-level = "s"` and `lto = "thin"` (~10% / 1.56 MB size saving, 2x faster build vs `opt-level = 3` + `lto = "fat"`). Future hardening notes (such as `rust_egui.md`) do not override this rule unless explicitly requested.
- After release builds, copy the release binary (`target\release\QuietReader.exe`, never a debug build) to project root: `QuietReader.exe`.

10. Workspace Hygiene and Cleanup

- Disk limit: If workspace size exceeds 3 GB, clean build artifacts (e.g., `cargo clean` or clearing `target/`) and temporary files, preserving source files, config, documentation, and the release root `QuietReader.exe`.
- File line limit: Enforce a 1000-line limit per file (not 700). Decompose files exceeding 1000 lines into separate modules, removing duplicate code and any comments (per Rule 12).
- Localization exception: the 1000-line limit does not apply to localization files (`src/i18n/*.rs`). A single language file may freely exceed 1000 lines.

11. Immutability of AGENTS.md

`AGENTS.md` is strictly inviolable. Any modification is forbidden without direct, explicit permission from the project owner.

12. Code Comments Policy

Writing or leaving comments in code is strictly prohibited. If comments exist in code, remove them.

13. Vector Icons Policy

All UI icons must be custom vector paths drawn with `egui::Painter` (`src/app/icons.rs`). Unicode symbols, text arrows (e.g., `→`, `←`), decorative UI bullets (`•`) used in chrome/labels, and emojis are forbidden to prevent font-fallback rendering artifacts ("tofu").

14. Git Synchronization Policy

Do not commit and push after every modification or task. Synchronize approximately once every 2 days: check git commit history and push only if at least 2 days have elapsed since the last commit/push.

15. Feature Documentation Policy

Every new feature or capability introduced into the project must be documented in `README.md`. Future agents, assistants, and contributors rely on `README.md` as the authoritative overview of available features and user-facing capabilities.

16. Test Placement Policy

- Tests must never live inline in production source files (no `mod tests { ... }` blocks) and never as loose `tests_*.rs` files in `src/`.
- Every module keeps its tests in a `test` subdirectory next to it and declares them with `#[cfg(all(test, feature = "enable_tests"))] mod test;`. Examples: `src/document/epub/test/` holds EPUB-only tests, `src/document/fb2/test/` FB2-only tests, `src/document/mobi/test/` MOBI-only tests, `src/app/test/` UI/app tests, `src/settings/test/` settings tests, `src/opds/test/` OPDS tests.
- The same rule applies to every new format and module: format-specific tests go into `<format module>/test/`; tests spanning several formats (archive safety, registry) go into `src/document/test/`.
- Split a `test` directory into several topic files (declared in its `mod.rs`) when it grows; the 1000-line limit and the no-comments rule apply to test files too.
- Helpers shared by tests of different modules live in `src/test/util.rs` (e.g. `unique_test_dir`); do not duplicate them.
- New tests must be added to the matching `test` directory in the same change as the feature or fix they cover.
- Unit and integration tests must be deterministic and offline-safe: never depend on live external servers to pass, and never silently return success when a network or service call fails.

17. Localization Policy

- Any text shown to the user — labels, buttons, tooltips, errors, statuses, dialogs, generated fallback titles/chapter names, and notifications — must be localized in every language supported by the project.
- Hardcoding user-visible text in a single language is forbidden (including inside document backends or background workers). Add keys to `Translations` / `ExtraTranslations` in `src/i18n/mod.rs` and provide complete native translations (no unrequested English fallbacks) in all locale files (`ru`, `en`, `fr`, `es`, `zh`, `ja`, `pt`, `it`, `tr`, `uk`, `ar`, `id`).
- A feature or bugfix that adds or changes user-visible text must include all 12 translations in the same change (see Rule 10 for the localization file line-limit exception).

18. Dependency Version Policy

- Use the latest stable release of every dependency by default.
- Using an older version is an exception that must be justified by a concrete blocker: an incompatible MSRV or toolchain, a confirmed regression or security issue, an unavailable platform target, or a transitive constraint that cannot be resolved.
- Whenever an older version is used, provide a complete and strict report before or with the change: the selected version, the latest available version, the exact reason the latest version cannot be used, affected platforms and features, security and maintenance impact, alternatives considered, and the condition under which the dependency must be upgraded.
- Do not silently pin, downgrade, or retain an outdated dependency.

19. UI Screenshot Verification

- Always capture the whole application window, never a fixed screen area. Maximize the window, bring it to the top (`ShowWindow` with `SW_MAXIMIZE`, `SetWindowPos` with topmost flags, `BringWindowToTop`, `SetForegroundWindow`) and wait 1–2 seconds before capturing.
- Get the real window bounds with `GetWindowRect` and size the bitmap with a margin: width `max(2000, right + 40)`, height `max(1200, bottom + 40)`. Never capture a fixed `1536x864`: DPI virtualization reports smaller screen metrics than the actual window, so the right edge (top-bar buttons, page counter, scrollbar) and the bottom edge get cut off.
- Capture from the top-left corner with `Graphics.CopyFromScreen(0, 0, 0, 0, bitmap.Size)`; use `PrintWindow` only as a fallback.
- After capturing, verify that the right and bottom window edges are present in the image (e.g. the fullscreen button, page counter, bottom status bar). If any edge is missing, enlarge the capture and repeat.
- Save screenshots to `%TEMP%\quietreader_screenshots\*.png` and inspect them with the image reader.
- Do not conclude that a UI element is missing, mispositioned, or unrendered until a full-window screenshot confirms it; clipped captures have repeatedly produced false conclusions.
- For synthetic input: keep the window foreground, move the cursor with `SetCursorPos`, send events with `mouse_event`, and pass the wheel delta as a signed `int` (for example `-120`), not `uint32`. Wait 100–200 ms between events and 1–2 seconds before capturing.

20. Testing Cadence

- Do not run the full test suite after every small edit. During implementation use targeted checks only: `cargo check --features enable_tests --all-targets`, a single test by name (`cargo test --features enable_tests --all-targets <module>::test::<name>`), or `cargo fmt --all`. Note: `Cargo.toml` sets `test = false` on the binary target, so `--all-targets` is required for `cargo test` to build and run unit tests.
- Run the full `cargo test --features enable_tests --all-targets` and `cargo fmt --all -- --check` only before finishing a task: before a release build, before replacing `QuietReader.exe`, before a commit, or when reporting completed work.
- If a change touches shared layout, persistence, networking, or archive parsing, run the full suite once for that change even if it is not the final step.

21. File Association Prompt During Development

- Always disable the file-association prompt while developing, debugging, taking screenshots, or testing the UI: set `"dismiss_association_prompt": true` in `%APPDATA%\QuietReader\settings.json` before launching the app. The modal dialog blocks the interface and breaks synthetic input and screenshots.
- Stop the running app before editing the settings file, write it as UTF-8 without BOM, and verify the value before launching; a running instance can overwrite the file with its in-memory settings.
- Enable the prompt (`"dismiss_association_prompt": false`) only for a production release: restore it before the final release build and before replacing the root `QuietReader.exe`, so first-run users see the association dialog.

22. User-Agent Policy

- Network requests that send a `User-Agent` header must always reflect the current application version dynamically using `env!("CARGO_PKG_VERSION")`, `crate::net::APP_USER_AGENT`, or `crate::net::user_agent(...)`.
- Hardcoding static, legacy, or outdated version strings in `User-Agent` headers is strictly prohibited. When adding or modifying network integrations, ensure the user agent format follows `QuietReader/<version>` or `QuietReader/<version> (<component>)`.

23. Multi-Monitor Responsive Design Policy

- Every UI view, layout, and component must be fully responsive and adapt seamlessly across all monitor types, screen resolutions, and aspect ratios:
  - Standard displays and laptops (16:9, 16:10, from 1366x768 up to 1080p): no horizontal clipping, awkward wrapping, or truncated controls.
  - Ultrawide and large displays (21:9, 32:9, 1440p, 4K+): layouts must maintain balanced proportions, using adaptive column grids, responsive card metrics, or max-width containers to prevent uncontrolled stretching.
  - Compact and low-height viewports (un-maximized windows, small screens): multi-column headers and dashboard widgets must gracefully fold or stack at responsive breakpoints; headers must not push primary document/shelf content beyond the viewport fold; vertical scrolling must always remain functional and smooth.
  - High-DPI and fractional OS scaling (100%, 125%, 150%, 175%, 200%): elements must honor OS DPI metrics, preventing blurry textures, clipped text galleys, or pixel-overlap artifacts.
  - Hardcoded, non-responsive fixed widths that cause overflow on smaller screens or severe disproportion on larger screens are strictly prohibited.

