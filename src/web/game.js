// Configuration
const RPC_URL = '/rpc';
const ASSETS_BASE = 'assets/';

// ─── RPC ─────────────────────────────────────────────────────────────────────

let rpcId = 0;

async function rpcCall(method, params) {
  const body = JSON.stringify({
    jsonrpc: '2.0',
    method,
    params,
    id: ++rpcId,
  });
  const resp = await fetch(RPC_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body,
  });
  const data = await resp.json();
  if (data.error) throw new Error(data.error.message);
  return data.result;
}

// ─── Position parsing ─────────────────────────────────────────────────────────
//
// Position string format: "gle/1c1/1C1/ELG b -"
//   - 4 rows separated by '/'; first row is the top of the board (row 4)
//   - Uppercase = current player's pieces, lowercase = opponent's
//   - 'b' = Sente to move, 'w' = Gote to move
//   - Hand section (after turn char) lists captured pieces available for drop
//
// Move format:
//   - Board move: "b2b3"  (fromCol fromRow toCol toRow, cols a-c, rows 1-4)
//   - Drop:       "C*b2"  (PieceChar * col row)

const PIECE_NAMES = { C: 'Chicken', E: 'Elephant', G: 'Giraffe', L: 'Lion', H: 'Hen' };

/**
 * Parse position string into a structured object.
 * Returns { grid, turn, senteHand, goteHand }
 *   grid[row][col]  row 0=top (row4), row 3=bottom (row1)
 *   Each cell: null | { kind: 'C'|'E'|'G'|'L'|'H', owner: 'sente'|'gote' }
 *   turn: 'sente' | 'gote'
 *   senteHand / goteHand: array of kind strings
 */
function parsePosition(posStr) {
  const [boardPart, turnChar, handPart] = posStr.split(' ');
  const turn = turnChar === 'b' ? 'sente' : 'gote';

  const grid = boardPart.split('/').map(rowStr => {
    const cells = [];
    for (const ch of rowStr) {
      if (ch >= '1' && ch <= '9') {
        for (let i = 0; i < parseInt(ch); i++) cells.push(null);
      } else {
        const kind = ch.toUpperCase();
        // FEN is always absolute Sente perspective: uppercase=Sente, lowercase=Gote
        const owner = ch === ch.toUpperCase() ? 'sente' : 'gote';
        cells.push({ kind, owner });
      }
    }
    return cells;
  });

  // Hand: uppercase chars = Sente's captures, lowercase = Gote's captures
  const senteHand = [];
  const goteHand  = [];
  if (handPart !== '-') {
    for (const c of handPart) {
      if (c === c.toUpperCase()) senteHand.push(c);
      else goteHand.push(c.toUpperCase());
    }
  }

  return { grid, turn, senteHand, goteHand };
}

function colToChar(col) { return String.fromCharCode('a'.charCodeAt(0) + col); }
function rowNumToStringIndex(rowNum) { return 4 - rowNum; } // row1=index3, row4=index0
function stringIndexToRowNum(idx) { return 4 - idx; }

/** Build a move string from two board coordinates */
function boardMoveStr(fromCol, fromRow, toCol, toRow) {
  return `${colToChar(fromCol)}${fromRow}${colToChar(toCol)}${toRow}`;
}

/** Build a drop move string */
function dropMoveStr(kind, toCol, toRow) {
  return `${kind}*${colToChar(toCol)}${toRow}`;
}

// ─── Game state ───────────────────────────────────────────────────────────────

const state = {
  gameId: null,       // opaque string from server, required for make_move
  position: null,     // raw string from server
  parsed: null,       // parsePosition result
  possibleMoves: [],  // string[] from server
  humanPlayer: null,  // 'sente' | 'gote'
  selected: null,     // { type: 'board', col, row } | { type: 'hand', kind } | null
  gameResult: null,   // null | 'YouWon' | 'IWon' | 'Draw'
  busy: false,        // waiting for server response
  record: [],         // [{ num, sente, gote }, …]
  halfMove: 0,        // 1-based half-move counter within current game
};

// ─── Game record ─────────────────────────────────────────────────────────────

function recordMove(mv, mover) {
  state.halfMove += 1;
  const num = Math.ceil(state.halfMove / 2);
  if (mover === 'sente') {
    state.record.push({ num, sente: mv, gote: null });
  } else {
    const last = state.record[state.record.length - 1];
    if (last && last.gote === null) {
      last.gote = mv;
    } else {
      state.record.push({ num, sente: null, gote: mv });
    }
  }
  renderRecord();
}

function renderRecord() {
  const list = $('record-list');
  list.innerHTML = '';
  for (const entry of state.record) {
    const row = document.createElement('div');
    row.className = 'record-row';
    row.innerHTML =
      `<span class="record-col-num">${entry.num}.</span>` +
      `<span class="record-col-sente">${entry.sente ?? ''}</span>` +
      `<span class="record-col-gote">${entry.gote ?? ''}</span>`;
    list.appendChild(row);
  }
  // Scroll to bottom so latest move is visible
  list.scrollTop = list.scrollHeight;
}

// ─── DOM helpers ─────────────────────────────────────────────────────────────

const $ = id => document.getElementById(id);

function setStatus(msg) {
  $('status-bar').textContent = msg;
}

// Sente pieces use SenteX.svg (facing up); Gote pieces use GoteX.svg (facing down).
// When the board is flipped (human is Gote), swap: Gote's pieces are now at the
// bottom and should appear facing up, so they get the Sente image, and vice versa.
function pieceImageSrc(kind, owner) {
  const flipped = state.humanPlayer === 'gote';
  const facingUp = flipped ? (owner === 'gote') : (owner === 'sente');
  return `${ASSETS_BASE}${facingUp ? 'Sente' : 'Gote'}${kind}.svg`;
}

// ─── Board rendering ──────────────────────────────────────────────────────────

function renderBoard() {
  const { parsed, possibleMoves, selected, humanPlayer, gameResult } = state;
  const boardEl = $('board');

  // Determine reachable destination squares from the current selection
  const reachable = new Set();
  if (selected && !gameResult) {
    for (const mv of possibleMoves) {
      const [fromStr, toStr] = parseMoveCoords(mv);
      if (selected.type === 'board') {
        const expectedFrom = `${colToChar(selected.col)}${selected.row}`;
        if (fromStr === expectedFrom) reachable.add(toStr);
      } else if (selected.type === 'hand') {
        // Drop move: "C*b2" — fromStr will be "C*" prefix, toStr will be destination
        if (mv.startsWith(`${selected.kind}*`)) {
          reachable.add(toStr);
        }
      }
    }
  }

  // Which squares have selectable pieces (human's pieces, human's turn)
  const isHumanTurn = parsed.turn === humanPlayer && !gameResult;

  // FEN is always Sente-absolute. For Gote, flip board so Gote's pieces
  // (rows 1-2 in FEN = grid indices 2-3) appear at the bottom of the screen.
  const flipped = humanPlayer === 'gote';

  $('board-bg').classList.toggle('flipped', flipped);
  // Clear only the cell elements, leaving #board-bg in place
  boardEl.querySelectorAll('.cell').forEach(el => el.remove());

  const rowIdxSeq = flipped ? [3, 2, 1, 0] : [0, 1, 2, 3];
  const colSeq    = flipped ? [2, 1, 0]    : [0, 1, 2];

  for (const rowIdx of rowIdxSeq) {
    const rowNum = stringIndexToRowNum(rowIdx); // 4 down to 1
    for (const col of colSeq) {
      const cell = document.createElement('div');
      cell.className = 'cell';
      cell.dataset.col = col;
      cell.dataset.row = rowNum;

      const coordStr = `${colToChar(col)}${rowNum}`;
      const piece = parsed.grid[rowIdx][col];

      if (piece) {
        const img = document.createElement('img');
        img.src = pieceImageSrc(piece.kind, piece.owner);
        img.alt = `${piece.owner} ${PIECE_NAMES[piece.kind]}`;
        img.className = 'piece';
        cell.appendChild(img);

        const isSelectable = isHumanTurn && piece.owner === humanPlayer;
        if (isSelectable) cell.classList.add('selectable');
      }

      const isSelected = selected?.type === 'board' &&
                         selected.col === col && selected.row === rowNum;
      if (isSelected) cell.classList.add('selected');

      if (reachable.has(coordStr)) cell.classList.add('reachable');

      cell.addEventListener('click', () => onCellClick(col, rowNum));
      boardEl.appendChild(cell);
    }
  }

  // When human is Gote, swap hand order: Sente's hand on top, Gote's on bottom.
  $('hand-gote').style.order  = humanPlayer === 'gote' ? '3' : '1';
  $('hand-sente').style.order = humanPlayer === 'gote' ? '1' : '3';

  renderHand('sente', parsed.senteHand);
  renderHand('gote',  parsed.goteHand);
}

function renderHand(owner, kinds) {
  const handEl = $(`hand-${owner}`).querySelector('.hand-pieces');
  handEl.innerHTML = '';
  const isHumanTurn = state.parsed.turn === state.humanPlayer && !state.gameResult;
  const isOwner = owner === state.humanPlayer;

  for (const kind of kinds) {
    const img = document.createElement('img');
    img.src = pieceImageSrc(kind, owner);
    img.alt = PIECE_NAMES[kind];
    img.className = 'piece hand-piece';
    img.dataset.kind = kind;
    if (isHumanTurn && isOwner) img.classList.add('selectable');

    const isSelected = state.selected?.type === 'hand' &&
                       state.selected.kind === kind &&
                       owner === state.humanPlayer;
    if (isSelected) img.classList.add('selected');

    img.addEventListener('click', e => {
      e.stopPropagation();
      onHandPieceClick(kind);
    });
    handEl.appendChild(img);
  }
}

// ─── Animation ────────────────────────────────────────────────────────────────

function cellEl(col, rowNum) {
  return document.querySelector(`#board .cell[data-col="${col}"][data-row="${rowNum}"]`);
}

// Fly a piece image from fromRect to toRect; resolves when done.
// If toImgSrc is given, swaps the image when the piece arrives (before removing).
function animatePiece(imgSrc, fromRect, toRect, toImgSrc = null) {
  return new Promise(resolve => {
    const img = document.createElement('img');
    img.src = imgSrc;
    img.style.cssText = [
      'position:fixed', 'pointer-events:none', 'z-index:1000',
      `left:${fromRect.left}px`, `top:${fromRect.top}px`,
      `width:${fromRect.width}px`, `height:${fromRect.height}px`,
    ].join(';');
    document.body.appendChild(img);
    img.getBoundingClientRect(); // force layout before starting transition
    img.style.transition = 'left 0.18s ease-in-out, top 0.18s ease-in-out';
    img.style.left = `${toRect.left}px`;
    img.style.top  = `${toRect.top}px`;
    img.addEventListener('transitionend', () => {
      if (toImgSrc) img.src = toImgSrc;
      img.remove();
      resolve();
    }, { once: true });
  });
}

// Returns [col, rowNum] from a coord string like "b3"
function parseCoord(s) {
  return [s.charCodeAt(0) - 'a'.charCodeAt(0), parseInt(s[1])];
}

// Apply a move locally to a parsed position, returning a new parsed object.
function applyMoveLocally(parsed, mv, mover) {
  const grid = parsed.grid.map(row => [...row]);
  const senteHand = [...parsed.senteHand];
  const goteHand  = [...parsed.goteHand];
  const hand = mover === 'sente' ? senteHand : goteHand;

  if (mv.includes('*')) {
    const [kindChar, toStr] = mv.split('*');
    const kind = kindChar.toUpperCase();
    const [toCol, toRow] = parseCoord(toStr);
    hand.splice(hand.indexOf(kind), 1);
    grid[rowNumToStringIndex(toRow)][toCol] = { kind, owner: mover };
  } else {
    const [fromCol, fromRow] = parseCoord(mv.slice(0, 2));
    const [toCol,   toRow]   = parseCoord(mv.slice(2, 4));
    const fromIdx = rowNumToStringIndex(fromRow);
    const toIdx   = rowNumToStringIndex(toRow);
    const captured = grid[toIdx][toCol];
    if (captured) {
      const baseKind = captured.kind === 'H' ? 'C' : captured.kind;
      (mover === 'sente' ? senteHand : goteHand).push(baseKind);
    }
    grid[toIdx][toCol]   = grid[fromIdx][fromCol];
    grid[fromIdx][fromCol] = null;
  }

  return { grid, turn: mover === 'sente' ? 'gote' : 'sente', senteHand, goteHand };
}

// Build animation promise(s) for a move applied to a given parsed state.
// For captures, also animates the captured piece flying to the mover's hand.
function buildMoveAnimation(mv, mover, parsed) {
  if (mv.includes('*')) {
    // Drop: find the specific piece in hand by kind
    const [kindChar, toStr] = mv.split('*');
    const kind = kindChar.toUpperCase();
    const [toCol, toRow] = parseCoord(toStr);
    const handImg = document.querySelector(`#hand-${mover} .hand-piece[data-kind="${kind}"]`);
    const toCell  = cellEl(toCol, toRow);
    if (!handImg || !toCell) return Promise.resolve();
    return animatePiece(pieceImageSrc(kind, mover),
      handImg.getBoundingClientRect(), toCell.getBoundingClientRect());
  }

  const [fromCol, fromRow] = parseCoord(mv.slice(0, 2));
  const [toCol,   toRow]   = parseCoord(mv.slice(2, 4));
  const fromCell = cellEl(fromCol, fromRow);
  const toCell   = cellEl(toCol, toRow);
  if (!fromCell || !toCell) return Promise.resolve();
  const piece = parsed.grid[rowNumToStringIndex(fromRow)][fromCol];
  if (!piece) return Promise.resolve();

  const moveAnim = animatePiece(
    pieceImageSrc(piece.kind, piece.owner),
    (fromCell.querySelector('img') || fromCell).getBoundingClientRect(),
    toCell.getBoundingClientRect());

  // If there's a capture, also animate the captured piece flying to the mover's hand.
  const captured = parsed.grid[rowNumToStringIndex(toRow)][toCol];
  if (!captured) return moveAnim;

  const handEl = document.querySelector(`#hand-${mover} .hand-pieces`);
  if (!handEl) return moveAnim;

  const baseKind = captured.kind === 'H' ? 'C' : captured.kind;
  const captureAnim = animatePiece(
    pieceImageSrc(captured.kind, captured.owner),
    (toCell.querySelector('img') || toCell).getBoundingClientRect(),
    handEl.getBoundingClientRect(),
    pieceImageSrc(baseKind, mover));  // swap to mover's facing on arrival

  return Promise.all([moveAnim, captureAnim]);
}

// ─── Interaction ──────────────────────────────────────────────────────────────

/** Parse a move string into [fromStr, toStr] for matching purposes */
function parseMoveCoords(mv) {
  if (mv.includes('*')) {
    // Drop: "C*b2" → fromStr="C*", toStr="b2"
    const [prefix, to] = mv.split('*');
    return [`${prefix}*`, to];
  }
  // Board move: "b2b3" → fromStr="b2", toStr="b3"
  return [mv.slice(0, 2), mv.slice(2, 4)];
}

function onCellClick(col, rowNum) {
  if (state.busy || state.gameResult) return;
  if (state.parsed.turn !== state.humanPlayer) return;

  const coordStr = `${colToChar(col)}${rowNum}`;

  // If a piece/hand is selected and this cell is reachable → make move
  if (state.selected) {
    let mv = null;
    if (state.selected.type === 'board') {
      const fromStr = `${colToChar(state.selected.col)}${state.selected.row}`;
      mv = `${fromStr}${coordStr}`;
    } else if (state.selected.type === 'hand') {
      mv = `${state.selected.kind}*${coordStr}`;
    }
    if (mv && state.possibleMoves.includes(mv)) {
      state.selected = null;
      sendHumanMove(mv);
      return;
    }
  }

  // Select a piece on this cell (if it belongs to human)
  const rowIdx = rowNumToStringIndex(rowNum);
  const piece = state.parsed.grid[rowIdx][col];
  if (piece && piece.owner === state.humanPlayer) {
    state.selected = { type: 'board', col, row: rowNum };
  } else {
    state.selected = null;
  }

  renderBoard();
}

function onHandPieceClick(kind) {
  if (state.busy || state.gameResult) return;
  if (state.parsed.turn !== state.humanPlayer) return;

  if (state.selected?.type === 'hand' && state.selected.kind === kind) {
    state.selected = null;
  } else {
    state.selected = { type: 'hand', kind };
  }
  renderBoard();
}

// ─── Server calls ─────────────────────────────────────────────────────────────

async function startGame(playerChoice) {
  if (state.gameId) {
    rpcCall('remove_game', { game_id: state.gameId }).catch(() => {});
    state.gameId = null;
  }
  state.busy = true;
  setStatus('Starting game…');
  try {
    const res = await rpcCall('start_game', { player: playerChoice });
    state.humanPlayer = playerChoice === 0 ? 'sente' : 'gote';
    state.gameId = res.game_id;
    applyServerResponse(res.position, res.possible_moves, null);
    if (res.last_move) {
      recordMove(res.last_move, 'sente'); // AI played first as Sente
      setStatus(`AI played ${res.last_move}. Your turn.`);
    } else {
      setStatus('Your turn.');
    }
  } catch (err) {
    setStatus(`Error: ${err.message}`);
  } finally {
    state.busy = false;
  }
}

async function sendHumanMove(mv) {
  state.busy = true;
  setStatus('Thinking…');

  // ── 1. Capture rects and start human animation BEFORE any re-render ──
  const humanAnim = buildMoveAnimation(mv, state.humanPlayer, state.parsed);

  // ── 2. Immediately render intermediate state so piece appears at destination
  //       when the overlay animation completes ──
  const intermediate = applyMoveLocally(state.parsed, mv, state.humanPlayer);
  state.parsed = intermediate;
  state.possibleMoves = [];
  renderBoard();

  // ── 3. Fire server request in parallel with human animation ──
  let res;
  try {
    [res] = await Promise.all([
      rpcCall('make_move', { game_id: state.gameId, move: mv }),
      humanAnim,
    ]);
  } catch (err) {
    setStatus(`Error: ${err.message}`);
    state.busy = false;
    renderBoard();
    return;
  }

  // ── 4. Record human move, animate AI move ──
  recordMove(mv, state.humanPlayer);
  const aiOwner = state.humanPlayer === 'sente' ? 'gote' : 'sente';
  if (res.last_move && !res.game_result) {
    await buildMoveAnimation(res.last_move, aiOwner, intermediate);
  }

  // ── 5. Apply final state ──
  if (res.last_move) recordMove(res.last_move, aiOwner);
  applyServerResponse(res.position, res.possible_moves, res.game_result);
  if (res.game_result === 'YouWon')     setStatus('You won!');
  else if (res.game_result === 'IWon')  setStatus(`AI played ${res.last_move}. AI wins!`);
  else if (res.game_result === 'Draw')  setStatus('Draw!');
  else                                  setStatus(`AI played ${res.last_move}. Your turn.`);

  state.busy = false;
}

function applyServerResponse(position, possibleMoves, gameResult) {
  state.position = position;
  state.parsed = parsePosition(position);
  state.possibleMoves = possibleMoves;
  state.gameResult = gameResult ?? null;
  state.selected = null;
  renderBoard();
}

// ─── Setup screen ─────────────────────────────────────────────────────────────

const INITIAL_POSITION = 'gle/1c1/1C1/ELG b -';

function showSetup() {
  $('setup-screen').classList.remove('hidden');
  // Show initial position on the board (non-interactive, Sente perspective)
  state.gameId = null;
  state.position = INITIAL_POSITION;
  state.parsed = parsePosition(INITIAL_POSITION);
  state.humanPlayer = 'sente'; // render from Sente perspective by default
  state.possibleMoves = [];
  state.selected = null;
  state.gameResult = null;
  state.busy = true; // prevent interaction
  state.record = [];
  state.halfMove = 0;
  renderRecord();
  renderBoard();
}

$('btn-play-first').addEventListener('click', () => {
  $('setup-screen').classList.add('hidden');
  state.busy = false;
  startGame(0);
});
$('btn-play-second').addEventListener('click', () => {
  $('setup-screen').classList.add('hidden');
  state.busy = false;
  startGame(1);
});
$('btn-new-game').addEventListener('click', showSetup);

// Show initial position immediately on page load
showSetup();
