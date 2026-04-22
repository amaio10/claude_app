<script lang="ts">
	import type { Cmd } from './commands';

	type Props = {
		items: Cmd[];
		activeIndex: number;
		onPick: (c: Cmd) => void;
	};
	let { items, activeIndex, onPick }: Props = $props();
</script>

{#if items.length > 0}
	<div
		class="absolute bottom-full left-0 right-0 mb-2 rounded-[12px] overflow-hidden shadow-xl"
		style:background="var(--color-bg-elev)"
		style:border="1px solid var(--color-border)"
	>
		<div class="text-[9px] uppercase tracking-wider px-3 pt-2 pb-1 opacity-60">
			Slash commands
		</div>
		<div class="pb-1 max-h-[280px] overflow-y-auto">
			{#each items as c, i}
				<button
					class="w-full flex items-start gap-3 px-3 py-2 text-left transition-colors"
					style:background={i === activeIndex ? 'var(--color-accent-soft)' : 'transparent'}
					style:color="var(--color-text)"
					onclick={() => onPick(c)}
					onmouseenter={() => (activeIndex = i)}
				>
					<span
						class="text-[11px] font-mono font-semibold pt-0.5"
						style:color={c.kind === 'client' ? 'var(--color-accent)' : 'var(--color-text)'}
					>
						{c.trigger}
					</span>
					<span class="flex-1 min-w-0">
						<span class="text-xs font-medium block">{c.label}</span>
						<span class="text-[11px] opacity-60 block truncate">{c.description}</span>
					</span>
					<span
						class="text-[9px] uppercase tracking-wider shrink-0 opacity-50 pt-1"
					>
						{c.kind}
					</span>
				</button>
			{/each}
		</div>
		<div
			class="flex items-center gap-3 px-3 py-1.5 text-[10px] font-mono opacity-50 border-t"
			style:border-color="var(--color-border)"
		>
			<span>↑↓ navigate</span>
			<span>Tab/Enter select</span>
			<span>Esc close</span>
		</div>
	</div>
{/if}
