# Next task — rehydration pointer

**For:** ZCode (or any agent picking this up), written 2026-09-04, updated
2026-09-06 to close out section 2 below (it originally pointed at open
work that has since been completed and merged — see the update note there
before acting on the rest of this file).

## 1. Status of the video work: done, nothing pending here

- PR #50 (`docs: video podcast media`) merged to `main`, including a
  follow-up fix commit (`564bb51`) that resolved all 7 Copilot review
  comments plus a few more path/logic bugs found while verifying the
  "Rebuilding" instructions actually work (see that commit's message for
  the full list — `audio/` vs `audio-dialogue/`, `video/` vs `clips/`, a
  `synthesize.js` retry-logic bug, hardcoded personal paths, an "xAA API"
  typo).
- Release [`podcast-2026-09-04`](https://github.com/daveboyd777/DaveBs-Freecell/releases/tag/podcast-2026-09-04)
  published with the final MP4 attached; SHA-256 verified to match the
  in-repo Git LFS copy exactly.
- No open PRs or issues in this repo as of this writing.

Nothing here needs re-verification or re-work. `AGENT_PLAN.md` and
`PODCAST.md` in this same folder are the historical record of that task,
kept as-is.

## 2. UPDATE 2026-09-06 — this item is CLOSED, not open work

The module described below has been **proposed, corrected, implemented, and
merged**. Do not re-propose or re-scope it — read the closeout first:

- **`daveboyd777/AIMaster-OS-v2`'s `docs/backlog.md` item #12** — status
  now CLOSED, cross-linking the story and archive tag below.
- **Closeout story:** `docs/stories/story-mediagen-module-2026-09-06.md`
  in that repo — what was asked, 5 corrections made to the original
  proposal (most notably: `DaveBs-Wiki` page slugs must match `[a-z0-9-]`
  exactly, and the cost-ledger `source` field has no `"mediaGen"` value),
  what was implemented, verification results, and what's explicitly left
  as follow-on work (converting this repo's `production/` into a real
  content pack/manifest, an actual OSv2 overview video, face/broll segment
  rendering, and a scheduled poll loop).
- **Usage/reference doc:** `docs/specs/media-gen.md` in that repo — the
  manifest schema and CLI (`aimaster mediaGen ...`) to use for any
  follow-on work, rather than the original scripts in this repo's own
  `production/` (superseded, kept here for history only).
- **Archive tag:** `archive/mediagen-module` on `daveboyd777/AIMaster-OS-v2`.

**Original context below, for history — superseded by the above:**

The operator's task was proposing (not yet implementing) a shared
documentation/media-generation module, generalizing the tools in
`production/` for use by OSv2 itself, the planned self-hosted wiki, and
FreeCell. "Cousins wiki" was confirmed 2026-09-05 to be the `DaveBs-Wiki`
repo (`github.com/daveboyd777/DaveBs-Wiki`, cloned locally at
`C:\Users\daveboyd\Desktop\rustwiki`), a sibling project to DaveBs-Freecell
and OSv2 — not the AIMaster-OS-v2 wiki.js effort. That resolution (recorded
in AIMaster-OS-v2 PR #198) is itself now superseded by the closeout above:
the module was implemented and merged the following day.
