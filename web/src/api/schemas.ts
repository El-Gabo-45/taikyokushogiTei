/**
 * Runtime schemas for every response type in ../types/board.ts.
 *
 * The HTTP client validates each server payload against these schemas
 * before it reaches the UI, so malformed JSON fails fast with an
 * `ApiError` naming the offending field instead of poisoning the game
 * state with `undefined`s.
 *
 * Field names and optionality mirror the Rust structs in
 * `src/server/types.rs`: `Option<T>` serialised without
 * `skip_serializing_if` → `.nullable()`, with `skip_serializing_if`
 * → `.optional()`.
 */

import { z } from 'zod';
import type {
  BoardState,
  CellData,
  EngineInfo,
  EvalInfo,
  HealthInfo,
  HistoryInfo,
  LastMove,
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

/**
 * Tie a schema to its hand-written interface in types/board.ts.
 *
 * Zod's `.optional()` infers `prop?: T | undefined`, which is not
 * assignable to the interface's `prop?: T` under
 * `exactOptionalPropertyTypes` even though both read identically. This
 * cast is compile-time only (validation behaviour is untouched) and
 * records the schema ↔ interface pairing in one place.
 */
function checked<T>(schema: z.ZodType): z.ZodType<T> {
  return schema as z.ZodType<T>;
}

/** 0 = Black, 1 = White. */
const pieceColor = z.union([z.literal(0), z.literal(1)]);
/** `[row, col]`. */
const square = z.tuple([z.number(), z.number()]);

// ── /api/state (also embedded in move/undo/load responses) ─────────

export const cellDataSchema = checked<CellData>(
  z.object({
    piece: z.string(),
    color: pieceColor,
    name: z.string(),
    value: z.number(),
    is_royal: z.boolean(),
  }),
);

export const boardStateSchema = checked<BoardState>(
  z.object({
    board: z.array(z.array(cellDataSchema.nullable())),
    side_to_move: pieceColor,
    move_number: z.number(),
    black_pieces: z.number(),
    white_pieces: z.number(),
    game_result: z.string().nullable(),
    version: z.number(),
    tsfen: z.string(),
    eval_score: z.number(),
    material_score: z.number(),
    no_progress_plies: z.number(),
    mode: z.string(),
    move_log: z.array(z.string()),
    score_history: z.array(z.number()),
    game_over: z.boolean(),
  }),
);

// ── Moves ──────────────────────────────────────────────────────────

export const moveDataSchema = checked<MoveData>(
  z.object({
    // The server omits `from`/`mid_sq` entirely when absent.
    from: square.nullable().optional(),
    to: square,
    promotion: z.boolean(),
    is_igui: z.boolean(),
    captured: z.string().nullable(),
    mid_sq: z.number().nullable().optional(),
  }),
);

/** `GET /api/moves?r=&c=` response. */
export const moveListSchema = checked<{ moves: MoveData[]; from: [number, number] }>(
  z.object({ moves: z.array(moveDataSchema), from: square }),
);

export const legalMovesSchema = checked<LegalMovesResponse>(
  z.object({
    moves: z.array(moveDataSchema),
    count: z.number(),
    side_to_move: pieceColor,
    game_over: z.boolean(),
  }),
);

export const lastMoveSchema = checked<LastMove>(
  z.object({ from: square, to: square }),
);

export const searchStatsSchema = checked<SearchStats>(
  z.object({
    depth: z.number(),
    score: z.number(),
    nodes: z.number(),
    time_ms: z.number(),
  }),
);

export const moveResponseSchema = checked<MoveResponse>(
  z.object({
    ok: z.boolean(),
    error: z.string().nullable().optional(),
    board: boardStateSchema.nullable().optional(),
    last_move: lastMoveSchema.nullable().optional(),
    search: searchStatsSchema.nullable().optional(),
  }),
);

export const undoResponseSchema = checked<UndoResponse>(
  z.object({
    ok: z.boolean(),
    error: z.string().nullable().optional(),
    board: boardStateSchema.nullable().optional(),
    undone_plies: z.number(),
  }),
);

export const newGameSchema = checked<{ ok: boolean; version: number }>(
  z.object({ ok: z.boolean(), version: z.number() }),
);

// ── Analysis ───────────────────────────────────────────────────────

export const searchResponseSchema = checked<SearchResponse>(
  z.object({
    ok: z.boolean(),
    best_move: moveDataSchema.nullable().optional(),
    score: z.number(),
    nodes: z.number(),
    time_ms: z.number(),
    depth: z.number(),
    tsfen: z.string(),
  }),
);

// ── Position / persistence ─────────────────────────────────────────

export const positionInfoSchema = checked<PositionInfo>(
  z.object({ tsfen: z.string(), side_to_move: pieceColor, move_number: z.number() }),
);

export const saveInfoSchema = checked<SaveInfo>(
  z.object({
    ok: z.boolean(),
    tsfen: z.string(),
    move_log: z.array(z.string()),
    version: z.number(),
  }),
);

// ── Engine information ─────────────────────────────────────────────

export const pieceInfoSchema = checked<PieceInfo>(
  z.object({
    abbrev: z.string(),
    name: z.string(),
    value: z.number(),
    promotes_to: z.string().nullable(),
    slide_directions: z.number(),
    jump_destinations: z.number(),
    has_hook: z.boolean(),
    area_steps: z.number(),
    has_range_capture: z.boolean(),
    has_igui: z.boolean(),
    is_royal: z.boolean(),
    betza: z.string().nullable(),
  }),
);

export const pieceListSchema = checked<PieceListResponse>(
  z.object({ count: z.number(), pieces: z.array(pieceInfoSchema) }),
);

export const engineInfoSchema = checked<EngineInfo>(
  z.object({
    name: z.string(),
    version: z.string(),
    board_size: z.number(),
    num_squares: z.number(),
    piece_types: z.number(),
    royal_piece_types: z.number(),
    black_pieces: z.number(),
    white_pieces: z.number(),
    side_to_move: pieceColor,
    move_number: z.number(),
    eval_backend: z.string(),
    using_nnue: z.boolean(),
    hash_mb: z.number(),
    default_threads: z.number(),
    max_threads: z.number(),
    search_techniques: z.array(z.string()),
  }),
);

export const evalInfoSchema = checked<EvalInfo>(
  z.object({
    eval_score: z.number(),
    material_score: z.number(),
    black_pieces: z.number(),
    white_pieces: z.number(),
    side_to_move: pieceColor,
  }),
);

// ── Status / history / health ──────────────────────────────────────

export const statusInfoSchema = checked<StatusInfo>(
  z.object({
    game_over: z.boolean(),
    result: z.string().nullable(),
    half_move: z.number(),
    move_number: z.number(),
    black_pieces: z.number(),
    white_pieces: z.number(),
    move_log: z.array(z.string()),
    tsfen: z.string(),
    version: z.number(),
  }),
);

export const historyInfoSchema = checked<HistoryInfo>(
  z.object({
    move_log: z.array(z.string()),
    half_move: z.number(),
    game_over: z.boolean(),
    score_history: z.array(z.number()),
  }),
);

export const healthInfoSchema = checked<HealthInfo>(
  z.object({
    ok: z.boolean(),
    name: z.string(),
    version: z.string(),
    board_size: z.number(),
    piece_types: z.number(),
  }),
);
