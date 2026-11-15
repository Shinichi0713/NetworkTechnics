//! 過去記事の関連度スコアリング。
//!
//! TF-IDF コサイン類似度、ドメインキーワードの IDF 重み付き一致、
//! タイトル一致の加点を Python 版と同じ式で計算する。

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::feed::FeedEntry;
use crate::markdown::RelatedCard;
use crate::text::{
    TITLE_TOPIC_KEYWORDS, cosine_similarity, extract_domain_keywords, merge_token_counters,
    normalize_search_text, plain_summary_text, weighted_token_counter,
};

static HEADING_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^#{1,6}\s+(.*)$").unwrap());

pub struct SelectOptions {
    pub count: usize,
    pub min_score: f64,
    pub exclude_urls: HashSet<String>,
}

fn strip_whitespace(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

pub fn select_related_articles(
    title: &str,
    article_md: &str,
    entries: &[FeedEntry],
    options: &SelectOptions,
) -> Vec<RelatedCard> {
    if entries.is_empty() {
        return Vec::new();
    }

    let normalized_title = strip_whitespace(&normalize_search_text(title));

    let candidates: Vec<&FeedEntry> = entries
        .iter()
        .filter(|entry| {
            let url = entry.url.trim();
            let entry_title = entry.title.trim();
            if url.is_empty() || entry_title.is_empty() {
                return false;
            }
            if options.exclude_urls.contains(url) {
                return false;
            }
            if !normalized_title.is_empty()
                && strip_whitespace(&normalize_search_text(entry_title)) == normalized_title
            {
                return false;
            }
            true
        })
        .collect();

    if candidates.is_empty() {
        return Vec::new();
    }

    let headings = HEADING_LINE_RE
        .captures_iter(article_md)
        .filter_map(|capture| capture.get(1).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>()
        .join("\n");

    let article_doc = merge_token_counters(&[
        weighted_token_counter(title, 4),
        weighted_token_counter(&headings, 3),
        weighted_token_counter(article_md, 1),
    ]);

    let candidate_docs: Vec<HashMap<String, f64>> = candidates
        .iter()
        .map(|entry| {
            merge_token_counters(&[
                weighted_token_counter(&entry.title, 4),
                weighted_token_counter(&entry.categories.join(" "), 3),
                weighted_token_counter(&entry.summary, 1),
                weighted_token_counter(&entry.content_text, 2),
            ])
        })
        .collect();

    let total_documents = candidates.len() + 1;
    let mut document_freq: HashMap<String, usize> = HashMap::new();
    document_freq.extend(article_doc.keys().map(|token| (token.clone(), 1)));
    for doc in &candidate_docs {
        for token in doc.keys() {
            *document_freq.entry(token.clone()).or_insert(0) += 1;
        }
    }
    let idf: HashMap<String, f64> = document_freq
        .iter()
        .map(|(token, frequency)| {
            (
                token.clone(),
                ((1.0 + total_documents as f64) / (1.0 + *frequency as f64)).ln() + 1.0,
            )
        })
        .collect();

    let to_vector = |document: &HashMap<String, f64>| -> HashMap<String, f64> {
        document
            .iter()
            .filter(|(_, value)| **value > 0.0)
            .map(|(token, value)| {
                (
                    token.clone(),
                    (1.0 + value.ln()) * idf.get(token).copied().unwrap_or(1.0),
                )
            })
            .collect()
    };

    let article_vector = to_vector(&article_doc);
    let article_keywords = extract_domain_keywords(&format!("{title}\n{article_md}"));

    let entry_keyword_sets: Vec<HashSet<&'static str>> = candidates
        .iter()
        .map(|entry| {
            let text = format!(
                "{} {} {} {}",
                entry.title,
                entry.categories.join(" "),
                entry.summary,
                entry.content_text
            );
            extract_domain_keywords(&text)
        })
        .collect();

    let mut keyword_freq: HashMap<&'static str, usize> = HashMap::new();
    for keywords in &entry_keyword_sets {
        for keyword in keywords {
            *keyword_freq.entry(keyword).or_insert(0) += 1;
        }
    }
    let keyword_idf: HashMap<&'static str, f64> = keyword_freq
        .iter()
        .map(|(keyword, frequency)| {
            (
                *keyword,
                ((1.0 + candidates.len() as f64) / (1.0 + *frequency as f64)).ln() + 1.0,
            )
        })
        .collect();

    let title_keywords: HashSet<&'static str> = TITLE_TOPIC_KEYWORDS.iter().copied().collect();

    let mut scored: Vec<RelatedCard> = Vec::new();

    for (entry, document, entry_keywords) in candidates
        .iter()
        .zip(&candidate_docs)
        .zip(&entry_keyword_sets)
        .map(|((entry, document), keywords)| (entry, document, keywords))
    {
        let vector = to_vector(document);
        let cosine = cosine_similarity(&article_vector, &vector);

        let intersection: HashSet<_> = article_keywords
            .intersection(entry_keywords)
            .copied()
            .collect();
        let union: HashSet<_> = article_keywords.union(entry_keywords).copied().collect();
        let weighted_union: f64 = union
            .iter()
            .map(|keyword| keyword_idf.get(keyword).copied().unwrap_or(1.0))
            .sum();
        let keyword_score = if weighted_union > 0.0 {
            intersection
                .iter()
                .map(|keyword| keyword_idf.get(keyword).copied().unwrap_or(1.0))
                .sum::<f64>()
                / weighted_union
        } else {
            0.0
        };

        let entry_title_keywords = extract_domain_keywords(&entry.title);
        let title_hits = entry_title_keywords
            .intersection(&article_keywords)
            .filter(|keyword| title_keywords.contains(*keyword))
            .count();
        let title_bonus = 0.10 * title_hits.min(2) as f64;

        let mut score = 0.68 * cosine + 0.32 * keyword_score + title_bonus;
        if entry
            .categories
            .iter()
            .any(|category| category == "ネットワーク")
        {
            score += 0.02;
        }
        if score < options.min_score {
            continue;
        }

        scored.push(RelatedCard {
            title: entry.title.clone(),
            url: entry.url.clone(),
            published: entry.published.clone(),
            categories: entry.categories.clone(),
            summary_text: plain_summary_text(&entry.summary, 118),
            score,
        });
    }

    scored.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| right.published.cmp(&left.published))
    });
    scored.truncate(options.count);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str, url: &str, summary: &str) -> FeedEntry {
        FeedEntry {
            title: title.to_string(),
            url: url.to_string(),
            published: "2024-01-01T00:00:00+09:00".to_string(),
            categories: vec!["ネットワーク".to_string()],
            summary: summary.to_string(),
            content_text: summary.to_string(),
        }
    }

    #[test]
    fn excludes_article_itself_by_title() {
        let entries = vec![entry("VXLANの話", "https://example.com/1", "VXLAN")];
        let cards = select_related_articles(
            "VXLANの話",
            "VXLANについて",
            &entries,
            &SelectOptions {
                count: 5,
                min_score: 0.0,
                exclude_urls: HashSet::new(),
            },
        );
        assert!(cards.is_empty());
    }

    #[test]
    fn selects_a_related_entry() {
        let entries = vec![
            entry(
                "ネットワーク拡張技術VXLAN",
                "https://example.com/vxlan",
                "VXLANのカプセル化とVTEPについて",
            ),
            entry("料理の話", "https://example.com/cook", "今日の夕飯"),
        ];
        let cards = select_related_articles(
            "VXLANとEVPNの解説",
            "VXLANのカプセル化、VTEP、VNIについて解説します。",
            &entries,
            &SelectOptions {
                count: 5,
                min_score: 0.0,
                exclude_urls: HashSet::new(),
            },
        );
        assert_eq!(cards[0].url, "https://example.com/vxlan");
    }
}
