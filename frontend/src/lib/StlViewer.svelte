<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import * as THREE from 'three';
	import { STLLoader } from 'three/examples/jsm/loaders/STLLoader.js';
	import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
	import RefreshCw from 'lucide-svelte/icons/refresh-cw';
	import Maximize from 'lucide-svelte/icons/maximize';
	import Grid from 'lucide-svelte/icons/grid-3x3';

	type Props = { path: string };
	let { path }: Props = $props();

	let hostEl: HTMLDivElement;
	let status = $state<'loading' | 'ready' | 'error'>('loading');
	let errMsg = $state('');
	let stats = $state<{ triangles: number; bytes: number; bbox: string } | null>(null);
	let showGrid = $state(true);

	let renderer: THREE.WebGLRenderer | null = null;
	let scene: THREE.Scene | null = null;
	let camera: THREE.PerspectiveCamera | null = null;
	let controls: OrbitControls | null = null;
	let mesh: THREE.Mesh | null = null;
	let grid: THREE.GridHelper | null = null;
	let raf = 0;
	let resizeObs: ResizeObserver | null = null;
	let disposed = false;

	function setupScene() {
		scene = new THREE.Scene();
		scene.background = new THREE.Color(0x1b1f24);

		const w = hostEl.clientWidth;
		const h = hostEl.clientHeight;
		camera = new THREE.PerspectiveCamera(45, w / h, 0.1, 10000);
		camera.position.set(80, 80, 120);

		renderer = new THREE.WebGLRenderer({ antialias: true });
		renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
		renderer.setSize(w, h);
		hostEl.appendChild(renderer.domElement);

		const hemi = new THREE.HemisphereLight(0xffffff, 0x1a1a1a, 0.9);
		scene.add(hemi);
		const key = new THREE.DirectionalLight(0xffffff, 1.1);
		key.position.set(1, 1.5, 1);
		scene.add(key);
		const fill = new THREE.DirectionalLight(0xffffff, 0.4);
		fill.position.set(-1, -0.5, -1);
		scene.add(fill);

		grid = new THREE.GridHelper(200, 20, 0x444a52, 0x2a2f36);
		(grid.material as THREE.Material).transparent = true;
		(grid.material as THREE.Material).opacity = 0.6;
		grid.visible = showGrid;
		scene.add(grid);

		controls = new OrbitControls(camera, renderer.domElement);
		controls.enableDamping = true;
		controls.dampingFactor = 0.08;

		const tick = () => {
			if (disposed) return;
			controls!.update();
			renderer!.render(scene!, camera!);
			raf = requestAnimationFrame(tick);
		};
		tick();

		resizeObs = new ResizeObserver(() => {
			if (!renderer || !camera || !hostEl) return;
			const nw = hostEl.clientWidth;
			const nh = hostEl.clientHeight;
			if (nw === 0 || nh === 0) return;
			renderer.setSize(nw, nh);
			camera.aspect = nw / nh;
			camera.updateProjectionMatrix();
		});
		resizeObs.observe(hostEl);
	}

	async function loadStl(p: string) {
		status = 'loading';
		errMsg = '';
		stats = null;

		try {
			const url = `/api/fs/raw?path=${encodeURIComponent(p)}`;
			const r = await fetch(url);
			if (!r.ok) throw new Error(await r.text());
			const buf = await r.arrayBuffer();

			const loader = new STLLoader();
			const geom = loader.parse(buf);
			geom.computeVertexNormals();
			geom.center();

			if (mesh) {
				scene!.remove(mesh);
				mesh.geometry.dispose();
				(mesh.material as THREE.Material).dispose();
			}

			const mat = new THREE.MeshStandardMaterial({
				color: 0xb8c1cc,
				metalness: 0.15,
				roughness: 0.55,
				flatShading: false
			});
			mesh = new THREE.Mesh(geom, mat);
			scene!.add(mesh);

			frameObject();

			const tri = (geom.getAttribute('position').count / 3) | 0;
			geom.computeBoundingBox();
			const bb = geom.boundingBox!;
			const sx = (bb.max.x - bb.min.x).toFixed(1);
			const sy = (bb.max.y - bb.min.y).toFixed(1);
			const sz = (bb.max.z - bb.min.z).toFixed(1);
			stats = {
				triangles: tri,
				bytes: buf.byteLength,
				bbox: `${sx} × ${sy} × ${sz}`
			};
			status = 'ready';
		} catch (e) {
			errMsg = (e as Error).message;
			status = 'error';
		}
	}

	function frameObject() {
		if (!mesh || !camera || !controls) return;
		mesh.geometry.computeBoundingSphere();
		const bs = mesh.geometry.boundingSphere!;
		const r = bs.radius || 50;
		const dist = r * 2.6;
		const dir = new THREE.Vector3(1, 0.8, 1.2).normalize();
		camera.position.copy(dir.multiplyScalar(dist));
		camera.near = Math.max(0.01, r / 100);
		camera.far = dist * 20;
		camera.lookAt(0, 0, 0);
		camera.updateProjectionMatrix();
		controls.target.set(0, 0, 0);
		controls.update();

		if (grid) {
			const size = Math.max(r * 3, 20);
			const divs = 20;
			scene!.remove(grid);
			grid.geometry.dispose();
			(grid.material as THREE.Material).dispose();
			grid = new THREE.GridHelper(size, divs, 0x444a52, 0x2a2f36);
			(grid.material as THREE.Material).transparent = true;
			(grid.material as THREE.Material).opacity = 0.6;
			grid.position.y = -r;
			grid.visible = showGrid;
			scene!.add(grid);
		}
	}

	function fmtBytes(n: number) {
		if (n < 1024) return `${n} B`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
		return `${(n / 1024 / 1024).toFixed(1)} MB`;
	}

	$effect(() => {
		if (!hostEl) return;
		const p = path;
		if (p) loadStl(p);
	});

	$effect(() => {
		if (grid) grid.visible = showGrid;
	});

	onMount(() => {
		setupScene();
	});

	onDestroy(() => {
		disposed = true;
		cancelAnimationFrame(raf);
		resizeObs?.disconnect();
		controls?.dispose();
		if (mesh) {
			mesh.geometry.dispose();
			(mesh.material as THREE.Material).dispose();
		}
		if (grid) {
			grid.geometry.dispose();
			(grid.material as THREE.Material).dispose();
		}
		renderer?.dispose();
		if (renderer?.domElement && renderer.domElement.parentNode) {
			renderer.domElement.parentNode.removeChild(renderer.domElement);
		}
	});
</script>

<div class="stl-root">
	<div bind:this={hostEl} class="stl-canvas-host"></div>

	{#if status === 'loading'}
		<div class="stl-overlay">
			<div class="stl-spinner"></div>
			<span>Loading mesh…</span>
		</div>
	{:else if status === 'error'}
		<div class="stl-overlay stl-error">
			<span>Failed to load STL</span>
			<code>{errMsg}</code>
		</div>
	{/if}

	<div class="stl-toolbar">
		<button
			class="stl-btn"
			onclick={() => (showGrid = !showGrid)}
			class:active={showGrid}
			title="Toggle grid"
		>
			<Grid class="size-3.5" />
		</button>
		<button class="stl-btn" onclick={frameObject} title="Reframe">
			<Maximize class="size-3.5" />
		</button>
		<button class="stl-btn" onclick={() => loadStl(path)} title="Reload">
			<RefreshCw class="size-3.5" />
		</button>
	</div>

	{#if stats}
		<div class="stl-status">
			<span class="font-mono">{stats.triangles.toLocaleString()} tris</span>
			<span class="sep">·</span>
			<span class="font-mono">{stats.bbox}</span>
			<span class="sep">·</span>
			<span class="font-mono">{fmtBytes(stats.bytes)}</span>
			<span class="ml-auto font-mono truncate opacity-70" title={path}>{path}</span>
		</div>
	{/if}
</div>

<style>
	.stl-root {
		position: relative;
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		background: #1b1f24;
	}
	.stl-canvas-host {
		flex: 1;
		min-height: 0;
		width: 100%;
		overflow: hidden;
	}
	.stl-canvas-host :global(canvas) {
		display: block;
	}
	.stl-toolbar {
		position: absolute;
		top: 10px;
		right: 10px;
		display: flex;
		gap: 4px;
		background: rgba(22, 27, 34, 0.88);
		backdrop-filter: blur(8px);
		padding: 4px;
		border-radius: 6px;
		border: 1px solid rgba(255, 255, 255, 0.08);
	}
	.stl-btn {
		width: 28px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 4px;
		background: transparent;
		color: #c9d1d9;
		cursor: pointer;
		transition: background 0.12s;
	}
	.stl-btn:hover {
		background: #21262d;
	}
	.stl-btn.active {
		background: #1f6feb;
		color: white;
	}
	.stl-overlay {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		color: #8b949e;
		font-size: 12px;
		pointer-events: none;
		background: rgba(27, 31, 36, 0.6);
	}
	.stl-overlay.stl-error {
		color: #f85149;
	}
	.stl-overlay code {
		font-family: var(--font-mono);
		font-size: 11px;
		color: #f85149;
		background: rgba(248, 81, 73, 0.08);
		padding: 4px 8px;
		border-radius: 4px;
		max-width: 80%;
		text-align: center;
	}
	.stl-spinner {
		width: 20px;
		height: 20px;
		border: 2px solid rgba(255, 255, 255, 0.15);
		border-top-color: #58a6ff;
		border-radius: 50%;
		animation: stl-spin 0.8s linear infinite;
	}
	@keyframes stl-spin {
		to {
			transform: rotate(360deg);
		}
	}
	.stl-status {
		height: 24px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 14px;
		font-size: 11px;
		background: #1b1f24;
		border-top: 1px solid rgba(255, 255, 255, 0.06);
		color: #8b949e;
	}
	.stl-status .sep {
		color: #30363d;
	}
</style>
