/** HTTP client for the Taikyoku Shogi engine API.
 * All methods return fresh data from the Rust backend.
 */

import type { BoardState, MoveResponse, MoveData, PieceInfoResponse } from '../types/board';

const BASE = '';

// HTTP helpers
async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { Accept: 'application/json' },
  });
  if (!res.ok) throw new Error(`GET ${path}: ${res.status}`);
  return res.json();
}

async function post<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new Error(`POST ${path}: ${res.status}`);
  return res.json();
}

/** Fetch the full board state */
export function fetchState(): Promise<BoardState> {
  return get<BoardState>('/api/state');
}

/** Fetch legal moves from a specific square */
export function fetchMoves(r: number, c: number) {
  return get<{ moves: MoveData[]; from: [number, number] }>(
    `/api/moves?r=${r}&c=${c}`,
  );
}

/** Fetch all legal moves (from any square) */
export function fetchLegalMoves() {
  return get<{ moves: MoveData[]; from: [number, number] }>('/api/legal-moves');
}

/** Apply a human move */
export function sendMove(
  from: [number, number],
  to: [number, number],
  promotion = false,
  mid_sq?: number | null,
): Promise<MoveResponse> {
  return post<MoveResponse>('/api/move', { from, to, promotion, mid_sq });
}

/** Ask the AI to generate and apply a move */
export function sendAiMove(depth = 0, timeLimit = 30_000): Promise<MoveResponse> {
  return post<MoveResponse>('/api/ai-move', { depth, time_limit: timeLimit });
}

/** Start a new game */
export function newGame(): Promise<{ ok: boolean }> {
  return post('/api/new-game');
}

/** Undo the last move */
export function undo(): Promise<{ ok: boolean; error?: string }> {
  return post('/api/undo');
}

/** Fetch the move history */
export function fetchHistory() {
  return get<{ move_log: string[]; half_move: number; game_over: boolean }>('/api/history');
}

/** Get the position in TSFEN format */
export function fetchPosition() {
  return get<{ tsfen: string; side_to_move: number; move_number: number }>('/api/position');
}

/** Set the position from TSFEN */
export function setPosition(tsfen: string): Promise<{ ok: boolean; tsfen: string; error?: string }> {
  return post<{ ok: boolean; tsfen: string; error?: string }>('/api/position', { tsfen });
}

/** Get piece information */
export function pieceInfo(abbrev: string): Promise<PieceInfoResponse> {
  return get<PieceInfoResponse>(`/api/piece-info/${abbrev}`);
}

/** Save the current game state */
export function saveGame(): Promise<{ ok: boolean; tsfen: string }> {
  return get<{ ok: boolean; tsfen: string }>('/api/save');
}

/** Load a game from TSFEN */
export function loadGame(tsfen: string): Promise<{ ok: boolean; tsfen: string }> {
  return post<{ ok: boolean; tsfen: string }>('/api/load', { tsfen });
}

/** Get game status */
export function getStatus(): Promise<{ game_over: boolean; result: string | null; half_move: number; move_number: number; black_pieces: number; white_pieces: number; move_log: string[] }> {
  return get<{ game_over: boolean; result: string | null; half_move: number; move_number: number; black_pieces: number; white_pieces: number; move_log: string[] }>('/api/status');
}
