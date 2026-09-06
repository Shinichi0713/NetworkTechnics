---
title: "拡張認証プロトコルEAP"
emoji: "❣"
type: "tech" # tech: 技術記事 / idea: アイデア記事
topics: ["reinforce-learning", "AI", "RL"]
published: true
---
組織のネットワークにデバイスを接続する＝組織の重要なデータにアクセスできるようになることを示します。
接続を提供するネットワーク側にはデバイスが本当に接続をしてよいかをすぐに判断する必要があります。
こんな時に使われる認証プロトコルにEAPと呼ばれるものがあります。

## 概要

EAP（Extensible Authentication Protocol：拡張認証プロトコル）は、様々な認証方式を柔軟に利用するための認証フレームワークです。[Extensible Authentication Protocol](https://ja.wikipedia.org/wiki/Extensible_Authentication_Protocol)

### 主な特徴

EAPは「認証方式そのもの」ではなく、複数の認証方式を統一的に扱えるようにする「共通の枠組み」です。[EAPとは](https://kobesoft.co.jp/mikata/words/security/eap/) そのため、パスワード、デジタル証明書、ICカード、指紋認証など、多様な認証方法を必要に応じて組み込むことができます。

### 主な用途

主に [PPP](https://yoshishinnze.hatenablog.com/entry/2025/12/15/000000?)（Point-to-Point Protocol）やイーサネットなどのデータリンク層で利用され、[IEEE 802.1X](https://ja.wikipedia.org/wiki/IEEE_802.1X) を使用した無線LAN（Wi-Fi）や有線LANのアクセス制御、VPN接続などのネットワーク認証で広く使われています。[Windows におけるネットワーク アクセスの拡張認証プロトコル (EAP)](https://learn.microsoft.com/ja-jp/windows-server/networking/technologies/extensible-authentication-protocol/network-access)

### 代表的な認証方式

EAP上で動作する主な認証方式には以下のようなものがあります。[EAPとは - IT用語辞典 e-Words](https://e-words.jp/w/EAP.html)

- **EAP-MD5**：ユーザー名とパスワードをMD5ハッシュ化して認証します（クライアントのみ認証）。
- **EAP-TLS**：サーバとクライアント双方に電子証明書を必要とし、相互認証を行います。
- **EAP-TTLS・PEAP**：サーバ側の証明書で暗号化トンネルを確立し、その中で認証情報をやり取りします。

EAPは1995年に開発され、現在では [RFC 3748](https://tex2e.github.io/rfc-translater/html/rfc3748.html) として標準化されています。[RFC 3748 - Extensible Authentication Protocol (EAP) 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc3748.html)

## EAPが提供するもの

EAPが具体的に提供しているものは、以下の通りです。

### 1. 認証方式の共通フレームワーク

EAPは「認証方式そのもの」ではなく、様々な認証方式を統一的に扱えるようにする共通の枠組み（フレームワーク）を提供します。[RFC 3748 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc3748.html) これにより、パスワード、証明書、ICカードなど異なる認証方式を、同じ手順で利用できるようになります。

### 2. 4種類の基本メッセージ形式

EAPは認証のやり取りに使用する標準的なメッセージ形式を定義しています。[RFC 3748 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc3748.html)

- **Request**：認証者からピア（クライアント）へ認証情報を要求
- **Response**：ピアから認証者へ認証情報を応答
- **Success**：認証が成功したことを通知
- **Failure**：認証が失敗したことを通知

### 3. 認証方式のネゴシエーション機能

EAPは、認証を開始する前に「どの認証方式を使うか」を交渉する手順を提供します。[EAPとは - IT用語辞典 e-Words](https://e-words.jp/w/EAP.html) 両者が対応している方式を確認し、合意した方式で認証を開始します。

### 4. 重複排除と再送信の仕組み

EAPは、通信の信頼性を高めるため、パケットの重複排除や再送信に関する独自のサポートを提供します。[RFC 3748 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc3748.html) ただし、パケットの順序保証は下位層（PPPやIEEE 802など）に依存します。

### 5. バックエンド認証サーバーとの連携（パススルー）

EAPは、認証装置（アクセスポイントやスイッチ）が認証処理を中継する「パススルー」動作を可能にします。[RFC 3748 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc3748.html) これにより、認証装置自体にすべての認証方式を実装しなくても、[RADIUS](https://ja.wikipedia.org/wiki/RADIUS) などのバックエンド認証サーバーに処理を委任できます。

### 6. IPを必要としないデータリンク層での動作

EAPは [PPP](https://ja.wikipedia.org/wiki/Point-to-Point_Protocol) や [IEEE 802](https://ja.wikipedia.org/wiki/IEEE_802)（イーサネット・無線LANなど）などのデータリンク層上で直接動作し、IP層を必要としません。[RFC 3748 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc3748.html) これにより、ネットワーク接続を確立する前の段階で認証を行うことができます。

## EAPの認証

EAPが認証を行う対象と手順について、以下にまとめます。

### 認証の対象

EAPが認証するのは、**ネットワークに接続しようとするクライアント（ピア）** です。[RFC 3748 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc3748.html)

具体的には以下の3つの役割が関与します。

- **ピア（Peer）**：認証を受ける側。PCやスマートフォンなどのクライアント。[IEEE 802.1X](https://ja.wikipedia.org/wiki/IEEE_802.1X) では**サプリカント**とも呼ばれます。
- **認証者（Authenticator）**：認証を仲介する装置。無線LANアクセスポイントやスイッチなどです。
- **EAPサーバー / バックエンド認証サーバー**：実際に認証を処理するサーバー。[RADIUS](https://ja.wikipedia.org/wiki/RADIUS) サーバーなどが該当します。

認証者はクライアントと認証サーバーの間でメッセージを中継（パススルー）し、認証サーバーが最終的に認証の成否を決定します。[RFC 3748 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc3748.html)

### 認証の手順

EAPの認証は、以下の流れで進みます。[IEEE802.1X認証とは、EAPとは - その2](https://www.infraexpert.com/study/wireless51.html)

**① 認証の開始**
認証者（APなど）からピアに対して [EAP-Request/Identity](https://tex2e.github.io/rfc-translater/html/rfc3748.html) を送信し、ピアは自身の識別子（ユーザー名など）を [EAP-Response/Identity](https://tex2e.github.io/rfc-translater/html/rfc3748.html) で返します。

**② 認証方式のネゴシエーション**
認証者とピアの間で、どのEAPメソッド（[EAP-TLS](https://study-sec.com/eap-tls/)、[PEAP](https://e-words.jp/w/EAP.html)、[EAP-TTLS](https://e-words.jp/w/EAP.html) など）を使用するかを交渉します。

**③ 認証情報の交換**
選択された認証方式に従い、ピアとEAPサーバーの間で認証情報をやり取りします。例えば、パスワードのハッシュ値やデジタル証明書などが交換されます。この間、認証者は両者のメッセージを中継します。[IEEE802.1X認証とは、EAPとは - その2](https://www.infraexpert.com/study/wireless51.html)

**④ 認証結果の通知**
認証が成功すれば、EAPサーバーから [EAP-Success](https://tex2e.github.io/rfc-translater/html/rfc3748.html) が送信されます。失敗した場合は [EAP-Failure](https://tex2e.github.io/rfc-translater/html/rfc3748.html) が送信されます。

**⑤ ネットワークアクセスの許可**
[EAP-Success](https://tex2e.github.io/rfc-translater/html/rfc3748.html) を受信した認証者は、ピアに対してポートのブロックを解除し、ネットワークへのデータ通信を許可します。認証に失敗した場合は、通信を継続して遮断します。[IEEE802.1X認証とは、EAPとは - その2](https://www.infraexpert.com/study/wireless51.html)

### 補足：相互認証

[EAP-TLS](https://study-sec.com/eap-tls/) のように、ピアだけでなくサーバー側も証明書で認証される**相互認証**を行う方式もあります。この場合、ピアとサーバーの両方が正当であることを確認し合います。[RFC 5216 - EAP-TLS認証プロトコル 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc5216.html)

## 用途

EAPは主に、ネットワークへの接続前に「誰が接続しようとしているか」を確認するために使われます。[RFC 3748 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc3748.html) 具体的な用途は以下の通りです。

### 1. 無線LAN（Wi-Fi）の認証

企業や学校などの [WPA2-Enterprise](https://ja.wikipedia.org/wiki/Wi-Fi_Protected_Access) や [WPA3-Enterprise](https://ja.wikipedia.org/wiki/Wi-Fi_Protected_Access) 環境で、EAPは標準的に使用されます。[IEEE 802.1X](https://ja.wikipedia.org/wiki/IEEE_802.1X) と組み合わせることで、各ユーザーごとに異なる認証情報を用いてWi-Fiに接続できます。[Windows におけるネットワーク アクセスの拡張認証プロトコル (EAP)](https://learn.microsoft.com/ja-jp/windows-server/networking/technologies/extensible-authentication-protocol/network-access)

### 2. 有線LANの認証

無線だけでなく、有線LANのスイッチポートにおいても [IEEE 802.1X](https://ja.wikipedia.org/wiki/IEEE_802.1X) 認証にEAPが使われます。これにより、不正な端末が社内ネットワークのLANポートに挿しても、認証に失敗すれば通信が遮断されます。[IEEE 802.1X認証とは？基本的な認証の仕組みを解説](https://www.dlink-jp.com/column/ieee802.1x.html)

### 3. VPN接続の認証

[PPP](https://ja.wikipedia.org/wiki/Point-to-Point_Protocol)（Point-to-Point Protocol）上で動作するVPN接続（ダイヤルアップやL2TPなど）において、EAPはユーザー認証の枠組みとして利用されます。[EAPとは - IT用語辞典 e-Words](https://e-words.jp/w/EAP.html)

### 4. ダイヤルアップ接続

EAPはもともとPPPの拡張として開発されたため、ダイヤルアップ回線でのユーザー認証にも使用されてきました。[RFC 3748 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc3748.html)

## RADIUSとの比較

EAPとRADIUSは、役割が異なる2つのプロトコルです。以下に違いをまとめます。

### EAPとは

EAP（Extensible Authentication Protocol）は、**「どの認証方式を使うか」を決める認証フレームワーク**です。[RFC 3748 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc3748.html) パスワード、証明書、ICカードなど、様々な認証方式を統一的に扱うための「共通の器」を提供します。データリンク層（[PPP](https://ja.wikipedia.org/wiki/Point-to-Point_Protocol) や [IEEE 802.1X](https://ja.wikipedia.org/wiki/IEEE_802.1X) など）上で直接動作します。

### RADIUSとは

RADIUS（Remote Authentication Dial In User Service）は、**認証・認可・課金（AAA）を行うための通信プロトコル**です。[RADIUSとは](https://www.soliton.co.jp/theme/column-radius.html) ネットワーク機器（アクセスポイントやスイッチなど）と認証サーバー間の通信に使用され、UDPベースで動作します。

### 主な違い

| 項目                     | EAP                                                  | RADIUS                                                 |
| ------------------------ | ---------------------------------------------------- | ------------------------------------------------------ |
| **役割**           | 認証方式を選び、認証情報をやり取りするフレームワーク | 認証・認可・課金をサーバーに問い合わせる通信プロトコル |
| **動作層**         | データリンク層（PPP、IEEE 802など）                  | ネットワーク層（UDPベース）                            |
| **主な機能**       | Request/Response/Success/Failureのメッセージ交換     | 認証問い合わせ、承認、利用記録の管理                   |
| **認証方式の決定** | EAPが認証方式をネゴシエーションする                  | RADIUS自体は認証方式を決めない                         |

### 両者の関係

EAPとRADIUSは対立するものではなく、**協力して動作**します。[RFC 3579](https://tex2e.github.io/rfc-translater/html/rfc3579.html) では、RADIUSがEAPメッセージを運ぶ仕組みが定義されています。

具体的な流れは以下の通りです。[【図解】RADIUSサーバの仕組みとは](https://www.cair-n.co.jp/blog/infra/p7155/)

1. クライアントと認証者（APなど）の間では、**EAP**で認証方式の交渉と認証情報のやり取りを行う
2. 認証者と認証サーバー（RADIUSサーバー）の間では、**RADIUS**でEAPメッセージを中継する
3. RADIUSサーバーが認証を処理し、結果を認証者へ返す
4. 認証者がクライアントへEAP-SuccessまたはEAP-Failureを通知する

たとえるなら、**EAPは「封筒の中身（認証情報）」を決めるもの**で、**RADIUSは「封筒を運ぶ郵便システム」** です。[EAPのはなし(1)](https://www.silex.jp/library/blog/20130918-1) 両者が組み合わさることで、企業のWi-FiやVPNなどの大規模な認証環境が実現されています。

## データリンク層での認証の必要性

データリンク層での認証が必要な理由は、**ネットワークの「入口」で接続を制御し、不正なアクセスを未然に防ぐため**です。[IEEE 802.1Xとは？ わかりやすく10分で解説](https://www.netattest.com/ieee-802-1x-2024_mkt_tst)

### 1. 通信が始まる前に接続を遮断できる

データリンク層（レイヤー2）はIP層（レイヤー3）より下位の層です。LANケーブルを挿したりWi-Fiの電波を受信したりした時点で認証を行うため、**IP通信が始まる前に不正な端末を遮断**できます。[IEEE 802.1X認証とは？基本的な認証の仕組みを解説](https://www.dlink-jp.com/column/ieee802.1x.html) 認証に成功するまで、スイッチやアクセスポイントのポートは閉じられた状態となり、通信が許可されません。[802.1X認証とは？その構成や認証方式を紹介](https://www.ruijie.co.jp/faq/what-is-8021x-authentication_442798201294815232.html)

### 2. 不正端末の侵入を防ぐ

認証なしでネットワークに接続できてしまうと、部外者や悪意のある端末が社内ネットワークに入り込み、機密情報の漏洩やデータの改竄といった深刻なリスクが生じます。[IEEE 802.1X認証とは？基本的な認証の仕組みを解説](https://www.dlink-jp.com/column/ieee802.1x.html) データリンク層で認証することで、**物理的にポートに接続しただけではネットワークに入れない**という「検問」の役割を果たします。[IEEE 802.1XとRADIUSを初心者向けに解説](https://note.com/tetiko/n/ndc77ee0b768c)

### 3. IPアドレスが不要で動作する

データリンク層での認証は、IPアドレスの割り当てやDHCPなどの上位層の仕組みに依存しません。[RFC 3748 日本語訳](https://tex2e.github.io/rfc-translater/html/rfc3748.html) つまり、端末がネットワークに接続した瞬間からすぐに認証を開始でき、IP通信を許可するかどうかを事前に判断できます。

### 4. ポート単位で細かく制御できる

[IEEE 802.1X](https://ja.wikipedia.org/wiki/IEEE_802.1X) は「ポートベースのネットワークアクセス制御」と呼ばれ、スイッチの各ポートや無線LANの各接続ごとに認証を要求します。[802.1Xはどのようにしてスイッチへのアクセスを保護するのか？](https://ja.szwecent.com/how-does-802-1x-secure-switch-access/) これにより、社内のLANポートに誰かが勝手にケーブルを挿しても、認証に失敗すればそのポートからの通信を完全に遮断できます。

## 総括

EAPの本質は、**「認証方式そのものではなく、どんな認証方式でも載せられる共通の枠組み（フレームワーク）」** です。

具体的には、以下の3点が核心となります。

1. **ネットワークの「入口」で動作する**：データリンク層（IP層より下位）で直接動作し、IPアドレスの割り当て前に認証を完了させます。これにより、不正な端末がネットワーク内部に入る前に遮断できます。
2. **認証方式の差異を吸収する**：パスワード、証明書、ICカードなど、異なる認証方式を統一的なメッセージ形式（Request/Response/Success/Failure）で扱えます。新しい認証方式が登場しても、枠組み自体を変えずに対応できます。
3. **バックエンド連携を可能にする**：認証装置（APやスイッチ）は認証処理を中継するだけで、実際の認証判断は [RADIUS](https://ja.wikipedia.org/wiki/RADIUS) などのバックエンドサーバーに委任できます。これにより、大規模なネットワークでも一元的な認証管理が実現できます。

つまり、EAPは **「誰が接続しようとしているか」をネットワーク接続の最も早い段階で確認し、正当な利用者のみに通信の許可を与えるための共通の土台**です。
