//! 設定管理モジュール
//!
//! 不揮発性メモリ（フラッシュ/EEPROM）に保存する設定値を管理します。
//! センサー設定、フィルタ係数、サンプリング周期などを永続化します。

use crate::error::{ConfigError, Result, SystemError};
use crate::sensor::SensorConfig;
use crate::MAX_SENSORS;
use heapless::Vec;

/// 設定データのマジックナンバー（バージョン識別用）
const CONFIG_MAGIC: u32 = 0x4553_4346; // "ESCF"
/// 設定データのバージョン
const CONFIG_VERSION: u16 = 1;

/// システム設定全体
#[derive(Debug, Clone)]
pub struct SystemConfig {
    /// マジックナンバー
    pub magic: u32,
    /// 設定バージョン
    pub version: u16,
    /// チェックサム
    pub checksum: u16,
    /// サンプリング周期（ミリ秒）
    pub sampling_interval_ms: u32,
    /// フィルタ係数（ローパス）
    pub filter_alpha: i32,
    /// 温度アラート閾値（ミリ度）
    pub temp_alert_threshold: i32,
    /// センサー設定リスト
    pub sensors: Vec<SensorConfig, MAX_SENSORS>,
    /// 通信ボーレート
    pub baud_rate: u32,
    /// デバッグモード
    pub debug_mode: bool,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            magic: CONFIG_MAGIC,
            version: CONFIG_VERSION,
            checksum: 0,
            sampling_interval_ms: 100,
            filter_alpha: 200,
            temp_alert_threshold: 35_000, // 35.000℃
            sensors: Vec::new(),
            baud_rate: 115_200,
            debug_mode: false,
        }
    }
}

impl SystemConfig {
    /// 新しいデフォルト設定を作成
    pub fn new() -> Self {
        let mut config = Self::default();
        config.add_default_sensors();
        config.checksum = config.calculate_checksum();
        config
    }

    /// デフルトのセンサー設定を追加
    fn add_default_sensors(&mut self) {
        // 温度センサー（LM75A）
        let _ = self.sensors.push(SensorConfig {
            id: 1,
            sensor_type: crate::sensor::SensorType::TemperatureI2c,
            i2c_address: Some(0x48),
            adc_channel: None,
            spi_cs: None,
            scale_num: 1,
            scale_den: 1,
            offset: 0,
            enabled: true,
        });

        // 圧力センサー（SPI）
        let _ = self.sensors.push(SensorConfig {
            id: 2,
            sensor_type: crate::sensor::SensorType::PressureSpi,
            i2c_address: None,
            adc_channel: None,
            spi_cs: Some(0),
            scale_num: 1,
            scale_den: 10,
            offset: 0,
            enabled: true,
        });

        // ADCアナログセンサー
        let _ = self.sensors.push(SensorConfig {
            id: 3,
            sensor_type: crate::sensor::SensorType::AnalogAdc,
            i2c_address: None,
            adc_channel: Some(0),
            spi_cs: None,
            scale_num: 3300,
            scale_den: 4095,
            offset: 0,
            enabled: true,
        });
    }

    /// チェックサムを計算（簡易CRC16）
    fn calculate_checksum(&self) -> u16 {
        let mut crc: u16 = 0xFFFF;
        let bytes = self.to_bytes();
        for byte in bytes.iter() {
            crc ^= (*byte as u16) << 8;
            for _ in 0..8 {
                if (crc & 0x8000) != 0 {
                    crc = (crc << 1) ^ 0x1021;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }

    /// 設定をバイト列にシリアライズ（簡易実装）
    fn to_bytes(&self) -> heapless::Vec<u8, 256> {
        let mut bytes = heapless::Vec::new();
        // マジック
        let _ = bytes.extend_from_slice(&self.magic.to_le_bytes());
        // バージョン
        let _ = bytes.extend_from_slice(&self.version.to_le_bytes());
        // サンプリング周期
        let _ = bytes.extend_from_slice(&self.sampling_interval_ms.to_le_bytes());
        // フィルタ係数
        let _ = bytes.extend_from_slice(&self.filter_alpha.to_le_bytes());
        // 温度閾値
        let _ = bytes.extend_from_slice(&self.temp_alert_threshold.to_le_bytes());
        // ボーレート
        let _ = bytes.extend_from_slice(&self.baud_rate.to_le_bytes());
        // デバッグモード
        let _ = bytes.push(self.debug_mode as u8);
        bytes
    }

    /// 設定の妥当性を検証
    pub fn validate(&self) -> Result<()> {
        if self.magic != CONFIG_MAGIC {
            return Err(SystemError::Config(ConfigError::VersionMismatch));
        }
        if self.version != CONFIG_VERSION {
            return Err(SystemError::Config(ConfigError::VersionMismatch));
        }
        let expected = self.calculate_checksum();
        if self.checksum != expected {
            return Err(SystemError::Config(ConfigError::FlashWriteError));
        }
        if self.sampling_interval_ms == 0 || self.sampling_interval_ms > 10_000 {
            return Err(SystemError::Config(ConfigError::InvalidValue));
        }
        if self.filter_alpha < 0 || self.filter_alpha > 1000 {
            return Err(SystemError::Config(ConfigError::InvalidValue));
        }
        Ok(())
    }

    /// 設定を更新してチェックサムを再計算
    pub fn update(&mut self) {
        self.checksum = self.calculate_checksum();
    }
}

/// 設定マネージャー
pub struct ConfigManager {
    config: SystemConfig,
    modified: bool,
}

impl ConfigManager {
    /// 新しい定マネージャーを作成
    pub fn new() -> Self {
        Self {
            config: SystemConfig::new(),
            modified: false,
        }
    }

    /// 設定を取得（参照）
    pub fn config(&self) -> &SystemConfig {
        &self.config
    }

    /// 設定を取得（可変参照）
    pub fn config_mut(&mut self) -> &mut SystemConfig {
        self.modified = true;
        &mut self.config
    }

    /// 設定が変更されたか確認
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// 設定を保存（不揮発性メモリへの書き込み）
    pub fn save(&mut self) -> Result<()> {
        self.config.update();
        // 実際の実装ではフラッシュ書き込みを行う
        // ここでは成功を返す
        self.modified = false;
        Ok(())
    }

    /// 設定を読み込み（不揮発性メモリからの読み出し）
    pub fn load(&mut self) -> Result<()> {
        // 実際実装ではフラッシュ読み出しを行う
        // ここではデフォルト設定を使用
        self.config = SystemConfig::new();
        self.config.validate()?;
        self.modified = false;
        Ok(())
    }

    /// 設定を工場出荷時状態にリセット
    pub fn factory_reset(&mut self) {
        self.config = SystemConfig::new();
        self.modified = true;
    }
}
