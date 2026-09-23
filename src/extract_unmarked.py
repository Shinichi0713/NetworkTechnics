#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
〇のついていない問題を抽出するスクリプト

マークダウン形式の答案データから、選択肢に「〇」が付いていない
問題番号を抽出します。

【想定フォーマット】
### 1
ア〇
### 2
イ
### 3
ウ〇

【使い方】
python extract_unmarked.py input.md
python extract_unmarked.py input.md > output.txt
"""

import sys
import re


def extract_unmarked_problems(text):
    """
    マークダウンテキストから、〇のついていない問題を抽出する。

    形式:
    ### {問題番号}
    {選択肢}  ← この行に「〇」が含まれていない場合、その問題番号を抽出
    """
    results = []
    lines = text.splitlines()

    i = 0
    while i < len(lines):
        line = lines[i].strip()
        # 問題番号の行を検出: ### 数字
        match = re.match(r'^###\s*(\d+)$', line)
        if match:
            problem_num = match.group(1)
            # 次の行を確認
            if i + 1 < len(lines):
                next_line = lines[i + 1].strip()
                # 次の行に「〇」が含まれていない場合のみ記録
                if '〇' not in next_line:
                    results.append((problem_num, next_line))
            i += 2
        else:
            i += 1

    return results


def main():
    if len(sys.argv) < 2:
        print("使い方: python extract_unmarked.py <input.md>", file=sys.stderr)
        sys.exit(1)

    filepath = sys.argv[1]

    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            text = f.read()
    except FileNotFoundError:
        print(f"エラー: ファイル '{filepath}' が見つかりません。", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"エラー: {e}", file=sys.stderr)
        sys.exit(1)

    unmarked = extract_unmarked_problems(text)

    print("=== 〇のついていない問題 ===")
    for num, choice in unmarked:
        print(f"問題 {num}: {choice}")
    print(f"
合計: {len(unmarked)} 問")


if __name__ == '__main__':
    main()
