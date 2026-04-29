<script lang="ts">
	import { onDestroy } from 'svelte';

	type SaveStatus = 'idle' | 'dirty' | 'saving' | 'saved' | 'error';

	export type TexEditorApi = { flush: () => Promise<void>; getContent: () => string };

	type Props = {
		path: string;
		initial: string;
		onStatus?: (status: SaveStatus, msg?: string) => void;
		onContentChange?: (content: string) => void;
		onReady?: (api: TexEditorApi) => void;
	};
	let { path, initial, onStatus, onContentChange, onReady }: Props = $props();

	let value = $state(initial);
	let taEl: HTMLTextAreaElement | null = $state(null);
	let saveTimer: ReturnType<typeof setTimeout> | null = null;
	let destroyed = false;

	async function save() {
		if (destroyed) return;
		onStatus?.('saving');
		try {
			const r = await fetch('/api/fs/write', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ path, content: value })
			});
			if (!r.ok) {
				onStatus?.('error', await r.text());
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

	function onInput(e: Event) {
		const v = (e.currentTarget as HTMLTextAreaElement).value;
		if (v === value) return;
		value = v;
		onContentChange?.(v);
		scheduleSave();
	}

	function flush() {
		if (saveTimer) {
			clearTimeout(saveTimer);
			saveTimer = null;
		}
		return save();
	}

	$effect(() => {
		onReady?.({ flush, getContent: () => value });
	});

	onDestroy(() => {
		destroyed = true;
		if (saveTimer) {
			clearTimeout(saveTimer);
			void save();
		}
	});

	// Tab key → insert 2 spaces instead of losing focus.
	function onKeydown(e: KeyboardEvent) {
		if (e.key !== 'Tab') return;
		e.preventDefault();
		const ta = e.currentTarget as HTMLTextAreaElement;
		const start = ta.selectionStart;
		const end = ta.selectionEnd;
		const indent = '  ';
		const next = value.slice(0, start) + indent + value.slice(end);
		value = next;
		onContentChange?.(next);
		scheduleSave();
		queueMicrotask(() => {
			ta.selectionStart = ta.selectionEnd = start + indent.length;
		});
	}
</script>

<textarea
	bind:this={taEl}
	class="tex-ta"
	spellcheck="false"
	autocomplete="off"
	autocapitalize="off"
	wrap="off"
	oninput={onInput}
	onkeydown={onKeydown}
	value={value}
></textarea>

<style>
	.tex-ta {
		flex: 1;
		min-height: 0;
		width: 100%;
		resize: none;
		border: 0;
		outline: none;
		padding: 14px 18px;
		font-family: var(--font-mono);
		font-feature-settings: 'calt' 0, 'liga' 0;
		font-variant-ligatures: none;
		font-size: 13px;
		line-height: 19px;
		tab-size: 2;
		background: #24292e;
		color: #e1e4e8;
		scrollbar-color: #30363d transparent;
	}
	.tex-ta::selection {
		background: rgba(88, 166, 255, 0.35);
	}
</style>
