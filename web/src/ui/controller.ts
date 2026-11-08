/**
 * Input controller.
 *
 * Translates mouse events on the canvas into game actions, triggers
 * re-renders when the game state changes, and offers a modal dialog when
 * one destination square has several legal moves (promotion / igui).
 */

import type { GameState } from '../game/state';
import type { MoveData, Square } from '../types/board';
import { fromViewSquare } from '../game/view';
import { layoutFor, render } from './renderer';
import { openChoiceModal } from './modal';

export class Controller {
  private readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private readonly state: GameState;
  private hoverSq: Square | null = null;
  private rafId = 0;

  constructor(canvas: HTMLCanvasElement, ctx: CanvasRenderingContext2D, state: GameState) {
    this.canvas = canvas;
    this.ctx = ctx;
    this.state = state;

    this.bindEvents();
    state.subscribe(() => this.scheduleDraw());
  }

  // ---------------------------------------------------------------
  // Mouse helpers
  // ---------------------------------------------------------------

  /** Convert a canvas-relative pixel coordinate to a board (row, col). */
  private pixelToSquare(px: number, py: number): Square | null {
    const { cell, coord } = layoutFor(this.state.settings.zoom);
    const vc = Math.floor((px - coord) / cell);
    const vr = Math.floor((py - coord) / cell);
    if (vr < 0 || vr >= 36 || vc < 0 || vc >= 36) return null;
    return fromViewSquare([vr, vc], this.state.settings.flip);
  }

  /** Get mouse position relative to the canvas, accounting for CSS scaling. */
  private mousePos(e: MouseEvent): { px: number; py: number } {
    const rect = this.canvas.getBoundingClientRect();
    const sx = this.canvas.width / rect.width;
    const sy = this.canvas.height / rect.height;
    return {
      px: (e.clientX - rect.left) * sx,
      py: (e.clientY - rect.top) * sy,
    };
  }

  // ---------------------------------------------------------------
  // Event binding
  // ---------------------------------------------------------------

  private bindEvents(): void {
    this.canvas.addEventListener('mousemove', (e) => this.onMouseMove(e));
    this.canvas.addEventListener('mouseleave', () => this.onMouseLeave());
    this.canvas.addEventListener('click', (e) => void this.onClick(e));
  }

  private onMouseMove(e: MouseEvent): void {
    const { px, py } = this.mousePos(e);
    const sq = this.pixelToSquare(px, py);

    if (sq) {
      if (!this.hoverSq || this.hoverSq[0] !== sq[0] || this.hoverSq[1] !== sq[1]) {
        this.hoverSq = sq;
        this.canvas.style.cursor = this.cursorFor(sq);
        this.scheduleDraw();
      }
    } else if (this.hoverSq) {
      this.hoverSq = null;
      this.canvas.style.cursor = 'default';
      this.scheduleDraw();
    }
  }

  private cursorFor([r, c]: Square): string {
    const board = this.state.board;
    if (!board) return 'default';
    if (this.state.selectedSq) {
      if (this.state.legalMoves.some((m) => m.to[0] === r && m.to[1] === c)) return 'pointer';
    }
    const cell = board.board[r]?.[c];
    return cell ? 'pointer' : 'default';
  }

  private onMouseLeave(): void {
    if (this.hoverSq) {
      this.hoverSq = null;
      this.scheduleDraw();
    }
  }

  private async onClick(e: MouseEvent): Promise<void> {
    const { px, py } = this.mousePos(e);
    const sq = this.pixelToSquare(px, py);
    if (!sq) return;

    const [r, c] = sq;
    const board = this.state.board;
    if (!board) return;

    // If a piece is selected and we click a legal target → play.
    if (this.state.selectedSq) {
      const matches = this.state.legalMoves.filter((m) => m.to[0] === r && m.to[1] === c);
      if (matches.length > 0) {
        await this.handleTarget(matches);
        return;
      }
      // Clicked a different own piece: switch selection.
      const clicked = board.board[r]?.[c] ?? null;
      if (clicked && clicked.color === board.side_to_move && !board.game_over) {
        this.state.clearSelection();
        await this.state.select(r, c);
        void this.state.inspect(r, c);
        return;
      }
      // Clicked elsewhere: drop selection but inspect the piece if any.
      this.state.clearSelection();
      if (clicked) void this.state.inspect(r, c);
      else this.state.clearInspection();
      return;
    }

    // No selection: select own piece, inspect any piece.
    const cell = board.board[r]?.[c] ?? null;
    if (!cell) {
      this.state.clearInspection();
      return;
    }
    if (cell.color === board.side_to_move && !board.game_over) {
      await this.state.select(r, c);
    }
    void this.state.inspect(r, c);
  }

  /** Apply a move—asking the user only when several moves are possible. */
  private async handleTarget(matches: MoveData[]): Promise<void> {
    const board = this.state.board;
    if (!board || !this.state.selectedSq) return;
    const [fr, fc] = this.state.selectedSq;

    let target: MoveData | null = matches[0];
    if (matches.length > 1) {
      const choices = matches.map((m, i) => ({
        label: describeMove(m, i),
        detail: [
          m.promotion ? 'promotes' : null,
          m.is_igui ? 'igui (capture in place)' : null,
          m.captured ? `captures ${m.captured}` : null,
          m.mid_sq != null ? `mid_sq=${m.mid_sq}` : null,
        ]
          .filter(Boolean)
          .join(' · '),
        value: m,
      }));
      target = await openChoiceModal(
        'Choose a move',
        `${matches.length} legal moves reach this square.`,
        choices,
      );
    }
    if (!target) return;

    await this.state.play(fr, fc, target.to[0], target.to[1], target.promotion, target.mid_sq ?? null);
  }

  // ---------------------------------------------------------------
  // Rendering loop
  // ---------------------------------------------------------------

  private scheduleDraw(): void {
    if (this.rafId) return;
    this.rafId = requestAnimationFrame(() => {
      this.rafId = 0;
      this.draw();
    });
  }

  private draw(): void {
    const board = this.state.board;
    if (!board) return;

    const { width } = layoutFor(this.state.settings.zoom);
    if (this.canvas.width !== width || this.canvas.height !== width) {
      this.canvas.width = width;
      this.canvas.height = width;
    }

    render(this.ctx, {
      state: board,
      selectedSq: this.state.selectedSq,
      inspectSq: this.state.inspection?.square ?? null,
      legalMoves: this.state.legalMoves,
      hoverSq: this.hoverSq,
      lastFrom: this.state.lastMoveFrom,
      lastTo: this.state.lastMoveTo,
      useKanji: this.state.settings.useKanji,
      flip: this.state.settings.flip,
      zoom: this.state.settings.zoom,
    });
  }
}

function describeMove(m: MoveData, index: number): string {
  const dest = `(${m.to[0]}, ${m.to[1]})`;
  const suffix = m.promotion ? ' +' : '';
  return `${index + 1}. Move to ${dest}${suffix}`;
}
