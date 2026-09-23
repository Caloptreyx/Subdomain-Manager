import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi, serializeForApi } from '@/lib/serialization/api-transform.ts';
import { type Domain, domainSchema, SUBDOMAIN_ADMIN_BASE } from '../../lib/schemas.ts';

export const updateDomainSchema = z.object({
  domain: z.string().min(1).optional(),
  provider: z.enum(['cloudflare', 'bunny', 'powerdns']).optional(),
  zoneId: z.string().min(1).optional(),
  credential: z.string().optional(),
  enabled: z.boolean().optional(),
});

export default async (uuid: string, payload: z.infer<typeof updateDomainSchema>): Promise<Domain> => {
  const { data } = await axiosInstance.patch(
    `${SUBDOMAIN_ADMIN_BASE}/domains/${uuid}`,
    serializeForApi(updateDomainSchema, payload),
  );
  return parseFromApi(domainSchema, data.domain);
};
