// Chess renderer and interaction

window.ChessGame = (function () {
  const PIECE_UNICODE = {
    white: { king: '♔', queen: '♕', rook: '♖', bishop: '♗', knight: '♘', pawn: '♙' },
    black: { king: '♚', queen: '♛', rook: '♜', bishop: '♝', knight: '♞', pawn: '♟' },
  };

  let canvas, ctx;
  let state = null;
  let selectedSquare = null;
  let legalMovesForSelected = [];
  let myColor = null;
  let onMove = null;
  let promotionPending = null;

  const COLORS = {
    light: '#f0d9b5',
    dark: '#b58863',
    selected: 'rgba(246,246,104,0.75)',
    legalMove: 'rgba(20,85,30,0.45)',
    legalCapture: 'rgba(200,20,20,0.35)',
    lastMove: 'rgba(205,210,106,0.5)',
    check: 'rgba(220,30,30,0.55)',
  };

  function init(canvasEl, color, moveCallback) {
    canvas = canvasEl;
    ctx = canvas.getContext('2d');
    myColor = color;
    onMove = moveCallback;
    canvas.addEventListener('click', handleClick);
    resize();
    window.addEventListener('resize', resize);
  }

  function resize() {
    const size = Math.min(560, window.innerWidth - 32, window.innerHeight - 180);
    canvas.width = size;
    canvas.height = size;
    if (state) render(state);
  }

  function squareSize() {
    return canvas.width / 8;
  }

  function toDisplay(row, col) {
    // White plays at bottom (row 0 = rank 1 = displayed at bottom)
    if (myColor === 'white') {
      return { dr: 7 - row, dc: col };
    } else {
      return { dr: row, dc: 7 - col };
    }
  }

  function fromDisplay(dr, dc) {
    if (myColor === 'white') {
      return { row: 7 - dr, col: dc };
    } else {
      return { row: dr, col: 7 - dc };
    }
  }

  function render(s) {
    state = s;
    const sq = squareSize();
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Draw squares
    for (let r = 0; r < 8; r++) {
      for (let c = 0; c < 8; c++) {
        const { row, col } = fromDisplay(r, c);
        const x = c * sq;
        const y = r * sq;

        const isLight = (row + col) % 2 === 1;
        ctx.fillStyle = isLight ? COLORS.light : COLORS.dark;
        ctx.fillRect(x, y, sq, sq);

        // Last move highlight
        if (s.last_move) {
          const [[lr, lc], [tr, tc]] = s.last_move;
          if ((row === lr && col === lc) || (row === tr && col === tc)) {
            ctx.fillStyle = COLORS.lastMove;
            ctx.fillRect(x, y, sq, sq);
          }
        }

        // Selected highlight
        if (selectedSquare && selectedSquare[0] === row && selectedSquare[1] === col) {
          ctx.fillStyle = COLORS.selected;
          ctx.fillRect(x, y, sq, sq);
        }

        // Legal move dots/rings
        const isLegal = legalMovesForSelected.some(([mr, mc]) => mr === row && mc === col);
        if (isLegal) {
          const piece = s.board[row][col];
          if (piece) {
            // Capture: ring
            ctx.strokeStyle = COLORS.legalCapture;
            ctx.lineWidth = sq * 0.1;
            ctx.strokeRect(x + sq * 0.05, y + sq * 0.05, sq * 0.9, sq * 0.9);
          } else {
            // Move: dot
            ctx.fillStyle = COLORS.legalMove;
            ctx.beginPath();
            ctx.arc(x + sq / 2, y + sq / 2, sq * 0.15, 0, Math.PI * 2);
            ctx.fill();
          }
        }

        // Check highlight
        if (s.in_check && s.board[row][col]?.type === 'king' && s.board[row][col]?.color === s.current_turn) {
          ctx.fillStyle = COLORS.check;
          ctx.fillRect(x, y, sq, sq);
        }
      }
    }

    // Draw pieces
    ctx.font = `${sq * 0.8}px serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (let r = 0; r < 8; r++) {
      for (let c = 0; c < 8; c++) {
        const { row, col } = fromDisplay(r, c);
        const piece = s.board[row][col];
        if (piece) {
          const glyph = PIECE_UNICODE[piece.color][piece.type];
          const x = c * sq + sq / 2;
          const y = r * sq + sq / 2;
          // Shadow for contrast
          ctx.fillStyle = piece.color === 'white' ? 'rgba(0,0,0,0.35)' : 'rgba(255,255,255,0.15)';
          ctx.fillText(glyph, x + 1, y + 1);
          ctx.fillStyle = piece.color === 'white' ? '#fff' : '#1a1a1a';
          ctx.fillText(glyph, x, y);
        }
      }
    }

    // Rank/file labels
    ctx.font = `${sq * 0.22}px monospace`;
    const files = 'abcdefgh';
    for (let i = 0; i < 8; i++) {
      const { row, col } = fromDisplay(7, i);
      const file = files[col];
      ctx.fillStyle = i % 2 === 0 ? COLORS.dark : COLORS.light;
      ctx.textAlign = 'right';
      ctx.fillText(file, (i + 1) * sq - 2, 7 * sq + sq - 3);
    }
    for (let i = 0; i < 8; i++) {
      const { row } = fromDisplay(i, 0);
      const rank = row + 1;
      ctx.fillStyle = i % 2 === 0 ? COLORS.light : COLORS.dark;
      ctx.textAlign = 'left';
      ctx.fillText(rank, 2, i * sq + 3 + sq * 0.22);
    }
  }

  function handleClick(e) {
    if (!state || state.status !== 'playing') return;
    if (state.current_turn !== myColor) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const sq = squareSize();
    const dc = Math.floor(x / sq);
    const dr = Math.floor(y / sq);
    const { row, col } = fromDisplay(dr, dc);

    if (selectedSquare) {
      const [selR, selC] = selectedSquare;
      if (legalMovesForSelected.some(([mr, mc]) => mr === row && mc === col)) {
        // Check if pawn promotion
        const piece = state.board[selR][selC];
        const promotionRow = myColor === 'white' ? 7 : 0;
        if (piece && piece.type === 'pawn' && row === promotionRow) {
          promotionPending = { from: [selR, selC], to: [row, col] };
          showPromotionDialog();
          return;
        }
        sendMove(selR, selC, row, col, null);
        selectedSquare = null;
        legalMovesForSelected = [];
      } else {
        // Select different piece or deselect
        const piece = state.board[row][col];
        if (piece && piece.color === myColor) {
          selectSquare(row, col);
        } else {
          selectedSquare = null;
          legalMovesForSelected = [];
        }
      }
    } else {
      const piece = state.board[row][col];
      if (piece && piece.color === myColor) {
        selectSquare(row, col);
      }
    }
    render(state);
  }

  function selectSquare(row, col) {
    selectedSquare = [row, col];
    legalMovesForSelected = [];
    if (state && state.legal_moves) {
      const entry = state.legal_moves.find(m => m.from[0] === row && m.from[1] === col);
      if (entry) legalMovesForSelected = entry.to;
    }
  }

  function sendMove(fromR, fromC, toR, toC, promotion) {
    if (onMove) {
      onMove({
        from: [fromR, fromC],
        to: [toR, toC],
        promotion: promotion,
      });
    }
  }

  function showPromotionDialog() {
    const dialog = document.getElementById('promotionDialog');
    if (dialog) {
      dialog.classList.remove('hidden');
      dialog.querySelectorAll('.promo-btn').forEach(btn => {
        btn.onclick = () => {
          if (promotionPending) {
            const { from, to } = promotionPending;
            sendMove(from[0], from[1], to[0], to[1], btn.dataset.piece);
            promotionPending = null;
            selectedSquare = null;
            legalMovesForSelected = [];
            dialog.classList.add('hidden');
          }
        };
      });
    }
  }

  function update(s) {
    selectedSquare = null;
    legalMovesForSelected = [];
    render(s);
  }

  function getCapturedPieces(s) {
    const initial = { queen: 1, rook: 2, bishop: 2, knight: 2, pawn: 8, king: 1 };
    const counts = { white: {}, black: {} };
    const onBoard = { white: {}, black: {} };

    for (const row of s.board) {
      for (const piece of row) {
        if (piece) {
          onBoard[piece.color][piece.type] = (onBoard[piece.color][piece.type] || 0) + 1;
        }
      }
    }

    for (const color of ['white', 'black']) {
      for (const [type, count] of Object.entries(initial)) {
        if (type === 'king') continue;
        const lost = count - (onBoard[color][type] || 0);
        if (lost > 0) {
          counts[color][type] = lost;
        }
      }
    }
    return counts;
  }

  function getStatusText(s, myColor) {
    if (s.status === 'playing') {
      if (s.current_turn === myColor) {
        return s.in_check ? '⚠ Your turn — In Check!' : 'Your turn';
      } else {
        return s.in_check ? "Opponent's turn — In Check" : "Opponent's turn";
      }
    }
    if (s.status === 'white_won') return myColor === 'white' ? 'You won!' : 'Opponent won';
    if (s.status === 'black_won') return myColor === 'black' ? 'You won!' : 'Opponent won';
    if (s.status === 'draw') return 'Draw (Stalemate)';
    return '';
  }

  return { init, update, getCapturedPieces, getStatusText, resize };
})();
