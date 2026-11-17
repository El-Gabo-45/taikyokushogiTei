/**
 * HTTP client for the Taikyoku Shogi engine API.
 *
 * Every method returns fresh data from the Rust backend. Responses are
 * validated at runtime against the schemas in ./schemas before they reach
 * the UI. Errors carry the HTTP status and the server's JSON `error`
 * message when available, so the UI can show *why* something failed
 * (illegal move, unknown piece, 409, malformed payload…).
 */

import type {
  BoardState,
  EngineInfo,
  EvalInfo,
  HealthInfo,
  HistoryInfo,
  LegalMovesResponse,
  MoveData,
  MoveResponse,
  PieceInfo,
  PieceListResponse,
  PositionInfo,
  SaveInfo,
  SearchResponse,
  SearchStats,
  StatusInfo,
  UndoResponse,
} from '../types/board';
import {
  boardStateSchema,
  engineInfoSchema,
  evalInfoSchema,
  healthInfoSchema,
  historyInfoSchema,
  legalMovesSchema,
  moveListSchema,
  moveResponseSchema,
  newGameSchema,
  pieceInfoSchema,
  pieceListSchema,
  positionInfoSchema,
  saveInfoSchema,
  searchResponseSchema,
  statusInfoSchema,
  undoResponseSchema,
} from './schemas';
import type { ZodType } from 'zod';

const BASE = '';

export class ApiError extends Error {
  readonly status: number;

  constructor(message: string, status: number) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

/**
 * Per-tab session id sent as `X-Session-Id`. Shared-mode servers ignore it;
 * a server started with `--sessions`/`TAIKYOKU_SESSIONS=1` uses it to give
 * every visitor (browser tab) an isolated game. See docs/WEB.md.
 *
 * `sessionStorage` is deliberately used (not localStorage): two tabs of the
 * same browser are two separate games. Ids are restricted to
 * [A-Za-z0-9_-], the charset the server accepts.
 */
function sessionId(): string {
  const store = typeof sessionStorage !== 'undefined' ? sessionStorage : null;
  if (!store) return 'anonymous';
  let id = store.getItem('taikyoku_session');
  if (!id) {
    id =
      globalThis.crypto?.randomUUID?.() ??
      `t-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
    store.setItem('taikyoku_session', id);
  }
  return id;
}

async function request<T>(
  path: string,
  schema: ZodType<T>,
  init?: RequestInit,
): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: {
      Accept: 'application/json',
      'X-Session-Id': sessionId(),
      ...(init?.body ? { 'Content-Type': 'application/json' } : {}),
    },
    ...init,
  });

  let payload: unknown = null;
  const text = await res.text();
  if (text) {
    try {
      payload = JSON.parse(text);
    } catch {
      // Non-JSON response (e.g. proxy error page).
      payload = null;
    }
  }

  if (!res.ok) {
    const message =
      (payload as { error?: string } | null)?.error ??
      `HTTP ${res.status} ${res.statusText} (${path})`;
    throw new ApiError(message, res.status);
  }

  const parsed = schema.safeParse(payload);
  if (!parsed.success) {
    const issue = parsed.error.issues[0];
    const where = issue && issue.path.length > 0 ? ` at "${issue.path.join('.')}"` : '';
    const more =
      parsed.error.issues.length > 1 ? ` (+${parsed.error.issues.length - 1} more)` : '';
    throw new ApiError(
      `Invalid response from ${path}: ${issue?.message ?? 'validation failed'}${where}${more}`,
      res.status,
    );
  }
  return parsed.data;
}

const get = <T>(path: string, schema: ZodType<T>) => request(path, schema);
const post = <T>(path: string, schema: ZodType<T>, body?: unknown) =>
  request(
    path,
    schema,
    body === undefined ? { method: 'POST' } : { method: 'POST', body: JSON.stringify(body) },
  );

// ── Game state ─────────────────────────────────────────────────────

/** Fetch the full board state. */
export function fetchState(): Promise<BoardState> {
  return get('/api/state', boardStateSchema);
}

/** Compact status summary. */
export function fetchStatus(): Promise<StatusInfo> {
  return get('/api/status', statusInfoSchema);
}

/** Move log + score history. */
export function fetchHistory(): Promise<HistoryInfo> {
  return get('/api/history', historyInfoSchema);
}

/** Legal moves for the piece on `(r, c)`. */
export function fetchMoves(r: number, c: number): Promise<{ moves: MoveData[]; from: [number, number] }> {
  return get(`/api/moves?r=${r}&c=${c}`, moveListSchema);
}

/** Every legal move of the side to move. */
export function fetchLegalMoves(): Promise<LegalMovesResponse> {
  return get('/api/legal-moves', legalMovesSchema);
}

/** Start a new game. */
export function newGame(): Promise<{ ok: boolean; version: number }> {
  return post('/api/new-game', newGameSchema);
}

/** Apply a human move. */
export function sendMove(
  from: [number, number],
  to: [number, number],
  promotion = false,
  mid_sq?: number | null,
): Promise<MoveResponse> {
  return post('/api/move', moveResponseSchema, { from, to, promotion, mid_sq: mid_sq ?? null });
}

/** Ask the AI to search and apply a move. `depth = 0` means random. */
export function sendAiMove(depth = 2, timeLimit = 30_000): Promise<MoveResponse> {
  return post('/api/ai-move', moveResponseSchema, { depth, time_limit: timeLimit });
}

/** Analysis search — does not mutate the board. */
export function analyze(depth = 2, timeLimit = 10_000): Promise<SearchResponse> {
  return post('/api/search', searchResponseSchema, { depth, time_limit: timeLimit });
}

/** Undo the last `plies` moves (default 1). */
export function undo(plies = 1): Promise<UndoResponse> {
  return post('/api/undo', undoResponseSchema, { plies });
}

// ── Position / persistence ────────────────────────────────────────

export function fetchPosition(): Promise<PositionInfo> {
  return get('/api/position', positionInfoSchema);
}

/** Install a position from TSFEN. */
export function setPosition(tsfen: string): Promise<BoardState> {
  return post('/api/position', boardStateSchema, { tsfen });
}

export function saveGame(): Promise<SaveInfo> {
  return get('/api/save', saveInfoSchema);
}

export function loadGame(tsfen: string): Promise<BoardState> {
  return post('/api/load', boardStateSchema, { tsfen });
}

// ── Engine information ────────────────────────────────────────────

/** Piece database entry by abbreviation; throws `ApiError` 404 when unknown. */
export function pieceInfo(abbrev: string): Promise<PieceInfo> {
  return get(`/api/piece-info/${encodeURIComponent(abbrev)}`, pieceInfoSchema);
}

/** Full piece database (301 piece types). */
export function fetchPieces(): Promise<PieceListResponse> {
  return get('/api/pieces', pieceListSchema);
}

/** Engine metadata: sizes, backend, hash size, search techniques. */
export function fetchEngineInfo(): Promise<EngineInfo> {
  return get('/api/engine-info', engineInfoSchema);
}

/** Static evaluation and material balance. */
export function fetchEval(): Promise<EvalInfo> {
  return get('/api/evaluate', evalInfoSchema);
}

/** Liveness probe / server discovery. */
export function health(): Promise<HealthInfo> {
  return get('/api/health', healthInfoSchema);
}

export type { SearchStats };
