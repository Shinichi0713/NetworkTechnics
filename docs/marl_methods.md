



はい、MAPPO（Multi-Agent PPO）の後継・改良手法は複数存在します。主な流れと代表的な手法を整理します。

---

### 1. HAPPO / HATRPO（2021年）
MAPPOの理論的な後継として、**HAPPO（Heterogeneous-Agent PPO）** と **HATRPO** が提案されました。<source-chip title="arXiv" url="https://arxiv.org/abs/2109.11251" />

- MAPPOではエージェントが同時にポリシーを更新するため、学習中の非定常性（non-stationarity）が問題になることがありました。
- HAPPOは**逐次更新（sequential update）**を導入し、1体ずつエージェントのポリシーを更新することで、理論的な単調な改善保証（monotonic improvement guarantee）をマルチエージェント設定に拡張しました。

---

### 2. CoPPO（Cooperative PPO）
MAPPOをベースに、**明示的な協調メカニズム**をクリッピングに組み込んだ簡潔な拡張です。<source-chip title="DeepWiki" url="https://deepwiki.com/xihuai18/A2PO-ICLR2023/2.2-coppo-(cooperative-ppo)" />

- 利得混合（advantage-mixing）やクレジット配分は使わず、クリッピング機構自体で協調を促進するアプローチです。

---

### 3. A2PO（Agent-by-Agent Policy Optimization, ICLR 2023）
HAPPOの重要な問題点である「**更新順序が性能に大きく影響する**」という課題を解決した手法です。<source-chip title="OpenReview" url="https://openreview.net/forum?id=Q-neeWNVv1" />

- HAPPOではエージェントの更新順序を固定・ランダムにするだけでしたが、A2POは**どの順序で更新すべきかを最適化**する枠組みを提案しました。
- 実装リポジトリでは、MAPPO・CoPPO・HAPPO・A2POが一貫した枠組みで比較実装されています。<source-chip title="GitHub" url="https://github.com/xihuai18/A2PO-ICLR2023" />

---

### 4. B2MAPO（2024年）
**Batch-by-Batch Multi-Agent Policy Optimization** として、同時更新と逐次更新の長所を統合しようとするアプローチです。<source-chip title="arXiv" url="https://arxiv.org/pdf/2407.15077" />

- 性能と学習効率のトレードオフを緩和するため、バッチ単位で更新戦略を調整します。

---

### 5. MARPO（2025年）
**Multi-Agent Reflective Policy Optimization** は、サンプル効率の低さを改善するための新しい枠組みです。<source-chip title="arXiv" url="https://arxiv.org/pdf/2512.22832" />

---

### 6. その他の拡張
- **非同期最適化MAPPO**（2024年）: MAPPOの非同期版を提案し、スケーラビリティを向上。<source-chip title="清華大学論文PDF" url="https://nicsefc.ee.tsinghua.edu.cn/nics_file/pdf/698728b2-2b03-4ec2-98be-d675dbb2779b.pdf" />
- **MAPPO-LCR**（2025年）: 空間的公共財ゲームなどに向けて、局所協調報酬（Local Cooperation Reward）を導入した拡張。<source-chip title="arXiv" url="https://arxiv.org/html/2512.17187" />

---

### まとめ
MAPPOそのものを完全に置き換える「単一の後継」というよりは、**HAPPO・A2PO・B2MAPO・MARPO** といった「MAPPOの問題点（非定常性・更新順序・サンプル効率）を理論的・実践的に改良する手法群」が次々と提案されています。

特に理論的な流れとしては **MAPPO → HAPPO → A2PO** が主要な系譜と言えます。



はい、複数存在します。先ほど挙げた手法のうち、**国際学会（または国際ジャーナル）に採択・掲載されているもの**は以下の通りです。

---

### 国際学会に採択されている手法

| 手法 | 採択会議・掲載誌 | 年 |
|------|------------------|-----|
| **HAPPO / HATRPO** | **ICLR 2022**（International Conference on Learning Representations） | 2022 |
| **CoPPO / A2PO** | **ICLR 2023** | 2023 |
| **非同期最適化MAPPO** | **CICAI 2021**（First CAAI International Conference on Artificial Intelligence） | 2021 |
| **B2MAPO** | **ECAI 2024**（European Conference on Artificial Intelligence） | 2024 |
| **MARPO** | **AAAI 2026**（Association for the Advancement of Artificial Intelligence） | 2026 |
| **MAPPO-LCR** | **Chaos, Solitons & Fractals**（国際ジャーナル、Elsevier） | 2026 |

---

### 補足

- **HAPPO / HATRPO** は ICLR 2022 に採択され、MAPPOの理論的な後継として「マルチエージェント設定での単調な改善保証」を初めて厳密に示した論文として知られています。<source-chip title="ICLR 2022 Virtual Poster" url="https://iclr.cc/virtual/2022/poster/6244" />

- **A2PO** は ICLR 2023 に採択され、HAPPOの弱点である「更新順序の影響」を解決した手法です。同じ論文内で **CoPPO** も提案されています。<source-chip title="OpenReview ICLR 2023" url="https://openreview.net/forum?id=Q-neeWNVv1" />

- **非同期最適化MAPPO** は CICAI 2021 に採択され、MAPPOの非同期版としてスケーラビリティを向上させています。<source-chip title="researchr publication" url="https://researchr.org/publication/FuYLW21" />

- **B2MAPO** は ECAI 2024 に採択され、同時更新と逐次更新の長所を統合するアプローチです。<source-chip title="researchr publication" url="https://researchr.org/publication/ZhangHW24-3" />

- **MARPO** は AAAI 2026 に採択され、サンプル効率を改善するReflective Policy Optimizationを提案しています。<source-chip title="AAAI Proceedings" url="https://ojs.aaai.org/index.php/AAAI/article/view/40219" />

- **MAPPO-LCR** は学会ではありませんが、Elsevierの国際ジャーナル **Chaos, Solitons & Fractals** に掲載されています。<source-chip title="exa.ai library" url="https://exa.ai/library/publication/cglt4pnt4zy" />

---

### まとめ

MAPPOの後継・改良手法の中で、**トップティアの国際学会（ICLR・AAAI・ECAI等）に採択されているものが複数存在**します。特に理論的な系譜としては **ICLR 2022のHAPPO → ICLR 2023のA2PO** が主要な流れとなっています。