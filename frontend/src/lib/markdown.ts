import { marked, type Tokens } from 'marked';
import markedKatex from 'marked-katex-extension';
import { markedHighlight } from 'marked-highlight';
import DOMPurify from 'dompurify';
import { getHighlighter, resolveLang, currentTheme } from './highlighter';

// Obsidian-style callout: > [!note] Title \n > body
type CalloutToken = Tokens.Generic & {
	type: 'callout';
	kind: string;
	title: string;
	body: string;
	tokens?: never;
};

const CALLOUT_ICONS: Record<string, string> = {
	note: '📝',
	info: 'ℹ️',
	tip: '💡',
	success: '✅',
	done: '✅',
	question: '❓',
	warning: '⚠️',
	caution: '⚠️',
	failure: '❌',
	danger: '⛔',
	bug: '🐞',
	example: '🔎',
	quote: '❝',
	todo: '🗹',
	abstract: '📄',
	summary: '📄'
};

const calloutExt = {
	name: 'callout',
	level: 'block' as const,
	start(src: string) {
		return src.match(/>\s*\[!/)?.index;
	},
	tokenizer(src: string): CalloutToken | undefined {
		const re = /^> \[!([a-zA-Z]+)\]([+-]?)([^\n]*)\n((?:>[^\n]*(?:\n|$))*)/;
		const m = re.exec(src);
		if (!m) return;
		const kind = m[1].toLowerCase();
		const title = m[3].trim();
		const body = m[4].replace(/^> ?/gm, '').trimEnd();
		return {
			type: 'callout',
			raw: m[0],
			kind,
			title,
			body
		};
	},
	renderer(token: CalloutToken): string {
		const icon = CALLOUT_ICONS[token.kind] ?? '💬';
		const inner = marked.parse(token.body, { async: false }) as string;
		const titleEsc = token.title
			? token.title
					.replace(/&/g, '&amp;')
					.replace(/</g, '&lt;')
					.replace(/>/g, '&gt;')
			: token.kind.charAt(0).toUpperCase() + token.kind.slice(1);
		return `<div class="md-callout md-callout-${token.kind}">
  <div class="md-callout-title"><span class="md-callout-icon">${icon}</span>${titleEsc}</div>
  <div class="md-callout-body">${inner}</div>
</div>`;
	}
};

let configured = false;
function configure() {
	if (configured) return;
	configured = true;

	marked.use(
		markedKatex({ throwOnError: false, nonStandard: true }) as never,
		markedHighlight({
			async: true,
			async highlight(code: string, lang: string) {
				try {
					const h = await getHighlighter();
					const loaded = h.getLoadedLanguages() as string[];
					const resolved = resolveLang(lang, loaded);
					if (!resolved) {
						return escape(code);
					}
					return h.codeToHtml(code, {
						lang: resolved,
						theme: currentTheme()
					});
				} catch (e) {
					console.warn('[md] highlight failed', e);
					return escape(code);
				}
			}
		}) as never
	);

	marked.use({ extensions: [calloutExt as never] });
	marked.use({ gfm: true, breaks: true });
}

function escape(s: string): string {
	return s
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;');
}

let hooksInstalled = false;
function installHooks() {
	if (hooksInstalled) return;
	hooksInstalled = true;
	DOMPurify.addHook('afterSanitizeAttributes', (node) => {
		if (node.tagName === 'A') {
			const href = node.getAttribute('href') || '';
			if (/^(https?:|mailto:|ftp:|tel:)/i.test(href)) {
				node.setAttribute('target', '_blank');
				node.setAttribute('rel', 'noopener noreferrer');
			}
		}
	});
}

const SANITIZE_CFG = {
	ADD_TAGS: [
		'math',
		'annotation',
		'semantics',
		'mrow',
		'mi',
		'mn',
		'mo',
		'mfrac',
		'msup',
		'msub',
		'munderover',
		'msqrt',
		'mroot',
		'mtext',
		'mspace',
		'mtable',
		'mtr',
		'mtd',
		'mstyle'
	],
	ADD_ATTR: ['class', 'style', 'data-lang', 'aria-hidden', 'target', 'rel'],
	FORBID_TAGS: ['script', 'style'],
	FORBID_ATTR: ['onerror', 'onload', 'onclick']
};

export async function renderMarkdown(source: string): Promise<string> {
	configure();
	installHooks();
	if (!source) return '';
	const raw = await marked.parse(source, { async: true });
	return DOMPurify.sanitize(String(raw), SANITIZE_CFG) as unknown as string;
}

export function renderMarkdownSync(source: string): string {
	configure();
	installHooks();
	if (!source) return '';
	const raw = marked.parse(source, { async: false }) as string;
	return DOMPurify.sanitize(String(raw), SANITIZE_CFG) as unknown as string;
}
