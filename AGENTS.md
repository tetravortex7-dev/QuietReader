1. Project Goal

QuietReader is a modular, cross-platform ebook reader for Windows, Linux, and macOS. Architecture must support multiple document formats, decoupled UI, maintainability, and clean extensibility without unnecessary refactoring. All development must strictly follow the rules in this document.

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

- Windows, Linux, and macOS are equal first-class targets.
- Zero platform-specific assumptions or hardcoded paths in shared code.
- Audit dependencies for maintenance, licensing, binary size, and cross-platform compatibility.
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
- Reporting: State exact changes and completed checks. Explicitly mention unverified platforms, missing checks, and unresolved issues.
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
