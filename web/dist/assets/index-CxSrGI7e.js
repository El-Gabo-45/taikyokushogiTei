//#region \0vite/modulepreload-polyfill.js
(function polyfill() {
	const relList = document.createElement("link").relList;
	if (relList && relList.supports && relList.supports("modulepreload")) return;
	for (const link of document.querySelectorAll("link[rel=\"modulepreload\"]")) processPreload(link);
	new MutationObserver((mutations) => {
		for (const mutation of mutations) {
			if (mutation.type !== "childList") continue;
			for (const node of mutation.addedNodes) if (node.tagName === "LINK" && node.rel === "modulepreload") processPreload(node);
		}
	}).observe(document, {
		childList: true,
		subtree: true
	});
	function getFetchOpts(link) {
		const fetchOpts = {};
		if (link.integrity) fetchOpts.integrity = link.integrity;
		if (link.referrerPolicy) fetchOpts.referrerPolicy = link.referrerPolicy;
		if (link.crossOrigin === "use-credentials") fetchOpts.credentials = "include";
		else if (link.crossOrigin === "anonymous") fetchOpts.credentials = "omit";
		else fetchOpts.credentials = "same-origin";
		return fetchOpts;
	}
	function processPreload(link) {
		if (link.ep) return;
		link.ep = true;
		const fetchOpts = getFetchOpts(link);
		fetch(link.href, fetchOpts);
	}
})();
//#endregion
//#region src/api/client.ts
var BASE = "";
var ApiError = class extends Error {
	status;
	constructor(message, status) {
		super(message);
		this.name = "ApiError";
		this.status = status;
	}
};
async function request(path, init) {
	const res = await fetch(`${BASE}${path}`, {
		headers: {
			Accept: "application/json",
			...init?.body ? { "Content-Type": "application/json" } : {}
		},
		...init
	});
	let payload = null;
	const text = await res.text();
	if (text) try {
		payload = JSON.parse(text);
	} catch {
		payload = null;
	}
	if (!res.ok) throw new ApiError(payload?.error ?? `HTTP ${res.status} ${res.statusText} (${path})`, res.status);
	return payload;
}
var get = (path) => request(path);
var post = (path, body) => request(path, {
	method: "POST",
	body: body === void 0 ? void 0 : JSON.stringify(body)
});
/** Fetch the full board state. */
function fetchState() {
	return get("/api/state");
}
/** Legal moves for the piece on `(r, c)`. */
function fetchMoves(r, c) {
	return get(`/api/moves?r=${r}&c=${c}`);
}
/** Start a new game. */
function newGame() {
	return post("/api/new-game");
}
/** Apply a human move. */
function sendMove(from, to, promotion = false, mid_sq) {
	return post("/api/move", {
		from,
		to,
		promotion,
		mid_sq: mid_sq ?? null
	});
}
/** Ask the AI to search and apply a move. `depth = 0` means random. */
function sendAiMove(depth = 2, timeLimit = 3e4) {
	return post("/api/ai-move", {
		depth,
		time_limit: timeLimit
	});
}
/** Analysis search — does not mutate the board. */
function analyze(depth = 2, timeLimit = 1e4) {
	return post("/api/search", {
		depth,
		time_limit: timeLimit
	});
}
/** Undo the last `plies` moves (default 1). */
function undo(plies = 1) {
	return post("/api/undo", { plies });
}
function saveGame() {
	return get("/api/save");
}
function loadGame(tsfen) {
	return post("/api/load", { tsfen });
}
/** Piece database entry by abbreviation; throws `ApiError` 404 when unknown. */
function pieceInfo(abbrev) {
	return get(`/api/piece-info/${encodeURIComponent(abbrev)}`);
}
/** Engine metadata: sizes, backend, hash size, search techniques. */
function fetchEngineInfo() {
	return get("/api/engine-info");
}
var MIN_ZOOM = .7;
var MAX_ZOOM = 1.8;
/** Map a board square to its on-screen square (flip = 180° rotation). */
function toViewSquare([r, c], flip) {
	return flip ? [35 - r, 35 - c] : [r, c];
}
/** Map an on-screen square back to the engine's board coordinates. */
function fromViewSquare([vr, vc], flip) {
	return flip ? [35 - vr, 35 - vc] : [vr, vc];
}
function clampZoom(zoom) {
	if (!Number.isFinite(zoom)) return 1;
	return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
}
function sideName(color) {
	return color === 0 ? "Black" : "White";
}
/** Human-readable text for the `game_result` field. */
function describeResult(result) {
	switch (result) {
		case "black_wins": return "Black wins";
		case "white_wins": return "White wins";
		case "draw": return "Draw";
		default: return "";
	}
}
/** "+12" / "-7" / "0". */
function formatScore(score) {
	return score > 0 ? `+${score}` : String(score);
}
/** Compact node counts: 12, 4.3k, 1.2M. */
function formatNodes(nodes) {
	if (nodes >= 1e6) return `${(nodes / 1e6).toFixed(1)}M`;
	if (nodes >= 1e3) return `${(nodes / 1e3).toFixed(1)}k`;
	return String(nodes);
}
/** Convert the material score history into chart coordinates. */
function chartPoints(values, width, height, padding = 3) {
	if (values.length === 0) return [];
	const maxAbs = Math.max(1, ...values.map((v) => Math.abs(v)));
	const usableW = Math.max(1, width - padding * 2);
	const usableH = Math.max(1, height - padding * 2);
	const step = values.length > 1 ? usableW / (values.length - 1) : 0;
	return values.map((v, i) => ({
		x: padding + i * step,
		y: padding + usableH / 2 - v / maxAbs * (usableH / 2)
	}));
}
//#endregion
//#region src/game/state.ts
var AI_DELAY_MS = 120;
var sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
var GameState = class {
	board = null;
	selectedSq = null;
	legalMoves = [];
	lastMoveFrom = null;
	lastMoveTo = null;
	settings = {
		mode: "human_vs_ai",
		humanColor: 0,
		aiDepth: 2,
		useKanji: true,
		flip: false,
		zoom: 1,
		autoPlay: false
	};
	/** Engine metadata, fetched once at startup. */
	engineInfo = null;
	/** Latest static evaluation (embedded in every board snapshot). */
	evalInfo = null;
	/** Telemetry of the most recent search. */
	lastSearch = null;
	/** Piece the user clicked, with database details. */
	inspection = null;
	/** Transient user-facing message. */
	message = null;
	/** True while a network request is in flight. */
	busy = false;
	listeners = /* @__PURE__ */ new Set();
	pieceCache = /* @__PURE__ */ new Map();
	aiPending = false;
	autoLoopActive = false;
	subscribe(fn) {
		this.listeners.add(fn);
		return () => {
			this.listeners.delete(fn);
		};
	}
	notify() {
		for (const fn of this.listeners) fn();
	}
	setMessage(text, kind = "info") {
		this.message = {
			text,
			kind
		};
	}
	/** Public helper so UI actions (copy, download…) can surface feedback. */
	announce(text, kind = "info") {
		this.setMessage(text, kind);
		this.notify();
	}
	/** Enable/disable the auto-play loop and kick it when enabling. */
	setAutoPlay(on) {
		this.settings.autoPlay = on;
		this.notify();
		if (on) this.kickAutoLoop();
	}
	/** Update derived eval info from a fresh board snapshot. */
	syncEval() {
		if (!this.board) return;
		this.evalInfo = {
			eval_score: this.board.eval_score,
			material_score: this.board.material_score,
			black_pieces: this.board.black_pieces,
			white_pieces: this.board.white_pieces,
			side_to_move: this.board.side_to_move
		};
	}
	applyBoard(board) {
		this.board = board;
		this.inspection = null;
		this.syncEval();
		if (board.game_over) {
			this.settings.autoPlay = false;
			const result = board.game_result ? board.game_result.replace("_", " ") : "game over";
			this.setMessage(`Game over — ${result}`, "success");
		}
		this.notify();
	}
	/** Initialise or reload the board from the backend. */
	async init() {
		this.busy = true;
		this.notify();
		try {
			const [board, engine] = await Promise.all([fetchState(), fetchEngineInfo().catch(() => null)]);
			if (engine) this.engineInfo = engine;
			this.selectedSq = null;
			this.legalMoves = [];
			this.lastMoveFrom = null;
			this.lastMoveTo = null;
			this.lastSearch = null;
			this.applyBoard(board);
			this.setMessage("Ready", "info");
		} catch (err) {
			this.setMessage(errorText(err), "error");
			this.notify();
			throw err;
		} finally {
			this.busy = false;
			this.notify();
		}
		this.kickAutoLoop();
	}
	/** Start a new game. */
	async reset() {
		this.busy = true;
		this.notify();
		try {
			await newGame();
			this.selectedSq = null;
			this.legalMoves = [];
			this.lastMoveFrom = null;
			this.lastMoveTo = null;
			this.lastSearch = null;
			const board = await fetchState();
			this.applyBoard(board);
			this.setMessage("New game", "success");
		} catch (err) {
			this.setMessage(errorText(err), "error");
		} finally {
			this.busy = false;
			this.notify();
		}
		this.kickAutoLoop();
	}
	/** Toggle piece symbol mode between kanji and English abbreviations. */
	toggleKanji() {
		this.settings.useKanji = !this.settings.useKanji;
		this.notify();
	}
	/** Rotate the board 180°. */
	toggleFlip() {
		this.settings.flip = !this.settings.flip;
		this.notify();
	}
	setZoom(zoom) {
		this.settings.zoom = clampZoom(zoom);
		this.notify();
	}
	/** Undo the last `plies` moves. */
	async undo(plies = 1) {
		if (!this.board) return;
		this.busy = true;
		this.notify();
		try {
			const res = await undo(plies);
			if (res.ok && res.board) {
				this.lastMoveFrom = null;
				this.lastMoveTo = null;
				this.lastSearch = null;
				this.applyBoard(res.board);
				this.setMessage(`Undid ${res.undone_plies} move(s)`, "success");
				this.settings.autoPlay = false;
			} else this.setMessage(res.error ?? "Nothing to undo", "error");
		} catch (err) {
			this.setMessage(errorText(err), "error");
		} finally {
			this.busy = false;
			this.notify();
		}
	}
	/** Select a square and fetch its legal moves. */
	async select(r, c) {
		if (!this.board) return;
		this.selectedSq = [r, c];
		this.notify();
		try {
			const resp = await fetchMoves(r, c);
			if (this.selectedSq?.[0] !== r || this.selectedSq?.[1] !== c) return;
			this.legalMoves = resp.moves;
		} catch (err) {
			this.legalMoves = [];
			this.setMessage(errorText(err), "error");
		}
		this.notify();
	}
	clearSelection() {
		this.selectedSq = null;
		this.legalMoves = [];
		this.notify();
	}
	/**
	* Inspect the piece on a square: shows name/value/movement data in the
	* sidebar regardless of which side is to move.
	*/
	async inspect(r, c) {
		const piece = this.board?.board[r]?.[c] ?? null;
		if (!piece) {
			this.clearInspection();
			return;
		}
		const cached = this.pieceCache.get(piece.piece) ?? null;
		this.inspection = {
			square: [r, c],
			piece,
			info: cached
		};
		this.notify();
		if (cached) return;
		try {
			const info = await pieceInfo(piece.piece);
			this.pieceCache.set(piece.piece, info);
			if (this.inspection?.piece.piece === piece.piece) this.inspection = {
				...this.inspection,
				info
			};
		} catch (err) {
			if (err instanceof ApiError && err.status === 404) {} else this.setMessage(errorText(err), "error");
		}
		this.notify();
	}
	clearInspection() {
		if (!this.inspection) return;
		this.inspection = null;
		this.notify();
	}
	/** Execute a human move. */
	async play(fr, fc, tr, tc, promotion, mid_sq) {
		if (!this.board || this.board.game_over || this.busy) return;
		this.selectedSq = null;
		this.legalMoves = [];
		try {
			const resp = await sendMove([fr, fc], [tr, tc], promotion, mid_sq);
			if (resp.ok && resp.board) {
				this.lastMoveFrom = resp.last_move?.from ?? null;
				this.lastMoveTo = resp.last_move?.to ?? null;
				this.applyBoard(resp.board);
			} else {
				this.setMessage(resp.error ?? "Illegal move", "error");
				this.notify();
			}
		} catch (err) {
			this.setMessage(errorText(err), "error");
			this.notify();
		}
		this.kickAutoLoop();
	}
	/** Ask the AI to move once (public API used by the UI buttons). */
	async aiMove(depth, timeLimit) {
		await this.aiMoveOnce(depth, timeLimit);
		this.kickAutoLoop();
	}
	/** Run an analysis search without touching the board. */
	async analyze(depth, timeLimit = 1e4) {
		if (!this.board || this.busy) return;
		this.busy = true;
		this.notify();
		try {
			const resp = await analyze(depth ?? Math.max(1, this.settings.aiDepth), timeLimit);
			if (resp.best_move) {
				this.lastSearch = {
					depth: resp.depth,
					score: resp.score,
					nodes: resp.nodes,
					time_ms: resp.time_ms
				};
				const [fr, fc] = resp.best_move.from ?? [0, 0];
				const [tr, tc] = resp.best_move.to;
				this.setMessage(`Analysis d${resp.depth}: best ${fr},${fc}→${tr},${tc} (${resp.score})`, "info");
			} else this.setMessage("Analysis returned no move (time limit?)", "error");
		} catch (err) {
			this.setMessage(errorText(err), "error");
		} finally {
			this.busy = false;
			this.notify();
		}
	}
	/** Save the current position and return the TSFEN payload. */
	async save() {
		try {
			const saved = await saveGame();
			this.setMessage(`Saved position (${saved.tsfen.length} chars)`, "success");
			return saved;
		} catch (err) {
			this.setMessage(errorText(err), "error");
			return null;
		} finally {
			this.notify();
		}
	}
	/** Load a position from TSFEN. */
	async load(tsfen) {
		this.busy = true;
		this.notify();
		try {
			const board = await loadGame(tsfen.trim());
			this.selectedSq = null;
			this.legalMoves = [];
			this.lastMoveFrom = null;
			this.lastMoveTo = null;
			this.lastSearch = null;
			this.applyBoard(board);
			this.setMessage("Position loaded", "success");
			this.settings.autoPlay = false;
			return true;
		} catch (err) {
			this.setMessage(errorText(err), "error");
			return false;
		} finally {
			this.busy = false;
			this.notify();
		}
	}
	/** Effective depth: random modes always use depth 0. */
	depthForMode() {
		if (this.settings.mode === "human_vs_random" || this.settings.mode === "random_vs_random") return 0;
		return this.settings.aiDepth;
	}
	shouldAutoPlay() {
		if (!this.board || this.board.game_over || this.aiPending) return false;
		const { mode, humanColor } = this.settings;
		const side = this.board.side_to_move;
		if (mode === "ai_vs_ai" || mode === "random_vs_random") return this.settings.autoPlay;
		if (mode === "human_vs_random" || mode === "human_vs_ai") return side !== humanColor;
		return false;
	}
	async aiMoveOnce(depth, timeLimit) {
		if (!this.board || this.board.game_over || this.aiPending) return false;
		this.aiPending = true;
		this.busy = true;
		this.notify();
		try {
			const d = depth ?? this.depthForMode();
			const resp = await sendAiMove(d, timeLimit ?? (d === 0 ? 0 : 3e4));
			if (resp.ok && resp.board) {
				this.lastMoveFrom = resp.last_move?.from ?? null;
				this.lastMoveTo = resp.last_move?.to ?? null;
				this.lastSearch = resp.search ?? null;
				this.applyBoard(resp.board);
				return true;
			}
			if (resp.error) this.setMessage(resp.error, "error");
			this.settings.autoPlay = false;
			return false;
		} catch (err) {
			this.setMessage(errorText(err), "error");
			this.settings.autoPlay = false;
			return false;
		} finally {
			this.aiPending = false;
			this.busy = false;
			this.notify();
		}
	}
	/** Start the auto-play loop unless one is already running. */
	kickAutoLoop() {
		this.runAutoLoop();
	}
	async runAutoLoop() {
		if (this.autoLoopActive) return;
		this.autoLoopActive = true;
		try {
			while (this.shouldAutoPlay()) {
				await sleep(AI_DELAY_MS);
				if (!await this.aiMoveOnce()) break;
			}
		} finally {
			this.autoLoopActive = false;
		}
	}
};
function errorText(err) {
	if (err instanceof Error) return err.message;
	return String(err);
}
//#endregion
//#region src/data/kanji.ts
/**
* Kanji symbols for all Taikyoku Shogi piece types.
*
* Sources: English names and kanji from the Wikipedia "Game equipment" section
* (https://en.wikipedia.org/wiki/Taikyoku_shogi).  The mapping key is the
* two-letter (or three-letter) abbreviation used internally by the engine.
*/
var KANJI_MAP = {
	"K": "王将",
	"CP": "太子",
	"GG": "大将",
	"G": "金将",
	"RG": "右将",
	"LG": "左将",
	"RS": "後旗",
	"Q": "奔王",
	"FT": "奔獏",
	"WO": "鳩槃",
	"CD": "鳩盤",
	"ED": "地龍",
	"FR": "奔鬼",
	"HR": "走馬",
	"BC": "獣曹",
	"LO": "天狗",
	"MR": "右山鷲",
	"ML": "左山鷲",
	"DM": "火鬼",
	"W": "鯨鯢",
	"RR": "走兎",
	"WT": "白虎",
	"TS": "玄武",
	"L": "香車",
	"RV": "反車",
	"FG": "香象",
	"WE": "白象",
	"TD": "山鳩",
	"FS": "飛燕",
	"CO": "禽吏",
	"RA": "雨龍",
	"FO": "森鬼",
	"MS": "山鹿",
	"RP": "走狗",
	"RU": "走蛇",
	"SS": "横蛇",
	"GR": "大鳩",
	"RT": "走虎",
	"BA": "走熊",
	"YA": "夜叉",
	"BD": "羅刹",
	"GU": "金剛",
	"WR": "力士",
	"S": "銀将",
	"DE": "酔象",
	"NK": "近王",
	"GC": "金車",
	"SI": "横龍",
	"RN": "走鹿",
	"RW": "走狼",
	"BG": "角将",
	"RO": "飛将",
	"TT": "右虎",
	"LT": "左虎",
	"RI": "右龍",
	"LE": "左龍",
	"BO": "獣吏",
	"WD": "風龍",
	"FP": "奔狗",
	"RB": "行鳥",
	"OK": "古鵄",
	"PC": "孔雀",
	"WA": "水龍",
	"FI": "火龍",
	"C": "銅将",
	"PM": "鳳師",
	"KM": "麟師",
	"SV": "銀車",
	"VE": "竪熊",
	"N": "桂馬",
	"PI": "豚将",
	"CG": "鶏将",
	"PG": "狗将",
	"H": "馬将",
	"O": "牛将",
	"CN": "中旗",
	"SA": "横猪",
	"SR": "銀兎",
	"GL": "金鹿",
	"LN": "獅子",
	"CT": "禽曹",
	"GS": "大鹿",
	"VD": "猛龍",
	"WL": "林鬼",
	"VG": "副将",
	"CI": "石車",
	"CE": "雲鷲",
	"B": "角行",
	"R": "飛車",
	"WF": "横狼",
	"FC": "飛猫",
	"MF": "山鷹",
	"VT": "竪虎",
	"SO": "兵士",
	"LS": "小旗",
	"CL": "雲龍",
	"CR": "銅車",
	"RH": "走車",
	"HE": "羊兵",
	"VO": "猛牛",
	"GD": "大龍",
	"GO": "金翅",
	"DS": "無明",
	"DV": "提婆",
	"WC": "木車",
	"WH": "白駒",
	"DL": "𠵇犬",
	"DR": "𠵇犬",
	"SM": "横行",
	"PR": "踊鹿",
	"WB": "水牛",
	"FL": "猛豹",
	"EG": "猛鷲",
	"FD": "飛龍",
	"PS": "毒蛇",
	"FY": "鳫飛",
	"ST": "烏行",
	"BI": "盲犬",
	"WG": "水将",
	"F": "火将",
	"PH": "鳳凰",
	"KR": "麒麟",
	"HM": "鉤行",
	"LL": "小亀",
	"GT": "大亀",
	"CA": "摩羯",
	"TC": "瓦車",
	"VW": "竪狼",
	"SX": "横牛",
	"DO": "驢馬",
	"FH": "馬麟",
	"VB": "猛熊",
	"AB": "嗔猪",
	"EW": "悪狼",
	"WI": "風馬",
	"CK": "鶏飛",
	"OM": "古猿",
	"CC": "淮鶏",
	"NB": "北狄",
	"SU": "南蛮",
	"WS": "西戎",
	"ES": "東夷",
	"VS": "猛鹿",
	"NT": "猛狼",
	"TF": "隠狐",
	"MT": "中師",
	"PE": "鵬師",
	"EC": "土車",
	"VI": "朱雀",
	"BL": "青龍",
	"EB": "変狸",
	"HO": "騎兵",
	"OW": "鴟行",
	"CM": "登猿",
	"CS": "猫刀",
	"SW": "燕羽",
	"BM": "盲猿",
	"BT": "盲虎",
	"OC": "牛車",
	"SF": "横飛",
	"BB": "盲熊",
	"OR": "老鼠",
	"SQ": "方行",
	"SN": "蟠蛇",
	"RD": "臥龍",
	"FE": "奔鷲",
	"LI": "獅鷹",
	"CH": "車兵",
	"SL": "横兵",
	"VR": "竪兵",
	"WN": "風将",
	"RE": "川将",
	"M": "山将",
	"SD": "前旗",
	"HS": "馬兵",
	"GN": "木将",
	"OS": "牛兵",
	"EA": "土将",
	"BS": "猪兵",
	"SG": "石将",
	"LP": "豹兵",
	"T": "瓦将",
	"BE": "熊兵",
	"I": "鉄将",
	"GE": "大旗",
	"GM": "大師",
	"RC": "右車",
	"LC": "左車",
	"MK": "横猿",
	"VM": "竪行",
	"OX": "飛牛",
	"LB": "弩兵",
	"VP": "竪狗",
	"VH": "竪馬",
	"BN": "炮兵",
	"DH": "龍馬",
	"DK": "龍王",
	"SE": "刀兵",
	"HF": "角鷹",
	"EL": "飛鷲",
	"SP": "鎗兵",
	"VL": "竪豹",
	"TG": "猛虎",
	"SC": "弓兵",
	"DG": "吼犬",
	"LD": "狛犬",
	"D": "犬",
	"GB": "仲人",
	"P": "歩兵"
};
/** Look up the kanji abbreviation for a piece code. */
function pieceKanji(abbrev) {
	return KANJI_MAP[abbrev] ?? abbrev;
}
//#endregion
//#region src/ui/renderer.ts
var BASE_CELL = 28;
var BASE_COORD = 18;
/** Compute the canvas geometry for a zoom factor. */
function layoutFor(zoom) {
	const cell = Math.max(10, Math.round(BASE_CELL * zoom));
	const coord = Math.max(14, Math.round(BASE_COORD * zoom));
	const boardPx = cell * 36;
	return {
		cell,
		coord,
		boardPx,
		width: coord * 2 + boardPx
	};
}
var LIGHT = "#c8b07a";
var DARK = "#a68a5b";
var BG = "#0a0a1a";
var SEL = "#ffd700";
var LEGAL_EMPTY = "rgba(92,184,92,0.55)";
var LEGAL_CAPT = "rgba(217,83,79,0.55)";
var LAST_MOVE = "rgba(74,144,217,0.45)";
var HOVER = "rgba(255,215,0,0.20)";
var INSPECT = "#4ad9d9";
var BLACK_PIECE = "#1a0505";
var WHITE_PIECE = "#f5f5ff";
var ROYAL_BLACK = "#c8a030";
var ROYAL_WHITE = "#3050c8";
var UNDERLINE = "#ccccff";
var COORD_COLOR = "#888";
function render(ctx, opts) {
	const { state, selectedSq, inspectSq, legalMoves, hoverSq, lastFrom, lastTo, useKanji, flip } = opts;
	const { cell, coord, boardPx, width } = layoutFor(opts.zoom);
	const W = width;
	const H = width;
	const cellX = (viewCol) => coord + viewCol * cell;
	const cellY = (viewRow) => coord + viewRow * cell;
	const view = (sq) => sq ? toViewSquare(sq, flip) : null;
	ctx.fillStyle = BG;
	ctx.fillRect(0, 0, W, H);
	if (!state.board) {
		ctx.fillStyle = "#999";
		ctx.font = "16px sans-serif";
		ctx.textAlign = "center";
		ctx.textBaseline = "middle";
		ctx.fillText("Loading...", W / 2, H / 2);
		return;
	}
	const lastFromView = view(lastFrom);
	const lastToView = view(lastTo);
	const selectedView = view(selectedSq);
	const inspectView = view(inspectSq);
	const hoverView = view(hoverSq);
	for (let vr = 0; vr < 36; vr++) for (let vc = 0; vc < 36; vc++) {
		const x = cellX(vc);
		const y = cellY(vr);
		ctx.fillStyle = (vr + vc) % 2 === 0 ? LIGHT : DARK;
		ctx.fillRect(x, y, cell, cell);
		if (lastFromView && lastFromView[0] === vr && lastFromView[1] === vc || lastToView && lastToView[0] === vr && lastToView[1] === vc) {
			ctx.fillStyle = LAST_MOVE;
			ctx.fillRect(x, y, cell, cell);
		}
	}
	for (const m of legalMoves) {
		const [tr, tc] = m.to;
		const [vr, vc] = toViewSquare([tr, tc], flip);
		const x = cellX(vc);
		const y = cellY(vr);
		if (state.board[tr]?.[tc] != null) {
			ctx.fillStyle = LEGAL_CAPT;
			ctx.fillRect(x, y, cell, cell);
		} else {
			ctx.fillStyle = LEGAL_EMPTY;
			ctx.beginPath();
			ctx.arc(x + cell / 2, y + cell / 2, Math.max(3, cell * .18), 0, Math.PI * 2);
			ctx.fill();
		}
	}
	const symbolFont = Math.max(7, Math.round(cell * .34));
	for (let br = 0; br < 36; br++) for (let bc = 0; bc < 36; bc++) {
		const piece = state.board[br][bc];
		if (!piece) continue;
		const [vr, vc] = toViewSquare([br, bc], flip);
		const x = cellX(vc);
		const y = cellY(vr);
		if (selectedView && selectedView[0] === vr && selectedView[1] === vc) {
			ctx.fillStyle = SEL;
			ctx.fillRect(x + 2, y + 2, cell - 4, cell - 4);
		}
		if (inspectView && inspectView[0] === vr && inspectView[1] === vc) {
			ctx.strokeStyle = INSPECT;
			ctx.lineWidth = 2;
			ctx.strokeRect(x + 1, y + 1, cell - 2, cell - 2);
		}
		if (hoverView && hoverView[0] === vr && hoverView[1] === vc && selectedSq === null) {
			ctx.fillStyle = HOVER;
			ctx.fillRect(x, y, cell, cell);
		}
		if (piece.is_royal) {
			ctx.fillStyle = piece.color === 0 ? ROYAL_BLACK : ROYAL_WHITE;
			ctx.fillRect(x + 2, y + 2, cell - 4, cell - 4);
		}
		const symbol = useKanji ? pieceKanji(piece.piece) : piece.piece;
		ctx.font = `bold ${symbolFont}px sans-serif`;
		ctx.textAlign = "center";
		ctx.textBaseline = "middle";
		ctx.fillStyle = piece.color === 0 ? BLACK_PIECE : WHITE_PIECE;
		ctx.fillText(symbol, x + cell / 2, y + cell / 2);
		if (piece.color === 1) {
			ctx.strokeStyle = UNDERLINE;
			ctx.lineWidth = 1;
			ctx.beginPath();
			ctx.moveTo(x + 5, y + cell - 4);
			ctx.lineTo(x + cell - 5, y + cell - 4);
			ctx.stroke();
		}
	}
	ctx.fillStyle = COORD_COLOR;
	ctx.font = `${Math.max(8, Math.round(9 * (cell / BASE_CELL)))}px monospace`;
	ctx.textBaseline = "middle";
	for (let i = 0; i < 36; i++) {
		const label = String(flip ? i + 1 : 36 - i);
		const x = cellX(i) + cell / 2;
		const y = cellY(i) + cell / 2;
		ctx.textAlign = "right";
		ctx.fillText(label, coord - 4, y);
		ctx.textAlign = "left";
		ctx.fillText(label, coord + boardPx + 4, y);
		ctx.textAlign = "center";
		ctx.textBaseline = "bottom";
		ctx.fillText(label, x, coord - 4);
		ctx.textBaseline = "top";
		ctx.fillText(label, x, coord + boardPx + 4);
		ctx.textBaseline = "middle";
	}
	if (state.game_over) {
		ctx.fillStyle = "rgba(10,10,26,0.55)";
		ctx.fillRect(coord, coord + boardPx / 2 - 30, boardPx, 60);
		ctx.fillStyle = "#ffd700";
		ctx.font = `bold ${Math.round(cell * .7)}px sans-serif`;
		ctx.textAlign = "center";
		ctx.textBaseline = "middle";
		const label = state.game_result ? state.game_result.replace("_", " ").toUpperCase() : "GAME OVER";
		ctx.fillText(label, W / 2, coord + boardPx / 2);
	}
}
//#endregion
//#region src/ui/modal.ts
function openChoiceModal(title, description, options) {
	return new Promise((resolve) => {
		const overlay = document.createElement("div");
		overlay.className = "modal-overlay";
		const box = document.createElement("div");
		box.className = "modal-box";
		box.setAttribute("role", "dialog");
		box.setAttribute("aria-modal", "true");
		const heading = document.createElement("h2");
		heading.textContent = title;
		box.appendChild(heading);
		if (description) {
			const paragraph = document.createElement("p");
			paragraph.className = "modal-description";
			paragraph.textContent = description;
			box.appendChild(paragraph);
		}
		const list = document.createElement("div");
		list.className = "modal-options";
		const close = (value) => {
			document.removeEventListener("keydown", onKey);
			overlay.remove();
			resolve(value);
		};
		options.forEach((option, index) => {
			const button = document.createElement("button");
			button.type = "button";
			button.className = "modal-option";
			button.dataset.index = String(index);
			const label = document.createElement("span");
			label.className = "modal-option-label";
			label.textContent = option.label;
			button.appendChild(label);
			if (option.detail) {
				const detail = document.createElement("span");
				detail.className = "modal-option-detail";
				detail.textContent = option.detail;
				button.appendChild(detail);
			}
			button.addEventListener("click", () => close(option.value));
			list.appendChild(button);
		});
		box.appendChild(list);
		const cancel = document.createElement("button");
		cancel.type = "button";
		cancel.className = "modal-cancel";
		cancel.textContent = "Cancel (Esc)";
		cancel.addEventListener("click", () => close(null));
		box.appendChild(cancel);
		const onKey = (event) => {
			if (event.key === "Escape") {
				event.preventDefault();
				close(null);
			}
		};
		document.addEventListener("keydown", onKey);
		overlay.addEventListener("click", (event) => {
			if (event.target === overlay) close(null);
		});
		overlay.appendChild(box);
		document.body.appendChild(overlay);
		box.querySelector(".modal-option")?.focus();
	});
}
//#endregion
//#region src/ui/controller.ts
var Controller = class {
	canvas;
	ctx;
	state;
	hoverSq = null;
	rafId = 0;
	constructor(canvas, ctx, state) {
		this.canvas = canvas;
		this.ctx = ctx;
		this.state = state;
		this.bindEvents();
		state.subscribe(() => this.scheduleDraw());
	}
	/** Convert a canvas-relative pixel coordinate to a board (row, col). */
	pixelToSquare(px, py) {
		const { cell, coord } = layoutFor(this.state.settings.zoom);
		const vc = Math.floor((px - coord) / cell);
		const vr = Math.floor((py - coord) / cell);
		if (vr < 0 || vr >= 36 || vc < 0 || vc >= 36) return null;
		return fromViewSquare([vr, vc], this.state.settings.flip);
	}
	/** Get mouse position relative to the canvas, accounting for CSS scaling. */
	mousePos(e) {
		const rect = this.canvas.getBoundingClientRect();
		const sx = this.canvas.width / rect.width;
		const sy = this.canvas.height / rect.height;
		return {
			px: (e.clientX - rect.left) * sx,
			py: (e.clientY - rect.top) * sy
		};
	}
	bindEvents() {
		this.canvas.addEventListener("mousemove", (e) => this.onMouseMove(e));
		this.canvas.addEventListener("mouseleave", () => this.onMouseLeave());
		this.canvas.addEventListener("click", (e) => void this.onClick(e));
	}
	onMouseMove(e) {
		const { px, py } = this.mousePos(e);
		const sq = this.pixelToSquare(px, py);
		if (sq) {
			if (!this.hoverSq || this.hoverSq[0] !== sq[0] || this.hoverSq[1] !== sq[1]) {
				this.hoverSq = sq;
				this.canvas.style.cursor = this.cursorFor(sq);
				this.scheduleDraw();
			}
		} else if (this.hoverSq) {
			this.hoverSq = null;
			this.canvas.style.cursor = "default";
			this.scheduleDraw();
		}
	}
	cursorFor([r, c]) {
		const board = this.state.board;
		if (!board) return "default";
		if (this.state.selectedSq) {
			if (this.state.legalMoves.some((m) => m.to[0] === r && m.to[1] === c)) return "pointer";
		}
		return board.board[r]?.[c] ? "pointer" : "default";
	}
	onMouseLeave() {
		if (this.hoverSq) {
			this.hoverSq = null;
			this.scheduleDraw();
		}
	}
	async onClick(e) {
		const { px, py } = this.mousePos(e);
		const sq = this.pixelToSquare(px, py);
		if (!sq) return;
		const [r, c] = sq;
		const board = this.state.board;
		if (!board) return;
		if (this.state.selectedSq) {
			const matches = this.state.legalMoves.filter((m) => m.to[0] === r && m.to[1] === c);
			if (matches.length > 0) {
				await this.handleTarget(matches);
				return;
			}
			const clicked = board.board[r]?.[c] ?? null;
			if (clicked && clicked.color === board.side_to_move && !board.game_over) {
				this.state.clearSelection();
				await this.state.select(r, c);
				this.state.inspect(r, c);
				return;
			}
			this.state.clearSelection();
			if (clicked) this.state.inspect(r, c);
			else this.state.clearInspection();
			return;
		}
		const cell = board.board[r]?.[c] ?? null;
		if (!cell) {
			this.state.clearInspection();
			return;
		}
		if (cell.color === board.side_to_move && !board.game_over) await this.state.select(r, c);
		this.state.inspect(r, c);
	}
	/** Apply a move—asking the user only when several moves are possible. */
	async handleTarget(matches) {
		if (!this.state.board || !this.state.selectedSq) return;
		const [fr, fc] = this.state.selectedSq;
		let target = matches[0];
		if (matches.length > 1) {
			const choices = matches.map((m, i) => ({
				label: describeMove(m, i),
				detail: [
					m.promotion ? "promotes" : null,
					m.is_igui ? "igui (capture in place)" : null,
					m.captured ? `captures ${m.captured}` : null,
					m.mid_sq != null ? `mid_sq=${m.mid_sq}` : null
				].filter(Boolean).join(" · "),
				value: m
			}));
			target = await openChoiceModal("Choose a move", `${matches.length} legal moves reach this square.`, choices);
		}
		if (!target) return;
		await this.state.play(fr, fc, target.to[0], target.to[1], target.promotion, target.mid_sq ?? null);
	}
	scheduleDraw() {
		if (this.rafId) return;
		this.rafId = requestAnimationFrame(() => {
			this.rafId = 0;
			this.draw();
		});
	}
	draw() {
		const board = this.state.board;
		if (!board) return;
		const { width } = layoutFor(this.state.settings.zoom);
		if (this.canvas.width !== width || this.canvas.height !== width) {
			this.canvas.width = width;
			this.canvas.height = width;
		}
		render(this.ctx, {
			state: board,
			selectedSq: this.state.selectedSq,
			inspectSq: this.state.inspection?.square ?? null,
			legalMoves: this.state.legalMoves,
			hoverSq: this.hoverSq,
			lastFrom: this.state.lastMoveFrom,
			lastTo: this.state.lastMoveTo,
			useKanji: this.state.settings.useKanji,
			flip: this.state.settings.flip,
			zoom: this.state.settings.zoom
		});
	}
};
function describeMove(m, index) {
	const dest = `(${m.to[0]}, ${m.to[1]})`;
	const suffix = m.promotion ? " +" : "";
	return `${index + 1}. Move to ${dest}${suffix}`;
}
//#endregion
//#region src/ui/panel.ts
var $$1 = (id) => document.getElementById(id);
function setText(id, text) {
	const el = $$1(id);
	if (el && el.textContent !== text) el.textContent = text;
}
function setDisabled(id, disabled) {
	const el = $$1(id);
	if (el) el.disabled = disabled;
}
var lastLogSignature = "";
function updatePanels(state) {
	const board = state.board;
	if (!board) return;
	setText("info-turn", sideName(board.side_to_move));
	setText("info-move", String(board.move_number));
	setText("info-black", String(board.black_pieces));
	setText("info-white", String(board.white_pieces));
	setText("info-score", formatScore(board.material_score));
	setText("info-eval", formatScore(board.eval_score));
	setText("info-progress", `${board.no_progress_plies} / 1000`);
	const statusMsg = $$1("status-msg");
	if (statusMsg) {
		const fallback = board.game_result ? describeResult(board.game_result) : board.game_over ? "Game over" : "—";
		const message = state.message?.text ?? fallback;
		const kind = state.message?.kind ?? (board.game_over ? "success" : "info");
		statusMsg.textContent = message;
		statusMsg.className = `status-msg ${kind}`;
	}
	setText("status", `${sideName(board.side_to_move)} to move · ${board.black_pieces} vs ${board.white_pieces} pieces · move ${board.move_number}`);
	setText("busy", state.busy ? "● thinking…" : "");
	const search = state.lastSearch;
	setText("search-depth", search ? `d${search.depth}` : "—");
	setText("search-score", search ? formatScore(search.score) : "—");
	setText("search-nodes", search ? formatNodes(search.nodes) : "—");
	setText("search-time", search ? `${search.time_ms} ms` : "—");
	const engine = state.engineInfo;
	setText("engine-version", engine ? `v${engine.version}` : "—");
	setText("engine-backend", engine ? engine.eval_backend : "—");
	setText("engine-hash", engine ? `${engine.hash_mb} MB` : "—");
	setText("engine-pieces", engine ? `${engine.piece_types} types` : "—");
	setText("engine-techniques", engine ? `${engine.search_techniques.length} enabled` : "—");
	updateInspector(state);
	const signature = `${board.move_log.length}:${board.move_log.at(-1) ?? ""}`;
	if (signature !== lastLogSignature) {
		lastLogSignature = signature;
		const log = $$1("move-log");
		if (log) {
			const fragment = document.createDocumentFragment();
			for (const entry of board.move_log) {
				const row = document.createElement("div");
				row.className = "entry";
				row.textContent = entry;
				fragment.appendChild(row);
			}
			if (board.move_log.length === 0) {
				const empty = document.createElement("div");
				empty.className = "entry empty";
				empty.textContent = "No moves yet";
				fragment.appendChild(empty);
			}
			log.replaceChildren(fragment);
			log.scrollTop = log.scrollHeight;
		}
	}
	drawScoreChart(board.score_history);
	const autoBtn = $$1("btn-auto");
	if (autoBtn) {
		autoBtn.textContent = state.settings.autoPlay ? "Stop" : "Auto";
		autoBtn.classList.toggle("active", state.settings.autoPlay);
	}
	const kanjiBtn = $$1("btn-kanji");
	if (kanjiBtn) kanjiBtn.textContent = state.settings.useKanji ? "Kanji" : "English";
	const flipBtn = $$1("btn-flip");
	if (flipBtn) flipBtn.textContent = state.settings.flip ? "Flip ↺" : "Flip";
	setDisabled("btn-undo", board.move_log.length === 0);
	setDisabled("btn-analyze", state.busy);
	setDisabled("btn-ai-move", state.busy || board.game_over);
	setDisabled("btn-load", state.busy);
	const tsfen = $$1("tsfen-box");
	if (tsfen && document.activeElement !== tsfen) tsfen.value = board.tsfen;
}
function updateInspector(state) {
	const inspection = state.inspection;
	const hint = $$1("inspect-hint");
	if (!inspection) {
		setText("inspect-title", "—");
		setText("inspect-name", "Click a piece");
		setText("inspect-abbrev", "");
		setText("inspect-value", "");
		setText("inspect-promotes", "");
		setText("inspect-betza", "");
		setText("inspect-tags", "");
		if (hint) hint.textContent = "Hover/click any piece to inspect its movement data.";
		return;
	}
	const { piece, info, square } = inspection;
	setText("inspect-title", `(${square[0]}, ${square[1]}) · ${sideName(piece.color)}`);
	setText("inspect-name", info?.name ?? piece.name);
	setText("inspect-abbrev", `[${piece.piece}]`);
	setText("inspect-value", `value ${info?.value ?? piece.value}`);
	setText("inspect-promotes", info?.promotes_to ? `promotes to ${info.promotes_to}` : "no promotion");
	setText("inspect-betza", info?.betza ? `Betza: ${info.betza}` : "");
	const tags = [];
	if (piece.is_royal || info?.is_royal) tags.push("royal");
	if (info) {
		if (info.slide_directions > 0) tags.push(`${info.slide_directions} slides`);
		if (info.jump_destinations > 0) tags.push(`${info.jump_destinations} jumps`);
		if (info.area_steps > 0) tags.push(`area ×${info.area_steps}`);
		if (info.has_hook) tags.push("hook");
		if (info.has_range_capture) tags.push("range capture");
		if (info.has_igui) tags.push("igui");
	}
	setText("inspect-tags", tags.join(" · "));
	if (hint) hint.textContent = info ? "" : "Loading movement data…";
}
function drawScoreChart(history) {
	const canvas = $$1("score-chart");
	if (!canvas) return;
	const dpr = Math.max(1, Math.min(2, window.devicePixelRatio || 1));
	const cssWidth = canvas.clientWidth || 220;
	const cssHeight = canvas.clientHeight || 64;
	if (canvas.width !== Math.round(cssWidth * dpr) || canvas.height !== Math.round(cssHeight * dpr)) {
		canvas.width = Math.round(cssWidth * dpr);
		canvas.height = Math.round(cssHeight * dpr);
	}
	const ctx = canvas.getContext("2d");
	if (!ctx) return;
	ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
	ctx.clearRect(0, 0, cssWidth, cssHeight);
	ctx.strokeStyle = "rgba(255,255,255,0.15)";
	ctx.lineWidth = 1;
	ctx.beginPath();
	ctx.moveTo(0, cssHeight / 2);
	ctx.lineTo(cssWidth, cssHeight / 2);
	ctx.stroke();
	if (history.length === 0) return;
	const points = chartPoints(history, cssWidth, cssHeight, 4);
	ctx.strokeStyle = "#4a90d9";
	ctx.lineWidth = 1.5;
	ctx.beginPath();
	points.forEach((p, i) => i === 0 ? ctx.moveTo(p.x, p.y) : ctx.lineTo(p.x, p.y));
	ctx.stroke();
	const last = points[points.length - 1];
	ctx.fillStyle = history[history.length - 1] >= 0 ? "#5cb85c" : "#d9534f";
	ctx.beginPath();
	ctx.arc(last.x, last.y, 2.5, 0, Math.PI * 2);
	ctx.fill();
}
//#endregion
//#region src/main.ts
/**
* Application entry point.
*
* Wires the game state, canvas renderer, input controller and DOM panels
* together, then boots the first game. All UI actions are also available as
* keyboard shortcuts:
*
*   N = new game · U = undo · A = auto-play · K = kanji/English
*   F = flip board · +/- = zoom · Space = ask the AI for a move
*/
var canvas = document.getElementById("board-canvas");
var ctx = canvas.getContext("2d");
if (!ctx) throw new Error("2D canvas context is not available in this browser");
var state = new GameState();
new Controller(canvas, ctx, state);
state.subscribe(() => updatePanels(state));
var $ = (id) => document.getElementById(id);
var select = (id) => $(id);
function readSettingsFromControls() {
	state.settings.mode = select("mode-select").value;
	state.settings.humanColor = Number(select("color-select").value);
	state.settings.aiDepth = Number(select("ai-depth").value);
}
function syncControlsFromSettings() {
	select("mode-select").value = state.settings.mode;
	select("color-select").value = String(state.settings.humanColor);
	select("ai-depth").value = String(state.settings.aiDepth);
	const zoom = $("zoom-range");
	zoom.value = String(Math.round(state.settings.zoom * 100));
	$("zoom-label").textContent = `${Math.round(state.settings.zoom * 100)}%`;
}
$("btn-new").addEventListener("click", () => {
	readSettingsFromControls();
	state.setAutoPlay(false);
	state.reset();
});
$("btn-undo").addEventListener("click", () => void state.undo(1));
$("btn-auto").addEventListener("click", () => {
	readSettingsFromControls();
	state.setAutoPlay(!state.settings.autoPlay);
});
$("btn-kanji").addEventListener("click", () => state.toggleKanji());
$("btn-flip").addEventListener("click", () => state.toggleFlip());
$("btn-analyze").addEventListener("click", () => void state.analyze());
$("btn-ai-move").addEventListener("click", () => void state.aiMove());
$("zoom-range").addEventListener("input", (event) => {
	const value = Number(event.target.value) / 100;
	state.setZoom(value);
	$("zoom-label").textContent = `${Math.round(state.settings.zoom * 100)}%`;
});
select("mode-select").addEventListener("change", readSettingsFromControls);
select("color-select").addEventListener("change", readSettingsFromControls);
select("ai-depth").addEventListener("change", readSettingsFromControls);
var tsfenBox = $("tsfen-box");
$("btn-save").addEventListener("click", async () => {
	const saved = await state.save();
	if (saved && document.activeElement !== tsfenBox) tsfenBox.value = saved.tsfen;
});
$("btn-load").addEventListener("click", async () => {
	readSettingsFromControls();
	const tsfen = tsfenBox.value.trim();
	if (!tsfen) {
		state.announce("Paste a TSFEN string first", "error");
		return;
	}
	state.setAutoPlay(false);
	await state.load(tsfen);
});
$("btn-copy").addEventListener("click", async () => {
	try {
		await navigator.clipboard.writeText(tsfenBox.value);
		state.announce("TSFEN copied to clipboard", "success");
	} catch {
		tsfenBox.select();
		state.announce("Clipboard blocked — text selected, press Ctrl+C", "error");
	}
});
$("btn-download").addEventListener("click", () => {
	const blob = new Blob([tsfenBox.value], { type: "text/plain" });
	const url = URL.createObjectURL(blob);
	const link = document.createElement("a");
	link.href = url;
	link.download = `taikyoku-move-${state.board?.move_number ?? 1}.tsfen`;
	link.click();
	URL.revokeObjectURL(url);
	state.announce("TSFEN downloaded", "success");
});
document.addEventListener("keydown", (event) => {
	const target = event.target;
	if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT")) return;
	if (event.ctrlKey || event.metaKey || event.altKey) return;
	switch (event.key.toLowerCase()) {
		case "n":
			readSettingsFromControls();
			state.setAutoPlay(false);
			state.reset();
			break;
		case "u":
			state.undo(1);
			break;
		case "a":
			readSettingsFromControls();
			state.setAutoPlay(!state.settings.autoPlay);
			break;
		case "k":
			state.toggleKanji();
			break;
		case "f":
			state.toggleFlip();
			break;
		case " ":
			event.preventDefault();
			state.aiMove();
			break;
		case "+":
		case "=":
			state.setZoom(Math.min(MAX_ZOOM, state.settings.zoom + .1));
			syncControlsFromSettings();
			break;
		case "-":
			state.setZoom(Math.max(MIN_ZOOM, state.settings.zoom - .1));
			syncControlsFromSettings();
			break;
		default: break;
	}
});
syncControlsFromSettings();
state.init().catch((err) => {
	const status = document.getElementById("status");
	if (status) status.textContent = `⚠️ Error: ${err instanceof Error ? err.message : String(err)} — is the Rust backend running on :8000?`;
	console.error("Failed to initialize game:", err);
});
//#endregion
