---
title: "GREヘッダ"
emoji: "👯‍♀️"
type: "tech" # tech: 技術記事 / idea: アイデア記事
topics: ["networking"]
published: true
---


## 概要

GRE（Generic Routing Encapsulation）ヘッダは、**あるネットワーク層のパケットを別のネットワーク層のパケット内に「包み込む（カプセル化）」** ためのトンネリングプロトコルで使用されるヘッダです。RFC 2784で標準化されています。

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

| フィールド | サイズ | 説明 |
|-----------|--------|------|
| **C (Checksum Present)** | 1ビット | チェックサムフィールドが存在するか |
| **R (Routing Present)** | 1ビット | オフセット・ルーティングフィールドが存在するか |
| **K (Key Present)** | 1ビット | Keyフィールドが存在するか（トンネル識別に使用） |
| **S (Sequence Number Present)** | 1ビット | シーケンス番号フィールドが存在するか |
| **s (Strict Source Route)** | 1ビット | 厳密ソースルーティングフラグ |
| **Recur (Recursion Control)** | 3ビット | カプセル化の再帰許可数 |
| **Flags** | 5ビット | 予約・フラグ |
| **Ver** | 3ビット | バージョン（通常0） |
| **Protocol Type** | 16ビット | **カプセル化されたペイロードのプロトコル種別**（例：0x0800=IPv4、0x86DD=IPv6） |
| **Checksum** | 16ビット（オプション） | GREヘッダとペイロードのエラー検出用 |
| **Offset** | 16ビット（オプション） | ルーティング情報のオフセット |
| **Key** | 32ビット（オプション） | トンネルの識別子（複数トンネルを区別） |
| **Sequence Number** | 32ビット（オプション） | 順序制御・重複検出用 |

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

| 用途 | プロトコル | ポート/番号 |
|------|----------|-----------|
| 制御（接続確立・管理） | TCP | ポート1723 |
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

| 観点 | GRE | IPsec |
|------|-----|-------|
| **暗号化** | なし | あり（ESP/AH） |
| **認証** | なし | あり |
| **対応プロトコル** | 任意（IPv4/IPv6/PPPなど） | 主にIPのみ |
| ** multicast対応** | 可能 | トンネルモードで可能 |
| **用途** | トポロジー隠蔽・プロトコル変換 | セキュア通信 |

実務では　**「GRE over IPsec」** という形で両者を組み合わせることが多く、GREで柔軟なトンネリングを行い、IPsecで暗号化・認証を行います。

## GREの主な用途（PPTP以外）

GREは**PPTP以外でも広く使われています**。むしろ、PPTPはGREの多くの用途の中の一つです。
GREは「汎用トンネリングプロトコル」として、インターネットインフラの様々な場面で活用されています。

### 1. サイト間VPN（Site-to-Site VPN）のトンネル基盤

企業の拠点間をインターネット経由で接続する際、GREトンネルを使って**仮想的な専用線（point-to-pointリンク）** を作ります。

- 拠点Aのルーターと拠点Bのルーターの間にGREトンネルを張る
- その上でOSPFやEIGRPなどの**ルーティングプロトコルを動作させる**
- セキュリティが必要な場合は「GRE over IPsec」として暗号化を追加

> GRE is the tunnel protocol you reach for when a VPN alone will not do the job. It carries routing protocols across the internet, connects two private networks over a public link. <source-chip title="Tech@Layer-x.com" url="https://tech.layer-x.com/mikrotik-gre-tunnels-configuration-and-use-cases/" />

### 2. ルーティングプロトコルの運搬（BGP/OSPF over GRE）

IPsec VPNだけでは**マルチキャストやブロードキャストが通らない**ため、動的ルーティングプロトコル（OSPF、BGPなど）が使えません。GREトンネルはマルチキャストをサポートするので、以下が可能になります。

- **BGP over GRE**: クラウドプロバイダー（AWS、Azureなど）とオンプレミス間でBGPピアを確立
- **OSPF over GRE**: 分散拠点間で同一OSPFドメインを維持
- **EIGRP over GRE**: Cisco環境での動的ルーティング

> I still reach for GRE when I need a simple, explicit overlay that makes two routed domains behave like they share a point-to-point link, even when the underlay between them is messy, shared, or owned by someone else. <source-chip title="TheLinuxCode" url="https://thelinuxcode.com/generic-routing-encapsulation-gre-tunnel-practical-design-sizing-security-and-operations/" />

### 3. クラウド接続・マルチクラウド接続

AWS、Azure、Google Cloudなどのクラウド環境とオンプレミスを接続する際、GREトンネルが使われます。

- **AWS Direct Connect / Azure ExpressRoute**の裏側でもGRE類似のカプセル化が使われることがある
- **Equinix Fabric**などのインターコネクションサービスでGREを提供
- 複数クラウド間のルーティングを統合する「マルチクラウドネットワーク」でGRE over IPsecが一般的

### 4. MPLS VPNでのPE-PEトンネリング

ISPや大規模企業ネットワークで使われるMPLS VPNにおいて、プロバイダーエッジ（PE）ルーター間のトンネルとしてGREが使われることがあります。

- RFC 4797で標準化された「PE-PE GRE in BGP/MPLS VPN」
- MPLSが使えない経路や、特定のVPNサイト間でGREをフォールバックとして使用

> RFC 4797 - Use of Provider Edge to Provider Edge (PE-PE) Generic Routing Encapsulation (GRE) or IP in BGP/MPLS IP Virtual Private Networks <source-chip title="RFC 4797" url="https://datatracker.ietf.org/doc/html/rfc4797" />

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

> GRE Tunnels - Palo Alto Networks <source-chip title="Palo Alto Networks" url="https://docs.paloaltonetworks.com/ngfw/networking/gre-tunnels" />

### 8. コンテナ・Kubernetesネットワーク（一部）

CalicoなどのKubernetes CNIプラグインは、ノード間のトンネリングにIP in IPやVXLAN、**場合によってはGRE**を使用する設定が可能です。ただし、最近はVXLANやWireGuardの方が主流です。

## GREが選ばれる理由と選ばれない理由

### GREが選ばれる場面

| 場面 | 理由 |
|------|------|
| ルーティングプロトコルをトンネル上で動かしたい | マルチキャスト・ブロードキャスト対応 |
| 複数プロトコルを運搬したい | IPv4内にIPv6、IPX、AppleTalkなど任意のプロトコル |
| シンプルなオーバーレイが欲しい | ヘッダが小さく、設定が簡単 |
| IPsecと組み合わせたい | GRE over IPsecで「柔軟性＋セキュリティ」を両立 |

### GREが選ばれない場面

| 場面 | 代替技術 |
|------|---------|
| セキュリティだけが必要 | IPsec単体、WireGuard |
| NAT越えが必要 | UDPベースのWireGuard、OpenVPN |
| 高パフォーマンスが必要 | WireGuard、VXLAN |
| クラウドネイティブ環境 | VXLAN、Geneve、SR-IOV |




