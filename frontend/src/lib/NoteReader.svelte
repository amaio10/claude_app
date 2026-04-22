<script lang="ts">
	import X from 'lucide-svelte/icons/x';
	import Code from 'lucide-svelte/icons/code';
	import BookOpen from 'lucide-svelte/icons/book-open';
	import Pencil from 'lucide-svelte/icons/pencil';
	import FileText from 'lucide-svelte/icons/file-text';
	import FileCode from 'lucide-svelte/icons/file-code-2';
	import Box from 'lucide-svelte/icons/box';
	import Image from 'lucide-svelte/icons/image';
	import Table from 'lucide-svelte/icons/table';
	import RefreshCw from 'lucide-svelte/icons/refresh-cw';
	import WrapText from 'lucide-svelte/icons/wrap-text';
	import Copy from 'lucide-svelte/icons/copy';
	import Save from 'lucide-svelte/icons/save';
	import Check from 'lucide-svelte/icons/check';
	import CircleAlert from 'lucide-svelte/icons/circle-alert';
	import Loader from 'lucide-svelte/icons/loader-circle';
	import { renderMarkdown } from './markdown';
	import { codeToHtml } from './highlighter';
	import { isMarkdown, isMesh, isImage, isData, langFor } from './filetypes';
	import StlViewer from './StlViewer.svelte';
	import ImageViewer from './ImageViewer.svelte';
	import CsvViewer from './CsvViewer.svelte';
	import MarkdownEditor, { type EditorApi } from './MarkdownEditor.svelte';

	type Props = {
		path: string;
		onClose: () => void;
	};
	let { path, onClose }: Props = $props();

	let rawContent = $state('');
	let truncated = $state(false);
	let size = $state(0);
	let html = $state('');
	let mode = $state<'rendered' | 'edit' | 'source'>('rendered');
	let loading = $state(false);
	let errMsg = $state('');
	let lang = $state<string | null>(null);
	let kind = $state<'markdown' | 'code' | 'mesh' | 'image' | 'csv'>('markdown');
	let wrap = $state(false);
	let copied = $state(false);
	let saveStatus = $state<'idle' | 'dirty' | 'saving' | 'saved' | 'error'>('idle');
	let saveMsg = $state('');
	let lastSavedAt = $state<number | null>(null);
	let editorApi = $state<EditorApi | null>(null);

	function relativeTime(ts: number): string {
		const d = Math.max(0, Math.floor((Date.now() - ts) / 1000));
		if (d < 5) return 'just now';
		if (d < 60) return `${d}s ago`;
		if (d < 3600) return `${Math.floor(d / 60)}m ago`;
		return new Date(ts).toLocaleTimeString();
	}

	let nowTick = $state(Date.now());
	$effect(() => {
		if (mode !== 'edit') return;
		const id = setInterval(() => (nowTick = Date.now()), 10_000);
		return () => clearInterval(id);
	});
	const savedLabel = $derived.by(() => {
		void nowTick;
		return lastSavedAt ? relativeTime(lastSavedAt) : '';
	});

	function handleSaveKeydown(e: KeyboardEvent) {
		if (mode !== 'edit') return;
		const isSave = (e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 's';
		if (!isSave) return;
		e.preventDefault();
		void editorApi?.flush();
	}

	const name = $derived(path.split('/').filter(Boolean).slice(-1)[0] || path);
	const lineCount = $derived(rawContent ? rawContent.split('\n').length : 0);

	$effect(() => {
		const p = path;
		if (p) load(p);
	});

	async function load(p: string) {
		loading = true;
		errMsg = '';
		try {
			const base = p.split('/').pop() || '';
			const ext = (base.split('.').pop() || '').toLowerCase();

			if (isMesh(ext)) {
				kind = 'mesh';
				lang = ext;
				rawContent = '';
				truncated = false;
				size = 0;
				html = '';
				return;
			}

			if (isImage(ext)) {
				kind = 'image';
				lang = ext;
				rawContent = '';
				truncated = false;
				size = 0;
				html = '';
				return;
			}

			if (isData(ext)) {
				kind = 'csv';
				lang = ext;
				rawContent = '';
				html = '';
				return;
			}

			const r = await fetch(`/api/fs/read?path=${encodeURIComponent(p)}`);
			if (!r.ok) {
				errMsg = await r.text();
				return;
			}
			const data = await r.json();
			rawContent = data.content;
			truncated = data.truncated;
			size = data.size;

			const detectedLang = langFor(base, ext);
			lang = detectedLang;

			if (isMarkdown(ext)) {
				kind = 'markdown';
				html = await renderMarkdown(data.content);
			} else {
				kind = 'code';
				html = await codeToHtml(data.content, detectedLang, {
					theme: 'github-dark'
				});
			}
		} catch (e) {
			errMsg = (e as Error).message;
		} finally {
			loading = false;
		}
	}

	function fmtSize(n: number): string {
		if (n < 1024) return `${n} B`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
		return `${(n / 1024 / 1024).toFixed(1)} MB`;
	}

	async function copyContent() {
		try {
			await navigator.clipboard.writeText(rawContent);
			copied = true;
			setTimeout(() => (copied = false), 1200);
		} catch (e) {
			console.error('[copy] failed', e);
		}
	}

	$effect(() => {
		if (mode !== 'edit') return;
		window.addEventListener('keydown', handleSaveKeydown, { capture: true });
		return () => window.removeEventListener('keydown', handleSaveKeydown, { capture: true });
	});

	const isDarkChrome = $derived(kind === 'code' || kind === 'mesh' || kind === 'image');
</script>

<div
	class="h-full flex flex-col min-w-0"
	class:dark-chrome={isDarkChrome}
	style:background={isDarkChrome ? '#24292e' : 'var(--color-bg)'}
	style:color={isDarkChrome ? '#e1e4e8' : 'var(--color-text)'}
	style:border-left="1px solid var(--color-border)"
>
	<div
		class="h-12 shrink-0 flex items-center justify-between px-3"
		style:border-bottom={isDarkChrome ? '1px solid #21262d' : '1px solid var(--color-border)'}
		style:background={isDarkChrome ? '#161b22' : 'var(--color-bg-elev)'}
	>
		<div class="flex items-center gap-2 min-w-0">
			{#if kind === 'code'}
				<FileCode class="size-4 opacity-70 shrink-0" />
			{:else if kind === 'mesh'}
				<Box class="size-4 opacity-70 shrink-0" />
			{:else if kind === 'image'}
				<Image class="size-4 opacity-70 shrink-0" />
			{:else if kind === 'csv'}
				<Table class="size-4 opacity-70 shrink-0" />
			{:else}
				<FileText class="size-4 opacity-70 shrink-0" />
			{/if}
			<span class="text-sm font-mono truncate" title={path}>{name}</span>
			{#if lang && lang !== 'text'}
				<span
					class="text-[9px] uppercase tracking-wider px-1.5 py-0.5 rounded shrink-0 font-mono"
					style:background={isDarkChrome ? '#21262d' : 'var(--color-surface)'}
					style:color={isDarkChrome ? '#8b949e' : 'var(--color-text-dim)'}
					style:border={isDarkChrome ? '1px solid #30363d' : '1px solid var(--color-border)'}
				>
					{lang}
				</span>
			{/if}
			{#if truncated}
				<span
					class="text-[10px] uppercase tracking-wider px-1.5 py-0.5 rounded shrink-0"
					style:background="#fef3c7"
					style:color="#78350f"
				>
					Truncated
				</span>
			{/if}
		</div>
		<div class="flex items-center gap-1">
			{#if kind === 'markdown'}
				{#if mode === 'edit'}
					<div
						class="save-pill"
						class:save-saving={saveStatus === 'saving'}
						class:save-saved={saveStatus === 'saved' || saveStatus === 'idle'}
						class:save-dirty={saveStatus === 'dirty'}
						class:save-error={saveStatus === 'error'}
						title={saveStatus === 'error' ? saveMsg : 'Auto-save is on'}
					>
						{#if saveStatus === 'saving'}
							<Loader class="size-3 spin" />
							<span>Saving…</span>
						{:else if saveStatus === 'dirty'}
							<span class="dot"></span>
							<span>Unsaved</span>
						{:else if saveStatus === 'error'}
							<CircleAlert class="size-3" />
							<span>Save failed</span>
						{:else if saveStatus === 'saved' && lastSavedAt}
							<Check class="size-3" />
							<span>Saved {savedLabel}</span>
						{:else}
							<Check class="size-3 opacity-60" />
							<span>Auto-save on</span>
						{/if}
					</div>
					<button
						class="h-7 px-2 rounded-md text-[11px] font-medium flex items-center gap-1 save-btn"
						class:save-btn-dirty={saveStatus === 'dirty' || saveStatus === 'error'}
						onclick={() => void editorApi?.flush()}
						disabled={saveStatus === 'saving'}
						title="Save now (Ctrl/Cmd+S)"
					>
						<Save class="size-3" /> Save
					</button>
				{/if}
				<button
					class="h-7 px-2 rounded-md text-[11px] font-medium flex items-center gap-1"
					style:background={mode === 'rendered'
						? 'var(--color-accent-soft)'
						: 'var(--color-surface)'}
					style:color="var(--color-text)"
					onclick={() => (mode = 'rendered')}
					title="Rendered"
				>
					<BookOpen class="size-3" /> Reader
				</button>
				<button
					class="h-7 px-2 rounded-md text-[11px] font-medium flex items-center gap-1"
					style:background={mode === 'edit'
						? 'var(--color-accent-soft)'
						: 'var(--color-surface)'}
					style:color="var(--color-text)"
					onclick={() => (mode = 'edit')}
					title="Edit"
				>
					<Pencil class="size-3" /> Edit
				</button>
				<button
					class="h-7 px-2 rounded-md text-[11px] font-medium flex items-center gap-1"
					style:background={mode === 'source'
						? 'var(--color-accent-soft)'
						: 'var(--color-surface)'}
					style:color="var(--color-text)"
					onclick={() => (mode = 'source')}
					title="Raw source"
				>
					<Code class="size-3" /> Source
				</button>
			{:else if kind === 'code'}
				<button
					class="size-7 rounded-md flex items-center justify-center"
					style:background={wrap ? '#30363d' : '#21262d'}
					style:color="#c9d1d9"
					onclick={() => (wrap = !wrap)}
					title="Toggle soft wrap"
				>
					<WrapText class="size-3.5" />
				</button>
				<button
					class="size-7 rounded-md flex items-center justify-center"
					style:background={copied ? '#1f6feb' : '#21262d'}
					style:color={copied ? 'white' : '#c9d1d9'}
					onclick={copyContent}
					title={copied ? 'Copied!' : 'Copy'}
				>
					<Copy class="size-3.5" />
				</button>
			{/if}
			<button
				class="size-7 rounded-md flex items-center justify-center"
				style:background={isDarkChrome ? '#21262d' : 'var(--color-surface)'}
				style:color={isDarkChrome ? '#c9d1d9' : 'var(--color-text)'}
				onclick={() => load(path)}
				title="Reload"
			>
				<RefreshCw class="size-3" />
			</button>
			<button
				class="size-7 rounded-md flex items-center justify-center ml-1"
				style:background={isDarkChrome ? '#21262d' : 'var(--color-surface)'}
				style:color={isDarkChrome ? '#c9d1d9' : 'var(--color-text)'}
				onclick={onClose}
				title="Close file"
			>
				<X class="size-3.5" />
			</button>
		</div>
	</div>

	<div class="flex-1 overflow-hidden min-h-0 flex flex-col">
		{#if loading}
			<div class="text-xs opacity-60 p-8" style:color={isDarkChrome ? '#8b949e' : 'inherit'}>
				Loading…
			</div>
		{:else if errMsg}
			<div class="text-sm p-6" style:color="#f85149">{errMsg}</div>
		{:else if kind === 'markdown' && mode === 'rendered'}
			<div class="flex-1 overflow-y-auto">
				<div class="prose-obsidian">
					<div class="md-body">{@html html}</div>
				</div>
			</div>
		{:else if kind === 'markdown' && mode === 'edit'}
			{#key path}
				<MarkdownEditor
					{path}
					initial={rawContent}
					onStatus={(s, m) => {
						saveStatus = s;
						saveMsg = m ?? '';
						if (s === 'saved') lastSavedAt = Date.now();
					}}
					onContentChange={(md) => (rawContent = md)}
					onReady={(api) => (editorApi = api)}
				/>
			{/key}
		{:else if kind === 'markdown' && mode === 'source'}
			<div class="flex-1 overflow-auto">
				<pre
					class="px-8 py-6 text-[13px] leading-relaxed font-mono whitespace-pre-wrap"
					style:color="var(--color-text)">{rawContent}</pre>
			</div>
		{:else if kind === 'mesh'}
			<StlViewer {path} />
		{:else if kind === 'image'}
			<ImageViewer {path} />
		{:else if kind === 'csv'}
			<CsvViewer {path} />
		{:else}
			<div class="code-view" class:code-view-wrap={wrap}>
				<div class="code-scroll">{@html html}</div>
			</div>
			<div class="code-status">
				<span class="font-mono">{lineCount} lines</span>
				<span class="sep">·</span>
				<span class="font-mono">{fmtSize(size)}</span>
				{#if lang && lang !== 'text'}
					<span class="sep">·</span>
					<span class="font-mono uppercase tracking-wider">{lang}</span>
				{/if}
				<span class="ml-auto font-mono truncate opacity-70" title={path}>{path}</span>
			</div>
		{/if}
	</div>
</div>

<style>
	.dark-chrome :global(::-webkit-scrollbar-thumb) {
		background: #30363d;
	}
	.dark-chrome :global(::-webkit-scrollbar-thumb:hover) {
		background: #484f58;
	}

	.code-view {
		flex: 1;
		min-height: 0;
		overflow: hidden;
		position: relative;
		font-family: var(--font-mono);
		font-feature-settings: 'calt' 0, 'liga' 0;
		font-variant-ligatures: none;
	}

	.code-scroll {
		height: 100%;
		overflow: auto;
		scrollbar-color: #30363d transparent;
	}

	/* Let shiki's own inline background/color win; we only control layout. */
	.code-view :global(pre.shiki),
	.code-view :global(pre.shiki-plain) {
		margin: 0;
		padding: 12px 0 !important;
		border: 0;
		border-radius: 0;
		min-height: 100%;
		tab-size: 4;
		font-family: var(--font-mono) !important;
		font-size: 13px !important;
		line-height: 19px !important;
		overflow: visible;
		white-space: pre;
	}

	.code-view :global(pre.shiki-plain) {
		background: #24292e;
		color: #e1e4e8;
	}

	.code-view :global(pre.shiki code),
	.code-view :global(pre.shiki-plain code) {
		display: block;
		padding: 0;
		background: transparent;
		font-family: inherit;
		counter-reset: line 0;
	}

	.code-view :global(pre.shiki .line) {
		display: block;
		position: relative;
		padding: 0 20px 0 64px;
		min-height: 19px;
	}

	.code-view :global(pre.shiki .line::before) {
		counter-increment: line 1;
		content: counter(line);
		position: absolute;
		left: 0;
		width: 52px;
		padding-right: 14px;
		text-align: right;
		color: rgba(255, 255, 255, 0.25);
		font-variant-numeric: tabular-nums;
		user-select: none;
		pointer-events: none;
	}

	.code-view :global(pre.shiki .line:hover) {
		background: rgba(255, 255, 255, 0.04);
	}
	.code-view :global(pre.shiki .line:hover::before) {
		color: rgba(255, 255, 255, 0.55);
	}

	.code-view :global(pre.shiki-plain code) {
		padding: 0 20px 0 20px;
		white-space: pre;
		display: block;
	}

	.code-view-wrap :global(pre.shiki),
	.code-view-wrap :global(pre.shiki-plain) {
		white-space: pre-wrap;
	}
	.code-view-wrap :global(pre.shiki .line) {
		white-space: pre-wrap;
		word-break: break-word;
	}
	.code-view-wrap :global(pre.shiki-plain code) {
		white-space: pre-wrap;
	}

	.code-status {
		height: 24px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 14px;
		font-size: 11px;
		background: #1b1f24;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		color: #8b949e;
	}
	.code-status .sep {
		color: #30363d;
	}

	.save-pill {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		height: 24px;
		padding: 0 8px;
		border-radius: 999px;
		font-size: 11px;
		font-weight: 500;
		letter-spacing: 0.01em;
		margin-right: 4px;
		border: 1px solid transparent;
		transition:
			background 0.15s ease,
			color 0.15s ease,
			border-color 0.15s ease;
	}
	.save-pill .dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: currentColor;
		display: inline-block;
	}
	.save-pill.save-dirty {
		background: color-mix(in oklab, #f59e0b 14%, transparent);
		color: #b45309;
		border-color: color-mix(in oklab, #f59e0b 28%, transparent);
	}
	.save-pill.save-saving {
		background: color-mix(in oklab, var(--color-accent) 14%, transparent);
		color: var(--color-accent);
		border-color: color-mix(in oklab, var(--color-accent) 28%, transparent);
	}
	.save-pill.save-saved {
		background: color-mix(in oklab, #22c55e 12%, transparent);
		color: #15803d;
		border-color: color-mix(in oklab, #22c55e 24%, transparent);
	}
	.save-pill.save-error {
		background: color-mix(in oklab, #ef4444 14%, transparent);
		color: #b91c1c;
		border-color: color-mix(in oklab, #ef4444 28%, transparent);
	}
	@media (prefers-color-scheme: dark) {
		.save-pill.save-dirty {
			color: #fbbf24;
		}
		.save-pill.save-saved {
			color: #4ade80;
		}
		.save-pill.save-error {
			color: #f87171;
		}
	}

	.save-btn {
		background: var(--color-surface);
		color: var(--color-text);
		border: 1px solid var(--color-border);
	}
	.save-btn:hover:not(:disabled) {
		background: var(--color-bg-elev);
	}
	.save-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.save-btn.save-btn-dirty {
		background: var(--color-accent);
		color: white;
		border-color: transparent;
	}
	.save-btn.save-btn-dirty:hover:not(:disabled) {
		filter: brightness(1.08);
	}

	:global(.spin) {
		animation: nr-spin 0.8s linear infinite;
	}
	@keyframes nr-spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
