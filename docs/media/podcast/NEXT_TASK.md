# Next task — rehydration pointer

**For:** ZCode (or any agent picking this up), written 2026-09-04 at the end
of the session that finished and merged the video work below.

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

## 2. The actual next task lives in AIMaster-OS-v2's backlog

The operator's next task — proposing (not yet implementing) a shared
documentation/media-generation module, generalizing the tools in
`production/` for use by OSv2 itself, the planned self-hosted wiki, and
FreeCell — is written up in full as **item #12** in
`daveboyd777/AIMaster-OS-v2`'s `docs/backlog.md`.

That's the canonical version; this file deliberately doesn't duplicate its
content (to avoid the two drifting apart) — go read it there. It covers:
the reference-implementation inventory of what each script in `production/`
actually does generically, the module-convention this repo's own patterns
suggest following, and five open design questions the proposal needs to
resolve (distribution model, content/engine separation, cost-tracking
integration, the async-generation polling gap, and how this relates to the
wiki.js effort).

**~~One thing that needs the operator, not an agent~~ — resolved
2026-09-05.** Backlog item #12 flagged that "cousins wiki" wasn't an
identified project. Dave confirmed: it is the `DaveBs-Wiki` repo
(`github.com/daveboyd777/DaveBs-Wiki`, cloned locally at
`C:\Users\daveboyd\Desktop\rustwiki`), a sibling project to DaveBs-Freecell
and OSv2. The shared documentation/media-generation module infrastructure
lives in OSv2, with FreeCell and `DaveBs-Wiki` as sibling consumers — and
this repo's video podcast is the reference instantiation of that pipeline,
to be instantiated for OSv2's own use as the next task. Recorded in
AIMaster-OS-v2 PR [#198](https://github.com/daveboyd777/AIMaster-OS-v2/pull/198)
(item #12 in `docs/backlog.md` remains the canonical write-up).
