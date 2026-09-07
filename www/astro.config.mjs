// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// Served at tryglide.net/docs (the marketing site rewrites /docs/* here).
// Sidebar follows Diátaxis: tutorials teach, how-to guides do, reference
// states, explanation reasons. Top-level entries are the two pages everyone
// hits first.
export default defineConfig({
	site: 'https://tryglide.net',
	base: '/docs',
	outDir: './dist/docs',
	trailingSlash: 'always',
	integrations: [
		starlight({
			title: 'Glide',
			description: 'Keep your priorities in view while you work in the terminal.',
			logo: { src: './src/assets/glide-mark.svg', replacesTitle: false },
			favicon: '/docs/favicon.svg',
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/matthewvilaysack/glide' }],
			editLink: { baseUrl: 'https://github.com/matthewvilaysack/glide/edit/main/www/' },
			customCss: ['./src/styles/custom.css'],
			sidebar: [
				{ label: 'Intro', slug: 'index' },
				{ label: 'Install', slug: 'how-to/install' },
				{
					label: 'Tutorials',
					items: [
						{ label: 'Quick start', slug: 'tutorials/quick-start' },
						{ label: 'A first day with an agent', slug: 'tutorials/first-day' },
					],
				},
				{
					label: 'How-to guides',
					items: [
						{ label: 'Put the line in your status bar', slug: 'how-to/status-bar' },
						{ label: 'Wire Claude Code', slug: 'how-to/claude-code' },
						{ label: 'Wire Warp', slug: 'how-to/warp' },
						{ label: 'Wire other agents', slug: 'how-to/other-agents' },
						{ label: 'Use the MCP server', slug: 'how-to/mcp' },
						{ label: 'Verify a download', slug: 'how-to/verify-download' },
						{ label: 'Patch a shipped release', slug: 'how-to/patch-release' },
						{ label: 'Troubleshooting', slug: 'how-to/troubleshooting' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'CLI', slug: 'reference/cli' },
						{ label: 'Focus verbs', slug: 'reference/focus' },
						{ label: 'The daily note', slug: 'reference/daily-note' },
						{ label: 'Config', slug: 'reference/config' },
						{ label: 'Console', slug: 'reference/console' },
						{ label: 'Releases and versioning', slug: 'reference/releases' },
						{ label: 'Privacy', slug: 'reference/privacy' },
					],
				},
				{
					label: 'Explanation',
					items: [
						{ label: 'How agents fit', slug: 'explanation/agents' },
						{ label: 'Why a hook, not a tool schema', slug: 'explanation/why-a-hook' },
						{ label: 'Why plain files', slug: 'explanation/plain-files' },
						{ label: 'What glide never does', slug: 'explanation/what-it-never-does' },
						{ label: 'The Teams tier', slug: 'explanation/teams' },
					],
				},
			],
		}),
	],
});
