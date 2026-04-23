# Agent Instructions

This repository is a Rust port of the original QBasic `GORILLAS.BAS`. The goal is maximum practical fidelity to the original game while using modern Rust, `winit`, and `wgpu` for the platform layer.

The job is not to make a new artillery game with the same premise. It is to recreate QBasic Gorillas as closely as possible: gameplay, pacing, shapes, colors, layout, text flow, and overall look and feel should resemble the original unless a modern framework constraint makes an exact match impractical.

## Primary References

- Use `GOAL.md` as the main project guideline. It annotates the original BASIC structure with the intended Rust architecture and translation notes.
- Use `GORILLAS.BAS` as the original source of behavior when implementing rules, rendering details, input flow, scoring, animation timing, and edge cases. `GOAL.md` is annotated, but the BASIC file is still the authority when a behavior is unclear.
- Use `reference.png` as the visual reference for the original game. Match the composition and visual vocabulary: 640-ish logical canvas, blue background, central sun, colorful skyline, yellow windows, two red gorillas, chunky primitive shapes, and flat EGA-style colors.

## Porting Priorities

1. Preserve original gameplay semantics and presentation where practical: turn flow, angle and velocity entry, wind, gravity, banana trajectory, collision, explosions, scoring, play-again loop, victory behavior, colors, proportions, and screen layout.
2. When choosing between a more polished modern design and a closer original match, choose the closer original match.
3. Prefer idiomatic Rust architecture over literal BASIC structure. BASIC globals, labels, `GOSUB`, and screen calls should become explicit structs, enums, state transitions, and rendering functions.
4. Keep the existing stack unless there is a strong reason to change it: `winit` for the app/event loop and `wgpu` for rendering.
5. Treat current deviations from the original as implementation gaps unless they are clearly intentional and documented.
6. Favor deterministic or testable game logic. Random city generation can use seeded RNGs for tests and unseeded/session RNGs for gameplay when appropriate.

## Visual Direction

- Keep the original game's low-resolution, flat-color, EGA-like look. Shapes, colors, proportions, and placement should resemble `reference.png` and the BASIC drawing routines.
- Do not replace the original look with modern gradients, antialiased illustration, lighting, textures, particle-heavy effects, smooth vector redesigns, or a different art style.
- Use logical coordinates based on the original screen dimensions, currently `640 x 350` in code. Preserve aspect and scale cleanly to the window.
- Prefer original palette values and BASIC primitive geometry when available. If exact EGA color behavior is awkward through the renderer, choose the nearest visual match and document the deviation.
- Compare visual changes against `reference.png` before calling them done.
- Procedural primitives are acceptable, but they should be shaped to match the original gorillas, banana, sun, buildings, windows, and explosions rather than merely representing the same objects.

## Code Organization

- Keep app/window lifecycle code in `src/app.rs`.
- Keep gameplay state, rules, city generation, drawing decisions, and domain types under `src/game/` unless a clearer module split is needed.
- Keep GPU setup and primitive batching under `src/render/`.
- Add focused modules when behavior grows: likely candidates are `physics`, `city`, `input`, `score`, and `animation`.
- Avoid broad rewrites while porting a specific feature. Move code only when it directly reduces complexity for the feature being implemented.

## Implementation Guidance

- Translate BASIC functions by intent, not by syntax. For example, `MakeCityScape`, `PlaceGorillas`, `PlotShot`, `DoShot`, `DoExplosion`, `DoSun`, and `UpdateScores` should become Rust functions or methods with clear ownership and typed results.
- Replace magic integer modes with enums where useful, especially for game screens, turn state, shot outcomes, gorilla arms, and sun expression.
- Keep numeric behavior close to the original unless changing it is necessary for stable frame-based animation. Document meaningful deviations.
- Prefer small, testable pure functions for physics, collision, scoring, and city placement.
- Do not reintroduce obsolete machine-specific QBasic behavior such as NumLock `PEEK`/`POKE` handling.

## Workflow

- There is currently no Taskfile. Use standard Cargo commands unless project automation is added later.
- Before finishing code changes, run at least:
  - `cargo fmt --check`
  - `cargo test`
- For rendering or app lifecycle changes, also run:
  - `cargo check`
  - `cargo run`
- If a command cannot be run in the current environment, state that explicitly in the final response.

## Quality Bar

- New gameplay logic should have unit tests when it can be tested without opening a window.
- Keep public APIs narrow and avoid exposing renderer internals to game logic.
- Do not silently remove original features. If a feature is deferred, leave the code in a state that makes the missing behavior clear.
- Keep comments sparse and useful: explain porting decisions, non-obvious BASIC behavior, or intentional deviations.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **gorillas** (142 symbols, 277 relationships, 8 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## When Debugging

1. `gitnexus_query({query: "<error or symptom>"})` — find execution flows related to the issue
2. `gitnexus_context({name: "<suspect function>"})` — see all callers, callees, and process participation
3. `READ gitnexus://repo/gorillas/process/{processName}` — trace the full execution flow step by step
4. For regressions: `gitnexus_detect_changes({scope: "compare", base_ref: "main"})` — see what your branch changed

## When Refactoring

- **Renaming**: MUST use `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` first. Review the preview — graph edits are safe, text_search edits need manual review. Then run with `dry_run: false`.
- **Extracting/Splitting**: MUST run `gitnexus_context({name: "target"})` to see all incoming/outgoing refs, then `gitnexus_impact({target: "target", direction: "upstream"})` to find all external callers before moving code.
- After any refactor: run `gitnexus_detect_changes({scope: "all"})` to verify only expected files changed.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Tools Quick Reference

| Tool | When to use | Command |
|------|-------------|---------|
| `query` | Find code by concept | `gitnexus_query({query: "auth validation"})` |
| `context` | 360-degree view of one symbol | `gitnexus_context({name: "validateUser"})` |
| `impact` | Blast radius before editing | `gitnexus_impact({target: "X", direction: "upstream"})` |
| `detect_changes` | Pre-commit scope check | `gitnexus_detect_changes({scope: "staged"})` |
| `rename` | Safe multi-file rename | `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` |
| `cypher` | Custom graph queries | `gitnexus_cypher({query: "MATCH ..."})` |

## Impact Risk Levels

| Depth | Meaning | Action |
|-------|---------|--------|
| d=1 | WILL BREAK — direct callers/importers | MUST update these |
| d=2 | LIKELY AFFECTED — indirect deps | Should test |
| d=3 | MAY NEED TESTING — transitive | Test if critical path |

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/gorillas/context` | Codebase overview, check index freshness |
| `gitnexus://repo/gorillas/clusters` | All functional areas |
| `gitnexus://repo/gorillas/processes` | All execution flows |
| `gitnexus://repo/gorillas/process/{name}` | Step-by-step execution trace |

## Self-Check Before Finishing

Before completing any code modification task, verify:
1. `gitnexus_impact` was run for all modified symbols
2. No HIGH/CRITICAL risk warnings were ignored
3. `gitnexus_detect_changes()` confirms changes match expected scope
4. All d=1 (WILL BREAK) dependents were updated

## Keeping the Index Fresh

After committing code changes, the GitNexus index becomes stale. Re-run analyze to update it:

```bash
npx gitnexus analyze
```

If the index previously included embeddings, preserve them by adding `--embeddings`:

```bash
npx gitnexus analyze --embeddings
```

To check whether embeddings exist, inspect `.gitnexus/meta.json` — the `stats.embeddings` field shows the count (0 means no embeddings). **Running analyze without `--embeddings` will delete any previously generated embeddings.**

> Claude Code users: A PostToolUse hook handles this automatically after `git commit` and `git merge`.

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
