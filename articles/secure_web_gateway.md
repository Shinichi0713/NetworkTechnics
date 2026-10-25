---
title: "SWGによるセキュリティディフェンス"
emoji: "👮"
type: "tech" # tech: 技術記事 / idea: アイデア記事
topics: ["networking", "vpn", "secure"]
published: true
---
組織のネットワークのセキュリティを守る機器として有名なものはファイアウォールが有名です。
ですが現在のセキュリティは通信の外側のみを見るファイアウォールのみでは不足で、通信の中身や、動的な変化への対応が求められるようになりました。

そんな中で開発されたセキュリティ技術がSWG（Secure Web Gateway）です。

## SWGとは

SWGは、組織のユーザーがインターネットにアクセスする際の通信を中継・監視し、Web上の脅威から組織を保護するサイバーセキュリティソリューションです。[Gartner](https://www.gartner.com/en/information-technology/glossary/secure-web-gateway)によると、SWGは「ユーザーが開始したWeb/インターネット通信から不要なソフトウェアやマルウェアをフィルタリングし、企業および規制ポリシーの遵守を強制するソリューション」と定義されています。[Microsoft Security](https://www.microsoft.com/en-us/security/business/security-101/what-is-secure-web-gateway-swg)も、SWGを「すべてのインターネット通信をチェックポイントとして検査・フィルタリングし、脅威をブロックするサイバーセキュリティソリューション」と説明しています。

[Cloudflare](https://www.cloudflare.com/learning/access-management/what-is-a-secure-web-gateway/)は、SWGを「危険なコンテンツをブロックまたはフィルタリングし、データ漏洩を防ぐ仕組み」と要約しており、従業員のインターネット通信はすべてSWGを通過するのが基本構成です。

## 開発背景

SWG（Secure Web Gateway）の開発背景は、インターネットの普及とWeb技術の進化に伴う脅威の変化にあります。以下に主な背景をご説明いたします。

### 1. Webがマルウェア感染の主要経路となったこと

2000年代半ば頃、インターネット上の脅威の中心はメールからWebへと移行しました。[Network World (2007)](https://www.networkworld.com/article/814687/tech-primers-secure-web-gateways-slamming-the-door-on-malware.html)の記事では、「Webは新しいセキュリティ戦線となり、メールを超えてマルウェア感染の主要な源となった」と指摘されています。同記事では、Googleの調査で「クロールしたWebサイトの10個に1個が悪意のあるペイロードを含んでいた」こと、そしてGartnerが「75%の企業がその年の内にWebマルウェアに感染すると推定」していたことが紹介されています。このような状況下で、Web通信を専門的に検査・フィルタリングする仕組みとしてSWGが登場しました。

### 2. 従来のセキュリティ対策では不十分になったこと

従来のネットワークセキュリティは、主にポートやIPアドレスを基にしたファイアウォールや、メールのウイルス対策が中心でした。しかし、Web通信（HTTP/HTTPS）はポート80や443を使って正当なサービスと同じ経路を通るため、従来のパケットフィルタ型ファイアウォールだけでは「中身」を見極めることができませんでした。[Netskopeのホワイトペーパー](https://www.netskope.com/fr/wp-content/uploads/2024/06/Historical-and-Future-Roles-for-Firewall-and-Proxy-Gateways.pdf)では、1990年代のパケットフィルタは「TCPポート80からの接続が到着したことを認識できたが、リクエストの内容までは把握できなかった」と説明されており、Web通信の内容を深く検査する必要性が高まったことが背景にあります。

### 3. Web 2.0とインタラクティブWebアプリケーションの登場

2000年代後半、Web 2.0と呼ばれる動的でインタラクティブなWebアプリケーションが普及しました。これにより、従来の静的なWebページとは異なり、JavaScriptやAjax、SNS、動画共有などの技術が使われるようになり、新たなセキュリティリスクが生じました。[Network World (2007)](https://www.networkworld.com/article/817642/lan-wan-new-secure-web-gateways-eye-next-generation-web-application-traffic.html)では、「Web 2.0が提起するセキュリティ問題を管理するための新しいクラスのSecure Web Gateway」としてSWGが紹介されています。従来のプロキシサーバーやURLフィルタリングだけでは、こうした動的なWebアプリケーションの脅威に対応しきれなくなったのです。

### 4. プロキシ技術からの進化

SWGの技術的な前身は「プロキシゲートウェイ」です。[Netskopeのホワイトペーパー](https://www.netskope.com/fr/wp-content/uploads/2024/06/Historical-and-Future-Roles-for-Firewall-and-Proxy-Gateways.pdf)では、プロキシ技術が「Secure Web Gateway（SWG）とWeb Application Firewall（WAF）に分岐」した歴史が説明されています。プロキシは元々、Webアクセスの中継とキャッシュ、簡易的なフィルタリングを行うものでしたが、脅威の高度化に伴い、マルウェア検知、SSL検査、DLPなどの高度なセキュリティ機能を統合した「SWG」へと進化しました。

## 主な機能

SWGには以下のような中核的な機能が備わっています。

### URLフィルタリング
ユーザーがアクセスできるWebサイトをカテゴリやドメイン単位で制御します。[Cloudflare One docs](https://developers.cloudflare.com/cloudflare-one/traffic-controls/)では、URLおよびドメインフィルタリングを「ユーザーがアクセスできるWebサイトを制御する機能」として挙げています。[Palo Alto Networks](https://www.paloaltonetworks.com/cyberpedia/swg-use-cases)も、URLフィルタリングをSWGの主要なユースケースの一つとして紹介しています。

### マルウェア対策・脅威保護
ダウンロードされるファイルをリアルタイムでスキャンし、マルウェアやランサムウェア、フィッシングサイトなどから組織を守ります。[Fortinet](https://www.fortinet.com/resources/cyberglossary/secure-web-gateways)は、SWGを「マルウェアやフィッシングなどの脅威を防ぐためにインターネット通信を監視・制御するサイバーセキュリティソリューション」と説明しています。

### データ損失防止（DLP）
機密情報が組織の外部に送信されるのを検知し、防止します。[Palo Alto Networks](https://www.paloaltonetworks.com/cyberpedia/swg-use-cases)は、DLPをSWGの重要なユースケースとしており、機密データの流出を防ぐ機能を指しています。[Forcepoint](https://www.forcepoint.com/product/secure-web-gateway-swg)も、Web上のあらゆる場所でのデータとトラフィックの可視化・制御により、データ損失を防ぐと述べています。

### SSL/TLS通信の検査（可視化）
近年のWeb通信はほとんどが暗号化（HTTPS）されていますが、SWGはSSL/TLS暗号化通信を安全に復号・検査し、中に隠れた脅威を検出します。[Palo Alto Networks](https://www.paloaltonetworks.com/cyberpedia/swg-use-cases)は、このSSL/TLSインスペクションをSWGの主要機能として挙げています。

## なぜSWGが重要なのか

リモートワークの普及やクラウドサービスの増加により、従来の「社内ネットワークの境界」を守るだけのセキュリティでは不十分になりました。[Zscaler](https://www.zscaler.com/resources/security-terms-glossary/what-is-secure-web-gateway)は、SWGが「安全でないインターネット通信が組織のネットワークに到達するのを防ぎ、ユーザーやアプリケーションを悪意のあるWebサイトやマルウェアから保護する」と説明しています。

また、[Cisco Umbrella](https://umbrella.cisco.com/blog/sase-breakdown-secure-web-gateway)は、多くのITやSecOpsチームがSWGを「サイバーセキュリティスタックのバックボーン（基幹）」と位置づけていると指摘しており、GartnerのSASE（Secure Access Service Edge）フレームワークにおいてもSWGは重要な構成要素として含まれています。


## 導入形態の変化

従来のSWGはオンプレミス（社内に機器を設置する形）が主流でしたが、近年はクラウド型SWGが急速に普及しています。[Gartnerのレポート（Cisco Blogsによる紹介）](https://blogs.cisco.com/security/gartner-releases-report-on-the-transformation-of-the-swg-market)では、クラウドベースのSWGサービスの急速な成長が市場の破壊的な力になっていると述べられています。これにより、外出先や自宅から働くユーザーでも、インターネットに接続するだけで組織のセキュリティポリシーを適用できるようになりました。

## ファイアウォールとSWG
もともと存在していたネットワークの玄関セキュリティ係だったファイアウォールとSWGの立ち位置はどうなっているのでしょうか。

結論から言うとファイアウォールとSWGは競合関係ではなく、現代の企業セキュリティでは「両方を組み合わせて使う」のが主流です。ただし、役割と導入の広さには違いがあります。

### ファイアウォールとSWGの役割の違い

まず、両者は検査する対象の「レイヤー（階層）」が異なります。

[SentinelOne](https://www.sentinelone.com/ja/cybersecurity-101/cybersecurity/swg-vs-firewall/)の解説によれば、SWGは**アプリケーション層（レイヤー7）** でWebトラフィックをフィルタリングするのに対し、ファイアウォールは**ネットワーク境界を越えるすべてのトラフィック**（Web以外の通信も含む）を検査します。[Check Point](https://www.checkpoint.com/cyber-hub/network-security/what-is-firewall/secure-web-gateway-vs-firewall/)も、「SWGはWebトラフィックのみに焦点を当てる一方、ファイアウォールは保護されたネットワーク境界を越えるすべてのトラフィックを検査する」と明確に区別しています。

| 比較項目 | ファイアウォール | SWG |
|---|---|---|
| 主な検査対象 | 全トラフィック（ネットワーク層〜） | Webトラフィック（HTTP/HTTPS）のみ |
| 役割 | ネットワーク境界の守り | Web上の脅威とデータ漏洩の防止 |
| 代表的な機能 | パケットフィルタ、IPS、VPN | URLフィルタリング、DLP、SSL検査 |
| 導入の必須性 | ほぼすべての組織で必須 | クラウド・リモートワーク時代に急速に必須化 |

### 導入率ではファイアウォールが圧倒的

ファイアウォールは企業ネットワークの「入り口」に必ず設置される基盤的なセキュリティ機器であり、導入率ではファイアウォールが圧倒的に高いです。[TechTargetのホワイトペーパー（2025年7月）](https://wp.techtarget.itmedia.co.jp/contents/91113)では、「エンタープライズのネットワークセキュリティにおいて、依然として重要な存在になっているのがファイアウォールだ」と述べられており、現在のエンタープライズファイアウォールはゼロトラスト、SD-WAN、SASEなど多様なニーズに応えるよう進化していると説明されています。

### しかしSWGも急速に「必須」へ

一方、SWGはクラウド化・テレワークの普及により、従来の「社内ファイアウォールだけでは守れない」状況を補うものとして急速に普及しています。[NTTセキュリティ・ジャパン](https://jp.security.ntt/insights_resources/security_magazine/what-is-swg/)は、「テレワークやクラウド利用が当たり前になり、社内と社外を分ける『境界型防御』が限界を迎えている」と指摘し、どこからアクセスしてもWeb通信を安全にするSWGが注目されていると説明しています。[NTTコム](https://www.ntt.com/business/services/xmanaged/lp/column/swg.html)も、DXが加速する現代においてSWGが「企業のセキュリティを確保するために欠かせない存在となっている」と述べています。


## 総括

ファイアウォールは「玄関の通用口」で、誰が出入りするか（IPアドレスやポート）を確認します。しかし、現代の脅威はWebサイトやクラウドサービスを通じて中に入ってくるため、**通信の「中身」まで見ないと危険なものが見逃されます**。

SWGはその役割を担います。ユーザーがWebサイトにアクセスする際の通信を一度SWGに通し、以下を行います。

- 危険なサイトへの接続をブロック（URLフィルタリング）
- ダウンロードするファイルにマルウェアがないか検査
- 社内の機密情報が外部に送られていないか監視（DLP）
- 暗号化された通信の中身も安全に検査（SSL/TLS検査）

### なぜ必要なのか

テレワークやクラウドの普及で、社内と社外の境界が曖昧になりました。従来の「社内に設置したファイアウォールだけで守る」方式では、外出先や自宅から働くユーザーのWeb通信までは細かく守れません。SWGはクラウド上でも動作するため、**どこからアクセスしても同じセキュリティポリシーを適用できます**。

### 結論

ファイアウォールが「ネットワーク全体の守り」なら、SWGは **「Web通信を専門に守る守り」** です。現代のセキュリティでは、両者を組み合わせて使うのが標準です。
