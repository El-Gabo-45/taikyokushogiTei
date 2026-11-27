import { describe, expect, it } from 'vitest';
import { describeSearch, telemetrySeries } from '../src/ui/telemetry';
import type { SearchStats } from '../src/types/board';

const s = (over: Partial<SearchStats> = {}): SearchStats => ({
  depth: 3,
  score: 500,
  nodes: 10000,
  time_ms: 500,
  hashfull: 250,
  ...over,
});

describe('telemetry helpers', () => {
  it('summarises a search with score, nps and TT fill', () => {
    const text = describeSearch(s());
    expect(text).toContain('d3');
    expect(text).toContain('score 500');
    expect(text).toContain('20,000 nps'); // 10000 / 0.5 s
    expect(text).toContain('TT 25.0%');
  });

  it('normalises score, nps and hashfull into [-1, 1]', () => {
    const [score, nps, tt] = telemetrySeries([s(), s({ score: -500, nodes: 5000 })]);
    expect(score[0]).toBeCloseTo(1);
    expect(score[1]).toBeCloseTo(-1);
    expect(nps[0]).toBeCloseTo(1);
    expect(nps[1]).toBeCloseTo(0.5);
    expect(tt[0]).toBeCloseTo(0.25);
    expect(tt[1]).toBeCloseTo(0.25);
  });

  it('returns three empty series for no searches', () => {
    expect(telemetrySeries([])).toEqual([[], [], []]);
  });
});
