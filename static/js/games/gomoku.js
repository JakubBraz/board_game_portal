// Gomoku renderer and interaction

window.GomokuGame = (function () {
  let canvas, ctx;
  let state = null;
  let myColor = null;
  let onMove = null;
  let hoverPos = null;

  const MARGIN_RATIO = 0.05;

  function init(canvasEl, color, moveCallback) {
    canvas = canvasEl;
    ctx = canvas.getContext('2d');
    myColor = color;
    onMove = moveCallback;
    canvas.addEventListener('click', handleClick);
    canvas.addEventListener('mousemove', handleMouseMove);
    canvas.addEventListener('mouseleave', () => { hoverPos = null; if (state) render(state); });
    resize();
    window.addEventListener('resize', resize);
  }

  function resize() {
    const size = Math.min(540, window.innerWidth - 32, window.innerHeight - 180);
    canvas.width = size;
    canvas.height = size;
    if (state) render(state);
  }

  function getMetrics(s) {
    const boardSize = s ? s.size : 15;
    const margin = Math.floor(canvas.width * MARGIN_RATIO + 20);
    const gridSize = canvas.width - margin * 2;
    const cellSize = gridSize / (boardSize - 1);
    return { margin, gridSize, cellSize, boardSize };
  }

  function posFromPixel(px, py, s) {
    const { margin, cellSize, boardSize } = getMetrics(s);
    const col = Math.round((px - margin) / cellSize);
    const row = Math.round((py - margin) / cellSize);
    if (row < 0 || row >= boardSize || col < 0 || col >= boardSize) return null;
    return { row, col };
  }

  function render(s) {
    state = s;
    const { margin, cellSize, boardSize } = getMetrics(s);
    const w = canvas.width;

    // Background
    ctx.fillStyle = '#d4a056';
    ctx.fillRect(0, 0, w, w);

    // Grid lines
    ctx.strokeStyle = '#8b5e1a';
    ctx.lineWidth = 1;
    for (let i = 0; i < boardSize; i++) {
      const x = margin + i * cellSize;
      const y = margin + i * cellSize;
      ctx.beginPath(); ctx.moveTo(x, margin); ctx.lineTo(x, margin + (boardSize - 1) * cellSize); ctx.stroke();
      ctx.beginPath(); ctx.moveTo(margin, y); ctx.lineTo(margin + (boardSize - 1) * cellSize, y); ctx.stroke();
    }

    // Star points for 15x15
    const mid = Math.floor(boardSize / 2);
    const starPts = [[3, 3], [3, mid], [3, boardSize - 4], [mid, 3], [mid, mid], [mid, boardSize - 4],
      [boardSize - 4, 3], [boardSize - 4, mid], [boardSize - 4, boardSize - 4]];
    for (const [r, c] of starPts) {
      if (r < boardSize && c < boardSize) {
        const x = margin + c * cellSize;
        const y = margin + r * cellSize;
        ctx.beginPath();
        ctx.arc(x, y, 3, 0, Math.PI * 2);
        ctx.fillStyle = '#5a3a0a';
        ctx.fill();
      }
    }

    // Last move marker
    if (s.last_move) {
      const [lr, lc] = s.last_move;
      const x = margin + lc * cellSize;
      const y = margin + lr * cellSize;
      const stoneR = Math.max(4, cellSize * 0.44);
      ctx.strokeStyle = 'rgba(255, 80, 80, 0.9)';
      ctx.lineWidth = 2;
      const mr = stoneR * 0.5;
      ctx.beginPath();
      ctx.moveTo(x - mr, y);
      ctx.lineTo(x + mr, y);
      ctx.moveTo(x, y - mr);
      ctx.lineTo(x, y + mr);
      ctx.stroke();
    }

    // Hover preview
    if (hoverPos && s.current_turn === myColor && s.status === 'playing') {
      const { row, col } = hoverPos;
      if (!s.board[row][col]) {
        const x = margin + col * cellSize;
        const y = margin + row * cellSize;
        const r = Math.max(4, cellSize * 0.44);
        ctx.beginPath();
        ctx.arc(x, y, r, 0, Math.PI * 2);
        ctx.fillStyle = myColor === 'black' ? 'rgba(20,20,20,0.4)' : 'rgba(240,240,240,0.4)';
        ctx.fill();
      }
    }

    // Stones
    const stoneR = Math.max(4, cellSize * 0.46);
    for (let r = 0; r < boardSize; r++) {
      for (let c = 0; c < boardSize; c++) {
        const stone = s.board[r][c];
        if (!stone) continue;

        const x = margin + c * cellSize;
        const y = margin + r * cellSize;

        ctx.beginPath();
        ctx.arc(x + 1.5, y + 2, stoneR, 0, Math.PI * 2);
        ctx.fillStyle = 'rgba(0,0,0,0.4)';
        ctx.fill();

        ctx.beginPath();
        ctx.arc(x, y, stoneR, 0, Math.PI * 2);
        if (stone === 'black') {
          const grad = ctx.createRadialGradient(x - stoneR * 0.3, y - stoneR * 0.3, 0, x, y, stoneR);
          grad.addColorStop(0, '#555');
          grad.addColorStop(1, '#000');
          ctx.fillStyle = grad;
        } else {
          const grad = ctx.createRadialGradient(x - stoneR * 0.3, y - stoneR * 0.3, 0, x, y, stoneR);
          grad.addColorStop(0, '#fff');
          grad.addColorStop(0.7, '#ddd');
          grad.addColorStop(1, '#bbb');
          ctx.fillStyle = grad;
        }
        ctx.fill();
        ctx.strokeStyle = stone === 'black' ? '#000' : '#aaa';
        ctx.lineWidth = 0.5;
        ctx.stroke();
      }
    }

    // Coordinate labels
    ctx.fillStyle = '#5a3a0a';
    ctx.font = `${Math.max(10, cellSize * 0.38)}px monospace`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (let i = 0; i < boardSize; i++) {
      const x = margin + i * cellSize;
      const label = String.fromCharCode(65 + i); // A, B, C...
      ctx.fillText(label, x, margin - 13);
      ctx.fillText(label, x, margin + (boardSize - 1) * cellSize + 13);
      ctx.fillText(i + 1, margin - 13, margin + i * cellSize);
      ctx.fillText(i + 1, margin + (boardSize - 1) * cellSize + 13, margin + i * cellSize);
    }
  }

  function handleMouseMove(e) {
    if (!state) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const pos = posFromPixel(x, y, state);
    if (pos && (pos.row !== hoverPos?.row || pos.col !== hoverPos?.col)) {
      hoverPos = pos;
      render(state);
    }
  }

  function handleClick(e) {
    if (!state || state.status !== 'playing') return;
    if (state.current_turn !== myColor) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const pos = posFromPixel(x, y, state);
    if (!pos) return;

    if (onMove) {
      onMove({ row: pos.row, col: pos.col });
    }
  }

  function update(s) { render(s); }

  function getStatusText(s, myColor) {
    if (s.status === 'playing') {
      return s.current_turn === myColor ? 'Your turn' : "Opponent's turn";
    }
    if (s.status === 'black_won') return myColor === 'black' ? 'You won! (5 in a row!)' : 'Opponent won (5 in a row)';
    if (s.status === 'white_won') return myColor === 'white' ? 'You won! (5 in a row!)' : 'Opponent won (5 in a row)';
    if (s.status === 'draw') return 'Draw (Board full)';
    return '';
  }

  function getCapturedPieces() { return { white: {}, black: {} }; }

  return { init, update, getStatusText, getCapturedPieces, resize };
})();
