/**
 * Application entry point.
 *
 * Wires the game state, canvas renderer, input controller and DOM panels
 * together, then boots the first game. All UI actions are also available as
 * keyboard shortcuts:
 *
 *   N = new game · U = undo · A = auto-play · K = kanji/English
 *   F = flip board · +/- = zoom · Space = ask the AI for a move
 *   D = toggle developer tools panel
 */

import './styles/index.css';

import { GameState, type Mode } from './game/state';
import { fetchConfig, fetchGpu, fetchLogs } from './api/client';
import {
  buildGameFile,
  buildPgn,
  gameFileName,
  serializeGameFile,
  type GameFormat,
} from './game/export';
import { Controller } from './ui/controller';
import { updatePanels } from './ui/panel';
import { MAX_ZOOM, MIN_ZOOM } from './game/view';
import { describeSearch, telemetrySeries } from './ui/telemetry';

// ── Bootstrap ──────────────────────────────────────────────────────

const canvas = document.getElementById('board-canvas') as HTMLCanvasElement;
const ctx = canvas.getContext('2d');
if (!ctx) throw new Error('2D canvas context is not available in this browser');

const state = new GameState();
// Constructed for side effects: binds events & subscribes to state.
new Controller(canvas, ctx, state);

// Update DOM panels whenever the game state changes.
state.subscribe(() => updatePanels(state));

const $ = <T extends HTMLElement = HTMLElement>(id: string): T =>
  document.getElementById(id) as T;
const select = (id: string): HTMLSelectElement => $<HTMLSelectElement>(id);

/** Start a browser download for generated text (TSFEN, JSON, PGN…). */
function downloadText(contents: string, filename: string, type: string): void {
  const blob = new Blob([contents], { type });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

// ── Toolbar ────────────────────────────────────────────────────────

function readSettingsFromControls(): void {
  state.settings.mode = select('mode-select').value as Mode;
  state.settings.humanColor = Number(select('color-select').value) as 0 | 1;
  state.settings.aiDepth = Number(select('ai-depth').value);
}

function syncControlsFromSettings(): void {
  select('mode-select').value = state.settings.mode;
  select('color-select').value = String(state.settings.humanColor);
  select('ai-depth').value = String(state.settings.aiDepth);
  const zoom = $<HTMLInputElement>('zoom-range');
  zoom.value = String(Math.round(state.settings.zoom * 100));
  $('zoom-label').textContent = `${Math.round(state.settings.zoom * 100)}%`;
}

$('btn-new').addEventListener('click', () => {
  readSettingsFromControls();
  state.setAutoPlay(false);
  void state.reset();
});

$('btn-undo').addEventListener('click', () => void state.undo(1));

$('btn-auto').addEventListener('click', () => {
  readSettingsFromControls();
  state.setAutoPlay(!state.settings.autoPlay);
});

$('btn-kanji').addEventListener('click', () => state.toggleKanji());
$('btn-flip').addEventListener('click', () => state.toggleFlip());
$('btn-analyze').addEventListener('click', () => void state.analyze());
$('btn-ai-move').addEventListener('click', () => void state.aiMove());

$<HTMLInputElement>('zoom-range').addEventListener('input', (event) => {
  const value = Number((event.target as HTMLInputElement).value) / 100;
  state.setZoom(value);
  $('zoom-label').textContent = `${Math.round(state.settings.zoom * 100)}%`;
});

select('mode-select').addEventListener('change', readSettingsFromControls);
select('color-select').addEventListener('change', readSettingsFromControls);
select('ai-depth').addEventListener('change', readSettingsFromControls);

// ── Position (TSFEN) panel ────────────────────────────────────────

const tsfenBox = $<HTMLTextAreaElement>('tsfen-box');

$('btn-save').addEventListener('click', async () => {
  const saved = await state.save();
  if (saved && document.activeElement !== tsfenBox) tsfenBox.value = saved.tsfen;
});

$('btn-load').addEventListener('click', async () => {
  readSettingsFromControls();
  const tsfen = tsfenBox.value.trim();
  if (!tsfen) {
    state.announce('Paste a TSFEN string first', 'error');
    return;
  }
  state.setAutoPlay(false);
  await state.load(tsfen);
});

$('btn-copy').addEventListener('click', async () => {
  try {
    await navigator.clipboard.writeText(tsfenBox.value);
    state.announce('TSFEN copied to clipboard', 'success');
  } catch {
    tsfenBox.select();
    state.announce('Clipboard blocked — text selected, press Ctrl+C', 'error');
  }
});

$('btn-download').addEventListener('click', () => {
  const filename = `taikyoku-move-${state.board?.move_number ?? 1}.tsfen`;
  downloadText(tsfenBox.value, filename, 'text/plain');
  state.announce('TSFEN downloaded', 'success');
});

// ── Game export ───────────────────────────────────────────────────

function exportGame(format: GameFormat): void {
  const board = state.board;
  if (!board) {
    state.announce('No game to export yet', 'error');
    return;
  }

  const now = new Date();
  const options = { exportedAt: now, engine: state.engineInfo };
  const contents =
    format === 'json'
      ? serializeGameFile(buildGameFile(board, options))
      : buildPgn(board, options);
  const type = format === 'json' ? 'application/json' : 'application/x-chess-pgn';
  downloadText(contents, gameFileName(now, board.move_number, format), type);

  const plies = board.move_log.length;
  state.announce(
    `Game exported as ${format.toUpperCase()} (${plies} ${plies === 1 ? 'ply' : 'plies'})`,
    'success',
  );
}

$('btn-export-game').addEventListener('click', () => exportGame('json'));
$('btn-export-game-pgn').addEventListener('click', () => exportGame('pgn'));

// ── Developer tools ───────────────────────────────────────────────

/** Toggle the developer-tools panel (keyboard: D). */
const devPanel = document.querySelector('.dev-tools') as HTMLElement | null;
let devVisible = false;

function toggleDevTools(): void {
  if (!devPanel) return;
  devVisible = !devVisible;
  devPanel.hidden = !devVisible;
  state.announce(devVisible ? 'Developer tools shown' : 'Developer tools hidden', 'info');
  if (devVisible) void refreshDiagnostics();
}

let logFilter: 'error' | 'warn' | 'info' = 'info';
let autoRefresh = false;
let autoTimer: number | null = null;
let lastLogs: Awaited<ReturnType<typeof fetchLogs>> | null = null;

/** Draw nps / score / hashfull over the last searches on a canvas. */
function drawSearchChart(): void {
  const canvas = document.getElementById('dev-search-chart') as HTMLCanvasElement | null;
  const out = document.getElementById('dev-search-stats');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  const { width: w, height: h } = canvas;
  ctx.clearRect(0, 0, w, h);
  const history = state.searchHistory;
  const last = history.at(-1);
  if (out) out.textContent = last ? `${history.length} searches · last ${describeSearch(last)}` : 'No searches yet';
  if (history.length < 2) return;
  const colors = ['#38bdf8', '#4ade80', '#fbbf24']; // score, nps, TT fill
  const series = telemetrySeries(history);
  ctx.lineWidth = 1.5;
  series.forEach((values, s) => {
    ctx.strokeStyle = colors[s]!;
    ctx.beginPath();
    values.forEach((v, i) => {
      const x = (i / Math.max(1, values.length - 1)) * (w - 4) + 2;
      const y = h - 3 - ((v + 1) / 2) * (h - 6);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();
  });
  ctx.fillStyle = '#64748b';
  ctx.font = '8px sans-serif';
  ctx.fillText('score', 4, 10);
  ctx.fillText('nps', 40, 10);
  ctx.fillText('TT', 68, 10);
}

/** Render the cached logs with the active level filter. */
function renderLogs(): void {
  const el = document.getElementById('dev-logs');
  if (!el || !lastLogs) return;
  const rank = { error: 3, warn: 2, info: 1, debug: 0 };
  const shown = lastLogs.entries.filter((e) => rank[e['level']] >= rank[logFilter]);
  el.textContent = shown.length
    ? shown
        .map(
          (e) =>
            `${new Date(e.ts_ms).toLocaleTimeString()} ${e.level.toUpperCase().padEnd(5)} ` +
            `[${e.request_id}] ${e.target}: ${e.message}`,
        )
        .join('\n')
    : `No ${logFilter}+ entries (${lastLogs.total} buffered)`;
}

// Keep the telemetry chart live while diagnostics are open.
state.subscribe(() => {
  if (devVisible) drawSearchChart();
});

/** Load the server's public config and the most recent structured logs. */
async function refreshDiagnostics(): Promise<void> {
  const configEl = document.getElementById('dev-config');
  try {
    const [config, logs, gpu] = await Promise.all([
      fetchConfig(),
      fetchLogs(50),
      fetchGpu(),
    ]);
    const gpuEl = document.getElementById('dev-gpu');
    if (gpuEl) {
      gpuEl.textContent = gpu.detect_enabled
        ? `GPU: ${gpu.adapters.length} adapter(s) · ${gpu.adapters.map((a) => `${a.name} (${a.backend})`).join(', ') || 'none'}`
        : 'GPU: detection off — rebuild with --features gpu-detect';
      gpuEl.title = gpu.report;
    }
    if (configEl) {
      configEl.textContent =
        `v${config.version} · ${config.sessions ? 'sessions' : 'shared'} (${config.anon_policy}) · ` +
        `body ≤ ${(config.max_body_bytes / 1024).toFixed(0)} KiB · ` +
        `search ≤ ${config.max_concurrent_searches} · ` +
        `log ring ${config.log_capacity}${config.log_file_enabled ? ' + file' : ''}`;
    }
    lastLogs = logs;
    renderLogs();
    drawSearchChart();
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    const el = document.getElementById('dev-logs');
    if (el) el.textContent = `Diagnostics unavailable: ${message}`;
  }
}

/** Map each dev button to the make/Cargo target it runs. */
const devTargets: Record<string, string> = {
  'dev-nnue-pack': 'make nnue-pack   (zstd-compress nets/tk.tkn + sha256)',
  'dev-nnue-parity': 'make nnue-parity   (dump positions + parity_gate.py)',
  'dev-version-race': 'make version-race   (play current vs older commit)',
  'dev-strength': 'make strength   (Elo ladder vs depth d-1)',
  'dev-coverage': 'make coverage   (dataset coverage analyzer)',
  'dev-calibration': 'make calibration   (bucket calibration analyzer)',
};

document
  .getElementById('dev-refresh-logs')
  ?.addEventListener('click', () => void refreshDiagnostics());
document.getElementById('dev-export-logs')?.addEventListener('click', () => {
  const blob = new Blob([JSON.stringify(lastLogs, null, 2)], { type: 'application/json' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = `taikyoku-logs-${Date.now()}.json`;
  a.click();
  URL.revokeObjectURL(a.href);
  state.announce('Logs exported', 'success');
});
document.getElementById('dev-toggle-auto')?.addEventListener('click', (event) => {
  autoRefresh = !autoRefresh;
  const btn = event.currentTarget as HTMLButtonElement;
  btn.textContent = `Auto: ${autoRefresh ? 'on' : 'off'}`;
  btn.setAttribute('aria-pressed', String(autoRefresh));
  if (autoRefresh) {
    autoTimer = window.setInterval(() => void refreshDiagnostics(), 5000);
  } else if (autoTimer !== null) {
    window.clearInterval(autoTimer);
    autoTimer = null;
  }
});
document.querySelectorAll<HTMLButtonElement>('.dev-filter button').forEach((btn) => {
  btn.addEventListener('click', () => {
    logFilter = btn.dataset['level'] as typeof logFilter;
    document.querySelectorAll('.dev-filter button').forEach((b) => b.setAttribute('aria-pressed', 'false'));
    btn.setAttribute('aria-pressed', 'true');
    renderLogs();
  });
});
document.getElementById('dev-copy-config')?.addEventListener('click', () => {
  void fetchConfig().then((cfg) => {
    void navigator.clipboard.writeText(JSON.stringify(cfg, null, 2)).then(() => {
      state.announce('Server config copied', 'success');
    });
  });
});

Object.entries(devTargets).forEach(([id, cmd]) => {
  const btn = document.getElementById(id) as HTMLButtonElement | null;
  if (!btn) return;
  btn.addEventListener('click', () => {
    navigator.clipboard.writeText(cmd).then(() => {
      state.announce(`Copied: ${cmd}`, 'success');
    });
  });
});

// ── Keyboard shortcuts ────────────────────────────────────────────

document.addEventListener('keydown', (event) => {
  const target = event.target as HTMLElement | null;
  if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT')) {
    return;
  }
  if (event.ctrlKey || event.metaKey || event.altKey) return;

  switch (event.key.toLowerCase()) {
    case 'n':
      readSettingsFromControls();
      state.setAutoPlay(false);
      void state.reset();
      break;
    case 'u':
      void state.undo(1);
      break;
    case 'a':
      readSettingsFromControls();
      state.setAutoPlay(!state.settings.autoPlay);
      break;
    case 'k':
      state.toggleKanji();
      break;
    case 'f':
      state.toggleFlip();
      break;
    case ' ':
      event.preventDefault();
      void state.aiMove();
      break;
    case '+':
    case '=':
      state.setZoom(Math.min(MAX_ZOOM, state.settings.zoom + 0.1));
      syncControlsFromSettings();
      break;
    case '-':
      state.setZoom(Math.max(MIN_ZOOM, state.settings.zoom - 0.1));
      syncControlsFromSettings();
      break;
    case 'd':
      toggleDevTools();
      break;
    default:
      break;
  }
});

// ── Go ────────────────────────────────────────────────────────────

syncControlsFromSettings();
void state.init().catch((err: unknown) => {
  const status = document.getElementById('status');
  if (status) {
    status.textContent = `⚠️ Error: ${err instanceof Error ? err.message : String(err)} — is the Rust backend running on :8000?`;
  }
  console.error('Failed to initialize game:', err);
});
