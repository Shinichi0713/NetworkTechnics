# Hatena Studio

Markdown の技術記事を、はてなブログの「HTML 編集モード」へそのまま貼り付けられる HTML に変換するローカル Web アプリです。元になった Python スクリプト `md_to_hatena.py` の変換・後処理・関連記事選定を Rust に移植し、GUI とワンショット CLI の両方から使えるようにしています。

- 外部 API キー・有料サービスは不要です。記事の変換はすべてローカルで完結します。
- 関連記事は、自分のブログが公開している Atom フィードを読み取って選定します。
- JavaScript を無効にしても記事として読める HTML を出力します（プログレッシブエンハンスメント）。

設計の詳細と開発の経緯は [docs/design.md](docs/design.md) を参照してください。

## 必要なもの

- Rust 1.85 以上（edition 2024）
- Windows / macOS / Linux
- フィード取得のため、初回とキャッシュ更新時のみインターネット接続

## ビルドと起動

```powershell
cd tool/hatena_studio
cargo build --release
.\target\release\hatena_studio.exe --dir ..\..\network_specialist\past_prblems\R6
```

起動すると `http://127.0.0.1:8787/` を開きます（使用中の場合は 8787〜8806 の範囲で空きポートを探します）。ブラウザを自動で開かない場合は `--no-open` を付けます。

```powershell
.\target\release\hatena_studio.exe --dir <記事フォルダ> --port 8787 --no-open
```

`--dir` を省略するとカレントディレクトリを記事フォルダとして扱います。

## GUI の使い方

1. 左側の「参照」から Markdown ファイルを選ぶか、パスを直接入力します。
2. 必要に応じてタイトル、関連記事の件数・最低スコア・フィードの最大ページ数、キャッシュ有効期間を調整します。
3. 「変換してプレビュー」を押します（`Ctrl + Enter` でも実行）。
4. プレビューを確認し、「HTML をコピー」でクリップボードへコピーします。はてなブログの編集画面で「HTML 編集モード」に切り替えて貼り付けます。

### ファイル選択ダイアログの操作

「参照」を押すと、以下の方法で素早く目的のファイルへ移動できます（1 階層ずつ辿る必要はありません）。

| 操作 | 内容 |
| --- | --- |
| **パンくず** | 表示された階層をクリックすると、そのフォルダへ一発で移動します |
| **⌂ ルート** | 起動時に指定した記事フォルダへ戻ります（`Alt + Home`） |
| **↑ 上へ** | 親フォルダへ移動します（`Alt + ↑`） |
| **パス入力 + 移動** | 相対パス／絶対パスを直接入力してジャンプします（入力欄で `Enter` でも可） |
| **再帰検索** | ファイル名の一部を入力すると、現在のフォルダ以下を再帰的に検索します（最大 300 件。隠しフォルダや `target` / `node_modules` 等は除外） |
| **キーボード** | `↑` `↓` で行を移動、`Enter` で決定、`/` で検索欄へフォーカス、`Esc` で閉じます |

- 同じフォルダを再訪したときは、セッション中のキャッシュから瞬時に再表示されます（スクロール位置も復元）。
- ダイアログを開いている間は `Ctrl + Enter` による変換は実行されません。

主な機能:

- **プレビュー / HTML ソース** タブの切り替え
- 関連記事カードの **×** で、その記事を除外して再選定
- **キャッシュ更新** で Atom フィードを再取得
- **別タブで開く** で生成したプレビューを単独表示
- **保存先を開く** でエクスプローラー（macOS は Finder）を表示
- 変換ログ、統計（見出し数・表・解答・サイズなど）の表示

## ワンショット CLI

GUI を起動せず 1 回だけ変換する場合は `--once` を使います。

```powershell
.\target\debug\hatena_studio.exe `
  --dir ..\..\network_specialist\past_prblems\R6 `
  --once 'R6_pm2_解答解説.md'
```

既定の出力先は入力ファイルと同じフォルダの以下です。

- `<元ファイル名>_hatena.html` … はてなブログへ貼り付ける HTML
- `<元ファイル名>_preview.html` … ブラウザ確認用の完全な HTML

### オプション

| オプション | 説明 |
| --- | --- |
| `-d, --dir <PATH>` | 記事フォルダ（既定: カレントディレクトリ） |
| `-p, --port <PORT>` | 待ち受けポート（既定: 8787） |
| `--cache <PATH>` | フィードキャッシュの保存先 |
| `--js <PATH>` | `hatena_interactive.js` の場所 |
| `--no-open` | ブラウザを自動で開かない |
| `--once <PATH>` | 1 回だけ変換して終了（GUI を起動しない） |
| `-o, --output <PATH>` | 出力 HTML のパス |
| `--preview <PATH>` | プレビュー HTML のパス |
| `--refresh-related` | 関連記事フィードを再取得 |
| `--no-related` | 関連記事を挿入しない |
| `--no-toc` | 目次を生成しない |
| `--no-js` | インタラクション JS を埋め込まない |
| `--no-widget` | VXLAN クイズを挿入しない |
| `--keep-title` | 先頭の H1 を本文に残す |
| `--count <N>` | 関連記事の件数 |
| `--min-score <F>` | 関連記事の最低スコア |
| `--pages <N>` | フィードの最大ページ数 |
| `--blog-url <URL>` | ブログ URL |

## 変換の仕様

- 目次、見出し ID、表の横スクロールラッパー、解答コールアウト、コードブロックのコピーなどはインラインスタイルで出力します。はてなブログのテーマ CSS に依存しません。
- 数式（`$...$` / `$$...$$`）は、はてな公式が「すべての編集モードで使える」と明言している tex 記法 `[$ ... $]` に変換します。ディスプレイ数式は `\displaystyle` を付けた中央寄せブロックとして出力します。HTML 編集モードに貼り付けても数式が変換されます（ブログの MathJax の `$` / `$$` は HTML 編集モードでは走りません）。ローカルのプレビューでは MathJax 用に `$` / `$$` へ戻して表示します。**はてなへ貼り付けるのは `_hatena.html`（`_preview.html` ではありません）。**
- 強調（`**…**`）は、CommonMark の right-flanking 規則で解釈されない日本語の助詞前パターンも Python 版に合わせて補正します。
- 関連記事は TF-IDF コサイン類似度、ドメインキーワードの IDF 重み付き一致、タイトル一致加点で選びます。記事自身はタイトル一致で除外します。
- 関連記事の HTML はビルド時に静的埋め込みします。閲覧時に JavaScript でフィードを取りに行くことはありません。
- Atom フィードは JSON キャッシュ（既定 12 時間）を使います。取得に失敗した場合は古いキャッシュを使い、キャッシュもなければ関連セクションだけを省略します。
- `hatena_interactive.js` は、作業フォルダ → exe 隣接 → 同梱版の順で探索します。

## ファイル構成

```
tool/hatena_studio/
├── Cargo.toml
├── README.md
├── assets/
│   ├── index.html              # Web UI
│   ├── app.css                 # UI スタイル
│   ├── app.js                  # UI 操作・API 呼び出し
│   └── hatena_interactive.js   # 記事に埋め込む同梱 JS
└── src/
    ├── main.rs                 # axum サーバ、API、CLI、設定
    ├── markdown.rs             # Markdown → はてな HTML、目次、クイズ
    ├── feed.rs                 # Atom フィード取得・キャッシュ
    ├── related.rs              # 関連記事の選定
    └── text.rs                 # 正規化・類似度・HTML テキスト化
```

## 設定とキャッシュ

- 設定: `%APPDATA%\hatena_studio\settings.json`（APPDATA がない環境では記事フォルダ内の `.hatena_studio_settings.json`）
- フィードキャッシュ: 記事フォルダ内の `.hatena_feed_cache.json`

キャッシュは Python 版と互換の JSON 形式です。`.hatena_feed_cache.json` はサイズが大きいため、リポジトリにはコミットしません（`.gitignore` 済み）。

## テスト

```powershell
cargo test
cargo build
```

変換結果の回帰確認には、同じ Markdown を Python 版と Rust 版の両方で変換し、タグ数と本文テキストを比較すると便利です。
