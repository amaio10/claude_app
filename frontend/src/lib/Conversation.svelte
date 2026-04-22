<script lang="ts">
	import { chats } from './stores.svelte';
	import { tick } from 'svelte';
	import MarkdownView from './MarkdownView.svelte';

	let scrollEl: HTMLDivElement | null = $state(null);

	const active = $derived(chats.active);

	$effect(() => {
		void active?.messages.length;
		void active?.messages.at(-1)?.text;
		tick().then(() => {
			if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
		});
	});
</script>

<div
	bind:this={scrollEl}
	class="flex-1 overflow-y-auto px-8 py-6"
	style:background="var(--color-bg)"
>
	<div class="mx-auto max-w-3xl space-y-5">
		{#if !active}
			<div class="flex flex-col items-center justify-center text-center pt-24 opacity-70">
				<h2 class="text-2xl font-semibold tracking-tight mb-2" style:color="var(--color-text)">
					Welcome
				</h2>
				<p class="text-sm max-w-md" style:color="var(--color-text-dim)">
					Create a new chat from the sidebar to start a conversation in a folder of your choice.
				</p>
			</div>
		{:else if active.messages.length === 0}
			<div class="flex flex-col items-center justify-center text-center pt-24 opacity-70">
				<h2 class="text-2xl font-semibold tracking-tight mb-2" style:color="var(--color-text)">
					{active.label}
				</h2>
				<p class="text-sm font-mono max-w-md" style:color="var(--color-text-dim)">
					{active.cwd}
				</p>
				<p class="text-xs max-w-md mt-4" style:color="var(--color-text-dim)">
					Voice or text. Type / for commands.
				</p>
			</div>
		{:else}
			{#each active.messages as m (m.id)}
				<div class="flex gap-3" class:flex-row-reverse={m.role === 'user'}>
					<div
						class="size-7 shrink-0 rounded-full flex items-center justify-center text-[11px] font-semibold tracking-tight"
						style:background={m.role === 'user'
							? 'var(--color-accent)'
							: 'var(--color-surface)'}
						style:color={m.role === 'user' ? 'white' : 'var(--color-text)'}
						style:border="1px solid var(--color-border)"
					>
						{m.role === 'user' ? 'You' : 'C'}
					</div>
					<div
						class="max-w-[85%] rounded-[12px] px-4 py-3 text-[14px] leading-relaxed"
						class:whitespace-pre-wrap={m.role === 'user'}
						style:background={m.role === 'user'
							? 'var(--color-accent-soft)'
							: 'var(--color-bg-elev)'}
						style:border="1px solid var(--color-border)"
						style:color="var(--color-text)"
					>
						{#if m.role === 'user'}
							{#if m.text}
								{m.text}
							{/if}
						{:else if m.text}
							<MarkdownView source={m.text} streaming={m.streaming} />
						{:else if m.streaming}
							<span class="inline-flex gap-1 opacity-60">
								<span class="size-1.5 rounded-full bg-current animate-pulse"></span>
								<span
									class="size-1.5 rounded-full bg-current animate-pulse"
									style:animation-delay="150ms"
								></span>
								<span
									class="size-1.5 rounded-full bg-current animate-pulse"
									style:animation-delay="300ms"
								></span>
							</span>
						{/if}
					</div>
				</div>
			{/each}
		{/if}
	</div>
</div>
