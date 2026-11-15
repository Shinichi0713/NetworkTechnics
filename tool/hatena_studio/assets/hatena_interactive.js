/*!
 * hatena_interactive.js
 * はてなブログの記事本文に貼り付ける、依存ライブラリなしの軽量インタラクション集。
 *
 * - 読了プログレスバー
 * - 解答のクイズモード（ぼかし → クリックで開示、進捗カウンタ）
 * - コードブロックのコピー
 * - 目次のスムーズスクロール＋現在地ハイライト
 * - 見出しクリックでリンクコピー
 * - VXLAN カプセル化のミニクイズ
 * - キーボードショートカット（Q / R / T / ?）
 *
 * 記事内のスクリプトが使えない場合は、はてなブログの
 * 「デザイン > カスタマイズ > 記事下」などにこのファイルの中身を貼り付けてください。
 */
(function () {
  "use strict";

  var root = document.querySelector("[data-hatena-article]");
  if (!root || root.getAttribute("data-enhanced") === "1") return;
  root.setAttribute("data-enhanced", "1");

  var reduceMotion = false;
  try {
    reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  } catch (err) {}

  var STORAGE_KEY = "hatena-quiz:" + location.pathname;

  function clamp(value, min, max) {
    return Math.min(max, Math.max(min, value));
  }

  function store(key, value) {
    try {
      window.localStorage.setItem(key, value);
    } catch (err) {}
  }

  function restore(key) {
    try {
      return window.localStorage.getItem(key);
    } catch (err) {
      return null;
    }
  }

  /* ------------------------------------------------------------------ */
  /* トースト                                                           */
  /* ------------------------------------------------------------------ */

  var toastEl = null;
  var toastTimer = null;

  function toast(message) {
    if (!toastEl) {
      toastEl = document.createElement("div");
      toastEl.setAttribute("role", "status");
      toastEl.setAttribute("aria-live", "polite");
      toastEl.style.cssText =
        "position:fixed;left:50%;bottom:92px;z-index:10000;" +
        "transform:translateX(-50%) translateY(10px);" +
        "max-width:min(88vw,440px);padding:10px 18px;border-radius:999px;" +
        "background:rgba(15,23,42,.94);color:#fff;font-size:13px;" +
        "line-height:1.65;text-align:center;font-weight:600;" +
        "box-shadow:0 10px 30px rgba(15,23,42,.32);opacity:0;" +
        "pointer-events:none;transition:opacity .2s ease,transform .2s ease;" +
        "font-family:inherit;";
      document.body.appendChild(toastEl);
    }

    toastEl.textContent = message;
    requestAnimationFrame(function () {
      toastEl.style.opacity = "1";
      toastEl.style.transform = "translateX(-50%) translateY(0)";
    });

    window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(function () {
      toastEl.style.opacity = "0";
      toastEl.style.transform = "translateX(-50%) translateY(10px)";
    }, 2200);
  }

  /* ------------------------------------------------------------------ */
  /* コピー                                                             */
  /* ------------------------------------------------------------------ */

  function copyText(text) {
    if (navigator.clipboard && window.isSecureContext) {
      return navigator.clipboard.writeText(text);
    }
    return new Promise(function (resolve, reject) {
      var area = document.createElement("textarea");
      area.value = text;
      area.setAttribute("readonly", "");
      area.style.cssText =
        "position:fixed;top:-1000px;left:-1000px;opacity:0;";
      document.body.appendChild(area);
      area.select();
      try {
        var ok = document.execCommand("copy");
        area.remove();
        ok ? resolve() : reject(new Error("copy failed"));
      } catch (err) {
        area.remove();
        reject(err);
      }
    });
  }

  /* ------------------------------------------------------------------ */
  /* 紙吹雪（小さい祝福）                                               */
  /* ------------------------------------------------------------------ */

  function burst(x, y, count) {
    if (reduceMotion) return;

    var colors = [
      "#2563eb",
      "#22d3ee",
      "#10b981",
      "#f59e0b",
      "#ef4444",
      "#8b5cf6",
    ];
    var layer = document.createElement("div");
    layer.setAttribute("aria-hidden", "true");
    layer.style.cssText =
      "position:fixed;inset:0;z-index:10001;overflow:hidden;" +
      "pointer-events:none;";
    document.body.appendChild(layer);

    for (var i = 0; i < (count || 32); i++) {
      var piece = document.createElement("span");
      var size = 6 + Math.random() * 7;
      piece.style.cssText =
        "position:absolute;left:" +
        x +
        "px;top:" +
        y +
        "px;width:" +
        size +
        "px;height:" +
        size * 0.55 +
        "px;border-radius:2px;background:" +
        colors[i % colors.length] +
        ";opacity:1;will-change:transform,opacity;";
      layer.appendChild(piece);

      var angle = Math.random() * Math.PI * 2;
      var distance = 80 + Math.random() * 230;
      var dx = Math.cos(angle) * distance;
      var dy = Math.sin(angle) * distance + 130;
      var spin = Math.random() * 720 - 360;

      var animation = piece.animate(
        [
          { transform: "translate(0,0) rotate(0deg)", opacity: 1 },
          {
            transform:
              "translate(" + dx + "px," + dy + "px) rotate(" + spin + "deg)",
            opacity: 0,
          },
        ],
        {
          duration: 1100 + Math.random() * 700,
          easing: "cubic-bezier(.2,.7,.3,1)",
          fill: "forwards",
        }
      );
      animation.onfinish = function () {
        this.cancel();
      };
    }

    window.setTimeout(function () {
      layer.remove();
    }, 2400);
  }

  /* ------------------------------------------------------------------ */
  /* 読了プログレスバー                                                 */
  /* ------------------------------------------------------------------ */

  var progressBar = document.createElement("div");
  progressBar.setAttribute("aria-hidden", "true");
  progressBar.style.cssText =
    "position:fixed;top:0;left:0;height:3px;width:0%;z-index:9999;" +
    "background:linear-gradient(90deg,#1e3a8a,#3b82f6,#22d3ee);" +
    "box-shadow:0 0 8px rgba(59,130,246,.6);pointer-events:none;" +
    "transition:width .08s linear;";
  document.body.appendChild(progressBar);

  function updateProgress() {
    var rect = root.getBoundingClientRect();
    var total = rect.height - window.innerHeight;
    var scrolled = -rect.top;
    var ratio = total > 0 ? scrolled / total : 1;
    progressBar.style.width = clamp(ratio, 0, 1) * 100 + "%";
  }

  /* ------------------------------------------------------------------ */
  /* フローティング操作パネル                                           */
  /* ------------------------------------------------------------------ */

  var panel = document.createElement("div");
  panel.setAttribute("data-role", "controls");
  panel.style.cssText =
    "position:fixed;right:14px;bottom:16px;z-index:9998;display:flex;" +
    "flex-wrap:wrap;align-items:center;justify-content:flex-end;gap:6px;" +
    "max-width:calc(100vw - 28px);padding:7px;border-radius:18px;" +
    "background:rgba(255,255,255,.94);border:1px solid #dbeafe;" +
    "box-shadow:0 12px 32px rgba(15,23,42,.18);font-family:inherit;";
  document.body.appendChild(panel);

  var BUTTON_STYLE =
    "appearance:none;border:0;border-radius:999px;padding:7px 12px;" +
    "font-size:12.5px;font-weight:700;line-height:1;cursor:pointer;" +
    "background:#eff6ff;color:#1e40af;font-family:inherit;" +
    "white-space:nowrap;transition:background .15s ease,color .15s ease," +
    "transform .12s ease;";

  function makeButton(label, title) {
    var button = document.createElement("button");
    button.type = "button";
    button.textContent = label;
    if (title) button.title = title;
    button.style.cssText = BUTTON_STYLE;
    button.addEventListener("mouseenter", function () {
      if (button.getAttribute("aria-pressed") === "true") return;
      button.style.background = "#dbeafe";
    });
    button.addEventListener("mouseleave", function () {
      if (button.getAttribute("aria-pressed") === "true") return;
      button.style.background = "#eff6ff";
    });
    button.addEventListener("pointerdown", function () {
      button.style.transform = "scale(.96)";
    });
    button.addEventListener("pointerup", function () {
      button.style.transform = "";
    });
    button.addEventListener("pointerleave", function () {
      button.style.transform = "";
    });
    panel.appendChild(button);
    return button;
  }

  var quizButton = makeButton("クイズ", "解答を隠して自習モードにする（Q）");
  var revealButton = makeButton("全表示", "解答をすべて表示する（R）");
  revealButton.style.display = "none";
  var counter = document.createElement("span");
  counter.setAttribute("aria-live", "polite");
  counter.style.cssText =
    "padding:0 4px;font-size:12px;font-weight:700;color:#64748b;" +
    "font-variant-numeric:tabular-nums;display:none;";
  panel.appendChild(counter);

  var topButton = makeButton("↑", "ページの先頭へ（T）");

  /* ------------------------------------------------------------------ */
  /* クイズモード                                                       */
  /* ------------------------------------------------------------------ */

  var answers = Array.prototype.slice.call(
    root.querySelectorAll('p[data-answer="1"]')
  );
  if (!answers.length) {
    answers = Array.prototype.slice.call(root.querySelectorAll("p")).filter(
      function (p) {
        return p.textContent.replace(/^\s+/, "").indexOf("解答：") === 0;
      }
    );
    answers.forEach(function (p) {
      p.setAttribute("data-answer", "1");
    });
  }

  var total = answers.length;
  var revealed = 0;
  var quizOn = false;
  var celebrated = false;

  answers.forEach(function (p, index) {
    p.setAttribute("data-revealed", "0");

    p.addEventListener("click", function () {
      if (!quizOn) return;
      revealAnswer(p);
    });

    p.addEventListener("keydown", function (event) {
      if (!quizOn) return;
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        revealAnswer(p);
      }
    });

    if (index === 0) {
      p.setAttribute("data-first-answer", "1");
    }
  });

  function updateCounter() {
    counter.textContent = revealed + " / " + total;
    if (revealed === total && total > 0) {
      counter.style.color = "#047857";
    } else {
      counter.style.color = "#64748b";
    }
  }

  function blurAnswer(p) {
    p.style.filter = "blur(5px)";
    p.style.cursor = "zoom-in";
    p.style.userSelect = "none";
    p.style.transition = "filter .25s ease";
    p.style.outline = "1px dashed rgba(16,185,129,.45)";
    p.style.outlineOffset = "2px";
    p.setAttribute("aria-label", "解答を表示（クリックまたは Enter）");
  }

  function unblurAnswer(p) {
    p.style.filter = "";
    p.style.cursor = "";
    p.style.userSelect = "";
    p.style.outline = "";
    p.style.outlineOffset = "";
  }

  function revealAnswer(p) {
    if (p.getAttribute("data-revealed") === "1") return;
    p.setAttribute("data-revealed", "1");
    p.style.filter = "none";
    p.style.cursor = "pointer";
    p.style.userSelect = "";
    p.style.outline = "";
    p.style.outlineOffset = "";
    p.setAttribute("aria-label", "解答を表示しました");
    revealed += 1;
    updateCounter();

    if (!reduceMotion) {
      p.animate(
        [
          { boxShadow: "0 0 0 0 rgba(16,185,129,.55)" },
          { boxShadow: "0 0 0 10px rgba(16,185,129,0)" },
        ],
        { duration: 620, easing: "ease-out" }
      );
    }

    if (quizOn && total > 0 && revealed === total && !celebrated) {
      celebrated = true;
      toast("全 " + total + " 問、解答を表示しました。おつかれさまでした！");
      burst(window.innerWidth / 2, window.innerHeight * 0.36, 40);
    }
  }

  function setQuiz(enabled, silent) {
    quizOn = enabled;
    celebrated = false;

    quizButton.setAttribute("aria-pressed", enabled ? "true" : "false");
    quizButton.textContent = enabled ? "クイズ中" : "クイズ";
    quizButton.style.background = enabled ? "#1e3a8a" : "#eff6ff";
    quizButton.style.color = enabled ? "#ffffff" : "#1e40af";
    quizButton.style.boxShadow = enabled
      ? "0 3px 10px rgba(30,58,138,.32)"
      : "";

    revealButton.style.display = enabled ? "" : "none";
    counter.style.display = enabled ? "" : "none";

    revealed = 0;
    celebrated = false;

    answers.forEach(function (p) {
      p.setAttribute("data-revealed", "0");
      if (enabled) {
        p.setAttribute("tabindex", "0");
        p.setAttribute("role", "button");
        blurAnswer(p);
      } else {
        p.removeAttribute("tabindex");
        p.removeAttribute("role");
        p.removeAttribute("aria-label");
        unblurAnswer(p);
      }
    });

    updateCounter();
    store(STORAGE_KEY, enabled ? "1" : "0");

    if (enabled && !silent) {
      toast("クイズモード：解答はクリックで表示（Q で解除）");
    }
  }

  quizButton.addEventListener("click", function () {
    setQuiz(!quizOn);
  });

  revealButton.addEventListener("click", function () {
    if (!quizOn) return;
    answers.forEach(revealAnswer);
  });

  topButton.addEventListener("click", function () {
    window.scrollTo({
      top: 0,
      behavior: reduceMotion ? "auto" : "smooth",
    });
  });

  if (total > 0 && restore(STORAGE_KEY) === "1") {
    setQuiz(true, true);
  } else if (total === 0) {
    quizButton.style.display = "none";
    revealButton.style.display = "none";
  }

  /* ------------------------------------------------------------------ */
  /* コードブロックのコピー                                             */
  /* ------------------------------------------------------------------ */

  Array.prototype.forEach.call(root.querySelectorAll("pre"), function (pre) {
    var wrapper = document.createElement("div");
    wrapper.style.cssText = "position:relative;margin:18px 0 24px;";
    pre.style.margin = "0";
    pre.parentNode.insertBefore(wrapper, pre);
    wrapper.appendChild(pre);

    var button = document.createElement("button");
    button.type = "button";
    button.textContent = "コピー";
    button.title = "このコードをコピー";
    button.style.cssText =
      "position:absolute;top:8px;right:8px;z-index:2;appearance:none;" +
      "border:1px solid rgba(148,163,184,.55);border-radius:6px;" +
      "padding:4px 10px;font-family:inherit;font-size:11.5px;" +
      "font-weight:700;line-height:1.5;cursor:pointer;" +
      "color:#e2e8f0;background:rgba(15,23,42,.72);" +
      "transition:background .15s ease,color .15s ease;";
    wrapper.appendChild(button);

    button.addEventListener("click", function () {
      copyText(pre.innerText)
        .then(function () {
          button.textContent = "コピー済み";
          button.style.background = "#10b981";
          button.style.borderColor = "#10b981";
          button.style.color = "#ffffff";
          window.setTimeout(function () {
            button.textContent = "コピー";
            button.style.background = "rgba(15,23,42,.72)";
            button.style.borderColor = "rgba(148,163,184,.55)";
            button.style.color = "#e2e8f0";
          }, 1600);
        })
        .catch(function () {
          toast("コピーできませんでした。手動で選択してください。");
        });
    });
  });

  /* ------------------------------------------------------------------ */
  /* 目次：スムーズスクロール＋現在地ハイライト                         */
  /* ------------------------------------------------------------------ */

  // 環境によって id が落ちても、目次リンクが動くように補完する
  Array.prototype.forEach.call(
    root.querySelectorAll("h2,h3,h4"),
    function (heading, index) {
      if (!heading.id) {
        heading.id = "sec-" + (index + 1);
      }
    }
  );

  var toc = root.querySelector('[aria-label="目次"]');
  var tocLinks = toc
    ? Array.prototype.slice.call(toc.querySelectorAll('a[href^="#"]'))
    : [];

  tocLinks.forEach(function (link) {
    link.style.padding = "1px 4px";
    link.style.margin = "0 -4px";
    link.style.borderRadius = "4px";
    link.style.transition = "background .15s ease,color .15s ease";

    link.addEventListener("click", function (event) {
      var id = link.getAttribute("href").slice(1);
      var target = document.getElementById(id);
      if (!target) return;
      event.preventDefault();
      target.scrollIntoView({
        behavior: reduceMotion ? "auto" : "smooth",
        block: "start",
      });
      if (window.history && window.history.replaceState) {
        window.history.replaceState(null, "", "#" + id);
      }
    });
  });

  var activeLink = null;

  function updateActiveToc() {
    if (!tocLinks.length) return;

    var line = window.scrollY + 140;
    var current = null;

    tocLinks.forEach(function (link) {
      var id = link.getAttribute("href").slice(1);
      var heading = document.getElementById(id);
      if (!heading) return;
      if (heading.getBoundingClientRect().top + window.scrollY <= line) {
        current = link;
      }
    });

    if (current === activeLink) return;

    if (activeLink) {
      activeLink.style.background = "";
      activeLink.style.color = "";
      activeLink.style.fontWeight = "";
    }

    activeLink = current;

    if (activeLink) {
      activeLink.style.background = "#dbeafe";
      activeLink.style.color = "#1e3a8a";
      activeLink.style.fontWeight = "700";
    }
  }

  /* ------------------------------------------------------------------ */
  /* 見出しクリックでリンクをコピー                                     */
  /* ------------------------------------------------------------------ */

  Array.prototype.forEach.call(
    root.querySelectorAll("h2[id],h3[id],h4[id]"),
    function (heading) {
      heading.style.cursor = "pointer";
      heading.title = "クリックでこの見出しの URL をコピー";

      heading.addEventListener("click", function () {
        var selected = window.getSelection
          ? String(window.getSelection())
          : "";
        if (selected.length > 0) return;

        var url =
          location.origin +
          location.pathname +
          location.search +
          "#" +
          heading.id;

        copyText(url)
          .then(function () {
            toast("見出しリンクをコピーしました");
          })
          .catch(function () {
            toast("コピーできませんでした");
          });
      });
    }
  );

  /* ------------------------------------------------------------------ */
  /* VXLAN カプセル化ミニクイズ                                         */
  /* ------------------------------------------------------------------ */

  (function initPuzzle() {
    var box = root.querySelector('[data-widget="vxlan-puzzle"]');
    if (!box) return;

    var chips = Array.prototype.slice.call(
      box.querySelectorAll("[data-order]")
    );
    var slotRow = box.querySelector('[data-role="slots"]');
    var status = box.querySelector('[data-role="status"]');
    var resetButton = box.querySelector('[data-role="reset"]');
    var hintButton = box.querySelector('[data-role="hint"]');
    var hintText = box.querySelector('[data-role="hint-text"]');
    if (!chips.length || !slotRow) return;

    var picked = [];
    var checked = false;

    function chipBaseStyle(chip) {
      chip.disabled = false;
      chip.style.opacity = "";
      chip.style.cursor = "pointer";
      chip.style.transform = "";
      chip.style.background = "#ffffff";
      chip.style.color = "#1e3a8a";
    }

    function render() {
      slotRow.innerHTML = "";

      for (var i = 0; i < 4; i++) {
        var slot = document.createElement("span");
        slot.style.cssText =
          "display:inline-flex;align-items:center;justify-content:center;" +
          "min-width:92px;min-height:36px;padding:6px 10px;" +
          "border-radius:8px;border:1px dashed #93c5fd;" +
          "background:#f8fafc;color:#1e3a8a;font-size:12px;" +
          "font-weight:700;text-align:center;line-height:1.35;" +
          "transition:background .15s ease,border-color .15s ease;";

        var chip = picked[i];

        if (chip) {
          slot.textContent = chip.textContent;

          if (checked) {
            var good = Number(chip.getAttribute("data-order")) === i + 1;
            slot.style.borderStyle = "solid";
            slot.style.borderColor = good ? "#10b981" : "#ef4444";
            slot.style.background = good ? "#ecfdf5" : "#fef2f2";
            slot.style.color = good ? "#065f46" : "#991b1b";
          } else {
            slot.style.borderStyle = "solid";
            slot.style.borderColor = "#60a5fa";
            slot.style.background = "#dbeafe";
          }
        } else {
          slot.textContent = "？";
        }

        slotRow.appendChild(slot);

        if (i < 3) {
          var arrow = document.createElement("span");
          arrow.textContent = "→";
          arrow.setAttribute("aria-hidden", "true");
          arrow.style.cssText =
            "color:#93c5fd;font-size:12px;font-weight:700;";
          slotRow.appendChild(arrow);
        }
      }
    }

    function check() {
      checked = true;
      var ok = picked.every(function (chip, index) {
        return Number(chip.getAttribute("data-order")) === index + 1;
      });

      render();

      if (ok) {
        status.textContent =
          "正解！外側から IPv4 → UDP → VXLAN → イーサネットフレームの順です。";
        status.style.color = "#047857";
        chips.forEach(function (chip) {
          chip.disabled = true;
          chip.style.cursor = "default";
        });
        var rect = box.getBoundingClientRect();
        burst(rect.left + rect.width / 2, rect.top + 40, 18);
      } else {
        status.textContent =
          "おしい！外側（トンネルの入口）から順番に選び直してみましょう。";
        status.style.color = "#b91c1c";
      }
    }

    chips.forEach(function (chip) {
      chipBaseStyle(chip);

      chip.addEventListener("click", function () {
        if (chip.disabled || picked.length >= 4) return;

        picked.push(chip);
        chip.disabled = true;
        chip.style.opacity = ".35";
        chip.style.cursor = "default";
        chip.style.transform = "scale(.97)";
        chip.style.background = "#e2e8f0";
        chip.style.color = "#64748b";

        checked = false;
        status.textContent = "";
        render();

        if (picked.length === 4) check();
      });
    });

    if (resetButton) {
      resetButton.addEventListener("click", function () {
        picked = [];
        checked = false;
        status.textContent = "";
        chips.forEach(chipBaseStyle);
        render();
      });
    }

    if (hintButton && hintText) {
      hintButton.addEventListener("click", function () {
        var shown = hintText.style.display !== "none";
        hintText.style.display = shown ? "none" : "";
        hintButton.textContent = shown ? "ヒント" : "ヒントを隠す";
      });
    }

    render();
  })();

  /* ------------------------------------------------------------------ */
  /* 関連記事カード                                                     */
  /* ------------------------------------------------------------------ */

  (function setupRelatedCards() {
    var cards = root.querySelectorAll("[data-related-card]");
    if (!cards.length) return;

    function activate(card) {
      if (!reduceMotion) {
        card.style.transform = "translateY(-2px)";
      }
      card.style.boxShadow = "0 10px 24px rgba(30,58,138,.16)";
      card.style.backgroundColor = "#f8fbff";
    }

    function deactivate(card) {
      card.style.transform = "";
      card.style.boxShadow = "";
      card.style.backgroundColor = "";
    }

    Array.prototype.forEach.call(cards, function (card) {
      card.addEventListener("mouseenter", function () {
        activate(card);
      });
      card.addEventListener("mouseleave", function () {
        deactivate(card);
      });
      card.addEventListener("focus", function () {
        activate(card);
      });
      card.addEventListener("blur", function () {
        deactivate(card);
      });
    });
  })();

  /* ------------------------------------------------------------------ */
  /* キーボードショートカット                                           */
  /* ------------------------------------------------------------------ */

  document.addEventListener("keydown", function (event) {
    if (event.metaKey || event.ctrlKey || event.altKey) return;

    var target = event.target;
    if (
      target &&
      (target.isContentEditable ||
        /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName))
    ) {
      return;
    }

    var key = event.key.toLowerCase();

    if (key === "q" || key === "Q") {
      if (total === 0) return;
      event.preventDefault();
      setQuiz(!quizOn);
    } else if (key === "r") {
      if (!quizOn) return;
      event.preventDefault();
      answers.forEach(revealAnswer);
    } else if (key === "t") {
      event.preventDefault();
      window.scrollTo({
        top: 0,
        behavior: reduceMotion ? "auto" : "smooth",
      });
    } else if (event.key === "?") {
      event.preventDefault();
      toast("Q: クイズ切替 / R: 解答を全表示 / T: 先頭へ / 見出しクリック: リンクをコピー");
    }
  });

  /* ------------------------------------------------------------------ */
  /* スクロールイベント                                                 */
  /* ------------------------------------------------------------------ */

  var ticking = false;

  function onScroll() {
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(function () {
      ticking = false;
      updateProgress();
      updateActiveToc();
    });
  }

  window.addEventListener("scroll", onScroll, { passive: true });
  window.addEventListener("resize", onScroll);
  onScroll();
})();
