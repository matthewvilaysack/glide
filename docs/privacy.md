# Glide — privacy and data handling (one-pager)

Pre-engagement answer to "where does our code go." Real, not aspirational. Updated 2026-05-28.

## What Glide reads

Only the sources you explicitly connect, all read-only:

- **GitHub.** Repos you grant access to. Code, PRs, issues, CODEOWNERS, commit history. We do not write or commit on your behalf. Read-only PAT or fine-grained scope.
- **Slack.** Only channels we are explicitly invited to. We do not read DMs, we do not request DM scope, we cannot see private channels we are not in.
- **Confluence or Notion.** Pages we are explicitly granted access to, scoped to the spaces you choose.
- **Linear or Jira.** Tickets in the project(s) you grant. Read-only.

If you don't grant a source, Glide doesn't see it.

## What Glide stores

Phase 0 (the concierge engagement, where we are today):
- Everything runs on the engineer's local machine. The graph (an indexed representation of your code, Slack history, and Confluence pages) lives on disk on one laptop, encrypted at rest.
- We do not ship this graph anywhere. We do not back it up to a third party.
- When the engagement ends or you ask us to, we delete it.

Phase 1 (the hosted product, post-design-partner):
- The graph lives in our hosted environment.
- Data is encrypted in transit (TLS 1.3) and at rest (AES-256).
- Each customer's graph is isolated. We do not train shared models on your data, ever.
- SOC2 Type 1 is committed for month 6 of Phase 1.

## What Glide does not touch

- Direct messages on Slack. Ever.
- Anything in a repo or channel you have not explicitly granted.
- Secrets, environment files, anything matching common secret patterns. Glide actively skips them during indexing.
- Production systems. Glide is read-only on metadata, not a runtime integration.

## Where the agent runs (Claude)

- Glide uses Anthropic's Claude API. Prompts contain the question being asked plus the relevant slice of your graph (file contents, ownership, ticket text).
- Anthropic's API enterprise commitments apply: zero data retention on the API by default, no training on inputs.
- We do not send your data to any other LLM provider.

## Deletion and export

- Email Matthew (hello@tryglide.dev) at any time. We confirm deletion within 24 hours during a Phase 0 engagement.
- For Phase 1, a self-serve delete button in the dashboard.
- Export of your graph (as JSON) on request.

## What we ask you to do

- Use a service account or fine-grained PAT for the GitHub connection, not a personal token.
- Add Glide to one Slack channel, not as a workspace-wide admin.
- Tell us in writing which Confluence spaces are off-limits, if any.

## What we'll ship before a paid engagement

This one-pager is sufficient for a Phase 0 concierge. Before any paid contract:

- Written DPA
- Sub-processor list (Anthropic, our hosting provider, our database provider)
- SOC2 progress letter

## Contact

Matthew Vilaysack — hello@tryglide.dev — for any data question, no question too small.
