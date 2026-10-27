# Bootstrap the spec of an existing capability

# Goal
A capability gets a spec only when an issue changes it, so a project that adopts the harness, or an old capability
of this repository, has behaviour no spec states. Give a way to write the spec of an existing capability from its code
and tests, traced to the tests that already prove it.

# Findings
* `specs/` covers the capabilities changed since the spec format existed; `issue-workflow` writes a spec only when
  an issue changes what a capability does.
* `write-spec` starts from an intended change, not from existing behaviour.

# Approach
1. A procedure in `write-spec` (or its own skill) for an existing capability: read the code and its tests, state
   only the behaviour a test already proves, add the `// spec:` citations, list the untested behaviour as a follow-up
   issue rather than a requirement.
2. A report of the code that no `accepted` spec covers (by module or by test file without a citation), to rank the
   capabilities worth a spec.

# Resources
* `files/.claude/skills/write-spec/SKILL.md`, `files/specs/README.md`, `files/.claude/specs/check_specs.rs`

# Hints
* A spec read back from the code states what the code does, bugs included: the user validates it like any spec.
