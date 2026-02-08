## SSL / TLS

ハンドシェイクとレコードプロトコルから構成されている。

>レコードプロトコル  
>ハンドシェイク中に作成されたキーを使用してアプリケーション データをセキュリティで保護します。 
>レコード プロトコルは、アプリケーション データをセキュリティで保護し、その 整合性 と配信元を確認する役割を担います。
>つまり。上位プロトコルのメッセージを暗号化するためのプロトコル。
>例. HTTPSで通信するときはレコードプロトコルのヘッダ情報でカプセル化され、ペイロードに格納される。

>ハンドシェイクプロトコル  
>TLSセッションIDで使用する共通鍵、MACシークレットの生成の他、使用する暗号化アルゴリズムやMACアルゴリズムを折衝する。


```mermaid
sequenceDiagram
    participant C as クライアント (Browser)
    participant S as サーバー (Web Server)

    Note over C, S: [TCP 3-Way Handshake] TCP接続の確立

    rect rgb(240, 248, 255)
    Note right of C: 1. 交渉開始
    C->>S: Client Hello (TLSバージョン, 乱数, 暗号スイート候補)
    S->>C: Server Hello (決定したTLSバージョン, 乱数, 暗号スイート)
    end

    rect rgb(255, 245, 238)
    Note right of S: 2. 認証と鍵交換の準備
    S->>C: Certificate (サーバー証明書)
    S->>C: Server Key Exchange (必要に応じて鍵交換用パラメータ)
    S->>C: Server Hello Done
    end

    rect rgb(240, 255, 240)
    Note right of C: 3. 鍵の生成と検証
    Note over C: 証明書の検証 (有効期限・署名)
    C->>S: Client Key Exchange (プリマスタシークレットを送信)
    C->>S: Change Cipher Spec (これ以降の通信を暗号化する宣言)
    C->>S: Finished (検証用データの送信)
    end

    rect rgb(255, 240, 245)
    Note right of S: 4. 準備完了
    S->>C: Change Cipher Spec
    S->>C: Finished
    end

    Note over C, S: TLSセッション確立 (暗号化されたデータの送受信開始)
```

Client Hello / Server Hello: 「挨拶」です。利用可能な暗号の組み合わせ（暗号スイート）を出し合い、共通のルールを決めます。

Certificate: サーバーが自分の身分証明書を送ります。クライアントは、その証明書が信頼できる機関（認証局）から発行されたものかを確認します。

Client Key Exchange: ここが最も重要です。共通鍵の元となる「プリマスタシークレット」という秘密の値を共有します。
- RSA方式の場合：サーバーの公開鍵で暗号化して送ります。
- DH方式の場合：お互いのパラメータから計算で導き出します。

Change Cipher Spec / Finished: 「これから暗号化を始めるよ」という合図です。



