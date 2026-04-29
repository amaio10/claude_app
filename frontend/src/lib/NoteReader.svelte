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
	import Play from 'lucide-svelte/icons/play';
	import Download from 'lucide-svelte/icons/download';
	import { renderMarkdown } from './markdown';
	import { codeToHtml } from './highlighter';
	import { isMarkdown, isMesh, isImage, isData, isTex, langFor } from './filetypes';
	import StlViewer from './StlViewer.svelte';
	import ImageViewer from './ImageViewer.svelte';
	import CsvViewer from './CsvViewer.svelte';
	import MarkdownEditor, { type EditorApi } from './MarkdownEditor.svelte';
	import TexEditor, { type TexEditorApi } from './TexEditor.svelte';
	import PdfViewer from './PdfViewer.svelte';

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
	let kind = $state<'markdown' | 'code' | 'mesh' | 'image' | 'csv' | 'tex'>('markdown');
	let wrap = $state(false);
	let copied = $state(false);
	let saveStatus = $state<'idle' | 'dirty' | 'saving' | 'saved' | 'error'>('idle');
	let saveMsg = $state('');
	let lastSavedAt = $state<number | null>(null);
	let editorApi = $state<EditorApi | null>(null);
	let texApi = $state<TexEditorApi | null>(null);

	// TeX compile state
	let texCompiling = $state(false);
	let texHasPdf = $state(false);
	let texPdfVersion = $state(0);
	let texLog = $state('');
	let texCompileError = $state(false);
	let texShowLog = $state(false);

	// TeX split (editor | pdf) — persisted
	const TEX_SPLIT_KEY = 'tex-split-ratio';
	let texSplitRatio = $state(0.5);
	let texSplitEl: HTMLDivElement | null = $state(null);
	let texDragging = $state(false);
	$effect(() => {
		if (typeof localStorage === 'undefined') return;
		const v = parseFloat(localStorage.getItem(TEX_SPLIT_KEY) || '');
		if (!isNaN(v) && v > 0.1 && v < 0.9) texSplitRatio = v;
	});

	function onSplitPointerDown(e: PointerEvent) {
		if (!texSplitEl) return;
		texDragging = true;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		e.preventDefault();
	}
	function onSplitPointerMove(e: PointerEvent) {
		if (!texDragging || !texSplitEl) return;
		const rect = texSplitEl.getBoundingClientRect();
		const MIN_PX = 220;
		const x = e.clientX - rect.left;
		const clamped = Math.max(MIN_PX, Math.min(rect.width - MIN_PX, x));
		texSplitRatio = clamped / rect.width;
	}
	function onSplitPointerUp(e: PointerEvent) {
		if (!texDragging) return;
		texDragging = false;
		try {
			(e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
		} catch {
			// ignore
		}
		if (typeof localStorage !== 'undefined') {
			localStorage.setItem(TEX_SPLIT_KEY, String(texSplitRatio));
		}
	}
	function onSplitDoubleClick() {
		texSplitRatio = 0.5;
		if (typeof localStorage !== 'undefined') {
			localStorage.setItem(TEX_SPLIT_KEY, '0.5');
		}
	}

	function relativeTime(ts: number): string {
		const d = Math.max(0, Math.floor((Date.now() - ts) / 1000));
		if (d < 5) return 'just now';
		if (d < 60) return `${d}s ago`;
		if (d < 3600) return `${Math.floor(d / 60)}m ago`;
		return new Date(ts).toLocaleTimeString();
	}

	let nowTick = $state(Date.now());
	$effect(() => {
		if (mode !== 'edit' && kind !== 'tex') return;
		const id = setInterval(() => (nowTick = Date.now()), 10_000);
		return () => clearInterval(id);
	});
	const savedLabel = $derived.by(() => {
		void nowTick;
		return lastSavedAt ? relativeTime(lastSavedAt) : '';
	});

	function handleSaveKeydown(e: KeyboardEvent) {
		const isSave = (e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 's';
		if (!isSave) return;
		if (kind === 'markdown' && mode === 'edit') {
			e.preventDefault();
			void editorApi?.flush();
		} else if (kind === 'tex') {
			e.preventDefault();
			void texApi?.flush();
		}
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
		// reset per-file tex state
		texHasPdf = false;
		texPdfVersion = 0;
		texLog = '';
		texCompileError = false;
		texShowLog = false;
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
			} else if (isTex(ext)) {
				kind = 'tex';
				html = '';
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
		if (mode !== 'edit' && kind !== 'tex') return;
		window.addEventListener('keydown', handleSaveKeydown, { capture: true });
		return () => window.removeEventListener('keydown', handleSaveKeydown, { capture: true });
	});

	const isDarkChrome = $derived(
		kind === 'code' || kind === 'mesh' || kind === 'image' || kind === 'tex'
	);

	async function recompileTex() {
		if (texCompiling) return;
		// flush any pending save first so tectonic sees the latest content
		if (texApi) await texApi.flush();
		texCompiling = true;
		texCompileError = false;
		try {
			const r = await fetch('/api/tex/compile', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ path })
			});
			if (!r.ok) {
				texCompileError = true;
				texLog = await r.text();
				texShowLog = true;
				return;
			}
			const data = (await r.json()) as { ok: boolean; pdf_path: string | null; log: string };
			texLog = data.log || '';
			if (data.ok) {
				texHasPdf = true;
				texPdfVersion = Date.now();
				texCompileError = false;
				texShowLog = false;
			} else {
				texCompileError = true;
				texShowLog = true;
			}
		} catch (e) {
			texCompileError = true;
			texLog = (e as Error).message;
			texShowLog = true;
		} finally {
			texCompiling = false;
		}
	}

	async function savePdfAs() {
		if (!texHasPdf) return;
		try {
			const r = await fetch(
				`/api/tex/pdf?path=${encodeURIComponent(path)}&v=${texPdfVersion}`
			);
			if (!r.ok) return;
			const blob = await r.blob();
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			const stem = (name || 'document').replace(/\.tex$/i, '');
			a.download = `${stem}.pdf`;
			document.body.appendChild(a);
			a.click();
			a.remove();
			setTimeout(() => URL.revokeObjectURL(url), 2000);
		} catch (e) {
			console.error('[save-pdf]', e);
		}
	}

	const pdfSrc = $derived(
		texHasPdf ? `/api/tex/pdf?path=${encodeURIComponent(path)}&v=${texPdfVersion}` : ''
	);
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
			{#if kind === 'code' || kind === 'tex'}
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
			{:else if kind === 'tex'}
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
					class="h-7 px-2 rounded-md text-[11px] font-medium flex items-center gap-1 tex-btn"
					onclick={() => void texApi?.flush()}
					disabled={saveStatus === 'saving'}
					title="Save .tex now (Ctrl/Cmd+S)"
				>
					<Save class="size-3" /> Save
				</button>
				<button
					class="h-7 px-2 rounded-md text-[11px] font-medium flex items-center gap-1 tex-btn-primary"
					onclick={recompileTex}
					disabled={texCompiling}
					title="Compile with tectonic"
				>
					{#if texCompiling}
						<Loader class="size-3 spin" />
						Compiling…
					{:else}
						<Play class="size-3" /> Recompile
					{/if}
				</button>
				<button
					class="h-7 px-2 rounded-md text-[11px] font-medium flex items-center gap-1 tex-btn"
					onclick={savePdfAs}
					disabled={!texHasPdf}
					title={texHasPdf ? 'Download PDF' : 'Compile first'}
				>
					<Download class="size-3" /> Save as PDF
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
		{:else if kind === 'tex'}
			<div
				bind:this={texSplitEl}
				class="tex-split"
				class:tex-split-dragging={texDragging}
				style:--tex-left={`${(texSplitRatio * 100).toFixed(3)}%`}
			>
				<div class="tex-pane tex-left">
					{#key path}
						<TexEditor
							{path}
							initial={rawContent}
							onStatus={(s, m) => {
								saveStatus = s;
								saveMsg = m ?? '';
								if (s === 'saved') lastSavedAt = Date.now();
							}}
							onContentChange={(txt) => (rawContent = txt)}
							onReady={(api) => (texApi = api)}
						/>
					{/key}
				</div>
				<div
					class="tex-gutter"
					role="separator"
					aria-orientation="vertical"
					aria-label="Resize editor / PDF"
					title="Drag to resize · double-click to reset"
					onpointerdown={onSplitPointerDown}
					onpointermove={onSplitPointerMove}
					onpointerup={onSplitPointerUp}
					onpointercancel={onSplitPointerUp}
					ondblclick={onSplitDoubleClick}
				></div>
				<div class="tex-pane tex-right">
					{#if texHasPdf}
						{#key pdfSrc}
							<PdfViewer src={pdfSrc} />
						{/key}
					{:else}
						<div class="tex-empty">
							{#if texCompiling}
								<Loader class="size-4 spin opacity-70" />
								<span>Compiling…</span>
							{:else if texCompileError}
								<span class="tex-err-icon"><CircleAlert class="size-4" /></span>
								<span>Compile failed — see log below.</span>
							{:else}
								<Play class="size-4 opacity-70" />
								<span>Hit <b>Recompile</b> to render the PDF.</span>
							{/if}
						</div>
					{/if}
					{#if texLog && (texShowLog || texCompileError)}
						<div class="tex-log-wrap">
							<div class="tex-log-head">
								<span>tectonic log</span>
								<button
									class="tex-log-close"
									onclick={() => (texShowLog = false)}
									title="Hide log"
								>×</button>
							</div>
							<pre class="tex-log">{texLog}</pre>
						</div>
					{/if}
				</div>
			</div>
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

	.tex-split {
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: minmax(0, var(--tex-left, 50%)) 6px minmax(0, 1fr);
		background: #21262d;
	}
	.tex-split-dragging {
		cursor: col-resize;
		user-select: none;
	}
	.tex-split-dragging :global(*) {
		user-select: none !important;
	}
	.tex-gutter {
		position: relative;
		background: #21262d;
		cursor: col-resize;
		touch-action: none;
	}
	.tex-gutter::before {
		content: '';
		position: absolute;
		inset: 0;
		left: -3px;
		right: -3px;
	}
	.tex-gutter::after {
		content: '';
		position: absolute;
		left: 50%;
		top: 50%;
		width: 2px;
		height: 28px;
		background: #30363d;
		transform: translate(-50%, -50%);
		border-radius: 1px;
		transition: background 0.12s ease;
	}
	.tex-gutter:hover::after {
		background: #58a6ff;
	}
	.tex-split-dragging .tex-gutter::after {
		background: #58a6ff;
	}
	.tex-pane {
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 0;
		background: #24292e;
		overflow: hidden;
	}
	.tex-right {
		background: #1b1f24;
		position: relative;
	}
	.tex-empty {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		color: #8b949e;
		font-size: 13px;
	}
	.tex-log-wrap {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		max-height: 45%;
		display: flex;
		flex-direction: column;
		background: #161b22;
		border-top: 1px solid #30363d;
		z-index: 2;
	}
	.tex-log-head {
		height: 26px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 10px;
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: #8b949e;
		background: #0d1117;
		border-bottom: 1px solid #21262d;
	}
	.tex-log-close {
		background: transparent;
		border: 0;
		color: #8b949e;
		font-size: 14px;
		line-height: 1;
		padding: 0 4px;
		cursor: pointer;
	}
	.tex-log-close:hover {
		color: #e1e4e8;
	}
	.tex-log {
		margin: 0;
		padding: 10px 12px;
		overflow: auto;
		color: #c9d1d9;
		font-family: var(--font-mono);
		font-size: 11px;
		line-height: 1.5;
		white-space: pre-wrap;
		word-break: break-word;
	}

	.tex-btn {
		background: #21262d;
		color: #c9d1d9;
		border: 1px solid #30363d;
	}
	.tex-btn:hover:not(:disabled) {
		background: #30363d;
	}
	.tex-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.tex-btn-primary {
		background: #238636;
		color: white;
		border: 1px solid transparent;
	}
	.tex-btn-primary:hover:not(:disabled) {
		background: #2ea043;
	}
	.tex-btn-primary:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.tex-err-icon {
		display: inline-flex;
		color: #f85149;
	}
</style>
