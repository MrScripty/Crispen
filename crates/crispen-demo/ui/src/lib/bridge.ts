/**
 * IPC bridge for Bevy <-> Svelte communication.
 *
 * Supports two modes:
 * - CEF (default): Uses console.log IPC injected by Rust via __CRISPEN_IPC__
 * - WebSocket (legacy fallback): Connects to ws://localhost:{port}
 */

import type { BevyToUi, GradingParams, LayoutRegion, UiToBevy } from './types';

declare global {
  interface Window {
    __CRISPEN_IPC__?: {
      postMessage: (msg: string) => void;
    };
    __CRISPEN_RECEIVE__?: (msg: string) => void;
    __CRISPEN_WS_PORT__?: number;
    ipc?: {
      postMessage: (msg: string) => void;
    };
  }
}

type MessageHandler = (msg: BevyToUi) => void;

function getNativeIpc(): { postMessage: (msg: string) => void } | null {
  if (window.__CRISPEN_IPC__) return window.__CRISPEN_IPC__;
  if (window.ipc) return window.ipc;
  return null;
}

/** True when running inside CEF (native IPC available or will be injected). */
function isCefMode(): boolean {
  return !window.__CRISPEN_WS_PORT__;
}

class CrispenBridge {
  private handlers: Set<MessageHandler> = new Set();
  private layoutDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  // WebSocket fallback fields
  private ws: WebSocket | null = null;
  private pendingMessages: string[] = [];
  private readonly useCef: boolean;

  // CEF pending message queue — buffered until IPC bridge is injected.
  private pendingCefMessages: string[] = [];
  private cefBridgeReady = false;

  constructor() {
    this.useCef = isCefMode();

    if (this.useCef) {
      // CEF mode: set up the receiver that Rust calls via eval
      const ipc = getNativeIpc();
      if (!window.__CRISPEN_IPC__ && ipc) {
        window.__CRISPEN_IPC__ = ipc;
        this.cefBridgeReady = true;
      }

      window.__CRISPEN_RECEIVE__ = (msgJson: string) => {
        try {
          const msg: BevyToUi = JSON.parse(msgJson);
          this.handlers.forEach((handler) => handler(msg));
        } catch (e) {
          console.error('Failed to parse IPC message:', e);
        }
      };

      // Poll until the native IPC bridge is injected, then flush any
      // messages that were queued before the bridge was ready.
      const waitForBridge = () => {
        if (getNativeIpc()) {
          this.cefBridgeReady = true;
          this.flushCefPending();
          this.send({ type: 'RequestState' });
        } else {
          requestAnimationFrame(waitForBridge);
        }
      };
      requestAnimationFrame(waitForBridge);
    } else {
      // WebSocket fallback
      this.connectWebSocket();
    }
  }

  private connectWebSocket(): void {
    const port = window.__CRISPEN_WS_PORT__ ?? 9400;
    const url = `ws://127.0.0.1:${port}`;
    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      for (const msg of this.pendingMessages) {
        this.ws?.send(msg);
      }
      this.pendingMessages = [];
      this.send({ type: 'RequestState' });
    };

    this.ws.onmessage = (event: MessageEvent) => {
      try {
        const msg: BevyToUi = JSON.parse(event.data as string);
        this.handlers.forEach((handler) => handler(msg));
      } catch (e) {
        console.error('Failed to parse WS message:', e);
      }
    };

    this.ws.onclose = () => {
      setTimeout(() => this.connectWebSocket(), 1000);
    };

    this.ws.onerror = () => {
      // onclose will fire after this, triggering reconnect
    };
  }

  /** Subscribe to messages from Bevy. Returns an unsubscribe function. */
  subscribe(handler: MessageHandler): () => void {
    this.handlers.add(handler);
    return () => {
      this.handlers.delete(handler);
    };
  }

  /** Send a message to Bevy. */
  send(msg: UiToBevy): void {
    const json = JSON.stringify(msg);

    if (this.useCef) {
      if (this.cefBridgeReady) {
        const ipc = getNativeIpc();
        if (ipc) {
          ipc.postMessage(json);
        }
      } else {
        this.pendingCefMessages.push(json);
      }
    } else {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.ws.send(json);
      } else {
        this.pendingMessages.push(json);
      }
    }
  }

  /** Flush queued CEF messages once the IPC bridge is available. */
  private flushCefPending(): void {
    const ipc = getNativeIpc();
    if (!ipc) return;
    for (const json of this.pendingCefMessages) {
      ipc.postMessage(json);
    }
    this.pendingCefMessages = [];
  }

  /** Mark the UI as dirty (triggers CEF framebuffer recapture). */
  markDirty(): void {
    this.send({ type: 'UiDirty' });
  }

  /** Send layout update with debouncing (~60fps). */
  updateLayout(regions: LayoutRegion[]): void {
    if (this.layoutDebounceTimer) {
      clearTimeout(this.layoutDebounceTimer);
    }
    this.layoutDebounceTimer = setTimeout(() => {
      this.send({ type: 'LayoutUpdate', data: { regions } });
      this.layoutDebounceTimer = null;
    }, 16);
  }

  /** Persist the current dockview layout. */
  saveLayout(layoutJson: string): void {
    this.send({ type: 'SaveLayout', data: { layout_json: layoutJson } });
  }

  // -- Convenience methods matching UiToBevy variants --

  setParams(params: GradingParams): void {
    this.send({ type: 'SetParams', data: { params } });
  }

  autoBalance(): void {
    this.send({ type: 'AutoBalance' });
  }

  resetGrade(): void {
    this.send({ type: 'ResetGrade' });
  }

  loadImage(path: string): void {
    this.send({ type: 'LoadImage', data: { path } });
  }

  loadLut(path: string, slot: string): void {
    this.send({ type: 'LoadLut', data: { path, slot } });
  }

  exportLut(path: string, size: number): void {
    this.send({ type: 'ExportLut', data: { path, size } });
  }

  toggleScope(scopeType: string, visible: boolean): void {
    this.send({ type: 'ToggleScope', data: { scope_type: scopeType, visible } });
  }
}

export const bridge = new CrispenBridge();

/** Set up automatic dirty marking on DOM mutations. */
export function setupAutoMarkDirty(): void {
  const observer = new MutationObserver(() => {
    bridge.markDirty();
  });

  observer.observe(document.body, {
    childList: true,
    subtree: true,
    attributes: true,
    characterData: true,
  });

  window.addEventListener('resize', () => {
    bridge.markDirty();
  });
}
