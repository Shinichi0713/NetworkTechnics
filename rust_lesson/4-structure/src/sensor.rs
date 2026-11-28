//! センサーモジュール
//!
//! 複数のセンサーデバイスを管理し、統一されたインタフェースでデータを提供します。
//! サポートするセンサー:
//! - I2C温度センサー (LM75A互換)
//! - SPI圧力センサー
//! - ADCアナログセンサー
//! - デジタルI/Oセンサー

use crate::error::{Result, SensorError, SystemError};
use crate::hal_mock::{MockAdc, MockI2c, MockSpi};
use heapless::Vec;

/// 最大センサー数
use crate::MAX_SENSORS;

/// センサーID型
pub type SensorId = u8;

/// センサータイプ
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SensorType {
    /// I2C接続温度センサー (LM75A)
    TemperatureI2c,
    /// SPI接続圧力センサー
    PressureSpi,
    /// ADC接続アナログセンサー
    AnalogAdc,
    /// デジタルI/Oセンサー
    DigitalIo,
}

/// センサーデータ
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorReading {
    /// センサーID
    pub sensor_id: SensorId,
    /// 生のセンサー値
    pub raw_value: i32,
    /// 変換後の物理値（浮動小数点の代わりに固定小数）
    /// 単位: 1/1000（例: 25000 = 25.000℃）
    pub scaled_value: i32,
    /// データ品質フラグ
    pub quality: DataQuality,
    /// タイムスタンプ（ミリ秒）
    pub timestamp_ms: u32,
}

/// データ品質
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataQuality {
    /// 正常
    Good,
    /// 不確定（値は有効だが信頼性に疑問あり）
    Uncertain,
    /// 不良（値が無効）
    Bad,
}

/// センサー設定
#[derive(Debug, Clone, Copy)]
pub struct SensorConfig {
    /// センサーID
    pub id: SensorId,
    /// センサータイプ
    pub sensor_type: SensorType,
    /// I2Cアドレス（I2Cセンサーの場合）
    pub i2c_address: Option<u8>,
    /// ADCチャンネル（ADCセンーの場合）
    pub adc_channel: Option<u8>,
    /// SPIチップセレクトピン番号（SPIセンサーの場合）
    pub spi_cs: Option<u8>,
    /// スケーリング係数（分子）
    pub scale_num: i32,
    /// スケーリング係数（分母）
    pub scale_den: i32,
    /// オフセット値
    pub offset: i32,
    /// サンプリング有効/無効
    pub enabled: bool,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            id: 0,
            sensor_type: SensorType::TemperatureI2c,
            i2c_address: None,
            adc_channel: None,
            spi_cs: None,
            scale_num: 1,
            scale_den: 1,
            offset: 0,
            enabled: true,
        }
    }
}

/// LM75A温度センサードライバ
pub struct Lm75aDriver {
    i2c: MockI2c,
    address: u8,
}

impl Lm75aDriver {
    const TEMP_REGISTER: u8 = 0x00;

    /// 新しいドライバを作成
    pub fn new(i2c: MockI2c, address: u8) -> Self {
        Self { i2c, address }
    }

    /// 温度を読取る（ミリ度）
    pub fn read(&mut self) -> Result<i32> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(self.address, &[Self::TEMP_REGISTER], &mut buf)
            .map_err(|_| SystemError::Sensor(SensorError::BusError))?;

        let raw = ((buf[0] as u16) << 8) | (buf[1] as u16);
        let temp_raw = raw >> 5;
        let temp_signed = if (temp_raw & 0x0400) != 0 {
            (temp_raw as i16 | 0xF800) as i16
        } else {
            temp_raw as i16
        };

        Ok((temp_signed as i32) * 125)
    }

    /// デバイス存在確認
    pub fn probe(&mut self) -> bool {
        self.i2c.write(self.address, &[]).is_ok()
    }
}

/// SPI圧力センサードライバ（簡易実装）
pub struct PressureSensorDriver {
    spi: MockSpi,
    _cs_pin: u8,
}

impl PressureSensorDriver {
    /// 新しいドライバを作成
    pub fn new(spi: MockSpi, cs_pin: u8) -> Self {
        Self { spi, _cs_pin: cs_pin }
    }

    /// 圧力を読み取る（Pa単位、ミリPaで返す）
    pub fn read(&mut self) -> Result<i32> {
        let mut buf = [0u8; 4];
        self.spi.transfer(&[0x00], &mut buf)
            .map_err(|_| SystemError::Sensor(SensorError::BusError))?;

        // モック: 24bitの生データを仮定
        let raw = ((buf[1] as i32) << 16) | ((buf[2] as i32) << 8) | (buf[3] as i32);
        Ok(raw)
    }
}

/// ADCアナログセンサードライバ
pub struct AdcSensorDriver {
    adc: MockAdc,
    channel: u8,
}

impl AdcSensorDriver {
    /// 新しいドライバを作成
    pub fn new(adc: MockAdc, channel: u8) -> Self {
        Self { adc, channel }
    }

    /// ADC値を読み取る（12bit想定）
    pub fn read(&mut self) -> Result<u16> {
        self.adc.read(self.channel)
            .map_err(|_| SystemError::Sensor(SensorError::BusError))
    }
}

/// センサーマネージャー
///
/// 複数のセンサーを一元管理し、一括でデータを収集します。
pub struct SensorManager {
    configs: Vec<SensorConfig, MAX_SENSORS>,
    temp_driver: Option<Lm75aDriver>,
    pressure_driver: Option<PressureSensorDriver>,
    adc_driver: Option<AdcSensorDriver>,
}

impl SensorManager {
    /// 新しいセンサーマネージャーを作成
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
            temp_driver: None,
            pressure_driver: None,
            adc_driver: None,
        }
    }

    /// センサー設定を登録
    pub fn register_sensor(&mut self, config: SensorConfig) -> Result<()> {
        if self.configs.len() >= MAX_SENSORS {
            return Err(SystemError::BufferOverflow);
        }
        self.configs.push(config)
            .map_err(|_| SystemError::BufferOverflow)
    }

    /// I2C温度センサードライバを設定
    pub fn set_temp_driver(&mut self, driver: Lm75aDriver) {
        self.temp_driver = Some(driver);
    }

    /// SPI圧力センサードライバを設定
    pub fn set_pressure_driver(&mut self, driver: PressureSensorDriver) {
        self.pressure_driver = Some(driver);
    }

    /// ADCドライバを設定
    pub fn set_adc_driver(&mut self, driver: AdcSensorDriver) {
        self.adc_driver = Some(driver);
    }

    /// 全センサーからデータを収集
    pub fn collect_all(&mut self, timestamp_ms: u32) -> Result<Vec<SensorReading, MAX_SENSORS>> {
        let mut readings = Vec::new();

        for config in self.configs.iter() {
            if !config.enabled {
                continue;
            }

            let reading = self.read_single(config, timestamp_ms)?;
            readings.push(reading)
                .map_err(|_| SystemError::BufferOverflow)?;
        }

        Ok(readings)
    }

    /// 単一センサーを読み取り
    fn read_single(&mut self, config: &SensorConfig, timestamp_ms: u32) -> Result<SensorReading> {
        let (raw, scaled, quality) = match config.sensor_type {
            SensorType::TemperatureI2c => {
                match &mut self.temp_driver {
                    Some(driver) => {
                        let raw = driver.read()?;
                        let scaled = raw * config.scale_num / config.scale_den + config.offset;
                        (raw, scaled, DataQuality::Good)
                    }
                    None => return Err(SystemError::Sensor(SensorError::DeviceNotFound)),
                }
            }
            SensorType::PressureSpi => {
                match &mut self.pressure_driver {
                    Some(driver) => {
                        let raw = driver.read()?;
                        let scaled = raw * config.scale_num / config.scale_den + config.offset;
                        (raw, scaled, DataQuality::Good)
                    }
                    None => return Err(SystemError::Sensor(SensorError::DeviceNotFound)),
                }
            }
            SensorType::AnalogAdc => {
                match &mut self.adc_driver {
                    Some(driver) => {
                        let raw_u16 = driver.read()?;
                        let raw = raw_u16 as i32;
                        let scaled = raw * config.scale_num / config.scale_den + config.offset;
                        (raw, scaled, DataQuality::Good)
                    }
                    None => return Err(SystemError::Sensor(SensorError::DeviceNotFound)),
                }
            }
            SensorType::DigitalIo => {
                // デジタルI/Oは0/1のみ
                (0, 0, DataQuality::Good)
            }
        };

        Ok(SensorReading {
            sensor_id: config.id,
            raw_value: raw,
            scaled_value: scaled,
            quality,
            timestamp_ms,
        })
    }

    /// センサー数を取得
    pub fn sensor_count(&self) -> usize {
        self.configs.len()
    }
}
