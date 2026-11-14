//! はてなブログの Atom フィード取得と、関連記事キャッシュの読み書き。
//!
//! キャッシュ JSON は Python 版 (`md_to_hatena.py`) と互換の形式にしている。
//! そのため、どちらのツールで更新しても同じキャッシュを共有できる。

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::text::html_to_plain_text;

pub const RELATED_PAGE_SIZE: usize = 30;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeedEntry {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub published: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub content_text: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FeedCache {
    #[serde(default)]
    pub blog_url: String,
    #[serde(default)]
    pub fetched_at: f64,
    #[serde(default)]
    pub entries: Vec<FeedEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheStatus {
    pub exists: bool,
    pub entries: usize,
    pub age_hours: Option<f64>,
    pub blog_url: Option<String>,
    pub fresh: bool,
}

impl CacheStatus {
    pub fn missing() -> Self {
        Self {
            exists: false,
            entries: 0,
            age_hours: None,
            blog_url: None,
            fresh: false,
        }
    }
}

pub struct BlogEntries {
    pub entries: Vec<FeedEntry>,
    pub logs: Vec<String>,
    pub status: CacheStatus,
    pub from_cache: bool,
}

pub fn now_epoch() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

pub fn read_cache_status(path: &Path, blog_url: &str, max_age_hours: f64) -> CacheStatus {
    let Some(cache) = load_cache(path) else {
        return CacheStatus::missing();
    };
    let age = (now_epoch() - cache.fetched_at).max(0.0) / 3600.0;
    CacheStatus {
        exists: true,
        entries: cache.entries.len(),
        age_hours: Some(age),
        blog_url: Some(cache.blog_url.clone()),
        fresh: cache.blog_url == blog_url && age < max_age_hours && !cache.entries.is_empty(),
    }
}

pub fn load_cache(path: &Path) -> Option<FeedCache> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_cache(path: &Path, blog_url: &str, entries: &[FeedEntry]) -> Result<()> {
    let payload = FeedCache {
        blog_url: blog_url.to_string(),
        fetched_at: now_epoch(),
        entries: entries.to_vec(),
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("キャッシュの保存先を作れません: {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&payload)?;
    fs::write(path, json)
        .with_context(|| format!("キャッシュを書き込めません: {}", path.display()))?;
    Ok(())
}

fn fetch_atom_page(blog_url: &str, page: usize, timeout: Duration) -> Result<Vec<FeedEntry>> {
    let url = format!(
        "{}/feed?page={}",
        blog_url.trim_end_matches('/'),
        page
    );
    let response = ureq::get(&url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (compatible; hatena-studio related-links)",
        )
        .timeout(timeout)
        .call()
        .map_err(|err| anyhow!("{url}: {err}"))?;
    let body = response
        .into_string()
        .map_err(|err| anyhow!("{url}: 本文を読めません: {err}"))?;
    parse_atom(&body).with_context(|| format!("{url} の Atom を解析できません"))
}

fn attribute(element: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
    element
        .try_get_attribute(name)
        .ok()
        .flatten()
        .map(|attr| String::from_utf8_lossy(&attr.value).to_string())
}

fn apply_link_or_category(
    element: &quick_xml::events::BytesStart<'_>,
    current: &mut Option<FeedEntry>,
) {
    let Some(entry) = current.as_mut() else {
        return;
    };
    match element.local_name().as_ref() {
        b"link" => {
            if entry.url.is_empty() {
                let rel = attribute(element, "rel");
                if matches!(rel.as_deref(), None | Some("alternate")) {
                    if let Some(href) = attribute(element, "href") {
                        if !href.is_empty() {
                            entry.url = href;
                        }
                    }
                }
            }
        }
        b"category" => {
            if let Some(term) = attribute(element, "term") {
                let term = term.trim().to_string();
                if !term.is_empty() {
                    entry.categories.push(term);
                }
            }
        }
        _ => {}
    }
}

fn parse_atom(xml: &str) -> Result<Vec<FeedEntry>> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    #[derive(Clone, Copy, PartialEq)]
    enum Field {
        Title,
        Published,
        Updated,
        Summary,
        Content,
    }

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut entries = Vec::new();
    let mut current: Option<FeedEntry> = None;
    let mut field: Option<Field> = None;

    loop {
        match reader.read_event()? {
            Event::Start(element) => {
                let name = element.local_name();
                match name.as_ref() {
                    b"entry" => {
                        current = Some(FeedEntry::default());
                        field = None;
                    }
                    b"link" | b"category" => {
                        apply_link_or_category(&element, &mut current);
                    }
                    b"title" if current.is_some() => field = Some(Field::Title),
                    b"published" if current.is_some() => field = Some(Field::Published),
                    b"updated" if current.is_some() => field = Some(Field::Updated),
                    b"summary" if current.is_some() => field = Some(Field::Summary),
                    b"content" if current.is_some() => field = Some(Field::Content),
                    _ => {}
                }
            }
            Event::Empty(element) => {
                apply_link_or_category(&element, &mut current);
            }
            Event::Text(text) => {
                if let (Some(entry), Some(active)) = (current.as_mut(), field) {
                    let decoded = quick_xml::escape::unescape(&text)
                        .map(|value| value.into_owned())
                        .unwrap_or_else(|_| text.to_string());
                    match active {
                        Field::Title => entry.title.push_str(&decoded),
                        Field::Published => entry.published.push_str(&decoded),
                        Field::Updated => {
                            if entry.published.is_empty() {
                                entry.published.push_str(&decoded);
                            }
                        }
                        Field::Summary => entry.summary.push_str(&decoded),
                        Field::Content => entry.content_text.push_str(&decoded),
                    }
                }
            }
            Event::CData(text) => {
                if let (Some(entry), Some(active)) = (current.as_mut(), field) {
                    let value = text.to_string();
                    match active {
                        Field::Title => entry.title.push_str(&value),
                        Field::Published => entry.published.push_str(&value),
                        Field::Updated => {
                            if entry.published.is_empty() {
                                entry.published.push_str(&value);
                            }
                        }
                        Field::Summary => entry.summary.push_str(&value),
                        Field::Content => entry.content_text.push_str(&value),
                    }
                }
            }
            Event::End(element) => match element.local_name().as_ref() {
                b"entry" => {
                    if let Some(mut entry) = current.take() {
                        entry.title = entry.title.trim().to_string();
                        entry.published = entry.published.trim().to_string();
                        entry.summary =
                            entry.summary.chars().take(1500).collect::<String>();
                        entry.content_text =
                            html_to_plain_text(&entry.content_text, Some(3000));
                        if !entry.url.trim().is_empty() {
                            entries.push(entry);
                        }
                    }
                    field = None;
                }
                b"title" | b"published" | b"updated" | b"summary" | b"content" => {
                    field = None;
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(entries)
}

fn fetch_related_entries(
    blog_url: &str,
    max_pages: usize,
    logs: &mut Vec<String>,
) -> Result<Vec<FeedEntry>> {
    let mut entries = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for page in 1..=max_pages {
        let batch = match fetch_atom_page(blog_url, page, Duration::from_secs(20)) {
            Ok(batch) => batch,
            Err(err) => {
                if page == 1 {
                    return Err(err);
                }
                logs.push(format!(
                    "[WARN] フィード {page} ページ目を取得できません: {err}"
                ));
                break;
            }
        };

        if batch.is_empty() {
            break;
        }

        let mut added = 0;
        for entry in batch {
            if seen.insert(entry.url.clone()) {
                entries.push(entry);
                added += 1;
            }
        }
        if added == 0 {
            break;
        }

        // 相手サイトへの負荷を抑える
        thread::sleep(Duration::from_millis(100));
    }

    Ok(entries)
}

/// キャッシュを優先し、必要ならフィードを再取得する。
///
/// フィード取得に失敗しても、キャッシュがあればそれを返して処理を続行する。
pub fn get_blog_entries(
    blog_url: &str,
    cache_path: &Path,
    max_pages: usize,
    refresh: bool,
    max_age_hours: f64,
) -> BlogEntries {
    let mut logs = Vec::new();
    let cache = load_cache(cache_path);
    let now = now_epoch();

    let cache_matches_blog = cache
        .as_ref()
        .is_some_and(|c| c.blog_url == blog_url);
    let cache_entries = cache
        .as_ref()
        .filter(|_| cache_matches_blog)
        .map(|c| c.entries.clone())
        .filter(|entries| !entries.is_empty());
    let fetched_at = cache.as_ref().map(|c| c.fetched_at).unwrap_or(0.0);

    let cache_fresh = cache_matches_blog
        && cache_entries.is_some()
        && (now - fetched_at) < max_age_hours * 3600.0;

    let status = CacheStatus {
        exists: cache.is_some(),
        entries: cache_entries.as_ref().map(Vec::len).unwrap_or(0),
        age_hours: cache.as_ref().map(|c| ((now - c.fetched_at) / 3600.0).max(0.0)),
        blog_url: cache.as_ref().map(|c| c.blog_url.clone()),
        fresh: cache_fresh,
    };

    if cache_fresh && !refresh {
        let entries = cache_entries.unwrap_or_default();
        let age = (now - fetched_at).max(0.0) / 3600.0;
        logs.push(format!(
            "[INFO] 関連記事キャッシュを使用: {} 件（取得から {:.1} 時間）",
            entries.len(),
            age
        ));
        return BlogEntries {
            entries,
            logs,
            status,
            from_cache: true,
        };
    }

    match fetch_related_entries(blog_url, max_pages, &mut logs) {
        Ok(entries) => {
            if entries.is_empty() {
                if let Some(cached) = cache_entries {
                    logs.push(
                        "[WARN] フィードが空だったため、古いキャッシュを使用します"
                            .to_string(),
                    );
                    return BlogEntries {
                        entries: cached,
                        logs,
                        status,
                        from_cache: true,
                    };
                }
                return BlogEntries {
                    entries: Vec::new(),
                    logs,
                    status,
                    from_cache: false,
                };
            }

            if let Err(err) = save_cache(cache_path, blog_url, &entries) {
                logs.push(format!("[WARN] 関連記事キャッシュを保存できません: {err}"));
            }
            logs.push(format!("[INFO] 関連記事フィードを取得: {} 件", entries.len()));
            let status = CacheStatus {
                exists: true,
                entries: entries.len(),
                age_hours: Some(0.0),
                blog_url: Some(blog_url.to_string()),
                fresh: true,
            };
            BlogEntries {
                entries,
                logs,
                status,
                from_cache: false,
            }
        }
        Err(err) => {
            if let Some(cached) = cache_entries {
                logs.push(format!(
                    "[WARN] フィード取得に失敗したため、古いキャッシュを使用します: {err}"
                ));
                BlogEntries {
                    entries: cached,
                    logs,
                    status,
                    from_cache: true,
                }
            } else {
                logs.push(format!(
                    "[WARN] フィード取得に失敗したため、関連記事をスキップします: {err}"
                ));
                BlogEntries {
                    entries: Vec::new(),
                    logs,
                    status,
                    from_cache: false,
                }
            }
        }
    }
}

/// キャッシュを無視してフィードを取り直す。
pub fn refresh_cache(
    blog_url: &str,
    cache_path: &Path,
    max_pages: usize,
) -> Result<(usize, Vec<String>)> {
    let mut logs = Vec::new();
    let entries = fetch_related_entries(blog_url, max_pages, &mut logs)?;
    if entries.is_empty() {
        return Err(anyhow!("フィードに記事がありませんでした"));
    }
    save_cache(cache_path, blog_url, &entries)?;
    let count = entries.len();
    logs.push(format!("[INFO] 関連記事フィードを取得: {count} 件"));
    Ok((count, logs))
}

pub fn default_cache_path(workspace: &Path) -> PathBuf {
    workspace.join(".hatena_feed_cache.json")
}
