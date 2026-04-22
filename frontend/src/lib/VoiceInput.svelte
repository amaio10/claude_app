<script lang="ts">
	import Mic from 'lucide-svelte/icons/mic';
	import Square from 'lucide-svelte/icons/square';
	import Loader2 from 'lucide-svelte/icons/loader-2';

	type Props = {
		onTranscribed: (text: string) => void;
		disabled?: boolean;
	};
	let { onTranscribed, disabled = false }: Props = $props();

	let recState = $state<'idle' | 'recording' | 'transcribing'>('idle');
	let recorder: MediaRecorder | null = null;
	let chunks: BlobPart[] = [];
	let audioCtx: AudioContext | null = null;
	let analyser: AnalyserNode | null = null;
	let rafId = 0;
	let levels = $state<number[]>(Array(28).fill(0.05));
	let sourceStream: MediaStream | null = null;

	async function start() {
		if (recState !== 'idle' || disabled) return;
		try {
			const stream = await navigator.mediaDevices.getUserMedia({
				audio: {
					echoCancellation: true,
					noiseSuppression: true,
					autoGainControl: true
				}
			});
			sourceStream = stream;
			chunks = [];
			const mimeCandidates = [
				'audio/webm;codecs=opus',
				'audio/webm',
				'audio/mp4',
				'audio/ogg;codecs=opus'
			];
			const mime = mimeCandidates.find((m) => MediaRecorder.isTypeSupported(m)) || '';
			recorder = new MediaRecorder(stream, mime ? { mimeType: mime } : undefined);
			recorder.ondataavailable = (e) => {
				if (e.data.size > 0) chunks.push(e.data);
			};
			recorder.onstop = async () => {
				const blob = new Blob(chunks, { type: recorder?.mimeType || 'audio/webm' });
				stopMeter();
				await send(blob);
			};
			recorder.start(250);
			startMeter(stream);
			recState = 'recording';
			console.log('[voice] recording with mime', recorder.mimeType);
		} catch (e) {
			const err = e as DOMException;
			console.error('[voice] getUserMedia failed', err.name, err.message, err);
			const hints: Record<string, string> = {
				NotAllowedError: 'Permission refused. Click the 🔒 in the URL bar → Microphone → Allow, then reload.',
				NotFoundError: 'No microphone detected. Plug one in and reload.',
				NotReadableError: 'Mic is in use by another app. Close it and retry.',
				SecurityError: 'Insecure origin. Use http://127.0.0.1:5173 or http://localhost:5173.',
				AbortError: 'Mic access aborted.',
				OverconstrainedError: 'Mic does not meet audio constraints.'
			};
			const hint = hints[err.name] ?? err.message ?? 'Unknown error';
			alert(`Mic error (${err.name}): ${hint}`);
		}
	}

	function stop() {
		if (recState !== 'recording') return;
		recorder?.stop();
		sourceStream?.getTracks().forEach((t) => t.stop());
		recState = 'transcribing';
	}

	async function send(blob: Blob) {
		const ext = (blob.type.split('/')[1] || 'webm').split(';')[0];
		const form = new FormData();
		form.append('audio', blob, `rec.${ext}`);
		try {
			const t0 = performance.now();
			const r = await fetch('/api/transcribe', { method: 'POST', body: form });
			const ms = Math.round(performance.now() - t0);
			if (!r.ok) {
				const err = await r.text();
				throw new Error(`${r.status} ${err}`);
			}
			const data = await r.json();
			console.log(`[voice] transcribed in ${ms}ms:`, data.text);
			onTranscribed(data.text);
		} catch (e) {
			console.error('[voice] transcribe failed', e);
			alert(`Transcription failed: ${(e as Error).message}`);
		} finally {
			recState = 'idle';
		}
	}

	function startMeter(stream: MediaStream) {
		audioCtx = new AudioContext();
		analyser = audioCtx.createAnalyser();
		analyser.fftSize = 64;
		const src = audioCtx.createMediaStreamSource(stream);
		src.connect(analyser);
		const buf = new Uint8Array(analyser.frequencyBinCount);
		const loop = () => {
			if (!analyser) return;
			analyser.getByteFrequencyData(buf);
			const nextLevels: number[] = [];
			const step = Math.floor(buf.length / levels.length);
			for (let i = 0; i < levels.length; i++) {
				let sum = 0;
				for (let j = 0; j < step; j++) sum += buf[i * step + j];
				nextLevels.push(Math.min(1, sum / (step * 255) + 0.05));
			}
			levels = nextLevels;
			rafId = requestAnimationFrame(loop);
		};
		loop();
	}

	function stopMeter() {
		if (rafId) cancelAnimationFrame(rafId);
		rafId = 0;
		analyser = null;
		if (audioCtx && audioCtx.state !== 'closed') audioCtx.close();
		audioCtx = null;
		levels = Array(28).fill(0.05);
	}

	function toggle() {
		if (recState === 'idle') start();
		else if (recState === 'recording') stop();
	}
</script>

<button
	class="group relative flex items-center gap-2 px-3 py-2 rounded-[10px] border transition-all duration-150"
	class:bg-accent={recState === 'recording'}
	class:text-white={recState === 'recording'}
	class:border-accent={recState === 'recording'}
	style:background={recState === 'recording' ? 'var(--color-accent)' : 'var(--color-surface)'}
	style:border-color={recState === 'recording' ? 'var(--color-accent)' : 'var(--color-border)'}
	style:color={recState === 'recording' ? 'white' : 'var(--color-text)'}
	disabled={disabled || recState === 'transcribing'}
	onclick={toggle}
	aria-label={recState === 'recording' ? 'Stop recording' : 'Start recording'}
	title="Hold to record — or just click"
>
	{#if recState === 'transcribing'}
		<Loader2 class="size-4 animate-spin" />
		<span class="text-xs font-medium">Transcribing…</span>
	{:else if recState === 'recording'}
		<Square class="size-4" fill="white" />
		<div class="flex items-end gap-[2px] h-4">
			{#each levels as l}
				<span
					class="w-[2px] rounded-full bg-white/90"
					style:height="{Math.max(2, l * 16)}px"
				></span>
			{/each}
		</div>
	{:else}
		<Mic class="size-4" />
		<span class="text-xs font-medium">Voice</span>
	{/if}
</button>
