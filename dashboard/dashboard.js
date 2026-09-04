// Web stats dashboard (issue #20, Track B of the two-track visualization
// architecture -- see ROADMAP.md). All DOM wiring and D3 rendering lives
// here; every number and bucket comes from stats-view.mjs's pure
// transforms of a `freecell stats --json` export. This file computes
// nothing about the game itself -- only pixel coordinates.

import * as d3 from "https://cdn.jsdelivr.net/npm/d3@7/+esm";
import { validateExport, cumulativeWinRate, moveCountBuckets, formatStreak } from "./stats-view.mjs";

// A game is fully described by its deal seed (ROADMAP.md); since the
// persisted history doesn't include the full action log (only the
// outcome and move count), "replay" here means "deal this same numbered
// game again" in the WASM GUI, not replaying the exact recorded moves.
// `../` because this page is served from `dashboard/` alongside the
// GUI's own build output at the site root (see .github/workflows/pages.yml).
function replayUrl(seed) {
  return `../?seed=${encodeURIComponent(seed)}`;
}

const SAMPLE_DATA = {
  version: 1,
  games_played: 24,
  games_won: 18,
  games_lost: 6,
  win_percentage: 75.0,
  current_streak: { type: "winning", length: 4 },
  longest_winning_streak: 4,
  longest_losing_streak: 2,
  history: [
    { seed: 617, won: true, moves: 35 },
    { seed: 42, won: true, moves: 41 },
    { seed: 11982, won: false, moves: 12 },
    { seed: 205, won: true, moves: 58 },
    { seed: 3391, won: true, moves: 44 },
    { seed: 88, won: true, moves: 39 },
    { seed: 156, won: false, moves: 20 },
    { seed: 4021, won: false, moves: 15 },
    { seed: 9999, won: true, moves: 62 },
    { seed: 730, won: true, moves: 71 },
    { seed: 55, won: true, moves: 48 },
    { seed: 1234, won: false, moves: 25 },
    { seed: 6789, won: true, moves: 55 },
    { seed: 321, won: true, moves: 60 },
    { seed: 45, won: true, moves: 33 },
    { seed: 8080, won: false, moves: 18 },
    { seed: 999, won: true, moves: 90 },
    { seed: 271, won: true, moves: 47 },
    { seed: 828, won: true, moves: 52 },
    { seed: 314, won: false, moves: 22 },
    { seed: 159, won: true, moves: 65 },
    { seed: 265, won: true, moves: 58 },
    { seed: 358, won: true, moves: 40 },
    { seed: 979, won: true, moves: 77 },
  ],
};

const errorEl = document.getElementById("error");
const summaryEl = document.getElementById("summary");
const chartsEl = document.getElementById("charts");
const historyWrapperEl = document.getElementById("history-table-wrapper");
const tooltipEl = document.getElementById("tooltip");
const bucketDetailEl = document.getElementById("bucket-detail");

function showError(message) {
  errorEl.textContent = message;
  errorEl.classList.add("visible");
  summaryEl.classList.remove("visible");
  chartsEl.classList.remove("visible");
  historyWrapperEl.classList.remove("visible");
}

function clearError() {
  errorEl.classList.remove("visible");
  errorEl.textContent = "";
}

function showTooltip(event, lines) {
  tooltipEl.innerHTML = lines.map((line) => `<div>${line}</div>`).join("");
  tooltipEl.style.left = `${event.clientX + 14}px`;
  tooltipEl.style.top = `${event.clientY + 14}px`;
  tooltipEl.style.display = "block";
}

function hideTooltip() {
  tooltipEl.style.display = "none";
}

function renderSummary(data) {
  document.getElementById("stat-played").textContent = data.games_played;
  document.getElementById("stat-won").textContent = data.games_won;
  document.getElementById("stat-lost").textContent = data.games_lost;
  document.getElementById("stat-win-pct").textContent = `${data.win_percentage.toFixed(1)}%`;
  document.getElementById("stat-streak").textContent = formatStreak(data.current_streak);
  document.getElementById("stat-best-streak").textContent = data.longest_winning_streak;
  document.getElementById("stat-worst-streak").textContent = data.longest_losing_streak;
  summaryEl.classList.add("visible");
}

// Chart pixel size and margins mirror gui/src/charts.rs's DISPLAY_SIZE and
// layout constants, so this dashboard's charts read the same way as the
// in-app ones (Track A and Track B staying visually consistent) even
// though nothing here is literally shared code with the Rust side.
const WIDTH = 560;
const HEIGHT = 320;
const MARGIN = { top: 30, right: 20, bottom: 40, left: 50 };

function drawWinRateChart(points) {
  const svg = d3.select("#win-rate-chart");
  svg.selectAll("*").remove();

  const gameCount = Math.max(points.length, 1);
  const x = d3
    .scaleLinear()
    .domain([1, gameCount])
    .range([MARGIN.left, WIDTH - MARGIN.right]);
  const y = d3
    .scaleLinear()
    .domain([0, 100])
    .range([HEIGHT - MARGIN.bottom, MARGIN.top]);

  svg
    .append("g")
    .attr("class", "axis")
    .attr("transform", `translate(0,${HEIGHT - MARGIN.bottom})`)
    .call(d3.axisBottom(x).ticks(Math.min(gameCount, 10)).tickFormat(d3.format("d")));
  svg.append("g").attr("class", "axis").attr("transform", `translate(${MARGIN.left},0)`).call(d3.axisLeft(y));

  if (points.length === 0) return;

  const line = d3
    .line()
    .x((d) => x(d.gameNumber))
    .y((d) => y(d.winRate));
  svg.append("path").datum(points).attr("class", "win-rate-line").attr("d", line);

  svg
    .selectAll(".win-rate-point")
    .data(points)
    .join("circle")
    .attr("class", "win-rate-point")
    .attr("cx", (d) => x(d.gameNumber))
    .attr("cy", (d) => y(d.winRate))
    .attr("r", 3.5)
    .on("mousemove", (event, d) => {
      showTooltip(event, [
        `Game #${d.gameNumber} — Deal #${d.seed}`,
        `${d.won ? "Won" : "Lost"} in ${d.moves} moves`,
        `Win rate after this game: ${d.winRate.toFixed(1)}%`,
        "Click to replay this deal",
      ]);
    })
    .on("mouseleave", hideTooltip)
    .on("click", (_event, d) => window.open(replayUrl(d.seed), "_blank", "noopener"));
}

function drawMoveCountChart(buckets) {
  const svg = d3.select("#move-count-chart");
  svg.selectAll("*").remove();
  bucketDetailEl.classList.remove("visible");
  bucketDetailEl.innerHTML = "";

  const labels = buckets.map((b) => `${b.from}-${b.to}`);
  const x = d3.scaleBand().domain(labels).range([MARGIN.left, WIDTH - MARGIN.right]).padding(0.2);
  const maxCount = d3.max(buckets, (b) => b.won + b.lost) || 1;
  const y = d3
    .scaleLinear()
    .domain([0, maxCount])
    .range([HEIGHT - MARGIN.bottom, MARGIN.top]);

  svg
    .append("g")
    .attr("class", "axis")
    .attr("transform", `translate(0,${HEIGHT - MARGIN.bottom})`)
    .call(d3.axisBottom(x).tickValues(labels.filter((_, i) => i % Math.ceil(labels.length / 8 || 1) === 0)));
  svg
    .append("g")
    .attr("class", "axis")
    .attr("transform", `translate(${MARGIN.left},0)`)
    .call(d3.axisLeft(y).ticks(Math.min(maxCount, 8)).tickFormat(d3.format("d")));

  function showBucketDetail(bucket) {
    if (bucket.games.length === 0) {
      bucketDetailEl.innerHTML = "<p>No games in this bucket.</p>";
    } else {
      const rows = bucket.games
        .map(
          (g) =>
            `<tr><td class="${g.won ? "won" : "lost"}">${g.won ? "Won" : "Lost"}</td>` +
            `<td>Deal #${g.seed}</td><td>${g.moves} moves</td>` +
            `<td class="replay-link"><a href="${replayUrl(g.seed)}" target="_blank" rel="noopener">Replay</a></td></tr>`,
        )
        .join("");
      bucketDetailEl.innerHTML =
        `<h3>${bucket.from}-${bucket.to} moves (${bucket.won} won, ${bucket.lost} lost)</h3>` +
        `<table><tbody>${rows}</tbody></table>`;
    }
    bucketDetailEl.classList.add("visible");
  }

  // Wins stacked from the bottom, losses on top -- same convention as
  // gui/src/charts.rs's draw_move_count_distribution.
  svg
    .selectAll(".bucket-bar.won")
    .data(buckets)
    .join("rect")
    .attr("class", "bucket-bar won")
    .attr("x", (b) => x(`${b.from}-${b.to}`))
    .attr("width", x.bandwidth())
    .attr("y", (b) => y(b.won))
    .attr("height", (b) => y(0) - y(b.won))
    .on("mousemove", (event, b) => showTooltip(event, [`${b.from}-${b.to} moves`, `${b.won} won, ${b.lost} lost`]))
    .on("mouseleave", hideTooltip)
    .on("click", (_event, b) => showBucketDetail(b));

  svg
    .selectAll(".bucket-bar.lost")
    .data(buckets)
    .join("rect")
    .attr("class", "bucket-bar lost")
    .attr("x", (b) => x(`${b.from}-${b.to}`))
    .attr("width", x.bandwidth())
    .attr("y", (b) => y(b.won + b.lost))
    .attr("height", (b) => y(b.won) - y(b.won + b.lost))
    .on("mousemove", (event, b) => showTooltip(event, [`${b.from}-${b.to} moves`, `${b.won} won, ${b.lost} lost`]))
    .on("mouseleave", hideTooltip)
    .on("click", (_event, b) => showBucketDetail(b));
}

let tableSort = { key: "gameNumber", ascending: true };

function renderHistoryTable(points) {
  const tbody = document.querySelector("#history-table tbody");
  const sorted = [...points].sort((a, b) => {
    const dir = tableSort.ascending ? 1 : -1;
    const av = a[tableSort.key];
    const bv = b[tableSort.key];
    if (av === bv) return 0;
    return av > bv ? dir : -dir;
  });

  tbody.innerHTML = sorted
    .map(
      (p) =>
        `<tr><td>${p.gameNumber}</td><td>Deal #${p.seed}</td>` +
        `<td class="${p.won ? "won" : "lost"}">${p.won ? "Won" : "Lost"}</td><td>${p.moves}</td>` +
        `<td class="replay-link"><a href="${replayUrl(p.seed)}" target="_blank" rel="noopener">Replay</a></td></tr>`,
    )
    .join("");
  historyWrapperEl.classList.add("visible");
}

let currentPoints = [];

function render(data) {
  clearError();
  renderSummary(data);
  currentPoints = cumulativeWinRate(data.history);
  drawWinRateChart(currentPoints);
  drawMoveCountChart(moveCountBuckets(data.history));
  renderHistoryTable(currentPoints);
  chartsEl.classList.add("visible");
}

function loadJsonText(text) {
  try {
    render(validateExport(JSON.parse(text)));
  } catch (err) {
    showError(err.message || String(err));
  }
}

function loadFile(file) {
  const reader = new FileReader();
  reader.onload = () => loadJsonText(String(reader.result));
  reader.onerror = () => showError(`Could not read ${file.name}.`);
  reader.readAsText(file);
}

const dropZone = document.getElementById("drop-zone");
const fileInput = document.getElementById("file-input");

dropZone.addEventListener("click", () => fileInput.click());
dropZone.addEventListener("keydown", (event) => {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    fileInput.click();
  }
});
fileInput.addEventListener("change", () => {
  if (fileInput.files.length > 0) loadFile(fileInput.files[0]);
});

["dragenter", "dragover"].forEach((eventName) => {
  dropZone.addEventListener(eventName, (event) => {
    event.preventDefault();
    dropZone.classList.add("drag-over");
  });
});
["dragleave", "drop"].forEach((eventName) => {
  dropZone.addEventListener(eventName, (event) => {
    event.preventDefault();
    dropZone.classList.remove("drag-over");
  });
});
dropZone.addEventListener("drop", (event) => {
  const file = event.dataTransfer?.files?.[0];
  if (file) loadFile(file);
});

document.getElementById("sample-data-button").addEventListener("click", () => {
  render(validateExport(SAMPLE_DATA));
});

document.querySelectorAll("#history-table th[data-sort]").forEach((th) => {
  th.addEventListener("click", () => {
    const key = th.dataset.sort;
    tableSort = { key, ascending: tableSort.key === key ? !tableSort.ascending : true };
    renderHistoryTable(currentPoints);
  });
});
