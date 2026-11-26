import { afterEach, describe, expect, it, vi } from 'vitest';
import type {
  BoardState,
  EngineInfo,
  EvalInfo,
  HealthInfo,
  PieceInfo,
  PositionInfo,
  SaveInfo,
  StatusInfo,
} from '../src/types/board';
import * as api from '../src/api/client';

interface FetchCall {
  url: string;
  init?: RequestInit;
}

/** Full `BoardState` payload; passes the client's runtime validation. */
const BOARD_STATE: BoardState = {
  board: [],
  side_to_move: 0,
  move_number: 1,
  black_pieces: 0,
  white_pieces: 0,
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
};

const STATUS: StatusInfo = {
  game_over: false,
  result: null,
  half_move: 0,
  move_number: 1,
  black_pieces: 0,
  white_pieces: 0,
  move_log: [],
  tsfen: 'initial b 1',
  version: 0,
};

const HISTORY = { move_log: [], half_move: 0, game_over: false, score_history: [0] };
const POSITION: PositionInfo = { tsfen: 'initial b 1', side_to_move: 0, move_number: 1 };

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

const PIECE_LIST = { count: 0, pieces: [] };

const EVAL: EvalInfo = {
  eval_score: 0,
  material_score: 0,
  black_pieces: 402,
  white_pieces: 402,
  side_to_move: 0,
};

const HEALTH: HealthInfo = {
  ok: true,
  name: 'taikyokushogi',
  version: '0.1.0',
  board_size: 36,
  piece_types: 301,
};

const PIECE_INFO: PieceInfo = {
  abbrev: 'CP',
  name: 'Crown Prince',
  value: 2,
  promotes_to: null,
  slide_directions: 0,
  jump_destinations: 0,
  has_hook: false,
  area_steps: 0,
  has_range_capture: false,
  has_igui: false,
  is_royal: true,
  betza: null,
};

const SAVE: SaveInfo = { ok: true, tsfen: 'x', move_log: [], version: 0 };

function jsonOf(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

/** Queue of JSON bodies: call N gets body N (last body repeats). */
function mockJson(...bodies: unknown[]): FetchCall[] {
  const calls: FetchCall[] = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), init });
      const body = bodies[calls.length - 1] ?? bodies[bodies.length - 1];
      return jsonOf(body);
    }),
  );
  return calls;
}

/** Single JSON body with a specific HTTP status. */
function mockJsonStatus(body: unknown, status: number): FetchCall[] {
  const calls: FetchCall[] = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), init });
      return jsonOf(body, status);
    }),
  );
  return calls;
}

function bodyOf(call: FetchCall): unknown {
  return call.init?.body ? JSON.parse(String(call.init.body)) : undefined;
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('api client — read endpoints', () => {
  it('fetches the board state', async () => {
    const calls = mockJson(BOARD_STATE);
    await api.fetchState();
    expect(calls[0].url).toBe('/api/state');
    expect(calls[0].init?.method ?? 'GET').toBe('GET');
  });

  it('passes coordinates to /api/moves', async () => {
    const calls = mockJson({ moves: [], from: [3, 4] });
    await api.fetchMoves(3, 4);
    expect(calls[0].url).toBe('/api/moves?r=3&c=4');
  });

  it('lists all legal moves', async () => {
    const calls = mockJson({ moves: [], count: 0, side_to_move: 0, game_over: false });
    await api.fetchLegalMoves();
    expect(calls[0].url).toBe('/api/legal-moves');
  });

  it('fetches status, history and position', async () => {
    const calls = mockJson(STATUS, HISTORY, POSITION);
    await api.fetchStatus();
    await api.fetchHistory();
    await api.fetchPosition();
    expect(calls.map((c) => c.url)).toEqual(['/api/status', '/api/history', '/api/position']);
  });
});

describe('api client — engine information', () => {
  it('fetches engine metadata, pieces and evaluation', async () => {
    const calls = mockJson(ENGINE_INFO, PIECE_LIST, EVAL, HEALTH);
    await api.fetchEngineInfo();
    await api.fetchPieces();
    await api.fetchEval();
    await api.health();
    expect(calls.map((c) => c.url)).toEqual([
      '/api/engine-info',
      '/api/pieces',
      '/api/evaluate',
      '/api/health',
    ]);
  });

  it('URL-encodes piece abbreviations', async () => {
    const calls = mockJson(PIECE_INFO);
    await api.pieceInfo('CP');
    expect(calls[0].url).toBe('/api/piece-info/CP');
  });

  it('turns a 404 into an ApiError with the server message', async () => {
    mockJsonStatus({ ok: false, error: 'unknown piece abbrev: ZZZ' }, 404);
    await expect(api.pieceInfo('ZZZ')).rejects.toMatchObject({
      name: 'ApiError',
      status: 404,
      message: 'unknown piece abbrev: ZZZ',
    });
  });

  it('handles non-JSON error responses without crashing', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('Bad Gateway', { status: 502 })),
    );
    const error = await api.fetchState().catch((e: unknown) => e);
    expect(error).toBeInstanceOf(api.ApiError);
    expect((error as api.ApiError).status).toBe(502);
    expect((error as api.ApiError).message).toContain('502');
  });
});

describe('api client — mutating endpoints', () => {
  it('posts a human move with promotion and mid_sq', async () => {
    const calls = mockJson({ ok: true });
    await api.sendMove([1, 2], [3, 4], true, 55);
    expect(calls[0].url).toBe('/api/move');
    expect(calls[0].init?.method).toBe('POST');
    expect(bodyOf(calls[0])).toEqual({
      from: [1, 2],
      to: [3, 4],
      promotion: true,
      mid_sq: 55,
    });
  });

  it('normalises a missing mid_sq to null', async () => {
    const calls = mockJson({ ok: true });
    await api.sendMove([1, 2], [3, 4]);
    expect(bodyOf(calls[0])).toMatchObject({ promotion: false, mid_sq: null });
  });

  it('posts depth and time limit for AI moves', async () => {
    const calls = mockJson({ ok: true });
    await api.sendAiMove(3, 5000);
    expect(calls[0].url).toBe('/api/ai-move');
    expect(bodyOf(calls[0])).toEqual({ depth: 3, time_limit: 5000 });
  });

  it('posts analysis searches to /api/search', async () => {
    const calls = mockJson({ ok: true, score: 0, nodes: 0, time_ms: 0, depth: 4, tsfen: '' });
    await api.analyze(4, 2000);
    expect(calls[0].url).toBe('/api/search');
    expect(bodyOf(calls[0])).toEqual({ depth: 4, time_limit: 2000 });
  });

  it('sends the number of plies to undo', async () => {
    const calls = mockJson({ ok: true, undone_plies: 0 });
    await api.undo(2);
    expect(calls[0].url).toBe('/api/undo');
    expect(bodyOf(calls[0])).toEqual({ plies: 2 });
  });

  it('creates new games without a body', async () => {
    const calls = mockJson({ ok: true, version: 1 });
    await api.newGame();
    expect(calls[0].url).toBe('/api/new-game');
    expect(calls[0].init?.body).toBeUndefined();
  });
});

describe('api client — persistence', () => {
  it('saves and loads via save/load endpoints', async () => {
    const calls = mockJson(SAVE, BOARD_STATE);
    await api.saveGame();
    await api.loadGame('abc');
    expect(calls[0].url).toBe('/api/save');
    expect(calls[1].url).toBe('/api/load');
    expect(bodyOf(calls[1])).toEqual({ tsfen: 'abc' });
  });

  it('sets a position via POST /api/position', async () => {
    const calls = mockJson(BOARD_STATE);
    await api.setPosition('tsfen');
    expect(calls[0].url).toBe('/api/position');
    expect(calls[0].init?.method).toBe('POST');
    expect(bodyOf(calls[0])).toEqual({ tsfen: 'tsfen' });
  });

  it('sets JSON content-type only when there is a body', async () => {
    const calls = mockJson(BOARD_STATE, { ok: true });
    await api.fetchState();
    await api.sendAiMove(1, 1000);
    const headers0 = calls[0].init?.headers as Record<string, string>;
    expect(headers0['Accept']).toBe('application/json');
    // Session id present and within the charset the server accepts
    // ([A-Za-z0-9_-]{1,64}, see docs/WEB.md).
    expect(headers0['X-Session-Id']).toMatch(/^[A-Za-z0-9_-]{1,64}$/);
    const headers = calls[1].init?.headers as Record<string, string>;
    expect(headers['Content-Type']).toBe('application/json');
  });
});

describe('api client — response validation', () => {
  it('rejects an incomplete board payload with an ApiError naming the field', async () => {
    mockJson({ board: [] });
    const error = await api.fetchState().catch((e: unknown) => e);
    expect(error).toBeInstanceOf(api.ApiError);
    expect((error as api.ApiError).message).toContain('side_to_move');
  });

  it('rejects values outside the wire contract', async () => {
    mockJson({ ...BOARD_STATE, side_to_move: 2 });
    const error = await api.fetchState().catch((e: unknown) => e);
    expect(error).toBeInstanceOf(api.ApiError);
    expect((error as api.ApiError).message).toContain('side_to_move');
  });

  it('rejects non-object payloads and names the endpoint', async () => {
    mockJson('nope');
    const error = await api.fetchState().catch((e: unknown) => e);
    expect(error).toBeInstanceOf(api.ApiError);
    expect((error as api.ApiError).message).toContain('/api/state');
    expect((error as api.ApiError).message).toContain('expected object');
  });

  it('validates nested board cells', async () => {
    mockJson({
      ...BOARD_STATE,
      board: [[{ piece: 'K', color: 7, name: 'King', value: 0, is_royal: true }]],
    });
    const error = await api.fetchState().catch((e: unknown) => e);
    expect(error).toBeInstanceOf(api.ApiError);
    expect((error as api.ApiError).message).toContain('board.0.0.color');
  });
});

describe('request timeout safety belt', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('sends an AbortSignal with every request', async () => {
    const calls = mockJson(HEALTH);
    await api.health();
    expect(calls[0]?.init?.signal).toBeInstanceOf(AbortSignal);
  });

  it('turns an aborted fetch into a 408 ApiError naming the endpoint', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async (_url: string | URL | Request, init?: RequestInit) => {
        // Simulate the browser rejecting a fetch whose signal was aborted.
        const signal = init?.signal;
        if (signal) {
          await new Promise<void>((resolve) => {
            signal.addEventListener('abort', () => resolve(), { once: true });
          });
          throw new DOMException('The operation was aborted', 'AbortError');
        }
        return jsonOf(HEALTH);
      }),
    );

    // Trigger the timeout immediately by overriding the exported cap is not
    // possible (const), so use fake timers to advance past it.
    vi.useFakeTimers();
    const promise = api.health().catch((e: unknown) => e);
    await vi.advanceTimersByTimeAsync(api.REQUEST_TIMEOUT_MS + 1);
    const error = await promise;
    vi.useRealTimers();

    expect(error).toBeInstanceOf(api.ApiError);
    expect((error as api.ApiError).status).toBe(408);
    expect((error as api.ApiError).message).toContain('/api/health');
    expect((error as api.ApiError).message).toContain('timed out');
  });
});
