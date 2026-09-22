import { axiosInstance } from '@/api/axios.ts';
import { serializeForApi } from '@/lib/serialization/api-transform.ts';
import { type ExtensionSettings, extensionSettingsSchema, SUBDOMAIN_ADMIN_BASE } from '../../lib/schemas.ts';

export default async (settings: ExtensionSettings): Promise<void> => {
  await axiosInstance.put(`${SUBDOMAIN_ADMIN_BASE}/settings`, serializeForApi(extensionSettingsSchema, settings));
};
