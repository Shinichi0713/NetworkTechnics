import React, { useState, useEffect } from 'react';
// wasm-packで生成されたモジュールをインポート
import init, { benchmark_ip_calculation } from './pkg/wasm_ip_calc.js';

export default function App() {
  const [wasmReady, setWasmReady] = useState(false);
  const [resultTime, setResultTime] = useState(null);

  useEffect(() => {
    // Wasmモジュールの初期化（非同期）
    init().then(() => {
      setWasmReady(true);
    });
  }, []);

  const handleRunWasm = () => {
    if (!wasmReady) return;
    // Rust側の関数を直接呼び出し（例：1,000,000回ループ）
    const elapsed = benchmark_ip_calculation(1000000);
    setResultTime(elapsed);
  };

  return (
    <div style={{ padding: '20px', fontFamily: 'sans-serif' }}>
      <h2>React + Rust (WebAssembly) サンプル</h2>
      <p>ステータス: {wasmReady ? 'Wasm ロード完了 🚀' : 'ロード中...'}</p>
      
      <button 
        onClick={handleRunWasm} 
        disabled={!wasmReady}
        style={{ padding: '10px 20px', fontSize: '16px' }}
      >
        Rust製IP計算処理を実行
      </button>

      {resultTime !== null && (
        <p style={{ marginTop: '20px', fontWeight: 'bold' }}>
          処理時間: {resultTime} ms
        </p>
      )}
    </div>
  );
}