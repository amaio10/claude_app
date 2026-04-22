<script lang="ts">
	import Table from 'lucide-svelte/icons/table';
	import Code from 'lucide-svelte/icons/code';
	import RefreshCw from 'lucide-svelte/icons/refresh-cw';

	type Props = { path: string };
	let { path }: Props = $props();

	let rows = $state<string[][]>([]);
	let rawText = $state('');
	let truncated = $state(false);
	let size = $state(0);
	let mode = $state<'table' | 'source'>('table');
	let delimiter = $state<',' | ';' | '\t' | '|'>(',');
	let delimDetected = $state<',' | ';' | '\t' | '|'>(',');
	let errMsg = $state('');
	let loading = $state(true);

	$effect(() => {
		void path;
		void load();
	});

	async function load() {
		loading = true;
		errMsg = '';
		try {
			const r = await fetch(`/api/fs/read?path=${encodeURIComponent(path)}`);
			if (!r.ok) {
				errMsg = await r.text();
				return;
			}
			const d = await r.json();
			rawText = d.content;
			truncated = d.truncated;
			size = d.size;
			delimDetected = detectDelim(rawText);
			delimiter = delimDetected;
			rows = parseCsv(rawText, delimiter);
		} catch (e) {
			errMsg = (e as Error).message;
		} finally {
			loading = false;
		}
	}

	function reparse(d: ',' | ';' | '\t' | '|') {
		delimiter = d;
		rows = parseCsv(rawText, d);
	}

	function detectDelim(s: string): ',' | ';' | '\t' | '|' {
		const sample = s.split(/\r?\n/).slice(0, 8).join('\n');
		const counts: Record<string, number> = {
			',': (sample.match(/,/g) || []).length,
			';': (sample.match(/;/g) || []).length,
			'\t': (sample.match(/\t/g) || []).length,
			'|': (sample.match(/\|/g) || []).length
		};
		let best: ',' | ';' | '\t' | '|' = ',';
		let bestN = counts[','];
		for (const k of [';', '\t', '|'] as const) {
			if (counts[k] > bestN) {
				bestN = counts[k];
				best = k;
			}
		}
		return best;
	}

	function parseCsv(text: string, delim: string): string[][] {
		const out: string[][] = [];
		let row: string[] = [];
		let cur = '';
		let inQ = false;
		for (let i = 0; i < text.length; i++) {
			const ch = text[i];
			if (inQ) {
				if (ch === '"') {
					if (text[i + 1] === '"') {
						cur += '"';
						i++;
						continue;
					}
					inQ = false;
					continue;
				}
				cur += ch;
				continue;
			}
			if (ch === '"') {
				inQ = true;
				continue;
			}
			if (ch === delim) {
				row.push(cur);
				cur = '';
				continue;
			}
			if (ch === '\n' || ch === '\r') {
				row.push(cur);
				out.push(row);
				row = [];
				cur = '';
				if (ch === '\r' && text[i + 1] === '\n') i++;
				continue;
			}
			cur += ch;
		}
		if (cur.length > 0 || row.length > 0) {
			row.push(cur);
			out.push(row);
		}
		// drop trailing empty row from trailing newline
		if (out.length && out[out.length - 1].length === 1 && out[out.length - 1][0] === '') {
			out.pop();
		}
		return out;
	}

	function fmtBytes(n: number) {
		if (n < 1024) return `${n} B`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
		return `${(n / 1024 / 1024).toFixed(1)} MB`;
	}

	function labelDelim(d: string) {
		if (d === '\t') return 'tab';
		return d;
	}

	const header = $derived(rows[0] ?? []);
	const body = $derived(rows.slice(1));
</script>

<div class="csv-root">
	<div class="csv-toolbar">
		<button
			class="csv-btn"
			class:active={mode === 'table'}
			onclick={() => (mode = 'table')}
			title="Table view"
		>
			<Table class="size-3.5" />
			<span>Table</span>
		</button>
		<button
			class="csv-btn"
			class:active={mode === 'source'}
			onclick={() => (mode = 'source')}
			title="Raw source"
		>
			<Code class="size-3.5" />
			<span>Source</span>
		</button>
		<span class="csv-sep"></span>
		<label class="csv-delim" title="Delimiter">
			<span class="csv-delim-label">Delim</span>
			<select
				value={delimiter}
				onchange={(e) => reparse((e.currentTarget.value || ',') as ',' | ';' | '\t' | '|')}
			>
				<option value=",">, (comma)</option>
				<option value=";">; (semicolon)</option>
				<option value={'\t'}>⇥ (tab)</option>
				<option value="|">| (pipe)</option>
			</select>
		</label>
		<button class="csv-btn" onclick={load} title="Reload" style="margin-left:auto">
			<RefreshCw class="size-3.5" />
		</button>
	</div>

	<div class="csv-pane">
		{#if loading}
			<div class="csv-msg">Loading…</div>
		{:else if errMsg}
			<div class="csv-msg csv-err">{errMsg}</div>
		{:else if mode === 'source'}
			<pre class="csv-source">{rawText}</pre>
		{:else if rows.length === 0}
			<div class="csv-msg">Empty file</div>
		{:else}
			<div class="csv-scroll">
				<table>
					<thead>
						<tr>
							<th class="rownum"></th>
							{#each header as h, i}
								<th title={h}>{h || `col ${i + 1}`}</th>
							{/each}
						</tr>
					</thead>
					<tbody>
						{#each body as r, ri}
							<tr>
								<td class="rownum">{ri + 1}</td>
								{#each header as _, ci}
									<td title={r[ci] ?? ''}>{r[ci] ?? ''}</td>
								{/each}
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	</div>

	<div class="csv-status">
		{#if rows.length}
			<span class="font-mono">{body.length.toLocaleString()} rows</span>
			<span class="sep">·</span>
			<span class="font-mono">{header.length} cols</span>
			<span class="sep">·</span>
		{/if}
		<span class="font-mono">delim: {labelDelim(delimiter)}</span>
		<span class="sep">·</span>
		<span class="font-mono">{fmtBytes(size)}</span>
		{#if truncated}
			<span class="sep">·</span>
			<span class="csv-trunc">truncated</span>
		{/if}
		<span class="ml-auto font-mono truncate opacity-70" title={path}>{path}</span>
	</div>
</div>

<style>
	.csv-root {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		background: var(--color-bg);
		color: var(--color-text);
	}
	.csv-toolbar {
		height: 36px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 0 10px;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-bg-elev);
	}
	.csv-btn {
		height: 26px;
		padding: 0 8px;
		display: inline-flex;
		align-items: center;
		gap: 5px;
		border-radius: 5px;
		font-size: 11px;
		font-weight: 500;
		color: var(--color-text);
		background: transparent;
	}
	.csv-btn:hover {
		background: var(--color-surface);
	}
	.csv-btn.active {
		background: var(--color-accent-soft);
	}
	.csv-sep {
		width: 1px;
		height: 18px;
		background: var(--color-border);
		margin: 0 6px;
	}
	.csv-delim {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		color: var(--color-text-dim);
	}
	.csv-delim select {
		height: 24px;
		padding: 0 6px;
		border-radius: 5px;
		border: 1px solid var(--color-border);
		background: var(--color-bg);
		color: var(--color-text);
		font-size: 11px;
		font-family: var(--font-mono);
	}
	.csv-pane {
		flex: 1;
		min-height: 0;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}
	.csv-msg {
		padding: 24px;
		font-size: 13px;
		color: var(--color-text-dim);
	}
	.csv-err {
		color: #ef4444;
	}
	.csv-source {
		flex: 1;
		margin: 0;
		padding: 16px 20px;
		overflow: auto;
		font-family: var(--font-mono);
		font-size: 12px;
		line-height: 1.5;
		white-space: pre;
		color: var(--color-text);
	}
	.csv-scroll {
		flex: 1;
		overflow: auto;
	}
	table {
		border-collapse: separate;
		border-spacing: 0;
		font-size: 12px;
		font-family: var(--font-mono);
		width: max-content;
		min-width: 100%;
	}
	thead th {
		position: sticky;
		top: 0;
		background: var(--color-bg-elev);
		z-index: 2;
		text-align: left;
		font-weight: 600;
		padding: 8px 12px;
		border-bottom: 1px solid var(--color-border);
		border-right: 1px solid var(--color-border);
		white-space: nowrap;
		color: var(--color-text);
	}
	tbody td {
		padding: 6px 12px;
		border-bottom: 1px solid var(--color-border);
		border-right: 1px solid var(--color-border);
		white-space: nowrap;
		max-width: 420px;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	tbody tr:nth-child(even) td {
		background: color-mix(in oklab, var(--color-text) 3%, transparent);
	}
	tbody tr:hover td {
		background: color-mix(in oklab, var(--color-accent) 10%, transparent);
	}
	.rownum {
		position: sticky;
		left: 0;
		background: var(--color-bg-elev);
		color: var(--color-text-dim);
		text-align: right;
		font-variant-numeric: tabular-nums;
		min-width: 40px;
		padding: 6px 10px;
		border-right: 1px solid var(--color-border);
		z-index: 1;
	}
	thead .rownum {
		z-index: 3;
	}
	.csv-status {
		height: 24px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 14px;
		font-size: 11px;
		background: var(--color-bg-elev);
		border-top: 1px solid var(--color-border);
		color: var(--color-text-dim);
	}
	.csv-status .sep {
		opacity: 0.5;
	}
	.csv-trunc {
		color: #b45309;
	}
</style>
