/**
 * WebSocket bridge for Bevy <-> Svelte IPC.
 *
 * Connects to ws://localhost:{port} where Bevy's ws_bridge.rs is listening.
 * Follows the same subscribe/send pattern as Pentimento's BevyBridge.
 */

import type { BevyToUi, GradingParams, UiToBevy } from './types';

declare global {
  interface Window {
    __CRISPEN_WS_PORT__?: number;
  }
}

type MessageHandler = (msg: BevyToUi) => void;

const DEFAULT_WS_PORT = 9400;
const RECONNECT_DELAY_MS = 1000;

class CrispenBridge {
  private handlers: Set<MessageHandler> = new Set();
  private ws: WebSocket | null = null;
  private pendingMessages: string[] = [];
  private readonly port: number;

  constructor() {
    this.port = window.__CRISPEN_WS_PORT__ ?? DEFAULT_WS_PORT;
    this.connect();
  }

  private connect(): void {
    const url = `ws://127.0.0.1:${this.port}`;

    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      for (const msg of this.pendingMessages) {
        this.ws?.send(msg);
      }
      this.pendingMessages = [];
    };

    this.ws.onmessage = (event: MessageEvent) => {
      try {
        const msg: BevyToUi = JSON.parse(event.data as string);
        this.handlers.forEach((handler) => handler(msg));
      } catch (e) {
        console.error('Failed to parse IPC message:', e);
      }
    };

    this.ws.onclose = () => {
      setTimeout(() => this.connect(), RECONNECT_DELAY_MS);
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

  /** Send a message to Bevy, queuing if not yet connected. */
  send(msg: UiToBevy): void {
    const json = JSON.stringify(msg);
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(json);
    } else {
      this.pendingMessages.push(json);
    }
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
