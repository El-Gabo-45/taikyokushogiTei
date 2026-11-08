import { describe, expect, it } from 'vitest';
import {
  BOARD_SIZE,
  MAX_ZOOM,
  MIN_ZOOM,
  chartPoints,
  clampZoom,
  describeResult,
  formatNodes,
  formatScore,
  fromViewSquare,
  isOnBoard,
  opponent,
  sideName,
  toViewSquare,
} from '../src/game/view';

describe('board orientation', () => {
  it('rotates 180° when flipped and back', () => {
    expect(toViewSquare([0, 0], true)).toEqual([35, 35]);
    expect(toViewSquare([0, 35], true)).toEqual([35, 0]);
    expect(toViewSquare([12, 7], true)).toEqual([23, 28]);
    expect(toViewSquare([12, 7], false)).toEqual([12, 7]);
  });

  it('round-trips view coordinates', () => {
    for (const square of [[0, 0], [5, 30], [35, 35]] as Array<[number, number]>) {
      expect(fromViewSquare(toViewSquare(square, true), true)).toEqual(square);
      expect(fromViewSquare(toViewSquare(square, false), false)).toEqual(square);
    }
  });

  it('validates squares against the 36x36 board', () => {
    expect(isOnBoard(0, 0)).toBe(true);
    expect(isOnBoard(35, 35)).toBe(true);
    expect(isOnBoard(36, 0)).toBe(false);
    expect(isOnBoard(0, 36)).toBe(false);
    expect(isOnBoard(-1, 0)).toBe(false);
    expect(isOnBoard(1.5, 2)).toBe(false);
    expect(BOARD_SIZE).toBe(36);
  });
});

describe('zoom', () => {
  it('clamps to the supported range', () => {
    expect(clampZoom(1)).toBe(1);
    expect(clampZoom(0.1)).toBe(MIN_ZOOM);
    expect(clampZoom(10)).toBe(MAX_ZOOM);
    expect(clampZoom(Number.NaN)).toBe(1);
  });
});

describe('formatting', () => {
  it('formats scores with sign', () => {
    expect(formatScore(12)).toBe('+12');
    expect(formatScore(-7)).toBe('-7');
    expect(formatScore(0)).toBe('0');
  });

  it('formats node counts', () => {
    expect(formatNodes(999)).toBe('999');
    expect(formatNodes(1234)).toBe('1.2k');
    expect(formatNodes(2_500_000)).toBe('2.5M');
  });

  it('describes results and sides', () => {
    expect(describeResult('black_wins')).toBe('Black wins');
    expect(describeResult('white_wins')).toBe('White wins');
    expect(describeResult('draw')).toBe('Draw');
    expect(describeResult(null)).toBe('');
    expect(sideName(0)).toBe('Black');
    expect(sideName(1)).toBe('White');
    expect(opponent(0)).toBe(1);
    expect(opponent(1)).toBe(0);
  });
});

describe('material chart', () => {
  it('returns no points for an empty history', () => {
    expect(chartPoints([], 100, 50)).toEqual([]);
  });

  it('centres a zero score and keeps values inside the box', () => {
    const points = chartPoints([0, 100, -100, 0], 100, 50, 5);
    expect(points).toHaveLength(4);
    for (const p of points) {
      expect(p.x).toBeGreaterThanOrEqual(5);
      expect(p.x).toBeLessThanOrEqual(95);
      expect(p.y).toBeGreaterThanOrEqual(5);
      expect(p.y).toBeLessThanOrEqual(45);
    }
    // Zero sits on the horizontal centre line.
    expect(points[0].y).toBe(25);
    // Positive extremes are above the line, negative below.
    expect(points[1].y).toBeLessThan(25);
    expect(points[2].y).toBeGreaterThan(25);
  });

  it('handles a single sample', () => {
    const points = chartPoints([42], 100, 50);
    expect(points).toHaveLength(1);
    expect(points[0].x).toBe(3);
  });
});
