---
title: "拡張認証プロトコルEAP"
emoji: "❣"
type: "tech" # tech: 技術記事 / idea: アイデア記事
topics: ["reinforce-learning", "AI", "RL"]
published: true
---

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

### まとめ
EAPが提供するのは「特定の認証アルゴリズム」ではなく、「どんな認証方式でも載せられる共通のレール」と「そのレール上で通信するための手順・メッセージ形式」です。これにより、新しい認証方式が登場しても、既存のネットワーク機器を変更せずに導入できる柔軟性が生まれます。



