import { afterEach, describe, expect, it, vi } from 'vitest';
import * as api from '../src/api/client';

interface FetchCall {
  url: string;
  init?: RequestInit;
}

function mockJson(body: unknown, status = 200): FetchCall[] {
  const calls: FetchCall[] = [];
  vi.stubGlobal(
    'fetch',
    vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      calls.push({ url: String(url), init });
      return new Response(JSON.stringify(body), {
        status,
        headers: { 'Content-Type': 'application/json' },
      });
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
    const calls = mockJson({ board: [] });
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
    const calls = mockJson({});
    await api.fetchStatus();
    await api.fetchHistory();
    await api.fetchPosition();
    expect(calls.map((c) => c.url)).toEqual(['/api/status', '/api/history', '/api/position']);
  });
});

describe('api client — engine information', () => {
  it('fetches engine metadata, pieces and evaluation', async () => {
    const calls = mockJson({});
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
    const calls = mockJson({ abbrev: 'CP' });
    await api.pieceInfo('CP');
    expect(calls[0].url).toBe('/api/piece-info/CP');
  });

  it('turns a 404 into an ApiError with the server message', async () => {
    mockJson({ ok: false, error: 'unknown piece abbrev: ZZZ' }, 404);
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
    const calls = mockJson({ ok: true });
    await api.analyze(4, 2000);
    expect(calls[0].url).toBe('/api/search');
    expect(bodyOf(calls[0])).toEqual({ depth: 4, time_limit: 2000 });
  });

  it('sends the number of plies to undo', async () => {
    const calls = mockJson({ ok: true });
    await api.undo(2);
    expect(calls[0].url).toBe('/api/undo');
    expect(bodyOf(calls[0])).toEqual({ plies: 2 });
  });

  it('creates new games without a body', async () => {
    const calls = mockJson({ ok: true });
    await api.newGame();
    expect(calls[0].url).toBe('/api/new-game');
    expect(calls[0].init?.body).toBeUndefined();
  });
});

describe('api client — persistence', () => {
  it('saves and loads via save/load endpoints', async () => {
    const calls = mockJson({ ok: true, tsfen: 'x' });
    await api.saveGame();
    await api.loadGame('abc');
    expect(calls[0].url).toBe('/api/save');
    expect(calls[1].url).toBe('/api/load');
    expect(bodyOf(calls[1])).toEqual({ tsfen: 'abc' });
  });

  it('sets a position via POST /api/position', async () => {
    const calls = mockJson({});
    await api.setPosition('tsfen');
    expect(calls[0].url).toBe('/api/position');
    expect(calls[0].init?.method).toBe('POST');
    expect(bodyOf(calls[0])).toEqual({ tsfen: 'tsfen' });
  });

  it('sets JSON content-type only when there is a body', async () => {
    const calls = mockJson({});
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
