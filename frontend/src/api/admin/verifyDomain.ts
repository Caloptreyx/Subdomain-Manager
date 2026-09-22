import { axiosInstance } from '@/api/axios.ts';
import { SUBDOMAIN_ADMIN_BASE } from '../../lib/schemas.ts';

export default async (uuid: string): Promise<string> => {
  const { data } = await axiosInstance.post(`${SUBDOMAIN_ADMIN_BASE}/domains/${uuid}/verify`);
  return data.zone_name;
};
