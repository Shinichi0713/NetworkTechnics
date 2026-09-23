---
title: "機械学習モデルをONNXによる別環境での推論"
emoji: "🧖"
type: "tech" # tech: 技術記事 / idea: アイデア記事
topics: ["AI", "architecture"]
published: true
---
Pythonで構築・学習したモデルのネットワークパラメータを、C++で推論に利用するようなことがしたい場合、ネットワークパラメータをONNXというフォーマットで保管します。

本日はこのフォーマットONNXについて説明します。

## 概要

ONNX（Open Neural Network Exchange）は、**機械学習モデルを表現・保存するためのオープンなファイルフォーマット（標準規格）** です。

### 簡単に言えば

「どの機械学習フレームワークで作ったモデルも、共通の形式で保存して、別の環境で動かせるようにするための標準フォーマット」です。

### なぜ必要なのか

組み込みなどを行うと、機械学習の世界では、モデルの訓練に使うフレームワークと、実際に推論（予測）を実行する環境が異なることがほとんどです。

| フェーズ | よく使われるフレームワーク |
|----------|---------------------------|
| 訓練（学習） | PyTorch、TensorFlow、scikit-learn など |
| 推論（実行） | ONNX Runtime、TensorRT、OpenVINO など |

もし PyTorch で訓練したモデルをそのままスマートフォンや組み込み機器で動かしたい場合、ONNX に変換することで、専用の高速推論エンジンで実行できます。

### ONNX の主な特徴

1. **フレームワーク間の相互運用**
   PyTorch、TensorFlow、scikit-learn、MXNet など、多くのフレームワークから ONNX 形式への変換（エクスポート）が可能です。

2. **推論の高速化**
   ONNX Runtime などの専用エンジンを使うことで、元のフレームワークより高速に推論できる場合があります。

3. **ハードウェア最適化**
   NVIDIA GPU（TensorRT）、Intel CPU（OpenVINO）、ARM プロセッサ、スマートフォンなど、様々なハードウェア向けに最適化された実行環境が存在します。

4. **オープン標準**
   Microsoft と Facebook（Meta）が共同で提唱し、現在は LF AI & Data 財団が管理するオープンソースプロジェクトです。

### ファイル形式

- 拡張子：`.onnx`
- 内部形式：Protocol Buffers（Googleが開発したバイナリシリアライゼーション形式）を使用
- 中身：計算グラフ（ノード＝演算、エッジ＝テンソルデータの流れ）と重み（パラメータ）が含まれる

### 使われている場面

- **エッジデバイス**：スマートフォン、カメラ、IoT機器へのモデルデプロイ
- **クラウド推論**：Azure ML、AWS、Google Cloud などでの高速推論
- **組み込み機器**：自動車、医療機器、産業用カメラなど
- **ブラウザ**：ONNX モデルを WebGL や WebGPU で実行

## 推論エンジン

C++ で ONNX モデルを「アーキテクチャ定義なし」で使うには、**ONNX Runtime** という推論専用エンジンを使います。

ONNX ファイル（`.onnx`）の中には、すでに「計算グラフ（アーキテクチャ）」と「学習済み重み」が両方とも含まれています。そのため、C++ 側でレイヤー構造を書く必要は一切ありません。

### 必要なもの

- **ONNX Runtime**（Microsoft が提供するC++推論ライブラリ）
- ヘッダーとライブラリ（`.lib` / `.dll`）をダウンロードしてリンクするだけ

[ ONNX Runtime のリリースページ ](https://github.com/microsoft/onnxruntime/releases) から、C++ 用のパッケージをダウンロードできます。

### 基本的な流れ

C++ でのコードは以下のような流れになります。

1. `Ort::Env` で環境を初期化
2. `Ort::Session` で `.onnx` モデルを読み込む
3. 入力データ（画像や数値など）を `Ort::Value` に変換
4. `session.Run()` で推論実行
5. 出力 `Ort::Value` から結果を取得

### コード例（シンプルな推論）

```cpp
#include <onnxruntime_cxx_api.h>
#include <vector>
#include <iostream>

int main() {
    // 1. ONNX Runtime 環境の作成
    Ort::Env env(ORT_LOGGING_LEVEL_WARNING, "test");

    // 2. セッションオプションの設定
    Ort::SessionOptions session_options;

    // 3. モデルの読み込み（アーキテクチャ定義は不要）
    const wchar_t* model_path = L"model.onnx";  // 読み込むモデル
    Ort::Session session(env, model_path, session_options);

    // 4. 入力データの準備（例：1×3×224×224 の画像テンソル）
    std::vector<float> input_tensor_values(1 * 3 * 224 * 224, 1.0f);
    std::vector<int64_t> input_shape = {1, 3, 224, 224};

    Ort::AllocatorWithDefaultOptions allocator;
    const char* input_name = session.GetInputNameAllocated(0, allocator).get();
    const char* output_name = session.GetOutputNameAllocated(0, allocator).get();

    Ort::Value input_tensor = Ort::Value::CreateTensor<float>(
        allocator.GetInfo(), input_tensor_values.data(), input_tensor_values.size(), input_shape.data(), input_shape.size());

    // 5. 推論実行
    Ort::RunOptions run_options;
    session.Run(run_options, &input_name, &input_tensor, 1, &output_name, 1);

    // 6. 結果の取得
    Ort::Value output_tensor = ...;  // Run の戻り値から取得
    float* output_data = output_tensor.GetTensorMutableData<float>();

    std::cout << "推論結果: " << output_data[0] << std::endl;

    return 0;
}
```

### なぜアーキテクチャ定義が不要か

ONNX ファイルは自己完結型です。中身には以下がバイナリ形式で格納されています。

- **ノード（演算）**：Conv2D、ReLU、MatMul などのレイヤー構造
- **エッジ（テンソル）**：データの流れ
- **重み（パラメータ）**：学習済みの係数

ONNX Runtime はこのファイルを読み込むと、内部で自動的に計算グラフを構築し、最適化して実行します。開発者は「どのレイヤーをどう繋ぐか」を意識する必要はありません。

## 総括

ONNXの本質は、**「機械学習モデルの中間表現（Intermediate Representation）として、フレームワークと実行環境を分離するための自己完結型の標準規格」** です。

### 本質を3つの軸で捉える

1. **フレームワークからの独立**
   PyTorchやTensorFlowなど「どのツールで作られたか」を問わず、共通の形式でモデルを保存できる。これにより、訓練環境と推論環境を切り離せる。

2. **自己完結型のパッケージ**
   `.onnx` ファイル一つに、アーキテクチャ（計算グラフ）と学習済み重みの両方が含まれる。推論側はモデルの構造を知らなくても、ファイルを読み込むだけで実行可能。

3. **ハードウェア非依存の標準**
   CPU、GPU、組み込み機器、スマートフォンなど、実行環境に応じて最適化された推論エンジン（ONNX Runtime、TensorRT、OpenVINOなど）が選択される。モデル開発者はハードウェアを意識せず、実行側はフレームワークを意識しない。

### 一言で表せば

**「機械学習モデルのPDFのようなもの」** です。

PDFが「どのワープロで作った文書でも、どのOS・デバイスでも同じ見た目で表示できる」共通フォーマットであるように、ONNXは「どのフレームワークで作ったモデルでも、どの環境でも同じように推論できる」共通フォーマットです。

