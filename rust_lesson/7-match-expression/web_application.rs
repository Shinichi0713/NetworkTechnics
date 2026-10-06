use anyhow::{anyhow, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use kvm_bindings::kvm_userspace_memory_region;
use kvm_ioctls::{Kvm, VcpuExit};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex as StdMutex},
};
use tokio::sync::{broadcast, mpsc, Mutex};

// -----------------------------------------------------------------------------
// データモデル & 内部通信イベント
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VmState {
    Created,
    Running,
    Stopped(String),
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmStatus {
    pub id: usize,
    pub name: String,
    pub state: VmState,
    pub output_log: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    VmList(Vec<VmStatus>),
    VmUpdated(VmStatus),
    LogAppended { id: usize, data: String },
}

#[derive(Deserialize)]
pub struct CreateVmRequest {
    pub id: usize,
    pub name: String,
    pub message: String,
}

pub enum VmInternalEvent {
    StateChanged { id: usize, new_state: VmState },
    OutputReceived { id: usize, data: char },
}

// -----------------------------------------------------------------------------
// KVM 仮想マシン実行処理
// -----------------------------------------------------------------------------

pub struct VirtualMachine {
    pub id: usize,
    pub name: String,
    pub code: Vec<u8>,
}

impl VirtualMachine {
    pub fn spawn_run(self, event_tx: mpsc::Sender<VmInternalEvent>) {
        tokio::task::spawn_blocking(move || {
            let id = self.id;
            let _ = event_tx.blocking_send(VmInternalEvent::StateChanged {
                id,
                new_state: VmState::Running,
            });

            match self.execute_kvm(&event_tx) {
                Ok(reason) => {
                    let _ = event_tx.blocking_send(VmInternalEvent::StateChanged {
                        id,
                        new_state: VmState::Stopped(reason),
                    });
                }
                Err(e) => {
                    let _ = event_tx.blocking_send(VmInternalEvent::StateChanged {
                        id,
                        new_state: VmState::Failed(e.to_string()),
                    });
                }
            }
        });
    }

    fn execute_kvm(&self, event_tx: &mpsc::Sender<VmInternalEvent>) -> Result<String> {
        let kvm = Kvm::new()?;
        let vm = kvm.create_vm()?;
        let mem_size = 0x1000;
        let load_addr = 0x1000;

        let guest_mem = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                mem_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_ANONYMOUS | libc::MAP_SHARED,
                -1,
                0,
            )
        };
        if guest_mem == libc::MAP_FAILED {
            return Err(anyhow!("mmap 失敗"));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(self.code.as_ptr(), guest_mem as *mut u8, self.code.len());
        }

        let mem_region = kvm_userspace_memory_region {
            slot: 0,
            flags: 0,
            guest_phys_addr: load_addr as u64,
            memory_size: mem_size as u64,
            userspace_addr: guest_mem as u64,
        };
        unsafe {
            vm.set_user_memory_region(mem_region)?;
        }

        let mut vcpu = vm.create_vcpu(0)?;
        let mut sregs = vcpu.get_sregs()?;
        sregs.cs.base = 0;
        sregs.cs.selector = 0;
        vcpu.set_sregs(&sregs)?;

        let mut regs = vcpu.get_regs()?;
        regs.rip = load_addr as u64;
        regs.rflags = 0x2;
        vcpu.set_regs(&regs)?;

        let exit_reason = loop {
            match vcpu.run()? {
                VcpuExit::IoOut(addr, data) => {
                    if addr == 0x10 {
                        if let Some(&byte) = data.first() {
                            let _ = event_tx.blocking_send(VmInternalEvent::OutputReceived {
                                id: self.id,
                                data: byte as char,
                            });
                        }
                    }
                }
                VcpuExit::Hlt => break "HLT 正常終了".to_string(),
                VcpuExit::FailEntry(r) => return Err(anyhow!("KVM エントリ失敗: {}", r)),
                VcpuExit::InternalError => return Err(anyhow!("KVM 内部エラー")),
                _ => {}
            }
        };

        unsafe {
            libc::munmap(guest_mem, mem_size);
        }
        Ok(exit_reason)
    }
}

// -----------------------------------------------------------------------------
// アプリケーション状態管理 (AppState)
// -----------------------------------------------------------------------------

pub struct AppState {
    pub vms: Arc<Mutex<HashMap<usize, VmStatus>>>,
    pub internal_tx: mpsc::Sender<VmInternalEvent>,
    pub ws_broadcast: broadcast::Sender<WsMessage>,
}

// -----------------------------------------------------------------------------
// WEB アプリケーション & API ハンドラ
// -----------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Web GUI 付き KVM VM マネージャ起動 ===");

    let (internal_tx, mut internal_rx) = mpsc::channel::<VmInternalEvent>(100);
    let (ws_broadcast, _) = broadcast::channel::<WsMessage>(100);

    let vms = Arc::new(Mutex::new(HashMap::<usize, VmStatus>::new()));

    let app_state = Arc::new(AppState {
        vms: vms.clone(),
        internal_tx,
        ws_broadcast: ws_broadcast.clone(),
    });

    // バックグラウンドタスク: KVMイベントを受信して状態更新＆WebSocketへブロードキャスト
    let vms_clone = vms.clone();
    let ws_bc_clone = ws_broadcast.clone();
    tokio::spawn(async move {
        while let Some(event) = internal_rx.recv().await {
            let mut guard = vms_clone.lock().await;
            match event {
                VmInternalEvent::StateChanged { id, new_state } => {
                    if let Some(vm) = guard.get_mut(&id) {
                        vm.state = new_state;
                        let _ = ws_bc_clone.send(WsMessage::VmUpdated(vm.clone()));
                    }
                }
                VmInternalEvent::OutputReceived { id, data } => {
                    if let Some(vm) = guard.get_mut(&id) {
                        vm.output_log.push(data);
                        let _ = ws_bc_clone.send(WsMessage::LogAppended {
                            id,
                            data: data.to_string(),
                        });
                    }
                }
            }
        }
    });

    // ルーティング設定
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/vms", get(list_vms_handler).post(create_vm_handler))
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Web GUI にアクセスしてください: http://localhost:3000");
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// REST API: 全VMリスト取得
async fn list_vms_handler(State(state): State<Arc<AppState>>) -> Json<Vec<VmStatus>> {
    let guard = state.vms.lock().await;
    Json(guard.values().cloned().collect())
}

// REST API: VM作成＆起動
async fn create_vm_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateVmRequest>,
) -> Json<VmStatus> {
    // 入力文字列から x86 機械語コードを動的に生成
    let mut code = Vec::new();
    for ch in req.message.chars() {
        code.extend_from_slice(&[0xb4, ch as u8, 0xe6, 0x10]);
    }
    code.extend_from_slice(&[0xb4, b'\n', 0xe6, 0x10, 0xf4]); // 改行 & HLT

    let vm_status = VmStatus {
        id: req.id,
        name: req.name.clone(),
        state: VmState::Created,
        output_log: String::new(),
    };

    state.vms.lock().await.insert(req.id, vm_status.clone());
    let _ = state.ws_broadcast.send(WsMessage::VmUpdated(vm_status.clone()));

    let vm = VirtualMachine {
        id: req.id,
        name: req.name,
        code,
    };
    vm.spawn_run(state.internal_tx.clone());

    Json(vm_status)
}

// WebSocket ハンドラ
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    // 初期接続時に現在の全VM状態を送信
    let initial_vms = {
        let guard = state.vms.lock().await;
        guard.values().cloned().collect::<Vec<_>>()
    };
    let msg = serde_json::to_string(&WsMessage::VmList(initial_vms)).unwrap();
    let _ = socket.send(Message::Text(msg)).await;

    let mut rx = state.ws_broadcast.subscribe();
    while let Ok(ws_msg) = rx.recv().await {
        if let Ok(json_text) = serde_json::to_string(&ws_msg) {
            if socket.send(Message::Text(json_text)).await.is_err() {
                break; // クライアント切断時
            }
        }
    }
}

// GUI 用 HTML / JavaScript インライン配信
async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

const INDEX_HTML: &str = r#"
<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <title>KVM Virtual Machine Web Manager</title>
    <style>
        body { font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; background: #1e1e2e; color: #cdd6f4; margin: 20px; }
        h1 { color: #89b4fa; }
        .container { display: flex; gap: 20px; }
        .form-card, .list-card { background: #313244; padding: 20px; border-radius: 8px; box-shadow: 0 4px 6px rgba(0,0,0,0.3); }
        .form-card { flex: 1; }
        .list-card { flex: 2; }
        label { display: block; margin-top: 10px; color: #a6adc8; }
        input[type="text"], input[type="number"] { width: 100%; padding: 8px; margin-top: 5px; border-radius: 4px; border: 1px solid #45475a; background: #181825; color: #cdd6f4; box-sizing: border-box; }
        button { margin-top: 15px; width: 100%; padding: 10px; border: none; border-radius: 4px; background: #89b4fa; color: #11111b; font-weight: bold; cursor: pointer; }
        button:hover { background: #b4befe; }
        table { width: 100%; border-collapse: collapse; margin-top: 10px; }
        th, td { padding: 10px; border-bottom: 1px solid #45475a; text-align: left; }
        th { background: #181825; color: #89b4fa; }
        .status-badge { padding: 4px 8px; border-radius: 4px; font-weight: bold; font-size: 0.85em; }
        .status-running { background: #a6e3a1; color: #11111b; }
        .status-stopped { background: #f38ba8; color: #11111b; }
        .status-created { background: #f9e2af; color: #11111b; }
        pre { margin: 0; background: #181825; padding: 5px; border-radius: 4px; font-family: monospace; }
    </style>
</head>
<body>
    <h1>⚡ KVM 仮想マシンコントロールパネル</h1>
    <div class="container">
        <div class="form-card">
            <h2>新規VMの作成・起動</h2>
            <form id="createForm">
                <label>VM ID:</label>
                <input type="number" id="vmId" value="1" required>
                <label>VM 名:</label>
                <input type="text" id="vmName" value="vm-node-01" required>
                <label>出力メッセージ (ゲストコード化):</label>
                <input type="text" id="vmMsg" value="Hello KVM Rust!" required>
                <button type="submit">VM 起動</button>
            </form>
        </div>
        <div class="list-card">
            <h2>稼働中 VM 一覧 (リアルタイム)</h2>
            <table>
                <thead>
                    <tr>
                        <th>ID</th>
                        <th>VM名</th>
                        <th>ステータス</th>
                        <th>シリアル出力ログ</th>
                    </tr>
                </thead>
                <tbody id="vmTableBody"></tbody>
            </table>
        </div>
    </div>

    <script>
        const vmMap = new Map();

        // WebSocket 接続 setup
        const ws = new WebSocket(`ws://${location.host}/ws`);
        ws.onmessage = (event) => {
            const msg = JSON.parse(event.data);
            if (msg.type === "VmList") {
                msg.payload.forEach(vm => vmMap.set(vm.id, vm));
                renderTable();
            } else if (msg.type === "VmUpdated") {
                vmMap.set(msg.payload.id, msg.payload);
                renderTable();
            } else if (msg.type === "LogAppended") {
                const vm = vmMap.get(msg.payload.id);
                if (vm) {
                    vm.output_log += msg.payload.data;
                    renderTable();
                }
            }
        };

        function renderTable() {
            const tbody = document.getElementById("vmTableBody");
            tbody.innerHTML = "";
            vmMap.forEach(vm => {
                const tr = document.createElement("tr");
                let statusClass = "status-created";
                let statusText = "CREATED";
                if (vm.state === "Running") {
                    statusClass = "status-running";
                    statusText = "RUNNING";
                } else if (typeof vm.state === "object" && "Stopped" in vm.state) {
                    statusClass = "status-stopped";
                    statusText = "STOPPED";
                }

                tr.innerHTML = `
                    <td>${vm.id}</td>
                    <td><strong>${vm.name}</strong></td>
                    <td><span class="status-badge ${statusClass}">${statusText}</span></td>
                    <td><pre>${vm.output_log}</pre></td>
                `;
                tbody.appendChild(tr);
            });
        }

        document.getElementById("createForm").addEventListener("submit", async (e) => {
            e.preventDefault();
            const id = parseInt(document.getElementById("vmId").value);
            const name = document.getElementById("vmName").value;
            const message = document.getElementById("vmMsg").value;

            await fetch("/api/vms", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ id, name, message })
            });

            document.getElementById("vmId").value = id + 1;
        });
    </script>
</body>
</html>
"#;