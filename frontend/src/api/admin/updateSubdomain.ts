import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { type AdminSubdomain, adminSubdomainSchema, SUBDOMAIN_ADMIN_BASE } from '../../lib/schemas.ts';

export default async (subdomainUuid: string, allocationUuid: string): Promise<AdminSubdomain> => {
  const { data } = await axiosInstance.patch(`${SUBDOMAIN_ADMIN_BASE}/subdomains/${subdomainUuid}`, {
    allocation_uuid: allocationUuid,
  });
  return parseFromApi(adminSubdomainSchema, data.subdomain);
};
