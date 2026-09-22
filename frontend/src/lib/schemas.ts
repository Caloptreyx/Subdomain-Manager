import { z } from 'zod';

export const recordTemplateAddressSchema = z.object({
  kind: z.literal('address'),
  name: z.string(),
  proxied: z.boolean(),
  ttl: z.number().int().min(0),
});

export const recordTemplateSrvSchema = z.object({
  kind: z.literal('srv'),
  service: z.string(),
  protocol: z.string(),
  priority: z.number().int().min(0),
  weight: z.number().int().min(0),
  ttl: z.number().int().min(0),
});

export const recordTemplateCustomSchema = z.object({
  kind: z.literal('custom'),
  recordType: z.enum(['A', 'AAAA', 'CNAME', 'TXT']),
  name: z.string(),
  content: z.string(),
  ttl: z.number().int().min(0),
});

export const recordTemplateSchema = z.discriminatedUnion('kind', [
  recordTemplateAddressSchema,
  recordTemplateSrvSchema,
  recordTemplateCustomSchema,
]);

export type RecordTemplate = z.infer<typeof recordTemplateSchema>;

export const extensionSettingsSchema = z.object({
  blacklist: z.array(z.string()),
  defaultLimit: z.number().int().min(0),
  defaultRecords: z.array(recordTemplateSchema),
  eggRecords: z.array(
    z.object({
      eggUuid: z.string(),
      records: z.array(recordTemplateSchema),
    }),
  ),
});

export type ExtensionSettings = z.infer<typeof extensionSettingsSchema>;

export const domainSchema = z.object({
  uuid: z.string(),
  domain: z.string(),
  provider: z.enum(['cloudflare', 'bunny']),
  zoneId: z.string(),
  enabled: z.boolean(),
  subdomainCount: z.number().int(),
  created: z.coerce.date(),
});

export type Domain = z.infer<typeof domainSchema>;

export const domainRefSchema = z.object({
  uuid: z.string(),
  domain: z.string(),
});

export const subdomainAllocationSchema = z.object({
  uuid: z.string(),
  ip: z.string(),
  ipAlias: z.string().nullable(),
  port: z.number().int(),
});

export const storedRecordSchema = z.object({
  recordType: z.string(),
  name: z.string(),
  content: z.string(),
});

export const subdomainSchema = z.object({
  uuid: z.string(),
  name: z.string(),
  fqdn: z.string(),
  domain: domainRefSchema,
  allocation: subdomainAllocationSchema.nullable(),
  records: z.array(storedRecordSchema),
  created: z.coerce.date(),
});

export type Subdomain = z.infer<typeof subdomainSchema>;

export const adminSubdomainSchema = subdomainSchema.extend({
  server: z.object({
    uuid: z.string(),
    name: z.string(),
  }),
});

export type AdminSubdomain = z.infer<typeof adminSubdomainSchema>;

export const serverFeatureLimitsExtensionSchema = z.object({
  subdomains: z.number().int().min(0),
});

export const SUBDOMAIN_NAME_REGEX = /^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$/;

export const SUBDOMAIN_ADMIN_BASE = '/api/admin/extensions/dev.caloptreyx.subdomains';

export const subdomainClientBase = (serverUuid: string) => `/api/client/servers/${serverUuid}/subdomains`;
