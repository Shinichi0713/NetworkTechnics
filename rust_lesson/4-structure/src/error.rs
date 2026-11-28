//! 統合エラー型
//!
//! システム全体で使用するエラー型を定義し、各モジュールのエラーを統合します。

use core::fmt;

/// システム全体のエラー型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemError {
    /// センサー関連エラー
    Sensor(SensorError),
    /// 通信関連エラー
    Communication(CommunicationError),
    /// フィルタ関連エラー
    Filter(FilterError),
    /// 設定関連エラー
    Config(ConfigError),
    /// バッファオーバーフロー
    BufferOverflow,
    /// 無効なパラメータ
    InvalidParameter,
    /// 未初期化
    NotInitialized,
    /// タイムアウト
    Timeout,
}

/// センサーエラー
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SensorError {
    /// I2C/SPIバスエラー
    BusError,
    /// デバイスが応答しない
    DeviceNotFound,
    /// データが異常値（範囲外）
    OutOfRange,
    /// CRCチェック失敗
    CrcError,
    /// センサー未キャリブレーション
    NotCalibrated,
    /// サポートされていないセンサー型
    UnsupportedType,
}

/// 通信エラー
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommunicationError {
    /// フレーミングエラー
    FramingError,
    /// パリティエラー
    ParityError,
    /// オーバーラン
    Overrun,
    /// 無効なコマンド
    InvalidCommand,
    /// コマンド引数エラー
    InvalidArgument,
    /// チェックサムエラー
    ChecksumError,
}

/// フィルタエラー
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterError {
    /// 未初期化
    NotInitialized,
    /// 係数が範囲外
    InvalidCoefficient,
}

/// 設定エラー
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigError {
    /// フラッシュ書き込み失敗
    FlashWriteError,
    /// 無効な設定値
    InvalidValue,
    /// 設定バージョン不一致
    VersionMismatch,
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemError::Sensor(e) => write!(f, "Sensor error: {:?}", e),
            SystemError::Communication(e) => write!(f, "Communication error: {:?}", e),
            SystemError::Filter(e) => write!(f, "Filter error: {:?}", e),
            SystemError::Config(e) => write!(f, "Config error: {:?}", e),
            SystemError::BufferOverflow => write!(f, "Buffer overflow"),
            SystemError::InvalidParameter => write!(f, "Invalid parameter"),
            SystemError::NotInitialized => write!(f, "Not initialized"),
            SystemError::Timeout => write!(f, "Timeout"),
        }
    }
}

/// `Result`型のエイリアス
pub type Result<T> = core::result::Result<T, SystemError>;
