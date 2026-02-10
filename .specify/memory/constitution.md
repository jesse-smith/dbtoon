<!--
Sync Impact Report
==================
Version change: N/A -> 1.0.0
Modified principles: N/A (initial creation)
Added sections:
  - Core Principles (5): Simplicity First, Engineering Fundamentals,
    Over-Engineering Guards, Test-Driven Development, Incremental Delivery
  - Principle Tensions & Resolution
  - Commit Discipline
  - Governance
Removed sections: N/A
Templates requiring updates:
  - .specify/templates/plan-template.md ........... no update needed
    (Constitution Check placeholder compatible)
  - .specify/templates/spec-template.md ........... no update needed
    (structure compatible)
  - .specify/templates/tasks-template.md .......... no update needed
    (TDD/commit references aligned)
  - .specify/templates/commands/*.md .............. no command templates exist
  - .specify/templates/agent-file-template.md ..... no update needed
    (no constitution references)
Follow-up TODOs: None
-->

# dbtoon Constitution

## Core Principles

### I. Simplicity First

Prioritize simplicity in user-facing design, architecture, and codebase.
When choosing between approaches, MUST prefer the one that is easier to
explain. This principle applies at every level: API surface, internal
architecture, and code structure.

**Rationale**: Simplicity reduces cognitive load, lowers onboarding cost,
and makes the system easier to change. Complexity that cannot be explained
is complexity that cannot be maintained.

### II. Engineering Fundamentals

All code MUST follow these software engineering principles:

- **DRY** (Don't Repeat Yourself) — eliminate knowledge duplication
- **YAGNI** (You Aren't Gonna Need It) — do not build for hypothetical
  futures
- **KISS** (Keep It Simple, Stupid) — prefer straightforward solutions
- **Separation of Concerns** — each module addresses one concern
- **Least Surprise** — behavior MUST match reasonable expectations
- **Composition over Inheritance** — favor composing small units
- **Dependency Inversion** — depend on abstractions, not concretions
- **Loose Coupling / High Cohesion** — minimize inter-module dependencies,
  maximize intra-module relatedness
- **Single Responsibility** — each unit has one reason to change
- **Explicit over Implicit** — make behavior and intent visible
- **Fail Fast** — surface errors immediately at the point of detection
- **Immutability by Default** — prefer immutable data; mutability MUST be
  justified
- **Meaningful Names** — names MUST communicate intent without requiring
  comments

### III. Over-Engineering Guards

Guard against over-engineering with these constraints:

- **Rule of Three**: Do not abstract until the pattern appears three times.
  Premature abstraction is worse than duplication.
- **Reversibility**: Prefer decisions that are easy to undo. When a
  reversible approach costs little, choose it even if YAGNI would reject
  the upfront investment.
- **Optimize for Changeability**: Design so that future modifications
  require minimal cascading changes.

### IV. Test-Driven Development (NON-NEGOTIABLE)

TDD is mandatory. Tests MUST be written before implementation.

- Tests encode **intent**, not implementation details
- Prefer fewer well-designed tests over shallow coverage
- Treat tests as first-class code with the same quality standards as
  production code
- Treat hard-to-write tests as **design signals** — if a test is difficult
  to write, the code under test likely needs restructuring
- Invest disproportionately in test quality

### V. Incremental Delivery

Develop in short sprints, each ending with a small concrete deliverable
that is ideally user-testable with minimal effort.

- Each commit MUST represent a **minimum viable unit of work** — the
  smallest change that is internally consistent, passes all tests, and
  leaves the codebase in a working state
- Each minimum viable unit of work MUST be committed before moving to
  the next
- Favor small, frequent deliverables over large, infrequent ones

## Principle Tensions & Resolution

Some principles are intentionally in tension. When conflicts arise, use
these resolution rules:

| Tension | Resolution |
|---------|------------|
| **DRY vs Rule of Three** | DRY governs *knowledge* duplication; Rule of Three governs *premature abstraction*. When they conflict, ask: am I duplicating **knowledge** or duplicating **code shape**? Duplicate code shape is acceptable until the third occurrence. |
| **Reversibility vs YAGNI** | Reversibility wins when the upfront cost is low. YAGNI wins when the upfront cost is not low. The threshold: would a future reversal require touching more than the current change set? |

## Commit Discipline

- Each commit represents one minimum viable unit of work
- A commit MUST be internally consistent, pass all tests, and leave the
  codebase in a working state
- Do not batch unrelated changes into a single commit
- Do not leave work uncommitted when moving to the next unit

## Governance

- This constitution supersedes all other development practices where
  conflicts arise
- Amendments require: (1) documented rationale, (2) review of impact on
  existing principles, and (3) version increment per semantic versioning
- All code reviews and PRs MUST verify compliance with these principles
- Complexity beyond what the principles permit MUST be explicitly justified
  in the relevant spec or plan document
- **Versioning policy**: MAJOR for principle removals/redefinitions, MINOR
  for new principles or material expansions, PATCH for clarifications and
  wording fixes

**Version**: 1.0.0 | **Ratified**: 2026-02-10 | **Last Amended**: 2026-02-10
