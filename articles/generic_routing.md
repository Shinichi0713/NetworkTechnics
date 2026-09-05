---
title: "GREヘッダ"
emoji: "👯‍♀️"
type: "tech" # tech: 技術記事 / idea: アイデア記事
topics: ["networking"]
published: true
---
ネットワークごしに拠点から拠点に、組織のプライベートな通信を転送したいということがよくあります。
ただ、この通信、インターネット対応していないプライベートIPを用いた通信です。そのままでは相手に届けることが出来ません。
こんな時に使われるのがGRE（Generic Routing Encapsulation）ヘッダを用いたカプセル化です。

本日はGREについて説明していきます。

## 概要

GREヘッダは、**あるネットワーク層のパケットを別のネットワーク層のパケット内に「包み込む（カプセル化）」** ためのトンネリングプロトコルで使用されるヘッダです。RFC 2784で標準化されています。

### 1. GREヘッダの構造

GREヘッダは**最小4バイト（32ビット）** から始まり、オプションにより最大16バイトまで拡張されます。

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|C|R|K|S|s|Recur|  Flags  | Ver |         Protocol Type         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|      Checksum (optional)      |       Offset (optional)       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         Key (optional)                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                 Sequence Number (optional)                    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                 Routing (optional)                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

__各フィールドの意味__

| フィールド                            | サイズ                 | 説明                                                                                 |
| ------------------------------------- | ---------------------- | ------------------------------------------------------------------------------------ |
| **C (Checksum Present)**        | 1ビット                | チェックサムフィールドが存在するか                                                   |
| **R (Routing Present)**         | 1ビット                | オフセット・ルーティングフィールドが存在するか                                       |
| **K (Key Present)**             | 1ビット                | Keyフィールドが存在するか（トンネル識別に使用）                                      |
| **S (Sequence Number Present)** | 1ビット                | シーケンス番号フィールドが存在するか                                                 |
| **s (Strict Source Route)**     | 1ビット                | 厳密ソースルーティングフラグ                                                         |
| **Recur (Recursion Control)**   | 3ビット                | カプセル化の再帰許可数                                                               |
| **Flags**                       | 5ビット                | 予約・フラグ                                                                         |
| **Ver**                         | 3ビット                | バージョン（通常0）                                                                  |
| **Protocol Type**               | 16ビット               | **カプセル化されたペイロードのプロトコル種別**（例：0x0800=IPv4、0x86DD=IPv6） |
| **Checksum**                    | 16ビット（オプション） | GREヘッダとペイロードのエラー検出用                                                  |
| **Offset**                      | 16ビット（オプション） | ルーティング情報のオフセット                                                         |
| **Key**                         | 32ビット（オプション） | トンネルの識別子（複数トンネルを区別）                                               |
| **Sequence Number**             | 32ビット（オプション） | 順序制御・重複検出用                                                                 |

### 2. GREの役割：トンネリング

GREの本質は、**「中身は何でもよい」** という点にあります。

```
[ 外側IPヘッダ ] [ GREヘッダ ] [ 内側IPパケット（ペイロード） ]
     ↑            ↑                  ↑
  配送用        トンネル制御      実際に送りたいデータ
```

- 外側IPヘッダの**プロトコル番号は47**（IP Protocol 47 = GRE）
- GREヘッダの**Protocol Type**で「中身が何か」を示す
- 中身はIPv4、IPv6、AppleTalk、IPX、PPPなど、**任意のプロトコル**をカプセル化可能

### 3. PPTPとの関係

PPTPはGREを **「PPPフレームの運搬手段」** として使用します。

__PPTPの通信構造__

| 用途                     | プロトコル                      | ポート/番号                  |
| ------------------------ | ------------------------------- | ---------------------------- |
| 制御（接続確立・管理）   | TCP                             | ポート1723                   |
| データ（実際の通信内容） | **GRE**（IP Protocol 47） | キーフィールドでコールID識別 |

__PPTPにおけるGREヘッダの特徴__

- **Keyフィールドを使用**: PPTPはGREヘッダのKeyフィールドに**コールID（Call ID）**を格納し、複数のVPNセッションを区別します
- **Sequence Number**: 順序を保証するためにシーケンス番号を使用することもあります
- **ペイロードはPPPフレーム**: GREヘッダのProtocol TypeにはPPP（通常0x880B）が設定されます

```
[ IPヘッダ (Proto=47) ] [ GREヘッダ (Key=Call ID, Proto=0x880B) ] [ PPPフレーム ] [ IPパケット ]
```

### 4. GREの特徴と限界

__メリット__

- **プロトコル非依存**: IPv4内にIPv6を送る、IPv6内にIPv4を送るなど、異種プロトコルのトンネリングが可能
- **シンプル**: ヘッダが小さく、オーバーヘッドが少ない
- **汎用性**: ルーター間のVPN、モバイルIP、6to4トンネルなど幅広く使われる

__デメリット・限界__

- **暗号化なし**: GRE自体は**暗号化を提供しません**。機密性が必要な場合はIPsecと組み合わせる必要があります
- **認証なし**: GREヘッダ単体では送信元の認証ができません
- **NAT越えが難しい**: IP Protocol 47（GRE）は、一般的なTCP/UDPポートフォワーディングとは異なり、NAT越えに対応していないルーターが多いです（PPTPがNAT環境で問題になりやすい理由の一つ）

### 5. GREとIPsecの違い

GREとIPsecはよく比較されますが、役割が異なります。

| 観点                     | GRE                            | IPsec                |
| ------------------------ | ------------------------------ | -------------------- |
| **暗号化**         | なし                           | あり（ESP/AH）       |
| **認証**           | なし                           | あり                 |
| **対応プロトコル** | 任意（IPv4/IPv6/PPPなど）      | 主にIPのみ           |
| ** multicast対応**       | 可能                           | トンネルモードで可能 |
| **用途**           | トポロジー隠蔽・プロトコル変換 | セキュア通信         |

実務では　**「GRE over IPsec」** という形で両者を組み合わせることが多く、GREで柔軟なトンネリングを行い、IPsecで暗号化・認証を行います。

## GREによるカプセル化

GREのカプセル化は、**「元のパケットをそのまま別のパケットのデータ領域に入れる」** というシンプルな操作です。ただし、ネットワークのレイヤーがどう変化するかを追うと、その仕組みが明確になります。

### カプセル化の基本ステップ

__ステップ1：元のパケット（カプセル化前）__

例として、拠点AのPCが拠点BのPCへ送信するIPv4パケットを想定します。

```
[ イーサネットヘッダ ] [ IPヘッダ (送信元10.0.1.10 → 宛先10.0.2.20) ] [ TCP/UDPデータ ]
        ↓
このパケットはプライベートIPアドレスを使っているため、インターネット上では直接届かない
```

__ステップ2：GREヘッダの付加__

ルーターがこのパケットをGREトンネルの対象と判断すると、**元のIPパケット全体の前にGREヘッダを挿入**します。

```
[ イーサネットヘッダ ] [ GREヘッダ ] [ IPヘッダ (10.0.1.10 → 10.0.2.20) ] [ TCP/UDPデータ ]
                          ↑
                    Protocol Type = 0x0800 (IPv4)
                    これは「中身はIPv4パケットです」と示す
```

この時点で、元のIPパケットは**GREヘッダの「ペイロード」** になっています。

__ステップ3：外側IPヘッダの付加（カプセル化の完成）__

さらに、GREヘッダ＋元パケット全体を**新しいIPパケットのデータ領域**として包み込みます。

```
[ 新イーサネットヘッダ ] [ 外側IPヘッダ ] [ GREヘッダ ] [ 内側IPヘッダ ] [ TCP/UDPデータ ]
                              ↑                ↑
                    (送信元203.0.113.1      Protocol Type
                     → 宛先198.51.100.1)    = 0x0800 (IPv4)

外側IPヘッダのプロトコル番号 = 47 (GRE)
```

__ステップ4：物理的な送信__

この完成したパケットは、インターネット上を**外側IPヘッダの送信元・宛先アドレス**に基づいてルーティングされます。中間のルーターはGREヘッダや内側パケットの存在を知りません。

### パケット構造の変化を図で比較

__カプセル化前（通常のパケット）__

```
┌─────────────────┐
│  イーサネット    │  ← レイヤー2
├─────────────────┤
│   IPヘッダ       │  ← レイヤー3（送信元10.0.1.10）
├─────────────────┤
│  TCP/UDPヘッダ   │  ← レイヤー4
├─────────────────┤
│     データ       │
└─────────────────┘
```

__カプセル化後（GREトンネルパケット）__

```
┌─────────────────┐
│  イーサネット    │  ← レイヤー2（新）
├─────────────────┤
│   外側IPヘッダ   │  ← レイヤー3（送信元203.0.113.1 → 198.51.100.1）
│  (Protocol = 47)│      プロトコル番号47 = GRE
├─────────────────┤
│   GREヘッダ      │  ← トンネル制御情報
│ (Proto = 0x0800)│      Protocol Type = IPv4
├─────────────────┤
│   内側IPヘッダ   │  ← 元のレイヤー3（送信元10.0.1.10 → 10.0.2.20）
├─────────────────┤
│  TCP/UDPヘッダ   │  ← 元のレイヤー4
├─────────────────┤
│     データ       │  ← 元のデータ
└─────────────────┘
         ↑
    この部分全体が「外側IPのデータ領域」として扱われる
```

### 受信側での「脱カプセル化」のプロセス

トンネルの出口ルーター（上記の例では198.51.100.1側）では、逆の操作が行われます。

| 順序 | 処理                                                               |
| ---- | ------------------------------------------------------------------ |
| 1    | 外側IPヘッダを確認し、プロトコル番号47（GRE）を検出                |
| 2    | GREヘッダを解析し、Protocol Typeから「中身がIPv4（0x0800）」と判別 |
| 3    | GREヘッダを取り除き、内側パケットを元のインターフェースから転送    |
| 4    | 内側IPパケットはプライベートネットワーク（10.0.2.0/24）へ届く      |

### PPTPの場合のカプセル化（具体例）

PPTPでは、GREのペイロードが**PPPフレーム**になります。構造が少し複雑です。

```
[ IPヘッダ (Proto=47) ] [ GREヘッダ ] [ PPPヘッダ ] [ IPパケット ] [ TCP/UDP ] [ データ ]
                              ↑            ↑
                    Key = Call ID      Protocol = 0x880B (PPP)
                    (セッション識別)
```

__PPTPカプセル化の特徴__

- GREヘッダの**Keyフィールド**にコールIDが入り、複数のVPNセッションを区別
- GREヘッダの**Protocol Type**は0x880B（PPP）
- PPPフレームの中にさらにIPパケットが入る**「三重の入れ子構造」**になる

## GREの主な用途（PPTP以外）

GREは**PPTP以外でも広く使われています**。むしろ、PPTPはGREの多くの用途の中の一つです。
GREは「汎用トンネリングプロトコル」として、インターネットインフラの様々な場面で活用されています。

### 1. サイト間VPN（Site-to-Site VPN）のトンネル基盤

企業の拠点間をインターネット経由で接続する際、GREトンネルを使って**仮想的な専用線（point-to-pointリンク）** を作ります。

- 拠点Aのルーターと拠点Bのルーターの間にGREトンネルを張る
- その上でOSPFやEIGRPなどの**ルーティングプロトコルを動作させる**
- セキュリティが必要な場合は「GRE over IPsec」として暗号化を追加

> GRE is the tunnel protocol you reach for when a VPN alone will not do the job. It carries routing protocols across the internet, connects two private networks over a public link. `<source-chip title="Tech@Layer-x.com" url="https://tech.layer-x.com/mikrotik-gre-tunnels-configuration-and-use-cases/" />`

### 2. ルーティングプロトコルの運搬（BGP/OSPF over GRE）

IPsec VPNだけでは**マルチキャストやブロードキャストが通らない**ため、動的ルーティングプロトコル（OSPF、BGPなど）が使えません。GREトンネルはマルチキャストをサポートするので、以下が可能になります。

- **BGP over GRE**: クラウドプロバイダー（AWS、Azureなど）とオンプレミス間でBGPピアを確立
- **OSPF over GRE**: 分散拠点間で同一OSPFドメインを維持
- **EIGRP over GRE**: Cisco環境での動的ルーティング

> I still reach for GRE when I need a simple, explicit overlay that makes two routed domains behave like they share a point-to-point link, even when the underlay between them is messy, shared, or owned by someone else. `<source-chip title="TheLinuxCode" url="https://thelinuxcode.com/generic-routing-encapsulation-gre-tunnel-practical-design-sizing-security-and-operations/" />`

### 3. クラウド接続・マルチクラウド接続

AWS、Azure、Google Cloudなどのクラウド環境とオンプレミスを接続する際、GREトンネルが使われます。

- **AWS Direct Connect / Azure ExpressRoute**の裏側でもGRE類似のカプセル化が使われることがある
- **Equinix Fabric**などのインターコネクションサービスでGREを提供
- 複数クラウド間のルーティングを統合する「マルチクラウドネットワーク」でGRE over IPsecが一般的

### 4. MPLS VPNでのPE-PEトンネリング

ISPや大規模企業ネットワークで使われるMPLS VPNにおいて、プロバイダーエッジ（PE）ルーター間のトンネルとしてGREが使われることがあります。

- RFC 4797で標準化された「PE-PE GRE in BGP/MPLS VPN」
- MPLSが使えない経路や、特定のVPNサイト間でGREをフォールバックとして使用

> RFC 4797 - Use of Provider Edge to Provider Edge (PE-PE) Generic Routing Encapsulation (GRE) or IP in BGP/MPLS IP Virtual Private Networks `<source-chip title="RFC 4797" url="https://datatracker.ietf.org/doc/html/rfc4797" />`

### 5. IPv6トンネリング（6to4、ISATAPなど）

IPv4ネットワーク上にIPv6を運搬するトンネリング技術でGREが使われます。

- **6to4**: IPv4インターネット上でIPv6サイトを接続
- **ISATAP (Intra-Site Automatic Tunnel Addressing Protocol)**: サイト内でIPv6をIPv4上に運搬
- **Teredo**: UDP上のIPv6トンネリング（GREとは異なるが、同じ「カプセル化」の発想）

### 6. モバイルIP（Mobile IP）

モバイル端末がネットワーク間を移動してもIPアドレスを維持する技術です。GREはモバイルノードとホームエージェント間のトンネルに使われます。

- 端末が外出先ネットワークにいても、ホームネットワークのIPを維持
- RFC 2003（IP in IP）やGREがトンネリング手段として使用

### 7. ネットワーク機器・セキュリティ製品間の通信

Palo Alto NetworksやCiscoなどのエンタープライズ機器で、GREトンネルは標準機能として提供されています。

- **Palo Alto Networks NGFW**: GREトンネルによるサイト間接続をサポート
- **Cisco IOS/IOS XE**: GRE over IPsecを大規模展開
- **MikroTik RouterOS**: GREトンネルによるルーティングプロトコル運搬が一般的

> GRE Tunnels - Palo Alto Networks `<source-chip title="Palo Alto Networks" url="https://docs.paloaltonetworks.com/ngfw/networking/gre-tunnels" />`

### 8. コンテナ・Kubernetesネットワーク（一部）

CalicoなどのKubernetes CNIプラグインは、ノード間のトンネリングにIP in IPやVXLAN、**場合によってはGRE**を使用する設定が可能です。ただし、最近はVXLANやWireGuardの方が主流です。

## GREが選ばれる理由と選ばれない理由

### GREが選ばれる場面

| 場面                                           | 理由                                             |
| ---------------------------------------------- | ------------------------------------------------ |
| ルーティングプロトコルをトンネル上で動かしたい | マルチキャスト・ブロードキャスト対応             |
| 複数プロトコルを運搬したい                     | IPv4内にIPv6、IPX、AppleTalkなど任意のプロトコル |
| シンプルなオーバーレイが欲しい                 | ヘッダが小さく、設定が簡単                       |
| IPsecと組み合わせたい                          | GRE over IPsecで「柔軟性＋セキュリティ」を両立   |

### GREが選ばれない場面

| 場面                   | 代替技術                      |
| ---------------------- | ----------------------------- |
| セキュリティだけが必要 | IPsec単体、WireGuard          |
| NAT越えが必要          | UDPベースのWireGuard、OpenVPN |
| 高パフォーマンスが必要 | WireGuard、VXLAN              |
| クラウドネイティブ環境 | VXLAN、Geneve、SR-IOV         |

## セキュリティ対策

GREは先述の通り**暗号化も認証も提供しない**プロトコルです。そのため、インターネットなどの非信頼ネットワークでGREトンネルを使う場合は、必ず何らかのセキュリティ対策が必要です。以下、実務で使われる主要な対策を整理します。

### 1. 根本対策：GRE over IPsec（最も重要・最も一般的）

GRE単体ではデータが平文のまま流れるため、**GRE over IPsec**という形でIPsecを組み合わせることが標準的な対策です。

__なぜ組み合わせるのか__

| プロトコル | 提供する機能                                     | 提供しない機能                       |
| ---------- | ------------------------------------------------ | ------------------------------------ |
| GRE        | カプセル化・マルチキャスト対応・多プロトコル運搬 | 暗号化・認証                         |
| IPsec      | 暗号化・認証・完全性保証                         | マルチキャスト対応・多プロトコル運搬 |

> GRE is just a way to encapsulate traffic, IPsec is what gives you the security, and GRE over IPsec is the classic recipe for building a secure routed overlay on top of an untrusted underlay. `<source-chip title="AlphaPrep" url="https://blog.alphaprep.net/gre-and-ipsec-tunneling-for-ccnp-350-401-encor-configuration-verification-and-troubleshooting/" />`

__2つの実装方式__

- **IPsec Transport Mode**: GREパケット全体を暗号化。オーバーヘッドが少なく、一般的に推奨される方式
- **IPsec Tunnel Mode**: 外側IPヘッダも含めて暗号化。より強固だがオーバーヘッドが大きい

> Combine GRE tunnels with IPsec transport mode to create encrypted GRE tunnels that provide both multi-protocol encapsulation and cryptographic security. `<source-chip title="OneUptime" url="https://oneuptime.com/blog/post/2026-03-20-gre-over-ipsec-encrypted-tunneling/view" />`

### 2. トンネルエンドポイントの保護：ACLとファイアウォール

GREトンネルの両端（トンネルソース・デスティネーションIP）に対し、**アクセス制御リスト（ACL）**で通信を絞り込みます。

__推奨ACLルール__

| 方向                       | 許可                                          | 拒否                        |
| -------------------------- | --------------------------------------------- | --------------------------- |
| 外部インターフェース（IN） | 信頼するピアIPからのIP Protocol 47（GRE）のみ | それ以外のGREパケットすべて |
| 外部インターフェース（IN） | IKE/IPsec用のUDP 500、4500（IPsec使用時）     | 不要なVPNプロトコル         |

__注意点__

- GREトンネルはIP Protocol 47を使用するため、**TCP/UDPのポート番号ではフィルタリングできません**
- ファイアウォールがIPsec/GREに対応していない場合、トンネルが正常に機能しないことがあります

### 3. トンネルキー（Keyフィールド）の活用

GREヘッダの**Keyフィールド（32ビット）** を使うことで、同一IP間の複数トンネルを識別し、**意図しないトンネル接続を防ぐ**ことができます。

__効果__

- 同じ送信元・宛先IP間で、トンネルIDを使って正当な接続のみを許可
- 設定ミスやなりすまし接続のリスクを軽減

__限界__

- Keyフィールドは**平文**で送信されるため、パケットキャプチャで読み取られる可能性がある
- セキュリティ機密情報としては扱えない（「簡易識別子」程度の位置づけ）

### 4. Keepaliveによるトンネル健全性監視

GREトンネルは「状態を持たない（stateless）」ため、相手がダウンしてもトンネルインターフェースがUPのままになることがあります。これにより**ブラックホール（通信が消える）** が発生します。

__Keepaliveの仕組み__

- トンネル両端が定期的にkeepaliveパケットを交換
- 一定回数応答がない場合、トンネルインターフェースをDOWNにし、代替経路（通常のインターネット経由など）に切り替える

> The IP-over-IP (usually GRE) tunnels (commonly in combination with IPsec to provide security) are frequently used when you want to transport private IP traffic over public IP network. If you use the GRE tunnels in combination with default routing (or route summarization), you can get serious routing issues when the tunnel destination disappears. `<source-chip title="ipSpace.net" url="https://blog.ipspace.net/2007/10/gre-tunnel-keepalives/" />`

__セキュリティ上の効果__

- DoS攻撃でトンネルが不通になった場合の**自動検知・フェイルオーバー**
- トンネルダウン時の**ルーティングループ防止**

### 5. ルーティング保護：再帰ルーティングの防止

GREトンネルの宛先IPへ到達する経路が、**トンネル自身の中を通る**ように設定されると、無限ループ（再帰ルーティング）が発生します。

__対策__

- トンネルデスティネーションIPへの経路は、**必ずトンネル外の物理インターフェース**を使うように設定する
- 静的ルートでトンネルデスティネーションIPのネクストホップを明示的に指定する

> Recursive routing, and how to avoid it. `<source-chip title="Network Direction" url="https://networkdirection.net/articles/routingandswitching/gretunnels/advancedgre/" />`

### 6. MTU・MSS調整による安定性確保

GREヘッダ（最小4バイト、オプション付きで最大16バイト）＋外側IPヘッダ（20バイト）により、**ペイロードが小さくなる**ため、パケット断片化や通信障害が起こりやすくなります。

__推奨設定__

| 項目                    | 推奨値                 | 説明                                                |
| ----------------------- | ---------------------- | --------------------------------------------------- |
| トンネルMTU             | 1400〜1476バイト       | デフォルト1500からGRE/IPsecオーバーヘッドを差し引く |
| TCP MSS Clamping        | 1360バイト程度         | TCP SYN時にMSSを自動調整                            |
| IPsecの「DFビット無視」 | 有効化（必要に応じて） | Don't Fragmentビットをクリアして断片化を許可        |

### 7. モダンな代替案：WireGuard・VXLANの検討

新規構築の場合、GRE over IPsecではなく、**より安全で管理しやすい技術**を検討する価値があります。

| 技術                | GRE over IPsecとの比較                                                                         |
| ------------------- | ---------------------------------------------------------------------------------------------- |
| **WireGuard** | カーネル内実装で高速。暗号化・認証を標準搭載。設定が極めてシンプル。UDPベースでNAT越えも容易。 |
| **VXLAN**     | データセンター・クラウド環境向け。UDPベースでGREよりNATに強い。                                |
| **OpenVPN**   | TCP/UDP両対応。クロスプラットフォーム。設定が柔軟。                                            |
| **IPsec VTI** | GREなしでIPsecだけで仮想トンネルインターフェースを作る。Ciscoなどでサポート。                  |

## 総括

GREの本質は、**「何でも包んで運べる、ただし何も守らない、シンプルなトンネルの接着剤」** です。

### 1. 包むことに特化したプロトコル

- GREは**カプセル化だけ**を行います
- IPv4、IPv6、PPP、AppleTalkなど**任意のプロトコル**を別のIPパケット内に包んで運べます
- ヘッダは最小4バイトと小さく、**オーバーヘッドが極めて少ない**
- IP Protocol 47で識別され、Protocol Typeフィールドで「中身が何か」を示す

### 2. セキュリティは持たない

- **暗号化なし**、**認証なし**、**完全性保証なし**
- 平文で流れるため、インターネット上で単独使用は危険
- セキュリティが必要なら、必ず**IPsecと組み合わせる（GRE over IPsec）**
- GREが「包む」、IPsecが「守る」という役割分担が基本

### 3. 柔軟性が最大の強み

- IPsecだけでは通せない**マルチキャスト・ブロードキャスト**を通せる
- ルーティングプロトコル（OSPF、BGP）をトンネル上で動かせる
- サイト間VPN、クラウド接続、IPv6移行、MPLSなど**インフラのあらゆる場所**で使われる
- PPTPはその応用例の一つに過ぎない

### 一言で表すと

> **GREは「包んで運ぶ」ことに特化したシンプルなトンネリング機構。セキュリティは持たないが、どんな中身でも運べる柔軟性が、現代のネットワークインフラにおいて依然として不可欠な存在である。**
