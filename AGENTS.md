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
- Contains application state, navigation, progress, bookmarks, history, and search coordination.
- Framework-agnostic, format-agnostic, and platform-agnostic.
- Coordinates backends and services via stable abstractions; UI must not depend on concrete backends.

3.2. Document Backends
- Handles parsing, metadata extraction, and content access behind format-agnostic interfaces.
- Backends must not depend on GUI frameworks or GUI resources.
- Supported and planned targets:
  - Flowable text: EPUB, TXT, MD, LOG, FB2.
  - Fixed-layout: PDF, DJVU.
  - Comics and manga: CBZ, CBR, CB7, image folders.
- Format lists define targets, not proof of support. Verify actual implementation before assuming availability.
- Adding formats requires updating the central registry only—no backend or UI rewrites.
- Optional features (TOC, search, covers) must not be mandatory for all formats.
- Shared dependencies and implementations allowed where logical; keep format dependencies isolated.

3.3. Presentation and UI
- Library, reader, toolbars, sidebars, themes, settings, fullscreen, and viewing modes.
- No parsing, decoding, or direct backend dependencies; communicates only with Core interfaces.
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

- Use release builds for production with stripping and optimizations enabled.
- Minimize dependencies; disable unused defaults (`default-features = false`). Do not use `panic = "abort"` purely for size.
- Mandatory release settings: `opt-level = "s"` and `lto = "thin"` (~10% / 1.56 MB size saving, 2x faster build vs `opt-level = 3` + `lto = "fat"`).
- After release builds, copy final binary to project root: `QuietReader.exe`.

10. Workspace Hygiene and Cleanup

- Disk limit: If workspace size exceeds 3 GB, clean build artifacts (e.g., `target/`) and temporary files, preserving only source files, config, and root `QuietReader.exe`.
- File line limit: Enforce a 1000-line limit per file (not 700). Decompose files exceeding 1000 lines into separate modules, removing duplicate code and redundant comments.
- Localization exception: the 1000-line limit does not apply to localization files (`src/i18n/*.rs`). A single language file may freely exceed 1000 lines.

11. Immutability of AGENTS.md

`AGENTS.md` is strictly inviolable. Any modification is forbidden without direct, explicit permission from the project owner.

12. Code Comments Policy

Writing or leaving comments in code is strictly prohibited. If comments exist in code, remove them.

13. Vector Icons Policy

All UI icons must be custom vector paths drawn with `egui::Painter` (`src/app/icons.rs`). Unicode symbols, text arrows, and emojis are forbidden to prevent font-fallback rendering artifacts ("tofu").

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

17. Localization Policy

- Any text shown to the user — labels, buttons, tooltips, errors, statuses, dialogs, and notifications — must be localized in every language supported by the project.
- Hardcoding user-visible text in a single language is forbidden. Add a key to `Translations` in `src/i18n/mod.rs` and provide translations in all locale files (`ru`, `en`, `fr`, `es`, `zh`, `ja`, `pt`, `it`, `tr`, `uk`, `ar`, `id`).
- A feature that adds or changes user-visible text must include all translations in the same change.
- Localization files (`src/i18n/*.rs`) are exempt from the 1000-line limit; a single language file may freely exceed 1000 lines.

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
- Save screenshots to `%TEMP%\opencode\*.png` and inspect them with the image reader.
- Do not conclude that a UI element is missing, mispositioned, or unrendered until a full-window screenshot confirms it; clipped captures have repeatedly produced false conclusions.
- For synthetic input: keep the window foreground, move the cursor with `SetCursorPos`, send events with `mouse_event`, and pass the wheel delta as a signed `int` (for example `-120`), not `uint32`. Wait 100–200 ms between events and 1–2 seconds before capturing.

20. Testing Cadence

- Do not run the full test suite after every small edit. During implementation use targeted checks only: `cargo check`, a single test by name (`cargo test --features enable_tests <module>::test::<name>`), or `cargo fmt --all`.
- Run the full `cargo test --features enable_tests --all-targets` and `cargo fmt --all -- --check` only before finishing a task: before a release build, before replacing `QuietReader.exe`, before a commit, or when reporting completed work.
- If a change touches shared layout, persistence, networking, or archive parsing, run the full suite once for that change even if it is not the final step.

21. File Association Prompt During Development

- Always disable the file-association prompt while developing, debugging, taking screenshots, or testing the UI: set `"dismiss_association_prompt": true` in `%APPDATA%\QuietReader\settings.json` before launching the app. The modal dialog blocks the interface and breaks synthetic input and screenshots.
- Stop the running app before editing the settings file, write it as UTF-8 without BOM, and verify the value before launching; a running instance can overwrite the file with its in-memory settings.
- Enable the prompt (`"dismiss_association_prompt": false`) only for a production release: restore it before the final release build and before replacing the root `QuietReader.exe`, so first-run users see the association dialog.

