// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// Served at tryglide.net/docs (the marketing site rewrites /docs/* here).
export default defineConfig({
	site: 'https://tryglide.net',
	base: '/docs',
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
				{ label: 'Install', slug: 'install' },
				{ label: 'Quick start', slug: 'quick-start' },
				{ label: 'Config', slug: 'config' },
				{ label: 'Troubleshooting', slug: 'troubleshooting' },
				{
					label: 'Usage',
					items: [
						{ label: 'Focus', slug: 'usage/focus' },
						{ label: 'The daily note', slug: 'usage/vault' },
						{ label: 'Status bar', slug: 'usage/status-bar' },
						{ label: 'Console', slug: 'usage/console' },
					],
				},
				{
					label: 'Agents',
					items: [
						{ label: 'Overview', slug: 'agents/overview' },
						{ label: 'Claude Code', slug: 'agents/claude-code' },
						{ label: 'Warp', slug: 'agents/warp' },
						{ label: 'Other agents', slug: 'agents/others' },
						{ label: 'MCP server', slug: 'agents/mcp' },
					],
				},
				{
					label: 'Teams',
					items: [
						{ label: 'Onboarding tier', slug: 'teams/onboarding' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'CLI', slug: 'reference/cli' },
						{ label: 'Releases', slug: 'reference/releases' },
						{ label: 'Privacy', slug: 'reference/privacy' },
					],
				},
			],
		}),
	],
});
