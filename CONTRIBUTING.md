# Contributing to TruShell

Welcome — we value your time and contributions. This document explains how to contribute, how we review work, and how to become a trusted committer.

Table of contents
- Quickstart (first-time contributors)
- How to file issues
- How to contribute code (PR process)
- Trusted committer path
- Mentorship & pairing
- Communication channels
- Security & reporting

Quickstart
1. Read CODE_OF_CONDUCT.md and GOVERNANCE.md.
2. Pick a "good-first-issue" or follow the New Contributor Checklist (NEW_CONTRIBUTOR_CHECKLIST.md).
3. Open a draft PR referencing an issue; add the checklist in the PR template; run tests locally and in CI.

How to file issues
- Use the issue templates (bug vs feature) under .github/ISSUE_TEMPLATE.
- Provide reproduction steps, environment, and a minimal log/trace.

How to contribute code
- Create an issue first for non-trivial work. Small fixes may open PRs but prefer an issue.
- Branch naming: feature/<short-desc>, fix/<short-desc>, chore/<short-desc>, docs/<short-desc>.
- PR must include: linked issue (or a short explanation), tests, changelog entry (if applicable), and the checklist in the PR template.
- CI must be green. New contributors' PRs are routed to the mentorship queue for at least one review by a core maintainer.
- Merges to protected branches require two human reviews and passing CI; critical subsystems require explicit core maintainer approval.

Trusted committer path
- Criteria: 10 merged PRs, demonstrated quality (tests, reviews), at least 1 mentor recommendation, and familiarity with release process.
- Trusted committers can be granted write access and responsibility for one working group.

Mentorship & pairing
- New contributors may request pairing sessions or office hours in the community channels.
- We encourage pair programming for your first 2–3 PRs.

Communication
- Primary channels: GitHub (issues/PRs), Discussions (for design), and our chat/Matrix/Discord (see GOVERNANCE.md for links).

Security & reporting
- Use the security disclosure process documented in GOVERNANCE.md.

Thank you for contributing to TruShell.
