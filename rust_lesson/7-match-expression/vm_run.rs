use anyhow::{anyhow, Result};
use kvm_bindings::{kvm_userspace_memory_region, KVM_MEM_LOG_DIRTY_PAGES};
use kvm_ioctls::{Kvm, VcpuExit};
use std::slice;

fn main() -> Result<()> {
    println!("=== Rust KVM 仮想マシン構築サンプル ===");

    // 1. KVM デバイス (/dev/kvm) のオープン
    let kvm = Kvm::new()?;

    // 2. VM (仮想マシン) インスタンスの作成
    let vm = kvm.create_vm()?;

    // 3. ゲストメモリの確保 (1 ページ: 4000 形式 / 4KB)
    let mem_size = 0x1000;
    let load_addr = 0x1000; // ゲスト物理アドレス
    
    // ホスト上の無名共有メモリ領域を割り当て
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
        return Err(anyhow!("mmap によるメモリ確保に失敗しました"));
    }

    // 4. ゲストメモリに実行コードを書き込む
    // 動作: ポート 0x10 に 'H', 'e', 'l', 'l', 'o', '\n' を出力し、HLT する x86 16bit アセンブリコード
    let code: [u8; 15] = [
        0xb4, b'H', 0xe6, 0x10, // mov ah, 'H'; out 0x10, ah
        0xb4, b'e', 0xe6, 0x10, // mov ah, 'e'; out 0x10, ah
        0xb4, b'!', 0xe6, 0x10, // mov ah, '!'; out 0x10, ah
        0xb4, b'\n', 0xe6, 0x10,// mov ah, '\n'; out 0x10, ah
        0xf4,                   // hlt
    ];

    unsafe {
        std::ptr::copy_nonoverlapping(code.as_ptr(), guest_mem as *mut u8, code.len());
    }

    // KVM にゲストメモリ領域を登録
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

    // 5. vCPU (仮想CPU) の作成
    let mut vcpu = vm.create_vcpu(0)?;

    // 6. vCPU の初期レジスタ設定 (x86 リアルモード設定)
    let mut sregs = vcpu.get_sregs()?;
    sregs.cs.base = 0;
    sregs.cs.selector = 0;
    vcpu.set_sregs(&sregs)?;

    let mut regs = vcpu.get_regs()?;
    regs.rip = load_addr as u64; // エントリポイントを指定
    regs.rflags = 0x2;           // x86 必須フラグ
    vcpu.set_regs(&regs)?;

    println!("VM 初期化完了。vCPU 実行を開始します...\n");

    // 7. VM 実行ループ (VM-Exit ハンドリング)
    loop {
        match vcpu.run()? {
            VcpuExit::IoIn(addr, data) => {
                println!("[VM-Exit] IO In: port=0x{:x}, len={}", addr, data.len());
            }
            VcpuExit::IoOut(addr, data) => {
                // ポート 0x10 への書き込みをキャッチしてホストの標準出力に出力
                if addr == 0x10 {
                    if let Some(&byte) = data.first() {
                        print!("{}", byte as char);
                    }
                } else {
                    println!("[VM-Exit] IO Out: port=0x{:x}", addr);
                }
            }
            VcpuExit::Hlt => {
                println!("\n[VM-Exit] ゲスト CPU が HLT (停止) 命令を実行しました。");
                break;
            }
            VcpuExit::FailEntry(reason) => {
                return Err(anyhow!("VM エントリ失敗: reason={}", reason));
            }
            VcpuExit::InternalError => {
                return Err(anyhow!("KVM 内部エラーが発生しました"));
            }
            other => {
                println!("[VM-Exit] その他のイベント: {:?}", other);
            }
        }
    }

    // メモリの解放
    unsafe {
        libc::munmap(guest_mem, mem_size);
    }

    Ok(())
}

use anyhow::{anyhow, Result};
use kvm_bindings::kvm_userspace_memory_region;
use kvm_ioctls::{Kvm, VcpuExit};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration};

// -----------------------------------------------------------------------------
// VMデータモデル & イベント定義
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmState {
    Created,
    Running,
    Stopped(String), // 停止理由
    Failed(String),  // エラー内容
}

#[derive(Debug)]
pub struct VmStatus {
    pub id: usize,
    pub name: String,
    pub state: VmState,
    pub output_log: String,
}

/// VM内部からマネージャへ送信されるイベントメッセージ
#[derive(Debug)]
pub enum VmEvent {
    StateChanged { id: usize, new_state: VmState },
    OutputReceived { id: usize, data: char },
}

// -----------------------------------------------------------------------------
// 仮想マシン (VM) 実行コアエンジン
// -----------------------------------------------------------------------------

pub struct VirtualMachine {
    pub id: usize,
    pub name: String,
    pub code: Vec<u8>,
}

impl VirtualMachine {
    pub fn new(id: usize, name: &str, code: Vec<u8>) -> Self {
        Self {
            id,
            name: name.to_string(),
            code,
        }
    }

    /// 別スレッド/タスク上でVMを実行し、イベントをチャンネル経由で通知する
    pub fn spawn_run(self, event_tx: mpsc::Sender<VmEvent>) {
        tokio::task::spawn_blocking(move || {
            let id = self.id;

            // 状態変更通知: Running
            let _ = event_tx.blocking_send(VmEvent::StateChanged {
                id,
                new_state: VmState::Running,
            });

            match self.execute_kvm(&event_tx) {
                Ok(reason) => {
                    let _ = event_tx.blocking_send(VmEvent::StateChanged {
                        id,
                        new_state: VmState::Stopped(reason),
                    });
                }
                Err(e) => {
                    let _ = event_tx.blocking_send(VmEvent::StateChanged {
                        id,
                        new_state: VmState::Failed(e.to_string()),
                    });
                }
            }
        });
    }

    fn execute_kvm(&self, event_tx: &mpsc::Sender<VmEvent>) -> Result<String> {
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
            return Err(anyhow!("mmap 割り当て失敗"));
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
                            let _ = event_tx.blocking_send(VmEvent::OutputReceived {
                                id: self.id,
                                data: byte as char,
                            });
                        }
                    }
                }
                VcpuExit::Hlt => {
                    break "HLT 命令実行により正常終了".to_string();
                }
                VcpuExit::FailEntry(reason) => {
                    return Err(anyhow!("KVM エントリ失敗: {}", reason));
                }
                VcpuExit::InternalError => {
                    return Err(anyhow!("KVM 内部エラー"));
                }
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
// VM マネージャ (状態管理・一括操作)
// -----------------------------------------------------------------------------

pub struct VmManager {
    vms: Arc<Mutex<HashMap<usize, VmStatus>>>,
    event_tx: mpsc::Sender<VmEvent>,
}

impl VmManager {
    pub fn new() -> (Self, mpsc::Receiver<VmEvent>) {
        let (tx, rx) = mpsc::channel(100);
        let manager = Self {
            vms: Arc::new(Mutex::new(HashMap::new())),
            event_tx: tx,
        };
        (manager, rx)
    }

    /// VMの登録と起動処理
    pub async fn launch_vm(&self, id: usize, name: &str, code: Vec<u8>) {
        let status = VmStatus {
            id,
            name: name.to_string(),
            state: VmState::Created,
            output_log: String::new(),
        };

        self.vms.lock().await.insert(id, status);

        let vm = VirtualMachine::new(id, name, code);
        vm.spawn_run(self.event_tx.clone());
    }

    /// 全VMの現在のステータスダッシュボード表示
    pub async fn print_dashboard(&self) {
        let vms = self.vms.lock().await;
        println!("\n=================== VM マネージャ ダッシュボード ===================");
        println!("{:<5} | {:<12} | {:<20} | {:<20}", "ID", "Name", "State", "Output Log");
        println!("{:-<67}", "");
        for (_, status) in vms.iter() {
            let state_str = match &status.state {
                VmState::Created => "CREATED".to_string(),
                VmState::Running => "RUNNING".to_string(),
                VmState::Stopped(_) => "STOPPED".to_string(),
                VmState::Failed(_) => "FAILED".to_string(),
            };
            println!(
                "{:<5} | {:<12} | {:<20} | {:<20}",
                status.id, status.name, state_str, status.output_log
            );
        }
        println!("===================================================================\n");
    }

    /// イベントループの実行（状態更新・ログ収集）
    pub async fn run_event_listener(vms: Arc<Mutex<HashMap<usize, VmStatus>>>, mut rx: mpsc::Receiver<VmEvent>) {
        while let Some(event) = rx.recv().await {
            let mut guard = vms.lock().await;
            match event {
                VmEvent::StateChanged { id, new_state } => {
                    if let Some(vm) = guard.get_mut(&id) {
                        vm.state = new_state;
                    }
                }
                VmEvent::OutputReceived { id, data } => {
                    if let Some(vm) = guard.get_mut(&id) {
                        vm.output_log.push(data);
                    }
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// メインルーチン
// -----------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== KVM マルチVMマネージャ起動 ===");

    let (manager, rx) = VmManager::new();

    // イベント受信ループを非同期タスクとしてバックグラウンド実行
    let vms_ref = Arc::clone(&manager.vms);
    tokio::spawn(VmManager::run_event_listener(vms_ref, rx));

    // 各VM用のゲストコード作成 (x86 16bit 機械語)
    // VM 1: "VM-1\n" を出力して終了
    let code_vm1: Vec<u8> = vec![
        0xb4, b'V', 0xe6, 0x10,
        0xb4, b'M', 0xe6, 0x10,
        0xb4, b'-', 0xe6, 0x10,
        0xb4, b'1', 0xe6, 0x10,
        0xb4, b'\n', 0xe6, 0x10,
        0xf4,
    ];

    // VM 2: "Node-B\n" を出力して終了
    let code_vm2: Vec<u8> = vec![
        0xb4, b'N', 0xe6, 0x10,
        0xb4, b'o', 0xe6, 0x10,
        0xb4, b'd', 0xe6, 0x10,
        0xb4, b'e', 0xe6, 0x10,
        0xb4, b'-', 0xe6, 0x10,
        0xb4, b'B', 0xe6, 0x10,
        0xb4, b'\n', 0xe6, 0x10,
        0xf4,
    ];

    // VM 3: "Worker3\n" を出力して終了
    let code_vm3: Vec<u8> = vec![
        0xb4, b'W', 0xe6, 0x10,
        0xb4, b'o', 0xe6, 0x10,
        0xb4, b'r', 0xe6, 0x10,
        0xb4, b'k', 0xe6, 0x10,
        0xb4, b'e', 0xe6, 0x10,
        0xb4, b'r', 0xe6, 0x10,
        0xb4, b'3', 0xe6, 0x10,
        0xb4, b'\n', 0xe6, 0x10,
        0xf4,
    ];

    // 複数VMの順次・並列起動
    println!("VMを順次スロットへ投入・起動します...");
    manager.launch_vm(1, "vm-alpha", code_vm1).await;
    manager.launch_vm(2, "vm-beta", code_vm2).await;
    manager.launch_vm(3, "vm-gamma", code_vm3).await;

    // 定期的にダッシュボードを出力して管理・監視
    for _ in 0..3 {
        sleep(Duration::from_millis(300)).await;
        manager.print_dashboard().await;
    }

    Ok(())
}