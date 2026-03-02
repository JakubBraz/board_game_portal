// Go renderer and interaction

window.GoGame = (function () {
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
    const size = Math.min(560, window.innerWidth - 32, window.innerHeight - 180);
    canvas.width = size;
    canvas.height = size;
    if (state) render(state);
  }

  function getMetrics(s) {
    const size = s ? s.size : 19;
    const margin = Math.floor(canvas.width * MARGIN_RATIO + 18);
    const gridSize = canvas.width - margin * 2;
    const cellSize = gridSize / (size - 1);
    return { margin, gridSize, cellSize, size };
  }

  function posFromPixel(px, py, s) {
    const { margin, cellSize, size } = getMetrics(s);
    const col = Math.round((px - margin) / cellSize);
    const row = Math.round((py - margin) / cellSize);
    if (row < 0 || row >= size || col < 0 || col >= size) return null;
    return { row, col };
  }

  function render(s) {
    state = s;
    const { margin, cellSize, size } = getMetrics(s);
    const w = canvas.width;
    const h = canvas.height;

    // Background
    ctx.fillStyle = '#c8a46e';
    ctx.fillRect(0, 0, w, h);

    // Board texture lines
    ctx.strokeStyle = '#7a5a2a';
    ctx.lineWidth = 1;

    for (let i = 0; i < size; i++) {
      const x = margin + i * cellSize;
      const y = margin + i * cellSize;

      // Vertical
      ctx.beginPath();
      ctx.moveTo(x, margin);
      ctx.lineTo(x, margin + (size - 1) * cellSize);
      ctx.stroke();

      // Horizontal
      ctx.beginPath();
      ctx.moveTo(margin, y);
      ctx.lineTo(margin + (size - 1) * cellSize, y);
      ctx.stroke();
    }

    // Star points
    const starPoints = size === 19
      ? [[3,3],[3,9],[3,15],[9,3],[9,9],[9,15],[15,3],[15,9],[15,15]]
      : size === 13
      ? [[3,3],[3,9],[6,6],[9,3],[9,9]]
      : [[2,2],[2,6],[4,4],[6,2],[6,6]];

    for (const [r, c] of starPoints) {
      if (r < size && c < size) {
        const x = margin + c * cellSize;
        const y = margin + r * cellSize;
        ctx.beginPath();
        ctx.arc(x, y, 3, 0, Math.PI * 2);
        ctx.fillStyle = '#5a3a0a';
        ctx.fill();
      }
    }

    // Ko point indicator
    if (s.ko_point) {
      const [kr, kc] = s.ko_point;
      const x = margin + kc * cellSize;
      const y = margin + kr * cellSize;
      ctx.strokeStyle = 'rgba(255, 80, 80, 0.8)';
      ctx.lineWidth = 2;
      ctx.strokeRect(x - cellSize * 0.3, y - cellSize * 0.3, cellSize * 0.6, cellSize * 0.6);
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
    for (let r = 0; r < size; r++) {
      for (let c = 0; c < size; c++) {
        const stone = s.board[r][c];
        if (!stone) continue;

        const x = margin + c * cellSize;
        const y = margin + r * cellSize;

        // Shadow
        ctx.beginPath();
        ctx.arc(x + 1.5, y + 2, stoneR, 0, Math.PI * 2);
        ctx.fillStyle = 'rgba(0,0,0,0.4)';
        ctx.fill();

        // Stone
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
    const files = 'ABCDEFGHJKLMNOPQRST'; // Go skips 'I'
    ctx.fillStyle = '#5a3a0a';
    ctx.font = `${Math.max(10, cellSize * 0.35)}px monospace`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (let i = 0; i < size; i++) {
      const x = margin + i * cellSize;
      ctx.fillText(files[i], x, margin - 14);
      ctx.fillText(files[i], x, margin + (size - 1) * cellSize + 14);
      const rankNum = size - i;
      ctx.fillText(rankNum, margin - 14, margin + i * cellSize);
      ctx.fillText(rankNum, margin + (size - 1) * cellSize + 14, margin + i * cellSize);
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
      onMove({ row: pos.row, col: pos.col, pass: false });
    }
  }

  function update(s) { render(s); }

  function getStatusText(s, myColor) {
    if (s.status === 'playing') {
      const turn = s.current_turn === myColor ? 'Your turn' : "Opponent's turn";
      if (s.consecutive_passes === 1) return `${turn} (opponent passed)`;
      return turn;
    }
    if (s.status === 'black_won') {
      const score = `B ${s.score_black} – W ${s.score_white}`;
      return myColor === 'black' ? `You won! (${score})` : `Opponent won (${score})`;
    }
    if (s.status === 'white_won') {
      const score = `W ${s.score_white} – B ${s.score_black}`;
      return myColor === 'white' ? `You won! (${score})` : `Opponent won (${score})`;
    }
    return 'Game over';
  }

  function getCapturedPieces() { return { white: {}, black: {} }; }

  return { init, update, getStatusText, getCapturedPieces, resize };
})();
