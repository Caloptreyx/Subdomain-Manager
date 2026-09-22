import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { type Domain, domainSchema, SUBDOMAIN_ADMIN_BASE } from '../../lib/schemas.ts';

export default async (): Promise<Domain[]> => {
  const { data } = await axiosInstance.get(`${SUBDOMAIN_ADMIN_BASE}/domains`);
  return data.domains.map((domain: unknown) => parseFromApi(domainSchema, domain));
};
