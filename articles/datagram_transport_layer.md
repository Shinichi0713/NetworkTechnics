---
title: "セキュリティ確保しながらリアルタイム性を確保するDTLS"
emoji: "😾"
type: "tech" # tech: 技術記事 / idea: アイデア記事
topics: ["networking", "real time", "security"]
published: true
---
通信にはトランスポートレイヤーという通信の順序や送達性を制御するレイヤーがあります。
このレイヤの中ではセキュリティやデータの送達性が重要な場合はTLSが使われ、リアルタイム性が重要な場合はUDPが使われていました。

ですがリアルタイム性も、セキュリティやデータの送達性ともに両立したい場面があります。
そんな場面で使われるトランスポートレイヤーの通信規則がDTLSと呼ばれるプロトコルです。

## DLTSとは
**DTLS（Datagram Transport Layer Security）** は、**UDPなどの「データグラム型・非信頼性」通信を暗号化するためのプロトコル**です。

### TLSとの違い

| 項目 | **TLS** | **DTLS** |
|---|---|---|
| **下位層** | TCP（信頼性あり、順序保証） | UDP（非信頼性、順序不保証） |
| **性質** | ストリーム指向 | データグラム（メッセージ）指向 |
| **保証** | 暗号化・改ざん防止・なりすまし防止 | 同じ（ただし順序・到達はUDPに依存） |
| **用途** | Webサイト（HTTPS）、メール | リアルタイム通信、IoT、VPN |

TLSはTCP上で動作し、パケットの順序や再送をTCPが保証します。しかし、**UDPではパケットが失われたり順番が入れ替わったりする**ため、TLSをそのまま使うと暗号化の状態が破綻します。DTLSは、この問題を解決するためにTLSを「データグラム向けに最小限の改変」を加えたプロトコルです。[RFC 6347](https://www.rfc-editor.org/rfc/rfc6347.html) [MDN Web Docs](https://developer.mozilla.org/en-US/docs/Glossary/DTLS)

### どんな場面で使われるか

**「遅延を抑えたい」「リアルタイム性が必要」** なため、TCP（TLS）ではなくUDPを使う場面で必要になります。

- **WebRTC**（ビデオ会議・音声通話）
- **IoT機器**（センサーデータの軽量・高速送信）
- **VPN**（OpenVPNなどはUDP上でDTLSを使用）
- **オンラインゲーム**（遅延に敏感な通信）

## 開発の背景
DTLSは、**2000年代初頭**に [Eric Rescorla](https://datatracker.ietf.org/doc/html/draft-rescorla-dtls-01) と [Nagendra Modadugu](https://crypto.stanford.edu/~nagendra/papers/dtls.pdf)（Stanford大学）によって開発されました。
当時、以下のような**遅延に敏感なアプリケーション**が普及し始めていました。

- **リアルタイムビデオ会議**
- **IP電話（VoIP）**
- **オンラインゲーム**（Quake、StarCraftなど）

これらのアプリケーションは、**TCPの再送による遅延を避けるためUDP（データグラム）を使用**していました。しかし、当時の標準的な暗号化プロトコルである **TLSはTCP（ストリーム）専用** であり、UDP上ではそのまま使えませんでした。

### 解決すべき問題

UDPでは以下の問題が発生するため、TLSをそのまま移植できませんでした。

| 問題 | TLSでの前提 | UDPでの実態 |
|---|---|---|
| **パケット喪失** | TCPが再送するので前提なし | パケットが途中で失われる |
| **順序入れ替え** | TCPが順序を保証する | 到着順が入れ替わる |
| **ストリーム前提** | 連続したデータ列として処理 | 個別のメッセージ（データグラム）単位 |

TLSは「データが順番に届く」ことを前提に暗号化状態を管理していましたが、UDPではこの前提が崩れるため、暗号化の同期が破綻してしまいました。

## 仕組み

DTLSは、UDPの「パケットが失われる・順番が入れ替わる」という課題に対し、**TLSに最小限の改変を加えることで、データグラム環境でも暗号化を成立させる**ように設計されました。

### 1. **シーケンス番号の明示化（順序入れ替えへの対処）**

TLSではTCPが順序を保証していたため、暗号化レコードに明示的な番号を持たせる必要がありませんでした。しかしUDPでは順序が入れ替わるため、DTLSは**各レコード（メッセージ）にシーケンス番号を付与**しました。これにより、到着順が入れ替わっても正しい順番で復号・再組み立てできます。

### 2. **ハンドシェイクの再設計（パケット喪失・再送への対処）**

TLSのハンドシェイクは「複数のメッセージを連続して交換する」前提でしたが、UDPでは途中でパケットが失われる可能性があります。DTLSでは以下の対策を取りました。

- **メッセージを個別のデータグラム単位で扱う**：大きなハンドシェイクメッセージは**フラグメント（分割）** して送信し、各断片にオフセット情報を付与
- **再送タイマーの導入**：TCPのような自動再送がないため、DTLS自身が**ハンドシェイク中の再送を管理**する
- **状態の保存**：失われたメッセージを検出し、必要な部分だけ再送する仕組み

### 3. **CookieによるDoS攻撃対策（UDPのコネクションレス性への対処）**

TCPでは3ウェイハンドシェイクが偽アドレス攻撃の防壁になりますが、UDPにはありません。DTLSでは **「HelloVerifyRequest」でCookieを発行**し、クライアントがそのCookieを返せることで、送信元IPアドレスの正当性を確認します。これにより、偽のアドレスからの大量ハンドシェイク（DoS）を防ぎます。

### 4. **レコード層の互換性維持**

TLSと同じ「暗号化・認証・整合性検証」というセキュリティ保証はそのまま維持しつつ、**下位層の「到達保証・順序保証がない」という性質だけを吸収**するように設計されました。つまり、暗号化の核心部分はTLSと共有し、UDP向けの「ラッパー」を追加した形です。

## 用途

DTLSは現在、主に以下の場面で使われています。

### 1. WebRTC（ビデオ会議・音声通話）

WebRTCはブラウザ間でリアルタイム通信を行う技術ですが、**すべての通信は暗号化が必須**です。DTLSはWebRTCのハンドシェイクで使われ、さらに**SRTP（Secure RTP）の鍵交換**にも利用されます。[Nabto](https://www.nabto.com/understanding-webrtc-security/) [Nat.io](https://nat.io/blog/dtls-srtp-webrtc-security) ZoomやGoogle Meet、Microsoft Teamsなどの多くのビデオ会議システムの裏側で動作しています。

### 2. IoT（モノのインターネット）

センサーデータや制御信号を**低低遅延・軽量**で送りたいIoT機器では、TCPよりUDPが選ばれることが多く、DTLSがその暗号化を担います。特にリソース制限のある機器向けに、DTLSの軽量実装が研究・開発されています。[IEEE Xplore](https://ieeexplore.ieee.org/abstract/document/7777209) [AMS-IoT](https://amsiot.com/blog/webrtc-for-iot-connection/)

### 3. VPN・通信回避ツール

OpenVPNはUDPモードでDTLSを使用しています。また、**Snowflake**（Tor Projectの検閲回避システム）では、通信を隠蔽するためにDTLSを利用し、検閲者によるフィンガープリンティング（プロトコル識別）対策も進められています。[PETS Symposium 2025](https://www.petsymposium.org/foci/2025/foci-2025-0006.pdf)

### 4. VoIP・SIP通信

IP電話やビデオ通話システム（Asterisk、Cisco、Twilioなど）でも、リアルタイムメディアストリームの暗号化にDTLS-SRTPが使われています。2024年〜2025年にも、これらの実装におけるDTLSハンドシェイクの脆弱性が複数報告されています。[Enable Security](https://www.enablesecurity.com/research/webrtc-hello-race-conditions-paper.pdf) [Enable Security Newsletter](https://www.enablesecurity.com/newsletter/2025-02-rtcsec-news.md)

## 総括

**DTLS（Datagram Transport Layer Security）** は、**UDPなどの非信頼性通信を暗号化するプロトコル**です。[RFC 6347](https://www.rfc-editor.org/rfc/rfc6347.html)

### 核心

| 項目 | 内容 |
|---|---|
| **何か** | TLSのデータグラム（UDP）版 |
| **なぜ作られたか** | リアルタイム通信（VoIP・ゲーム・ビデオ会議）はUDPを使うが、TLSはTCP専用でUDPでは暗号化が破綻したため |
| **課題の解決** | シーケンス番号で順序乱れに対応、ハンドシェイクをフラグメント化・再送制御で耐障害化、CookieでDoSを防止 |
| **どこで使われるか** | WebRTC（Zoom/Teamsなど）、IoT、VPN（OpenVPN）、VoIP |

### 一言でまとめると

> **DTLSは「UDPを使うリアルタイム通信でも、TLSと同等の暗号化・改ざん防止を提供する」ために開発されたプロトコルであり、現在はWebRTCやIoT・VPNの裏側で広く使われている。**
