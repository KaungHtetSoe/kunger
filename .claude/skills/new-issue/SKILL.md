---
name: new-issue
description: Create a GitHub issue in the Kunger repo using the recommended bug-report or feature-request template.
---

# Create a new Kunger GitHub issue

Use the repo's issue templates so every issue has the same structure and labels. Do not create blank issues unless the user explicitly asks for one.

## 1. Choose the template

Ask the user which kind of issue they want:

- **Bug report** — incorrect, missing, or crashing inventory behavior.
- **Feature request** — an enhancement or new capability.

If the user gives you a description in the same prompt, you can infer the template from the content instead of asking.

## 2. Gather the required fields

Open the template and fill in the sections with the user. Do not invent values the user hasn't provided.

### Bug report template (`.github/ISSUE_TEMPLATE/bug_report.md`)

Required frontmatter:

- `title`: starts with `[Bug] `
- `labels`: `bug`

Body sections to fill:

- **Description** — what went wrong.
- **Environment** — distribution/version, Kunger version, relevant package managers.
- **Steps to reproduce** — numbered list.
- **Expected behavior**
- **Actual behavior**
- **Logs / provider warnings** — redact sensitive paths/usernames.
- **Additional context** — optional.

### Feature request template (`.github/ISSUE_TEMPLATE/feature_request.md`)

Required frontmatter:

- `title`: starts with `[Feature] `
- `labels`: `enhancement`

Body sections to fill:

- **Problem** — what can't be done or is awkward.
- **Proposed solution**
- **Fits within Kunger's scope?** — confirm the request does not involve installing/updating/removing software, executing discovered binaries, or requiring root/sudo. Kunger is read-only (see `docs/PRODUCT_SPEC.md` Non-Goals).
- **Alternatives considered** — optional.

## 3. Create the issue

Use the `gh` CLI. Build the title and body from the filled template, then run:

```bash
gh issue create --repo <owner>/kunger --title "<title>" --body "$(cat <<'EOF'
<body>
EOF
)" --label "<label>"
```

For bug reports use `--label bug`; for feature requests use `--label enhancement`.

## 4. Report back

Share the created issue URL with the user. Do not assign the issue, set milestones, or add it to a project unless the user asks.
