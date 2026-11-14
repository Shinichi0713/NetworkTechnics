//! 検索用テキストの正規化・トークン化・類似度計算。
//!
//! 既存の Python 版 (`md_to_hatena.py`) と同じスコアになるように、
//! NFKC 正規化、日本語の文字バイグラム、TF-IDF コサイン類似度を
//! そのまま移植している。

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use unicode_normalization::UnicodeNormalization;

pub static ASCII_TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[a-z0-9][a-z0-9+#._/-]*").unwrap());
static SCRIPT_STYLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<(script|style)\b[^>]*>.*?</\1>").unwrap()
});
static HTML_TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<[^>]+>").unwrap());
static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static TOC_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^目次\s*").unwrap());
static BGM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[🎵♪]?\s*BGMを再生する\s*").unwrap());

/// 記事の主題を表す語。候補記事とのキーワード一致度に使う。
pub const DOMAIN_KEYWORDS: &[&str] = &[
    "vxlan",
    "evpn",
    "vtep",
    "vni",
    "vlan",
    "bum",
    "arp",
    "macアドレス",
    "ospf",
    "lsdb",
    "bgp",
    "ibgp",
    "mp-bgp",
    "ecmp",
    "l3sw",
    "ルータid",
    "ループバック",
    "クラスターid",
    "マルチキャスト",
    "ユニキャスト",
    "イーサネット",
    "カプセル化",
    "トンネル",
    "アンダーレイ",
    "オーバーレイ",
    "データセンター",
    "レイヤー2",
    "レイヤー3",
    "ipアドレス",
    "サブネット",
    "デフォルトゲートウェイ",
    "ゲートウェイ",
    "ファイアウォール",
    "nat",
    "dns",
    "mxレコード",
    "spf",
    "dkim",
    "s/mime",
    "dmarc",
    "smtp",
    "mail from",
    "ptr",
    "逆引き",
    "電子メール",
    "メールサーバ",
    "証明書",
    "公開鍵",
    "秘密鍵",
    "rsa",
    "デジタル署名",
    "署名",
    "暗号化",
    "なりすまし",
    "送信ドメイン認証",
    "tls",
    "http",
    "パケット",
    "フレーム",
    "プロトコル",
    "ルーティング",
    "経路",
    "冗長化",
    "負荷分散",
];

/// 記事タイトルに含まれていたら強く関連づける語。
pub const TITLE_TOPIC_KEYWORDS: &[&str] = &[
    "vxlan",
    "evpn",
    "vtep",
    "vni",
    "vlan",
    "bum",
    "arp",
    "ospf",
    "lsdb",
    "bgp",
    "ibgp",
    "mp-bgp",
    "ecmp",
    "l3sw",
    "spf",
    "dkim",
    "s/mime",
    "dmarc",
    "smtp",
    "mail from",
    "mxレコード",
    "ptr",
    "カプセル化",
    "トンネル",
    "オーバーレイ",
    "アンダーレイ",
    "データセンター",
    "電子メール",
    "メールサーバ",
    "送信ドメイン認証",
    "なりすまし",
    "ルーティング",
    "ルータid",
    "ループバック",
    "クラスターid",
];

/// 類似度計算用に全角半角・大文字小文字の揺れをならす。
pub fn normalize_search_text(text: &str) -> String {
    text.nfkc().collect::<String>().to_lowercase()
}

/// ASCII の語と日本語の文字バイグラムに分解する。
pub fn tokenize_search_text(text: &str) -> Vec<String> {
    let normalized = normalize_search_text(text);
    let mut tokens = Vec::new();

    for found in ASCII_TOKEN_RE.find_iter(&normalized) {
        let token = found.as_str().trim_matches(|c| c == '.' || c == '_' || c == '/' || c == '-');
        let len = token.chars().count();
        if (2..=32).contains(&len) {
            tokens.push(token.to_string());
        }
    }

    let mut segment = String::new();
    for ch in normalized.chars() {
        if is_japanese(ch) {
            segment.push(ch);
        } else {
            push_segment_bigrams(&segment, &mut tokens);
            segment.clear();
        }
    }
    push_segment_bigrams(&segment, &mut tokens);

    tokens
}

fn is_japanese(ch: char) -> bool {
    matches!(ch, '\u{3040}'..='\u{30ff}' | '\u{4e00}'..='\u{9fff}')
}

fn push_segment_bigrams(segment: &str, tokens: &mut Vec<String>) {
    let chars: Vec<char> = segment.chars().collect();
    if chars.len() < 2 {
        return;
    }
    for pair in chars.windows(2) {
        tokens.push(pair.iter().collect());
    }
}

pub fn weighted_token_counter(text: &str, weight: u32) -> HashMap<String, f64> {
    let mut counts: HashMap<String, f64> = HashMap::new();
    for token in tokenize_search_text(text) {
        *counts.entry(token).or_insert(0.0) += weight as f64;
    }
    counts
}

pub fn merge_token_counters(counters: &[HashMap<String, f64>]) -> HashMap<String, f64> {
    let mut merged = HashMap::new();
    for counter in counters {
        for (token, value) in counter {
            *merged.entry(token.clone()).or_insert(0.0) += value;
        }
    }
    merged
}

pub fn cosine_similarity(left: &HashMap<String, f64>, right: &HashMap<String, f64>) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let (small, large) = if left.len() > right.len() {
        (right, left)
    } else {
        (left, right)
    };

    let dot: f64 = small
        .iter()
        .map(|(token, value)| value * large.get(token).copied().unwrap_or(0.0))
        .sum();
    if dot == 0.0 {
        return 0.0;
    }

    let norm_left = left.values().map(|v| v * v).sum::<f64>().sqrt();
    let norm_right = right.values().map(|v| v * v).sum::<f64>().sqrt();
    if norm_left == 0.0 || norm_right == 0.0 {
        return 0.0;
    }
    dot / (norm_left * norm_right)
}

/// ドメインキーワードのうち、テキストに含まれるものを返す。
pub fn extract_domain_keywords(text: &str) -> HashSet<&'static str> {
    let normalized = normalize_search_text(text);
    let mut found = HashSet::new();

    for &keyword in DOMAIN_KEYWORDS {
        let hit = if keyword.is_ascii() {
            ascii_keyword_present(&normalized, keyword)
        } else {
            normalized.contains(keyword)
        };
        if hit {
            found.insert(keyword);
        }
    }
    found
}

/// ASCII 語を単語境界で判定する。
///
/// Rust の regex には後読みがないため、見つかった位置の前後を自前で調べる。
fn ascii_keyword_present(haystack: &str, keyword: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut offset = 0;
    while let Some(pos) = haystack[offset..].find(keyword) {
        let start = offset + pos;
        let end = start + keyword.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        offset = start + keyword.len().max(1);
        if offset >= haystack.len() {
            break;
        }
    }
    false
}

/// HTML 断片から本文テキストを取り出す。
pub fn html_to_plain_text(raw: &str, limit: Option<usize>) -> String {
    let without_scripts = SCRIPT_STYLE_RE.replace_all(raw, " ");
    let without_tags = HTML_TAG_RE.replace_all(&without_scripts, " ");
    let decoded = html_escape::decode_html_entities(&without_tags);
    let mut text = WHITESPACE_RE.replace_all(&decoded, " ").trim().to_string();
    if let Some(limit) = limit {
        if text.chars().count() > limit {
            text = text.chars().take(limit).collect::<String>();
            text = text.trim_end().to_string();
        }
    }
    text
}

/// フィードの summary をカード用のプレーンテキストにする。
pub fn plain_summary_text(raw: &str, limit: usize) -> String {
    let text = html_to_plain_text(raw, None);
    let text = TOC_PREFIX_RE.replace(&text, "");
    let text = BGM_RE.replace_all(&text, " ");
    let text = WHITESPACE_RE.replace_all(&text, " ").trim().to_string();
    if text.chars().count() > limit {
        let mut shortened: String = text.chars().take(limit).collect();
        shortened = shortened.trim_end().to_string();
        shortened.push('…');
        shortened
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_full_width_ascii() {
        assert_eq!(normalize_search_text("ＶＸＬＡＮ"), "vxlan");
    }

    #[test]
    fn tokenizes_ascii_and_japanese_bigrams() {
        let tokens = tokenize_search_text("VXLANヘッダー");
        assert!(tokens.contains(&"vxlan".to_string()));
        assert!(tokens.contains(&"ヘッ".to_string()));
        assert!(tokens.contains(&"ダー".to_string()));
    }

    #[test]
    fn ascii_keyword_respects_boundaries() {
        assert!(ascii_keyword_present("spfレコード", "spf"));
        assert!(!ascii_keyword_present("ospf", "spf"));
        assert!(ascii_keyword_present("mp-bgp", "bgp"));
    }

    #[test]
    fn strips_html_from_feed_content() {
        let text = html_to_plain_text("<p>Hello <strong>world</strong></p>", None);
        assert_eq!(text, "Hello world");
    }
}
