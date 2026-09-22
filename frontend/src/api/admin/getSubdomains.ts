import { axiosInstance } from '@/api/axios.ts';
import { parsePaginationFromApi } from '@/lib/serialization/api-transform.ts';
import { type AdminSubdomain, adminSubdomainSchema, SUBDOMAIN_ADMIN_BASE } from '../../lib/schemas.ts';

export default async (page: number, search?: string): Promise<Pagination<AdminSubdomain>> => {
  const { data } = await axiosInstance.get(`${SUBDOMAIN_ADMIN_BASE}/subdomains`, {
    params: { page, search },
  });
  return parsePaginationFromApi(adminSubdomainSchema, data.subdomains);
};
