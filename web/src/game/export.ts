/**
 * Game-file export.
 *
 * The download is plain JSON and deliberately keeps the CLI's `GameFile`
 * fields at the top level (`tsfen`, `moves`, `side`, `move_number`) so a file
 * exported from the browser can be loaded with `taikyokushogi-cli` →
 * `load <file.json>`. Extra metadata is ignored by the CLI's serde struct.
 */

import type { BoardState, EngineInfo } from '../types/board';

/** Marker that identifies a file written by the web GUI. */
export const GAME_FILE_FORMAT = 'taikyokushogi-game';
/** Version of the extra metadata (not of the CLI's `GameFile`). */
export const GAME_FILE_VERSION = 1;

/** JSON document downloaded by the "Export game (.json)" button. */
export interface GameFile {
  format: typeof GAME_FILE_FORMAT;
  version: typeof GAME_FILE_VERSION;
  /** ISO-8601 timestamp of the export. */
  exported_at: string;
  app: 'taikyokushogi-web';
  /** Engine that produced the game, when its metadata is available. */
  engine: { name: string; version: string } | null;

  // ── CLI-compatible game record ──────────────────────────────────
  /** Current position in TSFEN. */
  tsfen: string;
  /** Move list in CLI notation (one entry per played ply). */
  moves: string[];
  /** Side to move ("b" / "w"), as expected by the CLI. */
  side: 'b' | 'w';
  /** Full-move number, as expected by the CLI. */
  move_number: number;

  // ── Extra metadata (ignored by the CLI) ─────────────────────────
  /** Game result as reported by the server, or null while playing. */
  result: string | null;
  black_pieces: number;
  white_pieces: number;
  no_progress_plies: number;
}

export interface BuildGameOptions {
  /** Timestamp used for `exported_at`; defaults to now. */
  exportedAt?: Date;
  /** Engine metadata from the state, if it has been fetched. */
  engine?: EngineInfo | null;
}

/** Build the JSON-serialisable game record from a board snapshot. */
export function buildGameFile(board: BoardState, options: BuildGameOptions = {}): GameFile {
  const exportedAt = options.exportedAt ?? new Date();
  const engine = options.engine;

  return {
    format: GAME_FILE_FORMAT,
    version: GAME_FILE_VERSION,
    exported_at: exportedAt.toISOString(),
    app: 'taikyokushogi-web',
    engine: engine ? { name: engine.name, version: engine.version } : null,

    tsfen: board.tsfen,
    moves: [...board.move_log],
    side: board.side_to_move === 0 ? 'b' : 'w',
    move_number: board.move_number,

    result: board.game_result,
    black_pieces: board.black_pieces,
    white_pieces: board.white_pieces,
    no_progress_plies: board.no_progress_plies,
  };
}

/** Pretty-print a game record with a trailing newline (text-file friendly). */
export function serializeGameFile(file: GameFile): string {
  return `${JSON.stringify(file, null, 2)}\n`;
}

/** Timestamped download name, e.g. `taikyoku-game-20261110-153012-move-42.json`. */
export function gameFileName(when: Date, moveNumber: number): string {
  const pad = (value: number, width = 2): string => String(value).padStart(width, '0');
  const stamp =
    `${when.getFullYear()}${pad(when.getMonth() + 1)}${pad(when.getDate())}` +
    `-${pad(when.getHours())}${pad(when.getMinutes())}${pad(when.getSeconds())}`;
  return `taikyoku-game-${stamp}-move-${moveNumber}.json`;
}
