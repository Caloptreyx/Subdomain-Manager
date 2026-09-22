ALTER TABLE servers ADD COLUMN subdomain_limit INTEGER NOT NULL DEFAULT 0;

CREATE TABLE dev_caloptreyx_subdomains_domains (
    uuid uuid NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    domain VARCHAR(255) NOT NULL UNIQUE,
    provider VARCHAR(31) NOT NULL,
    zone_id VARCHAR(255) NOT NULL,
    credential TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE dev_caloptreyx_subdomains_subdomains (
    uuid uuid NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    server_uuid uuid NOT NULL REFERENCES servers(uuid) ON DELETE CASCADE,
    domain_uuid uuid NOT NULL REFERENCES dev_caloptreyx_subdomains_domains(uuid) ON DELETE RESTRICT,
    allocation_uuid uuid REFERENCES server_allocations(uuid) ON DELETE SET NULL,
    name VARCHAR(63) NOT NULL,
    records JSONB NOT NULL DEFAULT '[]',
    created TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (domain_uuid, name)
);

CREATE INDEX dev_caloptreyx_subdomains_subdomains_server_uuid_idx
    ON dev_caloptreyx_subdomains_subdomains (server_uuid);
