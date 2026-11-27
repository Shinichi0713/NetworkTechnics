// embedded-sensor-example.rs
// Rustによる組み込みソフトウェアでのセンサー読み取り実装例
// no_std環境を想定（Cortex-M等のマイクロコントローラ向け）

#![no_std]
#![no_main]

use core::fmt::Write;
use panic_halt as _;
use cortex_m_rt::entry;
use embedded_hal::i2c::I2c;

// ============================================
// センサードライバ: I2C接続温度センサー（LM75A互換を想定）
// ============================================

/// LM75A 温度センサーのI2Cデバイスアドレス
/// A0,A1,A2ピンの接続状態によって変化（ここでは0x48を想定）
const LM75A_ADDR: u8 = 0x48;

/// 温度レジスタのアドレス
const TEMP_REGISTER: u8 = 0x00;

/// センサー読み取り結果を格納する構造体
#[derive(Debug, Clone, Copy)]
pub struct SensorData {
    /// 温度（ミリ度単位: 1/1000 ℃）
    pub temperature_milli_celsius: i32,
    /// 生のレジスタ値（デバッグ用）
    pub raw_value: u16,
}

/// センサーエラーの種類
#[derive(Debug, Clone, Copy)]
pub enum SensorError {
    /// I2C通信エラー
    BusError,
    /// データ変換エラー
    InvalidData,
}

/// LM75A 温度センサーライブラリ
pub struct Lm75a<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C, E> Lm75a<I2C>
where
    I2C: I2c<Error = E>,
{
    /// 新しいLM75Aインスタンスを作成
    pub fn new(i2c: I2C, address: u8) -> Self {
        Self { i2c, address }
    }

    /// I2Cバスへの参照を取得（他のデバイスとバスを共有する場合に使用）
    pub fn bus(&mut self) -> &mut I2C {
        &mut self.i2c
    }

    /// 温度を読み取る（ミリ度で返す）
    ///
    /// LM75Aの温度レジスタは16bitで、上位11bitが温度データ。
    /// - 上位8bit: 整数部（符号付き）
    /// - 下位8bitの上位3bit: 小数部（0.125℃刻み）
    pub fn read_temperature(&mut self) -> Result<SensorData, SensorError> {
        let mut buf = [0u8; 2];

        // 温度レジスタを指定して2バイト読み出し
        self.i2c
            .write_read(self.address, &[TEMP_REGISTER], &mut buf)
            .map_err(|_| SensorError::BusError)?;

        let raw = ((buf[0] as u16) << 8) | (buf[1] as u16);

        // 11bitの符号付き温度データを解析
        let temp_raw = raw >> 5; // 上位11bitを抽出
        let temp_signed = if (temp_raw & 0x0400) != 0 {
            // 負の値（2の補数変換）
            (temp_raw as i16 | 0xF800) as i16
        } else {
            temp_raw as i16
        };

        // 0.125℃刻みなので、ミリ度に変換（×125）
        let milli_celsius = (temp_signed as i32) * 125;

        Ok(SensorData {
            temperature_milli_celsius: milli_celsius,
            raw_value: raw,
        })
    }

    /// センサーが応答するかを確認（簡易的な存在確認）
    pub fn is_connected(&mut self) -> bool {
        self.i2c.write(self.address, &[]).is_ok()
    }
}

// ============================================
// メインアプリケーション
// ============================================

/// 簡易的なログ出力用（UART等への接続を想定）
struct Logger;

impl Write for Logger {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // 実際のハードウェアではUARTトレイトを使って送信する
        // ここではプレースホルダー
        Ok(())
    }
}

/// センサーデータに対するフィルタ処理（簡易ローパスフィルタ）
fn apply_lowpass_filter(prev: i32, current: i32, alpha: i32) -> i32 {
    // alpha: 0〜1000 の範囲（1000=新しい値のみ使用）
    (prev * (1000 - alpha) + current * alpha) / 1000
}

/// 温度が閾値を超えたかをチェック
fn check_temperature_alert(temp_milli: i32, threshold_milli: i32) -> bool {
    temp_milli > threshold_milli
}

#[entry]
fn main() -> ! {
    // ============================================
    // ハードウェア初期化（各MCUのHALクレートを使用）
    // 以下は疑似コード的な初期化例
    // ============================================
    // let peripherals = stm32f4xx_hal::pac::Peripherals::take().unwrap();
    // let clocks = peripherals.RCC.constrain().cfgr.freeze();
    // let gpiob = peripherals.GPIOB.split();
    // let scl = gpiob.pb6.into_open_drain_output();
    // let sda = gpiob.pb7.into_open_drain_output();
    // let i2c = peripherals.I2C1.i2c((scl, sda), 100.kHz(), &clocks);
    // ============================================

    // 上記のi2cインスタンスを使ってセンサーを初期化
    // let mut sensor = Lm75a::new(i2c, LM75A_ADDR);

    // 以下は論理的な処理フローの例（実際のI2Cインスタンスが必要）
    // 実機では上記の初期化済みi2cを渡して使用する

    let mut logger = Logger;
    let mut _filtered_temp: i32 = 0;
    let alert_threshold: i32 = 35_000; // 35.000℃
    let filter_alpha: i32 = 200;       // フィタ係数（20%）

    loop {
        // 実際の読み取り処理（実機では以下のように使用）:
        // match sensor.read_temperature() {
        //     Ok(data) => {
        //         // フィルタ適用
        //         _filtered_temp = apply_lowpass_filter(
        //             _filtered_temp,
        //             data.temperature_milli_celsius,
        //             filter_alpha,
        //         );
        //
        //         // アラート判定
        //         if check_temperature_alert(_filtered_temp, alert_threshold) {
        //             let _ = writeln!(&mut logger,
        //                 "ALERT: Temperature {:.3} C exceeds threshold",
        //                 _filtered_temp as f32 / 1000.0
        //             );
        //         }
        //
        //         // 通常ログ出力
        //         let _ = writeln!(&mut logger,
        //             "Temp: {:.3} C (raw: 0x{:04X})",
        //             data.temperature_milli_celsius as f32 / 1000.0,
        //             data.raw_value
        //         );
        //     }
        //     Err(SensorError::BusError) => {
        //         let _ = writeln!(&mut logger, "Error: I2C bus error");
        //     }
        //     Err(SensorError::InvalidData) => {
        //         let _ = writeln!(&mut logger, "Error: Invalid sensor data");
        //     }
        // }

        // サンプリング周期（例: 100ms）
        // delay.delay_ms(100u16);
    }
}
