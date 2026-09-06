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


