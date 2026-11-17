//! Python bindings via PyO3 (enabled with the `python` feature).

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};

use crate::types::*;
use crate::{board, eval, movegen, pieces, search};

/// Validate a color parameter (`0` = black, `1` = white) and return its index
/// into the per-color tables. Raises `ValueError` for any other value instead
/// of letting it index out of bounds.
fn color_index(color: u8) -> PyResult<usize> {
    if color > 1 {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "color must be 0 (black) or 1 (white), got {color}"
        )));
    }
    Ok(color as usize)
}

/// Validate row/column parameters and return the board index. Raises
/// `IndexError` for out-of-range coordinates instead of panicking inside
/// [`sq_index`].
fn checked_sq(row: usize, col: usize) -> PyResult<u16> {
    if row >= BOARD_SIZE || col >= BOARD_SIZE {
        return Err(pyo3::exceptions::PyIndexError::new_err(format!(
            "row {row}, col {col} outside the board (rows and columns are 0..{BOARD_SIZE})"
        )));
    }
    Ok(sq_index(row, col) as u16)
}

/// A loaded NNUE network (TKN1 file). Load it once and share it between
/// boards: `board.set_network(net)`.
#[pyclass]
#[derive(Clone)]
pub struct PyNetwork {
    inner: std::sync::Arc<crate::Network>,
}

#[pymethods]
impl PyNetwork {
    /// Load a network file; raises `ValueError` with the reason on any problem
    /// (it never falls back to random weights).
    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        crate::Network::load(path)
            .map(|n| PyNetwork {
                inner: std::sync::Arc::new(n),
            })
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Number of piece classes of the network.
    #[getter]
    fn num_classes(&self) -> usize {
        self.inner.num_classes()
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyBoard {
    inner: board::Board,
}

#[pymethods]
impl PyBoard {
    #[new]
    fn new() -> Self {
        PyBoard {
            inner: board::Board::new(),
        }
    }

    fn setup_initial(&mut self) {
        self.inner.setup_initial();
    }

    /// Attach an NNUE network (`PyNetwork`) or detach it with `None`.
    /// `evaluate`/`search` then use it instead of the hand-crafted evaluator.
    #[pyo3(signature = (network=None))]
    fn set_network(&mut self, network: Option<PyRef<'_, PyNetwork>>) {
        self.inner.set_network(network.map(|n| n.inner.clone()));
    }

    /// Whether an NNUE network is attached.
    fn has_network(&self) -> bool {
        self.inner.network().is_some()
    }

    /// Piece at `(row, col)` as `(abbrev, color)`, or `None` when empty.
    /// Raises `IndexError` for out-of-range coordinates.
    fn at(&self, row: usize, col: usize) -> PyResult<Option<(String, u8)>> {
        let sq = checked_sq(row, col)? as usize;
        let cell = self.inner.cells[sq];
        if cell == EMPTY_CELL {
            Ok(None)
        } else {
            Ok(Some((
                pieces::abbrev(cell_piece(cell)).to_string(),
                cell_color(cell),
            )))
        }
    }

    #[getter]
    fn side_to_move(&self) -> u8 {
        self.inner.side_to_move
    }

    #[getter]
    fn move_number(&self) -> u32 {
        self.inner.move_number
    }

    fn black_piece_count(&self) -> usize {
        self.inner.piece_count[BLACK as usize]
    }
    fn white_piece_count(&self) -> usize {
        self.inner.piece_count[WHITE as usize]
    }

    fn game_result(&self) -> Option<String> {
        self.inner.game_result().map(|r| match r {
            GameResult::BlackWins => "black_wins".into(),
            GameResult::WhiteWins => "white_wins".into(),
            GameResult::Draw => "draw".into(),
        })
    }

    /// Mapping `(row, col) -> abbrev` for every piece of `color` (0 = black,
    /// 1 = white). Raises `ValueError` for any other color.
    fn piece_positions(&self, py: Python, color: u8) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        let c = color_index(color)?;
        for i in 0..self.inner.piece_list_len[c] {
            let sq = self.inner.piece_list[c][i];
            if sq == INVALID_SQ {
                continue;
            }
            let cell = self.inner.cells[sq as usize];
            if cell == EMPTY_CELL {
                continue;
            }
            let key = PyTuple::new(py, [sq as usize / BOARD_SIZE, sq as usize % BOARD_SIZE])?;
            dict.set_item(key, pieces::abbrev(cell_piece(cell)))?;
        }
        Ok(dict.into_any().unbind())
    }

    /// List of `(row, col)` tuples for the royal pieces (kings) of `color`.
    /// Raises `ValueError` for any other color.
    fn royal_positions(&self, py: Python, color: u8) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        let c = color_index(color)?;
        for i in 0..self.inner.royal_count[c] {
            let sq = self.inner.royal_list[c][i];
            list.append(PyTuple::new(
                py,
                [sq as usize / BOARD_SIZE, sq as usize % BOARD_SIZE],
            )?)?;
        }
        Ok(list.into_any().unbind())
    }

    fn score(&self) -> i32 {
        eval::material_score(&self.inner)
    }

    fn to_tsfen(&self) -> String {
        crate::tsfen::to_tsfen(&self.inner)
    }

    #[staticmethod]
    fn from_tsfen(tsfen: &str) -> PyResult<Self> {
        crate::tsfen::from_tsfen(tsfen)
            .map(|inner| PyBoard { inner })
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e))
    }

    /// Apply the first legal move matching `(from_r, from_c) -> (to_r, to_c)`
    /// with the given promotion/igui flags and optional middle square (igui
    /// moves). Raises `IndexError` for out-of-range coordinates; returns
    /// `false` when no legal move matches (the board is left unchanged).
    fn apply_move_py(
        &mut self,
        from_r: usize,
        from_c: usize,
        to_r: usize,
        to_c: usize,
        promotion: bool,
        mid_sq: Option<usize>,
        is_igui: bool,
    ) -> PyResult<bool> {
        let from_sq = checked_sq(from_r, from_c)?;
        let to_sq = checked_sq(to_r, to_c)?;
        if let Some(ms) = mid_sq.filter(|&ms| ms >= BOARD_SIZE * BOARD_SIZE) {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "mid_sq {ms} outside the board (squares are 0..{})",
                BOARD_SIZE * BOARD_SIZE
            )));
        }
        for m in &movegen::generate_legal_moves(&self.inner) {
            let mid_match = mid_sq.map_or(true, |ms| m.mid_sq as usize == ms);
            if m.from_sq == from_sq
                && m.to_sq == to_sq
                && m.promotion == promotion
                && m.is_igui == is_igui
                && mid_match
            {
                self.inner.apply_move(m);
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn undo(&mut self) -> bool {
        self.inner.undo_move()
    }

    fn legal_moves_py(&self, py: Python) -> PyResult<Py<PyAny>> {
        let moves = movegen::generate_legal_moves(&self.inner);
        let list = PyList::empty(py);
        for m in &moves {
            let d = PyDict::new(py);
            d.set_item(
                "from",
                vec![
                    m.from_sq as usize / BOARD_SIZE,
                    m.from_sq as usize % BOARD_SIZE,
                ],
            )?;
            d.set_item(
                "to",
                vec![m.to_sq as usize / BOARD_SIZE, m.to_sq as usize % BOARD_SIZE],
            )?;
            d.set_item("promotion", m.promotion)?;
            d.set_item("is_igui", m.is_igui)?;
            d.set_item(
                "captured",
                if m.captured_piece != 0 {
                    Some(pieces::abbrev(m.captured_piece))
                } else {
                    None
                },
            )?;
            list.append(d)?;
        }
        Ok(list.into_any().unbind())
    }

    /// Legal moves departing from `(r, c)`, deduplicated by destination.
    /// Raises `IndexError` for out-of-range coordinates.
    fn moves_from_py(&self, py: Python, r: usize, c: usize) -> PyResult<Py<PyAny>> {
        let sq = checked_sq(r, c)?;
        let moves = movegen::generate_legal_moves(&self.inner);
        let list = PyList::empty(py);
        let mut seen = std::collections::HashSet::new();
        for m in &moves {
            if m.from_sq == sq {
                let key = (m.to_sq, m.promotion, m.is_igui);
                if seen.insert(key) {
                    let d = PyDict::new(py);
                    d.set_item(
                        "to",
                        vec![m.to_sq as usize / BOARD_SIZE, m.to_sq as usize % BOARD_SIZE],
                    )?;
                    d.set_item("promotion", m.promotion)?;
                    d.set_item("is_igui", m.is_igui)?;
                    d.set_item(
                        "captured",
                        if m.captured_piece != 0 {
                            Some(pieces::name(m.captured_piece))
                        } else {
                            None
                        },
                    )?;
                    list.append(d)?;
                }
            }
        }
        Ok(list.into_any().unbind())
    }

    fn random_move_py(&self) -> Option<(usize, usize, usize, usize, bool)> {
        let moves = movegen::generate_legal_moves(&self.inner);
        if moves.is_empty() {
            return None;
        }
        use rand::Rng;
        let idx = rand::thread_rng().gen_range(0..moves.len());
        let m = &moves[idx];
        Some((
            m.from_sq as usize / BOARD_SIZE,
            m.from_sq as usize % BOARD_SIZE,
            m.to_sq as usize / BOARD_SIZE,
            m.to_sq as usize % BOARD_SIZE,
            m.promotion,
        ))
    }

    fn search_py(
        &mut self,
        py: Python,
        depth: u32,
        time_limit_ms: u64,
    ) -> (Option<(usize, usize, usize, usize, bool)>, i32, u64, u64) {
        let result = py.detach(|| search::search(&mut self.inner, depth, time_limit_ms));
        let mv = result.best_move.map(|m| {
            (
                m.from_sq as usize / BOARD_SIZE,
                m.from_sq as usize % BOARD_SIZE,
                m.to_sq as usize / BOARD_SIZE,
                m.to_sq as usize % BOARD_SIZE,
                m.promotion,
            )
        });
        (mv, result.score, result.nodes, result.time_ms)
    }

    fn display(&self) -> String {
        self.inner.display()
    }
}

#[pyfunction]
fn piece_name_py(abbrev: &str) -> String {
    pieces::find_by_abbrev(abbrev)
        .map(|pt| pieces::name(pt).to_string())
        .unwrap_or_else(|| abbrev.to_string())
}

#[pyfunction]
fn piece_value_py(abbrev: &str) -> PyResult<i32> {
    pieces::find_by_abbrev(abbrev)
        .map(|pt| pieces::value(pt))
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err(format!("unknown piece: {abbrev}")))
}

#[pyfunction]
fn piece_promotes_to_py(abbrev: &str) -> Option<String> {
    pieces::find_by_abbrev(abbrev)
        .and_then(|pt| pieces::promotes_to(pt))
        .map(|p| pieces::abbrev(p).to_string())
}

#[pyfunction]
fn piece_info_py(py: Python, abbrev: &str) -> PyResult<Py<PyAny>> {
    let d = PyDict::new(py);
    d.set_item("abbrev", abbrev)?;
    if let Some(pt) = pieces::find_by_abbrev(abbrev) {
        d.set_item("name", pieces::name(pt))?;
        d.set_item("value", pieces::value(pt))?;
        d.set_item(
            "promotes_to",
            pieces::promotes_to(pt).map(|p| pieces::name(p).to_string()),
        )?;
        let mv = pieces::movement(pt);
        d.set_item("slide_directions", mv.slides.len())?;
        d.set_item("jump_destinations", mv.jumps.len())?;
        let mut sp: Vec<&str> = Vec::new();
        if mv.hook.is_some() {
            sp.push("hook");
        }
        if mv.area > 0 {
            sp.push("area");
        }
        if !mv.range_capture.is_empty() {
            sp.push("range capture");
        }
        if mv.igui {
            sp.push("igui");
        }
        d.set_item("specials", sp)?;
    } else {
        d.set_item("name", abbrev)?;
        d.set_item("value", 0)?;
        d.set_item("promotes_to", py.None())?;
        d.set_item("slide_directions", 0)?;
        d.set_item("jump_destinations", 0)?;
        let empty: Vec<&str> = vec![];
        d.set_item("specials", empty)?;
    }
    Ok(d.into_any().unbind())
}

/// Register all Python-visible items on the module.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyBoard>()?;
    m.add_class::<PyNetwork>()?;
    m.add_function(pyo3::wrap_pyfunction!(piece_name_py, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(piece_value_py, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(piece_promotes_to_py, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(piece_info_py, m)?)?;
    m.add("BOARD_SIZE", BOARD_SIZE)?;
    m.add("BLACK", BLACK)?;
    m.add("WHITE", WHITE)?;
    Ok(())
}
