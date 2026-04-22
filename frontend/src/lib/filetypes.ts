export const EXT_LANG: Record<string, string> = {
	md: 'markdown',
	markdown: 'markdown',
	mdx: 'markdown',
	txt: 'text',
	json: 'json',
	jsonc: 'json',
	toml: 'toml',
	yaml: 'yaml',
	yml: 'yaml',
	js: 'js',
	mjs: 'js',
	cjs: 'js',
	jsx: 'jsx',
	ts: 'ts',
	mts: 'ts',
	cts: 'ts',
	tsx: 'tsx',
	svelte: 'svelte',
	html: 'html',
	htm: 'html',
	css: 'css',
	scss: 'scss',
	rs: 'rust',
	py: 'python',
	sh: 'bash',
	bash: 'bash',
	zsh: 'bash',
	go: 'go',
	java: 'java',
	c: 'c',
	cpp: 'cpp',
	cc: 'cpp',
	cxx: 'cpp',
	h: 'c',
	hpp: 'cpp',
	sql: 'sql',
	rb: 'ruby',
	php: 'php',
	diff: 'diff',
	patch: 'diff'
};

// filenames without extensions
export const NAME_LANG: Record<string, string> = {
	Dockerfile: 'docker',
	dockerfile: 'docker',
	Makefile: 'makefile',
	makefile: 'makefile',
	'.env': 'bash',
	'.gitignore': 'text',
	'.dockerignore': 'text'
};

export function langFor(name: string, ext?: string | null): string | null {
	if (ext && EXT_LANG[ext]) return EXT_LANG[ext];
	const base = name.split('/').pop() || name;
	if (NAME_LANG[base]) return NAME_LANG[base];
	return null;
}

export const MESH_EXTS = new Set(['stl']);
export const IMAGE_EXTS = new Set([
	'png',
	'jpg',
	'jpeg',
	'gif',
	'webp',
	'avif',
	'bmp',
	'ico',
	'svg'
]);
export const DATA_EXTS = new Set(['csv', 'tsv']);

export function isMesh(ext?: string | null): boolean {
	return !!ext && MESH_EXTS.has(ext);
}

export function isImage(ext?: string | null): boolean {
	return !!ext && IMAGE_EXTS.has(ext);
}

export function isData(ext?: string | null): boolean {
	return !!ext && DATA_EXTS.has(ext);
}

export function isViewable(isDir: boolean, name: string, ext?: string | null): boolean {
	if (isDir) return true;
	if (isMesh(ext) || isImage(ext) || isData(ext)) return true;
	return langFor(name, ext) != null;
}

export function isMarkdown(ext?: string | null): boolean {
	return ext === 'md' || ext === 'markdown' || ext === 'mdx';
}
