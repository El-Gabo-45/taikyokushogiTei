import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { BoardState, CellData, EngineInfo, PieceInfo } from '../src/types/board';

vi.mock('../src/api/client', () => {
  class ApiError extends Error {
    status: number;
    constructor(message: string, status: number) {
      super(message);
      this.name = 'ApiError';
      this.status = status;
    }
  }
  return {
    ApiError,
    fetchState: vi.fn(),
    fetchEngineInfo: vi.fn(),
    fetchMoves: vi.fn(),
    newGame: vi.fn(),
    undo: vi.fn(),
    sendMove: vi.fn(),
    sendAiMove: vi.fn(),
    analyze: vi.fn(),
    saveGame: vi.fn(),
    loadGame: vi.fn(),
    pieceInfo: vi.fn(),
  };
});

import * as api from '../src/api/client';
import { GameState } from '../src/game/state';

const mocked = vi.mocked(api);

// ── fixtures ─────────────────────────────────────────────────────

function emptyGrid(): (CellData | null)[][] {
  return Array.from({ length: 36 }, () => Array<CellData | null>(36).fill(null));
}

const WHITE_PAWN: CellData = {
  piece: 'P',
  color: 1,
  name: 'Pawn',
  value: 1,
  is_royal: false,
};

function makeBoard(overrides: Partial<BoardState> = {}): BoardState {
  return {
    board: emptyGrid(),
    side_to_move: 0,
    move_number: 1,
    black_pieces: 402,
    white_pieces: 402,
    game_result: null,
    version: 0,
    tsfen: 'initial b 1',
    eval_score: 0,
    material_score: 0,
    no_progress_plies: 0,
    mode: 'human_vs_ai',
    move_log: [],
    score_history: [0],
    game_over: false,
    ...overrides,
  };
}

const ENGINE_INFO: EngineInfo = {
  name: 'taikyokushogi',
  version: '0.1.0',
  board_size: 36,
  num_squares: 1296,
  piece_types: 301,
  royal_piece_types: 2,
  black_pieces: 402,
  white_pieces: 402,
  side_to_move: 0,
  move_number: 1,
  eval_backend: 'hand-crafted',
  using_nnue: false,
  hash_mb: 64,
  default_threads: 1,
  max_threads: 64,
  search_techniques: ['razoring', 'quiescence'],
};

const PAWN_INFO: PieceInfo = {
  abbrev: 'P',
  name: 'Pawn',
  value: 1,
  promotes_to: 'PG',
  slide_directions: 0,
  jump_destinations: 1,
  has_hook: false,
  area_steps: 0,
  has_range_capture: false,
  has_igui: false,
  is_royal: false,
  betza: 'fW',
};

function freshState(): GameState {
  const state = new GameState();
  // Human plays White so a Black-to-move snapshot never auto-plays in tests.
  state.settings.humanColor = 1;
  return state;
}

beforeEach(() => {
  // Fake timers keep the auto-play loop's 120 ms sleeps pending unless a
  // test explicitly advances them — no cross-test AI requests.
  vi.useFakeTimers();
  vi.resetAllMocks();
  mocked.fetchEngineInfo.mockResolvedValue(ENGINE_INFO);
});

afterEach(() => {
  vi.useRealTimers();
});

// ── init / reset ─────────────────────────────────────────────────

describe('GameState.init', () => {
  it('loads board, engine info and evaluation', async () => {
    const board = makeBoard({ eval_score: 42, material_score: 7 });
    mocked.fetchState.mockResolvedValue(board);

    const state = freshState();
    await state.init();

    expect(state.board).toBe(board);
    expect(state.engineInfo).toEqual(ENGINE_INFO);
    expect(state.evalInfo).toEqual({
      eval_score: 42,
      material_score: 7,
      black_pieces: 402,
      white_pieces: 402,
      side_to_move: 0,
    });
    expect(state.selectedSq).toBeNull();
    expect(state.message?.text).toBe('Ready');
  });

  it('survives an engine-info failure', async () => {
    mocked.fetchState.mockResolvedValue(makeBoard());
    mocked.fetchEngineInfo.mockRejectedValue(new Error('not available'));

    const state = freshState();
    await state.init();

    expect(state.engineInfo).toBeNull();
    expect(state.board).not.toBeNull();
  });

  it('reports unreachable backends without throwing into the UI silently', async () => {
    const error = new Error('Failed to fetch');
    mocked.fetchState.mockRejectedValue(error);

    const state = freshState();
    await expect(state.init()).rejects.toThrow('Failed to fetch');
    expect(state.message).toEqual({ text: 'Failed to fetch', kind: 'error' });
  });
});

describe('GameState.reset', () => {
  it('creates a new game and reloads the board', async () => {
    mocked.newGame.mockResolvedValue({ ok: true, version: 1 });
    mocked.fetchState.mockResolvedValue(makeBoard());

    const state = freshState();
    await state.reset();

    expect(mocked.newGame).toHaveBeenCalledOnce();
    expect(state.board?.move_number).toBe(1);
    expect(state.message?.text).toBe('New game');
  });
});

// ── selection & moves ────────────────────────────────────────────

describe('GameState selection', () => {
  it('fetches legal destinations for a square', async () => {
    mocked.fetchMoves.mockResolvedValue({
      moves: [{ to: [10, 10], promotion: false, is_igui: false, captured: null }],
      from: [9, 9],
    });
    const state = freshState();
    state.board = makeBoard();

    await state.select(9, 9);

    expect(mocked.fetchMoves).toHaveBeenCalledWith(9, 9);
    expect(state.selectedSq).toEqual([9, 9]);
    expect(state.legalMoves).toHaveLength(1);
  });

  it('clears the move list when the request fails', async () => {
    mocked.fetchMoves.mockRejectedValue(new Error('boom'));
    const state = freshState();
    state.board = makeBoard();

    await state.select(1, 1);

    expect(state.legalMoves).toEqual([]);
    expect(state.message?.kind).toBe('error');
  });
});

describe('GameState.play', () => {
  it('applies the server response and remembers the last move', async () => {
    const next = makeBoard({ side_to_move: 1, version: 1, move_log: ['1. Black: P'] });
    mocked.sendMove.mockResolvedValue({
      ok: true,
      board: next,
      last_move: { from: [1, 2], to: [3, 4] },
    });

    const state = freshState();
    state.board = makeBoard();
    await state.play(1, 2, 3, 4, false, null);

    expect(mocked.sendMove).toHaveBeenCalledWith([1, 2], [3, 4], false, null);
    expect(state.board).toBe(next);
    expect(state.lastMoveFrom).toEqual([1, 2]);
    expect(state.lastMoveTo).toEqual([3, 4]);
  });

  it('keeps the old board and shows the error for illegal moves', async () => {
    mocked.sendMove.mockResolvedValue({ ok: false, error: 'Illegal move' });
    const state = freshState();
    const before = makeBoard();
    state.board = before;

    await state.play(0, 0, 0, 0, false);

    expect(state.board).toBe(before);
    expect(state.message).toEqual({ text: 'Illegal move', kind: 'error' });
  });

  it('does nothing while the game is over', async () => {
    const state = freshState();
    state.board = makeBoard({ game_over: true });
    await state.play(0, 0, 1, 1, false);
    expect(mocked.sendMove).not.toHaveBeenCalled();
  });
});

// ── AI & analysis ────────────────────────────────────────────────

describe('GameState.aiMove', () => {
  it('stores search telemetry and the new board', async () => {
    const next = makeBoard({ side_to_move: 1, version: 2 });
    mocked.sendAiMove.mockResolvedValue({
      ok: true,
      board: next,
      last_move: { from: [5, 5], to: [6, 6] },
      search: { depth: 2, score: 15, nodes: 12345, time_ms: 30 },
    });

    const state = freshState();
    state.board = makeBoard();
    state.settings.aiDepth = 2;

    await state.aiMove();

    expect(mocked.sendAiMove).toHaveBeenCalledWith(2, 30_000);
    expect(state.board).toBe(next);
    expect(state.lastSearch).toEqual({ depth: 2, score: 15, nodes: 12345, time_ms: 30 });
  });

  it('uses depth 0 (random) in random modes', async () => {
    mocked.sendAiMove.mockResolvedValue({ ok: true, board: makeBoard({ side_to_move: 0 }) });
    const state = freshState();
    state.board = makeBoard();
    state.settings.mode = 'random_vs_random';

    await state.aiMove();

    expect(mocked.sendAiMove).toHaveBeenCalledWith(0, 0);
  });

  it('stops auto-play and reports errors', async () => {
    mocked.sendAiMove.mockResolvedValue({ ok: false, error: 'Game is over' });
    const state = freshState();
    state.board = makeBoard();
    state.settings.autoPlay = true;

    await state.aiMove();

    expect(state.settings.autoPlay).toBe(false);
    expect(state.message).toEqual({ text: 'Game is over', kind: 'error' });
  });
});

describe('GameState.analyze', () => {
  it('records analysis telemetry without changing the board', async () => {
    mocked.analyze.mockResolvedValue({
      ok: true,
      best_move: { from: [1, 1], to: [2, 2], promotion: false, is_igui: false, captured: null },
      score: -12,
      nodes: 999,
      time_ms: 10,
      depth: 2,
      tsfen: 'same b 1',
    });
    const state = freshState();
    const board = makeBoard();
    state.board = board;

    await state.analyze(2);

    expect(state.board).toBe(board);
    expect(state.lastSearch).toEqual({ depth: 2, score: -12, nodes: 999, time_ms: 10 });
    expect(state.message?.text).toContain('Analysis');
  });

  it('handles a search that returns no move', async () => {
    mocked.analyze.mockResolvedValue({
      ok: false,
      score: 0,
      nodes: 0,
      time_ms: 1,
      depth: 2,
      tsfen: 'x',
    });
    const state = freshState();
    state.board = makeBoard();

    await state.analyze(2);

    expect(state.lastSearch).toBeNull();
    expect(state.message?.kind).toBe('error');
  });
});

// ── undo / load / save ───────────────────────────────────────────

describe('GameState.undo', () => {
  it('restores the returned board and disables auto-play', async () => {
    const restored = makeBoard({ version: 3 });
    mocked.undo.mockResolvedValue({ ok: true, board: restored, undone_plies: 1 });
    const state = freshState();
    state.board = makeBoard({ version: 2 });
    state.settings.autoPlay = true;

    await state.undo(1);

    expect(state.board).toBe(restored);
    expect(state.settings.autoPlay).toBe(false);
    expect(state.message?.text).toContain('Undid');
  });

  it('shows the server error when there is nothing to undo', async () => {
    mocked.undo.mockResolvedValue({
      ok: false,
      error: 'Nothing to undo',
      undone_plies: 0,
    });
    const state = freshState();
    state.board = makeBoard();

    await state.undo();

    expect(state.message).toEqual({ text: 'Nothing to undo', kind: 'error' });
  });
});

describe('GameState.load / save', () => {
  it('loads a TSFEN position', async () => {
    const loaded = makeBoard({ tsfen: 'loaded w 5', side_to_move: 1, version: 9 });
    mocked.loadGame.mockResolvedValue(loaded);
    const state = freshState();
    state.board = makeBoard();

    const ok = await state.load('loaded w 5');

    expect(ok).toBe(true);
    expect(mocked.loadGame).toHaveBeenCalledWith('loaded w 5');
    expect(state.board).toBe(loaded);
    expect(state.lastMoveFrom).toBeNull();
  });

  it('keeps the board when loading fails', async () => {
    mocked.loadGame.mockRejectedValue(new api.ApiError('invalid TSFEN', 400));
    const state = freshState();
    const before = makeBoard();
    state.board = before;

    const ok = await state.load('garbage');

    expect(ok).toBe(false);
    expect(state.board).toBe(before);
    expect(state.message?.kind).toBe('error');
  });

  it('returns the save payload', async () => {
    mocked.saveGame.mockResolvedValue({
      ok: true,
      tsfen: 'saved b 1',
      move_log: [],
      version: 0,
    });
    const state = freshState();
    state.board = makeBoard();

    const saved = await state.save();

    expect(saved?.tsfen).toBe('saved b 1');
    expect(state.message?.kind).toBe('success');
  });
});

// ── inspection ───────────────────────────────────────────────────

describe('GameState.inspect', () => {
  it('fetches and caches piece database info', async () => {
    mocked.pieceInfo.mockResolvedValue(PAWN_INFO);
    const board = makeBoard();
    board.board[5][5] = WHITE_PAWN;

    const state = freshState();
    state.board = board;

    await state.inspect(5, 5);
    expect(state.inspection?.info).toEqual(PAWN_INFO);
    await state.inspect(5, 5);
    expect(mocked.pieceInfo).toHaveBeenCalledTimes(1);
  });

  it('shows board data even when the piece is unknown to the database', async () => {
    mocked.pieceInfo.mockRejectedValue(new api.ApiError('unknown', 404));
    const board = makeBoard();
    board.board[0][0] = { ...WHITE_PAWN, piece: 'ZZ' };

    const state = freshState();
    state.board = board;
    await state.inspect(0, 0);

    expect(state.inspection?.piece.piece).toBe('ZZ');
    expect(state.inspection?.info).toBeNull();
    expect(state.message).toBeNull();
  });

  it('clears the inspector for empty squares', async () => {
    const state = freshState();
    state.board = makeBoard();
    await state.inspect(3, 3);
    expect(state.inspection).toBeNull();
  });
});

// ── view settings & auto-play ────────────────────────────────────

describe('GameState view settings', () => {
  it('toggles kanji and flip', () => {
    const state = freshState();
    const initialKanji = state.settings.useKanji;
    const initialFlip = state.settings.flip;
    state.toggleKanji();
    state.toggleFlip();
    expect(state.settings.useKanji).toBe(!initialKanji);
    expect(state.settings.flip).toBe(!initialFlip);
  });

  it('clamps the zoom factor', () => {
    const state = freshState();
    state.setZoom(1000);
    expect(state.settings.zoom).toBe(1.8);
    state.setZoom(0.01);
    expect(state.settings.zoom).toBe(0.7);
  });

  it('notifies subscribers', () => {
    const state = freshState();
    const listener = vi.fn();
    const unsubscribe = state.subscribe(listener);
    state.toggleKanji();
    expect(listener).toHaveBeenCalledOnce();
    unsubscribe();
    state.toggleKanji();
    expect(listener).toHaveBeenCalledOnce();
  });
});

describe('GameState auto-play', () => {
  it('does not auto-run ai_vs_ai until Auto is enabled', async () => {
    vi.useFakeTimers();
    mocked.fetchState.mockResolvedValue(makeBoard());
    const state = freshState();
    state.settings.mode = 'ai_vs_ai';
    state.settings.autoPlay = false;

    await state.init();
    await vi.advanceTimersByTimeAsync(500);

    expect(mocked.sendAiMove).not.toHaveBeenCalled();
  });

  it('runs one AI move when Auto is enabled and stops on failure', async () => {
    vi.useFakeTimers();
    mocked.fetchState.mockResolvedValue(makeBoard());
    mocked.sendAiMove.mockResolvedValue({ ok: false, error: 'stale board' });
    const state = freshState();
    state.settings.mode = 'ai_vs_ai';

    await state.init();
    state.setAutoPlay(true);
    await vi.advanceTimersByTimeAsync(500);

    expect(mocked.sendAiMove).toHaveBeenCalledTimes(1);
    expect(state.settings.autoPlay).toBe(false);
  });
});
