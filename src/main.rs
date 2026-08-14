use freecell::{parse_move, replay, Action, Game, Store};
use std::io::{self, BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let original_seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or_else(random_seed);

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let mut store = Store::new(original_seed);
    // The replay log for the whole session: `Deal`/`Restart` both reset to an
    // absolute position, so this never needs clearing (see `replay`'s docs) —
    // it is always a valid `(original_seed, log)` reconstruction of `store`.
    let mut log: Vec<Action> = Vec::new();
    let mut replay_shown = false;

    println!("FreeCell - type ? for help");
    loop {
        render(&store);
        if store.state().is_won() {
            println!(
                "*** You won game #{}! ***",
                store.game().seed().unwrap_or(original_seed)
            );
            println!("Type n for a new game or q to quit.");
            if !replay_shown {
                print_replay(original_seed, &log, store.game());
                replay_shown = true;
            }
        } else {
            // Not won right now -- e.g. undo stepped back out of a win, or a
            // new deal/restart started. Reset so a later redo/replay back
            // into a won state gets verified again instead of being
            // silently skipped.
            replay_shown = false;
        }

        print!("> ");
        io::stdout().flush().ok();
        let line = match lines.next() {
            Some(Ok(line)) => line.trim().to_lowercase(),
            _ => break, // EOF
        };

        match line.as_str() {
            "" => continue,
            "q" | "quit" | "exit" => break,
            "?" | "help" => {
                print_help();
                continue;
            }
            "u" | "undo" => {
                dispatch(&mut store, &mut log, Action::Undo);
                continue;
            }
            "y" | "redo" => {
                dispatch(&mut store, &mut log, Action::Redo);
                continue;
            }
            "a" | "auto" => {
                let before = store.game().moves_played();
                if dispatch(&mut store, &mut log, Action::AutoPlay) {
                    println!(
                        "Sent {} card(s) home.",
                        store.game().moves_played() - before
                    );
                }
                continue;
            }
            "r" | "restart" => {
                dispatch(&mut store, &mut log, Action::Restart);
                continue;
            }
            _ => {}
        }

        if let Some(rest) = line.strip_prefix('n') {
            let new_seed = rest.trim().parse::<u32>().unwrap_or_else(|_| random_seed());
            dispatch(&mut store, &mut log, Action::Deal { seed: new_seed });
            continue;
        }

        match parse_move(&line) {
            Some((from, to)) => {
                dispatch(&mut store, &mut log, Action::Move { from, to });
            }
            None => println!("Unrecognized command '{line}' — type ? for help."),
        }
    }
}

/// Dispatch `action` through the store; on success, append it to the replay
/// log. Prints the action's error on failure. Returns whether it succeeded.
fn dispatch(store: &mut Store, log: &mut Vec<Action>, action: Action) -> bool {
    match store.dispatch(action) {
        Ok(()) => {
            log.push(action);
            true
        }
        Err(e) => {
            println!("Error: {e}.");
            false
        }
    }
}

/// Print the `(seed, actions)` replay log and verify it live: replaying it
/// from `seed` via [`freecell::replay`] must reproduce `expected` — the live
/// store's actual current `Game` (position, undo/redo history, and deal
/// seed), not merely *some* winning position. This is the proof issue #5
/// asks for, not just an assertion in tests.
fn print_replay(seed: u32, log: &[Action], expected: &Game) {
    println!("Replay log — deal #{seed}, {} action(s):", log.len());
    println!("  {log:?}");
    match replay(seed, log) {
        Ok(rebuilt) if &rebuilt == expected => {
            println!("  Replay verified: (seed, actions) reproduces the current game exactly.")
        }
        Ok(_) => println!("  Replay produced a different game (this is a bug)."),
        Err(e) => println!("  Replay failed: {e} (this is a bug)."),
    }
}

fn random_seed() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(1);
    // Classic deals are numbered 1..=32000.
    nanos % 32000 + 1
}

fn render(store: &Store) {
    let seed = store.game().seed().unwrap_or(0);
    println!();
    println!("Game #{seed}   moves: {}", store.game().moves_played());

    let free: Vec<String> = store
        .state()
        .freecells()
        .iter()
        .map(|c| c.map_or("__".to_string(), |c| c.to_string()))
        .collect();
    const SUIT_CHARS: [char; 4] = ['C', 'D', 'H', 'S'];
    const RANK_CHARS: [char; 14] = [
        '-', 'A', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K',
    ];
    let home: Vec<String> = store
        .state()
        .foundations()
        .iter()
        .enumerate()
        .map(|(i, &r)| format!("{}{}", SUIT_CHARS[i], RANK_CHARS[r as usize]))
        .collect();

    println!(
        "free  a:{} b:{} c:{} d:{}    home  {}",
        free[0],
        free[1],
        free[2],
        free[3],
        home.join(" ")
    );
    println!();
    println!("   1   2   3   4   5   6   7   8");

    let depth = store
        .state()
        .cascades()
        .iter()
        .map(|c| c.len())
        .max()
        .unwrap_or(0);
    for row in 0..depth {
        let mut line = String::from(" ");
        for col in store.state().cascades() {
            match col.get(row) {
                Some(card) => line.push_str(&format!("  {card}")),
                None => line.push_str("    "),
            }
        }
        println!("{line}");
    }
}

fn print_help() {
    println!(
        "\nMoves are two characters, source then destination:\n\
         \x20 1-8  cascade columns\n\
         \x20 a-d  free cells\n\
         \x20 h    foundations (home)\n\
         Examples: 35 = column 3 onto column 5, 1a = column 1 to free cell a,\n\
         \x20         2h = column 2 to its foundation, b4 = free cell b to column 4.\n\
         Ordered runs move together when enough free cells/columns are open.\n\n\
         Other commands:\n\
         \x20 a         send every playable card to the foundations\n\
         \x20 u         undo the last move\n\
         \x20 y         redo the last undone move\n\
         \x20 r         restart this deal\n\
         \x20 n [seed]  new game (optionally a specific deal number)\n\
         \x20 q         quit\n"
    );
}
