// Main game controller — WebSocket + rendering orchestration

(function () {
  // ── State ──────────────────────────────────────────────────────────────────
  let ws = null;
  let roomId = null;
  let playerId = null;
  let myColor = null;
  let gameType = null;
  let gameState = null;
  let renderer = null;
  let moveCount = 0;
  let wsReconnectTimer = null;
  let connectionRetries = 0;
  let vsComputer = false;
  let aiDifficulty = null;

  // ── Init ───────────────────────────────────────────────────────────────────
  function init() {
    const params = new URLSearchParams(window.location.search);
    roomId = params.get('room');
    // Always use sessionStorage — each tab has its own isolated ID.
    // Never pull from URL params: the share link must not carry player 1's ID.
    playerId = getOrCreatePlayerId();

    if (!roomId) {
      window.location.href = '/lobby.html';
      return;
    }

    // Share link — room only, no player ID
    const shareInput = document.getElementById('shareLink');
    if (shareInput) {
      const shareUrl = new URL(window.location.href);
      shareUrl.searchParams.delete('player');
      shareInput.value = shareUrl.href;
    }

    document.getElementById('copyLinkBtn').addEventListener('click', () => {
      navigator.clipboard.writeText(window.location.href).then(() => {
        document.getElementById('copyLinkBtn').textContent = 'Copied!';
        setTimeout(() => document.getElementById('copyLinkBtn').textContent = 'Copy', 2000);
      });
    });

    document.getElementById('copyGameLinkBtn').addEventListener('click', () => {
      navigator.clipboard.writeText(window.location.href);
    });

    document.getElementById('resignBtn').addEventListener('click', resign);

    // Chat
    document.getElementById('chatSendBtn').addEventListener('click', sendChat);
    document.getElementById('chatInput').addEventListener('keydown', e => {
      if (e.key === 'Enter') sendChat();
    });

    // Room ID badge
    document.getElementById('roomIdBadge').textContent = `Room: ${roomId}`;

    connectWS();
  }

  function getOrCreatePlayerId() {
    let id = sessionStorage.getItem('bgp_player_id');
    if (!id) {
      id = 'p_' + Math.random().toString(36).slice(2, 10);
      sessionStorage.setItem('bgp_player_id', id);
    }
    return id;
  }

  // ── WebSocket ──────────────────────────────────────────────────────────────
  function connectWS() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws/${roomId}?player_id=${playerId}`;

    ws = new WebSocket(wsUrl);

    ws.onopen = () => {
      connectionRetries = 0;
      addChat('system', 'Connected to server');
      if (wsReconnectTimer) { clearTimeout(wsReconnectTimer); wsReconnectTimer = null; }
    };

    ws.onmessage = (e) => {
      try {
        const msg = JSON.parse(e.data);
        handleMessage(msg);
      } catch (err) {
        console.error('Bad message:', e.data);
      }
    };

    ws.onerror = () => {
      addChat('system', 'Connection error');
    };

    ws.onclose = () => {
      if (!myColor && connectionRetries < 5) {
        // Connection closed before player was identified — retry silently
        const delay = Math.min(2000, 200 * Math.pow(2, connectionRetries++));
        wsReconnectTimer = setTimeout(connectWS, delay);
        return;
      }
      if (gameState && gameState.status === 'playing') {
        addChat('system', 'Disconnected. Reconnecting…');
        const delay = Math.min(5000, 1000 * Math.pow(2, connectionRetries++));
        wsReconnectTimer = setTimeout(connectWS, delay);
      }
    };
  }

  function send(obj) {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(obj));
    }
  }

  // ── Message handler ────────────────────────────────────────────────────────
  function handleMessage(msg) {
    switch (msg.type) {
      case 'connected':
        myColor = msg.color;
        vsComputer = msg.vs_computer || false;
        aiDifficulty = msg.ai_difficulty || null;
        connectionRetries = 0;
        setupPlayer(myColor, msg.room_status);
        break;

      case 'game_started':
        addChat('system', 'Game started! Both players connected.');
        showGameScreen();
        break;

      case 'game_state':
        gameState = msg.state;
        gameType = gameState.type;
        if (!renderer) {
          setupRenderer(gameType);
        }
        updateGameUI(gameState, msg);
        break;

      case 'game_over':
        handleGameOver(msg);
        break;

      case 'chat':
        addChat(msg.color, msg.text);
        break;

      case 'error':
        showError(msg.message);
        break;

      case 'opponent_disconnected':
        addChat('system', 'Opponent disconnected');
        break;

      case 'pong':
        break;
    }
  }

  // ── Setup ──────────────────────────────────────────────────────────────────
  function setupPlayer(color, roomStatus) {
    document.getElementById('yourColor').textContent = color;
    document.getElementById('yourColorWait').textContent = color;
    document.getElementById('yourAvatar').textContent = color === 'white' ? '☀' : '●';

    if (vsComputer) {
      const diffLabel = aiDifficulty ? ` (${aiDifficulty})` : '';
      document.getElementById('opponentColor').textContent = `Computer${diffLabel}`;
      document.getElementById('opponentAvatar').textContent = '🤖';
      // Hide chat — no point chatting with the AI
      const chatPanel = document.querySelector('.chat-panel');
      if (chatPanel) chatPanel.style.display = 'none';
    } else {
      document.getElementById('opponentColor').textContent = color === 'white' ? 'black' : 'white';
      document.getElementById('opponentAvatar').textContent = color === 'white' ? '●' : '☀';
    }

    const gameBadge = document.getElementById('gameTypeBadge');
    if (gameBadge) gameBadge.textContent = gameType || '…';

    if (roomStatus === 'playing') {
      showGameScreen();
    }
    // else stay on waiting screen
  }

  function setupRenderer(type) {
    const canvas = document.getElementById('gameCanvas');
    const gameRenderers = {
      chess: window.ChessGame,
      checkers: window.CheckersGame,
      go: window.GoGame,
      gomoku: window.GomokuGame,
    };

    renderer = gameRenderers[type];
    if (!renderer) return;

    renderer.init(canvas, myColor, (move) => {
      send({ type: 'make_move', move });
    });

    // Update game type badge
    const names = { chess: 'Chess', checkers: 'Checkers', go: 'Go', gomoku: 'Gomoku' };
    document.getElementById('gameTypeBadge').textContent = names[type] || type;

    // Show Go controls
    if (type === 'go') {
      document.getElementById('goControls').classList.remove('hidden');
      document.getElementById('passBtn').addEventListener('click', () => {
        send({ type: 'make_move', move: { pass: true } });
      });
    }
  }

  function showGameScreen() {
    document.getElementById('waitingScreen').classList.add('hidden');
    document.getElementById('gameScreen').classList.remove('hidden');
  }

  // ── UI Updates ─────────────────────────────────────────────────────────────
  function updateGameUI(s, msg) {
    if (msg && msg.room_status === 'playing') {
      showGameScreen();
    }

    if (!renderer) {
      setupRenderer(s.type);
    }

    if (renderer) {
      renderer.update(s);
    }

    // Status text
    const statusText = renderer ? renderer.getStatusText(s, myColor) : '';
    document.getElementById('statusText').textContent = statusText;

    const dot = document.querySelector('.status-dot');
    if (dot) {
      dot.className = 'status-dot';
      if (s.status !== 'playing') {
        dot.classList.add('over');
      } else if (s.current_turn === myColor) {
        dot.classList.add('your-turn');
      } else {
        dot.classList.add('waiting');
      }
    }

    // Go-specific
    if (s.type === 'go') {
      const bc = document.getElementById('blackCaptures');
      const wc = document.getElementById('whiteCaptures');
      if (bc) bc.textContent = s.captures_black;
      if (wc) wc.textContent = s.captures_white;
    }

    // Move history
    if (s.move_count !== undefined || s.last_move != null) {
      addMoveToHistory(s);
    }

    // Captured pieces display (chess)
    if (s.type === 'chess' && renderer.getCapturedPieces) {
      const captured = renderer.getCapturedPieces(s);
      updateCapturedDisplay(captured, myColor);
    }

    moveCount++;
  }

  function addMoveToHistory(s) {
    const list = document.getElementById('movesList');
    if (!list) return;

    let moveText = '';
    if (s.type === 'chess' && s.last_move) {
      const [[fr, fc], [tr, tc]] = s.last_move;
      const files = 'abcdefgh';
      moveText = `${files[fc]}${fr + 1}→${files[tc]}${tr + 1}`;
      if (s.in_check) moveText += '+';
    } else if (s.type === 'checkers' && s.last_move) {
      // Not tracked per-move in checkers state
      moveText = `move ${s.move_count || moveCount}`;
    } else if ((s.type === 'go' || s.type === 'gomoku') && s.last_move) {
      const [r, c] = s.last_move;
      const files = 'ABCDEFGHJKLMNOPQRST';
      moveText = s.type === 'go'
        ? `${files[c]}${s.size - r}`
        : `${String.fromCharCode(65 + c)}${r + 1}`;
    } else if (s.type === 'go' && s.consecutive_passes > 0) {
      moveText = 'pass';
    }

    if (moveText) {
      // Determine which color just moved (it's opposite of current turn)
      const justMoved = s.current_turn === 'white' ? 'black' : 'white';
      const div = document.createElement('div');
      div.className = `move-entry ${justMoved}`;
      div.textContent = `${justMoved === 'white' ? '☀' : '●'} ${moveText}`;
      list.appendChild(div);
      list.scrollTop = list.scrollHeight;
    }
  }

  function updateCapturedDisplay(captured, myColor) {
    const opponentColor = myColor === 'white' ? 'black' : 'white';
    const SYMBOLS = {
      queen: myColor === 'white' ? '♛' : '♕',
      rook: myColor === 'white' ? '♜' : '♖',
      bishop: myColor === 'white' ? '♝' : '♗',
      knight: myColor === 'white' ? '♞' : '♘',
      pawn: myColor === 'white' ? '♟' : '♙',
    };

    // Pieces captured by me (opponent's lost pieces)
    const capturedByMe = captured[opponentColor];
    const byYou = document.getElementById('capturedByYou');
    if (byYou) {
      byYou.textContent = '';
      for (const [type, count] of Object.entries(capturedByMe || {})) {
        byYou.textContent += SYMBOLS[type].repeat(count) + ' ';
      }
    }

    // Pieces captured by opponent (my lost pieces)
    const capturedByOpp = captured[myColor];
    const OPPONENT_SYMBOLS = {
      queen: myColor === 'black' ? '♛' : '♕',
      rook: myColor === 'black' ? '♜' : '♖',
      bishop: myColor === 'black' ? '♝' : '♗',
      knight: myColor === 'black' ? '♞' : '♘',
      pawn: myColor === 'black' ? '♟' : '♙',
    };
    const byOpponent = document.getElementById('capturedByOpponent');
    if (byOpponent) {
      byOpponent.textContent = '';
      for (const [type, count] of Object.entries(capturedByOpp || {})) {
        byOpponent.textContent += OPPONENT_SYMBOLS[type].repeat(count) + ' ';
      }
    }
  }

  // ── Game Over ──────────────────────────────────────────────────────────────
  function handleGameOver(msg) {
    const panel = document.getElementById('gameOverPanel');
    const icon  = document.getElementById('gameOverIcon');
    const title = document.getElementById('gameOverTitle');
    const message = document.getElementById('gameOverMessage');

    if (!panel) return;

    const status = msg.status;
    const isWin  = (status === 'white_won' && myColor === 'white') ||
                   (status === 'black_won' && myColor === 'black');
    const isLoss = (status === 'white_won' && myColor === 'black') ||
                   (status === 'black_won' && myColor === 'white');

    if (msg.reason === 'resignation') {
      icon.textContent    = isWin ? '🏆' : '🤝';
      title.textContent   = isWin ? 'You Won!' : 'You Resigned';
      message.textContent = isWin ? 'Opponent resigned.' : 'You resigned.';
    } else if (isWin) {
      icon.textContent    = '🏆';
      title.textContent   = 'You Won!';
      message.textContent = gameType === 'chess'   ? 'Checkmate!' :
                            gameType === 'gomoku'  ? 'Five in a row!' :
                            gameType === 'go'      ? `B ${gameState?.score_black} – W ${gameState?.score_white}` :
                            'All opponent pieces captured!';
    } else if (isLoss) {
      icon.textContent    = vsComputer ? '🤖' : '💀';
      title.textContent   = vsComputer ? 'Computer Won' : 'You Lost';
      message.textContent = gameType === 'chess'  ? (vsComputer ? 'The computer checkmated you.' : 'Checkmate.') :
                            gameType === 'gomoku' ? (vsComputer ? 'Computer got five in a row.' : 'Opponent got five in a row.') :
                            'Better luck next time!';
    } else {
      icon.textContent    = '🤝';
      title.textContent   = 'Draw!';
      message.textContent = gameType === 'chess' ? 'Stalemate.' : 'The board is full.';
    }

    // Scroll the right panel to top so the result card is immediately visible
    panel.closest('.side-panel-right')?.scrollTo({ top: 0, behavior: 'smooth' });
    panel.classList.remove('hidden');
  }

  // ── Actions ────────────────────────────────────────────────────────────────
  function resign() {
    send({ type: 'resign' });
  }

  function sendChat() {
    const input = document.getElementById('chatInput');
    const text = input.value.trim();
    if (!text) return;
    send({ type: 'chat', text });
    input.value = '';
  }

  function addChat(color, text) {
    const container = document.getElementById('chatMessages');
    if (!container) return;
    const div = document.createElement('div');
    div.className = `chat-msg ${color}`;
    if (color === 'system') {
      div.innerHTML = `<em>${escapeHtml(text)}</em>`;
    } else {
      div.innerHTML = `<span class="chat-author">${color === 'white' ? '☀' : '●'}</span>${escapeHtml(text)}`;
    }
    container.appendChild(div);
    container.scrollTop = container.scrollHeight;
  }

  function showError(msg) {
    // Flash a temporary error message near the board
    const statusEl = document.getElementById('statusText');
    if (statusEl) {
      const prev = statusEl.textContent;
      statusEl.textContent = '⚠ ' + msg;
      statusEl.style.color = '#e05a6a';
      setTimeout(() => {
        statusEl.textContent = prev;
        statusEl.style.color = '';
      }, 2500);
    }
  }

  function escapeHtml(str) {
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  // ── Start ──────────────────────────────────────────────────────────────────
  document.addEventListener('DOMContentLoaded', init);
})();
