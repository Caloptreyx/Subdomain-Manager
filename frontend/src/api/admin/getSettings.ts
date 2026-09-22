import { axiosInstance } from '@/api/axios.ts';
import { parseFromApi } from '@/lib/serialization/api-transform.ts';
import { type ExtensionSettings, extensionSettingsSchema, SUBDOMAIN_ADMIN_BASE } from '../../lib/schemas.ts';

export default async (): Promise<ExtensionSettings> => {
  const { data } = await axiosInstance.get(`${SUBDOMAIN_ADMIN_BASE}/settings`);
  return parseFromApi(extensionSettingsSchema, data.settings);
};
