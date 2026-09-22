import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi, serializeForApi } from '@/lib/serialization/api-transform.ts';
import { type Subdomain, subdomainClientBase, subdomainSchema } from '../../lib/schemas.ts';

export const updateSubdomainSchema = z.object({
  allocationUuid: z.string(),
});

export default async (
  serverUuid: string,
  subdomainUuid: string,
  payload: z.infer<typeof updateSubdomainSchema>,
): Promise<Subdomain> => {
  const { data } = await axiosInstance.patch(
    `${subdomainClientBase(serverUuid)}/${subdomainUuid}`,
    serializeForApi(updateSubdomainSchema, payload),
  );
  return parseFromApi(subdomainSchema, data.subdomain);
};
