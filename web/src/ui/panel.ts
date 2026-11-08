/**
 * Updates the sidebar DOM panels from the game state.
 *
 * Everything is written with `textContent` (never HTML injection) and the
 * move log/score chart are only rebuilt when their contents change.
 */

import type { GameState } from '../game/state';
import { chartPoints, describeResult, formatNodes, formatScore, sideName } from '../game/view';

const $ = <T extends HTMLElement = HTMLElement>(id: string): T | null =>
  document.getElementById(id) as T | null;

function setText(id: string, text: string): void {
  const el = $(id);
  if (el && el.textContent !== text) el.textContent = text;
}

function setDisabled(id: string, disabled: boolean): void {
  const el = $<HTMLButtonElement>(id);
  if (el) el.disabled = disabled;
}

let lastLogSignature = '';

export function updatePanels(state: GameState): void {
  const board = state.board;
  if (!board) return;

  // ── Info section ──────────────────────────────────────────
  setText('info-turn', sideName(board.side_to_move));
  setText('info-move', String(board.move_number));
  setText('info-black', String(board.black_pieces));
  setText('info-white', String(board.white_pieces));
  setText('info-score', formatScore(board.material_score));
  setText('info-eval', formatScore(board.eval_score));
  setText('info-progress', `${board.no_progress_plies} / 1000`);

  const statusMsg = $('status-msg');
  if (statusMsg) {
    const fallback = board.game_result
      ? describeResult(board.game_result)
      : board.game_over
        ? 'Game over'
        : '—';
    const message = state.message?.text ?? fallback;
    const kind = state.message?.kind ?? (board.game_over ? 'success' : 'info');
    statusMsg.textContent = message;
    statusMsg.className = `status-msg ${kind}`;
  }

  setText(
    'status',
    `${sideName(board.side_to_move)} to move · ${board.black_pieces} vs ${board.white_pieces} pieces · move ${board.move_number}`,
  );
  setText('busy', state.busy ? '● thinking…' : '');

  // ── Engine panel ──────────────────────────────────────────
  const search = state.lastSearch;
  setText('search-depth', search ? `d${search.depth}` : '—');
  setText('search-score', search ? formatScore(search.score) : '—');
  setText('search-nodes', search ? formatNodes(search.nodes) : '—');
  setText('search-time', search ? `${search.time_ms} ms` : '—');

  const engine = state.engineInfo;
  setText('engine-version', engine ? `v${engine.version}` : '—');
  setText('engine-backend', engine ? engine.eval_backend : '—');
  setText('engine-hash', engine ? `${engine.hash_mb} MB` : '—');
  setText('engine-pieces', engine ? `${engine.piece_types} types` : '—');
  setText(
    'engine-techniques',
    engine ? `${engine.search_techniques.length} enabled` : '—',
  );

  // ── Piece inspector ───────────────────────────────────────
  updateInspector(state);

  // ── Move log ──────────────────────────────────────────────
  const signature = `${board.move_log.length}:${board.move_log.at(-1) ?? ''}`;
  if (signature !== lastLogSignature) {
    lastLogSignature = signature;
    const log = $('move-log');
    if (log) {
      const fragment = document.createDocumentFragment();
      for (const entry of board.move_log) {
        const row = document.createElement('div');
        row.className = 'entry';
        row.textContent = entry;
        fragment.appendChild(row);
      }
      if (board.move_log.length === 0) {
        const empty = document.createElement('div');
        empty.className = 'entry empty';
        empty.textContent = 'No moves yet';
        fragment.appendChild(empty);
      }
      log.replaceChildren(fragment);
      log.scrollTop = log.scrollHeight;
    }
  }

  // ── Score chart ───────────────────────────────────────────
  drawScoreChart(board.score_history);

  // ── Toolbar state ─────────────────────────────────────────
  const autoBtn = $('btn-auto');
  if (autoBtn) {
    autoBtn.textContent = state.settings.autoPlay ? 'Stop' : 'Auto';
    autoBtn.classList.toggle('active', state.settings.autoPlay);
  }
  const kanjiBtn = $('btn-kanji');
  if (kanjiBtn) kanjiBtn.textContent = state.settings.useKanji ? 'Kanji' : 'English';
  const flipBtn = $('btn-flip');
  if (flipBtn) flipBtn.textContent = state.settings.flip ? 'Flip ↺' : 'Flip';

  setDisabled('btn-undo', board.move_log.length === 0);
  setDisabled('btn-analyze', state.busy);
  setDisabled('btn-ai-move', state.busy || board.game_over);
  setDisabled('btn-load', state.busy);

  // ── Position textarea ─────────────────────────────────────
  const tsfen = $<HTMLTextAreaElement>('tsfen-box');
  if (tsfen && document.activeElement !== tsfen) {
    tsfen.value = board.tsfen;
  }
}

function updateInspector(state: GameState): void {
  const inspection = state.inspection;
  const hint = $('inspect-hint');
  if (!inspection) {
    setText('inspect-title', '—');
    setText('inspect-name', 'Click a piece');
    setText('inspect-abbrev', '');
    setText('inspect-value', '');
    setText('inspect-promotes', '');
    setText('inspect-betza', '');
    setText('inspect-tags', '');
    if (hint) hint.textContent = 'Hover/click any piece to inspect its movement data.';
    return;
  }

  const { piece, info, square } = inspection;
  setText('inspect-title', `(${square[0]}, ${square[1]}) · ${sideName(piece.color)}`);
  setText('inspect-name', info?.name ?? piece.name);
  setText('inspect-abbrev', `[${piece.piece}]`);
  setText('inspect-value', `value ${info?.value ?? piece.value}`);
  setText(
    'inspect-promotes',
    info?.promotes_to ? `promotes to ${info.promotes_to}` : 'no promotion',
  );
  setText('inspect-betza', info?.betza ? `Betza: ${info.betza}` : '');

  const tags: string[] = [];
  if (piece.is_royal || info?.is_royal) tags.push('royal');
  if (info) {
    if (info.slide_directions > 0) tags.push(`${info.slide_directions} slides`);
    if (info.jump_destinations > 0) tags.push(`${info.jump_destinations} jumps`);
    if (info.area_steps > 0) tags.push(`area ×${info.area_steps}`);
    if (info.has_hook) tags.push('hook');
    if (info.has_range_capture) tags.push('range capture');
    if (info.has_igui) tags.push('igui');
  }
  setText('inspect-tags', tags.join(' · '));
  if (hint) hint.textContent = info ? '' : 'Loading movement data…';
}

function drawScoreChart(history: number[]): void {
  const canvas = $<HTMLCanvasElement>('score-chart');
  if (!canvas) return;
  const dpr = Math.max(1, Math.min(2, window.devicePixelRatio || 1));
  const cssWidth = canvas.clientWidth || 220;
  const cssHeight = canvas.clientHeight || 64;
  if (canvas.width !== Math.round(cssWidth * dpr) || canvas.height !== Math.round(cssHeight * dpr)) {
    canvas.width = Math.round(cssWidth * dpr);
    canvas.height = Math.round(cssHeight * dpr);
  }
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, cssWidth, cssHeight);

  // Zero line.
  ctx.strokeStyle = 'rgba(255,255,255,0.15)';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, cssHeight / 2);
  ctx.lineTo(cssWidth, cssHeight / 2);
  ctx.stroke();

  if (history.length === 0) return;
  const points = chartPoints(history, cssWidth, cssHeight, 4);
  ctx.strokeStyle = '#4a90d9';
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  points.forEach((p, i) => (i === 0 ? ctx.moveTo(p.x, p.y) : ctx.lineTo(p.x, p.y)));
  ctx.stroke();

  const last = points[points.length - 1];
  ctx.fillStyle = history[history.length - 1] >= 0 ? '#5cb85c' : '#d9534f';
  ctx.beginPath();
  ctx.arc(last.x, last.y, 2.5, 0, Math.PI * 2);
  ctx.fill();
}
