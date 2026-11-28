//! システム管理モジュール
//!
//! システム全体の状態管理、ステートマシン、スケジューリングを担当します。
//! メインループから呼び出される各種処理を統合します。

use crate::comms::{Command, CommunicationManager};
use crate::config::ConfigManager;
use crate::error::Result;
use crate::filter::FilterChain;
use crate::hal_mock::MockTimer;
use crate::sensor::{SensorManager, SensorReading};
use heapless::Vec;
use crate::MAX_SENSORS;

/// システム動作モード
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemMode {
    /// 初期化中
    Initializing,
    /// 通常運転
    Running,
    /// 設定モード
    Configuration,
    /// 低消費電力モード
    LowPower,
    /// エラー状態
    Error,
    /// ファクトリリセット待ち
    FactoryReset,
}

/// アラート状態
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlertState {
    /// 正常
    Normal,
    /// 警告（閾値接近）
    Warning,
    /// アラート（閾値超過）
    Alert,
}

/// システム状態
#[derive(Debug, Clone)]
pub struct SystemState {
    /// 現在のモード
    pub mode: SystemMode,
    /// アラート状態
    pub alert: AlertState,
    /// サンプリングカウント
    pub sample_count: u32,
    /// 最後のエラーコード
    pub last_error: Option<u32>,
    /// センサーデータ有効フラグ
    pub data_ready: bool,
    /// 通信受信待機中
    pub comm_pending: bool,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            mode: SystemMode::Initializing,
            alert: AlertState::Normal,
            sample_count: 0,
            last_error: None,
            data_ready: false,
            comm_pending: false,
        }
    }
}

/// システム全体を管理するコントローラー
pub struct SystemController {
    /// センサーマネージャー
    pub sensor_manager: SensorManager,
    /// 通信マネージャー
    pub comm_manager: CommunicationManager,
    /// 設定マネージャー
    pub config_manager: ConfigManager,
    /// フィルタチェーン
    pub filter_chain: FilterChain,
    /// タイマー
    pub timer: MockTimer,
    /// システム状態
    pub state: SystemState,
    /// フィルタ済みデータ
    filtered_readings: Vec<SensorReading, MAX_SENSORS>,
    /// 次のサンプリング時刻
    next_sample_time: u32,
}

impl SystemController {
    /// 新しいシステムコントローラーを作成
    pub fn new(
        sensor_manager: SensorManager,
        comm_manager: CommunicationManager,
        config_manager: ConfigManager,
        filter_chain: FilterChain,
        timer: MockTimer,
    ) -> Self {
        Self {
            sensor_manager,
            comm_manager,
            config_manager,
            filter_chain,
            timer,
            state: SystemState::default(),
            filtered_readings: Vec::new(),
            next_sample_time: 0,
        }
    }

    /// システム初期化
    pub fn initialize(&mut self) -> Result<()> {
        // 設定を読み込み
        self.config_manager.load()?;

        // センサー設定を反映
        let config = self.config_manager.config().clone();
        for sensor_cfg in config.sensors.iter() {
            self.sensor_manager.register_sensor(*sensor_cfg)?;
        }

        // フィルタ係数を反映
        self.filter_chain = FilterChain::new(config.filter_alpha);

        self.state.mode = SystemMode::Running;
        self.next_sample_time = self.timer.now_ms();

        Ok(())
    }

    /// メインループの1イテレーション
    pub fn tick(&mut self) -> Result<()> {
        // 1. コマンド処理
        self.process_commands()?;

        // 2. サンプリングタイミング判定
        let now = self.timer.now_ms();
        let interval = self.config_manager.config().sampling_interval_ms;

        if now >= self.next_sample_time {
            self.sample_and_process()?;
            self.next_sample_time = now + interval;
        }

        // 3. アラート判定
        self.check_alerts();

        Ok(())
    }

    /// コマンドを処理
    fn process_commands(&mut self) -> Result<()> {
        if let Some(cmd) = self.comm_manager.poll_command()? {
            match cmd {
                Command::GetAll => {
                    self.comm_manager.send_sensor_data(&self.filtered_readings)?;
                }
                Command::GetSensor { id } => {
                    let filtered: Vec<SensorReading, MAX_SENSORS> = self
                        .filtered_readings
                        .iter()
                        .filter(|r| r.sensor_id == id)
                        .copied()
                        .collect();
                    self.comm_manager.send_sensor_data(&filtered)?;
                }
                Command::SetFilter { alpha } => {
                    if (0..=1000).contains(&alpha) {
                        self.config_manager.config_mut().filter_alpha = alpha;
                        self.filter_chain = FilterChain::new(alpha);
                        self.comm_manager.send_ack()?;
                    } else {
                        self.comm_manager.send_error("alpha must be 0..=1000")?;
                    }
                }
                Command::SetInterval { ms } => {
                    if ms > 0 && ms <= 10_000 {
                        self.config_manager.config_mut().sampling_interval_ms = ms;
                        self.comm_manager.send_ack()?;
                    } else {
                        self.comm_manager.send_error("interval must be 1..=10000")?;
                    }
                }
                Command::GetStatus => {
                    let status = format_system_state(&self.state);
                    self.comm_manager.send_status(&status)?;
                }
                Command::GetVersion => {
                    self.comm_manager.send_version(crate::VERSION)?;
                }
                Command::Unknown => {
                    self.comm_manager.send_error("Unknown command")?;
                }
            }
        }
        Ok(())
    }

    /// センサーからデータを収集しフィルタ処理
    fn sample_and_process(&mut self) -> Result<()> {
        let now = self.timer.now_ms();
        let readings = self.sensor_manager.collect_all(now)?;

        self.filtered_readings.clear();
        for reading in readings.iter() {
            let filtered_value = self.filter_chain.process(reading.scaled_value);
            let mut filtered = *reading;
            filtered.scaled_value = filtered_value;
            let _ = self.filtered_readings.push(filtered);
        }

        self.state.sample_count += 1;
        self.state.data_ready = true;

        Ok(())
    }

    /// アラート判定
    fn check_alerts(&mut self) {
        let threshold = self.config_manager.config().temp_alert_threshold;
        let mut max_alert = AlertState::Normal;

        for reading in self.filtered_readings.iter() {
            if reading.scaled_value > threshold {
                max_alert = AlertState::Alert;
                break;
            } else if reading.scaled_value > threshold * 9 / 10 {
                // 閾値の90%以上
                if max_alert == AlertState::Normal {
                    max_alert = AlertState::Warning;
                }
            }
        }

        self.state.alert = max_alert;
    }

    /// システム状態を取得
    pub fn state(&self) -> &SystemState {
        &self.state
    }

    /// フィルタ済みデータを取得
    pub fn filtered_data(&self) -> &Vec<SensorReading, MAX_SENSORS> {
        &self.filtered_readings
    }
}

/// システム状態を文字列にフォーマット
fn format_system_state(state: &SystemState) -> heapless::String<128> {
    let mut s = heapless::String::<128>::new();
    let mode_str = match state.mode {
        SystemMode::Initializing => "INIT",
        SystemMode::Running => "RUN",
        SystemMode::Configuration => "CONFIG",
        SystemMode::LowPower => "LOWPWR",
        SystemMode::Error => "ERROR",
        SystemMode::FactoryReset => "FRESET",
    };
    let alert_str = match state.alert {
        AlertState::Normal => "OK",
        AlertState::Warning => "WARN",
        AlertState::Alert => "ALERT",
    };
    let _ = s.push_str("mode=");
    let _ = s.push_str(mode_str);
    let _ = s.push_str(",alert=");
    let _ = s.push_str(alert_str);
    let _ = s.push_str(",samples=");
    let _ = s.push_str(&itoa_u32(state.sample_count));
    s
}

/// u32を文字列に変換no_std用）
fn itoa_u32(n: u32) -> heapless::String<12> {
    let mut result = heapless::String::<12>::new();
    let mut n = n;
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
