---
name: sprint-board
description: "Manages items (draft issues) on the Sprint Board GitHub Project (wakeuplabs-io/projects/4) via gh CLI. Accepts free-form prompts (may reference a US, PRD, proposal, discovery, or arbitrary text), refines/verifies with the user, then creates or modifies."
disable-model-invocation: true
---

# Sprint Board — item management

You manage items on the Sprint Board at <https://github.com/orgs/wakeuplabs-io/projects/4> using `gh`. The input is free-form text and may reference:

- a User Story (`US-XX`) from `docs/3-stories/story-map.md`
- a requirement from `docs/0-prd/`, `docs/1-proposal/`, `docs/2-discovery/`
- an ADR from `docs/architecture/adrs/`
- an ad-hoc description with no reference

**Input:** $ARGUMENTS

**Constants:**

- Owner: `wakeuplabs-io`
- Project number: `4`

---

## Phase 1 — Parse intent

1. Parse `$ARGUMENTS` to identify the operation. Only two are supported in this version:
   - `create` — create a new item (draft issue on the board)
   - `modify` — change **title and/or body** of an existing item
2. Accepted synonyms: `create`/`new`/`add` → create; `modify`/`edit`/`change`/`update` → modify.
3. If the intent is unclear, stop and ask the user. Do not assume.
4. For `modify`, identify how the user is referencing the item. Valid formats:
   - Project item number (e.g. `#3`, `item 3`)
   - Unique title substring
   - Draft issue URL or project URL
   - Node ID (`PVTI_…` for the project item, `DI_…` for the draft issue)

   If ambiguous, stop and ask the user to clarify.

**Gate:** do not proceed without knowing the `operation + target item` (the latter only for modify).

---

## Phase 2 — Resolve context

Goal: gather material to draft a faithful title + body, without inventing anything.

- **If the prompt references `US-XX`:** read `docs/3-stories/story-map.md` and extract the full story block (story, classification, acceptance signals, source, slice, discovery notes if any). These are the skeleton of the body.
- **If it references a file in `docs/`:** read the cited section. Capture the verbatim text so it can be attributed in `## Source`.
- **If it is free text with no reference:** do not invent acceptance signals or sources. The body must reflect **only** what the user wrote.

**Gate:** you have enough context to draft. If not, **ask** instead of inventing.

---

## Phase 3 — Draft the content

### For `create`

- **Title** (≤ 70 chars):
  - If derived from a US, use the format `US-XX · <short description>` taken from the existing heading.
  - Otherwise, an imperative verb phrase or clear noun phrase.
- **Body** (Markdown). Include only applicable sections:
  - `## Context` — one paragraph: what and why
  - `## Acceptance signals` — **only** if derived from a US/PRD; copy signals verbatim, do not paraphrase
  - `## Source` — file path + section (e.g. `docs/3-stories/story-map.md § US-A1`)
  - `## Slice` — only if it comes from the story map
  - `## Notes` — optional, for additional context from the user

### For `modify`

1. Fetch the current item state:

   ```bash
   gh project item-list 4 --owner wakeuplabs-io --format json \
     | jq '.items[] | select(<appropriate-filter>)'
   ```

   Save: `id` (PVTI_…), `content.id` (DI_… if DraftIssue), `content.title`, `content.body`, `content.type`.

2. Draft the new version (only the fields the user asked to change — if only body was requested, do not touch the title).

---

## Phase 4 — Review & verify with the user

**Mandatory.** Show the draft in this exact format before touching GitHub:

```text
Board:     Sprint Board (wakeuplabs-io/projects/4)
Action:    <create | modify>
Item:      <modify only — number + current title + URL>

── Proposed ──
Title:     <title>
Body:
<body rendered with its sections>
```

For `modify`, also show a **readable diff** between current and proposed (use `---`/`+++` or `-`/`+` bullet lines).

Ask verbatim:

> Confirm? Reply **yes** to apply, **no** to cancel, or describe what to change.

- If the user replies **no** → abort without touching anything.
- If the user asks for changes → iterate (back to Phase 3) and **re-show** the full block.
- If the user replies **yes** → proceed to Phase 5.

**Never** skip this phase, even if the original prompt seems unambiguous.

---

## Phase 5 — Apply

### Create — draft issue on the board

Use stdin for the body to avoid multiline escaping issues:

```bash
printf '%s' "<body>" | gh project item-create 4 \
  --owner wakeuplabs-io \
  --title "<title>" \
  --body-file -
```

Capture the URL returned.

### Modify — title and/or body of a draft issue

The `title`/`body` fields of a draft issue **cannot** be edited with `gh project item-edit` (that command only touches project fields). Use the `updateProjectV2DraftIssue` GraphQL mutation with the `DI_…` id:

- If both change:

  ```bash
  gh api graphql -f query='
    mutation($id:ID!, $title:String!, $body:String!) {
      updateProjectV2DraftIssue(input:{draftIssueId:$id, title:$title, body:$body}) {
        draftIssue { id title }
      }
    }
  ' -F id=<DI_...> -f title="<title>" -f body="<body>"
  ```

- If only one changes, omit the other argument and its variable from the mutation.

**If the item is a real issue** (not a draft — `content.type == "Issue"`): use `gh issue edit <url> --title … --body …` on the underlying issue. The project item updates automatically.

### Common errors

- `your authentication token is missing required scopes` → tell the user to run `gh auth refresh -s read:project,project` and stop.
- Item not found → list available items (number + title) and ask the user to confirm which one.

---

## Phase 6 — Report

To the user, in 3–4 lines:

- Operation performed
- Final title
- Item URL
- What changed (modify only: summary of the fields touched)

---

## General rules

- **Confirmation is mandatory** (Phase 4) before any `gh` command that writes. Read-only commands (list/view) do not require it.
- **Do not invent** acceptance signals, sources, slices, or classification. If they are not in the prompt or in the referenced document, leave them out.
- **Do not modify** the project itself (fields, views, workflows). Items only.
- **Do not add** labels, assignees, status, priority, size, or dates in this version. Out of scope until the user explicitly requests it.
- **Do not use** `--no-verify`, destructive `gh auth login`, or delete items. This version does not support delete.
- When in doubt, **ask** instead of assuming.
