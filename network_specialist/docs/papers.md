以下、3つの分野からそれぞれ代表的な論文を選び、概要をまとめました。

---

## 1. 輻輳制御（Congestion Control）

### 論文①：「A Deep Reinforcement Learning Perspective on Internet Congestion Control」
- **著者**: Nathan Jay ら（University of Illinois 等）
- **発表**: ICML 2019
- **概要**: インターネットの輻輳制御を「RL問題」として定式化した先駆的研究です。送信側エージェントが、ネットワークの遅延やパケットロスを**状態**、送信レートの増減を**行動**、スループットと遅延のトレードオフを**報酬**として学習します。従来のTCP（Cubic, BBR等）と比較して、様々なネットワーク環境で**より高いスループットと低遅延**を達成しました。

### 論文②：「TCP-RL: Dynamic TCP Initial Windows and Congestion Control Schemes Through Reinforcement Learning」
- **著者**: Xiaohui Nie ら（清華大学）
- **発表**: IEEE JSAC 2019
- **概要**: TCPの**初期ウィンドウサイズ**と輻輳制御アルゴリズムを、フローごとにRLで動的に調整します。特に、Web検索やeコマースのような**短いフローが多い環境**で、従来の固定パラメータ手法より大幅な性能向上を示しました。

---

## 2. トラフィック工学・SDN（Traffic Engineering）

### 論文③：「CFR-RL: Traffic Engineering with Reinforcement Learning in SDN」
- **著者**: Junjie Zhang ら（北京理工大学 / Fortinet）
- **発表**: arXiv 2020（IEEE/ACM 系の会誌に掲載）
- **概要**: SDN環境で、複数のフローがリンク帯域を共有する際の**負荷分散**をRLで最適化します。状態として各リンクの利用率を観測し、行動として**フロー分割比率**を決定。従来の最適化ベース手法（如：CFR）と比較して、**計算時間を大幅に短縮**しながら同等以上の負荷分散性能を達成しました。

### 論文④：「A Deep-Reinforcement Learning Approach for Software-Defined Networking Routing Optimization」
- **著者**: Giorgio Stampa ら（UPC / CA Technologies）
- **発表**: 2017
- **概要**: SDNコントローラが、ネットワークトポロジとトラフィック需要を観測し、**動的ルーティング**を学習します。特にトラフィック需要が変動する環境で、静的な最短経路手法よりも**QoS（品質保証）を維持**できることを示しました。

---

## 3. エッジ・クラウド計算（Task Offloading）

### 論文⑤：「Decentralized Scheduling for Concurrent Tasks in Mobile Edge Computing via Deep Reinforcement Learning」
- **著者**: Ye Fan ら（南京大学 / Temple University）
- **発表**: IEEE TMC 2023
- **概要**: 複数のモバイル端末が、同時に発生するタスクを**ローカル実行するかエッジサーバーにオフロードするか**を、分散型の深層強化学習で決定します。各端末が独立に学習しながらも、全体として**遅延とエネルギー消費のトレードオフ**を最適化できます。

### 論文⑥：「TPTO: A Transformer-PPO based Task Offloading Solution for Edge Computing Environments」
- **著者**: Niloofar Gholipour ら（ÉTS, Canada）
- **発表**: 2023
- **概要**: エッジコンピューティング環境で、タスクの到着順序や依存関係を**Transformer**でエンコードし、**PPO（Proximal Policy Optimization）**でオフローディング決定を学習します。従来のヒューリスティクスや通常のDRLと比較して、**動的なワークロード変動に対する頑健性**が高いことが特徴です。

---

## まとめ表

| 分野 | 代表的論文 | 核心のアイデア |
|------|----------|--------------|
| **輻輳制御** | Jay et al. (ICML 2019) | 送信レート調整をRL問題として定式化 |
| **輻輳制御** | Nie et al. (JSAC 2019) | TCP初期ウィンドウをフローごとに動的最適化 |
| **SDN/TE** | Zhang et al. (2020) | フロー分割比率をRLで決定、負荷分散を最適化 |
| **SDN/TE** | Stampa et al. (2017) | 動的ルーティングをDRLで学習 |
| **エッジ計算** | Fan et al. (TMC 2023) | 分散型DRLでタスクオフローディングを最適化 |
| **エッジ計算** | Gholipour et al. (2023) | Transformer+PPOで動的ワークロードに対応 |

これらの研究の共通点は、**「従来の固定ルールやヒューリスティクスでは対応しきれない動的環境を、RLエージェントが経験から適応的に制御する」**という点です。


