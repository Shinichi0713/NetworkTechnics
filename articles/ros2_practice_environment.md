---
title: "ROS2とロボット制御の実験が可能な環境"
emoji: "🧏‍♀️"
type: "tech" # tech: 技術記事 / idea: アイデア記事
topics: ["communication", "interface", "robot", "control"]
published: true
---
ロボット制御は機能を独立させた分散制御が現時点では標準的かつベストな制御法です。
そして分散制御のため[ROS2](https://yoshishinnze.hatenablog.com/entry/2026/11/10/043000)が業界標準となっています。

そんなROS2を仮想的に実験できる環境について調査しました。

## 環境選定
特に **ROS2 の `ros2_control` フレームワーク**がまさに「上位・下位分離」の階層型制御を体験できる環境です。

### 1. The Construct（ブラウザ完結型・最も推奨）

[The Construct](https://www.theconstruct.ai) は、ブラウザ上で本格的な ROS2 環境を使えるオンラインサービスです。特に以下のコースで階層型制御を体験できます。

> 「Using the ROS2 Control Framework, robotics developers can easily customize, scale, and extend an entire robot control system by changing its decoupled components and seamlessly switching back and forth between real robots and simulations.」[The Construct ROS2 Control Training](https://www.theconstruct.ai/ros2-control-training/)

**体験できること**:

| 層 | 学習内容 |
|----|---------|
| **上位** | ROS2 ノードで経路計画・目標値生成（`/cmd_vel` 等のトピック発行） |
| **下位** | `ros2_control` のコントローラーマネージャーがモーター指令をリアルタイム生成 |
| **通信** | ROS2 トピックで上位・下位が連携 |

**特徴**: シミュレーション（Gazebo）と実機を**切り替えながら**同じコードを動かせるため、「シミュで上位・下位の連携を確認 → 実機で動かす」が体験できます。

### 2. ROS2Lab（ブラウザで Gazebo シミュレーション）

[ROS2Lab](https://www.ros2lab.com/) は、ブラウザ上で Gazebo シミュレーションと RViz2 を使える環境です。

> 「Live in your browser (emulation): my_bot in a Gazebo maze, diff_drive_controller, `/cmd_vel` from Terminator, lidar and camera in RViz2.」[ROS2Lab](https://www.ros2lab.com/)

**体験できること**:
- ブラウザ上で diff_drive_controller（下位）に `/cmd_vel`（上位からの目標速度）を送信
- LiDAR・カメラのセンサー情報を RViz2 で可視化
- ナビゲーション（上位）と車輪制御（下位）の分離を体験

### 3. Gazebo + `ros2_control`（ローカル or Docker）

`ros2_control` は ROS2 の公式制御フレームワークで、**ハードウェアインターフェース（下位層）とコントローラー（上位層）を明確に分離**しています。

> 「This package provides a Gazebo-Sim system plugin which instantiates a `ros2_control` controller manager and connects it to a Gazebo model.」[gz_ros2_control GitHub](https://github.com/ros-controls/gz_ros2_control/)

**アーキテクチャの構造**:

```
[Navigation Node: 上位（経路計画・SLAM）]
              ↓ /cmd_vel (ROS2 Topic)
[diff_drive_controller: 中位（速度→車輪角速度変換）]
              ↓
[ros2_control Hardware Interface: 下位（Gazeboの車輪にトルク指令）]
              ↓
        [Gazebo Physics Engine]
```

**体験方法**:
```bash
# Docker で即座に起動
docker run -it --rm --name ros2-control-tutorial osrf/ros:jazzy-desktop
# または Ubuntu + ROS2 Jazzy にて
sudo apt install ros-jazzy-ros2-control ros-jazzy-gz-ros2-control
ros2 launch gz_ros2_control_demos diff_drive_example.launch.py
```

### 4. デジタルツイン型 Web Lab（大学の教育環境）

近年、大学のロボット工学教育でも階層型制御のデジタルツイン環境が整備されています。

> 「The physical robot and its digital twin share the upper-level layers of the controller stack, which execute exclusively within the simulator, whereas the lower-level layers run in both the real and virtual robots.」[Simulator-Based Digital Twin of a Robotics Laboratory](https://www.mdpi.com/2075-1702/14/3/273)

**特徴**:
- **上位層（L1）**: シミュレータ内で経路計画・判断処理を実行
- **下位層（L0）**: 実機とシミュレーションの両方でリアルタイム制御
- **L0L1Link**: 上位・下位間の通信モジュールが透過的に動作

### 5. まとめ：どの環境を選ぶか

| 環境 | 難易度 | 階層型制御の体験度 | 推奨対象 |
|------|--------|-------------------|---------|
| **The Construct** | 中 | 高（ros2_controlの実践的コースあり） | 本格的に学びたい人 |
| **ROS2Lab** | 低 | 中（ブラウザで即座に試せる） | まず雰囲気を掴みたい人 |
| **Gazebo + ros2_control（ローカル）** | 高 | 高（フルカスタマイズ可能） | 開発者・研究者 |
| **大学のデジタルツイン Lab** | 中〜高 | 高（実機と連携） | 教育機関所属者 |


## 一言でまとめると

The Construct や ROS2Lab などのブラウザ完結型環境で、ros2_control を使った『上位（ナビゲーション）と下位（車輪制御）の分離』を体験できます。Gazebo シミュレーション内のロボットに対して、ROS2 トピックで目標値を送る（上位）と、ros2_control がリアルタイムでモーターを制御（下位）する構造が、まさに階層型制御アーキテクチャそのものです。

