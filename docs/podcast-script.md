# DaveB's Freecell: A Deep Dive — Podcast Script

A conversational, two-host script about this project, written to be fed
into any text-to-speech or AI podcast-generation tool of your choice
(NotebookLM-style "audio overview" tools, ElevenLabs, etc.). It's not
generated audio itself — just the script.

Two voices: **A** (curious host, asks the questions) and **B** (technical
host, explains the project). Runs roughly 12-15 minutes read aloud.

---

**A:** Okay, so today we're talking about a FreeCell implementation. Just
FreeCell — the solitaire card game. Why does that need a whole episode?

**B:** Because this one isn't just "deal cards, click cards." It's a
FreeCell game built in Rust, test-first, that grew into three different
user interfaces, a whole statistics and self-analysis system, a
from-scratch solver, and even a web dashboard written in JavaScript. And
every single one of those pieces shares the exact same game engine — no
duplicated rules anywhere.

**A:** Let's start at the beginning. What's the actual core of it?

**B:** The engine. It's a pure Rust library with zero I/O — no printing,
no file access, nothing. Just data and functions. The board state —
cascades, free cells, foundations — is one plain struct called
`GameState`. And here's the interesting architectural choice: it's built
Redux-style.

**A:** Redux, like the JavaScript state management library?

**B:** Same idea, different language. Every possible way the game can
change — moving a card, undoing, auto-playing to the foundations,
dealing a new game — is represented as one value in an `Action` enum.
And there's a single `reduce` function: you give it a state and an
action, it gives you back a new state. No mutation, no side effects.

**A:** Why go to that trouble for a card game?

**B:** Because it makes everything else fall out for free. Undo and redo
are just stacks of past and future states. And here's the fun part — a
game is *completely* described by two things: the seed it was dealt
from, and the list of actions played. That's it. So there's a `replay`
function that takes a seed and a list of actions and reconstructs the
exact game. Every single time this project's UI detects a win, it
actually replays the whole game from scratch and checks that it
reproduces the exact win — as a runtime proof, not just a test that
might bit-rot.

**A:** That's a nice sanity check to have running live.

**B:** It's also what makes the whole multi-UI story work. There's a
terminal UI, a desktop GUI, a browser version compiled to WebAssembly,
and — most recently — an Android build. All four call the exact same
`reduce` function. None of them know a single rule of FreeCell. A
supermove capacity formula, alternating colors, all of that logic lives
in exactly one place.

**A:** Let's talk about statistics, because I saw this thing tracks a
lot more than "did you win."

**B:** Right, so there's a `stats` module that's just a plain list of
game results plus some pure math over it — win percentage, current
streak, longest winning and losing streaks, that sort of thing. But how
it *gets* that data is the elegant part: it's a "subscriber" to the
Redux store. Every time an action successfully goes through, the stats
module gets a notification with the resulting state and the action that
caused it. It doesn't need any special hook into the game logic at all.

**A:** So adding a new UI would automatically feed stats too?

**B:** Exactly — and that's already proven three times over, because all
three live UIs share it. It even handles the annoying edge case of
someone just quitting mid-game without finishing — Ctrl+C, closing the
window, whatever — and still records that as a loss.

**A:** Now, the part I'm most curious about: you said there's a solver.
Doesn't that mean the computer can... solve FreeCell for you?

**B:** It can tell you whether a position is *still winnable*, and if so,
give you a full winning line. It's a depth-first search with a
transposition table — meaning it remembers every position it's already
explored so it never wastes time re-exploring the same board reached by
a different order of moves. And there's a classic FreeCell trick baked
in: a "safe autoplay" rule that sends cards to the foundation early
whenever it's mathematically guaranteed to never hurt you.

**A:** How fast is it? FreeCell's supposed to be really hard to solve in
general.

**B:** It genuinely is — there's a famous case, deal number 11982 in the
classic Microsoft numbering, that's provably unsolvable, and this solver
proves that in about seven and a half seconds on an unoptimized debug
build. Most positions resolve dramatically faster.

**A:** And this powers the hint button?

**B:** It does, with a twist. Hints use a much smaller search budget than
a full solve — tuned down specifically so it stays responsive during
play instead of freezing the UI for several seconds. If the small
budget can't find an answer, it just honestly says "no hint available"
rather than pretending the position is unsolvable.

**A:** That honesty thing seems important.

**B:** It's actually a whole design principle here. The solver has three
possible answers, not two: solvable, unsolvable, or *unknown* — meaning
"I didn't search deep enough to be sure." Every piece of code that uses
the solver is required to handle "unknown" as its own real case. Get
that wrong and you'd tell a player their game is dead when it might
still be winnable.

**A:** You mentioned a post-game report earlier too.

**B:** Right, this is one of my favorite pieces of the whole project.
When a game ends — or even if you just abandon it — there's a "grade"
function that tells you exactly where things went wrong. Not just
"you lost," but the specific move number where the position stopped
being winnable.

**A:** How do you even figure that out without solving every single
position in the game?

**B:** This is the elegant bit. There's a mathematical property here:
along one continuous real game, once a position becomes unsolvable, it
*stays* unsolvable for the rest of that game. It can never flip back to
solvable. Which means you can binary-search for the exact turning
point instead of checking every move one at a time. For a sixty-move
game, that's the difference between roughly sixty solver calls and
about six.

**A:** That's a nice bit of applied math for a card game.

**B:** It's the kind of thing that only shows up when you actually sit
down and think about what "solvable" means over a sequence of real
moves, rather than just brute-forcing it.

**A:** Let's shift to visualization, because I know there are actual
charts involved now.

**B:** Two tracks, deliberately. Track A is in-app — charts drawn with a
Rust charting library, embedded directly in the desktop and web builds:
a win-rate trend over time, and a histogram of how many moves your games
tend to take, broken down by wins and losses.

**A:** And track B?

**B:** Track B is a completely separate web dashboard, written in plain
JavaScript with D3 — no build tooling, no npm install, just static
files. It reads the exact same JSON export the Rust side produces. You
can hover a point on the win-rate chart to see which specific deal it
was, and click it to reopen the game pre-loaded on that exact numbered
deal for a rematch.

**A:** Why bother with two separate implementations of similar charts?

**B:** Because of a strict rule that keeps the whole thing safe: all
*computation* stays in Rust. The JSON schema is versioned and tested —
changing its shape is treated like a breaking API change, not a casual
edit. JavaScript is only ever allowed to render what's already computed,
never calculate a new statistic of its own. That means either
rendering track could be deleted and rewritten in a totally different
language tomorrow without touching a single line of game logic.

**A:** Real quick — you said replay links open "the exact numbered
deal." Is that the same as replaying the exact game you actually
played?

**B:** Good catch, and it's worth being precise about it: no. The
persisted history only stores the deal number, whether you won, and how
many moves it took — not the full move-by-move log. So a "replay" link
deals that same numbered game fresh, rather than replaying your exact
recorded sequence of moves. Since a deal number completely determines
the starting layout, it's still the same puzzle — just not a literal
instant-replay of your specific playthrough.

**A:** Fair enough. So what does the finished product actually look
like, end to end?

**B:** A text CLI for the terminal purists, a ratatui terminal UI with
mouse support and colored suits, a full desktop GUI built with egui —
complete with hand-drawn vector suit symbols, not font glyphs — a
browser version compiled straight to WebAssembly and deployed to GitHub
Pages, and as of very recently, a locally-buildable Android debug APK
using that exact same GUI code, just packaged differently for a phone.

**A:** All from the one engine.

**B:** All from the one engine. Continuous integration runs formatting
checks, lints, and the full test suite on every change, plus code
coverage reporting. There's a release workflow that packages binaries
for Windows, macOS, and Linux and publishes them to GitHub Releases
automatically whenever a version tag goes out. And Dependabot keeps
dependencies current, with routine patch-level updates merging
themselves automatically once tests pass.

**A:** It's a lot of engineering for a card game.

**B:** That's sort of the point, honestly — FreeCell is simple enough to
hold in your head completely, which makes it a great space to actually
practice good architecture without the domain complexity getting in the
way. Every one of these systems — the solver, the stats, the two
visualization tracks — is small enough to reason about on its own, and
that's exactly why it was possible to keep adding to it without anything
breaking.

**A:** Where would this go next?

**B:** There's a design-notes paper alongside this script that dives
into FreeCell solving strategies specifically — the patterns a solver
or a skilled player actually uses under the hood. Beyond that, the
natural next steps are things like solver-backed grading for entire
historical game libraries, or new UIs that plug into the same engine
the exact same way the existing four already do.

**A:** Well, that's DaveB's Freecell. Same fifty-two cards everyone
knows, a lot more going on underneath than you'd expect.

**B:** Every column laid out and dealt to prove it.

---

*(End of script.)*
