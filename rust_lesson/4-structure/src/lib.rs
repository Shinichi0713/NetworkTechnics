//! 組み込みセンサーシステム
//! 
//! no_std環境向けの複数センサー管理・データ収集・通信システム
//! 
//! ## アーキテクチャ
//! - `sensor`: センサーデバイスドライバ群とマネージャー
//! - `filter`: デジタルフィルタ（ローパス、カルマン）
//! - `comms`: シリアル通信とコマンド処理
//! - `system`: システム状態管理とステートマシン
//! - `config`: 不揮発性設定の管理
//! - `error`: 統合エラー型

#![no_std]
#![warn(missing_docs)]

pub mod comms;
pub mod config;
pub mod error;
pub mod filter;
pub mod hal_mock;
pub mod sensor;
pub mod system;

/// システムバージョン
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 最大センサー数
pub const MAX_SENSORS: usize = 8;

/// サンプリング周期（ミリ秒）
pub const SAMPLING_INTERVAL_MS: u32 = 100;

/// 通信受信バッファサイズ
pub const COMM_RX_BUFFER_SIZE: usize = 64;

/// 通信送信バッファサイズ
pub const COMM_TX_BUFFER_SIZE: usize = 256;
