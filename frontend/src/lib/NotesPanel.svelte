<script lang="ts">
	import ArrowUp from 'lucide-svelte/icons/arrow-up';
	import Home from 'lucide-svelte/icons/house';
	import Folder from 'lucide-svelte/icons/folder';
	import FolderOpen from 'lucide-svelte/icons/folder-open';
	import FileText from 'lucide-svelte/icons/file-text';
	import Eye from 'lucide-svelte/icons/eye';
	import EyeOff from 'lucide-svelte/icons/eye-off';
	import X from 'lucide-svelte/icons/x';
	import { chats } from './stores.svelte';
	import { isViewable } from './filetypes';

	type Entry = { name: string; path: string; is_dir: boolean; size?: number; ext?: string };
	type ListResp = { path: string; parent: string | null; entries: Entry[] };

	type Props = {
		onSelect: (path: string) => void;
		onClose: () => void;
		activePath: string | null;
	};
	let { onSelect, onClose, activePath }: Props = $props();

	let dir = $state<ListResp | null>(null);
	let loading = $state(false);
	let errMsg = $state('');
	let showHidden = $state(false);

	let lastSyncedCwd = $state<string | null>(null);
	$effect(() => {
		const cwd = chats.active?.cwd || '~';
		if (cwd !== lastSyncedCwd) {
			lastSyncedCwd = cwd;
			navigate(cwd);
		}
	});

	async function navigate(path: string) {
		loading = true;
		errMsg = '';
		try {
			const r = await fetch(
				`/api/fs/list?path=${encodeURIComponent(path)}&show_hidden=${showHidden}&files=true`
			);
			if (!r.ok) {
				errMsg = await r.text();
				return;
			}
			dir = await r.json();
		} catch (e) {
			errMsg = (e as Error).message;
		} finally {
			loading = false;
		}
	}

	function clickEntry(e: Entry) {
		if (e.is_dir) {
			navigate(e.path);
		} else if (isViewable(e.is_dir, e.name, e.ext)) {
			onSelect(e.path);
		}
	}

	function goUp() {
		if (dir?.parent) navigate(dir.parent);
	}

	async function toggleHidden() {
		showHidden = !showHidden;
		if (dir) await navigate(dir.path);
	}

	function fmtSize(n?: number): string {
		if (n == null) return '';
		if (n < 1024) return `${n} B`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
		return `${(n / 1024 / 1024).toFixed(1)} MB`;
	}

</script>

<aside
	class="shrink-0 flex flex-col w-[280px]"
	style:background="var(--color-surface)"
	style:border-left="1px solid var(--color-border)"
>
	<div
		class="h-12 shrink-0 flex items-center justify-between px-3 border-b"
		style:border-color="var(--color-border)"
	>
		<span class="text-[10px] uppercase tracking-wider font-semibold opacity-70">Files</span>
		<button
			class="size-7 rounded-md flex items-center justify-center"
			style:background="var(--color-bg-elev)"
			onclick={onClose}
			title="Close notes panel"
		>
			<X class="size-3.5" />
		</button>
	</div>

	<div
		class="flex items-center gap-1.5 p-2 border-b"
		style:border-color="var(--color-border)"
	>
		<button
			class="size-7 rounded-md flex items-center justify-center disabled:opacity-30"
			style:background="var(--color-bg-elev)"
			onclick={goUp}
			disabled={!dir?.parent}
			title="Up"
		>
			<ArrowUp class="size-3.5" />
		</button>
		<button
			class="size-7 rounded-md flex items-center justify-center"
			style:background="var(--color-bg-elev)"
			onclick={() => navigate(chats.active?.cwd || '~')}
			title="Jump to active chat cwd"
		>
			<Home class="size-3.5" />
		</button>
		<button
			class="size-7 rounded-md flex items-center justify-center ml-auto"
			style:background={showHidden ? 'var(--color-accent-soft)' : 'var(--color-bg-elev)'}
			onclick={toggleHidden}
			title={showHidden ? 'Hide dotfiles' : 'Show dotfiles'}
		>
			{#if showHidden}<Eye class="size-3.5" />{:else}<EyeOff class="size-3.5" />{/if}
		</button>
	</div>

	<div class="px-3 py-1 text-[10px] font-mono opacity-55 truncate">
		{dir?.path ?? '—'}
	</div>

	<div class="flex-1 overflow-y-auto py-1">
		{#if loading}
			<div class="text-xs opacity-60 p-3">Loading…</div>
		{:else if errMsg}
			<div class="text-xs p-3" style:color="#ef4444">{errMsg}</div>
		{:else if dir && dir.entries.length === 0}
			<div class="text-xs opacity-60 p-3">Empty folder.</div>
		{:else if dir}
			{#each dir.entries as e}
				{@const viewable = isViewable(e.is_dir, e.name, e.ext)}
				{@const active = activePath === e.path}
				<button
					class="w-full text-left flex items-center gap-2 px-3 py-1.5 text-xs transition-colors"
					style:background={active ? 'var(--color-accent-soft)' : 'transparent'}
					style:color={viewable ? 'var(--color-text)' : 'var(--color-text-dim)'}
					disabled={!viewable}
					onclick={() => clickEntry(e)}
					title={e.path}
				>
					{#if e.is_dir}
						{#if dir.entries.length < 60}
							<FolderOpen class="size-3.5 opacity-65 shrink-0" />
						{:else}
							<Folder class="size-3.5 opacity-65 shrink-0" />
						{/if}
					{:else}
						<FileText class="size-3.5 opacity-65 shrink-0" />
					{/if}
					<span class="truncate flex-1">{e.name}</span>
					{#if !e.is_dir && e.size != null}
						<span class="text-[10px] opacity-45 font-mono shrink-0">{fmtSize(e.size)}</span>
					{/if}
				</button>
			{/each}
		{/if}
	</div>
</aside>
