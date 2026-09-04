# Agent Plan — Video Podcast Media Check-in

**For:** Warp-X or GitHub Copilot (coding agent)
**Written:** 2026-09-04, after the products were produced and checked in by
the ZCode session that built the video. This document lets any agent
verify, reproduce, or extend that work without that session's context.

**Status update (2026-09-04, later the same day):** everything in this plan
is now done — PR merged (with a follow-up fix commit for real path/logic
bugs found while verifying it), release published, checksums verified. See
[`NEXT_TASK.md`](NEXT_TASK.md) in this same folder for what comes after this
plan, rather than re-running the checklist below.

## 1. What exists (product references)

All products were produced on 2026-09-04 on DESKTOP (`C:\Users\daveboyd\Desktop\`)
and checked into this repository on branch `docs/video-podcast-media`.

| Product | Build-machine path | Repo path (this repo) | Other location |
|---|---|---|---|
| Final video (11:52, 1080p24, ~290 MB) | `Desktop\DaveBs-Freecell-Video-Podcast.mp4` | `docs/media/podcast/DaveBs-Freecell-Video-Podcast.mp4` (Git LFS) | GitHub Release asset, tag `podcast-2026-09-04` |
| Source script document | `Desktop\podcast-script.pdf` | `docs/media/podcast/production/script-source.pdf` | — |
| Segmented script (51 beats, timings) | `Desktop\podcast-build\dialogue.json` | `docs/media/podcast/production/dialogue.json` | — |
| Full build system (7 scripts + HTML) | `Desktop\podcast-build\` | `docs/media/podcast/production/` | — |
| 45 synthesized voice lines | `Desktop\podcast-build\audio\` | `docs/media/podcast/production/audio-dialogue/` (MP3) | — |
| 8 generated video clips (paid, $5.32) | `Desktop\podcast-build\video\` | `docs/media/podcast/production/clips/` | — |
| Rendered cards/B-roll stills (4K) | `Desktop\podcast-build\images\` | `docs/media/podcast/production/images/` | — |
| Build intermediates (not checked in) | `Desktop\podcast-build\work\`, `frames\`, `qa\` | — | regenerable via `production/` scripts |

SHA-256 of the final master starts `2a8e76f9cb180fe9…`.

## 2. Already completed by the producing session

1. ✅ Products staged under `docs/media/podcast/` with this plan + `PODCAST.md`.
2. ✅ Large binaries (`.mp4`, `.mp3` under that folder) tracked via Git LFS
   (`.gitattributes` at repo root).
3. ✅ README link added ("Video tour" section).
4. ✅ Branch pushed; PR opened and merged to `main`.
5. ✅ Release `podcast-2026-09-04` published with the MP4 attached (primary
   public download — LFS quota stays untouched).
6. ✅ Verification issue opened on this repo (see §3 links) and closed when
   §4 checks passed.

If any of 4–6 are missing when you read this (e.g. the session was
interrupted), do them in that order — commands in §5.

## 3. Canonical URLs (after release publish)

- Release page: `https://github.com/daveboyd777/DaveBs-Freecell/releases/tag/podcast-2026-09-04`
- Direct MP4: `https://github.com/daveboyd777/DaveBs-Freecell/releases/download/podcast-2026-09-04/DaveBs-Freecell-Video-Podcast.mp4`
- Docs: `docs/media/podcast/PODCAST.md` in the default branch.

## 4. Verification checklist (acceptance criteria)

Run from a clean clone (NOT the original checkout):

- [ ] `git lfs install && git clone https://github.com/daveboyd777/DaveBs-Freecell.git && cd DaveBs-Freecell`
      → `docs/media/podcast/DaveBs-Freecell-Video-Podcast.mp4` exists and is
      ~290 MB (LFS fetched, not a pointer file larger than 200 bytes).
- [ ] `sha256sum docs/media/podcast/DaveBs-Freecell-Video-Podcast.mp4`
      begins `2a8e76f9cb180fe9`.
- [ ] `gh release view podcast-2026-09-04 --repo daveboyd777/DaveBs-Freecell`
      lists the MP4 asset; the browser release page renders with the asset.
- [ ] `ffprobe` on the LFS file: h264 1920×1080 24 fps + aac, duration ≈ 711.7 s.
- [ ] README "Video tour" section links to `docs/media/podcast/PODCAST.md`
      and the release URL (both resolve).
- [ ] CI on the merge commit is green (docs-only change; Rust suite unaffected).

## 5. Command reference (if steps 4–6 of §2 need executing)

```sh
# push the branch (from the repo root, on docs/video-podcast-media)
git push -u origin docs/video-podcast-media

# open + merge the PR
gh pr create --title "docs: video podcast media (Thin Man style explainer)" \
  --body "Adds docs/media/podcast/ with the finished 11:52 video (LFS), full production sources, PODCAST.md viewer doc, and AGENT_PLAN.md. See docs/media/podcast/PODCAST.md."
gh pr merge docs/video-podcast-media --merge

# publish the release with the master attached (tag may target main now)
gh release create podcast-2026-09-04 \
  docs/media/podcast/DaveBs-Freecell-Video-Podcast.mp4 \
  --repo daveboyd777/DaveBs-Freecell \
  --title "Video Podcast — DaveB's Freecell (Thin Man style)" \
  --notes "An 11:52 black-and-white video tour of the project. See docs/media/podcast/PODCAST.md in the repo for credits, production notes, and rebuild instructions."

# open the tracking issue (close it once §4 passes)
gh issue create --repo daveboyd777/DaveBs-Freecell \
  --title "Verify video podcast check-in (LFS fetch, release asset, README links)" \
  --body-file docs/media/podcast/AGENT_PLAN.md
```

## 6. Known limitations / natural follow-ups

1. **Nick's voice is pitch-matched, not cloned.** The build machine had no
   voice-cloning service. If an ElevenLabs (or similar) key is added later:
   `npm i elevenlabs`, replace `synthesize.js`'s Nick path with a clone call
   using the 52 s reference sample (original at
   `Desktop\davevoicesample.wav`), re-run `synthesize.js`, `mix.js`, and the
   final mux — video does not need re-rendering.
2. **LFS bandwidth:** the in-repo master consumes ~290 MB of the 1 GB/month
   free LFS bandwidth per full fetch. Public viewers should use the Release
   asset; the README and PODCAST.md say so.
3. **Regenerating paid clips** costs ~$0.08/s on `grok-imagine-video-1.5`
   (see PODCAST.md cost table). The checked-in `clips/` avoid that until a
   restyle is wanted.
4. **The dashboard/GUI B-roll are mockups** of the Phase-2 interfaces. When
   the real `gui/` crate ships, re-render `html/broll_gui.html`'s beat from
   an actual screenshot and re-run `build.js`.
