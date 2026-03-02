// Lobby page logic

let selectedGameType = 'chess';
let vsComputer = false;
let selectedDifficulty = 'normal';

function getOrCreatePlayerId() {
  let id = sessionStorage.getItem('bgp_player_id');
  if (!id) {
    id = 'p_' + Math.random().toString(36).slice(2, 10);
    sessionStorage.setItem('bgp_player_id', id);
  }
  return id;
}

function init() {
  // Restore last-used settings from localStorage
  const savedGame = localStorage.getItem('bgp_game_type');
  const savedVsComputer = localStorage.getItem('bgp_vs_computer');
  const savedDifficulty = localStorage.getItem('bgp_difficulty');

  document.querySelectorAll('.game-type-btn').forEach(btn => {
    btn.addEventListener('click', () => selectGameType(btn.dataset.type));
  });

  document.getElementById('vsHumanBtn').addEventListener('click', () => setVsMode(false));
  document.getElementById('vsComputerBtn').addEventListener('click', () => setVsMode(true));

  document.querySelectorAll('.difficulty-btn').forEach(btn => {
    btn.addEventListener('click', () => selectDifficulty(btn.dataset.difficulty));
  });

  // Apply restored (or URL-overridden) settings after listeners are wired up
  const params = new URLSearchParams(window.location.search);
  const preselect = params.get('game');
  selectGameType(preselect || savedGame || 'chess');
  setVsMode(savedVsComputer === '1');
  if (savedDifficulty) selectDifficulty(savedDifficulty);

  document.getElementById('createRoomBtn').addEventListener('click', createRoom);
  document.getElementById('refreshBtn').addEventListener('click', loadRooms);

  loadRooms();
  setInterval(loadRooms, 5000);
}

function selectGameType(type) {
  selectedGameType = type;
  document.querySelectorAll('.game-type-btn').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.type === type);
  });
  localStorage.setItem('bgp_game_type', type);
}

function setVsMode(isComputer) {
  vsComputer = isComputer;
  document.getElementById('vsHumanBtn').classList.toggle('active', !isComputer);
  document.getElementById('vsComputerBtn').classList.toggle('active', isComputer);
  document.getElementById('difficultySelector').classList.toggle('hidden', !isComputer);
  localStorage.setItem('bgp_vs_computer', isComputer ? '1' : '0');
}

function selectDifficulty(difficulty) {
  selectedDifficulty = difficulty;
  document.querySelectorAll('.difficulty-btn').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.difficulty === difficulty);
  });
  localStorage.setItem('bgp_difficulty', difficulty);
}

async function createRoom() {
  // Persist current settings so they're restored on "New Game"
  localStorage.setItem('bgp_game_type', selectedGameType);
  localStorage.setItem('bgp_vs_computer', vsComputer ? '1' : '0');
  localStorage.setItem('bgp_difficulty', selectedDifficulty);

  const btn = document.getElementById('createRoomBtn');
  btn.disabled = true;
  btn.textContent = 'Creating…';

  try {
    const body = { game_type: selectedGameType };
    if (vsComputer) {
      body.vs_computer = true;
      body.difficulty = selectedDifficulty;
    }
    const res = await fetch('/api/rooms', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    const data = await res.json();
    if (data.room_id) {
      const playerId = getOrCreatePlayerId();
      window.location.href = `/game.html?room=${data.room_id}&player=${playerId}`;
    }
  } catch (e) {
    alert('Failed to create room. Is the server running?');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Create Room';
  }
}

async function loadRooms() {
  const container = document.getElementById('roomsList');
  try {
    const res = await fetch('/api/rooms');
    const data = await res.json();
    const rooms = data.rooms || [];

    if (rooms.length === 0) {
      container.innerHTML = '<div class="empty-rooms">No open games. Create one above!</div>';
      return;
    }

    const waiting = rooms.filter(r => r.status === 'waiting');
    const playing = rooms.filter(r => r.status === 'playing');
    const all = [...waiting, ...playing];

    container.innerHTML = all.map(room => {
      const isWaiting = room.status === 'waiting';
      const gameIcons = { chess: '♟', checkers: '●', go: '○', gomoku: '◉' };
      const icon = gameIcons[room.game_type] || '?';
      return `
        <div class="room-item" onclick="${isWaiting ? `joinRoom('${room.id}')` : ''}">
          <span class="room-game">${icon} ${room.game_type}</span>
          <span class="room-id">Room ${room.id}</span>
          <span class="room-status-${room.status}">
            ${isWaiting ? '⬤ Waiting for opponent' : '⬤ In progress'}
          </span>
          ${isWaiting
            ? `<button class="btn btn-primary btn-sm" onclick="event.stopPropagation();joinRoom('${room.id}')">Join</button>`
            : `<button class="btn btn-secondary btn-sm" disabled>Full</button>`
          }
        </div>
      `;
    }).join('');
  } catch (e) {
    container.innerHTML = '<div class="empty-rooms">Could not load rooms.</div>';
  }
}

function joinRoom(roomId) {
  const playerId = getOrCreatePlayerId();
  window.location.href = `/game.html?room=${roomId}&player=${playerId}`;
}

document.addEventListener('DOMContentLoaded', init);
