/**
 * Game state machine.
 *
 * Owns the authoritative board state and exposes actions that synchronise
 * with the backend and update the UI subscription. All network errors are
 * converted into a `message` shown by the panels instead of throwing into
 * the void.
 */

import type {
  BoardState,
  CellData,
  EngineInfo,
  EvalInfo,
  MoveData,
  PieceInfo,
  SaveInfo,
  SearchStats,
  Square,
} from '../types/board';
import * as api from '../api/client';
import { clampZoom, DEFAULT_ZOOM, opponent } from './view';

export type Mode = 'human_vs_random' | 'human_vs_ai' | 'random_vs_random' | 'ai_vs_ai';

export type Listener = () => void;

/** Runtime settings that affect AI auto-play behaviour and board display. */
export interface Settings {
  mode: Mode;
  humanColor: 0 | 1;
  aiDepth: number;
  /** Show kanji (Japanese) piece symbols instead of English abbreviations. */
  useKanji: boolean;
  /** Rotate the board 180° (Black at the top). */
  flip: boolean;
  /** Canvas scale factor (0.7 – 1.8). */
  zoom: number;
  /** Whether the AI loop keeps playing automatically. */
  autoPlay: boolean;
}

export interface Inspection {
  square: Square;
  piece: CellData;
  info: PieceInfo | null;
}

export interface GameMessage {
  text: string;
  kind: 'info' | 'success' | 'error';
}

const AI_DELAY_MS = 120;

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

export class GameState {
  board: BoardState | null = null;
  selectedSq: Square | null = null;
  legalMoves: MoveData[] = [];
  lastMoveFrom: Square | null = null;
  lastMoveTo: Square | null = null;

  settings: Settings = {
    mode: 'human_vs_ai',
    humanColor: 0,
    aiDepth: 2,
    useKanji: true,
    flip: false,
    zoom: DEFAULT_ZOOM,
    autoPlay: false,
  };

  /** Engine metadata, fetched once at startup. */
  engineInfo: EngineInfo | null = null;
  /** Latest static evaluation (embedded in every board snapshot). */
  evalInfo: EvalInfo | null = null;
  /** Telemetry of the most recent search. */
  lastSearch: SearchStats | null = null;
  /** Piece the user clicked, with database details. */
  inspection: Inspection | null = null;
  /** Transient user-facing message. */
  message: GameMessage | null = null;
  /** True while a network request is in flight. */
  busy = false;

  private readonly listeners = new Set<Listener>();
  private readonly pieceCache = new Map<string, PieceInfo>();
  private aiPending = false;
  private autoLoopActive = false;

  // ----------------------------------------------------------------
  // Subscriptions
  // ----------------------------------------------------------------

  subscribe(fn: Listener): () => void {
    this.listeners.add(fn);
    return () => {
      this.listeners.delete(fn);
    };
  }

  private notify(): void {
    for (const fn of this.listeners) fn();
  }

  private setMessage(text: string, kind: GameMessage['kind'] = 'info'): void {
    this.message = { text, kind };
  }

  /** Public helper so UI actions (copy, download…) can surface feedback. */
  announce(text: string, kind: GameMessage['kind'] = 'info'): void {
    this.setMessage(text, kind);
    this.notify();
  }

  /** Enable/disable the auto-play loop and kick it when enabling. */
  setAutoPlay(on: boolean): void {
    this.settings.autoPlay = on;
    this.notify();
    if (on) this.kickAutoLoop();
  }

  /** Update derived eval info from a fresh board snapshot. */
  private syncEval(): void {
    if (!this.board) return;
    this.evalInfo = {
      eval_score: this.board.eval_score,
      material_score: this.board.material_score,
      black_pieces: this.board.black_pieces,
      white_pieces: this.board.white_pieces,
      side_to_move: this.board.side_to_move,
    };
  }

  private applyBoard(board: BoardState): void {
    this.board = board;
    this.inspection = null;
    this.syncEval();
    if (board.game_over) {
      this.settings.autoPlay = false;
      const result = board.game_result ? board.game_result.replace('_', ' ') : 'game over';
      this.setMessage(`Game over — ${result}`, 'success');
    }
    this.notify();
  }

  // ----------------------------------------------------------------
  // API-driven actions
  // ----------------------------------------------------------------

  /** Initialise or reload the board from the backend. */
  async init(): Promise<void> {
    this.busy = true;
    this.notify();
    try {
      const [board, engine] = await Promise.all([
        api.fetchState(),
        api.fetchEngineInfo().catch(() => null),
      ]);
      if (engine) this.engineInfo = engine;
      this.selectedSq = null;
      this.legalMoves = [];
      this.lastMoveFrom = null;
      this.lastMoveTo = null;
      this.lastSearch = null;
      this.applyBoard(board);
      this.setMessage('Ready', 'info');
    } catch (err) {
      this.setMessage(errorText(err), 'error');
      this.notify();
      throw err;
    } finally {
      this.busy = false;
      this.notify();
    }
    this.kickAutoLoop();
  }

  /** Start a new game. */
  async reset(): Promise<void> {
    this.busy = true;
    this.notify();
    try {
      await api.newGame();
      this.selectedSq = null;
      this.legalMoves = [];
      this.lastMoveFrom = null;
      this.lastMoveTo = null;
      this.lastSearch = null;
      const board = await api.fetchState();
      this.applyBoard(board);
      this.setMessage('New game', 'success');
    } catch (err) {
      this.setMessage(errorText(err), 'error');
    } finally {
      this.busy = false;
      this.notify();
    }
    this.kickAutoLoop();
  }

  /** Toggle piece symbol mode between kanji and English abbreviations. */
  toggleKanji(): void {
    this.settings.useKanji = !this.settings.useKanji;
    this.notify();
  }

  /** Rotate the board 180°. */
  toggleFlip(): void {
    this.settings.flip = !this.settings.flip;
    this.notify();
  }

  setZoom(zoom: number): void {
    this.settings.zoom = clampZoom(zoom);
    this.notify();
  }

  /** Undo the last `plies` moves. */
  async undo(plies = 1): Promise<void> {
    if (!this.board) return;
    this.busy = true;
    this.notify();
    try {
      const res = await api.undo(plies);
      if (res.ok && res.board) {
        this.lastMoveFrom = null;
        this.lastMoveTo = null;
        this.lastSearch = null;
        this.applyBoard(res.board);
        this.setMessage(`Undid ${res.undone_plies} move(s)`, 'success');
        this.settings.autoPlay = false;
      } else {
        this.setMessage(res.error ?? 'Nothing to undo', 'error');
      }
    } catch (err) {
      this.setMessage(errorText(err), 'error');
    } finally {
      this.busy = false;
      this.notify();
    }
  }

  /** Select a square and fetch its legal moves. */
  async select(r: number, c: number): Promise<void> {
    if (!this.board) return;
    this.selectedSq = [r, c];
    this.notify();
    try {
      const resp = await api.fetchMoves(r, c);
      // Ignore a response that arrived after the selection changed.
      if (this.selectedSq?.[0] !== r || this.selectedSq?.[1] !== c) return;
      this.legalMoves = resp.moves;
    } catch (err) {
      this.legalMoves = [];
      this.setMessage(errorText(err), 'error');
    }
    this.notify();
  }

  clearSelection(): void {
    this.selectedSq = null;
    this.legalMoves = [];
    this.notify();
  }

  /**
   * Inspect the piece on a square: shows name/value/movement data in the
   * sidebar regardless of which side is to move.
   */
  async inspect(r: number, c: number): Promise<void> {
    const piece = this.board?.board[r]?.[c] ?? null;
    if (!piece) {
      this.clearInspection();
      return;
    }
    const cached = this.pieceCache.get(piece.piece) ?? null;
    this.inspection = { square: [r, c], piece, info: cached };
    this.notify();

    if (cached) return;
    try {
      const info = await api.pieceInfo(piece.piece);
      this.pieceCache.set(piece.piece, info);
      // Only update if the user is still inspecting the same piece.
      if (this.inspection?.piece.piece === piece.piece) {
        this.inspection = { ...this.inspection, info };
      }
    } catch (err) {
      if (err instanceof api.ApiError && err.status === 404) {
        // Unknown piece abbrev in the database — keep the board data only.
      } else {
        this.setMessage(errorText(err), 'error');
      }
    }
    this.notify();
  }

  clearInspection(): void {
    if (!this.inspection) return;
    this.inspection = null;
    this.notify();
  }

  /** Execute a human move. */
  async play(
    fr: number,
    fc: number,
    tr: number,
    tc: number,
    promotion: boolean,
    mid_sq?: number | null,
  ): Promise<void> {
    if (!this.board || this.board.game_over || this.busy) return;
    this.selectedSq = null;
    this.legalMoves = [];
    try {
      const resp = await api.sendMove([fr, fc], [tr, tc], promotion, mid_sq);
      if (resp.ok && resp.board) {
        this.lastMoveFrom = resp.last_move?.from ?? null;
        this.lastMoveTo = resp.last_move?.to ?? null;
        this.applyBoard(resp.board);
      } else {
        this.setMessage(resp.error ?? 'Illegal move', 'error');
        this.notify();
      }
    } catch (err) {
      this.setMessage(errorText(err), 'error');
      this.notify();
    }
    this.kickAutoLoop();
  }

  /** Ask the AI to move once (public API used by the UI buttons). */
  async aiMove(depth?: number, timeLimit?: number): Promise<void> {
    await this.aiMoveOnce(depth, timeLimit);
    this.kickAutoLoop();
  }

  /** Run an analysis search without touching the board. */
  async analyze(depth?: number, timeLimit = 10_000): Promise<void> {
    if (!this.board || this.busy) return;
    this.busy = true;
    this.notify();
    try {
      const d = depth ?? Math.max(1, this.settings.aiDepth);
      const resp = await api.analyze(d, timeLimit);
      if (resp.best_move) {
        this.lastSearch = {
          depth: resp.depth,
          score: resp.score,
          nodes: resp.nodes,
          time_ms: resp.time_ms,
        };
        const [fr, fc] = resp.best_move.from ?? [0, 0];
        const [tr, tc] = resp.best_move.to;
        this.setMessage(
          `Analysis d${resp.depth}: best ${fr},${fc}→${tr},${tc} (${resp.score})`,
          'info',
        );
      } else {
        this.setMessage('Analysis returned no move (time limit?)', 'error');
      }
    } catch (err) {
      this.setMessage(errorText(err), 'error');
    } finally {
      this.busy = false;
      this.notify();
    }
  }

  /** Save the current position and return the TSFEN payload. */
  async save(): Promise<SaveInfo | null> {
    try {
      const saved = await api.saveGame();
      this.setMessage(`Saved position (${saved.tsfen.length} chars)`, 'success');
      return saved;
    } catch (err) {
      this.setMessage(errorText(err), 'error');
      return null;
    } finally {
      this.notify();
    }
  }

  /** Load a position from TSFEN. */
  async load(tsfen: string): Promise<boolean> {
    this.busy = true;
    this.notify();
    try {
      const board = await api.loadGame(tsfen.trim());
      this.selectedSq = null;
      this.legalMoves = [];
      this.lastMoveFrom = null;
      this.lastMoveTo = null;
      this.lastSearch = null;
      this.applyBoard(board);
      this.setMessage('Position loaded', 'success');
      this.settings.autoPlay = false;
      return true;
    } catch (err) {
      this.setMessage(errorText(err), 'error');
      return false;
    } finally {
      this.busy = false;
      this.notify();
    }
  }

  // ----------------------------------------------------------------
  // Auto-play
  // ----------------------------------------------------------------

  /** Effective depth: random modes always use depth 0. */
  private depthForMode(): number {
    if (this.settings.mode === 'human_vs_random' || this.settings.mode === 'random_vs_random') {
      return 0;
    }
    return this.settings.aiDepth;
  }

  private shouldAutoPlay(): boolean {
    if (!this.board || this.board.game_over || this.aiPending) return false;
    const { mode, humanColor } = this.settings;
    const side = this.board.side_to_move;
    // Computer-vs-computer games only run while the Auto toggle is on.
    if (mode === 'ai_vs_ai' || mode === 'random_vs_random') return this.settings.autoPlay;
    if (mode === 'human_vs_random' || mode === 'human_vs_ai') return side !== humanColor;
    return false;
  }

  private async aiMoveOnce(depth?: number, timeLimit?: number): Promise<boolean> {
    if (!this.board || this.board.game_over || this.aiPending) return false;
    this.aiPending = true;
    this.busy = true;
    this.notify();
    try {
      const d = depth ?? this.depthForMode();
      const limit = timeLimit ?? (d === 0 ? 0 : 30_000);
      const resp = await api.sendAiMove(d, limit);
      if (resp.ok && resp.board) {
        this.lastMoveFrom = resp.last_move?.from ?? null;
        this.lastMoveTo = resp.last_move?.to ?? null;
        this.lastSearch = resp.search ?? null;
        this.applyBoard(resp.board);
        return true;
      }
      if (resp.error) this.setMessage(resp.error, 'error');
      // Stop the loop on any failed move (stale board, search timeout…).
      this.settings.autoPlay = false;
      return false;
    } catch (err) {
      this.setMessage(errorText(err), 'error');
      this.settings.autoPlay = false;
      return false;
    } finally {
      this.aiPending = false;
      this.busy = false;
      this.notify();
    }
  }

  /** Start the auto-play loop unless one is already running. */
  private kickAutoLoop(): void {
    void this.runAutoLoop();
  }

  private async runAutoLoop(): Promise<void> {
    if (this.autoLoopActive) return;
    this.autoLoopActive = true;
    try {
      while (this.shouldAutoPlay()) {
        await sleep(AI_DELAY_MS);
        const applied = await this.aiMoveOnce();
        if (!applied) break;
      }
    } finally {
      this.autoLoopActive = false;
    }
  }
}

function errorText(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}

/** The color the local human plays; only meaningful in human_* modes. */
export function humanSide(state: GameState): 0 | 1 {
  return state.settings.humanColor;
}

/** The side the AI controls in the current mode, if any. */
export function aiSide(state: GameState): 0 | 1 | null {
  const { mode, humanColor } = state.settings;
  if (mode === 'ai_vs_ai' || mode === 'random_vs_random') return state.board?.side_to_move ?? null;
  if (mode === 'human_vs_ai' || mode === 'human_vs_random') return opponent(humanColor);
  return null;
}
