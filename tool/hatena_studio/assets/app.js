"use strict";

(function () {
  const $ = (id) => document.getElementById(id);

  const els = {
    inputPath: $("inputPath"),
    titleInput: $("titleInput"),
    browseBtn: $("browseBtn"),
    recentWrap: $("recentWrap"),
    recentList: $("recentList"),
    relatedEnabled: $("relatedEnabled"),
    relatedSettings: $("relatedSettings"),
    relatedCount: $("relatedCount"),
    relatedMinScore: $("relatedMinScore"),
    relatedPages: $("relatedPages"),
    cacheHours: $("cacheHours"),
    blogUrl: $("blogUrl"),
    relatedList: $("relatedList"),
    optToc: $("optToc"),
    optJs: $("optJs"),
    optWidget: $("optWidget"),
    optKeepTitle: $("optKeepTitle"),
    convertBtn: $("convertBtn"),
    convertSpinner: $("convertSpinner"),
    convertLabel: $("convertLabel"),
    outputPaths: $("outputPaths"),
    logsPanel: $("logsPanel"),
    logBody: $("logBody"),
    logCount: $("logCount"),
    previewFrame: $("previewFrame"),
    htmlView: $("htmlView"),
    emptyState: $("emptyState"),
    stats: $("stats"),
    tabPreview: $("tabPreview"),
    tabHtml: $("tabHtml"),
    openPreviewBtn: $("openPreviewBtn"),
    copyHtmlBtn: $("copyHtmlBtn"),
    openFolderBtn: $("openFolderBtn"),
    refreshCacheBtn: $("refreshCacheBtn"),
    cachePill: $("cachePill"),
    cacheText: $("cacheText"),
    fileModal: $("fileModal"),
    browseCrumbs: $("browseCrumbs"),
    rootBtn: $("rootBtn"),
    upBtn: $("upBtn"),
    browsePathInput: $("browsePathInput"),
    jumpBtn: $("jumpBtn"),
    browseSearch: $("browseSearch"),
    searchClearBtn: $("searchClearBtn"),
    browseList: $("browseList"),
    browseStatus: $("browseStatus"),
    toasts: $("toasts"),
  };

  const state = {
    excluded: new Set(),
    htmlText: "",
    previewUrl: "",
    lastOutputPath: "",
    lastPreviewPath: "",
    lastInputPath: "",
    busy: false,
    refreshing: false,
    browsePath: "",
    browseParent: null,
    browseRoot: "",
    browseCrumbs: [],
    browseCache: new Map(),
    browseScroll: new Map(),
    browseRows: [],
    browseRowEls: [],
    browseActive: -1,
    browseMode: "browse",
    lastBrowseData: null,
    searchTimer: null,
    saveTimer: null,
    activeTab: "preview",
  };

  /* ---------------------------------------------------------------- */
  /* ユーティリティ                                                     */
  /* ---------------------------------------------------------------- */

  function toast(message, type) {
    const node = document.createElement("div");
    node.className = "toast" + (type ? " " + type : "");
    node.textContent = message;
    els.toasts.appendChild(node);
    window.setTimeout(() => {
      node.style.opacity = "0";
      node.style.transition = "opacity .25s ease";
      window.setTimeout(() => node.remove(), 260);
    }, 4200);
  }

  async function api(path, options) {
    const response = await fetch(path, options);
    let data = null;
    try {
      data = await response.json();
    } catch (err) {
      /* JSON でないレスポンス */
    }
    if (!response.ok || !data || data.ok === false) {
      const message =
        (data && data.error) || "HTTP " + response.status + " " + response.statusText;
      throw new Error(message);
    }
    return data;
  }

  function numberValue(element, fallback) {
    const value = Number(element.value);
    return Number.isFinite(value) ? value : fallback;
  }

  function formatBytes(bytes) {
    if (!Number.isFinite(bytes)) return "";
    if (bytes >= 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + " MB";
    return (bytes / 1024).toFixed(1) + " KB";
  }

  function formatDate(epoch) {
    if (!epoch) return "";
    const date = new Date(epoch * 1000);
    if (Number.isNaN(date.getTime())) return "";
    const pad = (value) => String(value).padStart(2, "0");
    return (
      date.getFullYear() +
      "." +
      pad(date.getMonth() + 1) +
      "." +
      pad(date.getDate())
    );
  }

  function safeHref(url) {
    return /^https?:\/\//i.test(url || "") ? url : "#";
  }

  function setBusy(busy) {
    state.busy = busy;
    els.convertSpinner.hidden = !busy;
    els.convertLabel.textContent = busy ? "変換中…" : "変換してプレビュー";
    updateButtons();
  }

  function updateButtons() {
    const disabled = state.busy || state.refreshing;
    els.convertBtn.disabled = disabled;
    els.refreshCacheBtn.disabled = disabled;
    els.openPreviewBtn.disabled = !state.previewUrl;
    els.copyHtmlBtn.disabled = !state.previewUrl;
    els.openFolderBtn.disabled = !state.lastOutputPath && !els.inputPath.value.trim();
  }

  function setLogs(lines) {
    const list = Array.isArray(lines) ? lines : [];
    els.logBody.textContent = list.join("\n");
    els.logCount.textContent = list.length ? list.length + " 行" : "";
  }

  function appendLogs(lines) {
    const current = els.logBody.textContent ? els.logBody.textContent.split("\n") : [];
    setLogs(current.concat(lines));
  }

  /* ---------------------------------------------------------------- */
  /* 設定                                                               */
  /* ---------------------------------------------------------------- */

  function applySettings(settings) {
    if (!settings) settings = {};
    els.inputPath.value = settings.input || "";
    els.titleInput.value = settings.title || "";
    els.blogUrl.value =
      settings.blog_url || "https://yoshishinnze.hatenablog.com/";
    els.relatedEnabled.checked = settings.related !== false;
    els.relatedCount.value = settings.related_count || 5;
    els.relatedMinScore.value =
      settings.related_min_score !== undefined ? settings.related_min_score : 0.02;
    els.relatedPages.value = settings.related_pages || 15;
    els.cacheHours.value =
      settings.cache_hours !== undefined ? settings.cache_hours : 12;
    els.optToc.checked = settings.toc !== false;
    els.optJs.checked = settings.js !== false;
    els.optWidget.checked = settings.widget !== false;
    els.optKeepTitle.checked = settings.keep_title === true;
    syncRelatedUI();
    updateButtons();
  }

  function syncRelatedUI() {
    const enabled = els.relatedEnabled.checked;
    els.relatedSettings.style.opacity = enabled ? "1" : "0.55";
    els.relatedSettings.style.pointerEvents = enabled ? "" : "none";
  }

  function gatherSettings() {
    return {
      input: els.inputPath.value.trim(),
      title: els.titleInput.value.trim(),
      blog_url: els.blogUrl.value.trim(),
      related: els.relatedEnabled.checked,
      related_count: numberValue(els.relatedCount, 5),
      related_pages: numberValue(els.relatedPages, 15),
      related_min_score: numberValue(els.relatedMinScore, 0.02),
      cache_hours: numberValue(els.cacheHours, 12),
      toc: els.optToc.checked,
      js: els.optJs.checked,
      widget: els.optWidget.checked,
      keep_title: els.optKeepTitle.checked,
    };
  }

  function saveSettings() {
    window.clearTimeout(state.saveTimer);
    state.saveTimer = window.setTimeout(async () => {
      try {
        await fetch("/api/settings", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(gatherSettings()),
        });
      } catch (err) {
        /* 設定保存の失敗は致命的ではない */
      }
    }, 400);
  }

  function gatherPayload() {
    return Object.assign({}, gatherSettings(), {
      output: "",
      preview: "",
      refresh_related: false,
      write_preview: true,
      exclude_urls: Array.from(state.excluded),
    });
  }

  /* ---------------------------------------------------------------- */
  /* 初期表示                                                           */
  /* ---------------------------------------------------------------- */

  async function init() {
    try {
      const data = await api("/api/state");
      applySettings(data.settings);
      renderCache(data.cache);
      renderRecent(data.recent || []);
      if (!data.js_loaded) {
        toast("hatena_interactive.js が見つかりません。同梱版を使用します。", "warn");
      }
    } catch (err) {
      toast("初期化に失敗しました: " + err.message, "error");
    }
  }

  function renderRecent(files) {
    if (!files.length) {
      els.recentWrap.hidden = true;
      return;
    }
    els.recentWrap.hidden = false;
    els.recentList.replaceChildren();
    files.slice(0, 12).forEach((file) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "recent-item";
      button.title = file.path;

      const name = document.createElement("span");
      name.className = "recent-name";
      name.textContent = file.name;

      const meta = document.createElement("span");
      meta.className = "recent-meta";
      meta.textContent = formatDate(file.modified) || formatBytes(file.size);

      button.append(name, meta);
      button.addEventListener("click", () => {
        els.inputPath.value = file.path;
        state.excluded.clear();
        saveSettings();
        updateButtons();
        toast("記事を選択しました。変換ボタンでプレビューできます。");
      });
      els.recentList.appendChild(button);
    });
  }

  function renderCache(cache) {
    if (!cache || !cache.exists) {
      els.cachePill.className = "cache-pill";
      els.cacheText.textContent = "キャッシュなし";
      return;
    }
    const age = Number(cache.age_hours || 0);
    const ageText =
      age < 0.05 ? "たった今" : age.toFixed(age < 10 ? 1 : 0) + " 時間前";
    els.cachePill.className =
      "cache-pill " + (cache.fresh ? "is-fresh" : "is-stale");
    els.cacheText.textContent =
      cache.entries + " 件 ・ " + ageText + (cache.fresh ? "" : "（期限切れ）");
    els.cachePill.title =
      "関連記事フィードのキャッシュ: " +
      cache.entries +
      " 件 / " +
      ageText +
      (cache.blog_url ? " / " + cache.blog_url : "");
  }

  /* ---------------------------------------------------------------- */
  /* 変換                                                               */
  /* ---------------------------------------------------------------- */

  async function convert() {
    if (state.busy || state.refreshing) return;
    const input = els.inputPath.value.trim();
    if (!input) {
      toast("Markdown ファイルを選択してください。", "warn");
      openBrowse();
      return;
    }

    setBusy(true);
    try {
      const data = await api("/api/convert", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(gatherPayload()),
      });
      applyResult(data);
      saveSettings();
      toast("生成しました（" + data.elapsed_ms + " ms）");
    } catch (err) {
      toast("変換できませんでした: " + err.message, "error");
      appendLogs(["[ERROR] " + err.message]);
      els.logsPanel.open = true;
    } finally {
      setBusy(false);
    }
  }

  function applyResult(data) {
    state.previewUrl = data.preview_url || "/preview";
    state.lastOutputPath = data.output_path || "";
    state.lastPreviewPath = data.preview_path || "";
    state.lastInputPath = data.input_path || "";
    state.htmlText = "";

    if (!els.titleInput.value.trim()) {
      els.titleInput.value = data.title || "";
    }

    els.previewFrame.src =
      state.previewUrl +
      (state.previewUrl.includes("?") ? "&" : "?") +
      "t=" +
      Date.now();

    els.emptyState.hidden = true;
    setTab("preview");
    renderStats(data.stats, data.related);
    renderOutputPaths(data);
    renderRelated(data.related || []);
    renderCache(data.cache);
    setLogs(data.logs || []);
    updateButtons();
  }

  function renderStats(stats, related) {
    els.stats.replaceChildren();
    const chips = [
      ["関連", (related || []).length],
      ["見出し", stats.headings],
      ["目次", stats.toc_links],
      ["表", stats.tables],
      ["解答", stats.answers],
      ["コード", stats.code_blocks],
      ["サイズ", formatBytes(stats.bytes)],
    ];
    chips.forEach(([label, value]) => {
      const chip = document.createElement("span");
      chip.className = "stat-chip";
      const strong = document.createElement("strong");
      strong.textContent = value;
      chip.append(strong, document.createTextNode(" " + label));
      els.stats.appendChild(chip);
    });
  }

  function renderOutputPaths(data) {
    els.outputPaths.hidden = false;
    els.outputPaths.replaceChildren();
    const rows = [
      ["はてな用 HTML", data.output_path],
      ["プレビュー", data.preview_path],
    ];
    rows.forEach(([label, path]) => {
      if (!path) return;
      const line = document.createElement("div");
      line.className = "path-line";
      const title = document.createElement("b");
      title.textContent = label;
      const value = document.createElement("span");
      value.textContent = path;
      line.append(title, value);
      els.outputPaths.appendChild(line);
    });
  }

  function renderRelated(cards) {
    els.relatedList.replaceChildren();
    if (!cards.length) {
      const empty = document.createElement("p");
      empty.className = "empty-mini";
      empty.textContent = "関連記事は見つかりませんでした。";
      els.relatedList.appendChild(empty);
      return;
    }

    cards.forEach((card, index) => {
      const item = document.createElement("div");
      item.className = "related-item";

      const top = document.createElement("div");
      top.className = "related-top";

      const rank = document.createElement("span");
      rank.className = "rank";
      rank.textContent = String(index + 1);

      const score = document.createElement("span");
      score.className = "score";
      score.textContent = Number(card.score || 0).toFixed(3);

      const remove = document.createElement("button");
      remove.type = "button";
      remove.className = "related-remove";
      remove.title = "この記事を除外して選び直す";
      remove.setAttribute("aria-label", "この関連記事を除外");
      remove.textContent = "×";
      remove.addEventListener("click", () => {
        if (state.busy) return;
        state.excluded.add(card.url);
        toast("除外して関連記事を選び直します。");
        convert();
      });

      top.append(rank, score, remove);

      const link = document.createElement("a");
      link.className = "related-title";
      link.href = safeHref(card.url);
      link.target = "_blank";
      link.rel = "noopener";
      link.textContent = card.title || "(無題)";

      const meta = document.createElement("div");
      meta.className = "related-meta";
      const date = (card.published || "").slice(0, 10).replace(/-/g, ".");
      const categories = (card.categories || []).slice(0, 2).join("・");
      meta.textContent = [date, categories].filter(Boolean).join(" ・ ");

      const bar = document.createElement("div");
      bar.className = "score-bar";
      const fill = document.createElement("span");
      fill.style.width = Math.min(100, Math.max(2, (card.score || 0) * 100)) + "%";
      bar.appendChild(fill);

      item.append(top, link, meta, bar);
      els.relatedList.appendChild(item);
    });
  }

  /* ---------------------------------------------------------------- */
  /* キャッシュ更新                                                     */
  /* ---------------------------------------------------------------- */

  async function refreshCache() {
    if (state.busy || state.refreshing) return;
    state.refreshing = true;
    updateButtons();
    const label = els.refreshCacheBtn.querySelector(".btn-label");
    const original = label ? label.textContent : "";
    if (label) label.textContent = "取得中…";

    try {
      const data = await api("/api/refresh-cache", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          blog_url: els.blogUrl.value.trim(),
          related_pages: numberValue(els.relatedPages, 15),
        }),
      });
      renderCache(data.cache);
      appendLogs(data.logs || []);
      toast(data.count + " 件の記事を取得しました。");
      saveSettings();
    } catch (err) {
      toast("キャッシュ更新に失敗しました: " + err.message, "error");
      appendLogs(["[ERROR] " + err.message]);
      els.logsPanel.open = true;
    } finally {
      state.refreshing = false;
      if (label) label.textContent = original;
      updateButtons();
    }
  }

  /* ---------------------------------------------------------------- */
  /* タブ・コピー・外部で開く                                           */
  /* ---------------------------------------------------------------- */

  function setTab(name) {
    if (name === "html" && !state.previewUrl) {
      toast("先に記事を変換してください。", "warn");
      return;
    }
    state.activeTab = name;
    const html = name === "html";
    els.tabPreview.classList.toggle("active", !html);
    els.tabHtml.classList.toggle("active", html);
    els.tabPreview.setAttribute("aria-selected", String(!html));
    els.tabHtml.setAttribute("aria-selected", String(html));

    if (!state.previewUrl) {
      els.previewFrame.hidden = true;
      els.htmlView.hidden = true;
      els.emptyState.hidden = false;
      return;
    }

    els.emptyState.hidden = true;
    els.previewFrame.hidden = html;
    els.htmlView.hidden = !html;
    if (html) loadHtmlSource();
  }

  async function loadHtmlSource() {
    if (state.htmlText) {
      els.htmlView.textContent = state.htmlText;
      return;
    }
    els.htmlView.textContent = "読み込み中…";
    try {
      const response = await fetch("/api/latest/html", { cache: "no-store" });
      if (!response.ok) throw new Error("HTTP " + response.status);
      state.htmlText = await response.text();
      els.htmlView.textContent = state.htmlText;
    } catch (err) {
      els.htmlView.textContent = "HTML を読み込めませんでした: " + err.message;
    }
  }

  async function ensureHtml() {
    if (state.htmlText) return state.htmlText;
    const response = await fetch("/api/latest/html", { cache: "no-store" });
    if (!response.ok) throw new Error("HTTP " + response.status);
    state.htmlText = await response.text();
    return state.htmlText;
  }

  async function copyHtml() {
    if (!state.previewUrl) {
      toast("先に記事を変換してください。", "warn");
      return;
    }
    try {
      const html = await ensureHtml();
      await navigator.clipboard.writeText(html);
      toast("HTML をコピーしました。はてなブログの HTML 編集モードへ貼り付けてください。");
    } catch (err) {
      try {
        const html = await ensureHtml();
        const area = document.createElement("textarea");
        area.value = html;
        area.style.position = "fixed";
        area.style.opacity = "0";
        document.body.appendChild(area);
        area.select();
        document.execCommand("copy");
        area.remove();
        toast("HTML をコピーしました。");
      } catch (inner) {
        toast("コピーできませんでした: " + inner.message, "error");
      }
    }
  }

  function openPreview() {
    if (!state.previewUrl) {
      toast("先に記事を変換してください。", "warn");
      return;
    }
    window.open(state.previewUrl, "_blank", "noopener");
  }

  async function openFolder() {
    let path = state.lastOutputPath;
    let kind = "select";
    if (!path) {
      path = els.inputPath.value.trim();
      kind = "folder";
    }
    if (!path) {
      toast("保存先がまだありません。", "warn");
      return;
    }
    try {
      await api("/api/open", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ kind: kind, path: path }),
      });
    } catch (err) {
      toast("開けませんでした: " + err.message, "error");
    }
  }

  /* ---------------------------------------------------------------- */
  /* ファイルブラウザ                                                   */
  /* ---------------------------------------------------------------- */

  // Windows の "\\" と POSIX の "/" を比較用に統一する。
  function normPath(path) {
    return String(path || "")
      .replace(/\//g, "\\")
      .replace(/\\+$/, "")
      .toLowerCase();
  }

  // 絶対パスをルートから現在フォルダまでのパンくずに分解する。
  function buildCrumbs(path, root) {
    if (!path) return [];
    const sep = path.includes("\\") ? "\\" : "/";
    const parts = path.split(sep).filter(Boolean);
    const crumbs = [];
    let acc = "";
    parts.forEach((part, index) => {
      acc =
        index === 0
          ? path.startsWith(sep)
            ? sep + part
            : part
          : acc + sep + part;
      crumbs.push({
        label: part,
        path: acc,
        isRoot: !!root && normPath(acc) === normPath(root),
      });
    });
    return crumbs;
  }

  function renderCrumbs() {
    const box = els.browseCrumbs;
    box.replaceChildren();
    state.browseCrumbs.forEach((crumb, index) => {
      if (index > 0) {
        const sep = document.createElement("span");
        sep.className = "crumb-sep";
        sep.textContent = "›";
        box.appendChild(sep);
      }
      const node = document.createElement("button");
      const isLast = index === state.browseCrumbs.length - 1;
      node.type = "button";
      node.className =
        "crumb" + (isLast ? " is-current" : "") + (crumb.isRoot ? " is-root" : "");
      node.textContent = crumb.isRoot ? "⌂ " + crumb.label : crumb.label;
      node.title = crumb.path;
      node.addEventListener("click", () => loadDir(crumb.path));
      box.appendChild(node);
    });
    box.scrollLeft = box.scrollWidth;
  }

  function browseIcon(kind) {
    const span = document.createElement("span");
    span.className = "browse-icon";
    span.textContent = kind === "dir" ? "DIR" : "MD";
    return span;
  }

  function setActiveRow(index) {
    const rows = state.browseRowEls;
    if (!rows.length) return;
    if (state.browseActive >= 0 && rows[state.browseActive]) {
      rows[state.browseActive].classList.remove("is-active");
    }
    const next = Math.max(0, Math.min(index, rows.length - 1));
    state.browseActive = next;
    rows[next].classList.add("is-active");
    rows[next].scrollIntoView({ block: "nearest" });
  }

  function renderRows(descriptors) {
    els.browseList.replaceChildren();
    state.browseRows = descriptors;
    state.browseRowEls = [];
    state.browseActive = -1;

    if (!descriptors.length) {
      const empty = document.createElement("div");
      empty.className = "browse-empty";
      empty.textContent =
        state.browseMode === "search"
          ? "一致する Markdown がありません。"
          : "このフォルダには Markdown がありません。";
      els.browseList.appendChild(empty);
      return;
    }

    descriptors.forEach((item, index) => {
      const row = document.createElement("button");
      row.type = "button";
      row.className = "browse-row is-" + item.kind;

      const nameBox = document.createElement("span");
      nameBox.className = "browse-name-box";
      const name = document.createElement("span");
      name.className = "browse-name";
      name.textContent = item.name;
      nameBox.appendChild(name);
      if (item.relative) {
        const rel = document.createElement("small");
        rel.className = "browse-rel";
        rel.textContent = item.relative;
        nameBox.appendChild(rel);
      }

      const meta = document.createElement("span");
      meta.className = "browse-meta";
      meta.textContent = item.meta;
      row.append(browseIcon(item.kind), nameBox, meta);

      row.addEventListener("click", () => {
        state.browseActive = index;
        item.activate();
      });
      row.addEventListener("mouseenter", () => {
        if (state.browseActive !== index) setActiveRow(index);
      });

      state.browseRowEls.push(row);
      els.browseList.appendChild(row);
    });
  }

  function selectFile(file) {
    els.inputPath.value = file.path;
    state.excluded.clear();
    els.fileModal.hidden = true;
    saveSettings();
    updateButtons();
    toast("選択: " + file.name);
  }

  function fileMeta(file) {
    return [formatBytes(file.size), formatDate(file.modified)]
      .filter(Boolean)
      .join(" ・ ");
  }

  function renderBrowse(data) {
    const dirs = (data.dirs || []).map((dir) => ({
      kind: "dir",
      name: dir.name,
      meta: "フォルダ",
      activate: () => loadDir(dir.path),
    }));
    const files = (data.files || []).map((file) => ({
      kind: "file",
      name: file.name,
      meta: fileMeta(file),
      activate: () => selectFile(file),
    }));
    renderRows(dirs.concat(files));
    els.browseStatus.textContent =
      dirs.length + " フォルダ / " + files.length + " Markdown";
  }

  function applyBrowseData(data) {
    state.browsePath = data.path;
    state.browseParent = data.parent;
    if (data.root) state.browseRoot = data.root;
    state.lastBrowseData = data;
    state.browseMode = "browse";
    state.browseCrumbs = buildCrumbs(data.path, state.browseRoot);
    renderCrumbs();
    els.upBtn.disabled = !data.parent;
    els.rootBtn.disabled =
      !state.browseRoot || normPath(data.path) === normPath(state.browseRoot);
    els.browsePathInput.value = data.path;
    renderBrowse(data);
    els.browseList.scrollTop = state.browseScroll.get(normPath(data.path)) || 0;
  }

  function cancelSearch() {
    if (state.searchTimer) {
      clearTimeout(state.searchTimer);
      state.searchTimer = null;
    }
    if (els.browseSearch.value) els.browseSearch.value = "";
  }

  function showBrowseError(err) {
    els.browseStatus.textContent = "";
    state.browseMode = "browse";
    state.browseRows = [];
    state.browseRowEls = [];
    state.browseActive = -1;
    els.browseList.replaceChildren();
    const message = document.createElement("div");
    message.className = "browse-empty is-error";
    message.textContent = "フォルダを開けませんでした: " + err.message;
    els.browseList.appendChild(message);
  }

  async function loadDir(path) {
    // 別のフォルダへ移動する前に、現在のスクロール位置を覚えておく。
    if (state.browseMode === "browse" && state.browsePath) {
      state.browseScroll.set(normPath(state.browsePath), els.browseList.scrollTop);
    }
    cancelSearch();

    const requested = path || state.browseRoot || "";
    const key = normPath(requested);
    let data = state.browseCache.get(key);
    if (!data) {
      els.browseStatus.textContent = "読み込み中…";
      try {
        const query = requested ? "?path=" + encodeURIComponent(requested) : "";
        data = await api("/api/browse" + query);
        state.browseCache.set(normPath(data.path), data);
        state.browseCache.set(key, data);
      } catch (err) {
        showBrowseError(err);
        return;
      }
    }
    applyBrowseData(data);
  }

  async function runSearch() {
    const query = els.browseSearch.value.trim();
    if (!query) {
      cancelSearch();
      if (state.lastBrowseData) applyBrowseData(state.lastBrowseData);
      return;
    }
    const base = state.browsePath || state.browseRoot || "";
    try {
      const params = new URLSearchParams({ q: query });
      if (base) params.set("path", base);
      const data = await api("/api/search?" + params.toString());
      // 入力が変わっていたら古い応答は捨てる。
      if (els.browseSearch.value.trim() !== query) return;
      state.browseMode = "search";
      const files = (data.files || []).map((file) => ({
        kind: "file",
        name: file.name,
        relative: file.relative,
        meta: fileMeta(file),
        activate: () => selectFile(file),
      }));
      renderRows(files);
      els.browseStatus.textContent =
        files.length +
        " 件" +
        (data.truncated ? "（表示上限あり）" : "") +
        " — " +
        query;
    } catch (err) {
      els.browseStatus.textContent = "検索に失敗しました: " + err.message;
    }
  }

  function scheduleSearch() {
    if (state.searchTimer) clearTimeout(state.searchTimer);
    state.searchTimer = setTimeout(() => {
      state.searchTimer = null;
      runSearch();
    }, 180);
  }

  function jumpToPath() {
    const value = els.browsePathInput.value.trim();
    if (value) loadDir(value);
  }

  function goParent() {
    if (state.browseParent) loadDir(state.browseParent);
  }

  function goRoot() {
    if (state.browseRoot) loadDir(state.browseRoot);
  }

  function moveActive(delta) {
    const total = state.browseRows.length;
    if (!total) return;
    const from = state.browseActive < 0 ? (delta > 0 ? -1 : 0) : state.browseActive;
    let next = from + delta;
    if (next < 0) next = total - 1;
    if (next >= total) next = 0;
    setActiveRow(next);
  }

  function activateActive() {
    const item = state.browseRows[state.browseActive];
    if (item) item.activate();
  }

  function handleBrowseKey(event) {
    if (event.ctrlKey || event.metaKey) return false;
    const inPath = event.target === els.browsePathInput;
    const inSearch = event.target === els.browseSearch;

    if (event.altKey && event.key === "ArrowUp") {
      event.preventDefault();
      goParent();
      return true;
    }
    if (event.altKey && event.key === "Home") {
      event.preventDefault();
      goRoot();
      return true;
    }
    if (event.key === "/" && !inPath && !inSearch) {
      event.preventDefault();
      els.browseSearch.focus();
      return true;
    }
    if (inPath) {
      if (event.key === "Enter") {
        event.preventDefault();
        jumpToPath();
        return true;
      }
      return false;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      moveActive(event.key === "ArrowDown" ? 1 : -1);
      return true;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (state.browseActive >= 0) {
        activateActive();
      } else if (inSearch) {
        if (state.searchTimer) {
          clearTimeout(state.searchTimer);
          state.searchTimer = null;
        }
        runSearch();
      }
      return true;
    }
    return false;
  }

  async function openBrowse() {
    state.browseCache.clear();
    state.browseScroll.clear();
    els.fileModal.hidden = false;
    els.browseSearch.value = "";
    await loadDir(els.inputPath.value.trim() || state.browseRoot);
    els.browseSearch.focus();
  }

  function closeBrowse() {
    els.fileModal.hidden = true;
    if (state.searchTimer) {
      clearTimeout(state.searchTimer);
      state.searchTimer = null;
    }
  }

  /* ---------------------------------------------------------------- */
  /* イベント                                                           */
  /* ---------------------------------------------------------------- */

  els.convertBtn.addEventListener("click", convert);
  els.refreshCacheBtn.addEventListener("click", refreshCache);
  els.browseBtn.addEventListener("click", openBrowse);
  els.relatedEnabled.addEventListener("change", () => {
    syncRelatedUI();
    saveSettings();
  });
  els.openPreviewBtn.addEventListener("click", openPreview);
  els.copyHtmlBtn.addEventListener("click", copyHtml);
  els.openFolderBtn.addEventListener("click", openFolder);
  els.tabPreview.addEventListener("click", () => setTab("preview"));
  els.tabHtml.addEventListener("click", () => setTab("html"));
  els.upBtn.addEventListener("click", goParent);
  els.rootBtn.addEventListener("click", goRoot);
  els.jumpBtn.addEventListener("click", jumpToPath);
  els.browseSearch.addEventListener("input", scheduleSearch);
  els.searchClearBtn.addEventListener("click", () => {
    cancelSearch();
    if (state.lastBrowseData) applyBrowseData(state.lastBrowseData);
  });

  els.fileModal.querySelectorAll("[data-close]").forEach((node) => {
    node.addEventListener("click", closeBrowse);
  });

  els.inputPath.addEventListener("change", () => {
    state.excluded.clear();
    saveSettings();
    updateButtons();
  });
  [
    els.titleInput,
    els.blogUrl,
    els.relatedCount,
    els.relatedMinScore,
    els.relatedPages,
    els.cacheHours,
    els.optToc,
    els.optJs,
    els.optWidget,
    els.optKeepTitle,
  ].forEach((element) => {
    element.addEventListener("change", saveSettings);
  });

  document.addEventListener("keydown", (event) => {
    if (!els.fileModal.hidden) {
      if (event.key === "Escape") {
        event.preventDefault();
        closeBrowse();
        return;
      }
      if (handleBrowseKey(event)) return;
      // モーダル表示中は Ctrl+Enter で変換しない。
      if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
        event.preventDefault();
      }
      return;
    }
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
      event.preventDefault();
      convert();
    }
  });

  init();
})();
