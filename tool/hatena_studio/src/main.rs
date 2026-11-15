//! Hatena Studio — Markdown をはてなブログ向け HTML に変換するローカル Web UI。

mod feed;
mod markdown;
mod related;
mod text;

use std::collections::HashSet;
use std::fs;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::markdown::RenderOptions;

const DEFAULT_BLOG_URL: &str = "https://yoshishinnze.hatenablog.com/";
const INDEX_HTML: &str = include_str!("../assets/index.html");
const APP_CSS: &str = include_str!("../assets/app.css");
const APP_JS: &str = include_str!("../assets/app.js");
const EMBEDDED_JS: &str = include_str!("../assets/hatena_interactive.js");

// ---------------------------------------------------------------------------
// 設定
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    input: String,
    title: String,
    blog_url: String,
    related: bool,
    related_count: usize,
    related_pages: usize,
    related_min_score: f64,
    cache_hours: f64,
    toc: bool,
    js: bool,
    widget: bool,
    keep_title: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            input: String::new(),
            title: String::new(),
            blog_url: DEFAULT_BLOG_URL.to_string(),
            related: true,
            related_count: 5,
            related_pages: 15,
            related_min_score: 0.02,
            cache_hours: 12.0,
            toc: true,
            js: true,
            widget: true,
            keep_title: false,
        }
    }
}

impl Settings {
    fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("設定を保存できません: {}", path.display()))
    }
}

// ---------------------------------------------------------------------------
// アプリケーション状態
// ---------------------------------------------------------------------------

struct Latest {
    preview: String,
    fragment: String,
    output_path: Option<PathBuf>,
    preview_path: Option<PathBuf>,
    input_path: Option<PathBuf>,
    title: String,
}

struct Inner {
    workspace: PathBuf,
    cache_path: PathBuf,
    settings_path: PathBuf,
    interactive_js: Option<String>,
    latest: Mutex<Latest>,
}

#[derive(Clone)]
struct AppState {
    inner: Arc<Inner>,
}

// ---------------------------------------------------------------------------
// リクエスト / レスポンス
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct ConvertRequest {
    input: String,
    title: String,
    output: String,
    preview: String,
    blog_url: String,
    related: bool,
    related_count: usize,
    related_pages: usize,
    related_min_score: f64,
    cache_hours: f64,
    refresh_related: bool,
    toc: bool,
    js: bool,
    widget: bool,
    keep_title: bool,
    write_preview: bool,
    exclude_urls: Vec<String>,
}

impl Default for ConvertRequest {
    fn default() -> Self {
        let settings = Settings::default();
        Self {
            input: settings.input,
            title: settings.title,
            output: String::new(),
            preview: String::new(),
            blog_url: settings.blog_url,
            related: settings.related,
            related_count: settings.related_count,
            related_pages: settings.related_pages,
            related_min_score: settings.related_min_score,
            cache_hours: settings.cache_hours,
            refresh_related: false,
            toc: settings.toc,
            js: settings.js,
            widget: settings.widget,
            keep_title: settings.keep_title,
            write_preview: true,
            exclude_urls: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct BrowseQuery {
    path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RefreshRequest {
    blog_url: String,
    related_pages: usize,
}

#[derive(Debug, Deserialize)]
struct OpenRequest {
    kind: String,
    path: Option<String>,
}

// ---------------------------------------------------------------------------
// 変換
// ---------------------------------------------------------------------------

fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

fn resolve_input(workspace: &Path, input: &str) -> Result<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("入力 Markdown が指定されていません");
    }
    let path = PathBuf::from(trimmed);
    let path = if path.is_absolute() {
        path
    } else {
        workspace.join(path)
    };
    if !path.is_file() {
        bail!("入力ファイルが見つかりません: {}", path.display());
    }
    Ok(path)
}

fn resolve_output(input: &Path, requested: &str) -> PathBuf {
    let trimmed = requested.trim();
    if trimmed.is_empty() {
        let stem = input
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "article".to_string());
        return input.with_file_name(format!("{stem}_hatena.html"));
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        path
    } else {
        input
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    }
}

fn resolve_preview(input: &Path, requested: &str) -> PathBuf {
    let trimmed = requested.trim();
    if trimmed.is_empty() {
        let stem = input
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "article".to_string());
        return input.with_file_name(format!("{stem}_preview.html"));
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        path
    } else {
        input
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    }
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
        .with_context(|| format!("ファイルを書き込めません: {}", path.display()))
}

fn convert(inner: Arc<Inner>, request: ConvertRequest) -> Result<Value> {
    let started = Instant::now();
    let input = resolve_input(&inner.workspace, &request.input)?;
    let raw = fs::read(&input)
        .with_context(|| format!("入力ファイルを読めません: {}", input.display()))?;
    let md_text = String::from_utf8(raw)
        .with_context(|| format!("入力ファイルが UTF-8 ではありません: {}", input.display()))?;
    let md_text = strip_bom(&md_text).to_string();

    let (body_md, detected_title) = markdown::split_title(&md_text);
    let body_md = if request.keep_title {
        md_text.clone()
    } else {
        body_md
    };
    let title = if request.title.trim().is_empty() {
        detected_title.unwrap_or_else(|| {
            input
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "無題の記事".to_string())
        })
    } else {
        request.title.trim().to_string()
    };

    let mut logs = Vec::new();
    let mut cards = Vec::new();
    let cache_status;

    if request.related {
        let blog_url = if request.blog_url.trim().is_empty() {
            DEFAULT_BLOG_URL.to_string()
        } else {
            request.blog_url.trim().to_string()
        };
        let blog_entries = feed::get_blog_entries(
            &blog_url,
            &inner.cache_path,
            request.related_pages.clamp(1, 50),
            request.refresh_related,
            request.cache_hours.max(0.0),
        );
        logs.extend(blog_entries.logs.clone());
        cache_status = blog_entries.status.clone();

        if !blog_entries.entries.is_empty() {
            let exclude_urls: HashSet<String> = request
                .exclude_urls
                .iter()
                .map(|url| url.trim().to_string())
                .filter(|url| !url.is_empty())
                .collect();
            cards = related::select_related_articles(
                &title,
                &body_md,
                &blog_entries.entries,
                &related::SelectOptions {
                    count: request.related_count.clamp(1, 20),
                    min_score: request.related_min_score,
                    exclude_urls,
                },
            );
        }

        if cards.is_empty() {
            logs.push("[RELATED] 関連記事なし".to_string());
        } else {
            logs.push(format!("[RELATED] {} 件を挿入:", cards.len()));
            for (index, card) in cards.iter().enumerate() {
                logs.push(format!(
                    "  {}. [{:.3}] {} ({})",
                    index + 1,
                    card.score,
                    card.title,
                    card.published.chars().take(10).collect::<String>()
                ));
            }
        }
    } else {
        logs.push("[INFO] 関連記事の挿入はオフです".to_string());
        let blog_url = if request.blog_url.trim().is_empty() {
            DEFAULT_BLOG_URL
        } else {
            request.blog_url.trim()
        };
        cache_status =
            feed::read_cache_status(&inner.cache_path, blog_url, request.cache_hours.max(0.0));
    }

    let fragment = markdown::build_fragment(
        &body_md,
        RenderOptions {
            keep_title: request.keep_title,
            with_toc: request.toc,
            with_widget: request.widget,
            related: &cards,
            source_name: &input
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            interactive_js: if request.js {
                inner.interactive_js.as_deref()
            } else {
                None
            },
        },
    );

    let output_path = resolve_output(&input, &request.output);
    write_text(&output_path, &fragment.html)?;
    logs.push(format!(
        "[OK] はてな用HTML : {} ({:.1} KB)",
        output_path.display(),
        fragment.html.len() as f64 / 1024.0
    ));

    let preview = markdown::build_preview(&title, &fragment.html, !request.keep_title);
    let preview_path = if request.write_preview {
        let path = resolve_preview(&input, &request.preview);
        write_text(&path, &preview)?;
        logs.push(format!(
            "[OK] プレビュー   : {} ({:.1} KB)",
            path.display(),
            preview.len() as f64 / 1024.0
        ));
        Some(path)
    } else {
        None
    };
    logs.push(format!("[TITLE] 推奨タイトル: {title}"));

    {
        let mut latest = inner.latest.lock().unwrap();
        latest.preview = preview;
        latest.fragment = fragment.html.clone();
        latest.output_path = Some(output_path.clone());
        latest.preview_path = preview_path.clone();
        latest.input_path = Some(input.clone());
        latest.title = title.clone();
    }

    Ok(json!({
        "ok": true,
        "title": title,
        "input_path": input.to_string_lossy(),
        "output_path": output_path.to_string_lossy(),
        "preview_path": preview_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        "preview_url": format!("/preview?v={}", SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)),
        "related": cards,
        "logs": logs,
        "stats": fragment.stats,
        "cache": cache_status,
        "bytes": fragment.html.len(),
        "elapsed_ms": started.elapsed().as_millis(),
    }))
}

// ---------------------------------------------------------------------------
// ハンドラ
// ---------------------------------------------------------------------------

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn stylesheet() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        APP_CSS,
    )
}

async fn script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        APP_JS,
    )
}

fn error_response(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(json!({"ok": false, "error": message.into()})),
    )
        .into_response()
}

async fn preview(State(state): State<AppState>) -> Response {
    let latest = state.inner.latest.lock().unwrap();
    if latest.preview.is_empty() {
        return error_response(
            StatusCode::NOT_FOUND,
            "まだプレビューがありません。先に変換してください。",
        );
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(latest.preview.clone()))
        .unwrap()
}

async fn api_latest_html(State(state): State<AppState>) -> Response {
    let latest = state.inner.latest.lock().unwrap();
    if latest.fragment.is_empty() {
        return error_response(
            StatusCode::NOT_FOUND,
            "まだ生成された HTML がありません。",
        );
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from(latest.fragment.clone()))
        .unwrap()
}

async fn api_state(State(state): State<AppState>) -> Response {
    let settings = Settings::load(&state.inner.settings_path);
    let mut settings = settings;
    let recent = recent_markdown(&state.inner.workspace);
    let input_candidate = if !settings.input.trim().is_empty() {
        let path = PathBuf::from(settings.input.trim());
        let path = if path.is_absolute() {
            path
        } else {
            state.inner.workspace.join(path)
        };
        if path.is_file() {
            Some(path)
        } else {
            recent.first().map(|item| PathBuf::from(&item.path))
        }
    } else {
        recent.first().map(|item| PathBuf::from(&item.path))
    };

    if let Some(path) = &input_candidate {
        settings.input = path.to_string_lossy().to_string();
    }

    let cache = feed::read_cache_status(
        &state.inner.cache_path,
        &settings.blog_url,
        settings.cache_hours.max(0.0),
    );

    Json(json!({
        "ok": true,
        "workspace": state.inner.workspace.to_string_lossy(),
        "cache_path": state.inner.cache_path.to_string_lossy(),
        "settings_path": state.inner.settings_path.to_string_lossy(),
        "settings": settings,
        "recent": recent,
        "cache": cache,
        "js_loaded": state.inner.interactive_js.is_some(),
        "js_embedded": state.inner.interactive_js.as_deref() == Some(EMBEDDED_JS),
        "version": env!("CARGO_PKG_VERSION"),
    }))
    .into_response()
}

#[derive(Serialize)]
struct RecentFile {
    name: String,
    path: String,
    size: u64,
    modified: Option<u64>,
}

fn recent_markdown(workspace: &Path) -> Vec<RecentFile> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(workspace) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_markdown = path
            .extension()
            .map(|ext| {
                let ext = ext.to_string_lossy().to_ascii_lowercase();
                ext == "md" || ext == "markdown"
            })
            .unwrap_or(false);
        if !is_markdown {
            continue;
        }
        let metadata = entry.metadata().ok();
        files.push(RecentFile {
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            path: path.to_string_lossy().to_string(),
            size: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
            modified: metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
        });
    }
    files.sort_by(|left, right| right.modified.cmp(&left.modified));
    files.truncate(30);
    files
}

async fn api_browse(State(state): State<AppState>, Query(query): Query<BrowseQuery>) -> Response {
    let requested = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| state.inner.workspace.clone());

    let path = if requested.is_absolute() {
        requested
    } else {
        state.inner.workspace.join(requested)
    };
    let path = if path.is_file() {
        path.parent().unwrap_or(&path).to_path_buf()
    } else {
        path
    };

    let Ok(read_dir) = fs::read_dir(&path) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            format!("フォルダを開けません: {}", path.display()),
        );
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in read_dir.flatten() {
        let entry_path = entry.path();
        let name = entry
            .file_name()
            .to_string_lossy()
            .to_string();
        if entry_path.is_dir() {
            dirs.push(json!({
                "name": name,
                "path": entry_path.to_string_lossy(),
            }));
        } else if entry_path.is_file() {
            let ext = entry_path
                .extension()
                .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            if ext == "md" || ext == "markdown" {
                let metadata = entry.metadata().ok();
                files.push(json!({
                    "name": name,
                    "path": entry_path.to_string_lossy(),
                    "size": metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                    "modified": metadata
                        .as_ref()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs()),
                }));
            }
        }
    }

    dirs.sort_by(|left, right| {
        left["name"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .cmp(&right["name"].as_str().unwrap_or("").to_lowercase())
    });
    files.sort_by(|left, right| {
        right["modified"]
            .as_u64()
            .unwrap_or(0)
            .cmp(&left["modified"].as_u64().unwrap_or(0))
    });

    let parent = path
        .parent()
        .filter(|parent| *parent != path)
        .map(|parent| parent.to_string_lossy().to_string());

    Json(json!({
        "ok": true,
        "path": path.to_string_lossy(),
        "parent": parent,
        "dirs": dirs,
        "files": files,
    }))
    .into_response()
}

async fn api_convert(
    State(state): State<AppState>,
    Json(request): Json<ConvertRequest>,
) -> Response {
    let inner = state.inner.clone();
    let result = tokio::task::spawn_blocking(move || convert(inner, request)).await;
    match result {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(err)) => error_response(StatusCode::BAD_REQUEST, format!("{err:#}")),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("変換処理が異常終了しました: {err}"),
        ),
    }
}

async fn api_refresh_cache(
    State(state): State<AppState>,
    Json(request): Json<RefreshRequest>,
) -> Response {
    let blog_url = if request.blog_url.trim().is_empty() {
        DEFAULT_BLOG_URL.to_string()
    } else {
        request.blog_url.trim().to_string()
    };
    let pages = if request.related_pages == 0 {
        15
    } else {
        request.related_pages.clamp(1, 50)
    };
    let inner = state.inner.clone();
    let blog_url_for_fetch = blog_url.clone();
    let result = tokio::task::spawn_blocking(move || {
        feed::refresh_cache(&blog_url_for_fetch, &inner.cache_path, pages)
    })
    .await;

    match result {
        Ok(Ok((count, logs))) => {
            let settings = Settings::load(&state.inner.settings_path);
            let cache = feed::read_cache_status(
                &state.inner.cache_path,
                &blog_url,
                settings.cache_hours.max(0.0),
            );
            Json(json!({
                "ok": true,
                "count": count,
                "logs": logs,
                "cache": cache,
            }))
            .into_response()
        }
        Ok(Err(err)) => error_response(
            StatusCode::BAD_GATEWAY,
            format!("キャッシュ更新に失敗しました: {err:#}"),
        ),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("キャッシュ更新が異常終了しました: {err}"),
        ),
    }
}

async fn api_settings(
    State(state): State<AppState>,
    Json(settings): Json<Settings>,
) -> Response {
    match settings.save(&state.inner.settings_path) {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("設定を保存できません: {err:#}"),
        ),
    }
}

async fn api_open(Json(request): Json<OpenRequest>) -> Response {
    let Some(path) = request.path.as_deref().filter(|path| !path.is_empty()) else {
        return error_response(StatusCode::BAD_REQUEST, "パスが指定されていません");
    };
    let path = PathBuf::from(path);
    let result = match request.kind.as_str() {
        "select" => reveal_in_explorer(&path),
        "folder" => open_target(if path.is_dir() {
            &path
        } else {
            path.parent().unwrap_or(&path)
        }),
        _ => open_target(&path),
    };
    match result {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("開けませんでした: {err:#}"),
        ),
    }
}

#[cfg(target_os = "windows")]
fn reveal_in_explorer(path: &Path) -> Result<()> {
    Command::new("explorer.exe")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .context("explorer を起動できません")?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn reveal_in_explorer(path: &Path) -> Result<()> {
    open_target(path.parent().unwrap_or(path))
}

#[cfg(target_os = "windows")]
fn open_target(path: &Path) -> Result<()> {
    Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(path)
        .spawn()
        .context("既定のアプリを起動できません")?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_target(path: &Path) -> Result<()> {
    Command::new("open")
        .arg(path)
        .spawn()
        .context("open を起動できません")?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_target(path: &Path) -> Result<()> {
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .context("xdg-open を起動できません")?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_url(url: &str) {
    let _ = Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(url)
        .spawn();
}

#[cfg(target_os = "macos")]
fn open_url(url: &str) {
    let _ = Command::new("open").arg(url).spawn();
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_url(url: &str) {
    let _ = Command::new("xdg-open").arg(url).spawn();
}

// ---------------------------------------------------------------------------
// 起動
// ---------------------------------------------------------------------------

fn load_interactive_js(workspace: &Path, explicit: Option<&Path>) -> Option<String> {
    let mut candidates = Vec::new();
    if let Some(path) = explicit {
        candidates.push(path.to_path_buf());
    }
    candidates.push(workspace.join("hatena_interactive.js"));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("hatena_interactive.js"));
        }
    }

    for candidate in candidates {
        if candidate.is_file() {
            match fs::read_to_string(&candidate) {
                Ok(text) if !text.trim().is_empty() => {
                    println!("[JS] 読み込み: {}", candidate.display());
                    return Some(text);
                }
                Err(err) => eprintln!(
                    "[WARN] JS を読めません: {} ({err})",
                    candidate.display()
                ),
                _ => {}
            }
        }
    }

    println!("[JS] 同梱の hatena_interactive.js を使用します");
    Some(EMBEDDED_JS.to_string())
}

fn default_settings_path(workspace: &Path) -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        PathBuf::from(appdata)
            .join("hatena_studio")
            .join("settings.json")
    } else {
        workspace.join(".hatena_studio_settings.json")
    }
}

struct Cli {
    workspace: PathBuf,
    port: u16,
    no_open: bool,
    cache: Option<PathBuf>,
    js: Option<PathBuf>,
    once: Option<PathBuf>,
    output: Option<String>,
    preview: Option<String>,
    refresh_related: bool,
    no_related: bool,
    no_toc: bool,
    no_js: bool,
    no_widget: bool,
    keep_title: bool,
    count: Option<usize>,
    min_score: Option<f64>,
    pages: Option<usize>,
    blog_url: Option<String>,
}

fn parse_cli() -> Result<Cli> {
    let mut workspace = std::env::current_dir()?;
    let mut port = 8787;
    let mut no_open = false;
    let mut cache = None;
    let mut js = None;
    let mut once = None;
    let mut output = None;
    let mut preview = None;
    let mut refresh_related = false;
    let mut no_related = false;
    let mut no_toc = false;
    let mut no_js = false;
    let mut no_widget = false;
    let mut keep_title = false;
    let mut count = None;
    let mut min_score = None;
    let mut pages = None;
    let mut blog_url = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dir" | "-d" => {
                let value = args.next().context("--dir の値がありません")?;
                let path = PathBuf::from(value);
                workspace = if path.is_absolute() {
                    path
                } else {
                    std::env::current_dir()?.join(path)
                };
            }
            "--port" | "-p" => {
                port = args
                    .next()
                    .context("--port の値がありません")?
                    .parse()
                    .context("--port は数値で指定してください")?;
            }
            "--cache" => {
                cache = Some(PathBuf::from(
                    args.next().context("--cache の値がありません")?,
                ));
            }
            "--js" => {
                js = Some(PathBuf::from(args.next().context("--js の値がありません")?));
            }
            "--no-open" => no_open = true,
            "--once" => {
                once = Some(PathBuf::from(
                    args.next().context("--once の値がありません")?,
                ));
            }
            "--output" | "-o" => {
                output = Some(args.next().context("--output の値がありません")?);
            }
            "--preview" => {
                preview = Some(args.next().context("--preview の値がありません")?);
            }
            "--refresh-related" => refresh_related = true,
            "--no-related" => no_related = true,
            "--no-toc" => no_toc = true,
            "--no-js" => no_js = true,
            "--no-widget" => no_widget = true,
            "--keep-title" => keep_title = true,
            "--count" => {
                count = Some(
                    args.next()
                        .context("--count の値がありません")?
                        .parse()
                        .context("--count は数値で指定してください")?,
                );
            }
            "--min-score" => {
                min_score = Some(
                    args.next()
                        .context("--min-score の値がありません")?
                        .parse()
                        .context("--min-score は数値で指定してください")?,
                );
            }
            "--pages" => {
                pages = Some(
                    args.next()
                        .context("--pages の値がありません")?
                        .parse()
                        .context("--pages は数値で指定してください")?,
                );
            }
            "--blog-url" => {
                blog_url = Some(args.next().context("--blog-url の値がありません")?);
            }
            "--help" | "-h" => {
                println!(
                    "Hatena Studio {}\n\n\
                     使い方:\n\
                       hatena_studio [--dir <記事フォルダ>] [--port 8787] [--no-open]\n\
                       hatena_studio --once <記事.md> [--output <HTML>] [--no-related]\n\n\
                     オプション:\n\
                       -d, --dir <PATH>       記事フォルダ（既定: カレントディレクトリ）\n\
                       -p, --port <PORT>      待ち受けポート（既定: 8787）\n\
                           --cache <PATH>     フィードキャッシュの保存先\n\
                           --js <PATH>        hatena_interactive.js の場所\n\
                           --no-open          ブラウザを自動で開かない\n\
                           --once <PATH>      1 回だけ変換して終了（GUI を起動しない）\n\
                       -o, --output <PATH>    出力 HTML のパス\n\
                           --preview <PATH>   プレビュー HTML のパス\n\
                           --refresh-related  関連記事フィードを再取得\n\
                           --no-related       関連記事を挿入しない\n\
                           --no-toc           目次を生成しない\n\
                           --no-js            インタラクション JS を埋め込まない\n\
                           --no-widget        VXLAN クイズを挿入しない\n\
                           --keep-title       先頭の H1 を本文に残す\n\
                           --count <N>        関連記事の件数\n\
                           --min-score <F>    関連記事の最低スコア\n\
                           --pages <N>        フィードの最大ページ数\n\
                           --blog-url <URL>   ブログ URL\n",
                    env!("CARGO_PKG_VERSION")
                );
                std::process::exit(0);
            }
            other => bail!("不明な引数です: {other}（--help を参照）"),
        }
    }

    Ok(Cli {
        workspace,
        port,
        no_open,
        cache,
        js,
        once,
        output,
        preview,
        refresh_related,
        no_related,
        no_toc,
        no_js,
        no_widget,
        keep_title,
        count,
        min_score,
        pages,
        blog_url,
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = parse_cli()?;
    let workspace = cli.workspace;
    if !workspace.is_dir() {
        bail!("記事フォルダが見つかりません: {}", workspace.display());
    }

    let cache_path = cli
        .cache
        .unwrap_or_else(|| feed::default_cache_path(&workspace));
    let settings_path = default_settings_path(&workspace);
    let interactive_js = load_interactive_js(&workspace, cli.js.as_deref());

    let inner = Arc::new(Inner {
        workspace: workspace.clone(),
        cache_path,
        settings_path,
        interactive_js,
        latest: Mutex::new(Latest {
            preview: String::new(),
            fragment: String::new(),
            output_path: None,
            preview_path: None,
            input_path: None,
            title: String::new(),
        }),
    });
    let state = AppState {
        inner: inner.clone(),
    };

    if let Some(input) = cli.once.clone() {
        let defaults = Settings::load(&inner.settings_path);
        let request = ConvertRequest {
            input: input.to_string_lossy().to_string(),
            title: String::new(),
            output: cli.output.clone().unwrap_or_default(),
            preview: cli.preview.clone().unwrap_or_default(),
            blog_url: cli.blog_url.clone().unwrap_or(defaults.blog_url),
            related: !cli.no_related,
            related_count: cli.count.unwrap_or(defaults.related_count),
            related_pages: cli.pages.unwrap_or(defaults.related_pages),
            related_min_score: cli.min_score.unwrap_or(defaults.related_min_score),
            cache_hours: defaults.cache_hours,
            refresh_related: cli.refresh_related,
            toc: !cli.no_toc,
            js: !cli.no_js,
            widget: !cli.no_widget,
            keep_title: cli.keep_title,
            write_preview: true,
            exclude_urls: Vec::new(),
        };
        let started = Instant::now();
        let value = convert(inner.clone(), request)?;
        if let Some(logs) = value.get("logs").and_then(Value::as_array) {
            for line in logs {
                if let Some(line) = line.as_str() {
                    println!("{line}");
                }
            }
        }
        if let Some(stats) = value.get("stats") {
            println!("[STATS] {stats}");
        }
        println!("[TIME] {} ms", started.elapsed().as_millis());
        return Ok(());
    }

    let app = Router::new()
        .route("/", get(index))
        .route("/app.css", get(stylesheet))
        .route("/app.js", get(script))
        .route("/preview", get(preview))
        .route("/api/state", get(api_state))
        .route("/api/browse", get(api_browse))
        .route("/api/convert", post(api_convert))
        .route("/api/refresh-cache", post(api_refresh_cache))
        .route("/api/settings", post(api_settings))
        .route("/api/latest/html", get(api_latest_html))
        .route("/api/open", post(api_open))
        .with_state(state);

    let mut listener = None;
    let mut actual_port = cli.port;
    for offset in 0..20 {
        let port = cli.port.saturating_add(offset);
        match tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await {
            Ok(bound) => {
                actual_port = port;
                listener = Some(bound);
                break;
            }
            Err(err) => {
                if offset == 0 {
                    eprintln!("[WARN] ポート {port} を利用できません: {err}");
                }
            }
        }
    }
    let listener = listener.context("空いているポートが見つかりませんでした")?;
    let url = format!("http://127.0.0.1:{actual_port}/");

    println!("Hatena Studio {}", env!("CARGO_PKG_VERSION"));
    println!("  記事フォルダ : {}", workspace.display());
    println!("  キャッシュ   : {}", inner.cache_path.display());
    println!("  設定ファイル : {}", inner.settings_path.display());
    println!("  URL          : {url}");
    println!("  終了するには Ctrl+C を押してください。");

    if !cli.no_open {
        open_url(&url);
    }

    axum::serve(listener, app).await?;
    Ok(())
}
