# Rules of Taikyoku Shogi

Complete rules of the variant as implemented by this engine, including the
notes on how the rules map to code. For the movement of every single piece,
see [PIECES.md](../PIECES.md).

## Objective

Capture all of your opponent's **royal pieces** (King and Crown Prince). The game ends immediately when a player's last royal piece is captured. Unlike standard shogi, there is no check or checkmate — you may move your King into danger.

## Turns

Black (先手) moves first. Players alternate moving exactly one piece per turn. Passing is not allowed. Captured pieces are permanently removed (no drops).

## Movement

Pieces move orthogonally or diagonally. Most pieces have unique movement patterns combining:

| Type | Description |
| ------ | ------------- |
| **Step** | Move exactly 1 square |
| **Slide** | Move any number of empty squares in a line |
| **Limited** | Move up to N squares in a line (2-7) |
| **Jump** | Leap to a square, bypassing intervening pieces |
| **Hook** | Slide along a line, optionally turn 90 degrees once, then continue sliding |
| **Area** | Take multiple steps per turn (Lion-type moves) |
| **Range capture** | Fly over pieces along a line, capturing all of lower rank |
| **Igui** | Capture a neighboring piece without moving to its square (written `c5c5`, i.e. `from` = `to`) |

See [PIECES.md](../PIECES.md) for the complete movement reference for every piece.

## Promotion

The **promotion zone** is the 11 rows on the opponent's side of the board.

- A piece **may promote** when it moves into the promotion zone from outside, or when it captures an opponent's piece while inside the zone.
- A piece that entered the zone without promoting may only promote later by capturing inside the zone, or by leaving and re-entering.
- Promotion is **optional**, except for forward-only pieces (Pawn, Stone General, Iron General, Dog, Wood General, Incense Chariot, Ox Chariot, Fierce Tiger) which **must promote** upon reaching the farthest rank.
- Promotion is permanent — a piece cannot revert to its unpromoted form.

## Range Capture Ranking

Range-capturing pieces — Great General, Vice General, Flying General,
Angle General, Fierce Dragon and Flying Crocodile (the only six pieces with
`range_capture` rays) — fly over pieces along a line and capture every
occupied square up to their landing square, **but only pieces of strictly
lower rank**. The engine models five tiers (`RANK_*` in `src/types/constants.rs`);
a piece can range-capture everything in the tiers *below* its own:

1. King, Crown Prince (`RANK_ROYAL`) — can never be range-captured
2. Great General (`RANK_GREAT`)
3. Vice General (`RANK_VICE`)
4. Flying General, Angle General, Fierce Dragon, Flying Crocodile (`RANK_RANGE_CAP`)
5. all remaining pieces (`RANK_NORMAL`)

Pieces of the same rank cannot take each other by range capture (e.g. one
Great General cannot capture another), the ray **stops** at the first piece
that cannot be captured (nothing can fly over a King), and ordinary —
non-range — captures ignore ranks entirely.

## Draw by No Progress (500-Move Rule)

The game is automatically drawn if **500 consecutive full moves** (1,000 plies) pass with neither player making a capture nor a promotion. The counter resets whenever a piece is captured or a promotion occurs. A game in which **both** sides lose all their royal pieces (e.g. mutual captures in the same exchange) is also a draw.

## Initial Setup

Each side's 402 pieces occupy 12 ranks. Black occupies the bottom of the board (rows 25-36); White mirrors from the top (rows 1-12). The King sits at the center of the back rank (column 18), flanked by the Crown Prince (column 19).

## Engine Implementation Notes (variant rules vs. code)

- **No check filtering.** `legal_moves()` never disallows leaving a royal
  piece en prise — the variant has no check rule, so you may move your King
  into danger (`generate_legal_moves` is generation-only; the search avoids
  hanging royals through their material value: King = 100,000, Crown Prince
  = 50,000).
- **No stalemate-win rule.** When a side has no legal move: the search
  scores it as a loss for the side to move, engine-vs-engine games
  (`selfplay` / `match` / `sprt`) record it as a **draw**, and the web
  server simply ends the game.
- **Draws** come from the 500-move no-progress rule or from both sides
  losing all royal pieces (see above).
