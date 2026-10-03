---
title: "Visual StudioでMSYS2コマンドを呼び出す"
emoji: "😾"
type: "tech" # tech: 技術記事 / idea: アイデア記事
topics: ["native application", "Linux/Unix"]
published: true
---
著者は基本はWindowsユーザです。ですが、C++でアプリ開発をしているとLinux/Unix前提のパッケージを使わないといけないという場面があります。
こんな時に使うのがMSYS2です。
MSYS2を使うと、Linux/Unix系のソースコードやビルド手順、オープンソースライブラリを、Windowsネイティブのアプリケーション（.exe）として簡単にビルドできます。

- pacman でFLTK、OpenSSL、Boostなど、Unix系のライブラリをコマンド1つでインストール
- configure や make、g++ といったLinux系のビルドフローをWindows上でそのまま再現
- 依存関係も自動的に解決される

この意味では、「Linux系の開発環境を前提としたパッケージを、Windowsアプリに組み込む」という用途にMSYS2は非常に適しています。
今日はそんなMSYS2について説明していきます。

## MSYS2とは
**MSYS2**は、**Windows上でUnix/Linux風の開発環境を構築し、Windows用のネイティブアプリケーションをビルドするためのプラットフォーム**です。

### 主な構成要素

| 要素 | 役割 |
|------|------|
| **Unix風シェル（bash等）** | Windows上でLinuxライクなコマンドライン操作ができる |
| **pacman** | Arch Linux由来のパッケージマネージャー。コンパイラやライブラリをコマンド1つでインストールできる |
| **MinGW-w64** | WindowsネイティブのEXE/DLLを生成するクロスコンパイラ（gcc/g++） |
| **リポジトリ** | 数千のパッケージ（FLTK、OpenSSL、Boost等）が整備されている |

### MSYS2でできる主なこと

- **Windowsアプリをコンパイル**：MinGW-w64の `g++` で、Windows上で動くEXEを作れる。
- **ライブラリを簡単に入れる**：`pacman -S mingw-w64-x86_64-fltk` のように、依存関係を自動解決してインストールできる。
- **シェルスクリプトやMakefileを使う**：Linuxで使えるビルド手順をWindowsでそのまま再現できる。
- **複数環境の管理**：MINGW64（64bit Windowsアプリ）、UCRT64、CLANG64など、用途に応じた環境を分けられる。

### なぜWindowsに入れるのか

Windowsには標準で `gcc` や `make`、Unix風の開発ツールが付属していません。Visual Studioは強力ですが、オープンソースのライブラリ（FLTK等）を使う場合、**Linux/Unix系のビルド手順（configure、make等）をWindows上で再現する必要**があります。MSYS2はその橋渡しをします。

### 一言でまとめると

MSYS2は、**「Windows上でLinux風の開発ができて、かつWindows用のプログラムを作れる」** 環境です。  
パッケージ管理（pacman）とMinGWコンパイラがセットになっており、オープンソースのC/C++ライブラリを手軽に使いたいWindows開発者に広く使われています。

## MSYS2で出来ること

MSYS2がないと、**「Windows上でLinux/Unix系の開発手順をそのまま使って、オープンソースのC++ライブラリをビルドする」** ことができません。

具体的には、以下の3つができなくなります。

### 1. `g++`（MinGW版）をコマンド1つで入れられない

Windows標準には `gcc` や `g++` というコンパイラが入っていません。Visual Studioには**MSVC**（Microsoftのコンパイラ）しかありません。

MSYS2がない場合：
- MinGW-w64を**手動でダウンロード・展開・PATH設定**する必要がある
- バージョン管理も自分で行う必要がある

MSYS2がある場合：
- `pacman -S mingw-w64-x86_64-gcc` で完了

### 2. `fltk-config` や `configure`、 `make` が使えない

FLTKなどのオープンソースライブラリは、本来**Linux/Unixのビルド手順**（`configure` → `make` → `make install`）を前提に作られています。

MSYS2がない場合：
- Windows用のバイナリを探してくるか、Visual Studio用のプロジェクトファイルを手動で作る必要がある
- 依存関係（依存ライブラリ）も**手動で探して配置**する必要がある

MSYS2がある場合：
- `pacman -S mingw-w64-x86_64-fltk` で、FLTK本体と依存関係が**自動的に解決**され、`fltk-config` も使える


### 3. Unix風のシェルスクリプト（`.sh`）をそのまま実行できない

先ほどの `build.sh` のようなスクリプトは、**bash**というシェルで動きます。WindowsのコマンドプロンプトやPowerShellではそのまま動きません。

MSYS2がない場合：
- WSLを入れる、Cygwinを入れる、Git Bashを使う、PowerShellに書き換えるなど、別の対応が必要

MSYS2がある場合：
- `bash build.sh` でそのまま実行できる

### なぜ「Visual Studioだけ」では足りないのか

Visual Studioは**MSVC**というコンパイラを使います。先ほどのコードは **MinGW版のFLTK**（`pacman` で入れたもの）とリンクする前提で書かれています。

MSVCとMinGWは**バイナリ互換がない**ため、MinGWでビルドしたFLTKのライブラリを、Visual Studio（MSVC）にそのままリンクすることは**できません**。

つまり、MSYS2なしで先ほどのコードを動かすには、**FLTKをVisual Studio用に自分でビルドし直す**か、**vcpkg**など別の方法を使う必要があります。

### 正直なところ：MSYS2が「絶対に必要」というわけではない

MSYS2がなくても、以下の方法では同じことができます。

- **vcpkg** を使ってVisual Studio（MSVC）用のFLTKを入れる
- **Visual Studioのプロジェクトウィザード**でFLTKを手動リンクする
- **WSL**（Windows Subsystem for Linux）でLinux環境を使う

ただし、これらは**MSYS2の「pacmanで一発インストール → bashでビルド」という手軽さには及びません。**


## Visual Studioで呼び出す

Visual StudioからMSYS2のコマンドを呼び出すことが出来ます。

### 方法1：Visual Studio 2022の統合ターミナルをMSYS2にする（最も簡単・推奨）

Visual Studio 2022には**統合ターミナル**があり、既定のシェルをMSYS2 MINGW64に変更できます。

**設定手順**
1. Visual Studioで「ツール」→「オプション」→「環境」→「ターミナル」
2. シェルのパスに以下を設定

```
C:\msys64\msys2_shell.cmd
```

3. 引数に以下を設定

```
-defterm -here -no-start -mingw64
```

4. Visual Studio内で `Ctrl + @`（またはメニューの「表示」→「ターミナル」）でターミナルを開くと、**MSYS2 MINGW64が起動**します。

これで、Visual Studioのエディタでコードを編集し、統合ターミナルから `g++` や `make` を実行できます。

### 方法2：外部ツールとして登録し、ショートカットでビルド

1. 「ツール」→「外部ツール」→「追加」
2. 以下を設定

| 項目 | 値 |
|------|-----|
| タイトル | MSYS2 Build |
| コマンド | `C:\msys64\usr\bin\bash.exe` |
| 引数 | `-lc "cd /d/PycharmProjects/statistics/calc_example/math_practice && g++ -std=c++11 -O2 math_practice_fancy.cpp -o math_practice_fancy $(fltk-config --cxxflags --ldflags)"` |
| 初期ディレクトリ | `$(ProjectDir)` |

3. 「ツール」メニューから実行、またはショートカットキーを割り当てる。

### 方法3：Makefileプロジェクト（NMake）として作成

Visual Studioには**Makefileプロジェクト**のテンプレートがあり、ビルドコマンドを自由に指定できます。

1. 「新しいプロジェクト」→「Makefileプロジェクト」（または空のプロジェクトのプロパティで「NMake」を選択）
2. 「ビルドコマンドライン」に以下を設定

```
C:\msys64\usr\bin\bash.exe -lc "cd /d/... && g++ -std=c++11 -O2 math_practice_fancy.cpp -o math_practice_fancy $(fltk-config --cxxflags --ldflags)"
```

3. F7（ビルド）でMSYS2のコンパイラが呼び出されます。

**注意**：この構成ではVisual Studioの**IntelliSenseやデバッガはMSVC前提**のため、MinGWのヘッダーを正しく解析できない場合があります。エディタとしての快適さは低下します。

### 方法4：CMakeプロジェクト + MinGWツールチェーン

CMakeを使う場合、`CMakeSettings.json` または `CMakePresets.json` でMinGWを指定できます。

```json
{
  "name": "mingw64",
  "generator": "MinGW Makefiles",
  "cacheVariables": {
    "CMAKE_C_COMPILER": "C:/msys64/mingw64/bin/gcc.exe",
    "CMAKE_CXX_COMPILER": "C:/msys64/mingw64/bin/g++.exe",
    "CMAKE_MAKE_PROGRAM": "C:/msys64/mingw64/bin/mingw32-make.exe"
  }
}
```

ただし、Visual StudioのCMake統合はMSVCを第一に想定しており、**デバッガやIntelliSenseが完全には機能しない**ことがあります。


### 現実的なおすすめ

| 方法 | 快適さ | 備考 |
|------|--------|------|
| **統合ターミナルをMSYS2に** | ◎ | エディタ＋手動ビルド。最もトラブルが少ない |
| **外部ツール登録** | ○ | ショートカットでワンクリックビルド |
| **Makefileプロジェクト** | △ | F7ビルドはできるがIntelliSenseが弱い |
| **CMake + MinGW** | △ | 設定が複雑、機能制限あり |

**結論**：Visual Studioを「エディタ」として使い、ビルドは**統合ターミナルからMSYS2 MINGW64で行う**のが最も実用的です。MSVCとMinGWはバイナリ互換がないため、「Visual Studioの標準ビルドでMSYS2のライブラリをリンクする」ことはできません。

