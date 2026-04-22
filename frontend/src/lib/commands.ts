export type Cmd = {
	trigger: string;
	label: string;
	description: string;
	kind: 'client' | 'prompt';
	body?: string;
};

export const COMMANDS: Cmd[] = [
	{
		trigger: '/clear',
		label: 'Clear',
		description: 'Clear the current conversation and start fresh',
		kind: 'client'
	},
	{
		trigger: '/new',
		label: 'New conversation',
		description: 'Reset session_id and start a new conversation',
		kind: 'client'
	},
	{
		trigger: '/help',
		label: 'Help',
		description: 'Show available commands',
		kind: 'client'
	},
	{
		trigger: '/cwd',
		label: 'Change directory',
		description: 'Open the path picker',
		kind: 'client'
	},
	{
		trigger: '/terminal',
		label: 'Toggle terminal',
		description: 'Show or hide the embedded terminal',
		kind: 'client'
	},
	{
		trigger: '/init',
		label: 'Init CLAUDE.md',
		description: 'Generate a CLAUDE.md with codebase documentation',
		kind: 'prompt',
		body: 'Please run the /init skill to initialize a CLAUDE.md file for this project.'
	},
	{
		trigger: '/review',
		label: 'Review PR',
		description: 'Review the pending changes on the current branch',
		kind: 'prompt',
		body: 'Please run the /review skill on the current branch.'
	},
	{
		trigger: '/security-review',
		label: 'Security review',
		description: 'Run a security review on pending changes',
		kind: 'prompt',
		body: 'Please run the /security-review skill on the current branch.'
	},
	{
		trigger: '/simplify',
		label: 'Simplify',
		description: 'Review changed code for reuse, quality, and efficiency',
		kind: 'prompt',
		body: 'Please run the /simplify skill on the recent changes.'
	},
	{
		trigger: '/status',
		label: 'Status',
		description: 'Ask Claude for a git status summary',
		kind: 'prompt',
		body: 'Run `git status` and summarize what has changed.'
	},
	{
		trigger: '/explain',
		label: 'Explain repo',
		description: 'Ask for a tour of the current working directory',
		kind: 'prompt',
		body: 'Give me a concise overview of this codebase: structure, languages, what it does.'
	}
];

export function matchCommands(text: string): Cmd[] {
	const t = text.trim().toLowerCase();
	if (!t.startsWith('/')) return [];
	return COMMANDS.filter((c) => c.trigger.startsWith(t)).slice(0, 8);
}
