// Checkers renderer and interaction

window.CheckersGame = (function () {
  let canvas, ctx;
  let state = null;
  let selectedSquare = null;
  let validMovesMap = {};
  let myColor = null;
  let onMove = null;

  const COLORS = {
    lightSq: '#f0d9b5',
    darkSq: '#7a5230',
    selected: 'rgba(246,246,104,0.8)',
    validTarget: 'rgba(20,180,50,0.45)',
    mustJump: 'rgba(220,60,20,0.35)',
    whitePiece: '#f5f5f5',
    whitePieceBorder: '#ccc',
    blackPiece: '#1a1a1a',
    blackPieceBorder: '#555',
    kingCrown: '#f0c040',
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
    const size = Math.min(520, window.innerWidth - 32, window.innerHeight - 180);
    canvas.width = size;
    canvas.height = size;
    if (state) render(state);
  }

  function sq() { return canvas.width / 8; }

  function toDisplay(row, col) {
    if (myColor === 'white') return { dr: row, dc: col };
    return { dr: 7 - row, dc: 7 - col };
  }

  function fromDisplay(dr, dc) {
    if (myColor === 'white') return { row: dr, col: dc };
    return { row: 7 - dr, col: 7 - dc };
  }

  function buildValidMovesMap(s) {
    validMovesMap = {};
    if (!s.valid_moves) return;
    for (const entry of s.valid_moves) {
      const key = `${entry.from[0]},${entry.from[1]}`;
      validMovesMap[key] = { targets: entry.to, mustJump: entry.must_jump };
    }
  }

  function render(s) {
    state = s;
    buildValidMovesMap(s);
    const size = sq();
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Board squares
    for (let r = 0; r < 8; r++) {
      for (let c = 0; c < 8; c++) {
        const { row, col } = fromDisplay(r, c);
        const x = c * size;
        const y = r * size;
        const isLight = (row + col) % 2 === 0;
        ctx.fillStyle = isLight ? COLORS.lightSq : COLORS.darkSq;
        ctx.fillRect(x, y, size, size);
      }
    }

    // Highlight selected
    if (selectedSquare) {
      const [sr, sc] = selectedSquare;
      const { dr, dc } = toDisplay(sr, sc);
      ctx.fillStyle = COLORS.selected;
      ctx.fillRect(dc * size, dr * size, size, size);

      // Show valid targets
      const key = `${sr},${sc}`;
      if (validMovesMap[key]) {
        for (const [tr, tc] of validMovesMap[key].targets) {
          const { dr: tdr, dc: tdc } = toDisplay(tr, tc);
          ctx.fillStyle = validMovesMap[key].mustJump ? COLORS.mustJump : COLORS.validTarget;
          ctx.fillRect(tdc * size, tdr * size, size, size);
        }
      }
    }

    // Highlight pieces that must jump
    if (s.current_turn === myColor) {
      for (const [key, info] of Object.entries(validMovesMap)) {
        if (info.mustJump) {
          const [kr, kc] = key.split(',').map(Number);
          const { dr, dc } = toDisplay(kr, kc);
          ctx.strokeStyle = 'rgba(220,60,20,0.8)';
          ctx.lineWidth = 3;
          ctx.strokeRect(dc * size + 2, dr * size + 2, size - 4, size - 4);
        }
      }
    }

    // Draw pieces
    for (let r = 0; r < 8; r++) {
      for (let c = 0; c < 8; c++) {
        const { row, col } = fromDisplay(r, c);
        const piece = s.board[row][col];
        if (!piece) continue;

        const x = c * size + size / 2;
        const y = r * size + size / 2;
        const radius = size * 0.38;

        // Shadow
        ctx.beginPath();
        ctx.arc(x + 2, y + 3, radius, 0, Math.PI * 2);
        ctx.fillStyle = 'rgba(0,0,0,0.4)';
        ctx.fill();

        // Piece circle
        ctx.beginPath();
        ctx.arc(x, y, radius, 0, Math.PI * 2);
        if (piece.color === 'white') {
          const grad = ctx.createRadialGradient(x - radius * 0.3, y - radius * 0.3, 0, x, y, radius);
          grad.addColorStop(0, '#fff');
          grad.addColorStop(1, '#ccc');
          ctx.fillStyle = grad;
        } else {
          const grad = ctx.createRadialGradient(x - radius * 0.3, y - radius * 0.3, 0, x, y, radius);
          grad.addColorStop(0, '#444');
          grad.addColorStop(1, '#111');
          ctx.fillStyle = grad;
        }
        ctx.fill();

        ctx.strokeStyle = piece.color === 'white' ? COLORS.whitePieceBorder : COLORS.blackPieceBorder;
        ctx.lineWidth = 2;
        ctx.stroke();

        // Inner ring for depth
        ctx.beginPath();
        ctx.arc(x, y, radius * 0.75, 0, Math.PI * 2);
        ctx.strokeStyle = piece.color === 'white' ? 'rgba(180,180,180,0.5)' : 'rgba(80,80,80,0.5)';
        ctx.lineWidth = 1.5;
        ctx.stroke();

        // King crown
        if (piece.type === 'king') {
          ctx.font = `${size * 0.4}px serif`;
          ctx.textAlign = 'center';
          ctx.textBaseline = 'middle';
          ctx.fillStyle = COLORS.kingCrown;
          ctx.fillText('♛', x, y);
        }
      }
    }
  }

  function handleClick(e) {
    if (!state || state.status !== 'playing') return;
    if (state.current_turn !== myColor) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const s = sq();
    const dc = Math.floor(x / s);
    const dr = Math.floor(y / s);
    const { row, col } = fromDisplay(dr, dc);

    if (selectedSquare) {
      const [selR, selC] = selectedSquare;
      const key = `${selR},${selC}`;
      const info = validMovesMap[key];

      if (info && info.targets.some(([tr, tc]) => tr === row && tc === col)) {
        // Make the move
        if (onMove) {
          onMove({ from: [selR, selC], to: [row, col] });
        }
        selectedSquare = null;
      } else {
        // Try to select another piece
        const newKey = `${row},${col}`;
        if (validMovesMap[newKey]) {
          selectedSquare = [row, col];
        } else {
          selectedSquare = null;
        }
      }
    } else {
      const key = `${row},${col}`;
      if (validMovesMap[key]) {
        selectedSquare = [row, col];
      }
    }

    render(state);
  }

  function update(s) {
    selectedSquare = null;
    render(s);
  }

  function getStatusText(s, myColor) {
    const isContinuing = s.must_continue_from !== null && s.must_continue_from !== undefined;
    if (s.status === 'playing') {
      if (s.current_turn === myColor) {
        return isContinuing ? 'Your turn — continue jumping!' : 'Your turn';
      }
      return isContinuing ? "Opponent is multi-jumping" : "Opponent's turn";
    }
    if (s.status === 'white_won') return myColor === 'white' ? 'You won!' : 'Opponent won';
    if (s.status === 'black_won') return myColor === 'black' ? 'You won!' : 'Opponent won';
    if (s.status === 'draw') return 'Draw';
    return '';
  }

  function getCapturedPieces() { return { white: {}, black: {} }; }

  return { init, update, getStatusText, getCapturedPieces, resize };
})();
