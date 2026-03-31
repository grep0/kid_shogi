// Configuration
const RPC_URL = `http://${window.location.hostname}:3030`;
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
  const currentOwner = turn; // uppercase = current player
  const opponentOwner = turn === 'sente' ? 'gote' : 'sente';

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
};

// ─── DOM helpers ─────────────────────────────────────────────────────────────

const $ = id => document.getElementById(id);

function setStatus(msg) {
  $('status-bar').textContent = msg;
}

// Human's pieces always face up (SenteX.svg); AI's pieces always face down (GoteX.svg).
function pieceImageSrc(kind, owner) {
  return `${ASSETS_BASE}${owner === state.humanPlayer ? 'Sente' : 'Gote'}${kind}.svg`;
}

// ─── Board rendering ──────────────────────────────────────────────────────────

function renderBoard() {
  const { parsed, possibleMoves, selected, humanPlayer, gameResult } = state;
  const boardEl = $('board');
  boardEl.innerHTML = '';

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

  // The position string is always from the current player's perspective
  // (uppercase = current player, at bottom rows). Flip only when it's the
  // AI's turn so the human's pieces stay at the bottom of the screen.
  const flipped = parsed.turn !== humanPlayer;

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

  // Opponent's hand at top (order 0), human's hand at bottom (order 2).
  const opponent = humanPlayer === 'sente' ? 'gote' : 'sente';
  $(`hand-${opponent}`).style.order = '0';
  $(`hand-${humanPlayer}`).style.order = '2';

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
  state.busy = true;
  setStatus('Starting game…');
  try {
    const res = await rpcCall('start_game', { player: playerChoice });
    state.humanPlayer = playerChoice === 0 ? 'sente' : 'gote';
    state.gameId = res.game_id;
    applyServerResponse(res.position, res.possible_moves, null);
    if (res.last_move) {
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
  setStatus(`You played ${mv}. Thinking…`);
  renderBoard();
  try {
    const res = await rpcCall('make_move', { game_id: state.gameId, move: mv });
    applyServerResponse(res.position, res.possible_moves, res.game_result);
    if (res.game_result === 'YouWon') {
      setStatus('You won!');
    } else if (res.game_result === 'IWon') {
      setStatus(`AI played ${res.last_move}. AI wins!`);
    } else if (res.game_result === 'Draw') {
      setStatus('Draw!');
    } else {
      setStatus(`AI played ${res.last_move}. Your turn.`);
    }
  } catch (err) {
    setStatus(`Error: ${err.message}`);
    // Re-render to restore previous state
    renderBoard();
  } finally {
    state.busy = false;
  }
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

function showGame() {
  $('setup-screen').hidden = true;
  $('game-screen').hidden = false;
}

function showSetup() {
  $('setup-screen').hidden = false;
  $('game-screen').hidden = true;
  state.gameId = null;
  state.position = null;
  state.parsed = null;
  state.possibleMoves = [];
  state.selected = null;
  state.gameResult = null;
}

$('btn-play-first').addEventListener('click', () => {
  showGame();
  startGame(0);
});
$('btn-play-second').addEventListener('click', () => {
  showGame();
  startGame(1);
});
$('btn-new-game').addEventListener('click', showSetup);
