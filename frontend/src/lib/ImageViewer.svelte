<script lang="ts">
	import Maximize from 'lucide-svelte/icons/maximize';
	import Minimize from 'lucide-svelte/icons/minimize';
	import ExternalLink from 'lucide-svelte/icons/external-link';

	type Props = { path: string };
	let { path }: Props = $props();

	let fit = $state(true);
	let dims = $state<{ w: number; h: number } | null>(null);
	let size = $state(0);
	let errMsg = $state('');
	let loaded = $state(false);

	const src = $derived(`/api/fs/raw?path=${encodeURIComponent(path)}`);

	$effect(() => {
		// reset when path changes
		void path;
		dims = null;
		loaded = false;
		errMsg = '';
		fit = true;
		void fetchSize();
	});

	async function fetchSize() {
		try {
			const r = await fetch(src, { method: 'HEAD' });
			if (r.ok) {
				const len = r.headers.get('content-length');
				if (len) size = parseInt(len, 10);
			}
		} catch {
			// non-fatal
		}
	}

	function fmtBytes(n: number) {
		if (n < 1024) return `${n} B`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
		return `${(n / 1024 / 1024).toFixed(1)} MB`;
	}
</script>

<div class="img-root">
	<div
		class="img-stage"
		class:fit
		class:actual={!fit}
		onclick={() => (fit = !fit)}
		onkeydown={(e) => {
			if (e.key === ' ' || e.key === 'Enter') {
				e.preventDefault();
				fit = !fit;
			}
		}}
		role="button"
		tabindex="0"
		title={fit ? 'Click to view actual size' : 'Click to fit'}
	>
		{#if errMsg}
			<div class="img-err">{errMsg}</div>
		{:else}
			<img
				{src}
				alt={path}
				onload={(e) => {
					const el = e.currentTarget;
					dims = { w: el.naturalWidth, h: el.naturalHeight };
					loaded = true;
				}}
				onerror={() => (errMsg = 'Failed to load image')}
			/>
		{/if}
	</div>

	<div class="img-toolbar">
		<button class="img-btn" onclick={() => (fit = !fit)} title={fit ? 'Actual size' : 'Fit'}>
			{#if fit}
				<Maximize class="size-3.5" />
			{:else}
				<Minimize class="size-3.5" />
			{/if}
		</button>
		<a class="img-btn" href={src} target="_blank" rel="noopener" title="Open in new tab">
			<ExternalLink class="size-3.5" />
		</a>
	</div>

	<div class="img-status">
		{#if dims}
			<span class="font-mono">{dims.w} × {dims.h}</span>
			<span class="sep">·</span>
		{/if}
		{#if size > 0}
			<span class="font-mono">{fmtBytes(size)}</span>
			<span class="sep">·</span>
		{/if}
		<span class="font-mono">{fit ? 'fit' : '1:1'}</span>
		<span class="ml-auto font-mono truncate opacity-70" title={path}>{path}</span>
	</div>
</div>

<style>
	.img-root {
		position: relative;
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		background: #1b1f24;
	}
	.img-stage {
		flex: 1;
		min-height: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: zoom-in;
		outline: none;
		/* checkerboard */
		background-image:
			linear-gradient(45deg, rgba(255, 255, 255, 0.04) 25%, transparent 25%),
			linear-gradient(-45deg, rgba(255, 255, 255, 0.04) 25%, transparent 25%),
			linear-gradient(45deg, transparent 75%, rgba(255, 255, 255, 0.04) 75%),
			linear-gradient(-45deg, transparent 75%, rgba(255, 255, 255, 0.04) 75%);
		background-size: 20px 20px;
		background-position:
			0 0,
			0 10px,
			10px -10px,
			-10px 0;
	}
	.img-stage.actual {
		cursor: zoom-out;
		overflow: auto;
		align-items: flex-start;
		justify-content: flex-start;
	}
	.img-stage.fit img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
		display: block;
	}
	.img-stage.actual img {
		display: block;
		max-width: none;
		max-height: none;
		margin: 20px;
	}
	.img-err {
		color: #f85149;
		font-size: 12px;
	}
	.img-toolbar {
		position: absolute;
		top: 10px;
		right: 10px;
		display: flex;
		gap: 4px;
		background: rgba(22, 27, 34, 0.88);
		backdrop-filter: blur(8px);
		padding: 4px;
		border-radius: 6px;
		border: 1px solid rgba(255, 255, 255, 0.08);
	}
	.img-btn {
		width: 28px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
		background: transparent;
		color: #c9d1d9;
		cursor: pointer;
	}
	.img-btn:hover {
		background: #21262d;
	}
	.img-status {
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
	.img-status .sep {
		color: #30363d;
	}
</style>
