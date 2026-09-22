import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi, serializeForApi } from '@/lib/serialization/api-transform.ts';
import { SUBDOMAIN_NAME_REGEX, type Subdomain, subdomainClientBase, subdomainSchema } from '../../lib/schemas.ts';

export const createSubdomainSchema = z.object({
  domainUuid: z.string(),
  name: z.string().regex(SUBDOMAIN_NAME_REGEX),
  allocationUuid: z.string(),
});

export default async (serverUuid: string, payload: z.infer<typeof createSubdomainSchema>): Promise<Subdomain> => {
  const { data } = await axiosInstance.post(
    subdomainClientBase(serverUuid),
    serializeForApi(createSubdomainSchema, payload),
  );
  return parseFromApi(subdomainSchema, data.subdomain);
};
