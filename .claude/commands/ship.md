---
description: Review pending changes, create a branch, commit, and open a PR with juandhal as reviewer
---

Ship the current working-tree changes as a PR. Follow these steps strictly; stop and ask the user if any step is ambiguous.

## 1. Review changes

Run in parallel:
- `git status` (no `-uall`)
- `git diff` (unstaged)
- `git diff --staged`
- `git log --oneline -10`

Summarize in 2–4 bullets: what changed, why, and any risks. If there are no changes at all, stop and tell the user.

## 2. Branch from latest develop

**Important**: Per project memory, never stack on another feature branch — always branch from latest `develop`.

- If currently on `develop`: stash or carry uncommitted changes forward.
- Run `git checkout develop && git pull --ff-only origin develop`.
- Derive a kebab-case branch name from the change summary using a conventional prefix (`feat/`, `fix/`, `docs/`, `chore/`, `refactor/`, `test/`). Keep it ≤50 chars.
- `git checkout -b <branch>`.

If the working tree had uncommitted changes before switching, make sure they land on the new branch (via stash pop or by staging before the switch).

## 3. Commit

- Stage only the intended files by name (avoid `git add -A` / `git add .`).
- Do not stage anything that looks like a secret (`.env`, credentials, tokens).
- Write a conventional-style commit message: short subject (≤72 chars), then a body focused on the *why* when useful.
- End the message with:
  ```
  Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
  ```
- Pass the message via a HEREDOC. Never use `--no-verify` or `--amend`. If a pre-commit hook fails, fix the root cause and create a new commit.

## 4. Push and open PR

- `git push -u origin <branch>`.
- Create the PR against `develop` with `gh pr create`, using a HEREDOC body:

```
## Summary
<1–3 bullets>

## Test plan
- [ ] <checks relevant to this change>

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

- Add reviewer: pass `--reviewer juandhal` to `gh pr create` (or `gh pr edit --add-reviewer juandhal` if creation didn't accept it).
- Keep the PR title ≤70 chars.

## 5. Report back

Output the PR URL and a one-line summary. Nothing else.
