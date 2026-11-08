/** Wire types shared by the API client and the UI. */

/** Cell piece data returned by the API. */
export interface CellData {
  /** Engine abbreviation (ASCII). */
  piece: string;
  /** 0 = Black, 1 = White. */
  color: PieceColor;
  name: string;
  value: number;
  is_royal: boolean;
}

/** Full board state from `GET /api/state`. */
export interface BoardState {
  board: (CellData | null)[][];
  side_to_move: PieceColor;
  move_number: number;
  black_pieces: number;
  white_pieces: number;
  game_result: string | null;
  /** Monotonic server state counter. */
  version: number;
  /** Current position in TSFEN. */
  tsfen: string;
  /** Static evaluation from the side to move's perspective. */
  eval_score: number;
  /** Material balance from Black's perspective. */
  material_score: number;
  no_progress_plies: number;
  mode: string;
  move_log: string[];
  score_history: number[];
  game_over: boolean;
}

/** A legal destination for a selected piece. */
export interface MoveData {
  /** Origin; only present in full-position listings (`/api/legal-moves`). */
  from?: [number, number] | null;
  to: [number, number];
  promotion: boolean;
  is_igui: boolean;
  captured: string | null;
  /** Flat square index of the captured bystander for igui moves. */
  mid_sq?: number | null;
}

/** Search telemetry reported by `/api/ai-move` and `/api/search`. */
export interface SearchStats {
  depth: number;
  score: number;
  nodes: number;
  time_ms: number;
}

export interface LastMove {
  from: [number, number];
  to: [number, number];
}

export interface MoveResponse {
  ok: boolean;
  error?: string | null;
  board?: BoardState;
  last_move?: LastMove | null;
  search?: SearchStats | null;
}

export interface LegalMovesResponse {
  moves: MoveData[];
  count: number;
  side_to_move: PieceColor;
  game_over: boolean;
}

/** Piece database entry (`/api/pieces`, `/api/piece-info/:abbrev`). */
export interface PieceInfo {
  abbrev: string;
  name: string;
  value: number;
  promotes_to: string | null;
  slide_directions: number;
  jump_destinations: number;
  has_hook: boolean;
  area_steps: number;
  has_range_capture: boolean;
  has_igui: boolean;
  is_royal: boolean;
  betza: string | null;
}

export interface PieceListResponse {
  count: number;
  pieces: PieceInfo[];
}

/** Engine metadata (`/api/engine-info`). */
export interface EngineInfo {
  name: string;
  version: string;
  board_size: number;
  num_squares: number;
  piece_types: number;
  royal_piece_types: number;
  black_pieces: number;
  white_pieces: number;
  side_to_move: PieceColor;
  move_number: number;
  eval_backend: string;
  using_nnue: boolean;
  hash_mb: number;
  default_threads: number;
  max_threads: number;
  search_techniques: string[];
}

export interface EvalInfo {
  eval_score: number;
  material_score: number;
  black_pieces: number;
  white_pieces: number;
  side_to_move: PieceColor;
}

export interface StatusInfo {
  game_over: boolean;
  result: string | null;
  half_move: number;
  move_number: number;
  black_pieces: number;
  white_pieces: number;
  move_log: string[];
  tsfen: string;
  version: number;
}

export interface HistoryInfo {
  move_log: string[];
  half_move: number;
  game_over: boolean;
  score_history: number[];
}

export interface PositionInfo {
  tsfen: string;
  side_to_move: PieceColor;
  move_number: number;
}

export interface SaveInfo {
  ok: boolean;
  tsfen: string;
  move_log: string[];
  version: number;
}

/** Result of an analysis search (`POST /api/search`). */
export interface SearchResponse {
  ok: boolean;
  best_move?: MoveData | null;
  score: number;
  nodes: number;
  time_ms: number;
  depth: number;
  tsfen: string;
}

export interface UndoResponse {
  ok: boolean;
  error?: string | null;
  board?: BoardState;
  undone_plies: number;
}

export interface HealthInfo {
  ok: boolean;
  name: string;
  version: string;
  board_size: number;
  piece_types: number;
}

export type Square = [number, number];
export type PieceColor = 0 | 1;
