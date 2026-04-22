<script lang="ts">
	import { chats } from './stores.svelte';
	import Plus from 'lucide-svelte/icons/plus';
	import MessageSquare from 'lucide-svelte/icons/message-square';
	import Trash2 from 'lucide-svelte/icons/trash-2';
	import Pencil from 'lucide-svelte/icons/pencil';
	import Check from 'lucide-svelte/icons/check';
	import X from 'lucide-svelte/icons/x';
	import PanelLeft from 'lucide-svelte/icons/panel-left';

	type Props = {
		onNew: () => void;
		collapsed: boolean;
		onToggle: () => void;
	};
	let { onNew, collapsed, onToggle }: Props = $props();

	let renamingId = $state<string | null>(null);
	let renameValue = $state('');

	function startRename(id: string, current: string) {
		renamingId = id;
		renameValue = current;
	}
	function commitRename() {
		if (renamingId && renameValue.trim()) chats.rename(renamingId, renameValue.trim());
		renamingId = null;
	}
	function cancelRename() {
		renamingId = null;
	}

	function shortCwd(cwd: string) {
		const parts = cwd.split('/').filter(Boolean);
		return parts.slice(-2).join('/') || '/';
	}

	function time(ts: number) {
		const d = Date.now() - ts;
		if (d < 60_000) return 'now';
		if (d < 3_600_000) return `${Math.floor(d / 60_000)}m`;
		if (d < 86_400_000) return `${Math.floor(d / 3_600_000)}h`;
		return `${Math.floor(d / 86_400_000)}d`;
	}

	const sorted = $derived([...chats.chats].sort((a, b) => b.updatedAt - a.updatedAt));
</script>

<aside
	class="shrink-0 flex flex-col transition-all duration-200 overflow-hidden"
	style:width={collapsed ? '0px' : '260px'}
	style:border-right={collapsed ? 'none' : '1px solid var(--color-border)'}
	style:background="var(--color-surface)"
>
	<div
		class="h-12 shrink-0 flex items-center justify-between px-3 border-b"
		style:border-color="var(--color-border)"
	>
		<button
			class="size-8 rounded-md flex items-center justify-center"
			onclick={onToggle}
			title="Collapse sidebar"
			style:color="var(--color-text)"
		>
			<PanelLeft class="size-4 opacity-70" />
		</button>
		<button
			class="h-8 px-2.5 rounded-[8px] text-xs font-medium flex items-center gap-1.5"
			style:background="var(--color-accent)"
			style:color="white"
			onclick={onNew}
			title="New chat"
		>
			<Plus class="size-3.5" />
			New chat
		</button>
	</div>

	<div class="flex-1 overflow-y-auto py-1.5">
		{#if sorted.length === 0}
			<div class="text-xs opacity-60 px-3 py-3 text-center">
				No chats yet. Click "New chat" to start.
			</div>
		{/if}

		{#each sorted as c (c.id)}
			{@const active = c.id === chats.activeId}
			<div
				class="mx-1.5 mb-0.5 rounded-[8px] group relative"
				style:background={active ? 'var(--color-accent-soft)' : 'transparent'}
			>
				<button
					class="w-full text-left flex items-start gap-2 px-2.5 py-2 rounded-[8px]"
					onclick={() => chats.setActive(c.id)}
					style:color="var(--color-text)"
				>
					<span style:opacity={active ? '1' : '0.55'} class="shrink-0 mt-0.5">
						<MessageSquare class="size-3.5" />
					</span>
					<div class="flex-1 min-w-0">
						{#if renamingId === c.id}
							<input
								class="w-full text-xs font-medium outline-none bg-transparent"
								style:color="var(--color-text)"
								bind:value={renameValue}
								onclick={(e) => e.stopPropagation()}
								onkeydown={(e) => {
									if (e.key === 'Enter') {
										e.preventDefault();
										commitRename();
									} else if (e.key === 'Escape') {
										e.preventDefault();
										cancelRename();
									}
								}}
								autofocus
							/>
						{:else}
							<div class="text-xs font-medium truncate pr-1">{c.label}</div>
						{/if}
						<div class="flex items-center gap-1.5 mt-0.5">
							<span
								class="text-[10px] font-mono opacity-50 truncate"
								title={c.cwd}
							>
								{shortCwd(c.cwd)}
							</span>
							<span class="text-[10px] opacity-40">·</span>
							<span class="text-[10px] opacity-40 shrink-0">
								{c.messages.length} msg · {time(c.updatedAt)}
							</span>
						</div>
					</div>
				</button>

				<div
					class="absolute right-1.5 top-1.5 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity"
				>
					{#if renamingId === c.id}
						<button
							class="size-6 rounded flex items-center justify-center"
							style:background="var(--color-bg-elev)"
							onclick={commitRename}
							title="Save"
						>
							<Check class="size-3" />
						</button>
						<button
							class="size-6 rounded flex items-center justify-center"
							style:background="var(--color-bg-elev)"
							onclick={cancelRename}
							title="Cancel"
						>
							<X class="size-3" />
						</button>
					{:else}
						<button
							class="size-6 rounded flex items-center justify-center"
							style:background="var(--color-bg-elev)"
							onclick={(e) => {
								e.stopPropagation();
								startRename(c.id, c.label);
							}}
							title="Rename"
						>
							<Pencil class="size-3" />
						</button>
						<button
							class="size-6 rounded flex items-center justify-center"
							style:background="var(--color-bg-elev)"
							onclick={(e) => {
								e.stopPropagation();
								if (confirm(`Delete chat "${c.label}"? This also kills its terminal.`))
									chats.remove(c.id);
							}}
							title="Delete"
						>
							<Trash2 class="size-3" />
						</button>
					{/if}
				</div>
			</div>
		{/each}
	</div>

	<div
		class="shrink-0 px-3 py-2 text-[10px] opacity-50 border-t font-mono"
		style:border-color="var(--color-border)"
	>
		{chats.chats.length} chat{chats.chats.length !== 1 ? 's' : ''}
	</div>
</aside>
