export type WsEvent =
	| { type: 'start'; session_id: string; cwd: string }
	| { type: 'event'; data: unknown }
	| { type: 'text_delta'; text: string }
	| { type: 'done'; exit_code: number; duration_ms: number; session_id: string | null }
	| { type: 'error'; message: string };

export type WsHandlers = {
	onStart?: (e: Extract<WsEvent, { type: 'start' }>) => void;
	onDelta?: (text: string) => void;
	onEvent?: (data: unknown) => void;
	onDone?: (e: Extract<WsEvent, { type: 'done' }>) => void;
	onError?: (msg: string) => void;
	onOpen?: () => void;
	onClose?: () => void;
};

export class PromptClient {
	private ws: WebSocket | null = null;
	private handlers: WsHandlers = {};
	private url: string;
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

	constructor(path = '/ws') {
		const proto = location.protocol === 'https:' ? 'wss' : 'ws';
		this.url = `${proto}://${location.host}${path}`;
	}

	setHandlers(h: WsHandlers) {
		this.handlers = { ...this.handlers, ...h };
	}

	connect() {
		if (this.ws && (this.ws.readyState === 0 || this.ws.readyState === 1)) return;
		this.ws = new WebSocket(this.url);
		this.ws.onopen = () => {
			console.log('[ws] open');
			this.handlers.onOpen?.();
		};
		this.ws.onclose = () => {
			console.log('[ws] close');
			this.handlers.onClose?.();
			if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
			this.reconnectTimer = setTimeout(() => this.connect(), 1500);
		};
		this.ws.onerror = (e) => {
			console.error('[ws] error', e);
		};
		this.ws.onmessage = (m) => {
			try {
				const ev = JSON.parse(m.data) as WsEvent;
				if (ev.type === 'start') this.handlers.onStart?.(ev);
				else if (ev.type === 'text_delta') this.handlers.onDelta?.(ev.text);
				else if (ev.type === 'event') this.handlers.onEvent?.(ev.data);
				else if (ev.type === 'done') this.handlers.onDone?.(ev);
				else if (ev.type === 'error') this.handlers.onError?.(ev.message);
			} catch (e) {
				console.error('[ws] parse failed', e, m.data);
			}
		};
	}

	send(prompt: string, cwd: string | null, sessionId: string | null) {
		if (!this.ws || this.ws.readyState !== 1) {
			console.warn('[ws] not open, dropping send');
			return false;
		}
		this.ws.send(JSON.stringify({ prompt, cwd, session_id: sessionId }));
		return true;
	}

	close() {
		if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
		this.ws?.close();
		this.ws = null;
	}
}
