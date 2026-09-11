# Video Podcast — "DaveB's Freecell, in the Thin Man Style"

An 11-minute-21-second black-and-white video podcast that explains this
project the way the 1934 classic *The Thin Man* would: Nick Charles (the
technical half) walks Nora Charles (the curious half) through the engine,
the Redux-style store, the statistics module, the solver, the report-card
grading, and the four interfaces — with Asta the wire fox terrier providing
silent reaction shots.

- **Runtime:** 11:21 · 1920×1080 · 24 fps · H.264 + AAC (−16 LUFS)
- **Cast:** Dave Boyd as Nick (photo-animated, voice a real ElevenLabs clone
  of Dave's own recording) · Deb Boyd as Nora (photo-animated, neural TTS) ·
  Asta as himself (generated)
- **B-roll:** the terminal footage is a **real deal from the actual engine**
  (game #17901, captured from `freecell.exe`); the GUI and dashboard shots
  are rendered mockups of the Phase-2 interfaces
- **Score:** a short 1930s/40s-era light-jazz underscore (ElevenLabs Music),
  looped and ducked under the dialogue
- **Credits honesty:** styled as an homage after the fashion of the 1934
  film — not footage of William Powell or Myrna Loy

## Watch / download

| Source | Link |
|---|---|
| GitHub Release (recommended) | https://github.com/daveboyd777/DaveBs-Freecell/releases/tag/podcast-v2-2026-09-10 |
| Direct asset download | https://github.com/daveboyd777/DaveBs-Freecell/releases/download/podcast-v2-2026-09-10/DaveBs-Freecell-Video-Podcast.mp4 |
| In this repo (Git LFS) | [`DaveBs-Freecell-Video-Podcast.mp4`](DaveBs-Freecell-Video-Podcast.mp4) |

The Release asset is the practical download (no LFS quota involved). The
copy in this repo is the archival master; `git lfs install` + a normal
clone fetches it.

**SHA-256 (master):** `5b036351cb393e5d…` (full hash via `sha256sum`)

## Contents of this folder

```
docs/media/podcast/
├── PODCAST.md                             this file
├── AGENT_PLAN.md                          executable handoff plan (Warp-X / Copilot)
├── DaveBs-Freecell-Video-Podcast.mp4      final video (Git LFS, ~318 MB)
└── production/                            everything needed to rebuild or restyle
    ├── manifest.json                      the OSv2 mediaGen run manifest (54 segments) — the whole build definition
    ├── script-source.pdf                  the original script document (markdown twin: docs/podcast-script.md)
    ├── dialogue.json                      script parsed into 51 timed segments (manifest.json is generated from this)
    ├── analyze-voice.js                   F0 analysis of the voice sample (voice tuning)
    ├── tts-lines.json / tts-nick.json     the TTS line manifests fed to the mediaGen `tts` stage
    ├── jobs/jobs.jsonl                    clip-generation job records (prompts, ids)
    ├── html/                              title cards + B-roll pages (6 files)
    ├── images/                            rendered 4K stills + asta_ref.jpg (Asta conditioning frame)
    ├── audio/                             all 45 synthesized voice lines (MP3)
    ├── music/underscore.mp3               the period light-jazz score (ElevenLabs Music)
    └── clips/                             the generated video clips (MP4)
```

FreeCell's own generation scripts (`build.js`, `mix.js`, `synthesize.js`,
`shoot.js`, `queue-videos.sh`, `queue-faces.sh`) have been **retired** —
their logic now lives, generalized and tested, in the `mediaGen` module of
AIMaster-OS-v2, and `manifest.json` drives the whole build. See
`production/README.md`.

## How it was made (and what it cost)

| Stage | Tool | Cost |
|---|---|---|
| Script parsing | `dialogue.json` → `manifest.json` (from `script-source.pdf`) | $0 |
| Nick's voice | **Real ElevenLabs Instant voice clone** of Dave's own ~52 s recording | plan (Instant + Music, $60/yr) |
| Nora's voice | Microsoft `JennyNeural` via msedge-tts | $0 |
| Talking heads | xAI `grok-imagine-video-1.5` image-to-video from the two photos (4 clips × 10 s) | $3.24 |
| Asta + establishing shot | same model, text-to-video (3 × 6 s + 8 s) | $2.08 |
| Asta `establish` regen (v2) | same model, image-conditioned on `asta_ref.jpg` for continuity | ~$0.64 |
| Period score | ElevenLabs Music, one ≤90 s cue, looped + ducked | plan (see above) |
| Title cards, GUI, dashboard B-roll | HTML/CSS rendered by headless Edge | $0 |
| Terminal B-roll | real `freecell.exe` output | $0 |
| Assembly, film look, mix | OSv2 `mediaGen` (ffmpeg) | $0 |
| **Total metered generation** | | **≈ $5.96** |

Plus the ElevenLabs subscription (Instant voice cloning + Music, $60/yr)
that powers Nick's real clone and the score. Nick's voice is now a **real
clone of Dave's own recording**, not the pitch-matched TTS of the first
cut.

## Rebuilding

There is now **one pipeline**: the `mediaGen` module in AIMaster-OS-v2.
`manifest.json` in `production/` is the whole build definition. Requires:
Node 20+, ffmpeg, an AIMaster-OS-v2 checkout for the `mediaGen` CLI, and
(only to regenerate paid assets) `XAI_API_KEY` / `ELEVENLABS_API_KEY`.

```sh
# from an AIMaster-OS-v2 checkout (module + CLI live there):
aimaster mediaGen run \
  <this-repo>/docs/media/podcast/production/manifest.json \
  --work <this-repo>/docs/media/podcast/production/work
# queue -> poll -> tts -> assemble -> mix (voices + ducked period score) -> mux
```

The checked-in `audio/`, `clips/`, `music/underscore.mp3`, and B-roll stills
let `run` assemble the video without spending anything. To regenerate them,
use the module's `queue`/`tts`/`music` subcommands (needs the API keys) —
e.g. re-enroll Nick's clone from Dave's reference sample, re-synthesize the
TTS lines, or re-compose the score. See `production/README.md` for the
per-file map.

Every stage is deterministic given the same inputs (seeded noise, fixed
deal #17901), except the xAI generations and ElevenLabs synthesis — those
are re-rolls.
