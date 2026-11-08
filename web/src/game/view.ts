/** Pure view helpers (board orientation, scaling, formatting).
 *
 * Kept free of DOM/canvas access so they can be unit-tested with vitest.
 */

import type { PieceColor, Square } from '../types/board';

export const BOARD_SIZE = 36;

export const MIN_ZOOM = 0.7;
export const MAX_ZOOM = 1.8;
export const DEFAULT_ZOOM = 1;

/** Map a board square to its on-screen square (flip = 180° rotation). */
export function toViewSquare([r, c]: Square, flip: boolean): Square {
  return flip ? [BOARD_SIZE - 1 - r, BOARD_SIZE - 1 - c] : [r, c];
}

/** Map an on-screen square back to the engine's board coordinates. */
export function fromViewSquare([vr, vc]: Square, flip: boolean): Square {
  return flip ? [BOARD_SIZE - 1 - vr, BOARD_SIZE - 1 - vc] : [vr, vc];
}

export function isOnBoard(r: number, c: number): boolean {
  return Number.isInteger(r) && Number.isInteger(c) && r >= 0 && c >= 0 && r < BOARD_SIZE && c < BOARD_SIZE;
}

export function clampZoom(zoom: number): number {
  if (!Number.isFinite(zoom)) return DEFAULT_ZOOM;
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
}

export function sideName(color: PieceColor | number): string {
  return color === 0 ? 'Black' : 'White';
}

export function opponent(color: PieceColor): PieceColor {
  return color === 0 ? 1 : 0;
}

/** Human-readable text for the `game_result` field. */
export function describeResult(result: string | null): string {
  switch (result) {
    case 'black_wins':
      return 'Black wins';
    case 'white_wins':
      return 'White wins';
    case 'draw':
      return 'Draw';
    default:
      return '';
  }
}

/** "+12" / "-7" / "0". */
export function formatScore(score: number): string {
  return score > 0 ? `+${score}` : String(score);
}

/** Compact node counts: 12, 4.3k, 1.2M. */
export function formatNodes(nodes: number): string {
  if (nodes >= 1_000_000) return `${(nodes / 1_000_000).toFixed(1)}M`;
  if (nodes >= 1_000) return `${(nodes / 1_000).toFixed(1)}k`;
  return String(nodes);
}

/** Convert the material score history into chart coordinates. */
export function chartPoints(
  values: number[],
  width: number,
  height: number,
  padding = 3,
): Array<{ x: number; y: number }> {
  if (values.length === 0) return [];
  const maxAbs = Math.max(1, ...values.map((v) => Math.abs(v)));
  const usableW = Math.max(1, width - padding * 2);
  const usableH = Math.max(1, height - padding * 2);
  const step = values.length > 1 ? usableW / (values.length - 1) : 0;
  return values.map((v, i) => ({
    x: padding + i * step,
    y: padding + usableH / 2 - (v / maxAbs) * (usableH / 2),
  }));
}
