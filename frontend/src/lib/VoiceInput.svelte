<script lang="ts">
	import Mic from 'lucide-svelte/icons/mic';
	import Square from 'lucide-svelte/icons/square';
	import Loader2 from 'lucide-svelte/icons/loader-2';
	import workletUrl from '$lib/audio-recorder-worklet.js?url';

	type Props = {
		onTranscribed: (text: string) => void;
		disabled?: boolean;
	};
	let { onTranscribed, disabled = false }: Props = $props();

	let recState = $state<'idle' | 'recording' | 'transcribing'>('idle');
	let levels = $state<number[]>(Array(28).fill(0.05));

	const CHUNK_SECONDS = 15;
	const MIN_TAIL_RATIO = 0.02;

	let audioCtx: AudioContext | null = null;
	let sourceNode: MediaStreamAudioSourceNode | null = null;
	let analyser: AnalyserNode | null = null;
	let workletNode: AudioWorkletNode | null = null;
	let sourceStream: MediaStream | null = null;
	let rafId = 0;

	let sampleRate = 48000;
	let chunkSamples = 0;
	let pendingFrames: Float32Array[] = [];
	let pendingCount = 0;
	let chunkPromises: Promise<string>[] = [];

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

			audioCtx = new AudioContext();
			sampleRate = audioCtx.sampleRate;
			chunkSamples = Math.floor(sampleRate * CHUNK_SECONDS);
			pendingFrames = [];
			pendingCount = 0;
			chunkPromises = [];

			await audioCtx.audioWorklet.addModule(workletUrl);

			sourceNode = audioCtx.createMediaStreamSource(stream);
			analyser = audioCtx.createAnalyser();
			analyser.fftSize = 64;
			sourceNode.connect(analyser);

			workletNode = new AudioWorkletNode(audioCtx, 'recorder-processor');
			workletNode.port.onmessage = (e) => {
				const frame = e.data as Float32Array;
				pendingFrames.push(frame);
				pendingCount += frame.length;
				while (pendingCount >= chunkSamples) flushChunk(chunkSamples);
			};
			sourceNode.connect(workletNode);

			startMeter();
			recState = 'recording';
			console.log(`[voice] recording at ${sampleRate}Hz, slicing every ${CHUNK_SECONDS}s`);
		} catch (e) {
			const err = e as DOMException;
			console.error('[voice] start failed', err.name, err.message, err);
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
			cleanup();
		}
	}

	function flushChunk(samples: number) {
		if (samples <= 0) return;
		const out = new Float32Array(samples);
		let written = 0;
		while (written < samples && pendingFrames.length > 0) {
			const head = pendingFrames[0];
			const need = samples - written;
			if (head.length <= need) {
				out.set(head, written);
				written += head.length;
				pendingFrames.shift();
			} else {
				out.set(head.subarray(0, need), written);
				pendingFrames[0] = head.subarray(need);
				written += need;
			}
		}
		pendingCount -= samples;
		const idx = chunkPromises.length;
		const wav = encodeWav(out, sampleRate);
		chunkPromises.push(transcribeChunk(wav, idx));
	}

	async function transcribeChunk(blob: Blob, idx: number): Promise<string> {
		const form = new FormData();
		form.append('audio', blob, `chunk-${idx}.wav`);
		const t0 = performance.now();
		try {
			const r = await fetch('/api/transcribe', { method: 'POST', body: form });
			const ms = Math.round(performance.now() - t0);
			if (!r.ok) {
				const err = await r.text();
				throw new Error(`${r.status} ${err}`);
			}
			const data = await r.json();
			console.log(`[voice] chunk ${idx} → ${data.text.length} chars in ${ms}ms`);
			return data.text;
		} catch (e) {
			console.error(`[voice] chunk ${idx} failed`, e);
			return '';
		}
	}

	async function stop() {
		if (recState !== 'recording') return;
		recState = 'transcribing';

		try { sourceNode?.disconnect(); } catch {}
		try { workletNode?.disconnect(); } catch {}
		sourceStream?.getTracks().forEach((t) => t.stop());

		const minTail = Math.floor(chunkSamples * MIN_TAIL_RATIO);
		if (pendingCount > minTail) flushChunk(pendingCount);
		else { pendingFrames = []; pendingCount = 0; }

		stopMeter();

		try {
			const texts = await Promise.all(chunkPromises);
			const joined = texts.map((s) => s.trim()).filter(Boolean).join(' ');
			console.log(`[voice] ${chunkPromises.length} chunks → ${joined.length} chars`);
			if (joined) onTranscribed(joined);
		} finally {
			cleanup();
			recState = 'idle';
		}
	}

	function cleanup() {
		try { workletNode?.port.close(); } catch {}
		workletNode = null;
		sourceNode = null;
		analyser = null;
		if (audioCtx && audioCtx.state !== 'closed') audioCtx.close().catch(() => {});
		audioCtx = null;
		sourceStream = null;
		pendingFrames = [];
		pendingCount = 0;
		chunkPromises = [];
	}

	function encodeWav(samples: Float32Array, sr: number): Blob {
		const len = samples.length;
		const buf = new ArrayBuffer(44 + len * 2);
		const view = new DataView(buf);
		writeStr(view, 0, 'RIFF');
		view.setUint32(4, 36 + len * 2, true);
		writeStr(view, 8, 'WAVE');
		writeStr(view, 12, 'fmt ');
		view.setUint32(16, 16, true);
		view.setUint16(20, 1, true);
		view.setUint16(22, 1, true);
		view.setUint32(24, sr, true);
		view.setUint32(28, sr * 2, true);
		view.setUint16(32, 2, true);
		view.setUint16(34, 16, true);
		writeStr(view, 36, 'data');
		view.setUint32(40, len * 2, true);
		let off = 44;
		for (let i = 0; i < len; i++) {
			const s = Math.max(-1, Math.min(1, samples[i]));
			view.setInt16(off, s < 0 ? s * 0x8000 : s * 0x7fff, true);
			off += 2;
		}
		return new Blob([buf], { type: 'audio/wav' });
	}

	function writeStr(view: DataView, off: number, str: string) {
		for (let i = 0; i < str.length; i++) view.setUint8(off + i, str.charCodeAt(i));
	}

	function startMeter() {
		if (!analyser) return;
		const buf = new Uint8Array(analyser.frequencyBinCount);
		const loop = () => {
			if (!analyser) return;
			analyser.getByteFrequencyData(buf);
			const next: number[] = [];
			const step = Math.max(1, Math.floor(buf.length / levels.length));
			for (let i = 0; i < levels.length; i++) {
				let sum = 0;
				for (let j = 0; j < step; j++) sum += buf[i * step + j] || 0;
				next.push(Math.min(1, sum / (step * 255) + 0.05));
			}
			levels = next;
			rafId = requestAnimationFrame(loop);
		};
		loop();
	}

	function stopMeter() {
		if (rafId) cancelAnimationFrame(rafId);
		rafId = 0;
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
