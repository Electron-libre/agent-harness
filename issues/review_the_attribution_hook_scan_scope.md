# Narrow the text the attribution hook scans to the commit or pull request message

# Goal
`block_ai_attribution.rs` must refuse an agent credit in a message and must not refuse a command that only prints the
words; it must also catch the spaced variants of a trailer.

# Findings (from a code review, to reproduce first)
* `attribution` scans the whole command text. A commit with a clean message chained with an `echo` of a credit
  footer would be refused although the message is clean.
* The trailer test is a prefix match: a trailer written with a space before the colon may not be caught.

# Approach
1. Reproduce both with failing cases in `test_block_attribution.rs`.
2. Candidates: scan only the arguments of the commit or `gh` command segment; normalise the space before the colon.

# Resources
* `files/.claude/hooks/block_ai_attribution.rs`, `files/.claude/hooks/test_block_attribution.rs`

# Hints
* A message that only talks about the rule must keep passing.
