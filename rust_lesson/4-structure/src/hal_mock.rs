//! ハードウェア抽象化レイヤー（HAL）モック
//!
//! 実際のMCUでは各ベンダーのHALクレートを使用します。
//! このモックはテストおよび構造理解用です。

use crate::error::{Result, SystemError};

/// I2Cバスのモック
pub struct MockI2c;

impl MockI2c {
    /// 新しいI2Cモックを作成
    pub fn new() -> Self {
        Self
    }

    /// デバイスに書き込み
    pub fn write(&mut self, _addr: u8, _data: &[u8]) -> Result<()> {
        // 実際の実装ではHALのI2Cトレイトを使用
        Ok(())
    }

    /// デバイスから読み出し
    pub fn read(&mut self, _addr: u8, _buffer: &mut [u8]) -> Result<()> {
        // モックデータを返す（テスト用）
        if !_buffer.is_empty() {
            _buffer[0] = 0x19; // 約25℃の温度データ（上位バイト）
            if _buffer.len() > 1 {
                _buffer[1] = 0x80;
            }
        }
        Ok(())
    }

    /// 書き込み後に読み出し
    pub fn write_read(
        &mut self,
        _addr: u8,
        _write_data: &[u8],
        _read_buffer: &mut [u8],
    ) -> Result<()> {
        self.read(_addr, _read_buffer)
    }
}

/// SPIバスのモック
pub struct MockSpi;

impl MockSpi {
    /// 新しいSPIモックを作成
    pub fn new() -> Self {
        Self
    }

    /// データ転送
    pub fn transfer(&mut self, _write: &[u8], read: &mut [u8]) -> Result<()> {
        // モックデータ
        for (i, b) in read.iter_mut().enumerate() {
            *b = i as u8;
        }
        Ok(())
    }
}

/// UARTのモック
pub struct MockUart;

impl MockUart {
    /// 新しいUARTモックを作成
    pub fn new() -> Self {
        Self
    }

    /// 1バイト送信
    pub fn write_byte(&mut self, _byte: u8) -> Result<()> {
        Ok(())
    }

    /// バイト列送信
    pub fn write_slice(&mut self, data: &[u8]) -> Result<()> {
        for &b in data {
            self.write_byte(b)?;
        }
        Ok(())
    }

    /// 受信バイトの確認
    pub fn read_byte(&mut self) -> Result<u8> {
        // モックでは常に0を返す
        Ok(0)
    }

    /// 受信可能か確認
    pub fn is_readable(&self) -> bool {
        false
    }
}

/// タイマーのモック
pub struct MockTimer;

impl MockTimer {
    /// 新しいタイマーモックを作成
    pub fn new() -> Self {
        Self
    }

    /// ミリ秒待機
    pub fn delay_ms(&mut self, _ms: u32) {
        // 実際の実装では組み込みHALのDelayを使用
    }

    /// マイクロ秒待機
    pub fn delay_us(&mut self, _us: u32) {
        // 実際の実装では組み込みHALのDelayを使用
    }

    /// 現在のタイムスタンプ（ミリ秒）を取得
    pub fn now_ms(&self) -> u32 {
        // モックでは固定値
        0
    }
}

/// GPIOピンのモック
pub struct MockPin {
    state: bool,
}

impl MockPin {
    /// 新しいピンモックを作成
    pub fn new() -> Self {
        Self { state: false }
    }

    /// ピンをHighに設定
    pub fn set_high(&mut self) {
        self.state = true;
    }

    /// ピンをLowに設定
    pub fn set_low(&mut self) {
        self.state = false;
    }

    /// ピン状態を取得
    pub fn is_high(&self) -> bool {
        self.state
    }

    /// ピンをトグル
    pub fn toggle(&mut self) {
        self.state = !self.state;
    }
}

/// ADCのモック
pub struct MockAdc;

impl MockAdc {
    /// 新しいADCモックを作成
    pub fn new() -> Self {
        Self
    }

    /// チャンネルから読み取り
    pub fn read(&mut self, _channel: u8) -> Result<u16> {
        // モック値: 12bit ADCの半分
        Ok(2048)
    }
}
