import { axiosInstance } from '@/api/axios.ts';
import { subdomainClientBase } from '../../lib/schemas.ts';

export default async (serverUuid: string, subdomainUuid: string, force = false): Promise<void> => {
  await axiosInstance.delete(`${subdomainClientBase(serverUuid)}/${subdomainUuid}`, {
    params: { force },
  });
};
