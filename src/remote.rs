//! Spotify Connect-style remote control server for BitChord.
//! Serves a sleek mobile web controller over HTTP and synchronizes
//! playback state in real-time via WebSockets on port 8420.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

pub const REMOTE_PORT: u16 = 8420;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RemoteState {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub cover_url: Option<String>,
    pub duration_secs: f32,
    pub position_secs: f32,
    pub is_playing: bool,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: String, // "off", "all", "one"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "value")]
pub enum RemoteCommand {
    Play,
    Pause,
    Toggle,
    Next,
    Prev,
    Seek(f32),
    Volume(f32),
    Shuffle,
    Repeat,
}

#[derive(Clone)]
pub struct RemoteServer {
    state: Arc<Mutex<RemoteState>>,
    cmd_tx: std::sync::mpsc::Sender<RemoteCommand>,
    cmd_rx: Arc<Mutex<std::sync::mpsc::Receiver<RemoteCommand>>>,
    ws_tx: broadcast::Sender<String>,
    local_ip: String,
}

impl RemoteServer {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
        let (ws_tx, _) = broadcast::channel(32);
        let state = Arc::new(Mutex::new(RemoteState::default()));
        let local_ip = detect_local_ip();

        let server = Self {
            state: state.clone(),
            cmd_tx: cmd_tx.clone(),
            cmd_rx: Arc::new(Mutex::new(cmd_rx)),
            ws_tx: ws_tx.clone(),
            local_ip,
        };

        // Spawn async server on background tokio thread
        let bg_state = state.clone();
        let bg_cmd_tx = cmd_tx;
        let bg_ws_tx = ws_tx;

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime for remote server");

            rt.block_on(async move {
                let addr = SocketAddr::from(([0, 0, 0, 0], REMOTE_PORT));
                let listener = match TcpListener::bind(addr).await {
                    Ok(l) => {
                        log::info!("Remote server listening on http://0.0.0.0:{}", REMOTE_PORT);
                        l
                    }
                    Err(e) => {
                        log::warn!("Could not bind remote server on port {}: {}", REMOTE_PORT, e);
                        return;
                    }
                };

                loop {
                    if let Ok((stream, peer)) = listener.accept().await {
                        let state = bg_state.clone();
                        let cmd_tx = bg_cmd_tx.clone();
                        let ws_tx = bg_ws_tx.clone();
                        let ws_rx = ws_tx.subscribe();

                        tokio::spawn(async move {
                            handle_connection(stream, peer, state, cmd_tx, ws_tx, ws_rx).await;
                        });
                    }
                }
            });
        });

        server
    }

    /// Read incoming commands from connected mobile controllers.
    pub fn poll_commands(&self) -> Vec<RemoteCommand> {
        let mut cmds = Vec::new();
        if let Ok(rx) = self.cmd_rx.try_lock() {
            while let Ok(cmd) = rx.try_recv() {
                cmds.push(cmd);
            }
        }
        cmds
    }

    /// Update playback state and broadcast to all connected WebSocket clients.
    pub fn update_state(&self, new_state: RemoteState) {
        let should_broadcast = {
            let mut s = self.state.lock().unwrap();
            let changed = s.title != new_state.title
                || s.is_playing != new_state.is_playing
                || (s.position_secs - new_state.position_secs).abs() > 1.5
                || (s.volume - new_state.volume).abs() > 0.05
                || s.shuffle != new_state.shuffle
                || s.repeat != new_state.repeat;
            *s = new_state.clone();
            changed
        };

        if should_broadcast {
            if let Ok(json) = serde_json::to_string(&new_state) {
                let _ = self.ws_tx.send(json);
            }
        }
    }

    pub fn local_url(&self) -> String {
        format!("http://{}:{}", self.local_ip, REMOTE_PORT)
    }

    /// Generate an authentic QR Code PNG (strictly compliant with user rules).
    pub fn qr_png(&self) -> Option<Vec<u8>> {
        let url = self.local_url();
        generate_qr_png_bytes(&url)
    }
}

/// Generates a real PNG QR code from a text string.
pub fn generate_qr_png_bytes(data: &str) -> Option<Vec<u8>> {
    use qrcode::QrCode;
    let code = QrCode::new(data.as_bytes()).ok()?;
    let modules = code.to_colors();
    let qr_w = code.width();
    let quiet_zone = 4;
    let scale = 6;
    let total_modules = qr_w + quiet_zone * 2;
    let img_size = (total_modules * scale) as u32;
    let mut img = image::RgbaImage::from_pixel(img_size, img_size, image::Rgba([255, 255, 255, 255]));

    for y in 0..qr_w {
        for x in 0..qr_w {
            if modules[y * qr_w + x] != qrcode::Color::Light {
                let px = ((x + quiet_zone) * scale) as u32;
                let py = ((y + quiet_zone) * scale) as u32;
                for dy in 0..scale as u32 {
                    for dx in 0..scale as u32 {
                        img.put_pixel(px + dx, py + dy, image::Rgba([0, 0, 0, 255]));
                    }
                }
            }
        }
    }

    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    img.write_to(&mut cursor, image::ImageFormat::Png).ok()?;
    Some(buf)
}

fn detect_local_ip() -> String {
    // Try connecting to a public DNS IP to determine outbound interface IP without sending data
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                let ip = addr.ip().to_string();
                if ip != "0.0.0.0" {
                    return ip;
                }
            }
        }
    }
    "127.0.0.1".to_string()
}

async fn handle_connection(
    mut stream: TcpStream,
    _peer: SocketAddr,
    state: Arc<Mutex<RemoteState>>,
    cmd_tx: std::sync::mpsc::Sender<RemoteCommand>,
    _ws_tx: broadcast::Sender<String>,
    mut ws_rx: broadcast::Receiver<String>,
) {
    let mut buf = [0u8; 2048];
    let n = match stream.peek(&mut buf).await {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let req = String::from_utf8_lossy(&buf[..n]);

    if req.contains("Upgrade: websocket") || req.contains("upgrade: websocket") {
        if let Ok(mut ws_stream) = tokio_tungstenite::accept_async(stream).await {
            // Send initial state
            let current = {
                let s = state.lock().unwrap();
                serde_json::to_string(&*s).unwrap_or_default()
            };
            if !current.is_empty() {
                let _ = ws_stream.send(Message::Text(current.into())).await;
            }

            let (mut ws_sink, mut ws_src) = ws_stream.split();

            loop {
                tokio::select! {
                    msg = ws_src.next() => {
                        let Some(Ok(msg)) = msg else { break };
                        if let Message::Text(txt) = msg {
                            if let Ok(cmd) = serde_json::from_str::<RemoteCommand>(&txt) {
                                let _ = cmd_tx.send(cmd);
                            }
                        }
                    }
                    update = ws_rx.recv() => {
                        if let Ok(json) = update {
                            if ws_sink.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        }
        return;
    }

    // Serve HTML controller
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut header_buf = vec![0u8; 1024];
    let _ = stream.read(&mut header_buf).await;

    let body = HTML_PAGE;
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes()).await;
}

const HTML_PAGE: &str = r##"<!DOCTYPE html>
<html lang="de">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
  <title>BitChord Remote</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; user-select: none; -webkit-tap-highlight-color: transparent; }
    body {
      background: linear-gradient(180deg, #181818 0%, #0d0d0d 100%);
      color: #fff;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: space-between;
      padding: 24px 20px 32px;
      overflow: hidden;
    }
    .header {
      width: 100%;
      max-width: 420px;
      display: flex;
      justify-content: space-between;
      align-items: center;
      font-size: 13px;
      letter-spacing: 1px;
      text-transform: uppercase;
      color: #a7a7a7;
      font-weight: 600;
    }
    .badge {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      background: rgba(255,255,255,0.08);
      padding: 4px 10px;
      border-radius: 12px;
      color: #1db954;
      font-size: 11px;
    }
    .badge-dot {
      width: 7px;
      height: 7px;
      border-radius: 50%;
      background: #1db954;
      box-shadow: 0 0 8px #1db954;
    }
    .art-container {
      width: 100%;
      max-width: 320px;
      aspect-ratio: 1/1;
      margin: 20px 0;
      border-radius: 12px;
      overflow: hidden;
      box-shadow: 0 16px 40px rgba(0,0,0,0.6);
      background: #282828;
      display: flex;
      align-items: center;
      justify-content: center;
    }
    .art-container img {
      width: 100%;
      height: 100%;
      object-fit: cover;
      display: block;
    }
    .track-info {
      width: 100%;
      max-width: 360px;
      margin-bottom: 12px;
    }
    .track-title {
      font-size: 20px;
      font-weight: 700;
      color: #fff;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      margin-bottom: 4px;
    }
    .track-artist {
      font-size: 14px;
      color: #b3b3b3;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .scrub-container {
      width: 100%;
      max-width: 360px;
      margin-bottom: 20px;
    }
    .progress-bar-wrap {
      width: 100%;
      height: 20px;
      display: flex;
      align-items: center;
      cursor: pointer;
    }
    .progress-bg {
      width: 100%;
      height: 4px;
      background: rgba(255,255,255,0.2);
      border-radius: 2px;
      position: relative;
    }
    .progress-fill {
      height: 100%;
      background: #fff;
      border-radius: 2px;
      width: 0%;
      transition: width 0.1s linear;
    }
    .time-labels {
      display: flex;
      justify-content: space-between;
      font-size: 11px;
      color: #b3b3b3;
      margin-top: 4px;
      font-variant-numeric: tabular-nums;
    }
    .controls {
      width: 100%;
      max-width: 360px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 24px;
    }
    .btn {
      background: none;
      border: none;
      color: #b3b3b3;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 8px;
      border-radius: 50%;
      transition: all 0.15s ease;
    }
    .btn:active {
      transform: scale(0.92);
      color: #fff;
    }
    .btn.active {
      color: #1db954;
      position: relative;
    }
    .btn.active::after {
      content: "";
      position: absolute;
      bottom: 2px;
      width: 4px;
      height: 4px;
      border-radius: 50%;
      background: #1db954;
    }
    .btn-play {
      background: #fff;
      color: #000;
      width: 58px;
      height: 58px;
      border-radius: 50%;
      box-shadow: 0 4px 14px rgba(255,255,255,0.25);
    }
    .btn-play:active {
      transform: scale(0.94);
      background: #e0e0e0;
      color: #000;
    }
    .vol-container {
      width: 100%;
      max-width: 360px;
      display: flex;
      align-items: center;
      gap: 12px;
      color: #b3b3b3;
    }
    .vol-slider {
      flex: 1;
      accent-color: #1db954;
      height: 4px;
    }
  </style>
</head>
<body>
  <div class="header">
    <span>BitChord Desktop</span>
    <div class="badge"><div class="badge-dot"></div>Verbunden</div>
  </div>

  <div class="art-container">
    <img id="cover" src="" alt="Album Cover" style="display:none;">
    <svg id="no-cover" width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="#666" stroke-width="1.5">
      <circle cx="12" cy="12" r="10"></circle>
      <path d="M9 12a3 3 0 1 0 6 0 3 3 0 1 0-6 0"></path>
    </svg>
  </div>

  <div class="track-info">
    <div class="track-title" id="title">Kein Titel aktiv</div>
    <div class="track-artist" id="artist">BitChord Windows</div>
  </div>

  <div class="scrub-container">
    <div class="progress-bar-wrap" id="scrubber">
      <div class="progress-bg">
        <div class="progress-fill" id="progress"></div>
      </div>
    </div>
    <div class="time-labels">
      <span id="pos">0:00</span>
      <span id="dur">0:00</span>
    </div>
  </div>

  <div class="controls">
    <button class="btn" id="btn-shuffle" title="Shuffle">
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="16 3 21 3 21 8"></polyline>
        <line x1="4" y1="20" x2="21" y2="3"></line>
        <polyline points="21 16 21 21 16 21"></polyline>
        <line x1="15" y1="15" x2="21" y2="21"></line>
        <line x1="4" y1="4" x2="9" y2="9"></line>
      </svg>
    </button>
    <button class="btn" id="btn-prev" title="Zurück">
      <svg width="26" height="26" viewBox="0 0 24 24" fill="currentColor">
        <polygon points="19 20 9 12 19 4 19 20"></polygon>
        <line x1="5" y1="19" x2="5" y2="5" stroke="currentColor" stroke-width="3"></line>
      </svg>
    </button>
    <button class="btn btn-play" id="btn-play" title="Play/Pause">
      <svg id="play-icon" width="28" height="28" viewBox="0 0 24 24" fill="currentColor">
        <polygon points="5 3 19 12 5 21 5 3"></polygon>
      </svg>
      <svg id="pause-icon" width="28" height="28" viewBox="0 0 24 24" fill="currentColor" style="display:none;">
        <rect x="6" y="4" width="4" height="16"></rect>
        <rect x="14" y="4" width="4" height="16"></rect>
      </svg>
    </button>
    <button class="btn" id="btn-next" title="Weiter">
      <svg width="26" height="26" viewBox="0 0 24 24" fill="currentColor">
        <polygon points="5 4 15 12 5 20 5 4"></polygon>
        <line x1="19" y1="5" x2="19" y2="19" stroke="currentColor" stroke-width="3"></line>
      </svg>
    </button>
    <button class="btn" id="btn-repeat" title="Repeat">
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="17 1 21 5 17 9"></polyline>
        <path d="M3 11V9a4 4 0 0 1 4-4h14"></path>
        <polyline points="7 23 3 19 7 15"></polyline>
        <path d="M21 13v2a4 4 0 0 1-4 4H3"></path>
      </svg>
    </button>
  </div>

  <div class="vol-container">
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
      <path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07"></path>
    </svg>
    <input type="range" class="vol-slider" id="vol" min="0" max="1" step="0.01" value="0.7">
  </div>

  <script>
    let ws;
    let state = { duration_secs: 0, position_secs: 0, is_playing: false };
    let seekHold = false;

    function fmtTime(s) {
      s = Math.floor(s || 0);
      const m = Math.floor(s / 60);
      const sec = s % 60;
      return m + ':' + (sec < 10 ? '0' : '') + sec;
    }

    function connect() {
      const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
      ws = new WebSocket(`${proto}//${location.host}/ws`);

      ws.onmessage = (e) => {
        try {
          const data = JSON.parse(e.data);
          state = data;
          document.getElementById('title').textContent = data.title || 'Kein Titel';
          document.getElementById('artist').textContent = data.artist || 'BitChord';

          const cover = document.getElementById('cover');
          const noCover = document.getElementById('no-cover');
          if (data.cover_url) {
            cover.src = data.cover_url;
            cover.style.display = 'block';
            noCover.style.display = 'none';
          } else {
            cover.style.display = 'none';
            noCover.style.display = 'block';
          }

          document.getElementById('play-icon').style.display = data.is_playing ? 'none' : 'block';
          document.getElementById('pause-icon').style.display = data.is_playing ? 'block' : 'none';

          document.getElementById('btn-shuffle').classList.toggle('active', !!data.shuffle);
          document.getElementById('btn-repeat').classList.toggle('active', data.repeat !== 'off');

          document.getElementById('dur').textContent = fmtTime(data.duration_secs);
          document.getElementById('vol').value = data.volume;

          if (!seekHold) {
            updateProgress(data.position_secs, data.duration_secs);
          }
        } catch(err) {
          console.error(err);
        }
      };

      ws.onclose = () => {
        setTimeout(connect, 1500);
      };
    }

    function updateProgress(pos, dur) {
      document.getElementById('pos').textContent = fmtTime(pos);
      const pct = dur > 0 ? (pos / dur * 100) : 0;
      document.getElementById('progress').style.width = pct + '%';
    }

    // Client-side timer to tick position smoothly while playing
    setInterval(() => {
      if (state.is_playing && !seekHold && state.duration_secs > 0) {
        state.position_secs = Math.min(state.position_secs + 0.5, state.duration_secs);
        updateProgress(state.position_secs, state.duration_secs);
      }
    }, 500);

    function send(action, val) {
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ action: action, value: val }));
      }
    }

    document.getElementById('btn-play').onclick = () => send('Toggle');
    document.getElementById('btn-next').onclick = () => send('Next');
    document.getElementById('btn-prev').onclick = () => send('Prev');
    document.getElementById('btn-shuffle').onclick = () => send('Shuffle');
    document.getElementById('btn-repeat').onclick = () => send('Repeat');

    document.getElementById('vol').oninput = (e) => {
      send('Volume', parseFloat(e.target.value));
    };

    const scrubber = document.getElementById('scrubber');
    scrubber.onclick = (e) => {
      const rect = scrubber.getBoundingClientRect();
      const frac = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      const target = frac * (state.duration_secs || 0);
      send('Seek', target);
      state.position_secs = target;
      updateProgress(target, state.duration_secs);
    };

    connect();
  </script>
</body>
</html>
"##;
