<script lang="ts">
	import { onDestroy } from 'svelte';
	import { chats } from './stores.svelte';

	type Props = { visible: boolean };
	let { visible }: Props = $props();

	let host: HTMLDivElement | null = $state(null);
	let term: any = null;
	let fitAddon: any = null;
	let ws: WebSocket | null = null;
	let currentPtyKey = $state<string | null>(null);
	let currentCwd = $state<string | null>(null);
	let ro: ResizeObserver | null = null;

	async function ensureTerm() {
		if (!host || term) return;
		const xtermMod = await import('@xterm/xterm');
		const fitMod = await import('@xterm/addon-fit');
		term = new xtermMod.Terminal({
			fontFamily: 'SF Mono, JetBrains Mono, Menlo, Monaco, Consolas, monospace',
			fontSize: 12.5,
			theme: {
				background: '#0d0d0e',
				foreground: '#e6e6e6',
				cursor: '#e6e6e6'
			},
			cursorBlink: true,
			convertEol: true
		});
		fitAddon = new fitMod.FitAddon();
		term.loadAddon(fitAddon);
		term.open(host);
		try {
			fitAddon.fit();
		} catch {}

		term.onData((d: string) => {
			ws?.send(JSON.stringify({ type: 'data', data: d }));
		});

		ro = new ResizeObserver(() => {
			try {
				fitAddon.fit();
				const { cols, rows } = term;
				ws?.send(JSON.stringify({ type: 'resize', cols, rows }));
			} catch {}
		});
		ro.observe(host);
	}

	async function attach(ptyKey: string, cwd: string) {
		await ensureTerm();
		if (!term) return;
		if (ws) {
			ws.onmessage = null;
			ws.onclose = null;
			ws.onerror = null;
			ws.close();
			ws = null;
		}
		term.reset();
		currentPtyKey = ptyKey;
		currentCwd = cwd;

		const proto = location.protocol === 'https:' ? 'wss' : 'ws';
		const qs = new URLSearchParams({
			session: ptyKey,
			cwd,
			cols: String(term.cols),
			rows: String(term.rows)
		});
		ws = new WebSocket(`${proto}://${location.host}/pty?${qs.toString()}`);
		console.log('[pty] attach', ptyKey, 'cwd=', cwd);

		ws.onopen = () => {
			const { cols, rows } = term;
			ws?.send(JSON.stringify({ type: 'resize', cols, rows }));
		};
		ws.onmessage = (e) => {
			term.write(e.data);
		};
		ws.onerror = (e) => console.error('[pty] ws error', e);
		ws.onclose = () => console.log('[pty] ws closed');
	}

	$effect(() => {
		const active = chats.active;
		if (!visible || !active) return;
		if (active.ptyKey !== currentPtyKey) {
			attach(active.ptyKey, active.cwd);
		}
	});

	onDestroy(() => {
		ws?.close();
		term?.dispose();
		ro?.disconnect();
	});
</script>

<div
	class="transition-all duration-200 overflow-hidden"
	style:height={visible ? '260px' : '0px'}
	style:border-top={visible ? '1px solid var(--color-border)' : 'none'}
	style:background="#0d0d0e"
>
	<div bind:this={host} class="h-full w-full px-3 pt-2"></div>
</div>
