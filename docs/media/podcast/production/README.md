# Production sources — video podcast

Everything here rebuilds the 11:21 video in `../DaveBs-Freecell-Video-Podcast.mp4`.
See `../PODCAST.md` for the overview and cost table, and `../AGENT_PLAN.md`
for the agent-executable workflow.

## One pipeline: OSv2 `mediaGen`

This pack is driven by the **`mediaGen` module in AIMaster-OS-v2** — the single
video-production pipeline for all daveboyd/Softlo media work. FreeCell's own
generation scripts (`build.js`, `mix.js`, `synthesize.js`, `queue-videos.sh`,
`queue-faces.sh`, `shoot.js`) have been **retired**; their logic now lives,
generalized and unit-tested, in `core/modules/mediaGen/` there. The declarative
`manifest.json` replaces all of them.

```sh
# from AIMaster-OS-v2 (the mediaGen module + CLI live there):
aimaster mediaGen run \
  <this-repo>/docs/media/podcast/production/manifest.json \
  --work <this-repo>/docs/media/podcast/production/work
# assemble -> mix (voices + period music bed, ducked) -> mux
```

`mediaGen run` reads the checked-in assets below; TTS, clip generation, and the
music cue are pre-generated into the pack (regenerate via the module's
`queue`/`tts`/`music` subcommands + an `XAI_API_KEY` / `ELEVENLABS_API_KEY`).

## Quick map

| File / dir | Role |
|---|---|
| `manifest.json` | the `MediaRunManifest` — 54 segments, voices, music; the whole build definition |
| `script-source.pdf` | the original script document (source of truth for the dialogue) |
| `dialogue.json` | that script parsed into 51 timed segments (speakers, Asta beats, B-roll cues) — `manifest.json` is generated from this |
| `analyze-voice.js` | WAV F0 analysis helper — measured the reference voice at 96.7 Hz median (used when tuning a voice) |
| `audio/` | the 45 synthesized voice lines (MP3) — Nick: real ElevenLabs clone; Nora: msedge `JennyNeural` |
| `music/underscore.mp3` | the period (1930s/40s) light-jazz underscore, generated via ElevenLabs Music, mixed under dialogue |
| `clips/` | the generated video clips (Asta, establishing shot, talking heads) |
| `images/`, `html/` | title cards + terminal/GUI/dashboard B-roll (terminal text is real `freecell.exe` output, deal #17901); `asta_ref.jpg` conditions the establishing shot |
| `jobs/jobs.jsonl` | the clip-generation job records (prompts, ids) |
| `tts-lines.json`, `tts-nick.json` | the TTS line manifests fed to the `tts` stage |

## Not checked in

`work/` (and legacy `work-v2/`) render intermediates — regenerable. The
personal source inputs (Dave & Deb Boyd's photos, Dave's voice-clone sample)
live in the **private** `daveboyd777/AIMaster-OS-v2` repo under
`core/modules/mediaGen/source-data/` — not in this public repo.

Environment: Node 20+, ffmpeg, plus the AIMaster-OS-v2 checkout for the
`mediaGen` CLI. `XAI_API_KEY` / `ELEVENLABS_API_KEY` only for regeneration.
