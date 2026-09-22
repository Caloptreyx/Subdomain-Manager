import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi, serializeForApi } from '@/lib/serialization/api-transform.ts';
import { type Domain, domainSchema, SUBDOMAIN_ADMIN_BASE } from '../../lib/schemas.ts';

export const createDomainSchema = z.object({
  domain: z.string().min(1),
  provider: z.enum(['cloudflare', 'bunny']),
  zoneId: z.string().min(1),
  credential: z.string().min(1),
});

export default async (payload: z.infer<typeof createDomainSchema>): Promise<Domain> => {
  const { data } = await axiosInstance.post(
    `${SUBDOMAIN_ADMIN_BASE}/domains`,
    serializeForApi(createDomainSchema, payload),
  );
  return parseFromApi(domainSchema, data.domain);
};
