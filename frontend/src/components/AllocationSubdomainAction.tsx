import { faGlobe, faPlus } from '@fortawesome/free-solid-svg-icons';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { z } from 'zod';
import type { ContextMenuItem } from '@/elements/overlays/ContextMenu.tsx';
import { handleCopyToClipboard } from '@/lib/clipboard/copy.ts';
import { serverAllocationSchema } from '@/lib/schemas/server/allocations.ts';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { useServerStore } from '@/stores/server.ts';
import getSubdomains from '../api/server/getSubdomains.ts';
import SubdomainCreateModal from '../pages/server/modals/SubdomainCreateModal.tsx';
import { serverSubdomainsQueryKey } from '../pages/server/ServerSubdomainsPage.tsx';
import { useExtTranslations } from '../translations.ts';

const ownItems = new WeakSet<ContextMenuItem>();

export default function AllocationSubdomainAction({
  items,
  allocation,
}: {
  items: ContextMenuItem[];
  allocation: z.infer<typeof serverAllocationSchema>;
}) {
  const { t: tExt } = useExtTranslations();
  const { addToast } = useToast();
  const { server } = useServerStore();
  const canRead = useServerCan('subdomains.read');
  const canCreate = useServerCan('subdomains.create');
  const [opened, setOpened] = useState(false);

  const { data, refetch } = useQuery({
    queryKey: serverSubdomainsQueryKey(server.uuid),
    queryFn: () => getSubdomains(server.uuid),
    enabled: canRead,
  });

  const subdomains = data?.subdomains.filter((subdomain) => subdomain.allocation?.uuid === allocation.uuid) ?? [];
  const createDisabled = !data || data.subdomains.length >= data.limit || data.domains.length === 0;

  for (let i = items.length - 1; i >= 0; i--) {
    if (ownItems.has(items[i])) {
      items.splice(i, 1);
    }
  }

  const added: ContextMenuItem[] = [
    ...subdomains.map(
      (subdomain): ContextMenuItem => ({
        type: 'action',
        icon: faGlobe,
        label: subdomain.fqdn,
        onClick: handleCopyToClipboard(subdomain.fqdn, addToast),
        color: 'gray',
        canAccess: canRead,
      }),
    ),
    {
      type: 'action',
      icon: faPlus,
      label: tExt('pages.server.subdomains.button.create', {}),
      onClick: () => setOpened(true),
      color: 'gray',
      canAccess: canRead && canCreate,
      disabled: createDisabled,
    },
  ];

  for (const item of added) {
    ownItems.add(item);
    items.push(item);
  }

  return (
    <SubdomainCreateModal
      opened={opened}
      onClose={() => setOpened(false)}
      serverUuid={server.uuid}
      domains={data?.domains ?? []}
      allocationUuid={allocation.uuid}
      onCreated={refetch}
    />
  );
}
