//! 通信モジュール
//!
//! シリアル通信（UART）を介したコマンド受信・データ送信を管理します。
//! コマンドプロトコルはテキストベースで、以下の形式:
//!   COMMAND arg1 arg2 ...\n

use crate::error::{CommunicationError, Result, SystemError};
use crate::hal_mock::MockUart;
use crate::sensor::SensorReading;
use heapless::String;
use heapless::Vec;

/// 受信バッファサイズ
const RX_BUFFER_SIZE: usize = 64;
/// 送信バッファサイズ
const TX_BUFFER_SIZE: usize = 256;

/// コマンド種別
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// 全センサーデータ取得
    GetAll,
    /// 特定センサーデータ取得
    GetSensor { id: u8 },
    /// フィルタ係数設定
    SetFilter { alpha: i32 },
    /// サプリング周期設定
    SetInterval { ms: u32 },
    /// システム状態取得
    GetStatus,
    /// バージョン取得
    GetVersion,
    /// 不明なコマンド
    Unknown,
}

/// 通信マネージャー
pub struct CommunicationManager {
    uart: MockUart,
    rx_buffer: String<RX_BUFFER_SIZE>,
}

impl CommunicationManager {
    /// 新しい通信マネージャーを作成
    pub fn new(uart: MockUart) -> Self {
        Self {
            uart,
            rx_buffer: String::new(),
        }
    }

    /// 受信バッファをチェックしてコマンドを解析
    pub fn poll_command(&mut self) -> Result<Option<Command>> {
        if !self.uart.is_readable() {
            return Ok(None);
        }

        // 1バイトずつ読み込み、改行まで蓄積
        loop {
            let byte = self.uart.read_byte()?;
            if byte == b'\n' || byte == b'\r' {
                if !self.rx_buffer.is_empty() {
                    let cmd = self.parse_command();
                    self.rx_buffer.clear();
                    return Ok(Some(cmd));
                }
            } else if byte >= 0x20 && byte <= 0x7E {
                // 印字可能文字のみ
                let _ = self.rx_buffer.push(byte as char);
            }

            if !self.uart.is_readable() {
                break;
            }
        }

        Ok(None)
    }

    /// コマンド文字列を解析
    fn parse_command(&self) -> Command {
        let parts: Vec<&str, 8> = self.rx_buffer.split_whitespace().collect();
        if parts.is_empty() {
            return Command::Unknown;
        }

        match parts[0].to_uppercase().as_str() {
            "GETALL" => Command::GetAll,
            "GET" => {
                if parts.len() >= 2 {
                    if let Ok(id) = parts[1].parse::<u8>() {
                        return Command::GetSensor { id };
                    }
                }
                Command::Unknown
            }
            "FILTER" => {
                if parts.len() >= 2 {
                    if let Ok(alpha) = parts[1].parse::<i32>() {
                        return Command::SetFilter { alpha };
                    }
                }
                Command::Unknown
            }
            "INTERVAL" => {
                if parts.len() >= 2 {
                    if let Ok(ms) = parts[1].parse::<u32>() {
                        return Command::SetInterval { ms };
                    }
                }
                Command::Unknown
            }
            "STATUS" => Command::GetStatus,
            "VERSION" => Command::GetVersion,
            _ => Command::Unknown,
        }
    }

    /// センサーデータをJSON風のテキストフォーマットで送信
    pub fn send_sensor_data(&mut self, readings: &[SensorReading]) -> Result<()> {
        let mut tx_buf = String::<TX_BUFFER_SIZE>::new();
        let _ = tx_buf.push_str("{");
        let _ = tx_buf.push_str("\"sensors\":[");

        for (i, r) in readings.iter().enumerate() {
            if i > 0 {
                let _ = tx_buf.push_str(",");
            }
            let _ = tx_buf.push_str("{");
            // id
            let _ = tx_buf.push_str("\"id\":");
            let _ = tx_buf.push_str(itoa_simple(r.sensor_id as i32).as_str());
            let _ = tx_buf.push_str(",");
            // value
            let _ = tx_buf.push_str("\"val\":");
            let _ = tx_buf.push_str(itoa_simple(r.scaled_value).as_str());
            let _ = tx_buf.push_str(",");
            // quality
            let _ = tx_buf.push_str("\"q\":");
            let q_str = match r.quality {
                crate::sensor::DataQuality::Good => "G",
                crate::sensor::DataQuality::Uncertain => "U",
                crate::sensor::DataQuality::Bad => "B",
            };
            let _ = tx_buf.push_str("\"");
            let _ = tx_buf.push_str(q_str);
            let _ = tx_buf.push_str("\"");
            let _ = tx_buf.push_str("}");
        }

        let _ = tx_buf.push_str("]}\n");
        self.send_string(&tx_buf)
    }

    /// ステータスメッセージを送信
    pub fn send_status(&mut self, status: &str) -> Result<()> {
        let mut tx_buf = String::<TX_BUFFER_SIZE>::new();
        let _ = tx_buf.push_str("STATUS: ");
        let _ = tx_buf.push_str(status);
        let _ = tx_buf.push_str("\n");
        self.send_string(&tx_buf)
    }

    /// バージョン情報を送信
    pub fn send_version(&mut self, version: &str) -> Result<()> {
        let mut tx_buf = String::<TX_BUFFER_SIZE>::new();
        let _ = tx_buf.push_str("VERSION: ");
        let _ = tx_buf.push_str(version);
        let _ = tx_buf.push_str("\n");
        self.send_string(&tx_buf)
    }

    /// エラーメッセージを送信
    pub fn send_error(&mut self, msg: &str) -> Result<()> {
        let mut tx_buf = String::<TX_BUFFER_SIZE>::new();
        let _ = tx_buf.push_str("ERROR: ");
        let _ = tx_buf.push_str(msg);
        let _ = tx_buf.push_str("\n");
        self.send_string(&tx_buf)
    }

    /// 確認応答を送信
    pub fn send_ack(&mut self) -> Result<()> {
        self.send_string("OK\n")
    }

    /// 文字列をUARTに送信
    fn send_string(&mut self, s: &str) -> Result<()> {
        for byte in s.bytes() {
            self.uart.write_byte(byte)?;
        }
        Ok(())
    }
}

/// no_std環境用の簡易整数→文字列変換
fn itoa_simple(n: i32) -> String<16> {
    let mut result = String::<16>::new();
    if n < 0 {
        let _ = result.push('-');
    }
    let mut n = n.abs();
    let mut buf = [0u8; 12];
    let mut i = 0;
    loop {
        buf[i] = (n % 10) as u8 + b'0';
        n /= 10;
        i += 1;
        if n == 0 {
            break;
        }
    }
    for j in 0..i {
        let _ = result.push(buf[i - 1 - j] as char);
    }
    result
}
