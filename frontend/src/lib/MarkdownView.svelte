<script lang="ts">
	import { renderMarkdown } from './markdown';

	type Props = {
		source: string;
		streaming?: boolean;
	};
	let { source, streaming = false }: Props = $props();

	let html = $state('');
	let lastRendered = '';

	$effect(() => {
		const src = source;
		if (!src) {
			html = '';
			lastRendered = '';
			return;
		}
		if (streaming && src.length === lastRendered.length) return;
		lastRendered = src;
		renderMarkdown(src).then((out) => {
			html = out;
		});
	});
</script>

<div class="md-body">
	{@html html || (streaming ? '<span class="md-skel"></span>' : '')}
</div>
