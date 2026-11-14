#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
md_to_hatena.py
Markdown 記事を「はてなブログ」の HTML 編集モード向けに変換するスクリプト。

特徴
----
* すべてインラインスタイルで装飾するため、ブログテーマの CSS に左右されない
* 記事タイトル（先頭の # 見出し）は本文から除去（はてなのタイトル欄と重複しない）
* 「解答：」「採点講評」の段落を自動でコールアウト（強調ボックス）に変換
* 表を横スクロール可能なラッパーで囲み、スマホでも崩れない
* h2 / h3 から目次（TOC）を自動生成
* ブラウザで確認できるプレビュー HTML も同時出力
* 読了バー、解答クイズ、コードコピーなどの軽いインタラクションを付与
  （hatena_interactive.js を記事末尾にインライン展開）
* ブログの Atom フィードから過去記事を取得し、本文との類似度で
  「関連記事」セクションを自動生成（結果は JSON にキャッシュ）

使い方
------
    python md_to_hatena.py R6_pm2_解答解説.md
    python md_to_hatena.py input.md -o article.html --preview preview.html
    python md_to_hatena.py input.md --no-toc --keep-title
    python md_to_hatena.py input.md --no-js          # 静的な記事にしたい場合
    python md_to_hatena.py input.md --no-related     # 関連記事を付けない
    python md_to_hatena.py input.md --refresh-related  # フィードを再取得

インタラクションを無効にしたい場合は --no-js、VXLAN ミニクイズだけ
外したい場合は --no-widget を指定します。関連記事の取得に失敗しても、
キャッシュがあればそれを使い、なければ関連セクションだけを省いて
変換を続行します。

必要なもの
----------
    pip install markdown beautifulsoup4
"""

from __future__ import annotations

import argparse
import html
import json
import math
import re
import sys
import time
import unicodedata
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path
from string import Template
from urllib.error import URLError
from urllib.request import Request, urlopen

try:
    import markdown
except ImportError:  # pragma: no cover
    sys.exit("markdown が必要です: pip install markdown")

try:
    from bs4 import BeautifulSoup, NavigableString, Tag
except ImportError:  # pragma: no cover
    sys.exit("beautifulsoup4 が必要です: pip install beautifulsoup4")


# ---------------------------------------------------------------------------
# デザイントークン（インライン CSS）
# ---------------------------------------------------------------------------

FONT = (
    "'Helvetica Neue',Arial,'Hiragino Kaku Gothic ProN','Hiragino Sans',"
    "'BIZ UDPGothic',Meiryo,sans-serif"
)
MONO = "'SFMono-Regular',Consolas,'Liberation Mono',Menlo,monospace"

BASE = (
    f"font-family:{FONT};"
    "font-size:16px;line-height:1.9;color:#1f2937;letter-spacing:.01em;"
    "overflow-wrap:break-word;word-wrap:break-word;"
)

H1 = (
    "margin:0 0 28px;padding:0 0 18px;border-bottom:3px solid #1e3a8a;"
    "color:#0f172a;font-size:27px;line-height:1.45;font-weight:700;"
    "letter-spacing:.01em;scroll-margin-top:80px;"
)

H2 = (
    "margin:48px 0 22px;padding:12px 18px;border-radius:8px;"
    "background-color:#1e3a8a;"
    "background-image:linear-gradient(135deg,#1e3a8a 0%,#2563eb 100%);"
    "color:#ffffff;font-size:22px;line-height:1.5;font-weight:700;"
    "letter-spacing:.02em;box-shadow:0 2px 6px rgba(30,58,138,.18);"
    "scroll-margin-top:80px;"
)

H3 = (
    "margin:40px 0 16px;padding:0 0 7px;border-bottom:2px solid #bfdbfe;"
    "color:#1e3a8a;font-size:19px;line-height:1.55;font-weight:700;"
    "scroll-margin-top:80px;"
)

H3_WARN = (
    "margin:40px 0 16px;padding:11px 15px;border:1px solid #fecaca;"
    "border-left:6px solid #ef4444;border-radius:0 8px 8px 0;"
    "background-color:#fef2f2;color:#991b1b;font-size:19px;line-height:1.55;"
    "font-weight:700;scroll-margin-top:80px;"
)

H4 = (
    "margin:28px 0 12px;padding:2px 0 2px 11px;border-left:5px solid #3b82f6;"
    "color:#0f172a;font-size:16.5px;line-height:1.6;font-weight:700;"
    "scroll-margin-top:80px;"
)

H5 = (
    "margin:24px 0 10px;padding:0;color:#334155;font-size:15.5px;"
    "line-height:1.6;font-weight:700;"
)

P = (
    "margin:0 0 1.15em;font-size:inherit;line-height:inherit;color:inherit;"
)

ANSWER = (
    "margin:16px 0 20px;padding:11px 16px;border:1px solid #a7f3d0;"
    "border-left:5px solid #10b981;border-radius:0 8px 8px 0;"
    "background-color:#ecfdf5;color:#065f46;font-size:15.5px;"
    "line-height:1.8;font-weight:600;"
)

COMMENT = (
    "margin:18px 0;padding:10px 14px;border-left:5px solid #f59e0b;"
    "border-radius:0 6px 6px 0;background-color:#fffbeb;color:#78350f;"
    "font-size:14px;line-height:1.8;"
)

LABEL = (
    "margin:22px 0 10px;padding:7px 12px;border-left:4px solid #60a5fa;"
    "border-radius:0 4px 4px 0;background-color:#eff6ff;color:#1e40af;"
    "font-size:14.5px;line-height:1.7;"
)

NOTE = (
    "margin:20px 0;padding:14px 18px;border-left:4px solid #94a3b8;"
    "border-radius:0 8px 8px 0;background-color:#f8fafc;color:#475569;"
    "font-size:14.5px;line-height:1.85;"
)

WARN = (
    "margin:20px 0;padding:14px 18px;border-left:4px solid #ef4444;"
    "border-radius:0 8px 8px 0;background-color:#fef2f2;color:#991b1b;"
    "font-size:14.5px;line-height:1.85;"
)

META = (
    "margin:0 0 20px;padding:12px 16px;border:1px solid #e2e8f0;"
    "border-radius:8px;background-color:#f8fafc;font-size:13.5px;"
    "line-height:1.75;color:#475569;"
)

TABLE_WRAP = (
    "margin:18px 0 24px;overflow-x:auto;-webkit-overflow-scrolling:touch;"
    "border:1px solid #e2e8f0;border-radius:8px;"
)

TABLE = (
    "width:100%;border-collapse:collapse;border-spacing:0;"
    "background-color:#ffffff;font-size:14px;line-height:1.65;"
)

TH = (
    "padding:10px 12px;border:1px solid #cbd5e1;background-color:#1e3a8a;"
    "color:#ffffff;font-weight:700;font-size:13.5px;text-align:left;"
    "white-space:nowrap;vertical-align:middle;"
)

TD = (
    "padding:9px 12px;border:1px solid #e2e8f0;color:#1f2937;"
    "font-size:inherit;line-height:inherit;vertical-align:top;"
)

TABLE_HINT = (
    "margin:0 0 6px;font-size:12px;line-height:1.5;color:#94a3b8;"
    "text-align:right;"
)

PRE = (
    "margin:18px 0 24px;padding:16px 18px;border-radius:8px;"
    "background-color:#0f172a;color:#e2e8f0;"
    f"font-family:{MONO};font-size:13px;line-height:1.7;"
    "overflow-x:auto;-webkit-overflow-scrolling:touch;tab-size:4;"
)

CODE = (
    "padding:2px 6px;border-radius:4px;background-color:#eef2ff;"
    "color:#3730a3;"
    f"font-family:{MONO};font-size:.88em;"
)

LIST = (
    "margin:.8em 0 1.1em;padding-left:1.6em;"
    "font-size:inherit;line-height:inherit;color:inherit;"
)
LIST_NESTED = (
    "margin:.35em 0 .5em;padding-left:1.3em;"
    "font-size:inherit;line-height:inherit;color:inherit;"
)
LI = (
    "margin:.35em 0;font-size:inherit;line-height:inherit;color:inherit;"
)

HR = "margin:40px 0;border:0;height:1px;background-color:#e2e8f0;"

LINK = (
    "color:#1d4ed8;text-decoration:underline;text-underline-offset:2px;"
)

IMG = "max-width:100%;height:auto;border-radius:8px;"

TOC = (
    "margin:0 0 32px;padding:18px 22px;border:1px solid #dbeafe;"
    "border-radius:10px;background-color:#f8fafc;font-size:14px;"
    "line-height:1.75;"
)

TOC_TITLE = (
    "margin:0 0 10px;padding-left:10px;border-left:4px solid #2563eb;"
    "font-size:15px;font-weight:700;color:#1e3a8a;letter-spacing:.04em;"
)

TOC_OL = "margin:0;padding-left:1.4em;list-style:decimal;"
TOC_UL = "margin:4px 0 8px;padding-left:1.15em;list-style:circle;"
TOC_LI = "margin:4px 0;"
TOC_LINK = (
    "color:#1e40af;text-decoration:none;border-bottom:1px dotted #93c5fd;"
)

RELATED_WRAP = "margin:0;padding:0;"
RELATED_INTRO = (
    "margin:0 0 14px;font-size:13px;line-height:1.8;color:#475569;"
)
RELATED_CARD = (
    "display:block;margin:0 0 12px;padding:14px 16px;"
    "border:1px solid #dbeafe;border-left:5px solid #2563eb;"
    "border-radius:0 10px 10px 0;background-color:#ffffff;"
    "box-shadow:0 1px 2px rgba(15,23,42,.04);"
    "text-decoration:none;color:inherit;"
    "transition:transform .15s ease,box-shadow .15s ease,"
    "background-color .15s ease;"
)
RELATED_TOP = (
    "display:flex;flex-wrap:wrap;align-items:center;gap:8px;margin:0;"
)
RELATED_BADGE = (
    "display:inline-block;padding:2px 9px;border-radius:999px;"
    "background-color:#eff6ff;color:#1e40af;font-size:10.5px;"
    "font-weight:700;line-height:1.6;letter-spacing:.06em;"
)
RELATED_DATE = (
    "font-size:11px;line-height:1.6;color:#64748b;font-weight:600;"
    "letter-spacing:.02em;"
)
RELATED_TITLE = (
    "margin:7px 0 0;font-size:15.5px;line-height:1.55;font-weight:700;"
    "color:#1e3a8a;"
)
RELATED_ARROW = "color:#2563eb;font-weight:700;"
RELATED_SUMMARY = (
    "margin:5px 0 0;font-size:12.5px;line-height:1.7;color:#475569;"
)
RELATED_NOTE = (
    "margin:12px 0 0;font-size:11.5px;line-height:1.65;color:#94a3b8;"
)

# ---------------------------------------------------------------------------
# 埋め込みウィジェット
# ---------------------------------------------------------------------------

PUZZLE_CHIP = (
    "appearance:none;border:1px solid #93c5fd;border-radius:999px;"
    "padding:8px 14px;background-color:#ffffff;color:#1e3a8a;"
    "font-family:inherit;font-size:12.5px;font-weight:700;line-height:1.4;"
    "cursor:pointer;transition:transform .12s ease,background .12s ease,"
    "opacity .12s ease;"
)

PUZZLE_SUB_BUTTON = (
    "appearance:none;border:1px solid #cbd5e1;border-radius:8px;"
    "padding:5px 12px;background-color:#ffffff;color:#475569;"
    "font-family:inherit;font-size:11.5px;font-weight:700;line-height:1.5;"
    "cursor:pointer;transition:background .15s ease,color .15s ease;"
)

PUZZLE_SLOT = (
    "display:inline-flex;align-items:center;justify-content:center;"
    "min-width:92px;min-height:36px;padding:6px 10px;border-radius:8px;"
    "border:1px dashed #93c5fd;background-color:#f8fafc;color:#1e3a8a;"
    "font-size:12px;font-weight:700;text-align:center;line-height:1.35;"
)

PUZZLE_ARROW = "color:#93c5fd;font-size:12px;font-weight:700;"

PUZZLE_SLOTS = "".join(
    f'<span style="{PUZZLE_SLOT}">？</span>'
    + (
        f'<span aria-hidden="true" style="{PUZZLE_ARROW}">→</span>'
        if index < 3
        else ""
    )
    for index in range(4)
)

VXLAN_PUZZLE_HTML = f"""
<div data-widget="vxlan-puzzle" style="margin:26px 0 30px;padding:18px 20px;border:1px solid #bfdbfe;border-radius:12px;background-color:#f8fbff;background-image:linear-gradient(180deg,#ffffff 0%,#eff6ff 100%);">
  <div style="display:flex;flex-wrap:wrap;align-items:center;gap:8px;margin:0 0 8px;">
    <span style="display:inline-block;padding:3px 9px;border-radius:999px;background-color:#1e3a8a;color:#ffffff;font-size:10.5px;font-weight:700;letter-spacing:.12em;">MINI QUIZ</span>
    <span style="font-size:14px;font-weight:700;color:#1e3a8a;">VXLANカプセル化の順番</span>
  </div>
  <p style="margin:0 0 12px;font-size:13px;line-height:1.75;color:#334155;">外側（トンネルの入口）から内側へ、4つの層を順番にクリックしてください。</p>
  <div style="display:flex;justify-content:space-between;gap:12px;margin:0 0 6px;font-size:10.5px;font-weight:700;color:#64748b;">
    <span>外側（トンネル入口）</span><span>内側（元のフレーム）</span>
  </div>
  <div data-role="slots" aria-label="選択した順番" style="display:flex;flex-wrap:wrap;align-items:center;gap:6px;margin:0 0 14px;min-height:38px;">{PUZZLE_SLOTS}</div>
  <div data-role="chips" style="display:flex;flex-wrap:wrap;gap:8px;margin:0 0 12px;">
    <button type="button" data-order="3" style="{PUZZLE_CHIP}">VXLANヘッダー</button>
    <button type="button" data-order="4" style="{PUZZLE_CHIP}">イーサネットフレーム</button>
    <button type="button" data-order="1" style="{PUZZLE_CHIP}">IPv4ヘッダー</button>
    <button type="button" data-order="2" style="{PUZZLE_CHIP}">UDPヘッダー</button>
  </div>
  <div style="display:flex;flex-wrap:wrap;align-items:center;gap:8px;">
    <button type="button" data-role="reset" style="{PUZZLE_SUB_BUTTON}">やり直す</button>
    <button type="button" data-role="hint" style="{PUZZLE_SUB_BUTTON}">ヒント</button>
    <span data-role="hint-text" style="display:none;font-size:11.5px;line-height:1.65;color:#475569;">いちばん外側は「宛先 IP アドレス」を運ぶヘッダー。いちばん内側は、もともと送りたかったフレームです。</span>
  </div>
  <div data-role="status" role="status" aria-live="polite" style="margin:10px 0 0;min-height:1.65em;font-size:12.5px;font-weight:700;line-height:1.65;"></div>
  <noscript><p style="margin:10px 0 0;font-size:11.5px;line-height:1.6;color:#64748b;">このミニクイズは JavaScript を有効にすると遊べます。</p></noscript>
</div>
"""


# ---------------------------------------------------------------------------
# 変換処理
# ---------------------------------------------------------------------------

def read_markdown(path: Path) -> str:
    """BOM 付き UTF-8 も読めるようにする。"""
    return path.read_text(encoding="utf-8-sig")


def split_title(md_text: str) -> tuple[str, str | None]:
    """先頭の H1 を記事タイトルとして取り出し、本文からは除去する。"""
    lines = md_text.splitlines()
    in_fence = False

    for i, line in enumerate(lines):
        if re.match(r"^\s*(```|~~~)", line):
            in_fence = not in_fence
            continue
        if not in_fence and re.match(r"^#\s+\S", line):
            title = re.sub(r"^#\s+", "", line).strip()
            del lines[i]
            return "\n".join(lines), title

    return md_text, None


def markdown_to_html(md_text: str) -> str:
    return markdown.markdown(
        md_text,
        extensions=["extra", "sane_lists"],
        output_format="html5",
    )


def normalize_headings(soup: BeautifulSoup) -> None:
    """本文の見出しレベルをはてなブログ向けに整える。

    記事タイトルを除去した結果、本文には「問1」などの H1 が残る。
    最初の H1 以降を 1 段階下げて H2 を起点にし、それより前にある
    「0. 試験概要」などの前書きセクションは元のレベルを維持する。
    """
    seen_h1 = False
    for h in soup.find_all(["h1", "h2", "h3", "h4", "h5", "h6"]):
        # 本文に残した記事タイトル自身は h1 のままにする
        if h.get("data-role") == "article-title":
            continue
        if h.name == "h1":
            seen_h1 = True
        if seen_h1:
            level = min(int(h.name[1]) + 1, 6)
            h.name = f"h{level}"

        # 付録は本文の下位セクションではなく、独立した見出しとして扱う
        if h.name == "h3" and h.get_text("", strip=True).startswith(
            ("付録", "Appendix")
        ):
            h.name = "h2"


def wrap_meta(soup: BeautifulSoup) -> None:
    """冒頭の「対象：」「問題：」を 1 つのメタ情報カードにまとめる。"""
    candidates: list[Tag] = []

    for child in soup.children:
        if isinstance(child, NavigableString):
            continue
        name = getattr(child, "name", None)
        if name == "h1" and not candidates:
            continue
        if name == "p":
            text = child.get_text("", strip=True)
            if text.startswith(("対象：", "問題：")):
                candidates.append(child)
                continue
        break

    if not candidates:
        return

    box = soup.new_tag("div")
    box["style"] = META
    candidates[0].insert_before(box)

    for p in candidates:
        p["style"] = (
            "margin:0;font-size:13.5px;line-height:1.75;color:#475569;"
            "white-space:pre-line;"
        )
        box.append(p.extract())


def style_blockquotes(soup: BeautifulSoup) -> None:
    for bq in soup.find_all("blockquote"):
        text = bq.get_text("", strip=True)
        is_warning = ("不備" in text) or ("注意" in text)
        bq["style"] = WARN if is_warning else NOTE

        paragraphs = bq.find_all("p")
        for i, p in enumerate(paragraphs):
            margin = "0" if i == len(paragraphs) - 1 else "0 0 .6em"
            p["style"] = (
                f"margin:{margin};color:inherit;font-size:inherit;"
                "line-height:inherit;"
            )


def style_headings(soup: BeautifulSoup) -> None:
    for h in soup.find_all(["h1", "h2", "h3", "h4", "h5", "h6"]):
        text = h.get_text("", strip=True)
        if h.name == "h1":
            h["style"] = H1
        elif h.name == "h2":
            h["style"] = H2
        elif h.name == "h3":
            h["style"] = H3_WARN if "問題不備" in text else H3
        elif h.name == "h4":
            h["style"] = H4
        else:
            h["style"] = H5


def style_paragraphs(soup: BeautifulSoup) -> None:
    for p in soup.find_all("p"):
        # メタ情報カードや引用内の段落は個別にスタイル済み
        if p.find_parent(["blockquote"]):
            continue
        if p.parent is not None and p.parent.name == "div":
            continue

        text = p.get_text("", strip=True)

        if text.startswith("解答："):
            p["style"] = ANSWER
            p["data-answer"] = "1"
            for strong in p.find_all("strong"):
                strong["style"] = "color:inherit;font-weight:700;"
        elif text.startswith("採点講評"):
            p["style"] = COMMENT
            for strong in p.find_all("strong"):
                strong["style"] = "color:#b45309;font-weight:700;"
        elif (
            len(p.contents) == 1
            and isinstance(p.contents[0], Tag)
            and p.contents[0].name == "strong"
        ):
            # 「**社内→社外（…）**」のような小見出し
            p["style"] = LABEL
            p.find("strong")["style"] = "font-weight:700;"
        else:
            p["style"] = P


def style_lists(soup: BeautifulSoup) -> None:
    for lst in soup.find_all(["ul", "ol"]):
        nested = lst.parent is not None and lst.parent.name == "li"
        lst["style"] = LIST_NESTED if nested else LIST

    for li in soup.find_all("li"):
        li["style"] = LI
        # ルーズリスト内の段落は余白を詰める
        for p in li.find_all("p", recursive=False):
            p["style"] = (
                "margin:0;font-size:inherit;line-height:inherit;color:inherit;"
            )


def style_tables(soup: BeautifulSoup) -> None:
    for table in list(soup.find_all("table")):
        rows = table.find_all("tr")
        ncols = 0
        if rows:
            ncols = len(rows[0].find_all(["th", "td"]))

        wrapper = soup.new_tag("div")
        wrapper["style"] = TABLE_WRAP
        table.insert_before(wrapper)
        wrapper.append(table.extract())

        table_style = TABLE
        if ncols >= 8:
            table_style = table_style.replace(
                "font-size:14px;", "font-size:12.5px;"
            )
        if ncols >= 6:
            table_style += f"min-width:{max(420, ncols * 72)}px;"
        table["style"] = table_style

        for th in table.find_all("th"):
            th["style"] = TH

        body = table.find("tbody")
        body_rows = body.find_all("tr") if body else rows[1:]
        for i, tr in enumerate(body_rows):
            bg = "#ffffff" if i % 2 == 0 else "#f6f9fc"
            for td in tr.find_all("td"):
                td["style"] = TD + f"background-color:{bg};"

        if ncols >= 7:
            hint = soup.new_tag("div")
            hint["style"] = TABLE_HINT
            hint.string = "※ 横にスクロールできます"
            wrapper.insert_before(hint)


def style_code(soup: BeautifulSoup) -> None:
    for pre in soup.find_all("pre"):
        pre["style"] = PRE
        for code in pre.find_all("code"):
            code["style"] = (
                "margin:0;padding:0;border:0;background:none;color:inherit;"
                "font-family:inherit;font-size:inherit;white-space:pre;"
            )

    for code in soup.find_all("code"):
        if code.find_parent("pre"):
            continue
        code["style"] = CODE


def style_misc(soup: BeautifulSoup) -> None:
    for a in soup.find_all("a"):
        if not a.get("style"):
            a["style"] = LINK

    for hr in soup.find_all("hr"):
        hr["style"] = HR

    for img in soup.find_all("img"):
        img["style"] = IMG

    for strong in soup.find_all("strong"):
        if not strong.get("style"):
            strong["style"] = "font-weight:700;"


def build_toc(soup: BeautifulSoup, headings: list[Tag]) -> Tag:
    nav = soup.new_tag("div")
    nav["role"] = "navigation"
    nav["aria-label"] = "目次"
    nav["style"] = TOC

    title = soup.new_tag("div")
    title["style"] = TOC_TITLE
    title.string = "目次"
    nav.append(title)

    top = soup.new_tag("ol")
    top["style"] = TOC_OL
    nav.append(top)

    current_li: Tag | None = None
    current_sub: Tag | None = None

    for h in headings:
        text = h.get_text("", strip=True)
        link = soup.new_tag("a")
        link["href"] = "#" + h["id"]
        link["style"] = TOC_LINK
        if "問題不備" in text:
            link["style"] = TOC_LINK + "color:#b91c1c;font-weight:700;"
        link.string = text

        li = soup.new_tag("li")
        li["style"] = TOC_LI
        li.append(link)

        if h.name == "h2":
            top.append(li)
            current_li = li
            current_sub = None
        else:
            if current_li is None:
                top.append(li)
                continue
            if current_sub is None:
                current_sub = soup.new_tag("ul")
                current_sub["style"] = TOC_UL
                current_li.append(current_sub)
            current_sub.append(li)

    return nav


def insert_toc(soup: BeautifulSoup, nav: Tag, first_heading: Tag) -> None:
    """最初の見出しの直前（直前に hr があればその前）に目次を差し込む。"""
    prev = first_heading.previous_sibling
    while isinstance(prev, NavigableString) and not str(prev).strip():
        prev = prev.previous_sibling

    if getattr(prev, "name", None) == "hr":
        prev.insert_before(nav)
    else:
        first_heading.insert_before(nav)


def insert_vxlan_puzzle(soup: BeautifulSoup) -> None:
    """「図4の対応表の読み方」の直前に VXLAN ミニクイズを差し込む。"""
    target: Tag | None = None
    for heading in soup.find_all(["h2", "h3", "h4", "h5", "h6"]):
        if heading.get_text("", strip=True).startswith("図4の対応表"):
            target = heading
            break

    if target is None:
        return

    widget = BeautifulSoup(VXLAN_PUZZLE_HTML, "html.parser")
    for node in list(widget.contents):
        target.insert_before(node.extract())


def load_interactive_js() -> str | None:
    """記事末尾にインライン展開する JavaScript を読み込む。"""
    path = Path(__file__).resolve().with_name("hatena_interactive.js")
    if not path.is_file():
        print(
            f"[WARN] インタラクション用 JS が見つかりません: {path}\n"
            "       --no-js を付けるか、md_to_hatena.py と同じ場所に "
            "hatena_interactive.js を置いてください。",
            file=sys.stderr,
        )
        return None
    return path.read_text(encoding="utf-8").strip()


# ---------------------------------------------------------------------------
# 関連記事（過去ブログの Atom フィード）
# ---------------------------------------------------------------------------

DEFAULT_BLOG_URL = "https://yoshishinnze.hatenablog.com/"
RELATED_PAGE_SIZE = 30

# 記事の主題を表す語。候補記事とのキーワード一致度に使う。
# ASCII の語は単語境界で、日本語を含む語は部分一致で判定する。
DOMAIN_KEYWORDS = (
    "vxlan", "evpn", "vtep", "vni", "vlan", "bum", "arp", "macアドレス",
    "ospf", "lsdb", "bgp", "ibgp", "mp-bgp", "ecmp", "l3sw", "ルータid",
    "ループバック", "クラスターid", "マルチキャスト", "ユニキャスト",
    "イーサネット", "カプセル化", "トンネル", "アンダーレイ", "オーバーレイ",
    "データセンター", "レイヤー2", "レイヤー3", "ipアドレス", "サブネット",
    "デフォルトゲートウェイ", "ゲートウェイ", "ファイアウォール", "nat",
    "dns", "mxレコード", "spf", "dkim", "s/mime", "dmarc", "smtp",
    "mail from", "ptr", "逆引き", "電子メール", "メールサーバ", "証明書",
    "公開鍵", "秘密鍵", "rsa", "デジタル署名", "署名", "暗号化",
    "なりすまし", "送信ドメイン認証", "tls", "http", "パケット", "フレーム",
    "プロトコル", "ルーティング", "経路", "冗長化", "負荷分散",
)

# 記事タイトルに含まれていたら強く関連づける語。
# 「IPアドレス」「パケット」のような一般語は、タイトル一致の加点対象外。
TITLE_TOPIC_KEYWORDS = frozenset(
    {
        "vxlan", "evpn", "vtep", "vni", "vlan", "bum", "arp",
        "ospf", "lsdb", "bgp", "ibgp", "mp-bgp", "ecmp", "l3sw",
        "spf", "dkim", "s/mime", "dmarc", "smtp", "mail from",
        "mxレコード", "ptr", "カプセル化", "トンネル", "オーバーレイ",
        "アンダーレイ", "データセンター", "電子メール", "メールサーバ",
        "送信ドメイン認証", "なりすまし", "ルーティング", "ルータid",
        "ループバック", "クラスターid",
    }
)

ASCII_TOKEN_RE = re.compile(r"[a-z0-9][a-z0-9+#._/-]*")
JP_SEGMENT_RE = re.compile(r"[\u3040-\u30ff\u4e00-\u9fff]{2,}")
HEADING_LINE_RE = re.compile(r"^#{1,6}\s+(.*)$", re.MULTILINE)
SCRIPT_STYLE_RE = re.compile(
    r"(?is)<(script|style)\b[^>]*>.*?</\1>"
)
HTML_TAG_RE = re.compile(r"(?s)<[^>]+>")


def normalize_search_text(text: str) -> str:
    """類似度計算用に全角半角・大文字小文字の揺れをならす。"""
    return unicodedata.normalize("NFKC", text or "").lower()


def tokenize_search_text(text: str) -> list[str]:
    """ASCII の語と日本語の文字バイグラムに分解する。

    形態素解析ライブラリに依存せず、日本語は文字 n-gram で扱う。
    """
    normalized = normalize_search_text(text)
    tokens: list[str] = []

    for match in ASCII_TOKEN_RE.finditer(normalized):
        token = match.group(0).strip("._/-")
        if 2 <= len(token) <= 32:
            tokens.append(token)

    for match in JP_SEGMENT_RE.finditer(normalized):
        segment = match.group(0)
        for index in range(len(segment) - 1):
            tokens.append(segment[index : index + 2])

    return tokens


def weighted_token_counter(text: str, weight: int) -> Counter:
    counts = Counter(tokenize_search_text(text))
    if weight == 1:
        return counts
    return Counter(
        {token: count * weight for token, count in counts.items()}
    )


def merge_token_counters(*counters: Counter) -> Counter:
    merged: Counter = Counter()
    for counter in counters:
        merged.update(counter)
    return merged


def cosine_similarity(left: dict, right: dict) -> float:
    if not left or not right:
        return 0.0
    if len(left) > len(right):
        left, right = right, left
    dot = sum(value * right.get(token, 0.0) for token, value in left.items())
    if dot == 0:
        return 0.0
    norm_left = math.sqrt(sum(value * value for value in left.values()))
    norm_right = math.sqrt(sum(value * value for value in right.values()))
    if not norm_left or not norm_right:
        return 0.0
    return dot / (norm_left * norm_right)


def extract_domain_keywords(text: str) -> set[str]:
    normalized = normalize_search_text(text)
    found: set[str] = set()

    for keyword in DOMAIN_KEYWORDS:
        if keyword.isascii():
            pattern = (
                r"(?<![a-z0-9])"
                + re.escape(keyword)
                + r"(?![a-z0-9])"
            )
            if re.search(pattern, normalized):
                found.add(keyword)
        elif keyword in normalized:
            found.add(keyword)

    return found


def html_to_plain_text(raw: str, limit: int | None = None) -> str:
    """HTML 断片から本文テキストを取り出す。"""
    text = SCRIPT_STYLE_RE.sub(" ", raw or "")
    text = HTML_TAG_RE.sub(" ", text)
    text = html.unescape(text)
    text = re.sub(r"\s+", " ", text).strip()
    if limit is not None and len(text) > limit:
        text = text[:limit].rstrip()
    return text


def plain_summary_text(raw: str, limit: int = 118) -> str:
    """フィードの summary をカード用のプレーンテキストにする。"""
    text = html_to_plain_text(raw)
    text = re.sub(r"^目次\s*", "", text)
    # ブログ共通の BGM リンクなど、本文でない定型文を落とす
    text = re.sub(r"[🎵♪]?\s*BGMを再生する\s*", " ", text)
    text = re.sub(r"\s+", " ", text).strip()
    if len(text) > limit:
        text = text[:limit].rstrip() + "…"
    return text


def fetch_atom_page(
    blog_url: str, page: int, timeout: float = 20.0
) -> list[dict]:
    """Atom フィードの 1 ページ分を取得して必要な項目だけ返す。"""
    url = f"{blog_url.rstrip('/')}/feed?page={page}"
    request = Request(
        url,
        headers={
            "User-Agent": (
                "Mozilla/5.0 (compatible; md_to_hatena related-links)"
            )
        },
    )
    with urlopen(request, timeout=timeout) as response:
        payload = response.read()

    root = ET.fromstring(payload)
    namespace = ""
    if root.tag.startswith("{"):
        namespace = root.tag[1:].split("}", 1)[0]
    prefix = f"{{{namespace}}}" if namespace else ""

    entries: list[dict] = []
    for node in root.findall(f"{prefix}entry"):
        title = (node.findtext(f"{prefix}title") or "").strip()
        link = ""
        for link_node in node.findall(f"{prefix}link"):
            if link_node.get("rel") in (None, "alternate"):
                link = link_node.get("href") or ""
                if link:
                    break
        if not link:
            continue

        published = (
            node.findtext(f"{prefix}published")
            or node.findtext(f"{prefix}updated")
            or ""
        ).strip()
        categories = [
            (category.get("term") or "").strip()
            for category in node.findall(f"{prefix}category")
            if (category.get("term") or "").strip()
        ]
        summary = node.findtext(f"{prefix}summary") or ""
        content = node.findtext(f"{prefix}content") or ""

        entries.append(
            {
                "title": title,
                "url": link,
                "published": published,
                "categories": categories,
                "summary": summary[:1500],
                # 類似度計算用。フィードには本文全体が入っているので、
                # 先頭 3000 文字だけをキャッシュして精度を上げる。
                "content_text": html_to_plain_text(content, limit=3000),
            }
        )

    return entries


def load_related_cache(path: Path) -> dict | None:
    if not path.is_file():
        return None
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(
            f"[WARN] 関連記事キャッシュを読めません: {path} ({exc})",
            file=sys.stderr,
        )
        return None
    if not isinstance(data, dict) or not isinstance(
        data.get("entries"), list
    ):
        return None
    return data


def save_related_cache(
    path: Path, blog_url: str, entries: list[dict]
) -> None:
    payload = {
        "blog_url": blog_url,
        "fetched_at": time.time(),
        "entries": entries,
    }
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            json.dumps(payload, ensure_ascii=False, indent=1),
            encoding="utf-8",
        )
    except OSError as exc:
        print(
            f"[WARN] 関連記事キャッシュを保存できません: {path} ({exc})",
            file=sys.stderr,
        )


def fetch_related_entries(
    blog_url: str, max_pages: int, timeout: float = 20.0
) -> list[dict]:
    """フィードを先頭から順に取得する。最終ページは 30 件未満で判定する。"""
    entries: list[dict] = []
    seen_urls: set[str] = set()

    for page in range(1, max_pages + 1):
        try:
            batch = fetch_atom_page(blog_url, page, timeout)
        except (URLError, ET.ParseError, OSError) as exc:
            if page == 1:
                raise RuntimeError(f"フィードを取得できません: {exc}") from exc
            print(
                f"[WARN] フィード {page} ページ目を取得できません: {exc}",
                file=sys.stderr,
            )
            break

        if not batch:
            break

        added = 0
        for entry in batch:
            if entry["url"] in seen_urls:
                continue
            seen_urls.add(entry["url"])
            entries.append(entry)
            added += 1

        if added == 0:
            break
        if len(batch) < RELATED_PAGE_SIZE:
            break

        # 相手サイトへの負荷を抑える
        time.sleep(0.1)

    return entries


def get_blog_entries(
    *,
    blog_url: str,
    cache_path: Path,
    max_pages: int,
    refresh: bool,
    max_age_hours: float,
) -> list[dict]:
    """キャッシュを優先し、必要ならフィードを再取得する。"""
    cache = load_related_cache(cache_path)
    now = time.time()
    cache_matches_blog = bool(
        cache and cache.get("blog_url") == blog_url
    )
    cache_entries = (
        cache.get("entries") if cache and cache_matches_blog else None
    )
    fetched_at = float(cache.get("fetched_at") or 0) if cache else 0.0

    cache_fresh = bool(
        cache_matches_blog
        and cache_entries
        and (now - fetched_at) < max_age_hours * 3600
    )

    if cache_fresh and not refresh:
        age = (now - fetched_at) / 3600
        print(
            f"[INFO] 関連記事キャッシュを使用: {len(cache_entries)} 件"
            f"（取得から {age:.1f} 時間）"
        )
        return cache_entries

    try:
        entries = fetch_related_entries(blog_url, max_pages)
    except Exception as exc:  # ネットワーク・パース失敗は記事生成を止めない
        if cache_entries:
            print(
                "[WARN] フィード取得に失敗したため、古いキャッシュを"
                f"使用します: {exc}",
                file=sys.stderr,
            )
            return cache_entries
        print(
            "[WARN] フィード取得に失敗したため、関連記事を"
            f"スキップします: {exc}",
            file=sys.stderr,
        )
        return []

    if entries:
        save_related_cache(cache_path, blog_url, entries)
        print(f"[INFO] 関連記事フィードを取得: {len(entries)} 件")

    return entries


def select_related_articles(
    *,
    title: str,
    article_md: str,
    entries: list[dict],
    count: int,
    min_score: float,
    exclude_urls: set[str] | None = None,
) -> list[dict]:
    """TF-IDF コサイン類似度とドメインキーワードの一致度で選ぶ。"""
    if not entries:
        return []

    normalized_title = re.sub(
        r"\s+", "", normalize_search_text(title)
    )
    excluded = exclude_urls or set()

    candidates: list[dict] = []
    for entry in entries:
        url = (entry.get("url") or "").strip()
        entry_title = (entry.get("title") or "").strip()
        if not url or not entry_title:
            continue
        if url in excluded:
            continue
        if normalized_title and re.sub(
            r"\s+", "", normalize_search_text(entry_title)
        ) == normalized_title:
            continue
        candidates.append(entry)

    if not candidates:
        return []

    article_doc = merge_token_counters(
        weighted_token_counter(title, 4),
        weighted_token_counter(
            "\n".join(HEADING_LINE_RE.findall(article_md)), 3
        ),
        weighted_token_counter(article_md, 1),
    )

    candidate_docs = [
        merge_token_counters(
            weighted_token_counter(entry.get("title", ""), 4),
            weighted_token_counter(
                " ".join(entry.get("categories") or []), 3
            ),
            weighted_token_counter(entry.get("summary", ""), 1),
            weighted_token_counter(entry.get("content_text", ""), 2),
        )
        for entry in candidates
    ]

    documents = [article_doc, *candidate_docs]
    document_freq: Counter = Counter()
    for document in documents:
        document_freq.update(document.keys())

    total_documents = len(documents)
    idf = {
        token: math.log(
            (1 + total_documents) / (1 + frequency)
        )
        + 1.0
        for token, frequency in document_freq.items()
    }

    def to_vector(document: Counter) -> dict:
        return {
            token: (1.0 + math.log(value)) * idf[token]
            for token, value in document.items()
            if value > 0
        }

    article_vector = to_vector(article_doc)
    article_keywords = extract_domain_keywords(title + "\n" + article_md)

    entry_keyword_sets = [
        extract_domain_keywords(
            " ".join(
                [
                    entry.get("title", ""),
                    " ".join(entry.get("categories") or []),
                    entry.get("summary", ""),
                    entry.get("content_text", ""),
                ]
            )
        )
        for entry in candidates
    ]

    # よく出るキーワード（パケット・プロトコル等）の重みを下げる
    keyword_freq: Counter = Counter()
    for keywords in entry_keyword_sets:
        keyword_freq.update(keywords)
    keyword_idf = {
        keyword: math.log(
            (1 + len(candidates)) / (1 + frequency)
        )
        + 1.0
        for keyword, frequency in keyword_freq.items()
    }

    scored: list[dict] = []
    for entry, document, entry_keywords in zip(
        candidates, candidate_docs, entry_keyword_sets
    ):
        vector = to_vector(document)
        cosine = cosine_similarity(article_vector, vector)

        intersection = article_keywords & entry_keywords
        union = article_keywords | entry_keywords
        weighted_union = sum(
            keyword_idf.get(keyword, 1.0) for keyword in union
        )
        keyword_score = (
            sum(
                keyword_idf.get(keyword, 1.0)
                for keyword in intersection
            )
            / weighted_union
            if weighted_union
            else 0.0
        )

        title_hits = (
            extract_domain_keywords(entry.get("title", ""))
            & article_keywords
            & TITLE_TOPIC_KEYWORDS
        )
        title_bonus = 0.10 * min(len(title_hits), 2)

        score = 0.68 * cosine + 0.32 * keyword_score + title_bonus
        if "ネットワーク" in (entry.get("categories") or []):
            score += 0.02
        if score < min_score:
            continue

        scored.append(
            {
                **entry,
                "score": score,
                "summary_text": plain_summary_text(
                    entry.get("summary", "")
                ),
            }
        )

    scored.sort(
        key=lambda item: (
            item["score"],
            item.get("published") or "",
        ),
        reverse=True,
    )
    return scored[:count]


def build_related_section(
    soup: BeautifulSoup, related: list[dict]
) -> tuple[Tag, Tag]:
    heading = soup.new_tag("h2")
    heading["style"] = H2
    heading.string = "関連記事"

    wrap = soup.new_tag("div")
    wrap["data-role"] = "related-articles"
    wrap["style"] = RELATED_WRAP

    intro = soup.new_tag("p")
    intro["style"] = RELATED_INTRO
    intro.string = (
        "この記事と内容が近い過去記事を、ブログ内の記事から自動で選びました。"
    )
    wrap.append(intro)

    for item in related:
        card = soup.new_tag("a")
        card["href"] = item["url"]
        card["target"] = "_blank"
        card["rel"] = "noopener"
        card["data-related-card"] = "1"
        card["style"] = RELATED_CARD

        top = soup.new_tag("div")
        top["style"] = RELATED_TOP

        categories = item.get("categories") or []
        badge = soup.new_tag("span")
        badge["style"] = RELATED_BADGE
        badge.string = "・".join(categories[:2]) if categories else "記事"
        top.append(badge)

        published = (item.get("published") or "")[:10]
        if published:
            date = soup.new_tag("span")
            date["style"] = RELATED_DATE
            date.string = published.replace("-", ".")
            top.append(date)
        card.append(top)

        title = soup.new_tag("div")
        title["style"] = RELATED_TITLE
        title.append(NavigableString(item.get("title") or ""))
        arrow = soup.new_tag("span")
        arrow["aria-hidden"] = "true"
        arrow["style"] = RELATED_ARROW
        arrow.string = " →"
        title.append(arrow)
        card.append(title)

        summary_text = item.get("summary_text") or ""
        if summary_text:
            summary = soup.new_tag("p")
            summary["style"] = RELATED_SUMMARY
            summary.string = summary_text
            card.append(summary)

        wrap.append(card)

    note = soup.new_tag("p")
    note["style"] = RELATED_NOTE
    note.string = (
        "※ 本文と過去記事のタイトル・概要の類似度による自動選定です。"
    )
    wrap.append(note)

    return heading, wrap


def insert_related_section(
    soup: BeautifulSoup, related: list[dict]
) -> None:
    if not related:
        return
    heading, wrap = build_related_section(soup, related)
    soup.append(heading)
    soup.append(wrap)


def build_fragment(
    md_text: str,
    *,
    keep_title: bool,
    with_toc: bool,
    with_js: bool,
    with_widget: bool,
    related: list[dict] | None = None,
    source_name: str,
) -> str:
    soup = BeautifulSoup(markdown_to_html(md_text), "html.parser")

    if keep_title:
        title_heading = soup.find("h1")
        if title_heading is not None:
            title_heading["data-role"] = "article-title"

    normalize_headings(soup)

    wrap_meta(soup)
    style_blockquotes(soup)
    style_headings(soup)
    style_paragraphs(soup)
    style_lists(soup)
    style_tables(soup)
    style_code(soup)
    style_misc(soup)

    if with_widget:
        insert_vxlan_puzzle(soup)

    insert_related_section(soup, related or [])

    headings = soup.find_all(["h2", "h3", "h4"])
    for i, h in enumerate(headings, 1):
        h["id"] = f"sec-{i}"

    # 本文にタイトルを残す場合、タイトル自身は目次に載せない
    toc_targets = [
        h
        for h in headings
        if h.name in ("h2", "h3")
        and h.get("data-role") != "article-title"
    ]

    if with_toc and toc_targets:
        nav = build_toc(soup, toc_targets)
        insert_toc(soup, nav, toc_targets[0])

    body = soup.decode_contents()

    comment = (
        "<!-- ============================================================\n"
        "     はてなブログ「HTML編集」モード用の記事本文\n"
        f"     生成元: {source_name}\n"
        "     記事タイトルははてなブログ側のタイトル欄に設定してください\n"
        "     インタラクションが動かない場合は、末尾のスクリプト部分を\n"
        "     「デザイン > カスタマイズ > 記事下」へ移動してください\n"
        "     ============================================================ -->"
    )

    fragment = (
        f'{comment}\n<div data-hatena-article="1" style="{BASE}">\n'
        f"{body}\n</div>\n"
    )

    if with_js:
        script = load_interactive_js()
        if script:
            fragment += f"\n<script>\n{script}\n</script>\n"

    return fragment


# ---------------------------------------------------------------------------
# プレビュー HTML
# ---------------------------------------------------------------------------

PREVIEW_TEMPLATE = Template(
    """<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>$title</title>
<style>
  * { box-sizing: border-box; }
  body { margin: 0; background: #eef2f7; }
  .page {
    max-width: 900px;
    margin: 40px auto;
    padding: 48px 56px;
    background: #ffffff;
    border-radius: 12px;
    box-shadow: 0 10px 35px rgba(15, 23, 42, .08);
  }
  .article-title {
    margin: 0 0 28px;
    padding: 0 0 18px;
    border-bottom: 3px solid #1e3a8a;
    color: #0f172a;
    font-family: $font;
    font-size: 28px;
    line-height: 1.45;
    font-weight: 700;
    letter-spacing: .01em;
  }
  @media (max-width: 720px) {
    .page { margin: 0; padding: 28px 18px; border-radius: 0; }
    .article-title { font-size: 22px; }
  }
</style>
</head>
<body>
<main class="page">
$title_block
$body
</main>
</body>
</html>
"""
)


def build_preview(title: str, fragment: str, *, include_title: bool) -> str:
    title_block = ""
    if include_title:
        title_block = f'<h1 class="article-title">{html.escape(title)}</h1>'

    return PREVIEW_TEMPLATE.substitute(
        title=html.escape(title),
        font=FONT,
        title_block=title_block,
        body=fragment,
    )


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Markdown をはてなブログの HTML 編集モード向けに変換します。"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("input", type=Path, help="入力 Markdown ファイル")
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="はてな用 HTML の出力先（既定: <入力名>_hatena.html）",
    )
    parser.add_argument(
        "--preview",
        type=Path,
        help="プレビュー HTML の出力先（既定: <入力名>_preview.html）",
    )
    parser.add_argument(
        "--no-preview",
        action="store_true",
        help="プレビュー HTML を出力しない",
    )
    parser.add_argument(
        "--keep-title",
        action="store_true",
        help="先頭の H1 を本文に残す（既定では除去）",
    )
    parser.add_argument(
        "--no-toc",
        action="store_true",
        help="目次を生成しない",
    )
    parser.add_argument(
        "--title",
        help="記事タイトル（省略時は先頭の H1 を使用）",
    )
    parser.add_argument(
        "--no-js",
        action="store_true",
        help="読了バー・クイズなどのインタラクションを付けない",
    )
    parser.add_argument(
        "--no-widget",
        action="store_true",
        help="VXLAN ミニクイズを挿入しない",
    )
    parser.add_argument(
        "--no-related",
        action="store_true",
        help="過去記事の「関連記事」セクションを挿入しない",
    )
    parser.add_argument(
        "--refresh-related",
        action="store_true",
        help="関連記事フィードのキャッシュを無視して再取得する",
    )
    parser.add_argument(
        "--related-count",
        type=int,
        default=5,
        help="関連記事の最大件数（既定: 5、1〜20）",
    )
    parser.add_argument(
        "--related-pages",
        type=int,
        default=15,
        help="取得するフィードの最大ページ数（既定: 15、1〜50）",
    )
    parser.add_argument(
        "--related-min-score",
        type=float,
        default=0.02,
        help="関連記事として採用する最低スコア（既定: 0.02）",
    )
    parser.add_argument(
        "--related-cache",
        type=Path,
        help="フィードキャッシュの保存先（既定: スクリプトと同じ場所）",
    )
    parser.add_argument(
        "--related-cache-hours",
        type=float,
        default=12.0,
        help="フィードキャッシュの有効期間（時間、既定: 12）",
    )
    parser.add_argument(
        "--blog-url",
        default=DEFAULT_BLOG_URL,
        help=f"ブログのURL（既定: {DEFAULT_BLOG_URL}）",
    )
    parser.add_argument(
        "--related-exclude-url",
        action="append",
        default=[],
        metavar="URL",
        help="関連記事から除外するURL（複数指定可）",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    # Windows のコンソールでも日本語が文字化けしないようにする
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(encoding="utf-8")

    args = parse_args(argv)

    if not args.input.is_file():
        print(f"入力ファイルが見つかりません: {args.input}", file=sys.stderr)
        return 1

    md_text = read_markdown(args.input)

    # タイトルは常に検出し、既定では本文から除去する
    body_md, detected_title = split_title(md_text)
    if args.keep_title:
        body_md = md_text

    title = args.title or detected_title or args.input.stem

    related: list[dict] = []
    if not args.no_related:
        cache_path = args.related_cache or Path(__file__).resolve().with_name(
            ".hatena_feed_cache.json"
        )
        entries = get_blog_entries(
            blog_url=args.blog_url,
            cache_path=cache_path,
            max_pages=max(1, min(args.related_pages, 50)),
            refresh=args.refresh_related,
            max_age_hours=max(0.0, args.related_cache_hours),
        )
        if entries:
            try:
                related = select_related_articles(
                    title=title,
                    article_md=body_md,
                    entries=entries,
                    count=max(1, min(args.related_count, 20)),
                    min_score=args.related_min_score,
                    exclude_urls=set(args.related_exclude_url),
                )
            except Exception as exc:  # 記事生成を止めない
                print(
                    f"[WARN] 関連記事の選定に失敗しました: {exc}",
                    file=sys.stderr,
                )
                related = []

        if related:
            print(f"[RELATED] {len(related)} 件を挿入:")
            for index, item in enumerate(related, 1):
                print(
                    f"  {index}. [{item['score']:.3f}] "
                    f"{item['title']} ({(item.get('published') or '')[:10]})"
                )
        else:
            print("[RELATED] 関連記事なし")

    fragment = build_fragment(
        body_md,
        keep_title=args.keep_title,
        with_toc=not args.no_toc,
        with_js=not args.no_js,
        with_widget=not args.no_widget,
        related=related,
        source_name=args.input.name,
    )

    output = args.output or args.input.with_name(
        args.input.stem + "_hatena.html"
    )
    with output.open("w", encoding="utf-8", newline="\n") as f:
        f.write(fragment)

    print(f"[OK] はてな用HTML : {output} ({output.stat().st_size / 1024:.1f} KB)")

    if not args.no_preview:
        preview_path = args.preview or args.input.with_name(
            args.input.stem + "_preview.html"
        )
        preview = build_preview(
            title,
            fragment,
            include_title=not args.keep_title,
        )
        with preview_path.open("w", encoding="utf-8", newline="\n") as f:
            f.write(preview)
        print(
            f"[OK] プレビュー   : {preview_path} "
            f"({preview_path.stat().st_size / 1024:.1f} KB)"
        )

    print(f"[TITLE] 推奨タイトル: {title}")
    print(
        "       はてなブログの編集画面で「HTML編集」タブを開き、"
        "生成した HTML を貼り付けてください。"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
