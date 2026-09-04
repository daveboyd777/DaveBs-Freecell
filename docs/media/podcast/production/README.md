# Production sources — video podcast

Everything here rebuilds the 11:52 video in `../DaveBs-Freecell-Video-Podcast.mp4`.
See `../PODCAST.md` for the overview and cost table, and `../AGENT_PLAN.md`
for the agent-executable workflow. Quick map:

| File | Role |
|---|---|
| `script-source.pdf` | the original script document (source of truth for the dialogue) |
| `dialogue.json` | that script parsed into 51 timed segments (speakers, Asta beats, B-roll cues) |
| `analyze-voice.js` | WAV F0 analysis — measured the reference voice at 96.7 Hz median to tune Nick's TTS |
| `synthesize.js` | synthesizes all 45 spoken lines via msedge-tts (Nick: ChristopherNeural −12 Hz; Nora: JennyNeural) |
| `queue-videos.sh` / `queue-faces.sh` | xAI grok-imagine-video-1.5 jobs (Asta, establishing shot, talking heads) |
| `shoot.js` | renders the `html/` cards and B-roll pages to 4K PNGs with headless Edge |
| `build.js` | renders 54 video segments (AI clip → Ken Burns crossfade), concats |
| `mix.js` | places voices on the timeline, pans Nick L / Nora R, film-hiss bed, −16 LUFS loudnorm |
| `html/`, `images/` | title cards, terminal/GUI/dashboard B-roll (terminal text is real `freecell.exe` output, deal #17901) |
| `audio-dialogue/` | the 45 synthesized lines (MP3, 24 kHz) — regenerate with `synthesize.js` |
| `clips/` | the 8 paid generated clips ($5.32 total) |

Not checked in (regenerable, large): 48 kHz rubberband-shifted Nick WAVs
(one ffmpeg pass over `audio-dialogue/*_nick.mp3`, see `mix.js` input paths),
`work/` intermediates, `frames/`, `qa/`, `node_modules/`.

Environment needs: Node 20+, npm (dep: msedge-tts), ffmpeg with rubberband,
Microsoft Edge, `XAI_API_KEY` for clip regeneration only.
