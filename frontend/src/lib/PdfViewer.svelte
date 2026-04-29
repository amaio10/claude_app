<script lang="ts">
	import { onDestroy, onMount, tick } from 'svelte';
	import ZoomIn from 'lucide-svelte/icons/zoom-in';
	import ZoomOut from 'lucide-svelte/icons/zoom-out';
	import Maximize2 from 'lucide-svelte/icons/maximize-2';
	import FileText from 'lucide-svelte/icons/file-text';
	import ChevronLeft from 'lucide-svelte/icons/chevron-left';
	import ChevronRight from 'lucide-svelte/icons/chevron-right';

	type Props = { src: string };
	let { src }: Props = $props();

	let containerEl: HTMLDivElement | null = $state(null);
	let scrollEl: HTMLDivElement | null = $state(null);

	let loading = $state(true);
	let errMsg = $state('');
	let numPages = $state(0);
	let currentPage = $state(1);
	let scale = $state(1);
	let fitMode = $state<'width' | 'page' | 'custom'>('width');
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let pdfDoc: any = null;
	let renderedAt = 0;
	let resizeObs: ResizeObserver | null = null;

	const MIN_SCALE = 0.25;
	const MAX_SCALE = 5;

	async function loadPdfjs() {
		const pdfjs = await import('pdfjs-dist');
		const workerUrl = (await import('pdfjs-dist/build/pdf.worker.min.mjs?url')).default;
		pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;
		return pdfjs;
	}

	async function openDoc(url: string) {
		loading = true;
		errMsg = '';
		try {
			const pdfjs = await loadPdfjs();
			const task = pdfjs.getDocument({ url, disableRange: false, disableStream: false });
			const doc = await task.promise;
			if (pdfDoc) {
				try {
					pdfDoc.destroy();
				} catch {
					// ignore
				}
			}
			pdfDoc = doc;
			numPages = doc.numPages;
			if (currentPage > numPages) currentPage = numPages;
			if (currentPage < 1) currentPage = 1;
			await tick();
			await computeFitIfNeeded();
			await renderAllPages();
		} catch (e) {
			errMsg = (e as Error).message || 'Failed to load PDF';
		} finally {
			loading = false;
		}
	}

	async function computeFitIfNeeded() {
		if (!pdfDoc || !scrollEl) return;
		if (fitMode === 'custom') return;
		const page = await pdfDoc.getPage(1);
		const viewport = page.getViewport({ scale: 1 });
		const padding = 24; // leave room for margins/scrollbar
		const availW = scrollEl.clientWidth - padding;
		const availH = scrollEl.clientHeight - padding;
		if (fitMode === 'width') {
			scale = Math.max(MIN_SCALE, Math.min(MAX_SCALE, availW / viewport.width));
		} else {
			const sw = availW / viewport.width;
			const sh = availH / viewport.height;
			scale = Math.max(MIN_SCALE, Math.min(MAX_SCALE, Math.min(sw, sh)));
		}
	}

	async function renderAllPages() {
		if (!pdfDoc || !containerEl) return;
		const token = ++renderedAt;
		const dpr = Math.min(window.devicePixelRatio || 1, 2);
		// Clear old content
		containerEl.innerHTML = '';
		for (let i = 1; i <= numPages; i++) {
			if (token !== renderedAt) return;
			const page = await pdfDoc.getPage(i);
			const viewport = page.getViewport({ scale });
			const wrap = document.createElement('div');
			wrap.className = 'pdf-page-wrap';
			wrap.setAttribute('data-page', String(i));
			const canvas = document.createElement('canvas');
			canvas.width = Math.floor(viewport.width * dpr);
			canvas.height = Math.floor(viewport.height * dpr);
			canvas.style.width = `${Math.floor(viewport.width)}px`;
			canvas.style.height = `${Math.floor(viewport.height)}px`;
			const ctx = canvas.getContext('2d');
			if (!ctx) continue;
			wrap.appendChild(canvas);
			containerEl.appendChild(wrap);
			await page.render({
				canvas,
				canvasContext: ctx,
				viewport,
				transform: dpr !== 1 ? [dpr, 0, 0, dpr, 0, 0] : undefined
			}).promise;
		}
	}

	async function setScale(next: number) {
		fitMode = 'custom';
		scale = Math.max(MIN_SCALE, Math.min(MAX_SCALE, next));
		await renderAllPages();
	}

	async function zoomIn() {
		await setScale(scale * 1.2);
	}
	async function zoomOut() {
		await setScale(scale / 1.2);
	}
	async function fitWidth() {
		fitMode = 'width';
		await computeFitIfNeeded();
		await renderAllPages();
	}
	async function fitPage() {
		fitMode = 'page';
		await computeFitIfNeeded();
		await renderAllPages();
	}

	function scrollToPage(n: number) {
		if (!scrollEl || !containerEl) return;
		const el = containerEl.querySelector<HTMLDivElement>(`[data-page="${n}"]`);
		if (el) el.scrollIntoView({ block: 'start', behavior: 'smooth' });
	}

	function onKeydown(e: KeyboardEvent) {
		if (!scrollEl) return;
		const tgt = e.target as HTMLElement | null;
		if (tgt && (tgt.tagName === 'TEXTAREA' || tgt.tagName === 'INPUT')) return;
		if ((e.ctrlKey || e.metaKey) && (e.key === '+' || e.key === '=')) {
			e.preventDefault();
			void zoomIn();
		} else if ((e.ctrlKey || e.metaKey) && e.key === '-') {
			e.preventDefault();
			void zoomOut();
		} else if ((e.ctrlKey || e.metaKey) && e.key === '0') {
			e.preventDefault();
			void fitWidth();
		}
	}

	function onWheel(e: WheelEvent) {
		if (!e.ctrlKey && !e.metaKey) return;
		e.preventDefault();
		if (e.deltaY < 0) void zoomIn();
		else void zoomOut();
	}

	// Track current page by scroll position
	function onScroll() {
		if (!scrollEl || !containerEl || numPages === 0) return;
		const pages = containerEl.querySelectorAll<HTMLDivElement>('.pdf-page-wrap');
		const mid = scrollEl.scrollTop + scrollEl.clientHeight / 3;
		for (let i = 0; i < pages.length; i++) {
			const el = pages[i];
			if (el.offsetTop + el.offsetHeight > mid) {
				currentPage = i + 1;
				return;
			}
		}
		currentPage = numPages;
	}

	onMount(() => {
		window.addEventListener('keydown', onKeydown);
		if (scrollEl) {
			resizeObs = new ResizeObserver(() => {
				if (fitMode !== 'custom') {
					void computeFitIfNeeded().then(() => void renderAllPages());
				}
			});
			resizeObs.observe(scrollEl);
		}
	});

	onDestroy(() => {
		window.removeEventListener('keydown', onKeydown);
		resizeObs?.disconnect();
		if (pdfDoc) {
			try {
				pdfDoc.destroy();
			} catch {
				// ignore
			}
			pdfDoc = null;
		}
	});

	$effect(() => {
		const url = src;
		if (!url) return;
		currentPage = 1;
		void openDoc(url);
	});

	function onPageInput(e: Event) {
		const v = parseInt((e.currentTarget as HTMLInputElement).value, 10);
		if (isNaN(v)) return;
		const n = Math.max(1, Math.min(numPages, v));
		currentPage = n;
		scrollToPage(n);
	}
</script>

<div class="pdfv">
	<div class="pdfv-toolbar">
		<button
			class="pdfv-btn"
			onclick={zoomOut}
			disabled={loading || scale <= MIN_SCALE + 1e-3}
			title="Zoom out (Ctrl/Cmd -)"
		>
			<ZoomOut class="size-3.5" />
		</button>
		<button
			class="pdfv-btn pdfv-scale"
			onclick={() => void setScale(1)}
			disabled={loading}
			title="Reset to 100%"
		>
			{Math.round(scale * 100)}%
		</button>
		<button
			class="pdfv-btn"
			onclick={zoomIn}
			disabled={loading || scale >= MAX_SCALE - 1e-3}
			title="Zoom in (Ctrl/Cmd +)"
		>
			<ZoomIn class="size-3.5" />
		</button>
		<span class="pdfv-sep"></span>
		<button
			class="pdfv-btn"
			class:pdfv-btn-active={fitMode === 'width'}
			onclick={fitWidth}
			disabled={loading}
			title="Fit width (Ctrl/Cmd 0)"
		>
			<Maximize2 class="size-3.5 -rotate-45" />
		</button>
		<button
			class="pdfv-btn"
			class:pdfv-btn-active={fitMode === 'page'}
			onclick={fitPage}
			disabled={loading}
			title="Fit page"
		>
			<FileText class="size-3.5" />
		</button>
		{#if numPages > 1}
			<span class="pdfv-sep"></span>
			<button
				class="pdfv-btn"
				onclick={() => scrollToPage(Math.max(1, currentPage - 1))}
				disabled={loading || currentPage <= 1}
				title="Previous page"
			>
				<ChevronLeft class="size-3.5" />
			</button>
			<div class="pdfv-page">
				<input
					class="pdfv-page-input"
					type="number"
					min="1"
					max={numPages}
					value={currentPage}
					onchange={onPageInput}
				/>
				<span class="pdfv-page-total">/ {numPages}</span>
			</div>
			<button
				class="pdfv-btn"
				onclick={() => scrollToPage(Math.min(numPages, currentPage + 1))}
				disabled={loading || currentPage >= numPages}
				title="Next page"
			>
				<ChevronRight class="size-3.5" />
			</button>
		{/if}
	</div>

	<div
		bind:this={scrollEl}
		class="pdfv-scroll"
		onscroll={onScroll}
		onwheel={onWheel}
	>
		{#if errMsg}
			<div class="pdfv-state">Error: {errMsg}</div>
		{:else if loading}
			<div class="pdfv-state">Loading PDF…</div>
		{/if}
		<div bind:this={containerEl} class="pdfv-pages"></div>
	</div>
</div>

<style>
	.pdfv {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		background: #1b1f24;
		color: #c9d1d9;
	}
	.pdfv-toolbar {
		height: 32px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 0 8px;
		background: #0d1117;
		border-bottom: 1px solid #21262d;
	}
	.pdfv-sep {
		width: 1px;
		height: 18px;
		background: #21262d;
		margin: 0 4px;
	}
	.pdfv-btn {
		height: 22px;
		min-width: 26px;
		padding: 0 6px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 4px;
		border-radius: 4px;
		background: transparent;
		border: 0;
		color: #c9d1d9;
		font-size: 11px;
		cursor: pointer;
	}
	.pdfv-btn:hover:not(:disabled) {
		background: #21262d;
	}
	.pdfv-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.pdfv-btn-active {
		background: #21262d;
		color: #e1e4e8;
	}
	.pdfv-scale {
		min-width: 48px;
		font-variant-numeric: tabular-nums;
	}
	.pdfv-page {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 11px;
		color: #8b949e;
	}
	.pdfv-page-input {
		width: 42px;
		height: 22px;
		padding: 0 4px;
		background: #0d1117;
		border: 1px solid #30363d;
		border-radius: 4px;
		color: #c9d1d9;
		font-size: 11px;
		text-align: center;
		font-variant-numeric: tabular-nums;
		outline: none;
	}
	.pdfv-page-input:focus {
		border-color: #1f6feb;
	}
	.pdfv-page-total {
		color: #8b949e;
	}
	.pdfv-scroll {
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: 12px;
		background: #161b22;
		scrollbar-color: #30363d transparent;
	}
	.pdfv-state {
		padding: 16px;
		color: #8b949e;
		font-size: 12px;
	}
	.pdfv-pages {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
	}
	.pdfv-pages :global(.pdf-page-wrap) {
		background: white;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
	}
	.pdfv-pages :global(canvas) {
		display: block;
	}
</style>
