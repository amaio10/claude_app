<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Crepe } from '@milkdown/crepe';
	import '@milkdown/crepe/theme/common/style.css';
	import '@milkdown/crepe/theme/frame.css';

	type SaveStatus = 'idle' | 'dirty' | 'saving' | 'saved' | 'error';

	export type EditorApi = { flush: () => Promise<void> };

	type Props = {
		path: string;
		initial: string;
		onStatus?: (status: SaveStatus, msg?: string) => void;
		onContentChange?: (md: string) => void;
		onReady?: (api: EditorApi) => void;
	};
	let { path, initial, onStatus, onContentChange, onReady }: Props = $props();

	let hostEl: HTMLDivElement;
	let crepe: Crepe | null = null;
	let saveTimer: ReturnType<typeof setTimeout> | null = null;
	let destroyed = false;
	let latest = initial;

	async function save() {
		if (destroyed) return;
		onStatus?.('saving');
		try {
			const r = await fetch('/api/fs/write', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ path, content: latest })
			});
			if (!r.ok) {
				const msg = await r.text();
				onStatus?.('error', msg);
				return;
			}
			onStatus?.('saved');
		} catch (e) {
			onStatus?.('error', (e as Error).message);
		}
	}

	function scheduleSave() {
		if (saveTimer) clearTimeout(saveTimer);
		onStatus?.('dirty');
		saveTimer = setTimeout(save, 700);
	}

	onMount(async () => {
		crepe = new Crepe({
			root: hostEl,
			defaultValue: initial
		});
		crepe.on((listener) => {
			listener.markdownUpdated((_ctx, markdown, prev) => {
				if (prev === undefined) return; // initial set
				if (markdown === latest) return;
				latest = markdown;
				onContentChange?.(markdown);
				scheduleSave();
			});
		});
		await crepe.create();
		onReady?.({ flush });
	});

	onDestroy(() => {
		destroyed = true;
		if (saveTimer) {
			clearTimeout(saveTimer);
			// flush pending changes on unmount
			void save();
		}
		void crepe?.destroy();
	});

	function flush() {
		if (saveTimer) {
			clearTimeout(saveTimer);
			saveTimer = null;
			return save();
		}
		// No pending change — issue a save of current content anyway for explicit Save button.
		return save();
	}
</script>

<div bind:this={hostEl} class="md-editor-host"></div>

<style>
	.md-editor-host {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		background: var(--color-bg);
		color: var(--color-text);
	}
	/* Map Crepe's frame tokens to the app theme so the editor follows light/dark. */
	.md-editor-host :global(.milkdown) {
		min-height: 100%;
		background: var(--color-bg);
		--crepe-color-background: var(--color-bg);
		--crepe-color-on-background: var(--color-text);
		--crepe-color-surface: var(--color-surface);
		--crepe-color-surface-low: var(--color-bg-elev);
		--crepe-color-on-surface: var(--color-text);
		--crepe-color-on-surface-variant: var(--color-text-dim);
		--crepe-color-outline: var(--color-border);
		--crepe-color-primary: var(--color-accent);
		--crepe-color-secondary: var(--color-surface);
		--crepe-color-on-secondary: var(--color-text);
		--crepe-color-inverse: var(--color-bg-elev);
		--crepe-color-on-inverse: var(--color-text);
		--crepe-color-hover: color-mix(in oklab, var(--color-text) 8%, transparent);
		--crepe-color-selected: color-mix(in oklab, var(--color-accent) 22%, transparent);
		--crepe-color-inline-area: color-mix(in oklab, var(--color-text) 14%, transparent);
		--crepe-color-inline-code: var(--color-accent);
		--crepe-font-title: var(--font-prose, var(--font-sans));
		--crepe-font-default: var(--font-sans);
		--crepe-font-code: var(--font-mono);
	}
	.md-editor-host :global(.ProseMirror) {
		padding: 32px 48px 120px 48px;
		outline: none;
		font-family: var(--font-body, var(--font-sans, system-ui));
		font-size: 15px;
		line-height: 1.7;
		color: var(--color-text);
		max-width: 860px;
		margin: 0 auto;
	}
	.md-editor-host :global(.ProseMirror h1),
	.md-editor-host :global(.ProseMirror h2),
	.md-editor-host :global(.ProseMirror h3),
	.md-editor-host :global(.ProseMirror h4) {
		color: var(--color-text);
	}
	.md-editor-host :global(.ProseMirror table) {
		border-collapse: collapse;
		width: 100%;
		margin: 1em 0;
		font-size: 13px;
	}
	.md-editor-host :global(.ProseMirror th),
	.md-editor-host :global(.ProseMirror td) {
		border: 1px solid var(--color-border);
		padding: 8px 10px;
		vertical-align: top;
	}
	.md-editor-host :global(.ProseMirror th) {
		background: var(--color-bg-elev);
		font-weight: 600;
	}
	.md-editor-host :global(.ProseMirror blockquote) {
		border-left: 3px solid var(--color-accent, #58a6ff);
		padding-left: 14px;
		color: var(--color-text-dim);
		margin: 1em 0;
	}
	.md-editor-host :global(.ProseMirror code) {
		background: var(--color-surface);
		padding: 1px 5px;
		border-radius: 3px;
		font-family: var(--font-mono);
		font-size: 0.9em;
	}
	.md-editor-host :global(.ProseMirror pre) {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 6px;
		padding: 12px 14px;
	}
	.md-editor-host :global(.ProseMirror pre code) {
		background: transparent;
		padding: 0;
	}
</style>
