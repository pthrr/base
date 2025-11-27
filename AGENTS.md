# Agents

## Etiquette

- Be precise in what you propose and explain (what/why/where/current/future)
- Keep in mind that the user most often will not have the concrete code and problem at the top of his head. Therefore, provide context and explain from first principles.
- Never start writing code right away. Plan first and implement only when the user agreed.

## Workflow

- This project is interfaced using the tasks defined in Taskfile.yml. Building, testing, linting all run through `task` calls.
- Run `task ci` after completing substantial tasks for verification and validation.

## Code Style

- Minimize `#[allow(...)]` statements
- Minimize re-exports
- `lib.rs` and `mod.rs` only contain exports
- Only use `mod.rs`-style for modules
- `pub(crate)` and `pub(super)` are a code smells
- use proper variable names - even traditional i, x, c can become index, value, count

## Modifications

- Keep SOLID principles in mind when adding or modifying code
- For hot paths we want Data-oriented Design (DOD)
- Make sure changes are minimal and reuse as much existent primitives as possible (YAGNI)
- For tests we target behavioural and numerical equivalence - no bit-for-bit exactness
