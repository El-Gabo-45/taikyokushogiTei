import { describe, expect, it } from 'vitest';
import type { BoardState, CellData, EngineInfo } from '../src/types/board';
import {
  GAME_FILE_FORMAT,
  GAME_FILE_VERSION,
  buildGameFile,
  buildPgn,
  gameFileName,
  serializeGameFile,
} from '../src/game/export';

// ── fixtures ─────────────────────────────────────────────────────

function emptyGrid(): (CellData | null)[][] {
  return Array.from({ length: 36 }, () => Array<CellData | null>(36).fill(null));
}

function makeBoard(overrides: Partial<BoardState> = {}): BoardState {
  return {
    board: emptyGrid(),
    side_to_move: 0,
    move_number: 1,
    black_pieces: 402,
    white_pieces: 402,
    game_result: null,
    version: 7,
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

const ENGINE: EngineInfo = {
  name: 'taikyokushogi',
  version: '0.2.0',
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
  max_threads: 8,
  search_techniques: [],
};

const AT = new Date(2026, 10, 10, 15, 30, 12); // local time → 2026-11-10 15:30:12

// ── tests ────────────────────────────────────────────────────────

describe('buildGameFile', () => {
  it('writes the CLI-compatible fields at the top level', () => {
    const board = makeBoard({
      tsfen: 'tsfen-after-moves b 3',
      move_log: ['1. Black: P (35,0)-(34,0)', '1. White: P (2,0)-(3,0)'],
      move_number: 3,
      side_to_move: 1,
    });
    const file = buildGameFile(board, { exportedAt: AT, engine: ENGINE });

    // These four names and value types are what `GameFile` in src/cli.rs reads.
    expect(file.tsfen).toBe('tsfen-after-moves b 3');
    expect(file.moves).toEqual(board.move_log);
    expect(file.side).toBe('w');
    expect(file.move_number).toBe(3);
    expect(typeof file.tsfen).toBe('string');
    expect(Array.isArray(file.moves)).toBe(true);
  });

  it('identifies the format and adds metadata without touching the game', () => {
    const board = makeBoard({
      move_log: ['1. Black: P (35,0)-(34,0)'],
      game_result: 'black_wins',
      black_pieces: 401,
      white_pieces: 400,
      no_progress_plies: 1,
    });
    const file = buildGameFile(board, { exportedAt: AT, engine: ENGINE });

    expect(file.format).toBe(GAME_FILE_FORMAT);
    expect(file.version).toBe(GAME_FILE_VERSION);
    expect(file.app).toBe('taikyokushogi-web');
    expect(file.exported_at).toBe(AT.toISOString());
    expect(file.engine).toEqual({ name: 'taikyokushogi', version: '0.2.0' });
    expect(file.result).toBe('black_wins');
    expect(file.black_pieces).toBe(401);
    expect(file.white_pieces).toBe(400);
    expect(file.no_progress_plies).toBe(1);
    expect(file.tsfen).toBe(board.tsfen);
  });

  it('maps the side to move to the CLI letters', () => {
    expect(buildGameFile(makeBoard({ side_to_move: 0 })).side).toBe('b');
    expect(buildGameFile(makeBoard({ side_to_move: 1 })).side).toBe('w');
  });

  it('exports a fresh game with no moves and no engine metadata', () => {
    const file = buildGameFile(makeBoard(), { exportedAt: AT, engine: null });

    expect(file.moves).toEqual([]);
    expect(file.result).toBeNull();
    expect(file.engine).toBeNull();
    expect(file.tsfen).toBe('initial b 1');
  });

  it('copies the move list instead of sharing the board array', () => {
    const board = makeBoard({ move_log: ['1. Black: P (35,0)-(34,0)'] });
    const file = buildGameFile(board);
    file.moves.push('injected');

    expect(board.move_log).toEqual(['1. Black: P (35,0)-(34,0)']);
  });
});

describe('serializeGameFile', () => {
  it('returns pretty JSON ending in a newline that parses back', () => {
    const board = makeBoard({
      tsfen: 'after-move b 2',
      move_log: ['1. Black: P (35,0)-(34,0)'],
      move_number: 2,
      side_to_move: 1,
    });
    const text = serializeGameFile(buildGameFile(board, { exportedAt: AT, engine: ENGINE }));

    expect(text.endsWith('\n')).toBe(true);
    expect(text).toContain('\n  "tsfen": "after-move b 2",');
    expect(JSON.parse(text)).toMatchObject({
      tsfen: 'after-move b 2',
      moves: ['1. Black: P (35,0)-(34,0)'],
      side: 'w',
      move_number: 2,
    });
  });
});

describe('buildPgn', () => {
  it('writes standard-shaped headers and coordinate move text', () => {
    const board = makeBoard({
      move_log: [
        '1. Black: BG (33,31)-(29,27)',
        '2. White: BG (2,4)-(29,31)',
      ],
      move_number: 2,
      side_to_move: 0,
    });
    const pgn = buildPgn(board, { exportedAt: AT, engine: ENGINE });

    expect(pgn).toContain('[Event "Taikyoku Shogi - Web Game"]');
    expect(pgn).toContain('[Date "2026.11.10"]');
    expect(pgn).toContain('[Variant "Taikyoku Shogi"]');
    expect(pgn).toContain('[BoardSize "36x36"]');
    expect(pgn).toContain('[MoveFormat "piece.from-to"]');
    expect(pgn).toContain('[PlyCount "2"]');
    expect(pgn).toContain('[Engine "taikyokushogi 0.2.0"]');
    expect(pgn).toContain('[Result "*"]');
    expect(pgn.endsWith('1. BG.F34-B30 BG.e3-F30 *\n')).toBe(true);
  });

  it('maps finished games to PGN results and termination tags', () => {
    expect(buildPgn(makeBoard({ game_result: 'black_wins' }))).toContain(
      '[Result "1-0"]',
    );
    expect(buildPgn(makeBoard({ game_result: 'white_wins' }))).toContain(
      '[Result "0-1"]',
    );
    expect(buildPgn(makeBoard({ game_result: 'draw' }))).toContain(
      '[Result "1/2-1/2"]',
    );
    expect(buildPgn(makeBoard({ game_result: 'black_wins' }))).toContain(
      '[Termination "black wins"]',
    );
  });

  it('keeps promotion suffixes and numbers plies from the current snapshot', () => {
    const board = makeBoard({
      move_log: [
        '11. White: P (35,0)-(34,0)+',
        '12. Black: P (0,35)-(1,35)',
      ],
      move_number: 12,
      side_to_move: 1,
    });
    const pgn = buildPgn(board, { exportedAt: AT });

    expect(pgn).toContain('11... P.a36-a35+ 12. P.J1-J2 *');
  });

  it('exports a fresh game with only the result token', () => {
    const pgn = buildPgn(makeBoard(), { exportedAt: AT, engine: null });

    expect(pgn).toContain('[PlyCount "0"]');
    expect(pgn).toContain('[Result "*"]');
    expect(pgn.endsWith('\n\n*\n')).toBe(true);
  });

  it('escapes quotes and backslashes in tag values', () => {
    const engine = { ...ENGINE, name: 'taikyoku "test"\\net' };
    const pgn = buildPgn(makeBoard(), { engine });
    expect(pgn).toContain('[Engine "taikyoku \\"test\\"\\\\net 0.2.0"]');
  });
});

describe('gameFileName', () => {
  it('builds a timestamped, move-numbered .json name', () => {
    expect(gameFileName(AT, 42)).toBe('taikyoku-game-20261110-153012-move-42.json');
  });

  it('uses the requested extension', () => {
    expect(gameFileName(AT, 42, 'pgn')).toBe(
      'taikyoku-game-20261110-153012-move-42.pgn',
    );
  });

  it('zero-pads every timestamp component', () => {
    expect(gameFileName(new Date(2026, 0, 2, 3, 4, 5), 1)).toBe(
      'taikyoku-game-20260102-030405-move-1.json',
    );
  });
});
