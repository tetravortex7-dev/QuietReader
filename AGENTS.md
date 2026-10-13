1. Project Goal

QuietReader is a modular, cross-platform ebook reader for Windows, Linux, and macOS.

The architecture must support multiple document formats, independent UI development, maintainability, and future expansion without unnecessary refactoring.

All development must follow the principles and rules defined in this document.

2. Core Principles

SOLID

SOLID principles must guide architectural decisions:

SRP: Each module has one clear responsibility.

OCP: New formats and features must require minimal changes to existing code.

LSP: Implementations of the same interface must honor its contract.

ISP: Modules must not depend on capabilities they do not need.

DIP: High-level logic must depend on abstractions, not concrete implementations.

Do not introduce unnecessary abstractions merely to satisfy SOLID formally.

General Rules

Prefer modularity, clear boundaries, and low coupling.

Reuse existing functionality instead of duplicating it.

Preserve existing behavior unless a change is explicitly required.

Keep changes focused on the requested task.

Avoid monolithic modules and unrelated responsibilities.

3. Modular Architecture

The project must maintain clear separation between four areas.

3.1. Application Core

Contains application state, navigation, reading progress, bookmarks, history, search coordination, and other reader functionality.

Must not depend on a specific UI framework.

Must not contain format-specific parsing or platform-specific implementation details.

Coordinates document backends and application services through well-defined interfaces.

Exposes document operations and supported capabilities through stable abstractions. The UI must not need to know which concrete backend implements them.

3.2. Document Backends

Each document format must be handled by a backend exposed through an appropriate interface. Related formats may share an implementation when this keeps the architecture clear and maintainable.

Supported and planned formats:

Flowable text: EPUB, TXT, MD, LOG, FB2.

Fixed-layout documents: PDF, DJVU.

Comics and manga: CBZ, CBR, CB7, and image folders.

Rules:

The listed formats define project targets, not proof of existing support. Inspect the current implementation before assuming a format or capability is available. Do not claim support unless it is implemented and appropriately verified.

Adding a format must not require rewriting existing backends or format-specific UI components. Updating a central registry is acceptable.

Format-specific parsing, metadata extraction, and content access belong in document backends.

Use shared interfaces for common capabilities and separate interfaces for format-specific capabilities.

Optional features, such as table of contents, text search, and cover extraction, must not be mandatory for every format.

Related formats may share implementations and dependencies where appropriate.

Keep format-specific dependencies isolated wherever practical.

3.3. Presentation and UI

Contains the library, reader interface, navigation panels, toolbars, themes, settings screens, fullscreen behavior, and viewing modes.

Must not parse document files or implement format-specific decoding.

Must communicate with the application core through defined interfaces.

Must not depend directly on concrete document backends.

Keep visual styling and UI-specific resources separate from document processing and business logic.

Format-specific viewing modes may control presentation but must not implement document decoding.

3.4. Platform Services

Contains operating-system-specific functionality, including file associations, configuration paths, system integration, and screen-sleep prevention.

Hide platform-specific implementation details behind common interfaces.

Keep Windows, Linux, and macOS implementations isolated.

Use conditional compilation where appropriate.

Platform-specific code must not leak into document backends or general UI logic.

4. Document and Rendering Contracts

Common document operations must have consistent behavior and error handling.

Document types must expose only the capabilities they support.

Separate document decoding, content preparation, layout, and UI presentation into clear responsibilities.

Document backends must not depend on GUI frameworks or GUI-specific resources.

Expensive operations must not unnecessarily block the UI.

Handle missing, corrupted, and unsupported content gracefully.

5. Cross-Platform Requirements

Windows, Linux, and macOS are first-class target platforms.

Do not introduce platform-specific assumptions into shared application logic.

Avoid hardcoded paths and dependencies that unnecessarily restrict platform support.

Evaluate dependency maintenance, licensing, binary size, and cross-platform compatibility before adoption.

Isolate and document necessary platform-specific exceptions.

6. Reliability and Data Safety

Never silently discard user settings, bookmarks, reading progress, or history during routine changes.

Preserve existing data through appropriate migrations when persistent data formats change.

Failed migrations must not silently destroy user data; preserve recoverability whenever persistent data is modified.

Handle file and operating-system errors without uncontrolled application crashes.

Avoid loading entire large documents into memory when incremental processing or caching is appropriate.

Validate document and archive contents and apply reasonable resource limits.

Archive processing must prevent path traversal and account for decompression bombs, excessive entry counts, and unreasonable resource consumption.

Do not use panics as normal error handling for document parsing or I/O.

7. Development Rules for AI Agents

Before Changes

Inspect the existing project structure and relevant implementations.

Follow established architecture and reuse existing interfaces where appropriate.

Prefer the smallest correct change.

Do not perform unrelated refactoring or introduce unnecessary dependencies.

Do not weaken module boundaries for short-term convenience.

After Changes

Run appropriate formatting, compilation, and tests when available.

Add or update tests for new functionality and important bug fixes.

Report what changed and which checks were actually completed.

Explicitly mention unresolved issues and unverified platform compatibility.

If a relevant check cannot be run, state why and identify what remains unverified.

Make broader architectural changes only when necessary to satisfy requirements, fix a fundamental design issue, or prevent recurring problems. Explain the justification and keep the scope controlled. Do not undertake a large-scale migration without a clear need.

8. Architecture Decision Rule

Maintainability, modularity, cross-platform compatibility, and correctness take priority over premature optimization and unnecessary abstraction.

New formats and features must fit the architecture without coupling document processing, application logic, UI design, and operating-system integration.

9. Binary Size Optimization

Use release builds for production.

Apply stripping and compiler optimizations when supported and beneficial.

Evaluate binary size, build time, and runtime performance before adopting additional profile settings.

Avoid unnecessary dependencies and disable unused default features (default-features = false) where appropriate.

Do not use panic = "abort" solely for size reduction without considering its consequences.

Verify that optimizations preserve functionality and cross-platform compatibility.

Always use the verified release profile settings `opt-level = "s"` and `lto = "thin"`, as this configuration reduces binary size by ~10% (1.56 MB saved) and builds twice as fast compared to `opt-level = 3` with `lto = "fat"`. After building a release, copy the final binary to the project root (`QuietReader.exe`).

10. Workspace Hygiene and Cleanup

If the project folder size exceeds 3 GB (`> 3 GB`), clean up the workspace (e.g., `target/` build artifacts and temporary files), keeping only the source files, project configuration, and the root `QuietReader.exe` binary, and deleting everything unnecessary.

File line count checks must enforce a limit of 1000 lines per file (not 700). If a file exceeds 1000 lines, refactor it, remove redundant comments and duplicate code, and decompose it into separate modules whenever possible.

11. Immutability of AGENTS.md

`AGENTS.md` is strictly inviolable. Editing or modifying `AGENTS.md` is forbidden without direct, explicit permission given personally by the project owner (user).

12. Code Comments Policy

Do not write or leave comments in the code. Writing comments in code is strictly prohibited. If comments are present in the code, they must be removed.