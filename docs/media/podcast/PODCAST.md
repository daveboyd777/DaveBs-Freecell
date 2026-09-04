# Video Podcast — "DaveB's Freecell, in the Thin Man Style"

An 11-minute-52-second black-and-white video podcast that explains this
project the way the 1934 classic *The Thin Man* would: Nick Charles (the
technical half) walks Nora Charles (the curious half) through the engine,
the Redux-style store, the statistics module, the solver, the report-card
grading, and the four interfaces — with Asta the wire fox terrier providing
silent reaction shots.

- **Runtime:** 11:52 · 1920×1080 · 24 fps · H.264 + AAC (−16 LUFS)
- **Cast:** Dave Boyd as Nick (photo-animated, voice pitch-matched to a real
  voice sample) · Deb Boyd as Nora (photo-animated, neural TTS) · Asta as
  himself (generated)
- **B-roll:** the terminal footage is a **real deal from the actual engine**
  (game #17901, captured from `freecell.exe`); the GUI and dashboard shots
  are rendered mockups of the Phase-2 interfaces
- **Credits honesty:** styled as an homage after the fashion of the 1934
  film — not footage of William Powell or Myrna Loy

## Watch / download

| Source | Link |
|---|---|
| GitHub Release (recommended) | https://github.com/daveboyd777/DaveBs-Freecell/releases/tag/podcast-2026-09-04 |
| Direct asset download | https://github.com/daveboyd777/DaveBs-Freecell/releases/download/podcast-2026-09-04/DaveBs-Freecell-Video-Podcast.mp4 |
| In this repo (Git LFS) | [`DaveBs-Freecell-Video-Podcast.mp4`](DaveBs-Freecell-Video-Podcast.mp4) |

The Release asset is the practical download (no LFS quota involved). The
copy in this repo is the archival master; `git lfs install` + a normal
clone fetches it.

**SHA-256 (master):** `2a8e76f9cb180fe9…` (full hash via `sha256sum`)

## Contents of this folder

```
docs/media/podcast/
├── PODCAST.md                             this file
├── AGENT_PLAN.md                          executable handoff plan (Warp-X / Copilot)
├── DaveBs-Freecell-Video-Podcast.mp4      final video (Git LFS, ~290 MB)
└── production/                            everything needed to rebuild or restyle
    ├── script-source.pdf                  the original script document (markdown twin: docs/podcast-script.md)
    ├── dialogue.json                      script parsed into 51 timed segments
    ├── analyze-voice.js                   F0 analysis of the voice sample (pitch matching)
    ├── synthesize.js                      TTS synthesis (msedge-tts neural voices)
    ├── build.js                           segment renderer + concat (ffmpeg)
    ├── mix.js                             audio mix: placement, pan, film bed, loudnorm
    ├── shoot.js                           HTML → PNG via headless Edge
    ├── queue-videos.sh / queue-faces.sh   xAI grok-imagine-video generation jobs
    ├── package.json / package-lock.json   the one npm dependency (msedge-tts)
    ├── html/                              title cards + B-roll pages (6 files)
    ├── images/                            rendered 4K stills (8 PNG)
    ├── audio-dialogue/                    all 45 synthesized voice lines (MP3)
    └── clips/                             the 8 paid generated video clips (MP4)
```

## How it was made (and what it cost)

| Stage | Tool | Cost |
|---|---|---|
| Script parsing | `dialogue.json` (from `script-source.pdf`) | $0 |
| Nick's voice | Microsoft `ChristopherNeural` via msedge-tts, pitch-shifted (rubberband) to match the recorded sample's measured 96.7 Hz median F0 (lands at 97.6 Hz) | $0 |
| Nora's voice | Microsoft `JennyNeural` | $0 |
| Talking heads | xAI `grok-imagine-video-1.5` image-to-video from the two photos (4 clips × 10 s) | $3.24 |
| Asta + establishing shot | same model, text-to-video (3 × 6 s + 8 s) | $2.08 |
| Title cards, GUI, dashboard B-roll | HTML/CSS rendered by headless Edge | $0 |
| Terminal B-roll | real `freecell.exe` output | $0 |
| Assembly, film look, mix | FFmpeg (build.js / mix.js) | $0 |
| **Total (66 s of generated video @ $0.08/s)** | | **$5.32** |

That works out to ≈ **$0.45 per finished minute**. Known limitation (also
documented in AGENT_PLAN.md): Nick's voice is *pitch-matched*, not *cloned* —
no voice-cloning service was configured on the build machine. Adding one
(e.g. ElevenLabs, ~$5–22/mo tiers) only requires re-running the voice stage.

## Rebuilding

Requires: Node 20+, ffmpeg (with rubberband), Edge, an xAI API key (only if
regenerating paid clips). From `production/`:

```sh
npm install                       # msedge-tts
node analyze-voice.js path/to/your/sample.wav   # measure the reference voice sample
node synthesize.js                # all dialogue lines -> audio-dialogue/
node shoot.js                     # HTML cards/B-roll -> images/
bash queue-videos.sh              # Asta + establishing -> prints xAI request IDs (see note below)
bash queue-faces.sh               # talking-head clips -> prints xAI request IDs (see note below)
node build.js                     # 54 segments -> concat -> silent film
node mix.js                       # voice placement + pan + bed + loudnorm
ffmpeg -i ../work/film_video.mp4 -i ../work/mix.m4a -c copy -movflags +faststart out.mp4
```

`queue-videos.sh`/`queue-faces.sh` only *submit* the async xAI
generation jobs and print each one's request ID -- neither script polls
for completion or downloads the result. Once a job finishes, fetch its
video and save it as `clips/<name>.mp4` (matching the `queue`/`queue_face`
call's first argument, e.g. `nick_a`) before running `build.js`, which
reads directly from `clips/`.

Every stage is deterministic given the same inputs (seeded noise, fixed
deal #17901), except the xAI generations — those are re-rolls.
