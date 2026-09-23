import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi, serializeForApi } from '@/lib/serialization/api-transform.ts';
import {
  type AdminSubdomain,
  adminSubdomainSchema,
  SUBDOMAIN_ADMIN_BASE,
  SUBDOMAIN_NAME_REGEX,
} from '../../lib/schemas.ts';

export const adminCreateSubdomainSchema = z.object({
  serverUuid: z.string(),
  domainUuid: z.string(),
  name: z.string().regex(SUBDOMAIN_NAME_REGEX),
  allocationUuid: z.string(),
});

export default async (payload: z.infer<typeof adminCreateSubdomainSchema>): Promise<AdminSubdomain> => {
  const { data } = await axiosInstance.post(
    `${SUBDOMAIN_ADMIN_BASE}/subdomains`,
    serializeForApi(adminCreateSubdomainSchema, payload),
  );
  return parseFromApi(adminSubdomainSchema, data.subdomain);
};
