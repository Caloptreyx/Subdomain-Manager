import { SelectProps } from '@mantine/core';
import { z } from 'zod';
import getServerAllocations from '@/api/admin/servers/allocations/getServerAllocations.ts';
import getAllocations from '@/api/server/allocations/getAllocations.ts';
import Select from '@/elements/input/Select.tsx';
import { serverAllocationSchema } from '@/lib/schemas/server/allocations.ts';
import { useSearchableResource } from '@/plugins/resource/useSearchableResource.ts';
import { useExtTranslations } from '../translations.ts';

type Allocation = z.infer<typeof serverAllocationSchema>;

type Props = Omit<SelectProps, 'data' | 'value' | 'onChange'> & {
  serverUuid: string;
  /** Load allocations through the admin API (for the admin pages). */
  admin?: boolean;
  value: string | null;
  onChange: (uuid: string | null, allocation: Allocation | null) => void;
};

export default function AllocationSelect({ serverUuid, admin = false, value, onChange, ...rest }: Props) {
  const { t: tExt } = useExtTranslations();

  const allocations = useSearchableResource<Allocation>({
    queryKey: ['dev.caloptreyx.subdomains', admin ? 'admin' : 'server', serverUuid, 'allocations'],
    fetcher: (search) => (admin ? getServerAllocations(serverUuid, 1, search) : getAllocations(serverUuid, 1, search)),
  });

  const known = new Map<string, Allocation>();
  for (const allocation of allocations.items) {
    known.set(allocation.uuid, allocation);
  }

  const data = [...known.values()].map((allocation) => ({
    label: `${allocation.ipAlias ?? allocation.ip}:${allocation.port}${
      allocation.isPrimary ? ` (${tExt('pages.server.subdomains.badge.primary', {})})` : ''
    }`,
    value: allocation.uuid,
  }));

  return (
    <Select
      data={data}
      value={value}
      onChange={(uuid) => onChange(uuid, uuid ? (known.get(uuid) ?? null) : null)}
      searchable
      searchValue={allocations.search}
      onSearchChange={allocations.setSearch}
      loading={allocations.loading}
      {...rest}
    />
  );
}
