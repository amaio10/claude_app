<script lang="ts">
	import Folder from 'lucide-svelte/icons/folder';
	import FolderOpen from 'lucide-svelte/icons/folder-open';
	import Home from 'lucide-svelte/icons/house';
	import ArrowUp from 'lucide-svelte/icons/arrow-up';
	import Check from 'lucide-svelte/icons/check';
	import ChevronDown from 'lucide-svelte/icons/chevron-down';
	import Eye from 'lucide-svelte/icons/eye';
	import EyeOff from 'lucide-svelte/icons/eye-off';
	import X from 'lucide-svelte/icons/x';
	import { chats } from './stores.svelte';

	type Entry = { name: string; path: string; is_dir: boolean };
	type ListResp = { path: string; parent: string | null; entries: Entry[] };
	type QuickEntry = { label: string; path: string };
	type HomeResp = { home: string; quick: QuickEntry[] };

	type Props = { recent: string[]; onSet: (path: string) => void };
	let { recent, onSet }: Props = $props();

	let open = $state(false);
	let browsing = $state<ListResp | null>(null);
	let typed = $state('');
	let showHidden = $state(false);
	let quick = $state<QuickEntry[]>([]);
	let loading = $state(false);
	let errMsg = $state('');

	$effect(() => {
		if (open && !browsing && !loading) loadHome();
	});

	async function loadHome() {
		try {
			const h: HomeResp = await fetch('/api/fs/home').then((r) => r.json());
			quick = h.quick;
			await navigate(h.home);
		} catch (e) {
			errMsg = 'Could not load home';
			console.error(e);
		}
	}

	async function navigate(path: string) {
		loading = true;
		errMsg = '';
		try {
			const r = await fetch(
				`/api/fs/list?path=${encodeURIComponent(path)}&show_hidden=${showHidden}`
			);
			if (!r.ok) {
				errMsg = await r.text();
				return;
			}
			browsing = await r.json();
			typed = browsing!.path;
		} catch (e) {
			errMsg = (e as Error).message;
		} finally {
			loading = false;
		}
	}

	async function goUp() {
		if (browsing?.parent) await navigate(browsing.parent);
	}

	function pick(p: string) {
		onSet(p);
		open = false;
	}

	function pickCurrent() {
		if (browsing) pick(browsing.path);
	}

	async function goToTyped() {
		const p = typed.trim();
		if (!p) return;
		await navigate(p);
	}

	async function toggleHidden() {
		showHidden = !showHidden;
		if (browsing) await navigate(browsing.path);
	}

	function short(p: string) {
		if (!p) return '—';
		const parts = p.split('/').filter(Boolean);
		if (parts.length <= 2) return p;
		return '/…/' + parts.slice(-2).join('/');
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (open && e.key === 'Escape') open = false;
	}}
	onmousedown={(e) => {
		if (!open) return;
		const t = e.target as Element | null;
		if (!t?.closest('[data-path-picker-root]')) open = false;
	}}
/>

<div class="relative" data-path-picker-root>
	<button
		data-path-trigger
		class="flex items-center gap-2 h-8 px-2.5 rounded-[8px] text-xs font-medium transition-colors"
		style:background="var(--color-surface)"
		style:border="1px solid var(--color-border)"
		style:color="var(--color-text)"
		onclick={() => (open = !open)}
	>
		<Folder class="size-3.5 opacity-70" />
		<span class="font-mono">{short((chats.active?.cwd ?? ''))}</span>
		<ChevronDown class="size-3 opacity-60" />
	</button>

	{#if open}
		<div
			class="absolute right-0 top-full mt-1.5 w-[520px] rounded-[12px] z-50 shadow-xl overflow-hidden flex flex-col"
			style:background="var(--color-bg-elev)"
			style:border="1px solid var(--color-border)"
			style:max-height="70vh"
		>
			<!-- breadcrumb + controls -->
			<div class="flex items-center gap-1.5 p-2 border-b" style:border-color="var(--color-border)">
				<button
					class="size-7 rounded-md flex items-center justify-center disabled:opacity-30"
					style:background="var(--color-surface)"
					onclick={goUp}
					disabled={!browsing?.parent}
					title="Go up"
				>
					<ArrowUp class="size-3.5" />
				</button>
				<button
					class="size-7 rounded-md flex items-center justify-center"
					style:background="var(--color-surface)"
					onclick={loadHome}
					title="Home"
				>
					<Home class="size-3.5" />
				</button>
				<input
					class="flex-1 text-xs font-mono rounded-md px-2 py-1.5 outline-none"
					style:background="var(--color-surface)"
					style:border="1px solid var(--color-border)"
					style:color="var(--color-text)"
					bind:value={typed}
					onkeydown={(e) => e.key === 'Enter' && goToTyped()}
					placeholder="/path or ~/relative"
				/>
				<button
					class="size-7 rounded-md flex items-center justify-center"
					style:background={showHidden ? 'var(--color-accent-soft)' : 'var(--color-surface)'}
					onclick={toggleHidden}
					title={showHidden ? 'Hide dotfiles' : 'Show dotfiles'}
				>
					{#if showHidden}
						<Eye class="size-3.5" />
					{:else}
						<EyeOff class="size-3.5" />
					{/if}
				</button>
				<button
					class="size-7 rounded-md flex items-center justify-center"
					style:background="var(--color-surface)"
					onclick={() => (open = false)}
					title="Close (Esc)"
				>
					<X class="size-3.5" />
				</button>
			</div>

			<div class="flex flex-1 min-h-0">
				<!-- sidebar: quick + recent -->
				<div
					class="w-[150px] shrink-0 overflow-y-auto py-1.5 border-r"
					style:border-color="var(--color-border)"
					style:background="var(--color-surface)"
				>
					<div class="text-[9px] uppercase tracking-wider px-2.5 pt-1.5 pb-1 opacity-60">
						Quick
					</div>
					{#each quick as q}
						<button
							class="w-full text-left flex items-center gap-1.5 px-2.5 py-1.5 text-xs hover:brightness-95"
							onclick={() => navigate(q.path)}
							style:color="var(--color-text)"
						>
							<Home class="size-3 opacity-60 shrink-0" />
							<span class="truncate">{q.label}</span>
						</button>
					{/each}
					{#if recent.length > 0}
						<div
							class="text-[9px] uppercase tracking-wider px-2.5 pt-3 pb-1 opacity-60"
						>
							Recent
						</div>
						{#each recent as p}
							<button
								class="w-full text-left flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-mono hover:brightness-95"
								onclick={() => navigate(p)}
								style:color="var(--color-text)"
								title={p}
							>
								<Folder class="size-3 opacity-60 shrink-0" />
								<span class="truncate">{p.split('/').filter(Boolean).slice(-1)[0] || '/'}</span>
							</button>
						{/each}
					{/if}
				</div>

				<!-- main list -->
				<div class="flex-1 overflow-y-auto p-1">
					{#if loading}
						<div class="text-xs opacity-60 p-3">Loading…</div>
					{:else if errMsg}
						<div class="text-xs p-3" style:color="#ef4444">{errMsg}</div>
					{:else if browsing && browsing.entries.length === 0}
						<div class="text-xs opacity-60 p-3">Empty folder.</div>
					{:else if browsing}
						{#each browsing.entries as e}
							<button
								class="w-full text-left flex items-center gap-2 px-2 py-1.5 rounded-md text-xs hover:brightness-95 hover:bg-surface"
								ondblclick={() => pick(e.path)}
								onclick={() => navigate(e.path)}
								style:color="var(--color-text)"
							>
								{#if e.path === (chats.active?.cwd ?? '')}
									<Check class="size-3" />
								{:else}
									<FolderOpen class="size-3 opacity-60" />
								{/if}
								<span class="truncate">{e.name}</span>
							</button>
						{/each}
					{/if}
				</div>
			</div>

			<!-- footer -->
			<div
				class="flex items-center justify-between px-2 py-1.5 border-t text-[11px]"
				style:border-color="var(--color-border)"
				style:background="var(--color-surface)"
			>
				<span class="font-mono opacity-70 truncate">{browsing?.path ?? '—'}</span>
				<button
					class="text-xs px-2.5 py-1 rounded-md font-medium"
					style:background="var(--color-accent)"
					style:color="white"
					onclick={pickCurrent}
					disabled={!browsing}
				>
					Select this folder
				</button>
			</div>
		</div>
	{/if}
</div>
