//! Statistics charts (issue #14): win-rate trend and move-count
//! distribution, rendered with `plotters`.
//!
//! The two `draw_*` functions are generic over `DrawingBackend` and are
//! the single source of truth for each chart's content -- both the
//! on-screen egui display (rendered into an in-memory RGB buffer via
//! `BitMapBackend::with_buffer`, no file I/O) and the native-only PNG/SVG
//! export share them, so the exported image is guaranteed to look exactly
//! like what's on screen.

use eframe::egui;
use freecell::stats::GameResult;
use plotters::backend::BitMapBackend;
use plotters::coord::Shift;
use plotters::prelude::*;

/// Pixel size charts are rendered at for on-screen display.
const DISPLAY_SIZE: (u32, u32) = (560, 320);

/// Pixel size charts are rendered at for file export -- larger than the
/// on-screen size for a sharper result when viewed outside the app.
#[cfg(not(target_arch = "wasm32"))]
const EXPORT_SIZE: (u32, u32) = (1000, 600);

/// How many moves wide each move-count-distribution bucket is.
const MOVES_PER_BUCKET: u32 = 10;

/// Render the win-rate trend (cumulative win percentage after each game,
/// in play order) as an egui-ready image.
pub fn win_rate_trend_image(history: &[GameResult]) -> egui::ColorImage {
    render_to_image(DISPLAY_SIZE, |root| draw_win_rate_trend(root, history))
}

/// Render the move-count distribution (a histogram of moves played,
/// green segments for wins and red for losses stacked on top) as an
/// egui-ready image.
pub fn move_count_distribution_image(history: &[GameResult]) -> egui::ColorImage {
    render_to_image(DISPLAY_SIZE, |root| {
        draw_move_count_distribution(root, history)
    })
}

fn render_to_image(
    size: (u32, u32),
    draw: impl FnOnce(&DrawingArea<BitMapBackend<'_>, Shift>) -> Result<(), Box<dyn std::error::Error>>,
) -> egui::ColorImage {
    let (w, h) = size;
    let mut buffer = vec![0u8; (w * h * 3) as usize];
    {
        let root = BitMapBackend::with_buffer(&mut buffer, (w, h)).into_drawing_area();
        draw(&root).expect("chart rendering into an in-memory buffer should never fail");
        root.present()
            .expect("presenting an in-memory chart buffer should never fail");
    }
    egui::ColorImage::from_rgb([w as usize, h as usize], &buffer)
}

/// Save the win-rate trend chart to `path` as a PNG.
#[cfg(not(target_arch = "wasm32"))]
pub fn export_win_rate_trend_png(
    history: &[GameResult],
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(path, EXPORT_SIZE).into_drawing_area();
    draw_win_rate_trend(&root, history)?;
    Ok(root.present()?)
}

/// Save the win-rate trend chart to `path` as an SVG.
#[cfg(not(target_arch = "wasm32"))]
pub fn export_win_rate_trend_svg(
    history: &[GameResult],
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = plotters::backend::SVGBackend::new(path, EXPORT_SIZE).into_drawing_area();
    draw_win_rate_trend(&root, history)?;
    Ok(root.present()?)
}

/// Save the move-count distribution chart to `path` as a PNG.
#[cfg(not(target_arch = "wasm32"))]
pub fn export_move_count_distribution_png(
    history: &[GameResult],
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(path, EXPORT_SIZE).into_drawing_area();
    draw_move_count_distribution(&root, history)?;
    Ok(root.present()?)
}

/// Save the move-count distribution chart to `path` as an SVG.
#[cfg(not(target_arch = "wasm32"))]
pub fn export_move_count_distribution_svg(
    history: &[GameResult],
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = plotters::backend::SVGBackend::new(path, EXPORT_SIZE).into_drawing_area();
    draw_move_count_distribution(&root, history)?;
    Ok(root.present()?)
}

fn draw_win_rate_trend<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    history: &[GameResult],
) -> Result<(), Box<dyn std::error::Error>>
where
    DB::ErrorType: 'static,
{
    root.fill(&WHITE)?;
    let game_count = history.len().max(1) as f64;

    let mut chart = ChartBuilder::on(root)
        .caption("Win Rate Trend", ("sans-serif", 20))
        .margin(15)
        .x_label_area_size(30)
        .y_label_area_size(45)
        .build_cartesian_2d(1f64..game_count, 0f64..100f64)?;
    chart
        .configure_mesh()
        .x_desc("Game #")
        .y_desc("Win rate (%)")
        .draw()?;

    if !history.is_empty() {
        let mut wins = 0u32;
        let points: Vec<(f64, f64)> = history
            .iter()
            .enumerate()
            .map(|(i, g)| {
                if g.won {
                    wins += 1;
                }
                ((i + 1) as f64, f64::from(wins) / (i + 1) as f64 * 100.0)
            })
            .collect();
        chart.draw_series(LineSeries::new(points.clone(), BLUE))?;
        chart.draw_series(
            points
                .into_iter()
                .map(|(x, y)| Circle::new((x, y), 2, BLUE.filled())),
        )?;
    }

    Ok(())
}

fn draw_move_count_distribution<DB: DrawingBackend>(
    root: &DrawingArea<DB, Shift>,
    history: &[GameResult],
) -> Result<(), Box<dyn std::error::Error>>
where
    DB::ErrorType: 'static,
{
    root.fill(&WHITE)?;

    let max_moves = history.iter().map(|g| g.moves).max().unwrap_or(0);
    let bucket_count = (max_moves / MOVES_PER_BUCKET + 1).max(1) as usize;
    let mut won = vec![0u32; bucket_count];
    let mut lost = vec![0u32; bucket_count];
    for g in history {
        let bucket = (g.moves / MOVES_PER_BUCKET) as usize;
        if g.won {
            won[bucket] += 1;
        } else {
            lost[bucket] += 1;
        }
    }
    let max_count = won
        .iter()
        .zip(&lost)
        .map(|(&w, &l)| w + l)
        .max()
        .unwrap_or(0)
        .max(1);

    let mut chart = ChartBuilder::on(root)
        .caption("Move-Count Distribution", ("sans-serif", 20))
        .margin(15)
        .x_label_area_size(30)
        .y_label_area_size(40)
        .build_cartesian_2d(
            0u32..(bucket_count as u32 * MOVES_PER_BUCKET),
            0u32..max_count,
        )?;
    chart
        .configure_mesh()
        .x_desc("Moves")
        .y_desc("Games")
        .draw()?;

    // Wins first (green, from the bottom), losses stacked on top (red),
    // so each bucket's total bar height is that bucket's total game count
    // and the color split shows the won/lost breakdown within it.
    chart.draw_series(won.iter().enumerate().map(|(i, &count)| {
        let x0 = i as u32 * MOVES_PER_BUCKET;
        let x1 = x0 + MOVES_PER_BUCKET;
        Rectangle::new([(x0, 0), (x1, count)], GREEN.filled())
    }))?;
    chart.draw_series(lost.iter().enumerate().map(|(i, &count)| {
        let x0 = i as u32 * MOVES_PER_BUCKET;
        let x1 = x0 + MOVES_PER_BUCKET;
        let base = won[i];
        Rectangle::new([(x0, base), (x1, base + count)], RED.filled())
    }))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(seed: u32, won: bool, moves: u32) -> GameResult {
        GameResult { seed, won, moves }
    }

    fn sample_history() -> Vec<GameResult> {
        vec![
            g(1, true, 42),
            g(2, false, 15),
            g(3, true, 130),
            g(4, true, 60),
        ]
    }

    #[test]
    fn win_rate_trend_image_is_the_expected_display_size_even_with_no_history() {
        let image = win_rate_trend_image(&[]);
        assert_eq!(
            image.size,
            [DISPLAY_SIZE.0 as usize, DISPLAY_SIZE.1 as usize]
        );
    }

    #[test]
    fn win_rate_trend_image_renders_with_real_history() {
        let image = win_rate_trend_image(&sample_history());
        assert_eq!(
            image.size,
            [DISPLAY_SIZE.0 as usize, DISPLAY_SIZE.1 as usize]
        );
    }

    #[test]
    fn move_count_distribution_image_is_the_expected_display_size_even_with_no_history() {
        let image = move_count_distribution_image(&[]);
        assert_eq!(
            image.size,
            [DISPLAY_SIZE.0 as usize, DISPLAY_SIZE.1 as usize]
        );
    }

    #[test]
    fn move_count_distribution_image_renders_with_real_history() {
        let image = move_count_distribution_image(&sample_history());
        assert_eq!(
            image.size,
            [DISPLAY_SIZE.0 as usize, DISPLAY_SIZE.1 as usize]
        );
    }

    /// Every exported file must actually exist, be non-empty, and start
    /// with that format's real signature -- not just "the function didn't
    /// return an error," which wouldn't catch e.g. an empty file.
    #[cfg(not(target_arch = "wasm32"))]
    fn assert_exported_file_is_valid(path: &std::path::Path, magic: &[u8]) {
        let bytes = std::fs::read(path).expect("exported file should exist and be readable");
        assert!(!bytes.is_empty(), "exported file must not be empty");
        assert!(
            bytes.starts_with(magic),
            "exported file at {path:?} didn't start with the expected format signature"
        );
        std::fs::remove_file(path).ok();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn charts_export_to_real_valid_png_and_svg_files() {
        const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let history = sample_history();
        let dir = std::env::temp_dir();

        let win_rate_png = dir.join(format!(
            "freecell_charts_test_{}_wr.png",
            std::process::id()
        ));
        export_win_rate_trend_png(&history, &win_rate_png).expect("PNG export succeeds");
        assert_exported_file_is_valid(&win_rate_png, &PNG_MAGIC);

        let win_rate_svg = dir.join(format!(
            "freecell_charts_test_{}_wr.svg",
            std::process::id()
        ));
        export_win_rate_trend_svg(&history, &win_rate_svg).expect("SVG export succeeds");
        assert_exported_file_is_valid(&win_rate_svg, b"<svg ");

        let moves_png = dir.join(format!(
            "freecell_charts_test_{}_mc.png",
            std::process::id()
        ));
        export_move_count_distribution_png(&history, &moves_png).expect("PNG export succeeds");
        assert_exported_file_is_valid(&moves_png, &PNG_MAGIC);

        let moves_svg = dir.join(format!(
            "freecell_charts_test_{}_mc.svg",
            std::process::id()
        ));
        export_move_count_distribution_svg(&history, &moves_svg).expect("SVG export succeeds");
        assert_exported_file_is_valid(&moves_svg, b"<svg ");
    }
}
