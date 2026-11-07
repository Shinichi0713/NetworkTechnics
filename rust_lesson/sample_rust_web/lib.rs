use wasm_bindgen::prelude::*;

// JavaScriptから呼び出せるIPアドレス計算ベンチマーク用の関数
#[wasm_bindgen]
pub fn benchmark_ip_calculation(iterations: u32) -> f64 {
    let start = js_sys::Date::now();
    let mut _dummy: u32 = 0;
    
    // 大量のIPアドレス計算・ビット演算をシミュレート
    for i in 0..iterations {
        let ip = 0xC0A80101_u32.wrapping_add(i);
        _dummy = _dummy.wrapping_add(ip ^ (ip >> 2) ^ (ip << 1));
    }
    
    let end = js_sys::Date::now();
    end - start // 実行時間（ミリ秒）を返す
}