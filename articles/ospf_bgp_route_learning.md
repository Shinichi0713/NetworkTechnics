---
title: "ルーターは自組織の経路をどう覚えるか - OSPFとBGPからルートテーブルができるまで"
emoji: "🧭"
type: "tech" # tech: 技術記事 / idea: アイデア記事
topics: ["networking", "routing", "ospf", "bgp"]
published: true
---
ルーターはパケットを転送するとき、`show ip route` で見られる**ルートテーブル（RIB）** だけを眺めて「次にどこへ出せばよいか」を決めています。

では、そのルートテーブルは一体どうやって埋まっていくのでしょうか。

典型的な組織網では、

- **組織の中（AS 内）は OSPF**
- **組織の外（AS 間）は BGP**

という役割分担がされています。この記事では、インターフェースが Up した瞬間からルートテーブルにエントリが並ぶまでを、具体的なアドレスと出力例で追っていきます。

## この記事でわかること

1. 直結ルート・OSPF ルート・BGP ルートがそれぞれどうやって生まれるか
2. OSPF の LSDB と SPF 計算、BGP の BEST パス選択の違い
3. 複数の経路情報がぶつかったとき、何を基準に 1 つに絞られるか
4. 実際の `show ip route` / `show ip bgp` の見方と、よくある落とし穴

---

## 前提とするトポロジー

今回は、自組織を **AS 65001**、上流 ISP を **AS 65010（メイン）** と **AS 65011（バックアップ）** とした構成を考えます。

```
                      ISP-A (AS 65010)        ISP-B (AS 65011)
                       192.0.2.1/30            198.51.100.1/30
                            │                        │
                   192.0.2.2/30            198.51.100.2/30
                            │                        │
                   ┌────────┴────────┐               │
                   │   EDGE-R1 (ASBR)│               │
                   │  Lo0:10.255.0.1 │               │
                   └────────┬────────┘               │
                            │ 172.16.0.0/30          │
                            │ (Area 0)               │
                   ┌────────┴────────┐               │
                   │  CORE-R2 (ABR / │               │
                   │  Route Reflector)               │
                   │  Lo0:10.255.0.2 │               │
                   └───┬─────────┬───┘               │
        172.16.1.0/30  │         │  172.16.2.0/30   │
             (Area 10) │         │  (Area 20)       │
              ┌────────┴──┐   ┌──┴────────┐         │
              │ BR-A-R3   │   │ BR-B-R4   ├─────────┘
              │Lo0:.3     │   │Lo0:.4     │ (バックアップ回線)
              └─────┬─────┘   └─────┬─────┘
                    │               │
              10.20.1.0/24    10.30.1.0/24
```

| 項目                           | 内容                                      |
| ------------------------------ | ----------------------------------------- |
| 自組織の AS 番号               | 65001                                     |
| 本社 LAN（R1 直結）            | 10.10.0.0/16                              |
| 拠点 A LAN                     | 10.20.1.0/24（Area 10）                   |
| 拠点 B LAN                     | 10.30.1.0/24（Area 20）                   |
| ルーター ID                    | 10.255.0.1 〜 10.255.0.4（Loopback0 /32） |
| 自組織が持つグローバルアドレス | 203.0.113.0/24                            |
| ISP-A から受け取る経路         | 0.0.0.0/0、198.18.0.0/16                  |
| ISP-B から受け取る経路         | 0.0.0.0/0                                 |

> この記事の IP アドレスはすべて RFC 5737 のドキュメント用アドレス（192.0.2.0/24、198.51.100.0/24、203.0.113.0/24）と検証用アドレスです。

---

## 全体像：ルートがテーブルに至るまで

まず全体の流れを押さえておきます。

```
① インターフェースに IP を設定して Up
        ↓ 自動で生成
   Connected / Local ルート
        ↓
② OSPF: Hello → ネイバー → LSDB 同期 → SPF 計算
        ↓ "O" / "O IA" ルート
③ BGP : TCP 179 → OPEN → UPDATE → ベストパス選択
        ↓ "B" ルート
④ プロトコルごとの経路表から、1 つのメイン RIB に絞る
        ↓   AD（Administrative Distance）とメトリックで比較
⑤ メイン RIB → FIB（CEF / ASIC のハードウェアテーブル）へ書き込み
        ↓
⑥ パケット転送時は「最長前方一致（LPM）」でエントリを 1 つ選ぶ
```

ここで大事なのは、**「経路を選ぶ」局面が 3 種類ある**ということです。

| 局面                                               | 比較するもの        | 例                              |
| -------------------------------------------------- | ------------------- | ------------------------------- |
| プロトコル内部で最適パスを選ぶ                     | メトリック          | OSPF cost、BGP のベストパス属性 |
| 同じ prefix に複数のプロトコルがいたらどれを採るか | AD（管理距離）      | OSPF 110 vs eBGP 20             |
| 実際のパケットをどのエントリで送るか               | 最長前方一致（LPM） | /24 が /16 と /0 に優先する     |

この 3 つを混同すると、「ルートテーブルにはあるのに通らない」「優先度を変えたのに変わらない」という事態に陥ります。順に見ていきましょう。

---

## ステップ 0：まずは直結ルートが勝手にできる

ルーターは、IP アドレスとマスクを設定してインターフェースを Up した時点で、**そのネットワークへの経路を自動的に RIB へ入れます**。

EDGE-R1 の Gi0/0 に `172.16.0.1/30` を設定したとします。

```
C     172.16.0.0/30 is directly connected, GigabitEthernet0/0
L     172.16.0.1/32 is directly connected, GigabitEthernet0/0
```

- `C`（Connected）：このインターフェースから直接届く network
- `L`（Local）：そのインターフェース自身のアドレス（/32）

ここには設定も何も必要ありません。**ルーターが最初に持つ経路は、自分に直接つながっているネットワークだけ**です。

拠点 A の 10.20.1.0/24 がここに無いのは当然で、これは OSPF か BGP、あるいはスタティックで教えてもらう必要があります。

---

## ステップ 1：OSPF で隣のルーターと仲良くなる（ネイバー確立）

OSPF は **「自分のまわりのリンク状況を近所と交換し、地図を全員で作る」** タイプのプロトコルです（Link State 型）。

### 1-1. Hello パケットで隣人を探す

OSPF を有効にしたインターフェースから、マルチキャスト `224.0.0.5`（AllSPFRouters）へ Hello パケットが送られます。

```text
EDGE-R1(config)# router ospf 1
EDGE-R1(config-router)# router-id 10.255.0.1
EDGE-R1(config-router)# network 10.255.0.1 0.0.0.0 area 0
EDGE-R1(config-router)# network 10.10.0.0 0.0.255.255 area 0
EDGE-R1(config-router)# network 172.16.0.0 0.0.0.3 area 0
EDGE-R1(config-router)# passive-interface GigabitEthernet0/1
```

Hello には Router ID・Area ID・Hello/Dead タイマー・ネットワークマスクなどが入っていて、これらが一致しないとネイバーになりません。

ネイバーは以下の状態遷移をたどります。

```
Down → Init → 2-Way → ExStart → Exchange → Loading → Full
```

| 状態     | 何が起きているか                                                       |
| -------- | ---------------------------------------------------------------------- |
| Down     | まだ Hello を受信していない                                            |
| Init     | Hello を受信したが、自分の Router ID が相手に見えていない              |
| 2-Way    | 相互に Hello を確認。ブロードキャスト網ではここで**DR/BDR 選出** |
| ExStart  | Database Description（DD）の Master/Slave を決める                     |
| Exchange | DD パケットで「自分が持っている LSA のリスト」を交換                   |
| Loading  | 足りない LSA を LSR で要求し、LSU で受け取り、LSAck で応答             |
| Full     | LSDB（リンクステートデータベース）が完全に同期された                   |

```text
EDGE-R1# show ip ospf neighbor

Neighbor ID     Pri   State           Dead Time   Address         Interface
10.255.0.2        1   FULL/DR         00:00:37    172.16.0.2      GigabitEthernet0/0
```

> ブロードキャスト網（イーサネット）では、全員と Full になるわけではありません。**DR / BDR とは Full になり、DRother 同士は 2-Way で止まる**のが正常です。LSA の交換を DR に集約することで、無駄な通信量を抑えています。

### 1-2. LSDB に LSA が集まる

ネイバーが確立すると、LSA（Link State Advertisement）と呼ばれる「地図の部品」が Area 内で flooding され、各ルーターは同じ LSDB を持ちます。

代表的な LSA の種類は次のとおりです。

| Type | 名前            | 誰が出すか      | 内容                                                |
| ---- | --------------- | --------------- | --------------------------------------------------- |
| 1    | Router LSA      | 全ルーター      | 自分につながるリンクと cost（Area 内のみ）          |
| 2    | Network LSA     | DR              | その Ethernet に誰がつながっているか（Area 内のみ） |
| 3    | Summary LSA     | ABR             | 別 Area へのネットワーク（10.20.1.0/24 など）       |
| 4    | ASBR Summary    | ABR             | ASBR への到達情報                                   |
| 5    | AS External LSA | ASBR            | AS 外の経路（再配布された BGP 経路など）            |
| 7    | NSSA External   | ASBR（NSSA 内） | NSSA 用の外部経路。ABR が Type 5 に変換             |

```text
EDGE-R1# show ip ospf database

            OSPF Router with ID (10.255.0.1) (Process ID 1)

                Router Link States (Area 0)
Link ID         ADV Router      Age  Seq#        Checksum  Link count
10.255.0.1      10.255.0.1      812  0x8000000A  0x00A1B2   3
10.255.0.2      10.255.0.2      809  0x8000000C  0x003C4D   4

                Net Link States (Area 0)
Link ID         ADV Router      Age  Seq#        Checksum
172.16.0.2      10.255.0.2      809  0x80000003  0x007E10

                Summary Net Link States (Area 0)
Link ID         ADV Router      Age  Seq#        Checksum
10.20.1.0       10.255.0.2      700  0x80000005  0x0022AB
10.30.1.0       10.255.0.2      690  0x80000004  0x00189F
10.255.0.3      10.255.0.2      700  0x80000006  0x00BB22
10.255.0.4      10.255.0.2      690  0x80000005  0x00AA11
172.16.1.0      10.255.0.2      700  0x80000002  0x001122
172.16.2.0      10.255.0.2      690  0x80000002  0x003344
```

※ 数値はイメージです。Sequence Number や Checksum は LSA の新しさ・整合性の確認に使われます。

R1 は Area 0 にいるので、拠点 A/B の詳細な LSA（Type 1/2）は直接見えません。代わりに **ABR（CORE-R2）が Type 3 Summary LSA で「10.20.1.0/24 は自分（R2）の向こうにあるよ」と教えてくれている**ことがわかります。

### 1-3. SPF 計算で「自分の視点の最短経路木」を作る

LSDB が揃うと、各ルーターは **Dijkstra 法（SPF 算法）** で自分のルーター ID を根とする最短経路木を作ります。

cost の定義は次のとおりです。

```text
cost = 参考帯域幅 (reference bandwidth) / インターフェース帯域幅
```

デフォルトの参考帯域は 100 Mbps なので、修正しないと **1 Gbps も 10 Gbps も cost = 1** になってしまいます。実運用ではまずこれを上げます。

```text
EDGE-R1(config-router)# auto-cost reference-bandwidth 1000
```

R1 から拠点 A の 10.20.1.0/24 までの cost は、

```
R1 → R2 (1) + R2 → R3 (1) + R3 の LAN (1) = 3
```

となり、RIB には次のように入ります。

```
O IA  10.20.1.0/24 [110/3] via 172.16.0.2, 00:42:11, GigabitEthernet0/0
```

`[110/3]` は **AD = 110 / メトリック = 3** を表します。`O` は Area 内の経路、`O IA` は Area をまたいだ（Inter Area）経路です。

---

## ステップ 2：BGP で外界と経路を交換する

OSPF が「組織内の地図」を作るのに対して、BGP は **「どの AS を経由すればそのネットワークに届くか」を属性として運ぶ** プロトコルです。

### 2-1. TCP 179 でピアを張る

BGP は UDP の Hello ではなく、**TCP ポート 179** でセッションを確立します。

```text
EDGE-R1(config)# router bgp 65001
EDGE-R1(config-router)# bgp router-id 10.255.0.1
EDGE-R1(config-router)# neighbor 192.0.2.1 remote-as 65010
EDGE-R1(config-router)# neighbor 10.255.0.2 remote-as 65001
EDGE-R1(config-router)# neighbor 10.255.0.2 update-source Loopback0
EDGE-R1(config-router)# neighbor 10.255.0.2 next-hop-self
```

| 種類 | 相手                              | 特徴                                                                                 |
| ---- | --------------------------------- | ------------------------------------------------------------------------------------ |
| eBGP | 異なる AS（192.0.2.1 / AS 65010） | TTL = 1、NEXT_HOP を書き換える、AS_PATH でループ防止                                 |
| iBGP | 同じ AS（10.255.0.2 / AS 65001）  | 全員と Full Mesh が必要（または Route Reflector）、**NEXT_HOP を書き換えない** |

この構成では CORE-R2 を **Route Reflector（RR）** にして、R1 / R3 / R4 は R2 とだけ iBGP を張ります。

```text
CORE-R2(config-router)# neighbor 10.255.0.1 route-reflector-client
CORE-R2(config-router)# neighbor 10.255.0.3 route-reflector-client
CORE-R2(config-router)# neighbor 10.255.0.4 route-reflector-client
```

> iBGP には「iBGP で学んだ経路を別の iBGP ピアへ広告しない」というスプリットホライズン的な規則があります。Full Mesh か RR / Confederation でこれを回避します。

メッセージの種類は次の 5 つです。

| メッセージ    | 役割                                                 |
| ------------- | ---------------------------------------------------- |
| OPEN          | セッション確立（AS 番号、Router ID、能力情報の交換） |
| UPDATE        | 経路の追加・削除とパス属性を運ぶ                     |
| NOTIFICATION  | エラー通知してセッションを切る                       |
| KEEPALIVE     | セッション維持（通常 60 秒間隔）                     |
| ROUTE_REFRESH | 再送信を要求（ソフトリセット）                       |

### 2-2. 自組織の経路を「広報」する

BGP で自分のネットワークを広める方法は主に 3 つです。

| 方法                  | 内容                                                  |
| --------------------- | ----------------------------------------------------- |
| `network` コマンド  | すでに RIB にある prefix をそのまま広報（正確で推奨） |
| `redistribute`      | OSPF / スタティックなどを一括で再配布（filter 必須）  |
| `aggregate-address` | 複数の細かい prefix を集約して広報                    |

今回は自組織のグローバルアドレス 203.0.113.0/24 を広報します。`network` コマンドは **RIB に同じ prefix が存在しないと有効にならない** ため、Null0 向けの捨てルートを先に作るのが定番です。

```text
EDGE-R1(config)# ip route 203.0.113.0 255.255.255.0 Null0
EDGE-R1(config)# ip route 10.0.0.0 255.0.0.0 Null0
!
EDGE-R1(config)# router bgp 65001
EDGE-R1(config-router)# address-family ipv4
EDGE-R1(config-router-af)# network 203.0.113.0 mask 255.255.255.0
EDGE-R1(config-router-af)# neighbor 192.0.2.1 route-map FROM-ISP-A in
```

```text
EDGE-R1(config)# route-map FROM-ISP-A permit 10
EDGE-R1(config-route-map)# set local-preference 200
```

ISP-A から受け取った経路に `LOCAL_PREF = 200` を付与しています。これにより、バックアップ回線（ISP-B）よりメイン回線が優先されます。

> 自組織内の 10.0.0.0/8 は **ISP へ広報しません**。RFC 1918 のプライベートアドレスをインターネットに流すと、上流でフィルタされるか、最悪の場合他人の通信に影響します。内部は OSPF で完結させ、BGP に載せるのは公開アドレスのみ、が基本です。

### 2-3. UPDATE で届いた経路からベストパスを選ぶ

BGP は受け取った経路をすべて保持し、**ベストパス選択アルゴリズム**で 1 つだけを RIB に採用します（Cisco の代表的な順序）。

| 優先順 | 属性                                         | 望ましい値                             |
| ------ | -------------------------------------------- | -------------------------------------- |
| 1      | Weight（Cisco ローカル）                     | 大きい方                               |
| 2      | LOCAL_PREF                                   | 大きい方                               |
| 3      | ローカル生成経路                             | `network` / `aggregate` などが優先 |
| 4      | AS_PATH 長                                   | 短い方                                 |
| 5      | ORIGIN 種別                                  | IGP > EGP > Incomplete                 |
| 6      | MED                                          | 小さい方（通常同じ隣接 AS 同士で比較） |
| 7      | eBGP vs iBGP                                 | eBGP が優先                            |
| 8      | NEXT_HOP への IGP cost                       | 小さい方                               |
| 9      | 古い経路 / 小さい Router ID / 小さいピア地址 | タイブレーク                           |

OSPF が「cost の合計だけを見る」のに対し、BGP は **政策（ポリシー）を属性として載せて比較する** のが大きな違いです。

---

## ステップ 3：AD で「結局どのプロトコルの経路を採るか」を決める

ここまでで、RIB には次のような経路が同居しています。

- Connected（AD 0）
- Static（AD 1）
- OSPF（AD 110）
- eBGP（AD 20）
- iBGP（AD 200）

**同じ prefix に対して複数のプロトコルが経路を出していたら、AD（Administrative Distance / 管理距離）の小さい方が勝ちます。**

| 経路の出どころ | AD  |
| -------------- | --- |
| Connected      | 0   |
| Static         | 1   |
| eBGP           | 20  |
| EIGRP（内部）  | 90  |
| OSPF           | 110 |
| IS-IS          | 115 |
| RIP            | 120 |
| EIGRP（外部）  | 170 |
| iBGP           | 200 |

代表的な例が 203.0.113.0/24 です。

```
S     203.0.113.0/24 [1/0] via 0.0.0.0, Null0     ← AD 1 なのでこっちが RIB に残る
（BGP 的にも自局生成の 203.0.113.0/24 を持っているが RIB には載らない）
```

`show ip bgp` を見ると BGP テーブルには存在しているのに、`show ip route` では `S` になっている——これは異常ではなく **AD の仕組みどおり** です。

---

## 実際のルートテーブル

### EDGE-R1（ASBR / Area 0）のルートテーブル

```text
EDGE-R1# show ip route
Codes: L - local, C - connected, S - static, B - BGP
       O - OSPF, IA - OSPF inter area, E1 - OSPF external type 1
       E2 - OSPF external type 2, * - candidate default

Gateway of last resort is 192.0.2.1 to network 0.0.0.0

S     10.0.0.0/8 [1/0] via 0.0.0.0, Null0
C     10.10.0.0/16 is directly connected, GigabitEthernet0/1
L     10.10.0.1/32 is directly connected, GigabitEthernet0/1
O IA  10.20.1.0/24 [110/3] via 172.16.0.2, 00:42:11, GigabitEthernet0/0
O IA  10.30.1.0/24 [110/3] via 172.16.0.2, 00:41:58, GigabitEthernet0/0
C     10.255.0.1/32 is directly connected, Loopback0
O     10.255.0.2/32 [110/2] via 172.16.0.2, 00:42:15, GigabitEthernet0/0
O IA  10.255.0.3/32 [110/3] via 172.16.0.2, 00:42:05, GigabitEthernet0/0
O IA  10.255.0.4/32 [110/3] via 172.16.0.2, 00:41:52, GigabitEthernet0/0
C     172.16.0.0/30 is directly connected, GigabitEthernet0/0
L     172.16.0.1/32 is directly connected, GigabitEthernet0/0
O IA  172.16.1.0/30 [110/2] via 172.16.0.2, 00:42:11, GigabitEthernet0/0
O IA  172.16.2.0/30 [110/2] via 172.16.0.2, 00:41:58, GigabitEthernet0/0
S     203.0.113.0/24 [1/0] via 0.0.0.0, Null0
B     198.18.0.0/16 [20/0] via 192.0.2.1, 00:15:02
C     192.0.2.0/30 is directly connected, GigabitEthernet0/2
L     192.0.2.2/32 is directly connected, GigabitEthernet0/2
B*    0.0.0.0/0 [20/0] via 192.0.2.1, 00:15:02
```

読み解くポイントは次のとおりです。

| 出力                          | 意味                                                          |
| ----------------------------- | ------------------------------------------------------------- |
| `S 10.0.0.0/8 ... Null0`    | 未知の 10.x を破棄するための集約捨てルート                    |
| `O IA 10.20.1.0/24 [110/3]` | ABR（R2）経由で拠点 A を学習。cost 3                          |
| `B 198.18.0.0/16 [20/0]`    | eBGP で ISP-A から学習（AD 20）                               |
| `B* 0.0.0.0/0`              | `*` はデフォルト経路候補。`Gateway of last resort` と一致 |
| `via 192.0.2.1`             | 外部へのネクストホップは ISP 側のアドレス                     |

### BR-A-R3（拠点 A）のルートテーブル

```text
BR-A-R3# show ip route
Gateway of last resort is 10.255.0.1 to network 0.0.0.0

O IA  10.10.0.0/16 [110/3] via 172.16.1.1, 00:40:33, GigabitEthernet0/0
C     10.20.1.0/24 is directly connected, GigabitEthernet0/1
L     10.20.1.1/32 is directly connected, GigabitEthernet0/1
O IA  10.30.1.0/24 [110/3] via 172.16.1.1, 00:40:21, GigabitEthernet0/0
O IA  10.255.0.1/32 [110/3] via 172.16.1.1, 00:40:40, GigabitEthernet0/0
O     10.255.0.2/32 [110/2] via 172.16.1.1, 00:41:02, GigabitEthernet0/0
C     10.255.0.3/32 is directly connected, Loopback0
O IA  10.255.0.4/32 [110/3] via 172.16.1.1, 00:40:18, GigabitEthernet0/0
O IA  172.16.0.0/30 [110/2] via 172.16.1.1, 00:40:33, GigabitEthernet0/0
C     172.16.1.0/30 is directly connected, GigabitEthernet0/0
L     172.16.1.2/32 is directly connected, GigabitEthernet0/0
O IA  172.16.2.0/30 [110/2] via 172.16.1.1, 00:40:21, GigabitEthernet0/0
B*    0.0.0.0/0 [200/0] via 10.255.0.1, 00:14:50
```

R3 のデフォルト経路は **iBGP 由来（AD 200）** で、ネクストホップは R1 の Loopback `10.255.0.1` です。このアドレスは直接つながっていないので、R3 はさらに OSPF の `O IA 10.255.0.1/32 [110/3] via 172.16.1.1` を使って **再帰的にネクストホップを解決** しています。

> 「BGP ルートはあるのに ping が通らない」とき、まず疑うべきはこの再帰ルックアップです。iBGP は NEXT_HOP を書き換えないため、エッジで `neighbor ... next-hop-self` を忘れると、内部ルーターがネクストホップへ到達できません。

### BGP テーブルの中身

R3 からデフォルト経路を見ると、2 つのパスが並んでいます。

```text
BR-A-R3# show ip bgp 0.0.0.0
BGP routing table entry for 0.0.0.0/0, version 12
Paths: (2 available, best #1, table default)
  Advertised to update-groups: 1
  65010
    10.255.0.1 from 10.255.0.2 (10.255.0.1) originator 10.255.0.1
      Origin IGP, metric 0, localpref 200, valid, internal, best
      Originator: 10.255.0.1, Cluster list: 10.255.0.2
  65011
    10.255.0.4 from 10.255.0.2 (10.255.0.4) originator 10.255.0.4
      Origin IGP, metric 0, localpref 100, valid, internal
      Originator: 10.255.0.4, Cluster list: 10.255.0.2
```

- `*` … valid（ネクストホップへ到達できる）
- `>` … best（RIB に採用された）
- `i` … internal（iBGP 経由）
- `from 10.255.0.2` … この経路を教えてくれた RR
- `Originator` / `Cluster list` … RR が付けるループ防止用の属性

AS_PATH の長さはどちらも 1 ですが、**LOCAL_PREF 200 のメイン回線が選ばれている** ことがわかります。

### パケット転送時は「最長前方一致」

最後に、R1 宛てのパケットがどう扱われるかを見ます。

```text
EDGE-R1# show ip route 10.20.1.50
Routing entry for 10.20.1.0/24
  Known via "ospf 1", distance 110, metric 3, type inter area
  Last update from 172.16.0.2 on GigabitEthernet0/0, 00:42:11 ago
  Routing Descriptor Blocks:
  * 172.16.0.2, from 10.255.0.2, 00:42:11 ago, via GigabitEthernet0/0
      Route metric is 3, traffic share count is 1
```

送信先 `10.20.1.50` は R1 のルートテーブルで次の 3 つにマッチします。

| 候補         | 出どころ        | マスク長            |
| ------------ | --------------- | ------------------- |
| 10.20.1.0/24 | OSPF（O IA）    | 24 ←**採用** |
| 10.0.0.0/8   | Static（Null0） | 8                   |
| 0.0.0.0/0    | BGP（eBGP）     | 0                   |

ここで AD は関係ありません。**AD は同じ prefix 同士を比較するときにだけ使われ、長さが違うエントリ同士では最長前方一致（LPM）が優先されます**。

- `10.20.1.50` → /24 の OSPF 経路で R2 へ転送
- `10.99.99.99` → /8 の Null0 捨てルートで破棄
- `8.8.8.8` → /0 の BGP デフォルト経路で ISP-A へ転送

選ばれた RIB のエントリは、さらに **FIB（Forwarding Information Base）** へ書き込まれ、CEF の Adjacency テーブル（ARP / ND で得た L2 書き換え情報）と結びついて ASIC / TCAM 上で高速に転送されます。`show ip cef` や `show adjacency` でその状態を確認できます。

---

## 応用：再配布（redistribution）とその落とし穴

今回のトポロジーでは内部は OSPF、外部は BGP で役割が分かれているため、必須ではありませんが、**拠点間を BGP だけでつなぐ SD-WAN やクラウド接続**では、OSPF の経路を BGP に載せたり、その逆を行ったりします。

```text
EDGE-R1(config)# router bgp 65001
EDGE-R1(config-router)# address-family ipv4
EDGE-R1(config-router-af)# redistribute ospf 1 route-map OSPF-TO-BGP
EDGE-R1(config-router-af)# exit-address-family
```

ここで危険なのが **相互再配布ループ** です。

```
OSPF → BGP で広めた経路を、別の ASBR で BGP → OSPF に戻し、
さらに別の場所で OSPF → BGP に戻す … という循環
```

これを防ぐのが **route tag による印付け** です。

```text
route-map OSPF-TO-BGP deny 5
 match tag 65002          ! 外部(BGP)由来を OSPF が受け取ったものは戻さない
route-map OSPF-TO-BGP permit 10
 match ip address prefix-list OUR-PREFIXES
 set tag 65001            ! 自組織由来だと印をつける

route-map BGP-TO-OSPF deny 5
 match tag 65001          ! 自組織由来を BGP が受け取ったものは戻さない
route-map BGP-TO-OSPF permit 10
 set tag 65002            ! 外部由来だと印をつける
```

あわせて、prefix-list で「何を広めてよいか」を明示的に絞ることも必須です。

### デフォルト経路を OSPF にも流すとどうなるか

外部へのデフォルト経路を iBGP ではなく OSPF にも広報するとします。

```text
EDGE-R1(config-router)# default-information originate metric-type 1 metric 20
```

内部ルーターには Type 5 の External LSA が流れ、RIB は次のようになります。

```
O E1  0.0.0.0/0 [110/23] via 172.16.1.1, ...   ← AD 110
B*    0.0.0.0/0 [200/0]  via 10.255.0.1, ...   ← AD 200（同じ prefix なので比較される）
```

**同じ 0.0.0.0/0 同士なので AD が効き、OSPF（110）が iBGP（200）を上書きします。** 「iBGP に LOCAL_PREF を設定してメイン回線を選ばせたつもりが、OSPF のデフォルトに負けていた」という事故はよくあります。デフォルト経路をどちらのプロトコルで配るかは、設計で必ず一本化しましょう。

---

## よくある落とし穴チェックリスト

1. **`auto-cost reference-bandwidth` を変更していない**
   1 Gbps と 10 Gbps が同じ cost 1 になり、高速回線が選ばれない。全ルーターで同じ値に設定する。
2. **OSPF の `network` のワイルドカード指定ミス**
   想定外のインターフェースで Hello が飛んだり、逆に Loopback が広報されずネイバーが張れない。
3. **iBGP の NEXT_HOP を書き換えていない**
   `next-hop-self` をエッジ（または RR）で設定し、内部がネクストホップへ到達できるようにする。
4. **Route Reflector の client 指定漏れ**
   RR が経路を反射せず、一部のルーターだけ経路が見えない。
5. **AD と最長前方一致の混同**
   「AD が小さい方が勝つ」のは prefix が完全一致したときだけ。
6. **相互再配布でタグ付けを忘れる**
   自組織発の経路が外部を巡回して戻ってくる。prefix-list と tag の両方で絞る。
7. **プライベートアドレスを ISP へ広報する**
   RFC 1918 は広報せず、公開アドレスだけを BGP に載せる。

---

## まとめ

自組織のルートがルートテーブルに並ぶまでの流れを、もう一度コンパクトにまとめます。

| 段階                           | プロトコル / 仕組み                        | RIB での見え方      |
| ------------------------------ | ------------------------------------------ | ------------------- |
| インターフェース Up            | 自動生成                                   | `C` / `L`       |
| ローカルな集約・捨てルート     | 手動スタティック                           | `S`               |
| 組織内のネイバー確立・地図作成 | OSPF（Hello → LSDB → SPF）               | `O` / `O IA`    |
| 外部との経路交換               | eBGP（TCP 179, UPDATE）                    | `B`（AD 20）      |
| 組織内への外部経路の配布       | iBGP + Route Reflector                     | `B`（AD 200）     |
| 競合の解消                     | AD（同 prefix）＋ 各プロトコルのメトリック | 1 prefix 1 エントリ |
| 転送                           | 最長前方一致 → FIB / Adjacency            | ASIC で高速転送     |

**OSPF は「距離（cost）の合計」で最短の道を探し、BGP は「属性（LOCAL_PREF や AS_PATH）で政策を込めた道」を選ぶ。そして最後に AD と最長前方一致が、ルーターが実際に使う 1 行を決める。**

この 3 層を頭に入れておくと、`show ip route` の見え方がぐっと理解しやすくなります。

---

## 確認コマンド一覧

```text
show ip route                      ! メイン RIB（ルートテーブル）
show ip route <prefix>             ! 特定 prefix の詳細と採用理由
show ip route summary              ! プロトコル別の経路数
show ip ospf neighbor              ! OSPF ネイバーと状態（Full かどうか）
show ip ospf database              ! LSDB（LSA の一覧）
show ip ospf interface             ! cost、Hello/Dead タイマー、DR/BDR
show ip bgp summary                ! BGP ピアの状態と受信経路数
show ip bgp                        ! BGP テーブル（全パスとベストパス）
show ip bgp <prefix>               ! 特定 prefix の属性とベストパス理由
show ip bgp neighbors advertised-routes  ! ピアへ広報している経路
show ip cef <prefix>               ! FIB の状態
show adjacency                     ! L2 書き換え情報（ARP/ND の解決結果）
```
