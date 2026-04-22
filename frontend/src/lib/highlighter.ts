import type { Highlighter, BundledLanguage } from 'shiki';

let hp: Promise<Highlighter> | null = null;

const THEMES = ['github-dark-dimmed', 'github-light', 'github-dark'] as const;

export type ShikiTheme = (typeof THEMES)[number];
const LANGS: BundledLanguage[] = [
	'ts',
	'tsx',
	'js',
	'jsx',
	'rust',
	'python',
	'bash',
	'shellscript',
	'json',
	'html',
	'css',
	'scss',
	'svelte',
	'yaml',
	'toml',
	'markdown',
	'diff',
	'go',
	'java',
	'c',
	'cpp',
	'sql',
	'ruby',
	'php',
	'docker',
	'makefile'
];

const LANG_ALIAS: Record<string, BundledLanguage> = {
	sh: 'bash',
	shell: 'bash',
	zsh: 'bash',
	js: 'javascript',
	mjs: 'javascript',
	cjs: 'javascript',
	ts: 'typescript',
	mts: 'typescript',
	cts: 'typescript',
	py: 'python',
	rs: 'rust',
	yml: 'yaml',
	md: 'markdown',
	htm: 'html',
	'c++': 'cpp',
	hpp: 'cpp',
	cxx: 'cpp',
	rb: 'ruby',
	patch: 'diff',
	dockerfile: 'docker'
};

export function getHighlighter(): Promise<Highlighter> {
	if (!hp) {
		hp = import('shiki').then((s) =>
			s.createHighlighter({
				themes: [...THEMES],
				langs: LANGS
			})
		);
	}
	return hp;
}

export function currentTheme(): (typeof THEMES)[number] {
	if (typeof window === 'undefined') return 'github-light';
	return window.matchMedia('(prefers-color-scheme: dark)').matches
		? 'github-dark-dimmed'
		: 'github-light';
}

export function resolveLang(raw: string | null | undefined, loaded: string[]): string | null {
	if (!raw) return null;
	const l = raw.toLowerCase();
	const candidate = (LANG_ALIAS[l] ?? l) as string;
	return loaded.includes(candidate) ? candidate : null;
}

function escape(s: string): string {
	return s
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;');
}

export async function codeToHtml(
	code: string,
	lang: string | null,
	opts?: { theme?: ShikiTheme }
): Promise<string> {
	try {
		const h = await getHighlighter();
		const loaded = h.getLoadedLanguages() as string[];
		const resolved = resolveLang(lang, loaded);
		const theme = opts?.theme ?? currentTheme();
		if (!resolved) {
			return `<pre class="shiki-plain"><code>${escape(code)}</code></pre>`;
		}
		const raw = h.codeToHtml(code, { lang: resolved, theme });
		// Shiki inserts a literal "\n" between each <span class="line">…</span>.
		// With white-space: pre on <pre>, those newlines render as empty lines
		// between every visible line (double-spacing). Strip them.
		return raw.replace(/<\/span>\n(?=<span class="line")/g, '</span>');
	} catch (e) {
		console.warn('[shiki] codeToHtml failed', e);
		return `<pre class="shiki-plain"><code>${escape(code)}</code></pre>`;
	}
}
