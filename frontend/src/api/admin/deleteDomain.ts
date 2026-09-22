import { axiosInstance } from '@/api/axios.ts';
import { SUBDOMAIN_ADMIN_BASE } from '../../lib/schemas.ts';

export default async (uuid: string, force = false): Promise<void> => {
  await axiosInstance.delete(`${SUBDOMAIN_ADMIN_BASE}/domains/${uuid}`, {
    params: { force },
  });
};
