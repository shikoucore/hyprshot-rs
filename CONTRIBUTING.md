# Contributing Guide

Thanks for your interest in contributing to hyprshot-rs. This document explains
how we work and what we expect for contributions to be accepted.

## Project Scope
- hyprshot-rs is a Wayland screenshot tool.
- It targets Wayland compositors (Hyprland, Sway; others best‑effort).
- We prioritize small, predictable changes and low runtime overhead.

## Development Requirements
- Wayland compositor (Hyprland or Sway recommended).
- Tools: `cargo`, `rustfmt`, `clippy`.
- Runtime deps for testing: `wl-clipboard`, `slurp` (system or embedded path).

## Quick Start
- Fork the repo and create a feature branch.
- Make changes with clear commit messages.
- Add or update tests if behavior changes.
- Open a pull request.

## Non‑Negotiable Rules
- Any new user‑visible behavior must be documented (`README.md`, `doc/CLI.md`, `doc/CONFIGURATION.md`).
- Any bug fix must link to an Issue (or create one first).
- Every PR must reference the related Issue.
- New functionality should be discussed in an Issue before implementation.

## Issue‑First Policy
Before starting work:
- Open an Issue for new features.
- Open an Issue for bugs with reproduction steps and environment.
- Link the Issue in your PR description.

## Testing Policy
- Keep tests deterministic.
- Avoid environment‑specific assumptions.
- Run `cargo test` locally before opening a PR.

## Documentation & Changelog
- Update `README.md` for user‑facing changes.
- Update `CHANGELOG.md` for notable changes.
  Example:
  ```
  ### Fixed
  - **Short summary**: Clear description of what changed and why.
  ```

## Code Style & Quality
- Follow the existing style and structure.
- Avoid new `unsafe` unless absolutely necessary (explain why).
- Prefer explicit error handling over `unwrap()`/`expect()` in production code.
- Keep functions focused and readable.

## Pull Request Checklist
- [ ] Linked Issue in PR description
- [ ] Tests added/updated (if needed)
- [ ] `cargo test` passes locally
- [ ] Docs and changelog updated (if needed)

## Communication
- Keep PRs focused and small when possible.
- Be explicit about tradeoffs and alternatives.
- If unsure, open an Issue and ask first.
