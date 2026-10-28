# Settle the clippy pedantic follow-ups

# Goal
The `high` review of `rename_the_harness_named_tools_to_cardinal` found in commit `e5352ff` (clippy pedantic) a
change of behaviour at the edges and three fixes that moved a problem instead of removing it. Settle each one.

# Findings
* `crates/cardinal/src/tools/cardinal_report.rs:576`: `av == bv` became `av.total_cmp(&bv).is_eq()`. `total_cmp`
  orders `-0.0` before `0.0` and makes `NaN` equal to itself, so a `-0.00` after `0.00` reads `helped` or `hurt`
  instead of `no observed effect`. The metrics are counts and ratios of counts, so `-0.00` should not occur; not
  observed.
* `crates/cardinal/src/tools/check_specs.rs:270`: the split of `parse` declares the same `problem` closure
  (`out.push(format!("{file}: {m}"))`) twice; the two copies can drift.
* `crates/cardinal/src/tools/review_gate.rs:90`: `batch.iter().map(|n| [n, "\n"].concat()).collect::<String>()`
  still allocates one `String` per name; one buffer does it.
* `crates/cardinal/src/budget.rs:36`: an `expect("three parts")` became a `let … else { return Vec::new() }`: a
  broken invariant now drops the warning of SYN-38 silently instead of failing in the tests.

# Approach
1. `total_cmp`: restore `av == bv` under `#[allow(clippy::float_cmp)]` with its reason, or keep it
   and say why.
2. One shared helper for `problem`; `join` or `push_str` in `review_gate`; restore a loud failure in `budget`
   (`unreachable!` or `debug_assert!`), each under `just check`.

# Resources
* `git show e5352ff`
* `crates/cardinal/src/tools/cardinal_report.rs`, `check_specs.rs`, `review_gate.rs`, `crates/cardinal/src/budget.rs`
