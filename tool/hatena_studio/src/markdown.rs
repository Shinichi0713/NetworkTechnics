//! Markdown から「はてなブログ HTML 編集モード」向けの HTML を生成する。
//!
//! Python 版 (`md_to_hatena.py`) の変換結果にできるだけ近づけるため、
//! 見出しレベルの補正、メタ情報カード、解答コールアウト、表のラッパー、
//! 目次、関連記事、VXLAN ミニクイズなどを同じ順序で適用している。

use html_escape::{encode_double_quoted_attribute, encode_text};
use pulldown_cmark::{
    Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd,
};

// ---------------------------------------------------------------------------
// デザイントークン（インライン CSS）
// ---------------------------------------------------------------------------

pub const FONT: &str = "'Helvetica Neue',Arial,'Hiragino Kaku Gothic ProN','Hiragino Sans','BIZ UDPGothic',Meiryo,sans-serif";

pub const BASE: &str = "font-family:'Helvetica Neue',Arial,'Hiragino Kaku Gothic ProN','Hiragino Sans','BIZ UDPGothic',Meiryo,sans-serif;font-size:16px;line-height:1.9;color:#1f2937;letter-spacing:.01em;overflow-wrap:break-word;word-wrap:break-word;";

const H1: &str = "margin:0 0 28px;padding:0 0 18px;border-bottom:3px solid #1e3a8a;color:#0f172a;font-size:27px;line-height:1.45;font-weight:700;letter-spacing:.01em;scroll-margin-top:80px;";
const H2: &str = "margin:48px 0 22px;padding:12px 18px;border-radius:8px;background-color:#1e3a8a;background-image:linear-gradient(135deg,#1e3a8a 0%,#2563eb 100%);color:#ffffff;font-size:22px;line-height:1.5;font-weight:700;letter-spacing:.02em;box-shadow:0 2px 6px rgba(30,58,138,.18);scroll-margin-top:80px;";
const H3: &str = "margin:40px 0 16px;padding:0 0 7px;border-bottom:2px solid #bfdbfe;color:#1e3a8a;font-size:19px;line-height:1.55;font-weight:700;scroll-margin-top:80px;";
const H3_WARN: &str = "margin:40px 0 16px;padding:11px 15px;border:1px solid #fecaca;border-left:6px solid #ef4444;border-radius:0 8px 8px 0;background-color:#fef2f2;color:#991b1b;font-size:19px;line-height:1.55;font-weight:700;scroll-margin-top:80px;";
const H4: &str = "margin:28px 0 12px;padding:2px 0 2px 11px;border-left:5px solid #3b82f6;color:#0f172a;font-size:16.5px;line-height:1.6;font-weight:700;scroll-margin-top:80px;";
const H5: &str = "margin:24px 0 10px;padding:0;color:#334155;font-size:15.5px;line-height:1.6;font-weight:700;";

const P: &str = "margin:0 0 1.15em;font-size:inherit;line-height:inherit;color:inherit;";
const ANSWER: &str = "margin:16px 0 20px;padding:11px 16px;border:1px solid #a7f3d0;border-left:5px solid #10b981;border-radius:0 8px 8px 0;background-color:#ecfdf5;color:#065f46;font-size:15.5px;line-height:1.8;font-weight:600;";
const COMMENT: &str = "margin:18px 0;padding:10px 14px;border-left:5px solid #f59e0b;border-radius:0 6px 6px 0;background-color:#fffbeb;color:#78350f;font-size:14px;line-height:1.8;";
const LABEL: &str = "margin:22px 0 10px;padding:7px 12px;border-left:4px solid #60a5fa;border-radius:0 4px 4px 0;background-color:#eff6ff;color:#1e40af;font-size:14.5px;line-height:1.7;";
const NOTE: &str = "margin:20px 0;padding:14px 18px;border-left:4px solid #94a3b8;border-radius:0 8px 8px 0;background-color:#f8fafc;color:#475569;font-size:14.5px;line-height:1.85;";
const WARN: &str = "margin:20px 0;padding:14px 18px;border-left:4px solid #ef4444;border-radius:0 8px 8px 0;background-color:#fef2f2;color:#991b1b;font-size:14.5px;line-height:1.85;";
const META: &str = "margin:0 0 20px;padding:12px 16px;border:1px solid #e2e8f0;border-radius:8px;background-color:#f8fafc;font-size:13.5px;line-height:1.75;color:#475569;";
const META_P: &str = "margin:0;font-size:13.5px;line-height:1.75;color:#475569;white-space:pre-line;";

const TABLE_WRAP: &str = "margin:18px 0 24px;overflow-x:auto;-webkit-overflow-scrolling:touch;border:1px solid #e2e8f0;border-radius:8px;";
const TABLE: &str = "width:100%;border-collapse:collapse;border-spacing:0;background-color:#ffffff;font-size:14px;line-height:1.65;";
const TH: &str = "padding:10px 12px;border:1px solid #cbd5e1;background-color:#1e3a8a;color:#ffffff;font-weight:700;font-size:13.5px;text-align:left;white-space:nowrap;vertical-align:middle;";
const TD: &str = "padding:9px 12px;border:1px solid #e2e8f0;color:#1f2937;font-size:inherit;line-height:inherit;vertical-align:top;";
const TABLE_HINT: &str = "margin:0 0 6px;font-size:12px;line-height:1.5;color:#94a3b8;text-align:right;";

const PRE: &str = "margin:18px 0 24px;padding:16px 18px;border-radius:8px;background-color:#0f172a;color:#e2e8f0;font-family:'SFMono-Regular',Consolas,'Liberation Mono',Menlo,monospace;font-size:13px;line-height:1.7;overflow-x:auto;-webkit-overflow-scrolling:touch;tab-size:4;";
const CODE: &str = "padding:2px 6px;border-radius:4px;background-color:#eef2ff;color:#3730a3;font-family:'SFMono-Regular',Consolas,'Liberation Mono',Menlo,monospace;font-size:.88em;";

const LIST: &str = "margin:.8em 0 1.1em;padding-left:1.6em;font-size:inherit;line-height:inherit;color:inherit;";
const LIST_NESTED: &str = "margin:.35em 0 .5em;padding-left:1.3em;font-size:inherit;line-height:inherit;color:inherit;";
const LI: &str = "margin:.35em 0;font-size:inherit;line-height:inherit;color:inherit;";
const LI_P: &str = "margin:0;font-size:inherit;line-height:inherit;color:inherit;";

const HR: &str = "margin:40px 0;border:0;height:1px;background-color:#e2e8f0;";
const LINK: &str = "color:#1d4ed8;text-decoration:underline;text-underline-offset:2px;";
const IMG: &str = "max-width:100%;height:auto;border-radius:8px;";

const TOC: &str = "margin:0 0 32px;padding:18px 22px;border:1px solid #dbeafe;border-radius:10px;background-color:#f8fafc;font-size:14px;line-height:1.75;";
const TOC_TITLE: &str = "margin:0 0 10px;padding-left:10px;border-left:4px solid #2563eb;font-size:15px;font-weight:700;color:#1e3a8a;letter-spacing:.04em;";
const TOC_OL: &str = "margin:0;padding-left:1.4em;list-style:decimal;";
const TOC_UL: &str = "margin:4px 0 8px;padding-left:1.15em;list-style:circle;";
const TOC_LI: &str = "margin:4px 0;";
const TOC_LINK: &str = "color:#1e40af;text-decoration:none;border-bottom:1px dotted #93c5fd;";

const RELATED_WRAP: &str = "margin:0;padding:0;";
const RELATED_INTRO: &str = "margin:0 0 14px;font-size:13px;line-height:1.8;color:#475569;";
const RELATED_CARD: &str = "display:block;margin:0 0 12px;padding:14px 16px;border:1px solid #dbeafe;border-left:5px solid #2563eb;border-radius:0 10px 10px 0;background-color:#ffffff;box-shadow:0 1px 2px rgba(15,23,42,.04);text-decoration:none;color:inherit;transition:transform .15s ease,box-shadow .15s ease,background-color .15s ease;";
const RELATED_TOP: &str = "display:flex;flex-wrap:wrap;align-items:center;gap:8px;margin:0;";
const RELATED_BADGE: &str = "display:inline-block;padding:2px 9px;border-radius:999px;background-color:#eff6ff;color:#1e40af;font-size:10.5px;font-weight:700;line-height:1.6;letter-spacing:.06em;";
const RELATED_DATE: &str = "font-size:11px;line-height:1.6;color:#64748b;font-weight:600;letter-spacing:.02em;";
const RELATED_TITLE: &str = "margin:7px 0 0;font-size:15.5px;line-height:1.55;font-weight:700;color:#1e3a8a;";
const RELATED_ARROW: &str = "color:#2563eb;font-weight:700;";
const RELATED_SUMMARY: &str = "margin:5px 0 0;font-size:12.5px;line-height:1.7;color:#475569;";
const RELATED_NOTE: &str = "margin:12px 0 0;font-size:11.5px;line-height:1.65;color:#94a3b8;";

// ---------------------------------------------------------------------------
// 公開型
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct RelatedCard {
    pub title: String,
    pub url: String,
    pub published: String,
    pub categories: Vec<String>,
    pub summary_text: String,
    pub score: f64,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct Stats {
    pub headings: usize,
    pub toc_links: usize,
    pub tables: usize,
    pub code_blocks: usize,
    pub answers: usize,
    pub bytes: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderOptions<'a> {
    pub keep_title: bool,
    pub with_toc: bool,
    pub with_widget: bool,
    pub related: &'a [RelatedCard],
    pub source_name: &'a str,
    pub interactive_js: Option<&'a str>,
}

pub struct Fragment {
    pub html: String,
    pub stats: Stats,
}

// ---------------------------------------------------------------------------
// Markdown AST
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Inline {
    Text(String),
    Code(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Strike(Vec<Inline>),
    Link {
        href: String,
        title: String,
        children: Vec<Inline>,
    },
    Image {
        src: String,
        title: String,
        alt: String,
    },
    Html(String),
    SoftBreak,
    HardBreak,
    FootnoteRef(String),
}

#[derive(Debug, Clone)]
struct Heading {
    level: u8,
    inlines: Vec<Inline>,
    article_title: bool,
    id: Option<String>,
}

#[derive(Debug, Clone)]
struct Paragraph {
    inlines: Vec<Inline>,
    /// タイトリストの項目のように、`<p>` を付けずに描画する段落。
    bare: bool,
}

#[derive(Debug, Clone)]
struct ListBlock {
    ordered: bool,
    start: u64,
    items: Vec<Vec<Block>>,
}

#[derive(Debug, Clone)]
struct Cell {
    inlines: Vec<Inline>,
}

#[derive(Debug, Clone)]
struct TableBlock {
    header: Vec<Cell>,
    rows: Vec<Vec<Cell>>,
    #[allow(dead_code)]
    aligns: Vec<Alignment>,
}

#[derive(Debug, Clone)]
enum Block {
    Heading(Heading),
    Paragraph(Paragraph),
    Quote(Vec<Block>),
    List(ListBlock),
    Code { lang: String, text: String },
    Table(TableBlock),
    Rule,
    Html(String),
    Meta(Vec<Paragraph>),
}

#[derive(Clone, Copy, PartialEq)]
enum Stop {
    Document,
    Quote,
    Item,
}

// ---------------------------------------------------------------------------
// Markdown パース
// ---------------------------------------------------------------------------

fn markdown_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options
}

fn parse_blocks(events: &[Event<'_>], pos: &mut usize, stop: Stop) -> Vec<Block> {
    let mut blocks = Vec::new();

    while *pos < events.len() {
        let event = events[*pos].clone();
        match event {
            Event::Start(Tag::Paragraph) => {
                *pos += 1;
                let inlines = parse_inlines(events, pos, TagEnd::Paragraph);
                blocks.push(Block::Paragraph(Paragraph {
                    inlines,
                    bare: false,
                }));
            }
            Event::Start(Tag::Heading { level, .. }) => {
                *pos += 1;
                let inlines = parse_inlines(events, pos, TagEnd::Heading(level));
                blocks.push(Block::Heading(Heading {
                    level: level as u8,
                    inlines,
                    article_title: false,
                    id: None,
                }));
            }
            Event::Start(Tag::BlockQuote(_)) => {
                *pos += 1;
                let inner = parse_blocks(events, pos, Stop::Quote);
                blocks.push(Block::Quote(inner));
            }
            Event::Start(Tag::List(start)) => {
                *pos += 1;
                let ordered = start.is_some();
                let start_number = start.unwrap_or(1);
                let mut items = Vec::new();
                while *pos < events.len() {
                    match events[*pos].clone() {
                        Event::Start(Tag::Item) => {
                            *pos += 1;
                            items.push(parse_blocks(events, pos, Stop::Item));
                        }
                        Event::End(TagEnd::List(_)) => {
                            *pos += 1;
                            break;
                        }
                        _ => *pos += 1,
                    }
                }
                blocks.push(Block::List(ListBlock {
                    ordered,
                    start: start_number,
                    items,
                }));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                *pos += 1;
                let lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                let mut text = String::new();
                while *pos < events.len() {
                    match events[*pos].clone() {
                        Event::Text(chunk) => {
                            text.push_str(&chunk);
                            *pos += 1;
                        }
                        Event::End(TagEnd::CodeBlock) => {
                            *pos += 1;
                            break;
                        }
                        _ => *pos += 1,
                    }
                }
                blocks.push(Block::Code { lang, text });
            }
            Event::Start(Tag::Table(aligns)) => {
                *pos += 1;
                blocks.push(Block::Table(parse_table(events, pos, aligns)));
            }
            Event::Start(Tag::HtmlBlock) => {
                *pos += 1;
                let mut html = String::new();
                while *pos < events.len() {
                    match events[*pos].clone() {
                        Event::Html(chunk) | Event::Text(chunk) => {
                            html.push_str(&chunk);
                            *pos += 1;
                        }
                        Event::End(TagEnd::HtmlBlock) => {
                            *pos += 1;
                            break;
                        }
                        _ => *pos += 1,
                    }
                }
                blocks.push(Block::Html(html));
            }
            Event::Start(Tag::FootnoteDefinition(_)) => {
                // 脚注定義は本文に直接出さない。対応する End まで読み飛ばす。
                *pos += 1;
                let mut depth = 1usize;
                while *pos < events.len() {
                    match events[*pos] {
                        Event::Start(Tag::FootnoteDefinition(_)) => depth += 1,
                        Event::End(TagEnd::FootnoteDefinition) => {
                            depth -= 1;
                            if depth == 0 {
                                *pos += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    *pos += 1;
                }
            }
            Event::Rule => {
                *pos += 1;
                blocks.push(Block::Rule);
            }
            Event::Html(html) => {
                *pos += 1;
                blocks.push(Block::Html(html.to_string()));
            }
            Event::End(TagEnd::BlockQuote(_)) if stop == Stop::Quote => {
                *pos += 1;
                return blocks;
            }
            Event::End(TagEnd::Item) if stop == Stop::Item => {
                *pos += 1;
                return blocks;
            }
            Event::Text(_)
            | Event::Code(_)
            | Event::InlineHtml(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::FootnoteReference(_)
            | Event::TaskListMarker(_)
            | Event::Start(Tag::Strong)
            | Event::Start(Tag::Emphasis)
            | Event::Start(Tag::Strikethrough)
            | Event::Start(Tag::Link { .. })
            | Event::Start(Tag::Image { .. }) => {
                let inlines = parse_bare_inlines(events, pos);
                if !inlines.is_empty() {
                    blocks.push(Block::Paragraph(Paragraph {
                        inlines,
                        bare: true,
                    }));
                }
            }
            _ => *pos += 1,
        }
    }

    blocks
}

fn parse_inlines(events: &[Event<'_>], pos: &mut usize, end: TagEnd) -> Vec<Inline> {
    let mut inlines = Vec::new();

    while *pos < events.len() {
        match events[*pos].clone() {
            Event::End(tag) => {
                *pos += 1;
                if tag == end {
                    break;
                }
            }
            Event::Text(text) => {
                inlines.push(Inline::Text(text.to_string()));
                *pos += 1;
            }
            Event::Code(code) => {
                inlines.push(Inline::Code(code.to_string()));
                *pos += 1;
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                inlines.push(Inline::Html(html.to_string()));
                *pos += 1;
            }
            Event::SoftBreak => {
                inlines.push(Inline::SoftBreak);
                *pos += 1;
            }
            Event::HardBreak => {
                inlines.push(Inline::HardBreak);
                *pos += 1;
            }
            Event::FootnoteReference(name) => {
                inlines.push(Inline::FootnoteRef(name.to_string()));
                *pos += 1;
            }
            Event::TaskListMarker(_) => {
                *pos += 1;
            }
            Event::Start(Tag::Strong) => {
                *pos += 1;
                let children = parse_inlines(events, pos, TagEnd::Strong);
                inlines.push(Inline::Strong(children));
            }
            Event::Start(Tag::Emphasis) => {
                *pos += 1;
                let children = parse_inlines(events, pos, TagEnd::Emphasis);
                inlines.push(Inline::Emphasis(children));
            }
            Event::Start(Tag::Strikethrough) => {
                *pos += 1;
                let children = parse_inlines(events, pos, TagEnd::Strikethrough);
                inlines.push(Inline::Strike(children));
            }
            Event::Start(Tag::Superscript) => {
                *pos += 1;
                let children = parse_inlines(events, pos, TagEnd::Superscript);
                inlines.extend(children);
            }
            Event::Start(Tag::Subscript) => {
                *pos += 1;
                let children = parse_inlines(events, pos, TagEnd::Subscript);
                inlines.extend(children);
            }
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) => {
                *pos += 1;
                let children = parse_inlines(events, pos, TagEnd::Link);
                inlines.push(Inline::Link {
                    href: dest_url.to_string(),
                    title: title.to_string(),
                    children,
                });
            }
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                *pos += 1;
                let children = parse_inlines(events, pos, TagEnd::Image);
                inlines.push(Inline::Image {
                    src: dest_url.to_string(),
                    title: title.to_string(),
                    alt: inline_text(&children),
                });
            }
            _ => *pos += 1,
        }
    }

    normalize_text_runs(inlines)
}

/// `**強調**` が CommonMark の区切り規則で解釈されなかった場合の救済。
///
/// 日本語では `**LSDB（…）**を` のように閉じる `**` の直後が助詞になることが多く、
/// CommonMark の right-flanking 条件を満たさず強調になりません。
/// Python 版の markdown は強調として扱うため、同じ結果になるよう補正します。
fn split_bold_text(text: &str) -> Vec<Inline> {
    if !text.contains("**") {
        return vec![Inline::Text(text.to_string())];
    }

    let mut inlines = Vec::new();
    let mut rest = text;
    let mut guard = 0;

    while let Some(start) = rest.find("**") {
        let Some(end_rel) = rest[start + 2..].find("**") else {
            break;
        };
        let inner_start = start + 2;
        let inner_end = inner_start + end_rel;
        let inner = &rest[inner_start..inner_end];

        if inner.is_empty() || inner.contains('\n') {
            break;
        }

        if start > 0 {
            inlines.push(Inline::Text(rest[..start].to_string()));
        }
        inlines.push(Inline::Strong(vec![Inline::Text(inner.to_string())]));
        rest = &rest[inner_end + 2..];

        guard += 1;
        if guard > 500 {
            break;
        }
    }

    if !rest.is_empty() {
        inlines.push(Inline::Text(rest.to_string()));
    }

    if inlines.is_empty() {
        inlines.push(Inline::Text(text.to_string()));
    }
    inlines
}

fn normalize_text_runs(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut out = Vec::new();
    let mut buffer = String::new();

    for inline in inlines {
        match inline {
            Inline::Text(text) => buffer.push_str(&text),
            other => {
                if !buffer.is_empty() {
                    out.extend(split_bold_text(&buffer));
                    buffer.clear();
                }
                out.push(other);
            }
        }
    }
    if !buffer.is_empty() {
        out.extend(split_bold_text(&buffer));
    }

    out
}

/// ブロック境界に当たるまで、インライン要素だけを読み取る。
///
/// pulldown-cmark はタイトリスト（`- item` が続くだけのリスト）で
/// `Paragraph` イベントを出さないため、その本文をここで拾う。
fn parse_bare_inlines(events: &[Event<'_>], pos: &mut usize) -> Vec<Inline> {
    let mut inlines = Vec::new();

    while *pos < events.len() {
        match events[*pos].clone() {
            Event::Text(text) => {
                inlines.push(Inline::Text(text.to_string()));
                *pos += 1;
            }
            Event::Code(code) => {
                inlines.push(Inline::Code(code.to_string()));
                *pos += 1;
            }
            Event::InlineHtml(html) => {
                inlines.push(Inline::Html(html.to_string()));
                *pos += 1;
            }
            Event::SoftBreak => {
                inlines.push(Inline::SoftBreak);
                *pos += 1;
            }
            Event::HardBreak => {
                inlines.push(Inline::HardBreak);
                *pos += 1;
            }
            Event::FootnoteReference(name) => {
                inlines.push(Inline::FootnoteRef(name.to_string()));
                *pos += 1;
            }
            Event::TaskListMarker(_) => {
                *pos += 1;
            }
            Event::Start(Tag::Strong) => {
                *pos += 1;
                let children = parse_inlines(events, pos, TagEnd::Strong);
                inlines.push(Inline::Strong(children));
            }
            Event::Start(Tag::Emphasis) => {
                *pos += 1;
                let children = parse_inlines(events, pos, TagEnd::Emphasis);
                inlines.push(Inline::Emphasis(children));
            }
            Event::Start(Tag::Strikethrough) => {
                *pos += 1;
                let children = parse_inlines(events, pos, TagEnd::Strikethrough);
                inlines.push(Inline::Strike(children));
            }
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) => {
                *pos += 1;
                let children = parse_inlines(events, pos, TagEnd::Link);
                inlines.push(Inline::Link {
                    href: dest_url.to_string(),
                    title: title.to_string(),
                    children,
                });
            }
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                *pos += 1;
                let children = parse_inlines(events, pos, TagEnd::Image);
                inlines.push(Inline::Image {
                    src: dest_url.to_string(),
                    title: title.to_string(),
                    alt: inline_text(&children),
                });
            }
            _ => break,
        }
    }

    normalize_text_runs(inlines)
}

fn parse_table(events: &[Event<'_>], pos: &mut usize, aligns: Vec<Alignment>) -> TableBlock {
    let mut header = Vec::new();
    let mut rows = Vec::new();

    while *pos < events.len() {
        match events[*pos].clone() {
            Event::Start(Tag::TableHead) => {
                *pos += 1;
                header = parse_table_row(events, pos);
            }
            Event::Start(Tag::TableRow) => {
                rows.push(parse_table_row(events, pos));
            }
            Event::End(TagEnd::Table) => {
                *pos += 1;
                break;
            }
            _ => *pos += 1,
        }
    }

    TableBlock {
        header,
        rows,
        aligns,
    }
}

fn parse_table_row(events: &[Event<'_>], pos: &mut usize) -> Vec<Cell> {
    let mut cells = Vec::new();
    if *pos < events.len() && matches!(events[*pos], Event::Start(Tag::TableRow)) {
        *pos += 1;
    }

    while *pos < events.len() {
        match events[*pos].clone() {
            Event::Start(Tag::TableCell) => {
                *pos += 1;
                cells.push(Cell {
                    inlines: parse_inlines(events, pos, TagEnd::TableCell),
                });
            }
            Event::End(TagEnd::TableHead) => {
                *pos += 1;
                break;
            }
            Event::End(TagEnd::TableRow) => {
                *pos += 1;
                break;
            }
            _ => *pos += 1,
        }
    }
    cells
}

// ---------------------------------------------------------------------------
// テキスト化・HTML 化
// ---------------------------------------------------------------------------

fn inline_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(value) | Inline::Code(value) => text.push_str(value),
            Inline::Html(_) => {}
            Inline::Strong(children)
            | Inline::Emphasis(children)
            | Inline::Strike(children) => text.push_str(&inline_text(children)),
            Inline::Link { children, .. } => text.push_str(&inline_text(children)),
            Inline::Image { alt, .. } => text.push_str(alt),
            Inline::SoftBreak => text.push(' '),
            Inline::HardBreak => text.push(' '),
            Inline::FootnoteRef(name) => text.push_str(name),
        }
    }
    text
}

fn block_text(blocks: &[Block]) -> String {
    let mut text = String::new();
    for block in blocks {
        match block {
            Block::Heading(h) => text.push_str(&inline_text(&h.inlines)),
            Block::Paragraph(p) => text.push_str(&inline_text(&p.inlines)),
            Block::Quote(inner) => text.push_str(&block_text(inner)),
            Block::List(list) => {
                for item in &list.items {
                    text.push_str(&block_text(item));
                }
            }
            Block::Code { text: code, .. } => text.push_str(code),
            Block::Table(table) => {
                for cell in &table.header {
                    text.push_str(&inline_text(&cell.inlines));
                }
                for row in &table.rows {
                    for cell in row {
                        text.push_str(&inline_text(&cell.inlines));
                    }
                }
            }
            Block::Rule | Block::Html(_) | Block::Meta(_) => {}
        }
    }
    text
}

#[derive(Clone, Copy)]
enum StrongStyle {
    Default,
    Answer,
    Comment,
}

fn render_inlines(inlines: &[Inline], strong: StrongStyle) -> String {
    let mut html = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(text) => html.push_str(&encode_text(text)),
            Inline::Code(code) => {
                html.push_str(&format!(
                    "<code style=\"{CODE}\">{}</code>",
                    encode_text(code)
                ));
            }
            Inline::Strong(children) => {
                let style = match strong {
                    StrongStyle::Default => "font-weight:700;",
                    StrongStyle::Answer => "color:inherit;font-weight:700;",
                    StrongStyle::Comment => "color:#b45309;font-weight:700;",
                };
                html.push_str(&format!(
                    "<strong style=\"{style}\">{}</strong>",
                    render_inlines(children, strong)
                ));
            }
            Inline::Emphasis(children) => {
                html.push_str("<em>");
                html.push_str(&render_inlines(children, strong));
                html.push_str("</em>");
            }
            Inline::Strike(children) => {
                html.push_str("<del>");
                html.push_str(&render_inlines(children, strong));
                html.push_str("</del>");
            }
            Inline::Link {
                href,
                title,
                children,
            } => {
                html.push_str(&format!(
                    "<a href=\"{}\" style=\"{LINK}\"",
                    encode_double_quoted_attribute(href)
                ));
                if !title.is_empty() {
                    html.push_str(&format!(
                        " title=\"{}\"",
                        encode_double_quoted_attribute(title)
                    ));
                }
                html.push('>');
                html.push_str(&render_inlines(children, strong));
                html.push_str("</a>");
            }
            Inline::Image { src, title, alt } => {
                html.push_str(&format!(
                    "<img src=\"{}\" alt=\"{}\" style=\"{IMG}\"",
                    encode_double_quoted_attribute(src),
                    encode_double_quoted_attribute(alt)
                ));
                if !title.is_empty() {
                    html.push_str(&format!(
                        " title=\"{}\"",
                        encode_double_quoted_attribute(title)
                    ));
                }
                html.push_str(" loading=\"lazy\">");
            }
            Inline::Html(raw) => html.push_str(raw),
            Inline::SoftBreak => html.push('\n'),
            Inline::HardBreak => html.push_str("<br>\n"),
            Inline::FootnoteRef(name) => {
                html.push_str(&format!(
                    "<sup id=\"fnref-{}\">{}</sup>",
                    encode_double_quoted_attribute(name),
                    encode_text(name)
                ));
            }
        }
    }
    html
}

fn render_paragraph(paragraph: &Paragraph, context: ParaContext) -> (String, bool) {
    let text = inline_text(&paragraph.inlines);

    let (style, strong, extra, is_answer) = match context {
        ParaContext::ListItem => (LI_P, StrongStyle::Default, "", false),
        ParaContext::Normal => {
            if text.starts_with("解答：") {
                (ANSWER, StrongStyle::Answer, " data-answer=\"1\"", true)
            } else if text.starts_with("採点講評") {
                (COMMENT, StrongStyle::Comment, "", false)
            } else if paragraph.inlines.len() == 1
                && matches!(paragraph.inlines[0], Inline::Strong(_))
            {
                (LABEL, StrongStyle::Default, "", false)
            } else {
                (P, StrongStyle::Default, "", false)
            }
        }
    };

    let html = format!(
        "<p style=\"{style}\"{extra}>{}</p>",
        render_inlines(&paragraph.inlines, strong)
    );
    (html, is_answer)
}

#[derive(Clone, Copy)]
enum ParaContext {
    Normal,
    ListItem,
}

fn heading_style(level: u8, text: &str) -> &'static str {
    match level {
        1 => H1,
        2 => H2,
        3 => {
            if text.contains("問題不備") {
                H3_WARN
            } else {
                H3
            }
        }
        4 => H4,
        _ => H5,
    }
}

fn render_heading(heading: &Heading) -> String {
    let text = inline_text(&heading.inlines);
    let style = heading_style(heading.level, &text);
    let id = heading
        .id
        .as_deref()
        .map(|id| format!(" id=\"{}\"", encode_double_quoted_attribute(id)))
        .unwrap_or_default();
    let title_attr = if heading.article_title {
        " data-role=\"article-title\""
    } else {
        ""
    };
    format!(
        "<h{level}{id} style=\"{style}\"{title_attr}>{}</h{level}>",
        render_inlines(&heading.inlines, StrongStyle::Default),
        level = heading.level
    )
}

fn render_list(list: &ListBlock, nested: bool, stats: &mut Stats) -> String {
    let tag = if list.ordered { "ol" } else { "ul" };
    let style = if nested { LIST_NESTED } else { LIST };
    let start = if list.ordered && list.start != 1 {
        format!(" start=\"{}\"", list.start)
    } else {
        String::new()
    };

    let mut html = format!("<{tag} style=\"{style}\"{start}>");
    for item in &list.items {
        html.push_str(&format!("<li style=\"{LI}\">"));
        html.push_str(&render_blocks(item, true, stats));
        html.push_str("</li>");
    }
    html.push_str(&format!("</{tag}>"));
    html
}

fn render_table(table: &TableBlock, stats: &mut Stats) -> String {
    stats.tables += 1;
    let ncols = table.header.len().max(
        table
            .rows
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0),
    );

    let mut table_style = TABLE.to_string();
    if ncols >= 8 {
        table_style = table_style.replace("font-size:14px;", "font-size:12.5px;");
    }
    if ncols >= 6 {
        table_style.push_str(&format!("min-width:{}px;", (ncols * 72).max(420)));
    }

    let mut html = String::new();
    if ncols >= 7 {
        html.push_str(&format!(
            "<div style=\"{TABLE_HINT}\">※ 横にスクロールできます</div>"
        ));
    }
    html.push_str(&format!("<div style=\"{TABLE_WRAP}\">"));
    html.push_str(&format!("<table style=\"{table_style}\"><thead><tr>"));
    for cell in &table.header {
        html.push_str(&format!(
            "<th style=\"{TH}\">{}</th>",
            render_inlines(&cell.inlines, StrongStyle::Default)
        ));
    }
    html.push_str("</tr></thead><tbody>");
    for (index, row) in table.rows.iter().enumerate() {
        let background = if index % 2 == 0 { "#ffffff" } else { "#f6f9fc" };
        html.push_str("<tr>");
        for cell in row {
            html.push_str(&format!(
                "<td style=\"{TD}background-color:{background};\">{}</td>",
                render_inlines(&cell.inlines, StrongStyle::Default)
            ));
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table></div>");
    html
}

fn render_blocks(blocks: &[Block], in_list: bool, stats: &mut Stats) -> String {
    let mut rendered: Vec<String> = Vec::new();

    for block in blocks {
        match block {
            Block::Heading(heading) => {
                if heading.id.is_some() {
                    stats.headings += 1;
                }
                rendered.push(render_heading(heading));
            }
            Block::Paragraph(paragraph) => {
                if paragraph.bare {
                    rendered.push(render_inlines(
                        &paragraph.inlines,
                        StrongStyle::Default,
                    ));
                    continue;
                }
                let context = if in_list {
                    ParaContext::ListItem
                } else {
                    ParaContext::Normal
                };
                let (html, is_answer) = render_paragraph(paragraph, context);
                if is_answer {
                    stats.answers += 1;
                }
                rendered.push(html);
            }
            Block::Quote(inner) => {
                let text = block_text(inner);
                let style = if text.contains("不備") || text.contains("注意") {
                    WARN
                } else {
                    NOTE
                };
                let paragraphs: Vec<&Block> = inner
                    .iter()
                    .filter(|block| matches!(block, Block::Paragraph(_)))
                    .collect();
                let mut html = format!("<blockquote style=\"{style}\">");
                let mut paragraph_index = 0;
                for child in inner {
                    if let Block::Paragraph(paragraph) = child {
                        let margin = if paragraph_index + 1 == paragraphs.len() {
                            "0"
                        } else {
                            "0 0 .6em"
                        };
                        html.push_str(&format!(
                            "<p style=\"margin:{margin};color:inherit;font-size:inherit;line-height:inherit;\">{}</p>",
                            render_inlines(&paragraph.inlines, StrongStyle::Default)
                        ));
                        paragraph_index += 1;
                    } else {
                        html.push_str(&render_blocks(
                            std::slice::from_ref(child),
                            false,
                            stats,
                        ));
                    }
                }
                html.push_str("</blockquote>");
                rendered.push(html);
            }
            Block::List(list) => {
                rendered.push(render_list(list, in_list, stats));
            }
            Block::Code { lang, text } => {
                stats.code_blocks += 1;
                let class = if lang.is_empty() {
                    String::new()
                } else {
                    format!(
                        " class=\"language-{}\"",
                        encode_double_quoted_attribute(lang)
                    )
                };
                rendered.push(format!(
                    "<pre style=\"{PRE}\"><code{class} style=\"margin:0;padding:0;border:0;background:none;color:inherit;font-family:inherit;font-size:inherit;white-space:pre;\">{}</code></pre>",
                    encode_text(text)
                ));
            }
            Block::Table(table) => rendered.push(render_table(table, stats)),
            Block::Rule => rendered.push(format!("<hr style=\"{HR}\">")),
            Block::Html(raw) => rendered.push(raw.clone()),
            Block::Meta(paragraphs) => {
                let mut html = format!("<div style=\"{META}\">");
                for paragraph in paragraphs {
                    html.push_str(&format!(
                        "<p style=\"{META_P}\">{}</p>",
                        render_inlines(&paragraph.inlines, StrongStyle::Default)
                    ));
                }
                html.push_str("</div>");
                rendered.push(html);
            }
        }
    }

    rendered.join("\n")
}

// ---------------------------------------------------------------------------
// 変換（Python 版の後処理に相当）
// ---------------------------------------------------------------------------

pub fn split_title(md_text: &str) -> (String, Option<String>) {
    let mut lines: Vec<&str> = md_text.lines().collect();
    let mut in_fence = false;

    for i in 0..lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence && line.starts_with("# ") {
            let rest = line[2..].trim();
            if !rest.is_empty() {
                let title = rest.to_string();
                lines.remove(i);
                return (lines.join("\n"), Some(title));
            }
        }
    }

    (md_text.to_string(), None)
}

fn walk_headings_mut<F: FnMut(&mut Heading)>(blocks: &mut [Block], callback: &mut F) {
    for block in blocks {
        match block {
            Block::Heading(heading) => callback(heading),
            Block::Quote(inner) => walk_headings_mut(inner, callback),
            Block::List(list) => {
                for item in &mut list.items {
                    walk_headings_mut(item, callback);
                }
            }
            _ => {}
        }
    }
}

fn normalize_headings(blocks: &mut [Block]) {
    let mut seen_h1 = false;
    walk_headings_mut(blocks, &mut |heading| {
        if heading.article_title {
            return;
        }
        if heading.level == 1 {
            seen_h1 = true;
        }
        if seen_h1 {
            heading.level = (heading.level + 1).min(6);
        }
        let text = inline_text(&heading.inlines);
        if heading.level == 3
            && (text.starts_with("付録") || text.starts_with("Appendix"))
        {
            heading.level = 2;
        }
    });
}

fn wrap_meta(blocks: &mut Vec<Block>) {
    let mut index = 0;
    // 本文に残した記事タイトルは飛ばす
    while index < blocks.len() {
        match &blocks[index] {
            Block::Heading(heading) if heading.level == 1 => index += 1,
            _ => break,
        }
    }

    let start = index;
    while index < blocks.len() {
        if let Block::Paragraph(paragraph) = &blocks[index] {
            let text = inline_text(&paragraph.inlines);
            if text.starts_with("対象：") || text.starts_with("問題：") {
                index += 1;
                continue;
            }
        }
        break;
    }

    if index > start {
        let paragraphs: Vec<Paragraph> = blocks[start..index]
            .iter()
            .filter_map(|block| match block {
                Block::Paragraph(paragraph) => Some(paragraph.clone()),
                _ => None,
            })
            .collect();
        blocks.splice(start..index, [Block::Meta(paragraphs)]);
    }
}

fn insert_widget(blocks: &mut Vec<Block>) {
    let position = blocks.iter().position(|block| match block {
        Block::Heading(heading) => {
            inline_text(&heading.inlines).starts_with("図4の対応表")
        }
        _ => false,
    });

    if let Some(index) = position {
        blocks.insert(index, Block::Html(vxlan_puzzle_html()));
    }
}

fn assign_heading_ids(blocks: &mut [Block], counter: &mut usize, toc: &mut Vec<TocEntry>) {
    for block in blocks {
        match block {
            Block::Heading(heading) => {
                if (2..=4).contains(&heading.level) {
                    *counter += 1;
                    let id = format!("sec-{counter}");
                    heading.id = Some(id.clone());
                    if heading.level == 2 || heading.level == 3 {
                        if !heading.article_title {
                            toc.push(TocEntry {
                                id,
                                level: heading.level,
                                text: inline_text(&heading.inlines),
                            });
                        }
                    }
                }
            }
            Block::Quote(inner) => assign_heading_ids(inner, counter, toc),
            Block::List(list) => {
                for item in &mut list.items {
                    assign_heading_ids(item, counter, toc);
                }
            }
            _ => {}
        }
    }
}

struct TocEntry {
    id: String,
    level: u8,
    text: String,
}

fn build_toc(toc: &[TocEntry]) -> String {
    let mut html = format!(
        "<div role=\"navigation\" aria-label=\"目次\" style=\"{TOC}\">\n<div style=\"{TOC_TITLE}\">目次</div>\n<ol style=\"{TOC_OL}\">\n"
    );
    let mut in_sub = false;
    let mut has_top = false;

    for entry in toc {
        let mut style = TOC_LINK.to_string();
        if entry.text.contains("問題不備") {
            style.push_str("color:#b91c1c;font-weight:700;");
        }
        let link = format!(
            "<li style=\"{TOC_LI}\"><a href=\"#{}\" style=\"{style}\">{}</a></li>",
            encode_double_quoted_attribute(&entry.id),
            encode_text(&entry.text)
        );

        if entry.level == 2 {
            if in_sub {
                html.push_str("</ul>\n");
                in_sub = false;
            }
            html.push_str(&link);
            html.push('\n');
            has_top = true;
        } else if !has_top {
            // h2 より先に h3 が現れた場合は、入れ子にせずトップへ置く
            html.push_str(&link);
            html.push('\n');
        } else {
            if !in_sub {
                html.push_str(&format!("<ul style=\"{TOC_UL}\">\n"));
                in_sub = true;
            }
            html.push_str(&link);
            html.push('\n');
        }
    }

    if in_sub {
        html.push_str("</ul>\n");
    }
    html.push_str("</ol>\n</div>");
    html
}

fn insert_toc(blocks: &mut Vec<Block>, toc: &[TocEntry]) -> usize {
    if toc.is_empty() {
        return 0;
    }
    let nav = build_toc(toc);

    let Some(first) = blocks.iter().position(|block| match block {
        Block::Heading(heading) => (2..=3).contains(&heading.level)
            && !heading.article_title
            && heading.id.is_some(),
        _ => false,
    }) else {
        return 0;
    };

    let insert_at = if first > 0 && matches!(blocks[first - 1], Block::Rule) {
        first - 1
    } else {
        first
    };
    blocks.insert(insert_at, Block::Html(nav));
    toc.len()
}

fn build_related_html(related: &[RelatedCard]) -> String {
    let mut html = format!(
        "<div data-role=\"related-articles\" style=\"{RELATED_WRAP}\">\n<p style=\"{RELATED_INTRO}\">この記事と内容が近い過去記事を、ブログ内の記事から自動で選びました。</p>\n"
    );

    for item in related {
        let badge = if item.categories.is_empty() {
            "記事".to_string()
        } else {
            item.categories
                .iter()
                .take(2)
                .cloned()
                .collect::<Vec<_>>()
                .join("・")
        };
        let date = item.published.chars().take(10).collect::<String>();
        let date = date.replace('-', ".");

        html.push_str(&format!(
            "<a href=\"{}\" target=\"_blank\" rel=\"noopener\" data-related-card=\"1\" style=\"{RELATED_CARD}\">\n<div style=\"{RELATED_TOP}\"><span style=\"{RELATED_BADGE}\">{}</span>",
            encode_double_quoted_attribute(&item.url),
            encode_text(&badge)
        ));
        if !date.is_empty() {
            html.push_str(&format!(
                "<span style=\"{RELATED_DATE}\">{}</span>",
                encode_text(&date)
            ));
        }
        html.push_str(&format!(
            "</div>\n<div style=\"{RELATED_TITLE}\">{}<span aria-hidden=\"true\" style=\"{RELATED_ARROW}\"> →</span></div>",
            encode_text(&item.title)
        ));
        if !item.summary_text.is_empty() {
            html.push_str(&format!(
                "\n<p style=\"{RELATED_SUMMARY}\">{}</p>",
                encode_text(&item.summary_text)
            ));
        }
        html.push_str("\n</a>\n");
    }

    html.push_str(&format!(
        "<p style=\"{RELATED_NOTE}\">※ 本文と過去記事のタイトル・概要の類似度による自動選定です。</p>\n</div>"
    ));
    html
}

fn vxlan_puzzle_html() -> String {
    const CHIP: &str = "appearance:none;border:1px solid #93c5fd;border-radius:999px;padding:8px 14px;background-color:#ffffff;color:#1e3a8a;font-family:inherit;font-size:12.5px;font-weight:700;line-height:1.4;cursor:pointer;transition:transform .12s ease,background .12s ease,opacity .12s ease;";
    const SUB_BUTTON: &str = "appearance:none;border:1px solid #cbd5e1;border-radius:8px;padding:5px 12px;background-color:#ffffff;color:#475569;font-family:inherit;font-size:11.5px;font-weight:700;line-height:1.5;cursor:pointer;transition:background .15s ease,color .15s ease;";
    const SLOT: &str = "display:inline-flex;align-items:center;justify-content:center;min-width:92px;min-height:36px;padding:6px 10px;border-radius:8px;border:1px dashed #93c5fd;background-color:#f8fafc;color:#1e3a8a;font-size:12px;font-weight:700;text-align:center;line-height:1.35;";
    const ARROW: &str = "color:#93c5fd;font-size:12px;font-weight:700;";

    let mut slots = String::new();
    for index in 0..4 {
        slots.push_str(&format!("<span style=\"{SLOT}\">？</span>"));
        if index < 3 {
            slots.push_str(&format!(
                "<span aria-hidden=\"true\" style=\"{ARROW}\">→</span>"
            ));
        }
    }

    format!(
        r#"<div data-widget="vxlan-puzzle" style="margin:26px 0 30px;padding:18px 20px;border:1px solid #bfdbfe;border-radius:12px;background-color:#f8fbff;background-image:linear-gradient(180deg,#ffffff 0%,#eff6ff 100%);">
  <div style="display:flex;flex-wrap:wrap;align-items:center;gap:8px;margin:0 0 8px;">
    <span style="display:inline-block;padding:3px 9px;border-radius:999px;background-color:#1e3a8a;color:#ffffff;font-size:10.5px;font-weight:700;letter-spacing:.12em;">MINI QUIZ</span>
    <span style="font-size:14px;font-weight:700;color:#1e3a8a;">VXLANカプセル化の順番</span>
  </div>
  <p style="margin:0 0 12px;font-size:13px;line-height:1.75;color:#334155;">外側（トンネルの入口）から内側へ、4つの層を順番にクリックしてください。</p>
  <div style="display:flex;justify-content:space-between;gap:12px;margin:0 0 6px;font-size:10.5px;font-weight:700;color:#64748b;">
    <span>外側（トンネル入口）</span><span>内側（元のフレーム）</span>
  </div>
  <div data-role="slots" aria-label="選択した順番" style="display:flex;flex-wrap:wrap;align-items:center;gap:6px;margin:0 0 14px;min-height:38px;">{slots}</div>
  <div data-role="chips" style="display:flex;flex-wrap:wrap;gap:8px;margin:0 0 12px;">
    <button type="button" data-order="3" style="{chip}">VXLANヘッダー</button>
    <button type="button" data-order="4" style="{chip}">イーサネットフレーム</button>
    <button type="button" data-order="1" style="{chip}">IPv4ヘッダー</button>
    <button type="button" data-order="2" style="{chip}">UDPヘッダー</button>
  </div>
  <div style="display:flex;flex-wrap:wrap;align-items:center;gap:8px;">
    <button type="button" data-role="reset" style="{sub}">やり直す</button>
    <button type="button" data-role="hint" style="{sub}">ヒント</button>
    <span data-role="hint-text" style="display:none;font-size:11.5px;line-height:1.65;color:#475569;">いちばん外側は「宛先 IP アドレス」を運ぶヘッダー。いちばん内側は、もともと送りたかったフレームです。</span>
  </div>
  <div data-role="status" role="status" aria-live="polite" style="margin:10px 0 0;min-height:1.65em;font-size:12.5px;font-weight:700;line-height:1.65;"></div>
  <noscript><p style="margin:10px 0 0;font-size:11.5px;line-height:1.6;color:#64748b;">このミニクイズは JavaScript を有効にすると遊べます。</p></noscript>
</div>"#,
        chip = CHIP,
        sub = SUB_BUTTON
    )
}

pub fn build_fragment(md_text: &str, options: RenderOptions<'_>) -> Fragment {
    let events: Vec<Event<'_>> = Parser::new_ext(md_text, markdown_options()).collect();
    let mut position = 0;
    let mut blocks = parse_blocks(&events, &mut position, Stop::Document);

    if options.keep_title {
        let mut marked = false;
        walk_headings_mut(&mut blocks, &mut |heading| {
            if !marked && heading.level == 1 {
                heading.article_title = true;
                marked = true;
            }
        });
    }

    normalize_headings(&mut blocks);
    wrap_meta(&mut blocks);

    if options.with_widget {
        insert_widget(&mut blocks);
    }

    if !options.related.is_empty() {
        blocks.push(Block::Heading(Heading {
            level: 2,
            inlines: vec![Inline::Text("関連記事".to_string())],
            article_title: false,
            id: None,
        }));
        blocks.push(Block::Html(build_related_html(options.related)));
    }

    let mut stats = Stats::default();
    let mut counter = 0;
    let mut toc = Vec::new();
    assign_heading_ids(&mut blocks, &mut counter, &mut toc);

    if options.with_toc && !toc.is_empty() {
        stats.toc_links = insert_toc(&mut blocks, &toc);
    }

    let body = render_blocks(&blocks, false, &mut stats);

    let comment = format!(
        "<!-- ============================================================\n     はてなブログ「HTML編集」モード用の記事本文\n     生成元: {}\n     記事タイトルははてなブログ側のタイトル欄に設定してください\n     インタラクションが動かない場合は、末尾のスクリプト部分を\n     「デザイン > カスタマイズ > 記事下」へ移動してください\n     ============================================================ -->",
        options.source_name
    );

    let mut html = format!(
        "{comment}\n<div data-hatena-article=\"1\" style=\"{BASE}\">\n{body}\n</div>\n"
    );

    if let Some(script) = options.interactive_js {
        if !script.trim().is_empty() {
            html.push_str(&format!("\n<script>\n{}\n</script>\n", script.trim()));
        }
    }

    stats.bytes = html.len();
    Fragment { html, stats }
}

// ---------------------------------------------------------------------------
// プレビュー HTML
// ---------------------------------------------------------------------------

pub fn build_preview(title: &str, fragment: &str, include_title: bool) -> String {
    let title_block = if include_title {
        format!(
            "<h1 class=\"article-title\">{}</h1>",
            encode_text(title)
        )
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; background: #eef2f7; }}
  .page {{
    max-width: 900px;
    margin: 40px auto;
    padding: 48px 56px;
    background: #ffffff;
    border-radius: 12px;
    box-shadow: 0 10px 35px rgba(15, 23, 42, .08);
  }}
  .article-title {{
    margin: 0 0 28px;
    padding: 0 0 18px;
    border-bottom: 3px solid #1e3a8a;
    color: #0f172a;
    font-family: {font};
    font-size: 28px;
    line-height: 1.45;
    font-weight: 700;
    letter-spacing: .01em;
  }}
  @media (max-width: 720px) {{
    .page {{ margin: 0; padding: 28px 18px; border-radius: 0; }}
    .article-title {{ font-size: 22px; }}
  }}
</style>
</head>
<body>
<main class="page">
{title_block}
{fragment}
</main>
</body>
</html>
"#,
        title = encode_text(title),
        font = FONT,
        title_block = title_block,
        fragment = fragment
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(md: &str) -> String {
        build_fragment(
            md,
            RenderOptions {
                keep_title: false,
                with_toc: false,
                with_widget: false,
                related: &[],
                source_name: "probe.md",
                interactive_js: None,
            },
        )
        .html
    }

    #[test]
    fn renders_bold_before_japanese_particle() {
        let html = render("- 集め、**LSDB（Link State Database）**を保持します。");
        assert!(html.contains("<strong"));
        assert!(!html.contains("**"));
    }

    #[test]
    fn keeps_tight_list_text() {
        let html = render("- 一つ目\n- 二つ目\n");
        assert!(html.contains("一つ目"));
        assert!(html.contains("二つ目"));
    }

    #[test]
    fn renders_table_cells() {
        let html = render("| A | B |\n|---|---|\n| 1 | 2 |\n");
        assert!(html.contains("<table"));
        assert!(html.contains("<td"));
        assert!(html.contains(">1<"));
    }
}

