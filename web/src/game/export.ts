/**
 * Game-file export.
 *
 * JSON keeps the CLI's `GameFile` fields at the top level (`tsfen`, `moves`,
 * `side`, `move_number`) so a file exported from the browser can be loaded
 * with `taikyokushogi-cli` → `load <file.json>`. Extra metadata is ignored by
 * the CLI's serde struct.
 *
 * PGN is the human-readable/archival format (the one GUIs use for Stockfish
 * games). Taikyoku's 36×36 board and 301 piece types are not covered by the
 * PGN standard, so the move text uses coordinate notation and a `MoveFormat`
 * tag explains it. The PGN is meant for reading/sharing; use JSON to resume a
 * game in the CLI.
 */

import type { BoardState, EngineInfo } from '../types/board';

/** Marker that identifies a file written by the web GUI. */
export const GAME_FILE_FORMAT = 'taikyokushogi-game';
/** Version of the extra metadata (not of the CLI's `GameFile`). */
export const GAME_FILE_VERSION = 1;

/** Download formats offered by the move-log panel. */
export type GameFormat = 'json' | 'pgn';

/** JSON document downloaded by the "Export JSON" button. */
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

// ── PGN export ────────────────────────────────────────────────────

interface ParsedMove {
  side: 'b' | 'w';
  piece: string;
  from: [number, number];
  to: [number, number];
  promotion: boolean;
}

/** Matches the move-log lines produced by `record_move` in the Rust server. */
const MOVE_LOG_RE =
  /^(\d+)\.\s+(Black|White):\s+(\S+)\s+\((\d+),(\d+)\)-\((\d+),(\d+)\)(\+)?$/;

function parseMoveLogEntry(entry: string): ParsedMove | null {
  const match = MOVE_LOG_RE.exec(entry.trim());
  if (!match) return null;
  return {
    side: match[2] === 'Black' ? 'b' : 'w',
    piece: match[3],
    from: [Number(match[4]), Number(match[5])],
    to: [Number(match[6]), Number(match[7])],
    promotion: match[8] === '+',
  };
}

/** Columns 0–25 are files a–z; 26–35 are A–J (same as the CLI). */
function squareName([row, col]: [number, number]): string {
  const file =
    col < 26 ? String.fromCharCode(97 + col) : String.fromCharCode(65 + col - 26);
  return `${file}${row + 1}`;
}

/**
 * Full-move number for every ply. The engine increments `move_number` after
 * White's move, so the last ply's number can be derived from the current
 * snapshot and then walked back to the first logged ply.
 */
function pgnMoveNumbers(moves: ParsedMove[], board: BoardState): number[] {
  if (moves.length === 0) return [];
  const lastFull = board.move_number - (board.side_to_move === 0 ? 1 : 0);
  let full = lastFull;
  for (let i = 0; i < moves.length - 1; i += 1) {
    if (moves[i].side === 'w') full -= 1;
  }
  full = Math.max(1, full);

  const numbers: number[] = [];
  for (const move of moves) {
    numbers.push(full);
    if (move.side === 'w') full += 1;
  }
  return numbers;
}

function pgnMoveToken(move: ParsedMove): string {
  const suffix = move.promotion ? '+' : '';
  return `${move.piece}.${squareName(move.from)}-${squareName(move.to)}${suffix}`;
}

function wrapWords(text: string, width = 80): string {
  const words = text.split(/\s+/).filter(Boolean);
  const lines: string[] = [];
  let line = '';
  for (const word of words) {
    if (line.length === 0) {
      line = word;
    } else if (line.length + 1 + word.length <= width) {
      line += ` ${word}`;
    } else {
      lines.push(line);
      line = word;
    }
  }
  if (line.length > 0) lines.push(line);
  return lines.join('\n');
}

function pgnResult(result: string | null): string {
  switch (result) {
    case 'black_wins':
      return '1-0';
    case 'white_wins':
      return '0-1';
    case 'draw':
      return '1/2-1/2';
    default:
      return '*';
  }
}

function pgnDate(when: Date): string {
  const pad = (value: number): string => String(value).padStart(2, '0');
  return `${when.getFullYear()}.${pad(when.getMonth() + 1)}.${pad(when.getDate())}`;
}

function pgnTag(name: string, value: string): string {
  const escaped = value.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  return `[${name} "${escaped}"]`;
}

/** Build a PGN-style record. See the module comment for the format caveat. */
export function buildPgn(board: BoardState, options: BuildGameOptions = {}): string {
  const exportedAt = options.exportedAt ?? new Date();
  const engine = options.engine;
  const moves = board.move_log
    .map(parseMoveLogEntry)
    .filter((move): move is ParsedMove => move !== null);

  const result = pgnResult(board.game_result);
  const tags: Array<[string, string]> = [
    ['Event', 'Taikyoku Shogi - Web Game'],
    ['Site', 'Taikyoku Shogi Web GUI'],
    ['Date', pgnDate(exportedAt)],
    ['Round', '-'],
    ['White', '?'],
    ['Black', '?'],
    ['Result', result],
    ['Variant', 'Taikyoku Shogi'],
    ['BoardSize', '36x36'],
    ['MoveFormat', 'piece.from-to'],
    ['Mode', board.mode],
    ['PlyCount', String(moves.length)],
  ];
  if (engine) tags.push(['Engine', `${engine.name} ${engine.version}`]);
  if (board.game_result) tags.push(['Termination', board.game_result.replace(/_/g, ' ')]);

  const numbers = pgnMoveNumbers(moves, board);
  const parts: string[] = [];
  moves.forEach((move, index) => {
    const token = pgnMoveToken(move);
    if (move.side === 'b' || index === 0) {
      parts.push(`${numbers[index]}${move.side === 'b' ? '.' : '...'}`);
    }
    parts.push(token);
  });
  parts.push(result);

  const header = tags.map(([name, value]) => pgnTag(name, value)).join('\n');
  return `${header}\n\n${wrapWords(parts.join(' '))}\n`;
}

/** Timestamped download name, e.g. `taikyoku-game-20261110-153012-move-42.json`. */
export function gameFileName(
  when: Date,
  moveNumber: number,
  format: GameFormat = 'json',
): string {
  const pad = (value: number, width = 2): string => String(value).padStart(width, '0');
  const stamp =
    `${when.getFullYear()}${pad(when.getMonth() + 1)}${pad(when.getDate())}` +
    `-${pad(when.getHours())}${pad(when.getMinutes())}${pad(when.getSeconds())}`;
  return `taikyoku-game-${stamp}-move-${moveNumber}.${format}`;
}
