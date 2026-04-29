<script lang="ts">
	import { onMount } from 'svelte';
	import VoiceInput from '$lib/VoiceInput.svelte';
	import Conversation from '$lib/Conversation.svelte';
	import PathPicker from '$lib/PathPicker.svelte';
	import MiniTerminal from '$lib/MiniTerminal.svelte';
	import SlashMenu from '$lib/SlashMenu.svelte';
	import Sidebar from '$lib/Sidebar.svelte';
	import NotesPanel from '$lib/NotesPanel.svelte';
	import NoteReader from '$lib/NoteReader.svelte';
	import { PromptClient } from '$lib/ws';
	import { chats } from '$lib/stores.svelte';
	import { matchCommands, type Cmd } from '$lib/commands';
	import Send from 'lucide-svelte/icons/send';
	import Terminal from 'lucide-svelte/icons/terminal';
	import Sparkles from 'lucide-svelte/icons/sparkles';
	import PanelLeft from 'lucide-svelte/icons/panel-left';
	import PanelRight from 'lucide-svelte/icons/panel-right';
	import BookOpen from 'lucide-svelte/icons/book-open';

	let client = new PromptClient();
	let text = $state('');
	let inputEl: HTMLTextAreaElement | null = $state(null);
	let showTerminal = $state(false);
	let showNotes = $state(false);
	let openFilePath = $state<string | null>(null);
	let sidebarCollapsed = $state(false);
	let recent = $state<string[]>([]);
	let groqConfigured = $state(false);
	let version = $state('');
	let defaultCwd = $state('');
	let slashActive = $state(0);
	let readerWidthPct = $state(50);
	let dragging = $state(false);
	let pastedImages = $state<
		{ id: string; path: string; thumbUrl: string; uploading: boolean; error?: string }[]
	>([]);

	const active = $derived(chats.active);
	const suggestions = $derived(matchCommands(text));
	const slashOpen = $derived(suggestions.length > 0 && text.trim().startsWith('/'));

	onMount(async () => {
		try {
			const h = await fetch('/api/health').then((r) => r.json());
			defaultCwd = h.default_cwd;
			groqConfigured = h.groq_configured;
			version = h.version;
			console.log('[health]', h);
		} catch (e) {
			console.error('[health] failed', e);
		}

		try {
			const saved = localStorage.getItem('recent_cwds');
			if (saved) recent = JSON.parse(saved);
		} catch {}
		try {
			const w = localStorage.getItem('reader_width_pct');
			if (w) readerWidthPct = Math.min(75, Math.max(25, Number(w)));
		} catch {}

		if (chats.chats.length === 0 && defaultCwd) {
			chats.create(defaultCwd);
		}

		client.setHandlers({
			onOpen: () => (chats.connected = true),
			onClose: () => (chats.connected = false),
			onStart: (e) => console.log('[claude] start', e),
			onDelta: (delta) => chats.appendAssistant(delta),
			onEvent: (data) => console.debug('[claude] event', data),
			onDone: (e) => {
				console.log('[claude] done', e);
				if (e.session_id) chats.setClaudeSessionId(e.session_id);
				chats.finishAssistant();
				chats.inflight = false;
			},
			onError: (msg) => {
				console.error('[claude] error', msg);
				chats.appendAssistant(`\n[error] ${msg}`);
				chats.finishAssistant();
				chats.inflight = false;
			}
		});
		client.connect();
	});

	function persistRecent(p: string) {
		recent = [p, ...recent.filter((x) => x !== p)].slice(0, 8);
		localStorage.setItem('recent_cwds', JSON.stringify(recent));
	}

	function setCwd(p: string) {
		chats.setCwd(p);
		persistRecent(p);
		console.log('[cwd] set to', p);
	}

	function newChat() {
		const cwd = active?.cwd || defaultCwd;
		chats.create(cwd);
		setTimeout(
			() => document.querySelector<HTMLButtonElement>('[data-path-trigger]')?.click(),
			50
		);
	}

	function autosize(el: HTMLTextAreaElement | null) {
		if (!el) return;
		el.style.height = 'auto';
		el.style.height = Math.min(220, el.scrollHeight) + 'px';
	}

	function sendPrompt(prompt: string) {
		const a = chats.active;
		if (!prompt || chats.inflight || !a) return;
		chats.pushUser(prompt);
		chats.startAssistant();
		chats.inflight = true;
		const ok = client.send(prompt, a.cwd || null, a.claudeSessionId);
		if (!ok) {
			chats.appendAssistant('\n[error] backend not reachable');
			chats.finishAssistant();
			chats.inflight = false;
		}
	}

	function executeCommand(c: Cmd) {
		console.log('[cmd]', c.trigger, c.kind);
		text = '';
		autosize(inputEl);
		if (c.kind === 'client') {
			runClientCommand(c.trigger);
		} else if (c.kind === 'prompt' && c.body) {
			sendPrompt(c.body);
		}
	}

	function runClientCommand(trigger: string) {
		switch (trigger) {
			case '/clear':
			case '/new':
				chats.clearActive();
				break;
			case '/help':
				chats.pushUser('/help');
				chats.startAssistant();
				chats.appendAssistant(formatHelp());
				chats.finishAssistant();
				break;
			case '/cwd':
				document.querySelector<HTMLButtonElement>('[data-path-trigger]')?.click();
				break;
			case '/terminal':
				showTerminal = !showTerminal;
				break;
		}
	}

	function formatHelp(): string {
		const lines = ['Available commands:\n'];
		for (const c of matchCommands('/')) {
			lines.push(`  ${c.trigger.padEnd(18)} ${c.description}`);
		}
		return lines.join('\n');
	}

	async function onPaste(e: ClipboardEvent) {
		const items = e.clipboardData?.items;
		if (!items) return;
		const files: File[] = [];
		for (const it of items) {
			if (it.kind === 'file' && it.type.startsWith('image/')) {
				const f = it.getAsFile();
				if (f) files.push(f);
			}
		}
		if (files.length === 0) return;
		e.preventDefault();
		const cwd = active?.cwd || defaultCwd || '';
		for (const file of files) {
			const id = crypto.randomUUID();
			const thumbUrl = URL.createObjectURL(file);
			pastedImages.push({ id, path: '', thumbUrl, uploading: true });
			try {
				const fd = new FormData();
				fd.append('image', file);
				fd.append('cwd', cwd);
				const r = await fetch('/api/paste-image', { method: 'POST', body: fd });
				if (!r.ok) throw new Error(`upload ${r.status}`);
				const body = (await r.json()) as { path: string };
				const entry = pastedImages.find((x) => x.id === id);
				if (entry) {
					entry.path = body.path;
					entry.uploading = false;
				}
			} catch (err) {
				console.error('[paste] upload failed', err);
				const entry = pastedImages.find((x) => x.id === id);
				if (entry) {
					entry.uploading = false;
					entry.error = String(err);
				}
			}
		}
	}

	function removePastedImage(id: string) {
		const entry = pastedImages.find((x) => x.id === id);
		if (entry) URL.revokeObjectURL(entry.thumbUrl);
		pastedImages = pastedImages.filter((x) => x.id !== id);
	}

	function clearPastedImages() {
		for (const i of pastedImages) URL.revokeObjectURL(i.thumbUrl);
		pastedImages = [];
	}

	function send() {
		const body = text.trim();
		const ready = pastedImages.filter((i) => i.path && !i.error);
		if (!body && ready.length === 0) return;
		if (chats.inflight) return;
		if (pastedImages.some((i) => i.uploading)) return;
		if (body.startsWith('/') && ready.length === 0) {
			const exact = matchCommands(body).find((c) => c.trigger === body);
			if (exact) {
				executeCommand(exact);
				return;
			}
		}
		const refs = ready.map((i) => `![](${i.path})`).join('\n');
		const prompt = refs ? (body ? `${refs}\n\n${body}` : refs) : body;
		sendPrompt(prompt);
		text = '';
		clearPastedImages();
		autosize(inputEl);
	}

	function onVoice(t: string) {
		text = (text ? text + ' ' : '') + t;
		autosize(inputEl);
		inputEl?.focus();
	}

	function onKey(e: KeyboardEvent) {
		if (slashOpen) {
			if (e.key === 'ArrowDown') {
				e.preventDefault();
				slashActive = (slashActive + 1) % suggestions.length;
				return;
			}
			if (e.key === 'ArrowUp') {
				e.preventDefault();
				slashActive = (slashActive - 1 + suggestions.length) % suggestions.length;
				return;
			}
			if (e.key === 'Tab' || (e.key === 'Enter' && !e.shiftKey)) {
				e.preventDefault();
				const c = suggestions[slashActive];
				if (c) executeCommand(c);
				return;
			}
			if (e.key === 'Escape') {
				e.preventDefault();
				text = '';
				autosize(inputEl);
				return;
			}
		}
		if (e.key === 'Enter' && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
			e.preventDefault();
			send();
		}
	}

	$effect(() => {
		if (slashOpen) {
			slashActive = Math.min(slashActive, suggestions.length - 1);
			if (slashActive < 0) slashActive = 0;
		} else {
			slashActive = 0;
		}
	});

	function startDrag() {
		dragging = true;
	}
	function onDragMove(e: MouseEvent) {
		if (!dragging) return;
		const bodyEl = document.getElementById('workspace-body');
		if (!bodyEl) return;
		const rect = bodyEl.getBoundingClientRect();
		const notesOffset = showNotes ? 280 : 0;
		const available = rect.width - notesOffset;
		const rightEdgeOfReader = rect.right - notesOffset;
		const pct = ((rightEdgeOfReader - e.clientX) / available) * 100;
		readerWidthPct = Math.min(75, Math.max(25, pct));
	}
	function endDrag() {
		if (dragging) {
			dragging = false;
			localStorage.setItem('reader_width_pct', String(Math.round(readerWidthPct)));
		}
	}
</script>

<svelte:window onmousemove={onDragMove} onmouseup={endDrag} />

<div class="h-screen w-screen flex" style:background="var(--color-bg)">
	<Sidebar
		onNew={newChat}
		collapsed={sidebarCollapsed}
		onToggle={() => (sidebarCollapsed = !sidebarCollapsed)}
	/>

	<div class="flex-1 flex flex-col min-w-0">
		<header
			class="flex items-center justify-between px-4 h-12 shrink-0"
			style:border-bottom="1px solid var(--color-border)"
			style:background="var(--color-bg-elev)"
		>
			<div class="flex items-center gap-3 min-w-0">
				{#if sidebarCollapsed}
					<button
						class="size-8 rounded-md flex items-center justify-center shrink-0"
						onclick={() => (sidebarCollapsed = false)}
						title="Show chats"
						style:color="var(--color-text)"
					>
						<PanelLeft class="size-4 opacity-70" />
					</button>
				{/if}
				<div
					class="size-6 rounded-md flex items-center justify-center shrink-0"
					style:background="var(--color-accent)"
				>
					<Sparkles class="size-3.5 text-white" />
				</div>
				<span class="text-sm font-semibold tracking-tight truncate max-w-[220px]">
					{active?.label ?? 'Claude'}
				</span>
				<span class="text-[11px] opacity-40 font-mono shrink-0">v{version}</span>
			</div>
			<div class="flex items-center gap-2">
				<PathPicker {recent} onSet={setCwd} />
				<button
					class="h-8 px-2.5 rounded-[8px] text-xs font-medium flex items-center gap-1.5"
					style:background={showNotes ? 'var(--color-accent-soft)' : 'var(--color-surface)'}
					style:border="1px solid var(--color-border)"
					style:color="var(--color-text)"
					onclick={() => (showNotes = !showNotes)}
					title="Toggle notes panel"
				>
					<BookOpen class="size-3.5" />
					Notes
				</button>
				<button
					class="h-8 px-2.5 rounded-[8px] text-xs font-medium flex items-center gap-1.5"
					style:background={showTerminal ? 'var(--color-accent-soft)' : 'var(--color-surface)'}
					style:border="1px solid var(--color-border)"
					style:color="var(--color-text)"
					onclick={() => (showTerminal = !showTerminal)}
					title="Toggle terminal"
				>
					<Terminal class="size-3.5" />
					Terminal
				</button>
				{#if openFilePath}
					<button
						class="h-8 px-2.5 rounded-[8px] text-xs font-medium flex items-center gap-1.5"
						style:background="var(--color-surface)"
						style:border="1px solid var(--color-border)"
						style:color="var(--color-text)"
						onclick={() => (openFilePath = null)}
						title="Close open file"
					>
						<PanelRight class="size-3.5" />
						Close file
					</button>
				{/if}
				<span
					class="flex items-center gap-1.5 text-[10px] font-medium uppercase tracking-wider opacity-70 pl-2"
				>
					<span
						class="size-2 rounded-full inline-block"
						style:background={chats.connected ? '#22c55e' : '#ef4444'}
					></span>
					{chats.connected ? 'online' : 'offline'}
				</span>
			</div>
		</header>

		{#if !groqConfigured}
			<div
				class="px-4 py-1.5 text-[11px] font-medium text-center"
				style:background="#fef3c7"
				style:color="#78350f"
			>
				GROQ_API_KEY missing — voice transcription disabled. Set it in .env and restart backend.
			</div>
		{/if}

		<div id="workspace-body" class="flex-1 flex min-h-0">
			<!-- Chat column -->
			<div class="flex-1 min-w-0 flex flex-col">
				<Conversation />
				<MiniTerminal visible={showTerminal} />
				<div
					class="shrink-0 px-4 py-3"
					style:border-top="1px solid var(--color-border)"
					style:background="var(--color-bg-elev)"
				>
					<div class="mx-auto max-w-3xl relative">
						{#if slashOpen}
							<SlashMenu items={suggestions} activeIndex={slashActive} onPick={executeCommand} />
						{/if}
						<div
							class="rounded-[14px] flex flex-col gap-1.5 p-2"
							style:background="var(--color-surface)"
							style:border="1px solid var(--color-border)"
						>
							{#if pastedImages.length > 0}
								<div class="flex flex-wrap gap-1.5 px-1 pt-0.5">
									{#each pastedImages as img (img.id)}
										<div
											class="relative group rounded-[8px] overflow-hidden"
											style:border="1px solid var(--color-border)"
											style:width="56px"
											style:height="56px"
										>
											<img
												src={img.thumbUrl}
												alt="pasted"
												class="w-full h-full object-cover"
												style:opacity={img.uploading ? '0.5' : '1'}
											/>
											{#if img.uploading}
												<div
													class="absolute inset-0 flex items-center justify-center text-[9px] font-mono"
													style:color="var(--color-text)"
													style:background="rgba(0,0,0,0.25)"
												>
													…
												</div>
											{/if}
											{#if img.error}
												<div
													class="absolute inset-0 flex items-center justify-center text-[9px] font-mono text-white"
													style:background="rgba(220,38,38,0.8)"
													title={img.error}
												>
													err
												</div>
											{/if}
											<button
												class="absolute top-0.5 right-0.5 size-4 rounded-full text-[10px] leading-none flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
												style:background="rgba(0,0,0,0.7)"
												style:color="white"
												onclick={() => removePastedImage(img.id)}
												aria-label="Remove image"
												title="Remove"
											>
												×
											</button>
										</div>
									{/each}
								</div>
							{/if}
							<div class="flex items-end gap-2">
								<VoiceInput onTranscribed={onVoice} disabled={!groqConfigured} />
								<textarea
									bind:this={inputEl}
									bind:value={text}
									oninput={(e) => autosize(e.currentTarget)}
									onkeydown={onKey}
									onpaste={onPaste}
									placeholder={active
										? `Ask in "${active.label}" — type / for commands, paste images with Ctrl+V`
										: 'Create a chat to get started'}
									disabled={!active}
									rows="1"
									class="flex-1 bg-transparent resize-none outline-none text-[14px] leading-6 px-2 py-1.5 disabled:opacity-50"
									style:color="var(--color-text)"
									style:min-height="36px"
									style:max-height="220px"
								></textarea>
							<button
								class="size-9 rounded-[10px] flex items-center justify-center transition-all"
								style:background={(text.trim() || pastedImages.some((i) => i.path && !i.error)) &&
								!chats.inflight &&
								!pastedImages.some((i) => i.uploading) &&
								active
									? 'var(--color-accent)'
									: 'var(--color-border)'}
								style:color={(text.trim() || pastedImages.some((i) => i.path && !i.error)) &&
								!chats.inflight &&
								!pastedImages.some((i) => i.uploading) &&
								active
									? 'white'
									: 'var(--color-text-dim)'}
								disabled={(!text.trim() && !pastedImages.some((i) => i.path && !i.error)) ||
									chats.inflight ||
									pastedImages.some((i) => i.uploading) ||
									!active}
								onclick={send}
								aria-label="Send"
							>
								<Send class="size-4" />
							</button>
							</div>
						</div>
					</div>
					<div class="mx-auto max-w-3xl pt-1.5 text-[10px] opacity-50 font-mono text-center">
						Enter · Shift+Enter newline · / commands · cwd: {active?.cwd ?? defaultCwd ?? '—'}
					</div>
				</div>
			</div>

			<!-- Draggable splitter + file reader (right of chat, left of notes) -->
			{#if openFilePath}
				<div
					role="separator"
					aria-orientation="vertical"
					tabindex="-1"
					class="w-[5px] cursor-col-resize shrink-0 transition-colors"
					style:background={dragging ? 'var(--color-accent)' : 'var(--color-border)'}
					onmousedown={startDrag}
				></div>
				<div
					class="shrink-0 min-w-0"
					style:width="{readerWidthPct}%"
					style:user-select={dragging ? 'none' : 'auto'}
				>
					<NoteReader path={openFilePath} onClose={() => (openFilePath = null)} />
				</div>
			{/if}

			{#if showNotes}
				<NotesPanel
					onSelect={(p) => (openFilePath = p)}
					onClose={() => (showNotes = false)}
					activePath={openFilePath}
				/>
			{/if}
		</div>
	</div>
</div>
