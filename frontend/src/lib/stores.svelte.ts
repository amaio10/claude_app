export type Message = {
	id: string;
	role: 'user' | 'assistant';
	text: string;
	streaming?: boolean;
	ts: number;
};

export type Chat = {
	id: string;
	label: string;
	cwd: string;
	messages: Message[];
	claudeSessionId: string | null;
	ptyKey: string;
	createdAt: number;
	updatedAt: number;
};

const STORAGE_KEY = 'claude_app.chats.v1';

type Persisted = {
	chats: Chat[];
	activeId: string | null;
};

function load(): Persisted {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return { chats: [], activeId: null };
		const p = JSON.parse(raw) as Persisted;
		for (const c of p.chats) {
			for (const m of c.messages) m.streaming = false;
		}
		return p;
	} catch {
		return { chats: [], activeId: null };
	}
}

function persist(p: Persisted) {
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(p));
	} catch (e) {
		console.warn('[chats] persist failed', e);
	}
}

function labelFromCwd(cwd: string): string {
	const parts = cwd.split('/').filter(Boolean);
	return parts[parts.length - 1] || '/';
}

function createChatsState() {
	const initial = load();
	let chats = $state<Chat[]>(initial.chats);
	let activeId = $state<string | null>(initial.activeId);
	let connected = $state(false);
	let inflight = $state(false);

	$effect.root(() => {
		$effect(() => {
			persist({ chats: $state.snapshot(chats), activeId });
		});
	});

	function findActive(): Chat | null {
		return chats.find((c) => c.id === activeId) ?? null;
	}

	function create(cwd: string, label?: string): Chat {
		const id = crypto.randomUUID();
		const ptyKey = 'pty_' + id;
		const chat: Chat = {
			id,
			label: label ?? labelFromCwd(cwd),
			cwd,
			messages: [],
			claudeSessionId: null,
			ptyKey,
			createdAt: Date.now(),
			updatedAt: Date.now()
		};
		chats.push(chat);
		activeId = id;
		console.log('[chats] created', id, 'cwd=', cwd);
		return chat;
	}

	async function remove(id: string) {
		const idx = chats.findIndex((c) => c.id === id);
		if (idx < 0) return;
		const chat = chats[idx];
		try {
			await fetch(`/api/pty/${encodeURIComponent(chat.ptyKey)}`, { method: 'DELETE' });
		} catch (e) {
			console.warn('[chats] kill pty failed', e);
		}
		chats.splice(idx, 1);
		if (activeId === id) {
			activeId = chats[0]?.id ?? null;
		}
		console.log('[chats] removed', id);
	}

	function setActive(id: string) {
		if (chats.some((c) => c.id === id)) {
			activeId = id;
			console.log('[chats] active=', id);
		}
	}

	function rename(id: string, label: string) {
		const c = chats.find((x) => x.id === id);
		if (c) {
			c.label = label;
			c.updatedAt = Date.now();
		}
	}

	function setCwd(cwd: string) {
		const c = findActive();
		if (!c) return;
		c.cwd = cwd;
		if (!c.label || c.label === labelFromCwd(c.cwd)) {
			c.label = labelFromCwd(cwd);
		}
		c.updatedAt = Date.now();
	}

	function pushUser(text: string) {
		const c = findActive();
		if (!c) return;
		c.messages.push({
			id: crypto.randomUUID(),
			role: 'user',
			text,
			ts: Date.now()
		});
		c.updatedAt = Date.now();
	}

	function startAssistant() {
		const c = findActive();
		if (!c) return;
		c.messages.push({
			id: crypto.randomUUID(),
			role: 'assistant',
			text: '',
			streaming: true,
			ts: Date.now()
		});
	}

	function appendAssistant(delta: string) {
		const c = findActive();
		if (!c) return;
		const last = c.messages[c.messages.length - 1];
		if (last?.role === 'assistant') {
			last.text += delta;
		} else {
			startAssistant();
			c.messages[c.messages.length - 1].text = delta;
		}
		c.updatedAt = Date.now();
	}

	function finishAssistant() {
		const c = findActive();
		if (!c) return;
		const last = c.messages[c.messages.length - 1];
		if (last?.role === 'assistant') last.streaming = false;
	}

	function setClaudeSessionId(sid: string) {
		const c = findActive();
		if (c) c.claudeSessionId = sid;
	}

	function clearActive() {
		const c = findActive();
		if (c) {
			c.messages = [];
			c.claudeSessionId = null;
			c.updatedAt = Date.now();
		}
	}

	return {
		get chats() {
			return chats;
		},
		get active() {
			return findActive();
		},
		get activeId() {
			return activeId;
		},
		get connected() {
			return connected;
		},
		set connected(v: boolean) {
			connected = v;
		},
		get inflight() {
			return inflight;
		},
		set inflight(v: boolean) {
			inflight = v;
		},
		create,
		remove,
		setActive,
		rename,
		setCwd,
		pushUser,
		startAssistant,
		appendAssistant,
		finishAssistant,
		setClaudeSessionId,
		clearActive
	};
}

export const chats = createChatsState();
