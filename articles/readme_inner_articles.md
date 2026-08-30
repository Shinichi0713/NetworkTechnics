---
title: "readme"
emoji: "🥹"
type: "tech" # tech: 技術記事 / idea: アイデア記事
topics: ["networking"]
published: false
---
https://zenn.dev/puka/articles/pkpk3-zenn_emoji_list

## 画像連携の仕方

ZennのGitHub連携（`zenn-cli` 構成）において、Markdown内で参照している画像がZenn上で表示されない場合、主な原因は**画像ファイルの配置場所**または**Markdown内のパスの記述方法**にあります。

---

### 原因と解決手順

ZennのGitHub連携では、リポジトリ内の指定された**パブリックディレクトリ**に画像を配置する必要があります。

```
.
├── articles/            # 記事フォルダ
│   └── example.md
└── images/              # 画像は必ずここに配置
    └── my-image.png

```

#### 1. 画像ファイルを `images/` ディレクトリに配置する

リポジトリのルート（`articles/` や `books/` と同じ階層）に **`images/`** ディレクトリを作成し、そこに画像ファイルを保存してください。

* **NG例**: `articles/images/my-image.png` （`articles/` の中に画像フォルダを入れる）
* **OK例**: `images/my-image.png` （ルート直下の `images/` ディレクトリに入れる）

#### 2. Markdown内のパスをルート相対パス `/images/...` で記述する

Markdown内での参照は、先頭にスラッシュ `/` を付けた**ルート相対パス**で記述します。

```markdown
<!-- OK: 先頭に / をつける -->
![説明文](/images/my-image.png)

<!-- NG: 相対パスや相対指定は不可 -->
![説明文](./images/my-image.png)
![説明文](../images/my-image.png)
![説明文](images/my-image.png)

```

---

### チェックリスト（上記で解決しない場合）

1. **ファイル名・拡張子の表記チェック:**

* 大文字・小文字が一致しているか確認します（例: `.PNG` と `.png` の違いなど）。
* 日本語やスペースが含まれるファイル名は、URLエンコードやGitHub連携上の予期せぬトラブルの原因になるため、**英数字とハイフン・アンダースコア**のみに変更してください。

2. **ローカルでの表示確認（Zenn CLI）:**

* ローカル環境で `npx zenn preview` を実行し、ブラウザ上で画像が表示されるか確認します。
* ローカルで表示されない場合はパス記述ミス、ローカルで表示されるのにZenn本番で表示されない場合はGitHubへのPush漏れが原因です。

3. **GitHub リポジトリの同期確認:**

* 画像ファイル本体が実際にGitHubに `git push` されているか確認します（`.gitignore` 等で `images/` が除外されていないかチェック）。
