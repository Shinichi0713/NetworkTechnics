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