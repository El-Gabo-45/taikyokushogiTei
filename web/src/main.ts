/**
 * Application entry point.
 *
 * Wires the game state, canvas renderer, input controller and DOM panels
 * together, then boots the first game. All UI actions are also available as
 * keyboard shortcuts:
 *
 *   N = new game · U = undo · A = auto-play · K = kanji/English
 *   F = flip board · +/- = zoom · Space = ask the AI for a move
 */

import { GameState, type Mode } from './game/state';
import { buildGameFile, gameFileName, serializeGameFile } from './game/export';
import { Controller } from './ui/controller';
import { updatePanels } from './ui/panel';
import { MAX_ZOOM, MIN_ZOOM } from './game/view';

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
  // Toggling Auto only starts the loop; it must not play for the human.
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
  const blob = new Blob([tsfenBox.value], { type: 'text/plain' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = `taikyoku-move-${state.board?.move_number ?? 1}.tsfen`;
  link.click();
  URL.revokeObjectURL(url);
  state.announce('TSFEN downloaded', 'success');
});

// ── Game export ───────────────────────────────────────────────────

$('btn-export-game').addEventListener('click', () => {
  const board = state.board;
  if (!board) {
    state.announce('No game to export yet', 'error');
    return;
  }

  const now = new Date();
  const file = buildGameFile(board, { exportedAt: now, engine: state.engineInfo });
  const blob = new Blob([serializeGameFile(file)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = gameFileName(now, board.move_number);
  link.click();
  URL.revokeObjectURL(url);

  const plies = board.move_log.length;
  state.announce(`Game exported (${plies} ${plies === 1 ? 'ply' : 'plies'})`, 'success');
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
