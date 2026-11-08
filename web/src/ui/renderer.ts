/**
 * Canvas-based board renderer.
 *
 * Draws the 36×36 board, pieces, highlights and overlay indicators. Supports
 * zooming (cell size scale) and flipping the board 180°.
 */

import type { BoardState, CellData, MoveData, Square } from '../types/board';
import { pieceKanji } from '../data/kanji';
import { BOARD_SIZE, toViewSquare } from '../game/view';

// ── Layout ────────────────────────────────────────────────────────
const BASE_CELL = 28; // px per square at zoom 1
const BASE_COORD = 18; // px for coordinate margins at zoom 1

export interface Layout {
  cell: number;
  coord: number;
  boardPx: number;
  width: number;
}

/** Compute the canvas geometry for a zoom factor. */
export function layoutFor(zoom: number): Layout {
  const cell = Math.max(10, Math.round(BASE_CELL * zoom));
  const coord = Math.max(14, Math.round(BASE_COORD * zoom));
  const boardPx = cell * BOARD_SIZE;
  return { cell, coord, boardPx, width: coord * 2 + boardPx };
}

// ── Colours (tuned for dark theme) ────────────────────────────────
const LIGHT = '#c8b07a';
const DARK = '#a68a5b';

const BG = '#0a0a1a';
const SEL = '#ffd700';
const LEGAL_EMPTY = 'rgba(92,184,92,0.55)';
const LEGAL_CAPT = 'rgba(217,83,79,0.55)';
const LAST_MOVE = 'rgba(74,144,217,0.45)';
const HOVER = 'rgba(255,215,0,0.20)';
const INSPECT = '#4ad9d9';

const BLACK_PIECE = '#1a0505';
const WHITE_PIECE = '#f5f5ff';
const ROYAL_BLACK = '#c8a030';
const ROYAL_WHITE = '#3050c8';
const UNDERLINE = '#ccccff';
const COORD_COLOR = '#888';

// ── Drawing primitives ────────────────────────────────────────────

export interface RenderOptions {
  state: BoardState;
  selectedSq: Square | null;
  inspectSq: Square | null;
  legalMoves: MoveData[];
  hoverSq: Square | null;
  lastFrom: Square | null;
  lastTo: Square | null;
  useKanji: boolean;
  flip: boolean;
  zoom: number;
}

export function render(ctx: CanvasRenderingContext2D, opts: RenderOptions): void {
  const { state, selectedSq, inspectSq, legalMoves, hoverSq, lastFrom, lastTo, useKanji, flip } = opts;

  const layout = layoutFor(opts.zoom);
  const { cell, coord, boardPx, width } = layout;
  const W = width;
  const H = width;

  const cellX = (viewCol: number) => coord + viewCol * cell;
  const cellY = (viewRow: number) => coord + viewRow * cell;
  const view = (sq: Square | null): Square | null => (sq ? toViewSquare(sq, flip) : null);

  // ── Background ──────────────────────────────────────────────
  ctx.fillStyle = BG;
  ctx.fillRect(0, 0, W, H);

  // ── Guard: no board state ──────────────────────────────────
  if (!state.board) {
    ctx.fillStyle = '#999';
    ctx.font = '16px sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('Loading...', W / 2, H / 2);
    return;
  }

  const lastFromView = view(lastFrom);
  const lastToView = view(lastTo);
  const selectedView = view(selectedSq);
  const inspectView = view(inspectSq);
  const hoverView = view(hoverSq);

  // ── Board cells ────────────────────────────────────────────
  for (let vr = 0; vr < BOARD_SIZE; vr++) {
    for (let vc = 0; vc < BOARD_SIZE; vc++) {
      const x = cellX(vc);
      const y = cellY(vr);
      ctx.fillStyle = (vr + vc) % 2 === 0 ? LIGHT : DARK;
      ctx.fillRect(x, y, cell, cell);

      if (
        (lastFromView && lastFromView[0] === vr && lastFromView[1] === vc) ||
        (lastToView && lastToView[0] === vr && lastToView[1] === vc)
      ) {
        ctx.fillStyle = LAST_MOVE;
        ctx.fillRect(x, y, cell, cell);
      }
    }
  }

  // ── Legal-move indicators (accept both board- and view-space moves) ──
  for (const m of legalMoves) {
    const [tr, tc] = m.to;
    // Internal callers pass board coordinates; render via toViewSquare.
    const [vr, vc] = toViewSquare([tr, tc], flip);
    const x = cellX(vc);
    const y = cellY(vr);
    const isCapture = state.board[tr]?.[tc] != null;

    if (isCapture) {
      ctx.fillStyle = LEGAL_CAPT;
      ctx.fillRect(x, y, cell, cell);
    } else {
      ctx.fillStyle = LEGAL_EMPTY;
      ctx.beginPath();
      ctx.arc(x + cell / 2, y + cell / 2, Math.max(3, cell * 0.18), 0, Math.PI * 2);
      ctx.fill();
    }
  }

  // ── Pieces ─────────────────────────────────────────────────
  const symbolFont = Math.max(7, Math.round(cell * 0.34));
  for (let br = 0; br < BOARD_SIZE; br++) {
    for (let bc = 0; bc < BOARD_SIZE; bc++) {
      const piece: CellData | null = state.board[br][bc];
      if (!piece) continue;

      const [vr, vc] = toViewSquare([br, bc], flip);
      const x = cellX(vc);
      const y = cellY(vr);

      // Selection highlight
      if (selectedView && selectedView[0] === vr && selectedView[1] === vc) {
        ctx.fillStyle = SEL;
        ctx.fillRect(x + 2, y + 2, cell - 4, cell - 4);
      }

      // Inspector outline
      if (inspectView && inspectView[0] === vr && inspectView[1] === vc) {
        ctx.strokeStyle = INSPECT;
        ctx.lineWidth = 2;
        ctx.strokeRect(x + 1, y + 1, cell - 2, cell - 2);
      }

      // Hover
      if (hoverView && hoverView[0] === vr && hoverView[1] === vc && selectedSq === null) {
        ctx.fillStyle = HOVER;
        ctx.fillRect(x, y, cell, cell);
      }

      // Royal glow
      if (piece.is_royal) {
        ctx.fillStyle = piece.color === 0 ? ROYAL_BLACK : ROYAL_WHITE;
        ctx.fillRect(x + 2, y + 2, cell - 4, cell - 4);
      }

      // Piece symbol: kanji or English abbreviation
      const symbol = useKanji ? pieceKanji(piece.piece) : piece.piece;
      ctx.font = `bold ${symbolFont}px sans-serif`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillStyle = piece.color === 0 ? BLACK_PIECE : WHITE_PIECE;
      ctx.fillText(symbol, x + cell / 2, y + cell / 2);

      // White underline
      if (piece.color === 1) {
        ctx.strokeStyle = UNDERLINE;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(x + 5, y + cell - 4);
        ctx.lineTo(x + cell - 5, y + cell - 4);
        ctx.stroke();
      }
    }
  }

  // ── Coordinates ────────────────────────────────────────────
  ctx.fillStyle = COORD_COLOR;
  ctx.font = `${Math.max(8, Math.round(9 * (cell / BASE_CELL)))}px monospace`;
  ctx.textBaseline = 'middle';

  for (let i = 0; i < BOARD_SIZE; i++) {
    const label = String(flip ? i + 1 : BOARD_SIZE - i);
    const x = cellX(i) + cell / 2;
    const y = cellY(i) + cell / 2;

    ctx.textAlign = 'right';
    ctx.fillText(label, coord - 4, y);
    ctx.textAlign = 'left';
    ctx.fillText(label, coord + boardPx + 4, y);

    ctx.textAlign = 'center';
    ctx.textBaseline = 'bottom';
    ctx.fillText(label, x, coord - 4);
    ctx.textBaseline = 'top';
    ctx.fillText(label, x, coord + boardPx + 4);
    ctx.textBaseline = 'middle';
  }

  // ── Game-over overlay ──────────────────────────────────────
  if (state.game_over) {
    ctx.fillStyle = 'rgba(10,10,26,0.55)';
    ctx.fillRect(coord, coord + boardPx / 2 - 30, boardPx, 60);
    ctx.fillStyle = '#ffd700';
    ctx.font = `bold ${Math.round(cell * 0.7)}px sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    const label = state.game_result
      ? state.game_result.replace('_', ' ').toUpperCase()
      : 'GAME OVER';
    ctx.fillText(label, W / 2, coord + boardPx / 2);
  }
}
