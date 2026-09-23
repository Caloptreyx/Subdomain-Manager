import { axiosInstance } from '@/api/axios.ts';
import { SUBDOMAIN_ADMIN_BASE } from '../../lib/schemas.ts';

export default async (subdomainUuid: string, force = false): Promise<void> => {
  await axiosInstance.delete(`${SUBDOMAIN_ADMIN_BASE}/subdomains/${subdomainUuid}`, {
    params: { force },
  });
};
