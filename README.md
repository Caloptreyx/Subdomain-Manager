# Subdomain Manager

A [Calagopus Panel](https://calagopus.com) extension that lets users create and manage
subdomains for their game servers (mainly Minecraft) through **Cloudflare** or **Bunny.net DNS**.

Package name: `dev.caloptreyx.subdomains` · Requires panel `>=1.2.2`

## Features

- Users pick a domain, a name and one of their server's allocations; the extension creates the
  DNS records and shows the resulting `name.domain`.
- Seamless Minecraft integration: default record set is an address record plus a
  `_minecraft._tcp` SRV record, so players connect without typing a port.
- **Per-egg record templates** with placeholders (`{name}`, `{domain}`, `{fqdn}`, `{ip}`, `{port}`,
  `{server}`, `{server_name}`) – e.g. an A-only set for Minecraft Bedrock, or extra custom
  A/AAAA/CNAME/TXT records.
- **IP alias parsing**: the allocation's alias is used as the record target; hostnames become CNAME,
  IPv4 becomes A, IPv6 becomes AAAA.
- **Blacklist** subdomain names with regular expressions.
- **Per-server subdomain limit** exposed as `feature_limits.subdomains` and editable in the admin
  server create/update forms.
- **Proper cleanup**: DNS records are removed when a subdomain, its server or its allocation is
  deleted, and when a server is transferred to another node.
- **Allocation changes at any time**; after a server transfer the allocation is set to `null`
  (Unknown) so the user can re-point the subdomain, which creates new records.
- Admin overview of every subdomain across the panel, with search.

## Installation

Download `dev_caloptreyx_subdomains.c7s.zip` from the
[latest release](https://github.com/Caloptreyx/Subdomain-Manager/releases/latest) and either upload it under **Admin → Extensions**
or drop it into your heavy image's `build/extensions/` directory and `docker compose restart web`.
Extensions require the `:heavy` panel image (or a dev environment) — see the
[Calagopus docs](https://calagopus.com/docs/panel/extensions/installing-extensions).

## Configuration

**Admin → Extensions → Subdomain Manager → Configure**

- **Domains** – add the domains users may choose from.
  - *Cloudflare*: Zone ID (zone overview page) + an API token with `Zone:DNS:Edit` on that zone.
  - *Bunny.net*: numeric DNS zone id (from the dashboard URL) + an account API key.
  Credentials are verified against the provider before saving and stored encrypted.
- **Settings** – blacklist regexes, default subdomain limit for new servers, default record
  templates and per-egg overrides. A TTL of `0` means "provider automatic".
- **Subdomains** – searchable list of all subdomains with their servers.

Permissions: server `subdomains.read|create|update|delete`, admin `subdomains.read|manage`.

## API

- `GET|POST /api/client/servers/{server}/subdomains`, `PATCH|DELETE .../subdomains/{uuid}`
- `GET|PUT /api/admin/extensions/dev.caloptreyx.subdomains/settings`
- `GET|POST .../domains`, `PATCH|DELETE .../domains/{uuid}`, `POST .../domains/{uuid}/verify`
- `GET .../subdomains?page&per_page&search`

Full schemas are in the panel's OpenAPI document once installed.

## Development

The extension has to live inside a checkout of the panel repository
(`backend-extensions/dev_caloptreyx_subdomains`, a symlink to this repo works):

```bash
# from the panel repo root
SQLX_OFFLINE=true cargo check -p dev_caloptreyx_subdomains
SQLX_OFFLINE=true cargo clippy -p dev_caloptreyx_subdomains
SQLX_OFFLINE=true cargo test -p dev_caloptreyx_subdomains
cd frontend && pnpm build:ci && cd ..
SQLX_OFFLINE=true panel-rs extensions export dev.caloptreyx.subdomains  # -> exported-extensions/
```

Notes:
- `frontend/tsconfig.json` lists two `@/*` path fallbacks so the build works both in-tree and when
  the extension directory is symlinked from elsewhere.
- Do not run `panel-rs extensions resync` while the extension directory is a symlink; the panel
  skips symlinked entries and would drop the extension from its internal list.

## Roadmap

- CLI command to migrate data from the Pterodactyl subdomain manager extension.

## License

MIT
