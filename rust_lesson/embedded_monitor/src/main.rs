//! 組込みシステム リアルタイムモニタリング・シミュレータ
//! 
//! エンベデッドスペシャリスト試験の論文題材にもなるような、
//! センサーデータ収集・フィルタリング・異常検知・ログ管理・通信プロトコルを
//! 一つのRustアプリケーションで体験できます。
//!
//! 想定ハードウェア: 温度・振動・電圧センサーを持つ産業用モニタリング端末
//! OS: Linux (シミュレーション) / 組込みRTOSへの移植も容易な構造

use std::collections::VecDeque;
use std::thread;
use std::time::{Duration, Instant};

// ============================================================
// 定数定義（組込みらしくコンパイル時に決定）
// ============================================================
const SENSOR_COUNT: usize = 3;
const SAMPLE_INTERVAL_MS: u64 = 100;
const RING_BUFFER_SIZE: usize = 64;
const FILTER_WINDOW_SIZE: usize = 5;

// 異常閾値
const TEMP_HIGH_THRESHOLD: f32 = 80.0;
const VIBRATION_HIGH_THRESHOLD: f32 = 5.0;
const VOLTAGE_LOW_THRESHOLD: f32 = 3.0;
const VOLTAGE_HIGH_THRESHOLD: f32 = 5.5;

// ============================================================
// データ型定義
// ============================================================

/// センサー種別
#[derive(Debug, Clone, Copy, PartialEq)]
enum SensorType {
    Temperature,
    Vibration,
    Voltage,
}

impl SensorType {
    fn name(&self) -> &'static str {
        match self {
            SensorType::Temperature => "温度",
            SensorType::Vibration => "振動",
            SensorType::Voltage => "電圧",
        }
    }
}

/// 生センサーデータ
#[derive(Debug, Clone, Copy)]
struct SensorReading {
    sensor_type: SensorType,
    raw_value: f32,
    timestamp_ms: u64,
}

/// フィルタ済みデータ＋異常判定結果
#[derive(Debug, Clone, Copy)]
struct ProcessedData {
    sensor_type: SensorType,
    filtered_value: f32,
    is_abnormal: bool,
    timestamp_ms: u64,
}

/// 異常イベントログ
#[derive(Debug, Clone)]
struct AlarmLog {
    sensor_type: SensorType,
    value: f32,
    message: String,
    timestamp_ms: u64,
}

// ============================================================
// 移動平均フィルタ（ノイズ除去）
// ============================================================

struct MovingAverageFilter {
    window: VecDeque<f32>,
    size: usize,
}

impl MovingAverageFilter {
    fn new(size: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(size),
            size,
        }
    }

    fn push(&mut self, value: f32) -> f32 {
        if self.window.len() >= self.size {
            self.window.pop_front();
        }
        self.window.push_back(value);
        self.window.iter().sum::<f32>() / self.window.len() as f32
    }
}

// ============================================================
// リングバッファ（組込みでよく使う固定長キュー）
// ============================================================

struct RingBuffer<T> {
    buf: VecDeque<T>,
    capacity: usize,
}

impl<T: Clone> RingBuffer<T> {
    fn new(capacity: usize) -> Self {
        Self {
            buf: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn push(&mut self, item: T) {
        if self.buf.len() >= self.capacity {
            self.buf.pop_front();
        }
        self.buf.push_back(item);
    }

    fn iter(&self) -> std::collections::vec_deque::Iter<T> {
        self.buf.iter()
    }

    fn len(&self) -> usize {
        self.buf.len()
    }
}

// ============================================================
// 異常検知エンジン
// ============================================================

struct AnomalyDetector;

impl AnomalyDetector {
    fn check(reading: &SensorReading, filtered: f32) -> Option<AlarmLog> {
        let (is_abnormal, message) = match reading.sensor_type {
            SensorType::Temperature => {
                if filtered > TEMP_HIGH_THRESHOLD {
                    (true, format!("高温異常: {:.1}°C (閾値 {:.1}°C)", filtered, TEMP_HIGH_THRESHOLD))
                } else {
                    (false, String::new())
                }
            }
            SensorType::Vibration => {
                if filtered > VIBRATION_HIGH_THRESHOLD {
                    (true, format!("振動異常: {:.2}G (閾値 {:.2}G)", filtered, VIBRATION_HIGH_THRESHOLD))
                } else {
                    (false, String::new())
                }
            }
            SensorType::Voltage => {
                if filtered < VOLTAGE_LOW_THRESHOLD {
                    (true, format!("電圧低下: {:.2}V (下限 {:.2}V)", filtered, VOLTAGE_LOW_THRESHOLD))
                } else if filtered > VOLTAGE_HIGH_THRESHOLD {
                    (true, format!("電圧過大: {:.2}V (上限 {:.2}V)", filtered, VOLTAGE_HIGH_THRESHOLD))
                } else {
                    (false, String::new())
                }
            }
        };

        if is_abnormal {
            Some(AlarmLog {
                sensor_type: reading.sensor_type,
                value: filtered,
                message,
                timestamp_ms: reading.timestamp_ms,
            })
        } else {
            None
        }
    }
}

// ============================================================
// 簡易通信プロトコル（コマンド応答型）
// ============================================================

enum Command {
    GetLatest,
    GetAlarms,
    GetStats,
    Help,
    Unknown(String),
}

impl From<&str> for Command {
    fn from(s: &str) -> Self {
        match s.trim() {
            "latest" => Command::GetLatest,
            "alarms" => Command::GetAlarms,
            "stats" => Command::GetStats,
            "help" => Command::Help,
            other => Command::Unknown(other.to_string()),
        }
    }
}

struct ProtocolHandler;

impl ProtocolHandler {
    fn execute(cmd: Command, latest: &[ProcessedData], alarms: &RingBuffer<AlarmLog>) -> String {
        match cmd {
            Command::GetLatest => {
                let mut out = String::from("=== 最新センサーデータ ===\n");
                for d in latest {
                    let status = if d.is_abnormal { "【異常】" } else { "正常" };
                    out.push_str(&format!(
                        "[{}] {}: {:.2} {}\n",
                        d.timestamp_ms,
                        d.sensor_type.name(),
                        d.filtered_value,
                        status
                    ));
                }
                out
            }
            Command::GetAlarms => {
                let mut out = String::from("=== 異常ログ ===\n");
                if alarms.len() == 0 {
                    out.push_str("異常なし\n");
                } else {
                    for a in alarms.iter() {
                        out.push_str(&format!("[{}] {}\n", a.timestamp_ms, a.message));
                    }
                }
                out
            }
            Command::GetStats => {
                let mut out = String::from("=== システ統計 ===\n");
                out.push_str(&format!("センサー数: {}\n", SENSOR_COUNT));
                out.push_str(&format!("サンプリング間隔: {} ms\n", SAMPLE_INTERVAL_MS));
                out.push_str(&format!("フィルタ窓サイズ: {}\n", FILTER_WINDOW_SIZE));
                out.push_str(&format!("リングバッファサイズ: {}\n", RING_BUFFER_SIZE));
                out.push_str(&format!("異常ログ件数: {}\n", alarms.len()));
                out
            }
            Command::Help => {
                "=== コマンド一覧 ===\n\
                 latest : 最新センサーデータ表示\n\
                 alarms : 異常ログ表示\n\
                 stats  : システム統計表示\n\
                 help   : このヘルプ\n"
                    .to_string()
            }
            Command::Unknown(s) => format!("不明なコマンド: '{}'\nhelp で一覧を表示します。\n", s),
        }
    }
}

// ============================================================
// ハードウェア抽象化レイヤ（HAL）のシミュレーション
// ============================================================

struct SimulatedAdc;

impl SimulatedAdc {
    /// 疑似センサーデータを生成（実機ではADCレジスタ読み出し）
    fn read(sensor_type: SensorType, time_ms: u64) -> f32 {
        use std::f32::consts::PI;
        let t = (time_ms as f32) / 1000.0;
        let noise = ((time_ms % 97) as f32 - 48.5) / 20.0; // 擬似ノイズ
        match sensor_type {
            SensorType::Temperature => {
                // 徐々に上昇し、80°Cを超えるシナリオ
                let base = 50.0 + 15.0 * (t / 30.0).min(2.0);
                base + 3.0 * (2.0 * PI * t / 10.0).sin() + noise
            }
            SensorType::Vibration => {
                // 時々大きな振動が発生
                let spike = if time_ms % 7000 < 500 { 4.0 } else { 0.0 };
                1.5 + spike + noise
            }
            SensorType::Voltage => {
                // ゆるやかに低下し、3Vを下回るシナリオ
                let base = 4.8 - 0.3 * (t / 40.0).min(4.0);
                base + 0.1 * (2.0 * PI * t / 5.0).sin() + noise * 0.05
            }
        }
    }
}

// ============================================================
// メイン処理
// ============================================================

fn main() {
    println!("========================================");
    println!("  組込みリアルタイムモニタリング・シミュレータ");
    println!("  (エンベデッドスシャリスト対策向け)");
    println!("========================================\n");

    let start = Instant::now();
    let sensor_types = [
        SensorType::Temperature,
        SensorType::Vibration,
        SensorType::Voltage,
    ];

    // 各センサーに移動平均フィルタを割り当て
    let mut filters: Vec<MovingAverageFilter> = (0..SENSOR_COUNT)
        .map(|_| MovingAverageFilter::new(FILTER_WINDOW_SIZE))
        .collect();

    // 最新処理済みデータ
    let mut latest_data: Vec<ProcessedData> = Vec::with_capacity(SENSOR_COUNT);

    // 異常ログのリングバッファ
    let mut alarm_buffer: RingBuffer<AlarmLog> = RingBuffer::new(RING_BUFFER_SIZE);

    // サンプリングループ（組込みのメインループに相当）
    let simulation_duration = Duration::from_secs(60); // 60秒シミュレート
    let mut next_sample = Instant::now();

    println!("サンプリング開始（{}秒間シミュレート）...\n", simulation_duration.as_secs());

    while start.elapsed() < simulation_duration {
        let elapsed_ms = start.elapsed().as_millis() as u64;

        // センサーデータ取得 → フィルタリング → 異常検知
        latest_data.clear();
        for (i, &stype) in sensor_types.iter().enumerate() {
            let raw = SimulatedAdc::read(stype, elapsed_ms);
            let reading = SensorReading {
                sensor_type: stype,
                raw_value: raw,
                timestamp_ms: elapsed_ms,
            };

            let filtered = filters[i].push(raw);

            if let Some(alarm) = AnomalyDetector::check(&reading, filtered) {
                alarm_buffer.push(alarm);
            }

            let is_abnormal = match stype {
                SensorType::Temperature => filtered > TEMP_HIGH_THRESHOLD,
                SensorType::Vibration => filtered > VIBRATION_HIGH_THRESHOLD,
                SensorType::Voltage => {
                    filtered < VOLTAGE_LOW_THRESHOLD || filtered > VOLTAGE_HIGH_THRESHOLD
                }
            };

            latest_data.push(ProcessedData {
                sensor_type: stype,
                filtered_value: filtered,
                is_abnormal,
                timestamp_ms: elapsed_ms,
            });
        }

        // 一定間隔で画面に簡易表示（組込みならLCDやUART出力）
        if elapsed_ms % 1000 < SAMPLE_INTERVAL_MS {
            print!("\r[{:5}s] ", elapsed_ms / 1000);
            for d in &latest_data {
                let mark = if d.is_abnormal { "!" } else { " " };
                print!("{}{}:{:.2} ", mark, d.sensor_type.name(), d.filtered_value);
            }
        }

        // 通信コマンド処理のデモ（5秒ごとに自動実行）
        if elapsed_ms % 5000 < SAMPLE_INTERVAL_MS && elapsed_ms > 0 {
            println!("\n");
            let demo_cmds = ["latest", "alarms", "stats"];
            let cmd_idx = ((elapsed_ms / 5000) % 3) as usize;
            let cmd = Command::from(demo_cmds[cmd_idx]);
            let response = ProtocolHandler::execute(cmd, &latest_data, &alarm_buffer);
            println!("{}", response);
        }

        next_sample += Duration::from_millis(SAMPLE_INTERVAL_MS);
        if let Some(wait) = next_sample.checked_duration_since(Instant::now()) {
            thread::sleep(wait);
        }
    }

    println!("\n\n=== シミュレーション終了 ===");
    println!("異常ログ総数: {}", alarm_buffer.len());
    println!("\n最終異常ログ:");
    for (i, a) in alarm_buffer.iter().enumerate() {
        println!("  {}. [{}] {}", i + 1, a.timestamp_ms, a.message);
    }

    // 最後にhelpコマンドを表示
    println!("\n{}", ProtocolHandler::execute(Command::Help, &latest_data, &alarm_buffer));
}
