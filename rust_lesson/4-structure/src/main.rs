//! 組み込みセンサーシステム メインアプリケーション
//!
//! 本アプリケーションは以下の機能を提供します:
//! - 複数センサー（I2C温度、SPI圧力、ADCアナログ）のデータ収集
//! - デジタルフィルタ（ローパス + 移動平均）によるノイズ除去
//! - シリアル通信によるコマンド受信・データ送信
//! - 不揮発性設定の管理
//! - 温度アラート判定
//!
//! ## コマンド一覧
//! - `GETALL` - 全センサーデータをJSON形式で送信
//! - `GET <id>` - 指定センサーのデータを送信
//! - `FILTER <alpha>` - フィルタ係数を設定（0〜1000）
//! - `INTERVAL <ms>` - サンプリング周期を設定（1〜10000ms）
//! - `STATUS` - システム状態を送信
//! - `VERSION` - ファームウェアバージョンを送信
//!
//! ## アーキテクチャ
//! ```text
//!  main.rs
//!    │
//!    ▼
//! ┌─────────────┐
//! │ SystemController │
//! └──────┬──────┘
//!        │
//!   ┌────┼────┬────────┐
//!   ▼    ▼    ▼        ▼
//! Sensor Comm  Config  Filter
//! Manager Mgr  Manager Chain
//!   │    │
//!   ▼    ▼
//!  HAL  HAL
//! (I2C/SPI/ADC/UART/Timer)
//! ```

#![no_std]
#![no_main]

use panic_halt as _;

// 実際のMCUでは以下のようにエントリポイントを設定
// use cortex_m_rt::entry;

// モジュールインポート
use embedded_sensor_system::comms::CommunicationManager;
use embedded_sensor_system::config::ConfigManager;
use embedded_sensor_system::filter::FilterChain;
use embedded_sensor_system::hal_mock::{MockAdc, MockI2c, MockSpi, MockTimer, MockUart};
use embedded_sensor_system::sensor::{
    AdcSensorDriver, Lm75aDriver, PressureSensorDriver, SensorManager,
};
use embedded_sensor_system::system::SystemController;

/// システム初期化関数
///
/// 実際のMCUでは `#[entry]` 属性を付けた `main` 関数として実装します。
/// ここでは論理的な初期化フローを示します。
fn system_init() -> SystemController {
    // ============================================
    // 1. ハードウェア初期化
    // ============================================
    // 実際の実装では各MCUのHALクレートを使用:
    //
    // let peripherals = stm32f4xx_hal::pac::Peripherals::take().unwrap();
    // let clocks = peripherals.RCC.constrain().cfgr.freeze();
    // let gpiob = peripherals.GPIOB.split();
    // let scl = gpiob.pb6.into_open_drain_output();
    // let sda = gpiob.pb7.into_open_drain_output();
    // let i2c = peripherals.I2C1.i2c((scl, sda), 100.kHz(), &clocks);
    // let uart = peripherals.USART1.serial(...);
    // let adc = peripherals.ADC1.adc(...);
    // let timer = peripherals.TIM2.delay_ms(&clocks);
    //
    // ここではモックを使用:

    let i2c = MockI2c::new();
    let spi = MockSpi::new();
    let uart = MockUart::new();
    let adc = MockAdc::new();
    let timer = MockTimer::new();

    // ============================================
    // 2. センサーマネージャー初期化
    // ============================================
    let mut sensor_manager = SensorManager::new();

    // I2C温度センサードライバ設定
    let temp_driver = Lm75aDriver::new(i2c, 0x48);
    sensor_manager.set_temp_driver(temp_driver);

    // SPI圧力センサードライバ設定
    let pressure_driver = PressureSensorDriver::new(spi, 0);
    sensor_manager.set_pressure_driver(pressure_driver);

    // ADCドライバ設定
    let adc_driver = AdcSensorDriver::new(adc, 0);
    sensor_manager.set_adc_driver(adc_driver);

    // ============================================
    // 3. 通信マネージャー初期化
    // ============================================
    let comm_manager = CommunicationManager::new(uart);

    // ============================================
    // 4. 設定マネージャー初期化
    // ============================================
    let config_manager = ConfigManager::new();

    // ============================================
    // 5. フィルタチェーン初期化
    // ============================================
    let filter_chain = FilterChain::new(200); // alpha = 0.2

    // ============================================
    // 6. システムコントローラー作成・初期化
    // ============================================
    let mut controller = SystemController::new(
        sensor_manager,
        comm_manager,
        config_manager,
        filter_chain,
        timer,
    );

    // システム初期化実行
    if let Err(e) = controller.initialize() {
        // 初期化エラー時の理
        // 実際にはLED点滅やエラーログ出力など
        panic!("System initialization failed");
    }

    controller
}

/// メインループ
///
/// 実際のMCUでは `#[entry]` 属性を付けた関数内で無限ループとして実行します。
fn main_loop() -> ! {
    let mut controller = system_init();

    // ============================================
    // メインループ
    // ============================================
    loop {
        // 1ティック（1イテレーション）を実行
        // - コマンド受信処理
        // - サンプリングタイミング判定とデータ収集
        // - フィルタ処理
        // - アラート判定
        if let Err(e) = controller.tick() {
            // エラー処理
            controller.state_mut().mode =
                embedded_sensor_system::system::SystemMode::Error;
            controller.state_mut().last_error = Some(e as u32);
        }

        // 実際のMCUではWFI（Wait For Interrupt）で省電力
        // cortex_m::asm::wfi();
    }
}

// 実際のMCU用エントリポイント（コメントアウト）
// #[entry]
// fn main() -> ! {
//     main_loop()
// }

// ============================================
// テスト用の簡易実行（モック環境）
// ============================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_init() {
        let controller = system_init();
        assert_eq!(controller.sensor_manager.sensor_count(), 3);
    }

    #[test]
    fn test_tick() {
        let mut controller = system_init();
        controller.initialize().unwrap();
        controller.tick().unwrap();
        assert!(controller.state().data_ready);
    }
}
