/** @typedef {"navigate" | "preload"} LoadCommand */

/** @typedef {[LoadCommand, string, string | null, number, boolean]} LoadRequest */

/** @typedef {Array<[string, string]>} ThemeVariables */

/**
 * @typedef {Object} DioxusEval
 * @property {() => Promise<LoadRequest>} recv
 */

/**
 * @typedef {Object} ChapterSlot
 * @property {number} index
 * @property {HTMLIFrameElement} frame
 * @property {string | null} blobUrl
 * @property {string | null} url
 * @property {number | null} spineIndex
 * @property {number} generation
 * @property {boolean} ready
 * @property {number} pageCount
 * @property {number} page
 */

/**
 * @typedef {Object} PendingNavigation
 * @property {number} slot
 * @property {number} spineIndex
 * @property {-1 | 0 | 1} direction
 * @property {boolean} seekLast
 * @property {boolean} awaitScroll
 * @property {boolean} finishing
 */

/**
 * @typedef {
 *   | { kind: "ook-pages", count: number }
 *   | { kind: "ook-ready" }
 *   | { kind: "ook-scroll", page: number }
 *   | { kind: "ook-warn", message: string }
 *   | { kind: "ook-drag", dx: number }
 *   | { kind: "ook-drag-cancel" }
 *   | { kind: "ook-link", raw: string }
 *   | { kind: "ook-position", selector: string }
 *   | { kind: "ook-reflow", page: number }
 *   | { kind: "ook-key", key: string }
 *   | { kind: "ook-swipe", dx: number, dy: number, selected: boolean }
 *   | { kind: "ook-tap" }
 *   | { kind: "ook-pointerdown" }
 * } FrameMessage
 */

/**
 * @typedef {Object} ReaderController
 * @property {ChapterSlot[]} slots
 * @property {number} active
 * @property {PendingNavigation | null} pending
 * @property {number} dragX
 * @property {(message: string) => void} send
 * @property {(source: MessageEventSource | null) => ChapterSlot | undefined} slotForSource
 * @property {(spine: number) => ChapterSlot | undefined} slotForSpine
 * @property {(spine: number, page: number, animate: boolean) => void} setPage
 * @property {(vars: ThemeVariables) => void} setTheme
 * @property {(index: number) => void} setActive
 * @property {(slot: ChapterSlot, targetUrl: string, targetSpine: number, targetFragment: string | null) => Promise<boolean>} fetchInto
 * @property {(command: LoadCommand, targetUrl: string, targetFragment: string | null, targetSpine: number, targetLast: boolean) => Promise<void>} load
 * @property {(source: MessageEventSource | null, data: FrameMessage | null | undefined) => void} handleMessage
 * @property {(dx: number) => void} drag
 * @property {() => void} cancelDrag
 * @property {(accepted: boolean) => void} resolveGesture
 * @property {() => void} finishNavigation
 * @property {() => void} destroy
 */

/**
 * @typedef {Window & {
 *   __ookReader?: ReaderController | null,
 *   __ookReaderSend?: ((message: string) => void) | null
 * }} ReaderHostWindow
 */

const hostWindow = /** @type {ReaderHostWindow} */ (window);

/** @type {DioxusEval} */
// @ts-expect-error Dioxus injects this binding into document::eval scripts.
const dioxusEval = dioxus;
const [kind, url, fragment, spineIndex, seekLast] = await dioxusEval.recv();

if (!hostWindow.__ookReader) {
  // The two reusable iframe slots form a double buffer: one is visible while
  // the other can preload a chapter or animate into view.
  const frameNodes = /** @type {NodeListOf<HTMLIFrameElement>} */ (
    document.querySelectorAll("iframe.reader-frame")
  );
  const frames = Array.from(frameNodes);
  /** @type {ChapterSlot[]} */
  const slots = frames.map(function (frame, index) {
    return {
      index,
      frame,
      blobUrl: null,
      // These values describe the chapter currently occupying this slot.
      // They are replaced whenever fetchInto reuses the iframe.
      url: null,
      spineIndex: null,
      // A request version that prevents an older fetch from overwriting a
      // slot after a newer fetch has started.
      generation: 0,
      ready: false,
      pageCount: 0,
      page: 0,
    };
  });

  /** @type {ReaderController} */
  const reader = {
    slots,
    // Index of the slot currently visible to the reader.
    active: 0,
    // The navigation transaction currently moving toward a new active slot.
    // This is null while navigation is settled; it is not a chapter or slot.
    pending: null,
    // Horizontal displacement shared by an in-progress swipe and transition.
    dragX: 0,

    send(message) {
      hostWindow.__ookReaderSend?.(message);
    },

    slotForSource(source) {
      return this.slots.find((slot) => slot.frame.contentWindow === source);
    },

    slotForSpine(spine) {
      return this.slots.find((slot) => slot.spineIndex === spine);
    },

    setPage(spine, page, animate) {
      const slot = this.slotForSpine(spine);
      if (!slot) return;
      slot.page = page;
      slot.frame.contentWindow?.postMessage(
        { kind: "ook-set-page", page, animate },
        "*",
      );
    },

    setTheme(vars) {
      for (const slot of this.slots) {
        slot.frame.contentWindow?.postMessage(
          { kind: "ook-set-theme", vars },
          "*",
        );
      }
    },

    setActive(index) {
      this.active = index;
      for (const slot of this.slots) {
        const active = slot.index === index;
        slot.frame.classList.toggle("reader-frame--active", active);
        slot.frame.classList.toggle("reader-frame--standby", !active);
        slot.frame.setAttribute("aria-hidden", active ? "false" : "true");
        slot.frame.inert = !active;
        slot.frame.style.opacity = active ? "1" : "0";
        slot.frame.style.pointerEvents = active ? "auto" : "none";
        if (active) slot.frame.style.transform = "translate3d(0, 0, 0)";
      }
    },

    async fetchInto(slot, targetUrl, targetSpine, targetFragment) {
      const generation = ++slot.generation;
      slot.url = targetUrl;
      slot.spineIndex = targetSpine;
      slot.ready = false;
      slot.pageCount = 0;
      slot.page = 0;
      slot.frame.dataset.chapterUrl = targetUrl;

      const response = await fetch(targetUrl);
      if (!response.ok) {
        this.send(`warn:${response.status} loading ${targetUrl}`);
        return false;
      }
      const blob = await response.blob();
      if (slot.generation !== generation || slot.url !== targetUrl) return false;

      const next = URL.createObjectURL(blob);
      if (slot.blobUrl) URL.revokeObjectURL(slot.blobUrl);
      slot.blobUrl = next;
      slot.frame.src = targetFragment
        ? `${next}#${encodeURIComponent(targetFragment)}`
        : next;
      return true;
    },

    async load(command, targetUrl, targetFragment, targetSpine, targetLast) {
      const active = this.slots[this.active];
      // Preloading means the destination may already occupy either slot.
      const existing = this.slots.find((slot) => slot.url === targetUrl);

      if (
        command === "navigate" &&
        this.pending?.spineIndex === targetSpine
      ) {
        return;
      }

      if (command === "navigate" && active.url === targetUrl) {
        if (targetFragment) {
          const win = active.frame.contentWindow;
          win.location.hash = "";
          win.location.hash = encodeURIComponent(targetFragment);
        }
        return;
      }

      if (command === "preload") {
        // Preloading fills the standby slot without starting a navigation
        // transaction or changing which slot is active.
        if (existing || this.pending) return;
        const target = this.slots[1 - this.active];
        target.frame.style.opacity = "0";
        target.frame.style.pointerEvents = "none";
        await this.fetchInto(target, targetUrl, targetSpine, null);
        return;
      }

      // `target` is the destination slot while the chapter is being prepared.
      // Initial load uses the active slot; later loads use the other slot.
      const target = existing || (active.url ? this.slots[1 - this.active] : active);
      const activeSpine = /** @type {number} */ (active.spineIndex);
      const direction = /** @type {-1 | 0 | 1} */ (
        active.url
          ? Math.sign(targetSpine - activeSpine) || 1
          : 0
      );
      // `pending` records what must be true before target can become active.
      // Fragment navigation adds a scroll result to the normal ready gate.
      this.pending = {
        slot: target.index,
        spineIndex: targetSpine,
        direction,
        seekLast: targetLast,
        awaitScroll: Boolean(targetFragment),
        finishing: false,
      };

      if (!existing) {
        target.frame.style.opacity = "0";
        target.frame.style.pointerEvents = "none";
        const loaded = await this.fetchInto(
          target,
          targetUrl,
          targetSpine,
          targetFragment,
        );
        if (!loaded) return;
      } else if (targetFragment) {
        const win = target.frame.contentWindow;
        win.location.hash = "";
        win.location.hash = encodeURIComponent(targetFragment);
      }

      if (target.ready) this.finishNavigation();
    },

    handleMessage(source, data) {
      const slot = this.slotForSource(source);
      if (!slot || !data || !data.kind) return;
      // Active messages describe the chapter on screen; target messages
      // describe the destination of the current navigation transaction.
      const isActive = slot.index === this.active;
      const pending = this.pending;
      const isTarget = pending?.slot === slot.index;

      if (data.kind === "ook-pages") {
        slot.pageCount = data.count;
        if (isTarget || (isActive && !this.pending)) {
          this.send(`pages:${data.count}`);
        }
        if (isTarget && pending?.seekLast) {
          this.setPage(
            /** @type {number} */ (slot.spineIndex),
            Math.max(0, data.count - 1),
            false,
          );
        }
        return;
      }
      if (data.kind === "ook-ready") {
        slot.ready = true;
        if (isTarget) this.finishNavigation();
        return;
      }
      if (data.kind === "ook-scroll" && isTarget && pending) {
        pending.awaitScroll = false;
        this.send(`scroll:${data.page}`);
        this.finishNavigation();
        return;
      }
      if (data.kind === "ook-warn" && (isActive || isTarget)) {
        this.send(`warn:${data.message}`);
        return;
      }
      if (data.kind === "ook-drag" && isActive && !this.pending) {
        this.drag(data.dx);
        return;
      }
      if (data.kind === "ook-drag-cancel" && isActive) {
        this.cancelDrag();
        return;
      }
      if (!isActive || this.pending) return;

      const payloads = {
        "ook-link": () => `link:${data.raw}`,
        "ook-scroll": () => `scroll:${data.page}`,
        "ook-position": () => `position:${data.selector}`,
        "ook-reflow": () => `reflow:${data.page}`,
        "ook-key": () => `key:${data.key}`,
        "ook-swipe": () => `swipe:${data.dx},${data.dy},${data.selected}`,
        "ook-tap": () => "tap:",
      };
      const payload = payloads[data.kind]?.();
      if (payload) this.send(payload);
      if (data.kind === "ook-pointerdown") {
        slot.frame.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
      }
    },

    drag(dx) {
      const active = this.slots[this.active];
      const direction = dx < 0 ? 1 : -1;
      // `standby` only means the non-active slot. It is revealed during a drag
      // only when its preloaded chapter lies in the requested direction.
      const standby = this.slots[1 - this.active];
      const activeSpine = /** @type {number} */ (active.spineIndex);
      const standbySpine = /** @type {number} */ (standby.spineIndex);
      this.dragX = dx;
      active.frame.style.transform = `translate3d(${dx}px, 0, 0)`;
      if (
        standby.ready &&
        Math.sign(standbySpine - activeSpine) === direction
      ) {
        standby.frame.style.opacity = "1";
        standby.frame.style.transform = `translate3d(calc(${direction * 100}% + ${dx}px), 0, 0)`;
      }
    },

    cancelDrag() {
      const active = this.slots[this.active];
      const standby = this.slots[1 - this.active];
      const direction = this.dragX < 0 ? 1 : -1;
      active.frame.classList.add("reader-frame--settling");
      standby.frame.classList.add("reader-frame--settling");
      active.frame.style.transform = "translate3d(0, 0, 0)";
      standby.frame.style.transform = `translate3d(${direction * 100}%, 0, 0)`;
      window.setTimeout(() => {
        active.frame.classList.remove("reader-frame--settling");
        standby.frame.classList.remove("reader-frame--settling");
        standby.frame.style.opacity = "0";
      }, 260);
      this.dragX = 0;
    },

    resolveGesture(accepted) {
      if (accepted) return;
      this.cancelDrag();
      this.slots[this.active].frame.contentWindow?.postMessage(
        { kind: "ook-cancel-swipe" },
        "*",
      );
    },

    finishNavigation() {
      const pending = this.pending;
      if (!pending || pending.finishing) return;
      // The slot called `target` during loading becomes `incoming` once it is
      // ready to replace the visible slot.
      const incoming = this.slots[pending.slot];
      if (!incoming.ready || pending.awaitScroll) return;
      pending.finishing = true;

      if (incoming.pageCount) {
        this.send(`pages:${incoming.pageCount}`);
        const page = pending.seekLast ? incoming.pageCount - 1 : incoming.page;
        this.setPage(
          /** @type {number} */ (incoming.spineIndex),
          Math.max(0, page),
          false,
        );
      }

      if (pending.slot === this.active || pending.direction === 0) {
        this.setActive(pending.slot);
        this.pending = null;
        this.send("ready:");
        return;
      }

      // During the transition, incoming moves on screen and the old active
      // slot becomes outgoing. setActive commits the role change afterward.
      const outgoing = this.slots[this.active];
      const direction = pending.direction;
      const start = this.dragX;
      incoming.frame.style.opacity = "1";
      incoming.frame.style.transform = `translate3d(calc(${direction * 100}% + ${start}px), 0, 0)`;
      outgoing.frame.style.transform = `translate3d(${start}px, 0, 0)`;

      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          incoming.frame.classList.add("reader-frame--settling");
          outgoing.frame.classList.add("reader-frame--settling");
          incoming.frame.style.transform = "translate3d(0, 0, 0)";
          outgoing.frame.style.transform = `translate3d(${-direction * 100}%, 0, 0)`;

          let finished = false;
          const complete = () => {
            if (finished) return;
            finished = true;
            incoming.frame.classList.remove("reader-frame--settling");
            outgoing.frame.classList.remove("reader-frame--settling");
            this.setActive(incoming.index);
            outgoing.frame.style.opacity = "0";
            this.dragX = 0;
            this.pending = null;
            this.send("ready:");
            incoming.frame.contentWindow?.postMessage(
              { kind: "ook-set-page", page: incoming.page, animate: false },
              "*",
            );
          };
          incoming.frame.addEventListener("transitionend", complete, { once: true });
          window.setTimeout(complete, 300);
        });
      });
    },

    destroy() {
      for (const slot of this.slots) {
        if (slot.blobUrl) URL.revokeObjectURL(slot.blobUrl);
        slot.blobUrl = null;
      }
      hostWindow.__ookReader = null;
    },
  };

  reader.setActive(0);
  hostWindow.__ookReader = reader;
}

await hostWindow.__ookReader.load(kind, url, fragment, spineIndex, seekLast);
