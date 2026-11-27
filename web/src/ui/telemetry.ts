/** Pure helpers for the diagnostics panel's search-telemetry chart. */

import type { SearchStats } from '../types/board';

/** Human-readable one-line summary of the most recent search. */
export function describeSearch(s: SearchStats): string {
  const nps = Math.round((s.nodes / Math.max(1, s.time_ms)) * 1000).toLocaleString();
  return (
    `d${s.depth} score ${s.score} · ${s.nodes.toLocaleString()} nodes / ${s.time_ms} ms · ` +
    `${nps} nps · TT ${(s.hashfull / 10).toFixed(1)}%`
  );
}

/** Normalised series in [-1, 1] for drawing: score, nps and TT occupancy. */
export function telemetrySeries(history: SearchStats[]): number[][] {
  if (history.length === 0) return [[], [], []];
  const maxScore = Math.max(1, ...history.map((s) => Math.abs(s.score)));
  const maxNps = Math.max(
    1,
    ...history.map((s) => s.nodes / Math.max(1, s.time_ms)),
  );
  return [
    history.map((s) => s.score / maxScore),
    history.map((s) => s.nodes / Math.max(1, s.time_ms) / maxNps),
    history.map((s) => s.hashfull / 1000),
  ];
}
