(function () {
  "use strict";
  var root = document.documentElement;
  var port = null;
  var listeners = {};
  var state = { settings: {}, globals: {}, theme: "dark", language: root.lang || navigator.language || "en" };
  var resolveReady;
  var ready = new Promise(function (resolve) { resolveReady = resolve; });

  var STRINGS = {
    en: { empty: "Not set", recording: "Press a key combination…", ok: "OK", cancel: "Cancel", less: "Decrease", more: "Increase", hue: "Hue", shade: "Saturation and brightness" },
    ru: { empty: "Не задано", recording: "Нажмите сочетание…", ok: "ОК", cancel: "Отмена", less: "Меньше", more: "Больше", hue: "Оттенок", shade: "Насыщенность и яркость" },
    zh: { empty: "未设置", recording: "请按下组合键…", ok: "确定", cancel: "取消", less: "减少", more: "增加", hue: "色相", shade: "饱和度和亮度" },
  };
  var ICONS = {
    chevron: '<svg class="select__chevron" width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m3.5 5.25 3.5 3.5 3.5-3.5"/></svg>',
    minus: '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M3 7h8"/></svg>',
    plus: '<svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M7 3v8M3 7h8"/></svg>',
  };
  var MODIFIERS = ["Control", "Shift", "Alt", "Meta"];
  var META = /Mac/.test(navigator.userAgent) ? "Cmd" : /Windows/.test(navigator.userAgent) ? "Win" : "Super";
  var KEY_NAMES = { " ": "Space", ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→", Escape: "Esc" };

  function text(key) {
    return (STRINGS[state.language.slice(0, 2)] || STRINGS.en)[key];
  }

  function make(tag, className, html) {
    var el = document.createElement(tag);
    if (className) el.className = className;
    if (html) el.innerHTML = html;
    return el;
  }

  function fire(el, type) {
    el.dispatchEvent(new Event(type, { bubbles: true }));
  }

  function emit(type, value) {
    (listeners[type] || []).forEach(function (fn) {
      try { fn(value); } catch (e) { console.error(e); }
    });
  }

  function post(message) {
    if (port) port.postMessage(message);
    else ready.then(function () { port.postMessage(message); });
  }

  function applyTheme(theme) {
    state.theme = theme === "light" ? "light" : "dark";
    root.dataset.theme = state.theme;
  }

  applyTheme(window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark");

  function bindingOf(el) {
    return el.dataset.setting !== undefined ? ["data-setting", el.dataset.setting] : ["data-global", el.dataset.global];
  }

  function checkboxGroup(el) {
    var binding = bindingOf(el);
    return document.querySelectorAll('input[type="checkbox"][' + binding[0] + '="' + CSS.escape(binding[1]) + '"]');
  }

  function fieldValue(el) {
    if (el.type === "checkbox") {
      var group = checkboxGroup(el);
      if (group.length < 2) return el.checked;
      return Array.prototype.filter.call(group, function (box) { return box.checked; }).map(function (box) { return box.value; });
    }
    if (el.type === "number" || el.type === "range") return el.value === "" ? null : Number(el.value);
    return el.value;
  }

  function show(el, value) {
    if (value === undefined || value === null) return;
    if (el.type === "checkbox") el.checked = Array.isArray(value) ? value.indexOf(el.value) >= 0 : value === true;
    else if (el.type === "radio") el.checked = String(value) === el.value;
    else if (document.activeElement !== el) el.value = String(value);
  }

  function refresh(scope) {
    scope = scope || document;
    scope.querySelectorAll("[data-setting]").forEach(function (el) { show(el, state.settings[el.dataset.setting]); });
    scope.querySelectorAll("[data-global]").forEach(function (el) { show(el, state.globals[el.dataset.global]); });
    sync(scope);
  }

  function save(el) {
    var key = bindingOf(el)[1];
    var patch = {};
    patch[key] = fieldValue(el);
    if (el.dataset.setting !== undefined) {
      state.settings = Object.assign({}, state.settings, patch);
      post({ type: "saveSettings", settings: state.settings });
    } else {
      state.globals = Object.assign({}, state.globals, patch);
      post({ type: "saveGlobals", settings: state.globals });
    }
    refresh();
  }

  function bind(scope) {
    scope = scope || document;
    enhance(scope);
    scope.querySelectorAll("[data-setting], [data-global]").forEach(function (el) {
      if (el.__opensdBound) return;
      el.__opensdBound = true;
      var event = el.tagName === "SELECT" || ["checkbox", "radio", "color"].indexOf(el.type) >= 0 ? "change" : "input";
      el.addEventListener(event, function () {
        if (el.type === "radio" && !el.checked) return;
        save(el);
      });
    });
    refresh(scope);
  }

  function sync(scope) {
    scope = scope || document;
    scope.querySelectorAll('input[type="range"]').forEach(function (el) {
      var min = Number(el.min || 0);
      var max = Number(el.max || 100);
      el.style.setProperty("--fill", ((Number(el.value) - min) / (max - min || 1)) * 100 + "%");
      var slider = el.closest(".slider");
      var value = slider && slider.querySelector(".slider__value");
      if (value) value.textContent = el.value + (el.dataset.unit || "");
    });
    scope.querySelectorAll(".segmented").forEach(function (group) {
      if (!group.querySelector(".segmented__thumb")) group.prepend(make("span", "segmented__thumb"));
      var items = group.querySelectorAll(".segmented__item");
      var index = 0;
      items.forEach(function (item, i) { if (item.querySelector("input:checked")) index = i; });
      group.style.setProperty("--count", items.length);
      group.style.setProperty("--index", index);
    });
    scope.querySelectorAll(".swatch--custom").forEach(function (label) {
      var input = label.querySelector('input[type="color"]');
      var group = label.closest(".swatches");
      if (!input) return;
      label.style.setProperty("--swatch", input.value);
      label.classList.toggle("is-active", !(group && group.querySelector("input[type=radio]:checked")));
    });
    scope.querySelectorAll("select.is-enhanced").forEach(syncSelect);
    scope.querySelectorAll("input.is-enhanced[data-hotkey]").forEach(function (input) { input.__render(); });
    scope.querySelectorAll('input.is-enhanced[type="color"]').forEach(syncColor);
    updateBars();
  }

  function enhance(scope) {
    scope = scope || document;
    scope.querySelectorAll("select:not([multiple]):not([size])").forEach(enhanceSelect);
    scope.querySelectorAll(".stepper").forEach(enhanceStepper);
    scope.querySelectorAll("input[data-hotkey]").forEach(enhanceHotkey);
    scope.querySelectorAll(".tabs").forEach(enhanceTabs);
    scope.querySelectorAll(".dropzone").forEach(enhanceDropzone);
    scope.querySelectorAll('input[type="color"]').forEach(enhanceColor);
    scope.querySelectorAll("textarea, pre, .scroll").forEach(attachScroll);
    if (!pageFade && document.body) {
      pageFade = make("div", "page-fade");
      document.body.append(pageFade);
      attachScroll(document.scrollingElement);
    }
    updateBars();
  }

  var menu = null;

  function place(popup, anchor) {
    var rect = anchor.getBoundingClientRect();
    var below = window.innerHeight - rect.bottom - 14;
    var above = rect.top - 14;
    var up = below < popup.scrollHeight && above > below;
    popup.style.maxHeight = Math.max(120, up ? above : below) + "px";
    var width = popup.offsetWidth;
    popup.style.left = Math.max(8, Math.min(rect.left, window.innerWidth - width - 8)) + "px";
    popup.style.top = (up ? rect.top - 6 - popup.offsetHeight : rect.bottom + 6) + "px";
    return up;
  }

  function outside(popup, target) {
    return popup && !popup.contains(target) && !popup.__trigger.contains(target);
  }

  var PRESETS = ["#ffffff", "#c7c7cc", "#ff453a", "#ff9f0a", "#ffd60a", "#30d158", "#64d2ff", "#0a84ff", "#bf5af2", "#ff375f"];
  var picker = null;

  function clamp(value) {
    return Math.min(1, Math.max(0, value));
  }

  function normalizeHex(value) {
    var match = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(String(value).trim());
    if (!match) return null;
    return "#" + (match[1].length === 3 ? match[1].replace(/./g, "$&$&") : match[1]).toLowerCase();
  }

  function hexToHsv(hex) {
    var n = parseInt((normalizeHex(hex) || "#000000").slice(1), 16);
    var r = ((n >> 16) & 255) / 255;
    var g = ((n >> 8) & 255) / 255;
    var b = (n & 255) / 255;
    var max = Math.max(r, g, b);
    var d = max - Math.min(r, g, b);
    var h = d === 0 ? 0 : max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
    return { h: (h * 60 + 360) % 360, s: max ? d / max : 0, v: max };
  }

  function hsvToHex(c) {
    function channel(n) {
      var k = (n + c.h / 60) % 6;
      return Math.round((c.v - c.v * c.s * Math.max(0, Math.min(k, 4 - k, 1))) * 255);
    }
    return "#" + [channel(5), channel(3), channel(1)].map(function (x) { return x.toString(16).padStart(2, "0"); }).join("");
  }

  function enhanceColor(input) {
    if (input.__opensdUi) return;
    input.__opensdUi = true;
    var trigger = input.closest(".swatch--custom");
    if (trigger) trigger.tabIndex = 0;
    else {
      trigger = make("button", "color", '<span class="color__swatch"></span><span class="color__value"></span>');
      trigger.type = "button";
      input.after(trigger);
    }
    input.classList.add("is-enhanced");
    input.__trigger = trigger;
    trigger.setAttribute("aria-haspopup", "dialog");
    trigger.setAttribute("aria-expanded", "false");
    trigger.addEventListener("click", function (e) {
      e.preventDefault();
      if (input.disabled) return;
      if (picker && picker.__input === input) closePicker();
      else openPicker(input);
    });
    trigger.addEventListener("keydown", function (e) {
      if (trigger.tagName === "BUTTON" || (e.key !== "Enter" && e.key !== " ")) return;
      e.preventDefault();
      trigger.click();
    });
    syncColor(input);
  }

  function syncColor(input) {
    var trigger = input.__trigger;
    if (!trigger || trigger.tagName !== "BUTTON") return;
    trigger.style.setProperty("--color", input.value);
    trigger.querySelector(".color__value").textContent = input.value.toUpperCase();
    trigger.disabled = input.disabled;
  }

  function openPicker(input) {
    closeMenu();
    closePicker();
    var hsv = hexToHsv(input.value);
    var presets = input.dataset.colors === undefined ? PRESETS : input.dataset.colors.split(",").map(normalizeHex).filter(Boolean);
    picker = make(
      "div",
      "color-picker",
      '<div class="color-picker__area"><span class="color-picker__handle" tabindex="0" role="slider"></span></div>' +
        '<div class="color-picker__hue"><span class="color-picker__handle" tabindex="0" role="slider"></span></div>' +
        (presets.length ? '<div class="color-picker__swatches"></div>' : "") +
        '<div class="color-picker__row"><span class="color-picker__preview"></span><input spellcheck="false" maxlength="7" aria-label="HEX"></div>',
    );
    picker.setAttribute("role", "dialog");
    picker.__input = input;
    picker.__trigger = input.__trigger;
    var area = picker.querySelector(".color-picker__area");
    var hue = picker.querySelector(".color-picker__hue");
    var areaHandle = area.firstElementChild;
    var hueHandle = hue.firstElementChild;
    var field = picker.querySelector("input");
    var swatches = picker.querySelector(".color-picker__swatches");
    areaHandle.setAttribute("aria-label", text("shade"));
    hueHandle.setAttribute("aria-label", text("hue"));
    presets.forEach(function (color) {
      var swatch = make("button", "swatch");
      swatch.type = "button";
      swatch.style.setProperty("--swatch", color);
      swatch.dataset.color = color;
      swatch.setAttribute("aria-label", color);
      swatch.addEventListener("click", function () {
        hsv = hexToHsv(color);
        apply(true);
      });
      swatches.append(swatch);
    });

    function render(typing) {
      var hex = hsvToHex(hsv);
      picker.style.setProperty("--hue", hsv.h);
      picker.style.setProperty("--color", hex);
      areaHandle.style.left = hsv.s * 100 + "%";
      areaHandle.style.top = (1 - hsv.v) * 100 + "%";
      hueHandle.style.left = (hsv.h / 360) * 100 + "%";
      if (!typing) field.value = hex.toUpperCase();
      if (swatches) swatches.querySelectorAll(".swatch").forEach(function (s) { s.classList.toggle("is-active", s.dataset.color === hex); });
      return hex;
    }

    function apply(commit, typing) {
      var hex = render(typing);
      if (input.value !== hex) {
        input.value = hex;
        fire(input, "input");
      }
      if (commit) fire(input, "change");
    }

    function drag(el, move) {
      el.addEventListener("pointerdown", function (e) {
        e.preventDefault();
        el.setPointerCapture(e.pointerId);
        el.firstElementChild.focus({ preventScroll: true });
        move(e);
        function onMove(ev) { move(ev); }
        function onUp() {
          el.removeEventListener("pointermove", onMove);
          el.removeEventListener("pointerup", onUp);
          el.removeEventListener("pointercancel", onUp);
          apply(true);
        }
        el.addEventListener("pointermove", onMove);
        el.addEventListener("pointerup", onUp);
        el.addEventListener("pointercancel", onUp);
      });
    }

    drag(area, function (e) {
      var r = area.getBoundingClientRect();
      hsv.s = clamp((e.clientX - r.left) / r.width);
      hsv.v = 1 - clamp((e.clientY - r.top) / r.height);
      apply(false);
    });
    drag(hue, function (e) {
      var r = hue.getBoundingClientRect();
      hsv.h = clamp((e.clientX - r.left) / r.width) * 359.99;
      apply(false);
    });

    areaHandle.addEventListener("keydown", function (e) {
      var step = e.shiftKey ? 0.1 : 0.01;
      var dx = { ArrowLeft: -step, ArrowRight: step }[e.key] || 0;
      var dy = { ArrowDown: -step, ArrowUp: step }[e.key] || 0;
      if (!dx && !dy) return;
      e.preventDefault();
      hsv.s = clamp(hsv.s + dx);
      hsv.v = clamp(hsv.v + dy);
      apply(true);
    });
    hueHandle.addEventListener("keydown", function (e) {
      var step = { ArrowLeft: -1, ArrowDown: -1, ArrowRight: 1, ArrowUp: 1 }[e.key];
      if (!step) return;
      e.preventDefault();
      hsv.h = Math.min(359.99, Math.max(0, hsv.h + step * (e.shiftKey ? 10 : 1)));
      apply(true);
    });
    field.addEventListener("input", function () {
      var hex = normalizeHex(field.value);
      if (!hex) return;
      hsv = hexToHsv(hex);
      apply(true, true);
    });
    field.addEventListener("blur", function () { render(); });
    picker.addEventListener("keydown", function (e) {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      var trigger = picker.__trigger;
      closePicker();
      trigger.focus();
    });

    document.body.append(picker);
    render();
    picker.classList.toggle("color-picker--up", place(picker, input.__trigger));
    input.__trigger.setAttribute("aria-expanded", "true");
    areaHandle.focus({ preventScroll: true });
  }

  function closePicker() {
    if (!picker) return;
    var closing = picker;
    picker = null;
    closing.__trigger.setAttribute("aria-expanded", "false");
    closing.classList.add("color-picker--closing");
    setTimeout(function () { closing.remove(); }, 160);
  }

  var bars = [];
  var pageFade = null;

  function attachScroll(scroller) {
    if (scroller.__bar) return;
    var page = scroller === document.scrollingElement;
    var bar = make("div", "scrollbar");
    var thumb = make("div", "scrollbar__thumb");
    var timer = 0;
    var drag = null;
    var entry = { scroller: scroller, update: update, show: show };
    bar.append(thumb);
    bar.hidden = true;
    scroller.__bar = bar;
    document.body.append(bar);

    function show() {
      bar.classList.add("is-visible");
      clearTimeout(timer);
      timer = setTimeout(function () {
        if (!drag && !bar.matches(":hover")) bar.classList.remove("is-visible");
      }, 900);
    }

    function update() {
      if (!page && !scroller.isConnected) {
        bar.remove();
        bars.splice(bars.indexOf(entry), 1);
        return;
      }
      var overflow = scroller.scrollHeight - scroller.clientHeight;
      var top = scroller.scrollTop;
      if (page && pageFade) {
        pageFade.classList.toggle("is-top", top > 1);
        pageFade.classList.toggle("is-bottom", overflow - top > 1);
      }
      if (scroller.classList.contains("scroll")) {
        scroller.style.setProperty("--fade-top", Math.min(28, top) + "px");
        scroller.style.setProperty("--fade-bottom", Math.min(28, Math.max(0, overflow - top)) + "px");
      }
      var r = page ? { top: 0, bottom: window.innerHeight, right: window.innerWidth } : scroller.getBoundingClientRect();
      var inset = page ? 6 : 4;
      var track = r.bottom - r.top - inset * 2;
      bar.hidden = overflow <= 1 || track < 32;
      if (bar.hidden) return;
      var size = Math.max(24, (track * scroller.clientHeight) / scroller.scrollHeight);
      bar.style.top = r.top + inset + "px";
      bar.style.left = r.right - 10 + "px";
      bar.style.height = track + "px";
      thumb.style.height = size + "px";
      thumb.style.transform = "translateY(" + ((track - size) * top) / overflow + "px)";
    }

    bar.addEventListener("pointerenter", show);
    bar.addEventListener("pointerleave", show);
    thumb.addEventListener("pointerdown", function (e) {
      e.preventDefault();
      e.stopPropagation();
      thumb.setPointerCapture(e.pointerId);
      drag = { y: e.clientY, top: scroller.scrollTop };
      bar.classList.add("is-dragging");
      show();
    });
    thumb.addEventListener("pointermove", function (e) {
      if (!drag) return;
      var track = bar.clientHeight - thumb.offsetHeight;
      if (track > 0) scroller.scrollTop = drag.top + ((e.clientY - drag.y) * (scroller.scrollHeight - scroller.clientHeight)) / track;
    });
    function release() {
      drag = null;
      bar.classList.remove("is-dragging");
      show();
    }
    thumb.addEventListener("pointerup", release);
    thumb.addEventListener("pointercancel", release);
    bar.addEventListener("pointerdown", function (e) {
      if (e.target !== bar) return;
      e.preventDefault();
      var below = e.clientY > thumb.getBoundingClientRect().top;
      scroller.scrollBy({ top: (below ? 1 : -1) * scroller.clientHeight * 0.9, behavior: "smooth" });
    });

    var observer = new ResizeObserver(update);
    observer.observe(page ? document.body : scroller);
    if (!page && scroller.firstElementChild) observer.observe(scroller.firstElementChild);
    bars.push(entry);
    update();
  }

  function updateBars() {
    bars.slice().forEach(function (entry) { entry.update(); });
  }

  window.addEventListener("scroll", function (e) {
    var scroller = e.target === document ? document.scrollingElement : e.target;
    bars.slice().forEach(function (entry) {
      entry.update();
      if (entry.scroller === scroller) entry.show();
    });
  }, { capture: true, passive: true });
  window.addEventListener("resize", updateBars);


  function enhanceSelect(select) {
    if (select.__opensdUi) return;
    select.__opensdUi = true;
    var trigger = make("button", "select", '<span class="select__value"></span>' + ICONS.chevron);
    trigger.type = "button";
    trigger.setAttribute("aria-haspopup", "listbox");
    trigger.setAttribute("aria-expanded", "false");
    if (select.getAttribute("aria-label")) trigger.setAttribute("aria-label", select.getAttribute("aria-label"));
    if (select.id) trigger.id = select.id + "-select";
    select.classList.add("is-enhanced");
    select.after(trigger);
    select.__trigger = trigger;
    trigger.addEventListener("click", function () { menu && menu.__select === select ? closeMenu() : openMenu(select); });
    trigger.addEventListener("keydown", function (e) {
      if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
      e.preventDefault();
      openMenu(select);
    });
    syncSelect(select);
  }

  function syncSelect(select) {
    var trigger = select.__trigger;
    var option = select.options[select.selectedIndex];
    var value = trigger.querySelector(".select__value");
    value.textContent = option ? option.textContent : "";
    value.dataset.placeholder = select.dataset.placeholder || "";
    trigger.disabled = select.disabled;
    if (select.getAttribute("aria-invalid")) trigger.setAttribute("aria-invalid", select.getAttribute("aria-invalid"));
    else trigger.removeAttribute("aria-invalid");
  }

  function openMenu(select) {
    closeMenu();
    var trigger = select.__trigger;
    menu = make("div", "select__menu");
    menu.setAttribute("role", "listbox");
    menu.__select = select;
    menu.__trigger = trigger;
    Array.prototype.forEach.call(select.children, function (child) {
      if (child.tagName === "OPTGROUP") {
        var group = make("div", "select__group");
        group.textContent = child.label;
        menu.append(group);
        Array.prototype.forEach.call(child.children, addOption);
      } else addOption(child);
    });
    function addOption(option) {
      if (option.hidden) return;
      var item = make("button", "select__option");
      item.type = "button";
      item.setAttribute("role", "option");
      item.setAttribute("aria-selected", option.selected);
      item.disabled = option.disabled;
      item.append(option.textContent);
      if (option.dataset.hint) {
        var hint = make("span", "select__hint");
        hint.textContent = option.dataset.hint;
        item.append(hint);
      }
      item.addEventListener("click", function () {
        select.value = option.value;
        fire(select, "input");
        fire(select, "change");
        closeMenu();
        trigger.focus();
      });
      menu.append(item);
    }
    menu.addEventListener("keydown", menuKey);
    document.body.append(menu);

    menu.style.minWidth = trigger.offsetWidth + "px";
    menu.classList.toggle("select__menu--up", place(menu, trigger));
    trigger.setAttribute("aria-expanded", "true");

    var current = menu.querySelector('[aria-selected="true"]:not(:disabled)') || menu.querySelector(".select__option:not(:disabled)");
    if (current) {
      current.focus({ preventScroll: true });
      menu.scrollTop = current.offsetTop - menu.clientHeight / 2 + current.offsetHeight / 2;
    }
    attachScroll(menu);
  }

  function closeMenu() {
    if (!menu) return;
    var closing = menu;
    menu = null;
    closing.__trigger.setAttribute("aria-expanded", "false");
    closing.classList.add("select__menu--closing");
    setTimeout(function () {
      closing.remove();
      updateBars();
    }, 160);
  }

  function menuKey(e) {
    var items = Array.prototype.slice.call(menu.querySelectorAll(".select__option:not(:disabled)"));
    var index = items.indexOf(document.activeElement);
    var next = null;
    if (e.key === "ArrowDown") next = items[Math.min(items.length - 1, index + 1)];
    else if (e.key === "ArrowUp") next = items[Math.max(0, index - 1)];
    else if (e.key === "Home") next = items[0];
    else if (e.key === "End") next = items[items.length - 1];
    else if (e.key === "Escape" || e.key === "Tab") {
      var trigger = menu.__select.__trigger;
      e.preventDefault();
      e.stopPropagation();
      closeMenu();
      trigger.focus();
      return;
    } else if (e.key.length === 1) {
      var letter = e.key.toLowerCase();
      next = items.slice(index + 1).concat(items.slice(0, index + 1)).find(function (item) {
        return item.textContent.trim().toLowerCase().indexOf(letter) === 0;
      });
    }
    if (!next) return;
    e.preventDefault();
    next.focus({ preventScroll: true });
    if (next.offsetTop < menu.scrollTop) menu.scrollTop = next.offsetTop - 4;
    else if (next.offsetTop + next.offsetHeight > menu.scrollTop + menu.clientHeight) menu.scrollTop = next.offsetTop + next.offsetHeight - menu.clientHeight + 4;
  }

  function closePopups() {
    closeMenu();
    closePicker();
  }

  document.addEventListener("pointerdown", function (e) {
    if (outside(menu, e.target)) closeMenu();
    if (outside(picker, e.target)) closePicker();
  }, true);
  window.addEventListener("scroll", function (e) {
    if (menu && !menu.contains(e.target)) closeMenu();
    if (picker && !picker.contains(e.target)) closePicker();
  }, true);
  var viewport = window.innerWidth + "x" + window.innerHeight;
  window.addEventListener("resize", function () {
    var size = window.innerWidth + "x" + window.innerHeight;
    if (size !== viewport) closePopups();
    viewport = size;
  });
  window.addEventListener("blur", closePopups);

  function enhanceStepper(box) {
    var input = box.querySelector('input[type="number"]');
    if (!input || box.__opensdUi) return;
    box.__opensdUi = true;
    [["less", ICONS.minus, "stepDown", "before"], ["more", ICONS.plus, "stepUp", "after"]].forEach(function (spec) {
      var button = make("button", "stepper__button", spec[1]);
      button.type = "button";
      button.tabIndex = -1;
      button.setAttribute("aria-label", text(spec[0]));
      button.addEventListener("click", function () {
        if (input.disabled) return;
        input[spec[2]]();
        fire(input, "input");
        fire(input, "change");
      });
      input[spec[3]](button);
    });
  }

  function combo(e) {
    var parts = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.shiftKey) parts.push("Shift");
    if (e.altKey) parts.push("Alt");
    if (e.metaKey) parts.push(META);
    if (MODIFIERS.indexOf(e.key) >= 0) return parts;
    if (/^Key[A-Z]$/.test(e.code)) parts.push(e.code.slice(3));
    else if (/^Digit\d$/.test(e.code)) parts.push(e.code.slice(5));
    else parts.push(KEY_NAMES[e.key] || (e.key.length === 1 ? e.key.toUpperCase() : e.key));
    return parts;
  }

  function enhanceHotkey(input) {
    if (input.__opensdUi) return;
    input.__opensdUi = true;
    var button = make("button", "hotkey");
    var preview = [];
    button.type = "button";
    input.classList.add("is-enhanced");
    input.after(button);

    function render() {
      var keys = preview.length ? preview : input.value ? input.value.split(" + ") : [];
      var recording = button.classList.contains("hotkey--recording");
      var shown = keys.join(" + ") + "|" + recording + "|" + input.disabled;
      if (button.__shown === shown) return;
      button.__shown = shown;
      button.textContent = "";
      button.disabled = input.disabled;
      if (!keys.length) {
        var placeholder = make("span", "hotkey__placeholder");
        placeholder.textContent = recording ? text("recording") : input.placeholder || text("empty");
        button.append(placeholder);
        return;
      }
      keys.forEach(function (key) {
        var kbd = make("kbd");
        kbd.textContent = key;
        button.append(kbd);
      });
    }

    function set(value) {
      input.value = value;
      fire(input, "input");
      fire(input, "change");
    }

    input.__render = render;
    button.addEventListener("focus", function () {
      button.classList.add("hotkey--recording");
      render();
    });
    button.addEventListener("blur", function () {
      button.classList.remove("hotkey--recording");
      preview = [];
      render();
    });
    button.addEventListener("keydown", function (e) {
      if (e.key === "Tab" && !e.ctrlKey && !e.altKey && !e.metaKey) return;
      e.preventDefault();
      e.stopPropagation();
      if ((e.key === "Backspace" || e.key === "Delete") && !e.ctrlKey && !e.shiftKey && !e.altKey && !e.metaKey) {
        preview = [];
        set("");
        return;
      }
      preview = combo(e);
      render();
      if (MODIFIERS.indexOf(e.key) < 0) {
        set(preview.join(" + "));
        button.blur();
      }
    });
    button.addEventListener("keyup", function () {
      if (document.activeElement !== button) return;
      preview = [];
      render();
    });
    render();
  }

  function enhanceTabs(tabs) {
    var list = tabs.querySelector(".tabs__list");
    if (!list || tabs.__opensdUi) return;
    tabs.__opensdUi = true;
    var buttons = list.querySelectorAll(".tabs__tab");
    list.prepend(make("span", "segmented__thumb"));
    list.setAttribute("role", "tablist");
    list.style.setProperty("--count", buttons.length);

    function open(name) {
      buttons.forEach(function (button, i) {
        var on = button.dataset.tab === name;
        button.setAttribute("role", "tab");
        button.setAttribute("aria-selected", on);
        button.tabIndex = on ? 0 : -1;
        if (on) list.style.setProperty("--index", i);
      });
      tabs.querySelectorAll(".tabs__panel").forEach(function (panel) {
        if (panel.closest(".tabs") === tabs) panel.classList.toggle("is-active", panel.dataset.tab === name);
      });
    }

    buttons.forEach(function (button, i) {
      button.type = "button";
      button.addEventListener("click", function () {
        open(button.dataset.tab);
        tabs.dispatchEvent(new CustomEvent("tabchange", { detail: button.dataset.tab }));
      });
      button.addEventListener("keydown", function (e) {
        var step = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
        if (!step) return;
        var next = buttons[(i + step + buttons.length) % buttons.length];
        next.focus();
        next.click();
      });
    });
    var initial = list.querySelector('.tabs__tab[aria-selected="true"]') || buttons[0];
    if (initial) open(initial.dataset.tab);
  }

  function enhanceDropzone(zone) {
    var input = zone.querySelector('input[type="file"]');
    if (!input || zone.__opensdUi) return;
    zone.__opensdUi = true;
    ["dragenter", "dragover"].forEach(function (type) {
      zone.addEventListener(type, function (e) {
        e.preventDefault();
        zone.classList.add("dropzone--active");
      });
    });
    ["dragleave", "drop"].forEach(function (type) {
      zone.addEventListener(type, function () { zone.classList.remove("dropzone--active"); });
    });
    zone.addEventListener("drop", function (e) {
      e.preventDefault();
      if (!e.dataTransfer || !e.dataTransfer.files.length) return;
      input.files = e.dataTransfer.files;
      fire(input, "change");
    });
  }

  function closeDialog(dialog) {
    if (!dialog || !dialog.open || dialog.classList.contains("is-closing")) return;
    dialog.classList.add("is-closing");
    setTimeout(function () {
      dialog.classList.remove("is-closing");
      dialog.close();
    }, 200);
  }

  document.addEventListener("click", function (e) {
    var target = e.target instanceof Element ? e.target : null;
    if (!target) return;
    var opener = target.closest("[data-open]");
    var dialog = opener && document.getElementById(opener.dataset.open);
    if (dialog && dialog.showModal && !dialog.open) dialog.showModal();
    if (target.closest("[data-close]")) closeDialog(target.closest("dialog"));
    if (target.tagName === "DIALOG") {
      var r = target.getBoundingClientRect();
      var inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
      if (!inside) closeDialog(target);
    }
  });

  document.addEventListener("cancel", function (e) {
    if (e.target.tagName !== "DIALOG") return;
    e.preventDefault();
    closeDialog(e.target);
  }, true);

  function confirmDialog(options) {
    options = typeof options === "string" ? { text: options } : options || {};
    return new Promise(function (resolve) {
      var dialog = make("dialog", "", '<h2 class="dialog__title"></h2><p class="dialog__text"></p><div class="dialog__actions"><button class="button" value="0"></button><button class="button" value="1"></button></div>');
      var title = dialog.querySelector(".dialog__title");
      var body = dialog.querySelector(".dialog__text");
      var cancel = dialog.querySelector('[value="0"]');
      var ok = dialog.querySelector('[value="1"]');
      var result = false;
      title.textContent = options.title || "";
      title.hidden = !options.title;
      body.textContent = options.text || "";
      body.hidden = !options.text;
      cancel.textContent = options.cancel || text("cancel");
      ok.textContent = options.confirm || text("ok");
      ok.classList.add(options.danger ? "button--danger-solid" : "button--primary");
      dialog.addEventListener("click", function (e) {
        var button = e.target instanceof Element && e.target.closest("button[value]");
        if (!button) return;
        result = button.value === "1";
        closeDialog(dialog);
      });
      dialog.addEventListener("close", function () {
        dialog.remove();
        resolve(result);
      });
      document.body.append(dialog);
      dialog.showModal();
    });
  }

  function toast(message, kind) {
    var host = document.querySelector(".toasts");
    if (!host) {
      host = make("div", "toasts");
      host.setAttribute("aria-live", "polite");
      document.body.append(host);
    }
    var item = make("div", kind ? "toast toast--" + kind : "toast");
    item.textContent = String(message);
    host.append(item);
    while (host.children.length > 3) host.firstChild.remove();
    setTimeout(function () {
      item.classList.add("toast--leaving");
      setTimeout(function () { item.remove(); }, 200);
    }, 2600);
  }

  function readFile(file, as) {
    return new Promise(function (resolve, reject) {
      var reader = new FileReader();
      reader.onload = function () { resolve(reader.result); };
      reader.onerror = function () { reject(reader.error); };
      if (as === "text") reader.readAsText(file);
      else reader.readAsDataURL(file);
    });
  }

  var tip = null;
  var tipFor = null;
  var tipTimer = 0;

  function hideTip() {
    clearTimeout(tipTimer);
    tipFor = null;
    if (tip) tip.remove();
    tip = null;
  }

  function showTip(el) {
    var r = el.getBoundingClientRect();
    var below = r.bottom + 48 < window.innerHeight;
    tip = make("div", below ? "tooltip" : "tooltip tooltip--above");
    tip.setAttribute("role", "tooltip");
    tip.textContent = el.dataset.tip;
    if (below) tip.style.top = r.bottom + 8 + "px";
    else tip.style.bottom = window.innerHeight - r.top + 8 + "px";
    document.body.append(tip);
    var left = r.left + r.width / 2 - tip.offsetWidth / 2;
    tip.style.left = Math.min(Math.max(left, 8), window.innerWidth - tip.offsetWidth - 8) + "px";
    requestAnimationFrame(function () { if (tip) tip.classList.add("tooltip--visible"); });
  }

  document.addEventListener("pointerover", function (e) {
    var el = e.target instanceof Element ? e.target.closest("[data-tip]") : null;
    if (el === tipFor) return;
    hideTip();
    if (!el || !el.dataset.tip) return;
    tipFor = el;
    tipTimer = setTimeout(function () { showTip(el); }, 450);
  });
  document.addEventListener("pointerdown", hideTip, true);
  window.addEventListener("scroll", hideTip, true);
  window.addEventListener("blur", hideTip);
  document.addEventListener("input", function () { sync(); });
  document.addEventListener("change", function () { sync(); });

  function receive(event) {
    var m = event.data || {};
    switch (m.type) {
      case "settings":
        state.settings = m.settings || {};
        refresh();
        emit("settings", state.settings);
        break;
      case "globals":
        state.globals = m.settings || {};
        refresh();
        emit("globals", state.globals);
        break;
      case "message":
        emit("message", m.payload);
        break;
      case "theme":
        applyTheme(m.theme);
        emit("theme", state.theme);
        break;
    }
  }

  window.addEventListener("message", function (event) {
    var m = event.data;
    if (port || event.source !== window.parent || !m || m.opensd !== "connect" || !event.ports[0]) return;
    port = event.ports[0];
    port.onmessage = receive;
    var context = m.context || {};
    state.settings = context.settings || {};
    state.globals = context.globals || {};
    state.language = context.language || "en";
    applyTheme(context.theme);
    root.lang = state.language;
    root.dataset.kind = context.kind === "settings" ? "settings" : "inspector";
    function start() {
      bind();
      resolveReady(context);
    }
    if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start);
    else start();
  });

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", function () { enhance(); sync(); });
  else {
    enhance();
    sync();
  }

  window.opensd = {
    ready: ready,
    get settings() { return state.settings; },
    get globals() { return state.globals; },
    get theme() { return state.theme; },
    get language() { return state.language; },
    saveSettings: function (settings) {
      state.settings = settings || {};
      refresh();
      post({ type: "saveSettings", settings: state.settings });
    },
    saveGlobals: function (settings) {
      state.globals = settings || {};
      refresh();
      post({ type: "saveGlobals", settings: state.globals });
    },
    send: function (payload) { post({ type: "send", payload: payload === undefined ? null : payload }); },
    openSettings: function () { post({ type: "openSettings" }); },
    openUrl: function (url) { post({ type: "openUrl", url: String(url) }); },
    on: function (type, fn) { (listeners[type] = listeners[type] || []).push(fn); },
    bind: bind,
    toast: toast,
    confirm: confirmDialog,
    readFile: readFile,
  };

  var hellos = 0;
  (function hello() {
    if (port || hellos++ > 40 || window.parent === window) return;
    window.parent.postMessage({ opensd: "hello" }, "*");
    setTimeout(hello, 250);
  })();
})();
