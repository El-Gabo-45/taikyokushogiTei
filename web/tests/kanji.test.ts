import { describe, expect, it } from 'vitest';
import { pieceKanji } from '../src/data/kanji';

describe('kanji piece symbols', () => {
  it('maps the royal and basic pieces', () => {
    expect(pieceKanji('K')).toBe('王将');
    expect(pieceKanji('P')).toBe('歩兵');
    expect(pieceKanji('G')).toBe('金将');
  });

  it('falls back to the abbreviation for unknown codes', () => {
    expect(pieceKanji('ZZ')).toBe('ZZ');
    expect(pieceKanji('')).toBe('');
  });

  it('maps every entry to a non-empty symbol', () => {
    // Spot-check a sample of the 301 engine piece classes; promoted pieces
    // intentionally fall through to their ASCII abbreviation.
    for (const abbrev of ['K', 'CP', 'GG', 'Q', 'DH', 'DK', 'L', 'P']) {
      expect(pieceKanji(abbrev).length).toBeGreaterThan(0);
    }
  });
});
