# Roadmap

The planned work of Cardinal, grouped into milestones, in the order they are meant to land. A milestone becomes
issues once it is shaped; the `rank` of each issue file orders those issues. States: `idea`, `shaped`, `in issues`, `done`;
`just release` adds `- Released: v<version>` to a `done` one.
The work done before this file existed is in `issues/done/`.

## Plan and release

- Goal: plan the work above the issues, and ship numbered releases with a changelog that consumers can pin.
- Why: what is planned but not yet an issue has no home in the repository, and the consumers pin commits with no
  word of what changed.
- State: done
- Released: v0.2.0
- Issues:
  - [plan_the_milestones_in_a_roadmap_before_they_become_issues](issues/done/plan_the_milestones_in_a_roadmap_before_they_become_issues.md)
  - [release_numbered_versions_with_a_changelog](issues/done/release_numbered_versions_with_a_changelog.md)

## Team use

- Goal: let several people share a project driven by Cardinal without stepping on each other.
- Why: no project uses Cardinal in a team yet; these are the blockers an audit found.
- State: done
- Released: v1.0.0
- Issues:
  - [read_the_user_language_from_a_personal_file](issues/done/read_the_user_language_from_a_personal_file.md)
  - [rank_the_issues_without_a_shared_queue_file](issues/done/rank_the_issues_without_a_shared_queue_file.md)

## Advisor

- Goal: let a session consult a stronger advisor model at the moments where advice pays, and measure what it brings.
- Why: an advisor is configured but no call is seen in the transcripts, no skill says when to consult it, and the
  evaluation cannot tell an advised session from a solo one.
- State: in issues
- Issues:
  - [count_the_advisor_calls_of_each_session](issues/count_the_advisor_calls_of_each_session.md)
  - [tell_the_issue_workflow_when_to_consult_the_advisor](issues/tell_the_issue_workflow_when_to_consult_the_advisor.md)
  - [compare_an_advised_executor_with_a_solo_model](issues/compare_an_advised_executor_with_a_solo_model.md)

## Lean agent loop

- Goal: keep one agent loop driven by skills and tools, with a subagent only where a measure justifies it, and judge
  each change of the instructions before it spreads.
- Why: the one subagent of the workflow never got its verdict, a skill missed when due is not counted, the
  instructions grew one issue at a time, and every verdict is observational.
- State: in issues
- Issues:
  - [judge_the_delegated_exploration_before_keeping_it](issues/judge_the_delegated_exploration_before_keeping_it.md)
  - [count_the_skills_a_session_should_have_loaded](issues/count_the_skills_a_session_should_have_loaded.md)
  - [audit_the_shared_instructions_for_conflicts_and_stale_guidance](issues/audit_the_shared_instructions_for_conflicts_and_stale_guidance.md)
  - [replay_a_reference_task_set_to_compare_two_harness_variants](issues/replay_a_reference_task_set_to_compare_two_harness_variants.md)

## Longer autonomy

- Goal: let one session drive several issues in a row, stopping the user once for the plans of the batch.
- Why: one session per issue works today; chaining them saves the user the restarts.
- State: in issues
- Issues:
  - [drive_several_issues_in_a_row_from_one_session](issues/drive_several_issues_in_a_row_from_one_session.md)
