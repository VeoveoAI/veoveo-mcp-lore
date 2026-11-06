// What people, and the agents working for them, pushed to the Lore repositories this person can
// read: one list of pushes and one place to look at a push. It only views; agents do the editing.
// The gallery of what repositories hold is the Files view, and this runs beside it in its document.
// Every name and message is a stranger's text, so it reaches the page only as text.
const veoveoViewer = (() => {
  const PAGE = 30;
  const PICTURE_TIMEOUT = 240000;
  const DRAWN_AT_ONCE = 3;
  const OPEN_AT_ONCE = 6;
  const VIEWS = {
    activity: "Activity",
    repositories: "Repositories",
    files: "Files",
    branches: "Branches",
  };
  const LINK_ICON = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/></svg>';
  const WORDS = {
    no_run: "No server",
    name_refused: "Name refused",
    prepare_failed: "Should serve, doesn't",
    pending: "Should serve, doesn't",
    links_failed: "A link failed",
    unreadable: "Couldn't read",
  };

  const h = (tag, attrs = {}, ...kids) => {
    const element = document.createElement(tag);
    for (const [name, value] of Object.entries(attrs)) {
      if (value === null || value === undefined || value === false) continue;
      if (name === "on") for (const [event, handler] of Object.entries(value)) element.addEventListener(event, handler);
      else if (name === "class") element.className = value;
      else element.setAttribute(name, value === true ? "" : value);
    }
    for (const kid of kids.flat(Infinity)) if (kid !== null && kid !== undefined && kid !== false) element.append(kid);
    return element;
  };
  const plural = (count, word) => `${count} ${word}${count === 1 ? "" : "s"}`;
  const shortBranch = (name) => (name.length > 22 ? `${name.slice(0, 12)}…${name.slice(-6)}` : name);
  const shortRevision = (revision) => revision.slice(0, 8);
  const idOf = (push) => `${push.repository}\0${push.revision}`;
  const who = (push) => push.by.email || push.by.name || "Someone";
  const titleOf = (message) => message.split("\n")[0] || "No message";
  const bodyOf = (message) => message.split("\n").slice(1).join("\n").trim();
  const extension = (path) => {
    const name = path.split("/").pop();
    const dot = name.lastIndexOf(".");
    return dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
  };
  const PICTURES = new Set(["usd", "usda", "usdc", "usdz", "png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "heic", "heif", "avif", "ico", "psd", "svg", "tga", "exr", "hdr", "mp4", "m4v", "mov", "mkv", "webm", "avi", "wmv", "flv", "mpg", "mpeg", "ogv", "3gp", "glb", "gltf", "fbx", "obj", "stl", "ply", "dae", "3ds", "off", "step", "stp", "iges", "igs", "brep", "3mf", "pdf", "doc", "docx", "odt", "xls", "xlsx", "ods", "ppt", "pptx", "odp"]);
  const showsPicture = (path) => PICTURES.has(extension(path));
  const hasText = (path) => !showsPicture(path) || extension(path) === "usda";
  const dayOf = (at) => {
    const date = new Date(at);
    const midnight = (value) => new Date(value.getFullYear(), value.getMonth(), value.getDate()).getTime();
    const days = Math.round((midnight(new Date()) - midnight(date)) / 86400000);
    if (days === 0) return "Today";
    if (days === 1) return "Yesterday";
    return date.toLocaleDateString(undefined, { month: "short", day: "numeric", year: days > 300 ? "numeric" : undefined });
  };
  const agoOf = (at) => {
    const minutes = Math.floor((Date.now() - at) / 60000);
    if (minutes < 1) return "just now";
    if (minutes < 60) return `${minutes} min ago`;
    if (minutes < 1440) return plural(Math.floor(minutes / 60), "hour") + " ago";
    const day = dayOf(at);
    return day === "Yesterday" ? "yesterday" : day;
  };
  const whenOf = (at) => new Date(at).toLocaleString(undefined, { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
  const pictureUrl = (picture) => (picture && picture.data && (picture.mediaType || "").startsWith("image/")
    ? `data:${picture.mediaType};base64,${picture.data}`
    : null);

  let toastTimer;
  const toast = (text) => {
    document.querySelector(".toast")?.remove();
    const element = h("div", { class: "toast", role: "status" }, text);
    document.body.append(element);
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => element.remove(), 3200);
  };

  // ---- what the page knows

  const state = {
    view: "activity",
    repository: null, // { id, name }
    branch: null,
    selected: null,
    filter: "all",
    words: "",
    mode: "split",
    whole: false,
    pushes: [],
    next: null,
    branches: [],
    loading: true,
    problem: null,
    closed: new Set(),
    showText: new Set(),
  };
  let gallery = null;
  let turn = 0;
  const changes = new Map(); // `${id}\0${against}` -> a promise of the tool's answer
  const summaries = new Map(); // id -> summary
  const diffs = new Map(); // key -> { answer } or a pending promise
  const pictures = new Map(); // key -> { url } once read
  const linkPushes = new Map(); // key -> { answer }

  const visible = () => {
    const words = state.words.trim().toLowerCase();
    return state.pushes.filter((push) => {
      if (state.filter === "mine" && !push.by.yours) return false;
      if (state.filter === "failed" && !push.running?.failedStage) return false;
      if (!words) return true;
      return `${push.message} ${who(push)} ${push.repositoryName} ${push.branch}`.toLowerCase().includes(words);
    });
  };
  const selectedPush = () => state.pushes.find((push) => idOf(push) === state.selected) || null;

  const load = async (more) => {
    const mine = ++turn;
    const asked = { limit: PAGE };
    if (state.repository) asked.repository = state.repository.id;
    if (state.repository && state.branch) asked.branch = state.branch;
    if (more && state.next) asked.cursor = state.next;
    if (!more) {
      state.loading = true;
      state.problem = null;
      render();
    }
    try {
      const page = await veoveoApp.tool("activity", asked, 150000);
      if (mine !== turn) return;
      const known = new Set(more ? state.pushes.map(idOf) : []);
      const added = page.pushes.filter((push) => !known.has(idOf(push)));
      state.pushes = more ? [...state.pushes, ...added] : added;
      state.next = page.next || null;
      if (state.repository) state.branches = page.branches || [];
      state.loading = false;
      if (!visible().some((push) => idOf(push) === state.selected)) state.selected = visible()[0] ? idOf(visible()[0]) : null;
      render();
      summarize(added);
    } catch (error) {
      if (mine !== turn) return;
      state.loading = false;
      state.problem = `This could not be read: ${error.message}`;
      render();
    }
  };

  // How much each push changed, and small pictures of it, come after the list.
  const summarize = async (pushes) => {
    for (let at = 0; at < pushes.length; at += 30) {
      const part = pushes.slice(at, at + 30);
      try {
        const answer = await veoveoApp.tool("summaries", { pushes: part.map((push) => ({ repository: push.repository, revision: push.revision })) }, 150000);
        part.forEach((push, index) => summaries.set(idOf(push), answer.summaries[index]));
        render();
        drawMinis(part);
      } catch {
        // A row without its counts still says who did what.
      }
    }
  };
  const drawing = [];
  let drawers = 0;
  const pictureKey = (repository, revision, path, size) => `${repository}\0${revision}\0${path}\0${size}`;
  const askPicture = (repository, revision, path, size) => {
    const key = pictureKey(repository, revision, path, size);
    if (pictures.has(key)) return;
    pictures.set(key, { url: undefined });
    drawing.push({ repository, revision, path, size, key });
    pump();
  };
  const pump = () => {
    while (drawers < DRAWN_AT_ONCE && drawing.length) {
      const job = drawing.pop();
      drawers += 1;
      veoveoApp.tool("thumbnail", { repository: job.repository, branch: `@${job.revision}`, path: job.path, size: job.size }, PICTURE_TIMEOUT)
        .then((picture) => pictures.set(job.key, { url: pictureUrl(picture) }))
        .catch(() => pictures.set(job.key, { url: null }))
        .finally(() => {
          drawers -= 1;
          render();
          pump();
        });
    }
  };
  const drawMinis = (pushes) => {
    for (const push of pushes) {
      for (const path of summaries.get(idOf(push))?.pictures || []) askPicture(push.repository, push.revision, path, "small");
    }
  };
  const changesOf = (push, against) => {
    const key = `${idOf(push)}\0${against || ""}`;
    if (!changes.has(key)) {
      const asked = { repository: push.repository, revision: push.revision };
      if (against) asked.against = against;
      const entry = { answer: undefined, problem: undefined };
      changes.set(key, entry);
      veoveoApp.tool("changes", asked, 150000)
        .then((answer) => { entry.answer = answer; })
        .catch((error) => { entry.problem = error.message; })
        .finally(render);
    }
    return changes.get(key);
  };
  const diffOf = (push, change, against) => {
    const key = `${push.repository}\0${change.path}\0${against || ""}\0${push.revision}`;
    if (!diffs.has(key)) {
      const asked = { repository: push.repository, path: change.path };
      if (change.fromPath) asked.fromPath = change.fromPath;
      if (against && change.action !== "added") asked.from = against;
      if (change.action !== "removed") asked.to = push.revision;
      const entry = { answer: undefined, problem: undefined };
      diffs.set(key, entry);
      veoveoApp.tool("file_diff", asked, 90000)
        .then((answer) => { entry.answer = answer; })
        .catch((error) => { entry.problem = error.message; })
        .finally(render);
    }
    return diffs.get(key);
  };

  // ---- diffs

  const parseDiff = (patch) => {
    const rows = [];
    let oldNo = 0;
    let newNo = 0;
    let inHunk = false;
    for (const line of patch.split("\n")) {
      const hunk = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
      if (hunk) {
        inHunk = true;
        oldNo = Number(hunk[1]);
        newNo = Number(hunk[2]);
        rows.push({ kind: "hunk", text: line });
      } else if (!inHunk) {
        continue;
      } else if (line.startsWith("+")) {
        rows.push({ kind: "add", newNo: newNo++, text: line.slice(1) });
      } else if (line.startsWith("-")) {
        rows.push({ kind: "del", oldNo: oldNo++, text: line.slice(1) });
      } else if (line.startsWith(" ")) {
        rows.push({ kind: "same", oldNo: oldNo++, newNo: newNo++, text: line.slice(1) });
      }
    }
    return rows;
  };
  const aligned = (rows) => {
    const out = [];
    for (let at = 0; at < rows.length;) {
      if (rows[at].kind === "hunk") { out.push({ hunk: rows[at].text }); at += 1; continue; }
      if (rows[at].kind === "same") { out.push({ left: rows[at], right: rows[at] }); at += 1; continue; }
      const removed = [];
      const added = [];
      while (rows[at]?.kind === "del") removed.push(rows[at++]);
      while (rows[at]?.kind === "add") added.push(rows[at++]);
      for (let index = 0; index < Math.max(removed.length, added.length); index += 1) out.push({ left: removed[index], right: added[index] });
    }
    return out;
  };
  const diffTable = (patch) => {
    const rows = parseDiff(patch);
    const number = (value) => String(value ?? "");
    if (state.mode === "split") {
      const cell = (row, kind, mid) => (row
        ? [h("td", { class: `no ${kind}${mid ? " mid" : ""}` }, number(row.oldNo ?? row.newNo)), h("td", { class: kind }, row.text)]
        : [h("td", { class: `no blank${mid ? " mid" : ""}` }), h("td", { class: "blank" })]);
      return h("table", { class: "diff" },
        h("colgroup", {}, h("col", { style: "width: 44px" }), h("col"), h("col", { style: "width: 44px" }), h("col")),
        aligned(rows).map((row) => (row.hunk
          ? h("tr", { class: "hunk" }, h("td", { colspan: 4 }, row.hunk))
          : h("tr", {},
            cell(row.left, row.left?.kind === "del" ? "del" : "", false),
            cell(row.right, row.right?.kind === "add" ? "add" : "", true)))));
    }
    return h("table", { class: "diff" },
      h("colgroup", {}, h("col", { style: "width: 44px" }), h("col", { style: "width: 44px" }), h("col")),
      rows.map((row) => (row.kind === "hunk"
        ? h("tr", { class: "hunk" }, h("td", { colspan: 3 }, row.text))
        : h("tr", { class: row.kind === "same" ? "" : row.kind },
          h("td", { class: "no" }, number(row.oldNo)),
          h("td", { class: "no" }, number(row.newNo)),
          h("td", {}, `${row.kind === "add" ? "+" : row.kind === "del" ? "-" : " "}${row.text}`)))));
  };

  // ---- going places

  const go = (changed) => {
    const before = state.repository?.id;
    const beforeBranch = state.branch;
    Object.assign(state, changed);
    if (changed.selected) { state.closed = new Set(); state.showText = new Set(); state.whole = false; }
    const moved = state.repository?.id !== before || state.branch !== beforeBranch;
    if (state.view === "files" || state.view === "repositories") {
      showGallery();
    } else if (moved || changed.reload) {
      delete state.reload;
      load(false);
      return;
    }
    render();
  };
  const showGallery = () => {
    if (!gallery) return;
    gallery(state.view === "files" && state.repository ? { repository: state.repository, folders: [] } : null);
  };
  const openFactory = (running) => {
    const query = new URLSearchParams();
    if (running.instanceRepository) query.set("repository", running.instanceRepository);
    if (running.instanceBranch) query.set("branch", running.instanceBranch);
    if (running.link) query.set("link", running.link);
    veoveoApp.openLink(`/apps/console/factory?${query}`).catch(() => toast("The factory's page could not be opened."));
  };
  const openFile = (push, path, revision) => {
    const url = `/${["view", "lore", push.repository, `@${revision}`, ...path.split("/")].map(encodeURIComponent).join("/")}`;
    veoveoApp.openLink(url.replace("%40", "@")).catch(() => toast("That file could not be opened."));
  };

  // ---- the list

  const statusTag = (push) => {
    if (push.running?.failedStage) return h("span", { class: "tag failed" }, "Build failed");
    if (push.running?.serving) return h("span", { class: "tag serving" }, "Serving");
    return null;
  };
  const summaryOf = (push) => {
    const summary = summaries.get(idOf(push));
    if (!summary || summary.unknown) return null;
    return [
      summary.links ? h("span", {}, `${plural(summary.links, "link")} moved`) : null,
      summary.files ? h("span", {}, `${plural(summary.files, "file")}${summary.more ? "+" : ""}`) : null,
    ];
  };
  const minisOf = (push) => {
    const shown = (summaries.get(idOf(push))?.pictures || [])
      .map((path) => pictures.get(pictureKey(push.repository, push.revision, path, "small"))?.url)
      .filter(Boolean);
    return shown.length ? h("span", { class: "minis" }, shown.map((url) => h("img", { src: url, alt: "" }))) : null;
  };
  const pushRow = (push) => h("button", {
    class: `push${state.branch ? " railed" : ""}${push.merged ? " merge" : ""}`,
    type: "button",
    "aria-current": idOf(push) === state.selected ? "true" : "false",
    on: { click: () => go({ selected: idOf(push) }) },
  },
  h("span", { class: "where" },
    !state.repository && h("b", {}, push.repositoryName),
    !state.branch && h("code", { title: push.branch }, shortBranch(push.branch)),
    h("span", {}, `#${push.number}`)),
  h("span", { class: "msg", title: titleOf(push.message) }, titleOf(push.message)),
  h("span", { class: "meta" }, `${who(push)} · ${agoOf(push.at)}`),
  h("span", { class: "sum" }, statusTag(push), push.merged && h("span", { class: "tag quiet-tag" }, "Merge"), summaryOf(push)),
  minisOf(push));
  const listView = () => {
    const pushes = visible();
    const groups = [];
    for (const push of pushes) {
      const day = dayOf(push.at);
      const last = groups[groups.length - 1];
      if (last && last.day === day) last.rows.push(push); else groups.push({ day, rows: [push] });
    }
    return h("div", { class: "left" },
      h("div", { class: "left-head" },
        h("input", { class: "search", id: "viewer-filter", type: "search", placeholder: "Filter by words or person", "aria-label": "Filter", value: state.words,
          on: { input: (event) => { state.words = event.target.value; render(); } } }),
        h("div", { class: "chips", role: "group", "aria-label": "Show only" },
          [["all", "All"], ["mine", "Mine"], ["failed", "Failed"]].map(([key, label]) => h("button", {
            type: "button", "aria-pressed": state.filter === key ? "true" : "false",
            on: { click: () => { state.filter = key; render(); } } }, label)))),
      h("div", { class: "list", id: "viewer-list" },
        state.loading && !state.pushes.length ? h("div", { class: "loading" }, "Reading what happened…") : null,
        state.problem ? h("div", { class: "problem-row" }, state.problem) : null,
        !state.loading && !state.problem && !pushes.length ? h("div", { class: "empty" }, "Nothing here.") : null,
        groups.map((group) => [h("div", { class: "day" }, group.day), group.rows.map(pushRow)]),
        state.next ? h("button", { class: "more", type: "button", on: { click: () => load(true) } }, "More") : null));
  };

  // ---- one push

  const look = (push, title, before, after, showing) => {
    const stage = h("img", { alt: title });
    const buttons = {};
    let side = showing;
    const revisionOf = (which) => (which === "before" ? before : after);
    const show = () => {
      const url = pictures.get(pictureKey(push.repository, revisionOf(side).revision, revisionOf(side).path, "large"))?.url;
      const small = pictures.get(pictureKey(push.repository, revisionOf(side).revision, revisionOf(side).path, "small"))?.url;
      if (url || small) stage.src = url || small;
      for (const [name, button] of Object.entries(buttons)) button.setAttribute("aria-pressed", name === side ? "true" : "false");
    };
    const flip = (which) => {
      side = which;
      askPicture(push.repository, revisionOf(side).revision, revisionOf(side).path, "large");
      show();
    };
    const dialog = h("dialog", { "aria-label": title },
      h("div", { class: "stage" }, stage),
      h("div", { class: "facts" },
        h("div", { class: "what" }, h("div", { class: "title" }, title)),
        before && after ? h("div", { class: "look-switch", role: "group", "aria-label": "Which version" },
          buttons.before = h("button", { type: "button", on: { click: () => flip("before") } }, "Before"),
          buttons.after = h("button", { type: "button", on: { click: () => flip("after") } }, "After")) : null,
        h("button", { type: "button", on: { click: () => dialog.close() } }, "Close"),
        h("button", { id: "viewer-open", class: "open", type: "button", on: { click: () => openFile(push, revisionOf(side).path, revisionOf(side).revision) } }, "Open")));
    const redraw = setInterval(show, 500);
    dialog.addEventListener("close", () => {
      clearInterval(redraw);
      dialog.remove();
    });
    dialog.addEventListener("keydown", (event) => {
      if (before && after && (event.key === "ArrowLeft" || event.key === "ArrowRight")) flip(event.key === "ArrowLeft" ? "before" : "after");
    });
    document.body.append(dialog);
    dialog.showModal();
    flip(showing);
  };
  const pathView = (path) => {
    const cut = path.lastIndexOf("/");
    return h("span", { class: "path", title: path }, h("span", { class: "folder" }, path.slice(0, cut + 1)), h("b", {}, path.slice(cut + 1)));
  };
  const toggle = (set, key) => {
    if (set.has(key)) set.delete(key); else set.add(key);
    render();
  };
  const sideOf = (push, change, label, revision, path, showing, before, after) => {
    if (!revision) return h("span", { class: "side" }, h("span", { class: "label" }, label), h("span", { class: "blank" }, label === "Before" ? "Didn't exist" : "Removed"));
    askPicture(push.repository, revision, path, "small");
    const picture = pictures.get(pictureKey(push.repository, revision, path, "small"));
    return h("button", { class: "side", type: "button", disabled: !picture?.url, on: { click: () => look(push, change.path, before, after, showing) } },
      h("span", { class: "label" }, label),
      picture?.url
        ? h("img", { src: picture.url, alt: `${label}: ${change.path}` })
        : picture?.url === null ? h("span", { class: "blank" }, "No picture") : h("span", { class: "drawing", "aria-label": "Drawing" }));
  };
  const fileChange = (push, change, against, open) => {
    const key = change.path;
    const beforePath = change.fromPath || change.path;
    const before = change.action !== "added" && against ? { revision: against, path: beforePath } : null;
    const after = change.action !== "removed" ? { revision: push.revision, path: change.path } : null;
    const body = [];
    const textOf = () => {
      const diff = diffOf(push, change, against);
      if (diff.problem) return h("div", { class: "problem-row" }, `This could not be read: ${diff.problem}`);
      if (!diff.answer) return h("div", { class: "note-row" }, "Reading…");
      if (diff.answer.binary) return h("div", { class: "note-row" }, "Binary file. No text to compare.");
      if (diff.answer.tooLarge) return h("div", { class: "note-row" }, "Too large to compare here.");
      if (!diff.answer.patch || !diff.answer.patch.includes("@@")) return h("div", { class: "note-row" }, change.fromPath ? `From ${change.fromPath}. Same content.` : "Same content.");
      return diffTable(diff.answer.patch);
    };
    if (open) {
      if (showsPicture(change.path)) {
        body.push(h("div", { class: "pair" },
          sideOf(push, change, "Before", before?.revision, before?.path, "before", before, after),
          sideOf(push, change, "After", after?.revision, after?.path, "after", before, after)));
        if (hasText(change.path)) {
          const showText = state.showText.has(key);
          body.push(h("button", { class: "text-toggle", type: "button", on: { click: () => toggle(state.showText, key) } }, `${showText ? "▾" : "▸"} Text`));
          if (showText) body.push(textOf());
        }
      } else if (change.action === "moved" || change.action === "copied") {
        body.push(textOf());
      } else {
        body.push(textOf());
      }
    }
    const answered = open && !showsPicture(change.path) ? diffOf(push, change, against).answer : null;
    return h("div", { class: "change" },
      h("button", { class: "change-head", type: "button", "aria-expanded": open ? "true" : "false", on: { click: () => toggle(state.closed, key) } },
        h("span", { class: "chev" }, open ? "▾" : "▸"),
        h("span", { class: `tag ${change.action}` }, change.action[0].toUpperCase() + change.action.slice(1)),
        pathView(change.path),
        h("span", { class: "spacer" }),
        answered?.added ? h("span", { class: "plus" }, `+${answered.added}`) : null,
        answered?.removed ? h("span", { class: "minus" }, `−${answered.removed}`) : null,
        change.action !== "removed" ? h("span", { class: "open-file", role: "button", on: { click: (event) => {
          event.stopPropagation();
          openFile(push, change.path, push.revision);
        } } }, "Open") : null),
    body);
  };
  const linkHead = (change, open, right) => [
    h("span", { class: "chev" }, open === null ? "" : open ? "▾" : "▸"),
    h("span", { class: "tag moved" }, change.action === "changed" ? "Moved" : change.action[0].toUpperCase() + change.action.slice(1)),
    h("span", { class: "link-icon" }, (() => {
      const holder = document.createElement("template");
      holder.innerHTML = LINK_ICON;
      return holder.content.firstChild;
    })()),
    h("span", { class: "path", title: change.path }, h("b", {}, change.path), change.link?.title ? h("span", { class: "folder" }, `  ${change.link.title}`) : null),
    h("span", { class: "spacer" }),
    right,
  ];
  const linkChange = (change) => {
    if (!change.link?.reached) {
      return h("div", { class: "change" }, h("div", { class: "change-head", style: "cursor: default" }, linkHead(change, null, h("span", { class: "tag quiet-tag" }, "No grant"))));
    }
    const key = `link:${change.path}`;
    const open = !state.closed.has(key);
    const asked = change.link.to && change.link.repository ? `${change.link.repository}\0${change.link.to}\0${change.link.from || ""}` : null;
    if (open && asked && !linkPushes.has(asked)) {
      const entry = { answer: undefined };
      linkPushes.set(asked, entry);
      const request = { repository: change.link.repository, to: change.link.to };
      if (change.link.from) request.from = change.link.from;
      veoveoApp.tool("link_pushes", request, 90000)
        .then((answer) => { entry.answer = answer; })
        .catch(() => { entry.answer = { pushes: [] }; })
        .finally(render);
    }
    const answer = asked ? linkPushes.get(asked)?.answer : null;
    const count = answer ? h("span", { class: "movement" }, plural(answer.pushes.length, "push").replace("pushs", "pushes") + (answer.truncated ? "+" : "")) : null;
    return h("div", { class: "change" },
      h("button", { class: "change-head", type: "button", "aria-expanded": open ? "true" : "false", on: { click: () => toggle(state.closed, key) } }, linkHead(change, open, count)),
      open && answer ? h("div", { class: "linked-list" }, answer.pushes.map((push) => h("button", {
        class: "linked", type: "button",
        on: { click: () => go({ view: "activity", repository: { id: push.repository, name: push.repositoryName }, branch: push.branch, selected: idOf(push), filter: "all", words: "", reload: true }) } },
      h("span", { class: "num" }, `#${push.number}`),
      h("span", { class: "name" }, titleOf(push.message)),
      h("span", { class: "who" }, `${who(push)} · ${agoOf(push.at)}`)))) : null);
  };
  const factoryLine = (push) => {
    const running = push.running;
    if (!running) return null;
    if (running.failedStage) {
      return h("div", { class: "fail" }, h("b", {}, `Build failed at ${running.failedStage}. `),
        running.stillServing ? `${shortRevision(running.stillServing)} is still serving.` : "Nothing is serving.",
        running.failedMessage ? h("code", {}, running.failedMessage) : null);
    }
    if (!running.serving) return null;
    const name = running.link ? `${running.server || "?"} · ${running.link}` : running.server || "?";
    return h("button", { class: "factory", type: "button", on: { click: () => openFactory(running) } },
      h("b", {}, "Serving"), "as", h("code", {}, name), "on", h("code", { title: running.on || push.branch }, shortBranch(running.on || push.branch)));
  };
  const paneView = () => {
    const push = selectedPush();
    if (!push) return h("div", { class: "pane", id: "viewer-pane" }, h("div", { class: "empty" }, state.loading ? "" : "Pick a push to look at it."));
    const whole = state.whole && push.forkedAt;
    const entry = changesOf(push, whole ? push.forkedAt : null);
    const answer = entry.answer;
    const against = answer?.against || null;
    const parent = state.pushes.find((candidate) => candidate.repository === push.repository && candidate.revision === push.parent);
    const list = answer?.changes || [];
    const sorted = [
      ...list.filter((change) => change.type === "file" && showsPicture(change.path)),
      ...list.filter((change) => change.type === "link"),
      ...list.filter((change) => change.type === "file" && !showsPicture(change.path)),
    ];
    const opened = (change, index) => index < OPEN_AT_ONCE && !state.closed.has(change.path);
    return h("div", { class: "pane", id: "viewer-pane" },
      h("div", {},
        h("h2", {}, titleOf(push.message)),
        h("div", { class: "pane-meta" },
          h("b", {}, push.repositoryName),
          h("code", { title: push.branch }, shortBranch(push.branch)),
          h("span", {}, `#${push.number}`),
          h("span", {}, who(push)),
          h("span", {}, whenOf(push.at)),
          h("code", { title: push.revision }, shortRevision(push.revision)))),
      factoryLine(push),
      bodyOf(push.message) ? h("p", { class: "pane-body" }, bodyOf(push.message)) : null,
      parent || push.merged ? h("div", { class: "pane-actions" },
        parent ? h("button", { class: "quiet small", type: "button", on: { click: () => go({ selected: idOf(parent) }) } }, `Before this: #${parent.number}`) : null,
        push.merged ? h("span", { class: "tag quiet-tag" }, `Merged ${push.merged.branch || shortRevision(push.merged.revision)}`) : null) : null,
      h("div", { class: "changes-head" },
        push.forkedAt ? h("div", { class: "switch words", role: "group", "aria-label": "Which changes" },
          h("button", { type: "button", "aria-pressed": state.whole ? "false" : "true", on: { click: () => { state.whole = false; render(); } } }, "This push"),
          h("button", { type: "button", "aria-pressed": state.whole ? "true" : "false", on: { click: () => { state.whole = true; render(); } } }, "Whole branch"))
          : h("b", {}, answer ? plural(sorted.length, "change") + (answer.truncated ? "+" : "") : "Changes"),
        h("span", { class: "spacer" }),
        h("div", { class: "switch words", role: "group", "aria-label": "How to show text" },
          h("button", { type: "button", "aria-pressed": state.mode === "split" ? "true" : "false", on: { click: () => { state.mode = "split"; render(); } } }, "Side by side"),
          h("button", { type: "button", "aria-pressed": state.mode === "inline" ? "true" : "false", on: { click: () => { state.mode = "inline"; render(); } } }, "Inline"))),
      entry.problem ? h("div", { class: "problem-row" }, `This could not be read: ${entry.problem}`) : null,
      !answer && !entry.problem ? h("div", { class: "loading" }, "Reading what changed…") : null,
      answer?.tooLarge ? h("div", { class: "note-row" }, "Too many changes to list here.") : null,
      answer && !answer.tooLarge && !sorted.length ? h("div", { class: "note-row" }, "No files changed.") : null,
      h("div", { class: "changes" }, sorted.map((change, index) => (change.type === "link"
        ? linkChange(change)
        : fileChange(push, change, against, opened(change, index))))));
  };

  // ---- branches

  const branchesView = () => h("div", { class: "branches-wrap" }, state.loading && !state.branches.length ? h("div", { class: "loading" }, "Reading the branches…") : h("table", { class: "plain" },
    h("thead", {}, h("tr", {}, ["Branch", "Latest push", "By", "When", "Factory"].map((title) => h("th", {}, title)))),
    h("tbody", {}, state.branches.map((branch) => {
      const tip = branch.latest;
      const running = tip?.running;
      const factory = running?.failedStage
        ? [h("span", { class: "tag failed" }, "Build failed"), " ", running.stillServing ? h("span", { class: "muted" }, `${shortRevision(running.stillServing)} still serving`) : null]
        : running?.serving
          ? [h("span", { class: "tag serving" }, "Serving"), " ", h("code", {}, running.link ? `${running.server || "?"} · ${running.link}` : running.server || "?")]
          : h("span", { class: "muted" }, WORDS[branch.state] || (branch.state === "served" ? "Building" : "No server"));
      return h("tr", { on: { click: () => go({ view: "activity", branch: branch.name, selected: tip ? idOf(tip) : null, reload: true }) } },
        h("td", { title: branch.name }, h("code", {}, shortBranch(branch.name))),
        h("td", {}, tip ? [h("span", { class: "muted" }, `#${tip.number} `), titleOf(tip.message)] : ""),
        h("td", {}, tip ? who(tip) : ""),
        h("td", {}, tip ? agoOf(tip.at) : ""),
        h("td", {}, factory));
    }))));

  // ---- the page

  const tabsView = () => {
    const views = state.repository ? ["activity", "files", "branches"] : ["activity", "repositories"];
    return h("div", { class: "switch words", id: "viewer-tabs", role: "group", "aria-label": "View" }, views.map((view) => h("button", {
      type: "button", "aria-pressed": state.view === view ? "true" : "false",
      on: { click: () => go({ view }) } }, VIEWS[view])));
  };
  const branchSelect = () => {
    if (!state.repository || state.view === "branches") return null;
    const names = state.branches.map((branch) => branch.name);
    if (!names.length) return null;
    const files = state.view === "files";
    const chosen = state.branch || (files ? (names.includes("main") ? "main" : names[0]) : "");
    return h("select", { class: "branch", id: "viewer-branch", "aria-label": "Branch",
      on: { change: (event) => go({ branch: event.target.value || null, selected: null, reload: !files }) } },
    files ? null : h("option", { value: "" }, "All branches"),
    names.map((name) => h("option", { value: name, title: name, selected: name === chosen }, shortBranch(name))));
  };
  const crumbsView = () => {
    const crumbs = h("nav", { class: "crumbs", "aria-label": "Where you are" });
    if (!state.repository) {
      crumbs.append(h("span", { class: "here" }, "Repo"));
    } else {
      crumbs.append(
        h("button", { type: "button", on: { click: () => go({ repository: null, branch: null, view: "activity", selected: null, branches: [] }) } }, "Repo"),
        h("span", { class: "sep" }, "/"),
        h("span", { class: "here" }, state.repository.name));
    }
    return crumbs;
  };
  const refreshButton = () => h("button", { class: "quiet small", type: "button", on: { click: () => load(false) } }, "Refresh");

  const render = () => {
    const holder = document.getElementById("viewer");
    const main = document.querySelector("main");
    if (!holder || !main) return;
    const inGallery = state.view === "files" || state.view === "repositories";
    holder.hidden = inGallery;
    main.hidden = !inGallery;
    const galleryBar = main.querySelector(".bar");
    galleryBar.querySelectorAll("#viewer-tabs, #viewer-branch").forEach((element) => element.remove());
    if (inGallery) {
      const select = branchSelect();
      if (select) galleryBar.append(select);
      galleryBar.append(tabsView());
      holder.replaceChildren();
      return;
    }
    const listScroll = document.getElementById("viewer-list")?.scrollTop || 0;
    const oldPane = document.getElementById("viewer-pane");
    const paneScroll = oldPane?.scrollTop || 0;
    const shownBefore = oldPane?.dataset.shown;
    const focused = document.activeElement?.id;
    const bar = h("div", { class: "bar" }, crumbsView(), branchSelect(), state.view === "activity" ? refreshButton() : null, tabsView());
    if (state.view === "branches") {
      holder.replaceChildren(bar, branchesView());
      return;
    }
    holder.replaceChildren(bar, h("div", { class: "split" }, listView(), paneView()));
    const list = document.getElementById("viewer-list");
    if (list) list.scrollTop = listScroll;
    const pane = document.getElementById("viewer-pane");
    if (pane) {
      pane.dataset.shown = state.selected || "";
      pane.scrollTop = shownBefore === (state.selected || "") ? paneScroll : 0;
    }
    if (shownBefore !== (state.selected || "")) document.querySelector('.push[aria-current="true"]')?.scrollIntoView({ block: "nearest" });
    if (focused === "viewer-filter") {
      const input = document.getElementById("viewer-filter");
      input.focus();
      input.setSelectionRange(input.value.length, input.value.length);
    }
  };

  document.addEventListener("keydown", (event) => {
    if (state.view !== "activity" || document.querySelector("dialog[open]")) return;
    if (["INPUT", "SELECT"].includes(event.target.tagName)) return;
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    const pushes = visible();
    const here = pushes.findIndex((push) => idOf(push) === state.selected);
    const next = pushes[here + (event.key === "ArrowDown" ? 1 : -1)];
    if (next) {
      event.preventDefault();
      go({ selected: idOf(next) });
    }
  });

  // An address the page was opened at, as the host hands it over: `repository`, `branch`, `revision`.
  window.addEventListener("message", async (event) => {
    if (event.source !== window.parent) return;
    const message = event.data;
    if (!message || message.method !== "ui/notifications/tool-input") return;
    const asked = message.params?.arguments || {};
    if (!asked.repository) return;
    state.repository = { id: asked.repository, name: asked.repositoryName || asked.repository };
    state.branch = asked.branch || null;
    state.view = "activity";
    state.selected = asked.revision ? `${asked.repository}\0${asked.revision}` : null;
    await load(false);
    const named = state.pushes[0]?.repositoryName;
    if (named) {
      state.repository = { id: asked.repository, name: named };
      render();
    }
  });

  // The gallery connects to the host, and the viewer starts then.
  const connect = veoveoApp.connect;
  veoveoApp.connect = async (...args) => {
    await connect(...args);
    load(false);
  };
  const style = document.createElement("style");
  style.textContent = VIEWER_STYLE;
  document.head.append(style);
  document.body.prepend(h("section", { id: "viewer" }));
  document.querySelector("main").hidden = true;

  return {
    /** Which branch the gallery reads. */
    filesBranch: () => state.branch || "main",
    /** The gallery says how to move it. */
    moves: (go) => { gallery = go; },
    /** The gallery says where it went. */
    inRepository: (repository) => {
      const moved = (repository?.id || null) !== (state.repository?.id || null);
      if (moved) {
        state.repository = repository;
        state.branch = null;
        state.branches = [];
        state.pushes = [];
        state.next = null;
        state.selected = null;
        if (state.view === "repositories" && repository) state.view = "files";
        if (state.view === "files" && !repository) state.view = "repositories";
        if (repository) load(false);
      }
      render();
    },
  };
})();
