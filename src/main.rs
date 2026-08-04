use freecell::{Game, Loc};
use std::io::{self, BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or_else(random_seed);

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let mut seed = seed;
    let mut game = Game::deal(seed);

    println!("FreeCell - type ? for help");
    loop {
        render(&game, seed);
        if game.is_won() {
            println!("*** You won game #{seed}! ***");
            println!("Type n for a new game or q to quit.");
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
                if !game.undo() {
                    println!("Nothing to undo.");
                }
                continue;
            }
            "a" | "auto" => {
                let n = auto_to_foundations(&mut game);
                println!("Sent {n} card(s) home.");
                continue;
            }
            "r" | "restart" => {
                game = Game::deal(seed);
                continue;
            }
            _ => {}
        }

        if let Some(rest) = line.strip_prefix('n') {
            seed = rest.trim().parse::<u32>().unwrap_or_else(|_| random_seed());
            game = Game::deal(seed);
            continue;
        }

        match parse_move(&line) {
            Some((from, to)) => {
                if let Err(e) = game.do_move(from, to) {
                    println!("Illegal move: {e}.");
                }
            }
            None => println!("Unrecognized command '{line}' — type ? for help."),
        }
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

/// Commands are two location characters: 1-8 cascades, a-d free cells,
/// h (or f) the foundations. E.g. "1a", "35", "ah", "2h".
fn parse_move(cmd: &str) -> Option<(Loc, Loc)> {
    let mut chars = cmd.chars().filter(|c| !c.is_whitespace());
    let from = parse_loc(chars.next()?)?;
    let to = parse_loc(chars.next()?)?;
    if chars.next().is_some() {
        return None;
    }
    // Foundations are never a source.
    if from == Loc::Foundation {
        return None;
    }
    Some((from, to))
}

fn parse_loc(c: char) -> Option<Loc> {
    match c {
        '1'..='8' => Some(Loc::Cascade(c as usize - '1' as usize)),
        'a'..='d' => Some(Loc::Free(c as usize - 'a' as usize)),
        'h' | 'f' => Some(Loc::Foundation),
        _ => None,
    }
}

/// Repeatedly send every playable card to the foundations.
fn auto_to_foundations(game: &mut Game) -> usize {
    let mut sent = 0;
    loop {
        let mut progressed = false;
        for i in 0..8 {
            if game.do_move(Loc::Cascade(i), Loc::Foundation).is_ok() {
                progressed = true;
                sent += 1;
            }
        }
        for i in 0..4 {
            if game.do_move(Loc::Free(i), Loc::Foundation).is_ok() {
                progressed = true;
                sent += 1;
            }
        }
        if !progressed {
            return sent;
        }
    }
}

fn render(game: &Game, seed: u32) {
    println!();
    println!("Game #{seed}   moves: {}", game.moves_played());

    let free: Vec<String> = game
        .freecells()
        .iter()
        .map(|c| c.map_or("__".to_string(), |c| c.to_string()))
        .collect();
    const SUIT_CHARS: [char; 4] = ['C', 'D', 'H', 'S'];
    const RANK_CHARS: [char; 14] = [
        '-', 'A', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K',
    ];
    let home: Vec<String> = game
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

    let depth = game.cascades().iter().map(|c| c.len()).max().unwrap_or(0);
    for row in 0..depth {
        let mut line = String::from(" ");
        for col in game.cascades() {
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
         \x20 r         restart this deal\n\
         \x20 n [seed]  new game (optionally a specific deal number)\n\
         \x20 q         quit\n"
    );
}
