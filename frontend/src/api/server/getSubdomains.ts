import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { domainRefSchema, type Subdomain, subdomainClientBase, subdomainSchema } from '../../lib/schemas.ts';

export interface ServerSubdomainsResponse {
  subdomains: Subdomain[];
  limit: number;
  domains: z.infer<typeof domainRefSchema>[];
}

export default async (serverUuid: string): Promise<ServerSubdomainsResponse> => {
  const { data } = await axiosInstance.get(subdomainClientBase(serverUuid));
  return {
    subdomains: data.subdomains.map((subdomain: unknown) => parseFromApi(subdomainSchema, subdomain)),
    limit: data.limit,
    domains: data.domains.map((domain: unknown) => parseFromApi(domainRefSchema, domain)),
  };
};
