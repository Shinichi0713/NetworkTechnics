
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


