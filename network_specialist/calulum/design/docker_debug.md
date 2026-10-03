
Dockerの中でさるレポジトリからコードをDLしてCmakeによりコンパイルするという処理を記載していました。
ですが、コンパイルでこけていましたが、通常のCmakeに対してDockerの場合どんな点が火種になるか知りたいと思いました。

本日はそんなコンパイルエラーについて語ります。

## DockerのCmakeのコンパイルエラー
Dockerコンテナ内でのCMakeコンパイルエラーには、いくつかの典型的な原因が考えられます。エラーメッセージの内容によって原因は異なりますが、よくあるケースを以下にまとめました。

### 1. 必要なパッケージやライブラリが不足している

Dockerイメージは最小構成であることが多いため、コンパイルに必要なツールやライブラリが入っていないことがあります。

- `build-essential`（gcc, g++, make など）
- `cmake` 自体のバージョンが古い
- 依存する開発用ライブラリ（`libssl-dev`, `libboost-all-dev` など）
- `ninja-build` や他のビルドツール

**対策例:**
```dockerfile
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    lib<依存ライブラリ>-dev
```

### 2. CMakeのバージョンが要件を満たしていない

`CMakeLists.txt` で `cmake_minimum_required(VERSION 3.x)` と指定されているのに、コンテナ内のCMakeが古い場合、エラーが発生します。

**対策例:**
```dockerfile
# 公式のバイナリを取得するなどして新しいバージョンを入れる
RUN pip install --upgrade cmake
```

### 3. ソースコードやファイルのパーミッション問題

Docker内でファイルをマウントした場合、コンテナ内のユーザーとホスト側のユーザーが異なり、書き込み権限がないことがあります。

**対策例:**
```bash
# コンテナ内で
chmod -R 777 /workspace
# または、適切なUID/GIDでコンテナを実行する
docker run --user $(id -u):$(id -g) ...
```

### 4. ビルドディレクトリの問題

CMakeは「out-of-source build」を推奨しますが、ソースディレクトリ内で直接ビルドしようとすると、キャッシュの競合などでエラーが起きることがあります。

**対策例:**
```bash
mkdir build && cd build
cmake ..
make
```

### 5. コンパイラのバージョンや互換性

C++の標準規格（C++17, C++20など）を要求するコードに対して、古いGCC/Clangではコンパイルエラーになります。

**対策例:**
```dockerfile
RUN apt-get install -y gcc-11 g++-11
ENV CC=gcc-11 CXX=g++-11
```

### 6. メモリ不足

コンテナに割り当てられているメモリが少なく、大規模なコンパイル（特にテンプレートを多用するC++コード）でOOM（Out of Memory）が発生することがあります。

**対策例:**
```bash
# Docker Desktopの設定などでメモリ上限を増やす
docker run -m 4g ...
```

### 7. ネットワーク関連（外部リソースの取得）

CMakeが外部のGitリポジトリやファイルを `FetchContent` / `ExternalProject` で取得しようとした際、コンテナ内にGitがない、またはネットワークが不通である場合に失敗します。

## 見分けるポイント

Dockerコンテナ内でのCMakeコンパイルエラーを見分けるポイントは、**エラーメッセージの「キーワード」** と **「どの段階で失敗したか」** にあります。

以下、エラーの症状別に見分けるポイントをまとめました。

### 1. 必要なパッケージ・ライブラリ不足

__見分けるポイント__
- `cmake` 実行時に `Could not find <パッケージ名>` と出る
- `package '<名前>' not found` という `pkg-config` 関連のエラー
- `fatal error: <ヘッダーファイル名>: No such file or directory`（コンパイル中）

__確認方法__
```bash
# どのパッケージが不足しているか調べる
cmake .. 2>&1 | grep -i "could not find\|not found"

# インストール済みパッケージの確認例
dpkg -l | grep libssl
```

### 2. CMakeのバージョン不足

__見分けるポイント__
- エラーの**最初の方**に `CMake <バージョン> or higher is required.` と明確に出る
- `cmake_minimum_required` で指定されたバージョンより古いCMakeが入っている

__確認方法__
```bash
cmake --version
# コンテナ内とホストで比較する
```

### 3. パーミッション問題

__見分けるポイント__
- `Permission denied` が出る
- `cmake` 自体は通るが、`make` 時にオブジェクトファイル（`.o`）の書き込みで失敗する
- ビルドディレクトリの作成に失敗する（`mkdir: cannot create directory`)

__確認方法__
```bash
# コンテナ内のユーザー確認
whoami
id

# ビルドディレクトリの書き込み権限確認
touch /workspace/test_write && rm /workspace/test_write
```

### 4. ビルドディレクトリ（in-source build）の問題

__見分けるポイント__
- `CMakeLists.txt` があるディレクトリで直接 `cmake .` として実行している
- `CMake Error: The current CMakeCache.txt directory ... is different than` というメッセージ
- 過去のキャッシュが残っている旨の警告

__確認方法__
```bash
# カレントディレクトリに CMakeCache.txt が存在するか確認
ls CMakeCache.txt 2>/dev/null && echo "in-source build している可能性あり"
```

### 5. コンパイラのバージョン・互換性問題

__見分けるポイント__
- `error: 'xxx' is not a member of 'std'`（C++17/20の機能が使えない）
- `error: expected ')' before 'const'` など、構文エラーっぽいが実は規格対応の問題
- `This file requires compiler and library support for the ISO C++ 2011 standard`

__確認方法__
```bash
# コンパイラのバージョン確認
gcc --version
g++ --version

# C++規格のサポート確認
g++ -std=c++17 -dM -E -x c++ /dev/null | grep __cplusplus
```

### 6. メモリ不足（OOM）

__見分けるポイント__
- エラーが**特定の大きなソースファイル**をコンパイルしている途中で発生
- `g++: internal compiler error: Killed (program cc1plus)` と出る
- `make[2]: *** [xxx.o] Error 4` や `Error 137`（SIGKILLを受けた印）

__確認方法__
```bash
# コンテナ内のメモリ確認
free -h
cat /proc/meminfo | grep MemTotal

# Dockerのメモリ制限確認（ホスト側で）
docker stats --no-stream <コンテナ名>
```

### 7. ネットワーク関連（外部リソース取得）

__見分けるポイント__
- `FetchContent` や `ExternalProject_Add` を使っている箇所で止まる
- `git clone` や `download` でタイムアウト・接続拒否
- `Could not resolve host: github.com` などのDNSエラー

__確認方法__
```bash
# コンテナ内からネットワーク疎通確認
ping -c 1 github.com
curl -I https://github.com

# gitが入っているか確認
which git
git --version
```

### 調査の優先順位（おすすめの手順）

エラーが発生したら、以下の順序でログを確認すると効率的です。

1. **エラーメッセージの最初の10行を読む**（CMakeバージョン不足やパッケージ不足はここに出る）
2. **失敗したコマンドを確認する**（`cmake` 段階か `make` 段階か）
3. **コンテナ内の環境を確認する**（`cmake --version`, `gcc --version`, `free -h`）
4. **ビルドディレクトリを作り直す**（`rm -rf build && mkdir build && cd build`）

## 総括

Docker内でCMakeコンパイルを安定させるためのコツは、以下の通りです。

__1. ベースイメージを「最小構成」にしない__

`ubuntu` や `debian` の slim イメージだけでなく、コンパイル前提のパッケージを最初から入れておきます。

```dockerfile
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/*
```

__2. 依存ライブラリは明示的にインストールする__

`CMakeLists.txt` の `find_package` で探しているものを、事前に `-dev` パッケージとして入れます。

__3. ビルドは必ず専用ディレクトリで行う__

ソースディレクトリを汚さず、キャッシュの混乱を防ぎます。

```bash
mkdir build && cd build
cmake ..
cmake --build .
```

__4. バージョンを最初に確認する仕組みを入れる__

Dockerfile の冒頭でツールのバージョンを出力し、想定外の環境にすぐ気づけるようにします。

```dockerfile
RUN cmake --version && gcc --version
```

__5. マウントする際はUID/GIDを合わせる__

パーミッションエラーを防ぐため、コンテナ内のユーザーとホスト側を一致させるか、書き込み可能なディレクトリを明示します。

```bash
docker run --user $(id -u):$(id -g) -v $(pwd):/workspace ...
```

__6. メモリ上限に余裕を持たせる__

大規模なC++コンパイルでは、Dockerのメモリ制限（デフォルト2GBなど）を超えることがあります。必要に応じて4GB以上確保します。

```bash
docker run -m 4g ...
```

__7. ネットワーク依存のビルドは分離する__

`FetchContent` などで外部リソースを取得する場合、ビルドステージと実行ステージを分け、必要なものは事前に揃えておきます。

__8. エラーログの「最初の10行」を最初に読む__

原因の9割はエラーログの先頭に書かれています。`CMake Error` の直後のメッセージを確認する習慣をつけます。



